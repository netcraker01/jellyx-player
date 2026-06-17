//! Persistence models — entry types for favorites and history.

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