# Cross-Platform Directory Patterns

Detailed patterns for cross-platform directory handling, with focus on AI model management applications.

## Model Storage Conventions

Different AI runners store models in platform-specific locations:

### Ollama

| Platform | Models Directory |
|----------|-----------------|
| Linux | `~/.ollama/models` |
| macOS | `~/.ollama/models` |
| Windows | `%USERPROFILE%\.ollama\models` |

**Detection pattern:**
```rust
fn ollama_models_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".ollama").join("models"))
}
```

### LM Studio

| Platform | Models Directory |
|----------|-----------------|
| Linux | `~/.cache/lm-studio/models` |
| macOS | `~/.cache/lm-studio/models` |
| Windows | `%USERPROFILE%\.cache\lm-studio\models` |

**Detection pattern:**
```rust
fn lm_studio_models_dir() -> Option<PathBuf> {
    // LM Studio uses its own convention, not standard cache_dir
    dirs::home_dir().map(|h| h.join(".cache").join("lm-studio").join("models"))
}
```

### Hugging Face Cache

| Platform | Cache Directory |
|----------|-----------------|
| Linux | `~/.cache/huggingface/hub` |
| macOS | `~/.cache/huggingface/hub` |
| Windows | `%USERPROFILE%\.cache\huggingface\hub` |

Environment override: `HF_HOME` or `HUGGINGFACE_HUB_CACHE`

```rust
fn huggingface_cache_dir() -> Option<PathBuf> {
    // Check environment overrides first
    if let Ok(hf_home) = std::env::var("HF_HOME") {
        return Some(PathBuf::from(hf_home).join("hub"));
    }
    if let Ok(cache) = std::env::var("HUGGINGFACE_HUB_CACHE") {
        return Some(PathBuf::from(cache));
    }
    // Default location
    dirs::home_dir().map(|h| h.join(".cache").join("huggingface").join("hub"))
}
```

## Robust Directory Resolution

### With Environment Override

```rust
use std::path::PathBuf;

/// Resolve a directory with optional environment variable override.
fn resolve_dir(
    env_var: &str,
    default_fn: fn() -> Option<PathBuf>,
    subpath: &str,
) -> Option<PathBuf> {
    std::env::var(env_var)
        .ok()
        .map(PathBuf::from)
        .or_else(|| default_fn().map(|p| p.join(subpath)))
}

// Usage:
let config = resolve_dir("MYAPP_CONFIG", dirs::config_dir, "myapp");
let cache = resolve_dir("MYAPP_CACHE", dirs::cache_dir, "myapp");
```

### With Fallback Chain

```rust
/// Try multiple strategies to find a directory.
fn find_models_dir() -> Option<PathBuf> {
    // 1. Environment override
    if let Ok(dir) = std::env::var("MODELS_DIR") {
        let path = PathBuf::from(dir);
        if path.exists() {
            return Some(path);
        }
    }

    // 2. XDG data directory
    if let Some(data) = dirs::data_dir() {
        let path = data.join("models");
        if path.exists() {
            return Some(path);
        }
    }

    // 3. Home directory fallback
    if let Some(home) = dirs::home_dir() {
        let path = home.join(".models");
        return Some(path); // Return even if doesn't exist yet
    }

    None
}
```

## Directory Verification and Creation

### Ensure Directory Exists

```rust
use std::fs;
use std::io;
use std::path::PathBuf;

/// Ensure a directory exists, creating it if necessary.
fn ensure_dir(path: &PathBuf) -> io::Result<()> {
    if !path.exists() {
        fs::create_dir_all(path)?;
    } else if !path.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("{} exists but is not a directory", path.display()),
        ));
    }
    Ok(())
}

/// Get or create app config directory.
fn get_or_create_config_dir(app: &str) -> io::Result<PathBuf> {
    let dir = dirs::config_dir()
        .ok_or_else(|| io::Error::new(
            io::ErrorKind::NotFound,
            "could not determine config directory",
        ))?
        .join(app);
    ensure_dir(&dir)?;
    Ok(dir)
}
```

### Check Write Permissions

```rust
use std::fs;
use std::path::Path;

/// Check if a directory is writable by attempting to create a temp file.
fn is_writable(dir: &Path) -> bool {
    if !dir.exists() {
        return false;
    }

    let test_path = dir.join(".write_test");
    match fs::write(&test_path, b"test") {
        Ok(_) => {
            let _ = fs::remove_file(&test_path);
            true
        }
        Err(_) => false,
    }
}
```

## Platform-Specific Behavior

### Windows Path Handling

```rust
/// Normalize path for display (convert backslashes on Windows).
fn display_path(path: &Path) -> String {
    #[cfg(windows)]
    {
        path.to_string_lossy().replace('\\', "/")
    }
    #[cfg(not(windows))]
    {
        path.to_string_lossy().to_string()
    }
}
```

### Long Path Support (Windows)

```rust
/// Prefix path for Windows long path support (>260 chars).
#[cfg(windows)]
fn long_path(path: &Path) -> PathBuf {
    let path_str = path.to_string_lossy();
    if path_str.len() > 200 && !path_str.starts_with(r"\\?\") {
        PathBuf::from(format!(r"\\?\{}", path_str))
    } else {
        path.to_path_buf()
    }
}

#[cfg(not(windows))]
fn long_path(path: &Path) -> PathBuf {
    path.to_path_buf()
}
```

## Testing Patterns

### Mock Home Directory

```rust
#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    #[test]
    fn test_config_resolution() {
        let temp = TempDir::new().unwrap();

        // Note: dirs crate caches home_dir on first call
        // Use HOME env var for test isolation
        std::env::set_var("HOME", temp.path());

        // Now dirs::home_dir() returns temp path
        let home = dirs::home_dir().unwrap();
        assert_eq!(home, temp.path());
    }
}
```

### Cross-Platform Test Assertions

```rust
#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    fn assert_path_contains(path: &PathBuf, component: &str) {
        let path_str = path.to_string_lossy();
        assert!(
            path_str.contains(component),
            "Expected path to contain '{}', got: {}",
            component,
            path_str
        );
    }

    #[test]
    fn config_dir_is_platform_appropriate() {
        if let Some(config) = dirs::config_dir() {
            #[cfg(target_os = "linux")]
            assert_path_contains(&config, ".config");

            #[cfg(target_os = "macos")]
            assert_path_contains(&config, "Application Support");

            #[cfg(target_os = "windows")]
            assert_path_contains(&config, "AppData");
        }
    }
}
```

## Error Handling Patterns

### Custom Error Type

```rust
use thiserror::Error;
use std::path::PathBuf;

#[derive(Error, Debug)]
pub enum DirectoryError {
    #[error("home directory not found")]
    HomeNotFound,

    #[error("config directory not found")]
    ConfigNotFound,

    #[error("directory does not exist: {0}")]
    NotExists(PathBuf),

    #[error("path is not a directory: {0}")]
    NotDirectory(PathBuf),

    #[error("directory not writable: {0}")]
    NotWritable(PathBuf),

    #[error("failed to create directory: {0}")]
    CreateFailed(#[from] std::io::Error),
}

fn require_config_dir(app: &str) -> Result<PathBuf, DirectoryError> {
    let dir = dirs::config_dir()
        .ok_or(DirectoryError::ConfigNotFound)?
        .join(app);

    std::fs::create_dir_all(&dir)?;

    if !is_writable(&dir) {
        return Err(DirectoryError::NotWritable(dir));
    }

    Ok(dir)
}
```

## Symlink-Aware Patterns

### Resolve Symlinks

```rust
use std::fs;
use std::path::{Path, PathBuf};

/// Resolve a path, following symlinks.
fn resolve_path(path: &Path) -> std::io::Result<PathBuf> {
    if path.is_symlink() {
        fs::read_link(path)
    } else {
        Ok(path.to_path_buf())
    }
}

/// Canonicalize path if it exists, otherwise return as-is.
fn safe_canonicalize(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}
```

### Detect Shared Storage

```rust
/// Check if two paths are on the same filesystem.
#[cfg(unix)]
fn same_filesystem(path1: &Path, path2: &Path) -> std::io::Result<bool> {
    use std::os::unix::fs::MetadataExt;

    let meta1 = std::fs::metadata(path1)?;
    let meta2 = std::fs::metadata(path2)?;

    Ok(meta1.dev() == meta2.dev())
}

#[cfg(windows)]
fn same_filesystem(path1: &Path, path2: &Path) -> std::io::Result<bool> {
    // Windows: compare drive letters
    let root1 = path1.components().next();
    let root2 = path2.components().next();
    Ok(root1 == root2)
}
```
