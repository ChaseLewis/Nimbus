# Nimbus Engine

A high-performance game engine built in Rust, with a focus on an efficient Entity Component System (ECS) architecture.

## Project Goals

- **Performance First** — Designed for cache-efficient iteration and minimal overhead
- **Ergonomic API** — Bevy-inspired system functions with automatic dependency injection
- **Parallel by Default** — Built-in thread pool with scoped parallelism and async task support

## Workspace Structure

```
nimbus_engine/
├── libs/
│   ├── nimbus_ecs/        # Entity Component System
│   ├── nimbus_math/       # SIMD-accelerated math (vec, mat, quat)
│   ├── nimbus_macro_ecs/  # Derive macros (#[derive(Component)])
│   └── nimbus_engine/     # Engine facade
└── examples/
    └── basic/             # Getting started example
```

## nimbus_ecs

The ECS is the heart of the engine.

### Components
```rust
use nimbus_ecs::{Query, Commands, Component};

#[derive(Component)]
struct Position { x: f32, y: f32 }

#[derive(Component)]
struct Velocity { x: f32, y: f32 }

fn movement_system(mut query: Query<(&mut Position, &Velocity)>) {
    for (pos, vel) in query.iter() {
        pos.x += vel.x;
        pos.y += vel.y;
    }
}
```

### Events
Type-safe event channels for decoupled communication:

```rust
use nimbus_ecs::{EventWriter, EventReader};

#[derive(Clone)]
struct DamageEvent { entity: Entity, amount: i32 }

fn deal_damage(events: EventWriter<DamageEvent>) {
    events.send(DamageEvent { entity, amount: 10 });
}

fn apply_damage(events: EventReader<DamageEvent>, mut health: Query<&mut Health>) {
    for event in events.iter() {
        // Handle damage...
    }
}
```

## Building

```bash
# Build all crates
cargo build

# Run tests
cargo test

# Run the basic example
cargo run --example basic
```

## Current Roadmap

- [x] Archetype-based ECS
- [x] System parameter injection
- [x] Priority scheduler
- [x] Event system
- [x] Thread pool foundation
- [ ] Parallel query iteration
- [ ] System-level parallelism
- [ ] Async task integration
- [ ] Render pipeline
- [ ] Asset system

## License

MIT

