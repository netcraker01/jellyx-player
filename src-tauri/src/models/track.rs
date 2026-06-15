//! Track model — the unified rich track struct.
//!
//! This is the canonical location for `Track`.
//! Future changes will enrich it with `Source` enum, `album`, `metadata`, etc.

use serde::Serialize;

/// Result from a source search.
#[derive(Debug, Clone, Serialize)]
pub struct Track {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub duration: f64,
    pub thumbnail: String,
    pub stream_url: String,
    pub source: String,
}