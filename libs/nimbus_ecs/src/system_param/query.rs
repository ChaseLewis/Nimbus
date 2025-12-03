//! Query system parameter for iterating over entities with specific components.

use std::any::TypeId;
use std::marker::PhantomData;
use std::mem::size_of;

use crate::{
    archetype::Archetype,
    component::Component,
    entity::Entity,
    world::UnsafeWorldCell,
};

use super::{SystemParam, SystemParamError};

/// Sealed trait pattern to prevent external implementations of QueryParam.
mod sealed {
    use crate::component::Component;
    use crate::entity::Entity;
    
    pub trait Sealed {}
    
    impl Sealed for Entity {}
    impl<T: Component> Sealed for &T {}
    impl<T: Component> Sealed for &mut T {}
    impl<P: Sealed> Sealed for Option<P> {}
    
    // Implement for tuples
    macro_rules! impl_sealed_tuple {
        ($($name:ident),+) => {
            impl<$($name: Sealed),+> Sealed for ($($name,)+) {}
        };
    }
    
    impl_sealed_tuple!(A);
    impl_sealed_tuple!(A, B);
    impl_sealed_tuple!(A, B, C);
    impl_sealed_tuple!(A, B, C, D);
    impl_sealed_tuple!(A, B, C, D, E);
    impl_sealed_tuple!(A, B, C, D, E, F);
    impl_sealed_tuple!(A, B, C, D, E, F, G);
    impl_sealed_tuple!(A, B, C, D, E, F, G, H);
    impl_sealed_tuple!(A, B, C, D, E, F, G, H, I);
    impl_sealed_tuple!(A, B, C, D, E, F, G, H, I, J);
    impl_sealed_tuple!(A, B, C, D, E, F, G, H, I, J, K);
    impl_sealed_tuple!(A, B, C, D, E, F, G, H, I, J, K, L);
}

/// Cached state for a query, storing matching archetype indices.
/// 
/// This cache is incrementally updated when new archetypes are added,
/// avoiding full rescans on each system run.
#[derive(Default)]
pub struct QueryState {
    /// Cached list of archetype indices that match this query
    matching_archetypes: Vec<usize>,
    /// Number of archetypes in the world when we last computed matches.
    /// If world has more archetypes, we only need to scan the new ones.
    /// If we implement a way to cleanup empty archetypes we likely need 
    /// a generation version to avoid stale references.
    last_archetype_count: usize,
}

/// A dynamic query that can iterate over complex component sets with optional filters.
/// 
/// Queries find matching archetypes and iterate directly over their dense columns,
/// providing cache-efficient iteration without per-entity lookups.
#[allow(private_bounds)]
pub struct Query<'w, P, F = ()>
where
    P: QueryParam,
    F: QueryFilter,
{
    /// Archetype indices matching this query (borrowed from cache or owned)
    matching_archetypes: Vec<usize>,
    /// Pointer to world for accessing archetypes
    world: UnsafeWorldCell<'w>,
    _marker: PhantomData<(P, F)>,
}

#[allow(private_bounds)]
impl<'w, P, F> Query<'w, P, F>
where
    P: QueryParam,
    F: QueryFilter,
{
    /// Creates a query using cached state, incrementally updating if needed.
    pub(crate) fn new_with_state(cell: UnsafeWorldCell<'w>, state: &mut QueryState) -> Self {
        let archetypes = cell.archetypes();
        let current_count = archetypes.len();
        
        // Check if we need to scan for new archetypes
        if state.last_archetype_count < current_count {
            // Build required/excluded type lists
            let mut required_types = P::required_types();
            required_types.extend(F::required_types());
            let excluded_types = F::excluded_types();
            
            if state.last_archetype_count == 0 {
                // First run: use component index for fast lookup
                state.matching_archetypes = archetypes.matching_filtered(&required_types, &excluded_types);
            } else {
                // Incremental: only scan new archetypes
                for (idx, arch) in archetypes.iter().skip(state.last_archetype_count) {
                    let has_required = required_types.iter().all(|ty| arch.key().contains(*ty));
                    let not_excluded = excluded_types.iter().all(|ty| !arch.key().contains(*ty));
                    if has_required && not_excluded {
                        state.matching_archetypes.push(idx);
                    }
                }
            }
            state.last_archetype_count = current_count;
        }
        
        // Clone the cached list - this is cheap for typical query counts
        // and avoids lifetime complexity
        Self {
            matching_archetypes: state.matching_archetypes.clone(),
            world: cell,
            _marker: PhantomData,
        }
    }

    /// Creates a query without state caching (uses component index for fast lookup).
    pub(crate) fn new_from_world(cell: UnsafeWorldCell<'w>) -> Self {
        // Find all archetypes that contain the required component types
        // Combine required types from both query params and filters
        let mut required_types = P::required_types();
        required_types.extend(F::required_types());
        let excluded_types = F::excluded_types();
        
        // Use component index for O(smallest_set) matching instead of O(all_archetypes)
        let mut matching_archetypes = cell
            .archetypes()
            .matching_filtered(&required_types, &excluded_types);
        
        // Filter out empty archetypes
        matching_archetypes.retain(|&idx| {
            cell.archetypes().get(idx).map(|a| a.len() > 0).unwrap_or(false)
        });

        Self {
            matching_archetypes,
            world: cell,
            _marker: PhantomData,
        }
    }

    /// Returns an iterator over each entity that satisfies the query.
    pub fn iter(&mut self) -> QueryIter<'_, 'w, P, F> {
        QueryIter {
            archetypes: self.matching_archetypes.iter(),
            world: self.world,
            column_state: None,
            current_row: 0,
            current_len: 0,
            _marker: PhantomData,
        }
    }
}

/// Iterator over query results, processing archetypes one at a time.
#[allow(private_bounds)]
pub struct QueryIter<'state, 'w, P, F>
where
    P: QueryParam,
    F: QueryFilter,
{
    /// Iterator over matching archetype indices
    archetypes: std::slice::Iter<'state, usize>,
    /// World reference for accessing archetype data
    world: UnsafeWorldCell<'w>,
    /// Cached column state for current archetype (avoids HashMap lookups per row)
    column_state: Option<P::ColumnState<'state>>,
    /// Current row within the archetype
    current_row: usize,
    /// Length of current archetype
    current_len: usize,
    _marker: PhantomData<F>,
}

#[allow(private_bounds)]
impl<'state, 'w, P, F> Iterator for QueryIter<'state, 'w, P, F>
where
    P: QueryParam,
    F: QueryFilter,
{
    type Item = P::Item<'state>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            // If we have cached column state, iterate directly (fast path - no HashMap lookups!)
            if self.current_row < self.current_len {
                let row = self.current_row;
                self.current_row += 1;
                return Some(P::fetch_from_columns(self.column_state.as_mut().unwrap(), row));
            }

            // Move to the next archetype
            match self.archetypes.next() {
                Some(&arch_idx) => {
                    let archetype = self.world.archetypes().get(arch_idx).unwrap();
                    // Safety: archetype lives for 'w which outlives 'state
                    let archetype: &'state Archetype = unsafe { 
                        std::mem::transmute(archetype) 
                    };
                    
                    self.column_state = P::init_columns(archetype);
                    self.current_row = 0;
                    self.current_len = archetype.len();
                }
                None => return None,
            }
        }
    }
}

/// Types that can be pulled out of a [`Query`].
/// 
/// This trait is sealed and cannot be implemented outside this crate.
#[allow(private_interfaces)]
pub trait QueryParam: sealed::Sealed {
    /// The value yielded for each matching entity.
    type Item<'a>;
    
    /// Cached column pointers for fast iteration within an archetype.
    type ColumnState<'a>;

    /// Returns the TypeIds of components required by this query parameter.
    fn required_types() -> Vec<TypeId>;
    
    /// Initializes column state for fast iteration over an archetype.
    /// Returns None if the archetype doesn't have the required columns.
    fn init_columns<'a>(archetype: &'a Archetype) -> Option<Self::ColumnState<'a>>;
    
    /// Fetches from cached column state - no HashMap lookups!
    fn fetch_from_columns<'a>(state: &mut Self::ColumnState<'a>, row: usize) -> Self::Item<'a>;

    /// Attempts to fetch the query value for a single row in an archetype.
    /// Used for single-entity access. For iteration, use init_columns + fetch_from_columns.
    #[allow(dead_code)]
    fn fetch<'a>(archetype: &'a Archetype, row: usize) -> Option<Self::Item<'a>>;
}

/// Types that can filter entities in a [`Query`].
pub trait QueryFilter {
    /// Returns TypeIds that must NOT be present on matching archetypes.
    fn excluded_types() -> Vec<TypeId> {
        Vec::new()
    }

    /// Returns TypeIds that MUST be present on matching archetypes.
    fn required_types() -> Vec<TypeId> {
        Vec::new()
    }
}

// Unit type implements QueryFilter as a no-op (matches everything)
impl QueryFilter for () {}

// Query parameter for immutable component access: Query<&T>
#[allow(private_interfaces)]
impl<T: Component> QueryParam for &T {
    type Item<'a> = &'a T;
    type ColumnState<'a> = &'a [T];

    fn required_types() -> Vec<TypeId> {
        const { assert!(size_of::<T>() > 0, "Cannot query zero-sized types. Use With<T> filter instead.") };
        vec![TypeId::of::<T>()]
    }
    
    fn init_columns<'a>(archetype: &'a Archetype) -> Option<Self::ColumnState<'a>> {
        archetype.column::<T>().map(|col| col.as_slice())
    }
    
    fn fetch_from_columns<'a>(state: &mut Self::ColumnState<'a>, row: usize) -> Self::Item<'a> {
        &state[row]
    }

    fn fetch<'a>(archetype: &'a Archetype, row: usize) -> Option<Self::Item<'a>> {
        archetype.get::<T>(row)
    }
}

// Query parameter for mutable component access: Query<&mut T>
#[allow(private_interfaces)]
impl<T: Component> QueryParam for &mut T {
    type Item<'a> = &'a mut T;
    type ColumnState<'a> = *mut T;  // Pointer to start of column data

    fn required_types() -> Vec<TypeId> {
        const { assert!(size_of::<T>() > 0, "Cannot query zero-sized types. Use With<T> filter instead.") };
        vec![TypeId::of::<T>()]
    }
    
    fn init_columns<'a>(archetype: &'a Archetype) -> Option<Self::ColumnState<'a>> {
        // Safety: caller ensures exclusive access through the query system
        let archetype = archetype as *const Archetype as *mut Archetype;
        unsafe {
            (*archetype).column_mut::<T>().map(|col| col.as_mut_slice().as_mut_ptr())
        }
    }
    
    fn fetch_from_columns<'a>(state: &mut Self::ColumnState<'a>, row: usize) -> Self::Item<'a> {
        unsafe { &mut *state.add(row) }
    }

    fn fetch<'a>(archetype: &'a Archetype, row: usize) -> Option<Self::Item<'a>> {
        // Safety: caller ensures exclusive access through the query system
        let archetype = archetype as *const Archetype as *mut Archetype;
        unsafe { (*archetype).get_mut::<T>(row) }
    }
}

// Query parameter for Entity access: Query<Entity> or Query<(Entity, &T)>
#[allow(private_interfaces)]
impl QueryParam for Entity {
    type Item<'a> = Entity;
    type ColumnState<'a> = &'a [Entity];

    fn required_types() -> Vec<TypeId> {
        // Entity doesn't require any component types
        Vec::new()
    }
    
    fn init_columns<'a>(archetype: &'a Archetype) -> Option<Self::ColumnState<'a>> {
        Some(archetype.entities())
    }
    
    fn fetch_from_columns<'a>(state: &mut Self::ColumnState<'a>, row: usize) -> Self::Item<'a> {
        state[row]
    }

    fn fetch<'a>(archetype: &'a Archetype, row: usize) -> Option<Self::Item<'a>> {
        Some(archetype.entity_at(row))
    }
}

// Query parameter for optional component access
#[allow(private_interfaces)]
impl<P: QueryParam> QueryParam for Option<P> {
    type Item<'a> = Option<P::Item<'a>>;
    type ColumnState<'a> = Option<P::ColumnState<'a>>;

    fn required_types() -> Vec<TypeId> {
        // Optional components don't add required types
        Vec::new()
    }
    
    fn init_columns<'a>(archetype: &'a Archetype) -> Option<Self::ColumnState<'a>> {
        // Always succeeds - the Option wraps whether the column exists
        Some(P::init_columns(archetype))
    }
    
    fn fetch_from_columns<'a>(state: &mut Self::ColumnState<'a>, row: usize) -> Self::Item<'a> {
        state.as_mut().map(|s| P::fetch_from_columns(s, row))
    }

    fn fetch<'a>(archetype: &'a Archetype, row: usize) -> Option<Self::Item<'a>> {
        // Always return Some - the Option wraps whether the inner component exists
        Some(P::fetch(archetype, row))
    }
}

/// Query filter that requires an entity to have component T, without fetching it.
pub struct With<T: Component>(PhantomData<T>);

impl<T: Component> QueryFilter for With<T> {
    fn required_types() -> Vec<TypeId> {
        vec![TypeId::of::<T>()]
    }
}

/// Query filter that requires an entity to NOT have component T.
pub struct Without<T: Component>(PhantomData<T>);

impl<T: Component> QueryFilter for Without<T> {
    fn excluded_types() -> Vec<TypeId> {
        vec![TypeId::of::<T>()]
    }
}
// Implement QueryFilter for tuples of filters
macro_rules! impl_query_filter_tuple {
    ($($name:ident),+) => {
        impl<$($name: QueryFilter),+> QueryFilter for ($($name,)+) {
            fn excluded_types() -> Vec<TypeId> {
                let mut types = Vec::new();
                $(types.extend($name::excluded_types());)+
                types
            }

            fn required_types() -> Vec<TypeId> {
                let mut types = Vec::new();
                $(types.extend($name::required_types());)+
                types
            }
        }
    };
}

impl_query_filter_tuple!(A);
impl_query_filter_tuple!(A, B);
impl_query_filter_tuple!(A, B, C);
impl_query_filter_tuple!(A, B, C, D);
impl_query_filter_tuple!(A, B, C, D, E);
impl_query_filter_tuple!(A, B, C, D, E, F);
impl_query_filter_tuple!(A, B, C, D, E, F, G);
impl_query_filter_tuple!(A, B, C, D, E, F, G, H);

// Implement QueryParam for tuples
macro_rules! impl_query_param_tuple {
    ($($name:ident),+) => {
        #[allow(private_interfaces)]
        impl<$($name: QueryParam),+> QueryParam for ($($name,)+)
        {
            type Item<'a> = ($($name::Item<'a>,)+);
            type ColumnState<'a> = ($($name::ColumnState<'a>,)+);

            fn required_types() -> Vec<TypeId> {
                let mut types = Vec::new();
                $(types.extend($name::required_types());)+
                types
            }
            
            #[allow(non_snake_case)]
            fn init_columns<'a>(archetype: &'a Archetype) -> Option<Self::ColumnState<'a>> {
                Some(($($name::init_columns(archetype)?,)+))
            }
            
            #[allow(non_snake_case)]
            fn fetch_from_columns<'a>(
                state: &mut Self::ColumnState<'a>,
                row: usize,
            ) -> Self::Item<'a> {
                let ($($name,)+) = state;
                ($($name::fetch_from_columns($name, row),)+)
            }

            fn fetch<'a>(
                archetype: &'a Archetype,
                row: usize,
            ) -> Option<Self::Item<'a>> {
                Some(($($name::fetch(archetype, row)?,)+))
            }
        }
    };
}

impl_query_param_tuple!(A);
impl_query_param_tuple!(A, B);
impl_query_param_tuple!(A, B, C);
impl_query_param_tuple!(A, B, C, D);
impl_query_param_tuple!(A, B, C, D, E);
impl_query_param_tuple!(A, B, C, D, E, F);
impl_query_param_tuple!(A, B, C, D, E, F, G);
impl_query_param_tuple!(A, B, C, D, E, F, G, H);
impl_query_param_tuple!(A, B, C, D, E, F, G, H, I);
impl_query_param_tuple!(A, B, C, D, E, F, G, H, I, J);
impl_query_param_tuple!(A, B, C, D, E, F, G, H, I, J, K);
impl_query_param_tuple!(A, B, C, D, E, F, G, H, I, J, K, L);

// SystemParam implementation for Query
#[allow(private_bounds)]
impl<P: QueryParam + 'static, F: QueryFilter + 'static> SystemParam for Query<'_, P, F> {
    type State = QueryState;
    type Item<'w, 's> = Query<'w, P, F>;

    fn from_world_with_state<'w, 's>(
        world: UnsafeWorldCell<'w>,
        state: &'s mut Self::State,
    ) -> Result<Self::Item<'w, 's>, SystemParamError> {
        Ok(Query::new_with_state(world, state))
    }
}

#[cfg(test)]
mod tests {
    use crate::{Component, World};

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
    fn iterate_single_component() {
        let mut world = World::new();
        
        for i in 0..100 {
            world.spawn_with(Position { x: i as f32, y: i as f32 });
        }

        let count = world.query::<&Position>().iter().count();
        assert_eq!(count, 100);
    }

    #[test]
    fn iterate_multiple_components() {
        let mut world = World::new();
        
        // 100 entities with just Position
        for i in 0..100 {
            world.spawn_with(Position { x: i as f32, y: i as f32 });
        }

        // 100 entities with Position and Velocity
        for i in 0..100 {
            world.spawn_with((
                Position { x: i as f32, y: i as f32 },
                Velocity { x: i as f32, y: i as f32 },
            ));
        }

        assert_eq!(world.query::<&Position>().iter().count(), 200);
        assert_eq!(world.query::<(&Position, &Velocity)>().iter().count(), 100);
    }

    #[test]
    fn query_with_filter() {
        use super::{With, Without};

        let mut world = World::new();

        // Entity with both Position and Velocity
        world.spawn_with((
            Position { x: 1.0, y: 1.0 },
            Velocity { x: 0.5, y: 0.5 },
        ));

        // Entity with only Position
        world.spawn_with(Position { x: 2.0, y: 2.0 });

        // Query positions of entities that also have Velocity
        let positions_with_velocity: Vec<f32> = world
            .query_filtered::<&Position, With<Velocity>>()
            .iter()
            .map(|pos| pos.x)
            .collect();

        assert_eq!(positions_with_velocity.len(), 1);
        assert_eq!(positions_with_velocity[0], 1.0);

        // Query positions of entities that DON'T have Velocity
        let positions_without_velocity: Vec<f32> = world
            .query_filtered::<&Position, Without<Velocity>>()
            .iter()
            .map(|pos| pos.x)
            .collect();

        assert_eq!(positions_without_velocity.len(), 1);
        assert_eq!(positions_without_velocity[0], 2.0);
    }

    #[test]
    fn mutable_query() {
        use super::Query;

        let mut world = World::new();
        
        let e = world.spawn_with((
            Position { x: 1.0, y: 1.0 },
            Velocity { x: 0.5, y: 0.5 },
        ));

        fn movement_system(mut query: Query<(&mut Position, &Velocity)>) {
            for (pos, vel) in query.iter() {
                pos.x += vel.x;
                pos.y += vel.y;
            }
        }

        world.run_system(movement_system).unwrap();

        let pos = world.get::<Position>(e).unwrap();
        assert_eq!((pos.x, pos.y), (1.5, 1.5));
    }

    #[test]
    fn optional_query_components() {
        let mut world = World::new();

        // Entity with both Position and Velocity
        world.spawn_with((
            Position { x: 1.0, y: 1.0 },
            Velocity { x: 0.5, y: 0.5 },
        ));

        // Entity with only Position
        world.spawn_with(Position { x: 2.0, y: 2.0 });

        // Query with optional Velocity
        let mut results: Vec<(f32, Option<f32>)> = Vec::new();
        for (pos, vel) in world.query::<(&Position, Option<&Velocity>)>().iter() {
            results.push((pos.x, vel.map(|v| v.x)));
        }

        assert_eq!(results.len(), 2);
        assert!(results.iter().any(|(px, vx)| *px == 1.0 && *vx == Some(0.5)));
        assert!(results.iter().any(|(px, vx)| *px == 2.0 && *vx == None));
    }

    #[test]
    fn tag_components_with_filter() {
        use super::{With, Without};

        /// Zero-sized tag component
        #[derive(Component, Clone, Default)]
        struct IsAlive;

        #[derive(Component, Clone, Default)]
        struct IsPlayer;

        let mut world = World::new();

        // Living player with position
        world.spawn_with((Position { x: 1.0, y: 1.0 }, IsAlive, IsPlayer));

        // Living NPC with position
        world.spawn_with((Position { x: 2.0, y: 2.0 }, IsAlive));

        // Dead entity (no IsAlive tag)
        world.spawn_with(Position { x: 3.0, y: 3.0 });

        // Query only living entities
        let living: Vec<f32> = world
            .query_filtered::<&Position, With<IsAlive>>()
            .iter()
            .map(|p| p.x)
            .collect();
        assert_eq!(living.len(), 2);

        // Query only players
        let players: Vec<f32> = world
            .query_filtered::<&Position, With<IsPlayer>>()
            .iter()
            .map(|p| p.x)
            .collect();
        assert_eq!(players.len(), 1);
        assert_eq!(players[0], 1.0);

        // Query living non-players
        let npcs: Vec<f32> = world
            .query_filtered::<&Position, (With<IsAlive>, Without<IsPlayer>)>()
            .iter()
            .map(|p| p.x)
            .collect();
        assert_eq!(npcs.len(), 1);
        assert_eq!(npcs[0], 2.0);
    }

    #[test]
    fn query_with_entity() {
        use crate::Entity;

        let mut world = World::new();

        // Spawn some entities
        let e1 = world.spawn_with(Position { x: 1.0, y: 1.0 });
        let e2 = world.spawn_with(Position { x: 2.0, y: 2.0 });
        let e3 = world.spawn_with((
            Position { x: 3.0, y: 3.0 },
            Velocity { x: 0.5, y: 0.5 },
        ));

        // Query Entity alone
        let entities: Vec<Entity> = world.query::<Entity>().iter().collect();
        assert_eq!(entities.len(), 3);
        assert!(entities.contains(&e1));
        assert!(entities.contains(&e2));
        assert!(entities.contains(&e3));

        // Query Entity with component
        let entity_positions: Vec<(Entity, f32)> = world
            .query::<(Entity, &Position)>()
            .iter()
            .map(|(e, p)| (e, p.x))
            .collect();
        assert_eq!(entity_positions.len(), 3);
        assert!(entity_positions.iter().any(|(e, x)| *e == e1 && *x == 1.0));
        assert!(entity_positions.iter().any(|(e, x)| *e == e2 && *x == 2.0));
        assert!(entity_positions.iter().any(|(e, x)| *e == e3 && *x == 3.0));

        // Query Entity with multiple components
        let with_velocity: Vec<(Entity, f32, f32)> = world
            .query::<(Entity, &Position, &Velocity)>()
            .iter()
            .map(|(e, p, v)| (e, p.x, v.x))
            .collect();
        assert_eq!(with_velocity.len(), 1);
        assert_eq!(with_velocity[0], (e3, 3.0, 0.5));
    }
}



