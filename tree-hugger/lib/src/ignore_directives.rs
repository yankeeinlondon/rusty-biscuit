//! Ignore directive parsing for suppressing lint diagnostics.
//!
//! Supports comment-based suppression of lint rules:
//! - `// tree-hugger-ignore: rule1, rule2` - Ignore specific rules on next line
//! - `// tree-hugger-ignore` - Ignore all rules on next line
//! - `// tree-hugger-ignore-file: rule1` - Ignore rule for entire file
//!
//! This module provides two parsing strategies:
//! - `parse()`: Line-based fallback (fast but can have false positives)
//! - `parse_with_tree()`: Tree-sitter based (accurate, avoids false positives from strings)

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use tree_sitter::{Query, QueryCursor, StreamingIterator, Tree};

use crate::queries::{query_for, QueryKind};
use crate::shared::ProgrammingLanguage;

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

    /// Parses ignore directives using tree-sitter comment captures.
    ///
    /// This method uses tree-sitter to identify actual comment nodes in the AST,
    /// preventing false positives from `//` or `#` sequences inside string literals.
    ///
    /// ## Arguments
    /// - `source`: The source code text
    /// - `tree`: The parsed tree-sitter AST
    /// - `language`: The programming language for query selection
    ///
    /// ## Returns
    /// Returns `IgnoreDirectives` with all parsed suppression rules.
    /// Falls back to line-based parsing if no comments query is available.
    pub fn parse_with_tree(source: &str, tree: &Tree, language: ProgrammingLanguage) -> Self {
        // Try to get the comments query for this language
        let query = match query_for(language, QueryKind::Comments) {
            Ok(q) if q.pattern_count() > 0 => q,
            // Fall back to line-based parsing if no comments query
            _ => return Self::parse(source),
        };

        Self::parse_with_query(source, tree, &query)
    }

    /// Parses ignore directives using a pre-loaded comments query.
    ///
    /// Internal method that performs the actual tree-sitter based parsing.
    fn parse_with_query(source: &str, tree: &Tree, query: &Arc<Query>) -> Self {
        let mut directives = Self::new();
        let mut cursor = QueryCursor::new();
        let root = tree.root_node();

        let source_bytes = source.as_bytes();
        let mut matches = cursor.matches(query.as_ref(), root, source_bytes);
        matches.advance();

        while let Some(query_match) = matches.get() {
            for capture in query_match.captures {
                let node = capture.node;

                // Extract the comment text
                let comment_text = match node.utf8_text(source_bytes) {
                    Ok(text) => text,
                    Err(_) => continue,
                };

                // Get the line number (1-based) - we use end_line for multiline comments
                let end_line = node.end_position().row + 1;

                // Parse the comment content for directives
                let content = extract_comment_content(comment_text);
                if content.is_empty() {
                    continue;
                }

                // Check for file-level ignores
                if let Some(rules) = content.strip_prefix(IGNORE_FILE_PREFIX) {
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

                // Check for line-level ignores (affects next line after comment ends)
                if let Some(rules) = content.strip_prefix(IGNORE_LINE_PREFIX) {
                    let next_line = end_line + 1;
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

            matches.advance();
        }

        directives
    }

    /// Parses ignore directives from source code comments (line-based fallback).
    ///
    /// This is a simpler line-based scanner that works without tree-sitter.
    /// It may produce false positives if `//` or `#` appear inside string literals.
    /// Prefer `parse_with_tree()` when a parsed tree is available.
    pub fn parse(source: &str) -> Self {
        let mut directives = Self::new();

        for (line_idx, line) in source.lines().enumerate() {
            let line_num = line_idx + 1; // 1-based line numbers
            let trimmed = line.trim();

            // Look for single-line comment markers
            let comment = extract_comment_line(trimmed);
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
        if let Some(rule) = rule
            && self.file_ignores.contains(rule)
        {
            return true;
        }

        // Check line-level ignore all
        if self.ignore_all_lines.contains(&line) {
            return true;
        }

        // Check line-level rule-specific ignore
        if let Some(rule) = rule
            && let Some(rules) = self.line_ignores.get(&line)
            && rules.contains(rule)
        {
            return true;
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

/// Extracts directive content from a comment node's text.
///
/// Handles both line comments and block comments by stripping:
/// - Line comment prefixes: `//`, `#`, `--`, `;`
/// - Block comment delimiters: `/* */`, `(* *)`, `--[[ ]]`
fn extract_comment_content(comment: &str) -> &str {
    let trimmed = comment.trim();

    // Handle block comments first (before line comments to properly handle --[[ ]])

    // Handle Lua block comments: --[[ ... ]] or --[=[ ... ]=]
    if let Some(rest) = trimmed.strip_prefix("--[[") {
        return rest.trim_end_matches("]]").trim();
    }
    if let Some(rest) = trimmed.strip_prefix("--[=[") {
        return rest.trim_end_matches("]=]").trim();
    }

    // Handle block comments: /* ... */ or /** ... */
    if let Some(rest) = trimmed.strip_prefix("/*") {
        let rest = rest.strip_prefix('*').unwrap_or(rest); // Handle /** style
        let content = rest.trim_end_matches("*/");
        // For multiline block comments, strip leading * from each line
        return strip_block_comment_decoration(content);
    }

    // Handle ML-style block comments: (* ... *)
    if let Some(rest) = trimmed.strip_prefix("(*") {
        return rest.trim_end_matches("*)").trim();
    }

    // Handle line comment prefixes
    for prefix in &["///", "//!", "//", "##", "#!", "#", "--", ";;", ";"] {
        if let Some(rest) = trimmed.strip_prefix(prefix) {
            return rest.trim();
        }
    }

    trimmed
}

/// Strips block comment decoration (leading `*` on each line) and finds the directive.
///
/// For a multiline block comment like:
/// ```text
/// /*
///  * tree-hugger-ignore: rule
///  */
/// ```
/// This will return `"tree-hugger-ignore: rule"`.
fn strip_block_comment_decoration(content: &str) -> &str {
    // If it's a single line, just return trimmed content
    if !content.contains('\n') {
        return content.trim().trim_start_matches('*').trim();
    }

    // For multiline content, search for a line containing the directive
    // and return a slice pointing into the original content
    for line in content.lines() {
        let trimmed = line.trim();
        // Strip leading * decoration
        let stripped = trimmed.strip_prefix('*').unwrap_or(trimmed).trim();
        if stripped.starts_with("tree-hugger-ignore") {
            // Find this substring in the original content
            if let Some(pos) = content.find("tree-hugger-ignore") {
                // Find the end of this directive line (at newline or end of content)
                let remaining = &content[pos..];
                let end_pos = remaining.find('\n').unwrap_or(remaining.len());
                return remaining[..end_pos].trim();
            }
        }
    }

    content.trim()
}

/// Extracts the comment content from a line (line-based fallback).
///
/// Supports common single-line comment styles:
/// - `//` (Rust, JS, TS, Java, C, C++, Swift, Scala, Go)
/// - `#` (Python, Bash, Perl, Ruby)
/// - `--` (Lua, SQL)
/// - `;;` (Lisp, Scheme)
fn extract_comment_line(line: &str) -> &str {
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

    // =========================================================================
    // Line-based parsing tests (existing tests)
    // =========================================================================

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

    // =========================================================================
    // Tree-sitter based parsing tests
    // =========================================================================

    fn parse_rust(source: &str) -> Tree {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_rust::LANGUAGE.into())
            .unwrap();
        parser.parse(source, None).unwrap()
    }

    fn parse_javascript(source: &str) -> Tree {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_javascript::LANGUAGE.into())
            .unwrap();
        parser.parse(source, None).unwrap()
    }

    fn parse_python(source: &str) -> Tree {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_python::LANGUAGE.into())
            .unwrap();
        parser.parse(source, None).unwrap()
    }

    #[test]
    fn tree_sitter_line_ignore_rust() {
        let source = r#"
fn main() {
    // tree-hugger-ignore
    let x = value.unwrap();
}
"#;
        let tree = parse_rust(source);
        let directives = IgnoreDirectives::parse_with_tree(source, &tree, ProgrammingLanguage::Rust);
        assert!(directives.should_ignore(4, Some("unwrap-call")));
        assert!(!directives.should_ignore(2, Some("unwrap-call")));
    }

    #[test]
    fn tree_sitter_file_ignore_rust() {
        let source = r#"
// tree-hugger-ignore-file: unwrap-call
fn main() {
    let x = value.unwrap();
    let y = other.unwrap();
}
"#;
        let tree = parse_rust(source);
        let directives = IgnoreDirectives::parse_with_tree(source, &tree, ProgrammingLanguage::Rust);
        assert!(directives.should_ignore(4, Some("unwrap-call")));
        assert!(directives.should_ignore(5, Some("unwrap-call")));
        assert!(!directives.should_ignore(4, Some("expect-call")));
    }

    #[test]
    fn tree_sitter_block_comment_rust() {
        let source = r#"
fn main() {
    /* tree-hugger-ignore: unwrap-call */
    let x = value.unwrap();
}
"#;
        let tree = parse_rust(source);
        let directives = IgnoreDirectives::parse_with_tree(source, &tree, ProgrammingLanguage::Rust);
        assert!(directives.should_ignore(4, Some("unwrap-call")));
    }

    #[test]
    fn tree_sitter_no_false_positive_string_rust() {
        // This is the key test: `//` inside a string should NOT be treated as a comment
        let source = r#"
fn main() {
    let x = "// tree-hugger-ignore";
    let y = value.unwrap();
}
"#;
        let tree = parse_rust(source);
        let directives = IgnoreDirectives::parse_with_tree(source, &tree, ProgrammingLanguage::Rust);
        // Line 4 should NOT be ignored because the directive is inside a string
        assert!(!directives.should_ignore(4, Some("unwrap-call")));
        assert!(!directives.has_directives());
    }

    #[test]
    fn tree_sitter_no_false_positive_string_javascript() {
        let source = r#"
function main() {
    const x = "// tree-hugger-ignore";
    const y = value.unwrap();
}
"#;
        let tree = parse_javascript(source);
        let directives =
            IgnoreDirectives::parse_with_tree(source, &tree, ProgrammingLanguage::JavaScript);
        assert!(!directives.should_ignore(4, Some("unwrap-call")));
        assert!(!directives.has_directives());
    }

    #[test]
    fn tree_sitter_no_false_positive_string_python() {
        let source = "
def main():
    x = \"# tree-hugger-ignore\"
    y = value.unwrap()
";
        let tree = parse_python(source);
        let directives =
            IgnoreDirectives::parse_with_tree(source, &tree, ProgrammingLanguage::Python);
        assert!(!directives.should_ignore(4, Some("unwrap-call")));
        assert!(!directives.has_directives());
    }

    #[test]
    fn tree_sitter_multiline_block_comment_rust() {
        let source = r#"
fn main() {
    /*
     * tree-hugger-ignore: unwrap-call
     */
    let x = value.unwrap();
}
"#;
        let tree = parse_rust(source);
        let directives = IgnoreDirectives::parse_with_tree(source, &tree, ProgrammingLanguage::Rust);
        // The block comment ends on line 5, so it affects line 6
        assert!(directives.should_ignore(6, Some("unwrap-call")));
    }

    #[test]
    fn tree_sitter_javascript_line_comment() {
        let source = r#"
function main() {
    // tree-hugger-ignore: eval-call
    eval("code");
}
"#;
        let tree = parse_javascript(source);
        let directives =
            IgnoreDirectives::parse_with_tree(source, &tree, ProgrammingLanguage::JavaScript);
        assert!(directives.should_ignore(4, Some("eval-call")));
    }

    #[test]
    fn tree_sitter_javascript_block_comment() {
        let source = r#"
function main() {
    /* tree-hugger-ignore-file: debugger-statement */
    debugger;
    debugger;
}
"#;
        let tree = parse_javascript(source);
        let directives =
            IgnoreDirectives::parse_with_tree(source, &tree, ProgrammingLanguage::JavaScript);
        assert!(directives.should_ignore(4, Some("debugger-statement")));
        assert!(directives.should_ignore(5, Some("debugger-statement")));
    }

    #[test]
    fn tree_sitter_python_comment() {
        let source = r#"
def main():
    # tree-hugger-ignore: exec-call
    exec("code")
"#;
        let tree = parse_python(source);
        let directives =
            IgnoreDirectives::parse_with_tree(source, &tree, ProgrammingLanguage::Python);
        assert!(directives.should_ignore(4, Some("exec-call")));
    }

    #[test]
    fn extract_comment_content_handles_various_styles() {
        // Line comments
        assert_eq!(extract_comment_content("// tree-hugger-ignore"), "tree-hugger-ignore");
        assert_eq!(extract_comment_content("# tree-hugger-ignore"), "tree-hugger-ignore");
        assert_eq!(extract_comment_content("-- tree-hugger-ignore"), "tree-hugger-ignore");
        assert_eq!(extract_comment_content("; tree-hugger-ignore"), "tree-hugger-ignore");

        // Block comments
        assert_eq!(extract_comment_content("/* tree-hugger-ignore */"), "tree-hugger-ignore");
        assert_eq!(extract_comment_content("(* tree-hugger-ignore *)"), "tree-hugger-ignore");
        assert_eq!(extract_comment_content("--[[ tree-hugger-ignore ]]"), "tree-hugger-ignore");

        // Doc comments
        assert_eq!(extract_comment_content("/// tree-hugger-ignore"), "tree-hugger-ignore");
        assert_eq!(extract_comment_content("//! tree-hugger-ignore"), "tree-hugger-ignore");
        assert_eq!(extract_comment_content("/** tree-hugger-ignore */"), "tree-hugger-ignore");
    }
}
