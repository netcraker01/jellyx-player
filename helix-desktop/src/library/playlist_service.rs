//! Playlist service — CRUD for user-created local playlists.
//!
//! `PlaylistService` wraps the `Database` and provides business-logic-level
//! operations for user playlists. All methods return `AppError`
//! for consistent IPC error handling.

use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::Arc;

use crate::errors::types::AppError;
use helix_core::models::track::Track;
use crate::persistence::db::Database;
use crate::persistence::models::{PlaylistTrackEntry, UserPlaylist};

/// Fallback artist name when a track has no artist metadata.
const UNKNOWN_ARTIST: &str = "Unknown Artist";

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

    /// Get all playlists generated from a watched folder (parent + children).
    pub fn get_playlists_by_source_folder(
        &self,
        folder_path: &str,
    ) -> Result<Vec<UserPlaylist>, AppError> {
        self.db
            .get_playlists_by_source_folder(folder_path)
            .map_err(AppError::from)
    }

    /// Get all child playlists of a parent playlist.
    pub fn get_child_playlists(&self, parent_id: &str) -> Result<Vec<UserPlaylist>, AppError> {
        self.db.get_child_playlists(parent_id).map_err(AppError::from)
    }

    /// Delete all playlists generated from a watched folder (cascade).
    #[allow(dead_code)]
    pub fn delete_playlists_by_source_folder(
        &self,
        folder_path: &str,
    ) -> Result<u64, AppError> {
        self.db
            .delete_playlists_by_source_folder(folder_path)
            .map_err(AppError::from)
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

    /// Get up to 4 thumbnail URLs from a playlist's tracks.
    pub fn get_playlist_thumbnails(&self, playlist_id: &str) -> Result<Vec<String>, AppError> {
        self.db.get_playlist_thumbnails(playlist_id).map_err(AppError::from)
    }

    // ── Artist Playlist Generation ──────────────────────────────────────

    /// Generate one playlist per artist from the local track catalog.
    ///
    /// Groups all local tracks by `track.artist` (empty/whitespace-only artists
    /// fall back to "Unknown Artist"). For each artist, if a playlist with that
    /// exact title already exists, it is reused and only new tracks are appended
    /// (idempotent). Otherwise a new playlist is created and all the artist's
    /// tracks are added.
    ///
    /// Returns the list of playlists that were either created or updated
    /// during this run (existing-but-unchanged playlists are not included).
    pub fn generate_artist_playlists(&self) -> Result<Vec<UserPlaylist>, AppError> {
        let local_entries = self.db.get_all_local_tracks()?;

        // Nothing to do if there are no local tracks at all.
        if local_entries.is_empty() {
            return Ok(Vec::new());
        }

        // Group tracks by artist name (cloning the owned Track from each entry).
        let mut by_artist: HashMap<String, Vec<Track>> = HashMap::new();
        for entry in &local_entries {
            let trimmed_artist = entry.track.artist.trim();
            let artist = if trimmed_artist.is_empty() {
                UNKNOWN_ARTIST.to_string()
            } else {
                trimmed_artist.to_string()
            };
            by_artist.entry(artist).or_default().push(entry.track.clone());
        }

        // Load existing playlists once and index their IDs by title for O(1) lookup.
        let existing = self.db.get_all_playlists()?;
        let mut existing_by_title: HashMap<String, UserPlaylist> = existing
            .into_iter()
            .map(|pl| (pl.title.clone(), pl))
            .collect();

        let mut touched: Vec<UserPlaylist> = Vec::new();

        // Artists are processed in a stable alphabetical order so repeated runs
        // produce playlists in a deterministic sequence (useful for tests and UI).
        let mut artists: Vec<String> = by_artist.keys().cloned().collect();
        artists.sort();

        for artist in artists {
            let tracks = by_artist.get(&artist).expect("artist key must exist");

            let playlist = if let Some(pl) = existing_by_title.get(&artist) {
                pl.clone()
            } else {
                let pl = self.db.create_playlist(&artist)?;
                existing_by_title.insert(artist.clone(), pl.clone());
                pl
            };

            // Fetch the tracks already in this playlist so we only append the
            // missing ones. This keeps re-runs cheap and avoids duplicates.
            let existing_tracks = self.db.get_playlist_tracks(&playlist.id)?;
            let existing_ids: HashSet<&str> =
                existing_tracks.iter().map(|e| e.track.id.as_str()).collect();

            let mut added_any = false;
            for track in tracks {
                if existing_ids.contains(track.id.as_str()) {
                    continue;
                }
                self.db.add_track_to_playlist(&playlist.id, track)?;
                added_any = true;
            }

            // Only report playlists that were created or had new tracks added.
            if added_any || !existing_ids.is_empty() {
                // For newly created playlists `existing_ids` is empty, so the
                // `added_any` branch covers them. For existing playlists we only
                // include them if we actually appended tracks.
                if added_any {
                    touched.push(playlist);
                }
            } else {
                // Newly created playlist that somehow has no tracks — still report
                // it once so the caller knows it was created.
                touched.push(playlist);
            }
        }

        Ok(touched)
    }

    // ── Folder-as-Playlist Generation ─────────────────────────────────────

    /// Generate (or update) the folder-as-playlist hierarchy for a watched
    /// folder.
    ///
    /// Groups all local tracks belonging to `watched_folder_path` by their
    /// `subfolder_path` (relative to the watched root). Creates a parent
    /// playlist named after the folder's basename with `kind = "folder"` and
    /// `source_folder_path = watched_folder_path`. For each non-empty
    /// subfolder, creates a child playlist named
    /// `"{parent} - {relative_subfolder_path}"` (e.g. `"Rock - CD 1"` or
    /// `"Rock - Bonus/Live"`) with `parent_playlist_id = parent.id`. Using
    /// the full relative subfolder path — not just the final segment —
    /// guarantees uniqueness for nested subfolders that would otherwise
    /// collide (e.g. `Album1/Live` and `Album2/Live` both becoming
    /// `"Rock - Live"`). When the folder has no subfolders, the parent
    /// playlist contains all tracks directly.
    ///
    /// ## Synchronization with scanner state
    ///
    /// Because `playlist_tracks` stores serialized Track JSON with no
    /// foreign key to `local_tracks`, simply appending new tracks on re-scan
    /// would leave stale entries pointing at removed/moved files forever.
    /// To stay synchronized with the current scanner state, this method
    /// **wipes and rebuilds** the `playlist_tracks` rows for every
    /// folder-generated playlist belonging to `watched_folder_path` on each
    /// successful scan, then re-adds the current tracks from `local_tracks`.
    /// Manual playlists (`kind = "manual"`) and artist-generated playlists
    /// are never touched. This is the simplest safe approach: no diff
    /// computation, no orphaned JSON, and the rebuild is cheap because
    /// folder playlists are bounded by the watched folder's file count.
    pub fn generate_folder_playlists(
        &self,
        watched_folder_path: &str,
    ) -> Result<Vec<UserPlaylist>, AppError> {
        let entries = self.db.get_local_tracks(Some(watched_folder_path))?;
        if entries.is_empty() {
            return Ok(Vec::new());
        }

        // Folder basename is the parent playlist title. We intentionally
        // keep the trailing-slash-free form so "/home/u/Music/Rock" becomes
        // "Rock" and not "".
        let parent_name = Path::new(watched_folder_path)
            .file_name()
            .and_then(|s| s.to_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| watched_folder_path.to_string());

        // Look up any existing folder playlists for this watched folder.
        let existing = self.db.get_playlists_by_source_folder(watched_folder_path)?;
        let parent_playlist: UserPlaylist = if let Some(parent) = existing
            .iter()
            .find(|p| p.parent_playlist_id.is_none())
        {
            parent.clone()
        } else {
            self.db.create_folder_playlist(
                &parent_name,
                "folder",
                Some(watched_folder_path),
                None,
            )?
        };

        // Index existing children by title so we can reuse them idempotently.
        let existing_children_by_title: HashMap<String, UserPlaylist> = existing
            .iter()
            .filter(|p| p.parent_playlist_id.is_some())
            .map(|p| (p.title.clone(), p.clone()))
            .collect();

        // Group tracks by subfolder. Tracks directly in the watched root
        // (subfolder_path None or "") go into the parent playlist directly.
        let mut by_subfolder: HashMap<String, Vec<Track>> = HashMap::new();
        let mut root_tracks: Vec<Track> = Vec::new();
        for entry in entries {
            match entry.subfolder_path.as_deref() {
                None | Some("") => root_tracks.push(entry.track.clone()),
                Some(sub) => by_subfolder
                    .entry(sub.to_string())
                    .or_default()
                    .push(entry.track.clone()),
            }
        }

        // ── Synchronization step ───────────────────────────────────────
        // Wipe `playlist_tracks` for every folder-generated playlist that
        // belongs to this watched folder BEFORE rebuilding. This removes
        // stale entries for files that were deleted/moved since the last
        // scan. Only `kind = "folder"` playlists with a matching
        // `source_folder_path` are touched; manual and artist-generated
        // playlists are preserved.
        self.db.clear_playlist_tracks(&parent_playlist.id)?;
        for child in existing_children_by_title.values() {
            self.db.clear_playlist_tracks(&child.id)?;
        }

        let mut touched: Vec<UserPlaylist> = Vec::new();
        touched.push(parent_playlist.clone());

        // Sort subfolders alphabetically for deterministic ordering.
        let mut subfolders: Vec<String> = by_subfolder.keys().cloned().collect();
        subfolders.sort();
        let has_subfolders = !subfolders.is_empty();

        for sub in &subfolders {
            let tracks = by_subfolder.remove(sub).unwrap_or_default();
            // Child title uses the FULL relative subfolder path (relative to
            // the watched root) as the suffix — not just the final segment.
            // This guarantees uniqueness for nested subfolders: "Album1/Live"
            // becomes "Rock - Album1/Live" and "Album2/Live" becomes
            // "Rock - Album2/Live", so they no longer collide on "Rock - Live".
            let child_title = format!("{} - {}", parent_name, sub);

            let child = if let Some(existing) = existing_children_by_title.get(&child_title) {
                existing.clone()
            } else {
                self.db.create_folder_playlist(
                    &child_title,
                    "folder",
                    Some(watched_folder_path),
                    Some(&parent_playlist.id),
                )?
            };
            touched.push(child.clone());

            // Sort tracks by file path for stable ordering, then add them
            // fresh (the playlist was wiped above, so every track is new).
            let mut sorted = tracks;
            sorted.sort_by(|a, b| {
                a.local_path
                    .as_ref()
                    .unwrap_or(&a.id)
                    .cmp(b.local_path.as_ref().unwrap_or(&b.id))
            });
            self.add_tracks_to_playlist(&child.id, &sorted)?;
        }

        // Rebuild root-level tracks (files directly in the watched root) in
        // the parent playlist. When the folder has no subfolders, ALL tracks
        // are root tracks, so the parent playlist contains everything. When
        // the folder has subfolders AND root tracks, the parent ends up with
        // both its own root tracks and links to its children (handled by the
        // frontend via `parent_playlist_id`).
        if !root_tracks.is_empty() || !has_subfolders {
            let mut sorted = root_tracks;
            sorted.sort_by(|a, b| {
                a.local_path
                    .as_ref()
                    .unwrap_or(&a.id)
                    .cmp(b.local_path.as_ref().unwrap_or(&b.id))
            });
            self.add_tracks_to_playlist(&parent_playlist.id, &sorted)?;
        }

        Ok(touched)
    }

    /// Add tracks to a playlist unconditionally.
    ///
    /// Used by `generate_folder_playlists` after the playlist's
    /// `playlist_tracks` rows have been wiped, so every track in `tracks`
    /// is guaranteed to be new. This is the rebuild counterpart to the
    /// `clear_playlist_tracks` synchronization step.
    fn add_tracks_to_playlist(&self, playlist_id: &str, tracks: &[Track]) -> Result<(), AppError> {
        for track in tracks {
            self.db.add_track_to_playlist(playlist_id, track)?;
        }
        Ok(())
    }

    /// Append tracks to a playlist, skipping those already present.
    ///
    /// Helper used by `generate_folder_playlists` to keep the operation
    /// idempotent: re-running generation only appends new tracks.
    #[allow(dead_code)]
    fn append_missing_tracks(&self, playlist_id: &str, tracks: &[Track]) -> Result<(), AppError> {
        if tracks.is_empty() {
            return Ok(());
        }
        let existing = self.db.get_playlist_tracks(playlist_id)?;
        let existing_ids: HashSet<&str> = existing.iter().map(|e| e.track.id.as_str()).collect();
        for track in tracks {
            if existing_ids.contains(track.id.as_str()) {
                continue;
            }
            self.db.add_track_to_playlist(playlist_id, track)?;
        }
        Ok(())
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
        self.add_artist_favorite_with_source(
            artist_id,
            "local",
            artist_name,
            thumbnail,
            None,
        )
    }

    /// Add an artist to favorites with an explicit source dimension.
    ///
    /// Uses `INSERT ... ON CONFLICT(artist_id, source) DO NOTHING` so the
    /// first-seen `thumbnail` and `artist_name` are preserved when the same
    /// `(artist_id, source)` is favorited again.
    pub fn add_artist_favorite_with_source(
        &self,
        artist_id: &str,
        source: &str,
        artist_name: &str,
        thumbnail: Option<&str>,
        source_artist_ref: Option<&str>,
    ) -> Result<(), AppError> {
        self.db
            .add_artist_favorite(
                artist_id,
                source,
                artist_name,
                thumbnail,
                source_artist_ref,
            )
            .map_err(AppError::from)
    }

    /// Remove an artist from favorites.
    ///
    /// Pass `source = Some("youtube")` to remove only the YouTube favorite;
    /// pass `None` to remove every favorite for that artist across all sources.
    pub fn remove_artist_favorite(
        &self,
        artist_id: &str,
        source: Option<&str>,
    ) -> Result<(), AppError> {
        self.db
            .remove_artist_favorite(artist_id, source)
            .map_err(AppError::from)
    }

    /// Check if an artist is favorited.
    ///
    /// When `source` is `None`, returns true if any source has a favorite for
    /// this artist. When `source` is provided, returns true only if that
    /// exact `(artist_id, source)` pair exists.
    pub fn is_artist_favorite(
        &self,
        artist_id: &str,
        source: Option<&str>,
    ) -> Result<bool, AppError> {
        self.db
            .is_artist_favorite(artist_id, source)
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

#[cfg(test)]
mod tests {
    use super::*;
    use helix_core::models::source::Source;
    use crate::persistence::db::Database;
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
        db.upsert_local_track(
            file_path,
            track,
            folder,
            Some("1000"),
            Some(subfolder),
        )
        .unwrap();
    }

    #[test]
    fn generate_artist_playlists_groups_by_artist() {
        let db = Database::open_in_memory().unwrap();
        db.insert_watched_folder("/music").unwrap();

        seed_local_track(&db, &local_track("t1", "Daft Punk", "/music/a.mp3"), "/music/a.mp3", "/music");
        seed_local_track(&db, &local_track("t2", "Daft Punk", "/music/b.mp3"), "/music/b.mp3", "/music");
        seed_local_track(&db, &local_track("t3", "Queen", "/music/c.mp3"), "/music/c.mp3", "/music");
        seed_local_track(&db, &local_track("t4", "Queen", "/music/d.mp3"), "/music/d.mp3", "/music");

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

        seed_local_track(&db, &local_track("t1", "Daft Punk", "/music/a.mp3"), "/music/a.mp3", "/music");
        seed_local_track(&db, &local_track("t2", "Queen", "/music/c.mp3"), "/music/c.mp3", "/music");

        let svc = PlaylistService::new(Arc::new(db));

        // First run: 2 playlists created.
        let first = svc.generate_artist_playlists().unwrap();
        assert_eq!(first.len(), 2);

        // Second run: no new tracks, so nothing should be touched.
        let second = svc.generate_artist_playlists().unwrap();
        assert_eq!(second.len(), 0, "idempotent re-run should not create or modify playlists");

        // Total playlist count stays at 2.
        let all = svc.get_all_playlists().unwrap();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn generate_artist_playlists_appends_new_tracks_on_rerun() {
        let db = Arc::new(Database::open_in_memory().unwrap());
        db.insert_watched_folder("/music").unwrap();

        seed_local_track(&db, &local_track("t1", "Daft Punk", "/music/a.mp3"), "/music/a.mp3", "/music");

        let svc = PlaylistService::new(db.clone());
        let first = svc.generate_artist_playlists().unwrap();
        assert_eq!(first.len(), 1);

        // Add a new Daft Punk track after the first generation.
        seed_local_track(&db, &local_track("t2", "Daft Punk", "/music/b.mp3"), "/music/b.mp3", "/music");

        // Re-run: the existing Daft Punk playlist should be reused and the new
        // track appended. The playlist should be reported as touched.
        let second = svc.generate_artist_playlists().unwrap();
        assert_eq!(second.len(), 1, "existing playlist with new tracks should be touched");

        let pl = svc.get_all_playlists().unwrap().into_iter().find(|p| p.title == "Daft Punk").unwrap();
        let tracks = svc.get_playlist_tracks(&pl.id).unwrap();
        assert_eq!(tracks.len(), 2, "both tracks should now be in the playlist");
    }

    #[test]
    fn generate_artist_playlists_empty_library_returns_empty() {
        let db = Database::open_in_memory().unwrap();
        let svc = PlaylistService::new(Arc::new(db));

        let created = svc.generate_artist_playlists().unwrap();
        assert!(created.is_empty(), "empty local library should produce no playlists");
    }

    #[test]
    fn generate_artist_playlists_groups_unknown_artist() {
        let db = Database::open_in_memory().unwrap();
        db.insert_watched_folder("/music").unwrap();

        // Track with empty artist string → should become "Unknown Artist".
        seed_local_track(&db, &local_track("t1", "", "/music/a.mp3"), "/music/a.mp3", "/music");
        // Track with whitespace-only artist → also "Unknown Artist".
        seed_local_track(&db, &local_track("t2", "   ", "/music/b.mp3"), "/music/b.mp3", "/music");
        // A normal artist.
        seed_local_track(&db, &local_track("t3", "Queen", "/music/c.mp3"), "/music/c.mp3", "/music");

        let svc = PlaylistService::new(Arc::new(db));
        let created = svc.generate_artist_playlists().unwrap();

        // Two groups: "Unknown Artist" and "Queen".
        assert_eq!(created.len(), 2);
        let titles: Vec<String> = created.iter().map(|p| p.title.clone()).collect();
        assert!(titles.contains(&"Unknown Artist".to_string()));
        assert!(titles.contains(&"Queen".to_string()));

        // Unknown Artist playlist should have 2 tracks.
        let unknown = created.iter().find(|p| p.title == "Unknown Artist").unwrap();
        let tracks = svc.get_playlist_tracks(&unknown.id).unwrap();
        assert_eq!(tracks.len(), 2);
    }

    // ── Folder-as-Playlist Generation tests ────────────────────────────

    #[test]
    fn generate_folder_playlists_creates_parent_and_children_for_subfolders() {
        let db = Arc::new(Database::open_in_memory().unwrap());
        db.insert_watched_folder("/music/Rock").unwrap();

        // Two subfolders, each with two tracks.
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

        // Parent + 2 children = 3 playlists.
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
        for child in &children {
            assert_eq!(child.parent_playlist_id.as_deref(), Some(parent.id.as_str()));
            assert_eq!(child.kind, "folder");
            assert_eq!(child.source_folder_path.as_deref(), Some("/music/Rock"));
        }

        // Each child should contain only its own 2 tracks.
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

        // Parent should have no tracks (subfolders absorb all tracks).
        assert_eq!(svc.get_playlist_tracks(&parent.id).unwrap().len(), 0);
    }

    #[test]
    fn generate_folder_playlists_creates_single_playlist_when_no_subfolders() {
        let db = Arc::new(Database::open_in_memory().unwrap());
        db.insert_watched_folder("/music/Singles").unwrap();

        // All tracks at root level (no subfolder).
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

        // Only the parent, no children.
        assert_eq!(created.len(), 1, "should create only parent with no subfolders");
        assert_eq!(created[0].title, "Singles");
        assert_eq!(created[0].kind, "folder");

        // Parent should contain all tracks.
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

        let second = svc.generate_folder_playlists("/music/Rock").unwrap();
        // Should still report parent + child (existing) but no duplicates.
        let all = svc.get_all_playlists().unwrap();
        assert_eq!(all.len(), 2, "no duplicate playlists on re-run");

        // Child should still contain only 1 track (no duplication).
        let child = all
            .iter()
            .find(|p| p.parent_playlist_id.is_some())
            .expect("child should exist");
        assert_eq!(svc.get_playlist_tracks(&child.id).unwrap().len(), 1);

        // Second run should not have touched parent (it's already up to date).
        let _ = second;
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

        // Add a second track to the same subfolder.
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
        assert!(created.is_empty(), "empty folder should produce no playlists");
    }

    // ── Synchronization tests: folder playlists track scanner state ──

    #[test]
    fn generate_folder_playlists_removes_stale_tracks_on_rescan() {
        let db = Arc::new(Database::open_in_memory().unwrap());
        db.insert_watched_folder("/music/Rock").unwrap();

        // Initial scan: two tracks in Album1.
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
        assert_eq!(
            svc.get_playlist_tracks(&child.id).unwrap().len(),
            2,
            "initial state: 2 tracks"
        );

        // Simulate a rescan where a1 was deleted from disk. The scanner
        // would have removed the `local_tracks` row for it.
        db.delete_local_track_by_path("/music/Rock/Album1/a1.mp3")
            .unwrap();

        // Re-run generation: it must wipe and rebuild from the current
        // local_tracks, so the stale a1 entry must NOT survive in the
        // auto-generated playlist.
        let _ = svc.generate_folder_playlists("/music/Rock").unwrap();

        let tracks = svc.get_playlist_tracks(&child.id).unwrap();
        assert_eq!(
            tracks.len(),
            1,
            "stale track must be removed from auto-generated playlist after rescan"
        );
        assert_eq!(tracks[0].track.id, "a2");
    }

    #[test]
    fn generate_folder_playlists_reflects_renamed_files_on_rescan() {
        let db = Arc::new(Database::open_in_memory().unwrap());
        db.insert_watched_folder("/music/Rock").unwrap();

        // Initial state: one track at Album1/old.mp3.
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
        assert_eq!(tracks[0].track.local_path.as_deref(), Some("/music/Rock/Album1/old.mp3"));

        // Simulate a rename: scanner deletes the old row and upserts the new one.
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
            Some("/music/Rock/Album1/new.mp3"),
            "playlist must reflect the new path after rename"
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

        // User creates a manual playlist and adds a track to it.
        let manual = svc.create_playlist("My Mix").unwrap();
        svc.add_track_to_playlist(&manual.id, &local_track("m1", "Queen", "/music/Rock/Album1/a1.mp3"))
            .unwrap();

        let _ = svc.generate_folder_playlists("/music/Rock").unwrap();

        // The manual playlist must keep its track untouched.
        let manual_tracks = svc.get_playlist_tracks(&manual.id).unwrap();
        assert_eq!(
            manual_tracks.len(),
            1,
            "manual playlist tracks must NOT be wiped by folder playlist sync"
        );
        assert_eq!(manual_tracks[0].track.id, "m1");
    }

    // ── Child title uniqueness tests ──────────────────────────────────

    #[test]
    fn generate_folder_playlists_child_titles_use_full_relative_path() {
        let db = Arc::new(Database::open_in_memory().unwrap());
        db.insert_watched_folder("/music/Rock").unwrap();

        // Two nested subfolders that share the same final segment "Live"
        // but live under different parents. With the old scheme both would
        // become "Rock - Live" and collide. With the full-relative-path
        // scheme they become "Rock - Album1/Live" and "Rock - Album2/Live".
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
        assert!(
            titles.contains(&"Rock - Album1/Live".to_string()),
            "expected full-relative-path child title, got {:?}",
            titles
        );
        assert!(
            titles.contains(&"Rock - Album2/Live".to_string()),
            "expected full-relative-path child title, got {:?}",
            titles
        );
        assert_eq!(
            titles.iter().filter(|t| t == &"Rock - Live").count(),
            0,
            "no child should collapse to the colliding last-segment title"
        );

        // Idempotency: re-running must reuse the same full-path titles,
        // not create new playlists with different titles.
        let second = svc.generate_folder_playlists("/music/Rock").unwrap();
        let all = svc.get_all_playlists().unwrap();
        // Parent + 2 children = 3 total, no duplicates from re-run.
        assert_eq!(all.len(), 3, "re-run must not duplicate playlists");
        let _ = second;
    }

    #[test]
    fn generate_folder_playlists_simple_subfolder_uses_segment_as_suffix() {
        // Single-level subfolder: full relative path equals the segment,
        // so the child title is still "Rock - Album1". This guards the
        // backward-compatible case after the title-scheme change.
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

        let deleted = db.delete_playlists_by_source_folder("/music/Rock").unwrap();
        assert_eq!(deleted, 2, "should delete parent + child");

        let all = db.get_all_playlists().unwrap();
        assert_eq!(all.len(), 1, "manual playlist should remain");
        assert_eq!(all[0].id, manual.id);
        // The child id should not appear in the remaining playlists.
        assert!(all.iter().all(|p| p.id != child.id));
    }

    // ── Artist Favorite collision tests ───────────────────────────────

    #[test]
    fn add_artist_favorite_with_source_does_not_overwrite_other_source() {
        let db = Database::open_in_memory().unwrap();

        // Add YouTube favorite with thumbnail A.
        db.add_artist_favorite(
            "artist:daft-punk",
            "youtube",
            "Daft Punk",
            Some("thumb-a"),
            None,
        )
        .unwrap();

        // Add Local favorite with the same artist_id but different source.
        // This MUST NOT overwrite the YouTube entry.
        db.add_artist_favorite(
            "artist:daft-punk",
            "local",
            "Daft Punk",
            Some("thumb-b"),
            None,
        )
        .unwrap();

        let all = db.get_all_artist_favorites().unwrap();
        assert_eq!(all.len(), 2, "two distinct favorites should coexist");

        let yt = all.iter().find(|f| f.source == "youtube").unwrap();
        let lc = all.iter().find(|f| f.source == "local").unwrap();
        assert_eq!(yt.thumbnail.as_deref(), Some("thumb-a"));
        assert_eq!(lc.thumbnail.as_deref(), Some("thumb-b"));
    }

    #[test]
    fn add_artist_favorite_same_source_does_not_overwrite_existing_fields() {
        let db = Database::open_in_memory().unwrap();

        // First insert with thumbnail A.
        db.add_artist_favorite(
            "artist:daft-punk",
            "youtube",
            "Daft Punk",
            Some("thumb-a"),
            None,
        )
        .unwrap();

        // Second insert with the SAME (artist_id, source) but different
        // thumbnail. ON CONFLICT DO NOTHING should preserve thumb-a.
        db.add_artist_favorite(
            "artist:daft-punk",
            "youtube",
            "Daft Punk Remixed",
            Some("thumb-b"),
            None,
        )
        .unwrap();

        let all = db.get_all_artist_favorites().unwrap();
        assert_eq!(all.len(), 1, "same (artist_id, source) should not duplicate");
        assert_eq!(all[0].thumbnail.as_deref(), Some("thumb-a"));
        assert_eq!(all[0].artist_name, "Daft Punk");
    }
}
