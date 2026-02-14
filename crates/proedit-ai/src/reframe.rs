//! Smart reframing for multi-platform export.
//!
//! Automatically reframes 16:9 footage for 9:16 (TikTok/Reels), 4:5 (Instagram),
//! 1:1 (Square) by intelligently tracking subjects and re-composing shots.
//!
//! Uses face detection and subject tracking to determine optimal crop positions,
//! with temporal smoothing to avoid jitter.

use proedit_core::{FrameBuffer, Rect, Vec2};
use serde::{Deserialize, Serialize};

/// Common target aspect ratios for social media platforms.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum TargetAspect {
    /// 9:16 vertical (TikTok, Reels, Shorts).
    Vertical9x16,
    /// 4:5 portrait (Instagram feed).
    Portrait4x5,
    /// 1:1 square (Instagram, Facebook).
    Square,
    /// 16:9 landscape (YouTube, standard).
    Landscape16x9,
    /// Custom aspect ratio (width / height).
    Custom(f32),
}

impl TargetAspect {
    /// Get the aspect ratio as a float (width / height).
    pub fn ratio(&self) -> f32 {
        match self {
            Self::Vertical9x16 => 9.0 / 16.0,
            Self::Portrait4x5 => 4.0 / 5.0,
            Self::Square => 1.0,
            Self::Landscape16x9 => 16.0 / 9.0,
            Self::Custom(r) => *r,
        }
    }

    /// Display name for UI.
    pub fn display_name(&self) -> &str {
        match self {
            Self::Vertical9x16 => "9:16 (TikTok/Reels)",
            Self::Portrait4x5 => "4:5 (Instagram)",
            Self::Square => "1:1 (Square)",
            Self::Landscape16x9 => "16:9 (YouTube)",
            Self::Custom(_) => "Custom",
        }
    }
}

/// A detected face bounding box.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct FaceBbox {
    /// Bounding box in frame coordinates.
    pub rect: Rect,
    /// Detection confidence (0.0 to 1.0).
    pub confidence: f32,
}

/// Result of computing a reframe for a single frame.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ReframeResult {
    /// Crop rectangle in source frame coordinates.
    pub crop_rect: Rect,
    /// Confidence in the framing quality (0.0 to 1.0).
    pub confidence: f32,
}

/// Configuration for smart reframing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReframeConfig {
    /// Temporal smoothing factor (0.0 = no smoothing, 1.0 = max smoothing).
    /// Controls how quickly the crop follows subject movement.
    pub smoothing: f32,
    /// Whether to prioritize faces over other detected subjects.
    pub prefer_faces: bool,
    /// Padding factor around detected subjects (1.0 = no padding, 1.5 = 50% more space).
    pub subject_padding: f32,
}

impl Default for ReframeConfig {
    fn default() -> Self {
        Self {
            smoothing: 0.85,
            prefer_faces: true,
            subject_padding: 1.2,
        }
    }
}

/// Smart reframing engine.
pub struct SmartReframer {
    config: ReframeConfig,
    /// Previous crop position for temporal smoothing.
    prev_crop: Option<Rect>,
}

impl SmartReframer {
    /// Create a new reframer with the given configuration.
    pub fn new(config: ReframeConfig) -> Self {
        Self {
            config,
            prev_crop: None,
        }
    }

    /// Reset temporal state (call when switching clips or seeking).
    pub fn reset(&mut self) {
        self.prev_crop = None;
    }

    /// Compute the optimal reframe crop for a single frame.
    ///
    /// Uses detected faces and/or a subject center to position the crop
    /// within the target aspect ratio, with temporal smoothing.
    pub fn compute_reframe(
        &mut self,
        frame_width: u32,
        frame_height: u32,
        target_aspect: TargetAspect,
        faces: &[FaceBbox],
        subject_center: Option<Vec2>,
    ) -> ReframeResult {
        let src_w = frame_width as f32;
        let src_h = frame_height as f32;
        let target_ratio = target_aspect.ratio();

        // Calculate crop dimensions maintaining target aspect ratio
        let (crop_w, crop_h) = calculate_crop_dimensions(src_w, src_h, target_ratio);

        // Determine the focal point (where to center the crop)
        let focal = determine_focal_point(
            src_w,
            src_h,
            faces,
            subject_center,
            self.config.prefer_faces,
        );

        // Position crop centered on focal point, clamped to frame bounds
        let crop = position_crop(src_w, src_h, crop_w, crop_h, focal);

        // Apply temporal smoothing
        let smoothed = if let Some(prev) = self.prev_crop {
            lerp_rect(&prev, &crop, 1.0 - self.config.smoothing)
        } else {
            crop
        };

        self.prev_crop = Some(smoothed);

        // Confidence based on how well the subject fits
        let confidence = compute_framing_confidence(&smoothed, faces, subject_center);

        ReframeResult {
            crop_rect: smoothed,
            confidence,
        }
    }

    /// Compute reframe for an entire sequence of frames (batch mode).
    ///
    /// Returns a reframe result per frame, with temporal smoothing applied
    /// across the sequence.
    pub fn compute_sequence(
        &mut self,
        frame_width: u32,
        frame_height: u32,
        target_aspect: TargetAspect,
        per_frame_faces: &[Vec<FaceBbox>],
    ) -> Vec<ReframeResult> {
        self.reset();
        per_frame_faces
            .iter()
            .map(|faces| {
                self.compute_reframe(frame_width, frame_height, target_aspect, faces, None)
            })
            .collect()
    }

    /// Get the configuration.
    pub fn config(&self) -> &ReframeConfig {
        &self.config
    }
}

/// Calculate crop dimensions that fit within the source frame at the target aspect ratio.
fn calculate_crop_dimensions(src_w: f32, src_h: f32, target_ratio: f32) -> (f32, f32) {
    let src_ratio = src_w / src_h;

    if target_ratio > src_ratio {
        // Target is wider than source — fit to width
        (src_w, src_w / target_ratio)
    } else {
        // Target is taller than source — fit to height
        (src_h * target_ratio, src_h)
    }
}

/// Determine the focal point for the crop based on detected subjects.
fn determine_focal_point(
    src_w: f32,
    src_h: f32,
    faces: &[FaceBbox],
    subject_center: Option<Vec2>,
    prefer_faces: bool,
) -> Vec2 {
    // If faces detected and preferred, use face-weighted centroid
    if prefer_faces && !faces.is_empty() {
        return weighted_face_center(faces);
    }

    // Use subject center if provided
    if let Some(center) = subject_center {
        return center;
    }

    // If faces detected but not preferred, still use them as fallback
    if !faces.is_empty() {
        return weighted_face_center(faces);
    }

    // Default: center of frame
    Vec2::new(src_w * 0.5, src_h * 0.5)
}

/// Compute the confidence-weighted centroid of detected faces.
fn weighted_face_center(faces: &[FaceBbox]) -> Vec2 {
    let total_weight: f32 = faces.iter().map(|f| f.confidence).sum();
    if total_weight <= 0.0 {
        return faces.first().map(|f| f.rect.center()).unwrap_or(Vec2::ZERO);
    }

    let mut cx = 0.0_f32;
    let mut cy = 0.0_f32;
    for face in faces {
        let center = face.rect.center();
        cx += center.x * face.confidence;
        cy += center.y * face.confidence;
    }
    Vec2::new(cx / total_weight, cy / total_weight)
}

/// Position a crop rectangle centered on the focal point, clamped to frame bounds.
fn position_crop(src_w: f32, src_h: f32, crop_w: f32, crop_h: f32, focal: Vec2) -> Rect {
    let x = (focal.x - crop_w * 0.5).clamp(0.0, (src_w - crop_w).max(0.0));
    let y = (focal.y - crop_h * 0.5).clamp(0.0, (src_h - crop_h).max(0.0));
    Rect::new(x, y, crop_w, crop_h)
}

/// Linearly interpolate between two rectangles.
fn lerp_rect(a: &Rect, b: &Rect, t: f32) -> Rect {
    Rect::new(
        a.x + (b.x - a.x) * t,
        a.y + (b.y - a.y) * t,
        a.width + (b.width - a.width) * t,
        a.height + (b.height - a.height) * t,
    )
}

/// Compute framing confidence based on how well subjects fit in the crop.
fn compute_framing_confidence(
    crop: &Rect,
    faces: &[FaceBbox],
    subject_center: Option<Vec2>,
) -> f32 {
    if faces.is_empty() && subject_center.is_none() {
        return 0.5; // neutral confidence when no subject info
    }

    let mut score = 0.0_f32;
    let mut count = 0;

    // Check if faces are within the crop
    for face in faces {
        let face_center = face.rect.center();
        if crop.contains(face_center) {
            score += 1.0;
        } else {
            score += 0.2; // partial score for faces outside crop
        }
        count += 1;
    }

    if let Some(center) = subject_center {
        if crop.contains(center) {
            score += 1.0;
        } else {
            score += 0.2;
        }
        count += 1;
    }

    if count > 0 {
        (score / count as f32).clamp(0.0, 1.0)
    } else {
        0.5
    }
}

/// CPU-based face detection using simple skin color heuristic.
/// In production, this would use a lightweight ONNX face detection model.
pub fn cpu_detect_faces(frame: &FrameBuffer) -> Vec<FaceBbox> {
    // Placeholder: return empty for now.
    // Real implementation would use a face detection model or dlib.
    let _ = frame;
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_target_aspect_ratios() {
        assert!((TargetAspect::Vertical9x16.ratio() - 0.5625).abs() < 0.001);
        assert!((TargetAspect::Square.ratio() - 1.0).abs() < 0.001);
        assert!((TargetAspect::Landscape16x9.ratio() - 1.7778).abs() < 0.001);
    }

    #[test]
    fn test_crop_dimensions_wider_target() {
        // 1920x1080 source, 16:9 target (same ratio)
        let (w, h) = calculate_crop_dimensions(1920.0, 1080.0, 16.0 / 9.0);
        assert!((w - 1920.0).abs() < 1.0);
        assert!((h - 1080.0).abs() < 1.0);
    }

    #[test]
    fn test_crop_dimensions_taller_target() {
        // 1920x1080 source, 9:16 target (vertical)
        let (w, h) = calculate_crop_dimensions(1920.0, 1080.0, 9.0 / 16.0);
        assert!((h - 1080.0).abs() < 1.0);
        assert!((w - 607.5).abs() < 1.0);
    }

    #[test]
    fn test_position_crop_clamped() {
        // Focal point at corner — crop should be clamped to frame
        let crop = position_crop(1920.0, 1080.0, 600.0, 1080.0, Vec2::new(0.0, 540.0));
        assert!(crop.x >= 0.0, "Crop x should be >= 0, got {}", crop.x);
        assert!(crop.y >= 0.0, "Crop y should be >= 0, got {}", crop.y);
    }

    #[test]
    fn test_position_crop_centered() {
        let crop = position_crop(1920.0, 1080.0, 600.0, 1080.0, Vec2::new(960.0, 540.0));
        assert!(
            (crop.x - 660.0).abs() < 1.0,
            "Crop should be centered at x=660, got {}",
            crop.x
        );
    }

    #[test]
    fn test_lerp_rect() {
        let a = Rect::new(0.0, 0.0, 100.0, 100.0);
        let b = Rect::new(100.0, 100.0, 200.0, 200.0);
        let mid = lerp_rect(&a, &b, 0.5);
        assert!((mid.x - 50.0).abs() < 0.01);
        assert!((mid.y - 50.0).abs() < 0.01);
        assert!((mid.width - 150.0).abs() < 0.01);
    }

    #[test]
    fn test_weighted_face_center() {
        let faces = vec![
            FaceBbox {
                rect: Rect::new(100.0, 100.0, 50.0, 50.0),
                confidence: 1.0,
            },
            FaceBbox {
                rect: Rect::new(200.0, 200.0, 50.0, 50.0),
                confidence: 1.0,
            },
        ];
        let center = weighted_face_center(&faces);
        // Midpoint of two equally-weighted face centers (125, 125) and (225, 225)
        assert!((center.x - 175.0).abs() < 1.0);
        assert!((center.y - 175.0).abs() < 1.0);
    }

    #[test]
    fn test_reframer_temporal_smoothing() {
        let mut reframer = SmartReframer::new(ReframeConfig {
            smoothing: 0.5,
            ..Default::default()
        });

        // Frame 1: face on the left
        let faces_left = vec![FaceBbox {
            rect: Rect::new(100.0, 400.0, 100.0, 100.0),
            confidence: 1.0,
        }];
        let r1 =
            reframer.compute_reframe(1920, 1080, TargetAspect::Vertical9x16, &faces_left, None);

        // Frame 2: face on the right
        let faces_right = vec![FaceBbox {
            rect: Rect::new(1700.0, 400.0, 100.0, 100.0),
            confidence: 1.0,
        }];
        let r2 =
            reframer.compute_reframe(1920, 1080, TargetAspect::Vertical9x16, &faces_right, None);

        // With smoothing, the crop shouldn't jump all the way to the right
        assert!(
            r2.crop_rect.x < 1700.0,
            "Smoothed crop should lag behind the face, got x={}",
            r2.crop_rect.x
        );
        assert!(
            r2.crop_rect.x > r1.crop_rect.x,
            "Crop should have moved right from {} to {}",
            r1.crop_rect.x,
            r2.crop_rect.x
        );
    }

    #[test]
    fn test_reframer_no_faces_centers_crop() {
        let mut reframer = SmartReframer::new(ReframeConfig::default());
        let result = reframer.compute_reframe(1920, 1080, TargetAspect::Square, &[], None);

        // With no subjects, crop should be centered
        let center = result.crop_rect.center();
        assert!((center.x - 960.0).abs() < 1.0);
        assert!((center.y - 540.0).abs() < 1.0);
    }

    #[test]
    fn test_compute_sequence() {
        let mut reframer = SmartReframer::new(ReframeConfig::default());
        let frames_faces = vec![
            vec![FaceBbox {
                rect: Rect::new(500.0, 400.0, 100.0, 100.0),
                confidence: 0.9,
            }],
            vec![FaceBbox {
                rect: Rect::new(510.0, 400.0, 100.0, 100.0),
                confidence: 0.9,
            }],
            vec![FaceBbox {
                rect: Rect::new(520.0, 400.0, 100.0, 100.0),
                confidence: 0.9,
            }],
        ];
        let results =
            reframer.compute_sequence(1920, 1080, TargetAspect::Vertical9x16, &frames_faces);
        assert_eq!(results.len(), 3);
        // Each successive frame should move slightly right
        assert!(results[1].crop_rect.x >= results[0].crop_rect.x);
    }
}
