//! Library service — desktop adapter over the engine `LibraryService`.
//!
//! Desktop's `LibraryService` is a thin adapter that owns an `Arc<Database>`,
//! obtains a `SqliteHandle` from it, and delegates all business logic to
//! [`jellyx_engine::library_service::LibraryService`].
//!
//! Responsibilities kept here (presentation/IPC concerns):
//! - Map engine DTOs to desktop IPC DTOs (`crate::ipc::dto::*`).
//! - Deserialize `Track` from `HistoryRow::track_json` to build
//!   `HistoryEntry` (the IPC-shaped history type the frontend expects).
//! - Map engine `LibraryServiceError` to `AppError` for consistent IPC
//!   error handling.
//!
//! All business logic (search grouping, artist/album detail, home
//! recommendations) lives in the engine so both Tauri and Ratatui frontends
//! share a single implementation.

use std::sync::Arc;

use jellyx_core::models::track::Track;
use jellyx_engine::dto as engine_dto;
use jellyx_engine::library_service::{LibraryService as EngineLibraryService, LibraryServiceError};

use crate::errors::types::{AppError, LibraryError, ValidationError};
use crate::ipc::dto::{
    AlbumDetail, AlbumSummary, ArtistDetail, ArtistSummary, GroupedSearchResult, HomeSnapshot,
    RecommendationItem, SearchFilter,
};
use crate::persistence::db::Database;
use crate::persistence::models::HistoryEntry;

/// Service providing library operations (favorites, history).
///
/// Owns an `Arc<Database>` shared reference so it can be cheaply cloned
/// if needed in the future. All methods are synchronous since SQLite
/// operations are fast and the WAL mode handles concurrency.
pub struct LibraryService {
    /// Kept for direct `Database` access in tests; production code uses
    /// `engine` exclusively.
    #[cfg_attr(not(test), allow(dead_code))]
    db: Arc<Database>,
    engine: EngineLibraryService,
}

impl LibraryService {
    /// Create a new LibraryService backed by the given Database.
    pub fn new(db: Arc<Database>) -> Self {
        let engine = EngineLibraryService::new(db.handle());
        Self { db, engine }
    }

    /// Record a play event in history.
    #[allow(dead_code)]
    pub fn record_play(&self, track: &Track) -> Result<(), AppError> {
        self.engine.record_play(track).map_err(AppError::from)
    }

    /// Get recently played tracks deduplicated by track_id.
    ///
    /// Returns only the most recent entry per track so the same track doesn't
    /// appear multiple times in the "recently played" list. The engine returns
    /// raw `HistoryRow`s; this adapter deserializes `track_json` into a `Track`
    /// to build the `HistoryEntry` the IPC layer expects.
    pub fn get_recent_unique(&self, limit: u32) -> Result<Vec<HistoryEntry>, AppError> {
        let rows = self
            .engine
            .get_recent_unique(limit)
            .map_err(AppError::from)?;
        rows.into_iter()
            .map(|row| {
                let track: Track = serde_json::from_str(&row.track_json).map_err(|e| AppError {
                    code: "PERSISTENCE_ERROR".into(),
                    details: Some(format!("failed to deserialize track: {}", e)),
                })?;
                Ok(HistoryEntry {
                    id: row.id,
                    track,
                    played_at: row.played_at,
                })
            })
            .collect()
    }

    /// Clear all play history.
    pub fn clear_history(&self) -> Result<(), AppError> {
        self.engine.clear_history().map_err(AppError::from)
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
        let engine_filter = filter.map(map_search_filter);
        let engine_result = self
            .engine
            .search_grouped(query, engine_filter)
            .map_err(AppError::from)?;
        Ok(map_grouped_search_result(engine_result))
    }

    /// Get full artist detail by artist ID.
    ///
    /// Resolves the artist name from the ID, loads all tracks by that artist,
    /// and computes top tracks by play count (ties broken alphabetically).
    pub fn get_artist_detail(&self, id: &str) -> Result<ArtistDetail, AppError> {
        let engine_detail = self.engine.get_artist_detail(id).map_err(AppError::from)?;
        Ok(map_artist_detail(engine_detail))
    }

    /// Get full album detail by album ID.
    ///
    /// Resolves the album title and artist from the ID, loads matching tracks,
    /// and orders them by file path (which usually reflects track order).
    pub fn get_album_detail(&self, id: &str) -> Result<AlbumDetail, AppError> {
        let engine_detail = self.engine.get_album_detail(id).map_err(AppError::from)?;
        Ok(map_album_detail(engine_detail))
    }

    // ── Home snapshot ──────────────────────────────────────────────────

    /// Get the Home snapshot: recently played only (non-blocking).
    ///
    /// Recommendations are intentionally omitted here so the Home page
    /// renders immediately. Use `get_home_recommendations` for the heavy
    /// computation.
    pub fn get_home_snapshot(&self) -> Result<HomeSnapshot, AppError> {
        let engine_snapshot = self.engine.get_home_snapshot().map_err(AppError::from)?;
        let recently_played = engine_snapshot
            .recently_played
            .into_iter()
            .map(|row| {
                let track: Track = serde_json::from_str(&row.track_json).map_err(|e| AppError {
                    code: "PERSISTENCE_ERROR".into(),
                    details: Some(format!("failed to deserialize track: {}", e)),
                })?;
                Ok(HistoryEntry {
                    id: row.id,
                    track,
                    played_at: row.played_at,
                })
            })
            .collect::<Result<Vec<_>, AppError>>()?;
        Ok(HomeSnapshot {
            recently_played,
            recommendations: vec![],
        })
    }

    /// Compute heavy recommendations from history and local library.
    pub fn get_home_recommendations(&self) -> Result<Vec<RecommendationItem>, AppError> {
        let engine_recs = self
            .engine
            .get_home_recommendations()
            .map_err(AppError::from)?;
        Ok(engine_recs
            .into_iter()
            .map(map_recommendation_item)
            .collect())
    }

    // ── Direct database accessors used by tests ───────────────────────
    //
    // The desktop tests insert local tracks and history through the
    // `Database` directly. These accessors preserve that pattern without
    // leaking `Database` into the public API.

    #[cfg(test)]
    pub(crate) fn insert_watched_folder(&self, path: &str) -> Result<(), AppError> {
        self.db.insert_watched_folder(path).map_err(AppError::from)
    }

    #[cfg(test)]
    pub(crate) fn upsert_local_track(
        &self,
        file_path: &str,
        track: &Track,
        folder_path: &str,
        file_modified_at: Option<&str>,
        subfolder_path: Option<&str>,
    ) -> Result<(), AppError> {
        self.db
            .upsert_local_track(
                file_path,
                track,
                folder_path,
                file_modified_at,
                subfolder_path,
            )
            .map_err(AppError::from)
    }

    #[cfg(test)]
    pub(crate) fn insert_history(&self, track: &Track) -> Result<(), AppError> {
        self.db.insert_history(track).map_err(AppError::from)
    }

    #[cfg(test)]
    pub(crate) fn get_history(&self) -> Result<Vec<HistoryEntry>, AppError> {
        self.db.get_history().map_err(AppError::from)
    }
}

// ── Error mapping ─────────────────────────────────────────────────────

impl From<LibraryServiceError> for AppError {
    fn from(error: LibraryServiceError) -> Self {
        match error {
            LibraryServiceError::NotFound(msg) => LibraryError::NotFound(msg).into(),
            LibraryServiceError::ValidationError(msg) => {
                if msg.contains("empty") {
                    ValidationError::EmptyQuery.into()
                } else {
                    ValidationError::InvalidInput(msg).into()
                }
            }
            LibraryServiceError::Persistence(msg) => AppError {
                code: "PERSISTENCE_ERROR".into(),
                details: Some(msg),
            },
            LibraryServiceError::Deserialization(msg) => AppError {
                code: "PERSISTENCE_ERROR".into(),
                details: Some(format!("failed to deserialize track: {}", msg)),
            },
        }
    }
}

// ── DTO mapping ───────────────────────────────────────────────────────

fn map_search_filter(filter: SearchFilter) -> engine_dto::SearchFilter {
    match filter {
        SearchFilter::Songs => engine_dto::SearchFilter::Songs,
        SearchFilter::Artists => engine_dto::SearchFilter::Artists,
        SearchFilter::Albums => engine_dto::SearchFilter::Albums,
    }
}

fn map_grouped_search_result(result: engine_dto::GroupedSearchResult) -> GroupedSearchResult {
    GroupedSearchResult {
        songs: result.songs,
        artists: result.artists.into_iter().map(map_artist_summary).collect(),
        albums: result.albums.into_iter().map(map_album_summary).collect(),
        has_more_songs: result.has_more_songs,
    }
}

fn map_artist_summary(summary: engine_dto::ArtistSummary) -> ArtistSummary {
    ArtistSummary {
        id: summary.id,
        name: summary.name,
        thumbnail: summary.thumbnail,
        track_count: summary.track_count,
    }
}

fn map_album_summary(summary: engine_dto::AlbumSummary) -> AlbumSummary {
    AlbumSummary {
        id: summary.id,
        title: summary.title,
        artist: summary.artist,
        cover: summary.cover,
        year: summary.year,
        track_count: summary.track_count,
    }
}

fn map_artist_detail(detail: engine_dto::ArtistDetail) -> ArtistDetail {
    ArtistDetail {
        id: detail.id,
        name: detail.name,
        thumbnail: detail.thumbnail,
        top_tracks: detail.top_tracks,
        albums: detail.albums.into_iter().map(map_album_summary).collect(),
    }
}

fn map_album_detail(detail: engine_dto::AlbumDetail) -> AlbumDetail {
    AlbumDetail {
        id: detail.id,
        title: detail.title,
        artist: detail.artist,
        artist_id: detail.artist_id,
        cover: detail.cover,
        year: detail.year,
        tracks: detail.tracks,
    }
}

fn map_recommendation_item(item: engine_dto::RecommendationItem) -> RecommendationItem {
    match item {
        engine_dto::RecommendationItem::Track { track, reason } => {
            RecommendationItem::Track { track, reason }
        }
        engine_dto::RecommendationItem::Artist {
            id,
            name,
            thumbnail,
            track_count,
            reason,
        } => RecommendationItem::Artist {
            id,
            name,
            thumbnail,
            track_count,
            reason,
        },
        engine_dto::RecommendationItem::Album {
            id,
            title,
            artist,
            cover,
            track_count,
            reason,
        } => RecommendationItem::Album {
            id,
            title,
            artist,
            cover,
            track_count,
            reason,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ipc::dto::SearchFilter;
    use jellyx_core::models::source::Source;
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

    fn setup_service() -> LibraryService {
        let db = Database::open_in_memory().unwrap();
        LibraryService::new(Arc::new(db))
    }

    fn insert_local_tracks(svc: &LibraryService, tracks: &[Track], _folder: &str) {
        svc.insert_watched_folder(_folder).unwrap();
        for t in tracks {
            let path = t.local_path.as_ref().unwrap();
            svc.upsert_local_track(path, t, _folder, None, None)
                .unwrap();
        }
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

    // ── Grouped search tests (REQ-MS-1/2) ────────────────────────────────

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
        insert_local_tracks(&svc, &tracks, "/music");

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
        insert_local_tracks(&svc, &tracks, "/music");

        let result = svc
            .search_grouped("discovery", Some(SearchFilter::Albums))
            .unwrap();
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
        insert_local_tracks(&svc, &tracks, "/music");
        // t1 played twice, t2 once → t1 should be the first top track
        svc.insert_history(&tracks[0]).unwrap();
        svc.insert_history(&tracks[0]).unwrap();
        svc.insert_history(&tracks[1]).unwrap();

        let id = crate::ipc::dto::normalize_artist_id("Daft Punk");
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
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, "NOT_FOUND");
    }

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

    // ── Home snapshot tests ─────────────────────────────────────────────

    fn sample_local_track(id: &str, path: &str, artist: &str, album: Option<&str>) -> Track {
        Track {
            id: id.to_string(),
            source: Source::Local,
            source_id: path.to_string(),
            title: format!("Song {}", id),
            artist: artist.to_string(),
            album: album.map(|a| a.to_string()),
            duration: Some(180.0),
            thumbnail: None,
            stream_url: None,
            local_path: Some(path.to_string()),
            playlist_id: None,
            metadata: HashMap::new(),
        }
    }

    fn insert_history_at(svc: &LibraryService, track: &Track, _offset_ms: u64) {
        svc.record_play(track).unwrap();
    }

    #[test]
    fn get_home_snapshot_recently_played_max_20_and_recency_order() {
        let svc = setup_service();
        for i in 0..25 {
            let track = sample_track(&format!("hist-{}", i));
            insert_history_at(&svc, &track, i * 10);
            std::thread::sleep(std::time::Duration::from_millis(2));
        }

        let snapshot = svc.get_home_snapshot().unwrap();
        assert_eq!(snapshot.recently_played.len(), 20, "Should cap at 20");
        assert_eq!(
            snapshot.recently_played[0].track.id, "hist-24",
            "Most recent first"
        );
        assert_eq!(
            snapshot.recently_played[19].track.id, "hist-5",
            "Oldest in cap"
        );
        assert!(
            snapshot.recommendations.is_empty(),
            "Snapshot no longer carries recommendations"
        );
    }

    #[test]
    fn get_home_recommendations_artist_affinity() {
        let svc = setup_service();
        svc.insert_watched_folder("/music").unwrap();
        for i in 0..3 {
            let track = sample_local_track(
                &format!("local-{}", i),
                &format!("/music/{}.mp3", i),
                "Affinity Artist",
                Some("Album A"),
            );
            svc.upsert_local_track(
                &format!("/music/{}.mp3", i),
                &track,
                "/music",
                Some(&format!("100{}", i)),
                None,
            )
            .unwrap();
        }
        let played = sample_local_track(
            "hist-0",
            "/music/h0.mp3",
            "Affinity Artist",
            Some("Album A"),
        );
        insert_history_at(&svc, &played, 0);
        std::thread::sleep(std::time::Duration::from_millis(5));
        insert_history_at(&svc, &played, 10);

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
        svc.insert_watched_folder("/music").unwrap();
        let played = sample_local_track(
            "track-played",
            "/music/played.mp3",
            "Same Artist",
            Some("Album X"),
        );
        let alternative = sample_local_track(
            "track-alt",
            "/music/alt.mp3",
            "Same Artist",
            Some("Album X"),
        );
        svc.upsert_local_track("/music/played.mp3", &played, "/music", Some("1000"), None)
            .unwrap();
        svc.upsert_local_track("/music/alt.mp3", &alternative, "/music", Some("1001"), None)
            .unwrap();

        insert_history_at(&svc, &played, 0);

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
    fn get_home_recommendations_falls_back_to_library_discovery_with_empty_signals() {
        let svc = setup_service();
        svc.insert_watched_folder("/music").unwrap();
        for i in 0..5 {
            let track = sample_local_track(
                &format!("lib-{}", i),
                &format!("/music/{}.mp3", i),
                "Library Artist",
                None,
            );
            svc.upsert_local_track(
                &format!("/music/{}.mp3", i),
                &track,
                "/music",
                Some(&format!("100{}", i)),
                None,
            )
            .unwrap();
        }

        let recs = svc.get_home_recommendations().unwrap();
        assert!(
            !recs.is_empty(),
            "Empty history/favorites should still produce recommendations"
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
    fn get_home_snapshot_recommendations_max_20() {
        let svc = setup_service();
        svc.insert_watched_folder("/music").unwrap();
        for i in 0..60 {
            let track = sample_local_track(
                &format!("lib-{}", i),
                &format!("/music/{}.mp3", i),
                &format!("Artist {}", i % 5),
                Some("Album"),
            );
            svc.upsert_local_track(
                &format!("/music/{}.mp3", i),
                &track,
                "/music",
                Some(&format!("100{}", i)),
                None,
            )
            .unwrap();
        }
        // Add some history for affinity signals
        for i in 0..5 {
            let track = sample_track(&format!("hist-{}", i));
            insert_history_at(&svc, &track, i as u64 * 10);
            std::thread::sleep(std::time::Duration::from_millis(2));
        }

        let recs = svc.get_home_recommendations().unwrap();
        assert!(recs.len() <= 20, "Recommendations should be capped at 20");
    }
}
