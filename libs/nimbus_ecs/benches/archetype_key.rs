//! Benchmarks for ArchetypeKey operations.
//! Tests the production implementation using HashSet with identity hasher.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use nimbus_ecs::{ArchetypeKey, ComponentId};
use nimbus_ecs::component::const_fnv1a_64_str;

// ============================================================================
// ComponentId generation
// ============================================================================

/// Generate well-distributed ComponentIds that simulate real FNV-1a hashes.
/// Sequential IDs would cluster badly with identity hashing.
fn get_component_id(index: usize) -> ComponentId {
    // Simulate real component IDs by hashing a type-name-like string
    let name = format!("BenchComponent{}", index);
    ComponentId::new(const_fnv1a_64_str(&name))
}

fn get_new_component_id(key_size: usize) -> ComponentId {
    get_component_id(key_size)
}

fn get_existing_component_id(key_size: usize) -> ComponentId {
    get_component_id(key_size / 2)
}

// ============================================================================
// Benchmarks
// ============================================================================

fn bench_contains(c: &mut Criterion) {
    let mut group = c.benchmark_group("contains");

    for size in [5, 10, 20, 50] {
        let types: Vec<ComponentId> = (0..size).map(get_component_id).collect();
        let key = ArchetypeKey::new(types);

        let existing = get_existing_component_id(size);
        let non_existing = get_new_component_id(size);

        group.bench_with_input(
            BenchmarkId::new("existing", size),
            &size,
            |b, _| b.iter(|| black_box(key.contains(black_box(existing)))),
        );
        group.bench_with_input(
            BenchmarkId::new("missing", size),
            &size,
            |b, _| b.iter(|| black_box(key.contains(black_box(non_existing)))),
        );
    }

    group.finish();
}

fn bench_with_type(c: &mut Criterion) {
    let mut group = c.benchmark_group("with_type");

    for size in [5, 10, 20, 50] {
        let types: Vec<ComponentId> = (0..size).map(get_component_id).collect();
        let key = ArchetypeKey::new(types);

        let new_type = get_new_component_id(size);

        group.bench_with_input(BenchmarkId::new("add", size), &size, |b, _| {
            b.iter(|| black_box(key.with_type(black_box(new_type))))
        });
    }

    group.finish();
}

fn bench_without_type(c: &mut Criterion) {
    let mut group = c.benchmark_group("without_type");

    for size in [5, 10, 20, 50] {
        let types: Vec<ComponentId> = (0..size).map(get_component_id).collect();
        let key = ArchetypeKey::new(types);

        let existing = get_existing_component_id(size);

        group.bench_with_input(BenchmarkId::new("remove", size), &size, |b, _| {
            b.iter(|| black_box(key.without_type(black_box(existing))))
        });
    }

    group.finish();
}

fn bench_new(c: &mut Criterion) {
    let mut group = c.benchmark_group("new");

    for size in [5, 10, 20, 50] {
        let types: Vec<ComponentId> = (0..size).map(get_component_id).collect();

        group.bench_with_input(BenchmarkId::new("construct", size), &types, |b, types| {
            b.iter(|| black_box(ArchetypeKey::from_iter(black_box(types.iter().copied()))))
        });
    }

    group.finish();
}

fn bench_contains_all(c: &mut Criterion) {
    let mut group = c.benchmark_group("contains_all");

    for size in [5, 10, 20, 50] {
        let types: Vec<ComponentId> = (0..size).map(get_component_id).collect();
        let key = ArchetypeKey::new(types);

        // Query for half the types
        let query_types: Vec<ComponentId> = (0..size / 2).map(get_component_id).collect();

        group.bench_with_input(BenchmarkId::new("half", size), &size, |b, _| {
            b.iter(|| black_box(key.contains_all(black_box(&query_types))))
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_contains,
    bench_with_type,
    bench_without_type,
    bench_new,
    bench_contains_all
);
criterion_main!(benches);
