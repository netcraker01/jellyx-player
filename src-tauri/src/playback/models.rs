//! Playback internal DTOs.
//!
//! Domain-specific data transfer objects used by the playback module
//! and emitted as event payloads to the frontend.

use serde::{Deserialize, Serialize};

/// Progress tick payload emitted periodically during playback.
///
/// Serialized as camelCase to match TypeScript frontend types.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressTick {
    pub position: f64,
    pub duration: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_tick_camel_case_serialization() {
        let tick = ProgressTick {
            position: 45.2,
            duration: 240.0,
        };
        let json = serde_json::to_string(&tick).unwrap();
        assert!(json.contains("\"position\""), "position field should be present");
        assert!(json.contains("\"duration\""), "duration field should be present");
        assert!(json.contains("45.2"), "position value should serialize");
        assert!(json.contains("240.0"), "duration value should serialize");
    }

    #[test]
    fn progress_tick_deserialize_from_camel_case() {
        let json = r#"{"position": 45.2, "duration": 240.0}"#;
        let tick: ProgressTick = serde_json::from_str(json).unwrap();
        assert_eq!(tick.position, 45.2);
        assert_eq!(tick.duration, 240.0);
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