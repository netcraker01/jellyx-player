//! Tauri-free application boundary shared by Jellyx frontends.
//!
//! Business use cases will move here incrementally. Domain policy remains in
//! `jellyx-core`; platform and presentation adapters remain in frontend crates.

/// Identifies the intentionally dependency-free Unit 1 boundary.
pub const BOUNDARY_ESTABLISHED: bool = true;

pub mod artist_favorites;
pub mod audio_backend;
pub mod dto;
pub mod focus_session;
pub mod history;
pub mod http_stream;
pub mod library_service;
pub mod local_track;
pub mod migration_lock;
pub mod migrations;
pub mod playback_events;
pub mod playback_models;
pub mod playlist_service;
pub mod playlist_tracks;
pub mod preferences;
pub mod queue_controller;
pub mod settings;
pub mod settings_service;
pub mod source_resolver;
pub mod sqlite;
pub mod suggestions;
pub mod updater;
pub mod user_playlists;
pub mod watched_folder;

pub use sqlite::{SqliteIntegrityClassification, SqliteRecoveryError};
