//! Tauri event emissions — event name constants and re-exports.
//!
//! Defines the event names used in IPC between Rust and Svelte.
//! The actual emission logic lives in `playback::events::PlaybackEventEmitter`.

// Re-export event name constants and PlaybackEventEmitter for convenience.
pub use crate::playback::events::{
    PlaybackEventEmitter, EVENT_FREQUENCY_DATA, EVENT_PROGRESS_TICK, EVENT_QUEUE_UPDATED,
    EVENT_STATE_CHANGED, EVENT_TRACK_CHANGED,
};