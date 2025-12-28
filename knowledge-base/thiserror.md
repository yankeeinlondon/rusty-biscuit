---
name: thiserror
description: Comprehensive guide to Rust's thiserror crate for custom error types
created: 2025-12-08
hash: 292959cc19f00d45
tags:
  - rust
  - error-handling
  - thiserror
  - derive-macro
  - library-development
---

# Rust's `thiserror` Crate

The `thiserror` crate is a specialized error handling library designed for Rust developers who need to create well-structured custom error types with minimal boilerplate. Created by David Tolnay, `thiserror` provides a derive macro that automatically implements the standard error traits, making it an essential tool for library authors who want to expose precise error information to their users while maintaining Rust's strict type safety guarantees.

## Table of Contents

- [Rust's `thiserror` Crate](#rusts-thiserror-crate)
  - [Table of Contents](#table-of-contents)
  - [Why thiserror](#why-thiserror)
  - [Core Features](#core-features)
    - [Derive Macro Implementation](#derive-macro-implementation)
    - [Error Message Formatting](#error-message-formatting)
    - [Error Chaining with #\[source\]](#error-chaining-with-source)
    - [Automatic From Implementation](#automatic-from-implementation)
    - [Transparent Error Wrapping](#transparent-error-wrapping)
  - [Usage Patterns](#usage-patterns)
    - [Library Development](#library-development)
    - [Error Chain Propagation](#error-chain-propagation)
    - [Integration with Standard Library Errors](#integration-with-standard-library-errors)
    - [Generic Error Types](#generic-error-types)
  - [Comparison with Other Libraries](#comparison-with-other-libraries)
    - [thiserror vs anyhow](#thiserror-vs-anyhow)
    - [thiserror vs snafu](#thiserror-vs-snafu)
    - [Comparison Summary Table](#comparison-summary-table)
  - [Decision Guide](#decision-guide)
    - [When to Choose thiserror](#when-to-choose-thiserror)
    - [When to Choose Alternatives](#when-to-choose-alternatives)
    - [Hybrid Approaches](#hybrid-approaches)
  - [Best Practices](#best-practices)
  - [Quick Reference](#quick-reference)
  - [Resources](#resources)

## Why thiserror

The crate addresses a fundamental challenge in Rust development: the need to create descriptive, actionable errors without writing repetitive implementation code. Where Rust's standard library provides the foundation through the `std::error::Error` trait, `thiserror` builds upon this to offer a declarative approach to error type definition that integrates seamlessly with Rust's type system and error propagation mechanisms.

The primary goal is to reduce boilerplate, especially the repetitive implementation of `Display`, `Debug`, and `Error` traits that every custom error type requires.

## Core Features

### Derive Macro Implementation

At the heart of `thiserror` is the `#[derive(Error)]` macro which automatically implements several important traits for custom error types:

- **`std::error::Error`**: The fundamental trait for error handling in Rust
- **`std::fmt::Display`**: Implemented based on the provided `#[error(...)]` attributes
- **`std::fmt::Debug`**: Derived as usual (you still need `#[derive(Debug)]`)

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MyError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Parse error: {0}")]
    Parse(#[from] std::num::ParseIntError),

    #[error("Custom error: {msg}")]
    Custom { msg: String },
}
```

### Error Message Formatting

The `#[error]` attribute provides powerful formatting capabilities using standard Rust formatting syntax:

**Simple variants:**

```rust
#[derive(Error, Debug)]
pub enum MyError {
    #[error("Invalid configuration value provided.")]
    InvalidConfig,
}
```

**Tuple variants (use positional indices):**

```rust
#[derive(Error, Debug)]
pub enum MyError {
    #[error("Failed to read file '{0}'.")]
    IoError(String),
}
```

**Struct variants (use field names):**

```rust
#[derive(Error, Debug)]
pub enum MyError {
    #[error("Invalid header. Expected: {expected}, found: {found}")]
    InvalidHeader { expected: String, found: String },

    #[error("Resource limit of {limit} exceeded for user '{user}'.")]
    ResourceLimitExceeded { limit: usize, user: String },
}
```

### Error Chaining with #[source]

To create an error chain where one error is caused by another, use the `#[source]` attribute on a field. The type of this field must implement `std::error::Error`. This automatically implements the `source()` method:

```rust
use std::io;

#[derive(thiserror::Error, Debug)]
pub enum MyError {
    #[error("Database query failed.")]
    DatabaseError {
        #[source]
        source: sqlx::Error,
    },

    #[error("File operation error.")]
    IoOperation(#[source] io::Error),
}
```

### Automatic From Implementation

The `#[from]` attribute automatically implements `From<T>` for your error type, where `T` is the type of the field. This enables using the `?` operator to convert the source error into your custom error type:

```rust
use std::io;

#[derive(thiserror::Error, Debug)]
pub enum ConfigError {
    #[error("Could not read configuration file: {0}")]
    Io(#[from] io::Error),

    #[error("Could not parse configuration: {0}")]
    Parse(#[from] serde_json::Error),
}

// Now you can use `?` and errors automatically convert
fn load_config(path: &str) -> Result<Config, ConfigError> {
    let contents = std::fs::read_to_string(path)?; // io::Error -> ConfigError::Io
    let config = serde_json::from_str(&contents)?; // serde_json::Error -> ConfigError::Parse
    Ok(config)
}
```

> **Note:** The `#[from]` attribute implies `#[source]` for enum variants, so you typically do not need to specify both.

### Transparent Error Wrapping

The `#[error(transparent)]` attribute delegates the `Display` and `source` methods entirely to the underlying error type. This is useful for wrapping errors without adding new context:

```rust
#[derive(Error, Debug)]
pub enum MyError {
    #[error(transparent)]
    Underlying(#[from] SomeOtherError),
}
```

## Usage Patterns

### Library Development

For library authors, `thiserror` provides the ideal balance between precision and ergonomics:

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum LibraryError {
    #[error("Invalid input: {input} cannot be processed")]
    InvalidInput { input: String },

    #[error("Network failure: {0}")]
    Network(#[from] reqwest::Error),

    #[error("Authentication failed")]
    Authentication,
}

pub fn authenticate_user(user: &str) -> Result<(), LibraryError> {
    // Implementation that may return LibraryError
    todo!()
}
```

Consumers of the library can then match on specific error variants:

```rust
match parse_file("data.txt") {
    Err(ParserError::Io { .. }) => { /* retry logic */ },
    Err(ParserError::ParseInt { input, .. }) => { /* notify about bad data */ },
    Ok(_) => { /* success */ },
}
```

### Error Chain Propagation

The crate integrates seamlessly with Rust's `?` operator for error propagation:

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Configuration error: {0}")]
    Config(#[from] config::ConfigError),
}

fn query_user(id: u32) -> Result<User, AppError> {
    let user = sqlx::query_as!(User, "SELECT * FROM users WHERE id = ?", id)
        .fetch_one(&pool)?; // Automatically converts sqlx::Error to AppError
    Ok(user)
}
```

### Integration with Standard Library Errors

`thiserror` works seamlessly with standard library error types, allowing for comprehensive error handling:

```rust
use std::fs::File;
use std::io::{self, Read};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum FileError {
    #[error("File not found: {path}")]
    NotFound { path: String },

    #[error("Permission denied: {path}")]
    PermissionDenied { path: String },

    #[error("IO error: {0}")]
    Io(#[from] io::Error),
}

fn read_file_contents(path: &str) -> Result<String, FileError> {
    let mut file = File::open(path).map_err(|e| match e.kind() {
        io::ErrorKind::NotFound => FileError::NotFound { path: path.to_string() },
        io::ErrorKind::PermissionDenied => FileError::PermissionDenied { path: path.to_string() },
        _ => FileError::Io(e),
    })?;

    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    Ok(contents)
}
```

### Generic Error Types

`thiserror` supports creating error types that work with generic parameters:

```rust
#[derive(Error, Debug)]
pub enum GenericError<T: std::fmt::Debug> {
    #[error("Failed to process {0:?}")]
    ProcessingError(T),

    #[error("Invalid configuration: {0}")]
    ConfigError(String),
}
```

## Comparison with Other Libraries

### thiserror vs anyhow

The Rust ecosystem commonly pairs these two crates together, each serving a distinct purpose:

**thiserror** is designed for library authors who need to define precise error types that consumers can programmatically handle. It provides compile-time guarantees about which errors can occur.

**anyhow** is designed for application developers who prioritize ergonomics and flexibility over precise error typing. It wraps any error in a dynamic type and provides excellent context addition capabilities.

```rust
// thiserror approach - precise error types for libraries
#[derive(Error, Debug)]
pub enum MyError {
    #[error("Invalid configuration: {0}")]
    Config(#[from] config::ConfigError),
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
}

// anyhow approach - dynamic error type for applications
use anyhow::{Context, Result};

fn fetch_data() -> Result<String> {
    let config = load_config().context("Failed to load configuration")?;
    let data = reqwest::get(&config.url)?.text()?;
    Ok(data)
}
```

### thiserror vs snafu

Both crates offer similar functionality but with different approaches:

**snafu** provides more granular control over error context generation but requires more verbose code. It uses an explicit context generation pattern and is designed for complex systems where detailed, chainable error context is critical.

**thiserror** focuses on derive macros for a more concise approach.

```rust
// thiserror approach
#[derive(Error, Debug)]
pub enum MyError {
    #[error("User not found: {id}")]
    UserNotFound { id: u32 },
}

// snafu approach
use snafu::Snafu;

#[derive(Debug, Snafu)]
pub enum MyError {
    #[snafu(display("User not found: {}", id))]
    UserNotFound { id: u32 },
}
```

### Comparison Summary Table

| Feature | thiserror | anyhow | snafu |
|---------|-----------|--------|-------|
| **Core Philosophy** | Define static, custom error types for precise handling | Use a unified, dynamic error type for simplicity | Add structured context to errors for complex systems |
| **Error Type** | Your own `enum` or `struct` | Single `anyhow::Error` type (a trait object) | Your own types, generated with context |
| **Primary Use Case** | Library development | Application development | Complex systems |
| **Context Support** | Via fields in error variants | Dynamic strings via `.context()` method | Structured context via attributes |
| **Learning Curve** | Medium | Low | High |
| **Performance** | Zero-cost | Minimal overhead | Zero-cost |
| **Compile-time Checks** | Strong | Weak | Strong |
| **Backtrace Support** | Optional | Built-in | Optional |

## Decision Guide

### When to Choose thiserror

Ideal scenarios for using thiserror:

- **Library development**: When creating reusable components where consumers need to handle specific error cases
- **Public APIs**: When exposing error types as part of a public interface
- **Type safety**: When compile-time guarantees about error handling are important
- **Performance-critical code**: When zero-cost abstractions are necessary

```rust
// Example: Library error type
#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("Connection failed: {0}")]
    Connection(#[from] sqlx::Error),

    #[error("Query failed: {query}")]
    QueryFailed { query: String },

    #[error("Transaction aborted")]
    TransactionAborted,
}
```

### When to Choose Alternatives

**Consider anyhow when:**

- Building applications where error handling does not require precise typing
- Rapid prototyping when you need to move quickly without defining error types
- Adding rich context to errors from multiple sources is important

**Consider snafu when:**

- You need more control over context generation
- Building complex systems where detailed error stacks are critical
- Migrating from error-chain

### Hybrid Approaches

Many Rust projects use a combination of error handling strategies:

```rust
// Library using thiserror
use thiserror::Error;

#[derive(Error, Debug)]
pub enum LibraryError {
    #[error("Processing failed: {0}")]
    Processing(#[from] std::io::Error),
}

// Application using anyhow to consume library
use anyhow::{Context, Result};

fn process_data() -> Result<()> {
    my_library::process()
        .context("Failed to process user data")?;
    Ok(())
}
```

## Best Practices

1. **Use thiserror for libraries, anyhow for applications** - This simple guideline provides excellent results in most scenarios.

2. **Design error types for consumers** - Think about what information callers need to handle errors programmatically.

3. **Keep error messages actionable** - Include relevant context like file paths, user IDs, or operation names.

4. **Consider the "virtual error stack" pattern** - For complex applications, attach lightweight, structured context (like query IDs or operation stages) at multiple points as an error propagates. This provides more debuggable information than a root cause alone and is more performant than capturing full system backtraces.

5. **Use #[from] judiciously** - Only add automatic conversions where they make semantic sense. Not every underlying error should automatically convert.

## Quick Reference

| Attribute | Purpose | Example |
|-----------|---------|---------|
| `#[derive(Error)]` | Implement Error trait | `#[derive(Error, Debug)]` |
| `#[error("...")]` | Define Display message | `#[error("File not found: {path}")]` |
| `#[from]` | Auto-implement From trait | `Io(#[from] io::Error)` |
| `#[source]` | Mark source error field | `source: #[source] SqlError` |
| `#[error(transparent)]` | Delegate to underlying error | `#[error(transparent)]` |

**Common patterns:**

```rust
// Simple variant
#[error("Operation failed")]
OperationFailed,

// Tuple variant with positional formatting
#[error("Invalid value: {0}")]
InvalidValue(String),

// Struct variant with named fields
#[error("User {user_id} not found in {database}")]
UserNotFound { user_id: u32, database: String },

// Automatic From conversion
#[error("IO error: {0}")]
Io(#[from] std::io::Error),

// Transparent wrapper
#[error(transparent)]
Other(#[from] anyhow::Error),
```

## Resources

- [thiserror on crates.io](https://crates.io/crates/thiserror)
- [thiserror GitHub repository](https://github.com/dtolnay/thiserror)
- [anyhow crate (companion for applications)](https://crates.io/crates/anyhow)
- [Rust Error Handling Book](https://doc.rust-lang.org/book/ch09-00-error-handling.html)
- [snafu crate (alternative with structured context)](https://crates.io/crates/snafu)
