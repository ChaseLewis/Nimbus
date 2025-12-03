use criterion::{Criterion, black_box, criterion_group, criterion_main};
use glam::Vec2 as GlamVec2;
use nimbus_math::Vec2;

fn bench_add(c: &mut Criterion) {
    let mut group = c.benchmark_group("Vec2 Add");

    let a = Vec2::new(1.0, 2.0);
    let b = Vec2::new(3.0, 4.0);

    group.bench_function("nimbus-math", |bencher| {
        bencher.iter(|| black_box(a) + black_box(b));
    });

    let ga = GlamVec2::new(1.0, 2.0);
    let gb = GlamVec2::new(3.0, 4.0);

    group.bench_function("glam", |bencher| {
        bencher.iter(|| black_box(ga) + black_box(gb));
    });

    group.finish();
}

fn bench_sub(c: &mut Criterion) {
    let mut group = c.benchmark_group("Vec2 Sub");

    let a = Vec2::new(5.0, 7.0);
    let b = Vec2::new(2.0, 3.0);

    group.bench_function("nimbus-math", |bencher| {
        bencher.iter(|| black_box(a) - black_box(b));
    });

    let ga = GlamVec2::new(5.0, 7.0);
    let gb = GlamVec2::new(2.0, 3.0);

    group.bench_function("glam", |bencher| {
        bencher.iter(|| black_box(ga) - black_box(gb));
    });

    group.finish();
}

fn bench_mul_scalar(c: &mut Criterion) {
    let mut group = c.benchmark_group("Vec2 Mul Scalar");

    let v = Vec2::new(2.0, 3.0);
    let scalar = 2.5;

    group.bench_function("nimbus-math", |bencher| {
        bencher.iter(|| black_box(v) * black_box(scalar));
    });

    let gv = GlamVec2::new(2.0, 3.0);

    group.bench_function("glam", |bencher| {
        bencher.iter(|| black_box(gv) * black_box(scalar));
    });

    group.finish();
}

fn bench_dot(c: &mut Criterion) {
    let mut group = c.benchmark_group("Vec2 Dot");

    let a = Vec2::new(1.0, 2.0);
    let b = Vec2::new(3.0, 4.0);

    group.bench_function("nimbus-math", |bencher| {
        bencher.iter(|| black_box(a).dot(black_box(b)));
    });

    let ga = GlamVec2::new(1.0, 2.0);
    let gb = GlamVec2::new(3.0, 4.0);

    group.bench_function("glam", |bencher| {
        bencher.iter(|| black_box(ga).dot(black_box(gb)));
    });

    group.finish();
}

fn bench_length(c: &mut Criterion) {
    let mut group = c.benchmark_group("Vec2 Length");

    let v = Vec2::new(3.0, 4.0);

    group.bench_function("nimbus-math", |bencher| {
        bencher.iter(|| black_box(v).length());
    });

    let gv = GlamVec2::new(3.0, 4.0);

    group.bench_function("glam", |bencher| {
        bencher.iter(|| black_box(gv).length());
    });

    group.finish();
}

fn bench_normalize(c: &mut Criterion) {
    let mut group = c.benchmark_group("Vec2 Normalize");

    let v = Vec2::new(3.0, 4.0);

    group.bench_function("nimbus-math", |bencher| {
        bencher.iter(|| black_box(v).normalize());
    });

    let gv = GlamVec2::new(3.0, 4.0);

    group.bench_function("glam", |bencher| {
        bencher.iter(|| black_box(gv).normalize());
    });

    group.finish();
}

fn bench_length_squared(c: &mut Criterion) {
    let mut group = c.benchmark_group("Vec2 Length Squared");

    let v = Vec2::new(3.0, 4.0);

    group.bench_function("nimbus-math", |bencher| {
        bencher.iter(|| black_box(v).length_squared());
    });

    let gv = GlamVec2::new(3.0, 4.0);

    group.bench_function("glam", |bencher| {
        bencher.iter(|| black_box(gv).length_squared());
    });

    group.finish();
}

fn bench_floor(c: &mut Criterion) {
    let mut group = c.benchmark_group("Vec2 Floor");

    let v = Vec2::new(1.7, -1.3);

    group.bench_function("nimbus-math", |bencher| {
        bencher.iter(|| black_box(v).floor());
    });

    let gv = GlamVec2::new(1.7, -1.3);

    group.bench_function("glam", |bencher| {
        bencher.iter(|| black_box(gv).floor());
    });

    group.finish();
}

fn bench_ceil(c: &mut Criterion) {
    let mut group = c.benchmark_group("Vec2 Ceil");

    let v = Vec2::new(1.7, -1.3);

    group.bench_function("nimbus-math", |bencher| {
        bencher.iter(|| black_box(v).ceil());
    });

    let gv = GlamVec2::new(1.7, -1.3);

    group.bench_function("glam", |bencher| {
        bencher.iter(|| black_box(gv).ceil());
    });

    group.finish();
}

fn bench_round(c: &mut Criterion) {
    let mut group = c.benchmark_group("Vec2 Round");

    let v = Vec2::new(1.7, -1.3);

    group.bench_function("nimbus-math", |bencher| {
        bencher.iter(|| black_box(v).round());
    });

    let gv = GlamVec2::new(1.7, -1.3);

    group.bench_function("glam", |bencher| {
        bencher.iter(|| black_box(gv).round());
    });

    group.finish();
}

fn bench_trunc(c: &mut Criterion) {
    let mut group = c.benchmark_group("Vec2 Trunc");

    let v = Vec2::new(1.7, -1.3);

    group.bench_function("nimbus-math", |bencher| {
        bencher.iter(|| black_box(v).trunc());
    });

    let gv = GlamVec2::new(1.7, -1.3);

    group.bench_function("glam", |bencher| {
        bencher.iter(|| black_box(gv).trunc());
    });

    group.finish();
}

fn bench_min_f32(c: &mut Criterion) {
    let mut group = c.benchmark_group("Vec2 Min F32");

    let v = Vec2::new(5.0, 2.0);
    let scalar = 4.0;

    group.bench_function("nimbus-math", |bencher| {
        bencher.iter(|| black_box(v).min_f32(black_box(scalar)));
    });

    let gv = GlamVec2::new(5.0, 2.0);

    group.bench_function("glam", |bencher| {
        bencher.iter(|| black_box(gv).min(GlamVec2::splat(black_box(scalar))));
    });

    group.finish();
}

fn bench_max_f32(c: &mut Criterion) {
    let mut group = c.benchmark_group("Vec2 Max F32");

    let v = Vec2::new(5.0, 2.0);
    let scalar = 4.0;

    group.bench_function("nimbus-math", |bencher| {
        bencher.iter(|| black_box(v).max_f32(black_box(scalar)));
    });

    let gv = GlamVec2::new(5.0, 2.0);

    group.bench_function("glam", |bencher| {
        bencher.iter(|| black_box(gv).max(GlamVec2::splat(black_box(scalar))));
    });

    group.finish();
}

fn bench_clamp_f32(c: &mut Criterion) {
    let mut group = c.benchmark_group("Vec2 Clamp F32");

    let v = Vec2::new(1.0, 5.0);
    let min = 2.0;
    let max = 6.0;

    group.bench_function("nimbus-math", |bencher| {
        bencher.iter(|| black_box(v).clamp_f32(black_box(min), black_box(max)));
    });

    let gv = GlamVec2::new(1.0, 5.0);

    group.bench_function("glam", |bencher| {
        bencher.iter(|| {
            black_box(gv)
                .max(GlamVec2::splat(black_box(min)))
                .min(GlamVec2::splat(black_box(max)))
        });
    });

    group.finish();
}

fn bench_rem(c: &mut Criterion) {
    let mut group = c.benchmark_group("Vec2 Rem");

    let v = Vec2::new(10.0, 7.0);
    let scalar = 4.0;

    group.bench_function("nimbus-math", |bencher| {
        bencher.iter(|| black_box(v) % black_box(scalar));
    });

    let gv = GlamVec2::new(10.0, 7.0);

    group.bench_function("glam", |bencher| {
        bencher.iter(|| {
            let gv = black_box(gv);
            let s = black_box(scalar);
            GlamVec2::new(gv.x % s, gv.y % s)
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
