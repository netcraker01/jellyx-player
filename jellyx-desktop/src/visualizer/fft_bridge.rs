//! FFT IPC bridge — sends frequency data to the Svelte frontend via Tauri events.
//!
//! The FFT engine produces `FrequencyData` which is emitted as a Tauri event
//! (`"fft-frame"`) to the frontend. Events are fire-and-forget with no ordering
//! guarantees — if a frame is lost, the next one arrives normally. This avoids
//! the strict-index ordering of `Channel` IPC which could permanently stall
//! after a single delivery failure.
//!
//! The frontend listens via `listen("fft-frame", ...)` and converts the
//! JSON-serialized `FrequencyData` into a `Float32Array` for the visualizer.

use crate::audio::fft::FrequencyData;
use tauri::Emitter;

/// Event name emitted by the Rust FFT engine and listened by the JS frontend.
pub const FFT_FRAME_EVENT: &str = "fft-frame";

/// Emit a single FFT frame to all frontend listeners.
///
/// Uses `Webview::emit()` which serializes `FrequencyData` as JSON:
/// `{ "bins": [...], "sampleRate": 44100, "peak": 0.5 }`
///
/// The frontend receives this as a plain JS object, converts `bins` to
/// `Float32Array`, and feeds it to the visualizer.
///
/// Emit failures are non-fatal (frontend may not be listening).
pub fn emit_fft_frame<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    data: &FrequencyData,
) -> Result<(), tauri::Error> {
    app.emit(FFT_FRAME_EVENT, data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fft_frame_event_name_is_constant() {
        assert_eq!(FFT_FRAME_EVENT, "fft-frame");
    }
}
