use color_eyre::eyre::Result;

use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::Renderable;
use biscuit_terminal::components::table::table::{Table, TableCellContent, TableColumn};
use biscuit_terminal::utils::layout::{Alignment, Margin};
use claudine::events::{AgenticEvent, PROVIDERS_DISPLAY_ORDER, Provider};
use claudine::linking::{LinkableResource, capabilities_for};

use crate::cli_utils::bool_indicator;
use crate::log;

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
pub fn run() -> Result<()> {
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
}
