//! Playback state — Source of Truth for playback status and queue.
//!
//! `PlaybackState` represents the current playback status.
//! `QueueState` holds the current queue and active track index.

use crate::models::track::Track;
use serde::{Deserialize, Serialize};

/// Current playback state.
///
/// Serialized as PascalCase to match the TypeScript frontend enum.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum PlaybackState {
    Stopped,
    Playing,
    Paused,
    Buffering,
}

/// State of the playback queue.
///
/// Holds the list of tracks and the index of the currently playing track.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueueState {
    pub tracks: Vec<Track>,
    pub current_index: Option<usize>,
}

impl Default for QueueState {
    fn default() -> Self {
        Self {
            tracks: Vec::new(),
            current_index: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::source::Source;

    #[test]
    fn playback_state_playing_serializes_to_pascal_case() {
        let json = serde_json::to_string(&PlaybackState::Playing).unwrap();
        assert_eq!(json, "\"Playing\"");
    }

    #[test]
    fn playback_state_stopped_serializes_to_pascal_case() {
        let json = serde_json::to_string(&PlaybackState::Stopped).unwrap();
        assert_eq!(json, "\"Stopped\"");
    }

    #[test]
    fn playback_state_paused_serializes_to_pascal_case() {
        let json = serde_json::to_string(&PlaybackState::Paused).unwrap();
        assert_eq!(json, "\"Paused\"");
    }

    #[test]
    fn playback_state_buffering_serializes_to_pascal_case() {
        let json = serde_json::to_string(&PlaybackState::Buffering).unwrap();
        assert_eq!(json, "\"Buffering\"");
    }

    #[test]
    fn queue_state_default_is_empty() {
        let qs = QueueState::default();
        assert!(qs.tracks.is_empty());
        assert!(qs.current_index.is_none());
    }

    #[test]
    fn queue_state_camel_case_serialization() {
        let track = Track {
            id: "t1".to_string(),
            source: Source::YouTube,
            source_id: "yt-1".to_string(),
            title: "Song".to_string(),
            artist: "Artist".to_string(),
            album: None,
            duration: None,
            thumbnail: None,
            stream_url: None,
            local_path: None,
            metadata: std::collections::HashMap::new(),
        };
        let qs = QueueState {
            tracks: vec![track],
            current_index: Some(0),
        };
        let json = serde_json::to_string(&qs).unwrap();
        assert!(json.contains("\"currentIndex\""), "current_index should serialize as currentIndex");
        assert!(json.contains("\"tracks\""), "tracks should serialize as tracks");
    }

    #[test]
    fn queue_state_deserialize_from_camel_case() {
        let json = r#"{"tracks":[],"currentIndex":null}"#;
        let qs: QueueState = serde_json::from_str(json).unwrap();
        assert!(qs.tracks.is_empty());
        assert!(qs.current_index.is_none());
    }
}