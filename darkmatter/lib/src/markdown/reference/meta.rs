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
/// Scans for `<meta` tag boundaries and uses attribute extraction to identify
/// matching tags regardless of attribute order, extra whitespace, or additional
/// attributes. Only the first matching tag is replaced.
///
/// Returns `Some(updated_content)` if a replacement was made, `None` otherwise.
fn replace_existing_meta_tag(content: &str, key: &str, new_value: &str) -> Option<String> {
    // Determine which attribute identifies this key
    let (attr_name, attr_value): (&str, Option<&str>) = if key == "charset" {
        ("charset", None) // charset matches any value
    } else if key.contains(':') {
        ("property", Some(key))
    } else {
        ("name", Some(key))
    };

    // Scan for <meta tags (case-insensitive opening)
    let lower = content.to_lowercase();
    let mut search_from = 0;

    while let Some(rel_start) = lower[search_from..].find("<meta") {
        let start = search_from + rel_start;
        let rest = &content[start..];

        // Verify it's actually a tag opening (next non-alpha char must be space or >)
        let after_meta = &rest["<meta".len()..];
        if !after_meta.starts_with(|c: char| c.is_ascii_whitespace() || c == '>' || c == '/') {
            search_from = start + 1;
            continue;
        }

        // Find the closing >
        let Some(close_offset) = rest.find('>') else {
            search_from = start + 1;
            continue;
        };
        let tag_end = start + close_offset + 1;
        let tag_html = &content[start..tag_end];

        // Use attribute extraction to check if this tag matches the key
        let matches = if let Some(expected_value) = attr_value {
            html::extract_attribute(tag_html, attr_name)
                .map(|v| v == expected_value)
                .unwrap_or(false)
        } else {
            // charset: any tag with a charset attribute matches
            html::extract_attribute(tag_html, attr_name).is_some()
        };

        if matches {
            let new_tag = build_meta_tag_html(key, new_value);
            let mut result = String::with_capacity(content.len());
            result.push_str(&content[..start]);
            result.push_str(&new_tag);
            result.push_str(&content[tag_end..]);
            return Some(result);
        }

        search_from = tag_end;
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

    // ── edge-case meta tag replacement tests ───────────────────────

    #[test]
    fn set_meta_tag_reordered_attributes() {
        // content before name — attribute order reversed
        let mut md = Markdown::new("<meta content=\"Alice\" name=\"author\">\n");
        let updated = set_meta_tag(&mut md, "author", "Ken");
        assert_eq!(updated, 1, "should find tag despite reordered attributes");

        let tags = parse_meta_tags(md.content());
        assert_eq!(tags.get("author").unwrap().as_str(), "Ken");
    }

    #[test]
    fn set_meta_tag_extra_whitespace() {
        // extra spaces between attributes
        let mut md = Markdown::new("<meta   name=\"author\"   content=\"Alice\">\n");
        let updated = set_meta_tag(&mut md, "author", "Ken");
        assert_eq!(updated, 1, "should find tag despite extra whitespace");

        let tags = parse_meta_tags(md.content());
        assert_eq!(tags.get("author").unwrap().as_str(), "Ken");
    }

    #[test]
    fn set_meta_tag_additional_attributes() {
        // extra id attribute before the key attribute
        let mut md =
            Markdown::new("<meta id=\"m1\" name=\"author\" content=\"Alice\" class=\"meta\">\n");
        let updated = set_meta_tag(&mut md, "author", "Ken");
        assert_eq!(updated, 1, "should find tag with additional attributes");

        let tags = parse_meta_tags(md.content());
        assert_eq!(tags.get("author").unwrap().as_str(), "Ken");
    }

    #[test]
    fn set_meta_tag_uppercase_tag() {
        // uppercase META tag
        let mut md = Markdown::new("<META name=\"author\" content=\"Alice\">\n");
        let updated = set_meta_tag(&mut md, "author", "Ken");
        assert_eq!(updated, 1, "should find uppercase META tag");

        let tags = parse_meta_tags(md.content());
        assert_eq!(tags.get("author").unwrap().as_str(), "Ken");
    }

    #[test]
    fn set_meta_tag_og_reordered() {
        // OG property with content before property attribute
        let mut md = Markdown::new("<meta content=\"Old Title\" property=\"og:title\">\n");
        let updated = set_meta_tag(&mut md, "og:title", "New Title");
        assert_eq!(updated, 1, "should find OG tag despite reordered attributes");

        let tags = parse_meta_tags(md.content());
        assert_eq!(tags.get("og:title").unwrap().as_str(), "New Title");
    }

    #[test]
    fn set_meta_tag_charset_extra_whitespace() {
        let mut md = Markdown::new("<meta   charset = \"ascii\">\n");
        let updated = set_meta_tag(&mut md, "charset", "utf-8");
        assert_eq!(updated, 1, "should find charset with extra whitespace");
        assert!(md.content().contains("utf-8"));
        assert!(!md.content().contains("ascii"));
    }
}
