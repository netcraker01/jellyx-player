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
    /// Whether the current stream has already reported a CPAL stream error.
    ///
    /// CPAL may invoke the error callback repeatedly for the same underlying
    /// failure (e.g. device underrun/invalidated). Without this flag, every
    /// invocation would log the same error, spamming the console indefinitely.
    /// The flag is reset to `false` whenever a fresh stream is started in
    /// `start_stream`, so each stream lifecycle logs at most one error.
    stream_error_reported: bool,
}

impl AudioState {
    /// Record a CPAL stream error: transition playback to a safe `Stopped`
    /// state, drop any buffered PCM, and report whether this is the first
    /// error for the current stream.
    ///
    /// Returning `true` means the caller should log the error; `false` means
    /// a previous invocation already logged it for this stream, so the caller
    /// must stay silent to avoid log spam.
    fn record_stream_error(&mut self) -> bool {
        let first_error = !self.stream_error_reported;
        self.stream_error_reported = true;
        self.playback_state = PlaybackState::Stopped;
        self.pcm_buffer.clear();
        first_error
    }
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
                stream_error_reported: false,
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

    /// Prepare shared state for a fresh stream lifecycle.
    ///
    /// Sets `playback_state = Playing` and clears `stream_error_reported` so
    /// the next CPAL error logs once. This MUST be called BEFORE `stream.play()`
    /// so that an error callback firing immediately after play sees a correct
    /// fresh lifecycle and transitions to `Stopped` without being overwritten.
    ///
    /// Returns the previous state so callers can revert on `play()` failure.
    fn prepare_for_stream_start(&self) -> PlaybackState {
        let mut s = self.state.lock().unwrap();
        let prev = s.playback_state.clone();
        s.stream_error_reported = false;
        s.playback_state = PlaybackState::Playing;
        prev
    }

    /// Revert state after a failed `stream.play()`.
    ///
    /// Called when `play()` returns an error: the stream never went live, so
    /// the backend must not remain in a false `Playing` state.
    fn revert_failed_stream_start(&self, prev: PlaybackState) {
        let mut s = self.state.lock().unwrap();
        s.playback_state = prev;
        s.stream_error_reported = false;
        s.pcm_buffer.clear();
    }

    /// Render one audio callback tick into `data`.
    ///
    /// This is the pure body of the cpal data callback, extracted so it can be
    /// exercised by behavior-level tests without real audio hardware. It reads
    /// the current `playback_state`, volume and normalization gain from `state`,
    /// drains any available PCM frames from `subscriber` into `state.pcm_buffer`,
    /// maps the buffer into `data` with channel conversion, and advances the
    /// position estimate.
    ///
    /// Contract:
    /// - If `playback_state == Stopped` (e.g. after a stream error), `data` is
    ///   filled with silence and no PCM is consumed — the stopped stream must
    ///   never emit stale buffered audio.
    /// - If there is no subscriber, `data` is filled with silence.
    /// - Otherwise `data` receives volume-scaled, channel-converted samples and
    ///   `state.position` advances by the consumed sample duration (unless
    ///   paused, which preserves position).
    fn render_callback(
        state: &Mutex<AudioState>,
        subscriber: &Mutex<Option<PcmBusSubscriber>>,
        data: &mut [f32],
        source_channels: usize,
        output_channels: usize,
        sample_rate: u32,
    ) {
        let vol;
        let norm_gain;
        let is_stopped;
        {
            let s = state.lock().unwrap();
            vol = s.volume;
            norm_gain = s.normalize_gain;
            is_stopped = s.playback_state == PlaybackState::Stopped;
        }

        // If the stream errored (or was stopped), output silence and do not
        // consume any PCM data. This avoids feeding stale frames to a stream
        // that has already been logically stopped due to an error, which could
        // otherwise keep playing garbage until the stream handle is dropped.
        if is_stopped {
            data.fill(0.0);
            return;
        }

        // Read available PCM data from the bus
        let sub_guard = subscriber.lock().unwrap();
        if let Some(sub) = sub_guard.as_ref() {
            // Read all available frames
            let mut s = state.lock().unwrap();
            while let Some(frame) = sub.try_recv() {
                s.pcm_buffer.extend_from_slice(&frame);
            }

            // Map source samples to output buffer with channel conversion.
            // Apply normalization gain on top of user volume.
            let effective_volume = norm_gain * vol;
            let consumed = CpalBackend::map_channels(
                &s.pcm_buffer,
                source_channels,
                data,
                output_channels,
                effective_volume,
            );
            s.pcm_buffer.drain(..consumed);

            // Update position estimate (only while playing, so pause keeps position)
            let seconds_consumed =
                consumed as f64 / (source_channels as f64 * sample_rate as f64);
            if s.playback_state != PlaybackState::Paused {
                s.position += seconds_consumed;
            }
        } else {
            // No subscriber — output silence
            data.fill(0.0);
        }
    }

    /// Handle one cpal stream-error callback tick.
    ///
    /// This is the pure body of the cpal error callback, extracted so it can be
    /// exercised by behavior-level tests without real audio hardware. It
    /// transitions `playback_state` to `Stopped`, drops any buffered PCM, and
    /// dedups the log so repeated error callbacks for the same stream lifecycle
    /// do not spam the console.
    ///
    /// Returns `true` when the caller should log the error (the first error of
    /// the current lifecycle) and `false` for subsequent callbacks in the same
    /// lifecycle (already logged — stay silent). A poisoned state mutex yields
    /// `false` so a panic in one callback does not amplify into log spam.
    fn handle_stream_error(state: &Mutex<AudioState>) -> bool {
        let mut s = match state.lock() {
            Ok(guard) => guard,
            // State mutex poisoned — nothing useful to do here, but still avoid
            // spamming logs on every callback.
            Err(_) => return false,
        };
        s.record_stream_error()
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

        // Clone the Arcs for use in the audio callback.
        //
        // The cpal data/error callbacks are `FnMut` closures invoked from the
        // audio thread. They capture `state`, `subscriber` and `state_err` by
        // move and route through the pure `render_callback` / `handle_stream_error`
        // helpers, which is what the behavior-level tests exercise directly.
        let state = self.state.clone();
        let state_err = self.state.clone();
        let subscriber = self.subscriber.clone();

        let stream = device
            .build_output_stream(
                &config,
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    CpalBackend::render_callback(
                        &state,
                        &subscriber,
                        data,
                        channels as usize,
                        actual_channels,
                        config.sample_rate.0,
                    );
                },
                move |err| {
                    // Transition playback to a safe Stopped state and dedup the
                    // log so repeated CPAL error callbacks do not spam the
                    // console. Only the first error for the current stream
                    // lifecycle is logged; subsequent invocations silently
                    // update state (which is already Stopped) and return
                    // without logging.
                    if CpalBackend::handle_stream_error(&state_err) {
                        eprintln!("Audio stream error: {}", err);
                    }
                },
                None, // None = no timeout
            )
            .map_err(|e| AudioError::DeviceError(format!("Failed to build audio stream: {}", e)))?;

        // Mark the fresh stream lifecycle BEFORE going live.
        //
        // `stream.play()` activates the CPAL stream and the error callback can
        // fire immediately afterwards (e.g. device invalidated, underrun). If we
        // set `Playing` / clear `stream_error_reported` AFTER `play()`, an
        // error callback firing in that window would:
        //   1. transition state to `Stopped` (correct), then
        //   2. be overwritten back to `Playing` by our late update, and
        //   3. have its `stream_error_reported=true` clobbered back to `false`,
        //      re-enabling duplicate logging for the same failed lifecycle.
        //
        // Preparing state first means the callback observes a correct fresh
        // lifecycle and its `Stopped` transition survives.
        let prev_state = self.prepare_for_stream_start();

        let play_result = stream.play();

        if let Err(e) = play_result {
            // `play()` failed — the stream never went live. Revert the
            // speculative `Playing` state so we don't advertise a stream that
            // is not running, and drop the stream handle.
            self.revert_failed_stream_start(prev_state);
            let mut stream_guard = self.stream_drop.lock().unwrap();
            *stream_guard = None;
            return Err(AudioError::DeviceError(format!(
                "Failed to start audio stream: {}",
                e
            )));
        }

        // Store the stream so it stays alive.
        // SAFETY: This is called from the same thread context where the stream
        // was created. The stream will be dropped when CpalBackend is dropped
        // or when stop() is called.
        let mut stream_guard = self.stream_drop.lock().unwrap();
        *stream_guard = Some(stream);

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
        // No stream is alive, so a future start_stream resets this flag anyway;
        // reset it here too for consistency and to avoid a stale-true state.
        s.stream_error_reported = false;

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

    fn state_playing_with_buffer() -> AudioState {
        AudioState {
            volume: 1.0,
            normalize_gain: 1.0,
            position: 5.0,
            playback_state: PlaybackState::Playing,
            pcm_buffer: vec![0.1_f32, 0.2, 0.3, 0.4],
            stream_error_reported: false,
        }
    }

    #[test]
    fn record_stream_error_transitions_to_stopped_and_returns_true_first_time() {
        let mut s = state_playing_with_buffer();

        let first = s.record_stream_error();
        assert!(first, "First error should report should_log=true");
        assert_eq!(
            s.playback_state,
            PlaybackState::Stopped,
            "State must transition to Stopped on stream error"
        );
        assert!(
            s.pcm_buffer.is_empty(),
            "PCM buffer must be cleared on stream error to avoid stale audio"
        );
    }

    #[test]
    fn record_stream_error_dedups_repeated_invocations() {
        let mut s = state_playing_with_buffer();

        let first = s.record_stream_error();
        let second = s.record_stream_error();
        let third = s.record_stream_error();

        assert!(first, "First invocation must be logged");
        assert!(
            !second,
            "Second invocation must NOT be logged (already reported)"
        );
        assert!(
            !third,
            "Subsequent invocations must NOT be logged (already reported)"
        );
        assert_eq!(
            s.playback_state,
            PlaybackState::Stopped,
            "State must remain Stopped after repeated errors"
        );
    }

    #[test]
    fn new_backend_has_error_flag_unset() {
        // A freshly created backend must start with stream_error_reported=false
        // so the first genuine error on the first stream is logged exactly once.
        let backend = CpalBackend::new();
        let s = backend.state.lock().unwrap();
        assert!(
            !s.stream_error_reported,
            "New backend must start with stream_error_reported=false"
        );
    }

    #[test]
    fn stop_stream_resets_error_flag_for_next_stream() {
        let (producer, subscriber) = crate::audio::pipeline::PcmBus::new(44100, 2);
        let _ = producer; // unused, but required to keep the bus alive
        let mut backend = CpalBackend::new();
        backend.set_subscriber(subscriber);

        // Simulate a stream error before stopping
        {
            let mut s = backend.state.lock().unwrap();
            let should_log = s.record_stream_error();
            assert!(should_log);
        }

        backend.stop().unwrap();

        let s = backend.state.lock().unwrap();
        assert!(
            !s.stream_error_reported,
            "stop() must reset stream_error_reported so the next stream can log again"
        );
        assert_eq!(s.playback_state, PlaybackState::Stopped);
    }

    // === Regression coverage for the start_stream() ordering race ===
    //
    // The original bug: `start_stream()` called `stream.play()` and only
    // afterwards reset `stream_error_reported` and set `playback_state =
    // Playing`. A CPAL error callback firing in that window set the state to
    // `Stopped`, which was then overwritten back to `Playing`, and the error
    // flag was clobbered back to `false`, re-enabling duplicate logging.
    //
    // These tests exercise the behavior at the ordering boundary — they do
    // not call `start_stream()` (which needs real hardware) but they simulate
    // the exact interleaving the fix must survive: state is prepared for a
    // fresh lifecycle, an error callback fires, and no later "set Playing"
    // step may overwrite the resulting `Stopped` transition.

    #[test]
    fn prepare_for_stream_start_sets_playing_and_clears_error_flag_before_play() {
        // Precondition: a previously errored stream left state Stopped with
        // stream_error_reported=true (as record_stream_error does).
        let backend = CpalBackend::new();
        {
            let mut s = backend.state.lock().unwrap();
            let _ = s.record_stream_error();
            assert_eq!(s.playback_state, PlaybackState::Stopped);
            assert!(s.stream_error_reported);
        }

        // The fix prepares state BEFORE stream.play() is called. After
        // preparation, the backend must already look like a fresh Playing
        // lifecycle so an error callback firing immediately after play()
        // observes a correct starting state.
        let prev = backend.prepare_for_stream_start();

        assert_eq!(
            prev,
            PlaybackState::Stopped,
            "prepare_for_stream_start should return the previous state for revert"
        );
        {
            let s = backend.state.lock().unwrap();
            assert_eq!(
                s.playback_state,
                PlaybackState::Playing,
                "State must be Playing before play() so the lifecycle is correct"
            );
            assert!(
                !s.stream_error_reported,
                "Error flag must be cleared before play() so the next error logs once"
            );
        }
    }

    #[test]
    fn revert_failed_stream_start_restores_previous_state_and_clears_flag() {
        // Simulate a failed play(): the speculative Playing state must be
        // reverted so the backend does not advertise a stream that is not
        // running, and the error flag must be clean for the next attempt.
        let backend = CpalBackend::new();

        // Start from a concrete prior state to verify revert is faithful.
        {
            let mut s = backend.state.lock().unwrap();
            s.playback_state = PlaybackState::Paused;
        }

        let prev = backend.prepare_for_stream_start();
        assert_eq!(prev, PlaybackState::Paused);

        // Simulate play() failing.
        backend.revert_failed_stream_start(prev);

        {
            let s = backend.state.lock().unwrap();
            assert_eq!(
                s.playback_state,
                PlaybackState::Paused,
                "Failed play() must revert to the prior state, not stay Playing"
            );
            assert!(
                !s.stream_error_reported,
                "Failed play() must leave the error flag clean for the next attempt"
            );
            assert!(
                s.pcm_buffer.is_empty(),
                "Failed play() must drop any buffered PCM so it is not served stale"
            );
        }
    }

    #[test]
    fn error_callback_after_prepare_is_not_overwritten_by_late_playing_set() {
        // This is the DIRECT regression for the race class.
        //
        // Sequence the bug allowed:
        //   1. stream.play()
        //   2. error callback fires -> record_stream_error() -> Stopped, flag=true
        //   3. start_stream sets Playing + flag=false  (OVERWRITES the callback)
        //
        // Fixed sequence:
        //   1. prepare_for_stream_start() -> Playing, flag=false
        //   2. stream.play()
        //   3. error callback fires -> record_stream_error() -> Stopped, flag=true
        //   4. NO late "set Playing" step exists to clobber it.
        //
        // We simulate steps 1, 3 and then assert that no further state write
        // occurs — i.e. the Stopped transition from the callback survives.
        let backend = CpalBackend::new();

        // Step 1 (fix): prepare state before play.
        let _prev = backend.prepare_for_stream_start();
        {
            let s = backend.state.lock().unwrap();
            assert_eq!(s.playback_state, PlaybackState::Playing);
            assert!(!s.stream_error_reported);
        }

        // Step 2: simulate stream.play() succeeding (no-op for state).

        // Step 3: simulate the error callback firing immediately after play.
        let should_log = {
            let mut s = backend.state.lock().unwrap();
            s.record_stream_error()
        };
        assert!(
            should_log,
            "First error of the fresh lifecycle must be logged"
        );

        // Step 4: there is NO late "set Playing + clear flag" step in the
        // fixed code. Assert the callback's transition survives.
        {
            let s = backend.state.lock().unwrap();
            assert_eq!(
                s.playback_state,
                PlaybackState::Stopped,
                "Error callback's Stopped transition must NOT be overwritten by a late Playing set"
            );
            assert!(
                s.stream_error_reported,
                "Error flag must stay true so repeated callbacks do not re-log the same lifecycle"
            );
        }

        // A second callback for the same lifecycle must be deduped (not logged).
        let should_log_again = {
            let mut s = backend.state.lock().unwrap();
            s.record_stream_error()
        };
        assert!(
            !should_log_again,
            "Second callback in the same lifecycle must be deduped, proving the flag was not clobbered"
        );
    }

    // === Behavior-level coverage for the CPAL callback contract ===
    //
    // These tests exercise the real backend contract for stream errors and
    // stale audio through the extracted `render_callback` / `handle_stream_error`
    // helpers — the exact bodies the cpal data and error closures route through
    // in production. They do NOT require audio hardware: they feed a real PcmBus
    // with PCM frames and assert observable effects on the output buffer and
    // `AudioBackend::state()`, which is the contract the UI and Tauri commands
    // rely on.
    //
    // The contract under test:
    //   1. A stream error stops playback: `state()` reports `Stopped`.
    //   2. After an error, the data callback emits silence and does not drain
    //      stale buffered PCM into the output, preventing stale audio.
    //   3. Repeated error callbacks for the same stream lifecycle do not log
    //      again (dedup) and keep the backend in `Stopped`.
    //   4. A healthy lifecycle consumes PCM from the bus and writes volume-scaled
    //      samples to the output buffer, advancing the position.

    /// Build a backend wired to a real PcmBus so behavior tests can pump PCM
    /// frames through the same path the cpal data callback uses.
    fn backend_with_bus(sample_rate: u32, channels: u16) -> (CpalBackend, crate::audio::pipeline::PcmBusProducer) {
        let (producer, subscriber) = crate::audio::pipeline::PcmBus::new(sample_rate, channels);
        let backend = CpalBackend::new();
        backend.set_subscriber(subscriber);
        (backend, producer)
    }

    #[test]
    fn data_callback_renders_pcm_from_bus_into_output_buffer() {
        // Behavior: a healthy Playing stream consumes PCM from the bus, writes
        // volume-scaled, channel-converted samples into the output buffer, and
        // advances the position. This is the "happy path" the UI relies on for
        // audible playback.
        let (backend, producer) = backend_with_bus(44100, 2);

        // Prime the lifecycle (mirrors prepare_for_stream_start).
        {
            let mut s = backend.state.lock().unwrap();
            s.playback_state = PlaybackState::Playing;
        }

        // Push 2 stereo frames (4 samples) and render one callback tick into a
        // 4-sample stereo output buffer at volume 1.0 (default).
        producer.send(vec![0.1_f32, 0.2, 0.3, 0.4]).unwrap();
        let mut out = vec![-1.0_f32; 4];
        CpalBackend::render_callback(
            &backend.state,
            &backend.subscriber,
            &mut out,
            2,
            2,
            44100,
        );

        assert_eq!(
            out,
            vec![0.1_f32, 0.2, 0.3, 0.4],
            "Healthy stream must render PCM frames into the output buffer unchanged at volume 1.0"
        );

        let s = backend.state.lock().unwrap();
        assert!(
            s.pcm_buffer.is_empty(),
            "Rendered frames must be drained from the internal PCM buffer"
        );
        assert!(
            s.position > 0.0,
            "Position must advance by the consumed sample duration"
        );
        assert_eq!(
            s.playback_state,
            PlaybackState::Playing,
            "Healthy render must not change playback state"
        );
    }

    #[test]
    fn data_callback_applies_user_volume_to_rendered_output() {
        // Behavior: the data callback honors `set_volume` and scales the output.
        let (backend, producer) = backend_with_bus(44100, 2);

        {
            let mut s = backend.state.lock().unwrap();
            s.playback_state = PlaybackState::Playing;
            s.volume = 0.5;
        }

        producer.send(vec![1.0_f32, 0.5, 1.0, 0.5]).unwrap();
        let mut out = vec![0.0_f32; 4];
        CpalBackend::render_callback(
            &backend.state,
            &backend.subscriber,
            &mut out,
            2,
            2,
            44100,
        );

        assert_eq!(
            out,
            vec![0.5_f32, 0.25, 0.5, 0.25],
            "Rendered output must be scaled by the user volume"
        );
    }

    #[test]
    fn stream_error_callback_stops_playback_observable_via_state() {
        // Behavior (the core blocker): a CPAL stream error must stop playback so
        // the observable `AudioBackend::state()` reports `Stopped`. The UI uses
        // this to flip the transport button back to "play"; a stale `Playing`
        // here would leave the user unable to restart playback.
        let (backend, _producer) = backend_with_bus(44100, 2);
        {
            let mut s = backend.state.lock().unwrap();
            s.playback_state = PlaybackState::Playing;
        }
        assert_eq!(backend.state(), PlaybackState::Playing);

        // First error callback: must log and stop playback.
        let should_log = CpalBackend::handle_stream_error(&backend.state);
        assert!(
            should_log,
            "First error of the lifecycle must be logged exactly once"
        );
        assert_eq!(
            backend.state(),
            PlaybackState::Stopped,
            "After a stream error the backend must report Stopped via the public trait API"
        );
    }

    #[test]
    fn repeated_stream_error_callbacks_are_deduped_and_keep_stopped() {
        // Behavior: CPAL may invoke the error callback repeatedly for the same
        // failure (device invalidated, underrun). Each subsequent invocation
        // must stay silent AND keep the backend in `Stopped` — no log spam, no
        // state thrash.
        let (backend, _producer) = backend_with_bus(44100, 2);
        {
            let mut s = backend.state.lock().unwrap();
            s.playback_state = PlaybackState::Playing;
        }

        let first = CpalBackend::handle_stream_error(&backend.state);
        let second = CpalBackend::handle_stream_error(&backend.state);
        let third = CpalBackend::handle_stream_error(&backend.state);

        assert!(first, "First error must be logged");
        assert!(
            !second && !third,
            "Repeated error callbacks for the same lifecycle must NOT be logged"
        );
        assert_eq!(
            backend.state(),
            PlaybackState::Stopped,
            "Repeated errors must keep the backend Stopped"
        );
    }

    #[test]
    fn data_callback_after_error_emits_silence_and_does_not_serve_stale_pcm() {
        // Behavior (the stale-audio half of the blocker): once a stream error
        // has stopped playback, the data callback must emit SILENCE and must NOT
        // drain the PcmBus or serve buffered PCM. Without this, a stopped/error
        // stream could keep emitting stale audio until the stream handle is
        // dropped — exactly the regression the reviewer flagged.
        let (backend, producer) = backend_with_bus(44100, 2);

        // Start Playing and prime the bus with real PCM frames.
        {
            let mut s = backend.state.lock().unwrap();
            s.playback_state = PlaybackState::Playing;
        }
        producer.send(vec![0.5_f32, 0.5, 0.5, 0.5]).unwrap();

        // Now the stream errors out and stops playback.
        let logged = CpalBackend::handle_stream_error(&backend.state);
        assert!(logged);
        assert_eq!(backend.state(), PlaybackState::Stopped);

        // Push MORE frames after the error — these must never be served.
        producer.send(vec![0.9_f32, 0.9, 0.9, 0.9]).unwrap();

        // Render a callback tick into a buffer pre-filled with garbage so we can
        // detect silence vs passthrough.
        let mut out = vec![-1.0_f32; 4];
        CpalBackend::render_callback(
            &backend.state,
            &backend.subscriber,
            &mut out,
            2,
            2,
            44100,
        );

        assert_eq!(
            out,
            vec![0.0_f32; 4],
            "After a stream error the data callback must emit silence, not stale PCM"
        );

        // The internal PCM buffer must also be cleared by the error callback, so
        // a later resume cannot emit the pre-error frames.
        let s = backend.state.lock().unwrap();
        assert!(
            s.pcm_buffer.is_empty(),
            "Stream error must clear the PCM buffer to prevent stale audio on resume"
        );
    }

    #[test]
    fn data_callback_after_error_does_not_drain_new_frames_from_bus() {
        // Behavior corollary: a stopped stream must not silently consume frames
        // from the bus either — otherwise the decoder keeps producing, the
        // subscriber channel keeps filling, and frames pile up / get dropped
        // while the user hears nothing. The stopped path must be a true no-op
        // on the bus.
        let (backend, producer) = backend_with_bus(44100, 2);
        {
            let mut s = backend.state.lock().unwrap();
            s.playback_state = PlaybackState::Playing;
        }
        // Stop the stream via the error callback.
        CpalBackend::handle_stream_error(&backend.state);
        assert_eq!(backend.state(), PlaybackState::Stopped);

        // Push frames after the stop.
        producer.send(vec![0.7_f32, 0.7, 0.7, 0.7]).unwrap();

        let mut out = vec![0.0_f32; 4];
        CpalBackend::render_callback(
            &backend.state,
            &backend.subscriber,
            &mut out,
            2,
            2,
            44100,
        );
        assert_eq!(out, vec![0.0_f32; 4], "Stopped stream must emit silence");

        // The frame pushed AFTER the stop must still be sitting in the bus,
        // untouched, because the stopped render path returns before reading.
        let sub_guard = backend.subscriber.lock().unwrap();
        if let Some(ref sub) = *sub_guard {
            assert!(
                sub.try_recv().is_some(),
                "Stopped stream must not drain frames from the bus; the post-stop frame must still be present"
            );
        }
    }

    #[test]
    fn paused_render_consumes_frames_but_preserves_position() {
        // Behavior boundary: Paused is NOT Stopped. The data callback still
        // consumes PCM from the bus and renders it (so the buffer does not
        // back up), but the position estimate must NOT advance — the UI shows
        // the paused position, not a drifting one.
        let (backend, producer) = backend_with_bus(44100, 2);
        {
            let mut s = backend.state.lock().unwrap();
            s.playback_state = PlaybackState::Paused;
            s.position = 10.0;
        }
        producer.send(vec![0.3_f32, 0.3, 0.3, 0.3]).unwrap();
        let mut out = vec![-1.0_f32; 4];
        CpalBackend::render_callback(
            &backend.state,
            &backend.subscriber,
            &mut out,
            2,
            2,
            44100,
        );
        assert_eq!(out, vec![0.3_f32; 4], "Paused consumes frames but does not mute");
        let s = backend.state.lock().unwrap();
        assert!(
            (s.position - 10.0).abs() < f64::EPSILON,
            "Paused render must NOT advance the position"
        );
        assert_eq!(s.playback_state, PlaybackState::Paused);
    }

    #[test]
    fn stopped_render_emits_silence_without_advancing_position() {
        // Behavior boundary: Stopped (e.g. post-error) must emit silence and
        // must not advance the position. This is the counterpart to the Paused
        // test above: both produce no audible progress, but Stopped is terminal
        // for the lifecycle while Paused is a hold.
        let (backend, producer) = backend_with_bus(44100, 2);
        {
            let mut s = backend.state.lock().unwrap();
            s.playback_state = PlaybackState::Stopped;
            s.position = 7.0;
        }
        producer.send(vec![0.8_f32, 0.8, 0.8, 0.8]).unwrap();
        let mut out = vec![-1.0_f32; 4];
        CpalBackend::render_callback(
            &backend.state,
            &backend.subscriber,
            &mut out,
            2,
            2,
            44100,
        );
        assert_eq!(out, vec![0.0_f32; 4], "Stopped must emit silence");
        let s = backend.state.lock().unwrap();
        assert!(
            (s.position - 7.0).abs() < f64::EPSILON,
            "Stopped render must not advance position"
        );
        assert_eq!(s.playback_state, PlaybackState::Stopped);
    }

    #[test]
    fn fresh_lifecycle_after_stop_logs_error_again() {
        // Behavior: after the backend is stopped and a fresh stream starts,
        // the next error is a NEW lifecycle and must be logged again (not
        // permanently deduped). This protects against the regression where the
        // dedup flag was never reset and the second genuine failure went silent.
        let (backend, _producer) = backend_with_bus(44100, 2);
        {
            let mut s = backend.state.lock().unwrap();
            s.playback_state = PlaybackState::Playing;
        }

        // First lifecycle: error -> log + Stopped.
        let first = CpalBackend::handle_stream_error(&backend.state);
        assert!(first);
        assert_eq!(backend.state(), PlaybackState::Stopped);

        // A fresh stream resets the lifecycle (prepare_for_stream_start).
        let prev = backend.prepare_for_stream_start();
        assert_eq!(prev, PlaybackState::Stopped);
        assert_eq!(backend.state(), PlaybackState::Playing);

        // Second lifecycle: a new error must log again, not be silently
        // swallowed by the previous lifecycle's dedup flag.
        let second = CpalBackend::handle_stream_error(&backend.state);
        assert!(
            second,
            "A fresh lifecycle must log the first error again — dedup is per-lifecycle, not global"
        );
        assert_eq!(backend.state(), PlaybackState::Stopped);
    }
}
