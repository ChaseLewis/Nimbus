//! Plugin system for modular app configuration.
//!
//! Plugins allow bundling related functionality (systems, events, resources)
//! into reusable modules that can be added to an App.
//!
//! # Example
//! ```
//! use nimbus_ecs::{App, Plugin, Component, SystemPriority};
//!
//! // #[derive(Component)]  -- use this in your code
//! struct Gravity(f32);
//! # impl Component for Gravity {}
//!
//! struct PhysicsPlugin;
//!
//! impl Plugin for PhysicsPlugin {
//!     fn build(&self, app: &mut App) {
//!         // Add singleton resources
//!         app.world_mut().spawn_with(Gravity(9.81));
//!         
//!         // Register systems
//!         app.register_system(SystemPriority::Update, |_: nimbus_ecs::Query<&Gravity>| {
//!             // Physics update logic
//!         });
//!     }
//! }
//!
//! let mut app = App::new();
//! app.add_plugin(PhysicsPlugin);
//! ```

use crate::{scheduler::{Priority, SystemPriority}, App, GenericApp};

/// A modular unit of functionality that can be added to an App with default priorities.
///
/// This is a convenience trait for plugins that work with the default [`SystemPriority`].
/// For plugins that work with custom priority types, implement [`PluginExt`] instead.
///
/// Plugins are used to organize and bundle related features:
/// - Registering systems at various priorities
/// - Adding singleton resources/components
/// - Setting up event types
/// - Configuring initial world state
///
/// # Example
/// ```
/// use nimbus_ecs::{App, Plugin, SystemPriority};
///
/// struct MyPlugin {
///     config_value: i32,
/// }
///
/// impl Plugin for MyPlugin {
///     fn build(&self, app: &mut App) {
///         println!("Plugin initialized with config: {}", self.config_value);
///         // Add systems, resources, etc.
///     }
/// }
///
/// let mut app = App::new();
/// app.add_plugin(MyPlugin { config_value: 42 });
/// ```
pub trait Plugin {
    /// Called when the plugin is added to the app.
    ///
    /// Use this to register systems, add resources, and configure the world.
    fn build(&self, app: &mut App);
    
    /// Returns a name for this plugin, used for debugging.
    ///
    /// Defaults to the type name.
    fn name(&self) -> &str {
        std::any::type_name::<Self>()
    }
}

/// Generic plugin trait for apps with custom priority types.
///
/// Implement this trait when you need a plugin that works with a custom
/// priority enum instead of the default [`SystemPriority`].
///
/// # Example
/// ```
/// use nimbus_ecs::{GenericApp, scheduler::Priority};
/// use nimbus_ecs::plugin::PluginExt;
///
/// #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// enum GamePhase { Input, Physics, Render }
///
/// impl Priority for GamePhase {
///     fn phases() -> &'static [Self] {
///         &[Self::Input, Self::Physics, Self::Render]
///     }
/// }
///
/// struct PhysicsPlugin;
///
/// impl PluginExt<GamePhase> for PhysicsPlugin {
///     fn build(&self, app: &mut GenericApp<GamePhase>) {
///         // Register systems with custom phases
///         // app.register_system(GamePhase::Physics, physics_system);
///     }
/// }
/// ```
pub trait PluginExt<P: Priority> {
    /// Called when the plugin is added to the app.
    fn build(&self, app: &mut GenericApp<P>);
    
    /// Returns a name for this plugin, used for debugging.
    fn name(&self) -> &str {
        std::any::type_name::<Self>()
    }
}

// Implement PluginExt<SystemPriority> for all Plugin implementations
impl<T: Plugin> PluginExt<SystemPriority> for T {
    fn build(&self, app: &mut App) {
        Plugin::build(self, app);
    }
    
    fn name(&self) -> &str {
        Plugin::name(self)
    }
}

/// Implement Plugin for functions that take &mut App.
impl<F> Plugin for F
where
    F: Fn(&mut App),
{
    fn build(&self, app: &mut App) {
        self(app);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Component, Query, SystemPriority};
    use std::sync::{Arc, Mutex};

    #[derive(Component)]
    struct Position { x: f32, y: f32 }

    #[derive(Component)]
    #[allow(dead_code)]
    struct Velocity { x: f32, y: f32 }

    #[derive(Component)]
    #[allow(dead_code)]
    struct Counter(i32);

    #[test]
    fn plugin_can_spawn_entities() {
        struct SpawnPlugin;
        
        impl Plugin for SpawnPlugin {
            fn build(&self, app: &mut App) {
                app.world_mut().spawn_with(Position { x: 1.0, y: 2.0 });
                app.world_mut().spawn_with(Position { x: 3.0, y: 4.0 });
            }
        }

        let mut app = App::new();
        app.add_plugin(SpawnPlugin);

        let count = app.world_mut().query::<&Position>().iter().count();
        assert_eq!(count, 2);
    }

    #[test]
    fn plugin_can_register_systems() {
        struct SystemPlugin;

        impl Plugin for SystemPlugin {
            fn build(&self, app: &mut App) {
                app.register_system(SystemPriority::Update, |mut query: Query<&mut Position>| {
                    for pos in query.iter() {
                        pos.x += 1.0;
                        pos.y += 1.0;
                    }
                });
            }
        }

        let mut app = App::new();
        app.world_mut().spawn_with(Position { x: 0.0, y: 0.0 });
        app.add_plugin(SystemPlugin);
        
        assert_eq!(app.system_count(), 1);
        
        app.run().unwrap();
        
        let mut query = app.world_mut().query::<&Position>();
        let pos = query.iter().next().unwrap();
        assert_eq!((pos.x, pos.y), (1.0, 1.0));
    }

    #[test]
    fn plugin_can_add_singletons() {
        #[derive(Component)]
        struct GameConfig { difficulty: i32 }

        struct ConfigPlugin { difficulty: i32 }

        impl Plugin for ConfigPlugin {
            fn build(&self, app: &mut App) {
                app.world_mut().insert_singleton(GameConfig { difficulty: self.difficulty });
            }
        }

        let mut app = App::new();
        app.add_plugin(ConfigPlugin { difficulty: 5 });

        let config = app.world().get_singleton::<GameConfig>().unwrap();
        assert_eq!(config.difficulty, 5);
    }

    #[test]
    fn closure_plugin_works() {
        let mut app = App::new();
        
        app.add_plugin(|app: &mut App| {
            app.world_mut().spawn_with(Position { x: 42.0, y: 42.0 });
        });

        let mut query = app.world_mut().query::<&Position>();
        let pos = query.iter().next().unwrap();
        assert_eq!((pos.x, pos.y), (42.0, 42.0));
    }

    #[test]
    fn plugin_chaining_works() {
        struct PluginA;
        struct PluginB;

        impl Plugin for PluginA {
            fn build(&self, app: &mut App) {
                app.world_mut().spawn_with(Position { x: 1.0, y: 1.0 });
            }
        }

        impl Plugin for PluginB {
            fn build(&self, app: &mut App) {
                app.world_mut().spawn_with(Velocity { x: 2.0, y: 2.0 });
            }
        }

        let mut app = App::new();
        app.add_plugin(PluginA)
           .add_plugin(PluginB);

        assert_eq!(app.world_mut().query::<&Position>().iter().count(), 1);
        assert_eq!(app.world_mut().query::<&Velocity>().iter().count(), 1);
    }

    #[test]
    fn plugin_with_config() {
        struct SpawnNPlugin { count: usize }

        impl Plugin for SpawnNPlugin {
            fn build(&self, app: &mut App) {
                for i in 0..self.count {
                    app.world_mut().spawn_with(Counter(i as i32));
                }
            }
        }

        let mut app = App::new();
        app.add_plugin(SpawnNPlugin { count: 10 });

        assert_eq!(app.world_mut().query::<&Counter>().iter().count(), 10);
    }

    #[test]
    fn plugin_can_register_multiple_systems() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let log_clone = log.clone();

        struct MultiSystemPlugin {
            log: Arc<Mutex<Vec<&'static str>>>,
        }

        impl Plugin for MultiSystemPlugin {
            fn build(&self, app: &mut App) {
                let log1 = self.log.clone();
                let log2 = self.log.clone();

                app.register_system(SystemPriority::PreUpdate, move || {
                    log1.lock().unwrap().push("pre_update");
                });
                app.register_system(SystemPriority::Update, move || {
                    log2.lock().unwrap().push("update");
                });
            }
        }

        let mut app = App::new();
        app.add_plugin(MultiSystemPlugin { log: log_clone });
        
        assert_eq!(app.system_count(), 2);
        
        app.run().unwrap();
        
        let execution_order = log.lock().unwrap();
        assert_eq!(execution_order.as_slice(), &["pre_update", "update"]);
    }

    #[test]
    fn plugin_name_returns_type_name() {
        struct MyCustomPlugin;
        
        impl Plugin for MyCustomPlugin {
            fn build(&self, _app: &mut App) {}
        }

        let plugin = MyCustomPlugin;
        assert!(Plugin::name(&plugin).contains("MyCustomPlugin"));
    }

    #[test]
    fn generic_plugin_with_custom_priorities() {
        use crate::scheduler::Priority;

        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        enum CustomPhase {
            First,
            Second,
        }

        impl Priority for CustomPhase {
            fn phases() -> &'static [Self] {
                &[Self::First, Self::Second]
            }
        }

        struct CustomPlugin;

        impl PluginExt<CustomPhase> for CustomPlugin {
            fn build(&self, app: &mut GenericApp<CustomPhase>) {
                app.world_mut().spawn_with(Position { x: 99.0, y: 99.0 });
            }
        }

        let mut app: GenericApp<CustomPhase> = GenericApp::new();
        app.add_plugin(CustomPlugin);

        let count = app.world_mut().query::<&Position>().iter().count();
        assert_eq!(count, 1);
    }
}
