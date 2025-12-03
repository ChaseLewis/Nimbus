//! Parallel priority-based system scheduler.
//!
//! This scheduler extends [`PriorityScheduler`] with the ability to run
//! non-conflicting systems in parallel within each priority phase.
//!
//! # Execution Model
//!
//! For each priority phase:
//! 1. Event handlers run first (sequential, to maintain ordering)
//! 2. Regular systems run in parallel batches based on access conflicts
//! 3. Commands are flushed
//!
//! Systems are grouped into parallel batches where no two systems in the same
//! batch have conflicting access (one writes what another reads/writes).

use std::cell::RefCell;
use std::collections::HashMap;
use std::marker::PhantomData;

use crate::{
    commands::{CommandQueue, ParallelCommandBuffers},
    system_param::SystemParamError,
    systems::System,
    task::TaskPool,
    world::World,
};

use super::{Priority, Scheduler, SystemId};

/// Describes what a system reads and writes.
///
/// Used for conflict detection to determine which systems can run in parallel.
#[derive(Clone, Default)]
pub struct SystemAccess {
    /// Component types this system reads (immutably)
    pub reads: Vec<std::any::TypeId>,
    /// Component types this system writes (mutably)
    pub writes: Vec<std::any::TypeId>,
    /// Whether this system uses Commands (writes to command queue)
    pub uses_commands: bool,
    /// Whether this system accesses the World exclusively
    pub exclusive: bool,
}

impl SystemAccess {
    /// Returns true if this access conflicts with another.
    ///
    /// Conflict occurs when:
    /// - Either system is exclusive
    /// - One system writes what another reads or writes
    pub fn conflicts_with(&self, other: &SystemAccess) -> bool {
        // Exclusive access conflicts with everything
        if self.exclusive || other.exclusive {
            return true;
        }

        // Check if self writes what other reads or writes
        for write in &self.writes {
            if other.reads.contains(write) || other.writes.contains(write) {
                return true;
            }
        }

        // Check if other writes what self reads
        for write in &other.writes {
            if self.reads.contains(write) {
                return true;
            }
        }

        false
    }
}

/// A system with its access metadata for parallel scheduling.
struct ParallelSystem {
    id: SystemId,
    system: Box<dyn System>,
    enabled: bool,
    /// Cached access information (computed once)
    access: SystemAccess,
}

/// A batch of systems that can run in parallel (no conflicts between them).
struct ParallelBatch {
    systems: Vec<usize>, // Indices into the pass's system list
}

/// A priority pass with parallel execution support.
struct ParallelPass {
    /// Event handler systems (run sequentially first)
    events: Vec<ParallelSystem>,
    /// Regular systems (can run in parallel)
    systems: Vec<ParallelSystem>,
    /// Pre-computed parallel batches (invalidated when systems change)
    batches: Option<Vec<ParallelBatch>>,
}

impl Default for ParallelPass {
    fn default() -> Self {
        Self {
            events: Vec::new(),
            systems: Vec::new(),
            batches: None,
        }
    }
}

impl ParallelPass {
    fn add(&mut self, id: SystemId, system: Box<dyn System>) {
        // For now, assume all systems have exclusive access (conservative)
        // TODO: Extract actual access from system parameters
        let access = SystemAccess {
            exclusive: true, // Conservative default
            ..Default::default()
        };

        let is_event_reader = system.is_event_reader();
        let parallel_sys = ParallelSystem {
            id,
            system,
            enabled: true,
            access,
        };

        if is_event_reader {
            self.events.push(parallel_sys);
        } else {
            self.systems.push(parallel_sys);
            self.batches = None; // Invalidate cached batches
        }
    }

    fn remove(&mut self, id: SystemId) -> bool {
        if let Some(pos) = self.events.iter().position(|s| s.id == id) {
            self.events.remove(pos);
            return true;
        }
        if let Some(pos) = self.systems.iter().position(|s| s.id == id) {
            self.systems.remove(pos);
            self.batches = None; // Invalidate batches
            return true;
        }
        false
    }

    fn set_enabled(&mut self, id: SystemId, enabled: bool) -> bool {
        for sys in self.events.iter_mut().chain(self.systems.iter_mut()) {
            if sys.id == id {
                let was_enabled = sys.enabled;
                sys.enabled = enabled;
                // Invalidate batches if enabled state of a regular system changed
                if was_enabled != enabled && self.systems.iter().any(|s| s.id == id) {
                    self.batches = None;
                }
                return true;
            }
        }
        false
    }

    fn is_enabled(&self, id: SystemId) -> Option<bool> {
        for sys in self.events.iter().chain(self.systems.iter()) {
            if sys.id == id {
                return Some(sys.enabled);
            }
        }
        None
    }

    fn count(&self) -> usize {
        self.events.len() + self.systems.len()
    }

    fn clear(&mut self) {
        self.events.clear();
        self.systems.clear();
        self.batches = None;
    }

    /// Compute parallel batches using a greedy algorithm.
    ///
    /// Systems are assigned to batches such that no two systems in the same
    /// batch have conflicting access. Only considers enabled systems.
    fn compute_batches(&mut self) {
        if self.batches.is_some() {
            return; // Already computed
        }

        // Only consider enabled systems for batching
        let enabled_indices: Vec<usize> = self
            .systems
            .iter()
            .enumerate()
            .filter(|(_, s)| s.enabled)
            .map(|(i, _)| i)
            .collect();

        let mut batches: Vec<ParallelBatch> = Vec::new();
        let mut assigned = vec![false; enabled_indices.len()];

        // Greedy batch assignment
        while assigned.iter().any(|&a| !a) {
            let mut batch = ParallelBatch {
                systems: Vec::new(),
            };
            let mut batch_access = SystemAccess::default();

            for (assigned_idx, &sys_idx) in enabled_indices.iter().enumerate() {
                if assigned[assigned_idx] {
                    continue;
                }

                let sys = &self.systems[sys_idx];

                // Empty batch can accept any system (including exclusive ones)
                // Non-empty batches must check for conflicts
                let can_add = batch.systems.is_empty() || !sys.access.conflicts_with(&batch_access);

                if can_add {
                    batch.systems.push(sys_idx);
                    assigned[assigned_idx] = true;

                    // Merge access info
                    batch_access.reads.extend(sys.access.reads.iter().cloned());
                    batch_access.writes.extend(sys.access.writes.iter().cloned());
                    batch_access.exclusive |= sys.access.exclusive;
                }
            }

            if !batch.systems.is_empty() {
                batches.push(batch);
            }
        }

        self.batches = Some(batches);
    }

    /// Run the pass sequentially (fallback when no task pool available).
    fn run_sequential(
        &mut self,
        world: &mut World,
        commands: &RefCell<CommandQueue>,
    ) -> Result<(), SystemParamError> {
        // Run enabled event handlers first
        for sys in &mut self.events {
            if sys.enabled {
                sys.system.run_with_commands(world, commands)?;
            }
        }
        // Then enabled regular systems
        for sys in &mut self.systems {
            if sys.enabled {
                sys.system.run_with_commands(world, commands)?;
            }
        }
        Ok(())
    }

    /// Run the pass with parallel execution where possible.
    ///
    /// Uses `ParallelCommandBuffers` so each system gets its own command buffer,
    /// avoiding contention during parallel execution.
    fn run_parallel(
        &mut self,
        world: &mut World,
        buffers: &ParallelCommandBuffers,
        _pool: &TaskPool,
    ) -> Result<(), SystemParamError> {
        // Compute batches if needed
        self.compute_batches();

        // Event handlers run sequentially (order matters for events)
        // They share a single buffer since they run one at a time
        for sys in &mut self.events {
            if sys.enabled {
                let buffer = buffers.claim();
                sys.system.run_with_buffer(world, buffer)?;
            }
        }

        // Run batches - each system in a batch uses its own buffer
        // NOTE: Still running sequentially within batches because systems need &mut World
        // True parallelism requires thread-safe World access (next step)
        if let Some(batches) = &self.batches {
            for batch in batches {
                // TODO: Replace this sequential loop with pool.scope() when
                // we have thread-safe World access
                for &idx in &batch.systems {
                    let buffer = buffers.claim();
                    self.systems[idx].system.run_with_buffer(world, buffer)?;
                }
            }
        }

        Ok(())
    }
}

/// Parallel scheduler that can run non-conflicting systems concurrently.
///
/// This scheduler uses access metadata to determine which systems can safely
/// run in parallel, then groups them into batches.
///
/// # Current Limitations
///
/// - Systems are currently assumed to have exclusive access (conservative)
/// - Actual parallel execution requires thread-safe World access (WIP)
/// - Event handlers always run sequentially to maintain ordering
///
/// # Future Enhancements
///
/// - Extract access metadata from system parameters automatically
/// - Thread-safe World access with fine-grained locking
/// - Per-thread command queues
/// - Parallel query iteration within systems
///
/// # Example
///
/// ```ignore
/// use nimbus_ecs::{GenericApp, SystemPriority};
/// use nimbus_ecs::scheduler::ParallelPriorityScheduler;
/// use nimbus_ecs::task::TaskPool;
///
/// // Create app with parallel scheduler
/// let mut app = GenericApp::<SystemPriority>::from_parts(
///     World::new(),
///     ParallelPriorityScheduler::new(),
/// );
///
/// // Insert task pool for parallel execution
/// app.insert_singleton(TaskPool::new());
///
/// // Register systems - non-conflicting ones will run in parallel
/// app.register_system(SystemPriority::Update, system_a);
/// app.register_system(SystemPriority::Update, system_b);
/// ```
pub struct ParallelPriorityScheduler<P: Priority> {
    passes: HashMap<P, ParallelPass>,
    /// Command queue for sequential execution (fallback)
    command_queue: RefCell<CommandQueue>,
    /// Command buffers for parallel execution (lazily initialized)
    parallel_buffers: Option<ParallelCommandBuffers>,
    _marker: PhantomData<P>,
}

impl<P: Priority> ParallelPriorityScheduler<P> {
    /// Creates a new parallel scheduler.
    pub fn new() -> Self {
        Self::default()
    }

    /// Ensures parallel command buffers are initialized for the given thread count.
    fn ensure_parallel_buffers(&mut self, thread_count: usize) {
        if self.parallel_buffers.is_none() {
            // +1 for main thread participation in work-stealing
            self.parallel_buffers = Some(ParallelCommandBuffers::new(thread_count + 1));
        }
    }
}

impl<P: Priority> Default for ParallelPriorityScheduler<P> {
    fn default() -> Self {
        let mut passes = HashMap::new();
        for &phase in P::phases() {
            passes.insert(phase, ParallelPass::default());
        }
        Self {
            passes,
            command_queue: RefCell::new(CommandQueue::new()),
            parallel_buffers: None,
            _marker: PhantomData,
        }
    }
}

impl<P: Priority> Scheduler<P> for ParallelPriorityScheduler<P> {
    fn add_system(&mut self, priority: P, system: Box<dyn System>) -> SystemId {
        let id = SystemId::new();
        self.passes.entry(priority).or_default().add(id, system);
        id
    }

    fn remove(&mut self, id: SystemId) -> bool {
        for pass in self.passes.values_mut() {
            if pass.remove(id) {
                return true;
            }
        }
        false
    }

    fn set_enabled(&mut self, id: SystemId, enabled: bool) -> bool {
        for pass in self.passes.values_mut() {
            if pass.set_enabled(id, enabled) {
                return true;
            }
        }
        false
    }

    fn is_enabled(&self, id: SystemId) -> Option<bool> {
        for pass in self.passes.values() {
            if let Some(enabled) = pass.is_enabled(id) {
                return Some(enabled);
            }
        }
        None
    }

    fn system_count(&self) -> usize {
        self.passes.values().map(|p| p.count()).sum()
    }

    fn clear(&mut self) {
        for pass in self.passes.values_mut() {
            pass.clear();
        }
    }

    fn run(&mut self, world: &mut World) -> Result<(), SystemParamError> {
        // Check if task pool exists and get thread count
        let pool_info = world.get_singleton::<TaskPool>().map(|p| {
            let count = p.thread_count();
            let pool = p.clone();
            (pool, count)
        });

        // Initialize parallel buffers if we have a pool (do this once, outside the loop)
        if let Some((_, thread_count)) = &pool_info {
            self.ensure_parallel_buffers(*thread_count);
        }

        // Run all phases in order
        for &phase in P::phases() {
            if let Some(pass) = self.passes.get_mut(&phase) {
                if let Some((ref pool, _)) = pool_info {
                    // Use parallel buffers
                    let buffers = self.parallel_buffers.as_ref().unwrap();
                    
                    // Reset buffers for this phase
                    buffers.reset();
                    
                    // Run systems (each claims its own buffer)
                    pass.run_parallel(world, buffers, pool)?;
                    
                    // Flush all command buffers
                    buffers.flush(world);
                } else {
                    // Sequential fallback
                    pass.run_sequential(world, &self.command_queue)?;
                    self.command_queue.borrow_mut().apply(world);
                }
            }
        }

        // Clear events after all phases
        world.event_storage.clear_all();

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Component, Query, SystemPriority};

    #[derive(Component)]
    struct Position {
        x: f32,
        y: f32,
    }

    #[derive(Component)]
    struct Velocity {
        x: f32,
        y: f32,
    }

    #[test]
    fn parallel_scheduler_basic() {
        let mut world = World::new();
        world.spawn_with((Position { x: 0.0, y: 0.0 }, Velocity { x: 1.0, y: 2.0 }));

        let mut scheduler: ParallelPriorityScheduler<SystemPriority> =
            ParallelPriorityScheduler::new();

        fn movement(mut query: Query<(&mut Position, &Velocity)>) {
            for (pos, vel) in query.iter() {
                pos.x += vel.x;
                pos.y += vel.y;
            }
        }

        // Ergonomic API - just pass the function directly
        scheduler.register(SystemPriority::Update, movement);
        scheduler.run(&mut world).unwrap();

        let mut query = world.query::<&Position>();
        let pos = query.iter().next().unwrap();
        assert_eq!((pos.x, pos.y), (1.0, 2.0));
    }

    #[test]
    fn parallel_scheduler_with_pool() {
        let mut world = World::new();
        world.insert_singleton(TaskPool::new());
        world.spawn_with(Position { x: 0.0, y: 0.0 });

        let mut scheduler: ParallelPriorityScheduler<SystemPriority> =
            ParallelPriorityScheduler::new();

        fn increment(mut query: Query<&mut Position>) {
            for pos in query.iter() {
                pos.x += 1.0;
            }
        }

        // Ergonomic API - just pass the function directly
        scheduler.register(SystemPriority::Update, increment);
        scheduler.run(&mut world).unwrap();

        let mut query = world.query::<&Position>();
        let pos = query.iter().next().unwrap();
        assert_eq!(pos.x, 1.0);
    }

    #[test]
    fn access_conflict_detection() {
        use std::any::TypeId;

        // Read-read: no conflict
        let read_a = SystemAccess {
            reads: vec![TypeId::of::<Position>()],
            ..Default::default()
        };
        let read_b = SystemAccess {
            reads: vec![TypeId::of::<Position>()],
            ..Default::default()
        };
        assert!(!read_a.conflicts_with(&read_b));

        // Read-write: conflict
        let write_a = SystemAccess {
            writes: vec![TypeId::of::<Position>()],
            ..Default::default()
        };
        assert!(read_a.conflicts_with(&write_a));
        assert!(write_a.conflicts_with(&read_a));

        // Write-write: conflict
        let write_b = SystemAccess {
            writes: vec![TypeId::of::<Position>()],
            ..Default::default()
        };
        assert!(write_a.conflicts_with(&write_b));

        // Different types: no conflict
        let write_vel = SystemAccess {
            writes: vec![TypeId::of::<Velocity>()],
            ..Default::default()
        };
        assert!(!write_a.conflicts_with(&write_vel));

        // Exclusive: always conflicts
        let exclusive = SystemAccess {
            exclusive: true,
            ..Default::default()
        };
        assert!(exclusive.conflicts_with(&read_a));
        assert!(exclusive.conflicts_with(&write_a));
    }

    use crate::systems::IntoSystem;

    #[test]
    fn batch_computation() {
        // Test that non-conflicting systems end up in the same batch
        let mut pass = ParallelPass::default();

        // Two systems with no conflicts (both just read Position)
        let access = SystemAccess {
            reads: vec![std::any::TypeId::of::<Position>()],
            exclusive: false,
            ..Default::default()
        };

        fn sys_a(mut _q: Query<&Position>) {}
        fn sys_b(mut _q: Query<&Position>) {}

        // Manually create systems with non-exclusive access
        pass.systems.push(ParallelSystem {
            id: SystemId::new(),
            system: Box::new(sys_a.into_system()),
            enabled: true,
            access: access.clone(),
        });
        pass.systems.push(ParallelSystem {
            id: SystemId::new(),
            system: Box::new(sys_b.into_system()),
            enabled: true,
            access,
        });

        pass.compute_batches();

        // Both should be in the same batch (no conflicts)
        let batches = pass.batches.as_ref().unwrap();
        assert_eq!(batches.len(), 1);
        assert_eq!(batches[0].systems.len(), 2);
    }

    #[test]
    fn system_enable_disable() {
        let mut world = World::new();
        world.spawn_with(Position { x: 0.0, y: 0.0 });

        let mut scheduler: ParallelPriorityScheduler<SystemPriority> =
            ParallelPriorityScheduler::new();

        fn increment(mut query: Query<&mut Position>) {
            for pos in query.iter() {
                pos.x += 1.0;
            }
        }

        let id = scheduler.register(SystemPriority::Update, increment);

        // Run once - should increment
        scheduler.run(&mut world).unwrap();
        let mut query = world.query::<&Position>();
        assert_eq!(query.iter().next().unwrap().x, 1.0);

        // Disable and run again - should NOT increment
        scheduler.set_enabled(id, false);
        scheduler.run(&mut world).unwrap();
        let mut query = world.query::<&Position>();
        assert_eq!(query.iter().next().unwrap().x, 1.0); // Still 1.0

        // Re-enable and run - should increment
        scheduler.set_enabled(id, true);
        scheduler.run(&mut world).unwrap();
        let mut query = world.query::<&Position>();
        assert_eq!(query.iter().next().unwrap().x, 2.0);
    }

    #[test]
    fn system_remove() {
        let mut world = World::new();
        world.spawn_with(Position { x: 0.0, y: 0.0 });

        let mut scheduler: ParallelPriorityScheduler<SystemPriority> =
            ParallelPriorityScheduler::new();

        fn increment(mut query: Query<&mut Position>) {
            for pos in query.iter() {
                pos.x += 1.0;
            }
        }

        let id = scheduler.register(SystemPriority::Update, increment);
        assert_eq!(scheduler.system_count(), 1);

        // Remove the system
        assert!(scheduler.remove(id));
        assert_eq!(scheduler.system_count(), 0);

        // Run - should do nothing
        scheduler.run(&mut world).unwrap();
        let mut query = world.query::<&Position>();
        assert_eq!(query.iter().next().unwrap().x, 0.0);

        // Can't remove twice
        assert!(!scheduler.remove(id));
    }
}

