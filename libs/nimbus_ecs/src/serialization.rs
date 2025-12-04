//! Serialization support for ECS components.
//!
//! Components marked with `#[component(serializable)]` are automatically
//! registered and can be serialized/deserialized.
//!
//! # Example
//!
//! ```ignore
//! use nimbus_ecs::{component, ComponentRegistry, serialize, deserialize};
//!
//! #[component(serializable)]
//! struct Position { x: f32, y: f32 }
//!
//! // Access the global registry from anywhere
//! let registry = ComponentRegistry::global();
//!
//! // Check if a type is registered
//! if registry.get::<Position>().is_some() {
//!     // Serialize directly (no type erasure needed)
//!     let mut buffer = Vec::new();
//!     serialize(&position, &mut buffer)?;
//!     
//!     // Deserialize
//!     let restored: Position = deserialize(&mut cursor)?;
//! }
//! ```

use std::any::TypeId;
use std::sync::LazyLock;
use hashbrown::HashMap;
use std::io::{Read, Write};
use crate::archetype::{Column, ColumnData};
use crate::util::TypeHashMap;

/// Error type for serialization operations.
#[derive(Debug)]
pub enum Error {
    /// IO error during read/write
    Io(std::io::Error),
    /// Serialization format error
    Bincode(bincode::Error),
    /// Unknown component type during deserialization
    UnknownType { type_name: String },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Io(e) => write!(f, "IO error: {}", e),
            Error::Bincode(e) => write!(f, "serialization error: {}", e),
            Error::UnknownType { type_name } => {
                write!(f, "unknown component type: {}", type_name)
            }
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}

impl From<bincode::Error> for Error {
    fn from(e: bincode::Error) -> Self {
        Error::Bincode(e)
    }
}

/// Function type for serialization from raw pointer.
/// Safety: src must be a valid pointer to a value of the registered type.
type SerializeFn = unsafe fn(src: *const u8, writer: &mut dyn Write) -> Result<(), Error>;
/// Function type for deserialization into raw pointer.
/// Safety: dst must be a valid, properly aligned pointer with enough space for the type.
type DeserializeFn = unsafe fn(reader: &mut dyn Read, dst: *mut u8) -> Result<(), Error>;
/// Function type for getting type_id
type TypeIdFn = fn() -> TypeId;
/// Function type for creating an empty column
type ColumnFactoryFn = fn() -> Box<dyn Column>;
/// Function type for pushing raw bytes to a column
/// Safety: column must be the correct type, src must point to valid data
type ColumnPushFn = unsafe fn(column: &mut dyn Column, src: *const u8);

/// Registration entry for a serializable component.
///
/// These are collected at link time via the `inventory` crate.
/// Uses function pointers to be const-constructible.
///
/// # Pointer-based API
///
/// The serialize/deserialize functions operate on raw pointers, which is ideal for
/// ECS use cases where components are stored in contiguous archetype columns:
///
/// ```ignore
/// // Serialize directly from archetype column
/// unsafe {
///     reg.serialize_ptr(column.get_ptr(row), &mut writer)?;
/// }
///
/// // Deserialize: create column, push raw data
/// let mut column = reg.create_column();
/// let mut temp = vec![0u8; reg.size()];
/// reg.deserialize_ptr(&mut reader, temp.as_mut_ptr())?;
/// unsafe { reg.push_to_column(&mut *column, temp.as_ptr()); }
/// ```
pub struct ComponentRegistration {
    /// Human-readable type name
    pub type_name: &'static str,
    /// Size of the component type in bytes
    size: usize,
    /// Alignment of the component type
    align: usize,
    /// Function to get the runtime type identifier
    type_id_fn: TypeIdFn,
    /// Serialization function (reads from raw pointer)
    serialize_fn: SerializeFn,
    /// Deserialization function (writes to raw pointer)
    deserialize_fn: DeserializeFn,
    /// Creates an empty column for this component type
    column_factory_fn: ColumnFactoryFn,
    /// Pushes raw component bytes to a column
    column_push_fn: ColumnPushFn,
}

impl ComponentRegistration {
    /// Creates a new registration for type `T`.
    ///
    /// This is typically called by the `#[component(serializable)]` macro.
    /// The `type_name` parameter is provided by the macro since `std::any::type_name`
    /// is not yet const-stable.
    pub const fn new<T>(type_name: &'static str) -> Self
    where
        T: serde::Serialize + for<'de> serde::Deserialize<'de> + Send + Sync + 'static,
    {
        ComponentRegistration {
            type_name,
            size: std::mem::size_of::<T>(),
            align: std::mem::align_of::<T>(),
            type_id_fn: || TypeId::of::<T>(),
            serialize_fn: |src, writer| {
                // Safety: caller guarantees src points to a valid T
                unsafe { serialize_from_ptr::<T>(src, writer) }
            },
            deserialize_fn: |reader, dst| {
                // Safety: caller guarantees dst is valid, aligned, and has space for T
                unsafe { deserialize_to_ptr::<T>(reader, dst) }
            },
            column_factory_fn: || Box::new(ColumnData::<T>::new()),
            column_push_fn: |column, src| {
                // Safety: caller guarantees column is ColumnData<T> and src is valid T
                unsafe { push_to_column_typed::<T>(column, src) }
            },
        }
    }
    
    /// Returns the type ID for this component.
    #[inline]
    pub fn type_id(&self) -> TypeId {
        (self.type_id_fn)()
    }
    
    /// Returns the size of this component type in bytes.
    #[inline]
    pub fn size(&self) -> usize {
        self.size
    }
    
    /// Returns the alignment of this component type.
    #[inline]
    pub fn align(&self) -> usize {
        self.align
    }
    
    /// Serializes a component from a raw pointer.
    ///
    /// # Safety
    ///
    /// `src` must be a valid pointer to a value of this component's type.
    #[inline]
    pub unsafe fn serialize_ptr(&self, src: *const u8, writer: &mut dyn Write) -> Result<(), Error> {
        // Safety: caller guarantees src is valid; we forward that guarantee to the fn pointer
        unsafe { (self.serialize_fn)(src, writer) }
    }
    
    /// Deserializes a component into a raw pointer.
    ///
    /// # Safety
    ///
    /// - `dst` must be a valid pointer with proper alignment for this component type.
    /// - `dst` must have enough space to hold this component (use `self.size()`).
    /// - If `dst` points to an initialized value, it will be overwritten without dropping.
    #[inline]
    pub unsafe fn deserialize_ptr(&self, reader: &mut dyn Read, dst: *mut u8) -> Result<(), Error> {
        // Safety: caller guarantees dst is valid, aligned, and has space; we forward that
        unsafe { (self.deserialize_fn)(reader, dst) }
    }
    
    /// Creates an empty column for this component type.
    ///
    /// Use this when deserializing to dynamically create archetype columns.
    #[inline]
    pub(crate) fn create_column(&self) -> Box<dyn Column> {
        (self.column_factory_fn)()
    }
    
    /// Pushes a component from raw bytes to a column.
    ///
    /// # Safety
    ///
    /// - `column` must be a column created by this registration's `create_column()`.
    /// - `src` must be a valid pointer to initialized data of this component's type.
    #[inline]
    pub(crate) unsafe fn push_to_column(&self, column: &mut dyn Column, src: *const u8) {
        // Safety: caller guarantees column type matches and src is valid
        unsafe { (self.column_push_fn)(column, src) }
    }
}

/// Serializes a value directly without type erasure.
///
/// This is more efficient than going through `ComponentRegistration::serialize_any`.
pub fn serialize<T, W>(value: &T, writer: W) -> Result<(), Error>
where
    T: serde::Serialize,
    W: Write,
{
    bincode::serialize_into(writer, value)?;
    Ok(())
}

/// Deserializes a value directly without type erasure.
///
/// This is more efficient than going through `ComponentRegistration::deserialize_any`.
pub fn deserialize<T, R>(reader: R) -> Result<T, Error>
where
    T: for<'de> serde::Deserialize<'de>,
    R: Read,
{
    let value: T = bincode::deserialize_from(reader)?;
    Ok(value)
}

// These helper functions are monomorphized for each T
// Safety: caller guarantees src points to a valid T
unsafe fn serialize_from_ptr<T>(src: *const u8, writer: &mut dyn Write) -> Result<(), Error>
where
    T: serde::Serialize,
{
    // Safety: caller guarantees src points to a valid T
    let value = unsafe { &*(src as *const T) };
    bincode::serialize_into(writer, value)?;
    Ok(())
}

// Safety: caller guarantees dst is valid, properly aligned, and has space for T
unsafe fn deserialize_to_ptr<T>(reader: &mut dyn Read, dst: *mut u8) -> Result<(), Error>
where
    T: for<'de> serde::Deserialize<'de>,
{
    let value: T = bincode::deserialize_from(reader)?;
    // Safety: caller guarantees dst is valid, aligned, and has space for T
    unsafe { std::ptr::write(dst as *mut T, value) };
    Ok(())
}

// Safety: caller guarantees column is ColumnData<T> and src points to valid T
unsafe fn push_to_column_typed<T: 'static>(column: &mut dyn Column, src: *const u8) {
    let column = column
        .as_any_mut()
        .downcast_mut::<ColumnData<T>>()
        .expect("column type mismatch in push_to_column");
    // Read the value from src (takes ownership)
    // Safety: caller guarantees src points to valid initialized T
    let value = unsafe { std::ptr::read(src as *const T) };
    column.push(value);
}

// Tell inventory to collect ComponentRegistration instances
inventory::collect!(ComponentRegistration);

/// Global static registry, lazily initialized from inventory.
static GLOBAL_REGISTRY: LazyLock<ComponentRegistry> = LazyLock::new(ComponentRegistry::from_inventory);

/// Registry of all serializable components.
///
/// Built automatically from components registered via `#[component(serializable)]`.
///
/// # Global Access
///
/// Use [`ComponentRegistry::global()`] to access the static instance:
///
/// ```ignore
/// let reg = ComponentRegistry::global().get::<Position>();
/// ```
pub struct ComponentRegistry {
    by_type_id: TypeHashMap<&'static ComponentRegistration>,
    by_name: HashMap<String, &'static ComponentRegistration>,
}

impl ComponentRegistry {
    /// Returns the global static registry instance.
    ///
    /// This is lazily initialized on first access and discovers all
    /// `#[component(serializable)]` types automatically.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use nimbus_ecs::ComponentRegistry;
    ///
    /// // Access from anywhere without passing registry around
    /// if let Some(reg) = ComponentRegistry::global().get::<Position>() {
    ///     serialize(&position, &mut buffer)?;
    /// }
    /// ```
    #[inline]
    pub fn global() -> &'static ComponentRegistry {
        &GLOBAL_REGISTRY
    }

    /// Creates a new empty registry.
    pub fn new() -> Self {
        Self {
            by_type_id: HashMap::default(),
            by_name: HashMap::new(),
        }
    }

    /// Builds registry from all components registered via inventory.
    /// 
    /// Note: Prefer using [`ComponentRegistry::global()`] for the cached static instance.
    /// This method rebuilds the registry each time it's called.
    pub fn from_inventory() -> Self {
        let mut registry = Self::new();
        for entry in inventory::iter::<ComponentRegistration> {
            registry.by_type_id.insert(entry.type_id(), entry);
            registry.by_name.insert(entry.type_name.to_string(), entry);
        }
        registry
    }

    /// Gets registration for a specific component type.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let registry = ComponentRegistry::from_inventory();
    /// if let Some(reg) = registry.get::<Position>() {
    ///     reg.serialize(&position, &mut buffer)?;
    /// }
    /// ```
    pub fn get<T: 'static>(&self) -> Option<&'static ComponentRegistration> {
        self.by_type_id.get(&TypeId::of::<T>()).copied()
    }

    /// Gets registration by type ID.
    pub fn get_by_type_id(&self, type_id: TypeId) -> Option<&'static ComponentRegistration> {
        self.by_type_id.get(&type_id).copied()
    }

    /// Gets registration by type name.
    pub fn get_by_name(&self, name: &str) -> Option<&'static ComponentRegistration> {
        self.by_name.get(name).copied()
    }

    /// Returns an iterator over all registered components.
    pub fn iter(&self) -> impl Iterator<Item = &'static ComponentRegistration> + '_ {
        self.by_type_id.values().copied()
    }

    /// Returns the number of registered components.
    pub fn len(&self) -> usize {
        self.by_type_id.len()
    }

    /// Returns true if no components are registered.
    pub fn is_empty(&self) -> bool {
        self.by_type_id.is_empty()
    }
}

impl Default for ComponentRegistry {
    fn default() -> Self {
        Self::from_inventory()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::component;
    
    // Test serializable component using the new attribute macro
    #[component(serializable)]
    struct TestPosition {
        x: f32,
        y: f32,
    }
    
    // Test non-serializable component
    #[component]
    struct TestVelocity {
        dx: f32,
        dy: f32,
    }

    #[test]
    fn registry_global_access() {
        // Test the global static accessor
        let registry = ComponentRegistry::global();
        
        // Should have at least TestPosition registered
        assert!(registry.len() >= 1);
        
        // TestPosition should be findable using get<T>()
        let pos_reg = registry.get::<TestPosition>();
        assert!(pos_reg.is_some());
        
        // TestVelocity should NOT be in the registry (not serializable)
        let vel_reg = registry.get::<TestVelocity>();
        assert!(vel_reg.is_none());
        
        // Multiple calls return the same instance
        let registry2 = ComponentRegistry::global();
        assert!(std::ptr::eq(registry, registry2));
    }
    
    #[test]
    fn serialize_deserialize_roundtrip() {
        let original = TestPosition { x: 1.5, y: 2.5 };
        
        // Serialize using generic function
        let mut buffer = Vec::new();
        super::serialize(&original, &mut buffer).unwrap();
        
        // Deserialize using generic function
        let mut cursor = std::io::Cursor::new(buffer);
        let restored: TestPosition = super::deserialize(&mut cursor).unwrap();
        
        // Verify
        assert_eq!(restored.x, 1.5);
        assert_eq!(restored.y, 2.5);
    }
    
    #[test]
    fn serialize_deserialize_ptr_based() {
        let registry = ComponentRegistry::from_inventory();
        let reg = registry.get::<TestPosition>().unwrap();
        
        // Verify size and alignment are correct
        assert_eq!(reg.size(), std::mem::size_of::<TestPosition>());
        assert_eq!(reg.align(), std::mem::align_of::<TestPosition>());
        
        let original = TestPosition { x: 1.5, y: 2.5 };
        
        // Serialize from raw pointer
        let mut buffer = Vec::new();
        unsafe {
            reg.serialize_ptr(&original as *const _ as *const u8, &mut buffer).unwrap();
        }
        
        // Deserialize to raw pointer (stack-allocated destination)
        let mut restored = std::mem::MaybeUninit::<TestPosition>::uninit();
        let mut cursor = std::io::Cursor::new(buffer);
        unsafe {
            reg.deserialize_ptr(&mut cursor, restored.as_mut_ptr() as *mut u8).unwrap();
        }
        
        // Verify
        let restored = unsafe { restored.assume_init() };
        assert_eq!(restored.x, 1.5);
        assert_eq!(restored.y, 2.5);
    }
    
    #[test]
    fn serialize_deserialize_to_vec() {
        // Demonstrates writing directly to a pre-allocated buffer
        // (simulating archetype column storage)
        let registry = ComponentRegistry::from_inventory();
        let reg = registry.get::<TestPosition>().unwrap();
        
        // Create a "column" buffer with space for 3 components
        let mut column: Vec<u8> = vec![0u8; reg.size() * 3];
        
        // Serialize some test data
        let positions = [
            TestPosition { x: 1.0, y: 2.0 },
            TestPosition { x: 3.0, y: 4.0 },
            TestPosition { x: 5.0, y: 6.0 },
        ];
        
        // Serialize each to a buffer
        let mut serialized: Vec<Vec<u8>> = Vec::new();
        for pos in &positions {
            let mut buf = Vec::new();
            unsafe {
                reg.serialize_ptr(pos as *const _ as *const u8, &mut buf).unwrap();
            }
            serialized.push(buf);
        }
        
        // Deserialize directly into the column buffer
        for (i, data) in serialized.iter().enumerate() {
            let offset = i * reg.size();
            let dst = unsafe { column.as_mut_ptr().add(offset) };
            let mut cursor = std::io::Cursor::new(data);
            unsafe {
                reg.deserialize_ptr(&mut cursor, dst).unwrap();
            }
        }
        
        // Read back and verify
        for (i, original) in positions.iter().enumerate() {
            let offset = i * reg.size();
            let ptr = unsafe { column.as_ptr().add(offset) as *const TestPosition };
            let restored = unsafe { &*ptr };
            assert_eq!(restored.x, original.x);
            assert_eq!(restored.y, original.y);
        }
    }
}

