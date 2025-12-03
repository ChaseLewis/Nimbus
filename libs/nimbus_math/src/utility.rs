// Utility macros and helpers for SIMD operations

/// Helper macro for creating _mm_shuffle_ps masks
///
/// Usage: `shuffle_mask!(lane0, lane1, lane2, lane3)`
///
/// Each lane value (0-3) specifies which source lane to use for the destination lane.
/// For `_mm_shuffle_ps(a, b, mask)`: lanes 0-1 come from 'a', lanes 2-3 come from 'b'.
///
/// # Example
/// ```ignore
/// // Swap first two lanes: (y, x, z, w)
/// shuffle_mask!(1, 0, 2, 3);
///
/// // Reverse all lanes: (w, z, y, x)
/// shuffle_mask!(3, 2, 1, 0);
/// ```
#[macro_export]
macro_rules! shuffle_mask {
    ($lane0:expr, $lane1:expr, $lane2:expr, $lane3:expr) => {
        (($lane0 & 0b11) | (($lane1 & 0b11) << 2) | (($lane2 & 0b11) << 4) | (($lane3 & 0b11) << 6))
    };
}
