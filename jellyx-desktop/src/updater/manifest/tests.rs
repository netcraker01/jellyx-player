//! Tests for the release manifest fetcher.
//!
//! These tests verify that the updater always points at the Jellyx repository and
//! uses the Jellyx User-Agent, even when a static manifest falls back to a
//! default release URL.

use super::*;

#[test]
fn github_latest_url_points_to_jellyx_repo() {
    assert_eq!(
        GITHUB_LATEST_URL,
        "https://api.github.com/repos/netcraker01/jellyx-player/releases/latest"
    );
}

#[test]
fn user_agent_uses_jellyx_product_name() {
    let expected = format!("Jellyx/{}", env!("CARGO_PKG_VERSION"));
    assert_eq!(USER_AGENT, expected);
}

#[test]
fn static_manifest_fallback_url_points_to_jellyx_repo() {
    // The fallback release URL generated when a static manifest omits
    // `release_url` must target the Jellyx repo so old installs that reach this
    // code path still end up on the new repository.
    let version = env!("CARGO_PKG_VERSION");
    let expected = format!(
        "https://github.com/netcraker01/jellyx-player/releases/tag/v{}",
        version
    );
    assert_eq!(default_release_url(&format!("v{}", version)), expected);
}

#[test]
fn static_manifest_fallback_url_normalizes_tag_without_leading_v() {
    // Some manifests may store the version without the leading `v`.
    assert_eq!(
        default_release_url("0.3.3"),
        "https://github.com/netcraker01/jellyx-player/releases/tag/v0.3.3"
    );
}
