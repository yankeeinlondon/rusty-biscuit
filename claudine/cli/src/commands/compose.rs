//! Top-level composition commands.
//!
//! - `claudine compose <file>` — chained composition (no file mutation)
//! - `claudine inline-compose <file>` — inline composition (replaces body)
//!
//! Both commands are thin request builders that delegate to
//! [`execute_composition_request`] for wrapper-grade execution.

use std::collections::BTreeSet;

use clap::Args;
use claudine::composition::{self, CompositionExecutionRequest, CompositionMode};
use claudine::events::Provider;
use color_eyre::eyre::{Result, eyre};

use super::wrap::composition::execute_composition_request;
use crate::log;

/// Shared provider-override flags for composition commands.
#[derive(Debug, Clone, Args)]
pub struct ProviderOverrideArgs {
    /// Use Claude as the provider.
    #[arg(long, group = "provider_select")]
    pub claude: bool,

    /// Use Codex as the provider.
    #[arg(long, group = "provider_select")]
    pub codex: bool,

    /// Use Gemini as the provider.
    #[arg(long, group = "provider_select")]
    pub gemini: bool,

    /// Use OpenCode as the provider.
    #[arg(long, group = "provider_select")]
    pub opencode: bool,

    /// Use Qwen Code as the provider.
    #[arg(long, group = "provider_select")]
    pub qwen: bool,

    /// Use Goose as the provider.
    #[arg(long, group = "provider_select")]
    pub goose: bool,

    /// Use Kimi Code as the provider.
    #[arg(long, group = "provider_select")]
    pub kimi: bool,
}

impl ProviderOverrideArgs {
    fn resolve(&self) -> Option<Provider> {
        if self.claude {
            Some(Provider::Claude)
        } else if self.codex {
            Some(Provider::Codex)
        } else if self.gemini {
            Some(Provider::Gemini)
        } else if self.opencode {
            Some(Provider::OpenCode)
        } else if self.qwen {
            Some(Provider::QwenCode)
        } else if self.goose {
            Some(Provider::Goose)
        } else if self.kimi {
            Some(Provider::KimiCode)
        } else {
            None
        }
    }
}

/// Compose a Markdown document through an agentic CLI.
#[derive(Debug, Clone, Args)]
pub struct ComposeArgs {
    /// Enable provider-specific YOLO/auto-approval mode.
    #[arg(short = 'y', long)]
    pub yolo: bool,

    /// Run the provider session in interactive mode.
    #[arg(short = 'i', long)]
    pub interactive: bool,

    /// Exclude providers from automatic selection (repeatable).
    #[arg(long = "exclude", value_name = "PROVIDER")]
    pub exclude: Vec<String>,

    /// Preserve this env var even when it matches sensitive-name filters.
    #[arg(long = "include", value_name = "ENV_NAME")]
    pub include: Vec<String>,

    /// Override the model used by the provider.
    #[arg(short = 'm', long = "model", value_name = "MODEL")]
    pub model: Option<String>,

    /// Set the output format (json, text, stream).
    #[arg(short = 'o', long = "output", value_name = "FORMAT")]
    pub output: Option<String>,

    /// Set or append a system prompt (string or file path).
    #[arg(short = 's', long = "system-prompt", value_name = "PROMPT|FILE")]
    pub system_prompt: Option<String>,

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

    #[command(flatten)]
    pub provider: ProviderOverrideArgs,

    /// File reference to compose.
    #[arg(value_name = "FILE")]
    pub file: String,
}

/// Inline composition: use frontmatter `prompt`, replace body with output.
#[derive(Debug, Clone, Args)]
pub struct InlineComposeArgs {
    /// Enable provider-specific YOLO/auto-approval mode.
    #[arg(short = 'y', long)]
    pub yolo: bool,

    /// Run the provider session in interactive mode.
    #[arg(short = 'i', long)]
    pub interactive: bool,

    /// Exclude providers from automatic selection (repeatable).
    #[arg(long = "exclude", value_name = "PROVIDER")]
    pub exclude: Vec<String>,

    /// Preserve this env var even when it matches sensitive-name filters.
    #[arg(long = "include", value_name = "ENV_NAME")]
    pub include: Vec<String>,

    /// Override the model used by the provider.
    #[arg(short = 'm', long = "model", value_name = "MODEL")]
    pub model: Option<String>,

    /// Set the output format (json, text, stream).
    #[arg(short = 'o', long = "output", value_name = "FORMAT")]
    pub output: Option<String>,

    /// Set or append a system prompt (string or file path).
    #[arg(short = 's', long = "system-prompt", value_name = "PROMPT|FILE")]
    pub system_prompt: Option<String>,

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

    #[command(flatten)]
    pub provider: ProviderOverrideArgs,

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
    let excluded = parse_excluded(&args.exclude, args.silent || args.quiet);
    let explicit_provider = args.provider.resolve();
    let set_overrides = parse_set_json(args.set.as_deref())?;

    let source = composition::resolve_composition_source(&args.file).map_err(|e| eyre!("{e}"))?;

    let prepared =
        composition::prepare_direct(&source, set_overrides).map_err(|e| eyre!("{e}"))?;

    let request = CompositionExecutionRequest {
        mode: CompositionMode::ChainedDocument,
        file_ref: args.file,
        prepared,
        explicit_provider,
        excluded,
        yolo: args.yolo,
        include: args.include,
        model: args.model,
        output: args.output,
        system_prompt: args.system_prompt,
        timeout: args.timeout,
        operation: args.operation,
        sandbox: args.sandbox,
        repo: args.repo,
        dry_run: args.dry_run,
        mcp: args.mcp,
        mcp_use: args.mcp_use,
        strict: args.strict,
        session_interactive: args.interactive,
        quiet: args.quiet,
        silent: args.silent,
    };

    execute_composition_request(request, verbose)
}

fn run_inline_compose_inner(args: InlineComposeArgs, verbose: u8) -> Result<i32> {
    let excluded = parse_excluded(&args.exclude, args.silent || args.quiet);
    let explicit_provider = args.provider.resolve();
    let set_overrides = parse_set_json(args.set.as_deref())?;
    let show_checks = !args.silent;
    let term = if show_checks {
        Some(crate::log::terminal())
    } else {
        None
    };

    // -- Pre-validation: file resolution ------------------------------------

    let source = match composition::resolve_composition_source(&args.file) {
        Ok(source) => {
            if let Some(ref t) = term {
                claudine::harness::report::report_source_file(&args.file, &source.resolved_path, t);
            }
            source
        }
        Err(e) => {
            if let Some(ref t) = term {
                claudine::harness::report::report_source_file(
                    &args.file,
                    std::path::Path::new(""),
                    t,
                );
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

    let prepared =
        composition::prepare_inline(&source, set_overrides).map_err(|e| eyre!("{e}"))?;

    let request = CompositionExecutionRequest {
        mode: CompositionMode::InlineFrontmatterPrompt,
        file_ref: args.file,
        prepared,
        explicit_provider,
        excluded,
        yolo: args.yolo,
        include: args.include,
        model: args.model,
        output: args.output,
        system_prompt: args.system_prompt,
        timeout: args.timeout,
        operation: args.operation,
        sandbox: args.sandbox,
        repo: args.repo,
        dry_run: args.dry_run,
        mcp: args.mcp,
        mcp_use: args.mcp_use,
        strict: args.strict,
        session_interactive: args.interactive,
        quiet: args.quiet,
        silent: args.silent,
    };

    execute_composition_request(request, verbose)
}

/// Parse `--set` JSON/JSON5, validate it's an object, return as `serde_json::Value`.
fn parse_set_json(raw: Option<&str>) -> Result<Option<serde_json::Value>> {
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

fn parse_excluded(exclude: &[String], silent: bool) -> BTreeSet<Provider> {
    exclude
        .iter()
        .filter_map(|name| {
            Provider::fuzzy_match_cli_name(name).or_else(|| {
                if !silent {
                    eprintln!("warning: unknown provider '{name}', ignoring --exclude");
                }
                None
            })
        })
        .collect()
}
