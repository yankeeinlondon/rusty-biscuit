//! Validation engine for pre-checks and post-checks.
//!
//! Executes validation rules, captures pre-run snapshots for comparison-based
//! post-checks, and renders success/failure messages.

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::process::Command;

use crate::harness::error::HarnessError;
use crate::harness::model::{
    AttemptOutcome, FailurePhase, FileFingerprint, HarnessPermissionProbe, HarnessPlan,
    PermissionAssessment, PreRunSnapshot, StructuredShape, ValidationCheckOutcome, ValidationKind,
    ValidationPhaseReport, ValidationRule,
};

/// Evaluate all pre-checks in declaration order.
///
/// Runs every rule and collects all outcomes (does not short-circuit).
/// Returns a `ValidationPhaseReport`; the caller decides whether failures
/// are fatal and controls rendering.
pub fn evaluate_pre_checks(
    plan: &HarnessPlan,
    permission_probe: Option<&dyn HarnessPermissionProbe>,
) -> ValidationPhaseReport {
    run_checks(
        &plan.pre_checks,
        None,
        None,
        &plan.source_path,
        permission_probe,
        FailurePhase::PreCheck,
    )
}

/// Capture a pre-run snapshot for subjects referenced by post-checks.
///
/// Only captures state for files and frontmatter properties that are
/// actually referenced, keeping the snapshot small and deterministic.
pub fn capture_pre_run_snapshot(plan: &HarnessPlan) -> Result<PreRunSnapshot, HarnessError> {
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
    run_checks(
        &plan.post_checks,
        Some(snapshot),
        Some(outcome),
        &plan.source_path,
        permission_probe,
        FailurePhase::PostCheck,
    )
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
    let post_run_markdown = outcome.map(|_| match fs::read_to_string(source_path) {
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
        ValidationKind::JsonFileExists { file, shape } => check_json_file(file, shape.as_ref()),
        ValidationKind::YamlFileExists { file, shape } => check_yaml_file(file, shape.as_ref()),
        ValidationKind::TomlFileExists { file } => check_toml_file(file),
        ValidationKind::HasWritePermission { file } => {
            check_write_permission(file, source_path, permission_probe)
        }
        ValidationKind::ShellCommand {
            command,
            show_stdout,
            show_stderr,
        } => check_shell_command(command, *show_stdout, *show_stderr),

        // --- Git checks ---
        ValidationKind::NoDirtySourceCode { root } => check_dirty_source_code(root, false),
        ValidationKind::HasDirtySourceCode { root } => check_dirty_source_code(root, true),

        // --- Post-only: file comparison ---
        ValidationKind::FileChanged { file } => check_file_changed(file, snapshot, true),
        ValidationKind::FileUnchanged { file } => check_file_changed(file, snapshot, false),

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

// ---------------------------------------------------------------------------
// Individual check implementations
// ---------------------------------------------------------------------------

fn check_json_file(file: &Path, shape: Option<&StructuredShape>) -> CheckResult {
    if !file.exists() || file.is_dir() {
        return Err(format!("file does not exist: {}", file.display()));
    }
    let content =
        fs::read_to_string(file).map_err(|e| format!("cannot read {}: {e}", file.display()))?;
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
    let content =
        fs::read_to_string(file).map_err(|e| format!("cannot read {}: {e}", file.display()))?;
    let yaml = biscuit_file::Yaml::from_str(&content)
        .map_err(|e| format!("{} is not valid YAML: {e}", file.display()))?;
    if let Some(expected_shape) = shape {
        let json_value = yaml
            .as_json()
            .map_err(|e| format!("{} YAML-to-JSON conversion failed: {e}", file.display()))?;
        check_value_shape(&json_value, expected_shape, file)?;
    }
    Ok(())
}

fn check_toml_file(file: &Path) -> CheckResult {
    if !file.exists() || file.is_dir() {
        return Err(format!("file does not exist: {}", file.display()));
    }
    let content =
        fs::read_to_string(file).map_err(|e| format!("cannot read {}: {e}", file.display()))?;
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

/// Check that `file` is writable both at the filesystem level and under the
/// provider's runtime write policy (when a probe is supplied).
///
/// This is the same check used by the harness `has_write_permission`
/// validation. Exposed publicly so that non-harness inline composition can
/// enforce the same invariant.
pub fn check_write_permission(
    file: &Path,
    source_path: &Path,
    permission_probe: Option<&dyn HarnessPermissionProbe>,
) -> CheckResult {
    match std::fs::metadata(file) {
        Ok(metadata) => {
            if metadata.is_dir() {
                return Err(format!(
                    "{} is a directory; expected a writable file path",
                    file.display()
                ));
            }

            std::fs::OpenOptions::new()
                .write(true)
                .open(file)
                .map(|_| ())
                .map_err(|e| {
                    format!(
                        "cannot write existing file {}: {e} (filesystem write check failed; provider runtime policy may also deny writes)",
                        file.display()
                    )
                })?;
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            check_parent_allows_file_creation(file)?;
        }
        Err(e) => Err(format!(
            "cannot inspect {} for write access: {e}",
            file.display()
        ))?,
    }

    evaluate_provider_write_policy(file, source_path, permission_probe)
}

fn evaluate_provider_write_policy(
    file: &Path,
    source_path: &Path,
    permission_probe: Option<&dyn HarnessPermissionProbe>,
) -> CheckResult {
    let Some(probe) = permission_probe else {
        return Ok(());
    };

    match probe.can_write(file, source_path) {
        PermissionAssessment::Allowed => Ok(()),
        PermissionAssessment::Denied { reason } => Err(format!(
            "provider runtime policy denies writes to {}: {reason}",
            file.display()
        )),
        PermissionAssessment::Unknown { reason } => Err(format!(
            "provider runtime policy for {} is unknown: {reason}",
            file.display()
        )),
    }
}

fn check_parent_allows_file_creation(file: &Path) -> CheckResult {
    let parent = file
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));

    if !parent.exists() {
        return Err(format!(
            "cannot create {}: parent directory {} does not exist",
            file.display(),
            parent.display()
        ));
    }

    if !parent.is_dir() {
        return Err(format!(
            "cannot create {}: parent path {} is not a directory",
            file.display(),
            parent.display()
        ));
    }

    let probe_path = parent.join(format!(
        ".claudine-write-probe-{}-{}",
        std::process::id(),
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
    ));

    match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&probe_path)
    {
        Ok(_) => {
            let _ = std::fs::remove_file(&probe_path);
            Ok(())
        }
        Err(e) => Err(format!(
            "cannot create {} in {}: {e} (filesystem write check failed; provider runtime policy may also deny writes)",
            file.display(),
            parent.display()
        )),
    }
}

fn check_shell_command(
    command: &crate::harness::model::ApprovedRuntimeCommand,
    show_stdout: bool,
    show_stderr: bool,
) -> CheckResult {
    let timeout = std::time::Duration::from_secs(60);
    let (exit_code, stdout, stderr) = tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(
            crate::harness::shell::execute_approved_command(command, None, timeout),
        )
    })
    .map_err(|e| format!("shell command '{}' failed: {e}", command.raw))?;

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
    "rs", // JS/TS
    "js", "jsx", "ts", "tsx", "mjs", "cjs", // Python
    "py",  // Go
    "go",  // JVM
    "java", "kt", // Web
    "css", "scss", "html", // Shell
    "sh", "bash", "zsh",
];

/// Known source-adjacent filenames.
const SOURCE_FILENAMES: &[&str] = &["justfile", "Cargo.toml", "package.json"];

fn check_dirty_source_code(root: &Path, expect_dirty: bool) -> CheckResult {
    // Find repo root by walking up from the specified root
    let repo_root = find_git_repo_root(root)
        .ok_or_else(|| format!("no git repository found at or above {}", root.display()))?;

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
        format!(
            "internal error: file {} not tracked in snapshot",
            file.display()
        )
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
    post_run_markdown: Option<&PostRunMarkdownState>,
    expect_changed: bool,
) -> CheckResult {
    let snapshot =
        snapshot.ok_or("internal error: no pre-run snapshot for frontmatter comparison")?;
    let pre_value = snapshot.tracked_frontmatter.get(prop);

    // Read the current on-disk post-state from the post-run markdown.
    let post_md = get_post_run_markdown(post_run_markdown, "frontmatter comparison")?;

    let post_value = post_md.fm_get::<serde_json::Value>(prop).ok().flatten();

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
    post_run_markdown: Option<&PostRunMarkdownState>,
) -> CheckResult {
    let post_md = get_post_run_markdown(post_run_markdown, "frontmatter equals check")?;

    let mut mismatches = Vec::new();
    for (key, expected_val) in expected {
        let actual = post_md.fm_get::<serde_json::Value>(key).ok().flatten();
        match actual {
            Some(ref actual_val) if actual_val == expected_val => {}
            Some(actual_val) => {
                mismatches.push(format!("{key}: expected {expected_val}, got {actual_val}"));
            }
            None => {
                mismatches.push(format!(
                    "{key}: expected {expected_val}, but property is missing"
                ));
            }
        }
    }

    if mismatches.is_empty() {
        Ok(())
    } else {
        Err(format!("frontmatter mismatch: {}", mismatches.join("; ")))
    }
}

fn get_post_run_markdown<'a>(
    post_run_markdown: Option<&'a PostRunMarkdownState>,
    context: &str,
) -> Result<&'a darkmatter::markdown::Markdown, String> {
    match post_run_markdown {
        Some(PostRunMarkdownState::Loaded(markdown)) => Ok(markdown),
        Some(PostRunMarkdownState::ReadFailed { path, error }) => Err(format!(
            "failed to read {} for {context}: {error}",
            path.display()
        )),
        None => Err(format!(
            "internal error: post-run markdown not available for {context}"
        )),
    }
}

// ---------------------------------------------------------------------------
// Message rendering
// ---------------------------------------------------------------------------

/// Default message templates per validation kind.
fn default_message(kind: &ValidationKind, vars: &HashMap<&str, String>) -> String {
    let template = match kind {
        ValidationKind::FileExists { .. } => "the file {{file}} exists",
        ValidationKind::DirExists { .. } => "the directory {{dir}} exists",
        ValidationKind::JsonFileExists { .. } => "{{file}} is a valid JSON file",
        ValidationKind::YamlFileExists { .. } => "{{file}} is a valid YAML file",
        ValidationKind::TomlFileExists { .. } => "{{file}} is a valid TOML file",
        ValidationKind::HasWritePermission { .. } => "write permission for {{file}}",
        ValidationKind::ShellCommand { .. } => "shell command: {{command}}",
        ValidationKind::NoDirtySourceCode { .. } => "no dirty source code in {{dir}}",
        ValidationKind::HasDirtySourceCode { .. } => "dirty source code found in {{dir}}",
        ValidationKind::FileChanged { .. } => "the file {{file}} was modified",
        ValidationKind::FileUnchanged { .. } => "the file {{file}} was not modified",
        ValidationKind::FrontmatterPropChanged { .. } => {
            "frontmatter property \"{{prop}}\" was modified"
        }
        ValidationKind::FrontmatterPropUnchanged { .. } => {
            "frontmatter property \"{{prop}}\" was not modified"
        }
        ValidationKind::FrontmatterPropEquals { .. } => {
            "frontmatter properties match expected values"
        }
        ValidationKind::ResponseLengthAtLeast { .. } => {
            "response is at least {{length}} characters (actual: {{response_length}})"
        }
        ValidationKind::ResponseLengthAtMost { .. } => {
            "response is at most {{length}} characters (actual: {{response_length}})"
        }
        ValidationKind::ResponseIncludes { .. } => "response includes \"{{expected}}\"",
        ValidationKind::ResponseMissing { .. } => "response does not include \"{{expected}}\"",
    };
    render_template(template, vars)
}

/// Build the template variable map for a validation rule.
///
/// All user-controlled values are escaped for safe Prose interpolation.
fn build_vars(kind: &ValidationKind) -> HashMap<&str, String> {
    use crate::harness::report::prose_escape;

    let mut vars: HashMap<&str, String> = HashMap::new();

    match kind {
        ValidationKind::FileExists { file }
        | ValidationKind::JsonFileExists { file, .. }
        | ValidationKind::YamlFileExists { file, .. }
        | ValidationKind::TomlFileExists { file }
        | ValidationKind::HasWritePermission { file }
        | ValidationKind::FileChanged { file }
        | ValidationKind::FileUnchanged { file } => {
            vars.insert("file", prose_escape(&file.display().to_string()));
        }
        ValidationKind::DirExists { dir }
        | ValidationKind::NoDirtySourceCode { root: dir }
        | ValidationKind::HasDirtySourceCode { root: dir } => {
            vars.insert("dir", prose_escape(&dir.display().to_string()));
        }
        ValidationKind::ShellCommand { command, .. } => {
            vars.insert("command", prose_escape(&command.raw));
        }
        ValidationKind::FrontmatterPropChanged { prop }
        | ValidationKind::FrontmatterPropUnchanged { prop } => {
            vars.insert("prop", prose_escape(prop));
        }
        ValidationKind::FrontmatterPropEquals { .. } => {}
        ValidationKind::ResponseLengthAtLeast { length }
        | ValidationKind::ResponseLengthAtMost { length } => {
            vars.insert("length", length.to_string());
        }
        ValidationKind::ResponseIncludes { needle }
        | ValidationKind::ResponseMissing { needle } => {
            vars.insert("expected", prose_escape(needle));
        }
    }

    vars
}

/// Build the prose-ready markup string for a check result.
///
/// Returns a markup body suitable for `Status::from_prose(...)`.
/// Does not render to a terminal. The `Status` component provides the
/// pass/fail indicator via `StatusState`, so no inline glyph is emitted.
pub(crate) fn build_check_markup(rule: &ValidationRule) -> String {
    let vars = build_vars(&rule.kind);

    if let Some(ref tmpl) = rule.message_template {
        render_template(tmpl, &vars)
    } else {
        default_message(&rule.kind, &vars)
    }
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
        PermissionAssessment, ProcessTermination, ValidationEvent, ValidationPhase,
        ValidationRuleId,
    };
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

    // --- Filesystem checks ---

    #[test]
    fn file_exists_passes_for_existing_file() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("test.txt");
        fs::write(&file, "hello").unwrap();

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
        fs::write(&file, r#"{"key": "value"}"#).unwrap();

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
        fs::write(&file, "not json at all {").unwrap();

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
        fs::write(&file, "hello").unwrap();
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
        fs::write(&file, "- item1\n- item2\n").unwrap();

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
        fs::write(&file, "{{not toml}}").unwrap();

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
        fs::write(&file, "original content").unwrap();

        let pre_fingerprint = fingerprint_file(&file);
        let mut snapshot = PreRunSnapshot::default();
        snapshot.tracked_files.insert(file.clone(), pre_fingerprint);

        // Modify the file
        fs::write(&file, "modified content").unwrap();

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
        fs::write(&file, "same content").unwrap();

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
        fs::write(&file, "same content").unwrap();

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

    // -- Public check_write_permission tests (non-harness inline path) ----

    #[test]
    fn check_write_permission_public_passes_writable_file_no_probe() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("writable.md");
        fs::write(&file, "# hello").unwrap();

        let result = check_write_permission(&file, &file, None);
        assert!(result.is_ok());
    }

    #[test]
    fn check_write_permission_public_denies_when_probe_rejects() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("writable.md");
        fs::write(&file, "# hello").unwrap();
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
