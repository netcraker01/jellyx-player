//! Playback service orchestration.
//!
//! Placeholder — will be replaced with PlaybackService facade in Phase 2.

use std::sync::Mutex;

use crate::audio::AudioBackend;

/// Placeholder PlaybackService for module export.
/// Full implementation coming in Phase 2.
pub struct PlaybackService {
    _audio: Mutex<Box<dyn AudioBackend + Send>>,
}

impl PlaybackService {
    pub fn new(audio: Box<dyn AudioBackend + Send>) -> Self {
        Self {
            _audio: Mutex::new(audio),
        }
    }
}