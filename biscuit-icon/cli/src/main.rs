mod args;
mod commands;
mod sets_table;

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

    // Bare invocation (no subcommand, no filter) shows help and exits. Without
    // this short-circuit, the default `show` branch below would dump every
    // curated domain icon, which is rarely what a first-time user wants.
    if cli.command.is_none() && cli.filter.is_none() {
        let _ = Cli::command().print_help();
        return;
    }

    // Resolve the default `show` command when a filter was given without a
    // subcommand (e.g. `icon mdi:home`). The show format flags live on the
    // top-level Cli via flatten, so they are picked up here directly.
    let command = cli.command.unwrap_or_else(|| {
        let filter = cli.filter.unwrap_or_default();
        Commands::Show {
            ids: vec![filter],
            show: cli.show,
        }
    });

    if let Err(err) = commands::run(command, cli.nerd, cli.verbose).await {
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
    use biscuit_terminal::terminal::Terminal;

    eprintln!("{}", format_error(err, verbose, &Terminal::new()));
}

/// Formats `err` for `term`; each cause sits on its own line below the
/// `Caused by:` heading.
fn format_error(
    err: &color_eyre::eyre::Report,
    verbose: u8,
    term: &biscuit_terminal::terminal::Terminal,
) -> String {
    use biscuit_terminal::components::prose::{LineBreaks, Prose};
    use biscuit_terminal::components::renderable::TerminalRenderable;

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

    // The message is assembled line by line, so each `\n` is a line break.
    Prose::new(message)
        .with_line_breaks(LineBreaks::Hard)
        .render(term)
}

#[cfg(test)]
mod tests {
    use super::*;
    use biscuit_terminal::discovery::detection::ColorDepth;
    use biscuit_terminal::prelude::strip_escape_codes;
    use biscuit_terminal::terminal::Terminal;
    use color_eyre::eyre::eyre;

    #[test]
    fn verbose_error_keeps_each_cause_on_its_own_line() {
        // Arrange
        let err = eyre!("cache file is locked").wrap_err("could not open icon cache");
        let term = Terminal::builder()
            .is_tty(false)
            .color_depth(ColorDepth::None)
            .osc_link_support(false)
            .build();

        // Act
        let rendered = strip_escape_codes(format_error(&err, 1, &term));

        // Assert
        assert_eq!(
            rendered.lines().collect::<Vec<_>>(),
            vec![
                "Error: could not open icon cache",
                "Caused by:",
                "  - cache file is locked",
            ],
        );
    }
}
