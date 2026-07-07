//! `UpdateService` — orchestrates channel detection, prefs, and the update check.
//!
//! On startup (or when invoked by the periodic timer) the service:
//! 1. Resolves the install channel (build-time env + runtime heuristics).
//! 2. Persists the detected channel if it differs from the stored one.
//! 3. Reads persisted prefs (skip / remind-later / last-check).
//! 4. Fetches the latest release manifest.
//! 5. Applies suppression rules and decides whether to surface an update.
//!
//! The service does NOT emit Tauri events directly. It returns an
//! `Option<UpdateInfo>` from `check` and lets the caller (commands / setup)
//! decide whether to emit an event or respond synchronously. This keeps the
//! service testable without a Tauri app handle.

use std::sync::Arc;

use crate::errors::types::PersistenceError;
use crate::updater::channel::{install_channel, InstallChannel};
use crate::updater::checker::{build_update_info, is_newer, should_show, UpdateInfo};
use crate::updater::manifest::fetch_latest;
use crate::updater::prefs::{now_iso_utc, UpdatePrefs, UpdatePrefsService};

/// Default periodic re-check interval: 24 hours (in seconds).
pub const DEFAULT_CHECK_INTERVAL_SECS: u64 = 24 * 60 * 60;

/// High-level updater service.
pub struct UpdateService {
    prefs: UpdatePrefsService,
    channel: InstallChannel,
    current_version: String,
    http_client: reqwest::Client,
}

impl UpdateService {
    /// Construct a new service bound to the given database.
    pub fn new(
        db: Arc<crate::persistence::db::Database>,
        exe_path: Option<&std::path::Path>,
        current_version: &str,
        http_client: reqwest::Client,
    ) -> Self {
        let channel = install_channel(exe_path);
        let prefs = UpdatePrefsService::new(db);
        Self {
            prefs,
            channel,
            current_version: current_version.to_string(),
            http_client,
        }
    }

    /// The detected install channel for this running binary.
    pub fn channel(&self) -> InstallChannel {
        self.channel
    }

    /// The persisted prefs (read-only convenience).
    pub fn prefs(&self) -> Result<UpdatePrefs, PersistenceError> {
        self.prefs.get()
    }

    /// Persist the detected channel.
    pub fn persist_detected_channel(&self) -> Result<UpdatePrefs, PersistenceError> {
        self.prefs.set_detected_channel(self.channel)
    }

    /// Persist a skipped version.
    pub fn skip_version(&self, version: &str) -> Result<UpdatePrefs, PersistenceError> {
        self.prefs.set_skipped_version(version)
    }

    /// Persist a remind-later timestamp.
    pub fn remind_later(&self, iso_ts: &str) -> Result<UpdatePrefs, PersistenceError> {
        self.prefs.set_remind_later(iso_ts)
    }

    /// Run a single check (async) and return update info.
    pub async fn check(&self) -> Result<Option<UpdateInfo>, String> {
        let latest = match fetch_latest(&self.http_client).await? {
            Some(l) => l,
            None => return Ok(None),
        };

        if let Err(e) = self.prefs.set_last_check(&now_iso_utc()) {
            eprintln!("update check: failed to persist last_check_at: {:?}", e);
        }

        if !is_newer(&self.current_version, &latest.version) {
            return Ok(None);
        }

        let prefs = self.prefs.get().unwrap_or_default();
        if !should_show(&latest, &prefs, &now_iso_utc()) {
            return Ok(None);
        }

        Ok(Some(build_update_info(
            &self.current_version,
            &latest,
            self.channel,
        )))
    }

    /// Run a check without applying suppression rules (async).
    pub async fn check_unsuppressed(&self) -> Result<Option<UpdateInfo>, String> {
        let latest = match fetch_latest(&self.http_client).await? {
            Some(l) => l,
            None => return Ok(None),
        };

        if let Err(e) = self.prefs.set_last_check(&now_iso_utc()) {
            eprintln!("update check (manual): failed to persist last_check_at: {:?}", e);
        }
        if !is_newer(&self.current_version, &latest.version) {
            return Ok(None);
        }
        Ok(Some(build_update_info(
            &self.current_version,
            &latest,
            self.channel,
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::updater::manifest::LatestRelease;

    fn release(version: &str) -> LatestRelease {
        LatestRelease {
            version: version.to_string(),
            tag_name: format!("v{}", version),
            body: Some("notes".to_string()),
            release_url: format!("https://example.com/v{}", version),
            published_at: Some("2026-07-07T10:00:00Z".to_string()),
        }
    }

    #[test]
    fn build_info_from_release_carries_channel_and_policy() {
        let info = build_update_info("0.2.3", &release("0.2.4"), InstallChannel::LinuxDeb);
        assert_eq!(info.channel, "linux-deb");
        assert!(info.is_newer);
        assert_eq!(info.policy, InstallChannel::LinuxDeb.phase1_policy());
    }

    #[test]
    fn service_construction_persists_no_state() {
        let db = Arc::new(crate::persistence::db::Database::open_in_memory().unwrap());
        let http_client = reqwest::Client::new();
        let svc = UpdateService::new(db.clone(), None, "0.2.3", http_client);
        let _ = svc.channel();
        let prefs = svc.prefs().unwrap();
        assert_eq!(prefs, UpdatePrefs::default());
    }
}
