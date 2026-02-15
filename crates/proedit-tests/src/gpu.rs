//! Integration tests for the GPU subsystem.
//!
//! Exercises CPU-side logic only — no actual GPU required.

use proedit_core::{FrameRate, RationalTime};
use proedit_gpu::{BlendMode, FrameCache, NodeOp, RenderGraph};

#[test]
fn render_graph_models_three_layer_composite() {
    let mut graph = RenderGraph::new();
    let hd = (1920, 1080);

    let src_bg = graph.add_node(NodeOp::Source { frame_id: 0 }, vec![], hd);
    let src_fg = graph.add_node(NodeOp::Source { frame_id: 1 }, vec![], hd);
    let src_title = graph.add_node(NodeOp::Source { frame_id: 2 }, vec![], hd);

    let comp1 = graph.add_node(
        NodeOp::Composite {
            blend_mode: BlendMode::Normal as u32,
            opacity: 0.8,
        },
        vec![src_bg, src_fg],
        hd,
    );

    let comp2 = graph.add_node(
        NodeOp::Composite {
            blend_mode: BlendMode::Screen as u32,
            opacity: 1.0,
        },
        vec![comp1, src_title],
        hd,
    );

    let output = graph.add_node(NodeOp::Output, vec![comp2], hd);

    let sorted = graph.topological_sort().unwrap();
    assert_eq!(sorted.len(), 6);

    // Sources must come before composites, composites before output
    let pos = |id| sorted.iter().position(|nid| *nid == id).unwrap();
    assert!(pos(src_bg) < pos(comp1));
    assert!(pos(src_fg) < pos(comp1));
    assert!(pos(comp1) < pos(comp2));
    assert!(pos(src_title) < pos(comp2));
    assert!(pos(comp2) < pos(output));
}

#[test]
fn frame_cache_with_frame_id_keys() {
    let mut cache = FrameCache::new(100 * 1024 * 1024);
    let frame_size = 1920 * 1080 * 4;

    for frame_num in 0u64..10 {
        let data = vec![0u8; frame_size];
        cache.insert(frame_num, data, 1920, 1080);
    }

    for frame_num in 0u64..10 {
        assert!(
            cache.get(frame_num).is_some(),
            "frame {} should be cached",
            frame_num
        );
    }
}

#[test]
fn frame_cache_evicts_under_pressure() {
    let frame_size = 1920 * 1080 * 4;
    let mut cache = FrameCache::new(frame_size * 2 + 1);

    cache.insert(0, vec![0u8; frame_size], 1920, 1080);
    cache.insert(1, vec![0u8; frame_size], 1920, 1080);
    cache.insert(2, vec![0u8; frame_size], 1920, 1080); // should evict frame 0

    assert!(cache.get(0).is_none(), "LRU frame should be evicted");
    assert!(cache.get(2).is_some());
}

#[test]
fn all_blend_modes_have_names() {
    for mode in &BlendMode::ALL {
        assert!(!mode.name().is_empty(), "{:?} has no name", mode);
        assert!(!mode.category().is_empty(), "{:?} has no category", mode);
    }
    assert_eq!(BlendMode::ALL.len(), 28);
}

#[test]
fn timecode_as_frame_id_for_cache() {
    let rate = FrameRate::FPS_24;
    let time = RationalTime::new(5, 1);
    let frame = time.to_frames(rate) as u64;

    let mut cache = FrameCache::new(10 * 1024 * 1024);
    cache.insert(frame, vec![0u8; 100], 1920, 1080);

    // Retrieve by the same frame number computed from timecode
    let tc = "00:00:05:00";
    let parsed = RationalTime::from_timecode(tc, rate).unwrap();
    let parsed_frame = parsed.to_frames(rate) as u64;

    assert_eq!(frame, parsed_frame);
    assert!(cache.get(parsed_frame).is_some());
}
