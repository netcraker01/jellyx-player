use jellyx_engine::preferences::{PreferencesRepository, RepositoryError, SourcePreference};

use super::db::Database;

impl PreferencesRepository for Database {
    fn source_preferences(&self) -> Result<Vec<SourcePreference>, RepositoryError> {
        self.handle().source_preferences()
    }

    fn set_source_enabled(&self, source: &str, enabled: bool) -> Result<(), RepositoryError> {
        self.handle().set_source_enabled(source, enabled)
    }

    fn normalize_audio(&self) -> Result<bool, RepositoryError> {
        self.handle().normalize_audio()
    }

    fn set_normalize_audio(&self, enabled: bool) -> Result<(), RepositoryError> {
        self.handle().set_normalize_audio(enabled)
    }

    fn telemetry_enabled(&self) -> Result<bool, RepositoryError> {
        self.handle().telemetry_enabled()
    }

    fn set_telemetry_enabled(&self, enabled: bool) -> Result<(), RepositoryError> {
        self.handle().set_telemetry_enabled(enabled)
    }
}
