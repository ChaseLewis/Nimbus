use core::fmt;
use core::ops::{
    Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Rem, RemAssign, Sub, SubAssign,
};

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Default for Vec3 {
    #[inline]
    fn default() -> Self {
        Self::zero()
    }
}

impl Vec3 {
    #[inline]
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    #[inline]
    pub const fn zero() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }
    }

    #[inline]
    pub const fn one() -> Self {
        Self {
            x: 1.0,
            y: 1.0,
            z: 1.0,
        }
    }

    #[inline]
    pub const fn splat(value: f32) -> Self {
        Self {
            x: value,
            y: value,
            z: value,
        }
    }

    #[inline(always)]
    #[must_use]
    pub const fn from_array(arr: [f32; 3]) -> Self {
        Self {
            x: arr[0],
            y: arr[1],
            z: arr[2],
        }
    }

    /// Creates a vector from a slice.
    /// Panics if the slice has fewer than 3 elements.
    #[inline]
    #[must_use]
    pub fn from_slice(slice: &[f32]) -> Self {
        Self::new(slice[0], slice[1], slice[2])
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

    #[inline(always)]
    pub fn z(&self) -> f32 {
        self.z
    }

    #[inline]
    pub fn length_squared(self) -> f32 {
        self.x * self.x + self.y * self.y + self.z * self.z
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
            z: self.z * inv_len,
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
                z: self.z * inv_len,
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
                z: self.z * inv_len,
            }
        } else {
            Self::zero()
        }
    }

    #[inline]
    pub fn dot(self, other: Self) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    /// Multiplies two vectors element-wise.
    #[inline]
    pub fn mul_element_wise(self, other: Self) -> Self {
        Self {
            x: self.x * other.x,
            y: self.y * other.y,
            z: self.z * other.z,
        }
    }

    #[inline]
    pub fn cross(self, other: Self) -> Self {
        Self {
            x: self.y * other.z - self.z * other.y,
            y: self.z * other.x - self.x * other.z,
            z: self.x * other.y - self.y * other.x,
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
            z: self.z.abs(),
        }
    }

    #[inline]
    pub fn min(self, other: Self) -> Self {
        Self {
            x: self.x.min(other.x),
            y: self.y.min(other.y),
            z: self.z.min(other.z),
        }
    }

    #[inline]
    pub fn max(self, other: Self) -> Self {
        Self {
            x: self.x.max(other.x),
            y: self.y.max(other.y),
            z: self.z.max(other.z),
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
            z: self.z.min(scalar),
        }
    }

    #[inline]
    pub fn max_f32(self, scalar: f32) -> Self {
        Self {
            x: self.x.max(scalar),
            y: self.y.max(scalar),
            z: self.z.max(scalar),
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
            z: self.z.signum(),
        }
    }

    #[inline]
    pub fn copysign(self, sign: Self) -> Self {
        Self {
            x: self.x.copysign(sign.x),
            y: self.y.copysign(sign.y),
            z: self.z.copysign(sign.z),
        }
    }

    #[inline]
    pub fn trunc(self) -> Self {
        Self {
            x: self.x.trunc(),
            y: self.y.trunc(),
            z: self.z.trunc(),
        }
    }

    #[inline]
    pub fn floor(self) -> Self {
        Self {
            x: self.x.floor(),
            y: self.y.floor(),
            z: self.z.floor(),
        }
    }

    #[inline]
    pub fn ceil(self) -> Self {
        Self {
            x: self.x.ceil(),
            y: self.y.ceil(),
            z: self.z.ceil(),
        }
    }

    #[inline]
    pub fn round(self) -> Self {
        Self {
            x: self.x.round(),
            y: self.y.round(),
            z: self.z.round(),
        }
    }

    #[inline]
    pub fn to_array(self) -> [f32; 3] {
        [self.x, self.y, self.z]
    }

    /// Returns a slice of the vector components.
    #[inline]
    pub fn as_slice(&self) -> &[f32] {
        unsafe { std::slice::from_raw_parts(self as *const Self as *const f32, 3) }
    }

    /// Returns a mutable slice of the vector components.
    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [f32] {
        unsafe { std::slice::from_raw_parts_mut(self as *mut Self as *mut f32, 3) }
    }

    // Swizzle methods - create new vectors by repeating components
    /// Returns a Vec3 with all components set to x.
    #[inline]
    pub fn xxx(self) -> Self {
        Self {
            x: self.x,
            y: self.x,
            z: self.x,
        }
    }

    /// Returns a Vec3 with all components set to y.
    #[inline]
    pub fn yyy(self) -> Self {
        Self {
            x: self.y,
            y: self.y,
            z: self.y,
        }
    }

    /// Returns a Vec3 with all components set to z.
    #[inline]
    pub fn zzz(self) -> Self {
        Self {
            x: self.z,
            y: self.z,
            z: self.z,
        }
    }

    /// Returns a Vec3 with components (x, y, z) - identity operation.
    #[inline]
    pub fn xyz(self) -> Self {
        self
    }

    /// Returns a Vec3 with components (x, z, y).
    #[inline]
    pub fn xzy(self) -> Self {
        Self {
            x: self.x,
            y: self.z,
            z: self.y,
        }
    }

    /// Returns a Vec3 with components (y, x, z).
    #[inline]
    pub fn yxz(self) -> Self {
        Self {
            x: self.y,
            y: self.x,
            z: self.z,
        }
    }

    /// Returns a Vec3 with components (y, z, x).
    #[inline]
    pub fn yzx(self) -> Self {
        Self {
            x: self.y,
            y: self.z,
            z: self.x,
        }
    }

    /// Returns a Vec3 with components (z, x, y).
    #[inline]
    pub fn zxy(self) -> Self {
        Self {
            x: self.z,
            y: self.x,
            z: self.y,
        }
    }

    /// Returns a Vec3 with components (z, y, x).
    #[inline]
    pub fn zyx(self) -> Self {
        Self {
            x: self.z,
            y: self.y,
            z: self.x,
        }
    }
}

impl Add for Vec3 {
    type Output = Self;

    #[inline]
    fn add(self, other: Self) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
            z: self.z + other.z,
        }
    }
}

impl Sub for Vec3 {
    type Output = Self;

    #[inline]
    fn sub(self, other: Self) -> Self {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
            z: self.z - other.z,
        }
    }
}

impl Mul<f32> for Vec3 {
    type Output = Self;

    #[inline]
    fn mul(self, scalar: f32) -> Self {
        Self {
            x: self.x * scalar,
            y: self.y * scalar,
            z: self.z * scalar,
        }
    }
}

impl Mul<Vec3> for f32 {
    type Output = Vec3;

    #[inline]
    fn mul(self, vec: Vec3) -> Vec3 {
        vec * self
    }
}

impl Mul<Vec3> for Vec3 {
    type Output = Self;

    /// Multiplies two vectors element-wise.
    #[inline]
    fn mul(self, other: Self) -> Self {
        self.mul_element_wise(other)
    }
}

impl Div<f32> for Vec3 {
    type Output = Self;

    #[inline]
    fn div(self, scalar: f32) -> Self {
        self * (1.0 / scalar)
    }
}

impl Neg for Vec3 {
    type Output = Self;

    #[inline]
    fn neg(self) -> Self {
        Self {
            x: -self.x,
            y: -self.y,
            z: -self.z,
        }
    }
}

impl AddAssign for Vec3 {
    #[inline]
    fn add_assign(&mut self, other: Self) {
        *self = *self + other;
    }
}

impl SubAssign for Vec3 {
    #[inline]
    fn sub_assign(&mut self, other: Self) {
        *self = *self - other;
    }
}

impl MulAssign<f32> for Vec3 {
    #[inline]
    fn mul_assign(&mut self, scalar: f32) {
        *self = *self * scalar;
    }
}

impl DivAssign<f32> for Vec3 {
    #[inline]
    fn div_assign(&mut self, scalar: f32) {
        *self = *self / scalar;
    }
}

impl Rem<f32> for Vec3 {
    type Output = Self;

    #[inline]
    fn rem(self, scalar: f32) -> Self {
        Self {
            x: self.x % scalar,
            y: self.y % scalar,
            z: self.z % scalar,
        }
    }
}

impl RemAssign<f32> for Vec3 {
    #[inline]
    fn rem_assign(&mut self, scalar: f32) {
        *self = *self % scalar;
    }
}

impl fmt::Display for Vec3 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}, {}, {}]", self.x, self.y, self.z)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let v = Vec3::new(1.0, 2.0, 3.0);
        assert_eq!(v.x, 1.0);
        assert_eq!(v.y, 2.0);
        assert_eq!(v.z, 3.0);
    }

    #[test]
    fn test_constants() {
        assert_eq!(Vec3::zero(), Vec3::new(0.0, 0.0, 0.0));
        assert_eq!(Vec3::one(), Vec3::new(1.0, 1.0, 1.0));
        assert_eq!(Vec3::splat(5.0), Vec3::new(5.0, 5.0, 5.0));
    }

    #[test]
    fn test_add() {
        let a = Vec3::new(1.0, 2.0, 3.0);
        let b = Vec3::new(4.0, 5.0, 6.0);
        let result = a + b;
        assert_eq!(result, Vec3::new(5.0, 7.0, 9.0));
    }

    #[test]
    fn test_sub() {
        let a = Vec3::new(5.0, 7.0, 9.0);
        let b = Vec3::new(2.0, 3.0, 4.0);
        let result = a - b;
        assert_eq!(result, Vec3::new(3.0, 4.0, 5.0));
    }

    #[test]
    fn test_mul_scalar() {
        let v = Vec3::new(2.0, 3.0, 4.0);
        let result = v * 2.0;
        assert_eq!(result, Vec3::new(4.0, 6.0, 8.0));

        let result2 = 2.0 * v;
        assert_eq!(result2, Vec3::new(4.0, 6.0, 8.0));
    }

    #[test]
    fn test_div_scalar() {
        let v = Vec3::new(4.0, 6.0, 8.0);
        let result = v / 2.0;
        assert_eq!(result, Vec3::new(2.0, 3.0, 4.0));
    }

    #[test]
    fn test_neg() {
        let v = Vec3::new(1.0, -2.0, 3.0);
        let result = -v;
        assert_eq!(result, Vec3::new(-1.0, 2.0, -3.0));
    }

    #[test]
    fn test_assign_ops() {
        let mut v = Vec3::new(1.0, 2.0, 3.0);
        v += Vec3::new(3.0, 4.0, 5.0);
        assert_eq!(v, Vec3::new(4.0, 6.0, 8.0));

        v -= Vec3::new(1.0, 2.0, 3.0);
        assert_eq!(v, Vec3::new(3.0, 4.0, 5.0));

        v *= 2.0;
        assert_eq!(v, Vec3::new(6.0, 8.0, 10.0));

        v /= 2.0;
        assert_eq!(v, Vec3::new(3.0, 4.0, 5.0));
    }

    #[test]
    fn test_dot() {
        let a = Vec3::new(1.0, 2.0, 3.0);
        let b = Vec3::new(4.0, 5.0, 6.0);
        let result = a.dot(b);
        assert_eq!(result, 32.0); // 1*4 + 2*5 + 3*6 = 4 + 10 + 18 = 32
    }

    #[test]
    fn test_mul_element_wise() {
        let a = Vec3::new(1.0, 2.0, 3.0);
        let b = Vec3::new(4.0, 5.0, 6.0);
        assert_eq!(a.mul_element_wise(b), Vec3::new(4.0, 10.0, 18.0));
    }

    #[test]
    fn test_mul_vec3() {
        // Test that Mul<Vec3> performs element-wise multiplication
        let a = Vec3::new(2.0, 3.0, 4.0);
        let b = Vec3::new(1.0, 2.0, 3.0);
        let result = a * b;
        assert_eq!(result, Vec3::new(2.0, 6.0, 12.0));

        // Test that it matches mul_element_wise
        assert_eq!(result, a.mul_element_wise(b));

        // Test with identity (ones)
        let ones = Vec3::one();
        let v = Vec3::new(2.0, 3.0, 4.0);
        assert_eq!(ones * v, v);
        assert_eq!(v * ones, v);
    }

    #[test]
    fn test_cross() {
        let a = Vec3::new(1.0, 0.0, 0.0);
        let b = Vec3::new(0.0, 1.0, 0.0);
        let result = a.cross(b);
        assert_eq!(result, Vec3::new(0.0, 0.0, 1.0));

        let a2 = Vec3::new(1.0, 2.0, 3.0);
        let b2 = Vec3::new(4.0, 5.0, 6.0);
        let result2 = a2.cross(b2);
        // Expected: (2*6 - 3*5, 3*4 - 1*6, 1*5 - 2*4) = (-3, 6, -3)
        assert_eq!(result2, Vec3::new(-3.0, 6.0, -3.0));
    }

    #[test]
    fn test_length() {
        let v = Vec3::new(2.0, 3.0, 6.0);
        assert_eq!(v.length_squared(), 49.0); // 4 + 9 + 36 = 49
        assert_eq!(v.length(), 7.0);
    }

    #[test]
    fn test_normalize() {
        let v = Vec3::new(3.0, 4.0, 0.0);
        let normalized = v.normalize();
        assert!((normalized.length() - 1.0).abs() < 1e-6);
        assert!((normalized.x - 0.6).abs() < 1e-6);
        assert!((normalized.y - 0.8).abs() < 1e-6);
    }

    #[test]
    fn test_normalize_or_zero() {
        let v = Vec3::new(3.0, 4.0, 0.0);
        let normalized = v.normalize_or_zero();
        assert!((normalized.length() - 1.0).abs() < 1e-6);
        assert!((normalized.x - 0.6).abs() < 1e-6);
        assert!((normalized.y - 0.8).abs() < 1e-6);

        let zero = Vec3::zero();
        assert_eq!(zero.normalize_or_zero(), Vec3::zero());
    }

    #[test]
    fn test_distance() {
        let a = Vec3::new(0.0, 0.0, 0.0);
        let b = Vec3::new(2.0, 3.0, 6.0);
        assert_eq!(a.distance_squared(b), 49.0);
        assert_eq!(a.distance(b), 7.0);
    }

    #[test]
    fn test_lerp() {
        let a = Vec3::new(0.0, 0.0, 0.0);
        let b = Vec3::new(10.0, 10.0, 10.0);
        let result = a.lerp(b, 0.5);
        assert_eq!(result, Vec3::new(5.0, 5.0, 5.0));
    }

    #[test]
    fn test_abs() {
        let v = Vec3::new(-1.0, 2.0, -3.0);
        assert_eq!(v.abs(), Vec3::new(1.0, 2.0, 3.0));
    }

    #[test]
    fn test_min_max() {
        let a = Vec3::new(1.0, 5.0, 3.0);
        let b = Vec3::new(3.0, 2.0, 4.0);
        assert_eq!(a.min(b), Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(a.max(b), Vec3::new(3.0, 5.0, 4.0));
    }

    #[test]
    fn test_clamp() {
        let v = Vec3::new(5.0, -5.0, 2.0);
        let min = Vec3::new(0.0, 0.0, 0.0);
        let max = Vec3::new(3.0, 3.0, 3.0);
        assert_eq!(v.clamp(min, max), Vec3::new(3.0, 0.0, 2.0));
    }

    #[test]
    fn test_floor_ceil_round() {
        let v = Vec3::new(1.7, -1.3, 2.5);
        assert_eq!(v.floor(), Vec3::new(1.0, -2.0, 2.0));
        assert_eq!(v.ceil(), Vec3::new(2.0, -1.0, 3.0));
        // Rust's round() uses "round half away from zero", so 2.5 -> 3.0, -1.3 -> -1.0
        assert_eq!(v.round(), Vec3::new(2.0, -1.0, 3.0));
    }

    #[test]
    fn test_trunc() {
        let v = Vec3::new(1.7, -1.3, 2.5);
        assert_eq!(v.trunc(), Vec3::new(1.0, -1.0, 2.0));
    }

    #[test]
    fn test_min_f32() {
        let v = Vec3::new(5.0, 2.0, 8.0);
        assert_eq!(v.min_f32(4.0), Vec3::new(4.0, 2.0, 4.0));
    }

    #[test]
    fn test_max_f32() {
        let v = Vec3::new(5.0, 2.0, 8.0);
        assert_eq!(v.max_f32(4.0), Vec3::new(5.0, 4.0, 8.0));
    }

    #[test]
    fn test_clamp_f32() {
        let v = Vec3::new(1.0, 5.0, 2.0);
        assert_eq!(v.clamp_f32(2.0, 6.0), Vec3::new(2.0, 5.0, 2.0));
    }

    #[test]
    fn test_rem() {
        let v = Vec3::new(10.0, 7.0, 15.0);
        let result = v % 4.0;
        assert_eq!(result, Vec3::new(2.0, 3.0, 3.0));
    }

    #[test]
    fn test_rem_assign() {
        let mut v = Vec3::new(10.0, 7.0, 15.0);
        v %= 4.0;
        assert_eq!(v, Vec3::new(2.0, 3.0, 3.0));
    }

    #[test]
    fn test_to_array() {
        let v = Vec3::new(1.0, 2.0, 3.0);
        assert_eq!(v.to_array(), [1.0, 2.0, 3.0]);
    }

    #[test]
    fn test_from_array() {
        let arr = [1.0, 2.0, 3.0];
        let v = Vec3::from_array(arr);
        assert_eq!(v.x, 1.0);
        assert_eq!(v.y, 2.0);
        assert_eq!(v.z, 3.0);
        assert_eq!(v.to_array(), arr);
    }

    #[test]
    fn test_as_slice() {
        let v = Vec3::new(1.0, 2.0, 3.0);
        let slice = v.as_slice();
        assert_eq!(slice, &[1.0, 2.0, 3.0]);
        assert_eq!(slice.len(), 3);
    }

    #[test]
    fn test_as_mut_slice() {
        let mut v = Vec3::new(1.0, 2.0, 3.0);
        let slice = v.as_mut_slice();
        assert_eq!(slice, &[1.0, 2.0, 3.0]);

        slice[0] = 5.0;
        slice[1] = 6.0;
        slice[2] = 7.0;
        assert_eq!(v.x, 5.0);
        assert_eq!(v.y, 6.0);
        assert_eq!(v.z, 7.0);
    }

    #[test]
    fn test_display() {
        let v = Vec3::new(1.5, 2.5, 3.5);
        let s = format!("{}", v);
        assert_eq!(s, "[1.5, 2.5, 3.5]");
    }

    #[test]
    fn test_signum() {
        // Test with non-zero values
        let v1 = Vec3::new(5.0, -3.0, 2.0);
        let result1 = v1.signum();
        assert_eq!(result1, Vec3::new(1.0, -1.0, 1.0));

        // Test with zero
        let v2 = Vec3::new(5.0, -3.0, 0.0);
        let result2 = v2.signum();
        assert_eq!(result2.x, 1.0);
        assert_eq!(result2.y, -1.0);
        // Rust's signum(0.0) returns 0.0
        assert_eq!(result2.z, 0.0f32.signum());
    }

    #[test]
    fn test_copysign() {
        let v = Vec3::new(5.0, 3.0, 2.0);
        let sign = Vec3::new(-1.0, 1.0, -1.0);
        let result = v.copysign(sign);
        assert_eq!(result, Vec3::new(-5.0, 3.0, -2.0));
    }

    #[test]
    fn test_swizzle() {
        let v = Vec3::new(1.0, 2.0, 3.0);
        assert_eq!(v.xxx(), Vec3::new(1.0, 1.0, 1.0));
        assert_eq!(v.yyy(), Vec3::new(2.0, 2.0, 2.0));
        assert_eq!(v.zzz(), Vec3::new(3.0, 3.0, 3.0));
        assert_eq!(v.xyz(), Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(v.xzy(), Vec3::new(1.0, 3.0, 2.0));
        assert_eq!(v.yxz(), Vec3::new(2.0, 1.0, 3.0));
        assert_eq!(v.yzx(), Vec3::new(2.0, 3.0, 1.0));
        assert_eq!(v.zxy(), Vec3::new(3.0, 1.0, 2.0));
        assert_eq!(v.zyx(), Vec3::new(3.0, 2.0, 1.0));
    }
}
