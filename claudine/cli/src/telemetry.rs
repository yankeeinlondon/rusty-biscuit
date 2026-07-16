use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};

use chrono::Local;
use tracing::field::{Field, Visit};
use tracing::{Event, Span, Subscriber, info_span};
use tracing_subscriber::EnvFilter;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::fmt::format::{FmtSpan, FormatEvent, FormatFields, Writer};
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
    let source_base_dir = cwd.as_deref().and_then(find_repo_root).or(cwd.clone());

    tracing_subscriber::registry()
        .with(build_env_filter(rust_log.as_deref(), debug_level))
        .with(
            tracing_subscriber::fmt::layer()
                .with_span_events(span_events)
                .event_format(RelativePathEventFormat::new(source_base_dir))
                .with_writer(std::io::stderr),
        )
        .init();
}

pub(crate) fn build_env_filter(
    rust_log: Option<&str>,
    debug_level: Option<DebugLevel>,
) -> EnvFilter {
    // Default to OFF so no tracing events leak to stderr unless the user
    // explicitly opts in via RUST_LOG or --debug. See feedback_verbose_vs_debug.
    let builder =
        tracing_subscriber::EnvFilter::builder().with_default_directive(LevelFilter::OFF.into());

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
    let repo_root_display = repo_root
        .as_ref()
        .map(|path| path.display().to_string())
        .unwrap_or_default();
    let effective_cwd = if matches!(
        cli.command.as_ref(),
        Some(
            Commands::Claude(_)
                | Commands::Codex(_)
                | Commands::Gemini(_)
                | Commands::Kimi(_)
                | Commands::Qwen(_)
                | Commands::Opencode(_)
                | Commands::Goose(_)
                | Commands::Compose(_)
                | Commands::InlineCompose(_)
        )
    ) {
        repo_root.as_ref().unwrap_or(&cwd)
    } else {
        &cwd
    };
    let cwd_display = effective_cwd.display().to_string();
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
        Commands::Complete(_) => "__complete",
        Commands::Config(_) => "config",
        Commands::Sync(_) => "sync",
        Commands::Hooks(_) => "hooks",
        Commands::Actions(_) => "actions",
        Commands::Skills(_) => "skills",
        Commands::Agents(_) => "agents",
        Commands::SlashCommands(_) => "commands",
        Commands::Providers(_) => "providers",
        Commands::Signals(_) => "signals",
        Commands::Logs(_) => "logs",
        Commands::Uninstall(_) => "uninstall",
        Commands::Mcp(_) => "mcp",
        Commands::Claude(_)
        | Commands::Codex(_)
        | Commands::Gemini(_)
        | Commands::Kimi(_)
        | Commands::Qwen(_)
        | Commands::Opencode(_)
        | Commands::Goose(_)
        | Commands::Kilo(_)
        | Commands::Pi(_)
        | Commands::Antigravity(_) => "wrap",
        Commands::Compose(_) => "compose",
        Commands::InlineCompose(_) => "inline-compose",
        Commands::Sequence(_) => "sequence",
        Commands::Dashboard(_) => "dashboard",
        Commands::Context(_) => "context",
        Commands::Errors(_) => "errors",
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
        Commands::Kilo(_) => Some("kilo"),
        Commands::Pi(_) => Some("pi"),
        Commands::Antigravity(_) => Some("antigravity"),
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
        let event_fields = collect_event_fields(event);
        let scope_fields = collect_scope_fields(ctx);
        let span_names = collect_span_names(ctx);
        let message = event_fields
            .get("message")
            .map(String::as_str)
            .filter(|message| !message.is_empty())
            .unwrap_or("event");
        let detail_event = scope_fields
            .get("event")
            .or_else(|| event_fields.get("event"))
            .map(String::as_str)
            .filter(|value| !value.is_empty());
        let detail_tool = scope_fields
            .get("tool_name")
            .or_else(|| event_fields.get("tool_name"))
            .map(String::as_str)
            .filter(|value| !value.is_empty());
        let detail_tool_detail = event_fields
            .get("tool_detail")
            .or_else(|| scope_fields.get("tool_detail"))
            .map(String::as_str)
            .filter(|value| !value.is_empty());
        let busy = event_fields
            .get("time.busy")
            .or_else(|| event_fields.get("busy"))
            .map(String::as_str)
            .filter(|value| !value.is_empty());
        let idle = event_fields
            .get("time.idle")
            .or_else(|| event_fields.get("idle"))
            .map(String::as_str)
            .filter(|value| !value.is_empty());

        write_timestamp(&mut writer)?;
        write!(writer, " ")?;

        let meta = event.metadata();
        write_level(&mut writer, *meta.level())?;
        write!(writer, " ")?;
        if !span_names.is_empty() {
            write!(writer, "[{}] ", span_names.join(">"))?;
        }
        write_message(&mut writer, message)?;

        if detail_event.is_some()
            || detail_tool.is_some()
            || detail_tool_detail.is_some()
            || busy.is_some()
            || idle.is_some()
        {
            write!(writer, " ")?;
            write_details(
                &mut writer,
                detail_event,
                detail_tool,
                detail_tool_detail,
                busy,
                idle,
            )?;
        }

        if let Some(file) = meta.file() {
            let file = shorten_source_path(file, self.base_dir.as_deref());
            write!(writer, " ")?;
            write_location(&mut writer, &file, meta.line())?;
        }

        writeln!(writer)
    }
}

fn collect_event_fields(event: &Event<'_>) -> HashMap<String, String> {
    let mut visitor = EventFieldVisitor::default();
    event.record(&mut visitor);
    visitor.fields
}

fn collect_scope_fields<S, N>(ctx: &FmtContext<'_, S, N>) -> HashMap<String, String>
where
    S: Subscriber + for<'lookup> LookupSpan<'lookup>,
    N: for<'writer> FormatFields<'writer> + 'static,
{
    let mut fields = HashMap::new();

    if let Some(scope) = ctx.event_scope() {
        for span in scope.from_root() {
            let extensions = span.extensions();
            if let Some(formatted) = extensions.get::<FormattedFields<N>>() {
                for (key, value) in parse_formatted_fields(formatted) {
                    fields.insert(key, value);
                }
            }
        }
    }

    fields
}

fn collect_span_names<S, N>(ctx: &FmtContext<'_, S, N>) -> Vec<String>
where
    S: Subscriber + for<'lookup> LookupSpan<'lookup>,
    N: for<'writer> FormatFields<'writer> + 'static,
{
    let mut names = Vec::new();

    if let Some(scope) = ctx.event_scope() {
        for span in scope.from_root() {
            names.push(span.name().to_string());
        }
    }

    names
}

fn parse_formatted_fields(input: &str) -> Vec<(String, String)> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut escaped = false;

    for ch in input.chars() {
        if escaped {
            current.push(ch);
            escaped = false;
            continue;
        }

        match ch {
            '\\' if in_quotes => {
                current.push(ch);
                escaped = true;
            }
            '"' => {
                in_quotes = !in_quotes;
                current.push(ch);
            }
            ',' if !in_quotes => {
                if let Some(field) = parse_formatted_field(&current) {
                    fields.push(field);
                }
                current.clear();
            }
            _ => current.push(ch),
        }
    }

    if let Some(field) = parse_formatted_field(&current) {
        fields.push(field);
    }

    fields
}

fn parse_formatted_field(input: &str) -> Option<(String, String)> {
    let trimmed = input.trim();
    let (key, value) = trimmed.split_once('=')?;
    let key = key.trim();
    let value = value.trim();
    if key.is_empty() || value.is_empty() {
        return None;
    }

    let value = value
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .unwrap_or(value)
        .replace("\\\"", "\"");

    Some((key.to_string(), value))
}

#[derive(Default)]
struct EventFieldVisitor {
    fields: HashMap<String, String>,
}

impl Visit for EventFieldVisitor {
    fn record_str(&mut self, field: &Field, value: &str) {
        self.fields
            .insert(field.name().to_string(), value.to_string());
    }

    fn record_bool(&mut self, field: &Field, value: bool) {
        self.fields
            .insert(field.name().to_string(), value.to_string());
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        self.fields
            .insert(field.name().to_string(), value.to_string());
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        self.fields
            .insert(field.name().to_string(), value.to_string());
    }

    fn record_f64(&mut self, field: &Field, value: f64) {
        self.fields
            .insert(field.name().to_string(), value.to_string());
    }

    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        self.fields
            .insert(field.name().to_string(), format!("{value:?}"));
    }
}

fn write_timestamp(writer: &mut Writer<'_>) -> fmt::Result {
    let value = Local::now().format("%H:%M:%S%.3f");
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

fn write_message(writer: &mut Writer<'_>, message: &str) -> fmt::Result {
    write!(writer, "{message}")
}

fn write_details(
    writer: &mut Writer<'_>,
    event: Option<&str>,
    tool_name: Option<&str>,
    tool_detail: Option<&str>,
    busy: Option<&str>,
    idle: Option<&str>,
) -> fmt::Result {
    write!(writer, "(")?;

    let mut wrote_any = false;
    if event.is_some() || tool_name.is_some() || tool_detail.is_some() {
        if writer.has_ansi_escapes() {
            write!(writer, "\x1b[2m")?;
        }
        if let Some(event) = event {
            write!(writer, "{event}")?;
            wrote_any = true;
        }
        if let Some(tool_name) = tool_name {
            if wrote_any {
                write!(writer, ", ")?;
            }
            write!(writer, "{tool_name}")?;
            wrote_any = true;
        }
        if let Some(tool_detail) = tool_detail {
            if wrote_any {
                write!(writer, " ")?;
            }
            write!(writer, "{tool_detail}")?;
            wrote_any = true;
        }
        if writer.has_ansi_escapes() {
            write!(writer, "\x1b[0m")?;
        }
    }

    if let Some(busy) = busy {
        if wrote_any {
            write!(writer, ", ")?;
        }
        write!(writer, "busy: {busy}")?;
        wrote_any = true;
    }

    if let Some(idle) = idle {
        if wrote_any {
            write!(writer, ", ")?;
        }
        write!(writer, "idle: {idle}")?;
    }

    write!(writer, ")")
}

fn write_location(writer: &mut Writer<'_>, path: &str, line: Option<u32>) -> fmt::Result {
    if writer.has_ansi_escapes() {
        write!(writer, "\x1b[2m\x1b[3min \x1b[0m\x1b[34m{path}\x1b[0m")?;
        if let Some(line) = line {
            write!(writer, "\x1b[2m:{line}\x1b[0m")?;
        }
        Ok(())
    } else {
        write!(writer, "in {path}")?;
        if let Some(line) = line {
            write!(writer, ":{line}")?;
        }
        Ok(())
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
            edit: false,
            model: None,
            output: None,
            append_system_prompt: None,
            replace_system_prompt: None,
            timeout: None,
            step_timeout: None,
            stall_timeout: None,
            dry_run: false,
            quiet: false,
            silent: false,
            operation: None,
            sandbox: false,
            repo: false,
            mcp: false,
            mcp_use: Vec::new(),
            strict: false,
            perf: false,
            passthrough: Vec::new(),
        }
    }

    #[test]
    fn provider_subcommand_only_exists_for_wrapper_commands() {
        assert_eq!(
            provider_subcommand_name(Some(&Commands::Providers(
                crate::commands::providers::ProvidersArgs {
                    describe: false,
                    format: crate::commands::providers::ProvidersFormat::Text,
                    command: None,
                }
            ))),
            None
        );
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
        let shortened =
            shorten_source_path("claudine/cli/src/telemetry.rs", Some(Path::new("/repo")));
        assert_eq!(shortened, "claudine/cli/src/telemetry.rs");
    }

    #[test]
    fn write_level_plain_has_no_ansi_sequences() {
        let mut rendered = String::new();
        let mut writer = Writer::new(&mut rendered);
        write_level(&mut writer, tracing::Level::INFO).unwrap();
        assert_eq!(rendered, "INFO");
    }

    #[test]
    fn write_timestamp_plain_includes_millis() {
        let mut rendered = String::new();
        let mut writer = Writer::new(&mut rendered);
        write_timestamp(&mut writer).unwrap();

        assert_eq!(rendered.len(), 12);
        assert_eq!(rendered.chars().nth(8), Some('.'));
        assert!(
            rendered[..8]
                .chars()
                .all(|ch| ch.is_ascii_digit() || ch == ':')
        );
        assert!(rendered[9..].chars().all(|ch| ch.is_ascii_digit()));
    }

    #[test]
    fn write_details_renders_tool_detail_inline() {
        let mut rendered = String::new();
        let mut writer = Writer::new(&mut rendered);
        write_details(
            &mut writer,
            Some("before_tool"),
            Some("shell"),
            Some(r#"{"cmd":"git status"}"#),
            None,
            None,
        )
        .unwrap();

        assert_eq!(rendered, r#"(before_tool, shell {"cmd":"git status"})"#);
    }

    #[test]
    fn build_env_filter_is_silent_without_opt_in() {
        let filter = build_env_filter(None, None);
        assert_eq!(filter.to_string(), "off");
    }

    #[test]
    fn build_env_filter_honors_rust_log() {
        let filter = build_env_filter(Some("info"), None);
        assert!(filter.to_string().contains("info"));
    }

    #[test]
    fn build_env_filter_honors_debug_level() {
        let filter = build_env_filter(None, Some(DebugLevel::Debug));
        assert!(filter.to_string().contains("claudine=debug"));
    }

    #[test]
    fn parse_formatted_fields_handles_quotes() {
        let parsed = parse_formatted_fields(
            r#"provider=OpenCode, event=before_tool, tool_name="bash", can_block=true"#,
        );
        assert_eq!(
            parsed,
            vec![
                ("provider".to_string(), "OpenCode".to_string()),
                ("event".to_string(), "before_tool".to_string()),
                ("tool_name".to_string(), "bash".to_string()),
                ("can_block".to_string(), "true".to_string()),
            ]
        );
    }
}
