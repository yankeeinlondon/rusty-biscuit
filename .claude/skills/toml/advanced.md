# Advanced TOML Patterns

## Serde Field Attributes

### Flatten Nested Configs

Merge nested struct fields into parent level:

```rust
#[derive(Deserialize)]
struct Config {
    name: String,

    #[serde(flatten)]
    database: DatabaseConfig,

    #[serde(flatten)]
    server: ServerConfig,
}

#[derive(Deserialize)]
struct DatabaseConfig {
    db_host: String,
    db_port: u16,
}

#[derive(Deserialize)]
struct ServerConfig {
    server_host: String,
    server_port: u16,
}
```

TOML (flat structure):
```toml
name = "myapp"
db_host = "localhost"
db_port = 5432
server_host = "0.0.0.0"
server_port = 8080
```

### Rename Fields

```rust
#[derive(Deserialize)]
struct Config {
    #[serde(rename = "log-level")]  // TOML uses kebab-case
    log_level: String,

    #[serde(rename = "max-connections")]
    max_connections: u32,
}
```

### Rename All Fields in Struct

```rust
#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
struct Config {
    log_level: String,      // matches "log-level" in TOML
    max_connections: u32,   // matches "max-connections" in TOML
}
```

### Backward-Compatible Aliases

```rust
#[derive(Deserialize)]
struct Config {
    #[serde(alias = "host")]  // Accept both "server" and "host"
    server: String,

    #[serde(alias = "num_workers", alias = "worker_count")]
    workers: u32,
}
```

## Format-Preserving Edits with toml_edit

When you need to modify TOML while preserving comments and formatting:

```toml
# Cargo.toml
[dependencies]
toml_edit = "0.22"
```

```rust
use toml_edit::{DocumentMut, value};

fn update_version(toml_content: &str, new_version: &str) -> String {
    let mut doc = toml_content.parse::<DocumentMut>()
        .expect("Invalid TOML");

    doc["package"]["version"] = value(new_version);

    doc.to_string()
}

// Preserves all comments and formatting!
```

### Add New Section

```rust
use toml_edit::{DocumentMut, table, value};

fn add_feature(toml: &str, name: &str, default: bool) -> String {
    let mut doc = toml.parse::<DocumentMut>().unwrap();

    // Create [features] if missing
    if doc.get("features").is_none() {
        doc["features"] = table();
    }

    doc["features"][name] = value(default);
    doc.to_string()
}
```

## Inline Table Serialization

Control how tables are serialized:

```rust
use serde::Serialize;

#[derive(Serialize)]
struct Dependency {
    version: String,
    features: Vec<String>,
}

#[derive(Serialize)]
struct Cargo {
    #[serde(serialize_with = "toml::ser::tables_last")]
    dependencies: std::collections::BTreeMap<String, Dependency>,
}
```

## Custom Deserialization

### Deserialize from Multiple Formats

```rust
use serde::{Deserialize, Deserializer};

#[derive(Deserialize)]
struct Config {
    #[serde(deserialize_with = "deserialize_duration")]
    timeout: std::time::Duration,
}

fn deserialize_duration<'de, D>(deserializer: D) -> Result<std::time::Duration, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum DurationFormat {
        Seconds(u64),
        String(String),  // "30s", "5m", "1h"
    }

    match DurationFormat::deserialize(deserializer)? {
        DurationFormat::Seconds(s) => Ok(std::time::Duration::from_secs(s)),
        DurationFormat::String(s) => parse_duration(&s)
            .map_err(serde::de::Error::custom),
    }
}

fn parse_duration(s: &str) -> Result<std::time::Duration, String> {
    // Parse "30s", "5m", "1h" format
    let (num, unit) = s.split_at(s.len() - 1);
    let value: u64 = num.parse().map_err(|_| "Invalid number")?;
    match unit {
        "s" => Ok(std::time::Duration::from_secs(value)),
        "m" => Ok(std::time::Duration::from_secs(value * 60)),
        "h" => Ok(std::time::Duration::from_secs(value * 3600)),
        _ => Err(format!("Unknown unit: {unit}")),
    }
}
```

## Spanned Values for Error Locations

Track source locations for better error messages:

```rust
use toml::Spanned;

#[derive(Deserialize)]
struct Config {
    port: Spanned<u16>,
}

fn validate_config(config: &Config) -> Result<(), String> {
    if *config.port.get_ref() == 0 {
        let span = config.port.span();
        return Err(format!(
            "Port cannot be 0 (at bytes {}..{})",
            span.start, span.end
        ));
    }
    Ok(())
}
```

## toml! Macro for Test Data

```rust
use toml::toml;

#[test]
fn test_config_parsing() {
    let table = toml! {
        [server]
        host = "localhost"
        port = 8080

        [database]
        url = "postgres://localhost/test"
    };

    let config: Config = table.try_into().unwrap();
    assert_eq!(config.server.port, 8080);
}
```

## Validation Patterns

### Post-Deserialization Validation

```rust
#[derive(Deserialize)]
#[serde(try_from = "RawConfig")]
struct Config {
    port: u16,
    workers: u32,
}

#[derive(Deserialize)]
struct RawConfig {
    port: u16,
    workers: u32,
}

impl TryFrom<RawConfig> for Config {
    type Error = String;

    fn try_from(raw: RawConfig) -> Result<Self, String> {
        if raw.port == 0 {
            return Err("port must be > 0".into());
        }
        if raw.workers == 0 {
            return Err("workers must be > 0".into());
        }
        Ok(Config {
            port: raw.port,
            workers: raw.workers,
        })
    }
}
```

### Builder Pattern with Defaults

```rust
#[derive(Deserialize)]
struct ConfigBuilder {
    host: Option<String>,
    port: Option<u16>,
}

struct Config {
    host: String,
    port: u16,
}

impl ConfigBuilder {
    fn build(self) -> Result<Config, String> {
        Ok(Config {
            host: self.host.unwrap_or_else(|| "localhost".into()),
            port: self.port.unwrap_or(8080),
        })
    }
}

fn load_config(path: &str) -> Result<Config, Box<dyn std::error::Error>> {
    let content = std::fs::read_to_string(path)?;
    let builder: ConfigBuilder = toml::from_str(&content)?;
    Ok(builder.build()?)
}
```

## Key Ordering with preserve_order

```toml
[dependencies]
toml = { version = "0.9", features = ["preserve_order"] }
```

```rust
use toml::Table;

// Keys maintain insertion order when serializing
let mut table = Table::new();
table.insert("z_last".into(), "value".into());
table.insert("a_first".into(), "value".into());

// With preserve_order: z_last comes before a_first
// Without: alphabetically sorted (a_first, z_last)
```
