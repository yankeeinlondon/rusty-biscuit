use std::io::Read;
use std::path::PathBuf;

use biscuit_clipboard::client::{ClipperClient, ServiceStatus};
use clap::{Parser, Subcommand};
use futures_util::StreamExt;

mod autostart;

#[derive(Parser)]
#[command(name = "clip", about = "Biscuit clipboard CLI", version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Print current clipboard content
    Get {
        /// Format to retrieve (text, html, rtf, image, files)
        #[arg(long)]
        format: Option<String>,
        /// Encoding for binary payloads (e.g. base64)
        #[arg(long)]
        encoding: Option<String>,
    },
    /// Set clipboard content from argument or stdin
    Set {
        /// Text to set (reads from stdin if not provided)
        text: Option<String>,
    },
    /// Show metadata about current clipboard
    Info,
    /// Show clipboard history
    History {
        /// Output as JSON
        #[arg(long)]
        json: bool,
        /// Clear the history (sends DELETE /history)
        #[arg(long)]
        clear: bool,
    },
    /// Clear the OS clipboard (does NOT clear history; use `clip history --clear` for that)
    Clear,
    /// Watch clipboard changes
    ///
    /// By default streams events from the running `clipper` service via
    /// Server-Sent Events. Pass `--foreground` to run an in-process
    /// watcher (debugging only; refuses when the service is up).
    Watch {
        /// Run the legacy in-process watcher (debug only). Refuses when the
        /// service is already running.
        #[arg(long)]
        foreground: bool,
    },
    /// Manage the clipper background service
    Service {
        #[command(subcommand)]
        command: ServiceCommands,
    },
}

#[derive(Subcommand)]
enum ServiceCommands {
    /// Start the clipper background service
    Start,
    /// Stop the clipper background service
    Stop,
    /// Show service status
    Status,
    /// Install the per-user autostart manifest for `clipper`
    ///
    /// Writes a launchd plist on macOS, a systemd user unit on Linux, or
    /// a Startup folder shim on Windows. System-wide install is out of
    /// scope for v1.
    Install {
        /// Print the manifest body without writing to disk.
        #[arg(long)]
        dry_run: bool,
        /// Override the binary path embedded in the manifest.
        #[arg(long)]
        binary: Option<String>,
        /// Override the install root (default: user home / %APPDATA%).
        /// Honoured by the `CLIP_AUTOSTART_PREFIX` env var as well.
        #[arg(long)]
        prefix: Option<PathBuf>,
    },
    /// Remove the per-user autostart manifest written by `install`.
    Uninstall {
        /// Print what would be removed without touching disk.
        #[arg(long)]
        dry_run: bool,
        /// Override the install root (must match the value passed to
        /// `install`). Honoured by the `CLIP_AUTOSTART_PREFIX` env var
        /// as well.
        #[arg(long)]
        prefix: Option<PathBuf>,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    if let Err(e) = run(cli).await {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}

async fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    match cli.command {
        Commands::Get { format, encoding } => cmd_get(format, encoding).await,
        Commands::Set { text } => cmd_set(text).await,
        Commands::Info => cmd_info().await,
        Commands::History { json, clear } => cmd_history(json, clear).await,
        Commands::Clear => cmd_clear().await,
        Commands::Watch { foreground } => cmd_watch(foreground).await,
        Commands::Service { command } => match command {
            ServiceCommands::Start => cmd_service_start().await,
            ServiceCommands::Stop => cmd_service_stop().await,
            ServiceCommands::Status => cmd_service_status(),
            ServiceCommands::Install {
                dry_run,
                binary,
                prefix,
            } => cmd_service_install(dry_run, binary, prefix),
            ServiceCommands::Uninstall { dry_run, prefix } => {
                cmd_service_uninstall(dry_run, prefix)
            }
        },
    }
}

async fn cmd_get(
    format: Option<String>,
    encoding: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut client = ClipperClient::new()?;
    client.ensure_running().await?;
    let result = client
        .get_current_with_format(format.as_deref(), encoding.as_deref())
        .await?;
    match result {
        Some(text) => println!("{text}"),
        None => eprintln!("Clipboard is empty"),
    }
    Ok(())
}

async fn cmd_set(text: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
    let content = match text {
        Some(t) => t,
        None => {
            let mut buf = String::new();
            std::io::stdin().read_to_string(&mut buf)?;
            buf
        }
    };

    if content.is_empty() {
        eprintln!("No content provided");
        std::process::exit(1);
    }

    let mut client = ClipperClient::new()?;
    client.ensure_running().await?;
    let id = client.set_text(&content).await?;
    println!("{id}");
    Ok(())
}

async fn cmd_info() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = ClipperClient::new()?;
    client.ensure_running().await?;
    match client.get_latest().await {
        Ok(entry) => {
            println!("ID: {}", entry.id);
            println!("Type: {}", entry.content_type);
            println!("Preview: {}", entry.preview);
            println!("Size: {} bytes", entry.size_bytes);
            println!("Timestamp: {}", entry.timestamp);
        }
        Err(_) => eprintln!("No clipboard info available"),
    }
    Ok(())
}

async fn cmd_history(json: bool, clear: bool) -> Result<(), Box<dyn std::error::Error>> {
    let mut client = ClipperClient::new()?;
    client.ensure_running().await?;

    if clear {
        client.clear_history().await?;
        println!("History cleared.");
        return Ok(());
    }

    let entries = client.get_history().await?;
    if json {
        println!("{}", serde_json::to_string_pretty(&entries)?);
    } else if entries.is_empty() {
        println!("No history entries.");
    } else {
        for entry in &entries {
            let id_str = entry.id.as_str();
            let short_id = &id_str[..8.min(id_str.len())];
            println!(
                "{short_id} [{}] {} ({} bytes)",
                entry.content_type, entry.preview, entry.size_bytes
            );
        }
    }
    Ok(())
}

async fn cmd_clear() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = ClipperClient::new()?;
    client.ensure_running().await?;
    client.clear_clipboard().await?;
    println!("Clipboard cleared.");
    Ok(())
}

async fn cmd_watch(foreground: bool) -> Result<(), Box<dyn std::error::Error>> {
    if foreground {
        return cmd_watch_foreground().await;
    }
    cmd_watch_via_service().await
}

/// Default `clip watch` mode — auto-starts the service if needed and
/// streams clipboard changes from `GET /events`.
async fn cmd_watch_via_service() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = ClipperClient::new()?;
    client.ensure_running().await?;
    let mut stream = Box::pin(client.events_stream().await?);

    eprintln!("Watching clipboard via clipper service... Press Ctrl+C to stop.");

    while let Some(item) = stream.next().await {
        match item {
            Ok(summary) => {
                println!(
                    "[{}] {} ({} bytes)",
                    summary.content_type, summary.preview, summary.size_bytes
                );
            }
            Err(e) => {
                eprintln!("event stream error: {e}");
                break;
            }
        }
    }

    Ok(())
}

/// Legacy in-process watcher — only available with `--foreground` and
/// only when no `clipper` service is running. Documented for debugging.
async fn cmd_watch_foreground() -> Result<(), Box<dyn std::error::Error>> {
    use biscuit_clipboard::backend::SystemClipboard;
    use biscuit_clipboard::config;
    use biscuit_clipboard::watcher::{WatcherEvent, spawn_system_watcher};
    use std::sync::Arc;

    if let Some(pid) = config::read_pid_file()
        && config::is_pid_alive(pid)
    {
        return Err(format!(
            "clipper service is running (PID {pid}); refusing to start an \
             in-process watcher. Run `clip service stop` first or omit \
             `--foreground` to stream from the service."
        )
        .into());
    }

    let backend = Arc::new(SystemClipboard::new()?);
    let (_handle, mut rx) = spawn_system_watcher(backend)?;

    eprintln!("Watching clipboard (foreground; debug)... Press Ctrl+C to stop.");

    while let Some(event) = rx.recv().await {
        match event {
            WatcherEvent::Changed { formats, concealed } => {
                if concealed {
                    continue;
                }
                for fmt in &formats {
                    println!("{}", fmt.preview());
                }
            }
            WatcherEvent::Died { reason } => {
                eprintln!("watcher died: {reason}");
                break;
            }
        }
    }

    Ok(())
}

async fn cmd_service_start() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = ClipperClient::new()?;
    client.ensure_running().await?;
    println!("Service started.");
    Ok(())
}

async fn cmd_service_stop() -> Result<(), Box<dyn std::error::Error>> {
    let client = ClipperClient::new()?;
    client.stop_service().await?;
    println!("Service stopped.");
    Ok(())
}

fn cmd_service_install(
    dry_run: bool,
    binary: Option<String>,
    prefix: Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    let opts = autostart::Options {
        dry_run,
        binary_path_override: binary,
        prefix_override: prefix,
    };
    let report = autostart::install(opts)?;

    if dry_run {
        println!("# manifest path: {}", report.manifest_path.display());
        println!("{}", report.manifest_body);
        if let Some(step) = report.next_step {
            eprintln!("Next: {step}");
        }
        return Ok(());
    }

    match report.outcome {
        autostart::Outcome::Wrote => {
            println!("Installed autostart manifest at {}", report.manifest_path.display());
        }
        autostart::Outcome::AlreadyInstalled => {
            println!(
                "Autostart manifest already present at {}",
                report.manifest_path.display()
            );
        }
        autostart::Outcome::DryRun => unreachable!("dry_run handled above"),
    }
    if let Some(step) = report.next_step {
        eprintln!("Next: {step}");
    }
    eprintln!(
        "Note: clipper will start on next login. Run `clip service start` to start now."
    );
    Ok(())
}

fn cmd_service_uninstall(
    dry_run: bool,
    prefix: Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    let opts = autostart::Options {
        dry_run,
        binary_path_override: None,
        prefix_override: prefix,
    };
    let report = autostart::uninstall(opts)?;

    match report.outcome {
        autostart::Outcome::DryRun => {
            println!("# would remove: {}", report.manifest_path.display());
        }
        autostart::Outcome::Wrote => {
            println!(
                "Removed autostart manifest at {}",
                report.manifest_path.display()
            );
        }
        autostart::Outcome::AlreadyInstalled => {
            println!(
                "No autostart manifest present at {}",
                report.manifest_path.display()
            );
        }
    }
    Ok(())
}

fn cmd_service_status() -> Result<(), Box<dyn std::error::Error>> {
    let client = ClipperClient::new()?;
    match client.service_status() {
        ServiceStatus::Running { pid, port } => {
            println!("clipper is running (PID {pid}, port {port})");
        }
        ServiceStatus::Stopped => {
            println!("clipper is not running");
        }
    }
    Ok(())
}
