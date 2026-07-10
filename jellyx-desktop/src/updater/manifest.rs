//! Release manifest fetch and parsing.
//!
//! Long-term preferred source: a static `latest.json` file hosted on a CDN.
//! Phase 1 fallback: GitHub Releases latest release API for `netcraker01/helix`
//! (matches the repo in `tauri.conf.json` identifier conventions).
//!
//! Both shapes are normalized into [`LatestRelease`]. The fetcher tries the
//! static manifest first and falls back to the GitHub API on any error.

use serde::Deserialize;

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
    html_url: String,
    #[serde(default)]
    published_at: Option<String>,
}

/// Static `latest.json` manifest shape (intentionally similar to GitHub).
#[derive(Debug, Deserialize)]
struct StaticManifest {
    version: String,
    #[serde(default)]
    tag_name: Option<String>,
    #[serde(default)]
    body: Option<String>,
    #[serde(default)]
    release_url: Option<String>,
    #[serde(default)]
    published_at: Option<String>,
}

const GITHUB_LATEST_URL: &str =
    "https://api.github.com/repos/netcraker01/helix/releases/latest";
const STATIC_MANIFEST_URL: &str =
    "https://releases.helix.dev/latest.json";
const USER_AGENT: &str = concat!("Helix/", env!("CARGO_PKG_VERSION"));

/// Fetch the latest release info asynchronously, trying the static manifest first and
/// falling back to the GitHub Releases API.
pub async fn fetch_latest(client: &reqwest::Client) -> Result<Option<LatestRelease>, String> {
    if let Ok(rel) = fetch_static_manifest(client).await {
        return Ok(Some(rel));
    }
    fetch_github_latest(client).await
}

async fn fetch_static_manifest(client: &reqwest::Client) -> Result<LatestRelease, String> {
    let resp = client
        .get(STATIC_MANIFEST_URL)
        .header("User-Agent", USER_AGENT)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .map_err(|e| format!("static manifest: request: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("static manifest: status {}", resp.status()));
    }
    
    let manifest: StaticManifest = resp
        .json()
        .await
        .map_err(|e| format!("static manifest: decode: {}", e))?;

    let tag_name = manifest
        .tag_name
        .clone()
        .unwrap_or_else(|| format!("v{}", manifest.version));

    let release_url = manifest.release_url.unwrap_or_else(|| {
        format!("https://github.com/netcraker01/helix/releases/tag/{}", tag_name)
    });

    Ok(LatestRelease {
        version: strip_v(&manifest.version),
        tag_name,
        body: manifest.body,
        release_url,
        published_at: manifest.published_at,
    })
}

async fn fetch_github_latest(client: &reqwest::Client) -> Result<Option<LatestRelease>, String> {
    let resp = client
        .get(GITHUB_LATEST_URL)
        .header("User-Agent", USER_AGENT)
        .header("Accept", "application/vnd.github+json")
        .timeout(std::time::Duration::from_secs(10))
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

    Ok(Some(LatestRelease {
        version: strip_v(&release.tag_name),
        tag_name: release.tag_name,
        body: release.body,
        release_url: release.html_url,
        published_at: release.published_at,
    }))
}

fn strip_v(s: &str) -> String {
    s.trim().trim_start_matches('v').to_string()
}
