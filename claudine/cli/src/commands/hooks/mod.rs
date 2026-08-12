use clap::Args;
use color_eyre::eyre::Result;

use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::components::table::table::{Table, TableColumn};
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::layout::WordWrap;
use claudine::config::claudine_config::ClaudineConfig;
use claudine::config::{AgentConfigurator, detect_agents};
use claudine::dispatch::loader::load_claudine_config;
use claudine::provider::{PROVIDERS_DISPLAY_ORDER, Provider, UnmappedNativeEvent, provider_info};
use sniff::programs::InstalledAiClients;

use crate::log;
use crate::provider_values::provider_value_parser;

mod describe;
mod list;
mod mapping;
mod support;
mod variables;

use list::{run_provider_detail, run_simple, run_verbose};

/// Arguments for the hooks command.
#[derive(Args)]
pub struct HooksArgs {
    /// Optional provider name for detailed view (fuzzy matching supported)
    #[arg(value_name = "PROVIDER", value_parser = provider_value_parser())]
    pub provider: Option<Provider>,

    /// Show provider event support matrix (✅ hook / 🔶 indirect / 🅐 acp / – none)
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

    /// Hidden alias of `--support`; per-level capture detail now lives in
    /// the per-provider view (`claudine hooks <provider>`).
    #[arg(long = "capture-method", hide = true)]
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

pub(super) fn render_protect_visibility(config: Option<&ClaudineConfig>) {
    let Some(config) = config else { return };
    log::data("");
    if config.protect.enabled {
        log::data("Protect: enabled");
    } else {
        log::data("Protect: disabled");
    }
}

/// Every (provider, unmapped native event) pair, in provider display order.
///
/// These are provider-native hook phases the 16-event model cannot
/// represent; users must configure them directly in the provider.
pub(super) fn unmapped_native_events() -> Vec<(Provider, &'static UnmappedNativeEvent)> {
    ALL_PROVIDERS
        .iter()
        .flat_map(|provider| {
            provider_info(*provider)
                .unmapped_native_events
                .iter()
                .map(move |event| (*provider, event))
        })
        .collect()
}

/// Markup for one unmapped native event: name, description, remediation.
pub(super) fn unmapped_event_markup(event: &UnmappedNativeEvent) -> String {
    format!(
        "<cyan>{}</cyan> <dim>— {} {}</dim>",
        Prose::escape_text(event.native_event),
        Prose::escape_text(event.description),
        Prose::escape_text(event.remediation),
    )
}

/// Render the "Not mappable — configure natively" note.
///
/// Shared by `--support` and `--mapping`; silent when no provider carries
/// unmapped native events.
pub(super) fn render_unmapped_events_note(term: &Terminal) {
    let entries = unmapped_native_events();
    if entries.is_empty() {
        return;
    }

    log::data("");
    let header = Prose::new("<bold>Not mappable — configure natively:</bold>");
    log::data(&format!(" {}", header.render(term)));
    for (provider, event) in entries {
        let line = Prose::new(format!(
            "  <dim>-</dim> <bold>{provider}</bold>: {}",
            unmapped_event_markup(event)
        ))
        .with_word_wrap(WordWrap::WrapProse(Some(8), Some(4)));
        log::data(&format!(" {}", line.render(term)));
    }
}

/// Chunk sizes probed for provider matrices, widest first. `4` was the
/// historical fixed `--mapping` chunk size; `1` always renders.
const CHUNK_CANDIDATES: [usize; 4] = [ALL_PROVIDERS.len(), 4, 2, 1];

/// Whether the table renders at `width` with every column visible and no
/// wrapped cells.
fn table_fits(table: &Table, width: u32) -> bool {
    match table.plan_widths(width) {
        Ok(plan) if plan.dropped_column_indices.is_empty() => {
            !table.would_wrap(width).unwrap_or(true)
        }
        _ => false,
    }
}

/// Pick the widest candidate chunk size whose chunks all fit `width`.
///
/// Falls through to 1-provider chunks so a matrix always renders as
/// stacked tables — never a "Table could not be rendered" refusal.
pub(super) fn chunk_size_for_width(
    providers: &[Provider],
    width: u32,
    build_table: &dyn Fn(&[Provider]) -> Table,
) -> usize {
    for candidate in CHUNK_CANDIDATES {
        if candidate > providers.len() {
            continue;
        }
        let fits = providers
            .chunks(candidate)
            .all(|chunk| table_fits(&build_table(chunk), width));
        if fits {
            return candidate;
        }
    }
    1
}

/// Show registered hooks for all providers.
pub fn run(args: HooksArgs, verbose: bool) -> Result<()> {
    if args.support || args.capture_method {
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

    let config = load_claudine_config(None, None).ok();

    if let Some(provider) = args.provider {
        return run_provider_detail(provider, config.as_ref());
    }

    let agents = detect_agents();
    let clients = InstalledAiClients::new();

    if verbose {
        run_verbose(&agents, &clients, config.as_ref())
    } else {
        run_simple(&agents, &clients, config.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The note data must surface the catalog's known unmapped phases and
    /// carry a remediation for every entry.
    #[test]
    fn unmapped_native_events_surface_catalog_entries() {
        let entries = unmapped_native_events();

        let has = |provider: Provider, native: &str| {
            entries
                .iter()
                .any(|(p, e)| *p == provider && e.native_event == native)
        };
        assert!(has(Provider::Gemini, "BeforeToolSelection"));
        assert!(has(Provider::OpenCode, "tool.definition"));

        for (provider, event) in &entries {
            assert!(
                !event.remediation.is_empty(),
                "{provider} {} has no remediation",
                event.native_event
            );
            let markup = unmapped_event_markup(event);
            assert!(markup.contains(event.native_event));
        }
    }
}
