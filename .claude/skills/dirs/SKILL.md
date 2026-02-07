---
name: dirs
description: Platform-specific directory resolution for Rust applications. Use when working with config, cache, data, or home directories. Covers XDG spec on Linux, ~/Library on macOS, %APPDATA% on Windows, and cross-platform fallback strategies.
---

# dirs

Minimal Rust crate for platform-specific directory resolution. Returns paths without creating directories or verifying existence.

## Core Principles

- All functions return `Option<PathBuf>` - always handle `None`
- Paths are resolved, not created - create directories yourself if needed
- Use `dirs` for user-level directories, `directories` crate for app-specific paths
- Linux follows XDG Base Directory spec with sensible fallbacks
- macOS uses `~/Library/*` paths (Application Support, Caches, Preferences)
- Windows distinguishes Roaming (`%APPDATA%`) vs Local (`%LOCALAPPDATA%`)

## Platform Directory Matrix

| Function | Linux | macOS | Windows |
|----------|-------|-------|---------|
| `home_dir()` | `$HOME` | `$HOME` | `{FOLDERID_Profile}` |
| `config_dir()` | `$XDG_CONFIG_HOME` or `~/.config` | `~/Library/Application Support` | `%APPDATA%` (Roaming) |
| `config_local_dir()` | `$XDG_CONFIG_HOME` or `~/.config` | `~/Library/Application Support` | `%LOCALAPPDATA%` |
| `cache_dir()` | `$XDG_CACHE_HOME` or `~/.cache` | `~/Library/Caches` | `%LOCALAPPDATA%` |
| `data_dir()` | `$XDG_DATA_HOME` or `~/.local/share` | `~/Library/Application Support` | `%APPDATA%` (Roaming) |
| `data_local_dir()` | `$XDG_DATA_HOME` or `~/.local/share` | `~/Library/Application Support` | `%LOCALAPPDATA%` |
| `state_dir()` | `$XDG_STATE_HOME` or `~/.local/state` | **None** | **None** |
| `runtime_dir()` | `$XDG_RUNTIME_DIR` or **None** | **None** | **None** |
| `preference_dir()` | `$XDG_CONFIG_HOME` or `~/.config` | `~/Library/Preferences` | `%APPDATA%` (Roaming) |

## Windows: Roaming vs Local

| Type | Path | Purpose |
|------|------|---------|
| **Roaming** | `C:\Users\Alice\AppData\Roaming` | Syncs across domain machines, user settings |
| **Local** | `C:\Users\Alice\AppData\Local` | Machine-specific, caches, large data |

- Use `config_dir()` / `data_dir()` for roaming (settings that should sync)
- Use `config_local_dir()` / `data_local_dir()` / `cache_dir()` for local (caches, machine-specific)

## Quick Reference

### Basic Usage

```rust
use std::path::PathBuf;

fn get_app_config_dir(app_name: &str) -> Option<PathBuf> {
    dirs::config_dir().map(|p| p.join(app_name))
}

fn get_app_cache_dir(app_name: &str) -> Option<PathBuf> {
    dirs::cache_dir().map(|p| p.join(app_name))
}

fn get_app_data_dir(app_name: &str) -> Option<PathBuf> {
    dirs::data_dir().map(|p| p.join(app_name))
}
```

### Safe Directory Creation

```rust
use std::fs;
use std::path::PathBuf;

fn ensure_config_dir(app_name: &str) -> std::io::Result<PathBuf> {
    let dir = dirs::config_dir()
        .ok_or_else(|| std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "config directory not found"
        ))?
        .join(app_name);

    fs::create_dir_all(&dir)?;
    Ok(dir)
}
```

### Home Directory with Fallback

```rust
use std::path::PathBuf;

fn home_or_fallback() -> PathBuf {
    dirs::home_dir().unwrap_or_else(|| PathBuf::from("."))
}

// Pattern from this codebase (claudine, research):
fn home_with_tilde_fallback() -> PathBuf {
    dirs::home_dir().unwrap_or_else(|| PathBuf::from("~"))
}
```

### Expand Tilde Paths

```rust
fn expand_tilde(path: &std::path::Path) -> PathBuf {
    if path.starts_with("~") {
        dirs::home_dir()
            .map(|h| h.join(path.strip_prefix("~").unwrap_or(path)))
            .unwrap_or_else(|| path.to_path_buf())
    } else {
        path.to_path_buf()
    }
}
```

## XDG Environment Variables (Linux)

| Variable | Default | Purpose |
|----------|---------|---------|
| `XDG_CONFIG_HOME` | `~/.config` | User config files |
| `XDG_CACHE_HOME` | `~/.cache` | Non-essential cached data |
| `XDG_DATA_HOME` | `~/.local/share` | User data files |
| `XDG_STATE_HOME` | `~/.local/state` | Persistent state (logs, history) |
| `XDG_RUNTIME_DIR` | None (system-set) | Runtime files (sockets, PIDs) |

## macOS Directory Purposes

| Path | Purpose |
|------|---------|
| `~/Library/Application Support` | Config and data (combined) |
| `~/Library/Caches` | Cached data (can be cleared) |
| `~/Library/Preferences` | `.plist` preference files |
| `~/Library/Fonts` | User fonts |

## When Functions Return None

| Function | Returns None When |
|----------|-------------------|
| `home_dir()` | `$HOME` unset and `getpwuid_r` fails |
| `runtime_dir()` | Always on macOS/Windows; Linux if `$XDG_RUNTIME_DIR` unset |
| `state_dir()` | Always on macOS/Windows |
| `executable_dir()` | Always on macOS/Windows |
| User dirs (`desktop_dir`, etc.) | XDG entry deactivated or invalid |

## App-Specific Paths: Use `directories` Crate

For application-specific subdirectories with proper naming conventions, use the `directories` crate:

```rust
use directories::ProjectDirs;

// Platform-aware app directory structure
if let Some(proj_dirs) = ProjectDirs::from("com", "MyOrg", "MyApp") {
    let config = proj_dirs.config_dir();  // ~/.config/myapp (Linux)
    let cache = proj_dirs.cache_dir();    // ~/.cache/myapp (Linux)
    let data = proj_dirs.data_dir();      // ~/.local/share/myapp (Linux)
}
```

**Platform examples for `ProjectDirs::from("com", "Foo Corp", "Bar App")`:**

| Platform | Config Dir |
|----------|------------|
| Linux | `/home/alice/.config/barapp` |
| macOS | `/Users/Alice/Library/Application Support/com.Foo-Corp.Bar-App` |
| Windows | `C:\Users\Alice\AppData\Roaming\Foo Corp\Bar App` |

## Cargo.toml

```toml
[dependencies]
dirs = "6.0"

# For app-specific directories with organization/app naming:
directories = "6.0"
```

## Design Philosophy

- **No directory creation**: Only provides paths, never creates them
- **No existence checks**: Returns paths even if they don't exist yet
- **User-writable focus**: Only returns paths the user can write to
- **Platform-appropriate**: Uses native APIs (Known Folder on Windows, XDG on Linux)
- **Consistent semantics**: Same function returns equivalent-purpose paths across platforms

## Resources

- [dirs crate docs](https://docs.rs/dirs)
- [directories crate docs](https://docs.rs/directories)
- [XDG Base Directory Spec](https://specifications.freedesktop.org/basedir-spec/latest/)
- [macOS Library Directory](https://developer.apple.com/library/archive/documentation/FileManagement/Conceptual/FileSystemProgrammingGuide/FileSystemOverview/FileSystemOverview.html)
- [Windows Known Folders](https://docs.microsoft.com/en-us/windows/win32/shell/known-folders)
