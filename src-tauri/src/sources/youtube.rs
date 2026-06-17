//! YouTube source resolver via yt-dlp.
//!
//! Uses yt-dlp to search YouTube and resolve stream URLs.
//! yt-dlp handles extraction, format selection, and URL resolution.

use std::collections::HashMap;
use std::process::Command;

use uuid::Uuid;

use super::SourceResolver;
use crate::errors::types::SourceError;
use crate::models::source::Source;
use crate::models::track::Track;

/// Number of search results to request from yt-dlp.
const SEARCH_RESULT_COUNT: usize = 5;

pub struct YouTubeResolver;

impl YouTubeResolver {
    pub fn new() -> Self {
        Self
    }

    /// Check if yt-dlp is available on PATH.
    fn check_yt_dlp() -> Result<(), SourceError> {
        let result = Command::new("yt-dlp")
            .arg("--version")
            .output();

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
        let artist = value.get("artist")
            .or_else(|| value.get("uploader"))
            .or_else(|| value.get("channel"))
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown")
            .to_string();

        let duration = value.get("duration")
            .and_then(|v| v.as_f64());

        let thumbnail = value.get("thumbnail")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let album = value.get("album")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // webpage_url is the full YouTube URL for resolve
        let webpage_url = value.get("webpage_url")
            .or_else(|| value.get("url"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let mut metadata = HashMap::new();
        if let Some(url) = webpage_url {
            metadata.insert("webpage_url".to_string(), url);
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
            metadata,
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
            .arg("--dump-json")
            .arg("--no-download")
            .arg("--no-playlist")
            .output()
            .map_err(|e| SourceError::NetworkError(e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(SourceError::NetworkError(format!(
                "yt-dlp search failed: {}", stderr.trim()
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
            .arg("bestaudio")
            .arg("--no-playlist")
            .output()
            .map_err(|e| SourceError::NetworkError(e.to_string()))?;

        if !url_output.status.success() {
            let stderr = String::from_utf8_lossy(&url_output.stderr);
            return Err(SourceError::ResolveError(format!(
                "yt-dlp resolve failed: {}", stderr.trim()
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
                metadata: HashMap::new(),
            }
        };

        track.stream_url = Some(stream_url);
        Ok(track)
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
        let json = r#"{"id":"dQw4w9WgXcQ","title":"Rick Astley - Never Gonna Give You Up","artist":"Rick Astley","duration":212.0,"thumbnail":"https://img.youtube.com/vi/dQw4w9WgXcQ/0.jpg","webpage_url":"https://www.youtube.com/watch?v=dQw4w9WgXcQ"}"#;
        let track = YouTubeResolver::parse_track_from_json(json).unwrap();
        assert_eq!(track.source_id, "dQw4w9WgXcQ");
        assert_eq!(track.title, "Rick Astley - Never Gonna Give You Up");
        assert_eq!(track.artist, "Rick Astley");
        assert_eq!(track.duration, Some(212.0));
        assert_eq!(track.source, Source::YouTube);
        assert!(track.stream_url.is_none()); // Resolved lazily
        assert!(track.metadata.contains_key("webpage_url"));
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
        assert_eq!(SEARCH_RESULT_COUNT, 5);
    }
}