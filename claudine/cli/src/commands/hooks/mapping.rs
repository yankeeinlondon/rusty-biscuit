use color_eyre::eyre::Result;

use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::components::table::table::{Table, TableCellContent, TableColumn};
use biscuit_terminal::utils::layout::{Alignment, Edges, Length, Width};
use claudine::events::{NativeEventName, event_native_mapping_matrix};
use claudine::provider::Provider;

use crate::log;

use super::{ALL_PROVIDERS, bold};

/// Show native event name mappings for each provider.
///
/// Splits providers into as many stacked tables as the terminal width
/// requires (see [`super::chunk_size_for_width`]).
pub(super) fn run_mapping() -> Result<()> {
    let term = crate::log::terminal();
    let chunk_size = super::chunk_size_for_width(&ALL_PROVIDERS, term.width(), &|chunk| {
        build_mapping_table(chunk)
    });

    for provider_group in ALL_PROVIDERS.chunks(chunk_size) {
        let table = build_mapping_table(provider_group);
        let rendered = table.render(&term);
        log::data(&format!("\n{}", rendered));
    }

    log::data("");
    let legend =
        Prose::new("<dim>- Legend: (blank) = not supported or no specific native name</dim>");
    log::data(&format!(
        " {}",
        legend.render(&crate::log::optimistic_terminal(Some(100)))
    ));

    // Native phases with no canonical row above — they exist only natively.
    super::render_unmapped_events_note(&term);

    Ok(())
}

/// Build a mapping table for a subset of providers.
fn build_mapping_table(providers: &[Provider]) -> Table {
    let mut columns = vec![TableColumn::new(bold("Event"))];
    for provider in providers {
        columns
            .push(TableColumn::new(bold(&provider.to_string())).with_alignment(Alignment::Center));
    }

    let mut table = Table::new().with_columns(columns).prefer_cursor_alignment();
    table.layout_mut().margin = Edges::x(Length::ch(1));
    // Hug content: the default `Auto` width fills the terminal and the last
    // provider column absorbs all slack, which stretches absurdly wide.
    table.layout_mut().width = Width::FitContent;

    for matrix_row in event_native_mapping_matrix(providers) {
        let mut row: Vec<TableCellContent> = vec![matrix_row.event.as_pascal_case().into()];

        for cell in matrix_row.cells {
            let cell: TableCellContent = match cell.native {
                NativeEventName::Unsupported => "".into(),
                NativeEventName::NoSpecificName => "".into(),
                NativeEventName::Named(name) => name.into(),
            };
            row.push(cell);
        }

        table.add_row(row);
    }

    table
}
