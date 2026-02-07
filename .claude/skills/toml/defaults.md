# Default Values in TOML Config Parsing

Serde provides multiple strategies for handling missing fields when deserializing TOML.

## Field-Level Defaults

### Use Type's Default Implementation

```rust
#[derive(Deserialize)]
struct Config {
    #[serde(default)]  // Uses bool::default() = false
    enabled: bool,

    #[serde(default)]  // Uses String::default() = ""
    name: String,

    #[serde(default)]  // Uses Vec::default() = []
    items: Vec<String>,
}
```

### Custom Default Function

```rust
#[derive(Deserialize)]
struct ServerConfig {
    #[serde(default = "default_host")]
    host: String,

    #[serde(default = "default_port")]
    port: u16,

    #[serde(default = "default_timeout")]
    timeout_secs: u64,
}

fn default_host() -> String { "127.0.0.1".to_string() }
fn default_port() -> u16 { 8080 }
fn default_timeout() -> u64 { 30 }
```

### Associated Function or Trait Method

```rust
#[derive(Deserialize)]
struct Priority {
    #[serde(default = "Priority::normal")]
    level: u8,
}

impl Priority {
    fn normal() -> u8 { 5 }
}
```

## Struct-Level Default

Apply `#[serde(default)]` to the entire struct to use `Default::default()` for all missing fields:

```rust
#[derive(Deserialize, Default)]
#[serde(default)]  // Entire struct uses Default if section missing
struct LoggingConfig {
    level: String,      // Uses String::default()
    file: Option<String>, // Uses None
    max_size_mb: u32,   // Uses 0
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            file: None,
            max_size_mb: 100,
        }
    }
}
```

## Option vs Default

| Pattern | TOML Missing | Behavior |
|---------|--------------|----------|
| `field: Option<T>` | Absent | `None` |
| `#[serde(default)] field: T` | Absent | `T::default()` |
| `#[serde(default = "fn")] field: T` | Absent | `fn()` |

**Option does NOT need `#[serde(default)]`** - serde automatically treats missing optional fields as `None`.

```rust
#[derive(Deserialize)]
struct Config {
    // No #[serde(default)] needed - Option handles missing
    database_url: Option<String>,

    // Required: must be in TOML or error
    api_key: String,

    // Optional with non-None default
    #[serde(default = "default_retries")]
    retries: u32,
}

fn default_retries() -> u32 { 3 }
```

## Combining with skip_serializing_if

When round-tripping configs, avoid serializing default values:

```rust
#[derive(Serialize, Deserialize)]
struct Config {
    #[serde(default, skip_serializing_if = "is_default_port")]
    port: u16,
}

fn is_default_port(port: &u16) -> bool { *port == 8080 }
```

Or use the common `Option` pattern:

```rust
#[derive(Serialize, Deserialize)]
struct Config {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    custom_path: Option<String>,
}
```

## Environment Variable Override Pattern

Common pattern: TOML provides defaults, env vars override:

```rust
use std::env;

#[derive(Deserialize)]
struct Config {
    #[serde(default = "default_log_level")]
    log_level: String,
}

fn default_log_level() -> String {
    env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string())
}
```

## Nested Defaults

For deeply nested configs, implement `Default` at each level:

```rust
#[derive(Deserialize, Default)]
#[serde(default)]
struct AppConfig {
    server: ServerConfig,
    database: DatabaseConfig,
}

#[derive(Deserialize)]
#[serde(default)]
struct ServerConfig {
    host: String,
    port: u16,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 3000,
        }
    }
}

#[derive(Deserialize)]
#[serde(default)]
struct DatabaseConfig {
    url: String,
    pool_size: u32,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: "postgres://localhost/app".to_string(),
            pool_size: 10,
        }
    }
}
```

This allows a completely empty TOML file to parse successfully with all defaults.
