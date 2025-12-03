use crate::vec3::Vec3;
use core::fmt;
use core::ops::Mul;

/// A quaternion representing a rotation.
/// Stored as (x, y, z, w) where w is the scalar part and (x, y, z) is the vector part.
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Quat {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl Default for Quat {
    #[inline(always)]
    fn default() -> Self {
        Self::identity()
    }
}

impl Quat {
    /// Creates a quaternion from components.
    #[inline(always)]
    pub const fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self { x, y, z, w }
    }

    /// Creates the identity quaternion (no rotation).
    #[inline(always)]
    pub const fn identity() -> Self {
        Self::new(0.0, 0.0, 0.0, 1.0)
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

    /// Computes the conjugate of the quaternion (inverse rotation).
    #[inline(always)]
    #[must_use]
    pub fn conjugate(self) -> Self {
        Self::new(-self.x, -self.y, -self.z, self.w)
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
                -self.x * inv_norm_sq,
                -self.y * inv_norm_sq,
                -self.z * inv_norm_sq,
                self.w * inv_norm_sq,
            )
        } else {
            Self::identity()
        }
    }

    /// Computes the squared length of the quaternion.
    #[inline(always)]
    pub fn length_squared(self) -> f32 {
        self.x * self.x + self.y * self.y + self.z * self.z + self.w * self.w
    }

    /// Computes the length of the quaternion.
    #[inline]
    pub fn length(self) -> f32 {
        self.length_squared().sqrt()
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

    /// Rotates a 3D vector by this quaternion.
    /// This is equivalent to `q * v * q^-1` where v is treated as a pure quaternion.
    /// For unit quaternions, this is equivalent to `q * v * q.conjugate()`.
    ///
    /// Uses the formula: v' = v + 2 * (q.w * cross(q.xyz, v) + cross(q.xyz, cross(q.xyz, v)))
    #[inline]
    #[must_use]
    pub fn transform3(self, vec: Vec3) -> Vec3 {
        // More efficient formula for unit quaternions:
        // v' = v + 2 * (q.w * cross(q.xyz, v) + cross(q.xyz, cross(q.xyz, v)))
        let q_xyz = Vec3::new(self.x, self.y, self.z);
        let t = q_xyz.cross(vec);
        let result = vec + (t * self.w + q_xyz.cross(t)) * 2.0;
        result
    }
}

impl Mul<Quat> for Quat {
    type Output = Self;

    /// Multiplies two quaternions (combines rotations).
    /// The result represents the rotation of `self` followed by `other`.
    #[inline]
    fn mul(self, other: Self) -> Self {
        Self::new(
            self.w * other.x + self.x * other.w + self.y * other.z - self.z * other.y,
            self.w * other.y - self.x * other.z + self.y * other.w + self.z * other.x,
            self.w * other.z + self.x * other.y - self.y * other.x + self.z * other.w,
            self.w * other.w - self.x * other.x - self.y * other.y - self.z * other.z,
        )
    }
}

impl Mul<Vec3> for Quat {
    type Output = Vec3;

    /// Rotates a 3D vector by this quaternion.
    /// This allows using the `*` operator: `quat * vec3`.
    #[inline]
    fn mul(self, vec: Vec3) -> Vec3 {
        self.transform3(vec)
    }
}

impl fmt::Display for Quat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Quat({:.4}, {:.4}, {:.4}, {:.4})",
            self.x, self.y, self.z, self.w
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    #[test]
    fn test_identity() {
        let q = Quat::identity();
        assert_eq!(q.x, 0.0);
        assert_eq!(q.y, 0.0);
        assert_eq!(q.z, 0.0);
        assert_eq!(q.w, 1.0);
    }

    #[test]
    fn test_equality() {
        // Identity quaternion equals itself
        let id1 = Quat::identity();
        let id2 = Quat::identity();
        assert_eq!(id1, id2);
        assert_eq!(id1, id1);

        // Same quaternion equals itself
        let q1 = Quat::new(1.0, 2.0, 3.0, 4.0);
        let q2 = Quat::new(1.0, 2.0, 3.0, 4.0);
        assert_eq!(q1, q2);
        assert_eq!(q1, q1);

        // Quaternions created the same way are equal
        let q3 = Quat::from_rotation_x(PI / 4.0);
        let q4 = Quat::from_rotation_x(PI / 4.0);
        assert_eq!(q3, q4);

        // Different quaternions are not equal
        let q5 = Quat::new(1.0, 2.0, 3.0, 4.0);
        let q6 = Quat::new(1.0, 2.0, 3.0, 5.0);
        assert_ne!(q5, q6);

        let q7 = Quat::new(1.0, 2.0, 3.0, 4.0);
        let q8 = Quat::new(1.0, 2.0, 4.0, 4.0);
        assert_ne!(q7, q8);

        let q9 = Quat::new(1.0, 2.0, 3.0, 4.0);
        let q10 = Quat::new(1.0, 3.0, 3.0, 4.0);
        assert_ne!(q9, q10);

        let q11 = Quat::new(1.0, 2.0, 3.0, 4.0);
        let q12 = Quat::new(2.0, 2.0, 3.0, 4.0);
        assert_ne!(q11, q12);

        // Identity is not equal to non-identity
        let id = Quat::identity();
        let non_id = Quat::new(0.5, 0.5, 0.5, 0.5);
        assert_ne!(id, non_id);

        // Zero quaternion (if we create one)
        let zero1 = Quat::new(0.0, 0.0, 0.0, 0.0);
        let zero2 = Quat::new(0.0, 0.0, 0.0, 0.0);
        assert_eq!(zero1, zero2);

        // Different rotation quaternions are not equal
        let rot_x = Quat::from_rotation_x(PI / 2.0);
        let rot_y = Quat::from_rotation_y(PI / 2.0);
        assert_ne!(rot_x, rot_y);

        // Quaternion and its conjugate are not equal (unless it's identity)
        let q = Quat::from_rotation_x(PI / 4.0);
        let conj = q.conjugate();
        assert_ne!(q, conj);
    }

    #[test]
    fn test_from_angle_axis() {
        let q = Quat::from_angle_axis(PI / 2.0, Vec3::new(1.0, 0.0, 0.0));
        assert!((q.w - (PI / 4.0).cos()).abs() < 1e-6);
        assert!((q.x - (PI / 4.0).sin()).abs() < 1e-6);
        assert!(q.y.abs() < 1e-6);
        assert!(q.z.abs() < 1e-6);
    }

    #[test]
    fn test_from_angle_axis_unsafe() {
        let axis = Vec3::new(1.0, 0.0, 0.0);
        let q_safe = Quat::from_angle_axis(PI / 2.0, axis);
        let q_unsafe = unsafe { Quat::from_angle_axis_unsafe(PI / 2.0, axis) };
        assert!((q_safe.w - q_unsafe.w).abs() < 1e-6);
        assert!((q_safe.x - q_unsafe.x).abs() < 1e-6);
        assert!((q_safe.y - q_unsafe.y).abs() < 1e-6);
        assert!((q_safe.z - q_unsafe.z).abs() < 1e-6);
    }

    #[test]
    fn test_from_rotation_x() {
        let q = Quat::from_rotation_x(PI / 2.0);
        assert!((q.w - (PI / 4.0).cos()).abs() < 1e-6);
        assert!((q.x - (PI / 4.0).sin()).abs() < 1e-6);
        assert!(q.y.abs() < 1e-6);
        assert!(q.z.abs() < 1e-6);
    }

    #[test]
    fn test_from_rotation_y() {
        let q = Quat::from_rotation_y(PI / 2.0);
        assert!((q.w - (PI / 4.0).cos()).abs() < 1e-6);
        assert!(q.x.abs() < 1e-6);
        assert!((q.y - (PI / 4.0).sin()).abs() < 1e-6);
        assert!(q.z.abs() < 1e-6);
    }

    #[test]
    fn test_from_rotation_z() {
        let q = Quat::from_rotation_z(PI / 2.0);
        assert!((q.w - (PI / 4.0).cos()).abs() < 1e-6);
        assert!(q.x.abs() < 1e-6);
        assert!(q.y.abs() < 1e-6);
        assert!((q.z - (PI / 4.0).sin()).abs() < 1e-6);
    }

    #[test]
    fn test_from_euler_angles() {
        let q = Quat::from_euler_angles(0.0, 0.0, 0.0);
        assert!((q.w - 1.0).abs() < 1e-6);
        assert!(q.x.abs() < 1e-6);
        assert!(q.y.abs() < 1e-6);
        assert!(q.z.abs() < 1e-6);

        let q = Quat::from_euler_angles(PI / 2.0, 0.0, 0.0);
        let expected = Quat::from_rotation_x(PI / 2.0);
        assert!((q.w - expected.w).abs() < 1e-6);
        assert!((q.x - expected.x).abs() < 1e-6);
        assert!((q.y - expected.y).abs() < 1e-6);
        assert!((q.z - expected.z).abs() < 1e-6);

        let q = Quat::from_euler_angles(0.0, PI / 2.0, 0.0);
        let expected = Quat::from_rotation_y(PI / 2.0);
        assert!((q.w - expected.w).abs() < 1e-6);
        assert!((q.x - expected.x).abs() < 1e-6);
        assert!((q.y - expected.y).abs() < 1e-6);
        assert!((q.z - expected.z).abs() < 1e-6);

        let q = Quat::from_euler_angles(0.0, 0.0, PI / 2.0);
        let expected = Quat::from_rotation_z(PI / 2.0);
        assert!((q.w - expected.w).abs() < 1e-6);
        assert!((q.x - expected.x).abs() < 1e-6);
        assert!((q.y - expected.y).abs() < 1e-6);
        assert!((q.z - expected.z).abs() < 1e-6);

        let q = Quat::from_euler_angles(PI, 3.0 * PI / 4.0, -PI);
        let expected = Quat::from_rotation_z(-PI)
            * Quat::from_rotation_y(3.0 * PI / 4.0)
            * Quat::from_rotation_x(PI);
        assert!((q.w - expected.w).abs() < 1e-6);
        assert!((q.x - expected.x).abs() < 1e-6);
        assert!((q.y - expected.y).abs() < 1e-6);
        assert!((q.z - expected.z).abs() < 1e-6);
    }

    #[test]
    fn test_conjugate() {
        let q = Quat::new(2.0, 3.0, 4.0, 1.0);
        let conj = q.conjugate();
        assert_eq!(conj.w, 1.0);
        assert_eq!(conj.x, -2.0);
        assert_eq!(conj.y, -3.0);
        assert_eq!(conj.z, -4.0);
    }

    #[test]
    fn test_inverse() {
        let q = Quat::from_rotation_x(PI / 4.0);
        let inv = q.inverse();
        let product = q * inv;
        // q * q^-1 should be identity
        assert!((product.w - 1.0).abs() < 1e-5);
        assert!(product.x.abs() < 1e-5);
        assert!(product.y.abs() < 1e-5);
        assert!(product.z.abs() < 1e-5);
    }

    #[test]
    fn test_multiplication() {
        let q1 = Quat::from_rotation_x(PI / 2.0);
        let q2 = Quat::from_rotation_x(-PI / 2.0);
        let product = q1 * q2;

        // Verify it's a valid quaternion (unit length for rotation quaternions)
        let len_sq = product.length_squared();
        assert!((len_sq - 1.0).abs() < 1e-5);
        assert!((product.w - 1.0).abs() < 1e-5);
        assert!(product.x.abs() < 1e-5);
        assert!(product.y.abs() < 1e-5);
        assert!(product.z.abs() < 1e-5);

        let q1 = Quat::from_angle_axis(PI / 2.0, Vec3::new(1.0, 1.0, 1.0));
        let q2 = Quat::from_angle_axis(-PI / 2.0, Vec3::new(1.0, 1.0, 1.0));
        let product = q1 * q2;

        // Verify it's a valid quaternion (unit length for rotation quaternions)
        let len_sq = product.length_squared();
        assert!((len_sq - 1.0).abs() < 1e-5);
        assert!((product.w - 1.0).abs() < 1e-5);
        assert!(product.x.abs() < 1e-5);
        assert!(product.y.abs() < 1e-5);
        assert!(product.z.abs() < 1e-5);
    }

    #[test]
    fn test_multiplication_identity() {
        let q = Quat::from_rotation_x(PI / 4.0);
        let id = Quat::identity();
        let result = q * id;

        let len_sq = result.length_squared();
        assert!((len_sq - 1.0).abs() < 1e-5);
        assert!((result.w - q.w).abs() < 1e-6);
        assert!((result.x - q.x).abs() < 1e-6);
        assert!((result.y - q.y).abs() < 1e-6);
        assert!((result.z - q.z).abs() < 1e-6);
    }

    #[test]
    fn test_rotate_x() {
        let q = Quat::identity();
        let rotated = q.rotate_x(PI / 2.0);
        let expected = Quat::from_rotation_x(PI / 2.0);
        assert!((rotated.w - expected.w).abs() < 1e-6);
        assert!((rotated.x - expected.x).abs() < 1e-6);
        assert!((rotated.y - expected.y).abs() < 1e-6);
        assert!((rotated.z - expected.z).abs() < 1e-6);
    }

    #[test]
    fn test_rotate_y() {
        let q = Quat::identity();
        let rotated = q.rotate_y(PI / 2.0);
        let expected = Quat::from_rotation_y(PI / 2.0);
        assert!((rotated.w - expected.w).abs() < 1e-6);
        assert!((rotated.x - expected.x).abs() < 1e-6);
        assert!((rotated.y - expected.y).abs() < 1e-6);
        assert!((rotated.z - expected.z).abs() < 1e-6);
    }

    #[test]
    fn test_rotate_z() {
        let q = Quat::identity();
        let rotated = q.rotate_z(PI / 2.0);
        let expected = Quat::from_rotation_z(PI / 2.0);
        assert!((rotated.w - expected.w).abs() < 1e-6);
        assert!((rotated.x - expected.x).abs() < 1e-6);
        assert!((rotated.y - expected.y).abs() < 1e-6);
        assert!((rotated.z - expected.z).abs() < 1e-6);
    }

    #[test]
    fn test_rotate_axis() {
        let q = Quat::identity();
        let axis = Vec3::new(1.0, 0.0, 0.0);
        let rotated = q.rotate_axis(PI / 2.0, axis);
        let expected = Quat::from_angle_axis(PI / 2.0, axis);
        assert!((rotated.w - expected.w).abs() < 1e-6);
        assert!((rotated.x - expected.x).abs() < 1e-6);
        assert!((rotated.y - expected.y).abs() < 1e-6);
        assert!((rotated.z - expected.z).abs() < 1e-6);
    }

    #[test]
    fn test_rotate_axis_unsafe() {
        let q = Quat::identity();
        let axis = Vec3::new(1.0, 0.0, 0.0);
        let rotated = unsafe { q.rotate_axis_unsafe(PI / 2.0, axis) };
        let expected = unsafe { Quat::from_angle_axis_unsafe(PI / 2.0, axis) };
        assert!((rotated.w - expected.w).abs() < 1e-6);
        assert!((rotated.x - expected.x).abs() < 1e-6);
        assert!((rotated.y - expected.y).abs() < 1e-6);
        assert!((rotated.z - expected.z).abs() < 1e-6);
    }

    #[test]
    fn test_transform3() {
        use std::f32::consts::PI;

        // Test identity quaternion (no rotation)
        let q = Quat::identity();
        let v = Vec3::new(1.0, 2.0, 3.0);
        let result = q.transform3(v);
        assert!((result.x - v.x).abs() < 1e-6);
        assert!((result.y - v.y).abs() < 1e-6);
        assert!((result.z - v.z).abs() < 1e-6);

        // Test 90 degree rotation around Z axis
        // This should rotate (1, 0, 0) to (0, 1, 0)
        let q = Quat::from_rotation_z(PI / 2.0);
        let v = Vec3::new(1.0, 0.0, 0.0);
        let result = q.transform3(v);
        assert!((result.x).abs() < 1e-5);
        assert!((result.y - 1.0).abs() < 1e-5);
        assert!((result.z).abs() < 1e-5);

        // Test 90 degree rotation around X axis
        // This should rotate (0, 1, 0) to (0, 0, 1)
        let q = Quat::from_rotation_x(PI / 2.0);
        let v = Vec3::new(0.0, 1.0, 0.0);
        let result = q.transform3(v);
        assert!((result.x).abs() < 1e-5);
        assert!((result.y).abs() < 1e-5);
        assert!((result.z - 1.0).abs() < 1e-5);

        // Test 90 degree rotation around Y axis
        // This should rotate (0, 0, 1) to (1, 0, 0)
        let q = Quat::from_rotation_y(PI / 2.0);
        let v = Vec3::new(0.0, 0.0, 1.0);
        let result = q.transform3(v);
        assert!((result.x - 1.0).abs() < 1e-5);
        assert!((result.y).abs() < 1e-5);
        assert!((result.z).abs() < 1e-5);

        // Test that transform3 preserves vector length for rotations
        let q = Quat::from_rotation_x(PI / 4.0);
        let v = Vec3::new(1.0, 2.0, 3.0);
        let result = q.transform3(v);
        let original_len = v.length();
        let result_len = result.length();
        assert!((original_len - result_len).abs() < 1e-5);
    }

    #[test]
    fn test_mul_vec3() {
        use std::f32::consts::PI;

        // Test that Mul<Vec3> works the same as transform3
        let q = Quat::from_rotation_z(PI / 2.0);
        let v = Vec3::new(1.0, 0.0, 0.0);

        let result1 = q.transform3(v);
        let result2 = q * v;

        assert!((result1.x - result2.x).abs() < 1e-6);
        assert!((result1.y - result2.y).abs() < 1e-6);
        assert!((result1.z - result2.z).abs() < 1e-6);

        // Test with identity
        let q = Quat::identity();
        let v = Vec3::new(1.0, 2.0, 3.0);
        let result = q * v;
        assert!((result.x - v.x).abs() < 1e-6);
        assert!((result.y - v.y).abs() < 1e-6);
        assert!((result.z - v.z).abs() < 1e-6);
    }

    #[test]
    fn test_rotate_chained() {
        // Test chaining multiple rotations
        let q = Quat::identity();
        let rotated = q.rotate_x(PI / 4.0).rotate_y(PI / 6.0).rotate_z(PI / 3.0);

        // Verify the chained rotation produces a different quaternion
        // (quaternion multiplication of unit quaternions should preserve unit length,
        // but we're just verifying the operations work correctly)
        let single_rotation = Quat::from_rotation_x(PI / 4.0);
        assert!(
            (rotated.w - single_rotation.w).abs() > 1e-3
                || (rotated.x - single_rotation.x).abs() > 1e-3
                || (rotated.y - single_rotation.y).abs() > 1e-3
                || (rotated.z - single_rotation.z).abs() > 1e-3,
            "Chained rotations should produce different result than single rotation"
        );

        // Verify it's not identity
        assert!(
            (rotated.w - 1.0).abs() > 1e-3
                || rotated.x.abs() > 1e-3
                || rotated.y.abs() > 1e-3
                || rotated.z.abs() > 1e-3
        );
    }
}
