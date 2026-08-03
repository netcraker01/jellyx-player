use jellyx_engine::preferences::{PreferencesRepository, RepositoryError, SourcePreference};

use super::db::Database;
use crate::errors::types::PersistenceError;

pub(super) fn storage_error(error: PersistenceError) -> RepositoryError {
    let message = match error {
        PersistenceError::DatabaseError(message) | PersistenceError::WriteError(message) => message,
    };
    RepositoryError::Storage(message)
}

impl PreferencesRepository for Database {
    fn source_preferences(&self) -> Result<Vec<SourcePreference>, RepositoryError> {
        self.get_source_settings()
            .map(|settings| {
                settings
                    .into_iter()
                    .map(|setting| SourcePreference {
                        source: setting.source,
                        enabled: setting.enabled,
                    })
                    .collect()
            })
            .map_err(storage_error)
    }

    fn set_source_enabled(&self, source: &str, enabled: bool) -> Result<(), RepositoryError> {
        self.set_source_enabled(source, enabled)
            .map_err(storage_error)
    }

    fn normalize_audio(&self) -> Result<bool, RepositoryError> {
        self.get_normalize_audio().map_err(storage_error)
    }

    fn set_normalize_audio(&self, enabled: bool) -> Result<(), RepositoryError> {
        self.set_normalize_audio(enabled).map_err(storage_error)
    }

    fn telemetry_enabled(&self) -> Result<bool, RepositoryError> {
        self.get_telemetry_enabled().map_err(storage_error)
    }

    fn set_telemetry_enabled(&self, enabled: bool) -> Result<(), RepositoryError> {
        self.set_telemetry_enabled(enabled).map_err(storage_error)
    }
}
