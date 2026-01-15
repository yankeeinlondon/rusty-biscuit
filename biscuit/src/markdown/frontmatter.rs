//! Frontmatter parsing and manipulation utilities.

use super::types::{FrontmatterMap, MarkdownError, MarkdownResult};
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::collections::HashMap;

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
        Self(HashMap::new())
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
    /// # use shared::markdown::Frontmatter;
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
    /// # use shared::markdown::{Frontmatter, MergeStrategy};
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
            use std::collections::hash_map::Entry;
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
    /// # use shared::markdown::Frontmatter;
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

    // Parse YAML
    let frontmatter_map: FrontmatterMap = if yaml_content.trim().is_empty() {
        HashMap::new()
    } else {
        serde_yaml::from_str(&yaml_content)?
    };

    // Extract remaining content
    let content_lines = &lines[closing_idx + 1..];
    let remaining_content = content_lines.join("\n");

    Ok((Frontmatter::from_map(frontmatter_map), remaining_content))
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
}
