use std::borrow::Cow;
use std::fmt;
use std::path::{Path, PathBuf};

use chrono::Local;
use tracing::{Event, Span, Subscriber, info_span};
use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt::format::{
    FmtSpan, FormatEvent, FormatFields, Writer,
};
use tracing_subscriber::fmt::{FmtContext, FormattedFields};
use tracing_subscriber::prelude::*;
use tracing_subscriber::registry::LookupSpan;

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

    let cwd = std::env::current_dir().ok();
    let source_base_dir = cwd
        .as_deref()
        .and_then(find_repo_root)
        .or(cwd.clone());

    tracing_subscriber::registry()
        .with(build_env_filter(rust_log.as_deref(), debug_level))
        .with(
            tracing_subscriber::fmt::layer()
                .with_span_events(span_events)
                .event_format(RelativePathEventFormat::new(source_base_dir))
                .with_writer(std::io::stderr)
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
            subcommand = provider_subcommand_name(cli.command.as_ref()).unwrap_or(command),
            plain = cli.plain,
            cwd = %cwd_display,
            repo_root = %repo_root_display,
            pid,
            interactive = args.interactive,
            quiet = args.quiet,
            silent = args.silent,
            repo_mode = args.repo,
            mcp_enabled = args.mcp || !args.mcp_use.is_empty(),
        ),
        _ => info_span!(
            "cli_invocation",
            command,
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

fn provider_subcommand_name(command: Option<&Commands>) -> Option<&'static str> {
    match command? {
        Commands::Claude(_) => Some("claude"),
        Commands::Codex(_) => Some("codex"),
        Commands::Gemini(_) => Some("gemini"),
        Commands::Kimi(_) => Some("kimi"),
        Commands::Qwen(_) => Some("qwen"),
        Commands::Opencode(_) => Some("opencode"),
        Commands::Goose(_) => Some("goose"),
        _ => None,
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

fn shorten_source_path<'a>(file: &'a str, base_dir: Option<&Path>) -> Cow<'a, str> {
    let path = Path::new(file);
    if !path.is_absolute() {
        return Cow::Borrowed(file);
    }

    if let Some(base_dir) = base_dir
        && let Ok(relative) = path.strip_prefix(base_dir)
    {
        return Cow::Owned(relative.display().to_string());
    }

    Cow::Borrowed(file)
}

struct RelativePathEventFormat {
    base_dir: Option<PathBuf>,
}

impl RelativePathEventFormat {
    fn new(base_dir: Option<PathBuf>) -> Self {
        Self { base_dir }
    }
}

impl<S, N> FormatEvent<S, N> for RelativePathEventFormat
where
    S: Subscriber + for<'lookup> LookupSpan<'lookup>,
    N: for<'writer> FormatFields<'writer> + 'static,
{
    fn format_event(
        &self,
        ctx: &FmtContext<'_, S, N>,
        mut writer: Writer<'_>,
        event: &Event<'_>,
    ) -> fmt::Result {
        write_timestamp(&mut writer)?;
        write!(writer, " ")?;

        let meta = event.metadata();
        write_level(&mut writer, *meta.level())?;
        write!(writer, " ")?;

        if let Some(file) = meta.file() {
            let file = shorten_source_path(file, self.base_dir.as_deref());
            write_path(&mut writer, &file)?;
            if let Some(line) = meta.line() {
                write!(writer, ":{line}")?;
            }
            write!(writer, ":")?;
        }

        if let Some(scope) = ctx.event_scope() {
            for span in scope.from_root() {
                write!(writer, " {}", span.metadata().name())?;
                let extensions = span.extensions();
                if let Some(fields) = extensions.get::<FormattedFields<N>>()
                    && !fields.is_empty()
                {
                    write!(writer, "{{{fields}}}")?;
                }
                write!(writer, ":")?;
            }
        }

        write!(writer, " ")?;
        ctx.field_format().format_fields(writer.by_ref(), event)?;
        writeln!(writer)
    }
}

fn write_timestamp(writer: &mut Writer<'_>) -> fmt::Result {
    let value = Local::now().format("%H:%M:%S");
    if writer.has_ansi_escapes() {
        write!(writer, "\x1b[2m{value}\x1b[0m")
    } else {
        write!(writer, "{value}")
    }
}

fn write_level(writer: &mut Writer<'_>, level: tracing::Level) -> fmt::Result {
    let label = level.as_str();
    if !writer.has_ansi_escapes() {
        return write!(writer, "{label}");
    }

    let color = match level {
        tracing::Level::TRACE => "35",
        tracing::Level::DEBUG => "34",
        tracing::Level::INFO => "32",
        tracing::Level::WARN => "33",
        tracing::Level::ERROR => "31",
    };
    write!(writer, "\x1b[1;{color}m{label}\x1b[0m")
}

fn write_path(writer: &mut Writer<'_>, path: &str) -> fmt::Result {
    if writer.has_ansi_escapes() {
        write!(writer, "\x1b[2;36m{path}\x1b[0m")
    } else {
        write!(writer, "{path}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal_wrapper_args() -> crate::commands::wrap::WrapperArgs {
        crate::commands::wrap::WrapperArgs {
            help: false,
            yolo: false,
            include: Vec::new(),
            interactive: false,
            model: None,
            output: None,
            system_prompt: None,
            timeout: None,
            dry_run: false,
            quiet: false,
            silent: false,
            operation: None,
            sandbox: false,
            repo: false,
            mcp: false,
            mcp_use: Vec::new(),
            strict: false,
            passthrough: Vec::new(),
        }
    }

    #[test]
    fn provider_subcommand_only_exists_for_wrapper_commands() {
        assert_eq!(provider_subcommand_name(Some(&Commands::Providers)), None);
        assert_eq!(
            provider_subcommand_name(Some(&Commands::Codex(minimal_wrapper_args()))),
            Some("codex")
        );
    }

    #[test]
    fn shorten_source_path_strips_repo_root_prefix() {
        let shortened = shorten_source_path(
            "/repo/claudine/cli/src/telemetry.rs",
            Some(Path::new("/repo")),
        );
        assert_eq!(shortened, "claudine/cli/src/telemetry.rs");
    }

    #[test]
    fn shorten_source_path_keeps_relative_paths() {
        let shortened = shorten_source_path(
            "claudine/cli/src/telemetry.rs",
            Some(Path::new("/repo")),
        );
        assert_eq!(shortened, "claudine/cli/src/telemetry.rs");
    }

    #[test]
    fn write_level_plain_has_no_ansi_sequences() {
        let mut rendered = String::new();
        let mut writer = Writer::new(&mut rendered);
        write_level(&mut writer, tracing::Level::INFO).unwrap();
        assert_eq!(rendered, "INFO");
    }
}
