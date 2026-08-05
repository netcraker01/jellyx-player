//! Application state for the Jellyx TUI.
//!
//! Holds the engine services and current UI state. The engine owns all
//! business logic; this struct only tracks what the renderer needs.

use crossterm::event::KeyCode;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

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
        all[(idx + all.len() - 1) % all.len()]
    }
}

/// Top-level application state.
pub struct App {
    pub view: View,
    pub running: bool,
    pub message: String,
}

impl App {
    pub fn new() -> Self {
        Self {
            view: View::Library,
            running: true,
            message: "Welcome to Jellyx TUI — press q to quit, Tab to switch views".into(),
        }
    }

    /// Handle a key press. Returns `true` if the app should quit.
    pub fn handle_key(&mut self, key: KeyCode) -> bool {
        match key {
            KeyCode::Char('q') | KeyCode::Esc => true,
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
            _ => false,
        }
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}
