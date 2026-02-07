# Platform-Specific Config File Locations

Use the `directories` crate to find standard config paths across operating systems.

## Setup

```toml
[dependencies]
directories = "5.0"
toml = "0.9"
serde = { version = "1.0", features = ["derive"] }
```

## ProjectDirs for Application Config

```rust
use directories::ProjectDirs;
use std::path::PathBuf;

fn config_path(app_name: &str) -> Option<PathBuf> {
    ProjectDirs::from("com", "MyCompany", app_name)
        .map(|dirs| dirs.config_dir().join("config.toml"))
}
```

## Platform-Specific Paths

Given `ProjectDirs::from("com", "Acme", "MyApp")`:

| Platform | `config_dir()` |
|----------|----------------|
| Linux | `~/.config/myapp/` |
| macOS | `~/Library/Application Support/com.Acme.MyApp/` |
| Windows | `C:\Users\Alice\AppData\Roaming\Acme\MyApp\config\` |

## Directory Types

| Method | Purpose | Linux | macOS | Windows |
|--------|---------|-------|-------|---------|
| `config_dir()` | User config | `~/.config/app` | `~/Library/Application Support` | `AppData\Roaming` |
| `config_local_dir()` | Machine-local config | `~/.config/app` | `~/Library/Application Support` | `AppData\Local` |
| `data_dir()` | Application data | `~/.local/share/app` | `~/Library/Application Support` | `AppData\Roaming` |
| `cache_dir()` | Temporary cache | `~/.cache/app` | `~/Library/Caches` | `AppData\Local` |

## Complete Config Loading Pattern

```rust
use directories::ProjectDirs;
use serde::Deserialize;
use std::{env, fs, path::PathBuf};

#[derive(Deserialize, Default)]
#[serde(default)]
pub struct Config {
    pub llama_models_dir: Option<String>,
    pub log_level: String,
}

impl Config {
    /// Load config with priority: env vars > config file > defaults
    pub fn load() -> Self {
        let mut config = Self::from_file().unwrap_or_default();

        // Environment variables override file config
        if let Ok(dir) = env::var("LLAMA_MODELS_DIR") {
            config.llama_models_dir = Some(dir);
        }
        if let Ok(level) = env::var("LOG_LEVEL") {
            config.log_level = level;
        }

        config
    }

    fn from_file() -> Option<Self> {
        let path = Self::config_path()?;
        let content = fs::read_to_string(&path).ok()?;
        toml::from_str(&content).ok()
    }

    pub fn config_path() -> Option<PathBuf> {
        ProjectDirs::from("", "", "model-citizen")
            .map(|dirs| dirs.config_dir().join("config.toml"))
    }

    /// Create config directory if it doesn't exist
    pub fn ensure_config_dir() -> std::io::Result<PathBuf> {
        if let Some(dirs) = ProjectDirs::from("", "", "model-citizen") {
            let config_dir = dirs.config_dir();
            fs::create_dir_all(config_dir)?;
            Ok(config_dir.to_path_buf())
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Could not determine config directory"
            ))
        }
    }
}
```

## XDG Base Directory Support (Linux)

The `directories` crate respects XDG environment variables on Linux:

| Variable | Overrides |
|----------|-----------|
| `XDG_CONFIG_HOME` | `config_dir()` |
| `XDG_DATA_HOME` | `data_dir()` |
| `XDG_CACHE_HOME` | `cache_dir()` |

```rust
// Respects XDG_CONFIG_HOME if set
let config_dir = ProjectDirs::from("", "", "myapp")
    .map(|d| d.config_dir().to_path_buf());
```

## Multiple Config Locations (Layered Config)

```rust
use std::path::PathBuf;

fn config_search_paths(app_name: &str) -> Vec<PathBuf> {
    let mut paths = vec![];

    // 1. Current directory (development)
    paths.push(PathBuf::from("config.toml"));

    // 2. User config directory
    if let Some(dirs) = ProjectDirs::from("", "", app_name) {
        paths.push(dirs.config_dir().join("config.toml"));
    }

    // 3. System-wide config (Linux/macOS only)
    #[cfg(unix)]
    paths.push(PathBuf::from(format!("/etc/{}/config.toml", app_name)));

    paths
}

fn load_first_config<T: for<'de> serde::Deserialize<'de>>(
    paths: &[PathBuf]
) -> Option<T> {
    for path in paths {
        if let Ok(content) = std::fs::read_to_string(path) {
            if let Ok(config) = toml::from_str(&content) {
                return Some(config);
            }
        }
    }
    None
}
```

## CLI Override Pattern

Integrate with clap for CLI flag overrides:

```rust
use clap::Parser;

#[derive(Parser)]
struct Cli {
    /// Config file path (overrides default location)
    #[arg(short, long)]
    config: Option<PathBuf>,

    /// Model directory (overrides config file)
    #[arg(long, env = "LLAMA_MODELS_DIR")]
    models_dir: Option<String>,
}

fn main() {
    let cli = Cli::parse();

    // Priority: CLI flag > env var > config file > default
    let config_path = cli.config
        .or_else(Config::config_path)
        .expect("No config path");

    let mut config = Config::from_path(&config_path)
        .unwrap_or_default();

    // CLI/env override
    if let Some(dir) = cli.models_dir {
        config.llama_models_dir = Some(dir);
    }
}
```

## Example Config File

`~/.config/model-citizen/config.toml`:

```toml
# Model Citizen Configuration

# Path to llama.cpp models (can also set LLAMA_MODELS_DIR env var)
llama_models_dir = "/home/user/models/llama"

# Logging level: error, warn, info, debug, trace
log_level = "info"

[ollama]
# Custom Ollama host (default: localhost:11434)
host = "localhost"
port = 11434

[lmstudio]
# Custom LM Studio API port (default: 1234)
port = 1234
```
