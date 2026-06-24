//! YouTube source resolver via yt-dlp.
//!
//! Uses yt-dlp to search YouTube and resolve stream URLs.
//! yt-dlp handles extraction, format selection, and URL resolution.
//! Supports playlist search and resolution via `--yes-playlist`.

use std::collections::HashMap;
use std::process::Command;

use uuid::Uuid;

use super::SourceResolver;
use crate::errors::types::SourceError;
use crate::models::playlist::Playlist;
use crate::models::source::Source;
use crate::models::track::Track;

/// Number of search results to request from yt-dlp.
const SEARCH_RESULT_COUNT: usize = 20;

/// Number of playlist search results to request from yt-dlp.
const PLAYLIST_SEARCH_RESULT_COUNT: usize = 10;

/// Preferred yt-dlp format selector for YouTube audio.
/// Prefers m4a/AAC (itag 140) for Symphonia compatibility, then mp4a codecs, then any bestaudio.
pub const YOUTUBE_AUDIO_FORMAT: &str = "bestaudio[ext=m4a]/bestaudio[acodec^=mp4a]/bestaudio";

pub struct YouTubeResolver;

impl YouTubeResolver {
    pub fn new() -> Self {
        Self
    }

    /// Check if yt-dlp is available on PATH.
    fn check_yt_dlp() -> Result<(), SourceError> {
        let result = Command::new("yt-dlp").arg("--version").output();

        match result {
            Ok(_) => Ok(()),
            Err(_) => Err(SourceError::DependencyMissing(
                "yt-dlp is not installed or not on PATH. Install it from https://github.com/yt-dlp/yt-dlp".to_string(),
            )),
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
            .or_else(|| value.get("url"))
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
            stream_url: None, // Resolved lazily on play
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
        let mut lines = stdout.lines().filter(|l| !l.trim().is_empty());

        // First line: playlist metadata
        let first_line = lines.next().ok_or_else(|| {
            SourceError::ResolveError("Empty yt-dlp playlist output".to_string())
        })?;

        let playlist_value: serde_json::Value = serde_json::from_str(first_line)
            .map_err(|e| SourceError::ResolveError(format!("Invalid playlist JSON: {}", e)))?;

        let source_id = playlist_value
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        let title = playlist_value
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("Untitled Playlist")
            .to_string();

        let thumbnail = playlist_value
            .get("thumbnail")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // Remaining lines: individual video entries
        let mut tracks = Vec::new();
        for line in lines {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if let Some(mut track) = Self::parse_track_from_json(line) {
                // Filter out non-audio entries (e.g., playlists nested inside)
                let entry_type = serde_json::from_str::<serde_json::Value>(line)
                    .ok()
                    .and_then(|v| v.get("_type").cloned())
                    .and_then(|v| v.as_str().map(|s| s.to_string()));
                if let Some(t) = entry_type {
                    if t != "video" && !t.is_empty() {
                        continue; // Skip non-video entries (playlists, etc.)
                    }
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

    fn search(&self, query: &str) -> Result<Vec<Track>, SourceError> {
        Self::check_yt_dlp()?;

        let output = Command::new("yt-dlp")
            .arg(format!("ytsearch{}:{}", SEARCH_RESULT_COUNT, query))
            .arg("--flat-playlist")
            .arg("--dump-json")
            .arg("--no-download")
            .arg("--no-playlist")
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

        // Build YouTube URL from video ID if it's not already a full URL
        let url = if id.starts_with("http") {
            id.to_string()
        } else {
            format!("https://www.youtube.com/watch?v={}", id)
        };

        // Get stream URL
        let url_output = Command::new("yt-dlp")
            .arg(&url)
            .arg("--get-url")
            .arg("--format")
            .arg(YOUTUBE_AUDIO_FORMAT)
            .arg("--no-playlist")
            .output()
            .map_err(|e| SourceError::NetworkError(e.to_string()))?;

        if !url_output.status.success() {
            let stderr = String::from_utf8_lossy(&url_output.stderr);
            return Err(SourceError::ResolveError(format!(
                "yt-dlp resolve failed: {}",
                stderr.trim()
            )));
        }

        let stream_url = String::from_utf8_lossy(&url_output.stdout)
            .lines()
            .next()
            .unwrap_or("")
            .trim()
            .to_string();

        if stream_url.is_empty() {
            return Err(SourceError::ResolveError(
                "No stream URL returned by yt-dlp".to_string(),
            ));
        }

        // Get metadata with --dump-json
        let json_output = Command::new("yt-dlp")
            .arg(&url)
            .arg("--dump-json")
            .arg("--no-download")
            .arg("--no-playlist")
            .output()
            .map_err(|e| SourceError::NetworkError(e.to_string()))?;

        let mut track = if json_output.status.success() {
            let json_str = String::from_utf8_lossy(&json_output.stdout);
            let first_line = json_str.lines().next().unwrap_or("");
            Self::parse_track_from_json(first_line).unwrap_or_else(|| Track {
                id: Uuid::new_v4().to_string(),
                source: Source::YouTube,
                source_id: id.to_string(),
                title: "Unknown".to_string(),
                artist: "Unknown".to_string(),
                album: None,
                duration: None,
                thumbnail: None,
                stream_url: None,
                local_path: None,
                playlist_id: None,
                metadata: HashMap::new(),
            })
        } else {
            Track {
                id: Uuid::new_v4().to_string(),
                source: Source::YouTube,
                source_id: id.to_string(),
                title: "Unknown".to_string(),
                artist: "Unknown".to_string(),
                album: None,
                duration: None,
                thumbnail: None,
                stream_url: None,
                local_path: None,
                playlist_id: None,
                metadata: HashMap::new(),
            }
        };

        track.stream_url = Some(stream_url);
        Ok(track)
    }

    fn search_playlists(&self, query: &str) -> Result<Vec<Playlist>, SourceError> {
        Self::check_yt_dlp()?;

        let output = Command::new("yt-dlp")
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

        let output = Command::new("yt-dlp")
            .arg(url)
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
        assert!(track.stream_url.is_none()); // Resolved lazily
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
        assert_eq!(SEARCH_RESULT_COUNT, 20);
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
        let playlist_json = r#"{"id":"PLminimal","title":"Minimal Playlist"}"#;
        let stdout = playlist_json.to_string();
        let playlist = YouTubeResolver::parse_playlist_dump(&stdout).unwrap();

        assert_eq!(playlist.source_id, "PLminimal");
        assert_eq!(playlist.title, "Minimal Playlist");
        assert!(playlist.thumbnail.is_none());
        assert_eq!(playlist.tracks.len(), 0);
    }
}
