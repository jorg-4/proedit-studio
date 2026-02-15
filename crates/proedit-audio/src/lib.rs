//! ProEdit Audio - Audio engine
//!
//! Handles audio playback, mixing, and effects.
//!
//! Architecture:
//! - `RingBuffer`: Lock-free SPSC buffer between mixer thread and audio callback
//! - `Mixer`: Combines multiple channels with volume/pan/solo/mute
//! - `Waveform`: Pre-computed waveform data for UI display
//! - `AudioEngine`: Top-level orchestrator

pub mod mixer;
pub mod ring_buffer;
pub mod waveform;

pub use mixer::{Mixer, MixerChannel};
pub use ring_buffer::RingBuffer;
pub use waveform::{Waveform, WaveformSample};

use proedit_core::Result;
use std::sync::Arc;
use tracing::info;

/// Audio engine state.
pub struct AudioEngine {
    sample_rate: u32,
    channels: u16,
    playing: bool,
    /// The mixer for combining audio tracks.
    pub mixer: Mixer,
}

impl AudioEngine {
    /// Create a new audio engine.
    pub fn new() -> Result<Self> {
        info!("Initializing audio engine");
        // Buffer size: ~100ms at 48kHz stereo
        let buffer_samples = 48000 / 10 * 2;
        Ok(Self {
            sample_rate: 48000,
            channels: 2,
            playing: false,
            mixer: Mixer::new(3, buffer_samples),
        })
    }

    /// Start audio playback.
    pub fn play(&mut self) {
        self.playing = true;
        info!("Audio playback started");
    }

    /// Stop audio playback.
    pub fn stop(&mut self) {
        self.playing = false;
        self.mixer.output_buffer.clear();
        info!("Audio playback stopped");
    }

    /// Check if audio is playing.
    pub fn is_playing(&self) -> bool {
        self.playing
    }

    /// Get sample rate.
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Get number of channels.
    pub fn channels(&self) -> u16 {
        self.channels
    }

    /// Get the output ring buffer for the audio callback.
    pub fn output_buffer(&self) -> Arc<RingBuffer> {
        Arc::clone(&self.mixer.output_buffer)
    }
}

impl Default for AudioEngine {
    fn default() -> Self {
        Self::new().unwrap_or(Self {
            sample_rate: 48000,
            channels: 2,
            playing: false,
            mixer: Mixer::new(3, 9600),
        })
    }
}
