//! Audio pipeline module.

pub mod decoder;
pub mod fft;
pub mod http_stream;
pub mod output;
pub mod pipeline;

// Re-export PlaybackState from the engine.
pub use jellyx_engine::playback_models::PlaybackState;

// Re-export AudioBackend trait and AudioError from the engine.
pub use jellyx_engine::audio_backend::{AudioBackend, AudioError};
