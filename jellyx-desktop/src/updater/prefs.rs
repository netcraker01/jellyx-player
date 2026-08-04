//! Updater preferences persistence.
//!
//! Stored in a dedicated `update_prefs` table (single-row, keyed by `id = 1`)
//! rather than the generic `_meta` key-value table. The design decision was
//! made in `sdd/channel-aware-updater/design`: dedicated tables are the
//! codebase convention for distinct domains (history, playlists, etc.) and
//! this keeps the updater state shape explicit and typed.
//!
//! Required stored fields (per spec "Updater State Persistence"):
//! - `skipped_version`: latest version the user chose to skip
//! - `remind_later_at`: ISO-8601 timestamp; modal suppressed before this time
//! - `last_check_at`: ISO-8601 timestamp of the last successful check
//! - `detected_channel`: canonical channel string detected at startup

use std::sync::Arc;

use jellyx_engine::updater::{RepositoryError, UpdatePreferencesRepository};

use crate::errors::types::PersistenceError;
use crate::updater::channel::InstallChannel;

pub use jellyx_engine::updater::UpdatePrefs;

/// Updater preference use cases over the platform-neutral repository port.
pub struct UpdatePrefsService {
    repository: Arc<dyn UpdatePreferencesRepository>,
}

impl UpdatePrefsService {
    pub fn new(repository: Arc<dyn UpdatePreferencesRepository>) -> Self {
        Self { repository }
    }

    /// Read the current updater prefs. Missing fields default to `None`.
    pub fn get(&self) -> Result<UpdatePrefs, PersistenceError> {
        self.repository
            .update_preferences()
            .map_err(repository_error)
    }

    /// Persist the full prefs row (insert or replace).
    pub fn save(&self, prefs: &UpdatePrefs) -> Result<(), PersistenceError> {
        self.repository
            .save_update_preferences(prefs)
            .map_err(repository_error)
    }

    /// Set the skipped version and return the updated prefs.
    pub fn set_skipped_version(&self, version: &str) -> Result<UpdatePrefs, PersistenceError> {
        let mut prefs = self.get()?;
        prefs.skipped_version = Some(version.to_string());
        self.save(&prefs)?;
        Ok(prefs)
    }

    /// Set the remind-later timestamp and return the updated prefs.
    pub fn set_remind_later(&self, iso_ts: &str) -> Result<UpdatePrefs, PersistenceError> {
        let mut prefs = self.get()?;
        prefs.remind_later_at = Some(iso_ts.to_string());
        self.save(&prefs)?;
        Ok(prefs)
    }

    /// Set the last successful check timestamp and return the updated prefs.
    pub fn set_last_check(&self, iso_ts: &str) -> Result<UpdatePrefs, PersistenceError> {
        let mut prefs = self.get()?;
        prefs.last_check_at = Some(iso_ts.to_string());
        self.save(&prefs)?;
        Ok(prefs)
    }

    /// Persist the detected install channel (called once at startup).
    pub fn set_detected_channel(
        &self,
        channel: InstallChannel,
    ) -> Result<UpdatePrefs, PersistenceError> {
        let mut prefs = self.get()?;
        prefs.detected_channel = Some(channel.as_str().to_string());
        self.save(&prefs)?;
        Ok(prefs)
    }
}

fn repository_error(error: RepositoryError) -> PersistenceError {
    match error {
        RepositoryError::Storage(message) => PersistenceError::DatabaseError(message),
    }
}

/// Format `now` as an ISO-8601 UTC timestamp suitable for storing in
/// `remind_later_at` / `last_check_at`.
///
/// Implemented without pulling `chrono` into the public API: we use SQLite's
/// `datetime('now')` for `last_check_at` writes inside the DB layer, and for
/// remind-later we add a number of seconds in the frontend-agnostic way. For
/// the in-Rust timestamp we use `std::time::SystemTime` and format manually
/// to avoid a hard chrono dependency in callers.
pub fn now_iso_utc() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format_iso_utc(secs)
}

/// Format a Unix-seconds timestamp as `YYYY-MM-DDTHH:MM:SSZ`.
pub fn format_iso_utc(unix_secs: u64) -> String {
    // Minimal civil-time conversion (Howard Hinnant's algorithm). Good for
    // 1970-2100; no leap-second handling (acceptable for updater timestamps).
    let days = (unix_secs / 86400) as i64;
    let secs_of_day = (unix_secs % 86400) as u64;
    let (y, m, d) = civil_from_days(days);
    let hh = secs_of_day / 3600;
    let mm = (secs_of_day % 3600) / 60;
    let ss = secs_of_day % 60;
    format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z", y, m, d, hh, mm, ss)
}

/// Add `seconds` to a `now_iso_utc()` timestamp and return the resulting ISO string.
pub fn now_plus_seconds(seconds: u64) -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() + seconds)
        .unwrap_or(seconds);
    format_iso_utc(secs)
}

fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as i64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    use jellyx_engine::updater::{RepositoryError, UpdatePreferencesRepository};

    use crate::persistence::db::Database;

    #[derive(Default)]
    struct FakeUpdatePreferencesRepository(Mutex<UpdatePrefs>);

    impl UpdatePreferencesRepository for FakeUpdatePreferencesRepository {
        fn update_preferences(&self) -> Result<UpdatePrefs, RepositoryError> {
            Ok(self.0.lock().unwrap().clone())
        }

        fn save_update_preferences(&self, prefs: &UpdatePrefs) -> Result<(), RepositoryError> {
            *self.0.lock().unwrap() = prefs.clone();
            Ok(())
        }
    }

    #[test]
    fn prefs_service_uses_repository_without_database() {
        let repository = Arc::new(FakeUpdatePreferencesRepository::default());
        let service = UpdatePrefsService::new(repository.clone());

        service.set_skipped_version("v0.4.4").unwrap();
        let prefs = service.set_remind_later("2026-08-04T10:00:00Z").unwrap();

        assert_eq!(prefs.skipped_version.as_deref(), Some("v0.4.4"));
        assert_eq!(
            repository.update_preferences().unwrap(),
            UpdatePrefs {
                skipped_version: Some("v0.4.4".into()),
                remind_later_at: Some("2026-08-04T10:00:00Z".into()),
                ..UpdatePrefs::default()
            }
        );
    }

    #[test]
    fn is_skipped_matches_with_or_without_v_prefix() {
        let p = UpdatePrefs {
            skipped_version: Some("v0.2.3".to_string()),
            ..Default::default()
        };
        assert!(p.is_skipped("0.2.3"));
        assert!(p.is_skipped("v0.2.3"));
        assert!(!p.is_skipped("0.2.4"));
    }

    #[test]
    fn is_reminded_later_compares_lexicographically() {
        let p = UpdatePrefs {
            remind_later_at: Some("2026-07-10T00:00:00Z".to_string()),
            ..Default::default()
        };
        assert!(p.is_reminded_later("2026-07-09T00:00:00Z"));
        assert!(!p.is_reminded_later("2026-07-10T00:00:00Z"));
        assert!(!p.is_reminded_later("2026-07-11T00:00:00Z"));
    }

    #[test]
    fn prefs_service_roundtrip_in_memory() {
        let db = Arc::new(Database::open_in_memory().expect("open in-memory db"));
        let svc = UpdatePrefsService::new(db);

        // Fresh install has empty state.
        let empty = svc.get().expect("get empty prefs");
        assert_eq!(empty, UpdatePrefs::default());

        // Save and reload.
        let prefs = UpdatePrefs {
            skipped_version: Some("v0.3.0".to_string()),
            remind_later_at: Some("2026-08-01T00:00:00Z".to_string()),
            last_check_at: Some("2026-07-07T10:00:00Z".to_string()),
            detected_channel: Some("linux-deb".to_string()),
        };
        svc.save(&prefs).expect("save prefs");
        let loaded = svc.get().expect("reload prefs");
        assert_eq!(loaded, prefs);

        // Incremental updates.
        let after_skip = svc.set_skipped_version("v0.4.0").expect("set skip");
        assert_eq!(after_skip.skipped_version.as_deref(), Some("v0.4.0"));
        assert_eq!(
            after_skip.last_check_at.as_deref(),
            Some("2026-07-07T10:00:00Z")
        );

        let after_channel = svc
            .set_detected_channel(InstallChannel::Flatpak)
            .expect("set channel");
        assert_eq!(after_channel.detected_channel.as_deref(), Some("flatpak"));
    }

    #[test]
    fn now_iso_utc_is_z_suffixed() {
        let ts = now_iso_utc();
        assert!(ts.ends_with('Z'), "expected Z suffix, got {}", ts);
        assert_eq!(
            ts.len(),
            20,
            "expected YYYY-MM-DDTHH:MM:SSZ (20 chars), got {}",
            ts
        );
    }

    #[test]
    fn format_iso_utc_known_epoch() {
        // 1783527660 = 2026-07-08T16:21:00Z (verified with `date -u -d @1783527660`).
        assert_eq!(format_iso_utc(1783527660), "2026-07-08T16:21:00Z");
        // A second known point: 0 = epoch.
        assert_eq!(format_iso_utc(0), "1970-01-01T00:00:00Z");
        // 86400 = exactly one day after epoch.
        assert_eq!(format_iso_utc(86400), "1970-01-02T00:00:00Z");
    }

    #[test]
    fn now_plus_seconds_advances_time() {
        let base = now_iso_utc();
        let future = now_plus_seconds(3600);
        // Future must be strictly greater than base.
        assert!(future.as_bytes() > base.as_bytes());
    }
}
