//! Settings service — manages source plugin enable/disable state and audio settings.

use std::sync::Arc;

use jellyx_engine::preferences::{PreferencesRepository, RepositoryError};

use crate::errors::types::AppError;
use crate::persistence::models::{AudioSettings, SourceSetting, TelemetrySettings};

/// Service for managing application settings, including source enablement and audio normalization.
pub struct SettingsService {
    repository: Arc<dyn PreferencesRepository>,
}

impl SettingsService {
    pub fn new(repository: Arc<dyn PreferencesRepository>) -> Self {
        Self { repository }
    }

    /// Get all source settings (YouTube, SoundCloud), defaulting to enabled.
    pub fn get_source_settings(&self) -> Result<Vec<SourceSetting>, AppError> {
        self.repository
            .source_preferences()
            .map(|settings| {
                settings
                    .into_iter()
                    .map(|setting| SourceSetting {
                        label: setting.source.clone(),
                        source: setting.source,
                        enabled: setting.enabled,
                    })
                    .collect()
            })
            .map_err(repository_error)
    }

    /// Set whether a source is enabled.
    pub fn set_source_enabled(&self, source: &str, enabled: bool) -> Result<(), AppError> {
        self.repository
            .set_source_enabled(source, enabled)
            .map_err(repository_error)
    }

    /// Get the set of currently enabled source names.
    pub fn get_enabled_sources(&self) -> Result<std::collections::HashSet<String>, AppError> {
        Ok(self
            .get_source_settings()?
            .into_iter()
            .filter(|setting| setting.enabled)
            .map(|setting| setting.source)
            .collect())
    }

    /// Get audio settings (normalization toggle, etc.).
    pub fn get_audio_settings(&self) -> Result<AudioSettings, AppError> {
        let normalize_audio = self
            .repository
            .normalize_audio()
            .map_err(repository_error)?;
        Ok(AudioSettings { normalize_audio })
    }

    /// Set whether audio normalization is enabled.
    pub fn set_normalize_audio(&self, enabled: bool) -> Result<(), AppError> {
        self.repository
            .set_normalize_audio(enabled)
            .map_err(repository_error)
    }

    pub fn get_telemetry_settings(&self) -> Result<TelemetrySettings, AppError> {
        Ok(TelemetrySettings {
            enabled: self
                .repository
                .telemetry_enabled()
                .map_err(repository_error)?,
        })
    }

    pub fn set_telemetry_enabled(&self, enabled: bool) -> Result<(), AppError> {
        self.repository
            .set_telemetry_enabled(enabled)
            .map_err(repository_error)
    }
}

fn repository_error(error: RepositoryError) -> AppError {
    AppError {
        code: "PERSISTENCE_ERROR".into(),
        details: Some(error.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jellyx_engine::preferences::{PreferencesRepository, RepositoryError, SourcePreference};
    use std::sync::Mutex;

    struct FakePreferencesRepository {
        sources: Mutex<Vec<SourcePreference>>,
        normalize_audio: Mutex<bool>,
        telemetry_enabled: Mutex<bool>,
    }

    impl PreferencesRepository for FakePreferencesRepository {
        fn source_preferences(&self) -> Result<Vec<SourcePreference>, RepositoryError> {
            Ok(self.sources.lock().unwrap().clone())
        }

        fn set_source_enabled(&self, source: &str, enabled: bool) -> Result<(), RepositoryError> {
            self.sources
                .lock()
                .unwrap()
                .iter_mut()
                .find(|setting| setting.source == source)
                .unwrap()
                .enabled = enabled;
            Ok(())
        }

        fn normalize_audio(&self) -> Result<bool, RepositoryError> {
            Ok(*self.normalize_audio.lock().unwrap())
        }

        fn set_normalize_audio(&self, enabled: bool) -> Result<(), RepositoryError> {
            *self.normalize_audio.lock().unwrap() = enabled;
            Ok(())
        }

        fn telemetry_enabled(&self) -> Result<bool, RepositoryError> {
            Ok(*self.telemetry_enabled.lock().unwrap())
        }

        fn set_telemetry_enabled(&self, enabled: bool) -> Result<(), RepositoryError> {
            *self.telemetry_enabled.lock().unwrap() = enabled;
            Ok(())
        }
    }

    #[test]
    fn settings_service_uses_preferences_repository_without_database() {
        let repository = Arc::new(FakePreferencesRepository {
            sources: Mutex::new(vec![SourcePreference {
                source: "YouTube".into(),
                enabled: true,
            }]),
            normalize_audio: Mutex::new(true),
            telemetry_enabled: Mutex::new(false),
        });
        let service = SettingsService::new(repository);

        service.set_source_enabled("YouTube", false).unwrap();
        service.set_normalize_audio(false).unwrap();
        service.set_telemetry_enabled(true).unwrap();

        assert!(service.get_enabled_sources().unwrap().is_empty());
        assert!(!service.get_audio_settings().unwrap().normalize_audio);
        assert!(service.get_telemetry_settings().unwrap().enabled);
    }

    #[test]
    fn sqlite_adapter_preserves_defaults_and_roundtrips_preferences() {
        let database = Arc::new(crate::persistence::db::Database::open_in_memory().unwrap());
        let service = SettingsService::new(database);

        let sources = service.get_source_settings().unwrap();
        assert_eq!(sources.len(), 2);
        assert!(sources
            .iter()
            .any(|source| source.source == "YouTube" && source.enabled));
        assert!(sources
            .iter()
            .any(|source| source.source == "SoundCloud" && source.enabled));
        assert!(service.get_audio_settings().unwrap().normalize_audio);
        assert!(!service.get_telemetry_settings().unwrap().enabled);

        service.set_source_enabled("SoundCloud", false).unwrap();
        service.set_normalize_audio(false).unwrap();
        service.set_telemetry_enabled(true).unwrap();
        assert_eq!(
            service.get_enabled_sources().unwrap(),
            ["YouTube".to_string()].into()
        );
        assert!(!service.get_audio_settings().unwrap().normalize_audio);
        assert!(service.get_telemetry_settings().unwrap().enabled);
    }

    #[test]
    fn repository_service_writes_survive_file_reopen_for_legacy_readers() {
        let path = std::env::temp_dir().join(format!(
            "jellyx-preferences-repository-{}.db",
            uuid::Uuid::new_v4()
        ));
        {
            let database = Arc::new(crate::persistence::db::Database::open(&path).unwrap());
            let service = SettingsService::new(database);
            service.set_source_enabled("YouTube", false).unwrap();
            service.set_normalize_audio(false).unwrap();
            service.set_telemetry_enabled(true).unwrap();
        }

        let database = Arc::new(crate::persistence::db::Database::open(&path).unwrap());
        assert!(!database.get_normalize_audio().unwrap());
        let service = SettingsService::new(database);
        assert_eq!(
            service.get_enabled_sources().unwrap(),
            ["SoundCloud".to_string()].into()
        );
        assert!(service.get_telemetry_settings().unwrap().enabled);
        drop(service);

        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(format!("{}.migration.lock", path.display()));
    }
}
