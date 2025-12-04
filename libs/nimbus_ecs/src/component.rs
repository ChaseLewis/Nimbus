//! Component trait and identification.

use std::fmt;
use std::hash::{Hash, Hasher};

// ============================================================================
// Const Hash Functions
// ============================================================================

/// FNV-1a 64-bit offset basis
const FNV_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
/// FNV-1a 64-bit prime
const FNV_PRIME: u64 = 0x100000001b3;

/// MurmurHash3-style finalizer - dramatically improves avalanche properties.
/// Each bit of input affects ~50% of output bits.
#[inline]
const fn fmix64(mut h: u64) -> u64 {
    h ^= h >> 33;
    h = h.wrapping_mul(0xff51afd7ed558ccd);
    h ^= h >> 33;
    h = h.wrapping_mul(0xc4ceb9fe1a85ec53);
    h ^= h >> 33;
    h
}

/// Computes a well-distributed 64-bit hash of a byte slice at compile time.
/// Uses FNV-1a for mixing followed by MurmurHash3 finalizer for avalanche.
pub const fn const_fnv1a_64(bytes: &[u8]) -> u64 {
    let mut hash = FNV_OFFSET_BASIS;
    let mut i = 0;
    while i < bytes.len() {
        hash ^= bytes[i] as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
        i += 1;
    }
    // Finalize: spread bits for better distribution in hash tables
    fmix64(hash)
}

/// Computes a well-distributed 64-bit hash of a string at compile time.
pub const fn const_fnv1a_64_str(s: &str) -> u64 {
    const_fnv1a_64(s.as_bytes())
}

// ============================================================================
// ComponentId
// ============================================================================

/// Unique identifier for a component type.
///
/// Uses a 64-bit FNV-1a hash of the fully qualified type name, computed at
/// compile time. This provides:
/// - Stable IDs across builds (same name = same ID)
/// - No runtime TypeId dependency
/// - User-overridable via `#[component(id = ...)]`
///
/// Collision detection happens at runtime during `ComponentRegistry` initialization.
#[repr(transparent)]
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct ComponentId(pub(crate) u64);

impl ComponentId {
    /// Creates a `ComponentId` from a raw u64 value.
    ///
    /// This is typically called by the `#[component]` macro with a compile-time
    /// hash or user-provided ID.
    #[inline]
    pub const fn new(id: u64) -> Self {
        ComponentId(id)
    }
    
    /// Returns the `ComponentId` for type `T`.
    ///
    /// This reads the compile-time computed ID from the `Component` trait.
    #[inline]
    pub fn of<T: Component>() -> Self {
        T::COMPONENT_ID
    }
    
    /// Creates a `ComponentId` from a TypeId.
    ///
    /// This is for internal use and testing where types may not implement `Component`.
    /// The resulting ID is based on the TypeId's hash and may not be stable across builds.
    #[inline]
    pub(crate) fn from_type_id(type_id: std::any::TypeId) -> Self {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        type_id.hash(&mut hasher);
        ComponentId(hasher.finish())
    }
    
    /// Returns the raw u64 value of this ID.
    #[inline]
    pub const fn raw(&self) -> u64 {
        self.0
    }
}

impl Hash for ComponentId {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl fmt::Debug for ComponentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ComponentId({:#018x})", self.0)
    }
}

impl fmt::Display for ComponentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:#018x}", self.0)
    }
}

// ============================================================================
// Component Trait
// ============================================================================

/// Marker trait for data that can be stored on entities.
///
/// Components must be `'static` to be safely stored inside the world and are
/// required to be `Send + Sync` so they can be accessed from multiple systems.
///
/// # Implementation
///
/// Use the `#[component]` attribute macro to implement this trait:
///
/// ```ignore
/// use nimbus_ecs::component;
///
/// #[component]
/// struct Position { x: f32, y: f32 }
///
/// // With custom ID (for stable serialization):
/// #[component(id = 0x1234567890ABCDEF)]
/// struct Velocity { x: f32, y: f32 }
///
/// // With serialization support:
/// #[component(serializable)]
/// struct Health { current: f32, max: f32 }
/// ```
pub trait Component: 'static + Send + Sync {
    /// The compile-time computed unique identifier for this component type.
    const COMPONENT_ID: ComponentId;
}
