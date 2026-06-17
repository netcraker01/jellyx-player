//! FFT IPC binary bridge — sends frequency data to Svelte frontend.
//!
//! `FftChannel` receives `FrequencyData` from the FFT engine and sends it
//! via Tauri v2 binary IPC (`Channel<Vec<u8>>`) to the Svelte visualizer.
//! Binary events avoid JSON serialization overhead at 60fps.

use std::sync::{Arc, Mutex};

use crate::audio::fft::encode_frequency_data_binary;
use crate::audio::fft::FrequencyData;

/// Binary FFT streaming channel.
///
/// Wraps a Tauri IPC Channel that sends raw byte frames to the frontend.
/// The frontend creates the Channel and passes it via the `start_fft_stream`
/// command. The FFT thread calls `send()` to push binary frames.
pub struct FftChannel {
    channel: Arc<Mutex<Option<tauri::ipc::Channel<Vec<u8>>>>>,
}

impl FftChannel {
    /// Create a new FftChannel sharing the same Arc as AppState.fft_channel.
    pub fn new(channel: Arc<Mutex<Option<tauri::ipc::Channel<Vec<u8>>>>>) -> Self {
        Self { channel }
    }

    /// Send frequency data as a binary frame to the frontend.
    ///
    /// Encodes `FrequencyData` into the binary frame format:
    /// [4B sample_rate u32 LE][4B peak f32 LE][N*4B bins f32 LE]
    ///
    /// Silently drops the frame if the channel is not available or send fails.
    pub fn send(&self, data: &FrequencyData) {
        let frame = encode_frequency_data_binary(data);

        let ch = self.channel.lock().ok();
        if let Some(guard) = ch {
            if let Some(ref channel) = *guard {
                let _ = channel.send(frame);
            }
        }
    }

    /// Clear the channel reference (e.g., when playback stops).
    pub fn clear(&self) {
        if let Ok(mut guard) = self.channel.lock() {
            *guard = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::fft::FrequencyData;

    #[test]
    fn fft_channel_send_encodes_binary_frame() {
        let data = FrequencyData {
            bins: vec![0.1, 0.2, 0.3],
            sample_rate: 44100,
            peak: 0.3,
        };

        // Verify the binary encoding function produces correct output
        let frame = encode_frequency_data_binary(&data);
        assert_eq!(frame.len(), 20, "3 bins should produce 20-byte frame");

        let sr = u32::from_le_bytes(frame[0..4].try_into().unwrap());
        assert_eq!(sr, 44100);
    }

    #[test]
    fn fft_channel_clear_sets_none() {
        let channel: Arc<Mutex<Option<tauri::ipc::Channel<Vec<u8>>>>> = Arc::new(Mutex::new(None));
        let fft = FftChannel::new(channel.clone());

        // Clear should work even when channel is already None
        fft.clear();

        let guard = channel.lock().unwrap();
        assert!(guard.is_none());
    }

    #[test]
    fn fft_channel_send_does_nothing_when_no_channel() {
        let channel: Arc<Mutex<Option<tauri::ipc::Channel<Vec<u8>>>>> = Arc::new(Mutex::new(None));
        let fft = FftChannel::new(channel.clone());

        let data = FrequencyData {
            bins: vec![0.5; 128],
            sample_rate: 48000,
            peak: 0.5,
        };

        // Should not panic when sending with no channel
        fft.send(&data);
    }

    #[test]
    fn binary_frame_format_matches_spec() {
        let data = FrequencyData {
            bins: vec![1.0, 2.0, 3.0, 4.0],
            sample_rate: 22050,
            peak: 4.0,
        };

        let frame = encode_frequency_data_binary(&data);

        // Verify frame structure
        assert_eq!(frame.len(), 8 + 4 * 4, "Frame: 8 header + 4 bins * 4 bytes");

        // sample_rate at offset 0
        let sr = u32::from_le_bytes(frame[0..4].try_into().unwrap());
        assert_eq!(sr, 22050);

        // peak at offset 4
        let pk = f32::from_le_bytes(frame[4..8].try_into().unwrap());
        assert!((pk - 4.0).abs() < f32::EPSILON);

        // bins starting at offset 8
        for (i, expected) in [1.0f32, 2.0, 3.0, 4.0].iter().enumerate() {
            let offset = 8 + i * 4;
            let val = f32::from_le_bytes(frame[offset..offset + 4].try_into().unwrap());
            assert!((val - *expected).abs() < f32::EPSILON, "bin[{}] mismatch", i);
        }
    }
}