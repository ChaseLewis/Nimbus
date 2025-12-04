//! Archetype-based entity storage with SoA component columns.
//!
//! Each archetype stores entities with the same component signature.
//! Components are stored in dense columns for cache-efficient iteration.

mod archetype;
mod archetypes;
mod key;

// Public API
pub use archetypes::Archetypes;
pub use key::ArchetypeKey;

// Crate-internal
pub(crate) use archetype::{Archetype, Column, ColumnData};
pub(crate) use archetypes::SendArchetypesPtr;