use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::components::table::{Table, TableCellContent, TableColumn};
use biscuit_terminal::components::table::types::ColumnType;
use biscuit_terminal::components::two_column::TwoColumn;
use biscuit_terminal::terminal::Terminal;

/// A presentation row for the `sets` command output.
#[derive(Debug, Clone)]
pub struct SetRow {
    pub prefix: String,
    pub title: String,
    pub total: Option<usize>,
    pub cached: usize,
}

/// Base URL of the Iconify icon-set browser. Each set has a page keyed on its
/// prefix, e.g. `mdi` → `https://icon-sets.iconify.design/mdi`.
const ICONIFY_SET_BASE_URL: &str = "https://icon-sets.iconify.design";

const SPLIT_GAP: u32 = 3;
const MIN_TABLE_WIDTH: u32 = 48;
const MIN_SPLIT_WIDTH: u32 = MIN_TABLE_WIDTH * 2 + SPLIT_GAP;
const TABLE_OVERHEAD: u32 = 4; // top border + header + separator + bottom border

/// Builds the Set-name cell, linking the title to its Iconify catalog page.
///
/// Only terminals that advertise OSC8 support get the hyperlink: the link's URL
/// lives in an invisible escape, so the cell's visible width stays at the label.
/// Without OSC8 the render tree would degrade the link to a `[label](url)`
/// reference whose long, barely-breakable URL inflates the column's minimum
/// width and can make the table unrenderable in narrow terminals — so those
/// terminals get the plain title instead.
fn title_cell(title: &str, prefix: &str, term: &Terminal) -> TableCellContent {
    if !term.osc_link_support {
        return title.to_string().into();
    }
    let url = format!("{ICONIFY_SET_BASE_URL}/{prefix}");
    Prose::new(format!(
        "<blue><a href={href}>{label}</a></blue>",
        href = Prose::quoted_attr(&url),
        label = Prose::escape_text(title),
    ))
    .into()
}

/// Builds a striped four-column table from rows.
pub fn build_table(rows: &[SetRow], term: &Terminal) -> Table {
    let data: Vec<Vec<TableCellContent>> = rows
        .iter()
        .map(|r| {
            let total_cell = match r.total {
                Some(n) => TableCellContent::Integer(n as i64),
                None => TableCellContent::Text("Unknown".into()),
            };
            vec![
                title_cell(&r.title, &r.prefix, term),
                r.prefix.clone().into(),
                total_cell,
                TableCellContent::Integer(r.cached as i64),
            ]
        })
        .collect();

    Table::new()
        .with_columns(vec![
            TableColumn::new("Set")
                .with_max_width(30)
                .with_word_wrap(biscuit_terminal::utils::wrap_policy::WordWrap::WrapProse(None, None)),
            TableColumn::new("Prefix")
                .with_max_width(20)
                .with_word_wrap(biscuit_terminal::utils::wrap_policy::WordWrap::WrapProse(None, None)),
            TableColumn::new("Total").with_type(ColumnType::Integer),
            TableColumn::new("Cached").with_type(ColumnType::Integer),
        ])
        .with_data(data)
        .alternate_background_color()
}

/// Returns the number of data rows that fit in one table for the given height.
pub fn rows_per_table(term_height: u32) -> usize {
    let available = term_height.saturating_sub(TABLE_OVERHEAD);
    available.max(1) as usize
}

/// The chosen layout for rendering set rows.
#[allow(clippy::large_enum_variant)]
pub enum Layout {
    Single(Table),
    Split(Table, Table),
}

/// Chooses a layout based on row count and terminal dimensions.
pub fn choose_layout(rows: &[SetRow], term: &Terminal) -> Layout {
    let per_table = rows_per_table(term.height());

    if rows.len() <= per_table {
        Layout::Single(build_table(rows, term))
    } else if term.width() >= MIN_SPLIT_WIDTH {
        let midpoint = rows.len().div_ceil(2);
        let (left, right) = rows.split_at(midpoint);
        Layout::Split(build_table(left, term), build_table(right, term))
    } else {
        Layout::Single(build_table(rows, term))
    }
}

/// Renders the set rows through the chosen layout.
pub fn render_sets(rows: &[SetRow], term: &Terminal) -> String {
    match choose_layout(rows, term) {
        Layout::Single(table) => table.render(term),
        Layout::Split(left, right) => {
            TwoColumn::new(left, right)
                .with_left_percent(0.5)
                .with_gap(SPLIT_GAP)
                .render(term)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_rows(count: usize) -> Vec<SetRow> {
        (0..count)
            .map(|i| SetRow {
                prefix: format!("p{}", i),
                title: format!("Set {}", i),
                total: Some(i * 1000),
                cached: i,
            })
            .collect()
    }

    fn term(width: u32, height: u32) -> Terminal {
        Terminal::builder()
            .width(width)
            .height(height)
            .build()
    }

    #[test]
    fn rows_per_table_accounts_for_overhead() {
        assert_eq!(rows_per_table(4), 1); // clamped to minimum
        assert_eq!(rows_per_table(5), 1);
        assert_eq!(rows_per_table(8), 4);
        assert_eq!(rows_per_table(10), 6);
    }

    #[test]
    fn rows_per_table_clamps_to_at_least_one() {
        assert_eq!(rows_per_table(0), 1);
        assert_eq!(rows_per_table(1), 1);
        assert_eq!(rows_per_table(3), 1);
    }

    #[test]
    fn choose_layout_single_when_rows_fit() {
        let rows = test_rows(3);
        let t = term(80, 10);
        match choose_layout(&rows, &t) {
            Layout::Single(_) => {}
            Layout::Split(_, _) => panic!("expected single table when rows fit"),
        }
    }

    #[test]
    fn choose_layout_single_when_narrow() {
        let rows = test_rows(20);
        // Height allows 6 rows, but width is too narrow for split
        let t = term(60, 10);
        match choose_layout(&rows, &t) {
            Layout::Single(_) => {}
            Layout::Split(_, _) => panic!("expected single table when narrow"),
        }
    }

    #[test]
    fn choose_layout_split_when_wide_and_tall() {
        let rows = test_rows(20);
        // Height allows 6 rows per table, 20 rows need split
        // Width is wide enough for split (>= 99)
        let t = term(120, 10);
        match choose_layout(&rows, &t) {
            Layout::Split(_, _) => {}
            Layout::Single(_) => panic!("expected split when wide and too many rows"),
        }
    }

    #[test]
    fn split_is_balanced() {
        let rows = test_rows(5);
        let t = term(120, 4); // 4 height -> 1 row per table, 5 rows need split
        match choose_layout(&rows, &t) {
            Layout::Split(left, right) => {
                // We need to inspect the tables to count rows.
                // Table doesn't expose row count directly, but we can render and check.
                let left_rendered = left.render_optimistic(Some(60));
                let right_rendered = right.render_optimistic(Some(60));
                // Both tables have headers, so both should contain "Set"
                assert!(left_rendered.contains("Set"));
                assert!(right_rendered.contains("Set"));
                // Left should have 3 data rows (p0, p1, p2)
                assert!(left_rendered.contains("p0"));
                assert!(left_rendered.contains("p1"));
                assert!(left_rendered.contains("p2"));
                // Right should have 2 data rows (p3, p4)
                assert!(right_rendered.contains("p3"));
                assert!(right_rendered.contains("p4"));
            }
            Layout::Single(_) => panic!("expected split"),
        }
    }

    #[test]
    fn split_differs_by_at_most_one() {
        for count in [2, 3, 4, 5, 6, 7, 8, 9, 10] {
            let rows = test_rows(count);
            let t = term(120, 4); // force split
            match choose_layout(&rows, &t) {
                Layout::Split(left, right) => {
                    let left_out = left.render_optimistic(Some(60));
                    let right_out = right.render_optimistic(Some(60));
                    // Count prefix occurrences in each table (data rows)
                    let left_count = (0..count)
                        .filter(|i| left_out.contains(&format!("p{}", i)))
                        .count();
                    let right_count = (0..count)
                        .filter(|i| right_out.contains(&format!("p{}", i)))
                        .count();
                    let diff = left_count.abs_diff(right_count);
                    assert!(
                        diff <= 1,
                        "count={}: left={} right={} diff={}",
                        count, left_count, right_count, diff
                    );
                    assert_eq!(left_count + right_count, count);
                }
                Layout::Single(_) => {
                    // For count=2 with height=4, rows_per_table=1, so 2 > 1 means split
                    // But width=120 >= 99, so should split
                    panic!("expected split for count={}", count);
                }
            }
        }
    }

    #[test]
    fn rendered_output_contains_headers() {
        let rows = test_rows(2);
        let t = term(80, 10);
        let output = render_sets(&rows, &t);
        assert!(output.contains("Set"));
        assert!(output.contains("Prefix"));
        assert!(output.contains("Total"));
        assert!(output.contains("Cached"));
    }

    #[test]
    fn rendered_output_shows_unknown_for_missing_total() {
        let rows = vec![SetRow {
            prefix: "test".into(),
            title: "Test Set".into(),
            total: None,
            cached: 0,
        }];
        let t = term(80, 10);
        let output = render_sets(&rows, &t);
        assert!(output.contains("Unknown"));
    }

    #[test]
    fn rendered_output_shows_thousands_separator() {
        let rows = vec![SetRow {
            prefix: "test".into(),
            title: "Test Set".into(),
            total: Some(1_234_567),
            cached: 0,
        }];
        let t = term(80, 10);
        let output = render_sets(&rows, &t);
        assert!(output.contains("1,234,567"), "expected thousands separator; got: {}", output);
    }

    #[test]
    fn long_prefix_wraps_within_column_keeping_counts() {
        // A prefix longer than the Prefix column's 20-char max must wrap onto a
        // continuation line at a break character rather than push the count
        // columns off screen. The full Total stays visible on the row.
        let rows = vec![SetRow {
            prefix: "streamlineplump-emojibits".into(),
            title: "Stream".into(),
            total: Some(1_234_567),
            cached: 7,
        }];
        let t = term(100, 30);
        let output = render_sets(&rows, &t);
        let lines: Vec<&str> = output.lines().collect();

        // The full count value renders in full, not clipped by the wide prefix.
        assert!(
            output.contains("1,234,567"),
            "full Total must remain visible alongside a wrapped prefix; got:\n{output}"
        );

        // The prefix tail lands on a continuation line that neither repeats the
        // leading prefix segment nor the title.
        let continuation = lines
            .iter()
            .find(|l| l.contains("emojibits") && !l.contains("streamlineplump"))
            .unwrap_or_else(|| {
                panic!("prefix did not wrap onto a continuation line:\n{output}")
            });
        assert!(
            !continuation.contains("Stream"),
            "continuation line must not repeat the title: {continuation:?}"
        );
    }

    #[test]
    fn no_raw_prose_markup_in_output() {
        let rows = test_rows(3);
        let t = term(80, 10);
        let output = render_sets(&rows, &t);
        // Prose markup uses <tag> syntax; the table should not contain raw Prose.
        assert!(
            !output.contains('<') || !output.contains('>'),
            "output should not contain raw Prose markup tags; got: {}",
            output
        );
    }

    #[test]
    fn set_title_links_to_iconify_page() {
        // The Set name links to its Iconify catalog page, which lives at a
        // route keyed on the set prefix.
        let rows = vec![SetRow {
            prefix: "mdi".into(),
            title: "Material Design Icons".into(),
            total: Some(7447),
            cached: 0,
        }];
        let t = Terminal::builder()
            .width(120)
            .height(20)
            .is_tty(true)
            .osc_link_support(true)
            .color_depth(biscuit_terminal::discovery::detection::ColorDepth::TrueColor)
            .build();
        let output = render_sets(&rows, &t);
        assert!(
            output.contains("https://icon-sets.iconify.design/mdi"),
            "expected an OSC8 link to the set's Iconify page; got:\n{output}"
        );
    }

    #[test]
    fn set_title_never_leaks_raw_prose_markup() {
        // Whatever the terminal capabilities, the styled title must lower to
        // escapes or a markdown fallback — never raw `<blue>`/`<a href>` tags.
        let rows = vec![SetRow {
            prefix: "mdi".into(),
            title: "Material Design Icons".into(),
            total: Some(7447),
            cached: 0,
        }];
        let t = Terminal::builder()
            .width(120)
            .height(20)
            .is_tty(false)
            .osc_link_support(false)
            .build();
        let output = render_sets(&rows, &t);
        assert!(
            !output.contains("<blue>") && !output.contains("<a href"),
            "output must not contain raw Prose markup; got:\n{output}"
        );
    }

    #[test]
    fn title_with_special_chars_is_escaped_through_prose() {
        // Prose grammar treats `<` and `>` as tag delimiters and `&`/`"` as
        // markup-friendly text — a title containing any of them must be
        // escape-quoted by `Prose::escape_text`/`Prose::quoted_attr` so the
        // render tree never sees them as raw markup. The hyperlink itself
        // still wraps the (now-escaped) label.
        let rows = vec![SetRow {
            prefix: "mdi".into(),
            title: "Set <with> & \"special\" chars".into(),
            total: Some(7447),
            cached: 0,
        }];
        let t = Terminal::builder()
            .width(120)
            .height(20)
            .is_tty(true)
            .osc_link_support(true)
            .color_depth(biscuit_terminal::discovery::detection::ColorDepth::TrueColor)
            .build();
        let output = render_sets(&rows, &t);

        assert!(
            !output.contains("<blue>") && !output.contains("<a href"),
            "raw Prose tags must not leak; got:\n{output}"
        );

        assert!(
            output.contains("https://icon-sets.iconify.design/mdi"),
            "OSC8 link to the set's Iconify page must still be emitted; got:\n{output}"
        );

        assert!(
            output.contains("Set <with> & \"special\" chars"),
            "title text must survive the escape pipeline unaltered; got:\n{output}"
        );
    }

    #[test]
    fn set_title_emits_blue_sgr_in_osc8_output() {
        // The spec calls for `<blue><a href=…>{set-name}</a></blue>`, which
        // lowers to SGR 34 around the link. This pins the wiring — if the
        // Prose pipeline ever drops the blue wrapper, the assertion catches
        // it before a user does.
        let rows = vec![SetRow {
            prefix: "mdi".into(),
            title: "Material Design Icons".into(),
            total: Some(7447),
            cached: 0,
        }];
        let t = Terminal::builder()
            .width(120)
            .height(20)
            .is_tty(true)
            .osc_link_support(true)
            .color_depth(biscuit_terminal::discovery::detection::ColorDepth::TrueColor)
            .build();
        let output = render_sets(&rows, &t);
        assert!(
            output.contains("\x1b[34m"),
            "expected the blue basic SGR (ESC[34m) in the OSC8 path; got:\n{output}"
        );
    }

    #[test]
    fn wide_and_tall_uses_single_table() {
        let rows = test_rows(5);
        let t = term(120, 20); // 20 height -> 16 rows per table, 5 fits
        match choose_layout(&rows, &t) {
            Layout::Single(_) => {}
            Layout::Split(_, _) => panic!("expected single table when all rows fit"),
        }
    }
}
