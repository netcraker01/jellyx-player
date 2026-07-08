//! YouTube source resolver via yt-dlp.
//!
//! Uses yt-dlp to search YouTube and resolve stream URLs.
//! yt-dlp handles extraction, format selection, and URL resolution.
//! Supports playlist search and resolution via `--yes-playlist`.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use uuid::Uuid;

use super::SourceResolver;
use super::yt_dlp;
use crate::errors::types::SourceError;
use helix_core::models::playlist::Playlist;
use helix_core::models::source::Source;
use helix_core::models::track::Track;

/// Number of playlist search results to request from yt-dlp.
const PLAYLIST_SEARCH_RESULT_COUNT: usize = 10;

/// Preferred yt-dlp format selector for YouTube audio.
/// Prefers m4a/AAC (itag 140) for Symphonia compatibility, then mp4a codecs, then any bestaudio.
pub const YOUTUBE_AUDIO_FORMAT: &str = "bestaudio[ext=m4a]/bestaudio[acodec^=mp4a]/bestaudio";

/// TTL for cached resolved tracks. YouTube stream URLs are signed with ~6h expiry;
/// 5h is a safe margin that avoids stale-URL 403s while maximizing cache hits.
const RESOLVE_CACHE_TTL: Duration = Duration::from_secs(3600 * 5);

/// Cache entry: the resolved track plus the time it was cached.
struct CacheEntry {
    track: Track,
    cached_at: Instant,
}

/// Global resolve cache keyed by video ID.
/// Eliminates redundant yt-dlp invocations for recently resolved tracks.
static RESOLVE_CACHE: OnceLock<Mutex<HashMap<String, CacheEntry>>> = OnceLock::new();

fn resolve_cache() -> &'static Mutex<HashMap<String, CacheEntry>> {
    RESOLVE_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

pub struct YouTubeResolver;

/// Cached result of yt-dlp availability check.
/// Delegates to the shared `yt_dlp` module which resolves bundled or PATH yt-dlp.
static YT_DLP_AVAILABLE: OnceLock<bool> = OnceLock::new();

impl YouTubeResolver {
    pub fn new() -> Self {
        Self
    }

    /// Check if yt-dlp is available (auto-downloaded, bundled, or system PATH).
    /// Cached after first check — avoids ~100-300ms subprocess spawn per resolve.
    /// On first call, triggers auto-download if no yt-dlp is found.
    fn check_yt_dlp() -> Result<(), SourceError> {
        let available = *YT_DLP_AVAILABLE.get_or_init(|| {
            yt_dlp::check_yt_dlp().is_ok()
        });

        if available {
            Ok(())
        } else {
            Err(SourceError::DependencyMissing(
                "yt-dlp is not available and auto-download failed. \
                 Install it from https://github.com/yt-dlp/yt-dlp or check your internet connection.".to_string(),
            ))
        }
    }
 
    /// Parse a single yt-dlp JSON line into a Track.
    fn parse_track_from_json(json_str: &str) -> Option<Track> {
        let value: serde_json::Value = serde_json::from_str(json_str).ok()?;

        let source_id = value.get("id")?.as_str()?.to_string();
        let title = value.get("title")?.as_str()?.to_string();

        // yt-dlp provides "uploader" or "artist" or "channel"
        let artist = value
            .get("artist")
            .or_else(|| value.get("uploader"))
            .or_else(|| value.get("channel"))
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown")
            .to_string();

        let duration = value.get("duration").and_then(|v| v.as_f64());

        let thumbnail = value
            .get("thumbnail")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .or_else(|| {
                value
                    .get("thumbnails")
                    .and_then(|v| v.as_array())
                    .and_then(|arr| arr.last())
                    .and_then(|thumb| thumb.get("url"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
            });

        let album = value
            .get("album")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // webpage_url is the full YouTube URL for resolve
        let webpage_url = value
            .get("webpage_url")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let mut metadata = HashMap::new();
        if let Some(url) = webpage_url {
            metadata.insert("webpage_url".to_string(), url);
        }
        if let Some(description) = value.get("description").and_then(|v| v.as_str()) {
            metadata.insert("description".to_string(), description.to_string());
        }
        if let Some(view_count) = value.get("view_count").and_then(|v| v.as_u64()) {
            metadata.insert("view_count".to_string(), view_count.to_string());
        }

        Some(Track {
            id: Uuid::new_v4().to_string(),
            source: Source::YouTube,
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

    /// Parse a yt-dlp JSON line into a Playlist (when the entry is a playlist).
    ///
    /// yt-dlp `--yes-playlist` with `ytsearch` may return playlist entries.
    /// Each entry has `_type: "playlist"` and an `entries` array with track items.
    fn parse_playlist_from_json(json_str: &str) -> Option<Playlist> {
        let value: serde_json::Value = serde_json::from_str(json_str).ok()?;

        // Only parse entries that are playlists
        let entry_type = value.get("_type").and_then(|v| v.as_str()).unwrap_or("");
        if entry_type != "playlist" {
            return None;
        }

        let source_id = value.get("id")?.as_str()?.to_string();
        let title = value
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("Untitled Playlist")
            .to_string();

        let thumbnail = value
            .get("thumbnail")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // Parse entries array into tracks
        let entries = value.get("entries").and_then(|v| v.as_array());
        let mut tracks = Vec::new();

        if let Some(entries_arr) = entries {
            for entry in entries_arr {
                let entry_str = serde_json::to_string(entry).ok();
                if let Some(s) = entry_str {
                    if let Some(mut track) = Self::parse_track_from_json(&s) {
                        track.playlist_id = Some(source_id.clone());
                        tracks.push(track);
                    }
                }
            }
        }

        let track_count = tracks.len();

        Some(Playlist {
            id: Uuid::new_v4().to_string(),
            source: Source::YouTube,
            source_id,
            title,
            thumbnail,
            track_count,
            tracks,
        })
    }

    /// Parse yt-dlp playlist dump output into a Playlist with all tracks.
    ///
    /// The first JSON line is the playlist metadata; subsequent lines are entries.
    /// yt-dlp `--yes-playlist --dump-json` outputs one JSON object per line:
    ///   line 0: playlist metadata (with `_type: "playlist"` and `entries` array)
    ///   lines 1+: individual entry metadata (each with `_type: "video"` or absent)
    fn parse_playlist_dump(stdout: &str) -> Result<Playlist, SourceError> {
        let lines: Vec<&str> = stdout.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();
        if lines.is_empty() {
            return Err(SourceError::ResolveError(
                "Empty yt-dlp playlist output".to_string(),
            ));
        }

        // Parse all JSON entries. With --flat-playlist, every line is a video
        // entry and playlist metadata is embedded in each entry's `playlist_*`
        // fields. Without --flat-playlist, the first line is playlist metadata.
        let mut entries: Vec<serde_json::Value> = Vec::new();
        for line in &lines {
            match serde_json::from_str::<serde_json::Value>(line) {
                Ok(v) => entries.push(v),
                Err(_) => continue,
            }
        }

        if entries.is_empty() {
            return Err(SourceError::ResolveError(
                "No valid JSON entries in playlist output".to_string(),
            ));
        }

        // Extract playlist-level metadata.
        // Check if the first entry is a playlist (has _type: "playlist")
        // or if all entries are videos (flat-playlist mode).
        let first = &entries[0];
        let first_type = first.get("_type").and_then(|v| v.as_str()).unwrap_or("");

        let (source_id, title, thumbnail) = if first_type == "playlist" {
            // Non-flat mode: first entry is playlist metadata, rest are videos.
            let source_id = first.get("id").and_then(|v| v.as_str()).unwrap_or("unknown").to_string();
            let title = first.get("title").and_then(|v| v.as_str()).unwrap_or("Untitled Playlist").to_string();
            let thumbnail = first.get("thumbnail").and_then(|v| v.as_str()).map(|s| s.to_string());
            (source_id, title, thumbnail)
        } else {
            // Flat mode: all entries are videos. Extract playlist metadata
            // from the `playlist_*` fields present in each video entry.
            // Fall back to top-level `id`/`title` for test/edge cases.
            let source_id = first
                .get("playlist_id")
                .and_then(|v| v.as_str())
                .or_else(|| first.get("id").and_then(|v| v.as_str()))
                .unwrap_or("unknown")
                .to_string();
            let title = first
                .get("playlist_title")
                .and_then(|v| v.as_str())
                .or_else(|| first.get("title").and_then(|v| v.as_str()))
                .unwrap_or("Untitled Playlist")
                .to_string();
            let thumbnail = first
                .get("playlist_thumbnail")
                .and_then(|v| v.as_str())
                .or_else(|| first.get("thumbnail").and_then(|v| v.as_str()))
                .map(|s| s.to_string());
            (source_id, title, thumbnail)
        };

        // Parse tracks from entries. In non-flat mode, skip the first entry
        // (playlist metadata). In flat mode, all entries are videos.
        let track_entries = if first_type == "playlist" { &entries[1..] } else { &entries[..] };

        let mut tracks = Vec::new();
        for entry in track_entries {
            let line = serde_json::to_string(entry).unwrap_or_default();
            if let Some(mut track) = Self::parse_track_from_json(&line) {
                // Filter out non-video entries (e.g., nested playlists)
                let entry_type = entry.get("_type").and_then(|v| v.as_str()).unwrap_or("");
                if !entry_type.is_empty() && entry_type != "video" && entry_type != "url" {
                    continue;
                }
                track.playlist_id = Some(source_id.clone());
                tracks.push(track);
            }
        }

        let track_count = tracks.len();

        Ok(Playlist {
            id: Uuid::new_v4().to_string(),
            source: Source::YouTube,
            source_id,
            title,
            thumbnail,
            track_count,
            tracks,
        })
    }
}

impl SourceResolver for YouTubeResolver {
    fn source_type(&self) -> Source {
        Source::YouTube
    }

    fn search(&self, query: &str, offset: usize, limit: usize) -> Result<Vec<Track>, SourceError> {
        Self::check_yt_dlp()?;

        // yt-dlp pagination: request enough results to cover offset+limit, then
        // use --playlist-start/--playlist-end to extract only the requested page.
        let end = offset + limit;
        let output = yt_dlp::yt_dlp_command()?
            .arg(format!("ytsearch{}:{}", end, query))
            .arg("--flat-playlist")
            .arg("--dump-json")
            .arg("--no-download")
            .arg("--no-playlist")
            .arg("--playlist-start")
            .arg((offset + 1).to_string())
            .arg("--playlist-end")
            .arg(end.to_string())
            .output()
            .map_err(|e| SourceError::NetworkError(e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(SourceError::NetworkError(format!(
                "yt-dlp search failed: {}",
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
        // Key by the raw id (video ID or full URL) for O(1) lookup.
        if let Ok(cache) = resolve_cache().lock() {
            if let Some(entry) = cache.get(id) {
                if entry.cached_at.elapsed() < RESOLVE_CACHE_TTL {
                    return Ok(entry.track.clone());
                }
            }
        }

        // Build YouTube URL from video ID if it's not already a full URL
        let url = if id.starts_with("http") {
            id.to_string()
        } else {
            format!("https://www.youtube.com/watch?v={}", id)
        };

        // Resolve stream URL and metadata in a single yt-dlp invocation.
        // --print %(url)s outputs the resolved format URL on its own line,
        // --dump-json outputs the full metadata as JSON on subsequent lines.
        // This avoids running yt-dlp twice (which would double the extraction time).
        let output = yt_dlp::yt_dlp_command()?
            .arg(&url)
            .arg("--format")
            .arg(YOUTUBE_AUDIO_FORMAT)
            .arg("--print")
            .arg("%(url)s")
            .arg("--dump-json")
            .arg("--no-download")
            .arg("--no-playlist")
            .output()
            .map_err(|e| SourceError::NetworkError(e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(SourceError::ResolveError(format!(
                "yt-dlp resolve failed: {}",
                stderr.trim()
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut lines = stdout.lines().filter(|l| !l.trim().is_empty());

        // First line: the resolved stream URL (--print %(url)s)
        let stream_url = lines.next()
            .unwrap_or("")
            .trim()
            .to_string();

        if stream_url.is_empty() {
            return Err(SourceError::ResolveError(
                "No stream URL returned by yt-dlp".to_string(),
            ));
        }

        // Find the JSON line: it's the first line that starts with '{'
        let json_line = lines.find(|l| l.trim().starts_with('{'))
            .unwrap_or("");

        let mut track = Self::parse_track_from_json(json_line).unwrap_or_else(|| Track {
            id: Uuid::new_v4().to_string(),
            source: Source::YouTube,
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
            cache.insert(id.to_string(), CacheEntry {
                track: track.clone(),
                cached_at: Instant::now(),
            });
        }

        Ok(track)
    }

    fn search_playlists(&self, query: &str) -> Result<Vec<Playlist>, SourceError> {
        Self::check_yt_dlp()?;

        let output = yt_dlp::yt_dlp_command()?
            .arg(format!(
                "ytsearch{}:{}",
                PLAYLIST_SEARCH_RESULT_COUNT, query
            ))
            .arg("--dump-json")
            .arg("--no-download")
            .arg("--yes-playlist")
            .output()
            .map_err(|e| SourceError::NetworkError(e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // If yt-dlp returns no results, it may exit with an error — return empty
            if stderr.contains("no matching") || stderr.contains("Unable to") {
                return Ok(Vec::new());
            }
            return Err(SourceError::NetworkError(format!(
                "yt-dlp playlist search failed: {}",
                stderr.trim()
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut playlists = Vec::new();

        for line in stdout.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            // Only playlist-type entries are relevant for playlist search
            if let Some(playlist) = Self::parse_playlist_from_json(line) {
                playlists.push(playlist);
            }
        }

        Ok(playlists)
    }

    fn resolve_playlist(&self, url: &str) -> Result<Playlist, SourceError> {
        Self::check_yt_dlp()?;

        // --flat-playlist fetches all entries from the playlist page in a single
        // request, without resolving each video individually. Without it, yt-dlp
        // makes one HTTP request per video, which is extremely slow for large
        // playlists (50+ videos = 50+ sequential requests).
        let output = yt_dlp::yt_dlp_command()?
            .arg(url)
            .arg("--flat-playlist")
            .arg("--dump-json")
            .arg("--no-download")
            .arg("--yes-playlist")
            .output()
            .map_err(|e| SourceError::NetworkError(e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stderr_str = stderr.trim();

            // Detect private or unavailable playlists
            if stderr_str.contains("Private")
                || stderr_str.contains("Sign in")
                || stderr_str.contains("not available")
            {
                return Err(SourceError::ResolveError(
                    "Playlist is private or unavailable".to_string(),
                ));
            }

            return Err(SourceError::ResolveError(format!(
                "yt-dlp playlist resolve failed: {}",
                stderr_str
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        Self::parse_playlist_dump(&stdout)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Page size for paginated search — used only in tests to verify the constant value.
    const SEARCH_PAGE_SIZE: usize = 50;

    #[test]
    fn youtube_resolver_source_type() {
        let resolver = YouTubeResolver::new();
        assert_eq!(resolver.source_type(), Source::YouTube);
    }

    #[test]
    fn youtube_resolver_parse_valid_json() {
        let json = r#"{"id":"dQw4w9WgXcQ","title":"Rick Astley - Never Gonna Give You Up","artist":"Rick Astley","duration":212.0,"thumbnail":"https://img.youtube.com/vi/dQw4w9WgXcQ/0.jpg","description":"Classic pop video","webpage_url":"https://www.youtube.com/watch?v=dQw4w9WgXcQ"}"#;
        let track = YouTubeResolver::parse_track_from_json(json).unwrap();
        assert_eq!(track.source_id, "dQw4w9WgXcQ");
        assert_eq!(track.title, "Rick Astley - Never Gonna Give You Up");
        assert_eq!(track.artist, "Rick Astley");
        assert_eq!(track.duration, Some(212.0));
        assert_eq!(track.source, Source::YouTube);
        // stream_url comes from --print %(url)s, not from JSON parse
        assert!(track.stream_url.is_none());
        assert!(track.metadata.contains_key("webpage_url"));
        assert!(track.metadata.contains_key("description"));
    }

    #[test]
    fn youtube_resolver_parse_missing_fields() {
        let json = r#"{"id":"abc123","title":"Some Song"}"#;
        let track = YouTubeResolver::parse_track_from_json(json).unwrap();
        assert_eq!(track.source_id, "abc123");
        assert_eq!(track.artist, "Unknown");
        assert!(track.duration.is_none());
        assert!(track.thumbnail.is_none());
    }

    #[test]
    fn youtube_resolver_parse_thumbnail_from_thumbnails_array() {
        let json = r#"{"id":"-8W1m57U5zc","title":"DeepMe - Live @ High Desert","uploader":"DeepMe","duration":3632.0,"thumbnails":[{"url":"https://i.ytimg.com/vi/-8W1m57U5zc/hq360.jpg","height":202,"width":360},{"url":"https://i.ytimg.com/vi/-8W1m57U5zc/hq720.jpg","height":404,"width":720}],"webpage_url":"https://www.youtube.com/watch?v=-8W1m57U5zc"}"#;
        let track = YouTubeResolver::parse_track_from_json(json).unwrap();
        assert_eq!(
            track.thumbnail,
            Some("https://i.ytimg.com/vi/-8W1m57U5zc/hq720.jpg".to_string())
        );
    }

    #[test]
    fn youtube_resolver_parse_invalid_json() {
        let json = "not json at all";
        let result = YouTubeResolver::parse_track_from_json(json);
        assert!(result.is_none());
    }

    #[test]
    fn youtube_resolver_parse_missing_id() {
        let json = r#"{"title":"No ID"}"#;
        let result = YouTubeResolver::parse_track_from_json(json);
        assert!(result.is_none());
    }

    #[test]
    fn youtube_resolver_search_result_count_constant() {
        assert_eq!(SEARCH_PAGE_SIZE, 50);
        assert_eq!(PLAYLIST_SEARCH_RESULT_COUNT, 10);
    }

    #[test]
    fn youtube_resolver_format_prefers_m4a_aac() {
        assert!(YOUTUBE_AUDIO_FORMAT.contains("ext=m4a"), "Format should prefer m4a extension");
        assert!(YOUTUBE_AUDIO_FORMAT.contains("acodec^=mp4a"), "Format should fallback to mp4a codec");
        assert!(YOUTUBE_AUDIO_FORMAT.ends_with("bestaudio"), "Format should fallback to bestaudio");
    }

    #[test]
    fn youtube_resolver_parse_playlist_from_json_valid() {
        let json = r#"{"_type":"playlist","id":"PLrAXtmErZgOei3XmJLpYCyoF7RjRlS1MF","title":"Test Playlist","thumbnail":"https://img.test/pl.jpg","playlist_count":3,"entries":[{"id":"v1","title":"Song 1","artist":"Artist A","duration":200.0},{"id":"v2","title":"Song 2","uploader":"Artist B","duration":180.0}]}"#;
        let playlist = YouTubeResolver::parse_playlist_from_json(json).unwrap();
        assert_eq!(playlist.source_id, "PLrAXtmErZgOei3XmJLpYCyoF7RjRlS1MF");
        assert_eq!(playlist.title, "Test Playlist");
        assert_eq!(playlist.thumbnail, Some("https://img.test/pl.jpg".to_string()));
        assert_eq!(playlist.source, Source::YouTube);
        assert_eq!(playlist.tracks.len(), 2);
        assert_eq!(playlist.tracks[0].playlist_id, Some("PLrAXtmErZgOei3XmJLpYCyoF7RjRlS1MF".to_string()));
    }

    #[test]
    fn youtube_resolver_parse_playlist_from_json_video_entry_skipped() {
        // Video entries should not be parsed as playlists
        let json = r#"{"id":"dQw4w9WgXcQ","title":"Rick Astley - Never Gonna Give You Up","_type":"video"}"#;
        let result = YouTubeResolver::parse_playlist_from_json(json);
        assert!(result.is_none());
    }

    #[test]
    fn youtube_resolver_parse_playlist_from_json_missing_type_not_playlist() {
        // Entries without _type are not playlists
        let json = r#"{"id":"abc123","title":"Not A Playlist"}"#;
        let result = YouTubeResolver::parse_playlist_from_json(json);
        assert!(result.is_none());
    }

    #[test]
    fn youtube_resolver_parse_playlist_from_json_no_entries() {
        let json = r#"{"_type":"playlist","id":"PLtest","title":"Empty Playlist","entries":[]}"#;
        let playlist = YouTubeResolver::parse_playlist_from_json(json).unwrap();
        assert_eq!(playlist.tracks.len(), 0);
        assert_eq!(playlist.track_count, 0);
    }

    #[test]
    fn youtube_resolver_parse_playlist_dump_valid() {
        let playlist_json = r#"{"_type":"playlist","id":"PLrAXtmErZgOei3XmJLpYCyoF7RjRlS1MF","title":"My Great Playlist","thumbnail":"https://img.test/pl.jpg"}"#;
        let track1_json = r#"{"id":"v1","title":"Song One","artist":"Artist A","duration":200.0,"webpage_url":"https://youtube.com/watch?v=v1"}"#;
        let track2_json = r#"{"id":"v2","title":"Song Two","uploader":"Artist B","duration":180.0,"webpage_url":"https://youtube.com/watch?v=v2"}"#;

        let stdout = format!("{}\n{}\n{}", playlist_json, track1_json, track2_json);
        let playlist = YouTubeResolver::parse_playlist_dump(&stdout).unwrap();

        assert_eq!(playlist.source_id, "PLrAXtmErZgOei3XmJLpYCyoF7RjRlS1MF");
        assert_eq!(playlist.title, "My Great Playlist");
        assert_eq!(playlist.tracks.len(), 2);
        assert_eq!(playlist.tracks[0].source_id, "v1");
        assert_eq!(playlist.tracks[1].source_id, "v2");
        assert_eq!(playlist.tracks[0].playlist_id, Some("PLrAXtmErZgOei3XmJLpYCyoF7RjRlS1MF".to_string()));
        assert_eq!(playlist.track_count, 2);
    }

    #[test]
    fn youtube_resolver_parse_playlist_dump_filters_non_video_entries() {
        let playlist_json = r#"{"_type":"playlist","id":"PLtest","title":"Mixed Playlist","thumbnail":"https://img.test/pl.jpg"}"#;
        let video_json = r#"{"id":"v1","title":"Song One","artist":"Artist A","duration":200.0}"#;
        let non_video_json = r#"{"id":"pl1","title":"Nested Playlist","_type":"playlist","duration":0}"#;

        let stdout = format!("{}\n{}\n{}", playlist_json, video_json, non_video_json);
        let playlist = YouTubeResolver::parse_playlist_dump(&stdout).unwrap();

        // Only the video entry should be included
        assert_eq!(playlist.tracks.len(), 1);
        assert_eq!(playlist.tracks[0].source_id, "v1");
    }

    #[test]
    fn youtube_resolver_parse_playlist_dump_empty_input() {
        let result = YouTubeResolver::parse_playlist_dump("");
        assert!(result.is_err());
    }

    #[test]
    fn youtube_resolver_parse_playlist_dump_defaults_on_missing_fields() {
        // In flat-playlist mode, entries without _type are treated as video
        // entries. Playlist metadata is extracted from playlist_* fields with
        // fallback to top-level id/title.
        let playlist_json = r#"{"id":"PLminimal","title":"Minimal Playlist"}"#;
        let stdout = playlist_json.to_string();
        let playlist = YouTubeResolver::parse_playlist_dump(&stdout).unwrap();

        // No playlist_id field, so falls back to top-level id
        assert_eq!(playlist.source_id, "PLminimal");
        // No playlist_title field, so falls back to top-level title
        assert_eq!(playlist.title, "Minimal Playlist");
        assert!(playlist.thumbnail.is_none());
        // The single entry is treated as a video track
        assert_eq!(playlist.tracks.len(), 1);
    }
}
