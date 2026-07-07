//! Local file source resolver — implements SourceResolver for local files.
//!
//! `LocalResolver` searches and resolves tracks from the local SQLite database
//! (populated by `ScannerService`). It does NOT access the filesystem directly —
//! the scanner handles that. This resolver only queries the database.

use std::sync::Arc;

use crate::errors::types::SourceError;
use crate::models::source::Source;
use crate::models::track::Track;
use crate::persistence::db::Database;
use crate::sources::SourceResolver;

/// Resolver for local file tracks stored in the SQLite database.
pub struct LocalResolver {
    db: Arc<Database>,
}

impl LocalResolver {
    /// Create a new LocalResolver backed by the given Database.
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }
}

impl SourceResolver for LocalResolver {
    fn source_type(&self) -> Source {
        Source::Local
    }

    fn search(&self, query: &str, _offset: usize, _limit: usize) -> Result<Vec<Track>, SourceError> {
        self.db
            .search_local_tracks(query)
            .map_err(|e| SourceError::NetworkError(format!("local search failed: {:?}", e)))
    }

    fn resolve(&self, id: &str) -> Result<Track, SourceError> {
        if let Some(track) = self
            .db
            .get_local_track_by_id(id)
            .map_err(|e| SourceError::ResolveError(format!("local resolve failed: {:?}", e)))?
        {
            return Ok(track);
        }

        self.db
            .get_local_track_by_path(id)
            .map_err(|e| SourceError::ResolveError(format!("local resolve failed: {:?}", e)))?
            .ok_or_else(|| SourceError::ResolveError(format!("local track not found: {}", id)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::source::Source;
    use std::collections::HashMap;

    fn setup_resolver() -> LocalResolver {
        let db = Database::open_in_memory().unwrap();
        LocalResolver::new(Arc::new(db))
    }

    fn sample_track(id: &str, path: &str) -> Track {
        Track {
            id: id.to_string(),
            source: Source::Local,
            source_id: path.to_string(),
            title: format!("Song {}", id),
            artist: "Artist".to_string(),
            album: Some("Album".to_string()),
            duration: Some(180.0),
            thumbnail: None,
            stream_url: None,
            local_path: Some(path.to_string()),
            playlist_id: None,
            metadata: HashMap::new(),
        }
    }

    #[test]
    fn local_resolver_source_type() {
        let resolver = setup_resolver();
        assert_eq!(resolver.source_type(), Source::Local);
    }

    #[test]
    fn local_resolver_search_empty_db() {
        let resolver = setup_resolver();
        let results = resolver.search("anything", 0, 50).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn local_resolver_resolve_nonexistent() {
        let resolver = setup_resolver();
        let result = resolver.resolve("/nonexistent/path.mp3");
        assert!(result.is_err());
    }

    #[test]
    fn local_resolver_search_and_resolve() {
        let db = Database::open_in_memory().unwrap();

        // Insert watched folder first (foreign key constraint)
        db.insert_watched_folder("/music").unwrap();

        let track = sample_track("t1", "/music/song.mp3");
        db.upsert_local_track("/music/song.mp3", &track, "/music", Some("1000"), None)
            .unwrap();

        let resolver = LocalResolver::new(Arc::new(db));

        // Search should find the track via JSON LIKE query
        let results = resolver.search("Song", 0, 50).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "t1");

        // Resolve should return the track by path
        let resolved = resolver.resolve("/music/song.mp3").unwrap();
        assert_eq!(resolved.id, "t1");
        assert_eq!(resolved.local_path, Some("/music/song.mp3".to_string()));
    }

    #[test]
    fn local_resolver_resolves_local_track_by_helix_id() {
        let db = Database::open_in_memory().unwrap();
        db.insert_watched_folder("/music").unwrap();

        let track = sample_track("9f8f1f9e-17d6-4d3f-8a0d-c2f8a7cbe123", "/music/song.mp3");
        db.upsert_local_track("/music/song.mp3", &track, "/music", Some("1000"), None)
            .unwrap();

        let resolver = LocalResolver::new(Arc::new(db));
        let resolved = resolver
            .resolve("9f8f1f9e-17d6-4d3f-8a0d-c2f8a7cbe123")
            .unwrap();

        assert_eq!(resolved.id, "9f8f1f9e-17d6-4d3f-8a0d-c2f8a7cbe123");
        assert_eq!(resolved.local_path, Some("/music/song.mp3".to_string()));
    }
}
