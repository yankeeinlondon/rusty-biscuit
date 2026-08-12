use color_eyre::eyre::Result;

use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::components::table::table::{Table, TableCellContent, TableColumn};
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::UnicodeWidthStr;
use biscuit_terminal::utils::layout::{Alignment, Edges, Length, Width, WordWrap};
use claudine::events::event_support_matrix;
use claudine::provider::{EventSupportLevel, Provider};

use crate::log;

use super::{ALL_PROVIDERS, bold, chunk_size_for_width, render_unmapped_events_note};

/// One entry in the support-glyph vocabulary.
///
/// The same entries drive both the matrix cells and the legend, so a glyph
/// that can appear in a cell is always documented and the two cannot drift.
pub(super) struct SupportGlyph {
    /// Cell glyph. Emoji entries are display-width 2; the dash is width 1
    /// and center-aligned, which reads fine alongside width-2 cells.
    pub glyph: &'static str,
    /// Short label shown in the legend.
    pub label: &'static str,
    /// Legend elaboration.
    pub meaning: &'static str,
    /// Render the cell dimmed (used for the "not supported" dash).
    pub dim: bool,
}

const HOOK: SupportGlyph = SupportGlyph {
    glyph: "✅",
    label: "hook",
    meaning: "config-file registration",
    dim: false,
};
const INDIRECT: SupportGlyph = SupportGlyph {
    glyph: "🔶",
    label: "indirect",
    meaning: "delivered via wrapper, stream parsing, or wire proxy",
    dim: false,
};
const ACP: SupportGlyph = SupportGlyph {
    glyph: "🅐",
    label: "acp",
    meaning: "captured via the Agent Client Protocol",
    dim: false,
};
const NONE: SupportGlyph = SupportGlyph {
    glyph: "–",
    label: "not supported",
    meaning: "no capture path",
    dim: true,
};

/// Complete glyph vocabulary, in legend order.
pub(super) const SUPPORT_GLYPHS: [&SupportGlyph; 4] = [&HOOK, &INDIRECT, &ACP, &NONE];

/// Map a support level onto the glyph vocabulary.
pub(super) fn support_glyph(level: &EventSupportLevel) -> &'static SupportGlyph {
    match level {
        EventSupportLevel::Hook { .. } => &HOOK,
        EventSupportLevel::StreamParse { .. }
        | EventSupportLevel::WireProxy { .. }
        | EventSupportLevel::Wrapper { .. } => &INDIRECT,
        EventSupportLevel::Acp { .. } => &ACP,
        EventSupportLevel::NotSupported => &NONE,
    }
}

/// Legend markup derived from [`SUPPORT_GLYPHS`], one line per glyph.
///
/// Stacked lines keep every entry readable at any terminal width; a single
/// joined line overflows narrow panes mid-word (Prose does not wrap by
/// default).
fn legend_lines() -> Vec<String> {
    SUPPORT_GLYPHS
        .iter()
        .enumerate()
        .map(|(index, glyph)| {
            let prefix = if index == 0 {
                "<dim>Legend: </dim>"
            } else {
                "        "
            };
            // Emoji glyphs are display-width 2, the dash is 1; pad so the
            // `=` signs line up.
            let pad = " ".repeat(2usize.saturating_sub(UnicodeWidthStr::width(glyph.glyph)));
            format!(
                "{prefix}{}{pad}<dim> = {} ({})</dim>",
                glyph.glyph, glyph.label, glyph.meaning
            )
        })
        .collect()
}

/// Build the glyph matrix for a subset of providers.
fn build_support_table(providers: &[Provider], term: &Terminal) -> Table {
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

    for matrix_row in event_support_matrix(providers) {
        let mut row: Vec<TableCellContent> = vec![matrix_row.event.as_pascal_case().into()];

        for cell in matrix_row.cells {
            let glyph = support_glyph(&cell.level);
            let rendered: TableCellContent = if glyph.dim {
                Prose::new(format!("<dim>{}</dim>", glyph.glyph))
                    .render(term)
                    .into()
            } else {
                glyph.glyph.into()
            };
            row.push(rendered);
        }

        table.add_row(row);
    }

    table
}

/// Show the provider event support matrix with a glyph per capture path.
pub(super) fn run_support() -> Result<()> {
    let term = crate::log::terminal();
    let chunk_size = chunk_size_for_width(&ALL_PROVIDERS, term.width(), &|chunk| {
        build_support_table(chunk, &term)
    });

    for chunk in ALL_PROVIDERS.chunks(chunk_size) {
        let table = build_support_table(chunk, &term);
        log::data(&format!("\n{}", table.render(&term)));
    }

    log::data("");
    for line in legend_lines() {
        let prose = Prose::new(line).with_word_wrap(WordWrap::WrapProse(Some(8), Some(4)));
        log::data(&format!(" {}", prose.render(&term)));
    }

    render_unmapped_events_note(&term);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use claudine::events::AgenticEvent;

    #[test]
    fn legend_documents_every_cell_glyph() {
        let legend = legend_lines().join("\n");

        // Every glyph a cell can produce is documented.
        for provider in ALL_PROVIDERS {
            for event in AgenticEvent::ALL {
                let glyph = support_glyph(&provider.event_support_level(&event));
                assert!(
                    legend.contains(glyph.glyph),
                    "legend is missing glyph `{}` produced by {provider}/{event:?}",
                    glyph.glyph
                );
            }
        }

        // And the vocabulary itself is fully documented, glyph and label.
        for glyph in SUPPORT_GLYPHS {
            assert!(legend.contains(glyph.glyph));
            assert!(legend.contains(glyph.label));
        }
    }

    #[test]
    fn wide_terminals_render_a_single_table() {
        let term = Terminal::new_optimistic(200);
        let size = chunk_size_for_width(&ALL_PROVIDERS, 200, &|chunk| {
            build_support_table(chunk, &term)
        });
        assert_eq!(size, ALL_PROVIDERS.len());
    }

    #[test]
    fn narrow_terminals_chunk_providers() {
        let term = Terminal::new_optimistic(70);
        let size = chunk_size_for_width(&ALL_PROVIDERS, 70, &|chunk| {
            build_support_table(chunk, &term)
        });
        assert!(
            size < ALL_PROVIDERS.len(),
            "70 columns cannot fit all {} providers in one table (got chunk size {size})",
            ALL_PROVIDERS.len()
        );
        assert!(size >= 1);

        // Every chunk the chosen size produces must actually plan.
        for chunk in ALL_PROVIDERS.chunks(size) {
            let table = build_support_table(chunk, &term);
            assert!(
                table.plan_widths(70).is_ok(),
                "chunk {chunk:?} does not plan at 70 columns"
            );
        }
    }
}
