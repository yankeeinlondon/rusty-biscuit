//! Source resolution for choice options.
//!
//! This module turns the various CLI source flags into a flat
//! `Vec<RawOption>` of typed records, ready for normalization. Each
//! `RawOption` preserves the structured fields that object-shaped
//! sources (JSON/YAML/TOML/CSV/markdown frontmatter object arrays) can
//! supply, so downstream normalization sees the original `label`,
//! `value`, `hotkey`, and `disabled` fields without lossy
//! re-encoding through `format!("{}::{}", ...)`.

use std::fs;
use std::io::{self, IsTerminal, Read};
use std::path::Path;

use serde_json::Value as JsonValue;

/// Errors that can occur while resolving options from a source.
#[derive(Debug, thiserror::Error)]
pub enum SourceError {
    #[error(
        "no options provided: pass options as positional args, via stdin, or use a source flag"
    )]
    NoSource,
    #[error("multiple option sources provided: sources are mutually exclusive")]
    MultipleSources,
    #[error("failed to read file: {0}")]
    ReadFile(#[from] io::Error),
    #[error("failed to parse file: {0}")]
    Parse(String),
    #[error("file content must be an array of options")]
    NotAnArray,
    #[error("markdown frontmatter property '{prop}' not found or not an array")]
    MdPropNotArray { prop: String },
    #[error(
        "unsupported file format '{ext}': supported extensions are json, jsonl, ndjson, yaml, yml, toml, csv"
    )]
    UnsupportedFormat { ext: String },
}

/// Typed raw option record produced by source resolution.
///
/// String-shaped sources (positional args, `--csv`, `--list`, plain
/// string array entries) populate only `label`. Object-shaped sources
/// (JSON/YAML/TOML object arrays, multi-column CSV rows, markdown
/// frontmatter object arrays) preserve `value`, `hotkey`, and the
/// optional `disabled` flag end-to-end.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawOption {
    /// Display text for the option.
    pub label: String,
    /// Optional explicit value. When `None`, downstream normalization
    /// derives the value from the label (after applying `::` splitting
    /// and value conventions).
    pub value: Option<String>,
    /// Optional explicit hotkey, encoded as a string such as
    /// `"CTRL+R"`, `"ALT+B"`, or `"OPT+B"`. When `None`, the prefix
    /// parser in `choice_normalize` may still extract a hotkey from a
    /// `[CTRL+X]` prefix on the label.
    pub hotkey: Option<String>,
    /// Optional disabled flag. When `Some(true)`, the option is
    /// rendered as disabled.
    pub disabled: Option<bool>,
}

impl RawOption {
    /// Constructs a `RawOption` carrying only a label (the common case
    /// for string-shaped sources).
    pub fn from_label(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            value: None,
            hotkey: None,
            disabled: None,
        }
    }
}

impl From<String> for RawOption {
    fn from(label: String) -> Self {
        Self::from_label(label)
    }
}

impl From<&str> for RawOption {
    fn from(label: &str) -> Self {
        Self::from_label(label.to_string())
    }
}

#[allow(clippy::too_many_arguments)]
pub fn resolve_raw_options(
    csv: Option<&str>,
    list: Option<&str>,
    rows: Option<&str>,
    file: Option<&Path>,
    md: Option<(&Path, &str)>,
    options_from_file: Option<&Path>,
    options_from_dictionary: Option<&Path>,
    positional: Vec<String>,
) -> Result<Vec<RawOption>, SourceError> {
    let mut source_count = 0;
    if csv.is_some() {
        source_count += 1;
    }
    if list.is_some() {
        source_count += 1;
    }
    if rows.is_some() {
        source_count += 1;
    }
    if file.is_some() {
        source_count += 1;
    }
    if md.is_some() {
        source_count += 1;
    }
    if options_from_file.is_some() {
        source_count += 1;
    }
    if options_from_dictionary.is_some() {
        source_count += 1;
    }
    if !positional.is_empty() {
        source_count += 1;
    }

    if source_count > 1 {
        return Err(SourceError::MultipleSources);
    }

    if let Some(csv) = csv {
        return Ok(parse_csv(csv));
    }
    if let Some(list) = list {
        return Ok(parse_list(list));
    }
    if let Some(rows) = rows {
        return Ok(parse_rows(rows));
    }
    if let Some(file) = file {
        return parse_file(file);
    }
    if let Some((path, prop)) = md {
        return parse_md(path, prop);
    }
    if let Some(path) = options_from_file {
        let body = fs::read_to_string(path)?;
        return Ok(parse_markdown_list(&body));
    }
    if let Some(path) = options_from_dictionary {
        let body = fs::read_to_string(path)?;
        return parse_dictionary(&body);
    }
    if !positional.is_empty() {
        return Ok(positional.into_iter().map(RawOption::from).collect());
    }

    if io::stdin().is_terminal() {
        return Err(SourceError::NoSource);
    }
    let mut buf = String::new();
    io::stdin().read_to_string(&mut buf)?;
    let lines = parse_list(&buf);
    if lines.is_empty() {
        return Err(SourceError::NoSource);
    }
    Ok(lines)
}

fn parse_csv(csv: &str) -> Vec<RawOption> {
    csv.split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(RawOption::from)
        .collect()
}

fn parse_list(text: &str) -> Vec<RawOption> {
    parse_line_labels(text)
        .into_iter()
        .map(|line| strip_simple_markdown_list_prefix(line).unwrap_or(line))
        .map(RawOption::from)
        .collect()
}

fn parse_rows(text: &str) -> Vec<RawOption> {
    parse_line_labels(text)
        .into_iter()
        .map(RawOption::from)
        .collect()
}

fn parse_line_labels(text: &str) -> Vec<&str> {
    text.lines()
        .map(|line| line.trim_end_matches('\r'))
        .filter(|line| !line.is_empty())
        .collect()
}

fn strip_simple_markdown_list_prefix(line: &str) -> Option<&str> {
    let trimmed = line.trim_start();
    strip_bullet_prefix(trimmed).or_else(|| strip_numbered_prefix(trimmed))
}

fn parse_file(path: &Path) -> Result<Vec<RawOption>, SourceError> {
    let body = fs::read_to_string(path)?;
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        "json" => parse_json(&body),
        "jsonl" | "ndjson" => parse_jsonl(&body),
        "yaml" | "yml" => parse_yaml(&body),
        "toml" => parse_toml(&body),
        "csv" => parse_csv_file(&body),
        other => Err(SourceError::UnsupportedFormat {
            ext: if other.is_empty() {
                "(none)".to_string()
            } else {
                other.to_string()
            },
        }),
    }
}

fn parse_json(body: &str) -> Result<Vec<RawOption>, SourceError> {
    let value: JsonValue =
        serde_json::from_str(body).map_err(|e| SourceError::Parse(e.to_string()))?;
    extract_string_array(&value)
}

fn parse_jsonl(body: &str) -> Result<Vec<RawOption>, SourceError> {
    let mut results = Vec::new();
    for line in body.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let value: JsonValue =
            serde_json::from_str(line).map_err(|e| SourceError::Parse(e.to_string()))?;
        match value {
            JsonValue::String(s) => results.push(RawOption::from(s)),
            JsonValue::Object(map) => {
                results.push(json_object_to_raw_option(&map)?);
            }
            _ => {
                return Err(SourceError::Parse(
                    "JSONL lines must be strings or objects".into(),
                ));
            }
        }
    }
    Ok(results)
}

fn parse_yaml(body: &str) -> Result<Vec<RawOption>, SourceError> {
    let value: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(body).map_err(|e| SourceError::Parse(e.to_string()))?;
    extract_yaml_string_array(&value)
}

fn parse_toml(body: &str) -> Result<Vec<RawOption>, SourceError> {
    let value: toml::Value = toml::from_str(body).map_err(|e| SourceError::Parse(e.to_string()))?;
    match &value {
        toml::Value::Table(table) => {
            let options = table.get("options").ok_or(SourceError::NotAnArray)?;
            extract_toml_options_array(options)
        }
        toml::Value::Array(_) => extract_toml_options_array(&value),
        _ => Err(SourceError::NotAnArray),
    }
}

fn parse_csv_file(body: &str) -> Result<Vec<RawOption>, SourceError> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .from_reader(body.as_bytes());
    let mut results = Vec::new();
    for record in reader.records() {
        let record = record.map_err(|e| SourceError::Parse(e.to_string()))?;
        match record.len() {
            0 => {}
            1 => results.push(RawOption::from_label(record[0].trim())),
            2 => results.push(RawOption {
                label: record[0].trim().to_string(),
                value: Some(record[1].trim().to_string()),
                hotkey: None,
                disabled: None,
            }),
            _ => {
                let hotkey = record[2].trim();
                results.push(RawOption {
                    label: record[0].trim().to_string(),
                    value: Some(record[1].trim().to_string()),
                    hotkey: if hotkey.is_empty() {
                        None
                    } else {
                        Some(hotkey.to_string())
                    },
                    disabled: None,
                });
            }
        }
    }
    Ok(results)
}

fn extract_string_array(value: &JsonValue) -> Result<Vec<RawOption>, SourceError> {
    let arr = value.as_array().ok_or(SourceError::NotAnArray)?;
    let mut results = Vec::new();
    for item in arr {
        match item {
            JsonValue::String(s) => results.push(RawOption::from(s.clone())),
            JsonValue::Object(map) => {
                results.push(json_object_to_raw_option(map)?);
            }
            other => results.push(RawOption::from(other.to_string())),
        }
    }
    Ok(results)
}

fn json_object_to_raw_option(
    map: &serde_json::Map<String, JsonValue>,
) -> Result<RawOption, SourceError> {
    let label = match (map.get("label"), map.get("value")) {
        (Some(JsonValue::String(label)), _) => label.clone(),
        (None, Some(JsonValue::String(value))) => value.clone(),
        _ => {
            return Err(SourceError::Parse(
                "JSON object must have a 'label' or 'value' field".into(),
            ));
        }
    };
    let value = match map.get("value") {
        Some(JsonValue::String(s)) => Some(s.clone()),
        Some(JsonValue::Null) | None => None,
        Some(other) => Some(other.to_string()),
    };
    let hotkey = match map.get("hotkey") {
        Some(JsonValue::String(s)) if !s.is_empty() => Some(s.clone()),
        _ => None,
    };
    let disabled = match map.get("disabled") {
        Some(JsonValue::Bool(b)) => Some(*b),
        _ => None,
    };
    Ok(RawOption {
        label,
        value,
        hotkey,
        disabled,
    })
}

fn extract_yaml_string_array(value: &serde_yaml_ng::Value) -> Result<Vec<RawOption>, SourceError> {
    let arr = value.as_sequence().ok_or(SourceError::NotAnArray)?;
    let mut results = Vec::new();
    for item in arr {
        match item {
            serde_yaml_ng::Value::String(s) => results.push(RawOption::from(s.clone())),
            serde_yaml_ng::Value::Mapping(map) => {
                results.push(yaml_mapping_to_raw_option(map)?);
            }
            other => {
                let s = serde_yaml_ng::to_string(other).unwrap_or_default();
                results.push(RawOption::from(s.trim().to_string()));
            }
        }
    }
    Ok(results)
}

fn yaml_mapping_to_raw_option(map: &serde_yaml_ng::Mapping) -> Result<RawOption, SourceError> {
    let label_key = serde_yaml_ng::Value::String("label".into());
    let value_key = serde_yaml_ng::Value::String("value".into());
    let hotkey_key = serde_yaml_ng::Value::String("hotkey".into());
    let disabled_key = serde_yaml_ng::Value::String("disabled".into());

    let label = match (map.get(&label_key), map.get(&value_key)) {
        (Some(serde_yaml_ng::Value::String(label)), _) => label.clone(),
        (None, Some(serde_yaml_ng::Value::String(value))) => value.clone(),
        _ => {
            return Err(SourceError::Parse(
                "YAML object must have a 'label' or 'value' field".into(),
            ));
        }
    };
    let value = match map.get(&value_key) {
        Some(serde_yaml_ng::Value::String(s)) => Some(s.clone()),
        Some(serde_yaml_ng::Value::Null) | None => None,
        Some(other) => Some(yaml_value_to_string(other)),
    };
    let hotkey = match map.get(&hotkey_key) {
        Some(serde_yaml_ng::Value::String(s)) if !s.is_empty() => Some(s.clone()),
        _ => None,
    };
    let disabled = match map.get(&disabled_key) {
        Some(serde_yaml_ng::Value::Bool(b)) => Some(*b),
        _ => None,
    };
    Ok(RawOption {
        label,
        value,
        hotkey,
        disabled,
    })
}

fn extract_toml_options_array(value: &toml::Value) -> Result<Vec<RawOption>, SourceError> {
    let arr = value.as_array().ok_or(SourceError::NotAnArray)?;
    let mut results = Vec::new();
    for item in arr {
        match item {
            toml::Value::String(s) => results.push(RawOption::from(s.clone())),
            toml::Value::Table(map) => {
                results.push(toml_table_to_raw_option(map)?);
            }
            other => results.push(RawOption::from(other.to_string())),
        }
    }
    Ok(results)
}

fn toml_table_to_raw_option(
    map: &toml::map::Map<String, toml::Value>,
) -> Result<RawOption, SourceError> {
    let label = match (map.get("label"), map.get("value")) {
        (Some(toml::Value::String(label)), _) => label.clone(),
        (None, Some(toml::Value::String(value))) => value.clone(),
        _ => {
            return Err(SourceError::Parse(
                "TOML object must have a 'label' or 'value' field".into(),
            ));
        }
    };
    let value = match map.get("value") {
        Some(toml::Value::String(s)) => Some(s.clone()),
        None => None,
        Some(other) => Some(other.to_string()),
    };
    let hotkey = match map.get("hotkey") {
        Some(toml::Value::String(s)) if !s.is_empty() => Some(s.clone()),
        _ => None,
    };
    let disabled = match map.get("disabled") {
        Some(toml::Value::Boolean(b)) => Some(*b),
        _ => None,
    };
    Ok(RawOption {
        label,
        value,
        hotkey,
        disabled,
    })
}

fn parse_markdown_list(body: &str) -> Vec<RawOption> {
    body.lines()
        .filter_map(|line| {
            let trimmed = line.trim_start();
            if let Some(rest) = strip_bullet_prefix(trimmed) {
                let value = rest.trim();
                if value.is_empty() {
                    return None;
                }
                return Some(RawOption::from(value.to_string()));
            }
            if let Some(rest) = strip_numbered_prefix(trimmed) {
                let value = rest.trim();
                if value.is_empty() {
                    return None;
                }
                return Some(RawOption::from(value.to_string()));
            }
            None
        })
        .collect()
}

fn strip_bullet_prefix(line: &str) -> Option<&str> {
    let mut chars = line.chars();
    let marker = chars.next()?;
    if !matches!(marker, '-' | '*' | '+') {
        return None;
    }
    let rest = chars.as_str();
    rest.chars()
        .next()
        .is_some_and(char::is_whitespace)
        .then(|| rest.trim_start())
}

fn strip_numbered_prefix(line: &str) -> Option<&str> {
    let digit_count = line.chars().take_while(char::is_ascii_digit).count();
    if digit_count == 0 {
        return None;
    }
    let (digits, rest) = line.split_at(digit_count);
    let _: u32 = digits.parse().ok()?;
    let rest = rest.strip_prefix('.').or_else(|| rest.strip_prefix(')'))?;
    rest.chars()
        .next()
        .is_some_and(char::is_whitespace)
        .then(|| rest.trim_start())
}

fn parse_dictionary(body: &str) -> Result<Vec<RawOption>, SourceError> {
    let value: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(body).map_err(|e| SourceError::Parse(e.to_string()))?;
    let mapping = match value {
        serde_yaml_ng::Value::Mapping(m) => m,
        _ => {
            return Err(SourceError::Parse(
                "dictionary input must be a mapping/object".into(),
            ));
        }
    };
    Ok(mapping
        .into_iter()
        .map(|(key, value)| {
            let label = yaml_value_to_string(&key);
            let val = yaml_value_to_string(&value);
            if label == val {
                RawOption::from_label(label)
            } else {
                RawOption {
                    label,
                    value: Some(val),
                    hotkey: None,
                    disabled: None,
                }
            }
        })
        .collect())
}

fn yaml_value_to_string(value: &serde_yaml_ng::Value) -> String {
    match value {
        serde_yaml_ng::Value::Null => String::new(),
        serde_yaml_ng::Value::Bool(b) => b.to_string(),
        serde_yaml_ng::Value::Number(n) => n.to_string(),
        serde_yaml_ng::Value::String(s) => s.clone(),
        other => serde_yaml_ng::to_string(other)
            .unwrap_or_default()
            .trim()
            .to_string(),
    }
}

fn parse_md(path: &Path, prop: &str) -> Result<Vec<RawOption>, SourceError> {
    let body = fs::read_to_string(path)?;
    // Strip an optional UTF-8 BOM, then normalize CRLF to LF so the
    // literal `\n---` close-fence search is robust against editors
    // that emit Windows line endings or a leading byte-order mark.
    let body = body.strip_prefix('\u{feff}').unwrap_or(&body);
    let body = body.replace("\r\n", "\n");
    // Extract frontmatter between --- delimiters
    let trimmed = body.trim_start();
    let after_first = trimmed.strip_prefix("---").ok_or_else(|| {
        SourceError::Parse("markdown file must have frontmatter delimited by ---".into())
    })?;
    let Some(end_idx) = after_first.find("\n---") else {
        return Err(SourceError::Parse(
            "markdown frontmatter not properly closed".into(),
        ));
    };
    let frontmatter = &after_first[..end_idx];
    let value: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(frontmatter).map_err(|e| SourceError::Parse(e.to_string()))?;
    let mapping = match value {
        serde_yaml_ng::Value::Mapping(m) => m,
        _ => {
            return Err(SourceError::Parse(
                "frontmatter must be a YAML mapping".into(),
            ));
        }
    };
    let prop_value = mapping
        .get(serde_yaml_ng::Value::String(prop.into()))
        .ok_or_else(|| SourceError::MdPropNotArray {
            prop: prop.to_string(),
        })?;
    extract_yaml_string_array(prop_value).map_err(|_| SourceError::MdPropNotArray {
        prop: prop.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn labels(items: &[RawOption]) -> Vec<&str> {
        items.iter().map(|o| o.label.as_str()).collect()
    }

    #[test]
    fn parse_csv_splits_and_trims() {
        let result = parse_csv("Red, Green , Blue");
        assert_eq!(labels(&result), vec!["Red", "Green", "Blue"]);
        assert!(result.iter().all(|o| o.value.is_none()));
    }

    #[test]
    fn parse_csv_drops_empty() {
        let result = parse_csv("a, ,b,");
        assert_eq!(labels(&result), vec!["a", "b"]);
    }

    #[test]
    fn parse_list_splits_lines() {
        let result = parse_list("alpha\nbeta\n\ngamma\n");
        assert_eq!(labels(&result), vec!["alpha", "beta", "gamma"]);
    }

    #[test]
    fn parse_list_strips_markdown_bullets_after_indentation() {
        let result = parse_list("  - Apple\n\t* Banana\n+ Cherry\n");
        assert_eq!(labels(&result), vec!["Apple", "Banana", "Cherry"]);
    }

    #[test]
    fn parse_list_strips_markdown_ordered_markers_after_indentation() {
        let result = parse_list("  1. Apple\n2) Banana\n10.\tCherry\n");
        assert_eq!(labels(&result), vec!["Apple", "Banana", "Cherry"]);
    }

    #[test]
    fn parse_list_preserves_plain_lines_and_non_marker_hyphens() {
        let result = parse_list("plain\n--flag\nnorth-east\n-Not a bullet\n1.Not ordered\n");
        assert_eq!(
            labels(&result),
            vec![
                "plain",
                "--flag",
                "north-east",
                "-Not a bullet",
                "1.Not ordered"
            ]
        );
    }

    #[test]
    fn parse_rows_preserves_markdown_like_prefixes_raw() {
        let result = parse_rows("- Red::apple\n1) Blue::sky\n");
        assert_eq!(labels(&result), vec!["- Red::apple", "1) Blue::sky"]);
    }

    #[test]
    fn parse_file_json_array() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_options.json");
        std::fs::write(&path, r#"["Red", "Green", "Blue"]"#).unwrap();
        let result = parse_file(&path).unwrap();
        assert_eq!(labels(&result), vec!["Red", "Green", "Blue"]);
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn parse_file_yaml_array() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_options.yaml");
        std::fs::write(&path, "- Red\n- Green\n- Blue\n").unwrap();
        let result = parse_file(&path).unwrap();
        assert_eq!(labels(&result), vec!["Red", "Green", "Blue"]);
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn parse_file_toml_options_string_array() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_options.toml");
        std::fs::write(&path, "options = [\"Red\", \"Green\", \"Blue\"]\n").unwrap();
        let result = parse_file(&path).unwrap();
        assert_eq!(labels(&result), vec!["Red", "Green", "Blue"]);
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn parse_file_toml_options_inline_table_array_preserves_fields() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_options_inline.toml");
        std::fs::write(
            &path,
            "options = [{ label = \"Red\", value = \"apple\", hotkey = \"CTRL+R\", disabled = true }, { label = \"Blue\", value = \"sky\", hotkey = \"ALT+B\" }]\n",
        )
        .unwrap();
        let result = parse_file(&path).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].label, "Red");
        assert_eq!(result[0].value.as_deref(), Some("apple"));
        assert_eq!(result[0].hotkey.as_deref(), Some("CTRL+R"));
        assert_eq!(result[0].disabled, Some(true));
        assert_eq!(result[1].label, "Blue");
        assert_eq!(result[1].value.as_deref(), Some("sky"));
        assert_eq!(result[1].hotkey.as_deref(), Some("ALT+B"));
        assert_eq!(result[1].disabled, None);
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn parse_file_toml_array_of_tables_preserves_fields() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_options_array_of_tables.toml");
        std::fs::write(
            &path,
            "[[options]]\nlabel = \"Red\"\nvalue = \"apple\"\nhotkey = \"CTRL+R\"\ndisabled = true\n\n[[options]]\nlabel = \"Blue\"\nvalue = \"sky\"\nhotkey = \"ALT+B\"\n",
        )
        .unwrap();
        let result = parse_file(&path).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].label, "Red");
        assert_eq!(result[0].value.as_deref(), Some("apple"));
        assert_eq!(result[0].hotkey.as_deref(), Some("CTRL+R"));
        assert_eq!(result[0].disabled, Some(true));
        assert_eq!(result[1].label, "Blue");
        assert_eq!(result[1].value.as_deref(), Some("sky"));
        assert_eq!(result[1].hotkey.as_deref(), Some("ALT+B"));
        assert_eq!(result[1].disabled, None);
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn parse_file_toml_missing_options_key_is_not_array() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_options_missing.toml");
        std::fs::write(&path, "items = [\"Red\", \"Green\"]\n").unwrap();
        let result = parse_file(&path);
        assert!(matches!(result, Err(SourceError::NotAnArray)));
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn parse_file_toml_options_non_array_is_not_array() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_options_non_array.toml");
        std::fs::write(&path, "options = \"Red\"\n").unwrap();
        let result = parse_file(&path);
        assert!(matches!(result, Err(SourceError::NotAnArray)));
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn parse_file_rejects_txt_plain_list() {
        let dir = std::env::temp_dir();
        let path = dir.join("parse_file_rejects_options.txt");
        std::fs::write(&path, "Red\nGreen\nBlue\n").unwrap();
        let result = parse_file(&path);
        match result {
            Err(SourceError::UnsupportedFormat { ext }) => assert_eq!(ext, "txt"),
            other => panic!("expected UnsupportedFormat with ext=txt, got {other:?}"),
        }
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn parse_file_rejects_unknown_extension() {
        let dir = std::env::temp_dir();
        let path = dir.join("parse_file_rejects_options.dat");
        // Body is valid JSON, but the extension is authoritative.
        std::fs::write(&path, r#"["Red","Green"]"#).unwrap();
        let result = parse_file(&path);
        match result {
            Err(SourceError::UnsupportedFormat { ext }) => assert_eq!(ext, "dat"),
            other => panic!("expected UnsupportedFormat with ext=dat, got {other:?}"),
        }
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn parse_file_rejects_no_extension() {
        let dir = std::env::temp_dir();
        let path = dir.join("parse_file_rejects_options_no_ext");
        std::fs::write(&path, "anything\n").unwrap();
        let result = parse_file(&path);
        match result {
            Err(SourceError::UnsupportedFormat { ext }) => assert_eq!(ext, "(none)"),
            other => panic!("expected UnsupportedFormat with ext=(none), got {other:?}"),
        }
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn parse_file_rejects_md_extension() {
        // `.md` is intentionally routed through `--md <file> <prop>`,
        // not `--file`. `--file foo.md` must error.
        let dir = std::env::temp_dir();
        let path = dir.join("parse_file_rejects_options.md");
        std::fs::write(&path, "- Red\n- Green\n").unwrap();
        let result = parse_file(&path);
        match result {
            Err(SourceError::UnsupportedFormat { ext }) => assert_eq!(ext, "md"),
            other => panic!("expected UnsupportedFormat with ext=md, got {other:?}"),
        }
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn parse_file_csv_extension_still_works() {
        let dir = std::env::temp_dir();
        let path = dir.join("parse_file_supported_options.csv");
        std::fs::write(&path, "Red,apple\nBlue,sky\n").unwrap();
        let result = parse_file(&path).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].label, "Red");
        assert_eq!(result[0].value.as_deref(), Some("apple"));
        assert_eq!(result[1].label, "Blue");
        assert_eq!(result[1].value.as_deref(), Some("sky"));
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn parse_file_jsonl_extension_still_works() {
        let dir = std::env::temp_dir();
        let path = dir.join("parse_file_supported_options.jsonl");
        std::fs::write(&path, "\"Red\"\n\"Blue\"\n").unwrap();
        let result = parse_file(&path).unwrap();
        assert_eq!(labels(&result), vec!["Red", "Blue"]);
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn parse_file_ndjson_extension_still_works() {
        let dir = std::env::temp_dir();
        let path = dir.join("parse_file_supported_options.ndjson");
        std::fs::write(&path, "\"Red\"\n\"Blue\"\n").unwrap();
        let result = parse_file(&path).unwrap();
        assert_eq!(labels(&result), vec!["Red", "Blue"]);
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn resolve_raw_options_csv_source() {
        let result =
            resolve_raw_options(Some("a,b,c"), None, None, None, None, None, None, vec![]).unwrap();
        assert_eq!(labels(&result), vec!["a", "b", "c"]);
    }

    #[test]
    fn resolve_raw_options_multiple_sources_error() {
        let result = resolve_raw_options(
            Some("a,b"),
            Some("c\nd"),
            None,
            None,
            None,
            None,
            None,
            vec![],
        );
        assert!(matches!(result, Err(SourceError::MultipleSources)));
    }

    #[test]
    fn parse_md_frontmatter_string_array() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_options_string.md");
        std::fs::write(&path, "---\nitems:\n  - Red\n  - Green\n---\n# Hello\n").unwrap();
        let result = parse_md(&path, "items").unwrap();
        assert_eq!(labels(&result), vec!["Red", "Green"]);
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn parse_md_tolerates_bom_and_crlf_line_endings() {
        use std::io::Write;
        let dir = std::env::temp_dir();
        let path = dir.join("question_review10_md_bom_crlf.md");
        // Build a frontmatter document with:
        //   * UTF-8 BOM at the very start
        //   * CRLF line endings throughout
        //   * Frontmatter property `colors` as a YAML array
        //   * A leading blank line before the opening `---`
        let body = b"\xef\xbb\xbf\r\n---\r\ncolors:\r\n  - Red\r\n  - Green\r\n  - Blue\r\n---\r\n# Body content\r\n";
        std::fs::File::create(&path)
            .unwrap()
            .write_all(body)
            .unwrap();

        let result = parse_md(&path, "colors").expect("parse_md should tolerate BOM + CRLF");
        assert_eq!(labels(&result), vec!["Red", "Green", "Blue"]);

        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn parse_md_tolerates_utf8_bom_with_lf_line_endings() {
        use std::io::Write;
        let dir = std::env::temp_dir();
        let path = dir.join("question_review10_md_bom_only.md");
        let body = b"\xef\xbb\xbf---\nopts:\n  - a\n  - b\n---\n";
        std::fs::File::create(&path)
            .unwrap()
            .write_all(body)
            .unwrap();
        let result = parse_md(&path, "opts").expect("parse_md should tolerate BOM");
        assert_eq!(labels(&result), vec!["a", "b"]);
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn parse_md_tolerates_crlf_line_endings_without_bom() {
        use std::io::Write;
        let dir = std::env::temp_dir();
        let path = dir.join("question_review10_md_crlf_only.md");
        let body = b"---\r\nopts:\r\n  - a\r\n  - b\r\n---\r\n";
        std::fs::File::create(&path)
            .unwrap()
            .write_all(body)
            .unwrap();
        let result = parse_md(&path, "opts").expect("parse_md should tolerate CRLF");
        assert_eq!(labels(&result), vec!["a", "b"]);
        std::fs::remove_file(&path).unwrap();
    }

    // --- Phase 3: object-record preservation tests ----------------

    #[test]
    fn json_array_of_objects_preserves_label_value_hotkey() {
        let body = r#"[{"label":"Red","value":"apple","hotkey":"CTRL+R"}]"#;
        let result = parse_json(body).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].label, "Red");
        assert_eq!(result[0].value.as_deref(), Some("apple"));
        assert_eq!(result[0].hotkey.as_deref(), Some("CTRL+R"));
    }

    #[test]
    fn json_array_of_objects_preserves_disabled_flag() {
        let body = r#"[{"label":"Red","value":"r","disabled":true}]"#;
        let result = parse_json(body).unwrap();
        assert_eq!(result[0].disabled, Some(true));
    }

    #[test]
    fn json_string_array_yields_label_only_options() {
        let body = r#"["Red","Green"]"#;
        let result = parse_json(body).unwrap();
        assert_eq!(labels(&result), vec!["Red", "Green"]);
        assert!(result.iter().all(|o| o.value.is_none()));
        assert!(result.iter().all(|o| o.hotkey.is_none()));
    }

    #[test]
    fn json_object_with_only_value_uses_value_as_label() {
        let body = r#"[{"value":"apple"}]"#;
        let result = parse_json(body).unwrap();
        assert_eq!(result[0].label, "apple");
        assert_eq!(result[0].value.as_deref(), Some("apple"));
    }

    #[test]
    fn jsonl_object_preserves_value_and_hotkey() {
        let body = "{\"label\":\"Red\",\"value\":\"apple\",\"hotkey\":\"CTRL+R\"}\n{\"label\":\"Blue\",\"value\":\"sky\",\"hotkey\":\"ALT+B\"}\n";
        let result = parse_jsonl(body).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].label, "Red");
        assert_eq!(result[0].value.as_deref(), Some("apple"));
        assert_eq!(result[0].hotkey.as_deref(), Some("CTRL+R"));
        assert_eq!(result[1].label, "Blue");
        assert_eq!(result[1].value.as_deref(), Some("sky"));
        assert_eq!(result[1].hotkey.as_deref(), Some("ALT+B"));
    }

    #[test]
    fn ndjson_object_preserves_value_and_hotkey() {
        // NDJSON is the same wire format as JSONL for our purposes.
        let body = "{\"label\":\"Red\",\"value\":\"apple\",\"hotkey\":\"CTRL+R\"}\n";
        let result = parse_jsonl(body).unwrap();
        assert_eq!(result[0].value.as_deref(), Some("apple"));
        assert_eq!(result[0].hotkey.as_deref(), Some("CTRL+R"));
    }

    #[test]
    fn yaml_array_of_mappings_preserves_value_and_hotkey() {
        let body = "- label: Red\n  value: apple\n  hotkey: CTRL+R\n- label: Blue\n  value: sky\n  hotkey: ALT+B\n";
        let result = parse_yaml(body).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].label, "Red");
        assert_eq!(result[0].value.as_deref(), Some("apple"));
        assert_eq!(result[0].hotkey.as_deref(), Some("CTRL+R"));
        assert_eq!(result[1].label, "Blue");
        assert_eq!(result[1].value.as_deref(), Some("sky"));
        assert_eq!(result[1].hotkey.as_deref(), Some("ALT+B"));
    }

    #[test]
    fn yaml_array_of_mappings_preserves_disabled() {
        let body = "- label: Red\n  value: r\n  disabled: true\n";
        let result = parse_yaml(body).unwrap();
        assert_eq!(result[0].disabled, Some(true));
    }

    #[test]
    fn toml_table_preserves_value_and_hotkey() {
        let body2 = "options = [{ label = \"Red\", value = \"apple\", hotkey = \"CTRL+R\" }, { label = \"Blue\", value = \"sky\", hotkey = \"ALT+B\" }]\n";
        let result = parse_toml(body2).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].label, "Red");
        assert_eq!(result[0].value.as_deref(), Some("apple"));
        assert_eq!(result[0].hotkey.as_deref(), Some("CTRL+R"));
        assert_eq!(result[1].label, "Blue");
        assert_eq!(result[1].value.as_deref(), Some("sky"));
        assert_eq!(result[1].hotkey.as_deref(), Some("ALT+B"));
    }

    #[test]
    fn csv_three_columns_preserves_value_and_hotkey() {
        let body = "Red,apple,CTRL+R\nBlue,sky,ALT+B\n";
        let result = parse_csv_file(body).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].label, "Red");
        assert_eq!(result[0].value.as_deref(), Some("apple"));
        assert_eq!(result[0].hotkey.as_deref(), Some("CTRL+R"));
        assert_eq!(result[1].label, "Blue");
        assert_eq!(result[1].value.as_deref(), Some("sky"));
        assert_eq!(result[1].hotkey.as_deref(), Some("ALT+B"));
    }

    #[test]
    fn csv_two_columns_preserves_value_no_hotkey() {
        let body = "Red,apple\nBlue,sky\n";
        let result = parse_csv_file(body).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].label, "Red");
        assert_eq!(result[0].value.as_deref(), Some("apple"));
        assert!(result[0].hotkey.is_none());
        assert_eq!(result[1].label, "Blue");
        assert_eq!(result[1].value.as_deref(), Some("sky"));
        assert!(result[1].hotkey.is_none());
    }

    #[test]
    fn csv_one_column_yields_label_only() {
        let body = "Red\nBlue\n";
        let result = parse_csv_file(body).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].label, "Red");
        assert!(result[0].value.is_none());
    }

    #[test]
    fn non_array_top_level_returns_invalid_source_shape() {
        // JSON top-level object (not an array) must fail with NotAnArray.
        let body = r#"{"label":"Red"}"#;
        let result = parse_json(body);
        assert!(matches!(result, Err(SourceError::NotAnArray)));
    }

    #[test]
    fn markdown_frontmatter_object_array_preserves_value_and_hotkey() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_options_object_array.md");
        let body = "---\nitems:\n  - label: Red\n    value: apple\n    hotkey: CTRL+R\n  - label: Blue\n    value: sky\n    hotkey: ALT+B\n---\n# Hello\n";
        std::fs::write(&path, body).unwrap();
        let result = parse_md(&path, "items").unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].label, "Red");
        assert_eq!(result[0].value.as_deref(), Some("apple"));
        assert_eq!(result[0].hotkey.as_deref(), Some("CTRL+R"));
        assert_eq!(result[1].label, "Blue");
        assert_eq!(result[1].value.as_deref(), Some("sky"));
        assert_eq!(result[1].hotkey.as_deref(), Some("ALT+B"));
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn raw_option_from_string_label_only() {
        let opt: RawOption = "Red".to_string().into();
        assert_eq!(opt.label, "Red");
        assert!(opt.value.is_none());
        assert!(opt.hotkey.is_none());
        assert!(opt.disabled.is_none());
    }

    #[test]
    fn raw_option_from_str_label_only() {
        let opt: RawOption = "Green".into();
        assert_eq!(opt.label, "Green");
        assert!(opt.value.is_none());
    }
}
