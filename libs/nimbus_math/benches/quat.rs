use criterion::{Criterion, black_box, criterion_group, criterion_main};
use glam::Quat as GlamQuat;
use nimbus_math::{Vec3, quat::Quat};
use std::f32::consts::PI;

fn bench_quat_mul(c: &mut Criterion) {
    let mut group = c.benchmark_group("Quat * Quat");

    let q1 = Quat::from_angle_axis(PI / 4.0, Vec3::new(1.0, 0.0, 0.0));
    let q2 = Quat::from_angle_axis(PI / 6.0, Vec3::new(0.0, 1.0, 0.0));

    group.bench_function("nimbus-math", |bencher| {
        bencher.iter(|| black_box(q1) * black_box(q2));
    });

    let gq1 = GlamQuat::from_axis_angle(glam::Vec3::X, PI / 4.0);
    let gq2 = GlamQuat::from_axis_angle(glam::Vec3::Y, PI / 6.0);

    group.bench_function("glam", |bencher| {
        bencher.iter(|| black_box(gq1) * black_box(gq2));
    });

    group.finish();
}

fn bench_quat_from_angle_axis(c: &mut Criterion) {
    let mut group = c.benchmark_group("Quat from_angle_axis");

    let axis = Vec3::new(1.0, 2.0, 3.0).normalize();
    let angle = PI / 3.0;

    group.bench_function("nimbus-math", |bencher| {
        bencher.iter(|| Quat::from_angle_axis(black_box(angle), black_box(axis)));
    });

    let gaxis = glam::Vec3::new(1.0, 2.0, 3.0).normalize();

    group.bench_function("glam", |bencher| {
        bencher.iter(|| GlamQuat::from_axis_angle(black_box(gaxis), black_box(angle)));
    });

    group.finish();
}

fn bench_quat_from_angle_axis_unsafe(c: &mut Criterion) {
    let mut group = c.benchmark_group("Quat from_angle_axis_unsafe");

    let axis = Vec3::new(1.0, 2.0, 3.0).normalize();
    let angle = PI / 3.0;

    group.bench_function("nimbus-math", |bencher| {
        bencher.iter(|| unsafe { Quat::from_angle_axis_unsafe(black_box(angle), black_box(axis)) });
    });

    let gaxis = glam::Vec3::new(1.0, 2.0, 3.0).normalize();

    group.bench_function("glam", |bencher| {
        bencher.iter(|| GlamQuat::from_axis_angle(black_box(gaxis), black_box(angle)));
    });

    group.finish();
}

fn bench_quat_from_rotation_x(c: &mut Criterion) {
    let mut group = c.benchmark_group("Quat from_rotation_x");

    let angle = PI / 4.0;

    group.bench_function("nimbus-math", |bencher| {
        bencher.iter(|| Quat::from_rotation_x(black_box(angle)));
    });

    group.bench_function("glam", |bencher| {
        bencher.iter(|| GlamQuat::from_axis_angle(glam::Vec3::X, black_box(angle)));
    });

    group.finish();
}

fn bench_quat_from_rotation_y(c: &mut Criterion) {
    let mut group = c.benchmark_group("Quat from_rotation_y");

    let angle = PI / 4.0;

    group.bench_function("nimbus-math", |bencher| {
        bencher.iter(|| Quat::from_rotation_y(black_box(angle)));
    });

    group.bench_function("glam", |bencher| {
        bencher.iter(|| GlamQuat::from_axis_angle(glam::Vec3::Y, black_box(angle)));
    });

    group.finish();
}

fn bench_quat_from_rotation_z(c: &mut Criterion) {
    let mut group = c.benchmark_group("Quat from_rotation_z");

    let angle = PI / 4.0;

    group.bench_function("nimbus-math", |bencher| {
        bencher.iter(|| Quat::from_rotation_z(black_box(angle)));
    });

    group.bench_function("glam", |bencher| {
        bencher.iter(|| GlamQuat::from_axis_angle(glam::Vec3::Z, black_box(angle)));
    });

    group.finish();
}

fn bench_quat_from_euler_angles(c: &mut Criterion) {
    let mut group = c.benchmark_group("Quat from_euler_angles");

    let x = PI / 6.0;
    let y = PI / 4.0;
    let z = PI / 3.0;

    group.bench_function("nimbus-math", |bencher| {
        bencher.iter(|| Quat::from_euler_angles(black_box(x), black_box(y), black_box(z)));
    });

    group.bench_function("glam", |bencher| {
        bencher.iter(|| {
            GlamQuat::from_euler(
                glam::EulerRot::XYZ,
                black_box(x),
                black_box(y),
                black_box(z),
            )
        });
    });

    group.finish();
}

fn bench_quat_inverse(c: &mut Criterion) {
    let mut group = c.benchmark_group("Quat inverse");

    let q = Quat::from_angle_axis(PI / 4.0, Vec3::new(1.0, 2.0, 3.0).normalize());

    group.bench_function("nimbus-math", |bencher| {
        bencher.iter(|| black_box(q).inverse());
    });

    let gq = GlamQuat::from_axis_angle(glam::Vec3::new(1.0, 2.0, 3.0).normalize(), PI / 4.0);

    group.bench_function("glam", |bencher| {
        bencher.iter(|| black_box(gq).inverse());
    });

    group.finish();
}

fn bench_quat_conjugate(c: &mut Criterion) {
    let mut group = c.benchmark_group("Quat conjugate");

    let q = Quat::from_angle_axis(PI / 4.0, Vec3::new(1.0, 2.0, 3.0).normalize());

    group.bench_function("nimbus-math", |bencher| {
        bencher.iter(|| black_box(q).conjugate());
    });

    let gq = GlamQuat::from_axis_angle(glam::Vec3::new(1.0, 2.0, 3.0).normalize(), PI / 4.0);

    group.bench_function("glam", |bencher| {
        bencher.iter(|| black_box(gq).conjugate());
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_quat_mul,
    bench_quat_from_angle_axis,
    bench_quat_from_angle_axis_unsafe,
    bench_quat_from_rotation_x,
    bench_quat_from_rotation_y,
    bench_quat_from_rotation_z,
    bench_quat_from_euler_angles,
    bench_quat_inverse,
    bench_quat_conjugate
);
criterion_main!(benches);
