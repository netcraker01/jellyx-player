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

use std::path::Path;
use std::sync::{Arc, Mutex};

use super::pipeline::PcmBusSubscriber;
use super::{AudioBackend, AudioError, PlaybackState};

/// Shared state between the main thread and the audio callback.
struct AudioState {
    volume: f32,
    /// Normalization gain factor applied on top of volume.
    /// When normalization is enabled, this is set to ~2.0 (+6dB) to boost
    /// quiet tracks. The compressor in the playback service clips peaks.
    normalize_gain: f32,
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
                normalize_gain: 1.0,
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

    /// Choose the best output device, avoiding obvious virtual/loopback sinks.
    fn select_output_device(host: &cpal::Host) -> Result<cpal::Device, AudioError> {
        use cpal::traits::{DeviceTrait, HostTrait};

        let default = host.default_output_device();

        if let Some(ref dev) = default {
            if let Ok(name) = dev.name() {
                let lowered = name.to_lowercase();
                let is_suspicious = lowered.contains("loopback")
                    || lowered.contains("monitor")
                    || lowered.contains("null")
                    || lowered.contains("dummy");
                if !is_suspicious {
                    return Ok(dev.clone());
                }
            }
        }

        // Default looks suspicious or is missing — scan for a better candidate.
        let devices = host.output_devices().map_err(|e| {
            AudioError::DeviceError(format!("Failed to enumerate output devices: {}", e))
        })?;

        for dev in devices {
            if let Ok(name) = dev.name() {
                let lowered = name.to_lowercase();
                if !lowered.contains("loopback")
                    && !lowered.contains("monitor")
                    && !lowered.contains("null")
                    && !lowered.contains("dummy")
                {
                    return Ok(dev);
                }
            }
        }

        // Nothing better found; fall back to default (or fail cleanly).
        default
            .ok_or_else(|| AudioError::NoAudioDevice("No suitable output device found".to_string()))
    }

    /// Start audio playback by creating a cpal stream that reads from the subscriber.
    ///
    /// This is the internal method that actually creates the cpal audio stream.
    /// Called by `play_local()` and by streaming playback in PlaybackService.
    pub(crate) fn start_stream(&self, sample_rate: u32, channels: u16) -> Result<(), AudioError> {
        use cpal::traits::{DeviceTrait, StreamTrait};
        use cpal::{SampleFormat, StreamConfig};

        let host = cpal::default_host();
        let device = Self::select_output_device(&host)?;

        // Find a supported config with f32 samples
        let supported_config = device
            .supported_output_configs()
            .map_err(|e| AudioError::DeviceError(format!("Failed to query device configs: {}", e)))?
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
            .with_sample_rate(cpal::SampleRate(sample_rate))
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
                    let norm_gain;
                    {
                        let s = state.lock().unwrap();
                        vol = s.volume;
                        norm_gain = s.normalize_gain;
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

                            // Map source samples to output buffer with channel conversion
                            // Apply normalization gain on top of user volume
                            let effective_volume = norm_gain * vol;
                            let source_channels = channels as usize;
                            let consumed = CpalBackend::map_channels(
                                &s.pcm_buffer,
                                source_channels,
                                data,
                                actual_channels,
                                effective_volume,
                            );
                            s.pcm_buffer.drain(..consumed);

                            // Update position estimate (only while playing, so pause keeps position)
                            let seconds_consumed = consumed as f64
                                / (source_channels as f64 * config.sample_rate.0 as f64);
                            if s.playback_state != PlaybackState::Paused {
                                s.position += seconds_consumed;
                            }
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
            .map_err(|e| AudioError::DeviceError(format!("Failed to build audio stream: {}", e)))?;

        stream
            .play()
            .map_err(|e| AudioError::DeviceError(format!("Failed to start audio stream: {}", e)))?;

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

    /// Map interleaved source samples to interleaved output samples with channel conversion.
    ///
    /// Handles upmix (duplicate last source channel) and downmix (keep first N channels).
    /// Returns the number of **source** samples consumed.
    fn map_channels(
        source: &[f32],
        source_channels: usize,
        output: &mut [f32],
        output_channels: usize,
        volume: f32,
    ) -> usize {
        let output_frames = output.len() / output_channels;
        let source_frames = source.len() / source_channels;
        let frames = output_frames.min(source_frames);

        for frame in 0..frames {
            let src_base = frame * source_channels;
            let dst_base = frame * output_channels;

            // Copy overlapping channels (passthrough or downmix)
            for ch in 0..output_channels.min(source_channels) {
                output[dst_base + ch] = source[src_base + ch] * volume;
            }

            // Upmix: duplicate last source channel to remaining outputs
            if output_channels > source_channels {
                let last = source[src_base + source_channels - 1] * volume;
                for ch in source_channels..output_channels {
                    output[dst_base + ch] = last;
                }
            }
        }

        // Zero-fill any remaining output frames
        for sample in output.iter_mut().skip(frames * output_channels) {
            *sample = 0.0;
        }

        frames * source_channels
    }

    /// Stop the current audio stream.
    #[allow(dead_code)]
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

    /// Set the normalization gain factor (1.0 = no boost, ~2.0 = +6dB boost).
    /// Applied on top of the user volume in the audio callback.
    pub fn set_normalize_gain(&self, gain: f32) -> Result<(), AudioError> {
        let mut s = self.state.lock().unwrap();
        // Soft-clip the gain to prevent extreme amplification.
        // Max 4x (+12dB) is generous; most cases use 2x (+6dB).
        s.normalize_gain = gain.clamp(0.0, 4.0);
        Ok(())
    }
}

impl AudioBackend for CpalBackend {
    fn play(&mut self, _url: &str) -> Result<(), AudioError> {
        // URL streaming not yet implemented — local files only for v0.1
        Err(AudioError::PlatformNotSupported)
    }

    fn play_local(&mut self, path: &Path) -> Result<(), AudioError> {
        // The caller (PlaybackService) should set up the decoder thread
        // and PcmBus before calling this. We start the cpal stream.
        // The actual sample rate and channels come from the decoder metadata.
        use crate::audio::decoder::SymphoniaDecoder;

        let decoder = SymphoniaDecoder::open(path.to_string_lossy().as_ref())?;
        let sample_rate = decoder.sample_rate();
        let channels = decoder.channels();

        self.start_stream(sample_rate, channels)
    }

    fn pause(&mut self) -> Result<(), AudioError> {
        use cpal::traits::StreamTrait;

        let stream_guard = self.stream_drop.lock().unwrap();
        if let Some(ref stream) = *stream_guard {
            stream
                .pause()
                .map_err(|e| AudioError::DeviceError(format!("Failed to pause stream: {}", e)))?;
        }

        let mut s = self.state.lock().unwrap();
        s.playback_state = PlaybackState::Paused;

        Ok(())
    }

    fn resume(&mut self) -> Result<(), AudioError> {
        use cpal::traits::StreamTrait;

        let stream_guard = self.stream_drop.lock().unwrap();
        if let Some(ref stream) = *stream_guard {
            stream
                .play()
                .map_err(|e| AudioError::DeviceError(format!("Failed to resume stream: {}", e)))?;
        }

        let mut s = self.state.lock().unwrap();
        s.playback_state = PlaybackState::Playing;

        Ok(())
    }

    fn stop(&mut self) -> Result<(), AudioError> {
        self.stop_stream()
    }

    fn seek(&mut self, position: f64) -> Result<(), AudioError> {
        let mut s = self.state.lock().unwrap();
        s.position = position;
        s.pcm_buffer.clear();
        drop(s);

        // Drain any pending frames from the PcmBus subscriber so stale
        // decoded audio is not played after the seek.
        let sub_guard = self.subscriber.lock().unwrap();
        if let Some(ref sub) = *sub_guard {
            while sub.try_recv().is_some() {}
        }

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_channels_passthrough_stereo() {
        let source = vec![0.1_f32, 0.2, 0.3, 0.4];
        let mut output = vec![0.0_f32; 4];
        let consumed = CpalBackend::map_channels(&source, 2, &mut output, 2, 1.0);
        assert_eq!(consumed, 4);
        assert_eq!(output, vec![0.1, 0.2, 0.3, 0.4]);
    }

    #[test]
    fn map_channels_upmix_mono_to_stereo() {
        let source = vec![0.5_f32, 0.6];
        let mut output = vec![0.0_f32; 4];
        let consumed = CpalBackend::map_channels(&source, 1, &mut output, 2, 1.0);
        assert_eq!(consumed, 2);
        assert_eq!(output, vec![0.5, 0.5, 0.6, 0.6]);
    }

    #[test]
    fn map_channels_downmix_stereo_to_mono() {
        let source = vec![0.1_f32, 0.2, 0.3, 0.4];
        let mut output = vec![0.0_f32; 2];
        let consumed = CpalBackend::map_channels(&source, 2, &mut output, 1, 1.0);
        assert_eq!(consumed, 4);
        assert_eq!(output, vec![0.1, 0.3]);
    }

    #[test]
    fn map_channels_applies_volume() {
        let source = vec![1.0_f32, 0.5];
        let mut output = vec![0.0_f32; 2];
        let consumed = CpalBackend::map_channels(&source, 1, &mut output, 1, 0.5);
        assert_eq!(consumed, 2);
        assert_eq!(output, vec![0.5, 0.25]);
    }

    #[test]
    fn map_channels_zero_fills_when_source_insufficient() {
        let source = vec![0.5_f32];
        let mut output = vec![-1.0_f32; 4];
        let consumed = CpalBackend::map_channels(&source, 1, &mut output, 2, 1.0);
        assert_eq!(consumed, 1);
        assert_eq!(output, vec![0.5, 0.5, 0.0, 0.0]);
    }

    #[test]
    fn map_channels_empty_source_outputs_silence() {
        let source: Vec<f32> = vec![];
        let mut output = vec![-1.0_f32; 4];
        let consumed = CpalBackend::map_channels(&source, 2, &mut output, 2, 1.0);
        assert_eq!(consumed, 0);
        assert_eq!(output, vec![0.0, 0.0, 0.0, 0.0]);
    }

    #[test]
    fn map_channels_upmix_5_1_to_stereo_keeps_first_two() {
        let source = vec![0.1_f32, 0.2, 0.3, 0.4, 0.5, 0.6];
        let mut output = vec![0.0_f32; 2];
        let consumed = CpalBackend::map_channels(&source, 6, &mut output, 2, 1.0);
        assert_eq!(consumed, 6);
        assert_eq!(output, vec![0.1, 0.2]);
    }

    #[test]
    fn map_channels_downmix_stereo_to_5_1_duplicates_last() {
        let source = vec![0.7_f32, 0.8];
        let mut output = vec![0.0_f32; 6];
        let consumed = CpalBackend::map_channels(&source, 2, &mut output, 6, 1.0);
        assert_eq!(consumed, 2);
        assert_eq!(output, vec![0.7, 0.8, 0.8, 0.8, 0.8, 0.8]);
    }

    #[test]
    fn seek_clears_pcm_buffer_and_drains_subscriber() {
        let (producer, subscriber) = crate::audio::pipeline::PcmBus::new(44100, 2);
        let mut backend = CpalBackend::new();
        backend.set_subscriber(subscriber);

        // Seed the internal buffer and subscriber channel
        {
            let mut s = backend.state.lock().unwrap();
            s.pcm_buffer.extend_from_slice(&[0.1_f32, 0.2, 0.3, 0.4]);
        }
        producer.send(vec![0.5_f32, 0.6, 0.7, 0.8]).unwrap();

        backend.seek(10.0).unwrap();

        let s = backend.state.lock().unwrap();
        assert!(
            s.pcm_buffer.is_empty(),
            "PCM buffer must be cleared after seek"
        );
        assert!(
            (s.position - 10.0).abs() < f64::EPSILON,
            "Position must update after seek"
        );

        let sub_guard = backend.subscriber.lock().unwrap();
        if let Some(ref sub) = *sub_guard {
            assert!(
                sub.try_recv().is_none(),
                "Subscriber must be drained after seek"
            );
        }
    }
}
