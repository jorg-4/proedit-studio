//! Waveform computation for audio visualization.
//!
//! Generates min/max pairs for waveform display at various zoom levels.

use serde::{Deserialize, Serialize};

/// A min/max pair representing the amplitude range at a pixel position.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct WaveformSample {
    pub min: f32,
    pub max: f32,
}

/// Pre-computed waveform data for a single audio channel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Waveform {
    /// Samples per waveform pixel (the reduction ratio).
    pub samples_per_pixel: usize,
    /// Min/max pairs for display.
    pub data: Vec<WaveformSample>,
    /// Source sample rate.
    pub sample_rate: u32,
}

impl Waveform {
    /// Compute a waveform from raw audio samples.
    ///
    /// `samples` — mono audio data (f32).
    /// `samples_per_pixel` — how many source samples per output pixel.
    pub fn compute(samples: &[f32], samples_per_pixel: usize, sample_rate: u32) -> Self {
        if samples_per_pixel == 0 || samples.is_empty() {
            return Self {
                samples_per_pixel: samples_per_pixel.max(1),
                data: Vec::new(),
                sample_rate,
            };
        }

        let num_pixels = samples.len().div_ceil(samples_per_pixel);
        let mut data = Vec::with_capacity(num_pixels);

        for chunk in samples.chunks(samples_per_pixel) {
            let mut min = f32::MAX;
            let mut max = f32::MIN;
            for &s in chunk {
                if s < min {
                    min = s;
                }
                if s > max {
                    max = s;
                }
            }
            data.push(WaveformSample { min, max });
        }

        Self {
            samples_per_pixel,
            data,
            sample_rate,
        }
    }

    /// Get the RMS energy for a range of pixels.
    pub fn rms_range(&self, start_pixel: usize, end_pixel: usize) -> f32 {
        let start = start_pixel.min(self.data.len());
        let end = end_pixel.min(self.data.len());
        if start >= end {
            return 0.0;
        }

        let mut sum = 0.0f64;
        for sample in &self.data[start..end] {
            let peak = sample.max.abs().max(sample.min.abs()) as f64;
            sum += peak * peak;
        }
        let count = (end - start) as f64;
        (sum / count).sqrt() as f32
    }

    /// Duration in seconds.
    pub fn duration_seconds(&self) -> f64 {
        if self.sample_rate == 0 {
            return 0.0;
        }
        (self.data.len() * self.samples_per_pixel) as f64 / self.sample_rate as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_waveform_basic() {
        // 100 samples, 10 per pixel → 10 pixels
        let samples: Vec<f32> = (0..100).map(|i| (i as f32 / 100.0) * 2.0 - 1.0).collect();
        let wf = Waveform::compute(&samples, 10, 44100);
        assert_eq!(wf.data.len(), 10);

        // First pixel: samples 0-9 → values -1.0 to -0.82
        assert!(wf.data[0].min < -0.8);
        assert!(wf.data[0].max < 0.0);

        // Last pixel: samples 90-99 → values 0.8 to 0.98
        assert!(wf.data[9].min > 0.7);
        assert!(wf.data[9].max > 0.9);
    }

    #[test]
    fn test_waveform_rms() {
        let samples = vec![0.5f32; 1000];
        let wf = Waveform::compute(&samples, 100, 48000);
        let rms = wf.rms_range(0, wf.data.len());
        assert!((rms - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_waveform_empty() {
        let wf = Waveform::compute(&[], 100, 48000);
        assert!(wf.data.is_empty());
    }

    #[test]
    fn test_waveform_duration() {
        let samples = vec![0.0f32; 48000]; // 1 second at 48kHz
        let wf = Waveform::compute(&samples, 480, 48000);
        assert!((wf.duration_seconds() - 1.0).abs() < 0.01);
    }
}
