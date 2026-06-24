//! Tauri command handlers — the IPC bridge between Svelte frontend and Rust backend.
//!
//! All `#[tauri::command]` functions delegate to PlaybackService or LibraryService.
//! AppState holds Arc<PlaybackService> and Arc<LibraryService> shared across commands.
//!
//! ## Async / Blocking hygiene
//!
//! Commands that invoke subprocess calls (yt-dlp), network I/O, or heavy file
//! operations are marked `async` and offload their blocking work via
//! `tokio::task::spawn_blocking`. This prevents the Tauri main-thread pool
//! from stalling — the UI stays responsive even when yt-dlp takes seconds to
//! return search results.
//!
//! Commands that only touch in-memory state or fast SQLite queries remain
//! synchronous (plain `fn`) since they complete in microseconds.

use std::sync::{Arc, Mutex};

use crate::errors::types::AppError;
use crate::ipc::dto::{
    AlbumDetail, ArtistDetail, ArtistSummary, GroupedSearchResult, HomeSnapshot, RecommendationItem, SearchFilter,
    UserPlaylist as UserPlaylistDto,
    PlaylistTrackEntry as PlaylistTrackEntryDto,
    ArtistFavorite as ArtistFavoriteDto,
};
use crate::library::{LibraryService, PlaylistService, SettingsService};
use crate::models::playlist::Playlist;
use crate::models::track::Track;
use crate::persistence::models::{HistoryEntry, LocalTrackEntry, WatchedFolder};
use crate::playback::service::PlaybackService;
use crate::sources::local::{ScanResult, ScannerService};

/// Application state shared across Tauri commands.
/// PlaybackService is the single authority for all playback operations.
/// LibraryService manages favorites and history.
/// PlaylistService manages user-created playlists and artist favorites.
/// SettingsService manages source enable/disable state.
/// ScannerService manages local file scanning.
/// fft_channel holds the Tauri Channel for binary FFT streaming.
pub struct AppState {
    pub playback: Arc<PlaybackService>,
    pub library: Arc<LibraryService>,
    pub playlist: Arc<PlaylistService>,
    pub settings: Arc<SettingsService>,
    pub scanner: Arc<ScannerService>,
    /// Binary FFT streaming channel — set by `start_fft_stream`, used by FFT thread.
    pub fft_channel: Arc<Mutex<Option<tauri::ipc::Channel<Vec<u8>>>>>,
}

// ── Synchronous commands (fast, in-memory or SQLite) ──────────────────

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

// ── Library commands (fast SQLite) ──────────────────────────────────

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

// ── Playlist commands (fast SQLite) ──────────────────────────────

#[tauri::command]
pub fn create_playlist(state: tauri::State<AppState>, title: String) -> Result<UserPlaylistDto, AppError> {
    let pl = state.playlist.create_playlist(&title)?;
    Ok(UserPlaylistDto {
        id: pl.id,
        title: pl.title,
        created_at: pl.created_at,
        updated_at: pl.updated_at,
    })
}

#[tauri::command]
pub fn rename_playlist(state: tauri::State<AppState>, id: String, title: String) -> Result<(), AppError> {
    state.playlist.rename_playlist(&id, &title)
}

#[tauri::command]
pub fn delete_playlist(state: tauri::State<AppState>, id: String) -> Result<(), AppError> {
    state.playlist.delete_playlist(&id)
}

#[tauri::command]
pub fn get_all_playlists(state: tauri::State<AppState>) -> Result<Vec<UserPlaylistDto>, AppError> {
    let playlists = state.playlist.get_all_playlists()?;
    Ok(playlists.into_iter().map(|pl| UserPlaylistDto {
        id: pl.id,
        title: pl.title,
        created_at: pl.created_at,
        updated_at: pl.updated_at,
    }).collect())
}

#[tauri::command]
pub fn get_recent_playlists(state: tauri::State<AppState>, limit: Option<u32>) -> Result<Vec<UserPlaylistDto>, AppError> {
    let playlists = state.playlist.get_recent_playlists(limit.unwrap_or(5))?;
    Ok(playlists.into_iter().map(|pl| UserPlaylistDto {
        id: pl.id,
        title: pl.title,
        created_at: pl.created_at,
        updated_at: pl.updated_at,
    }).collect())
}

#[tauri::command]
pub fn search_user_playlists(state: tauri::State<AppState>, query: String) -> Result<Vec<UserPlaylistDto>, AppError> {
    let playlists = state.playlist.search_playlists(&query)?;
    Ok(playlists.into_iter().map(|pl| UserPlaylistDto {
        id: pl.id,
        title: pl.title,
        created_at: pl.created_at,
        updated_at: pl.updated_at,
    }).collect())
}

#[tauri::command]
pub fn add_track_to_playlist(state: tauri::State<AppState>, playlist_id: String, track: Track) -> Result<(), AppError> {
    state.playlist.add_track_to_playlist(&playlist_id, &track)
}

#[tauri::command]
pub fn remove_track_from_playlist(state: tauri::State<AppState>, playlist_id: String, position: i64) -> Result<(), AppError> {
    state.playlist.remove_track_from_playlist(&playlist_id, position)
}

#[tauri::command]
pub fn get_playlist_tracks(state: tauri::State<AppState>, playlist_id: String) -> Result<Vec<PlaylistTrackEntryDto>, AppError> {
    let entries = state.playlist.get_playlist_tracks(&playlist_id)?;
    Ok(entries.into_iter().map(|e| PlaylistTrackEntryDto {
        playlist_id: e.playlist_id,
        position: e.position,
        track: e.track,
        added_at: e.added_at,
    }).collect())
}

#[tauri::command]
pub fn count_playlist_tracks(state: tauri::State<AppState>, playlist_id: String) -> Result<u32, AppError> {
    state.playlist.count_playlist_tracks(&playlist_id)
}

// ── Artist Favorite commands (fast SQLite) ──────────────────────────

#[tauri::command]
pub fn add_artist_favorite(
    state: tauri::State<AppState>,
    artist_id: String,
    artist_name: String,
    thumbnail: Option<String>,
) -> Result<(), AppError> {
    state.playlist.add_artist_favorite(&artist_id, &artist_name, thumbnail.as_deref())
}

#[tauri::command]
pub fn remove_artist_favorite(state: tauri::State<AppState>, artist_id: String) -> Result<(), AppError> {
    state.playlist.remove_artist_favorite(&artist_id)
}

#[tauri::command]
pub fn is_artist_favorite(state: tauri::State<AppState>, artist_id: String) -> Result<bool, AppError> {
    state.playlist.is_artist_favorite(&artist_id)
}

#[tauri::command]
pub fn get_all_artist_favorites(state: tauri::State<AppState>) -> Result<Vec<ArtistFavoriteDto>, AppError> {
    let entries = state.playlist.get_all_artist_favorites()?;
    Ok(entries.into_iter().map(|e| ArtistFavoriteDto {
        artist_id: e.artist_id,
        artist_name: e.artist_name,
        thumbnail: e.thumbnail,
        added_at: e.added_at,
    }).collect())
}

// ── Async commands (blocking I/O offloaded to tokio) ─────────────────
//
// These commands invoke yt-dlp subprocess calls, HTTP streaming, or heavy
// filesystem scans. They are `async` and use `tokio::task::spawn_blocking`
// to move the blocking work off the Tauri main-thread pool so the UI
// stays responsive while the operation runs.

/// Search for tracks across all registered sources (YouTube, SoundCloud, etc.).
///
/// Offloaded to `spawn_blocking` because source resolvers invoke yt-dlp.
#[tauri::command]
pub async fn search(state: tauri::State<'_, AppState>, query: String) -> Result<Vec<Track>, AppError> {
    let playback = state.playback.clone();
    tokio::task::spawn_blocking(move || playback.search(&query))
        .await
        .map_err(|e| AppError {
            code: "INTERNAL_ERROR".into(),
            details: Some(format!("search task join error: {}", e)),
        })?
}

/// Search with grouped results (songs, artists, albums).
/// Optional filter: "songs", "artists", "albums", or None for all.
///
/// Combines local library results with remote (YouTube, SoundCloud) results.
/// Offloaded to `spawn_blocking` because source search invokes yt-dlp.
#[tauri::command]
pub async fn search_grouped(
    state: tauri::State<'_, AppState>,
    query: String,
    filter: Option<String>,
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

    let library = state.library.clone();
    let playback = state.playback.clone();
    // If source settings are unavailable, fall back to all sources enabled
    let enabled = state.settings.get_enabled_sources().unwrap_or_else(|_| {
        // Default: YouTube and SoundCloud both enabled
        let mut set = std::collections::HashSet::new();
        set.insert("YouTube".to_string());
        set.insert("SoundCloud".to_string());
        set
    });
    tokio::task::spawn_blocking(move || {
        // 1. Local grouped results (songs, artists, albums from library)
        let mut result = library.search_grouped(&query, parsed_filter)?;

        // 2. Remote tracks from enabled sources only
        let remote_tracks = playback.search_all_tracks_enabled(&query, &enabled);
        if !remote_tracks.is_empty() {
            // Merge remote tracks into songs if filter allows
            let include_songs = parsed_filter.is_none()
                || parsed_filter == Some(SearchFilter::Songs);
            if include_songs {
                result.songs.extend(remote_tracks.clone());
            }

            // Group remote tracks by artist and merge into artists if filter allows
            let include_artists = parsed_filter.is_none()
                || parsed_filter == Some(SearchFilter::Artists);
            if include_artists {
                let mut remote_artist_map: std::collections::HashMap<String, Vec<&Track>> =
                    std::collections::HashMap::new();
                for track in &remote_tracks {
                    let artist_id = crate::ipc::dto::normalize_artist_id(&track.artist);
                    remote_artist_map.entry(artist_id).or_default().push(track);
                }

                // Build a lookup of existing artist IDs (case-insensitive) for dedup
                let mut existing_ids: std::collections::HashSet<String> =
                    result.artists.iter().map(|a| a.id.to_lowercase()).collect();

                for (id, tracks) in remote_artist_map {
                    let id_lower = id.to_lowercase();
                    if existing_ids.contains(&id_lower) {
                        // Merge: increment track_count of existing artist
                        if let Some(existing) = result.artists.iter_mut().find(
                            |a| a.id.to_lowercase() == id_lower,
                        ) {
                            existing.track_count += tracks.len() as u32;
                        }
                    } else {
                        existing_ids.insert(id_lower);
                        result.artists.push(ArtistSummary {
                            id,
                            name: tracks[0].artist.clone(),
                            thumbnail: tracks.iter().find_map(|t| t.thumbnail.clone()),
                            track_count: tracks.len() as u32,
                        });
                    }
                }

                result.artists.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
            }
        }

        Ok(result)
    })
    .await
    .map_err(|e| AppError {
        code: "INTERNAL_ERROR".into(),
        details: Some(format!("search_grouped task join error: {}", e)),
    })?
}

/// Get full artist detail by artist ID.
///
/// First tries the local library. If no local tracks are found,
/// falls back to searching remote sources (YouTube, SoundCloud)
/// for tracks by that artist name.
#[tauri::command]
pub async fn get_artist_detail(
    state: tauri::State<'_, AppState>,
    id: String,
) -> Result<ArtistDetail, AppError> {
    // Try local library first — if it returns NotFound or empty tracks, fall back to remote.
    let library = state.library.clone();
    let id_for_local = id.clone();
    let local_result = tokio::task::spawn_blocking(move || library.get_artist_detail(&id_for_local))
        .await
        .map_err(|e| AppError {
            code: "INTERNAL_ERROR".into(),
            details: Some(format!("get_artist_detail task join error: {}", e)),
        })?;

    // If local library found tracks, return immediately
    if let Ok(detail) = local_result {
        if !detail.top_tracks.is_empty() {
            return Ok(detail);
        }
    }

    // No local tracks — search remote sources by artist name
    let normalized_name = crate::ipc::dto::denormalize_artist_id(&id)
        .ok_or_else(|| crate::errors::types::ValidationError::InvalidInput(
            format!("Invalid artist ID: {}", id),
        ))?;

    let playback = state.playback.clone();
    let artist_id = id.clone();
    let search_name = normalized_name.clone();
    let remote_tracks = tokio::task::spawn_blocking(move || playback.search_all_tracks(&search_name))
        .await
        .map_err(|e| AppError {
            code: "INTERNAL_ERROR".into(),
            details: Some(format!("remote artist search task join error: {}", e)),
        })?;

    // Filter tracks that match the artist name (case-insensitive)
    let matching: Vec<Track> = remote_tracks
        .into_iter()
        .filter(|t| t.artist.to_lowercase() == normalized_name.to_lowercase())
        .collect();

    if matching.is_empty() {
        return Err(crate::errors::types::LibraryError::NotFound(id).into());
    }

    let thumbnail = matching.iter().find_map(|t| t.thumbnail.clone());
    let canonical_name = matching[0].artist.clone();

    Ok(ArtistDetail {
        id: artist_id,
        name: canonical_name,
        thumbnail,
        top_tracks: matching,
        albums: Vec::new(),
    })
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

/// Add a track to the queue by resolving it from the appropriate source.
///
/// Offloaded to `spawn_blocking` because it may invoke yt-dlp resolution.
/// Prefer `add_to_queue_with_track` when the full Track object is available —
/// it skips the slow resolve step.
#[tauri::command]
pub async fn add_to_queue(
    state: tauri::State<'_, AppState>,
    track_id: String,
) -> Result<(), AppError> {
    let playback = state.playback.clone();
    tokio::task::spawn_blocking(move || playback.add_to_queue(&track_id))
        .await
        .map_err(|e| AppError {
            code: "INTERNAL_ERROR".into(),
            details: Some(format!("add_to_queue task join error: {}", e)),
        })?
}

/// Add a track to the queue using the full Track object — no resolution needed.
///
/// Fast for both local and remote tracks because it skips the resolve step.
#[tauri::command]
pub fn add_to_queue_with_track(
    state: tauri::State<AppState>,
    track: Track,
) -> Result<(), AppError> {
    state.playback.add_to_queue_with_track(&track)
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
///
/// Offloaded to `spawn_blocking` because it may invoke yt-dlp resolution.
/// Prefer `play_next_with_track` when the full Track object is available —
/// it skips the slow resolve step.
#[tauri::command]
pub async fn play_next(
    state: tauri::State<'_, AppState>,
    track_id: String,
) -> Result<(), AppError> {
    let playback = state.playback.clone();
    tokio::task::spawn_blocking(move || playback.play_next(&track_id))
        .await
        .map_err(|e| AppError {
            code: "INTERNAL_ERROR".into(),
            details: Some(format!("play_next task join error: {}", e)),
        })?
}

/// Insert a track immediately after the current queue position using the full Track object.
///
/// Fast for both local and remote tracks because it skips the resolve step.
#[tauri::command]
pub fn play_next_with_track(
    state: tauri::State<AppState>,
    track: Track,
) -> Result<(), AppError> {
    state.playback.play_next_with_track(&track)
}

// ── Local Scanner commands ──────────────────────────────────────────

/// Scan a folder for audio files and add to local library.
///
/// Offloaded to `spawn_blocking` because filesystem traversal is slow.
#[tauri::command]
pub async fn scan_folder(
    state: tauri::State<'_, AppState>,
    folder_path: String,
) -> Result<ScanResult, AppError> {
    let scanner = state.scanner.clone();
    tokio::task::spawn_blocking(move || scanner.scan_folder(&folder_path))
        .await
        .map_err(|e| AppError {
            code: "INTERNAL_ERROR".into(),
            details: Some(format!("scan_folder task join error: {}", e)),
        })?
}

// ── Streaming & Playlist commands (all async — yt-dlp / HTTP) ────────

/// Play a remote track by resolving its stream URL and starting playback.
///
/// Offloaded to `spawn_blocking` because `play_stream` resolves via yt-dlp
/// and opens an HTTP stream — both blocking operations.
#[tauri::command]
pub async fn play_stream(
    state: tauri::State<'_, AppState>,
    track: Track,
) -> Result<(), AppError> {
    let playback = state.playback.clone();
    tokio::task::spawn_blocking(move || playback.play_stream(track))
        .await
        .map_err(|e| AppError {
            code: "INTERNAL_ERROR".into(),
            details: Some(format!("play_stream task join error: {}", e)),
        })?
}

/// Search for playlists across enabled sources only.
///
/// Mirrors `search_grouped`: fetches enabled sources from settings and passes
/// them to `search_playlists_enabled` so disabled sources are skipped. If
/// source settings are unavailable, falls back to all sources enabled.
///
/// Offloaded to `spawn_blocking` because it invokes yt-dlp playlist search.
#[tauri::command]
pub async fn search_playlists(
    state: tauri::State<'_, AppState>,
    query: String,
) -> Result<Vec<Playlist>, AppError> {
    if query.trim().is_empty() {
        return Ok(Vec::new());
    }
    let playback = state.playback.clone();
    // If source settings are unavailable, fall back to all sources enabled
    let enabled = state.settings.get_enabled_sources().unwrap_or_else(|_| {
        // Default: YouTube and SoundCloud both enabled
        let mut set = std::collections::HashSet::new();
        set.insert("YouTube".to_string());
        set.insert("SoundCloud".to_string());
        set
    });
    tokio::task::spawn_blocking(move || playback.search_playlists_enabled(&query, &enabled))
        .await
        .map_err(|e| AppError {
            code: "INTERNAL_ERROR".into(),
            details: Some(format!("search_playlists task join error: {}", e)),
        })
}

/// Resolve a playlist by source and URL/identifier.
///
/// Offloaded to `spawn_blocking` because it invokes yt-dlp playlist resolution.
#[tauri::command]
pub async fn resolve_playlist(
    state: tauri::State<'_, AppState>,
    source: String,
    url: String,
) -> Result<Playlist, AppError> {
    let source_type: crate::models::source::Source =
        serde_json::from_str(&format!("\"{}\"", source)).unwrap_or(crate::models::source::Source::YouTube);
    let playback = state.playback.clone();
    tokio::task::spawn_blocking(move || {
        playback
            .resolve_playlist(&source_type, &url)
            .map_err(AppError::from)
    })
    .await
    .map_err(|e| AppError {
        code: "INTERNAL_ERROR".into(),
        details: Some(format!("resolve_playlist task join error: {}", e)),
    })?
}

/// Play all tracks in a playlist, replacing the current queue and starting from the first.
///
/// Offloaded to `spawn_blocking` because it resolves the playlist URL via
/// yt-dlp and then starts playback.
#[tauri::command]
pub async fn play_playlist(
    state: tauri::State<'_, AppState>,
    source: String,
    url: String,
) -> Result<(), AppError> {
    let source_type: crate::models::source::Source =
        serde_json::from_str(&format!("\"{}\"", source)).unwrap_or(crate::models::source::Source::YouTube);
    let playback = state.playback.clone();
    tokio::task::spawn_blocking(move || playback.play_playlist(&source_type, &url))
        .await
        .map_err(|e| AppError {
            code: "INTERNAL_ERROR".into(),
            details: Some(format!("play_playlist task join error: {}", e)),
        })?
}

/// Resolve a track's stream URL without starting playback.
///
/// Offloaded to `spawn_blocking` because it invokes yt-dlp resolution.
#[tauri::command]
pub async fn resolve_track(
    state: tauri::State<'_, AppState>,
    source: String,
    id: String,
) -> Result<Track, AppError> {
    let source_type: crate::models::source::Source =
        serde_json::from_str(&format!("\"{}\"", source)).unwrap_or(crate::models::source::Source::YouTube);
    let playback = state.playback.clone();
    tokio::task::spawn_blocking(move || {
        playback
            .resolve_track_by_source(&source_type, &id)
            .map_err(AppError::from)
    })
    .await
    .map_err(|e| AppError {
        code: "INTERNAL_ERROR".into(),
        details: Some(format!("resolve_track task join error: {}", e)),
    })?
}

// ── Source Settings commands ──────────────────────────────────────────

/// Get all source settings (YouTube, SoundCloud), defaulting to enabled.
#[tauri::command]
pub fn get_source_settings(
    state: tauri::State<AppState>,
) -> Result<Vec<SourceSettingDto>, AppError> {
    let settings = state.settings.get_source_settings()?;
    Ok(settings.into_iter().map(SourceSettingDto::from).collect())
}

/// Enable or disable a source plugin.
#[tauri::command]
pub fn set_source_enabled(
    state: tauri::State<AppState>,
    source: String,
    enabled: bool,
) -> Result<(), AppError> {
    state.settings.set_source_enabled(&source, enabled)
}

/// DTO for source settings sent to the frontend.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceSettingDto {
    pub source: String,
    pub enabled: bool,
    pub label: String,
}

impl From<crate::persistence::models::SourceSetting> for SourceSettingDto {
    fn from(s: crate::persistence::models::SourceSetting) -> Self {
        Self {
            source: s.source,
            enabled: s.enabled,
            label: s.label,
        }
    }
}