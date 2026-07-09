//! Terminal information utility CLI.
//!
//! Displays terminal metadata and capabilities including:
//! - Terminal application detection
//! - Color depth and mode
//! - Feature support (italics, images, underlines, OSC links)
//! - Multiplexing status
//! - OS and distribution information

use biscuit_terminal::{
    discovery::{clipboard, eval, fonts, mode_2027, osc_queries},
    utils::escape_codes,
};

pub mod args;
pub mod commands;
pub mod output;
pub mod types;

use args::*;
use commands::{shared::terminal_for_render, CliContext, Run};
use output::*;

use clap::{CommandFactory, Parser};
use clap_complete::Shell;
use clap_complete::engine::{CompletionCandidate, PathCompleter};

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    // Handle dynamic completions (COMPLETE env var)
    clap_complete::CompleteEnv::with_factory(Args::command).complete();

    // Setup logging if RUST_LOG is set
    if std::env::var("RUST_LOG").is_ok() {
        tracing_subscriber::fmt()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .init();
    }

    let args = Args::parse();

    // Handle --completions flag (generates static completion scripts)
    if let Some(ref shell_arg) = args.completions {
        return handle_completions(shell_arg);
    }

    let ctx = CliContext {
        json: args.json,
        plain: args.plain,
    };

    // Handle subcommands
    if let Some(cmd) = args.command {
        tracing::debug!(command = ?std::mem::discriminant(&cmd), "Dispatching subcommand");
        return cmd.run(&ctx);
    }

    // No subcommand: handle positional content or print default metadata
    let content = if args.content.is_empty() {
        None
    } else {
        Some(args.content.join(" "))
    };

    if let Some(content) = content.as_deref() {
        let analysis = analyze_content(content);
        if args.json {
            println!("{}", serde_json::to_string_pretty(&analysis)?);
        } else {
            let term = terminal_for_render(args.plain);
            print_content_analysis(&analysis, &term);
        }
        return Ok(());
    }

    // --silent implies --quiet
    let quiet = args.quiet || args.silent;

    let metadata = collect_metadata();
    if args.json {
        if !args.silent {
            println!("{}", serde_json::to_string_pretty(&metadata)?);
        }
    } else if !quiet {
        let term = terminal_for_render(args.plain);
        print_pretty(&metadata, args.verbose, &term);
    }

    Ok(())
}

/// Handles the --completions flag by generating shell completion scripts.
fn handle_completions(shell_type: &ShellType) -> color_eyre::Result<()> {
    let shell = match shell_type {
        ShellType::Bash => Shell::Bash,
        ShellType::Elvish => Shell::Elvish,
        ShellType::Fish => Shell::Fish,
        ShellType::Powershell => Shell::PowerShell,
        ShellType::Zsh => Shell::Zsh,
    };

    print_completions(shell);
    Ok(())
}

/// Prints shell completions to stdout.
fn print_completions(shell: Shell) {
    let mut cmd = Args::command();
    clap_complete::generate(shell, &mut cmd, "bt", &mut std::io::stdout());
}

/// Completes font family names from system fonts available to resvg.
pub fn font_completer(current: &std::ffi::OsStr) -> Vec<CompletionCandidate> {
    use std::sync::OnceLock;

    static FONTS: OnceLock<Vec<String>> = OnceLock::new();
    let fonts =
        FONTS.get_or_init(biscuit_terminal::components::graph_expression::available_font_families);

    let prefix = current.to_str().unwrap_or("");
    let prefix_lower = prefix.to_lowercase();

    fonts
        .iter()
        .filter(|name| name.to_lowercase().starts_with(&prefix_lower))
        .map(|name| CompletionCandidate::new(name.as_str()))
        .collect()
}

/// Completes files with extensions: png, jpg, jpeg, gif (case-insensitive).
/// Also completes directories to allow navigation.
pub fn image_completer() -> PathCompleter {
    PathCompleter::any().filter(|path| {
        if path.is_dir() {
            return true;
        }

        path.extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| {
                let ext_lower = ext.to_lowercase();
                matches!(ext_lower.as_str(), "png" | "jpg" | "jpeg" | "gif")
            })
    })
}

impl Run for Command {
    fn run(self, ctx: &CliContext) -> color_eyre::Result<()> {
        match self {
            Command::About(args) => args.run(ctx),
            Command::Image(args) => args.run(ctx),
            Command::Flowchart(args) => args.run(ctx),
            Command::Quadrant(args) => args.run(ctx),
            Command::PieChart(args) => args.run(ctx),
            Command::GitGraph(args) => args.run(ctx),
            Command::BarChart(args) => args.run(ctx),
            Command::LineChart(args) => args.run(ctx),
            Command::Timeline(args) => args.run(ctx),
            Command::StateDiagram(args) => args.run(ctx),
            Command::GraphExpression(args) => args.run(ctx),
            Command::Erd(args) => args.run(ctx),
            Command::Prose(args) => args.run(ctx),
            Command::Quote(args) => args.run(ctx),
            Command::List(args) => args.run(ctx),
            Command::PadLeft(args) => args.run(ctx),
            Command::PadRight(args) => args.run(ctx),
            Command::Columns(args) => args.run(ctx),
            Command::Dir(args) => args.run(ctx),
            Command::Block(args) => args.run(ctx),
            Command::Progress(args) => args.run(ctx),
            Command::Table(args) => args.run(ctx),
            Command::Compose(args) => args.run(ctx),
            Command::Section(args) => args.run(ctx),
            Command::StatusBlock(args) => args.run(ctx),
            Command::TextBlock(args) => args.run(ctx),
            Command::Todo(args) => args.run(ctx),
        }
    }
}
