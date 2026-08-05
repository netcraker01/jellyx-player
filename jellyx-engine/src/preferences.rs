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

const SETTINGS_SINGLETON_ID: i64 = 1;

impl PreferencesRepository for crate::sqlite::SqliteHandle {
    fn source_preferences(&self) -> Result<Vec<SourcePreference>, RepositoryError> {
        let conn = self
            .lock()
            .map_err(|e| RepositoryError::Storage(e.to_string()))?;
        for source in &["YouTube", "SoundCloud"] {
            let exists: bool = conn
                .query_row(
                    "SELECT COUNT(*) FROM source_settings WHERE source = ?1",
                    rusqlite::params![source],
                    |row| row.get::<_, i64>(0),
                )
                .map(|c| c > 0)
                .unwrap_or(false);
            if !exists {
                conn.execute(
                    "INSERT INTO source_settings (source, enabled) VALUES (?1, 1)",
                    rusqlite::params![source],
                )
                .map_err(|e| RepositoryError::Storage(e.to_string()))?;
            }
        }
        let mut stmt = conn
            .prepare("SELECT source, enabled FROM source_settings ORDER BY source")
            .map_err(|e| RepositoryError::Storage(e.to_string()))?;
        let entries = stmt
            .query_map([], |row| {
                Ok(SourcePreference {
                    source: row.get(0)?,
                    enabled: row.get::<_, i64>(1)? != 0,
                })
            })
            .map_err(|e| RepositoryError::Storage(e.to_string()))?
            .filter_map(|e| e.ok())
            .collect();
        Ok(entries)
    }

    fn set_source_enabled(&self, source: &str, enabled: bool) -> Result<(), RepositoryError> {
        let conn = self
            .lock()
            .map_err(|e| RepositoryError::Storage(e.to_string()))?;
        conn.execute(
            "INSERT INTO source_settings (source, enabled) VALUES (?1, ?2)
             ON CONFLICT(source) DO UPDATE SET enabled = ?2",
            rusqlite::params![source, enabled as i64],
        )
        .map_err(|e| RepositoryError::Storage(e.to_string()))?;
        Ok(())
    }

    fn normalize_audio(&self) -> Result<bool, RepositoryError> {
        let conn = self
            .lock()
            .map_err(|e| RepositoryError::Storage(e.to_string()))?;
        let result = conn.query_row(
            "SELECT value FROM audio_settings WHERE key = 'normalize_audio'",
            [],
            |row| row.get::<_, String>(0),
        );
        match result {
            Ok(val) => Ok(val == "1" || val == "true"),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(true),
            Err(e) => Err(RepositoryError::Storage(e.to_string())),
        }
    }

    fn set_normalize_audio(&self, enabled: bool) -> Result<(), RepositoryError> {
        let conn = self
            .lock()
            .map_err(|e| RepositoryError::Storage(e.to_string()))?;
        let val = if enabled { "1" } else { "0" };
        conn.execute(
            "INSERT INTO audio_settings (key, value) VALUES ('normalize_audio', ?1)
             ON CONFLICT(key) DO UPDATE SET value = ?1",
            rusqlite::params![val],
        )
        .map_err(|e| RepositoryError::Storage(e.to_string()))?;
        Ok(())
    }

    fn telemetry_enabled(&self) -> Result<bool, RepositoryError> {
        let conn = self
            .lock()
            .map_err(|e| RepositoryError::Storage(e.to_string()))?;
        let result = conn.query_row(
            "SELECT enabled FROM telemetry_prefs WHERE id = ?1",
            rusqlite::params![SETTINGS_SINGLETON_ID],
            |row| row.get::<_, i64>(0),
        );
        match result {
            Ok(value) => Ok(value != 0),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(false),
            Err(e) => Err(RepositoryError::Storage(e.to_string())),
        }
    }

    fn set_telemetry_enabled(&self, enabled: bool) -> Result<(), RepositoryError> {
        let conn = self
            .lock()
            .map_err(|e| RepositoryError::Storage(e.to_string()))?;
        conn.execute(
            "INSERT OR REPLACE INTO telemetry_prefs (id, enabled) VALUES (?1, ?2)",
            rusqlite::params![SETTINGS_SINGLETON_ID, i64::from(enabled)],
        )
        .map_err(|e| RepositoryError::Storage(e.to_string()))?;
        Ok(())
    }
}
