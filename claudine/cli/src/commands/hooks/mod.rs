use clap::Args;
use color_eyre::eyre::Result;

use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::components::table::table::TableColumn;
use biscuit_terminal::utils::layout::WordWrap;
use claudine::config::claudine_config::ClaudineConfig;
use claudine::config::{AgentConfigurator, detect_agents};
use claudine::dispatch::loader::load_claudine_config;
use claudine::provider::{PROVIDERS_DISPLAY_ORDER, Provider};
use sniff::programs::InstalledAiClients;

use crate::log;
use crate::provider_values::provider_value_parser;

mod capture_method;
mod describe;
mod list;
mod mapping;
mod support;
mod variables;

use list::{run_provider_detail, run_simple, run_verbose, validate_sound_effects};

/// Arguments for the hooks command.
#[derive(Args)]
pub struct HooksArgs {
    /// Optional provider name for detailed view (fuzzy matching supported)
    #[arg(value_name = "PROVIDER", value_parser = provider_value_parser())]
    pub provider: Option<Provider>,

    /// Show provider event support matrix (✅ hook / ⛔️ non-hook / ❌ none)
    #[arg(long)]
    pub support: bool,

    /// Show native event name mappings for each provider
    #[arg(long)]
    pub mapping: bool,

    /// Show event descriptions and schemas
    #[arg(long)]
    pub describe: bool,

    /// Show available template variables for speak/report actions
    #[arg(long)]
    pub variables: bool,

    /// Show per-event capture method (hook / non-hook / acp / -) for each provider
    #[arg(long = "capture-method")]
    pub capture_method: bool,
}

/// All supported providers in display order.
pub(super) const ALL_PROVIDERS: [Provider; claudine::provider::PROVIDER_COUNT] =
    PROVIDERS_DISPLAY_ORDER;
/// Keep provider names from wrapping into multi-line labels in narrow terminals.
const PROVIDER_COLUMN_MIN_WIDTH: usize = 11;

/// Wrap a header label in bold styling via Prose.
pub(super) fn bold(label: &str) -> String {
    Prose::new(format!("<bold>{label}</bold>")).render(&crate::log::optimistic_terminal(None))
}

pub(super) fn provider_column() -> TableColumn {
    TableColumn::new(bold("Provider"))
        .with_min_width(PROVIDER_COLUMN_MIN_WIDTH)
        .with_word_wrap(WordWrap::None)
}

/// Find the configurator for a given provider, if detected.
pub(super) fn find_configurator(
    agents: &[(Provider, Box<dyn AgentConfigurator>)],
    provider: Provider,
) -> Option<&dyn AgentConfigurator> {
    agents
        .iter()
        .find(|(p, _)| *p == provider)
        .map(|(_, cfg)| cfg.as_ref())
}

fn render_protect_visibility(config: Option<&ClaudineConfig>) {
    let Some(config) = config else { return };
    log::data("");
    if config.protect.enabled {
        log::data("Protect: enabled");
    } else {
        log::data("Protect: disabled");
    }
}

/// Show registered hooks for all providers.
pub fn run(args: HooksArgs, verbose: bool) -> Result<()> {
    if args.support {
        return support::run_support();
    }
    if args.mapping {
        return mapping::run_mapping();
    }
    if args.describe {
        return describe::run_describe();
    }
    if args.variables {
        return variables::run_variables();
    }
    if args.capture_method {
        return capture_method::run_capture_method();
    }

    let config = load_claudine_config(None, None).ok();
    render_protect_visibility(config.as_ref());

    if let Some(provider) = args.provider {
        let result = run_provider_detail(provider, config.as_ref());

        if let Some(cfg) = config.as_ref() {
            validate_sound_effects(cfg);
        }

        return result;
    }

    let agents = detect_agents();
    let clients = InstalledAiClients::new();

    let result = if verbose {
        run_verbose(&agents, &clients, config.as_ref())
    } else {
        run_simple(&agents, &clients, config.as_ref())
    };

    if let Some(cfg) = config.as_ref() {
        validate_sound_effects(cfg);
    }

    result
}
