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

    init_tracing(cli.debug);

    // Resolve the default `icons` command when none is given.
    let command = cli
        .command
        .unwrap_or(Commands::Icons { filter: cli.filter, from: cli.from });

    if let Err(err) = commands::run(command, cli.nerd).await {
        render_error(&err, cli.verbose);
        std::process::exit(1);
    }
}

fn init_tracing(debug: u8) {
    let explicit = std::env::var("RUST_LOG").ok();
    if debug == 0 && explicit.is_none() {
        return;
    }
    let base = explicit.unwrap_or_else(|| match debug {
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

/// Renders a human-readable, Prose-styled error to stderr.
///
/// With `--verbose`, the cause chain is deduplicated and appended.
fn render_error(err: &color_eyre::eyre::Report, verbose: u8) {
    use biscuit_terminal::components::prose::Prose;
    use biscuit_terminal::components::renderable::TerminalRenderable;
    use biscuit_terminal::terminal::Terminal;

    let term = Terminal::new();
    let mut message = format!("<red><b>Error:</b></red> {err}");

    if verbose > 0 {
        let mut seen = Vec::new();
        for cause in err.chain().skip(1) {
            let text = cause.to_string();
            if !seen.contains(&text) {
                seen.push(text.clone());
            }
        }
        if !seen.is_empty() {
            message.push_str("\n<dim>Caused by:</dim>");
            for cause in seen {
                message.push_str(&format!("\n  <dim>- {cause}</dim>"));
            }
        }
    }

    let prose = Prose::new(message);
    eprintln!("{}", prose.render(&term));
}
