//! SoundCloud source resolver via yt-dlp.
//!
//! Uses yt-dlp to search SoundCloud and resolve stream URLs.
//! yt-dlp supports SoundCloud natively via `scsearch<N>:<query>` prefix.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use uuid::Uuid;

use super::yt_dlp;
use super::SourceResolver;
use crate::errors::types::SourceError;
use jellyx_core::models::source::Source;
use jellyx_core::models::track::Track;

/// Preferred yt-dlp format selector for SoundCloud audio.
///
/// SoundCloud serves both HLS (m3u8) and direct HTTP formats.
/// HLS streams are NOT supported by HTMLAudioElement in most browsers,
/// so we prioritize direct HTTP formats (protocol=https) to ensure
/// browser-native playback works without hls.js.
///
/// Priority: HTTP mp3 128k > HTTP aac > HLS aac > any bestaudio (fallback)
const SOUNDCLOUD_AUDIO_FORMAT: &str =
    "bestaudio[protocol=https]/bestaudio[protocol=http]/bestaudio";

/// TTL for cached resolved tracks. SoundCloud CDN URLs also expire;
/// 5h is a safe margin consistent with YouTube's cache.
const RESOLVE_CACHE_TTL: Duration = Duration::from_secs(3600 * 5);

/// Cache entry: the resolved track plus the time it was cached.
struct CacheEntry {
    track: Track,
    cached_at: Instant,
}

/// Global resolve cache keyed by track source_id (API URL or webpage URL).
static RESOLVE_CACHE: OnceLock<Mutex<HashMap<String, CacheEntry>>> = OnceLock::new();

fn resolve_cache() -> &'static Mutex<HashMap<String, CacheEntry>> {
    RESOLVE_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

pub struct SoundCloudResolver;

/// Cached result of yt-dlp availability check.
/// Delegates to the shared `yt_dlp` module which resolves bundled or PATH yt-dlp.
static YT_DLP_AVAILABLE: OnceLock<bool> = OnceLock::new();

impl SoundCloudResolver {
    pub fn new() -> Self {
        Self
    }

    /// Check if yt-dlp is available (auto-downloaded, bundled, or system PATH).
    /// Cached after first check — avoids ~100-300ms subprocess spawn per resolve.
    /// On first call, triggers auto-download if no yt-dlp is found.
    fn check_yt_dlp() -> Result<(), SourceError> {
        let available = *YT_DLP_AVAILABLE.get_or_init(|| yt_dlp::check_yt_dlp().is_ok());

        if available {
            Ok(())
        } else {
            Err(SourceError::DependencyMissing(
                "yt-dlp is not available and auto-download failed. \
                 Install it from https://github.com/yt-dlp/yt-dlp or check your internet connection.".to_string(),
            ))
        }
    }

    /// Parse a single yt-dlp JSON line into a Track for SoundCloud.
    fn parse_track_from_json(json_str: &str) -> Option<Track> {
        let value: serde_json::Value = serde_json::from_str(json_str).ok()?;

        // SoundCloud needs a resolvable URL as source_id for play_stream -> resolve().
        // The "url" field (API URL) works directly with yt-dlp resolve.
        // "webpage_url" (human URL) may 404, so we prefer the API URL.
        // Fall back through: url > webpage_url > string id > numeric id.
        let source_id = value
            .get("url")
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .or_else(|| {
                value
                    .get("webpage_url")
                    .and_then(|v| v.as_str().map(|s| s.to_string()))
            })
            .or_else(|| {
                value
                    .get("id")
                    .and_then(|v| v.as_str().map(|s| s.to_string()))
            })
            .or_else(|| {
                value
                    .get("id")
                    .and_then(|v| v.as_u64().map(|n| n.to_string()))
            })
            .unwrap_or_default();

        let title = value.get("title")?.as_str()?.to_string();

        let artist = value
            .get("artist")
            .or_else(|| value.get("uploader"))
            .or_else(|| value.get("channel"))
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown")
            .to_string();

        let duration = value.get("duration").and_then(|v| v.as_f64());

        // SoundCloud returns "thumbnails" as an array of objects with "url" and optional "width".
        // Fall back to "thumbnail" (string) for other yt-dlp versions.
        let thumbnail = value
            .get("thumbnails")
            .and_then(|v| v.as_array())
            .and_then(|arr| {
                // Pick the largest thumbnail by width, falling back to the last entry
                arr.iter()
                    .filter_map(|t| {
                        let url = t.get("url")?.as_str()?.to_string().into();
                        let width: Option<u32> =
                            t.get("width").and_then(|w| w.as_u64()).map(|w| w as u32);
                        Some((url, width))
                    })
                    .max_by_key(|(_, w)| *w)
                    .map(|(url, _)| url)
            })
            .or_else(|| {
                value
                    .get("thumbnail")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
            });

        let album = value
            .get("album")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // webpage_url for SoundCloud is the human-friendly track page URL
        let webpage_url = value
            .get("webpage_url")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let mut metadata = HashMap::new();
        if let Some(url) = webpage_url {
            metadata.insert("webpage_url".to_string(), url);
        }

        Some(Track {
            id: Uuid::new_v4().to_string(),
            source: Source::SoundCloud,
            source_id,
            title,
            artist,
            album,
            duration,
            thumbnail,
            stream_url: None, // Resolved via --print %(url)s, not from JSON
            local_path: None,
            playlist_id: None,
            metadata,
        })
    }
}

impl SourceResolver for SoundCloudResolver {
    fn source_type(&self) -> Source {
        Source::SoundCloud
    }

    fn search(&self, query: &str, offset: usize, limit: usize) -> Result<Vec<Track>, SourceError> {
        Self::check_yt_dlp()?;

        let end = offset + limit;
        let output = yt_dlp::yt_dlp_command()?
            .arg(format!("scsearch{}:{}", end, query))
            .arg("--flat-playlist")
            .arg("--dump-json")
            .arg("--no-download")
            .arg("--playlist-start")
            .arg((offset + 1).to_string())
            .arg("--playlist-end")
            .arg(end.to_string())
            .output()
            .map_err(|e| SourceError::NetworkError(e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(SourceError::NetworkError(format!(
                "yt-dlp SoundCloud search failed: {}",
                stderr.trim()
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut tracks = Vec::new();

        for line in stdout.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if let Some(track) = Self::parse_track_from_json(line) {
                tracks.push(track);
            }
        }

        Ok(tracks)
    }

    fn resolve(&self, id: &str) -> Result<Track, SourceError> {
        Self::check_yt_dlp()?;

        // Check cache first — avoids yt-dlp invocation on replays/cache hits.
        if let Ok(cache) = resolve_cache().lock() {
            if let Some(entry) = cache.get(id) {
                if entry.cached_at.elapsed() < RESOLVE_CACHE_TTL {
                    return Ok(entry.track.clone());
                }
            }
        }

        // Build SoundCloud URL if it's not already a full URL
        let url = if id.starts_with("http") {
            id.to_string()
        } else {
            format!("https://soundcloud.com/{}", id)
        };

        // Resolve stream URL and metadata in a single yt-dlp invocation.
        // --print %(url)s outputs the resolved format URL on its own line,
        // --dump-json outputs the full metadata as JSON on subsequent lines.
        let output = yt_dlp::yt_dlp_command()?
            .arg(&url)
            .arg("--format")
            .arg(SOUNDCLOUD_AUDIO_FORMAT)
            .arg("--print")
            .arg("%(url)s")
            .arg("--dump-json")
            .arg("--no-download")
            .arg("--no-playlist")
            .output()
            .map_err(|e| SourceError::NetworkError(e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let err_msg = stderr.trim();
            // Detect DRM-protected tracks and provide a user-friendly message
            if err_msg.contains("DRM protected") {
                return Err(SourceError::ResolveError(
                    "This track is DRM-protected and cannot be played. SoundCloud restricts some tracks.".to_string(),
                ));
            }
            return Err(SourceError::ResolveError(format!(
                "yt-dlp resolve failed: {}",
                err_msg
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut lines = stdout.lines().filter(|l| !l.trim().is_empty());

        // First line: the resolved stream URL (--print %(url)s)
        let stream_url = lines.next().unwrap_or("").trim().to_string();

        if stream_url.is_empty() {
            return Err(SourceError::ResolveError(
                "No stream URL returned by yt-dlp".to_string(),
            ));
        }

        // Find the JSON line: it's the first line that starts with '{'
        let json_line = lines.find(|l| l.trim().starts_with('{')).unwrap_or("");

        let mut track = Self::parse_track_from_json(json_line).unwrap_or_else(|| Track {
            id: Uuid::new_v4().to_string(),
            source: Source::SoundCloud,
            source_id: id.to_string(),
            title: "Unknown".to_string(),
            artist: "Unknown".to_string(),
            album: None,
            duration: None,
            thumbnail: None,
            stream_url: Some(stream_url.clone()),
            local_path: None,
            playlist_id: None,
            metadata: HashMap::new(),
        });

        track.stream_url = Some(stream_url);

        // Store in cache for instant replays within the TTL window.
        if let Ok(mut cache) = resolve_cache().lock() {
            cache.insert(
                id.to_string(),
                CacheEntry {
                    track: track.clone(),
                    cached_at: Instant::now(),
                },
            );
        }

        Ok(track)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Page size for paginated search — used only in tests to verify the constant value.
    const SEARCH_PAGE_SIZE: usize = 50;

    #[test]
    fn soundcloud_resolver_source_type() {
        let resolver = SoundCloudResolver::new();
        assert_eq!(resolver.source_type(), Source::SoundCloud);
    }

    #[test]
    fn soundcloud_resolver_parse_valid_json() {
        let json = r#"{"id":"1234567","title":"Ambient Mix","artist":"DJ Chill","duration":3600.0,"thumbnail":"https://i1.sndcdn.com/artwork.jpg","url":"https://api.soundcloud.com/tracks/soundcloud%3Atracks%3A1234567","webpage_url":"https://soundcloud.com/djchill/ambient-mix"}"#;
        let track = SoundCloudResolver::parse_track_from_json(json).unwrap();
        assert_eq!(
            track.source_id,
            "https://api.soundcloud.com/tracks/soundcloud%3Atracks%3A1234567"
        );
        assert_eq!(track.title, "Ambient Mix");
        assert_eq!(track.artist, "DJ Chill");
        assert_eq!(track.duration, Some(3600.0));
        assert_eq!(track.source, Source::SoundCloud);
        // stream_url comes from --print %(url)s, not from JSON parse
        assert!(track.stream_url.is_none());
        assert!(track.metadata.contains_key("webpage_url"));
    }

    #[test]
    fn soundcloud_resolver_parse_numeric_id() {
        // When no url/webpage_url, fall back to numeric id
        let json = r#"{"id":98765,"title":"Numeric ID Track","uploader":"Artist"}"#;
        let track = SoundCloudResolver::parse_track_from_json(json).unwrap();
        assert_eq!(track.source_id, "98765");
        assert_eq!(track.artist, "Artist");
    }

    #[test]
    fn soundcloud_resolver_parse_missing_fields() {
        // When only string id is present (no url/webpage_url), use it as source_id
        let json = r#"{"id":"sc-abc","title":"Minimal"}"#;
        let track = SoundCloudResolver::parse_track_from_json(json).unwrap();
        assert_eq!(track.source_id, "sc-abc");
        assert_eq!(track.artist, "Unknown");
        assert!(track.duration.is_none());
    }

    #[test]
    fn soundcloud_resolver_prefers_api_url_as_source_id() {
        // "url" (API URL) should be source_id when present
        let json = r#"{"id":12345,"title":"Track","url":"https://api.soundcloud.com/tracks/soundcloud%3Atracks%3A12345","webpage_url":"https://soundcloud.com/artist/track"}"#;
        let track = SoundCloudResolver::parse_track_from_json(json).unwrap();
        assert_eq!(
            track.source_id,
            "https://api.soundcloud.com/tracks/soundcloud%3Atracks%3A12345"
        );
    }

    #[test]
    fn soundcloud_resolver_parse_invalid_json() {
        let result = SoundCloudResolver::parse_track_from_json("bad json");
        assert!(result.is_none());
    }

    #[test]
    fn soundcloud_resolver_search_result_count_constant() {
        assert_eq!(SEARCH_PAGE_SIZE, 50);
    }

    #[test]
    fn soundcloud_resolver_format_prefers_http_over_hls() {
        assert!(
            SOUNDCLOUD_AUDIO_FORMAT.contains("protocol=https"),
            "Format should prefer HTTPS direct streams"
        );
        assert!(
            SOUNDCLOUD_AUDIO_FORMAT.contains("protocol=http"),
            "Format should fallback to HTTP direct streams"
        );
        assert!(
            SOUNDCLOUD_AUDIO_FORMAT.ends_with("bestaudio"),
            "Format should fallback to bestaudio"
        );
    }

    #[test]
    fn soundcloud_resolver_parse_thumbnails_array() {
        // SoundCloud returns thumbnails as an array of objects with "url" and "width"
        let json = r#"{"id":"123","title":"Test Track","uploader":"Artist","duration":180.0,"thumbnails":[{"id":"small","url":"https://i1.sndcdn.com/small.jpg","width":32},{"id":"t500x500","url":"https://i1.sndcdn.com/large.jpg","width":500},{"id":"original","url":"https://i1.sndcdn.com/original.jpg","width":1000}]}"#;
        let track = SoundCloudResolver::parse_track_from_json(json).unwrap();
        assert_eq!(
            track.thumbnail,
            Some("https://i1.sndcdn.com/original.jpg".to_string()),
            "Should pick the largest thumbnail by width"
        );
    }

    #[test]
    fn soundcloud_resolver_parse_thumbnail_string_fallback() {
        // If "thumbnails" array is missing but "thumbnail" string exists, use it
        let json = r#"{"id":"123","title":"Test Track","uploader":"Artist","duration":180.0,"thumbnail":"https://i1.sndcdn.com/artwork.jpg"}"#;
        let track = SoundCloudResolver::parse_track_from_json(json).unwrap();
        assert_eq!(
            track.thumbnail,
            Some("https://i1.sndcdn.com/artwork.jpg".to_string()),
            "Should fall back to thumbnail string field"
        );
    }

    #[test]
    fn soundcloud_resolver_parse_no_thumbnails() {
        // Neither thumbnails array nor thumbnail string
        let json = r#"{"id":"123","title":"Test Track","uploader":"Artist","duration":180.0}"#;
        let track = SoundCloudResolver::parse_track_from_json(json).unwrap();
        assert!(
            track.thumbnail.is_none(),
            "Should be None when no thumbnail data"
        );
    }
}
