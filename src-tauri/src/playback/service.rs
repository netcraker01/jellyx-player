//! Playback service — single entry point for all playback operations.
//!
//! `PlaybackService` orchestrates the full audio pipeline:
//! decoder thread → PcmBus → CpalBackend + FftEngine.
//!
//! It owns the pipeline lifecycle and emits events to the frontend.
//! Commands delegate here — never directly to AudioBackend.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::audio::decoder::SymphoniaDecoder;
use crate::audio::fft::FftEngine;
use crate::audio::output::CpalBackend;
use crate::audio::pipeline::PcmBus;
use crate::audio::{AudioBackend, PlaybackState};
use crate::errors::types::{AppError, PlaybackError, ValidationError};
use crate::models::track::Track;
use crate::playback::events::PlaybackEventEmitter;
use crate::playback::state::QueueState;
use crate::sources::SourceResolver;
use crate::visualizer::fft_bridge::FftBridge;

/// How often (in ms) progress-tick events are emitted during playback.
const PROGRESS_TICK_INTERVAL_MS: u64 = 250;

/// Facade that owns audio backend, queue, FFT bridge, and event emitter.
///
/// All IPC commands go through this service. It manages the pipeline
/// lifecycle: decoder thread, PCM bus, FFT engine, and progress timer.
pub struct PlaybackService {
    /// The current playback state shared across threads.
    state: Arc<Mutex<InternalState>>,
    /// Event emitter for Tauri frontend notifications.
    emitter: PlaybackEventEmitter,
}

/// Internal state protected by the Mutex.
struct InternalState {
    /// Current playback state (Stopped/Playing/Paused/Buffering).
    playback_state: PlaybackState,
    /// Current track being played.
    current_track: Option<Track>,
    /// Queue state (tracks, current index).
    queue: QueueState,
    /// Volume level (0.0..=1.0).
    volume: f32,
    /// Position in seconds.
    position: f64,
    /// Duration of the current track in seconds.
    duration: f64,
}

impl PlaybackService {
    /// Create a new PlaybackService.
    ///
    /// The `app` handle is used for emitting events to the frontend.
    /// The actual audio backend (CpalBackend) is created internally
    /// when `play_local()` is called, not at construction time.
    pub fn new(app: tauri::AppHandle) -> Self {
        Self {
            state: Arc::new(Mutex::new(InternalState {
                playback_state: PlaybackState::Stopped,
                current_track: None,
                queue: QueueState::default(),
                volume: 1.0,
                position: 0.0,
                duration: 0.0,
            })),
            emitter: PlaybackEventEmitter::new(app),
        }
    }

    /// Play a local audio file.
    ///
    /// This is the primary playback method for v0.1. It:
    /// 1. Opens the file with SymphoniaDecoder
    /// 2. Creates a PcmBus
    /// 3. Starts a decoder thread that pushes frames to the bus
    /// 4. Creates a CpalBackend and connects it as a subscriber
    /// 5. Creates an FftEngine and connects it as a subscriber
    /// 6. Starts the cpal audio stream
    /// 7. Starts the FFT analysis timer
    /// 8. Starts the progress tick timer
    pub fn play_local(&self, path: &str) -> Result<(), AppError> {
        // Stop any currently playing audio first
        self.stop()?;

        // Open the decoder to get stream info
        let mut decoder = SymphoniaDecoder::open(path)
            .map_err(|e| AppError::from(e))?;

        let sample_rate = decoder.sample_rate();
        let channels = decoder.channels();
        let duration = decoder.duration();

        // Create the PCM bus
        let (mut bus_producer, output_subscriber) = PcmBus::new(sample_rate, channels);

        // Subscribe the FFT engine
        let fft_subscriber = bus_producer.subscribe();

        // Create a track object for the event
        let track_name = PathBuf::from(path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Unknown")
            .to_string();

        let track = Track {
            id: format!("local-{}", path.len()),
            source: crate::models::source::Source::Local,
            source_id: path.to_string(),
            title: track_name,
            artist: String::new(),
            album: None,
            duration: Some(duration),
            thumbnail: None,
            stream_url: None,
            local_path: Some(path.to_string()),
            metadata: std::collections::HashMap::new(),
        };

        // Update state
        {
            let mut s = self.state.lock().map_err(|_| AppError {
                code: "UNKNOWN_ERROR".into(),
                details: Some("mutex lock".into()),
            })?;
            s.playback_state = PlaybackState::Buffering;
            s.current_track = Some(track.clone());
            s.position = 0.0;
            s.duration = duration;
        }

        // Emit events
        let _ = self.emitter.emit_track_changed(&track);
        let _ = self.emitter.emit_state_changed(&PlaybackState::Buffering);

        // Spawn decoder thread
        let decoder_state = self.state.clone();
        let channels_f64 = channels as f64;
        let sample_rate_f64 = sample_rate as f64;
        let _decoder_handle = thread::spawn(move || {
            let mut buf = vec![0.0f32; 4096];
            loop {
                // Check if we should stop
                {
                    let s = decoder_state.lock().unwrap();
                    if s.playback_state == PlaybackState::Stopped {
                        break;
                    }
                }

                match decoder.decode_next(&mut buf) {
                    Ok(0) => {
                        // End of stream — set state to Stopped
                        let mut s = decoder_state.lock().unwrap();
                        s.playback_state = PlaybackState::Stopped;
                        break;
                    }
                    Ok(samples_read) => {
                        // Update position estimate: samples / (channels * sample_rate) = seconds
                        let seconds_advanced = samples_read as f64 / (channels_f64 * sample_rate_f64);
                        {
                            let mut s = decoder_state.lock().unwrap();
                            s.position += seconds_advanced;
                        }

                        let frame = buf[..samples_read].to_vec();
                        if bus_producer.send(frame).is_err() {
                            // Bus is closed — stop decoding
                            break;
                        }
                    }
                    Err(_) => {
                        // Decode error — stop
                        let mut s = decoder_state.lock().unwrap();
                        s.playback_state = PlaybackState::Stopped;
                        break;
                    }
                }
            }
        });

        // Create and configure the audio backend
        let mut cpal_backend = CpalBackend::new();
        cpal_backend.set_subscriber(output_subscriber);
        cpal_backend.play_local(&PathBuf::from(path))
            .map_err(|e| AppError::from(e))?;

        // Update state to Playing
        {
            let mut s = self.state.lock().map_err(|_| AppError {
                code: "UNKNOWN_ERROR".into(),
                details: Some("mutex lock".into()),
            })?;
            s.playback_state = PlaybackState::Playing;
        }
        let _ = self.emitter.emit_state_changed(&PlaybackState::Playing);

        // Start FFT analysis timer
        let fft_app = self.emitter.app_handle();
        let fft_engine_state = self.state.clone();
        let fft_sample_rate = sample_rate;
        thread::spawn(move || {
            let fft_bridge = FftBridge::new(fft_app);
            let mut fft_engine = FftEngine::new(1024, fft_subscriber, fft_sample_rate);

            loop {
                // Check if we should stop
                {
                    let s = fft_engine_state.lock().unwrap();
                    if s.playback_state == PlaybackState::Stopped {
                        break;
                    }
                }

                // Collect frames and analyze
                fft_engine.collect_frames();
                if let Some(freq_data) = fft_engine.analyze_if_ready() {
                    let _ = fft_bridge.emit_frequency_data(&freq_data);
                }

                // Sleep to avoid busy-looping (~60Hz for visualization)
                thread::sleep(Duration::from_millis(16));
            }
        });

        // Start progress tick timer
        self.start_progress_tick_timer();

        Ok(())
    }

    /// Start playing a URL. Reserved for future use (streaming).
    ///
    /// For v0.1, this returns `PlatformNotSupported`.
    pub fn play(&self, _url: &str) -> Result<(), AppError> {
        Err(AppError::from(crate::audio::AudioError::PlatformNotSupported))
    }

    /// Pause playback. Emits state_changed.
    pub fn pause(&self) -> Result<(), AppError> {
        {
            let mut s = self.state.lock().map_err(|_| AppError {
                code: "UNKNOWN_ERROR".into(),
                details: Some("mutex lock".into()),
            })?;
            if s.playback_state != PlaybackState::Playing {
                return Err(AppError::from(PlaybackError::NoCurrentTrack));
            }
            s.playback_state = PlaybackState::Paused;
        }
        let _ = self.emitter.emit_state_changed(&PlaybackState::Paused);
        Ok(())
    }

    /// Resume playback. Emits state_changed.
    pub fn resume(&self) -> Result<(), AppError> {
        {
            let mut s = self.state.lock().map_err(|_| AppError {
                code: "UNKNOWN_ERROR".into(),
                details: Some("mutex lock".into()),
            })?;
            if s.playback_state != PlaybackState::Paused {
                return Err(AppError::from(PlaybackError::NoCurrentTrack));
            }
            s.playback_state = PlaybackState::Playing;
        }
        let _ = self.emitter.emit_state_changed(&PlaybackState::Playing);
        Ok(())
    }

    /// Stop playback and clean up the pipeline.
    ///
    /// Sets state to Stopped, drops pipeline handles (which terminates
    /// the decoder thread and closes PCM bus channels), and emits state_changed.
    pub fn stop(&self) -> Result<(), AppError> {
        {
            let mut s = self.state.lock().map_err(|_| AppError {
                code: "UNKNOWN_ERROR".into(),
                details: Some("mutex lock".into()),
            })?;
            s.playback_state = PlaybackState::Stopped;
            s.position = 0.0;
        }
        let _ = self.emitter.emit_state_changed(&PlaybackState::Stopped);
        Ok(())
    }

    /// Seek to a position in the current track (seconds).
    ///
    /// For v0.1, this updates the position estimate. A full seek
    /// implementation would restart the decoder from the new position.
    pub fn seek(&self, position: f64) -> Result<(), AppError> {
        let mut s = self.state.lock().map_err(|_| AppError {
            code: "UNKNOWN_ERROR".into(),
            details: Some("mutex lock".into()),
        })?;
        s.position = position.clamp(0.0, s.duration);
        Ok(())
    }

    /// Set the playback volume (0.0 to 1.0).
    pub fn set_volume(&self, level: f32) -> Result<(), AppError> {
        let mut s = self.state.lock().map_err(|_| AppError {
            code: "UNKNOWN_ERROR".into(),
            details: Some("mutex lock".into()),
        })?;
        s.volume = level.clamp(0.0, 1.0);
        Ok(())
    }

    /// Get the current playback state.
    pub fn state(&self) -> PlaybackState {
        self.state.lock().unwrap().playback_state.clone()
    }

    /// Get the current position in seconds.
    pub fn position(&self) -> f64 {
        self.state.lock().unwrap().position
    }

    /// Get the duration of the current track in seconds.
    pub fn duration(&self) -> f64 {
        self.state.lock().unwrap().duration
    }

    /// Get the current volume level (0.0 to 1.0).
    pub fn volume(&self) -> f32 {
        self.state.lock().unwrap().volume
    }

    /// Search for tracks. Currently hardcodes YouTubeResolver for v0.1.
    pub fn search(&self, query: &str) -> Result<Vec<Track>, AppError> {
        if query.trim().is_empty() {
            return Err(ValidationError::EmptyQuery.into());
        }
        let resolver = crate::sources::youtube::YouTubeResolver::new();
        resolver.search(query).map_err(Into::into)
    }

    /// Add a track to the queue by ID. Emits queue_updated.
    pub fn add_to_queue(&self, track_id: &str) -> Result<(), AppError> {
        if track_id.trim().is_empty() {
            return Err(ValidationError::InvalidInput("track_id must not be empty".into()).into());
        }

        let resolver = crate::sources::youtube::YouTubeResolver::new();
        let track = resolver.resolve(track_id)?;

        let mut queue = self.state.lock().map_err(|_| AppError {
            code: "UNKNOWN_ERROR".into(),
            details: Some("mutex lock".into()),
        })?;

        queue.queue.tracks.push(track.clone());
        if queue.queue.current_index.is_none() && queue.queue.tracks.len() == 1 {
            queue.queue.current_index = Some(0);
        }

        let tracks_snapshot = queue.queue.tracks.clone();
        drop(queue);

        let _ = self.emitter.emit_queue_updated(&tracks_snapshot);
        Ok(())
    }

    /// Get the current queue as a list of tracks.
    pub fn get_queue(&self) -> Result<Vec<Track>, AppError> {
        let s = self.state.lock().map_err(|_| AppError {
            code: "UNKNOWN_ERROR".into(),
            details: Some("mutex lock".into()),
        })?;
        Ok(s.queue.tracks.clone())
    }

    /// Skip to the next track in the queue.
    pub fn next(&self) -> Result<(), AppError> {
        let mut s = self.state.lock().map_err(|_| AppError {
            code: "UNKNOWN_ERROR".into(),
            details: Some("mutex lock".into()),
        })?;

        if s.queue.tracks.is_empty() {
            return Err(PlaybackError::QueueEmpty.into());
        }

        let current = s.queue.current_index.unwrap_or(0);
        let next_index = current + 1;
        if next_index < s.queue.tracks.len() {
            s.queue.current_index = Some(next_index);
            let track = s.queue.tracks[next_index].clone();
            drop(s);

            let _ = self.emitter.emit_track_changed(&track);
            let _ = self.emitter.emit_state_changed(&PlaybackState::Playing);

            // If the track has a local_path, play it
            if let Some(ref local_path) = track.local_path {
                return self.play_local(local_path);
            }

            let mut s = self.state.lock().map_err(|_| AppError {
                code: "UNKNOWN_ERROR".into(),
                details: Some("mutex lock".into()),
            })?;
            s.current_track = Some(track);
        }

        Ok(())
    }

    /// Go to the previous track in the queue.
    pub fn previous(&self) -> Result<(), AppError> {
        let mut s = self.state.lock().map_err(|_| AppError {
            code: "UNKNOWN_ERROR".into(),
            details: Some("mutex lock".into()),
        })?;

        if s.queue.tracks.is_empty() {
            return Err(PlaybackError::QueueEmpty.into());
        }

        let current = s.queue.current_index.unwrap_or(0);
        if current > 0 {
            s.queue.current_index = Some(current - 1);
            let track = s.queue.tracks[current - 1].clone();
            drop(s);

            let _ = self.emitter.emit_track_changed(&track);
            let _ = self.emitter.emit_state_changed(&PlaybackState::Playing);

            if let Some(ref local_path) = track.local_path {
                return self.play_local(local_path);
            }

            let mut s = self.state.lock().map_err(|_| AppError {
                code: "UNKNOWN_ERROR".into(),
                details: Some("mutex lock".into()),
            })?;
            s.current_track = Some(track);
        }

        Ok(())
    }

    /// Start a background timer that emits progress-tick events.
    ///
    /// Emits at ~4Hz (every 250ms) during playback. The timer stops
    /// when the state changes to Stopped.
    fn start_progress_tick_timer(&self) {
        let state = self.state.clone();
        let emitter = self.emitter.clone_sender();

        thread::spawn(move || {
            loop {
                thread::sleep(Duration::from_millis(PROGRESS_TICK_INTERVAL_MS));

                let s = state.lock().unwrap();
                if s.playback_state == PlaybackState::Stopped {
                    break;
                }

                let position = s.position;
                let duration = s.duration;
                drop(s);

                let _ = emitter.emit_progress_tick(position, duration);
            }
        });
    }
}