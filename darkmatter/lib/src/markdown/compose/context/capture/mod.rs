//! Raw runtime fact capture from chrono, std::env, and sniff.

mod agent;
mod changes;
mod datetime;
mod docs;
mod git;
mod groups;
mod host;
mod invocation;
mod languages;
mod repo;
mod snapshot;

use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

use serde_json::{Map, Value};

use super::diagnostics::ContextMergeDiagnostic;

/// Result of a context capture pass: merged values, any diagnostics, and per-group timings.
pub(super) type CaptureResult = (
    Map<String, Value>,
    Vec<ContextMergeDiagnostic>,
    Vec<(String, Duration)>,
    HashMap<String, String>,
);

pub use groups::{ContextGroup, ContextRequirements};
pub use snapshot::ContextCaptureEvidence;
pub(crate) use datetime::populate_datetime;

fn capture_repository_scope_catalog(
    base_dir: &Path,
) -> Option<biscuit_file::RepositoryScopeCatalog> {
    let capture = snapshot::ContextCapture::new(
        base_dir,
        &[ContextGroup::Repo],
        Ok(base_dir.to_path_buf()),
    );
    let root = capture.repo_root.as_deref()?;
    match capture.repo_info.as_ref() {
        Some(repo) => super::repository_scope::repository_scope_catalog(repo, root).ok(),
        None => biscuit_file::RepositoryScopeCatalog::new(
            root,
            Vec::new(),
            Vec::new(),
            biscuit_file::PackageAreaFallback::None,
        )
        .ok(),
    }
}

pub fn capture_file_resolution_context(base_dir: &Path) -> biscuit_file::FileResolutionContext {
    let mut context = biscuit_file::FileResolutionContext::new(base_dir);
    if let Some(catalog) = capture_repository_scope_catalog(base_dir) {
        context = context.with_repository_scope_catalog(catalog);
    }
    context
}

/// Capture all runtime context variables for the given base directory.
pub(crate) fn capture_runtime_context(base_dir: &Path) -> CaptureResult {
    capture_runtime_context_for_requirements(base_dir, &ContextRequirements::all())
}

/// Capture only the context groups needed for the given document content.
///
/// Scans `content` for `ctx.*` references and only captures the required
/// groups. If no `ctx.*` references are found, only populates datetime
/// (local computation plus a cheap OS timezone read — no network probe).
pub(crate) fn capture_runtime_context_for_content(base_dir: &Path, content: &str) -> CaptureResult {
    capture_runtime_context_for_requirements(base_dir, &ContextRequirements::for_content(content))
}

/// Capture runtime context for the specified groups only.
pub(crate) fn capture_runtime_context_for_groups(
    base_dir: &Path,
    groups: &[ContextGroup],
) -> CaptureResult {
    let requirements = ContextRequirements::from_groups(groups.iter().copied());
    capture_runtime_context_for_requirements(base_dir, &requirements)
}

fn capture_runtime_context_for_requirements(
    base_dir: &Path,
    requirements: &ContextRequirements,
) -> CaptureResult {
    capture_runtime_context_for_requirements_with_cwd(
        base_dir,
        requirements,
        Ok(base_dir.to_path_buf()),
    )
}

fn capture_runtime_context_for_requirements_with_cwd(
    base_dir: &Path,
    requirements: &ContextRequirements,
    invocation_cwd: std::io::Result<std::path::PathBuf>,
) -> CaptureResult {
    let environment = std::env::vars().collect();
    let groups: Vec<_> = requirements.iter().collect();
    let cap = snapshot::ContextCapture::new(base_dir, &groups, invocation_cwd);
    populate_capture(cap, requirements, environment)
}

pub(super) fn capture_runtime_context_with_evidence(
    base_dir: &Path,
    requirements: &ContextRequirements,
    evidence: &ContextCaptureEvidence,
) -> CaptureResult {
    let groups: Vec<_> = requirements.iter().collect();
    let cap = snapshot::ContextCapture::from_evidence(base_dir, &groups, evidence);
    populate_capture(cap, requirements, evidence.environment().clone())
}

fn populate_capture(
    cap: snapshot::ContextCapture,
    requirements: &ContextRequirements,
    environment: HashMap<String, String>,
) -> CaptureResult {
    let mut values = Map::new();

    if requirements.contains(ContextGroup::Invocation) {
        invocation::populate_invocation(&cap, &mut values);
    }

    if requirements.contains(ContextGroup::DateTime) {
        datetime::populate_datetime(&mut values);
    }

    if requirements.contains(ContextGroup::Git) {
        git::populate_git(&cap, &mut values);
    }

    if requirements.contains(ContextGroup::Repo) {
        repo::populate_repo(&cap, &mut values);
    }

    if requirements.contains(ContextGroup::FileChanges) {
        changes::populate_file_changes(&cap, &mut values);
        changes::populate_package_changes(&cap, &mut values);
    }

    if requirements.contains(ContextGroup::Languages) {
        languages::populate_languages(&cap, &mut values);
    }

    if requirements.contains(ContextGroup::Documents) {
        docs::populate_docs(&cap, &mut values);
        docs::populate_skills(&cap, &mut values);
    }

    if requirements.contains(ContextGroup::Os) {
        host::populate_os(&cap, &mut values);
    }

    if requirements.contains(ContextGroup::Hardware) {
        host::populate_hardware(&cap, &mut values);
    }

    if requirements.contains(ContextGroup::Gpu) {
        host::populate_gpu(&cap, &mut values);
    }

    if requirements.contains(ContextGroup::Agent) {
        agent::populate_agent(&environment, &mut values);
    }

    (values, cap.diagnostics, cap.timings, environment)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_without_runtime_context_only_populates_datetime() {
        let (values, diagnostics, timings, _) =
            capture_runtime_context_for_content(Path::new("."), "ordinary markdown");

        assert!(values.contains_key("now"));
        assert!(!values.contains_key("repo"));
        assert!(!values.contains_key("os"));
        assert!(!values.contains_key("gpu"));
        assert!(diagnostics.is_empty());
        assert!(timings.is_empty());
    }

    #[test]
    fn cwd_capture_is_absolute_portable_and_repository_independent() {
        let outside = tempfile::tempdir().unwrap();
        let requirements = ContextRequirements::from_groups([ContextGroup::Invocation]);
        let (values, diagnostics, timings, _) = capture_runtime_context_for_requirements_with_cwd(
            outside.path(),
            &requirements,
            Ok(outside.path().to_path_buf()),
        );

        let cwd = values.get("cwd").and_then(Value::as_str).expect("ctx.cwd");
        assert_eq!(cwd, biscuit_file::to_portable_string(outside.path()));
        assert!(Path::new(cwd).is_absolute());
        assert!(!values.contains_key("repo_root"));
        assert!(diagnostics.is_empty());
        assert!(timings.is_empty());
    }

    #[test]
    fn cwd_capture_failure_is_null_with_a_partial_diagnostic() {
        let outside = tempfile::tempdir().unwrap();
        let requirements = ContextRequirements::from_groups([ContextGroup::Invocation]);
        let (values, diagnostics, timings, _) = capture_runtime_context_for_requirements_with_cwd(
            outside.path(),
            &requirements,
            Err(std::io::Error::other("forced current directory failure")),
        );

        assert_eq!(values.get("cwd"), Some(&Value::Null));
        assert!(diagnostics.iter().any(|diagnostic| matches!(
            diagnostic,
            ContextMergeDiagnostic::PartialRuntimeCapture { area: "invocation.cwd", detail }
                if detail.contains("forced current directory failure")
        )));
        assert!(timings.is_empty());
    }

    #[test]
    fn invocation_projection_has_no_ambient_cwd_read() {
        for source in [
            include_str!("invocation.rs"),
            include_str!("groups.rs"),
            include_str!("snapshot.rs"),
        ] {
            assert!(
                !source.contains(concat!("current", "_dir(")),
                "invocation capture must consume boundary evidence instead of rediscovering CWD",
            );
        }
    }
}
