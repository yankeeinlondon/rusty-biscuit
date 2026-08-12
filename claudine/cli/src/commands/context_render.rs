use biscuit_terminal::components::list::UnorderedList;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::components::table::table::{Table, TableColumn};
use biscuit_terminal::discovery::detection::ColorDepth;
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::block_constraint::visible_width;
use biscuit_terminal::utils::layout::{Alignment, Edges, Length, TargetValue, Width, WordWrap};

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

/// Minimum terminal width, in visible cells, at which the metadata reports keep
/// their optional `Example` column.
///
/// The `Example` column is the widest optional content in the expression and
/// side-effect reports. At narrow widths the four-column layout cannot satisfy
/// the table planner without overflow, so the reports drop `Example` (the
/// documented "where layout permits" contract) and render the remaining columns
/// only. At or above this width the column is shown.
#[allow(dead_code)]
pub const EXAMPLE_COLUMN_MIN_WIDTH: u32 = 70;

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

/// Middle-elide a single-token value (e.g. a filesystem path) to fit `budget`
/// visible columns, inserting `…` so both the head and the meaningful tail
/// survive: `/Users/ken/.claudine/worktrees/rusty-biscuit/claudine` →
/// `/Users/ken/…/rusty-biscuit/claudine`.
///
/// This is the values-report defense against a wrapped path reading as a
/// complete (parent) path: the report-content column *must* keep path-separator
/// break characters so tables still render at the minimum supported width, but
/// breaking a path at `/` produces a line ending in `/` that looks like a real
/// directory. Pre-eliding a path that would otherwise wrap keeps it on one line
/// and unambiguous. `budget` should be the resolved content-column width; values
/// at or under it are returned unchanged.
#[allow(dead_code)]
pub fn middle_elide(value: &str, budget: usize) -> String {
    let width = visible_width(value) as usize;
    if budget < 3 || width <= budget {
        return value.to_string();
    }
    let chars: Vec<char> = value.chars().collect();
    // Reserve one column for the ellipsis; split the remainder head/tail,
    // biasing the tail (the distinctive leaf) one char larger on odd budgets.
    let keep = budget - 1;
    let head_len = keep / 2;
    let tail_len = keep - head_len;
    let head: String = chars[..head_len].iter().collect();
    let tail: String = chars[chars.len() - tail_len..].iter().collect();
    format!("{head}…{tail}")
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
/// - `width: 100%` so every report table fills the available width to the right
///   margin (the last column absorbs any slack), rather than hugging content —
///   tables with and without wrapped cells then share one right edge
/// - Total rendered width never exceeds `min(terminal width, 140)` visible
///   cells, counting margins, borders, separators, and content
/// - Below 140ch the table uses the available width without overflow
#[allow(dead_code)]
pub fn configure_shared_table(table: &mut Table) {
    table.layout_mut().margin = Edges::x(Length::ch(1));
    table.layout_mut().width = Width::Fixed(TargetValue::universal(Length::Percent(100.0)));
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
pub const TABLE_WIDTH_ERROR_SENTINEL: &str = "could not be rendered";

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

/// `Example`-column floor: the intrinsic width of the rendered example cells
/// (and the `Example` header), capped at 40ch.
///
/// Used as a `min_width` on the trailing `Example` column so a long
/// `Description` line cannot starve it. The shared `Table` planner grants its
/// surplus width to shrinkable columns greedily in column order (see
/// `resolve_width_plan`), so without a floor a group whose widest description
/// happens to consume the surplus leaves `Example` at its break-minimum — a
/// one-token-wide sliver that wraps almost vertically. Reserving the floor
/// before the surplus pass keeps the two flexible columns balanced. At widths
/// too narrow to honor the floor, [`render_table_resilient`] drops the pins and
/// falls back to the break-aware [`TableLayout::Wrap`] layout.
#[allow(dead_code)]
pub fn example_column_floor<'a>(examples: impl Iterator<Item = &'a str>) -> usize {
    examples
        .map(|s| visible_width(s) as usize)
        .max()
        .unwrap_or(0)
        .max(visible_width("Example") as usize)
        .min(40)
}

#[cfg(test)]
mod tests;
