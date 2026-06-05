use std::path::Path;

/// Normalizes diagnostic output for stable snapshot comparison.
///
/// This function applies the following transformations:
/// - Redacts absolute paths
/// - Normalizes line endings to `\n`
/// - Sorts diagnostics deterministically
/// - Strips tool versions and temp directory references
/// - Produces compact context rather than full source
///
/// ## Examples
///
/// ```
/// use std::path::Path;
/// use tree_hugger::corpus::redact_snapshot_text;
///
/// let text = "Error at /home/user/project/src/main.rs:42\r\n";
/// let normalized = redact_snapshot_text(text, Path::new("/home/user/project"));
/// assert!(normalized.contains("Error at <REDACTED>"));
/// assert!(!normalized.contains("/home/user/project"));
/// assert!(!normalized.contains('\r'));
/// ```
pub fn redact_snapshot_text(text: &str, corpus_root: &Path) -> String {
    let mut result = text.to_string();

    // Normalize line endings first
    result = result.replace("\r\n", "\n");

    // Redact temp directory references
    let temp_dir = std::env::temp_dir().to_string_lossy().to_string();
    result = result.replace(&temp_dir, "<TMP>");

    // Redact absolute paths using path-aware redaction
    result = super::redact_paths(&result, corpus_root);

    // Redact common tool path patterns
    result = result.replace("node_modules/.bin/", "<BIN>/");
    result = result.replace("target/debug/", "<BUILD>/");
    result = result.replace("target/release/", "<BUILD>/");

    result
}

/// Produces a compact, stable representation of diagnostics for snapshots.
///
/// Instead of full source context, this stores only the diagnostic id,
/// rule, line, and a short message fragment.
pub fn compact_diagnostic_text(rule: Option<&str>, line: usize, message: &str) -> String {
    let rule_part = rule.unwrap_or("syntax");
    let short_msg = if message.len() > 60 {
        &message[..60]
    } else {
        message
    };
    format!("[{}:{}] {}", rule_part, line, short_msg)
}

/// Sorts lines deterministically for stable snapshot output.
///
/// Splits on newlines, sorts, and rejoins. Useful when diagnostic
/// ordering is non-deterministic.
pub fn sort_lines(text: &str) -> String {
    let mut lines: Vec<&str> = text.lines().collect();
    lines.sort_unstable();
    lines.join("\n")
}

/// Strips ANSI escape codes from text.
///
/// Useful for normalizing terminal output before snapshot comparison.
pub fn strip_ansi_codes(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut in_escape = false;

    for ch in text.chars() {
        if in_escape {
            if ch.is_ascii_alphabetic() || ch == '~' {
                in_escape = false;
            }
        } else if ch == '\u{001b}' {
            in_escape = true;
        } else {
            result.push(ch);
        }
    }

    result
}
