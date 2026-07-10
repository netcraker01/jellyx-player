//! Tests for Tauri command handlers.
//!
//! These tests focus on pure, side-effect-free validation logic that does not
//! require a running Tauri runtime.

use super::*;

#[test]
fn open_release_page_accepts_jellyx_repo_url() {
    let url = "https://github.com/netcraker01/jellyx-player/releases/tag/v0.3.3";
    assert!(
        is_release_url_allowed(url),
        "expected Jellyx release URL to be allowed: {}",
        url
    );
}

#[test]
fn open_release_page_accepts_legacy_helix_repo_url() {
    let url = "https://github.com/netcraker01/helix/releases/tag/v0.3.3";
    assert!(
        is_release_url_allowed(url),
        "expected legacy Helix release URL to be allowed (GitHub redirects): {}",
        url
    );
}

#[test]
fn open_release_page_rejects_non_github_url() {
    let url = "https://example.com/evil";
    assert!(!is_release_url_allowed(url));
}

#[test]
fn open_release_page_rejects_malformed_github_path() {
    let url = "https://github.com/netcraker01/jellyx-player/issues/1";
    assert!(!is_release_url_allowed(url));
}
