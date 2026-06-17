//! Local file scanner — walks directories and extracts metadata from audio files.
//!
//! `ScannerService` owns an `Arc<Database>` and provides methods to:
//! - Scan a folder (walk + extract + persist)
//! - Retrieve local tracks and watched folders
//! - Remove a watched folder and its tracks
//!
//! Uses `walkdir` for directory traversal and `symphonia` for metadata extraction.

use std::collections::HashMap;
use std::fs::File;
use std::path::Path;
use std::sync::Arc;
use std::time::UNIX_EPOCH;

use symphonia::core::formats::FormatOptions;
use symphonia::core::formats::probe::Hint;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::meta::StandardTag;
use uuid::Uuid;
use walkdir::WalkDir;

use crate::errors::types::{AppError, LibraryError, ScannerError};
use crate::models::source::Source;
use crate::models::track::Track;
use crate::persistence::db::Database;
use crate::persistence::models::{LocalTrackEntry, WatchedFolder};

/// Supported audio file extensions for scanning.
const SUPPORTED_EXTENSIONS: &[&str] = &["mp3", "flac", "ogg", "wav", "aac", "m4a"];

/// Result summary returned after a scan operation.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanResult {
    pub folder_path: String,
    pub files_scanned: u32,
    pub files_added: u32,
    pub files_updated: u32,
    pub files_skipped: u32,
    pub errors: u32,
}

/// Service for scanning local music directories.
pub struct ScannerService {
    db: Arc<Database>,
}

impl ScannerService {
    /// Create a new ScannerService backed by the given Database.
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    /// Check if a file extension is a supported audio format.
    fn is_supported_extension(path: &Path) -> bool {
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| SUPPORTED_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
            .unwrap_or(false)
    }

    /// Get the file's modification time as a Unix timestamp string.
    fn file_mtime(path: &Path) -> Option<String> {
        std::fs::metadata(path)
            .ok()
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_secs().to_string())
    }

    /// Extract metadata from an audio file using symphonia probe.
    ///
    /// Returns a Track with tags populated from the file's metadata.
    /// Falls back to filename-based metadata when tags are unavailable.
    fn extract_metadata(path: &Path) -> Result<Track, ScannerError> {
        let file = File::open(path).map_err(|e| {
            ScannerError::MetadataError(format!("failed to open file {:?}: {}", path, e))
        })?;

        let mss = MediaSourceStream::new(Box::new(file), Default::default());

        let mut hint = Hint::new();
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            hint.with_extension(ext);
        }

        let format_opts: FormatOptions = Default::default();
        let meta_opts: MetadataOptions = Default::default();

        // Probe the media source stream for a format
        let mut format_reader = symphonia::default::get_probe()
            .probe(&hint, mss, format_opts, meta_opts)
            .map_err(|e| {
                ScannerError::MetadataError(format!("failed to probe {:?}: {}", path, e))
            })?;

        let mut title: Option<String> = None;
        let mut artist: Option<String> = None;
        let mut album: Option<String> = None;
        let mut duration: Option<f64> = None;

        // Extract metadata from tags
        let metadata = format_reader.metadata();
        if let Some(revision) = metadata.current() {
            for tag in &revision.media.tags {
                if let Some(ref std_tag) = tag.std {
                    match std_tag {
                        StandardTag::TrackTitle(t) => {
                            title = Some(t.to_string());
                        }
                        StandardTag::Album(a) => {
                            album = Some(a.to_string());
                        }
                        StandardTag::Artist(a) => {
                            artist = Some(a.to_string());
                        }
                        _ => {}
                    }
                }
            }
        }

        // Extract duration from track info
        let tracks = format_reader.tracks();
        if let Some(track) = tracks.first() {
            if let Some(tb) = track.time_base {
                if let Some(n_frames) = track.num_frames {
                    // duration = n_frames * time_base
                    let tb_secs = tb.numer.get() as f64 / tb.denom.get() as f64;
                    duration = Some(n_frames as f64 * tb_secs);
                }
            }
        }

        // Fallback: use filename (without extension) as title
        let file_stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Unknown")
            .to_string();

        let final_title = title.unwrap_or_else(|| file_stem.clone());
        let final_artist = artist.unwrap_or_else(|| "Unknown".to_string());
        let final_album = album;

        let path_str = path.to_string_lossy().to_string();

        Ok(Track {
            id: Uuid::new_v4().to_string(),
            source: Source::Local,
            source_id: path_str.clone(),
            title: final_title,
            artist: final_artist,
            album: final_album,
            duration,
            thumbnail: None,
            stream_url: None,
            local_path: Some(path_str.clone()),
            metadata: HashMap::new(),
        })
    }

    /// Scan a folder: walk directory, extract metadata, persist tracks.
    ///
    /// If the folder is already watched, performs an incremental scan
    /// (skips files whose mtime hasn't changed).
    pub fn scan_folder(&self, folder_path: &str) -> Result<ScanResult, AppError> {
        let path = Path::new(folder_path);
        if !path.is_dir() {
            return Err(AppError::from(ScannerError::WalkError(
                format!("path is not a directory: {}", folder_path),
            )));
        }

        // Register folder as watched
        if !self.db.watched_folder_exists(folder_path).map_err(AppError::from)? {
            self.db.insert_watched_folder(folder_path).map_err(AppError::from)?;
        }

        let mut result = ScanResult {
            folder_path: folder_path.to_string(),
            files_scanned: 0,
            files_added: 0,
            files_updated: 0,
            files_skipped: 0,
            errors: 0,
        };

        // Get existing tracks for incremental scan comparison
        let existing_tracks = self.db.get_local_tracks(Some(folder_path))
            .map_err(AppError::from)?;
        let existing_map: HashMap<String, String> = existing_tracks.iter()
            .map(|e| (e.file_path.clone(), e.file_modified_at.clone().unwrap_or_default()))
            .collect();

        for entry in WalkDir::new(folder_path).follow_links(true) {
            let entry = match entry {
                Ok(e) => e,
                Err(_) => {
                    result.errors += 1;
                    continue;
                }
            };

            let path = entry.path();

            // Skip hidden directories/files
            if path.components().any(|c| {
                c.as_os_str().to_str().map(|s| s.starts_with('.')).unwrap_or(false)
            }) {
                continue;
            }

            // Skip directories
            if !path.is_file() {
                continue;
            }

            // Skip unsupported extensions
            if !Self::is_supported_extension(path) {
                continue;
            }

            result.files_scanned += 1;

            let path_str = path.to_string_lossy().to_string();
            let current_mtime = Self::file_mtime(path);

            // Incremental scan: skip if mtime unchanged
            if let Some(ref mtime) = current_mtime {
                if let Some(existing_mtime) = existing_map.get(&path_str) {
                    if existing_mtime == mtime {
                        result.files_skipped += 1;
                        continue;
                    }
                }
            }

            // Extract metadata
            match Self::extract_metadata(path) {
                Ok(track) => {
                    let is_new = !existing_map.contains_key(&path_str);
                    let mtime_str = current_mtime.as_deref();

                    match self.db.upsert_local_track(&path_str, &track, folder_path, mtime_str) {
                        Ok(()) => {
                            if is_new {
                                result.files_added += 1;
                            } else {
                                result.files_updated += 1;
                            }
                        }
                        Err(_) => {
                            result.errors += 1;
                        }
                    }
                }
                Err(_) => {
                    result.errors += 1;
                }
            }
        }

        // Update folder scan time
        let _ = self.db.update_folder_scan_time(folder_path);

        Ok(result)
    }

    /// Get all local tracks, optionally filtered by folder.
    pub fn get_tracks(&self, folder_path: Option<&str>) -> Result<Vec<LocalTrackEntry>, AppError> {
        self.db.get_local_tracks(folder_path).map_err(AppError::from)
    }

    /// Get all watched folders.
    pub fn get_watched_folders(&self) -> Result<Vec<WatchedFolder>, AppError> {
        self.db.get_watched_folders().map_err(AppError::from)
    }

    /// Remove a watched folder and its associated tracks.
    pub fn remove_folder(&self, folder_path: &str) -> Result<(), AppError> {
        let removed = self.db.remove_watched_folder(folder_path).map_err(AppError::from)?;
        if !removed {
            return Err(AppError::from(LibraryError::NotFound(folder_path.to_string())));
        }
        // CASCADE should handle local_tracks deletion, but clean up explicitly
        let _ = self.db.delete_local_tracks_by_folder(folder_path);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supported_extensions_includes_common_formats() {
        assert!(ScannerService::is_supported_extension(Path::new("song.mp3")));
        assert!(ScannerService::is_supported_extension(Path::new("song.flac")));
        assert!(ScannerService::is_supported_extension(Path::new("song.ogg")));
        assert!(ScannerService::is_supported_extension(Path::new("song.wav")));
        assert!(ScannerService::is_supported_extension(Path::new("song.aac")));
        assert!(ScannerService::is_supported_extension(Path::new("song.m4a")));
    }

    #[test]
    fn unsupported_extensions_rejected() {
        assert!(!ScannerService::is_supported_extension(Path::new("song.txt")));
        assert!(!ScannerService::is_supported_extension(Path::new("song.mp4")));
        assert!(!ScannerService::is_supported_extension(Path::new("song.pdf")));
        assert!(!ScannerService::is_supported_extension(Path::new("noext")));
    }

    #[test]
    fn extension_check_case_insensitive() {
        assert!(ScannerService::is_supported_extension(Path::new("song.MP3")));
        assert!(ScannerService::is_supported_extension(Path::new("song.Flac")));
        assert!(ScannerService::is_supported_extension(Path::new("song.OGG")));
    }

    #[test]
    fn scan_result_serializes_camel_case() {
        let result = ScanResult {
            folder_path: "/music".to_string(),
            files_scanned: 10,
            files_added: 5,
            files_updated: 3,
            files_skipped: 2,
            errors: 0,
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"folderPath\""));
        assert!(json.contains("\"filesScanned\""));
        assert!(json.contains("\"filesAdded\""));
        assert!(json.contains("\"filesUpdated\""));
        assert!(json.contains("\"filesSkipped\""));
    }

    #[test]
    fn scanner_service_new_creates_instance() {
        let db = Database::open_in_memory().unwrap();
        let _service = ScannerService::new(Arc::new(db));
    }

    #[test]
    fn scan_nonexistent_folder_returns_error() {
        let db = Database::open_in_memory().unwrap();
        let service = ScannerService::new(Arc::new(db));
        let result = service.scan_folder("/nonexistent/path/that/does/not/exist");
        assert!(result.is_err());
    }

    #[test]
    fn scan_empty_folder_succeeds() {
        let db = Database::open_in_memory().unwrap();
        let service = ScannerService::new(Arc::new(db));

        // Create a temp empty directory
        let temp_dir = std::env::temp_dir().join("helix_test_scan_empty");
        std::fs::create_dir_all(&temp_dir).unwrap();

        let result = service.scan_folder(temp_dir.to_str().unwrap());
        assert!(result.is_ok());
        let scan = result.unwrap();
        assert_eq!(scan.files_scanned, 0);
        assert_eq!(scan.files_added, 0);

        // Clean up
        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn get_watched_folders_initially_empty() {
        let db = Database::open_in_memory().unwrap();
        let service = ScannerService::new(Arc::new(db));
        let folders = service.get_watched_folders().unwrap();
        assert!(folders.is_empty());
    }
}