//! Persistence models — entry types for history, local scanner, playlists, artist favorites, and source settings.

use serde::{Deserialize, Serialize};

use jellyx_core::models::track::Track;
/// A play history entry with timestamp.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub id: i64,
    pub track: Track,
    pub played_at: String,
}

/// A watched folder entry for the local file scanner.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WatchedFolder {
    pub path: String,
    pub last_scanned_at: Option<String>,
    pub added_at: String,
}

/// A local track entry from the file scanner.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalTrackEntry {
    pub track: Track,
    pub file_path: String,
    pub folder_path: String,
    pub file_modified_at: Option<String>,
    /// Relative path of the file's parent directory with respect to the
    /// watched folder root. Empty string when the file lives directly in
    /// the watched root. Used by folder-as-playlist generation to group
    /// tracks into parent/child playlists.
    #[serde(default)]
    pub subfolder_path: Option<String>,
}

/// A user-created local playlist.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserPlaylist {
    pub id: String,
    pub title: String,
    pub created_at: String,
    pub updated_at: String,
    /// Playlist kind: `"manual"` (user-created), `"folder"` (auto-generated
    /// from a watched folder), or `"generated_artist"` (legacy artist-gen flow).
    #[serde(default = "default_playlist_kind")]
    pub kind: String,
    /// For folder-derived playlists: the watched folder path this playlist
    /// was generated from. Used for cascade-delete on folder removal.
    #[serde(default)]
    pub source_folder_path: Option<String>,
    /// For child folder playlists: the parent playlist's id. `None` for
    /// manual playlists and folder parent playlists.
    #[serde(default)]
    pub parent_playlist_id: Option<String>,
}

fn default_playlist_kind() -> String {
    "manual".to_string()
}

/// A track entry inside a user playlist.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistTrackEntry {
    pub playlist_id: String,
    pub position: i64,
    pub track: Track,
    pub added_at: String,
}

/// A favorited artist.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtistFavorite {
    pub artist_id: String,
    /// Source dimension ("local", "youtube", "soundcloud", ...). Together
    /// with `artist_id` it forms the composite primary key, so the same
    /// artist name from different sources no longer collides.
    #[serde(default = "default_favorite_source")]
    pub source: String,
    pub artist_name: String,
    pub thumbnail: Option<String>,
    /// Optional reference to the source-specific artist identifier
    /// (e.g. Spotify/YouTube artist id). `None` for local favorites.
    #[serde(default)]
    pub source_artist_ref: Option<String>,
    pub added_at: String,
}

fn default_favorite_source() -> String {
    "local".to_string()
}

/// A source plugin setting (enabled/disabled).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceSetting {
    pub source: String,
    pub enabled: bool,
    pub label: String,
}

/// Audio settings returned to the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioSettings {
    pub normalize_audio: bool,
}
