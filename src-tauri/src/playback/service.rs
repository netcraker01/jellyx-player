//! Playback service facade — the single entry point for all playback operations.
//!
//! Commands delegate to PlaybackService, which internally manages
//! the audio backend, queue state, and event emissions.
//! Method stubs return Ok(()) or sensible defaults until real audio is integrated.

use std::sync::Mutex;

use crate::audio::AudioBackend;
use crate::errors::types::{AppError, PlaybackError, ValidationError};
use crate::models::track::Track;
use crate::playback::events::PlaybackEventEmitter;
use crate::playback::state::{PlaybackState, QueueState};
use crate::sources::youtube::YouTubeResolver;
use crate::sources::SourceResolver;
use tauri::AppHandle;

/// Facade that owns audio backend, queue, and event emitter.
/// All commands go through this service — never directly to AudioBackend.
pub struct PlaybackService {
    audio: Mutex<Box<dyn AudioBackend + Send>>,
    queue: Mutex<QueueState>,
    current_track: Mutex<Option<Track>>,
    emitter: PlaybackEventEmitter,
}

impl PlaybackService {
    /// Create a new PlaybackService wrapping an AudioBackend with an AppHandle for events.
    pub fn new(audio: Box<dyn AudioBackend + Send>, app: AppHandle) -> Self {
        Self {
            audio: Mutex::new(audio),
            queue: Mutex::new(QueueState::default()),
            current_track: Mutex::new(None),
            emitter: PlaybackEventEmitter::new(app),
        }
    }

    /// Start playing a URL. Updates current track and emits track_changed + state_changed.
    pub fn play(&self, url: &str) -> Result<(), AppError> {
        let mut audio = self.audio.lock().map_err(|_| AppError {
            code: "UNKNOWN_ERROR".into(),
            details: Some("mutex lock".into()),
        })?;
        audio.play(url)?;
        // Stub: in a real implementation, resolve the track from the URL
        // and emit track_changed + state_changed events.
        Ok(())
    }

    /// Pause playback. Emits state_changed.
    pub fn pause(&self) -> Result<(), AppError> {
        let mut audio = self.audio.lock().map_err(|_| AppError {
            code: "UNKNOWN_ERROR".into(),
            details: Some("mutex lock".into()),
        })?;
        audio.pause()?;
        let _ = self.emitter.emit_state_changed(&PlaybackState::Paused);
        Ok(())
    }

    /// Resume playback. Emits state_changed.
    pub fn resume(&self) -> Result<(), AppError> {
        let mut audio = self.audio.lock().map_err(|_| AppError {
            code: "UNKNOWN_ERROR".into(),
            details: Some("mutex lock".into()),
        })?;
        audio.resume()?;
        let _ = self.emitter.emit_state_changed(&PlaybackState::Playing);
        Ok(())
    }

    /// Skip to next track in queue. Emits track_changed and state_changed.
    /// Returns PlaybackError::QueueEmpty if queue is empty.
    pub fn next(&self) -> Result<(), AppError> {
        let mut queue = self.queue.lock().map_err(|_| AppError {
            code: "UNKNOWN_ERROR".into(),
            details: Some("mutex lock".into()),
        })?;

        if queue.tracks.is_empty() {
            return Err(PlaybackError::QueueEmpty.into());
        }

        let current = queue.current_index.unwrap_or(0);
        let next_index = current + 1;
        if next_index < queue.tracks.len() {
            queue.current_index = Some(next_index);
            let track = queue.tracks[next_index].clone();
            drop(queue); // Release lock before emitting events

            let _ = self.emitter.emit_track_changed(&track);
            let _ = self.emitter.emit_state_changed(&PlaybackState::Playing);

            // Update current track
            let mut current_track = self.current_track.lock().map_err(|_| AppError {
                code: "UNKNOWN_ERROR".into(),
                details: Some("mutex lock".into()),
            })?;
            *current_track = Some(track);

            // Play the next track URL if available
            if let Some(ref track) = *self.current_track.lock().map_err(|_| AppError {
                code: "UNKNOWN_ERROR".into(),
                details: Some("mutex lock".into()),
            })? {
                if let Some(ref url) = track.stream_url {
                    let mut audio = self.audio.lock().map_err(|_| AppError {
                        code: "UNKNOWN_ERROR".into(),
                        details: Some("mutex lock".into()),
                    })?;
                    audio.play(url)?;
                }
            }
        }

        Ok(())
    }

    /// Go to previous track in queue. Emits track_changed and state_changed.
    pub fn previous(&self) -> Result<(), AppError> {
        let mut queue = self.queue.lock().map_err(|_| AppError {
            code: "UNKNOWN_ERROR".into(),
            details: Some("mutex lock".into()),
        })?;

        if queue.tracks.is_empty() {
            return Err(PlaybackError::QueueEmpty.into());
        }

        let current = queue.current_index.unwrap_or(0);
        if current > 0 {
            queue.current_index = Some(current - 1);
            let track = queue.tracks[current - 1].clone();
            drop(queue);

            let _ = self.emitter.emit_track_changed(&track);
            let _ = self.emitter.emit_state_changed(&PlaybackState::Playing);

            let mut current_track = self.current_track.lock().map_err(|_| AppError {
                code: "UNKNOWN_ERROR".into(),
                details: Some("mutex lock".into()),
            })?;
            *current_track = Some(track);

            if let Some(ref track) = *self.current_track.lock().map_err(|_| AppError {
                code: "UNKNOWN_ERROR".into(),
                details: Some("mutex lock".into()),
            })? {
                if let Some(ref url) = track.stream_url {
                    let mut audio = self.audio.lock().map_err(|_| AppError {
                        code: "UNKNOWN_ERROR".into(),
                        details: Some("mutex lock".into()),
                    })?;
                    audio.play(url)?;
                }
            }
        }

        Ok(())
    }

    /// Seek to a position in the current track.
    pub fn seek(&self, position: f64) -> Result<(), AppError> {
        let mut audio = self.audio.lock().map_err(|_| AppError {
            code: "UNKNOWN_ERROR".into(),
            details: Some("mutex lock".into()),
        })?;
        audio.seek(position)?;
        Ok(())
    }

    /// Set the playback volume.
    pub fn set_volume(&self, level: f32) -> Result<(), AppError> {
        let mut audio = self.audio.lock().map_err(|_| AppError {
            code: "UNKNOWN_ERROR".into(),
            details: Some("mutex lock".into()),
        })?;
        audio.volume(level)?;
        Ok(())
    }

    /// Search for tracks. Currently hardcodes YouTubeResolver for v0.1.
    pub fn search(&self, query: &str) -> Result<Vec<Track>, AppError> {
        if query.trim().is_empty() {
            return Err(ValidationError::EmptyQuery.into());
        }
        let resolver = YouTubeResolver::new();
        resolver.search(query).map_err(Into::into)
    }

    /// Add a track to the queue by ID. Emits queue_updated.
    /// Stub: in v0.1, resolves the track via YouTubeResolver.
    pub fn add_to_queue(&self, track_id: &str) -> Result<(), AppError> {
        if track_id.trim().is_empty() {
            return Err(ValidationError::InvalidInput("track_id must not be empty".into()).into());
        }

        let resolver = YouTubeResolver::new();
        let track = resolver.resolve(track_id)?;

        let mut queue = self.queue.lock().map_err(|_| AppError {
            code: "UNKNOWN_ERROR".into(),
            details: Some("mutex lock".into()),
        })?;

        queue.tracks.push(track.clone());
        // If this is the first track, set current_index to 0
        if queue.current_index.is_none() && queue.tracks.len() == 1 {
            queue.current_index = Some(0);
        }

        let tracks_snapshot = queue.tracks.clone();
        drop(queue);

        let _ = self.emitter.emit_queue_updated(&tracks_snapshot);
        Ok(())
    }

    /// Get the current queue as a list of tracks.
    pub fn get_queue(&self) -> Result<Vec<Track>, AppError> {
        let queue = self.queue.lock().map_err(|_| AppError {
            code: "UNKNOWN_ERROR".into(),
            details: Some("mutex lock".into()),
        })?;
        Ok(queue.tracks.clone())
    }
}