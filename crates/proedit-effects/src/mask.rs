//! Bezier mask paths for rotoscoping and shape masking.

use serde::{Deserialize, Serialize};

/// A vertex in a mask path with tangent handles for cubic Bezier curves.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaskVertex {
    /// Position in normalized coordinates [0,1] or pixel coordinates.
    pub position: [f32; 2],
    /// Per-vertex feather amount (pixels).
    pub feather: f32,
    /// Incoming tangent handle (relative to position).
    pub tangent_in: [f32; 2],
    /// Outgoing tangent handle (relative to position).
    pub tangent_out: [f32; 2],
}

impl MaskVertex {
    /// Create a simple vertex with no tangents.
    pub fn new(x: f32, y: f32) -> Self {
        Self {
            position: [x, y],
            feather: 0.0,
            tangent_in: [0.0, 0.0],
            tangent_out: [0.0, 0.0],
        }
    }
}

/// A closed or open mask path.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaskPath {
    pub vertices: Vec<MaskVertex>,
    pub closed: bool,
    pub inverted: bool,
    pub opacity: f32,
    /// Global feather multiplier.
    pub feather_falloff: f32,
}

impl Default for MaskPath {
    fn default() -> Self {
        Self {
            vertices: Vec::new(),
            closed: true,
            inverted: false,
            opacity: 1.0,
            feather_falloff: 1.0,
        }
    }
}

impl MaskPath {
    /// Sample a cubic Bezier curve between two vertices at parameter t.
    pub fn sample_bezier(v0: &MaskVertex, v1: &MaskVertex, t: f32) -> [f32; 2] {
        let p0 = v0.position;
        let p1 = [
            v0.position[0] + v0.tangent_out[0],
            v0.position[1] + v0.tangent_out[1],
        ];
        let p2 = [
            v1.position[0] + v1.tangent_in[0],
            v1.position[1] + v1.tangent_in[1],
        ];
        let p3 = v1.position;

        let mt = 1.0 - t;
        let mt2 = mt * mt;
        let mt3 = mt2 * mt;
        let t2 = t * t;
        let t3 = t2 * t;

        [
            mt3 * p0[0] + 3.0 * mt2 * t * p1[0] + 3.0 * mt * t2 * p2[0] + t3 * p3[0],
            mt3 * p0[1] + 3.0 * mt2 * t * p1[1] + 3.0 * mt * t2 * p2[1] + t3 * p3[1],
        ]
    }

    /// Compute the bounding box of the mask path.
    pub fn bounding_box(&self) -> ([f32; 2], [f32; 2]) {
        if self.vertices.is_empty() {
            return ([0.0, 0.0], [0.0, 0.0]);
        }

        let mut min_x = f32::MAX;
        let mut min_y = f32::MAX;
        let mut max_x = f32::MIN;
        let mut max_y = f32::MIN;

        let n = self.vertices.len();
        let segments = if self.closed { n } else { n.saturating_sub(1) };

        for seg in 0..segments {
            let v0 = &self.vertices[seg];
            let v1 = &self.vertices[(seg + 1) % n];
            // Sample the curve at multiple points
            for step in 0..=16 {
                let t = step as f32 / 16.0;
                let p = Self::sample_bezier(v0, v1, t);
                min_x = min_x.min(p[0]);
                min_y = min_y.min(p[1]);
                max_x = max_x.max(p[0]);
                max_y = max_y.max(p[1]);
            }
        }

        ([min_x, min_y], [max_x, max_y])
    }

    /// Test if a point is inside the mask path using ray casting.
    pub fn contains_point(&self, point: [f32; 2]) -> bool {
        if self.vertices.len() < 3 || !self.closed {
            return false;
        }

        let n = self.vertices.len();
        let mut crossings = 0;
        let steps = 32;

        for seg in 0..n {
            let v0 = &self.vertices[seg];
            let v1 = &self.vertices[(seg + 1) % n];

            for step in 0..steps {
                let t0 = step as f32 / steps as f32;
                let t1 = (step + 1) as f32 / steps as f32;
                let p0 = Self::sample_bezier(v0, v1, t0);
                let p1 = Self::sample_bezier(v0, v1, t1);

                // Ray casting: count horizontal ray crossings
                if (p0[1] <= point[1] && p1[1] > point[1])
                    || (p1[1] <= point[1] && p0[1] > point[1])
                {
                    let t = (point[1] - p0[1]) / (p1[1] - p0[1]);
                    let x_intersect = p0[0] + t * (p1[0] - p0[0]);
                    if point[0] < x_intersect {
                        crossings += 1;
                    }
                }
            }
        }

        let inside = crossings % 2 == 1;
        if self.inverted {
            !inside
        } else {
            inside
        }
    }

    /// Rasterize the mask to a float alpha matte.
    pub fn rasterize(&self, w: u32, h: u32) -> Vec<f32> {
        let size = (w * h) as usize;
        let mut matte = vec![0.0f32; size];

        if self.vertices.len() < 3 {
            return matte;
        }

        let max_feather = self
            .vertices
            .iter()
            .map(|v| v.feather)
            .fold(0.0f32, f32::max)
            * self.feather_falloff;

        for y in 0..h {
            for x in 0..w {
                let px = x as f32 + 0.5;
                let py = y as f32 + 0.5;

                if self.contains_point([px, py]) {
                    let mut alpha = self.opacity;

                    // Apply feathering based on distance to edge
                    if max_feather > 0.5 {
                        let dist = self.distance_to_edge([px, py]);
                        if dist < max_feather {
                            alpha *= dist / max_feather;
                        }
                    }

                    matte[(y * w + x) as usize] = alpha;
                } else if max_feather > 0.5 {
                    // Outside but within feather distance
                    let dist = self.distance_to_edge([px, py]);
                    if dist < max_feather {
                        let alpha = self.opacity * (1.0 - dist / max_feather);
                        matte[(y * w + x) as usize] = alpha;
                    }
                }
            }
        }

        if self.inverted {
            for v in matte.iter_mut() {
                *v = self.opacity - *v;
            }
        }

        matte
    }

    /// Approximate distance from a point to the nearest edge of the path.
    fn distance_to_edge(&self, point: [f32; 2]) -> f32 {
        let n = self.vertices.len();
        let segments = if self.closed { n } else { n.saturating_sub(1) };
        let mut min_dist = f32::MAX;

        for seg in 0..segments {
            let v0 = &self.vertices[seg];
            let v1 = &self.vertices[(seg + 1) % n];
            let steps = 16;
            for step in 0..steps {
                let t = step as f32 / steps as f32;
                let p = Self::sample_bezier(v0, v1, t);
                let dx = point[0] - p[0];
                let dy = point[1] - p[1];
                let dist = (dx * dx + dy * dy).sqrt();
                min_dist = min_dist.min(dist);
            }
        }

        min_dist
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect_mask(x: f32, y: f32, w: f32, h: f32) -> MaskPath {
        MaskPath {
            vertices: vec![
                MaskVertex::new(x, y),
                MaskVertex::new(x + w, y),
                MaskVertex::new(x + w, y + h),
                MaskVertex::new(x, y + h),
            ],
            closed: true,
            inverted: false,
            opacity: 1.0,
            feather_falloff: 1.0,
        }
    }

    #[test]
    fn test_rectangle_mask_rasterize() {
        let mask = rect_mask(2.0, 2.0, 4.0, 4.0);
        let matte = mask.rasterize(8, 8);
        // Center pixel should be filled
        assert!(matte[3 * 8 + 3] > 0.5);
        // Corner pixel should be empty
        assert!(matte[0] < 0.1);
    }

    #[test]
    fn test_contains_point() {
        let mask = rect_mask(10.0, 10.0, 100.0, 100.0);
        assert!(mask.contains_point([50.0, 50.0]));
        assert!(!mask.contains_point([5.0, 5.0]));
        assert!(!mask.contains_point([150.0, 150.0]));
    }

    #[test]
    fn test_inverted_mask() {
        let mask = MaskPath {
            inverted: true,
            ..rect_mask(10.0, 10.0, 80.0, 80.0)
        };
        // Point inside the rectangle should return false when inverted
        assert!(!mask.contains_point([50.0, 50.0]));
        // Point outside should return true when inverted
        assert!(mask.contains_point([5.0, 5.0]));
    }

    #[test]
    fn test_bounding_box() {
        let mask = rect_mask(10.0, 20.0, 100.0, 50.0);
        let (min, max) = mask.bounding_box();
        assert!((min[0] - 10.0).abs() < 1.0);
        assert!((min[1] - 20.0).abs() < 1.0);
        assert!((max[0] - 110.0).abs() < 1.0);
        assert!((max[1] - 70.0).abs() < 1.0);
    }

    #[test]
    fn test_bezier_sampling() {
        let v0 = MaskVertex::new(0.0, 0.0);
        let v1 = MaskVertex::new(100.0, 100.0);
        let start = MaskPath::sample_bezier(&v0, &v1, 0.0);
        let end = MaskPath::sample_bezier(&v0, &v1, 1.0);
        assert!((start[0]).abs() < 0.01);
        assert!((end[0] - 100.0).abs() < 0.01);
    }

    #[test]
    fn test_empty_mask() {
        let mask = MaskPath::default();
        let matte = mask.rasterize(10, 10);
        assert!(matte.iter().all(|&v| v == 0.0));
    }
}
