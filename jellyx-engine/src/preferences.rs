//! Platform-neutral contracts for persisted application preferences.

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourcePreference {
    pub source: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RepositoryError {
    Storage(String),
}

impl fmt::Display for RepositoryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Storage(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for RepositoryError {}

pub trait PreferencesRepository: Send + Sync {
    fn source_preferences(&self) -> Result<Vec<SourcePreference>, RepositoryError>;
    fn set_source_enabled(&self, source: &str, enabled: bool) -> Result<(), RepositoryError>;
    fn normalize_audio(&self) -> Result<bool, RepositoryError>;
    fn set_normalize_audio(&self, enabled: bool) -> Result<(), RepositoryError>;
    fn telemetry_enabled(&self) -> Result<bool, RepositoryError>;
    fn set_telemetry_enabled(&self, enabled: bool) -> Result<(), RepositoryError>;
}
