//! Library service — CRUD for favorites, history.
//!
//! `LibraryService` wraps the `Database` and provides business-logic-level
//! operations for favorites and history. All methods return `AppError`
//! for consistent IPC error handling.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::SeedableRng;

use crate::errors::types::{AppError, LibraryError};
use crate::ipc::dto::{HomeSnapshot, RecommendationItem};
use crate::models::track::Track;
use crate::persistence::db::Database;
use crate::persistence::models::{FavoriteEntry, HistoryEntry, LocalTrackEntry};

/// Maximum items in the recently played section.
const RECENTLY_PLAYED_LIMIT: usize = 20;
/// Maximum recommendation items.
const RECOMMENDATIONS_LIMIT: usize = 20;
/// Maximum artist affinity recommendations.
const ARTIST_AFFINITY_LIMIT: usize = 8;
/// Maximum album affinity recommendations.
const ALBUM_AFFINITY_LIMIT: usize = 4;
/// Maximum favorite discovery recommendations.
const FAVORITE_DISCOVERY_LIMIT: usize = 4;
/// Maximum library discovery recommendations.
const LIBRARY_DISCOVERY_LIMIT: usize = 4;
/// Seconds in a day, used to derive a deterministic daily seed.
const SECONDS_PER_DAY: u64 = 86_400;

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

    /// Build the Home snapshot: recently played + recommendations.
    pub fn get_home_snapshot(&self) -> Result<HomeSnapshot, AppError> {
        let history = self.db.get_history().map_err(AppError::from)?;
        let favorites = self.db.get_favorites().map_err(AppError::from)?;
        let local_tracks = self.db.get_all_local_tracks().map_err(AppError::from)?;

        let recently_played = history.iter().take(RECENTLY_PLAYED_LIMIT).cloned().collect();
        let recommendations = self.build_recommendations(&history,&favorites,&local_tracks,
        );

        Ok(HomeSnapshot {
            recently_played,
            recommendations,
        })
    }

    /// Assemble recommendations from history, favorites, and local library.
    ///
    /// Heuristics are deterministic and capped per category:
    /// 1. Artist affinity (max 8): artists with the most plays in recent history.
    /// 2. Album affinity (max 4): albums with multiple plays in recent history.
    /// 3. Favorite discovery (max 4): other tracks by favorite artists.
    /// 4. Library discovery (max 4): deterministic daily-seeded picks from local tracks.
    fn build_recommendations(
        &self,
        history: &[HistoryEntry],
        favorites: &[FavoriteEntry],
        local_tracks: &[LocalTrackEntry],
    ) -> Vec<RecommendationItem> {
        let recent_track_ids: HashSet<String> = history
            .iter()
            .take(RECENTLY_PLAYED_LIMIT)
            .map(|entry| entry.track.id.clone())
            .collect();

        let mut recommended_ids: HashSet<String> = HashSet::new();
        let mut recommendations: Vec<RecommendationItem> = Vec::new();

        // 1. Artist affinity
        let artist_counts = count_artists_in_history(history);
        let mut artists_by_plays: Vec<(&String, &usize)> = artist_counts.iter().collect();
        artists_by_plays.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));

        for (artist, _count) in artists_by_plays.iter().take(ARTIST_AFFINITY_LIMIT) {
            let artist_tracks: Vec<&LocalTrackEntry> = local_tracks
                .iter()
                .filter(|entry| entry.track.artist == **artist)
                .collect();
            let _unplayed_count = artist_tracks
                .iter()
                .filter(|entry| !recent_track_ids.contains(&entry.track.id))
                .count() as u32;
            let total_count = artist_tracks.len() as u32;
            if total_count == 0 {
                continue;
            }
            let id = make_artist_id(artist);
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
        let mut albums_by_plays: Vec<((String, String), usize)> = album_counts.into_iter().collect();
        albums_by_plays.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0 .0.cmp(&b.0 .0)));

        for ((artist, album), _count) in albums_by_plays.iter().take(ALBUM_AFFINITY_LIMIT) {
            let album_tracks: Vec<&LocalTrackEntry> = local_tracks
                .iter()
                .filter(|entry| {
                    entry.track.artist == *artist
                        && entry.track.album.as_ref() == Some(album)
                })
                .collect();
            if album_tracks.is_empty() {
                continue;
            }
            let id = make_album_id(artist, album);
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

        // 3. Favorite discovery
        let favorite_artists: HashSet<String> = favorites
            .iter()
            .map(|fav| fav.track.artist.clone())
            .collect();
        let mut favorite_tracks_added = 0;
        for entry in local_tracks.iter() {
            if favorite_tracks_added >= FAVORITE_DISCOVERY_LIMIT {
                break;
            }
            if !favorite_artists.contains(&entry.track.artist) {
                continue;
            }
            if recent_track_ids.contains(&entry.track.id) {
                continue;
            }
            if !recommended_ids.insert(entry.track.id.clone()) {
                continue;
            }
            recommendations.push(RecommendationItem::Track {
                track: entry.track.clone(),
                reason: "From your favorites".to_string(),
            });
            favorite_tracks_added += 1;
        }

        // 4. Library discovery
        let seed = daily_seed();
        let mut rng = StdRng::seed_from_u64(seed);
        let mut candidates: Vec<&LocalTrackEntry> = local_tracks
            .iter()
            .filter(|entry| !recent_track_ids.contains(&entry.track.id)
                && !recommended_ids.contains(&entry.track.id))
            .collect();
        candidates.shuffle(&mut rng);

        for entry in candidates.iter().take(LIBRARY_DISCOVERY_LIMIT) {
            if recommended_ids.insert(entry.track.id.clone()) {
                recommendations.push(RecommendationItem::Track {
                    track: entry.track.clone(),
                    reason: "Discover from your library".to_string(),
                });
            }
        }

        recommendations.truncate(RECOMMENDATIONS_LIMIT);
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

fn make_artist_id(name: &str) -> String {
    format!("artist-{}", name.replace(' ', "_"))
}

fn make_album_id(artist: &str, title: &str) -> String {
    format!(
        "album-{}-{}",
        artist.replace(' ', "_"),
        title.replace(' ', "_")
    )
}

fn daily_seed() -> u64 {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    now / SECONDS_PER_DAY
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
            metadata: HashMap::new(),
        }
    }

    fn insert_history_at(svc: &LibraryService, track: &Track, _offset_ms: u64) {
        svc.record_play(track).unwrap();
    }

    #[test]
    fn get_home_snapshot_recently_played_max_20_and_recency_order() {
        let svc = setup_service();
        // Seed 25 history entries, newest IDs last.
        for i in 0..25 {
            let track = sample_track(&format!("hist-{}", i));
            insert_history_at(&svc, &track, i * 10);
            std::thread::sleep(std::time::Duration::from_millis(2));
        }

        let snapshot = svc.get_home_snapshot().unwrap();
        assert_eq!(snapshot.recently_played.len(), 20, "Should cap at 20");
        assert_eq!(snapshot.recently_played[0].track.id, "hist-24", "Most recent first");
        assert_eq!(snapshot.recently_played[19].track.id, "hist-5", "Oldest in cap");
    }

    #[test]
    fn get_home_snapshot_recommends_artist_affinity() {
        let svc = setup_service();
        svc.db.insert_watched_folder("/music").unwrap();
        // Local tracks by "Affinity Artist"
        for i in 0..3 {
            let track = sample_local_track(&format!("local-{}", i), &format!("/music/{}.mp3", i), "Affinity Artist", Some("Album A"));
            svc.db.upsert_local_track(&format!("/music/{}.mp3", i), &track, "/music", Some(&format!("100{}", i))).unwrap();
        }
        // History: two plays of "Affinity Artist"
        let played = sample_local_track("hist-0", "/music/h0.mp3", "Affinity Artist", Some("Album A"));
        insert_history_at(&svc, &played, 0);
        std::thread::sleep(std::time::Duration::from_millis(5));
        insert_history_at(&svc, &played, 10);

        let snapshot = svc.get_home_snapshot().unwrap();
        let has_artist = snapshot.recommendations.iter().any(|item| match item {
            RecommendationItem::Artist { name, .. } if name == "Affinity Artist" => true,
            _ => false,
        });
        assert!(has_artist, "Should recommend artist affinity item");
    }

    #[test]
    fn get_home_snapshot_excludes_recently_played_when_alternatives_exist() {
        let svc = setup_service();
        svc.db.insert_watched_folder("/music").unwrap();
        // Two local tracks by same artist; one will be in history, one is alternative.
        let played = sample_local_track("track-played", "/music/played.mp3", "Same Artist", Some("Album X"));
        let alternative = sample_local_track("track-alt", "/music/alt.mp3", "Same Artist", Some("Album X"));
        svc.db.upsert_local_track("/music/played.mp3", &played, "/music", Some("1000")).unwrap();
        svc.db.upsert_local_track("/music/alt.mp3", &alternative, "/music", Some("1001")).unwrap();

        insert_history_at(&svc, &played, 0);

        let snapshot = svc.get_home_snapshot().unwrap();
        // The recommendations should not include the exact track that was recently played
        // as a library-discovery track when an alternative exists.
        let has_played_track = snapshot.recommendations.iter().any(|item| match item {
            RecommendationItem::Track { track, .. } if track.id == "track-played" => true,
            _ => false,
        });
        assert!(!has_played_track, "Should not recommend the exact recently played track");
    }

    #[test]
    fn get_home_snapshot_falls_back_to_library_discovery_with_empty_signals() {
        let svc = setup_service();
        svc.db.insert_watched_folder("/music").unwrap();
        for i in 0..5 {
            let track = sample_local_track(&format!("lib-{}", i), &format!("/music/{}.mp3", i), "Library Artist", None);
            svc.db.upsert_local_track(&format!("/music/{}.mp3", i), &track, "/music", Some(&format!("100{}", i))).unwrap();
        }

        let snapshot = svc.get_home_snapshot().unwrap();
        assert!(
            !snapshot.recommendations.is_empty(),
            "Empty history/favorites should still produce recommendations"
        );
        let has_library_track = snapshot.recommendations.iter().any(|item| matches!(item,
            RecommendationItem::Track { reason, .. } if reason.contains("library")));
        assert!(has_library_track, "Should include library discovery fallback items");
    }

    #[test]
    fn get_home_snapshot_recommendations_max_20() {
        let svc = setup_service();
        svc.db.insert_watched_folder("/music").unwrap();
        // Seed many local tracks
        for i in 0..60 {
            let track = sample_local_track(
                &format!("lib-{}", i),
                &format!("/music/{}.mp3", i),
                &format!("Artist {}", i % 5),
                Some("Album"),
            );
            svc.db.upsert_local_track(&format!("/music/{}.mp3", i), &track, "/music", Some(&format!("100{}", i))).unwrap();
        }
        // Add some history for affinity signals
        for i in 0..10 {
            let mut track = sample_track(&format!("hist-{}", i));
            track.artist = format!("Artist {}", i % 5);
            insert_history_at(&svc, &track, i * 5);
            std::thread::sleep(std::time::Duration::from_millis(2));
        }

        let snapshot = svc.get_home_snapshot().unwrap();
        assert!(
            snapshot.recommendations.len() <= 20,
            "Recommendations should never exceed 20 items, got {}",
            snapshot.recommendations.len()
        );
    }
}