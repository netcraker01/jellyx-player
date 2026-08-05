use jellyx_engine::preferences::{PreferencesRepository, RepositoryError, SourcePreference};

use super::db::Database;

impl PreferencesRepository for Database {
    fn source_preferences(&self) -> Result<Vec<SourcePreference>, RepositoryError> {
        self.engine.source_preferences()
    }

    fn set_source_enabled(&self, source: &str, enabled: bool) -> Result<(), RepositoryError> {
        self.engine.set_source_enabled(source, enabled)
    }

    fn normalize_audio(&self) -> Result<bool, RepositoryError> {
        self.engine.normalize_audio()
    }

    fn set_normalize_audio(&self, enabled: bool) -> Result<(), RepositoryError> {
        self.engine.set_normalize_audio(enabled)
    }

    fn telemetry_enabled(&self) -> Result<bool, RepositoryError> {
        self.engine.telemetry_enabled()
    }

    fn set_telemetry_enabled(&self, enabled: bool) -> Result<(), RepositoryError> {
        self.engine.set_telemetry_enabled(enabled)
    }
}
