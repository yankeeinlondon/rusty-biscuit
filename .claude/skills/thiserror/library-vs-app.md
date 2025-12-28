# Library vs Application Error Handling

Guidelines for choosing between thiserror, anyhow, and snafu based on your use case.

## The Golden Rule

> **Use `thiserror` for libraries, `anyhow` for applications**

This simple guideline works for most Rust projects.

## Decision Matrix

| Scenario | Recommended | Reason |
|----------|-------------|--------|
| Library with public API | thiserror | Callers need to match on specific errors |
| CLI application | anyhow | Errors are displayed, not matched |
| Web service (business logic) | thiserror | Internal errors may need handling |
| Web service (handlers) | anyhow | Top-level error formatting |
| Complex system with error context | snafu | Structured error stacks |
| Rapid prototyping | anyhow | Minimal boilerplate |

## Comparison Table

| Feature | thiserror | anyhow | snafu |
|---------|-----------|--------|-------|
| **Error Type** | Your enum/struct | `anyhow::Error` (trait object) | Your enum/struct |
| **Pattern Matching** | Yes | No (downcast required) | Yes |
| **Context Addition** | Via variant fields | `.context()` method | Structured attributes |
| **Performance** | Zero-cost | Minimal overhead | Zero-cost |
| **Compile-time Safety** | Strong | Weak | Strong |
| **Learning Curve** | Medium | Low | High |

## thiserror: Library Pattern

```rust
// Library crate: my_parser
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("Unexpected token '{token}' at line {line}")]
    UnexpectedToken { token: String, line: usize },

    #[error("Missing required field: {0}")]
    MissingField(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

// Callers can match on specific cases
pub fn parse(input: &str) -> Result<Ast, ParseError> {
    // ...
}
```

## anyhow: Application Pattern

```rust
// Application crate
use anyhow::{Context, Result};

fn main() -> Result<()> {
    let config = load_config()
        .context("Failed to load configuration")?;

    let data = fetch_data(&config.url)
        .context("Failed to fetch remote data")?;

    process(data)
        .context("Processing failed")?;

    Ok(())
}
```

## Hybrid Pattern

Many projects combine both approaches:

```rust
// Internal library module using thiserror
mod database {
    use thiserror::Error;

    #[derive(Error, Debug)]
    pub enum DbError {
        #[error("Connection failed: {0}")]
        Connection(#[from] sqlx::Error),

        #[error("Query timeout after {seconds}s")]
        Timeout { seconds: u64 },
    }
}

// Application layer using anyhow
use anyhow::{Context, Result};

fn handle_request(id: u64) -> Result<Response> {
    let user = database::get_user(id)
        .context("Failed to fetch user")?;

    let data = process_user(&user)
        .context("Failed to process user data")?;

    Ok(Response::new(data))
}
```

## When to Use snafu

Consider snafu when you need:

- Extremely detailed error context at every level
- "Error stacks" without performance overhead of backtraces
- More explicit control over context generation

```rust
// snafu approach (for comparison)
use snafu::{Snafu, ResultExt};

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Failed to read config from {path}"))]
    ReadConfig {
        path: String,
        source: std::io::Error,
    },
}

fn load() -> Result<Config, Error> {
    let content = std::fs::read_to_string(&path)
        .context(ReadConfigSnafu { path: &path })?;
    // ...
}
```

## Checklist: Which to Choose?

Ask these questions:

1. **Are you building a library with a public API?**
   - Yes -> thiserror

2. **Do callers need to programmatically handle specific error cases?**
   - Yes -> thiserror
   - No, just display/log -> anyhow

3. **Are you prototyping or building a quick tool?**
   - Yes -> anyhow

4. **Do you need rich, structured error context at every layer?**
   - Yes -> snafu (or carefully designed thiserror enums)

5. **Is this a web service?**
   - Domain layer -> thiserror
   - Handler layer -> anyhow with context

## Related

- [Error Definition Patterns](./error-patterns.md) - How to define thiserror types
- [Error Chaining](./error-chaining.md) - Connecting errors together
