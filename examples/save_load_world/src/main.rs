//! Example: Save and Load a Game World
//!
//! Demonstrates serializing and deserializing a world with typical game components.
//! Compares different serialization formats.
//!
//! Run with: `cargo run -p save_load_world`

use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::time::Instant;

use nimbus_ecs::{component, World, TextFormat};
use nimbus_ecs::serializers::nimbus::{NimbusSerializer, NimbusDeserializer};
use nimbus_ecs::serializers::json::{JsonSerializer, JsonDeserializer};

// ============================================================================
// Game Components
// ============================================================================

/// 2D position in world space (required - no sensible default)
#[component(no_default)]
#[derive(Debug, Clone, PartialEq)]
pub struct Position {
    pub x: f32,
    pub y: f32,
}

/// 2D velocity for movement (optional - auto-derives Default to zero)
#[component]
#[derive(Debug, Clone, PartialEq)]
pub struct Velocity {
    pub dx: f32,
    pub dy: f32,
}

/// Entity health (optional - custom default for full health)
#[component(custom_default)]
#[derive(Debug, Clone, PartialEq)]
pub struct Health {
    pub current: i32,
    pub max: i32,
}

impl Default for Health {
    fn default() -> Self {
        Self { current: 100, max: 100 }  // Full health by default
    }
}

/// Player marker component (ZST - always optional, no data)
#[component]
#[derive(Debug, Clone, PartialEq)]
pub struct Player;

/// Enemy with a target entity reference (required - needs target)
#[component(no_default)]
#[derive(Debug, Clone, PartialEq)]
pub struct Enemy {
    pub aggro_range: f32,
    /// The entity this enemy is targeting
    pub target: nimbus_ecs::Entity,
}

/// Collectible item (optional - defaults to value 0)
#[component]
#[derive(Debug, Clone, PartialEq)]
pub struct Collectible {
    pub value: i32,
}

/// Name component for debugging (required - no sensible default)
#[component(no_default)]
#[derive(Debug, Clone, PartialEq)]
pub struct Name(pub String);

/// A follower component - references another entity to follow (required)
#[component(no_default)]
#[derive(Debug, Clone, PartialEq)]
pub struct Follows {
    pub target: nimbus_ecs::Entity,
}

// ============================================================================
// Main Example
// ============================================================================

fn main() {
    println!("=== Nimbus ECS Save/Load Example ===\n");

    // Test entity cross-references
    println!("=== Entity Cross-Reference Test ===\n");
    test_entity_references();

    // Test both JSON formats
    println!("\n=== Format Comparison ===\n");
    let world = create_game_world();
    println!("Created world with {} entities\n", count_entities(&world));
    
    // JSON format (compact)
    println!("--- JSON (compact) ---");
    test_format(&world, TextFormat::Json, "test_world.nimbus");
    
    println!();
    
    // JSON format (pretty, multi-line)
    println!("--- JSON (pretty, multi-line) ---");
    test_format(&world, TextFormat::JsonPretty, "test_world_pretty.nimbus");

    println!("\n=== Performance Benchmark (100 iterations) ===\n");
    
    // Benchmark with larger world
    let large_world = create_large_world(1000);
    println!("Created large world with {} entities\n", count_entities(&large_world));
    
    println!("--- Nimbus Format (line-based) ---");
    benchmark_format(&large_world, TextFormat::Json, "Nimbus+JSON");
    
    println!("\n--- Pure JSON Format ---");
    benchmark_pure_json(&large_world);

    println!("\n=== Example Complete ===");
}

/// Tests that entity cross-references are correctly remapped after deserialization.
fn test_entity_references() {
    let mut world = World::new();
    
    // Create a player that enemies will target
    let player = world.spawn_with((
        Position { x: 0.0, y: 0.0 },
        Health { current: 100, max: 100 },
        Player,
        Name("Hero".to_string()),
    ));
    println!("Created player: {:?}", player);
    
    // Create enemies that target the player
    let enemy1 = world.spawn_with((
        Position { x: 10.0, y: 0.0 },
        Health { current: 50, max: 50 },
        Enemy { aggro_range: 15.0, target: player },
        Name("Goblin".to_string()),
    ));
    println!("Created enemy1: {:?} (targets player)", enemy1);
    
    let enemy2 = world.spawn_with((
        Position { x: 20.0, y: 0.0 },
        Health { current: 50, max: 50 },
        Enemy { aggro_range: 15.0, target: player },
        Name("Orc".to_string()),
    ));
    println!("Created enemy2: {:?} (targets player)", enemy2);
    
    // Create a pet that follows the player
    let pet = world.spawn_with((
        Position { x: 1.0, y: 1.0 },
        Follows { target: player },
        Name("Dog".to_string()),
    ));
    println!("Created pet: {:?} (follows player)", pet);
    
    // Serialize
    let path = "test_refs.nimbus";
    save_world(&world, path, TextFormat::JsonPretty).expect("Failed to save");
    
    println!("\n--- Serialized World ---");
    let contents = std::fs::read_to_string(path).expect("Failed to read");
    println!("{}", contents);
    
    // Deserialize into a new world
    let mut loaded_world = World::new();
    let entity_map = load_world(&mut loaded_world, path, TextFormat::JsonPretty)
        .expect("Failed to load");
    
    println!("--- Entity Mapping ---");
    println!("File index -> New Entity:");
    for (file_idx, new_entity) in entity_map.iter() {
        println!("  {} -> {:?}", file_idx, new_entity);
    }
    
    // Get the new player entity
    let new_player = entity_map.get(0).expect("Player should be entity 0");
    println!("\nNew player entity: {:?}", new_player);
    
    // Verify: Check that enemies' target references can be remapped
    // Note: Currently, the component data isn't automatically remapped.
    // The user must manually remap entity references using entity_map.remap()
    
    println!("\n--- Remapping Entity References ---");
    println!("Original player was: {:?}", player);
    println!("After remap, player is: {:?}", entity_map.remap(player));
    
    // The serialized Enemy.target contains the OLD player entity.
    // After deserializing, we need to remap it to the NEW player entity.
    // This demonstrates how to use entity_map.remap():
    let old_target = player; // This is what was serialized
    let new_target = entity_map.remap(old_target);
    println!("\nEntity remap test:");
    println!("  Old target (serialized): {:?}", old_target);
    println!("  New target (remapped):   {:?}", new_target);
    println!("  Matches new player?      {}", new_target == Some(new_player));
    
    assert_eq!(new_target, Some(new_player), "Entity reference should remap correctly!");
    println!("\n✓ Entity cross-references remap correctly!");
    
    // Cleanup
    std::fs::remove_file(path).ok();
}

fn test_format(world: &World, format: TextFormat, path: &str) {
    // Save
    save_world(world, path, format).expect("Failed to save");
    
    // Read file size and first few lines
    let contents = std::fs::read_to_string(path).expect("Failed to read");
    let lines: Vec<&str> = contents.lines().take(15).collect();
    
    println!("File size: {} bytes", contents.len());
    println!("Preview:");
    for line in lines {
        println!("  {}", line);
    }
    if contents.lines().count() > 15 {
        println!("  ... ({} more lines)", contents.lines().count() - 15);
    }
    
    // Load back
    let mut loaded_world = World::new();
    let entity_map = load_world(&mut loaded_world, path, format).expect("Failed to load");
    println!("Loaded: {} entities", entity_map.iter().count());
    
    // Cleanup
    std::fs::remove_file(path).ok();
}

fn benchmark_format(world: &World, format: TextFormat, name: &str) {
    const ITERATIONS: u32 = 100;
    
    // Serialize benchmark
    let mut total_serialize = std::time::Duration::ZERO;
    let mut serialized_size = 0;
    
    for _ in 0..ITERATIONS {
        let mut buffer = Vec::new();
        let start = Instant::now();
        let serializer = NimbusSerializer::with_format(world, format);
        serializer.serialize_to(&mut buffer).expect("Failed to serialize");
        total_serialize += start.elapsed();
        serialized_size = buffer.len();
    }
    
    // Deserialize benchmark
    let mut buffer = Vec::new();
    let serializer = NimbusSerializer::with_format(world, format);
    serializer.serialize_to(&mut buffer).expect("Failed to serialize");
    let serialized = String::from_utf8(buffer).expect("Invalid UTF-8");
    
    let mut total_deserialize = std::time::Duration::ZERO;
    for _ in 0..ITERATIONS {
        let mut loaded_world = World::new();
        let start = Instant::now();
        let deserializer = NimbusDeserializer::with_format(format);
        deserializer.deserialize_into(&serialized, &mut loaded_world).expect("Failed to deserialize");
        total_deserialize += start.elapsed();
    }
    
    let avg_serialize = total_serialize / ITERATIONS;
    let avg_deserialize = total_deserialize / ITERATIONS;
    
    println!("{} Format:", name);
    println!("  Serialized size: {} bytes", serialized_size);
    println!("  Avg serialize:   {:?}", avg_serialize);
    println!("  Avg deserialize: {:?}", avg_deserialize);
    println!("  Throughput:      {:.2} MB/s (serialize)", 
        (serialized_size as f64 / avg_serialize.as_secs_f64()) / 1_000_000.0);
}

fn benchmark_pure_json(world: &World) {
    const ITERATIONS: u32 = 100;
    
    // Streaming JSON (no intermediate Value tree)
    let mut total_streaming = std::time::Duration::ZERO;
    let mut streaming_size = 0;
    
    for _ in 0..ITERATIONS {
        let mut buffer = Vec::new();
        let start = Instant::now();
        let serializer = JsonSerializer::new(world);
        serializer.serialize_streaming(&mut buffer).expect("Failed to serialize");
        total_streaming += start.elapsed();
        streaming_size = buffer.len();
    }
    
    let avg_streaming = total_streaming / ITERATIONS;
    
    println!("Pure JSON (streaming):");
    println!("  Serialized size: {} bytes", streaming_size);
    println!("  Avg serialize:   {:?}", avg_streaming);
    println!("  Throughput:      {:.2} MB/s (serialize)", 
        (streaming_size as f64 / avg_streaming.as_secs_f64()) / 1_000_000.0);
    
    // With Value tree (for comparison)
    let mut total_value = std::time::Duration::ZERO;
    let mut value_size = 0;
    
    for _ in 0..ITERATIONS {
        let mut buffer = Vec::new();
        let start = Instant::now();
        let serializer = JsonSerializer::new(world);
        serializer.serialize_to(&mut buffer).expect("Failed to serialize");
        total_value += start.elapsed();
        value_size = buffer.len();
    }
    
    let avg_value = total_value / ITERATIONS;
    
    println!("\nPure JSON (with Value tree):");
    println!("  Serialized size: {} bytes", value_size);
    println!("  Avg serialize:   {:?}", avg_value);
    println!("  Throughput:      {:.2} MB/s (serialize)", 
        (value_size as f64 / avg_value.as_secs_f64()) / 1_000_000.0);
}

/// Creates a sample game world with various entities
fn create_game_world() -> World {
    let mut world = World::new();

    // Player entity - spawn with all components at once (single archetype placement)
    let player = world.spawn_with((
        Position { x: 0.0, y: 0.0 },
        Velocity { dx: 0.0, dy: 0.0 },
        Health { current: 100, max: 100 },
        Player,
        Name("Hero".to_string()),
    ));

    // Some enemies that target the player
    for i in 0..3 {
        world.spawn_with((
            Position { 
                x: 10.0 + i as f32 * 5.0, 
                y: 10.0 + i as f32 * 3.0,
            },
            Velocity { dx: -1.0, dy: 0.0 },
            Health { 
                current: 30 + i * 10, 
                max: 50,
            },
            Enemy { aggro_range: 15.0, target: player },
            Name(format!("Goblin_{}", i)),
        ));
    }

    // Some collectibles (different archetype - no velocity/health)
    for i in 0..5 {
        world.spawn_with((
            Position { 
                x: i as f32 * 7.0, 
                y: 20.0,
            },
            Collectible { value: 10 * (i + 1) as i32 },
        ));
    }

    // Static objects (just position + name)
    for i in 0..3 {
        world.spawn_with((
            Position { 
                x: -10.0 - i as f32 * 5.0, 
                y: 0.0,
            },
            Name(format!("Rock_{}", i)),
        ));
    }

    world
}

/// Creates a larger world for benchmarking
fn create_large_world(entity_count: usize) -> World {
    let mut world = World::new();

    for i in 0..entity_count {
        world.spawn_with((
            Position { x: i as f32, y: (i * 2) as f32 },
            Velocity { dx: 1.0, dy: -1.0 },
            Health { current: 100, max: 100 },
            Name(format!("Entity_{}", i)),
        ));
    }

    world
}

/// Counts entities in the world
fn count_entities(world: &World) -> usize {
    world.entities().count()
}

/// Saves the world to a file
fn save_world(world: &World, path: &str, format: TextFormat) -> Result<(), Box<dyn std::error::Error>> {
    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);
    
    let serializer = NimbusSerializer::with_format(world, format);
    serializer.serialize_to(&mut writer)?;
    
    Ok(())
}

/// Loads a world from a file
fn load_world(world: &mut World, path: &str, format: TextFormat) -> Result<nimbus_ecs::serializers::nimbus::EntityMap, Box<dyn std::error::Error>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    
    let deserializer = NimbusDeserializer::with_format(format);
    let entity_map = deserializer.deserialize_from(reader, world)?;
    
    Ok(entity_map)
}
