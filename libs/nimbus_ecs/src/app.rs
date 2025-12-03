//! Application container and world initialization trait.
//!
//! The `App` struct combines a `World` with a `PriorityScheduler` for
//! convenient game loop management. The `WorldInit` trait provides a
//! common interface for entity and singleton management.

use std::marker::PhantomData;

use crate::{
    bundle::Bundle,
    entity::Entity,
    plugin::PluginExt,
    scheduler::{Priority, PriorityScheduler, Scheduler, SystemId, SystemPriority},
    systems::IntoSystem,
    world::World,
    SystemParamError,
};

/// Trait for world initialization operations.
///
/// This trait provides a common interface for spawning entities and managing
/// singletons, implemented by both `World` and `App`.
pub trait WorldInit {
    /// Spawns a new entity with no components.
    fn spawn(&mut self) -> Entity;

    /// Spawns a new entity with the provided component bundle.
    fn spawn_with<B: Bundle + 'static>(&mut self, bundle: B) -> Entity;

    /// Spawns multiple entities from an iterator of bundles.
    fn spawn_batch<B, I>(&mut self, bundles: I) -> Vec<Entity>
    where
        B: Bundle + 'static,
        I: IntoIterator<Item = B>;

    /// Removes an entity and all of its components.
    fn despawn(&mut self, entity: Entity) -> bool;

    /// Inserts or replaces a singleton value.
    fn insert_singleton<T: 'static>(&mut self, value: T);

    /// Gets an immutable reference to a singleton value.
    fn get_singleton<T: 'static>(&self) -> Option<&T>;

    /// Gets a mutable reference to a singleton value.
    fn get_singleton_mut<T: 'static>(&mut self) -> Option<&mut T>;

    /// Removes a singleton value.
    fn remove_singleton<T: 'static>(&mut self) -> Option<T>;
}

// WorldInit is implemented for World in world.rs

/// Generic application container combining World and Scheduler.
///
/// `GenericApp` provides a convenient way to manage the game loop by combining
/// entity data (`World`) with system scheduling (`PriorityScheduler`).
///
/// # Type Parameter
///
/// - `P`: The priority enum type that defines execution phases.
///   Use [`SystemPriority`] for the default phases, or define your own.
///
/// # Example with Custom Priorities
///
/// ```
/// use nimbus_ecs::{GenericApp, WorldInit, scheduler::Priority};
///
/// #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// enum GamePhase {
///     Input,
///     Physics,
///     Render,
/// }
///
/// impl Priority for GamePhase {
///     fn phases() -> &'static [Self] {
///         &[Self::Input, Self::Physics, Self::Render]
///     }
/// }
///
/// let mut app: GenericApp<GamePhase> = GenericApp::new();
/// // app.register_system(GamePhase::Physics, physics_system);
/// ```
pub struct GenericApp<P: Priority> {
    /// The world containing all entity/component data.
    pub world: World,
    /// The scheduler containing all registered systems.
    pub scheduler: PriorityScheduler<P>,
    _marker: PhantomData<P>,
}

impl<P: Priority> GenericApp<P> {
    /// Creates a new App with an empty World and scheduler.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates an App from existing World and Scheduler.
    ///
    /// Useful for benchmarks or when migrating from manual world management.
    ///
    /// # Example
    ///
    /// ```
    /// use nimbus_ecs::{App, World};
    /// use nimbus_ecs::scheduler::PriorityScheduler;
    ///
    /// let world = World::new();
    /// let scheduler = PriorityScheduler::new();
    /// let app = App::from_parts(world, scheduler);
    /// ```
    pub fn from_parts(world: World, scheduler: PriorityScheduler<P>) -> Self {
        Self {
            world,
            scheduler,
            _marker: PhantomData,
        }
    }

    /// Returns an immutable reference to the world.
    #[inline]
    pub fn world(&self) -> &World {
        &self.world
    }

    /// Returns a mutable reference to the world.
    ///
    /// Use this for operations not covered by `WorldInit`, such as
    /// `insert`, `remove`, `query`, etc.
    #[inline]
    pub fn world_mut(&mut self) -> &mut World {
        &mut self.world
    }

    /// Registers a system to run at the given priority.
    ///
    /// Returns a [`SystemId`] that can be used to remove or enable/disable the system.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let physics_id = app.register_system(Update, physics_system);
    /// app.set_system_enabled(physics_id, false); // Disable for pause menu
    /// ```
    pub fn register_system<M, S: IntoSystem<M>>(
        &mut self,
        priority: P,
        system: S,
    ) -> SystemId {
        self.scheduler
            .add_system(priority, Box::new(system.into_system()))
    }

    /// Removes a system from the scheduler.
    ///
    /// Returns `true` if the system was found and removed, `false` otherwise.
    pub fn remove_system(&mut self, id: SystemId) -> bool {
        self.scheduler.remove(id)
    }

    /// Enables or disables a system.
    ///
    /// Disabled systems remain registered but are skipped during execution.
    /// This is more efficient than removing and re-adding systems that are
    /// frequently toggled (e.g., debug overlays, pause menu).
    ///
    /// Returns `true` if the system was found, `false` otherwise.
    pub fn set_system_enabled(&mut self, id: SystemId, enabled: bool) -> bool {
        self.scheduler.set_enabled(id, enabled)
    }

    /// Returns whether a system is enabled.
    ///
    /// Returns `None` if the system ID is not found.
    pub fn is_system_enabled(&self, id: SystemId) -> Option<bool> {
        self.scheduler.is_enabled(id)
    }

    /// Adds a plugin to configure the app.
    ///
    /// Plugins can register systems, add resources, and configure the world.
    /// Returns `&mut Self` for method chaining.
    pub fn add_plugin<Pl: PluginExt<P>>(&mut self, plugin: Pl) -> &mut Self {
        plugin.build(self);
        self
    }

    /// Runs all registered systems once in priority order.
    ///
    /// This is the main game loop tick - call this once per frame.
    pub fn run(&mut self) -> Result<(), SystemParamError> {
        self.scheduler.run(&mut self.world)?;
        self.world.tick += 1;
        Ok(())
    }

    /// Returns the total number of registered systems across all priorities.
    pub fn system_count(&self) -> usize {
        self.scheduler.system_count()
    }

    /// Clears all registered systems from all priority levels.
    pub fn clear_systems(&mut self) {
        self.scheduler.clear();
    }
}

impl<P: Priority> Default for GenericApp<P> {
    fn default() -> Self {
        Self {
            world: World::new(),
            scheduler: PriorityScheduler::new(),
            _marker: PhantomData,
        }
    }
}

// Implement WorldInit for GenericApp by delegating to the internal World
impl<P: Priority> WorldInit for GenericApp<P> {
    #[inline]
    fn spawn(&mut self) -> Entity {
        self.world.spawn()
    }

    #[inline]
    fn spawn_with<B: Bundle + 'static>(&mut self, bundle: B) -> Entity {
        self.world.spawn_with(bundle)
    }

    #[inline]
    fn spawn_batch<B, I>(&mut self, bundles: I) -> Vec<Entity>
    where
        B: Bundle + 'static,
        I: IntoIterator<Item = B>,
    {
        self.world.spawn_batch(bundles)
    }

    #[inline]
    fn despawn(&mut self, entity: Entity) -> bool {
        self.world.despawn(entity)
    }

    #[inline]
    fn insert_singleton<T: 'static>(&mut self, value: T) {
        self.world.insert_singleton(value)
    }

    #[inline]
    fn get_singleton<T: 'static>(&self) -> Option<&T> {
        self.world.get_singleton()
    }

    #[inline]
    fn get_singleton_mut<T: 'static>(&mut self) -> Option<&mut T> {
        self.world.get_singleton_mut()
    }

    #[inline]
    fn remove_singleton<T: 'static>(&mut self) -> Option<T> {
        self.world.remove_singleton()
    }
}

/// Application container using the default [`SystemPriority`] phases.
///
/// This is a type alias for `GenericApp<SystemPriority>`.
///
/// # Example
/// ```
/// use nimbus_ecs::{App, Component, Query, SystemPriority, WorldInit};
///
/// // #[derive(Component)]  -- use this in your code
/// struct Health(i32);
/// # impl nimbus_ecs::component::Component for Health {}
///
/// fn damage_system(mut query: Query<&mut Health>) {
///     for health in query.iter() {
///         health.0 -= 1;
///     }
/// }
///
/// let mut app = App::new();
/// app.spawn_with(Health(100));  // WorldInit method works directly on App
/// app.register_system(SystemPriority::Update, damage_system);
/// app.run().unwrap();
/// ```
pub type App = GenericApp<SystemPriority>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Component;

    #[derive(Component)]
    struct Position {
        x: f32,
        y: f32,
    }

    #[derive(Component)]
    struct GameConfig {
        difficulty: i32,
    }

    #[test]
    fn app_world_init_spawn() {
        let mut app = App::new();
        let entity = app.spawn();
        assert!(app.world().is_alive(entity));
    }

    #[test]
    fn app_world_init_spawn_with() {
        let mut app = App::new();
        let entity = app.spawn_with(Position { x: 1.0, y: 2.0 });
        assert!(app.world().is_alive(entity));

        let pos = app.world().get::<Position>(entity).unwrap();
        assert_eq!(pos.x, 1.0);
        assert_eq!(pos.y, 2.0);
    }

    #[test]
    fn app_world_init_spawn_batch() {
        let mut app = App::new();
        let positions = vec![
            Position { x: 0.0, y: 0.0 },
            Position { x: 1.0, y: 1.0 },
            Position { x: 2.0, y: 2.0 },
        ];
        let entities = app.spawn_batch(positions);
        assert_eq!(entities.len(), 3);

        for entity in entities {
            assert!(app.world().is_alive(entity));
        }
    }

    #[test]
    fn app_world_init_despawn() {
        let mut app = App::new();
        let entity = app.spawn();
        assert!(app.world().is_alive(entity));

        assert!(app.despawn(entity));
        assert!(!app.world().is_alive(entity));
    }

    #[test]
    fn app_world_init_singletons() {
        let mut app = App::new();

        // Insert
        app.insert_singleton(GameConfig { difficulty: 5 });

        // Get immutable
        let config = app.get_singleton::<GameConfig>().unwrap();
        assert_eq!(config.difficulty, 5);

        // Get mutable
        let config_mut = app.get_singleton_mut::<GameConfig>().unwrap();
        config_mut.difficulty = 10;

        // Verify change
        let config = app.get_singleton::<GameConfig>().unwrap();
        assert_eq!(config.difficulty, 10);

        // Remove
        let removed = app.remove_singleton::<GameConfig>().unwrap();
        assert_eq!(removed.difficulty, 10);

        // Verify removed
        assert!(app.get_singleton::<GameConfig>().is_none());
    }

    // Test custom priorities work
    #[test]
    fn generic_app_with_custom_priorities() {
        use crate::scheduler::Priority;

        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        enum CustomPhase {
            Early,
            Main,
            Late,
        }

        impl Priority for CustomPhase {
            fn phases() -> &'static [Self] {
                &[Self::Early, Self::Main, Self::Late]
            }
        }

        let mut app: GenericApp<CustomPhase> = GenericApp::new();
        app.spawn_with(Position { x: 0.0, y: 0.0 });

        fn movement_system(mut query: crate::Query<&mut Position>) {
            for pos in query.iter() {
                pos.x += 1.0;
            }
        }

        app.register_system(CustomPhase::Main, movement_system);
        app.run().unwrap();

        let mut query = app.world_mut().query::<&Position>();
        let pos = query.iter().next().unwrap();
        assert_eq!(pos.x, 1.0);
    }
}
