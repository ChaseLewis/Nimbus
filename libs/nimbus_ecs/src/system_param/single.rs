//! Singleton system parameters for accessing global resources.

use std::ops::{Deref, DerefMut};

use crate::{world::UnsafeWorldCell, Component};

use super::{singleton_not_found, SystemParam, SystemParamError};

/// Provides immutable access to a singleton component.
pub struct Single<'a, T: Component> {
    value: &'a T,
}

impl<'a, T: Component> Deref for Single<'a, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.value
    }
}

impl<'a, T: Component> SystemParam for Single<'a, T> {
    type State = ();
    type Item<'w, 's> = Single<'w, T>;

    fn from_world_with_state<'w, 's>(
        world: UnsafeWorldCell<'w>,
        _state: &'s mut Self::State,
    ) -> Result<Self::Item<'w, 's>, SystemParamError> {
        let value = world
            .get_singleton::<T>()
            .ok_or_else(singleton_not_found::<T>)?;
        let ptr = value as *const T;
        // SAFETY: The pointer is valid because it comes from a valid reference `value`,
        // and we're extending its lifetime to 'w which is safe in the context of UnsafeWorldCell.
        Ok(unsafe { Single { value: &*ptr } })
    }
}

/// Provides mutable access to a singleton component.
pub struct SingleMut<'a, T: Component> {
    value: &'a mut T,
}

impl<'a, T: Component> Deref for SingleMut<'a, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.value
    }
}

impl<'a, T: Component> DerefMut for SingleMut<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.value
    }
}

impl<'a, T: Component> SystemParam for SingleMut<'a, T> {
    type State = ();
    type Item<'w, 's> = SingleMut<'w, T>;

    fn from_world_with_state<'w, 's>(
        world: UnsafeWorldCell<'w>,
        _state: &'s mut Self::State,
    ) -> Result<Self::Item<'w, 's>, SystemParamError> {
        let value = world
            .as_mut()
            .get_singleton_mut::<T>()
            .ok_or_else(singleton_not_found::<T>)?;
        Ok(SingleMut { value })
    }
}

// ============================================================================
// Option<Single<T>> - optional singleton access
// ============================================================================

impl<'a, T: Component> SystemParam for Option<Single<'a, T>> {
    type State = ();
    type Item<'w, 's> = Option<Single<'w, T>>;

    fn from_world_with_state<'w, 's>(
        world: UnsafeWorldCell<'w>,
        _state: &'s mut Self::State,
    ) -> Result<Self::Item<'w, 's>, SystemParamError> {
        let result = world.get_singleton::<T>().map(|value| {
            let ptr = value as *const T;
            // SAFETY: The pointer is valid because it comes from a valid reference,
            // and we're extending its lifetime to 'w which is safe in the context of UnsafeWorldCell.
            unsafe { Single { value: &*ptr } }
        });
        Ok(result)
    }
}

// ============================================================================
// Option<SingleMut<T>> - optional mutable singleton access
// ============================================================================

impl<'a, T: Component> SystemParam for Option<SingleMut<'a, T>> {
    type State = ();
    type Item<'w, 's> = Option<SingleMut<'w, T>>;

    fn from_world_with_state<'w, 's>(
        world: UnsafeWorldCell<'w>,
        _state: &'s mut Self::State,
    ) -> Result<Self::Item<'w, 's>, SystemParamError> {
        let result = world.as_mut().get_singleton_mut::<T>().map(|value| {
            SingleMut { value }
        });
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::{Single, SingleMut};
    use crate::{Component, Query, World};

    #[derive(Component, Debug)]
    struct Counter {
        value: i32,
    }

    #[derive(Component)]
    struct Health {
        value: f32,
    }

    #[derive(Component)]
    struct GameConfig {
        damage_multiplier: f32,
    }

    #[test]
    fn singleton_systems() {
        let mut world = World::new();
        world.insert_singleton(Counter { value: 0 });

        fn increment_counter(mut counter: SingleMut<Counter>) {
            // Use explicit dereference - (*counter) gives &mut Counter
            (*counter).value += 1;
        }

        world.run_system(increment_counter).unwrap();
        world.run_system(increment_counter).unwrap();
        world.run_system(increment_counter).unwrap();

        let value = world.get_singleton::<Counter>().unwrap().value;
        assert_eq!(value, 3);
    }

    #[test]
    fn singleton_and_query_system() {
        let mut world = World::new();
        world.insert_singleton(GameConfig {
            damage_multiplier: 2.0,
        });
        let entity = world.spawn_with(Health { value: 100.0 });

        fn apply_damage(mut query: Query<&mut Health>, config: Single<GameConfig>) {
            for health in query.iter() {
                health.value -= 10.0 * config.damage_multiplier;
            }
        }

        world.run_system(apply_damage).unwrap();

        let health = world.get::<Health>(entity).unwrap();
        assert_eq!(health.value, 80.0); // 100 - (10 * 2.0)
    }

    #[test]
    fn optional_singleton_when_present() {
        let mut world = World::new();
        world.insert_singleton(Counter { value: 10 });

        fn read_optional(counter: Option<Single<Counter>>) {
            assert!(counter.is_some());
            assert_eq!((*counter.unwrap()).value, 10);
        }

        world.run_system(read_optional).unwrap();
    }

    #[test]
    fn optional_singleton_when_missing() {
        let mut world = World::new();
        // Don't insert Counter singleton

        fn read_optional(counter: Option<Single<Counter>>) {
            assert!(counter.is_none());
        }

        // System runs successfully even without the singleton
        world.run_system(read_optional).unwrap();
    }

    #[test]
    fn optional_singleton_mut_when_present() {
        let mut world = World::new();
        world.insert_singleton(Counter { value: 5 });

        fn modify_optional(counter: Option<SingleMut<Counter>>) {
            if let Some(mut c) = counter {
                (*c).value += 10;
            }
        }

        world.run_system(modify_optional).unwrap();

        assert_eq!(world.get_singleton::<Counter>().unwrap().value, 15);
    }

    #[test]
    fn optional_singleton_mut_when_missing() {
        let mut world = World::new();
        // Don't insert Counter singleton

        fn modify_optional(counter: Option<SingleMut<Counter>>) {
            assert!(counter.is_none());
        }

        // System runs successfully even without the singleton
        world.run_system(modify_optional).unwrap();
    }

    #[test]
    fn required_singleton_errors_when_missing() {
        use crate::system_param::SystemParamError;

        let mut world = World::new();
        // Don't insert Counter singleton

        fn requires_counter(_counter: Single<Counter>) {
            panic!("This should never run!");
        }

        // System should return an error, not run
        let result = world.run_system(requires_counter);
        assert!(result.is_err());
        
        match result.unwrap_err() {
            SystemParamError::SingletonNotFound { type_name } => {
                assert!(type_name.contains("Counter"));
            }
        }
    }

    #[test]
    fn required_singleton_mut_errors_when_missing() {
        use crate::system_param::SystemParamError;

        let mut world = World::new();
        // Don't insert Counter singleton

        fn requires_counter_mut(_counter: SingleMut<Counter>) {
            panic!("This should never run!");
        }

        // System should return an error, not run
        let result = world.run_system(requires_counter_mut);
        assert!(result.is_err());
        
        match result.unwrap_err() {
            SystemParamError::SingletonNotFound { type_name } => {
                assert!(type_name.contains("Counter"));
            }
        }
    }
}
