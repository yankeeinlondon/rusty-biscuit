mod args;
mod commands;
mod install;
mod install_plan_cmd;
mod output;

use tracing_subscriber::{filter::EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

/// Initialize tracing subscriber based on verbosity level.
///
/// Verbosity levels:
/// - 0 (default): No subscriber (zero overhead)
/// - 1 (-v): INFO for sniff crates
/// - 2 (-vv): DEBUG for sniff crates
/// - 3+ (-vvv): TRACE for sniff crates with file/line numbers
///
/// Setting `RUST_LOG` overrides all verbosity levels.
pub(crate) fn init_tracing(verbose: u8) {
    let explicit_rust_log = std::env::var("RUST_LOG").ok();
    if verbose == 0 && explicit_rust_log.is_none() {
        return;
    }

    let base_filter = explicit_rust_log.unwrap_or_else(|| match verbose {
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
                .with_file(verbose >= 3)
                .with_line_number(verbose >= 3)
                .with_writer(std::io::stderr)
                .compact(),
        )
        .init();
}

#[tokio::main]
async fn main() {
    if let Err(e) = commands::run().await {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
