//! Playback event emissions via Tauri v2 AppHandle.
//!
//! `PlaybackEventEmitter` wraps `AppHandle` and provides typed methods
//! for emitting playback events to the frontend.

use crate::errors::types::IPCError;
use crate::models::track::Track;
use crate::playback::models::ProgressTick;
use crate::playback::state::{PlaybackState, QueueState};
use tauri::{AppHandle, Emitter, Runtime};

/// Event name constants for playback events.
/// Using lowercase-hyphen format per design convention.
pub const EVENT_TRACK_CHANGED: &str = "track-changed";
pub const EVENT_STATE_CHANGED: &str = "state-changed";
pub const EVENT_QUEUE_UPDATED: &str = "queue-updated";
pub const EVENT_PROGRESS_TICK: &str = "progress-tick";
#[allow(dead_code)]
pub const EVENT_BUFFERING_PROGRESS: &str = "buffering-progress";
pub const EVENT_STREAM_RESOLVED: &str = "stream-resolved";
pub const EVENT_CACHE_CORRUPTED: &str = "cache-corrupted";

/// Buffering progress payload emitted when a remote track is buffering.
///
/// Serialized as camelCase to match TypeScript frontend types.
#[allow(dead_code)]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BufferingProgress {
    /// Progress percentage from 0.0 to 1.0.
    pub progress: f32,
    /// The ID of the track being buffered.
    pub track_id: String,
}

/// Stream resolved payload emitted when a remote track's stream URL is ready.
///
/// The frontend uses `stream_url` with HTMLAudio for browser-native playback.
/// `remote_url` carries the raw remote URL so the frontend can request a local
/// cache download for YouTube tracks without re-resolving via yt-dlp.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamResolved {
    /// The ID of the track.
    pub track_id: String,
    /// The (proxied) stream URL the frontend should load.
    pub stream_url: String,
    /// The raw remote stream URL (before proxying). Present for remote tracks
    /// so the frontend can call `cache_remote_stream` to download a local copy
    /// for instant seeking. `None` for local-file proxy URLs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote_url: Option<String>,
}

/// Emits typed playback events via Tauri's event system.
pub struct PlaybackEventEmitter<R: Runtime = tauri::Wry> {
    app: AppHandle<R>,
}

impl<R: Runtime> PlaybackEventEmitter<R> {
    pub fn new(app: AppHandle<R>) -> Self {
        Self { app }
    }

    /// Emit a track-changed event with the given Track payload.
    pub fn emit_track_changed(&self, track: &Track) -> Result<(), IPCError> {
        self.app
            .emit(EVENT_TRACK_CHANGED, track)
            .map_err(|e| IPCError::CommandFailed(e.to_string()))
    }

    /// Emit a state-changed event with the given PlaybackState payload.
    pub fn emit_state_changed(&self, state: &PlaybackState) -> Result<(), IPCError> {
        self.app
            .emit(EVENT_STATE_CHANGED, state)
            .map_err(|e| IPCError::CommandFailed(e.to_string()))
    }

    /// Emit a queue-updated event with the full queue snapshot as payload.
    pub fn emit_queue_updated(&self, queue: &QueueState) -> Result<(), IPCError> {
        self.app
            .emit(EVENT_QUEUE_UPDATED, queue)
            .map_err(|e| IPCError::CommandFailed(e.to_string()))
    }

    /// Emit a progress-tick event with position and duration.
    pub fn emit_progress_tick(&self, position: f64, duration: f64) -> Result<(), IPCError> {
        let tick = ProgressTick { position, duration };
        self.app
            .emit(EVENT_PROGRESS_TICK, tick)
            .map_err(|e| IPCError::CommandFailed(e.to_string()))
    }

    /// Emit a buffering-progress event with progress percentage and track ID.
    #[allow(dead_code)]
    pub fn emit_buffering_progress(&self, progress: f32, track_id: &str) -> Result<(), IPCError> {
        let payload = BufferingProgress {
            progress,
            track_id: track_id.to_string(),
        };
        self.app
            .emit(EVENT_BUFFERING_PROGRESS, payload)
            .map_err(|e| IPCError::CommandFailed(e.to_string()))
    }

    /// Emit a stream-resolved event with the proxied stream URL.
    ///
    /// Sent when a remote track's stream URL has been resolved and proxied.
    /// The frontend uses this URL with HTMLAudio for browser-native playback.
    /// If `remote_url` is provided, the frontend can call `cache_remote_stream`
    /// to download a local copy for instant seeking (YouTube).
    pub fn emit_stream_resolved(&self, track_id: &str, stream_url: &str, remote_url: Option<&str>) -> Result<(), IPCError> {
        let payload = StreamResolved {
            track_id: track_id.to_string(),
            stream_url: stream_url.to_string(),
            remote_url: remote_url.map(|s| s.to_string()),
        };
        self.app
            .emit(EVENT_STREAM_RESOLVED, payload)
            .map_err(|e| IPCError::CommandFailed(e.to_string()))
    }

    /// Emit a cache-corrupted event when a cached stream file fails validation.
    ///
    /// This signals that a previous download left a corrupt file and the
    /// frontend should stay on the proxy URL instead of swapping to local.
    /// Logged to stdout so issues are visible without DevTools.
    pub fn emit_cache_corrupted(&self, source_id: &str, reason: &str) -> Result<(), IPCError> {
        eprintln!(
            "[cache] corrupted cache file for source_id={}: {}",
            source_id, reason
        );
        let payload = serde_json::json!({
            "sourceId": source_id,
            "reason": reason,
        });
        self.app
            .emit(EVENT_CACHE_CORRUPTED, payload)
            .map_err(|e| IPCError::CommandFailed(e.to_string()))
    }

    /// Clone the emitter for use in another thread.
    ///
    /// `AppHandle` is `Clone + Send + Sync`, so this is safe.
    pub fn clone_sender(&self) -> Self {
        Self {
            app: self.app.clone(),
        }
    }
}

#[cfg(test)]
impl PlaybackEventEmitter<tauri::test::MockRuntime> {
    /// Create a no-op emitter for unit tests using Tauri's mock runtime.
    ///
    /// Event emissions are swallowed by the mock runtime, so tests can focus
    /// on state mutations without needing a real Tauri app.
    pub fn test() -> Self {
        Self {
            app: tauri::test::mock_app().handle().clone(),
        }
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
        assert_eq!(EVENT_BUFFERING_PROGRESS, "buffering-progress");
        assert_eq!(EVENT_STREAM_RESOLVED, "stream-resolved");
    }

    #[test]
    fn stream_resolved_serializes_camel_case() {
        let payload = StreamResolved {
            track_id: "t-1".to_string(),
            stream_url: "http://127.0.0.1:8765/proxy?url=abc".to_string(),
            remote_url: Some("https://remote.example.com/stream".to_string()),
        };
        let json = serde_json::to_string(&payload).unwrap();
        assert!(json.contains("\"trackId\""), "track_id should serialize as trackId");
        assert!(json.contains("\"streamUrl\""), "stream_url should serialize as streamUrl");
        assert!(json.contains("\"remoteUrl\""), "remote_url should serialize as remoteUrl");
    }

    #[test]
    fn stream_resolved_skips_none_remote_url() {
        let payload = StreamResolved {
            track_id: "t-1".to_string(),
            stream_url: "http://127.0.0.1:8765/proxy?url=abc".to_string(),
            remote_url: None,
        };
        let json = serde_json::to_string(&payload).unwrap();
        assert!(json.contains("\"trackId\""), "track_id should serialize as trackId");
        assert!(json.contains("\"streamUrl\""), "stream_url should serialize as streamUrl");
        assert!(!json.contains("\"remoteUrl\""), "None remote_url should be skipped");
    }
}
