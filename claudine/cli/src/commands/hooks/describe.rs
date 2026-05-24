use color_eyre::eyre::Result;

use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::components::table::table::{Table, TableCellContent, TableColumn};
use biscuit_terminal::utils::layout::{Length, Margin};
use claudine::events::AgenticEvent;

use crate::log;

use super::bold;

/// Show event descriptions and schemas.
pub(super) fn run_describe() -> Result<()> {
    let columns = vec![
        TableColumn::new(bold("Event")),
        TableColumn::new(bold("Response Schema")),
        TableColumn::new(bold("Return Schema")),
        TableColumn::new(bold("Description")),
    ];

    let term = crate::log::terminal();
    let mut table = Table::new().with_columns(columns).alternate_text_color();
    table.layout_mut().margin = Margin::x(Length::ch(1));

    for event in AgenticEvent::ALL {
        let row: Vec<TableCellContent> = vec![
            event.as_pascal_case().into(),
            event.response_schema().into(),
            event.return_schema().into(),
            event.description().into(),
        ];
        table.add_row(row);
    }

    let rendered = table.render(&term);
    log::data(&format!("\n{}", rendered));

    log::data("");
    let legend = Prose::new("<dim>- Response Schema: fields available in the event payload</dim>");
    log::data(&format!(" {}", legend.render(&term)));
    let legend2 = Prose::new(
        "<dim>- Return Schema: what hooks can return to influence agent behavior (blocking hooks only)</dim>",
    );
    log::data(&format!(" {}", legend2.render(&term)));

    Ok(())
}
