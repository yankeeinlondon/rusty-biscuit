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

/// Compose a Markdown document through an agentic CLI.
#[derive(Debug, Clone, Args)]
pub struct ComposeArgs {
    /// Run the provider session in interactive mode.
    #[arg(short = 'i', long)]
    pub interactive: bool,

    /// Exclude providers from automatic selection (repeatable).
    #[arg(long = "exclude", value_name = "PROVIDER")]
    pub exclude: Vec<String>,

    /// Suppress all output except the composition result.
    #[arg(long)]
    pub silent: bool,

    /// Enable Claudine-managed MCP session composition.
    #[arg(long)]
    pub mcp: bool,

    /// Activate specific MCP servers by ID or alias (comma-separated).
    #[arg(long = "use", value_name = "ID", value_delimiter = ',')]
    pub mcp_use: Vec<String>,

    /// Treat unresolved or ambiguous MCP tags as hard errors.
    #[arg(long)]
    pub strict: bool,

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

    /// File reference to compose.
    #[arg(value_name = "FILE")]
    pub file: String,
}

/// Inline composition: use frontmatter `prompt`, replace body with output.
#[derive(Debug, Clone, Args)]
pub struct InlineComposeArgs {
    /// Run the provider session in interactive mode.
    #[arg(short = 'i', long)]
    pub interactive: bool,

    /// Exclude providers from automatic selection (repeatable).
    #[arg(long = "exclude", value_name = "PROVIDER")]
    pub exclude: Vec<String>,

    /// Suppress all output except the composition result.
    #[arg(long)]
    pub silent: bool,

    /// Enable Claudine-managed MCP session composition.
    #[arg(long)]
    pub mcp: bool,

    /// Activate specific MCP servers by ID or alias (comma-separated).
    #[arg(long = "use", value_name = "ID", value_delimiter = ',')]
    pub mcp_use: Vec<String>,

    /// Treat unresolved or ambiguous MCP tags as hard errors.
    #[arg(long)]
    pub strict: bool,

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
    let excluded = parse_excluded(&args.exclude, args.silent);
    let explicit_provider = resolve_explicit_provider(
        args.claude,
        args.codex,
        args.gemini,
        args.opencode,
        args.qwen,
        args.goose,
        args.kimi,
    );

    let source =
        composition::resolve_composition_source(&args.file).map_err(|e| eyre!("{e}"))?;

    let prepared =
        composition::prepare_direct(&source).map_err(|e| eyre!("{e}"))?;

    let request = CompositionExecutionRequest {
        mode: CompositionMode::ChainedDocument,
        file_ref: args.file,
        prepared,
        explicit_provider,
        excluded,
        mcp: args.mcp,
        mcp_use: args.mcp_use,
        strict: args.strict,
        session_interactive: args.interactive,
        silent: args.silent,
    };

    execute_composition_request(request, verbose)
}

fn run_inline_compose_inner(args: InlineComposeArgs, verbose: u8) -> Result<i32> {
    let excluded = parse_excluded(&args.exclude, args.silent);
    let explicit_provider = resolve_explicit_provider(
        args.claude,
        args.codex,
        args.gemini,
        args.opencode,
        args.qwen,
        args.goose,
        args.kimi,
    );

    let source =
        composition::resolve_composition_source(&args.file).map_err(|e| eyre!("{e}"))?;

    // Validate file read/write permissions before proceeding.
    composition::validate_file_permissions(&source.resolved_path).map_err(|e| eyre!("{e}"))?;

    let repo_root = find_git_root();
    let prepared =
        composition::prepare_inline(&source, repo_root.as_deref())
            .map_err(|e| eyre!("{e}"))?;

    let request = CompositionExecutionRequest {
        mode: CompositionMode::InlineFrontmatterPrompt,
        file_ref: args.file,
        prepared,
        explicit_provider,
        excluded,
        mcp: args.mcp,
        mcp_use: args.mcp_use,
        strict: args.strict,
        session_interactive: args.interactive,
        silent: args.silent,
    };

    execute_composition_request(request, verbose)
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

fn resolve_explicit_provider(
    claude: bool,
    codex: bool,
    gemini: bool,
    opencode: bool,
    qwen: bool,
    goose: bool,
    kimi: bool,
) -> Option<Provider> {
    if claude {
        Some(Provider::Claude)
    } else if codex {
        Some(Provider::Codex)
    } else if gemini {
        Some(Provider::Gemini)
    } else if opencode {
        Some(Provider::OpenCode)
    } else if qwen {
        Some(Provider::QwenCode)
    } else if goose {
        Some(Provider::Goose)
    } else if kimi {
        Some(Provider::KimiCode)
    } else {
        None
    }
}

/// Lightweight git root detection for guardrails loading.
fn find_git_root() -> Option<std::path::PathBuf> {
    let cwd = std::env::current_dir().ok()?;
    let mut dir = cwd.as_path();
    loop {
        if dir.join(".git").exists() {
            return Some(dir.to_path_buf());
        }
        dir = dir.parent()?;
    }
}
