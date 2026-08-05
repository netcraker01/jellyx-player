//! Audio backend trait and error type — shared by all frontends.
//!
//! Desktop implements this with CPAL + Symphonia.
//! The future TUI will implement it with CPAL + Symphonia (headless or via
//! a virtual sink). Mobile would use platform-specific backends.

use crate::playback_models::PlaybackState;
use std::path::Path;

/// Trait that abstracts the audio backend for cross-platform support.
#[allow(dead_code)]
pub trait AudioBackend {
    fn play(&mut self, url: &str) -> Result<(), AudioError>;
    fn play_local(&mut self, path: &Path) -> Result<(), AudioError>;
    fn pause(&mut self) -> Result<(), AudioError>;
    fn resume(&mut self) -> Result<(), AudioError>;
    fn stop(&mut self) -> Result<(), AudioError>;
    fn seek(&mut self, position: f64) -> Result<(), AudioError>;
    fn volume(&mut self, level: f32) -> Result<(), AudioError>;
    fn position(&self) -> f64;
    fn state(&self) -> PlaybackState;
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AudioError {
    DecodeError(String),
    DeviceError(String),
    UnsupportedFormat,
    PlatformNotSupported,
    NoAudioDevice(String),
    DecodeFailed(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audio_error_snake_case() {
        assert_eq!(
            serde_json::to_string(&AudioError::UnsupportedFormat).unwrap(),
            "\"unsupported_format\""
        );
        assert!(
            serde_json::to_string(&AudioError::DecodeError("x".into()))
                .unwrap()
                .contains("\"decode_error\"")
        );
    }
}
