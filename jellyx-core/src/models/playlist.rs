//! Playlist model — represents a collection of tracks from a remote source.
//!
//! Matches the design: `Playlist` struct with id, source, source_id, title,
//! thumbnail, track_count, and tracks. Supports full serde roundtrip with
//! camelCase serialization for the TypeScript frontend.

use serde::{Deserialize, Serialize};

use crate::models::source::Source;
use crate::models::track::Track;

/// A playlist of tracks from a remote source (YouTube, SoundCloud, etc.).
///
/// Playlists are resolved via `SourceResolver::search_playlists()` and
/// `SourceResolver::resolve_playlist()`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Playlist {
    pub id: String,
    pub source: Source,
    pub source_id: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumbnail: Option<String>,
    pub track_count: usize,
    pub tracks: Vec<Track>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn sample_track() -> Track {
        Track {
            id: "track-1".to_string(),
            source: Source::YouTube,
            source_id: "yt-abc123".to_string(),
            title: "Test Song".to_string(),
            artist: "Test Artist".to_string(),
            album: None,
            duration: Some(240.0),
            thumbnail: None,
            stream_url: None,
            local_path: None,
            metadata: HashMap::new(),
            playlist_id: None,
        }
    }

    #[test]
    fn playlist_roundtrip_all_fields() {
        let track = sample_track();
        let playlist = Playlist {
            id: "pl-1".to_string(),
            source: Source::YouTube,
            source_id: "yt-playlist-123".to_string(),
            title: "My Playlist".to_string(),
            thumbnail: Some("https://img.test/pl.jpg".to_string()),
            track_count: 1,
            tracks: vec![track],
        };

        let json = serde_json::to_string(&playlist).unwrap();
        let deserialized: Playlist = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.id, "pl-1");
        assert_eq!(deserialized.source, Source::YouTube);
        assert_eq!(deserialized.source_id, "yt-playlist-123");
        assert_eq!(deserialized.title, "My Playlist");
        assert_eq!(deserialized.thumbnail, Some("https://img.test/pl.jpg".to_string()));
        assert_eq!(deserialized.track_count, 1);
        assert_eq!(deserialized.tracks.len(), 1);
        assert_eq!(deserialized.tracks[0].id, "track-1");
    }

    #[test]
    fn playlist_camel_case_field_names() {
        let playlist = Playlist {
            id: "pl-2".to_string(),
            source: Source::YouTube,
            source_id: "yt-pl-456".to_string(),
            title: "Test".to_string(),
            thumbnail: None,
            track_count: 0,
            tracks: vec![],
        };
        let json = serde_json::to_string(&playlist).unwrap();
        assert!(json.contains("\"sourceId\""), "source_id should be camelCase");
        assert!(json.contains("\"trackCount\""), "track_count should be camelCase");
        assert!(
            !json.contains("\"thumbnail\""),
            "None thumbnail should be absent from JSON"
        );
    }

    #[test]
    fn playlist_deserialize_from_camel_case_json() {
        let json = r#"{"id":"pl-3","source":"YouTube","sourceId":"yt-789","title":"Cool Mix","trackCount":2,"tracks":[]}"#;
        let playlist: Playlist = serde_json::from_str(json).unwrap();
        assert_eq!(playlist.id, "pl-3");
        assert_eq!(playlist.source, Source::YouTube);
        assert_eq!(playlist.source_id, "yt-789");
        assert_eq!(playlist.title, "Cool Mix");
        assert_eq!(playlist.track_count, 2);
        assert!(playlist.thumbnail.is_none());
    }
}