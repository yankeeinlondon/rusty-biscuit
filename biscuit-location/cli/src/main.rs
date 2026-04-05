//! CLI for location services: GPS, IP lookup, reverse geocoding, and distance.
//!
//! ## Usage
//!
//! ```bash
//! where gps                                   # Get location from GPS
//! where ip 8.8.8.8                            # Look up IP location
//! where reverse 34.0522 -118.2437             # Reverse geocode coordinates
//! where distance 34.05,-118.24 40.71,-74.01   # Distance between points
//! ```
//!
//! ## Output Modes
//!
//! - Default: terminal-styled text (stdout)
//! - `--plain`: plain text, no ANSI escape codes
//! - `--json`: machine-readable JSON
//!
//! ## Diagnostics
//!
//! Verbosity on stderr is controlled by `-v`/`-vv`/`-vvv` or the `RUST_LOG`
//! environment variable. STDOUT always stays pure data (or empty with
//! `--quiet`).

mod args;
mod commands;
mod output;

use std::io::{self, Write};

use clap::{CommandFactory, Parser};
use clap_complete::{generate, Shell};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use args::{Cli, Commands};
use output::OutputMode;

#[tokio::main]
async fn main() {
    // Errors from color_eyre::install are ignored: they only happen when a
    // global handler is already installed, which does not affect us.
    color_eyre::install().ok();

    let cli = Cli::parse();

    // Resolve output mode up front so errors render consistently.
    let mode = OutputMode::resolve(cli.json, cli.plain);

    // Shell completions short-circuit everything else.
    if let Commands::Completions { shell } = cli.command {
        print_completions(shell);
        return;
    }

    init_tracing(cli.verbose);

    // Race the command against Ctrl+C so GPS fixes and reverse-geocoding
    // requests can be cancelled without leaving partial output on stdout.
    // 130 is the conventional exit code for SIGINT-terminated programs.
    let result = tokio::select! {
        biased;
        r = commands::run(cli, mode) => r,
        _ = tokio::signal::ctrl_c() => {
            let rendered = output::render_error("interrupted", mode);
            let mut stderr = io::stderr().lock();
            let _ = writeln!(stderr, "{rendered}");
            std::process::exit(130);
        }
    };

    if let Err(err) = result {
        // Format the full cause chain, deduped, rather than collapsing to the
        // top-level message like the previous implementation did.
        let message = format_error_chain(&err);
        let rendered = output::render_error(&message, mode);
        let mut stderr = io::stderr().lock();
        let _ = writeln!(stderr, "{rendered}");
        std::process::exit(1);
    }
}

/// Initialize the tracing subscriber if the user asked for diagnostics.
///
/// ## Environment Variables
///
/// ### Priority Order
///
/// `RUST_LOG` (if set) beats any `-v` flag. When `RUST_LOG` is unset, the
/// verbosity count selects the filter level.
///
/// ### Fallback Behavior
///
/// With `verbose == 0` and `RUST_LOG` unset, no subscriber is installed —
/// this keeps zero-flag invocations silent on stderr.
fn init_tracing(verbose: u8) {
    let explicit_rust_log = std::env::var("RUST_LOG").ok();
    if verbose == 0 && explicit_rust_log.is_none() {
        return;
    }

    // The binary is named `where` (a Rust keyword), so the compiled crate
    // name — and therefore every tracing target — carries an `r#` prefix.
    let base_filter = explicit_rust_log.unwrap_or_else(|| match verbose {
        1 => "warn,biscuit_location=info,r#where=info".into(),
        2 => "info,biscuit_location=debug,r#where=debug".into(),
        _ => "debug,biscuit_location=trace,r#where=trace".into(),
    });

    let filter = EnvFilter::try_new(&base_filter).unwrap_or_else(|_| EnvFilter::new("warn"));

    tracing_subscriber::registry()
        .with(filter)
        .with(
            fmt::layer()
                .with_target(true)
                .with_level(true)
                .with_file(verbose >= 3)
                .with_line_number(verbose >= 3)
                .with_writer(std::io::stderr)
                .compact(),
        )
        .init();
}

/// Walk the eyre report's cause chain, deduplicating consecutive duplicates,
/// and join into a single `: `-separated string for display.
fn format_error_chain(err: &color_eyre::Report) -> String {
    let mut parts: Vec<String> = Vec::new();
    for cause in err.chain() {
        let text = cause.to_string();
        if parts.last() == Some(&text) {
            continue;
        }
        // Skip suffixes that are already included in the prior message.
        if parts.last().is_some_and(|prev| prev.ends_with(&text)) {
            continue;
        }
        parts.push(text);
    }
    parts.join(": ")
}

fn print_completions(shell: Shell) {
    let mut cmd = Cli::command();
    let name = cmd.get_name().to_string();
    generate(shell, &mut cmd, name, &mut io::stdout());
}
