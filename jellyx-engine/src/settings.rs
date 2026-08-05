//! Settings repository for audio and source configuration.
//! Desktop delegates here.

use crate::sqlite::SqliteHandle;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SourceSettingRow {
    pub source: String,
    pub enabled: bool,
}

pub struct SettingsRepository {
    handle: SqliteHandle,
}

impl SettingsRepository {
    pub fn new(handle: SqliteHandle) -> Self {
        Self { handle }
    }

    /// Get whether audio normalization is enabled.
    /// Defaults to true (enabled).
    pub fn get_normalize_audio(&self) -> Result<bool, rusqlite::Error> {
        let conn = self.handle.lock().map_err(|_| {
            rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_INTERNAL),
                Some("sqlite connection lock poisoned".to_string()),
            )
        })?;

        let result = conn.query_row(
            "SELECT value FROM audio_settings WHERE key = 'normalize_audio'",
            [],
            |row| row.get::<_, String>(0),
        );

        match result {
            Ok(val) => Ok(val == "1" || val == "true"),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(true),
            Err(e) => Err(e),
        }
    }

    /// Set whether audio normalization is enabled.
    pub fn set_normalize_audio(&self, enabled: bool) -> Result<(), rusqlite::Error> {
        let conn = self.handle.lock().map_err(|_| {
            rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_INTERNAL),
                Some("sqlite connection lock poisoned".to_string()),
            )
        })?;

        let val = if enabled { "1" } else { "0" };
        conn.execute(
            "INSERT INTO audio_settings (key, value) VALUES ('normalize_audio', ?1)
             ON CONFLICT(key) DO UPDATE SET value = ?1",
            rusqlite::params![val],
        )?;

        Ok(())
    }

    /// Get all source settings, seeding defaults for YouTube and SoundCloud.
    pub fn get_source_settings(&self) -> Result<Vec<SourceSettingRow>, rusqlite::Error> {
        let conn = self.handle.lock().map_err(|_| {
            rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_INTERNAL),
                Some("sqlite connection lock poisoned".to_string()),
            )
        })?;

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
                )?;
            }
        }

        let mut stmt =
            conn.prepare("SELECT source, enabled FROM source_settings ORDER BY source")?;

        let rows = stmt
            .query_map([], |row| {
                let source: String = row.get(0)?;
                let enabled: bool = row.get::<_, i64>(1)? != 0;
                Ok(SourceSettingRow { source, enabled })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(rows)
    }

    /// Set whether a source is enabled.
    pub fn set_source_enabled(&self, source: &str, enabled: bool) -> Result<(), rusqlite::Error> {
        let conn = self.handle.lock().map_err(|_| {
            rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_INTERNAL),
                Some("sqlite connection lock poisoned".to_string()),
            )
        })?;

        conn.execute(
            "INSERT INTO source_settings (source, enabled) VALUES (?1, ?2)
             ON CONFLICT(source) DO UPDATE SET enabled = ?2",
            rusqlite::params![source, enabled as i64],
        )?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn fresh_handle() -> SqliteHandle {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE audio_settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            CREATE TABLE source_settings (
                source TEXT PRIMARY KEY,
                enabled INTEGER NOT NULL DEFAULT 1
            );",
        )
        .unwrap();
        SqliteHandle::new(conn)
    }

    #[test]
    fn get_normalize_audio_defaults_to_true() {
        let handle = fresh_handle();
        let repo = SettingsRepository::new(handle);
        assert!(repo.get_normalize_audio().unwrap());
    }

    #[test]
    fn set_and_get_normalize_audio() {
        let handle = fresh_handle();
        let repo = SettingsRepository::new(handle);

        repo.set_normalize_audio(false).unwrap();
        assert!(!repo.get_normalize_audio().unwrap());

        repo.set_normalize_audio(true).unwrap();
        assert!(repo.get_normalize_audio().unwrap());
    }

    #[test]
    fn source_settings_seeds_defaults() {
        let handle = fresh_handle();
        let repo = SettingsRepository::new(handle);

        let rows = repo.get_source_settings().unwrap();
        assert_eq!(rows.len(), 2);

        let sources: Vec<&str> = rows.iter().map(|r| r.source.as_str()).collect();
        assert!(sources.contains(&"SoundCloud"));
        assert!(sources.contains(&"YouTube"));

        for row in &rows {
            assert!(row.enabled);
        }
    }

    #[test]
    fn set_source_enabled_toggles() {
        let handle = fresh_handle();
        let repo = SettingsRepository::new(handle);

        repo.get_source_settings().unwrap();
        repo.set_source_enabled("YouTube", false).unwrap();

        let rows = repo.get_source_settings().unwrap();
        let yt = rows.iter().find(|r| r.source == "YouTube").unwrap();
        assert!(!yt.enabled);

        let sc = rows.iter().find(|r| r.source == "SoundCloud").unwrap();
        assert!(sc.enabled);
    }

    #[test]
    fn set_source_enabled_idempotent() {
        let handle = fresh_handle();
        let repo = SettingsRepository::new(handle);

        repo.get_source_settings().unwrap();
        repo.set_source_enabled("YouTube", false).unwrap();
        repo.set_source_enabled("YouTube", false).unwrap();

        let rows = repo.get_source_settings().unwrap();
        let yt = rows.iter().find(|r| r.source == "YouTube").unwrap();
        assert!(!yt.enabled);
    }
}
