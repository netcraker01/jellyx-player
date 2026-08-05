// Quick test: verify the TUI's engine initialization logic works
// without needing a terminal. We replicate the App::try_init_engine flow.

use std::path::PathBuf;
use std::time::Duration;

fn main() {
    let db_path: PathBuf = dirs::data_local_dir()
        .map(|d| d.join("jellyx").join("jellyx.db"))
        .expect("no data dir");

    println!("DB path: {}", db_path.display());
    println!("DB exists: {}", db_path.exists());

    if !db_path.exists() {
        eprintln!("No DB found — cannot test.");
        return;
    }

    let init = |handle: &jellyx_engine::sqlite::SqliteHandle| -> Result<(), String> {
        handle
            .initialize_schema()
            .map_err(|e| format!("schema init: {e}"))?;
        println!("Schema initialized OK");
        Ok(())
    };

    match jellyx_engine::sqlite::SqliteHandle::open_with_recovery(
        &db_path,
        Duration::from_secs(5),
        init,
    ) {
        Ok(handle) => {
            println!("DB opened OK");

            // Test LibraryService
            let library = jellyx_engine::library_service::LibraryService::new(handle.clone());
            let recent = library.get_recent_unique(10);
            match &recent {
                Ok(tracks) => println!("Recent tracks: {}", tracks.len()),
                Err(e) => println!("Recent tracks error: {e}"),
            }

            // Test local tracks
            let repo = jellyx_engine::local_track::LocalTrackRepository::new(handle.clone());
            match repo.get_all(None) {
                Ok(rows) => {
                    println!("Local tracks in DB: {}", rows.len());
                    for (i, r) in rows.iter().take(5).enumerate() {
                        if let Ok(track) =
                            serde_json::from_str::<jellyx_core::models::track::Track>(&r.track_json)
                        {
                            println!(
                                "  [{}] {} — {} (path: {:?})",
                                i,
                                track.artist,
                                track.title,
                                track.local_path.as_deref().map(|p| &p[..p.len().min(40)])
                            );
                        }
                    }
                }
                Err(e) => println!("Local tracks error: {e}"),
            }

            // Test playlists
            let playlists = jellyx_engine::playlist_service::PlaylistService::new(handle.clone());
            match playlists.get_all_playlists() {
                Ok(list) => {
                    println!("Playlists: {}", list.len());
                    for p in list.iter().take(5) {
                        let count = playlists.count_playlist_tracks(&p.id).unwrap_or(0);
                        println!("  {} ({} tracks)", p.title, count);
                    }
                }
                Err(e) => println!("Playlists error: {e}"),
            }

            // Test focus
            let focus = jellyx_engine::focus_session::FocusSessionRepository::new(handle.clone());
            match focus.get_nonterminal_session() {
                Ok(Some(s)) => println!(
                    "Active focus: {} (round {}, phase: {})",
                    s.intention, s.round, s.phase
                ),
                Ok(None) => println!("No active focus session"),
                Err(e) => println!("Focus error: {e}"),
            }
            match focus.get_preferences() {
                Ok(p) => println!(
                    "Focus prefs: workflow={}, work={}ms, break={}ms",
                    p.default_workflow, p.work_duration_ms, p.break_duration_ms
                ),
                Err(e) => println!("Focus prefs error: {e}"),
            }

            // Test settings
            let settings_svc =
                jellyx_engine::settings_service::SettingsService::new(std::sync::Arc::new(handle));
            match settings_svc.get_source_settings() {
                Ok(sources) => {
                    println!("Sources: {}", sources.len());
                    for s in &sources {
                        println!(
                            "  {} = {}",
                            s.source,
                            if s.enabled { "enabled" } else { "disabled" }
                        );
                    }
                }
                Err(e) => println!("Settings error: {e}"),
            }
            match settings_svc.get_audio_settings() {
                Ok(a) => println!("Normalize audio: {}", a.normalize_audio),
                Err(e) => println!("Audio settings error: {e}"),
            }
            match settings_svc.get_telemetry_settings() {
                Ok(t) => println!("Telemetry: {}", t.enabled),
                Err(e) => println!("Telemetry error: {e}"),
            }

            println!(
                "\n✅ Engine initialization works — TUI will load this data when launched in a real terminal."
            );
        }
        Err(e) => println!("DB open failed: {e}"),
    }
}
