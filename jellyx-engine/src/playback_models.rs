//! Playback state and queue models — shared by all frontends.

use jellyx_core::models::track::Track;
use serde::{Deserialize, Serialize};

/// Current playback state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum PlaybackState {
    Stopped,
    Playing,
    Paused,
    Buffering(f32),
}

/// Repeat mode for queue playback.
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

/// Progress tick payload emitted periodically during playback.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressTick {
    pub position: f64,
    pub duration: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use jellyx_core::models::source::Source;

    #[test]
    fn playback_state_playing_serializes_to_pascal_case() {
        let json = serde_json::to_string(&PlaybackState::Playing).unwrap();
        assert_eq!(json, "\"Playing\"");
    }

    #[test]
    fn playback_state_buffering_serializes_to_pascal_case() {
        let json = serde_json::to_string(&PlaybackState::Buffering(0.75)).unwrap();
        assert!(json.contains("\"Buffering\""));
        assert!(json.contains("0.75"));
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
            playlist_id: None,
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
        assert!(json.contains("\"currentIndex\""));
        assert!(json.contains("\"playedIndices\""));
        assert!(json.contains("\"repeatMode\""));
    }

    #[test]
    fn queue_state_deserialize_from_camel_case() {
        let json = r#"{"tracks":[],"currentIndex":null,"shuffle":false,"playedIndices":[],"repeatMode":"Off"}"#;
        let qs: QueueState = serde_json::from_str(json).unwrap();
        assert!(qs.tracks.is_empty());
        assert_eq!(qs.repeat_mode, RepeatMode::Off);
    }

    #[test]
    fn repeat_mode_serializes_to_pascal_case() {
        assert_eq!(serde_json::to_string(&RepeatMode::Off).unwrap(), "\"Off\"");
        assert_eq!(serde_json::to_string(&RepeatMode::All).unwrap(), "\"All\"");
        assert_eq!(serde_json::to_string(&RepeatMode::One).unwrap(), "\"One\"");
    }

    #[test]
    fn progress_tick_roundtrip() {
        let tick = ProgressTick {
            position: 120.5,
            duration: 360.0,
        };
        let json = serde_json::to_string(&tick).unwrap();
        let deserialized: ProgressTick = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.position, tick.position);
        assert_eq!(deserialized.duration, tick.duration);
    }
}
