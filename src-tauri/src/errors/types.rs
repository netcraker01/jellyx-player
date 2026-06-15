//! Canonical error types for Helix.
//!
//! `AppError` is the structured error type returned by all Tauri commands.
//! Frontend maps `code` to a localized string and can use `details` for params.
//!
//! `SourceError` is the error type for source resolution failures.

use crate::audio::AudioError;
use serde::Serialize;

/// Error type for source resolution failures.
#[derive(Debug)]
pub enum SourceError {
    NetworkError(String),
    ResolveError(String),
    UnsupportedSource,
}

/// Structured error with a translatable code + optional details.
/// Frontend maps `code` to a localized string and can use `details` for params.
#[derive(Debug, Serialize)]
pub struct AppError {
    pub code: String,
    pub details: Option<String>,
}

impl From<SourceError> for AppError {
    fn from(e: SourceError) -> Self {
        match e {
            SourceError::NetworkError(msg) => AppError {
                code: "NETWORK_TIMEOUT".into(),
                details: Some(msg),
            },
            SourceError::ResolveError(msg) => AppError {
                code: "STREAM_NOT_FOUND".into(),
                details: Some(msg),
            },
            SourceError::UnsupportedSource => AppError {
                code: "UNKNOWN_ERROR".into(),
                details: None,
            },
        }
    }
}

impl From<AudioError> for AppError {
    fn from(e: AudioError) -> Self {
        match e {
            AudioError::DecodeError(msg) => AppError {
                code: "PLAYBACK_ERROR".into(),
                details: Some(format!("decode: {}", msg)),
            },
            AudioError::DeviceError(msg) => AppError {
                code: "DEVICE_NOT_FOUND".into(),
                details: Some(msg),
            },
            AudioError::UnsupportedFormat => AppError {
                code: "PLAYBACK_ERROR".into(),
                details: Some("unsupported format".into()),
            },
            AudioError::PlatformNotSupported => AppError {
                code: "PLAYBACK_ERROR".into(),
                details: Some("platform not supported".into()),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_error_network_maps_to_network_timeout() {
        let err = AppError::from(SourceError::NetworkError("timeout".into()));
        assert_eq!(err.code, "NETWORK_TIMEOUT");
        assert_eq!(err.details, Some("timeout".to_string()));
    }

    #[test]
    fn source_error_resolve_maps_to_stream_not_found() {
        let err = AppError::from(SourceError::ResolveError("not found".into()));
        assert_eq!(err.code, "STREAM_NOT_FOUND");
        assert_eq!(err.details, Some("not found".to_string()));
    }

    #[test]
    fn source_error_unsupported_maps_to_unknown_error() {
        let err = AppError::from(SourceError::UnsupportedSource);
        assert_eq!(err.code, "UNKNOWN_ERROR");
        assert!(err.details.is_none());
    }

    #[test]
    fn audio_error_decode_maps_to_playback_error() {
        let err = AppError::from(AudioError::DecodeError("corrupt frame".into()));
        assert_eq!(err.code, "PLAYBACK_ERROR");
        assert!(err.details.as_ref().unwrap().contains("decode: corrupt frame"));
    }

    #[test]
    fn audio_error_device_maps_to_device_not_found() {
        let err = AppError::from(AudioError::DeviceError("no output".into()));
        assert_eq!(err.code, "DEVICE_NOT_FOUND");
        assert_eq!(err.details, Some("no output".to_string()));
    }
}