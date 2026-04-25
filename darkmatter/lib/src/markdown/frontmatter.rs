//! Frontmatter parsing and manipulation utilities.

use super::types::{FrontmatterMap, MarkdownError, MarkdownResult};
use biscuit_file::serde_yaml_ng;
use serde::Serialize;
use serde::de::DeserializeOwned;

/// Strategy for merging frontmatter fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MergeStrategy {
    /// Error if a field exists in both frontmatters.
    ErrorOnConflict,
    /// Prefer the external (incoming) value on conflict.
    PreferExternal,
    /// Prefer the document's existing value on conflict.
    PreferDocument,
}

/// Wrapper type for frontmatter with typed accessors.
#[derive(Debug, Clone, PartialEq)]
pub struct Frontmatter(FrontmatterMap);

impl Frontmatter {
    /// Creates a new empty frontmatter.
    pub fn new() -> Self {
        Self(FrontmatterMap::new())
    }

    /// Creates frontmatter from a map.
    pub fn from_map(map: FrontmatterMap) -> Self {
        Self(map)
    }

    /// Gets a typed value from frontmatter.
    ///
    /// ## Examples
    ///
    /// ```
    /// # use darkmatter::markdown::Frontmatter;
    /// # use serde_json::json;
    /// let mut fm = Frontmatter::new();
    /// fm.insert("title", json!("Hello"));
    /// let title: Option<String> = fm.get("title").unwrap();
    /// assert_eq!(title, Some("Hello".to_string()));
    /// ```
    pub fn get<T: DeserializeOwned>(&self, key: &str) -> MarkdownResult<Option<T>> {
        match self.0.get(key) {
            Some(value) => {
                let result = serde_json::from_value(value.clone())?;
                Ok(Some(result))
            }
            None => Ok(None),
        }
    }

    /// Inserts a value into frontmatter.
    pub fn insert<T: Serialize>(&mut self, key: &str, value: T) -> MarkdownResult<()> {
        let json_value = serde_json::to_value(value)?;
        self.0.insert(key.to_string(), json_value);
        Ok(())
    }

    /// Merges another value into this frontmatter using the specified strategy.
    ///
    /// ## Examples
    ///
    /// ```
    /// # use darkmatter::markdown::{Frontmatter, MergeStrategy};
    /// # use serde_json::json;
    /// let mut fm = Frontmatter::new();
    /// fm.insert("title", json!("Original")).unwrap();
    ///
    /// let other = json!({"author": "Alice"});
    /// fm.merge_with(&other, MergeStrategy::ErrorOnConflict).unwrap();
    ///
    /// let author: Option<String> = fm.get("author").unwrap();
    /// assert_eq!(author, Some("Alice".to_string()));
    /// ```
    pub fn merge_with<T: Serialize>(
        &mut self,
        other: T,
        strategy: MergeStrategy,
    ) -> MarkdownResult<()> {
        let other_value = serde_json::to_value(other)?;
        let other_map: FrontmatterMap = serde_json::from_value(other_value)?;

        for (key, value) in other_map {
            use indexmap::map::Entry;
            match self.0.entry(key) {
                Entry::Occupied(mut entry) => match strategy {
                    MergeStrategy::ErrorOnConflict => {
                        return Err(MarkdownError::FrontmatterMerge(format!(
                            "Conflicting key: {}",
                            entry.key()
                        )));
                    }
                    MergeStrategy::PreferExternal => {
                        entry.insert(value);
                    }
                    MergeStrategy::PreferDocument => {
                        // Keep existing, do nothing
                    }
                },
                Entry::Vacant(entry) => {
                    entry.insert(value);
                }
            }
        }

        Ok(())
    }

    /// Sets default values for missing keys.
    ///
    /// ## Examples
    ///
    /// ```
    /// # use darkmatter::markdown::Frontmatter;
    /// # use serde_json::json;
    /// let mut fm = Frontmatter::new();
    /// fm.insert("title", json!("My Document")).unwrap();
    ///
    /// let defaults = json!({"title": "Default", "author": "Anonymous"});
    /// fm.set_defaults(&defaults).unwrap();
    ///
    /// let title: Option<String> = fm.get("title").unwrap();
    /// let author: Option<String> = fm.get("author").unwrap();
    /// assert_eq!(title, Some("My Document".to_string()));
    /// assert_eq!(author, Some("Anonymous".to_string()));
    /// ```
    pub fn set_defaults<T: Serialize>(&mut self, defaults: T) -> MarkdownResult<()> {
        let defaults_value = serde_json::to_value(defaults)?;
        let defaults_map: FrontmatterMap = serde_json::from_value(defaults_value)?;

        for (key, value) in defaults_map {
            self.0.entry(key).or_insert(value);
        }

        Ok(())
    }

    /// Returns a reference to the underlying map.
    pub fn as_map(&self) -> &FrontmatterMap {
        &self.0
    }

    /// Returns a mutable reference to the underlying map.
    pub fn as_map_mut(&mut self) -> &mut FrontmatterMap {
        &mut self.0
    }

    /// Consumes self and returns the underlying map.
    pub fn into_map(self) -> FrontmatterMap {
        self.0
    }

    /// Returns true if frontmatter is empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns the number of frontmatter fields.
    pub fn len(&self) -> usize {
        self.0.len()
    }
}

impl Default for Frontmatter {
    fn default() -> Self {
        Self::new()
    }
}

/// Parses frontmatter from markdown content.
///
/// Frontmatter must be at the start of the document between `---` delimiters.
pub(super) fn parse_frontmatter(content: &str) -> MarkdownResult<(Frontmatter, String)> {
    let lines: Vec<&str> = content.lines().collect();

    // Check if document starts with frontmatter delimiter
    if lines.is_empty() || lines[0].trim() != "---" {
        return Ok((Frontmatter::new(), content.to_string()));
    }

    // Find closing delimiter
    let closing_idx = lines
        .iter()
        .skip(1)
        .position(|line| line.trim() == "---")
        .map(|idx| idx + 1); // Adjust for skip(1)

    let Some(closing_idx) = closing_idx else {
        // No closing delimiter, treat as regular content
        return Ok((Frontmatter::new(), content.to_string()));
    };

    // Extract YAML content between delimiters
    let yaml_lines = &lines[1..closing_idx];
    let yaml_content = yaml_lines.join("\n");

    // Parse YAML with fallback strategies:
    // 1. Direct parse
    // 2. Tab normalization
    // 3. Protect interpolation expressions ({{ }}) that may contain
    //    YAML-significant characters (e.g., quotes in fallback values)
    let frontmatter_map: FrontmatterMap = if yaml_content.trim().is_empty() {
        FrontmatterMap::new()
    } else {
        parse_yaml_with_fallbacks(&yaml_content)?
    };

    // Extract remaining content
    let content_lines = &lines[closing_idx + 1..];
    let remaining_content = content_lines.join("\n");

    Ok((Frontmatter::from_map(frontmatter_map), remaining_content))
}

// UTF-8-boundary audit (2026-04-24): the byte-indexed scanners in this file
// were the only `markdown/*` sites that copied `yaml[pos..pos + 1]` into a
// String. Sibling scanners under `markdown/` walk `as_bytes()` only for
// ASCII sentinel checks and never slice the source by an arbitrary `pos`.
/// Attempts to parse YAML with progressive fallback strategies.
///
/// 1. Direct parse
/// 2. Tab-normalized parse
/// 3. Expression-protected parse: temporarily replaces `$(...)` shell command
///    substitutions and `{{ }}` interpolation expressions with safe
///    placeholders so YAML-significant characters inside them (e.g., nested
///    double quotes in `"$(dirname "{{path}}")"`) don't break the YAML parser.
fn parse_yaml_with_fallbacks(yaml: &str) -> MarkdownResult<FrontmatterMap> {
    // Strategy 1: direct parse
    match serde_yaml_ng::from_str(yaml) {
        Ok(map) => Ok(map),
        Err(original_err) => {
            // Strategy 2: tab normalization
            let normalized = normalize_frontmatter_indentation(yaml);
            if normalized != *yaml
                && let Ok(map) = serde_yaml_ng::from_str(&normalized)
            {
                return Ok(map);
            }

            // Strategy 3: protect shell and interpolation expressions.
            // Shell protection runs first so a `{{ }}` placeholder never
            // swallows a surrounding `$(...)` it was embedded in; shell
            // replacements are applied last during restoration so any
            // shell placeholder that survives inside a restored expression
            // body (e.g., `{{ fallback || "$(pwd)" }}`) resolves correctly.
            let target = if normalized != *yaml {
                normalized.as_str()
            } else {
                yaml
            };

            let (shell_protected, shell_replacements) = protect_shell_expressions(target);
            let (fully_protected, expr_replacements) =
                protect_interpolation_expressions(&shell_protected);

            if shell_replacements.is_empty() && expr_replacements.is_empty() {
                return Err(original_err.into());
            }

            match serde_yaml_ng::from_str::<FrontmatterMap>(&fully_protected) {
                Ok(map) => {
                    let map = restore_expressions_in_map(map, &expr_replacements);
                    let map = restore_expressions_in_map(map, &shell_replacements);
                    Ok(map)
                }
                Err(_) => Err(original_err.into()),
            }
        }
    }
}

/// Replaces `$(...)` shell command substitution bodies with safe placeholders.
///
/// Shell command substitutions inside double-quoted YAML values often contain
/// their own double quotes (e.g., `"$(dirname "{{path}}")"`), which breaks
/// standard YAML parsing. This helper walks the raw YAML text, finds balanced
/// `$(...)` spans (tracking nested parentheses), and replaces each with an
/// opaque `__DM_SHELL_N__` placeholder so the remaining YAML parses cleanly.
/// Placeholders are later restored in parsed scalar values via
/// [`restore_expressions_in_map`].
///
/// The scanner is quoting-agnostic: it does not attempt to understand shell
/// quoting rules, because any valid shell `$(...)` must have balanced
/// parentheses regardless of inner quoting. Backtick substitutions and process
/// substitutions (`<(...)`, `>(...)`) are not protected — add them when a
/// real case emerges.
///
/// Non-matched content advances by full UTF-8 scalar boundaries (via
/// `yaml[pos..].chars().next()` + `char.len_utf8()`) so `pos` never lands
/// inside a multi-byte scalar.
fn protect_shell_expressions(yaml: &str) -> (String, Vec<(String, String)>) {
    let mut result = String::with_capacity(yaml.len());
    let mut replacements = Vec::new();
    let bytes = yaml.as_bytes();
    let mut pos = 0;

    while pos < bytes.len() {
        if pos + 1 < bytes.len() && bytes[pos] == b'$' && bytes[pos + 1] == b'(' {
            let start = pos;
            pos += 2;
            let mut depth = 1usize;
            while pos < bytes.len() && depth > 0 {
                match bytes[pos] {
                    b'(' => {
                        depth += 1;
                        pos += 1;
                    }
                    b')' => {
                        depth -= 1;
                        pos += 1;
                        if depth == 0 {
                            break;
                        }
                    }
                    _ => pos += 1,
                }
            }

            if depth != 0 {
                // Unbalanced: emit the rest verbatim and stop scanning for
                // shell expressions. Returning an unclosed placeholder would
                // leave a token in the YAML that never gets restored.
                result.push_str(&yaml[start..]);
                return (result, replacements);
            }

            let original = &yaml[start..pos];
            let placeholder = format!("__DM_SHELL_{}__", replacements.len());
            replacements.push((placeholder.clone(), original.to_string()));
            result.push_str(&placeholder);
        } else if let Some(ch) = yaml[pos..].chars().next() {
            // Non-matching byte: advance by a full UTF-8 scalar so `pos`
            // always lands on a char boundary. Slicing `yaml[pos..pos + 1]`
            // here would panic when `pos` falls inside a multi-byte scalar.
            result.push(ch);
            pos += ch.len_utf8();
        } else {
            break;
        }
    }

    (result, replacements)
}

/// Replaces `{{ }}` expression bodies with safe placeholders.
///
/// Returns the modified string and a map of placeholder → original expression.
///
/// Non-matched content advances by full UTF-8 scalar boundaries (via
/// `yaml[pos..].chars().next()` + `char.len_utf8()`) so `pos` never lands
/// inside a multi-byte scalar.
fn protect_interpolation_expressions(yaml: &str) -> (String, Vec<(String, String)>) {
    let mut result = String::with_capacity(yaml.len());
    let mut replacements = Vec::new();
    let bytes = yaml.as_bytes();
    let mut pos = 0;

    while pos < bytes.len() {
        if pos + 1 < bytes.len() && bytes[pos] == b'{' && bytes[pos + 1] == b'{' {
            // Find matching }}
            let start = pos;
            pos += 2;
            let mut depth = 1;
            while pos + 1 < bytes.len() && depth > 0 {
                if bytes[pos] == b'{' && bytes[pos + 1] == b'{' {
                    depth += 1;
                    pos += 2;
                } else if bytes[pos] == b'}' && bytes[pos + 1] == b'}' {
                    depth -= 1;
                    if depth == 0 {
                        pos += 2;
                        break;
                    }
                    pos += 2;
                } else {
                    pos += 1;
                }
            }
            let original = &yaml[start..pos];
            let placeholder = format!("__DM_EXPR_{}__", replacements.len());
            replacements.push((placeholder.clone(), original.to_string()));
            result.push_str(&placeholder);
        } else if let Some(ch) = yaml[pos..].chars().next() {
            // Non-matching byte: advance by a full UTF-8 scalar so `pos`
            // always lands on a char boundary. Slicing `yaml[pos..pos + 1]`
            // here would panic when `pos` falls inside a multi-byte scalar.
            result.push(ch);
            pos += ch.len_utf8();
        } else {
            break;
        }
    }

    (result, replacements)
}

/// Restores interpolation expressions from placeholders in parsed YAML values.
fn restore_expressions_in_map(
    map: FrontmatterMap,
    replacements: &[(String, String)],
) -> FrontmatterMap {
    map.into_iter()
        .map(|(k, v)| (k, restore_expressions_in_value(v, replacements)))
        .collect()
}

/// Recursively restores placeholders in a JSON value tree.
fn restore_expressions_in_value(
    value: serde_json::Value,
    replacements: &[(String, String)],
) -> serde_json::Value {
    match value {
        serde_json::Value::String(s) => {
            let mut restored = s;
            for (placeholder, original) in replacements {
                restored = restored.replace(placeholder, original);
            }
            serde_json::Value::String(restored)
        }
        serde_json::Value::Array(arr) => serde_json::Value::Array(
            arr.into_iter()
                .map(|v| restore_expressions_in_value(v, replacements))
                .collect(),
        ),
        serde_json::Value::Object(obj) => serde_json::Value::Object(
            obj.into_iter()
                .map(|(k, v)| (k, restore_expressions_in_value(v, replacements)))
                .collect(),
        ),
        other => other,
    }
}

/// Normalizes frontmatter indentation by replacing leading tabs with spaces.
///
/// Only tabs in the leading indentation prefix are rewritten (1 tab = 2 spaces).
/// Tabs after the first non-whitespace character in a line are preserved.
fn normalize_frontmatter_indentation(yaml: &str) -> String {
    let mut normalized = String::with_capacity(yaml.len());

    for (idx, line) in yaml.split('\n').enumerate() {
        if idx > 0 {
            normalized.push('\n');
        }

        let mut saw_tab_in_indent = false;
        let mut indent_end = 0usize;
        let mut indent = String::new();

        for (byte_idx, ch) in line.char_indices() {
            match ch {
                ' ' => {
                    indent.push(' ');
                    indent_end = byte_idx + ch.len_utf8();
                }
                '\t' => {
                    indent.push_str("  ");
                    saw_tab_in_indent = true;
                    indent_end = byte_idx + ch.len_utf8();
                }
                _ => break,
            }
        }

        if !saw_tab_in_indent {
            normalized.push_str(line);
            continue;
        }

        normalized.push_str(&indent);
        normalized.push_str(&line[indent_end..]);
    }

    normalized
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_frontmatter_get() {
        let mut fm = Frontmatter::new();
        fm.insert("title", json!("Test")).unwrap();

        let title: Option<String> = fm.get("title").unwrap();
        assert_eq!(title, Some("Test".to_string()));

        let missing: Option<String> = fm.get("missing").unwrap();
        assert_eq!(missing, None);
    }

    #[test]
    fn test_frontmatter_merge_no_conflict() {
        let mut fm = Frontmatter::new();
        fm.insert("title", json!("Original")).unwrap();

        let other = json!({"author": "Alice"});
        fm.merge_with(&other, MergeStrategy::ErrorOnConflict)
            .unwrap();

        let author: Option<String> = fm.get("author").unwrap();
        assert_eq!(author, Some("Alice".to_string()));
    }

    #[test]
    fn test_frontmatter_merge_conflict_error() {
        let mut fm = Frontmatter::new();
        fm.insert("title", json!("Original")).unwrap();

        let other = json!({"title": "New"});
        let result = fm.merge_with(&other, MergeStrategy::ErrorOnConflict);

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            MarkdownError::FrontmatterMerge(_)
        ));
    }

    #[test]
    fn test_frontmatter_merge_prefer_external() {
        let mut fm = Frontmatter::new();
        fm.insert("title", json!("Original")).unwrap();

        let other = json!({"title": "New"});
        fm.merge_with(&other, MergeStrategy::PreferExternal)
            .unwrap();

        let title: Option<String> = fm.get("title").unwrap();
        assert_eq!(title, Some("New".to_string()));
    }

    #[test]
    fn test_frontmatter_merge_prefer_document() {
        let mut fm = Frontmatter::new();
        fm.insert("title", json!("Original")).unwrap();

        let other = json!({"title": "New"});
        fm.merge_with(&other, MergeStrategy::PreferDocument)
            .unwrap();

        let title: Option<String> = fm.get("title").unwrap();
        assert_eq!(title, Some("Original".to_string()));
    }

    #[test]
    fn test_frontmatter_set_defaults() {
        let mut fm = Frontmatter::new();
        fm.insert("title", json!("My Document")).unwrap();

        let defaults = json!({"title": "Default", "author": "Anonymous"});
        fm.set_defaults(&defaults).unwrap();

        let title: Option<String> = fm.get("title").unwrap();
        let author: Option<String> = fm.get("author").unwrap();
        assert_eq!(title, Some("My Document".to_string()));
        assert_eq!(author, Some("Anonymous".to_string()));
    }

    #[test]
    fn test_parse_frontmatter_with_yaml() {
        let content = r#"---
title: Test Document
author: Alice
---
# Hello World

This is content."#;

        let (fm, remaining) = parse_frontmatter(content).unwrap();
        let title: Option<String> = fm.get("title").unwrap();
        let author: Option<String> = fm.get("author").unwrap();

        assert_eq!(title, Some("Test Document".to_string()));
        assert_eq!(author, Some("Alice".to_string()));
        assert!(remaining.contains("# Hello World"));
    }

    #[test]
    fn test_parse_frontmatter_no_frontmatter() {
        let content = "# Hello World\n\nNo frontmatter here.";

        let (fm, remaining) = parse_frontmatter(content).unwrap();

        assert!(fm.is_empty());
        assert_eq!(remaining, content);
    }

    #[test]
    fn test_parse_frontmatter_empty_frontmatter() {
        let content = "---\n---\n# Content";

        let (fm, remaining) = parse_frontmatter(content).unwrap();

        assert!(fm.is_empty());
        assert_eq!(remaining, "# Content");
    }

    #[test]
    fn test_parse_frontmatter_no_closing_delimiter() {
        let content = "---\ntitle: Test\n# Content";

        let (fm, remaining) = parse_frontmatter(content).unwrap();

        assert!(fm.is_empty());
        assert_eq!(remaining, content);
    }

    #[test]
    fn test_parse_frontmatter_with_tab_indentation_block_scalar() {
        let content = "---\nprompt: |-\n\tLine one\n\tLine two\nlast_updated: 2026-02-27\n---\n# macOS Audio\n";

        let (fm, remaining) = parse_frontmatter(content).unwrap();
        let prompt: Option<String> = fm.get("prompt").unwrap();
        let last_updated: Option<String> = fm.get("last_updated").unwrap();

        assert_eq!(prompt, Some("Line one\nLine two".to_string()));
        assert_eq!(last_updated, Some("2026-02-27".to_string()));
        assert!(remaining.starts_with("# macOS Audio"));
    }

    #[test]
    fn test_normalize_frontmatter_indentation_only_changes_leading_tabs() {
        let input = "key: \"a\tb\"\n \tchild: true\n\t\tgrandchild: 1";
        let normalized = normalize_frontmatter_indentation(input);

        assert_eq!(
            normalized,
            "key: \"a\tb\"\n   child: true\n    grandchild: 1"
        );
    }

    #[test]
    fn test_parse_frontmatter_with_nested_quotes_in_interpolation() {
        // Double quotes inside {{ }} expressions break standard YAML parsing.
        // The expression-protection fallback should handle this.
        let content = "---\ntopic: \"\"\nplan_file: \"prefix/{{topic}}/{{plan || \"plan.md\"}}\"\n---\n# Body\n";

        let (fm, remaining) = parse_frontmatter(content).unwrap();
        let topic: Option<String> = fm.get("topic").unwrap();
        let plan_file: Option<String> = fm.get("plan_file").unwrap();

        assert_eq!(topic, Some(String::new()));
        assert_eq!(
            plan_file,
            Some("prefix/{{topic}}/{{plan || \"plan.md\"}}".to_string())
        );
        assert!(remaining.starts_with("# Body"));
    }

    #[test]
    fn test_protect_interpolation_expressions() {
        let yaml = r#"key: "{{foo || "bar"}}""#;
        let (protected, replacements) = protect_interpolation_expressions(yaml);

        assert_eq!(replacements.len(), 1);
        assert!(!protected.contains("{{"));
        assert!(protected.contains("__DM_EXPR_0__"));
        assert_eq!(replacements[0].1, r#"{{foo || "bar"}}"#);
    }

    #[test]
    fn test_protect_multiple_expressions() {
        let yaml = r#"a: "{{x}}/{{y || "z"}}""#;
        let (protected, replacements) = protect_interpolation_expressions(yaml);

        assert_eq!(replacements.len(), 2);
        assert!(protected.contains("__DM_EXPR_0__"));
        assert!(protected.contains("__DM_EXPR_1__"));
    }

    #[test]
    fn test_protect_interpolation_expressions_unicode_outside_sentinel_preserved() {
        // Pre-fix: this panics with "byte index N is not a char boundary"
        // because the non-matching branch sliced `yaml[pos..pos + 1]` while
        // `pos` was inside the multi-byte scalar for the emoji.
        let yaml = "key: \"\u{1F5A5}\u{FE0F} prefix {{var}} suffix\"";
        let (protected, replacements) = protect_interpolation_expressions(yaml);

        // Sentinel still detected and replaced.
        assert_eq!(replacements.len(), 1);
        assert_eq!(replacements[0].1, "{{var}}");
        assert!(protected.contains("__DM_EXPR_0__"));

        // Non-ASCII characters outside the sentinel are preserved byte-identically.
        assert!(protected.contains('\u{1F5A5}'));
        assert!(protected.contains('\u{FE0F}'));
        assert!(protected.contains("prefix"));
        assert!(protected.contains("suffix"));

        // No lossy `'?'` substitution survives in the output.
        assert!(!protected.contains('?'));
    }

    #[test]
    fn test_restore_expressions_in_value() {
        let replacements = vec![
            ("__DM_EXPR_0__".to_string(), "{{foo}}".to_string()),
            (
                "__DM_EXPR_1__".to_string(),
                "{{bar || \"baz\"}}".to_string(),
            ),
        ];

        let value = json!("prefix/__DM_EXPR_0__/__DM_EXPR_1__");
        let restored = restore_expressions_in_value(value, &replacements);
        assert_eq!(restored, json!("prefix/{{foo}}/{{bar || \"baz\"}}"));
    }

    #[test]
    fn test_parse_frontmatter_with_nested_quotes_in_shell_expression() {
        // Shell command substitutions with quoted arguments inside a double-quoted
        // YAML value break the standard YAML parser. The fallback should protect
        // `$(...)` expressions so the value is preserved verbatim for later
        // frontmatter shell expansion.
        let content = concat!(
            "---\n",
            "review: \"reviews/foo.md\"\n",
            "dir: \"$(dirname \"reviews/foo.md\")\"\n",
            "basename: \"$(basename \"reviews/foo.md\" .md)\"\n",
            "---\n",
            "# Body\n",
        );

        let (fm, remaining) = parse_frontmatter(content).unwrap();
        let review: Option<String> = fm.get("review").unwrap();
        let dir: Option<String> = fm.get("dir").unwrap();
        let basename: Option<String> = fm.get("basename").unwrap();

        assert_eq!(review, Some("reviews/foo.md".to_string()));
        assert_eq!(dir, Some("$(dirname \"reviews/foo.md\")".to_string()));
        assert_eq!(
            basename,
            Some("$(basename \"reviews/foo.md\" .md)".to_string())
        );
        assert!(remaining.starts_with("# Body"));
    }

    #[test]
    fn test_parse_frontmatter_with_interpolation_inside_shell_expression() {
        // Classic user pattern: shell substitution wrapping an interpolation.
        // Both protections must coexist so `{{review}}` survives for later
        // frontmatter interpolation and `$(dirname ...)` survives for later
        // frontmatter shell expansion.
        let content = concat!(
            "---\n",
            "review: \"\"\n",
            "dir: \"$(dirname \"{{review}}\")\"\n",
            "basename: \"$(basename \"{{review}}\" .md)\"\n",
            "plan: \"{{dir}}/plan-for-{{basename}}.md\"\n",
            "---\n",
            "Body: {{review}}\n",
        );

        let (fm, _remaining) = parse_frontmatter(content).unwrap();
        let dir: Option<String> = fm.get("dir").unwrap();
        let basename: Option<String> = fm.get("basename").unwrap();
        let plan: Option<String> = fm.get("plan").unwrap();

        assert_eq!(dir, Some("$(dirname \"{{review}}\")".to_string()));
        assert_eq!(basename, Some("$(basename \"{{review}}\" .md)".to_string()));
        assert_eq!(plan, Some("{{dir}}/plan-for-{{basename}}.md".to_string()));
    }

    #[test]
    fn test_protect_shell_expressions_basic() {
        let yaml = r#"key: "$(dirname "path/file.md")""#;
        let (protected, replacements) = protect_shell_expressions(yaml);

        assert_eq!(replacements.len(), 1);
        assert!(!protected.contains("$("));
        assert!(protected.contains("__DM_SHELL_0__"));
        assert_eq!(replacements[0].1, r#"$(dirname "path/file.md")"#);
    }

    #[test]
    fn test_protect_shell_expressions_nested_parens() {
        // Nested parens are common in shell: `$(cmd (inner) arg)` or similar.
        let yaml = r#"key: "$(echo $(date))""#;
        let (protected, replacements) = protect_shell_expressions(yaml);

        assert_eq!(replacements.len(), 1);
        assert_eq!(replacements[0].1, r#"$(echo $(date))"#);
        assert!(protected.contains("__DM_SHELL_0__"));
    }

    #[test]
    fn test_protect_shell_expressions_multiple() {
        let yaml = r#"a: "$(echo a)"
b: "$(echo b)""#;
        let (protected, replacements) = protect_shell_expressions(yaml);

        assert_eq!(replacements.len(), 2);
        assert!(protected.contains("__DM_SHELL_0__"));
        assert!(protected.contains("__DM_SHELL_1__"));
    }

    #[test]
    fn test_protect_shell_expressions_unicode_outside_sentinel_preserved() {
        // Pre-fix: this panics with "byte index N is not a char boundary"
        // because the non-matching branch sliced `yaml[pos..pos + 1]` while
        // `pos` was inside the multi-byte scalar for the emoji.
        let yaml = "key: \"\u{1F5A5}\u{FE0F} prefix $(echo hi) suffix\"";
        let (protected, replacements) = protect_shell_expressions(yaml);

        // Sentinel still detected and replaced.
        assert_eq!(replacements.len(), 1);
        assert_eq!(replacements[0].1, "$(echo hi)");
        assert!(protected.contains("__DM_SHELL_0__"));

        // Non-ASCII characters outside the sentinel are preserved byte-identically.
        assert!(protected.contains('\u{1F5A5}'));
        assert!(protected.contains('\u{FE0F}'));
        assert!(protected.contains("prefix"));
        assert!(protected.contains("suffix"));

        // No lossy `'?'` substitution survives in the output.
        assert!(!protected.contains('?'));
    }

    #[test]
    fn test_markdown_from_str_with_shell_in_frontmatter_strips_frontmatter() {
        use crate::markdown::Markdown;

        // Regression test: when frontmatter contains shell substitutions
        // with nested quotes, parsing must still succeed so the raw
        // `---...---` block does NOT leak into `content()`.
        let content = concat!(
            "---\n",
            "dir: \"$(dirname \"x.md\")\"\n",
            "---\n",
            "# Body\n",
        );

        let md: Markdown = content.into();
        assert!(
            !md.frontmatter().is_empty(),
            "frontmatter should parse successfully"
        );
        assert!(
            !md.content().contains("---"),
            "content must not contain the raw frontmatter delimiters; got: {:?}",
            md.content()
        );
        assert!(md.content().starts_with("# Body"));
    }

    #[test]
    fn test_parse_frontmatter_with_emoji_value_and_unquoted_interpolation() {
        // Reproduction from the spec: an unquoted `{{...}}` value forces
        // parse_yaml_with_fallbacks() into the byte-scanning fallback, and
        // the multi-byte emoji in a quoted value used to panic the scanner.
        // Both must round-trip verbatim through the fallback path.
        let content = concat!(
            "---\n",
            "area: {{ctx.current_package_area}}\n",
            "success_message: \"\u{1F5A5}\u{FE0F} Build succeeded\"\n",
            "---\n",
            "# Body\n",
        );

        let (fm, remaining) = parse_frontmatter(content).unwrap();
        let area: Option<String> = fm.get("area").unwrap();
        let message: Option<String> = fm.get("success_message").unwrap();

        assert_eq!(area, Some("{{ctx.current_package_area}}".to_string()));
        assert_eq!(message, Some("\u{1F5A5}\u{FE0F} Build succeeded".to_string()));
        assert!(remaining.starts_with("# Body"));
    }
}
