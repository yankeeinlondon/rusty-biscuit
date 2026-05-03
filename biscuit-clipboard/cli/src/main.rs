use std::io::Read;

use biscuit_clipboard::client::{ClipperClient, ServiceStatus};
use clap::{Parser, Subcommand};

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
    },
    /// Clear clipboard history
    Clear,
    /// Watch clipboard changes in foreground (for debugging)
    Watch,
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
        Commands::Get { format: _ } => cmd_get().await,
        Commands::Set { text } => cmd_set(text).await,
        Commands::Info => cmd_info().await,
        Commands::History { json } => cmd_history(json).await,
        Commands::Clear => cmd_clear().await,
        Commands::Watch => cmd_watch().await,
        Commands::Service { command } => match command {
            ServiceCommands::Start => cmd_service_start().await,
            ServiceCommands::Stop => cmd_service_stop().await,
            ServiceCommands::Status => cmd_service_status(),
        },
    }
}

async fn cmd_get() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = ClipperClient::new();
    client.ensure_running().await?;
    match client.get_current().await? {
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

    let mut client = ClipperClient::new();
    client.ensure_running().await?;
    let id = client.set_text(&content).await?;
    println!("{id}");
    Ok(())
}

async fn cmd_info() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = ClipperClient::new();
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

async fn cmd_history(json: bool) -> Result<(), Box<dyn std::error::Error>> {
    let mut client = ClipperClient::new();
    client.ensure_running().await?;
    let entries = client.get_history().await?;
    if json {
        println!("{}", serde_json::to_string_pretty(&entries)?);
    } else if entries.is_empty() {
        println!("No history entries.");
    } else {
        for entry in &entries {
            let short_id = &entry.id[..8.min(entry.id.len())];
            println!(
                "{short_id} [{}] {} ({} bytes)",
                entry.content_type, entry.preview, entry.size_bytes
            );
        }
    }
    Ok(())
}

async fn cmd_clear() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = ClipperClient::new();
    client.ensure_running().await?;
    client.clear_history().await?;
    println!("History cleared.");
    Ok(())
}

async fn cmd_watch() -> Result<(), Box<dyn std::error::Error>> {
    use biscuit_clipboard::backend::SystemClipboard;
    use biscuit_clipboard::spawn_watcher;

    let backend = SystemClipboard::new()?;
    let (_handle, mut rx) = spawn_watcher(backend)?;

    println!("Watching clipboard... Press Ctrl+C to stop.");

    while let Some(event) = rx.recv().await {
        for fmt in &event.formats {
            println!("{}", fmt.preview());
        }
    }

    Ok(())
}

async fn cmd_service_start() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = ClipperClient::new();
    client.ensure_running().await?;
    println!("Service started.");
    Ok(())
}

async fn cmd_service_stop() -> Result<(), Box<dyn std::error::Error>> {
    let client = ClipperClient::new();
    client.stop_service().await?;
    println!("Service stopped.");
    Ok(())
}

fn cmd_service_status() -> Result<(), Box<dyn std::error::Error>> {
    let client = ClipperClient::new();
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
