//! Real-time FFT analysis for audio visualization.
//!
//! Uses rustfft to compute frequency spectrum from PCM audio buffers.
//! The `FftEngine` subscribes to a `PcmBusSubscriber`, collects samples
//! into a circular buffer, computes FFT, and produces `FrequencyData`
//! for the frontend visualizer.

use std::collections::VecDeque;

use rustfft::FftPlanner;
use rustfft::num_complex::Complex;
use rustfft::num_traits::Zero;

use super::pipeline::PcmBusSubscriber;

/// FFT analysis engine that consumes PCM frames from the bus.
///
/// The engine maintains a circular buffer that accumulates PCM samples
/// from the `PcmBusSubscriber`. When enough samples are collected
/// (>= fft_size), it computes the FFT and returns `FrequencyData`.
pub struct FftEngine {
    fft_size: usize,
    planner: FftPlanner<f64>,
    /// Circular buffer of interleaved f32 PCM samples.
    buffer: VecDeque<f32>,
    /// Subscriber to the PCM Bus for receiving audio frames.
    subscriber: PcmBusSubscriber,
    /// Sample rate of the audio stream (for FrequencyData metadata).
    sample_rate: u32,
}

/// Standalone FFT analyzer for direct sample analysis (no bus).
///
/// Used for one-shot analysis when you already have PCM samples.
/// For real-time streaming, use `FftEngine` instead.
#[allow(dead_code)]
pub struct AudioAnalyzer {
    fft_size: usize,
    planner: FftPlanner<f64>,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FrequencyData {
    pub bins: Vec<f32>,
    pub sample_rate: u32,
    pub peak: f32,
}

impl FftEngine {
    /// Create a new FftEngine that reads from a PcmBusSubscriber.
    ///
    /// `fft_size` determines the FFT window size (e.g., 1024 or 2048).
    /// `subscriber` receives PCM frames from the PcmBus.
    /// `sample_rate` is the audio sample rate in Hz.
    pub fn new(fft_size: usize, subscriber: PcmBusSubscriber, sample_rate: u32) -> Self {
        Self {
            fft_size,
            planner: FftPlanner::new(),
            buffer: VecDeque::with_capacity(fft_size * 2),
            subscriber,
            sample_rate,
        }
    }

    /// Drain available PCM frames from the subscriber into the circular buffer.
    ///
    /// Called from a timer or poll loop. The buffer accumulates samples;
    /// when enough are present, call `analyze_if_ready()` to compute FFT.
    pub fn collect_frames(&mut self) {
        while let Some(frame) = self.subscriber.try_recv() {
            self.buffer.extend(frame);
        }

        // Keep buffer size bounded — drop oldest samples if buffer exceeds
        // twice the FFT size (allows some history without unbounded growth).
        let max_capacity = self.fft_size * 2;
        while self.buffer.len() > max_capacity {
            self.buffer.pop_front();
        }
    }

    /// Compute FFT if enough samples are available in the circular buffer.
    ///
    /// Returns `Some(FrequencyData)` if >= fft_size samples are available,
    /// `None` otherwise. On success, the consumed samples are removed from
    /// the buffer (sliding window advances).
    pub fn analyze_if_ready(&mut self) -> Option<FrequencyData> {
        if self.buffer.len() < self.fft_size {
            return None;
        }

        // Take fft_size samples from the front of the buffer
        let samples: Vec<f32> = self.buffer.drain(..self.fft_size).collect();

        let frequency_data = compute_fft(&samples, self.fft_size, &mut self.planner, self.sample_rate);

        Some(frequency_data)
    }

    /// Force an FFT analysis with whatever samples are in the buffer.
    ///
    /// Pads with zeros if fewer than fft_size samples are available.
    /// Useful for getting a partial spectrum on demand.
    #[allow(dead_code)]
    pub fn analyze_partial(&mut self) -> FrequencyData {
        let available = self.buffer.len().min(self.fft_size);
        let samples: Vec<f32> = self.buffer.drain(..available).collect();

        compute_fft(&samples, self.fft_size, &mut self.planner, self.sample_rate)
    }

    /// Get the current buffer length (number of PCM samples pending analysis).
    #[allow(dead_code)]
    pub fn buffer_len(&self) -> usize {
        self.buffer.len()
    }

    /// Get the sample rate.
    #[allow(dead_code)]
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
}

/// Core FFT computation: converts a slice of f32 PCM samples to FrequencyData.
///
/// Pads input with zeros if shorter than fft_size.
fn compute_fft(
    samples: &[f32],
    fft_size: usize,
    planner: &mut FftPlanner<f64>,
    sample_rate: u32,
) -> FrequencyData {
    let mut buffer: Vec<Complex<f64>> = samples
        .iter()
        .map(|&s| Complex::new(s as f64, 0.0))
        .chain(std::iter::repeat(Complex::zero()))
        .take(fft_size)
        .collect();

    let fft = planner.plan_fft_forward(fft_size);
    fft.process(&mut buffer);

    // Magnitude spectrum (only the first half — Nyquist)
    let bins: Vec<f32> = buffer
        .iter()
        .take(fft_size / 2)
        .map(|c| (c.norm() / fft_size as f64) as f32)
        .collect();

    let peak = bins.iter().cloned().fold(0.0_f32, f32::max);

    FrequencyData {
        bins,
        sample_rate,
        peak,
    }
}

/// Encode `FrequencyData` into a binary frame for IPC transfer.
///
/// Frame layout (all little-endian):
/// - Bytes 0-3: `sample_rate` as `u32` LE
/// - Bytes 4-7: `peak` as `f32` LE
/// - Bytes 8+: `bins` as `N * f32` LE (N = bins.len())
///
/// Total frame size = 8 + bins.len() * 4 bytes.
pub fn encode_frequency_data_binary(data: &FrequencyData) -> Vec<u8> {
    let bin_count = data.bins.len();
    let mut buf = Vec::with_capacity(8 + bin_count * 4);

    // sample_rate: u32 LE
    buf.extend_from_slice(&data.sample_rate.to_le_bytes());
    // peak: f32 LE
    buf.extend_from_slice(&data.peak.to_le_bytes());
    // bins: N * f32 LE
    for &bin in &data.bins {
        buf.extend_from_slice(&bin.to_le_bytes());
    }

    buf
}

impl AudioAnalyzer {
    #[allow(dead_code)]
    pub fn new(fft_size: usize) -> Self {
        Self {
            fft_size,
            planner: FftPlanner::new(),
        }
    }

    /// Convert PCM samples to frequency spectrum bins (one-shot analysis).
    #[allow(dead_code)]
    pub fn analyze(&mut self, samples: &[f32], sample_rate: u32) -> FrequencyData {
        compute_fft(samples, self.fft_size, &mut self.planner, sample_rate)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::pipeline::PcmBus;

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

    #[test]
    fn fft_engine_collects_frames_from_bus() {
        // Create a PcmBus and a subscriber
        let (producer, subscriber) = PcmBus::new(44100, 2);
        let mut engine = FftEngine::new(256, subscriber, 44100);

        // Send some frames through the bus
        producer.send(vec![0.5; 128]).unwrap();
        producer.send(vec![0.3; 128]).unwrap();

        // Collect frames into the engine's circular buffer
        engine.collect_frames();

        // Buffer should have 256 samples (128 + 128)
        assert_eq!(engine.buffer_len(), 256);
    }

    #[test]
    fn fft_engine_analyze_if_ready_when_enough_samples() {
        let (producer, subscriber) = PcmBus::new(44100, 2);
        let mut engine = FftEngine::new(256, subscriber, 44100);

        // Send enough samples for one FFT window
        for _ in 0..2 {
            producer.send(vec![0.1; 128]).unwrap();
        }

        engine.collect_frames();
        let result = engine.analyze_if_ready();

        assert!(result.is_some(), "Should return FrequencyData when enough samples");
        let data = result.unwrap();
        assert!(!data.bins.is_empty(), "Bins should not be empty");
        assert_eq!(data.sample_rate, 44100);
    }

    #[test]
    fn fft_engine_analyze_if_ready_returns_none_when_insufficient() {
        let (_, subscriber) = PcmBus::new(44100, 2);
        let mut engine = FftEngine::new(1024, subscriber, 44100);

        // No frames sent — buffer is empty
        engine.collect_frames();
        let result = engine.analyze_if_ready();
        assert!(result.is_none(), "Should return None when insufficient samples");
    }

    #[test]
    fn fft_engine_analyze_partial_pads_with_zeros() {
        let (_, subscriber) = PcmBus::new(44100, 2);
        let mut engine = FftEngine::new(256, subscriber, 44100);

        // Buffer is empty — analyze_partial should still work (all zeros)
        let data = engine.analyze_partial();
        assert!(!data.bins.is_empty(), "Bins should not be empty even with zero input");
        // All-zero input should produce all-zero bins (or near-zero due to float math)
        let max_bin = data.bins.iter().cloned().fold(0.0_f32, f32::max);
        assert!(max_bin < 0.001, "Zero input should produce near-zero bins, got {}", max_bin);
    }

    #[test]
    fn fft_engine_buffer_drops_oldest_when_over_capacity() {
        let (producer, subscriber) = PcmBus::new(44100, 2);
        let mut engine = FftEngine::new(256, subscriber, 44100);

        // Send more than 2x fft_size samples
        for i in 0..6 {
            let val = i as f32 * 0.1;
            producer.send(vec![val; 128]).unwrap();
        }

        engine.collect_frames();

        // Buffer should be capped at 2 * fft_size = 512
        assert!(engine.buffer_len() <= 512, "Buffer should be capped at 2 * fft_size, got {}", engine.buffer_len());
    }

    #[test]
    fn encode_binary_frame_layout_correct() {
        let data = FrequencyData {
            bins: vec![0.1, 0.2, 0.3],
            sample_rate: 44100,
            peak: 0.3,
        };

        let frame = encode_frequency_data_binary(&data);

        // Total size: 8 header + 3 bins * 4 = 20 bytes
        assert_eq!(frame.len(), 20, "Frame size should be 8 + bins.len() * 4");

        // sample_rate at offset 0 (u32 LE)
        let sr = u32::from_le_bytes(frame[0..4].try_into().unwrap());
        assert_eq!(sr, 44100, "sample_rate should be at offset 0");

        // peak at offset 4 (f32 LE)
        let pk = f32::from_le_bytes(frame[4..8].try_into().unwrap());
        assert!((pk - 0.3).abs() < f32::EPSILON, "peak should be at offset 4");

        // bins start at offset 8
        let bin0 = f32::from_le_bytes(frame[8..12].try_into().unwrap());
        let bin1 = f32::from_le_bytes(frame[12..16].try_into().unwrap());
        let bin2 = f32::from_le_bytes(frame[16..20].try_into().unwrap());
        assert!((bin0 - 0.1).abs() < 1e-6, "bin[0] should be at offset 8");
        assert!((bin1 - 0.2).abs() < 1e-6, "bin[1] should be at offset 12");
        assert!((bin2 - 0.3).abs() < 1e-6, "bin[2] should be at offset 16");
    }

    #[test]
    fn encode_binary_empty_bins() {
        let data = FrequencyData {
            bins: vec![],
            sample_rate: 48000,
            peak: 0.0,
        };

        let frame = encode_frequency_data_binary(&data);
        assert_eq!(frame.len(), 8, "Empty bins should produce 8-byte header only");

        let sr = u32::from_le_bytes(frame[0..4].try_into().unwrap());
        assert_eq!(sr, 48000);
        let pk = f32::from_le_bytes(frame[4..8].try_into().unwrap());
        assert!((pk - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn encode_binary_large_fft() {
        let data = FrequencyData {
            bins: vec![0.5; 512],
            sample_rate: 44100,
            peak: 0.5,
        };

        let frame = encode_frequency_data_binary(&data);
        assert_eq!(frame.len(), 8 + 512 * 4, "512 bins should produce 2056-byte frame");
    }

    #[test]
    fn encode_binary_roundtrip_values() {
        let data = FrequencyData {
            bins: vec![0.001, 0.999, -0.5, 1.0],
            sample_rate: 96000,
            peak: 1.0,
        };

        let frame = encode_frequency_data_binary(&data);

        // Decode and verify
        let sr = u32::from_le_bytes(frame[0..4].try_into().unwrap());
        assert_eq!(sr, 96000);

        let pk = f32::from_le_bytes(frame[4..8].try_into().unwrap());
        assert!((pk - 1.0).abs() < f32::EPSILON);

        for (i, expected) in data.bins.iter().enumerate() {
            let offset = 8 + i * 4;
            let val = f32::from_le_bytes(frame[offset..offset + 4].try_into().unwrap());
            assert!((val - expected).abs() < 1e-6, "bin[{}] roundtrip failed", i);
        }
    }

    #[test]
    fn audio_analyzer_still_works_after_refactor() {
        let mut analyzer = AudioAnalyzer::new(256);
        let samples: Vec<f32> = vec![0.5; 256];
        let result = analyzer.analyze(&samples, 44100);
        assert!(!result.bins.is_empty());
        assert_eq!(result.sample_rate, 44100);
    }
}
