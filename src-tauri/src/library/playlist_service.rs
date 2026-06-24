//! Playlist service — CRUD for user-created local playlists.
//!
//! `PlaylistService` wraps the `Database` and provides business-logic-level
//! operations for user playlists. All methods return `AppError`
//! for consistent IPC error handling.

use std::sync::Arc;

use crate::errors::types::AppError;
use crate::models::track::Track;
use crate::persistence::db::Database;
use crate::persistence::models::{PlaylistTrackEntry, UserPlaylist};

/// Service providing user playlist operations.
///
/// Owns an `Arc<Database>` shared reference so it can be cheaply cloned.
/// All methods are synchronous since SQLite operations are fast.
pub struct PlaylistService {
    db: Arc<Database>,
}

impl PlaylistService {
    /// Create a new PlaylistService backed by the given Database.
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    /// Create a new user playlist.
    pub fn create_playlist(&self, title: &str) -> Result<UserPlaylist, AppError> {
        self.db.create_playlist(title).map_err(AppError::from)
    }

    /// Rename an existing user playlist.
    pub fn rename_playlist(&self, id: &str, title: &str) -> Result<(), AppError> {
        self.db.rename_playlist(id, title).map_err(AppError::from)
    }

    /// Delete a user playlist by ID.
    pub fn delete_playlist(&self, id: &str) -> Result<(), AppError> {
        self.db.delete_playlist(id).map_err(AppError::from)
    }

    /// Get all user playlists.
    pub fn get_all_playlists(&self) -> Result<Vec<UserPlaylist>, AppError> {
        self.db.get_all_playlists().map_err(AppError::from)
    }

    /// Get recent playlists (limited).
    pub fn get_recent_playlists(&self, limit: u32) -> Result<Vec<UserPlaylist>, AppError> {
        self.db.get_recent_playlists(limit).map_err(AppError::from)
    }

    /// Search playlists by title.
    pub fn search_playlists(&self, query: &str) -> Result<Vec<UserPlaylist>, AppError> {
        self.db.search_playlists(query).map_err(AppError::from)
    }

    /// Add a track to a playlist.
    pub fn add_track_to_playlist(&self, playlist_id: &str, track: &Track) -> Result<(), AppError> {
        self.db
            .add_track_to_playlist(playlist_id, track)
            .map_err(AppError::from)
    }

    /// Remove a track from a playlist by position.
    pub fn remove_track_from_playlist(
        &self,
        playlist_id: &str,
        position: i64,
    ) -> Result<(), AppError> {
        self.db
            .remove_track_from_playlist(playlist_id, position)
            .map_err(AppError::from)
    }

    /// Get all tracks in a playlist.
    pub fn get_playlist_tracks(
        &self,
        playlist_id: &str,
    ) -> Result<Vec<PlaylistTrackEntry>, AppError> {
        self.db
            .get_playlist_tracks(playlist_id)
            .map_err(AppError::from)
    }

    /// Count tracks in a playlist.
    pub fn count_playlist_tracks(&self, playlist_id: &str) -> Result<u32, AppError> {
        self.db.count_playlist_tracks(playlist_id).map_err(AppError::from)
    }

    // ── Artist Favorites ────────────────────────────────────────────────

    /// Add an artist to favorites.
    pub fn add_artist_favorite(
        &self,
        artist_id: &str,
        artist_name: &str,
        thumbnail: Option<&str>,
    ) -> Result<(), AppError> {
        self.db
            .add_artist_favorite(artist_id, artist_name, thumbnail)
            .map_err(AppError::from)
    }

    /// Remove an artist from favorites.
    pub fn remove_artist_favorite(&self, artist_id: &str) -> Result<(), AppError> {
        self.db
            .remove_artist_favorite(artist_id)
            .map_err(AppError::from)
    }

    /// Check if an artist is favorited.
    pub fn is_artist_favorite(&self, artist_id: &str) -> Result<bool, AppError> {
        self.db
            .is_artist_favorite(artist_id)
            .map_err(AppError::from)
    }

    /// Get all favorited artists.
    pub fn get_all_artist_favorites(
        &self,
    ) -> Result<Vec<crate::persistence::models::ArtistFavorite>, AppError> {
        self.db
            .get_all_artist_favorites()
            .map_err(AppError::from)
    }
}
