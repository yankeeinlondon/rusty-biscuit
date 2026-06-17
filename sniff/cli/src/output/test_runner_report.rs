//! Output rendering for `sniff repo test-runner`.
//!
//! Reports library-provided `TestRunnerUsage` values (runner + evidence
//! source). The CLI never re-detects; it only formats what the library
//! already collapsed via `aggregate_package_values`.

use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::components::table::{Table as TerminalTable, TableCellContent, TableColumn};
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::layout::Alignment;
use sniff::filesystem::repo::{AggregateResult, TestRunnerSource, TestRunnerUsage};
use sniff::programs::contract::CategoryEnum;
use sniff::programs::schema::ProgramMetadata;

/// Build the `--json` value for a test-runner aggregate result.
///
/// Shape:
/// - Singular: `{ "test_runner": <usage> }`
/// - Multiple: `{ "test_runners": [<usage>, ...] }`
/// - Empty:    `{ "test_runner": null }`
pub fn build_test_runner_json(result: &AggregateResult<TestRunnerUsage>) -> serde_json::Value {
    match result {
        AggregateResult::Singular(usage) => serde_json::json!({
            "test_runner": serde_json::to_value(usage).unwrap_or(serde_json::Value::Null),
        }),
        AggregateResult::Multiple(usages) => {
            let arr: Vec<serde_json::Value> = usages
                .iter()
                .map(|u| serde_json::to_value(u).unwrap_or(serde_json::Value::Null))
                .collect();
            serde_json::json!({ "test_runners": arr })
        }
        AggregateResult::Empty => serde_json::json!({ "test_runner": null }),
    }
}

/// Render the singular (single-runner) default styled text.
pub fn render_singular(usage: &TestRunnerUsage, verbose: u8, term: &Terminal) -> String {
    let info = usage.runner.info();
    let body = format!(
        "<b>{}</b> <dim>{}</dim>",
        info.display_name,
        render_source_suffix(&usage.source, term)
    );
    let mut out = Prose::new(body).render(term);
    if verbose > 0 {
        out.push_str(&render_evidence_detail(usage, term));
    }
    out.push('\n');
    out
}

/// Render the multiple-runner default styled text as a small table.
pub fn render_multiple(
    usages: &[TestRunnerUsage],
    scope_kind: &str,
    verbose: u8,
    term: &Terminal,
) -> String {
    let mut columns = vec![
        TableColumn::new("Runner"),
        TableColumn::new("Source")
            .with_alignment(Alignment::Left)
            .with_max_width(28),
    ];
    if verbose > 0 {
        columns.push(TableColumn::new("Ecosystem"));
        columns.push(TableColumn::new("Kind"));
    }

    let mut table = TerminalTable::new()
        .with_columns(columns)
        .prefer_cursor_alignment();

    for usage in usages {
        let info = usage.runner.info();
        let mut cells: Vec<TableCellContent> = vec![
            Prose::new(format!(
                r#"<a href="{}">{}</a>"#,
                info.website,
                info.display_name
            ))
            .render(term)
            .into(),
            render_source_cell(&usage.source, term).into(),
        ];
        if verbose > 0 {
            let spec =
                sniff::programs::test_runner_spec::TEST_RUNNER_SPEC[usage.runner.variant_index()];
            cells.push(format!("{:?}", spec.ecosystem).to_lowercase().into());
            cells.push(format!("{:?}", spec.kind).to_lowercase().into());
        }
        table.add_row(cells);
    }

    let mut out = table.display(term).to_string();
    // Append a dim scope hint so a reader knows whether the list is a
    // package-area/repo union or a single package.
    let hint = match scope_kind {
        "package-area" => " (across the current package-area)",
        "repo" => " (across all packages)",
        _ => "",
    };
    if !hint.is_empty() {
        let _ = std::fmt::Write::write_fmt(
            &mut out,
            format_args!(
                "\n{}",
                Prose::new(format!("<dim>distinct runners{hint}</dim>")).render(term)
            ),
        );
    }
    out.push('\n');
    out
}

fn render_source_cell(source: &TestRunnerSource, term: &Terminal) -> String {
    match source {
        TestRunnerSource::Config { filename } => Prose::new(format!(
            "<green>config</green> <dim>{filename}</dim>"
        ))
        .render(term),
        TestRunnerSource::Manifest { key } => Prose::new(format!(
            "<cyan>manifest</cyan> <dim>{key}</dim>"
        ))
        .render(term),
        TestRunnerSource::EcosystemDefault => {
            Prose::new("<dim>ecosystem default</dim>").render(term)
        }
        TestRunnerSource::Convention => Prose::new("<dim>convention</dim>").render(term),
    }
}

fn render_source_suffix(source: &TestRunnerSource, term: &Terminal) -> String {
    match source {
        TestRunnerSource::Config { filename } => {
            Prose::new(format!("via <green>config</green> <dim>{filename}</dim>")).render(term)
        }
        TestRunnerSource::Manifest { key } => {
            Prose::new(format!("via <cyan>manifest</cyan> <dim>{key}</dim>")).render(term)
        }
        TestRunnerSource::EcosystemDefault => {
            Prose::new("<dim>ecosystem default</dim>").render(term)
        }
        TestRunnerSource::Convention => Prose::new("<dim>convention</dim>").render(term),
    }
}

fn render_evidence_detail(usage: &TestRunnerUsage, term: &Terminal) -> String {
    let spec = sniff::programs::test_runner_spec::TEST_RUNNER_SPEC[usage.runner.variant_index()];
    format!(
        "\n{}",
        Prose::new(format!(
            "<dim>ecosystem {} · kind {} · invocation {}</dim>",
            format!("{:?}", spec.ecosystem).to_lowercase(),
            format!("{:?}", spec.kind).to_lowercase(),
            format!("{:?}", spec.invocation).to_lowercase(),
        ))
        .render(term)
    )
}
