//! IPC DTOs — shared data transfer objects for Tauri commands.
//!
//! These structures are serialized as camelCase JSON for the Svelte frontend.
//! New DTOs for artist/album detail and grouped search live here so both
//! commands and services can depend on them without circular imports.

use serde::{Deserialize, Serialize};

use crate::models::track::Track;

/// Filter for grouped search: limit results to a single entity type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SearchFilter {
    Songs,
    Artists,
    Albums,
}

/// Grouped search result returned by `search_grouped`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupedSearchResult {
    pub songs: Vec<Track>,
    pub artists: Vec<ArtistSummary>,
    pub albums: Vec<AlbumSummary>,
}

/// Lightweight artist summary for search results.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtistSummary {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumbnail: Option<String>,
    pub track_count: u32,
}

/// Lightweight album summary for search results.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlbumSummary {
    pub id: String,
    pub title: String,
    pub artist: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub year: Option<u32>,
    pub track_count: u32,
}

/// Full artist detail for `/artist/:id` view.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtistDetail {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumbnail: Option<String>,
    pub top_tracks: Vec<Track>,
    pub albums: Vec<AlbumSummary>,
}

/// Full album detail for `/album/:id` view.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlbumDetail {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub artist_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub year: Option<u32>,
    pub tracks: Vec<Track>,
}

// ── ID normalization helpers ─────────────────────────────────────────

/// Normalize a raw artist name into a stable artist ID.
///
/// Format: `artist:{lowercase-trimmed-dashes}`
/// Spaces and consecutive whitespace become single hyphens.
pub fn normalize_artist_id(name: &str) -> String {
    let normalized = name
        .trim()
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("-");
    format!("artist:{}", normalized)
}

/// Normalize album title and artist into a stable album ID.
///
/// Format: `album:{lowercase-title}:{lowercase-artist}`
/// Spaces are collapsed to hyphens per design AD-3.
pub fn normalize_album_id(title: &str, artist: &str) -> String {
    let norm_title = title
        .trim()
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("-");
    let norm_artist = artist
        .trim()
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("-");
    format!("album:{}:{}", norm_title, norm_artist)
}

/// Extract the original artist name from an artist ID.
///
/// Returns `None` if the ID does not start with `artist:`.
pub fn denormalize_artist_id(id: &str) -> Option<String> {
    id.strip_prefix("artist:")
        .map(|name| name.split('-').collect::<Vec<_>>().join(" ").to_lowercase())
}

/// Extract the original album title and artist name from an album ID.
///
/// Returns `None` if the ID is not in `album:{title}:{artist}` format.
pub fn denormalize_album_id(id: &str) -> Option<(String, String)> {
    let rest = id.strip_prefix("album:")?;
    let mut parts = rest.splitn(2, ':');
    let title = parts
        .next()?
        .split('-')
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase();
    let artist = parts
        .next()?
        .split('-')
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase();
    Some((title, artist))
}

#[cfg(test)]
mod tests {
    use super::*;

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
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"songs\""));
        assert!(json.contains("\"artists\""));
        assert!(json.contains("\"albums\""));
        assert!(json.contains("\"trackCount\""));
        assert!(!json.contains("\"thumbnail\""), "None thumbnail should be skipped");
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
}
