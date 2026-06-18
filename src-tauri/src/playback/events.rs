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
        let tick = ProgressTick {
            position,
            duration,
        };
        self.app
            .emit(EVENT_PROGRESS_TICK, tick)
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
    }
}