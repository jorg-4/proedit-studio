//! Integration tests for the timeline subsystem.
//!
//! Exercises cross-crate interactions between proedit-core,
//! proedit-timeline, and proedit-media.

use proedit_core::{EasingCurve, FrameRate, KeyframeTrack, RationalTime};
use proedit_media::export::{ExportFormat, ExportJob};
use proedit_timeline::{
    Clip, ClipRef, EditCommand, Project, ProjectFile, Sequence, Track, UndoStack,
};

// ── Helpers ────────────────────────────────────────────────────

fn clip(name: &str, secs: i64) -> Clip {
    Clip::new(
        name,
        ClipRef::new("media/test.mp4", RationalTime::new(secs, 1)),
    )
}

fn build_project() -> Project {
    let mut project = Project::new("Integration Test Project");
    let mut seq = Sequence::new("Main Timeline", 1920, 1080, FrameRate::FPS_24);

    seq.video_tracks[0].append_clip(clip("Intro", 5));
    seq.video_tracks[0].append_clip(clip("Body", 30));
    seq.video_tracks[0].append_clip(clip("Outro", 10));
    seq.audio_tracks[0].append_clip(clip("Music", 45));

    project.add_sequence(seq);
    project
}

// ── Project assembly & timing ──────────────────────────────────

#[test]
fn project_duration_is_max_of_tracks() {
    let project = build_project();
    let seq = project.active_sequence().unwrap();
    assert_eq!(seq.duration(), RationalTime::new(45, 1));
}

#[test]
fn video_track_clip_count() {
    let project = build_project();
    let seq = project.active_sequence().unwrap();
    assert_eq!(seq.video_tracks[0].clip_count(), 3);
}

#[test]
fn item_at_time_finds_correct_clip() {
    let project = build_project();
    let seq = project.active_sequence().unwrap();
    let track = &seq.video_tracks[0];

    let (idx, offset) = track.item_at_time(RationalTime::ZERO).unwrap();
    assert_eq!(idx, 0);
    assert_eq!(offset, RationalTime::ZERO);

    let (idx, offset) = track.item_at_time(RationalTime::new(6, 1)).unwrap();
    assert_eq!(idx, 1);
    assert_eq!(offset, RationalTime::new(1, 1));

    let (idx, offset) = track.item_at_time(RationalTime::new(36, 1)).unwrap();
    assert_eq!(idx, 2);
    assert_eq!(offset, RationalTime::new(1, 1));
}

// ── Edit operations with undo ──────────────────────────────────

#[test]
fn insert_and_undo_restores_clip_count() {
    let mut project = build_project();
    let mut undo = UndoStack::new(100);
    let seq = project.active_sequence_mut().unwrap();
    let track = &mut seq.video_tracks[0];
    let track_id = track.id;

    assert_eq!(track.clip_count(), 3);

    let new_clip = clip("Inserted", 8);
    let cmd = EditCommand::InsertClip {
        track_id,
        index: 1,
        clip: new_clip.clone(),
    };
    track.insert_clip(1, new_clip);
    undo.push(cmd);

    assert_eq!(track.clip_count(), 4);
    assert_eq!(track.duration(), RationalTime::new(53, 1));

    let inverse = undo.undo().unwrap();
    if let EditCommand::RemoveClip { index, .. } = inverse {
        track.remove_item(index);
    } else {
        panic!("expected RemoveClip inverse");
    }

    assert_eq!(track.clip_count(), 3);
    assert_eq!(track.duration(), RationalTime::new(45, 1));
}

#[test]
fn toggle_enabled_is_self_inverse() {
    let mut project = build_project();
    let mut undo = UndoStack::new(100);
    let seq = project.active_sequence_mut().unwrap();
    let track = &mut seq.video_tracks[0];
    let track_id = track.id;

    let cmd = EditCommand::ToggleClipEnabled {
        track_id,
        clip_index: 0,
    };
    track.clip_at_mut(0).unwrap().enabled = false;
    undo.push(cmd);

    assert!(!track.clip_at(0).unwrap().enabled);

    let inverse = undo.undo().unwrap();
    assert!(matches!(inverse, EditCommand::ToggleClipEnabled { .. }));
    track.clip_at_mut(0).unwrap().enabled = true;

    assert!(track.clip_at(0).unwrap().enabled);
}

#[test]
fn batch_undo_reverses_all_operations() {
    let mut project = build_project();
    let mut undo = UndoStack::new(100);
    let seq = project.active_sequence_mut().unwrap();
    let track = &mut seq.video_tracks[0];
    let track_id = track.id;

    let c1 = clip("Batch1", 3);
    let c2 = clip("Batch2", 4);
    let batch = EditCommand::Batch(vec![
        EditCommand::InsertClip {
            track_id,
            index: 0,
            clip: c1.clone(),
        },
        EditCommand::InsertClip {
            track_id,
            index: 1,
            clip: c2.clone(),
        },
    ]);
    track.insert_clip(0, c1);
    track.insert_clip(1, c2);
    undo.push(batch);

    assert_eq!(track.clip_count(), 5);

    let inverse = undo.undo().unwrap();
    if let EditCommand::Batch(cmds) = inverse {
        assert_eq!(cmds.len(), 2);
        for cmd in &cmds {
            if let EditCommand::RemoveClip { index, .. } = cmd {
                track.remove_item(*index);
            }
        }
    } else {
        panic!("expected Batch inverse");
    }

    assert_eq!(track.clip_count(), 3);
}

// ── Serialization roundtrip ────────────────────────────────────

#[test]
fn project_survives_serialization_roundtrip() {
    let project = build_project();
    let file = ProjectFile::new(project);

    let json = file.to_json().unwrap();
    let loaded = ProjectFile::from_json(&json).unwrap();

    assert_eq!(loaded.project.name, "Integration Test Project");
    let seq = loaded.project.active_sequence().unwrap();
    assert_eq!(seq.video_tracks[0].clip_count(), 3);
    assert_eq!(seq.audio_tracks[0].clip_count(), 1);
    assert_eq!(seq.duration(), RationalTime::new(45, 1));
}

#[test]
fn edited_project_serializes_correctly() {
    let mut project = build_project();

    let seq = project.active_sequence_mut().unwrap();
    let mut v2 = Track::new_video("V2");
    v2.append_clip(clip("Overlay", 15));
    seq.video_tracks.push(v2);

    let file = ProjectFile::new(project);
    let json = file.to_json().unwrap();
    let loaded = ProjectFile::from_json(&json).unwrap();

    let seq = loaded.project.active_sequence().unwrap();
    assert_eq!(seq.video_tracks.len(), 2);
    assert_eq!(seq.video_tracks[1].clip_count(), 1);
}

// ── Export pipeline integration ────────────────────────────────

#[test]
fn export_job_computes_correct_frame_count() {
    let project = build_project();
    let seq = project.active_sequence().unwrap();

    let job = ExportJob::new("/tmp/out.mp4", ExportFormat::h264_hd());
    let total = job.total_frames(seq.duration());
    assert_eq!(total, 1080); // 45s × 24fps
}

#[test]
fn export_range_subsets_timeline() {
    let project = build_project();
    let seq = project.active_sequence().unwrap();

    let job = ExportJob::new("/tmp/body.mp4", ExportFormat::h264_hd())
        .with_range(RationalTime::new(5, 1), RationalTime::new(35, 1));

    let total = job.total_frames(seq.duration());
    assert_eq!(total, 720); // 30s × 24fps
}

#[test]
fn export_4k_produces_correct_ffmpeg_args() {
    let format = ExportFormat::h265_4k();
    let job = ExportJob::new("/tmp/out.mp4", format);
    let args = job.ffmpeg_args();

    assert!(args.contains(&"libx265".to_string()));
    assert!(args.contains(&"3840x2160".to_string()));
}

// ── Timecode integration ───────────────────────────────────────

#[test]
fn clip_boundaries_produce_valid_timecodes() {
    let project = build_project();
    let seq = project.active_sequence().unwrap();
    let track = &seq.video_tracks[0];
    let rate = seq.frame_rate;

    assert_eq!(track.item_start_time(0).to_timecode(rate), "00:00:00:00");
    assert_eq!(track.item_start_time(1).to_timecode(rate), "00:00:05:00");
    assert_eq!(track.item_start_time(2).to_timecode(rate), "00:00:35:00");
    assert_eq!(seq.duration().to_timecode(rate), "00:00:45:00");
}

#[test]
fn drop_frame_timecode_at_sequence_boundary() {
    let mut project = Project::new("DF Test");
    let mut seq = Sequence::new("DF Seq", 1920, 1080, FrameRate::FPS_29_97);
    seq.video_tracks[0].append_clip(clip("Long", 3600));
    project.add_sequence(seq);

    let seq = project.active_sequence().unwrap();
    let dur = seq.duration();
    let tc = dur.to_timecode_drop_frame(seq.frame_rate);
    assert_eq!(tc, "01:00:00;00");
}

// ── Track operations ───────────────────────────────────────────

#[test]
fn consolidate_gaps_merges_adjacent() {
    let mut track = Track::new_video("Test");
    track.append_clip(clip("A", 5));
    track.append_gap(RationalTime::new(2, 1));
    track.append_gap(RationalTime::new(3, 1));
    track.append_clip(clip("B", 5));

    assert_eq!(track.items.len(), 4);

    track.consolidate_gaps();
    assert_eq!(track.items.len(), 3);
    assert_eq!(track.duration(), RationalTime::new(15, 1));
}

#[test]
fn find_clip_by_uuid() {
    let mut track = Track::new_video("Test");
    let c = clip("Target", 10);
    let target_id = c.id;
    track.append_clip(clip("Before", 5));
    track.append_clip(c);
    track.append_clip(clip("After", 5));

    let (idx, found) = track.find_clip(target_id).unwrap();
    assert_eq!(idx, 1);
    assert_eq!(found.name, "Target");
}

// ── Keyframe + timeline timing ─────────────────────────────────

#[test]
fn keyframe_values_at_clip_boundaries() {
    let mut kf = KeyframeTrack::new("opacity");
    kf.set(RationalTime::ZERO, 0.0, EasingCurve::Linear);
    kf.set(RationalTime::new(10, 1), 1.0, EasingCurve::Linear);

    // At clip boundary 5s → opacity should be 0.5
    let opacity = kf.evaluate(RationalTime::new(5, 1));
    assert!((opacity - 0.5).abs() < 0.001);

    // At clip start → 0.0
    assert!((kf.evaluate(RationalTime::ZERO)).abs() < 0.001);

    // At clip end → 1.0
    assert!((kf.evaluate(RationalTime::new(10, 1)) - 1.0).abs() < 0.001);
}
