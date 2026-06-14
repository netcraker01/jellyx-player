//! Audio playback pipeline using symphonia (decoding) + cpal (output).
//!
//! This is the desktop implementation. Mobile platforms will have
//! their own implementations behind the `AudioBackend` trait.

use super::{AudioBackend, AudioError, PlaybackState};
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;

pub struct CpalBackend {
    state: Arc<Mutex<PlaybackState>>,
    // TODO: real implementation
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
        // TODO: implement full pipeline
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
