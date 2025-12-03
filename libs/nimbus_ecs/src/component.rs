//! Component trait definition.

/// Marker trait for data that can be stored on entities.
///
/// Components must be `'static` to be safely stored inside the world and are
/// required to be `Send + Sync` so they can be accessed from multiple systems.
/// Implement the trait manually or use `#[derive(Component)]` from the
/// `nimbus_macro_ecs` crate.
pub trait Component: 'static + Send + Sync {}
