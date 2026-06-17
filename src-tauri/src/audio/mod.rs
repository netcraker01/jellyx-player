//! Audio pipeline module.
//!
//! Platform-agnostic audio backend trait.
//! Desktop uses `cpal + symphonia`. Mobile will use platform backends.

pub mod decoder;
pub mod fft;
pub mod output;
pub mod pipeline;

// Re-export PlaybackState from the playback module — it is the Source of Truth.
pub use crate::playback::state::PlaybackState;

use std::path::PathBuf;

/// Trait that abstracts the audio backend for cross-platform support.
///
/// Desktop → cpal backend
/// Android → Oboe backend
/// iOS → AVAudioEngine backend
pub trait AudioBackend {
    /// Play audio from a URL (streaming source).
    /// Reserved for future use (YouTube, SoundCloud streams).
    fn play(&mut self, url: &str) -> Result<(), AudioError>;

    /// Play a local audio file.
    /// The primary playback method for v0.1 (local-file pipeline).
    fn play_local(&mut self, path: &PathBuf) -> Result<(), AudioError>;

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
    fn playback_state_playing_serializes_to_pascal_case() {
        let json = serde_json::to_string(&PlaybackState::Playing).unwrap();
        assert_eq!(json, "\"Playing\"");
    }

    #[test]
    fn playback_state_stopped_serializes_to_pascal_case() {
        let json = serde_json::to_string(&PlaybackState::Stopped).unwrap();
        assert_eq!(json, "\"Stopped\"");
    }

    #[test]
    fn playback_state_paused_serializes_to_pascal_case() {
        let json = serde_json::to_string(&PlaybackState::Paused).unwrap();
        assert_eq!(json, "\"Paused\"");
    }

    #[test]
    fn playback_state_buffering_serializes_to_pascal_case() {
        let json = serde_json::to_string(&PlaybackState::Buffering).unwrap();
        assert_eq!(json, "\"Buffering\"");
    }

    #[test]
    fn audio_error_decode_error_serializes_snake_case() {
        let err = AudioError::DecodeError("bad frame".to_string());
        let json = serde_json::to_string(&err).unwrap();
        assert!(json.contains("\"decode_error\""), "AudioError variants should serialize as snake_case");
    }

    #[test]
    fn audio_error_device_error_serializes_snake_case() {
        let err = AudioError::DeviceError("no device".to_string());
        let json = serde_json::to_string(&err).unwrap();
        assert!(json.contains("\"device_error\""), "AudioError variants should serialize as snake_case");
    }

    #[test]
    fn audio_error_unsupported_format_serializes_snake_case() {
        let json = serde_json::to_string(&AudioError::UnsupportedFormat).unwrap();
        assert_eq!(json, "\"unsupported_format\"");
    }

    #[test]
    fn audio_error_platform_not_supported_serializes_snake_case() {
        let json = serde_json::to_string(&AudioError::PlatformNotSupported).unwrap();
        assert_eq!(json, "\"platform_not_supported\"");
    }

    #[test]
    fn audio_error_no_audio_device_serializes_snake_case() {
        let err = AudioError::NoAudioDevice("No output device found".to_string());
        let json = serde_json::to_string(&err).unwrap();
        assert!(json.contains("\"no_audio_device\""), "AudioError NoAudioDevice should serialize as snake_case");
    }

    #[test]
    fn audio_error_decode_failed_serializes_snake_case() {
        let err = AudioError::DecodeFailed("codec error".to_string());
        let json = serde_json::to_string(&err).unwrap();
        assert!(json.contains("\"decode_failed\""), "AudioError DecodeFailed should serialize as snake_case");
    }
}