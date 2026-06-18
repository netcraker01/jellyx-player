//! Library service — CRUD for favorites, history.
//!
//! `LibraryService` wraps the `Database` and provides business-logic-level
//! operations for favorites and history. All methods return `AppError`
//! for consistent IPC error handling.

use std::sync::Arc;

use crate::errors::types::{AppError, LibraryError};
use crate::ipc::dto::{
    normalize_album_id, normalize_artist_id, AlbumDetail, AlbumSummary, ArtistDetail,
    ArtistSummary, GroupedSearchResult, SearchFilter,
};
use crate::models::track::Track;
use crate::persistence::db::Database;
use crate::persistence::models::{FavoriteEntry, HistoryEntry};
use std::collections::HashMap;

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

    /// Search local tracks and group results into songs, artists, and albums.
    ///
    /// When `filter` is `None` all groups are populated. When a filter is
    /// provided, only the matching group is populated; the others are empty.
    pub fn search_grouped(
        &self,
        query: &str,
        filter: Option<SearchFilter>,
    ) -> Result<GroupedSearchResult, AppError> {
        let trimmed = query.trim();
        if trimmed.is_empty() {
            return Err(crate::errors::types::ValidationError::EmptyQuery.into());
        }

        let matching_tracks = self.db.search_local_tracks(trimmed).map_err(AppError::from)?;

        let include_all = filter.is_none();
        let include_songs = include_all || filter == Some(SearchFilter::Songs);
        let include_artists = include_all || filter == Some(SearchFilter::Artists);
        let include_albums = include_all || filter == Some(SearchFilter::Albums);

        let songs = if include_songs { matching_tracks.clone() } else { Vec::new() };

        let mut artists: Vec<ArtistSummary> = Vec::new();
        let mut albums: Vec<AlbumSummary> = Vec::new();

        if include_artists || include_albums {
            let mut artist_map: HashMap<String, Vec<&Track>> = HashMap::new();
            let mut album_map: HashMap<String, Vec<&Track>> = HashMap::new();

            for track in &matching_tracks {
                if include_artists {
                    let artist_id = normalize_artist_id(&track.artist);
                    artist_map.entry(artist_id).or_default().push(track);
                }

                if include_albums {
                    if let Some(ref album) = track.album {
                        let album_id = normalize_album_id(album, &track.artist);
                        album_map.entry(album_id).or_default().push(track);
                    }
                }
            }

            artists = artist_map
                .into_iter()
                .map(|(id, tracks)| ArtistSummary {
                    id,
                    name: tracks[0].artist.clone(),
                    thumbnail: tracks.iter().find_map(|t| t.thumbnail.clone()),
                    track_count: tracks.len() as u32,
                })
                .collect();

            albums = album_map
                .into_iter()
                .map(|(id, tracks)| {
                    let title = tracks[0].album.clone().unwrap_or_default();
                    let artist = tracks[0].artist.clone();
                    let cover = tracks.iter().find_map(|t| t.thumbnail.clone());
                    let year = tracks
                        .iter()
                        .find_map(|t| t.metadata.get("year").and_then(|y| y.parse().ok()));
                    AlbumSummary {
                        id,
                        title,
                        artist,
                        cover,
                        year,
                        track_count: tracks.len() as u32,
                    }
                })
                .collect();

            artists.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
            albums.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));
        }

        Ok(GroupedSearchResult {
            songs,
            artists,
            albums,
        })
    }

    /// Get full artist detail by artist ID.
    ///
    /// Resolves the artist name from the ID, loads all tracks by that artist,
    /// and computes top tracks by play count (ties broken alphabetically).
    pub fn get_artist_detail(&self, id: &str) -> Result<ArtistDetail, AppError> {
        let normalized_name = crate::ipc::dto::denormalize_artist_id(id)
            .ok_or_else(|| LibraryError::NotFound(id.to_string()))?;

        let tracks = self
            .db
            .get_local_tracks_by_artist(&normalized_name)
            .map_err(AppError::from)?;
        if tracks.is_empty() {
            return Err(LibraryError::NotFound(id.to_string()).into());
        }

        let canonical_name = tracks[0].artist.clone();

        let play_counts = self.db.get_track_play_counts().map_err(AppError::from)?;

        let mut top_tracks = tracks.clone();
        top_tracks.sort_by(|a, b| {
            let count_a = play_counts.get(&a.id).copied().unwrap_or(0);
            let count_b = play_counts.get(&b.id).copied().unwrap_or(0);
            count_b
                .cmp(&count_a)
                .then_with(|| a.title.to_lowercase().cmp(&b.title.to_lowercase()))
        });

        let thumbnail = tracks.iter().find_map(|t| t.thumbnail.clone());

        let albums = Self::build_album_summaries(&tracks);

        Ok(ArtistDetail {
            id: id.to_string(),
            name: canonical_name,
            thumbnail,
            top_tracks,
            albums,
        })
    }

    /// Get full album detail by album ID.
    ///
    /// Resolves the album title and artist from the ID, loads matching tracks,
    /// and orders them by file path (which usually reflects track order).
    pub fn get_album_detail(&self, id: &str) -> Result<AlbumDetail, AppError> {
        let (normalized_title, normalized_artist) = crate::ipc::dto::denormalize_album_id(id)
            .ok_or_else(|| LibraryError::NotFound(id.to_string()))?;

        let mut tracks = self
            .db
            .get_local_tracks_by_album(&normalized_title, &normalized_artist)
            .map_err(AppError::from)?;
        if tracks.is_empty() {
            return Err(LibraryError::NotFound(id.to_string()).into());
        }

        tracks.sort_by(|a, b| {
            a.local_path
                .as_ref()
                .unwrap_or(&a.id)
                .cmp(b.local_path.as_ref().unwrap_or(&b.id))
        });

        let title = tracks[0].album.clone().unwrap_or_default();
        let artist = tracks[0].artist.clone();
        let artist_id = normalize_artist_id(&artist);
        let cover = tracks.iter().find_map(|t| t.thumbnail.clone());
        let year = tracks
            .iter()
            .find_map(|t| t.metadata.get("year").and_then(|y| y.parse().ok()));

        Ok(AlbumDetail {
            id: id.to_string(),
            title,
            artist,
            artist_id,
            cover,
            year,
            tracks,
        })
    }

    /// Build a list of unique album summaries from a slice of tracks.
    fn build_album_summaries(tracks: &[Track]) -> Vec<AlbumSummary> {
        let mut seen: HashMap<String, Vec<&Track>> = HashMap::new();
        for track in tracks {
            if let Some(ref album) = track.album {
                let id = normalize_album_id(album, &track.artist);
                seen.entry(id).or_default().push(track);
            }
        }

        let mut summaries: Vec<AlbumSummary> = seen
            .into_iter()
            .map(|(id, tracks)| {
                let title = tracks[0].album.clone().unwrap_or_default();
                let artist = tracks[0].artist.clone();
                let cover = tracks.iter().find_map(|t| t.thumbnail.clone());
                let year = tracks
                    .iter()
                    .find_map(|t| t.metadata.get("year").and_then(|y| y.parse().ok()));
                AlbumSummary {
                    id,
                    title,
                    artist,
                    cover,
                    year,
                    track_count: tracks.len() as u32,
                }
            })
            .collect();

        summaries.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));
        summaries
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ipc::dto::SearchFilter;
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

    fn local_track(
        id: &str,
        title: &str,
        artist: &str,
        album: &str,
        path: &str,
    ) -> Track {
        Track {
            id: id.to_string(),
            source: Source::Local,
            source_id: path.to_string(),
            title: title.to_string(),
            artist: artist.to_string(),
            album: Some(album.to_string()),
            duration: Some(180.0),
            thumbnail: None,
            stream_url: None,
            local_path: Some(path.to_string()),
            metadata: HashMap::new(),
        }
    }

    fn setup_service() -> LibraryService {
        let db = Database::open_in_memory().unwrap();
        LibraryService::new(Arc::new(db))
    }

    fn insert_local_tracks(svc: &LibraryService, tracks: &[Track], _folder: &str) {
        svc.db.insert_watched_folder(_folder).unwrap();
        for t in tracks {
            let path = t.local_path.as_ref().unwrap();
            svc.db.upsert_local_track(path, t, _folder, None).unwrap();
        }
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

    // ── Grouped search tests (REQ-MS-1/2) ────────────────────────────────

    #[test]
    fn search_grouped_returns_mixed_groups() {
        let svc = setup_service();
        let tracks = vec![
            local_track("t1", "One More Time", "Daft Punk", "Discovery", "/music/one.mp3"),
            local_track("t2", "Harder Better", "Daft Punk", "Discovery", "/music/harder.mp3"),
            local_track("t3", "Bohemian Rhapsody", "Queen", "A Night at the Opera", "/music/bohemian.mp3"),
        ];
        insert_local_tracks(&svc, &tracks, "/music");

        let result = svc.search_grouped("daft", None).unwrap();
        assert_eq!(result.songs.len(), 2, "Should match two Daft Punk songs");
        assert_eq!(result.artists.len(), 1, "Should find one Daft Punk artist");
        assert_eq!(result.artists[0].name, "Daft Punk");
        assert_eq!(result.artists[0].track_count, 2);
        assert_eq!(result.albums.len(), 1, "Should find one Discovery album");
        assert_eq!(result.albums[0].title, "Discovery");
        assert_eq!(result.albums[0].track_count, 2);
    }

    #[test]
    fn search_grouped_returns_empty_groups_for_no_matches() {
        let svc = setup_service();
        let result = svc.search_grouped("zzzz", None).unwrap();
        assert!(result.songs.is_empty());
        assert!(result.artists.is_empty());
        assert!(result.albums.is_empty());
    }

    #[test]
    fn search_grouped_filter_artists_only() {
        let svc = setup_service();
        let tracks = vec![
            local_track("t1", "Bohemian Rhapsody", "Queen", "A Night at the Opera", "/music/bohemian.mp3"),
            local_track("t2", "We Will Rock You", "Queen", "News of the World", "/music/rockyou.mp3"),
        ];
        insert_local_tracks(&svc, &tracks, "/music");

        let result = svc.search_grouped("queen", Some(SearchFilter::Artists)).unwrap();
        assert!(result.songs.is_empty());
        assert_eq!(result.artists.len(), 1);
        assert!(result.albums.is_empty());
    }

    #[test]
    fn search_grouped_filter_albums_only() {
        let svc = setup_service();
        let tracks = vec![
            local_track("t1", "One More Time", "Daft Punk", "Discovery", "/music/one.mp3"),
            local_track("t2", "Harder Better", "Daft Punk", "Discovery", "/music/harder.mp3"),
        ];
        insert_local_tracks(&svc, &tracks, "/music");

        let result = svc.search_grouped("discovery", Some(SearchFilter::Albums)).unwrap();
        assert!(result.songs.is_empty());
        assert!(result.artists.is_empty());
        assert_eq!(result.albums.len(), 1);
        assert_eq!(result.albums[0].track_count, 2);
    }

    // ── Artist / Album detail tests (REQ-AD-1, REQ-AL-1/2) ───────────────

    #[test]
    fn get_artist_detail_returns_top_tracks_and_albums() {
        let svc = setup_service();
        let tracks = vec![
            local_track("t1", "One More Time", "Daft Punk", "Discovery", "/music/one.mp3"),
            local_track("t2", "Harder Better", "Daft Punk", "Discovery", "/music/harder.mp3"),
            local_track("t3", "Aerodynamic", "Daft Punk", "Discovery", "/music/aero.mp3"),
        ];
        insert_local_tracks(&svc, &tracks, "/music");
        // t1 played twice, t2 once → t1 should be the first top track
        svc.db.insert_history(&tracks[0]).unwrap();
        svc.db.insert_history(&tracks[0]).unwrap();
        svc.db.insert_history(&tracks[1]).unwrap();

        let id = crate::ipc::dto::normalize_artist_id("Daft Punk");
        let detail = svc.get_artist_detail(&id).unwrap();
        assert_eq!(detail.name, "Daft Punk");
        assert_eq!(detail.top_tracks.len(), 3);
        assert_eq!(detail.top_tracks[0].id, "t1", "Most-played track should be first");
        assert_eq!(detail.albums.len(), 1);
        assert_eq!(detail.albums[0].title, "Discovery");
    }

    #[test]
    fn get_artist_detail_not_found_for_unknown_artist() {
        let svc = setup_service();
        let result = svc.get_artist_detail("artist:ghost-band");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, "NOT_FOUND");
    }

    #[test]
    fn get_album_detail_returns_tracks_in_order() {
        let svc = setup_service();
        let tracks = vec![
            local_track("t1", "One More Time", "Daft Punk", "Discovery", "/music/01-one.mp3"),
            local_track("t2", "Aerodynamic", "Daft Punk", "Discovery", "/music/02-aero.mp3"),
            local_track("t3", "Digital Love", "Daft Punk", "Discovery", "/music/03-digital.mp3"),
        ];
        insert_local_tracks(&svc, &tracks, "/music");

        let id = crate::ipc::dto::normalize_album_id("Discovery", "Daft Punk");
        let detail = svc.get_album_detail(&id).unwrap();
        assert_eq!(detail.title, "Discovery");
        assert_eq!(detail.artist, "Daft Punk");
        assert_eq!(detail.tracks.len(), 3);
        assert_eq!(detail.tracks[0].title, "One More Time");
        assert_eq!(detail.tracks[1].title, "Aerodynamic");
        assert_eq!(detail.tracks[2].title, "Digital Love");
    }

    #[test]
    fn get_album_detail_not_found_for_unknown_album() {
        let svc = setup_service();
        let result = svc.get_album_detail("album:ghost:ghost-band");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, "NOT_FOUND");
    }
}