//! FFT IPC binary bridge — sends frequency data to Svelte frontend.
//!
//! `FftBridge` receives `FrequencyData` from the FFT engine and emits it
//! via Tauri v2 binary IPC (`AppHandle.emit()`) to the Svelte visualizer.
//! Binary events avoid JSON serialization overhead at 60fps.

use crate::audio::fft::FrequencyData;
use crate::errors::types::IPCError;
use tauri::{AppHandle, Emitter};

/// Event name for FFT frequency data (binary IPC).
pub const EVENT_FREQUENCY_DATA: &str = "frequency-data";

/// Bridge between FFT engine and Tauri frontend.
///
/// Receives frequency analysis data and emits it as a Tauri event.
/// The frontend subscribes to `frequency-data` events and receives
/// the `FrequencyData` struct serialized as JSON (for v0.1).
pub struct FftBridge {
    app: AppHandle,
}

impl FftBridge {
    /// Create a new FftBridge with the given Tauri AppHandle.
    pub fn new(app: AppHandle) -> Self {
        Self { app }
    }

    /// Emit frequency data to the frontend.
    ///
    /// For v0.1, this uses Tauri's standard `emit()` with JSON serialization.
    /// Future optimization: use binary IPC (Uint8Array) to avoid JSON overhead.
    pub fn emit_frequency_data(&self, data: &FrequencyData) -> Result<(), IPCError> {
        self.app
            .emit(EVENT_FREQUENCY_DATA, data)
            .map_err(|e| IPCError::CommandFailed(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::fft::FrequencyData;

    #[test]
    fn frequency_data_event_constant() {
        assert_eq!(EVENT_FREQUENCY_DATA, "frequency-data");
    }

    #[test]
    fn fft_bridge_new_creates_instance() {
        // We can't test emit_frequency_data without a Tauri runtime,
        // but we can verify that the event constant is correct and that
        // FrequencyData serializes properly for the bridge.
        let data = FrequencyData {
            bins: vec![0.1, 0.2, 0.3, 0.4, 0.5],
            sample_rate: 44100,
            peak: 0.5,
        };

        // Verify FrequencyData is Serialize (required for emit)
        let json = serde_json::to_string(&data).unwrap();
        assert!(json.contains("\"bins\""));
        assert!(json.contains("\"sampleRate\""));
        assert!(json.contains("\"peak\""));
    }

    #[test]
    fn frequency_data_serializes_for_ipc() {
        // Simulate what the frontend would receive
        let data = FrequencyData {
            bins: vec![0.0; 128],
            sample_rate: 48000,
            peak: 0.0,
        };

        let json = serde_json::to_string(&data).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        // Verify structure matches TypeScript expectations
        assert!(parsed.get("bins").unwrap().is_array(), "bins must be array");
        assert_eq!(parsed.get("bins").unwrap().as_array().unwrap().len(), 128);
        assert_eq!(parsed.get("sampleRate").unwrap().as_u64(), Some(48000));
        assert_eq!(parsed.get("peak").unwrap().as_f64(), Some(0.0));
    }

    #[test]
    fn frequency_data_with_peak_serializes() {
        let data = FrequencyData {
            bins: vec![0.1, 0.5, 0.3],
            sample_rate: 44100,
            peak: 0.5,
        };

        let json = serde_json::to_string(&data).unwrap();

        // Peak should be the max of bins
        assert!(json.contains("\"peak\":0.5") || json.contains("\"peak\": 0.5"),
            "Peak value should serialize correctly");
    }

    #[test]
    fn fft_bridge_event_name_is_valid_tauri_event() {
        // Tauri event names should be lowercase-hyphen (not camelCase, not snake_case)
        assert_eq!(EVENT_FREQUENCY_DATA, "frequency-data");
        assert!(!EVENT_FREQUENCY_DATA.contains('_'),
            "Event name should not use snake_case");
        assert!(!EVENT_FREQUENCY_DATA.contains(char::is_uppercase),
            "Event name should be all lowercase");
    }
}