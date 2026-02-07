---
description: Parse and serialize TOML config files in Rust with serde, defaults, and helpful error messages
---

# TOML Crate Skill

The `toml` crate (v0.9.x) is the primary Rust library for TOML parsing and serialization with full serde integration.

## Quick Start

```toml
# Cargo.toml
[dependencies]
toml = "0.9"
serde = { version = "1.0", features = ["derive"] }
```

```rust
use serde::Deserialize;

#[derive(Deserialize)]
struct Config {
    server: ServerConfig,
    #[serde(default)]
    logging: LoggingConfig,
}

#[derive(Deserialize)]
struct ServerConfig {
    host: String,
    #[serde(default = "default_port")]
    port: u16,
}

#[derive(Deserialize, Default)]
struct LoggingConfig {
    #[serde(default)]
    level: String,
    #[serde(default)]
    file: Option<String>,
}

fn default_port() -> u16 { 8080 }

fn load_config(path: &str) -> Result<Config, toml::de::Error> {
    let content = std::fs::read_to_string(path).expect("read file");
    toml::from_str(&content)
}
```

## Core Functions

| Function | Purpose |
|----------|---------|
| `toml::from_str(&str)` | Deserialize TOML string to type |
| `toml::from_slice(&[u8])` | Deserialize TOML bytes to type |
| `toml::to_string(&T)` | Serialize to compact TOML |
| `toml::to_string_pretty(&T)` | Serialize to formatted TOML |
| `toml!{}` macro | Construct `toml::Table` inline |

## Feature Flags

| Flag | Description | Default |
|------|-------------|---------|
| `parse` | Enable parsing APIs | Yes |
| `display` | Enable formatting APIs | Yes |
| `preserve_order` | Keep key insertion order (uses `IndexMap`) | No |

## Documents

- [Default Values](./defaults.md) - `#[serde(default)]` patterns for optional fields
- [Error Handling](./errors.md) - Helpful error messages with line/column info
- [Config Locations](./config-locations.md) - Platform-specific paths with `directories` crate
- [Advanced Patterns](./advanced.md) - Flatten, rename, validation, toml_edit for format-preserving edits

## When to Use Which Crate

| Use Case | Crate |
|----------|-------|
| Read-only config parsing | `toml` (this skill) |
| Format-preserving editing | `toml_edit` |
| Schema validation/LSP | `taplo` |

## Error Display Example

```
TOML parse error at line 3, column 12
  |
3 | port = "not_a_number"
  |        ^^^^^^^^^^^^^^
expected integer
```

## Quick Patterns

**Optional with default:**
```rust
#[serde(default = "default_timeout")]
timeout_secs: u64,
```

**Nested config flattening:**
```rust
#[serde(flatten)]
database: DatabaseConfig,
```

**Backward-compatible rename:**
```rust
#[serde(alias = "old_name")]
new_name: String,
```

**Skip serialization of None:**
```rust
#[serde(skip_serializing_if = "Option::is_none")]
optional_field: Option<String>,
```
