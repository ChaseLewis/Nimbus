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
use std::sync::atomic::{AtomicBool, Ordering};

use crate::{
    commands::{CommandQueue, ParallelCommandBuffers},
    component::ComponentId,
    parallel_world::ParallelWorldCell,
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
    pub reads: Vec<ComponentId>,
    /// Component types this system writes (mutably)
    pub writes: Vec<ComponentId>,
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
        // Extract actual access from system parameters
        let param_access = system.access();
        let access = SystemAccess {
            reads: param_access.reads,
            writes: param_access.writes,
            uses_commands: false, // Commands don't conflict - each system gets its own buffer
            exclusive: param_access.exclusive,
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
        pool: &TaskPool,
    ) -> Result<(), SystemParamError> {
        // Compute batches if needed
        self.compute_batches();

        // Event handlers run sequentially (order matters for events)
        for sys in &mut self.events {
            if sys.enabled {
                let buffer = buffers.claim();
                sys.system.run_with_buffer(world, buffer)?;
            }
        }

        // Run batches in parallel
        if let Some(batches) = &self.batches {
            // Create thread-safe world cell for parallel access
            let world_cell = ParallelWorldCell::new(world);
            let had_error = AtomicBool::new(false);

            for batch in batches {
                if batch.systems.len() == 1 {
                    // Single system - run directly (avoid scope overhead)
                    let idx = batch.systems[0];
                    let buffer = buffers.claim();
                    // SAFETY: Single system has exclusive access
                    let world_mut = unsafe { world_cell.world_mut() };
                    if self.systems[idx].system.run_with_buffer(world_mut, buffer).is_err() {
                        had_error.store(true, Ordering::Relaxed);
                    }
                } else {
                    // Multiple systems - run in parallel
                    // Wrap raw pointers in a Send wrapper for safe cross-thread transfer
                    struct SendSystemPtr(*mut Box<dyn System>);
                    // SAFETY: We ensure each pointer is only accessed by one thread
                    // and Systems are required to be Send
                    unsafe impl Send for SendSystemPtr {}
                    
                    impl SendSystemPtr {
                        unsafe fn get(&self) -> &mut Box<dyn System> {
                            unsafe { &mut *self.0 }
                        }
                    }

                    let system_ptrs: Vec<_> = batch.systems.iter()
                        .map(|&idx| SendSystemPtr(&mut self.systems[idx].system as *mut _))
                        .collect();

                    pool.scope(|s| {
                        for send_ptr in system_ptrs {
                            let world_cell = &world_cell;
                            let had_error = &had_error;
                            let buffers = &*buffers;

                            s.spawn(move |_| {
                                // Claim buffer inside the thread (buffers is Sync)
                                let buffer = buffers.claim();
                                
                                // SAFETY: 
                                // 1. Each system_ptr is unique (from different indices)
                                // 2. Access tracking ensures no component conflicts within batch
                                // 3. The scope ensures all spawned work completes before we continue
                                let system = unsafe { send_ptr.get() };
                                let world_mut = unsafe { world_cell.world_mut() };
                                
                                if system.run_with_buffer(world_mut, buffer).is_err() {
                                    had_error.store(true, Ordering::Relaxed);
                                }
                            });
                        }
                    });
                }

                if had_error.load(Ordering::Relaxed) {
                    // For now, just report a generic error
                    // In the future, we could collect and report specific errors
                    return Err(SystemParamError::SingletonNotFound { 
                        type_name: "parallel execution error" 
                    });
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

    /// Ensures parallel command buffers are initialized with enough capacity.
    /// 
    /// We need one buffer per system that could run (not per thread), since
    /// each system claims its own buffer regardless of which thread runs it.
    fn ensure_parallel_buffers(&mut self, min_capacity: usize) {
        match &self.parallel_buffers {
            None => {
                self.parallel_buffers = Some(ParallelCommandBuffers::new(min_capacity));
            }
            Some(existing) if existing.capacity() < min_capacity => {
                // Need to grow - replace with larger pool
                self.parallel_buffers = Some(ParallelCommandBuffers::new(min_capacity));
            }
            _ => {} // Already have enough capacity
        }
    }

    /// Returns the maximum number of systems that could claim buffers in a single phase.
    fn max_systems_per_phase(&self) -> usize {
        self.passes.values().map(|p| p.count()).max().unwrap_or(0)
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
        // Check if task pool exists
        let pool = world.get_singleton::<TaskPool>().cloned();

        // Initialize parallel buffers with enough capacity for all systems
        // We need one buffer per system (not per thread) since each system claims its own
        if pool.is_some() {
            let max_systems = self.max_systems_per_phase();
            // +1 for safety margin (e.g., if systems are added during run)
            self.ensure_parallel_buffers(max_systems + 1);
        }

        // Run all phases in order
        for &phase in P::phases() {
            if let Some(pass) = self.passes.get_mut(&phase) {
                if let Some(ref pool) = pool {
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
        use crate::component::ComponentId;
        
        // Helper to create ComponentId from TypeId for test types
        fn id_of<T: 'static>() -> ComponentId {
            ComponentId::from_type_id(TypeId::of::<T>())
        }

        // Read-read: no conflict
        let read_a = SystemAccess {
            reads: vec![id_of::<Position>()],
            ..Default::default()
        };
        let read_b = SystemAccess {
            reads: vec![id_of::<Position>()],
            ..Default::default()
        };
        assert!(!read_a.conflicts_with(&read_b));

        // Read-write: conflict
        let write_a = SystemAccess {
            writes: vec![id_of::<Position>()],
            ..Default::default()
        };
        assert!(read_a.conflicts_with(&write_a));
        assert!(write_a.conflicts_with(&read_a));

        // Write-write: conflict
        let write_b = SystemAccess {
            writes: vec![id_of::<Position>()],
            ..Default::default()
        };
        assert!(write_a.conflicts_with(&write_b));

        // Different types: no conflict
        let write_vel = SystemAccess {
            writes: vec![id_of::<Velocity>()],
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
        use std::any::TypeId;
        use crate::component::ComponentId;
        
        fn id_of<T: 'static>() -> ComponentId {
            ComponentId::from_type_id(TypeId::of::<T>())
        }
        
        // Test that non-conflicting systems end up in the same batch
        let mut pass = ParallelPass::default();

        // Two systems with no conflicts (both just read Position)
        let access = SystemAccess {
            reads: vec![id_of::<Position>()],
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

    // =========================================================================
    // Complex parallel execution tests
    // =========================================================================

    #[derive(Component)]
    struct Health(i32);

    #[derive(Component)]
    struct Mana(i32);

    #[derive(Component)]
    struct Stamina(i32);

    #[test]
    fn systems_with_different_components_batch_together() {
        // Systems accessing different components should be in the same batch
        let mut scheduler: ParallelPriorityScheduler<SystemPriority> =
            ParallelPriorityScheduler::new();

        fn read_position(_q: Query<&Position>) {}
        fn write_velocity(mut _q: Query<&mut Velocity>) {}
        fn write_health(mut _q: Query<&mut Health>) {}

        scheduler.register(SystemPriority::Update, read_position);
        scheduler.register(SystemPriority::Update, write_velocity);
        scheduler.register(SystemPriority::Update, write_health);

        // Get the pass and check batches
        let pass = scheduler.passes.get_mut(&SystemPriority::Update).unwrap();
        pass.compute_batches();
        
        let batches = pass.batches.as_ref().unwrap();
        
        // All three systems should be in ONE batch (no conflicts)
        // - read_position: reads Position
        // - write_velocity: writes Velocity
        // - write_health: writes Health
        assert_eq!(batches.len(), 1, "Non-conflicting systems should be in one batch");
        assert_eq!(batches[0].systems.len(), 3);
    }

    #[test]
    fn conflicting_systems_separate_batches() {
        // Systems with read-write conflict should be in different batches
        let mut scheduler: ParallelPriorityScheduler<SystemPriority> =
            ParallelPriorityScheduler::new();

        fn read_position(_q: Query<&Position>) {}
        fn write_position(mut _q: Query<&mut Position>) {}

        scheduler.register(SystemPriority::Update, read_position);
        scheduler.register(SystemPriority::Update, write_position);

        let pass = scheduler.passes.get_mut(&SystemPriority::Update).unwrap();
        pass.compute_batches();
        
        let batches = pass.batches.as_ref().unwrap();
        
        // Two batches needed: read and write can't run together
        assert_eq!(batches.len(), 2, "Conflicting systems should be in separate batches");
    }

    #[test]
    fn multiple_readers_same_batch() {
        // Multiple readers of the same component can run together
        let mut scheduler: ParallelPriorityScheduler<SystemPriority> =
            ParallelPriorityScheduler::new();

        fn read_position_1(_q: Query<&Position>) {}
        fn read_position_2(_q: Query<&Position>) {}
        fn read_position_3(_q: Query<&Position>) {}

        scheduler.register(SystemPriority::Update, read_position_1);
        scheduler.register(SystemPriority::Update, read_position_2);
        scheduler.register(SystemPriority::Update, read_position_3);

        let pass = scheduler.passes.get_mut(&SystemPriority::Update).unwrap();
        pass.compute_batches();
        
        let batches = pass.batches.as_ref().unwrap();
        
        // All readers should be in ONE batch
        assert_eq!(batches.len(), 1, "Multiple readers should be in one batch");
        assert_eq!(batches[0].systems.len(), 3);
    }

    #[test]
    fn complex_parallel_execution() {
        use std::sync::atomic::{AtomicI32, Ordering};
        use std::sync::Arc;

        // Create counters to track execution
        let health_counter = Arc::new(AtomicI32::new(0));
        let mana_counter = Arc::new(AtomicI32::new(0));
        let stamina_counter = Arc::new(AtomicI32::new(0));
        let total_counter = Arc::new(AtomicI32::new(0));

        let mut world = World::new();
        world.insert_singleton(TaskPool::new());

        // Spawn entities with various components
        for i in 0..100 {
            match i % 3 {
                0 => { world.spawn_with(Health(1)); }
                1 => { world.spawn_with(Mana(1)); }
                _ => { world.spawn_with(Stamina(1)); }
            }
        }

        let mut scheduler: ParallelPriorityScheduler<SystemPriority> =
            ParallelPriorityScheduler::new();

        // Create systems using closures that capture counters
        let hc = Arc::clone(&health_counter);
        let tc1 = Arc::clone(&total_counter);
        scheduler.register(SystemPriority::Update, move |mut q: Query<&Health>| {
            for h in q.iter() {
                hc.fetch_add(h.0, Ordering::Relaxed);
                tc1.fetch_add(1, Ordering::Relaxed);
            }
        });

        let mc = Arc::clone(&mana_counter);
        let tc2 = Arc::clone(&total_counter);
        scheduler.register(SystemPriority::Update, move |mut q: Query<&Mana>| {
            for m in q.iter() {
                mc.fetch_add(m.0, Ordering::Relaxed);
                tc2.fetch_add(1, Ordering::Relaxed);
            }
        });

        let sc = Arc::clone(&stamina_counter);
        let tc3 = Arc::clone(&total_counter);
        scheduler.register(SystemPriority::Update, move |mut q: Query<&Stamina>| {
            for s in q.iter() {
                sc.fetch_add(s.0, Ordering::Relaxed);
                tc3.fetch_add(1, Ordering::Relaxed);
            }
        });

        // Run the scheduler
        scheduler.run(&mut world).unwrap();

        // Verify all systems executed
        // 100 entities: ~33 Health, ~33 Mana, ~34 Stamina
        let health_sum = health_counter.load(Ordering::Relaxed);
        let mana_sum = mana_counter.load(Ordering::Relaxed);
        let stamina_sum = stamina_counter.load(Ordering::Relaxed);
        let total = total_counter.load(Ordering::Relaxed);

        assert!(health_sum > 0, "Health system should have run");
        assert!(mana_sum > 0, "Mana system should have run");
        assert!(stamina_sum > 0, "Stamina system should have run");
        assert_eq!(total, 100, "Should have processed 100 entities total");
        assert_eq!(health_sum + mana_sum + stamina_sum, 100);
    }

    #[test]
    fn parallel_execution_with_mutations() {
        let mut world = World::new();
        world.insert_singleton(TaskPool::new());

        // Spawn entities with different components
        for _ in 0..50 {
            world.spawn_with((Position { x: 0.0, y: 0.0 }, Velocity { x: 1.0, y: 2.0 }));
        }
        for _ in 0..50 {
            world.spawn_with(Health(100));
        }

        let mut scheduler: ParallelPriorityScheduler<SystemPriority> =
            ParallelPriorityScheduler::new();

        // System 1: Updates Position based on Velocity
        fn movement(mut query: Query<(&mut Position, &Velocity)>) {
            for (pos, vel) in query.iter() {
                pos.x += vel.x;
                pos.y += vel.y;
            }
        }

        // System 2: Decrements Health (independent of movement)
        fn damage(mut query: Query<&mut Health>) {
            for health in query.iter() {
                health.0 -= 1;
            }
        }

        scheduler.register(SystemPriority::Update, movement);
        scheduler.register(SystemPriority::Update, damage);

        // These should run in the same batch (different components)
        let pass = scheduler.passes.get_mut(&SystemPriority::Update).unwrap();
        pass.compute_batches();
        let batches = pass.batches.as_ref().unwrap();
        assert_eq!(batches.len(), 1, "movement and damage should batch together");

        // Reset batches for actual execution
        scheduler.passes.get_mut(&SystemPriority::Update).unwrap().batches = None;

        // Run multiple frames
        for _ in 0..10 {
            scheduler.run(&mut world).unwrap();
        }

        // Verify results
        let mut pos_query = world.query::<&Position>();
        for pos in pos_query.iter() {
            assert_eq!(pos.x, 10.0, "Position.x should be 10 after 10 frames");
            assert_eq!(pos.y, 20.0, "Position.y should be 20 after 10 frames");
        }

        let mut health_query = world.query::<&Health>();
        for health in health_query.iter() {
            assert_eq!(health.0, 90, "Health should be 90 after 10 damage ticks");
        }
    }
}

