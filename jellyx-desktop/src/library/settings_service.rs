//! Settings service — manages source plugin enable/disable state and audio settings.

use std::sync::Arc;

use crate::errors::types::AppError;
use crate::persistence::db::Database;
use crate::persistence::models::{AudioSettings, SourceSetting, TelemetrySettings};

/// Service for managing application settings, including source enablement and audio normalization.
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

    /// Get audio settings (normalization toggle, etc.).
    pub fn get_audio_settings(&self) -> Result<AudioSettings, AppError> {
        let normalize_audio = self.db.get_normalize_audio().map_err(AppError::from)?;
        Ok(AudioSettings { normalize_audio })
    }

    /// Set whether audio normalization is enabled.
    pub fn set_normalize_audio(&self, enabled: bool) -> Result<(), AppError> {
        self.db.set_normalize_audio(enabled).map_err(AppError::from)
    }

    pub fn get_telemetry_settings(&self) -> Result<TelemetrySettings, AppError> {
        Ok(TelemetrySettings {
            enabled: self.db.get_telemetry_enabled().map_err(AppError::from)?,
        })
    }

    pub fn set_telemetry_enabled(&self, enabled: bool) -> Result<(), AppError> {
        self.db
            .set_telemetry_enabled(enabled)
            .map_err(AppError::from)
    }
}
