use biscuit_terminal::components::list::UnorderedList;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::components::table::table::{Table, TableColumn};
use biscuit_terminal::discovery::detection::ColorDepth;
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::block_constraint::visible_width;
use biscuit_terminal::utils::layout::{Alignment, Edges, Length, TargetValue, WordWrap};

/// Maximum visible width for context reports, including margins, borders, and
/// separators.
#[allow(dead_code)]
pub const MAX_REPORT_WIDTH: u32 = 140;

/// Minimum terminal width, in visible cells, at which every context report
/// renders all of its required columns by wrapping.
///
/// The reports never drop a column or introduce a Claudine-specific narrow
/// layout (see the spec's "Narrow Terminals" clause). Every required column is
/// preserved by letting the break-aware wrap policy fit content to the available
/// width — but the widest unbreakable catalog tokens (e.g. the `Type` value
/// `NestedMarkdownList` and the `Safety` value `MarkdownMutation`) impose a hard
/// floor below which three columns cannot coexist without overflow. At or above
/// this width all columns survive; below it the shared `Table` component's own
/// constrained-width behavior applies. This is the documented minimum supported
/// width referenced by the spec.
#[allow(dead_code)]
pub const MIN_SUPPORTED_REPORT_WIDTH: u32 = 53;

/// Break characters for report table cells.
///
/// Catalog tokens — `ctx.`-prefixed property names, function/capability
/// signatures, and type names — contain no spaces, so the default prose policy
/// (which only breaks on whitespace and `-`) treats e.g.
/// `ctx.current_package_area_has_staged_files` as one 41-cell unbreakable token.
/// The planner then floors the column at that token's width and refuses to lay
/// out the table at narrow terminals. Allowing breaks at identifier and
/// signature punctuation lets long tokens wrap at meaningful boundaries
/// (`ctx.current` ⇒ `package` ⇒ …) instead of failing to render.
#[allow(dead_code)]
const REPORT_BREAK_CHARS: &[char] = &['-', '_', '.', '(', ')', ',', '/', ':', '[', ']'];

/// The wrap policy applied to every report table column: prose wrapping plus the
/// identifier/signature break characters in [`REPORT_BREAK_CHARS`].
#[allow(dead_code)]
fn report_wrap() -> WordWrap {
    WordWrap::BespokeProse(Some(8), REPORT_BREAK_CHARS.to_vec(), None)
}

/// A report table column with the shared break-aware wrap policy applied.
#[allow(dead_code)]
pub fn report_column(header: impl Into<String>) -> TableColumn {
    TableColumn::new(header.into()).with_word_wrap(report_wrap())
}

/// Inline-code helper: convert backtick-delimited spans in `input` to
/// `<inverse>…</inverse>` for styled output, preserving surrounding prose.
///
/// In plain / `NO_COLOR` / `--plain` output the original backticks are kept
/// visible. Unmatched backticks are rendered as literal text and do not panic.
pub fn render_inline_code(input: &str, term: &Terminal) -> String {
    if term.color_depth == ColorDepth::None {
        return input.to_string();
    }

    let mut output = String::with_capacity(input.len() + 32);
    let mut in_code = false;
    let mut buf = String::new();

    for ch in input.chars() {
        if ch == '`' {
            if in_code {
                output.push_str("<inverse>");
                output.push_str(&buf);
                output.push_str("</inverse>");
                buf.clear();
            } else {
                output.push_str(&buf);
                buf.clear();
            }
            in_code = !in_code;
        } else {
            buf.push(ch);
        }
    }

    if in_code {
        // Unmatched backtick: emit the opening tick and remaining buffer as
        // literal text so the string is always renderable.
        output.push('`');
        output.push_str(&buf);
    } else {
        output.push_str(&buf);
    }

    output
}

/// Convenience wrapper that returns a [`Prose`] with inline-code spans
/// resolved for the current terminal.
#[allow(dead_code)]
pub fn prose_with_inline_code(input: &str, term: &Terminal) -> Prose {
    Prose::new(render_inline_code(input, term))
}

/// The single inline-code-aware path for table headers and cells.
///
/// Returns a fully rendered string: backtick spans become inverse SGR in styled
/// output and keep visible backticks in plain output. `TableColumn`/`TableCell`
/// treat their text as opaque (no Prose markup interpretation and width is
/// measured ANSI-aware), so headers and cells must arrive already rendered —
/// passing raw `<inverse>…</inverse>` markup into a cell would print the literal
/// tags. Use this for every header or cell that may contain inline code.
#[allow(dead_code)]
pub fn inline_code_text(input: &str, term: &Terminal) -> String {
    prose_with_inline_code(input, term).render(term)
}

/// Unordered-list helper: renders `items` through [`UnorderedList`] with the
/// default `- ` bullet. Each item is run through the inline-code helper so
/// backticked spans are styled correctly.
///
/// The list has a 1ch right margin and is surrounded by exactly one blank
/// line on each side.
#[allow(dead_code)]
pub fn render_unordered_list(items: &[String], term: &Terminal) -> String {
    if items.is_empty() {
        return String::new();
    }

    let mut list = UnorderedList::empty();
    for item in items {
        let rendered = render_inline_code(item, term);
        list.add(Prose::new(rendered));
    }

    list.layout_mut().margin.right = TargetValue::universal(Length::ch(1));
    list.layout_mut().word_wrap = WordWrap::WrapProse(None, None);

    format!("\n{}\n", list.render(term))
}

/// Shared table-layout helper: applies the 140ch-inclusive width contract to a
/// [`Table`].
///
/// - 1ch left margin and 1ch right margin
/// - Total rendered width never exceeds `min(terminal width, 140)` visible
///   cells, counting margins, borders, separators, and content
/// - Below 140ch the table uses the available width without overflow
#[allow(dead_code)]
pub fn configure_shared_table(table: &mut Table) {
    table.layout_mut().margin = Edges::x(Length::ch(1));
}

/// Renders `table` under the shared width contract.
#[allow(dead_code)]
pub fn render_table_within_contract(table: &Table, term: &Terminal) -> String {
    let mut capped = term.clone();
    capped.fixed_width = Some(term.width().min(MAX_REPORT_WIDTH));
    table.render(&capped)
}

/// The layout a [`render_table_resilient`] builder must produce for each
/// escalation step.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[allow(dead_code)]
pub enum TableLayout {
    /// Width pins applied so sections align across the report.
    Pinned,
    /// Pins removed; break-aware columns wrap to fit the available width while
    /// every column is preserved.
    Wrap,
}

/// The sentinel the table planner emits inline when no layout fits the width.
const TABLE_WIDTH_ERROR_SENTINEL: &str = "could not be rendered";

/// Renders a table under the width contract, relaxing the shared width pins only
/// as far as the terminal forces.
///
/// `build` constructs the table for a given [`TableLayout`]. Pins
/// (`with_min_width`/`with_fixed_width`) are hard floors — too narrow to honor
/// them, the planner emits its width-error string inline — so this first tries
/// the pinned layout and falls back to the break-aware [`TableLayout::Wrap`]
/// layout, which keeps every column and lets the break-aware wrap policy fit the
/// content to the available width. No column is ever dropped: the spec forbids a
/// Claudine-specific narrow layout, so below the [minimum supported width] the
/// `Wrap` layout's output is returned verbatim (the shared `Table` component's
/// own constrained-width behavior applies).
///
/// [minimum supported width]: MIN_SUPPORTED_REPORT_WIDTH
///
/// `build` is invoked once per attempt so callers do not need clonable cells.
#[allow(dead_code)]
pub fn render_table_resilient(term: &Terminal, build: impl Fn(TableLayout) -> Table) -> String {
    let pinned = render_table_within_contract(&build(TableLayout::Pinned), term);
    if !pinned.contains(TABLE_WIDTH_ERROR_SENTINEL) {
        return pinned;
    }
    render_table_within_contract(&build(TableLayout::Wrap), term)
}

/// Renders one context section table (`Property` / `Type` / final column).
///
/// The shared `property_width`/`type_width` are pinned as minimum widths so
/// sections align — but only when the width can afford them. Below that the pins
/// drop and the break-aware columns wrap, keeping all three columns at every
/// supported width. No column is dropped (see [`MIN_SUPPORTED_REPORT_WIDTH`]).
///
/// `add_rows` receives the table and pushes one row per descriptor; it is
/// invoked once per layout attempt so callers do not need clonable cells.
#[allow(dead_code)]
pub fn render_context_section(
    term: &Terminal,
    property_width: usize,
    type_width: usize,
    final_header: &str,
    add_rows: impl Fn(&mut Table),
) -> String {
    render_table_resilient(term, |layout| {
        let mut property = report_column("Property");
        let mut ty = report_column("Type").with_alignment(Alignment::Center);
        if layout == TableLayout::Pinned {
            property = property.with_min_width(property_width);
            ty = ty.with_min_width(type_width);
        }
        let mut table = Table::new().with_columns(vec![
            property,
            ty,
            report_column(final_header.to_string()),
        ]);
        configure_shared_table(&mut table);
        add_rows(&mut table);
        table
    })
}

/// Context-column width helper: returns the intrinsic `(property_width,
/// type_width)` across the supplied labels.
///
/// The two widths are computed independently and are **not** forced equal.
/// Headers (`"Property"` and `"Type"`) are included in the measurement.
#[allow(dead_code)]
pub fn context_column_widths(
    property_labels: &[&str],
    type_labels: &[&str],
) -> (usize, usize) {
    let property_width = max_visible_width(property_labels.iter().copied(), "Property");
    let type_width = max_visible_width(type_labels.iter().copied(), "Type");
    (property_width, type_width)
}

#[allow(dead_code)]
fn max_visible_width<'a>(labels: impl Iterator<Item = &'a str>, header: &str) -> usize {
    labels
        .map(|s| visible_width(s) as usize)
        .chain(std::iter::once(visible_width(header) as usize))
        .max()
        .unwrap_or(0)
}

/// Function/capability first-column helper: returns the intrinsic width for a
/// signature column, capped at 40ch. Headers (`"Function"` or `"Capability"`)
/// participate in the measurement.
#[allow(dead_code)]
pub fn function_first_column_width<'a>(
    header: &str,
    signatures: impl Iterator<Item = &'a str>,
) -> usize {
    let header_width = visible_width(header) as usize;
    signatures
        .map(|s| visible_width(s) as usize)
        .max()
        .unwrap_or(0)
        .max(header_width)
        .min(40)
}

#[cfg(test)]
mod tests {
    use super::*;
    use biscuit_terminal::components::table::table::{Table, TableCellContent, TableColumn};
    use biscuit_terminal::utils::layout::Alignment;

    fn styled_term(width: u32) -> Terminal {
        Terminal::new_optimistic(width)
    }

    fn plain_term(width: u32) -> Terminal {
        let mut term = Terminal::new_optimistic(width);
        term.color_depth = ColorDepth::None;
        term.is_tty = false;
        term
    }

    fn max_line_visible_width(output: &str) -> usize {
        output
            .lines()
            .map(|line| visible_width(line) as usize)
            .max()
            .unwrap_or(0)
    }

    // =====================================================================
    // Inline code
    // =====================================================================

    #[test]
    fn inline_code_styled_wraps_single_span_in_inverse() {
        let term = styled_term(80);
        let out = Prose::new(render_inline_code("use `ctx.today` here", &term)).render(&term);
        assert!(
            out.contains("\x1b[7m"),
            "styled output must contain inverse SGR; got {out:?}"
        );
        assert!(
            !out.contains("`"),
            "styled output must not contain literal backticks; got {out:?}"
        );
    }

    #[test]
    fn inline_code_plain_keeps_visible_backticks() {
        let term = plain_term(80);
        let out = Prose::new(render_inline_code("use `ctx.today` here", &term)).render(&term);
        assert!(
            out.contains("`ctx.today`"),
            "plain output must keep visible backticks; got {out:?}"
        );
        assert!(
            !out.contains("\x1b[7m"),
            "plain output must not emit inverse SGR; got {out:?}"
        );
    }

    #[test]
    fn inline_code_styled_handles_multiple_spans_independently() {
        let term = styled_term(80);
        let input = "refer to `ctx.today` and `ctx.now` for timing";
        let rendered = render_inline_code(input, &term);
        let out = Prose::new(rendered).render(&term);

        let inverse_count = out.matches("\x1b[7m").count();
        assert_eq!(
            inverse_count, 2,
            "expected two inverse regions for two code spans; got {out:?}"
        );
    }

    #[test]
    fn inline_code_unmatched_backtick_does_not_panic() {
        let term = styled_term(80);
        let input = "unmatched `backtick";
        let rendered = render_inline_code(input, &term);
        // Should fall back to literal text (opening backtick preserved).
        assert!(rendered.contains("`backtick"));
        let _ = Prose::new(rendered).render(&term);
    }

    #[test]
    fn inline_code_text_renders_inverse_styled_and_backticks_plain() {
        // Headers and cells share this one path; it must emit inverse SGR in
        // styled output and visible backticks (never raw markup) in plain.
        let styled = inline_code_text("`||` meaning", &styled_term(80));
        assert!(
            styled.contains("\x1b[7m") && !styled.contains('`') && !styled.contains("<inverse>"),
            "styled header text must be inverse SGR with no backticks/markup; got {styled:?}"
        );

        let plain = inline_code_text("`||` meaning", &plain_term(80));
        assert!(
            plain.contains("`||`") && !plain.contains("\x1b[7m") && !plain.contains("<inverse>"),
            "plain header text must keep visible backticks and no SGR/markup; got {plain:?}"
        );
    }

    #[test]
    fn inline_code_routes_through_prose_list_and_table_cells() {
        let term = styled_term(80);
        let items = vec!["call `min(a, b)` to compare".to_string()];
        let list_out = render_unordered_list(&items, &term);
        assert!(
            list_out.contains("\x1b[7m"),
            "list items must carry inverse styling; got {list_out:?}"
        );

        let cell: TableCellContent =
            Prose::new(render_inline_code("use `ctx.foo`", &term)).render(&term).into();
        let mut table = Table::new().with_columns(vec![TableColumn::new("Hint")]);
        table.add_row(vec![cell]);
        let table_out = table.render(&term);
        assert!(
            table_out.contains("\x1b[7m"),
            "table cells must carry inverse styling; got {table_out:?}"
        );
    }

    // =====================================================================
    // Unordered list
    // =====================================================================

    #[test]
    fn unordered_list_uses_dash_bullet_and_hanging_indent() {
        let term = styled_term(80);
        let items = vec![
            "First item".to_string(),
            "Second item that is intentionally long and should wrap within the available width so we can verify hanging indent".to_string(),
        ];
        let out = render_unordered_list(&items, &term);

        assert!(
            out.starts_with('\n'),
            "list must start with a blank line; got {out:?}"
        );
        assert!(
            out.ends_with('\n'),
            "list must end with a blank line; got {out:?}"
        );

        let lines: Vec<&str> = out.lines().collect();
        let first_line = lines.iter().find(|l| l.trim_start().starts_with("- First"))
            .expect("first item must render with '- ' bullet");
        assert!(first_line.contains("- First item"));

        let first_indent = first_line.len() - first_line.trim_start().len();
        let continuation = lines
            .iter()
            .find(|l| l.starts_with("  width so we can verify"))
            .expect("wrapped continuation line must be present");
        let continuation_indent = continuation.len() - continuation.trim_start().len();
        assert!(
            continuation_indent > first_indent,
            "hanging indent must align continuation past the bullet; first={first_indent}, continuation={continuation_indent}"
        );
    }

    #[test]
    fn unordered_list_empty_yields_empty_string() {
        let term = styled_term(80);
        assert!(render_unordered_list(&[], &term).is_empty());
    }

    // =====================================================================
    // Table layout contract
    // =====================================================================

    fn build_sample_table() -> Table {
        let columns = vec![
            TableColumn::new("Property"),
            TableColumn::new("Type").with_alignment(Alignment::Center),
            TableColumn::new("Description"),
        ];
        let mut table = Table::new().with_columns(columns);
        configure_shared_table(&mut table);
        table.add_row(vec![
            "ctx.today".into(),
            "String".into(),
            "Local date in ISO-8601 format.".into(),
        ]);
        table
    }

    #[test]
    fn table_contract_respects_below_140_width() {
        for width in [80_u32, 100, 120] {
            let term = styled_term(width);
            let table = build_sample_table();
            let out = render_table_within_contract(&table, &term);
            let max = max_line_visible_width(&out);
            assert!(
                max <= width as usize,
                "at width {width} output must fit; max={max}; output:\n{out}"
            );
        }
    }

    #[test]
    fn table_contract_respects_140_width_cap() {
        let term = styled_term(140);
        let table = build_sample_table();
        let out = render_table_within_contract(&table, &term);
        let max = max_line_visible_width(&out);
        assert!(
            max <= 140,
            "at width 140 output must fit within 140; max={max}; output:\n{out}"
        );
    }

    #[test]
    fn table_contract_caps_above_140_width() {
        let term = styled_term(200);
        let table = build_sample_table();
        let out = render_table_within_contract(&table, &term);
        let max = max_line_visible_width(&out);
        assert!(
            max <= 140,
            "at width 200 output must be capped to 140; max={max}; output:\n{out}"
        );
    }

    #[test]
    fn table_contract_margins_counted_within_cap() {
        let term = styled_term(200);
        let table = build_sample_table();
        let out = render_table_within_contract(&table, &term);
        // With 1ch margins, the leftmost character on every line should be a
        // leading space and the table should still fit inside 140.
        for line in out.lines() {
            let w = visible_width(line) as usize;
            assert!(
                w <= 140,
                "line exceeded 140 visible cells: {w}; line={line:?}"
            );
        }
    }

    #[test]
    fn table_contract_narrow_terminal_wraps_without_panic() {
        let term = styled_term(30);
        let mut table = Table::new().with_columns(vec![TableColumn::new("Note")]);
        configure_shared_table(&mut table);
        table.add_row(vec![
            "This is a long note made of short words so it can wrap to fit a very narrow terminal width".into(),
        ]);
        let out = render_table_within_contract(&table, &term);
        // The table should wrap the single column rather than error out.
        let max = max_line_visible_width(&out);
        assert!(
            max <= 30,
            "narrow output must fit within 30; max={max}; output:\n{out}"
        );
        assert!(!out.is_empty());
        assert!(
            out.lines().count() > 3,
            "content should wrap to multiple lines; output:\n{out}"
        );
    }

    // =====================================================================
    // Context column widths
    // =====================================================================

    #[test]
    fn context_column_widths_are_independent() {
        // Construct a fixture where property and type widths should differ.
        let properties = vec!["ctx.a", "ctx.very_long_property_name"];
        let types = vec!["String", "Csv"];
        let (prop_w, type_w) = context_column_widths(&properties, &types);

        assert!(
            prop_w > type_w,
            "property width should exceed type width in this fixture; prop={prop_w}, type={type_w}"
        );
        assert_eq!(prop_w, visible_width("ctx.very_long_property_name") as usize);
    }

    #[test]
    fn context_column_widths_include_headers() {
        let properties = vec!["ctx.x"];
        let types = vec!["x"];
        let (prop_w, type_w) = context_column_widths(&properties, &types);
        assert!(
            prop_w >= visible_width("Property") as usize,
            "property width must cover the header"
        );
        assert!(
            type_w >= visible_width("Type") as usize,
            "type width must cover the header"
        );
    }

    // =====================================================================
    // Function/capability first-column width
    // =====================================================================

    #[test]
    fn function_first_column_width_caps_at_40() {
        let sigs = vec![
            "short(x)",
            "this_is_an_extremely_long_function_name_with_many_arguments(a, b, c, d, e)",
        ];
        let w = function_first_column_width("Function", sigs.into_iter());
        assert_eq!(w, 40, "first-column width must be capped at 40ch");
    }

    #[test]
    fn function_first_column_width_uses_intrinsic_when_below_cap() {
        let sigs = vec!["min(a, b)", "max(a, b)"];
        let w = function_first_column_width("Function", sigs.into_iter());
        let expected = visible_width("min(a, b)") as usize;
        assert_eq!(w, expected.max(visible_width("Function") as usize).min(40));
    }

    #[test]
    fn function_first_column_long_signature_wraps_within_table_contract() {
        let long_sig = "this_is_a_very_long_function_name_that_exceeds_forty_characters(x, y, z)";
        let w = function_first_column_width("Capability", std::iter::once(long_sig));
        assert_eq!(w, 40);

        let mut table = Table::new().with_columns(vec![
            TableColumn::new("Capability")
                .with_min_width(w)
                .with_max_width(w),
            TableColumn::new("Description"),
        ]);
        configure_shared_table(&mut table);
        table.add_row(vec![
            long_sig.into(),
            "Does something useful".into(),
        ]);

        let term = styled_term(200);
        let out = render_table_within_contract(&table, &term);
        let max = max_line_visible_width(&out);
        assert!(
            max <= 140,
            "table with capped first column must fit contract; max={max}; output:\n{out}"
        );
        // Wrapping produces a multi-line cell.
        assert!(
            out.lines().count() > 3,
            "long signature must wrap to multiple lines; output:\n{out}"
        );
    }
}
