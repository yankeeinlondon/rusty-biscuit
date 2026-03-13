//! Top-level `compose` command for agent-selected composition.
//!
//! - `claudine compose <file>` — chained composition (no file mutation)
//! - `claudine compose inline <file>` — inline composition (replaces body)

use std::collections::BTreeSet;
use std::io::IsTerminal;

use clap::{Args, Subcommand};
use claudine::composition::{
    self, CompositionMode, CompositionRequest, PreparedPrompt, ResolvedCompositionSource,
    SelectedProvider, SelectionReason,
};
use claudine::events::Provider;
use color_eyre::eyre::{Result, eyre};
use inquire::Select;
use sniff::programs::InstalledAiClients;

use super::wrap::{env, exec, profile};
use crate::log;

/// Compose a Markdown document through an agentic CLI.
#[derive(Debug, Clone, Args)]
pub struct ComposeArgs {
    /// Force interactive provider selection.
    #[arg(short = 'i', long)]
    pub interactive: bool,

    /// Exclude providers from automatic selection (repeatable).
    #[arg(long = "exclude", value_name = "PROVIDER")]
    pub exclude: Vec<String>,

    /// Timeout in seconds for the provider session.
    #[arg(short = 't', long = "timeout", value_name = "SECONDS")]
    pub timeout: Option<u64>,

    /// Suppress all output except the composition result.
    #[arg(long)]
    pub silent: bool,

    /// File reference to compose (chained mode).
    #[arg(value_name = "FILE")]
    pub file: Option<String>,

    #[command(subcommand)]
    pub subcommand: Option<ComposeSubcommand>,
}

#[derive(Debug, Clone, Subcommand)]
pub enum ComposeSubcommand {
    /// Inline composition: use frontmatter `prompt`, replace body with output.
    Inline(InlineArgs),
}

#[derive(Debug, Clone, Args)]
pub struct InlineArgs {
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

fn run_compose_inner(args: ComposeArgs, _verbose: u8) -> Result<i32> {
    // Determine mode and file reference
    let (mode, file_ref) = match &args.subcommand {
        Some(ComposeSubcommand::Inline(inline_args)) => {
            (CompositionMode::InlineFrontmatterPrompt, inline_args.file.clone())
        }
        None => {
            let file = args.file.as_ref().ok_or_else(|| eyre!("file argument is required"))?;
            (CompositionMode::ChainedDocument, file.clone())
        }
    };

    // Parse excluded providers
    let excluded: BTreeSet<Provider> = args
        .exclude
        .iter()
        .filter_map(|name| {
            Provider::fuzzy_match_cli_name(name).or_else(|| {
                if !args.silent {
                    eprintln!("warning: unknown provider '{}', ignoring --exclude", name);
                }
                None
            })
        })
        .collect();

    // Resolve source document
    let source = composition::resolve_composition_source(&file_ref)
        .map_err(|e| eyre!("{e}"))?;

    // Prepare prompt
    let prepared = match mode {
        CompositionMode::InlineFrontmatterPrompt => {
            composition::prepare_inline_prompt(&source)
                .map_err(|e| eyre!("{e}"))?
        }
        CompositionMode::ChainedDocument => {
            composition::prepare_chained_prompt(&source)
                .map_err(|e| eyre!("{e}"))?
        }
    };

    // Build composition request
    let request = CompositionRequest {
        mode,
        file_ref: file_ref.clone(),
        explicit_provider: None,
        excluded,
        force_interactive_selection: args.interactive,
    };

    // Detect installed providers
    let clients = InstalledAiClients::new();
    let installed: Vec<Provider> = [
        Provider::Claude,
        Provider::Codex,
        Provider::Gemini,
        Provider::Goose,
        Provider::KimiCode,
        Provider::OpenCode,
        Provider::QwenCode,
    ]
    .into_iter()
    .filter(|p| clients.path(p.sniff_ai_cli()).is_some())
    .collect();

    // Load provider preferences
    let preference = load_provider_preferences();

    // Select provider (may require interactive selection)
    let selected = match composition::select_provider(&request, &prepared, &installed, &preference)
    {
        Ok(selected) => selected,
        Err(claudine::composition::CompositionError::InteractiveSelectionRequired) => {
            interactive_select(&installed, &request.excluded)?
        }
        Err(claudine::composition::CompositionError::AgentHintAmbiguous {
            hint: _,
            matches: _,
            providers,
        }) => {
            if is_tty() {
                interactive_select_from(&providers)?
            } else {
                return Err(eyre!(
                    "agent hint is ambiguous and no TTY available for interactive selection"
                ));
            }
        }
        Err(e) => return Err(eyre!("{e}")),
    };

    if !args.silent {
        eprintln!(
            "  Using {} ({})",
            selected.provider,
            reason_label(selected.reason)
        );
    }

    // Execute with provider retry for preference fallback
    execute_composition(
        &source,
        &prepared,
        selected,
        &installed,
        &preference,
        &request,
        &clients,
        &args,
    )
}

#[allow(clippy::too_many_arguments)]
fn execute_composition(
    source: &ResolvedCompositionSource,
    prepared: &PreparedPrompt,
    selected: SelectedProvider,
    installed: &[Provider],
    preference: &[Provider],
    request: &CompositionRequest,
    clients: &InstalledAiClients,
    args: &ComposeArgs,
) -> Result<i32> {
    let mut providers_to_try = vec![selected.provider];

    // For preference fallback, add remaining ranked providers
    if selected.reason == SelectionReason::PreferenceFallback {
        let ranked = claudine::linking::ranked_provider_preferences(installed, preference);
        for p in ranked {
            if p != selected.provider
                && !request.excluded.contains(&p)
                && !providers_to_try.contains(&p)
            {
                providers_to_try.push(p);
            }
        }
    }

    for (i, provider) in providers_to_try.iter().enumerate() {
        let result = run_provider_composition(
            source, prepared, *provider, clients, args,
        );

        match result {
            Ok(code) => return Ok(code),
            Err(e) => {
                if i + 1 < providers_to_try.len()
                    && selected.reason == SelectionReason::PreferenceFallback
                {
                    if !args.silent {
                        eprintln!(
                            "  {} failed ({}), trying {}...",
                            provider,
                            e,
                            providers_to_try[i + 1]
                        );
                    }
                    continue;
                }
                return Err(e);
            }
        }
    }

    Err(eyre!("all providers failed"))
}

fn run_provider_composition(
    source: &ResolvedCompositionSource,
    prepared: &PreparedPrompt,
    provider: Provider,
    clients: &InstalledAiClients,
    args: &ComposeArgs,
) -> Result<i32> {
    let wrapper_profile = profile::profile_for_provider(provider)
        .ok_or_else(|| eyre!("{} cannot be wrapped", provider))?;

    let binary_path = clients
        .path(provider.sniff_ai_cli())
        .ok_or_else(|| eyre!("{} is not installed", provider))?;

    let cwd = std::env::current_dir()?;

    // Build minimal child args and env
    let mut child_args = Vec::new();
    let mut stdin_seed: Option<String> = None;

    wrapper_profile.apply_non_interactive(&mut child_args)?;
    wrapper_profile.apply_non_interactive_defaults(&mut child_args);

    // Deliver the prompt
    wrapper_profile.apply_prompt_body(
        &mut child_args,
        &mut stdin_seed,
        &prepared.prompt,
        true,
    )?;

    // Build env
    let env_plan = env::build_child_env(
        wrapper_profile,
        provider,
        &[],   // no include overrides
        false,  // no yolo
        false,  // non-interactive
        &[],    // no raw agent params
        &cwd,
        &[],    // no env overrides
        false,  // no repo mode
        false,  // no mcp shadow home
    )?;

    let child_cwd = env_plan.repo_root.as_deref().unwrap_or(&cwd);

    let stdout_noise = wrapper_profile.stdout_noise_prefixes();
    let stderr_noise = wrapper_profile.stderr_noise_prefixes();

    let captured = exec::run_child_capture(
        &binary_path,
        &child_args,
        &env_plan.env,
        child_cwd,
        args.timeout,
        exec::ChildIoOptions {
            stdout_noise_prefixes: stdout_noise,
            stderr_noise_prefixes: stderr_noise,
            stdin_seed: stdin_seed.as_deref(),
        },
    )?;

    match prepared.mode {
        CompositionMode::ChainedDocument => {
            if captured.exit_code == 0 {
                print!("{}", captured.stdout);
            } else if !captured.stderr.is_empty() {
                eprintln!("{}", captured.stderr);
            }
            Ok(captured.exit_code)
        }
        CompositionMode::InlineFrontmatterPrompt => {
            if captured.exit_code == 0 {
                let mut updated_md = source.markdown.clone();
                let today = chrono::Local::now().format("%Y-%m-%d").to_string();
                updated_md
                    .fm_insert("last_updated", &today)
                    .map_err(|e| eyre!("failed to update last_updated: {e}"))?;
                *updated_md.content_mut() = captured.stdout;

                let doc_string = updated_md.as_string();
                claudine::config::atomic::atomic_write(
                    &source.resolved_path,
                    doc_string.as_bytes(),
                )
                .map_err(|e| eyre!("failed to write: {e}"))?;

                if !args.silent {
                    eprintln!(
                        "  \x1b[32m✓\x1b[0m Updated {}",
                        source.resolved_path.display()
                    );
                }
            } else if !captured.stderr.is_empty() {
                eprintln!("{}", captured.stderr);
            }
            Ok(captured.exit_code)
        }
    }
}

fn interactive_select(
    installed: &[Provider],
    excluded: &BTreeSet<Provider>,
) -> Result<SelectedProvider> {
    if !is_tty() {
        return Err(eyre!(
            "interactive provider selection required but no TTY available; \
             set AGENT env var or add an `agent` frontmatter property"
        ));
    }

    let candidates: Vec<Provider> = composition::build_candidate_set(installed, excluded);
    if candidates.is_empty() {
        return Err(eyre!("no runnable providers available"));
    }

    interactive_select_from(&candidates)
}

fn interactive_select_from(candidates: &[Provider]) -> Result<SelectedProvider> {
    let options: Vec<String> = candidates.iter().map(|p| p.to_string()).collect();
    let selection = Select::new("Choose a provider:", options)
        .prompt()
        .map_err(|e| eyre!("selection cancelled: {e}"))?;

    let provider = candidates
        .iter()
        .find(|p| p.to_string() == selection)
        .copied()
        .ok_or_else(|| eyre!("invalid selection"))?;

    Ok(SelectedProvider {
        provider,
        reason: SelectionReason::InteractiveChoice,
    })
}

fn is_tty() -> bool {
    std::io::stdin().is_terminal() && std::io::stdout().is_terminal()
}

fn load_provider_preferences() -> Vec<Provider> {
    // TODO: Load from claudine settings once settings infrastructure exists.
    // For now, return a sensible default ordering.
    vec![
        Provider::Claude,
        Provider::Codex,
        Provider::Gemini,
    ]
}

fn reason_label(reason: SelectionReason) -> &'static str {
    match reason {
        SelectionReason::ExplicitProvider => "explicit",
        SelectionReason::EnvironmentOverride => "AGENT env",
        SelectionReason::FrontmatterHint => "frontmatter hint",
        SelectionReason::InteractiveChoice => "interactive choice",
        SelectionReason::PreferenceFallback => "preference",
    }
}
