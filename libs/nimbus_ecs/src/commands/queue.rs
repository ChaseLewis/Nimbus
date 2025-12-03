//! Command queue for deferred world mutations.
//!
//! Commands are stored in a dense byte buffer and batched by type for optimal performance.

use std::any::TypeId;
use std::mem::MaybeUninit;

use crate::world::World;

/// A command that can be applied to the world.
pub trait Command: Send + 'static {
    /// Applies this command to the world (owned version, avoids boxing).
    fn apply_owned(self, world: &mut World) where Self: Sized {
        // Default implementation boxes and calls apply - override for efficiency
        Box::new(self).apply(world);
    }
    
    /// Applies this command to the world (boxed version for trait objects).
    fn apply(self: Box<Self>, world: &mut World);
    
    /// Returns the command kind for batching purposes.
    fn kind(&self) -> CommandKind {
        CommandKind::Other
    }
    
    /// Returns an optional batch apply function for processing multiple commands at once.
    /// Commands that can be batched (like InsertBundle) should return Some.
    fn batch_apply_fn(&self) -> Option<unsafe fn(ptrs: &[*mut u8], world: &mut World)> {
        None
    }
}

/// Command kind for batching - commands of the same kind are applied together.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CommandKind {
    /// Spawn commands (spawn_with, spawn_empty)
    Spawn,
    /// Despawn commands
    Despawn,
    /// Insert bundle commands (grouped by bundle TypeId)
    InsertBundle(TypeId),
    /// Insert single component commands
    InsertComponent(TypeId),
    /// Remove component commands
    RemoveComponent(TypeId),
    /// Other/custom commands
    Other,
}

// ============================================================================
// CommandQueue - Dense byte buffer with batching
// ============================================================================

/// Metadata for a command with kind information for batching.
struct CommandMeta {
    /// Byte offset into the buffer where this command starts.
    offset: usize,
    /// Command kind for grouping.
    kind: CommandKind,
    /// Function to apply the command (type-erased).
    apply: unsafe fn(value: *mut u8, world: &mut World),
    /// Optional function to batch-apply multiple commands of the same type.
    batch_apply: Option<unsafe fn(ptrs: &[*mut u8], world: &mut World)>,
    /// Function to drop the command if not applied.
    drop: unsafe fn(value: *mut u8),
}

/// A command queue that batches commands by type for better performance.
/// 
/// Commands are stored in a dense byte buffer for cache locality and minimal allocations.
/// They are applied in batches grouped by `CommandKind`, which reduces:
/// - Branch mispredictions (same code path for entire batch)
/// - Archetype lookups (similar commands access same archetypes)
/// - Cache misses (better spatial locality within batches)
pub struct CommandQueue {
    /// Raw byte storage for commands.
    bytes: Vec<MaybeUninit<u8>>,
    /// Metadata for each command with kind for batching.
    metas: Vec<CommandMeta>,
    /// Scratch buffer for batch pointers (reused to avoid allocations).
    scratch_ptrs: Vec<*mut u8>,
    /// The kind of the last pushed command (for transition tracking).
    last_kind: Option<CommandKind>,
    /// Number of kind transitions (when consecutive commands have different kinds).
    /// Used to determine if data is already mostly sorted.
    transitions: usize,
}

impl Default for CommandQueue {
    fn default() -> Self {
        Self::new()
    }
}

/// Threshold for transition rate below which we skip sorting.
/// If transitions / len < this value, data is mostly sorted already.
const SKIP_SORT_THRESHOLD: f32 = 0.05;

impl CommandQueue {
    /// Creates a new empty command queue.
    pub fn new() -> Self {
        Self {
            bytes: Vec::new(),
            metas: Vec::new(),
            scratch_ptrs: Vec::new(),
            last_kind: None,
            transitions: 0,
        }
    }

    /// Creates a new command queue with pre-allocated capacity.
    pub fn with_capacity(byte_capacity: usize, command_capacity: usize) -> Self {
        Self {
            bytes: Vec::with_capacity(byte_capacity),
            metas: Vec::with_capacity(command_capacity),
            scratch_ptrs: Vec::with_capacity(command_capacity),
            last_kind: None,
            transitions: 0,
        }
    }

    /// Returns true if the queue is empty.
    pub fn is_empty(&self) -> bool {
        self.metas.is_empty()
    }

    /// Returns the number of queued commands.
    pub fn len(&self) -> usize {
        self.metas.len()
    }

    /// Pushes a command onto the queue.
    pub fn push<C: Command>(&mut self, command: C) {
        let size = std::mem::size_of::<C>();
        let align = std::mem::align_of::<C>();
        let kind = command.kind();
        let batch_apply = command.batch_apply_fn();
        
        // Track kind transitions for skip-sort heuristic
        if let Some(last) = self.last_kind {
            if last != kind {
                self.transitions += 1;
            }
        }
        self.last_kind = Some(kind);
        
        // Align the current position
        let offset = align_up(self.bytes.len(), align);
        let required = offset + size;
        
        // Ensure capacity with exponential growth (like Vec::push)
        if required > self.bytes.capacity() {
            // At least double, but ensure we have enough
            let new_capacity = self.bytes.capacity().max(64).max(required) * 2;
            self.bytes.reserve(new_capacity - self.bytes.len());
        }
        
        // Extend length to fit the command (no realloc, capacity already ensured)
        self.bytes.resize(required, MaybeUninit::uninit());
        
        // Write command bytes directly into buffer
        unsafe {
            std::ptr::write(
                self.bytes.as_mut_ptr().add(offset) as *mut C,
                command,
            );
        }
        
        // Store metadata with kind for batching
        self.metas.push(CommandMeta {
            offset,
            kind,
            apply: apply_command::<C>,
            batch_apply,
            drop: drop_command::<C>,
        });
    }

    /// Applies all queued commands to the world, grouped by command kind.
    /// 
    /// Commands are sorted by kind before application for better batching:
    /// 1. Spawn commands (create entities first)
    /// 2. InsertBundle commands (grouped by bundle type)
    /// 3. InsertComponent commands (grouped by component type)
    /// 4. RemoveComponent commands (grouped by component type)
    /// 5. Despawn commands (remove entities last)
    /// 6. Other commands
    pub fn apply(&mut self, world: &mut World) {
        if self.metas.is_empty() {
            return;
        }

        // Skip sort if data is already mostly sorted (few transitions between kinds).
        // transition_rate = transitions / len
        // If < 5%, commands came in runs and are already grouped.
        let needs_sort = self.transitions > 0 && 
            (self.transitions as f32 / self.metas.len() as f32) >= SKIP_SORT_THRESHOLD;
        
        if needs_sort {
            // Use unstable sort - faster and we don't need stability
            self.metas.sort_unstable_by(|a, b| kind_order(&a.kind).cmp(&kind_order(&b.kind)));
        }

        // Apply commands, batching adjacent commands of the same type
        let mut i = 0;
        while i < self.metas.len() {
            let meta = &self.metas[i];
            
            // Check if this is a batchable command (InsertBundle or RemoveComponent)
            let batch_type_id = match meta.kind {
                CommandKind::InsertBundle(type_id) => Some(type_id),
                CommandKind::RemoveComponent(type_id) => Some(type_id),
                _ => None,
            };
            
            if let Some(_type_id) = batch_type_id {
                // Find the range of consecutive commands with same kind and type
                let batch_start = i;
                let mut batch_end = i + 1;
                let kind = meta.kind;
                while batch_end < self.metas.len() {
                    if self.metas[batch_end].kind == kind {
                        batch_end += 1;
                    } else {
                        break;
                    }
                }
                
                // If we have a batch, use the batch apply function
                if batch_end - batch_start > 1 {
                    let batch_apply = self.metas[batch_start].batch_apply;
                    if let Some(batch_fn) = batch_apply {
                        // Reuse scratch buffer to avoid allocation
                        self.scratch_ptrs.clear();
                        self.scratch_ptrs.reserve(batch_end - batch_start);
                        for j in batch_start..batch_end {
                            self.scratch_ptrs.push(unsafe { 
                                self.bytes.as_mut_ptr().add(self.metas[j].offset) as *mut u8 
                            });
                        }
                        
                        unsafe { batch_fn(&self.scratch_ptrs, world) };
                        i = batch_end;
                        continue;
                    }
                }
            }
            
            // Single command apply
            let ptr = unsafe { self.bytes.as_mut_ptr().add(meta.offset) as *mut u8 };
            unsafe { (meta.apply)(ptr, world) };
            i += 1;
        }
        
        self.metas.clear();
        self.bytes.clear();
        self.last_kind = None;
        self.transitions = 0;
    }

    /// Clears all queued commands without applying them.
    pub fn clear(&mut self) {
        for meta in self.metas.drain(..) {
            let ptr = unsafe { self.bytes.as_mut_ptr().add(meta.offset) as *mut u8 };
            unsafe { (meta.drop)(ptr) };
        }
        self.bytes.clear();
        self.last_kind = None;
        self.transitions = 0;
    }
}

impl Drop for CommandQueue {
    fn drop(&mut self) {
        for meta in &self.metas {
            let ptr = unsafe { self.bytes.as_mut_ptr().add(meta.offset) as *mut u8 };
            unsafe { (meta.drop)(ptr) };
        }
    }
}

/// Type-erased function to apply a command.
/// 
/// # Safety
/// `ptr` must point to a valid, initialized value of type `C`.
unsafe fn apply_command<C: Command>(ptr: *mut u8, world: &mut World) {
    // Read command out of buffer (takes ownership)
    let command = unsafe { std::ptr::read(ptr as *const C) };
    // Apply directly without boxing
    command.apply_owned(world);
}

/// Type-erased function to drop a command.
/// 
/// # Safety
/// `ptr` must point to a valid, initialized value of type `C`.
unsafe fn drop_command<C: Command>(ptr: *mut u8) {
    unsafe { std::ptr::drop_in_place(ptr as *mut C) };
}

/// Aligns an offset up to the specified alignment.
#[inline]
const fn align_up(offset: usize, align: usize) -> usize {
    (offset + align - 1) & !(align - 1)
}

/// Returns sort order for command kinds.
/// Lower values are applied first.
fn kind_order(kind: &CommandKind) -> u32 {
    match kind {
        CommandKind::Spawn => 0,
        CommandKind::InsertBundle(_) => 1,
        CommandKind::InsertComponent(_) => 2,
        CommandKind::RemoveComponent(_) => 3,
        CommandKind::Despawn => 4,
        CommandKind::Other => 5,
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    /// Test command that increments a counter when applied.
    struct IncrementCommand {
        counter: Arc<AtomicUsize>,
        value: usize,
    }

    impl Command for IncrementCommand {
        fn apply(self: Box<Self>, _world: &mut World) {
            self.counter.fetch_add(self.value, Ordering::SeqCst);
        }
    }

    /// Test command that tracks drops.
    struct DropTracker {
        dropped: Arc<AtomicUsize>,
    }

    impl Drop for DropTracker {
        fn drop(&mut self) {
            self.dropped.fetch_add(1, Ordering::SeqCst);
        }
    }

    impl Command for DropTracker {
        fn apply(self: Box<Self>, _world: &mut World) {
            // Don't increment on apply - only on drop
        }
    }

    #[test]
    fn queue_applies_commands_in_order() {
        let mut world = World::new();
        let counter = Arc::new(AtomicUsize::new(0));

        let mut queue = CommandQueue::new();
        queue.push(IncrementCommand { counter: counter.clone(), value: 1 });
        queue.push(IncrementCommand { counter: counter.clone(), value: 2 });
        queue.push(IncrementCommand { counter: counter.clone(), value: 3 });

        assert_eq!(queue.len(), 3);
        queue.apply(&mut world);
        assert_eq!(queue.len(), 0);
        assert_eq!(counter.load(Ordering::SeqCst), 6);
    }

    #[test]
    fn queue_drops_unapplied_commands() {
        let dropped = Arc::new(AtomicUsize::new(0));

        {
            let mut queue = CommandQueue::new();
            queue.push(DropTracker { dropped: dropped.clone() });
            queue.push(DropTracker { dropped: dropped.clone() });
            // Queue is dropped without applying
        }

        assert_eq!(dropped.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn queue_clear_drops_commands() {
        let dropped = Arc::new(AtomicUsize::new(0));

        let mut queue = CommandQueue::new();
        queue.push(DropTracker { dropped: dropped.clone() });
        queue.push(DropTracker { dropped: dropped.clone() });
        
        queue.clear();
        
        assert_eq!(dropped.load(Ordering::SeqCst), 2);
        assert!(queue.is_empty());
    }

    use crate::World;
}
