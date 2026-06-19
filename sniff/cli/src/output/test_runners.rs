//! Test-runner section output formatting (table and JSON).
//!
//! Distinct from `programs.rs` because test runners carry an `Availability`
//! discriminator (`installed`, `local`, `via_parent`, `not_found`) rather
//! than the bare `installed: bool` used by the other 8 categories.

use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::components::table::{Table as TerminalTable, TableCellContent, TableColumn};
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::layout::Alignment;
use sniff::programs::ProgramMetadata;
use sniff::programs::test_runner::InstalledTestRunners;
use sniff::programs::test_runner_spec::Availability;

/// Mode for emitting the optional `sniff software test-runners` hint.
///
/// The hint is the same prose in both modes; the discriminator exists so the
/// caller can target stdout/stderr cleanly without re-querying the CLI flags.
#[derive(Debug, Clone, Copy)]
pub enum HintMode {
    /// Render for stderr (default text mode). The hint is suppressed under
    /// `--plain` and `--json` by the caller.
    Stderr,
}

/// Build a serde JSON value for the test-runner category.
///
/// Output shape mirrors the other 8 categories (a map keyed by `serde_key`)
/// but each entry carries `availability` instead of `installed`.
pub fn build_test_runners_json(
    runners: &InstalledTestRunners,
) -> serde_json::Result<serde_json::Value> {
    serde_json::to_value(runners)
}

/// Render the test-runner table for terminal output.
pub fn render_test_runners_markdown(runners: &InstalledTestRunners, verbose: u8) -> String {
    let term = Terminal::default();

    let mut columns = vec![
        TableColumn::new("Name"),
        TableColumn::new("Availability")
            .with_alignment(Alignment::Center)
            .with_uniform_alignment(true)
            .with_max_width(20),
    ];

    if verbose > 0 {
        columns.push(TableColumn::new("Ecosystem"));
        columns.push(TableColumn::new("Kind"));
        columns.push(TableColumn::new("Binary"));
        columns.push(TableColumn::new("Path").with_max_width(30));
    }
    if verbose > 1 {
        columns.push(TableColumn::new("Root").with_max_width(20));
    }

    columns.push(TableColumn::new("Description").with_max_width(30));

    let mut table = TerminalTable::new()
        .with_columns(columns)
        .prefer_cursor_alignment();

    for entry in &runners.entries {
        let info = entry.runner.info();
        let mut cells: Vec<TableCellContent> = vec![
            linked_name_cell(info.display_name, info.website, &term).into(),
            availability_cell(&entry.availability, &term).into(),
        ];

        if verbose > 0 {
            cells.push(format!("{:?}", entry.spec.ecosystem).to_lowercase().into());
            cells.push(format!("{:?}", entry.spec.kind).to_lowercase().into());
            cells.push(info.binary_name.to_string().into());
            cells.push(
                entry
                    .path()
                    .map(|p| p.display().to_string())
                    .unwrap_or_default()
                    .into(),
            );
        }
        if verbose > 1 {
            if let Availability::Local { ref root, .. } = entry.availability {
                cells.push(root.display().to_string().into());
            } else {
                cells.push(String::new().into());
            }
        }

        cells.push(info.description.to_string().into());
        table.add_row(cells);
    }

    table.display(&term).to_string()
}

/// Returns the search-context hint for `sniff software test-runners`.
///
/// The hint explains that test-runner output is cwd-sensitive (project-local
/// bins are searched relative to the current directory) and points at
/// `sniff repo test-runner` for repo-declared usage. The hint belongs on
/// **stderr** so stdout stays parseable.
pub fn test_runners_search_hint(_mode: HintMode) -> String {
    Prose::new(
        "<dim>Searched from the current directory. Use `sniff repo test-runner` for \
         repo-declared usage.</dim>",
    )
    .render(&Terminal::default())
}

fn linked_name_cell(name: &str, website: &str, term: &Terminal) -> String {
    if website.is_empty() {
        return name.to_string();
    }
    Prose::new(format!(r#"<a href="{website}">{name}</a>"#)).render(term)
}

fn availability_cell(availability: &Availability, term: &Terminal) -> String {
    match availability {
        Availability::Installed { .. } => Prose::new("<green>installed</green>").render(term),
        Availability::Local { .. } => Prose::new("<cyan>local</cyan>").render(term),
        Availability::ViaParent { parent } => {
            Prose::new(format!("<yellow>via_parent</yellow> <dim>{parent}</dim>")).render(term)
        }
        Availability::NotFound => Prose::new("<dim>not_found</dim>").render(term),
    }
}
