//! Block and frontmatter transclusion parsing.

use super::types::{
    BlockDirective, BlockOptions, DeferredSetError, DirectiveKind, FrontmatterRefs, ReplaceOption,
    TransclusionError,
};
use crate::markdown::FrontmatterMap;
use crate::markdown::compose::body_origin::DataRanges;
use crate::markdown::compose::parse_utils::{
    Cursor, authored_directive, find_code_regions, is_in_code_region, structural_view,
};
use biscuit_terminal::errors::SourceContext;
use serde_json::Value;

/// Returns `true` when `line` starts with one of the block transclusion
/// directive keywords (`::file`, `::code`, `::url`) and is **not** a
/// distinct directive such as `::file-links` that merely shares a prefix.
fn is_block_directive_line(line: &str) -> bool {
    let prefixes = [("::file", "::file-links"), ("::code", ""), ("::url", "")];
    for (prefix, exclusion) in prefixes {
        if let Some(rest) = line.strip_prefix(prefix) {
            if !exclusion.is_empty() && line.starts_with(exclusion) {
                return false;
            }
            // Require a word boundary after the prefix so `::file-foo` is
            // rejected rather than mis-parsed.
            return rest.is_empty() || rest.starts_with(' ') || rest.starts_with('\t');
        }
    }
    false
}

/// Parses block transclusion directives from markdown content.
pub fn parse_directives(
    content: &str,
    ctx: SourceContext,
) -> Result<Vec<BlockDirective>, TransclusionError> {
    parse_directives_with_line_offset(content, ctx, 0)
}

/// Parses block transclusion directives whose body-relative lines must be
/// projected into a larger source context.
pub(crate) fn parse_directives_with_line_offset(
    content: &str,
    ctx: SourceContext,
    line_offset: usize,
) -> Result<Vec<BlockDirective>, TransclusionError> {
    parse_directives_in(content, None, ctx, line_offset)
}

/// [`parse_directives_with_line_offset`] over a body whose `data` bytes were
/// inserted by an earlier stage.
///
/// A directive counts only when its line start, prefix, and keyword are
/// authored, and code regions are found in the masked view, so data can
/// neither create nor hide a directive. Data in the target is expected
/// (`::file {{ path }}`); data in the options makes the option values data
/// ([`BlockOptions::values_origin`]).
pub(crate) fn parse_directives_in(
    content: &str,
    data: Option<&DataRanges>,
    ctx: SourceContext,
    line_offset: usize,
) -> Result<Vec<BlockDirective>, TransclusionError> {
    let code_regions = find_code_regions(&structural_view(content, data));
    let mut directives = Vec::new();

    let bytes = content.as_bytes();
    let mut line_start = 0usize;
    let mut line_number = 1usize + line_offset;

    for i in 0..=bytes.len() {
        let is_eol = i == bytes.len() || bytes[i] == b'\n';
        if !is_eol {
            continue;
        }

        let line_end = i;
        let span_end = if i < bytes.len() { i + 1 } else { i };
        let line = &content[line_start..line_end];
        let trimmed = line.trim();

        if is_block_directive_line(trimmed) {
            let first_non_ws = line_start + line.len().saturating_sub(line.trim_start().len());
            let keyword_len = trimmed.find(char::is_whitespace).unwrap_or(trimmed.len());
            if !is_in_code_region(first_non_ws, &code_regions)
                && authored_directive(data, line_start, first_non_ws + keyword_len)
            {
                let (kind, raw_target, mut options) =
                    parse_directive_line(trimmed, line_number, &ctx)?;
                if let Some(data) = data {
                    let after_keyword = first_non_ws + keyword_len;
                    let options_start = content[after_keyword..line_end]
                        .find(raw_target.as_str())
                        .filter(|_| !raw_target.is_empty())
                        .map_or(after_keyword, |at| after_keyword + at + raw_target.len());
                    if data.intersects(&(options_start..line_end)) {
                        options.values_origin = crate::markdown::compose::OverrideOrigin::Data;
                    }
                }
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

        // `::code` has no `interpolate` option: there it falls through to the
        // unknown-option warning below, unvalidated.
        if key == "interpolate" && matches!(kind, DirectiveKind::File | DirectiveKind::Url) {
            parse_interpolate_option(&mut cursor, &mut options, line, ctx)?;
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

/// Parses an `interpolate=<value>` clause on `::file` or `::url`.
///
/// Each occurrence is validated as it is read, so an invalid earlier one
/// errors even when a later one is valid; among valid ones the last wins.
fn parse_interpolate_option(
    cursor: &mut Cursor<'_>,
    options: &mut BlockOptions,
    line: usize,
    ctx: &SourceContext,
) -> Result<(), TransclusionError> {
    cursor.skip_ws();
    cursor
        .expect_char('=', line)
        .map_err(|e| e.into_transclusion_error(ctx.clone()))?;
    cursor.skip_ws();

    let column = cursor.current_column();
    let was_quoted = matches!(cursor.current(), Some('\'') | Some('"'));
    let decoded = cursor
        .read_value(line)
        .map_err(|e| e.into_transclusion_error(ctx.clone()))?;

    match crate::markdown::compose::directives_api::parse_interpolate_value(&decoded, was_quoted) {
        Ok(value) => {
            options.interpolate = value.unwrap_or(true);
            Ok(())
        }
        Err(message) => Err(TransclusionError::ParseDirective {
            ctx: Box::new(ctx.clone()),
            line,
            message,
            caret_col: Some(column),
        }),
    }
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

/// Parses `raw` as a JSON5 value, returning the underlying JSON value. A
/// non-finite number (`Infinity`, `NaN`, `1e400`) is an error, never `null`.
fn parse_json5_value(raw: &str) -> Result<Value, String> {
    let json5 = biscuit_file::Json5::from_str(raw).map_err(|e| e.to_string())?;
    simplified_schema::numeric::reject_non_finite_json5(&json5)
        .map_err(|reason| format!("a number is {reason}"))?;
    Ok(json5.value().clone())
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
                let parsed: Value =
                    serde_json::from_str(value).map_err(|_| TransclusionError::ParseDirective {
                        ctx: Box::new(ctx.clone()),
                        line,
                        message: "replace option must be true/false or a JSON object".to_string(),
                        caret_col: None,
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
            } else if value.eq_ignore_ascii_case("true") {
                options.disclosure = Some(String::new());
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
    fn parses_disclosure_true_as_empty_summary() {
        let content = r#"::code ./mod.rs disclosure=true"#;
        let directives = parse_directives(content, dummy_ctx(content)).unwrap();

        assert_eq!(directives.len(), 1);
        assert_eq!(directives[0].options.disclosure, Some(String::new()));
    }

    #[test]
    fn parses_disclosure_false_as_disabled() {
        let content = r#"::code ./mod.rs disclosure=false"#;
        let directives = parse_directives(content, dummy_ctx(content)).unwrap();

        assert_eq!(directives.len(), 1);
        assert_eq!(directives[0].options.disclosure, None);
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
    fn set_property_non_finite_number_is_an_error_not_null() {
        for value in ["Infinity", "NaN", "1e400"] {
            let content = format!("::file ./foo.md set.n={value}\n");
            let error = parse_directives(&content, dummy_ctx(&content)).expect_err(value);
            assert!(error.to_string().contains("a number is not finite"), "{error}");
        }
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

    /// The local `interpolate` value `content`'s single directive parses to.
    fn interpolate_of(content: &str) -> bool {
        let directives = parse_directives(content, dummy_ctx(content))
            .unwrap_or_else(|err| panic!("{content:?} must parse: {err:?}"));
        assert_eq!(directives.len(), 1, "{content:?}");
        directives[0].options.interpolate
    }

    /// The located error `content`'s single directive line produces.
    fn interpolate_error(content: &str) -> (usize, String, Option<usize>) {
        match parse_directives(content, dummy_ctx(content)) {
            Err(TransclusionError::ParseDirective { ctx, line, message, caret_col }) => {
                assert_eq!(ctx.absolute, PathBuf::from("/test.md"), "{content:?}");
                (line, message, caret_col)
            }
            other => panic!("{content:?} must be a located directive error, got {other:?}"),
        }
    }

    #[test]
    fn interpolate_absent_is_the_default_true_and_equals_explicit_true() {
        assert!(interpolate_of("::file ./a.md\n"));
        let absent = parse_directives("::file ./a.md\n", dummy_ctx("")).unwrap();
        let explicit = parse_directives("::file ./a.md interpolate=true\n", dummy_ctx("")).unwrap();
        let null = parse_directives("::file ./a.md interpolate=null\n", dummy_ctx("")).unwrap();
        assert_eq!(absent[0].options, explicit[0].options);
        assert_eq!(absent[0].options, null[0].options);
        assert_eq!(BlockOptions::default(), absent[0].options);
    }

    #[test]
    fn interpolate_unquoted_null_is_the_default() {
        assert!(interpolate_of("::file ./a.md interpolate=null"));
        assert!(interpolate_of("::url https://example.com/a.md interpolate=null"));
    }

    #[test]
    fn interpolate_true_words_in_any_case_are_true() {
        for word in ["true", "TRUE", "yes", "Yes", "on", "ON", "1"] {
            assert!(interpolate_of(&format!("::file ./a.md interpolate={word}")), "{word}");
        }
        assert!(interpolate_of("::file ./a.md interpolate=' true '"));
    }

    #[test]
    fn interpolate_false_words_in_any_case_are_false() {
        for word in ["false", "False", "no", "NO", "off", "Off", "0"] {
            assert!(!interpolate_of(&format!("::file ./a.md interpolate={word}")), "{word}");
            assert!(!interpolate_of(&format!("::url https://example.com/a.md interpolate={word}")), "{word}");
        }
        assert!(!interpolate_of("::file ./a.md interpolate = false"));
    }

    #[test]
    fn interpolate_quoted_boolean_words_convert_like_bare_ones() {
        assert!(!interpolate_of(r#"::file ./a.md interpolate="false""#));
        assert!(!interpolate_of("::file ./a.md interpolate='No'"));
        assert!(interpolate_of(r#"::file ./a.md interpolate="yes""#));
    }

    #[test]
    fn interpolate_false_leaves_every_other_option_untouched() {
        let content = "::file ./a.md quotation=true interpolate=false when=env.CI exclude=Notes";
        let directives = parse_directives(content, dummy_ctx(content)).unwrap();
        let options = &directives[0].options;
        assert!(!options.interpolate);
        assert_eq!(options.quotation, Some(String::new()));
        assert_eq!(options.when_expr.as_deref(), Some("env.CI"));
        assert_eq!(options.exclude, vec!["Notes".to_string()]);
        assert!(options.unknown_options.is_empty());
        assert_eq!(directives[0].raw_target, "./a.md");
    }

    #[test]
    fn interpolate_quoted_null_is_text_and_a_located_error() {
        let content = r#"::file ./a.md interpolate="null""#;
        let (line, message, caret_col) = interpolate_error(content);
        assert_eq!(line, 1);
        assert!(message.contains("interpolate"), "{message}");
        assert!(message.contains("\"null\""), "{message}");
        // The caret sits on the value: `::file ./a.md interpolate=` is 26 columns.
        assert_eq!(caret_col, Some(27));
    }

    #[test]
    fn interpolate_error_reports_the_directive_line_in_the_document() {
        let content = "# Title\n\nBody.\n::file ./a.md interpolate=banana\n";
        let (line, message, caret_col) = interpolate_error(content);
        assert_eq!(line, 4);
        assert!(message.contains("banana"), "{message}");
        assert_eq!(caret_col, Some(27));
    }

    #[test]
    fn interpolate_empty_values_are_located_errors() {
        let (_, quoted, caret) = interpolate_error(r#"::file ./a.md interpolate="""#);
        assert!(quoted.contains("requires a value"), "{quoted}");
        assert_eq!(caret, Some(27));
        let (_, single, _) = interpolate_error("::file ./a.md interpolate=''");
        assert!(single.contains("requires a value"), "{single}");
        let (_, eol, caret) = interpolate_error("::file ./a.md interpolate=");
        assert!(eol.contains("Expected value"), "{eol}");
        assert_eq!(caret, Some(27));
    }

    #[test]
    fn interpolate_objects_arrays_and_other_text_are_located_errors() {
        for value in ["{a:1}", "[1]", "banana", "True-ish", "2", "nil", "NULL"] {
            let content = format!("::file ./a.md interpolate={value}");
            let (line, message, caret_col) = interpolate_error(&content);
            assert_eq!(line, 1, "{value}");
            assert!(message.contains("must be true or false"), "{value}: {message}");
            assert_eq!(caret_col, Some(27), "{value}");
        }
    }

    #[test]
    fn interpolate_repeated_last_valid_wins() {
        assert!(!interpolate_of("::file ./a.md interpolate=true interpolate=false"));
        assert!(interpolate_of("::file ./a.md interpolate=false interpolate=true"));
        assert!(interpolate_of("::file ./a.md interpolate=false interpolate=null"));
    }

    #[test]
    fn interpolate_every_repeated_occurrence_is_validated() {
        let content = "::file ./a.md interpolate=banana interpolate=false";
        let (_, message, caret_col) = interpolate_error(content);
        assert!(message.contains("banana"), "{message}");
        assert_eq!(caret_col, Some(27));
        let content = "::file ./a.md interpolate=false interpolate=banana";
        let (_, message, caret_col) = interpolate_error(content);
        assert!(message.contains("banana"), "{message}");
        assert_eq!(caret_col, Some(45));
    }

    #[test]
    fn interpolate_on_code_is_an_unknown_option_and_never_validated() {
        for value in ["false", "banana", "\"null\""] {
            let content = format!("::code ./mod.rs interpolate={value}");
            let directives = parse_directives(&content, dummy_ctx(&content))
                .unwrap_or_else(|err| panic!("{content:?} must parse: {err:?}"));
            let options = &directives[0].options;
            assert_eq!(options.unknown_options, vec!["interpolate".to_string()], "{value}");
            assert!(options.interpolate, "{value}: ::code keeps the default");
        }
    }
}
