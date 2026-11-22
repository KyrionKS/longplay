//! Longplay — a skeuomorphic turntable for a local music library.
//!
//! The Rust core owns the library index, the audio pipeline and the playback
//! model. The webview is a thin presentation layer.

pub mod audio;
pub mod library;
pub mod playback;
pub mod platform;

/// Version shown in the settings panel.
#[tauri::command]
fn app_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![app_version])
        .run(tauri::generate_context!())
        .expect("failed to run Longplay");
}
