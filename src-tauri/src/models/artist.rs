//! Artist model — represents a music artist.

use serde::{Deserialize, Serialize};

use crate::models::source::Source;

/// A music artist from a source.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Artist {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumbnail: Option<String>,
    pub source: Source,
    pub source_id: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn artist_roundtrip() {
        let artist = Artist {
            id: "artist-1".to_string(),
            name: "Test Artist".to_string(),
            thumbnail: Some("https://img.test/artist.jpg".to_string()),
            source: Source::YouTube,
            source_id: "yt-artist-1".to_string(),
        };
        let json = serde_json::to_string(&artist).unwrap();
        let deserialized: Artist = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, "artist-1");
        assert_eq!(deserialized.source, Source::YouTube);
        assert_eq!(deserialized.thumbnail, Some("https://img.test/artist.jpg".to_string()));
    }

    #[test]
    fn artist_camel_case_fields() {
        let artist = Artist {
            id: "a1".to_string(),
            name: "Art".to_string(),
            thumbnail: None,
            source: Source::Local,
            source_id: "local-1".to_string(),
        };
        let json = serde_json::to_string(&artist).unwrap();
        assert!(json.contains("\"sourceId\""), "source_id should be camelCase");
        assert!(!json.contains("\"thumbnail\""), "None thumbnail should be absent");
    }
}