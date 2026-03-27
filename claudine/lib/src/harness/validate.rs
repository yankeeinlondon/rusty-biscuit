//! Validation engine for pre-checks and post-checks.
//!
//! Executes validation rules, captures pre-run snapshots for comparison-based
//! post-checks, and renders success/failure messages.

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::process::Command;

use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::prelude::Renderable;
use biscuit_terminal::terminal::Terminal;

use crate::harness::error::HarnessError;
use crate::harness::model::{
    AttemptOutcome, FileFingerprint, HarnessPlan, PreRunSnapshot, StructuredShape,
    ValidationFailure, ValidationKind, ValidationRule,
};

/// Evaluate all pre-checks in declaration order.
///
/// Runs every rule and collects all failures (does not short-circuit).
/// Returns `Ok(())` if all pass, or `HarnessError::PreCheckFailed` with
/// all failures.
pub fn evaluate_pre_checks(
    plan: &HarnessPlan,
    term: &Terminal,
) -> Result<(), HarnessError> {
    let failures = run_checks(&plan.pre_checks, None, None, None, term);
    if failures.is_empty() {
        Ok(())
    } else {
        Err(HarnessError::PreCheckFailed { failures })
    }
}

/// Capture a pre-run snapshot for subjects referenced by post-checks.
///
/// Only captures state for files and frontmatter properties that are
/// actually referenced, keeping the snapshot small and deterministic.
pub fn capture_pre_run_snapshot(
    plan: &HarnessPlan,
) -> Result<PreRunSnapshot, HarnessError> {
    let mut snapshot = PreRunSnapshot::default();

    for rule in &plan.post_checks {
        match &rule.kind {
            ValidationKind::FileChanged { file } | ValidationKind::FileUnchanged { file } => {
                if !snapshot.tracked_files.contains_key(file) {
                    snapshot
                        .tracked_files
                        .insert(file.clone(), fingerprint_file(file));
                }
            }
            ValidationKind::FrontmatterPropChanged { prop }
            | ValidationKind::FrontmatterPropUnchanged { prop } => {
                if snapshot.source_markdown.is_none() {
                    // Load and parse source document frontmatter
                    if let Ok(text) = fs::read_to_string(&plan.source_path) {
                        let md: darkmatter::markdown::Markdown = text.into();
                        // Capture the specific property values
                        if let Ok(Some(val)) = md.fm_get::<serde_json::Value>(prop) {
                            snapshot
                                .tracked_frontmatter
                                .insert(prop.clone(), val);
                        }
                        snapshot.source_markdown = Some(md);
                    }
                } else if !snapshot.tracked_frontmatter.contains_key(prop) {
                    // Source already loaded, just grab this property
                    if let Some(ref md) = snapshot.source_markdown {
                        if let Ok(Some(val)) = md.fm_get::<serde_json::Value>(prop) {
                            snapshot
                                .tracked_frontmatter
                                .insert(prop.clone(), val);
                        }
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
/// The `source_path` is used to re-read the current on-disk frontmatter
/// for frontmatter comparison checks.
pub fn evaluate_post_checks(
    plan: &HarnessPlan,
    snapshot: &PreRunSnapshot,
    outcome: &AttemptOutcome,
    term: &Terminal,
) -> Result<(), HarnessError> {
    let failures = run_checks(
        &plan.post_checks,
        Some(snapshot),
        Some(outcome),
        Some(&plan.source_path),
        term,
    );
    if failures.is_empty() {
        Ok(())
    } else {
        Err(HarnessError::PostCheckFailed { failures })
    }
}

/// Run a set of checks and return all failures.
fn run_checks(
    rules: &[ValidationRule],
    snapshot: Option<&PreRunSnapshot>,
    outcome: Option<&AttemptOutcome>,
    source_path: Option<&Path>,
    term: &Terminal,
) -> Vec<ValidationFailure> {
    let mut failures = Vec::new();

    // For post-checks involving frontmatter, parse the current on-disk
    // markdown once and share it across all frontmatter checks.
    let post_run_markdown = source_path.and_then(|p| {
        fs::read_to_string(p).ok().map(|text| {
            let md: darkmatter::markdown::Markdown = text.into();
            md
        })
    });

    for rule in rules {
        let result = evaluate_single(rule, snapshot, outcome, post_run_markdown.as_ref());
        let (passed, rendered) = render_check_result(rule, &result, term);
        // Print the check line
        eprintln!("{rendered}");

        if !passed {
            failures.push(ValidationFailure {
                rule_id: rule.id,
                event: rule.event.clone(),
                subject_key: rule.subject_key.clone(),
                message: result.unwrap_err(),
            });
        }
    }

    failures
}

/// Result of evaluating a single validation: `Ok(())` for pass, `Err(message)` for fail.
type CheckResult = Result<(), String>;

/// Evaluate a single validation rule.
fn evaluate_single(
    rule: &ValidationRule,
    snapshot: Option<&PreRunSnapshot>,
    outcome: Option<&AttemptOutcome>,
    post_run_markdown: Option<&darkmatter::markdown::Markdown>,
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
        ValidationKind::JsonFileExists { file, shape } => {
            check_json_file(file, shape.as_ref())
        }
        ValidationKind::YamlFileExists { file, shape } => {
            check_yaml_file(file, shape.as_ref())
        }
        ValidationKind::TomlFileExists { file } => {
            check_toml_file(file)
        }
        ValidationKind::HasWritePermission { file } => {
            check_write_permission(file)
        }
        ValidationKind::ShellCommand { command, show_stdout, show_stderr } => {
            check_shell_command(command, *show_stdout, *show_stderr)
        }

        // --- Git checks ---
        ValidationKind::NoDirtySourceCode { root } => {
            check_dirty_source_code(root, false)
        }
        ValidationKind::HasDirtySourceCode { root } => {
            check_dirty_source_code(root, true)
        }

        // --- Post-only: file comparison ---
        ValidationKind::FileChanged { file } => {
            check_file_changed(file, snapshot, true)
        }
        ValidationKind::FileUnchanged { file } => {
            check_file_changed(file, snapshot, false)
        }

        // --- Post-only: frontmatter comparison ---
        ValidationKind::FrontmatterPropChanged { prop } => {
            check_frontmatter_prop_changed(prop, snapshot, post_run_markdown, true)
        }
        ValidationKind::FrontmatterPropUnchanged { prop } => {
            check_frontmatter_prop_changed(prop, snapshot, post_run_markdown, false)
        }
        ValidationKind::FrontmatterPropEquals { expected } => {
            check_frontmatter_prop_equals(expected, post_run_markdown)
        }

        // --- Post-only: response checks ---
        ValidationKind::ResponseLengthAtLeast { length } => {
            let resp = outcome.map(|o| o.final_response.chars().count()).unwrap_or(0);
            if resp >= *length {
                Ok(())
            } else {
                Err(format!(
                    "response length {resp} is less than required {length}"
                ))
            }
        }
        ValidationKind::ResponseLengthAtMost { length } => {
            let resp = outcome.map(|o| o.final_response.chars().count()).unwrap_or(0);
            if resp <= *length {
                Ok(())
            } else {
                Err(format!(
                    "response length {resp} exceeds maximum {length}"
                ))
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

// ---------------------------------------------------------------------------
// Individual check implementations
// ---------------------------------------------------------------------------

fn check_json_file(file: &Path, shape: Option<&StructuredShape>) -> CheckResult {
    if !file.exists() || file.is_dir() {
        return Err(format!("file does not exist: {}", file.display()));
    }
    let content = fs::read_to_string(file)
        .map_err(|e| format!("cannot read {}: {e}", file.display()))?;
    let value: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("{} is not valid JSON: {e}", file.display()))?;
    if let Some(expected_shape) = shape {
        check_value_shape(&value, expected_shape, file)?;
    }
    Ok(())
}

fn check_yaml_file(file: &Path, shape: Option<&StructuredShape>) -> CheckResult {
    if !file.exists() || file.is_dir() {
        return Err(format!("file does not exist: {}", file.display()));
    }
    let content = fs::read_to_string(file)
        .map_err(|e| format!("cannot read {}: {e}", file.display()))?;
    let yaml = biscuit_file::Yaml::from_str(&content)
        .map_err(|e| format!("{} is not valid YAML: {e}", file.display()))?;
    if let Some(expected_shape) = shape {
        let json_value = yaml.as_json()
            .map_err(|e| format!("{} YAML-to-JSON conversion failed: {e}", file.display()))?;
        check_value_shape(&json_value, expected_shape, file)?;
    }
    Ok(())
}

fn check_toml_file(file: &Path) -> CheckResult {
    if !file.exists() || file.is_dir() {
        return Err(format!("file does not exist: {}", file.display()));
    }
    let content = fs::read_to_string(file)
        .map_err(|e| format!("cannot read {}: {e}", file.display()))?;
    let _: toml::Value = toml::from_str(&content)
        .map_err(|e| format!("{} is not valid TOML: {e}", file.display()))?;
    Ok(())
}

fn check_value_shape(
    value: &serde_json::Value,
    expected: &StructuredShape,
    file: &Path,
) -> CheckResult {
    let actual = match value {
        serde_json::Value::Array(_) => StructuredShape::Array,
        serde_json::Value::Object(_) => StructuredShape::Object,
        _ => StructuredShape::Scalar,
    };
    if actual == *expected {
        Ok(())
    } else {
        Err(format!(
            "{}: root shape is {actual:?} but expected {expected:?}",
            file.display()
        ))
    }
}

fn check_write_permission(file: &Path) -> CheckResult {
    // OS writability check
    match std::fs::OpenOptions::new().write(true).open(file) {
        Ok(_) => Ok(()),
        Err(e) => Err(format!(
            "no write permission for {}: {e}",
            file.display()
        )),
    }
}

fn check_shell_command(
    command: &crate::harness::model::ApprovedRuntimeCommand,
    show_stdout: bool,
    show_stderr: bool,
) -> CheckResult {
    let timeout = std::time::Duration::from_secs(60);
    let (exit_code, stdout, stderr) =
        crate::harness::shell::execute_approved_command(command, None, timeout).map_err(|e| {
            format!("shell command '{}' failed: {e}", command.raw)
        })?;

    if show_stdout && !stdout.trim().is_empty() {
        eprintln!("{stdout}");
    }
    if show_stderr && !stderr.trim().is_empty() {
        eprintln!("{stderr}");
    }

    if exit_code == 0 {
        Ok(())
    } else {
        Err(format!(
            "shell command '{}' exited with code {exit_code}",
            command.raw
        ))
    }
}

/// Known source code extensions for dirty-source-code checks.
const SOURCE_EXTENSIONS: &[&str] = &[
    // Rust
    "rs",
    // JS/TS
    "js", "jsx", "ts", "tsx", "mjs", "cjs",
    // Python
    "py",
    // Go
    "go",
    // JVM
    "java", "kt",
    // Web
    "css", "scss", "html",
    // Shell
    "sh", "bash", "zsh",
];

/// Known source-adjacent filenames.
const SOURCE_FILENAMES: &[&str] = &["justfile", "Cargo.toml", "package.json"];

fn check_dirty_source_code(root: &Path, expect_dirty: bool) -> CheckResult {
    // Find repo root by walking up from the specified root
    let repo_root = find_git_repo_root(root).ok_or_else(|| {
        format!("no git repository found at or above {}", root.display())
    })?;

    let output = Command::new("git")
        .args(["status", "--porcelain", "--"])
        .arg(root)
        .current_dir(&repo_root)
        .output()
        .map_err(|e| format!("failed to run git status: {e}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let dirty_files: Vec<&str> = stdout
        .lines()
        .filter(|line| {
            // git status --porcelain format: XY filename
            let filename = line.get(3..).unwrap_or("").trim();
            is_source_file(filename)
        })
        .collect();

    if expect_dirty {
        if dirty_files.is_empty() {
            Err("no dirty source code found".to_string())
        } else {
            Ok(())
        }
    } else if dirty_files.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "dirty source code found: {}",
            dirty_files
                .iter()
                .map(|l| l.get(3..).unwrap_or("").trim())
                .collect::<Vec<_>>()
                .join(", ")
        ))
    }
}

fn is_source_file(path: &str) -> bool {
    let filename = path.rsplit('/').next().unwrap_or(path);
    if SOURCE_FILENAMES.contains(&filename) {
        return true;
    }
    if let Some(ext) = path.rsplit('.').next() {
        SOURCE_EXTENSIONS.contains(&ext)
    } else {
        false
    }
}

fn find_git_repo_root(start: &Path) -> Option<std::path::PathBuf> {
    let mut current = if start.is_file() {
        start.parent()?.to_path_buf()
    } else {
        start.to_path_buf()
    };
    loop {
        if current.join(".git").exists() {
            return Some(current);
        }
        if !current.pop() {
            return None;
        }
    }
}

// ---------------------------------------------------------------------------
// Post-check comparison helpers
// ---------------------------------------------------------------------------

fn fingerprint_file(path: &Path) -> FileFingerprint {
    let exists = path.exists();
    let is_dir = path.is_dir();
    let blake3 = if exists && !is_dir {
        fs::read(path).ok().map(|bytes| {
            let hash_bytes = biscuit_hash::blake3_hash_bytes(&bytes);
            hash_bytes
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
        })
    } else {
        None
    };
    FileFingerprint {
        exists,
        is_dir,
        blake3,
    }
}

fn check_file_changed(
    file: &Path,
    snapshot: Option<&PreRunSnapshot>,
    expect_changed: bool,
) -> CheckResult {
    let snapshot = snapshot.ok_or("internal error: no pre-run snapshot for file comparison")?;
    let pre = snapshot.tracked_files.get(file).ok_or_else(|| {
        format!("internal error: file {} not tracked in snapshot", file.display())
    })?;
    let post = fingerprint_file(file);

    let changed = pre.blake3 != post.blake3;
    if expect_changed {
        if changed {
            Ok(())
        } else {
            Err(format!("file {} was not modified", file.display()))
        }
    } else if changed {
        Err(format!("file {} was unexpectedly modified", file.display()))
    } else {
        Ok(())
    }
}

fn check_frontmatter_prop_changed(
    prop: &str,
    snapshot: Option<&PreRunSnapshot>,
    post_run_markdown: Option<&darkmatter::markdown::Markdown>,
    expect_changed: bool,
) -> CheckResult {
    let snapshot = snapshot.ok_or("internal error: no pre-run snapshot for frontmatter comparison")?;
    let pre_value = snapshot.tracked_frontmatter.get(prop);

    // Read the current on-disk post-state from the post-run markdown.
    let post_md = post_run_markdown.ok_or(
        "internal error: post-run markdown not available for frontmatter comparison",
    )?;

    let post_value = post_md
        .fm_get::<serde_json::Value>(prop)
        .ok()
        .flatten();

    let changed = pre_value != post_value.as_ref();
    if expect_changed {
        if changed {
            Ok(())
        } else {
            Err(format!("frontmatter property \"{prop}\" was not modified"))
        }
    } else if changed {
        Err(format!(
            "frontmatter property \"{prop}\" was unexpectedly modified"
        ))
    } else {
        Ok(())
    }
}

fn check_frontmatter_prop_equals(
    expected: &indexmap::IndexMap<String, serde_json::Value>,
    post_run_markdown: Option<&darkmatter::markdown::Markdown>,
) -> CheckResult {
    let post_md = post_run_markdown.ok_or(
        "internal error: post-run markdown not available for frontmatter equals check",
    )?;

    let mut mismatches = Vec::new();
    for (key, expected_val) in expected {
        let actual = post_md.fm_get::<serde_json::Value>(key).ok().flatten();
        match actual {
            Some(ref actual_val) if actual_val == expected_val => {}
            Some(actual_val) => {
                mismatches.push(format!("{key}: expected {expected_val}, got {actual_val}"));
            }
            None => {
                mismatches.push(format!("{key}: expected {expected_val}, but property is missing"));
            }
        }
    }

    if mismatches.is_empty() {
        Ok(())
    } else {
        Err(format!("frontmatter mismatch: {}", mismatches.join("; ")))
    }
}

// ---------------------------------------------------------------------------
// Message rendering
// ---------------------------------------------------------------------------

/// Default message templates per validation kind.
fn default_message(kind: &ValidationKind, vars: &HashMap<&str, String>) -> String {
    let template = match kind {
        ValidationKind::FileExists { .. } => "{{status}} the file {{file}} exists",
        ValidationKind::DirExists { .. } => "{{status}} the directory {{dir}} exists",
        ValidationKind::JsonFileExists { .. } => "{{status}} {{file}} is a valid JSON file",
        ValidationKind::YamlFileExists { .. } => "{{status}} {{file}} is a valid YAML file",
        ValidationKind::TomlFileExists { .. } => "{{status}} {{file}} is a valid TOML file",
        ValidationKind::HasWritePermission { .. } => "{{status}} write permission for {{file}}",
        ValidationKind::ShellCommand { .. } => "{{status}} shell command: {{command}}",
        ValidationKind::NoDirtySourceCode { .. } => "{{status}} no dirty source code in {{dir}}",
        ValidationKind::HasDirtySourceCode { .. } => "{{status}} dirty source code found in {{dir}}",
        ValidationKind::FileChanged { .. } => "{{status}} the file {{file}} was modified",
        ValidationKind::FileUnchanged { .. } => "{{status}} the file {{file}} was not modified",
        ValidationKind::FrontmatterPropChanged { .. } => "{{status}} frontmatter property \"{{prop}}\" was modified",
        ValidationKind::FrontmatterPropUnchanged { .. } => "{{status}} frontmatter property \"{{prop}}\" was not modified",
        ValidationKind::FrontmatterPropEquals { .. } => "{{status}} frontmatter properties match expected values",
        ValidationKind::ResponseLengthAtLeast { .. } => "{{status}} response is at least {{length}} characters (actual: {{response_length}})",
        ValidationKind::ResponseLengthAtMost { .. } => "{{status}} response is at most {{length}} characters (actual: {{response_length}})",
        ValidationKind::ResponseIncludes { .. } => "{{status}} response includes \"{{expected}}\"",
        ValidationKind::ResponseMissing { .. } => "{{status}} response does not include \"{{expected}}\"",
    };
    render_template(template, vars)
}

/// Build the template variable map for a validation rule.
fn build_vars<'a>(kind: &'a ValidationKind, status: &'a str) -> HashMap<&'a str, String> {
    let mut vars: HashMap<&str, String> = HashMap::new();
    vars.insert("status", status.to_string());

    match kind {
        ValidationKind::FileExists { file }
        | ValidationKind::JsonFileExists { file, .. }
        | ValidationKind::YamlFileExists { file, .. }
        | ValidationKind::TomlFileExists { file }
        | ValidationKind::HasWritePermission { file }
        | ValidationKind::FileChanged { file }
        | ValidationKind::FileUnchanged { file } => {
            vars.insert("file", file.display().to_string());
        }
        ValidationKind::DirExists { dir }
        | ValidationKind::NoDirtySourceCode { root: dir }
        | ValidationKind::HasDirtySourceCode { root: dir } => {
            vars.insert("dir", dir.display().to_string());
        }
        ValidationKind::ShellCommand { command, .. } => {
            vars.insert("command", command.raw.clone());
        }
        ValidationKind::FrontmatterPropChanged { prop }
        | ValidationKind::FrontmatterPropUnchanged { prop } => {
            vars.insert("prop", prop.clone());
        }
        ValidationKind::FrontmatterPropEquals { .. } => {}
        ValidationKind::ResponseLengthAtLeast { length }
        | ValidationKind::ResponseLengthAtMost { length } => {
            vars.insert("length", length.to_string());
        }
        ValidationKind::ResponseIncludes { needle }
        | ValidationKind::ResponseMissing { needle } => {
            vars.insert("expected", needle.clone());
        }
    }

    vars
}

/// Render a check result into a formatted line.
///
/// Returns `(passed, rendered_string)`.
fn render_check_result(
    rule: &ValidationRule,
    result: &CheckResult,
    term: &Terminal,
) -> (bool, String) {
    let passed = result.is_ok();
    let status_token = if passed {
        "<b><green-500>\u{2713}</green-500></b>"
    } else {
        "<b><red-500>\u{2a2f}</red-500></b>"
    };

    let vars = build_vars(&rule.kind, status_token);

    let message = if let Some(ref tmpl) = rule.message_template {
        render_template(tmpl, &vars)
    } else {
        default_message(&rule.kind, &vars)
    };

    let rendered = Prose::new(&message).render(term);
    (passed, rendered)
}

/// Simple Handlebars-style template renderer: replaces `{{key}}` with values.
fn render_template(template: &str, vars: &HashMap<&str, String>) -> String {
    let mut result = template.to_string();
    for (key, value) in vars {
        result = result.replace(&format!("{{{{{key}}}}}"), value);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::harness::model::{
        ProcessTermination, ValidationEvent, ValidationPhase, ValidationRuleId,
    };
    use tempfile::TempDir;

    fn make_rule(id: u32, event: ValidationEvent, kind: ValidationKind) -> ValidationRule {
        ValidationRule {
            id: ValidationRuleId(id),
            event,
            phase: ValidationPhase::Both,
            kind,
            message_template: None,
            subject_key: None,
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

    // --- Filesystem checks ---

    #[test]
    fn file_exists_passes_for_existing_file() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("test.txt");
        fs::write(&file, "hello").unwrap();

        let result = evaluate_single(
            &make_rule(0, ValidationEvent::FileExists, ValidationKind::FileExists { file }),
            None,
            None,
            None,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn file_exists_fails_for_missing_file() {
        let result = evaluate_single(
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
        let result = evaluate_single(
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
        fs::write(&file, r#"{"key": "value"}"#).unwrap();

        let result = evaluate_single(
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
        fs::write(&file, "not json at all {").unwrap();

        let result = evaluate_single(
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
    fn yaml_file_exists_passes_for_valid_yaml_with_shape() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("data.yaml");
        fs::write(&file, "- item1\n- item2\n").unwrap();

        let result = evaluate_single(
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
        fs::write(&file, "{{not toml}}").unwrap();

        let result = evaluate_single(
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
        fs::write(&file, "original content").unwrap();

        let pre_fingerprint = fingerprint_file(&file);
        let mut snapshot = PreRunSnapshot::default();
        snapshot.tracked_files.insert(file.clone(), pre_fingerprint);

        // Modify the file
        fs::write(&file, "modified content").unwrap();

        let result = evaluate_single(
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
        fs::write(&file, "same content").unwrap();

        let pre_fingerprint = fingerprint_file(&file);
        let mut snapshot = PreRunSnapshot::default();
        snapshot.tracked_files.insert(file.clone(), pre_fingerprint);

        // Don't modify — same hash
        let result = evaluate_single(
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
        fs::write(&file, "same content").unwrap();

        let pre_fingerprint = fingerprint_file(&file);
        let mut snapshot = PreRunSnapshot::default();
        snapshot.tracked_files.insert(file.clone(), pre_fingerprint);

        let result = evaluate_single(
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
        let result = evaluate_single(
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
        let result = evaluate_single(
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
        let result = evaluate_single(
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
        let result = evaluate_single(
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

        let term = Terminal::default();
        let failures = run_checks(&rules, None, None, None, &term);
        assert_eq!(failures.len(), 3, "all failures should be collected");
    }

    // --- Snapshot tests ---

    #[test]
    fn snapshot_captures_blake3_for_file_changed() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("tracked.txt");
        fs::write(&file, "content").unwrap();

        let plan = HarnessPlan {
            source_path: dir.path().join("source.md"),
            timeout: None,
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
        fs::write(&tracked, "a").unwrap();
        fs::write(&untracked, "b").unwrap();

        let plan = HarnessPlan {
            source_path: dir.path().join("source.md"),
            timeout: None,
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
        vars.insert("status", "OK".to_string());
        vars.insert("file", "/path/to/file.txt".to_string());

        let result = render_template("{{status}} the file {{file}} exists", &vars);
        assert_eq!(result, "OK the file /path/to/file.txt exists");
    }

    #[test]
    fn default_message_used_when_msg_is_none() {
        let kind = ValidationKind::FileExists {
            file: std::path::PathBuf::from("/test/file.txt"),
        };
        let vars = build_vars(&kind, "OK");
        let msg = default_message(&kind, &vars);
        assert!(msg.contains("/test/file.txt"));
        assert!(msg.contains("OK"));
    }

    #[test]
    fn render_check_result_success_token() {
        let term = Terminal::default();
        let rule = make_rule(
            0,
            ValidationEvent::FileExists,
            ValidationKind::FileExists {
                file: std::path::PathBuf::from("/test.txt"),
            },
        );
        let (passed, rendered) = render_check_result(&rule, &Ok(()), &term);
        assert!(passed);
        // The rendered output should contain the check mark character
        assert!(rendered.contains('\u{2713}'));
    }

    #[test]
    fn render_check_result_failure_token() {
        let term = Terminal::default();
        let rule = make_rule(
            0,
            ValidationEvent::FileExists,
            ValidationKind::FileExists {
                file: std::path::PathBuf::from("/test.txt"),
            },
        );
        let (passed, rendered) =
            render_check_result(&rule, &Err("not found".to_string()), &term);
        assert!(!passed);
        // The rendered output should contain the cross mark character
        assert!(rendered.contains('\u{2a2f}'));
    }
}
