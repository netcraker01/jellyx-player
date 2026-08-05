//! Application state for the Jellyx TUI.
//!
//! Holds the engine services and current UI state. The engine owns all
//! business logic; this struct only tracks what the renderer needs.

use crossterm::event::KeyCode;
use jellyx_engine::audio_backend::AudioBackend;
use jellyx_engine::focus_session::{FocusPreferencesRow, FocusSessionRepository};
use jellyx_engine::library_service::LibraryService;
use jellyx_engine::local_track::LocalTrackRepository;
use jellyx_engine::playback_models::PlaybackState;
use jellyx_engine::playlist_service::PlaylistService;
use jellyx_engine::preferences::PreferencesRepository;
use jellyx_engine::settings_service::SettingsService;
use jellyx_engine::source_resolver::{SourceRegistry, SourceResolver};
use jellyx_engine::sqlite::SqliteHandle;
use jellyx_engine::user_playlists::UserPlaylist;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use crate::audio::TuiAudioBackend;

/// Which tab/view is currently active.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    Library,
    NowPlaying,
    Playlists,
    Focus,
    Settings,
}

impl View {
    pub fn label(self) -> &'static str {
        match self {
            Self::Library => "Library",
            Self::NowPlaying => "Now Playing",
            Self::Playlists => "Playlists",
            Self::Focus => "Focus",
            Self::Settings => "Settings",
        }
    }

    pub fn all() -> [Self; 5] {
        [
            Self::Library,
            Self::NowPlaying,
            Self::Playlists,
            Self::Focus,
            Self::Settings,
        ]
    }

    pub fn next(self) -> Self {
        let all = Self::all();
        let idx = all.iter().position(|v| *v == self).unwrap_or(0);
        all[(idx + 1) % all.len()]
    }

    pub fn prev(self) -> Self {
        let all = Self::all();
        let idx = all.iter().position(|v| *v == self).unwrap_or(0);
        all[(idx + all.len() + 1) % all.len()]
    }
}

fn db_path() -> Option<PathBuf> {
    dirs::data_local_dir().map(|d| d.join("jellyx").join("jellyx.db"))
}

/// A track entry for the library view.
pub struct TrackEntry {
    pub title: String,
    pub artist: String,
    pub local_path: Option<String>,
    pub source: jellyx_core::models::source::Source,
    pub source_id: String,
}

/// A playlist entry for the playlists view.
pub struct PlaylistEntry {
    pub id: String,
    pub title: String,
    pub track_count: u32,
}

/// A track inside a playlist.
pub struct PlaylistTrackEntry {
    pub title: String,
    pub artist: String,
    pub local_path: Option<String>,
    pub source: jellyx_core::models::source::Source,
    pub source_id: String,
}

/// A remote track from YouTube/SoundCloud search.
pub struct RemoteTrackEntry {
    pub id: String,
    pub source: jellyx_core::models::source::Source,
    pub title: String,
    pub artist: String,
    pub duration: Option<f64>,
}

/// Top-level application state.
pub struct App {
    pub view: View,
    pub running: bool,
    pub message: String,
    pub db: Option<SqliteHandle>,
    pub library: Option<LibraryService>,
    pub settings: Option<SettingsService>,
    pub playlists: Option<PlaylistService>,
    // Library view
    pub tracks: Vec<TrackEntry>,
    pub selected_track: usize,
    // Playlist view
    pub playlist_list: Vec<PlaylistEntry>,
    pub selected_playlist: usize,
    pub playlist_tracks: Vec<PlaylistTrackEntry>,
    pub selected_playlist_track: usize,
    pub viewing_playlist_tracks: bool,
    // Settings
    pub source_settings: Vec<(String, bool)>,
    pub normalize_audio: bool,
    pub telemetry_enabled: bool,
    // Playback
    pub audio: TuiAudioBackend,
    pub playback_state: PlaybackState,
    pub volume: f32,
    pub now_playing: Option<String>,
    // Focus
    pub focus_prefs: Option<FocusPreferencesRow>,
    pub focus_active: Option<String>,
    // Remote sources
    pub sources: SourceRegistry,
    pub search_query: String,
    pub search_results: Vec<RemoteTrackEntry>,
    pub search_cursor: usize,
    pub searching: bool,
}

impl App {
    pub fn new() -> Self {
        let mut app = Self {
            view: View::Library,
            running: true,
            message: "Welcome to Jellyx TUI — q quit, Tab switch, Enter play".into(),
            db: None,
            library: None,
            settings: None,
            playlists: None,
            tracks: Vec::new(),
            selected_track: 0,
            playlist_list: Vec::new(),
            selected_playlist: 0,
            playlist_tracks: Vec::new(),
            selected_playlist_track: 0,
            viewing_playlist_tracks: false,
            source_settings: Vec::new(),
            normalize_audio: true,
            telemetry_enabled: false,
            audio: TuiAudioBackend::new(),
            playback_state: PlaybackState::Stopped,
            volume: 1.0,
            now_playing: None,
            focus_prefs: None,
            focus_active: None,
            sources: {
                let mut reg = SourceRegistry::new();
                reg.register(Box::new(crate::sources::YouTubeResolver));
                reg.register(Box::new(crate::sources::SoundCloudResolver));
                reg
            },
            search_query: String::new(),
            search_results: Vec::new(),
            search_cursor: 0,
            searching: false,
        };
        app.try_init_engine();
        app
    }

    fn try_init_engine(&mut self) {
        let path = match db_path() {
            Some(p) => p,
            None => {
                self.message = "Could not find data directory".into();
                return;
            }
        };

        if !path.exists() {
            self.message = format!("No DB at {}", path.display());
            return;
        }

        // Open with recovery — init closure runs schema + migrations.
        let init = |handle: &SqliteHandle| -> Result<(), String> {
            handle
                .initialize_schema()
                .map_err(|e| format!("schema init: {e}"))?;
            // Migrations are run by the desktop; the TUI just needs the
            // schema to be current. If the desktop has already migrated,
            // initialize_schema is idempotent.
            Ok(())
        };

        match SqliteHandle::open_with_recovery(&path, Duration::from_secs(5), init) {
            Ok(handle) => {
                self.library = Some(LibraryService::new(handle.clone()));
                self.settings = Some(SettingsService::new(Arc::new(handle.clone())));
                self.playlists = Some(PlaylistService::new(handle.clone()));
                self.db = Some(handle);
                self.refresh_data();
                self.message = "Engine initialized — data loaded".into();
            }
            Err(e) => {
                self.message = format!("DB open failed: {e}");
            }
        }
    }

    pub fn refresh_data(&mut self) {
        if let Some(handle) = &self.db {
            let repo = LocalTrackRepository::new(handle.clone());
            if let Ok(rows) = repo.get_all(None) {
                self.tracks = rows
                    .into_iter()
                    .filter_map(|r| {
                        serde_json::from_str::<jellyx_core::models::track::Track>(&r.track_json)
                            .ok()
                            .map(|t| TrackEntry {
                                title: t.title,
                                artist: t.artist,
                                local_path: t.local_path,
                                source: t.source,
                                source_id: t.source_id,
                            })
                    })
                    .take(200)
                    .collect();
            }
        }

        if let Some(playlists) = &self.playlists {
            if let Ok(list) = playlists.get_all_playlists() {
                self.playlist_list = list
                    .into_iter()
                    .map(|p| PlaylistEntry {
                        track_count: playlists.count_playlist_tracks(&p.id).unwrap_or(0),
                        id: p.id,
                        title: p.title,
                    })
                    .collect();
            }
        }

        if let Some(settings) = &self.settings {
            if let Ok(sources) = settings.get_source_settings() {
                self.source_settings = sources.into_iter().map(|s| (s.source, s.enabled)).collect();
            }
            if let Ok(audio) = settings.get_audio_settings() {
                self.normalize_audio = audio.normalize_audio;
            }
            if let Ok(telemetry) = settings.get_telemetry_settings() {
                self.telemetry_enabled = telemetry.enabled;
            }
        }

        // Focus data
        if let Some(handle) = &self.db {
            let repo = FocusSessionRepository::new(handle.clone());
            if let Ok(prefs) = repo.get_preferences() {
                self.focus_prefs = Some(prefs);
            }
            if let Ok(Some(session)) = repo.get_nonterminal_session() {
                self.focus_active = Some(format!(
                    "{} (round {}, phase: {})",
                    session.intention, session.round, session.phase
                ));
            } else {
                self.focus_active = None;
            }
        }
    }

    pub fn handle_key(&mut self, key: KeyCode) -> bool {
        match key {
            KeyCode::Char('q') => true,
            KeyCode::Esc => {
                if self.viewing_playlist_tracks {
                    self.viewing_playlist_tracks = false;
                    self.playlist_tracks.clear();
                    self.message = "Back to playlists".into();
                    false
                } else {
                    true
                }
            }
            KeyCode::Tab => {
                self.view = self.view.next();
                self.message = format!("View: {}", self.view.label());
                false
            }
            KeyCode::BackTab => {
                self.view = self.view.prev();
                self.message = format!("View: {}", self.view.label());
                false
            }
            KeyCode::Char('r') => {
                self.refresh_data();
                self.message = "Data refreshed".into();
                false
            }
            KeyCode::Char(' ') => self.handle_space(),
            KeyCode::Char('s') => {
                let _ = self.audio.stop();
                self.playback_state = PlaybackState::Stopped;
                self.now_playing = None;
                self.message = "Stopped".into();
                false
            }
            // Library
            KeyCode::Up if self.view == View::Library => {
                if self.selected_track > 0 {
                    self.selected_track -= 1;
                }
                false
            }
            KeyCode::Down if self.view == View::Library => {
                if self.selected_track + 1 < self.tracks.len() {
                    self.selected_track += 1;
                }
                false
            }
            KeyCode::Enter if self.view == View::Library => {
                self.play_selected_track();
                false
            }
            // Playlists list
            KeyCode::Up if self.view == View::Playlists && !self.viewing_playlist_tracks => {
                if self.selected_playlist > 0 {
                    self.selected_playlist -= 1;
                }
                false
            }
            KeyCode::Down if self.view == View::Playlists && !self.viewing_playlist_tracks => {
                if self.selected_playlist + 1 < self.playlist_list.len() {
                    self.selected_playlist += 1;
                }
                false
            }
            KeyCode::Enter if self.view == View::Playlists && !self.viewing_playlist_tracks => {
                self.load_playlist_tracks();
                false
            }
            // Playlist tracks
            KeyCode::Up if self.view == View::Playlists && self.viewing_playlist_tracks => {
                if self.selected_playlist_track > 0 {
                    self.selected_playlist_track -= 1;
                }
                false
            }
            KeyCode::Down if self.view == View::Playlists && self.viewing_playlist_tracks => {
                if self.selected_playlist_track + 1 < self.playlist_tracks.len() {
                    self.selected_playlist_track += 1;
                }
                false
            }
            KeyCode::Enter if self.view == View::Playlists && self.viewing_playlist_tracks => {
                self.play_playlist_track();
                false
            }
            // Search mode in Library
            KeyCode::Char('/') if self.view == View::Library => {
                self.search_query.clear();
                self.search_results.clear();
                self.searching = true;
                self.message = "Search (YouTube/SoundCloud): type query, Enter to search".into();
                false
            }
            KeyCode::Char(c) if self.view == View::Library && self.searching => {
                if c == '\n' || c == '\r' {
                    self.do_remote_search();
                } else if (c as u8) == 127 || c == '\u{8}' {
                    self.search_query.pop();
                } else {
                    self.search_query.push(c);
                }
                false
            }
            KeyCode::Enter
                if self.view == View::Library
                    && self.searching
                    && !self.search_results.is_empty() =>
            {
                self.play_remote_track();
                false
            }
            KeyCode::Up
                if self.view == View::Library
                    && self.searching
                    && !self.search_results.is_empty() =>
            {
                if self.search_cursor > 0 {
                    self.search_cursor -= 1;
                }
                false
            }
            KeyCode::Down
                if self.view == View::Library
                    && self.searching
                    && !self.search_results.is_empty() =>
            {
                if self.search_cursor + 1 < self.search_results.len() {
                    self.search_cursor += 1;
                }
                false
            }
            KeyCode::Esc if self.searching => {
                self.searching = false;
                self.search_results.clear();
                self.message = "Search cancelled".into();
                false
            }
            _ => false,
        }
    }

    fn do_remote_search(&mut self) {
        if self.search_query.is_empty() {
            self.message = "Empty query".into();
            return;
        }
        self.message = format!("Searching: {}...", self.search_query);
        let results = self.sources.search_all(&self.search_query);
        self.search_results = results
            .into_iter()
            .map(|t| RemoteTrackEntry {
                id: t.id.clone(),
                source: t.source.clone(),
                title: t.title,
                artist: t.artist,
                duration: t.duration,
            })
            .take(50)
            .collect();
        self.search_cursor = 0;
        self.message = format!(
            "Found {} results — Enter to play, Up/Down to navigate",
            self.search_results.len()
        );
    }

    fn play_remote_track(&mut self) {
        let entry = match self.search_results.get(self.search_cursor) {
            Some(e) => e.clone(),
            None => return,
        };

        self.message = format!("Resolving stream: {} — {}...", entry.artist, entry.title);
        match self.sources.resolve_stream_url(&entry.source, &entry.id) {
            Ok(url) => {
                self.message = format!("Stream resolved, downloading...");
                match self.audio.play(&url) {
                    Ok(()) => {
                        self.playback_state = PlaybackState::Playing;
                        self.now_playing = Some(format!("{} — {}", entry.artist, entry.title));
                        self.message = format!("Playing: {}", self.now_playing.as_ref().unwrap());
                        self.searching = false;
                    }
                    Err(e) => {
                        self.message = format!("Playback error: {e:?}");
                    }
                }
            }
            Err(e) => {
                self.message = format!("Resolve error: {e}");
            }
        }
    }

    fn handle_space(&mut self) -> bool {
        match self.playback_state {
            PlaybackState::Playing => {
                let _ = self.audio.pause();
                self.playback_state = PlaybackState::Paused;
                self.message = "Paused".into();
            }
            PlaybackState::Paused => {
                let _ = self.audio.resume();
                self.playback_state = PlaybackState::Playing;
                self.message = "Resumed".into();
            }
            _ => {}
        }
        false
    }

    fn play_selected_track(&mut self) {
        if let Some(track) = self.tracks.get(self.selected_track) {
            let title = track.title.clone();
            let artist = track.artist.clone();
            let path = track.local_path.clone();
            let source = track.source.clone();
            let source_id = track.source_id.clone();
            self.play_track(&title, &artist, path.as_deref(), &source, &source_id);
        }
    }

    fn play_playlist_track(&mut self) {
        if let Some(track) = self.playlist_tracks.get(self.selected_playlist_track) {
            let title = track.title.clone();
            let artist = track.artist.clone();
            let path = track.local_path.clone();
            let source = track.source.clone();
            let source_id = track.source_id.clone();
            self.play_track(&title, &artist, path.as_deref(), &source, &source_id);
        }
    }

    fn play_track(
        &mut self,
        title: &str,
        artist: &str,
        local_path: Option<&str>,
        source: &jellyx_core::models::source::Source,
        source_id: &str,
    ) {
        if let Some(path) = local_path {
            // Local file — play directly
            match self.audio.play_local(std::path::Path::new(path)) {
                Ok(()) => {
                    self.playback_state = PlaybackState::Playing;
                    self.now_playing = Some(format!("{artist} — {title}"));
                    self.message = format!("Playing: {}", self.now_playing.as_ref().unwrap());
                }
                Err(e) => {
                    self.message = format!("Playback error: {e:?}");
                }
            }
        } else {
            // Remote track — resolve stream URL and play
            self.message = format!("Resolving stream: {artist} — {title}...");
            match self.sources.resolve_stream_url(source, source_id) {
                Ok(url) => {
                    self.message = "Stream resolved, downloading...".into();
                    match self.audio.play(&url) {
                        Ok(()) => {
                            self.playback_state = PlaybackState::Playing;
                            self.now_playing = Some(format!("{artist} — {title}"));
                            self.message =
                                format!("Playing: {}", self.now_playing.as_ref().unwrap());
                        }
                        Err(e) => {
                            self.message = format!("Stream error: {e:?}");
                        }
                    }
                }
                Err(e) => {
                    self.message = format!("Resolve error: {e}");
                }
            }
        }
    }

    fn load_playlist_tracks(&mut self) {
        let playlist_id = match self.playlist_list.get(self.selected_playlist) {
            Some(p) => p.id.clone(),
            None => return,
        };

        if let Some(playlists) = &self.playlists {
            if let Ok(tracks) = playlists.get_playlist_tracks(&playlist_id) {
                self.playlist_tracks = tracks
                    .into_iter()
                    .map(|t| PlaylistTrackEntry {
                        title: t.track.title.clone(),
                        artist: t.track.artist.clone(),
                        local_path: t.track.local_path.clone(),
                        source: t.track.source.clone(),
                        source_id: t.track.source_id.clone(),
                    })
                    .collect();
                self.selected_playlist_track = 0;
                self.viewing_playlist_tracks = true;
                self.message = format!(
                    "{} ({} tracks) — Esc to go back",
                    self.playlist_list[self.selected_playlist].title,
                    self.playlist_tracks.len()
                );
            }
        }
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}
