//! Output rendering for `sniff repo test-runner`.
//!
//! Reports library-provided [`TestRunnerAttribution`] entries (a distinct
//! runner + evidence source, plus the packages that attribute it). The CLI
//! never re-detects or re-prioritizes; it only formats what the library
//! collapsed via `aggregate_test_runners`.

use std::path::Path;

use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::terminal::Terminal;
use sniff::filesystem::repo::{TestRunnerAttribution, TestRunnerSource};
use sniff::programs::schema::ProgramMetadata;
use sniff::programs::test_runner_spec::run_command;

/// Build the `--json` value: always `{ "test_runners": [ ... ] }`.
///
/// Each entry carries the runner identity, the literal run command (`binary`),
/// the documentation `website`, the evidence `source`, and the in-scope
/// `packages` that attribute it.
pub fn build_test_runner_json(
    entries: &[TestRunnerAttribution],
    repo_root: &Path,
) -> serde_json::Value {
    let runners: Vec<serde_json::Value> = entries
        .iter()
        .map(|entry| {
            let info = entry.usage.runner.info();
            serde_json::json!({
                "runner": serde_json::to_value(entry.usage.runner)
                    .unwrap_or(serde_json::Value::Null),
                "name": info.display_name,
                "binary": run_command(entry.usage.runner),
                "website": info.website,
                "source": source_json(&entry.usage.source, repo_root),
                "packages": entry.packages,
            })
        })
        .collect();
    serde_json::json!({ "test_runners": runners })
}

/// Serialize a source, enriching `Config` with an absolute `href` to the file.
fn source_json(source: &TestRunnerSource, repo_root: &Path) -> serde_json::Value {
    let mut value = serde_json::to_value(source).unwrap_or(serde_json::Value::Null);
    if let (TestRunnerSource::Config { path, .. }, Some(obj)) = (source, value.as_object_mut()) {
        obj.insert(
            "href".to_string(),
            serde_json::Value::String(file_href(repo_root, path)),
        );
    }
    value
}

/// Render the styled, comma-separated answer (the default text format).
///
/// A single entry collapses to a bare runner name; multiple entries are joined
/// with `, `. Under `--verbose` each entry gains its evidence source, and when
/// more than one entry is present, the attributing package is named so the
/// caller can tell which package uses which runner.
pub fn render_entries(
    entries: &[TestRunnerAttribution],
    verbose: u8,
    repo_root: &Path,
    term: &Terminal,
) -> String {
    let multi = entries.len() > 1;
    let markup = entries
        .iter()
        .map(|entry| entry_markup(entry, verbose, multi, repo_root))
        .collect::<Vec<_>>()
        .join(", ");
    let mut out = Prose::new(markup).render(term);
    out.push('\n');
    out
}

/// Distinct runner display names, first-seen order — the payload for the
/// name-only `--csv` / `--list` / `--md` formats without `--verbose`.
pub fn entry_names(entries: &[TestRunnerAttribution]) -> Vec<&'static str> {
    let mut names: Vec<&'static str> = Vec::new();
    for entry in entries {
        let name = entry.usage.runner.info().display_name;
        if !names.contains(&name) {
            names.push(name);
        }
    }
    names
}

/// Styled rendering of a single entry (name + verbose detail + attribution),
/// for the `--list` / `--md` / `--csv` formats under `--verbose`. Identical
/// styling to the default CSV; only the join delimiter differs at the call
/// site. `--plain` strips the styling as usual.
pub fn render_one(
    entry: &TestRunnerAttribution,
    verbose: u8,
    multi: bool,
    repo_root: &Path,
    term: &Terminal,
) -> String {
    Prose::new(entry_markup(entry, verbose, multi, repo_root)).render(term)
}

/// Markup for one entry: the bold runner name, plus a parenthetical with the
/// evidence source (verbose) and/or attributing package (multi-entry).
fn entry_markup(
    entry: &TestRunnerAttribution,
    verbose: u8,
    multi: bool,
    repo_root: &Path,
) -> String {
    let name = entry.usage.runner.info().display_name;
    let mut detail: Vec<String> = Vec::new();
    if verbose > 0 {
        detail.push(source_detail_markup(&entry.usage.source, repo_root));
    }
    // Name the package only when disambiguation helps: a shared usage (one
    // workspace-root config across many crates) names no single package.
    if multi && entry.packages.len() == 1 {
        detail.push(format!(
            "<yellow>{}</yellow><i> package</i>",
            entry.packages[0]
        ));
    }
    if detail.is_empty() {
        format!("<b>{name}</b>")
    } else {
        format!("<b>{name}</b> (<dim>{}</dim>)", detail.join(", "))
    }
}

/// The inner (un-parenthesized) markup describing why a runner was attributed.
fn source_detail_markup(source: &TestRunnerSource, repo_root: &Path) -> String {
    match source {
        TestRunnerSource::Config { path, .. } => format!(
            "<i>configuration located at:</i> <blue><a href=\"{}\">{path}</a></blue>",
            file_href(repo_root, path)
        ),
        TestRunnerSource::Manifest { key } => {
            format!("<i>declared as dependency:</i> {key}")
        }
        TestRunnerSource::EcosystemDefault => "<i>ecosystem default</i>".to_string(),
        TestRunnerSource::Convention => "<i>by convention</i>".to_string(),
    }
}

/// A `file://` URL to the repo-relative config `rel`, for terminal hyperlinks.
fn file_href(repo_root: &Path, rel: &str) -> String {
    format!("file://{}", repo_root.join(rel).display())
}
