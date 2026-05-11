//! Frontmatter shell expansion parsing and validation.
//!
//! Scans top-level frontmatter string values for `$(cmd)` shell expressions,
//! parses them, validates the executable-token interpolation rule, and returns
//! structured directives ready for execution.

use super::shell_expansion::store::resolve_policy_paths;
use super::shell_expansion::tokenize::{parse_pipeline, tokenize};
use super::shell_expansion::types::{
    ErrorHandling, PipelineRuntime, ShellCommandOrigin, ShellDirective, ShellExpansionError,
    ShellPipeline,
};
use super::shell_expansion::{execute_prepared_directive, prepare_directive};
use super::types::{ComposeOptions, ComposeWarning};
use crate::markdown::frontmatter::Frontmatter;
use crate::markdown::types::MarkdownResult;
use biscuit_terminal::errors::SourceContext;
use rayon::prelude::*;
use serde_json::Value;
use std::collections::HashMap;

/// A parsed frontmatter shell directive ready for execution.
#[derive(Debug, Clone)]
pub(crate) struct FrontmatterShellDirective {
    pub key: String,
    pub raw_command: String,
    pub executable: String,
    pub args: Vec<String>,
    pub timeout_override: Option<std::time::Duration>,
    pub pipeline: Option<ShellPipeline>,
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
/// - Return an error for any parse/validation failure once the value matches
///   the `$(` shell-expression shape
pub(crate) fn parse_shell_value(
    value: &str,
    key: &str,
    original_value: Option<&str>,
    ctx: &SourceContext,
) -> Result<Option<FrontmatterShellDirective>, ShellExpansionError> {
    // Must start with $(
    if !value.starts_with("$(") {
        return Ok(None);
    }

    let rest = &value[2..];
    let close_pos = find_unquoted_closing_paren(rest, key, ctx)?;

    let inner_command = &rest[..close_pos];
    let after_close = &rest[close_pos + 1..];

    // Check for ::timeout:N suffix
    let timeout_override = if let Some(timeout_str) = after_close.strip_prefix("::timeout:") {
        let timeout_val: u64 = timeout_str.parse().map_err(|_| {
            frontmatter_parse_error(
                key,
                ctx,
                "Invalid ::timeout value in frontmatter shell expression; expected a positive integer number of seconds",
            )
        })?;
        if timeout_val == 0 {
            return Err(frontmatter_parse_error(
                key,
                ctx,
                "Frontmatter shell timeout must be greater than zero",
            ));
        }
        Some(std::time::Duration::from_secs(timeout_val))
    } else if !after_close.is_empty() {
        return Err(frontmatter_parse_error(
            key,
            ctx,
            "Unexpected trailing content after frontmatter shell expression",
        ));
    } else {
        None
    };

    // Check executable-token interpolation rule if we have the original
    if let Some(original) = original_value {
        validate_no_executable_interpolation(original, key, ctx)?;
    }

    // Tokenize the inner command
    let tokens =
        tokenize(inner_command, ctx).map_err(|error| remap_parse_error(error, key, ctx))?;

    // Parse into a pipeline
    let pipeline =
        parse_pipeline(&tokens, ctx).map_err(|error| remap_parse_error(error, key, ctx))?;

    let executable = pipeline.actions[0].command.executable.clone();
    let args = pipeline.actions[0].command.args.clone();

    Ok(Some(FrontmatterShellDirective {
        key: key.to_string(),
        raw_command: inner_command.to_string(),
        executable,
        args,
        timeout_override,
        pipeline: Some(pipeline),
    }))
}

/// Validates that no executable token in the pipeline comes from interpolation.
///
/// Checks the ORIGINAL (pre-interpolation) string to ensure `{{ }}` doesn't
/// appear in any executable position — that is, the first non-whitespace token
/// after `$(`, plus the first non-whitespace token after every top-level `&&`
/// or `||` chain operator.
fn validate_no_executable_interpolation(
    original: &str,
    key: &str,
    ctx: &SourceContext,
) -> Result<(), ShellExpansionError> {
    // Must start with $(
    if !original.starts_with("$(") {
        return Ok(());
    }

    let rest = &original[2..];
    let close_pos = find_unquoted_closing_paren(rest, key, ctx)?;

    let inner = &rest[..close_pos];

    for segment in split_at_chain_operators(inner) {
        let executable_portion = first_token_portion(segment);
        if executable_portion.contains("{{") && executable_portion.contains("}}") {
            return Err(frontmatter_parse_error(
                key,
                ctx,
                "Frontmatter shell executable may not come from interpolation",
            ));
        }
    }

    Ok(())
}

/// Splits an inner `$(...)` body at top-level `&&` and `||` operators,
/// respecting single quotes, double quotes, and backslash escaping.
///
/// Each returned segment corresponds to one chain action; the executable
/// position of that action is the first non-whitespace token of the segment.
fn split_at_chain_operators(input: &str) -> Vec<&str> {
    let mut segments = Vec::new();
    let mut start = 0usize;
    let mut in_single = false;
    let mut in_double = false;
    let mut escaped = false;
    let bytes = input.as_bytes();
    let mut i = 0usize;

    while i < bytes.len() {
        if escaped {
            escaped = false;
            i += 1;
            continue;
        }

        let ch = bytes[i];
        match ch {
            b'\\' if !in_single => {
                escaped = true;
                i += 1;
            }
            b'\'' if !in_double => {
                in_single = !in_single;
                i += 1;
            }
            b'"' if !in_single => {
                in_double = !in_double;
                i += 1;
            }
            b'&' if !in_single && !in_double && i + 1 < bytes.len() && bytes[i + 1] == b'&' => {
                segments.push(&input[start..i]);
                i += 2;
                start = i;
            }
            b'|' if !in_single && !in_double && i + 1 < bytes.len() && bytes[i + 1] == b'|' => {
                segments.push(&input[start..i]);
                i += 2;
                start = i;
            }
            _ => {
                i += 1;
            }
        }
    }

    segments.push(&input[start..]);
    segments
}

/// Scans all top-level string-valued frontmatter entries and returns candidates.
///
/// Only examines top-level keys — nested objects and arrays are ignored.
pub(crate) fn scan_frontmatter(
    frontmatter: &Frontmatter,
    pre_interpolation_snapshot: Option<&HashMap<String, String>>,
    ctx: &SourceContext,
) -> Result<Vec<FrontmatterShellDirective>, ShellExpansionError> {
    let mut directives = Vec::new();
    let fm = frontmatter.as_map();

    for (key, value) in fm.iter() {
        // Only process top-level string values
        if let Value::String(s) = value {
            let original = pre_interpolation_snapshot
                .and_then(|map| map.get(key))
                .map(|s| s.as_str());

            if let Some(directive) = parse_shell_value(s, key, original, ctx)? {
                directives.push(directive);
            }
        }
    }

    Ok(directives)
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
    ctx: &SourceContext,
) -> MarkdownResult<FrontmatterShellExpansionReport> {
    let candidates = scan_frontmatter(frontmatter, pre_interpolation_snapshot, ctx)?;

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

    let mut prepared = Vec::with_capacity(candidates.len());

    for (index, candidate) in candidates.iter().enumerate() {
        // Build a ShellDirective compatible with the existing execution infrastructure
        let directive = ShellDirective {
            raw_command: candidate.raw_command.clone(),
            executable: candidate.executable.clone(),
            args: candidate.args.clone(),
            span: 0..0,
            origin: ShellCommandOrigin::Frontmatter {
                key: candidate.key.clone(),
            },
            error_handling: ErrorHandling::default(),
            timeout_override: candidate.timeout_override,
            pipeline: candidate.pipeline.clone(),
            ctx: ctx.clone(),
        };

        prepared.push((
            index,
            candidate.key.clone(),
            prepare_directive(&directive, options, &policy_paths, &mut runtime.shell)?,
        ));
    }

    let mut executions = prepared
        .into_par_iter()
        .map(|(index, key, prepared)| (index, key, execute_prepared_directive(&prepared, options)))
        .collect::<Vec<_>>();
    executions.sort_by_key(|(index, _, _)| *index);

    // Rewrite frontmatter values
    let fm_mut = frontmatter.as_map_mut();
    let mut warnings = Vec::new();
    for (_, key, execution) in executions {
        let execution = execution?;
        warnings.extend(execution.warnings);
        fm_mut.insert(key, Value::String(execution.stdout.trim().to_string()));
    }

    let approvals_used = runtime.shell.take_recent_approval_count();

    Ok(FrontmatterShellExpansionReport {
        replacements: candidates.len(),
        approvals_used,
        warnings,
    })
}

fn frontmatter_parse_error(
    key: &str,
    ctx: &SourceContext,
    message: impl Into<String>,
) -> ShellExpansionError {
    ShellExpansionError::ParseDirective {
        ctx: Box::new(ctx.clone()),
        origin: ShellCommandOrigin::Frontmatter {
            key: key.to_string(),
        },
        message: message.into(),
    }
}

fn remap_parse_error(
    error: ShellExpansionError,
    key: &str,
    ctx: &SourceContext,
) -> ShellExpansionError {
    match error {
        ShellExpansionError::ParseDirective { message, .. } => {
            frontmatter_parse_error(key, ctx, message)
        }
        other => other,
    }
}

fn find_unquoted_closing_paren(
    rest: &str,
    key: &str,
    ctx: &SourceContext,
) -> Result<usize, ShellExpansionError> {
    let mut in_single = false;
    let mut in_double = false;
    let mut escaped = false;

    for (idx, ch) in rest.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }

        match ch {
            '\\' if !in_single => escaped = true,
            '\'' if !in_double => in_single = !in_single,
            '"' if !in_single => in_double = !in_double,
            ')' if !in_single && !in_double => return Ok(idx),
            _ => {}
        }
    }

    Err(frontmatter_parse_error(
        key,
        ctx,
        "Missing closing ')' in frontmatter shell expression",
    ))
}

fn first_token_portion(input: &str) -> &str {
    let trimmed = input.trim_start();
    if trimmed.is_empty() {
        return trimmed;
    }

    let mut in_single = false;
    let mut in_double = false;
    let mut escaped = false;

    for (idx, ch) in trimmed.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }

        match ch {
            '\\' if !in_single => escaped = true,
            '\'' if !in_double => in_single = !in_single,
            '"' if !in_single => in_double = !in_double,
            c if c.is_whitespace() && !in_single && !in_double => return &trimmed[..idx],
            _ => {}
        }
    }

    trimmed
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn test_ctx() -> SourceContext {
        SourceContext::new(
            std::path::PathBuf::from("/test"),
            std::path::PathBuf::from("test"),
            String::new(),
        )
    }

    fn parse_shell_value(
        value: &str,
        key: &str,
        original_value: Option<&str>,
    ) -> Result<Option<FrontmatterShellDirective>, ShellExpansionError> {
        super::parse_shell_value(value, key, original_value, &test_ctx())
    }

    #[allow(dead_code)]
    fn scan_frontmatter(
        frontmatter: &Frontmatter,
        pre_interpolation_snapshot: Option<&std::collections::HashMap<String, String>>,
    ) -> Result<Vec<FrontmatterShellDirective>, ShellExpansionError> {
        super::scan_frontmatter(frontmatter, pre_interpolation_snapshot, &test_ctx())
    }

    fn execute_frontmatter_shell_expansion(
        frontmatter: &mut Frontmatter,
        options: &ComposeOptions,
        runtime: &mut PipelineRuntime,
        pre_interpolation_snapshot: Option<&std::collections::HashMap<String, String>>,
    ) -> MarkdownResult<FrontmatterShellExpansionReport> {
        super::execute_frontmatter_shell_expansion(
            frontmatter,
            options,
            runtime,
            pre_interpolation_snapshot,
            &test_ctx(),
        )
    }

    #[test]
    fn detects_simple_shell_expression() {
        let result = parse_shell_value("$(echo hello)", "key", None);
        assert!(result.is_ok());
        let directive = result.unwrap().unwrap();
        assert_eq!(directive.executable, "echo");
        assert_eq!(directive.args, vec!["hello"]);
        assert_eq!(directive.raw_command, "echo hello");
        assert!(directive.timeout_override.is_none());
    }

    #[test]
    fn detects_expression_with_timeout() {
        let result = parse_shell_value("$(pwd)::timeout:3", "key", None);
        assert!(result.is_ok());
        let directive = result.unwrap().unwrap();
        assert_eq!(directive.executable, "pwd");
        assert_eq!(directive.args.len(), 0);
        assert_eq!(
            directive.timeout_override,
            Some(std::time::Duration::from_secs(3))
        );
    }

    #[test]
    fn ignores_non_shell_string() {
        let result = parse_shell_value("plain text", "key", None);
        assert!(matches!(result, Ok(None)));
    }

    #[test]
    fn ignores_partial_match_no_closing() {
        let result = parse_shell_value("$(echo hello", "key", None);
        assert!(result.is_err());
    }

    #[test]
    fn ignores_embedded_expression() {
        let result = parse_shell_value("prefix $(cmd) suffix", "key", None);
        assert!(matches!(result, Ok(None)));
    }

    #[test]
    fn rejects_zero_timeout() {
        let result = parse_shell_value("$(echo)::timeout:0", "key", None);
        assert!(result.is_err());
    }

    #[test]
    fn rejects_non_integer_timeout() {
        let result = parse_shell_value("$(echo)::timeout:abc", "key", None);
        assert!(result.is_err());
    }

    #[test]
    fn rejects_interpolated_executable() {
        let original = "$({{cmd}} arg)";
        let resolved = "$(ls arg)";
        let result = parse_shell_value(resolved, "key", Some(original));
        assert!(result.is_err());
    }

    #[test]
    fn rejects_interpolated_executable_after_or_operator() {
        let original = "$(false || {{cmd}} arg)";
        let resolved = "$(false || echo arg)";
        let err = parse_shell_value(resolved, "key", Some(original)).unwrap_err();
        match err {
            ShellExpansionError::ParseDirective { message, .. } => {
                assert!(
                    message.contains("may not come from interpolation"),
                    "unexpected message: {message}"
                );
            }
            other => panic!("Expected ParseDirective, got {other:?}"),
        }
    }

    #[test]
    fn rejects_interpolated_executable_after_and_operator() {
        let original = "$(true && {{cmd}} arg)";
        let resolved = "$(true && echo arg)";
        let err = parse_shell_value(resolved, "key", Some(original)).unwrap_err();
        match err {
            ShellExpansionError::ParseDirective { message, .. } => {
                assert!(
                    message.contains("may not come from interpolation"),
                    "unexpected message: {message}"
                );
            }
            other => panic!("Expected ParseDirective, got {other:?}"),
        }
    }

    #[test]
    fn rejects_interpolated_executable_in_third_chain_segment() {
        let original = "$(true && false || {{cmd}} arg)";
        let resolved = "$(true && false || echo arg)";
        let result = parse_shell_value(resolved, "key", Some(original));
        assert!(result.is_err());
    }

    #[test]
    fn accepts_interpolated_argument_after_chain_operator() {
        let original = "$(false || echo {{file}})";
        let resolved = "$(false || echo README.md)";
        let result = parse_shell_value(resolved, "key", Some(original));
        assert!(result.is_ok());
    }

    #[test]
    fn ignores_chain_operators_inside_quotes() {
        // `||` inside single quotes is literal, not a chain operator,
        // so the interpolation here is in argument position, not executable.
        let original = "$(echo 'a || {{cmd}}')";
        let resolved = "$(echo 'a || hello')";
        let result = parse_shell_value(resolved, "key", Some(original));
        assert!(result.is_ok());
    }

    #[test]
    fn accepts_interpolated_argument() {
        let original = "$(dirname {{file}})";
        let resolved = "$(dirname README.md)";
        let result = parse_shell_value(resolved, "key", Some(original));
        assert!(result.is_ok());
        let directive = result.unwrap().unwrap();
        assert_eq!(directive.executable, "dirname");
        assert_eq!(directive.args, vec!["README.md"]);
    }

    #[test]
    fn accepts_no_interpolation_at_all() {
        let original = "$(echo hello)";
        let result = parse_shell_value(original, "key", Some(original));
        assert!(result.is_ok());
        let directive = result.unwrap().unwrap();
        assert_eq!(directive.executable, "echo");
        assert_eq!(directive.args, vec!["hello"]);
    }

    #[test]
    fn accepts_closing_paren_inside_quoted_argument() {
        let result = parse_shell_value("$(printf ')')", "key", None).unwrap();
        let directive = result.unwrap();
        assert_eq!(directive.executable, "printf");
        assert_eq!(directive.args, vec![")"]);
    }

    #[test]
    fn scan_finds_shell_in_top_level_strings() {
        let mut fm = Frontmatter::new();
        fm.insert("cmd1", json!("$(echo hello)")).unwrap();
        fm.insert("plain", json!("not a shell command")).unwrap();
        fm.insert("cmd2", json!("$(pwd)")).unwrap();
        fm.insert("number", json!(42)).unwrap();

        let directives = scan_frontmatter(&fm, None).unwrap();
        assert_eq!(directives.len(), 2);
        assert!(directives.iter().any(|d| d.key == "cmd1"));
        assert!(directives.iter().any(|d| d.key == "cmd2"));
    }

    #[test]
    fn scan_skips_nested_objects() {
        let mut fm = Frontmatter::new();
        fm.insert("outer", json!({"inner": "$(echo nested)"}))
            .unwrap();
        fm.insert("top", json!("$(echo top)")).unwrap();

        let directives = scan_frontmatter(&fm, None).unwrap();
        assert_eq!(directives.len(), 1);
        assert_eq!(directives[0].key, "top");
    }

    #[test]
    fn scan_skips_arrays() {
        let mut fm = Frontmatter::new();
        fm.insert("arr", json!(["$(echo one)", "$(echo two)"]))
            .unwrap();
        fm.insert("top", json!("$(echo top)")).unwrap();

        let directives = scan_frontmatter(&fm, None).unwrap();
        assert_eq!(directives.len(), 1);
        assert_eq!(directives[0].key, "top");
    }

    #[test]
    fn scan_errors_on_malformed_shell_expression() {
        let mut fm = Frontmatter::new();
        fm.insert("bad", json!("$(echo hi)::timeout:0")).unwrap();

        let err = scan_frontmatter(&fm, None).unwrap_err();
        match err {
            ShellExpansionError::ParseDirective { origin, .. } => {
                assert_eq!(
                    origin,
                    ShellCommandOrigin::Frontmatter {
                        key: "bad".to_string()
                    }
                );
            }
            other => panic!("Expected ParseDirective, got {other:?}"),
        }
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
    use serial_test::serial;
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::time::{Duration, Instant};
    use tempfile::TempDir;

    fn fm_from_json(data: serde_json::Value) -> Frontmatter {
        let map: crate::markdown::types::FrontmatterMap = match data {
            serde_json::Value::Object(obj) => obj.into_iter().collect(),
            _ => Default::default(),
        };
        Frontmatter::from_map(map)
    }

    fn test_ctx() -> SourceContext {
        SourceContext::new(PathBuf::from("/test"), PathBuf::from("test"), String::new())
    }

    fn execute_frontmatter_shell_expansion(
        frontmatter: &mut Frontmatter,
        options: &ComposeOptions,
        runtime: &mut PipelineRuntime,
        pre_interpolation_snapshot: Option<&std::collections::HashMap<String, String>>,
    ) -> crate::markdown::types::MarkdownResult<FrontmatterShellExpansionReport> {
        super::execute_frontmatter_shell_expansion(
            frontmatter,
            options,
            runtime,
            pre_interpolation_snapshot,
            &test_ctx(),
        )
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

    fn find_python() -> Option<PathBuf> {
        ["python3", "python"]
            .into_iter()
            .find_map(|candidate| which::which(candidate).ok())
            .filter(|path| !path.as_os_str().is_empty())
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

    #[test]
    fn execute_frontmatter_uses_stdout_only() {
        let Some(python) = find_python() else {
            return;
        };

        let temp_dir = TempDir::new().unwrap();
        let mut fm = fm_from_json(json!({
            "val": format!(
                "$({} -c \"import sys; sys.stdout.write('out'); sys.stderr.write('warn')\")",
                python.display()
            )
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
        assert_eq!(fm.as_map().get("val"), Some(&json!("out")));
    }

    #[test]
    fn execute_timeout_fallback_emits_warning() {
        let temp_dir = TempDir::new().unwrap();
        let mut fm = fm_from_json(json!({
            "val": "$(sleep 1)"
        }));
        let options = ComposeOptions::new().with_shell(ShellExpansionOptions {
            timeout: Duration::from_millis(100),
            timeout_behavior:
                super::super::shell_expansion::types::ShellTimeoutBehavior::EmptyString,
            policy_root: Some(temp_dir.path().to_path_buf()),
            approval_handler: Some(Arc::new(MockApproval)),
            ..Default::default()
        });
        let mut runtime = make_runtime();

        let report =
            execute_frontmatter_shell_expansion(&mut fm, &options, &mut runtime, None).unwrap();
        assert_eq!(fm.as_map().get("val"), Some(&json!("")));
        assert_eq!(report.warnings.len(), 1);
        assert!(report.warnings[0].message.contains("timed out"));
    }

    #[test]
    fn execute_pipeline_timeout_fallback_emits_warning() {
        let temp_dir = TempDir::new().unwrap();
        let mut fm = fm_from_json(json!({
            "val": "$(sleep 1 && echo after)"
        }));
        let options = ComposeOptions::new().with_shell(ShellExpansionOptions {
            timeout: Duration::from_millis(100),
            timeout_behavior:
                super::super::shell_expansion::types::ShellTimeoutBehavior::EmptyString,
            policy_root: Some(temp_dir.path().to_path_buf()),
            approval_handler: Some(Arc::new(MockApproval)),
            ..Default::default()
        });
        let mut runtime = make_runtime();

        let report =
            execute_frontmatter_shell_expansion(&mut fm, &options, &mut runtime, None).unwrap();
        assert_eq!(fm.as_map().get("val"), Some(&json!("after")));
        assert_eq!(report.warnings.len(), 1);
        assert!(report.warnings[0].message.contains("timed out"));
    }

    #[test]
    #[serial]
    fn execute_frontmatter_commands_concurrently() {
        let Some(python) = find_python() else {
            return;
        };

        let temp_dir = TempDir::new().unwrap();
        let sleep_cmd = |label: &str| {
            format!(
                "$({} -c \"import time; time.sleep(0.35); print('{}', end='')\")",
                python.display(),
                label
            )
        };
        let mut fm = fm_from_json(json!({
            "one": sleep_cmd("one"),
            "two": sleep_cmd("two")
        }));
        let options = ComposeOptions::new().with_shell(ShellExpansionOptions {
            timeout: Duration::from_secs(2),
            policy_root: Some(temp_dir.path().to_path_buf()),
            approval_handler: Some(Arc::new(MockApproval)),
            ..Default::default()
        });
        let mut runtime = make_runtime();

        let start = Instant::now();
        let report =
            execute_frontmatter_shell_expansion(&mut fm, &options, &mut runtime, None).unwrap();
        let elapsed = start.elapsed();

        assert_eq!(report.replacements, 2);
        assert_eq!(fm.as_map().get("one"), Some(&json!("one")));
        assert_eq!(fm.as_map().get("two"), Some(&json!("two")));
        assert!(
            elapsed < Duration::from_millis(650),
            "expected concurrent execution under 650ms, got {elapsed:?}"
        );
    }
}
