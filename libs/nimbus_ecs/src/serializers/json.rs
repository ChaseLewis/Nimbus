//! Pure JSON world serialization.
//!
//! Uses serde_json directly for maximum performance.
//! The entire world is serialized as a single JSON document.
//!
//! # Format
//!
//! ```json
//! {
//!   "archetypes": [
//!     {
//!       "components": ["Position", "Health"],
//!       "entities": [
//!         {"Position": {"x": 0.0, "y": 0.0}, "Health": {"current": 100, "max": 100}}
//!       ]
//!     }
//!   ]
//! }
//! ```

use std::io::{Read, Write};
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use serde_json::Value;

use crate::entity::Entity;
use crate::serialization::ComponentRegistry;
use crate::World;

/// Error type for JSON serialization.
#[derive(Debug)]
pub enum JsonError {
    Io(std::io::Error),
    Json(serde_json::Error),
    Serialization(crate::serialization::Error),
    UnknownComponent(String),
}

impl std::fmt::Display for JsonError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JsonError::Io(e) => write!(f, "IO error: {}", e),
            JsonError::Json(e) => write!(f, "JSON error: {}", e),
            JsonError::Serialization(e) => write!(f, "Serialization error: {}", e),
            JsonError::UnknownComponent(name) => write!(f, "Unknown component: {}", name),
        }
    }
}

impl std::error::Error for JsonError {}

impl From<std::io::Error> for JsonError {
    fn from(e: std::io::Error) -> Self { JsonError::Io(e) }
}

impl From<serde_json::Error> for JsonError {
    fn from(e: serde_json::Error) -> Self { JsonError::Json(e) }
}

impl From<crate::serialization::Error> for JsonError {
    fn from(e: crate::serialization::Error) -> Self { JsonError::Serialization(e) }
}

/// The JSON world format.
#[derive(Serialize, Deserialize)]
struct JsonWorld {
    archetypes: Vec<JsonArchetype>,
}

#[derive(Serialize, Deserialize)]
struct JsonArchetype {
    /// Component type names
    components: Vec<String>,
    /// Entities as maps of component name -> value
    entities: Vec<JsonEntity>,
}

#[derive(Serialize, Deserialize)]
struct JsonEntity {
    /// File-local entity index
    id: u32,
    /// Component data as raw JSON values
    #[serde(flatten)]
    components: HashMap<String, Value>,
}

/// Entity mapping from file indices to runtime entities.
pub struct EntityMap {
    map: HashMap<u32, Entity>,
}

impl EntityMap {
    pub fn new() -> Self {
        Self { map: HashMap::new() }
    }

    pub fn insert(&mut self, file_index: u32, entity: Entity) {
        self.map.insert(file_index, entity);
    }

    pub fn get(&self, file_index: u32) -> Option<Entity> {
        self.map.get(&file_index).copied()
    }

    /// Remaps a serialized entity reference to the new runtime entity.
    pub fn remap(&self, serialized: Entity) -> Option<Entity> {
        self.map.get(&serialized.index()).copied()
    }

    pub fn iter(&self) -> impl Iterator<Item = (u32, Entity)> + '_ {
        self.map.iter().map(|(&k, &v)| (k, v))
    }
}

impl Default for EntityMap {
    fn default() -> Self { Self::new() }
}

// ============================================================================
// Serializer
// ============================================================================

/// Pure JSON world serializer.
pub struct JsonSerializer<'w> {
    world: &'w World,
    registry: &'static ComponentRegistry,
    pretty: bool,
}

impl<'w> JsonSerializer<'w> {
    /// Creates a new JSON serializer (compact output).
    pub fn new(world: &'w World) -> Self {
        Self {
            world,
            registry: ComponentRegistry::global(),
            pretty: false,
        }
    }

    /// Creates a new JSON serializer with pretty output.
    pub fn pretty(world: &'w World) -> Self {
        Self {
            world,
            registry: ComponentRegistry::global(),
            pretty: true,
        }
    }

    /// Serializes the world to a writer.
    pub fn serialize_to<W: Write>(&self, writer: W) -> Result<(), JsonError> {
        let json_world = self.build_json_world()?;
        
        if self.pretty {
            serde_json::to_writer_pretty(writer, &json_world)?;
        } else {
            serde_json::to_writer(writer, &json_world)?;
        }
        
        Ok(())
    }

    /// Builds the JSON representation of the world.
    fn build_json_world(&self) -> Result<JsonWorld, JsonError> {
        let archetypes = self.world.archetypes();
        let mut json_archetypes = Vec::new();
        let mut file_index: u32 = 0;

        for arch_idx in 0..archetypes.len() {
            let Some(archetype) = archetypes.get(arch_idx) else { continue };
            if archetype.entities().is_empty() { continue }

            // Get component info for this archetype
            let mut component_names: Vec<String> = archetype
                .columns()
                .keys()
                .filter_map(|&id| {
                    self.registry.get_by_id(id).map(|reg| reg.type_name.to_string())
                })
                .collect();
            component_names.sort(); // Deterministic order

            if component_names.is_empty() { continue }

            // Serialize entities
            let mut json_entities = Vec::with_capacity(archetype.entities().len());
            
            for (row, &_entity) in archetype.entities().iter().enumerate() {
                let mut components = HashMap::new();
                
                for name in &component_names {
                    let reg = self.registry.get_by_name(name).unwrap();
                    let id = reg.component_id();
                    let column = archetype.columns().get(&id).unwrap();
                    let ptr = column.get_ptr(row);
                    
                    // Serialize component to JSON Value
                    let json_str = unsafe {
                        reg.serialize_text(ptr, crate::serialization::TextFormat::Json)?
                    };
                    let value: Value = serde_json::from_str(&json_str)?;
                    components.insert(name.clone(), value);
                }
                
                json_entities.push(JsonEntity {
                    id: file_index,
                    components,
                });
                file_index += 1;
            }

            json_archetypes.push(JsonArchetype {
                components: component_names,
                entities: json_entities,
            });
        }

        Ok(JsonWorld { archetypes: json_archetypes })
    }

    /// Serializes directly to writer without building intermediate structures.
    /// This is faster than build_json_world for large worlds.
    pub fn serialize_streaming<W: Write>(&self, mut writer: W) -> Result<(), JsonError> {
        let archetypes = self.world.archetypes();
        let mut file_index: u32 = 0;
        let mut first_archetype = true;

        writer.write_all(b"{\"archetypes\":[")?;

        for arch_idx in 0..archetypes.len() {
            let Some(archetype) = archetypes.get(arch_idx) else { continue };
            if archetype.entities().is_empty() { continue }

            // Get sorted component info
            let mut component_info: Vec<_> = archetype
                .columns()
                .keys()
                .filter_map(|&id| {
                    self.registry.get_by_id(id).map(|reg| (reg.type_name, id))
                })
                .collect();
            component_info.sort_by_key(|(name, _)| *name);

            if component_info.is_empty() { continue }

            if !first_archetype { writer.write_all(b",")?; }
            first_archetype = false;

            // Write archetype header
            writer.write_all(b"{\"components\":[")?;
            for (i, (name, _)) in component_info.iter().enumerate() {
                if i > 0 { writer.write_all(b",")?; }
                write!(writer, "\"{}\"", name)?;
            }
            writer.write_all(b"],\"entities\":[")?;

            // Write entities
            for (row, &_entity) in archetype.entities().iter().enumerate() {
                if row > 0 { writer.write_all(b",")?; }
                
                write!(writer, "{{\"id\":{}", file_index)?;
                
                for (name, id) in &component_info {
                    let reg = self.registry.get_by_id(*id).unwrap();
                    let column = archetype.columns().get(id).unwrap();
                    let ptr = column.get_ptr(row);
                    
                    // Write component directly to writer (no intermediate String!)
                    write!(writer, ",\"{}\":", name)?;
                    unsafe {
                        reg.serialize_json_to(ptr, &mut writer)?;
                    }
                }
                
                writer.write_all(b"}")?;
                file_index += 1;
            }

            writer.write_all(b"]}")?;
        }

        writer.write_all(b"]}")?;
        Ok(())
    }
}

// ============================================================================
// Deserializer
// ============================================================================

/// Pure JSON world deserializer.
pub struct JsonDeserializer {
    registry: &'static ComponentRegistry,
}

impl JsonDeserializer {
    pub fn new() -> Self {
        Self {
            registry: ComponentRegistry::global(),
        }
    }

    /// Deserializes from a reader into an existing world.
    pub fn deserialize_from<R: Read>(&self, reader: R, world: &mut World) -> Result<EntityMap, JsonError> {
        let json_world: JsonWorld = serde_json::from_reader(reader)?;
        self.load_into(json_world, world)
    }

    /// Deserializes from a string into an existing world.
    pub fn deserialize_str(&self, s: &str, world: &mut World) -> Result<EntityMap, JsonError> {
        let json_world: JsonWorld = serde_json::from_str(s)?;
        self.load_into(json_world, world)
    }

    fn load_into(&self, json_world: JsonWorld, world: &mut World) -> Result<EntityMap, JsonError> {
        let mut entity_map = EntityMap::new();

        // First pass: spawn entities
        for archetype in &json_world.archetypes {
            for entity_data in &archetype.entities {
                let entity = world.spawn();
                entity_map.insert(entity_data.id, entity);
            }
        }

        // Second pass: add components (TODO: implement raw component insertion)
        // For now, entities are spawned but components aren't loaded
        // This requires extending World to support insert-by-ComponentId

        Ok(entity_map)
    }
}

impl Default for JsonDeserializer {
    fn default() -> Self { Self::new() }
}

