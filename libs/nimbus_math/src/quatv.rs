use crate::vec3::Vec3;
use core::arch::x86_64::*;
use core::fmt;
use core::ops::Mul;

// Union for casting between __m128 and [f32; 4]
#[repr(C)]
union QuatCast {
    arr: [f32; 4],
    vec: __m128,
}

/// A vectorized quaternion representing a rotation.
/// Stored as (w, x, y, z) where w is the scalar part and (x, y, z) is the vector part.
/// This version uses SSE optimizations when available.
#[repr(transparent)]
#[derive(Copy, Clone, Debug)]
pub struct QuatV {
    // Internal storage: [w, x, y, z]
    #[cfg(all(target_arch = "x86_64", target_feature = "sse"))]
    pub(crate) data: __m128,
    #[cfg(not(all(target_arch = "x86_64", target_feature = "sse")))]
    pub(crate) data: [f32; 4],
}

impl PartialEq for QuatV {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        #[cfg(all(target_arch = "x86_64", target_feature = "sse"))]
        {
            unsafe { _mm_movemask_ps(_mm_cmpeq_ps(self.data, other.data)) == 0xF }
        }
        #[cfg(not(all(target_arch = "x86_64", target_feature = "sse")))]
        {
            self.data[0] == other.data[0]
                && self.data[1] == other.data[1]
                && self.data[2] == other.data[2]
                && self.data[3] == other.data[3]
        }
    }
}

impl Default for QuatV {
    #[inline(always)]
    fn default() -> Self {
        Self::identity()
    }
}

impl QuatV {
    /// Creates a quaternion from components.
    #[inline(always)]
    pub const fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        #[cfg(all(target_arch = "x86_64", target_feature = "sse"))]
        {
            unsafe {
                Self {
                    data: QuatCast { arr: [x, y, z, w] }.vec,
                }
            }
        }
        #[cfg(not(all(target_arch = "x86_64", target_feature = "sse")))]
        {
            Self { data: [x, y, z, w] }
        }
    }

    /// Creates the identity quaternion (no rotation).
    #[inline(always)]
    pub const fn identity() -> Self {
        Self::new(0.0, 0.0, 0.0, 1.0)
    }

    /// Returns the w component (scalar part).
    #[inline(always)]
    pub fn w(&self) -> f32 {
        #[cfg(all(target_arch = "x86_64", target_feature = "sse"))]
        {
            unsafe { QuatCast { vec: self.data }.arr[3] }
        }
        #[cfg(not(all(target_arch = "x86_64", target_feature = "sse")))]
        {
            self.data[3]
        }
    }

    /// Returns the x component (vector part).
    #[inline(always)]
    pub fn x(&self) -> f32 {
        #[cfg(all(target_arch = "x86_64", target_feature = "sse"))]
        {
            unsafe { QuatCast { vec: self.data }.arr[0] }
        }
        #[cfg(not(all(target_arch = "x86_64", target_feature = "sse")))]
        {
            self.data[0]
        }
    }

    /// Returns the y component (vector part).
    #[inline(always)]
    pub fn y(&self) -> f32 {
        #[cfg(all(target_arch = "x86_64", target_feature = "sse"))]
        {
            unsafe { QuatCast { vec: self.data }.arr[1] }
        }
        #[cfg(not(all(target_arch = "x86_64", target_feature = "sse")))]
        {
            self.data[1]
        }
    }

    /// Returns the z component (vector part).
    #[inline(always)]
    pub fn z(&self) -> f32 {
        #[cfg(all(target_arch = "x86_64", target_feature = "sse"))]
        {
            unsafe { QuatCast { vec: self.data }.arr[2] }
        }
        #[cfg(not(all(target_arch = "x86_64", target_feature = "sse")))]
        {
            self.data[2]
        }
    }

    /// Creates a quaternion from an angle (in radians) and an axis vector.
    /// The axis vector will be normalized automatically.
    #[inline]
    #[must_use]
    pub fn from_angle_axis(angle: f32, axis: Vec3) -> Self {
        let axis = axis.normalize();
        let half_angle = angle * 0.5;
        let (s, c) = half_angle.sin_cos();
        Self::new(axis.x() * s, axis.y() * s, axis.z() * s, c)
    }

    /// Creates a quaternion from an angle (in radians) and an axis vector.
    ///
    /// # Safety
    ///
    /// The axis vector must be normalized (have unit length). Calling this function
    /// with a non-normalized axis will produce an incorrect quaternion.
    #[inline]
    #[must_use]
    pub unsafe fn from_angle_axis_unsafe(angle: f32, axis: Vec3) -> Self {
        let half_angle = angle * 0.5;
        let (s, c) = half_angle.sin_cos();
        Self::new(axis.x() * s, axis.y() * s, axis.z() * s, c)
    }

    /// Creates a quaternion representing a rotation around the X axis.
    #[inline]
    #[must_use]
    pub fn from_rotation_x(angle: f32) -> Self {
        let half_angle = angle * 0.5;
        let (s, c) = half_angle.sin_cos();
        Self::new(s, 0.0, 0.0, c)
    }

    /// Creates a quaternion representing a rotation around the Y axis.
    #[inline]
    #[must_use]
    pub fn from_rotation_y(angle: f32) -> Self {
        let half_angle = angle * 0.5;
        let (s, c) = half_angle.sin_cos();
        Self::new(0.0, s, 0.0, c)
    }

    /// Creates a quaternion representing a rotation around the Z axis.
    #[inline]
    #[must_use]
    pub fn from_rotation_z(angle: f32) -> Self {
        let half_angle = angle * 0.5;
        let (s, c) = half_angle.sin_cos();
        Self::new(0.0, 0.0, s, c)
    }

    /// Creates a quaternion from Euler angles (in radians).
    /// Uses XYZ convention where rotations are applied in order: Z, Y, X.
    #[inline]
    #[must_use]
    pub fn from_euler_angles(x: f32, y: f32, z: f32) -> Self {
        let half_x = x * 0.5;
        let half_y = y * 0.5;
        let half_z = z * 0.5;

        #[cfg(all(target_arch = "x86_64", target_feature = "sse"))]
        {
            unsafe {
                let (sx, cx) = half_x.sin_cos();
                let (sy, cy) = half_y.sin_cos();
                let (sz, cz) = half_z.sin_cos();

                //[sx, sy, sz, cx]
                let sin_mask = _mm_setr_ps(sx, sy, sz, cx);

                //[cx, cy, cz, sx]
                let cos_mask = _mm_setr_ps(cx, cy, cz, sx);

                //[sx*cy, sy*cx, sz*cx, cx*cy]
                let s1 = _mm_mul_ps(
                    sin_mask,
                    _mm_shuffle_ps(cos_mask, cos_mask, shuffle_mask!(1, 2, 0, 1)),
                );
                //[cx*sy, cy*sz, cz*sx, sx*sy]
                let c1 = _mm_mul_ps(
                    cos_mask,
                    _mm_shuffle_ps(sin_mask, sin_mask, shuffle_mask!(1, 2, 0, 1)),
                );

                //[sx*cy*cz, sy*cx*cz, sz*cx*cy, cx*cy*cz]
                let s2 = _mm_mul_ps(
                    s1,
                    _mm_shuffle_ps(cos_mask, cos_mask, shuffle_mask!(2, 0, 1, 2)),
                );

                //[cx*sy*sz, cy*sz*sx, cz*sx*sy, sx*sy*sz]
                let c2 = _mm_mul_ps(
                    c1,
                    _mm_shuffle_ps(sin_mask, sin_mask, shuffle_mask!(2, 0, 1, 2)),
                );

                let c3 = _mm_xor_ps(c2, _mm_setr_ps(-0.0, 0.0, -0.0, 0.0));
                let result = _mm_add_ps(s2, c3);

                Self { data: result }
            }
        }
        #[cfg(not(all(target_arch = "x86_64", target_feature = "sse")))]
        {
            let (sx, cx) = half_x.sin_cos();
            let (sy, cy) = half_y.sin_cos();
            let (sz, cz) = half_z.sin_cos();

            // Quaternion multiplication: q_z * q_y * q_x
            Self::new(
                sx * cy * cz - cx * sy * sz,
                cx * sy * cz + sx * cy * sz,
                cx * cy * sz - sx * sy * cz,
                cx * cy * cz + sx * sy * sz,
            )
        }
    }

    /// Computes the conjugate of the quaternion (inverse rotation).
    #[inline(always)]
    #[must_use]
    pub fn conjugate(self) -> Self {
        #[cfg(all(target_arch = "x86_64", target_feature = "sse"))]
        {
            unsafe {
                let sign_mask = _mm_castsi128_ps(_mm_setr_epi32(
                    0x80000000u32 as i32,
                    0x80000000u32 as i32,
                    0x80000000u32 as i32,
                    0,
                ));
                Self {
                    data: _mm_xor_ps(self.data, sign_mask),
                }
            }
        }
        #[cfg(not(all(target_arch = "x86_64", target_feature = "sse")))]
        {
            Self::new(-self.x(), -self.y(), -self.z(), self.w())
        }
    }

    /// Computes the inverse of the quaternion.
    /// For unit quaternions, this is the same as the conjugate.
    #[inline]
    #[must_use]
    pub fn inverse(self) -> Self {
        let norm_sq = self.length_squared();
        if norm_sq > 0.0 {
            let inv_norm_sq = norm_sq.recip();
            Self::new(
                -self.x() * inv_norm_sq,
                -self.y() * inv_norm_sq,
                -self.z() * inv_norm_sq,
                self.w() * inv_norm_sq,
            )
        } else {
            Self::identity()
        }
    }

    /// Computes the squared length of the quaternion.
    #[inline(always)]
    pub fn length_squared(self) -> f32 {
        #[cfg(all(target_arch = "x86_64", target_feature = "sse"))]
        {
            unsafe {
                let mul = _mm_mul_ps(self.data, self.data);
                let sum1 = _mm_add_ps(mul, _mm_shuffle_ps(mul, mul, shuffle_mask!(1, 0, 3, 2)));
                let sum2 = _mm_add_ps(sum1, _mm_shuffle_ps(sum1, sum1, shuffle_mask!(2, 3, 0, 1)));
                _mm_cvtss_f32(sum2)
            }
        }
        #[cfg(not(all(target_arch = "x86_64", target_feature = "sse")))]
        {
            self.data[0] * self.data[0]
                + self.data[1] * self.data[1]
                + self.data[2] * self.data[2]
                + self.data[3] * self.data[3]
        }
    }

    /// Computes the length of the quaternion.
    #[inline]
    pub fn length(self) -> f32 {
        #[cfg(all(target_arch = "x86_64", target_feature = "sse"))]
        {
            unsafe {
                let mul = _mm_mul_ps(self.data, self.data);
                let sum1 = _mm_add_ps(mul, _mm_shuffle_ps(mul, mul, shuffle_mask!(1, 0, 3, 2)));
                let sum2 = _mm_add_ps(sum1, _mm_shuffle_ps(sum1, sum1, shuffle_mask!(2, 3, 0, 1)));
                _mm_cvtss_f32(_mm_sqrt_ss(sum2))
            }
        }
        #[cfg(not(all(target_arch = "x86_64", target_feature = "sse")))]
        {
            (self.data[0] * self.data[0]
                + self.data[1] * self.data[1]
                + self.data[2] * self.data[2]
                + self.data[3] * self.data[3])
                .sqrt()
        }
    }

    /// Rotates this quaternion around the X axis by the given angle (in radians).
    /// Returns a new quaternion representing the combined rotation.
    #[inline]
    #[must_use]
    pub fn rotate_x(self, angle: f32) -> Self {
        self * Self::from_rotation_x(angle)
    }

    /// Rotates this quaternion around the Y axis by the given angle (in radians).
    /// Returns a new quaternion representing the combined rotation.
    #[inline]
    #[must_use]
    pub fn rotate_y(self, angle: f32) -> Self {
        self * Self::from_rotation_y(angle)
    }

    /// Rotates this quaternion around the Z axis by the given angle (in radians).
    /// Returns a new quaternion representing the combined rotation.
    #[inline]
    #[must_use]
    pub fn rotate_z(self, angle: f32) -> Self {
        self * Self::from_rotation_z(angle)
    }

    /// Rotates this quaternion around the given axis by the given angle (in radians).
    /// The axis vector will be normalized automatically.
    /// Returns a new quaternion representing the combined rotation.
    #[inline]
    #[must_use]
    pub fn rotate_axis(self, angle: f32, axis: Vec3) -> Self {
        self * Self::from_angle_axis(angle, axis)
    }

    /// Rotates this quaternion around the given axis by the given angle (in radians).
    ///
    /// # Safety
    ///
    /// The axis vector must be normalized (have unit length). Calling this function
    /// with a non-normalized axis will produce an incorrect rotation.
    ///
    /// Returns a new quaternion representing the combined rotation.
    #[inline]
    #[must_use]
    pub unsafe fn rotate_axis_unsafe(self, angle: f32, axis: Vec3) -> Self {
        unsafe { self * Self::from_angle_axis_unsafe(angle, axis) }
    }
}

impl Mul<QuatV> for QuatV {
    type Output = Self;

    /// Multiplies two quaternions (combines rotations).
    /// The result represents the rotation of `self` followed by `other`.
    /// Based on https://github.com/nfrechette/rtm `rtm::quat_mul` and glam's implementation.
    #[inline]
    fn mul(self, other: Self) -> Self {
        #[cfg(all(target_arch = "x86_64", target_feature = "sse"))]
        {
            // Based on https://github.com/nfrechette/rtm `rtm::quat_mul`
            const CONTROL_WZYX: __m128 = unsafe { core::mem::transmute([1.0f32, -1.0, 1.0, -1.0]) };
            const CONTROL_ZWXY: __m128 = unsafe { core::mem::transmute([1.0f32, 1.0, -1.0, -1.0]) };
            const CONTROL_YXWZ: __m128 = unsafe { core::mem::transmute([-1.0f32, 1.0, 1.0, -1.0]) };

            unsafe {
                let lhs = self.data;
                let rhs = other.data;

                let r_xxxx = _mm_shuffle_ps(lhs, lhs, 0b00_00_00_00);
                let r_yyyy = _mm_shuffle_ps(lhs, lhs, 0b01_01_01_01);
                let r_zzzz = _mm_shuffle_ps(lhs, lhs, 0b10_10_10_10);
                let r_wwww = _mm_shuffle_ps(lhs, lhs, 0b11_11_11_11);

                let lxrw_lyrw_lzrw_lwrw = _mm_mul_ps(r_wwww, rhs);
                let l_wzyx = _mm_shuffle_ps(rhs, rhs, 0b00_01_10_11);

                let lwrx_lzrx_lyrx_lxrx = _mm_mul_ps(r_xxxx, l_wzyx);
                let l_zwxy = _mm_shuffle_ps(l_wzyx, l_wzyx, 0b10_11_00_01);

                let lwrx_nlzrx_lyrx_nlxrx = _mm_mul_ps(lwrx_lzrx_lyrx_lxrx, CONTROL_WZYX);

                let lzry_lwry_lxry_lyry = _mm_mul_ps(r_yyyy, l_zwxy);
                let l_yxwz = _mm_shuffle_ps(l_zwxy, l_zwxy, 0b00_01_10_11);

                let lzry_lwry_nlxry_nlyry = _mm_mul_ps(lzry_lwry_lxry_lyry, CONTROL_ZWXY);

                let lyrz_lxrz_lwrz_lzrz = _mm_mul_ps(r_zzzz, l_yxwz);
                let result0 = _mm_add_ps(lxrw_lyrw_lzrw_lwrw, lwrx_nlzrx_lyrx_nlxrx);

                let nlyrz_lxrz_lwrz_wlzrz = _mm_mul_ps(lyrz_lxrz_lwrz_lzrz, CONTROL_YXWZ);
                let result1 = _mm_add_ps(lzry_lwry_nlxry_nlyry, nlyrz_lxrz_lwrz_wlzrz);

                Self {
                    data: _mm_add_ps(result0, result1),
                }
            }
        }
        #[cfg(not(all(target_arch = "x86_64", target_feature = "sse")))]
        {
            let x1 = self.x();
            let y1 = self.y();
            let z1 = self.z();
            let w1 = self.w();

            let x2 = other.x();
            let y2 = other.y();
            let z2 = other.z();
            let w2 = other.w();
            // Use scalar path for now - SSE optimization needs to be fixed
            Self::new(
                w1 * x2 + x1 * w2 + y1 * z2 - z1 * y2,
                w1 * y2 - x1 * z2 + y1 * w2 + z1 * x2,
                w1 * z2 + x1 * y2 - y1 * x2 + z1 * w2,
                w1 * w2 - x1 * x2 - y1 * y2 - z1 * z2,
            )
        }
    }
}

impl fmt::Display for QuatV {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "QuatV({:.4}, {:.4}, {:.4}, {:.4})",
            self.x(),
            self.y(),
            self.z(),
            self.w()
        )
    }
}
