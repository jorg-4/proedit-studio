//! Geometric primitives for 2D transformations.

use bytemuck::{Pod, Zeroable};
use glam::{Affine2, Mat3, Vec2 as GlamVec2};
use serde::{Deserialize, Serialize};

/// 2D vector.
pub type Vec2 = GlamVec2;

/// Axis-aligned rectangle.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize, Pod, Zeroable)]
#[repr(C)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    /// Create a new rectangle.
    #[inline]
    pub const fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Create a rectangle from two corners.
    pub fn from_corners(min: Vec2, max: Vec2) -> Self {
        Self {
            x: min.x,
            y: min.y,
            width: max.x - min.x,
            height: max.y - min.y,
        }
    }

    /// Create a rectangle from center and size.
    pub fn from_center_size(center: Vec2, size: Vec2) -> Self {
        Self {
            x: center.x - size.x * 0.5,
            y: center.y - size.y * 0.5,
            width: size.x,
            height: size.y,
        }
    }

    /// Minimum corner (top-left).
    #[inline]
    pub fn min(self) -> Vec2 {
        Vec2::new(self.x, self.y)
    }

    /// Maximum corner (bottom-right).
    #[inline]
    pub fn max(self) -> Vec2 {
        Vec2::new(self.x + self.width, self.y + self.height)
    }

    /// Center point.
    #[inline]
    pub fn center(self) -> Vec2 {
        Vec2::new(self.x + self.width * 0.5, self.y + self.height * 0.5)
    }

    /// Size as a vector.
    #[inline]
    pub fn size(self) -> Vec2 {
        Vec2::new(self.width, self.height)
    }

    /// Area of the rectangle.
    #[inline]
    pub fn area(self) -> f32 {
        self.width * self.height
    }

    /// Check if a point is inside the rectangle.
    #[inline]
    pub fn contains(self, point: Vec2) -> bool {
        point.x >= self.x
            && point.x < self.x + self.width
            && point.y >= self.y
            && point.y < self.y + self.height
    }

    /// Check if two rectangles overlap.
    pub fn overlaps(self, other: Self) -> bool {
        self.x < other.x + other.width
            && self.x + self.width > other.x
            && self.y < other.y + other.height
            && self.y + self.height > other.y
    }

    /// Compute intersection with another rectangle.
    pub fn intersection(self, other: Self) -> Option<Self> {
        let x1 = self.x.max(other.x);
        let y1 = self.y.max(other.y);
        let x2 = (self.x + self.width).min(other.x + other.width);
        let y2 = (self.y + self.height).min(other.y + other.height);

        if x1 < x2 && y1 < y2 {
            Some(Self::new(x1, y1, x2 - x1, y2 - y1))
        } else {
            None
        }
    }

    /// Compute union with another rectangle (bounding box).
    pub fn union(self, other: Self) -> Self {
        let x1 = self.x.min(other.x);
        let y1 = self.y.min(other.y);
        let x2 = (self.x + self.width).max(other.x + other.width);
        let y2 = (self.y + self.height).max(other.y + other.height);
        Self::new(x1, y1, x2 - x1, y2 - y1)
    }

    /// Expand the rectangle by a margin on all sides.
    pub fn expand(self, margin: f32) -> Self {
        Self::new(
            self.x - margin,
            self.y - margin,
            self.width + margin * 2.0,
            self.height + margin * 2.0,
        )
    }
}

/// 2D affine transformation matrix.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Transform2D {
    #[serde(skip)]
    inner: Affine2,
    // Store components for serialization
    translation: [f32; 2],
    rotation: f32,
    scale: [f32; 2],
}

impl Transform2D {
    /// Identity transform.
    pub const IDENTITY: Self = Self {
        inner: Affine2::IDENTITY,
        translation: [0.0, 0.0],
        rotation: 0.0,
        scale: [1.0, 1.0],
    };

    /// Create a translation transform.
    #[inline]
    pub fn translate(x: f32, y: f32) -> Self {
        Self {
            inner: Affine2::from_translation(Vec2::new(x, y)),
            translation: [x, y],
            rotation: 0.0,
            scale: [1.0, 1.0],
        }
    }

    /// Create a scale transform.
    #[inline]
    pub fn scale(x: f32, y: f32) -> Self {
        Self {
            inner: Affine2::from_scale(Vec2::new(x, y)),
            translation: [0.0, 0.0],
            rotation: 0.0,
            scale: [x, y],
        }
    }

    /// Create a uniform scale transform.
    #[inline]
    pub fn scale_uniform(s: f32) -> Self {
        Self::scale(s, s)
    }

    /// Create a rotation transform (radians).
    #[inline]
    pub fn rotate(angle: f32) -> Self {
        Self {
            inner: Affine2::from_angle(angle),
            translation: [0.0, 0.0],
            rotation: angle,
            scale: [1.0, 1.0],
        }
    }

    /// Create a transform from translation, rotation (radians), and scale.
    pub fn from_trs(translation: Vec2, rotation: f32, scale: Vec2) -> Self {
        let inner = Affine2::from_scale_angle_translation(scale, rotation, translation);
        Self {
            inner,
            translation: [translation.x, translation.y],
            rotation,
            scale: [scale.x, scale.y],
        }
    }

    /// Combine two transforms (self * other).
    #[inline]
    pub fn then(self, other: Self) -> Self {
        Self {
            inner: self.inner * other.inner,
            // Combined transform - store approximate values
            translation: [
                self.translation[0] + other.translation[0],
                self.translation[1] + other.translation[1],
            ],
            rotation: self.rotation + other.rotation,
            scale: [
                self.scale[0] * other.scale[0],
                self.scale[1] * other.scale[1],
            ],
        }
    }

    /// Transform a point.
    #[inline]
    pub fn transform_point(self, point: Vec2) -> Vec2 {
        self.inner.transform_point2(point)
    }

    /// Transform a vector (ignores translation).
    #[inline]
    pub fn transform_vector(self, vec: Vec2) -> Vec2 {
        self.inner.transform_vector2(vec)
    }

    /// Get the inverse transform.
    #[inline]
    pub fn inverse(self) -> Self {
        Self {
            inner: self.inner.inverse(),
            translation: [-self.translation[0], -self.translation[1]],
            rotation: -self.rotation,
            scale: [
                if self.scale[0] != 0.0 {
                    1.0 / self.scale[0]
                } else {
                    0.0
                },
                if self.scale[1] != 0.0 {
                    1.0 / self.scale[1]
                } else {
                    0.0
                },
            ],
        }
    }

    /// Convert to a 3x3 matrix for GPU upload.
    pub fn to_mat3(self) -> Mat3 {
        self.inner.into()
    }

    /// Get translation component.
    pub fn get_translation(&self) -> Vec2 {
        Vec2::new(self.translation[0], self.translation[1])
    }

    /// Get rotation component (radians).
    pub fn get_rotation(&self) -> f32 {
        self.rotation
    }

    /// Get scale component.
    pub fn get_scale(&self) -> Vec2 {
        Vec2::new(self.scale[0], self.scale[1])
    }
}

impl Default for Transform2D {
    fn default() -> Self {
        Self::IDENTITY
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rect_contains() {
        let rect = Rect::new(0.0, 0.0, 100.0, 100.0);
        assert!(rect.contains(Vec2::new(50.0, 50.0)));
        assert!(!rect.contains(Vec2::new(150.0, 50.0)));
    }

    #[test]
    fn test_rect_intersection() {
        let a = Rect::new(0.0, 0.0, 100.0, 100.0);
        let b = Rect::new(50.0, 50.0, 100.0, 100.0);
        let i = a.intersection(b).unwrap();
        assert_eq!(i.x, 50.0);
        assert_eq!(i.y, 50.0);
        assert_eq!(i.width, 50.0);
        assert_eq!(i.height, 50.0);
    }

    #[test]
    fn test_transform_translate() {
        let t = Transform2D::translate(10.0, 20.0);
        let p = t.transform_point(Vec2::new(5.0, 5.0));
        assert!((p.x - 15.0).abs() < 0.001);
        assert!((p.y - 25.0).abs() < 0.001);
    }

    #[test]
    fn test_transform_scale() {
        let t = Transform2D::scale(2.0, 3.0);
        let p = t.transform_point(Vec2::new(10.0, 10.0));
        assert!((p.x - 20.0).abs() < 0.001);
        assert!((p.y - 30.0).abs() < 0.001);
    }
}
