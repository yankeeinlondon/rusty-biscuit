//! Validation engine for pre-checks and post-checks.
//!
//! Executes validation rules, captures pre-run snapshots for comparison-based
//! post-checks, and renders success/failure messages.

use std::path::Path;

use tracing::{debug, info_span};

use crate::harness::error::HarnessError;
use crate::harness::model::{
    AttemptOutcome, FailurePhase, HarnessPermissionProbe, HarnessPlan, PreRunSnapshot,
    ValidationCheckOutcome, ValidationKind, ValidationPhaseReport, ValidationRule,
};

mod compare;
mod fs;
mod git;
mod render;

pub use fs::check_write_permission;
pub(crate) use render::build_check_markup;

/// Evaluate all pre-checks in declaration order.
///
/// Runs every rule and collects all outcomes (does not short-circuit).
/// Returns a `ValidationPhaseReport`; the caller decides whether failures
/// are fatal and controls rendering.
pub fn evaluate_pre_checks(
    plan: &HarnessPlan,
    permission_probe: Option<&dyn HarnessPermissionProbe>,
) -> ValidationPhaseReport {
    let _span = info_span!(
        "harness_pre_checks",
        source = %plan.source_path.display(),
        check_count = plan.pre_checks.len(),
    )
    .entered();
    let report = run_checks(
        &plan.pre_checks,
        None,
        None,
        &plan.source_path,
        permission_probe,
        FailurePhase::PreCheck,
    );
    debug!(
        total = report.count(),
        passed = report.outcomes.iter().filter(|o| o.passed).count(),
        failed = report.failures().len(),
        "pre-checks complete",
    );
    report
}

/// Capture a pre-run snapshot for subjects referenced by post-checks.
///
/// Only captures state for files and frontmatter properties that are
/// actually referenced, keeping the snapshot small and deterministic.
pub fn capture_pre_run_snapshot(plan: &HarnessPlan) -> Result<PreRunSnapshot, HarnessError> {
    let mut snapshot = PreRunSnapshot::default();

    for rule in &plan.post_checks {
        match &rule.kind {
            ValidationKind::FileChanged { file } | ValidationKind::FileUnchanged { file }
                if !snapshot.tracked_files.contains_key(file) =>
            {
                snapshot
                    .tracked_files
                    .insert(file.clone(), compare::fingerprint_file(file));
            }
            ValidationKind::FrontmatterPropChanged { prop }
            | ValidationKind::FrontmatterPropUnchanged { prop } => {
                if snapshot.source_markdown.is_none() {
                    // Load and parse source document frontmatter
                    if let Ok(text) = std::fs::read_to_string(&plan.source_path) {
                        let md: darkmatter::markdown::Markdown = text.into();
                        // Capture the specific property values
                        if let Ok(Some(val)) = md.fm_get::<serde_json::Value>(prop) {
                            snapshot.tracked_frontmatter.insert(prop.clone(), val);
                        }
                        snapshot.source_markdown = Some(md);
                    }
                } else if !snapshot.tracked_frontmatter.contains_key(prop) {
                    // Source already loaded, just grab this property
                    if let Some(ref md) = snapshot.source_markdown
                        && let Ok(Some(val)) = md.fm_get::<serde_json::Value>(prop)
                    {
                        snapshot.tracked_frontmatter.insert(prop.clone(), val);
                    }
                }
            }
            ValidationKind::FrontmatterPropEquals { .. } => {
                // Will compare against on-disk state at post-check time; no pre-snapshot needed
            }
            // Response-based and filesystem existence checks don't need pre-snapshots
            _ => {}
        }
    }

    Ok(snapshot)
}

/// Evaluate all post-checks in declaration order.
///
/// Uses pre-state from `snapshot` and post-state from disk/outcome.
/// Returns a `ValidationPhaseReport`; the caller decides whether failures
/// are fatal and controls rendering.
pub fn evaluate_post_checks(
    plan: &HarnessPlan,
    snapshot: &PreRunSnapshot,
    outcome: &AttemptOutcome,
    permission_probe: Option<&dyn HarnessPermissionProbe>,
) -> ValidationPhaseReport {
    let _span = info_span!(
        "harness_post_checks",
        source = %plan.source_path.display(),
        check_count = plan.post_checks.len(),
        attempt = outcome.attempt,
        termination = %outcome.termination,
    )
    .entered();
    let report = run_checks(
        &plan.post_checks,
        Some(snapshot),
        Some(outcome),
        &plan.source_path,
        permission_probe,
        FailurePhase::PostCheck,
    );
    debug!(
        total = report.count(),
        passed = report.outcomes.iter().filter(|o| o.passed).count(),
        failed = report.failures().len(),
        "post-checks complete",
    );
    report
}

/// Run a set of checks and return a structured phase report.
fn run_checks(
    rules: &[ValidationRule],
    snapshot: Option<&PreRunSnapshot>,
    outcome: Option<&AttemptOutcome>,
    source_path: &Path,
    permission_probe: Option<&dyn HarnessPermissionProbe>,
    failure_phase: FailurePhase,
) -> ValidationPhaseReport {
    let mut outcomes = Vec::new();

    // For post-checks involving frontmatter, parse the current on-disk
    // markdown once and share it across all frontmatter checks.
    let post_run_markdown = outcome.map(|_| match std::fs::read_to_string(source_path) {
        Ok(text) => PostRunMarkdownState::Loaded(text.into()),
        Err(error) => PostRunMarkdownState::ReadFailed {
            path: source_path.to_path_buf(),
            error: error.to_string(),
        },
    });

    for rule in rules {
        let result = evaluate_single(
            rule,
            snapshot,
            outcome,
            source_path,
            permission_probe,
            post_run_markdown.as_ref(),
        );
        let passed = result.is_ok();
        let markup = build_check_markup(rule);
        let failure_message = result.err();

        outcomes.push(ValidationCheckOutcome {
            rule_id: rule.id,
            event: rule.event.clone(),
            subject_key: rule.subject_key.clone(),
            passed,
            markup,
            failure_message,
            source: rule.source.clone(),
        });
    }

    ValidationPhaseReport {
        phase: failure_phase,
        outcomes,
    }
}

/// Result of evaluating a single validation: `Ok(())` for pass, `Err(message)` for fail.
type CheckResult = Result<(), String>;

enum PostRunMarkdownState {
    Loaded(darkmatter::markdown::Markdown),
    ReadFailed {
        path: std::path::PathBuf,
        error: String,
    },
}

/// Evaluate a single validation rule.
fn evaluate_single(
    rule: &ValidationRule,
    snapshot: Option<&PreRunSnapshot>,
    outcome: Option<&AttemptOutcome>,
    source_path: &Path,
    permission_probe: Option<&dyn HarnessPermissionProbe>,
    post_run_markdown: Option<&PostRunMarkdownState>,
) -> CheckResult {
    match &rule.kind {
        // --- Filesystem checks ---
        ValidationKind::FileExists { file } => {
            if file.exists() && !file.is_dir() {
                Ok(())
            } else {
                Err(format!("file does not exist: {}", file.display()))
            }
        }
        ValidationKind::DirExists { dir } => {
            if dir.exists() && dir.is_dir() {
                Ok(())
            } else {
                Err(format!("directory does not exist: {}", dir.display()))
            }
        }
        ValidationKind::JsonFileExists { file, shape } => fs::check_json_file(file, shape.as_ref()),
        ValidationKind::YamlFileExists { file, shape } => fs::check_yaml_file(file, shape.as_ref()),
        ValidationKind::TomlFileExists { file } => fs::check_toml_file(file),
        ValidationKind::HasWritePermission { file } => {
            fs::check_write_permission(file, source_path, permission_probe)
        }
        ValidationKind::ShellCommand {
            command,
            show_stdout,
            show_stderr,
        } => fs::check_shell_command(command, *show_stdout, *show_stderr),

        // --- Git checks ---
        ValidationKind::NoDirtySourceCode { root } => git::check_dirty_source_code(root, false),
        ValidationKind::HasDirtySourceCode { root } => git::check_dirty_source_code(root, true),

        // --- Post-only: file comparison ---
        ValidationKind::FileChanged { file } => compare::check_file_changed(file, snapshot, true),
        ValidationKind::FileUnchanged { file } => {
            compare::check_file_changed(file, snapshot, false)
        }

        // --- Post-only: frontmatter comparison ---
        ValidationKind::FrontmatterPropChanged { prop } => {
            compare::check_frontmatter_prop_changed(prop, snapshot, post_run_markdown, true)
        }
        ValidationKind::FrontmatterPropUnchanged { prop } => {
            compare::check_frontmatter_prop_changed(prop, snapshot, post_run_markdown, false)
        }
        ValidationKind::FrontmatterPropEquals { expected } => {
            compare::check_frontmatter_prop_equals(expected, post_run_markdown)
        }

        // --- Post-only: response checks ---
        ValidationKind::ResponseLengthAtLeast { length } => {
            let resp = outcome
                .map(|o| o.final_response.chars().count())
                .unwrap_or(0);
            if resp >= *length {
                Ok(())
            } else {
                Err(format!(
                    "response length {resp} is less than required {length}"
                ))
            }
        }
        ValidationKind::ResponseLengthAtMost { length } => {
            let resp = outcome
                .map(|o| o.final_response.chars().count())
                .unwrap_or(0);
            if resp <= *length {
                Ok(())
            } else {
                Err(format!("response length {resp} exceeds maximum {length}"))
            }
        }
        ValidationKind::ResponseIncludes { needle } => {
            let resp = outcome.map(|o| o.final_response.as_str()).unwrap_or("");
            if resp.contains(needle.as_str()) {
                Ok(())
            } else {
                Err(format!("response does not include \"{needle}\""))
            }
        }
        ValidationKind::ResponseMissing { needle } => {
            let resp = outcome.map(|o| o.final_response.as_str()).unwrap_or("");
            if !resp.contains(needle.as_str()) {
                Ok(())
            } else {
                Err(format!("response unexpectedly includes \"{needle}\""))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::harness::model::StructuredShape;
    use crate::harness::model::{
        PermissionAssessment, ProcessTermination, RuleSource, ValidationEvent, ValidationPhase,
        ValidationRuleId,
    };
    use crate::harness::validate::compare::{check_frontmatter_prop_equals, fingerprint_file};
    use crate::harness::validate::render::{build_vars, default_message, render_template};
    use std::collections::HashMap;
    use std::process::Command;
    use tempfile::TempDir;

    struct StaticPermissionProbe {
        assessment: PermissionAssessment,
    }

    impl HarnessPermissionProbe for StaticPermissionProbe {
        fn can_write(&self, _path: &Path, _source_path: &Path) -> PermissionAssessment {
            self.assessment.clone()
        }
    }

    fn make_rule(id: u32, event: ValidationEvent, kind: ValidationKind) -> ValidationRule {
        ValidationRule {
            id: ValidationRuleId(id),
            event,
            phase: ValidationPhase::Both,
            kind,
            message_template: None,
            subject_key: None,
            source: None,
        }
    }

    fn make_outcome(response: &str) -> AttemptOutcome {
        AttemptOutcome {
            attempt: 1,
            session_id: None,
            final_response: response.to_string(),
            exit_code: 0,
            termination: ProcessTermination::Completed,
            stderr_text: None,
        }
    }

    fn evaluate_single_for_tests(
        rule: &ValidationRule,
        snapshot: Option<&PreRunSnapshot>,
        outcome: Option<&AttemptOutcome>,
        post_run_markdown: Option<&PostRunMarkdownState>,
    ) -> CheckResult {
        let source_path = std::path::PathBuf::from("/tmp/source.md");
        evaluate_single(
            rule,
            snapshot,
            outcome,
            &source_path,
            None,
            post_run_markdown,
        )
    }

    fn approved_shell_command(
        shell_fragment: &str,
    ) -> crate::harness::model::ApprovedRuntimeCommand {
        crate::harness::model::ApprovedRuntimeCommand {
            raw: shell_fragment.to_string(),
            executable: "sh".to_string(),
            args: vec!["-c".to_string(), shell_fragment.to_string()],
        }
    }

    fn init_committed_repo(
        dir: &TempDir,
        relative_path: &str,
        content: &str,
    ) -> std::path::PathBuf {
        assert!(
            Command::new("git")
                .arg("init")
                .current_dir(dir.path())
                .status()
                .unwrap()
                .success()
        );
        assert!(
            Command::new("git")
                .args(["config", "user.email", "claudine-tests@example.com"])
                .current_dir(dir.path())
                .status()
                .unwrap()
                .success()
        );
        assert!(
            Command::new("git")
                .args(["config", "user.name", "Claudine Tests"])
                .current_dir(dir.path())
                .status()
                .unwrap()
                .success()
        );

        let file = dir.path().join(relative_path);
        if let Some(parent) = file.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&file, content).unwrap();

        assert!(
            Command::new("git")
                .args(["add", "."])
                .current_dir(dir.path())
                .status()
                .unwrap()
                .success()
        );
        assert!(
            Command::new("git")
                .args(["commit", "-m", "initial"])
                .current_dir(dir.path())
                .status()
                .unwrap()
                .success()
        );

        file
    }

    // --- Filesystem checks ---

    #[test]
    fn file_exists_passes_for_existing_file() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("test.txt");
        std::fs::write(&file, "hello").unwrap();

        let result = evaluate_single_for_tests(
            &make_rule(
                0,
                ValidationEvent::FileExists,
                ValidationKind::FileExists { file },
            ),
            None,
            None,
            None,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn file_exists_fails_for_missing_file() {
        let result = evaluate_single_for_tests(
            &make_rule(
                0,
                ValidationEvent::FileExists,
                ValidationKind::FileExists {
                    file: std::path::PathBuf::from("/nonexistent/file.txt"),
                },
            ),
            None,
            None,
            None,
        );
        assert!(result.is_err());
    }

    #[test]
    fn dir_exists_passes_for_existing_dir() {
        let dir = TempDir::new().unwrap();
        let result = evaluate_single_for_tests(
            &make_rule(
                0,
                ValidationEvent::DirExists,
                ValidationKind::DirExists {
                    dir: dir.path().to_path_buf(),
                },
            ),
            None,
            None,
            None,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn json_file_exists_passes_for_valid_json() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("data.json");
        std::fs::write(&file, r#"{"key": "value"}"#).unwrap();

        let result = evaluate_single_for_tests(
            &make_rule(
                0,
                ValidationEvent::JsonFileExists,
                ValidationKind::JsonFileExists {
                    file,
                    shape: Some(StructuredShape::Object),
                },
            ),
            None,
            None,
            None,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn json_file_exists_fails_for_invalid_json() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("bad.json");
        std::fs::write(&file, "not json at all {").unwrap();

        let result = evaluate_single_for_tests(
            &make_rule(
                0,
                ValidationEvent::JsonFileExists,
                ValidationKind::JsonFileExists { file, shape: None },
            ),
            None,
            None,
            None,
        );
        assert!(result.is_err());
    }

    #[test]
    fn has_write_permission_passes_for_creatable_missing_file() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("new-file.txt");

        let result = evaluate_single_for_tests(
            &make_rule(
                0,
                ValidationEvent::HasWritePermission,
                ValidationKind::HasWritePermission { file: file.clone() },
            ),
            None,
            None,
            None,
        );

        assert!(
            result.is_ok(),
            "expected creatable path to pass: {result:?}"
        );
        assert!(
            !file.exists(),
            "write-permission probe should not leave the target file behind"
        );
    }

    #[test]
    fn has_write_permission_fails_when_parent_directory_is_missing() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("missing").join("new-file.txt");

        let result = evaluate_single_for_tests(
            &make_rule(
                0,
                ValidationEvent::HasWritePermission,
                ValidationKind::HasWritePermission { file: file.clone() },
            ),
            None,
            None,
            None,
        );

        let error = result.unwrap_err();
        assert!(error.contains("parent directory"));
        assert!(error.contains("does not exist"));
    }

    #[test]
    fn has_write_permission_fails_when_provider_probe_denies_write() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("allowed-by-os.txt");
        std::fs::write(&file, "hello").unwrap();
        let source_path = dir.path().join("source.md");
        let probe = StaticPermissionProbe {
            assessment: PermissionAssessment::Denied {
                reason: "sandbox is read-only".to_string(),
            },
        };

        let result = evaluate_single(
            &make_rule(
                0,
                ValidationEvent::HasWritePermission,
                ValidationKind::HasWritePermission { file: file.clone() },
            ),
            None,
            None,
            &source_path,
            Some(&probe),
            None,
        );

        let error = result.unwrap_err();
        assert!(error.contains("provider runtime policy denies writes"));
        assert!(error.contains("sandbox is read-only"));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn shell_command_passes_for_zero_exit_status() {
        let result = evaluate_single_for_tests(
            &make_rule(
                0,
                ValidationEvent::ShellCommand,
                ValidationKind::ShellCommand {
                    command: approved_shell_command("exit 0"),
                    show_stdout: false,
                    show_stderr: false,
                },
            ),
            None,
            None,
            None,
        );

        assert!(result.is_ok(), "expected shell command to pass: {result:?}");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn shell_command_failure_message_includes_command_and_exit_code() {
        let result = evaluate_single_for_tests(
            &make_rule(
                0,
                ValidationEvent::ShellCommand,
                ValidationKind::ShellCommand {
                    command: approved_shell_command("printf 'boom' >&2; exit 7"),
                    show_stdout: false,
                    show_stderr: false,
                },
            ),
            None,
            None,
            None,
        );

        let error = result.unwrap_err();
        assert!(error.contains("printf 'boom' >&2; exit 7"));
        assert!(error.contains("code 7"));
    }

    #[test]
    fn no_dirty_source_code_passes_for_clean_repo() {
        let dir = TempDir::new().unwrap();
        let source_file = init_committed_repo(&dir, "src/main.rs", "fn main() {}\n");

        let result = evaluate_single_for_tests(
            &make_rule(
                0,
                ValidationEvent::NoDirtySourceCode,
                ValidationKind::NoDirtySourceCode {
                    root: source_file.parent().unwrap().to_path_buf(),
                },
            ),
            None,
            None,
            None,
        );

        assert!(result.is_ok(), "expected clean repo to pass: {result:?}");
    }

    #[test]
    fn no_dirty_source_code_reports_dirty_source_files() {
        let dir = TempDir::new().unwrap();
        let source_file = init_committed_repo(&dir, "src/main.rs", "fn main() {}\n");
        std::fs::write(&source_file, "fn main() { println!(\"dirty\"); }\n").unwrap();

        let result = evaluate_single_for_tests(
            &make_rule(
                0,
                ValidationEvent::NoDirtySourceCode,
                ValidationKind::NoDirtySourceCode {
                    root: source_file.parent().unwrap().to_path_buf(),
                },
            ),
            None,
            None,
            None,
        );

        let error = result.unwrap_err();
        assert!(error.contains("dirty source code found"));
        assert!(error.contains("src/main.rs"));
    }

    #[test]
    fn has_dirty_source_code_passes_for_modified_source_files() {
        let dir = TempDir::new().unwrap();
        let source_file = init_committed_repo(&dir, "src/lib.rs", "pub fn run() {}\n");
        std::fs::write(&source_file, "pub fn run() { println!(\"dirty\"); }\n").unwrap();

        let result = evaluate_single_for_tests(
            &make_rule(
                0,
                ValidationEvent::HasDirtySourceCode,
                ValidationKind::HasDirtySourceCode {
                    root: source_file.parent().unwrap().to_path_buf(),
                },
            ),
            None,
            None,
            None,
        );

        assert!(
            result.is_ok(),
            "expected dirty repo to satisfy has_dirty_source_code: {result:?}"
        );
    }

    #[test]
    fn frontmatter_equals_reports_post_run_read_error_clearly() {
        let mut expected = indexmap::IndexMap::new();
        expected.insert("status".to_string(), serde_json::json!("done"));
        let state = PostRunMarkdownState::ReadFailed {
            path: std::path::PathBuf::from("/tmp/missing.md"),
            error: "No such file or directory".to_string(),
        };

        let result = check_frontmatter_prop_equals(&expected, Some(&state));

        let error = result.unwrap_err();
        assert!(error.contains("failed to read /tmp/missing.md"));
        assert!(error.contains("frontmatter equals check"));
    }

    #[test]
    fn yaml_file_exists_passes_for_valid_yaml_with_shape() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("data.yaml");
        std::fs::write(&file, "- item1\n- item2\n").unwrap();

        let result = evaluate_single_for_tests(
            &make_rule(
                0,
                ValidationEvent::YamlFileExists,
                ValidationKind::YamlFileExists {
                    file,
                    shape: Some(StructuredShape::Array),
                },
            ),
            None,
            None,
            None,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn toml_file_exists_fails_for_non_toml() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("bad.toml");
        std::fs::write(&file, "{{not toml}}").unwrap();

        let result = evaluate_single_for_tests(
            &make_rule(
                0,
                ValidationEvent::TomlFileExists,
                ValidationKind::TomlFileExists { file },
            ),
            None,
            None,
            None,
        );
        assert!(result.is_err());
    }

    // --- Post-check: file comparison ---

    #[test]
    fn file_changed_passes_when_hashes_differ() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("target.txt");
        std::fs::write(&file, "original content").unwrap();

        let pre_fingerprint = fingerprint_file(&file);
        let mut snapshot = PreRunSnapshot::default();
        snapshot.tracked_files.insert(file.clone(), pre_fingerprint);

        // Modify the file
        std::fs::write(&file, "modified content").unwrap();

        let result = evaluate_single_for_tests(
            &make_rule(
                0,
                ValidationEvent::FileChanged,
                ValidationKind::FileChanged { file },
            ),
            Some(&snapshot),
            None,
            None,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn file_changed_fails_when_hashes_match() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("target.txt");
        std::fs::write(&file, "same content").unwrap();

        let pre_fingerprint = fingerprint_file(&file);
        let mut snapshot = PreRunSnapshot::default();
        snapshot.tracked_files.insert(file.clone(), pre_fingerprint);

        // Don't modify — same hash
        let result = evaluate_single_for_tests(
            &make_rule(
                0,
                ValidationEvent::FileChanged,
                ValidationKind::FileChanged { file },
            ),
            Some(&snapshot),
            None,
            None,
        );
        assert!(result.is_err());
    }

    #[test]
    fn file_unchanged_passes_when_hashes_match() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("target.txt");
        std::fs::write(&file, "same content").unwrap();

        let pre_fingerprint = fingerprint_file(&file);
        let mut snapshot = PreRunSnapshot::default();
        snapshot.tracked_files.insert(file.clone(), pre_fingerprint);

        let result = evaluate_single_for_tests(
            &make_rule(
                0,
                ValidationEvent::FileUnchanged,
                ValidationKind::FileUnchanged { file },
            ),
            Some(&snapshot),
            None,
            None,
        );
        assert!(result.is_ok());
    }

    // --- Post-check: response checks ---

    #[test]
    fn response_length_at_least_passes_at_boundary() {
        let outcome = make_outcome("hello"); // len=5
        let result = evaluate_single_for_tests(
            &make_rule(
                0,
                ValidationEvent::ResponseLengthAtLeast,
                ValidationKind::ResponseLengthAtLeast { length: 5 },
            ),
            None,
            Some(&outcome),
            None,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn response_length_at_most_fails_above_boundary() {
        let outcome = make_outcome("hello world"); // len=11
        let result = evaluate_single_for_tests(
            &make_rule(
                0,
                ValidationEvent::ResponseLengthAtMost,
                ValidationKind::ResponseLengthAtMost { length: 5 },
            ),
            None,
            Some(&outcome),
            None,
        );
        assert!(result.is_err());
    }

    #[test]
    fn response_includes_passes_with_substring() {
        let outcome = make_outcome("the quick brown fox");
        let result = evaluate_single_for_tests(
            &make_rule(
                0,
                ValidationEvent::ResponseIncludes,
                ValidationKind::ResponseIncludes {
                    needle: "brown".to_string(),
                },
            ),
            None,
            Some(&outcome),
            None,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn response_missing_fails_with_substring_present() {
        let outcome = make_outcome("operation failed");
        let result = evaluate_single_for_tests(
            &make_rule(
                0,
                ValidationEvent::ResponseMissing,
                ValidationKind::ResponseMissing {
                    needle: "failed".to_string(),
                },
            ),
            None,
            Some(&outcome),
            None,
        );
        assert!(result.is_err());
    }

    // --- Source propagation onto outcomes ---

    #[test]
    fn outcome_carries_rule_source_clone() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("test.txt");
        std::fs::write(&file, "hello").unwrap();

        let source = RuleSource {
            file: std::path::PathBuf::from("/repo/prompts/run.md"),
            line_range: Some(10..=12),
            yaml_snippet: "file_exists: \"test.txt\"\n".to_string(),
        };
        let mut rule = make_rule(
            0,
            ValidationEvent::FileExists,
            ValidationKind::FileExists { file: file.clone() },
        );
        rule.source = Some(source.clone());

        let report = run_checks(
            std::slice::from_ref(&rule),
            None,
            None,
            std::path::Path::new("/tmp/source.md"),
            None,
            FailurePhase::PreCheck,
        );

        assert_eq!(report.outcomes.len(), 1);
        let outcome_source = report.outcomes[0]
            .source
            .as_ref()
            .expect("outcome should carry rule source");
        assert_eq!(outcome_source.file, source.file);
        assert_eq!(outcome_source.line_range, source.line_range);
        assert_eq!(outcome_source.yaml_snippet, source.yaml_snippet);
    }

    #[test]
    fn outcome_source_is_none_when_rule_has_none() {
        let rule = make_rule(
            0,
            ValidationEvent::FileExists,
            ValidationKind::FileExists {
                file: std::path::PathBuf::from("/nonexistent/x.txt"),
            },
        );
        let report = run_checks(
            std::slice::from_ref(&rule),
            None,
            None,
            std::path::Path::new("/tmp/source.md"),
            None,
            FailurePhase::PreCheck,
        );
        assert!(report.outcomes[0].source.is_none());
    }

    // --- Multiple failures collected ---

    #[test]
    fn all_failures_collected_not_short_circuited() {
        let rules = vec![
            make_rule(
                0,
                ValidationEvent::FileExists,
                ValidationKind::FileExists {
                    file: std::path::PathBuf::from("/nonexistent/a.txt"),
                },
            ),
            make_rule(
                1,
                ValidationEvent::FileExists,
                ValidationKind::FileExists {
                    file: std::path::PathBuf::from("/nonexistent/b.txt"),
                },
            ),
            make_rule(
                2,
                ValidationEvent::FileExists,
                ValidationKind::FileExists {
                    file: std::path::PathBuf::from("/nonexistent/c.txt"),
                },
            ),
        ];

        let report = run_checks(
            &rules,
            None,
            None,
            std::path::Path::new("/tmp/source.md"),
            None,
            FailurePhase::PreCheck,
        );
        assert_eq!(
            report.failures().len(),
            3,
            "all failures should be collected"
        );
    }

    // --- Snapshot tests ---

    #[test]
    fn snapshot_captures_blake3_for_file_changed() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("tracked.txt");
        std::fs::write(&file, "content").unwrap();

        let plan = HarnessPlan {
            source_path: dir.path().join("source.md"),
            timeout: None,
            step_timeout: None,
            timeout_warn: None,
            step_timeout_warn: None,
            pre_checks: Vec::new(),
            post_checks: vec![make_rule(
                0,
                ValidationEvent::FileChanged,
                ValidationKind::FileChanged { file: file.clone() },
            )],
            handlers: Default::default(),
            programmatic_handler: None,
        };

        let snapshot = capture_pre_run_snapshot(&plan).unwrap();
        assert!(snapshot.tracked_files.contains_key(&file));
        assert!(snapshot.tracked_files[&file].blake3.is_some());
    }

    #[test]
    fn snapshot_only_captures_referenced_files() {
        let dir = TempDir::new().unwrap();
        let tracked = dir.path().join("tracked.txt");
        let untracked = dir.path().join("untracked.txt");
        std::fs::write(&tracked, "a").unwrap();
        std::fs::write(&untracked, "b").unwrap();

        let plan = HarnessPlan {
            source_path: dir.path().join("source.md"),
            timeout: None,
            step_timeout: None,
            timeout_warn: None,
            step_timeout_warn: None,
            pre_checks: Vec::new(),
            post_checks: vec![make_rule(
                0,
                ValidationEvent::FileChanged,
                ValidationKind::FileChanged {
                    file: tracked.clone(),
                },
            )],
            handlers: Default::default(),
            programmatic_handler: None,
        };

        let snapshot = capture_pre_run_snapshot(&plan).unwrap();
        assert!(snapshot.tracked_files.contains_key(&tracked));
        assert!(!snapshot.tracked_files.contains_key(&untracked));
    }

    // --- Message renderer tests ---

    #[test]
    fn template_substitution_replaces_placeholders() {
        let mut vars = HashMap::new();
        vars.insert("file", "/path/to/file.txt".to_string());
        vars.insert("dir", "/project".to_string());

        let result = render_template("the file {{file}} in {{dir}} exists", &vars);
        assert_eq!(result, "the file /path/to/file.txt in /project exists");
    }

    #[test]
    fn default_message_used_when_msg_is_none() {
        let kind = ValidationKind::FileExists {
            file: std::path::PathBuf::from("/test/file.txt"),
        };
        let vars = build_vars(&kind);
        let msg = default_message(&kind, &vars);
        assert!(msg.contains("/test/file.txt"));
    }

    #[test]
    fn build_check_markup_contains_description_not_glyph() {
        let rule = make_rule(
            0,
            ValidationEvent::FileExists,
            ValidationKind::FileExists {
                file: std::path::PathBuf::from("/test.txt"),
            },
        );
        let markup = build_check_markup(&rule);
        // Status component handles the indicator — no inline glyph
        assert!(!markup.contains('\u{2713}'));
        assert!(!markup.contains('\u{2a2f}'));
        assert!(markup.contains("/test.txt"));
    }

    #[test]
    fn build_check_markup_escapes_user_values() {
        let rule = make_rule(
            0,
            ValidationEvent::FileExists,
            ValidationKind::FileExists {
                file: std::path::PathBuf::from("/path/<evil>.txt"),
            },
        );
        let markup = build_check_markup(&rule);
        assert!(!markup.contains("<evil>"));
        assert!(markup.contains("\\<evil\\>"));
    }

    #[test]
    fn render_template_no_placeholders_returns_unchanged() {
        let vars = HashMap::new();
        let result = render_template("plain message text", &vars);
        assert_eq!(result, "plain message text");
    }

    #[test]
    fn render_template_unknown_bare_variable_is_preserved() {
        // {{response_length}} is intentionally not in build_vars because
        // it depends on outcome data; preserving the token avoids
        // rendering an empty string in default messages.
        let mut vars = HashMap::new();
        vars.insert("length", "10".to_string());

        let result = render_template(
            "response is at least {{length}} characters (actual: {{response_length}})",
            &vars,
        );
        assert_eq!(
            result,
            "response is at least 10 characters (actual: {{response_length}})"
        );
    }

    #[test]
    fn render_template_fallback_uses_default_when_missing() {
        let vars = HashMap::new();
        let result = render_template("path: {{file || \"n/a\"}}", &vars);
        assert_eq!(result, "path: n/a");
    }

    #[test]
    fn render_template_fallback_keeps_value_when_present() {
        let mut vars = HashMap::new();
        vars.insert("file", "/tmp/source.md".to_string());

        let result = render_template("path: {{file || \"n/a\"}}", &vars);
        assert_eq!(result, "path: /tmp/source.md");
    }

    #[test]
    fn render_template_ternary_evaluates_branches() {
        let mut vars = HashMap::new();
        vars.insert("file", "/tmp/source.md".to_string());

        let result = render_template("{{file ? \"present\" : \"missing\"}}", &vars);
        assert_eq!(result, "present");

        let empty = HashMap::new();
        let result = render_template("{{file ? \"present\" : \"missing\"}}", &empty);
        assert_eq!(result, "missing");
    }

    #[test]
    fn render_template_length_helper_works() {
        let mut vars = HashMap::new();
        vars.insert("file", "abc".to_string());

        let result = render_template("{{length(file) > 5 ? \"long\" : \"short\"}}", &vars);
        assert_eq!(result, "short");

        vars.insert("file", "abcdefghij".to_string());
        let result = render_template("{{length(file) > 5 ? \"long\" : \"short\"}}", &vars);
        assert_eq!(result, "long");
    }

    #[test]
    fn render_template_malformed_token_is_preserved() {
        let vars = HashMap::new();
        // `+` is not a supported operator in interpolation mode, so the
        // expression fails to parse and the original token survives.
        let result = render_template("{{a + b}}", &vars);
        assert_eq!(result, "{{a + b}}");
    }

    #[test]
    fn render_template_empty_braces_are_preserved() {
        let vars = HashMap::new();
        let result = render_template("before {{}} after", &vars);
        assert_eq!(result, "before {{}} after");
    }

    // -- Public check_write_permission tests (non-harness inline path) ----

    #[test]
    fn check_write_permission_public_passes_writable_file_no_probe() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("writable.md");
        std::fs::write(&file, "# hello").unwrap();

        let result = check_write_permission(&file, &file, None);
        assert!(result.is_ok());
    }

    #[test]
    fn check_write_permission_public_denies_when_probe_rejects() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("writable.md");
        std::fs::write(&file, "# hello").unwrap();
        let source = dir.path().join("source.md");

        let probe = StaticPermissionProbe {
            assessment: PermissionAssessment::Denied {
                reason: "read-only sandbox".to_string(),
            },
        };

        let result = check_write_permission(&file, &source, Some(&probe));
        let error = result.unwrap_err();
        assert!(error.contains("provider runtime policy denies writes"));
        assert!(error.contains("read-only sandbox"));
    }

    #[test]
    fn check_write_permission_public_denies_missing_parent() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("no-such-parent").join("target.md");
        let source = dir.path().join("source.md");

        let result = check_write_permission(&file, &source, None);
        let error = result.unwrap_err();
        assert!(error.contains("parent directory"));
        assert!(error.contains("does not exist"));
    }
}
