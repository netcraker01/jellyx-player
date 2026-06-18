//! IPC DTOs — data transfer objects shared between backend and frontend.
//!
//! These types are intentionally shaped for serialization to the Svelte UI:
//! camelCase field names, tagged unions via `type`, and skipped `None` fields.

use serde::{Deserialize, Serialize};

use crate::models::track::Track;
use crate::persistence::models::HistoryEntry;

/// A single recommendation item, which may be a track, artist, or album.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(rename_all_fields = "camelCase")]
#[serde(tag = "type")]
pub enum RecommendationItem {
    #[serde(rename = "Track")]
    Track { track: Track, reason: String },
    #[serde(rename = "Artist")]
    Artist {
        id: String,
        name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        thumbnail: Option<String>,
        track_count: u32,
        reason: String,
    },
    #[serde(rename = "Album")]
    Album {
        id: String,
        title: String,
        artist: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        cover: Option<String>,
        track_count: u32,
        reason: String,
    },
}

/// Home snapshot returned by `get_home_snapshot`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HomeSnapshot {
    pub recently_played: Vec<HistoryEntry>,
    pub recommendations: Vec<RecommendationItem>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::source::Source;
    use std::collections::HashMap;

    fn sample_track(id: &str) -> Track {
        Track {
            id: id.to_string(),
            source: Source::Local,
            source_id: format!("local-{}", id),
            title: format!("Song {}", id),
            artist: "Test Artist".to_string(),
            album: Some("Test Album".to_string()),
            duration: Some(180.0),
            thumbnail: Some("https://img.test/thumb.jpg".to_string()),
            stream_url: None,
            local_path: Some(format!("/music/{}.mp3", id)),
            metadata: HashMap::new(),
        }
    }

    #[test]
    fn recommendation_item_track_serializes_with_type_and_reason() {
        let item = RecommendationItem::Track {
            track: sample_track("t1"),
            reason: "Because you listened to Test Artist".to_string(),
        };
        let json = serde_json::to_string(&item).unwrap();
        assert!(json.contains("\"type\":\"Track\""), "type should be Track");
        assert!(json.contains("\"reason\":\"Because you listened to Test Artist\""), "reason should be present");
        assert!(json.contains("\"track\":"), "track object should be present");
    }

    #[test]
    fn recommendation_item_artist_serializes_with_camel_case_fields() {
        let item = RecommendationItem::Artist {
            id: "artist-1".to_string(),
            name: "Test Artist".to_string(),
            thumbnail: Some("https://img.test/artist.jpg".to_string()),
            track_count: 7,
            reason: "Because you listened to Test Artist".to_string(),
        };
        let json = serde_json::to_string(&item).unwrap();
        assert!(json.contains("\"type\":\"Artist\""), "type should be Artist");
        assert!(json.contains("\"id\":\"artist-1\""), "id should be present");
        assert!(json.contains("\"name\":\"Test Artist\""), "name should be present");
        assert!(json.contains("\"thumbnail\":\"https://img.test/artist.jpg\""), "thumbnail should be present");
        assert!(json.contains("\"trackCount\":7"), "track_count should be camelCase");
        assert!(!json.contains("\"track_count\""), "snake_case track_count should not appear");
        assert!(json.contains("\"reason\":\"Because you listened to Test Artist\""), "reason should be present");
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
        assert!(json.contains("\"title\":\"Test Album\""), "title should be present");
        assert!(json.contains("\"artist\":\"Test Artist\""), "artist should be present");
        assert!(json.contains("\"cover\":\"https://img.test/cover.jpg\""), "cover should be present");
        assert!(json.contains("\"trackCount\":10"), "track_count should be camelCase");
        assert!(!json.contains("\"track_count\""), "snake_case track_count should not appear");
        assert!(json.contains("\"reason\":\"Based on your listening\""), "reason should be present");
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
        assert!(!artist_json.contains("\"thumbnail\""), "None thumbnail should be skipped for artist");
        assert!(!album_json.contains("\"cover\""), "None cover should be skipped for album");
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
        assert!(json.contains("\"recentlyPlayed\""), "recently_played should be camelCase");
        assert!(json.contains("\"recommendations\""), "recommendations should be present");
        assert!(!json.contains("\"recently_played\""), "snake_case recently_played should not appear");
    }

    #[test]
    fn home_snapshot_empty_sections_serialize_correctly() {
        let snapshot = HomeSnapshot {
            recently_played: vec![],
            recommendations: vec![],
        };
        let json = serde_json::to_string(&snapshot).unwrap();
        assert!(json.contains("\"recentlyPlayed\":[]"), "empty recentlyPlayed should serialize as []");
        assert!(json.contains("\"recommendations\":[]"), "empty recommendations should serialize as []");
    }
}
