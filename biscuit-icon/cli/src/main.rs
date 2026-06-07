mod args;
mod commands;

use clap::{CommandFactory, Parser};
use clap_complete::CompleteEnv;
use clap_complete::aot::generate;
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

use args::{Cli, Commands};

#[tokio::main]
async fn main() {
    color_eyre::install().ok();

    // Dynamic-completion entrypoint: when the shell invokes us with the
    // completion env var set, this emits candidates (driven by the per-arg
    // `ArgValueCompleter`s) and exits before normal parsing.
    CompleteEnv::with_factory(Cli::command).complete();

    let cli = Cli::parse();

    if let Some(Commands::Completions { shell }) = cli.command {
        let mut cmd = Cli::command();
        generate(shell, &mut cmd, "icon", &mut std::io::stdout());
        return;
    }

    init_tracing(cli.verbose);

    // Resolve the default `icons` command when none is given.
    let command = cli.command.unwrap_or(Commands::Icons { filter: cli.filter, from: None });

    if let Err(err) = commands::run(command).await {
        eprintln!("\x1b[31m\x1b[1mError:\x1b[0m {err}");
        std::process::exit(1);
    }
}

fn init_tracing(verbose: u8) {
    let explicit = std::env::var("RUST_LOG").ok();
    if verbose == 0 && explicit.is_none() {
        return;
    }
    let base = explicit.unwrap_or_else(|| match verbose {
        1 => "warn,biscuit_icon=info,icon=info".into(),
        2 => "info,biscuit_icon=debug,icon=debug".into(),
        _ => "debug,biscuit_icon=trace,icon=trace".into(),
    });
    let filter = EnvFilter::try_new(&base).unwrap_or_else(|_| EnvFilter::new("warn"));
    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().with_writer(std::io::stderr).compact())
        .init();
}
