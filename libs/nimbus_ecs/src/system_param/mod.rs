//! System parameter types and traits for dependency injection.

mod entity_api;
mod query;
mod single;
mod tasks;

pub use entity_api::{EntityAPI, EntityRef};
pub use query::{Query, QueryFilter, QueryIter, QueryParam, With, Without};
pub use single::{Single, SingleMut};
pub use tasks::Tasks;

use std::any::{type_name, TypeId};
use std::fmt;

use crate::parallel_world::ParamAccess;
use crate::world::UnsafeWorldCell;

/// Error returned when a system parameter cannot be fetched from the world.
#[derive(Debug, Clone)]
pub enum SystemParamError {
    /// A required singleton was not found in the world.
    SingletonNotFound { type_name: &'static str },
}

impl fmt::Display for SystemParamError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SystemParamError::SingletonNotFound { type_name } => {
                write!(f, "singleton not found: {}", type_name)
            }
        }
    }
}

impl std::error::Error for SystemParamError {}

/// A parameter that can be used in a [`System`](super::System).
///
/// # Derive
///
/// This trait can be derived with the [`derive@super::SystemParam`] macro.
/// This macro only works if each field on the derived struct implements [`SystemParam`].
/// Note: There are additional requirements on the field types.
/// See the *Generic `SystemParam`s* section for details and workarounds of the probable
/// cause if this derive causes an error to be emitted.
///
/// Derived `SystemParam` structs may have two lifetimes: `'w` for data stored in the [`World`],
/// and `'s` for data stored in the parameter's state.
pub trait SystemParam: Sized {
    /// Cached state that persists across system runs.
    type State: Default + 'static;
    type Item<'world, 'state>: SystemParam<State = Self::State>;

    /// Creates the parameter from the world, using and updating the provided state cache.
    fn from_world_with_state<'w, 's>(
        world: UnsafeWorldCell<'w>,
        state: &'s mut Self::State,
    ) -> Result<Self::Item<'w, 's>, SystemParamError>;

    /// Creates the parameter from the world without state caching.
    /// This is a convenience method that creates a temporary state.
    fn from_world<'w>(world: UnsafeWorldCell<'w>) -> Result<Self::Item<'w, 'static>, SystemParamError> {
        // Use a leaked box to satisfy the 'static lifetime for stateless usage
        let state = Box::leak(Box::new(Self::State::default()));
        Self::from_world_with_state(world, state)
    }

    /// Returns the event TypeId if this parameter is an EventReader.
    /// Returns None for all other system parameters.
    fn event_type_id() -> Option<TypeId> {
        None
    }

    /// Returns true if this parameter is an EventReader.
    fn is_event_reader() -> bool {
        Self::event_type_id().is_some()
    }

    /// Returns the access pattern for this parameter.
    /// 
    /// Used by the parallel scheduler to determine which systems can
    /// run concurrently without conflicts.
    fn access() -> ParamAccess {
        // Default: no access (safe to run in parallel with anything)
        ParamAccess::none()
    }
}

pub type SystemParamItem<'w, 's, P> = <P as SystemParam>::Item<'w, 's>;

/// Helper function to create a singleton not found error with the type name.
pub fn singleton_not_found<T>() -> SystemParamError {
    SystemParamError::SingletonNotFound {
        type_name: type_name::<T>(),
    }
}

// Implement SystemParam for tuples
macro_rules! impl_system_param_tuple {
    () => {
        impl SystemParam for () {
            type State = ();
            type Item<'w, 's> = ();

            fn from_world_with_state<'w, 's>(
                _world: UnsafeWorldCell<'w>,
                _state: &'s mut Self::State,
            ) -> Result<Self::Item<'w, 's>, SystemParamError> {
                Ok(())
            }
        }
    };
    ($($param: ident),*) => {
        #[allow(non_snake_case)]
        impl<$($param: SystemParam),*> SystemParam for ($($param,)*) {
            type State = ($(<$param as SystemParam>::State,)*);
            type Item<'w, 's> = ($(SystemParamItem<'w, 's, $param>,)*);

            fn from_world_with_state<'w, 's>(
                world: UnsafeWorldCell<'w>,
                state: &'s mut Self::State,
            ) -> Result<Self::Item<'w, 's>, SystemParamError> {
                let ($($param,)*) = state;
                Ok(($(<$param as SystemParam>::from_world_with_state(world, $param)?,)*))
            }

            fn access() -> ParamAccess {
                let mut access = ParamAccess::none();
                $(access.merge(&<$param as SystemParam>::access());)*
                access
            }
        }
    }
}

impl_system_param_tuple!();
impl_system_param_tuple!(A);
impl_system_param_tuple!(A, B);
impl_system_param_tuple!(A, B, C);
impl_system_param_tuple!(A, B, C, D);
impl_system_param_tuple!(A, B, C, D, E);
impl_system_param_tuple!(A, B, C, D, E, F);
impl_system_param_tuple!(A, B, C, D, E, F, G);
impl_system_param_tuple!(A, B, C, D, E, F, G, H);
