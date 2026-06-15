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