//! Library service — CRUD for favorites, history.
//!
//! `LibraryService` wraps the `Database` and provides business-logic-level
//! operations for favorites and history. All methods return `AppError`
//! for consistent IPC error handling.
//!
//! Favorite operations use a **source-aware key**: `track.id` for local tracks
//! (stable UUID) and `track.source_id` for remote tracks (stable resolver ID).
//! This ensures that remote tracks, which get a new internal UUID on each
//! resolution, can still be reliably favorited and unfavorited.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::SeedableRng;

use crate::errors::types::{AppError, LibraryError};
use crate::ipc::dto::{
    normalize_album_id, normalize_artist_id, AlbumDetail, AlbumSummary, ArtistDetail,
    ArtistSummary, GroupedSearchResult, HomeSnapshot, RecommendationItem, SearchFilter,
};
use crate::models::track::Track;
use crate::persistence::db::Database;
use crate::persistence::models::{HistoryEntry, LocalTrackEntry};
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

    /// Record a play event in history.
    #[allow(dead_code)]
    pub fn record_play(&self, track: &Track) -> Result<(), AppError> {
        self.db.insert_history(track).map_err(AppError::from)
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

        let matching_tracks = self
            .db
            .search_local_tracks(trimmed)
            .map_err(AppError::from)?;

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

    // ── Home snapshot ──────────────────────────────────────────────────

    const RECENTLY_PLAYED_LIMIT: usize = 20;
    const ARTIST_AFFINITY_LIMIT: usize = 8;
    const ALBUM_AFFINITY_LIMIT: usize = 4;
    const FAVORITE_DISCOVERY_LIMIT: usize = 4;
    const LIBRARY_DISCOVERY_LIMIT: usize = 4;
    const RECOMMENDATIONS_LIMIT: usize = 20;
    const SECONDS_PER_DAY: u64 = 86400;

    /// Get the Home snapshot: recently played only (non-blocking).
    ///
    /// Recommendations are intentionally omitted here so the Home page
    /// renders immediately. Use `get_home_recommendations` for the heavy
    /// computation.
    pub fn get_home_snapshot(&self) -> Result<HomeSnapshot, AppError> {
        let history = self.db.get_history().map_err(AppError::from)?;
        let recently_played = history
            .iter()
            .take(Self::RECENTLY_PLAYED_LIMIT)
            .cloned()
            .collect();
        Ok(HomeSnapshot {
            recently_played,
            recommendations: vec![],
        })
    }

    /// Compute heavy recommendations from history and local library.
    pub fn get_home_recommendations(&self) -> Result<Vec<RecommendationItem>, AppError> {
        let history = self.db.get_history().map_err(AppError::from)?;
        let local_tracks = self.db.get_all_local_tracks().map_err(AppError::from)?;
        Ok(self.build_recommendations(&history, &local_tracks))
    }

    /// Assemble recommendations from history and local library.
    fn build_recommendations(
        &self,
        history: &[HistoryEntry],
        local_tracks: &[LocalTrackEntry],
    ) -> Vec<RecommendationItem> {
        let recent_track_ids: HashSet<String> = history
            .iter()
            .take(Self::RECENTLY_PLAYED_LIMIT)
            .map(|entry| entry.track.id.clone())
            .collect();

        let mut recommended_ids: HashSet<String> = HashSet::new();
        let mut recommendations: Vec<RecommendationItem> = Vec::new();

        // 1. Artist affinity
        let artist_counts = count_artists_in_history(history);
        let mut artists_by_plays: Vec<(&String, &usize)> = artist_counts.iter().collect();
        artists_by_plays.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));

        for (artist, _count) in artists_by_plays.iter().take(Self::ARTIST_AFFINITY_LIMIT) {
            let artist_tracks: Vec<&LocalTrackEntry> = local_tracks
                .iter()
                .filter(|entry| entry.track.artist == **artist)
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
        albums_by_plays.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0 .0.cmp(&b.0 .0)));

        for ((artist, album), _count) in albums_by_plays.iter().take(Self::ALBUM_AFFINITY_LIMIT) {
            let album_tracks: Vec<&LocalTrackEntry> = local_tracks
                .iter()
                .filter(|entry| {
                    entry.track.artist == *artist && entry.track.album.as_ref() == Some(album)
                })
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
        let mut candidates: Vec<&LocalTrackEntry> = local_tracks
            .iter()
            .filter(|entry| {
                !recent_track_ids.contains(&entry.track.id)
                    && !recommended_ids.contains(&entry.track.id)
            })
            .collect();
        candidates.shuffle(&mut rng);

        for entry in candidates.iter().take(Self::LIBRARY_DISCOVERY_LIMIT) {
            if recommended_ids.insert(entry.track.id.clone()) {
                recommendations.push(RecommendationItem::Track {
                    track: entry.track.clone(),
                    reason: "Discover from your library".to_string(),
                });
            }
        }

        recommendations.truncate(Self::RECOMMENDATIONS_LIMIT);
        recommendations
    }
}

fn count_artists_in_history(history: &[HistoryEntry]) -> HashMap<String, usize> {
    let mut counts = HashMap::new();
    for entry in history.iter() {
        *counts.entry(entry.track.artist.clone()).or_insert(0) += 1;
    }
    counts
}

fn count_albums_in_history(history: &[HistoryEntry]) -> HashMap<(String, String), usize> {
    let mut counts = HashMap::new();
    for entry in history.iter() {
        if let Some(album) = entry.track.album.as_ref() {
            *counts
                .entry((entry.track.artist.clone(), album.clone()))
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
        svc.db.insert_watched_folder(_folder).unwrap();
        for t in tracks {
            let path = t.local_path.as_ref().unwrap();
            svc.db.upsert_local_track(path, t, _folder, None).unwrap();
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
        svc.db.insert_history(&tracks[0]).unwrap();
        svc.db.insert_history(&tracks[0]).unwrap();
        svc.db.insert_history(&tracks[1]).unwrap();

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
        svc.db.insert_watched_folder("/music").unwrap();
        for i in 0..3 {
            let track = sample_local_track(
                &format!("local-{}", i),
                &format!("/music/{}.mp3", i),
                "Affinity Artist",
                Some("Album A"),
            );
            svc.db
                .upsert_local_track(
                    &format!("/music/{}.mp3", i),
                    &track,
                    "/music",
                    Some(&format!("100{}", i)),
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
        svc.db.insert_watched_folder("/music").unwrap();
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
        svc.db
            .upsert_local_track("/music/played.mp3", &played, "/music", Some("1000"))
            .unwrap();
        svc.db
            .upsert_local_track("/music/alt.mp3", &alternative, "/music", Some("1001"))
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
        svc.db.insert_watched_folder("/music").unwrap();
        for i in 0..5 {
            let track = sample_local_track(
                &format!("lib-{}", i),
                &format!("/music/{}.mp3", i),
                "Library Artist",
                None,
            );
            svc.db
                .upsert_local_track(
                    &format!("/music/{}.mp3", i),
                    &track,
                    "/music",
                    Some(&format!("100{}", i)),
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
        svc.db.insert_watched_folder("/music").unwrap();
        for i in 0..60 {
            let track = sample_local_track(
                &format!("lib-{}", i),
                &format!("/music/{}.mp3", i),
                &format!("Artist {}", i % 5),
                Some("Album"),
            );
            svc.db
                .upsert_local_track(
                    &format!("/music/{}.mp3", i),
                    &track,
                    "/music",
                    Some(&format!("100{}", i)),
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
