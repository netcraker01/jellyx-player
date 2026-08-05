//! Rendering for the Jellyx TUI.

use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Tabs};

use crate::app::{App, View};
use jellyx_engine::audio_backend::AudioBackend;
use jellyx_engine::playback_models::PlaybackState;

pub fn draw(frame: &mut Frame, app: &App) {
    let area = frame.area();

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

fn draw_library(frame: &mut Frame, area: Rect, app: &App) {
    if app.searching {
        draw_search(frame, area, app);
    } else {
        draw_track_list(frame, area, app);
    }
}

fn draw_track_list(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default().borders(Borders::ALL).title(format!(
        "Library ({}) — Up/Down navigate, Enter play, / search, r refresh",
        app.tracks.len()
    ));

    if app.tracks.is_empty() {
        let content = vec![
            Line::from(Span::styled(
                "No local tracks found.",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(""),
            Line::from("Press '/' to search YouTube/SoundCloud."),
            Line::from("Press 'r' to refresh local tracks."),
        ];
        frame.render_widget(Paragraph::new(content).block(block), area);
    } else {
        let items: Vec<ListItem> = app
            .tracks
            .iter()
            .enumerate()
            .map(|(i, t)| {
                let style = if i == app.selected_track {
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                let play_icon =
                    if i == app.selected_track && app.playback_state != PlaybackState::Stopped {
                        if app.playback_state == PlaybackState::Playing {
                            "▶"
                        } else {
                            "⏸"
                        }
                    } else {
                        " "
                    };
                ListItem::new(Line::from(Span::styled(
                    format!("{} {} — {}", play_icon, t.artist, t.title),
                    style,
                )))
            })
            .collect();
        frame.render_widget(List::new(items).block(block), area);
    }
}

fn draw_search(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!("Search: \"{}\" — Esc cancel", app.search_query));

    if app.search_results.is_empty() && !app.search_query.is_empty() {
        let content = vec![
            Line::from(Span::styled(
                format!("Query: {}", app.search_query),
                Style::default().fg(Color::Yellow),
            )),
            Line::from(""),
            Line::from("Press Enter to search YouTube + SoundCloud."),
        ];
        frame.render_widget(Paragraph::new(content).block(block), area);
    } else if app.search_results.is_empty() {
        let content = vec![
            Line::from(Span::styled(
                "Type to search — results from YouTube + SoundCloud",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(""),
            Line::from("Press '/' to start, type query, Enter to search."),
            Line::from("Up/Down navigate, Enter to play, Esc to cancel."),
        ];
        frame.render_widget(Paragraph::new(content).block(block), area);
    } else {
        let items: Vec<ListItem> = app
            .search_results
            .iter()
            .enumerate()
            .map(|(i, t)| {
                let style = if i == app.search_cursor {
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                let source_tag = match t.source {
                    jellyx_core::models::source::Source::YouTube => "[YT]",
                    jellyx_core::models::source::Source::SoundCloud => "[SC]",
                    _ => "[??]",
                };
                let dur = t
                    .duration
                    .map(|d| {
                        format!(
                            " ({:.0}:{:02})",
                            (d / 60.0).floor(),
                            (d % 60.0).round() as i64
                        )
                    })
                    .unwrap_or_default();
                ListItem::new(Line::from(Span::styled(
                    format!("  {} {} — {}{}", source_tag, t.artist, t.title, dur),
                    style,
                )))
            })
            .collect();
        frame.render_widget(List::new(items).block(block), area);
    }
}

fn draw_now_playing(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default().borders(Borders::ALL).title("Now Playing");

    let state_str = match app.playback_state {
        PlaybackState::Stopped => "Stopped",
        PlaybackState::Playing => "Playing",
        PlaybackState::Paused => "Paused",
        PlaybackState::Buffering(_) => "Buffering",
    };

    let pos = app.audio.position();

    let mut lines = vec![Line::from(Span::styled(
        format!("State: {}", state_str),
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    ))];

    if let Some(ref np) = app.now_playing {
        lines.push(Line::from(format!("Track: {}", np)));
        lines.push(Line::from(format!("Position: {:.1}s", pos)));
    } else {
        lines.push(Line::from(Span::styled(
            "No track playing.",
            Style::default().fg(Color::DarkGray),
        )));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Controls",
        Style::default().fg(Color::Yellow),
    )));
    lines.push(Line::from("  Space — Play/Pause"));
    lines.push(Line::from("  s — Stop"));
    lines.push(Line::from("  Up/Down — Navigate"));
    lines.push(Line::from("  Enter — Play selected track"));

    frame.render_widget(Paragraph::new(lines).block(block), area);
}

fn draw_playlists(frame: &mut Frame, area: Rect, app: &App) {
    if app.viewing_playlist_tracks {
        draw_playlist_tracks(frame, area, app);
    } else {
        draw_playlist_list(frame, area, app);
    }
}

fn draw_playlist_list(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default().borders(Borders::ALL).title(format!(
        "Playlists ({}) — Enter to open",
        app.playlist_list.len()
    ));

    if app.playlist_list.is_empty() {
        let content = vec![
            Line::from(Span::styled(
                "No playlists found.",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(""),
            Line::from("Press 'r' to refresh."),
        ];
        frame.render_widget(Paragraph::new(content).block(block), area);
    } else {
        let items: Vec<ListItem> = app
            .playlist_list
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let style = if i == app.selected_playlist {
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                ListItem::new(Line::from(Span::styled(
                    format!("  {} ({} tracks)", p.title, p.track_count),
                    style,
                )))
            })
            .collect();
        frame.render_widget(List::new(items).block(block), area);
    }
}

fn draw_playlist_tracks(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default().borders(Borders::ALL).title(format!(
        "Tracks ({}) — Esc to go back, Enter to play",
        app.playlist_tracks.len()
    ));

    if app.playlist_tracks.is_empty() {
        frame.render_widget(
            Paragraph::new(vec![Line::from(Span::styled(
                "This playlist is empty.",
                Style::default().fg(Color::DarkGray),
            ))])
            .block(block),
            area,
        );
    } else {
        let items: Vec<ListItem> = app
            .playlist_tracks
            .iter()
            .enumerate()
            .map(|(i, t)| {
                let style = if i == app.selected_playlist_track {
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                ListItem::new(Line::from(Span::styled(
                    format!("  {} — {}", t.artist, t.title),
                    style,
                )))
            })
            .collect();
        frame.render_widget(List::new(items).block(block), area);
    }
}

fn draw_focus(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Focus Session");

    let mut lines = Vec::new();

    if let Some(ref active) = app.focus_active {
        lines.push(Line::from(Span::styled(
            format!("Active: {}", active),
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )));
    } else {
        lines.push(Line::from(Span::styled(
            "No active focus session.",
            Style::default().fg(Color::DarkGray),
        )));
    }

    lines.push(Line::from(""));

    if let Some(ref prefs) = app.focus_prefs {
        lines.push(Line::from(Span::styled(
            "Preferences",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(format!(
            "  Workflow: {}",
            prefs.default_workflow
        )));
        lines.push(Line::from(format!(
            "  Work: {}ms, Break: {}ms, Rounds: {}",
            prefs.work_duration_ms, prefs.break_duration_ms, prefs.rounds
        )));
        lines.push(Line::from(format!(
            "  Music: {} {}",
            prefs.music_strategy,
            prefs.music_value.as_deref().unwrap_or("")
        )));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Press 'r' to refresh focus data.",
        Style::default().fg(Color::Yellow),
    )));

    frame.render_widget(Paragraph::new(lines).block(block), area);
}

fn draw_settings(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default().borders(Borders::ALL).title("Settings");

    let mut lines = vec![
        Line::from(Span::styled(
            "Audio",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(format!(
            "  Normalize: {}",
            if app.normalize_audio { "ON" } else { "OFF" }
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Sources",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
    ];

    for (source, enabled) in &app.source_settings {
        lines.push(Line::from(format!(
            "  {}: {}",
            source,
            if *enabled { "enabled" } else { "disabled" }
        )));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Privacy",
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(format!(
        "  Telemetry: {}",
        if app.telemetry_enabled {
            "enabled"
        } else {
            "disabled"
        }
    )));

    frame.render_widget(Paragraph::new(lines).block(block), area);
}

fn draw_status_bar(frame: &mut Frame, area: Rect, app: &App) {
    let style = Style::default().fg(Color::Black).bg(Color::Cyan);
    let text = Line::from(Span::styled(format!(" {} ", app.message), style));
    frame.render_widget(Paragraph::new(text).alignment(Alignment::Left), area);
}
