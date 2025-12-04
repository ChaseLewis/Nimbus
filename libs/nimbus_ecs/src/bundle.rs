use crate::{
    component::{Component, ComponentId},
    entity::Entity,
    world::World,
};

/// Collection of components that can be spawned together.
pub trait Bundle: Sized + 'static {
    /// The number of components in this bundle.
    const LEN: usize;
    
    /// The component IDs in this bundle, as a const array.
    /// Use `type_ids()` for a slice view.
    const TYPE_IDS: &'static [ComponentId];

    /// Inserts every component of the bundle onto the entity.
    fn insert(self, entity: Entity, world: &mut World);
}

impl<T> Bundle for T
where
    T: Component,
{
    const LEN: usize = 1;
    const TYPE_IDS: &'static [ComponentId] = &[T::COMPONENT_ID];

    fn insert(self, entity: Entity, world: &mut World) {
        world.write_component(entity, self);
    }
}

macro_rules! impl_bundle_tuple {
    ($len:expr, $($name:ident),+) => {
        impl<$($name),+> Bundle for ($($name,)+)
        where
            $($name: Component,)+
        {
            const LEN: usize = $len;
            const TYPE_IDS: &'static [ComponentId] = &[$($name::COMPONENT_ID),+];

            #[allow(non_snake_case)]
            fn insert(self, entity: Entity, world: &mut World) {
                let ($($name,)+) = self;
                $(
                    world.write_component(entity, $name);
                )+
            }
        }
    };
}

impl_bundle_tuple!(2, A, B);
impl_bundle_tuple!(3, A, B, C);
impl_bundle_tuple!(4, A, B, C, D);
impl_bundle_tuple!(5, A, B, C, D, E);
impl_bundle_tuple!(6, A, B, C, D, E, F);
impl_bundle_tuple!(7, A, B, C, D, E, F, G);
impl_bundle_tuple!(8, A, B, C, D, E, F, G, H);
