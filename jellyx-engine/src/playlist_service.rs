//! Playlist service — engine-side business logic for user playlists.
//!
//! `PlaylistService` wraps a [`SqliteHandle`] and composes the engine
//! repositories (`UserPlaylistsRepository`, `PlaylistTracksRepository`,
//! `ArtistFavoritesRepository`, `LocalTrackRepository`) to provide
//! business-logic-level operations: playlist CRUD, playlist track management,
//! artist/folder playlist generation, and artist favorites.
//!
//! Track serialization is owned by this service: repositories return raw JSON
//! strings (`PlaylistTrackRow::track_json`, `LocalTrackRow::track_json`) and
//! the service deserializes them into `jellyx_core::models::track::Track`.
//!
//! Both Tauri and Ratatui frontends depend on this service so the business
//! logic lives in exactly one place. Desktop's `PlaylistService` is a thin
//! adapter that maps engine types to IPC DTOs and engine errors to `AppError`.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use jellyx_core::models::track::Track;

use crate::artist_favorites::{ArtistFavoriteRow, ArtistFavoritesRepository};
use crate::local_track::LocalTrackRepository;
use crate::playlist_tracks::PlaylistTracksRepository;
use crate::sqlite::SqliteHandle;
use crate::user_playlists::{UserPlaylist, UserPlaylistsRepository};

/// Fallback artist name when a track has no artist metadata.
const UNKNOWN_ARTIST: &str = "Unknown Artist";

/// Errors produced by the engine `PlaylistService`.
///
/// Desktop maps these to `AppError` via the adapter in
/// `jellyx-desktop/src/library/playlist_service.rs`.
#[derive(Debug)]
pub enum PlaylistServiceError {
    /// Entity (playlist, track) was not found.
    NotFound(String),
    /// SQLite failure from a repository call.
    Persistence(String),
    /// Track JSON failed to deserialize.
    Deserialization(String),
    /// Track JSON failed to serialize.
    Serialization(String),
}

impl std::fmt::Display for PlaylistServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound(msg) => write!(f, "not found: {msg}"),
            Self::Persistence(msg) => write!(f, "persistence error: {msg}"),
            Self::Deserialization(msg) => write!(f, "deserialization error: {msg}"),
            Self::Serialization(msg) => write!(f, "serialization error: {msg}"),
        }
    }
}

impl std::error::Error for PlaylistServiceError {}

impl From<rusqlite::Error> for PlaylistServiceError {
    fn from(error: rusqlite::Error) -> Self {
        if error == rusqlite::Error::QueryReturnedNoRows {
            Self::NotFound(error.to_string())
        } else {
            Self::Persistence(error.to_string())
        }
    }
}

impl From<serde_json::Error> for PlaylistServiceError {
    fn from(error: serde_json::Error) -> Self {
        Self::Deserialization(error.to_string())
    }
}

/// A track entry inside a user playlist, with the `Track` deserialized.
///
/// The engine repository stores `track_json` as a raw string; this service
/// deserializes it into a `Track` so callers don't have to.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PlaylistTrackEntry {
    pub playlist_id: String,
    pub position: i64,
    pub track: Track,
    pub added_at: String,
}

/// Service providing user playlist operations.
///
/// Owns a cloneable [`SqliteHandle`] and composes the engine repositories
/// on each call. All methods are synchronous since SQLite operations are
/// fast and the WAL mode handles concurrency.
pub struct PlaylistService {
    db: SqliteHandle,
}

impl PlaylistService {
    /// Create a new PlaylistService backed by the given handle.
    pub fn new(db: SqliteHandle) -> Self {
        Self { db }
    }

    /// Create a new user playlist.
    pub fn create_playlist(&self, title: &str) -> Result<UserPlaylist, PlaylistServiceError> {
        Ok(UserPlaylistsRepository::new(self.db.clone()).create(title)?)
    }

    /// Rename an existing user playlist.
    pub fn rename_playlist(&self, id: &str, title: &str) -> Result<(), PlaylistServiceError> {
        UserPlaylistsRepository::new(self.db.clone()).rename(id, title)?;
        Ok(())
    }

    /// Delete a user playlist by ID.
    pub fn delete_playlist(&self, id: &str) -> Result<(), PlaylistServiceError> {
        UserPlaylistsRepository::new(self.db.clone()).delete(id)?;
        Ok(())
    }

    /// Get all user playlists.
    pub fn get_all_playlists(&self) -> Result<Vec<UserPlaylist>, PlaylistServiceError> {
        Ok(UserPlaylistsRepository::new(self.db.clone()).get_all()?)
    }

    /// Get all playlists generated from a watched folder (parent + children).
    pub fn get_playlists_by_source_folder(
        &self,
        folder_path: &str,
    ) -> Result<Vec<UserPlaylist>, PlaylistServiceError> {
        Ok(UserPlaylistsRepository::new(self.db.clone()).get_by_source_folder(folder_path)?)
    }

    /// Get all child playlists of a parent playlist.
    pub fn get_child_playlists(
        &self,
        parent_id: &str,
    ) -> Result<Vec<UserPlaylist>, PlaylistServiceError> {
        Ok(UserPlaylistsRepository::new(self.db.clone()).get_child_playlists(parent_id)?)
    }

    /// Delete all playlists generated from a watched folder (cascade).
    pub fn delete_playlists_by_source_folder(
        &self,
        folder_path: &str,
    ) -> Result<u64, PlaylistServiceError> {
        Ok(UserPlaylistsRepository::new(self.db.clone()).delete_by_source_folder(folder_path)?)
    }

    /// Get recent playlists (limited).
    pub fn get_recent_playlists(
        &self,
        limit: u32,
    ) -> Result<Vec<UserPlaylist>, PlaylistServiceError> {
        Ok(UserPlaylistsRepository::new(self.db.clone()).get_recent(limit)?)
    }

    /// Search playlists by title.
    pub fn search_playlists(&self, query: &str) -> Result<Vec<UserPlaylist>, PlaylistServiceError> {
        Ok(UserPlaylistsRepository::new(self.db.clone()).search(query)?)
    }

    /// Add a track to a playlist.
    pub fn add_track_to_playlist(
        &self,
        playlist_id: &str,
        track: &Track,
    ) -> Result<(), PlaylistServiceError> {
        let track_json = serde_json::to_string(track)
            .map_err(|e| PlaylistServiceError::Serialization(e.to_string()))?;
        PlaylistTracksRepository::new(self.db.clone()).add_track(playlist_id, &track_json)?;
        Ok(())
    }

    /// Remove a track from a playlist by position.
    pub fn remove_track_from_playlist(
        &self,
        playlist_id: &str,
        position: i64,
    ) -> Result<(), PlaylistServiceError> {
        PlaylistTracksRepository::new(self.db.clone()).remove_track(playlist_id, position)?;
        Ok(())
    }

    /// Get all tracks in a playlist.
    ///
    /// Deserializes `track_json` from each [`PlaylistTrackRow`] into a
    /// [`Track`] so callers receive a fully-typed entry.
    pub fn get_playlist_tracks(
        &self,
        playlist_id: &str,
    ) -> Result<Vec<PlaylistTrackEntry>, PlaylistServiceError> {
        let rows = PlaylistTracksRepository::new(self.db.clone()).get_tracks(playlist_id)?;
        rows.into_iter()
            .map(|row| {
                let track: Track = serde_json::from_str(&row.track_json)?;
                Ok(PlaylistTrackEntry {
                    playlist_id: row.playlist_id,
                    position: row.position,
                    track,
                    added_at: row.added_at,
                })
            })
            .collect()
    }

    /// Count tracks in a playlist.
    pub fn count_playlist_tracks(&self, playlist_id: &str) -> Result<u32, PlaylistServiceError> {
        Ok(PlaylistTracksRepository::new(self.db.clone()).count_tracks(playlist_id)?)
    }

    /// Get up to 4 thumbnail URLs from a playlist's tracks.
    pub fn get_playlist_thumbnails(
        &self,
        playlist_id: &str,
    ) -> Result<Vec<String>, PlaylistServiceError> {
        Ok(PlaylistTracksRepository::new(self.db.clone()).get_thumbnails(playlist_id)?)
    }

    // ── Artist Playlist Generation ──────────────────────────────────────

    /// Generate one playlist per artist from the local track catalog.
    ///
    /// Groups all local tracks by `track.artist` (empty/whitespace-only
    /// artists fall back to "Unknown Artist"). For each artist, if a playlist
    /// with that exact title already exists, it is reused and only new tracks
    /// are appended (idempotent). Otherwise a new playlist is created and all
    /// the artist's tracks are added.
    ///
    /// Returns the list of playlists that were either created or updated
    /// during this run (existing-but-unchanged playlists are not included).
    pub fn generate_artist_playlists(&self) -> Result<Vec<UserPlaylist>, PlaylistServiceError> {
        let local_rows = LocalTrackRepository::new(self.db.clone()).get_all(None)?;
        let local_entries: Vec<Track> = local_rows
            .into_iter()
            .map(|r| serde_json::from_str(&r.track_json))
            .collect::<Result<_, _>>()?;

        // Nothing to do if there are no local tracks at all.
        if local_entries.is_empty() {
            return Ok(Vec::new());
        }

        // Group tracks by artist name.
        let mut by_artist: HashMap<String, Vec<Track>> = HashMap::new();
        for track in &local_entries {
            let trimmed_artist = track.artist.trim();
            let artist = if trimmed_artist.is_empty() {
                UNKNOWN_ARTIST.to_string()
            } else {
                trimmed_artist.to_string()
            };
            by_artist.entry(artist).or_default().push(track.clone());
        }

        // Load existing playlists once and index their IDs by title for O(1) lookup.
        let existing = UserPlaylistsRepository::new(self.db.clone()).get_all()?;
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
                let pl = UserPlaylistsRepository::new(self.db.clone()).create(&artist)?;
                existing_by_title.insert(artist.clone(), pl.clone());
                pl
            };

            // Fetch the tracks already in this playlist so we only append the
            // missing ones. This keeps re-runs cheap and avoids duplicates.
            let existing_tracks = self.get_playlist_tracks(&playlist.id)?;
            let existing_ids: HashSet<String> =
                existing_tracks.iter().map(|e| e.track.id.clone()).collect();

            let mut added_any = false;
            for track in tracks {
                if existing_ids.contains(&track.id) {
                    continue;
                }
                self.add_track_to_playlist(&playlist.id, track)?;
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
    /// Because `playlist_tracks` stores serialized Track JSON with no foreign
    /// key to `local_tracks`, simply appending new tracks on re-scan would
    /// leave stale entries pointing at removed/moved files forever. To stay
    /// synchronized with the current scanner state, this method **wipes and
    /// rebuilds** the `playlist_tracks` rows for every folder-generated
    /// playlist belonging to `watched_folder_path` on each successful scan,
    /// then re-adds the current tracks from `local_tracks`. Manual playlists
    /// (`kind = "manual"`) and artist-generated playlists are never touched.
    pub fn generate_folder_playlists(
        &self,
        watched_folder_path: &str,
    ) -> Result<Vec<UserPlaylist>, PlaylistServiceError> {
        let entries =
            LocalTrackRepository::new(self.db.clone()).get_all(Some(watched_folder_path))?;
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
        let existing = UserPlaylistsRepository::new(self.db.clone())
            .get_by_source_folder(watched_folder_path)?;
        let parent_playlist: UserPlaylist =
            if let Some(parent) = existing.iter().find(|p| p.parent_playlist_id.is_none()) {
                parent.clone()
            } else {
                UserPlaylistsRepository::new(self.db.clone()).create_folder(
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
            let track: Track = serde_json::from_str(&entry.track_json)?;
            match entry.subfolder_path.as_deref() {
                None | Some("") => root_tracks.push(track),
                Some(sub) => by_subfolder.entry(sub.to_string()).or_default().push(track),
            }
        }

        // ── Synchronization step ───────────────────────────────────────
        // Wipe `playlist_tracks` for every folder-generated playlist that
        // belongs to this watched folder BEFORE rebuilding. This removes
        // stale entries for files that were deleted/moved since the last
        // scan. Only `kind = "folder"` playlists with a matching
        // `source_folder_path` are touched; manual and artist-generated
        // playlists are preserved.
        let track_repo = PlaylistTracksRepository::new(self.db.clone());
        track_repo.clear_tracks(&parent_playlist.id)?;
        for child in existing_children_by_title.values() {
            track_repo.clear_tracks(&child.id)?;
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
                UserPlaylistsRepository::new(self.db.clone()).create_folder(
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
    fn add_tracks_to_playlist(
        &self,
        playlist_id: &str,
        tracks: &[Track],
    ) -> Result<(), PlaylistServiceError> {
        for track in tracks {
            self.add_track_to_playlist(playlist_id, track)?;
        }
        Ok(())
    }

    // ── Artist Favorites ────────────────────────────────────────────────

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
    ) -> Result<(), PlaylistServiceError> {
        ArtistFavoritesRepository::new(self.db.clone()).add(
            artist_id,
            source,
            artist_name,
            thumbnail,
            source_artist_ref,
        )?;
        Ok(())
    }

    /// Remove an artist from favorites.
    ///
    /// Pass `source = Some("youtube")` to remove only the YouTube favorite;
    /// pass `None` to remove every favorite for that artist across all
    /// sources.
    pub fn remove_artist_favorite(
        &self,
        artist_id: &str,
        source: Option<&str>,
    ) -> Result<(), PlaylistServiceError> {
        ArtistFavoritesRepository::new(self.db.clone()).remove(artist_id, source)?;
        Ok(())
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
    ) -> Result<bool, PlaylistServiceError> {
        Ok(ArtistFavoritesRepository::new(self.db.clone()).is_favorite(artist_id, source)?)
    }

    /// Get all favorited artists.
    pub fn get_all_artist_favorites(&self) -> Result<Vec<ArtistFavoriteRow>, PlaylistServiceError> {
        Ok(ArtistFavoritesRepository::new(self.db.clone()).get_all()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sqlite::SqliteHandle;
    use jellyx_core::models::source::Source;
    use rusqlite::Connection;
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

    /// Build an in-memory handle with the canonical pre-migration schema for
    /// `watched_folders`, `local_tracks`, `user_playlists`, `playlist_tracks`,
    /// and `artist_favorites`. Seed one watched folder at `/music`.
    fn fresh_handle() -> SqliteHandle {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE watched_folders (
                path TEXT PRIMARY KEY,
                last_scanned_at TEXT,
                added_at TEXT NOT NULL DEFAULT (datetime('now'))
            );
            CREATE TABLE local_tracks (
                file_path TEXT PRIMARY KEY,
                track_json TEXT NOT NULL,
                folder_path TEXT NOT NULL,
                file_modified_at TEXT,
                subfolder_path TEXT,
                FOREIGN KEY(folder_path) REFERENCES watched_folders(path) ON DELETE CASCADE
            );
            CREATE INDEX IF NOT EXISTS idx_local_tracks_folder ON local_tracks(folder_path);
            CREATE TABLE user_playlists (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                kind TEXT NOT NULL DEFAULT 'manual',
                source_folder_path TEXT,
                parent_playlist_id TEXT,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now'))
            );
            CREATE TABLE playlist_tracks (
                playlist_id TEXT NOT NULL,
                position INTEGER NOT NULL,
                track_json TEXT NOT NULL,
                added_at TEXT NOT NULL DEFAULT (datetime('now')),
                PRIMARY KEY (playlist_id, position),
                FOREIGN KEY (playlist_id) REFERENCES user_playlists(id) ON DELETE CASCADE
            );
            CREATE INDEX IF NOT EXISTS idx_playlist_tracks_playlist
                ON playlist_tracks(playlist_id, position);
            CREATE TABLE artist_favorites (
                artist_id TEXT NOT NULL,
                source TEXT NOT NULL DEFAULT 'local',
                artist_name TEXT NOT NULL,
                thumbnail TEXT,
                source_artist_ref TEXT,
                added_at TEXT NOT NULL DEFAULT (datetime('now')),
                PRIMARY KEY (artist_id, source)
            );",
        )
        .unwrap();
        // Seed watched folders so local_tracks FK constraints pass.
        conn.execute(
            "INSERT INTO watched_folders (path) VALUES ('/music'), ('/music/Rock'), ('/music/Singles'), ('/music/Empty')",
            [],
        )
        .unwrap();
        SqliteHandle::new(conn)
    }

    fn setup_service() -> PlaylistService {
        PlaylistService::new(fresh_handle())
    }

    /// Insert a local track via the engine repository.
    fn seed_local_track(svc: &PlaylistService, track: &Track, file_path: &str, folder: &str) {
        let json = serde_json::to_string(track).unwrap();
        LocalTrackRepository::new(svc.db.clone())
            .upsert(file_path, &json, folder, Some("1000"), None)
            .unwrap();
    }

    /// Insert a local track with an explicit subfolder_path relative to the
    /// watched folder root.
    fn seed_local_track_in_subfolder(
        svc: &PlaylistService,
        track: &Track,
        file_path: &str,
        folder: &str,
        subfolder: &str,
    ) {
        let json = serde_json::to_string(track).unwrap();
        LocalTrackRepository::new(svc.db.clone())
            .upsert(file_path, &json, folder, Some("1000"), Some(subfolder))
            .unwrap();
    }

    // ── create_playlist + add_track + get_playlist_tracks ──────────────

    #[test]
    fn create_playlist_returns_manual_kind() {
        let svc = setup_service();
        let pl = svc.create_playlist("My Mix").unwrap();
        assert_eq!(pl.title, "My Mix");
        assert_eq!(pl.kind, "manual");
        assert!(pl.source_folder_path.is_none());
        assert!(pl.parent_playlist_id.is_none());
    }

    #[test]
    fn add_track_and_get_playlist_tracks_round_trip() {
        let svc = setup_service();
        let pl = svc.create_playlist("Mix").unwrap();
        let track = local_track("t1", "Daft Punk", "/music/a.mp3");
        svc.add_track_to_playlist(&pl.id, &track).unwrap();

        let entries = svc.get_playlist_tracks(&pl.id).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].playlist_id, pl.id);
        assert_eq!(entries[0].position, 0);
        assert_eq!(entries[0].track.id, "t1");
        assert_eq!(entries[0].track.artist, "Daft Punk");
    }

    #[test]
    fn count_playlist_tracks_reflects_inserts() {
        let svc = setup_service();
        let pl = svc.create_playlist("Mix").unwrap();
        assert_eq!(svc.count_playlist_tracks(&pl.id).unwrap(), 0);
        svc.add_track_to_playlist(&pl.id, &local_track("t1", "A", "/music/a.mp3"))
            .unwrap();
        svc.add_track_to_playlist(&pl.id, &local_track("t2", "A", "/music/b.mp3"))
            .unwrap();
        assert_eq!(svc.count_playlist_tracks(&pl.id).unwrap(), 2);
    }

    #[test]
    fn remove_track_reindexes_positions() {
        let svc = setup_service();
        let pl = svc.create_playlist("Mix").unwrap();
        svc.add_track_to_playlist(&pl.id, &local_track("t1", "A", "/music/a.mp3"))
            .unwrap();
        svc.add_track_to_playlist(&pl.id, &local_track("t2", "A", "/music/b.mp3"))
            .unwrap();
        svc.add_track_to_playlist(&pl.id, &local_track("t3", "A", "/music/c.mp3"))
            .unwrap();

        svc.remove_track_from_playlist(&pl.id, 1).unwrap();

        let entries = svc.get_playlist_tracks(&pl.id).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].position, 0);
        assert_eq!(entries[0].track.id, "t1");
        assert_eq!(entries[1].position, 1);
        assert_eq!(entries[1].track.id, "t3");
    }

    #[test]
    fn rename_and_delete_playlist() {
        let svc = setup_service();
        let pl = svc.create_playlist("Old").unwrap();
        svc.rename_playlist(&pl.id, "New").unwrap();
        let all = svc.get_all_playlists().unwrap();
        assert_eq!(all[0].title, "New");

        svc.delete_playlist(&pl.id).unwrap();
        assert!(svc.get_all_playlists().unwrap().is_empty());
    }

    #[test]
    fn search_playlists_by_title() {
        let svc = setup_service();
        svc.create_playlist("Summer Hits").unwrap();
        svc.create_playlist("Winter Chill").unwrap();
        let results = svc.search_playlists("Summer").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Summer Hits");
    }

    #[test]
    fn get_recent_playlists_respects_limit() {
        let svc = setup_service();
        svc.create_playlist("A").unwrap();
        svc.create_playlist("B").unwrap();
        svc.create_playlist("C").unwrap();
        assert_eq!(svc.get_recent_playlists(2).unwrap().len(), 2);
    }

    #[test]
    fn get_playlist_thumbnails_extracts_up_to_four() {
        let svc = setup_service();
        let pl = svc.create_playlist("Mix").unwrap();
        for i in 0..6 {
            let mut t = local_track(&format!("t{}", i), "A", &format!("/music/{}.mp3", i));
            t.thumbnail = Some(format!("http://x/{}.jpg", i));
            svc.add_track_to_playlist(&pl.id, &t).unwrap();
        }
        let thumbs = svc.get_playlist_thumbnails(&pl.id).unwrap();
        assert_eq!(thumbs.len(), 4);
        assert_eq!(thumbs[0], "http://x/0.jpg");
    }

    // ── generate_artist_playlists ──────────────────────────────────────

    #[test]
    fn generate_artist_playlists_groups_by_artist() {
        let svc = setup_service();
        seed_local_track(
            &svc,
            &local_track("t1", "Daft Punk", "/music/a.mp3"),
            "/music/a.mp3",
            "/music",
        );
        seed_local_track(
            &svc,
            &local_track("t2", "Daft Punk", "/music/b.mp3"),
            "/music/b.mp3",
            "/music",
        );
        seed_local_track(
            &svc,
            &local_track("t3", "Queen", "/music/c.mp3"),
            "/music/c.mp3",
            "/music",
        );
        seed_local_track(
            &svc,
            &local_track("t4", "Queen", "/music/d.mp3"),
            "/music/d.mp3",
            "/music",
        );

        let created = svc.generate_artist_playlists().unwrap();
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
        let svc = setup_service();
        seed_local_track(
            &svc,
            &local_track("t1", "Daft Punk", "/music/a.mp3"),
            "/music/a.mp3",
            "/music",
        );
        seed_local_track(
            &svc,
            &local_track("t2", "Queen", "/music/c.mp3"),
            "/music/c.mp3",
            "/music",
        );

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
        let svc = setup_service();
        seed_local_track(
            &svc,
            &local_track("t1", "Daft Punk", "/music/a.mp3"),
            "/music/a.mp3",
            "/music",
        );

        let first = svc.generate_artist_playlists().unwrap();
        assert_eq!(first.len(), 1);

        seed_local_track(
            &svc,
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
        let svc = setup_service();
        let created = svc.generate_artist_playlists().unwrap();
        assert!(
            created.is_empty(),
            "empty local library should produce no playlists"
        );
    }

    #[test]
    fn generate_artist_playlists_groups_unknown_artist() {
        let svc = setup_service();
        seed_local_track(
            &svc,
            &local_track("t1", "", "/music/a.mp3"),
            "/music/a.mp3",
            "/music",
        );
        seed_local_track(
            &svc,
            &local_track("t2", "   ", "/music/b.mp3"),
            "/music/b.mp3",
            "/music",
        );
        seed_local_track(
            &svc,
            &local_track("t3", "Queen", "/music/c.mp3"),
            "/music/c.mp3",
            "/music",
        );

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

    // ── generate_folder_playlists ──────────────────────────────────────

    #[test]
    fn generate_folder_playlists_creates_parent_and_children_for_subfolders() {
        let svc = setup_service();
        seed_local_track_in_subfolder(
            &svc,
            &local_track("a1", "AC/DC", "/music/Rock/Album1/a1.mp3"),
            "/music/Rock/Album1/a1.mp3",
            "/music/Rock",
            "Album1",
        );
        seed_local_track_in_subfolder(
            &svc,
            &local_track("a2", "AC/DC", "/music/Rock/Album1/a2.mp3"),
            "/music/Rock/Album1/a2.mp3",
            "/music/Rock",
            "Album1",
        );
        seed_local_track_in_subfolder(
            &svc,
            &local_track("b1", "AC/DC", "/music/Rock/Album2/b1.mp3"),
            "/music/Rock/Album2/b1.mp3",
            "/music/Rock",
            "Album2",
        );
        seed_local_track_in_subfolder(
            &svc,
            &local_track("b2", "AC/DC", "/music/Rock/Album2/b2.mp3"),
            "/music/Rock/Album2/b2.mp3",
            "/music/Rock",
            "Album2",
        );

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

        // Parent should have no tracks (subfolders absorb all tracks).
        assert_eq!(svc.get_playlist_tracks(&parent.id).unwrap().len(), 0);
    }

    #[test]
    fn generate_folder_playlists_creates_single_playlist_when_no_subfolders() {
        let svc = setup_service();
        seed_local_track(
            &svc,
            &local_track("s1", "DJ", "/music/Singles/s1.mp3"),
            "/music/Singles/s1.mp3",
            "/music/Singles",
        );
        seed_local_track(
            &svc,
            &local_track("s2", "DJ", "/music/Singles/s2.mp3"),
            "/music/Singles/s2.mp3",
            "/music/Singles",
        );

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
        let svc = setup_service();
        seed_local_track_in_subfolder(
            &svc,
            &local_track("a1", "AC/DC", "/music/Rock/Album1/a1.mp3"),
            "/music/Rock/Album1/a1.mp3",
            "/music/Rock",
            "Album1",
        );

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
    fn generate_folder_playlists_removes_stale_tracks_on_rescan() {
        let svc = setup_service();
        seed_local_track_in_subfolder(
            &svc,
            &local_track("a1", "AC/DC", "/music/Rock/Album1/a1.mp3"),
            "/music/Rock/Album1/a1.mp3",
            "/music/Rock",
            "Album1",
        );
        seed_local_track_in_subfolder(
            &svc,
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
        assert_eq!(svc.get_playlist_tracks(&child.id).unwrap().len(), 2);

        // Simulate a rescan where a1 was deleted from disk.
        LocalTrackRepository::new(svc.db.clone())
            .delete_by_path("/music/Rock/Album1/a1.mp3")
            .unwrap();

        let _ = svc.generate_folder_playlists("/music/Rock").unwrap();
        let tracks = svc.get_playlist_tracks(&child.id).unwrap();
        assert_eq!(tracks.len(), 1, "stale track must be removed after rescan");
        assert_eq!(tracks[0].track.id, "a2");
    }

    #[test]
    fn generate_folder_playlists_child_titles_use_full_relative_path() {
        let svc = setup_service();
        seed_local_track_in_subfolder(
            &svc,
            &local_track("l1", "AC/DC", "/music/Rock/Album1/Live/l1.mp3"),
            "/music/Rock/Album1/Live/l1.mp3",
            "/music/Rock",
            "Album1/Live",
        );
        seed_local_track_in_subfolder(
            &svc,
            &local_track("l2", "AC/DC", "/music/Rock/Album2/Live/l2.mp3"),
            "/music/Rock/Album2/Live/l2.mp3",
            "/music/Rock",
            "Album2/Live",
        );

        let created = svc.generate_folder_playlists("/music/Rock").unwrap();
        let titles: Vec<String> = created.iter().map(|p| p.title.clone()).collect();
        assert!(titles.contains(&"Rock - Album1/Live".to_string()));
        assert!(titles.contains(&"Rock - Album2/Live".to_string()));
        assert_eq!(titles.iter().filter(|t| t == &"Rock - Live").count(), 0);
    }

    #[test]
    fn generate_folder_playlists_empty_folder_returns_empty() {
        let svc = setup_service();
        let created = svc.generate_folder_playlists("/music/Empty").unwrap();
        assert!(
            created.is_empty(),
            "empty folder should produce no playlists"
        );
    }

    // ── Artist favorites ───────────────────────────────────────────────

    #[test]
    fn add_artist_favorite_with_source_does_not_overwrite_other_source() {
        let svc = setup_service();
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
    fn is_artist_favorite_checks_source_dimension() {
        let svc = setup_service();
        svc.add_artist_favorite_with_source("a1", "local", "Artist One", None, None)
            .unwrap();
        assert!(svc.is_artist_favorite("a1", Some("local")).unwrap());
        assert!(!svc.is_artist_favorite("a1", Some("youtube")).unwrap());
        assert!(svc.is_artist_favorite("a1", None).unwrap());
    }

    #[test]
    fn remove_artist_favorite_without_source_removes_all() {
        let svc = setup_service();
        svc.add_artist_favorite_with_source("a1", "local", "Artist One", None, None)
            .unwrap();
        svc.add_artist_favorite_with_source("a1", "youtube", "Artist One", None, None)
            .unwrap();
        svc.remove_artist_favorite("a1", None).unwrap();
        assert!(svc.get_all_artist_favorites().unwrap().is_empty());
    }

    // ── error mapping ──────────────────────────────────────────────────

    #[test]
    fn playlist_service_error_from_rusqlite_and_serde() {
        let persistence: PlaylistServiceError = rusqlite::Error::InvalidColumnIndex(0).into();
        assert!(matches!(persistence, PlaylistServiceError::Persistence(_)));

        let not_found: PlaylistServiceError = rusqlite::Error::QueryReturnedNoRows.into();
        assert!(matches!(not_found, PlaylistServiceError::NotFound(_)));

        let json: PlaylistServiceError = serde_json::from_str::<Track>("not json")
            .unwrap_err()
            .into();
        assert!(matches!(json, PlaylistServiceError::Deserialization(_)));
    }
}
