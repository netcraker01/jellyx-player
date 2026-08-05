//! Jellyx TUI — full-parity Ratatui frontend.
//!
//! Uses `jellyx-engine` for all application logic (library, playback,
//! settings, focus). This crate only handles terminal I/O and rendering.

mod app;
mod audio;
mod event;
mod sources;
mod ui;

use std::io;

use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

use crate::app::App;
use crate::event::Event;

fn main() -> io::Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Run app
    let app = App::new();
    let result = run(&mut terminal, app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    result
}

fn run(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, mut app: App) -> io::Result<()> {
    loop {
        terminal.draw(|frame| ui::draw(frame, &app))?;

        match event::poll_event()? {
            Event::Quit => break,
            Event::Key(key) => {
                if app.handle_key(key.code) {
                    break;
                }
            }
            Event::Tick => {}
        }
    }
    Ok(())
}
