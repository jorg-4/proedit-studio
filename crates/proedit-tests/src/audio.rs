//! Integration tests for the audio subsystem.

use proedit_audio::AudioEngine;

#[test]
fn audio_engine_creates_successfully() {
    let engine = AudioEngine::default();
    assert!(!engine.is_playing());
    assert_eq!(engine.sample_rate(), 48000);
    assert_eq!(engine.channels(), 2);
}

#[test]
fn audio_engine_output_buffer_is_shared() {
    let engine = AudioEngine::default();
    let buf1 = engine.output_buffer();
    let buf2 = engine.output_buffer();
    assert!(std::sync::Arc::ptr_eq(&buf1, &buf2));
}

#[test]
fn audio_engine_play_stop_cycle() {
    let mut engine = AudioEngine::default();
    assert!(!engine.is_playing());
    engine.play();
    assert!(engine.is_playing());
    engine.stop();
    assert!(!engine.is_playing());
}

#[test]
fn mixer_muted_channel_produces_silence() {
    use proedit_audio::mixer::Mixer;

    let mut mixer = Mixer::new(1, 4096);
    mixer.channel_mut(0).unwrap().muted = true;

    let source = vec![1.0f32; 128];
    mixer.mix(&[&source], 64);

    let mut out = vec![0.0f32; 128];
    let read = mixer.output_buffer.read(&mut out);
    assert_eq!(read, 128);
    assert!(out.iter().all(|&s| s.abs() < 1e-6));
}

#[test]
fn mixer_solo_isolates_channel() {
    use proedit_audio::mixer::Mixer;

    let mut mixer = Mixer::new(2, 4096);
    mixer.channel_mut(0).unwrap().solo = true;

    let ch0 = vec![0.8f32; 64];
    let ch1 = vec![0.8f32; 64];
    mixer.mix(&[&ch0, &ch1], 32);

    let mut out = vec![0.0f32; 64];
    mixer.output_buffer.read(&mut out);

    for s in &out {
        assert!(*s > 0.0);
        assert!(*s < 1.0);
    }
}

#[test]
fn ring_buffer_high_throughput() {
    use proedit_audio::RingBuffer;

    let buf = RingBuffer::new(4096);
    let mut total_written = 0usize;
    let mut total_read = 0usize;

    for _ in 0..100 {
        let data: Vec<f32> = (0..32).map(|i| i as f32).collect();
        let written = buf.write(&data);
        total_written += written;

        let mut out = vec![0.0f32; 32];
        let read = buf.read(&mut out);
        total_read += read;
    }

    assert_eq!(total_written, total_read);
}

#[test]
fn waveform_pixel_count_matches_duration() {
    use proedit_audio::Waveform;

    let sample_rate = 48000;
    let duration_secs = 10;
    let samples: Vec<f32> = (0..sample_rate * duration_secs)
        .map(|i| (i as f32 * 0.001).sin())
        .collect();

    let spp = 480;
    let waveform = Waveform::compute(&samples, spp, sample_rate as u32);
    assert_eq!(waveform.data.len(), 1000);
}
