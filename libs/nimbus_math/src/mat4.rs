use crate::quat::Quat;
use crate::vec3::Vec3;
use crate::vec4::Vec4;
use core::arch::x86_64::*;
use core::fmt;
use core::ops::{Mul, MulAssign};

#[cfg(all(target_arch = "x86_64", target_feature = "sse"))]
#[inline(always)]
fn dot4_ps(a: __m128, b: __m128) -> f32 {
    unsafe {
        let mul = _mm_mul_ps(a, b);
        let sum1 = _mm_add_ps(mul, _mm_shuffle_ps(mul, mul, shuffle_mask!(1, 0, 3, 2)));
        let sum2 = _mm_add_ps(sum1, _mm_shuffle_ps(sum1, sum1, shuffle_mask!(2, 3, 0, 1)));
        _mm_cvtss_f32(sum2)
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Mat4 {
    pub x_axis: Vec4,
    pub y_axis: Vec4,
    pub z_axis: Vec4,
    pub w_axis: Vec4,
}

impl Default for Mat4 {
    #[inline(always)]
    fn default() -> Self {
        Self::identity()
    }
}

impl Mat4 {
    /// Creates a 4x4 identity matrix.
    #[inline(always)]
    #[must_use]
    pub const fn identity() -> Self {
        Self {
            x_axis: Vec4::new(1.0, 0.0, 0.0, 0.0),
            y_axis: Vec4::new(0.0, 1.0, 0.0, 0.0),
            z_axis: Vec4::new(0.0, 0.0, 1.0, 0.0),
            w_axis: Vec4::new(0.0, 0.0, 0.0, 1.0),
        }
    }

    /// Creates a matrix with the given diagonal values.
    #[inline(always)]
    #[must_use]
    pub fn from_diagonal(diagonal: Vec4) -> Self {
        Self {
            x_axis: Vec4::new(diagonal.x(), 0.0, 0.0, 0.0),
            y_axis: Vec4::new(0.0, diagonal.y(), 0.0, 0.0),
            z_axis: Vec4::new(0.0, 0.0, diagonal.z(), 0.0),
            w_axis: Vec4::new(0.0, 0.0, 0.0, diagonal.w()),
        }
    }

    /// Creates a matrix from four column vectors.
    #[inline(always)]
    #[must_use]
    pub const fn from_cols(x_axis: Vec4, y_axis: Vec4, z_axis: Vec4, w_axis: Vec4) -> Self {
        Self {
            x_axis,
            y_axis,
            z_axis,
            w_axis,
        }
    }

    /// Creates a matrix from four column vectors.
    #[inline(always)]
    #[must_use]
    pub const fn from_col_arrays(
        x_axis: [f32; 4],
        y_axis: [f32; 4],
        z_axis: [f32; 4],
        w_axis: [f32; 4],
    ) -> Self {
        Self {
            x_axis: Vec4::from_array(x_axis),
            y_axis: Vec4::from_array(y_axis),
            z_axis: Vec4::from_array(z_axis),
            w_axis: Vec4::from_array(w_axis),
        }
    }

    /// Creates a rotation matrix from an angle (in radians) and an axis vector.
    /// Uses Rodrigues' rotation formula: R = I + sin(θ)[k]× + (1 - cos(θ))[k]×²
    /// where [k]× is the skew-symmetric (cross-product) matrix of the normalized axis.
    ///
    /// The axis vector will be normalized automatically.
    #[inline]
    #[must_use]
    pub fn from_angle_axis(angle: f32, axis: Vec3) -> Self {
        let axis = axis.normalize();
        let (s, c) = angle.sin_cos();
        let omc = 1.0 - c; // one minus cosine

        let x = axis.x();
        let y = axis.y();
        let z = axis.z();

        // Skew-symmetric matrix [k]× components
        // [k]× = [0  -z   y]
        //        [z   0  -x]
        //        [-y  x   0]

        // [k]×² = k⊗k - I (outer product minus identity)
        let xx = x * x;
        let xy = x * y;
        let xz = x * z;
        let yy = y * y;
        let yz = y * z;
        let zz = z * z;

        // Rodrigues' rotation formula: R = I + sin(θ)[k]× + (1 - cos(θ))[k]×²
        Self {
            x_axis: Vec4::new(c + omc * xx, omc * xy + s * z, omc * xz - s * y, 0.0),
            y_axis: Vec4::new(omc * xy - s * z, c + omc * yy, omc * yz + s * x, 0.0),
            z_axis: Vec4::new(omc * xz + s * y, omc * yz - s * x, c + omc * zz, 0.0),
            w_axis: Vec4::new(0.0, 0.0, 0.0, 1.0),
        }
    }

    /// Creates a rotation matrix from an angle (in radians) and an axis vector.
    /// Uses Rodrigues' rotation formula: R = I + sin(θ)[k]× + (1 - cos(θ))[k]×²
    /// where [k]× is the skew-symmetric (cross-product) matrix of the normalized axis.
    ///
    /// # Safety
    ///
    /// The axis vector must be normalized (have unit length). Calling this function
    /// with a non-normalized axis will produce an incorrect rotation matrix.
    ///
    /// This function is unsafe because it does not verify that the axis is normalized,
    /// which can lead to incorrect results if the precondition is violated.
    #[inline]
    #[must_use]
    pub unsafe fn from_angle_axis_unsafe(angle: f32, axis: Vec3) -> Self {
        let (s, c) = angle.sin_cos();
        let omc = 1.0 - c; // one minus cosine

        let x = axis.x();
        let y = axis.y();
        let z = axis.z();

        // Skew-symmetric matrix [k]× components
        // [k]× = [0  -z   y]
        //        [z   0  -x]
        //        [-y  x   0]

        // [k]×² = k⊗k - I (outer product minus identity)
        let xx = x * x;
        let xy = x * y;
        let xz = x * z;
        let yy = y * y;
        let yz = y * z;
        let zz = z * z;

        // Rodrigues' rotation formula: R = I + sin(θ)[k]× + (1 - cos(θ))[k]×²
        Self {
            x_axis: Vec4::new(c + omc * xx, omc * xy + s * z, omc * xz - s * y, 0.0),
            y_axis: Vec4::new(omc * xy - s * z, c + omc * yy, omc * yz + s * x, 0.0),
            z_axis: Vec4::new(omc * xz + s * y, omc * yz - s * x, c + omc * zz, 0.0),
            w_axis: Vec4::new(0.0, 0.0, 0.0, 1.0),
        }
    }

    /// Creates a transformation matrix from translation, rotation (quaternion), and scale.
    /// The resulting matrix represents the transformation: T * R * S
    /// where T is translation, R is rotation, and S is scale.
    #[inline]
    #[must_use]
    pub fn from_trs(translation: Vec3, rotation: Quat, scale: Vec3) -> Self {
        let x = rotation.x;
        let y = rotation.y;
        let z = rotation.z;
        let w = rotation.w;

        // Precompute squared components
        let xx = x * x;
        let yy = y * y;
        let zz = z * z;
        let xy = x * y;
        let xz = x * z;
        let yz = y * z;
        let wx = w * x;
        let wy = w * y;
        let wz = w * z;

        // Quaternion to rotation matrix conversion
        // [1-2(y²+z²)  2(xy-wz)     2(xz+wy)     0]
        // [2(xy+wz)    1-2(x²+z²)   2(yz-wx)     0]
        // [2(xz-wy)    2(yz+wx)     1-2(x²+y²)   0]
        // [0           0            0           1]
        let sx = scale.x();
        let sy = scale.y();
        let sz = scale.z();

        Self {
            x_axis: Vec4::new(
                (1.0 - 2.0 * (yy + zz)) * sx,
                (2.0 * (xy + wz)) * sx,
                (2.0 * (xz - wy)) * sx,
                translation.x(),
            ),
            y_axis: Vec4::new(
                (2.0 * (xy - wz)) * sy,
                (1.0 - 2.0 * (xx + zz)) * sy,
                (2.0 * (yz + wx)) * sy,
                translation.y(),
            ),
            z_axis: Vec4::new(
                (2.0 * (xz + wy)) * sz,
                (2.0 * (yz - wx)) * sz,
                (1.0 - 2.0 * (xx + yy)) * sz,
                translation.z(),
            ),
            w_axis: Vec4::new(0.0, 0.0, 0.0, 1.0),
        }
    }

    /// Computes the determinant of the matrix.
    /// Uses SSE-optimized algorithm based on glm's `glm_mat4_determinant_lowp`.
    #[inline]
    #[must_use]
    pub fn determinant(self) -> f32 {
        #[cfg(all(target_arch = "x86_64", target_feature = "sse"))]
        {
            unsafe {
                // Based on https://github.com/g-truc/glm `glm_mat4_determinant_lowp`
                // This algorithm computes the determinant using SSE operations for better performance

                // Shuffle z_axis and w_axis for cross products
                // Using exact binary values from glm_mat4_determinant_lowp
                let swp2a = _mm_shuffle_ps(self.z_axis.0, self.z_axis.0, 0b00_01_01_10);
                let swp3a = _mm_shuffle_ps(self.w_axis.0, self.w_axis.0, 0b11_10_11_11);
                let swp2b = _mm_shuffle_ps(self.z_axis.0, self.z_axis.0, 0b11_10_11_11);
                let swp3b = _mm_shuffle_ps(self.w_axis.0, self.w_axis.0, 0b00_01_01_10);
                let swp2c = _mm_shuffle_ps(self.z_axis.0, self.z_axis.0, 0b00_00_01_10);
                let swp3c = _mm_shuffle_ps(self.w_axis.0, self.w_axis.0, 0b01_10_00_00);

                // Compute cross products: mula = swp2a * swp3a, etc.
                let mula = _mm_mul_ps(swp2a, swp3a);
                let mulb = _mm_mul_ps(swp2b, swp3b);
                let mulc = _mm_mul_ps(swp2c, swp3c);

                // Compute differences
                let sube = _mm_sub_ps(mula, mulb);
                let subf = _mm_sub_ps(_mm_movehl_ps(mulc, mulc), mulc);

                // Compute cofactors for y_axis
                // Using exact binary values from glm_mat4_determinant_lowp
                let subfaca = _mm_shuffle_ps(sube, sube, 0b10_01_00_00);
                let swpfaca = _mm_shuffle_ps(self.y_axis.0, self.y_axis.0, 0b00_00_00_01);
                let mulfaca = _mm_mul_ps(swpfaca, subfaca);

                let subtmpb = _mm_shuffle_ps(sube, subf, 0b00_00_11_01);
                let subfacb = _mm_shuffle_ps(subtmpb, subtmpb, 0b11_01_01_00);
                let swpfacb = _mm_shuffle_ps(self.y_axis.0, self.y_axis.0, 0b01_01_10_10);
                let mulfacb = _mm_mul_ps(swpfacb, subfacb);

                let subres = _mm_sub_ps(mulfaca, mulfacb);

                let subtmpc = _mm_shuffle_ps(sube, subf, 0b01_00_10_10);
                let subfacc = _mm_shuffle_ps(subtmpc, subtmpc, 0b11_11_10_00);
                let swpfacc = _mm_shuffle_ps(self.y_axis.0, self.y_axis.0, 0b10_11_11_11);
                let mulfacc = _mm_mul_ps(swpfacc, subfacc);

                let addres = _mm_add_ps(subres, mulfacc);

                // Apply alternating signs: (1.0, -1.0, 1.0, -1.0) using bitwise XOR on sign bits
                // Create mask with sign bits set for lanes 1 and 3 (0x80000000), clear for 0 and 2 (0)
                // _mm_set_epi32 sets from high to low: w, z, y, x
                let sign_mask = _mm_castsi128_ps(_mm_set_epi32(
                    0x80000000u32 as i32,
                    0,
                    0x80000000u32 as i32,
                    0,
                ));
                let detcof = _mm_xor_ps(addres, sign_mask);

                // Final dot product: x_axis · detcof
                let mul = _mm_mul_ps(self.x_axis.0, detcof);
                let sum1 = _mm_add_ps(mul, _mm_shuffle_ps(mul, mul, shuffle_mask!(1, 0, 3, 2)));
                let sum2 = _mm_add_ps(sum1, _mm_shuffle_ps(sum1, sum1, shuffle_mask!(2, 3, 0, 1)));
                _mm_cvtss_f32(sum2)
            }
        }
        #[cfg(not(all(target_arch = "x86_64", target_feature = "sse")))]
        {
            // Fallback to scalar computation
            // Direct computation using Sarrus-like expansion for 4x4
            // More efficient than Laplace expansion with fewer function calls

            // Extract all elements directly
            let m00 = self.x_axis.x();
            let m01 = self.x_axis.y();
            let m02 = self.x_axis.z();
            let m03 = self.x_axis.w();

            let m10 = self.y_axis.x();
            let m11 = self.y_axis.y();
            let m12 = self.y_axis.z();
            let m13 = self.y_axis.w();

            let m20 = self.z_axis.x();
            let m21 = self.z_axis.y();
            let m22 = self.z_axis.z();
            let m23 = self.z_axis.w();

            let m30 = self.w_axis.x();
            let m31 = self.w_axis.y();
            let m32 = self.w_axis.z();
            let m33 = self.w_axis.w();

            // Compute common subexpressions to reduce redundant calculations
            let s0 = m20 * m31 - m21 * m30;
            let s1 = m20 * m32 - m22 * m30;
            let s2 = m20 * m33 - m23 * m30;
            let s3 = m21 * m32 - m22 * m31;
            let s4 = m21 * m33 - m23 * m31;
            let s5 = m22 * m33 - m23 * m32;

            // Compute 3x3 minors using common subexpressions
            let det00 = m11 * s5 - m12 * s4 + m13 * s3;
            let det01 = m10 * s5 - m12 * s2 + m13 * s1;
            let det02 = m10 * s4 - m11 * s2 + m13 * s0;
            let det03 = m10 * s3 - m11 * s1 + m12 * s0;

            // Final determinant: m00 * det00 - m01 * det01 + m02 * det02 - m03 * det03
            m00 * det00 - m01 * det01 + m02 * det02 - m03 * det03
        }
    }

    /// Computes the inverse of the matrix.
    /// Returns None if the matrix is singular (determinant is zero).
    #[inline]
    #[must_use]
    pub fn inverse(self) -> Option<Self> {
        #[cfg(all(target_arch = "x86_64", target_feature = "sse"))]
        unsafe {
            const EPS: f32 = 1e-10;

            let fac0 = {
                let swp0a = _mm_shuffle_ps(self.w_axis.0, self.z_axis.0, 0b11_11_11_11);
                let swp0b = _mm_shuffle_ps(self.w_axis.0, self.z_axis.0, 0b10_10_10_10);
                let swp00 = _mm_shuffle_ps(self.z_axis.0, self.y_axis.0, 0b10_10_10_10);
                let swp01 = _mm_shuffle_ps(swp0a, swp0a, 0b10_00_00_00);
                let swp02 = _mm_shuffle_ps(swp0b, swp0b, 0b10_00_00_00);
                let swp03 = _mm_shuffle_ps(self.z_axis.0, self.y_axis.0, 0b11_11_11_11);
                let mul00 = _mm_mul_ps(swp00, swp01);
                let mul01 = _mm_mul_ps(swp02, swp03);
                _mm_sub_ps(mul00, mul01)
            };

            let fac1 = {
                let swp0a = _mm_shuffle_ps(self.w_axis.0, self.z_axis.0, 0b11_11_11_11);
                let swp0b = _mm_shuffle_ps(self.w_axis.0, self.z_axis.0, 0b01_01_01_01);
                let swp00 = _mm_shuffle_ps(self.z_axis.0, self.y_axis.0, 0b01_01_01_01);
                let swp01 = _mm_shuffle_ps(swp0a, swp0a, 0b10_00_00_00);
                let swp02 = _mm_shuffle_ps(swp0b, swp0b, 0b10_00_00_00);
                let swp03 = _mm_shuffle_ps(self.z_axis.0, self.y_axis.0, 0b11_11_11_11);
                let mul00 = _mm_mul_ps(swp00, swp01);
                let mul01 = _mm_mul_ps(swp02, swp03);
                _mm_sub_ps(mul00, mul01)
            };

            let fac2 = {
                let swp0a = _mm_shuffle_ps(self.w_axis.0, self.z_axis.0, 0b10_10_10_10);
                let swp0b = _mm_shuffle_ps(self.w_axis.0, self.z_axis.0, 0b01_01_01_01);
                let swp00 = _mm_shuffle_ps(self.z_axis.0, self.y_axis.0, 0b01_01_01_01);
                let swp01 = _mm_shuffle_ps(swp0a, swp0a, 0b10_00_00_00);
                let swp02 = _mm_shuffle_ps(swp0b, swp0b, 0b10_00_00_00);
                let swp03 = _mm_shuffle_ps(self.z_axis.0, self.y_axis.0, 0b10_10_10_10);
                let mul00 = _mm_mul_ps(swp00, swp01);
                let mul01 = _mm_mul_ps(swp02, swp03);
                _mm_sub_ps(mul00, mul01)
            };

            let fac3 = {
                let swp0a = _mm_shuffle_ps(self.w_axis.0, self.z_axis.0, 0b11_11_11_11);
                let swp0b = _mm_shuffle_ps(self.w_axis.0, self.z_axis.0, 0b00_00_00_00);
                let swp00 = _mm_shuffle_ps(self.z_axis.0, self.y_axis.0, 0b00_00_00_00);
                let swp01 = _mm_shuffle_ps(swp0a, swp0a, 0b10_00_00_00);
                let swp02 = _mm_shuffle_ps(swp0b, swp0b, 0b10_00_00_00);
                let swp03 = _mm_shuffle_ps(self.z_axis.0, self.y_axis.0, 0b11_11_11_11);
                let mul00 = _mm_mul_ps(swp00, swp01);
                let mul01 = _mm_mul_ps(swp02, swp03);
                _mm_sub_ps(mul00, mul01)
            };

            let fac4 = {
                let swp0a = _mm_shuffle_ps(self.w_axis.0, self.z_axis.0, 0b10_10_10_10);
                let swp0b = _mm_shuffle_ps(self.w_axis.0, self.z_axis.0, 0b00_00_00_00);
                let swp00 = _mm_shuffle_ps(self.z_axis.0, self.y_axis.0, 0b00_00_00_00);
                let swp01 = _mm_shuffle_ps(swp0a, swp0a, 0b10_00_00_00);
                let swp02 = _mm_shuffle_ps(swp0b, swp0b, 0b10_00_00_00);
                let swp03 = _mm_shuffle_ps(self.z_axis.0, self.y_axis.0, 0b10_10_10_10);
                let mul00 = _mm_mul_ps(swp00, swp01);
                let mul01 = _mm_mul_ps(swp02, swp03);
                _mm_sub_ps(mul00, mul01)
            };

            let fac5 = {
                let swp0a = _mm_shuffle_ps(self.w_axis.0, self.z_axis.0, 0b01_01_01_01);
                let swp0b = _mm_shuffle_ps(self.w_axis.0, self.z_axis.0, 0b00_00_00_00);
                let swp00 = _mm_shuffle_ps(self.z_axis.0, self.y_axis.0, 0b00_00_00_00);
                let swp01 = _mm_shuffle_ps(swp0a, swp0a, 0b10_00_00_00);
                let swp02 = _mm_shuffle_ps(swp0b, swp0b, 0b10_00_00_00);
                let swp03 = _mm_shuffle_ps(self.z_axis.0, self.y_axis.0, 0b01_01_01_01);
                let mul00 = _mm_mul_ps(swp00, swp01);
                let mul01 = _mm_mul_ps(swp02, swp03);
                _mm_sub_ps(mul00, mul01)
            };

            let sign_a = _mm_set_ps(1.0, -1.0, 1.0, -1.0);
            let sign_b = _mm_set_ps(-1.0, 1.0, -1.0, 1.0);

            let temp0 = _mm_shuffle_ps(self.y_axis.0, self.x_axis.0, 0b00_00_00_00);
            let vec0 = _mm_shuffle_ps(temp0, temp0, 0b10_10_10_00);
            let temp1 = _mm_shuffle_ps(self.y_axis.0, self.x_axis.0, 0b01_01_01_01);
            let vec1 = _mm_shuffle_ps(temp1, temp1, 0b10_10_10_00);
            let temp2 = _mm_shuffle_ps(self.y_axis.0, self.x_axis.0, 0b10_10_10_10);
            let vec2 = _mm_shuffle_ps(temp2, temp2, 0b10_10_10_00);
            let temp3 = _mm_shuffle_ps(self.y_axis.0, self.x_axis.0, 0b11_11_11_11);
            let vec3 = _mm_shuffle_ps(temp3, temp3, 0b10_10_10_00);

            let mul00 = _mm_mul_ps(vec1, fac0);
            let mul01 = _mm_mul_ps(vec2, fac1);
            let mul02 = _mm_mul_ps(vec3, fac2);
            let sub00 = _mm_sub_ps(mul00, mul01);
            let add00 = _mm_add_ps(sub00, mul02);
            let inv0 = _mm_mul_ps(sign_b, add00);

            let mul03 = _mm_mul_ps(vec0, fac0);
            let mul04 = _mm_mul_ps(vec2, fac3);
            let mul05 = _mm_mul_ps(vec3, fac4);
            let sub01 = _mm_sub_ps(mul03, mul04);
            let add01 = _mm_add_ps(sub01, mul05);
            let inv1 = _mm_mul_ps(sign_a, add01);

            let mul06 = _mm_mul_ps(vec0, fac1);
            let mul07 = _mm_mul_ps(vec1, fac3);
            let mul08 = _mm_mul_ps(vec3, fac5);
            let sub02 = _mm_sub_ps(mul06, mul07);
            let add02 = _mm_add_ps(sub02, mul08);
            let inv2 = _mm_mul_ps(sign_b, add02);

            let mul09 = _mm_mul_ps(vec0, fac2);
            let mul10 = _mm_mul_ps(vec1, fac4);
            let mul11 = _mm_mul_ps(vec2, fac5);
            let sub03 = _mm_sub_ps(mul09, mul10);
            let add03 = _mm_add_ps(sub03, mul11);
            let inv3 = _mm_mul_ps(sign_a, add03);

            let row0 = _mm_shuffle_ps(inv0, inv1, 0b00_00_00_00);
            let row1 = _mm_shuffle_ps(inv2, inv3, 0b00_00_00_00);
            let row2 = _mm_shuffle_ps(row0, row1, 0b10_00_10_00);
            let det = dot4_ps(self.x_axis.0, row2);

            if det.abs() < EPS {
                return None;
            }

            let inv_det = _mm_set1_ps(det.recip());
            Some(Self {
                x_axis: Vec4(_mm_mul_ps(inv0, inv_det)),
                y_axis: Vec4(_mm_mul_ps(inv1, inv_det)),
                z_axis: Vec4(_mm_mul_ps(inv2, inv_det)),
                w_axis: Vec4(_mm_mul_ps(inv3, inv_det)),
            })
        }

        #[cfg(not(all(target_arch = "x86_64", target_feature = "sse")))]
        {
            inverse_gauss_jordan(self)
        }
    }

    /// Converts the matrix to a column-major array of 16 elements.
    /// The array is ordered as: [m00, m10, m20, m30, m01, m11, m21, m31, m02, m12, m22, m32, m03, m13, m23, m33]
    #[inline]
    #[must_use]
    pub fn to_array(self) -> [f32; 16] {
        [
            self.x_axis.x(),
            self.x_axis.y(),
            self.x_axis.z(),
            self.x_axis.w(),
            self.y_axis.x(),
            self.y_axis.y(),
            self.y_axis.z(),
            self.y_axis.w(),
            self.z_axis.x(),
            self.z_axis.y(),
            self.z_axis.z(),
            self.z_axis.w(),
            self.w_axis.x(),
            self.w_axis.y(),
            self.w_axis.z(),
            self.w_axis.w(),
        ]
    }

    /// Returns a slice reference to the matrix data as a column-major array.
    #[inline]
    pub fn to_slice(&self) -> &[f32] {
        // Safety: Mat4 is repr(C) and contains 4 Vec4s which are each 4 f32s
        unsafe { core::slice::from_raw_parts(self as *const Self as *const f32, 16) }
    }

    /// Creates a matrix from a column-major array of 16 elements.
    /// The array should be ordered as: [m00, m10, m20, m30, m01, m11, m21, m31, m02, m12, m22, m32, m03, m13, m23, m33]
    #[inline]
    #[must_use]
    pub const fn from_array(arr: [f32; 16]) -> Self {
        Self {
            x_axis: Vec4::new(arr[0], arr[1], arr[2], arr[3]),
            y_axis: Vec4::new(arr[4], arr[5], arr[6], arr[7]),
            z_axis: Vec4::new(arr[8], arr[9], arr[10], arr[11]),
            w_axis: Vec4::new(arr[12], arr[13], arr[14], arr[15]),
        }
    }

    /// Creates a matrix from a slice.
    /// Panics if the slice has fewer than 16 elements.
    #[inline]
    #[must_use]
    pub fn from_slice(slice: &[f32]) -> Self {
        Self {
            x_axis: Vec4::new(slice[0], slice[1], slice[2], slice[3]),
            y_axis: Vec4::new(slice[4], slice[5], slice[6], slice[7]),
            z_axis: Vec4::new(slice[8], slice[9], slice[10], slice[11]),
            w_axis: Vec4::new(slice[12], slice[13], slice[14], slice[15]),
        }
    }

    /// Transposes the matrix.
    #[inline]
    #[must_use]
    pub fn transpose(self) -> Self {
        #[cfg(all(target_arch = "x86_64", target_feature = "sse"))]
        {
            unsafe {
                // Based on https://github.com/microsoft/DirectXMath `XMMatrixTranspose`
                let tmp0 = _mm_shuffle_ps(self.x_axis.0, self.y_axis.0, 0b01_00_01_00);
                let tmp1 = _mm_shuffle_ps(self.x_axis.0, self.y_axis.0, 0b11_10_11_10);
                let tmp2 = _mm_shuffle_ps(self.z_axis.0, self.w_axis.0, 0b01_00_01_00);
                let tmp3 = _mm_shuffle_ps(self.z_axis.0, self.w_axis.0, 0b11_10_11_10);

                Self {
                    x_axis: Vec4(_mm_shuffle_ps(tmp0, tmp2, 0b10_00_10_00)),
                    y_axis: Vec4(_mm_shuffle_ps(tmp0, tmp2, 0b11_01_11_01)),
                    z_axis: Vec4(_mm_shuffle_ps(tmp1, tmp3, 0b10_00_10_00)),
                    w_axis: Vec4(_mm_shuffle_ps(tmp1, tmp3, 0b11_01_11_01)),
                }
            }
        }
        #[cfg(not(all(target_arch = "x86_64", target_feature = "sse")))]
        {
            Self {
                x_axis: Vec4::new(
                    self.x_axis.x(),
                    self.y_axis.x(),
                    self.z_axis.x(),
                    self.w_axis.x(),
                ),
                y_axis: Vec4::new(
                    self.x_axis.y(),
                    self.y_axis.y(),
                    self.z_axis.y(),
                    self.w_axis.y(),
                ),
                z_axis: Vec4::new(
                    self.x_axis.z(),
                    self.y_axis.z(),
                    self.z_axis.z(),
                    self.w_axis.z(),
                ),
                w_axis: Vec4::new(
                    self.x_axis.w(),
                    self.y_axis.w(),
                    self.z_axis.w(),
                    self.w_axis.w(),
                ),
            }
        }
    }
}

impl Mul<Mat4> for Mat4 {
    type Output = Self;

    #[inline]
    fn mul(self, other: Self) -> Self {
        // Matrix multiplication: result[i] = self * other[i]
        // Each column of result is self * other's column
        Self {
            x_axis: self.mul_vec4(other.x_axis),
            y_axis: self.mul_vec4(other.y_axis),
            z_axis: self.mul_vec4(other.z_axis),
            w_axis: self.mul_vec4(other.w_axis),
        }
    }
}

impl Mul<Vec4> for Mat4 {
    type Output = Vec4;

    #[inline]
    fn mul(self, vec: Vec4) -> Vec4 {
        self.mul_vec4(vec)
    }
}

impl Mul<f32> for Mat4 {
    type Output = Self;

    #[inline]
    fn mul(self, scalar: f32) -> Self {
        #[cfg(all(target_arch = "x86_64", target_feature = "sse"))]
        {
            let scalar_vec = Vec4::splat(scalar);
            Self {
                x_axis: self.x_axis.mul_element_wise(scalar_vec),
                y_axis: self.y_axis.mul_element_wise(scalar_vec),
                z_axis: self.z_axis.mul_element_wise(scalar_vec),
                w_axis: self.w_axis.mul_element_wise(scalar_vec),
            }
        }
        #[cfg(not(all(target_arch = "x86_64", target_feature = "sse")))]
        {
            Self {
                x_axis: self.x_axis * scalar,
                y_axis: self.y_axis * scalar,
                z_axis: self.z_axis * scalar,
                w_axis: self.w_axis * scalar,
            }
        }
    }
}

impl MulAssign<f32> for Mat4 {
    #[inline]
    fn mul_assign(&mut self, scalar: f32) {
        *self = *self * scalar;
    }
}

impl Mat4 {
    /// Multiplies the matrix by a vector (optimized SSE version).
    #[inline]
    #[must_use]
    pub fn mul_vec4(self, vec: Vec4) -> Vec4 {
        #[cfg(all(target_arch = "x86_64", target_feature = "sse"))]
        {
            unsafe {
                // result = x_axis * vec.x + y_axis * vec.y + z_axis * vec.z + w_axis * vec.w
                // Use swizzle methods to broadcast components efficiently
                let vx = vec.xxxx().0;
                let vy = vec.yyyy().0;
                let vz = vec.zzzz().0;
                let vw = vec.wwww().0;

                let x_scaled = _mm_mul_ps(self.x_axis.0, vx);
                let y_scaled = _mm_mul_ps(self.y_axis.0, vy);
                let z_scaled = _mm_mul_ps(self.z_axis.0, vz);
                let w_scaled = _mm_mul_ps(self.w_axis.0, vw);

                let sum1 = _mm_add_ps(x_scaled, y_scaled);
                let sum2 = _mm_add_ps(z_scaled, w_scaled);
                Vec4(_mm_add_ps(sum1, sum2))
            }
        }
        #[cfg(not(all(target_arch = "x86_64", target_feature = "sse")))]
        {
            // Use swizzle methods to extract and broadcast components as scalars
            self.x_axis * vec.x()
                + self.y_axis * vec.y()
                + self.z_axis * vec.z()
                + self.w_axis * vec.w()
        }
    }
}

impl fmt::Display for Mat4 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[[{:.4}, {:.4}, {:.4}, {:.4}],\n [{:.4}, {:.4}, {:.4}, {:.4}],\n [{:.4}, {:.4}, {:.4}, {:.4}],\n [{:.4}, {:.4}, {:.4}, {:.4}]]",
            self.x_axis.x(),
            self.y_axis.x(),
            self.z_axis.x(),
            self.w_axis.x(),
            self.x_axis.y(),
            self.y_axis.y(),
            self.z_axis.y(),
            self.w_axis.y(),
            self.x_axis.z(),
            self.y_axis.z(),
            self.z_axis.z(),
            self.w_axis.z(),
            self.x_axis.w(),
            self.y_axis.w(),
            self.z_axis.w(),
            self.w_axis.w(),
        )
    }
}

#[cfg(not(all(target_arch = "x86_64", target_feature = "sse")))]
fn row_component(vec: &Vec4, row: usize) -> f32 {
    match row {
        0 => vec.x(),
        1 => vec.y(),
        2 => vec.z(),
        _ => vec.w(),
    }
}

#[cfg(not(all(target_arch = "x86_64", target_feature = "sse")))]
fn inverse_gauss_jordan(mat: Mat4) -> Option<Mat4> {
    const EPS: f32 = 1e-10;

    let det = mat.determinant();
    if det.abs() < EPS {
        return None;
    }

    let mut aug = [[0.0f32; 8]; 4];
    for row in 0..4 {
        aug[row][0] = row_component(&mat.x_axis, row);
        aug[row][1] = row_component(&mat.y_axis, row);
        aug[row][2] = row_component(&mat.z_axis, row);
        aug[row][3] = row_component(&mat.w_axis, row);
        aug[row][row + 4] = 1.0;
    }

    for col in 0..4 {
        let mut pivot_row = col;
        let mut max = aug[pivot_row][col].abs();
        for row in (col + 1)..4 {
            let value = aug[row][col].abs();
            if value > max {
                max = value;
                pivot_row = row;
            }
        }

        if max < EPS {
            return None;
        }

        if pivot_row != col {
            aug.swap(pivot_row, col);
        }

        let pivot = aug[col][col];
        for j in 0..8 {
            aug[col][j] /= pivot;
        }

        for row in 0..4 {
            if row == col {
                continue;
            }
            let factor = aug[row][col];
            if factor == 0.0 {
                continue;
            }
            for j in 0..8 {
                aug[row][j] -= factor * aug[col][j];
            }
        }
    }

    let x_axis = Vec4::new(aug[0][4], aug[1][4], aug[2][4], aug[3][4]);
    let y_axis = Vec4::new(aug[0][5], aug[1][5], aug[2][5], aug[3][5]);
    let z_axis = Vec4::new(aug[0][6], aug[1][6], aug[2][6], aug[3][6]);
    let w_axis = Vec4::new(aug[0][7], aug[1][7], aug[2][7], aug[3][7]);

    Some(Mat4::from_cols(x_axis, y_axis, z_axis, w_axis))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identity() {
        let m = Mat4::identity();
        assert_eq!(m.x_axis.x(), 1.0);
        assert_eq!(m.y_axis.y(), 1.0);
        assert_eq!(m.z_axis.z(), 1.0);
        assert_eq!(m.w_axis.w(), 1.0);
    }

    #[test]
    fn test_from_diagonal() {
        let diag = Vec4::new(2.0, 3.0, 4.0, 5.0);
        let m = Mat4::from_diagonal(diag);
        assert_eq!(m.x_axis.x(), 2.0);
        assert_eq!(m.y_axis.y(), 3.0);
        assert_eq!(m.z_axis.z(), 4.0);
        assert_eq!(m.w_axis.w(), 5.0);
    }

    #[test]
    fn test_determinant_identity() {
        let m = Mat4::identity();
        assert!((m.determinant() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_determinant_zero() {
        // Matrix with values [1,2,3,4,5,6,7,8,-1,-2,-3,-4,-5,-6,-7,-8]
        // This matrix is singular because z_axis = -x_axis and w_axis = -y_axis
        // Column-major layout: x_axis=[1,2,3,4], y_axis=[5,6,7,8], z_axis=[-1,-2,-3,-4], w_axis=[-5,-6,-7,-8]
        let m = Mat4::from_array([
            1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, -1.0, -2.0, -3.0, -4.0, -5.0, -6.0, -7.0, -8.0,
        ]);
        let det = m.determinant();
        assert!(
            (det - 0.0).abs() < 1e-5,
            "Expected determinant to be 0, got {}",
            det
        );
    }

    #[test]
    fn test_determinant_primes() {
        // Matrix with primes: 2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 47, 53, 59
        // Using from_array with the primes in sequential order (column-major storage):
        // The array [2,3,5,7, 11,13,17,19, 23,29,31,37, 41,47,53,59] is interpreted as:
        // Column 0: [2, 3, 5, 7]
        // Column 1: [11, 13, 17, 19]
        // Column 2: [23, 29, 31, 37]
        // Column 3: [41, 47, 53, 59]
        let m = Mat4::from_array([
            2.0, 3.0, 5.0, 7.0, 11.0, 13.0, 17.0, 19.0, 23.0, 29.0, 31.0, 37.0, 41.0, 47.0, 53.0,
            59.0,
        ]);
        let det = m.determinant();
        // The determinant of this matrix is 280
        assert!(
            (det - 280.0).abs() < 1e-5,
            "Expected determinant to be 280, got {}",
            det
        );
    }

    #[test]
    fn test_inverse_identity() {
        let m = Mat4::identity();
        let inv = m.inverse().unwrap();
        assert!((inv.x_axis.x() - 1.0).abs() < 1e-6);
        assert!((inv.y_axis.y() - 1.0).abs() < 1e-6);
        assert!((inv.z_axis.z() - 1.0).abs() < 1e-6);
        assert!((inv.w_axis.w() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_matrix_vector_multiplication() {
        let m = Mat4::identity();
        let v = Vec4::new(1.0, 2.0, 3.0, 4.0);
        let result = m * v;
        assert_eq!(result.x(), 1.0);
        assert_eq!(result.y(), 2.0);
        assert_eq!(result.z(), 3.0);
        assert_eq!(result.w(), 4.0);
    }

    #[test]
    fn test_matrix_multiplication() {
        let m1 = Mat4::identity();
        let m2 = Mat4::identity();
        let result = m1 * m2;
        assert!((result.x_axis.x() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_transpose() {
        // Create a matrix with unique values for each position
        // Matrix layout (column-major):
        // [ 1,  2,  3,  4]  <- x_axis (column 0)
        // [ 5,  6,  7,  8]  <- y_axis (column 1)
        // [ 9, 10, 11, 12]  <- z_axis (column 2)
        // [13, 14, 15, 16]  <- w_axis (column 3)
        let m = Mat4::from_cols(
            Vec4::new(1.0, 2.0, 3.0, 4.0),
            Vec4::new(5.0, 6.0, 7.0, 8.0),
            Vec4::new(9.0, 10.0, 11.0, 12.0),
            Vec4::new(13.0, 14.0, 15.0, 16.0),
        );
        let t = m.transpose();

        // After transpose, should be (row-major view):
        // [ 1,  5,  9, 13]
        // [ 2,  6, 10, 14]
        // [ 3,  7, 11, 15]
        // [ 4,  8, 12, 16]
        // But stored as columns:
        // x_axis = [1, 5, 9, 13]
        // y_axis = [2, 6, 10, 14]
        // z_axis = [3, 7, 11, 15]
        // w_axis = [4, 8, 12, 16]

        // Exhaustively test all 16 values
        assert_eq!(t.x_axis.x(), 1.0); // row 0, col 0
        assert_eq!(t.x_axis.y(), 5.0); // row 0, col 1
        assert_eq!(t.x_axis.z(), 9.0); // row 0, col 2
        assert_eq!(t.x_axis.w(), 13.0); // row 0, col 3

        assert_eq!(t.y_axis.x(), 2.0); // row 1, col 0
        assert_eq!(t.y_axis.y(), 6.0); // row 1, col 1
        assert_eq!(t.y_axis.z(), 10.0); // row 1, col 2
        assert_eq!(t.y_axis.w(), 14.0); // row 1, col 3

        assert_eq!(t.z_axis.x(), 3.0); // row 2, col 0
        assert_eq!(t.z_axis.y(), 7.0); // row 2, col 1
        assert_eq!(t.z_axis.z(), 11.0); // row 2, col 2
        assert_eq!(t.z_axis.w(), 15.0); // row 2, col 3

        assert_eq!(t.w_axis.x(), 4.0); // row 3, col 0
        assert_eq!(t.w_axis.y(), 8.0); // row 3, col 1
        assert_eq!(t.w_axis.z(), 12.0); // row 3, col 2
        assert_eq!(t.w_axis.w(), 16.0); // row 3, col 3

        // Also test using array methods
        let original_array = m.to_array();
        let transposed_array = t.to_array();

        // Verify transpose using array representation
        // Original: [1,2,3,4, 5,6,7,8, 9,10,11,12, 13,14,15,16] (column-major)
        // Transposed: [1,5,9,13, 2,6,10,14, 3,7,11,15, 4,8,12,16] (column-major)
        assert_eq!(transposed_array[0], original_array[0]); // m00 -> m00
        assert_eq!(transposed_array[1], original_array[4]); // m10 -> m01
        assert_eq!(transposed_array[2], original_array[8]); // m20 -> m02
        assert_eq!(transposed_array[3], original_array[12]); // m30 -> m03
        assert_eq!(transposed_array[4], original_array[1]); // m01 -> m10
        assert_eq!(transposed_array[5], original_array[5]); // m11 -> m11
        assert_eq!(transposed_array[6], original_array[9]); // m21 -> m12
        assert_eq!(transposed_array[7], original_array[13]); // m31 -> m13
        assert_eq!(transposed_array[8], original_array[2]); // m02 -> m20
        assert_eq!(transposed_array[9], original_array[6]); // m12 -> m21
        assert_eq!(transposed_array[10], original_array[10]); // m22 -> m22
        assert_eq!(transposed_array[11], original_array[14]); // m32 -> m23
        assert_eq!(transposed_array[12], original_array[3]); // m03 -> m30
        assert_eq!(transposed_array[13], original_array[7]); // m13 -> m31
        assert_eq!(transposed_array[14], original_array[11]); // m23 -> m32
        assert_eq!(transposed_array[15], original_array[15]); // m33 -> m33
    }

    #[test]
    fn test_to_array() {
        let m = Mat4::from_cols(
            Vec4::new(1.0, 2.0, 3.0, 4.0),
            Vec4::new(5.0, 6.0, 7.0, 8.0),
            Vec4::new(9.0, 10.0, 11.0, 12.0),
            Vec4::new(13.0, 14.0, 15.0, 16.0),
        );
        let arr = m.to_array();
        // Column-major order: [m00, m10, m20, m30, m01, m11, m21, m31, m02, m12, m22, m32, m03, m13, m23, m33]
        assert_eq!(
            arr,
            [
                1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0,
                16.0
            ]
        );
    }

    #[test]
    fn test_from_array() {
        let arr = [
            1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0,
        ];
        let m = Mat4::from_array(arr);
        assert_eq!(m.x_axis.x(), 1.0);
        assert_eq!(m.x_axis.y(), 2.0);
        assert_eq!(m.x_axis.z(), 3.0);
        assert_eq!(m.x_axis.w(), 4.0);
        assert_eq!(m.y_axis.x(), 5.0);
        assert_eq!(m.y_axis.y(), 6.0);
        assert_eq!(m.y_axis.z(), 7.0);
        assert_eq!(m.y_axis.w(), 8.0);
        assert_eq!(m.z_axis.x(), 9.0);
        assert_eq!(m.z_axis.y(), 10.0);
        assert_eq!(m.z_axis.z(), 11.0);
        assert_eq!(m.z_axis.w(), 12.0);
        assert_eq!(m.w_axis.x(), 13.0);
        assert_eq!(m.w_axis.y(), 14.0);
        assert_eq!(m.w_axis.z(), 15.0);
        assert_eq!(m.w_axis.w(), 16.0);

        // Round-trip test
        assert_eq!(m.to_array(), arr);
    }

    #[test]
    fn test_to_slice() {
        let m = Mat4::from_cols(
            Vec4::new(1.0, 2.0, 3.0, 4.0),
            Vec4::new(5.0, 6.0, 7.0, 8.0),
            Vec4::new(9.0, 10.0, 11.0, 12.0),
            Vec4::new(13.0, 14.0, 15.0, 16.0),
        );
        let slice = m.to_slice();
        assert_eq!(slice.len(), 16);
        assert_eq!(slice[0], 1.0);
        assert_eq!(slice[15], 16.0);
    }

    #[test]
    fn test_from_slice() {
        let arr = [
            1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0,
        ];
        let m = Mat4::from_slice(&arr);
        assert_eq!(m.x_axis.x(), 1.0);
        assert_eq!(m.w_axis.w(), 16.0);

        // Round-trip test
        let reconstructed = m.to_array();
        assert_eq!(reconstructed, arr);
    }

    #[test]
    fn test_from_angle_axis_zero() {
        // Zero rotation should give identity matrix
        let m = Mat4::from_angle_axis(0.0, Vec3::new(1.0, 0.0, 0.0));
        assert!((m.x_axis.x() - 1.0).abs() < 1e-6);
        assert!((m.y_axis.y() - 1.0).abs() < 1e-6);
        assert!((m.z_axis.z() - 1.0).abs() < 1e-6);
        assert!((m.w_axis.w() - 1.0).abs() < 1e-6);
        assert!((m.x_axis.y()).abs() < 1e-6);
        assert!((m.x_axis.z()).abs() < 1e-6);
    }

    #[test]
    fn test_from_angle_axis_x_90() {
        use std::f32::consts::PI;
        // 90 degree rotation around X axis
        let m = Mat4::from_angle_axis(PI / 2.0, Vec3::new(1.0, 0.0, 0.0));
        // Expected rotation matrix for 90° around X (column-major):
        // Column 0: [1, 0, 0]
        // Column 1: [0, 0, 1]
        // Column 2: [0, -1, 0]
        assert!((m.x_axis.x() - 1.0).abs() < 1e-6); // X axis unchanged
        assert!((m.y_axis.y()).abs() < 1e-6); // cos(90°) ≈ 0
        assert!((m.y_axis.z() - 1.0).abs() < 1e-6); // sin(90°) = 1
        assert!((m.z_axis.y() - (-1.0)).abs() < 1e-6); // -sin(90°) = -1
        assert!((m.z_axis.z()).abs() < 1e-6); // cos(90°) ≈ 0
    }

    #[test]
    fn test_from_angle_axis_y_90() {
        use std::f32::consts::PI;
        // 90 degree rotation around Y axis
        let m = Mat4::from_angle_axis(PI / 2.0, Vec3::new(0.0, 1.0, 0.0));
        // Verify it's a valid rotation matrix (orthogonal, determinant = 1)
        let det = m.determinant();
        assert!((det - 1.0).abs() < 1e-5);

        // Expected rotation matrix for 90° around Y (column-major):
        // Column 0: [0, 0, -1]
        // Column 1: [0, 1, 0]
        // Column 2: [1, 0, 0]
        assert!((m.x_axis.x()).abs() < 1e-6); // cos(90°) ≈ 0
        assert!((m.x_axis.z() - (-1.0)).abs() < 1e-6); // -sin(90°) = -1
        assert!((m.y_axis.y() - 1.0).abs() < 1e-6); // Y axis unchanged
        assert!((m.z_axis.x() - 1.0).abs() < 1e-6); // sin(90°) = 1
        assert!((m.z_axis.z()).abs() < 1e-6); // cos(90°) ≈ 0
    }

    #[test]
    fn test_from_angle_axis_z_90() {
        use std::f32::consts::PI;
        // 90 degree rotation around Z axis
        let m = Mat4::from_angle_axis(PI / 2.0, Vec3::new(0.0, 0.0, 1.0));

        // Verify it's a valid rotation matrix (orthogonal, determinant = 1)
        let det = m.determinant();
        assert!((det - 1.0).abs() < 1e-5);

        // Expected rotation matrix for 90° around Z (column-major):
        // Column 0: [0, 1, 0]
        // Column 1: [-1, 0, 0]
        // Column 2: [0, 0, 1]
        assert!((m.x_axis.x()).abs() < 1e-6); // cos(90°) ≈ 0
        assert!((m.x_axis.y() - 1.0).abs() < 1e-6); // sin(90°) = 1
        assert!((m.y_axis.x() - (-1.0)).abs() < 1e-6); // -sin(90°) = -1
        assert!((m.y_axis.y()).abs() < 1e-6); // cos(90°) ≈ 0
        assert!((m.z_axis.z() - 1.0).abs() < 1e-6); // Z axis unchanged
    }

    #[test]
    fn test_from_angle_axis_combined() {
        use std::f32::consts::PI;
        // Test rotation around an arbitrary axis
        let m = Mat4::from_angle_axis(PI / 4.0, Vec3::new(1.0, 1.0, 0.0).normalize());

        // Verify it's a valid rotation matrix (orthogonal, determinant = 1)
        let det = m.determinant();
        assert!((det - 1.0).abs() < 1e-5);

        // Verify the last row is [0, 0, 0, 1]
        assert!((m.x_axis.w()).abs() < 1e-6);
        assert!((m.y_axis.w()).abs() < 1e-6);
        assert!((m.z_axis.w()).abs() < 1e-6);
        assert!((m.w_axis.w() - 1.0).abs() < 1e-6);

        // Verify the last column is [0, 0, 0, 1]
        assert!((m.w_axis.x()).abs() < 1e-6);
        assert!((m.w_axis.y()).abs() < 1e-6);
        assert!((m.w_axis.z()).abs() < 1e-6);
    }

    #[test]
    fn test_inverse_angle_axis() {
        use std::f32::consts::PI;

        // Test that the inverse of a rotation matrix equals rotating by the negative angle
        let angle = PI / 3.0;
        let axis = Vec3::new(1.0, 2.0, 3.0).normalize();

        // Create rotation matrix
        let m = Mat4::from_angle_axis(angle, axis);

        // Compute inverse
        let inv = m.inverse().expect("Rotation matrix should be invertible");

        // Create rotation matrix with negative angle
        let m_neg = Mat4::from_angle_axis(-angle, axis);

        // They should be equal (or very close due to floating point precision)
        assert!((inv.x_axis.x() - m_neg.x_axis.x()).abs() < 1e-5);
        assert!((inv.x_axis.y() - m_neg.x_axis.y()).abs() < 1e-5);
        assert!((inv.x_axis.z() - m_neg.x_axis.z()).abs() < 1e-5);
        assert!((inv.y_axis.x() - m_neg.y_axis.x()).abs() < 1e-5);
        assert!((inv.y_axis.y() - m_neg.y_axis.y()).abs() < 1e-5);
        assert!((inv.y_axis.z() - m_neg.y_axis.z()).abs() < 1e-5);
        assert!((inv.z_axis.x() - m_neg.z_axis.x()).abs() < 1e-5);
        assert!((inv.z_axis.y() - m_neg.z_axis.y()).abs() < 1e-5);
        assert!((inv.z_axis.z() - m_neg.z_axis.z()).abs() < 1e-5);

        // Also test with different angles and axes
        let angle2 = PI / 6.0;
        let axis2 = Vec3::new(0.5, -0.5, 0.707).normalize();
        let m2 = Mat4::from_angle_axis(angle2, axis2);
        let inv2 = m2.inverse().expect("Rotation matrix should be invertible");
        let m2_neg = Mat4::from_angle_axis(-angle2, axis2);

        assert!((inv2.x_axis.x() - m2_neg.x_axis.x()).abs() < 1e-5);
        assert!((inv2.x_axis.y() - m2_neg.x_axis.y()).abs() < 1e-5);
        assert!((inv2.x_axis.z() - m2_neg.x_axis.z()).abs() < 1e-5);
        assert!((inv2.y_axis.x() - m2_neg.y_axis.x()).abs() < 1e-5);
        assert!((inv2.y_axis.y() - m2_neg.y_axis.y()).abs() < 1e-5);
        assert!((inv2.y_axis.z() - m2_neg.y_axis.z()).abs() < 1e-5);
        assert!((inv2.z_axis.x() - m2_neg.z_axis.x()).abs() < 1e-5);
        assert!((inv2.z_axis.y() - m2_neg.z_axis.y()).abs() < 1e-5);
        assert!((inv2.z_axis.z() - m2_neg.z_axis.z()).abs() < 1e-5);
    }

    #[test]
    fn test_from_angle_axis_unsafe() {
        use std::f32::consts::PI;

        // Test that the unsafe version produces the same results as the safe version
        // when given a normalized axis
        let angle = PI / 4.0;
        let axis_normalized = Vec3::new(1.0, 2.0, 3.0).normalize();

        let m_safe = Mat4::from_angle_axis(angle, axis_normalized);
        let m_unsafe = unsafe { Mat4::from_angle_axis_unsafe(angle, axis_normalized) };

        // They should be identical
        assert!((m_safe.x_axis.x() - m_unsafe.x_axis.x()).abs() < 1e-6);
        assert!((m_safe.x_axis.y() - m_unsafe.x_axis.y()).abs() < 1e-6);
        assert!((m_safe.x_axis.z() - m_unsafe.x_axis.z()).abs() < 1e-6);
        assert!((m_safe.y_axis.x() - m_unsafe.y_axis.x()).abs() < 1e-6);
        assert!((m_safe.y_axis.y() - m_unsafe.y_axis.y()).abs() < 1e-6);
        assert!((m_safe.y_axis.z() - m_unsafe.y_axis.z()).abs() < 1e-6);
        assert!((m_safe.z_axis.x() - m_unsafe.z_axis.x()).abs() < 1e-6);
        assert!((m_safe.z_axis.y() - m_unsafe.z_axis.y()).abs() < 1e-6);
        assert!((m_safe.z_axis.z() - m_unsafe.z_axis.z()).abs() < 1e-6);

        // Test with standard unit vectors
        let m_x_safe = Mat4::from_angle_axis(PI / 3.0, Vec3::new(1.0, 0.0, 0.0));
        let m_x_unsafe =
            unsafe { Mat4::from_angle_axis_unsafe(PI / 3.0, Vec3::new(1.0, 0.0, 0.0)) };
        assert!((m_x_safe.x_axis.x() - m_x_unsafe.x_axis.x()).abs() < 1e-6);
        assert!((m_x_safe.y_axis.y() - m_x_unsafe.y_axis.y()).abs() < 1e-6);
        assert!((m_x_safe.z_axis.z() - m_x_unsafe.z_axis.z()).abs() < 1e-6);
    }

    #[test]
    fn test_from_trs() {
        use crate::quat::Quat;
        use std::f32::consts::PI;

        // Test identity transform
        let translation = Vec3::zero();
        let rotation = Quat::identity();
        let scale = Vec3::one();
        let m = Mat4::from_trs(translation, rotation, scale);

        // Should be identity matrix
        let identity = Mat4::identity();
        assert!((m.x_axis.x() - identity.x_axis.x()).abs() < 1e-6);
        assert!((m.y_axis.y() - identity.y_axis.y()).abs() < 1e-6);
        assert!((m.z_axis.z() - identity.z_axis.z()).abs() < 1e-6);
        assert!((m.w_axis.w() - identity.w_axis.w()).abs() < 1e-6);

        // Test translation only
        let translation = Vec3::new(1.0, 2.0, 3.0);
        let rotation = Quat::identity();
        let scale = Vec3::one();
        let m = Mat4::from_trs(translation, rotation, scale);

        // Translation should be in w components of x_axis, y_axis, z_axis
        assert!((m.x_axis.w() - 1.0).abs() < 1e-6);
        assert!((m.y_axis.w() - 2.0).abs() < 1e-6);
        assert!((m.z_axis.w() - 3.0).abs() < 1e-6);
        // w_axis should be [0, 0, 0, 1]
        assert!((m.w_axis.x()).abs() < 1e-6);
        assert!((m.w_axis.y()).abs() < 1e-6);
        assert!((m.w_axis.z()).abs() < 1e-6);
        assert!((m.w_axis.w() - 1.0).abs() < 1e-6);

        // Test rotation only (90 degrees around Z axis)
        let translation = Vec3::zero();
        let rotation = Quat::from_rotation_z(PI / 2.0);
        let scale = Vec3::one();
        let m = Mat4::from_trs(translation, rotation, scale);

        // Verify rotation matrix components (90° rotation around Z)
        // x_axis should be [0, 1, 0]
        assert!((m.x_axis.x()).abs() < 1e-5);
        assert!((m.x_axis.y() - 1.0).abs() < 1e-5);
        assert!((m.x_axis.z()).abs() < 1e-5);
        // y_axis should be [-1, 0, 0]
        assert!((m.y_axis.x() - (-1.0)).abs() < 1e-5);
        assert!((m.y_axis.y()).abs() < 1e-5);
        assert!((m.y_axis.z()).abs() < 1e-5);

        // Test scale only
        let translation = Vec3::zero();
        let rotation = Quat::identity();
        let scale = Vec3::new(2.0, 3.0, 4.0);
        let m = Mat4::from_trs(translation, rotation, scale);

        // Scale should be applied to diagonal
        assert!((m.x_axis.x() - 2.0).abs() < 1e-6);
        assert!((m.y_axis.y() - 3.0).abs() < 1e-6);
        assert!((m.z_axis.z() - 4.0).abs() < 1e-6);

        // Test combined TRS
        let translation = Vec3::new(10.0, 20.0, 30.0);
        let rotation = Quat::from_rotation_x(PI / 4.0);
        let scale = Vec3::new(2.0, 2.0, 2.0);
        let m = Mat4::from_trs(translation, rotation, scale);

        // Verify translation is in w components of x_axis, y_axis, z_axis
        assert!((m.x_axis.w() - 10.0).abs() < 1e-5);
        assert!((m.y_axis.w() - 20.0).abs() < 1e-5);
        assert!((m.z_axis.w() - 30.0).abs() < 1e-5);

        // Verify w_axis is [0, 0, 0, 1]
        assert!((m.w_axis.x()).abs() < 1e-5);
        assert!((m.w_axis.y()).abs() < 1e-5);
        assert!((m.w_axis.z()).abs() < 1e-5);
        assert!((m.w_axis.w() - 1.0).abs() < 1e-5);

        // Verify scale is applied (rotation matrix columns should be scaled)
        // For a 45° rotation around X, the x_axis should be [1, 0, 0] scaled by 2
        assert!((m.x_axis.x() - 2.0).abs() < 1e-5);
        assert!((m.x_axis.y()).abs() < 1e-5);
        assert!((m.x_axis.z()).abs() < 1e-5);
    }
}
