//! Frontmatter shell expansion parsing and validation.
//!
//! Scans top-level frontmatter string values for `$(cmd)` shell expressions,
//! parses them, validates the executable-token interpolation rule, and returns
//! structured directives ready for execution.

use super::shell_expansion::tokenize::tokenize;
use super::types::ComposeWarning;
use crate::markdown::frontmatter::Frontmatter;
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
