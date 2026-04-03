use std::path::{Path, PathBuf};

use tracing::{Span, info_span};
use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::prelude::*;

use crate::args::{Cli, Commands, DebugLevel};

pub(crate) fn init_tracing(debug_level: Option<DebugLevel>) {
    let rust_log = std::env::var("RUST_LOG")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let span_events = if rust_log.is_some() || debug_level.is_some() {
        FmtSpan::CLOSE
    } else {
        FmtSpan::NONE
    };

    tracing_subscriber::registry()
        .with(build_env_filter(rust_log.as_deref(), debug_level))
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(std::io::stderr)
                .with_target(true)
                .with_file(true)
                .with_line_number(true)
                .with_span_events(span_events),
        )
        .init();
}

pub(crate) fn build_env_filter(
    rust_log: Option<&str>,
    debug_level: Option<DebugLevel>,
) -> EnvFilter {
    let builder = tracing_subscriber::EnvFilter::builder()
        .with_default_directive(tracing::Level::WARN.into());

    if let Some(rust_log) = rust_log {
        return builder.parse_lossy(rust_log);
    }

    if let Some(debug_level) = debug_level {
        return builder.parse_lossy(format!("claudine={}", debug_level.as_str()));
    }

    builder.from_env_lossy()
}

pub(crate) fn root_span(cli: &Cli) -> Span {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let repo_root = find_repo_root(&cwd);
    let command = cli.command.as_ref().map(command_name).unwrap_or("help");
    let subcommand = cli.command.as_ref().map(subcommand_name).unwrap_or("help");
    let cwd_display = cwd.display().to_string();
    let repo_root_display = repo_root
        .as_ref()
        .map(|path| path.display().to_string())
        .unwrap_or_default();
    let pid = std::process::id();

    match cli.command.as_ref() {
        Some(
            Commands::Claude(args)
            | Commands::Codex(args)
            | Commands::Gemini(args)
            | Commands::Kimi(args)
            | Commands::Qwen(args)
            | Commands::Opencode(args)
            | Commands::Goose(args),
        ) => info_span!(
            "cli_invocation",
            command,
            subcommand,
            plain = cli.plain,
            cwd = %cwd_display,
            repo_root = %repo_root_display,
            pid,
            provider = subcommand,
            interactive = args.interactive,
            quiet = args.quiet,
            silent = args.silent,
            repo_mode = args.repo,
            mcp_enabled = args.mcp || !args.mcp_use.is_empty(),
        ),
        _ => info_span!(
            "cli_invocation",
            command,
            subcommand,
            plain = cli.plain,
            cwd = %cwd_display,
            repo_root = %repo_root_display,
            pid,
        ),
    }
}

fn command_name(command: &Commands) -> &'static str {
    match command {
        Commands::Handle(_) => "handle",
        Commands::Completions(_) => "completions",
        Commands::Init(_) => "init",
        Commands::Sync(_) => "sync",
        Commands::Hooks(_) => "hooks",
        Commands::Actions(_) => "actions",
        Commands::Skills(_) => "skills",
        Commands::Agents(_) => "agents",
        Commands::SlashCommands(_) => "commands",
        Commands::Providers => "providers",
        Commands::Logs(_) => "logs",
        Commands::Uninstall(_) => "uninstall",
        Commands::Mcp(_) => "mcp",
        Commands::Claude(_)
        | Commands::Codex(_)
        | Commands::Gemini(_)
        | Commands::Kimi(_)
        | Commands::Qwen(_)
        | Commands::Opencode(_)
        | Commands::Goose(_) => "wrap",
        Commands::Compose(_) => "compose",
        Commands::InlineCompose(_) => "inline-compose",
    }
}

fn subcommand_name(command: &Commands) -> &'static str {
    match command {
        Commands::Claude(_) => "claude",
        Commands::Codex(_) => "codex",
        Commands::Gemini(_) => "gemini",
        Commands::Kimi(_) => "kimi",
        Commands::Qwen(_) => "qwen",
        Commands::Opencode(_) => "opencode",
        Commands::Goose(_) => "goose",
        _ => command_name(command),
    }
}

fn find_repo_root(start: &Path) -> Option<PathBuf> {
    let mut current = Some(start);
    while let Some(path) = current {
        if path.join(".git").exists() {
            return Some(path.to_path_buf());
        }
        current = path.parent();
    }
    None
}
