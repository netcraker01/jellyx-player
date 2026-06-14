//! Audio pipeline module.
//!
//! Platform-agnostic audio backend trait.
//! Desktop uses `cpal + symphonia`. Mobile will use platform backends.

pub mod playback;
pub mod fft;

/// Trait that abstracts the audio backend for cross-platform support.
///
/// Desktop → cpal backend
/// Android → Oboe backend
/// iOS → AVAudioEngine backend
pub trait AudioBackend {
    fn play(&mut self, url: &str) -> Result<(), AudioError>;
    fn pause(&mut self) -> Result<(), AudioError>;
    fn resume(&mut self) -> Result<(), AudioError>;
    fn stop(&mut self) -> Result<(), AudioError>;
    fn seek(&mut self, position: f64) -> Result<(), AudioError>;
    fn volume(&mut self, level: f32) -> Result<(), AudioError>;
    fn position(&self) -> f64;
    fn state(&self) -> PlaybackState;
}

#[derive(Debug, Clone, PartialEq)]
pub enum PlaybackState {
    Stopped,
    Playing,
    Paused,
    Buffering,
}

#[derive(Debug)]
pub enum AudioError {
    DecodeError(String),
    DeviceError(String),
    UnsupportedFormat,
    PlatformNotSupported,
}
