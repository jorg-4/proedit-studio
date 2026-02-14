//! Scene/cut detection using frame-to-frame pixel differencing.
//!
//! Detects hard cuts and dissolves in video footage by comparing
//! consecutive frames. Works without any ONNX model — pure math.

use proedit_core::FrameBuffer;
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

/// A detected scene boundary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneBoundary {
    /// Frame number where the scene change occurs.
    pub frame_number: i64,
    /// Timestamp in seconds.
    pub timestamp_secs: f64,
    /// Confidence score (0.0 to 1.0).
    pub confidence: f32,
    /// Type of transition detected.
    pub transition_type: TransitionType,
}

/// Type of scene transition.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum TransitionType {
    /// Abrupt scene change.
    HardCut,
    /// Gradual transition (dissolve, fade).
    Dissolve,
    /// Could not determine transition type.
    Unknown,
}

/// Configuration for scene detection.
#[derive(Debug, Clone)]
pub struct SceneDetectConfig {
    /// Mean absolute difference threshold for hard cuts (default: 0.5).
    pub hard_cut_threshold: f32,
    /// Mean absolute difference threshold for dissolves (default: 0.3).
    pub dissolve_threshold: f32,
    /// Minimum number of frames between detected cuts (default: 15).
    pub min_scene_duration_frames: i64,
}

impl Default for SceneDetectConfig {
    fn default() -> Self {
        Self {
            hard_cut_threshold: 0.5,
            dissolve_threshold: 0.3,
            min_scene_duration_frames: 15,
        }
    }
}

/// Detect scenes using frame-to-frame pixel difference (no model needed).
///
/// Compares consecutive frames using mean absolute difference (MAD) and
/// classifies transitions as hard cuts or dissolves based on thresholds.
/// Enforces minimum scene duration to suppress rapid false positives.
pub fn detect_scenes_by_difference(
    frames: &[FrameBuffer],
    fps: f64,
    config: &SceneDetectConfig,
) -> Vec<SceneBoundary> {
    if frames.len() < 2 {
        return Vec::new();
    }

    let mut candidates = Vec::new();

    // Step 1: Compute MAD for each consecutive pair
    for i in 0..frames.len() - 1 {
        let mad = match mean_absolute_difference(&frames[i], &frames[i + 1]) {
            Some(val) => val,
            None => {
                warn!(
                    frame = i,
                    "Skipping frame pair: dimension mismatch or empty"
                );
                continue;
            }
        };

        let frame_number = (i + 1) as i64;
        let timestamp_secs = frame_number as f64 / fps;

        if mad >= config.hard_cut_threshold {
            candidates.push(SceneBoundary {
                frame_number,
                timestamp_secs,
                confidence: mad.min(1.0),
                transition_type: TransitionType::HardCut,
            });
        } else if mad >= config.dissolve_threshold {
            candidates.push(SceneBoundary {
                frame_number,
                timestamp_secs,
                confidence: mad / config.hard_cut_threshold,
                transition_type: TransitionType::Dissolve,
            });
        }
    }

    // Step 2: Suppress detections within min_scene_duration_frames of each other
    let mut result = Vec::new();
    let mut last_frame: i64 = -config.min_scene_duration_frames; // allow first detection

    for candidate in candidates {
        if candidate.frame_number - last_frame >= config.min_scene_duration_frames {
            debug!(
                frame = candidate.frame_number,
                confidence = candidate.confidence,
                transition = ?candidate.transition_type,
                "Scene boundary detected"
            );
            last_frame = candidate.frame_number;
            result.push(candidate);
        }
    }

    result
}

/// Compute mean absolute difference between two RGBA8 frames.
///
/// Returns a value in `[0.0, 1.0]` where 0 = identical, 1 = maximum difference.
/// Compares RGB channels only (skips alpha). Returns `None` if frames have
/// different dimensions.
fn mean_absolute_difference(a: &FrameBuffer, b: &FrameBuffer) -> Option<f32> {
    if a.width != b.width || a.height != b.height {
        return None;
    }

    let w = a.width as usize;
    let h = a.height as usize;
    let pixel_count = w * h;
    if pixel_count == 0 {
        return Some(0.0);
    }

    let a_plane = a.primary_plane();
    let b_plane = b.primary_plane();

    let channels = 3; // Compare RGB, skip alpha
    let mut total_diff: f64 = 0.0;
    let mut compared_pixels: usize = 0;

    for y in 0..h {
        let a_row = a_plane.row(y as u32);
        let b_row = b_plane.row(y as u32);

        for x in 0..w {
            let base = x * 4; // RGBA = 4 bytes per pixel
            if base + 2 >= a_row.len() || base + 2 >= b_row.len() {
                break;
            }
            for c in 0..channels {
                let va = a_row[base + c] as f64;
                let vb = b_row[base + c] as f64;
                total_diff += (va - vb).abs();
            }
            compared_pixels += 1;
        }
    }

    if compared_pixels == 0 {
        return Some(0.0);
    }

    let mad = total_diff / (compared_pixels as f64 * channels as f64 * 255.0);
    Some(mad as f32)
}

/// Helper to create an RGBA8 solid-color frame for testing.
#[cfg(test)]
fn make_solid_frame(width: u32, height: u32, r: u8, g: u8, b: u8) -> FrameBuffer {
    let mut frame = FrameBuffer::new(width, height, proedit_core::PixelFormat::Rgba8);
    let plane = frame.primary_plane_mut();
    for y in 0..height {
        let row = plane.row_mut(y);
        for x in 0..width as usize {
            let base = x * 4;
            if base + 3 < row.len() {
                row[base] = r;
                row[base + 1] = g;
                row[base + 2] = b;
                row[base + 3] = 255;
            }
        }
    }
    frame
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identical_frames_zero_difference() {
        let a = make_solid_frame(64, 64, 128, 128, 128);
        let b = make_solid_frame(64, 64, 128, 128, 128);
        let diff = mean_absolute_difference(&a, &b).expect("should compute diff");
        assert!(
            (diff - 0.0).abs() < 1e-6,
            "Identical frames should have 0 difference, got {diff}"
        );
    }

    #[test]
    fn test_opposite_frames_max_difference() {
        let a = make_solid_frame(64, 64, 0, 0, 0);
        let b = make_solid_frame(64, 64, 255, 255, 255);
        let diff = mean_absolute_difference(&a, &b).expect("should compute diff");
        assert!(
            (diff - 1.0).abs() < 1e-6,
            "Black vs white should be ~1.0, got {diff}"
        );
    }

    #[test]
    fn test_different_sizes_returns_none() {
        let a = make_solid_frame(64, 64, 0, 0, 0);
        let b = make_solid_frame(32, 32, 0, 0, 0);
        assert!(mean_absolute_difference(&a, &b).is_none());
    }

    #[test]
    fn test_detect_hard_cut() {
        let frames = vec![
            make_solid_frame(64, 64, 0, 0, 0),
            make_solid_frame(64, 64, 0, 0, 0),
            make_solid_frame(64, 64, 0, 0, 0),
            make_solid_frame(64, 64, 255, 255, 255), // frame 3: hard cut
            make_solid_frame(64, 64, 255, 255, 255),
            make_solid_frame(64, 64, 255, 255, 255),
        ];
        let config = SceneDetectConfig::default();
        let scenes = detect_scenes_by_difference(&frames, 30.0, &config);

        assert_eq!(scenes.len(), 1, "Should detect exactly 1 cut");
        assert_eq!(scenes[0].frame_number, 3, "Cut should be at frame 3");
        assert_eq!(scenes[0].transition_type, TransitionType::HardCut);
        assert!(scenes[0].confidence > 0.9);
    }

    #[test]
    fn test_no_cuts_in_static_footage() {
        let frames: Vec<_> = (0..30)
            .map(|_| make_solid_frame(64, 64, 100, 100, 100))
            .collect();
        let config = SceneDetectConfig::default();
        let scenes = detect_scenes_by_difference(&frames, 30.0, &config);
        assert!(scenes.is_empty(), "Static footage should have no cuts");
    }

    #[test]
    fn test_min_scene_duration_suppresses_rapid_cuts() {
        let mut frames = Vec::new();
        for i in 0..30 {
            if i % 2 == 0 {
                frames.push(make_solid_frame(64, 64, 0, 0, 0));
            } else {
                frames.push(make_solid_frame(64, 64, 255, 255, 255));
            }
        }
        let config = SceneDetectConfig {
            min_scene_duration_frames: 10,
            ..Default::default()
        };
        let scenes = detect_scenes_by_difference(&frames, 30.0, &config);
        assert!(
            scenes.len() <= 3,
            "Suppression should limit rapid cuts, got {}",
            scenes.len()
        );
    }
}
