# Installation and Setup

Basic setup and configuration for color-eyre in Rust applications.

## Basic Installation

Add to `Cargo.toml`:

```toml
[dependencies]
color-eyre = "0.6"
```

## Minimal Setup

```rust
use color_eyre::eyre::Result;

fn main() -> Result<()> {
    color_eyre::install()?;  // Must be first line

    // Your application logic
    run()?;

    Ok(())
}
```

**Why first line?** The `install()` function sets up global panic and error hooks. Any errors before this call won't have the enhanced formatting.

## Custom Configuration with HookBuilder

For advanced customization, use `HookBuilder`:

```rust
use color_eyre::{config::HookBuilder, Result};

fn main() -> Result<()> {
    let (panic_hook, eyre_hook) = HookBuilder::default()
        .panic_section("Consider reporting this bug at: https://github.com/my/project/issues")
        .display_env_section(false)  // Hide environment info
        .into_hooks();

    eyre_hook.install()?;
    std::panic::set_hook(Box::new(panic_hook));

    // Application logic
    Ok(())
}
```

## HookBuilder Options

| Method | Purpose |
|--------|---------|
| `.panic_section(text)` | Add custom text to panic reports |
| `.display_env_section(bool)` | Show/hide environment section |
| `.display_location_section(bool)` | Show/hide location info |
| `.add_frame_filter(fn)` | Filter backtrace frames |
| `.theme(Theme)` | Customize colors |

## SpanTrace Integration

For async applications using `tracing`:

```toml
[dependencies]
color-eyre = "0.6"
tracing = "0.1"
tracing-error = "0.2"
tracing-subscriber = "0.3"
```

```rust
use color_eyre::eyre::Result;
use tracing_error::ErrorLayer;
use tracing_subscriber::prelude::*;

fn main() -> Result<()> {
    // Set up tracing with error layer
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(ErrorLayer::default())
        .init();

    color_eyre::install()?;

    // Now errors will capture SpanTraces
    Ok(())
}
```

**SpanTrace vs Backtrace:**
- Backtrace: Stack frames (function calls)
- SpanTrace: Logical spans (units of work) - more semantic, less noisy for async code

## What install() Does

1. **Error Reporting Hook**: Customizes `eyre::Report` output formatting
2. **Panic Hook**: Replaces default panic handler with colorful, detailed reports
3. **Backtrace Capture**: Enables automatic backtrace collection

## Related

- [Context and Help](./context-and-help.md) - Adding context to errors
- [Comparison Guide](./comparison.md) - When to use color-eyre
