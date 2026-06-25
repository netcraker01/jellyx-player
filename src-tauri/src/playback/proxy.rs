//! Local stream proxy for remote media URLs.
//!
//! Bridges the WebView/Web browser audio stack to remote stream URLs that
//! require CORS headers or have other access restrictions.
//!
//! Design (inspired by Nuclear):
//! - Starts a lightweight HTTP server on a local port (e.g. 8765)
//! - Accepts proxy requests like: /proxy?url=<encoded_remote_url>
//! - Forwards the request to the remote URL with necessary headers
//! - Adds CORS headers to the response for WebView compatibility
//! - Supports Range requests for audio seeking
//!
//! This avoids forcing YouTube streams through the Rust Symphonia/cpal pipeline.

use percent_encoding::{percent_decode_str, NON_ALPHANUMERIC, percent_encode};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, atomic::{AtomicBool, Ordering}, OnceLock};
use std::thread;
use std::time::Duration;

/// Default port for the local stream proxy.
const PROXY_PORT: u16 = 8765;

/// Shared HTTP client — avoids creating a new reqwest client (with TLS context)
/// on every proxy request, which is especially costly during seeks where the
/// browser opens a new connection for each Range request.
static HTTP_CLIENT: OnceLock<reqwest::blocking::Client> = OnceLock::new();

fn http_client() -> &'static reqwest::blocking::Client {
    HTTP_CLIENT.get_or_init(|| {
        reqwest::blocking::Client::builder()
            .connect_timeout(Duration::from_secs(15))
            .timeout(Duration::from_secs(300))
            .no_proxy()
            // Force HTTP/1.1 — reqwest's blocking Read trait only reads the first
            // ~32KB of HTTP/2 responses then reports EOF. HTTP/1.1 works correctly.
            .http1_only()
            .pool_idle_timeout(Duration::from_secs(60))
            .pool_max_idle_per_host(4)
            .build()
            .expect("failed to build proxy HTTP client")
    })
}

/// Start a simple HTTP proxy server on a local port.
///
/// Handles GET /proxy?url=<url> by forwarding to the remote URL
/// and returning the response with CORS headers.
///
/// Returns the bound port number (in case the default was in use).
pub fn start_proxy_server() -> Result<u16, ProxyError> {
    let listener = TcpListener::bind(format!("127.0.0.1:{}", PROXY_PORT))
        .or_else(|_| {
            // Try any available port if default is taken
            TcpListener::bind("127.0.0.1:0")
        })
        .map_err(|e| ProxyError::BindFailed(e.to_string()))?;

    let port = listener.local_addr()
        .map_err(|e| ProxyError::BindFailed(e.to_string()))?
        .port();

    let running = Arc::new(AtomicBool::new(true));
    let running_clone = running.clone();

    thread::spawn(move || {
        listener.set_nonblocking(true).ok();

        for stream in listener.incoming() {
            if !running_clone.load(Ordering::Relaxed) {
                break;
            }

            match stream {
                Ok(stream) => {
                    // Spawn a thread per connection so slow upstream responses
                    // don't block other requests (e.g. seek opens a new connection
                    // while the previous one is still streaming).
                    thread::spawn(move || {
                        handle_proxy_request(stream);
                    });
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(10));
                    continue;
                }
                Err(e) => {
                    eprintln!("[Proxy] Accept error: {}", e);
                }
            }
        }
    });

    Ok(port)
}

/// Construct a proxied URL that routes through the local proxy server.
///
/// Takes the raw remote stream URL and returns a local URL like:
/// `http://127.0.0.1:8765/proxy?url=<encoded_remote_url>`
pub fn proxied_url(port: u16, remote_url: &str) -> String {
    let encoded = percent_encode(remote_url.as_bytes(), &NON_ALPHANUMERIC).to_string();
    format!("http://127.0.0.1:{}/proxy?url={}", port, encoded)
}

/// Proxy errors.
#[derive(Debug)]
pub enum ProxyError {
    BindFailed(String),
    RequestFailed(String),
}

/// Handle a single HTTP request on the proxy server.
///
/// Parses the request, extracts the `url` query parameter,
/// fetches the remote content, and returns it with CORS headers.
fn handle_proxy_request(mut stream: TcpStream) {
    let mut buffer = [0u8; 4096];
    let bytes_read = match stream.read(&mut buffer) {
        Ok(0) => return, // Connection closed
        Ok(n) => n,
        Err(e) => {
            eprintln!("[Proxy] Read error: {}", e);
            return;
        }
    };

    let request = String::from_utf8_lossy(&buffer[..bytes_read]);

    // Parse the request line
    let request_line = request.lines().next().unwrap_or("");
    let parts: Vec<&str> = request_line.split_whitespace().collect();

    if parts.len() < 2 || parts[0] != "GET" {
        send_response(&mut stream, 400, "Bad Request", b"Only GET /proxy?url=... is supported");
        return;
    }

    let path = parts[1];

    // Extract URL from query string: /proxy?url=...&seekto=<seconds>&duration=<seconds>
    if let Some(url_start) = path.find("?url=") {
        let query = &path[url_start + 5..];
        
        // Split URL and optional seekto/duration parameters
        let (encoded_url, seekto, duration) = {
            let mut url_part = query;
            let mut seek_val: Option<f64> = None;
            let mut dur_val: Option<f64> = None;
            
            if let Some(pos) = query.find("&seekto=") {
                url_part = &query[..pos];
                let rest = &query[pos + 8..];
                let end = rest.find('&').unwrap_or(rest.len());
                seek_val = rest[..end].parse::<f64>().ok();
            }
            if let Some(pos) = query.find("&duration=") {
                let rest = &query[pos + 10..];
                let end = rest.find('&').unwrap_or(rest.len());
                dur_val = rest[..end].parse::<f64>().ok();
            }
            (url_part, seek_val, dur_val)
        };

        let decoded_url = match percent_decode_str(encoded_url).decode_utf8() {
            Ok(url) => url.to_string(),
            Err(e) => {
                eprintln!("[Proxy] URL decode error: {}", e);
                send_response(&mut stream, 400, "Bad Request", b"Invalid URL encoding");
                return;
            }
        };

        // Forward the request to the remote URL
        if let Err(e) = forward_request(&decoded_url, &request, &mut stream, seekto, duration) {
            eprintln!("[Proxy] Forward error for {}: {}", decoded_url, e);
            send_response(&mut stream, 502, "Bad Gateway", format!("Proxy error: {}", e).as_bytes());
        }
    } else {
        send_response(&mut stream, 400, "Bad Request", b"Missing url parameter");
    }
}

/// Forward an HTTP request to the remote URL.
///
/// Uses reqwest::blocking to fetch the remote content and streams
/// the response body in chunks to the client. Response headers are
/// forwarded immediately so the browser can start buffering audio
/// progressively.
fn forward_request(url: &str, original_request: &str, stream: &mut TcpStream, seekto: Option<f64>, duration: Option<f64>) -> Result<(), String> {
    // Extract Range header from original request if present
    let range_header = original_request
        .lines()
        .find(|line| line.to_lowercase().starts_with("range:"))
        .map(|line| line.strip_prefix("Range:").or_else(|| line.strip_prefix("range:")).unwrap_or("").trim());

    let client = http_client();

    let mut request = client.get(url);

    // If seekto and duration are provided, calculate byte offset and serve
    // a Range response from that position. The browser will make its own
    // Range requests for the init segment (first bytes) before requesting
    // the seek position, so we just need to handle the Range header properly.
    if let (Some(seek_seconds), Some(dur)) = (seekto, duration) {
        if dur > 0.0 && seek_seconds < dur {
            // Get total content size
            let head_resp = client.get(url)
                .header("User-Agent", "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36")
                .header("Accept-Encoding", "identity")
                .header("Range", "bytes=0-0")
                .send()
                .map_err(|e| format!("HEAD request failed: {}", e))?;

            let total_size = head_resp.headers()
                .get("content-range")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.split('/').nth(1))
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(0);
            drop(head_resp);

            if total_size > 0 {
                let ratio = seek_seconds / dur;
                let byte_offset = (ratio * total_size as f64) as u64;
                eprintln!("[Proxy] Seek: {:.1}s / {:.1}s = ratio {:.3} -> byte {} / {}", seek_seconds, dur, ratio, byte_offset, total_size);
                // Use Range from the calculated byte offset
                request = request.header("Range", format!("bytes={}-", byte_offset));
            }
        }
    } else if let Some(range) = range_header {
        request = request.header("Range", range);
    }

    request = request.header("User-Agent", "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36");
    request = request.header("Accept-Encoding", "identity");

    let mut response = request.send().map_err(|e| format!("Request failed: {}", e))?;
    let status = response.status();
    let headers = response.headers().clone();
    let content_length = response.content_length();

    // Build HTTP response with CORS headers
    let mut response_text = format!(
        "HTTP/1.1 {} {}\r\n",
        status.as_u16(),
        status.canonical_reason().unwrap_or("OK")
    );

    response_text.push_str("Access-Control-Allow-Origin: *\r\n");
    response_text.push_str("Access-Control-Allow-Methods: GET, HEAD, OPTIONS\r\n");
    response_text.push_str("Access-Control-Allow-Headers: Range, Content-Type\r\n");
    response_text.push_str("Access-Control-Expose-Headers: Content-Range, Content-Length\r\n");

    if let Some(content_type) = headers.get("content-type") {
        if let Ok(ct) = content_type.to_str() {
            response_text.push_str(&format!("Content-Type: {}\r\n", ct));
        }
    }

    if let Some(cl) = content_length {
        response_text.push_str(&format!("Content-Length: {}\r\n", cl));
    }

    if let Some(content_range) = headers.get("content-range") {
        if let Ok(cr) = content_range.to_str() {
            response_text.push_str(&format!("Content-Range: {}\r\n", cr));
        }
    }

    response_text.push_str("Accept-Ranges: bytes\r\n");
    response_text.push_str("Connection: close\r\n");
    response_text.push_str("\r\n");

    stream.write_all(response_text.as_bytes())
        .map_err(|e| format!("Failed to write response headers: {}", e))?;

    // Stream body to the client. The browser may close the connection early
    // (BrokenPipe) after receiving enough data for buffering, then open a new
    // one with a Range request for seeking — this is normal and not an error.
    let mut buf = [0u8; 32768];
    let mut total_written: u64 = 0;
    loop {
        match response.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                if let Err(e) = stream.write_all(&buf[..n]) {
                    if e.kind() != std::io::ErrorKind::BrokenPipe
                        && e.kind() != std::io::ErrorKind::ConnectionReset
                    {
                        eprintln!("[Proxy] Write error after {} bytes: {}", total_written, e);
                    }
                    break;
                }
                let _ = stream.flush();
                total_written += n as u64;
            }
            Err(_) => break,
        }
    }

    Ok(())
}

/// Send a simple HTTP response.
fn send_response(stream: &mut TcpStream, status: u16, status_text: &str, body: &[u8]) {
    let response = format!(
        "HTTP/1.1 {} {}\r\n\
         Content-Type: text/plain\r\n\
         Content-Length: {}\r\n\
         Access-Control-Allow-Origin: *\r\n\
         Connection: close\r\n\
         \r\n",
        status,
        status_text,
        body.len()
    );

    let _ = stream.write_all(response.as_bytes());
    let _ = stream.write_all(body);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;

    #[test]
    fn proxied_url_encoding() {
        let url = proxied_url(8765, "https://example.com/stream?foo=bar");
        assert!(url.starts_with("http://127.0.0.1:8765/proxy?url="));
        // Should contain encoded characters
        assert!(url.contains("%3A") || url.contains("https"), "URL should be encoded or contain https");
    }

    #[test]
    fn proxied_url_special_chars() {
        let remote = "https://test.com/path?a=1&b=2";
        let url = proxied_url(8765, remote);
        assert!(url.starts_with("http://127.0.0.1:8765/proxy?url="));
        // The encoded URL should round-trip correctly
        let encoded = &url["http://127.0.0.1:8765/proxy?url=".len()..];
        let decoded = percent_decode_str(encoded).decode_utf8().unwrap();
        assert_eq!(decoded, remote);
    }

    #[test]
    fn proxy_error_display() {
        let err = ProxyError::BindFailed("port in use".to_string());
        let msg = format!("{:?}", err);
        assert!(msg.contains("port in use"));
    }

    #[test]
    fn proxy_streams_upstream_response() {
        // Start a tiny upstream HTTP server
        let upstream = TcpListener::bind("127.0.0.1:0").unwrap();
        let upstream_port = upstream.local_addr().unwrap().port();

        thread::spawn(move || {
            let (mut stream, _) = upstream.accept().unwrap();
            let mut buf = [0u8; 4096];
            let n = stream.read(&mut buf).unwrap();
            let req = String::from_utf8_lossy(&buf[..n]);

            // Verify proxy forwarded User-Agent (case-insensitive) — if not, return 400 so test fails clearly
            let has_ua = req.to_lowercase().contains("mozilla/5.0");
            let body = "Hello from upstream";
            let (status, response_body) = if has_ua {
                ("200 OK", body)
            } else {
                ("400 Bad Request", "Missing User-Agent")
            };
            let response = format!(
                "HTTP/1.1 {}\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}",
                status,
                response_body.len(),
                response_body
            );
            let _ = stream.write_all(response.as_bytes());
        });

        // Start proxy server
        let proxy_port = start_proxy_server().unwrap();

        // Give the proxy thread a moment to start accepting
        thread::sleep(Duration::from_millis(50));

        // Build proxy URL
        let remote_url = format!("http://127.0.0.1:{}/test", upstream_port);
        let proxy_url = proxied_url(proxy_port, &remote_url);

        // Request through proxy using reqwest
        let client = reqwest::blocking::Client::new();
        let response = client.get(&proxy_url).send().unwrap();

        assert_eq!(response.status(), 200, "Proxy did not forward User-Agent or upstream failed");
        // Check headers before consuming body
        let has_cors = response.headers().get("access-control-allow-origin").is_some();
        let body = response.text().unwrap();
        assert_eq!(body, "Hello from upstream");

        // Verify CORS headers were injected
        assert!(has_cors);
    }

    #[test]
    fn proxy_forwards_range_request_and_response() {
        // Start a tiny upstream HTTP server that validates Range
        let upstream = TcpListener::bind("127.0.0.1:0").unwrap();
        let upstream_port = upstream.local_addr().unwrap().port();

        thread::spawn(move || {
            let (mut stream, _) = upstream.accept().unwrap();
            let mut buf = [0u8; 4096];
            let n = stream.read(&mut buf).unwrap();
            let req = String::from_utf8_lossy(&buf[..n]);

            // Verify proxy forwarded the Range header (case-insensitive)
            let has_range = req.to_lowercase().contains("range: bytes=0-4");
            let (status, body, extra_headers) = if has_range {
                (
                    "206 Partial Content",
                    "Hello",
                    "Content-Range: bytes 0-4/11\r\n"
                )
            } else {
                (
                    "400 Bad Request",
                    "Missing Range header",
                    ""
                )
            };
            let response = format!(
                "HTTP/1.1 {}\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n{}\r\n{}",
                status,
                body.len(),
                extra_headers,
                body
            );
            let _ = stream.write_all(response.as_bytes());
        });

        let proxy_port = start_proxy_server().unwrap();
        thread::sleep(Duration::from_millis(50));

        let remote_url = format!("http://127.0.0.1:{}/audio.mp3", upstream_port);
        let proxy_url = proxied_url(proxy_port, &remote_url);

        let client = reqwest::blocking::Client::new();
        let response = client.get(&proxy_url).header("Range", "bytes=0-4").send().unwrap();

        assert_eq!(response.status(), 206);
        // Extract headers before consuming body
        let content_range = response.headers().get("content-range")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());
        let accept_ranges = response.headers().get("accept-ranges")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());
        let body = response.text().unwrap();
        assert_eq!(body, "Hello");

        // Verify Content-Range was forwarded
        assert_eq!(content_range.unwrap(), "bytes 0-4/11");

        // Verify Accept-Ranges was injected
        assert_eq!(accept_ranges.unwrap(), "bytes");
    }
}
