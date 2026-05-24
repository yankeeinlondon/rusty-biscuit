//! Notification helpers section output formatting (table and JSON).

use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::components::table::{Table as TerminalTable, TableCellContent, TableColumn};
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::layout::Alignment;
use sniff::programs::ProgramMetadata;
use sniff::programs::notification_helpers::InstalledNotificationHelpers;
use strum::IntoEnumIterator;

/// Render notification helpers information as a markdown table.
///
/// ## Returns
///
/// A String containing the formatted table ready for terminal output.
pub fn render_notification_helpers_markdown(
    helpers: &InstalledNotificationHelpers,
    verbose: u8,
) -> String {
    let term = Terminal::default();

    let mut columns = vec![
        TableColumn::new("Name"),
        TableColumn::new("Installed")
            .with_alignment(Alignment::Center)
            .with_uniform_alignment(true),
    ];

    if verbose > 0 {
        columns.push(TableColumn::new("Binary"));
        columns.push(TableColumn::new("Path"));
    }
    if verbose > 1 {
        columns.push(TableColumn::new("Version"));
    }

    columns.push(TableColumn::new("Description"));

    let mut table = TerminalTable::new()
        .with_columns(columns)
        .prefer_cursor_alignment();

    for helper in sniff::programs::NotificationHelper::iter() {
        let path_info = helpers.path_with_source(helper);
        let installed = path_info.is_some();
        let path = path_info.as_ref().map(|(p, _)| p.display().to_string());
        let version = if installed && verbose > 1 {
            helpers.version(helper).ok()
        } else {
            None
        };

        let mut cells: Vec<TableCellContent> = vec![
            helper.display_name().to_string().into(),
            (if installed { "✅" } else { "❌" }).into(),
        ];

        if verbose > 0 {
            cells.push(helper.binary_name().to_string().into());
            cells.push(path.as_deref().unwrap_or("").into());
        }
        if verbose > 1 {
            cells.push(version.as_deref().unwrap_or("").into());
        }

        cells.push(helper.description().to_string().into());
        table.add_row(cells);
    }

    let mut out = String::new();
    out.push_str("## Notification Helpers\n\n");
    out.push_str(&table.display(&term).to_string());

    if let Some(ref daemon) = helpers.active_daemon {
        out.push_str("\n\n**Active daemon:** ");
        out.push_str(&daemon.name);
        if let Some(ref version) = daemon.version {
            out.push_str(&format!(" ({})", version));
        }
        out.push('\n');
    }

    out
}

/// Print notification helpers information as JSON.
pub fn print_notification_helpers_json(
    helpers: &InstalledNotificationHelpers,
    performance: Option<&sniff::PerformanceReport>,
) -> serde_json::Result<()> {
    let value = serde_json::to_value(helpers)?;
    crate::output::print_json_value(value, performance);
    Ok(())
}
