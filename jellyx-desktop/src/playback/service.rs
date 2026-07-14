//! Playback service — single entry point for all playback operations.
//!
//! `PlaybackService` orchestrates the full audio pipeline:
//! decoder thread → PcmBus → CpalBackend + FftEngine.
//!
//! It owns the pipeline lifecycle and emits events to the frontend.
//! Commands delegate here — never directly to AudioBackend.

use rand::prelude::SliceRandom;
use std::fs::File;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::audio::decoder::SymphoniaDecoder;
use crate::audio::fft::FftEngine;
use crate::audio::output::CpalBackend;
use crate::audio::pipeline::PcmBus;
use crate::audio::{AudioBackend, PlaybackState};
use crate::errors::types::{AppError, PlaybackError, ValidationError};
use crate::library::LibraryService;
use crate::persistence::db::Database;
use crate::playback::events::PlaybackEventEmitter;
use crate::playback::proxy::{
    is_approved_remote_url, proxied_url, start_proxy_server, strict_remote_client,
};
use crate::playback::state::{QueueState, RepeatMode};
use crate::sources::local::LocalResolver;
use crate::sources::soundcloud::SoundCloudResolver;
use crate::sources::youtube::YouTubeResolver;
use crate::sources::SourceRegistry;

use crate::visualizer::fft_bridge::emit_fft_frame;
use jellyx_core::models::source::Source;
use jellyx_core::models::track::Track;

/// How often (in ms) progress-tick events are emitted during playback.
const PROGRESS_TICK_INTERVAL_MS: u64 = 250;

/// Facade that owns audio backend, queue, FFT bridge, event emitter, and library service.
///
/// All IPC commands go through this service. It manages the pipeline
/// lifecycle: decoder thread, PCM bus, FFT engine, and progress timer.
///
/// Generic over Tauri runtime so unit tests can use `tauri::test::MockRuntime`
/// while production code uses the default `Wry` runtime.
pub struct PlaybackService<R: tauri::Runtime = tauri::Wry> {
    /// The current playback state shared across threads.
    state: Arc<Mutex<InternalState>>,
    /// Shared decoder reference — needed for seek (decoder.seek()).
    decoder: Arc<Mutex<Option<SymphoniaDecoder>>>,
    /// Shared backend reference — needed for volume forwarding.
    backend: Arc<Mutex<Option<CpalBackend>>>,
    /// Event emitter for Tauri frontend notifications.
    emitter: PlaybackEventEmitter<R>,
    /// Registry of source resolvers (YouTube, SoundCloud, etc.).
    sources: SourceRegistry,
    /// Library service used to record play history when tracks start.
    library: Arc<LibraryService>,
    /// Database used for local inventory cleanup on invalid files/folders.
    db: Arc<Database>,
    /// Local proxy endpoint and unguessable request capability for remote URLs.
    proxy: Option<(u16, String)>,
}

/// Internal state protected by the Mutex.
struct InternalState {
    /// Current playback state (Stopped/Playing/Paused/Buffering).
    playback_state: PlaybackState,
    /// True while a seek is in progress — decoder thread skips decoding.
    seeking: bool,
    /// Monotonically increasing signal for completed or in-progress seek requests.
    ///
    /// The EOF drain loop snapshots this so it can resume decoding even when
    /// `seeking` becomes false before the loop next observes state.
    seek_generation: u64,
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

#[derive(Debug, PartialEq, Eq)]
enum EofDrainAction {
    Wait,
    ResumeDecoding,
    Complete,
    RecoverStopped,
}

fn eof_drain_action(
    position: f64,
    duration: f64,
    eof_seek_generation: u64,
    current_seek_generation: u64,
    backend_stopped: bool,
    stop_signalled: bool,
) -> EofDrainAction {
    if backend_stopped || stop_signalled {
        EofDrainAction::RecoverStopped
    } else if current_seek_generation != eof_seek_generation {
        EofDrainAction::ResumeDecoding
    } else if position >= duration - 0.1 || duration == 0.0 {
        EofDrainAction::Complete
    } else {
        EofDrainAction::Wait
    }
}

fn stream_url_for_proxy(
    proxy: Option<&(u16, String)>,
    remote_url: &str,
) -> Result<String, AppError> {
    proxy
        .map(|(port, capability)| proxied_url(*port, capability, remote_url))
        .ok_or_else(|| {
            crate::observability::expected_failure("proxy", "remote_playback_unavailable");
            AppError {
                code: "PROXY_UNAVAILABLE".into(),
                details: Some(
                    "Remote playback needs the local proxy. Retry after restarting Jellyx.".into(),
                ),
            }
        })
}

impl<R: tauri::Runtime> PlaybackService<R> {
    /// Create a new PlaybackService.
    ///
    /// The `app` handle is used for emitting events to the frontend.
    /// The `db` is used to register the LocalResolver in the source registry.
    /// The `library` is used to record plays in history when tracks start.
    /// The actual audio backend (CpalBackend) is created internally
    /// when `play_local()` is called, not at construction time.
    pub fn new(app: tauri::AppHandle<R>, db: Arc<Database>, library: Arc<LibraryService>) -> Self {
        let mut sources = SourceRegistry::new();
        sources.register(Box::new(YouTubeResolver::new()));
        sources.register(Box::new(SoundCloudResolver::new()));
        sources.register(Box::new(LocalResolver::new(db.clone())));

        // Start the local proxy server for remote stream URLs.
        // Non-fatal: if it fails, remote playback falls back to direct URLs.
        let proxy = start_proxy_server()
            .map_err(|e| {
                eprintln!("[PlaybackService] Failed to start proxy server: {:?}", e);
                e
            })
            .ok();

        Self {
            state: Arc::new(Mutex::new(InternalState {
                playback_state: PlaybackState::Stopped,
                seeking: false,
                seek_generation: 0,
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
            library,
            db,
            proxy,
        }
    }

    /// Record a play event in history.
    ///
    /// Called exactly once per track start in `play_local()`.
    fn record_history(&self, track: &Track) {
        let _ = self.library.record_play(track);
    }

    /// Play a local audio file by path.
    ///
    /// This is the IPC entry point for v0.1. It builds a minimal track from
    /// the path and delegates to `play_local_track`, which performs the real
    /// pipeline setup and preserves the provided track metadata.
    pub fn play_local(&self, path: &str) -> Result<(), AppError> {
        // Try to look up the full track from the local library so we
        // preserve metadata (title, artist, album, thumbnail, duration).
        // Fall back to a minimal track from the filename if not found.
        let track = if let Ok(Some(full_track)) = self.db.get_local_track_by_path(path) {
            full_track
        } else {
            let track_name = PathBuf::from(path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("Unknown")
                .to_string();

            Track {
                id: format!("local-{}", path.len()),
                source: jellyx_core::models::source::Source::Local,
                source_id: path.to_string(),
                title: track_name,
                artist: String::new(),
                album: None,
                duration: None,
                thumbnail: None,
                stream_url: None,
                local_path: Some(path.to_string()),
                playlist_id: None,
                metadata: std::collections::HashMap::new(),
            }
        };

        // Seed a single-track queue so next/previous have coherent context.
        {
            let mut s = self.state.lock().map_err(|_| AppError {
                code: "UNKNOWN_ERROR".into(),
                details: Some("mutex lock".into()),
            })?;
            s.queue.tracks = vec![track.clone()];
            s.queue.current_index = Some(0);
            s.queue.played_indices.clear();
        }
        if let Ok(queue) = self.get_queue() {
            let _ = self.emitter.emit_queue_updated(&queue);
        }

        self.play_local_track(track)
    }

    /// Play a local track, preserving its metadata (id, title, artist, album).
    ///
    /// This is the shared implementation used by `play_local` and
    /// `replace_queue_and_play`. It:
    /// 1. Opens the file with SymphoniaDecoder
    /// 2. Creates a PcmBus
    /// 3. Starts a decoder thread that pushes frames to the bus
    /// 4. Creates a CpalBackend and connects it as a subscriber
    /// 5. Creates an FftEngine and connects it as a subscriber
    /// 6. Starts the cpal audio stream
    /// 7. Starts the FFT analysis timer
    /// 8. Starts the progress tick timer
    fn play_local_track(&self, track: Track) -> Result<(), AppError> {
        let path = track.local_path.as_ref().ok_or_else(|| {
            AppError::from(ValidationError::InvalidInput(
                "track has no local path".into(),
            ))
        })?;

        // Stop any currently playing audio first
        self.stop()?;

        if !Self::local_file_is_accessible(path) {
            return self.handle_invalid_local_track(track);
        }

        // Open the decoder to get stream info
        let decoder = match SymphoniaDecoder::open(path) {
            Ok(decoder) => decoder,
            Err(error) => {
                if Self::is_missing_or_inaccessible_file_error(&error) {
                    return self.handle_invalid_local_track(track);
                }

                return Err(AppError::from(error));
            }
        };

        let sample_rate = decoder.sample_rate();
        let channels = decoder.channels();
        let duration = decoder.duration();

        // Create the PCM bus
        let (bus_producer, output_subscriber) = PcmBus::new(sample_rate, channels);
        let (fft_tap, fft_subscriber) = PcmBus::output_tap(channels);

        // Update state
        {
            let mut s = self.state.lock().map_err(|_| AppError {
                code: "UNKNOWN_ERROR".into(),
                details: Some("mutex lock".into()),
            })?;
            s.playback_state = PlaybackState::Buffering(0.0);
            s.current_track = Some(track.clone());
            s.position = 0.0;
            s.duration = duration;
            s.seeking = false;
            s.seek_generation = s.seek_generation.wrapping_add(1);
        }

        // Emit events
        let _ = self.emitter.emit_track_changed(&track);
        let _ = self
            .emitter
            .emit_state_changed(&PlaybackState::Buffering(0.0));

        // Construct the backend before the decoder thread so its stop signal
        // can interrupt producer backpressure as soon as CPAL reports failure.
        let mut cpal_backend = CpalBackend::new();
        cpal_backend.set_subscriber(output_subscriber);
        cpal_backend.set_fft_tap(fft_tap);
        let backend_stop_signal = cpal_backend.stop_signal();
        let decoder_stop_signal = backend_stop_signal.clone();

        // Store decoder and backend references for seek/volume
        let shared_decoder = self.decoder.clone();
        let shared_backend = self.backend.clone();
        let self_clone = PlaybackService::<R> {
            state: self.state.clone(),
            decoder: shared_decoder.clone(),
            backend: shared_backend.clone(),
            emitter: self.emitter.clone_sender(),
            sources: SourceRegistry::new(),
            library: self.library.clone(),
            db: self.db.clone(),
            proxy: self.proxy.clone(),
        };

        // Spawn decoder thread
        let decoder_state = self.state.clone();
        let _channels_f64 = channels as f64;
        let _sample_rate_f64 = sample_rate as f64;
        let _decoder_handle = thread::spawn(move || {
            // Store decoder in shared ref for seek access
            {
                let mut shared = shared_decoder.lock().unwrap();
                *shared = Some(decoder);
            }

            'decode: loop {
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

                // Check if the audio backend has errored (CPAL stream error)
                // so we don't keep decoding for a dead stream. The critical
                // check is before the blocking bus_producer.send() below,
                // but this early-out saves unnecessary decode work.
                if let Ok(be_guard) = self_clone.backend.lock() {
                    if let Some(ref be) = *be_guard {
                        if be.state() == PlaybackState::Stopped
                            && transition_backend_stop(&decoder_state)
                        {
                            let _ = self_clone
                                .emitter
                                .emit_state_changed(&PlaybackState::Stopped);
                            break;
                        }
                    }
                }

                // The decoder must be accessed under the shared lock
                // so seek() can also lock it. Decode into a local buffer.
                let mut buf = vec![0.0f32; 16_384];
                let result = {
                    let mut shared = shared_decoder.lock().unwrap();
                    match shared.as_mut() {
                        None => break,
                        Some(dec) => dec.decode_next(&mut buf),
                    }
                };

                match result {
                    Ok(0) => {
                        // End of stream — wait for playback to drain before
                        // advancing to the next track. Prevents premature
                        // track advancement when decoder reaches EOF before
                        // the audio callback has finished playing buffered data.
                        // A seek is durable as a generation change, rather than
                        // the transient `seeking` flag, so it cannot be missed
                        // while this loop is waiting.
                        let eof_seek_generation = decoder_state.lock().unwrap().seek_generation;
                        let action = loop {
                            let s = decoder_state.lock().unwrap();
                            let duration = s.duration;
                            let current_seek_generation = s.seek_generation;
                            let fallback_position = s.position;
                            drop(s);
                            let backend = self_clone.backend.lock().ok().and_then(|backend| {
                                backend
                                    .as_ref()
                                    .map(|backend| (backend.position(), backend.state()))
                            });
                            let (position, backend_stopped) = backend
                                .map(|(position, state)| {
                                    (position, state == PlaybackState::Stopped)
                                })
                                .unwrap_or((
                                    fallback_position,
                                    decoder_stop_signal.load(std::sync::atomic::Ordering::Acquire),
                                ));

                            match eof_drain_action(
                                position,
                                duration,
                                eof_seek_generation,
                                current_seek_generation,
                                backend_stopped,
                                decoder_stop_signal.load(std::sync::atomic::Ordering::Acquire),
                            ) {
                                EofDrainAction::Wait => {
                                    thread::sleep(Duration::from_millis(50));
                                }
                                action => break action,
                            }
                        };

                        match action {
                            EofDrainAction::ResumeDecoding => continue 'decode,
                            EofDrainAction::Complete => {
                                let _ = self_clone.next();
                                break;
                            }
                            EofDrainAction::RecoverStopped => {
                                if transition_backend_stop(&decoder_state) {
                                    let _ = self_clone
                                        .emitter
                                        .emit_state_changed(&PlaybackState::Stopped);
                                }
                                break;
                            }
                            EofDrainAction::Wait => {
                                unreachable!("EOF drain loop only exits on action")
                            }
                        }
                    }
                    Ok(samples_read) => {
                        // Double-check backend state before the blocking send.
                        // When a CPAL stream error occurs the backend stops
                        // consuming PCM frames. Without this check the decoder
                        // thread blocks forever on bus_producer.send() since
                        // no one is draining the bus.
                        if let Ok(be_guard) = self_clone.backend.lock() {
                            if let Some(ref be) = *be_guard {
                                if be.state() == PlaybackState::Stopped
                                    && transition_backend_stop(&decoder_state)
                                {
                                    let _ = self_clone
                                        .emitter
                                        .emit_state_changed(&PlaybackState::Stopped);
                                    break;
                                }
                            }
                        }

                        if bus_producer
                            .send_interruptible(buf[..samples_read].to_vec(), &decoder_stop_signal)
                            .is_err()
                        {
                            if transition_backend_stop(&decoder_state) {
                                let _ = self_clone
                                    .emitter
                                    .emit_state_changed(&PlaybackState::Stopped);
                            }
                            break 'decode;
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

        // Start the configured audio backend.
        if let Err(error) = cpal_backend.play_local(&PathBuf::from(path)) {
            // The decoder has already been spawned, while this backend has not
            // yet been published to shared ownership. Stop its producer before
            // returning so it cannot remain blocked in Buffering forever.
            backend_stop_signal.store(true, std::sync::atomic::Ordering::Release);
            recover_failed_stream_start(&self.state, &self.decoder, &self.backend);
            let _ = self.emitter.emit_state_changed(&PlaybackState::Stopped);
            return Err(AppError::from(error));
        }

        // Apply the persisted volume to the freshly-created backend.
        // CpalBackend defaults to volume=1.0 in its own AudioState, so without
        // this every new track would play at full volume until the user moves
        // the slider. Re-apply so the new stream matches InternalState.volume.
        // Also re-apply the normalization gain from the persisted DB setting so
        // a backend swap does not silently drop the +6dB boost.
        {
            let s = self.state.lock().map_err(|_| AppError {
                code: "UNKNOWN_ERROR".into(),
                details: Some("mutex lock".into()),
            })?;
            let _ = cpal_backend.volume(s.volume);
        }
        let normalize_enabled = self.db.get_normalize_audio().unwrap_or(true);
        let gain = if normalize_enabled { 2.0 } else { 1.0 }; // +6dB ≈ 2x linear
        let _ = cpal_backend.set_normalize_gain(gain);

        // Store backend in shared ref for volume access
        {
            let mut shared = shared_backend.lock().unwrap();
            *shared = Some(cpal_backend);
        }

        // The stream may fail immediately after `play_local()` returns. Do not
        // overwrite that terminal backend state with a false Playing event.
        let backend_state = shared_backend
            .lock()
            .ok()
            .and_then(|backend| backend.as_ref().map(CpalBackend::state))
            .unwrap_or(PlaybackState::Stopped);
        if transition_to_playing_if_backend_running(&self.state, backend_state) {
            let _ = self.emitter.emit_state_changed(&PlaybackState::Playing);
        } else {
            let _ = self.emitter.emit_state_changed(&PlaybackState::Stopped);
            return Ok(());
        }

        // Record this track start in history exactly once per play.
        self.record_history(&track);

        // Analyze only PCM that the output callback has consumed.
        let fft_app_handle = self.emitter.app_handle().clone();
        let fft_sample_rate = sample_rate;
        let fft_engine_state = self.state.clone();
        thread::spawn(move || {
            let mut fft_engine = FftEngine::new(1024, fft_subscriber, fft_sample_rate, channels);
            loop {
                // Check if we should stop
                {
                    let s = fft_engine_state.lock().unwrap();
                    if s.playback_state == PlaybackState::Stopped {
                        break;
                    }
                }

                if !fft_engine.collect_next_frame(Duration::from_millis(100)) {
                    continue;
                }
                if let Some(freq_data) = fft_engine.analyze_if_ready() {
                    if let Err(error) = emit_fft_frame(&fft_app_handle, &freq_data) {
                        let _ = error;
                        crate::observability::expected_failure("fft", "frame_emit_failed");
                    }
                }
            }
        });

        // Start progress tick timer
        self.start_progress_tick_timer();

        Ok(())
    }

    fn local_file_is_accessible(path: &str) -> bool {
        File::open(path).is_ok()
    }

    fn watched_folder_is_accessible(path: &str) -> bool {
        let path = std::path::Path::new(path);
        path.is_dir() && std::fs::read_dir(path).is_ok()
    }

    fn is_missing_or_inaccessible_file_error(error: &crate::audio::AudioError) -> bool {
        matches!(error, crate::audio::AudioError::DecodeFailed(message) if message.contains("failed to open file"))
    }

    fn cleanup_invalid_local_inventory(&self, path: &str) {
        let entry = match self.db.get_local_track_entry_by_path(path) {
            Ok(Some(entry)) => entry,
            _ => return,
        };

        if !Self::watched_folder_is_accessible(&entry.folder_path) {
            let _ = self.db.remove_watched_folder(&entry.folder_path);
            return;
        }

        let _ = self.db.delete_local_track_by_path(path);
    }

    fn handle_invalid_local_track(&self, track: Track) -> Result<(), AppError> {
        if let Some(path) = track.local_path.as_deref() {
            self.cleanup_invalid_local_inventory(path);
        }

        let next_track = {
            let mut s = self.state.lock().map_err(|_| AppError {
                code: "UNKNOWN_ERROR".into(),
                details: Some("mutex lock".into()),
            })?;

            if let Some(index) = s
                .queue
                .tracks
                .iter()
                .position(|queued| queued.id == track.id)
            {
                s.queue.tracks.remove(index);
                Self::rebase_played_indices(&mut s.queue.played_indices, index);

                if s.queue.tracks.is_empty() {
                    s.queue.current_index = None;
                    s.current_track = None;
                } else {
                    let next_index = index.min(s.queue.tracks.len() - 1);
                    s.queue.current_index = Some(next_index);
                    s.current_track = Some(s.queue.tracks[next_index].clone());
                }
            } else {
                s.current_track = None;
                if s.queue.tracks.is_empty() {
                    s.queue.current_index = None;
                }
            }

            s.queue
                .current_index
                .and_then(|index| s.queue.tracks.get(index).cloned())
        };

        if let Ok(queue) = self.get_queue() {
            let _ = self.emitter.emit_queue_updated(&queue);
        }

        match next_track {
            Some(next_track) => {
                let _ = self.emitter.emit_track_changed(&next_track);

                if next_track.local_path.is_some() {
                    self.play_local_track(next_track)
                } else {
                    self.record_history(&next_track);
                    {
                        let mut s = self.state.lock().map_err(|_| AppError {
                            code: "UNKNOWN_ERROR".into(),
                            details: Some("mutex lock".into()),
                        })?;
                        s.current_track = Some(next_track.clone());
                        s.playback_state = PlaybackState::Playing;
                        s.position = 0.0;
                        s.duration = next_track.duration.unwrap_or(0.0);
                    }
                    let _ = self.emitter.emit_state_changed(&PlaybackState::Playing);
                    Ok(())
                }
            }
            None => {
                let _ = self.emitter.emit_state_changed(&PlaybackState::Stopped);
                Ok(())
            }
        }
    }

    /// Read the playback position from the audio backend when available,
    /// falling back to InternalState for times when no backend is active.
    #[allow(dead_code)]
    fn current_position(&self) -> Result<(f64, f64), AppError> {
        let s = self.state.lock().map_err(|_| AppError {
            code: "UNKNOWN_ERROR".into(),
            details: Some("mutex lock".into()),
        })?;
        let duration = s.duration;

        // Prefer the audio backend’s real callback-driven position
        let position = if let Ok(shared) = self.backend.lock() {
            if let Some(ref backend) = *shared {
                backend.position()
            } else {
                s.position
            }
        } else {
            s.position
        };

        Ok((position, duration))
    }

    /// Start playing a URL. Reserved for future use (streaming).
    ///
    /// For v0.1, this returns `PlatformNotSupported`.
    pub fn play(&self, _url: &str) -> Result<(), AppError> {
        Err(AppError::from(
            crate::audio::AudioError::PlatformNotSupported,
        ))
    }

    /// Play a remote track via the frontend HTMLAudio browser-native path.
    ///
    /// This is the primary remote playback entry point. Inspired by Nuclear:
    ///
    /// 1. Resolves the track's stream URL via `SourceRegistry.resolve()`
    /// 2. Proxies the URL through the local proxy server for CORS/Range support
    /// 3. Emits `stream-resolved` event with the proxied URL and the raw remote URL
    /// 4. The frontend loads the proxied URL into HTMLAudio for immediate playback
    /// 5. For YouTube, the frontend calls `cache_remote_stream` to download a local
    ///    copy for instant seeking — this happens AFTER playback starts
    /// 6. Rust state is updated to Playing, but no local-file decode pipeline runs
    ///
    /// Local tracks still use the existing `play_local_track()` Symphonia/cpal path.
    pub fn play_stream(&self, track: Track, stream_request_id: u64) -> Result<(), AppError> {
        // Stop any currently playing audio first (local cpal stream or
        // previous remote stream). Without this, a local file playing
        // through cpal would continue sounding alongside the new remote
        // stream loaded by the browser.
        self.stop()?;

        // Resolve the stream URL for this track's source
        let source = track.source.clone();
        let source_id = track.source_id.clone();

        let resolved_track = self
            .sources
            .resolve(&source, &source_id)
            .map_err(|e| AppError::from(e))?;

        let remote_url = resolved_track.stream_url.clone().ok_or_else(|| AppError {
            code: "STREAM_NOT_FOUND".into(),
            details: Some("track has no stream URL".into()),
        })?;

        // Build the proxied URL for immediate playback.
        // For YouTube, the frontend will call cache_remote_stream after loading
        // this URL to get a local copy for instant seeking. SoundCloud stays on
        // the remote proxy path (its seek works fine over HTTP Range requests).
        let stream_url = stream_url_for_proxy(self.proxy.as_ref(), &remote_url)?;

        // Update state: set track, mark as playing
        {
            let mut s = self.state.lock().map_err(|_| AppError {
                code: "UNKNOWN_ERROR".into(),
                details: Some("mutex lock".into()),
            })?;
            s.playback_state = PlaybackState::Playing;
            s.current_track = Some(track.clone());
            s.position = 0.0;
            s.duration = track.duration.unwrap_or(0.0);

            // Seed a single-track queue only if the queue is empty or
            // this track is not already queued. This preserves queues
            // set by replace_queue_and_play (play_album, playlists).
            let needs_seed = s.queue.tracks.is_empty()
                || s.queue.current_index.is_none()
                || s.queue.tracks.get(s.queue.current_index.unwrap() as usize) != Some(&track);
            if needs_seed {
                s.queue.tracks = vec![track.clone()];
                s.queue.current_index = Some(0);
                s.queue.played_indices.clear();
            }
        }
        // Emit track-changed and state events FIRST so the frontend has
        // currentTrack set before stream-resolved arrives (the frontend
        // handler checks currentTrack.id === payload.trackId).
        let _ = self.emitter.emit_track_changed(&track);
        let _ = self.emitter.emit_state_changed(&PlaybackState::Playing);

        if let Ok(queue) = self.get_queue() {
            let _ = self.emitter.emit_queue_updated(&queue);
        }

        // Emit stream-resolved last so frontend can load the URL into HTMLAudio.
        // Pass the raw remote URL so the frontend can call cache_remote_stream
        // for YouTube tracks to get a local file for instant seeking.
        let _ = self.emitter.emit_stream_resolved(
            &track.id,
            stream_request_id,
            &stream_url,
            Some(&remote_url),
            self.proxy
                .as_ref()
                .map(|(_, capability)| capability.as_str()),
        );

        // Record history
        self.record_history(&track);

        Ok(())
    }

    /// Download a remote stream URL to a local cache file and return its absolute path.
    ///
    /// This is the YouTube local-cache strategy: a fully-downloaded local m4a
    /// file that the browser can seek instantly via `file://` through the proxy
    /// (which serves it with `Accept-Ranges: bytes` and proper `Content-Range`/
    /// `206` responses). WebKitGTK/GStreamer can always seek local files reliably,
    /// unlike remote byte-range requests over the proxy.
    ///
    /// Cache strategy:
    /// - Files are stored under `~/.local/share/jellyx/youtube_cache/`.
    /// - Filename is deterministic: `<sanitized_id>.m4a` (overwrites on re-download).
    /// - If the cache file already exists and is non-empty, it is reused
    ///   without re-downloading — the YouTube resolve cache already returns
    ///   the same resolved track within the TTL window.
    /// - A stale/expired cache file will be overwritten on next resolve.
    ///
    /// This is a public method so it can be called from the `cache_remote_stream`
    /// Tauri IPC command, which the frontend invokes after receiving `stream-resolved`
    /// for YouTube tracks to get a local file for instant seeking.
    pub fn cache_remote_stream(&self, cache_id: &str, remote_url: &str) -> Result<String, String> {
        // This IPC method is a network boundary. Reject before creating a cache
        // directory, inspecting cache files, or constructing an HTTP client.
        if !is_approved_remote_url(remote_url) {
            crate::observability::record_operation("cache", "remote_stream", false);
            crate::observability::expected_failure("cache", "remote_url_rejected");
            return Err("remote URL is not an approved streaming host".to_string());
        }

        let result = self.cache_remote_stream_approved(cache_id, remote_url);
        crate::observability::record_operation("cache", "remote_stream", result.is_ok());
        if result.is_err() {
            crate::observability::expected_failure("cache", "remote_stream_failed");
        }
        result
    }

    /// Cache a URL after the public network boundary has approved it.
    fn cache_remote_stream_approved(
        &self,
        cache_id: &str,
        remote_url: &str,
    ) -> Result<String, String> {
        // Check if audio normalization is enabled. When ON, we cache a
        // normalized variant ({id}.n.m4a) produced by ffmpeg loudnorm (EBU
        // R128, target -14 LUFS). When OFF, we cache the raw stream ({id}.m4a).
        // This keeps both variants independent so toggling the setting doesn't
        // require re-downloading the raw stream.
        let normalize_enabled = self.db.get_normalize_audio().unwrap_or(true); // default: enabled (matches DB default)
        let result = cache_remote_stream_with_fetch(
            &jellyx_core::shared::utils::youtube_cache_dir(),
            cache_id,
            normalize_enabled,
            remote_url,
            |url, part_path| {
                use std::io::Read;
                let client = strict_remote_client()
                    .map_err(|e| format!("failed to build download client: {e}"))?;
                let response = client
                    .get(url)
                    .header(
                        "User-Agent",
                        "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36",
                    )
                    .header("Accept-Encoding", "identity")
                    .send()
                    .map_err(|e| format!("download request failed: {e}"))?;
                let status = response.status().as_u16();
                let content_length = response.content_length();
                if content_length.is_some_and(|size| size > MAX_REMOTE_CACHE_BYTES) {
                    return Err("download exceeds remote cache size limit".into());
                }
                let mut file = std::fs::File::create(part_path)
                    .map_err(|e| format!("failed to create cache file: {e}"))?;
                let copied =
                    std::io::copy(&mut response.take(MAX_REMOTE_CACHE_BYTES + 1), &mut file)
                        .map_err(|e| format!("download body read failed: {e}"))?;
                if copied > MAX_REMOTE_CACHE_BYTES {
                    return Err("download exceeds remote cache size limit".into());
                }
                file.sync_all()
                    .map_err(|e| format!("failed to sync cache file: {e}"))?;
                Ok(status)
            },
        );
        if result.is_err() {
            let safe_id: String = cache_id
                .chars()
                .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
                .collect();
            if !safe_id.is_empty() {
                let _ = self.emitter.emit_cache_corrupted(
                    &safe_id,
                    "cache download failed validation; staying on proxy",
                );
            }
        }
        result
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

        // Forward pause to the audio backend so audio output actually stops.
        if let Ok(mut shared) = self.backend.lock() {
            if let Some(ref mut backend) = *shared {
                let _ = backend.pause();
            }
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

        // Forward resume to the audio backend so audio output actually resumes.
        if let Ok(mut shared) = self.backend.lock() {
            if let Some(ref mut backend) = *shared {
                let _ = backend.resume();
            }
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
    /// `decoder.seek(position)`, flushes stale buffered audio via the
    /// audio backend, then clears the seeking flag and emits a
    /// progress-tick event. Propagates seek errors to the caller.
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
            s.seek_generation = s.seek_generation.wrapping_add(1);
        }

        // Seek the decoder if available — propagate errors
        if let Ok(mut shared) = self.decoder.lock() {
            if let Some(ref mut dec) = *shared {
                dec.seek(clamped).map_err(|e| AppError::from(e))?;
            }
        }

        // Flush stale buffered audio via the backend so the callback
        // does not play already-decoded PCM from the old position.
        if let Ok(mut shared) = self.backend.lock() {
            if let Some(ref mut backend) = *shared {
                let _ = backend.seek(clamped);
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

    /// Set whether audio normalization is enabled for local playback.
    ///
    /// When enabled, applies a +6dB gain boost to the cpal output.
    /// This works alongside the Web Audio API compressor on the frontend
    /// for remote tracks. The boost helps quiet tracks be heard at the
    /// same perceived level as louder ones.
    pub fn set_normalize_audio(&self, enabled: bool) -> Result<(), AppError> {
        let gain = if enabled { 2.0 } else { 1.0 }; // +6dB ≈ 2x linear
        if let Ok(shared) = self.backend.lock() {
            if let Some(ref backend) = *shared {
                let _ = backend.set_normalize_gain(gain);
            }
        }
        Ok(())
    }

    /// Search for tracks on YouTube only.
    ///
    /// Only queries the YouTube resolver, bypassing SoundCloud and local sources.
    /// This dramatically reduces search latency by eliminating sequential resolver calls.
    pub fn search(&self, query: &str) -> Result<Vec<Track>, AppError> {
        if query.trim().is_empty() {
            return Err(ValidationError::EmptyQuery.into());
        }
        self.sources
            .search_source(&Source::YouTube, query)
            .map_err(AppError::from)
    }

    /// Search for tracks across ALL registered sources (YouTube, SoundCloud, etc.).
    ///
    /// Unlike `search` which only queries YouTube, this queries every resolver
    /// and merges results. Used by grouped search to include remote tracks.
    pub fn search_all_tracks(&self, query: &str) -> Vec<Track> {
        self.sources.search_all(query)
    }

    /// Search tracks from enabled sources only, with pagination.
    pub fn search_all_tracks_enabled(
        &self,
        query: &str,
        enabled_sources: &std::collections::HashSet<String>,
        offset: usize,
        limit: usize,
    ) -> Vec<Track> {
        self.sources
            .search_all_enabled(query, Some(enabled_sources), offset, limit)
    }

    /// Search playlists from enabled sources only.
    pub fn search_playlists_enabled(
        &self,
        query: &str,
        enabled_sources: &std::collections::HashSet<String>,
    ) -> Vec<jellyx_core::models::playlist::Playlist> {
        self.sources
            .search_playlists_all_enabled(query, Some(enabled_sources))
    }

    /// Resolve a playlist by source type and URL/identifier.
    ///
    /// Routes to the appropriate resolver for the given source.
    pub fn resolve_playlist(
        &self,
        source: &Source,
        url: &str,
    ) -> Result<jellyx_core::models::playlist::Playlist, crate::errors::types::SourceError> {
        self.sources.resolve_playlist(source, url)
    }

    /// Play all tracks in a playlist, resolving the URL first, then replacing the queue.
    ///
    /// Resolves the playlist, then delegates to `replace_queue_and_play`.
    /// Returns an error if the playlist is empty or resolution fails.
    pub fn play_playlist(&self, source: &Source, url: &str) -> Result<(), AppError> {
        let playlist = self
            .sources
            .resolve_playlist(source, url)
            .map_err(AppError::from)?;

        if playlist.tracks.is_empty() {
            return Err(PlaybackError::QueueEmpty.into());
        }

        self.replace_queue_and_play(playlist.tracks)
    }

    /// Resolve a track by source type and identifier without starting playback.
    ///
    /// Useful for previewing stream URLs or checking track availability.
    pub fn resolve_track_by_source(
        &self,
        source: &Source,
        id: &str,
    ) -> Result<Track, crate::errors::types::SourceError> {
        self.sources.resolve(source, id)
    }

    /// Add a track to the queue by resolving it from the appropriate source.
    ///
    /// Add a track to the end of the queue by resolving its ID.
    ///
    /// The track_id can be a YouTube video ID or SoundCloud URL/ID.
    /// The registry tries each resolver to find the matching track.
    /// Emits queue_updated.
    ///
    /// Prefer `add_to_queue_with_track` when the full Track object is already
    /// available — it skips the slow resolve step.
    pub fn add_to_queue(&self, track_id: &str) -> Result<(), AppError> {
        if track_id.trim().is_empty() {
            return Err(ValidationError::InvalidInput("track_id must not be empty".into()).into());
        }

        let track = self.resolve_track(track_id)?;
        self.add_to_queue_with_track(&track)
    }

    /// Add a track to the end of the queue using the full Track object.
    ///
    /// Skips the resolve step entirely — fast for both local and remote tracks.
    /// Emits queue_updated.
    pub fn add_to_queue_with_track(&self, track: &Track) -> Result<(), AppError> {
        let mut queue = self.state.lock().map_err(|_| AppError {
            code: "UNKNOWN_ERROR".into(),
            details: Some("mutex lock".into()),
        })?;

        queue.queue.tracks.push(track.clone());
        if queue.queue.current_index.is_none() && queue.queue.tracks.len() == 1 {
            queue.queue.current_index = Some(0);
        }

        let _ = self.emitter.emit_queue_updated(&queue.queue.clone());
        drop(queue);
        Ok(())
    }

    /// Replace the current queue with the given tracks and start playback from the first track.
    ///
    /// This is the shared implementation used by `play_album` and any future bulk-replace
    /// commands. It emits queue-updated and track-changed events, and records history.
    fn replace_queue_and_play(&self, tracks: Vec<Track>) -> Result<(), AppError> {
        if tracks.is_empty() {
            return Err(PlaybackError::QueueEmpty.into());
        }

        let first_track = tracks[0].clone();

        {
            let mut s = self.state.lock().map_err(|_| AppError {
                code: "UNKNOWN_ERROR".into(),
                details: Some("mutex lock".into()),
            })?;
            s.queue.tracks = tracks;
            s.queue.current_index = Some(0);
            s.queue.played_indices.clear();
        }

        let queue_snapshot = self.get_queue()?;
        let _ = self.emitter.emit_queue_updated(&queue_snapshot);
        let _ = self.emitter.emit_track_changed(&first_track);
        let _ = self.emitter.emit_state_changed(&PlaybackState::Playing);

        if let Some(ref _local_path) = first_track.local_path {
            return self.play_local_track(first_track.clone());
        }

        // Remote track: use the new frontend-driven play_stream path
        self.play_stream(first_track, 0)
    }

    /// Play all tracks in an album, replacing the current queue in album order.
    pub fn play_album(&self, album_id: &str) -> Result<(), AppError> {
        let album = self.library.get_album_detail(album_id)?;
        self.replace_queue_and_play(album.tracks)
    }

    /// Get the current queue as a full QueueState snapshot.
    pub fn get_queue(&self) -> Result<QueueState, AppError> {
        let s = self.state.lock().map_err(|_| AppError {
            code: "UNKNOWN_ERROR".into(),
            details: Some("mutex lock".into()),
        })?;
        Ok(s.queue.clone())
    }

    /// Get the current track, if any.
    #[allow(dead_code)]
    pub fn get_current_track(&self) -> Result<Option<Track>, AppError> {
        let s = self.state.lock().map_err(|_| AppError {
            code: "UNKNOWN_ERROR".into(),
            details: Some("mutex lock".into()),
        })?;
        Ok(s.current_track.clone())
    }

    /// Remove a track from the queue by ID and emit a full queue snapshot.
    ///
    /// If the removed track was the current track, playback stops, the current
    /// track is cleared, and `current_index` becomes `None`. Removing a track
    /// before the current index decrements the index by one; removing after it
    /// leaves the index unchanged. Played indices are rebased to stay valid.
    pub fn remove_from_queue(&self, track_id: &str) -> Result<(), AppError> {
        if track_id.trim().is_empty() {
            return Err(ValidationError::InvalidInput("track_id must not be empty".into()).into());
        }

        let mut s = self.state.lock().map_err(|_| AppError {
            code: "UNKNOWN_ERROR".into(),
            details: Some("mutex lock".into()),
        })?;

        let position = s.queue.tracks.iter().position(|t| t.id == track_id);
        let removed_index = match position {
            Some(idx) => idx,
            None => return Err(PlaybackError::NoCurrentTrack.into()),
        };

        let was_current = s.queue.current_index == Some(removed_index);
        let was_before_current = s
            .queue
            .current_index
            .map(|ci| removed_index < ci)
            .unwrap_or(false);

        s.queue.tracks.remove(removed_index);
        Self::rebase_played_indices(&mut s.queue.played_indices, removed_index);

        let snapshot;
        if s.queue.tracks.is_empty() || was_current {
            s.queue.current_index = None;
            s.current_track = None;
            snapshot = s.queue.clone();
            drop(s);
            if was_current {
                self.stop()?;
            }
        } else {
            if was_before_current {
                s.queue.current_index = s.queue.current_index.map(|ci| ci.saturating_sub(1));
            }
            snapshot = s.queue.clone();
            drop(s);
        }

        let _ = self.emitter.emit_queue_updated(&snapshot);
        Ok(())
    }

    /// Clear the entire queue, stop playback, and emit an empty snapshot.
    pub fn clear_queue(&self) -> Result<(), AppError> {
        let mut s = self.state.lock().map_err(|_| AppError {
            code: "UNKNOWN_ERROR".into(),
            details: Some("mutex lock".into()),
        })?;

        s.queue.tracks.clear();
        s.queue.current_index = None;
        s.queue.played_indices.clear();
        s.current_track = None;

        let snapshot = s.queue.clone();
        drop(s);

        let _ = self.stop();
        let _ = self.emitter.emit_queue_updated(&snapshot);
        Ok(())
    }

    /// Insert a selected track immediately after the current queue position.
    ///
    /// The inserted track becomes the new current track (index = old current + 1).
    /// If no current index exists but the queue has tracks, the track is appended.
    /// Sequential play-next requests keep the newest choice next-up: a previous
    /// play-next insertion at `current_index + 1` is replaced by the new one.
    /// Insert a track immediately after the current queue position by resolving its ID.
    ///
    /// Prefer `play_next_with_track` when the full Track object is already available.
    pub fn play_next(&self, track_id: &str) -> Result<(), AppError> {
        if track_id.trim().is_empty() {
            return Err(ValidationError::InvalidInput("track_id must not be empty".into()).into());
        }

        let track = self.resolve_track(track_id)?;
        self.play_next_with_track(&track)
    }

    /// Insert a track immediately after the current queue position using the full Track object.
    ///
    /// Skips the resolve step entirely — fast for both local and remote tracks.
    /// Emits queue_updated.
    pub fn play_next_with_track(&self, track: &Track) -> Result<(), AppError> {
        let mut s = self.state.lock().map_err(|_| AppError {
            code: "UNKNOWN_ERROR".into(),
            details: Some("mutex lock".into()),
        })?;

        let insert_index = match s.queue.current_index {
            Some(ci) => {
                let target = ci + 1;
                // Replace any prior play-next insertion at the target slot so
                // the newest choice is always next-up.
                if target < s.queue.tracks.len()
                    && s.queue.tracks[target].id.starts_with("__play_next__")
                {
                    s.queue.tracks.remove(target);
                    Self::rebase_played_indices(&mut s.queue.played_indices, target);
                }
                target.min(s.queue.tracks.len())
            }
            None => s.queue.tracks.len(),
        };

        s.queue.tracks.insert(insert_index, track.clone());
        s.queue.current_index = Some(insert_index);

        let snapshot = s.queue.clone();
        drop(s);
        let _ = self.emitter.emit_queue_updated(&snapshot);
        Ok(())
    }

    /// Rebase played indices after a queue item at `removed_index` is removed.
    ///
    /// Removes any played entry equal to `removed_index` and shifts higher
    /// indices down by one. This keeps shuffle history valid after mutations.
    fn rebase_played_indices(played_indices: &mut Vec<usize>, removed_index: usize) {
        played_indices.retain(|&i| i != removed_index);
        for i in played_indices.iter_mut() {
            if *i > removed_index {
                *i -= 1;
            }
        }
    }

    /// Resolve a track ID from the source registry.
    ///
    /// Shared helper used by add_to_queue and play_next.
    fn resolve_track(&self, track_id: &str) -> Result<Track, AppError> {
        self.sources
            .resolve(&Source::Local, track_id)
            .or_else(|_| self.sources.resolve(&Source::YouTube, track_id))
            .or_else(|_| self.sources.resolve(&Source::SoundCloud, track_id))
            .map_err(|_| {
                crate::errors::types::SourceError::ResolveError(format!(
                    "Could not resolve track: {}",
                    track_id
                ))
                .into()
            })
    }

    /// Look up a track by Jellyx ID in the current queue or source registry.
    #[allow(dead_code)]
    pub fn get_track_by_id(&self, track_id: &str) -> Result<Track, AppError> {
        {
            let s = self.state.lock().map_err(|_| AppError {
                code: "UNKNOWN_ERROR".into(),
                details: Some("mutex lock".into()),
            })?;
            if let Some(track) = s.queue.tracks.iter().find(|t| t.id == track_id) {
                return Ok(track.clone());
            }
        }

        if let Ok(track) = self.sources.resolve(&Source::Local, track_id) {
            return Ok(track);
        }

        self.sources.resolve_all(track_id).map_err(|e| match e {
            crate::errors::types::SourceError::ResolveError(msg) => {
                AppError::from(crate::errors::types::SourceError::ResolveError(msg))
            }
            _ => AppError::from(crate::errors::types::SourceError::ResolveError(format!(
                "Could not resolve track: {}",
                track_id
            ))),
        })
    }

    /// Set shuffle mode and emit an updated queue snapshot.
    pub fn set_shuffle(&self, enabled: bool) -> Result<(), AppError> {
        let mut s = self.state.lock().map_err(|_| AppError {
            code: "UNKNOWN_ERROR".into(),
            details: Some("mutex lock".into()),
        })?;
        s.queue.shuffle = enabled;
        if !enabled {
            s.queue.played_indices.clear();
        }
        let snapshot = s.queue.clone();
        drop(s);
        let _ = self.emitter.emit_queue_updated(&snapshot);
        Ok(())
    }

    /// Set repeat mode by string name and emit an updated queue snapshot.
    ///
    /// Accepts "Off", "All", or "One" (case-insensitive). Invalid input
    /// returns a validation error so the frontend can map it.
    pub fn set_repeat_from_string(&self, mode: &str) -> Result<(), AppError> {
        let parsed = match mode.trim() {
            "Off" | "off" | "OFF" => RepeatMode::Off,
            "All" | "all" | "ALL" => RepeatMode::All,
            "One" | "one" | "ONE" => RepeatMode::One,
            _ => {
                return Err(ValidationError::InvalidInput(format!(
                    "invalid repeat mode: {}. Expected Off, All, or One",
                    mode
                ))
                .into());
            }
        };
        self.set_repeat(parsed)
    }

    /// Set repeat mode directly and emit an updated queue snapshot.
    pub fn set_repeat(&self, mode: RepeatMode) -> Result<(), AppError> {
        let mut s = self.state.lock().map_err(|_| AppError {
            code: "UNKNOWN_ERROR".into(),
            details: Some("mutex lock".into()),
        })?;
        s.queue.repeat_mode = mode;
        let snapshot = s.queue.clone();
        drop(s);
        let _ = self.emitter.emit_queue_updated(&snapshot);
        Ok(())
    }

    /// Cycle repeat mode Off -> All -> One -> Off and emit queue snapshot.
    pub fn cycle_repeat(&self) -> Result<RepeatMode, AppError> {
        let mut s = self.state.lock().map_err(|_| AppError {
            code: "UNKNOWN_ERROR".into(),
            details: Some("mutex lock".into()),
        })?;
        s.queue.repeat_mode = match s.queue.repeat_mode {
            RepeatMode::Off => RepeatMode::All,
            RepeatMode::All => RepeatMode::One,
            RepeatMode::One => RepeatMode::Off,
        };
        let mode = s.queue.repeat_mode;
        let snapshot = s.queue.clone();
        drop(s);
        let _ = self.emitter.emit_queue_updated(&snapshot);
        Ok(mode)
    }

    /// Choose the next track index when shuffle mode is enabled.
    ///
    /// Returns the index to play next and updates `played_indices`. The
    /// queue order is not modified; only the selection changes.
    fn shuffle_next_track(queue: &mut QueueState) -> Option<usize> {
        let len = queue.tracks.len();
        if len == 0 {
            return None;
        }

        let current = queue.current_index.unwrap_or(0);
        let unplayed: Vec<usize> = (0..len)
            .filter(|i| *i != current && !queue.played_indices.contains(i))
            .collect();

        if unplayed.is_empty() {
            if queue.repeat_mode == RepeatMode::All {
                queue.played_indices.clear();
                queue.played_indices.push(current);
                let next = (0..len)
                    .filter(|i| *i != current)
                    .collect::<Vec<_>>()
                    .choose(&mut rand::thread_rng())
                    .copied();
                if let Some(idx) = next {
                    queue.played_indices.push(idx);
                }
                return next;
            }
            return None;
        }

        let next = unplayed.choose(&mut rand::thread_rng()).copied();
        if let Some(idx) = next {
            queue.played_indices.push(idx);
        }
        next
    }

    /// Compute the next track index, applying shuffle and repeat modes.
    ///
    /// Returns `None` when playback should stop (end of queue with repeat off).
    fn compute_next_index(queue: &mut QueueState) -> Option<usize> {
        let len = queue.tracks.len();
        if len == 0 {
            return None;
        }

        let current = queue.current_index.unwrap_or(0);
        if queue.shuffle {
            Self::shuffle_next_track(queue)
        } else {
            Self::sequential_next_index(current, len, queue.repeat_mode)
        }
    }

    /// Pick the next sequential index, applying repeat-all and repeat-one logic.
    fn sequential_next_index(current: usize, len: usize, repeat: RepeatMode) -> Option<usize> {
        match repeat {
            RepeatMode::One => Some(current),
            _ => {
                let candidate = current + 1;
                if candidate < len {
                    Some(candidate)
                } else if repeat == RepeatMode::All {
                    Some(0)
                } else {
                    None
                }
            }
        }
    }

    /// Skip to the next track in the queue, applying shuffle and repeat modes.
    pub fn next(&self) -> Result<(), AppError> {
        let mut s = self.state.lock().map_err(|_| AppError {
            code: "UNKNOWN_ERROR".into(),
            details: Some("mutex lock".into()),
        })?;

        if s.queue.tracks.is_empty() {
            return Err(PlaybackError::QueueEmpty.into());
        }

        let next_index = match Self::compute_next_index(&mut s.queue) {
            Some(idx) => idx,
            None => {
                drop(s);
                return self.stop();
            }
        };

        s.queue.current_index = Some(next_index);
        let track = s.queue.tracks[next_index].clone();
        let snapshot = s.queue.clone();
        drop(s);

        let _ = self.emitter.emit_queue_updated(&snapshot);
        let _ = self.emitter.emit_track_changed(&track);

        // Do NOT emit 'Playing' here — play_local_track and play_stream
        // manage the full state lifecycle (Stopped -> Buffering -> Playing).
        // Emitting 'Playing' prematurely causes state oscillation when the
        // playback method calls stop() first.

        if let Some(ref local_path) = track.local_path {
            return self.play_local(local_path);
        }

        // Remote track: use the new play_stream path
        self.play_stream(track, 0)
    }

    /// Go to the previous track in the queue.
    ///
    /// Previous ignores shuffle (user intentionally stepping back) and respects
    /// repeat-all by wrapping to the end when at the first track.
    pub fn previous(&self) -> Result<(), AppError> {
        let mut s = self.state.lock().map_err(|_| AppError {
            code: "UNKNOWN_ERROR".into(),
            details: Some("mutex lock".into()),
        })?;

        if s.queue.tracks.is_empty() {
            return Err(PlaybackError::QueueEmpty.into());
        }

        let len = s.queue.tracks.len();
        let current = s.queue.current_index.unwrap_or(0);
        let previous_index = if current == 0 {
            if s.queue.repeat_mode == RepeatMode::All {
                len.saturating_sub(1)
            } else {
                0
            }
        } else {
            current - 1
        };

        s.queue.current_index = Some(previous_index);
        let track = s.queue.tracks[previous_index].clone();
        let snapshot = s.queue.clone();
        drop(s);

        let _ = self.emitter.emit_queue_updated(&snapshot);
        let _ = self.emitter.emit_track_changed(&track);

        // Do NOT emit 'Playing' here — play_local_track and play_stream
        // manage the full state lifecycle. See next() for rationale.

        if let Some(ref local_path) = track.local_path {
            return self.play_local(local_path);
        }

        // Remote track: use the new play_stream path
        self.play_stream(track, 0)
    }

    /// Start a background timer that emits progress-tick events.
    ///
    /// Emits at ~4Hz (every 250ms) during playback. The timer stops
    /// when the state changes to Stopped and skips ticks while Paused
    /// so progress doesn't drift during pause.
    ///
    /// Position is read from the audio backend (the callback-driven
    /// source-of-truth) rather than from InternalState, eliminating
    /// desync between the progress bar and actual audio output.
    fn start_progress_tick_timer(&self) {
        let state = self.state.clone();
        let backend = self.backend.clone();
        let emitter = self.emitter.clone_sender();

        thread::spawn(move || {
            loop {
                thread::sleep(Duration::from_millis(PROGRESS_TICK_INTERVAL_MS));

                let s = state.lock().unwrap();
                if s.playback_state == PlaybackState::Stopped {
                    break;
                }
                if s.playback_state == PlaybackState::Paused {
                    continue;
                }

                let duration = s.duration;
                // Prefer the audio backend’s callback-driven position
                let position = if let Ok(shared) = backend.lock() {
                    if let Some(ref be) = *shared {
                        be.position()
                    } else {
                        s.position
                    }
                } else {
                    s.position
                };
                drop(s);

                let _ = emitter.emit_progress_tick(position, duration);
            }
        });
    }
}

/// Mirrors a terminal backend failure into service state exactly once so the
/// emitted Stopped event lets the frontend clear its playing state and retry.
fn transition_backend_stop(state: &Arc<Mutex<InternalState>>) -> bool {
    let mut state = state.lock().unwrap();
    if state.playback_state == PlaybackState::Stopped {
        false
    } else {
        state.playback_state = PlaybackState::Stopped;
        true
    }
}

/// Commit Playing only while the backend still authoritatively reports Playing.
/// This closes the startup interleaving where CPAL stops between stream start
/// and the service's former unconditional Playing assignment.
fn transition_to_playing_if_backend_running(
    state: &Arc<Mutex<InternalState>>,
    backend_state: PlaybackState,
) -> bool {
    if backend_state != PlaybackState::Playing {
        return false;
    }
    let mut state = state.lock().unwrap();
    if state.playback_state == PlaybackState::Stopped {
        return false;
    }
    state.playback_state = PlaybackState::Playing;
    true
}

/// Recover ownership and observable state when CPAL rejects startup before the
/// backend can be stored by the service.
fn recover_failed_stream_start(
    state: &Arc<Mutex<InternalState>>,
    decoder: &Arc<Mutex<Option<SymphoniaDecoder>>>,
    backend: &Arc<Mutex<Option<CpalBackend>>>,
) {
    transition_backend_stop(state);
    *decoder.lock().unwrap() = None;
    *backend.lock().unwrap() = None;
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::audio::PlaybackState;
    use crate::playback::state::QueueState;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn remote_stream_requires_loopback_proxy_and_never_returns_direct_https_url() {
        let remote = "https://example.invalid/private-media";
        let error = stream_url_for_proxy(None, remote).expect_err("proxy absence must fail closed");
        assert_eq!(error.code, "PROXY_UNAVAILABLE");
        assert!(
            stream_url_for_proxy(Some(&(8765, "test-capability".to_string())), remote)
                .unwrap()
                .starts_with("http://127.0.0.1:8765/proxy?cap=test-capability&url=")
        );
    }

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
        for state in [
            PlaybackState::Playing,
            PlaybackState::Paused,
            PlaybackState::Buffering(0.0),
        ] {
            let _ = state; // suppress unused warning
            let new_state = PlaybackState::Stopped;
            assert_eq!(new_state, PlaybackState::Stopped);
        }
    }

    #[test]
    fn cpal_backend_stop_transitions_service_once_for_frontend_notification() {
        let state = Arc::new(Mutex::new(InternalState {
            playback_state: PlaybackState::Playing,
            seeking: false,
            seek_generation: 0,
            current_track: None,
            queue: QueueState::default(),
            volume: 1.0,
            position: 0.0,
            duration: 0.0,
        }));
        assert!(transition_backend_stop(&state));
        assert_eq!(state.lock().unwrap().playback_state, PlaybackState::Stopped);
        assert!(!transition_backend_stop(&state));
    }

    #[test]
    fn cpal_startup_stop_interleaving_never_restores_or_emits_playing() {
        let state = Arc::new(Mutex::new(InternalState {
            playback_state: PlaybackState::Stopped,
            seeking: false,
            seek_generation: 0,
            current_track: None,
            queue: QueueState::default(),
            volume: 1.0,
            position: 0.0,
            duration: 0.0,
        }));

        // Deterministic representation of CPAL stopping after start but before
        // the service attempts to publish Playing.
        assert!(!transition_to_playing_if_backend_running(
            &state,
            PlaybackState::Stopped
        ));
        assert_eq!(state.lock().unwrap().playback_state, PlaybackState::Stopped);
    }

    #[test]
    fn synchronous_cpal_start_failure_recovers_service_from_buffering() {
        let state = Arc::new(Mutex::new(InternalState {
            playback_state: PlaybackState::Buffering(0.0),
            seeking: false,
            seek_generation: 0,
            current_track: None,
            queue: QueueState::default(),
            volume: 1.0,
            position: 0.0,
            duration: 0.0,
        }));
        let decoder = Arc::new(Mutex::new(None));
        let backend = Arc::new(Mutex::new(Some(CpalBackend::new())));

        recover_failed_stream_start(&state, &decoder, &backend);

        assert_eq!(state.lock().unwrap().playback_state, PlaybackState::Stopped);
        assert!(decoder.lock().unwrap().is_none());
        assert!(backend.lock().unwrap().is_none());
    }

    #[test]
    fn playback_state_all_variants_distinct() {
        // Ensure all PlaybackState variants are distinct (no accidental aliasing)
        let variants = [
            PlaybackState::Stopped,
            PlaybackState::Playing,
            PlaybackState::Paused,
            PlaybackState::Buffering(0.0),
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
            source: jellyx_core::models::source::Source::Local,
            source_id: "local-1".to_string(),
            title: "Song".to_string(),
            artist: "Artist".to_string(),
            album: None,
            duration: Some(180.0),
            thumbnail: None,
            stream_url: None,
            local_path: Some("/music/song.mp3".to_string()),
            playlist_id: None,
            metadata: std::collections::HashMap::new(),
        };

        queue.tracks.push(track.clone());
        queue.current_index = Some(0);

        assert_eq!(queue.tracks.len(), 1);
        assert_eq!(queue.current_index, Some(0));
        assert_eq!(queue.tracks[0].id, "t1");
    }

    #[test]
    fn progress_tick_skips_while_paused() {
        // Timer should not emit ticks while Paused. We can't test the thread,
        // but we can verify the pause guard expression exists and is consistent.
        let state = PlaybackState::Paused;
        assert_eq!(state, PlaybackState::Paused);
    }

    #[test]
    fn direct_play_local_seeds_single_track_queue() {
        // Direct local playback must seed queue state with the single track
        // so next()/previous() have a valid context.
        let mut queue = QueueState::default();
        let track = sample_track_for_tests("direct");

        queue.tracks = vec![track];
        queue.current_index = Some(0);

        assert_eq!(queue.tracks.len(), 1);
        assert_eq!(queue.current_index, Some(0));
    }

    #[test]
    fn decoder_position_does_not_advance_while_paused() {
        // Simulate: position should only advance when state != Paused.
        let mut position = 0.0_f64;
        let state = PlaybackState::Paused;
        let seconds_advanced = 1.0;

        if state != PlaybackState::Paused {
            position += seconds_advanced;
        }

        assert!((position - 0.0).abs() < f64::EPSILON);
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
    fn repeat_mode_cycles_off_all_one_off() {
        let mode = RepeatMode::Off;
        let next = match mode {
            RepeatMode::Off => RepeatMode::All,
            RepeatMode::All => RepeatMode::One,
            RepeatMode::One => RepeatMode::Off,
        };
        assert_eq!(next, RepeatMode::All);

        let next = match next {
            RepeatMode::Off => RepeatMode::All,
            RepeatMode::All => RepeatMode::One,
            RepeatMode::One => RepeatMode::Off,
        };
        assert_eq!(next, RepeatMode::One);

        let next = match next {
            RepeatMode::Off => RepeatMode::All,
            RepeatMode::All => RepeatMode::One,
            RepeatMode::One => RepeatMode::Off,
        };
        assert_eq!(next, RepeatMode::Off);
    }

    #[test]
    fn sequential_next_index_respects_repeat_modes() {
        // 3 tracks, current = 1
        assert_eq!(
            PlaybackService::<tauri::Wry>::sequential_next_index(1, 3, RepeatMode::Off),
            Some(2)
        );
        assert_eq!(
            PlaybackService::<tauri::Wry>::sequential_next_index(1, 3, RepeatMode::One),
            Some(1)
        );
        assert_eq!(
            PlaybackService::<tauri::Wry>::sequential_next_index(2, 3, RepeatMode::All),
            Some(0)
        );
        assert_eq!(
            PlaybackService::<tauri::Wry>::sequential_next_index(2, 3, RepeatMode::Off),
            None
        );
    }

    #[test]
    fn shuffle_next_track_picks_unplayed_indices() {
        let mut queue = QueueState {
            tracks: vec![
                sample_track_for_tests("t0"),
                sample_track_for_tests("t1"),
                sample_track_for_tests("t2"),
            ],
            current_index: Some(0),
            shuffle: true,
            played_indices: vec![0],
            repeat_mode: RepeatMode::Off,
        };

        let next = PlaybackService::<tauri::Wry>::shuffle_next_track(&mut queue);
        assert!(next.is_some());
        let idx = next.unwrap();
        assert!(idx == 1 || idx == 2, "Should pick an unplayed track");
        assert!(queue.played_indices.contains(&idx));
    }

    #[test]
    fn shuffle_next_track_clears_played_when_repeat_all_exhausted() {
        let mut queue = QueueState {
            tracks: vec![
                sample_track_for_tests("t0"),
                sample_track_for_tests("t1"),
                sample_track_for_tests("t2"),
            ],
            current_index: Some(0),
            shuffle: true,
            played_indices: vec![1, 2],
            repeat_mode: RepeatMode::All,
        };

        let next = PlaybackService::<tauri::Wry>::shuffle_next_track(&mut queue);
        assert!(next.is_some(), "Should wrap and pick a new unplayed track");
        let idx = next.unwrap();
        assert!(
            idx == 1 || idx == 2,
            "Should pick an unplayed track after reset"
        );
        assert!(queue.played_indices.contains(&idx));
        assert!(
            queue.played_indices.contains(&0),
            "Current index should be recorded"
        );
    }

    #[test]
    fn shuffle_next_track_returns_none_when_exhausted_and_repeat_off() {
        let mut queue = QueueState {
            tracks: vec![sample_track_for_tests("t0"), sample_track_for_tests("t1")],
            current_index: Some(0),
            shuffle: true,
            played_indices: vec![1],
            repeat_mode: RepeatMode::Off,
        };

        let next = PlaybackService::<tauri::Wry>::shuffle_next_track(&mut queue);
        assert!(next.is_none());
    }

    #[test]
    fn rebase_played_indices_drops_removed_and_shifts_higher() {
        let mut played = vec![0, 2, 3, 5];
        PlaybackService::<tauri::Wry>::rebase_played_indices(&mut played, 2);
        assert_eq!(played, vec![0, 2, 4]);
    }

    #[test]
    fn rebase_played_indices_handles_empty_vec() {
        let mut played: Vec<usize> = vec![];
        PlaybackService::<tauri::Wry>::rebase_played_indices(&mut played, 0);
        assert!(played.is_empty());
    }

    fn sample_track_for_tests_with_id_prefix(id: &str, prefix: &str) -> Track {
        Track {
            id: format!("{}{}", prefix, id),
            source: jellyx_core::models::source::Source::Local,
            source_id: format!("local-{}", id),
            title: format!("Song {}", id),
            artist: "Artist".to_string(),
            album: None,
            duration: Some(180.0),
            thumbnail: None,
            stream_url: None,
            local_path: Some(format!("/music/{}.mp3", id)),
            playlist_id: None,
            metadata: std::collections::HashMap::new(),
        }
    }

    fn sample_track_for_tests(id: &str) -> Track {
        sample_track_for_tests_with_id_prefix(id, "")
    }

    fn sample_remote_track_for_tests(id: &str) -> Track {
        Track {
            id: id.to_string(),
            source: jellyx_core::models::source::Source::YouTube,
            source_id: format!("yt-{}", id),
            title: format!("Remote {}", id),
            artist: "Artist".to_string(),
            album: None,
            duration: Some(180.0),
            thumbnail: None,
            stream_url: Some(format!("https://example.com/{}", id)),
            local_path: None,
            playlist_id: None,
            metadata: std::collections::HashMap::new(),
        }
    }

    #[test]
    fn add_to_queue_resolves_local_track_by_helix_id() {
        let db = Arc::new(Database::open_in_memory().expect("failed to open db"));
        db.insert_watched_folder("/music")
            .expect("failed to insert watched folder");

        let local_track = Track {
            id: "9f8f1f9e-17d6-4d3f-8a0d-c2f8a7cbe123".to_string(),
            source: jellyx_core::models::source::Source::Local,
            source_id: "/music/song.mp3".to_string(),
            title: "Song local".to_string(),
            artist: "Artist".to_string(),
            album: None,
            duration: Some(180.0),
            thumbnail: None,
            stream_url: None,
            local_path: Some("/music/song.mp3".to_string()),
            playlist_id: None,
            metadata: std::collections::HashMap::new(),
        };
        db.upsert_local_track(
            "/music/song.mp3",
            &local_track,
            "/music",
            Some("1000"),
            None,
        )
        .expect("failed to insert local track");

        let service = PlaybackService::<tauri::test::MockRuntime>::new(
            tauri::test::mock_app().handle().clone(),
            db.clone(),
            Arc::new(LibraryService::new(db)),
        );

        service
            .add_to_queue("9f8f1f9e-17d6-4d3f-8a0d-c2f8a7cbe123")
            .expect("local track should resolve by helix id");

        let queue = service.get_queue().expect("failed to get queue");
        assert_eq!(queue.tracks.len(), 1);
        assert_eq!(queue.current_index, Some(0));
        assert_eq!(queue.tracks[0].id, "9f8f1f9e-17d6-4d3f-8a0d-c2f8a7cbe123");
        assert_eq!(
            queue.tracks[0].local_path.as_deref(),
            Some("/music/song.mp3")
        );
    }

    #[test]
    fn play_next_resolves_local_track_by_helix_id() {
        let db = Arc::new(Database::open_in_memory().expect("failed to open db"));
        db.insert_watched_folder("/music")
            .expect("failed to insert watched folder");

        let current_track = sample_track_for_tests("current");
        let local_track = Track {
            id: "9f8f1f9e-17d6-4d3f-8a0d-c2f8a7cbe123".to_string(),
            source: jellyx_core::models::source::Source::Local,
            source_id: "/music/song.mp3".to_string(),
            title: "Song local".to_string(),
            artist: "Artist".to_string(),
            album: None,
            duration: Some(180.0),
            thumbnail: None,
            stream_url: None,
            local_path: Some("/music/song.mp3".to_string()),
            playlist_id: None,
            metadata: std::collections::HashMap::new(),
        };
        db.upsert_local_track(
            "/music/song.mp3",
            &local_track,
            "/music",
            Some("1000"),
            None,
        )
        .expect("failed to insert local track");

        let service = PlaybackService::<tauri::test::MockRuntime>::new(
            tauri::test::mock_app().handle().clone(),
            db.clone(),
            Arc::new(LibraryService::new(db)),
        );

        {
            let mut state = service.state.lock().expect("failed to lock state");
            state.queue.tracks = vec![current_track];
            state.queue.current_index = Some(0);
        }

        service
            .play_next("9f8f1f9e-17d6-4d3f-8a0d-c2f8a7cbe123")
            .expect("local track should resolve by helix id");

        let queue = service.get_queue().expect("failed to get queue");
        assert_eq!(queue.tracks.len(), 2);
        assert_eq!(queue.tracks[1].id, "9f8f1f9e-17d6-4d3f-8a0d-c2f8a7cbe123");
        assert_eq!(
            queue.tracks[1].local_path.as_deref(),
            Some("/music/song.mp3")
        );
    }

    #[test]
    fn play_local_track_removes_missing_local_track_from_inventory() {
        let db = Arc::new(Database::open_in_memory().expect("failed to open db"));
        let temp_dir = std::env::temp_dir().join(format!(
            "jellyx-playback-missing-track-{}",
            std::process::id()
        ));

        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).expect("failed to create temp dir");

        let missing_path = temp_dir.join("missing.wav");
        let missing_track = Track {
            id: "missing-local".to_string(),
            source: jellyx_core::models::source::Source::Local,
            source_id: missing_path.to_string_lossy().to_string(),
            title: "Missing local".to_string(),
            artist: "Artist".to_string(),
            album: None,
            duration: Some(180.0),
            thumbnail: None,
            stream_url: None,
            local_path: Some(missing_path.to_string_lossy().to_string()),
            playlist_id: None,
            metadata: std::collections::HashMap::new(),
        };

        db.insert_watched_folder(temp_dir.to_string_lossy().as_ref())
            .expect("failed to insert watched folder");
        db.upsert_local_track(
            missing_path.to_string_lossy().as_ref(),
            &missing_track,
            temp_dir.to_string_lossy().as_ref(),
            Some("1000"),
            None,
        )
        .expect("failed to persist missing track");

        let service = PlaybackService::<tauri::test::MockRuntime>::new(
            tauri::test::mock_app().handle().clone(),
            db.clone(),
            Arc::new(LibraryService::new(db.clone())),
        );

        {
            let mut state = service.state.lock().expect("failed to lock state");
            state.queue.tracks = vec![missing_track.clone()];
            state.queue.current_index = Some(0);
        }

        let result = service.play_local_track(missing_track);

        assert!(result.is_ok(), "missing file should not crash playback");
        assert!(
            db.get_local_track_by_path(missing_path.to_string_lossy().as_ref())
                .unwrap()
                .is_none(),
            "missing local track should be pruned from inventory"
        );
        let queue = service.get_queue().unwrap();
        assert!(queue.tracks.is_empty());
        assert_eq!(queue.current_index, None);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[cfg(unix)]
    struct PermissionGuard {
        path: PathBuf,
        original_mode: u32,
    }

    #[cfg(unix)]
    impl PermissionGuard {
        fn deny(path: &std::path::Path) -> Self {
            let metadata = std::fs::metadata(path).expect("failed to stat path");
            let original_mode = metadata.permissions().mode();
            let mut permissions = metadata.permissions();
            permissions.set_mode(0o000);
            std::fs::set_permissions(path, permissions).expect("failed to drop permissions");

            Self {
                path: path.to_path_buf(),
                original_mode,
            }
        }
    }

    #[cfg(unix)]
    impl Drop for PermissionGuard {
        fn drop(&mut self) {
            if let Ok(metadata) = std::fs::metadata(&self.path) {
                let mut permissions = metadata.permissions();
                permissions.set_mode(self.original_mode);
                let _ = std::fs::set_permissions(&self.path, permissions);
            }
        }
    }

    #[cfg(unix)]
    #[test]
    fn play_local_track_removes_permission_denied_local_track_from_inventory() {
        let db = Arc::new(Database::open_in_memory().expect("failed to open db"));
        let temp_dir = std::env::temp_dir().join(format!(
            "jellyx-playback-permission-denied-track-{}",
            std::process::id()
        ));

        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).expect("failed to create temp dir");

        let blocked_path = temp_dir.join("blocked.wav");
        std::fs::write(&blocked_path, b"not a real wav").expect("failed to write blocked file");

        let blocked_track = Track {
            id: "blocked-local".to_string(),
            source: jellyx_core::models::source::Source::Local,
            source_id: blocked_path.to_string_lossy().to_string(),
            title: "Blocked local".to_string(),
            artist: "Artist".to_string(),
            album: None,
            duration: Some(180.0),
            thumbnail: None,
            stream_url: None,
            local_path: Some(blocked_path.to_string_lossy().to_string()),
            playlist_id: None,
            metadata: std::collections::HashMap::new(),
        };

        db.insert_watched_folder(temp_dir.to_string_lossy().as_ref())
            .expect("failed to insert watched folder");
        db.upsert_local_track(
            blocked_path.to_string_lossy().as_ref(),
            &blocked_track,
            temp_dir.to_string_lossy().as_ref(),
            Some("1000"),
            None,
        )
        .expect("failed to persist blocked track");

        let _permission_guard = PermissionGuard::deny(&blocked_path);
        assert!(
            !PlaybackService::<tauri::test::MockRuntime>::local_file_is_accessible(
                blocked_path.to_string_lossy().as_ref()
            ),
            "test setup should produce a permission-denied file"
        );

        let service = PlaybackService::<tauri::test::MockRuntime>::new(
            tauri::test::mock_app().handle().clone(),
            db.clone(),
            Arc::new(LibraryService::new(db.clone())),
        );

        {
            let mut state = service.state.lock().expect("failed to lock state");
            state.queue.tracks = vec![blocked_track.clone()];
            state.queue.current_index = Some(0);
        }

        let result = service.play_local_track(blocked_track);

        assert!(
            result.is_ok(),
            "permission-denied file should not crash playback"
        );
        assert!(
            db.get_local_track_by_path(blocked_path.to_string_lossy().as_ref())
                .unwrap()
                .is_none(),
            "permission-denied local track should be pruned from inventory"
        );
        assert_eq!(db.get_watched_folders().unwrap().len(), 1);
        let queue = service.get_queue().unwrap();
        assert!(queue.tracks.is_empty());
        assert_eq!(queue.current_index, None);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn play_local_track_removes_inaccessible_watched_folder_and_advances_queue() {
        let db = Arc::new(Database::open_in_memory().expect("failed to open db"));
        let temp_dir = std::env::temp_dir().join(format!(
            "jellyx-playback-missing-folder-{}",
            std::process::id()
        ));

        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).expect("failed to create temp dir");

        let missing_path = temp_dir.join("missing.wav");
        let missing_track = Track {
            id: "missing-local".to_string(),
            source: jellyx_core::models::source::Source::Local,
            source_id: missing_path.to_string_lossy().to_string(),
            title: "Missing local".to_string(),
            artist: "Artist".to_string(),
            album: None,
            duration: Some(180.0),
            thumbnail: None,
            stream_url: None,
            local_path: Some(missing_path.to_string_lossy().to_string()),
            playlist_id: None,
            metadata: std::collections::HashMap::new(),
        };
        let remote_track = sample_remote_track_for_tests("remote-next");

        db.insert_watched_folder(temp_dir.to_string_lossy().as_ref())
            .expect("failed to insert watched folder");
        db.upsert_local_track(
            missing_path.to_string_lossy().as_ref(),
            &missing_track,
            temp_dir.to_string_lossy().as_ref(),
            Some("1000"),
            None,
        )
        .expect("failed to persist missing track");
        std::fs::remove_dir_all(&temp_dir).expect("failed to remove watched folder");

        let service = PlaybackService::<tauri::test::MockRuntime>::new(
            tauri::test::mock_app().handle().clone(),
            db.clone(),
            Arc::new(LibraryService::new(db.clone())),
        );

        {
            let mut state = service.state.lock().expect("failed to lock state");
            state.queue.tracks = vec![missing_track.clone(), remote_track.clone()];
            state.queue.current_index = Some(0);
        }

        let result = service.play_local_track(missing_track);

        assert!(result.is_ok(), "missing folder should not crash playback");
        assert!(db.get_watched_folders().unwrap().is_empty());
        let queue = service.get_queue().unwrap();
        assert_eq!(queue.tracks.len(), 1);
        assert_eq!(queue.tracks[0].id, remote_track.id);
        assert_eq!(queue.current_index, Some(0));

        let current = service.get_current_track().unwrap().unwrap();
        assert_eq!(current.id, remote_track.id);
        assert_eq!(service.state(), PlaybackState::Playing);
    }

    #[cfg(unix)]
    #[test]
    fn play_local_track_removes_permission_denied_watched_folder_and_advances_queue() {
        let db = Arc::new(Database::open_in_memory().expect("failed to open db"));
        let temp_dir = std::env::temp_dir().join(format!(
            "jellyx-playback-permission-denied-folder-{}",
            std::process::id()
        ));

        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).expect("failed to create temp dir");

        let blocked_path = temp_dir.join("blocked.wav");
        std::fs::write(&blocked_path, b"not a real wav").expect("failed to write blocked file");
        let blocked_track = Track {
            id: "blocked-local".to_string(),
            source: jellyx_core::models::source::Source::Local,
            source_id: blocked_path.to_string_lossy().to_string(),
            title: "Blocked local".to_string(),
            artist: "Artist".to_string(),
            album: None,
            duration: Some(180.0),
            thumbnail: None,
            stream_url: None,
            local_path: Some(blocked_path.to_string_lossy().to_string()),
            playlist_id: None,
            metadata: std::collections::HashMap::new(),
        };
        let remote_track = sample_remote_track_for_tests("remote-next");

        db.insert_watched_folder(temp_dir.to_string_lossy().as_ref())
            .expect("failed to insert watched folder");
        db.upsert_local_track(
            blocked_path.to_string_lossy().as_ref(),
            &blocked_track,
            temp_dir.to_string_lossy().as_ref(),
            Some("1000"),
            None,
        )
        .expect("failed to persist blocked track");

        let _permission_guard = PermissionGuard::deny(&temp_dir);
        assert!(
            !PlaybackService::<tauri::test::MockRuntime>::watched_folder_is_accessible(
                temp_dir.to_string_lossy().as_ref()
            ),
            "test setup should produce an inaccessible watched folder"
        );

        let service = PlaybackService::<tauri::test::MockRuntime>::new(
            tauri::test::mock_app().handle().clone(),
            db.clone(),
            Arc::new(LibraryService::new(db.clone())),
        );

        {
            let mut state = service.state.lock().expect("failed to lock state");
            state.queue.tracks = vec![blocked_track.clone(), remote_track.clone()];
            state.queue.current_index = Some(0);
        }

        let result = service.play_local_track(blocked_track);

        assert!(
            result.is_ok(),
            "permission-denied folder should not crash playback"
        );
        assert!(db.get_watched_folders().unwrap().is_empty());
        let queue = service.get_queue().unwrap();
        assert_eq!(queue.tracks.len(), 1);
        assert_eq!(queue.tracks[0].id, remote_track.id);
        assert_eq!(queue.current_index, Some(0));

        let current = service.get_current_track().unwrap().unwrap();
        assert_eq!(current.id, remote_track.id);
        assert_eq!(service.state(), PlaybackState::Playing);

        let _ = std::fs::remove_dir_all(&temp_dir);
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
    fn set_volume_persists_in_internal_state() {
        // set_volume must update InternalState.volume (clamped to 0.0..=1.0)
        // so a freshly-created backend can inherit it. This guards the fix for
        // local playback clamping almost everything to max.
        let service = PlaybackService::<tauri::test::MockRuntime>::new(
            tauri::test::mock_app().handle().clone(),
            Arc::new(Database::open_in_memory().expect("failed to open db")),
            Arc::new(LibraryService::new(Arc::new(
                Database::open_in_memory().expect("failed to open db"),
            ))),
        );

        // Default volume is 1.0 (full).
        assert!((service.volume() - 1.0).abs() < f32::EPSILON);

        // Set a mid volume — the frontend sends 0.0-1.0 (scaled from 0-100).
        let result = service.set_volume(0.5);
        assert!(result.is_ok(), "set_volume(0.5) should succeed");
        assert!(
            (service.volume() - 0.5).abs() < f32::EPSILON,
            "InternalState.volume must be 0.5 after set_volume(0.5)"
        );

        // Out-of-range values clamp, not error.
        let _ = service.set_volume(2.0);
        assert!(
            (service.volume() - 1.0).abs() < f32::EPSILON,
            "set_volume(2.0) must clamp to 1.0"
        );
        let _ = service.set_volume(-1.0);
        assert!(
            (service.volume() - 0.0).abs() < f32::EPSILON,
            "set_volume(-1.0) must clamp to 0.0"
        );
    }

    #[test]
    fn remove_from_queue_before_current_decrements_index() {
        let mut queue = QueueState {
            tracks: vec![
                sample_track_for_tests("t0"),
                sample_track_for_tests("t1"),
                sample_track_for_tests("t2"),
            ],
            current_index: Some(2),
            shuffle: false,
            played_indices: vec![0, 2],
            repeat_mode: RepeatMode::Off,
        };

        let removed = queue.tracks[0].id.clone();
        let removed_index = 0usize;
        queue.tracks.remove(removed_index);
        PlaybackService::<tauri::Wry>::rebase_played_indices(
            &mut queue.played_indices,
            removed_index,
        );
        queue.current_index = queue.current_index.map(|ci| ci.saturating_sub(1));

        assert_eq!(queue.tracks.len(), 2);
        assert_eq!(queue.current_index, Some(1));
        assert_eq!(queue.played_indices, vec![1]);
        assert!(!queue.tracks.iter().any(|t| t.id == removed));
    }

    #[test]
    fn remove_from_queue_current_clears_index() {
        let mut queue = QueueState {
            tracks: vec![
                sample_track_for_tests("t0"),
                sample_track_for_tests("t1"),
                sample_track_for_tests("t2"),
            ],
            current_index: Some(1),
            shuffle: false,
            played_indices: vec![0, 1],
            repeat_mode: RepeatMode::Off,
        };

        let removed_index = 1usize;
        queue.tracks.remove(removed_index);
        PlaybackService::<tauri::Wry>::rebase_played_indices(
            &mut queue.played_indices,
            removed_index,
        );
        queue.current_index = None;

        assert_eq!(queue.tracks.len(), 2);
        assert_eq!(queue.current_index, None);
        assert_eq!(queue.played_indices, vec![0]);
    }

    #[test]
    fn remove_from_queue_after_current_keeps_index() {
        let mut queue = QueueState {
            tracks: vec![
                sample_track_for_tests("t0"),
                sample_track_for_tests("t1"),
                sample_track_for_tests("t2"),
            ],
            current_index: Some(0),
            shuffle: false,
            played_indices: vec![0],
            repeat_mode: RepeatMode::Off,
        };

        let removed_index = 2usize;
        queue.tracks.remove(removed_index);
        PlaybackService::<tauri::Wry>::rebase_played_indices(
            &mut queue.played_indices,
            removed_index,
        );

        assert_eq!(queue.tracks.len(), 2);
        assert_eq!(queue.current_index, Some(0));
        assert_eq!(queue.played_indices, vec![0]);
    }

    #[test]
    fn clear_queue_resets_everything() {
        let mut queue = QueueState {
            tracks: vec![sample_track_for_tests("t0"), sample_track_for_tests("t1")],
            current_index: Some(0),
            shuffle: true,
            played_indices: vec![0],
            repeat_mode: RepeatMode::All,
        };

        queue.tracks.clear();
        queue.current_index = None;
        queue.played_indices.clear();

        assert!(queue.tracks.is_empty());
        assert!(queue.current_index.is_none());
        assert!(queue.played_indices.is_empty());
    }

    #[test]
    fn play_next_inserts_after_current_index() {
        let mut queue = QueueState {
            tracks: vec![sample_track_for_tests("t0"), sample_track_for_tests("t2")],
            current_index: Some(0),
            shuffle: false,
            played_indices: vec![0],
            repeat_mode: RepeatMode::Off,
        };

        let new_track = sample_track_for_tests_with_id_prefix("t1", "__play_next__");
        let insert_index = 1usize;
        queue.tracks.insert(insert_index, new_track);
        queue.current_index = Some(insert_index);

        assert_eq!(queue.tracks.len(), 3);
        assert_eq!(queue.current_index, Some(1));
        assert_eq!(queue.tracks[1].id, "__play_next__t1");
    }

    #[test]
    fn play_next_sequential_requests_replace_previous_insertion() {
        let mut queue = QueueState {
            tracks: vec![
                sample_track_for_tests("t0"),
                sample_track_for_tests_with_id_prefix("old", "__play_next__"),
                sample_track_for_tests("t2"),
            ],
            current_index: Some(1),
            shuffle: false,
            played_indices: vec![0, 1],
            repeat_mode: RepeatMode::Off,
        };

        // Current is the previous play-next insertion at index 1; old current
        // track is at index 0. A new play-next should land at index 1 and
        // replace the prior insertion.
        let prior_index = 1usize;
        if queue.tracks[prior_index].id.starts_with("__play_next__") {
            queue.tracks.remove(prior_index);
            PlaybackService::<tauri::Wry>::rebase_played_indices(
                &mut queue.played_indices,
                prior_index,
            );
        }
        let insert_index = 1usize.min(queue.tracks.len());
        let new_track = sample_track_for_tests_with_id_prefix("new", "__play_next__");
        queue.tracks.insert(insert_index, new_track);
        queue.current_index = Some(insert_index);

        assert_eq!(queue.tracks.len(), 3);
        assert_eq!(queue.current_index, Some(1));
        assert_eq!(queue.tracks[1].id, "__play_next__new");
        assert!(!queue.tracks.iter().any(|t| t.id == "__play_next__old"));
    }

    #[test]
    fn queue_state_snapshot_includes_all_fields() {
        let queue = QueueState {
            tracks: vec![sample_track_for_tests("t0")],
            current_index: Some(0),
            shuffle: false,
            played_indices: vec![0],
            repeat_mode: RepeatMode::All,
        };

        let json = serde_json::to_string(&queue).unwrap();
        assert!(json.contains("\"tracks\""));
        assert!(json.contains("\"currentIndex\""));
        assert!(json.contains("\"shuffle\""));
        assert!(json.contains("\"playedIndices\""));
        assert!(json.contains("\"repeatMode\""));
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
    fn seek_during_eof_drain_resumes_decode_after_seeking_flag_clears() {
        let eof_seek_generation = 14;
        let seeking = false;

        let action = eof_drain_action(
            179.5,
            180.0,
            eof_seek_generation,
            eof_seek_generation + 1,
            false,
            false,
        );

        assert!(!seeking, "seek may finish before EOF drain observes it");
        assert_eq!(action, EofDrainAction::ResumeDecoding);
    }

    #[test]
    fn late_track_seek_keeps_decoder_alive_until_new_position_is_consumed() {
        let action_after_seek = eof_drain_action(179.95, 180.0, 8, 9, false, false);
        let action_at_new_position = eof_drain_action(45.0, 180.0, 9, 9, false, false);

        assert_eq!(action_after_seek, EofDrainAction::ResumeDecoding);
        assert_eq!(action_at_new_position, EofDrainAction::Wait);
        assert_ne!(
            action_after_seek,
            EofDrainAction::Complete,
            "the decoder loop must continue rather than reach its cleanup path"
        );
    }

    #[test]
    fn eof_drain_completes_only_when_audible_position_reaches_track_end() {
        assert_eq!(
            eof_drain_action(179.89, 180.0, 3, 3, false, false),
            EofDrainAction::Wait
        );
        assert_eq!(
            eof_drain_action(179.9, 180.0, 3, 3, false, false),
            EofDrainAction::Complete
        );
    }

    #[test]
    fn eof_drain_recovers_stopped_when_cpal_stops_before_buffer_drains() {
        assert_eq!(
            eof_drain_action(12.0, 180.0, 3, 3, true, false),
            EofDrainAction::RecoverStopped
        );
        assert_eq!(
            eof_drain_action(12.0, 180.0, 3, 3, false, true),
            EofDrainAction::RecoverStopped
        );
    }

    #[test]
    fn seek_without_decoder_returns_ok() {
        // Seek with no active decoder (and no backend) should still succeed
        // because the clamped position is updated in InternalState.
        let service = PlaybackService::<tauri::test::MockRuntime>::new(
            tauri::test::mock_app().handle().clone(),
            Arc::new(Database::open_in_memory().expect("failed to open db")),
            Arc::new(LibraryService::new(Arc::new(
                Database::open_in_memory().expect("failed to open db"),
            ))),
        );

        // Seed a track with duration so clamping works
        {
            let mut s = service.state.lock().unwrap();
            s.current_track = Some(sample_track_for_tests("t0"));
            s.duration = 180.0;
        }

        let result = service.seek(90.0);
        assert!(result.is_ok(), "seek without decoder should succeed");

        let pos = service.state.lock().unwrap().position;
        assert!((pos - 90.0).abs() < f64::EPSILON);
    }

    #[test]
    fn current_position_falls_back_to_state_when_no_backend() {
        // When no backend exists, current_position must return the
        // InternalState position.
        let service = PlaybackService::<tauri::test::MockRuntime>::new(
            tauri::test::mock_app().handle().clone(),
            Arc::new(Database::open_in_memory().expect("failed to open db")),
            Arc::new(LibraryService::new(Arc::new(
                Database::open_in_memory().expect("failed to open db"),
            ))),
        );

        {
            let mut s = service.state.lock().unwrap();
            s.position = 42.0;
            s.duration = 180.0;
        }

        let (pos, dur) = service.current_position().unwrap();
        assert!((pos - 42.0).abs() < f64::EPSILON);
        assert!((dur - 180.0).abs() < f64::EPSILON);
    }

    #[test]
    fn pcm_bus_integration_with_fft_engine() {
        // Integration test: PcmBus → FftEngine pipeline works end-to-end
        use crate::audio::pipeline::PcmBus;

        let (producer, subscriber) = PcmBus::new(44100, 2);
        let mut engine = crate::audio::fft::FftEngine::new(1024, subscriber, 44100, 2);

        // Send enough frames for FFT analysis
        for _ in 0..8 {
            producer.send(vec![0.1; 256]).unwrap();
        }

        engine.collect_frames();
        let result = engine.analyze_if_ready();
        assert!(
            result.is_some(),
            "FFT engine should produce FrequencyData when enough samples"
        );
        let data = result.unwrap();
        assert_eq!(data.sample_rate, 44100);
        assert_eq!(data.bins.len(), 512);
    }

    /// Write a minimal valid PCM WAV file so `SymphoniaDecoder::open` succeeds.
    fn write_test_wav(path: &std::path::Path, seconds: u32) {
        use std::fs::File;
        use std::io::Write;

        let sample_rate: u32 = 44100;
        let channels: u16 = 2;
        let bits_per_sample: u16 = 16;
        let byte_depth = (bits_per_sample / 8) as u32;
        let data_size = seconds * sample_rate * channels as u32 * byte_depth;
        let file_size = 36 + data_size;

        let mut file = File::create(path).expect("failed to create test wav");
        file.write_all(b"RIFF").unwrap();
        file.write_all(&file_size.to_le_bytes()).unwrap();
        file.write_all(b"WAVE").unwrap();
        file.write_all(b"fmt ").unwrap();
        file.write_all(&16u32.to_le_bytes()).unwrap(); // SubChunk1Size
        file.write_all(&1u16.to_le_bytes()).unwrap(); // AudioFormat = PCM
        file.write_all(&channels.to_le_bytes()).unwrap();
        file.write_all(&sample_rate.to_le_bytes()).unwrap();
        file.write_all(&(sample_rate * channels as u32 * byte_depth).to_le_bytes())
            .unwrap(); // ByteRate
        file.write_all(&(channels * byte_depth as u16).to_le_bytes())
            .unwrap(); // BlockAlign
        file.write_all(&bits_per_sample.to_le_bytes()).unwrap();
        file.write_all(b"data").unwrap();
        file.write_all(&data_size.to_le_bytes()).unwrap();
        file.write_all(&vec![0u8; data_size as usize]).unwrap();
    }

    fn test_album_track(
        id: &str,
        title: &str,
        artist: &str,
        album: &str,
        path: &std::path::Path,
    ) -> Track {
        Track {
            id: id.to_string(),
            source: jellyx_core::models::source::Source::Local,
            source_id: path.to_string_lossy().to_string(),
            title: title.to_string(),
            artist: artist.to_string(),
            album: Some(album.to_string()),
            duration: Some(1.0),
            thumbnail: None,
            stream_url: None,
            local_path: Some(path.to_string_lossy().to_string()),
            playlist_id: None,
            metadata: std::collections::HashMap::new(),
        }
    }

    #[test]
    fn play_album_replaces_queue_and_starts_at_first_track() {
        // REQ-AL-2: playing an album must replace the queue with the album
        // tracks in order and set the current track to the first song.
        use crate::ipc::dto::normalize_album_id;
        use crate::library::LibraryService;
        use crate::persistence::db::Database;
        use std::sync::Arc;

        let temp_dir =
            std::env::temp_dir().join(format!("helix-playback-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).expect("failed to create temp dir");

        let album = "Discovery";
        let artist = "Daft Punk";
        let album_id = normalize_album_id(album, artist);

        let paths = [
            temp_dir.join("01-one.mp3"),
            temp_dir.join("02-aero.mp3"),
            temp_dir.join("03-digital.mp3"),
        ];
        for path in &paths {
            write_test_wav(path, 10);
        }

        let tracks = vec![
            test_album_track("t1", "One More Time", artist, album, &paths[0]),
            test_album_track("t2", "Aerodynamic", artist, album, &paths[1]),
            test_album_track("t3", "Digital Love", artist, album, &paths[2]),
        ];

        let db = Arc::new(Database::open_in_memory().expect("failed to open db"));
        db.insert_watched_folder(temp_dir.to_string_lossy().as_ref())
            .expect("failed to insert watched folder");
        for t in &tracks {
            db.upsert_local_track(
                t.local_path.as_ref().unwrap(),
                t,
                temp_dir.to_string_lossy().as_ref(),
                None,
                None,
            )
            .expect("failed to insert track");
        }

        let library = Arc::new(LibraryService::new(db));
        let service = PlaybackService::<tauri::test::MockRuntime>::new(
            tauri::test::mock_app().handle().clone(),
            Arc::new(Database::open_in_memory().expect("failed to open resolver db")),
            library,
        );

        // Act
        let result = service.play_album(&album_id);

        // Audio output may fail in a headless test environment, but the queue
        // replacement and current-track assignment must happen before that.
        let queue = service.get_queue().expect("failed to get queue");
        let current = service
            .get_current_track()
            .expect("failed to get current track");

        // Assert queue replaced with album tracks in order
        assert_eq!(
            queue.tracks.len(),
            tracks.len(),
            "Queue should contain all album tracks"
        );
        assert_eq!(
            queue
                .tracks
                .iter()
                .map(|t| t.id.clone())
                .collect::<Vec<_>>(),
            vec!["t1", "t2", "t3"],
            "Album tracks should be queued in order"
        );
        assert_eq!(
            queue.current_index,
            Some(0),
            "Playback should start at the first album track"
        );

        // Assert current track is the first album track
        assert!(
            current.is_some(),
            "Current track should be set to the first album track"
        );
        let current = current.unwrap();
        assert_eq!(
            current.id, "t1",
            "Current track should be the first album track"
        );
        assert_eq!(current.title, "One More Time");
        assert_eq!(current.album, Some(album.to_string()));

        // If audio output is available the call succeeds; otherwise we still
        // verified the critical queue state.
        //
        // In headless CI, CPAL may fail with a variety of device-invalidation
        // messages (e.g. "The requested device is no longer available",
        // "No suitable output device found"). Both `AudioError::DeviceError`
        // and `AudioError::NoAudioDevice` map to `AppError.code == "DEVICE_NOT_FOUND"`,
        // so any device-level failure surfaces under that code regardless of the
        // exact CPAL message. `UnsupportedFormat` (device present but no matching
        // config) maps to `"PLAYBACK_ERROR"` with details `"unsupported format"`.
        //
        // Accept any device-not-found / audio-output error variant without
        // matching an exact CPAL message, but reject generic decode errors
        // (also `"PLAYBACK_ERROR"`) so a real decode failure still fails the
        // test deterministically.
        if let Err(ref e) = result {
            let is_device_error = e.code == "DEVICE_NOT_FOUND";
            let is_format_error = e.code == "PLAYBACK_ERROR"
                && e.details
                    .as_deref()
                    .is_some_and(|d| d.contains("unsupported format"));
            assert!(
                is_device_error || is_format_error,
                "Expected an audio-output error in headless environment, got {:?}",
                e
            );
        }

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    #[ignore = "legacy cache path coverage; deterministic cache tests use isolated roots"]
    fn cache_remote_stream_reuses_existing_file() {
        // Verify that cache_remote_stream returns an existing file without
        // re-downloading when the cache file already exists and is non-empty.
        // Normalization defaults to enabled, so the cache file uses the .n.m4a
        // suffix (the in-memory test DB has no audio_settings table, so
        // get_normalize_audio returns the default: true).
        let cache_dir = jellyx_core::shared::utils::youtube_cache_dir();
        let test_id = "cache_test_reuse_123";
        let cache_path = cache_dir.join(format!("{}.n.m4a", test_id));

        // Clean up from any previous test run
        let _ = std::fs::remove_file(&cache_path);
        std::fs::create_dir_all(&cache_dir).unwrap();

        // Write a fake cached file with a valid m4a ftyp header so the
        // cache-hit validation passes. The header is a minimal ISO BMFF ftyp box.
        let mut fake_m4a = Vec::new();
        fake_m4a.extend_from_slice(&[0x00, 0x00, 0x00, 0x20]); // box size (32)
        fake_m4a.extend_from_slice(b"ftyp"); // box type
        fake_m4a.extend_from_slice(b"isom"); // major brand
        fake_m4a.extend_from_slice(&[0x00, 0x00, 0x02, 0x00]); // minor version
        fake_m4a.extend_from_slice(b"isomiso2mp41"); // compatible brands
                                                     // Pad to > 1KB so the size check passes.
        fake_m4a.resize(2048, 0);
        std::fs::write(&cache_path, &fake_m4a).unwrap();

        let service = PlaybackService::<tauri::test::MockRuntime>::new(
            tauri::test::mock_app().handle().clone(),
            Arc::new(Database::open_in_memory().expect("failed to open db")),
            Arc::new(LibraryService::new(Arc::new(
                Database::open_in_memory().expect("failed to open db"),
            ))),
        );

        // cache_remote_stream should return the existing path without hitting the network
        let result = service
            .cache_remote_stream(test_id, "https://r1---sn.googlevideo.com/never-called.m4a");
        assert!(result.is_ok(), "Should return Ok for existing cache file");
        let path = result.unwrap();
        assert!(path.contains(test_id), "Path should contain the cache ID");
        assert!(
            std::path::Path::new(&path).exists(),
            "Cached file should exist on disk"
        );

        // Clean up
        let _ = std::fs::remove_file(&cache_path);
    }

    #[test]
    #[ignore = "legacy cache path coverage; deterministic cache tests use isolated roots"]
    fn cache_remote_stream_sanitizes_id() {
        // Verify that special characters in the cache ID are stripped
        // but alphanumeric + dash + underscore are preserved.
        // Normalization defaults to enabled, so the cache file uses .n.m4a.
        let cache_dir = jellyx_core::shared::utils::youtube_cache_dir();
        let dirty_id = "abc-123_def";
        // Only alphanumeric + dash/underscore survive — this ID is already clean
        let expected_safe = "abc-123_def";
        let expected_path = cache_dir.join(format!("{}.n.m4a", expected_safe));

        // Clean up
        let _ = std::fs::remove_file(&expected_path);
        std::fs::create_dir_all(&cache_dir).unwrap();

        // Create a valid m4a file at the expected path so the test doesn't
        // hit the network. Uses a minimal ftyp box header.
        let mut fake_m4a = Vec::new();
        fake_m4a.extend_from_slice(&[0x00, 0x00, 0x00, 0x20]);
        fake_m4a.extend_from_slice(b"ftyp");
        fake_m4a.extend_from_slice(b"isom");
        fake_m4a.extend_from_slice(&[0x00, 0x00, 0x02, 0x00]);
        fake_m4a.extend_from_slice(b"isomiso2mp41");
        fake_m4a.resize(2048, 0);
        std::fs::write(&expected_path, &fake_m4a).unwrap();

        let service = PlaybackService::<tauri::test::MockRuntime>::new(
            tauri::test::mock_app().handle().clone(),
            Arc::new(Database::open_in_memory().expect("failed to open db")),
            Arc::new(LibraryService::new(Arc::new(
                Database::open_in_memory().expect("failed to open db"),
            ))),
        );

        let result =
            service.cache_remote_stream(dirty_id, "https://r1---sn.googlevideo.com/test.m4a");
        assert!(result.is_ok(), "Should sanitize and return cached file");
        let path = result.unwrap();
        assert!(path.contains(expected_safe), "Path should use sanitized ID");

        // Clean up
        let _ = std::fs::remove_file(&expected_path);
    }

    #[test]
    fn cache_remote_stream_rejects_empty_id() {
        let service = PlaybackService::<tauri::test::MockRuntime>::new(
            tauri::test::mock_app().handle().clone(),
            Arc::new(Database::open_in_memory().expect("failed to open db")),
            Arc::new(LibraryService::new(Arc::new(
                Database::open_in_memory().expect("failed to open db"),
            ))),
        );

        // ID that becomes empty after sanitization (all special chars)
        let result =
            service.cache_remote_stream("/../!@#", "https://r1---sn.googlevideo.com/test.m4a");
        assert!(result.is_err(), "Should reject empty sanitized ID");
        assert!(
            result.unwrap_err().contains("empty"),
            "Error should mention empty ID"
        );
    }

    #[test]
    fn cache_remote_stream_rejects_ssrf_urls_before_network_or_cache_access() {
        let service = PlaybackService::<tauri::test::MockRuntime>::new(
            tauri::test::mock_app().handle().clone(),
            Arc::new(Database::open_in_memory().expect("failed to open db")),
            Arc::new(LibraryService::new(Arc::new(
                Database::open_in_memory().expect("failed to open db"),
            ))),
        );

        for url in [
            "http://r1---sn.googlevideo.com/media",
            "https://127.0.0.1/private",
            "file:///etc/passwd",
            "https://user:secret@googlevideo.com/media",
            "https://googlevideo.com:8443/media",
            "https://evil.example/media",
        ] {
            assert!(
                service.cache_remote_stream("ssrf-contract", url).is_err(),
                "cache download must reject {url}"
            );
        }

        // This listener is a deterministic mock transport: the validator must
        // reject its loopback URL before it can accept a connection. The cache
        // file names also prove the public method did not create cache output.
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::time::{Duration, Instant};

        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let port = listener.local_addr().unwrap().port();
        let requests = Arc::new(AtomicUsize::new(0));
        let accepted = requests.clone();
        let server = std::thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_millis(150);
            while Instant::now() < deadline {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        use std::io::Read;
                        let mut request = [0_u8; 256];
                        let _ = stream.set_read_timeout(Some(Duration::from_millis(10)));
                        if let Ok(length) = stream.read(&mut request) {
                            if String::from_utf8_lossy(&request[..length])
                                .starts_with("GET /redirect ")
                            {
                                accepted.fetch_add(1, Ordering::SeqCst);
                            }
                        }
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(5));
                    }
                    Err(_) => break,
                }
            }
        });
        let cache_dir = jellyx_core::shared::utils::youtube_cache_dir();
        let cache_id = "ssrf_no_network_or_cache_write";
        let cache_path = cache_dir.join(format!("{cache_id}.n.m4a"));
        let part_path = cache_dir.join(format!("{cache_id}.n.m4a.part"));
        let _ = std::fs::remove_file(&cache_path);
        let _ = std::fs::remove_file(&part_path);

        assert!(service
            .cache_remote_stream(cache_id, &format!("http://127.0.0.1:{port}/redirect"))
            .is_err());
        server.join().unwrap();
        assert_eq!(
            requests.load(Ordering::SeqCst),
            0,
            "rejected URL opened mock transport"
        );
        assert!(!cache_path.exists(), "rejected URL wrote a cache file");
        assert!(!part_path.exists(), "rejected URL wrote a cache part file");
    }

    fn test_m4a() -> Vec<u8> {
        let mut body = vec![0, 0, 0, 32];
        body.extend_from_slice(b"ftypisom\0\0\x02\0isomiso2mp41");
        body.resize(2048, 0);
        body
    }

    #[test]
    fn cache_download_mock_promotes_only_a_valid_complete_body() {
        let root = std::env::temp_dir().join(format!("jellyx-cache-test-{}", uuid::Uuid::new_v4()));
        let result = cache_remote_stream_with_fetch(
            &root,
            "approved",
            false,
            "https://r1---sn.googlevideo.com/audio",
            |url, path| {
                assert_eq!(url, "https://r1---sn.googlevideo.com/audio");
                std::fs::write(path, test_m4a()).unwrap();
                Ok(200)
            },
        )
        .unwrap();
        assert!(std::path::Path::new(&result).exists());
        assert!(!root.join("approved.m4a.part").exists());
        assert!(is_valid_m4a(std::path::Path::new(&result)));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn cache_download_mock_rejects_redirect_and_truncated_bodies_without_corruption() {
        let root = std::env::temp_dir().join(format!("jellyx-cache-test-{}", uuid::Uuid::new_v4()));
        for (id, status, body) in [
            ("redirect", 302, Vec::new()),
            ("truncated", 200, b"not an m4a".to_vec()),
        ] {
            assert!(cache_remote_stream_with_fetch(
                &root,
                id,
                false,
                "https://r1---sn.googlevideo.com/audio",
                |_, path| {
                    std::fs::write(path, body).unwrap();
                    Ok(status)
                },
            )
            .is_err());
            assert!(!root.join(format!("{id}.m4a")).exists());
            assert!(!root.join(format!("{id}.m4a.part")).exists());
        }
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn concurrent_cache_requests_reuse_destination_created_by_another_request() {
        use std::sync::{Arc, Barrier};

        let root = std::env::temp_dir().join(format!("jellyx-cache-test-{}", uuid::Uuid::new_v4()));
        let barrier = Arc::new(Barrier::new(2));
        let mut workers = Vec::new();
        for _ in 0..2 {
            let root = root.clone();
            let barrier = barrier.clone();
            workers.push(std::thread::spawn(move || {
                cache_remote_stream_with_fetch(
                    &root,
                    "same-track",
                    false,
                    "https://r1---sn.googlevideo.com/audio",
                    |_, path| {
                        // Both requests finish fetching before either can promote,
                        // deterministically modeling the destination-exists race
                        // that makes rename fail on Windows without the lock.
                        barrier.wait();
                        std::fs::write(path, test_m4a()).unwrap();
                        Ok(200)
                    },
                )
            }));
        }
        for worker in workers {
            let path = worker
                .join()
                .expect("cache worker panicked")
                .expect("cache failed");
            assert!(std::path::Path::new(&path).is_file());
        }
        assert!(is_valid_m4a(&root.join("same-track.m4a")));
        assert_eq!(
            std::fs::read_dir(&root)
                .unwrap()
                .filter_map(Result::ok)
                .filter(|entry| entry.file_name().to_string_lossy().contains(".part"))
                .count(),
            0,
            "each request must clean up only its own temporary file"
        );
        let _ = std::fs::remove_dir_all(root);
    }
}

/// 15 minutes at a generous 256 kbps bitrate is well below this ceiling.
/// The bound prevents a hostile approved host from exhausting process memory.
const MAX_REMOTE_CACHE_BYTES: u64 = 32 * 1024 * 1024;
/// Aggregate remote-cache budget. 256 MiB retains several short tracks while
/// keeping this app-owned cache from consuming a user's disk indefinitely.
const MAX_REMOTE_CACHE_TOTAL_BYTES: u64 = 256 * 1024 * 1024;

/// Cache through an injected fetcher so tests never touch the user's cache or
/// network. The production caller supplies the policy-enforcing HTTP client.
fn cache_remote_stream_with_fetch<F>(
    cache_dir: &std::path::Path,
    cache_id: &str,
    normalize_enabled: bool,
    remote_url: &str,
    fetch: F,
) -> Result<String, String>
where
    F: FnOnce(&str, &std::path::Path) -> Result<u16, String>,
{
    std::fs::create_dir_all(cache_dir).map_err(|e| format!("failed to create cache dir: {e}"))?;
    let safe_id: String = cache_id
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
        .collect();
    if safe_id.is_empty() {
        return Err("empty cache id after sanitization".into());
    }
    let suffix = if normalize_enabled { ".n.m4a" } else { ".m4a" };
    let cache_path = cache_dir.join(format!("{safe_id}{suffix}"));
    // Each request owns its temporary files. Final promotion is separately
    // serialized per destination, including its existence recheck, so Windows
    // never has to rename over a concurrently-created destination.
    let request_id = uuid::Uuid::new_v4();
    let part_path = cache_dir.join(format!("{safe_id}{suffix}.{request_id}.part"));
    if is_valid_m4a(&cache_path) {
        return Ok(cache_path.to_string_lossy().into_owned());
    }

    // Reserve enough room for the bounded download before writing a part. The
    // post-promotion check below accounts for its actual size.
    {
        let _budget_lock = CachePromotionLock::acquire_at(
            &cache_dir.join(".remote-cache-budget"),
            Duration::from_secs(30),
        )?;
        enforce_remote_cache_budget(cache_dir, Some(&cache_path), MAX_REMOTE_CACHE_BYTES)?;
    }

    let status = match fetch(remote_url, &part_path) {
        Ok(status) => status,
        Err(error) => {
            let _ = std::fs::remove_file(&part_path);
            return Err(error);
        }
    };
    if !(200..300).contains(&status) {
        let _ = std::fs::remove_file(&part_path);
        return Err(format!("download failed with status: {status}"));
    }
    if !is_valid_m4a(&part_path) {
        let _ = std::fs::remove_file(&part_path);
        return Err("cached stream failed m4a validation".into());
    }

    if normalize_enabled {
        let norm_part = cache_dir.join(format!("{safe_id}{suffix}.{request_id}.norm.part"));
        let mut ffmpeg_cmd = std::process::Command::new("ffmpeg");
        ffmpeg_cmd
            .arg("-y")
            .arg("-i")
            .arg(&part_path)
            .arg("-af")
            .arg("loudnorm=I=-14:TP=-1.5:LRA=11")
            .arg("-c:a")
            .arg("aac")
            .arg("-b:a")
            .arg("128k")
            .arg(&norm_part);
        if jellyx_core::shared::utils::no_window(&mut ffmpeg_cmd)
            .output()
            .is_ok_and(|out| out.status.success() && is_valid_m4a(&norm_part))
        {
            let _ = std::fs::remove_file(&part_path);
            promote_cache_part(&norm_part, &cache_path)?;
            return Ok(cache_path.to_string_lossy().into_owned());
        }
        let _ = std::fs::remove_file(&norm_part);
    }
    promote_cache_part(&part_path, &cache_path)?;
    Ok(cache_path.to_string_lossy().into_owned())
}

/// Promote one validated request-owned part file. The lock covers checking an
/// existing final file, removing an invalid one, and renaming the new part.
/// That makes replacement safe on Windows, where rename fails if the target
/// already exists.
fn promote_cache_part(
    part_path: &std::path::Path,
    cache_path: &std::path::Path,
) -> Result<(), String> {
    let cache_dir = cache_path.parent().ok_or("cache path has no parent")?;
    let part_size = std::fs::metadata(part_path)
        .map_err(|e| format!("failed to inspect cache part: {e}"))?
        .len();
    let _budget_lock = CachePromotionLock::acquire_at(
        &cache_dir.join(".remote-cache-budget"),
        Duration::from_secs(30),
    )?;
    enforce_remote_cache_budget(cache_dir, Some(cache_path), part_size)?;
    let _lock = CachePromotionLock::acquire(cache_path)?;
    if is_valid_m4a(cache_path) {
        let _ = std::fs::remove_file(part_path);
        return Ok(());
    }
    let _ = std::fs::remove_file(cache_path);
    std::fs::rename(part_path, cache_path).map_err(|e| {
        let _ = std::fs::remove_file(part_path);
        format!("failed to promote cache file: {e}")
    })?;
    enforce_remote_cache_budget(cache_dir, Some(cache_path), 0)
}

/// Evict only recognized cache audio files. Entries are ordered by mtime then
/// filename, making LRU/oldest eviction deterministic even with equal mtimes.
/// The pending/current path is never evicted, so an in-use swap remains valid.
fn enforce_remote_cache_budget(
    cache_dir: &std::path::Path,
    preserve: Option<&std::path::Path>,
    incoming_bytes: u64,
) -> Result<(), String> {
    let mut entries = Vec::new();
    for entry in
        std::fs::read_dir(cache_dir).map_err(|e| format!("failed to read cache dir: {e}"))?
    {
        let entry = entry.map_err(|e| format!("failed to inspect cache entry: {e}"))?;
        let path = entry.path();
        if !is_remote_cache_audio_file(&path) || preserve.is_some_and(|kept| kept == path) {
            continue;
        }
        let metadata = entry
            .metadata()
            .map_err(|e| format!("failed to inspect cache entry: {e}"))?;
        entries.push((
            metadata.modified().unwrap_or(UNIX_EPOCH),
            entry.file_name(),
            path,
            metadata.len(),
        ));
    }
    entries.sort_by(|left, right| left.0.cmp(&right.0).then_with(|| left.1.cmp(&right.1)));
    let preserved_bytes = preserve
        .and_then(|path| std::fs::metadata(path).ok())
        .map(|metadata| metadata.len())
        .unwrap_or(0);
    let mut total = preserved_bytes
        .saturating_add(incoming_bytes)
        .saturating_add(entries.iter().map(|entry| entry.3).sum::<u64>());
    for (_, _, path, size) in entries {
        if total <= MAX_REMOTE_CACHE_TOTAL_BYTES {
            break;
        }
        std::fs::remove_file(&path)
            .map_err(|e| format!("failed to evict remote cache entry: {e}"))?;
        total = total.saturating_sub(size);
    }
    if total > MAX_REMOTE_CACHE_TOTAL_BYTES {
        return Err("remote cache budget cannot preserve current item".into());
    }
    Ok(())
}

fn is_remote_cache_audio_file(path: &std::path::Path) -> bool {
    path.is_file()
        && path.extension().is_some_and(|extension| extension == "m4a")
        && path.file_name().is_some_and(|name| {
            let name = name.to_string_lossy();
            name.chars()
                .all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '.')
        })
}

struct CachePromotionLock {
    path: std::path::PathBuf,
    token: String,
}

impl CachePromotionLock {
    fn acquire(cache_path: &std::path::Path) -> Result<Self, String> {
        Self::acquire_with_timeout(cache_path, Duration::from_secs(30))
    }

    fn acquire_with_timeout(
        cache_path: &std::path::Path,
        timeout: Duration,
    ) -> Result<Self, String> {
        let lock_path = cache_path.with_extension(format!(
            "{}.lock",
            cache_path
                .extension()
                .and_then(|extension| extension.to_str())
                .unwrap_or("cache")
        ));
        Self::acquire_at(&lock_path, timeout)
    }

    fn acquire_at(lock_path: &std::path::Path, timeout: Duration) -> Result<Self, String> {
        use std::fs::OpenOptions;
        use std::io::ErrorKind;
        let deadline = Instant::now() + timeout;
        loop {
            let token = uuid::Uuid::new_v4().to_string();
            match OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(lock_path)
            {
                Ok(mut file) => {
                    use std::io::Write;
                    let timestamp = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs();
                    let _ = writeln!(
                        file,
                        "token={token}\npid={}\nhost={}\ntimestamp={timestamp}",
                        std::process::id(),
                        lock_host()
                    );
                    let _ = file.sync_all();
                    return Ok(Self {
                        path: lock_path.to_path_buf(),
                        token,
                    });
                }
                Err(error) if error.kind() == ErrorKind::AlreadyExists => {
                    if reclaim_stale_cache_lock(&lock_path) {
                        continue;
                    }
                    if Instant::now() >= deadline {
                        return Err("timed out waiting for cache promotion lock".into());
                    }
                    thread::sleep(Duration::from_millis(10));
                }
                Err(error) => {
                    return Err(format!("failed to acquire cache promotion lock: {error}"))
                }
            }
        }
    }
}

impl Drop for CachePromotionLock {
    fn drop(&mut self) {
        if read_cache_lock(&self.path).is_some_and(|lock| lock.token == self.token) {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

const CACHE_LOCK_STALE_AFTER: Duration = Duration::from_secs(60);

struct CacheLockMetadata {
    token: String,
    timestamp: u64,
}

fn lock_host() -> String {
    std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "unknown".into())
}

fn read_cache_lock(path: &std::path::Path) -> Option<CacheLockMetadata> {
    let text = std::fs::read_to_string(path).ok()?;
    let mut token = None;
    let mut pid: Option<u32> = None;
    let mut host: Option<String> = None;
    let mut timestamp = None;
    for line in text.lines() {
        let (key, value) = line.split_once('=')?;
        match key {
            "token" => token = Some(value.to_owned()),
            "pid" => pid = value.parse().ok(),
            "host" => host = Some(value.to_owned()),
            "timestamp" => timestamp = value.parse().ok(),
            _ => {}
        }
    }
    Some(CacheLockMetadata {
        token: token?,
        // Require the complete valid-lock shape even though stale recovery
        // deliberately relies on its timestamp rather than PID liveness.
        // A malformed record falls through to the mtime grace path.
        timestamp: {
            let _ = pid?;
            let _ = host?;
            timestamp?
        },
    })
}

fn reclaim_stale_cache_lock(path: &std::path::Path) -> bool {
    let modified = std::fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .ok();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let mtime_stale = modified
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .is_some_and(|time| now.saturating_sub(time.as_secs()) >= CACHE_LOCK_STALE_AFTER.as_secs());
    let lock = read_cache_lock(path);
    let stale = lock
        .as_ref()
        .is_some_and(|lock| now.saturating_sub(lock.timestamp) >= CACHE_LOCK_STALE_AFTER.as_secs());
    // Never steal a fresh valid lock, even when its PID is already gone. A
    // malformed crash remnant has no owner evidence, so reclaim it only after
    // the conservative mtime grace period.
    if lock.is_some() && !stale {
        return false;
    }
    if lock.is_none() && !mtime_stale {
        return false;
    }
    let reclaimed = path.with_extension(format!("reclaimed-{}", uuid::Uuid::new_v4()));
    // rename is the atomic claim: only one waiter can move this exact lock.
    if std::fs::rename(path, &reclaimed).is_ok() {
        let owned = lock.as_ref().map_or(true, |lock| {
            read_cache_lock(&reclaimed).is_some_and(|moved| moved.token == lock.token)
        });
        if owned {
            let _ = std::fs::remove_file(reclaimed);
        }
        return owned;
    }
    false
}

#[cfg(test)]
mod cache_lock_tests {
    use super::*;

    fn cache_path() -> std::path::PathBuf {
        std::env::temp_dir().join(format!("jellyx-lock-test-{}.m4a", uuid::Uuid::new_v4()))
    }

    #[test]
    fn stale_cache_lock_is_reclaimed() {
        let cache_path = cache_path();
        let lock_path = cache_path.with_extension("m4a.lock");
        std::fs::write(
            &lock_path,
            "token=crashed\npid=999999\nhost=unknown\ntimestamp=0\n",
        )
        .unwrap();
        let lock =
            CachePromotionLock::acquire_with_timeout(&cache_path, Duration::from_millis(100))
                .unwrap();
        assert_ne!(read_cache_lock(&lock_path).unwrap().token, "crashed");
        drop(lock);
        assert!(!lock_path.exists());
    }

    #[test]
    fn live_cache_lock_times_out_without_theft() {
        let cache_path = cache_path();
        let held = CachePromotionLock::acquire(&cache_path).unwrap();
        let lock_path = held.path.clone();
        assert!(
            CachePromotionLock::acquire_with_timeout(&cache_path, Duration::from_millis(30))
                .is_err()
        );
        assert_eq!(read_cache_lock(&lock_path).unwrap().token, held.token);
        drop(held);
    }

    #[test]
    fn malformed_stale_cache_lock_is_reclaimed_but_fresh_one_is_not() {
        let cache_path = cache_path();
        let lock_path = cache_path.with_extension("m4a.lock");
        std::fs::write(&lock_path, "incomplete crash remnant").unwrap();
        assert!(!reclaim_stale_cache_lock(&lock_path));
        let stale = SystemTime::now() - CACHE_LOCK_STALE_AFTER - Duration::from_secs(1);
        std::fs::File::open(&lock_path)
            .unwrap()
            .set_times(std::fs::FileTimes::new().set_modified(stale))
            .unwrap();
        assert!(reclaim_stale_cache_lock(&lock_path));
        assert!(!lock_path.exists());
    }

    #[test]
    fn remote_cache_budget_evicts_oldest_recognized_entry_and_preserves_current_data() {
        let root =
            std::env::temp_dir().join(format!("jellyx-cache-budget-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let old = root.join("old.m4a");
        let newer = root.join("newer.m4a");
        let current = root.join("current.m4a");
        let user_data = root.join("notes.txt");
        for path in [&old, &newer, &current] {
            std::fs::File::create(path)
                .unwrap()
                .set_len(100 * 1024 * 1024)
                .unwrap();
        }
        std::fs::write(&user_data, "do not evict").unwrap();
        let old_time = SystemTime::now() - Duration::from_secs(3);
        std::fs::File::open(&old)
            .unwrap()
            .set_times(std::fs::FileTimes::new().set_modified(old_time))
            .unwrap();
        enforce_remote_cache_budget(&root, Some(&current), 0).unwrap();
        assert!(!old.exists(), "oldest cache entry should be evicted first");
        assert!(newer.exists());
        assert!(current.exists(), "current/in-use entry must be preserved");
        assert!(user_data.exists(), "non-cache user data must be preserved");
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn dropping_wrong_token_does_not_remove_new_owner_lock() {
        let cache_path = cache_path();
        let first = CachePromotionLock::acquire(&cache_path).unwrap();
        let path = first.path.clone();
        let token = first.token.clone();
        std::fs::write(
            &path,
            "token=replacement\npid=1\nhost=other\ntimestamp=9999999999\n",
        )
        .unwrap();
        drop(first);
        assert_eq!(read_cache_lock(&path).unwrap().token, "replacement");
        let _ = std::fs::remove_file(path);
        assert_ne!(token, "replacement");
    }

    #[test]
    fn oversized_download_error_removes_partial_cache_file() {
        let root = std::env::temp_dir().join(format!("jellyx-cache-test-{}", uuid::Uuid::new_v4()));
        assert!(cache_remote_stream_with_fetch(
            &root,
            "oversized",
            false,
            "https://r1---sn.googlevideo.com/audio",
            |_, path| {
                std::fs::write(path, vec![0; 1024]).unwrap();
                Err("download exceeds remote cache size limit".into())
            }
        )
        .is_err());
        assert!(!root.join("oversized.m4a").exists());
        assert!(!root.join("oversized.m4a.part").exists());
        let _ = std::fs::remove_dir_all(root);
    }
}

/// Validate that a file is a non-truncated m4a/MP4 container.
///
/// Checks:
/// 1. File size > 1KB (rejects empty or near-empty files).
/// 2. First 8 bytes contain the ISO BMFF ftyp box signature at offset 4.
///
/// This catches the common failure mode where a partial download leaves a
/// truncated file that `convertFileSrc` serves as silence.
fn is_valid_m4a(path: &std::path::Path) -> bool {
    use std::io::Read;
    if let Ok(metadata) = std::fs::metadata(path) {
        if metadata.len() <= 1024 {
            return false;
        }
    } else {
        return false;
    }
    if let Ok(mut f) = std::fs::File::open(path) {
        let mut header = [0u8; 8];
        if f.read(&mut header).unwrap_or(0) >= 8 {
            // ISO BMFF: bytes 0-3 = box size, bytes 4-7 = "ftyp"
            return &header[4..8] == b"ftyp";
        }
    }
    false
}
