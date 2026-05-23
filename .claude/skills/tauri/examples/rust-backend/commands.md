# Rust Commands Example

Command patterns for Tauri 2.0.

**Path:** `src-tauri/src/commands/file.rs`

```rust
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, State};
use crate::{error::AppError, state::AppState};

/// Read file content
/// 
/// # Permissions required
/// - fs:allow-read
#[tauri::command]
pub async fn read_file(path: String) -> Result<String, AppError> {
    fs::read_to_string(&path)
        .map_err(|e| AppError::Io(format!("Failed to read {}: {}", path, e)))
}

/// Write content to file
/// 
/// # Permissions required
/// - fs:allow-write
#[tauri::command]
pub async fn save_file(path: String, content: String) -> Result<(), AppError> {
    fs::write(&path, &content)
        .map_err(|e| AppError::Io(format!("Failed to write {}: {}", path, e)))
}

/// List files in directory
#[tauri::command]
pub async fn list_files(dir: String) -> Result<Vec<String>, AppError> {
    let entries = fs::read_dir(&dir)
        .map_err(|e| AppError::Io(format!("Failed to read dir {}: {}", dir, e)))?;
    
    let files: Vec<String> = entries
        .filter_map(|entry| {
            entry.ok().and_then(|e| {
                e.path()
                    .file_name()
                    .and_then(|n| n.to_str().map(String::from))
            })
        })
        .collect();
    
    Ok(files)
}

/// Command with state access
#[tauri::command]
pub async fn get_recent_files(
    state: State<'_, AppState>,
) -> Result<Vec<String>, AppError> {
    let cache = state.cache.lock()
        .map_err(|_| AppError::Internal("Failed to lock state".into()))?;
    
    Ok(cache.clone())
}

/// Command with app handle for path resolution
#[tauri::command]
pub async fn get_app_data_path(
    app: AppHandle,
) -> Result<String, AppError> {
    let path = app.path()
        .app_data_dir()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    
    Ok(path.to_string_lossy().to_string())
}
```

## Register in lib.rs

```rust
mod commands;

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::file::read_file,
            commands::file::save_file,
            commands::file::list_files,
            commands::file::get_recent_files,
            commands::file::get_app_data_path,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```
