//! Settings service — manages source plugin enable/disable state.

use std::sync::Arc;

use crate::errors::types::AppError;
use crate::persistence::db::Database;
use crate::persistence::models::SourceSetting;

/// Service for managing application settings, including source enablement.
pub struct SettingsService {
    db: Arc<Database>,
}

impl SettingsService {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    /// Get all source settings (YouTube, SoundCloud), defaulting to enabled.
    pub fn get_source_settings(&self) -> Result<Vec<SourceSetting>, AppError> {
        self.db.get_source_settings().map_err(AppError::from)
    }

    /// Set whether a source is enabled.
    pub fn set_source_enabled(&self, source: &str, enabled: bool) -> Result<(), AppError> {
        self.db
            .set_source_enabled(source, enabled)
            .map_err(AppError::from)
    }

    /// Get the set of currently enabled source names.
    pub fn get_enabled_sources(&self) -> Result<std::collections::HashSet<String>, AppError> {
        self.db.get_enabled_sources().map_err(AppError::from)
    }
}