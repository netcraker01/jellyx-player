//! Settings service — manages source enable/disable, audio normalization,
//! and telemetry consent.
//!
//! Uses the `PreferencesRepository` trait so any concrete implementation
//! (SqliteHandle, fake, future remote) can be injected.

use std::collections::HashSet;
use std::sync::Arc;

use crate::preferences::{PreferencesRepository, RepositoryError};

/// Source setting returned by the settings service.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceSetting {
    pub source: String,
    pub enabled: bool,
    /// Human-readable label (same as source name for now).
    pub label: String,
}

/// Audio settings returned by the settings service.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioSettings {
    pub normalize_audio: bool,
}

/// Telemetry consent settings.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TelemetrySettings {
    pub enabled: bool,
}

/// Error returned by the settings service.
#[derive(Debug, Clone)]
pub struct SettingsError(pub String);

impl std::fmt::Display for SettingsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<RepositoryError> for SettingsError {
    fn from(e: RepositoryError) -> Self {
        Self(e.to_string())
    }
}

/// Service for managing application settings.
pub struct SettingsService {
    repository: Arc<dyn PreferencesRepository>,
}

impl SettingsService {
    pub fn new(repository: Arc<dyn PreferencesRepository>) -> Self {
        Self { repository }
    }

    /// Get all source settings, defaulting to enabled for known sources.
    pub fn get_source_settings(&self) -> Result<Vec<SourceSetting>, SettingsError> {
        self.repository
            .source_preferences()
            .map(|settings| {
                settings
                    .into_iter()
                    .map(|s| SourceSetting {
                        label: s.source.clone(),
                        source: s.source,
                        enabled: s.enabled,
                    })
                    .collect()
            })
            .map_err(SettingsError::from)
    }

    /// Set whether a source is enabled.
    pub fn set_source_enabled(&self, source: &str, enabled: bool) -> Result<(), SettingsError> {
        self.repository
            .set_source_enabled(source, enabled)
            .map_err(SettingsError::from)
    }

    /// Get the set of currently enabled source names.
    pub fn get_enabled_sources(&self) -> Result<HashSet<String>, SettingsError> {
        Ok(self
            .get_source_settings()?
            .into_iter()
            .filter(|s| s.enabled)
            .map(|s| s.source)
            .collect())
    }

    /// Get audio settings (normalization toggle).
    pub fn get_audio_settings(&self) -> Result<AudioSettings, SettingsError> {
        let normalize_audio = self
            .repository
            .normalize_audio()
            .map_err(SettingsError::from)?;
        Ok(AudioSettings { normalize_audio })
    }

    /// Set whether audio normalization is enabled.
    pub fn set_normalize_audio(&self, enabled: bool) -> Result<(), SettingsError> {
        self.repository
            .set_normalize_audio(enabled)
            .map_err(SettingsError::from)
    }

    /// Get telemetry consent settings.
    pub fn get_telemetry_settings(&self) -> Result<TelemetrySettings, SettingsError> {
        let enabled = self
            .repository
            .telemetry_enabled()
            .map_err(SettingsError::from)?;
        Ok(TelemetrySettings { enabled })
    }

    /// Set telemetry consent.
    pub fn set_telemetry_enabled(&self, enabled: bool) -> Result<(), SettingsError> {
        self.repository
            .set_telemetry_enabled(enabled)
            .map_err(SettingsError::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::preferences::{PreferencesRepository, RepositoryError, SourcePreference};
    use crate::sqlite::SqliteHandle;
    use std::sync::Mutex;

    struct FakeRepo {
        sources: Mutex<Vec<SourcePreference>>,
        normalize: Mutex<bool>,
        telemetry: Mutex<bool>,
    }

    impl PreferencesRepository for FakeRepo {
        fn source_preferences(&self) -> Result<Vec<SourcePreference>, RepositoryError> {
            Ok(self.sources.lock().unwrap().clone())
        }
        fn set_source_enabled(&self, source: &str, enabled: bool) -> Result<(), RepositoryError> {
            self.sources
                .lock()
                .unwrap()
                .iter_mut()
                .find(|s| s.source == source)
                .unwrap()
                .enabled = enabled;
            Ok(())
        }
        fn normalize_audio(&self) -> Result<bool, RepositoryError> {
            Ok(*self.normalize.lock().unwrap())
        }
        fn set_normalize_audio(&self, enabled: bool) -> Result<(), RepositoryError> {
            *self.normalize.lock().unwrap() = enabled;
            Ok(())
        }
        fn telemetry_enabled(&self) -> Result<bool, RepositoryError> {
            Ok(*self.telemetry.lock().unwrap())
        }
        fn set_telemetry_enabled(&self, enabled: bool) -> Result<(), RepositoryError> {
            *self.telemetry.lock().unwrap() = enabled;
            Ok(())
        }
    }

    #[test]
    fn fake_repo_round_trip() {
        let repo = Arc::new(FakeRepo {
            sources: Mutex::new(vec![SourcePreference {
                source: "YouTube".into(),
                enabled: true,
            }]),
            normalize: Mutex::new(true),
            telemetry: Mutex::new(false),
        });
        let svc = SettingsService::new(repo);

        svc.set_source_enabled("YouTube", false).unwrap();
        svc.set_normalize_audio(false).unwrap();
        svc.set_telemetry_enabled(true).unwrap();

        assert!(svc.get_enabled_sources().unwrap().is_empty());
        assert!(!svc.get_audio_settings().unwrap().normalize_audio);
        assert!(svc.get_telemetry_settings().unwrap().enabled);
    }
    #[test]
    fn sqlite_handle_round_trip() {
        use rusqlite::Connection;
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS audio_settings (key TEXT PRIMARY KEY, value TEXT);
             CREATE TABLE IF NOT EXISTS source_settings (source TEXT PRIMARY KEY, enabled INTEGER);
             CREATE TABLE IF NOT EXISTS telemetry_prefs (id INTEGER PRIMARY KEY, enabled INTEGER);",
        )
        .unwrap();
        let handle = SqliteHandle::new(conn);

        let svc = SettingsService::new(Arc::new(handle));
        let sources = svc.get_source_settings().unwrap();
        assert_eq!(sources.len(), 2);
        assert!(sources.iter().any(|s| s.source == "YouTube" && s.enabled));
        assert!(
            sources
                .iter()
                .any(|s| s.source == "SoundCloud" && s.enabled)
        );
        assert!(svc.get_audio_settings().unwrap().normalize_audio);
        assert!(!svc.get_telemetry_settings().unwrap().enabled);

        svc.set_source_enabled("SoundCloud", false).unwrap();
        svc.set_normalize_audio(false).unwrap();
        svc.set_telemetry_enabled(true).unwrap();

        assert_eq!(
            svc.get_enabled_sources().unwrap(),
            ["YouTube".to_string()].into()
        );
        assert!(!svc.get_audio_settings().unwrap().normalize_audio);
        assert!(svc.get_telemetry_settings().unwrap().enabled);
    }
}
