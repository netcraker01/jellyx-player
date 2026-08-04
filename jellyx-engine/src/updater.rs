//! Platform-neutral updater preference contracts and suppression policy.

use serde::{Deserialize, Serialize};

pub use crate::preferences::RepositoryError;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePrefs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skipped_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remind_later_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_check_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detected_channel: Option<String>,
}

impl UpdatePrefs {
    pub fn is_skipped(&self, latest_version: &str) -> bool {
        match &self.skipped_version {
            Some(version) => same_version(version, latest_version),
            None => false,
        }
    }

    pub fn is_reminded_later(&self, now_iso: &str) -> bool {
        match &self.remind_later_at {
            Some(timestamp) => timestamp.as_bytes() > now_iso.as_bytes(),
            None => false,
        }
    }
}

fn same_version(left: &str, right: &str) -> bool {
    let normalize = |version: &str| version.trim().trim_start_matches('v').trim().to_lowercase();
    normalize(left) == normalize(right)
}

pub trait UpdatePreferencesRepository: Send + Sync {
    fn update_preferences(&self) -> Result<UpdatePrefs, RepositoryError>;
    fn save_update_preferences(&self, prefs: &UpdatePrefs) -> Result<(), RepositoryError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serde_preserves_fields_and_missing_values_default_to_none() {
        assert_eq!(
            serde_json::from_str::<UpdatePrefs>("{}").unwrap(),
            UpdatePrefs::default()
        );
        let prefs = UpdatePrefs {
            skipped_version: Some("v0.4.4".into()),
            remind_later_at: Some("2026-08-04T10:00:00Z".into()),
            ..UpdatePrefs::default()
        };
        assert_eq!(
            serde_json::to_value(prefs).unwrap(),
            serde_json::json!({
                "skippedVersion": "v0.4.4",
                "remindLaterAt": "2026-08-04T10:00:00Z"
            })
        );
    }

    #[test]
    fn suppression_policy_preserves_version_and_iso_comparisons() {
        let prefs = UpdatePrefs {
            skipped_version: Some("v0.4.4".into()),
            remind_later_at: Some("2026-08-04T10:00:00Z".into()),
            ..UpdatePrefs::default()
        };
        assert!(prefs.is_skipped("0.4.4"));
        assert!(prefs.is_reminded_later("2026-08-04T09:59:59Z"));
        assert!(!prefs.is_reminded_later("2026-08-04T10:00:00Z"));
    }
}
