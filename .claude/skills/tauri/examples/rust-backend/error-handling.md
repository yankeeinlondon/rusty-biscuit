# Rust Error Handling Example

Error handling patterns for Tauri 2.0.

**Path:** `src-tauri/src/error.rs`

```rust
use serde::Serialize;

/// Application-wide error type
///
/// All errors are serialized to JSON for frontend consumption
#[derive(Debug, Serialize)]
#[serde(tag = "type", content = "message")]
pub enum AppError {
    /// File system errors
    Io(String),
    /// Database errors
    Database(String),
    /// Validation errors
    Validation(String),
    /// Resource not found
    NotFound(String),
    /// Permission denied
    PermissionDenied(String),
    /// Internal server error
    Internal(String),
}

// Convert from std::io::Error
impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        AppError::Io(err.to_string())
    }
}

// Convert from serde_json::Error
impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> Self {
        AppError::Internal(format!("JSON error: {}", err))
    }
}

// Convert to Tauri invoke error
// This allows returning Result<T, AppError> from commands
impl From<AppError> for tauri::ipc::InvokeError {
    fn from(err: AppError) -> Self {
        // Serialize to JSON string for frontend parsing
        let json = serde_json::to_string(&err)
            .unwrap_or_else(|_| r#"{"type":"Internal","message":"Serialization failed"}"#.into());

        tauri::ipc::InvokeError::from(json)
    }
}
```

## Usage in Commands

```rust
#[tauri::command]
pub async fn risky_operation() -> Result<String, AppError> {
    // Automatic conversion from std::io::Error
    let content = std::fs::read_to_string("file.txt")?;

    // Manual error creation
    if content.is_empty() {
        return Err(AppError::Validation("File is empty".into()));
    }

    Ok(content)
}
```

## Frontend Error Handling

```typescript
try {
  const result = await invoke('risky_operation');
} catch (error) {
  const parsed = JSON.parse(error as string);
  console.error(parsed.type, parsed.message);
  // Output: "Io" "No such file or directory"
}
```
