//! Source resolution for choice option strings.
//!
//! This module turns the various CLI source flags into a flat
//! `Vec<String>` of raw option strings, ready for normalization.

use std::fs;
use std::io::{self, IsTerminal, Read};
use std::path::Path;

use serde_json::Value as JsonValue;

/// Errors that can occur while resolving options from a source.
#[derive(Debug, thiserror::Error)]
pub enum SourceError {
    #[error("no options provided: pass options as positional args, via stdin, or use a source flag")]
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
) -> Result<Vec<String>, SourceError> {
    let mut source_count = 0;
    if csv.is_some() { source_count += 1; }
    if list.is_some() { source_count += 1; }
    if rows.is_some() { source_count += 1; }
    if file.is_some() { source_count += 1; }
    if md.is_some() { source_count += 1; }
    if options_from_file.is_some() { source_count += 1; }
    if options_from_dictionary.is_some() { source_count += 1; }
    if !positional.is_empty() { source_count += 1; }

    if source_count > 1 {
        return Err(SourceError::MultipleSources);
    }

    if let Some(csv) = csv { return Ok(parse_csv(csv)); }
    if let Some(list) = list { return Ok(parse_list(list)); }
    if let Some(rows) = rows { return Ok(parse_rows(rows)); }
    if let Some(file) = file { return parse_file(file); }
    if let Some((path, prop)) = md { return parse_md(path, prop); }
    if let Some(path) = options_from_file {
        let body = fs::read_to_string(path)?;
        return Ok(parse_markdown_list(&body));
    }
    if let Some(path) = options_from_dictionary {
        let body = fs::read_to_string(path)?;
        return parse_dictionary(&body);
    }
    if !positional.is_empty() { return Ok(positional); }

    if io::stdin().is_terminal() {
        return Err(SourceError::NoSource);
    }
    let mut buf = String::new();
    io::stdin().read_to_string(&mut buf)?;
    let lines = parse_list(&buf);
    if lines.is_empty() { return Err(SourceError::NoSource); }
    Ok(lines)
}

fn parse_csv(csv: &str) -> Vec<String> {
    csv.split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}

fn parse_list(text: &str) -> Vec<String> {
    text.lines()
        .map(|line| line.trim_end_matches('\r'))
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect()
}

fn parse_rows(text: &str) -> Vec<String> {
    parse_list(text)
}

fn parse_file(path: &Path) -> Result<Vec<String>, SourceError> {
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
        _ => {
            let trimmed = body.trim_start();
            if trimmed.starts_with('[') || trimmed.starts_with('{') {
                parse_json(&body)
            } else if trimmed.starts_with("---") || trimmed.starts_with('-') {
                parse_yaml(&body)
            } else {
                Ok(parse_list(&body))
            }
        }
    }
}

fn parse_json(body: &str) -> Result<Vec<String>, SourceError> {
    let value: JsonValue =
        serde_json::from_str(body).map_err(|e| SourceError::Parse(e.to_string()))?;
    extract_string_array(&value)
}

fn parse_jsonl(body: &str) -> Result<Vec<String>, SourceError> {
    let mut results = Vec::new();
    for line in body.lines() {
        let line = line.trim();
        if line.is_empty() { continue; }
        let value: JsonValue =
            serde_json::from_str(line).map_err(|e| SourceError::Parse(e.to_string()))?;
        match value {
            JsonValue::String(s) => results.push(s),
            JsonValue::Object(mut map) => {
                if let Some(JsonValue::String(label)) = map.remove("label") {
                    results.push(label);
                } else if let Some(JsonValue::String(value)) = map.remove("value") {
                    results.push(value);
                } else {
                    return Err(SourceError::Parse(
                        "JSONL object must have a 'label' or 'value' field".into(),
                    ));
                }
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

fn parse_yaml(body: &str) -> Result<Vec<String>, SourceError> {
    let value: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(body).map_err(|e| SourceError::Parse(e.to_string()))?;
    extract_yaml_string_array(&value)
}

fn parse_toml(body: &str) -> Result<Vec<String>, SourceError> {
    let value: toml::Value =
        toml::from_str(body).map_err(|e| SourceError::Parse(e.to_string()))?;
    extract_toml_string_array(&value)
}

fn parse_csv_file(body: &str) -> Result<Vec<String>, SourceError> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .from_reader(body.as_bytes());
    let mut results = Vec::new();
    for record in reader.records() {
        let record = record.map_err(|e| SourceError::Parse(e.to_string()))?;
        if record.len() >= 2 {
            results.push(format!("{}::{}", record[0].trim(), record[1].trim()));
        } else if record.len() == 1 {
            results.push(record[0].trim().to_string());
        }
    }
    Ok(results)
}

fn extract_string_array(value: &JsonValue) -> Result<Vec<String>, SourceError> {
    let arr = value.as_array().ok_or(SourceError::NotAnArray)?;
    let mut results = Vec::new();
    for item in arr {
        match item {
            JsonValue::String(s) => results.push(s.clone()),
            JsonValue::Object(map) => {
                if let Some(JsonValue::String(label)) = map.get("label") {
                    results.push(label.clone());
                } else if let Some(JsonValue::String(value)) = map.get("value") {
                    results.push(value.clone());
                } else {
                    return Err(SourceError::Parse(
                        "JSON object must have a 'label' or 'value' field".into(),
                    ));
                }
            }
            other => results.push(other.to_string()),
        }
    }
    Ok(results)
}

fn extract_yaml_string_array(value: &serde_yaml_ng::Value) -> Result<Vec<String>, SourceError> {
    let arr = value.as_sequence().ok_or(SourceError::NotAnArray)?;
    let mut results = Vec::new();
    for item in arr {
        match item {
            serde_yaml_ng::Value::String(s) => results.push(s.clone()),
            serde_yaml_ng::Value::Mapping(map) => {
                if let Some(serde_yaml_ng::Value::String(label)) = map.get(serde_yaml_ng::Value::String("label".into())) {
                    results.push(label.clone());
                } else if let Some(serde_yaml_ng::Value::String(value)) = map.get(serde_yaml_ng::Value::String("value".into())) {
                    results.push(value.clone());
                } else {
                    return Err(SourceError::Parse(
                        "YAML object must have a 'label' or 'value' field".into(),
                    ));
                }
            }
            other => {
                let s = serde_yaml_ng::to_string(other).unwrap_or_default();
                results.push(s.trim().to_string());
            }
        }
    }
    Ok(results)
}

fn extract_toml_string_array(value: &toml::Value) -> Result<Vec<String>, SourceError> {
    let arr = value.as_array().ok_or(SourceError::NotAnArray)?;
    let mut results = Vec::new();
    for item in arr {
        match item {
            toml::Value::String(s) => results.push(s.clone()),
            toml::Value::Table(map) => {
                if let Some(toml::Value::String(label)) = map.get("label") {
                    results.push(label.clone());
                } else if let Some(toml::Value::String(value)) = map.get("value") {
                    results.push(value.clone());
                } else {
                    return Err(SourceError::Parse(
                        "TOML object must have a 'label' or 'value' field".into(),
                    ));
                }
            }
            other => results.push(other.to_string()),
        }
    }
    Ok(results)
}

fn parse_markdown_list(body: &str) -> Vec<String> {
    body.lines()
        .filter_map(|line| {
            let trimmed = line.trim_start();
            if let Some(rest) = strip_bullet_prefix(trimmed) {
                let value = rest.trim();
                if value.is_empty() { return None; }
                return Some(value.to_string());
            }
            if let Some(rest) = strip_numbered_prefix(trimmed) {
                let value = rest.trim();
                if value.is_empty() { return None; }
                return Some(value.to_string());
            }
            None
        })
        .collect()
}

fn strip_bullet_prefix(line: &str) -> Option<&str> {
    for marker in ["- ", "* ", "+ "] {
        if let Some(rest) = line.strip_prefix(marker) {
            return Some(rest);
        }
    }
    None
}

fn strip_numbered_prefix(line: &str) -> Option<&str> {
    let digit_count = line.chars().take_while(char::is_ascii_digit).count();
    if digit_count == 0 { return None; }
    let (digits, rest) = line.split_at(digit_count);
    let _: u32 = digits.parse().ok()?;
    let rest = rest.strip_prefix('.').or_else(|| rest.strip_prefix(')'))?;
    let rest = rest.strip_prefix(' ')?;
    Some(rest)
}

fn parse_dictionary(body: &str) -> Result<Vec<String>, SourceError> {
    let value: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(body).map_err(|e| SourceError::Parse(e.to_string()))?;
    let mapping = match value {
        serde_yaml_ng::Value::Mapping(m) => m,
        _ => return Err(SourceError::Parse("dictionary input must be a mapping/object".into())),
    };
    Ok(mapping
        .into_iter()
        .map(|(key, value)| {
            let label = yaml_value_to_string(&key);
            let val = yaml_value_to_string(&value);
            if label == val {
                label
            } else {
                format!("{}::{}", label, val)
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

fn parse_md(path: &Path, prop: &str) -> Result<Vec<String>, SourceError> {
    let body = fs::read_to_string(path)?;
    // Extract frontmatter between --- delimiters
    let trimmed = body.trim_start();
    let after_first = trimmed.strip_prefix("---").ok_or_else(|| {
        SourceError::Parse("markdown file must have frontmatter delimited by ---".into())
    })?;
    let Some(end_idx) = after_first.find("\n---") else {
        return Err(SourceError::Parse("markdown frontmatter not properly closed".into()));
    };
    let frontmatter = &after_first[..end_idx];
    let value: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(frontmatter).map_err(|e| SourceError::Parse(e.to_string()))?;
    let mapping = match value {
        serde_yaml_ng::Value::Mapping(m) => m,
        _ => return Err(SourceError::Parse("frontmatter must be a YAML mapping".into())),
    };
    let prop_value = mapping
        .get(serde_yaml_ng::Value::String(prop.into()))
        .ok_or_else(|| SourceError::MdPropNotArray { prop: prop.to_string() })?;
    extract_yaml_string_array(prop_value)
        .map_err(|_| SourceError::MdPropNotArray { prop: prop.to_string() })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_csv_splits_and_trims() {
        let result = parse_csv("Red, Green , Blue");
        assert_eq!(result, vec!["Red", "Green", "Blue"]);
    }

    #[test]
    fn parse_csv_drops_empty() {
        let result = parse_csv("a, ,b,");
        assert_eq!(result, vec!["a", "b"]);
    }

    #[test]
    fn parse_list_splits_lines() {
        let result = parse_list("alpha\nbeta\n\ngamma\n");
        assert_eq!(result, vec!["alpha", "beta", "gamma"]);
    }

    #[test]
    fn parse_file_json_array() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_options.json");
        std::fs::write(&path, r#"["Red", "Green", "Blue"]"#).unwrap();
        let result = parse_file(&path).unwrap();
        assert_eq!(result, vec!["Red", "Green", "Blue"]);
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn parse_file_yaml_array() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_options.yaml");
        std::fs::write(&path, "- Red\n- Green\n- Blue\n").unwrap();
        let result = parse_file(&path).unwrap();
        assert_eq!(result, vec!["Red", "Green", "Blue"]);
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn parse_file_toml_array() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_options.toml");
        std::fs::write(&path, "options = [\"Red\", \"Green\", \"Blue\"]\n").unwrap();
        // TOML files must have a top-level key, not just an array
        // So this test should expect NotAnArray
        let result = parse_file(&path);
        assert!(matches!(result, Err(SourceError::NotAnArray)));
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn resolve_raw_options_csv_source() {
        let result = resolve_raw_options(
            Some("a,b,c"),
            None, None, None, None, None, None,
            vec![],
        ).unwrap();
        assert_eq!(result, vec!["a", "b", "c"]);
    }

    #[test]
    fn resolve_raw_options_multiple_sources_error() {
        let result = resolve_raw_options(
            Some("a,b"),
            Some("c\nd"),
            None, None, None, None, None,
            vec![],
        );
        assert!(matches!(result, Err(SourceError::MultipleSources)));
    }

    #[test]
    fn parse_md_frontmatter() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_options.md");
        std::fs::write(&path, "---\nitems:\n  - Red\n  - Green\n---\n# Hello\n").unwrap();
        let result = parse_md(&path, "items").unwrap();
        assert_eq!(result, vec!["Red", "Green"]);
        std::fs::remove_file(&path).unwrap();
    }
}