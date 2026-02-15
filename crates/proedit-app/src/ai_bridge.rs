//! Bridge between AI features and the application.

#![allow(dead_code)]

use proedit_ai::auto_color::{analyze_and_correct, ColorCorrection};
use proedit_ai::scene_detect::{detect_scenes_by_difference, SceneBoundary, SceneDetectConfig};
use proedit_ai::AIEngine;
use proedit_core::FrameBuffer;

/// Run scene detection on a sequence of frames.
/// Returns scene boundaries as (frame_number, timestamp, confidence) tuples.
pub fn run_scene_detection(frames: &[FrameBuffer], fps: f64) -> Vec<SceneBoundary> {
    let config = SceneDetectConfig::default();
    detect_scenes_by_difference(frames, fps, &config)
}

/// Analyze a frame and produce auto color correction.
pub fn run_auto_color(frame: &FrameBuffer) -> ColorCorrection {
    analyze_and_correct(frame)
}

/// Initialize the AI engine with default settings.
pub fn init_ai_engine() -> AIEngine {
    AIEngine::default()
}
