# Rust State Management Example

State management patterns for Tauri 2.0.

**Path:** `src-tauri/src/state/mod.rs`

```rust
use std::sync::Mutex;
use serde::{Deserialize, Serialize};

/// Main application state
/// 
/// Registered with `tauri::Builder::default().manage(AppState::default())`
#[derive(Default)]
pub struct AppState {
    /// User settings (persisted)
    pub settings: Mutex<AppSettings>,
    /// Recently accessed files (in-memory cache)
    pub cache: Mutex<Vec<String>>,
    /// Current active document
    pub active_document: Mutex<Option<String>>,
}

/// User settings structure
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub theme: Theme,
    pub language: String,
    pub auto_save: bool,
    pub font_size: u32,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    Light,
    #[default]
    Dark,
    System,
}
```

## Usage in Commands

```rust
use tauri::State;
use crate::error::AppError;

#[tauri::command]
pub async fn get_settings(
    state: State<'_, AppState>,
) -> Result<AppSettings, AppError> {
    let settings = state.settings.lock()
        .map_err(|_| AppError::Internal("Failed to lock settings".into()))?;
    
    Ok(settings.clone())
}

#[tauri::command]
pub async fn update_settings(
    state: State<'_, AppState>,
    new_settings: AppSettings,
) -> Result<(), AppError> {
    let mut settings = state.settings.lock()
        .map_err(|_| AppError::Internal("Failed to lock settings".into()))?;
    
    *settings = new_settings;
    Ok(())
}

#[tauri::command]
pub async fn add_to_cache(
    state: State<'_, AppState>,
    item: String,
) -> Result<(), AppError> {
    let mut cache = state.cache.lock()
        .map_err(|_| AppError::Internal("Failed to lock cache".into()))?;
    
    // Keep only last 10 items
    if cache.len() >= 10 {
        cache.remove(0);
    }
    cache.push(item);
    
    Ok(())
}
```

## Async-Heavy Workloads (RwLock)

```rust
use tokio::sync::RwLock;

pub struct AsyncAppState {
    pub settings: RwLock<AppSettings>,
}

// Read access
let settings = state.settings.read().await;

// Write access  
let mut settings = state.settings.write().await;
*settings = new_settings;
```
