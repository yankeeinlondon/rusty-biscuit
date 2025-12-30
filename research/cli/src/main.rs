//! Research CLI - Automated research tool for software libraries

use clap::{Parser, Subcommand};
use research_lib::research;
use std::io::{self, BufRead};
use std::path::PathBuf;
use tracing_subscriber::{filter::EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};
use tts::Tts;

#[derive(Parser)]
#[command(name = "research")]
#[command(about = "Automated research tool for software libraries", long_about = None)]
struct Cli {
    /// Increase verbosity (-v, -vv, -vvv)
    #[arg(short = 'v', action = clap::ArgAction::Count, global = true)]
    log_verbosity: u8,

    /// Output logs as JSON
    #[arg(long, global = true)]
    json: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Research a software library
    Library {
        /// The library/topic to research (use "-" to read from stdin)
        #[arg(value_name = "TOPIC")]
        topic: String,

        /// Additional questions to research in parallel
        #[arg(value_name = "QUESTIONS")]
        questions: Vec<String>,

        /// Output directory for research files [default: research/<TOPIC>]
        #[arg(short, long, value_name = "DIR")]
        output: Option<PathBuf>,
    },

    /// List all research topics
    List {
        /// Glob patterns to filter topics (e.g., "foo", "foo*", "bar")
        #[arg(value_name = "FILTER")]
        filters: Vec<String>,

        /// Filter by research type (repeatable: -t library -t software)
        #[arg(short = 't', long = "type", value_name = "TYPE")]
        types: Vec<String>,

        /// Show detailed metadata for each topic (sub-bullets with issues)
        #[arg(short = 'v', long)]
        verbose: bool,

        /// Output as JSON instead of terminal format
        #[arg(long)]
        json: bool,
    },

    /// Create symbolic links from research skills to Claude Code and OpenCode
    Link {
        /// Glob patterns to filter topics (e.g., "foo", "foo*", "bar")
        #[arg(value_name = "FILTER")]
        filters: Vec<String>,

        /// Filter by research type (repeatable: -t library -t software)
        #[arg(short = 't', long = "type", value_name = "TYPE")]
        types: Vec<String>,

        /// Output as JSON instead of terminal format
        #[arg(long)]
        json: bool,
    },
}

fn read_topic_from_stdin() -> io::Result<String> {
    let stdin = io::stdin();
    let mut line = String::new();
    stdin.lock().read_line(&mut line)?;
    Ok(line.trim().to_string())
}

/// Initialize tracing subscriber based on verbosity and output format
fn init_tracing(verbose: u8, json: bool) {
    // Determine base filter from RUST_LOG or verbosity flags
    // Default (verbose=0) shows only WARN level to reduce noise
    // Use -v flags to increase verbosity for debugging
    let base_filter = match std::env::var("RUST_LOG") {
        Ok(filter) => filter,
        Err(_) => match verbose {
            // Default: WARN only to reduce stderr noise
            0 => "warn".to_string(),
            // -v: Show INFO for research progress and tool calls
            1 => "warn,research_lib=info,shared::tools=info".to_string(),
            // -vv: Show DEBUG for research_lib and shared
            2 => "info,research_lib=debug,shared=debug".to_string(),
            // -vvv+: Show TRACE for detailed debugging
            _ => "debug,research_lib=trace,shared=trace".to_string(),
        },
    };

    let filter = EnvFilter::try_new(&base_filter).unwrap_or_else(|_| EnvFilter::new("warn"));

    if json {
        // JSON output for structured log processing
        tracing_subscriber::registry()
            .with(filter)
            .with(fmt::layer().json().with_writer(std::io::stderr))
            .init();
    } else {
        // Human-readable console output to stderr
        tracing_subscriber::registry()
            .with(filter)
            .with(
                fmt::layer()
                    .with_target(true)
                    .with_level(true)
                    .with_thread_ids(false)
                    .with_file(verbose >= 3)
                    .with_line_number(verbose >= 3)
                    .with_writer(std::io::stderr)
                    .compact(),
            )
            .init();
    }
}

#[tokio::main]
async fn main() {
    // Load environment variables from .env file
    dotenvy::dotenv().ok();

    let cli = Cli::parse();
    init_tracing(cli.log_verbosity, cli.json);

    tracing::info!("Research CLI starting");

    match cli.command {
        Commands::Library {
            topic,
            questions,
            output,
        } => {
            // Read topic from stdin if "-" is provided
            let topic = if topic == "-" {
                match read_topic_from_stdin() {
                    Ok(t) if !t.is_empty() => t,
                    Ok(_) => {
                        eprintln!("Error: No topic provided on stdin");
                        std::process::exit(1);
                    }
                    Err(e) => {
                        eprintln!("Error reading from stdin: {}", e);
                        std::process::exit(1);
                    }
                }
            } else {
                topic
            };

            match research(&topic, output, &questions).await {
                Ok(result) => {
                    println!("\n{}", "=".repeat(60));
                    if result.cancelled {
                        println!(
                            "Cancelled: {} succeeded, {} failed in {:.1}s",
                            result.succeeded, result.failed, result.total_time_secs
                        );
                    } else {
                        println!(
                            "Complete: {} succeeded, {} failed in {:.1}s",
                            result.succeeded, result.failed, result.total_time_secs
                        );
                    }
                    println!(
                        "Total tokens: {} in, {} out, {} total",
                        result.total_input_tokens, result.total_output_tokens, result.total_tokens
                    );
                    println!("Output: {:?}", result.output_dir);
                    println!("{}", "=".repeat(60));

                    // Only announce if not cancelled
                    if !result.cancelled {
                        announce_completion(&result.topic);
                    }
                }
                Err(e) => {
                    eprintln!("Research failed: {}", e);
                    std::process::exit(1);
                }
            }
        }

        Commands::List {
            filters,
            types,
            verbose,
            json,
        } => {
            match research_lib::list(filters, types, verbose, json).await {
                Ok(()) => {
                    // Success - output already written to stdout
                }
                Err(e) => {
                    eprintln!("List failed: {}", e);
                    std::process::exit(1);
                }
            }
        }

        Commands::Link {
            filters,
            types,
            json,
        } => {
            match research_lib::link(filters, types, json).await {
                Ok(_) => {
                    // Output already printed by library
                }
                Err(e) => {
                    eprintln!("Link failed: {}", e);
                    std::process::exit(1);
                }
            }
        }
    }
}

fn announce_completion(topic: &str) {
    if let Ok(mut tts) = Tts::default() {
        if let Ok(voices) = tts.voices() {
            if let Some(voice) = voices.iter().find(|v| {
                !v.id().contains("compact")
                    && !v.id().contains("eloquence")
                    && v.language().starts_with("en")
            }) {
                let _ = tts.set_voice(voice);
            }
        }

        let message = format!("Research for the {} library has completed", topic);
        if tts.speak(&message, false).is_ok() {
            std::thread::sleep(std::time::Duration::from_millis(100));
            while tts.is_speaking().unwrap_or(false) {
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        }
    }
}
