//! Persistence models — entry types for favorites, history, and local scanner.

use serde::{Deserialize, Serialize};

use crate::models::track::Track;

/// A favorited track with metadata about when it was added.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FavoriteEntry {
    pub track: Track,
    pub added_at: String,
}

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