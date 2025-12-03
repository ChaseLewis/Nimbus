//! ECS benchmarks comparing nimbus_ecs against bevy_ecs
//!
//! Benchmarks:
//! 1. Spawn 1000 entities with Position, Velocity bundle
//! 2. Start with 1000 entities, remove velocity from one
//! 3. Start with 1000 entities, add acceleration to one
//! 4. Iterate 1000 entities with system that adds velocity to position
//! 5. Spawn 1000, remove 500, then iterate remaining
//! 6. Start with 1000 Position entities, use Commands to add Velocity+Acceleration bundle to all

use criterion::{black_box, criterion_group, criterion_main, Criterion};

// ============================================================================
// Nimbus ECS types and helpers
// ============================================================================

#[allow(dead_code)]
mod nimbus {
    use nimbus_ecs::{Commands, Component, Entity, Query, World};

    #[derive(Component, Clone, Copy)]
    pub struct Position {
        pub x: f32,
        pub y: f32,
    }

    #[derive(Component, Clone, Copy)]
    pub struct Velocity {
        pub x: f32,
        pub y: f32,
    }

    #[derive(Component, Clone, Copy)]
    pub struct Acceleration {
        pub x: f32,
        pub y: f32,
    }

    pub fn spawn_1000_entities(world: &mut World) -> Vec<Entity> {
        (0..1000)
            .map(|i| {
                let f = i as f32;
                world.spawn_with((Position { x: f, y: f }, Velocity { x: 1.0, y: 1.0 }))
            })
            .collect()
    }

    pub fn spawn_1000_entities_batch(world: &mut World) -> Vec<Entity> {
        let bundles: Vec<_> = (0..1000)
            .map(|i| {
                let f = i as f32;
                (Position { x: f, y: f }, Velocity { x: 1.0, y: 1.0 })
            })
            .collect();
        world.spawn_batch(bundles)
    }

    pub fn spawn_1000_position_only(world: &mut World) -> Vec<Entity> {
        let bundles: Vec<_> = (0..1000)
            .map(|i| {
                let f = i as f32;
                Position { x: f, y: f }
            })
            .collect();
        world.spawn_batch(bundles)
    }

    pub fn movement_system(mut query: Query<(&mut Position, &Velocity)>) {
        for (pos, vel) in query.iter() {
            pos.x += vel.x;
            pos.y += vel.y;
        }
    }

    /// System that adds Velocity and Acceleration to all Position entities using insert_bundle
    pub fn add_components_system(mut query: Query<Entity>, commands: Commands) {
        for entity in query.iter() {
            commands.entity(entity)
                .insert_bundle((Velocity { x: 1.0, y: 1.0 }, Acceleration { x: 0.1, y: 0.1 }));
        }
    }
}

// ============================================================================
// Bevy ECS types and helpers
// ============================================================================

#[allow(dead_code)]
mod bevy {
    use bevy_ecs::prelude::*;

    #[derive(Component, Clone, Copy)]
    pub struct Position {
        pub x: f32,
        pub y: f32,
    }

    #[derive(Component, Clone, Copy)]
    pub struct Velocity {
        pub x: f32,
        pub y: f32,
    }

    #[derive(Component, Clone, Copy)]
    pub struct Acceleration {
        pub x: f32,
        pub y: f32,
    }

    /// Bundle for adding Velocity and Acceleration together
    #[derive(Bundle, Clone, Copy)]
    pub struct MovementBundle {
        pub velocity: Velocity,
        pub acceleration: Acceleration,
    }

    pub fn spawn_1000_entities(world: &mut World) -> Vec<Entity> {
        (0..1000)
            .map(|i| {
                let f = i as f32;
                world.spawn((Position { x: f, y: f }, Velocity { x: 1.0, y: 1.0 })).id()
            })
            .collect()
    }

    pub fn spawn_1000_entities_batch(world: &mut World) -> Vec<Entity> {
        let bundles: Vec<_> = (0..1000)
            .map(|i| {
                let f = i as f32;
                (Position { x: f, y: f }, Velocity { x: 1.0, y: 1.0 })
            })
            .collect();
        world.spawn_batch(bundles).collect()
    }

    pub fn spawn_1000_position_only(world: &mut World) -> Vec<Entity> {
        let bundles: Vec<_> = (0..1000)
            .map(|i| {
                let f = i as f32;
                Position { x: f, y: f }
            })
            .collect();
        world.spawn_batch(bundles).collect()
    }

    /// System that adds Velocity and Acceleration bundle to all Position entities
    pub fn add_components_system(query: Query<Entity, With<Position>>, mut commands: Commands) {
        for entity in query.iter() {
            commands.entity(entity).insert(MovementBundle {
                velocity: Velocity { x: 1.0, y: 1.0 },
                acceleration: Acceleration { x: 0.1, y: 0.1 },
            });
        }
    }
}

// ============================================================================
// Benchmark 1: Spawn 1000 entities with Position, Velocity bundle (one at a time)
// ============================================================================

fn bench_spawn_1000(c: &mut Criterion) {
    let mut group = c.benchmark_group("spawn_1000_individual");

    group.bench_function("nimbus", |b| {
        b.iter(|| {
            let mut world = nimbus_ecs::World::new();
            let entities = nimbus::spawn_1000_entities(&mut world);
            black_box(entities.len())
        });
    });

    group.bench_function("bevy", |b| {
        b.iter(|| {
            let mut world = bevy_ecs::world::World::new();
            let entities = bevy::spawn_1000_entities(&mut world);
            black_box(entities.len())
        });
    });

    group.finish();
}

// ============================================================================
// Benchmark 1b: Spawn 1000 entities with Position, Velocity bundle (batch)
// ============================================================================

fn bench_spawn_1000_batch(c: &mut Criterion) {
    let mut group = c.benchmark_group("spawn_1000_batch");

    group.bench_function("nimbus", |b| {
        b.iter(|| {
            let mut world = nimbus_ecs::World::new();
            let entities = nimbus::spawn_1000_entities_batch(&mut world);
            black_box(entities.len())
        });
    });

    group.bench_function("bevy", |b| {
        b.iter(|| {
            let mut world = bevy_ecs::world::World::new();
            let entities = bevy::spawn_1000_entities_batch(&mut world);
            black_box(entities.len())
        });
    });

    group.finish();
}

// ============================================================================
// Benchmark 2: Start with 1000 entities, remove velocity from one
// ============================================================================

fn bench_remove_component(c: &mut Criterion) {
    let mut group = c.benchmark_group("remove_component");

    group.bench_function("nimbus", |b| {
        b.iter_batched(
            || {
                let mut world = nimbus_ecs::World::new();
                let entities = nimbus::spawn_1000_entities(&mut world);
                (world, entities[500])
            },
            |(mut world, entity)| {
                world.remove::<nimbus::Velocity>(entity);
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.bench_function("bevy", |b| {
        b.iter_batched(
            || {
                let mut world = bevy_ecs::world::World::new();
                let entities = bevy::spawn_1000_entities(&mut world);
                (world, entities[500])
            },
            |(mut world, entity)| {
                world.entity_mut(entity).remove::<bevy::Velocity>();
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.finish();
}

// ============================================================================
// Benchmark 3: Start with 1000 entities, add acceleration to one
// ============================================================================

fn bench_add_component(c: &mut Criterion) {
    let mut group = c.benchmark_group("add_component");

    group.bench_function("nimbus", |b| {
        b.iter_batched(
            || {
                let mut world = nimbus_ecs::World::new();
                let entities = nimbus::spawn_1000_entities(&mut world);
                (world, entities[500])
            },
            |(mut world, entity)| {
                let _ = world.insert(entity, nimbus::Acceleration { x: 0.1, y: 0.1 });
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.bench_function("bevy", |b| {
        b.iter_batched(
            || {
                let mut world = bevy_ecs::world::World::new();
                let entities = bevy::spawn_1000_entities(&mut world);
                (world, entities[500])
            },
            |(mut world, entity)| {
                world.entity_mut(entity).insert(bevy::Acceleration { x: 0.1, y: 0.1 });
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.finish();
}

// ============================================================================
// Benchmark 4: Iterate 1000 entities with movement system
// ============================================================================

fn bench_iterate_system(c: &mut Criterion) {
    let mut group = c.benchmark_group("iterate_1000");

    // Baseline: raw Vec iteration (theoretical best case)
    group.bench_function("vec_baseline", |b| {
        // Two parallel arrays (SoA layout) - same as ECS archetype storage
        let mut positions: Vec<(f32, f32)> = (0..1000).map(|i| (i as f32, i as f32)).collect();
        let velocities: Vec<(f32, f32)> = (0..1000).map(|_| (1.0, 1.0)).collect();

        b.iter(|| {
            let ipositions = black_box(&mut positions);
            let ivelocities = black_box(&velocities);
            for (pos, vel) in ipositions.iter_mut().zip(ivelocities.iter()) {
                pos.0 += vel.0;
                pos.1 += vel.1;
            }
        });
    });

    group.bench_function("nimbus", |b| {
        let mut world = nimbus_ecs::World::new();
        nimbus::spawn_1000_entities(&mut world);

        b.iter(|| {
            world.run_system(nimbus::movement_system).unwrap();
        });
    });

    group.bench_function("bevy", |b| {
        let mut world = bevy_ecs::world::World::new();
        bevy::spawn_1000_entities(&mut world);

        // Create a system state for the query
        let mut system_state: bevy_ecs::system::SystemState<
            bevy_ecs::prelude::Query<(&mut bevy::Position, &bevy::Velocity)>,
        > = bevy_ecs::system::SystemState::new(&mut world);

        b.iter(|| {
            let mut query = system_state.get_mut(&mut world);
            for (mut pos, vel) in query.iter_mut() {
                pos.x += vel.x;
                pos.y += vel.y;
            }
        });
    });

    group.finish();
}

// ============================================================================
// Benchmark 5: Spawn 1000, remove 500, then iterate
// ============================================================================

fn bench_sparse_iteration(c: &mut Criterion) {
    let mut group = c.benchmark_group("iterate_sparse_500");

    group.bench_function("nimbus", |b| {
        b.iter_batched(
            || {
                let mut world = nimbus_ecs::World::new();
                let entities = nimbus::spawn_1000_entities(&mut world);
                // Remove every other entity
                for i in (0..1000).step_by(2) {
                    world.despawn(entities[i]);
                }
                world
            },
            |mut world| {
                world.run_system(nimbus::movement_system).unwrap();
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.bench_function("bevy", |b| {
        b.iter_batched(
            || {
                let mut world = bevy_ecs::world::World::new();
                let entities = bevy::spawn_1000_entities(&mut world);
                // Remove every other entity
                for i in (0..1000).step_by(2) {
                    world.despawn(entities[i]);
                }

                // Create the system state with the world
                let system_state: bevy_ecs::system::SystemState<
                    bevy_ecs::prelude::Query<(&mut bevy::Position, &bevy::Velocity)>,
                > = bevy_ecs::system::SystemState::new(&mut world);
                (world, system_state)
            },
            |(mut world, mut system_state)| {
                let mut query = system_state.get_mut(&mut world);
                for (mut pos, vel) in query.iter_mut() {
                    pos.x += vel.x;
                    pos.y += vel.y;
                }
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.finish();
}

// ============================================================================
// Benchmark 6: Commands add bundle to 1000 entities
// ============================================================================

fn bench_commands_add_bundle_to_all(c: &mut Criterion) {
    let mut group = c.benchmark_group("commands_add_bundle_1000");

    group.bench_function("nimbus", |b| {
        b.iter_batched(
            || {
                let mut world = nimbus_ecs::World::new();
                nimbus::spawn_1000_position_only(&mut world);
                world
            },
            |mut world| {
                world.run_system(nimbus::add_components_system).unwrap();
                black_box(())
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.bench_function("bevy", |b| {
        b.iter_batched(
            || {
                let mut world = bevy_ecs::world::World::new();
                bevy::spawn_1000_position_only(&mut world);
                
                // Create a schedule with our system
                let mut schedule = bevy_ecs::schedule::Schedule::default();
                schedule.add_systems(bevy::add_components_system);
                
                (world, schedule)
            },
            |(mut world, mut schedule)| {
                // Run the schedule (executes system + applies commands)
                schedule.run(&mut world);
                black_box(())
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.finish();
}

// ============================================================================
// Benchmark 8: Commands spawn 1000 entities
// ============================================================================

fn bench_commands_spawn_1000(c: &mut Criterion) {
    let mut group = c.benchmark_group("commands_spawn_1000");

    group.bench_function("nimbus", |b| {
        b.iter_batched(
            || nimbus_ecs::World::new(),
            |mut world| {
                world.run_system(|commands: nimbus_ecs::Commands| {
                    for i in 0..1000 {
                        let f = i as f32;
                        commands.spawn((
                            nimbus::Position { x: f, y: f },
                            nimbus::Velocity { x: 1.0, y: 1.0 },
                        ));
                    }
                }).unwrap();
                black_box(())
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.bench_function("bevy", |b| {
        b.iter_batched(
            || {
                #[allow(unused_mut)]
                let mut world = bevy_ecs::world::World::new();
                let mut schedule = bevy_ecs::schedule::Schedule::default();
                schedule.add_systems(|mut commands: bevy_ecs::prelude::Commands| {
                    for i in 0..1000 {
                        let f = i as f32;
                        commands.spawn((
                            bevy::Position { x: f, y: f },
                            bevy::Velocity { x: 1.0, y: 1.0 },
                        ));
                    }
                });
                (world, schedule)
            },
            |(mut world, mut schedule)| {
                schedule.run(&mut world);
                black_box(())
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.finish();
}

// ============================================================================
// Benchmark 9: Commands remove component from 1000 entities
// ============================================================================

fn bench_commands_remove_1000(c: &mut Criterion) {
    let mut group = c.benchmark_group("commands_remove_1000");

    group.bench_function("nimbus", |b| {
        b.iter_batched(
            || {
                let mut world = nimbus_ecs::World::new();
                nimbus::spawn_1000_entities(&mut world);
                world
            },
            |mut world| {
                world.run_system(|mut query: nimbus_ecs::Query<nimbus_ecs::Entity>, commands: nimbus_ecs::Commands| {
                    for entity in query.iter() {
                        commands.entity(entity).remove::<nimbus::Velocity>();
                    }
                }).unwrap();
                black_box(())
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.bench_function("bevy", |b| {
        b.iter_batched(
            || {
                let mut world = bevy_ecs::world::World::new();
                bevy::spawn_1000_entities(&mut world);
                
                let mut schedule = bevy_ecs::schedule::Schedule::default();
                schedule.add_systems(|query: bevy_ecs::prelude::Query<bevy_ecs::prelude::Entity>, mut commands: bevy_ecs::prelude::Commands| {
                    for entity in query.iter() {
                        commands.entity(entity).remove::<bevy::Velocity>();
                    }
                });
                (world, schedule)
            },
            |(mut world, mut schedule)| {
                schedule.run(&mut world);
                black_box(())
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.finish();
}

// ============================================================================
// Benchmark 10: Commands mixed operations (interleaved - worst case for naive impl)
// ============================================================================

fn bench_commands_mixed_1000(c: &mut Criterion) {
    let mut group = c.benchmark_group("commands_mixed_1000");

    // Interleaved pattern: spawn, insert, remove, despawn in round-robin
    // This tests the sort+batch optimization - without it, each command type
    // would be processed separately, causing many small batches
    
    group.bench_function("nimbus", |b| {
        b.iter_batched(
            || {
                let mut world = nimbus_ecs::World::new();
                let entities = nimbus::spawn_1000_entities(&mut world);
                (world, entities)
            },
            |(mut world, entities)| {
                world.run_system(move |commands: nimbus_ecs::Commands| {
                    // Interleave commands: step by 4, each iteration does one of each type
                    for i in 0..250 {
                        // Spawn
                        let f = i as f32;
                        commands.spawn((nimbus::Position { x: f, y: f }, nimbus::Velocity { x: 1.0, y: 1.0 }));
                        
                        // Insert (on entities 0-249)
                        commands.entity(entities[i]).insert(nimbus::Acceleration { x: 0.1, y: 0.1 });
                        
                        // Remove (on entities 250-499)
                        commands.entity(entities[250 + i]).remove::<nimbus::Velocity>();
                        
                        // Despawn (on entities 500-749)
                        commands.despawn(entities[500 + i]);
                    }
                }).unwrap();
                black_box(())
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.bench_function("bevy", |b| {
        b.iter_batched(
            || {
                let mut world = bevy_ecs::world::World::new();
                let entities: Vec<_> = bevy::spawn_1000_entities(&mut world).into_iter().collect();
                (world, entities)
            },
            |(mut world, entities)| {
                let mut system_state: bevy_ecs::system::SystemState<bevy_ecs::prelude::Commands> = 
                    bevy_ecs::system::SystemState::new(&mut world);
                {
                    let mut commands = system_state.get_mut(&mut world);
                    // Interleave commands: step by 4, each iteration does one of each type
                    for i in 0..250 {
                        // Spawn
                        let f = i as f32;
                        commands.spawn((bevy::Position { x: f, y: f }, bevy::Velocity { x: 1.0, y: 1.0 }));
                        
                        // Insert (on entities 0-249)
                        commands.entity(entities[i]).insert(bevy::Acceleration { x: 0.1, y: 0.1 });
                        
                        // Remove (on entities 250-499)
                        commands.entity(entities[250 + i]).remove::<bevy::Velocity>();
                        
                        // Despawn (on entities 500-749)
                        commands.entity(entities[500 + i]).despawn();
                    }
                }
                system_state.apply(&mut world);
                black_box(())
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.finish();
}

// ============================================================================
// Run length benchmark - tests skip-sort heuristic with different patterns
// ============================================================================

/// Benchmark that tests different "run lengths" of commands.
/// With larger runs, commands come in groups of the same type (few transitions).
/// With smaller runs, commands are more interleaved (many transitions).
fn bench_commands_run_lengths(c: &mut Criterion) {
    let mut group = c.benchmark_group("commands_run_lengths");
    
    // Test different run lengths: commands come in runs of N before switching type
    for run_length in [500, 250, 100, 50, 1] {
        group.bench_function(format!("nimbus_run_{}", run_length), |b| {
            b.iter_batched(
                || {
                    let mut world = nimbus_ecs::World::new();
                    // Pre-spawn 2000 entities so we have targets for insert/remove/despawn
                    let entities: Vec<_> = (0..2000)
                        .map(|i| {
                            let f = i as f32;
                            world.spawn_with((nimbus::Position { x: f, y: f }, nimbus::Velocity { x: 1.0, y: 1.0 }))
                        })
                        .collect();
                    (world, entities)
                },
                |(mut world, entities)| {
                    let run_length = run_length;
                    world.run_system(move |commands: nimbus_ecs::Commands| {
                        // Total: 2000 commands (500 of each type)
                        // Pattern: run_length of spawn, run_length of insert, run_length of remove, run_length of despawn, repeat
                        let mut spawn_idx = 0usize;
                        let mut insert_idx = 0usize;
                        let mut remove_idx = 0usize;
                        let mut despawn_idx = 0usize;
                        
                        let total_per_type = 500;
                        let mut remaining = [total_per_type; 4]; // spawn, insert, remove, despawn
                        
                        while remaining.iter().any(|&r| r > 0) {
                            // Spawn run
                            for _ in 0..run_length.min(remaining[0]) {
                                let f = spawn_idx as f32;
                                commands.spawn((nimbus::Position { x: f, y: f }, nimbus::Velocity { x: 1.0, y: 1.0 }));
                                spawn_idx += 1;
                                remaining[0] -= 1;
                            }
                            // Insert run
                            for _ in 0..run_length.min(remaining[1]) {
                                commands.entity(entities[insert_idx]).insert(nimbus::Acceleration { x: 0.1, y: 0.1 });
                                insert_idx += 1;
                                remaining[1] -= 1;
                            }
                            // Remove run
                            for _ in 0..run_length.min(remaining[2]) {
                                commands.entity(entities[500 + remove_idx]).remove::<nimbus::Velocity>();
                                remove_idx += 1;
                                remaining[2] -= 1;
                            }
                            // Despawn run
                            for _ in 0..run_length.min(remaining[3]) {
                                commands.despawn(entities[1000 + despawn_idx]);
                                despawn_idx += 1;
                                remaining[3] -= 1;
                            }
                        }
                    }).unwrap();
                    black_box(())
                },
                criterion::BatchSize::SmallInput,
            );
        });
        
        group.bench_function(format!("bevy_run_{}", run_length), |b| {
            b.iter_batched(
                || {
                    let mut world = bevy_ecs::world::World::new();
                    // Pre-spawn 2000 entities
                    let entities: Vec<_> = (0..2000)
                        .map(|i| {
                            let f = i as f32;
                            world.spawn((bevy::Position { x: f, y: f }, bevy::Velocity { x: 1.0, y: 1.0 })).id()
                        })
                        .collect();
                    (world, entities)
                },
                |(mut world, entities)| {
                    let mut system_state: bevy_ecs::system::SystemState<bevy_ecs::prelude::Commands> = 
                        bevy_ecs::system::SystemState::new(&mut world);
                    {
                        let mut commands = system_state.get_mut(&mut world);
                        
                        let mut spawn_idx = 0usize;
                        let mut insert_idx = 0usize;
                        let mut remove_idx = 0usize;
                        let mut despawn_idx = 0usize;
                        
                        let total_per_type = 500;
                        let mut remaining = [total_per_type; 4];
                        
                        while remaining.iter().any(|&r| r > 0) {
                            // Spawn run
                            for _ in 0..run_length.min(remaining[0]) {
                                let f = spawn_idx as f32;
                                commands.spawn((bevy::Position { x: f, y: f }, bevy::Velocity { x: 1.0, y: 1.0 }));
                                spawn_idx += 1;
                                remaining[0] -= 1;
                            }
                            // Insert run
                            for _ in 0..run_length.min(remaining[1]) {
                                commands.entity(entities[insert_idx]).insert(bevy::Acceleration { x: 0.1, y: 0.1 });
                                insert_idx += 1;
                                remaining[1] -= 1;
                            }
                            // Remove run
                            for _ in 0..run_length.min(remaining[2]) {
                                commands.entity(entities[500 + remove_idx]).remove::<bevy::Velocity>();
                                remove_idx += 1;
                                remaining[2] -= 1;
                            }
                            // Despawn run
                            for _ in 0..run_length.min(remaining[3]) {
                                commands.entity(entities[1000 + despawn_idx]).despawn();
                                despawn_idx += 1;
                                remaining[3] -= 1;
                            }
                        }
                    }
                    system_state.apply(&mut world);
                    black_box(())
                },
                criterion::BatchSize::SmallInput,
            );
        });
    }
    
    group.finish();
}

criterion_group!(
    benches,
    bench_spawn_1000,
    bench_spawn_1000_batch,
    bench_remove_component,
    bench_add_component,
    bench_iterate_system,
    bench_sparse_iteration,
    bench_commands_add_bundle_to_all,
    bench_commands_spawn_1000,
    bench_commands_remove_1000,
    bench_commands_mixed_1000,
    bench_commands_run_lengths,
);
criterion_main!(benches);
