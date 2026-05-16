use color_eyre::eyre::Result;

use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::components::table::table::{Table, TableCellContent, TableColumn};
use biscuit_terminal::utils::layout::{Alignment, Margin};
use claudine::events::event_support_matrix;
use claudine::provider::EventSupportLevel;

use crate::log;

use super::{ALL_PROVIDERS, bold};

/// Indicator for hook-based support (config file registration).
const HOOK_SUPPORT: &str = "✅";
/// Indicator for non-hook support (wrapper/wire-mode required).
const NON_HOOK_SUPPORT: &str = "⛔️";
/// Indicator for ACP-based support (Goose request_permission, Kimi ApprovalRequest).
const ACP_SUPPORT: &str = "🅐";
/// Indicator for no support (using ❌ U+274C which has width 2 like other emoji).
const NO_SUPPORT: &str = "❌";

/// Show provider event support matrix with ✅/⛔️/❌ indicators.
pub(super) fn run_support() -> Result<()> {
    let term = crate::log::terminal();
    let matrix = event_support_matrix(&ALL_PROVIDERS);

    let mut columns = vec![TableColumn::new(bold("Event"))];
    for provider in ALL_PROVIDERS {
        columns
            .push(TableColumn::new(bold(&provider.to_string())).with_alignment(Alignment::Center));
    }

    let mut table = Table::new().with_columns(columns).prefer_cursor_alignment();
    table.layout_mut().left_margin = Margin::Chars(1);

    for matrix_row in matrix {
        let mut row: Vec<TableCellContent> = vec![matrix_row.event.as_pascal_case().into()];

        for cell in matrix_row.cells {
            let rendered = match cell.level {
                EventSupportLevel::Hook { .. } => HOOK_SUPPORT.into(),
                EventSupportLevel::StreamParse { .. }
                | EventSupportLevel::WireProxy { .. }
                | EventSupportLevel::Wrapper { .. } => NON_HOOK_SUPPORT.into(),
                EventSupportLevel::Acp { .. } => ACP_SUPPORT.into(),
                EventSupportLevel::NotSupported => NO_SUPPORT.into(),
            };
            row.push(rendered);
        }

        table.add_row(row);
    }

    let rendered = table.render(&term);
    log::data(&format!("\n{}", rendered));

    log::data("");
    let legend = Prose::new(
        "{{dim}}Legend: {{reset}}✅{{dim}} = hook support (config file), {{reset}}⛔️{{dim}} = non-hook (wrapper/proxy required), {{reset}}{{NO_SUPPORT}}{{dim}} = not supported{{reset}}",
    );
    log::data(&format!(" {}\n", legend.render(&term)));

    Ok(())
}
