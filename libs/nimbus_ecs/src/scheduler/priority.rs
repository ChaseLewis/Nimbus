//! Priority-based system scheduler with automatic event handler grouping.
//!
//! Systems with `EventReader<T>` as their first parameter are automatically
//! grouped into event passes that run before regular systems at each priority.
//!
//! Execution order for each phase:
//! 1. Event handlers run first
//! 2. Regular systems run second
//! 3. Commands are flushed
//!
//! After all phases complete, events are cleared.

use std::cell::RefCell;
use std::collections::HashMap;
use std::marker::PhantomData;

use crate::{
    commands::CommandQueue,
    system_param::SystemParamError,
    systems::System,
    world::World,
};

use super::{Priority, Scheduler, SystemId};

/// A registered system with its metadata.
struct RegisteredSystem {
    id: SystemId,
    system: Box<dyn System>,
    enabled: bool,
}

/// A priority pass containing both event handlers and regular systems.
#[derive(Default)]
struct PriorityPass {
    /// Systems that read events (EventReader as first param).
    events: Vec<RegisteredSystem>,
    /// Regular systems.
    systems: Vec<RegisteredSystem>,
}

impl PriorityPass {
    fn add(&mut self, id: SystemId, system: Box<dyn System>) {
        let registered = RegisteredSystem {
            id,
            system,
            enabled: true,
        };
        if registered.system.is_event_reader() {
            self.events.push(registered);
        } else {
            self.systems.push(registered);
        }
    }

    fn remove(&mut self, id: SystemId) -> bool {
        if let Some(pos) = self.events.iter().position(|s| s.id == id) {
            self.events.remove(pos);
            return true;
        }
        if let Some(pos) = self.systems.iter().position(|s| s.id == id) {
            self.systems.remove(pos);
            return true;
        }
        false
    }

    fn set_enabled(&mut self, id: SystemId, enabled: bool) -> bool {
        for sys in self.events.iter_mut().chain(self.systems.iter_mut()) {
            if sys.id == id {
                sys.enabled = enabled;
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
    }

    /// Runs enabled event handlers then enabled regular systems.
    fn run(
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
}

/// Generic scheduler that organizes systems by priority level with automatic event grouping.
///
/// Systems with `EventReader<T>` as their first parameter are automatically
/// detected and run in a separate event pass before regular systems at each priority.
///
/// Execution order per frame:
/// 1. For each phase in `P::phases()`: run event handlers → run systems → flush commands
/// 2. Clear all events after the last phase
///
/// All passes within a frame can see same-frame events written by earlier systems!
///
/// # Type Parameter
///
/// - `P`: The priority enum type. Must implement [`Priority`].
///
/// # Example
///
/// ```
/// use nimbus_ecs::scheduler::{PriorityScheduler, SystemPriority};
///
/// // Using the default SystemPriority
/// let scheduler: PriorityScheduler<SystemPriority> = PriorityScheduler::new();
/// ```
pub struct PriorityScheduler<P: Priority> {
    passes: HashMap<P, PriorityPass>,
    command_queue: RefCell<CommandQueue>,
    _marker: PhantomData<P>,
}

impl<P: Priority> PriorityScheduler<P> {
    /// Creates a new empty priority scheduler.
    pub fn new() -> Self {
        Self::default()
    }
}

impl<P: Priority> Default for PriorityScheduler<P> {
    fn default() -> Self {
        let mut passes = HashMap::new();
        // Pre-create passes for all phases
        for &phase in P::phases() {
            passes.insert(phase, PriorityPass::default());
        }
        Self {
            passes,
            command_queue: RefCell::new(CommandQueue::new()),
            _marker: PhantomData,
        }
    }
}

impl<P: Priority> Scheduler<P> for PriorityScheduler<P> {
    fn add_system(&mut self, priority: P, system: Box<dyn System>) -> SystemId {
        let id = SystemId::new();
        self.passes
            .entry(priority)
            .or_default()
            .add(id, system);
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
        // Run all phases in order defined by Priority::phases()
        for &phase in P::phases() {
            if let Some(pass) = self.passes.get_mut(&phase) {
                pass.run(world, &self.command_queue)?;
                self.command_queue.borrow_mut().apply(world);
            }
        }

        // Clear events after all phases complete
        world.event_storage.clear_all();

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::super::SystemPriority;
    use crate::{App, Commands, Component, Entity, Query, Single};

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

    #[derive(Component)]
    struct Acceleration {
        x: f32,
        y: f32,
    }

    #[test]
    fn register_and_run_systems() {
        let mut app = App::new();

        // Create entity with position and velocity
        let entity = app.world.spawn_with((
            Position { x: 0.0, y: 0.0 },
            Velocity { x: 1.0, y: 2.0 },
            Acceleration { x: 0.1, y: 0.1 },
        ));

        // System that applies velocity to position
        fn apply_velocity(mut query: Query<(&mut Position, &Velocity)>) {
            for (pos, vel) in query.iter() {
                pos.x += vel.x;
                pos.y += vel.y;
            }
        }

        // System that applies acceleration to velocity
        fn apply_acceleration(mut query: Query<(&mut Velocity, &Acceleration)>) {
            for (vel, acc) in query.iter() {
                vel.x += acc.x;
                vel.y += acc.y;
            }
        }

        // Register systems - order matters!
        // Acceleration is applied first, then velocity
        app.register_system(SystemPriority::Update, apply_acceleration);
        app.register_system(SystemPriority::Update, apply_velocity);

        assert_eq!(app.system_count(), 2);

        // Run all systems once
        app.run().unwrap();

        // After first run:
        // - Acceleration applied: vel = (1.1, 2.1)
        // - Velocity applied: pos = (1.1, 2.1)
        let pos = app.world.get::<Position>(entity).unwrap();
        assert_eq!((pos.x, pos.y), (1.1, 2.1));

        let vel = app.world.get::<Velocity>(entity).unwrap();
        assert_eq!((vel.x, vel.y), (1.1, 2.1));

        // Run again
        app.run().unwrap();

        // After second run:
        // - Acceleration applied: vel = (1.2, 2.2)
        // - Velocity applied: pos = (1.1 + 1.2, 2.1 + 2.2) = (2.3, 4.3)
        let pos = app.world.get::<Position>(entity).unwrap();
        assert!((pos.x - 2.3).abs() < 0.001);
        assert!((pos.y - 4.3).abs() < 0.001);
    }

    #[test]
    fn clear_systems() {
        let mut app = App::new();

        fn dummy_system() {}

        app.register_system(SystemPriority::Update, dummy_system);
        app.register_system(SystemPriority::Update, dummy_system);
        assert_eq!(app.system_count(), 2);

        app.clear_systems();
        assert_eq!(app.system_count(), 0);
    }

    #[test]
    fn system_priority_ordering() {
        let mut app = App::new();

        // Track execution order using a shared vec
        #[derive(Component)]
        struct ExecutionLog {
            order: Arc<Mutex<Vec<&'static str>>>,
        }

        let log = Arc::new(Mutex::new(Vec::new()));
        app.world.insert_singleton(ExecutionLog { order: log.clone() });

        fn pre_update_system(log: Single<ExecutionLog>) {
            log.order.lock().unwrap().push("pre_update");
        }

        fn update_system(log: Single<ExecutionLog>) {
            log.order.lock().unwrap().push("update");
        }

        fn post_update_system(log: Single<ExecutionLog>) {
            log.order.lock().unwrap().push("post_update");
        }

        // Register in reverse order to prove priority matters, not registration order
        app.register_system(SystemPriority::PostUpdate, post_update_system);
        app.register_system(SystemPriority::PreUpdate, pre_update_system);
        app.register_system(SystemPriority::Update, update_system);

        assert_eq!(app.system_count(), 3);

        app.run().unwrap();

        // Systems should run in priority order regardless of registration order
        let execution_order = log.lock().unwrap();
        assert_eq!(
            execution_order.as_slice(),
            &["pre_update", "update", "post_update"]
        );
    }

    #[test]
    fn systems_within_same_priority_run_in_order() {
        let mut app = App::new();

        #[derive(Component)]
        struct ExecutionLog {
            order: Arc<Mutex<Vec<i32>>>,
        }

        let log = Arc::new(Mutex::new(Vec::new()));
        app.world.insert_singleton(ExecutionLog { order: log.clone() });

        fn system_1(log: Single<ExecutionLog>) {
            log.order.lock().unwrap().push(1);
        }

        fn system_2(log: Single<ExecutionLog>) {
            log.order.lock().unwrap().push(2);
        }

        fn system_3(log: Single<ExecutionLog>) {
            log.order.lock().unwrap().push(3);
        }

        // All registered to Update priority
        app.register_system(SystemPriority::Update, system_1);
        app.register_system(SystemPriority::Update, system_2);
        app.register_system(SystemPriority::Update, system_3);

        app.run().unwrap();

        // Should run in registration order within same priority
        let execution_order = log.lock().unwrap();
        assert_eq!(execution_order.as_slice(), &[1, 2, 3]);
    }

    #[test]
    fn commands_spawn_entities() {
        let mut app = App::new();

        fn spawn_system(commands: Commands) {
            commands.spawn(Position { x: 1.0, y: 2.0 });
            commands.spawn(Position { x: 3.0, y: 4.0 });
        }

        app.register_system(SystemPriority::Update, spawn_system);
        app.run().unwrap();

        // Entities should now exist
        let positions: Vec<(f32, f32)> = app.world
            .query::<&Position>()
            .iter()
            .map(|p| (p.x, p.y))
            .collect();
        
        assert_eq!(positions.len(), 2);
        assert!(positions.contains(&(1.0, 2.0)));
        assert!(positions.contains(&(3.0, 4.0)));
    }

    #[test]
    fn commands_despawn_entities() {
        let mut app = App::new();

        // Spawn some entities first
        let e1 = app.world.spawn_with(Position { x: 1.0, y: 1.0 });
        let e2 = app.world.spawn_with(Position { x: 2.0, y: 2.0 });
        let e3 = app.world.spawn_with(Position { x: 3.0, y: 3.0 });

        assert_eq!(app.world.query::<&Position>().iter().count(), 3);

        // System that despawns entity with x == 2.0
        fn despawn_system(mut query: Query<(Entity, &Position)>, commands: Commands) {
            for (entity, pos) in query.iter() {
                if pos.x == 2.0 {
                    commands.despawn(entity);
                }
            }
        }

        app.register_system(SystemPriority::Update, despawn_system);
        app.run().unwrap();

        // Only 2 entities should remain
        assert_eq!(app.world.query::<&Position>().iter().count(), 2);
        assert!(app.world.get::<Position>(e1).is_some());
        assert!(app.world.get::<Position>(e2).is_none()); // Despawned
        assert!(app.world.get::<Position>(e3).is_some());
    }

    #[test]
    fn commands_insert_component() {
        let mut app = App::new();
        let entity = app.world.spawn_with(Position { x: 1.0, y: 1.0 });

        fn add_velocity_system(mut query: Query<Entity>, commands: Commands) {
            for entity in query.iter() {
                commands.entity(entity).insert(Velocity { x: 5.0, y: 5.0 });
            }
        }

        app.register_system(SystemPriority::Update, add_velocity_system);
        app.run().unwrap();

        // Entity should now have velocity
        let vel = app.world.get::<Velocity>(entity).unwrap();
        assert_eq!((vel.x, vel.y), (5.0, 5.0));
    }

    #[test]
    fn commands_remove_component() {
        let mut app = App::new();
        let entity = app.world.spawn_with((
            Position { x: 1.0, y: 1.0 },
            Velocity { x: 5.0, y: 5.0 },
        ));

        assert!(app.world.get::<Position>(entity).is_some());
        assert!(app.world.get::<Velocity>(entity).is_some());

        // Query for entities with both Position and Velocity
        fn remove_velocity_system(mut query: Query<(Entity, &Position, &Velocity)>, commands: Commands) {
            for (entity, _pos, _vel) in query.iter() {
                commands.entity(entity).remove::<Velocity>();
            }
        }

        app.register_system(SystemPriority::Update, remove_velocity_system);
        app.run().unwrap();

        // Velocity should be removed, Position should remain
        assert!(app.world.get::<Position>(entity).is_some());
        assert!(app.world.get::<Velocity>(entity).is_none());
    }

    #[test]
    fn commands_flushed_between_priorities() {
        let mut app = App::new();

        // PreUpdate: spawn an entity
        fn spawn_system(commands: Commands) {
            commands.spawn(Position { x: 42.0, y: 42.0 });
        }

        // Update: count entities (should see the spawned one)
        #[derive(Component)]
        struct EntityCount(usize);

        fn count_system(mut query: Query<&Position>, commands: Commands) {
            let count = query.iter().count();
            commands.spawn(EntityCount(count));
        }

        app.register_system(SystemPriority::PreUpdate, spawn_system);
        app.register_system(SystemPriority::Update, count_system);
        app.run().unwrap();

        // The count system should have seen 1 entity (spawned in PreUpdate)
        let count = app.world.query::<&EntityCount>().iter().next().unwrap().0;
        assert_eq!(count, 1);
    }

    // =========================================================================
    // Event tests
    // =========================================================================

    use crate::{EventReader, EventWriter};

    #[derive(Clone, Debug)]
    struct DamageEvent {
        amount: i32,
    }

    #[test]
    fn events_write_and_read_same_frame() {
        let mut app = App::new();
        
        #[derive(Component)]
        struct TotalDamage(i32);
        
        app.world.spawn_with(TotalDamage(0));
        
        // System that sends events (runs in Update)
        fn send_damage(events: EventWriter<DamageEvent>) {
            events.send(DamageEvent { amount: 10 });
            events.send(DamageEvent { amount: 20 });
        }
        
        // System that reads events (runs in PostUpdate, same frame!)
        fn read_damage(events: EventReader<DamageEvent>, mut query: Query<&mut TotalDamage>) {
            for event in events.iter() {
                for total in query.iter() {
                    total.0 += event.amount;
                }
            }
        }
        
        app.register_system(SystemPriority::Update, send_damage);
        app.register_system(SystemPriority::PostUpdate, read_damage);
        
        // First frame: send and read in same frame!
        app.run().unwrap();
        assert_eq!(app.world.query::<&TotalDamage>().iter().next().unwrap().0, 30);
        
        // Second frame: same thing, total is now 60
        app.run().unwrap();
        assert_eq!(app.world.query::<&TotalDamage>().iter().next().unwrap().0, 60);
    }

    #[test]
    fn events_cleared_after_end_of_frame() {
        let mut app = App::new();
        
        #[derive(Component)]
        struct EventCount(usize);
        
        app.world.spawn_with(EventCount(0));
        
        fn send_once(events: EventWriter<DamageEvent>) {
            events.send(DamageEvent { amount: 5 });
        }
        
        fn count_events(events: EventReader<DamageEvent>, mut query: Query<&mut EventCount>) {
            let count = events.len();
            for ec in query.iter() {
                ec.0 += count;
            }
        }
        
        app.register_system(SystemPriority::PreUpdate, send_once);
        app.register_system(SystemPriority::EndOfFrame, count_events);
        
        // Frame 1: send in PreUpdate, count in EndOfFrame sees it (same frame)
        app.run().unwrap();
        assert_eq!(app.world.query::<&EventCount>().iter().next().unwrap().0, 1);
        
        // Frame 2: same thing, events were cleared, count sees new event
        app.run().unwrap();
        assert_eq!(app.world.query::<&EventCount>().iter().next().unwrap().0, 2);
        
        // Frame 3: same
        app.run().unwrap();
        assert_eq!(app.world.query::<&EventCount>().iter().next().unwrap().0, 3);
    }

    #[test]
    fn events_not_visible_next_frame() {
        let mut app = App::new();
        
        #[derive(Component)]
        struct EventCount(usize);
        
        app.world.spawn_with(EventCount(0));
        
        fn count_events(events: EventReader<DamageEvent>, mut query: Query<&mut EventCount>) {
            let count = events.len();
            for ec in query.iter() {
                ec.0 += count;
            }
        }
        
        app.register_system(SystemPriority::Update, count_events);
        
        // Frame 1: manually send event, count sees it
        {
            let queue = app.world.event_storage.ensure_queue::<DamageEvent>();
            queue.borrow_mut().as_any_mut()
                .downcast_mut::<crate::events::Events<DamageEvent>>()
                .unwrap()
                .send(DamageEvent { amount: 5 });
        }
        app.run().unwrap();
        assert_eq!(app.world.query::<&EventCount>().iter().next().unwrap().0, 1);
        
        // Frame 2: no new events, count is still 1 (events were cleared)
        app.run().unwrap();
        assert_eq!(app.world.query::<&EventCount>().iter().next().unwrap().0, 1);
    }

    #[test]
    fn event_handlers_run_before_regular_systems() {
        use std::sync::{Arc, Mutex};

        let mut app = App::new();

        #[derive(Component)]
        struct ExecutionLog {
            order: Arc<Mutex<Vec<&'static str>>>,
        }

        let log = Arc::new(Mutex::new(Vec::new()));
        app.world.insert_singleton(ExecutionLog { order: log.clone() });

        // Event handler (has EventReader as first param)
        fn event_handler(_events: EventReader<DamageEvent>, log: Single<ExecutionLog>) {
            log.order.lock().unwrap().push("event_handler");
        }

        // Regular system (no EventReader)
        fn regular_system(log: Single<ExecutionLog>) {
            log.order.lock().unwrap().push("regular_system");
        }

        // Register in reverse order to prove auto-grouping works
        app.register_system(SystemPriority::Update, regular_system);
        app.register_system(SystemPriority::Update, event_handler);

        app.run().unwrap();

        // Event handler should run before regular system despite registration order
        let execution_order = log.lock().unwrap();
        assert_eq!(
            execution_order.as_slice(),
            &["event_handler", "regular_system"]
        );
    }

    #[test]
    fn same_frame_event_propagation() {
        // After StartOfFrame clear, events written in PreUpdate are
        // immediately visible to Update event handlers (same frame!)
        let mut app = App::new();

        #[derive(Component)]
        struct EventsSeen(i32);

        app.world.spawn_with(EventsSeen(0));

        // PreUpdate sends events
        fn send_events(events: EventWriter<DamageEvent>) {
            events.send(DamageEvent { amount: 100 });
        }

        // Update reads events (same frame - single buffered!)
        fn read_events(events: EventReader<DamageEvent>, mut query: Query<&mut EventsSeen>) {
            for event in events.iter() {
                for seen in query.iter() {
                    seen.0 += event.amount;
                }
            }
        }

        app.register_system(SystemPriority::PreUpdate, send_events);
        app.register_system(SystemPriority::Update, read_events);

        // First frame: PreUpdate sends, Update sees it immediately!
        app.run().unwrap();
        assert_eq!(app.world.query::<&EventsSeen>().iter().next().unwrap().0, 100);

        // Second frame: events were cleared after StartOfFrame,
        // but PreUpdate sends again and Update sees it
        app.run().unwrap();
        assert_eq!(app.world.query::<&EventsSeen>().iter().next().unwrap().0, 200);
    }

    // =========================================================================
    // Custom priority tests
    // =========================================================================

    use super::super::Priority;

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    enum CustomPhase {
        Input,
        Physics,
        Render,
    }

    impl Priority for CustomPhase {
        fn phases() -> &'static [Self] {
            &[Self::Input, Self::Physics, Self::Render]
        }
    }

    #[test]
    fn custom_priority_enum() {
        use super::PriorityScheduler;
        use crate::scheduler::Scheduler;

        let mut scheduler: PriorityScheduler<CustomPhase> = PriorityScheduler::new();
        
        // This would be the system registration pattern
        // For now just verify the scheduler compiles with custom priority
        assert_eq!(scheduler.system_count(), 0);
        scheduler.clear();
    }
}
