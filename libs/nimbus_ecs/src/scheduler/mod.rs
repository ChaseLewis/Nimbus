//! System scheduling and execution ordering.
//!
//! This module provides a generic scheduler that can work with any priority
//! enum defined by the application.
//!
//! # Custom Priority Example
//!
//! ```
//! use nimbus_ecs::scheduler::Priority;
//!
//! #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
//! pub enum GamePhase {
//!     Input,
//!     Physics,
//!     Animation,
//!     GameLogic,
//!     Render,
//! }
//!
//! impl Priority for GamePhase {
//!     fn phases() -> &'static [Self] {
//!         &[
//!             Self::Input,
//!             Self::Physics,
//!             Self::Animation,
//!             Self::GameLogic,
//!             Self::Render,
//!         ]
//!     }
//! }
//! ```

mod priority;

pub use priority::PriorityScheduler;

use crate::{system_param::SystemParamError, systems::System, world::World};

/// Trait for priority enums that define system execution phases.
///
/// Implement this trait on your own enum to define custom execution phases
/// for your application.
///
/// # Requirements
///
/// - `phases()` must return all variants in the order they should execute
/// - The returned slice must be stable (same order every call)
///
/// # Example
///
/// ```
/// use nimbus_ecs::scheduler::Priority;
///
/// #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// pub enum MyPhases {
///     Early,
///     Main,
///     Late,
/// }
///
/// impl Priority for MyPhases {
///     fn phases() -> &'static [Self] {
///         &[Self::Early, Self::Main, Self::Late]
///     }
/// }
/// ```
pub trait Priority: Copy + Eq + std::hash::Hash + std::fmt::Debug + 'static {
    /// Returns all priority phases in execution order.
    ///
    /// Systems are executed in this order, with events cleared after the last phase.
    fn phases() -> &'static [Self];
}

/// Default priority levels for system execution order.
///
/// Systems are executed in order: StartOfFrame → PreUpdate → Update → PostUpdate → EndOfFrame.
/// Events are cleared after EndOfFrame.
///
/// This is the default priority type used by [`App`](crate::App).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SystemPriority {
    /// Runs at the very beginning of the frame.
    /// Use for reading events from the previous frame.
    StartOfFrame,
    /// Runs before the main update phase.
    /// Use for input handling, time updates, etc.
    PreUpdate,
    /// The main update phase.
    Update,
    /// Runs after the main update phase.
    /// Use for rendering preparation, cleanup, etc.
    PostUpdate,
    /// Runs at the very end of the frame, before events are cleared.
    /// Use for final cleanup, late event handling, etc.
    EndOfFrame,
}

impl Priority for SystemPriority {
    fn phases() -> &'static [Self] {
        &[
            Self::StartOfFrame,
            Self::PreUpdate,
            Self::Update,
            Self::PostUpdate,
            Self::EndOfFrame,
        ]
    }
}

/// Trait for system schedulers that organize and execute systems.
pub trait Scheduler<P: Priority>: Default {
    /// Adds a system to the scheduler with the specified priority.
    fn add_system(&mut self, priority: P, system: Box<dyn System>);

    /// Returns the total number of systems in the scheduler.
    fn system_count(&self) -> usize;

    /// Clears all systems from the scheduler.
    fn clear(&mut self);

    /// Runs all systems in the scheduler's defined order.
    fn run(&mut self, world: &mut World) -> Result<(), SystemParamError>;
}
