# Error Chaining

Patterns for connecting errors and building error chains with thiserror.

## The #[from] Attribute

Automatically implements `From<T>` for your error type, enabling the `?` operator:

```rust
use thiserror::Error;
use std::io;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Could not read configuration file: {0}")]
    Io(#[from] io::Error),

    #[error("Could not parse configuration: {0}")]
    Parse(#[from] serde_json::Error),
}

fn load_config(path: &str) -> Result<Config, ConfigError> {
    let content = std::fs::read_to_string(path)?;  // io::Error -> ConfigError::Io
    let config = serde_json::from_str(&content)?;  // serde_json::Error -> ConfigError::Parse
    Ok(config)
}
```

**Key points:**
- `#[from]` implies `#[source]` - the source() method returns the wrapped error
- Only one field per variant can have `#[from]`
- The variant must have exactly one field (tuple or named)

## The #[source] Attribute

Explicitly marks a field as the error source without generating `From`:

```rust
#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("Query execution failed")]
    QueryFailed {
        query: String,
        #[source]
        source: sqlx::Error,
    },
}

// Must manually construct - no automatic From conversion
fn run_query(query: &str) -> Result<(), DatabaseError> {
    sqlx::query(query)
        .execute(&pool)
        .await
        .map_err(|e| DatabaseError::QueryFailed {
            query: query.to_string(),
            source: e,
        })?;
    Ok(())
}
```

**When to use:** When you need additional context fields alongside the source error.

## Combining #[from] and #[source]

For tuple variants, `#[from]` implies `#[source]`. For struct variants with a single field, you can be explicit:

```rust
#[derive(Error, Debug)]
pub enum MyError {
    // #[from] implies #[source] for tuple variant
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    // Explicit #[source] with #[from] for clarity
    #[error("IO operation failed")]
    Io {
        #[from]
        #[source]
        source: std::io::Error,
    },
}
```

## Transparent Errors

Delegate Display and source() entirely to the underlying error:

```rust
#[derive(Error, Debug)]
pub enum ServiceError {
    #[error("Database error: {0}")]
    Database(#[from] DatabaseError),

    // Transparent - uses inner error's Display and source directly
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}
```

**When to use:**
- Wrapping errors without adding context
- Creating "catch-all" variants
- Bridging between error types

## Building Error Stacks

For complex systems, design errors to carry contextual information:

```rust
#[derive(Error, Debug)]
pub enum ProcessingError {
    #[error("Failed to process record {record_id} in batch {batch_id}")]
    RecordFailed {
        record_id: u64,
        batch_id: u64,
        #[source]
        source: RecordError,
    },
}

#[derive(Error, Debug)]
pub enum RecordError {
    #[error("Validation failed: {0}")]
    Validation(String),

    #[error("Transform error")]
    Transform(#[source] TransformError),
}
```

This creates a chain: `ProcessingError` -> `RecordError` -> `TransformError`

## Iterating Error Chains

Use the source() method to walk the chain:

```rust
fn log_error_chain(error: &dyn std::error::Error) {
    eprintln!("Error: {}", error);
    let mut source = error.source();
    while let Some(e) = source {
        eprintln!("  Caused by: {}", e);
        source = e.source();
    }
}
```

## Related

- [Error Definition Patterns](./error-patterns.md) - Defining error types and variants
- [Library vs Application](./library-vs-app.md) - Choosing the right error strategy
