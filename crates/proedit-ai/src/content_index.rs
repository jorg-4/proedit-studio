//! Visual content indexing using CLIP embeddings.
//!
//! Encodes sampled video frames into vector representations using CLIP (or SigLIP),
//! enabling natural language search ("sunset over water" → matching frames).
//!
//! Also computes: dominant colors per shot, shot type classification, and
//! camera motion estimation.

use crate::error::{AiError, AiResult};
use proedit_core::FrameBuffer;
use serde::{Deserialize, Serialize};

/// A vector embedding for a single frame.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameEmbedding {
    /// Frame number in the source video.
    pub frame_number: i64,
    /// Timestamp in seconds.
    pub timestamp_secs: f64,
    /// Embedding vector (typically 512 dimensions for CLIP ViT-B/32).
    pub vector: Vec<f32>,
}

/// Shot type classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ShotType {
    /// Extreme close-up (ECU).
    ExtremeCloseUp,
    /// Close-up (CU).
    CloseUp,
    /// Medium close-up (MCU).
    MediumCloseUp,
    /// Medium shot (MS).
    Medium,
    /// Medium wide / cowboy shot.
    MediumWide,
    /// Wide shot (WS).
    Wide,
    /// Extreme wide / establishing shot.
    ExtremeWide,
    /// Overhead / aerial / drone shot.
    Aerial,
    /// Could not determine.
    Unknown,
}

impl ShotType {
    /// Display name for UI.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::ExtremeCloseUp => "Extreme Close-Up",
            Self::CloseUp => "Close-Up",
            Self::MediumCloseUp => "Medium Close-Up",
            Self::Medium => "Medium",
            Self::MediumWide => "Medium Wide",
            Self::Wide => "Wide",
            Self::ExtremeWide => "Extreme Wide",
            Self::Aerial => "Aerial",
            Self::Unknown => "Unknown",
        }
    }
}

/// Camera motion type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CameraMotion {
    Static,
    PanLeft,
    PanRight,
    TiltUp,
    TiltDown,
    ZoomIn,
    ZoomOut,
    Dolly,
    Handheld,
    Unknown,
}

impl CameraMotion {
    /// Display name for UI.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Static => "Static",
            Self::PanLeft => "Pan Left",
            Self::PanRight => "Pan Right",
            Self::TiltUp => "Tilt Up",
            Self::TiltDown => "Tilt Down",
            Self::ZoomIn => "Zoom In",
            Self::ZoomOut => "Zoom Out",
            Self::Dolly => "Dolly",
            Self::Handheld => "Handheld",
            Self::Unknown => "Unknown",
        }
    }
}

/// Dominant color extracted from a frame or scene.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct DominantColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    /// Fraction of the frame this color occupies (0.0 to 1.0).
    pub weight: f32,
}

/// Visual metadata for a scene/shot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneVisualInfo {
    /// Frame range for this scene.
    pub start_frame: i64,
    /// Frame range for this scene.
    pub end_frame: i64,
    /// Classified shot type.
    pub shot_type: ShotType,
    /// Estimated camera motion.
    pub camera_motion: CameraMotion,
    /// Top dominant colors (up to 5).
    pub dominant_colors: Vec<DominantColor>,
    /// Average brightness (0.0 to 1.0).
    pub avg_brightness: f32,
}

/// Result of a text-based visual search.
#[derive(Debug, Clone)]
pub struct SearchResult {
    /// Frame number.
    pub frame_number: i64,
    /// Timestamp in seconds.
    pub timestamp_secs: f64,
    /// Cosine similarity score (higher = more relevant).
    pub score: f32,
}

/// In-memory vector index for visual search.
pub struct ContentIndex {
    embeddings: Vec<FrameEmbedding>,
}

impl ContentIndex {
    /// Create a new empty content index.
    pub fn new() -> Self {
        Self {
            embeddings: Vec::new(),
        }
    }

    /// Add a frame embedding to the index.
    pub fn add_embedding(&mut self, embedding: FrameEmbedding) {
        self.embeddings.push(embedding);
    }

    /// Number of indexed frames.
    pub fn len(&self) -> usize {
        self.embeddings.len()
    }

    /// Whether the index is empty.
    pub fn is_empty(&self) -> bool {
        self.embeddings.is_empty()
    }

    /// Search by text query embedding (cosine similarity).
    ///
    /// In production, the query text would first be encoded through CLIP's
    /// text encoder to produce a vector. Here we accept a pre-computed vector.
    pub fn search_by_vector(&self, query_vector: &[f32], top_k: usize) -> Vec<SearchResult> {
        let mut scored: Vec<SearchResult> = self
            .embeddings
            .iter()
            .map(|emb| {
                let score = cosine_similarity(&emb.vector, query_vector);
                SearchResult {
                    frame_number: emb.frame_number,
                    timestamp_secs: emb.timestamp_secs,
                    score,
                }
            })
            .collect();

        // Sort by score descending
        scored.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(top_k);
        scored
    }

    /// Get all embeddings.
    pub fn embeddings(&self) -> &[FrameEmbedding] {
        &self.embeddings
    }

    /// Save embeddings to a binary file (for mmap-ing).
    pub fn save_binary(&self, path: &std::path::Path) -> AiResult<()> {
        let json = serde_json::to_vec(&self.embeddings).map_err(|e| {
            AiError::SerializationError(format!("Failed to serialize embeddings: {e}"))
        })?;
        std::fs::write(path, json)?;
        Ok(())
    }

    /// Load embeddings from a binary file.
    pub fn load_binary(path: &std::path::Path) -> AiResult<Self> {
        let data = std::fs::read(path)?;
        let embeddings: Vec<FrameEmbedding> = serde_json::from_slice(&data).map_err(|e| {
            AiError::SerializationError(format!("Failed to deserialize embeddings: {e}"))
        })?;
        Ok(Self { embeddings })
    }
}

impl Default for ContentIndex {
    fn default() -> Self {
        Self::new()
    }
}

/// Cosine similarity between two vectors.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }

    let mut dot = 0.0_f32;
    let mut norm_a = 0.0_f32;
    let mut norm_b = 0.0_f32;

    for i in 0..a.len() {
        dot += a[i] * b[i];
        norm_a += a[i] * a[i];
        norm_b += b[i] * b[i];
    }

    let denom = norm_a.sqrt() * norm_b.sqrt();
    if denom > 0.0 {
        dot / denom
    } else {
        0.0
    }
}

/// Extract dominant colors from a frame using k-means-like clustering.
pub fn extract_dominant_colors(frame: &FrameBuffer, max_colors: usize) -> Vec<DominantColor> {
    let w = frame.width as usize;
    let h = frame.height as usize;
    let plane = frame.primary_plane();

    // Sample pixels (every 4th pixel for speed)
    let mut pixels: Vec<[u8; 3]> = Vec::new();
    for y in (0..h).step_by(4) {
        let row = plane.row(y as u32);
        for x in (0..w).step_by(4) {
            let base = x * 4;
            if base + 2 < row.len() {
                pixels.push([row[base], row[base + 1], row[base + 2]]);
            }
        }
    }

    if pixels.is_empty() {
        return Vec::new();
    }

    // Simple quantization: group into buckets of 64 levels per channel
    let mut buckets: std::collections::HashMap<(u8, u8, u8), usize> =
        std::collections::HashMap::new();
    for px in &pixels {
        let key = (px[0] / 64, px[1] / 64, px[2] / 64);
        *buckets.entry(key).or_insert(0) += 1;
    }

    // Sort by frequency and take top colors
    let mut sorted: Vec<_> = buckets.into_iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(&a.1));

    let total = pixels.len() as f32;
    sorted
        .into_iter()
        .take(max_colors)
        .map(|((r, g, b), count)| DominantColor {
            r: r * 64 + 32, // center of bucket
            g: g * 64 + 32,
            b: b * 64 + 32,
            weight: count as f32 / total,
        })
        .collect()
}

/// Classify shot type from a frame using heuristic analysis.
/// In production, this would use a trained classifier or CLIP zero-shot.
pub fn classify_shot_type(frame: &FrameBuffer) -> ShotType {
    let w = frame.width as usize;
    let h = frame.height as usize;
    let plane = frame.primary_plane();

    // Compute edge density as a proxy for shot type
    // More edges in center = closer shot, more uniform = wider shot
    let center_third_x = w / 3..2 * w / 3;
    let center_third_y = h / 3..2 * h / 3;

    let mut center_variance = 0.0_f64;
    let mut center_count = 0u64;
    let mut total_variance = 0.0_f64;
    let mut total_count = 0u64;

    for y in (1..h - 1).step_by(2) {
        let row_prev = plane.row((y - 1) as u32);
        let row_curr = plane.row(y as u32);
        let row_next = plane.row((y + 1) as u32);

        for x in (1..w - 1).step_by(2) {
            let base = x * 4;
            if base + 4 >= row_curr.len() {
                break;
            }

            // Simple gradient magnitude (Sobel-like)
            let dx = (row_curr[base + 4] as f64) - (row_curr[base - 4] as f64);
            let dy = (row_next[base] as f64) - (row_prev[base] as f64);
            let grad = (dx * dx + dy * dy).sqrt();

            total_variance += grad;
            total_count += 1;

            if center_third_x.contains(&x) && center_third_y.contains(&y) {
                center_variance += grad;
                center_count += 1;
            }
        }
    }

    if total_count == 0 || center_count == 0 {
        return ShotType::Unknown;
    }

    let center_density = center_variance / center_count as f64;
    let total_density = total_variance / total_count as f64;
    let center_ratio = if total_density > 0.0 {
        center_density / total_density
    } else {
        1.0
    };

    // Higher center ratio = more detail in center = closer shot
    if center_ratio > 1.5 {
        ShotType::CloseUp
    } else if center_ratio > 1.2 {
        ShotType::Medium
    } else if center_ratio > 0.9 {
        ShotType::MediumWide
    } else {
        ShotType::Wide
    }
}

/// Estimate average brightness of a frame (0.0 to 1.0).
pub fn average_brightness(frame: &FrameBuffer) -> f32 {
    let w = frame.width as usize;
    let h = frame.height as usize;
    let plane = frame.primary_plane();
    let mut sum = 0.0_f64;
    let mut count = 0u64;

    for y in (0..h).step_by(4) {
        let row = plane.row(y as u32);
        for x in (0..w).step_by(4) {
            let base = x * 4;
            if base + 2 < row.len() {
                // Luminance formula: 0.299*R + 0.587*G + 0.114*B
                let lum = 0.299 * row[base] as f64
                    + 0.587 * row[base + 1] as f64
                    + 0.114 * row[base + 2] as f64;
                sum += lum;
                count += 1;
            }
        }
    }

    if count > 0 {
        (sum / count as f64 / 255.0) as f32
    } else {
        0.0
    }
}

#[cfg(test)]
fn make_solid_frame(w: u32, h: u32, r: u8, g: u8, b: u8) -> FrameBuffer {
    let mut frame = FrameBuffer::new(w, h, proedit_core::PixelFormat::Rgba8);
    let plane = frame.primary_plane_mut();
    for y in 0..h {
        let row = plane.row_mut(y);
        for x in 0..w as usize {
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
    fn test_cosine_similarity_identical() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![1.0, 2.0, 3.0];
        let sim = cosine_similarity(&a, &b);
        assert!((sim - 1.0).abs() < 1e-5);
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let a = vec![1.0, 0.0];
        let b = vec![0.0, 1.0];
        let sim = cosine_similarity(&a, &b);
        assert!(sim.abs() < 1e-5);
    }

    #[test]
    fn test_cosine_similarity_opposite() {
        let a = vec![1.0, 0.0];
        let b = vec![-1.0, 0.0];
        let sim = cosine_similarity(&a, &b);
        assert!((sim - (-1.0)).abs() < 1e-5);
    }

    #[test]
    fn test_content_index_search() {
        let mut index = ContentIndex::new();
        index.add_embedding(FrameEmbedding {
            frame_number: 0,
            timestamp_secs: 0.0,
            vector: vec![1.0, 0.0, 0.0],
        });
        index.add_embedding(FrameEmbedding {
            frame_number: 30,
            timestamp_secs: 1.0,
            vector: vec![0.0, 1.0, 0.0],
        });
        index.add_embedding(FrameEmbedding {
            frame_number: 60,
            timestamp_secs: 2.0,
            vector: vec![0.9, 0.1, 0.0],
        });

        let query = vec![1.0, 0.0, 0.0];
        let results = index.search_by_vector(&query, 2);
        assert_eq!(results.len(), 2);
        // Frame 0 should be the best match (exact direction)
        assert_eq!(results[0].frame_number, 0);
        assert!(results[0].score > 0.9);
    }

    #[test]
    fn test_extract_dominant_colors() {
        let frame = make_solid_frame(64, 64, 200, 100, 50);
        let colors = extract_dominant_colors(&frame, 3);
        assert!(!colors.is_empty());
        // The dominant color should be close to (200, 100, 50)
        assert!(colors[0].weight > 0.5);
    }

    #[test]
    fn test_average_brightness() {
        let black = make_solid_frame(32, 32, 0, 0, 0);
        assert!(average_brightness(&black) < 0.01);

        let white = make_solid_frame(32, 32, 255, 255, 255);
        assert!((average_brightness(&white) - 1.0).abs() < 0.01);

        let mid = make_solid_frame(32, 32, 128, 128, 128);
        let brightness = average_brightness(&mid);
        assert!(
            (brightness - 0.502).abs() < 0.02,
            "Mid-gray brightness should be ~0.5, got {brightness}"
        );
    }

    #[test]
    fn test_index_save_load() {
        let tmp = tempfile::tempdir().expect("failed to create tempdir");
        let path = tmp.path().join("test_embeddings.bin");

        let mut index = ContentIndex::new();
        index.add_embedding(FrameEmbedding {
            frame_number: 42,
            timestamp_secs: 1.4,
            vector: vec![0.5, 0.3, 0.2],
        });
        index.save_binary(&path).unwrap();

        let loaded = ContentIndex::load_binary(&path).unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded.embeddings()[0].frame_number, 42);
    }

    #[test]
    fn test_shot_type_display() {
        assert_eq!(ShotType::CloseUp.display_name(), "Close-Up");
        assert_eq!(ShotType::Wide.display_name(), "Wide");
        assert_eq!(ShotType::Aerial.display_name(), "Aerial");
    }

    #[test]
    fn test_camera_motion_display() {
        assert_eq!(CameraMotion::Static.display_name(), "Static");
        assert_eq!(CameraMotion::PanLeft.display_name(), "Pan Left");
    }
}
