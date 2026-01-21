//! Ignore directive parsing for suppressing lint diagnostics.
//!
//! Supports comment-based suppression of lint rules:
//! - `// tree-hugger-ignore: rule1, rule2` - Ignore specific rules on next line
//! - `// tree-hugger-ignore` - Ignore all rules on next line
//! - `// tree-hugger-ignore-file: rule1` - Ignore rule for entire file

use std::collections::{HashMap, HashSet};

/// Directive prefix for line-level ignores.
const IGNORE_LINE_PREFIX: &str = "tree-hugger-ignore";

/// Directive prefix for file-level ignores.
const IGNORE_FILE_PREFIX: &str = "tree-hugger-ignore-file";

/// Tracks which diagnostics should be suppressed.
#[derive(Debug, Default)]
pub struct IgnoreDirectives {
    /// Rules to ignore for the entire file.
    file_ignores: HashSet<String>,

    /// All rules are ignored for the entire file.
    ignore_all_file: bool,

    /// Line-specific ignores: line number -> set of rules (empty = all rules).
    line_ignores: HashMap<usize, HashSet<String>>,

    /// Lines where all rules should be ignored.
    ignore_all_lines: HashSet<usize>,
}

impl IgnoreDirectives {
    /// Creates a new empty IgnoreDirectives.
    pub fn new() -> Self {
        Self::default()
    }

    /// Parses ignore directives from source code comments.
    ///
    /// Scans the source for comments containing ignore directives and
    /// builds a mapping of what to suppress.
    pub fn parse(source: &str) -> Self {
        let mut directives = Self::new();

        for (line_idx, line) in source.lines().enumerate() {
            let line_num = line_idx + 1; // 1-based line numbers
            let trimmed = line.trim();

            // Look for single-line comment markers
            let comment = extract_comment(trimmed);
            if comment.is_empty() {
                continue;
            }

            // Check for file-level ignores
            if let Some(rules) = comment.strip_prefix(IGNORE_FILE_PREFIX) {
                let rules = rules.trim_start_matches(':').trim();
                if rules.is_empty() {
                    directives.ignore_all_file = true;
                } else {
                    for rule in rules.split(',') {
                        let rule = rule.trim();
                        if !rule.is_empty() {
                            directives.file_ignores.insert(rule.to_string());
                        }
                    }
                }
                continue;
            }

            // Check for line-level ignores (affects next line)
            if let Some(rules) = comment.strip_prefix(IGNORE_LINE_PREFIX) {
                let next_line = line_num + 1;
                let rules = rules.trim_start_matches(':').trim();
                if rules.is_empty() {
                    directives.ignore_all_lines.insert(next_line);
                } else {
                    let rule_set = directives.line_ignores.entry(next_line).or_default();
                    for rule in rules.split(',') {
                        let rule = rule.trim();
                        if !rule.is_empty() {
                            rule_set.insert(rule.to_string());
                        }
                    }
                }
            }
        }

        directives
    }

    /// Checks if a diagnostic should be suppressed.
    ///
    /// Returns true if the diagnostic at the given line with the given rule
    /// should be ignored based on parsed directives.
    pub fn should_ignore(&self, line: usize, rule: Option<&str>) -> bool {
        // Check file-level ignore all
        if self.ignore_all_file {
            return true;
        }

        // Check file-level rule-specific ignore
        if let Some(rule) = rule {
            if self.file_ignores.contains(rule) {
                return true;
            }
        }

        // Check line-level ignore all
        if self.ignore_all_lines.contains(&line) {
            return true;
        }

        // Check line-level rule-specific ignore
        if let Some(rule) = rule {
            if let Some(rules) = self.line_ignores.get(&line) {
                if rules.contains(rule) {
                    return true;
                }
            }
        }

        false
    }

    /// Returns true if there are any ignore directives.
    pub fn has_directives(&self) -> bool {
        self.ignore_all_file
            || !self.file_ignores.is_empty()
            || !self.ignore_all_lines.is_empty()
            || !self.line_ignores.is_empty()
    }
}

/// Extracts the comment content from a line.
///
/// Supports common single-line comment styles:
/// - `//` (Rust, JS, TS, Java, C, C++, Swift, Scala, Go)
/// - `#` (Python, Bash, Perl, Ruby)
/// - `--` (Lua, SQL)
/// - `;;` (Lisp, Scheme)
fn extract_comment(line: &str) -> &str {
    // Try common comment prefixes
    for prefix in &["//", "#", "--", ";;", ";"] {
        if let Some(rest) = line.strip_prefix(prefix) {
            return rest.trim();
        }
    }

    // Also check for comment content inside block comments /* ... */
    if line.starts_with("/*") || line.starts_with("(*") {
        let content = line
            .trim_start_matches("/*")
            .trim_start_matches("(*")
            .trim_end_matches("*/")
            .trim_end_matches("*)")
            .trim();
        return content;
    }

    ""
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_line_ignore_all() {
        let source = r#"
let x = 1;
// tree-hugger-ignore
let y = x.unwrap();
"#;
        let directives = IgnoreDirectives::parse(source);
        assert!(directives.should_ignore(4, Some("unwrap-call")));
        assert!(directives.should_ignore(4, Some("any-rule")));
        assert!(!directives.should_ignore(2, Some("unwrap-call")));
    }

    #[test]
    fn parse_line_ignore_specific_rules() {
        let source = r#"
// tree-hugger-ignore: unwrap-call, expect-call
let x = value.unwrap();
"#;
        let directives = IgnoreDirectives::parse(source);
        assert!(directives.should_ignore(3, Some("unwrap-call")));
        assert!(directives.should_ignore(3, Some("expect-call")));
        assert!(!directives.should_ignore(3, Some("dead-code")));
    }

    #[test]
    fn parse_file_ignore() {
        let source = r#"
// tree-hugger-ignore-file: unused-import
import { foo } from "bar";
import { baz } from "qux";
"#;
        let directives = IgnoreDirectives::parse(source);
        assert!(directives.should_ignore(3, Some("unused-import")));
        assert!(directives.should_ignore(4, Some("unused-import")));
        assert!(directives.should_ignore(100, Some("unused-import")));
        assert!(!directives.should_ignore(3, Some("undefined-symbol")));
    }

    #[test]
    fn parse_file_ignore_all() {
        let source = "// tree-hugger-ignore-file\nlet x = 1;";
        let directives = IgnoreDirectives::parse(source);
        assert!(directives.should_ignore(1, Some("any-rule")));
        assert!(directives.should_ignore(2, Some("any-rule")));
    }

    #[test]
    fn python_comment_style() {
        let source = r#"
# tree-hugger-ignore: unused-import
import os
"#;
        let directives = IgnoreDirectives::parse(source);
        assert!(directives.should_ignore(3, Some("unused-import")));
    }

    #[test]
    fn lua_comment_style() {
        let source = r#"
-- tree-hugger-ignore: unused-symbol
local x = 1
"#;
        let directives = IgnoreDirectives::parse(source);
        assert!(directives.should_ignore(3, Some("unused-symbol")));
    }

    #[test]
    fn no_directives() {
        let source = "let x = 1;\nlet y = 2;";
        let directives = IgnoreDirectives::parse(source);
        assert!(!directives.has_directives());
        assert!(!directives.should_ignore(1, Some("any-rule")));
    }
}
