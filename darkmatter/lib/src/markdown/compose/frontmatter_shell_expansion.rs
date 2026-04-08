//! Frontmatter shell expansion parsing and validation.
//!
//! Scans top-level frontmatter string values for `$(cmd)` shell expressions,
//! parses them, validates the executable-token interpolation rule, and returns
//! structured directives ready for execution.

use super::shell_expansion::store::resolve_policy_paths;
use super::shell_expansion::tokenize::tokenize;
use super::shell_expansion::types::{
    ErrorHandling, PipelineRuntime, ShellCommandOrigin, ShellDirective, ShellExpansionError,
};
use super::types::{ComposeOptions, ComposeWarning};
use crate::markdown::frontmatter::Frontmatter;
use crate::markdown::types::MarkdownResult;
use serde_json::Value;
use std::collections::HashMap;

/// A parsed frontmatter shell directive ready for execution.
#[derive(Debug, Clone)]
pub(crate) struct FrontmatterShellDirective {
    /// The frontmatter key this directive was extracted from.
    pub key: String,
    /// The raw template string (post-interpolation value).
    pub raw_template: String,
    /// The command string between $( and ).
    pub raw_command: String,
    /// The resolved executable name.
    pub executable: String,
    /// The resolved arguments.
    pub args: Vec<String>,
    /// Per-command timeout override from ::timeout:N suffix.
    pub timeout_override: Option<std::time::Duration>,
}

/// Result of frontmatter shell expansion.
pub(crate) struct FrontmatterShellExpansionReport {
    /// Number of frontmatter values rewritten.
    pub replacements: usize,
    /// Number of shell approvals consumed.
    pub approvals_used: usize,
    /// Warnings emitted.
    pub warnings: Vec<ComposeWarning>,
}

/// Parses a single frontmatter string value for shell expression.
///
/// Returns `Some` if the value matches `$(cmd)` or `$(cmd)::timeout:N`.
///
/// ## Rules
///
/// - Value must start with `$(`
/// - Must have a closing `)` — either at the end, or followed by `::timeout:N`
/// - If `::timeout:N` suffix present: extract it, validate N > 0
/// - Extract the inner command string between `$(` and `)`
/// - Tokenize with the existing tokenizer
/// - If `original_value` is provided (pre-interpolation snapshot), check the
///   executable-token interpolation rule:
///   - Find the executable portion in the ORIGINAL string (first token between
///     `$(` and first whitespace)
///   - If that portion contains `{{` and `}}`, reject (return None)
///   - Also check after pipe/chain operators in the original for executable-position
///     interpolation
/// - Return None for any parse failure (frontmatter shell expressions silently
///   skip invalid values)
pub(crate) fn parse_shell_value(
    value: &str,
    key: &str,
    original_value: Option<&str>,
) -> Option<FrontmatterShellDirective> {
    // Must start with $(
    if !value.starts_with("$(") {
        return None;
    }

    // Find the closing )
    let rest = &value[2..];
    let close_pos = rest.find(')')?;

    let inner_command = &rest[..close_pos];
    let after_close = &rest[close_pos + 1..];

    // Check for ::timeout:N suffix
    let timeout_override = if let Some(timeout_str) = after_close.strip_prefix("::timeout:") {
        // Must be the entire remainder
        let timeout_val: u64 = timeout_str.parse().ok()?;
        if timeout_val == 0 {
            return None; // reject zero timeout
        }
        Some(std::time::Duration::from_secs(timeout_val))
    } else if !after_close.is_empty() {
        // If there's content after ) but it's not ::timeout:, reject
        return None;
    } else {
        None
    };

    // Check executable-token interpolation rule if we have the original
    if let Some(original) = original_value {
        if !validate_no_executable_interpolation(original) {
            return None;
        }
    }

    // Tokenize the inner command
    let tokens = tokenize(inner_command).ok()?;
    if tokens.is_empty() {
        return None;
    }

    let executable = tokens[0].clone();
    let args = tokens[1..].to_vec();

    Some(FrontmatterShellDirective {
        key: key.to_string(),
        raw_template: value.to_string(),
        raw_command: inner_command.to_string(),
        executable,
        args,
        timeout_override,
    })
}

/// Validates that the executable token doesn't contain interpolation.
///
/// Checks the ORIGINAL (pre-interpolation) string to ensure `{{ }}` doesn't
/// appear in the executable position (first token after `$(` and before whitespace).
fn validate_no_executable_interpolation(original: &str) -> bool {
    // Must start with $(
    if !original.starts_with("$(") {
        return true;
    }

    let rest = &original[2..];
    let close_pos = match rest.find(')') {
        Some(pos) => pos,
        None => return true, // malformed, will fail elsewhere
    };

    let inner = &rest[..close_pos];

    // Extract the executable portion (up to first whitespace)
    let executable_portion = inner
        .split_whitespace()
        .next()
        .unwrap_or(inner);

    // Check if it contains {{ and }}
    if executable_portion.contains("{{") && executable_portion.contains("}}") {
        return false;
    }

    // Also check for operators that could introduce executable interpolation
    // The tokenizer already rejects pipes, semicolons, etc., but we need to
    // check the ORIGINAL string before tokenization for interpolation in those positions
    // For now, the main check is the first token
    true
}

/// Scans all top-level string-valued frontmatter entries and returns candidates.
///
/// Only examines top-level keys — nested objects and arrays are ignored.
pub(crate) fn scan_frontmatter(
    frontmatter: &Frontmatter,
    pre_interpolation_snapshot: Option<&HashMap<String, String>>,
) -> Vec<FrontmatterShellDirective> {
    let mut directives = Vec::new();
    let fm = frontmatter.as_map();

    for (key, value) in fm.iter() {
        // Only process top-level string values
        if let Value::String(s) = value {
            let original = pre_interpolation_snapshot
                .and_then(|map| map.get(key))
                .map(|s| s.as_str());

            if let Some(directive) = parse_shell_value(s, key, original) {
                directives.push(directive);
            }
        }
    }

    directives
}

/// Executes all frontmatter shell expansion directives and rewrites
/// frontmatter values in place.
///
/// Scans top-level string-valued frontmatter entries for `$(...)` expressions,
/// validates them, executes approved commands through the shared shell runtime,
/// trims output, and rewrites the frontmatter values.
///
/// ## Errors
///
/// Returns an error if:
/// - Policy file I/O fails
/// - Command execution fails (blacklisted, denied, execution error, etc.)
pub(crate) fn execute_frontmatter_shell_expansion(
    frontmatter: &mut Frontmatter,
    options: &ComposeOptions,
    runtime: &mut PipelineRuntime,
    pre_interpolation_snapshot: Option<&HashMap<String, String>>,
) -> MarkdownResult<FrontmatterShellExpansionReport> {
    let candidates = scan_frontmatter(frontmatter, pre_interpolation_snapshot);

    if candidates.is_empty() {
        return Ok(FrontmatterShellExpansionReport {
            replacements: 0,
            approvals_used: 0,
            warnings: vec![],
        });
    }

    // Resolve policy paths once for all directives
    let shell_opts = options.shell_options();
    let policy_paths = resolve_policy_paths(&shell_opts, &options.source)?;
    runtime.shell.ensure_loaded(&policy_paths)?;

    let mut results: Vec<(String, String)> = Vec::new();

    for candidate in &candidates {
        // Build a ShellDirective compatible with the existing execution infrastructure
        let directive = ShellDirective {
            raw_command: candidate.raw_command.clone(),
            executable: candidate.executable.clone(),
            args: candidate.args.clone(),
            span: 0..0, // Not relevant for frontmatter
            origin: ShellCommandOrigin::Frontmatter {
                key: candidate.key.clone(),
            },
            error_handling: ErrorHandling::default(),
            timeout_override: candidate.timeout_override,
        };

        // Execute through existing shell expansion infrastructure
        let output = execute_directive(&directive, options, &policy_paths, &mut runtime.shell)?;

        // Trim all surrounding whitespace per spec
        let trimmed = output.trim().to_string();

        results.push((candidate.key.clone(), trimmed));
    }

    // Rewrite frontmatter values
    let fm_mut = frontmatter.as_map_mut();
    let replacements = results.len();
    for (key, value) in results {
        fm_mut.insert(key, Value::String(value));
    }

    let approvals_used = runtime.shell.take_recent_approval_count();

    Ok(FrontmatterShellExpansionReport {
        replacements,
        approvals_used,
        warnings: vec![],
    })
}

/// Executes a shell directive with policy enforcement and approval flow.
///
/// This is a thin wrapper around the existing shell expansion infrastructure
/// that adapts it for frontmatter shell commands.
fn execute_directive(
    directive: &ShellDirective,
    options: &ComposeOptions,
    policy_paths: &super::shell_expansion::types::ShellPolicyPaths,
    shell_runtime: &mut super::shell_expansion::types::ShellExpansionRuntime,
) -> Result<String, ShellExpansionError> {
    // Delegate to the existing shell expansion infrastructure
    super::shell_expansion::execute_directive(directive, options, policy_paths, shell_runtime)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn detects_simple_shell_expression() {
        let result = parse_shell_value("$(echo hello)", "key", None);
        assert!(result.is_some());
        let directive = result.unwrap();
        assert_eq!(directive.executable, "echo");
        assert_eq!(directive.args, vec!["hello"]);
        assert_eq!(directive.raw_command, "echo hello");
        assert!(directive.timeout_override.is_none());
    }

    #[test]
    fn detects_expression_with_timeout() {
        let result = parse_shell_value("$(pwd)::timeout:3", "key", None);
        assert!(result.is_some());
        let directive = result.unwrap();
        assert_eq!(directive.executable, "pwd");
        assert_eq!(directive.args.len(), 0);
        assert_eq!(directive.timeout_override, Some(std::time::Duration::from_secs(3)));
    }

    #[test]
    fn ignores_non_shell_string() {
        let result = parse_shell_value("plain text", "key", None);
        assert!(result.is_none());
    }

    #[test]
    fn ignores_partial_match_no_closing() {
        let result = parse_shell_value("$(echo hello", "key", None);
        assert!(result.is_none());
    }

    #[test]
    fn ignores_embedded_expression() {
        let result = parse_shell_value("prefix $(cmd) suffix", "key", None);
        assert!(result.is_none());
    }

    #[test]
    fn rejects_zero_timeout() {
        let result = parse_shell_value("$(echo)::timeout:0", "key", None);
        assert!(result.is_none());
    }

    #[test]
    fn rejects_non_integer_timeout() {
        let result = parse_shell_value("$(echo)::timeout:abc", "key", None);
        assert!(result.is_none());
    }

    #[test]
    fn rejects_interpolated_executable() {
        let original = "$({{cmd}} arg)";
        let resolved = "$(ls arg)";
        let result = parse_shell_value(resolved, "key", Some(original));
        assert!(result.is_none());
    }

    #[test]
    fn accepts_interpolated_argument() {
        let original = "$(dirname {{file}})";
        let resolved = "$(dirname README.md)";
        let result = parse_shell_value(resolved, "key", Some(original));
        assert!(result.is_some());
        let directive = result.unwrap();
        assert_eq!(directive.executable, "dirname");
        assert_eq!(directive.args, vec!["README.md"]);
    }

    #[test]
    fn accepts_no_interpolation_at_all() {
        let original = "$(echo hello)";
        let result = parse_shell_value(original, "key", Some(original));
        assert!(result.is_some());
        let directive = result.unwrap();
        assert_eq!(directive.executable, "echo");
        assert_eq!(directive.args, vec!["hello"]);
    }

    #[test]
    fn scan_finds_shell_in_top_level_strings() {
        let mut fm = Frontmatter::new();
        fm.insert("cmd1", json!("$(echo hello)")).unwrap();
        fm.insert("plain", json!("not a shell command")).unwrap();
        fm.insert("cmd2", json!("$(pwd)")).unwrap();
        fm.insert("number", json!(42)).unwrap();

        let directives = scan_frontmatter(&fm, None);
        assert_eq!(directives.len(), 2);
        assert!(directives.iter().any(|d| d.key == "cmd1"));
        assert!(directives.iter().any(|d| d.key == "cmd2"));
    }

    #[test]
    fn scan_skips_nested_objects() {
        let mut fm = Frontmatter::new();
        fm.insert("outer", json!({"inner": "$(echo nested)"})).unwrap();
        fm.insert("top", json!("$(echo top)")).unwrap();

        let directives = scan_frontmatter(&fm, None);
        assert_eq!(directives.len(), 1);
        assert_eq!(directives[0].key, "top");
    }

    #[test]
    fn scan_skips_arrays() {
        let mut fm = Frontmatter::new();
        fm.insert("arr", json!(["$(echo one)", "$(echo two)"])).unwrap();
        fm.insert("top", json!("$(echo top)")).unwrap();

        let directives = scan_frontmatter(&fm, None);
        assert_eq!(directives.len(), 1);
        assert_eq!(directives[0].key, "top");
    }
}

#[cfg(test)]
mod execution_tests {
    use super::*;
    use crate::markdown::compose::cache::CacheAccessMode;
    use crate::markdown::compose::shell_expansion::types::{
        PipelineRuntime, ShellApprovalDecision, ShellApprovalHandler, ShellApprovalRequest,
        ShellExpansionError, ShellExpansionOptions,
    };
    use crate::markdown::compose::types::ComposeOptions;
    use crate::markdown::frontmatter::Frontmatter;
    use serde_json::json;
    use std::sync::Arc;
    use tempfile::TempDir;

    fn fm_from_json(data: serde_json::Value) -> Frontmatter {
        let map: crate::markdown::types::FrontmatterMap = match data {
            serde_json::Value::Object(obj) => obj.into_iter().collect(),
            _ => Default::default(),
        };
        Frontmatter::from_map(map)
    }

    struct MockApproval;
    impl ShellApprovalHandler for MockApproval {
        fn approve(
            &self,
            _request: ShellApprovalRequest,
        ) -> Result<ShellApprovalDecision, ShellExpansionError> {
            Ok(ShellApprovalDecision::AllowOnce)
        }
    }

    fn make_runtime() -> PipelineRuntime {
        PipelineRuntime::new(16, CacheAccessMode::Off, None)
    }

    #[test]
    fn execute_replaces_frontmatter_value_with_output() {
        let temp_dir = TempDir::new().unwrap();
        let mut fm = fm_from_json(json!({
            "greeting": "$(echo hello world)"
        }));
        let options = ComposeOptions::new().with_shell(ShellExpansionOptions {
            policy_root: Some(temp_dir.path().to_path_buf()),
            approval_handler: Some(Arc::new(MockApproval)),
            ..Default::default()
        });
        let mut runtime = make_runtime();

        let report =
            execute_frontmatter_shell_expansion(&mut fm, &options, &mut runtime, None).unwrap();
        assert_eq!(report.replacements, 1);
        assert_eq!(fm.as_map().get("greeting"), Some(&json!("hello world")));
    }

    #[test]
    fn execute_trims_output_whitespace() {
        let temp_dir = TempDir::new().unwrap();
        let mut fm = fm_from_json(json!({
            "val": "$(echo '  padded  ')"
        }));
        let options = ComposeOptions::new().with_shell(ShellExpansionOptions {
            policy_root: Some(temp_dir.path().to_path_buf()),
            approval_handler: Some(Arc::new(MockApproval)),
            ..Default::default()
        });
        let mut runtime = make_runtime();

        let report =
            execute_frontmatter_shell_expansion(&mut fm, &options, &mut runtime, None).unwrap();
        assert_eq!(report.replacements, 1);
        assert_eq!(fm.as_map().get("val"), Some(&json!("padded")));
    }

    #[test]
    fn execute_skips_non_shell_values() {
        let temp_dir = TempDir::new().unwrap();
        let mut fm = fm_from_json(json!({
            "title": "Hello",
            "count": 42,
            "cmd": "$(echo result)"
        }));
        let options = ComposeOptions::new().with_shell(ShellExpansionOptions {
            policy_root: Some(temp_dir.path().to_path_buf()),
            approval_handler: Some(Arc::new(MockApproval)),
            ..Default::default()
        });
        let mut runtime = make_runtime();

        let report =
            execute_frontmatter_shell_expansion(&mut fm, &options, &mut runtime, None).unwrap();
        assert_eq!(report.replacements, 1);
        assert_eq!(fm.as_map().get("title"), Some(&json!("Hello")));
        assert_eq!(fm.as_map().get("count"), Some(&json!(42)));
        assert_eq!(fm.as_map().get("cmd"), Some(&json!("result")));
    }

    #[test]
    fn execute_no_candidates_returns_empty_report() {
        let mut fm = fm_from_json(json!({
            "title": "Hello",
            "count": 42
        }));
        let options = ComposeOptions::new();
        let mut runtime = make_runtime();

        let report =
            execute_frontmatter_shell_expansion(&mut fm, &options, &mut runtime, None).unwrap();
        assert_eq!(report.replacements, 0);
        assert_eq!(report.approvals_used, 0);
    }
}
