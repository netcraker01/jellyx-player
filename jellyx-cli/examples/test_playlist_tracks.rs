use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

fn main() {
    let db_path: PathBuf = dirs::data_local_dir()
        .map(|d| d.join("jellyx").join("jellyx.db"))
        .expect("no data dir");

    let init = |handle: &jellyx_engine::sqlite::SqliteHandle| -> Result<(), String> {
        handle
            .initialize_schema()
            .map_err(|e| format!("schema: {e}"))?;
        Ok(())
    };

    let handle = jellyx_engine::sqlite::SqliteHandle::open_with_recovery(
        &db_path,
        Duration::from_secs(5),
        init,
    )
    .unwrap();

    let playlists = jellyx_engine::playlist_service::PlaylistService::new(handle.clone());
    let list = playlists.get_all_playlists().unwrap();

    for p in list.iter().take(3) {
        println!("\nPlaylist: {} (id: {})", p.title, p.id);
        match playlists.get_playlist_tracks(&p.id) {
            Ok(tracks) => {
                for (i, t) in tracks.iter().take(3).enumerate() {
                    println!(
                        "  [{}] {} — {} | local_path: {:?} | source: {:?}",
                        i, t.track.artist, t.track.title, t.track.local_path, t.track.source
                    );
                }
            }
            Err(e) => println!("  Error: {e}"),
        }
    }

    // Also check local tracks
    let repo = jellyx_engine::local_track::LocalTrackRepository::new(handle);
    if let Ok(rows) = repo.get_all(None) {
        println!("\n\nLocal tracks (first 3):");
        for (i, r) in rows.iter().take(3).enumerate() {
            if let Ok(track) =
                serde_json::from_str::<jellyx_core::models::track::Track>(&r.track_json)
            {
                println!(
                    "  [{}] {} — {} | local_path: {:?} | source: {:?}",
                    i, track.artist, track.title, track.local_path, track.source
                );
            }
        }
    }
}
