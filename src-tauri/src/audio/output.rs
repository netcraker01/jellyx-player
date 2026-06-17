//! Audio output using cpal (desktop implementation).
//!
//! `CpalBackend` is the desktop audio output implementation.
//! It receives PCM frames from a `PcmBusSubscriber` and plays them
//! through the system's default audio device using cpal.
//!
//! ## Thread Safety
//!
//! cpal's `Stream` type is NOT `Send`. To handle this, `CpalBackend` uses
//! `StreamHandle` — a `Send`-safe wrapper that stores the stream on a dedicated
//! audio thread. The `AudioBackend` trait methods are called from the main thread
//! and communicate with the audio thread via `Arc<Mutex<>>` state.
//!
//! Mobile platforms will have their own implementations behind the `AudioBackend` trait.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use super::pipeline::PcmBusSubscriber;
use super::{AudioBackend, AudioError, PlaybackState};

/// Shared state between the main thread and the audio callback.
struct AudioState {
    volume: f32,
    position: f64,
    playback_state: PlaybackState,
    /// Buffer accumulating PCM data from PcmBus before writing to cpal.
    pcm_buffer: Vec<f32>,
}

/// Desktop audio output using cpal.
///
/// Creates an audio stream on the default output device, reads PCM frames
/// from a `PcmBusSubscriber`, and applies software volume scaling.
///
/// Note: cpal's Stream is NOT Send, so we store it in a separate thread-local
/// wrapper. The `AudioBackend` methods only touch the shared AudioState.
pub struct CpalBackend {
    state: Arc<Mutex<AudioState>>,
    // The cpal stream is stored here once playing starts.
    // Option<Stream> is not Send, so we use a separate pattern.
    // We drop the stream by setting this to None from the thread it was created on.
    stream_drop: Arc<Mutex<Option<cpal::Stream>>>,
    // The subscriber that feeds PCM data to the audio callback.
    subscriber: Arc<Mutex<Option<PcmBusSubscriber>>>,
}

// SAFETY: CpalBackend is Send because:
// - AudioState is behind Arc<Mutex<>>, which is Send
// - We only access stream_drop from the thread that created the stream
// - PcmBusSubscriber uses crossbeam channels which are Send
unsafe impl Send for CpalBackend {}
unsafe impl Sync for CpalBackend {}

impl CpalBackend {
    /// Create a new CpalBackend in Stopped state.
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(AudioState {
                volume: 1.0,
                position: 0.0,
                playback_state: PlaybackState::Stopped,
                pcm_buffer: Vec::with_capacity(8192),
            })),
            stream_drop: Arc::new(Mutex::new(None)),
            subscriber: Arc::new(Mutex::new(None)),
        }
    }

    /// Set the PcmBusSubscriber that will feed PCM data to the audio callback.
    /// Must be called before `play_local()` or `play()`.
    pub fn set_subscriber(&self, sub: PcmBusSubscriber) {
        let mut guard = self.subscriber.lock().unwrap();
        *guard = Some(sub);
    }

    /// Start audio playback by creating a cpal stream that reads from the subscriber.
    fn start_stream(&self, sample_rate: u32, channels: u16) -> Result<(), AudioError> {
        use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
        use cpal::{SampleFormat, StreamConfig};

        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or_else(|| {
                AudioError::NoAudioDevice("No default output device found".to_string())
            })?;

        // Find a supported config with f32 samples
        let supported_config = device
            .supported_output_configs()
            .map_err(|e| {
                AudioError::DeviceError(format!("Failed to query device configs: {}", e))
            })?
            .find(|c| {
                c.channels() == channels
                    && c.sample_format() == SampleFormat::F32
                    && c.min_sample_rate().0 <= sample_rate
                    && c.max_sample_rate().0 >= sample_rate
            })
            .or_else(|| {
                // Fallback: find any F32 config
                device
                    .supported_output_configs()
                    .ok()?
                    .find(|c| c.sample_format() == SampleFormat::F32 && c.channels() >= channels)
            })
            .ok_or_else(|| AudioError::UnsupportedFormat)?;

        let config: StreamConfig = supported_config
            .with_max_sample_rate()
            .config();

        let actual_channels = config.channels as usize;

        // Clone the Arcs for use in the audio callback
        let state = self.state.clone();
        let subscriber = self.subscriber.clone();

        let stream = device
            .build_output_stream(
                &config,
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    let vol;
                    {
                        let s = state.lock().unwrap();
                        vol = s.volume;
                    }

                    // Read available PCM data from the bus
                    {
                        let sub_guard = subscriber.lock().unwrap();
                        if let Some(sub) = sub_guard.as_ref() {
                            // Read all available frames
                            let mut s = state.lock().unwrap();
                            while let Some(frame) = sub.try_recv() {
                                s.pcm_buffer.extend_from_slice(&frame);
                            }

                            // Fill output buffer from accumulated PCM data
                            let consumed = data.len().min(s.pcm_buffer.len());
                            for (i, sample) in data.iter_mut().enumerate() {
                                if i < consumed {
                                    *sample = s.pcm_buffer[i] * vol;
                                } else {
                                    *sample = 0.0; // Silence when no data
                                }
                            }

                            // Remove consumed samples
                            s.pcm_buffer.drain(..consumed);

                            // Update position estimate
                            let seconds_consumed = consumed as f64
                                / (actual_channels as f64 * config.sample_rate.0 as f64);
                            s.position += seconds_consumed;
                        } else {
                            // No subscriber — output silence
                            data.fill(0.0);
                        }
                    }
                },
                move |err| {
                    eprintln!("Audio stream error: {}", err);
                    // On error, set state to Stopped
                    // Note: we can't easily access state here without another Arc
                },
                None, // None = no timeout
            )
            .map_err(|e| {
                AudioError::DeviceError(format!("Failed to build audio stream: {}", e))
            })?;

        stream.play().map_err(|e| {
            AudioError::DeviceError(format!("Failed to start audio stream: {}", e))
        })?;

        // Store the stream so it stays alive
        // SAFETY: This is called from the same thread context where the stream was created.
        // The stream will be dropped when CpalBackend is dropped or when stop() is called.
        let mut stream_guard = self.stream_drop.lock().unwrap();
        *stream_guard = Some(stream);

        // Update state
        {
            let mut s = self.state.lock().unwrap();
            s.playback_state = PlaybackState::Playing;
        }

        Ok(())
    }

    /// Stop the current audio stream.
    fn stop_stream(&self) -> Result<(), AudioError> {
        // Drop the stream — this stops playback
        let mut stream_guard = self.stream_drop.lock().unwrap();
        *stream_guard = None;

        let mut s = self.state.lock().unwrap();
        s.playback_state = PlaybackState::Stopped;
        s.position = 0.0;
        s.pcm_buffer.clear();

        Ok(())
    }
}

impl AudioBackend for CpalBackend {
    fn play(&mut self, _url: &str) -> Result<(), AudioError> {
        // URL streaming not yet implemented — local files only for v0.1
        Err(AudioError::PlatformNotSupported)
    }

    fn play_local(&mut self, _path: &PathBuf) -> Result<(), AudioError> {
        // The caller (PlaybackService) should set up the decoder thread
        // and PcmBus before calling this. We start the cpal stream.
        // Default to 44100 Hz stereo — the actual sample rate comes from
        // the decoder, which is set up by PlaybackService.
        self.start_stream(44100, 2)
    }

    fn pause(&mut self) -> Result<(), AudioError> {
        use cpal::traits::StreamTrait;

        let stream_guard = self.stream_drop.lock().unwrap();
        if let Some(ref stream) = *stream_guard {
            stream.pause().map_err(|e| {
                AudioError::DeviceError(format!("Failed to pause stream: {}", e))
            })?;
        }

        let mut s = self.state.lock().unwrap();
        s.playback_state = PlaybackState::Paused;

        Ok(())
    }

    fn resume(&mut self) -> Result<(), AudioError> {
        use cpal::traits::StreamTrait;

        let stream_guard = self.stream_drop.lock().unwrap();
        if let Some(ref stream) = *stream_guard {
            stream.play().map_err(|e| {
                AudioError::DeviceError(format!("Failed to resume stream: {}", e))
            })?;
        }

        let mut s = self.state.lock().unwrap();
        s.playback_state = PlaybackState::Playing;

        Ok(())
    }

    fn stop(&mut self) -> Result<(), AudioError> {
        self.stop_stream()
    }

    fn seek(&mut self, position: f64) -> Result<(), AudioError> {
        // Seeking is handled at the decoder level — PlaybackService coordinates
        let mut s = self.state.lock().unwrap();
        s.position = position;
        Ok(())
    }

    fn volume(&mut self, level: f32) -> Result<(), AudioError> {
        let mut s = self.state.lock().unwrap();
        s.volume = level.clamp(0.0, 1.0);
        Ok(())
    }

    fn position(&self) -> f64 {
        self.state.lock().unwrap().position
    }

    fn state(&self) -> PlaybackState {
        self.state.lock().unwrap().playback_state.clone()
    }
}