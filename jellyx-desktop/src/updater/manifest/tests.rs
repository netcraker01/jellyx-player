//! Tests for the release manifest fetcher.
//!
//! These tests verify that the updater always points at the Jellyx repository
//! and does not depend on a separately hosted static manifest.

use super::*;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;
use std::time::Duration;

fn controlled_server(status: &str, body: &'static str, delay: Duration) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let status = status.to_owned();
    thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut request = [0_u8; 1024];
        let _ = stream.read(&mut request);
        thread::sleep(delay);
        write!(
            stream,
            "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len(),
        ).unwrap();
    });
    format!("http://{address}/releases/latest")
}

fn fetch_from(endpoint: &str, timeout: Duration) -> Result<Option<LatestRelease>, String> {
    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(fetch_github_latest_at(
            &reqwest::Client::new(),
            endpoint,
            timeout,
        ))
}

#[test]
fn github_latest_url_points_to_jellyx_repo() {
    assert_eq!(GITHUB_REPOSITORY, "netcraker01/jellyx-player");
    assert_eq!(
        GITHUB_LATEST_URL,
        "https://api.github.com/repos/netcraker01/jellyx-player/releases/latest"
    );
}

#[test]
fn controlled_github_fetch_parses_a_successful_release() {
    let endpoint = controlled_server(
        "200 OK",
        r#"{"tag_name":"v0.4.2","body":"fixes","html_url":"https://github.com/netcraker01/jellyx-player/releases/tag/v0.4.2","published_at":"2026-07-13T00:00:00Z"}"#,
        Duration::ZERO,
    );
    let release = fetch_from(&endpoint, Duration::from_secs(1))
        .unwrap()
        .unwrap();
    assert_eq!(release.version, "0.4.2");
    assert_eq!(release.body.as_deref(), Some("fixes"));
}

#[test]
fn controlled_github_fetch_treats_404_as_no_published_release() {
    let endpoint = controlled_server("404 Not Found", "{}", Duration::ZERO);
    assert_eq!(fetch_from(&endpoint, Duration::from_secs(1)).unwrap(), None);
}

#[test]
fn controlled_github_fetch_rejects_malformed_json_without_fallback() {
    let endpoint = controlled_server("200 OK", "not json", Duration::ZERO);
    let error = fetch_from(&endpoint, Duration::from_secs(1)).unwrap_err();
    assert!(error.starts_with("github api: decode:"));
}

#[test]
fn controlled_github_fetch_returns_status_error_without_fallback() {
    let endpoint = controlled_server("500 Internal Server Error", "{}", Duration::ZERO);
    let error = fetch_from(&endpoint, Duration::from_secs(1)).unwrap_err();
    assert_eq!(error, "github api: status 500 Internal Server Error");
}

#[test]
fn controlled_github_fetch_returns_request_error_on_timeout_without_fallback() {
    let endpoint = controlled_server("200 OK", "{}", Duration::from_millis(100));
    let error = fetch_from(&endpoint, Duration::from_millis(10)).unwrap_err();
    assert!(error.starts_with("github api: request:"));
}

#[test]
fn user_agent_uses_jellyx_product_name() {
    let expected = format!("Jellyx/{}", env!("CARGO_PKG_VERSION"));
    assert_eq!(USER_AGENT, expected);
}

#[test]
fn release_url_normalizes_tag_without_leading_v() {
    assert_eq!(
        default_release_url("0.3.3"),
        "https://github.com/netcraker01/jellyx-player/releases/tag/v0.3.3"
    );
}
