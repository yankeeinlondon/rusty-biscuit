# TOML Error Handling

The `toml` crate provides detailed error messages with source location information for debugging parse failures.

## Error Types

```rust
use toml::de::Error;  // Deserialization errors
use toml::ser::Error; // Serialization errors (rare)
```

## Error Information

The `toml::de::Error` type provides:

| Method | Returns | Description |
|--------|---------|-------------|
| `message()` | `&str` | Human-readable error description |
| `span()` | `Option<Range<usize>>` | Byte range in source where error occurred |
| `Display` | Formatted string | Full error with line/column context |

## Error Display Format

When printed, errors show precise location with visual context:

```
TOML parse error at line 3, column 12
  |
3 | port = "not_a_number"
  |        ^^^^^^^^^^^^^^
expected integer
```

For nested parsing errors:

```
TOML parse error at line 1, column 10
  | 1 | 00:32:00.a999999
  |            ^ Unexpected `a`
Expected `digit`
While parsing a Time
While parsing a Date-Time
```

## Basic Error Handling

```rust
use std::fs;
use toml::de::Error as TomlError;

fn load_config(path: &str) -> Result<Config, TomlError> {
    let content = fs::read_to_string(path)
        .expect("failed to read file");
    toml::from_str(&content)
}

fn main() {
    match load_config("config.toml") {
        Ok(config) => println!("Loaded: {:?}", config),
        Err(e) => {
            // Error already includes line/column context
            eprintln!("Config error: {e}");
        }
    }
}
```

## Enhanced Error Reporting with color-eyre

```rust
use color_eyre::{eyre::WrapErr, Result};
use std::fs;

fn load_config(path: &str) -> Result<Config> {
    let content = fs::read_to_string(path)
        .wrap_err_with(|| format!("Failed to read config file: {path}"))?;

    toml::from_str(&content)
        .wrap_err("Invalid TOML configuration")
}
```

## Custom Error Type Integration

With `thiserror`:

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Failed to read config file: {path}")]
    IoError {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("Invalid TOML in {path}: {source}")]
    ParseError {
        path: String,
        #[source]
        source: toml::de::Error,
    },

    #[error("Missing required field: {0}")]
    MissingField(String),
}

fn load_config(path: &str) -> Result<Config, ConfigError> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| ConfigError::IoError {
            path: path.to_string(),
            source: e,
        })?;

    toml::from_str(&content)
        .map_err(|e| ConfigError::ParseError {
            path: path.to_string(),
            source: e,
        })
}
```

## Extracting Span for Custom Formatting

```rust
fn format_error(content: &str, error: &toml::de::Error) -> String {
    let mut msg = error.message().to_string();

    if let Some(span) = error.span() {
        // Calculate line and column from byte offset
        let prefix = &content[..span.start];
        let line = prefix.matches('\n').count() + 1;
        let col = prefix.rfind('\n')
            .map(|pos| span.start - pos)
            .unwrap_or(span.start + 1);

        msg = format!("Line {line}, column {col}: {msg}");
    }

    msg
}
```

## Validation After Parsing

For semantic validation (beyond TOML syntax):

```rust
#[derive(Deserialize)]
struct Config {
    port: u16,
    max_connections: u32,
}

impl Config {
    fn validate(&self) -> Result<(), ConfigError> {
        if self.port == 0 {
            return Err(ConfigError::MissingField("port must be > 0".into()));
        }
        if self.max_connections > 10000 {
            return Err(ConfigError::MissingField(
                "max_connections cannot exceed 10000".into()
            ));
        }
        Ok(())
    }
}

fn load_and_validate(path: &str) -> Result<Config, ConfigError> {
    let config: Config = load_config(path)?;
    config.validate()?;
    Ok(config)
}
```

## Common Parse Errors

| Error | Cause | Fix |
|-------|-------|-----|
| `expected integer` | String where number expected | Remove quotes: `port = 8080` |
| `expected string` | Unquoted string with special chars | Add quotes: `path = "/var/log"` |
| `duplicate key` | Same key defined twice | Remove duplicate |
| `invalid escape` | Bad escape sequence | Use raw strings: `path = 'C:\Users'` |
| `unexpected character` | Invalid TOML syntax | Check for typos |

## Testing Error Messages

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invalid_port_type() {
        let toml = r#"port = "not_a_number""#;
        let err = toml::from_str::<Config>(toml).unwrap_err();

        assert!(err.message().contains("expected integer"));
        assert!(err.span().is_some());
    }

    #[test]
    fn test_missing_required_field() {
        let toml = r#"optional = true"#;
        let err = toml::from_str::<Config>(toml).unwrap_err();

        assert!(err.message().contains("missing field"));
    }
}
```
