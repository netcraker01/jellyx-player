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
use crate::library::{LibraryService, PlaylistService, SettingsService, SuggestionCategory};
use crate::models::playlist::Playlist;
use crate::models::track::Track;
use crate::persistence::models::{HistoryEntry, LocalTrackEntry, WatchedFolder};
use crate::playback::service::PlaybackService;
use crate::sources::local::{ScanResult, ScannerService};
use crate::updater::prefs::{now_iso_utc, now_plus_seconds};
use crate::updater::service::UpdateService;

/// Application state shared across Tauri commands.
/// PlaybackService is the single authority for all playback operations.
/// LibraryService manages favorites and history.
/// PlaylistService manages user-created playlists and artist favorites.
/// SettingsService manages source enable/disable state.
/// ScannerService manages local file scanning.
/// fft_channel holds the Tauri Channel for binary FFT streaming.
/// updater runs the channel-aware update check (Phase 1: notify-only).
pub struct AppState {
    pub playback: Arc<PlaybackService>,
    pub library: Arc<LibraryService>,
    pub playlist: Arc<PlaylistService>,
    pub settings: Arc<SettingsService>,
    pub scanner: Arc<ScannerService>,
    /// Binary FFT streaming channel — set by `start_fft_stream`, used by FFT thread.
    pub fft_channel: Arc<Mutex<Option<tauri::ipc::Channel<Vec<u8>>>>>,
    /// Channel-aware updater service (Phase 1: notify-only / open-release-page).
    pub updater: Arc<UpdateService>,
    /// Shared HTTP client for all async network requests.
    pub http_client: reqwest::Client,
}

/// Convert a persistence-layer `UserPlaylist` into its IPC DTO form.
fn playlist_to_dto(pl: crate::persistence::models::UserPlaylist) -> UserPlaylistDto {
    UserPlaylistDto {
        id: pl.id,
        title: pl.title,
        kind: pl.kind,
        source_folder_path: pl.source_folder_path,
        parent_playlist_id: pl.parent_playlist_id,
        created_at: pl.created_at,
        updated_at: pl.updated_at,
    }
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

/// Set whether audio normalization is enabled for local (Symphonia/cpal) playback.
#[tauri::command]
pub fn set_playback_normalize_audio(
    state: tauri::State<AppState>,
    enabled: bool,
) -> Result<(), AppError> {
    state.playback.set_normalize_audio(enabled)
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

/// Get suggestion categories with dynamic year injection.
///
/// Returns the full list of genre/mood categories for the Discover section
/// on the Home and Search pages. Each category includes a search query
/// template with `{YEAR}` already resolved to the current year.
#[tauri::command]
pub fn get_suggestion_categories() -> Vec<SuggestionCategory> {
    crate::library::suggestions::get_suggestion_categories()
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
        kind: pl.kind,
        source_folder_path: pl.source_folder_path,
        parent_playlist_id: pl.parent_playlist_id,
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
    Ok(playlists.into_iter().map(playlist_to_dto).collect())
}

#[tauri::command]
pub fn get_recent_playlists(state: tauri::State<AppState>, limit: Option<u32>) -> Result<Vec<UserPlaylistDto>, AppError> {
    let playlists = state.playlist.get_recent_playlists(limit.unwrap_or(5))?;
    Ok(playlists.into_iter().map(playlist_to_dto).collect())
}

#[tauri::command]
pub fn search_user_playlists(state: tauri::State<AppState>, query: String) -> Result<Vec<UserPlaylistDto>, AppError> {
    let playlists = state.playlist.search_playlists(&query)?;
    Ok(playlists.into_iter().map(playlist_to_dto).collect())
}

#[tauri::command]
pub fn add_track_to_playlist(state: tauri::State<AppState>, playlist_id: String, track: Track) -> Result<(), AppError> {
    state.playlist.add_track_to_playlist(&playlist_id, &track)
}

/// Batch-add multiple tracks to a playlist in a single IPC call.
/// Used by the playlist import flow to avoid N sequential IPC round-trips.
#[tauri::command]
pub fn add_tracks_to_playlist(state: tauri::State<AppState>, playlist_id: String, tracks: Vec<Track>) -> Result<usize, AppError> {
    let mut added = 0;
    for track in &tracks {
        match state.playlist.add_track_to_playlist(&playlist_id, track) {
            Ok(()) => added += 1,
            Err(e) => {
                eprintln!("Failed to add track to playlist {}: {:?}", playlist_id, e);
            }
        }
    }
    Ok(added)
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

#[tauri::command]
pub fn get_playlist_thumbnails(state: tauri::State<AppState>, playlist_id: String) -> Result<Vec<String>, AppError> {
    state.playlist.get_playlist_thumbnails(&playlist_id)
}

/// Generate one playlist per artist from the local track catalog (idempotent).
///
/// Groups all local tracks by artist and creates or updates a playlist per
/// artist. Returns the playlists that were created or had tracks added.
#[tauri::command]
pub fn generate_artist_playlists(state: tauri::State<AppState>) -> Result<Vec<UserPlaylistDto>, AppError> {
    let playlists = state.playlist.generate_artist_playlists()?;
    Ok(playlists.into_iter().map(playlist_to_dto).collect())
}

/// Generate folder-as-playlist hierarchy for a watched folder.
///
/// Creates a parent playlist named after the folder and a child playlist for
/// each subfolder that contains audio files. Idempotent: re-running on an
/// already-scanned folder reuses existing playlists and only appends new
/// tracks. Returns all parent + child playlists touched or existing.
#[tauri::command]
pub fn generate_folder_playlists(
    state: tauri::State<AppState>,
    folder_path: String,
) -> Result<Vec<UserPlaylistDto>, AppError> {
    let playlists = state.playlist.generate_folder_playlists(&folder_path)?;
    Ok(playlists.into_iter().map(playlist_to_dto).collect())
}

/// Get all playlists generated from a watched folder (parent + children).
#[tauri::command]
pub fn get_playlists_by_source_folder(
    state: tauri::State<AppState>,
    folder_path: String,
) -> Result<Vec<UserPlaylistDto>, AppError> {
    let playlists = state.playlist.get_playlists_by_source_folder(&folder_path)?;
    Ok(playlists.into_iter().map(playlist_to_dto).collect())
}

/// Get child playlists of a parent playlist (folder-as-playlist children).
#[tauri::command]
pub fn get_child_playlists(
    state: tauri::State<AppState>,
    parent_id: String,
) -> Result<Vec<UserPlaylistDto>, AppError> {
    let playlists = state.playlist.get_child_playlists(&parent_id)?;
    Ok(playlists.into_iter().map(playlist_to_dto).collect())
}

// ── Artist Favorite commands (fast SQLite) ──────────────────────────

/// Add an artist to favorites.
///
/// Accepts an optional `source` so the same artist name from different
/// sources (e.g. "local" vs "youtube") can coexist without overwriting each
/// other. Defaults to `"local"` for backward compatibility.
#[tauri::command]
pub fn add_artist_favorite(
    state: tauri::State<AppState>,
    artist_id: String,
    artist_name: String,
    thumbnail: Option<String>,
    source: Option<String>,
    source_artist_ref: Option<String>,
) -> Result<(), AppError> {
    let src = source.as_deref().unwrap_or("local");
    state
        .playlist
        .add_artist_favorite_with_source(
            &artist_id,
            src,
            &artist_name,
            thumbnail.as_deref(),
            source_artist_ref.as_deref(),
        )
}

/// Remove an artist from favorites.
///
/// Pass `source` to remove only a specific source favorite; omit it to
/// remove every favorite for that artist across all sources.
#[tauri::command]
pub fn remove_artist_favorite(
    state: tauri::State<AppState>,
    artist_id: String,
    source: Option<String>,
) -> Result<(), AppError> {
    state
        .playlist
        .remove_artist_favorite(&artist_id, source.as_deref())
}

/// Check if an artist is favorited.
///
/// Pass `source` to check a specific source; omit it to check if any source
/// has a favorite for that artist.
#[tauri::command]
pub fn is_artist_favorite(
    state: tauri::State<AppState>,
    artist_id: String,
    source: Option<String>,
) -> Result<bool, AppError> {
    state
        .playlist
        .is_artist_favorite(&artist_id, source.as_deref())
}

#[tauri::command]
pub fn get_all_artist_favorites(state: tauri::State<AppState>) -> Result<Vec<ArtistFavoriteDto>, AppError> {
    let entries = state.playlist.get_all_artist_favorites()?;
    Ok(entries.into_iter().map(|e| ArtistFavoriteDto {
        artist_id: e.artist_id,
        source: e.source,
        artist_name: e.artist_name,
        thumbnail: e.thumbnail,
        source_artist_ref: e.source_artist_ref,
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
///
/// Pagination: `offset` and `limit` control the remote song results slice.
/// Page 1 = offset 0, limit 50. Page 2 = offset 50, limit 50. Etc.
/// Local library songs are always included in full (typically small).
/// `has_more_songs` in the response indicates more remote results are available.
#[tauri::command]
pub async fn search_grouped(
    state: tauri::State<'_, AppState>,
    query: String,
    filter: Option<String>,
    offset: Option<usize>,
    limit: Option<usize>,
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

    let page_offset = offset.unwrap_or(0);
    let page_limit = limit.unwrap_or(50);

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

        // 2. Remote tracks from enabled sources only, paginated
        let remote_tracks = playback.search_all_tracks_enabled(&query, &enabled, page_offset, page_limit);
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

        // Indicate whether more song results are available.
        // If we got a full page of remote results from at least one source,
        // assume there are more. With multiple enabled sources (YouTube +
        // SoundCloud), remote_tracks can exceed page_limit (e.g. 100 for
        // a 50-item page). Use >= so multi-source searches paginate too.
        result.has_more_songs = remote_tracks.len() >= page_limit;

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
/// Merges local library tracks with remote (YouTube, SoundCloud) tracks for
/// the same artist. Local tracks always come first, remote tracks are
/// appended after. This replaces the previous short-circuit behavior that
/// hid remote content whenever local tracks existed.
#[tauri::command]
pub async fn get_artist_detail(
    state: tauri::State<'_, AppState>,
    id: String,
) -> Result<ArtistDetail, AppError> {
    // Resolve the artist name from the ID. The ID may carry a source
    // dimension (e.g. `artist:daft-punk:youtube`) but for the local lookup
    // we only need the name portion.
    let normalized_name = crate::ipc::dto::denormalize_artist_id(&id)
        .ok_or_else(|| crate::errors::types::ValidationError::InvalidInput(
            format!("Invalid artist ID: {}", id),
        ))?;

    // Query local library first.
    let library = state.library.clone();
    let id_for_local = id.clone();
    let local_result = tokio::task::spawn_blocking(move || library.get_artist_detail(&id_for_local))
        .await
        .map_err(|e| AppError {
            code: "INTERNAL_ERROR".into(),
            details: Some(format!("get_artist_detail task join error: {}", e)),
        })?;

    // Query remote sources (YouTube, SoundCloud) by artist name.
    let playback = state.playback.clone();
    let search_name = normalized_name.clone();
    let remote_tracks = tokio::task::spawn_blocking(move || playback.search_all_tracks(&search_name))
        .await
        .map_err(|e| AppError {
            code: "INTERNAL_ERROR".into(),
            details: Some(format!("remote artist search task join error: {}", e)),
        })?;

    // Filter remote tracks that match the artist name (case-insensitive).
    let matching_remote: Vec<Track> = remote_tracks
        .into_iter()
        .filter(|t| t.artist.to_lowercase() == normalized_name.to_lowercase())
        .collect();

    // Merge: if we have a local result, use it as the base and append the
    // remote tracks. If we don't, build the result from the remote tracks.
    match local_result {
        Ok(mut detail) => {
            if !matching_remote.is_empty() {
                // Avoid duplicates by track id.
                let local_ids: std::collections::HashSet<String> =
                    detail.top_tracks.iter().map(|t| t.id.clone()).collect();
                for t in matching_remote {
                    if !local_ids.contains(&t.id) {
                        detail.top_tracks.push(t);
                    }
                }
                if detail.thumbnail.is_none() {
                    detail.thumbnail = detail.top_tracks.iter().find_map(|t| t.thumbnail.clone());
                }
            }
            Ok(detail)
        }
        Err(_) => {
            if matching_remote.is_empty() {
                return Err(crate::errors::types::LibraryError::NotFound(id).into());
            }
            let thumbnail = matching_remote.iter().find_map(|t| t.thumbnail.clone());
            let canonical_name = matching_remote[0].artist.clone();
            Ok(ArtistDetail {
                id: id.clone(),
                name: canonical_name,
                thumbnail,
                top_tracks: matching_remote,
                albums: Vec::new(),
            })
        }
    }
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

/// Download a remote stream URL to a local cache file for instant seeking.
///
/// The frontend calls this for YouTube tracks after receiving `stream-resolved`
/// to get a local file path that the browser can seek instantly. SoundCloud
/// tracks don't need this — their seek works fine over the remote proxy.
///
/// Offloaded to `spawn_blocking` because it performs a blocking HTTP download.
///
/// Returns the absolute path to the cached file (e.g. `/home/user/.local/share/helix/youtube_cache/dQw4w9WgXcQ.m4a`).
/// If the file is already cached, returns the existing path immediately without re-downloading.
#[tauri::command]
pub async fn cache_remote_stream(
    state: tauri::State<'_, AppState>,
    cache_id: String,
    remote_url: String,
) -> Result<String, AppError> {
    let playback = state.playback.clone();
    tokio::task::spawn_blocking(move || {
        playback
            .cache_remote_stream(&cache_id, &remote_url)
            .map_err(|e| AppError {
                code: "CACHE_ERROR".into(),
                details: Some(e),
            })
    })
    .await
    .map_err(|e| AppError {
        code: "INTERNAL_ERROR".into(),
        details: Some(format!("cache_remote_stream task join error: {}", e)),
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

// ── Audio Settings commands ─────────────────────────────────────────────

/// DTO for audio settings sent to the frontend.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioSettingsDto {
    pub normalize_audio: bool,
}

impl From<crate::persistence::models::AudioSettings> for AudioSettingsDto {
    fn from(s: crate::persistence::models::AudioSettings) -> Self {
        Self {
            normalize_audio: s.normalize_audio,
        }
    }
}

/// Get audio settings (normalization toggle, etc.).
#[tauri::command]
pub fn get_audio_settings(
    state: tauri::State<AppState>,
) -> Result<AudioSettingsDto, AppError> {
    let settings = state.settings.get_audio_settings()?;
    Ok(AudioSettingsDto::from(settings))
}

/// Enable or disable audio normalization.
#[tauri::command]
pub fn set_normalize_audio(
    state: tauri::State<AppState>,
    enabled: bool,
) -> Result<(), AppError> {
    state.settings.set_normalize_audio(enabled)
}

// ── Updater commands (Phase 1: notify-only) ───────────────────────────
//
// `check_for_updates` is async because it performs network I/O. Preference
// mutations remain synchronous because they only touch SQLite state.

/// Check for updates and return info if a newer version is available.
///
/// Applies suppression rules (skip / remind-later) so startup checks and
/// frontend-triggered automatic checks behave consistently.
#[tauri::command]
pub async fn check_for_updates(
    state: tauri::State<'_, AppState>,
) -> Result<Option<crate::ipc::dto::UpdaterInfo>, AppError> {
    let updater = state.updater.clone();
    updater.check().await.map_err(|e| AppError {
        code: "UPDATE_CHECK_FAILED".into(),
        details: Some(e),
    })
}

/// Persist a skipped version (user clicked "Skip this version").
#[tauri::command]
pub fn skip_update_version(
    state: tauri::State<AppState>,
    version: String,
) -> Result<crate::ipc::dto::UpdaterPrefs, AppError> {
    state
        .updater
        .skip_version(&version)
        .map_err(|e| AppError {
            code: "UPDATE_PREFS_ERROR".into(),
            details: Some(format!("{:?}", e)),
        })
}

/// Persist a remind-later timestamp (user clicked "Remind me later").
///
/// Accepts an optional `hours` argument (default 24). The backend computes
/// the timestamp so the frontend doesn't need to do clock math.
#[tauri::command]
pub fn remind_update_later(
    state: tauri::State<AppState>,
    hours: Option<u64>,
) -> Result<crate::ipc::dto::UpdaterPrefs, AppError> {
    let secs = hours.unwrap_or(24) * 3600;
    let ts = now_plus_seconds(secs);
    state
        .updater
        .remind_later(&ts)
        .map_err(|e| AppError {
            code: "UPDATE_PREFS_ERROR".into(),
            details: Some(format!("{:?}", e)),
        })
}

/// Read the persisted updater prefs.
#[tauri::command]
pub fn get_update_prefs(
    state: tauri::State<AppState>,
) -> Result<crate::ipc::dto::UpdaterPrefs, AppError> {
    state.updater.prefs().map_err(|e| AppError {
        code: "UPDATE_PREFS_ERROR".into(),
        details: Some(format!("{:?}", e)),
    })
}

/// Open the release page in the system default browser.
///
/// Implemented via `tauri-plugin-shell`'s `open` API. The `shell` plugin's
/// `open` permission is enabled in `tauri.conf.json` (`"shell": { "open": true }`).
#[tauri::command]
pub fn open_release_page(
    app: tauri::AppHandle,
    url: String,
) -> Result<(), AppError> {
    if !url.starts_with("https://github.com/netcraker01/helix/releases/") {
        return Err(AppError {
            code: "OPEN_RELEASE_PAGE_DENIED".into(),
            details: Some("release URL is outside the Helix GitHub releases allowlist".into()),
        });
    }

    use tauri_plugin_shell::ShellExt;
    #[allow(deprecated)]
    app.shell()
        .open(url, None)
        .map_err(|e| AppError {
            code: "OPEN_RELEASE_PAGE_FAILED".into(),
            details: Some(format!("{}", e)),
        })
}

/// Return the current app version (used by the frontend to display
/// "You're up to date" alongside the latest version).
#[tauri::command]
pub fn get_updater_current_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Returns the ISO-8601 UTC timestamp for "now" — exposed so the frontend can
/// compute remind-later displays consistently with the backend's clock.
#[tauri::command]
pub fn updater_now_iso() -> String {
    now_iso_utc()
}
