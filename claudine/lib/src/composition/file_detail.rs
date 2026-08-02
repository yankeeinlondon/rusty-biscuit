//! Detail block for autocomplete candidates.
//!
//! Phase 1 of the `2026-06-14-auto-complete` feature. Both the operation-file
//! ENTER autocomplete and the frontmatter `file`/`file[]` property chooser
//! render the same detail block: a badge, the file name, its path, an
//! optional description, and the `$schema` value rendered as YAML lines.
//!
//! The extractor is intentionally tolerant: any read, parse, or type failure
//! falls back to the file stem and the documented "no description" /
//! "no schema defined" placeholders.

use std::path::{Path, PathBuf};

use darkmatter::markdown::Markdown;
use indexmap::IndexMap;
use serde_json::Value;

/// Renderable detail for a single autocomplete candidate file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileDetail {
    /// Styled badge rendered before the file name (e.g. `COMPOSE`,
    /// `INLINE_COMPOSE`, `SEQUENCE`).
    pub badge: String,
    /// Display name for the file.
    pub name: String,
    /// Absolute path to the file.
    pub path: PathBuf,
    /// Schema-provided or frontmatter description when available.
    pub description: Option<String>,
    /// Serialized `$schema` value split into lines, empty when none.
    pub schema_lines: Vec<String>,
    /// Whether the [`name`](Self::name) value came from an explicit
    /// `name` frontmatter key rather than the file-stem fallback.
    pub has_custom_name: bool,
}

/// Extract a [`FileDetail`] from a Markdown file.
///
/// `badge` is supplied by the caller because the same Markdown file may be
/// offered by `compose`, `inline-compose`, or `sequence` depending on the
/// active operation.
///
/// Reads the following frontmatter keys:
///
/// - `name` — overrides the file-stem display name.
/// - `description` — shown below the name when present.
/// - `$schema` — rendered as YAML lines in the schema block.
pub fn extract_markdown_detail(path: &Path, badge: impl Into<String>) -> FileDetail {
    let badge = badge.into();
    let fallback_name = file_stem_fallback(path);

    match Markdown::try_from(path) {
        Ok(markdown) => {
            let map = markdown.frontmatter().as_map();
            let custom_name = string_from_map(map, "name");
            let has_custom_name = custom_name.is_some();
            let name = custom_name.unwrap_or(fallback_name);
            let description = string_from_map(map, "description");
            // Prefer the raw frontmatter source for `$schema` so authored key
            // order survives the serde_json::Value round-trip (which sorts
            // objects unless the `preserve_order` feature is enabled).
            let schema_lines = schema_lines_from_markdown(&markdown);
            FileDetail {
                badge,
                name,
                path: path.to_path_buf(),
                description,
                schema_lines,
                has_custom_name,
            }
        }
        Err(_) => FileDetail {
            badge,
            name: fallback_name,
            path: path.to_path_buf(),
            description: None,
            schema_lines: Vec::new(),
            has_custom_name: false,
        },
    }
}

/// Extract a [`FileDetail`] from a YAML sequence file.
///
/// Per spec §158, a YAML sequence document is treated as "frontmatter
/// without a body". The badge is always `SEQUENCE`. Top-level keys
/// `name`, `description`, and `$schema` are read; missing values use the
/// same fallbacks as the Markdown path.
pub fn extract_yaml_sequence_detail(path: &Path, badge: impl Into<String>) -> FileDetail {
    let badge = badge.into();
    let fallback_name = file_stem_fallback(path);

    let Some(text) = std::fs::read_to_string(path).ok() else {
        return FileDetail {
            badge,
            name: fallback_name,
            path: path.to_path_buf(),
            description: None,
            schema_lines: Vec::new(),
            has_custom_name: false,
        };
    };

    match biscuit_file::serde_yaml_ng::from_str::<biscuit_file::serde_yaml_ng::Value>(&text) {
        Ok(biscuit_file::serde_yaml_ng::Value::Mapping(map)) => {
            let custom_name = string_from_yaml_map(&map, "name");
            let has_custom_name = custom_name.is_some();
            let name = custom_name.unwrap_or(fallback_name);
            let description = string_from_yaml_map(&map, "description");
            let schema_lines = schema_lines_from_yaml_map(&map);
            FileDetail {
                badge,
                name,
                path: path.to_path_buf(),
                description,
                schema_lines,
                has_custom_name,
            }
        }
        _ => FileDetail {
            badge,
            name: fallback_name,
            path: path.to_path_buf(),
            description: None,
            schema_lines: Vec::new(),
            has_custom_name: false,
        },
    }
}

fn file_stem_fallback(path: &Path) -> String {
    path.file_stem()
        .and_then(|s| s.to_str())
        .map(str::to_string)
        .unwrap_or_else(|| biscuit_file::to_portable_string(path))
}

fn string_from_map(map: &IndexMap<String, Value>, key: &str) -> Option<String> {
    map.get(key)
        .and_then(|v| {
            if let Value::String(s) = v {
                Some(s.clone())
            } else {
                serde_json::to_string(v).ok()
            }
        })
        .filter(|s| !s.trim().is_empty())
}

fn schema_lines_from_map(map: &IndexMap<String, Value>) -> Vec<String> {
    map.get("$schema").map(yaml_lines_from_json).unwrap_or_default()
}

/// Extract `$schema` YAML lines from a Markdown document, preserving the
/// authored key order by parsing the raw frontmatter source with
/// `serde_yaml_ng` rather than round-tripping through `serde_json::Value`.
///
/// Falls back to [`schema_lines_from_map`] when the raw source is unavailable
/// or fails to parse.
fn schema_lines_from_markdown(markdown: &Markdown) -> Vec<String> {
    let Some(raw) = markdown.frontmatter().raw_source() else {
        return schema_lines_from_map(markdown.frontmatter().as_map());
    };
    let Ok(yaml) = biscuit_file::serde_yaml_ng::from_str::<biscuit_file::serde_yaml_ng::Value>(raw)
    else {
        return schema_lines_from_map(markdown.frontmatter().as_map());
    };
    let Some(schema) = yaml.get(biscuit_file::serde_yaml_ng::Value::String("$schema".to_string()))
    else {
        return schema_lines_from_map(markdown.frontmatter().as_map());
    };
    yaml_lines(schema)
}

fn string_from_yaml_map(
    map: &biscuit_file::serde_yaml_ng::Mapping,
    key: &str,
) -> Option<String> {
    map.get(biscuit_file::serde_yaml_ng::Value::String(key.to_string()))
        .and_then(|v| {
            if let biscuit_file::serde_yaml_ng::Value::String(s) = v {
                Some(s.clone())
            } else {
                biscuit_file::serde_yaml_ng::to_string(v).ok()
            }
        })
        .filter(|s| !s.trim().is_empty())
}

fn schema_lines_from_yaml_map(
    map: &biscuit_file::serde_yaml_ng::Mapping,
) -> Vec<String> {
    map.get(biscuit_file::serde_yaml_ng::Value::String("$schema".to_string()))
        .map(yaml_lines)
        .unwrap_or_default()
}

fn yaml_lines_from_json(value: &Value) -> Vec<String> {
    // Round-trip through serde_yaml_ng so JSON values render as YAML lines.
    match serde_json::to_string(value)
        .ok()
        .and_then(|json| biscuit_file::serde_yaml_ng::from_str::<biscuit_file::serde_yaml_ng::Value>(&json).ok())
    {
        Some(yaml) => yaml_lines(&yaml),
        None => Vec::new(),
    }
}

fn yaml_lines(value: &biscuit_file::serde_yaml_ng::Value) -> Vec<String> {
    biscuit_file::serde_yaml_ng::to_string(value)
        .map(|s| s.lines().map(str::to_string).collect())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn write(path: &Path, content: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, content).unwrap();
    }

    #[test]
    fn markdown_detail_uses_frontmatter_name_and_description() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("plan.md");
        write(
            &path,
            "---\nname: 'Review Plan'\ndescription: 'A helpful prompt'\n$schema:\n  title: 'string(required)'\n---\nBody\n",
        );

        let detail = extract_markdown_detail(&path, "COMPOSE");
        assert_eq!(detail.name, "Review Plan");
        assert_eq!(detail.description.as_deref(), Some("A helpful prompt"));
        assert!(detail.schema_lines.iter().any(|l| l.contains("title")));
        assert_eq!(detail.badge, "COMPOSE");
    }

    #[test]
    fn markdown_detail_falls_back_to_file_stem() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("my-prompt.md");
        write(&path, "---\n$schema:\n  title: string\n---\nBody\n");

        let detail = extract_markdown_detail(&path, "INLINE_COMPOSE");
        assert_eq!(detail.name, "my-prompt");
        assert!(detail.description.is_none());
        assert_eq!(detail.badge, "INLINE_COMPOSE");
    }

    #[test]
    fn markdown_detail_no_schema_defined() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("plain.md");
        write(&path, "# Hello\n\nBody\n");

        let detail = extract_markdown_detail(&path, "COMPOSE");
        assert_eq!(detail.name, "plain");
        assert!(detail.description.is_none());
        assert!(detail.schema_lines.is_empty());
    }

    #[test]
    fn markdown_detail_survives_parse_failure() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("bad.md");
        fs::write(&path, b"\xff\xfe\x00\x00").unwrap();

        let detail = extract_markdown_detail(&path, "COMPOSE");
        assert_eq!(detail.name, "bad");
        assert!(detail.description.is_none());
        assert!(detail.schema_lines.is_empty());
    }

    #[test]
    fn yaml_sequence_detail_reads_top_level_keys() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("steps.yaml");
        write(
            &path,
            "name: 'Build Pipeline'\ndescription: 'CI steps'\n$schema:\n  env: 'enum(dev, prod)'\nsequence:\n  - one\n  - two\n",
        );

        let detail = extract_yaml_sequence_detail(&path, "SEQUENCE");
        assert_eq!(detail.name, "Build Pipeline");
        assert_eq!(detail.description.as_deref(), Some("CI steps"));
        assert!(detail.schema_lines.iter().any(|l| l.contains("env")));
        assert_eq!(detail.badge, "SEQUENCE");
    }

    #[test]
    fn yaml_sequence_detail_falls_back_when_missing_keys() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("empty.yaml");
        write(&path, "sequence:\n  - one\n");

        let detail = extract_yaml_sequence_detail(&path, "SEQUENCE");
        assert_eq!(detail.name, "empty");
        assert!(detail.description.is_none());
        assert!(detail.schema_lines.is_empty());
    }

    #[test]
    fn yaml_sequence_detail_rejects_non_mapping_root() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("list.yaml");
        write(&path, "- one\n- two\n");

        let detail = extract_yaml_sequence_detail(&path, "SEQUENCE");
        assert_eq!(detail.name, "list");
        assert!(detail.description.is_none());
        assert!(detail.schema_lines.is_empty());
    }

    #[test]
    fn markdown_detail_preserves_schema_property_order() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("plan.md");
        write(
            &path,
            r#"---
name: 'Plan'
description: 'Creates a multi-phase, high confidence plan from a _feature_ or _plan_'
$schema:
  spec: 'file(required;match(**/*spec*.md);eager)'
  design: 'file(match(**/*design*.md))'
  plan: 'file'
---
# Plan
"#,
        );

        let detail = extract_markdown_detail(&path, "COMPOSE");
        assert_eq!(detail.name, "Plan");
        assert!(detail.description.as_deref().unwrap().contains("_feature_"));
        assert_eq!(
            detail.schema_lines,
            vec![
                "spec: file(required;match(**/*spec*.md);eager)",
                "design: file(match(**/*design*.md))",
                "plan: file",
            ]
        );
    }
}
