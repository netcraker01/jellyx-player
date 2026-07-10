//! Album model — represents a music album.

use serde::{Deserialize, Serialize};

use crate::models::source::Source;

/// A music album from a source.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct Album {
    pub id: String,
    pub title: String,
    pub artist: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub year: Option<u32>,
    pub source: Source,
    pub source_id: String,
    pub tracks: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn album_roundtrip() {
        let album = Album {
            id: "album-1".to_string(),
            title: "Test Album".to_string(),
            artist: "Test Artist".to_string(),
            cover: Some("https://img.test/cover.jpg".to_string()),
            year: Some(2023),
            source: Source::SoundCloud,
            source_id: "sc-album-1".to_string(),
            tracks: vec!["track-1".to_string(), "track-2".to_string()],
        };
        let json = serde_json::to_string(&album).unwrap();
        let deserialized: Album = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, "album-1");
        assert_eq!(deserialized.source, Source::SoundCloud);
        assert_eq!(deserialized.year, Some(2023));
        assert_eq!(deserialized.tracks, vec!["track-1", "track-2"]);
    }

    #[test]
    fn album_camel_case_fields() {
        let album = Album {
            id: "a1".to_string(),
            title: "Al".to_string(),
            artist: "Art".to_string(),
            cover: None,
            year: None,
            source: Source::Local,
            source_id: "local-1".to_string(),
            tracks: vec![],
        };
        let json = serde_json::to_string(&album).unwrap();
        assert!(
            json.contains("\"sourceId\""),
            "source_id should be camelCase"
        );
        assert!(!json.contains("\"cover\""), "None cover should be absent");
        assert!(!json.contains("\"year\""), "None year should be absent");
    }
}
