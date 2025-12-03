use core::fmt;
use core::ops::{
    Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Rem, RemAssign, Sub, SubAssign,
};

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Default for Vec2 {
    #[inline]
    fn default() -> Self {
        Self::zero()
    }
}

impl Vec2 {
    #[inline]
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    #[inline]
    pub const fn zero() -> Self {
        Self { x: 0.0, y: 0.0 }
    }

    #[inline]
    pub const fn one() -> Self {
        Self { x: 1.0, y: 1.0 }
    }

    #[inline]
    pub const fn splat(value: f32) -> Self {
        Self { x: value, y: value }
    }

    #[inline(always)]
    #[must_use]
    pub const fn from_array(arr: [f32; 2]) -> Self {
        Self {
            x: arr[0],
            y: arr[1],
        }
    }

    /// Creates a vector from a slice.
    /// Panics if the slice has fewer than 2 elements.
    #[inline]
    #[must_use]
    pub fn from_slice(slice: &[f32]) -> Self {
        Self::new(slice[0], slice[1])
    }

    // Accessors for individual components
    #[inline(always)]
    pub fn x(&self) -> f32 {
        self.x
    }

    #[inline(always)]
    pub fn y(&self) -> f32 {
        self.y
    }

    #[inline]
    pub fn length_squared(self) -> f32 {
        self.x * self.x + self.y * self.y
    }

    #[inline]
    pub fn length(self) -> f32 {
        self.length_squared().sqrt()
    }

    /// Normalizes the vector. Assumes the vector is non-zero.
    /// Use `normalize_or_zero()` for safe normalization that handles zero vectors.
    #[inline]
    pub fn normalize(self) -> Self {
        let inv_len = self.length().recip();
        Self {
            x: self.x * inv_len,
            y: self.y * inv_len,
        }
    }

    /// Normalizes the vector and returns both the normalized vector and the original length.
    /// Assumes the vector is non-zero.
    /// This is more efficient than calling `normalize()` and `length()` separately.
    #[inline]
    pub fn normalize_and_length(self) -> (Self, f32) {
        let len = self.length();
        let inv_len = len.recip();
        (
            Self {
                x: self.x * inv_len,
                y: self.y * inv_len,
            },
            len,
        )
    }

    /// Normalizes the vector, returning zero if the length is zero.
    #[inline]
    pub fn normalize_or_zero(self) -> Self {
        let len_sq = self.length_squared();
        if len_sq > 0.0 {
            let inv_len = len_sq.sqrt().recip();
            Self {
                x: self.x * inv_len,
                y: self.y * inv_len,
            }
        } else {
            Self::zero()
        }
    }

    #[inline]
    pub fn dot(self, other: Self) -> f32 {
        self.x * other.x + self.y * other.y
    }

    /// Multiplies two vectors element-wise.
    #[inline]
    pub fn mul_element_wise(self, other: Self) -> Self {
        Self {
            x: self.x * other.x,
            y: self.y * other.y,
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
        Self {
            x: self.x.abs(),
            y: self.y.abs(),
        }
    }

    #[inline]
    pub fn min(self, other: Self) -> Self {
        Self {
            x: self.x.min(other.x),
            y: self.y.min(other.y),
        }
    }

    #[inline]
    pub fn max(self, other: Self) -> Self {
        Self {
            x: self.x.max(other.x),
            y: self.y.max(other.y),
        }
    }

    #[inline]
    pub fn clamp(self, min: Self, max: Self) -> Self {
        self.max(min).min(max)
    }

    #[inline]
    pub fn min_f32(self, scalar: f32) -> Self {
        Self {
            x: self.x.min(scalar),
            y: self.y.min(scalar),
        }
    }

    #[inline]
    pub fn max_f32(self, scalar: f32) -> Self {
        Self {
            x: self.x.max(scalar),
            y: self.y.max(scalar),
        }
    }

    #[inline]
    pub fn clamp_f32(self, min: f32, max: f32) -> Self {
        self.max_f32(min).min_f32(max)
    }

    #[inline]
    pub fn signum(self) -> Self {
        Self {
            x: self.x.signum(),
            y: self.y.signum(),
        }
    }

    #[inline]
    pub fn copysign(self, sign: Self) -> Self {
        Self {
            x: self.x.copysign(sign.x),
            y: self.y.copysign(sign.y),
        }
    }

    #[inline]
    pub fn trunc(self) -> Self {
        Self {
            x: self.x.trunc(),
            y: self.y.trunc(),
        }
    }

    #[inline]
    pub fn floor(self) -> Self {
        Self {
            x: self.x.floor(),
            y: self.y.floor(),
        }
    }

    #[inline]
    pub fn ceil(self) -> Self {
        Self {
            x: self.x.ceil(),
            y: self.y.ceil(),
        }
    }

    #[inline]
    pub fn round(self) -> Self {
        Self {
            x: self.x.round(),
            y: self.y.round(),
        }
    }

    #[inline]
    pub fn to_array(self) -> [f32; 2] {
        [self.x, self.y]
    }

    /// Returns a slice of the vector components.
    #[inline]
    pub fn as_slice(&self) -> &[f32] {
        unsafe { std::slice::from_raw_parts(self as *const Self as *const f32, 2) }
    }

    /// Returns a mutable slice of the vector components.
    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [f32] {
        unsafe { std::slice::from_raw_parts_mut(self as *mut Self as *mut f32, 2) }
    }

    // Swizzle methods - create new vectors by repeating components
    /// Returns a Vec2 with both components set to x.
    #[inline]
    pub fn xx(self) -> Self {
        Self {
            x: self.x,
            y: self.x,
        }
    }

    /// Returns a Vec2 with both components set to y.
    #[inline]
    pub fn yy(self) -> Self {
        Self {
            x: self.y,
            y: self.y,
        }
    }

    /// Returns a Vec2 with components (x, y) - identity operation.
    #[inline]
    pub fn xy(self) -> Self {
        self
    }

    /// Returns a Vec2 with components swapped (y, x).
    #[inline]
    pub fn yx(self) -> Self {
        Self {
            x: self.y,
            y: self.x,
        }
    }
}

impl Add for Vec2 {
    type Output = Self;

    #[inline]
    fn add(self, other: Self) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
}

impl Sub for Vec2 {
    type Output = Self;

    #[inline]
    fn sub(self, other: Self) -> Self {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
        }
    }
}

impl Mul<f32> for Vec2 {
    type Output = Self;

    #[inline]
    fn mul(self, scalar: f32) -> Self {
        Self {
            x: self.x * scalar,
            y: self.y * scalar,
        }
    }
}

impl Mul<Vec2> for f32 {
    type Output = Vec2;

    #[inline]
    fn mul(self, vec: Vec2) -> Vec2 {
        vec * self
    }
}

impl Mul<Vec2> for Vec2 {
    type Output = Self;

    /// Multiplies two vectors element-wise.
    #[inline]
    fn mul(self, other: Self) -> Self {
        self.mul_element_wise(other)
    }
}

impl Div<f32> for Vec2 {
    type Output = Self;

    #[inline]
    fn div(self, scalar: f32) -> Self {
        self * (1.0 / scalar)
    }
}

impl Neg for Vec2 {
    type Output = Self;

    #[inline]
    fn neg(self) -> Self {
        Self {
            x: -self.x,
            y: -self.y,
        }
    }
}

impl AddAssign for Vec2 {
    #[inline]
    fn add_assign(&mut self, other: Self) {
        *self = *self + other;
    }
}

impl SubAssign for Vec2 {
    #[inline]
    fn sub_assign(&mut self, other: Self) {
        *self = *self - other;
    }
}

impl MulAssign<f32> for Vec2 {
    #[inline]
    fn mul_assign(&mut self, scalar: f32) {
        *self = *self * scalar;
    }
}

impl DivAssign<f32> for Vec2 {
    #[inline]
    fn div_assign(&mut self, scalar: f32) {
        *self = *self / scalar;
    }
}

impl Rem<f32> for Vec2 {
    type Output = Self;

    #[inline]
    fn rem(self, scalar: f32) -> Self {
        Self {
            x: self.x % scalar,
            y: self.y % scalar,
        }
    }
}

impl RemAssign<f32> for Vec2 {
    #[inline]
    fn rem_assign(&mut self, scalar: f32) {
        *self = *self % scalar;
    }
}

impl fmt::Display for Vec2 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}, {}]", self.x, self.y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let v = Vec2::new(1.0, 2.0);
        assert_eq!(v.x, 1.0);
        assert_eq!(v.y, 2.0);
    }

    #[test]
    fn test_constants() {
        assert_eq!(Vec2::zero(), Vec2::new(0.0, 0.0));
        assert_eq!(Vec2::one(), Vec2::new(1.0, 1.0));
        assert_eq!(Vec2::splat(5.0), Vec2::new(5.0, 5.0));
    }

    #[test]
    fn test_add() {
        let a = Vec2::new(1.0, 2.0);
        let b = Vec2::new(3.0, 4.0);
        let result = a + b;
        assert_eq!(result, Vec2::new(4.0, 6.0));
    }

    #[test]
    fn test_sub() {
        let a = Vec2::new(5.0, 7.0);
        let b = Vec2::new(2.0, 3.0);
        let result = a - b;
        assert_eq!(result, Vec2::new(3.0, 4.0));
    }

    #[test]
    fn test_mul_scalar() {
        let v = Vec2::new(2.0, 3.0);
        let result = v * 2.0;
        assert_eq!(result, Vec2::new(4.0, 6.0));

        let result2 = 2.0 * v;
        assert_eq!(result2, Vec2::new(4.0, 6.0));
    }

    #[test]
    fn test_div_scalar() {
        let v = Vec2::new(4.0, 6.0);
        let result = v / 2.0;
        assert_eq!(result, Vec2::new(2.0, 3.0));
    }

    #[test]
    fn test_neg() {
        let v = Vec2::new(1.0, -2.0);
        let result = -v;
        assert_eq!(result, Vec2::new(-1.0, 2.0));
    }

    #[test]
    fn test_assign_ops() {
        let mut v = Vec2::new(1.0, 2.0);
        v += Vec2::new(3.0, 4.0);
        assert_eq!(v, Vec2::new(4.0, 6.0));

        v -= Vec2::new(1.0, 2.0);
        assert_eq!(v, Vec2::new(3.0, 4.0));

        v *= 2.0;
        assert_eq!(v, Vec2::new(6.0, 8.0));

        v /= 2.0;
        assert_eq!(v, Vec2::new(3.0, 4.0));
    }

    #[test]
    fn test_dot() {
        let a = Vec2::new(1.0, 2.0);
        let b = Vec2::new(3.0, 4.0);
        let result = a.dot(b);
        assert_eq!(result, 11.0); // 1*3 + 2*4 = 3 + 8 = 11
    }

    #[test]
    fn test_mul_element_wise() {
        let a = Vec2::new(2.0, 3.0);
        let b = Vec2::new(4.0, 5.0);
        assert_eq!(a.mul_element_wise(b), Vec2::new(8.0, 15.0));
    }

    #[test]
    fn test_mul_vec2() {
        // Test that Mul<Vec2> performs element-wise multiplication
        let a = Vec2::new(2.0, 3.0);
        let b = Vec2::new(4.0, 5.0);
        let result = a * b;
        assert_eq!(result, Vec2::new(8.0, 15.0));

        // Test that it matches mul_element_wise
        assert_eq!(result, a.mul_element_wise(b));

        // Test with identity (ones)
        let ones = Vec2::one();
        let v = Vec2::new(2.0, 3.0);
        assert_eq!(ones * v, v);
        assert_eq!(v * ones, v);
    }

    #[test]
    fn test_length() {
        let v = Vec2::new(3.0, 4.0);
        assert_eq!(v.length_squared(), 25.0);
        assert_eq!(v.length(), 5.0);
    }

    #[test]
    fn test_normalize() {
        let v = Vec2::new(3.0, 4.0);
        let normalized = v.normalize();
        assert!((normalized.length() - 1.0).abs() < 1e-6);
        assert_eq!(normalized, Vec2::new(0.6, 0.8));
    }

    #[test]
    fn test_normalize_or_zero() {
        let v = Vec2::new(3.0, 4.0);
        let normalized = v.normalize_or_zero();
        assert!((normalized.length() - 1.0).abs() < 1e-6);
        assert_eq!(normalized, Vec2::new(0.6, 0.8));

        let zero = Vec2::zero();
        assert_eq!(zero.normalize_or_zero(), Vec2::zero());
    }

    #[test]
    fn test_distance() {
        let a = Vec2::new(0.0, 0.0);
        let b = Vec2::new(3.0, 4.0);
        assert_eq!(a.distance_squared(b), 25.0);
        assert_eq!(a.distance(b), 5.0);
    }

    #[test]
    fn test_lerp() {
        let a = Vec2::new(0.0, 0.0);
        let b = Vec2::new(10.0, 10.0);
        let result = a.lerp(b, 0.5);
        assert_eq!(result, Vec2::new(5.0, 5.0));
    }

    #[test]
    fn test_abs() {
        let v = Vec2::new(-1.0, 2.0);
        assert_eq!(v.abs(), Vec2::new(1.0, 2.0));
    }

    #[test]
    fn test_min_max() {
        let a = Vec2::new(1.0, 5.0);
        let b = Vec2::new(3.0, 2.0);
        assert_eq!(a.min(b), Vec2::new(1.0, 2.0));
        assert_eq!(a.max(b), Vec2::new(3.0, 5.0));
    }

    #[test]
    fn test_clamp() {
        let v = Vec2::new(5.0, -5.0);
        let min = Vec2::new(0.0, 0.0);
        let max = Vec2::new(3.0, 3.0);
        assert_eq!(v.clamp(min, max), Vec2::new(3.0, 0.0));
    }

    #[test]
    fn test_floor_ceil_round() {
        let v = Vec2::new(1.7, -1.3);
        assert_eq!(v.floor(), Vec2::new(1.0, -2.0));
        assert_eq!(v.ceil(), Vec2::new(2.0, -1.0));
        assert_eq!(v.round(), Vec2::new(2.0, -1.0));
    }

    #[test]
    fn test_trunc() {
        let v = Vec2::new(1.7, -1.3);
        assert_eq!(v.trunc(), Vec2::new(1.0, -1.0));
    }

    #[test]
    fn test_min_f32() {
        let v = Vec2::new(5.0, 2.0);
        assert_eq!(v.min_f32(4.0), Vec2::new(4.0, 2.0));
    }

    #[test]
    fn test_max_f32() {
        let v = Vec2::new(5.0, 2.0);
        assert_eq!(v.max_f32(4.0), Vec2::new(5.0, 4.0));
    }

    #[test]
    fn test_clamp_f32() {
        let v = Vec2::new(1.0, 5.0);
        assert_eq!(v.clamp_f32(2.0, 6.0), Vec2::new(2.0, 5.0));
    }

    #[test]
    fn test_rem() {
        let v = Vec2::new(10.0, 7.0);
        let result = v % 4.0;
        assert_eq!(result, Vec2::new(2.0, 3.0));
    }

    #[test]
    fn test_rem_assign() {
        let mut v = Vec2::new(10.0, 7.0);
        v %= 4.0;
        assert_eq!(v, Vec2::new(2.0, 3.0));
    }

    #[test]
    fn test_to_array() {
        let v = Vec2::new(1.0, 2.0);
        assert_eq!(v.to_array(), [1.0, 2.0]);
    }

    #[test]
    fn test_from_array() {
        let arr = [1.0, 2.0];
        let v = Vec2::from_array(arr);
        assert_eq!(v.x, 1.0);
        assert_eq!(v.y, 2.0);
        assert_eq!(v.to_array(), arr);
    }

    #[test]
    fn test_as_slice() {
        let v = Vec2::new(1.0, 2.0);
        let slice = v.as_slice();
        assert_eq!(slice, &[1.0, 2.0]);
        assert_eq!(slice.len(), 2);
    }

    #[test]
    fn test_as_mut_slice() {
        let mut v = Vec2::new(1.0, 2.0);
        let slice = v.as_mut_slice();
        assert_eq!(slice, &[1.0, 2.0]);

        slice[0] = 5.0;
        slice[1] = 6.0;
        assert_eq!(v.x, 5.0);
        assert_eq!(v.y, 6.0);
    }

    #[test]
    fn test_display() {
        let v = Vec2::new(1.5, 2.5);
        let s = format!("{}", v);
        assert_eq!(s, "[1.5, 2.5]");
    }

    #[test]
    fn test_signum() {
        let v = Vec2::new(5.0, -3.0);
        let result = v.signum();
        assert_eq!(result, Vec2::new(1.0, -1.0));
    }

    #[test]
    fn test_copysign() {
        let v = Vec2::new(5.0, 3.0);
        let sign = Vec2::new(-1.0, 1.0);
        let result = v.copysign(sign);
        assert_eq!(result, Vec2::new(-5.0, 3.0));
    }

    #[test]
    fn test_swizzle() {
        let v = Vec2::new(1.0, 2.0);
        assert_eq!(v.xx(), Vec2::new(1.0, 1.0));
        assert_eq!(v.yy(), Vec2::new(2.0, 2.0));
        assert_eq!(v.xy(), Vec2::new(1.0, 2.0));
        assert_eq!(v.yx(), Vec2::new(2.0, 1.0));
    }
}
