//! Longplay — a skeuomorphic turntable for a local music library.
//!
//! The Rust core owns the library index, the audio pipeline and the playback
//! model. The webview is a thin presentation layer.

pub mod audio;
pub mod library;
pub mod platform;
pub mod playback;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use tauri::Manager;

use audio::{PlaybackState, Session};
use library::db::Db;
use library::model::Album;
use library::scanner::{self, ScanSummary};

pub struct AppState {
    db: Arc<Mutex<Db>>,
    covers_dir: PathBuf,
    playback: Arc<Session>,
}

/// Version shown in the settings panel.
#[tauri::command]
fn app_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[tauri::command]
fn library_load(state: tauri::State<'_, AppState>) -> Result<Vec<Album>, String> {
    let db = state
        .db
        .lock()
        .map_err(|_| "Datenbank ist gesperrt".to_string())?;
    db.load_albums().map_err(|error| error.to_string())
}

#[tauri::command]
fn library_folders(state: tauri::State<'_, AppState>) -> Result<Vec<String>, String> {
    let db = state
        .db
        .lock()
        .map_err(|_| "Datenbank ist gesperrt".to_string())?;
    db.folders().map_err(|error| error.to_string())
}

#[tauri::command]
async fn library_scan(
    folders: Vec<String>,
    state: tauri::State<'_, AppState>,
) -> Result<ScanSummary, String> {
    let db = state.db.clone();
    let covers_dir = state.covers_dir.clone();

    tauri::async_runtime::spawn_blocking(move || {
        let roots: Vec<PathBuf> = folders.iter().map(PathBuf::from).collect();
        let mut guard = db
            .lock()
            .map_err(|_| "Datenbank ist gesperrt".to_string())?;
        let summary =
            scanner::scan(&roots, &mut guard, &covers_dir).map_err(|error| error.to_string())?;
        guard
            .set_folders(&roots)
            .map_err(|error| error.to_string())?;
        Ok(summary)
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
fn playback_load(
    album_id: String,
    side_index: usize,
    autoplay: bool,
    state: tauri::State<'_, AppState>,
) -> Result<PlaybackState, String> {
    let album = {
        let db = state
            .db
            .lock()
            .map_err(|_| "Datenbank ist gesperrt".to_string())?;
        db.album(&album_id)
            .map_err(|error| error.to_string())?
            .ok_or_else(|| "Album nicht gefunden".to_string())?
    };

    let side = album
        .sides
        .get(side_index)
        .ok_or_else(|| "Seite nicht gefunden".to_string())?;

    state
        .playback
        .load(&album_id, side_index, side, album.format, autoplay);
    Ok(state.playback.state())
}

#[tauri::command]
fn playback_state(state: tauri::State<'_, AppState>) -> PlaybackState {
    state.playback.state()
}

#[tauri::command]
fn playback_toggle(state: tauri::State<'_, AppState>) -> PlaybackState {
    state.playback.toggle();
    state.playback.state()
}

#[tauri::command]
fn playback_pause(state: tauri::State<'_, AppState>) -> PlaybackState {
    state.playback.pause();
    state.playback.state()
}

#[tauri::command]
fn playback_stop(state: tauri::State<'_, AppState>) -> PlaybackState {
    state.playback.stop();
    state.playback.state()
}

#[tauri::command]
fn playback_seek(seconds: f64, state: tauri::State<'_, AppState>) -> PlaybackState {
    state.playback.seek(seconds);
    state.playback.state()
}

#[tauri::command]
fn playback_set_rpm(rpm: u32, state: tauri::State<'_, AppState>) -> PlaybackState {
    state.playback.set_rpm(rpm);
    state.playback.state()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            let cache_dir = app.path().app_cache_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            let covers_dir = cache_dir.join("covers");
            std::fs::create_dir_all(&covers_dir)?;

            let mut db =
                Db::open(&data_dir.join("library.sqlite3")).map_err(|error| error.to_string())?;

            // Dev hook: scan one or more folders at startup so the UI can be
            // checked without clicking through the folder picker.
            if let Some(paths) = std::env::var_os("LONGPLAY_SCAN_DIR") {
                let roots: Vec<PathBuf> = std::env::split_paths(&paths).collect();
                match scanner::scan(&roots, &mut db, &covers_dir) {
                    Ok(summary) => {
                        eprintln!("LONGPLAY_SCAN_DIR: {} Alben gescannt", summary.albums);
                        let _ = db.set_folders(&roots);
                    }
                    Err(error) => eprintln!("LONGPLAY_SCAN_DIR: {error}"),
                }
            }

            let playback = Arc::new(Session::new());
            playback.start_output();

            app.manage(AppState {
                db: Arc::new(Mutex::new(db)),
                covers_dir,
                playback,
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            app_version,
            library_load,
            library_folders,
            library_scan,
            playback_load,
            playback_state,
            playback_toggle,
            playback_pause,
            playback_stop,
            playback_seek,
            playback_set_rpm
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Longplay");
}
