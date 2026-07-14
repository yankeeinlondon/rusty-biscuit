//! Frontmatter YAML parsing, bounds detection, indentation normalization, and
//! markdown document rewriting — the file-I/O half of the compatibility layer.

use std::path::Path;

use biscuit_file::serde_yaml_ng;

use crate::error::{ClaudineError, Result};

#[derive(Debug, Clone)]
pub(crate) struct ParsedMarkdown {
    pub(crate) frontmatter: serde_yaml_ng::Mapping,
    pub(crate) body: String,
    pub(crate) had_frontmatter: bool,
}

#[derive(Debug, Clone, Copy)]
struct FrontmatterBounds {
    yaml_start: usize,
    yaml_end: usize,
    body_start: usize,
}

pub(crate) fn parse_markdown_document(content: &str) -> Result<ParsedMarkdown> {
    let content = content.strip_prefix('\u{feff}').unwrap_or(content);
    let Some(bounds) = frontmatter_bounds(content)? else {
        return Ok(ParsedMarkdown {
            frontmatter: serde_yaml_ng::Mapping::new(),
            body: content.to_string(),
            had_frontmatter: false,
        });
    };

    let frontmatter_raw = &content[bounds.yaml_start..bounds.yaml_end];
    let body = &content[bounds.body_start..];
    let frontmatter = parse_frontmatter_mapping(frontmatter_raw)?;
    Ok(ParsedMarkdown {
        frontmatter,
        body: body.to_string(),
        had_frontmatter: true,
    })
}

fn parse_frontmatter_mapping(raw: &str) -> Result<serde_yaml_ng::Mapping> {
    if raw.trim().is_empty() {
        return Ok(serde_yaml_ng::Mapping::new());
    }

    let value: serde_yaml_ng::Value = match serde_yaml_ng::from_str(raw) {
        Ok(value) => value,
        Err(_) => return Ok(parse_frontmatter_lines(raw)),
    };
    match value {
        serde_yaml_ng::Value::Mapping(mapping) => Ok(mapping),
        serde_yaml_ng::Value::Null => Ok(serde_yaml_ng::Mapping::new()),
        _ => Err(ClaudineError::LinkingError(
            "frontmatter must be a YAML mapping".to_string(),
        )),
    }
}

/// Line-by-line fallback for frontmatter that isn't strict YAML.
///
/// Handles values like `argument-hint: [--force] [msg]` where square brackets
/// are literal text, not YAML flow sequences.
pub(super) fn parse_frontmatter_lines(raw: &str) -> serde_yaml_ng::Mapping {
    let mut mapping = serde_yaml_ng::Mapping::new();
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = trimmed.split_once(": ") {
            let key = key.trim();
            let value = value.trim();
            if !key.is_empty() && !key.contains(' ') {
                mapping.insert(
                    serde_yaml_ng::Value::String(key.to_string()),
                    serde_yaml_ng::Value::String(value.to_string()),
                );
            }
        }
    }
    mapping
}

pub(crate) fn frontmatter_has_indentation_tabs(content: &str) -> Result<bool> {
    let Some(bounds) = frontmatter_bounds(content)? else {
        return Ok(false);
    };

    Ok(yaml_indentation_has_tabs(
        &content[bounds.yaml_start..bounds.yaml_end],
    ))
}

pub(crate) fn fix_frontmatter_indentation_tabs(path: &Path) -> Result<bool> {
    let content = match std::fs::read_to_string(path) {
        Ok(content) => content,
        Err(_) => return Ok(false),
    };

    let Some(bounds) = frontmatter_bounds(&content)? else {
        return Ok(false);
    };

    let raw = &content[bounds.yaml_start..bounds.yaml_end];
    if !yaml_indentation_has_tabs(raw) {
        return Ok(false);
    }

    let normalized = normalize_yaml_indentation_tabs(raw);
    let mut rewritten =
        String::with_capacity(content.len() + normalized.len().saturating_sub(raw.len()));
    rewritten.push_str(&content[..bounds.yaml_start]);
    rewritten.push_str(&normalized);
    rewritten.push_str(&content[bounds.yaml_end..]);

    std::fs::write(path, rewritten)?;
    Ok(true)
}

fn frontmatter_bounds(content: &str) -> Result<Option<FrontmatterBounds>> {
    let bom_len = content
        .strip_prefix('\u{feff}')
        .map(|_| '\u{feff}'.len_utf8())
        .unwrap_or(0);
    let content = &content[bom_len..];

    let opening_len = if content.starts_with("---\n") {
        4
    } else if content.starts_with("---\r\n") {
        5
    } else {
        return Ok(None);
    };

    let rest = &content[opening_len..];
    let mut offset = 0usize;
    for line in rest.split_inclusive('\n') {
        let trimmed = line.trim_end_matches('\n').trim_end_matches('\r');
        if trimmed == "---" {
            let yaml_start = bom_len + opening_len;
            let yaml_end = yaml_start + offset;
            let body_start = yaml_start + offset + line.len();
            return Ok(Some(FrontmatterBounds {
                yaml_start,
                yaml_end,
                body_start,
            }));
        }
        offset += line.len();
    }

    if rest.trim_end_matches('\r') == "---" {
        let yaml_start = bom_len + opening_len;
        let yaml_end = yaml_start;
        let body_start = bom_len + opening_len + rest.len();
        return Ok(Some(FrontmatterBounds {
            yaml_start,
            yaml_end,
            body_start,
        }));
    }

    Err(ClaudineError::LinkingError(
        "unclosed YAML frontmatter delimiter".to_string(),
    ))
}

fn yaml_indentation_has_tabs(raw: &str) -> bool {
    raw.lines().any(|line| {
        let indent_len = line
            .char_indices()
            .find_map(|(idx, ch)| (!matches!(ch, ' ' | '\t')).then_some(idx))
            .unwrap_or(line.len());
        line[..indent_len].contains('\t')
    })
}

fn normalize_yaml_indentation_tabs(raw: &str) -> String {
    let mut normalized = String::with_capacity(raw.len());

    for line in raw.split_inclusive('\n') {
        let (line, ending) = if let Some(stripped) = line.strip_suffix("\r\n") {
            (stripped, "\r\n")
        } else if let Some(stripped) = line.strip_suffix('\n') {
            (stripped, "\n")
        } else {
            (line, "")
        };

        let indent_len = line
            .char_indices()
            .find_map(|(idx, ch)| (!matches!(ch, ' ' | '\t')).then_some(idx))
            .unwrap_or(line.len());
        let indent = &line[..indent_len];
        let rest = &line[indent_len..];

        if indent.contains('\t') {
            for ch in indent.chars() {
                match ch {
                    '\t' => normalized.push_str("    "),
                    ' ' => normalized.push(' '),
                    _ => {}
                }
            }
        } else {
            normalized.push_str(indent);
        }

        normalized.push_str(rest);
        normalized.push_str(ending);
    }

    normalized
}

pub(super) fn write_markdown_document(path: &Path, parsed: &ParsedMarkdown) -> Result<()> {
    let yaml = serde_yaml_ng::to_string(&parsed.frontmatter)?;
    let yaml = yaml.strip_prefix("---\n").unwrap_or(&yaml);
    let yaml = yaml.trim_end_matches('\n');

    let rendered = if parsed.frontmatter.is_empty() && !parsed.had_frontmatter {
        parsed.body.clone()
    } else if yaml.is_empty() {
        format!("---\n---\n{}", parsed.body)
    } else {
        format!("---\n{yaml}\n---\n{}", parsed.body)
    };

    std::fs::write(path, rendered)?;
    Ok(())
}
