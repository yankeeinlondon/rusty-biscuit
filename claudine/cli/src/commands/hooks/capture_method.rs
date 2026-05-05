use color_eyre::eyre::Result;

use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::Renderable;
use biscuit_terminal::components::table::table::{Table, TableCellContent, TableColumn};
use biscuit_terminal::utils::layout::{Alignment, Margin};
use claudine::events::event_support_matrix;
use claudine::provider::EventSupportLevel;

use crate::log;

use super::{ALL_PROVIDERS, bold};

/// Indicator for ACP-based support (Goose request_permission, Kimi ApprovalRequest).
const ACP_SUPPORT: &str = "🅐";

/// Show the capture method (hook / non-hook / acp / -) for each event x
/// provider pair.
pub(super) fn run_capture_method() -> Result<()> {
    use claudine::provider::provider_info;

    let term = crate::log::terminal();
    let matrix = event_support_matrix(&ALL_PROVIDERS);

    let mut columns = vec![TableColumn::new(bold("Event"))];
    for provider in ALL_PROVIDERS {
        let info = provider_info(provider);
        let header = if info.acp.is_supported() {
            format!("{} {}", provider, ACP_SUPPORT)
        } else {
            provider.to_string()
        };
        columns.push(TableColumn::new(bold(&header)).with_alignment(Alignment::Center));
    }

    let mut table = Table::new().with_columns(columns).prefer_cursor_alignment();
    table.layout_mut().left_margin = Margin::Chars(1);

    for matrix_row in matrix {
        let mut row: Vec<TableCellContent> = vec![matrix_row.event.as_pascal_case().into()];

        for cell in matrix_row.cells {
            let rendered: TableCellContent = match cell.level {
                EventSupportLevel::Hook { .. } => "hook".into(),
                EventSupportLevel::StreamParse { protocol, .. } => {
                    format!("stream-parse ({protocol:?})").into()
                }
                EventSupportLevel::WireProxy { mode, .. } => {
                    format!("wire-proxy ({mode:?})").into()
                }
                EventSupportLevel::Wrapper { .. } => "wrapper".into(),
                EventSupportLevel::Acp { .. } => {
                    Prose::new("{{cyan}}acp{{reset}}").render(&term).into()
                }
                EventSupportLevel::NotSupported => {
                    Prose::new("{{dim}}-{{reset}}").render(&term).into()
                }
            };
            row.push(rendered);
        }

        table.add_row(row);
    }

    let rendered = table.render(&term);
    log::data(&format!("\n{}", rendered));

    log::data("");
    let legend = Prose::new(
        "{{dim}}Legend: {{reset}}hook{{dim}} = config-file hook, {{reset}}stream-parse{{dim}} = stream parsing, {{reset}}wire-proxy{{dim}} = wire-mode proxy, {{reset}}wrapper{{dim}} = wrapper script, {{reset}}{{cyan}}acp{{reset}}{{dim}} = Agent Client Protocol, {{reset}}-{{dim}} = not supported{{reset}}",
    );
    log::data(&format!(" {}", legend.render(&term)));

    let acp_note = Prose::new(
        "{{dim}}- 🅐 next to a provider name marks an ACP-supported provider (server or wire-proxy){{reset}}",
    );
    log::data(&format!(" {}", acp_note.render(&term)));

    Ok(())
}
