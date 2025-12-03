//! EntityAPI - read-only random access to entity components.

use crate::component::Component;
use crate::entity::Entity;
use crate::world::UnsafeWorldCell;
use super::{SystemParam, SystemParamError};

/// System parameter for read-only random access to entity components.
///
/// Use this when you need to look up components on entities that aren't
/// part of your query (e.g., following entity references stored in components).
///
/// # Example
/// ```
/// use nimbus_ecs::{Query, Entity, Component, EntityAPI};
///
/// // #[derive(Component)]  -- use this in your code
/// struct Parent { entity: Option<Entity> }
/// # impl Component for Parent {}
/// // #[derive(Component)]
/// struct Name(String);
/// # impl Component for Name {}
///
/// fn print_parent_names(
///     mut query: Query<(&Parent,)>,
///     entities: EntityAPI,
/// ) {
///     for (parent,) in query.iter() {
///         if let Some(parent_entity) = parent.entity {
///             // Random access lookup on referenced entity
///             if let Some(name) = entities.get::<Name>(parent_entity) {
///                 println!("Parent name: {}", name.0);
///             }
///         }
///     }
/// }
/// ```
///
/// # Thread Safety
/// `EntityAPI` provides read-only access. For mutations, use [`Commands`](crate::commands::Commands)
/// which queues changes to be applied after the system completes.
pub struct EntityAPI<'w> {
    world: UnsafeWorldCell<'w>,
}

impl<'w> EntityAPI<'w> {
    /// Creates a new EntityAPI with a reference to the world.
    pub(crate) fn new(world: UnsafeWorldCell<'w>) -> Self {
        Self { world }
    }

    /// Returns `true` if the entity exists and is alive.
    #[inline]
    pub fn is_alive(&self, entity: Entity) -> bool {
        // Safety: read-only access to entity metadata
        unsafe { &*self.world.world() }.is_alive(entity)
    }

    /// Gets an immutable reference to a component on an entity.
    /// Returns `None` if the entity doesn't exist or doesn't have the component.
    #[inline]
    pub fn get<T: Component>(&self, entity: Entity) -> Option<&T> {
        // Safety: read-only access, Component requires Sync
        return self.world.get(entity);
    }

    /// Returns `true` if the entity has the specified component.
    #[inline]
    pub fn has<T: Component>(&self, entity: Entity) -> bool {
        self.get::<T>(entity).is_some()
    }

    /// Gets a component from an optional entity.
    /// Convenience method that handles `Option<Entity>` ergonomically.
    #[inline]
    pub fn get_opt<T: Component>(&self, entity: Option<Entity>) -> Option<&T> {
        entity.and_then(|e| self.get::<T>(e))
    }

    /// Returns `true` if the optional entity exists and has the component.
    #[inline]
    pub fn has_opt<T: Component>(&self, entity: Option<Entity>) -> bool {
        entity.map_or(false, |e| self.has::<T>(e))
    }

    /// Returns an EntityRef for validated entity access.
    /// Returns `None` if the entity doesn't exist.
    #[inline]
    pub fn entity(&self, entity: Entity) -> Option<EntityRef<'_, 'w>> {
        if self.is_alive(entity) {
            Some(EntityRef { entity, api: self })
        } else {
            None
        }
    }

    /// Returns an EntityRef from an optional entity.
    #[inline]
    pub fn entity_opt(&self, entity: Option<Entity>) -> Option<EntityRef<'_, 'w>> {
        entity.and_then(|e| self.entity(e))
    }
}

/// A validated reference to an entity, guaranteed to be alive at creation time.
///
/// Provides convenient access to multiple components without repeated validity checks.
/// Note: The entity could be despawned by commands before those commands are applied,
/// but within a single system run, the entity remains valid.
pub struct EntityRef<'a, 'w> {
    entity: Entity,
    api: &'a EntityAPI<'w>,
}

impl<'a, 'w> EntityRef<'a, 'w> {
    /// Returns the entity handle.
    #[inline]
    pub fn id(&self) -> Entity {
        self.entity
    }

    /// Gets an immutable reference to a component.
    #[inline]
    pub fn get<T: Component>(&self) -> Option<&T> {
        self.api.get::<T>(self.entity)
    }

    /// Returns `true` if this entity has the specified component.
    #[inline]
    pub fn has<T: Component>(&self) -> bool {
        self.api.has::<T>(self.entity)
    }
}

// ============================================================================
// SystemParam implementation
// ============================================================================

/// State for EntityAPI - empty since we just need world access.
#[derive(Default)]
pub struct EntityAPIState;

impl SystemParam for EntityAPI<'_> {
    type State = EntityAPIState;
    type Item<'w, 's> = EntityAPI<'w>;

    fn from_world_with_state<'w, 's>(
        world: UnsafeWorldCell<'w>,
        _state: &'s mut Self::State,
    ) -> Result<Self::Item<'w, 's>, SystemParamError> {
        Ok(EntityAPI::new(world))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::World;

    #[derive(Debug, PartialEq)]
    struct Position { x: f32, y: f32 }
    impl crate::component::Component for Position {}

    #[derive(Debug, PartialEq)]
    struct Health { value: i32 }
    impl crate::component::Component for Health {}

    #[derive(Debug)]
    #[allow(dead_code)]
    struct Target { entity: Option<Entity> }
    impl crate::component::Component for Target {}

    #[test]
    fn entity_api_get_component() {
        let mut world = World::new();
        let entity = world.spawn_with(Position { x: 1.0, y: 2.0 });
        
        // EntityAPI works with read-only world cell
        let api = EntityAPI::new(UnsafeWorldCell::new_readonly(&world));
        
        assert!(api.is_alive(entity));
        assert_eq!(api.get::<Position>(entity), Some(&Position { x: 1.0, y: 2.0 }));
        assert!(api.has::<Position>(entity));
        assert!(!api.has::<Health>(entity));
    }

    #[test]
    fn entity_api_optional_entity() {
        let mut world = World::new();
        let entity = world.spawn_with(Health { value: 100 });
        
        let api = EntityAPI::new(UnsafeWorldCell::new_readonly(&world));
        
        // With Some(entity)
        assert_eq!(api.get_opt::<Health>(Some(entity)), Some(&Health { value: 100 }));
        assert!(api.has_opt::<Health>(Some(entity)));
        
        // With None
        assert_eq!(api.get_opt::<Health>(None), None);
        assert!(!api.has_opt::<Health>(None));
    }

    #[test]
    fn entity_ref_multiple_lookups() {
        let mut world = World::new();
        let entity = world.spawn_with((Position { x: 1.0, y: 2.0 }, Health { value: 100 }));
        
        let api = EntityAPI::new(UnsafeWorldCell::new_readonly(&world));
        
        let entity_ref = api.entity(entity).expect("entity should exist");
        
        assert_eq!(entity_ref.id(), entity);
        assert_eq!(entity_ref.get::<Position>(), Some(&Position { x: 1.0, y: 2.0 }));
        assert_eq!(entity_ref.get::<Health>(), Some(&Health { value: 100 }));
        assert!(entity_ref.has::<Position>());
        assert!(entity_ref.has::<Health>());
        assert!(!entity_ref.has::<Target>());
    }

    #[test]
    fn entity_api_dead_entity() {
        let mut world = World::new();
        let entity = world.spawn_with(Position { x: 1.0, y: 2.0 });
        world.despawn(entity);
        
        let api = EntityAPI::new(UnsafeWorldCell::new_readonly(&world));
        
        assert!(!api.is_alive(entity));
        assert_eq!(api.get::<Position>(entity), None);
        assert!(api.entity(entity).is_none());
    }
}

