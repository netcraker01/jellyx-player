//! Source resolvers for the TUI — uses yt-dlp for YouTube and SoundCloud.
//!
//! Implements the engine's `SourceResolver` trait by calling `yt-dlp`
//! as a subprocess. This is the same approach as the desktop, but without
//! the caching/metadata complexity — just enough to resolve stream URLs.

use std::process::Command;

use jellyx_core::models::playlist::Playlist;
use jellyx_core::models::source::Source;
use jellyx_core::models::track::Track;
use jellyx_engine::source_resolver::{SourceError, SourceResolver};

/// Find the yt-dlp binary.
fn yt_dlp_command() -> Result<Command, SourceError> {
    let cmd = which::which("yt-dlp")
        .or_else(|_| which::which("youtube-dl"))
        .map_err(|_| SourceError::DependencyMissing("yt-dlp not found".into()))?;
    Ok(Command::new(cmd))
}

/// YouTube resolver using yt-dlp.
pub struct YouTubeResolver;

impl SourceResolver for YouTubeResolver {
    fn source_type(&self) -> Source {
        Source::YouTube
    }

    fn search(&self, query: &str, offset: usize, limit: usize) -> Result<Vec<Track>, SourceError> {
        let end = offset + limit;
        let output = yt_dlp_command()?
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
            if let Ok(track) = parse_track_from_json(line, Source::YouTube) {
                tracks.push(track);
            }
        }
        Ok(tracks)
    }

    fn resolve(&self, id: &str) -> Result<Track, SourceError> {
        let url = if id.starts_with("http") {
            id.to_string()
        } else {
            format!("https://www.youtube.com/watch?v={id}")
        };

        let output = yt_dlp_command()?
            .arg(&url)
            .arg("--dump-json")
            .arg("--no-download")
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
        parse_track_from_json(stdout.trim(), Source::YouTube)
    }

    fn resolve_stream_url(&self, id: &str) -> Result<String, SourceError> {
        let url = if id.starts_with("http") {
            id.to_string()
        } else {
            format!("https://www.youtube.com/watch?v={id}")
        };

        let output = yt_dlp_command()?
            .arg(&url)
            .arg("--print")
            .arg("%(url)s")
            .arg("--no-download")
            .output()
            .map_err(|e| SourceError::NetworkError(e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(SourceError::ResolveError(format!(
                "yt-dlp stream URL failed: {}",
                stderr.trim()
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let url = stdout.trim().to_string();
        if url.is_empty() {
            Err(SourceError::ResolveError("no stream URL".into()))
        } else {
            Ok(url)
        }
    }
}

/// SoundCloud resolver using yt-dlp.
pub struct SoundCloudResolver;

impl SourceResolver for SoundCloudResolver {
    fn source_type(&self) -> Source {
        Source::SoundCloud
    }

    fn search(&self, query: &str, offset: usize, limit: usize) -> Result<Vec<Track>, SourceError> {
        let end = offset + limit;
        let output = yt_dlp_command()?
            .arg(format!("scsearch{}:{}", end, query))
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
            if let Ok(track) = parse_track_from_json(line, Source::SoundCloud) {
                tracks.push(track);
            }
        }
        Ok(tracks)
    }

    fn resolve(&self, id: &str) -> Result<Track, SourceError> {
        let url = if id.starts_with("http") {
            id.to_string()
        } else {
            format!("https://soundcloud.com/{id}")
        };

        let output = yt_dlp_command()?
            .arg(&url)
            .arg("--dump-json")
            .arg("--no-download")
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
        parse_track_from_json(stdout.trim(), Source::SoundCloud)
    }

    fn resolve_stream_url(&self, id: &str) -> Result<String, SourceError> {
        let url = if id.starts_with("http") {
            id.to_string()
        } else {
            format!("https://soundcloud.com/{id}")
        };

        let output = yt_dlp_command()?
            .arg(&url)
            .arg("--print")
            .arg("%(url)s")
            .arg("--no-download")
            .output()
            .map_err(|e| SourceError::NetworkError(e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(SourceError::ResolveError(format!(
                "yt-dlp stream URL failed: {}",
                stderr.trim()
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let url = stdout.trim().to_string();
        if url.is_empty() {
            Err(SourceError::ResolveError("no stream URL".into()))
        } else {
            Ok(url)
        }
    }

    fn search_playlists(&self, query: &str) -> Result<Vec<Playlist>, SourceError> {
        let output = yt_dlp_command()?
            .arg(format!("scsearch10:{query}"))
            .arg("--flat-playlist")
            .arg("--dump-json")
            .arg("--no-download")
            .output()
            .map_err(|e| SourceError::NetworkError(e.to_string()))?;

        if !output.status.success() {
            return Ok(Vec::new());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut playlists = Vec::new();
        for line in stdout.lines() {
            if let Ok(playlist) = parse_playlist_from_json(line) {
                playlists.push(playlist);
            }
        }
        Ok(playlists)
    }
}

/// Parse a yt-dlp --dump-json line into a Track.
fn parse_track_from_json(line: &str, source: Source) -> Result<Track, SourceError> {
    let json: serde_json::Value = serde_json::from_str(line)
        .map_err(|e| SourceError::ResolveError(format!("parse JSON: {e}")))?;

    let id = json
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();

    let title = json
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown")
        .to_string();

    let artist = json
        .get("uploader")
        .or_else(|| json.get("channel"))
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown")
        .to_string();

    let thumbnail = json
        .get("thumbnail")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let duration = json
        .get("duration")
        .and_then(|v| v.as_f64())
        .map(|d| d as f64);

    let stream_url = json
        .get("url")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    Ok(Track {
        id: id.clone(),
        source,
        source_id: id,
        title,
        artist,
        album: None,
        duration,
        thumbnail,
        stream_url,
        local_path: None,
        playlist_id: None,
        metadata: std::collections::HashMap::new(),
    })
}

/// Parse a yt-dlp --dump-json line into a Playlist.
fn parse_playlist_from_json(line: &str) -> Result<Playlist, SourceError> {
    let json: serde_json::Value = serde_json::from_str(line)
        .map_err(|e| SourceError::ResolveError(format!("parse JSON: {e}")))?;

    let id = json
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();
    let source_id = id.clone();

    let title = json
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown")
        .to_string();

    Ok(Playlist {
        id,
        source: Source::SoundCloud,
        source_id,
        title,
        thumbnail: None,
        track_count: 0,
        tracks: Vec::new(),
    })
}
