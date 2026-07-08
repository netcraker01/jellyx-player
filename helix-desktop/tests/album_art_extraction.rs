//! Integration test for album art extraction from real audio files.
//!
//! Tests the full `scan_folder` → `extract_metadata` → `cache_art` pipeline
//! with actual audio files containing embedded art.
//!
//! Gated behind `integration` feature to avoid requiring fixture files in CI.
//!
//! # Fixture requirements
//!
//! Place these files in `tests/fixtures/`:
//! - `with_art.mp3` — MP3 file with embedded ID3v2 FrontCover JPEG
//! - `without_art.wav` — WAV file with no embedded visuals
//!
//! # Running
//!
//! ```sh
//! cargo test --features integration --test album_art_extraction
//! ```

#[cfg(feature = "integration")]
mod integration {
    use helix_lib::persistence::db::Database;
    use helix_lib::sources::local::ScannerService;
    use std::sync::Arc;

    /// Verify that scanning a folder with embedded art populates Track.thumbnail.
    #[test]
    fn scan_folder_with_art_populates_thumbnail() {
        let fixtures_dir = std::path::Path::new("tests/fixtures");
        let art_file = fixtures_dir.join("with_art.mp3");
        if !art_file.exists() {
            eprintln!("SKIP: tests/fixtures/with_art.mp3 not found");
            return;
        }

        let db = Arc::new(Database::open_in_memory().unwrap());
        let service = ScannerService::new(db);

        let result = service.scan_folder(fixtures_dir.to_str().unwrap());
        assert!(result.is_ok(), "scan should succeed");

        let tracks = service.get_tracks(None).unwrap();
        let entry = tracks.iter().find(|t| t.file_path.contains("with_art"));
        assert!(entry.is_some(), "should find the with_art track");

        // The serialized track JSON should contain a non-null thumbnail
        let json = &entry.unwrap().track_json;
        assert!(
            json.contains("\"thumbnail\""),
            "track with art should have thumbnail field"
        );
    }

    /// Verify that scanning a folder without embedded art leaves Track.thumbnail as None.
    #[test]
    fn scan_folder_without_art_has_no_thumbnail() {
        let fixtures_dir = std::path::Path::new("tests/fixtures");
        let no_art_file = fixtures_dir.join("without_art.wav");
        if !no_art_file.exists() {
            eprintln!("SKIP: tests/fixtures/without_art.wav not found");
            return;
        }

        let db = Arc::new(Database::open_in_memory().unwrap());
        let service = ScannerService::new(db);

        let result = service.scan_folder(fixtures_dir.to_str().unwrap());
        assert!(result.is_ok(), "scan should succeed");

        let tracks = service.get_tracks(None).unwrap();
        let entry = tracks.iter().find(|t| t.file_path.contains("without_art"));
        assert!(entry.is_some(), "should find the without_art track");

        // The serialized track JSON should NOT contain a thumbnail field
        // (skip_serializing_if = "Option::is_none" removes it)
        let json = &entry.unwrap().track_json;
        assert!(
            !json.contains("\"thumbnail\""),
            "track without art should not have thumbnail"
        );
    }
}
