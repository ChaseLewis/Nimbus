# nimbus-math

High-performance vector math library with SSE optimizations.

## Features

- `Vec2`, `Vec3`, and `Vec4` types with comprehensive math operations
- SSE4.1 optimizations for `Vec4` on x86_64 platforms
- No external dependencies (except for benchmarks)

## SSE4.1 Support

This library uses SSE4.1 instructions by default for optimal performance on x86_64 platforms. The library is configured to enable SSE4.1 via `.cargo/config.toml` for local builds.

### When Using as a Dependency

If you're using this library as a dependency in your project, you can enable SSE4.1 by:

1. **Setting RUSTFLAGS** (recommended):
   ```bash
   export RUSTFLAGS="-C target-feature=+sse4.1"
   ```

2. **Or adding to your project's `.cargo/config.toml`**:
   ```toml
   [build]
   rustflags = ["-C", "target-feature=+sse4.1"]
   ```

### Compatibility

SSE4.1 is supported on all modern x86_64 CPUs (Intel Core 2 and later, AMD K10 and later). If you need to support older CPUs, the library will automatically fall back to scalar implementations.

## Usage

```rust
use nimbus_math::prelude::*;

let v1 = Vec4::new(1.0, 2.0, 3.0, 4.0);
let v2 = Vec4::new(5.0, 6.0, 7.0, 8.0);
let result = v1 + v2;
```

## Benchmarks

Run benchmarks with:
```bash
cargo bench
```

Benchmarks compare `nimbus-math` against the `glam` library.

