//! Playlist service — desktop adapter over the engine `PlaylistService`.
//!
//! Desktop's `PlaylistService` is a thin adapter that owns an `Arc<Database>`,
//! obtains a `SqliteHandle` from it, and delegates all business logic to
//! [`jellyx_engine::playlist_service::PlaylistService`].
//!
//! Responsibilities kept here (presentation/IPC concerns):
//! - Map engine types to desktop persistence models
//!   (`crate::persistence::models::{UserPlaylist, PlaylistTrackEntry,
//!   ArtistFavorite}`) so the IPC command layer keeps working unchanged.
//! - Map engine `PlaylistServiceError` to `AppError` for consistent IPC
//!   error handling.
//!
//! All business logic (playlist CRUD, artist/folder playlist generation,
//! artist favorites) lives in the engine so both Tauri and Ratatui frontends
//! share a single implementation.

use std::sync::Arc;

use jellyx_core::models::track::Track;
use jellyx_engine::playlist_service::{
    PlaylistService as EnginePlaylistService, PlaylistServiceError,
};

use crate::errors::types::AppError;
use crate::persistence::db::Database;
use crate::persistence::models::{ArtistFavorite, PlaylistTrackEntry, UserPlaylist};

/// Service providing user playlist operations.
///
/// Owns an `Arc<Database>` shared reference so it can be cheaply cloned.
/// All methods are synchronous since SQLite operations are fast.
pub struct PlaylistService {
    /// Kept for direct `Database` access in tests; production code uses
    /// `engine` exclusively.
    #[cfg_attr(not(test), allow(dead_code))]
    db: Arc<Database>,
    engine: EnginePlaylistService,
}

impl PlaylistService {
    /// Create a new PlaylistService backed by the given Database.
    pub fn new(db: Arc<Database>) -> Self {
        let engine = EnginePlaylistService::new(db.handle());
        Self { db, engine }
    }

    /// Create a new user playlist.
    pub fn create_playlist(&self, title: &str) -> Result<UserPlaylist, AppError> {
        self.engine
            .create_playlist(title)
            .map(map_playlist)
            .map_err(AppError::from)
    }

    /// Rename an existing user playlist.
    pub fn rename_playlist(&self, id: &str, title: &str) -> Result<(), AppError> {
        self.engine
            .rename_playlist(id, title)
            .map_err(AppError::from)
    }

    /// Delete a user playlist by ID.
    pub fn delete_playlist(&self, id: &str) -> Result<(), AppError> {
        self.engine.delete_playlist(id).map_err(AppError::from)
    }

    /// Get all user playlists.
    pub fn get_all_playlists(&self) -> Result<Vec<UserPlaylist>, AppError> {
        self.engine
            .get_all_playlists()
            .map(|v| v.into_iter().map(map_playlist).collect())
            .map_err(AppError::from)
    }

    /// Get all playlists generated from a watched folder (parent + children).
    pub fn get_playlists_by_source_folder(
        &self,
        folder_path: &str,
    ) -> Result<Vec<UserPlaylist>, AppError> {
        self.engine
            .get_playlists_by_source_folder(folder_path)
            .map(|v| v.into_iter().map(map_playlist).collect())
            .map_err(AppError::from)
    }

    /// Get all child playlists of a parent playlist.
    pub fn get_child_playlists(&self, parent_id: &str) -> Result<Vec<UserPlaylist>, AppError> {
        self.engine
            .get_child_playlists(parent_id)
            .map(|v| v.into_iter().map(map_playlist).collect())
            .map_err(AppError::from)
    }

    /// Delete all playlists generated from a watched folder (cascade).
    #[allow(dead_code)]
    pub fn delete_playlists_by_source_folder(&self, folder_path: &str) -> Result<u64, AppError> {
        self.engine
            .delete_playlists_by_source_folder(folder_path)
            .map_err(AppError::from)
    }

    /// Get recent playlists (limited).
    pub fn get_recent_playlists(&self, limit: u32) -> Result<Vec<UserPlaylist>, AppError> {
        self.engine
            .get_recent_playlists(limit)
            .map(|v| v.into_iter().map(map_playlist).collect())
            .map_err(AppError::from)
    }

    /// Search playlists by title.
    pub fn search_playlists(&self, query: &str) -> Result<Vec<UserPlaylist>, AppError> {
        self.engine
            .search_playlists(query)
            .map(|v| v.into_iter().map(map_playlist).collect())
            .map_err(AppError::from)
    }

    /// Add a track to a playlist.
    pub fn add_track_to_playlist(&self, playlist_id: &str, track: &Track) -> Result<(), AppError> {
        self.engine
            .add_track_to_playlist(playlist_id, track)
            .map_err(AppError::from)
    }

    /// Remove a track from a playlist by position.
    pub fn remove_track_from_playlist(
        &self,
        playlist_id: &str,
        position: i64,
    ) -> Result<(), AppError> {
        self.engine
            .remove_track_from_playlist(playlist_id, position)
            .map_err(AppError::from)
    }

    /// Get all tracks in a playlist.
    pub fn get_playlist_tracks(
        &self,
        playlist_id: &str,
    ) -> Result<Vec<PlaylistTrackEntry>, AppError> {
        self.engine
            .get_playlist_tracks(playlist_id)
            .map(|v| v.into_iter().map(map_playlist_track).collect())
            .map_err(AppError::from)
    }

    /// Count tracks in a playlist.
    pub fn count_playlist_tracks(&self, playlist_id: &str) -> Result<u32, AppError> {
        self.engine
            .count_playlist_tracks(playlist_id)
            .map_err(AppError::from)
    }

    /// Get up to 4 thumbnail URLs from a playlist's tracks.
    pub fn get_playlist_thumbnails(&self, playlist_id: &str) -> Result<Vec<String>, AppError> {
        self.engine
            .get_playlist_thumbnails(playlist_id)
            .map_err(AppError::from)
    }

    // ── Artist Playlist Generation ──────────────────────────────────────

    /// Generate one playlist per artist from the local track catalog
    /// (idempotent). Returns the playlists that were created or had tracks
    /// added.
    pub fn generate_artist_playlists(&self) -> Result<Vec<UserPlaylist>, AppError> {
        self.engine
            .generate_artist_playlists()
            .map(|v| v.into_iter().map(map_playlist).collect())
            .map_err(AppError::from)
    }

    /// Generate folder-as-playlist hierarchy for a watched folder.
    pub fn generate_folder_playlists(
        &self,
        watched_folder_path: &str,
    ) -> Result<Vec<UserPlaylist>, AppError> {
        self.engine
            .generate_folder_playlists(watched_folder_path)
            .map(|v| v.into_iter().map(map_playlist).collect())
            .map_err(AppError::from)
    }

    // ── Artist Favorites ────────────────────────────────────────────────

    /// Add an artist to favorites.
    ///
    /// `source` defaults to `"local"` for backward compatibility with callers
    /// that predate the source dimension.
    #[allow(dead_code)]
    pub fn add_artist_favorite(
        &self,
        artist_id: &str,
        artist_name: &str,
        thumbnail: Option<&str>,
    ) -> Result<(), AppError> {
        self.add_artist_favorite_with_source(artist_id, "local", artist_name, thumbnail, None)
    }

    /// Add an artist to favorites with an explicit source dimension.
    pub fn add_artist_favorite_with_source(
        &self,
        artist_id: &str,
        source: &str,
        artist_name: &str,
        thumbnail: Option<&str>,
        source_artist_ref: Option<&str>,
    ) -> Result<(), AppError> {
        self.engine
            .add_artist_favorite_with_source(
                artist_id,
                source,
                artist_name,
                thumbnail,
                source_artist_ref,
            )
            .map_err(AppError::from)
    }

    /// Remove an artist from favorites.
    pub fn remove_artist_favorite(
        &self,
        artist_id: &str,
        source: Option<&str>,
    ) -> Result<(), AppError> {
        self.engine
            .remove_artist_favorite(artist_id, source)
            .map_err(AppError::from)
    }

    /// Check if an artist is favorited.
    pub fn is_artist_favorite(
        &self,
        artist_id: &str,
        source: Option<&str>,
    ) -> Result<bool, AppError> {
        self.engine
            .is_artist_favorite(artist_id, source)
            .map_err(AppError::from)
    }

    /// Get all favorited artists.
    pub fn get_all_artist_favorites(&self) -> Result<Vec<ArtistFavorite>, AppError> {
        self.engine
            .get_all_artist_favorites()
            .map(|v| v.into_iter().map(map_artist_favorite).collect())
            .map_err(AppError::from)
    }
}

// ── Error mapping ─────────────────────────────────────────────────────

impl From<PlaylistServiceError> for AppError {
    fn from(error: PlaylistServiceError) -> Self {
        match error {
            PlaylistServiceError::NotFound(msg) => AppError {
                code: "NOT_FOUND".into(),
                details: Some(msg),
            },
            PlaylistServiceError::Persistence(msg) => AppError {
                code: "PERSISTENCE_ERROR".into(),
                details: Some(msg),
            },
            PlaylistServiceError::Deserialization(msg) => AppError {
                code: "PERSISTENCE_ERROR".into(),
                details: Some(format!("failed to deserialize track: {}", msg)),
            },
            PlaylistServiceError::Serialization(msg) => AppError {
                code: "PERSISTENCE_ERROR".into(),
                details: Some(format!("failed to serialize track: {}", msg)),
            },
        }
    }
}

// ── Type mapping ──────────────────────────────────────────────────────

fn map_playlist(p: jellyx_engine::user_playlists::UserPlaylist) -> UserPlaylist {
    UserPlaylist {
        id: p.id,
        title: p.title,
        kind: p.kind,
        source_folder_path: p.source_folder_path,
        parent_playlist_id: p.parent_playlist_id,
        created_at: p.created_at,
        updated_at: p.updated_at,
    }
}

fn map_playlist_track(
    e: jellyx_engine::playlist_service::PlaylistTrackEntry,
) -> PlaylistTrackEntry {
    PlaylistTrackEntry {
        playlist_id: e.playlist_id,
        position: e.position,
        track: e.track,
        added_at: e.added_at,
    }
}

fn map_artist_favorite(row: jellyx_engine::artist_favorites::ArtistFavoriteRow) -> ArtistFavorite {
    ArtistFavorite {
        artist_id: row.artist_id,
        source: row.source,
        artist_name: row.artist_name,
        thumbnail: row.thumbnail,
        source_artist_ref: row.source_artist_ref,
        added_at: row.added_at,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence::db::Database;
    use jellyx_core::models::source::Source;
    use std::collections::HashMap;

    /// Build a local Track with the given id, artist, and file path.
    fn local_track(id: &str, artist: &str, path: &str) -> Track {
        Track {
            id: id.to_string(),
            source: Source::Local,
            source_id: path.to_string(),
            title: format!("Song {}", id),
            artist: artist.to_string(),
            album: None,
            duration: Some(180.0),
            thumbnail: None,
            stream_url: None,
            local_path: Some(path.to_string()),
            playlist_id: None,
            metadata: HashMap::new(),
        }
    }

    /// Insert a local track into the DB via the scanner persistence layer.
    fn seed_local_track(db: &Database, track: &Track, file_path: &str, folder: &str) {
        db.upsert_local_track(file_path, track, folder, Some("1000"), None)
            .unwrap();
    }

    /// Insert a local track with an explicit subfolder_path relative to the
    /// watched folder root.
    fn seed_local_track_in_subfolder(
        db: &Database,
        track: &Track,
        file_path: &str,
        folder: &str,
        subfolder: &str,
    ) {
        db.upsert_local_track(file_path, track, folder, Some("1000"), Some(subfolder))
            .unwrap();
    }

    #[test]
    fn generate_artist_playlists_groups_by_artist() {
        let db = Database::open_in_memory().unwrap();
        db.insert_watched_folder("/music").unwrap();

        seed_local_track(
            &db,
            &local_track("t1", "Daft Punk", "/music/a.mp3"),
            "/music/a.mp3",
            "/music",
        );
        seed_local_track(
            &db,
            &local_track("t2", "Daft Punk", "/music/b.mp3"),
            "/music/b.mp3",
            "/music",
        );
        seed_local_track(
            &db,
            &local_track("t3", "Queen", "/music/c.mp3"),
            "/music/c.mp3",
            "/music",
        );
        seed_local_track(
            &db,
            &local_track("t4", "Queen", "/music/d.mp3"),
            "/music/d.mp3",
            "/music",
        );

        let svc = PlaylistService::new(Arc::new(db));
        let created = svc.generate_artist_playlists().unwrap();

        // Two artists → two playlists.
        assert_eq!(created.len(), 2, "should create one playlist per artist");

        let titles: Vec<String> = created.iter().map(|p| p.title.clone()).collect();
        assert!(titles.contains(&"Daft Punk".to_string()));
        assert!(titles.contains(&"Queen".to_string()));

        // Each playlist should contain the right number of tracks.
        let all = svc.get_all_playlists().unwrap();
        for pl in &all {
            let tracks = svc.get_playlist_tracks(&pl.id).unwrap();
            match pl.title.as_str() {
                "Daft Punk" => assert_eq!(tracks.len(), 2),
                "Queen" => assert_eq!(tracks.len(), 2),
                _ => panic!("unexpected playlist title: {}", pl.title),
            }
        }
    }

    #[test]
    fn generate_artist_playlists_is_idempotent() {
        let db = Database::open_in_memory().unwrap();
        db.insert_watched_folder("/music").unwrap();

        seed_local_track(
            &db,
            &local_track("t1", "Daft Punk", "/music/a.mp3"),
            "/music/a.mp3",
            "/music",
        );
        seed_local_track(
            &db,
            &local_track("t2", "Queen", "/music/c.mp3"),
            "/music/c.mp3",
            "/music",
        );

        let svc = PlaylistService::new(Arc::new(db));

        let first = svc.generate_artist_playlists().unwrap();
        assert_eq!(first.len(), 2);

        let second = svc.generate_artist_playlists().unwrap();
        assert_eq!(
            second.len(),
            0,
            "idempotent re-run should not touch playlists"
        );

        assert_eq!(svc.get_all_playlists().unwrap().len(), 2);
    }

    #[test]
    fn generate_artist_playlists_appends_new_tracks_on_rerun() {
        let db = Arc::new(Database::open_in_memory().unwrap());
        db.insert_watched_folder("/music").unwrap();

        seed_local_track(
            &db,
            &local_track("t1", "Daft Punk", "/music/a.mp3"),
            "/music/a.mp3",
            "/music",
        );

        let svc = PlaylistService::new(db.clone());
        let first = svc.generate_artist_playlists().unwrap();
        assert_eq!(first.len(), 1);

        seed_local_track(
            &db,
            &local_track("t2", "Daft Punk", "/music/b.mp3"),
            "/music/b.mp3",
            "/music",
        );

        let second = svc.generate_artist_playlists().unwrap();
        assert_eq!(
            second.len(),
            1,
            "existing playlist with new tracks should be touched"
        );

        let pl = svc
            .get_all_playlists()
            .unwrap()
            .into_iter()
            .find(|p| p.title == "Daft Punk")
            .unwrap();
        let tracks = svc.get_playlist_tracks(&pl.id).unwrap();
        assert_eq!(tracks.len(), 2, "both tracks should now be in the playlist");
    }

    #[test]
    fn generate_artist_playlists_empty_library_returns_empty() {
        let db = Database::open_in_memory().unwrap();
        let svc = PlaylistService::new(Arc::new(db));

        let created = svc.generate_artist_playlists().unwrap();
        assert!(
            created.is_empty(),
            "empty local library should produce no playlists"
        );
    }

    #[test]
    fn generate_artist_playlists_groups_unknown_artist() {
        let db = Database::open_in_memory().unwrap();
        db.insert_watched_folder("/music").unwrap();

        seed_local_track(
            &db,
            &local_track("t1", "", "/music/a.mp3"),
            "/music/a.mp3",
            "/music",
        );
        seed_local_track(
            &db,
            &local_track("t2", "   ", "/music/b.mp3"),
            "/music/b.mp3",
            "/music",
        );
        seed_local_track(
            &db,
            &local_track("t3", "Queen", "/music/c.mp3"),
            "/music/c.mp3",
            "/music",
        );

        let svc = PlaylistService::new(Arc::new(db));
        let created = svc.generate_artist_playlists().unwrap();

        assert_eq!(created.len(), 2);
        let titles: Vec<String> = created.iter().map(|p| p.title.clone()).collect();
        assert!(titles.contains(&"Unknown Artist".to_string()));
        assert!(titles.contains(&"Queen".to_string()));

        let unknown = created
            .iter()
            .find(|p| p.title == "Unknown Artist")
            .unwrap();
        let tracks = svc.get_playlist_tracks(&unknown.id).unwrap();
        assert_eq!(tracks.len(), 2);
    }

    // ── Folder-as-Playlist Generation tests ────────────────────────────

    #[test]
    fn generate_folder_playlists_creates_parent_and_children_for_subfolders() {
        let db = Arc::new(Database::open_in_memory().unwrap());
        db.insert_watched_folder("/music/Rock").unwrap();

        seed_local_track_in_subfolder(
            &db,
            &local_track("a1", "AC/DC", "/music/Rock/Album1/a1.mp3"),
            "/music/Rock/Album1/a1.mp3",
            "/music/Rock",
            "Album1",
        );
        seed_local_track_in_subfolder(
            &db,
            &local_track("a2", "AC/DC", "/music/Rock/Album1/a2.mp3"),
            "/music/Rock/Album1/a2.mp3",
            "/music/Rock",
            "Album1",
        );
        seed_local_track_in_subfolder(
            &db,
            &local_track("b1", "AC/DC", "/music/Rock/Album2/b1.mp3"),
            "/music/Rock/Album2/b1.mp3",
            "/music/Rock",
            "Album2",
        );
        seed_local_track_in_subfolder(
            &db,
            &local_track("b2", "AC/DC", "/music/Rock/Album2/b2.mp3"),
            "/music/Rock/Album2/b2.mp3",
            "/music/Rock",
            "Album2",
        );

        let svc = PlaylistService::new(db.clone());
        let created = svc.generate_folder_playlists("/music/Rock").unwrap();

        assert_eq!(created.len(), 3, "should create parent + 2 children");

        let parent = created
            .iter()
            .find(|p| p.parent_playlist_id.is_none())
            .expect("parent playlist should exist");
        assert_eq!(parent.title, "Rock");
        assert_eq!(parent.kind, "folder");
        assert_eq!(parent.source_folder_path.as_deref(), Some("/music/Rock"));

        let children: Vec<&UserPlaylist> = created
            .iter()
            .filter(|p| p.parent_playlist_id.is_some())
            .collect();
        assert_eq!(children.len(), 2, "should have 2 children");

        let album1 = children
            .iter()
            .find(|c| c.title == "Rock - Album1")
            .expect("Album1 child should exist");
        assert_eq!(svc.get_playlist_tracks(&album1.id).unwrap().len(), 2);

        let album2 = children
            .iter()
            .find(|c| c.title == "Rock - Album2")
            .expect("Album2 child should exist");
        assert_eq!(svc.get_playlist_tracks(&album2.id).unwrap().len(), 2);

        assert_eq!(svc.get_playlist_tracks(&parent.id).unwrap().len(), 0);
    }

    #[test]
    fn generate_folder_playlists_creates_single_playlist_when_no_subfolders() {
        let db = Arc::new(Database::open_in_memory().unwrap());
        db.insert_watched_folder("/music/Singles").unwrap();

        seed_local_track(
            &db,
            &local_track("s1", "DJ", "/music/Singles/s1.mp3"),
            "/music/Singles/s1.mp3",
            "/music/Singles",
        );
        seed_local_track(
            &db,
            &local_track("s2", "DJ", "/music/Singles/s2.mp3"),
            "/music/Singles/s2.mp3",
            "/music/Singles",
        );

        let svc = PlaylistService::new(db.clone());
        let created = svc.generate_folder_playlists("/music/Singles").unwrap();

        assert_eq!(
            created.len(),
            1,
            "should create only parent with no subfolders"
        );
        assert_eq!(created[0].title, "Singles");
        assert_eq!(created[0].kind, "folder");
        assert_eq!(svc.get_playlist_tracks(&created[0].id).unwrap().len(), 2);
    }

    #[test]
    fn generate_folder_playlists_is_idempotent_on_rerun() {
        let db = Arc::new(Database::open_in_memory().unwrap());
        db.insert_watched_folder("/music/Rock").unwrap();

        seed_local_track_in_subfolder(
            &db,
            &local_track("a1", "AC/DC", "/music/Rock/Album1/a1.mp3"),
            "/music/Rock/Album1/a1.mp3",
            "/music/Rock",
            "Album1",
        );

        let svc = PlaylistService::new(db.clone());
        let first = svc.generate_folder_playlists("/music/Rock").unwrap();
        assert_eq!(first.len(), 2, "first run: parent + 1 child");

        let _ = svc.generate_folder_playlists("/music/Rock").unwrap();
        let all = svc.get_all_playlists().unwrap();
        assert_eq!(all.len(), 2, "no duplicate playlists on re-run");

        let child = all
            .iter()
            .find(|p| p.parent_playlist_id.is_some())
            .expect("child should exist");
        assert_eq!(svc.get_playlist_tracks(&child.id).unwrap().len(), 1);
    }

    #[test]
    fn generate_folder_playlists_appends_new_tracks_on_rerun() {
        let db = Arc::new(Database::open_in_memory().unwrap());
        db.insert_watched_folder("/music/Rock").unwrap();

        seed_local_track_in_subfolder(
            &db,
            &local_track("a1", "AC/DC", "/music/Rock/Album1/a1.mp3"),
            "/music/Rock/Album1/a1.mp3",
            "/music/Rock",
            "Album1",
        );

        let svc = PlaylistService::new(db.clone());
        let _ = svc.generate_folder_playlists("/music/Rock").unwrap();

        seed_local_track_in_subfolder(
            &db,
            &local_track("a2", "AC/DC", "/music/Rock/Album1/a2.mp3"),
            "/music/Rock/Album1/a2.mp3",
            "/music/Rock",
            "Album1",
        );

        let _ = svc.generate_folder_playlists("/music/Rock").unwrap();

        let all = svc.get_all_playlists().unwrap();
        let child = all
            .iter()
            .find(|p| p.parent_playlist_id.is_some())
            .expect("child should exist");
        let tracks = svc.get_playlist_tracks(&child.id).unwrap();
        assert_eq!(tracks.len(), 2, "child should now have 2 tracks");
    }

    #[test]
    fn generate_folder_playlists_empty_folder_returns_empty() {
        let db = Arc::new(Database::open_in_memory().unwrap());
        db.insert_watched_folder("/music/Empty").unwrap();
        let svc = PlaylistService::new(db);
        let created = svc.generate_folder_playlists("/music/Empty").unwrap();
        assert!(
            created.is_empty(),
            "empty folder should produce no playlists"
        );
    }

    // ── Synchronization tests: folder playlists track scanner state ──

    #[test]
    fn generate_folder_playlists_removes_stale_tracks_on_rescan() {
        let db = Arc::new(Database::open_in_memory().unwrap());
        db.insert_watched_folder("/music/Rock").unwrap();

        seed_local_track_in_subfolder(
            &db,
            &local_track("a1", "AC/DC", "/music/Rock/Album1/a1.mp3"),
            "/music/Rock/Album1/a1.mp3",
            "/music/Rock",
            "Album1",
        );
        seed_local_track_in_subfolder(
            &db,
            &local_track("a2", "AC/DC", "/music/Rock/Album1/a2.mp3"),
            "/music/Rock/Album1/a2.mp3",
            "/music/Rock",
            "Album1",
        );

        let svc = PlaylistService::new(db.clone());
        let _ = svc.generate_folder_playlists("/music/Rock").unwrap();

        let all = svc.get_all_playlists().unwrap();
        let child = all
            .iter()
            .find(|p| p.parent_playlist_id.is_some())
            .expect("child should exist");
        assert_eq!(svc.get_playlist_tracks(&child.id).unwrap().len(), 2);

        db.delete_local_track_by_path("/music/Rock/Album1/a1.mp3")
            .unwrap();

        let _ = svc.generate_folder_playlists("/music/Rock").unwrap();

        let tracks = svc.get_playlist_tracks(&child.id).unwrap();
        assert_eq!(tracks.len(), 1, "stale track must be removed after rescan");
        assert_eq!(tracks[0].track.id, "a2");
    }

    #[test]
    fn generate_folder_playlists_reflects_renamed_files_on_rescan() {
        let db = Arc::new(Database::open_in_memory().unwrap());
        db.insert_watched_folder("/music/Rock").unwrap();

        seed_local_track_in_subfolder(
            &db,
            &local_track("a1", "AC/DC", "/music/Rock/Album1/old.mp3"),
            "/music/Rock/Album1/old.mp3",
            "/music/Rock",
            "Album1",
        );

        let svc = PlaylistService::new(db.clone());
        let _ = svc.generate_folder_playlists("/music/Rock").unwrap();

        let all = svc.get_all_playlists().unwrap();
        let child = all
            .iter()
            .find(|p| p.parent_playlist_id.is_some())
            .expect("child should exist");
        let tracks = svc.get_playlist_tracks(&child.id).unwrap();
        assert_eq!(tracks.len(), 1);
        assert_eq!(
            tracks[0].track.local_path.as_deref(),
            Some("/music/Rock/Album1/old.mp3")
        );

        db.delete_local_track_by_path("/music/Rock/Album1/old.mp3")
            .unwrap();
        seed_local_track_in_subfolder(
            &db,
            &local_track("a1", "AC/DC", "/music/Rock/Album1/new.mp3"),
            "/music/Rock/Album1/new.mp3",
            "/music/Rock",
            "Album1",
        );

        let _ = svc.generate_folder_playlists("/music/Rock").unwrap();

        let tracks = svc.get_playlist_tracks(&child.id).unwrap();
        assert_eq!(tracks.len(), 1, "renamed file must replace the old entry");
        assert_eq!(
            tracks[0].track.local_path.as_deref(),
            Some("/music/Rock/Album1/new.mp3")
        );
    }

    #[test]
    fn generate_folder_playlists_preserves_manual_playlists_on_rescan() {
        let db = Arc::new(Database::open_in_memory().unwrap());
        db.insert_watched_folder("/music/Rock").unwrap();

        seed_local_track_in_subfolder(
            &db,
            &local_track("a1", "AC/DC", "/music/Rock/Album1/a1.mp3"),
            "/music/Rock/Album1/a1.mp3",
            "/music/Rock",
            "Album1",
        );

        let svc = PlaylistService::new(db.clone());

        let manual = svc.create_playlist("My Mix").unwrap();
        svc.add_track_to_playlist(
            &manual.id,
            &local_track("m1", "Queen", "/music/Rock/Album1/a1.mp3"),
        )
        .unwrap();

        let _ = svc.generate_folder_playlists("/music/Rock").unwrap();

        let manual_tracks = svc.get_playlist_tracks(&manual.id).unwrap();
        assert_eq!(
            manual_tracks.len(),
            1,
            "manual playlist tracks must NOT be wiped"
        );
        assert_eq!(manual_tracks[0].track.id, "m1");
    }

    // ── Child title uniqueness tests ──────────────────────────────────

    #[test]
    fn generate_folder_playlists_child_titles_use_full_relative_path() {
        let db = Arc::new(Database::open_in_memory().unwrap());
        db.insert_watched_folder("/music/Rock").unwrap();

        seed_local_track_in_subfolder(
            &db,
            &local_track("l1", "AC/DC", "/music/Rock/Album1/Live/l1.mp3"),
            "/music/Rock/Album1/Live/l1.mp3",
            "/music/Rock",
            "Album1/Live",
        );
        seed_local_track_in_subfolder(
            &db,
            &local_track("l2", "AC/DC", "/music/Rock/Album2/Live/l2.mp3"),
            "/music/Rock/Album2/Live/l2.mp3",
            "/music/Rock",
            "Album2/Live",
        );

        let svc = PlaylistService::new(db.clone());
        let created = svc.generate_folder_playlists("/music/Rock").unwrap();

        let titles: Vec<String> = created.iter().map(|p| p.title.clone()).collect();
        assert!(titles.contains(&"Rock - Album1/Live".to_string()));
        assert!(titles.contains(&"Rock - Album2/Live".to_string()));
        assert_eq!(titles.iter().filter(|t| t == &"Rock - Live").count(), 0);

        let _ = svc.generate_folder_playlists("/music/Rock").unwrap();
        let all = svc.get_all_playlists().unwrap();
        assert_eq!(all.len(), 3, "re-run must not duplicate playlists");
    }

    #[test]
    fn generate_folder_playlists_simple_subfolder_uses_segment_as_suffix() {
        let db = Arc::new(Database::open_in_memory().unwrap());
        db.insert_watched_folder("/music/Rock").unwrap();

        seed_local_track_in_subfolder(
            &db,
            &local_track("a1", "AC/DC", "/music/Rock/Album1/a1.mp3"),
            "/music/Rock/Album1/a1.mp3",
            "/music/Rock",
            "Album1",
        );

        let svc = PlaylistService::new(db);
        let created = svc.generate_folder_playlists("/music/Rock").unwrap();

        let child = created
            .iter()
            .find(|p| p.parent_playlist_id.is_some())
            .expect("child should exist");
        assert_eq!(child.title, "Rock - Album1");
    }

    // ── Cascade delete tests ───────────────────────────────────────────

    #[test]
    fn delete_playlists_by_source_folder_removes_folder_playlists_and_preserves_manual() {
        let db = Arc::new(Database::open_in_memory().unwrap());
        db.insert_watched_folder("/music/Rock").unwrap();

        let manual = db.create_playlist("My Mix").unwrap();
        let parent = db
            .create_folder_playlist("Rock", "folder", Some("/music/Rock"), None)
            .unwrap();
        let child = db
            .create_folder_playlist(
                "Rock - Album1",
                "folder",
                Some("/music/Rock"),
                Some(&parent.id),
            )
            .unwrap();

        let svc = PlaylistService::new(db.clone());
        let deleted = svc
            .delete_playlists_by_source_folder("/music/Rock")
            .unwrap();
        assert_eq!(deleted, 2, "should delete parent + child");

        let all = svc.get_all_playlists().unwrap();
        assert_eq!(all.len(), 1, "manual playlist should remain");
        assert_eq!(all[0].id, manual.id);
        assert!(all.iter().all(|p| p.id != child.id));
    }

    // ── Artist Favorite collision tests ───────────────────────────────

    #[test]
    fn add_artist_favorite_with_source_does_not_overwrite_other_source() {
        let db = Database::open_in_memory().unwrap();
        let svc = PlaylistService::new(Arc::new(db));

        svc.add_artist_favorite_with_source(
            "artist:daft-punk",
            "youtube",
            "Daft Punk",
            Some("thumb-a"),
            None,
        )
        .unwrap();
        svc.add_artist_favorite_with_source(
            "artist:daft-punk",
            "local",
            "Daft Punk",
            Some("thumb-b"),
            None,
        )
        .unwrap();

        let all = svc.get_all_artist_favorites().unwrap();
        assert_eq!(all.len(), 2, "two distinct favorites should coexist");

        let yt = all.iter().find(|f| f.source == "youtube").unwrap();
        let lc = all.iter().find(|f| f.source == "local").unwrap();
        assert_eq!(yt.thumbnail.as_deref(), Some("thumb-a"));
        assert_eq!(lc.thumbnail.as_deref(), Some("thumb-b"));
    }

    #[test]
    fn add_artist_favorite_same_source_does_not_overwrite_existing_fields() {
        let db = Database::open_in_memory().unwrap();
        let svc = PlaylistService::new(Arc::new(db));

        svc.add_artist_favorite_with_source(
            "artist:daft-punk",
            "youtube",
            "Daft Punk",
            Some("thumb-a"),
            None,
        )
        .unwrap();
        svc.add_artist_favorite_with_source(
            "artist:daft-punk",
            "youtube",
            "Daft Punk Remixed",
            Some("thumb-b"),
            None,
        )
        .unwrap();

        let all = svc.get_all_artist_favorites().unwrap();
        assert_eq!(
            all.len(),
            1,
            "same (artist_id, source) should not duplicate"
        );
        assert_eq!(all[0].thumbnail.as_deref(), Some("thumb-a"));
        assert_eq!(all[0].artist_name, "Daft Punk");
    }
}
