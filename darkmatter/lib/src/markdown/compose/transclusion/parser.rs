//! Block and frontmatter transclusion parsing.

use super::types::{
    BlockDirective, BlockOptions, DeferredSetError, DirectiveKind, FrontmatterRefs, ReplaceOption,
    TransclusionError,
};
use crate::markdown::FrontmatterMap;
use crate::markdown::compose::parse_utils::{
    Cursor, find_code_regions, is_in_code_region,
};
use biscuit_terminal::errors::SourceContext;
use serde_json::Value;

/// Parses block transclusion directives from markdown content.
pub fn parse_directives(
    content: &str,
    ctx: SourceContext,
) -> Result<Vec<BlockDirective>, TransclusionError> {
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

        if trimmed.starts_with("::file")
            || trimmed.starts_with("::code")
            || trimmed.starts_with("::url")
        {
            let first_non_ws = line_start + line.len().saturating_sub(line.trim_start().len());
            if !is_in_code_region(first_non_ws, &code_regions) {
                let (kind, raw_target, options) = parse_directive_line(trimmed, line_number, &ctx)?;
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
    ctx: SourceContext,
) -> Result<FrontmatterRefs, TransclusionError> {
    let prologue = parse_reference_field(frontmatter.get("prologue"), "prologue", &ctx)?;
    let epilogue = parse_reference_field(frontmatter.get("epilogue"), "epilogue", &ctx)?;

    Ok(FrontmatterRefs { prologue, epilogue })
}

fn parse_reference_field(
    value: Option<&Value>,
    field_name: &str,
    ctx: &SourceContext,
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
                        ctx: Box::new(ctx.clone()),
                        line: 0,
                        message: format!(
                            "Frontmatter '{}' must be a string or array of strings",
                            field_name
                        ),
                        caret_col: None,
                    });
                };
                refs.push(s.to_string());
            }
            Ok(refs)
        }
        _ => Err(TransclusionError::ParseDirective {
            ctx: Box::new(ctx.clone()),
            line: 0,
            message: format!(
                "Frontmatter '{}' must be a string or array of strings",
                field_name
            ),
            caret_col: None,
        }),
    }
}

fn parse_directive_line(
    input: &str,
    line: usize,
    ctx: &SourceContext,
) -> Result<(DirectiveKind, String, BlockOptions), TransclusionError> {
    let mut cursor = Cursor::new(input);

    cursor
        .expect_literal("::", line)
        .map_err(|e| e.into_transclusion_error(ctx.clone()))?;
    let kind_str = cursor
        .read_identifier(line)
        .map_err(|e| e.into_transclusion_error(ctx.clone()))?;
    let kind = match kind_str.as_str() {
        "file" => DirectiveKind::File,
        "code" => DirectiveKind::Code,
        "url" => DirectiveKind::Url,
        _ => {
            return Err(TransclusionError::ParseDirective {
                ctx: Box::new(ctx.clone()),
                line,
                message: format!(
                    "Unknown directive kind '{}': expected file/code/url",
                    kind_str
                ),
                caret_col: None,
            });
        }
    };

    cursor.skip_ws();
    let raw_target = cursor
        .read_value(line)
        .map_err(|e| e.into_transclusion_error(ctx.clone()))?;
    if raw_target.is_empty() {
        return Err(TransclusionError::ParseDirective {
            ctx: Box::new(ctx.clone()),
            line,
            message: "Directive target cannot be empty".to_string(),
            caret_col: None,
        });
    }

    let mut options = BlockOptions::default();

    while !cursor.is_eof() {
        cursor.skip_ws();
        if cursor.is_eof() {
            break;
        }

        let key = cursor
            .read_identifier(line)
            .map_err(|e| e.into_transclusion_error(ctx.clone()))?;

        if key == "set" {
            parse_set_option(&mut cursor, &mut options, line, ctx)?;
            continue;
        }

        cursor.skip_ws();
        cursor
            .expect_char('=', line)
            .map_err(|e| e.into_transclusion_error(ctx.clone()))?;
        cursor.skip_ws();
        let value = cursor
            .read_value(line)
            .map_err(|e| e.into_transclusion_error(ctx.clone()))?;

        apply_option(&mut options, &key, &value, line, ctx)?;
    }

    Ok((kind, raw_target, options))
}

/// Parses a `set=<object>` or `set.NAME=<value>` clause.
///
/// The leading `set` identifier has already been consumed. This function
/// consumes the optional `.NAME` suffix, the required `=`, and the JSON5
/// RHS, then installs the parsed payload onto `options`.
fn parse_set_option(
    cursor: &mut Cursor<'_>,
    options: &mut BlockOptions,
    line: usize,
    ctx: &SourceContext,
) -> Result<(), TransclusionError> {
    let dotted = cursor
        .read_dotted_suffix(line)
        .map_err(|e| e.into_transclusion_error(ctx.clone()))?;
    cursor.skip_ws();
    cursor
        .expect_char('=', line)
        .map_err(|e| e.into_transclusion_error(ctx.clone()))?;
    cursor.skip_ws();

    let was_quoted = matches!(cursor.current(), Some('\'') | Some('"'));
    let raw = cursor
        .read_value(line)
        .map_err(|e| e.into_transclusion_error(ctx.clone()))?;

    if raw.is_empty() && !was_quoted {
        return Err(TransclusionError::ParseDirective {
            ctx: Box::new(ctx.clone()),
            line,
            message: match &dotted {
                Some(name) => format!("'set.{}=' requires a JSON5 value", name),
                None => "'set=' requires a JSON5 value".to_string(),
            },
            caret_col: None,
        });
    }

    match dotted {
        Some(name) => apply_set_property(options, name, &raw, was_quoted, line, ctx),
        None => apply_set_object(options, &raw, was_quoted, line),
    }
}

/// Applies a `set.NAME=<value>` property-form clause.
///
/// When the RHS was wrapped in shell-style quotes, the stripped content
/// is taken as a literal string. Otherwise the content is parsed as a
/// JSON5 value so numeric/boolean/null/array/object literals work.
fn apply_set_property(
    options: &mut BlockOptions,
    name: String,
    raw: &str,
    was_quoted: bool,
    line: usize,
    ctx: &SourceContext,
) -> Result<(), TransclusionError> {
    let parsed = if was_quoted {
        parse_json5_value(raw).unwrap_or_else(|_| Value::String(raw.to_string()))
    } else {
        parse_json5_value(raw).map_err(|e| TransclusionError::ParseDirective {
            ctx: Box::new(ctx.clone()),
            line,
            message: format!("'set.{}=' requires a JSON5 value: {}", name, e),
            caret_col: None,
        })?
    };

    if options.set_properties.iter().any(|(n, _)| n == &name) {
        options
            .deferred_set_errors
            .push(DeferredSetError::ReassignedProperty { name: name.clone() });
    }
    options.set_properties.push((name, parsed));
    Ok(())
}

/// Applies a `set=<value>` object-form clause.
///
/// The RHS must parse as a JSON5 object. Any other JSON5 value (scalar,
/// array) is a grammar error even if it parses successfully. Duplicate
/// `set=` on the same directive uses the `<object>` sentinel name.
fn apply_set_object(
    options: &mut BlockOptions,
    raw: &str,
    was_quoted: bool,
    line: usize,
) -> Result<(), TransclusionError> {
    let parsed = match parse_json5_value(raw) {
        Ok(v) => v,
        Err(e) => {
            options
                .deferred_set_errors
                .push(DeferredSetError::InvalidAssignment {
                    raw: format_raw_for_error(raw, was_quoted),
                    reason: format!("failed to parse as JSON5: {}", e),
                    line,
                });
            return Ok(());
        }
    };

    let Value::Object(map) = parsed else {
        options
            .deferred_set_errors
            .push(DeferredSetError::InvalidAssignment {
                raw: format_raw_for_error(raw, was_quoted),
                reason: "expected JSON5 object".to_string(),
                line,
            });
        return Ok(());
    };

    if options.set_object.is_some() {
        options
            .deferred_set_errors
            .push(DeferredSetError::ReassignedProperty {
                name: "<object>".to_string(),
            });
        return Ok(());
    }
    options.set_object = Some(map);
    Ok(())
}

/// Re-wraps a quoted RHS with single quotes for error messages so users
/// see the original directive shape rather than the cursor-stripped form.
fn format_raw_for_error(raw: &str, was_quoted: bool) -> String {
    if was_quoted {
        format!("'{}'", raw)
    } else {
        raw.to_string()
    }
}

/// Parses `raw` as a JSON5 value, returning the underlying JSON value.
fn parse_json5_value(raw: &str) -> Result<Value, String> {
    biscuit_file::Json5::from_str(raw)
        .map(|j| j.value().clone())
        .map_err(|e| e.to_string())
}

fn apply_option(
    options: &mut BlockOptions,
    key: &str,
    value: &str,
    line: usize,
    ctx: &SourceContext,
) -> Result<(), TransclusionError> {
    match key {
        "replace" => {
            if value.eq_ignore_ascii_case("true") {
                options.replace = ReplaceOption::ParentWins;
            } else if value.eq_ignore_ascii_case("false") {
                options.replace = ReplaceOption::InheritDefault;
            } else {
                let parsed: Value = serde_json::from_str(value).map_err(|_| {
                    TransclusionError::ParseDirective {
                        ctx: Box::new(ctx.clone()),
                        line,
                        message: "replace option must be true/false or a JSON object".to_string(),
                        caret_col: None,
                    }
                })?;
                let Some(obj) = parsed.as_object() else {
                    return Err(TransclusionError::ParseDirective {
                        ctx: Box::new(ctx.clone()),
                        line,
                        message: "replace option must be true/false or a JSON object".to_string(),
                        caret_col: None,
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::Arc;

    fn dummy_ctx(content: &str) -> SourceContext {
        SourceContext::new(
            PathBuf::from("/test.md"),
            PathBuf::from("test.md"),
            Arc::from(content),
        )
    }

    #[test]
    fn parses_simple_file_directive() {
        let content = "::file ./doc.md\n";
        let directives = parse_directives(content, dummy_ctx(content)).unwrap();

        assert_eq!(directives.len(), 1);
        assert_eq!(directives[0].kind, DirectiveKind::File);
        assert_eq!(directives[0].raw_target, "./doc.md");
    }

    #[test]
    fn parses_code_directive_options() {
        let content = r#"::code ./mod.rs quotation=true disclosure=More when=env.DEBUG"#;
        let directives = parse_directives(content, dummy_ctx(content)).unwrap();

        assert_eq!(directives.len(), 1);
        let options = &directives[0].options;
        assert_eq!(options.quotation, Some(String::new()));
        assert_eq!(options.disclosure, Some("More".to_string()));
        assert_eq!(options.when_expr, Some("env.DEBUG".to_string()));
    }

    #[test]
    fn parses_json_replace_option() {
        let content = r#"::file ./doc.md replace={"FOO":"BAR"}"#;
        let directives = parse_directives(content, dummy_ctx(content)).unwrap();

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
        let directives = parse_directives(content, dummy_ctx(content)).unwrap();

        assert_eq!(directives.len(), 1);
        assert_eq!(directives[0].raw_target, "./included.md");
    }

    #[test]
    fn parses_exclude_option() {
        let content = r###"::file ./doc.md exclude="## Bad Section" exclude="## Also Bad*""###;
        let directives = parse_directives(content, dummy_ctx(content)).unwrap();

        assert_eq!(directives.len(), 1);
        assert_eq!(
            directives[0].options.exclude,
            vec!["## Bad Section", "## Also Bad*"]
        );
    }

    #[test]
    fn parses_nested_double_quotes_in_when() {
        let content = r#"::file ./doc.md when="stage == "tech-design"""#;
        let directives = parse_directives(content, dummy_ctx(content)).unwrap();

        assert_eq!(directives.len(), 1);
        assert_eq!(
            directives[0].options.when_expr,
            Some(r#"stage == "tech-design""#.to_string())
        );
    }

    #[test]
    fn parses_single_quotes_in_when() {
        let content = r#"::file ./doc.md when="stage == 'tech-design'""#;
        let directives = parse_directives(content, dummy_ctx(content)).unwrap();

        assert_eq!(directives.len(), 1);
        assert_eq!(
            directives[0].options.when_expr,
            Some("stage == 'tech-design'".to_string())
        );
    }

    #[test]
    fn parses_nested_quotes_followed_by_another_option() {
        let content = r#"::file ./doc.md when="stage == "plan"" quotation=true"#;
        let directives = parse_directives(content, dummy_ctx(content)).unwrap();

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

        let content = "";
        let refs = parse_frontmatter_refs(&fm, dummy_ctx(content)).unwrap();
        assert_eq!(refs.prologue, vec!["./a.md"]);
        assert_eq!(refs.epilogue, vec!["./b.md", "./c.md"]);
    }

    // ── set / set.NAME grammar ───────────────────────────────────────

    fn parse_single(content: &str) -> BlockDirective {
        let mut directives = parse_directives(content, dummy_ctx(content)).unwrap();
        assert_eq!(directives.len(), 1);
        directives.pop().unwrap()
    }

    #[test]
    fn parses_set_property_string() {
        let directive = parse_single(r#"::file ./foo.md set.name="Bob""#);
        assert_eq!(directive.options.set_properties.len(), 1);
        assert_eq!(directive.options.set_properties[0].0, "name");
        assert_eq!(
            directive.options.set_properties[0].1,
            Value::String("Bob".to_string())
        );
    }

    #[test]
    fn parses_set_property_scalars() {
        let directive = parse_single(
            r#"::file ./foo.md set.age=42 set.tags=[1,2,3] set.meta={x:1} set.x=null set.ok=true"#,
        );
        let props: Vec<_> = directive
            .options
            .set_properties
            .iter()
            .map(|(n, v)| (n.as_str(), v.clone()))
            .collect();
        assert_eq!(props[0], ("age", serde_json::json!(42)));
        assert_eq!(props[1], ("tags", serde_json::json!([1, 2, 3])));
        assert_eq!(props[2], ("meta", serde_json::json!({"x": 1})));
        assert_eq!(props[3], ("x", Value::Null));
        assert_eq!(props[4], ("ok", Value::Bool(true)));
    }

    #[test]
    fn parses_set_object_form() {
        let directive = parse_single(r#"::file ./foo.md set='{a:1,b:"x"}'"#);
        let map = directive.options.set_object.expect("set_object present");
        assert_eq!(map.get("a"), Some(&serde_json::json!(1)));
        assert_eq!(map.get("b"), Some(&Value::String("x".to_string())));
    }

    #[test]
    fn parses_set_object_and_properties_in_order() {
        let directive = parse_single(r#"::file ./foo.md set='{a:1}' set.a="Z" set.b=2"#);
        let map = directive.options.set_object.expect("set_object present");
        assert_eq!(map.get("a"), Some(&serde_json::json!(1)));
        assert_eq!(directive.options.set_properties.len(), 2);
        assert_eq!(
            directive.options.set_properties[0],
            ("a".to_string(), Value::String("Z".to_string()))
        );
        assert_eq!(
            directive.options.set_properties[1],
            ("b".to_string(), serde_json::json!(2))
        );
    }

    #[test]
    fn set_unquoted_bare_word_is_parse_error() {
        let content = "::file ./foo.md set.name=Bob";
        let err = parse_directives(content, dummy_ctx(content)).unwrap_err();
        matches_parse_directive(&err);
    }

    #[test]
    fn set_empty_rhs_is_parse_error() {
        let content = "::file ./foo.md set.name=";
        let err = parse_directives(content, dummy_ctx(content)).unwrap_err();
        matches_parse_directive(&err);
    }

    #[test]
    fn set_bare_without_equals_is_parse_error() {
        let content = "::file ./foo.md set";
        let err = parse_directives(content, dummy_ctx(content)).unwrap_err();
        matches_parse_directive(&err);
    }

    #[test]
    fn set_nested_dotted_key_is_parse_error() {
        let content = r#"::file ./foo.md set.author.name="Bob""#;
        let err = parse_directives(content, dummy_ctx(content)).unwrap_err();
        matches_parse_directive(&err);
    }

    #[test]
    fn set_non_object_rhs_defers_error() {
        let directive = parse_single("::file ./foo.md set=42");
        assert!(
            directive
                .options
                .deferred_set_errors
                .iter()
                .any(|e| matches!(e, DeferredSetError::InvalidAssignment { .. })),
            "expected InvalidAssignment deferred error, got {:?}",
            directive.options.deferred_set_errors
        );
    }

    #[test]
    fn set_object_duplicate_defers_error() {
        let directive = parse_single(r#"::file ./foo.md set='{a:1}' set='{b:2}'"#);
        assert!(
            directive
                .options
                .deferred_set_errors
                .iter()
                .any(|e| matches!(
                    e,
                    DeferredSetError::ReassignedProperty { name } if name == "<object>"
                )),
            "expected ReassignedProperty(<object>) deferred error, got {:?}",
            directive.options.deferred_set_errors
        );
        assert!(directive.options.set_object.is_some());
    }

    #[test]
    fn set_property_duplicate_defers_error_and_rightmost_wins() {
        let directive = parse_single(r#"::file ./foo.md set.name="Bob" set.name="Mary""#);
        assert!(
            directive
                .options
                .deferred_set_errors
                .iter()
                .any(|e| matches!(
                    e,
                    DeferredSetError::ReassignedProperty { name } if name == "name"
                )),
            "expected ReassignedProperty(name) deferred error, got {:?}",
            directive.options.deferred_set_errors
        );
        assert_eq!(directive.options.set_properties.len(), 2);
        assert_eq!(
            directive.options.set_properties[0],
            ("name".to_string(), Value::String("Bob".to_string()))
        );
        assert_eq!(
            directive.options.set_properties[1],
            ("name".to_string(), Value::String("Mary".to_string()))
        );
    }

    #[test]
    fn set_invalid_with_valid_sibling_defers_invalid_and_keeps_valid() {
        let directive = parse_single(r#"::file ./foo.md set=42 set.name="Bob""#);
        assert!(directive.options.set_object.is_none());
        assert!(
            directive
                .options
                .deferred_set_errors
                .iter()
                .any(|e| matches!(e, DeferredSetError::InvalidAssignment { .. })),
        );
        assert_eq!(directive.options.set_properties.len(), 1);
        assert_eq!(
            directive.options.set_properties[0],
            ("name".to_string(), Value::String("Bob".to_string()))
        );
    }

    #[test]
    fn quoted_property_object_parsed_as_json5() {
        let directive = parse_single(r#"::file ./foo.md set.author='{name: null}'"#);
        assert_eq!(directive.options.set_properties.len(), 1);
        assert_eq!(directive.options.set_properties[0].0, "author");
        assert_eq!(
            directive.options.set_properties[0].1,
            serde_json::json!({"name": null})
        );
    }

    #[test]
    fn quoted_property_object_deep_merge_value() {
        let directive = parse_single(r#"::file ./foo.md set.a='{y:2}'"#);
        assert_eq!(directive.options.set_properties.len(), 1);
        assert_eq!(
            directive.options.set_properties[0].1,
            serde_json::json!({"y": 2})
        );
    }

    #[test]
    fn quoted_property_string_fallback_when_not_json5() {
        let directive = parse_single(r#"::file ./foo.md set.name="Bob""#);
        assert_eq!(directive.options.set_properties.len(), 1);
        assert_eq!(
            directive.options.set_properties[0].1,
            Value::String("Bob".to_string())
        );
    }

    fn matches_parse_directive(err: &TransclusionError) {
        assert!(
            matches!(err, TransclusionError::ParseDirective { .. }),
            "expected ParseDirective, got {:?}",
            err
        );
    }
}
