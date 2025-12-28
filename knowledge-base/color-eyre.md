---
name: color-eyre
description: Comprehensive guide to the color-eyre Rust crate for beautiful, diagnostic error handling
created: 2025-12-08
hash: a5d06fc570ba04f2
tags:
  - rust
  - error-handling
  - cli
  - debugging
  - eyre
---

# color-eyre: Beautiful Error Handling for Rust Applications

`color-eyre` is a Rust crate for application-level error handling that transforms error reporting from cryptic stack traces into beautiful, actionable diagnostic output. Built as an extension of the `eyre` crate (itself a fork of `anyhow`), it provides colorized, multi-line error reports complete with backtraces, source code snippets, and user-defined help text.

If you are building a CLI tool, server, or any application where developers or users will see error output directly, `color-eyre` dramatically improves the debugging experience.

## Table of Contents

- [Getting Started](#getting-started)
- [Core Concepts](#core-concepts)
- [Adding Context to Errors](#adding-context-to-errors)
- [Help Text and Notes](#help-text-and-notes)
- [Error Report Output](#error-report-output)
- [Configuration and Customization](#configuration-and-customization)
- [SpanTrace Integration](#spantrace-integration)
- [Comparison with Other Crates](#comparison-with-other-crates)
- [Best Practices](#best-practices)
- [Quick Reference](#quick-reference)
- [Resources](#resources)

## Getting Started

### Installation

Add `color-eyre` to your `Cargo.toml`:

```toml
[dependencies]
color-eyre = "0.6"
```

### Basic Setup

Install the error and panic hooks at the very beginning of your `main` function:

```rust
use color_eyre::eyre::Result;

fn main() -> Result<()> {
    // Install the global panic and error report handlers.
    // This should be the first thing in main.
    color_eyre::install()?;

    // ... your application logic here ...

    Ok(())
}
```

The `install()` function sets up:

- **Error Reporting Hook**: Customizes how `eyre::Report` errors are formatted when returned from `main`
- **Panic Hook**: Replaces the default Rust panic handler to produce the same colorful, detailed reports for panics

## Core Concepts

### The Result Type Alias

`color-eyre` provides a convenient type alias that simplifies function signatures:

```rust
pub type Result<T, E = Report> = std::result::Result<T, E>;
```

This allows you to write `eyre::Result<T>` instead of `std::result::Result<T, MyErrorType>`:

```rust
use color_eyre::eyre::Result;

fn do_something() -> Result<String> {
    // The error type defaults to color_eyre::Report
    Ok("success".to_string())
}
```

### The Report Type

`eyre::Report` is a dynamic, type-erased error container. It can hold any error type that implements `std::error::Error`, making it ideal for applications that need to handle errors from many different sources without creating elaborate error type hierarchies.

## Adding Context to Errors

### The WrapErr Trait

The `WrapErr` trait (from `eyre`) adds context to errors while preserving the original error in a chain:

```rust
use color_eyre::eyre::{WrapErr, Result};
use std::fs;
use std::path::Path;

fn read_file_contents(path: &Path) -> Result<String> {
    fs::read_to_string(path)
        .wrap_err_with(|| format!("Failed to read file from path: {}", path.display()))
}
```

**Why use `wrap_err` instead of `map_err`?**

- **Preserves the original error**: The source `std::io::Error` is not lost; it becomes part of the error chain
- **Creates a breadcrumb trail**: Call `.wrap_err()` at multiple levels to build a complete picture of what failed
- **Ergonomic syntax**: Clean and readable compared to manual `format!` calls

### Building Error Chains

You can chain multiple `.wrap_err()` calls to create detailed error reports:

```rust
use color_eyre::eyre::{WrapErr, Result};
use std::fs;
use std::path::Path;

fn read_config(path: &Path) -> Result<serde_json::Value> {
    let content = fs::read_to_string(path)
        .wrap_err_with(|| format!("Failed to read config file at {}", path.display()))?;

    serde_json::from_str(&content)
        .wrap_err("The config file is not valid JSON")
}

fn initialize_app(config_path: &Path) -> Result<()> {
    let config = read_config(config_path)
        .wrap_err("Could not load application configuration")?;

    // ... use config ...
    Ok(())
}
```

### Creating New Errors

Use the `eyre!` macro to create new errors from scratch:

```rust
use color_eyre::eyre::{eyre, Result};

fn validate_input(value: &str) -> Result<()> {
    if value.is_empty() {
        return Err(eyre!("Input cannot be empty"));
    }
    Ok(())
}
```

## Help Text and Notes

One of `color-eyre`'s unique features is the ability to attach help text and notes to errors. This is invaluable for guiding users toward solutions.

### Adding Help Text

Use the `Help` trait to add suggestions:

```rust
use color_eyre::eyre::{WrapErr, Result, Help};
use std::env;

fn get_api_key() -> Result<String> {
    env::var("API_KEY")
        .map_err(|e| color_eyre::eyre::Report::new(e))
        .with_help(|| "Set the API_KEY environment variable to access the service. \
                        For example: `export API_KEY=your_secret_key`")
}
```

### Adding Notes

Notes provide additional context without being prescriptive:

```rust
use color_eyre::eyre::{WrapErr, Result, Help};

fn parse_config(content: &str) -> Result<Config> {
    serde_json::from_str(content)
        .wrap_err("Failed to parse configuration")
        .with_note(|| format!("File content was:\n---\n{}\n---", content))
}
```

### Sections (SectionExt)

The `SectionExt` trait provides methods to attach custom warnings or suggestions that are displayed independently from the error chain:

```rust
use color_eyre::eyre::Result;
use color_eyre::Section;

fn load_config() -> Result<Config> {
    // ... error handling ...
    Err(eyre!("Configuration missing"))
        .section("Check your configuration file at /path/to/config.toml")
}
```

## Error Report Output

### Report Structure

When an error is printed, `color-eyre` produces a structured, multi-line report containing:

1. **The Error Chain**: Each layer of context added by `.wrap_err()`, from most recent to original source
2. **The Root Cause**: The underlying error that started the chain
3. **Notes and Help**: Any attached diagnostic information
4. **Backtrace**: Stack trace pointing to the error location (when enabled)
5. **Source Snippets**: Actual source code around the error (when enabled)

### Example Output

```text
Error:
   0: Could not load application configuration
   1: The config file is not valid JSON
   2: Failed to read config file at config.toml

  ------------------------------------------------

Caused by:
    EOF while parsing a value at line 1 column 0

Note:
    File content was:
    ---
    this is not json
    ---

Backtrace omitted. Run with RUST_BACKTRACE=1 to display it.
Run with RUST_BACKTRACE=full to include source snippets.
```

### Controlling Verbosity

Environment variables control the level of detail in error reports:

| Environment Variable | Value | Effect |
|---------------------|-------|--------|
| `RUST_LIB_BACKTRACE` | (unset) | Minimal output, no backtrace |
| `RUST_LIB_BACKTRACE` | `1` | Include backtrace |
| `RUST_LIB_BACKTRACE` | `full` | Include backtrace with source code snippets |
| `RUST_BACKTRACE` | `1` | Also enables backtraces (fallback) |

## Configuration and Customization

### Using HookBuilder

For advanced customization, use `HookBuilder` instead of the simple `install()`:

```rust
use color_eyre::{config::HookBuilder, Result};

fn main() -> Result<()> {
    let (panic_hook, eyre_hook) = HookBuilder::default()
        .panic_section("Consider reporting this bug at: https://github.com/my/project/issues")
        .into_hooks();

    eyre_hook.install()?;
    std::panic::set_hook(Box::new(panic_hook));

    // ... application logic ...
    Ok(())
}
```

### Customization Options

`HookBuilder` allows you to customize:

- Color themes for terminal output
- Symbols used in the report (arrows, bullet points, etc.)
- Default panic section text for bug reporting
- Filtering of backtrace frames

## SpanTrace Integration

When used with the `tracing` ecosystem, `color-eyre` can capture a `SpanTrace` in addition to traditional backtraces.

### Backtrace vs SpanTrace

| Feature | Backtrace | SpanTrace |
|---------|-----------|-----------|
| **Tracks** | Function calls (stack frames) | User-defined spans (units of work) |
| **Granularity** | Every function on the stack | Only instrumented code |
| **Async Support** | Can be confusing for async code | Designed for async workflows |
| **Semantics** | Low-level, implementation details | High-level, domain-specific |

### Enabling SpanTrace

Use the `tracing-error` crate alongside `color-eyre`:

```toml
[dependencies]
color-eyre = "0.6"
tracing = "0.1"
tracing-error = "0.2"
tracing-subscriber = "0.3"
```

SpanTraces provide more semantic context, especially useful for tracing the flow through asynchronous operations.

## Comparison with Other Crates

### Overview Table

| Feature | `color-eyre` | `anyhow` | `thiserror` |
|---------|--------------|----------|-------------|
| **Use Case** | Application error handling | Application error handling | Library error definition |
| **Error Type** | Dynamic (`eyre::Report`) | Dynamic (`anyhow::Error`) | Static (custom enum/struct) |
| **Context Addition** | `.wrap_err()` | `.context()` | Defined in type |
| **Error Reporting** | Colored, multi-line, backtraces | Plain text, single-line | Depends on `Display` impl |
| **Help Text** | Built-in via `Help` trait | No | No |
| **Backtraces** | Built-in and colored | Available via feature | Not handled |
| **Error Matching** | Requires downcasting | Requires downcasting | Easy pattern matching |
| **Performance** | Slight overhead for rich reporting | Very low overhead | Zero-cost |

### color-eyre vs anyhow

Both crates serve the same purpose with nearly identical APIs. The key difference is **presentation**:

- **`anyhow`**: Lightweight, minimal dependencies, plain text output. The de-facto standard for simple application error handling.
- **`color-eyre`**: Rich terminal output with colors, symbols, backtraces, and help text. Ideal when error readability is a priority.

**Choose `anyhow` if**: You want minimal dependencies and don't need fancy output.

**Choose `color-eyre` if**: You're building a CLI or server where developers/users see error output directly.

### color-eyre vs thiserror

These crates serve different purposes and are often used together:

- **`thiserror`**: For **library authors** who need to expose a stable, well-defined error API. Users can pattern match on specific error variants.
- **`color-eyre`**: For **application authors** who consume libraries and need to handle/report errors to users.

**Common workflow**: A library uses `thiserror` to define its errors; an application uses `color-eyre` to handle and report them:

```rust
// In a library using thiserror
use thiserror::Error;

#[derive(Error, Debug)]
pub enum LibError {
    #[error("Invalid configuration: {0}")]
    BadConfig(String),
    #[error("Network failure: {source}")]
    NetworkError { #[from] source: reqwest::Error },
}

// In your application using color-eyre
use color_eyre::eyre::{Result, WrapErr};
use my_lib::do_something;

fn main() -> Result<()> {
    color_eyre::install()?;
    do_something().wrap_err("The library operation failed")?;
    Ok(())
}
```

## Best Practices

### When to Use color-eyre

1. **Building applications, not libraries**: CLIs, web servers, GUI applications
2. **Developer experience is a priority**: When you or your users will be debugging from a terminal
3. **You want to provide helpful hints**: Use `Help` trait for common configuration issues
4. **You want batteries-included**: One `install()` call gives you better error and panic reports

### When NOT to Use color-eyre

1. **In library public APIs**: Use `thiserror` instead to expose structured error types
2. **When minimal dependencies matter**: Use `anyhow` or standard library types
3. **When you need programmatic error handling**: Type-erased errors make matching difficult

### Error Handling Patterns

```rust
use color_eyre::eyre::{eyre, Result, WrapErr, Help};

// Pattern 1: Add context at each level
fn high_level() -> Result<()> {
    mid_level().wrap_err("High-level operation failed")?;
    Ok(())
}

// Pattern 2: Add help for common issues
fn load_env_config() -> Result<Config> {
    // ...
    Err(eyre!("Missing DATABASE_URL"))
        .with_help("Set DATABASE_URL in your .env file or environment")
}

// Pattern 3: Add notes with debugging info
fn parse_input(input: &str) -> Result<Data> {
    serde_json::from_str(input)
        .wrap_err("Failed to parse input")
        .with_note(|| format!("Input was: {}", input))
}
```

## Quick Reference

### Essential Imports

```rust
use color_eyre::eyre::{eyre, Result, WrapErr, Help};
```

### Common Methods

| Method | Purpose |
|--------|---------|
| `color_eyre::install()` | Initialize error/panic hooks |
| `.wrap_err("message")` | Add context to an error |
| `.wrap_err_with(\|\| format!(...))` | Add lazy context |
| `.with_help("text")` | Add help/suggestion text |
| `.with_note("text")` | Add informational note |
| `eyre!("message")` | Create a new error |
| `bail!("message")` | Return early with an error |

### Environment Variables

| Variable | Purpose |
|----------|---------|
| `RUST_LIB_BACKTRACE=1` | Enable backtraces |
| `RUST_LIB_BACKTRACE=full` | Enable backtraces with source snippets |
| `RUST_BACKTRACE=1` | Fallback backtrace control |
| `NO_COLOR=1` | Disable colored output |

## Resources

- [color-eyre on crates.io](https://crates.io/crates/color-eyre)
- [color-eyre documentation](https://docs.rs/color-eyre)
- [eyre crate (foundation)](https://crates.io/crates/eyre)
- [anyhow crate (alternative)](https://crates.io/crates/anyhow)
- [thiserror crate (for libraries)](https://crates.io/crates/thiserror)
- [tracing-error (SpanTrace integration)](https://crates.io/crates/tracing-error)
