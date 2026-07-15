use super::*;
use biscuit_terminal::components::table::table::{Table, TableCellContent, TableColumn};
use biscuit_terminal::utils::layout::Alignment;

fn styled_term(width: u32) -> Terminal {
    Terminal::new_optimistic(width)
}

#[test]
fn middle_elide_keeps_head_and_tail_and_inserts_ellipsis() {
    let path = "/Users/ken/.claudine/worktrees/rusty-biscuit/claudine";
    let out = middle_elide(path, 24);
    assert_eq!(visible_width(&out) as usize, 24, "elided to exact budget: {out:?}");
    assert!(out.contains('…'), "has ellipsis: {out:?}");
    assert!(out.starts_with("/Users/"), "keeps head: {out:?}");
    assert!(out.ends_with("claudine"), "keeps the leaf: {out:?}");
    // The deceptive failure mode is a value that reads as a complete path;
    // the ellipsis guarantees it cannot.
    assert!(!out.ends_with('/'), "never ends in a bare separator: {out:?}");
}

#[test]
fn middle_elide_returns_short_values_unchanged() {
    assert_eq!(middle_elide("/tmp/x", 24), "/tmp/x");
    // Degenerate budgets are a no-op rather than a panic.
    assert_eq!(middle_elide("/a/very/long/path", 2), "/a/very/long/path");
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

// =====================================================================
// Example-column floor
// =====================================================================

#[test]
fn example_column_floor_uses_intrinsic_width_and_includes_header() {
    // The widest example governs the floor; the `Example` header still
    // participates so a column of tiny examples never falls below it.
    let examples = vec!["a → 1", "kebab_case(\"Hello World\") → hello-world"];
    let w = example_column_floor(examples.into_iter());
    assert_eq!(w, visible_width("kebab_case(\"Hello World\") → hello-world") as usize);

    let tiny = vec!["x", "y"];
    assert_eq!(
        example_column_floor(tiny.into_iter()),
        visible_width("Example") as usize,
        "floor never drops below the header width",
    );
}

#[test]
fn example_column_floor_caps_at_40() {
    let huge = "an_extremely_long_invocation(with, many, arguments) → and_a_long_result_value";
    assert_eq!(example_column_floor(std::iter::once(huge)), 40);
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
