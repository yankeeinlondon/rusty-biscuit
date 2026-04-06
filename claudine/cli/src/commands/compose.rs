//! Top-level composition commands.
//!
//! - `claudine compose <file>` — chained composition (no file mutation)
//! - `claudine inline-compose <file>` — inline composition (replaces body)
//!
//! Both commands are thin request builders that delegate to
//! [`execute_composition_request`] for wrapper-grade execution.

use std::collections::BTreeSet;
use std::path::PathBuf;

use clap::Args;
use claudine::composition::{
    self, CompositionExecutionRequest, CompositionMode, OutputFormat as CompositionOutputFormat,
    SystemPromptInput,
};
use claudine::events::Provider;
use color_eyre::eyre::{Result, eyre};

use super::wrap::composition::execute_composition_request;
use crate::log;
use crate::provider_values::provider_value_parser;

/// Shared flags for composition commands.
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

    /// Set or append an inline system prompt.
    #[arg(
        short = 's',
        long = "system-prompt",
        value_name = "PROMPT",
        conflicts_with = "system_prompt_file"
    )]
    pub system_prompt: Option<String>,

    /// Load the system prompt from a file.
    #[arg(
        long = "system-prompt-file",
        value_name = "FILE",
        conflicts_with = "system_prompt"
    )]
    pub system_prompt_file: Option<PathBuf>,

    /// Timeout in seconds (sends SIGTERM then SIGKILL). Only valid in non-interactive mode.
    #[arg(short = 't', long = "timeout", value_name = "SECONDS")]
    pub timeout: Option<u64>,

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

    /// Show only the header line; suppress env details and info messages.
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
}

impl SharedComposeArgs {
    pub(crate) fn explicit_provider(&self) -> Option<Provider> {
        self.provider
            .or_else(|| self.claude.then_some(Provider::Claude))
            .or_else(|| self.codex.then_some(Provider::Codex))
            .or_else(|| self.gemini.then_some(Provider::Gemini))
            .or_else(|| self.goose.then_some(Provider::Goose))
            .or_else(|| self.kimicode.then_some(Provider::KimiCode))
            .or_else(|| self.opencode.then_some(Provider::OpenCode))
            .or_else(|| self.qwen.then_some(Provider::QwenCode))
            .or_else(|| self.roo.then_some(Provider::RooCode))
    }

    pub(crate) fn excluded(&self) -> BTreeSet<Provider> {
        self.exclude.iter().copied().collect()
    }

    fn system_prompt_input(&self) -> Option<SystemPromptInput> {
        self.system_prompt
            .as_ref()
            .map(|prompt| SystemPromptInput::Inline {
                prompt: prompt.clone(),
            })
            .or_else(|| {
                self.system_prompt_file
                    .as_ref()
                    .map(|path| SystemPromptInput::File { path: path.clone() })
            })
    }
}

/// Compose a Markdown document through an agentic CLI.
#[derive(Debug, Clone, Args)]
pub struct ComposeArgs {
    #[command(flatten)]
    pub shared: SharedComposeArgs,

    /// File reference to compose.
    #[arg(value_name = "FILE")]
    pub file: String,
}

/// Inline composition: use frontmatter `prompt`, replace body with output.
#[derive(Debug, Clone, Args)]
pub struct InlineComposeArgs {
    #[command(flatten)]
    pub shared: SharedComposeArgs,

    /// File reference to compose.
    #[arg(value_name = "FILE")]
    pub file: String,
}

/// Entry point for `claudine compose`.
pub fn run_compose(args: ComposeArgs, verbose: u8) -> Result<()> {
    let code = match run_compose_inner(args, verbose) {
        Ok(code) => code,
        Err(error) => {
            log::error(&error.to_string());
            1
        }
    };
    std::process::exit(code);
}

/// Entry point for `claudine inline-compose`.
pub fn run_inline_compose(args: InlineComposeArgs, verbose: u8) -> Result<()> {
    let code = match run_inline_compose_inner(args, verbose) {
        Ok(code) => code,
        Err(error) => {
            log::error(&error.to_string());
            1
        }
    };
    std::process::exit(code);
}

fn run_compose_inner(args: ComposeArgs, verbose: u8) -> Result<i32> {
    let ComposeArgs { shared, file } = args;
    let set_overrides = parse_set_json(shared.set.as_deref())?;
    let system_prompt = shared.system_prompt_input();

    let source = composition::resolve_composition_source(&file).map_err(|e| eyre!("{e}"))?;

    // ── Pre-flight shell approval ────────────────────────────────────
    let compose_options = {
        let mut opts = darkmatter::markdown::compose::ComposeOptions::new()
            .with_source_file(&source.resolved_path);
        if let Some(ref overrides) = set_overrides {
            opts = opts.with_set_overrides(overrides.clone());
        }
        opts
    };

    let approval_options =
        super::wrap::build_harness_shell_options(&source.resolved_path, None, shared.interactive);

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
            ..Default::default()
        },
    )
    .map_err(|e| eyre!("{e}"))?;

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
        system_prompt,
        timeout: shared.timeout,
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
        shared_approval_cache: None,
        sequence: false,
    };

    execute_composition_request(request, verbose)
}

fn run_inline_compose_inner(args: InlineComposeArgs, verbose: u8) -> Result<i32> {
    let InlineComposeArgs { shared, file } = args;
    let set_overrides = parse_set_json(shared.set.as_deref())?;
    let system_prompt = shared.system_prompt_input();
    let show_checks = !shared.silent;
    let term = if show_checks {
        Some(crate::log::terminal())
    } else {
        None
    };

    // -- Pre-validation: file resolution ------------------------------------

    let source = match composition::resolve_composition_source(&file) {
        Ok(source) => {
            if let Some(ref t) = term {
                claudine::harness::report::report_source_file(&file, &source.resolved_path, t);
            }
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

    let approval_options =
        super::wrap::build_harness_shell_options(&source.resolved_path, None, shared.interactive);

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
            ..Default::default()
        },
    )
    .map_err(|e| eyre!("{e}"))?;

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
        system_prompt,
        timeout: shared.timeout,
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
        shared_approval_cache: None,
        sequence: false,
    };

    execute_composition_request(request, verbose)
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
