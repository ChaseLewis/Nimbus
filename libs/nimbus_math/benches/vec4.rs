use criterion::{Criterion, black_box, criterion_group, criterion_main};
use glam::Vec4 as GlamVec4;
use nimbus_math::Vec4;

fn bench_add(c: &mut Criterion) {
    let mut group = c.benchmark_group("Vec4 Add");

    let a = Vec4::new(1.0, 2.0, 3.0, 4.0);
    let b = Vec4::new(5.0, 6.0, 7.0, 8.0);

    group.bench_function("nimbus-math", |bencher| {
        bencher.iter(|| black_box(a) + black_box(b));
    });

    let ga = GlamVec4::new(1.0, 2.0, 3.0, 4.0);
    let gb = GlamVec4::new(5.0, 6.0, 7.0, 8.0);

    group.bench_function("glam", |bencher| {
        bencher.iter(|| black_box(ga) + black_box(gb));
    });

    group.finish();
}

fn bench_sub(c: &mut Criterion) {
    let mut group = c.benchmark_group("Vec4 Sub");

    let a = Vec4::new(5.0, 7.0, 9.0, 11.0);
    let b = Vec4::new(2.0, 3.0, 4.0, 5.0);

    group.bench_function("nimbus-math", |bencher| {
        bencher.iter(|| black_box(a) - black_box(b));
    });

    let ga = GlamVec4::new(5.0, 7.0, 9.0, 11.0);
    let gb = GlamVec4::new(2.0, 3.0, 4.0, 5.0);

    group.bench_function("glam", |bencher| {
        bencher.iter(|| black_box(ga) - black_box(gb));
    });

    group.finish();
}

fn bench_mul_scalar(c: &mut Criterion) {
    let mut group = c.benchmark_group("Vec4 Mul Scalar");

    let v = Vec4::new(2.0, 3.0, 4.0, 5.0);
    let scalar = 2.5;

    group.bench_function("nimbus-math", |bencher| {
        bencher.iter(|| black_box(v) * black_box(scalar));
    });

    let gv = GlamVec4::new(2.0, 3.0, 4.0, 5.0);

    group.bench_function("glam", |bencher| {
        bencher.iter(|| black_box(gv) * black_box(scalar));
    });

    group.finish();
}

fn bench_dot(c: &mut Criterion) {
    let mut group = c.benchmark_group("Vec4 Dot");

    let a = Vec4::new(1.0, 2.0, 3.0, 4.0);
    let b = Vec4::new(5.0, 6.0, 7.0, 8.0);

    group.bench_function("nimbus-math", |bencher| {
        bencher.iter(|| black_box(a).dot(black_box(b)));
    });

    let ga = GlamVec4::new(1.0, 2.0, 3.0, 4.0);
    let gb = GlamVec4::new(5.0, 6.0, 7.0, 8.0);

    group.bench_function("glam", |bencher| {
        bencher.iter(|| black_box(ga).dot(black_box(gb)));
    });

    group.finish();
}

fn bench_length(c: &mut Criterion) {
    let mut group = c.benchmark_group("Vec4 Length");

    let v = Vec4::new(2.0, 3.0, 6.0, 6.0);

    group.bench_function("nimbus-math", |bencher| {
        bencher.iter(|| black_box(v).length());
    });

    let gv = GlamVec4::new(2.0, 3.0, 6.0, 6.0);

    group.bench_function("glam", |bencher| {
        bencher.iter(|| black_box(gv).length());
    });

    group.finish();
}

fn bench_normalize(c: &mut Criterion) {
    let mut group = c.benchmark_group("Vec4 Normalize");

    let v = Vec4::new(2.0, 3.0, 6.0, 6.0);

    group.bench_function("nimbus-math", |bencher| {
        bencher.iter(|| black_box(v).normalize());
    });

    let gv = GlamVec4::new(2.0, 3.0, 6.0, 6.0);

    group.bench_function("glam", |bencher| {
        bencher.iter(|| black_box(gv).normalize());
    });

    group.finish();
}

fn bench_length_squared(c: &mut Criterion) {
    let mut group = c.benchmark_group("Vec4 Length Squared");

    let v = Vec4::new(2.0, 3.0, 6.0, 6.0);

    group.bench_function("nimbus-math", |bencher| {
        bencher.iter(|| black_box(v).length_squared());
    });

    let gv = GlamVec4::new(2.0, 3.0, 6.0, 6.0);

    group.bench_function("glam", |bencher| {
        bencher.iter(|| black_box(gv).length_squared());
    });

    group.finish();
}

fn bench_floor(c: &mut Criterion) {
    let mut group = c.benchmark_group("Vec4 Floor");

    let v = Vec4::new(1.7, -1.3, 2.5, -0.5);

    group.bench_function("nimbus-math", |bencher| {
        bencher.iter(|| black_box(v).floor());
    });

    let gv = GlamVec4::new(1.7, -1.3, 2.5, -0.5);

    group.bench_function("glam", |bencher| {
        bencher.iter(|| black_box(gv).floor());
    });

    group.finish();
}

fn bench_ceil(c: &mut Criterion) {
    let mut group = c.benchmark_group("Vec4 Ceil");

    let v = Vec4::new(1.7, -1.3, 2.5, -0.5);

    group.bench_function("nimbus-math", |bencher| {
        bencher.iter(|| black_box(v).ceil());
    });

    let gv = GlamVec4::new(1.7, -1.3, 2.5, -0.5);

    group.bench_function("glam", |bencher| {
        bencher.iter(|| black_box(gv).ceil());
    });

    group.finish();
}

fn bench_round(c: &mut Criterion) {
    let mut group = c.benchmark_group("Vec4 Round");

    let v = Vec4::new(1.7, -1.3, 2.5, -0.5);

    group.bench_function("nimbus-math", |bencher| {
        bencher.iter(|| black_box(v).round());
    });

    let gv = GlamVec4::new(1.7, -1.3, 2.5, -0.5);

    group.bench_function("glam", |bencher| {
        bencher.iter(|| black_box(gv).round());
    });

    group.finish();
}

// Benchmark comparing SSE4.1 vs SSE (glam-style) rounding implementations
fn bench_round_implementations(c: &mut Criterion) {
    let mut group = c.benchmark_group("Vec4 Round Implementation Comparison");

    let v = Vec4::new(1.7, -1.3, 2.5, -0.5);

    #[cfg(all(target_arch = "x86_64", target_feature = "sse4.1"))]
    {
        // SSE4.1 implementation (blendv + round_ps)
        group.bench_function("SSE4.1 (blendv + round_ps)", |bencher| {
            bencher.iter(|| black_box(v).round());
        });
    }

    #[cfg(all(target_arch = "x86_64", target_feature = "sse"))]
    {
        // SSE (glam/DirectXMath style) implementation
        use nimbus_math::round_sse;
        group.bench_function("SSE (glam-style DirectXMath)", |bencher| {
            bencher.iter(|| round_sse(black_box(v)));
        });
    }

    // For comparison with glam
    let gv = GlamVec4::new(1.7, -1.3, 2.5, -0.5);
    group.bench_function("glam", |bencher| {
        bencher.iter(|| black_box(gv).round());
    });

    group.finish();
}

fn bench_trunc(c: &mut Criterion) {
    let mut group = c.benchmark_group("Vec4 Trunc");

    let v = Vec4::new(1.7, -1.3, 2.5, -0.5);

    group.bench_function("nimbus-math", |bencher| {
        bencher.iter(|| black_box(v).trunc());
    });

    let gv = GlamVec4::new(1.7, -1.3, 2.5, -0.5);

    group.bench_function("glam", |bencher| {
        bencher.iter(|| black_box(gv).trunc());
    });

    group.finish();
}

fn bench_min_f32(c: &mut Criterion) {
    let mut group = c.benchmark_group("Vec4 Min F32");

    let v = Vec4::new(5.0, 2.0, 8.0, 3.0);
    let scalar = 4.0;

    group.bench_function("nimbus-math", |bencher| {
        bencher.iter(|| black_box(v).min_f32(black_box(scalar)));
    });

    let gv = GlamVec4::new(5.0, 2.0, 8.0, 3.0);

    group.bench_function("glam", |bencher| {
        bencher.iter(|| black_box(gv).min(GlamVec4::splat(black_box(scalar))));
    });

    group.finish();
}

fn bench_max_f32(c: &mut Criterion) {
    let mut group = c.benchmark_group("Vec4 Max F32");

    let v = Vec4::new(5.0, 2.0, 8.0, 3.0);
    let scalar = 4.0;

    group.bench_function("nimbus-math", |bencher| {
        bencher.iter(|| black_box(v).max_f32(black_box(scalar)));
    });

    let gv = GlamVec4::new(5.0, 2.0, 8.0, 3.0);

    group.bench_function("glam", |bencher| {
        bencher.iter(|| black_box(gv).max(GlamVec4::splat(black_box(scalar))));
    });

    group.finish();
}

fn bench_clamp_f32(c: &mut Criterion) {
    let mut group = c.benchmark_group("Vec4 Clamp F32");

    let v = Vec4::new(1.0, 5.0, 2.0, 8.0);
    let min = 2.0;
    let max = 6.0;

    group.bench_function("nimbus-math", |bencher| {
        bencher.iter(|| black_box(v).clamp_f32(black_box(min), black_box(max)));
    });

    let gv = GlamVec4::new(1.0, 5.0, 2.0, 8.0);

    group.bench_function("glam", |bencher| {
        bencher.iter(|| {
            black_box(gv)
                .max(GlamVec4::splat(black_box(min)))
                .min(GlamVec4::splat(black_box(max)))
        });
    });

    group.finish();
}

fn bench_rem(c: &mut Criterion) {
    let mut group = c.benchmark_group("Vec4 Rem");

    let v = Vec4::new(10.0, 7.0, 15.0, 9.0);
    let scalar = 4.0;

    group.bench_function("nimbus-math", |bencher| {
        bencher.iter(|| black_box(v) % black_box(scalar));
    });

    let gv = GlamVec4::new(10.0, 7.0, 15.0, 9.0);

    group.bench_function("glam", |bencher| {
        bencher.iter(|| black_box(gv) % black_box(scalar));
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
    bench_round_implementations,
    bench_trunc,
    bench_min_f32,
    bench_max_f32,
    bench_clamp_f32,
    bench_rem
);
criterion_main!(benches);
