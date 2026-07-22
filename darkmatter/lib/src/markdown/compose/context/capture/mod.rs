//! Raw runtime fact capture from chrono, std::env, and sniff.

mod agent;
mod changes;
mod datetime;
mod docs;
mod git;
mod groups;
mod host;
mod languages;
mod repo;
mod snapshot;

use std::path::Path;
use std::time::Duration;

use serde_json::{Map, Value};

use super::diagnostics::ContextMergeDiagnostic;

/// Result of a context capture pass: merged values, any diagnostics, and per-group timings.
type CaptureResult = (
    Map<String, Value>,
    Vec<ContextMergeDiagnostic>,
    Vec<(String, Duration)>,
);

pub(crate) use groups::{ContextGroup, scan_needed_groups};
pub(crate) use datetime::populate_datetime;

/// Capture all runtime context variables for the given base directory.
pub(crate) fn capture_runtime_context(base_dir: &Path) -> CaptureResult {
    capture_runtime_context_for_groups(base_dir, &ContextGroup::all())
}

/// Capture only the context groups needed for the given document content.
///
/// Scans `content` for `ctx.*` references and only captures the required
/// groups. If no `ctx.*` references are found, only populates datetime
/// (local computation plus a cheap OS timezone read — no network probe).
pub(crate) fn capture_runtime_context_for_content(base_dir: &Path, content: &str) -> CaptureResult {
    let mut groups = scan_needed_groups(content);
    // DateTime is always included: local chrono computation plus a cheap OS
    // timezone read (`detect_timezone_with_options(false)` — no network probe).
    groups.insert(ContextGroup::DateTime);
    let groups_vec: Vec<ContextGroup> = groups.into_iter().collect();
    capture_runtime_context_for_groups(base_dir, &groups_vec)
}

/// Capture runtime context for the specified groups only.
pub(crate) fn capture_runtime_context_for_groups(
    base_dir: &Path,
    groups: &[ContextGroup],
) -> CaptureResult {
    let cap = snapshot::ContextCapture::new(base_dir, groups);
    let mut values = Map::new();

    if groups.contains(&ContextGroup::DateTime) {
        datetime::populate_datetime(&mut values);
    }

    if groups.contains(&ContextGroup::Git) {
        git::populate_git(&cap, &mut values);
    }

    if groups.contains(&ContextGroup::Repo) {
        repo::populate_repo(&cap, &mut values);
    }

    if groups.contains(&ContextGroup::FileChanges) {
        changes::populate_file_changes(&cap, &mut values);
        changes::populate_package_changes(&cap, &mut values);
    }

    if groups.contains(&ContextGroup::Languages) {
        languages::populate_languages(&cap, &mut values);
    }

    if groups.contains(&ContextGroup::Documents) {
        docs::populate_docs(&cap, &mut values);
        docs::populate_skills(&cap, &mut values);
    }

    if groups.contains(&ContextGroup::Os) {
        host::populate_os(&cap, &mut values);
    }

    if groups.contains(&ContextGroup::Hardware) {
        host::populate_hardware(&cap, &mut values);
    }

    if groups.contains(&ContextGroup::Gpu) {
        host::populate_gpu(&cap, &mut values);
    }

    if groups.contains(&ContextGroup::Agent) {
        agent::populate_agent(&mut values);
    }

    (values, cap.diagnostics, cap.timings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_without_runtime_context_only_populates_datetime() {
        let (values, diagnostics, timings) =
            capture_runtime_context_for_content(Path::new("."), "ordinary markdown");

        assert!(values.contains_key("now"));
        assert!(!values.contains_key("repo"));
        assert!(!values.contains_key("os"));
        assert!(!values.contains_key("gpu"));
        assert!(diagnostics.is_empty());
        assert!(timings.is_empty());
    }
}
