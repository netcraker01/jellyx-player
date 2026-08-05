//! Event handling for the Jellyx TUI.

use std::time::Duration;

use crossterm::event::{self, Event as CrosstermEvent, KeyEvent};
use crossterm::execute;
use crossterm::terminal::disable_raw_mode;

/// Application events.
pub enum Event {
    Key(KeyEvent),
    Tick,
    Quit,
}

/// Poll for an event with a 250ms timeout.
///
/// If no event arrives within the timeout, returns `Event::Tick` so the
/// UI can refresh progress bars and other time-dependent elements.
pub fn poll_event() -> std::io::Result<Event> {
    let timeout = Duration::from_millis(250);
    if event::poll(timeout)? {
        match event::read()? {
            CrosstermEvent::Key(key) => Ok(Event::Key(key)),
            CrosstermEvent::Resize(_, _) => Ok(Event::Tick),
            _ => Ok(Event::Tick),
        }
    } else {
        Ok(Event::Tick)
    }
}

/// Restore terminal on panic.
pub fn restore_terminal_on_panic() {
    let _ = disable_raw_mode();
    let _ = execute!(std::io::stdout(), crossterm::terminal::LeaveAlternateScreen);
}
