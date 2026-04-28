use color_eyre::eyre::{Result, WrapErr};

use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::Renderable;
use biscuit_terminal::components::table::table::{Table, TableCellContent, TableColumn};
use biscuit_terminal::utils::layout::{Alignment, Margin};
use clap::{Args, ValueEnum};
use claudine::events::AgenticEvent;
use claudine::provider::{PROVIDERS_DISPLAY_ORDER, Provider};
use claudine::linking::{LinkableResource, capabilities_for};
use claudine::provider::{all_providers, provider_info};

use crate::cli_utils::bool_indicator;
use crate::log;

/// Output format for `claudine providers --describe`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ProvidersFormat {
    /// Default human-readable text output (table).
    Text,
    /// Structured JSON output sourced from `provider_info(p)`.
    Json,
}

/// Arguments accepted by the `claudine providers` command.
#[derive(Debug, Args)]
pub struct ProvidersArgs {
    /// Render structured `ProviderInfo` data from the central catalog.
    ///
    /// Without this flag, `claudine providers` shows the legacy capability
    /// matrix (skill/slash/agent/hooks). With this flag, output reflects the
    /// fields exposed by `claudine::provider::provider_info`.
    #[arg(long)]
    pub describe: bool,

    /// Output format. Only meaningful with `--describe`.
    #[arg(long, value_enum, default_value_t = ProvidersFormat::Text)]
    pub format: ProvidersFormat,
}

fn supports_custom_resource(provider: Provider, resource: LinkableResource) -> bool {
    capabilities_for(provider)
        .support_for(resource)
        .level
        .allows_custom()
}

fn supported_hook_count(provider: Provider) -> usize {
    AgenticEvent::ALL
        .into_iter()
        .filter(|event| provider.supports_event_via_hook(event))
        .count()
}

/// Show provider capabilities for skills, slash commands, agents, and hooks.
pub fn run(args: ProvidersArgs) -> Result<()> {
    if args.describe {
        return run_describe(args.format);
    }
    let term = crate::log::terminal();

    let columns = vec![
        TableColumn::new("Provider"),
        TableColumn::new("Skill").with_alignment(Alignment::Center),
        TableColumn::new("Slash").with_alignment(Alignment::Center),
        TableColumn::new("Agent").with_alignment(Alignment::Center),
        TableColumn::new("Hooks").with_alignment(Alignment::Center),
    ];

    let mut table = Table::new().with_columns(columns).prefer_cursor_alignment();
    table.layout_mut().left_margin = Margin::Chars(1);

    for provider in PROVIDERS_DISPLAY_ORDER {
        let provider_cell: TableCellContent = if crate::log::is_plain()
            || std::env::var_os("NO_COLOR").is_some()
        {
            provider.to_string().into()
        } else {
            let provider_link = format!(r#"<a href="{}">{}</a>"#, provider.docs_url(), provider);
            Prose::new(provider_link)
                .render(&crate::log::optimistic_terminal(None))
                .into()
        };

        table.add_row(vec![
            provider_cell,
            bool_indicator(supports_custom_resource(provider, LinkableResource::Skill)),
            bool_indicator(supports_custom_resource(
                provider,
                LinkableResource::Command,
            )),
            bool_indicator(supports_custom_resource(provider, LinkableResource::Agent)),
            supported_hook_count(provider).to_string().into(),
        ]);
    }

    let rendered = table.render(&term);
    log::data(&format!("\n{rendered}"));

    Ok(())
}

/// Render structured provider catalog data sourced from
/// [`claudine::provider::provider_info`].
fn run_describe(format: ProvidersFormat) -> Result<()> {
    match format {
        ProvidersFormat::Json => {
            let payload: Vec<&'static claudine::provider::ProviderInfo> = all_providers().collect();
            let json = serde_json::to_string_pretty(&payload)
                .wrap_err("failed to serialize ProviderInfo catalog")?;
            log::data(&json);
        }
        ProvidersFormat::Text => {
            let term = crate::log::terminal();
            let columns = vec![
                TableColumn::new("Provider"),
                TableColumn::new("Slug"),
                TableColumn::new("Binary"),
                TableColumn::new("Sniff Binding"),
                TableColumn::new("Skills").with_alignment(Alignment::Center),
            ];
            let mut table = Table::new().with_columns(columns).prefer_cursor_alignment();
            table.layout_mut().left_margin = Margin::Chars(1);

            for provider in PROVIDERS_DISPLAY_ORDER {
                let info = provider_info(provider);
                table.add_row(vec![
                    info.display_name.into(),
                    info.slug.into(),
                    info.binary.into(),
                    format!("{:?}", info.sniff_binding).into(),
                    bool_indicator(info.supports_skills),
                ]);
            }
            let rendered = table.render(&term);
            log::data(&format!("\n{rendered}"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_custom_command_support() {
        assert!(supports_custom_resource(
            Provider::Claude,
            LinkableResource::Command
        ));
        assert!(supports_custom_resource(
            Provider::Gemini,
            LinkableResource::Command
        ));
        assert!(supports_custom_resource(
            Provider::QwenCode,
            LinkableResource::Command
        ));
        assert!(supports_custom_resource(
            Provider::RooCode,
            LinkableResource::Command
        ));
        assert!(!supports_custom_resource(
            Provider::KimiCode,
            LinkableResource::Command
        ));
    }

    #[test]
    fn reports_custom_agent_support() {
        assert!(supports_custom_resource(
            Provider::Claude,
            LinkableResource::Agent
        ));
        assert!(supports_custom_resource(
            Provider::KimiCode,
            LinkableResource::Agent
        ));
        assert!(supports_custom_resource(
            Provider::Gemini,
            LinkableResource::Agent
        ));
    }

    #[test]
    fn counts_hook_attach_points() {
        assert_eq!(supported_hook_count(Provider::Claude), 13);
        assert_eq!(supported_hook_count(Provider::Codex), 1);
        assert_eq!(supported_hook_count(Provider::Gemini), 10);
        assert_eq!(supported_hook_count(Provider::OpenCode), 13);
        assert_eq!(supported_hook_count(Provider::Goose), 0);
        assert_eq!(supported_hook_count(Provider::KimiCode), 0);
        assert_eq!(supported_hook_count(Provider::QwenCode), 0);
        assert_eq!(supported_hook_count(Provider::RooCode), 0);
    }

    #[test]
    fn describe_json_serializes_all_providers() {
        let payload: Vec<&claudine::provider::ProviderInfo> = all_providers().collect();
        let json = serde_json::to_value(&payload).expect("ProviderInfo serializes");
        let array = json.as_array().expect("payload is a JSON array");
        assert_eq!(array.len(), PROVIDERS_DISPLAY_ORDER.len());
        for (index, provider) in PROVIDERS_DISPLAY_ORDER.into_iter().enumerate() {
            let entry = &array[index];
            assert_eq!(
                entry["provider"],
                serde_json::to_value(provider).unwrap(),
                "entry {index} has unexpected provider"
            );
            assert!(
                entry.get("display_name").is_some(),
                "entry {index} missing display_name"
            );
            assert!(entry.get("slug").is_some(), "entry {index} missing slug");
            assert!(
                entry.get("docs_url").is_some(),
                "entry {index} missing docs_url"
            );
        }
    }
}
