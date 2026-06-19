---
name: indicatif
description: Expert knowledge for the Rust indicatif crate — progress bars, spinners, multi-progress, download tracking, and async/Tokio integration. Use when adding progress indicators to a CLI, tracking long-running or concurrent tasks, or troubleshooting redraw and throughput issues.
---

# Indicatif

> Terminal progress bars, spinners, and multi-progress management for Rust CLIs.

- **Crate**: [indicatif](https://crates.io/crates/indicatif) (v0.17+)
- **Docs**: [docs.rs/indicatif](https://docs.rs/indicatif)
- **GitHub**: [console-rs/indicatif](https://github.com/console-rs/indicatif)
- **Dependency**: [console](https://crates.io/crates/console) (terminal abstraction)

## Quick Reference

| Use Case | Type | Key Methods |
|----------|------|-------------|
| Known length | `ProgressBar::new(len)` | `set_position()`, `inc()`, `finish()` |
| Unknown length | `ProgressBar::new_spinner()` | `tick()`, `enable_steady_tick()` |
| Parallel tasks | `MultiProgress::new()` | `add()`, `insert_after()`, `println()` |
| Download bytes | `ProgressBar::new(total_bytes)` | `set_position()` with `{bytes}` template |
| Iterator wrap | `.progress()` trait | Automatic position tracking |

## Installation

```toml
[dependencies]
indicatif = "0.17"
# Optional features:
# indicatif = { version = "0.17", features = ["tokio", "rayon", "futures"] }
```

**Feature Flags**:
- `tokio` - Async runtime support
- `rayon` - Parallel iterator support
- `futures` - Futures-core integration
- `unicode-width` - Unicode character width (default)
- `in_memory` - VT100 terminal emulation for testing

## Core Patterns

### 1. Download Progress Bar with Bytes

```rust
use indicatif::{ProgressBar, ProgressStyle};

let total_bytes = response.content_length().unwrap_or(0);
let pb = ProgressBar::new(total_bytes);

pb.set_style(ProgressStyle::with_template(
    "{msg}\n{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})"
)?.progress_chars("#>-"));

pb.set_message(format!("Downloading {}", filename));

// Update in download loop
while let Some(chunk) = stream.next().await {
    file.write_all(&chunk)?;
    pb.inc(chunk.len() as u64);
}

pb.finish_with_message("Download complete");
```

### 2. Multi-Progress for Parallel Downloads

```rust
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::sync::Arc;

let mp = Arc::new(MultiProgress::new());
let style = ProgressStyle::with_template(
    "[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}"
)?.progress_chars("##-");

// Spawn parallel download tasks
let handles: Vec<_> = urls.iter().map(|url| {
    let mp = Arc::clone(&mp);
    let pb = mp.add(ProgressBar::new(0)); // Length set after HEAD request
    pb.set_style(style.clone());

    tokio::spawn(async move {
        // Get content length
        pb.set_length(content_length);
        pb.set_message(filename.clone());

        // Download with progress
        while let Some(chunk) = stream.next().await {
            pb.inc(chunk.len() as u64);
        }
        pb.finish_with_message("done");
    })
}).collect();

futures::future::join_all(handles).await;
mp.clear()?;
```

### 3. Spinner for Indeterminate Operations

```rust
use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;

let spinner = ProgressBar::new_spinner();
spinner.set_style(
    ProgressStyle::with_template("{prefix:.bold.dim} {spinner} {wide_msg}")?
        .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ ")
);
spinner.set_prefix("[1/3]");
spinner.set_message("Connecting to server...");

// Auto-tick every 100ms (works in async contexts)
spinner.enable_steady_tick(Duration::from_millis(100));

// Do work...
spinner.finish_with_message("Connected!");
```

### 4. Async Download with Reqwest

```rust
use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::Client;
use std::cmp::min;
use tokio::io::AsyncWriteExt;

pub async fn download_file(client: &Client, url: &str, path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let res = client.get(url).send().await?;
    let total_size = res.content_length().unwrap_or(0);

    let pb = ProgressBar::new(total_size);
    pb.set_style(ProgressStyle::with_template(
        "{msg}\n{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})"
    )?.progress_chars("#>-"));
    pb.set_message(format!("Downloading {}", url));

    let mut file = tokio::fs::File::create(path).await?;
    let mut downloaded: u64 = 0;
    let mut stream = res.bytes_stream();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        file.write_all(&chunk).await?;
        downloaded = min(downloaded + chunk.len() as u64, total_size);
        pb.set_position(downloaded);
    }

    pb.finish_with_message(format!("Downloaded to {}", path));
    Ok(())
}
```

## Template Placeholders

### Position & Length
| Placeholder | Description |
|-------------|-------------|
| `{pos}` | Current position (numeric) |
| `{len}` | Total length |
| `{human_pos}` | Human-readable position (e.g., "1.5M") |
| `{human_len}` | Human-readable length |
| `{percent}` | Percentage (integer) |
| `{percent_precise}` | Percentage (3 decimals) |

### Bytes (for downloads)
| Placeholder | Description |
|-------------|-------------|
| `{bytes}` | Current bytes (human-readable) |
| `{total_bytes}` | Total bytes (human-readable) |
| `{bytes_per_sec}` | Transfer speed |
| `{binary_bytes}` | Binary format (KiB, MiB) |
| `{decimal_bytes}` | Decimal format (KB, MB) |

### Time
| Placeholder | Description |
|-------------|-------------|
| `{elapsed}` | Elapsed time (compact) |
| `{elapsed_precise}` | Elapsed time (HH:MM:SS) |
| `{eta}` | Estimated time remaining |
| `{eta_precise}` | ETA (HH:MM:SS) |
| `{duration}` | Total duration (after finish) |

### Visual Elements
| Placeholder | Description |
|-------------|-------------|
| `{bar}` | Fixed-width progress bar |
| `{wide_bar}` | Expanding progress bar |
| `{spinner}` | Animated spinner character |
| `{msg}` | Message text |
| `{wide_msg}` | Expanding message |
| `{prefix}` | Prefix text |

### Formatting Syntax

```
{placeholder:alignment width! .style/alt_style}
```

- **Alignment**: `<` (left), `^` (center), `>` (right)
- **Width**: Number of characters
- **`!`**: Enable truncation
- **Style**: Color/attribute (e.g., `.cyan`, `.bold.dim`)
- **Alt style**: For incomplete bar portion

Examples:
```rust
"{bar:40.cyan/blue}"      // 40-char bar, cyan filled, blue empty
"{pos:>7}"                // Right-aligned, 7 chars wide
"{msg:!.bold}"            // Truncatable, bold
"{spinner:.green}"        // Green spinner
```

## Terminal Detection & Non-TTY Handling

Indicatif uses the `console` crate which provides automatic terminal detection:

```rust
use indicatif::ProgressBar;

// Progress bars auto-hide when piped to files
let pb = ProgressBar::new(100);
// If stdout is not a TTY, the bar is completely hidden

// Force hidden for non-interactive contexts
let pb = ProgressBar::hidden();

// Check if we should show progress
use std::io::IsTerminal;
if std::io::stderr().is_terminal() {
    pb.set_draw_target(indicatif::ProgressDrawTarget::stderr());
}
```

## Best Practices

### Thread Safety
Progress bars are `Send + Sync` - safely share across threads:
```rust
let pb = Arc::new(ProgressBar::new(100));
let pb_clone = Arc::clone(&pb);
tokio::spawn(async move { pb_clone.inc(1); });
```

### Refresh Rate
Default: 20 FPS (50ms). Increase for high-frequency updates:
```rust
pb.set_draw_target(ProgressDrawTarget::stderr_with_hz(60));
```

### Logging Integration
Use `indicatif-log-bridge` to prevent log messages from breaking progress bars:
```rust
use indicatif_log_bridge::LogWrapper;
let logger = /* your logger */;
LogWrapper::new(multi_progress, logger).try_init()?;
```

### Tracing Integration
Use `tracing-indicatif` for span-based progress:
```rust
use tracing_indicatif::IndicatifLayer;
let subscriber = tracing_subscriber::registry()
    .with(IndicatifLayer::new());
```

## Common Download Template

Recommended style for model-citizen download command:

```rust
const DOWNLOAD_TEMPLATE: &str =
    "{msg}\n\
     {spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] \
     {bytes}/{total_bytes} ({bytes_per_sec}, eta: {eta})";

const PROGRESS_CHARS: &str = "=>-";  // Or "##-" for classic

fn download_style() -> ProgressStyle {
    ProgressStyle::with_template(DOWNLOAD_TEMPLATE)
        .unwrap()
        .progress_chars(PROGRESS_CHARS)
}
```

## See Also

- [reqwest skill](../reqwest/SKILL.md) - HTTP client for streaming downloads
- [tokio skill](../tokio/SKILL.md) - Async runtime
- [clap skill](../clap/SKILL.md) - CLI argument parsing
