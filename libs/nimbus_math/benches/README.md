# Benchmark Suite

This directory contains benchmarks comparing `nimbus-math` vector types against the `glam` library.

## Running Benchmarks

To run all benchmarks:
```bash
cargo bench
```

To run a specific benchmark:
```bash
cargo bench --bench vec2
cargo bench --bench vec3
cargo bench --bench vec4
```

## Benchmark Results

Results are saved to `target/criterion/` with HTML reports that can be viewed in a browser.

## Benchmarked Operations

Each vector type (Vec2, Vec3, Vec4) includes benchmarks for:
- Addition (`+`)
- Subtraction (`-`)
- Scalar multiplication (`*`)
- Dot product
- Length
- Length squared
- Normalize
- Cross product (Vec3 only)
- Floor
- Ceil
- Round
- Trunc
- Min F32 (component-wise min with scalar)
- Max F32 (component-wise max with scalar)
- Clamp F32 (component-wise clamp with scalar range)
- Remainder (`%`)

## SSE4.1 Configuration

Benchmarks are configured to use SSE4.1 features via `.cargo/config.toml`. This enables optimized implementations for:
- `floor`, `ceil`, `round`, and `trunc` operations in Vec4 (using `_mm_floor_ps`, `_mm_ceil_ps`, `_mm_round_ps`)

The configuration ensures that all benchmarks run with the same CPU feature set for fair comparison.

## Dependencies

- `glam` - Only included as a dev-dependency for benchmarking
- `criterion` - Benchmarking framework

These dependencies are **not** required when using `nimbus-math` as a library dependency.

