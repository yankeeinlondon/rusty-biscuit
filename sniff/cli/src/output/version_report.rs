//! Output rendering for `sniff repo version`.
//!
//! Reports library-provided [`VersionAttribution`] entries (a distinct version
//! string + the manifest sources that produced it + the packages that carry
//! it). The CLI never re-detects or re-prioritizes; it only formats what the
//! library collapsed via `aggregate_versions`.

use std::path::Path;

use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::terminal::Terminal;
use sniff::filesystem::repo::{VersionAttribution, VersionSourceAttribution};

/// Build the `--json` value: always `{ "versions": [ ... ] }`.
///
/// Each entry carries the version string, the in-scope packages that carry
/// it, and the manifest sources that produced it (each source carrying its
/// own `packages` list). Every source object is enriched with an absolute
/// `href` for terminal hyperlinking.
pub fn build_version_json(entries: &[VersionAttribution], repo_root: &Path) -> serde_json::Value {
    let versions: Vec<serde_json::Value> = entries
        .iter()
        .map(|entry| {
            serde_json::json!({
                "version": entry.version,
                "packages": entry.packages,
                "sources": entry.sources.iter().map(|s| source_json(s, repo_root)).collect::<Vec<_>>(),
            })
        })
        .collect();
    serde_json::json!({ "versions": versions })
}

/// Serialize a source attribution, enriching it with an absolute `href` to
/// the manifest file.
fn source_json(source: &VersionSourceAttribution, repo_root: &Path) -> serde_json::Value {
    serde_json::json!({
        "manifest": source.source.manifest,
        "path": source.source.path,
        "href": file_href(repo_root, &source.source.path),
        "inherited": source.source.inherited,
        "packages": source.packages,
    })
}

/// Render the styled, comma-separated answer (the default text format).
///
/// A single entry collapses to a bare version string; multiple entries are
/// joined with `, `. Under `--verbose` each entry gains its source
/// information, and a shared source across many packages is summarized as
/// `from N manifests` rather than naming a misleading first source.
pub fn render_entries(
    entries: &[VersionAttribution],
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

/// Distinct version strings, first-seen order — the payload for the
/// name-only `--csv` / `--list` / `--md` formats without `--verbose`.
pub fn entry_names(entries: &[VersionAttribution]) -> Vec<&str> {
    let mut names: Vec<&str> = Vec::new();
    for entry in entries {
        if !names.contains(&entry.version.as_str()) {
            names.push(entry.version.as_str());
        }
    }
    names
}

/// Styled rendering of a single entry (version + verbose detail + source
/// attribution), for the `--list` / `--md` / `--csv` formats under `--verbose`.
/// Identical styling to the default CSV; only the join delimiter differs at
/// the call site. `--plain` strips the styling as usual.
pub fn render_one(
    entry: &VersionAttribution,
    verbose: u8,
    multi: bool,
    repo_root: &Path,
    term: &Terminal,
) -> String {
    Prose::new(entry_markup(entry, verbose, multi, repo_root)).render(term)
}

/// Markup for one entry: the bold version, plus a parenthetical with the
/// source information (verbose) and/or attributing package (multi-entry).
fn entry_markup(
    entry: &VersionAttribution,
    verbose: u8,
    multi: bool,
    repo_root: &Path,
) -> String {
    let mut detail: Vec<String> = Vec::new();
    if verbose > 0 {
        detail.push(source_detail_markup(entry, repo_root));
    }
    // Name the package only when disambiguation helps: a single source shared
    // by many packages (e.g. workspace inheritance) names no single package.
    if multi && entry.packages.len() == 1 {
        detail.push(format!(
            "<yellow>{}</yellow><i> package</i>",
            entry.packages[0]
        ));
    }
    if detail.is_empty() {
        format!("<b>{}</b>", entry.version)
    } else {
        format!("<b>{}</b> (<dim>{}</dim>)", entry.version, detail.join(", "))
    }
}

/// The inner (un-parenthesized) markup describing where the version was
/// read from.
///
/// When one collapsed version has multiple manifest sources, render a
/// summary like `from 12 manifests` instead of a misleading first source.
/// Otherwise hyperlink the single manifest. Workspace-inherited Cargo
/// versions are named as `[workspace.package]` in the root manifest so the
/// user can tell the value is inherited.
fn source_detail_markup(entry: &VersionAttribution, repo_root: &Path) -> String {
    if entry.sources.len() != 1 {
        return format!(
            "<i>from </i><b>{}</b><i> manifests</i>",
            entry.sources.len()
        );
    }
    let source = &entry.sources[0].source;
    if source.inherited {
        // Markdown link syntax so the brackets survive the preprocessor
        // (the preprocessor lifts `[desc](ref)` directly to `<a>`).
        format!(
            "<i>from </i><blue>[[workspace.package]]({})</blue> <dim>in Cargo.toml</dim>",
            file_href(repo_root, &source.path)
        )
    } else {
        format!(
            "<i>from </i><blue><a href=\"{}\">{}</a></blue>",
            file_href(repo_root, &source.path),
            source.path
        )
    }
}

/// A `file://` URL to the repo-relative manifest `rel`, for terminal
/// hyperlinks.
fn file_href(repo_root: &Path, rel: &str) -> String {
    format!("file://{}", repo_root.join(rel).display())
}
