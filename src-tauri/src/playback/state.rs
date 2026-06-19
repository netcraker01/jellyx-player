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

/// Repeat mode for queue playback.
///
/// Serialized as PascalCase to match the TypeScript frontend enum.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum RepeatMode {
    Off,
    All,
    One,
}

impl Default for RepeatMode {
    fn default() -> Self {
        Self::Off
    }
}

/// State of the playback queue.
///
/// Holds the list of tracks, the active track index, and playback mode state.
/// The `tracks` Vec is always kept in original order; shuffle only affects
/// which index is selected next via `current_index` and `played_indices`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueueState {
    pub tracks: Vec<Track>,
    pub current_index: Option<usize>,
    pub shuffle: bool,
    pub played_indices: Vec<usize>,
    pub repeat_mode: RepeatMode,
}

impl Default for QueueState {
    fn default() -> Self {
        Self {
            tracks: Vec::new(),
            current_index: None,
            shuffle: false,
            played_indices: Vec::new(),
            repeat_mode: RepeatMode::default(),
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
            shuffle: true,
            played_indices: vec![0],
            repeat_mode: RepeatMode::All,
        };
        let json = serde_json::to_string(&qs).unwrap();
        assert!(
            json.contains("\"currentIndex\""),
            "current_index should serialize as currentIndex"
        );
        assert!(
            json.contains("\"tracks\""),
            "tracks should serialize as tracks"
        );
        assert!(
            json.contains("\"shuffle\""),
            "shuffle should serialize as shuffle"
        );
        assert!(
            json.contains("\"playedIndices\""),
            "played_indices should serialize as playedIndices"
        );
        assert!(
            json.contains("\"repeatMode\""),
            "repeat_mode should serialize as repeatMode"
        );
    }

    #[test]
    fn queue_state_deserialize_from_camel_case() {
        let json = r#"{"tracks":[],"currentIndex":null,"shuffle":false,"playedIndices":[],"repeatMode":"Off"}"#;
        let qs: QueueState = serde_json::from_str(json).unwrap();
        assert!(qs.tracks.is_empty());
        assert!(qs.current_index.is_none());
        assert!(!qs.shuffle);
        assert!(qs.played_indices.is_empty());
        assert_eq!(qs.repeat_mode, RepeatMode::Off);
    }

    #[test]
    fn queue_state_default_has_default_modes() {
        let qs = QueueState::default();
        assert!(qs.tracks.is_empty());
        assert!(qs.current_index.is_none());
        assert!(!qs.shuffle);
        assert!(qs.played_indices.is_empty());
        assert_eq!(qs.repeat_mode, RepeatMode::Off);
    }

    #[test]
    fn repeat_mode_serializes_to_pascal_case() {
        assert_eq!(serde_json::to_string(&RepeatMode::Off).unwrap(), "\"Off\"");
        assert_eq!(serde_json::to_string(&RepeatMode::All).unwrap(), "\"All\"");
        assert_eq!(serde_json::to_string(&RepeatMode::One).unwrap(), "\"One\"");
    }

    #[test]
    fn repeat_mode_deserializes_from_pascal_case() {
        let off: RepeatMode = serde_json::from_str("\"Off\"").unwrap();
        let all: RepeatMode = serde_json::from_str("\"All\"").unwrap();
        let one: RepeatMode = serde_json::from_str("\"One\"").unwrap();
        assert_eq!(off, RepeatMode::Off);
        assert_eq!(all, RepeatMode::All);
        assert_eq!(one, RepeatMode::One);
    }
}
