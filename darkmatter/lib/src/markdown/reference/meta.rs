//! Meta tag parsing and frontmatter merge.
//!
//! Extracts `<meta>` tags from document content and provides utilities
//! to merge meta values into frontmatter.

use indexmap::IndexMap;

use crate::markdown::Markdown;
use crate::markdown::compose::ComposeSource;
use crate::markdown::types::MarkdownResult;
use super::html;

/// A meta tag value (single or multiple).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MetaValue {
    /// A single string value.
    String(String),
    /// Multiple values for the same key.
    Many(Vec<String>),
}

impl MetaValue {
    /// Returns the first value as a string.
    pub fn as_str(&self) -> &str {
        match self {
            Self::String(s) => s,
            Self::Many(v) => v.first().map(|s| s.as_str()).unwrap_or(""),
        }
    }

    /// Promotes to `Many` and appends a new value.
    pub fn push(&mut self, value: String) {
        match self {
            Self::String(existing) => {
                *self = Self::Many(vec![existing.clone(), value]);
            }
            Self::Many(values) => {
                values.push(value);
            }
        }
    }
}

/// Ordered map of meta tag keys to values.
pub type MetaTagMap = IndexMap<String, MetaValue>;

/// Parse `<meta>` tags from document content into a normalized map.
///
/// Key priority:
/// 1. `name` attribute
/// 2. `property` attribute (Open Graph)
/// 3. `http-equiv` attribute
/// 4. `charset` attribute
///
/// Value source is the `content` attribute, except `charset` which uses
/// the attribute value directly.
pub fn parse_meta_tags(content: &str) -> MetaTagMap {
    let records = html::extract_html_meta_tags(content, &ComposeSource::Unknown);
    let mut map = MetaTagMap::new();

    for record in &records {
        let attrs = &record.attributes;

        // Determine the key
        let key = if let Some(name) = attrs.get("name").and_then(|v| v.as_str()) {
            name.to_string()
        } else if let Some(property) = attrs.get("property").and_then(|v| v.as_str()) {
            property.to_string()
        } else if let Some(http_equiv) = attrs.get("http-equiv").and_then(|v| v.as_str()) {
            http_equiv.to_string()
        } else if attrs.get("charset").and_then(|v| v.as_str()).is_some() {
            "charset".to_string()
        } else {
            continue; // No recognizable key attribute
        };

        // Determine the value
        let value = if key == "charset" {
            attrs
                .get("charset")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string()
        } else {
            attrs
                .get("content")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string()
        };

        // Insert or merge
        if let Some(existing) = map.get_mut(&key) {
            existing.push(value);
        } else {
            map.insert(key, MetaValue::String(value));
        }
    }

    map
}

/// Merge meta tags into frontmatter.
///
/// Returns the number of keys inserted or updated.
pub fn merge_meta_into_frontmatter(
    meta: &MetaTagMap,
    md: &mut Markdown,
    overwrite: bool,
) -> MarkdownResult<usize> {
    let mut count = 0;

    for (key, value) in meta {
        let fm_value = match value {
            MetaValue::String(s) => serde_json::Value::String(s.clone()),
            MetaValue::Many(v) => {
                serde_json::Value::Array(v.iter().map(|s| serde_json::Value::String(s.clone())).collect())
            }
        };

        let existing: Option<serde_json::Value> = md.fm_get(key)?;
        if existing.is_none() || overwrite {
            md.fm_insert(key, fm_value)?;
            count += 1;
        }
    }

    Ok(count)
}

/// Sets a `<meta>` tag in the document content.
///
/// If a `<meta>` tag with the same key already exists, its `content`
/// attribute is updated. Otherwise, a new `<meta>` tag is appended
/// at the end of the document.
///
/// The `key` determines which HTML attribute is used:
/// - Keys starting with `og:` or containing `:` use `property`
/// - `charset` uses the `charset` attribute (no `content` attribute)
/// - All other keys use `name`
///
/// Returns the number of tags modified (0 = inserted new, 1 = updated existing).
pub fn set_meta_tag(md: &mut Markdown, key: &str, value: &str) -> usize {
    let content = md.content().to_string();

    // Build the new tag HTML
    let new_tag = build_meta_tag_html(key, value);

    // Try to find and replace an existing tag with the same key
    if let Some(updated) = replace_existing_meta_tag(&content, key, value) {
        *md.content_mut() = updated;
        return 1;
    }

    // No existing tag — append at the end
    let mut new_content = content;
    if !new_content.ends_with('\n') {
        new_content.push('\n');
    }
    new_content.push_str(&new_tag);
    new_content.push('\n');
    *md.content_mut() = new_content;
    0
}

/// Build the HTML string for a meta tag.
fn build_meta_tag_html(key: &str, value: &str) -> String {
    if key == "charset" {
        format!("<meta charset=\"{value}\">")
    } else if key.contains(':') {
        // Open Graph or namespaced property
        format!("<meta property=\"{key}\" content=\"{value}\">")
    } else {
        format!("<meta name=\"{key}\" content=\"{value}\">")
    }
}

/// Try to replace an existing `<meta>` tag that matches the given key.
///
/// Returns `Some(updated_content)` if a replacement was made, `None` otherwise.
fn replace_existing_meta_tag(content: &str, key: &str, new_value: &str) -> Option<String> {
    // Build patterns to match existing meta tags for this key
    let patterns: Vec<String> = if key == "charset" {
        vec![
            format!("<meta charset=\""),
            format!("<meta charset='"),
        ]
    } else if key.contains(':') {
        vec![
            format!("<meta property=\"{key}\""),
            format!("<meta property='{key}'"),
        ]
    } else {
        vec![
            format!("<meta name=\"{key}\""),
            format!("<meta name='{key}'"),
        ]
    };

    for pattern in &patterns {
        if let Some(start) = content.find(pattern.as_str()) {
            // Find the end of the tag
            let rest = &content[start..];
            let end = rest.find('>').map(|i| start + i + 1)?;

            let new_tag = build_meta_tag_html(key, new_value);
            let mut result = String::with_capacity(content.len());
            result.push_str(&content[..start]);
            result.push_str(&new_tag);
            result.push_str(&content[end..]);
            return Some(result);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::Markdown;

    #[test]
    fn parse_meta_name() {
        let content = r#"<meta name="author" content="Ken">"#;
        let map = parse_meta_tags(content);
        assert_eq!(map.get("author").unwrap().as_str(), "Ken");
    }

    #[test]
    fn parse_meta_og_property() {
        let content = r#"<meta property="og:title" content="Hello">"#;
        let map = parse_meta_tags(content);
        assert_eq!(map.get("og:title").unwrap().as_str(), "Hello");
    }

    #[test]
    fn parse_meta_charset() {
        let content = r#"<meta charset="utf-8">"#;
        let map = parse_meta_tags(content);
        assert_eq!(map.get("charset").unwrap().as_str(), "utf-8");
    }

    #[test]
    fn parse_meta_duplicates() {
        let content = r#"<meta name="keywords" content="rust">

<meta name="keywords" content="markdown">"#;
        let map = parse_meta_tags(content);
        match map.get("keywords").unwrap() {
            MetaValue::Many(v) => {
                assert_eq!(v, &["rust", "markdown"]);
            }
            _ => panic!("expected Many"),
        }
    }

    #[test]
    fn merge_into_empty_frontmatter() {
        let mut md = Markdown::new("# Hello");
        let mut meta = MetaTagMap::new();
        meta.insert("author".into(), MetaValue::String("Ken".into()));
        meta.insert("description".into(), MetaValue::String("A doc".into()));

        let count = merge_meta_into_frontmatter(&meta, &mut md, false).unwrap();
        assert_eq!(count, 2);
        let author: Option<String> = md.fm_get("author").unwrap();
        assert_eq!(author, Some("Ken".to_string()));
    }

    #[test]
    fn merge_no_overwrite() {
        let mut md: Markdown = "---\nauthor: Alice\n---\n# Hello".into();
        let mut meta = MetaTagMap::new();
        meta.insert("author".into(), MetaValue::String("Ken".into()));

        let count = merge_meta_into_frontmatter(&meta, &mut md, false).unwrap();
        assert_eq!(count, 0);
        let author: Option<String> = md.fm_get("author").unwrap();
        assert_eq!(author, Some("Alice".to_string()));
    }

    #[test]
    fn merge_with_overwrite() {
        let mut md: Markdown = "---\nauthor: Alice\n---\n# Hello".into();
        let mut meta = MetaTagMap::new();
        meta.insert("author".into(), MetaValue::String("Ken".into()));

        let count = merge_meta_into_frontmatter(&meta, &mut md, true).unwrap();
        assert_eq!(count, 1);
        let author: Option<String> = md.fm_get("author").unwrap();
        assert_eq!(author, Some("Ken".to_string()));
    }

    // ── set_meta_tag tests ──────────────────────────────────────────

    #[test]
    fn set_meta_tag_insert_new() {
        let mut md = Markdown::new("# Hello");
        let updated = set_meta_tag(&mut md, "author", "Ken");
        assert_eq!(updated, 0, "should return 0 for new insert");

        let tags = parse_meta_tags(md.content());
        assert_eq!(tags.get("author").unwrap().as_str(), "Ken");
    }

    #[test]
    fn set_meta_tag_update_existing() {
        let mut md = Markdown::new("# Hello\n\n<meta name=\"author\" content=\"Alice\">\n");
        let updated = set_meta_tag(&mut md, "author", "Ken");
        assert_eq!(updated, 1, "should return 1 for update");

        let tags = parse_meta_tags(md.content());
        assert_eq!(tags.get("author").unwrap().as_str(), "Ken");
    }

    #[test]
    fn set_meta_tag_charset() {
        let mut md = Markdown::new("# Hello");
        set_meta_tag(&mut md, "charset", "utf-8");

        let tags = parse_meta_tags(md.content());
        assert_eq!(tags.get("charset").unwrap().as_str(), "utf-8");
        assert!(md.content().contains("<meta charset=\"utf-8\">"));
    }

    #[test]
    fn set_meta_tag_charset_update() {
        let mut md = Markdown::new("<meta charset=\"ascii\">\n\n# Hello");
        let updated = set_meta_tag(&mut md, "charset", "utf-8");
        assert_eq!(updated, 1);
        assert!(md.content().contains("<meta charset=\"utf-8\">"));
        assert!(!md.content().contains("ascii"));
    }

    #[test]
    fn set_meta_tag_og_property() {
        let mut md = Markdown::new("# Hello");
        set_meta_tag(&mut md, "og:title", "My Page");

        let tags = parse_meta_tags(md.content());
        assert_eq!(tags.get("og:title").unwrap().as_str(), "My Page");
        assert!(md.content().contains("<meta property=\"og:title\" content=\"My Page\">"));
    }

    #[test]
    fn set_meta_tag_og_property_update() {
        let mut md = Markdown::new("<meta property=\"og:title\" content=\"Old\">\n\n# Hello");
        let updated = set_meta_tag(&mut md, "og:title", "New Title");
        assert_eq!(updated, 1);

        let tags = parse_meta_tags(md.content());
        assert_eq!(tags.get("og:title").unwrap().as_str(), "New Title");
    }

    #[test]
    fn set_meta_tag_duplicate_key_updates_first() {
        let mut md = Markdown::new(
            "<meta name=\"keywords\" content=\"rust\">\n<meta name=\"keywords\" content=\"markdown\">\n",
        );
        let updated = set_meta_tag(&mut md, "keywords", "programming");
        assert_eq!(updated, 1, "should update the first occurrence");
        assert!(md.content().contains("programming"));
    }
}
