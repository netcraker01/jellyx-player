//! Audio output using cpal (desktop implementation).
//!
//! `CpalBackend` is the desktop audio output implementation.
//! Mobile platforms will have their own implementations behind the `AudioBackend` trait.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use super::{AudioBackend, AudioError, PlaybackState};

pub struct CpalBackend {
    state: Arc<Mutex<PlaybackState>>,
    // TODO: real cpal Stream will be added in Phase 2
}

impl CpalBackend {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(PlaybackState::Stopped)),
        }
    }
}

impl AudioBackend for CpalBackend {
    fn play(&mut self, _url: &str) -> Result<(), AudioError> {
        // TODO: implement streaming URL playback (future)
        Ok(())
    }

    fn play_local(&mut self, _path: &PathBuf) -> Result<(), AudioError> {
        // TODO: implement in Phase 2 — will start decoder thread + cpal stream
        Ok(())
    }

    fn pause(&mut self) -> Result<(), AudioError> {
        // TODO
        Ok(())
    }

    fn resume(&mut self) -> Result<(), AudioError> {
        // TODO
        Ok(())
    }

    fn stop(&mut self) -> Result<(), AudioError> {
        // TODO
        Ok(())
    }

    fn seek(&mut self, _position: f64) -> Result<(), AudioError> {
        // TODO
        Ok(())
    }

    fn volume(&mut self, _level: f32) -> Result<(), AudioError> {
        // TODO
        Ok(())
    }

    fn position(&self) -> f64 {
        0.0
    }

    fn state(&self) -> PlaybackState {
        self.state.lock().unwrap().clone()
    }
}