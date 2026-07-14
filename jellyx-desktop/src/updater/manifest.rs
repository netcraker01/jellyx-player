//! Release manifest fetch and parsing.
//!
//! Source: GitHub Releases latest release API for `netcraker01/jellyx-player`
//! (matches the repo in `tauri.conf.json` identifier conventions).
//!
//! This avoids allowing a stale, independently hosted static manifest to hide
//! a newer validated GitHub release.

use serde::Deserialize;
use std::time::{Duration, Instant};

/// Normalized latest release info, independent of the source manifest shape.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LatestRelease {
    /// Version tag without leading `v` (e.g. `"0.2.4"`).
    pub version: String,
    /// Original tag name as published (e.g. `"v0.2.4"`).
    pub tag_name: String,
    /// Release notes / body markdown.
    pub body: Option<String>,
    /// HTML URL of the release page (opened by "Update now").
    pub release_url: String,
    /// ISO-8601 published timestamp.
    pub published_at: Option<String>,
}

/// GitHub Releases "latest release" API response shape (only the fields we use).
#[derive(Debug, Deserialize)]
struct GithubRelease {
    tag_name: String,
    #[serde(default)]
    body: Option<String>,
    #[serde(default)]
    published_at: Option<String>,
}

/// The sole production updater source. Test endpoints are accepted only by the
/// internal test seam; public fetching always uses this HTTPS GitHub endpoint.
pub const GITHUB_REPOSITORY: &str = "netcraker01/jellyx-player";
pub const GITHUB_WEB_BASE_URL: &str = "https://github.com";
const GITHUB_LATEST_URL: &str =
    "https://api.github.com/repos/netcraker01/jellyx-player/releases/latest";
const USER_AGENT: &str = concat!("Jellyx/", env!("CARGO_PKG_VERSION"));
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

/// Fetch the latest published GitHub release. Draft staging releases are excluded
/// by the API, so an updater never points users at an unvalidated release.
pub async fn fetch_latest(client: &reqwest::Client) -> Result<Option<LatestRelease>, String> {
    let started = Instant::now();
    let result = fetch_github_latest_at(client, GITHUB_LATEST_URL, REQUEST_TIMEOUT).await;
    crate::observability::record_latency(
        "updater",
        "latest_release_fetch",
        started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64,
    );
    crate::observability::record_operation("updater", "latest_release_fetch", result.is_ok());
    result
}

async fn fetch_github_latest_at(
    client: &reqwest::Client,
    endpoint: &str,
    timeout: Duration,
) -> Result<Option<LatestRelease>, String> {
    let resp = client
        .get(endpoint)
        .header("User-Agent", USER_AGENT)
        .header("Accept", "application/vnd.github+json")
        .timeout(timeout)
        .send()
        .await
        .map_err(|e| format!("github api: request: {}", e))?;

    if resp.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }

    if !resp.status().is_success() {
        return Err(format!("github api: status {}", resp.status()));
    }

    let release: GithubRelease = resp
        .json()
        .await
        .map_err(|e| format!("github api: decode: {}", e))?;

    let release_url = default_release_url(&release.tag_name);
    Ok(Some(LatestRelease {
        version: strip_v(&release.tag_name),
        tag_name: release.tag_name,
        body: release.body,
        release_url,
        published_at: release.published_at,
    }))
}

fn strip_v(s: &str) -> String {
    s.trim().trim_start_matches('v').to_string()
}

/// Default release page URL used when a static manifest omits `release_url`.
/// Keeps the updater pointing at the Jellyx repository even if the CDN manifest
/// is incomplete.
pub fn default_release_url(tag_name: &str) -> String {
    let normalized = if tag_name.starts_with('v') {
        tag_name.to_string()
    } else {
        format!("v{}", tag_name)
    };
    format!(
        "{GITHUB_WEB_BASE_URL}/{GITHUB_REPOSITORY}/releases/tag/{}",
        normalized
    )
}

#[cfg(test)]
mod tests;
