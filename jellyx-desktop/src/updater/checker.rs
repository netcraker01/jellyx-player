//! Update checker — semver comparison, suppression logic, and `UpdateInfo` DTO.
//!
//! The `UpdateInfo` DTO is the shape returned to the frontend via the
//! `check_for_updates` command. It mirrors what the modal needs to render:
//! current vs latest version, release notes, release URL, channel, and policy.

use serde::{Deserialize, Serialize};

use crate::updater::channel::{ChannelPolicy, InstallChannel};
use crate::updater::manifest::LatestRelease;
use crate::updater::prefs::UpdatePrefs;

/// Information about an available update, returned to the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub current_version: String,
    pub latest_version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    pub release_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub published_at: Option<String>,
    /// Detected install channel (kebab-case string, e.g. `"linux-deb"`).
    pub channel: String,
    /// Phase 1 policy: `"notify_only"` or `"open_release_page"`.
    pub policy: ChannelPolicy,
    /// Whether the latest version is strictly newer than the current one.
    pub is_newer: bool,
}

/// Parse a version string into a `semver::Version`, tolerating a leading `v`
/// and pre-release suffixes. Returns `None` when the string cannot be parsed
/// (rather than panicking) so the checker can degrade gracefully.
pub fn parse_version(s: &str) -> Option<semver::Version> {
    let trimmed = s.trim().trim_start_matches('v');
    semver::Version::parse(trimmed).ok()
}

/// Returns true when `latest` is strictly greater than `current` (semver).
/// Falls back to string comparison when either side fails to parse so a
/// non-semver tag (e.g. a date-based nightly) still surfaces as an update.
pub fn is_newer(current: &str, latest: &str) -> bool {
    match (parse_version(current), parse_version(latest)) {
        (Some(c), Some(l)) => l > c,
        _ => latest.trim().trim_start_matches('v') != current.trim().trim_start_matches('v'),
    }
}

/// Decide whether the modal should be shown for `latest` given persisted prefs.
///
/// Suppression rules (per spec "Update Notification Modal"):
/// 1. `latest == skipped_version` => suppress
/// 2. `remind_later_at` is in the future => suppress
/// Otherwise show.
pub fn should_show(latest: &LatestRelease, prefs: &UpdatePrefs, now_iso: &str) -> bool {
    if prefs.is_skipped(&latest.version) {
        return false;
    }
    if prefs.is_reminded_later(now_iso) {
        return false;
    }
    true
}

/// Build an [`UpdateInfo`] from the latest release + detected channel + current version.
pub fn build_update_info(
    current_version: &str,
    latest: &LatestRelease,
    channel: InstallChannel,
) -> UpdateInfo {
    UpdateInfo {
        current_version: current_version.to_string(),
        latest_version: latest.version.clone(),
        body: latest.body.clone(),
        release_url: latest.release_url.clone(),
        published_at: latest.published_at.clone(),
        channel: channel.as_str().to_string(),
        policy: channel.phase1_policy(),
        is_newer: is_newer(current_version, &latest.version),
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
    fn parse_version_strips_v_prefix() {
        assert_eq!(parse_version("v0.2.3"), Some(semver::Version::new(0, 2, 3)));
        assert_eq!(parse_version("0.2.3"), Some(semver::Version::new(0, 2, 3)));
        assert!(parse_version("not-a-version").is_none());
    }

    #[test]
    fn is_newer_strict_semver() {
        assert!(is_newer("0.2.3", "0.2.4"));
        assert!(is_newer("v0.2.3", "v0.3.0"));
        assert!(is_newer("1.0.0", "2.0.0"));
        assert!(!is_newer("0.2.3", "0.2.3"));
        assert!(!is_newer("0.2.4", "0.2.3"));
    }

    #[test]
    fn is_newer_falls_back_to_string_when_unparseable() {
        // Non-semver tags still surface as different (an update) but equal
        // tags don't.
        assert!(is_newer("nightly-2026-07-01", "nightly-2026-07-07"));
        assert!(!is_newer("nightly-2026-07-07", "nightly-2026-07-07"));
    }

    #[test]
    fn should_show_when_no_prefs() {
        let prefs = UpdatePrefs::default();
        assert!(should_show(
            &release("0.2.4"),
            &prefs,
            "2026-07-07T00:00:00Z"
        ));
    }

    #[test]
    fn should_not_show_when_version_skipped() {
        let prefs = UpdatePrefs {
            skipped_version: Some("v0.2.4".to_string()),
            ..Default::default()
        };
        assert!(!should_show(
            &release("0.2.4"),
            &prefs,
            "2026-07-07T00:00:00Z"
        ));
        // A newer version resurfaces the modal even though 0.2.4 was skipped.
        assert!(should_show(
            &release("0.2.5"),
            &prefs,
            "2026-07-07T00:00:00Z"
        ));
    }

    #[test]
    fn should_not_show_when_remind_later_in_future() {
        let prefs = UpdatePrefs {
            remind_later_at: Some("2026-07-10T00:00:00Z".to_string()),
            ..Default::default()
        };
        assert!(!should_show(
            &release("0.2.4"),
            &prefs,
            "2026-07-07T00:00:00Z"
        ));
        // After the remind-later timestamp passes, the modal shows again.
        assert!(should_show(
            &release("0.2.4"),
            &prefs,
            "2026-07-11T00:00:00Z"
        ));
    }

    #[test]
    fn build_update_info_carries_all_fields() {
        let info = build_update_info("0.2.3", &release("0.2.4"), InstallChannel::LinuxDeb);
        assert_eq!(info.current_version, "0.2.3");
        assert_eq!(info.latest_version, "0.2.4");
        assert_eq!(info.channel, "linux-deb");
        assert_eq!(info.policy, ChannelPolicy::OpenReleasePage);
        assert!(info.is_newer);
        assert_eq!(info.release_url, "https://example.com/v0.2.4");
    }

    #[test]
    fn update_info_serializes_camel_case() {
        let info = build_update_info("0.2.3", &release("0.2.4"), InstallChannel::LinuxDeb);
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("\"currentVersion\""));
        assert!(json.contains("\"latestVersion\""));
        assert!(json.contains("\"releaseUrl\""));
        assert!(json.contains("\"publishedAt\""));
        assert!(json.contains("\"isNewer\""));
        assert!(!json.contains("\"current_version\""));
    }
}
