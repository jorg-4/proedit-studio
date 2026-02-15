//! CPU frame compositor — builds a render graph per frame and composites layers.
//!
//! Uses `RenderGraph` from `proedit-gpu` for dependency ordering, but executes
//! compositing on the CPU for portability. GPU path can be added later using
//! `GpuContext`, `BlitPipeline`, and `TexturePool`.

#![allow(dead_code)]

use proedit_core::{FrameBuffer, PixelFormat};
use proedit_gpu::render_graph::{NodeId, NodeOp, RenderGraph};
use proedit_ui::timeline::TimelineClip;

/// A composited output frame.
pub struct CompositeFrame {
    pub buffer: FrameBuffer,
}

/// Configuration for the compositor.
pub struct CompositorConfig {
    pub width: u32,
    pub height: u32,
}

impl Default for CompositorConfig {
    fn default() -> Self {
        Self {
            width: 1920,
            height: 1080,
        }
    }
}

/// Build a render graph for the given playhead position.
///
/// Determines which clips are visible at `playhead_frame`, creates source
/// nodes for each, and chains them through a composite node to the output.
pub fn build_render_graph(
    clips: &[TimelineClip],
    playhead_frame: f32,
    config: &CompositorConfig,
) -> (RenderGraph, NodeId) {
    let mut graph = RenderGraph::new();
    let size = (config.width, config.height);

    // Find clips visible at the current playhead, sorted by track (back to front)
    let mut visible: Vec<&TimelineClip> = clips
        .iter()
        .filter(|c| playhead_frame >= c.start && playhead_frame < c.start + c.dur)
        .collect();
    visible.sort_by_key(|c| std::cmp::Reverse(c.track));

    if visible.is_empty() {
        // Black frame
        let src = graph.add_node(NodeOp::Source { frame_id: 0 }, vec![], size);
        let out = graph.add_node(NodeOp::Output, vec![src], size);
        return (graph, out);
    }

    // Create source nodes for each visible clip
    let source_nodes: Vec<NodeId> = visible
        .iter()
        .enumerate()
        .map(|(i, _clip)| graph.add_node(NodeOp::Source { frame_id: i as u64 }, vec![], size))
        .collect();

    // Chain composites: bottom layer first, each subsequent layer composited on top
    let mut current = source_nodes[0];
    for &layer in &source_nodes[1..] {
        current = graph.add_node(
            NodeOp::Composite {
                blend_mode: 0, // Normal
                opacity: 1.0,
            },
            vec![current, layer],
            size,
        );
    }

    let out = graph.add_node(NodeOp::Output, vec![current], size);
    (graph, out)
}

/// Composite all visible clips at the given playhead into a single RGBA8 frame.
///
/// This is the CPU fallback path. Each "source" node produces a solid-color
/// frame from the clip's color (placeholder for decoded video frames).
pub fn composite_frame(
    clips: &[TimelineClip],
    playhead_frame: f32,
    config: &CompositorConfig,
) -> CompositeFrame {
    let w = config.width;
    let h = config.height;

    // Collect visible clips sorted by track (back to front: higher track = further back)
    let mut visible: Vec<&TimelineClip> = clips
        .iter()
        .filter(|c| playhead_frame >= c.start && playhead_frame < c.start + c.dur)
        .collect();
    visible.sort_by_key(|c| std::cmp::Reverse(c.track));

    // Start with black background
    let mut output = FrameBuffer::new(w, h, PixelFormat::Rgba8);

    if visible.is_empty() {
        return CompositeFrame { buffer: output };
    }

    // Simple over-compositing: paint each layer on top
    for clip in &visible {
        let [r, g, b, a] = clip.color.to_srgba_unmultiplied();
        let alpha = a as f32 / 255.0;
        let plane = output.primary_plane_mut();

        for y in 0..h {
            let row = plane.row_mut(y);
            for x in 0..w as usize {
                let idx = x * 4;
                // Alpha-over compositing
                let dst_r = row[idx] as f32;
                let dst_g = row[idx + 1] as f32;
                let dst_b = row[idx + 2] as f32;
                row[idx] = (r as f32 * alpha + dst_r * (1.0 - alpha)) as u8;
                row[idx + 1] = (g as f32 * alpha + dst_g * (1.0 - alpha)) as u8;
                row[idx + 2] = (b as f32 * alpha + dst_b * (1.0 - alpha)) as u8;
                row[idx + 3] = 255;
            }
        }
    }

    CompositeFrame { buffer: output }
}

/// Render a single solid-color RGBA8 frame (utility for export pipeline).
pub fn render_black_frame(width: u32, height: u32) -> Vec<u8> {
    vec![0u8; (width * height * 4) as usize]
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::Color32;
    use proedit_ui::timeline::ClipKind;

    fn make_clip(id: usize, start: f32, dur: f32, track: usize, color: Color32) -> TimelineClip {
        TimelineClip {
            id,
            name: format!("clip_{id}"),
            color,
            start,
            dur,
            track,
            clip_type: ClipKind::Video,
        }
    }

    #[test]
    fn test_build_graph_empty() {
        let (graph, _out) = build_render_graph(&[], 0.0, &CompositorConfig::default());
        // Should have source + output = 2 nodes
        assert_eq!(graph.node_count(), 2);
    }

    #[test]
    fn test_build_graph_single_clip() {
        let clips = vec![make_clip(1, 0.0, 100.0, 0, Color32::RED)];
        let (graph, _out) = build_render_graph(&clips, 50.0, &CompositorConfig::default());
        // source + output = 2
        assert_eq!(graph.node_count(), 2);
    }

    #[test]
    fn test_build_graph_two_clips_overlapping() {
        let clips = vec![
            make_clip(1, 0.0, 100.0, 0, Color32::RED),
            make_clip(2, 0.0, 100.0, 1, Color32::BLUE),
        ];
        let (graph, _out) = build_render_graph(&clips, 50.0, &CompositorConfig::default());
        // 2 sources + 1 composite + output = 4
        assert_eq!(graph.node_count(), 4);
        assert!(graph.topological_sort().is_some());
    }

    #[test]
    fn test_build_graph_clip_not_visible() {
        let clips = vec![make_clip(1, 100.0, 50.0, 0, Color32::RED)];
        let (graph, _out) = build_render_graph(&clips, 0.0, &CompositorConfig::default());
        // Clip not visible at frame 0, so just black source + output
        assert_eq!(graph.node_count(), 2);
    }

    #[test]
    fn test_composite_empty() {
        let result = composite_frame(&[], 0.0, &CompositorConfig::default());
        // Should be a black frame
        let plane = result.buffer.primary_plane();
        let row = plane.row(0);
        assert_eq!(row[0], 0); // R
        assert_eq!(row[1], 0); // G
        assert_eq!(row[2], 0); // B
    }

    #[test]
    fn test_composite_single_opaque_clip() {
        let clips = vec![make_clip(1, 0.0, 100.0, 0, Color32::from_rgb(200, 100, 50))];
        let config = CompositorConfig {
            width: 4,
            height: 4,
        };
        let result = composite_frame(&clips, 50.0, &config);
        let plane = result.buffer.primary_plane();
        let row = plane.row(0);
        assert_eq!(row[0], 200);
        assert_eq!(row[1], 100);
        assert_eq!(row[2], 50);
        assert_eq!(row[3], 255);
    }

    #[test]
    fn test_composite_two_layers() {
        let clips = vec![
            make_clip(
                1,
                0.0,
                100.0,
                2,
                Color32::from_rgba_unmultiplied(255, 0, 0, 255),
            ),
            make_clip(
                2,
                0.0,
                100.0,
                1,
                Color32::from_rgba_unmultiplied(0, 0, 255, 128),
            ),
        ];
        let config = CompositorConfig {
            width: 2,
            height: 2,
        };
        let result = composite_frame(&clips, 50.0, &config);
        let plane = result.buffer.primary_plane();
        let row = plane.row(0);
        // Blue (track 1, front) at ~50% over Red (track 2, back)
        // R: 255 * (1 - 0.502) + 0 * 0.502 ≈ 127
        // B: 0 * (1 - 0.502) + 255 * 0.502 ≈ 128
        assert!(row[0] > 100 && row[0] < 150, "R = {}", row[0]);
        assert!(row[2] > 100 && row[2] < 150, "B = {}", row[2]);
    }

    #[test]
    fn test_render_black_frame() {
        let frame = render_black_frame(4, 4);
        assert_eq!(frame.len(), 64); // 4*4*4 bytes
        assert!(frame.iter().all(|&b| b == 0));
    }
}
