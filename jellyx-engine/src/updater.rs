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

const SETTINGS_SINGLETON_ID: i64 = 1;

impl UpdatePreferencesRepository for crate::sqlite::SqliteHandle {
    fn update_preferences(&self) -> Result<UpdatePrefs, RepositoryError> {
        let conn = self
            .lock()
            .map_err(|e| RepositoryError::Storage(e.to_string()))?;
        let result = conn.query_row(
            "SELECT skipped_version, remind_later_at, last_check_at, detected_channel
             FROM update_prefs WHERE id = ?1",
            rusqlite::params![SETTINGS_SINGLETON_ID],
            |row| {
                Ok(UpdatePrefs {
                    skipped_version: row.get(0)?,
                    remind_later_at: row.get(1)?,
                    last_check_at: row.get(2)?,
                    detected_channel: row.get(3)?,
                })
            },
        );
        match result {
            Ok(prefs) => Ok(prefs),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(UpdatePrefs::default()),
            Err(e) => Err(RepositoryError::Storage(e.to_string())),
        }
    }

    fn save_update_preferences(&self, prefs: &UpdatePrefs) -> Result<(), RepositoryError> {
        let conn = self
            .lock()
            .map_err(|e| RepositoryError::Storage(e.to_string()))?;
        conn.execute(
            "INSERT OR REPLACE INTO update_prefs
                (id, skipped_version, remind_later_at, last_check_at, detected_channel)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![
                SETTINGS_SINGLETON_ID,
                prefs.skipped_version,
                prefs.remind_later_at,
                prefs.last_check_at,
                prefs.detected_channel
            ],
        )
        .map_err(|e| RepositoryError::Storage(e.to_string()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::preferences::PreferencesRepository;
    use crate::sqlite::SqliteHandle;

    fn test_handle() -> SqliteHandle {
        let handle = SqliteHandle::open_in_memory().unwrap();
        handle.initialize_schema().unwrap();
        handle
    }

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

    #[test]
    fn update_preferences_returns_default_when_empty() {
        let h = test_handle();
        assert_eq!(h.update_preferences().unwrap(), UpdatePrefs::default());
    }

    #[test]
    fn save_and_get_update_preferences_round_trip() {
        let h = test_handle();
        let prefs = UpdatePrefs {
            skipped_version: Some("v0.4.5".into()),
            remind_later_at: Some("2026-01-01T00:00:00Z".into()),
            ..Default::default()
        };
        h.save_update_preferences(&prefs).unwrap();
        assert_eq!(h.update_preferences().unwrap(), prefs);
    }

    #[test]
    fn telemetry_defaults_to_disabled() {
        let h = test_handle();
        assert!(!h.telemetry_enabled().unwrap());
    }

    #[test]
    fn set_telemetry_enabled_round_trip() {
        let h = test_handle();
        h.set_telemetry_enabled(true).unwrap();
        assert!(h.telemetry_enabled().unwrap());
    }
}
