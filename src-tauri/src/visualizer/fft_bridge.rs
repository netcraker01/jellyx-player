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

    #[test]
    fn frequency_data_event_constant() {
        assert_eq!(EVENT_FREQUENCY_DATA, "frequency-data");
    }
}