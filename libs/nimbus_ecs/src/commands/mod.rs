//! Deferred world mutations via Commands.
//!
//! Commands allow systems to queue structural changes to the world
//! (spawning, despawning, inserting/removing components) that are
//! applied after all systems in a priority level complete.
//!
//! This ensures iteration safety - you can't invalidate a query
//! while iterating over it.
//!
//! # Parallel Execution
//!
//! For parallel system execution, use [`ParallelCommandBuffers`] which
//! provides a pool of command queues that systems can claim atomically.

mod parallel;
mod queue;

pub use parallel::ParallelCommandBuffers;
pub use queue::{Command, CommandKind, CommandQueue};

use std::cell::RefCell;
use std::marker::PhantomData;

use crate::bundle::Bundle;
use crate::component::Component;
use crate::entity::Entity;
use crate::system_param::{SystemParam, SystemParamError};
use crate::world::UnsafeWorldCell;

// ============================================================================
// Commands SystemParam
// ============================================================================

/// System parameter for queueing deferred world mutations.
///
/// Commands are not applied immediately - they are queued and applied
/// after all systems in the current priority level complete.
///
/// # Example
/// ```
/// use nimbus_ecs::{Commands, Query, Entity, Component, ComponentId};
///
/// struct Position { x: f32, y: f32 }
/// impl Component for Position {
///     const COMPONENT_ID: ComponentId = ComponentId::new(0x1234);
/// }
/// struct Velocity { x: f32, y: f32 }
/// impl Component for Velocity {
///     const COMPONENT_ID: ComponentId = ComponentId::new(0x5678);
/// }
/// struct Health { value: i32 }
/// impl Component for Health {
///     const COMPONENT_ID: ComponentId = ComponentId::new(0x9ABC);
/// }
///
/// fn spawn_system(commands: Commands) {
///     // Queue a spawn - doesn't happen until after this system completes
///     commands.spawn((Position { x: 0.0, y: 0.0 }, Velocity { x: 1.0, y: 0.0 }));
/// }
///
/// fn cleanup_system(mut query: Query<(Entity, &Health)>, commands: Commands) {
///     for (entity, health) in query.iter() {
///         if health.value <= 0 {
///             commands.despawn(entity);
///         }
///     }
/// }
/// ```
pub struct Commands<'w> {
    queue: &'w RefCell<CommandQueue>,
}

impl<'w> Commands<'w> {
    /// Creates a new Commands with a reference to the command queue.
    pub(crate) fn new(queue: &'w RefCell<CommandQueue>) -> Self {
        Self { queue }
    }

    /// Spawns a new entity with the given bundle of components.
    /// 
    /// Note: Since spawning is deferred, the returned EntityCommands
    /// refers to a placeholder entity. For chaining operations on the
    /// spawned entity, use the callback-based spawn_with_callback instead.
    pub fn spawn<B: Bundle + Send + 'static>(&self, bundle: B) {
        self.queue.borrow_mut().push(SpawnCommand { bundle });
    }

    /// Spawns an empty entity with no components.
    pub fn spawn_empty(&self) -> EntityCommands<'w> {
        self.queue.borrow_mut().push(SpawnEmptyCommand);
        EntityCommands {
            entity: Entity::PLACEHOLDER,
            queue: self.queue,
            _marker: PhantomData,
        }
    }

    /// Gets an EntityCommands for an existing entity.
    pub fn entity(&self, entity: Entity) -> EntityCommands<'w> {
        EntityCommands {
            entity,
            queue: self.queue,
            _marker: PhantomData,
        }
    }

    /// Despawns an entity and all its components.
    pub fn despawn(&self, entity: Entity) {
        self.queue.borrow_mut().push(DespawnCommand(entity));
    }
}

// ============================================================================
// EntityCommands
// ============================================================================

/// Commands for a specific entity.
pub struct EntityCommands<'w> {
    entity: Entity,
    queue: &'w RefCell<CommandQueue>,
    _marker: PhantomData<&'w ()>,
}

impl<'w> EntityCommands<'w> {
    /// Returns the entity this commands refers to.
    pub fn id(&self) -> Entity {
        self.entity
    }

    /// Inserts a component on this entity.
    pub fn insert<T: Component>(&self, component: T) -> &Self {
        self.queue.borrow_mut().push(InsertCommand {
            entity: self.entity,
            component,
        });
        self
    }

    /// Inserts a bundle of components on this entity.
    /// 
    /// More efficient than multiple `insert` calls because it performs
    /// a single archetype migration instead of one per component.
    pub fn insert_bundle<B: Bundle + Send + 'static>(&self, bundle: B) -> &Self {
        self.queue.borrow_mut().push(InsertBundleCommand {
            entity: self.entity,
            bundle,
        });
        self
    }

    /// Removes a component from this entity.
    pub fn remove<T: Component>(&self) -> &Self {
        self.queue.borrow_mut().push(RemoveCommand::<T> {
            entity: self.entity,
            _marker: PhantomData,
        });
        self
    }

    /// Despawns this entity.
    pub fn despawn(&self) {
        self.queue.borrow_mut().push(DespawnCommand(self.entity));
    }
}

// ============================================================================
// Concrete Command Types
// ============================================================================

use std::any::TypeId;
use crate::world::World;

struct SpawnCommand<B: Bundle + Send> {
    bundle: B,
}

impl<B: Bundle + Send + 'static> Command for SpawnCommand<B> {
    fn apply_owned(self, world: &mut World) {
        world.spawn_with(self.bundle);
    }
    
    fn apply(self: Box<Self>, world: &mut World) {
        world.spawn_with(self.bundle);
    }
    
    fn kind(&self) -> CommandKind {
        CommandKind::Spawn
    }
}

struct SpawnEmptyCommand;

impl Command for SpawnEmptyCommand {
    fn apply_owned(self, world: &mut World) {
        world.spawn();
    }
    
    fn apply(self: Box<Self>, world: &mut World) {
        world.spawn();
    }
    
    fn kind(&self) -> CommandKind {
        CommandKind::Spawn
    }
}

struct DespawnCommand(Entity);

impl Command for DespawnCommand {
    fn apply_owned(self, world: &mut World) {
        world.despawn(self.0);
    }
    
    fn apply(self: Box<Self>, world: &mut World) {
        world.despawn(self.0);
    }
    
    fn kind(&self) -> CommandKind {
        CommandKind::Despawn
    }
}

struct InsertCommand<T: Component> {
    entity: Entity,
    component: T,
}

impl<T: Component> Command for InsertCommand<T> {
    fn apply_owned(self, world: &mut World) {
        let _ = world.insert(self.entity, self.component);
    }
    
    fn apply(self: Box<Self>, world: &mut World) {
        let _ = world.insert(self.entity, self.component);
    }
    
    fn kind(&self) -> CommandKind {
        CommandKind::InsertComponent(TypeId::of::<T>())
    }
}

struct InsertBundleCommand<B: Bundle + Send> {
    entity: Entity,
    bundle: B,
}

impl<B: Bundle + Send + 'static> Command for InsertBundleCommand<B> {
    fn apply_owned(self, world: &mut World) {
        let _ = world.insert_bundle(self.entity, self.bundle);
    }
    
    fn apply(self: Box<Self>, world: &mut World) {
        let _ = world.insert_bundle(self.entity, self.bundle);
    }
    
    fn kind(&self) -> CommandKind {
        CommandKind::InsertBundle(TypeId::of::<B>())
    }
    
    fn batch_apply_fn(&self) -> Option<unsafe fn(ptrs: &[*mut u8], world: &mut World)> {
        Some(batch_apply_insert_bundle::<B>)
    }
}

/// Batch apply function for InsertBundleCommand.
/// Reads all commands from pointers, extracts (entity, bundle) pairs, and calls batch insert.
/// 
/// # Safety
/// All pointers must point to valid InsertBundleCommand<B> values.
unsafe fn batch_apply_insert_bundle<B: Bundle + Send + 'static>(ptrs: &[*mut u8], world: &mut World) {
    // Extract (entity, bundle) pairs from command pointers
    let batch: Vec<(Entity, B)> = ptrs
        .iter()
        .map(|&ptr| {
            // Safety: caller guarantees ptr points to InsertBundleCommand<B>
            let cmd = unsafe { std::ptr::read(ptr as *const InsertBundleCommand<B>) };
            (cmd.entity, cmd.bundle)
        })
        .collect();
    
    // Use batch insert for better performance
    world.insert_bundle_batch(batch);
}

struct RemoveCommand<T: Component> {
    entity: Entity,
    _marker: PhantomData<T>,
}

impl<T: Component> Command for RemoveCommand<T> {
    fn apply_owned(self, world: &mut World) {
        world.remove::<T>(self.entity);
    }
    
    fn apply(self: Box<Self>, world: &mut World) {
        world.remove::<T>(self.entity);
    }
    
    fn kind(&self) -> CommandKind {
        CommandKind::RemoveComponent(TypeId::of::<T>())
    }
    
    fn batch_apply_fn(&self) -> Option<unsafe fn(ptrs: &[*mut u8], world: &mut World)> {
        Some(batch_apply_remove::<T>)
    }
}

/// Batch apply function for RemoveCommand.
/// Extracts entities and calls batch remove.
/// 
/// # Safety
/// All pointers must point to valid RemoveCommand<T> values.
unsafe fn batch_apply_remove<T: Component>(ptrs: &[*mut u8], world: &mut World) {
    let entities: Vec<Entity> = ptrs
        .iter()
        .map(|&ptr| {
            let cmd = unsafe { std::ptr::read(ptr as *const RemoveCommand<T>) };
            cmd.entity
        })
        .collect();
    
    world.remove_batch::<T>(&entities);
}

// ============================================================================
// SystemParam implementation
// ============================================================================

/// State for Commands - just a marker since queue comes from scheduler
#[derive(Default)]
pub struct CommandsState;

impl SystemParam for Commands<'_> {
    type State = CommandsState;
    type Item<'w, 's> = Commands<'w>;

    fn from_world_with_state<'w, 's>(
        world: UnsafeWorldCell<'w>,
        _state: &'s mut Self::State,
    ) -> Result<Self::Item<'w, 's>, SystemParamError> {
        // Get the command queue from the world
        Ok(Commands::new(world.command_queue()))
    }
}

