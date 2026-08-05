//! Playback event emissions via Tauri v2 AppHandle.
//!
//! Implements the engine's `PlaybackEventEmitter` trait using Tauri's
//! event system. Event name constants, payload types, and the trait
//! contract live in `jellyx_engine::playback_events`.

use jellyx_core::models::track::Track;
use tauri::{AppHandle, Emitter, Runtime};

use crate::playback::models::ProgressTick;
use crate::playback::state::{PlaybackState, QueueState};

// Re-export shared types and constants for backward compatibility.
pub use jellyx_engine::playback_events::{
    BufferingProgress, EmitError, PlaybackEventEmitter as EngineEmitter, StreamResolved,
    EVENT_BUFFERING_PROGRESS, EVENT_CACHE_CORRUPTED, EVENT_PROGRESS_TICK, EVENT_QUEUE_UPDATED,
    EVENT_STATE_CHANGED, EVENT_STREAM_RESOLVED, EVENT_TRACK_CHANGED,
};

/// Tauri-backed implementation of the engine's PlaybackEventEmitter trait.
pub struct PlaybackEventEmitter<R: Runtime = tauri::Wry> {
    app: AppHandle<R>,
}

impl<R: Runtime> PlaybackEventEmitter<R> {
    pub fn new(app: AppHandle<R>) -> Self {
        Self { app }
    }

    /// Clone the emitter for use in another thread.
    pub fn clone_sender(&self) -> Self {
        Self {
            app: self.app.clone(),
        }
    }

    /// Return a reference to the underlying `AppHandle`.
    pub fn app_handle(&self) -> &AppHandle<R> {
        &self.app
    }
}

impl<R: Runtime> EngineEmitter for PlaybackEventEmitter<R> {
    fn emit_track_changed(&self, track: &Track) -> Result<(), EmitError> {
        self.app
            .emit(EVENT_TRACK_CHANGED, track)
            .map_err(|e| EmitError(e.to_string()))
    }

    fn emit_state_changed(&self, state: &PlaybackState) -> Result<(), EmitError> {
        self.app
            .emit(EVENT_STATE_CHANGED, state)
            .map_err(|e| EmitError(e.to_string()))
    }

    fn emit_queue_updated(&self, queue: &QueueState) -> Result<(), EmitError> {
        self.app
            .emit(EVENT_QUEUE_UPDATED, queue)
            .map_err(|e| EmitError(e.to_string()))
    }

    fn emit_progress_tick(&self, position: f64, duration: f64) -> Result<(), EmitError> {
        let tick = ProgressTick { position, duration };
        self.app
            .emit(EVENT_PROGRESS_TICK, tick)
            .map_err(|e| EmitError(e.to_string()))
    }

    fn emit_buffering_progress(&self, progress: f32, track_id: &str) -> Result<(), EmitError> {
        let payload = BufferingProgress {
            progress,
            track_id: track_id.to_string(),
        };
        self.app
            .emit(EVENT_BUFFERING_PROGRESS, payload)
            .map_err(|e| EmitError(e.to_string()))
    }

    fn emit_stream_resolved(
        &self,
        track_id: &str,
        stream_request_id: u64,
        stream_url: &str,
        remote_url: Option<&str>,
        proxy_capability: Option<&str>,
    ) -> Result<(), EmitError> {
        let payload = StreamResolved {
            track_id: track_id.to_string(),
            stream_request_id,
            stream_url: stream_url.to_string(),
            remote_url: remote_url.map(|s| s.to_string()),
            proxy_capability: proxy_capability.map(|s| s.to_string()),
        };
        self.app
            .emit(EVENT_STREAM_RESOLVED, payload)
            .map_err(|e| EmitError(e.to_string()))
    }

    fn emit_cache_corrupted(&self, source_id: &str, reason: &str) -> Result<(), EmitError> {
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
            .map_err(|e| EmitError(e.to_string()))
    }
}

#[cfg(test)]
impl PlaybackEventEmitter<tauri::test::MockRuntime> {
    pub fn test() -> Self {
        Self {
            app: tauri::test::mock_app().handle().clone(),
        }
    }
}
