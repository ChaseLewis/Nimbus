use criterion::{Criterion, black_box, criterion_group, criterion_main};
use glam::Vec3 as GlamVec3;
use nimbus_math::Vec3;

fn bench_add(c: &mut Criterion) {
    let mut group = c.benchmark_group("Vec3 Add");

    let a = Vec3::new(1.0, 2.0, 3.0);
    let b = Vec3::new(4.0, 5.0, 6.0);

    group.bench_function("nimbus-math", |bencher| {
        bencher.iter(|| black_box(a) + black_box(b));
    });

    let ga = GlamVec3::new(1.0, 2.0, 3.0);
    let gb = GlamVec3::new(4.0, 5.0, 6.0);

    group.bench_function("glam", |bencher| {
        bencher.iter(|| black_box(ga) + black_box(gb));
    });

    group.finish();
}

fn bench_sub(c: &mut Criterion) {
    let mut group = c.benchmark_group("Vec3 Sub");

    let a = Vec3::new(5.0, 7.0, 9.0);
    let b = Vec3::new(2.0, 3.0, 4.0);

    group.bench_function("nimbus-math", |bencher| {
        bencher.iter(|| black_box(a) - black_box(b));
    });

    let ga = GlamVec3::new(5.0, 7.0, 9.0);
    let gb = GlamVec3::new(2.0, 3.0, 4.0);

    group.bench_function("glam", |bencher| {
        bencher.iter(|| black_box(ga) - black_box(gb));
    });

    group.finish();
}

fn bench_mul_scalar(c: &mut Criterion) {
    let mut group = c.benchmark_group("Vec3 Mul Scalar");

    let v = Vec3::new(2.0, 3.0, 4.0);
    let scalar = 2.5;

    group.bench_function("nimbus-math", |bencher| {
        bencher.iter(|| black_box(v) * black_box(scalar));
    });

    let gv = GlamVec3::new(2.0, 3.0, 4.0);

    group.bench_function("glam", |bencher| {
        bencher.iter(|| black_box(gv) * black_box(scalar));
    });

    group.finish();
}

fn bench_dot(c: &mut Criterion) {
    let mut group = c.benchmark_group("Vec3 Dot");

    let a = Vec3::new(1.0, 2.0, 3.0);
    let b = Vec3::new(4.0, 5.0, 6.0);

    group.bench_function("nimbus-math", |bencher| {
        bencher.iter(|| black_box(a).dot(black_box(b)));
    });

    let ga = GlamVec3::new(1.0, 2.0, 3.0);
    let gb = GlamVec3::new(4.0, 5.0, 6.0);

    group.bench_function("glam", |bencher| {
        bencher.iter(|| black_box(ga).dot(black_box(gb)));
    });

    group.finish();
}

// Scalar implementation of cross product for comparison
#[inline]
fn vec3_cross_scalar(a: Vec3, b: Vec3) -> Vec3 {
    Vec3 {
        x: a.y * b.z - a.z * b.y,
        y: a.z * b.x - a.x * b.z,
        z: a.x * b.y - a.y * b.x,
    }
}

fn bench_cross(c: &mut Criterion) {
    let mut group = c.benchmark_group("Vec3 Cross");

    let a = Vec3::new(1.0, 2.0, 3.0);
    let b = Vec3::new(4.0, 5.0, 6.0);

    // Test current implementation (SSE when available)
    group.bench_function("nimbus-math (current)", |bencher| {
        bencher.iter(|| black_box(a).cross(black_box(b)));
    });

    // Test scalar implementation explicitly
    group.bench_function("nimbus-math (scalar)", |bencher| {
        bencher.iter(|| vec3_cross_scalar(black_box(a), black_box(b)));
    });

    let ga = GlamVec3::new(1.0, 2.0, 3.0);
    let gb = GlamVec3::new(4.0, 5.0, 6.0);

    group.bench_function("glam", |bencher| {
        bencher.iter(|| black_box(ga).cross(black_box(gb)));
    });

    group.finish();
}

fn bench_length(c: &mut Criterion) {
    let mut group = c.benchmark_group("Vec3 Length");

    let v = Vec3::new(2.0, 3.0, 6.0);

    group.bench_function("nimbus-math", |bencher| {
        bencher.iter(|| black_box(v).length());
    });

    let gv = GlamVec3::new(2.0, 3.0, 6.0);

    group.bench_function("glam", |bencher| {
        bencher.iter(|| black_box(gv).length());
    });

    group.finish();
}

fn bench_normalize(c: &mut Criterion) {
    let mut group = c.benchmark_group("Vec3 Normalize");

    let v = Vec3::new(2.0, 3.0, 6.0);

    group.bench_function("nimbus-math", |bencher| {
        bencher.iter(|| black_box(v).normalize());
    });

    let gv = GlamVec3::new(2.0, 3.0, 6.0);

    group.bench_function("glam", |bencher| {
        bencher.iter(|| black_box(gv).normalize());
    });

    group.finish();
}

fn bench_length_squared(c: &mut Criterion) {
    let mut group = c.benchmark_group("Vec3 Length Squared");

    let v = Vec3::new(2.0, 3.0, 6.0);

    group.bench_function("nimbus-math", |bencher| {
        bencher.iter(|| black_box(v).length_squared());
    });

    let gv = GlamVec3::new(2.0, 3.0, 6.0);

    group.bench_function("glam", |bencher| {
        bencher.iter(|| black_box(gv).length_squared());
    });

    group.finish();
}

fn bench_floor(c: &mut Criterion) {
    let mut group = c.benchmark_group("Vec3 Floor");

    let v = Vec3::new(1.7, -1.3, 2.5);

    group.bench_function("nimbus-math", |bencher| {
        bencher.iter(|| black_box(v).floor());
    });

    let gv = GlamVec3::new(1.7, -1.3, 2.5);

    group.bench_function("glam", |bencher| {
        bencher.iter(|| black_box(gv).floor());
    });

    group.finish();
}

fn bench_ceil(c: &mut Criterion) {
    let mut group = c.benchmark_group("Vec3 Ceil");

    let v = Vec3::new(1.7, -1.3, 2.5);

    group.bench_function("nimbus-math", |bencher| {
        bencher.iter(|| black_box(v).ceil());
    });

    let gv = GlamVec3::new(1.7, -1.3, 2.5);

    group.bench_function("glam", |bencher| {
        bencher.iter(|| black_box(gv).ceil());
    });

    group.finish();
}

fn bench_round(c: &mut Criterion) {
    let mut group = c.benchmark_group("Vec3 Round");

    let v = Vec3::new(1.7, -1.3, 2.5);

    group.bench_function("nimbus-math", |bencher| {
        bencher.iter(|| black_box(v).round());
    });

    let gv = GlamVec3::new(1.7, -1.3, 2.5);

    group.bench_function("glam", |bencher| {
        bencher.iter(|| black_box(gv).round());
    });

    group.finish();
}

fn bench_trunc(c: &mut Criterion) {
    let mut group = c.benchmark_group("Vec3 Trunc");

    let v = Vec3::new(1.7, -1.3, 2.5);

    group.bench_function("nimbus-math", |bencher| {
        bencher.iter(|| black_box(v).trunc());
    });

    let gv = GlamVec3::new(1.7, -1.3, 2.5);

    group.bench_function("glam", |bencher| {
        bencher.iter(|| black_box(gv).trunc());
    });

    group.finish();
}

fn bench_min_f32(c: &mut Criterion) {
    let mut group = c.benchmark_group("Vec3 Min F32");

    let v = Vec3::new(5.0, 2.0, 8.0);
    let scalar = 4.0;

    group.bench_function("nimbus-math", |bencher| {
        bencher.iter(|| black_box(v).min_f32(black_box(scalar)));
    });

    let gv = GlamVec3::new(5.0, 2.0, 8.0);

    group.bench_function("glam", |bencher| {
        bencher.iter(|| black_box(gv).min(GlamVec3::splat(black_box(scalar))));
    });

    group.finish();
}

fn bench_max_f32(c: &mut Criterion) {
    let mut group = c.benchmark_group("Vec3 Max F32");

    let v = Vec3::new(5.0, 2.0, 8.0);
    let scalar = 4.0;

    group.bench_function("nimbus-math", |bencher| {
        bencher.iter(|| black_box(v).max_f32(black_box(scalar)));
    });

    let gv = GlamVec3::new(5.0, 2.0, 8.0);

    group.bench_function("glam", |bencher| {
        bencher.iter(|| black_box(gv).max(GlamVec3::splat(black_box(scalar))));
    });

    group.finish();
}

fn bench_clamp_f32(c: &mut Criterion) {
    let mut group = c.benchmark_group("Vec3 Clamp F32");

    let v = Vec3::new(1.0, 5.0, 2.0);
    let min = 2.0;
    let max = 6.0;

    group.bench_function("nimbus-math", |bencher| {
        bencher.iter(|| black_box(v).clamp_f32(black_box(min), black_box(max)));
    });

    let gv = GlamVec3::new(1.0, 5.0, 2.0);

    group.bench_function("glam", |bencher| {
        bencher.iter(|| {
            black_box(gv)
                .max(GlamVec3::splat(black_box(min)))
                .min(GlamVec3::splat(black_box(max)))
        });
    });

    group.finish();
}

fn bench_rem(c: &mut Criterion) {
    let mut group = c.benchmark_group("Vec3 Rem");

    let v = Vec3::new(10.0, 7.0, 15.0);
    let scalar = 4.0;

    group.bench_function("nimbus-math", |bencher| {
        bencher.iter(|| black_box(v) % black_box(scalar));
    });

    let gv = GlamVec3::new(10.0, 7.0, 15.0);

    group.bench_function("glam", |bencher| {
        bencher.iter(|| {
            let gv = black_box(gv);
            let s = black_box(scalar);
            GlamVec3::new(gv.x % s, gv.y % s, gv.z % s)
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_add,
    bench_sub,
    bench_mul_scalar,
    bench_dot,
    bench_cross,
    bench_length,
    bench_normalize,
    bench_length_squared,
    bench_floor,
    bench_ceil,
    bench_round,
    bench_trunc,
    bench_min_f32,
    bench_max_f32,
    bench_clamp_f32,
    bench_rem
);
criterion_main!(benches);
