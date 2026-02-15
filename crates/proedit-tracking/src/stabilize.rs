//! Video stabilization using point tracking and motion smoothing.

use crate::point_tracker::PointTracker;
use crate::pyramid::GrayImage;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum StabilizationMethod {
    #[default]
    Translation,
    Rotation,
    Perspective,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StabilizationParams {
    pub method: StabilizationMethod,
    pub smoothness: f32,
    pub crop_ratio: f32,
}

impl Default for StabilizationParams {
    fn default() -> Self {
        Self {
            method: StabilizationMethod::Translation,
            smoothness: 30.0,
            crop_ratio: 0.9,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MotionData {
    pub dx: Vec<f32>,
    pub dy: Vec<f32>,
    pub rotation: Vec<f32>,
}

impl MotionData {
    pub fn new(len: usize) -> Self {
        Self {
            dx: vec![0.0; len],
            dy: vec![0.0; len],
            rotation: vec![0.0; len],
        }
    }
    pub fn len(&self) -> usize {
        self.dx.len()
    }
    pub fn is_empty(&self) -> bool {
        self.dx.is_empty()
    }
}

pub fn analyze_motion(frames: &[GrayImage]) -> MotionData {
    if frames.len() < 2 {
        return MotionData::new(frames.len());
    }
    let n = frames.len() - 1;
    let mut motion = MotionData::new(n);
    for i in 0..n {
        let prev = &frames[i];
        let curr = &frames[i + 1];
        let mut tracker = PointTracker::new();
        tracker.pyramid_levels = 2;
        let spacing = 40u32;
        for y in (spacing..prev.height.saturating_sub(spacing)).step_by(spacing as usize) {
            for x in (spacing..prev.width.saturating_sub(spacing)).step_by(spacing as usize) {
                tracker.add_point(x as f32, y as f32);
            }
        }
        let orig_positions: Vec<[f32; 2]> = tracker.points.iter().map(|p| p.position).collect();
        tracker.track_frame(prev, curr);
        let mut tdx = 0.0f32;
        let mut tdy = 0.0f32;
        let mut count = 0;
        for (j, point) in tracker.points.iter().enumerate() {
            if !point.lost {
                tdx += point.position[0] - orig_positions[j][0];
                tdy += point.position[1] - orig_positions[j][1];
                count += 1;
            }
        }
        if count > 0 {
            motion.dx[i] = tdx / count as f32;
            motion.dy[i] = tdy / count as f32;
        }
    }
    motion
}

pub fn smooth_motion(raw: &MotionData, params: &StabilizationParams) -> MotionData {
    let n = raw.len();
    if n == 0 {
        return MotionData::new(0);
    }
    let mut cum_dx = vec![0.0f32; n + 1];
    let mut cum_dy = vec![0.0f32; n + 1];
    let mut cum_rot = vec![0.0f32; n + 1];
    for i in 0..n {
        cum_dx[i + 1] = cum_dx[i] + raw.dx[i];
        cum_dy[i + 1] = cum_dy[i] + raw.dy[i];
        cum_rot[i + 1] = cum_rot[i] + raw.rotation[i];
    }
    let smooth_dx = gaussian_smooth_1d(&cum_dx, params.smoothness);
    let smooth_dy = gaussian_smooth_1d(&cum_dy, params.smoothness);
    let smooth_rot = gaussian_smooth_1d(&cum_rot, params.smoothness);
    let mut smoothed = MotionData::new(n);
    for i in 0..n {
        smoothed.dx[i] = smooth_dx[i + 1] - smooth_dx[i];
        smoothed.dy[i] = smooth_dy[i + 1] - smooth_dy[i];
        smoothed.rotation[i] = smooth_rot[i + 1] - smooth_rot[i];
    }
    smoothed
}

pub fn compute_correction(raw: &MotionData, smooth: &MotionData) -> Vec<(f32, f32, f32)> {
    let n = raw.len().min(smooth.len());
    (0..n)
        .map(|i| {
            (
                smooth.dx[i] - raw.dx[i],
                smooth.dy[i] - raw.dy[i],
                smooth.rotation[i] - raw.rotation[i],
            )
        })
        .collect()
}

fn gaussian_smooth_1d(data: &[f32], sigma: f32) -> Vec<f32> {
    if sigma < 0.5 {
        return data.to_vec();
    }
    let n = data.len();
    let radius = (sigma * 3.0).ceil() as i32;
    let sigma2 = 2.0 * sigma * sigma;
    let mut result = vec![0.0f32; n];
    for i in 0..n as i32 {
        let mut sum = 0.0f32;
        let mut weight = 0.0f32;
        for k in -radius..=radius {
            let j = (i + k).clamp(0, n as i32 - 1) as usize;
            let w = (-((k * k) as f32) / sigma2).exp();
            sum += data[j] * w;
            weight += w;
        }
        result[i as usize] = if weight > 0.0 {
            sum / weight
        } else {
            data[i as usize]
        };
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_smooth_motion_reduces_jitter() {
        let n = 100;
        let mut raw = MotionData::new(n);
        for i in 0..n {
            raw.dx[i] = (i as f32 * 0.5).sin() * 10.0;
        }
        let params = StabilizationParams {
            smoothness: 10.0,
            ..Default::default()
        };
        let smoothed = smooth_motion(&raw, &params);
        let raw_var: f32 = raw.dx.iter().map(|v| v * v).sum::<f32>() / n as f32;
        let smooth_var: f32 = smoothed.dx.iter().map(|v| v * v).sum::<f32>() / n as f32;
        assert!(smooth_var < raw_var);
    }

    #[test]
    fn test_compute_correction() {
        let raw = MotionData {
            dx: vec![10.0, 20.0],
            dy: vec![5.0, 10.0],
            rotation: vec![0.0, 0.0],
        };
        let smooth = MotionData {
            dx: vec![15.0, 20.0],
            dy: vec![7.0, 10.0],
            rotation: vec![0.0, 0.0],
        };
        let corrections = compute_correction(&raw, &smooth);
        assert!((corrections[0].0 - 5.0).abs() < 0.01);
    }

    #[test]
    fn test_empty_motion() {
        let motion = MotionData::new(0);
        assert!(motion.is_empty());
        let smoothed = smooth_motion(&motion, &StabilizationParams::default());
        assert!(smoothed.is_empty());
    }
}
