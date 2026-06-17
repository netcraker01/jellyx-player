//! Playback domain module.
//!
//! Contains the PlaybackService facade, state management,
//! event emissions, and internal DTOs.

pub mod events;
pub mod models;
pub mod service;
pub mod state;

// Re-export key types for convenience.
pub use events::PlaybackEventEmitter;
pub use models::ProgressTick;
pub use service::PlaybackService;
pub use state::{PlaybackState, QueueState};