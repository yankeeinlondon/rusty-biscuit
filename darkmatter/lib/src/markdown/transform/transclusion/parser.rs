//! Block and frontmatter transclusion parsing.

use super::types::{
    BlockDirective, BlockOptions, DirectiveKind, FrontmatterRefs, ReplaceOption, TransclusionError,
};
use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use serde_json::Value;
use std::collections::HashMap;

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
    frontmatter: &HashMap<String, Value>,
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

fn find_code_regions(content: &str) -> Vec<(usize, usize)> {
    let mut regions = Vec::new();
    let mut in_code_block = false;
    let mut code_block_start = 0;

    for (event, range) in Parser::new_ext(content, Options::all()).into_offset_iter() {
        match event {
            Event::Code(_) => {
                regions.push((range.start, range.end));
            }
            Event::Start(Tag::CodeBlock(_)) => {
                in_code_block = true;
                code_block_start = range.start;
            }
            Event::End(TagEnd::CodeBlock) => {
                if in_code_block {
                    regions.push((code_block_start, range.end));
                    in_code_block = false;
                }
            }
            _ => {}
        }
    }

    regions
}

fn is_in_code_region(position: usize, regions: &[(usize, usize)]) -> bool {
    regions
        .iter()
        .any(|(start, end)| position >= *start && position < *end)
}

struct Cursor<'a> {
    input: &'a str,
    pos: usize,
}

impl<'a> Cursor<'a> {
    fn new(input: &'a str) -> Self {
        Self { input, pos: 0 }
    }

    fn is_eof(&self) -> bool {
        self.pos >= self.input.len()
    }

    fn current(&self) -> Option<char> {
        self.input[self.pos..].chars().next()
    }

    fn advance(&mut self) {
        if let Some(ch) = self.current() {
            self.pos += ch.len_utf8();
        }
    }

    fn skip_ws(&mut self) {
        while let Some(ch) = self.current() {
            if ch.is_whitespace() {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn expect_literal(&mut self, literal: &str, line: usize) -> Result<(), TransclusionError> {
        if self.input[self.pos..].starts_with(literal) {
            self.pos += literal.len();
            Ok(())
        } else {
            Err(TransclusionError::ParseDirective {
                line,
                message: format!("Expected '{}'", literal),
            })
        }
    }

    fn expect_char(&mut self, expected: char, line: usize) -> Result<(), TransclusionError> {
        match self.current() {
            Some(ch) if ch == expected => {
                self.advance();
                Ok(())
            }
            Some(ch) => Err(TransclusionError::ParseDirective {
                line,
                message: format!("Expected '{}', found '{}'", expected, ch),
            }),
            None => Err(TransclusionError::ParseDirective {
                line,
                message: format!("Expected '{}' at end of directive", expected),
            }),
        }
    }

    fn read_identifier(&mut self, line: usize) -> Result<String, TransclusionError> {
        let mut out = String::new();
        let Some(ch) = self.current() else {
            return Err(TransclusionError::ParseDirective {
                line,
                message: "Unexpected end of directive".to_string(),
            });
        };

        if !is_identifier_start(ch) {
            return Err(TransclusionError::ParseDirective {
                line,
                message: format!("Expected identifier, found '{}'", ch),
            });
        }

        while let Some(ch) = self.current() {
            if is_identifier_char(ch) {
                out.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        Ok(out)
    }

    fn read_value(&mut self, line: usize) -> Result<String, TransclusionError> {
        let Some(ch) = self.current() else {
            return Err(TransclusionError::ParseDirective {
                line,
                message: "Expected value, found end of directive".to_string(),
            });
        };

        match ch {
            '\'' | '"' => self.read_quoted_value(line),
            '{' | '[' => self.read_balanced_value(ch, line),
            _ => self.read_bare_value(),
        }
    }

    fn read_quoted_value(&mut self, line: usize) -> Result<String, TransclusionError> {
        let quote = self
            .current()
            .ok_or_else(|| TransclusionError::ParseDirective {
                line,
                message: "Expected quote".to_string(),
            })?;
        self.advance();

        let mut out = String::new();
        loop {
            match self.current() {
                None => {
                    return Err(TransclusionError::ParseDirective {
                        line,
                        message: "Unterminated quoted value".to_string(),
                    });
                }
                Some(ch) if ch == quote => {
                    self.advance();
                    break;
                }
                Some('\\') => {
                    self.advance();
                    match self.current() {
                        Some('n') => {
                            out.push('\n');
                            self.advance();
                        }
                        Some('t') => {
                            out.push('\t');
                            self.advance();
                        }
                        Some('r') => {
                            out.push('\r');
                            self.advance();
                        }
                        Some('\\') => {
                            out.push('\\');
                            self.advance();
                        }
                        Some(ch) if ch == quote => {
                            out.push(ch);
                            self.advance();
                        }
                        Some(ch) => {
                            out.push(ch);
                            self.advance();
                        }
                        None => {
                            return Err(TransclusionError::ParseDirective {
                                line,
                                message: "Unterminated escape sequence".to_string(),
                            });
                        }
                    }
                }
                Some(ch) => {
                    out.push(ch);
                    self.advance();
                }
            }
        }

        Ok(out)
    }

    fn read_balanced_value(
        &mut self,
        opener: char,
        line: usize,
    ) -> Result<String, TransclusionError> {
        let closer = if opener == '{' { '}' } else { ']' };
        let mut depth = 0usize;
        let mut out = String::new();
        let mut in_string: Option<char> = None;

        while let Some(ch) = self.current() {
            out.push(ch);
            self.advance();

            match in_string {
                Some(quote) => {
                    if ch == '\\' {
                        if let Some(next) = self.current() {
                            out.push(next);
                            self.advance();
                        }
                    } else if ch == quote {
                        in_string = None;
                    }
                }
                None => {
                    if ch == '\'' || ch == '"' {
                        in_string = Some(ch);
                    } else if ch == opener {
                        depth += 1;
                    } else if ch == closer {
                        depth = depth.saturating_sub(1);
                        if depth == 0 {
                            return Ok(out);
                        }
                    }
                }
            }
        }

        Err(TransclusionError::ParseDirective {
            line,
            message: "Unterminated JSON option value".to_string(),
        })
    }

    fn read_bare_value(&mut self) -> Result<String, TransclusionError> {
        let mut out = String::new();
        while let Some(ch) = self.current() {
            if ch.is_whitespace() {
                break;
            }
            out.push(ch);
            self.advance();
        }
        Ok(out)
    }
}

fn is_identifier_start(ch: char) -> bool {
    ch.is_ascii_alphabetic() || ch == '_'
}

fn is_identifier_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_' || ch == '-'
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
        let content =
            r###"::file ./doc.md exclude="## Bad Section" exclude="## Also Bad*""###;
        let directives = parse_directives(content).unwrap();

        assert_eq!(directives.len(), 1);
        assert_eq!(
            directives[0].options.exclude,
            vec!["## Bad Section", "## Also Bad*"]
        );
    }

    #[test]
    fn parses_frontmatter_refs() {
        let fm: HashMap<String, Value> = serde_json::from_value(serde_json::json!({
            "prologue": "./a.md",
            "epilogue": ["./b.md", "./c.md"]
        }))
        .unwrap();

        let refs = parse_frontmatter_refs(&fm).unwrap();
        assert_eq!(refs.prologue, vec!["./a.md"]);
        assert_eq!(refs.epilogue, vec!["./b.md", "./c.md"]);
    }
}
