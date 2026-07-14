---
name: color-eyre
description: Expert knowledge for using color-eyre in Rust applications for rich error handling with colored backtraces, contextual error chains, help text, and beautiful terminal output. Use for CLI tools, servers, and application-level error reporting with eyre::Report.
hash: f0034a2060dfca2e
---

# color-eyre

A Rust crate for application-level error handling that provides colorful, contextual, and beautifully formatted error reports. Fork of `eyre` (itself a fork of `anyhow`) focused on developer-friendly error presentation.

## Core Principles

- Install `color_eyre::install()` as the **first line** in `main()`
- Use `eyre::Result<T>` as return type for application functions
- Add context with `.wrap_err()` to build error chains (breadcrumb trail)
- Attach help text with `.with_help()` for user-actionable suggestions
- Use `eyre!()` macro to create ad-hoc errors
- Set `RUST_LIB_BACKTRACE=1` for backtraces, `=full` for source snippets
- Pair with `thiserror` in libraries, consume with `color_eyre` in applications
- Never use `color_eyre` in library public APIs (use `thiserror` instead)

## Quick Reference

```rust
use color_eyre::eyre::{Result, WrapErr, eyre};

fn main() -> Result<()> {
    color_eyre::install()?;  // First line!
    run_app()
}

fn run_app() -> Result<()> {
    let config = load_config()
        .wrap_err("Failed to load application config")?;
    Ok(())
}

fn load_config() -> Result<Config> {
    std::fs::read_to_string("config.toml")
        .wrap_err("Could not read config file")?
        .parse()
        .wrap_err("Invalid config format")
}
```

## Topics

### Setup and Configuration

- [Installation and Setup](./setup.md) - Basic setup, custom hooks, panic sections

### Error Context

- [Context and Help](./context-and-help.md) - WrapErr, Help trait, SectionExt, notes

### Decision Making

- [Comparison Guide](./comparison.md) - When to use color-eyre vs anyhow vs thiserror

## Common Patterns

### Adding Context to Errors

```rust
fs::read_to_string(path)
    .wrap_err_with(|| format!("Failed to read {}", path.display()))?
```

### Providing Help Text

```rust
env::var("API_KEY")
    .map_err(|e| eyre::Report::new(e))
    .with_help(|| "Set API_KEY environment variable: export API_KEY=xxx")
```

### Creating Ad-hoc Errors

```rust
if value.is_empty() {
    return Err(eyre!("Value cannot be empty"))
        .with_help("Provide a non-empty value in config");
}
```

### Adding Notes for Debugging

```rust
parse_json(&content)
    .wrap_err("Invalid JSON")
    .with_note(|| format!("Content was:\n{}", content))
```

## Environment Variables

| Variable | Values | Effect |
|----------|--------|--------|
| `RUST_LIB_BACKTRACE` | `0` | No backtrace |
| `RUST_LIB_BACKTRACE` | `1` | Short backtrace |
| `RUST_LIB_BACKTRACE` | `full` | Full backtrace with source snippets |
| `RUST_BACKTRACE` | `1` | Fallback if `RUST_LIB_BACKTRACE` not set |

## Resources

- [GitHub - color-eyre](https://github.com/eyre-rs/color-eyre)
- [docs.rs - color-eyre](https://docs.rs/color-eyre)
- [GitHub - eyre](https://github.com/eyre-rs/eyre)
