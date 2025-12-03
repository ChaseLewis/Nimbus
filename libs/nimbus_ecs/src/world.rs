use std::{
    any::TypeId,
    cell::RefCell,
    fmt,
    ops::{Deref, DerefMut},
};

use crate::{
    archetype::{ArchetypeKey, Archetypes},
    bundle::Bundle,
    commands::CommandQueue,
    component::Component,
    entity::{Entities, Entity, EntityLocation},
    system_param::{Query, QueryFilter, QueryParam, SystemParamError},
    systems::{IntoSystem, System},
    util::TypeHashMap,
};

/// Provides interior mutability access to the World for system parameters.
#[derive(Clone, Copy)]
pub struct UnsafeWorldCell<'w> {
    world: &'w World,
    command_queue: Option<&'w RefCell<CommandQueue>>,
}

impl<'w> UnsafeWorldCell<'w> {
    /// Creates a new UnsafeWorldCell with a command queue for system execution.
    pub fn new(world: &'w World, command_queue: &'w RefCell<CommandQueue>) -> Self {
        Self { world, command_queue: Some(command_queue) }
    }

    /// Creates a new UnsafeWorldCell without a command queue (for direct queries).
    pub fn new_readonly(world: &'w World) -> Self {
        Self { world, command_queue: None }
    }

    /// Returns a raw pointer to the world.
    /// 
    /// # Safety
    /// The returned pointer is valid for the lifetime 'w.
    /// Caller must ensure proper synchronization if used across threads.
    #[inline]
    pub fn world(&self) -> *const World {
        self.world
    }

    pub fn as_mut(self) -> &'w mut World {
        let ptr = self.world as *const World as *mut World;
        unsafe { ptr.as_mut().unwrap() }
    }

    /// Returns a reference to the command queue for deferred mutations.
    /// Panics if called when no command queue is available (e.g., from direct queries).
    pub fn command_queue(&self) -> &'w RefCell<CommandQueue> {
        self.command_queue.expect("Commands not available in this context (use within a system)")
    }
}

impl<'w> Deref for UnsafeWorldCell<'w> {
    type Target = World;

    fn deref(&self) -> &Self::Target {
        self.world
    }
}

impl<'w> DerefMut for UnsafeWorldCell<'w> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        let ptr: *mut World = self.world as *const World as *mut World;
        unsafe { ptr.as_mut().unwrap() }
    }
}

/// Errors that can occur when interacting with the world.
#[derive(Debug, Clone)]
pub enum WorldError {
    /// The referenced entity is not currently alive.
    EntityNotAlive(Entity),
}

impl fmt::Display for WorldError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WorldError::EntityNotAlive(entity) => {
                write!(f, "entity {entity} is no longer alive")
            }
        }
    }
}

impl std::error::Error for WorldError {}

type Singletons = TypeHashMap<Box<dyn std::any::Any>>;

/// Central storage for entities and their components.
/// 
/// Components are stored in archetypes - groups of entities with the same
/// component signature. This enables cache-efficient iteration.
/// 
/// For scheduling systems, use [`App`] which combines a World with a scheduler.
pub struct World {
    entities: Entities,
    archetypes: Archetypes,
    singletons: Singletons,
    /// Cache: Bundle TypeId -> archetype index for fast repeated spawns
    bundle_archetype_cache: TypeHashMap<usize>,
    /// Event storage for double-buffered events
    pub(crate) event_storage: crate::events::EventStorage,
    /// Number of times `run()` has been called
    pub(crate) tick: u64,
}

impl World {
    /// Creates an empty world.
    pub fn new() -> Self {
        Self::default()
    }

    /// Spawns a new entity with no components.
    pub fn spawn(&mut self) -> Entity {
        let entity = self.entities.alloc();
        let archetype_index = self.archetypes.ensure(ArchetypeKey::default());
        let row = self.archetypes.get_mut(archetype_index).unwrap().push_entity(entity);
        self.entities.set_location(entity, EntityLocation { archetype: archetype_index, slot: row });
        entity
    }

    /// Spawns a new entity and immediately inserts the provided component(s).
    pub fn spawn_with<B: Bundle + 'static>(&mut self, bundle: B) -> Entity {
        let entity = self.entities.alloc();
        
        // Fast path: check cache for this bundle type's archetype
        let bundle_type_id = TypeId::of::<B>();
        let archetype_index = if let Some(&cached_index) = self.bundle_archetype_cache.get(&bundle_type_id) {
            cached_index
        } else {
            // Slow path: build key and ensure archetype exists, then cache
            let types = B::type_ids();
            let key = ArchetypeKey::new(types);
            let index = self.archetypes.ensure(key);
            self.bundle_archetype_cache.insert(bundle_type_id, index);
            index
        };
        
        // Push entity first (columns will be populated by bundle.insert)
        let row = self.archetypes.get_mut(archetype_index).unwrap().push_entity(entity);
        self.entities.set_location(entity, EntityLocation { archetype: archetype_index, slot: row });
        
        // Now insert components - they go into the archetype columns
        bundle.insert(entity, self);
        
        entity
    }

    /// Spawns multiple entities from an iterator of bundles.
    /// 
    /// More efficient than calling `spawn_with` repeatedly because:
    /// - Single archetype lookup (cached)
    /// - Batch entity allocation
    /// - Direct column insertions
    /// 
    /// Returns a Vec of the spawned entities.
    /// 
    /// # Example
    /// ```
    /// use nimbus_ecs::{World, Component};
    ///
    /// // #[derive(Component)]  -- use this in your code
    /// struct Position { x: f32, y: f32 }
    /// # impl Component for Position {}
    ///
    /// let mut world = World::new();
    /// let positions = vec![
    ///     Position { x: 0.0, y: 0.0 },
    ///     Position { x: 1.0, y: 1.0 },
    ///     Position { x: 2.0, y: 2.0 },
    /// ];
    /// let entities = world.spawn_batch(positions);
    /// assert_eq!(entities.len(), 3);
    /// ```
    pub fn spawn_batch<B, I>(&mut self, bundles: I) -> Vec<Entity>
    where
        B: Bundle + 'static,
        I: IntoIterator<Item = B>,
    {
        //Taking a slice here might be more efficient
        //but we could support from_iter also, but in generally reallocating here seems spurious
        let bundles: Vec<B> = bundles.into_iter().collect();
        if bundles.is_empty() {
            return Vec::new();
        }

        // Get/cache archetype once for all spawns
        let bundle_type_id = TypeId::of::<B>();
        let archetype_index = if let Some(&cached_index) = self.bundle_archetype_cache.get(&bundle_type_id) {
            cached_index
        } else {
            let types = B::type_ids();
            let key = ArchetypeKey::new(types);
            let index = self.archetypes.ensure(key);
            self.bundle_archetype_cache.insert(bundle_type_id, index);
            index
        };

        // Allocate all entities and set up their locations
        let mut entities = Vec::with_capacity(bundles.len());
        for _ in 0..bundles.len() {
            let entity = self.entities.alloc();
            let row = self.archetypes.get_mut(archetype_index).unwrap().push_entity(entity);
            self.entities.set_location(entity, EntityLocation { archetype: archetype_index, slot: row });
            entities.push(entity);
        }

        // Insert all components
        for (entity, bundle) in entities.iter().zip(bundles) {
            bundle.insert(*entity, self);
        }

        entities
    }

    /// Removes an entity and all of its components.
    pub fn despawn(&mut self, entity: Entity) -> bool {
        if !self.entities.is_alive(entity) {
            return false;
        }

        // Remove from archetype (swap-remove handles column cleanup)
        if let Some(location) = self.entities.location(entity) {
            if let Some(archetype) = self.archetypes.get_mut(location.archetype) {
                if let Some(swapped_entity) = archetype.swap_remove(location.slot) {
                    // Update the swapped entity's location
                    self.entities.set_location(
                        swapped_entity,
                        EntityLocation {
                            archetype: location.archetype,
                            slot: location.slot,
                        },
                    );
                }
            }
        }

        let _ = self.entities.despawn(entity);
        true
    }

    /// Inserts a component instance onto an entity.
    /// If the entity already has this component type, it's replaced.
    pub fn insert<T: Component>(&mut self, entity: Entity, component: T) -> Result<(), WorldError> {
        self.ensure_alive(entity)?;
        
        let location = self.entities.location(entity).unwrap();
        let current_key = self.archetypes.key(location.archetype).cloned().unwrap_or_default();
        let type_id = TypeId::of::<T>();
        
        if current_key.contains(type_id) {
            // Component already exists - just update in place
            let archetype = self.archetypes.get_mut(location.archetype).unwrap();
            if let Some(col) = archetype.column_mut::<T>() {
                if let Some(slot) = col.get_mut(location.slot) {
                    *slot = component;
                }
            }
        } else {
            // Need to migrate to new archetype
            self.migrate_entity_add_component(entity, component);
        }
        
        Ok(())
    }

    /// Inserts a bundle of components onto an existing entity.
    /// 
    /// This is more efficient than multiple `insert` calls because:
    /// - Single archetype migration (vs one per component)
    /// - O(1) archetype lookup using Bundle's TypeId as cache key
    /// - No intermediate key representations
    /// 
    /// If the entity already has some of the bundle's components, they are replaced.
    pub fn insert_bundle<B: Bundle + 'static>(&mut self, entity: Entity, bundle: B) -> Result<(), WorldError> {
        self.ensure_alive(entity)?;
        
        let location = self.entities.location(entity).unwrap();
        let old_archetype_index = location.archetype;
        
        // Use Bundle's TypeId as cache key for O(1) transition lookup
        let bundle_type_id = TypeId::of::<B>();
        let component_type_ids = B::type_ids();
        let new_archetype_index = self.archetypes.get_add_bundle_target(
            old_archetype_index, 
            bundle_type_id,
            &component_type_ids,
        );
        
        match new_archetype_index {
            Some(new_idx) => {
                // Migrate entity to new archetype, then insert bundle
                self.migrate_entity_for_bundle(entity, new_idx, bundle);
            }
            None => {
                // All types already exist - just write in place
                self.overwrite_bundle_components(entity, bundle);
            }
        }
        
        Ok(())
    }

    /// Migrates an entity for bundle insertion (internal helper).
    fn migrate_entity_for_bundle<B: Bundle + 'static>(&mut self, entity: Entity, new_archetype_index: usize, bundle: B) {
        let location = match self.entities.location(entity) {
            Some(loc) => loc,
            None => return,
        };
        
        let old_key = match self.archetypes.key(location.archetype) {
            Some(key) => key.clone(),
            None => return,
        };
        
        let old_row = location.slot;
        let old_archetype_index = location.archetype;
        
        // Move each existing component from old archetype to new archetype
        for &ty in &old_key {
            let old_col_ptr: *const dyn crate::archetype::Column = {
                let old_arch = self.archetypes.get(old_archetype_index).unwrap();
                match old_arch.column_raw(ty) {
                    Some(c) => c as *const _,
                    None => continue,
                }
            };
            
            let new_col_ptr: *mut dyn crate::archetype::Column = {
                let new_arch = self.archetypes.get_mut(new_archetype_index).unwrap();
                match new_arch.column_raw_mut(ty) {
                    Some(c) => c as *mut _,
                    None => continue,
                }
            };
            
            // Move component (old slot becomes uninitialized)
            unsafe {
                (*old_col_ptr).move_to(old_row, &mut *new_col_ptr);
            }
        }
        
        // Add entity to new archetype
        let new_archetype = self.archetypes.get_mut(new_archetype_index).unwrap();
        let new_row = new_archetype.push_entity(entity);
        
        // Update entity location BEFORE bundle.insert so write_component uses correct slot
        self.entities.set_location(
            entity,
            EntityLocation {
                archetype: new_archetype_index,
                slot: new_row,
            },
        );
        
        // Now insert the bundle components
        bundle.insert(entity, self);
        
        // Remove from old archetype WITHOUT dropping (components were moved)
        let old_archetype = self.archetypes.get_mut(old_archetype_index).unwrap();
        // Safety: all components were moved out via move_to above
        let swapped = unsafe { old_archetype.swap_remove_moved(old_row) };
        
        // Update swapped entity's location if any
        if let Some(swapped_entity) = swapped {
            self.entities.set_location(
                swapped_entity,
                EntityLocation {
                    archetype: old_archetype_index,
                    slot: old_row,
                },
            );
        }
    }

    /// Overwrites existing component slots with bundle values (internal helper).
    /// 
    /// This is called when all bundle components already exist on the entity.
    /// `write_component` automatically detects that the slot is within bounds
    /// and uses set semantics (drop old, write new) instead of push.
    fn overwrite_bundle_components<B: Bundle + 'static>(&mut self, entity: Entity, bundle: B) {
        bundle.insert(entity, self);
    }

    /// Inserts bundles onto multiple entities in a batch.
    /// 
    /// This is more efficient than calling `insert_bundle` repeatedly because:
    /// - Groups entities by source archetype
    /// - Performs bulk migrations per archetype group
    /// - Single archetype lookup per group
    /// 
    /// # Arguments
    /// * `batch` - Iterator of (Entity, Bundle) pairs
    pub fn insert_bundle_batch<B: Bundle + 'static>(&mut self, batch: impl IntoIterator<Item = (Entity, B)>) {
        let bundle_type_id = TypeId::of::<B>();
        let component_type_ids = B::type_ids();
        
        // Collect and group by source archetype in one pass
        let mut groups: hashbrown::HashMap<usize, Vec<(Entity, usize, B)>> = hashbrown::HashMap::new();
        
        for (entity, bundle) in batch {
            if !self.entities.is_alive(entity) {
                continue;
            }
            
            if let Some(location) = self.entities.location(entity) {
                groups
                    .entry(location.archetype)
                    .or_default()
                    .push((entity, location.slot, bundle));
            }
        }
        
        // Process each archetype group
        for (source_arch_idx, entities_bundles) in groups {
            self.migrate_bundle_batch::<B>(source_arch_idx, bundle_type_id, &component_type_ids, entities_bundles);
        }
    }
    
    /// Migrates a batch of entities from the same source archetype.
    fn migrate_bundle_batch<B: Bundle + 'static>(
        &mut self,
        source_arch_idx: usize,
        bundle_type_id: TypeId,
        component_type_ids: &[TypeId],
        mut entities_bundles: Vec<(Entity, usize, B)>,
    ) {
        if entities_bundles.is_empty() {
            return;
        }
        
        // Get target archetype (all entities in this batch go to same target)
        let target_arch_idx = match self.archetypes.get_add_bundle_target(
            source_arch_idx,
            bundle_type_id,
            component_type_ids,
        ) {
            Some(idx) => idx,
            None => {
                // All bundle types already exist - overwrite in place
                for (entity, _, bundle) in entities_bundles {
                    self.overwrite_bundle_components(entity, bundle);
                }
                return;
            }
        };
        
        // Sort by row descending so swap_remove doesn't invalidate indices
        entities_bundles.sort_by(|a, b| b.1.cmp(&a.1));
        
        let batch_size = entities_bundles.len();
        
        // Collect rows and entities for bulk operations
        let rows: Vec<usize> = entities_bundles.iter().map(|(_, row, _)| *row).collect();
        let entities: Vec<Entity> = entities_bundles.iter().map(|(e, _, _)| *e).collect();
        
        // Get source archetype key for component iteration
        let source_key = match self.archetypes.key(source_arch_idx) {
            Some(key) => key.clone(),
            None => return,
        };
        
        // Reserve space in target archetype for all entities at once
        self.archetypes.get_mut(target_arch_idx).unwrap().reserve_bulk(batch_size);
        
        // PHASE 1: Bulk move all columns from source to target
        for &ty in &source_key {
            let old_col_ptr: *const dyn crate::archetype::Column = {
                let old_arch = self.archetypes.get(source_arch_idx).unwrap();
                match old_arch.column_raw(ty) {
                    Some(c) => c as *const _,
                    None => continue,
                }
            };
            
            let new_col_ptr: *mut dyn crate::archetype::Column = {
                let new_arch = self.archetypes.get_mut(target_arch_idx).unwrap();
                match new_arch.column_raw_mut(ty) {
                    Some(c) => c as *mut _,
                    None => continue,
                }
            };
            
            // Bulk move all elements for this column
            unsafe {
                (*old_col_ptr).move_bulk(&rows, &mut *new_col_ptr);
            }
        }
        
        // PHASE 2: Bulk add entities to target archetype
        let start_row = self.archetypes.get_mut(target_arch_idx).unwrap()
            .push_entities_bulk(&entities);
        
        // PHASE 3: Update all entity locations
        for (i, &entity) in entities.iter().enumerate() {
            self.entities.set_location(
                entity,
                EntityLocation {
                    archetype: target_arch_idx,
                    slot: start_row + i,
                },
            );
        }
        
        // PHASE 4: Insert bundle components for all entities
        for (entity, _, bundle) in entities_bundles {
            bundle.insert(entity, self);
        }
        
        // PHASE 5: Bulk remove from source archetype (no drop - components were moved)
        let swapped = unsafe {
            self.archetypes.get_mut(source_arch_idx).unwrap().swap_remove_moved_bulk(&rows)
        };
        
        // Update swapped entities' locations
        for (i, swap_result) in swapped.into_iter().enumerate() {
            if let Some(swapped_entity) = swap_result {
                self.entities.set_location(
                    swapped_entity,
                    EntityLocation {
                        archetype: source_arch_idx,
                        slot: rows[i],
                    },
                );
            }
        }
    }

    /// Removes and returns a component instance if it exists.
    pub fn remove<T: Component>(&mut self, entity: Entity) -> Option<T> {
        if !self.entities.is_alive(entity) {
            return None;
        }

        let location = self.entities.location(entity)?;
        let current_key = self.archetypes.key(location.archetype).cloned()?;
        let type_id = TypeId::of::<T>();
        
        if !current_key.contains(type_id) {
            return None;
        }

        // Migrate to archetype without this component (this extracts and returns the value)
        self.migrate_entity_remove_component::<T>(entity)
    }

    /// Removes a component from multiple entities efficiently.
    /// 
    /// This is more efficient than calling `remove` multiple times because it:
    /// - Groups entities by source archetype
    /// - Uses bulk column moves
    /// - Batches entity location updates
    /// 
    /// Note: Unlike `remove`, this does NOT return the removed components (they are dropped).
    pub fn remove_batch<T: Component>(&mut self, entities: &[Entity]) {
        if entities.is_empty() {
            return;
        }
        
        let type_id = TypeId::of::<T>();
        
        // Group entities by their current archetype
        let mut groups: hashbrown::HashMap<usize, Vec<(Entity, usize)>> = hashbrown::HashMap::new();
        
        for &entity in entities {
            if !self.entities.is_alive(entity) {
                continue;
            }
            
            if let Some(location) = self.entities.location(entity) {
                // Check if entity actually has this component
                if let Some(key) = self.archetypes.key(location.archetype) {
                    if key.contains(type_id) {
                        groups
                            .entry(location.archetype)
                            .or_default()
                            .push((entity, location.slot));
                    }
                }
            }
        }
        
        // Process each archetype group
        for (source_arch_idx, entity_rows) in groups {
            self.migrate_remove_batch::<T>(source_arch_idx, entity_rows);
        }
    }
    
    /// Migrates a batch of entities removing component T.
    fn migrate_remove_batch<T: Component>(
        &mut self,
        source_arch_idx: usize,
        mut entity_rows: Vec<(Entity, usize)>,
    ) {
        if entity_rows.is_empty() {
            return;
        }
        
        let type_id = TypeId::of::<T>();
        
        // Get target archetype
        let target_arch_idx = match self.archetypes.get_remove_target(source_arch_idx, type_id) {
            Some(idx) => idx,
            None => return, // Component not in archetype
        };
        
        // Sort by row descending for safe swap_remove
        entity_rows.sort_by(|a, b| b.1.cmp(&a.1));
        
        let batch_size = entity_rows.len();
        let rows: Vec<usize> = entity_rows.iter().map(|(_, row)| *row).collect();
        let entities: Vec<Entity> = entity_rows.iter().map(|(e, _)| *e).collect();
        
        // Get the new archetype key (components to migrate)
        let new_key = match self.archetypes.key(target_arch_idx) {
            Some(key) => key.clone(),
            None => return,
        };
        
        // Reserve space in target archetype
        self.archetypes.get_mut(target_arch_idx).unwrap().reserve_bulk(batch_size);
        
        // PHASE 1: Drop the removed components
        {
            let source = self.archetypes.get_mut(source_arch_idx).unwrap();
            if let Some(col) = source.column_mut::<T>() {
                // Drop each component (in descending row order is fine)
                for &row in &rows {
                    // Read and drop the component
                    if let Some(ptr) = col.get_mut(row) {
                        unsafe { std::ptr::drop_in_place(ptr) };
                    }
                }
            }
        }
        
        // PHASE 2: Bulk move remaining columns from source to target
        for &comp_type_id in new_key.iter() {
            let old_col_ptr: *const dyn crate::archetype::Column = {
                let old_arch = self.archetypes.get(source_arch_idx).unwrap();
                match old_arch.column_raw(comp_type_id) {
                    Some(c) => c as *const _,
                    None => continue,
                }
            };
            
            let new_col_ptr: *mut dyn crate::archetype::Column = {
                let new_arch = self.archetypes.get_mut(target_arch_idx).unwrap();
                match new_arch.column_raw_mut(comp_type_id) {
                    Some(c) => c as *mut _,
                    None => continue,
                }
            };
            
            unsafe {
                (*old_col_ptr).move_bulk(&rows, &mut *new_col_ptr);
            }
        }
        
        // PHASE 3: Bulk add entities to target archetype
        let start_row = self.archetypes.get_mut(target_arch_idx).unwrap()
            .push_entities_bulk(&entities);
        
        // PHASE 4: Update all entity locations
        for (i, &entity) in entities.iter().enumerate() {
            self.entities.set_location(
                entity,
                EntityLocation {
                    archetype: target_arch_idx,
                    slot: start_row + i,
                },
            );
        }
        
        // PHASE 5: Bulk remove from source archetype
        let swapped = unsafe {
            self.archetypes.get_mut(source_arch_idx).unwrap().swap_remove_moved_bulk(&rows)
        };
        
        // Update swapped entities' locations
        for (i, swap_result) in swapped.into_iter().enumerate() {
            if let Some(swapped_entity) = swap_result {
                self.entities.set_location(
                    swapped_entity,
                    EntityLocation {
                        archetype: source_arch_idx,
                        slot: rows[i],
                    },
                );
            }
        }
    }

    /// Returns `true` if the entity exists and is alive.
    #[inline]
    pub fn is_alive(&self, entity: Entity) -> bool {
        self.entities.is_alive(entity)
    }

    /// Fetches an immutable reference to a component.
    pub fn get<T: Component>(&self, entity: Entity) -> Option<&T> {
        if !self.entities.is_alive(entity) {
            return None;
        }

        let location = self.entities.location(entity)?;
        let archetype = self.archetypes.get(location.archetype)?;
        archetype.get::<T>(location.slot)
    }

    /// Fetches a mutable reference to a component.
    pub fn get_mut<T: Component>(&mut self, entity: Entity) -> Option<&mut T> {
        if !self.entities.is_alive(entity) {
            return None;
        }

        let location = self.entities.location(entity)?;
        let archetype = self.archetypes.get_mut(location.archetype)?;
        archetype.get_mut::<T>(location.slot)
    }

    /// Shortcut that checks if a component exists on an entity.
    pub fn contains<T: Component>(&self, entity: Entity) -> bool {
        self.get::<T>(entity).is_some()
    }

    /// Creates a [`Query`] that can iterate complex component patterns.
    /// 
    /// # Example
    /// ```
    /// use nimbus_ecs::{World, Component};
    ///
    /// // #[derive(Component)]  -- use this in your code
    /// struct Position { x: f32, y: f32 }
    /// # impl Component for Position {}
    ///
    /// let mut world = World::new();
    /// world.spawn_with(Position { x: 1.0, y: 2.0 });
    ///
    /// // Query all positions
    /// let mut query = world.query::<&Position>();
    /// assert_eq!(query.iter().count(), 1);
    /// ```
    #[allow(private_bounds)]
    pub fn query<P>(&mut self) -> Query<'_, P, ()>
    where
        P: QueryParam,
    {
        let cell = UnsafeWorldCell::new_readonly(self);
        Query::new_from_world(cell)
    }

    /// Creates a [`Query`] with a filter that can iterate complex component patterns.
    /// 
    /// # Type Parameters
    /// - `P`: The query parameters (components to fetch)
    /// - `F`: Filter to apply (e.g., `With<T>`, `Without<T>`)
    /// 
    /// # Example
    /// ```
    /// use nimbus_ecs::{World, Component, With};
    ///
    /// // #[derive(Component)]  -- use this in your code
    /// struct Position { x: f32, y: f32 }
    /// # impl Component for Position {}
    /// // #[derive(Component)]
    /// struct Velocity { x: f32, y: f32 }
    /// # impl Component for Velocity {}
    ///
    /// let mut world = World::new();
    /// world.spawn_with((Position { x: 0.0, y: 0.0 }, Velocity { x: 1.0, y: 0.0 }));
    /// world.spawn_with(Position { x: 1.0, y: 1.0 }); // No velocity
    ///
    /// // Query positions of entities that have Velocity
    /// let mut query = world.query_filtered::<&Position, With<Velocity>>();
    /// assert_eq!(query.iter().count(), 1);
    /// ```
    #[allow(private_bounds)]
    pub fn query_filtered<P, F>(&mut self) -> Query<'_, P, F>
    where
        P: QueryParam,
        F: QueryFilter,
    {
        let cell = UnsafeWorldCell::new_readonly(self);
        Query::new_from_world(cell)
    }

    /// Runs a system function against the world once.
    pub fn run_system<M, S: IntoSystem<M>>(&mut self, system: S) -> Result<(), SystemParamError>
    {
        system.into_system().run(self)
    }

    /// Returns the current tick count (number of times the world has been run).
    #[inline]
    pub fn tick(&self) -> u64 {
        self.tick
    }

    /// Inserts or replaces a singleton value.
    pub fn insert_singleton<T: 'static>(&mut self, value: T) {
        self.singletons.insert(TypeId::of::<T>(), Box::new(value));
    }

    /// Gets an immutable reference to a singleton value.
    pub fn get_singleton<T: 'static>(&self) -> Option<&T> {
        self.singletons
            .get(&TypeId::of::<T>())
            .and_then(|any| any.downcast_ref::<T>())
    }

    /// Gets a mutable reference to a singleton value.
    pub fn get_singleton_mut<T: 'static>(&mut self) -> Option<&mut T> {
        self.singletons
            .get_mut(&TypeId::of::<T>())
            .and_then(|any| any.downcast_mut::<T>())
    }

    /// Removes a singleton value.
    pub fn remove_singleton<T: 'static>(&mut self) -> Option<T> {
        self.singletons
            .remove(&TypeId::of::<T>())
            .and_then(|any| any.downcast().ok())
            .map(|boxed| *boxed)
    }

    #[allow(dead_code)]
    pub(crate) fn singleton_ptr<T: 'static>(&self) -> Option<*const T> {
        self.singletons
            .get(&TypeId::of::<T>())
            .and_then(|any| any.downcast_ref::<T>())
            .map(|value| value as *const T)
    }

    #[allow(dead_code)]
    pub(crate) fn singleton_mut_ptr<T: 'static>(&mut self) -> Option<*mut T> {
        self.singletons
            .get_mut(&TypeId::of::<T>())
            .and_then(|any| any.downcast_mut::<T>())
            .map(|value| value as *mut T)
    }

    /// Returns a reference to the archetypes collection.
    pub(crate) fn archetypes(&self) -> &Archetypes {
        &self.archetypes
    }

    /// Returns a mutable reference to the archetypes collection.
    #[allow(dead_code)]
    pub(crate) fn archetypes_mut(&mut self) -> &mut Archetypes {
        &mut self.archetypes
    }

    fn ensure_alive(&self, entity: Entity) -> Result<(), WorldError> {
        if self.entities.is_alive(entity) {
            Ok(())
        } else {
            Err(WorldError::EntityNotAlive(entity))
        }
    }

    /// Writes a component to an entity's archetype column.
    /// Called by Bundle implementations during spawn_with and insert_bundle.
    /// 
    /// Automatically determines the correct operation:
    /// - If the entity's slot is at/beyond column end → push (extending)
    /// - If the entity's slot is within column → set (overwriting)
    pub(crate) fn write_component<T: Component>(&mut self, entity: Entity, component: T) {
        let Some(location) = self.entities.location(entity) else { return };
        let Some(archetype) = self.archetypes.get_mut(location.archetype) else { return };
        
        // Ensure column exists
        archetype.ensure_column::<T>();
        
        let column = archetype.column_mut::<T>().unwrap();
        let slot = location.slot;
        
        if slot >= column.len() {
            // Slot is at/beyond end - extending the column (spawn_with, migration)
            column.push(component);
        } else {
            // Slot is within column - overwriting existing value
            column.set(slot, component);
        }
    }

    /// Migrates an entity to a new archetype by adding a component.
    fn migrate_entity_add_component<T: Component>(&mut self, entity: Entity, new_component: T) {
        let location = match self.entities.location(entity) {
            Some(loc) => loc,
            None => return,
        };

        let old_row = location.slot;
        let old_archetype_index = location.archetype;

        // Use cached graph edge for O(1) archetype transition lookup
        let type_id = TypeId::of::<T>();
        let new_archetype_index = match self.archetypes.get_add_target(old_archetype_index, type_id) {
            Some(idx) => idx,
            None => return, // Already has this component
        };

        // Get the old key for iterating components (needed for move)
        let old_key = match self.archetypes.key(old_archetype_index) {
            Some(key) => key.clone(),
            None => return,
        };

        // Move each component from old archetype to new archetype
        for &ty in &old_key {
            // Get pointers to both columns (using raw pointers to work around borrow checker)
            let old_col_ptr: *const dyn crate::archetype::Column = {
                let old_arch = self.archetypes.get(old_archetype_index).unwrap();
                match old_arch.column_raw(ty) {
                    Some(c) => c as *const _,
                    None => continue,
                }
            };
            
            let new_col_ptr: *mut dyn crate::archetype::Column = {
                let new_arch = self.archetypes.get_mut(new_archetype_index).unwrap();
                match new_arch.column_raw_mut(ty) {
                    Some(c) => c as *mut _,
                    None => continue,
                }
            };
            
            // Move component from old to new (old slot becomes uninitialized)
            unsafe {
                (*old_col_ptr).move_to(old_row, &mut *new_col_ptr);
            }
        }

        // Push the new component to the new archetype
        self.archetypes.get_mut(new_archetype_index).unwrap().push_component(new_component);

        // Add entity to new archetype
        let new_archetype = self.archetypes.get_mut(new_archetype_index).unwrap();
        let new_row = new_archetype.push_entity(entity);

        // Remove from old archetype WITHOUT dropping (components were moved)
        let old_archetype = self.archetypes.get_mut(old_archetype_index).unwrap();
        // Safety: all components were moved out via move_to above
        let swapped = unsafe { old_archetype.swap_remove_moved(old_row) };
        
        // Update swapped entity's location if any
        if let Some(swapped_entity) = swapped {
            self.entities.set_location(
                swapped_entity,
                EntityLocation {
                    archetype: old_archetype_index,
                    slot: old_row,
                },
            );
        }
        
        // Update migrated entity's location
        self.entities.set_location(
            entity,
            EntityLocation {
                archetype: new_archetype_index,
                slot: new_row,
            },
        );
    }

    /// Migrates an entity to a new archetype by removing a component.
    fn migrate_entity_remove_component<T: Component>(&mut self, entity: Entity) -> Option<T> {
        let location = self.entities.location(entity)?;
        let type_id = TypeId::of::<T>();
        
        let old_row = location.slot;
        let old_archetype_index = location.archetype;

        // Use cached graph edge for O(1) archetype transition lookup
        let new_archetype_index = self.archetypes.get_remove_target(old_archetype_index, type_id)?;
        
        // Get the new key for iterating components to move
        let new_key = self.archetypes.key(new_archetype_index).cloned()?;

        // Extract the component to be removed by reading it via pointer (moves it out)
        let removed_component = {
            let old_archetype = self.archetypes.get_mut(old_archetype_index)?;
            let col = old_archetype.column_mut::<T>()?;
            let ptr = col.get_mut(old_row)? as *mut T;
            // Read the value out - slot is now uninitialized
            unsafe { std::ptr::read(ptr) }
        };

        // Move OTHER components (not T) from old to new archetype
        for &comp_type_id in new_key.iter() {
            // The new_key doesn't include type_id, so no need to skip
            let old_col_ptr: *mut dyn crate::archetype::Column = self.archetypes.get_mut(old_archetype_index)
                .unwrap()
                .column_raw_mut(comp_type_id)
                .unwrap();
            let new_col_ptr: *mut dyn crate::archetype::Column = self.archetypes.get_mut(new_archetype_index)
                .unwrap()
                .column_raw_mut(comp_type_id)
                .unwrap();
            
            // Move component from old to new (old slot becomes uninitialized)
            unsafe {
                (*old_col_ptr).move_to(old_row, &mut *new_col_ptr);
            }
        }

        // Add entity to new archetype
        let new_archetype = self.archetypes.get_mut(new_archetype_index).unwrap();
        let new_row = new_archetype.push_entity(entity);

        // Remove from old archetype WITHOUT dropping (all components were moved/read out)
        let old_archetype = self.archetypes.get_mut(old_archetype_index).unwrap();
        // Safety: removed component was read via ptr::read, others were moved via move_to
        let swapped = unsafe { old_archetype.swap_remove_moved(old_row) };

        // Update swapped entity's location if any
        if let Some(swapped_entity) = swapped {
            self.entities.set_location(
                swapped_entity,
                EntityLocation {
                    archetype: old_archetype_index,
                    slot: old_row,
                },
            );
        }

        // Update migrated entity's location
        self.entities.set_location(
            entity,
            EntityLocation {
                archetype: new_archetype_index,
                slot: new_row,
            },
        );

        Some(removed_component)
    }
}

impl Default for World {
    fn default() -> Self {
        Self {
            entities: Entities::default(),
            archetypes: Archetypes::new(),
            singletons: TypeHashMap::default(),
            bundle_archetype_cache: TypeHashMap::default(),
            event_storage: crate::events::EventStorage::new(),
            tick: 0,
        }
    }
}

// ============================================================================
// WorldInit Implementation for World
// ============================================================================

impl crate::app::WorldInit for World {
    #[inline]
    fn spawn(&mut self) -> Entity {
        World::spawn(self)
    }

    #[inline]
    fn spawn_with<B: Bundle + 'static>(&mut self, bundle: B) -> Entity {
        World::spawn_with(self, bundle)
    }

    #[inline]
    fn spawn_batch<B, I>(&mut self, bundles: I) -> Vec<Entity>
    where
        B: Bundle + 'static,
        I: IntoIterator<Item = B>,
    {
        World::spawn_batch(self, bundles)
    }

    #[inline]
    fn despawn(&mut self, entity: Entity) -> bool {
        World::despawn(self, entity)
    }

    #[inline]
    fn insert_singleton<T: 'static>(&mut self, value: T) {
        World::insert_singleton(self, value)
    }

    #[inline]
    fn get_singleton<T: 'static>(&self) -> Option<&T> {
        World::get_singleton(self)
    }

    #[inline]
    fn get_singleton_mut<T: 'static>(&mut self) -> Option<&mut T> {
        World::get_singleton_mut(self)
    }

    #[inline]
    fn remove_singleton<T: 'static>(&mut self) -> Option<T> {
        World::remove_singleton(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Component;

    #[derive(Component, Clone, Default)]
    struct Position {
        x: f32,
        y: f32,
    }

    #[derive(Component, Clone, Default)]
    struct Velocity {
        x: f32,
        y: f32,
    }

    #[derive(Component, Clone, Default)]
    #[allow(dead_code)]
    struct Acceleration {
        x: f32,
        y: f32,
    }

    #[test]
    fn spawn_and_get_component() {
        let mut world = World::new();
        let e = world.spawn_with(Position { x: 1.0, y: 2.0 });
        
        let pos = world.get::<Position>(e).unwrap();
        assert_eq!((pos.x, pos.y), (1.0, 2.0));
    }

    #[test]
    fn spawn_with_bundle() {
        let mut world = World::new();
        let e = world.spawn_with((
            Position { x: 5.0, y: -3.0 },
            Velocity { x: 0.25, y: 0.75 },
        ));

        let pos = world.get::<Position>(e).unwrap();
        let vel = world.get::<Velocity>(e).unwrap();

        assert_eq!((pos.x, pos.y), (5.0, -3.0));
        assert_eq!((vel.x, vel.y), (0.25, 0.75));
    }

    #[test]
    fn despawn_entity() {
        let mut world = World::new();
        let e = world.spawn_with(Position { x: 1.0, y: 2.0 });
        
        assert!(world.get::<Position>(e).is_some());
        assert!(world.despawn(e));
        assert!(world.get::<Position>(e).is_none());
    }

    #[test]
    fn insert_replaces_component() {
        let mut world = World::new();
        let e = world.spawn_with(Position { x: 1.0, y: 2.0 });
        
        world.insert(e, Position { x: 10.0, y: 20.0 }).unwrap();
        
        let pos = world.get::<Position>(e).unwrap();
        assert_eq!((pos.x, pos.y), (10.0, 20.0));
    }

    #[test]
    fn insert_new_component_preserves_existing() {
        let mut world = World::new();
        
        // Spawn entity with Position
        let e = world.spawn_with(Position { x: 42.0, y: 99.0 });
        
        // Verify initial state
        let pos = world.get::<Position>(e).unwrap();
        assert_eq!((pos.x, pos.y), (42.0, 99.0));
        assert!(world.get::<Velocity>(e).is_none());
        
        // Add Velocity component (triggers migration to new archetype)
        world.insert(e, Velocity { x: 1.5, y: 2.5 }).unwrap();
        
        // Verify old component is preserved with same values
        let pos = world.get::<Position>(e).unwrap();
        assert_eq!((pos.x, pos.y), (42.0, 99.0));
        
        // Verify new component exists
        let vel = world.get::<Velocity>(e).unwrap();
        assert_eq!((vel.x, vel.y), (1.5, 2.5));
        
        // Add a third component to test multiple migrations
        world.insert(e, Acceleration { x: 0.1, y: 0.2 }).unwrap();
        
        // Verify all components are preserved
        let pos = world.get::<Position>(e).unwrap();
        let vel = world.get::<Velocity>(e).unwrap();
        let acc = world.get::<Acceleration>(e).unwrap();
        assert_eq!((pos.x, pos.y), (42.0, 99.0));
        assert_eq!((vel.x, vel.y), (1.5, 2.5));
        assert_eq!((acc.x, acc.y), (0.1, 0.2));
    }

    #[test]
    fn tick_increments_on_run() {
        let mut app = crate::App::new();
        
        assert_eq!(app.world().tick(), 0);
        
        app.run().unwrap();
        assert_eq!(app.world().tick(), 1);
        
        app.run().unwrap();
        app.run().unwrap();
        assert_eq!(app.world().tick(), 3);
    }

    #[test]
    fn spawn_batch_creates_multiple_entities() {
        let mut world = World::new();

        // Spawn batch of single-component entities
        let positions = vec![
            Position { x: 0.0, y: 0.0 },
            Position { x: 1.0, y: 1.0 },
            Position { x: 2.0, y: 2.0 },
        ];
        let entities = world.spawn_batch(positions);

        assert_eq!(entities.len(), 3);
        assert_eq!(world.get::<Position>(entities[0]).unwrap().x, 0.0);
        assert_eq!(world.get::<Position>(entities[1]).unwrap().x, 1.0);
        assert_eq!(world.get::<Position>(entities[2]).unwrap().x, 2.0);

        // Spawn batch of bundle entities
        let bundles: Vec<(Position, Velocity)> = (0..5)
            .map(|i| {
                (
                    Position { x: i as f32, y: i as f32 * 2.0 },
                    Velocity { x: 0.1 * i as f32, y: 0.2 * i as f32 },
                )
            })
            .collect();
        let entities = world.spawn_batch(bundles);

        assert_eq!(entities.len(), 5);
        for (i, &e) in entities.iter().enumerate() {
            let pos = world.get::<Position>(e).unwrap();
            let vel = world.get::<Velocity>(e).unwrap();
            assert_eq!(pos.x, i as f32);
            assert_eq!(pos.y, i as f32 * 2.0);
            assert_eq!(vel.x, 0.1 * i as f32);
            assert_eq!(vel.y, 0.2 * i as f32);
        }

        // Verify total entity count
        assert_eq!(world.query::<&Position>().iter().count(), 8); // 3 + 5
    }

    #[test]
    fn spawn_batch_empty_returns_empty() {
        let mut world = World::new();
        let entities = world.spawn_batch::<Position, _>(Vec::new());
        assert!(entities.is_empty());
    }

    #[test]
    fn remove_component_preserves_other_components() {
        let mut world = World::new();
        let entity = world.spawn_with((
            Position { x: 1.0, y: 2.0 },
            Velocity { x: 3.0, y: 4.0 },
        ));

        // Verify both components exist
        assert!(world.get::<Position>(entity).is_some());
        assert!(world.get::<Velocity>(entity).is_some());

        // Remove Velocity
        let removed = world.remove::<Velocity>(entity);
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().x, 3.0);

        // Position should still exist, Velocity should not
        let pos = world.get::<Position>(entity).unwrap();
        assert_eq!((pos.x, pos.y), (1.0, 2.0));
        assert!(world.get::<Velocity>(entity).is_none());
    }

    // ========================================================================
    // Drop behavior tests - ensure components drop at the right times
    // ========================================================================

    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    /// A component that tracks when it's dropped via an atomic counter.
    #[derive(Component)]
    struct DropTracker {
        drop_count: Arc<AtomicUsize>,
    }

    impl Drop for DropTracker {
        fn drop(&mut self) {
            self.drop_count.fetch_add(1, Ordering::SeqCst);
        }
    }

    /// A second drop tracker for bundle tests.
    #[derive(Component)]
    struct DropTracker2 {
        drop_count: Arc<AtomicUsize>,
    }

    impl Drop for DropTracker2 {
        fn drop(&mut self) {
            self.drop_count.fetch_add(1, Ordering::SeqCst);
        }
    }

    /// A simple tag component (no drop tracking).
    #[derive(Component)]
    struct Tag;

    #[test]
    fn add_component_transition_does_not_double_drop() {
        let drop_count = Arc::new(AtomicUsize::new(0));

        {
            let mut world = World::new();
            
            // Spawn entity with DropTracker
            let entity = world.spawn_with(DropTracker { drop_count: drop_count.clone() });
            
            // Add a new component - this triggers archetype transition
            // The DropTracker should NOT be dropped during migration
            let _ = world.insert(entity, Tag);
            
            // Verify component is still accessible
            assert!(world.get::<DropTracker>(entity).is_some());
            assert!(world.get::<Tag>(entity).is_some());
            
            // Should not have dropped yet
            assert_eq!(drop_count.load(Ordering::SeqCst), 0, "Component dropped during add transition!");
        }

        // After world drops, component should be dropped exactly once
        assert_eq!(drop_count.load(Ordering::SeqCst), 1, "Component not dropped when world dropped");
    }

    #[test]
    fn remove_component_does_not_double_drop_others() {
        let drop_count_a = Arc::new(AtomicUsize::new(0));
        let drop_count_b = Arc::new(AtomicUsize::new(0));

        {
            let mut world = World::new();
            
            // Spawn entity with two drop trackers
            let entity = world.spawn_with((
                DropTracker { drop_count: drop_count_a.clone() },
                DropTracker2 { drop_count: drop_count_b.clone() },
            ));
            
            // Remove one component - this triggers archetype transition
            // The OTHER component should NOT be dropped during migration
            let removed = world.remove::<DropTracker2>(entity);
            
            // The removed component SHOULD be dropped (when `removed` goes out of scope)
            assert!(removed.is_some());
            drop(removed); // Explicitly drop to count it
            assert_eq!(drop_count_b.load(Ordering::SeqCst), 1, "Removed component should drop once");
            
            // The remaining component should NOT have been dropped yet
            assert_eq!(drop_count_a.load(Ordering::SeqCst), 0, "Remaining component dropped during remove transition!");
            
            // Verify remaining component is still accessible
            assert!(world.get::<DropTracker>(entity).is_some());
        }

        // After world drops, remaining component should be dropped
        assert_eq!(drop_count_a.load(Ordering::SeqCst), 1, "Remaining component not dropped when world dropped");
    }

    #[test]
    fn add_bundle_transition_does_not_double_drop() {
        let drop_count = Arc::new(AtomicUsize::new(0));

        {
            let mut world = World::new();
            
            // Spawn entity with just DropTracker
            let entity = world.spawn_with(DropTracker { drop_count: drop_count.clone() });
            
            // Add a bundle - this triggers archetype transition
            // The existing DropTracker should NOT be dropped during migration
            let _ = world.insert_bundle(entity, (Position { x: 1.0, y: 2.0 }, Velocity { x: 3.0, y: 4.0 }));
            
            // Verify all components are accessible
            assert!(world.get::<DropTracker>(entity).is_some());
            assert!(world.get::<Position>(entity).is_some());
            assert!(world.get::<Velocity>(entity).is_some());
            
            // Should not have dropped yet
            assert_eq!(drop_count.load(Ordering::SeqCst), 0, "Component dropped during bundle add transition!");
        }

        // After world drops, component should be dropped exactly once
        assert_eq!(drop_count.load(Ordering::SeqCst), 1, "Component not dropped when world dropped");
    }

    #[test]
    fn overwrite_bundle_drops_old_values() {
        let drop_count = Arc::new(AtomicUsize::new(0));

        {
            let mut world = World::new();
            
            // Spawn entity with DropTracker
            let entity = world.spawn_with(DropTracker { drop_count: drop_count.clone() });
            
            // Overwrite with new DropTracker (same archetype, no migration)
            let _ = world.insert_bundle(entity, DropTracker { drop_count: drop_count.clone() });
            
            // Old value should have been dropped during overwrite
            assert_eq!(drop_count.load(Ordering::SeqCst), 1, "Old component not dropped during overwrite!");
            
            // New component should still be accessible
            assert!(world.get::<DropTracker>(entity).is_some());
        }

        // After world drops, new component should be dropped (total = 2)
        assert_eq!(drop_count.load(Ordering::SeqCst), 2, "New component not dropped when world dropped");
    }

    #[test]
    fn despawn_drops_all_components() {
        let drop_count_a = Arc::new(AtomicUsize::new(0));
        let drop_count_b = Arc::new(AtomicUsize::new(0));

        let mut world = World::new();
        
        let entity = world.spawn_with((
            DropTracker { drop_count: drop_count_a.clone() },
            DropTracker2 { drop_count: drop_count_b.clone() },
        ));
        
        // Neither should be dropped yet
        assert_eq!(drop_count_a.load(Ordering::SeqCst), 0);
        assert_eq!(drop_count_b.load(Ordering::SeqCst), 0);
        
        // Despawn should drop both components
        world.despawn(entity);
        
        assert_eq!(drop_count_a.load(Ordering::SeqCst), 1, "Component A not dropped on despawn");
        assert_eq!(drop_count_b.load(Ordering::SeqCst), 1, "Component B not dropped on despawn");
    }

    #[test]
    fn world_drop_cleans_up_all_entities() {
        let drop_count = Arc::new(AtomicUsize::new(0));

        {
            let mut world = World::new();
            
            // Spawn multiple entities with drop trackers
            for _ in 0..10 {
                world.spawn_with(DropTracker { drop_count: drop_count.clone() });
            }
            
            // None should be dropped yet
            assert_eq!(drop_count.load(Ordering::SeqCst), 0);
        }

        // All 10 should be dropped when world drops
        assert_eq!(drop_count.load(Ordering::SeqCst), 10, "Not all components dropped when world dropped");
    }

    #[test]
    fn multiple_archetype_transitions_no_extra_drops() {
        let drop_count = Arc::new(AtomicUsize::new(0));

        {
            let mut world = World::new();
            
            // Spawn with DropTracker
            let entity = world.spawn_with(DropTracker { drop_count: drop_count.clone() });
            assert_eq!(drop_count.load(Ordering::SeqCst), 0);
            
            // Transition 1: Add Position
            let _ = world.insert(entity, Position { x: 1.0, y: 2.0 });
            assert_eq!(drop_count.load(Ordering::SeqCst), 0, "Dropped during transition 1");
            
            // Transition 2: Add Velocity  
            let _ = world.insert(entity, Velocity { x: 3.0, y: 4.0 });
            assert_eq!(drop_count.load(Ordering::SeqCst), 0, "Dropped during transition 2");
            
            // Transition 3: Remove Position
            world.remove::<Position>(entity);
            assert_eq!(drop_count.load(Ordering::SeqCst), 0, "Dropped during transition 3");
            
            // Transition 4: Add Position back
            let _ = world.insert(entity, Position { x: 5.0, y: 6.0 });
            assert_eq!(drop_count.load(Ordering::SeqCst), 0, "Dropped during transition 4");
            
            // Verify component is still there
            assert!(world.get::<DropTracker>(entity).is_some());
        }

        // Finally drops when world is dropped
        assert_eq!(drop_count.load(Ordering::SeqCst), 1, "Component not dropped when world dropped");
    }
}
