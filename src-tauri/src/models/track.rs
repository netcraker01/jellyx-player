//! Track model — the unified rich track struct.
//!
//! Matches ARCHITECTURE.md §4.1: the canonical Track model
//! shared by playback, library, search, and UI bridge.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::models::source::Source;

/// Result from a source search or library lookup.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Track {
    pub id: String,
    pub source: Source,
    pub source_id: String,
    pub title: String,
    pub artist: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub album: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumbnail: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub local_path: Option<String>,
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_track() -> Track {
        let mut metadata = HashMap::new();
        metadata.insert("quality".to_string(), "hd".to_string());
        Track {
            id: "track-1".to_string(),
            source: Source::YouTube,
            source_id: "yt-abc123".to_string(),
            title: "Test Song".to_string(),
            artist: "Test Artist".to_string(),
            album: Some("Test Album".to_string()),
            duration: Some(240.0),
            thumbnail: Some("https://img.test/thumb.jpg".to_string()),
            stream_url: Some("https://stream.test/audio.mp3".to_string()),
            local_path: None,
            metadata,
        }
    }

    #[test]
    fn track_roundtrip_all_fields() {
        let track = sample_track();
        let json = serde_json::to_string(&track).unwrap();
        let deserialized: Track = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, "track-1");
        assert_eq!(deserialized.source, Source::YouTube);
        assert_eq!(deserialized.source_id, "yt-abc123");
        assert_eq!(deserialized.album, Some("Test Album".to_string()));
        assert_eq!(deserialized.duration, Some(240.0));
        assert_eq!(deserialized.metadata.get("quality").unwrap(), "hd");
    }

    #[test]
    fn track_camel_case_field_names() {
        let track = Track {
            id: "track-1".to_string(),
            source: Source::YouTube,
            source_id: "yt-abc123".to_string(),
            title: "Test Song".to_string(),
            artist: "Test Artist".to_string(),
            album: Some("Test Album".to_string()),
            duration: Some(240.0),
            thumbnail: Some("https://img.test/thumb.jpg".to_string()),
            stream_url: Some("https://stream.test/audio.mp3".to_string()),
            local_path: Some("/music/track.mp3".to_string()),
            metadata: HashMap::new(),
        };
        let json = serde_json::to_string(&track).unwrap();
        // Verify camelCase serialization — all fields present
        assert!(
            json.contains("\"sourceId\""),
            "source_id should be camelCase"
        );
        assert!(
            json.contains("\"streamUrl\""),
            "stream_url should be camelCase"
        );
        assert!(
            json.contains("\"localPath\""),
            "local_path should be camelCase"
        );
    }

    #[test]
    fn track_none_fields_absent_from_json() {
        let track = Track {
            id: "track-2".to_string(),
            source: Source::SoundCloud,
            source_id: "sc-xyz".to_string(),
            title: "Minimal".to_string(),
            artist: "Artist".to_string(),
            album: None,
            duration: None,
            thumbnail: None,
            stream_url: None,
            local_path: None,
            metadata: HashMap::new(),
        };
        let json = serde_json::to_string(&track).unwrap();
        assert!(!json.contains("\"album\""), "None album should be absent");
        assert!(
            !json.contains("\"duration\""),
            "None duration should be absent"
        );
        assert!(
            !json.contains("\"thumbnail\""),
            "None thumbnail should be absent"
        );
        assert!(
            !json.contains("\"streamUrl\""),
            "None stream_url should be absent"
        );
        assert!(
            !json.contains("\"localPath\""),
            "None local_path should be absent"
        );
    }

    #[test]
    fn track_deserialize_from_camel_case_json() {
        let json = r#"{"id":"t3","source":"SoundCloud","sourceId":"sc-789","title":"Song","artist":"Art","duration":180.5}"#;
        let track: Track = serde_json::from_str(json).unwrap();
        assert_eq!(track.id, "t3");
        assert_eq!(track.source, Source::SoundCloud);
        assert_eq!(track.source_id, "sc-789");
        assert_eq!(track.duration, Some(180.5));
        assert!(track.album.is_none());
    }
}
