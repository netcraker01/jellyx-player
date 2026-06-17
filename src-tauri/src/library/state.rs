//! Library state — in-memory cache for library data.
//!
//! Provides a Svelte-compatible reactive store for favorites.
//! The Database is the source of truth; this cache avoids
//! redundant IPC round-trips for frequently accessed data.

use std::sync::Mutex;

use crate::persistence::db::Database;
use crate::persistence::models::FavoriteEntry;

/// In-memory cache of library state.
///
/// Intentionally minimal — favorites are the most frequently
/// accessed data. History is queried on demand since it changes
/// every play and is accessed less often.
#[allow(dead_code)]
pub struct LibraryState {
    /// Cached favorites, loaded on startup and updated on add/remove.
    favorites: Mutex<Vec<FavoriteEntry>>,
}

#[allow(dead_code)]
impl LibraryState {
    /// Create a new LibraryState, loading favorites from the database.
    pub fn load(db: &Database) -> Result<Self, crate::errors::types::PersistenceError> {
        let favorites = db.get_favorites()?;
        Ok(Self {
            favorites: Mutex::new(favorites),
        })
    }

    /// Get the cached favorites list.
    pub fn get_favorites(&self) -> Vec<FavoriteEntry> {
        self.favorites.lock().unwrap().clone()
    }

    /// Add a favorite to the cache (call after successful DB insert).
    pub fn add_to_cache(&self, entry: FavoriteEntry) {
        self.favorites.lock().unwrap().insert(0, entry);
    }

    /// Remove a favorite from the cache by track ID (call after successful DB delete).
    pub fn remove_from_cache(&self, track_id: &str) {
        self.favorites
            .lock()
            .unwrap()
            .retain(|f| f.track.id != track_id);
    }

    /// Check if a track is in the favorites cache.
    pub fn is_favorite(&self, track_id: &str) -> bool {
        self.favorites
            .lock()
            .unwrap()
            .iter()
            .any(|f| f.track.id == track_id)
    }

    /// Replace the entire favorites cache (e.g., after bulk reload).
    pub fn reload(&self, entries: Vec<FavoriteEntry>) {
        *self.favorites.lock().unwrap() = entries;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::source::Source;
    use crate::models::track::Track;
    use std::collections::HashMap;

    fn sample_favorite(id: &str) -> FavoriteEntry {
        FavoriteEntry {
            track: Track {
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
            },
            added_at: "2026-01-01 00:00:00".to_string(),
        }
    }

    #[test]
    fn library_state_caches_favorites() {
        let db = Database::open_in_memory().unwrap();
        let state = LibraryState::load(&db).unwrap();
        assert_eq!(state.get_favorites().len(), 0);
    }

    #[test]
    fn add_to_cache_prepends() {
        let db = Database::open_in_memory().unwrap();
        let state = LibraryState::load(&db).unwrap();
        state.add_to_cache(sample_favorite("t1"));
        state.add_to_cache(sample_favorite("t2"));
        let favs = state.get_favorites();
        assert_eq!(favs[0].track.id, "t2", "Most recent first");
        assert_eq!(favs.len(), 2);
    }

    #[test]
    fn remove_from_cache() {
        let db = Database::open_in_memory().unwrap();
        let state = LibraryState::load(&db).unwrap();
        state.add_to_cache(sample_favorite("t1"));
        state.add_to_cache(sample_favorite("t2"));
        state.remove_from_cache("t1");
        let favs = state.get_favorites();
        assert_eq!(favs.len(), 1);
        assert_eq!(favs[0].track.id, "t2");
    }

    #[test]
    fn is_favorite_check() {
        let db = Database::open_in_memory().unwrap();
        let state = LibraryState::load(&db).unwrap();
        assert!(!state.is_favorite("t1"));
        state.add_to_cache(sample_favorite("t1"));
        assert!(state.is_favorite("t1"));
    }
}