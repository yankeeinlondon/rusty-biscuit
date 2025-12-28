---
name: thiserror
description: Expert knowledge for Rust error handling with thiserror crate - derive macros for custom error types, #[error] formatting, #[from] conversions, #[source] chaining, and comparison with anyhow/snafu
hash: 0818e6c807d7d386
---

# thiserror

A Rust crate for creating custom error types with minimal boilerplate using derive macros. Created by David Tolnay, it automatically implements `Error`, `Display`, and `From` traits.

**Use for:** Library development, public APIs, when callers need to match on specific error variants.

## Core Principles

- Use `thiserror` for libraries, `anyhow` for applications
- Always derive both `Error` and `Debug` together
- Use `#[error("...")]` for Display formatting with field interpolation
- Use `#[from]` for automatic `From` impl enabling `?` operator conversion
- Use `#[source]` to mark the underlying cause (for error chaining)
- Use `#[error(transparent)]` to delegate Display/source to wrapped error
- Design error enums as part of your public API contract
- Prefer struct variants over tuple variants for better error messages
- Keep error messages actionable and specific
- Include relevant context in error variant fields

## Quick Reference

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MyError {
    // Simple variant
    #[error("Operation failed")]
    Simple,

    // With field interpolation
    #[error("Invalid input: {input} (expected {expected})")]
    InvalidInput { input: String, expected: String },

    // Auto From impl with #[from]
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    // Explicit source without From
    #[error("Database query failed")]
    Database { #[source] source: sqlx::Error },

    // Transparent wrapper
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}
```

## Topics

### Core Usage

- [Error Definition Patterns](./error-patterns.md) - Enum variants, struct errors, field interpolation
- [Error Chaining](./error-chaining.md) - #[from], #[source], transparent errors

### Decision Making

- [Library vs Application](./library-vs-app.md) - When to use thiserror vs anyhow vs snafu

## Common Patterns

### Library Error Type

```rust
#[derive(Error, Debug)]
pub enum LibraryError {
    #[error("Invalid configuration: {0}")]
    Config(String),

    #[error("Network request failed: {0}")]
    Network(#[from] reqwest::Error),

    #[error("Authentication failed for user {user}")]
    Auth { user: String },
}
```

### Mapping Specific Error Cases

```rust
fn read_config(path: &str) -> Result<Config, ConfigError> {
    let content = std::fs::read_to_string(path).map_err(|e| match e.kind() {
        std::io::ErrorKind::NotFound => ConfigError::NotFound { path: path.into() },
        std::io::ErrorKind::PermissionDenied => ConfigError::PermissionDenied { path: path.into() },
        _ => ConfigError::Io(e),
    })?;
    // ...
}
```

### Generic Error Types

```rust
#[derive(Error, Debug)]
pub enum ProcessError<T: std::fmt::Debug> {
    #[error("Failed to process: {0:?}")]
    Processing(T),

    #[error("Validation failed: {0}")]
    Validation(String),
}
```

## Resources

- [Crate Documentation](https://docs.rs/thiserror)
- [GitHub Repository](https://github.com/dtolnay/thiserror)
- [Rust Error Handling Book](https://doc.rust-lang.org/book/ch09-00-error-handling.html)
