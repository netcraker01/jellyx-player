//! Real-time FFT analysis for audio visualization.
//!
//! Uses rustfft to compute frequency spectrum from PCM audio buffers.
//! The frequency data is sent to the frontend for rendering.

use rustfft::FftPlanner;
use rustfft::num_complex::Complex;
use rustfft::num_traits::Zero;

pub struct AudioAnalyzer {
    fft_size: usize,
    planner: FftPlanner<f64>,
    // TODO: circular buffer for real-time data
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FrequencyData {
    pub bins: Vec<f32>,
    pub sample_rate: u32,
    pub peak: f32,
}

impl AudioAnalyzer {
    pub fn new(fft_size: usize) -> Self {
        Self {
            fft_size,
            planner: FftPlanner::new(),
        }
    }

    /// Convert PCM samples to frequency spectrum bins.
    pub fn analyze(&mut self, samples: &[f32], sample_rate: u32) -> FrequencyData {
        let mut buffer: Vec<Complex<f64>> = samples
            .iter()
            .map(|&s| Complex::new(s as f64, 0.0))
            .chain(std::iter::repeat(Complex::zero()))
            .take(self.fft_size)
            .collect();

        let fft = self.planner.plan_fft_forward(self.fft_size);
        fft.process(&mut buffer);

        // Magnitude spectrum
        let bins: Vec<f32> = buffer
            .iter()
            .take(self.fft_size / 2)
            .map(|c| (c.norm() / self.fft_size as f64) as f32)
            .collect();

        let peak = bins.iter().cloned().fold(0.0_f32, f32::max);

        FrequencyData {
            bins,
            sample_rate,
            peak,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frequency_data_serializes_camel_case() {
        let data = FrequencyData {
            bins: vec![0.1, 0.5, 0.3],
            sample_rate: 44100,
            peak: 0.5,
        };
        let json = serde_json::to_string(&data).unwrap();
        assert!(json.contains("\"sampleRate\""), "sample_rate should serialize as camelCase");
        assert!(json.contains("\"bins\""), "bins should be present");
        assert!(json.contains("\"peak\""), "peak should be present");
    }

    #[test]
    fn frequency_data_all_fields_present() {
        let data = FrequencyData {
            bins: vec![1.0, 2.0],
            sample_rate: 48000,
            peak: 2.0,
        };
        let json = serde_json::to_string(&data).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(parsed.get("bins").is_some(), "bins field must be present");
        assert!(parsed.get("sampleRate").is_some(), "sampleRate field must be present");
        assert!(parsed.get("peak").is_some(), "peak field must be present");
    }
}
