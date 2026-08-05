//! Library service — engine-side business logic for history, search, and home.
//!
//! `LibraryService` wraps a [`SqliteHandle`] and composes the engine
//! repositories (`HistoryRepository`, `LocalTrackRepository`) to provide
//! business-logic-level operations: recording plays, grouped search, artist
//! and album detail, and home recommendations.
//!
//! Track serialization is owned by this service: repositories return raw JSON
//! strings (`HistoryRow::track_json`, `LocalTrackRow::track_json`) and the
//! service deserializes them into `jellyx_core::models::track::Track`.
//!
//! Both Tauri and Ratatui frontends depend on this service so the business
//! logic lives in exactly one place. Desktop's `LibraryService` is a thin
//! adapter that maps engine DTOs to IPC DTOs and engine errors to `AppError`.

use std::collections::{HashMap, HashSet};
use std::time::{SystemTime, UNIX_EPOCH};

use jellyx_core::models::track::Track;
use rand::SeedableRng;
use rand::rngs::StdRng;
use rand::seq::SliceRandom;

use crate::dto::{
    AlbumDetail, AlbumSummary, ArtistDetail, ArtistSummary, GroupedSearchResult, HomeSnapshot,
    RecommendationItem, SearchFilter, denormalize_album_id, denormalize_artist_id,
    normalize_album_id, normalize_artist_id,
};
use crate::history::{HistoryRepository, HistoryRow};
use crate::local_track::LocalTrackRepository;
use crate::sqlite::SqliteHandle;

/// Errors produced by the engine `LibraryService`.
///
/// Desktop maps these to `AppError` via the adapter in
/// `jellyx-desktop/src/library/service.rs`.
#[derive(Debug)]
pub enum LibraryServiceError {
    /// Entity (artist, album) was not found.
    NotFound(String),
    /// Input validation failed (e.g. empty search query).
    ValidationError(String),
    /// SQLite failure from a repository call.
    Persistence(String),
    /// Track JSON failed to deserialize.
    Deserialization(String),
}

impl std::fmt::Display for LibraryServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound(msg) => write!(f, "not found: {msg}"),
            Self::ValidationError(msg) => write!(f, "validation error: {msg}"),
            Self::Persistence(msg) => write!(f, "persistence error: {msg}"),
            Self::Deserialization(msg) => write!(f, "deserialization error: {msg}"),
        }
    }
}

impl std::error::Error for LibraryServiceError {}

impl From<rusqlite::Error> for LibraryServiceError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Persistence(error.to_string())
    }
}

impl From<serde_json::Error> for LibraryServiceError {
    fn from(error: serde_json::Error) -> Self {
        Self::Deserialization(error.to_string())
    }
}

/// Service providing library operations (history, search, home).
///
/// Owns a cloneable [`SqliteHandle`] and composes the engine repositories
/// on each call. All methods are synchronous since SQLite operations are
/// fast and the WAL mode handles concurrency.
pub struct LibraryService {
    db: SqliteHandle,
}

impl LibraryService {
    /// Create a new LibraryService backed by the given handle.
    pub fn new(db: SqliteHandle) -> Self {
        Self { db }
    }

    /// Record a play event in history.
    pub fn record_play(&self, track: &Track) -> Result<(), LibraryServiceError> {
        let track_json = serde_json::to_string(track)?;
        HistoryRepository::new(self.db.clone()).insert(&track.id, &track_json)?;
        Ok(())
    }

    /// Get recently played tracks deduplicated by track_id.
    ///
    /// Returns only the most recent entry per track so the same track doesn't
    /// appear multiple times in the "recently played" list. Tracks are
    /// deserialized from `HistoryRow::track_json`.
    pub fn get_recent_unique(&self, limit: u32) -> Result<Vec<HistoryRow>, LibraryServiceError> {
        Ok(HistoryRepository::new(self.db.clone()).get_recent_unique(limit)?)
    }

    /// Clear all play history.
    pub fn clear_history(&self) -> Result<(), LibraryServiceError> {
        HistoryRepository::new(self.db.clone()).clear()?;
        Ok(())
    }

    /// Search local tracks and group results into songs, artists, and albums.
    ///
    /// When `filter` is `None` all groups are populated. When a filter is
    /// provided, only the matching group is populated; the others are empty.
    pub fn search_grouped(
        &self,
        query: &str,
        filter: Option<SearchFilter>,
    ) -> Result<GroupedSearchResult, LibraryServiceError> {
        let trimmed = query.trim();
        if trimmed.is_empty() {
            return Err(LibraryServiceError::ValidationError(
                "query must not be empty".to_string(),
            ));
        }

        let rows = LocalTrackRepository::new(self.db.clone()).search(trimmed)?;
        let matching_tracks: Vec<Track> = rows
            .into_iter()
            .map(|r| deserialize_track(&r.track_json))
            .collect::<Result<_, _>>()?;

        let include_all = filter.is_none();
        let include_songs = include_all || filter == Some(SearchFilter::Songs);
        let include_artists = include_all || filter == Some(SearchFilter::Artists);
        let include_albums = include_all || filter == Some(SearchFilter::Albums);

        let songs = if include_songs {
            matching_tracks.clone()
        } else {
            Vec::new()
        };

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
            has_more_songs: false,
        })
    }

    /// Get full artist detail by artist ID.
    ///
    /// Resolves the artist name from the ID, loads all tracks by that artist,
    /// and computes top tracks by play count (ties broken alphabetically).
    pub fn get_artist_detail(&self, id: &str) -> Result<ArtistDetail, LibraryServiceError> {
        let normalized_name = denormalize_artist_id(id)
            .ok_or_else(|| LibraryServiceError::NotFound(id.to_string()))?;

        let rows = LocalTrackRepository::new(self.db.clone()).get_by_artist(&normalized_name)?;
        let mut tracks: Vec<Track> = rows
            .into_iter()
            .map(|r| deserialize_track(&r.track_json))
            .collect::<Result<_, _>>()?;
        if tracks.is_empty() {
            return Err(LibraryServiceError::NotFound(id.to_string()));
        }

        let canonical_name = tracks[0].artist.clone();

        let play_counts = HistoryRepository::new(self.db.clone()).play_counts()?;

        tracks.sort_by(|a, b| {
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
            top_tracks: tracks,
            albums,
        })
    }

    /// Get full album detail by album ID.
    ///
    /// Resolves the album title and artist from the ID, loads matching tracks,
    /// and orders them by file path (which usually reflects track order).
    pub fn get_album_detail(&self, id: &str) -> Result<AlbumDetail, LibraryServiceError> {
        let (normalized_title, normalized_artist) = denormalize_album_id(id)
            .ok_or_else(|| LibraryServiceError::NotFound(id.to_string()))?;

        let rows = LocalTrackRepository::new(self.db.clone()).get_by_album(&normalized_title)?;
        let mut tracks: Vec<Track> = rows
            .into_iter()
            .map(|r| deserialize_track(&r.track_json))
            .collect::<Result<_, _>>()?;
        if tracks.is_empty() {
            return Err(LibraryServiceError::NotFound(id.to_string()));
        }

        // Engine's `get_by_album` matches by album title pattern; filter by
        // artist client-side to mirror desktop's behavior exactly.
        tracks.retain(|t| t.artist.to_lowercase() == normalized_artist);

        if tracks.is_empty() {
            return Err(LibraryServiceError::NotFound(id.to_string()));
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

    // ── Home snapshot ──────────────────────────────────────────────────

    const RECENTLY_PLAYED_LIMIT: usize = 20;
    const ARTIST_AFFINITY_LIMIT: usize = 8;
    const ALBUM_AFFINITY_LIMIT: usize = 4;
    const LIBRARY_DISCOVERY_LIMIT: usize = 4;
    const RECOMMENDATIONS_LIMIT: usize = 20;

    /// Get the Home snapshot: recently played only (non-blocking).
    ///
    /// Recommendations are intentionally omitted here so the Home page
    /// renders immediately. Use `get_home_recommendations` for the heavy
    /// computation.
    pub fn get_home_snapshot(&self) -> Result<HomeSnapshot, LibraryServiceError> {
        let recently_played = HistoryRepository::new(self.db.clone())
            .get_recent_unique(Self::RECENTLY_PLAYED_LIMIT as u32)?;
        Ok(HomeSnapshot {
            recently_played,
            recommendations: vec![],
        })
    }

    /// Compute heavy recommendations from history and local library.
    pub fn get_home_recommendations(&self) -> Result<Vec<RecommendationItem>, LibraryServiceError> {
        let history_rows = HistoryRepository::new(self.db.clone()).get()?;
        let history: Vec<Track> = history_rows
            .into_iter()
            .map(|r| deserialize_track(&r.track_json))
            .collect::<Result<_, _>>()?;

        let local_rows = LocalTrackRepository::new(self.db.clone()).get_all(None)?;
        let local_tracks: Vec<Track> = local_rows
            .into_iter()
            .map(|r| deserialize_track(&r.track_json))
            .collect::<Result<_, _>>()?;

        Ok(self.build_recommendations(&history, &local_tracks))
    }

    /// Assemble recommendations from history and local library.
    fn build_recommendations(
        &self,
        history: &[Track],
        local_tracks: &[Track],
    ) -> Vec<RecommendationItem> {
        let recent_track_ids: HashSet<String> = history
            .iter()
            .take(Self::RECENTLY_PLAYED_LIMIT)
            .map(|track| track.id.clone())
            .collect();

        let mut recommended_ids: HashSet<String> = HashSet::new();
        let mut recommendations: Vec<RecommendationItem> = Vec::new();

        // 1. Artist affinity
        let artist_counts = count_artists_in_history(history);
        let mut artists_by_plays: Vec<(&String, &usize)> = artist_counts.iter().collect();
        artists_by_plays.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));

        for (artist, _count) in artists_by_plays.iter().take(Self::ARTIST_AFFINITY_LIMIT) {
            let artist_tracks: Vec<&Track> = local_tracks
                .iter()
                .filter(|track| track.artist == **artist)
                .collect();
            let total_count = artist_tracks.len() as u32;
            if total_count == 0 {
                continue;
            }
            let id = normalize_artist_id(artist);
            if recommended_ids.insert(id.clone()) {
                let reason = format!("Because you listened to {}", artist);
                recommendations.push(RecommendationItem::Artist {
                    id,
                    name: (*artist).clone(),
                    thumbnail: None,
                    track_count: total_count,
                    reason,
                });
            }
        }

        // 2. Album affinity
        let album_counts = count_albums_in_history(history);
        let mut albums_by_plays: Vec<((String, String), usize)> =
            album_counts.into_iter().collect();
        albums_by_plays.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.0.cmp(&b.0.0)));

        for ((artist, album), _count) in albums_by_plays.iter().take(Self::ALBUM_AFFINITY_LIMIT) {
            let album_tracks: Vec<&Track> = local_tracks
                .iter()
                .filter(|track| track.artist == *artist && track.album.as_ref() == Some(album))
                .collect();
            if album_tracks.is_empty() {
                continue;
            }
            let id = normalize_album_id(artist, album);
            if recommended_ids.insert(id.clone()) {
                let reason = "Based on your listening".to_string();
                recommendations.push(RecommendationItem::Album {
                    id,
                    title: album.clone(),
                    artist: artist.clone(),
                    cover: None,
                    track_count: album_tracks.len() as u32,
                    reason,
                });
            }
        }

        // 3. Library discovery
        let seed = daily_seed();
        let mut rng = StdRng::seed_from_u64(seed);
        let mut candidates: Vec<&Track> = local_tracks
            .iter()
            .filter(|track| {
                !recent_track_ids.contains(&track.id) && !recommended_ids.contains(&track.id)
            })
            .collect();
        candidates.shuffle(&mut rng);

        for track in candidates.iter().take(Self::LIBRARY_DISCOVERY_LIMIT) {
            if recommended_ids.insert(track.id.clone()) {
                recommendations.push(RecommendationItem::Track {
                    track: (*track).clone(),
                    reason: "Discover from your library".to_string(),
                });
            }
        }

        recommendations.truncate(Self::RECOMMENDATIONS_LIMIT);
        recommendations
    }
}

fn deserialize_track(json: &str) -> Result<Track, LibraryServiceError> {
    serde_json::from_str(json).map_err(LibraryServiceError::from)
}

fn count_artists_in_history(history: &[Track]) -> HashMap<String, usize> {
    let mut counts = HashMap::new();
    for track in history.iter() {
        *counts.entry(track.artist.clone()).or_insert(0) += 1;
    }
    counts
}

fn count_albums_in_history(history: &[Track]) -> HashMap<(String, String), usize> {
    let mut counts = HashMap::new();
    for track in history.iter() {
        if let Some(album) = track.album.as_ref() {
            *counts
                .entry((track.artist.clone(), album.clone()))
                .or_insert(0) += 1;
        }
    }
    counts
}

fn daily_seed() -> u64 {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    now / 86400
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sqlite::SqliteHandle;
    use jellyx_core::models::source::Source;
    use rusqlite::Connection;

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
            playlist_id: None,
            metadata: HashMap::new(),
        }
    }

    fn local_track(id: &str, title: &str, artist: &str, album: &str, path: &str) -> Track {
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
            playlist_id: None,
            metadata: HashMap::new(),
        }
    }

    fn fresh_handle() -> SqliteHandle {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE _meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
             INSERT INTO _meta (key, value) VALUES ('schema_version', '0');
             CREATE TABLE history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                track_id TEXT NOT NULL,
                track_json TEXT NOT NULL,
                played_at TEXT NOT NULL DEFAULT (datetime('now'))
            );
             CREATE INDEX IF NOT EXISTS idx_history_played_at ON history(played_at DESC);
             CREATE TABLE watched_folders (
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
             CREATE INDEX IF NOT EXISTS idx_local_tracks_folder ON local_tracks(folder_path);",
        )
        .unwrap();
        // Seed a watched folder so FK constraints pass.
        conn.execute("INSERT INTO watched_folders (path) VALUES ('/music')", [])
            .unwrap();
        SqliteHandle::new(conn)
    }

    fn setup_service() -> LibraryService {
        LibraryService::new(fresh_handle())
    }

    fn insert_local_tracks(svc: &LibraryService, tracks: &[Track]) {
        let repo = LocalTrackRepository::new(svc.db.clone());
        for t in tracks {
            let path = t.local_path.as_ref().unwrap();
            let json = serde_json::to_string(t).unwrap();
            repo.upsert(path, &json, "/music", None, None).unwrap();
        }
    }

    fn record_play(svc: &LibraryService, track: &Track) {
        svc.record_play(track).unwrap();
    }

    // ── search_grouped ─────────────────────────────────────────────────

    #[test]
    fn search_grouped_returns_mixed_groups() {
        let svc = setup_service();
        let tracks = vec![
            local_track(
                "t1",
                "One More Time",
                "Daft Punk",
                "Discovery",
                "/music/one.mp3",
            ),
            local_track(
                "t2",
                "Harder Better",
                "Daft Punk",
                "Discovery",
                "/music/harder.mp3",
            ),
            local_track(
                "t3",
                "Bohemian Rhapsody",
                "Queen",
                "A Night at the Opera",
                "/music/bohemian.mp3",
            ),
        ];
        insert_local_tracks(&svc, &tracks);

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
            local_track(
                "t1",
                "Bohemian Rhapsody",
                "Queen",
                "A Night at the Opera",
                "/music/bohemian.mp3",
            ),
            local_track(
                "t2",
                "We Will Rock You",
                "Queen",
                "News of the World",
                "/music/rockyou.mp3",
            ),
        ];
        insert_local_tracks(&svc, &tracks);

        let result = svc
            .search_grouped("queen", Some(SearchFilter::Artists))
            .unwrap();
        assert!(result.songs.is_empty());
        assert_eq!(result.artists.len(), 1);
        assert!(result.albums.is_empty());
    }

    #[test]
    fn search_grouped_filter_albums_only() {
        let svc = setup_service();
        let tracks = vec![
            local_track(
                "t1",
                "One More Time",
                "Daft Punk",
                "Discovery",
                "/music/one.mp3",
            ),
            local_track(
                "t2",
                "Harder Better",
                "Daft Punk",
                "Discovery",
                "/music/harder.mp3",
            ),
        ];
        insert_local_tracks(&svc, &tracks);

        let result = svc
            .search_grouped("discovery", Some(SearchFilter::Albums))
            .unwrap();
        assert!(result.songs.is_empty());
        assert!(result.artists.is_empty());
        assert_eq!(result.albums.len(), 1);
        assert_eq!(result.albums[0].track_count, 2);
    }

    #[test]
    fn search_grouped_rejects_empty_query() {
        let svc = setup_service();
        let result = svc.search_grouped("   ", None);
        assert!(matches!(
            result,
            Err(LibraryServiceError::ValidationError(_))
        ));
    }

    // ── get_artist_detail ──────────────────────────────────────────────

    #[test]
    fn get_artist_detail_returns_top_tracks_and_albums() {
        let svc = setup_service();
        let tracks = vec![
            local_track(
                "t1",
                "One More Time",
                "Daft Punk",
                "Discovery",
                "/music/one.mp3",
            ),
            local_track(
                "t2",
                "Harder Better",
                "Daft Punk",
                "Discovery",
                "/music/harder.mp3",
            ),
            local_track(
                "t3",
                "Aerodynamic",
                "Daft Punk",
                "Discovery",
                "/music/aero.mp3",
            ),
        ];
        insert_local_tracks(&svc, &tracks);
        // t1 played twice, t2 once → t1 should be the first top track.
        record_play(&svc, &tracks[0]);
        record_play(&svc, &tracks[0]);
        record_play(&svc, &tracks[1]);

        let id = normalize_artist_id("Daft Punk");
        let detail = svc.get_artist_detail(&id).unwrap();
        assert_eq!(detail.name, "Daft Punk");
        assert_eq!(detail.top_tracks.len(), 3);
        assert_eq!(
            detail.top_tracks[0].id, "t1",
            "Most-played track should be first"
        );
        assert_eq!(detail.albums.len(), 1);
        assert_eq!(detail.albums[0].title, "Discovery");
    }

    #[test]
    fn get_artist_detail_not_found_for_unknown_artist() {
        let svc = setup_service();
        let result = svc.get_artist_detail("artist:ghost-band");
        assert!(matches!(result, Err(LibraryServiceError::NotFound(_))));
    }

    // ── get_album_detail ───────────────────────────────────────────────

    #[test]
    fn get_album_detail_returns_tracks_in_order() {
        let svc = setup_service();
        let tracks = vec![
            local_track(
                "t1",
                "One More Time",
                "Daft Punk",
                "Discovery",
                "/music/01-one.mp3",
            ),
            local_track(
                "t2",
                "Aerodynamic",
                "Daft Punk",
                "Discovery",
                "/music/02-aero.mp3",
            ),
            local_track(
                "t3",
                "Digital Love",
                "Daft Punk",
                "Discovery",
                "/music/03-digital.mp3",
            ),
        ];
        insert_local_tracks(&svc, &tracks);

        let id = normalize_album_id("Discovery", "Daft Punk");
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
        assert!(matches!(result, Err(LibraryServiceError::NotFound(_))));
    }

    #[test]
    fn get_album_detail_filters_by_artist_when_album_title_matches_multiple() {
        let svc = setup_service();
        // Two different artists with the same album title "Greatest Hits".
        let daft = local_track(
            "t1",
            "One More Time",
            "Daft Punk",
            "Greatest Hits",
            "/music/dp/01.mp3",
        );
        let queen = local_track(
            "t2",
            "Bohemian Rhapsody",
            "Queen",
            "Greatest Hits",
            "/music/q/01.mp3",
        );
        insert_local_tracks(&svc, &[daft.clone(), queen]);

        let id = normalize_album_id("Greatest Hits", "Daft Punk");
        let detail = svc.get_album_detail(&id).unwrap();
        assert_eq!(detail.artist, "Daft Punk");
        assert_eq!(detail.tracks.len(), 1);
        assert_eq!(detail.tracks[0].id, "t1");
    }

    // ── history + home ──────────────────────────────────────────────────

    #[test]
    fn record_play_and_get_recent_unique() {
        let svc = setup_service();
        record_play(&svc, &sample_track("t1"));
        record_play(&svc, &sample_track("t2"));

        let rows = svc.get_recent_unique(10).unwrap();
        assert_eq!(rows.len(), 2);
        // Most recent first.
        assert_eq!(rows[0].track_id, "t2");
        assert_eq!(rows[1].track_id, "t1");
    }

    #[test]
    fn clear_history_removes_all() {
        let svc = setup_service();
        record_play(&svc, &sample_track("t1"));
        record_play(&svc, &sample_track("t2"));
        svc.clear_history().unwrap();
        assert!(svc.get_recent_unique(10).unwrap().is_empty());
    }

    #[test]
    fn get_home_snapshot_recently_played_capped_and_recency_order() {
        let svc = setup_service();
        for i in 0..25 {
            record_play(&svc, &sample_track(&format!("hist-{}", i)));
            std::thread::sleep(std::time::Duration::from_millis(2));
        }

        let snapshot = svc.get_home_snapshot().unwrap();
        assert_eq!(snapshot.recently_played.len(), 20, "Should cap at 20");
        assert_eq!(
            snapshot.recently_played[0].track_id, "hist-24",
            "Most recent first"
        );
        assert!(snapshot.recommendations.is_empty());
    }

    #[test]
    fn get_home_recommendations_artist_affinity() {
        let svc = setup_service();
        for i in 0..3 {
            let track = local_track(
                &format!("local-{}", i),
                &format!("Song {}", i),
                "Affinity Artist",
                "Album A",
                &format!("/music/{}.mp3", i),
            );
            let json = serde_json::to_string(&track).unwrap();
            LocalTrackRepository::new(svc.db.clone())
                .upsert(
                    &track.local_path.clone().unwrap(),
                    &json,
                    "/music",
                    None,
                    None,
                )
                .unwrap();
        }
        let played = local_track(
            "hist-0",
            "Song hist-0",
            "Affinity Artist",
            "Album A",
            "/music/h0.mp3",
        );
        record_play(&svc, &played);
        std::thread::sleep(std::time::Duration::from_millis(5));
        record_play(&svc, &played);

        let recs = svc.get_home_recommendations().unwrap();
        let has_artist = recs.iter().any(|item| match item {
            RecommendationItem::Artist { name, .. } if name == "Affinity Artist" => true,
            _ => false,
        });
        assert!(has_artist, "Should recommend artist affinity item");
    }

    #[test]
    fn get_home_recommendations_excludes_recently_played_when_alternatives_exist() {
        let svc = setup_service();
        let played = local_track(
            "track-played",
            "Song played",
            "Same Artist",
            "Album X",
            "/music/played.mp3",
        );
        let alternative = local_track(
            "track-alt",
            "Song alt",
            "Same Artist",
            "Album X",
            "/music/alt.mp3",
        );
        for t in [&played, &alternative] {
            let json = serde_json::to_string(t).unwrap();
            LocalTrackRepository::new(svc.db.clone())
                .upsert(t.local_path.as_ref().unwrap(), &json, "/music", None, None)
                .unwrap();
        }

        record_play(&svc, &played);

        let recs = svc.get_home_recommendations().unwrap();
        let has_played_track = recs.iter().any(|item| match item {
            RecommendationItem::Track { track, .. } if track.id == "track-played" => true,
            _ => false,
        });
        assert!(
            !has_played_track,
            "Should not recommend the exact recently played track"
        );
    }

    #[test]
    fn get_home_recommendations_falls_back_to_library_discovery() {
        let svc = setup_service();
        for i in 0..5 {
            let track = local_track(
                &format!("lib-{}", i),
                &format!("Song {}", i),
                "Library Artist",
                "Album",
                &format!("/music/{}.mp3", i),
            );
            let json = serde_json::to_string(&track).unwrap();
            LocalTrackRepository::new(svc.db.clone())
                .upsert(
                    &track.local_path.clone().unwrap(),
                    &json,
                    "/music",
                    None,
                    None,
                )
                .unwrap();
        }

        let recs = svc.get_home_recommendations().unwrap();
        assert!(
            !recs.is_empty(),
            "Empty history should still produce recommendations"
        );
        let has_library_track = recs.iter().any(|item| {
            matches!(item,
                RecommendationItem::Track { reason, .. } if reason.contains("library"))
        });
        assert!(
            has_library_track,
            "Should include library discovery fallback items"
        );
    }

    #[test]
    fn get_home_recommendations_capped_at_20() {
        let svc = setup_service();
        for i in 0..60 {
            let track = local_track(
                &format!("lib-{}", i),
                &format!("Song {}", i),
                &format!("Artist {}", i % 5),
                "Album",
                &format!("/music/{}.mp3", i),
            );
            let json = serde_json::to_string(&track).unwrap();
            LocalTrackRepository::new(svc.db.clone())
                .upsert(
                    &track.local_path.clone().unwrap(),
                    &json,
                    "/music",
                    None,
                    None,
                )
                .unwrap();
        }
        // Add some history for affinity signals.
        for i in 0..5 {
            record_play(&svc, &sample_track(&format!("hist-{}", i)));
            std::thread::sleep(std::time::Duration::from_millis(2));
        }

        let recs = svc.get_home_recommendations().unwrap();
        assert!(recs.len() <= 20, "Recommendations should be capped at 20");
    }

    // ── error mapping ──────────────────────────────────────────────────

    #[test]
    fn library_service_error_display_and_from_impls() {
        let persistence: LibraryServiceError = rusqlite::Error::InvalidColumnIndex(0).into();
        assert!(matches!(persistence, LibraryServiceError::Persistence(_)));

        let json: LibraryServiceError = serde_json::from_str::<Track>("not json")
            .unwrap_err()
            .into();
        assert!(matches!(json, LibraryServiceError::Deserialization(_)));

        let not_found = LibraryServiceError::NotFound("x".into());
        assert!(not_found.to_string().contains("not found: x"));
    }
}
