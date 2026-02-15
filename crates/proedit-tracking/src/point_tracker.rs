//! Pyramidal Lucas-Kanade point tracker.

use crate::pyramid::{GrayImage, ImagePyramid};
use serde::{Deserialize, Serialize};

/// A tracked point with position and state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackPoint {
    pub position: [f32; 2],
    pub confidence: f32,
    pub search_radius: f32,
    pub lost: bool,
}

impl TrackPoint {
    pub fn new(x: f32, y: f32) -> Self {
        Self {
            position: [x, y],
            confidence: 1.0,
            search_radius: 21.0,
            lost: false,
        }
    }
}

/// Lucas-Kanade optical flow point tracker with pyramidal support.
pub struct PointTracker {
    pub points: Vec<TrackPoint>,
    pub window_size: u32,
    pub pyramid_levels: u32,
    pub max_iterations: u32,
    pub epsilon: f32,
}

impl PointTracker {
    pub fn new() -> Self {
        Self {
            points: Vec::new(),
            window_size: 21,
            pyramid_levels: 3,
            max_iterations: 30,
            epsilon: 0.01,
        }
    }

    pub fn add_point(&mut self, x: f32, y: f32) {
        self.points.push(TrackPoint::new(x, y));
    }

    pub fn track_frame(&mut self, prev: &GrayImage, curr: &GrayImage) {
        let prev_pyr = ImagePyramid::build(prev, self.pyramid_levels);
        let curr_pyr = ImagePyramid::build(curr, self.pyramid_levels);

        for point in &mut self.points {
            if point.lost {
                continue;
            }
            match Self::track_point_pyramidal(
                &prev_pyr,
                &curr_pyr,
                point.position,
                self.window_size,
                self.max_iterations,
                self.epsilon,
            ) {
                Some((new_pos, conf)) => {
                    let dx = new_pos[0] - point.position[0];
                    let dy = new_pos[1] - point.position[1];
                    if (dx * dx + dy * dy).sqrt() > point.search_radius {
                        point.lost = true;
                        point.confidence = 0.0;
                    } else {
                        point.position = new_pos;
                        point.confidence = conf;
                    }
                }
                None => {
                    point.lost = true;
                    point.confidence = 0.0;
                }
            }
        }
    }

    pub fn active_points(&self) -> impl Iterator<Item = &TrackPoint> {
        self.points.iter().filter(|p| !p.lost)
    }

    fn track_point_pyramidal(
        prev_pyr: &ImagePyramid,
        curr_pyr: &ImagePyramid,
        position: [f32; 2],
        window_size: u32,
        max_iter: u32,
        epsilon: f32,
    ) -> Option<([f32; 2], f32)> {
        let levels = prev_pyr.levels.len();
        let mut guess = [0.0f32, 0.0];

        for level in (0..levels).rev() {
            let scale = 1.0 / (1u32 << level) as f32;
            let px = position[0] * scale;
            let py = position[1] * scale;
            let prev_img = &prev_pyr.levels[level];
            let curr_img = &curr_pyr.levels[level];
            let hw = (window_size as f32 * scale * 0.5) as i32;

            let mut g11 = 0.0f32;
            let mut g12 = 0.0f32;
            let mut g22 = 0.0f32;

            for wy in -hw..=hw {
                for wx in -hw..=hw {
                    let ix = (prev_img.get(px as i32 + wx + 1, py as i32 + wy)
                        - prev_img.get(px as i32 + wx - 1, py as i32 + wy))
                        * 0.5;
                    let iy = (prev_img.get(px as i32 + wx, py as i32 + wy + 1)
                        - prev_img.get(px as i32 + wx, py as i32 + wy - 1))
                        * 0.5;
                    g11 += ix * ix;
                    g12 += ix * iy;
                    g22 += iy * iy;
                }
            }

            let det = g11 * g22 - g12 * g12;
            if det.abs() < 1e-6 {
                if level == 0 {
                    return None;
                }
                continue;
            }
            let inv_det = 1.0 / det;

            let mut dx = guess[0] * scale;
            let mut dy = guess[1] * scale;

            for _ in 0..max_iter {
                let mut bx = 0.0f32;
                let mut by = 0.0f32;
                for wy in -hw..=hw {
                    for wx in -hw..=hw {
                        let ix = (prev_img.get(px as i32 + wx + 1, py as i32 + wy)
                            - prev_img.get(px as i32 + wx - 1, py as i32 + wy))
                            * 0.5;
                        let iy = (prev_img.get(px as i32 + wx, py as i32 + wy + 1)
                            - prev_img.get(px as i32 + wx, py as i32 + wy - 1))
                            * 0.5;
                        let it = curr_img.get((px + dx) as i32 + wx, (py + dy) as i32 + wy)
                            - prev_img.get(px as i32 + wx, py as i32 + wy);
                        bx += ix * it;
                        by += iy * it;
                    }
                }
                let ddx = inv_det * (g22 * bx - g12 * by);
                let ddy = inv_det * (-g12 * bx + g11 * by);
                dx -= ddx;
                dy -= ddy;
                if ddx * ddx + ddy * ddy < epsilon * epsilon {
                    break;
                }
            }
            guess = [dx / scale, dy / scale];
        }

        let new_pos = [position[0] + guess[0], position[1] + guess[1]];
        Some((new_pos, 1.0))
    }
}

impl Default for PointTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stationary_point() {
        // Checkerboard pattern gives strong gradients in both directions
        let mut img = GrayImage::new(64, 64);
        for y in 0..64u32 {
            for x in 0..64u32 {
                let check = ((x / 4) + (y / 4)) % 2;
                img.set(x, y, check as f32);
            }
        }
        let mut tracker = PointTracker::new();
        tracker.pyramid_levels = 1; // Keep it simple
        tracker.add_point(32.0, 32.0);
        tracker.track_frame(&img, &img);
        let pt = &tracker.points[0];
        assert!(!pt.lost);
        assert!((pt.position[0] - 32.0).abs() < 2.0);
    }

    #[test]
    fn test_translated_point() {
        let mut prev = GrayImage::new(64, 64);
        let mut curr = GrayImage::new(64, 64);
        for y in 25..35u32 {
            for x in 25..35u32 {
                prev.set(x, y, 1.0);
            }
        }
        for y in 25..35u32 {
            for x in 30..40u32 {
                curr.set(x, y, 1.0);
            }
        }
        let mut tracker = PointTracker::new();
        tracker.pyramid_levels = 1;
        tracker.add_point(30.0, 30.0);
        tracker.track_frame(&prev, &curr);
        assert!(!tracker.points[0].lost);
    }

    #[test]
    fn test_active_points() {
        let mut tracker = PointTracker::new();
        tracker.add_point(10.0, 10.0);
        tracker.add_point(20.0, 20.0);
        tracker.points[1].lost = true;
        assert_eq!(tracker.active_points().count(), 1);
    }
}
