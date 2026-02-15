//! Benchmarks for proedit-core time operations.
//!
//! Run with: cargo bench -p proedit-core

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use proedit_core::{CubicBezier, EasingCurve, FrameRate, KeyframeTrack, RationalTime};

fn bench_rational_time_arithmetic(c: &mut Criterion) {
    let a = RationalTime::new(1001, 30);
    let b = RationalTime::new(500, 24);

    c.bench_function("rational_time_add", |bencher| {
        bencher.iter(|| black_box(a) + black_box(b));
    });

    c.bench_function("rational_time_mul_i64", |bencher| {
        bencher.iter(|| black_box(a) * black_box(100));
    });
}

fn bench_frame_conversion(c: &mut Criterion) {
    let time = RationalTime::new(3600, 1); // 1 hour
    let rate = FrameRate::FPS_24;

    c.bench_function("to_frames_1hr", |bencher| {
        bencher.iter(|| black_box(time).to_frames(black_box(rate)));
    });

    c.bench_function("from_frames_86400", |bencher| {
        bencher.iter(|| RationalTime::from_frames(black_box(86400), black_box(rate)));
    });
}

fn bench_timecode_formatting(c: &mut Criterion) {
    let time = RationalTime::new(3723, 1); // 1:02:03
    let rate_24 = FrameRate::FPS_24;
    let rate_2997 = FrameRate::FPS_29_97;

    c.bench_function("to_timecode_24fps", |bencher| {
        bencher.iter(|| black_box(time).to_timecode(black_box(rate_24)));
    });

    c.bench_function("to_timecode_dropframe_29.97", |bencher| {
        bencher.iter(|| black_box(time).to_timecode_drop_frame(black_box(rate_2997)));
    });

    c.bench_function("from_timecode_parse", |bencher| {
        bencher.iter(|| RationalTime::from_timecode(black_box("01:02:03:04"), black_box(rate_24)));
    });
}

fn bench_keyframe_evaluation(c: &mut Criterion) {
    let mut track = KeyframeTrack::new("bench_param");
    // Create a track with 100 keyframes
    for i in 0..100 {
        let easing = if i % 2 == 0 {
            EasingCurve::Linear
        } else {
            EasingCurve::Bezier(CubicBezier::EASE_IN_OUT)
        };
        track.set(RationalTime::new(i, 1), (i as f64 * 0.1).sin(), easing);
    }

    c.bench_function("keyframe_evaluate_linear_100kf", |bencher| {
        bencher.iter(|| track.evaluate(black_box(RationalTime::new(50, 1))));
    });

    c.bench_function("keyframe_evaluate_bezier_100kf", |bencher| {
        bencher.iter(|| track.evaluate(black_box(RationalTime::new(51, 1))));
    });
}

criterion_group!(
    benches,
    bench_rational_time_arithmetic,
    bench_frame_conversion,
    bench_timecode_formatting,
    bench_keyframe_evaluation,
);
criterion_main!(benches);
