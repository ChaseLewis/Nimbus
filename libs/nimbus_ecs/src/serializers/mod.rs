//! World serialization formats.
//!
//! This module provides different serialization formats for saving and loading
//! ECS worlds.
//!
//! # Formats
//!
//! - **nimbus**: Human-readable, line-based format (`.nimbus` files)
//! - **json**: Pure JSON format - faster but less readable
//! - **binary**: (TODO) Optimized binary format for release builds

pub mod nimbus;
pub mod json;

pub use nimbus::{NimbusSerializer, NimbusDeserializer, NimbusError};
pub use json::{JsonSerializer, JsonDeserializer, JsonError};

