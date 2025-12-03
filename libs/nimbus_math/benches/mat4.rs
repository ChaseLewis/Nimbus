use criterion::{Criterion, black_box, criterion_group, criterion_main};
use glam::{Mat4 as GlamMat4, Quat as GlamQuat, Vec3 as GlamVec3, Vec4 as GlamVec4};
use nimbus_math::{Vec3, Vec4, mat4::Mat4, quat::Quat};
use std::f32::consts::PI;

fn bench_mat4_mat4_mul(c: &mut Criterion) {
    let mut group = c.benchmark_group("Mat4 * Mat4");

    let m1 = Mat4::from_cols(
        Vec4::new(1.0, 2.0, 3.0, 4.0),
        Vec4::new(5.0, 6.0, 7.0, 8.0),
        Vec4::new(9.0, 10.0, 11.0, 12.0),
        Vec4::new(13.0, 14.0, 15.0, 16.0),
    );
    let m2 = Mat4::from_cols(
        Vec4::new(2.0, 3.0, 4.0, 5.0),
        Vec4::new(6.0, 7.0, 8.0, 9.0),
        Vec4::new(10.0, 11.0, 12.0, 13.0),
        Vec4::new(14.0, 15.0, 16.0, 17.0),
    );

    group.bench_function("nimbus-math", |bencher| {
        bencher.iter(|| black_box(m1) * black_box(m2));
    });

    let gm1 = GlamMat4::from_cols_array(&[
        1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0,
    ]);
    let gm2 = GlamMat4::from_cols_array(&[
        2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0,
    ]);

    group.bench_function("glam", |bencher| {
        bencher.iter(|| black_box(gm1) * black_box(gm2));
    });

    group.finish();
}

fn bench_mat4_vec4_mul(c: &mut Criterion) {
    let mut group = c.benchmark_group("Mat4 * Vec4");

    let m = Mat4::from_cols(
        Vec4::new(1.0, 2.0, 3.0, 4.0),
        Vec4::new(5.0, 6.0, 7.0, 8.0),
        Vec4::new(9.0, 10.0, 11.0, 12.0),
        Vec4::new(13.0, 14.0, 15.0, 16.0),
    );
    let v = Vec4::new(1.0, 2.0, 3.0, 4.0);

    group.bench_function("nimbus-math", |bencher| {
        bencher.iter(|| black_box(m) * black_box(v));
    });

    let gm = GlamMat4::from_cols_array(&[
        1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0,
    ]);
    let gv = GlamVec4::new(1.0, 2.0, 3.0, 4.0);

    group.bench_function("glam", |bencher| {
        bencher.iter(|| black_box(gm) * black_box(gv));
    });

    group.finish();
}

fn bench_mat4_determinant(c: &mut Criterion) {
    let mut group = c.benchmark_group("Mat4 Determinant");

    let m = Mat4::from_cols(
        Vec4::new(1.0, 2.0, 3.0, 4.0),
        Vec4::new(5.0, 6.0, 7.0, 8.0),
        Vec4::new(9.0, 10.0, 11.0, 12.0),
        Vec4::new(13.0, 14.0, 15.0, 16.0),
    );

    group.bench_function("nimbus-math", |bencher| {
        bencher.iter(|| black_box(m).determinant());
    });

    let gm = GlamMat4::from_cols_array(&[
        1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0,
    ]);

    group.bench_function("glam", |bencher| {
        bencher.iter(|| black_box(gm).determinant());
    });

    group.finish();
}

fn bench_mat4_inverse(c: &mut Criterion) {
    let mut group = c.benchmark_group("Mat4 Inverse");

    // Use a non-singular matrix
    let m = Mat4::from_cols(
        Vec4::new(1.0, 0.0, 0.0, 0.0),
        Vec4::new(0.0, 2.0, 0.0, 0.0),
        Vec4::new(0.0, 0.0, 3.0, 0.0),
        Vec4::new(1.0, 1.0, 1.0, 1.0),
    );

    group.bench_function("nimbus-math", |bencher| {
        bencher.iter(|| black_box(m).inverse());
    });

    let gm = GlamMat4::from_cols_array(&[
        1.0, 0.0, 0.0, 0.0, 0.0, 2.0, 0.0, 0.0, 0.0, 0.0, 3.0, 0.0, 1.0, 1.0, 1.0, 1.0,
    ]);

    group.bench_function("glam", |bencher| {
        bencher.iter(|| black_box(gm).inverse());
    });

    group.finish();
}

fn bench_mat4_transpose(c: &mut Criterion) {
    let mut group = c.benchmark_group("Mat4 Transpose");

    let m = Mat4::from_cols(
        Vec4::new(1.0, 2.0, 3.0, 4.0),
        Vec4::new(5.0, 6.0, 7.0, 8.0),
        Vec4::new(9.0, 10.0, 11.0, 12.0),
        Vec4::new(13.0, 14.0, 15.0, 16.0),
    );

    group.bench_function("nimbus-math", |bencher| {
        bencher.iter(|| black_box(m).transpose());
    });

    let gm = GlamMat4::from_cols_array(&[
        1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0,
    ]);

    group.bench_function("glam", |bencher| {
        bencher.iter(|| black_box(gm).transpose());
    });

    group.finish();
}

fn bench_mat4_from_trs(c: &mut Criterion) {
    let mut group = c.benchmark_group("Mat4 from_trs");

    let translation = Vec3::new(1.0, 2.0, 3.0);
    let rotation = Quat::from_angle_axis(PI / 4.0, Vec3::new(1.0, 1.0, 0.0).normalize());
    let scale = Vec3::new(2.0, 3.0, 4.0);

    group.bench_function("nimbus-math", |bencher| {
        bencher.iter(|| {
            Mat4::from_trs(
                black_box(translation),
                black_box(rotation),
                black_box(scale),
            )
        });
    });

    let gtranslation = GlamVec3::new(1.0, 2.0, 3.0);
    let grotation = GlamQuat::from_axis_angle(GlamVec3::new(1.0, 1.0, 0.0).normalize(), PI / 4.0);
    let gscale = GlamVec3::new(2.0, 3.0, 4.0);

    group.bench_function("glam", |bencher| {
        bencher.iter(|| {
            GlamMat4::from_scale_rotation_translation(
                black_box(gscale),
                black_box(grotation),
                black_box(gtranslation),
            )
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_mat4_mat4_mul,
    bench_mat4_vec4_mul,
    bench_mat4_determinant,
    bench_mat4_inverse,
    bench_mat4_transpose,
    bench_mat4_from_trs
);
criterion_main!(benches);
