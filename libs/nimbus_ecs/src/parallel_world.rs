//! Thread-safe World access for parallel system execution.
//!
//! This module provides `ParallelWorldCell`, a wrapper around `World` that allows
//! multiple systems to access it concurrently with runtime borrow checking.
//!
//! # Safety Model
//!
//! - Multiple readers for the same component type are allowed
//! - Writers have exclusive access to their component type
//! - Different component types can be accessed concurrently
//! - Conflicts are detected at runtime and panic in debug builds

use std::any::TypeId;
use std::cell::UnsafeCell;
use std::sync::atomic::{AtomicIsize, Ordering};
use std::sync::RwLock;
use std::collections::HashMap;

use crate::commands::CommandQueue;
use crate::component::ComponentId;
use crate::world::World;

/// Tracks concurrent access to a single resource (component type or singleton).
/// 
/// Uses a signed counter:
/// - Positive: number of active readers
/// - -1: exclusive writer
/// - 0: not accessed
#[derive(Debug)]
pub struct AccessTracker {
    /// Positive = readers, -1 = writer, 0 = free
    state: AtomicIsize,
    #[cfg(debug_assertions)]
    type_name: &'static str,
}

impl AccessTracker {
    pub fn new(#[allow(unused)] type_name: &'static str) -> Self {
        Self {
            state: AtomicIsize::new(0),
            #[cfg(debug_assertions)]
            type_name,
        }
    }

    /// Acquires read access. Panics if a writer holds access.
    pub fn acquire_read(&self) {
        loop {
            let current = self.state.load(Ordering::Acquire);
            if current < 0 {
                #[cfg(debug_assertions)]
                panic!(
                    "Cannot read {} while it's being written",
                    self.type_name
                );
                #[cfg(not(debug_assertions))]
                std::hint::spin_loop();
            } else {
                if self.state.compare_exchange_weak(
                    current,
                    current + 1,
                    Ordering::AcqRel,
                    Ordering::Relaxed,
                ).is_ok() {
                    return;
                }
            }
        }
    }

    /// Releases read access.
    pub fn release_read(&self) {
        let prev = self.state.fetch_sub(1, Ordering::Release);
        debug_assert!(prev > 0, "Released read that wasn't acquired");
    }

    /// Acquires write access. Panics if any readers or writers hold access.
    pub fn acquire_write(&self) {
        loop {
            let current = self.state.load(Ordering::Acquire);
            if current != 0 {
                #[cfg(debug_assertions)]
                panic!(
                    "Cannot write {} while it's being {} (state={})",
                    self.type_name,
                    if current > 0 { "read" } else { "written" },
                    current
                );
                #[cfg(not(debug_assertions))]
                std::hint::spin_loop();
            } else {
                if self.state.compare_exchange_weak(
                    0,
                    -1,
                    Ordering::AcqRel,
                    Ordering::Relaxed,
                ).is_ok() {
                    return;
                }
            }
        }
    }

    /// Releases write access.
    pub fn release_write(&self) {
        let prev = self.state.swap(0, Ordering::Release);
        debug_assert_eq!(prev, -1, "Released write that wasn't acquired");
    }

    /// Returns true if currently accessed (read or write).
    pub fn is_accessed(&self) -> bool {
        self.state.load(Ordering::Relaxed) != 0
    }
}

/// Thread-safe access to World for parallel system execution.
///
/// Tracks which component types are being read/written and ensures
/// no conflicting access occurs.
///
/// # Example
///
/// ```ignore
/// let cell = ParallelWorldCell::new(&mut world);
///
/// // In parallel threads:
/// pool.scope(|s| {
///     s.spawn(|_| {
///         // System A reads Position
///         let guard = cell.acquire_read::<Position>();
///         // ... read Position components ...
///     });
///     s.spawn(|_| {
///         // System B writes Health (OK - different type)
///         let guard = cell.acquire_write::<Health>();
///         // ... write Health components ...
///     });
/// });
/// ```
pub struct ParallelWorldCell<'w> {
    /// World wrapped in UnsafeCell for interior mutability
    world: UnsafeCell<&'w mut World>,
    /// Per-component-type access tracking (RwLock for thread-safe lazy init)
    trackers: RwLock<HashMap<TypeId, AccessTracker>>,
    /// Command queue reference for this cell
    command_queue: Option<*const std::cell::RefCell<CommandQueue>>,
}

// SAFETY: ParallelWorldCell is designed for controlled concurrent access.
// The trackers HashMap is protected by RwLock for thread-safe initialization.
// During parallel execution, atomic operations in AccessTracker handle concurrent access.
// Access to World is controlled via acquire_read/acquire_write guards.
unsafe impl<'w> Send for ParallelWorldCell<'w> {}
unsafe impl<'w> Sync for ParallelWorldCell<'w> {}

impl<'w> ParallelWorldCell<'w> {
    /// Creates a new parallel world cell.
    pub fn new(world: &'w mut World) -> Self {
        Self {
            world: UnsafeCell::new(world),
            trackers: RwLock::new(HashMap::new()),
            command_queue: None,
        }
    }

    /// Creates a new parallel world cell with a command queue.
    pub fn with_commands(world: &'w mut World, commands: &'w std::cell::RefCell<CommandQueue>) -> Self {
        Self {
            world: UnsafeCell::new(world),
            trackers: RwLock::new(HashMap::new()),
            command_queue: Some(commands as *const _),
        }
    }

    /// Returns a reference to the underlying world.
    /// 
    /// # Safety
    /// 
    /// Caller must ensure proper access tracking is in place.
    #[inline]
    pub unsafe fn world(&self) -> &World {
        unsafe { &**self.world.get() }
    }

    /// Returns a mutable reference to the underlying world.
    /// 
    /// # Safety
    /// 
    /// Caller must have exclusive access or proper tracking in place.
    #[inline]
    pub unsafe fn world_mut(&self) -> &mut World {
        unsafe { &mut **self.world.get() }
    }

    /// Gets or creates a tracker for a component type, then calls the provided function.
    /// 
    /// Uses read-lock fast path when tracker exists, write-lock only for creation.
    fn with_tracker<T: 'static, R>(&self, f: impl FnOnce(&AccessTracker) -> R) -> R {
        let type_id = TypeId::of::<T>();
        
        // Fast path: try read lock first (most common case after initialization)
        {
            let trackers = self.trackers.read().unwrap();
            if let Some(tracker) = trackers.get(&type_id) {
                return f(tracker);
            }
        }
        
        // Slow path: need to create tracker, acquire write lock
        let mut trackers = self.trackers.write().unwrap();
        // Double-check after acquiring write lock (another thread may have created it)
        let tracker = trackers.entry(type_id).or_insert_with(|| {
            AccessTracker::new(std::any::type_name::<T>())
        });
        f(tracker)
    }

    /// Releases read access for a component type.
    fn release_read<T: 'static>(&self) {
        let type_id = TypeId::of::<T>();
        let trackers = self.trackers.read().unwrap();
        if let Some(tracker) = trackers.get(&type_id) {
            tracker.release_read();
        }
    }

    /// Releases write access for a component type.
    fn release_write<T: 'static>(&self) {
        let type_id = TypeId::of::<T>();
        let trackers = self.trackers.read().unwrap();
        if let Some(tracker) = trackers.get(&type_id) {
            tracker.release_write();
        }
    }

    /// Acquires read access for a component type.
    pub fn acquire_read<T: 'static>(&self) -> ReadGuard<'w, '_, T> {
        self.with_tracker::<T, _>(|tracker| tracker.acquire_read());
        ReadGuard {
            cell: self,
            _marker: std::marker::PhantomData,
        }
    }

    /// Acquires write access for a component type.
    pub fn acquire_write<T: 'static>(&self) -> WriteGuard<'w, '_, T> {
        self.with_tracker::<T, _>(|tracker| tracker.acquire_write());
        WriteGuard {
            cell: self,
            _marker: std::marker::PhantomData,
        }
    }

    /// Returns the command queue if available.
    pub fn command_queue(&self) -> Option<&std::cell::RefCell<CommandQueue>> {
        self.command_queue.map(|ptr| unsafe { &*ptr })
    }
}

/// RAII guard for read access to a component type.
pub struct ReadGuard<'w, 'a, T: 'static> {
    cell: &'a ParallelWorldCell<'w>,
    _marker: std::marker::PhantomData<T>,
}

impl<T: 'static> Drop for ReadGuard<'_, '_, T> {
    fn drop(&mut self) {
        self.cell.release_read::<T>();
    }
}

/// RAII guard for write access to a component type.
pub struct WriteGuard<'w, 'a, T: 'static> {
    cell: &'a ParallelWorldCell<'w>,
    _marker: std::marker::PhantomData<T>,
}

impl<T: 'static> Drop for WriteGuard<'_, '_, T> {
    fn drop(&mut self) {
        self.cell.release_write::<T>();
    }
}

/// Describes the access pattern of a system parameter.
#[derive(Debug, Clone, Default)]
pub struct ParamAccess {
    /// Component types read immutably
    pub reads: Vec<ComponentId>,
    /// Component types written mutably
    pub writes: Vec<ComponentId>,
    /// Whether this param needs exclusive world access
    pub exclusive: bool,
}

impl ParamAccess {
    /// Creates empty access (no components accessed).
    pub fn none() -> Self {
        Self::default()
    }

    /// Creates read-only access for a single component type.
    pub fn read<T: crate::Component>() -> Self {
        Self {
            reads: vec![ComponentId::of::<T>()],
            writes: Vec::new(),
            exclusive: false,
        }
    }

    /// Creates write access for a single component type.
    pub fn write<T: crate::Component>() -> Self {
        Self {
            reads: Vec::new(),
            writes: vec![ComponentId::of::<T>()],
            exclusive: false,
        }
    }

    /// Creates exclusive access (blocks all other systems).
    pub fn exclusive() -> Self {
        Self {
            reads: Vec::new(),
            writes: Vec::new(),
            exclusive: true,
        }
    }

    /// Merges another access into this one.
    pub fn merge(&mut self, other: &ParamAccess) {
        self.reads.extend(other.reads.iter().cloned());
        self.writes.extend(other.writes.iter().cloned());
        self.exclusive |= other.exclusive;
    }

    /// Returns true if this access conflicts with another.
    pub fn conflicts_with(&self, other: &ParamAccess) -> bool {
        if self.exclusive || other.exclusive {
            return true;
        }

        // Write-read conflict
        for write in &self.writes {
            if other.reads.contains(write) || other.writes.contains(write) {
                return true;
            }
        }

        // Read-write conflict
        for write in &other.writes {
            if self.reads.contains(write) {
                return true;
            }
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn access_tracker_multiple_readers() {
        let tracker = AccessTracker::new("Test");
        
        tracker.acquire_read();
        tracker.acquire_read();
        tracker.acquire_read();
        
        assert!(tracker.is_accessed());
        
        tracker.release_read();
        tracker.release_read();
        tracker.release_read();
        
        assert!(!tracker.is_accessed());
    }

    #[test]
    fn access_tracker_exclusive_writer() {
        let tracker = AccessTracker::new("Test");
        
        tracker.acquire_write();
        assert!(tracker.is_accessed());
        
        tracker.release_write();
        assert!(!tracker.is_accessed());
    }

    #[test]
    #[should_panic(expected = "Cannot write")]
    fn access_tracker_write_while_reading_panics() {
        let tracker = AccessTracker::new("Test");
        tracker.acquire_read();
        tracker.acquire_write(); // Should panic
    }

    #[test]
    #[should_panic(expected = "Cannot read")]
    fn access_tracker_read_while_writing_panics() {
        let tracker = AccessTracker::new("Test");
        tracker.acquire_write();
        tracker.acquire_read(); // Should panic
    }

    #[test]
    fn parallel_world_cell_concurrent_readers() {
        let mut world = World::new();
        let cell = ParallelWorldCell::new(&mut world);

        // Use scoped threads to allow borrowing
        std::thread::scope(|s| {
            for _ in 0..4 {
                s.spawn(|| {
                    struct TestComponent;
                    let _guard = cell.acquire_read::<TestComponent>();
                    // Simulate some work
                    std::thread::sleep(std::time::Duration::from_millis(10));
                });
            }
        });
    }

    #[test]
    fn param_access_conflict_detection() {
        use crate::component::ComponentId;
        
        // Helper to create ComponentId from TypeId for test types
        fn id_of<T: 'static>() -> ComponentId {
            ComponentId::from_type_id(TypeId::of::<T>())
        }
        
        struct Position;
        struct Velocity;
        struct Health;

        // Read Position, Read Velocity - no conflict
        let a = ParamAccess {
            reads: vec![id_of::<Position>(), id_of::<Velocity>()],
            writes: vec![],
            exclusive: false,
        };
        let b = ParamAccess {
            reads: vec![id_of::<Position>()],
            writes: vec![],
            exclusive: false,
        };
        assert!(!a.conflicts_with(&b));

        // Read Position vs Write Position - conflict!
        let c = ParamAccess {
            reads: vec![id_of::<Position>()],
            writes: vec![],
            exclusive: false,
        };
        let d = ParamAccess {
            reads: vec![],
            writes: vec![id_of::<Position>()],
            exclusive: false,
        };
        assert!(c.conflicts_with(&d));

        // Write Health vs Read Position - no conflict
        let e = ParamAccess {
            reads: vec![],
            writes: vec![id_of::<Health>()],
            exclusive: false,
        };
        let f = ParamAccess {
            reads: vec![id_of::<Position>()],
            writes: vec![],
            exclusive: false,
        };
        assert!(!e.conflicts_with(&f));

        // Exclusive conflicts with everything
        let g = ParamAccess::exclusive();
        assert!(g.conflicts_with(&a));
        assert!(g.conflicts_with(&ParamAccess::none()));
    }
}

