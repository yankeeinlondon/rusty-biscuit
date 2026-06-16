//! Library surface for the `sniff` CLI.
//!
//! The binary entrypoint (`src/main.rs`) is a thin shell over this crate so
//! that test-only helper binaries (e.g. `src/bin/render_cicd_fixture.rs`) and
//! integration tests can drive the real rendering code through the same
//! module tree the production binary uses.

pub mod args;
pub mod commands;
pub mod install;
pub mod install_plan_cmd;
pub mod install_ui;
pub mod output;
pub mod perf;

use tracing_subscriber::{filter::EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

/// Initialize tracing subscriber for raw developer-facing diagnostics.
///
/// `--verbose` is reserved for styled user-facing output and never drives
/// tracing. Raw tracing output is opt-in via `--debug` or `RUST_LOG`.
///
/// Debug levels:
/// - 0 (default): No subscriber (zero overhead)
/// - 1 (--debug): INFO for sniff crates
/// - 2 (--debug --debug): DEBUG for sniff crates
/// - 3+ (--debug x3): TRACE for sniff crates with file/line numbers
///
/// Setting `RUST_LOG` overrides all debug levels.
pub fn init_tracing(debug: u8) {
    let explicit_rust_log = std::env::var("RUST_LOG").ok();
    if debug == 0 && explicit_rust_log.is_none() {
        return;
    }

    let base_filter = explicit_rust_log.unwrap_or_else(|| match debug {
        1 => "warn,sniff=info,sniff_cli=info".into(),
        2 => "info,sniff=debug,sniff_cli=debug".into(),
        _ => "debug,sniff=trace,sniff_cli=trace".into(),
    });

    let filter = EnvFilter::try_new(&base_filter).unwrap_or_else(|_| EnvFilter::new("warn"));

    tracing_subscriber::registry()
        .with(filter)
        .with(
            fmt::layer()
                .with_target(true)
                .with_level(true)
                .with_thread_ids(false)
                .with_file(debug >= 3)
                .with_line_number(debug >= 3)
                .with_writer(std::io::stderr)
                .compact(),
        )
        .init();
}
