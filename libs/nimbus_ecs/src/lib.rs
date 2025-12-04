#![doc = "Bevy-inspired Entity Component System foundations."]

mod app;
pub(crate) mod archetype;
mod bundle;
mod commands;
pub mod component;
mod entity;
pub mod events;
mod parallel_world;
pub mod plugin;
pub mod scheduler;
pub mod serialization;
mod system_param;
mod systems;
pub mod task;
pub(crate) mod util;
pub mod world;

pub use app::{App, GenericApp, WorldInit};
pub use archetype::ArchetypeKey;
pub use bundle::Bundle;
pub use commands::{Commands, ParallelCommandBuffers};
pub use entity::Entity;
pub use events::{EventReader, EventWriter};
pub use parallel_world::{ParallelWorldCell, ParamAccess};
pub use plugin::Plugin;
pub use scheduler::{Priority, Scheduler, SystemId, SystemPriority};
pub use serialization::{
    ComponentRegistration, ComponentRegistry,
    serialize, deserialize,
};
pub use system_param::{EntityAPI, EntityRef, Query, QueryFilter, QueryIter, Single, SingleMut, SystemParamError, Tasks, With, Without};
pub use systems::{IntoSystem, System};
pub use world::{World, WorldError};

// Export Component trait
pub use component::Component;

// Export macros: both attribute macro and legacy derive
pub use nimbus_macro_ecs::component;
pub use nimbus_macro_ecs::Component;

// Re-export inventory for macro use
pub use inventory;
