#[macro_use]
pub mod utility;
pub mod mat4;
pub mod prelude;
pub mod quat;
pub mod quatv;
pub mod vec2;
pub mod vec3;
pub mod vec4;

pub use prelude::*;

// Re-export benchmarking helpers
#[cfg(all(target_arch = "x86_64", target_feature = "sse"))]
pub use vec4::round_sse;
