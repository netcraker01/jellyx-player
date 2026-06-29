//! Persistence models — entry types for history, local scanner, playlists, artist favorites, and source settings.

use serde::{Deserialize, Serialize};

use crate::models::track::Track;
/// A play history entry with timestamp.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub id: i64,
    pub track: Track,
    pub played_at: String,
}

/// A watched folder entry for the local file scanner.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WatchedFolder {
    pub path: String,
    pub last_scanned_at: Option<String>,
    pub added_at: String,
}

/// A local track entry from the file scanner.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalTrackEntry {
    pub track: Track,
    pub file_path: String,
    pub folder_path: String,
    pub file_modified_at: Option<String>,
}

/// A user-created local playlist.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserPlaylist {
    pub id: String,
    pub title: String,
    pub created_at: String,
    pub updated_at: String,
}

/// A track entry inside a user playlist.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistTrackEntry {
    pub playlist_id: String,
    pub position: i64,
    pub track: Track,
    pub added_at: String,
}

/// A favorited artist.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtistFavorite {
    pub artist_id: String,
    pub artist_name: String,
    pub thumbnail: Option<String>,
    pub added_at: String,
}

/// A source plugin setting (enabled/disabled).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceSetting {
    pub source: String,
    pub enabled: bool,
    pub label: String,
}

/// Audio settings returned to the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioSettings {
    pub normalize_audio: bool,
}
