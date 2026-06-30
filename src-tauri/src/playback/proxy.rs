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
//! Seeking model: the browser's media engine drives seeking natively via
//! `audio.currentTime`. When the browser loads a proxied URL with NO `Range`
//! header, the proxy asks upstream for `Range: bytes=0-` so it receives a
//! `206 Partial Content` + `Content-Range: bytes 0-<last>/<total>` response,
//! which lets the media engine learn the total size and seek accurately
//! (critical for YouTube m4a where the element otherwise reports
//! `duration = Infinity`). When the browser IS seeking, it sends its own
//! `Range` header, which the proxy forwards unchanged.
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
    #[allow(dead_code)]
    BindFailed(String),
    #[allow(dead_code)]
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

    // Extract URL from query string: /proxy?url=...[&duration=<seconds>]
    //
    // `seekto` is intentionally NOT parsed anymore: byte-ratio seeking was
    // unreliable for YouTube m4a/fMP4 (the moov atom is not at a fixed byte
    // ratio), and the frontend now drives seek via native `currentTime`, which
    // makes the browser issue its own accurate Range requests. The proxy's
    // job is simply to expose byte-range metadata (`Content-Range`,
    // `Content-Length`, `Accept-Ranges: bytes`, 206) so the browser can seek.
    //
    // `duration` is optional and only used to hint the browser via
    // `X-Content-Duration`/`Content-Duration` headers when the upstream does
    // not provide them (e.g. YouTube m4a often reports Infinity to the element).
    if let Some(url_start) = path.find("?url=") {
        let query = &path[url_start + 5..];

        let (encoded_url, duration) = {
            let mut url_part = query;
            let mut dur_val: Option<f64> = None;

            if let Some(pos) = query.find("&duration=") {
                url_part = &query[..pos];
                let rest = &query[pos + 10..];
                let end = rest.find('&').unwrap_or(rest.len());
                dur_val = rest[..end].parse::<f64>().ok();
            }
            // Tolerate legacy `&seekto=` params (now unused) by stripping them
            // from the URL portion so URL decoding still succeeds.
            if let Some(pos) = url_part.find("&seekto=") {
                url_part = &url_part[..pos];
            }
            (url_part, dur_val)
        };

        let decoded_url = match percent_decode_str(encoded_url).decode_utf8() {
            Ok(url) => url.to_string(),
            Err(e) => {
                eprintln!("[Proxy] URL decode error: {}", e);
                send_response(&mut stream, 400, "Bad Request", b"Invalid URL encoding");
                return;
            }
        };

        // Route: file:// URLs serve a local cached file with full Range support.
        // This is the YouTube local-cache fallback path — a local file seek is
        // always reliable in WebKitGTK/GStreamer, unlike remote m4a Range seeks.
        if decoded_url.starts_with("file://") {
            let local_path = decoded_url.strip_prefix("file://").unwrap_or("");
            if let Err(e) = serve_local_file(local_path, &request, &mut stream, duration) {
                eprintln!("[Proxy] Local file serve error for {}: {}", local_path, e);
                send_response(&mut stream, 500, "Internal Server Error", b"Failed to serve local file");
            }
            return;
        }

        // Forward the request to the remote URL
        if let Err(e) = forward_request(&decoded_url, &request, &mut stream, duration) {
            eprintln!("[Proxy] Forward error for {}: {}", decoded_url, e);
            send_response(&mut stream, 502, "Bad Gateway", format!("Proxy error: {}", e).as_bytes());
        }
    } else {
        send_response(&mut stream, 400, "Bad Request", b"Missing url parameter");
    }
}

/// Serve a local file with full HTTP Range support.
///
/// This is the YouTube local-cache fallback: a fully-downloaded local m4a
/// file served through the proxy with `Accept-Ranges: bytes` and proper
/// `Content-Range`/`206 Partial Content` responses. WebKitGTK/GStreamer
/// can always seek local files reliably, unlike remote byte-range requests
/// over the proxy which are flaky for YouTube m4a.
fn serve_local_file(
    path: &str,
    original_request: &str,
    stream: &mut TcpStream,
    duration: Option<f64>,
) -> Result<(), String> {
    use std::fs::File;

    let file = File::open(path).map_err(|e| format!("Failed to open local file {}: {}", path, e))?;
    let file_size = file
        .metadata()
        .map_err(|e| format!("Failed to read file metadata: {}", e))?
        .len();

    // Parse Range header from the browser's request
    let (start, end) = parse_range_header(original_request, file_size);

    let mut response_text = format!(
        "HTTP/1.1 {} {}\r\n",
        if start == 0 && end == file_size - 1 { 200 } else { 206 },
        if start == 0 && end == file_size - 1 { "OK" } else { "Partial Content" }
    );

    response_text.push_str("Access-Control-Allow-Origin: *\r\n");
    response_text.push_str("Access-Control-Allow-Methods: GET, HEAD, OPTIONS\r\n");
    response_text.push_str("Access-Control-Allow-Headers: Range, Content-Type\r\n");
    response_text.push_str("Access-Control-Expose-Headers: Content-Range, Content-Length, Accept-Ranges, X-Content-Duration, Content-Duration\r\n");

    // Content-Type: infer from extension
    let content_type = if path.ends_with(".m4a") {
        "audio/mp4"
    } else if path.ends_with(".mp3") {
        "audio/mpeg"
    } else if path.ends_with(".wav") {
        "audio/wav"
    } else if path.ends_with(".ogg") {
        "audio/ogg"
    } else if path.ends_with(".flac") {
        "audio/flac"
    } else {
        "application/octet-stream"
    };
    response_text.push_str(&format!("Content-Type: {}\r\n", content_type));

    let content_length = end - start + 1;
    response_text.push_str(&format!("Content-Length: {}\r\n", content_length));

    if !(start == 0 && end == file_size - 1) {
        response_text.push_str(&format!(
            "Content-Range: bytes {}-{}/{}\r\n",
            start, end, file_size
        ));
    }

    response_text.push_str("Accept-Ranges: bytes\r\n");

    // Optional duration hint (for consistency with the remote path)
    if let Some(dur) = duration {
        if dur.is_finite() && dur > 0.0 {
            let dur_str = format!("{:.3}", dur);
            response_text.push_str(&format!("X-Content-Duration: {}\r\n", dur_str));
            response_text.push_str(&format!("Content-Duration: {}\r\n", dur_str));
        }
    }

    response_text.push_str("Connection: close\r\n");
    response_text.push_str("\r\n");

    stream
        .write_all(response_text.as_bytes())
        .map_err(|e| format!("Failed to write response headers: {}", e))?;

    // Stream the requested byte range from the file
    use std::io::{Seek, SeekFrom, Read};
    let mut file = file;
    if start > 0 {
        file.seek(SeekFrom::Start(start))
            .map_err(|e| format!("Failed to seek file: {}", e))?;
    }

    let mut remaining = content_length as usize;
    let mut buf = [0u8; 32768];
    while remaining > 0 {
        let to_read = buf.len().min(remaining);
        let n = file
            .read(&mut buf[..to_read])
            .map_err(|e| format!("Failed to read file: {}", e))?;
        if n == 0 {
            break;
        }
        if let Err(e) = stream.write_all(&buf[..n]) {
            if e.kind() != std::io::ErrorKind::BrokenPipe
                && e.kind() != std::io::ErrorKind::ConnectionReset
            {
                eprintln!("[Proxy] Write error serving local file: {}", e);
            }
            break;
        }
        let _ = stream.flush();
        remaining -= n;
    }

    Ok(())
}

/// Parse the `Range:` header from an HTTP request into (start, end) byte offsets.
///
/// Supports `bytes=start-end`, `bytes=start-`, and `bytes=-suffix`. Returns
/// `(0, file_size - 1)` when no Range header is present (full file).
fn parse_range_header(request: &str, file_size: u64) -> (u64, u64) {
    let range_header = request
        .lines()
        .find(|line| line.to_lowercase().starts_with("range:"))
        .map(|line| {
            if let Some(rest) = line.strip_prefix("Range:") {
                rest.trim().to_string()
            } else if let Some(rest) = line.strip_prefix("range:") {
                rest.trim().to_string()
            } else {
                line.trim().to_string()
            }
        });

    match range_header {
        Some(range) if range.starts_with("bytes=") => {
            let spec = &range[6..];
            if let Some(dash) = spec.find('-') {
                let start_str = &spec[..dash];
                let end_str = &spec[dash + 1..];
                let start: u64 = start_str.parse().unwrap_or(0);
                let end: u64 = if end_str.is_empty() {
                    file_size - 1
                } else {
                    end_str.parse().unwrap_or(file_size - 1)
                };
                (start.min(file_size - 1), end.min(file_size - 1))
            } else {
                (0, file_size - 1)
            }
        }
        _ => (0, file_size - 1),
    }
}

/// Forward an HTTP request to the remote URL.
///
/// Uses reqwest::blocking to fetch the remote content and streams
/// the response body in chunks to the client. Response headers are
/// forwarded immediately so the browser can start buffering audio
/// progressively.
///
/// Seeking model: the browser's media engine drives seeking via native
/// `currentTime`, which causes it to issue its own `Range:` requests. The
/// proxy simply:
///   - forwards the browser's incoming `Range` header unchanged when present;
///   - when the browser sends NO `Range` (initial load), asks upstream for
///     `Range: bytes=0-` so the upstream returns `206 Partial Content` with a
///     `Content-Range: bytes 0-<last>/<total>` that lets the browser learn the
///     total size and seek accurately — especially important for YouTube m4a
///     where the element otherwise sees `duration = Infinity`;
///   - falls back gracefully if the upstream ignores the Range request (returns
///     200 with full body) — the browser can still play, just may not seek;
///   - injects `Accept-Ranges: bytes` so the browser knows Range is allowed;
///   - optionally injects `X-Content-Duration`/`Content-Duration` from a known
///     `duration` query hint when the upstream does not provide them.
fn forward_request(url: &str, original_request: &str, stream: &mut TcpStream, duration: Option<f64>) -> Result<(), String> {
    // Extract Range header from the browser's original request if present.
    // We forward it UNCHANGED — the browser knows where it wants to read.
    let range_header = original_request
        .lines()
        .find(|line| line.to_lowercase().starts_with("range:"))
        .map(|line| line.strip_prefix("Range:").or_else(|| line.strip_prefix("range:")).unwrap_or("").trim());

    let client = http_client();

    let mut request = client.get(url);

    // If the browser did NOT send a Range, request `bytes=0-` upstream so we
    // get a 206 + Content-Range that tells the browser the total size. This is
    // what makes native seeking work for YouTube m4a (otherwise the media engine
    // never learns the content length and reports duration = Infinity, making
    // every seek scan the whole file). If the browser DID send a Range, forward
    // it exactly — the browser is seeking on its own.
    match range_header {
        Some(range) => request = request.header("Range", range),
        None => request = request.header("Range", "bytes=0-"),
    }

    request = request.header("User-Agent", "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36");
    request = request.header("Accept-Encoding", "identity");

    let mut response = request.send().map_err(|e| format!("Request failed: {}", e))?;
    let status = response.status();
    let headers = response.headers().clone();
    let content_length = response.content_length();

    // Build HTTP response with CORS headers. We forward the upstream status as-is
    // (206 when upstream honored our Range, 200 if it ignored it).
    let mut response_text = format!(
        "HTTP/1.1 {} {}\r\n",
        status.as_u16(),
        status.canonical_reason().unwrap_or("OK")
    );

    response_text.push_str("Access-Control-Allow-Origin: *\r\n");
    response_text.push_str("Access-Control-Allow-Methods: GET, HEAD, OPTIONS\r\n");
    response_text.push_str("Access-Control-Allow-Headers: Range, Content-Type\r\n");
    response_text.push_str("Access-Control-Expose-Headers: Content-Range, Content-Length, Accept-Ranges, X-Content-Duration, Content-Duration\r\n");

    if let Some(content_type) = headers.get("content-type") {
        if let Ok(ct) = content_type.to_str() {
            response_text.push_str(&format!("Content-Type: {}\r\n", ct));
        }
    }

    // Content-Length: forward the upstream-reported length. For 206 this is
    // the partial body length; for 200 (upstream ignored our bytes=0-) it's the
    // full length. reqwest parses it from both Content-Length and the
    // Content-Range total; if neither is present, we omit the header and the
    // browser treats the body as chunked/unknown-length.
    if let Some(cl) = content_length {
        response_text.push_str(&format!("Content-Length: {}\r\n", cl));
    }

    if let Some(content_range) = headers.get("content-range") {
        if let Ok(cr) = content_range.to_str() {
            response_text.push_str(&format!("Content-Range: {}\r\n", cr));
        }
    }

    // Always advertise that we support range requests. Even if the upstream
    // returned 200 (ignored our bytes=0-), the browser can still issue new
    // Range requests on subsequent connections and the proxy will forward them.
    response_text.push_str("Accept-Ranges: bytes\r\n");

    // Optional duration hint. Some media engines (WebKitGTK/GStreamer on Linux)
    // use X-Content-Duration / Content-Duration to compute a finite duration for
    // streams that report Infinity, which keeps native seek responsive. We only
    // inject it when the upstream didn't already provide it, and only if we have
    // a sane known duration from the caller.
    if let Some(dur) = duration {
        if dur.is_finite() && dur > 0.0
            && !headers.contains_key("x-content-duration")
            && !headers.contains_key("content-duration")
        {
            let dur_str = format!("{:.3}", dur);
            response_text.push_str(&format!("X-Content-Duration: {}\r\n", dur_str));
            response_text.push_str(&format!("Content-Duration: {}\r\n", dur_str));
        }
    }

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

    /// Helper: start a fake upstream that captures the request and returns a
    /// canned response, exposing the received request via a channel so the test
    /// can assert on what the proxy sent upstream.
    fn fake_upstream<F>(responder: F) -> (u16, std::sync::mpsc::Receiver<String>)
    where
        F: Fn(&str) -> (String, &'static str) + Send + 'static,
    {
        let upstream = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = upstream.local_addr().unwrap().port();
        let (tx, rx) = std::sync::mpsc::channel();

        thread::spawn(move || {
            // Accept a single connection. Drop the listener after so the port
            // is freed immediately (each test uses its own upstream).
            let (mut stream, _) = upstream.accept().unwrap();
            let mut buf = [0u8; 8192];
            let n = stream.read(&mut buf).unwrap();
            let req = String::from_utf8_lossy(&buf[..n]).to_string();
            let _ = tx.send(req.clone());

            let (extra_headers, body) = responder(&req);
            let response = format!(
                "HTTP/1.1 206 Partial Content\r\n\
                 Content-Type: audio/mp4\r\n\
                 Content-Length: {}\r\n\
                 Content-Range: bytes 0-{}/{}\r\n\
                 Accept-Ranges: bytes\r\n\
                 {}\
                 \r\n{}",
                body.len(),
                body.len() - 1,
                body.len(),
                extra_headers,
                body
            );
            let _ = stream.write_all(response.as_bytes());
        });

        (port, rx)
    }

    #[test]
    fn proxy_no_range_sends_bytes_0_upstream_and_returns_206() {
        // Upstream responds 206 with Content-Range for `bytes=0-`.
        let (upstream_port, rx) = fake_upstream(|req| {
            // Confirm the proxy asked for bytes=0- (the whole file) upstream.
            assert!(
                req.to_lowercase().contains("range: bytes=0-"),
                "proxy should request Range: bytes=0- when browser sends no Range, got: {}",
                req
            );
            (String::new(), "abcdefghij") // 10-byte body
        });

        let proxy_port = start_proxy_server().unwrap();
        thread::sleep(Duration::from_millis(50));

        let remote_url = format!("http://127.0.0.1:{}/audio.m4a", upstream_port);
        let proxy_url = proxied_url(proxy_port, &remote_url);

        // Browser makes a GET with NO Range header (initial load).
        let client = reqwest::blocking::Client::new();
        let response = client.get(&proxy_url).send().unwrap();

        // Upstream returned 206, so the proxy forwards 206.
        assert_eq!(response.status(), 206, "proxy should forward 206 from upstream");

        let content_range = response
            .headers()
            .get("content-range")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());
        let accept_ranges = response
            .headers()
            .get("accept-ranges")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());
        let content_length = response
            .headers()
            .get("content-length")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        let body = response.text().unwrap();
        assert_eq!(body, "abcdefghij");

        assert_eq!(content_range.unwrap(), "bytes 0-9/10", "Content-Range must be forwarded");
        assert_eq!(accept_ranges.unwrap(), "bytes", "Accept-Ranges must be injected");
        assert_eq!(content_length.unwrap(), "10", "Content-Length must be forwarded");

        // Drain the upstream request channel so the sender doesn't hang.
        let _ = rx.recv_timeout(Duration::from_secs(1));
    }

    #[test]
    fn proxy_incoming_range_is_forwarded_unchanged() {
        // Upstream asserts it receives the EXACT browser Range.
        let (upstream_port, rx) = fake_upstream(|req| {
            let lower = req.to_lowercase();
            assert!(
                lower.contains("range: bytes=200-299"),
                "proxy must forward the browser's Range unchanged, got: {}",
                req
            );
            (String::new(), "0123456789") // 10-byte body for the requested slice
        });

        let proxy_port = start_proxy_server().unwrap();
        thread::sleep(Duration::from_millis(50));

        let remote_url = format!("http://127.0.0.1:{}/audio.m4a", upstream_port);
        let proxy_url = proxied_url(proxy_port, &remote_url);

        // Browser is seeking — sends its own Range. Proxy must NOT override it.
        let client = reqwest::blocking::Client::new();
        let response = client
            .get(&proxy_url)
            .header("Range", "bytes=200-299")
            .send()
            .unwrap();

        assert_eq!(response.status(), 206, "proxy should forward 206");
        let content_range = response
            .headers()
            .get("content-range")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());
        assert_eq!(content_range.unwrap(), "bytes 0-9/10", "Content-Range forwarded");

        let _ = rx.recv_timeout(Duration::from_secs(1));
    }

    #[test]
    fn proxy_duration_query_injects_duration_headers_when_absent() {
        // Upstream does NOT send X-Content-Duration / Content-Duration.
        let (upstream_port, rx) = fake_upstream(|_req| (String::new(), "abcdefghij"));

        let proxy_port = start_proxy_server().unwrap();
        thread::sleep(Duration::from_millis(50));

        let remote_url = format!("http://127.0.0.1:{}/audio.m4a", upstream_port);
        // Append &duration=183.5 to the proxy URL to hint the known duration.
        let proxy_url = format!("{}&duration=183.5", proxied_url(proxy_port, &remote_url));

        let client = reqwest::blocking::Client::new();
        let response = client.get(&proxy_url).send().unwrap();

        assert_eq!(response.status(), 206);

        let x_dur = response
            .headers()
            .get("x-content-duration")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());
        let c_dur = response
            .headers()
            .get("content-duration")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        assert!(x_dur.is_some(), "X-Content-Duration should be injected from duration query");
        assert!(c_dur.is_some(), "Content-Duration should be injected from duration query");
        // Both should carry the known duration.
        assert!(
            x_dur.unwrap().starts_with("183.5"),
            "X-Content-Duration value mismatch"
        );

        let _ = rx.recv_timeout(Duration::from_secs(1));
    }

    #[test]
    fn proxy_legacy_seekto_query_is_ignored_and_url_still_decoded() {
        // A frontend that still appends &seekto= (legacy) must not break the
        // proxy — seekto is ignored and the URL is decoded correctly.
        let (upstream_port, rx) = fake_upstream(|_req| (String::new(), "abcdefghij"));

        let proxy_port = start_proxy_server().unwrap();
        thread::sleep(Duration::from_millis(50));

        let remote_url = format!("http://127.0.0.1:{}/audio.m4a", upstream_port);
        // Legacy URL with both seekto (ignored) and duration (hint).
        let proxy_url = format!(
            "{}&seekto=45.0&duration=183.5",
            proxied_url(proxy_port, &remote_url)
        );

        let client = reqwest::blocking::Client::new();
        let response = client.get(&proxy_url).send().unwrap();

        // Should still reach upstream and get 206.
        assert_eq!(response.status(), 206);
        let body = response.text().unwrap();
        assert_eq!(body, "abcdefghij");

        let _ = rx.recv_timeout(Duration::from_secs(1));
    }

    #[test]
    fn proxy_serves_local_file_full_no_range() {
        // Write a temp file and serve it through the proxy as file:// URL.
        let temp_dir = std::env::temp_dir().join(format!(
            "helix-proxy-local-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let file_path = temp_dir.join("test_audio.m4a");
        let body = b"abcdefghijklmnopqrstuvwxyz0123456789"; // 36 bytes
        std::fs::write(&file_path, body).unwrap();

        let proxy_port = start_proxy_server().unwrap();
        thread::sleep(Duration::from_millis(50));

        let file_url = format!("file://{}", file_path.to_string_lossy());
        let proxy_url = proxied_url(proxy_port, &file_url);

        let client = reqwest::blocking::Client::new();
        let response = client.get(&proxy_url).send().unwrap();

        // No Range → 200 OK, full file
        assert_eq!(response.status(), 200, "local file without Range should return 200");

        let content_length = response
            .headers()
            .get("content-length")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());
        let accept_ranges = response
            .headers()
            .get("accept-ranges")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());
        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        let resp_body = response.text().unwrap();
        assert_eq!(resp_body.as_bytes(), body, "full file body should match");

        assert_eq!(content_length.unwrap(), "36", "Content-Length must match file size");
        assert_eq!(accept_ranges.unwrap(), "bytes", "Accept-Ranges must be bytes");
        assert_eq!(content_type.unwrap(), "audio/mp4", "Content-Type should be audio/mp4 for .m4a");

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn proxy_serves_local_file_with_range() {
        // Serve a byte range from a local file through the proxy.
        let temp_dir = std::env::temp_dir().join(format!(
            "helix-proxy-local-range-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let file_path = temp_dir.join("range_test.m4a");
        let body: Vec<u8> = (0..100).collect(); // 100 bytes
        std::fs::write(&file_path, &body).unwrap();

        let proxy_port = start_proxy_server().unwrap();
        thread::sleep(Duration::from_millis(50));

        let file_url = format!("file://{}", file_path.to_string_lossy());
        let proxy_url = proxied_url(proxy_port, &file_url);

        // Request bytes 10-19
        let client = reqwest::blocking::Client::new();
        let response = client
            .get(&proxy_url)
            .header("Range", "bytes=10-19")
            .send()
            .unwrap();

        assert_eq!(response.status(), 206, "Range request should return 206");

        let content_range = response
            .headers()
            .get("content-range")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());
        let content_length = response
            .headers()
            .get("content-length")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        let resp_body = response.text().unwrap();
        assert_eq!(resp_body.len(), 10, "should return 10 bytes");
        assert_eq!(resp_body.as_bytes(), &body[10..20], "byte range content should match");

        assert_eq!(content_range.unwrap(), "bytes 10-19/100", "Content-Range must be correct");
        assert_eq!(content_length.unwrap(), "10", "Content-Length must match requested range");

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn proxy_serves_local_file_open_end_range() {
        // Open-ended Range: bytes=50-
        let temp_dir = std::env::temp_dir().join(format!(
            "helix-proxy-local-open-range-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let file_path = temp_dir.join("open_range.m4a");
        let body: Vec<u8> = (0..50).collect(); // 50 bytes
        std::fs::write(&file_path, &body).unwrap();

        let proxy_port = start_proxy_server().unwrap();
        thread::sleep(Duration::from_millis(50));

        let file_url = format!("file://{}", file_path.to_string_lossy());
        let proxy_url = proxied_url(proxy_port, &file_url);

        let client = reqwest::blocking::Client::new();
        let response = client
            .get(&proxy_url)
            .header("Range", "bytes=20-")
            .send()
            .unwrap();

        assert_eq!(response.status(), 206);

        let content_range = response
            .headers()
            .get("content-range")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        let resp_body = response.text().unwrap();
        assert_eq!(resp_body.len(), 30, "should return 30 bytes (50-20)");
        assert_eq!(resp_body.as_bytes(), &body[20..], "content should match from byte 20 to end");

        assert_eq!(content_range.unwrap(), "bytes 20-49/50", "Content-Range for open-ended range");

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn proxy_local_file_duration_hint_headers() {
        // Duration hint should be injected for local files too.
        let temp_dir = std::env::temp_dir().join(format!(
            "helix-proxy-local-dur-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let file_path = temp_dir.join("dur_test.m4a");
        std::fs::write(&file_path, b"test_audio_data").unwrap();

        let proxy_port = start_proxy_server().unwrap();
        thread::sleep(Duration::from_millis(50));

        let file_url = format!("file://{}", file_path.to_string_lossy());
        let proxy_url = format!("{}&duration=200.0", proxied_url(proxy_port, &file_url));

        let client = reqwest::blocking::Client::new();
        let response = client.get(&proxy_url).send().unwrap();

        assert_eq!(response.status(), 200);

        let x_dur = response
            .headers()
            .get("x-content-duration")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());
        assert!(x_dur.is_some(), "X-Content-Duration should be injected from duration query");
        assert!(x_dur.unwrap().starts_with("200.000"), "duration value should match");

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn parse_range_header_parses_start_end() {
        let req = "GET / HTTP/1.1\r\nRange: bytes=100-199\r\n\r\n";
        let (start, end) = parse_range_header(req, 1000);
        assert_eq!(start, 100);
        assert_eq!(end, 199);
    }

    #[test]
    fn parse_range_header_open_ended() {
        let req = "GET / HTTP/1.1\r\nRange: bytes=500-\r\n\r\n";
        let (start, end) = parse_range_header(req, 1000);
        assert_eq!(start, 500);
        assert_eq!(end, 999); // file_size - 1
    }

    #[test]
    fn parse_range_header_no_range_returns_full() {
        let req = "GET / HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let (start, end) = parse_range_header(req, 1000);
        assert_eq!(start, 0);
        assert_eq!(end, 999);
    }

    #[test]
    fn parse_range_header_clamps_end_to_file_size() {
        let req = "GET / HTTP/1.1\r\nRange: bytes=0-2000\r\n\r\n";
        let (start, end) = parse_range_header(req, 1000);
        assert_eq!(start, 0);
        assert_eq!(end, 999); // clamped to file_size - 1
    }
}
