use core::arch::x86_64::*;
use core::fmt;
use core::ops::{
    Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Rem, RemAssign, Sub, SubAssign,
};

// Constants for SSE rounding (DirectXMath style, used by glam)
const PS_SIGN_MASK: __m128 = unsafe { core::mem::transmute([0x80000000u32; 4]) }; // Sign bit mask (-0.0f32)
const PS_NO_FRACTION: __m128 = unsafe { core::mem::transmute([0x4B000000u32; 4]) }; // 8388608.0
const PS_INV_SIGN_MASK: __m128 = unsafe { core::mem::transmute([0x7FFFFFFFu32; 4]) };

// Union for casting between __m128 and [f32; 4]
#[repr(C)]
union UnionCast {
    arr: [f32; 4],
    vec: __m128,
}

#[repr(transparent)]
#[derive(Copy, Clone, Debug)]
pub struct Vec4(
    #[cfg(all(target_arch = "x86_64", target_feature = "sse"))] pub(crate) __m128,
    #[cfg(not(all(target_arch = "x86_64", target_feature = "sse")))] pub(crate) [f32; 4],
);

impl PartialEq for Vec4 {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        #[cfg(all(target_arch = "x86_64", target_feature = "sse"))]
        {
            unsafe { _mm_movemask_ps(_mm_cmpeq_ps(self.0, other.0)) == 0xF }
        }
        #[cfg(not(all(target_arch = "x86_64", target_feature = "sse")))]
        {
            self.0[0] == other.0[0]
                && self.0[1] == other.0[1]
                && self.0[2] == other.0[2]
                && self.0[3] == other.0[3]
        }
    }
}

impl Default for Vec4 {
    #[inline]
    fn default() -> Self {
        Self::zero()
    }
}

impl Vec4 {
    #[inline(always)]
    #[must_use]
    pub const fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        #[cfg(all(target_arch = "x86_64", target_feature = "sse"))]
        {
            unsafe { Self(UnionCast { arr: [x, y, z, w] }.vec) }
        }
        #[cfg(not(all(target_arch = "x86_64", target_feature = "sse")))]
        {
            Self([x, y, z, w])
        }
    }

    #[inline(always)]
    #[must_use]
    pub const fn zero() -> Self {
        Self::new(0.0, 0.0, 0.0, 0.0)
    }

    #[inline(always)]
    #[must_use]
    pub const fn one() -> Self {
        Self::new(1.0, 1.0, 1.0, 1.0)
    }

    #[inline(always)]
    #[must_use]
    pub const fn splat(value: f32) -> Self {
        #[cfg(all(target_arch = "x86_64", target_feature = "sse"))]
        {
            unsafe { Self(UnionCast { arr: [value; 4] }.vec) }
        }
        #[cfg(not(all(target_arch = "x86_64", target_feature = "sse")))]
        {
            Self([value; 4])
        }
    }

    #[inline(always)]
    #[must_use]
    pub const fn from_array(arr: [f32; 4]) -> Self {
        #[cfg(all(target_arch = "x86_64", target_feature = "sse"))]
        {
            unsafe { Self(UnionCast { arr }.vec) }
        }
        #[cfg(not(all(target_arch = "x86_64", target_feature = "sse")))]
        {
            Self(arr)
        }
    }

    /// Creates a vector from a slice.
    /// Panics if the slice has fewer than 4 elements.
    #[inline]
    #[must_use]
    pub fn from_slice(slice: &[f32]) -> Self {
        Self::new(slice[0], slice[1], slice[2], slice[3])
    }

    // Accessors for individual components
    #[inline(always)]
    pub fn x(&self) -> f32 {
        #[cfg(all(target_arch = "x86_64", target_feature = "sse"))]
        {
            unsafe { UnionCast { vec: self.0 }.arr[0] }
        }
        #[cfg(not(all(target_arch = "x86_64", target_feature = "sse")))]
        {
            self.0[0]
        }
    }

    #[inline(always)]
    pub fn y(&self) -> f32 {
        #[cfg(all(target_arch = "x86_64", target_feature = "sse"))]
        {
            unsafe { UnionCast { vec: self.0 }.arr[1] }
        }
        #[cfg(not(all(target_arch = "x86_64", target_feature = "sse")))]
        {
            self.0[1]
        }
    }

    #[inline(always)]
    pub fn z(&self) -> f32 {
        #[cfg(all(target_arch = "x86_64", target_feature = "sse"))]
        {
            unsafe { UnionCast { vec: self.0 }.arr[2] }
        }
        #[cfg(not(all(target_arch = "x86_64", target_feature = "sse")))]
        {
            self.0[2]
        }
    }

    #[inline(always)]
    pub fn w(&self) -> f32 {
        #[cfg(all(target_arch = "x86_64", target_feature = "sse"))]
        {
            unsafe { UnionCast { vec: self.0 }.arr[3] }
        }
        #[cfg(not(all(target_arch = "x86_64", target_feature = "sse")))]
        {
            self.0[3]
        }
    }

    #[inline]
    pub fn length_squared(self) -> f32 {
        #[cfg(all(target_arch = "x86_64", target_feature = "sse"))]
        {
            unsafe {
                let mul = _mm_mul_ps(self.0, self.0);
                // Horizontal add: (x+y, x+y, z+w, z+w) then (x+y+z+w, x+y+z+w, x+y+z+w, x+y+z+w)
                let sum1 = _mm_add_ps(mul, _mm_shuffle_ps(mul, mul, shuffle_mask!(1, 0, 3, 2)));
                let sum2 = _mm_add_ps(sum1, _mm_shuffle_ps(sum1, sum1, shuffle_mask!(2, 3, 0, 1)));
                _mm_cvtss_f32(sum2)
            }
        }
        #[cfg(not(all(target_arch = "x86_64", target_feature = "sse")))]
        {
            let [x, y, z, w] = self.0;
            x * x + y * y + z * z + w * w
        }
    }

    #[inline]
    pub fn length(self) -> f32 {
        #[cfg(all(target_arch = "x86_64", target_feature = "sse"))]
        {
            unsafe {
                let mul = _mm_mul_ps(self.0, self.0);
                // Horizontal add: (x+y, x+y, z+w, z+w) then (x+y+z+w, x+y+z+w, x+y+z+w, x+y+z+w)
                let sum1 = _mm_add_ps(mul, _mm_shuffle_ps(mul, mul, shuffle_mask!(1, 0, 3, 2)));
                let sum2 = _mm_add_ps(sum1, _mm_shuffle_ps(sum1, sum1, shuffle_mask!(2, 3, 0, 1)));
                // Use _mm_sqrt_ss directly on the sum2 result
                _mm_cvtss_f32(_mm_sqrt_ss(sum2))
            }
        }
        #[cfg(not(all(target_arch = "x86_64", target_feature = "sse")))]
        {
            self.length_squared().sqrt()
        }
    }

    /// Normalizes the vector. Assumes the vector is non-zero.
    /// Use `normalize_or_zero()` for safe normalization that handles zero vectors.
    #[inline]
    pub fn normalize(self) -> Self {
        let inv_len = self.length().recip();
        self * inv_len
    }

    /// Normalizes the vector and returns both the normalized vector and the original length.
    /// Assumes the vector is non-zero.
    /// This is more efficient than calling `normalize()` and `length()` separately.
    #[inline]
    pub fn normalize_and_length(self) -> (Self, f32) {
        let len = self.length();
        let inv_len = len.recip();
        (self * inv_len, len)
    }

    /// Normalizes the vector, returning zero if the length is zero.
    #[inline]
    pub fn normalize_or_zero(self) -> Self {
        let len_sq = self.length_squared();
        if len_sq > 0.0 {
            let inv_len = len_sq.sqrt().recip();
            self * inv_len
        } else {
            Self::zero()
        }
    }

    #[inline(always)]
    pub fn dot(self, other: Self) -> f32 {
        #[cfg(all(target_arch = "x86_64", target_feature = "sse"))]
        {
            unsafe {
                let mul = _mm_mul_ps(self.0, other.0);
                // Horizontal add: (x+y, x+y, z+w, z+w) then (x+y+z+w, x+y+z+w, x+y+z+w, x+y+z+w)
                let sum1 = _mm_add_ps(mul, _mm_shuffle_ps(mul, mul, shuffle_mask!(1, 0, 3, 2)));
                let sum2 = _mm_add_ps(sum1, _mm_shuffle_ps(sum1, sum1, shuffle_mask!(2, 3, 0, 1)));
                _mm_cvtss_f32(sum2)
            }
        }
        #[cfg(not(all(target_arch = "x86_64", target_feature = "sse")))]
        {
            let [x1, y1, z1, w1] = self.0;
            let [x2, y2, z2, w2] = other.0;
            x1 * x2 + y1 * y2 + z1 * z2 + w1 * w2
        }
    }

    /// Multiplies two vectors element-wise.
    #[inline]
    pub fn mul_element_wise(self, other: Self) -> Self {
        #[cfg(all(target_arch = "x86_64", target_feature = "sse"))]
        {
            unsafe { Self(_mm_mul_ps(self.0, other.0)) }
        }
        #[cfg(not(all(target_arch = "x86_64", target_feature = "sse")))]
        {
            let [x1, y1, z1, w1] = self.0;
            let [x2, y2, z2, w2] = other.0;
            Self::new(x1 * x2, y1 * y2, z1 * z2, w1 * w2)
        }
    }

    #[inline]
    pub fn distance_squared(self, other: Self) -> f32 {
        (self - other).length_squared()
    }

    #[inline]
    pub fn distance(self, other: Self) -> f32 {
        (self - other).length()
    }

    /// Linear interpolation using the exact formula: `self * (1.0 - t) + other * t`
    #[inline]
    pub fn lerp(self, other: Self, t: f32) -> Self {
        self * (1.0 - t) + other * t
    }

    /// Fast linear interpolation using: `self + (other - self) * t`
    /// This version may have slightly less precision but is faster.
    #[inline]
    pub fn lerp_fast(self, other: Self, t: f32) -> Self {
        self + (other - self) * t
    }

    #[inline]
    pub fn abs(self) -> Self {
        #[cfg(all(target_arch = "x86_64", target_feature = "sse"))]
        {
            unsafe {
                // Use bitwise AND to clear sign bit
                let mask_inv = _mm_castsi128_ps(_mm_set1_epi32(0x7FFFFFFF));
                Self(_mm_and_ps(self.0, mask_inv))
            }
        }
        #[cfg(not(all(target_arch = "x86_64", target_feature = "sse")))]
        {
            let [x, y, z, w] = self.0;
            Self::new(x.abs(), y.abs(), z.abs(), w.abs())
        }
    }

    #[inline]
    pub fn min(self, other: Self) -> Self {
        #[cfg(all(target_arch = "x86_64", target_feature = "sse"))]
        {
            unsafe { Self(_mm_min_ps(self.0, other.0)) }
        }
        #[cfg(not(all(target_arch = "x86_64", target_feature = "sse")))]
        {
            let [x1, y1, z1, w1] = self.0;
            let [x2, y2, z2, w2] = other.0;
            Self::new(x1.min(x2), y1.min(y2), z1.min(z2), w1.min(w2))
        }
    }

    #[inline]
    pub fn max(self, other: Self) -> Self {
        #[cfg(all(target_arch = "x86_64", target_feature = "sse"))]
        {
            unsafe { Self(_mm_max_ps(self.0, other.0)) }
        }
        #[cfg(not(all(target_arch = "x86_64", target_feature = "sse")))]
        {
            let [x1, y1, z1, w1] = self.0;
            let [x2, y2, z2, w2] = other.0;
            Self::new(x1.max(x2), y1.max(y2), z1.max(z2), w1.max(w2))
        }
    }

    #[inline]
    pub fn clamp(self, min: Self, max: Self) -> Self {
        self.max(min).min(max)
    }

    #[inline]
    pub fn min_f32(self, scalar: f32) -> Self {
        #[cfg(all(target_arch = "x86_64", target_feature = "sse"))]
        {
            unsafe {
                let scalar_vec = _mm_set1_ps(scalar);
                Self(_mm_min_ps(self.0, scalar_vec))
            }
        }
        #[cfg(not(all(target_arch = "x86_64", target_feature = "sse")))]
        {
            let [x, y, z, w] = self.0;
            Self::new(x.min(scalar), y.min(scalar), z.min(scalar), w.min(scalar))
        }
    }

    #[inline]
    pub fn max_f32(self, scalar: f32) -> Self {
        #[cfg(all(target_arch = "x86_64", target_feature = "sse"))]
        {
            unsafe {
                let scalar_vec = _mm_set1_ps(scalar);
                Self(_mm_max_ps(self.0, scalar_vec))
            }
        }
        #[cfg(not(all(target_arch = "x86_64", target_feature = "sse")))]
        {
            let [x, y, z, w] = self.0;
            Self::new(x.max(scalar), y.max(scalar), z.max(scalar), w.max(scalar))
        }
    }

    #[inline]
    pub fn clamp_f32(self, min: f32, max: f32) -> Self {
        self.max_f32(min).min_f32(max)
    }

    #[inline]
    pub fn signum(self) -> Self {
        #[cfg(all(target_arch = "x86_64", target_feature = "sse"))]
        {
            unsafe {
                // Use scalar for signum to match Rust's exact behavior
                let arr = UnionCast { vec: self.0 }.arr;
                Self::new(
                    arr[0].signum(),
                    arr[1].signum(),
                    arr[2].signum(),
                    arr[3].signum(),
                )
            }
        }
        #[cfg(not(all(target_arch = "x86_64", target_feature = "sse")))]
        {
            let [x, y, z, w] = self.0;
            Self::new(x.signum(), y.signum(), z.signum(), w.signum())
        }
    }

    #[inline]
    pub fn copysign(self, sign: Self) -> Self {
        #[cfg(all(target_arch = "x86_64", target_feature = "sse"))]
        {
            unsafe {
                // Extract sign bits from sign vector and apply to self
                let sign_mask = _mm_set1_ps(-0.0f32); // -0.0 has sign bit set
                let sign_bits = _mm_and_ps(sign.0, sign_mask);
                let abs_self = _mm_andnot_ps(sign_mask, self.0);
                Self(_mm_or_ps(abs_self, sign_bits))
            }
        }
        #[cfg(not(all(target_arch = "x86_64", target_feature = "sse")))]
        {
            let [x1, y1, z1, w1] = self.0;
            let [x2, y2, z2, w2] = sign.0;
            Self::new(
                x1.copysign(x2),
                y1.copysign(y2),
                z1.copysign(z2),
                w1.copysign(w2),
            )
        }
    }

    #[inline]
    pub fn trunc(self) -> Self {
        #[cfg(all(target_arch = "x86_64", target_feature = "sse4.1"))]
        {
            unsafe {
                // _MM_FROUND_TO_ZERO | _MM_FROUND_NO_EXC = 0x03
                Self(_mm_round_ps(self.0, _MM_FROUND_TO_ZERO | _MM_FROUND_NO_EXC))
            }
        }
        #[cfg(all(
            target_arch = "x86_64",
            target_feature = "sse",
            not(target_feature = "sse4.1")
        ))]
        {
            unsafe {
                // Fallback to scalar for SSE without SSE4.1
                let arr = UnionCast { vec: self.0 }.arr;
                Self::new(
                    arr[0].trunc(),
                    arr[1].trunc(),
                    arr[2].trunc(),
                    arr[3].trunc(),
                )
            }
        }
        #[cfg(not(all(target_arch = "x86_64", target_feature = "sse")))]
        {
            let [x, y, z, w] = self.0;
            Self::new(x.trunc(), y.trunc(), z.trunc(), w.trunc())
        }
    }

    #[inline]
    pub fn floor(self) -> Self {
        #[cfg(all(target_arch = "x86_64", target_feature = "sse4.1"))]
        {
            unsafe { Self(_mm_floor_ps(self.0)) }
        }
        #[cfg(all(
            target_arch = "x86_64",
            target_feature = "sse",
            not(target_feature = "sse4.1")
        ))]
        {
            unsafe {
                // Fallback to scalar for SSE without SSE4.1
                let arr = UnionCast { vec: self.0 }.arr;
                Self::new(
                    arr[0].floor(),
                    arr[1].floor(),
                    arr[2].floor(),
                    arr[3].floor(),
                )
            }
        }
        #[cfg(not(all(target_arch = "x86_64", target_feature = "sse")))]
        {
            let [x, y, z, w] = self.0;
            Self::new(x.floor(), y.floor(), z.floor(), w.floor())
        }
    }

    #[inline]
    pub fn ceil(self) -> Self {
        #[cfg(all(target_arch = "x86_64", target_feature = "sse4.1"))]
        {
            unsafe { Self(_mm_ceil_ps(self.0)) }
        }
        #[cfg(all(
            target_arch = "x86_64",
            target_feature = "sse",
            not(target_feature = "sse4.1")
        ))]
        {
            unsafe {
                // Fallback to scalar for SSE without SSE4.1
                let arr = UnionCast { vec: self.0 }.arr;
                Self::new(arr[0].ceil(), arr[1].ceil(), arr[2].ceil(), arr[3].ceil())
            }
        }
        #[cfg(not(all(target_arch = "x86_64", target_feature = "sse")))]
        {
            let [x, y, z, w] = self.0;
            Self::new(x.ceil(), y.ceil(), z.ceil(), w.ceil())
        }
    }

    #[inline]
    pub fn round(self) -> Self {
        #[cfg(all(target_arch = "x86_64", target_feature = "sse4.1"))]
        {
            unsafe {
                // Implement "round half away from zero" using SSE4.1:
                // Add 0.5 to positive values, subtract 0.5 from negative values, then truncate
                // Use copysign-style approach: extract sign from self and apply to 0.5
                let half = _mm_set1_ps(0.5);

                // Extract sign bits from self and apply to 0.5 (makes it -0.5 for negative, 0.5 for positive)
                let sign_bits = _mm_and_ps(self.0, PS_SIGN_MASK);
                let signed_half = _mm_or_ps(half, sign_bits);

                // Add signed half and truncate (round toward zero)
                let adjusted = _mm_add_ps(self.0, signed_half);
                Self(_mm_round_ps(
                    adjusted,
                    _MM_FROUND_TO_ZERO | _MM_FROUND_NO_EXC,
                ))
            }
        }
        #[cfg(all(
            target_arch = "x86_64",
            target_feature = "sse",
            not(target_feature = "sse4.1")
        ))]
        {
            unsafe {
                // Glam's SSE rounding approach (DirectXMath style)
                // Based on https://github.com/microsoft/DirectXMath XMVectorRound
                let sign = _mm_and_ps(self.0, PS_SIGN_MASK);
                let s_magic = _mm_or_ps(PS_NO_FRACTION, sign);
                let r1 = _mm_add_ps(self.0, s_magic);
                let r1 = _mm_sub_ps(r1, s_magic);
                let r2 = _mm_and_ps(self.0, PS_INV_SIGN_MASK);
                let mask = _mm_cmple_ps(r2, PS_NO_FRACTION);
                let r2 = _mm_andnot_ps(mask, self.0);
                let r1 = _mm_and_ps(r1, mask);
                Self(_mm_xor_ps(r1, r2))
            }
        }
        #[cfg(not(all(target_arch = "x86_64", target_feature = "sse")))]
        {
            let [x, y, z, w] = self.0;
            Self::new(x.round(), y.round(), z.round(), w.round())
        }
    }

    #[inline]
    pub fn to_array(self) -> [f32; 4] {
        #[cfg(all(target_arch = "x86_64", target_feature = "sse"))]
        {
            unsafe { UnionCast { vec: self.0 }.arr }
        }
        #[cfg(not(all(target_arch = "x86_64", target_feature = "sse")))]
        {
            self.0
        }
    }

    /// Returns a slice of the vector components.
    #[inline]
    pub fn as_slice(&self) -> &[f32] {
        #[cfg(all(target_arch = "x86_64", target_feature = "sse"))]
        {
            unsafe {
                let arr = &UnionCast { vec: self.0 }.arr;
                std::slice::from_raw_parts(arr as *const [f32; 4] as *const f32, 4)
            }
        }
        #[cfg(not(all(target_arch = "x86_64", target_feature = "sse")))]
        {
            &self.0
        }
    }

    /// Returns a mutable slice of the vector components.
    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [f32] {
        #[cfg(all(target_arch = "x86_64", target_feature = "sse"))]
        {
            unsafe {
                // Cast the struct to a pointer to the first f32
                std::slice::from_raw_parts_mut(self as *mut Self as *mut f32, 4)
            }
        }
        #[cfg(not(all(target_arch = "x86_64", target_feature = "sse")))]
        {
            &mut self.0
        }
    }

    // Swizzle methods - create new vectors by repeating components
    // Using SSE2 shuffle operations for optimized performance

    /// Returns a Vec4 with all components set to x.
    #[inline]
    pub fn xxxx(self) -> Self {
        #[cfg(all(target_arch = "x86_64", target_feature = "sse"))]
        {
            unsafe {
                // Shuffle x component to all lanes: (x, x, x, x)
                Self(_mm_shuffle_ps(self.0, self.0, shuffle_mask!(0, 0, 0, 0)))
            }
        }
        #[cfg(not(all(target_arch = "x86_64", target_feature = "sse")))]
        {
            let x = self.x();
            Self::new(x, x, x, x)
        }
    }

    /// Returns a Vec4 with all components set to y.
    #[inline]
    pub fn yyyy(self) -> Self {
        #[cfg(all(target_arch = "x86_64", target_feature = "sse"))]
        {
            unsafe {
                // Shuffle y component to all lanes: (y, y, y, y)
                Self(_mm_shuffle_ps(self.0, self.0, shuffle_mask!(1, 1, 1, 1)))
            }
        }
        #[cfg(not(all(target_arch = "x86_64", target_feature = "sse")))]
        {
            let y = self.y();
            Self::new(y, y, y, y)
        }
    }

    /// Returns a Vec4 with all components set to z.
    #[inline]
    pub fn zzzz(self) -> Self {
        #[cfg(all(target_arch = "x86_64", target_feature = "sse"))]
        {
            unsafe {
                // Shuffle z component to all lanes: (z, z, z, z)
                Self(_mm_shuffle_ps(self.0, self.0, shuffle_mask!(2, 2, 2, 2)))
            }
        }
        #[cfg(not(all(target_arch = "x86_64", target_feature = "sse")))]
        {
            let z = self.z();
            Self::new(z, z, z, z)
        }
    }

    /// Returns a Vec4 with all components set to w.
    #[inline]
    pub fn wwww(self) -> Self {
        #[cfg(all(target_arch = "x86_64", target_feature = "sse"))]
        {
            unsafe {
                // Shuffle w component to all lanes: (w, w, w, w)
                Self(_mm_shuffle_ps(self.0, self.0, shuffle_mask!(3, 3, 3, 3)))
            }
        }
        #[cfg(not(all(target_arch = "x86_64", target_feature = "sse")))]
        {
            let w = self.w();
            Self::new(w, w, w, w)
        }
    }

    /// Returns a Vec4 with components (x, y, z, w) - identity operation.
    #[inline]
    pub fn xyzw(self) -> Self {
        self
    }

    /// Returns a Vec4 with components (x, y, w, z).
    #[inline]
    pub fn xywz(self) -> Self {
        #[cfg(all(target_arch = "x86_64", target_feature = "sse"))]
        {
            unsafe { Self(_mm_shuffle_ps(self.0, self.0, shuffle_mask!(0, 1, 3, 2))) }
        }
        #[cfg(not(all(target_arch = "x86_64", target_feature = "sse")))]
        {
            Self::new(self.x(), self.y(), self.w(), self.z())
        }
    }

    /// Returns a Vec4 with components (x, z, y, w).
    #[inline]
    pub fn xzyw(self) -> Self {
        #[cfg(all(target_arch = "x86_64", target_feature = "sse"))]
        {
            unsafe { Self(_mm_shuffle_ps(self.0, self.0, shuffle_mask!(0, 2, 1, 3))) }
        }
        #[cfg(not(all(target_arch = "x86_64", target_feature = "sse")))]
        {
            Self::new(self.x(), self.z(), self.y(), self.w())
        }
    }

    /// Returns a Vec4 with components (x, z, w, y).
    #[inline]
    pub fn xzwy(self) -> Self {
        #[cfg(all(target_arch = "x86_64", target_feature = "sse"))]
        {
            unsafe { Self(_mm_shuffle_ps(self.0, self.0, shuffle_mask!(0, 2, 3, 1))) }
        }
        #[cfg(not(all(target_arch = "x86_64", target_feature = "sse")))]
        {
            Self::new(self.x(), self.z(), self.w(), self.y())
        }
    }

    /// Returns a Vec4 with components (x, w, y, z).
    #[inline]
    pub fn xwyz(self) -> Self {
        #[cfg(all(target_arch = "x86_64", target_feature = "sse"))]
        {
            unsafe { Self(_mm_shuffle_ps(self.0, self.0, shuffle_mask!(0, 3, 1, 2))) }
        }
        #[cfg(not(all(target_arch = "x86_64", target_feature = "sse")))]
        {
            Self::new(self.x(), self.w(), self.y(), self.z())
        }
    }

    /// Returns a Vec4 with components (x, w, z, y).
    #[inline]
    pub fn xwzy(self) -> Self {
        #[cfg(all(target_arch = "x86_64", target_feature = "sse"))]
        {
            unsafe { Self(_mm_shuffle_ps(self.0, self.0, shuffle_mask!(0, 3, 2, 1))) }
        }
        #[cfg(not(all(target_arch = "x86_64", target_feature = "sse")))]
        {
            Self::new(self.x(), self.w(), self.z(), self.y())
        }
    }

    /// Returns a Vec4 with components (y, x, z, w).
    #[inline]
    pub fn yxzw(self) -> Self {
        #[cfg(all(target_arch = "x86_64", target_feature = "sse"))]
        {
            unsafe { Self(_mm_shuffle_ps(self.0, self.0, shuffle_mask!(1, 0, 2, 3))) }
        }
        #[cfg(not(all(target_arch = "x86_64", target_feature = "sse")))]
        {
            Self::new(self.y(), self.x(), self.z(), self.w())
        }
    }

    /// Returns a Vec4 with components (y, x, w, z).
    #[inline]
    pub fn yxwz(self) -> Self {
        #[cfg(all(target_arch = "x86_64", target_feature = "sse"))]
        {
            unsafe { Self(_mm_shuffle_ps(self.0, self.0, shuffle_mask!(1, 0, 3, 2))) }
        }
        #[cfg(not(all(target_arch = "x86_64", target_feature = "sse")))]
        {
            Self::new(self.y(), self.x(), self.w(), self.z())
        }
    }

    /// Returns a Vec4 with components (y, z, x, w).
    #[inline]
    pub fn yzxw(self) -> Self {
        #[cfg(all(target_arch = "x86_64", target_feature = "sse"))]
        {
            unsafe { Self(_mm_shuffle_ps(self.0, self.0, shuffle_mask!(1, 2, 0, 3))) }
        }
        #[cfg(not(all(target_arch = "x86_64", target_feature = "sse")))]
        {
            Self::new(self.y(), self.z(), self.x(), self.w())
        }
    }

    /// Returns a Vec4 with components (y, z, w, x).
    #[inline]
    pub fn yzwx(self) -> Self {
        #[cfg(all(target_arch = "x86_64", target_feature = "sse"))]
        {
            unsafe { Self(_mm_shuffle_ps(self.0, self.0, shuffle_mask!(1, 2, 3, 0))) }
        }
        #[cfg(not(all(target_arch = "x86_64", target_feature = "sse")))]
        {
            Self::new(self.y(), self.z(), self.w(), self.x())
        }
    }

    /// Returns a Vec4 with components (y, w, x, z).
    #[inline]
    pub fn ywxz(self) -> Self {
        #[cfg(all(target_arch = "x86_64", target_feature = "sse"))]
        {
            unsafe { Self(_mm_shuffle_ps(self.0, self.0, shuffle_mask!(1, 3, 0, 2))) }
        }
        #[cfg(not(all(target_arch = "x86_64", target_feature = "sse")))]
        {
            Self::new(self.y(), self.w(), self.x(), self.z())
        }
    }

    /// Returns a Vec4 with components (y, w, z, x).
    #[inline]
    pub fn ywzx(self) -> Self {
        #[cfg(all(target_arch = "x86_64", target_feature = "sse"))]
        {
            unsafe { Self(_mm_shuffle_ps(self.0, self.0, shuffle_mask!(1, 3, 2, 0))) }
        }
        #[cfg(not(all(target_arch = "x86_64", target_feature = "sse")))]
        {
            Self::new(self.y(), self.w(), self.z(), self.x())
        }
    }

    /// Returns a Vec4 with components (z, x, y, w).
    #[inline]
    pub fn zxyw(self) -> Self {
        #[cfg(all(target_arch = "x86_64", target_feature = "sse"))]
        {
            unsafe { Self(_mm_shuffle_ps(self.0, self.0, shuffle_mask!(2, 0, 1, 3))) }
        }
        #[cfg(not(all(target_arch = "x86_64", target_feature = "sse")))]
        {
            Self::new(self.z(), self.x(), self.y(), self.w())
        }
    }

    /// Returns a Vec4 with components (z, x, w, y).
    #[inline]
    pub fn zxwy(self) -> Self {
        #[cfg(all(target_arch = "x86_64", target_feature = "sse"))]
        {
            unsafe { Self(_mm_shuffle_ps(self.0, self.0, shuffle_mask!(2, 0, 3, 1))) }
        }
        #[cfg(not(all(target_arch = "x86_64", target_feature = "sse")))]
        {
            Self::new(self.z(), self.x(), self.w(), self.y())
        }
    }

    /// Returns a Vec4 with components (z, y, x, w).
    #[inline]
    pub fn zyxw(self) -> Self {
        #[cfg(all(target_arch = "x86_64", target_feature = "sse"))]
        {
            unsafe { Self(_mm_shuffle_ps(self.0, self.0, shuffle_mask!(2, 1, 0, 3))) }
        }
        #[cfg(not(all(target_arch = "x86_64", target_feature = "sse")))]
        {
            Self::new(self.z(), self.y(), self.x(), self.w())
        }
    }

    /// Returns a Vec4 with components (z, y, w, x).
    #[inline]
    pub fn zywx(self) -> Self {
        #[cfg(all(target_arch = "x86_64", target_feature = "sse"))]
        {
            unsafe { Self(_mm_shuffle_ps(self.0, self.0, shuffle_mask!(2, 1, 3, 0))) }
        }
        #[cfg(not(all(target_arch = "x86_64", target_feature = "sse")))]
        {
            Self::new(self.z(), self.y(), self.w(), self.x())
        }
    }

    /// Returns a Vec4 with components (z, w, x, y).
    #[inline]
    pub fn zwxy(self) -> Self {
        #[cfg(all(target_arch = "x86_64", target_feature = "sse"))]
        {
            unsafe { Self(_mm_shuffle_ps(self.0, self.0, shuffle_mask!(2, 3, 0, 1))) }
        }
        #[cfg(not(all(target_arch = "x86_64", target_feature = "sse")))]
        {
            Self::new(self.z(), self.w(), self.x(), self.y())
        }
    }

    /// Returns a Vec4 with components (z, w, y, x).
    #[inline]
    pub fn zwyx(self) -> Self {
        #[cfg(all(target_arch = "x86_64", target_feature = "sse"))]
        {
            unsafe { Self(_mm_shuffle_ps(self.0, self.0, shuffle_mask!(2, 3, 1, 0))) }
        }
        #[cfg(not(all(target_arch = "x86_64", target_feature = "sse")))]
        {
            Self::new(self.z(), self.w(), self.y(), self.x())
        }
    }

    /// Returns a Vec4 with components (w, x, y, z).
    #[inline]
    pub fn wxyz(self) -> Self {
        #[cfg(all(target_arch = "x86_64", target_feature = "sse"))]
        {
            unsafe { Self(_mm_shuffle_ps(self.0, self.0, shuffle_mask!(3, 0, 1, 2))) }
        }
        #[cfg(not(all(target_arch = "x86_64", target_feature = "sse")))]
        {
            Self::new(self.w(), self.x(), self.y(), self.z())
        }
    }

    /// Returns a Vec4 with components (w, x, z, y).
    #[inline]
    pub fn wxzy(self) -> Self {
        #[cfg(all(target_arch = "x86_64", target_feature = "sse"))]
        {
            unsafe { Self(_mm_shuffle_ps(self.0, self.0, shuffle_mask!(3, 0, 2, 1))) }
        }
        #[cfg(not(all(target_arch = "x86_64", target_feature = "sse")))]
        {
            Self::new(self.w(), self.x(), self.z(), self.y())
        }
    }

    /// Returns a Vec4 with components (w, y, x, z).
    #[inline]
    pub fn wyxz(self) -> Self {
        #[cfg(all(target_arch = "x86_64", target_feature = "sse"))]
        {
            unsafe { Self(_mm_shuffle_ps(self.0, self.0, shuffle_mask!(3, 1, 0, 2))) }
        }
        #[cfg(not(all(target_arch = "x86_64", target_feature = "sse")))]
        {
            Self::new(self.w(), self.y(), self.x(), self.z())
        }
    }

    /// Returns a Vec4 with components (w, y, z, x).
    #[inline]
    pub fn wyzx(self) -> Self {
        #[cfg(all(target_arch = "x86_64", target_feature = "sse"))]
        {
            unsafe { Self(_mm_shuffle_ps(self.0, self.0, shuffle_mask!(3, 1, 2, 0))) }
        }
        #[cfg(not(all(target_arch = "x86_64", target_feature = "sse")))]
        {
            Self::new(self.w(), self.y(), self.z(), self.x())
        }
    }

    /// Returns a Vec4 with components (w, z, x, y).
    #[inline]
    pub fn wzxy(self) -> Self {
        #[cfg(all(target_arch = "x86_64", target_feature = "sse"))]
        {
            unsafe { Self(_mm_shuffle_ps(self.0, self.0, shuffle_mask!(3, 2, 0, 1))) }
        }
        #[cfg(not(all(target_arch = "x86_64", target_feature = "sse")))]
        {
            Self::new(self.w(), self.z(), self.x(), self.y())
        }
    }

    /// Returns a Vec4 with components (w, z, y, x).
    #[inline]
    pub fn wzyx(self) -> Self {
        #[cfg(all(target_arch = "x86_64", target_feature = "sse"))]
        {
            unsafe { Self(_mm_shuffle_ps(self.0, self.0, shuffle_mask!(3, 2, 1, 0))) }
        }
        #[cfg(not(all(target_arch = "x86_64", target_feature = "sse")))]
        {
            Self::new(self.w(), self.z(), self.y(), self.x())
        }
    }
}

/// Round using SSE (glam/DirectXMath style) - for benchmarking comparison
#[cfg(all(target_arch = "x86_64", target_feature = "sse"))]
#[inline]
pub fn round_sse(v: Vec4) -> Vec4 {
    unsafe {
        // Glam's SSE rounding approach (DirectXMath style)
        let sign = _mm_and_ps(v.0, PS_SIGN_MASK);
        let s_magic = _mm_or_ps(PS_NO_FRACTION, sign);
        let r1 = _mm_add_ps(v.0, s_magic);
        let r1 = _mm_sub_ps(r1, s_magic);
        let r2 = _mm_and_ps(v.0, PS_INV_SIGN_MASK);
        let mask = _mm_cmple_ps(r2, PS_NO_FRACTION);
        let r2 = _mm_andnot_ps(mask, v.0);
        let r1 = _mm_and_ps(r1, mask);
        Vec4(_mm_xor_ps(r1, r2))
    }
}

impl Add for Vec4 {
    type Output = Self;

    #[inline]
    fn add(self, other: Self) -> Self {
        #[cfg(all(target_arch = "x86_64", target_feature = "sse"))]
        {
            unsafe { Self(_mm_add_ps(self.0, other.0)) }
        }
        #[cfg(not(all(target_arch = "x86_64", target_feature = "sse")))]
        {
            let [x1, y1, z1, w1] = self.0;
            let [x2, y2, z2, w2] = other.0;
            Self::new(x1 + x2, y1 + y2, z1 + z2, w1 + w2)
        }
    }
}

impl Sub for Vec4 {
    type Output = Self;

    #[inline]
    fn sub(self, other: Self) -> Self {
        #[cfg(all(target_arch = "x86_64", target_feature = "sse"))]
        {
            unsafe { Self(_mm_sub_ps(self.0, other.0)) }
        }
        #[cfg(not(all(target_arch = "x86_64", target_feature = "sse")))]
        {
            let [x1, y1, z1, w1] = self.0;
            let [x2, y2, z2, w2] = other.0;
            Self::new(x1 - x2, y1 - y2, z1 - z2, w1 - w2)
        }
    }
}

impl Mul<f32> for Vec4 {
    type Output = Self;

    #[inline]
    fn mul(self, scalar: f32) -> Self {
        #[cfg(all(target_arch = "x86_64", target_feature = "sse"))]
        {
            unsafe { Self(_mm_mul_ps(self.0, _mm_set1_ps(scalar))) }
        }
        #[cfg(not(all(target_arch = "x86_64", target_feature = "sse")))]
        {
            let [x, y, z, w] = self.0;
            Self::new(x * scalar, y * scalar, z * scalar, w * scalar)
        }
    }
}

impl Mul<Vec4> for f32 {
    type Output = Vec4;

    #[inline]
    fn mul(self, vec: Vec4) -> Vec4 {
        vec * self
    }
}

impl Mul<Vec4> for Vec4 {
    type Output = Self;

    /// Multiplies two vectors element-wise.
    #[inline]
    fn mul(self, other: Self) -> Self {
        self.mul_element_wise(other)
    }
}

impl Div<f32> for Vec4 {
    type Output = Self;

    #[inline]
    fn div(self, scalar: f32) -> Self {
        self * (1.0 / scalar)
    }
}

impl Neg for Vec4 {
    type Output = Self;

    #[inline]
    fn neg(self) -> Self {
        #[cfg(all(target_arch = "x86_64", target_feature = "sse"))]
        {
            unsafe {
                // Flip sign bit using XOR with -0.0
                let sign_mask = _mm_set1_ps(-0.0f32);
                Self(_mm_xor_ps(self.0, sign_mask))
            }
        }
        #[cfg(not(all(target_arch = "x86_64", target_feature = "sse")))]
        {
            let [x, y, z, w] = self.0;
            Self::new(-x, -y, -z, -w)
        }
    }
}

impl AddAssign for Vec4 {
    #[inline]
    fn add_assign(&mut self, other: Self) {
        *self = *self + other;
    }
}

impl SubAssign for Vec4 {
    #[inline]
    fn sub_assign(&mut self, other: Self) {
        *self = *self - other;
    }
}

impl MulAssign<f32> for Vec4 {
    #[inline]
    fn mul_assign(&mut self, scalar: f32) {
        *self = *self * scalar;
    }
}

impl DivAssign<f32> for Vec4 {
    #[inline]
    fn div_assign(&mut self, scalar: f32) {
        *self = *self / scalar;
    }
}

impl Rem<f32> for Vec4 {
    type Output = Self;

    #[inline]
    fn rem(self, scalar: f32) -> Self {
        #[cfg(all(target_arch = "x86_64", target_feature = "sse"))]
        {
            unsafe {
                let rhs = _mm_set1_ps(scalar);
                let n = _mm_floor_ps(_mm_div_ps(self.0, rhs));
                Self(_mm_sub_ps(self.0, _mm_mul_ps(n, rhs)))
            }
        }
        #[cfg(not(all(target_arch = "x86_64", target_feature = "sse")))]
        {
            let [x, y, z, w] = self.0;
            Self::new(x % scalar, y % scalar, z % scalar, w % scalar)
        }
    }
}

impl RemAssign<f32> for Vec4 {
    #[inline]
    fn rem_assign(&mut self, scalar: f32) {
        *self = *self % scalar;
    }
}

impl fmt::Display for Vec4 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[{}, {}, {}, {}]",
            self.x(),
            self.y(),
            self.z(),
            self.w()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let v = Vec4::new(1.0, 2.0, 3.0, 4.0);
        assert_eq!(v.x(), 1.0);
        assert_eq!(v.y(), 2.0);
        assert_eq!(v.z(), 3.0);
        assert_eq!(v.w(), 4.0);
    }

    #[test]
    fn test_constants() {
        assert_eq!(Vec4::zero(), Vec4::new(0.0, 0.0, 0.0, 0.0));
        assert_eq!(Vec4::one(), Vec4::new(1.0, 1.0, 1.0, 1.0));
        assert_eq!(Vec4::splat(5.0), Vec4::new(5.0, 5.0, 5.0, 5.0));
    }

    #[test]
    fn test_equality() {
        // Zero vector equals itself
        let zero1 = Vec4::zero();
        let zero2 = Vec4::zero();
        assert_eq!(zero1, zero2);
        assert_eq!(zero1, zero1);

        // Same vector equals itself
        let v1 = Vec4::new(1.0, 2.0, 3.0, 4.0);
        let v2 = Vec4::new(1.0, 2.0, 3.0, 4.0);
        assert_eq!(v1, v2);
        assert_eq!(v1, v1);

        // Vectors created the same way are equal
        let v3 = Vec4::splat(5.0);
        let v4 = Vec4::splat(5.0);
        assert_eq!(v3, v4);

        let v5 = Vec4::one();
        let v6 = Vec4::one();
        assert_eq!(v5, v6);

        // Different vectors are not equal
        let v7 = Vec4::new(1.0, 2.0, 3.0, 4.0);
        let v8 = Vec4::new(1.0, 2.0, 3.0, 5.0);
        assert_ne!(v7, v8);

        let v9 = Vec4::new(1.0, 2.0, 3.0, 4.0);
        let v10 = Vec4::new(1.0, 2.0, 4.0, 4.0);
        assert_ne!(v9, v10);

        let v11 = Vec4::new(1.0, 2.0, 3.0, 4.0);
        let v12 = Vec4::new(1.0, 3.0, 3.0, 4.0);
        assert_ne!(v11, v12);

        let v13 = Vec4::new(1.0, 2.0, 3.0, 4.0);
        let v14 = Vec4::new(2.0, 2.0, 3.0, 4.0);
        assert_ne!(v13, v14);

        // Zero is not equal to non-zero
        let zero = Vec4::zero();
        let non_zero = Vec4::new(1.0, 2.0, 3.0, 4.0);
        assert_ne!(zero, non_zero);

        // One is not equal to zero
        let one = Vec4::one();
        assert_ne!(one, zero);

        // Negative vectors
        let v15 = Vec4::new(-1.0, -2.0, -3.0, -4.0);
        let v16 = Vec4::new(-1.0, -2.0, -3.0, -4.0);
        assert_eq!(v15, v16);

        // Mixed positive and negative
        let v17 = Vec4::new(1.0, -2.0, 3.0, -4.0);
        let v18 = Vec4::new(1.0, -2.0, 3.0, -4.0);
        assert_eq!(v17, v18);

        let v19 = Vec4::new(1.0, -2.0, 3.0, -4.0);
        let v20 = Vec4::new(1.0, -2.0, 3.0, 4.0);
        assert_ne!(v19, v20);
    }

    #[test]
    fn test_add() {
        let a = Vec4::new(1.0, 2.0, 3.0, 4.0);
        let b = Vec4::new(5.0, 6.0, 7.0, 8.0);
        let result = a + b;
        assert_eq!(result, Vec4::new(6.0, 8.0, 10.0, 12.0));
    }

    #[test]
    fn test_sub() {
        let a = Vec4::new(5.0, 7.0, 9.0, 11.0);
        let b = Vec4::new(2.0, 3.0, 4.0, 5.0);
        let result = a - b;
        assert_eq!(result, Vec4::new(3.0, 4.0, 5.0, 6.0));
    }

    #[test]
    fn test_mul_scalar() {
        let v = Vec4::new(2.0, 3.0, 4.0, 5.0);
        let result = v * 2.0;
        assert_eq!(result, Vec4::new(4.0, 6.0, 8.0, 10.0));

        let result2 = 2.0 * v;
        assert_eq!(result2, Vec4::new(4.0, 6.0, 8.0, 10.0));
    }

    #[test]
    fn test_div_scalar() {
        let v = Vec4::new(4.0, 6.0, 8.0, 10.0);
        let result = v / 2.0;
        assert_eq!(result, Vec4::new(2.0, 3.0, 4.0, 5.0));
    }

    #[test]
    fn test_neg() {
        let v = Vec4::new(1.0, -2.0, 3.0, -4.0);
        let result = -v;
        assert_eq!(result, Vec4::new(-1.0, 2.0, -3.0, 4.0));
    }

    #[test]
    fn test_assign_ops() {
        let mut v = Vec4::new(1.0, 2.0, 3.0, 4.0);
        v += Vec4::new(3.0, 4.0, 5.0, 6.0);
        assert_eq!(v, Vec4::new(4.0, 6.0, 8.0, 10.0));

        v -= Vec4::new(1.0, 2.0, 3.0, 4.0);
        assert_eq!(v, Vec4::new(3.0, 4.0, 5.0, 6.0));

        v *= 2.0;
        assert_eq!(v, Vec4::new(6.0, 8.0, 10.0, 12.0));

        v /= 2.0;
        assert_eq!(v, Vec4::new(3.0, 4.0, 5.0, 6.0));
    }

    #[test]
    fn test_dot() {
        let a = Vec4::new(1.0, 2.0, 3.0, 4.0);
        let b = Vec4::new(5.0, 6.0, 7.0, 8.0);
        let result = a.dot(b);
        assert_eq!(result, 70.0); // 1*5 + 2*6 + 3*7 + 4*8 = 5 + 12 + 21 + 32 = 70
    }

    #[test]
    fn test_mul_element_wise() {
        let a = Vec4::new(1.0, 2.0, 3.0, 4.0);
        let b = Vec4::new(2.0, 3.0, 4.0, 5.0);
        assert_eq!(a.mul_element_wise(b), Vec4::new(2.0, 6.0, 12.0, 20.0));
    }

    #[test]
    fn test_mul_vec4() {
        // Test that Mul<Vec4> performs element-wise multiplication
        let a = Vec4::new(1.0, 2.0, 3.0, 4.0);
        let b = Vec4::new(2.0, 3.0, 4.0, 5.0);
        let result = a * b;
        assert_eq!(result, Vec4::new(2.0, 6.0, 12.0, 20.0));

        // Test that it matches mul_element_wise
        assert_eq!(result, a.mul_element_wise(b));

        // Test with identity (ones)
        let ones = Vec4::one();
        let v = Vec4::new(2.0, 3.0, 4.0, 5.0);
        assert_eq!(ones * v, v);
        assert_eq!(v * ones, v);
    }

    #[test]
    fn test_length() {
        let v = Vec4::new(2.0, 3.0, 6.0, 6.0);
        assert_eq!(v.length_squared(), 85.0); // 4 + 9 + 36 + 36 = 85
        assert!((v.length() - 85.0_f32.sqrt()).abs() < 1e-6);
    }

    #[test]
    fn test_normalize() {
        let v = Vec4::new(2.0, 0.0, 0.0, 0.0);
        let normalized = v.normalize();
        assert!((normalized.length() - 1.0).abs() < 1e-6);
        assert_eq!(normalized, Vec4::new(1.0, 0.0, 0.0, 0.0));
    }

    #[test]
    fn test_normalize_or_zero() {
        let v = Vec4::new(2.0, 0.0, 0.0, 0.0);
        let normalized = v.normalize_or_zero();
        assert!((normalized.length() - 1.0).abs() < 1e-6);
        assert_eq!(normalized, Vec4::new(1.0, 0.0, 0.0, 0.0));

        let zero = Vec4::zero();
        assert_eq!(zero.normalize_or_zero(), Vec4::zero());
    }

    #[test]
    fn test_distance() {
        let a = Vec4::new(0.0, 0.0, 0.0, 0.0);
        let b = Vec4::new(2.0, 3.0, 6.0, 6.0);
        assert_eq!(a.distance_squared(b), 85.0);
        assert!((a.distance(b) - 85.0_f32.sqrt()).abs() < 1e-6);
    }

    #[test]
    fn test_lerp() {
        let a = Vec4::new(0.0, 0.0, 0.0, 0.0);
        let b = Vec4::new(10.0, 10.0, 10.0, 10.0);
        let result = a.lerp(b, 0.5);
        assert_eq!(result, Vec4::new(5.0, 5.0, 5.0, 5.0));
    }

    #[test]
    fn test_abs() {
        let v = Vec4::new(-1.0, 2.0, -3.0, 4.0);
        assert_eq!(v.abs(), Vec4::new(1.0, 2.0, 3.0, 4.0));
    }

    #[test]
    fn test_min_max() {
        let a = Vec4::new(1.0, 5.0, 3.0, 7.0);
        let b = Vec4::new(3.0, 2.0, 4.0, 5.0);
        assert_eq!(a.min(b), Vec4::new(1.0, 2.0, 3.0, 5.0));
        assert_eq!(a.max(b), Vec4::new(3.0, 5.0, 4.0, 7.0));
    }

    #[test]
    fn test_clamp() {
        let v = Vec4::new(5.0, -5.0, 2.0, 10.0);
        let min = Vec4::new(0.0, 0.0, 0.0, 0.0);
        let max = Vec4::new(3.0, 3.0, 3.0, 3.0);
        assert_eq!(v.clamp(min, max), Vec4::new(3.0, 0.0, 2.0, 3.0));
    }

    #[test]
    fn test_floor_ceil_round() {
        let v = Vec4::new(1.7, -1.3, 2.5, -0.5);
        assert_eq!(v.floor(), Vec4::new(1.0, -2.0, 2.0, -1.0));
        assert_eq!(v.ceil(), Vec4::new(2.0, -1.0, 3.0, 0.0));
        // Rust's round() uses "round half away from zero", so 2.5 -> 3.0, -0.5 -> -1.0
        assert_eq!(v.round(), Vec4::new(2.0, -1.0, 3.0, -1.0));
    }

    #[test]
    fn test_trunc() {
        let v = Vec4::new(1.7, -1.3, 2.5, -0.5);
        assert_eq!(v.trunc(), Vec4::new(1.0, -1.0, 2.0, -0.0));
    }

    #[test]
    fn test_min_f32() {
        let v = Vec4::new(5.0, 2.0, 8.0, 3.0);
        assert_eq!(v.min_f32(4.0), Vec4::new(4.0, 2.0, 4.0, 3.0));
    }

    #[test]
    fn test_max_f32() {
        let v = Vec4::new(5.0, 2.0, 8.0, 3.0);
        assert_eq!(v.max_f32(4.0), Vec4::new(5.0, 4.0, 8.0, 4.0));
    }

    #[test]
    fn test_clamp_f32() {
        let v = Vec4::new(1.0, 5.0, 2.0, 8.0);
        assert_eq!(v.clamp_f32(2.0, 6.0), Vec4::new(2.0, 5.0, 2.0, 6.0));
    }

    #[test]
    fn test_rem() {
        let v = Vec4::new(10.0, 7.0, 15.0, 9.0);
        let result = v % 4.0;
        assert_eq!(result, Vec4::new(2.0, 3.0, 3.0, 1.0));
    }

    #[test]
    fn test_rem_assign() {
        let mut v = Vec4::new(10.0, 7.0, 15.0, 9.0);
        v %= 4.0;
        assert_eq!(v, Vec4::new(2.0, 3.0, 3.0, 1.0));
    }

    #[test]
    fn test_to_array() {
        let v = Vec4::new(1.0, 2.0, 3.0, 4.0);
        assert_eq!(v.to_array(), [1.0, 2.0, 3.0, 4.0]);
    }

    #[test]
    fn test_from_array() {
        let arr = [1.0, 2.0, 3.0, 4.0];
        let v = Vec4::from_array(arr);
        assert_eq!(v.x(), 1.0);
        assert_eq!(v.y(), 2.0);
        assert_eq!(v.z(), 3.0);
        assert_eq!(v.w(), 4.0);
        assert_eq!(v.to_array(), arr);
    }

    #[test]
    fn test_as_slice() {
        let v = Vec4::new(1.0, 2.0, 3.0, 4.0);
        let slice = v.as_slice();
        assert_eq!(slice, &[1.0, 2.0, 3.0, 4.0]);
        assert_eq!(slice.len(), 4);
    }

    #[test]
    fn test_as_mut_slice() {
        let mut v = Vec4::new(1.0, 2.0, 3.0, 4.0);
        let slice = v.as_mut_slice();
        assert_eq!(slice, &[1.0, 2.0, 3.0, 4.0]);

        slice[0] = 5.0;
        slice[1] = 6.0;
        slice[2] = 7.0;
        slice[3] = 8.0;
        assert_eq!(v.x(), 5.0);
        assert_eq!(v.y(), 6.0);
        assert_eq!(v.z(), 7.0);
        assert_eq!(v.w(), 8.0);
    }

    #[test]
    fn test_display() {
        let v = Vec4::new(1.5, 2.5, 3.5, 4.5);
        let s = format!("{}", v);
        assert_eq!(s, "[1.5, 2.5, 3.5, 4.5]");
    }

    #[test]
    fn test_signum() {
        // Test with non-zero values
        let v1 = Vec4::new(5.0, -3.0, 2.0, -1.0);
        let result1 = v1.signum();
        assert_eq!(result1.x(), 1.0);
        assert_eq!(result1.y(), -1.0);
        // z component: 2.0.signum() = 1.0
        assert_eq!(result1.z(), 1.0);
        assert_eq!(result1.w(), -1.0);

        // Test with zero - Rust's signum(0.0) returns 0.0
        let v2 = Vec4::new(5.0, -3.0, 0.0, 0.0);
        // Verify input is actually 0.0
        assert_eq!(v2.z(), 0.0);
        assert_eq!(v2.w(), 0.0);
        let result2 = v2.signum();
        assert_eq!(result2.x(), 1.0);
        assert_eq!(result2.y(), -1.0);
        // Compare against Rust's signum behavior
        assert_eq!(result2.z(), 0.0f32.signum());
        assert_eq!(result2.w(), 0.0f32.signum());
    }

    #[test]
    fn test_copysign() {
        let v = Vec4::new(5.0, 3.0, 2.0, 1.0);
        let sign = Vec4::new(-1.0, 1.0, -1.0, 1.0);
        let result = v.copysign(sign);
        assert_eq!(result, Vec4::new(-5.0, 3.0, -2.0, 1.0));
    }

    #[test]
    fn test_swizzle() {
        let v = Vec4::new(1.0, 2.0, 3.0, 4.0);
        assert_eq!(v.xxxx(), Vec4::new(1.0, 1.0, 1.0, 1.0));
        assert_eq!(v.yyyy(), Vec4::new(2.0, 2.0, 2.0, 2.0));
        assert_eq!(v.zzzz(), Vec4::new(3.0, 3.0, 3.0, 3.0));
        assert_eq!(v.wwww(), Vec4::new(4.0, 4.0, 4.0, 4.0));
        assert_eq!(v.xyzw(), Vec4::new(1.0, 2.0, 3.0, 4.0));
        assert_eq!(v.wzyx(), Vec4::new(4.0, 3.0, 2.0, 1.0));
        assert_eq!(v.xzyw(), Vec4::new(1.0, 3.0, 2.0, 4.0));
        assert_eq!(v.yxzw(), Vec4::new(2.0, 1.0, 3.0, 4.0));
    }
}
