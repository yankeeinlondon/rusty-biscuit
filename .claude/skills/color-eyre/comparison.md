# Comparison Guide

When to use color-eyre vs alternatives for Rust error handling.

## Quick Decision Tree

```
Are you writing a library with a public API?
  YES -> Use thiserror
  NO  -> Continue...

Do you need rich, colored error output?
  YES -> Use color-eyre
  NO  -> Use anyhow

Do you need help text / suggestions in errors?
  YES -> Use color-eyre
  NO  -> anyhow is simpler
```

## Comparison Table

| Feature | color-eyre | anyhow | thiserror |
|---------|-----------|--------|-----------|
| **Use Case** | Applications (CLI, servers) | Applications (simple) | Libraries |
| **Error Type** | Dynamic (`eyre::Report`) | Dynamic (`anyhow::Error`) | Static (custom enum/struct) |
| **Context Chaining** | `.wrap_err()` | `.context()` | N/A (define in type) |
| **Help Text** | Yes (`with_help()`) | No | No |
| **Colored Output** | Yes | No | No |
| **Backtraces** | Automatic, colored | Available | Not handled |
| **Source Snippets** | Yes (with `RUST_LIB_BACKTRACE=full`) | No | No |
| **Panic Hook** | Yes | No | No |
| **Pattern Matching** | Requires downcasting | Requires downcasting | Native (it's your type) |
| **Performance** | Slight overhead | Minimal overhead | Zero-cost |

## color-eyre vs anyhow

Most direct comparison - both are for applications.

**Similarities:**
- Dynamic error types (type-erased)
- Context chaining via traits
- `Result<T>` type alias
- Any error type can be converted

**Key Differences:**

| Aspect | color-eyre | anyhow |
|--------|-----------|--------|
| Output | Multi-line, colored, structured | Plain text |
| Help text | Built-in | Not available |
| Panic handling | Integrated | Separate |
| Dependencies | More (color libs) | Minimal |

**Choose anyhow when:**
- Minimal dependencies matter
- Errors are logged, not displayed to users
- You don't need colored output
- Simple error propagation is sufficient

**Choose color-eyre when:**
- Building CLIs or tools developers use directly
- Error readability is a priority
- You want to guide users with help text
- You want consistent panic and error formatting

## color-eyre vs thiserror

Different paradigms - often used together.

**thiserror** is for:
- Library authors defining public error APIs
- Structured errors users can match against
- Programmatic error handling

**color-eyre** is for:
- Application authors consuming libraries
- Terminal/UI error presentation
- Developer debugging experience

### Common Pattern: Use Both

```rust
// In library crate (my-lib)
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
    #[error("Query timeout after {0}ms")]
    Timeout(u64),
}

// In application crate (my-app)
use color_eyre::eyre::{Result, WrapErr};
use my_lib::DatabaseError;

fn main() -> Result<()> {
    color_eyre::install()?;

    fetch_data()
        .wrap_err("Failed to fetch application data")
        .with_help(|| "Check database connectivity")?;

    Ok(())
}

fn fetch_data() -> Result<Data> {
    // DatabaseError auto-converts to eyre::Report via ?
    let conn = my_lib::connect()?;
    Ok(conn.query()?)
}
```

## When NOT to Use color-eyre

1. **Library public APIs** - Hides specific error types, prevents matching
2. **Performance-critical hot paths** - Has formatting overhead
3. **Non-terminal environments** - ANSI colors may not render
4. **Minimal dependency requirements** - Pulls in color crates

## Migration from anyhow

Mostly drop-in replacement:

```rust
// Before (anyhow)
use anyhow::{Context, Result};

fn main() -> Result<()> {
    do_thing().context("failed")?;
    Ok(())
}

// After (color-eyre)
use color_eyre::eyre::{WrapErr, Result};

fn main() -> Result<()> {
    color_eyre::install()?;  // Add this line
    do_thing().wrap_err("failed")?;  // context -> wrap_err
    Ok(())
}
```

**Note:** `Context` trait in color-eyre is an alias for `WrapErr`. Both work.

## Related

- [Installation and Setup](./setup.md) - Getting started
- [Context and Help](./context-and-help.md) - Using error context features
