# Indicatif Advanced Patterns

## Multi-File Download Manager

Complete example for downloading multiple files with a main progress bar and per-file bars:

```rust
use futures_util::StreamExt;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use reqwest::Client;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;

pub struct DownloadManager {
    client: Client,
    multi_progress: Arc<MultiProgress>,
    main_style: ProgressStyle,
    file_style: ProgressStyle,
}

impl DownloadManager {
    pub fn new() -> Self {
        let main_style = ProgressStyle::with_template(
            "{prefix:.bold} [{bar:40.green/dim}] {pos}/{len} files"
        ).unwrap().progress_chars("=>-");

        let file_style = ProgressStyle::with_template(
            "  {spinner:.cyan} {msg:30!} [{bar:25.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec})"
        ).unwrap().progress_chars("=>-");

        Self {
            client: Client::new(),
            multi_progress: Arc::new(MultiProgress::new()),
            main_style,
            file_style,
        }
    }

    pub async fn download_all(&self, downloads: Vec<Download>) -> Result<(), DownloadError> {
        let total = downloads.len() as u64;
        let main_pb = self.multi_progress.add(ProgressBar::new(total));
        main_pb.set_style(self.main_style.clone());
        main_pb.set_prefix("Downloading");

        let handles: Vec<_> = downloads.into_iter().map(|dl| {
            let client = self.client.clone();
            let mp = Arc::clone(&self.multi_progress);
            let style = self.file_style.clone();
            let main_pb = main_pb.clone();

            tokio::spawn(async move {
                let file_pb = mp.insert_after(&main_pb, ProgressBar::new(0));
                file_pb.set_style(style);
                file_pb.set_message(dl.filename.clone());

                match download_single(&client, &dl, &file_pb).await {
                    Ok(_) => {
                        file_pb.finish_with_message(format!("{} done", dl.filename));
                        main_pb.inc(1);
                        Ok(())
                    }
                    Err(e) => {
                        file_pb.abandon_with_message(format!("{} FAILED", dl.filename));
                        Err(e)
                    }
                }
            })
        }).collect();

        let results = futures::future::join_all(handles).await;
        main_pb.finish_with_message("Complete");

        // Check for any failures
        for result in results {
            result??;
        }
        Ok(())
    }
}

async fn download_single(
    client: &Client,
    download: &Download,
    pb: &ProgressBar,
) -> Result<(), DownloadError> {
    let res = client.get(&download.url).send().await?;

    if let Some(len) = res.content_length() {
        pb.set_length(len);
    }

    let mut file = tokio::fs::File::create(&download.path).await?;
    let mut stream = res.bytes_stream();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        file.write_all(&chunk).await?;
        pb.inc(chunk.len() as u64);
    }

    Ok(())
}
```

## Resumable Download with Range Requests

```rust
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::{header, Client};
use tokio::io::{AsyncSeekExt, AsyncWriteExt};

pub async fn download_resumable(
    client: &Client,
    url: &str,
    path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Check existing file size for resume
    let existing_size = tokio::fs::metadata(path)
        .await
        .map(|m| m.len())
        .unwrap_or(0);

    // Make HEAD request to get total size
    let head_res = client.head(url).send().await?;
    let total_size = head_res
        .headers()
        .get(header::CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);

    // Check if already complete
    if existing_size >= total_size && total_size > 0 {
        println!("File already downloaded");
        return Ok(());
    }

    // Check if server supports range requests
    let accepts_ranges = head_res
        .headers()
        .get(header::ACCEPT_RANGES)
        .map(|v| v.to_str().unwrap_or("") == "bytes")
        .unwrap_or(false);

    let pb = ProgressBar::new(total_size);
    pb.set_style(ProgressStyle::with_template(
        "{msg}\n{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})"
    )?.progress_chars("#>-"));

    let (mut file, start_pos) = if accepts_ranges && existing_size > 0 {
        pb.set_message(format!("Resuming {} from {}", url, existing_size));
        pb.set_position(existing_size);

        let mut file = tokio::fs::OpenOptions::new()
            .append(true)
            .open(path)
            .await?;
        file.seek(std::io::SeekFrom::End(0)).await?;
        (file, existing_size)
    } else {
        pb.set_message(format!("Downloading {}", url));
        (tokio::fs::File::create(path).await?, 0)
    };

    // Build request with range header if resuming
    let mut req = client.get(url);
    if start_pos > 0 {
        req = req.header(header::RANGE, format!("bytes={}-", start_pos));
    }

    let res = req.send().await?;
    let mut stream = res.bytes_stream();

    use futures_util::StreamExt;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        file.write_all(&chunk).await?;
        pb.inc(chunk.len() as u64);
    }

    pb.finish_with_message("Download complete");
    Ok(())
}
```

## Spinner Styles Collection

Common spinner patterns for different operations:

```rust
use indicatif::ProgressStyle;

/// Braille dots spinner (default)
pub fn spinner_braille() -> ProgressStyle {
    ProgressStyle::with_template("{spinner} {msg}")
        .unwrap()
        .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ ")
}

/// Classic ASCII spinner
pub fn spinner_ascii() -> ProgressStyle {
    ProgressStyle::with_template("{spinner} {msg}")
        .unwrap()
        .tick_chars("|/-\\ ")
}

/// Dots growing
pub fn spinner_dots() -> ProgressStyle {
    ProgressStyle::with_template("{spinner} {msg}")
        .unwrap()
        .tick_chars("⣾⣽⣻⢿⡿⣟⣯⣷ ")
}

/// Arrow spinner
pub fn spinner_arrows() -> ProgressStyle {
    ProgressStyle::with_template("{spinner} {msg}")
        .unwrap()
        .tick_chars("←↖↑↗→↘↓↙ ")
}

/// Box drawing spinner
pub fn spinner_box() -> ProgressStyle {
    ProgressStyle::with_template("{spinner} {msg}")
        .unwrap()
        .tick_chars("┤┘┴└├┌┬┐ ")
}

/// Bouncing ball
pub fn spinner_bounce() -> ProgressStyle {
    ProgressStyle::with_template("{spinner} {msg}")
        .unwrap()
        .tick_chars("⠁⠂⠄⠂ ")
}

/// Moon phases
pub fn spinner_moon() -> ProgressStyle {
    ProgressStyle::with_template("{spinner} {msg}")
        .unwrap()
        .tick_chars("🌑🌒🌓🌔🌕🌖🌗🌘 ")
}

/// Clock
pub fn spinner_clock() -> ProgressStyle {
    ProgressStyle::with_template("{spinner} {msg}")
        .unwrap()
        .tick_chars("🕐🕑🕒🕓🕔🕕🕖🕗🕘🕙🕚🕛 ")
}
```

## Progress Bar for Unknown Total (Streaming)

When total size is unknown (chunked transfer encoding):

```rust
use indicatif::{ProgressBar, ProgressStyle};

/// Progress bar that works without knowing total size
pub fn streaming_progress() -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::with_template(
            "{spinner:.green} {msg} [{elapsed_precise}] {bytes} ({bytes_per_sec})"
        )
        .unwrap()
        .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ ")
    );
    pb
}

// Usage:
// let pb = streaming_progress();
// pb.set_message("Downloading...");
// pb.enable_steady_tick(Duration::from_millis(100));
// while let Some(chunk) = stream.next().await {
//     pb.inc(chunk.len() as u64);
// }
// pb.finish_with_message("Complete");
```

## Testing with In-Memory Terminal

```rust
#[cfg(test)]
mod tests {
    use indicatif::{InMemoryTerm, ProgressBar, ProgressDrawTarget, ProgressStyle};

    #[test]
    fn test_progress_output() {
        let term = InMemoryTerm::new(80, 10);
        let pb = ProgressBar::with_draw_target(
            Some(100),
            ProgressDrawTarget::term_like(Box::new(term.clone()))
        );

        pb.set_style(
            ProgressStyle::with_template("[{bar:40}] {pos}/{len}")
                .unwrap()
        );

        pb.set_position(50);

        let contents = term.contents();
        assert!(contents.contains("50/100"));
    }
}
```

Requires the `in_memory` feature flag:
```toml
[dev-dependencies]
indicatif = { version = "0.17", features = ["in_memory"] }
```

## Integration with Ctrl+C Handling

```rust
use indicatif::{MultiProgress, ProgressBar};
use std::sync::Arc;
use tokio::signal;

pub async fn download_with_cancellation(
    mp: Arc<MultiProgress>,
    pb: ProgressBar,
) -> Result<(), Box<dyn std::error::Error>> {
    let pb_cancel = pb.clone();

    tokio::select! {
        result = async {
            // Your download logic here
            for i in 0..100 {
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                pb.inc(1);
            }
            pb.finish_with_message("Complete");
            Ok::<_, Box<dyn std::error::Error>>(())
        } => result,

        _ = signal::ctrl_c() => {
            pb_cancel.abandon_with_message("Cancelled");
            mp.clear()?;
            std::process::exit(130); // Standard Ctrl+C exit code
        }
    }
}
```

## Model-Citizen Specific: GGUF Download Progress

Tailored for model-citizen's download command:

```rust
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

/// Download style for model-citizen
pub fn model_download_style() -> ProgressStyle {
    ProgressStyle::with_template(
        "{prefix:.bold.cyan} {msg}\n\
         {spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] \
         {bytes}/{total_bytes} ({bytes_per_sec}, eta: {eta})"
    )
    .unwrap()
    .progress_chars("=>-")
}

/// Variant selection spinner
pub fn variant_scan_style() -> ProgressStyle {
    ProgressStyle::with_template(
        "{prefix:.bold.dim} {spinner} {msg}"
    )
    .unwrap()
    .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ ")
}

/// Multi-download progress manager
pub fn multi_download_style() -> ProgressStyle {
    ProgressStyle::with_template(
        "{prefix:.bold.green} [{bar:40.green/dim}] {pos}/{len} variants"
    )
    .unwrap()
    .progress_chars("=>-")
}

/// Per-variant download in multi-download
pub fn variant_download_style() -> ProgressStyle {
    ProgressStyle::with_template(
        "  {spinner:.cyan} {msg:35!} [{bar:20.cyan/blue}] {bytes}/{total_bytes}"
    )
    .unwrap()
    .progress_chars("=>-")
}
```
