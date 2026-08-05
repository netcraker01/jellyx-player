//! Shared data transfer objects for library, search, and home views.
//!
//! These types are used by both Tauri and Ratatui frontends. They depend on
//! `jellyx-core` for the `Track` model but contain no platform-specific code.

use jellyx_core::models::track::Track;
use serde::{Deserialize, Serialize};

use crate::history::HistoryRow;

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
    #[serde(default)]
    pub has_more_songs: bool,
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

/// Full artist detail for the artist view.
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

/// Full album detail for the album view.
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
    pub recently_played: Vec<HistoryRow>,
    pub recommendations: Vec<RecommendationItem>,
}

// ── ID normalization helpers ─────────────────────────────────────────

/// Normalize a raw artist name into a stable artist ID.
pub fn normalize_artist_id(name: &str) -> String {
    normalize_artist_id_with_source(name, None)
}

/// Normalize a raw artist name into a stable artist ID, optionally tagged
/// with a source dimension.
pub fn normalize_artist_id_with_source(name: &str, source: Option<&str>) -> String {
    let normalized = name
        .trim()
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("-");
    match source {
        Some(src) if !src.is_empty() => {
            format!("artist:{}:{}", normalized, src.to_lowercase())
        }
        _ => format!("artist:{}", normalized),
    }
}

/// Extract the artist name from an artist ID.
pub fn denormalize_artist_id(id: &str) -> Option<String> {
    let rest = id.strip_prefix("artist:")?;
    match rest.rsplit_once(':') {
        Some((name, src)) if !src.contains('-') || src.is_empty() => {
            Some(name.split('-').collect::<Vec<_>>().join(" ").to_lowercase())
        }
        _ => Some(rest.split('-').collect::<Vec<_>>().join(" ").to_lowercase()),
    }
}

/// Extract the source dimension from an artist ID, if present.
pub fn artist_id_source(id: &str) -> Option<String> {
    let rest = id.strip_prefix("artist:")?;
    let (_, src) = rest.rsplit_once(':')?;
    if src.is_empty() {
        None
    } else {
        Some(src.to_string())
    }
}

/// Normalize album title and artist into a stable album ID.
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

/// Extract the original album title and artist name from an album ID.
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
    fn artist_id_roundtrip() {
        let id = normalize_artist_id("Daft Punk");
        assert_eq!(id, "artist:daft-punk");
        assert_eq!(denormalize_artist_id(&id).unwrap(), "daft punk");
    }

    #[test]
    fn artist_id_with_source() {
        let id = normalize_artist_id_with_source("Daft Punk", Some("local"));
        assert_eq!(id, "artist:daft-punk:local");
        assert_eq!(artist_id_source(&id).unwrap(), "local");
        assert_eq!(denormalize_artist_id(&id).unwrap(), "daft punk");
    }

    #[test]
    fn album_id_roundtrip() {
        let id = normalize_album_id("Discovery", "Daft Punk");
        assert_eq!(id, "album:discovery:daft-punk");
        let (title, artist) = denormalize_album_id(&id).unwrap();
        assert_eq!(title, "discovery");
        assert_eq!(artist, "daft punk");
    }

    #[test]
    fn denormalize_returns_none_for_invalid_prefix() {
        assert_eq!(denormalize_artist_id("not-an-artist-id"), None);
        assert_eq!(denormalize_album_id("not-an-album-id"), None);
    }
}
