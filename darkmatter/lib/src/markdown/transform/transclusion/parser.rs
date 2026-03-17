//! Block and frontmatter transclusion parsing.

use super::types::{
    BlockDirective, BlockOptions, DirectiveKind, FrontmatterRefs, ReplaceOption, TransclusionError,
};
use crate::markdown::FrontmatterMap;
use crate::markdown::transform::parse_utils::{
    Cursor, CursorError, find_code_regions, is_in_code_region,
};
use serde_json::Value;

/// Parses block transclusion directives from markdown content.
pub fn parse_directives(content: &str) -> Result<Vec<BlockDirective>, TransclusionError> {
    let code_regions = find_code_regions(content);
    let mut directives = Vec::new();

    let bytes = content.as_bytes();
    let mut line_start = 0usize;
    let mut line_number = 1usize;

    for i in 0..=bytes.len() {
        let is_eol = i == bytes.len() || bytes[i] == b'\n';
        if !is_eol {
            continue;
        }

        let line_end = i;
        let span_end = if i < bytes.len() { i + 1 } else { i };
        let line = &content[line_start..line_end];
        let trimmed = line.trim();

        if trimmed.starts_with("::") {
            let first_non_ws = line_start + line.len().saturating_sub(line.trim_start().len());
            if !is_in_code_region(first_non_ws, &code_regions) {
                let (kind, raw_target, options) = parse_directive_line(trimmed, line_number)?;
                directives.push(BlockDirective {
                    kind,
                    raw_target,
                    options,
                    span: line_start..span_end,
                    line: line_number,
                });
            }
        }

        line_start = i.saturating_add(1);
        line_number += 1;
    }

    Ok(directives)
}

/// Parses frontmatter `prologue` and `epilogue` references.
pub fn parse_frontmatter_refs(
    frontmatter: &FrontmatterMap,
) -> Result<FrontmatterRefs, TransclusionError> {
    let prologue = parse_reference_field(frontmatter.get("prologue"), "prologue")?;
    let epilogue = parse_reference_field(frontmatter.get("epilogue"), "epilogue")?;

    Ok(FrontmatterRefs { prologue, epilogue })
}

fn parse_reference_field(
    value: Option<&Value>,
    field_name: &str,
) -> Result<Vec<String>, TransclusionError> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };

    match value {
        Value::String(s) => Ok(vec![s.clone()]),
        Value::Array(items) => {
            let mut refs = Vec::with_capacity(items.len());
            for item in items {
                let Some(s) = item.as_str() else {
                    return Err(TransclusionError::ParseDirective {
                        line: 0,
                        message: format!(
                            "Frontmatter '{}' must be a string or array of strings",
                            field_name
                        ),
                    });
                };
                refs.push(s.to_string());
            }
            Ok(refs)
        }
        _ => Err(TransclusionError::ParseDirective {
            line: 0,
            message: format!(
                "Frontmatter '{}' must be a string or array of strings",
                field_name
            ),
        }),
    }
}

fn parse_directive_line(
    input: &str,
    line: usize,
) -> Result<(DirectiveKind, String, BlockOptions), TransclusionError> {
    let mut cursor = Cursor::new(input);

    cursor.expect_literal("::", line)?;
    let kind_str = cursor.read_identifier(line)?;
    let kind = match kind_str.as_str() {
        "file" => DirectiveKind::File,
        "code" => DirectiveKind::Code,
        "url" => DirectiveKind::Url,
        _ => {
            return Err(TransclusionError::ParseDirective {
                line,
                message: format!(
                    "Unknown directive kind '{}': expected file/code/url",
                    kind_str
                ),
            });
        }
    };

    cursor.skip_ws();
    let raw_target = cursor.read_value(line)?;
    if raw_target.is_empty() {
        return Err(TransclusionError::ParseDirective {
            line,
            message: "Directive target cannot be empty".to_string(),
        });
    }

    let mut options = BlockOptions::default();

    while !cursor.is_eof() {
        cursor.skip_ws();
        if cursor.is_eof() {
            break;
        }

        let key = cursor.read_identifier(line)?;
        cursor.skip_ws();
        cursor.expect_char('=', line)?;
        cursor.skip_ws();
        let value = cursor.read_value(line)?;

        apply_option(&mut options, &key, &value, line)?;
    }

    Ok((kind, raw_target, options))
}

fn apply_option(
    options: &mut BlockOptions,
    key: &str,
    value: &str,
    line: usize,
) -> Result<(), TransclusionError> {
    match key {
        "replace" => {
            if value.eq_ignore_ascii_case("true") {
                options.replace = ReplaceOption::ParentWins;
            } else if value.eq_ignore_ascii_case("false") {
                options.replace = ReplaceOption::InheritDefault;
            } else {
                let parsed: Value = serde_json::from_str(value)?;
                let Some(obj) = parsed.as_object() else {
                    return Err(TransclusionError::ParseDirective {
                        line,
                        message: "replace option must be true/false or a JSON object".to_string(),
                    });
                };
                options.replace = ReplaceOption::OneOff(obj.clone());
            }
        }
        "quotation" => {
            if value.eq_ignore_ascii_case("false") {
                options.quotation = None;
            } else if value.eq_ignore_ascii_case("true") {
                options.quotation = Some(String::new());
            } else {
                options.quotation = Some(value.to_string());
            }
        }
        "disclosure" => {
            if value.eq_ignore_ascii_case("false") {
                options.disclosure = None;
            } else {
                options.disclosure = Some(value.to_string());
            }
        }
        "when" => {
            options.when_expr = Some(value.to_string());
        }
        "exclude" => {
            options.exclude.push(value.to_string());
        }
        _ => {
            options.unknown_options.push(key.to_string());
        }
    }

    Ok(())
}

impl From<CursorError> for TransclusionError {
    fn from(e: CursorError) -> Self {
        TransclusionError::ParseDirective {
            line: e.line,
            message: e.message,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_file_directive() {
        let content = "::file ./doc.md\n";
        let directives = parse_directives(content).unwrap();

        assert_eq!(directives.len(), 1);
        assert_eq!(directives[0].kind, DirectiveKind::File);
        assert_eq!(directives[0].raw_target, "./doc.md");
    }

    #[test]
    fn parses_code_directive_options() {
        let content = r#"::code ./mod.rs quotation=true disclosure=More when=env.DEBUG"#;
        let directives = parse_directives(content).unwrap();

        assert_eq!(directives.len(), 1);
        let options = &directives[0].options;
        assert_eq!(options.quotation, Some(String::new()));
        assert_eq!(options.disclosure, Some("More".to_string()));
        assert_eq!(options.when_expr, Some("env.DEBUG".to_string()));
    }

    #[test]
    fn parses_json_replace_option() {
        let content = r#"::file ./doc.md replace={"FOO":"BAR"}"#;
        let directives = parse_directives(content).unwrap();

        assert_eq!(directives.len(), 1);
        match &directives[0].options.replace {
            ReplaceOption::OneOff(map) => {
                assert_eq!(map.get("FOO"), Some(&Value::String("BAR".to_string())));
            }
            _ => panic!("Expected one-off replace map"),
        }
    }

    #[test]
    fn ignores_directives_inside_fenced_code() {
        let content = r#"```md
::file ./ignored.md
```
::file ./included.md
"#;
        let directives = parse_directives(content).unwrap();

        assert_eq!(directives.len(), 1);
        assert_eq!(directives[0].raw_target, "./included.md");
    }

    #[test]
    fn parses_exclude_option() {
        let content = r###"::file ./doc.md exclude="## Bad Section" exclude="## Also Bad*""###;
        let directives = parse_directives(content).unwrap();

        assert_eq!(directives.len(), 1);
        assert_eq!(
            directives[0].options.exclude,
            vec!["## Bad Section", "## Also Bad*"]
        );
    }

    #[test]
    fn parses_nested_double_quotes_in_when() {
        let content = r#"::file ./doc.md when="stage == "tech-design"""#;
        let directives = parse_directives(content).unwrap();

        assert_eq!(directives.len(), 1);
        assert_eq!(
            directives[0].options.when_expr,
            Some(r#"stage == "tech-design""#.to_string())
        );
    }

    #[test]
    fn parses_single_quotes_in_when() {
        let content = r#"::file ./doc.md when="stage == 'tech-design'""#;
        let directives = parse_directives(content).unwrap();

        assert_eq!(directives.len(), 1);
        assert_eq!(
            directives[0].options.when_expr,
            Some("stage == 'tech-design'".to_string())
        );
    }

    #[test]
    fn parses_nested_quotes_followed_by_another_option() {
        let content = r#"::file ./doc.md when="stage == "plan"" quotation=true"#;
        let directives = parse_directives(content).unwrap();

        assert_eq!(directives.len(), 1);
        assert_eq!(
            directives[0].options.when_expr,
            Some(r#"stage == "plan""#.to_string())
        );
        assert_eq!(directives[0].options.quotation, Some(String::new()));
    }

    #[test]
    fn parses_frontmatter_refs() {
        let fm: FrontmatterMap = serde_json::from_value(serde_json::json!({
            "prologue": "./a.md",
            "epilogue": ["./b.md", "./c.md"]
        }))
        .unwrap();

        let refs = parse_frontmatter_refs(&fm).unwrap();
        assert_eq!(refs.prologue, vec!["./a.md"]);
        assert_eq!(refs.epilogue, vec!["./b.md", "./c.md"]);
    }
}
