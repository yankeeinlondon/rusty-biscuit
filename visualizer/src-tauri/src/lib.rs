//! Visualizer — a Tauri 2 desktop application shell.
//!
//! This is the bare application skeleton. The command/event surface, document
//! registry, and Excalidraw integration described in
//! `../features/2026-06-14-excalidraw/spec.md` are not implemented yet.

/// Build and run the Tauri application.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("error while running Visualizer");
}
