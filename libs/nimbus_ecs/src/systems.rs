//! System execution and function-to-system conversion.

use std::any::TypeId;
use std::cell::RefCell;
use std::marker::PhantomData;

use crate::{
    commands::CommandQueue,
    system_param::{SystemParam, SystemParamError, SystemParamItem},
    world::{UnsafeWorldCell, World},
};

pub trait System: 'static {
    /// Runs the system with a shared command queue for deferred mutations.
    ///
    /// This is the primary method used by sequential schedulers.
    fn run_with_commands(
        &mut self,
        world: &mut World,
        commands: &RefCell<CommandQueue>,
    ) -> Result<(), SystemParamError>;

    /// Runs the system with an exclusive command buffer.
    ///
    /// This is used by parallel schedulers where each system gets its own
    /// command buffer from a [`ParallelCommandBuffers`](crate::ParallelCommandBuffers) pool.
    ///
    /// The buffer is wrapped in a temporary RefCell for the duration of the call.
    fn run_with_buffer(
        &mut self,
        world: &mut World,
        buffer: &mut CommandQueue,
    ) -> Result<(), SystemParamError> {
        // Temporarily wrap in RefCell for UnsafeWorldCell compatibility
        // This is safe because we have exclusive access to the buffer
        let cell = RefCell::new(std::mem::take(buffer));
        let result = self.run_with_commands(world, &cell);
        *buffer = cell.into_inner();
        result
    }

    /// Runs the system without a command queue (creates a temporary one and applies immediately).
    fn run(&mut self, world: &mut World) -> Result<(), SystemParamError> {
        let queue = RefCell::new(CommandQueue::new());
        self.run_with_commands(world, &queue)?;
        queue.borrow_mut().apply(world);
        Ok(())
    }

    /// Returns the event TypeId if this system's first parameter is an EventReader.
    fn event_type_id(&self) -> Option<TypeId> {
        None
    }

    /// Returns true if this system's first parameter is an EventReader.
    fn is_event_reader(&self) -> bool {
        self.event_type_id().is_some()
    }
}

/// Helper trait to extract event type from the first parameter of a tuple.
pub trait FirstParamEventType {
    fn first_event_type_id() -> Option<TypeId>;
}

// Implement for unit (no params)
impl FirstParamEventType for () {
    fn first_event_type_id() -> Option<TypeId> {
        None
    }
}

// Implement for tuples - checks the first element
macro_rules! impl_first_param_event_type {
    ($first:ident $(, $rest:ident)*) => {
        impl<$first: SystemParam $(, $rest: SystemParam)*> FirstParamEventType for ($first, $($rest,)*) {
            fn first_event_type_id() -> Option<TypeId> {
                $first::event_type_id()
            }
        }
    };
}

impl_first_param_event_type!(A);
impl_first_param_event_type!(A, B);
impl_first_param_event_type!(A, B, C);
impl_first_param_event_type!(A, B, C, D);
impl_first_param_event_type!(A, B, C, D, E);
impl_first_param_event_type!(A, B, C, D, E, F);
impl_first_param_event_type!(A, B, C, D, E, F, G);
impl_first_param_event_type!(A, B, C, D, E, F, G, H);

pub trait SystemParamFunction<Marker>: 'static {
    type Param: SystemParam;
    fn call(&mut self, param: SystemParamItem<Self::Param>);
}

macro_rules! impl_system_function {
    () => {
        impl<Func> SystemParamFunction<fn()> for Func
        where
            Func: Send + Sync + 'static + FnMut(),
        {
            type Param = ();

            #[inline]
            fn call(&mut self, _param: SystemParamItem<'_, '_, ()>) {
                self();
            }
        }
    };
    ($($param: ident),*) => {
        #[allow(non_snake_case)]
        impl<Func, $($param: SystemParam),*> SystemParamFunction<fn($($param,)*)> for Func
        where
            Func: Send + Sync + 'static,
            for<'a> &'a mut Func: FnMut($($param),*) + FnMut($(SystemParamItem<$param>),*)
        {
            type Param = ($($param,)*);

            #[inline]
            fn call(&mut self, param: SystemParamItem<'_, '_, ($($param,)*)>) {
                #[allow(clippy::too_many_arguments)]
                fn call_inner<$($param,)*>(
                    mut f: impl FnMut($($param,)*),
                    $($param: $param,)*
                ) {
                    f($($param),*)
                }
                let ($($param,)*) = param;
                call_inner(self, $($param),*)
            }
        }
    }
}

#[allow(non_snake_case)]
const _: () = {
    impl_system_function!();
    impl_system_function!(A);
    impl_system_function!(A, B);
    impl_system_function!(A, B, C);
    impl_system_function!(A, B, C, D);
    impl_system_function!(A, B, C, D, E);
    impl_system_function!(A, B, C, D, E, F);
    impl_system_function!(A, B, C, D, E, F, G);
    impl_system_function!(A, B, C, D, E, F, G, H);
};

pub struct FunctionSystem<F, Marker>
where
    F: SystemParamFunction<Marker>,
{
    func: F,
    /// Cached state for system parameters (e.g., query archetype matches)
    state: <F::Param as SystemParam>::State,
    _marker: PhantomData<Marker>,
}

impl<F, Marker> FunctionSystem<F, Marker>
where
    F: SystemParamFunction<Marker>,
{
    pub fn new(func: F) -> Self {
        Self {
            func,
            state: Default::default(),
            _marker: PhantomData,
        }
    }
}

impl<Marker: 'static, F: SystemParamFunction<Marker>> System for FunctionSystem<F, Marker>
where
    F::Param: FirstParamEventType,
{
    fn run_with_commands(
        &mut self,
        world: &mut World,
        commands: &RefCell<CommandQueue>,
    ) -> Result<(), SystemParamError> {
        let cell = UnsafeWorldCell::new(world, commands);
        let param = <F::Param as SystemParam>::from_world_with_state(cell, &mut self.state)?;
        self.func.call(param);
        Ok(())
    }

    fn event_type_id(&self) -> Option<TypeId> {
        <F::Param as FirstParamEventType>::first_event_type_id()
    }
}

pub trait IntoSystem<Marker>: Sized {
    type System: System;
    fn into_system(self) -> Self::System;
}

impl<F, Marker: 'static> IntoSystem<Marker> for F
where
    F: SystemParamFunction<Marker>,
    F::Param: FirstParamEventType,
{
    type System = FunctionSystem<F, Marker>;
    fn into_system(self) -> Self::System {
        FunctionSystem::new(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Component, Query, World};

    fn empty_system() {}

    #[derive(Component)]
    struct Position {
        x: f32,
        y: f32,
    }

    #[derive(Component)]
    struct Velocity {
        x: f32,
        y: f32,
    }

    fn query_system(mut query: Query<&mut Position>) {
        for pos in query.iter() {
            pos.x += 1.0;
            pos.y += 1.0;
        }
    }

    // Bevy-style reference syntax
    fn query_system_ref_syntax(mut query: Query<(&mut Position, &Velocity)>) {
        for (pos, vel) in query.iter() {
            pos.x += vel.x;
            pos.y += vel.y;
        }
    }

    fn apply_velocity(mut query: Query<(&mut Position, &Velocity)>) {
        for (pos, vel) in query.iter() {
            pos.x += vel.x;
            pos.y += vel.y;
        }
    }

    #[test]
    fn test_into_system() {
        let _system = empty_system.into_system();
    }

    #[test]
    fn test_query_system() {
        let _system = query_system.into_system();
    }

    #[test]
    fn test_query_system_ref_syntax() {
        let _system = query_system_ref_syntax.into_system();
    }

    #[test]
    fn systems_mutate_components() {
        let mut world = World::new();
        let entity = world.spawn_with(Position { x: 0.0, y: 0.0 });
        world.insert(entity, Velocity { x: 1.5, y: 2.25 }).unwrap();

        world.run_system(apply_velocity).unwrap();

        let position = world.get::<Position>(entity).unwrap();
        assert_eq!((position.x, position.y), (1.5, 2.25));
    }

    #[test]
    fn system_event_type_introspection() {
        use std::any::TypeId;
        use crate::EventReader;

        #[derive(Clone)]
        struct DamageEvent { _amount: i32 }

        #[derive(Clone)]
        struct HealEvent { _amount: i32 }

        // Event handler system
        fn deal_damage(_events: EventReader<DamageEvent>, mut _query: Query<&mut Position>) {}
        
        fn heal(_events: EventReader<HealEvent>) {}

        // Non-event system
        fn regular_system(mut _query: Query<&Position>) {}

        // Event handler is detected
        let damage_system = deal_damage.into_system();
        assert!(damage_system.is_event_reader());
        assert_eq!(damage_system.event_type_id(), Some(TypeId::of::<DamageEvent>()));

        let heal_system = heal.into_system();
        assert!(heal_system.is_event_reader());
        assert_eq!(heal_system.event_type_id(), Some(TypeId::of::<HealEvent>()));

        // Different event types
        assert_ne!(damage_system.event_type_id(), heal_system.event_type_id());

        // Regular system is not an event handler
        let regular = regular_system.into_system();
        assert!(!regular.is_event_reader());
        assert_eq!(regular.event_type_id(), None);

        // Empty system
        let empty = empty_system.into_system();
        assert!(!empty.is_event_reader());
        assert_eq!(empty.event_type_id(), None);
    }
}
