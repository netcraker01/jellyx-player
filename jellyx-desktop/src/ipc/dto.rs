//! IPC DTOs — shared data transfer objects for Tauri commands.
//!
//! Shared DTOs (search, artist/album detail, recommendations, ID normalization)
//! live in `jellyx-engine::dto` so both frontends use the same definitions.
//! Desktop-specific DTOs (UserPlaylist, PlaylistTrackEntry, ArtistFavorite)
//! remain here because they mirror the persistence models with IPC serde
//! defaults.

use serde::{Deserialize, Serialize};

use crate::persistence::models::HistoryEntry;
use crate::updater::checker::UpdateInfo;
use crate::updater::prefs::UpdatePrefs;
use jellyx_core::models::track::Track;

// Re-export shared DTOs from the engine
pub use jellyx_engine::dto::{
    artist_id_source, denormalize_album_id, denormalize_artist_id, normalize_album_id,
    normalize_artist_id, normalize_artist_id_with_source, AlbumDetail, AlbumSummary, ArtistDetail,
    ArtistSummary, GroupedSearchResult, HomeSnapshot as EngineHomeSnapshot, RecommendationItem,
    SearchFilter,
};

/// A user-created local playlist (DTO for IPC).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserPlaylist {
    pub id: String,
    pub title: String,
    #[serde(default = "default_playlist_kind_dto")]
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_folder_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_playlist_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

fn default_playlist_kind_dto() -> String {
    "manual".to_string()
}

/// A track entry inside a user playlist (DTO for IPC).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistTrackEntry {
    pub playlist_id: String,
    pub position: i64,
    pub track: Track,
    pub added_at: String,
}

/// A favorited artist (DTO for IPC).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtistFavorite {
    pub artist_id: String,
    #[serde(default = "default_favorite_source_dto")]
    pub source: String,
    pub artist_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumbnail: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_artist_ref: Option<String>,
    pub added_at: String,
}

fn default_favorite_source_dto() -> String {
    "local".to_string()
}

/// Home snapshot returned by `get_home_snapshot`.
///
/// Uses the engine's `HomeSnapshot` internally but wraps it with the
/// desktop's `HistoryEntry` type for IPC compatibility.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HomeSnapshot {
    pub recently_played: Vec<HistoryEntry>,
    pub recommendations: Vec<RecommendationItem>,
}

// ── Updater DTOs ─────────────────────────────────────────────────────
pub type UpdaterInfo = UpdateInfo;
pub type UpdaterPrefs = UpdatePrefs;

#[cfg(test)]
mod tests {
    use super::*;
    use jellyx_core::models::source::Source;
    use std::collections::HashMap;

    fn sample_track(id: &str) -> Track {
        Track {
            id: id.to_string(),
            source: Source::Local,
            source_id: format!("local-{}", id),
            title: format!("Song {}", id),
            artist: "Test Artist".to_string(),
            album: None,
            duration: Some(180.0),
            thumbnail: None,
            stream_url: None,
            local_path: None,
            playlist_id: None,
            metadata: HashMap::new(),
        }
    }

    #[test]
    fn normalize_artist_id_lowercase_and_dashes() {
        assert_eq!(normalize_artist_id("Daft Punk"), "artist:daft-punk");
        assert_eq!(normalize_artist_id("  Queen  "), "artist:queen");
        assert_eq!(
            normalize_artist_id("AC/DC"),
            "artist:ac/dc" // slashes are kept, only spaces → dashes
        );
    }

    #[test]
    fn normalize_album_id_title_and_artist() {
        assert_eq!(
            normalize_album_id("Discovery", "Daft Punk"),
            "album:discovery:daft-punk"
        );
        assert_eq!(
            normalize_album_id("  The Wall  ", "Pink Floyd"),
            "album:the-wall:pink-floyd"
        );
    }

    #[test]
    fn denormalize_artist_id_reverses_normalization() {
        assert_eq!(
            denormalize_artist_id("artist:daft-punk"),
            Some("daft punk".to_string())
        );
        assert_eq!(
            denormalize_artist_id("artist:ac/dc"),
            Some("ac/dc".to_string())
        );
        assert!(denormalize_artist_id("album:discovery:daft-punk").is_none());
    }

    #[test]
    fn denormalize_album_id_reverses_normalization() {
        assert_eq!(
            denormalize_album_id("album:discovery:daft-punk"),
            Some(("discovery".to_string(), "daft punk".to_string()))
        );
        assert_eq!(
            denormalize_album_id("album:the-wall:pink-floyd"),
            Some(("the wall".to_string(), "pink floyd".to_string()))
        );
        assert!(denormalize_album_id("artist:daft-punk").is_none());
    }

    #[test]
    fn roundtrip_artist_id() {
        let original = "Daft Punk";
        let id = normalize_artist_id(original);
        let back = denormalize_artist_id(&id).unwrap();
        assert_eq!(back, original.to_lowercase());
    }

    #[test]
    fn roundtrip_album_id() {
        let title = "Discovery";
        let artist = "Daft Punk";
        let id = normalize_album_id(title, artist);
        let (back_title, back_artist) = denormalize_album_id(&id).unwrap();
        assert_eq!(back_title, title.to_lowercase());
        assert_eq!(back_artist, artist.to_lowercase());
    }

    #[test]
    fn search_filter_serializes_camel_case() {
        let json = serde_json::to_string(&SearchFilter::Artists).unwrap();
        assert_eq!(json, "\"artists\"");
    }

    #[test]
    fn grouped_search_result_camel_case_serialization() {
        let result = GroupedSearchResult {
            songs: vec![],
            artists: vec![ArtistSummary {
                id: "artist:queen".into(),
                name: "Queen".into(),
                thumbnail: None,
                track_count: 10,
            }],
            albums: vec![],
            has_more_songs: false,
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"songs\""));
        assert!(json.contains("\"artists\""));
        assert!(json.contains("\"albums\""));
        assert!(json.contains("\"trackCount\""));
        assert!(
            !json.contains("\"thumbnail\""),
            "None thumbnail should be skipped"
        );
    }

    #[test]
    fn artist_detail_camel_case_serialization() {
        let detail = ArtistDetail {
            id: "artist:queen".into(),
            name: "Queen".into(),
            thumbnail: Some("https://img.test/queen.jpg".into()),
            top_tracks: vec![],
            albums: vec![],
        };
        let json = serde_json::to_string(&detail).unwrap();
        assert!(json.contains("\"topTracks\""));
        assert!(json.contains("\"thumbnail\""));
    }

    #[test]
    fn album_detail_camel_case_serialization() {
        let detail = AlbumDetail {
            id: "album:discovery:daft-punk".into(),
            title: "Discovery".into(),
            artist: "Daft Punk".into(),
            artist_id: "artist:daft-punk".into(),
            cover: None,
            year: Some(2001),
            tracks: vec![],
        };
        let json = serde_json::to_string(&detail).unwrap();
        assert!(json.contains("\"artistId\""));
        assert!(json.contains("\"year\""));
        assert!(!json.contains("\"cover\""), "None cover should be skipped");
    }

    #[test]
    fn recommendation_item_track_serializes_with_camel_case_fields() {
        let item = RecommendationItem::Track {
            track: sample_track("r1"),
            reason: "From your favorites".to_string(),
        };
        let json = serde_json::to_string(&item).unwrap();
        assert!(json.contains("\"type\":\"Track\""), "type should be Track");
        assert!(
            json.contains("\"reason\":\"From your favorites\""),
            "reason should be present"
        );
    }

    #[test]
    fn recommendation_item_artist_serializes_with_camel_case_fields() {
        let item = RecommendationItem::Artist {
            id: "artist-1".to_string(),
            name: "Test Artist".to_string(),
            thumbnail: Some("https://img.test/artist.jpg".to_string()),
            track_count: 5,
            reason: "Because you listened to Test Artist".to_string(),
        };
        let json = serde_json::to_string(&item).unwrap();
        assert!(
            json.contains("\"type\":\"Artist\""),
            "type should be Artist"
        );
        assert!(
            json.contains("\"trackCount\":5"),
            "track_count should be camelCase"
        );
        assert!(
            !json.contains("\"track_count\""),
            "snake_case track_count should not appear"
        );
        assert!(
            json.contains("\"reason\":\"Because you listened to Test Artist\""),
            "reason should be present"
        );
    }

    #[test]
    fn recommendation_item_album_serializes_with_camel_case_fields() {
        let item = RecommendationItem::Album {
            id: "album-1".to_string(),
            title: "Test Album".to_string(),
            artist: "Test Artist".to_string(),
            cover: Some("https://img.test/cover.jpg".to_string()),
            track_count: 10,
            reason: "Based on your listening".to_string(),
        };
        let json = serde_json::to_string(&item).unwrap();
        assert!(json.contains("\"type\":\"Album\""), "type should be Album");
        assert!(json.contains("\"id\":\"album-1\""), "id should be present");
        assert!(
            json.contains("\"title\":\"Test Album\""),
            "title should be present"
        );
        assert!(
            json.contains("\"artist\":\"Test Artist\""),
            "artist should be present"
        );
        assert!(
            json.contains("\"cover\":\"https://img.test/cover.jpg\""),
            "cover should be present"
        );
        assert!(
            json.contains("\"trackCount\":10"),
            "track_count should be camelCase"
        );
        assert!(
            !json.contains("\"track_count\""),
            "snake_case track_count should not appear"
        );
        assert!(
            json.contains("\"reason\":\"Based on your listening\""),
            "reason should be present"
        );
    }

    #[test]
    fn recommendation_item_none_fields_are_skipped() {
        let artist = RecommendationItem::Artist {
            id: "artist-2".to_string(),
            name: "No Thumbnail Artist".to_string(),
            thumbnail: None,
            track_count: 3,
            reason: "Discover from your library".to_string(),
        };
        let album = RecommendationItem::Album {
            id: "album-2".to_string(),
            title: "No Cover Album".to_string(),
            artist: "No Thumbnail Artist".to_string(),
            cover: None,
            track_count: 5,
            reason: "Discover from your library".to_string(),
        };
        let artist_json = serde_json::to_string(&artist).unwrap();
        let album_json = serde_json::to_string(&album).unwrap();
        assert!(
            !artist_json.contains("\"thumbnail\""),
            "None thumbnail should be skipped for artist"
        );
        assert!(
            !album_json.contains("\"cover\""),
            "None cover should be skipped for album"
        );
    }

    #[test]
    fn home_snapshot_serializes_recently_played_and_recommendations() {
        let entry = HistoryEntry {
            id: 1,
            track: sample_track("t1"),
            played_at: "2026-01-01 10:00:00".to_string(),
        };
        let item = RecommendationItem::Track {
            track: sample_track("t2"),
            reason: "From your favorites".to_string(),
        };
        let snapshot = HomeSnapshot {
            recently_played: vec![entry],
            recommendations: vec![item],
        };
        let json = serde_json::to_string(&snapshot).unwrap();
        assert!(
            json.contains("\"recentlyPlayed\""),
            "recently_played should be camelCase"
        );
        assert!(
            json.contains("\"recommendations\""),
            "recommendations should be present"
        );
        assert!(
            !json.contains("\"recently_played\""),
            "snake_case recently_played should not appear"
        );
    }

    #[test]
    fn home_snapshot_empty_sections_serialize_correctly() {
        let snapshot = HomeSnapshot {
            recently_played: vec![],
            recommendations: vec![],
        };
        let json = serde_json::to_string(&snapshot).unwrap();
        assert!(
            json.contains("\"recentlyPlayed\":[]"),
            "empty recentlyPlayed should serialize as []"
        );
        assert!(
            json.contains("\"recommendations\":[]"),
            "empty recommendations should serialize as []"
        );
    }
}
