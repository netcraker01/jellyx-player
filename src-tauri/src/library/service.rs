//! Library service — CRUD for favorites, history.
//!
//! `LibraryService` wraps the `Database` and provides business-logic-level
//! operations for favorites and history. All methods return `AppError`
//! for consistent IPC error handling.

use std::sync::Arc;

use crate::errors::types::{AppError, LibraryError};
use crate::models::track::Track;
use crate::persistence::db::Database;
use crate::persistence::models::{FavoriteEntry, HistoryEntry};

/// Service providing library operations (favorites, history).
///
/// Owns an `Arc<Database>` shared reference so it can be cheaply cloned
/// if needed in the future. All methods are synchronous since SQLite
/// operations are fast and the WAL mode handles concurrency.
pub struct LibraryService {
    db: Arc<Database>,
}

impl LibraryService {
    /// Create a new LibraryService backed by the given Database.
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    /// Add a track to the user's favorites.
    ///
    /// Returns `LibraryError::AlreadyExists` if the track is already favorited.
    pub fn add_favorite(&self, track: Track) -> Result<(), AppError> {
        if self.db.favorite_exists(&track.id).map_err(AppError::from)? {
            return Err(AppError::from(LibraryError::AlreadyExists(track.id)));
        }
        self.db.insert_favorite(&track).map_err(AppError::from)
    }

    /// Remove a track from favorites by its Helix ID.
    ///
    /// Returns `LibraryError::NotFound` if the track is not in favorites.
    pub fn remove_favorite(&self, track_id: &str) -> Result<(), AppError> {
        let removed = self.db.remove_favorite(track_id).map_err(AppError::from)?;
        if !removed {
            return Err(AppError::from(LibraryError::NotFound(track_id.to_string())));
        }
        Ok(())
    }

    /// Get all favorited tracks, ordered by most recently added first.
    pub fn get_favorites(&self) -> Result<Vec<FavoriteEntry>, AppError> {
        self.db.get_favorites().map_err(AppError::from)
    }

    /// Record a play event in history.
    #[allow(dead_code)]
    pub fn record_play(&self, track: &Track) -> Result<(), AppError> {
        self.db.insert_history(track).map_err(AppError::from)
    }

    /// Toggle a track's favorite state.
    ///
    /// If the track is already favorited it is removed and `false` is returned.
    /// If it is not favorited it is added and `true` is returned.
    pub fn toggle_favorite(&self, track: &Track) -> Result<bool, AppError> {
        if self.db.favorite_exists(&track.id).map_err(AppError::from)? {
            self.db.remove_favorite(&track.id).map_err(AppError::from)?;
            Ok(false)
        } else {
            self.db.insert_favorite(track).map_err(AppError::from)?;
            Ok(true)
        }
    }

    /// Check whether a track is currently favorited by its Helix ID.
    pub fn favorite_exists(&self, track_id: &str) -> Result<bool, AppError> {
        self.db.favorite_exists(track_id).map_err(AppError::from)
    }

    /// Get play history, ordered by most recent first (max 100 entries).
    pub fn get_history(&self) -> Result<Vec<HistoryEntry>, AppError> {
        self.db.get_history().map_err(AppError::from)
    }

    /// Clear all play history.
    pub fn clear_history(&self) -> Result<(), AppError> {
        self.db.clear_history().map_err(AppError::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::source::Source;
    use std::collections::HashMap;

    fn sample_track(id: &str) -> Track {
        Track {
            id: id.to_string(),
            source: Source::YouTube,
            source_id: format!("yt-{}", id),
            title: format!("Song {}", id),
            artist: "Artist".to_string(),
            album: None,
            duration: Some(180.0),
            thumbnail: None,
            stream_url: None,
            local_path: None,
            metadata: HashMap::new(),
        }
    }

    fn setup_service() -> LibraryService {
        let db = Database::open_in_memory().unwrap();
        LibraryService::new(Arc::new(db))
    }

    #[test]
    fn add_and_get_favorites() {
        let svc = setup_service();
        let track = sample_track("t1");
        svc.add_favorite(track).unwrap();

        let favs = svc.get_favorites().unwrap();
        assert_eq!(favs.len(), 1);
        assert_eq!(favs[0].track.id, "t1");
    }

    #[test]
    fn add_duplicate_favorite_returns_already_exists() {
        let svc = setup_service();
        svc.add_favorite(sample_track("t1")).unwrap();
        let result = svc.add_favorite(sample_track("t1"));
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code, "ALREADY_EXISTS");
    }

    #[test]
    fn remove_existing_favorite() {
        let svc = setup_service();
        svc.add_favorite(sample_track("t1")).unwrap();
        svc.remove_favorite("t1").unwrap();
        assert_eq!(svc.get_favorites().unwrap().len(), 0);
    }

    #[test]
    fn remove_nonexistent_favorite_returns_not_found() {
        let svc = setup_service();
        let result = svc.remove_favorite("ghost");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code, "NOT_FOUND");
    }

    #[test]
    fn record_play_and_get_history() {
        let svc = setup_service();
        let track = sample_track("t1");
        svc.record_play(&track).unwrap();

        let history = svc.get_history().unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].track.id, "t1");
    }

    #[test]
    fn repeat_play_creates_multiple_history_entries() {
        let svc = setup_service();
        let track = sample_track("t1");
        svc.record_play(&track).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));
        svc.record_play(&track).unwrap();

        let history = svc.get_history().unwrap();
        assert_eq!(history.len(), 2);
    }

    #[test]
    fn clear_history_removes_all() {
        let svc = setup_service();
        svc.record_play(&sample_track("t1")).unwrap();
        svc.record_play(&sample_track("t2")).unwrap();
        svc.clear_history().unwrap();
        assert_eq!(svc.get_history().unwrap().len(), 0);
    }

    #[test]
    fn empty_favorites_and_history() {
        let svc = setup_service();
        assert_eq!(svc.get_favorites().unwrap().len(), 0);
        assert_eq!(svc.get_history().unwrap().len(), 0);
    }

    #[test]
    fn toggle_favorite_adds_when_not_favorited() {
        let svc = setup_service();
        let track = sample_track("t1");
        let result = svc.toggle_favorite(&track).unwrap();
        assert!(result, "Should return true when adding");
        assert_eq!(svc.get_favorites().unwrap().len(), 1);
    }

    #[test]
    fn toggle_favorite_removes_when_favorited() {
        let svc = setup_service();
        let track = sample_track("t1");
        svc.add_favorite(track.clone()).unwrap();
        let result = svc.toggle_favorite(&track).unwrap();
        assert!(!result, "Should return false when removing");
        assert_eq!(svc.get_favorites().unwrap().len(), 0);
    }

    #[test]
    fn toggle_favorite_twice_returns_to_initial_state() {
        let svc = setup_service();
        let track = sample_track("t1");
        assert!(svc.toggle_favorite(&track).unwrap());
        assert!(!svc.toggle_favorite(&track).unwrap());
        assert!(svc.toggle_favorite(&track).unwrap());
        assert_eq!(svc.get_favorites().unwrap().len(), 1);
    }
}