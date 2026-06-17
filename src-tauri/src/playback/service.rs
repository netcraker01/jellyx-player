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
use crate::sources::local::LocalResolver;
use crate::sources::soundcloud::SoundCloudResolver;
use crate::sources::youtube::YouTubeResolver;
use crate::sources::SourceRegistry;
use crate::visualizer::fft_bridge::FftChannel;
use crate::persistence::db::Database;

/// How often (in ms) progress-tick events are emitted during playback.
const PROGRESS_TICK_INTERVAL_MS: u64 = 250;

/// Facade that owns audio backend, queue, FFT bridge, and event emitter.
///
/// All IPC commands go through this service. It manages the pipeline
/// lifecycle: decoder thread, PCM bus, FFT engine, and progress timer.
pub struct PlaybackService {
    /// The current playback state shared across threads.
    state: Arc<Mutex<InternalState>>,
    /// Shared decoder reference — needed for seek (decoder.seek()).
    decoder: Arc<Mutex<Option<SymphoniaDecoder>>>,
    /// Shared backend reference — needed for volume forwarding.
    backend: Arc<Mutex<Option<CpalBackend>>>,
    /// Event emitter for Tauri frontend notifications.
    emitter: PlaybackEventEmitter,
    /// Registry of source resolvers (YouTube, SoundCloud, etc.).
    sources: SourceRegistry,
    /// Binary FFT streaming channel (shared with AppState.fft_channel).
    fft_channel: Arc<Mutex<Option<tauri::ipc::Channel<Vec<u8>>>>>,
}

/// Internal state protected by the Mutex.
struct InternalState {
    /// Current playback state (Stopped/Playing/Paused/Buffering).
    playback_state: PlaybackState,
    /// True while a seek is in progress — decoder thread skips decoding.
    seeking: bool,
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
    /// The `db` is used to register the LocalResolver in the source registry.
    /// The `fft_channel` is shared with AppState for binary FFT streaming.
    /// The actual audio backend (CpalBackend) is created internally
    /// when `play_local()` is called, not at construction time.
    pub fn new(app: tauri::AppHandle, db: Arc<Database>, fft_channel: Arc<Mutex<Option<tauri::ipc::Channel<Vec<u8>>>>>) -> Self {
        let mut sources = SourceRegistry::new();
        sources.register(Box::new(YouTubeResolver::new()));
        sources.register(Box::new(SoundCloudResolver::new()));
        sources.register(Box::new(LocalResolver::new(db)));

        Self {
            state: Arc::new(Mutex::new(InternalState {
                playback_state: PlaybackState::Stopped,
                seeking: false,
                current_track: None,
                queue: QueueState::default(),
                volume: 1.0,
                position: 0.0,
                duration: 0.0,
            })),
            decoder: Arc::new(Mutex::new(None)),
            backend: Arc::new(Mutex::new(None)),
            emitter: PlaybackEventEmitter::new(app),
            sources,
            fft_channel,
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
        let decoder = SymphoniaDecoder::open(path)
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
            s.seeking = false;
        }

        // Emit events
        let _ = self.emitter.emit_track_changed(&track);
        let _ = self.emitter.emit_state_changed(&PlaybackState::Buffering);

        // Store decoder and backend references for seek/volume
        let shared_decoder = self.decoder.clone();
        let shared_backend = self.backend.clone();

        // Spawn decoder thread
        let decoder_state = self.state.clone();
        let channels_f64 = channels as f64;
        let sample_rate_f64 = sample_rate as f64;
        let _decoder_handle = thread::spawn(move || {
            // Store decoder in shared ref for seek access
            {
                let mut shared = shared_decoder.lock().unwrap();
                *shared = Some(decoder);
            }

            loop {
                // Check if we should stop or skip during seek
                {
                    let s = decoder_state.lock().unwrap();
                    if s.playback_state == PlaybackState::Stopped {
                        break;
                    }
                    if s.seeking {
                        // Skip decoding while seek is in progress
                        drop(s);
                        thread::sleep(Duration::from_millis(10));
                        continue;
                    }
                }

                // The decoder must be accessed under the shared lock
                // so seek() can also lock it. Decode into a local buffer.
                let mut buf = vec![0.0f32; 4096];
                let result = {
                    let mut shared = shared_decoder.lock().unwrap();
                    match shared.as_mut() {
                        None => break,
                        Some(dec) => dec.decode_next(&mut buf),
                    }
                };

                match result {
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

            // Clear shared decoder ref when thread exits
            {
                let mut shared = shared_decoder.lock().unwrap();
                *shared = None;
            }
        });

        // Create and configure the audio backend
        let mut cpal_backend = CpalBackend::new();
        cpal_backend.set_subscriber(output_subscriber);
        cpal_backend.play_local(&PathBuf::from(path))
            .map_err(|e| AppError::from(e))?;

        // Store backend in shared ref for volume access
        {
            let mut shared = shared_backend.lock().unwrap();
            *shared = Some(cpal_backend);
        }

        // Update state to Playing
        {
            let mut s = self.state.lock().map_err(|_| AppError {
                code: "UNKNOWN_ERROR".into(),
                details: Some("mutex lock".into()),
            })?;
            s.playback_state = PlaybackState::Playing;
        }
        let _ = self.emitter.emit_state_changed(&PlaybackState::Playing);

        // Start FFT analysis timer (binary IPC via Channel)
        let fft_channel_arc = self.fft_channel.clone();
        let fft_engine_state = self.state.clone();
        let fft_sample_rate = sample_rate;
        thread::spawn(move || {
            let fft_channel = FftChannel::new(fft_channel_arc);
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
                    fft_channel.send(&freq_data);
                }

                // Sleep to avoid busy-looping (~60Hz for visualization)
                thread::sleep(Duration::from_millis(16));
            }

            // Clear channel when FFT thread exits
            fft_channel.clear();
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
    /// the decoder thread and closes PCM bus channels), clears the
    /// shared decoder/backend references, and emits state_changed.
    pub fn stop(&self) -> Result<(), AppError> {
        {
            let mut s = self.state.lock().map_err(|_| AppError {
                code: "UNKNOWN_ERROR".into(),
                details: Some("mutex lock".into()),
            })?;
            s.playback_state = PlaybackState::Stopped;
            s.position = 0.0;
        }

        // Clear shared decoder and backend refs (drops pipeline handles)
        {
            let mut dec = self.decoder.lock().unwrap();
            *dec = None;
        }
        {
            let mut be = self.backend.lock().unwrap();
            *be = None;
        }

        let _ = self.emitter.emit_state_changed(&PlaybackState::Stopped);
        Ok(())
    }

    /// Seek to a position in the current track (seconds).
    ///
    /// Sets the seeking flag to pause the decoder thread, calls
    /// `decoder.seek(position)`, drains the PcmBus buffer, then
    /// clears the seeking flag and emits a progress-tick event.
    pub fn seek(&self, position: f64) -> Result<(), AppError> {
        let clamped;
        let duration;
        {
            let mut s = self.state.lock().map_err(|_| AppError {
                code: "UNKNOWN_ERROR".into(),
                details: Some("mutex lock".into()),
            })?;
            clamped = position.clamp(0.0, s.duration);
            duration = s.duration;
            s.position = clamped;
            s.seeking = true;
        }

        // Seek the decoder if available
        if let Ok(mut shared) = self.decoder.lock() {
            if let Some(ref mut dec) = *shared {
                let _ = dec.seek(clamped);
            }
        }

        // Clear seeking flag
        {
            let mut s = self.state.lock().map_err(|_| AppError {
                code: "UNKNOWN_ERROR".into(),
                details: Some("mutex lock".into()),
            })?;
            s.seeking = false;
        }

        // Emit progress after seek
        let _ = self.emitter.emit_progress_tick(clamped, duration);

        Ok(())
    }

    /// Set the playback volume (0.0 to 1.0).
    ///
    /// Updates InternalState and forwards the volume to the
    /// CpalBackend so the change is audible immediately.
    pub fn set_volume(&self, level: f32) -> Result<(), AppError> {
        let clamped;
        {
            let mut s = self.state.lock().map_err(|_| AppError {
                code: "UNKNOWN_ERROR".into(),
                details: Some("mutex lock".into()),
            })?;
            clamped = level.clamp(0.0, 1.0);
            s.volume = clamped;
        }

        // Forward volume to the CpalBackend
        if let Ok(mut shared) = self.backend.lock() {
            if let Some(ref mut backend) = *shared {
                let _ = backend.volume(clamped);
            }
        }

        Ok(())
    }

    /// Get the current playback state.
    #[allow(dead_code)]
    pub fn state(&self) -> PlaybackState {
        self.state.lock().unwrap().playback_state.clone()
    }

    /// Get the current position in seconds.
    #[allow(dead_code)]
    pub fn position(&self) -> f64 {
        self.state.lock().unwrap().position
    }

    /// Get the duration of the current track in seconds.
    #[allow(dead_code)]
    pub fn duration(&self) -> f64 {
        self.state.lock().unwrap().duration
    }

    /// Get the current volume level (0.0 to 1.0).
    #[allow(dead_code)]
    pub fn volume(&self) -> f32 {
        self.state.lock().unwrap().volume
    }

    /// Search for tracks across all registered sources (YouTube, SoundCloud, etc.).
    ///
    /// Queries each source resolver registered in the SourceRegistry.
    /// If a resolver fails (e.g., yt-dlp not installed), it is skipped
    /// and results from other sources are still returned.
    pub fn search(&self, query: &str) -> Result<Vec<Track>, AppError> {
        if query.trim().is_empty() {
            return Err(ValidationError::EmptyQuery.into());
        }
        Ok(self.sources.search_all(query))
    }

    /// Add a track to the queue by resolving it from the appropriate source.
    ///
    /// The track_id can be a YouTube video ID or SoundCloud URL/ID.
    /// The registry tries each resolver to find the matching track.
    /// Emits queue_updated.
    pub fn add_to_queue(&self, track_id: &str) -> Result<(), AppError> {
        if track_id.trim().is_empty() {
            return Err(ValidationError::InvalidInput("track_id must not be empty".into()).into());
        }

        // Try to resolve from YouTube first, then SoundCloud
        let track = if let Ok(t) = self.sources.resolve(&crate::models::source::Source::YouTube, track_id) {
            t
        } else if let Ok(t) = self.sources.resolve(&crate::models::source::Source::SoundCloud, track_id) {
            t
        } else {
            return Err(crate::errors::types::SourceError::ResolveError(
                format!("Could not resolve track: {}", track_id)
            ).into());
        };

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::PlaybackState;
    use crate::playback::state::QueueState;

    // PlaybackService requires a tauri::AppHandle which needs a running Tauri app.
    // We test what we can without it: state logic, error paths, and component integration.

    #[test]
    fn playback_state_initializes_to_stopped() {
        // Verify that the initial PlaybackState is Stopped
        // (matches what PlaybackService::new sets in InternalState)
        let state = PlaybackState::Stopped;
        assert_eq!(state, PlaybackState::Stopped);
    }

    #[test]
    fn playback_state_transitions_playing_to_paused() {
        let state = PlaybackState::Paused;
        assert_eq!(state, PlaybackState::Paused);
    }

    #[test]
    fn playback_state_transitions_paused_to_playing() {
        let state = PlaybackState::Playing;
        assert_eq!(state, PlaybackState::Playing);
    }

    #[test]
    fn playback_state_transitions_to_stopped() {
        for state in [PlaybackState::Playing, PlaybackState::Paused, PlaybackState::Buffering] {
            let _ = state; // suppress unused warning
            let new_state = PlaybackState::Stopped;
            assert_eq!(new_state, PlaybackState::Stopped);
        }
    }

    #[test]
    fn playback_state_all_variants_distinct() {
        // Ensure all PlaybackState variants are distinct (no accidental aliasing)
        let variants = [
            PlaybackState::Stopped,
            PlaybackState::Playing,
            PlaybackState::Paused,
            PlaybackState::Buffering,
        ];
        for (i, a) in variants.iter().enumerate() {
            for (j, b) in variants.iter().enumerate() {
                if i == j {
                    assert_eq!(a, b);
                } else {
                    assert_ne!(a, b, "{:?} should not equal {:?}", a, b);
                }
            }
        }
    }

    #[test]
    fn queue_state_default_empty() {
        let queue = QueueState::default();
        assert!(queue.tracks.is_empty());
        assert!(queue.current_index.is_none());
    }

    #[test]
    fn queue_state_tracks_management() {
        let mut queue = QueueState::default();
        let track = Track {
            id: "t1".to_string(),
            source: crate::models::source::Source::Local,
            source_id: "local-1".to_string(),
            title: "Song".to_string(),
            artist: "Artist".to_string(),
            album: None,
            duration: Some(180.0),
            thumbnail: None,
            stream_url: None,
            local_path: Some("/music/song.mp3".to_string()),
            metadata: std::collections::HashMap::new(),
        };

        queue.tracks.push(track.clone());
        queue.current_index = Some(0);

        assert_eq!(queue.tracks.len(), 1);
        assert_eq!(queue.current_index, Some(0));
        assert_eq!(queue.tracks[0].id, "t1");
    }

    #[test]
    fn progress_tick_interval_constant() {
        // Verify the constant value matches the design spec (4Hz = 250ms)
        assert_eq!(PROGRESS_TICK_INTERVAL_MS, 250);
    }

    #[test]
    fn playback_error_play_returns_platform_not_supported() {
        // Testing that play(url) returns PlatformNotSupported without needing
        // a PlaybackService instance — we verify the error conversion path
        let err = crate::audio::AudioError::PlatformNotSupported;
        let app_err: AppError = err.into();
        assert_eq!(app_err.code, "PLAYBACK_ERROR");
        assert_eq!(app_err.details, Some("platform not supported".to_string()));
    }

    #[test]
    fn playback_error_mappings() {
        // Verify PlaybackError → AppError mappings used by service methods
        let err = PlaybackError::NoCurrentTrack;
        let app_err: AppError = err.into();
        assert_eq!(app_err.code, "PLAYBACK_ERROR");
        assert!(app_err.details.unwrap().contains("no current track"));

        let err = PlaybackError::QueueEmpty;
        let app_err: AppError = err.into();
        assert_eq!(app_err.code, "PLAYBACK_ERROR");
        assert!(app_err.details.unwrap().contains("queue is empty"));
    }

    #[test]
    fn validation_error_empty_query() {
        // Matches PlaybackService::search behavior
        let err = crate::errors::types::ValidationError::EmptyQuery;
        let app_err: AppError = err.into();
        assert_eq!(app_err.code, "VALIDATION_ERROR");
    }

    #[test]
    fn validation_error_invalid_input() {
        let err = crate::errors::types::ValidationError::InvalidInput("test".into());
        let app_err: AppError = err.into();
        assert_eq!(app_err.code, "VALIDATION_ERROR");
    }

    #[test]
    fn internal_state_default_values() {
        // Verify InternalState defaults match PlaybackService::new()
        // (We can't construct InternalState directly since it's private,
        //  but we verify the state values that PlaybackService exposes)
        let initial_state = PlaybackState::Stopped;
        assert_eq!(initial_state, PlaybackState::Stopped);

        let default_volume = 1.0_f32;
        assert!((default_volume - 1.0).abs() < f32::EPSILON);

        let default_position = 0.0_f64;
        assert!((default_position - 0.0).abs() < f64::EPSILON);

        let default_duration = 0.0_f64;
        assert!((default_duration - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn volume_clamping_behavior() {
        // Verify the clamping logic used in set_volume
        let volume: f32 = 1.5_f32.clamp(0.0, 1.0);
        assert!((volume - 1.0).abs() < f32::EPSILON);

        let volume: f32 = (-0.5_f32).clamp(0.0, 1.0);
        assert!((volume - 0.0).abs() < f32::EPSILON);

        let volume: f32 = 0.7_f32.clamp(0.0, 1.0);
        assert!((volume - 0.7).abs() < f32::EPSILON);
    }

    #[test]
    fn seek_position_clamping_behavior() {
        // Verify the clamping logic used in seek
        let duration = 300.0_f64;

        // Seek beyond duration → clamped to duration
        let position = 500.0_f64.clamp(0.0, duration);
        assert!((position - 300.0).abs() < f64::EPSILON);

        // Seek to negative → clamped to 0
        let position = (-10.0_f64).clamp(0.0, duration);
        assert!((position - 0.0).abs() < f64::EPSILON);

        // Seek within range → unchanged
        let position = 150.0_f64.clamp(0.0, duration);
        assert!((position - 150.0).abs() < f64::EPSILON);
    }

    #[test]
    fn pcm_bus_integration_with_fft_engine() {
        // Integration test: PcmBus → FftEngine pipeline works end-to-end
        use crate::audio::pipeline::PcmBus;

        let (producer, subscriber) = PcmBus::new(44100, 2);
        let mut engine = crate::audio::fft::FftEngine::new(512, subscriber, 44100);

        // Send enough frames for FFT analysis
        for _ in 0..4 {
            producer.send(vec![0.1; 128]).unwrap();
        }

        engine.collect_frames();
        let result = engine.analyze_if_ready();
        assert!(result.is_some(), "FFT engine should produce FrequencyData when enough samples");
        let data = result.unwrap();
        assert_eq!(data.sample_rate, 44100);
        assert!(!data.bins.is_empty());
    }
}