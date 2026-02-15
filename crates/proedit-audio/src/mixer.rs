//! Audio mixer — mixes multiple tracks into a stereo output buffer.

use crate::ring_buffer::RingBuffer;
use std::sync::Arc;

/// Per-track mixer channel configuration.
#[derive(Debug, Clone)]
pub struct MixerChannel {
    /// Volume (0.0 to 1.0).
    pub volume: f32,
    /// Pan (-1.0 = full left, 0.0 = center, 1.0 = full right).
    pub pan: f32,
    /// Whether this channel is muted.
    pub muted: bool,
    /// Whether this channel is soloed.
    pub solo: bool,
}

impl Default for MixerChannel {
    fn default() -> Self {
        Self {
            volume: 1.0,
            pan: 0.0,
            muted: false,
            solo: false,
        }
    }
}

impl MixerChannel {
    /// Compute left/right gain from volume and pan (constant-power panning).
    pub fn stereo_gain(&self) -> (f32, f32) {
        if self.muted {
            return (0.0, 0.0);
        }
        // Constant-power panning: use sin/cos curve
        let angle = (self.pan + 1.0) * 0.25 * std::f32::consts::PI;
        let left = self.volume * angle.cos();
        let right = self.volume * angle.sin();
        (left, right)
    }
}

/// Audio mixer that combines multiple channels into stereo output.
pub struct Mixer {
    /// Per-track channels.
    channels: Vec<MixerChannel>,
    /// Master volume.
    pub master_volume: f32,
    /// Master limiter enabled.
    pub limiter_enabled: bool,
    /// Limiter threshold in linear amplitude.
    pub limiter_threshold: f32,
    /// Output ring buffer for the audio callback.
    pub output_buffer: Arc<RingBuffer>,
    /// Scratch buffer for mixing.
    scratch: Vec<f32>,
}

impl Mixer {
    /// Create a new mixer with the given number of channels.
    pub fn new(num_channels: usize, buffer_size: usize) -> Self {
        Self {
            channels: (0..num_channels).map(|_| MixerChannel::default()).collect(),
            master_volume: 1.0,
            limiter_enabled: false,
            limiter_threshold: 0.95,
            output_buffer: Arc::new(RingBuffer::new(buffer_size)),
            scratch: vec![0.0; 4096],
        }
    }

    /// Get channel configuration.
    pub fn channel(&self, index: usize) -> Option<&MixerChannel> {
        self.channels.get(index)
    }

    /// Get mutable channel configuration.
    pub fn channel_mut(&mut self, index: usize) -> Option<&mut MixerChannel> {
        self.channels.get_mut(index)
    }

    /// Number of mixer channels.
    pub fn channel_count(&self) -> usize {
        self.channels.len()
    }

    /// Add a channel.
    pub fn add_channel(&mut self) -> usize {
        let idx = self.channels.len();
        self.channels.push(MixerChannel::default());
        idx
    }

    /// Check if any channel is soloed.
    fn any_solo(&self) -> bool {
        self.channels.iter().any(|c| c.solo)
    }

    /// Mix interleaved stereo source data from multiple channels into
    /// the scratch buffer and write to the output ring buffer.
    ///
    /// `sources` is a slice of interleaved stereo f32 buffers (one per channel).
    /// Each buffer must have exactly `frame_count * 2` samples.
    pub fn mix(&mut self, sources: &[&[f32]], frame_count: usize) {
        let output_len = frame_count * 2;
        if self.scratch.len() < output_len {
            self.scratch.resize(output_len, 0.0);
        }

        // Zero the scratch buffer
        for s in self.scratch[..output_len].iter_mut() {
            *s = 0.0;
        }

        let has_solo = self.any_solo();

        for (ch_idx, source) in sources.iter().enumerate() {
            let channel = match self.channels.get(ch_idx) {
                Some(c) => c,
                None => continue,
            };

            // If any track is soloed, only play soloed tracks
            if has_solo && !channel.solo {
                continue;
            }

            let (gain_l, gain_r) = channel.stereo_gain();

            for frame in 0..frame_count {
                let src_idx = frame * 2;
                if src_idx + 1 >= source.len() {
                    break;
                }
                let sl = source[src_idx];
                let sr = source[src_idx + 1];

                self.scratch[src_idx] += sl * gain_l;
                self.scratch[src_idx + 1] += sr * gain_r;
            }
        }

        // Apply master volume
        for s in self.scratch[..output_len].iter_mut() {
            *s *= self.master_volume;
        }

        // Apply limiter (simple hard clamp)
        if self.limiter_enabled {
            let threshold = self.limiter_threshold;
            for s in self.scratch[..output_len].iter_mut() {
                *s = s.clamp(-threshold, threshold);
            }
        }

        // Write to output ring buffer
        self.output_buffer.write(&self.scratch[..output_len]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stereo_gain_center() {
        let ch = MixerChannel::default();
        let (l, r) = ch.stereo_gain();
        // At center pan, both channels should be roughly equal
        assert!((l - r).abs() < 0.01);
        assert!(l > 0.5);
    }

    #[test]
    fn test_stereo_gain_muted() {
        let ch = MixerChannel {
            muted: true,
            ..Default::default()
        };
        let (l, r) = ch.stereo_gain();
        assert_eq!(l, 0.0);
        assert_eq!(r, 0.0);
    }

    #[test]
    fn test_stereo_gain_pan_left() {
        let ch = MixerChannel {
            pan: -1.0,
            ..Default::default()
        };
        let (l, r) = ch.stereo_gain();
        assert!(l > r);
        assert!(r.abs() < 0.01);
    }

    #[test]
    fn test_mixer_basic() {
        let mut mixer = Mixer::new(2, 4096);

        // Two channels, 4 frames each (interleaved stereo)
        let ch0: Vec<f32> = vec![0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5];
        let ch1: Vec<f32> = vec![0.3, 0.3, 0.3, 0.3, 0.3, 0.3, 0.3, 0.3];

        mixer.mix(&[&ch0, &ch1], 4);

        // Should have 8 samples in the output buffer
        assert_eq!(mixer.output_buffer.available_read(), 8);

        let mut out = vec![0.0f32; 8];
        mixer.output_buffer.read(&mut out);
        // All values should be > 0 (mixed)
        for s in &out {
            assert!(*s > 0.0);
        }
    }

    #[test]
    fn test_mixer_solo() {
        let mut mixer = Mixer::new(2, 4096);
        mixer.channel_mut(1).unwrap().solo = true;

        let ch0 = vec![1.0f32; 8];
        let ch1 = vec![0.5f32; 8];

        mixer.mix(&[&ch0, &ch1], 4);

        let mut out = vec![0.0f32; 8];
        mixer.output_buffer.read(&mut out);
        // Only ch1 should be heard (ch0 is not soloed)
        for s in &out {
            assert!(*s < 0.6); // ch1 alone, with pan gain
        }
    }

    #[test]
    fn test_mixer_limiter() {
        let mut mixer = Mixer::new(1, 4096);
        mixer.limiter_enabled = true;
        mixer.limiter_threshold = 0.8;

        let loud = vec![2.0f32; 8];
        mixer.mix(&[&loud], 4);

        let mut out = vec![0.0f32; 8];
        mixer.output_buffer.read(&mut out);
        for s in &out {
            assert!(s.abs() <= 0.81); // within threshold + rounding
        }
    }
}
