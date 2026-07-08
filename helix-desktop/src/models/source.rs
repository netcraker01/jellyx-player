//! Source enum — identifies the origin of a track, artist, or album.
//!
//! Variants map to TypeScript `Source` enum values via PascalCase serialization.

use serde::{Deserialize, Serialize};

/// The origin of a track, artist, or album.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum Source {
    YouTube,
    SoundCloud,
    Local,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_youtube_roundtrip() {
        let source = Source::YouTube;
        let json = serde_json::to_string(&source).unwrap();
        assert_eq!(json, "\"YouTube\"");
        let deserialized: Source = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, Source::YouTube);
    }

    #[test]
    fn source_soundcloud_roundtrip() {
        let source = Source::SoundCloud;
        let json = serde_json::to_string(&source).unwrap();
        assert_eq!(json, "\"SoundCloud\"");
        let deserialized: Source = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, Source::SoundCloud);
    }

    #[test]
    fn source_local_roundtrip() {
        let source = Source::Local;
        let json = serde_json::to_string(&source).unwrap();
        assert_eq!(json, "\"Local\"");
        let deserialized: Source = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, Source::Local);
    }
}
