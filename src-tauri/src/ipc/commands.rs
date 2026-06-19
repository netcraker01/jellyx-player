//! Tauri command handlers — the IPC bridge between Svelte frontend and Rust backend.
//!
//! All `#[tauri::command]` functions delegate to PlaybackService or LibraryService.
//! AppState holds Arc<PlaybackService> and Arc<LibraryService> shared across commands.

use std::sync::{Arc, Mutex};

use crate::errors::types::AppError;
use crate::ipc::dto::{
    AlbumDetail, ArtistDetail, GroupedSearchResult, HomeSnapshot, RecommendationItem, SearchFilter,
};
use crate::library::LibraryService;
use crate::models::track::Track;
use crate::persistence::models::{FavoriteEntry, HistoryEntry, LocalTrackEntry, WatchedFolder};
use crate::playback::service::PlaybackService;
use crate::sources::local::{ScanResult, ScannerService};

/// Application state shared across Tauri commands.
/// PlaybackService is the single authority for all playback operations.
/// LibraryService manages favorites and history.
/// ScannerService manages local file scanning.
/// fft_channel holds the Tauri Channel for binary FFT streaming.
pub struct AppState {
    pub playback: Arc<PlaybackService>,
    pub library: Arc<LibraryService>,
    pub scanner: Arc<ScannerService>,
    /// Binary FFT streaming channel — set by `start_fft_stream`, used by FFT thread.
    pub fft_channel: Arc<Mutex<Option<tauri::ipc::Channel<Vec<u8>>>>>,
}

#[tauri::command]
pub fn play(state: tauri::State<AppState>, url: &str) -> Result<(), AppError> {
    state.playback.play(url)
}

#[tauri::command]
pub fn play_local(state: tauri::State<AppState>, path: &str) -> Result<(), AppError> {
    state.playback.play_local(path)
}

#[tauri::command]
pub fn pause(state: tauri::State<AppState>) -> Result<(), AppError> {
    state.playback.pause()
}

#[tauri::command]
pub fn resume(state: tauri::State<AppState>) -> Result<(), AppError> {
    state.playback.resume()
}

#[tauri::command]
pub fn next(state: tauri::State<AppState>) -> Result<(), AppError> {
    state.playback.next()
}

#[tauri::command]
pub fn previous(state: tauri::State<AppState>) -> Result<(), AppError> {
    state.playback.previous()
}

#[tauri::command]
pub fn seek(state: tauri::State<AppState>, position: f64) -> Result<(), AppError> {
    state.playback.seek(position)
}

#[tauri::command]
pub fn set_volume(state: tauri::State<AppState>, volume: f32) -> Result<(), AppError> {
    state.playback.set_volume(volume)
}

#[tauri::command]
pub fn search(state: tauri::State<AppState>, query: &str) -> Result<Vec<Track>, AppError> {
    state.playback.search(query)
}

/// Search with grouped results (songs, artists, albums).
/// Optional filter: "songs", "artists", "albums", or None for all.
#[tauri::command]
pub fn search_grouped(
    state: tauri::State<AppState>,
    query: &str,
    filter: Option<&str>,
) -> Result<GroupedSearchResult, AppError> {
    let parsed_filter = filter
        .map(|f| match f.to_lowercase().as_str() {
            "songs" => Ok(SearchFilter::Songs),
            "artists" => Ok(SearchFilter::Artists),
            "albums" => Ok(SearchFilter::Albums),
            _ => Err(crate::errors::types::ValidationError::InvalidInput(
                format!(
                    "invalid search filter: {}. Expected songs, artists, or albums",
                    f
                ),
            )),
        })
        .transpose()?;
    state.library.search_grouped(query, parsed_filter)
}

/// Get full artist detail by artist ID.
#[tauri::command]
pub fn get_artist_detail(
    state: tauri::State<AppState>,
    id: &str,
) -> Result<ArtistDetail, AppError> {
    state.library.get_artist_detail(id)
}

/// Get full album detail by album ID.
#[tauri::command]
pub fn get_album_detail(state: tauri::State<AppState>, id: &str) -> Result<AlbumDetail, AppError> {
    state.library.get_album_detail(id)
}

/// Play all tracks in an album, replacing the current queue.
#[tauri::command]
pub fn play_album(state: tauri::State<AppState>, album_id: &str) -> Result<(), AppError> {
    state.playback.play_album(album_id)
}

#[tauri::command]
pub fn add_to_queue(state: tauri::State<AppState>, track_id: &str) -> Result<(), AppError> {
    state.playback.add_to_queue(track_id)
}

/// Remove a track from the queue by its Helix track ID.
#[tauri::command]
pub fn remove_from_queue(state: tauri::State<AppState>, track_id: &str) -> Result<(), AppError> {
    state.playback.remove_from_queue(track_id)
}

/// Clear the entire queue and stop playback.
#[tauri::command]
pub fn clear_queue(state: tauri::State<AppState>) -> Result<(), AppError> {
    state.playback.clear_queue()
}

/// Insert a selected track immediately after the current queue position.
#[tauri::command]
pub fn play_next(state: tauri::State<AppState>, track_id: &str) -> Result<(), AppError> {
    state.playback.play_next(track_id)
}

/// Get the current queue as a full QueueState snapshot.
#[tauri::command]
pub fn get_queue(
    state: tauri::State<AppState>,
) -> Result<crate::playback::state::QueueState, AppError> {
    state.playback.get_queue()
}

/// Get the app version — no state needed.
#[tauri::command]
pub fn get_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

// ── Library commands ────────────────────────────────────────────────

/// Get all favorited tracks, ordered by most recently added first.
#[tauri::command]
pub fn get_favorites(state: tauri::State<AppState>) -> Result<Vec<FavoriteEntry>, AppError> {
    state.library.get_favorites()
}

/// Add a track to favorites. Expects a Track object in the payload.
#[tauri::command]
pub fn add_favorite(state: tauri::State<AppState>, track: Track) -> Result<(), AppError> {
    state.library.add_favorite(track)
}

/// Remove a track from favorites by its Helix track ID.
#[tauri::command]
pub fn remove_favorite(state: tauri::State<AppState>, track_id: &str) -> Result<(), AppError> {
    state.library.remove_favorite(track_id)
}

/// Get play history, ordered by most recent first (max 100 entries).
#[tauri::command]
pub fn get_history(state: tauri::State<AppState>) -> Result<Vec<HistoryEntry>, AppError> {
    state.library.get_history()
}

/// Clear all play history.
#[tauri::command]
pub fn clear_history(state: tauri::State<AppState>) -> Result<(), AppError> {
    state.library.clear_history()
}

/// Toggle a track's favorite state by its Helix track ID.
///
/// The command first tries to find the track in the current queue so the full
/// Track payload is available. If the track is not queued, it falls back to
/// resolving it from the source registry.
#[tauri::command]
pub fn toggle_favorite(state: tauri::State<AppState>, track_id: String) -> Result<bool, AppError> {
    let track = state.playback.get_track_by_id(&track_id)?;
    state.library.toggle_favorite(&track)
}

/// Check whether a track is currently favorited.
#[tauri::command]
pub fn is_favorite(state: tauri::State<AppState>, track_id: String) -> Result<bool, AppError> {
    state
        .library
        .favorite_exists(&track_id)
        .map_err(AppError::from)
}

/// Set shuffle mode on or off.
#[tauri::command]
pub fn set_shuffle(state: tauri::State<AppState>, enabled: bool) -> Result<(), AppError> {
    state.playback.set_shuffle(enabled)
}

/// Set repeat mode by name ("Off", "All", or "One").
#[tauri::command]
pub fn set_repeat(state: tauri::State<AppState>, mode: String) -> Result<(), AppError> {
    state.playback.set_repeat_from_string(&mode)
}

/// Cycle repeat mode Off -> All -> One -> Off.
#[tauri::command]
pub fn cycle_repeat(state: tauri::State<AppState>) -> Result<String, AppError> {
    let mode = state.playback.cycle_repeat()?;
    Ok(format!("{:?}", mode))
}

// ── Local Scanner commands ──────────────────────────────────────────

/// Scan a folder for audio files and add to local library.
#[tauri::command]
pub fn scan_folder(
    state: tauri::State<AppState>,
    folder_path: &str,
) -> Result<ScanResult, AppError> {
    state.scanner.scan_folder(folder_path)
}

/// Get all local tracks, optionally filtered by folder path.
#[tauri::command]
pub fn get_local_tracks(
    state: tauri::State<AppState>,
    folder_path: Option<&str>,
) -> Result<Vec<LocalTrackEntry>, AppError> {
    state.scanner.get_tracks(folder_path)
}

/// Get all watched folders.
#[tauri::command]
pub fn get_watched_folders(state: tauri::State<AppState>) -> Result<Vec<WatchedFolder>, AppError> {
    state.scanner.get_watched_folders()
}

/// Get the Home snapshot with recently played tracks and recommendations.
#[tauri::command]
pub fn get_home_snapshot(state: tauri::State<AppState>) -> Result<HomeSnapshot, AppError> {
    state.library.get_home_snapshot()
}

/// Get heavy Home recommendations computed from history, favorites, and local library.
#[tauri::command]
pub fn get_home_recommendations(
    state: tauri::State<AppState>,
) -> Result<Vec<RecommendationItem>, AppError> {
    state.library.get_home_recommendations()
}

/// Remove a watched folder and its associated tracks.
#[tauri::command]
pub fn remove_watched_folder(
    state: tauri::State<AppState>,
    folder_path: &str,
) -> Result<(), AppError> {
    state.scanner.remove_folder(folder_path)
}

/// Start streaming binary FFT data to the frontend.
///
/// The frontend creates a `Channel<Uint8Array>` and passes it to this command.
/// The Channel is stored in `AppState.fft_channel` so the FFT thread can send
/// binary frames at ~60fps. The Channel is cleared when playback stops.
#[tauri::command]
pub fn start_fft_stream(
    state: tauri::State<AppState>,
    channel: tauri::ipc::Channel<Vec<u8>>,
) -> Result<(), AppError> {
    let mut fft_ch = state.fft_channel.lock().map_err(|_| AppError {
        code: "UNKNOWN_ERROR".into(),
        details: Some("mutex lock".into()),
    })?;
    *fft_ch = Some(channel);
    Ok(())
}
