//! The Nimbus human-readable serialization format.
//!
//! Designed for git-diffable scene files with clean entity boundaries.
//!
//! # Format Example
//!
//! ```text
//! # Player entity
//! @entity 0
//!   Position { x: 100.0, y: 50.0 }
//!   Velocity { dx: 0.0, dy: -9.8 }
//!   Health { current: 100, max: 100 }
//!   Name("Player")
//!
//! # Enemy that targets player
//! @entity 1
//!   Position { x: 200.0, y: 100.0 }
//!   Enemy { target: @0 }
//!
//! # Camera following player
//! @entity 2
//!   Position { x: 0.0, y: 0.0 }
//!   Camera { follow: @0 }
//! ```
//!
//! # Features
//!
//! - `@entity N` - Entity declaration with file-local index
//! - `@N` - Entity reference (remapped on load)
//! - `#` - Line comments
//! - Indented components grouped under entity
//! - Deterministic output (sorted entities and components)

use hashbrown::HashMap;
use std::fmt;
use std::io::{self, BufRead, Write};

use crate::component::ComponentId;
use crate::entity::Entity;
use crate::serialization::{ComponentRegistry, TextFormat};
use crate::World;

// ============================================================================
// Error Types
// ============================================================================

/// Errors that can occur during nimbus format serialization/deserialization.
#[derive(Debug)]
pub enum NimbusError {
    /// IO error during read/write
    Io(io::Error),
    /// Format parsing error
    Parse { line: usize, message: String },
    /// Unknown component type
    UnknownComponent { line: usize, name: String },
    /// Invalid entity reference
    InvalidReference { line: usize, reference: String },
    /// Serialization error for a component
    Serialize { entity: u32, component: String, message: String },
    /// Deserialization error for a component
    Deserialize { line: usize, component: String, message: String },
}

impl fmt::Display for NimbusError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NimbusError::Io(e) => write!(f, "IO error: {}", e),
            NimbusError::Parse { line, message } => {
                write!(f, "Parse error at line {}: {}", line, message)
            }
            NimbusError::UnknownComponent { line, name } => {
                write!(f, "Unknown component '{}' at line {}", name, line)
            }
            NimbusError::InvalidReference { line, reference } => {
                write!(f, "Invalid entity reference '{}' at line {}", reference, line)
            }
            NimbusError::Serialize { entity, component, message } => {
                write!(f, "Failed to serialize {} on entity {}: {}", component, entity, message)
            }
            NimbusError::Deserialize { line, component, message } => {
                write!(f, "Failed to deserialize {} at line {}: {}", component, line, message)
            }
        }
    }
}

impl std::error::Error for NimbusError {}

impl From<io::Error> for NimbusError {
    fn from(e: io::Error) -> Self {
        NimbusError::Io(e)
    }
}

// ============================================================================
// Serialized Representation
// ============================================================================

/// A serialized entity with file-local index and components.
#[derive(Debug, Clone)]
pub struct SerializedEntity {
    /// File-local index (0, 1, 2, ...)
    pub index: u32,
    /// Optional comment/name for this entity
    pub comment: Option<String>,
    /// Components as (type_name, serialized_data) pairs
    pub components: Vec<(String, String)>,
}

/// A group of entities with the same component types (archetype).
#[derive(Debug, Clone)]
pub struct SerializedArchetype {
    /// Component type names in this archetype
    pub component_types: Vec<String>,
    /// Entities in this archetype
    pub entities: Vec<SerializedEntity>,
}

/// A complete serialized world.
#[derive(Debug, Clone, Default)]
pub struct SerializedWorld {
    /// Header comment for the file
    pub header: Option<String>,
    /// Archetype-grouped entities (preferred for bulk loading)
    pub archetypes: Vec<SerializedArchetype>,
    /// Standalone entities not in an archetype block (legacy format)
    pub entities: Vec<SerializedEntity>,
}

// ============================================================================
// Serializer
// ============================================================================

/// Serializes a World to the nimbus format.
pub struct NimbusSerializer<'w> {
    world: &'w World,
    registry: &'static ComponentRegistry,
    format: TextFormat,
}

impl<'w> NimbusSerializer<'w> {
    /// Creates a new serializer for the given world using compact JSON.
    pub fn new(world: &'w World) -> Self {
        Self::with_format(world, TextFormat::Json)
    }
    
    /// Creates a new serializer with a specific text format.
    pub fn with_format(world: &'w World, format: TextFormat) -> Self {
        Self {
            world,
            format,
            registry: ComponentRegistry::global(),
        }
    }

    /// Serializes the world to a string.
    pub fn serialize(&self) -> Result<String, NimbusError> {
        // Estimate capacity: ~100 bytes per entity is a reasonable guess
        let entity_count = self.world.entities().count();
        let estimated_size = 64 + entity_count * 100;
        
        let mut buffer = Vec::with_capacity(estimated_size);
        self.serialize_to(&mut buffer)?;
        
        // SAFETY: We only write valid UTF-8 (ASCII text + JSON)
        Ok(unsafe { String::from_utf8_unchecked(buffer) })
    }

    /// Serializes the world directly to a writer (most efficient).
    /// 
    /// Entities are grouped by archetype for efficient bulk deserialization.
    pub fn serialize_to<W: Write>(&self, writer: &mut W) -> Result<(), NimbusError> {
        // Use buffered writer for efficiency
        let mut writer = io::BufWriter::with_capacity(8192, writer);
        
        // Header
        writer.write_all(b"# Nimbus Scene Format v1\n")?;
        writer.write_all(b"# Generated by nimbus_ecs\n\n")?;
        
        let archetypes = self.world.archetypes();
        
        // Build entity → file index mapping (assign indices in archetype order)
        let mut entity_to_index: HashMap<Entity, u32> = HashMap::new();
        let mut file_index = 0u32;
        
        for arch_idx in 0..archetypes.len() {
            if let Some(archetype) = archetypes.get(arch_idx) {
                for &entity in archetype.entities() {
                    entity_to_index.insert(entity, file_index);
                    file_index += 1;
                }
            }
        }
        
        // Reusable buffers
        let mut component_buffer = Vec::with_capacity(256);
        let mut itoa_buffer = itoa::Buffer::new();
        
        // Serialize by archetype for bulk deserialization
        for arch_idx in 0..archetypes.len() {
            let Some(archetype) = archetypes.get(arch_idx) else { continue };
            if archetype.entities().is_empty() { continue };
            
            // Get sorted component info for this archetype
            let component_info = self.get_sorted_components(archetype);
            if component_info.is_empty() { continue }; // Skip archetypes with no serializable components
            
            // Write archetype header: @archetype [Type1, Type2, ...]
            writer.write_all(b"@archetype [")?;
            for (i, (name, _)) in component_info.iter().enumerate() {
                if i > 0 { writer.write_all(b", ")?; }
                writer.write_all(name.as_bytes())?;
            }
            writer.write_all(b"]\n")?;
            
            // Write all entities in this archetype
            for (row, &entity) in archetype.entities().iter().enumerate() {
                let file_idx = entity_to_index[&entity];
                
                self.serialize_entity_in_archetype(
                    &mut writer,
                    file_idx,
                    row,
                    archetype,
                    &component_info,
                    &mut component_buffer,
                    &mut itoa_buffer,
                )?;
            }
            
            writer.write_all(b"\n")?;
        }
        
        writer.flush()?;
        Ok(())
    }

    /// Gets sorted component info for an archetype.
    fn get_sorted_components(&self, archetype: &crate::archetype::Archetype) -> Vec<(&'static str, ComponentId)> {
        let mut components: Vec<(&str, ComponentId)> = archetype
            .columns()
            .keys()
            .filter_map(|&id| {
                self.registry.get_by_id(id).map(|reg| (reg.type_name, id))
            })
            .collect();
        
        // Sort by type name for deterministic output
        components.sort_by_key(|(name, _)| *name);
        components
    }

    /// Serializes a single entity within an archetype block.
    fn serialize_entity_in_archetype<W: Write>(
        &self,
        writer: &mut W,
        file_index: u32,
        row: usize,
        archetype: &crate::archetype::Archetype,
        component_info: &[(&str, ComponentId)],
        _component_buffer: &mut Vec<u8>,
        itoa_buffer: &mut itoa::Buffer,
    ) -> Result<(), NimbusError> {
        // Write: "  @entity N\n"
        writer.write_all(b"  @entity ")?;
        writer.write_all(itoa_buffer.format(file_index).as_bytes())?;
        writer.write_all(b"\n")?;
        
        // Serialize each component in sorted order
        for (type_name, component_id) in component_info {
            let registration = self.registry.get_by_id(*component_id).unwrap();
            let column = archetype.columns().get(component_id).unwrap();
            let ptr = column.get_ptr(row);
            
            // Write: "    TypeName: "
            writer.write_all(b"    ")?;
            writer.write_all(type_name.as_bytes())?;
            writer.write_all(b": ")?;
            
            // For compact JSON, stream directly; for pretty, use string (has newlines)
            match self.format {
                TextFormat::Json => {
                    unsafe {
                        registration.serialize_json_to(ptr, writer)
                            .map_err(|e| NimbusError::Serialize {
                                entity: file_index,
                                component: type_name.to_string(),
                                message: e.to_string(),
                            })?;
                    }
                }
                TextFormat::JsonPretty => {
                    let text_string = unsafe {
                        registration.serialize_text(ptr, self.format)
                            .map_err(|e| NimbusError::Serialize {
                                entity: file_index,
                                component: type_name.to_string(),
                                message: e.to_string(),
                            })?
                    };
                    writer.write_all(text_string.as_bytes())?;
                }
            }
            writer.write_all(b"\n")?;
        }
        
        Ok(())
    }
}


// ============================================================================
// Helper Functions
// ============================================================================

/// Counts net brace depth change in a string.
/// Returns positive for more opens, negative for more closes.
/// Handles both `{}` (JSON) and `()` (RON).
fn count_braces(s: &str) -> i32 {
    let mut depth = 0i32;
    let mut in_string = false;
    let mut escape_next = false;
    
    for c in s.chars() {
        if escape_next {
            escape_next = false;
            continue;
        }
        
        match c {
            '\\' if in_string => escape_next = true,
            '"' => in_string = !in_string,
            '{' | '(' if !in_string => depth += 1,
            '}' | ')' if !in_string => depth -= 1,
            _ => {}
        }
    }
    
    depth
}


// ============================================================================
// Deserializer
// ============================================================================

/// Deserializes a World from the nimbus format.
pub struct NimbusDeserializer {
    registry: &'static ComponentRegistry,
    format: TextFormat,
}

impl NimbusDeserializer {
    /// Creates a new deserializer using JSON format.
    pub fn new() -> Self {
        Self::with_format(TextFormat::Json)
    }
    
    /// Creates a new deserializer with a specific text format.
    pub fn with_format(format: TextFormat) -> Self {
        Self {
            registry: ComponentRegistry::global(),
            format,
        }
    }

    /// Deserializes from a string into an existing world.
    pub fn deserialize_into(&self, input: &str, world: &mut World) -> Result<EntityMap, NimbusError> {
        let parsed = self.parse(input)?;
        self.load_into(parsed, world)
    }

    /// Deserializes from a reader into an existing world.
    pub fn deserialize_from<R: BufRead>(&self, reader: R, world: &mut World) -> Result<EntityMap, NimbusError> {
        let mut input = String::new();
        for line in reader.lines() {
            input.push_str(&line?);
            input.push('\n');
        }
        self.deserialize_into(&input, world)
    }

    /// Parses the nimbus format into an intermediate representation.
    /// 
    /// Handles both archetype-grouped format (v1) and flat entity format.
    /// Supports multi-line JSON values by tracking brace depth.
    fn parse(&self, input: &str) -> Result<SerializedWorld, NimbusError> {
        let mut world = SerializedWorld::default();
        let mut current_archetype: Option<SerializedArchetype> = None;
        let mut current_entity: Option<SerializedEntity> = None;
        
        let lines: Vec<&str> = input.lines().collect();
        let mut i = 0;
        
        while i < lines.len() {
            let line_num = i + 1; // 1-indexed for error messages
            let line = lines[i];
            let trimmed = line.trim();
            
            // Skip empty lines and comments
            if trimmed.is_empty() || trimmed.starts_with('#') {
                i += 1;
                continue;
            }
            
            // Archetype declaration: @archetype [Type1, Type2, ...]
            if trimmed.starts_with("@archetype") {
                // Save previous archetype
                if let Some(mut arch) = current_archetype.take() {
                    if let Some(entity) = current_entity.take() {
                        arch.entities.push(entity);
                    }
                    world.archetypes.push(arch);
                }
                
                // Parse component types
                let bracket_start = trimmed.find('[').ok_or_else(|| NimbusError::Parse {
                    line: line_num,
                    message: "Expected '[' after @archetype".to_string(),
                })?;
                let bracket_end = trimmed.rfind(']').ok_or_else(|| NimbusError::Parse {
                    line: line_num,
                    message: "Expected ']' in @archetype".to_string(),
                })?;
                
                let types_str = &trimmed[bracket_start + 1..bracket_end];
                let component_types: Vec<String> = types_str
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                
                current_archetype = Some(SerializedArchetype {
                    component_types,
                    entities: Vec::new(),
                });
                current_entity = None;
                i += 1;
            }
            // Entity declaration: @entity N (can be at root or inside archetype)
            else if trimmed.starts_with("@entity") {
                // Save previous entity
                if let Some(entity) = current_entity.take() {
                    if let Some(ref mut arch) = current_archetype {
                        arch.entities.push(entity);
                    } else {
                        world.entities.push(entity);
                    }
                }
                
                // Parse entity index
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                if parts.len() < 2 {
                    return Err(NimbusError::Parse {
                        line: line_num,
                        message: "Expected entity index after @entity".to_string(),
                    });
                }
                
                let index: u32 = parts[1].parse().map_err(|_| NimbusError::Parse {
                    line: line_num,
                    message: format!("Invalid entity index: {}", parts[1]),
                })?;
                
                current_entity = Some(SerializedEntity {
                    index,
                    comment: None,
                    components: Vec::new(),
                });
                i += 1;
            }
            // Component line (indented)
            else if line.starts_with(' ') || line.starts_with('\t') {
                let Some(ref mut entity) = current_entity else {
                    return Err(NimbusError::Parse {
                        line: line_num,
                        message: "Component outside of entity block".to_string(),
                    });
                };
                
                // Parse: TypeName: value (possibly multi-line)
                let (type_name, value, lines_consumed) = self.parse_component_value(&lines, i)?;
                entity.components.push((type_name.to_string(), value));
                i += lines_consumed;
            }
            else {
                return Err(NimbusError::Parse {
                    line: line_num,
                    message: format!("Unexpected content: {}", trimmed),
                });
            }
        }
        
        // Don't forget the last entity/archetype
        if let Some(entity) = current_entity.take() {
            if let Some(ref mut arch) = current_archetype {
                arch.entities.push(entity);
            } else {
                world.entities.push(entity);
            }
        }
        if let Some(arch) = current_archetype {
            world.archetypes.push(arch);
        }
        
        Ok(world)
    }

    /// Parses a component value, handling multi-line JSON/RON.
    /// Returns (type_name, value, lines_consumed).
    fn parse_component_value(&self, lines: &[&str], start: usize) -> Result<(String, String, usize), NimbusError> {
        let line_num = start + 1;
        let first_line = lines[start].trim();
        
        // Find the colon separator
        let colon_pos = first_line.find(':')
            .ok_or_else(|| NimbusError::Parse {
                line: line_num,
                message: "Expected ':' after component type name".to_string(),
            })?;
        
        let type_name = first_line[..colon_pos].trim().to_string();
        let first_value_part = first_line[colon_pos + 1..].trim();
        
        // Check if the value is complete on one line
        let brace_depth = count_braces(first_value_part);
        
        if brace_depth == 0 {
            // Single line value (or no braces like strings/numbers)
            return Ok((type_name, first_value_part.to_string(), 1));
        }
        
        // Multi-line value - accumulate lines until braces balance
        let mut value = first_value_part.to_string();
        let mut depth = brace_depth;
        let mut lines_consumed = 1;
        
        for line in lines.iter().skip(start + 1) {
            lines_consumed += 1;
            value.push('\n');
            value.push_str(line.trim());
            
            depth += count_braces(line);
            
            if depth == 0 {
                break;
            }
        }
        
        if depth != 0 {
            return Err(NimbusError::Parse {
                line: line_num,
                message: "Unclosed braces in component value".to_string(),
            });
        }
        
        Ok((type_name, value, lines_consumed))
    }

    /// Loads parsed data into a world, returning the entity mapping.
    fn load_into(&self, parsed: SerializedWorld, world: &mut World) -> Result<EntityMap, NimbusError> {
        let mut entity_map = EntityMap::new();
        
        // Load archetype-grouped entities (bulk spawn for efficiency)
        for archetype in &parsed.archetypes {
            // TODO: Implement bulk spawn with components
            // For now, spawn individually
            for serialized in &archetype.entities {
                let entity = world.spawn();
                entity_map.insert(serialized.index, entity);
            }
        }
        
        // Load standalone entities
        for serialized in &parsed.entities {
            let entity = world.spawn();
            entity_map.insert(serialized.index, entity);
        }
        
        // Second pass: add components
        for archetype in &parsed.archetypes {
            for serialized in &archetype.entities {
                let entity = entity_map.get(serialized.index)
                    .expect("Entity should exist from first pass");
                
                for (type_name, value) in &serialized.components {
                    self.load_component(world, entity, type_name, value, &entity_map)?;
                }
            }
        }
        
        for serialized in &parsed.entities {
            let entity = entity_map.get(serialized.index)
                .expect("Entity should exist from first pass");
            
            for (type_name, value) in &serialized.components {
                self.load_component(world, entity, type_name, value, &entity_map)?;
            }
        }
        
        Ok(entity_map)
    }

    /// Loads a single component onto an entity.
    fn load_component(
        &self,
        _world: &mut World,
        _entity: Entity,
        type_name: &str,
        value: &str,
        _entity_map: &EntityMap,
    ) -> Result<(), NimbusError> {
        let registration = self.registry.get_by_name(type_name)
            .ok_or_else(|| NimbusError::UnknownComponent {
                line: 0, // TODO: track line number
                name: type_name.to_string(),
            })?;
        
        // Allocate space for the component
        let size = registration.size();
        let mut buffer = vec![0u8; size];
        
        // Deserialize from text format
        unsafe {
            registration.deserialize_text(value, buffer.as_mut_ptr(), self.format)
                .map_err(|e| NimbusError::Deserialize {
                    line: 0,
                    component: type_name.to_string(),
                    message: e.to_string(),
                })?;
        }
        
        // TODO: Insert component into world
        // Need to extend World API to insert raw component bytes by ComponentId
        // For now, the buffer contains the deserialized component
        let _ = buffer;
        
        Ok(())
    }
}

impl Default for NimbusDeserializer {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Entity Mapping
// ============================================================================

/// Maps file-local entity indices to actual Entity values.
#[derive(Debug, Clone, Default)]
pub struct EntityMap {
    map: HashMap<u32, Entity>,
}

impl EntityMap {
    /// Creates a new empty entity map.
    pub fn new() -> Self {
        Self { map: HashMap::new() }
    }

    /// Inserts a mapping from file index to entity.
    pub fn insert(&mut self, file_index: u32, entity: Entity) {
        self.map.insert(file_index, entity);
    }

    /// Gets the entity for a file index.
    pub fn get(&self, file_index: u32) -> Option<Entity> {
        self.map.get(&file_index).copied()
    }

    /// Remaps an entity reference from its serialized form to the new runtime entity.
    /// 
    /// During serialization, entities are assigned sequential file indices (0, 1, 2, ...).
    /// The serialized Entity's `index` field contains this file index.
    /// This method looks up that index and returns the new runtime Entity.
    /// 
    /// Returns None if the file index is not found (entity wasn't in the serialized world).
    pub fn remap(&self, serialized_entity: Entity) -> Option<Entity> {
        self.map.get(&serialized_entity.index).copied()
    }

    /// Returns an iterator over all mappings.
    pub fn iter(&self) -> impl Iterator<Item = (u32, Entity)> + '_ {
        self.map.iter().map(|(&k, &v)| (k, v))
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_empty() {
        let deserializer = NimbusDeserializer::new();
        let result = deserializer.parse("").unwrap();
        assert!(result.entities.is_empty());
    }

    #[test]
    fn test_parse_comments_only() {
        let deserializer = NimbusDeserializer::new();
        let input = r#"
# This is a comment
# Another comment
        "#;
        let result = deserializer.parse(input).unwrap();
        assert!(result.entities.is_empty());
    }

    #[test]
    fn test_parse_single_entity() {
        let deserializer = NimbusDeserializer::new();
        let input = r#"
@entity 0
  Position: {"x": 1.0, "y": 2.0}
  Velocity: {"dx": 0.0, "dy": 0.0}
        "#;
        let result = deserializer.parse(input).unwrap();
        assert_eq!(result.entities.len(), 1);
        assert_eq!(result.entities[0].index, 0);
        assert_eq!(result.entities[0].components.len(), 2);
    }

    #[test]
    fn test_parse_multiple_entities() {
        let deserializer = NimbusDeserializer::new();
        let input = r#"
@entity 0
  Position: {"x": 0.0, "y": 0.0}

@entity 1
  Position: {"x": 10.0, "y": 20.0}
  Enemy: {"target": 0}
        "#;
        let result = deserializer.parse(input).unwrap();
        assert_eq!(result.entities.len(), 2);
        assert_eq!(result.entities[0].index, 0);
        assert_eq!(result.entities[1].index, 1);
        assert_eq!(result.entities[1].components.len(), 2);
    }

    #[test]
    fn test_entity_map() {
        let mut map = EntityMap::new();
        let e1 = Entity::from_raw(1, 0);
        let e2 = Entity::from_raw(5, 2);
        
        map.insert(0, e1);
        map.insert(1, e2);
        
        assert_eq!(map.get(0), Some(e1));
        assert_eq!(map.get(1), Some(e2));
        assert_eq!(map.get(2), None);
    }
}

