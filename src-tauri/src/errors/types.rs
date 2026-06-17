//! Canonical error types for Helix.
//!
//! `AppError` is the structured error type returned by all Tauri commands.
//! Frontend maps `code` to a localized string and can use `details` for params.
//!
//! Domain error enums map to `AppError` via `From` impls:
//! - `SourceError` → network/stream errors
//! - `AudioError` → playback/device errors
//! - `PlaybackError` → playback state errors
//! - `LibraryError` → library CRUD errors
//! - `PersistenceError` → database/storage errors
//! - `ValidationError` → input validation errors
//! - `IPCError` → command/serialization errors

use crate::audio::AudioError;
use serde::Serialize;

/// Error type for source resolution failures.
#[derive(Debug)]
pub enum SourceError {
    NetworkError(String),
    ResolveError(String),
    UnsupportedSource,
}

/// Playback state errors.
#[derive(Debug)]
pub enum PlaybackError {
    AlreadyStopped,
    QueueEmpty,
    NoCurrentTrack,
    NoAudioDevice(String),
    DecodeFailed(String),
}

/// Library operation errors.
#[derive(Debug)]
pub enum LibraryError {
    NotFound(String),
    AlreadyExists(String),
}

/// Persistence/storage errors.
#[derive(Debug)]
pub enum PersistenceError {
    DatabaseError(String),
    WriteError(String),
}

/// Input validation errors.
#[derive(Debug)]
pub enum ValidationError {
    InvalidInput(String),
    EmptyQuery,
}

/// IPC command/serialization errors.
#[derive(Debug)]
pub enum IPCError {
    CommandFailed(String),
    SerializationError(String),
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
            AudioError::NoAudioDevice(msg) => AppError {
                code: "DEVICE_NOT_FOUND".into(),
                details: Some(msg),
            },
            AudioError::DecodeFailed(msg) => AppError {
                code: "PLAYBACK_ERROR".into(),
                details: Some(format!("decode: {}", msg)),
            },
        }
    }
}

impl From<PlaybackError> for AppError {
    fn from(e: PlaybackError) -> Self {
        match e {
            PlaybackError::AlreadyStopped => AppError {
                code: "PLAYBACK_ERROR".into(),
                details: Some("already stopped".into()),
            },
            PlaybackError::QueueEmpty => AppError {
                code: "PLAYBACK_ERROR".into(),
                details: Some("queue is empty".into()),
            },
            PlaybackError::NoCurrentTrack => AppError {
                code: "PLAYBACK_ERROR".into(),
                details: Some("no current track".into()),
            },
            PlaybackError::NoAudioDevice(msg) => AppError {
                code: "DEVICE_NOT_FOUND".into(),
                details: Some(msg),
            },
            PlaybackError::DecodeFailed(msg) => AppError {
                code: "PLAYBACK_ERROR".into(),
                details: Some(format!("decode: {}", msg)),
            },
        }
    }
}

impl From<LibraryError> for AppError {
    fn from(e: LibraryError) -> Self {
        match e {
            LibraryError::NotFound(msg) => AppError {
                code: "NOT_FOUND".into(),
                details: Some(msg),
            },
            LibraryError::AlreadyExists(msg) => AppError {
                code: "ALREADY_EXISTS".into(),
                details: Some(msg),
            },
        }
    }
}

impl From<PersistenceError> for AppError {
    fn from(e: PersistenceError) -> Self {
        match e {
            PersistenceError::DatabaseError(msg) => AppError {
                code: "PERSISTENCE_ERROR".into(),
                details: Some(msg),
            },
            PersistenceError::WriteError(msg) => AppError {
                code: "PERSISTENCE_ERROR".into(),
                details: Some(msg),
            },
        }
    }
}

impl From<ValidationError> for AppError {
    fn from(e: ValidationError) -> Self {
        match e {
            ValidationError::InvalidInput(msg) => AppError {
                code: "VALIDATION_ERROR".into(),
                details: Some(msg),
            },
            ValidationError::EmptyQuery => AppError {
                code: "VALIDATION_ERROR".into(),
                details: Some("query must not be empty".into()),
            },
        }
    }
}

impl From<IPCError> for AppError {
    fn from(e: IPCError) -> Self {
        match e {
            IPCError::CommandFailed(msg) => AppError {
                code: "IPC_ERROR".into(),
                details: Some(msg),
            },
            IPCError::SerializationError(msg) => AppError {
                code: "IPC_ERROR".into(),
                details: Some(msg),
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

    #[test]
    fn playback_error_already_stopped() {
        let err = AppError::from(PlaybackError::AlreadyStopped);
        assert_eq!(err.code, "PLAYBACK_ERROR");
        assert_eq!(err.details, Some("already stopped".to_string()));
    }

    #[test]
    fn playback_error_queue_empty() {
        let err = AppError::from(PlaybackError::QueueEmpty);
        assert_eq!(err.code, "PLAYBACK_ERROR");
        assert_eq!(err.details, Some("queue is empty".to_string()));
    }

    #[test]
    fn playback_error_no_current_track() {
        let err = AppError::from(PlaybackError::NoCurrentTrack);
        assert_eq!(err.code, "PLAYBACK_ERROR");
        assert_eq!(err.details, Some("no current track".to_string()));
    }

    #[test]
    fn library_error_not_found() {
        let err = AppError::from(LibraryError::NotFound("track 42".into()));
        assert_eq!(err.code, "NOT_FOUND");
        assert_eq!(err.details, Some("track 42".to_string()));
    }

    #[test]
    fn library_error_already_exists() {
        let err = AppError::from(LibraryError::AlreadyExists("playlist favs".into()));
        assert_eq!(err.code, "ALREADY_EXISTS");
        assert_eq!(err.details, Some("playlist favs".to_string()));
    }

    #[test]
    fn persistence_error_database() {
        let err = AppError::from(PersistenceError::DatabaseError("conn refused".into()));
        assert_eq!(err.code, "PERSISTENCE_ERROR");
        assert_eq!(err.details, Some("conn refused".to_string()));
    }

    #[test]
    fn persistence_error_write() {
        let err = AppError::from(PersistenceError::WriteError("disk full".into()));
        assert_eq!(err.code, "PERSISTENCE_ERROR");
        assert_eq!(err.details, Some("disk full".to_string()));
    }

    #[test]
    fn validation_error_invalid_input() {
        let err = AppError::from(ValidationError::InvalidInput("bad url".into()));
        assert_eq!(err.code, "VALIDATION_ERROR");
        assert_eq!(err.details, Some("bad url".to_string()));
    }

    #[test]
    fn validation_error_empty_query() {
        let err = AppError::from(ValidationError::EmptyQuery);
        assert_eq!(err.code, "VALIDATION_ERROR");
        assert_eq!(err.details, Some("query must not be empty".to_string()));
    }

    #[test]
    fn ipc_error_command_failed() {
        let err = AppError::from(IPCError::CommandFailed("timeout".into()));
        assert_eq!(err.code, "IPC_ERROR");
        assert_eq!(err.details, Some("timeout".to_string()));
    }

    #[test]
    fn ipc_error_serialization() {
        let err = AppError::from(IPCError::SerializationError("bad json".into()));
        assert_eq!(err.code, "IPC_ERROR");
        assert_eq!(err.details, Some("bad json".to_string()));
    }

    #[test]
    fn playback_error_no_audio_device() {
        let err = AppError::from(PlaybackError::NoAudioDevice("no output".into()));
        assert_eq!(err.code, "DEVICE_NOT_FOUND");
        assert_eq!(err.details, Some("no output".to_string()));
    }

    #[test]
    fn playback_error_decode_failed() {
        let err = AppError::from(PlaybackError::DecodeFailed("corrupt header".into()));
        assert_eq!(err.code, "PLAYBACK_ERROR");
        assert!(err.details.as_ref().unwrap().contains("decode: corrupt header"));
    }

    #[test]
    fn audio_error_no_audio_device() {
        let err = AppError::from(AudioError::NoAudioDevice("not found".into()));
        assert_eq!(err.code, "DEVICE_NOT_FOUND");
        assert_eq!(err.details, Some("not found".to_string()));
    }

    #[test]
    fn audio_error_decode_failed() {
        let err = AppError::from(AudioError::DecodeFailed("codec error".into()));
        assert_eq!(err.code, "PLAYBACK_ERROR");
        assert!(err.details.as_ref().unwrap().contains("decode: codec error"));
    }
}