//! Top-level composition commands.
//!
//! - `claudine compose <file>` — chained composition (no file mutation)
//! - `claudine inline-compose <file>` — inline composition (replaces body)
//!
//! Both commands are thin request builders that delegate to
//! [`execute_composition_request`] for wrapper-grade execution.

use std::collections::BTreeSet;

use clap::Args;
use claudine::composition::{
    self, CompositionExecutionRequest, CompositionMode, OutputFormat as CompositionOutputFormat,
};
use claudine::events::Provider;
use claudine::system_prompt::SystemPromptArgs;
use color_eyre::eyre::{Result, eyre};
use tracing::info_span;

use super::wrap::composition::execute_composition_request;
use crate::log;
use crate::provider_values::provider_value_parser;

/// Shared flags for composition commands.
///
/// ## Notes
///
/// The eight provider boolean fields (`claude`, `codex`, `gemini`, `goose`,
/// `kimicode`, `opencode`, `qwen`, `roo`) are handled entirely by the
/// pre-clap argv normalizer in [`crate::argv`]: Rule 1 rewrites each
/// `--<provider>` token to the canonical `--provider <slug>` pair before
/// clap ever sees it. The struct fields and clap `#[arg(...)]` declarations
/// are retained as user-facing help entries only — their parsed boolean
/// values are never read at runtime (see [`Self::explicit_provider`]).
///
/// Retiring these fields is tracked in the
/// `2026-04-17-cli-pre-processing` spec follow-ups.
#[derive(Debug, Clone, Args)]
pub struct SharedComposeArgs {
    /// Use a specific provider.
    #[arg(long, value_parser = provider_value_parser(), group = "compose_provider")]
    pub provider: Option<Provider>,

    /// Use Claude Code.
    #[arg(long, group = "compose_provider")]
    pub claude: bool,

    /// Use Codex CLI.
    #[arg(long, group = "compose_provider")]
    pub codex: bool,

    /// Use Gemini CLI.
    #[arg(long, group = "compose_provider")]
    pub gemini: bool,

    /// Use Goose.
    #[arg(long, group = "compose_provider")]
    pub goose: bool,

    /// Use Kimi Code.
    #[arg(long = "kimi", group = "compose_provider")]
    pub kimicode: bool,

    /// Use OpenCode.
    #[arg(long, group = "compose_provider")]
    pub opencode: bool,

    /// Use Qwen Code.
    #[arg(long = "qwen", group = "compose_provider")]
    pub qwen: bool,

    /// Use Roo Code.
    #[arg(long = "roo", group = "compose_provider")]
    pub roo: bool,

    /// Exclude providers from automatic selection (repeatable).
    #[arg(long = "exclude", value_name = "PROVIDER", value_parser = provider_value_parser())]
    pub exclude: Vec<Provider>,

    /// Enable provider-specific YOLO/auto-approval mode.
    #[arg(short = 'y', long)]
    pub yolo: bool,

    /// Run the provider session in interactive mode.
    #[arg(short = 'i', long)]
    pub interactive: bool,

    /// Preserve this env var even when it matches sensitive-name filters.
    #[arg(long = "include", value_name = "ENV_NAME")]
    pub include: Vec<String>,

    /// Override the model used by the provider.
    #[arg(short = 'm', long = "model", value_name = "MODEL")]
    pub model: Option<String>,

    /// Set the output format (json, text, stream).
    #[arg(short = 'o', long = "output", value_name = "FORMAT")]
    pub output: Option<CompositionOutputFormat>,

    /// Append a system prompt from a file.
    #[arg(
        long = "append-system-prompt",
        visible_alias = "asp",
        value_name = "FILE",
        conflicts_with = "replace_system_prompt"
    )]
    pub append_system_prompt: Option<String>,

    /// Replace the provider's system prompt with contents from a file.
    #[arg(
        long = "replace-system-prompt",
        visible_alias = "rsp",
        value_name = "FILE",
        conflicts_with = "append_system_prompt"
    )]
    pub replace_system_prompt: Option<String>,

    /// Timeout in seconds (sends SIGTERM then SIGKILL). Only valid in non-interactive mode.
    #[arg(short = 't', long = "timeout", value_name = "SECONDS")]
    pub timeout: Option<u64>,

    /// Step-silence timeout (e.g. `30s`, `5m`). Kills the child when no stream
    /// event is observed for this long. Only valid in non-interactive
    /// structured-stream mode.
    #[arg(long = "step-timeout", value_name = "DURATION")]
    pub step_timeout: Option<String>,

    /// Set the OPERATION env var for the composed session.
    #[arg(long = "operation", visible_alias = "op", value_name = "OP")]
    pub operation: Option<String>,

    /// Enable provider-specific sandboxing.
    #[arg(long)]
    pub sandbox: bool,

    /// Use only repo-scoped skills, commands, and agents via a shadow HOME.
    #[arg(long)]
    pub repo: bool,

    /// Show what would be executed without launching the child.
    #[arg(long)]
    pub dry_run: bool,

    /// Suppress env details and info messages, but still show the system prompt when set.
    #[arg(short = 'q', long)]
    pub quiet: bool,

    /// Suppress all output except the composition result.
    #[arg(long, conflicts_with = "quiet")]
    pub silent: bool,

    /// Override frontmatter values as JSON/JSON5 (e.g. `--set '{"key":"val"}'`).
    #[arg(long, value_name = "JSON")]
    pub set: Option<String>,

    /// Enable Claudine-managed MCP session composition.
    #[arg(long)]
    pub mcp: bool,

    /// Activate specific MCP servers by ID or alias (comma-separated).
    #[arg(long = "use", value_name = "ID", value_delimiter = ',')]
    pub mcp_use: Vec<String>,

    /// Treat unresolved or ambiguous MCP tags as hard errors.
    #[arg(long)]
    pub strict: bool,

    /// Emit a performance report to stderr after command completion.
    #[arg(long)]
    pub perf: bool,
}

impl SharedComposeArgs {
    /// Explicit provider selected on the command line.
    ///
    /// After [`crate::argv::normalize`], provider boolean flags (`--claude`,
    /// `--gemini`, …) have already been rewritten to `--provider <slug>`,
    /// so runtime selection only needs to read the canonical `provider`
    /// field. The boolean fields remain on the struct as user-facing help
    /// entries but are never set in practice.
    pub(crate) fn explicit_provider(&self) -> Option<Provider> {
        self.provider
    }

    pub(crate) fn excluded(&self) -> BTreeSet<Provider> {
        self.exclude.iter().copied().collect()
    }

    pub(crate) fn system_prompt_args(&self) -> SystemPromptArgs {
        SystemPromptArgs {
            append_file: self.append_system_prompt.clone(),
            replace_file: self.replace_system_prompt.clone(),
        }
    }

    /// Parse `--step-timeout DURATION` into seconds using the same
    /// [`claudine::harness::parse_timeout`] grammar frontmatter uses, so CLI
    /// and frontmatter errors share one vocabulary. Returns `Ok(None)` when
    /// the flag was not supplied.
    pub(crate) fn step_timeout_secs(&self) -> Result<Option<u64>> {
        match self.step_timeout.as_deref() {
            Some(raw) => {
                let duration =
                    claudine::harness::parse_timeout(raw, std::path::Path::new("<--step-timeout>"))
                        .map_err(|e| eyre!("invalid --step-timeout value: {e}"))?;
                Ok(Some(duration.as_secs()))
            }
            None => Ok(None),
        }
    }
}

/// Compose a Markdown document through an agentic CLI.
///
/// Positional tokens are one file reference plus optional `key=value` setters
/// in any order. Inline setters override `--set` on overlapping keys.
#[derive(Debug, Clone, Args)]
pub struct ComposeArgs {
    #[command(flatten)]
    pub shared: SharedComposeArgs,

    /// File reference and/or `key=value` setters.
    #[arg(value_name = "ARG", num_args = 1.., required = true)]
    pub args: Vec<String>,
}

/// Inline composition: use frontmatter `prompt`, replace body with output.
///
/// Positional tokens are one file reference plus optional `key=value` setters
/// in any order. Inline setters override `--set` on overlapping keys.
#[derive(Debug, Clone, Args)]
pub struct InlineComposeArgs {
    #[command(flatten)]
    pub shared: SharedComposeArgs,

    /// File reference and/or `key=value` setters.
    #[arg(value_name = "ARG", num_args = 1.., required = true)]
    pub args: Vec<String>,
}

/// Entry point for `claudine compose`.
pub fn run_compose(
    args: ComposeArgs,
    verbose: u8,
    startup_timings: Option<crate::perf::StartupTimings>,
) -> Result<()> {
    let code = match run_compose_inner(args, verbose, startup_timings) {
        Ok(code) => code,
        Err(error) => {
            if !crate::output::shell_expansion_error::is_pre_rendered(&error) {
                log::error(&error.to_string());
            }
            1
        }
    };
    std::process::exit(code);
}

/// Entry point for `claudine inline-compose`.
pub fn run_inline_compose(
    args: InlineComposeArgs,
    verbose: u8,
    startup_timings: Option<crate::perf::StartupTimings>,
) -> Result<()> {
    let code = match run_inline_compose_inner(args, verbose, startup_timings) {
        Ok(code) => code,
        Err(error) => {
            if !crate::output::shell_expansion_error::is_pre_rendered(&error) {
                log::error(&error.to_string());
            }
            1
        }
    };
    std::process::exit(code);
}

fn run_compose_inner(
    args: ComposeArgs,
    verbose: u8,
    startup_timings: Option<crate::perf::StartupTimings>,
) -> Result<i32> {
    let ComposeArgs { shared, args } = args;
    let parsed = parse_composition_positionals(&args)?;
    let file = parsed.file_ref.ok_or_else(|| {
        eyre!("missing file reference: expected exactly one file reference plus optional key=value setters")
    })?;
    if shared.step_timeout.is_some() && shared.interactive {
        return Err(eyre!(
            "--step-timeout cannot be used with --interactive mode"
        ));
    }
    let cli_step_timeout_secs = shared.step_timeout_secs()?;
    let set_overrides = merge_set_overrides(shared.set.as_deref(), parsed.shorthand_setters)?;
    let system_prompt_args = shared.system_prompt_args();

    let source = composition::resolve_composition_source(&file).map_err(|e| eyre!("{e}"))?;

    let _compose_span = info_span!(
        "compose",
        file = %source.resolved_path.display(),
        provider = ?shared.explicit_provider(),
        interactive = shared.interactive,
    )
    .entered();

    // ── Pre-flight shell approval ────────────────────────────────────
    let compose_options = {
        let mut opts = darkmatter::markdown::compose::ComposeOptions::new()
            .with_source_file(&source.resolved_path);
        if let Some(ref overrides) = set_overrides {
            opts = opts.with_set_overrides(overrides.clone());
        }
        opts
    };

    let shared_approval_cache =
        std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::new()));

    let approval_options = super::wrap::build_harness_shell_options_with_cache(
        &source.resolved_path,
        None,
        Some(std::sync::Arc::clone(&shared_approval_cache)),
    );

    let preflight = composition::resolve_shell_approvals(
        Some(&source.markdown),
        Some(&compose_options),
        None,
        &approval_options,
    )
    .map_err(|e| eyre!("{e}"))?;

    let prepared = composition::prepare_direct(
        &source,
        composition::PrepareOptions {
            set_overrides,
            pre_approved_commands: Some(preflight.approved_commands),
            perf_enabled: shared.perf,
            ..Default::default()
        },
    )
    .map_err(crate::output::shell_expansion_error::pretty_or_report)?;

    let request = CompositionExecutionRequest {
        mode: CompositionMode::ChainedDocument,
        file_ref: file,
        prepared,
        explicit_provider: shared.explicit_provider(),
        excluded: shared.excluded(),
        yolo: shared.yolo,
        include: shared.include,
        model: shared.model,
        output: shared.output,
        system_prompt_args,
        timeout: shared.timeout,
        step_timeout: cli_step_timeout_secs,
        operation: shared.operation,
        sandbox: shared.sandbox,
        repo: shared.repo,
        dry_run: shared.dry_run,
        mcp: shared.mcp,
        mcp_use: shared.mcp_use,
        strict: shared.strict,
        session_interactive: shared.interactive,
        quiet: shared.quiet,
        silent: shared.silent,
        env_overrides: std::collections::BTreeMap::new(),
        shared_approval_cache: Some(shared_approval_cache),
        sequence: false,
    };

    execute_composition_request(request, verbose, startup_timings, shared.perf)
}

fn run_inline_compose_inner(
    args: InlineComposeArgs,
    verbose: u8,
    startup_timings: Option<crate::perf::StartupTimings>,
) -> Result<i32> {
    let InlineComposeArgs { shared, args } = args;
    let parsed = parse_composition_positionals(&args)?;
    let file = parsed.file_ref.ok_or_else(|| {
        eyre!("missing file reference: expected exactly one file reference plus optional key=value setters")
    })?;
    if shared.step_timeout.is_some() && shared.interactive {
        return Err(eyre!(
            "--step-timeout cannot be used with --interactive mode"
        ));
    }
    let cli_step_timeout_secs = shared.step_timeout_secs()?;
    let set_overrides = merge_set_overrides(shared.set.as_deref(), parsed.shorthand_setters)?;
    let system_prompt_args = shared.system_prompt_args();
    let show_checks = !shared.silent;
    let term = if show_checks {
        Some(crate::log::terminal())
    } else {
        None
    };

    let source = match composition::resolve_composition_source(&file) {
        Ok(source) => {
            if let Some(ref t) = term {
                claudine::harness::report::report_source_file(&file, &source.resolved_path, t);
            }
            let _inline_span = info_span!(
                "inline_compose",
                file = %source.resolved_path.display(),
                provider = ?shared.explicit_provider(),
                interactive = shared.interactive,
            )
            .entered();
            source
        }
        Err(e) => {
            if let Some(ref t) = term {
                claudine::harness::report::report_source_file(&file, std::path::Path::new(""), t);
            }
            return Err(eyre!("{e}"));
        }
    };

    // -- Pre-validation: prompt frontmatter property ------------------------

    let prompt_value = source
        .markdown
        .frontmatter()
        .as_map()
        .get("prompt")
        .cloned();
    let has_prompt = prompt_value.is_some();
    let is_non_empty = prompt_value
        .as_ref()
        .and_then(|v| v.as_str())
        .is_some_and(|s| !s.trim().is_empty());

    if let Some(ref t) = term {
        claudine::harness::report::report_prompt_property(has_prompt, is_non_empty, t);
    }

    // ── Pre-flight shell approval ────────────────────────────────────
    let compose_options = {
        let mut opts = darkmatter::markdown::compose::ComposeOptions::new()
            .with_source_file(&source.resolved_path);
        if let Some(ref overrides) = set_overrides {
            opts = opts.with_set_overrides(overrides.clone());
        }
        opts
    };

    let shared_approval_cache =
        std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::new()));

    let approval_options = super::wrap::build_harness_shell_options_with_cache(
        &source.resolved_path,
        None,
        Some(std::sync::Arc::clone(&shared_approval_cache)),
    );

    let preflight = composition::resolve_shell_approvals(
        Some(&source.markdown),
        Some(&compose_options),
        None,
        &approval_options,
    )
    .map_err(|e| eyre!("{e}"))?;

    let prepared = composition::prepare_inline(
        &source,
        composition::PrepareOptions {
            set_overrides,
            pre_approved_commands: Some(preflight.approved_commands),
            perf_enabled: shared.perf,
            ..Default::default()
        },
    )
    .map_err(crate::output::shell_expansion_error::pretty_or_report)?;

    let request = CompositionExecutionRequest {
        mode: CompositionMode::InlineFrontmatterPrompt,
        file_ref: file,
        prepared,
        explicit_provider: shared.explicit_provider(),
        excluded: shared.excluded(),
        yolo: shared.yolo,
        include: shared.include,
        model: shared.model,
        output: shared.output,
        system_prompt_args,
        timeout: shared.timeout,
        step_timeout: cli_step_timeout_secs,
        operation: shared.operation,
        sandbox: shared.sandbox,
        repo: shared.repo,
        dry_run: shared.dry_run,
        mcp: shared.mcp,
        mcp_use: shared.mcp_use,
        strict: shared.strict,
        session_interactive: shared.interactive,
        quiet: shared.quiet,
        silent: shared.silent,
        env_overrides: std::collections::BTreeMap::new(),
        shared_approval_cache: Some(shared_approval_cache),
        sequence: false,
    };

    execute_composition_request(request, verbose, startup_timings, shared.perf)
}

/// Parse `--set` JSON/JSON5, validate it's an object, return as `serde_json::Value`.
pub(crate) fn parse_set_json(raw: Option<&str>) -> Result<Option<serde_json::Value>> {
    let Some(json_str) = raw else {
        return Ok(None);
    };
    let parsed = biscuit_file::Json5::from_str(json_str)
        .map_err(|e| eyre!("Invalid JSON/JSON5 in --set argument: {e}"))?;
    let value = parsed.value().clone();
    if !value.is_object() {
        return Err(eyre!(
            "Invalid --set argument: expected a JSON object like {{\"name\":\"Alice\"}}"
        ));
    }
    Ok(Some(value))
}

/// Parse an inline shorthand setter value: JSON5 first, string fallback.
pub(crate) fn parse_shorthand_value(raw: &str) -> serde_json::Value {
    if raw.is_empty() {
        return serde_json::Value::String(String::new());
    }
    match biscuit_file::Json5::from_str(raw) {
        Ok(parsed) => parsed.value().clone(),
        Err(_) => serde_json::Value::String(raw.to_string()),
    }
}

/// Classify a positional token as a shorthand setter.
///
/// ## Returns
/// - `None` — token is not a setter (pass through as file candidate)
/// - `Some(Err)` — setter syntax recognized but invalid (empty key)
/// - `Some(Ok((key, value)))` — valid setter
pub(crate) fn parse_compose_setter(
    token: &str,
) -> Option<std::result::Result<(String, serde_json::Value), String>> {
    let eq_pos = token.find('=')?;
    let key = &token[..eq_pos];
    let raw_value = &token[eq_pos + 1..];

    if key.is_empty() {
        return Some(Err("setter key must not be empty".to_string()));
    }

    let mut chars = key.chars();
    let first = chars.next().unwrap();
    if !first.is_ascii_alphabetic() && first != '_' {
        return None;
    }

    for ch in chars {
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
            continue;
        }
        return None;
    }

    let value = parse_shorthand_value(raw_value);
    Some(Ok((key.to_string(), value)))
}

/// Result of classifying composition positional tokens.
#[derive(Debug, Default)]
pub(crate) struct ParsedCompositionPositionals {
    pub file_ref: Option<String>,
    pub shorthand_setters: serde_json::Map<String, serde_json::Value>,
}

/// Classify positional tokens into an optional file reference and setter map.
///
/// ## Errors
/// - empty-key setter (`=foo`)
/// - multiple non-setter tokens (more than one file-ref candidate)
pub(crate) fn parse_composition_positionals(
    args: &[String],
) -> Result<ParsedCompositionPositionals> {
    let mut file_ref: Option<String> = None;
    let mut shorthand_setters = serde_json::Map::new();

    for token in args {
        match parse_compose_setter(token) {
            Some(Ok((key, value))) => {
                shorthand_setters.insert(key, value);
            }
            Some(Err(e)) => {
                return Err(eyre!("Invalid setter '{}': {}", token, e));
            }
            None => {
                if file_ref.is_some() {
                    let candidates: Vec<&str> = args
                        .iter()
                        .filter(|t| parse_compose_setter(t).is_none())
                        .map(|t| t.as_str())
                        .collect();
                    return Err(eyre!(
                        "expected at most one file reference, but got multiple: {}",
                        candidates.join(", ")
                    ));
                }
                file_ref = Some(token.clone());
            }
        }
    }

    Ok(ParsedCompositionPositionals {
        file_ref,
        shorthand_setters,
    })
}

/// Merge `--set` JSON with shorthand setters. Shorthand wins on overlapping keys.
///
/// ## Returns
/// - `Ok(None)` when both sources are empty
/// - `Ok(Some(Value::Object(...)))` otherwise
pub(crate) fn merge_set_overrides(
    raw_set: Option<&str>,
    shorthand: serde_json::Map<String, serde_json::Value>,
) -> Result<Option<serde_json::Value>> {
    let base = parse_set_json(raw_set)?;
    let mut map = match base {
        Some(serde_json::Value::Object(m)) => m,
        Some(_) => unreachable!("parse_set_json enforces object shape"),
        None => serde_json::Map::new(),
    };
    for (key, value) in shorthand {
        map.insert(key, value);
    }
    if map.is_empty() {
        Ok(None)
    } else {
        Ok(Some(serde_json::Value::Object(map)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    fn s(values: &[&str]) -> Vec<String> {
        values.iter().map(|v| v.to_string()).collect()
    }

    // ── parse_shorthand_value ────────────────────────────────────────

    #[test]
    fn shorthand_value_empty_string() {
        assert_eq!(parse_shorthand_value(""), Value::String(String::new()));
    }

    #[test]
    fn shorthand_value_number() {
        assert_eq!(parse_shorthand_value("3"), json!(3));
    }

    #[test]
    fn shorthand_value_boolean() {
        assert_eq!(parse_shorthand_value("true"), json!(true));
    }

    #[test]
    fn shorthand_value_array() {
        assert_eq!(parse_shorthand_value(r#"["a","b"]"#), json!(["a", "b"]));
    }

    #[test]
    fn shorthand_value_object_json5() {
        assert_eq!(
            parse_shorthand_value(r#"{mode:"fast"}"#),
            json!({"mode": "fast"})
        );
    }

    #[test]
    fn shorthand_value_plain_string_fallback() {
        assert_eq!(
            parse_shorthand_value("review.md"),
            Value::String("review.md".into())
        );
    }

    #[test]
    fn shorthand_value_url_fallback() {
        assert_eq!(
            parse_shorthand_value("https://x/?a=b"),
            Value::String("https://x/?a=b".into())
        );
    }

    // ── parse_compose_setter ─────────────────────────────────────────

    #[test]
    fn setter_valid_string() {
        let res = parse_compose_setter("review=review.md").unwrap().unwrap();
        assert_eq!(res, ("review".into(), Value::String("review.md".into())));
    }

    #[test]
    fn setter_underscore_key() {
        let res = parse_compose_setter("_private=true").unwrap().unwrap();
        assert_eq!(res, ("_private".into(), json!(true)));
    }

    #[test]
    fn setter_hyphen_key() {
        let res = parse_compose_setter("my-key=value").unwrap().unwrap();
        assert_eq!(res, ("my-key".into(), Value::String("value".into())));
    }

    #[test]
    fn setter_empty_value() {
        let res = parse_compose_setter("key=").unwrap().unwrap();
        assert_eq!(res, ("key".into(), Value::String("".into())));
    }

    #[test]
    fn setter_first_eq_split() {
        let res = parse_compose_setter("url=https://x/?a=b").unwrap().unwrap();
        assert_eq!(res, ("url".into(), Value::String("https://x/?a=b".into())));
    }

    #[test]
    fn setter_empty_key_errors() {
        let res = parse_compose_setter("=foo").unwrap();
        assert!(res.is_err());
    }

    #[test]
    fn setter_digit_start_rejected() {
        assert!(parse_compose_setter("9key=value").is_none());
    }

    #[test]
    fn setter_dot_path_rejected() {
        assert!(parse_compose_setter("foo.bar=baz").is_none());
    }

    #[test]
    fn setter_slash_key_rejected() {
        assert!(parse_compose_setter("/path=val").is_none());
    }

    #[test]
    fn setter_no_equals_rejected() {
        assert!(parse_compose_setter("file.md").is_none());
    }

    // ── parse_composition_positionals ────────────────────────────────

    #[test]
    fn positionals_file_only() {
        let parsed = parse_composition_positionals(&s(&["file.md"])).unwrap();
        assert_eq!(parsed.file_ref.as_deref(), Some("file.md"));
        assert!(parsed.shorthand_setters.is_empty());
    }

    #[test]
    fn positionals_setter_only() {
        let parsed = parse_composition_positionals(&s(&["key=val"])).unwrap();
        assert!(parsed.file_ref.is_none());
        assert_eq!(
            parsed.shorthand_setters.get("key"),
            Some(&Value::String("val".into()))
        );
    }

    #[test]
    fn positionals_file_then_setter() {
        let parsed = parse_composition_positionals(&s(&["file.md", "key=val"])).unwrap();
        assert_eq!(parsed.file_ref.as_deref(), Some("file.md"));
        assert_eq!(
            parsed.shorthand_setters.get("key"),
            Some(&Value::String("val".into()))
        );
    }

    #[test]
    fn positionals_setter_then_file() {
        let parsed = parse_composition_positionals(&s(&["key=val", "file.md"])).unwrap();
        assert_eq!(parsed.file_ref.as_deref(), Some("file.md"));
        assert_eq!(
            parsed.shorthand_setters.get("key"),
            Some(&Value::String("val".into()))
        );
    }

    #[test]
    fn positionals_multiple_setters_around_file() {
        let parsed = parse_composition_positionals(&s(&["a=1", "file.md", "b=2"])).unwrap();
        assert_eq!(parsed.file_ref.as_deref(), Some("file.md"));
        assert_eq!(parsed.shorthand_setters.get("a"), Some(&json!(1)));
        assert_eq!(parsed.shorthand_setters.get("b"), Some(&json!(2)));
    }

    #[test]
    fn positionals_duplicate_setter_last_wins() {
        let parsed = parse_composition_positionals(&s(&["k=old", "k=new"])).unwrap();
        assert_eq!(
            parsed.shorthand_setters.get("k"),
            Some(&Value::String("new".into()))
        );
    }

    #[test]
    fn positionals_two_files_errors() {
        let err = parse_composition_positionals(&s(&["a.md", "b.md"])).unwrap_err();
        assert!(err.to_string().contains("multiple"));
    }

    #[test]
    fn positionals_empty_key_errors() {
        let err = parse_composition_positionals(&s(&["=foo"])).unwrap_err();
        assert!(err.to_string().contains("must not be empty"));
    }

    #[test]
    fn positionals_dot_path_is_file_candidate() {
        let parsed = parse_composition_positionals(&s(&["foo.bar=baz"])).unwrap();
        assert_eq!(parsed.file_ref.as_deref(), Some("foo.bar=baz"));
        assert!(parsed.shorthand_setters.is_empty());
    }

    // ── merge_set_overrides ──────────────────────────────────────────

    #[test]
    fn merge_both_empty() {
        let result = merge_set_overrides(None, serde_json::Map::new()).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn merge_set_only() {
        let result = merge_set_overrides(Some(r#"{"a":"b"}"#), serde_json::Map::new()).unwrap();
        assert_eq!(result, Some(json!({"a": "b"})));
    }

    #[test]
    fn merge_shorthand_only() {
        let mut short = serde_json::Map::new();
        short.insert("k".into(), Value::String("v".into()));
        let result = merge_set_overrides(None, short).unwrap();
        assert_eq!(result, Some(json!({"k": "v"})));
    }

    #[test]
    fn merge_shorthand_wins() {
        let mut short = serde_json::Map::new();
        short.insert("k".into(), Value::String("new".into()));
        let result = merge_set_overrides(Some(r#"{"k":"old"}"#), short).unwrap();
        assert_eq!(result, Some(json!({"k": "new"})));
    }

    #[test]
    fn merge_disjoint() {
        let mut short = serde_json::Map::new();
        short.insert("b".into(), json!(2));
        let result = merge_set_overrides(Some(r#"{"a":"1"}"#), short).unwrap();
        assert_eq!(result, Some(json!({"a": "1", "b": 2})));
    }
}
