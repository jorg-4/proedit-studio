//! Planar (homography) tracker using RANSAC.

use crate::point_tracker::PointTracker;
use crate::pyramid::GrayImage;
use serde::{Deserialize, Serialize};

/// A planar region defined by 4 corner points.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanarRegion {
    pub corners: [[f32; 2]; 4],
}

/// Planar tracker computing a homography via point tracking + RANSAC.
pub struct PlanarTracker {
    pub region: PlanarRegion,
    point_tracker: PointTracker,
    initial_points: Vec<[f32; 2]>,
    pub ransac_iterations: u32,
    pub ransac_threshold: f32,
}

impl PlanarTracker {
    pub fn new(region: PlanarRegion) -> Self {
        let mut point_tracker = PointTracker::new();
        let mut initial_points = Vec::new();
        let grid_size = 8;
        for gy in 0..grid_size {
            for gx in 0..grid_size {
                let u = (gx as f32 + 0.5) / grid_size as f32;
                let v = (gy as f32 + 0.5) / grid_size as f32;
                let top = [
                    region.corners[0][0] + (region.corners[1][0] - region.corners[0][0]) * u,
                    region.corners[0][1] + (region.corners[1][1] - region.corners[0][1]) * u,
                ];
                let bot = [
                    region.corners[3][0] + (region.corners[2][0] - region.corners[3][0]) * u,
                    region.corners[3][1] + (region.corners[2][1] - region.corners[3][1]) * u,
                ];
                let px = top[0] + (bot[0] - top[0]) * v;
                let py = top[1] + (bot[1] - top[1]) * v;
                point_tracker.add_point(px, py);
                initial_points.push([px, py]);
            }
        }
        Self {
            region,
            point_tracker,
            initial_points,
            ransac_iterations: 1000,
            ransac_threshold: 3.0,
        }
    }

    pub fn track_frame(&mut self, prev: &GrayImage, curr: &GrayImage) {
        self.point_tracker.track_frame(prev, curr);
        let (src, dst) = self.collect_matches();
        if src.len() >= 4 {
            if let Some(h) =
                ransac_homography(&src, &dst, self.ransac_iterations, self.ransac_threshold)
            {
                for corner in &mut self.region.corners {
                    let [x, y] = *corner;
                    let w = h[2][0] * x + h[2][1] * y + h[2][2];
                    if w.abs() > 1e-8 {
                        corner[0] = (h[0][0] * x + h[0][1] * y + h[0][2]) / w;
                        corner[1] = (h[1][0] * x + h[1][1] * y + h[1][2]) / w;
                    }
                }
            }
        }
    }

    pub fn homography(&self) -> [[f32; 3]; 3] {
        let (src, dst) = self.collect_matches();
        if src.len() >= 4 {
            compute_homography(&src, &dst).unwrap_or([
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.0, 0.0, 1.0],
            ])
        } else {
            [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]
        }
    }

    fn collect_matches(&self) -> (Vec<[f32; 2]>, Vec<[f32; 2]>) {
        let mut src = Vec::new();
        let mut dst = Vec::new();
        for (i, point) in self.point_tracker.points.iter().enumerate() {
            if !point.lost {
                src.push(self.initial_points[i]);
                dst.push(point.position);
            }
        }
        (src, dst)
    }
}

/// Compute homography using DLT from 4+ point pairs.
pub fn compute_homography(src: &[[f32; 2]], dst: &[[f32; 2]]) -> Option<[[f32; 3]; 3]> {
    if src.len() < 4 || src.len() != dst.len() {
        return None;
    }
    let n = src.len().min(4);
    let mut a = [[0.0f64; 9]; 8];
    for i in 0..n {
        let (x, y) = (src[i][0] as f64, src[i][1] as f64);
        let (xp, yp) = (dst[i][0] as f64, dst[i][1] as f64);
        a[i * 2] = [-x, -y, -1.0, 0.0, 0.0, 0.0, x * xp, y * xp, xp];
        a[i * 2 + 1] = [0.0, 0.0, 0.0, -x, -y, -1.0, x * yp, y * yp, yp];
    }

    let mut m = a;
    #[allow(clippy::needless_range_loop)]
    for col in 0..8 {
        let mut max_row = col;
        let mut max_val = m[col][col].abs();
        for row in (col + 1)..8 {
            if m[row][col].abs() > max_val {
                max_val = m[row][col].abs();
                max_row = row;
            }
        }
        if max_val < 1e-10 {
            return None;
        }
        m.swap(col, max_row);
        let pivot = m[col][col];
        for j in col..9 {
            m[col][j] /= pivot;
        }
        for row in 0..8 {
            if row != col {
                let factor = m[row][col];
                for j in col..9 {
                    m[row][j] -= factor * m[col][j];
                }
            }
        }
    }

    let mut h = [0.0f64; 9];
    h[8] = 1.0;
    for i in 0..8 {
        h[i] = -m[i][8];
    }
    if h[8].abs() > 1e-10 {
        let inv = 1.0 / h[8];
        for v in &mut h {
            *v *= inv;
        }
    }

    Some([
        [h[0] as f32, h[1] as f32, h[2] as f32],
        [h[3] as f32, h[4] as f32, h[5] as f32],
        [h[6] as f32, h[7] as f32, h[8] as f32],
    ])
}

/// RANSAC-based homography estimation.
pub fn ransac_homography(
    src: &[[f32; 2]],
    dst: &[[f32; 2]],
    iterations: u32,
    threshold: f32,
) -> Option<[[f32; 3]; 3]> {
    if src.len() < 4 {
        return None;
    }
    let n = src.len();
    let mut best_h: Option<[[f32; 3]; 3]> = None;
    let mut best_inliers = 0;
    let mut seed = 12345u64;

    for _ in 0..iterations {
        let mut indices = [0usize; 4];
        for idx in &mut indices {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            *idx = (seed >> 33) as usize % n;
        }
        let s: Vec<[f32; 2]> = indices.iter().map(|&i| src[i]).collect();
        let d: Vec<[f32; 2]> = indices.iter().map(|&i| dst[i]).collect();
        if let Some(h) = compute_homography(&s, &d) {
            let mut inliers = 0;
            for i in 0..n {
                let [x, y] = src[i];
                let w = h[2][0] * x + h[2][1] * y + h[2][2];
                if w.abs() < 1e-8 {
                    continue;
                }
                let px = (h[0][0] * x + h[0][1] * y + h[0][2]) / w;
                let py = (h[1][0] * x + h[1][1] * y + h[1][2]) / w;
                if ((px - dst[i][0]).powi(2) + (py - dst[i][1]).powi(2)).sqrt() < threshold {
                    inliers += 1;
                }
            }
            if inliers > best_inliers {
                best_inliers = inliers;
                best_h = Some(h);
            }
        }
    }
    best_h
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identity_homography() {
        let pts = [[0.0, 0.0], [100.0, 0.0], [100.0, 100.0], [0.0, 100.0]];
        let h = compute_homography(&pts, &pts).unwrap();
        assert!((h[0][0] - 1.0).abs() < 0.1);
        assert!((h[1][1] - 1.0).abs() < 0.1);
    }

    #[test]
    fn test_translation_homography() {
        let src = [[0.0, 0.0], [100.0, 0.0], [100.0, 100.0], [0.0, 100.0]];
        let dst = [[10.0, 20.0], [110.0, 20.0], [110.0, 120.0], [10.0, 120.0]];
        let h = compute_homography(&src, &dst).unwrap();
        assert!((h[0][2] - 10.0).abs() < 1.0);
        assert!((h[1][2] - 20.0).abs() < 1.0);
    }

    #[test]
    fn test_planar_tracker_creation() {
        let region = PlanarRegion {
            corners: [[10.0, 10.0], [110.0, 10.0], [110.0, 110.0], [10.0, 110.0]],
        };
        let tracker = PlanarTracker::new(region);
        assert_eq!(tracker.point_tracker.points.len(), 64);
    }
}
