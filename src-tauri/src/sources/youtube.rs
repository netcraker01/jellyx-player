//! YouTube source resolver via yt-dlp.
//!
//! Uses yt-dlp to search YouTube and resolve stream URLs.
//! yt-dlp handles extraction, format selection, and URL resolution.

use super::SourceResolver;
use crate::errors::types::SourceError;
use crate::models::track::Track;
use std::process::Command;

pub struct YouTubeResolver;

impl YouTubeResolver {
    pub fn new() -> Self {
        Self
    }
}

impl SourceResolver for YouTubeResolver {
    fn search(&self, query: &str) -> Result<Vec<Track>, SourceError> {
        // yt-dlp "ytsearch5:query" --dump-json --no-download
        let output = Command::new("yt-dlp")
            .arg(format!("ytsearch5:{}", query))
            .arg("--dump-json")
            .arg("--no-download")
            .arg("--no-playlist")
            .output()
            .map_err(|e| SourceError::NetworkError(e.to_string()))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let tracks = Vec::new();

        for line in stdout.lines() {
            // TODO: parse JSON lines into Track structs
            // let json: serde_json::Value = serde_json::from_str(line).unwrap();
            // tracks.push(Track { source: Source::YouTube, ... });
            let _ = line; // placeholder
        }

        Ok(tracks)
    }

    fn resolve(&self, id: &str) -> Result<Track, SourceError> {
        // yt-dlp "https://youtube.com/watch?v=ID" --get-url --get-title
        let _ = id;
        Err(SourceError::UnsupportedSource) // TODO
    }
}