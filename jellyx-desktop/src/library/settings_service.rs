//! Settings service — re-exported from the engine.
//!
//! The engine owns the settings service logic so both Tauri and Ratatui
//! frontends share a single source of truth. Desktop re-exports the engine
//! types and provides an adapter that maps `SettingsError` to `AppError`.

use std::sync::Arc;

use jellyx_engine::preferences::PreferencesRepository;
use jellyx_engine::settings_service::{
    AudioSettings as EngineAudioSettings, SettingsError, SettingsService as EngineSettingsService,
    SourceSetting as EngineSourceSetting, TelemetrySettings as EngineTelemetrySettings,
};

use crate::errors::types::AppError;
use crate::persistence::models::{AudioSettings, SourceSetting, TelemetrySettings};

pub struct SettingsService {
    inner: EngineSettingsService,
}

impl SettingsService {
    pub fn new(repository: Arc<dyn PreferencesRepository>) -> Self {
        Self {
            inner: EngineSettingsService::new(repository),
        }
    }

    pub fn get_source_settings(&self) -> Result<Vec<SourceSetting>, AppError> {
        self.inner
            .get_source_settings()
            .map(|settings| {
                settings
                    .into_iter()
                    .map(|s| SourceSetting {
                        source: s.source,
                        enabled: s.enabled,
                        label: s.label,
                    })
                    .collect()
            })
            .map_err(map_error)
    }

    pub fn set_source_enabled(&self, source: &str, enabled: bool) -> Result<(), AppError> {
        self.inner
            .set_source_enabled(source, enabled)
            .map_err(map_error)
    }

    pub fn get_enabled_sources(&self) -> Result<std::collections::HashSet<String>, AppError> {
        self.inner.get_enabled_sources().map_err(map_error)
    }

    pub fn get_audio_settings(&self) -> Result<AudioSettings, AppError> {
        self.inner
            .get_audio_settings()
            .map(|a| AudioSettings {
                normalize_audio: a.normalize_audio,
            })
            .map_err(map_error)
    }

    pub fn set_normalize_audio(&self, enabled: bool) -> Result<(), AppError> {
        self.inner.set_normalize_audio(enabled).map_err(map_error)
    }

    pub fn get_telemetry_settings(&self) -> Result<TelemetrySettings, AppError> {
        self.inner
            .get_telemetry_settings()
            .map(|t| TelemetrySettings { enabled: t.enabled })
            .map_err(map_error)
    }

    pub fn set_telemetry_enabled(&self, enabled: bool) -> Result<(), AppError> {
        self.inner.set_telemetry_enabled(enabled).map_err(map_error)
    }
}

fn map_error(e: SettingsError) -> AppError {
    AppError {
        code: "PERSISTENCE_ERROR".into(),
        details: Some(e.0),
    }
}
