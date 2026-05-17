//! Research CLI - Automated research tool for software libraries

use clap::{Parser, Subcommand};
use research::research;
use std::io::{self, BufRead};
use std::path::PathBuf;
use tracing_subscriber::{filter::EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

/// Debug trace level for tracing output.
///
/// Use this to control the verbosity of internal debug traces.
/// The default is `warn`, which only shows warnings and errors.
/// Use `RUST_LOG` environment variable to override.
#[derive(clap::ValueEnum, Clone, Debug, Default)]
enum DebugLevel {
    /// Trace level - most verbose, shows all debug details
    Trace,
    /// Debug level - detailed debug information
    Debug,
    /// Info level - informational messages
    Info,
    /// Warn level - warnings only (default)
    #[default]
    Warn,
    /// Error level - only errors
    Error,
    /// Disable all tracing output
    Off,
}

impl std::fmt::Display for DebugLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DebugLevel::Trace => write!(f, "trace"),
            DebugLevel::Debug => write!(f, "debug"),
            DebugLevel::Info => write!(f, "info"),
            DebugLevel::Warn => write!(f, "warn"),
            DebugLevel::Error => write!(f, "error"),
            DebugLevel::Off => write!(f, "off"),
        }
    }
}

impl DebugLevel {
    fn as_str(&self) -> Option<&'static str> {
        match self {
            DebugLevel::Trace => Some("trace"),
            DebugLevel::Debug => Some("debug"),
            DebugLevel::Info => Some("info"),
            DebugLevel::Warn => Some("warn"),
            DebugLevel::Error => Some("error"),
            DebugLevel::Off => None,
        }
    }
}

#[derive(Parser)]
#[command(name = "research")]
#[command(about = "Automated research tool for software libraries", long_about = None)]
struct Cli {
    /// Increase output verbosity (-v, -vv, -vvv) for richer console output
    ///
    /// This flag controls the amount of human-readable status output shown to the user.
    /// It does NOT control debug tracing - use --debug <level> for that.
    #[arg(short = 'v', action = clap::ArgAction::Count, global = true)]
    verbose: u8,

    /// Debug trace level: trace, debug, info, warn, error
    ///
    /// Controls the verbosity of internal debug traces sent to stderr.
    /// Use `RUST_LOG` environment variable to override this setting.
    ///
    /// Examples:
    ///   --debug debug    Show debug messages
    ///   --debug trace    Show all trace messages (most verbose)
    ///   --debug off     Disable all tracing output
    #[arg(long, global = true, value_name = "LEVEL", default_value_t = DebugLevel::Warn)]
    debug: DebugLevel,

    /// Output results as JSON (for data output)
    ///
    /// This affects the format of data written to stdout.
    /// Tracing output format is controlled separately via RUST_LOG.
    #[arg(long, global = true)]
    json: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Research a software library
    #[command(alias = "lib")]
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

        /// Regenerate skill files from existing research
        ///
        /// Requires all underlying research documents (overview, similar_libraries, etc.)
        /// to exist. Removes skill/* contents and regenerates SKILL.md.
        #[arg(long)]
        skill: bool,

        /// Force recreation of all research output documents
        ///
        /// Bypasses incremental mode and regenerates all ResearchOutput documents
        /// (overview, similar_libraries, etc.) even if they already exist.
        #[arg(long)]
        force: bool,
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
        #[arg(long)]
        verbose: bool,

        /// Output as JSON instead of terminal format
        #[arg(long)]
        json: bool,

        /// Migrate all v0 metadata files to v1 schema
        #[arg(long)]
        migrate: bool,
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

    /// Show a research topic's deep dive document
    Show {
        /// The topic to show (directory name under ~/.research/library/)
        #[arg(value_name = "TOPIC")]
        topic: String,
    },

    /// Research a public API
    Api {
        /// The API name (e.g., "stripe", "github", "openai")
        #[arg(required = true, value_name = "API_NAME")]
        api_name: String,

        /// Additional research questions
        #[arg(value_name = "QUESTIONS")]
        questions: Vec<String>,

        /// Output directory [default: $RESEARCH_DIR/.research/api/<api-name>]
        #[arg(short, long, value_name = "DIR")]
        output: Option<PathBuf>,

        /// Force recreation even if research exists
        #[arg(short, long)]
        force: bool,
    },

    /// Pull a research skill from user scope to the current repository
    ///
    /// Copies the skill directory from ~/.research/library/<topic>/skill/
    /// to .claude/skills/<topic>/ in the current git repository.
    /// Also creates relative symlinks for Roo (.roo/skills/) and
    /// OpenCode (.opencode/skill/) if those frameworks are detected.
    Pull {
        /// The topic to pull from the user's research library
        #[arg(required = true, value_name = "TOPIC")]
        topic: String,

        /// Also copy underlying research documents to .claude/research/<topic>/
        #[arg(long)]
        local: bool,
    },
}

fn read_topic_from_stdin() -> io::Result<String> {
    let stdin = io::stdin();
    let mut line = String::new();
    stdin.lock().read_line(&mut line)?;
    Ok(line.trim().to_string())
}

/// Show a research topic's deep dive document in the system's default application.
///
/// Discovers topics by globbing for `{RESEARCH_DIR}/.research/library/*/deep_dive.md`.
fn show_topic(topic: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Match the pattern used in research (respects RESEARCH_DIR env var)
    let base = std::env::var("RESEARCH_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from(".")));

    let library_dir = base.join(".research").join("library");

    // Glob only immediate children, matching discover_topics() behavior
    let pattern = format!("{}/*/deep_dive.md", library_dir.display());

    for entry in glob::glob(&pattern)? {
        let path = entry?;
        // Get parent (should be the topic directory)
        if let Some(parent) = path.parent()
            && let Some(name) = parent.file_name()
            && name.to_string_lossy() == topic
        {
            open::that(&path)?;
            return Ok(());
        }
    }

    Err(format!(
        "Topic '{}' not found. Run 'research list' to see available topics.",
        topic
    )
    .into())
}

/// Initialize tracing subscriber based on debug level and output format.
///
/// Tracing output goes to stderr to keep stdout clean for data output.
///
/// Precedence for filter level:
/// 1. RUST_LOG environment variable (if set)
/// 2. --debug command line flag
/// 3. Default: warn
fn init_tracing(debug_level: DebugLevel, verbose: u8, json: bool) {
    // Precedence: RUST_LOG > --debug > default
    let filter = std::env::var("RUST_LOG")
        .ok()
        .or_else(|| debug_level.as_str().map(String::from))
        .unwrap_or_else(|| "warn".to_string());

    let filter = EnvFilter::try_new(&filter).unwrap_or_else(|_| EnvFilter::new("warn"));

    if json {
        tracing_subscriber::registry()
            .with(filter)
            .with(fmt::layer().json().with_writer(std::io::stderr))
            .init();
    } else {
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
    init_tracing(cli.debug, cli.verbose, cli.json);

    tracing::info!("Research CLI starting");

    match cli.command {
        Commands::Library {
            topic,
            questions,
            output,
            skill,
            force,
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

            match research(&topic, output, &questions, skill, force).await {
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
                        use biscuit_speaks::{TtsConfig, speak_when_able};
                        let message =
                            format!("Research for the {} library has completed", result.topic);
                        speak_when_able(&message, &TtsConfig::default()).await;
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
            migrate,
        } => {
            match research::list_with_migrate(filters, types, verbose, json, migrate).await {
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
            match research::link(filters, types, json).await {
                Ok(_) => {
                    // Output already printed by library
                }
                Err(e) => {
                    eprintln!("Link failed: {}", e);
                    std::process::exit(1);
                }
            }
        }

        Commands::Show { topic } => {
            if let Err(e) = show_topic(&topic) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }

        Commands::Api {
            api_name,
            questions,
            output,
            force,
        } => {
            match research::research_api(&api_name, output, &questions, force).await {
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
                        use biscuit_speaks::{TtsConfig, speak_when_able};
                        let message =
                            format!("Research for the {} API has completed", result.topic);
                        speak_when_able(&message, &TtsConfig::default()).await;
                    }
                }
                Err(e) => {
                    eprintln!("API research failed: {}", e);
                    std::process::exit(1);
                }
            }
        }

        Commands::Pull { topic, local } => {
            use research::pull::{PullOptions, pull_topic};

            let options = PullOptions { topic, local };

            match pull_topic(&options) {
                Ok(result) => {
                    println!("Pulled '{}' to repository", result.topic);
                    println!("  Skill: {}", result.skill_path.display());

                    if let Some(research_path) = result.research_path {
                        println!("  Research: {}", research_path.display());
                    }

                    if !result.symlinks_created.is_empty() {
                        println!("  Symlinks created:");
                        for symlink in &result.symlinks_created {
                            println!("    - {}", symlink.display());
                        }
                    }

                    if !result.warnings.is_empty() {
                        println!("\nWarnings:");
                        for warning in &result.warnings {
                            println!("  - {}", warning);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Pull failed: {}", e);
                    std::process::exit(1);
                }
            }
        }
    }
}
