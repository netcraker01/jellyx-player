//! Rendering for the Jellyx TUI.

use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Tabs};

use crate::app::{App, View};

/// Main draw function called every frame.
pub fn draw(frame: &mut Frame, app: &App) {
    let area = frame.area();

    // Layout: top tabs, middle content, bottom status bar
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(area);

    draw_tabs(frame, chunks[0], app);
    draw_content(frame, chunks[1], app);
    draw_status_bar(frame, chunks[2], app);
}

fn draw_tabs(frame: &mut Frame, area: Rect, app: &App) {
    let titles: Vec<Line> = View::all()
        .iter()
        .map(|v| {
            let style = if *v == app.view {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            Line::from(Span::styled(format!(" {} ", v.label()), style))
        })
        .collect();

    let tabs = Tabs::new(titles)
        .block(Block::default().borders(Borders::ALL).title("Jellyx"))
        .highlight_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );

    frame.render_widget(tabs, area);
}

fn draw_content(frame: &mut Frame, area: Rect, app: &App) {
    match app.view {
        View::Library => draw_library(frame, area, app),
        View::NowPlaying => draw_now_playing(frame, area, app),
        View::Playlists => draw_playlists(frame, area, app),
        View::Focus => draw_focus(frame, area, app),
        View::Settings => draw_settings(frame, area, app),
    }
}

fn draw_library(frame: &mut Frame, area: Rect, _app: &App) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Library")
        .style(Style::default().fg(Color::White));

    let content = vec![
        Line::from(Span::styled(
            "No tracks loaded yet.",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(""),
        Line::from("Press Tab to switch views."),
    ];

    frame.render_widget(Paragraph::new(content).block(block), area);
}

fn draw_now_playing(frame: &mut Frame, area: Rect, _app: &App) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Now Playing")
        .style(Style::default().fg(Color::White));

    let content = vec![
        Line::from(Span::styled(
            "No track playing.",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(""),
        Line::from("Playback controls will appear here."),
    ];

    frame.render_widget(Paragraph::new(content).block(block), area);
}

fn draw_playlists(frame: &mut Frame, area: Rect, _app: &App) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Playlists")
        .style(Style::default().fg(Color::White));

    let content = vec![Line::from(Span::styled(
        "No playlists found.",
        Style::default().fg(Color::DarkGray),
    ))];

    frame.render_widget(Paragraph::new(content).block(block), area);
}

fn draw_focus(frame: &mut Frame, area: Rect, _app: &App) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Focus Session")
        .style(Style::default().fg(Color::White));

    let content = vec![
        Line::from(Span::styled(
            "No active focus session.",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(""),
        Line::from("Press 's' to start a Pomodoro session (future)."),
    ];

    frame.render_widget(Paragraph::new(content).block(block), area);
}

fn draw_settings(frame: &mut Frame, area: Rect, _app: &App) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Settings")
        .style(Style::default().fg(Color::White));

    let content = vec![
        Line::from("Audio normalization: enabled (default)"),
        Line::from("Telemetry: disabled (default)"),
        Line::from(""),
        Line::from("Press Tab to switch views."),
    ];

    frame.render_widget(Paragraph::new(content).block(block), area);
}

fn draw_status_bar(frame: &mut Frame, area: Rect, app: &App) {
    let style = Style::default().fg(Color::Black).bg(Color::Cyan);
    let text = Line::from(Span::styled(format!(" {} ", app.message), style));
    frame.render_widget(Paragraph::new(text).alignment(Alignment::Left), area);
}
