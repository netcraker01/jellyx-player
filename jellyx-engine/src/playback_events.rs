//! Playback event emission trait — shared by all frontends.
//!
//! Desktop implements this with Tauri's `AppHandle::emit`.
//! The TUI will implement it with a channel-based or no-op emitter.

use crate::playback_models::{PlaybackState, ProgressTick, QueueState};
use jellyx_core::models::track::Track;
use serde::{Deserialize, Serialize};

/// Event name constants for playback events.
pub const EVENT_TRACK_CHANGED: &str = "track-changed";
pub const EVENT_STATE_CHANGED: &str = "state-changed";
pub const EVENT_QUEUE_UPDATED: &str = "queue-updated";
pub const EVENT_PROGRESS_TICK: &str = "progress-tick";
pub const EVENT_BUFFERING_PROGRESS: &str = "buffering-progress";
pub const EVENT_STREAM_RESOLVED: &str = "stream-resolved";
pub const EVENT_CACHE_CORRUPTED: &str = "cache-corrupted";

/// Buffering progress payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BufferingProgress {
    pub progress: f32,
    pub track_id: String,
}

/// Stream resolved payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamResolved {
    pub track_id: String,
    pub stream_request_id: u64,
    pub stream_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proxy_capability: Option<String>,
}

/// Error type for event emission.
#[derive(Debug, Clone)]
pub struct EmitError(pub String);

impl std::fmt::Display for EmitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Trait for emitting playback events to a frontend.
///
/// The engine uses this trait to notify frontends of state changes,
/// track switches, progress updates, and stream resolutions.
/// Desktop implements it with Tauri's event system; the TUI can
/// implement it with channels or ignore events entirely.
pub trait PlaybackEventEmitter: Send + Sync {
    fn emit_track_changed(&self, track: &Track) -> Result<(), EmitError>;
    fn emit_state_changed(&self, state: &PlaybackState) -> Result<(), EmitError>;
    fn emit_queue_updated(&self, queue: &QueueState) -> Result<(), EmitError>;
    fn emit_progress_tick(&self, position: f64, duration: f64) -> Result<(), EmitError>;
    fn emit_buffering_progress(&self, progress: f32, track_id: &str) -> Result<(), EmitError>;
    fn emit_stream_resolved(
        &self,
        track_id: &str,
        stream_request_id: u64,
        stream_url: &str,
        remote_url: Option<&str>,
        proxy_capability: Option<&str>,
    ) -> Result<(), EmitError>;
    fn emit_cache_corrupted(&self, source_id: &str, reason: &str) -> Result<(), EmitError>;
}

/// A no-op emitter for testing and headless TUI mode.
pub struct NoopEmitter;

impl PlaybackEventEmitter for NoopEmitter {
    fn emit_track_changed(&self, _track: &Track) -> Result<(), EmitError> {
        Ok(())
    }
    fn emit_state_changed(&self, _state: &PlaybackState) -> Result<(), EmitError> {
        Ok(())
    }
    fn emit_queue_updated(&self, _queue: &QueueState) -> Result<(), EmitError> {
        Ok(())
    }
    fn emit_progress_tick(&self, _position: f64, _duration: f64) -> Result<(), EmitError> {
        Ok(())
    }
    fn emit_buffering_progress(&self, _progress: f32, _track_id: &str) -> Result<(), EmitError> {
        Ok(())
    }
    fn emit_stream_resolved(
        &self,
        _track_id: &str,
        _stream_request_id: u64,
        _stream_url: &str,
        _remote_url: Option<&str>,
        _proxy_capability: Option<&str>,
    ) -> Result<(), EmitError> {
        Ok(())
    }
    fn emit_cache_corrupted(&self, _source_id: &str, _reason: &str) -> Result<(), EmitError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_name_constants_are_lowercase_hyphen() {
        assert_eq!(EVENT_TRACK_CHANGED, "track-changed");
        assert_eq!(EVENT_STATE_CHANGED, "state-changed");
        assert_eq!(EVENT_QUEUE_UPDATED, "queue-updated");
        assert_eq!(EVENT_PROGRESS_TICK, "progress-tick");
        assert_eq!(EVENT_STREAM_RESOLVED, "stream-resolved");
    }

    #[test]
    fn stream_resolved_serializes_camel_case() {
        let payload = StreamResolved {
            track_id: "t-1".to_string(),
            stream_request_id: 7,
            stream_url: "http://127.0.0.1:8765/proxy?url=abc".to_string(),
            remote_url: Some("https://remote.example.com/stream".to_string()),
            proxy_capability: Some("test-cap".to_string()),
        };
        let json = serde_json::to_string(&payload).unwrap();
        assert!(json.contains("\"trackId\""));
        assert!(json.contains("\"streamUrl\""));
        assert!(json.contains("\"remoteUrl\""));
        assert!(json.contains("\"proxyCapability\""));
    }

    #[test]
    fn noop_emitter_never_errors() {
        let emitter = NoopEmitter;
        assert!(
            emitter
                .emit_track_changed(&Track {
                    id: "t".into(),
                    source: jellyx_core::models::source::Source::Local,
                    source_id: "s".into(),
                    title: "Song".into(),
                    artist: "Artist".into(),
                    album: None,
                    duration: None,
                    thumbnail: None,
                    stream_url: None,
                    local_path: None,
                    playlist_id: None,
                    metadata: std::collections::HashMap::new(),
                })
                .is_ok()
        );
    }
}
