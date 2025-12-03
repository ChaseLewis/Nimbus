use std::any::TypeId;

use crate::{component::Component, entity::Entity, world::World};

/// Collection of components that can be spawned together.
pub trait Bundle: Sized {
    /// Returns the component type identifiers contained in the bundle.
    fn type_ids() -> Vec<TypeId>;

    /// Inserts every component of the bundle onto the entity.
    fn insert(self, entity: Entity, world: &mut World);
}

impl<T> Bundle for T
where
    T: Component,
{
    fn type_ids() -> Vec<TypeId> {
        vec![TypeId::of::<T>()]
    }

    fn insert(self, entity: Entity, world: &mut World) {
        world.write_component(entity, self);
    }
}

macro_rules! impl_bundle_tuple {
    ($($name:ident),+) => {
        impl<$($name),+> Bundle for ($($name,)+)
        where
            $($name: Component,)+
        {
            fn type_ids() -> Vec<TypeId> {
                vec![$(TypeId::of::<$name>()),+]
            }

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

impl_bundle_tuple!(A, B);
impl_bundle_tuple!(A, B, C);
impl_bundle_tuple!(A, B, C, D);
impl_bundle_tuple!(A, B, C, D, E);
impl_bundle_tuple!(A, B, C, D, E, F);
impl_bundle_tuple!(A, B, C, D, E, F, G);
impl_bundle_tuple!(A, B, C, D, E, F, G, H);
