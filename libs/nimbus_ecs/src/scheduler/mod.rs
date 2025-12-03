//! System scheduling and execution ordering.
//!
//! This module provides schedulers for organizing and executing systems.
//!
//! # Schedulers
//!
//! - [`PriorityScheduler`] - Sequential execution by priority phase
//! - [`ParallelPriorityScheduler`] - Parallel execution of non-conflicting systems
//!
//! # System Management
//!
//! Systems can be added, removed, enabled, and disabled at runtime:
//!
//! ```ignore
//! let physics_id = app.register_system(Update, physics_system);
//! let debug_id = app.register_system(Update, debug_overlay);
//!
//! // Disable debug overlay in release builds
//! app.set_system_enabled(debug_id, false);
//!
//! // Remove physics entirely
//! app.remove_system(physics_id);
//! ```
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

mod parallel;
mod priority;

pub use parallel::{ParallelPriorityScheduler, SystemAccess};
pub use priority::PriorityScheduler;

use std::sync::atomic::{AtomicU64, Ordering};

use crate::{system_param::SystemParamError, systems::{IntoSystem, System}, world::World};

// Global counter for unique system IDs
static NEXT_SYSTEM_ID: AtomicU64 = AtomicU64::new(1);

/// Unique identifier for a registered system.
///
/// Returned when registering a system, used to remove or enable/disable it later.
///
/// # Example
///
/// ```ignore
/// let id = scheduler.register(Update, my_system);
/// scheduler.set_enabled(id, false);  // Disable
/// scheduler.remove(id);               // Remove entirely
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SystemId(u64);

impl SystemId {
    /// Creates a new unique SystemId.
    pub(crate) fn new() -> Self {
        Self(NEXT_SYSTEM_ID.fetch_add(1, Ordering::Relaxed))
    }

    /// Returns the raw numeric ID (for debugging).
    pub fn raw(&self) -> u64 {
        self.0
    }
}

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
    /// Adds a boxed system to the scheduler with the specified priority.
    ///
    /// Returns a [`SystemId`] that can be used to remove or enable/disable the system.
    ///
    /// Prefer using [`register`](Self::register) for a more ergonomic API.
    fn add_system(&mut self, priority: P, system: Box<dyn System>) -> SystemId;

    /// Registers a system to run at the given priority.
    ///
    /// Returns a [`SystemId`] that can be used to remove or enable/disable the system.
    ///
    /// This is the ergonomic way to add systems - just pass a function:
    ///
    /// ```ignore
    /// let id = scheduler.register(SystemPriority::Update, my_system);
    /// scheduler.set_enabled(id, false);
    /// ```
    fn register<M>(&mut self, priority: P, system: impl IntoSystem<M>) -> SystemId {
        self.add_system(priority, Box::new(system.into_system()))
    }

    /// Removes a system from the scheduler.
    ///
    /// Returns `true` if the system was found and removed, `false` otherwise.
    fn remove(&mut self, id: SystemId) -> bool;

    /// Enables or disables a system.
    ///
    /// Disabled systems remain registered but are skipped during execution.
    /// This is more efficient than removing and re-adding systems that are
    /// frequently toggled.
    ///
    /// Returns `true` if the system was found, `false` otherwise.
    fn set_enabled(&mut self, id: SystemId, enabled: bool) -> bool;

    /// Returns whether a system is enabled.
    ///
    /// Returns `None` if the system ID is not found.
    fn is_enabled(&self, id: SystemId) -> Option<bool>;

    /// Returns the total number of systems in the scheduler (including disabled).
    fn system_count(&self) -> usize;

    /// Clears all systems from the scheduler.
    fn clear(&mut self);

    /// Runs all enabled systems in the scheduler's defined order.
    fn run(&mut self, world: &mut World) -> Result<(), SystemParamError>;
}
