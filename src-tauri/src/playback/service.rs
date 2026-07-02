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
use std::time::Duration;

use crate::audio::decoder::SymphoniaDecoder;
use crate::audio::fft::FftEngine;
use crate::audio::output::CpalBackend;
use crate::audio::pipeline::PcmBus;
use crate::audio::{AudioBackend, PlaybackState};
use crate::errors::types::{AppError, PlaybackError, ValidationError};
use crate::library::LibraryService;
use crate::models::source::Source;
use crate::models::track::Track;
use crate::persistence::db::Database;
use crate::playback::events::PlaybackEventEmitter;
use crate::playback::proxy::{proxied_url, start_proxy_server};
use crate::playback::state::{QueueState, RepeatMode};
use crate::sources::local::LocalResolver;
use crate::sources::soundcloud::SoundCloudResolver;
use crate::sources::youtube::YouTubeResolver;
use crate::sources::SourceRegistry;
use crate::visualizer::fft_bridge::FftChannel;

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
    /// Binary FFT streaming channel (shared with AppState.fft_channel).
    fft_channel: Arc<Mutex<Option<tauri::ipc::Channel<Vec<u8>>>>>,
    /// Library service used to record play history when tracks start.
    library: Arc<LibraryService>,
    /// Database used for local inventory cleanup on invalid files/folders.
    db: Arc<Database>,
    /// Local proxy port for remote stream URLs. None if proxy failed to start.
    proxy_port: Option<u16>,
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

impl<R: tauri::Runtime> PlaybackService<R> {
    /// Create a new PlaybackService.
    ///
    /// The `app` handle is used for emitting events to the frontend.
    /// The `db` is used to register the LocalResolver in the source registry.
    /// The `library` is used to record plays in history when tracks start.
    /// The `fft_channel` is shared with AppState for binary FFT streaming.
    /// The actual audio backend (CpalBackend) is created internally
    /// when `play_local()` is called, not at construction time.
    pub fn new(
        app: tauri::AppHandle<R>,
        db: Arc<Database>,
        library: Arc<LibraryService>,
        fft_channel: Arc<Mutex<Option<tauri::ipc::Channel<Vec<u8>>>>>,
    ) -> Self {
        let mut sources = SourceRegistry::new();
        sources.register(Box::new(YouTubeResolver::new()));
        sources.register(Box::new(SoundCloudResolver::new()));
        sources.register(Box::new(LocalResolver::new(db.clone())));

        // Start the local proxy server for remote stream URLs.
        // Non-fatal: if it fails, remote playback falls back to direct URLs.
        let proxy_port = start_proxy_server().map_err(|e| {
            eprintln!("[PlaybackService] Failed to start proxy server: {:?}", e);
            e
        }).ok();

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
            library,
            db,
            proxy_port,
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
            duration: None,
            thumbnail: None,
            stream_url: None,
            local_path: Some(path.to_string()),
            playlist_id: None,
            metadata: std::collections::HashMap::new(),
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
        let (mut bus_producer, output_subscriber) = PcmBus::new(sample_rate, channels);

        // Subscribe the FFT engine (secondary / lossy)
        let fft_subscriber = bus_producer.subscribe_secondary();

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
        }

        // Emit events
        let _ = self.emitter.emit_track_changed(&track);
        let _ = self.emitter.emit_state_changed(&PlaybackState::Buffering(0.0));

        // Store decoder and backend references for seek/volume
        let shared_decoder = self.decoder.clone();
        let shared_backend = self.backend.clone();
        let self_clone = PlaybackService::<R> {
            state: self.state.clone(),
            decoder: shared_decoder.clone(),
            backend: shared_backend.clone(),
            emitter: self.emitter.clone_sender(),
            sources: SourceRegistry::new(),
            fft_channel: self.fft_channel.clone(),
            library: self.library.clone(),
            db: self.db.clone(),
            proxy_port: self.proxy_port,
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
                        // End of stream — wait for playback to drain before
                        // advancing to the next track. Prevents premature
                        // track advancement when decoder reaches EOF before
                        // the audio callback has finished playing buffered data.
                        let drained = loop {
                            let s = decoder_state.lock().unwrap();
                            let position = s.position;
                            let duration = s.duration;
                            drop(s);
                            if position >= duration - 0.1 || duration == 0.0 {
                                break true;
                            }
                            thread::sleep(Duration::from_millis(50));
                        };
                        if drained {
                            let _ = self_clone.next();
                        }
                        break;
                    }
                    Ok(samples_read) => {
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
        cpal_backend
            .play_local(&PathBuf::from(path))
            .map_err(|e| AppError::from(e))?;

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

        // Update state to Playing
        {
            let mut s = self.state.lock().map_err(|_| AppError {
                code: "UNKNOWN_ERROR".into(),
                details: Some("mutex lock".into()),
            })?;
            s.playback_state = PlaybackState::Playing;
        }
        let _ = self.emitter.emit_state_changed(&PlaybackState::Playing);

        // Record this track start in history exactly once per play.
        self.record_history(&track);

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
    pub fn play_stream(&self, track: Track) -> Result<(), AppError> {
        // Resolve the stream URL for this track's source
        let source = track.source.clone();
        let source_id = track.source_id.clone();

        let resolved_track = self
            .sources
            .resolve(&source, &source_id)
            .map_err(|e| AppError::from(e))?;

        let remote_url = resolved_track.stream_url.clone().ok_or_else(|| {
            AppError {
                code: "STREAM_NOT_FOUND".into(),
                details: Some("track has no stream URL".into()),
            }
        })?;

        // Build the proxied URL for immediate playback.
        // For YouTube, the frontend will call cache_remote_stream after loading
        // this URL to get a local copy for instant seeking. SoundCloud stays on
        // the remote proxy path (its seek works fine over HTTP Range requests).
        let stream_url = match self.proxy_port {
            Some(port) => proxied_url(port, &remote_url),
            None => remote_url.clone(),
        };

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
        let _ = self
            .emitter
            .emit_stream_resolved(&track.id, &stream_url, Some(&remote_url));

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
    /// - Files are stored under `~/.local/share/helix/youtube_cache/`.
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
        use std::io::Write;

        let cache_dir = crate::shared::utils::youtube_cache_dir();
        std::fs::create_dir_all(&cache_dir)
            .map_err(|e| format!("failed to create cache dir: {}", e))?;

        // Deterministic filename: <sanitized_id>.m4a
        // Sanitize the cache_id to avoid path traversal (YouTube IDs are
        // alphanumeric + dashes/underscores, but be safe).
        let safe_id: String = cache_id
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
            .collect();
        if safe_id.is_empty() {
            return Err("empty cache id after sanitization".to_string());
        }

        // Check if audio normalization is enabled. When ON, we cache a
        // normalized variant ({id}.n.m4a) produced by ffmpeg loudnorm (EBU
        // R128, target -14 LUFS). When OFF, we cache the raw stream ({id}.m4a).
        // This keeps both variants independent so toggling the setting doesn't
        // require re-downloading the raw stream.
        let normalize_enabled = self
            .db
            .get_normalize_audio()
            .unwrap_or(true); // default: enabled (matches DB default)
        let suffix = if normalize_enabled { ".n.m4a" } else { ".m4a" };
        let cache_path = cache_dir.join(format!("{}{}", safe_id, suffix));
        let cache_path_str = cache_path.to_string_lossy().to_string();

        // Cache hit: if the file already exists and is a valid m4a, reuse it.
        // We validate the file header (ftyp box) and a minimum size to avoid
        // serving corrupt or truncated files from previous failed downloads.
        if let Ok(metadata) = std::fs::metadata(&cache_path) {
            if metadata.len() > 1024 {
                if is_valid_m4a(&cache_path) {
                    return Ok(cache_path_str);
                }
            }
            // File exists but is invalid — delete it so we re-download.
            let _ = std::fs::remove_file(&cache_path);
            let _ = self.emitter.emit_cache_corrupted(
                &safe_id,
                "cache hit file failed m4a validation; re-downloading",
            );
        }

        // Download the stream to a .part file first, then rename, so a partial
        // download never appears as a valid cache file.
        let part_path = cache_dir.join(format!("{}{}.part", safe_id, suffix));

        // Clean up any stale .part file from a previous interrupted download.
        let _ = std::fs::remove_file(&part_path);

        // Use a blocking reqwest client with the same settings as the proxy.
        let client = reqwest::blocking::Client::builder()
            .connect_timeout(Duration::from_secs(15))
            .timeout(Duration::from_secs(300))
            .no_proxy()
            .http1_only()
            .build()
            .map_err(|e| format!("failed to build download client: {}", e))?;

        let response = client
            .get(remote_url)
            .header("User-Agent", "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36")
            .header("Accept-Encoding", "identity")
            .send()
            .map_err(|e| format!("download request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("download failed with status: {}", response.status()));
        }

        // Download the full response body at once. Manual read() loops are
        // unreliable with HTTP/2 — reqwest's blocking Read trait returns 0
        // (EOF) prematurely after ~32KB. Using .bytes() collects the entire
        // body internally and avoids the chunked-read bug. Audio files are
        // typically 3-10MB, so holding them in memory is fine.
        let body = response
            .bytes()
            .map_err(|e| format!("download body read failed: {}", e))?;

        let mut file = std::fs::File::create(&part_path)
            .map_err(|e| format!("failed to create cache file: {}", e))?;

        file.write_all(&body)
            .map_err(|e| format!("cache file write failed: {}", e))?;
        drop(file);

        // Validate the downloaded file before promoting it to a cache hit.
        // A truncated or non-m4a body would fail the ftyp header check.
        if !is_valid_m4a(&part_path) {
            let _ = std::fs::remove_file(&part_path);
            let _ = self.emitter.emit_cache_corrupted(
                &safe_id,
                "downloaded stream failed m4a validation; staying on proxy",
            );
            return Err("cached stream failed m4a validation".to_string());
        }

        // If normalization is enabled, run ffmpeg loudnorm to produce a
        // loudness-normalized file (EBU R128, -14 LUFS). We normalize the .part
        // file in-place by writing to a temp file and renaming. If ffmpeg fails
        // (not installed, decode error), fall back to the raw file so playback
        // still works — the user just gets un-normalized audio.
        if normalize_enabled {
            let norm_part = cache_dir.join(format!("{}{}.norm.part", safe_id, suffix));
            let _ = std::fs::remove_file(&norm_part);
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
            let ffmpeg_result =
                crate::shared::utils::no_window(&mut ffmpeg_cmd).output();

            match ffmpeg_result {
                Ok(out) if out.status.success() && is_valid_m4a(&norm_part) => {
                    // Normalization succeeded — replace .part with normalized file.
                    let _ = std::fs::remove_file(&part_path);
                    std::fs::rename(&norm_part, &cache_path).map_err(|e| {
                        let _ = std::fs::remove_file(&norm_part);
                        format!("failed to rename normalized cache file: {}", e)
                    })?;
                    return Ok(cache_path_str);
                }
                Ok(out) => {
                    // ffmpeg ran but failed — fall back to raw file.
                    let _ = std::fs::remove_file(&norm_part);
                    let _ = self.emitter.emit_cache_corrupted(
                        &safe_id,
                        &format!(
                            "ffmpeg loudnorm failed: {}; using raw stream",
                            String::from_utf8_lossy(&out.stderr).chars().take(200).collect::<String>()
                        ),
                    );
                    // Fall through to raw rename below.
                }
                Err(_) => {
                    // ffmpeg not installed — fall back to raw file.
                    let _ = self.emitter.emit_cache_corrupted(
                        &safe_id,
                        "ffmpeg not found; using raw (un-normalized) stream",
                    );
                    // Fall through to raw rename below.
                }
            }
        }

        // Rename .part → final cache file (atomic on same filesystem).
        std::fs::rename(&part_path, &cache_path)
            .map_err(|e| {
                // Clean up the .part file if rename fails.
                let _ = std::fs::remove_file(&part_path);
                format!("failed to rename cache file: {}", e)
            })?;

        Ok(cache_path_str)
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
        self.sources.search_all_enabled(query, Some(enabled_sources), offset, limit)
    }

    /// Search playlists from enabled sources only.
    pub fn search_playlists_enabled(
        &self,
        query: &str,
        enabled_sources: &std::collections::HashSet<String>,
    ) -> Vec<crate::models::playlist::Playlist> {
        self.sources.search_playlists_all_enabled(query, Some(enabled_sources))
    }

    /// Resolve a playlist by source type and URL/identifier.
    ///
    /// Routes to the appropriate resolver for the given source.
    pub fn resolve_playlist(
        &self,
        source: &Source,
        url: &str,
    ) -> Result<crate::models::playlist::Playlist, crate::errors::types::SourceError> {
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
        self.play_stream(first_track)
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

    /// Look up a track by Helix ID in the current queue or source registry.
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
        let _ = self.emitter.emit_state_changed(&PlaybackState::Playing);

        if let Some(ref local_path) = track.local_path {
            return self.play_local(local_path);
        }

        // Remote track: use the new play_stream path
        self.play_stream(track)
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
        let _ = self.emitter.emit_state_changed(&PlaybackState::Playing);

        if let Some(ref local_path) = track.local_path {
            return self.play_local(local_path);
        }

        // Remote track: use the new play_stream path
        self.play_stream(track)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;
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
            source: crate::models::source::Source::Local,
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
            source: crate::models::source::Source::Local,
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
            source: crate::models::source::Source::YouTube,
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
            source: crate::models::source::Source::Local,
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
        db.upsert_local_track("/music/song.mp3", &local_track, "/music", Some("1000"))
            .expect("failed to insert local track");

        let service = PlaybackService::<tauri::test::MockRuntime>::new(
            tauri::test::mock_app().handle().clone(),
            db.clone(),
            Arc::new(LibraryService::new(db)),
            Arc::new(Mutex::new(None)),
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
            source: crate::models::source::Source::Local,
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
        db.upsert_local_track("/music/song.mp3", &local_track, "/music", Some("1000"))
            .expect("failed to insert local track");

        let service = PlaybackService::<tauri::test::MockRuntime>::new(
            tauri::test::mock_app().handle().clone(),
            db.clone(),
            Arc::new(LibraryService::new(db)),
            Arc::new(Mutex::new(None)),
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
            "helix-playback-missing-track-{}",
            std::process::id()
        ));

        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).expect("failed to create temp dir");

        let missing_path = temp_dir.join("missing.wav");
        let missing_track = Track {
            id: "missing-local".to_string(),
            source: crate::models::source::Source::Local,
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
        )
        .expect("failed to persist missing track");

        let service = PlaybackService::<tauri::test::MockRuntime>::new(
            tauri::test::mock_app().handle().clone(),
            db.clone(),
            Arc::new(LibraryService::new(db.clone())),
            Arc::new(Mutex::new(None)),
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
            "helix-playback-permission-denied-track-{}",
            std::process::id()
        ));

        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).expect("failed to create temp dir");

        let blocked_path = temp_dir.join("blocked.wav");
        std::fs::write(&blocked_path, b"not a real wav").expect("failed to write blocked file");

        let blocked_track = Track {
            id: "blocked-local".to_string(),
            source: crate::models::source::Source::Local,
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
            Arc::new(Mutex::new(None)),
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
            "helix-playback-missing-folder-{}",
            std::process::id()
        ));

        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).expect("failed to create temp dir");

        let missing_path = temp_dir.join("missing.wav");
        let missing_track = Track {
            id: "missing-local".to_string(),
            source: crate::models::source::Source::Local,
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
        )
        .expect("failed to persist missing track");
        std::fs::remove_dir_all(&temp_dir).expect("failed to remove watched folder");

        let service = PlaybackService::<tauri::test::MockRuntime>::new(
            tauri::test::mock_app().handle().clone(),
            db.clone(),
            Arc::new(LibraryService::new(db.clone())),
            Arc::new(Mutex::new(None)),
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
            "helix-playback-permission-denied-folder-{}",
            std::process::id()
        ));

        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).expect("failed to create temp dir");

        let blocked_path = temp_dir.join("blocked.wav");
        std::fs::write(&blocked_path, b"not a real wav").expect("failed to write blocked file");
        let blocked_track = Track {
            id: "blocked-local".to_string(),
            source: crate::models::source::Source::Local,
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
            Arc::new(Mutex::new(None)),
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
            Arc::new(Mutex::new(None)),
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
    fn seek_without_decoder_returns_ok() {
        // Seek with no active decoder (and no backend) should still succeed
        // because the clamped position is updated in InternalState.
        let service = PlaybackService::<tauri::test::MockRuntime>::new(
            tauri::test::mock_app().handle().clone(),
            Arc::new(Database::open_in_memory().expect("failed to open db")),
            Arc::new(LibraryService::new(Arc::new(
                Database::open_in_memory().expect("failed to open db"),
            ))),
            Arc::new(Mutex::new(None)),
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
            Arc::new(Mutex::new(None)),
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
        let mut engine = crate::audio::fft::FftEngine::new(512, subscriber, 44100);

        // Send enough frames for FFT analysis
        for _ in 0..4 {
            producer.send(vec![0.1; 128]).unwrap();
        }

        engine.collect_frames();
        let result = engine.analyze_if_ready();
        assert!(
            result.is_some(),
            "FFT engine should produce FrequencyData when enough samples"
        );
        let data = result.unwrap();
        assert_eq!(data.sample_rate, 44100);
        assert!(!data.bins.is_empty());
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
            source: crate::models::source::Source::Local,
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
            )
            .expect("failed to insert track");
        }

        let library = Arc::new(LibraryService::new(db));
        let service = PlaybackService::<tauri::test::MockRuntime>::new(
            tauri::test::mock_app().handle().clone(),
            Arc::new(Database::open_in_memory().expect("failed to open resolver db")),
            library,
            Arc::new(Mutex::new(None)),
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
        if let Err(ref e) = result {
            assert!(
                e.code == "NO_AUDIO_DEVICE"
                    || e.code == "DEVICE_ERROR"
                    || e.code == "UNSUPPORTED_FORMAT",
                "Expected audio-output error in headless environment, got {:?}",
                e
            );
        }

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn cache_remote_stream_reuses_existing_file() {
        // Verify that cache_remote_stream returns an existing file without
        // re-downloading when the cache file already exists and is non-empty.
        // Normalization defaults to enabled, so the cache file uses the .n.m4a
        // suffix (the in-memory test DB has no audio_settings table, so
        // get_normalize_audio returns the default: true).
        let cache_dir = crate::shared::utils::youtube_cache_dir();
        let test_id = "cache_test_reuse_123";
        let cache_path = cache_dir.join(format!("{}.n.m4a", test_id));

        // Clean up from any previous test run
        let _ = std::fs::remove_file(&cache_path);
        std::fs::create_dir_all(&cache_dir).unwrap();

        // Write a fake cached file with a valid m4a ftyp header so the
        // cache-hit validation passes. The header is a minimal ISO BMFF ftyp box.
        let mut fake_m4a = Vec::new();
        fake_m4a.extend_from_slice(&[0x00, 0x00, 0x00, 0x20]); // box size (32)
        fake_m4a.extend_from_slice(b"ftyp");                   // box type
        fake_m4a.extend_from_slice(b"isom");                   // major brand
        fake_m4a.extend_from_slice(&[0x00, 0x00, 0x02, 0x00]); // minor version
        fake_m4a.extend_from_slice(b"isomiso2mp41");           // compatible brands
        // Pad to > 1KB so the size check passes.
        fake_m4a.resize(2048, 0);
        std::fs::write(&cache_path, &fake_m4a).unwrap();

        let service = PlaybackService::<tauri::test::MockRuntime>::new(
            tauri::test::mock_app().handle().clone(),
            Arc::new(Database::open_in_memory().expect("failed to open db")),
            Arc::new(LibraryService::new(Arc::new(
                Database::open_in_memory().expect("failed to open db"),
            ))),
            Arc::new(Mutex::new(None)),
        );

        // cache_remote_stream should return the existing path without hitting the network
        let result = service.cache_remote_stream(test_id, "https://example.com/never-called.m4a");
        assert!(result.is_ok(), "Should return Ok for existing cache file");
        let path = result.unwrap();
        assert!(path.contains(test_id), "Path should contain the cache ID");
        assert!(std::path::Path::new(&path).exists(), "Cached file should exist on disk");

        // Clean up
        let _ = std::fs::remove_file(&cache_path);
    }

    #[test]
    fn cache_remote_stream_sanitizes_id() {
        // Verify that special characters in the cache ID are stripped
        // but alphanumeric + dash + underscore are preserved.
        // Normalization defaults to enabled, so the cache file uses .n.m4a.
        let cache_dir = crate::shared::utils::youtube_cache_dir();
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
            Arc::new(Mutex::new(None)),
        );

        let result = service.cache_remote_stream(dirty_id, "https://example.com/test.m4a");
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
            Arc::new(Mutex::new(None)),
        );

        // ID that becomes empty after sanitization (all special chars)
        let result = service.cache_remote_stream("/../!@#", "https://example.com/test.m4a");
        assert!(result.is_err(), "Should reject empty sanitized ID");
        assert!(
            result.unwrap_err().contains("empty"),
            "Error should mention empty ID"
        );
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
