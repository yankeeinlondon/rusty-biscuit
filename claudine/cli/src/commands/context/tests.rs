//! Tests for the `claudine context` reports.

use super::effects::group_effect_descriptors;
use super::expressions::{render_expression_table, show_examples};
use super::format::{
    format_context_value_type, format_example, format_effect_example, format_safety, format_value,
};
use super::*;
use crate::commands::context_render::{
    configure_shared_table, context_column_widths, function_first_column_width, inline_code_text,
    render_context_section, render_table_resilient, render_table_within_contract, report_column,
    TableLayout,
};
use biscuit_terminal::components::table::table::{Table, TableCellContent, TableColumn};
use biscuit_terminal::terminal::Terminal;

/// The default, expression, and side-effect reports are documentation
/// only: the spec forbids them from instantiating or probing an effect
/// engine or touching the network. Rather than asserting that by code
/// inspection, this drives the real render functions and checks Darkmatter's
/// process-wide instrumentation counters did not move — catching a
/// regression that constructs an `EffectEngine` (the gateway to
/// allowlist/config reads and host discovery) or attempts an HTTP request.
#[test]
fn metadata_reports_construct_no_engine_and_attempt_no_network() {
    use darkmatter::effects::{engine_build_count, network_attempt_count};

    let builds_before = engine_build_count();
    let network_before = network_attempt_count();

    render_default_report();
    render_expressions_report();
    effects::render_side_effects_report();

    assert_eq!(
        engine_build_count(),
        builds_before,
        "documentation-only reports must not construct an EffectEngine",
    );
    assert_eq!(
        network_attempt_count(),
        network_before,
        "documentation-only reports must attempt no network access",
    );
}

/// The values report must capture the runtime context exactly once per
/// invocation — never per section or per row. The capture seam lets us
/// assert that automatically rather than by code inspection.
#[test]
fn values_report_captures_context_exactly_once() {
    use std::cell::Cell;
    let calls = Cell::new(0u32);
    render_values_report_with(|| {
        calls.set(calls.get() + 1);
        ComposeContext::capture()
    });
    assert_eq!(
        calls.get(),
        1,
        "values report must invoke capture exactly once; got {} calls",
        calls.get(),
    );
}

/// The spec keeps `Property`, `Type`, and the final column in the default
/// and values reports at every supported width, relying on wrapping rather
/// than a Claudine-specific narrow layout. At the documented minimum
/// supported width (`MIN_SUPPORTED_REPORT_WIDTH`), `render_context_section`
/// must keep all three columns — even with the widest unbreakable property
/// name *and* the binding `NestedMarkdownList` type token — by letting the
/// columns wrap instead of dropping `Type` or emitting the planner's
/// width-error string inline. Representative content from every column must
/// survive too, not merely the headers.
#[test]
fn default_report_preserves_all_columns_at_minimum_supported_width() {
    use crate::commands::context_render::MIN_SUPPORTED_REPORT_WIDTH;

    // The widest real property; 41 unbreakable cells on spaces.
    let property_width = "ctx.current_package_area_has_staged_files".len();
    // The widest real type token — the constraint that fixes the floor.
    let type_width = "NestedMarkdownList".len();
    let term = Terminal::new_optimistic(MIN_SUPPORTED_REPORT_WIDTH);

    let output = render_context_section(
        &term,
        property_width,
        type_width,
        "Description",
        |table| {
            table.add_row(vec![
                "ctx.current_package_area_has_staged_files".into(),
                "Boolean".into(),
                "Whether the current package area has staged files.".into(),
            ]);
            table.add_row(vec![
                "ctx.docs_outline".into(),
                "NestedMarkdownList".into(),
                "Nested outline of the in-scope docs.".into(),
            ]);
        },
    );

    assert!(
        !output.contains("could not be rendered"),
        "render at the minimum supported width must wrap content rather than \
         emit the width-error string; output was:\n{output}",
    );
    // All three columns survive — no alternate narrow layout drops Type.
    for header in ["Property", "Type", "Description"] {
        assert!(
            output.contains(header),
            "minimum-width report must keep the `{header}` column; output was:\n{output}",
        );
    }
    // Representative content from each column survives the wrap (tokens may
    // wrap across rows, so assert fragments that stay intact).
    for fragment in ["ctx.", "Boolean", "NestedMarkdownList", "staged"] {
        assert!(
            output.contains(fragment),
            "minimum-width report must retain `{fragment}` content; output was:\n{output}",
        );
    }
    // Output fits the terminal width without intentional overflow.
    let max = output
        .lines()
        .map(|l| biscuit_terminal::utils::block_constraint::visible_width(l) as usize)
        .max()
        .unwrap_or(0);
    assert!(
        max <= MIN_SUPPORTED_REPORT_WIDTH as usize,
        "minimum-width report must fit within the terminal width; max={max}; output:\n{output}",
    );
}

/// Every context section must use the same computed Property/Type widths.
/// The widths are derived from the complete catalog and reused per section.
#[test]
fn shared_property_type_widths_across_sections() {
    let term = Terminal::new_optimistic(200);
    let groups = group_context_descriptors();

    let all_properties: Vec<String> = groups
        .iter()
        .flat_map(|(_, vars)| vars.iter().map(|v| format!("ctx.{}", v.name)))
        .collect();
    let all_types: Vec<String> = groups
        .iter()
        .flat_map(|(_, vars)| {
            vars.iter()
                .map(|v| format_context_value_type(&v.display_type, &term))
        })
        .collect();

    let property_labels: Vec<&str> = all_properties.iter().map(|s| s.as_str()).collect();
    let type_labels: Vec<&str> = all_types.iter().map(|s| s.as_str()).collect();
    let (property_width, type_width) =
        context_column_widths(&property_labels, &type_labels);

    // Widths must be positive and independently computed.
    assert!(
        property_width > 0,
        "Property width must be positive; got {property_width}"
    );
    assert!(
        type_width > 0,
        "Type width must be positive; got {type_width}"
    );

    // Verify the widths cover the longest entries in the catalog.
    let max_property = property_labels
        .iter()
        .map(|s| biscuit_terminal::utils::block_constraint::visible_width(s) as usize)
        .max()
        .unwrap_or(0);
    let max_type = type_labels
        .iter()
        .map(|s| biscuit_terminal::utils::block_constraint::visible_width(s) as usize)
        .max()
        .unwrap_or(0);

    assert!(
        property_width >= max_property,
        "Property width ({property_width}) must cover longest property ({max_property})"
    );
    assert!(
        type_width >= max_type,
        "Type width ({type_width}) must cover longest type ({max_type})"
    );
}

/// The expression report groups the complete function catalog by metadata
/// category — each category emitted exactly once, even though the catalog is
/// physically laid out by implementation grouping (so `Math` and `Collection`
/// appear in non-adjacent runs). Within each category the signatures must
/// follow the descriptors' stable `order`, not catalog position.
#[test]
fn expression_groups_consolidate_categories_in_metadata_order() {
    use std::collections::HashSet;

    let groups = group_expression_descriptors();

    let mut seen = HashSet::new();
    for (category, _) in &groups {
        assert!(
            seen.insert(*category),
            "category `{category}` emitted more than once",
        );
    }

    for (category, functions) in &groups {
        let orders: Vec<usize> = functions.iter().map(|f| f.order).collect();
        let mut expected = orders.clone();
        expected.sort_unstable();
        assert_eq!(
            orders, expected,
            "category `{category}` signatures must follow metadata order; got {orders:?}",
        );
    }
}

/// The expression report's `Example` column follows the documented "where
/// layout permits" contract: it is dropped below `EXAMPLE_COLUMN_MIN_WIDTH`
/// (mirroring the side-effect report) and present at or above it. The guard
/// is what keeps the narrow report short enough to stay within the L2
/// capture pane. Drive `render_expression_table` with a real example cell and
/// assert the `Example` header's presence flips at the threshold.
#[test]
fn expression_table_drops_example_column_below_threshold() {
    use crate::commands::context_render::EXAMPLE_COLUMN_MIN_WIDTH;

    let example_cell: TableCellContent = "min(1, 2) → 1".into();
    let build_rows = || {
        vec![(
            "min(a, b)".into(),
            "smaller of two values".into(),
            example_cell.clone(),
        )]
    };

    // At the threshold the Example column (header and content) is present.
    let at = Terminal::new_optimistic(EXAMPLE_COLUMN_MIN_WIDTH);
    let wide = render_expression_table(&at, "Function", 9, "Description", build_rows());
    assert!(
        wide.contains("Example"),
        "Example column must be present at the threshold width; output:\n{wide}",
    );

    // One cell below the threshold the column is dropped entirely — no empty
    // cells, no header.
    let below = Terminal::new_optimistic(EXAMPLE_COLUMN_MIN_WIDTH - 1);
    let narrow = render_expression_table(&below, "Function", 9, "Description", build_rows());
    assert!(
        !narrow.contains("Example"),
        "Example column must be dropped below the threshold width; output:\n{narrow}",
    );
    // The remaining columns survive.
    for header in ["Function", "Description"] {
        assert!(
            narrow.contains(header),
            "narrow expression table must keep the `{header}` column; output:\n{narrow}",
        );
    }
}

/// A long `Description` line must not starve the trailing `Example` column.
/// The shared planner grants surplus width greedily in column order, so
/// before the floor was introduced a group with one very long description
/// (the `String Mutations` `without_date` row) collapsed `Example` to a
/// one-token-wide sliver. With the floor, a representative example stays on
/// a single rendered line.
#[test]
fn expression_table_example_column_survives_long_description() {
    // Plain terminal so the example renders verbatim (no inline-code SGR).
    let mut term = Terminal::new_optimistic(140);
    term.color_depth =
        biscuit_terminal::discovery::detection::ColorDepth::None;
    term.is_tty = false;

    let example = "kebab_case(\"Hello World\") → hello-world";
    let long_description =
        "Removes substrings that are real YYYY-MM-DD calendar dates, leaving surrounding text untouched.";
    let rows = vec![
        (
            "kebab_case(x)".into(),
            "Converts a string to kebab-case.".into(),
            example.into(),
        ),
        (
            "without_date(string)".into(),
            long_description.into(),
            "without_date(\"Note 2024-06-15\") → Note".into(),
        ),
    ];
    let out = render_expression_table(&term, "Function", 20, "Description", rows);

    assert!(
        out.lines().any(|line| line.contains(example)),
        "example must render contiguously on one line, not wrapped to a sliver; output:\n{out}",
    );
}

/// `show_examples` is the single threshold both the expression sub-tables and
/// the side-effect report consult, so its boundary must hold exactly at
/// `EXAMPLE_COLUMN_MIN_WIDTH`.
#[test]
fn show_examples_threshold_is_inclusive() {
    use crate::commands::context_render::EXAMPLE_COLUMN_MIN_WIDTH;

    assert!(!show_examples(&Terminal::new_optimistic(
        EXAMPLE_COLUMN_MIN_WIDTH - 1
    )));
    assert!(show_examples(&Terminal::new_optimistic(EXAMPLE_COLUMN_MIN_WIDTH)));
    assert!(show_examples(&Terminal::new_optimistic(
        EXAMPLE_COLUMN_MIN_WIDTH + 1
    )));
}

/// Scalar array elements render with plain values — strings without JSON
/// quotes — while nested arrays/objects keep compact JSON serialization.
#[test]
fn format_value_renders_arrays_plainly() {
    let term = Terminal::new_optimistic(200);

    assert_eq!(
        format_value(&serde_json::json!(["alpha", "beta"]), &term),
        "alpha, beta",
    );
    assert_eq!(
        format_value(&serde_json::json!([1, 2, 3]), &term),
        "1, 2, 3",
    );
    assert_eq!(
        format_value(&serde_json::json!([true, false]), &term),
        "true, false",
    );
    // Nested structured elements keep compact JSON.
    assert_eq!(
        format_value(&serde_json::json!([{"k": 1}, ["x"]]), &term),
        r#"{"k":1}, ["x"]"#,
    );
}

#[test]
fn format_example_renders_tabs_as_visible_literals() {
    let mut term = Terminal::new_optimistic(140);
    term.color_depth = biscuit_terminal::discovery::detection::ColorDepth::None;
    term.is_tty = false;
    let descriptor = darkmatter::markdown::compose::expression::expression_function_descriptors()
        .iter()
        .find(|descriptor| descriptor.signature == "as_tsv(list)")
        .expect("as_tsv descriptor");

    let rendered = format_example(descriptor.example.as_ref(), &term);
    assert_eq!(rendered, r#"as_tsv(items) → 1\t2\t3"#);
    assert!(!rendered.contains('\t'));
}

/// Verify that expression and side-effect reports render without
/// exceeding the 140ch contract at wide terminal widths.
#[test]
fn reports_render_within_140ch_contract_at_wide_terminals() {
    use biscuit_terminal::utils::block_constraint::visible_width;

    let term = Terminal::new_optimistic(200);

    // Build an expression-function table similar to the real report.
    let func_groups = group_expression_descriptors();
    let all_sigs: Vec<String> = func_groups
        .iter()
        .flat_map(|(_, funcs)| funcs.iter().map(|f| f.signature.to_string()))
        .collect();
    let sig_refs: Vec<&str> = all_sigs.iter().map(|s| s.as_str()).collect();
    let func_width = function_first_column_width("Function", sig_refs.into_iter());

    for (category, functions) in &func_groups {
        let columns = vec![
            TableColumn::new("Function")
                .with_min_width(func_width)
                .with_max_width(func_width),
            TableColumn::new("Description"),
            TableColumn::new("Example"),
        ];
        let mut table = Table::new().with_columns(columns);
        configure_shared_table(&mut table);

        for func in functions {
            table.add_row(vec![
                inline_code_text(func.signature, &term).into(),
                inline_code_text(func.description, &term).into(),
                format_example(func.example.as_ref(), &term).into(),
            ]);
        }

        let out = render_table_within_contract(&table, &term);
        let max = out.lines().map(|l| visible_width(l) as usize).max().unwrap_or(0);
        assert!(
            max <= 140,
            "expression table for '{category}' exceeded 140ch (max={max})"
        );
    }

    // Build a side-effect table similar to the real report.
    let effect_groups = group_effect_descriptors();
    let all_effect_sigs: Vec<String> = effect_groups
        .iter()
        .flat_map(|(_, effects)| effects.iter().map(|e| e.signature.to_string()))
        .collect();
    let effect_sig_refs: Vec<&str> = all_effect_sigs.iter().map(|s| s.as_str()).collect();
    let cap_width = function_first_column_width("Capability", effect_sig_refs.into_iter());

    for (category, effects) in &effect_groups {
        let out = render_table_resilient(&term, |layout| {
            let mut capability = report_column("Capability");
            if layout == TableLayout::Pinned {
                capability = capability.with_min_width(cap_width).with_max_width(cap_width);
            }
            let mut table = Table::new().with_columns(vec![
                capability,
                report_column("Description"),
                report_column("Example"),
                report_column("Safety"),
            ]);
            configure_shared_table(&mut table);

            for effect in effects {
                table.add_row(vec![
                    inline_code_text(effect.signature, &term).into(),
                    inline_code_text(effect.description, &term).into(),
                    format_effect_example(effect.example.as_ref(), &term).into(),
                    format_safety(effect.safety, &term).into(),
                ]);
            }
            table
        });
        let max = out.lines().map(|l| visible_width(l) as usize).max().unwrap_or(0);
        assert!(
            max <= 140,
            "side-effect table for '{category}' exceeded 140ch (max={max})"
        );
    }
}
