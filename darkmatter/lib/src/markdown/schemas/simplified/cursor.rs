//! Tolerant parser state for a cursor inside a partially authored
//! `type-definition` value.
//!
//! The source-aware parsers in [`super::source`] answer "where is this parsed
//! structure in the source?" and therefore require a value that parses. An
//! editor asks the opposite question — "what am I in the middle of typing?" —
//! about text that by definition does not parse yet: an unclosed `(`, a
//! half-typed keyword, an open `{`.
//!
//! [`locate_type_definition_cursor`] answers that question from the same
//! grammar authority: it drives [`super::grammar::Lexer`] (the lexer the real
//! parser uses, so identifier, bare-word, and quoting rules cannot diverge)
//! over the text authored *before* the cursor, tracking the structural frames
//! the grammar's EBNF defines. It never searches decoded text for a delimiter.
//!
//! Positions project through the [`super::yaml_scalar`] seam, so plain,
//! single-quoted, and double-quoted scalars, CRLF input, and multibyte content
//! all report authored document ranges rather than decoded ones. Nothing here
//! reads a file, expands an import, or evaluates anything.

use std::ops::Range;

use super::grammar::{self, Lexer};
use super::source::{
    ExpressionContext, SchemaSourcePath, SchemaSourcePathSegment, quoted_flow_scalar_end,
    scan_expression, split_flow_entries,
};
use super::yaml_scalar::decode_partial_scalar_at;

/// The structural role a cursor occupies inside a partially authored value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchemaCursorRole {
    /// A type keyword, import name, or whole-definition scaffold position.
    Type,
    /// A constraint keyword inside a `(…)` constraint list.
    Constraint {
        /// The type keyword the constraint list attaches to, when one was typed.
        subject: Option<String>,
        /// Whether this is the postfix `[](…)` array-level list, whose accepted
        /// constraints are the array set rather than the item set.
        array_level: bool,
    },
    /// An argument inside a constraint call's `(…)` argument list.
    Argument {
        /// The type keyword the enclosing constraint list attaches to.
        subject: Option<String>,
        /// The constraint keyword whose arguments enclose the cursor.
        constraint: String,
        /// Whether the enclosing constraint list is array-level.
        array_level: bool,
    },
    /// A property key inside an inline object literal.
    InlineObjectKey,
    /// The file-reference half of a `Name@reference` import.
    ImportReference {
        /// The named type authored to the left of `@`.
        name: String,
    },
}

/// A tolerant reading of the structural state at a cursor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaCursor {
    /// Structural path shared with [`super::SchemaSourceMap`]: the union arms
    /// and inline-object properties enclosing the cursor, outermost first.
    pub path: SchemaSourcePath,
    /// What the cursor is positioned to author.
    pub role: SchemaCursorRole,
    /// The token text already authored at the cursor, decoded. Empty when the
    /// cursor sits at a fresh position.
    pub token: String,
    /// The authored document byte range `token` occupies. Empty and
    /// zero-width at a fresh position, so a completion text edit inserts
    /// rather than replaces.
    pub token_span: Range<usize>,
}

/// Locates the structural authoring state of a cursor inside a partially
/// authored `type-definition` value.
///
/// `value_source` is the authored YAML value text exactly as typed (never
/// line-ending-normalized), `value_offset` its byte offset in the caller's
/// document, and `cursor` a document byte offset at or inside the value.
///
/// ## Examples
///
/// ```
/// use darkmatter::markdown::schemas::{
///     SchemaCursorRole, locate_type_definition_cursor,
/// };
///
/// // The innermost `(` belongs to `pattern`, but the cursor is back in the
/// // outer constraint list after `;`.
/// let value = "string(pattern(^a); re";
/// let state = locate_type_definition_cursor(value, 0, value.len()).unwrap();
/// assert_eq!(state.token, "re");
/// assert!(matches!(
///     state.role,
///     SchemaCursorRole::Constraint { array_level: false, .. }
/// ));
/// ```
///
/// ## Returns
///
/// `None` when `cursor` is outside `value_source`, does not land on a UTF-8
/// character boundary, or falls inside a `-> description`, where the grammar
/// is prose rather than structure.
pub fn locate_type_definition_cursor(
    value_source: &str,
    value_offset: usize,
    cursor: usize,
) -> Option<SchemaCursor> {
    let relative = cursor.checked_sub(value_offset)?;
    if relative > value_source.len() || !value_source.is_char_boundary(relative) {
        return None;
    }
    locate_in(&value_source[..relative], value_offset, SchemaSourcePath::root(), false)
}

/// Locates the structural authoring state of a cursor inside a partially
/// authored `schema` declaration value.
///
/// A declaration arm is a file reference or a whole-declaration scaffold rather
/// than a type expression, so the arm's complete authored text is the token.
/// A flow sequence is a union, as in [`locate_type_definition_cursor`], so
/// `[./a.yaml, ./b` reports arm 1, but its arms split by YAML's entry rule:
/// a parenthesis is part of the reference, so `[./a(b.yaml, ./b` also
/// reports arm 1.
///
/// ## Returns
///
/// `None` under the same conditions as [`locate_type_definition_cursor`].
pub fn locate_schema_declaration_cursor(
    value_source: &str,
    value_offset: usize,
    cursor: usize,
) -> Option<SchemaCursor> {
    let relative = cursor.checked_sub(value_offset)?;
    if relative > value_source.len() || !value_source.is_char_boundary(relative) {
        return None;
    }
    locate_in(&value_source[..relative], value_offset, SchemaSourcePath::root(), true)
}

/// Walks the YAML shape layer (a flow sequence is a union) and then reads the
/// enclosing scalar, either as a type expression or as a whole declaration arm.
fn locate_in(
    prefix: &str,
    offset: usize,
    path: SchemaSourcePath,
    declaration: bool,
) -> Option<SchemaCursor> {
    let lead = prefix.len() - prefix.trim_start().len();
    if prefix[lead..].starts_with('[') {
        let body_start = lead + 1;
        let body = &prefix[body_start..];
        let starts = if declaration {
            yaml_flow_arm_starts(body)
        } else {
            expression_flow_arm_starts(body)
        };
        let (arm, arm_start) = flow_union_arm(body, &starts);
        return locate_in(
            arm,
            offset + body_start + arm_start,
            path.union_arm(starts.len()),
            declaration,
        );
    }
    if declaration {
        return whole_arm(prefix, offset, path);
    }
    scan_scalar(prefix, offset, path)
}

/// Reads a declaration arm as one opaque authored token.
fn whole_arm(prefix: &str, offset: usize, path: SchemaSourcePath) -> Option<SchemaCursor> {
    let (scalar, _) = decode_partial_scalar_at(prefix, 0);
    let decoded = scalar.decoded();
    let span = scalar.project(0..decoded.len())?;
    Some(SchemaCursor {
        path,
        role: SchemaCursorRole::Type,
        token: decoded.to_string(),
        token_span: shift(span, offset),
    })
}

/// The trailing arm of a partially authored flow sequence whose arms begin at
/// `starts` (after the first), plus its byte offset within `body`.
fn flow_union_arm<'a>(body: &'a str, starts: &[usize]) -> (&'a str, usize) {
    let start = starts.last().copied().unwrap_or(0);
    let arm = &body[start..];
    let lead = arm.len() - arm.trim_start().len();
    (&arm[lead..], start + lead)
}

/// The byte offset just past each top-level `,` in `body`, the raw YAML text
/// after a declaration's flow sequence `[`. Arms are opaque file references,
/// so YAML's own entry rule applies: only `[`/`{` nest, and a parenthesis is
/// plain-scalar content (`[./a(b.yaml, ./b` has two arms).
fn yaml_flow_arm_starts(body: &str) -> Vec<usize> {
    split_flow_entries(body, 0..body.len())
        .iter()
        .skip(1)
        .map(|entry| entry.start)
        .collect()
}

/// The byte offset just past each top-level `,` in `body`, the text after a
/// type definition's flow sequence `[`.
///
/// Each arm is read in two layers. A quote at the arm's first character opens
/// a YAML quoted scalar, which is skipped whole, so `['enum("a)b", c)', str`
/// has two arms. The rest of a plain arm is type-expression text read by
/// [`scan_expression`] in the grammar's own lexical modes: the `,` inside
/// `enum(a, b)` or a quoted argument (`enum('a)b', c)`) stays in its argument
/// list; a file reference (`Name@./a(b.yaml`) is opaque up to its `,`; and a
/// top-level description is prose up to YAML's next `,` or `]`, so
/// `[string -> (it's fine), s` has two arms. `[` nests outside those modes.
/// Scanning stops inside an unterminated string or quoted scalar, where the
/// cursor still is.
fn expression_flow_arm_starts(body: &str) -> Vec<usize> {
    let bytes = body.as_bytes();
    let mut starts = Vec::new();
    let mut arm = 0;
    loop {
        let mut from = arm + body[arm..].len() - body[arm..].trim_start().len();
        if matches!(bytes.get(from), Some(b'\'' | b'"')) {
            let Some(end) = quoted_flow_scalar_end(bytes, from, body.len()) else {
                return starts;
            };
            from = end;
        }
        let mut nesting = 0usize;
        let mut next = None;
        scan_expression(body, from..body.len(), ExpressionContext::FlowArm, |index, byte, level| {
            if !level.is_top() {
                return false;
            }
            match byte {
                b'[' => nesting += 1,
                b']' => nesting = nesting.saturating_sub(1),
                b',' if nesting == 0 => {
                    next = Some(index + 1);
                    return true;
                }
                _ => {}
            }
            false
        });
        let Some(next) = next else {
            return starts;
        };
        starts.push(next);
        arm = next;
    }
}

/// One enclosing structural frame of the type-expression grammar.
#[derive(Debug, Clone)]
enum Frame {
    /// Inside `{ … }`. `key` is the property being defined once `:` is typed.
    InlineObject { key: Option<String>, in_value: bool },
    /// Inside a `(…)` constraint list attached to `subject`.
    ConstraintList { subject: Option<String>, array_level: bool },
    /// Inside a constraint call's `(…)` argument list.
    ArgList {
        subject: Option<String>,
        constraint: String,
        array_level: bool,
    },
}

/// Drives the grammar lexer across the authored prefix of one scalar and
/// reports the frame stack and trailing token at its end.
///
/// Only type text and argument lists are lexed. Inside a constraint or
/// argument list, `{`, `[`, `@`, `:`, and `<` are argument text. An imported
/// file reference, an inline-object description, and a pattern key are each
/// skipped to the end the grammar gives them, so their punctuation never opens
/// a frame; the cursor resumes in type context after a description's `,` and
/// keeps the import-reference role while it is still inside a reference.
fn scan_scalar(prefix: &str, offset: usize, path: SchemaSourcePath) -> Option<SchemaCursor> {
    let (scalar, _) = decode_partial_scalar_at(prefix, 0);
    let src = scalar.decoded().to_string();
    let bytes = src.as_bytes();
    let mut lex = Lexer::new(&src);
    let mut stack: Vec<Frame> = Vec::new();
    // The type keyword at the current type position, and whether a `[]` has
    // been typed since it — together they decide whether the *next* `(` opens
    // the item-level or the array-level constraint list.
    let mut subject: Option<String> = None;
    let mut saw_array = false;
    let mut last_word: Option<String> = None;
    let mut import_name: Option<String> = None;
    let mut token: Option<(String, Range<usize>)> = None;

    loop {
        lex.skip_ws();
        let start = lex.pos;
        let Some(byte) = lex.peek_byte() else { break };
        let in_list = matches!(
            stack.last(),
            Some(Frame::ArgList { .. } | Frame::ConstraintList { .. })
        );
        let arrow = byte == b'-' && bytes.get(start + 1) == Some(&b'>');
        let structural = if in_list {
            matches!(byte, b'(' | b')' | b',' | b';' | b'\'' | b'"')
        } else {
            matches!(byte, b'{' | b'}' | b'(' | b')' | b'[' | b']' | b',' | b':' | b'@' | b'\'' | b'"')
                || arrow
                || (byte == b'<'
                    && matches!(stack.last(), Some(Frame::InlineObject { in_value: false, .. })))
        };
        if !structural {
            let (word, span) = if matches!(stack.last(), Some(Frame::ArgList { .. })) {
                lex.read_word()
            } else {
                lex.read_ident()
            };
            if word.is_empty() {
                // An unrecognized character: consume it whole (never a
                // partial UTF-8 sequence) and keep scanning, so a stray
                // byte never aborts the reading.
                lex.pos += src[start..].chars().next().map_or(1, char::len_utf8);
                token = None;
                continue;
            }
            match stack.last_mut() {
                Some(Frame::ArgList { .. } | Frame::ConstraintList { .. }) => {}
                Some(Frame::InlineObject { in_value: false, .. }) => {}
                _ => {
                    subject = Some(word.clone());
                    saw_array = false;
                }
            }
            last_word = Some(word.clone());
            token = Some((word, span));
            continue;
        }
        match byte {
            b'{' => {
                lex.pos += 1;
                stack.push(Frame::InlineObject { key: None, in_value: false });
                subject = None;
                saw_array = false;
                token = None;
            }
            b'}' => {
                lex.pos += 1;
                while let Some(frame) = stack.pop() {
                    if matches!(frame, Frame::InlineObject { .. }) {
                        break;
                    }
                }
                subject = None;
                token = None;
            }
            b'(' => {
                lex.pos += 1;
                let frame = match stack.last() {
                    Some(Frame::ConstraintList { subject, array_level }) => Frame::ArgList {
                        subject: subject.clone(),
                        constraint: last_word.clone().unwrap_or_default(),
                        array_level: *array_level,
                    },
                    // `enum` and `literal` take positional values in their
                    // item-level list, which `parse_constraint_list` lexes in
                    // argument mode rather than as constraint keywords.
                    _ if !saw_array
                        && matches!(subject.as_deref(), Some("enum" | "literal")) =>
                    {
                        Frame::ArgList {
                            subject: subject.clone(),
                            constraint: subject.clone().unwrap_or_default(),
                            array_level: false,
                        }
                    }
                    _ => Frame::ConstraintList {
                        subject: subject.clone(),
                        array_level: saw_array,
                    },
                };
                stack.push(frame);
                token = None;
            }
            b')' => {
                lex.pos += 1;
                stack.pop();
                token = None;
            }
            b'[' | b']' => {
                lex.pos += 1;
                saw_array = true;
                token = None;
            }
            b',' => {
                lex.pos += 1;
                if let Some(Frame::InlineObject { key, in_value }) = stack.last_mut() {
                    *key = None;
                    *in_value = false;
                    subject = None;
                    saw_array = false;
                }
                token = None;
            }
            b';' => {
                lex.pos += 1;
                token = None;
            }
            b':' => {
                lex.pos += 1;
                if let Some(Frame::InlineObject { key, in_value }) = stack.last_mut() {
                    *key = last_word.clone();
                    *in_value = true;
                }
                subject = None;
                saw_array = false;
                token = None;
            }
            b'@' => {
                let reference = start + 1;
                let end = grammar::file_reference_end(bytes, reference, src.len());
                lex.pos = end;
                token = None;
                if end == src.len() {
                    // The cursor is inside the reference, which is one token
                    // from its first non-blank byte however it is spelled.
                    let lead = reference + src[reference..].len() - src[reference..].trim_start().len();
                    import_name = subject.clone().or_else(|| last_word.clone());
                    token = Some((src[lead..].to_string(), lead..src.len()));
                }
            }
            b'<' => match grammar::pattern_key_end(bytes, start, src.len()) {
                Some(end) => {
                    lex.pos = end;
                    last_word = Some(src[start..end].to_string());
                    token = Some((src[start..end].to_string(), start..end));
                }
                None => {
                    lex.pos = src.len();
                    token = Some((src[start..].to_string(), start..src.len()));
                }
            },
            b'\'' | b'"' => {
                let (text, span) = read_quoted_prefix(&src, &mut lex, byte);
                token = Some((text, span));
            }
            _ => {
                // `->` opens a human description. At the top level it runs to
                // the end of the value; inside an inline object it ends at the
                // object's next `,` or `}`. Either way, a cursor inside it is
                // in prose rather than grammar the cursor API can speak to.
                if !matches!(stack.last(), Some(Frame::InlineObject { .. })) {
                    return None;
                }
                match grammar::inline_description_end(bytes, start + 2, src.len()) {
                    Ok(end) if end < src.len() => {
                        lex.pos = end;
                        token = None;
                    }
                    _ => return None,
                }
            }
        }
    }

    // A token that does not run up to the cursor was followed by whitespace, so
    // the cursor is at a fresh position rather than inside that token.
    let (text, decoded_span) = match token.filter(|(_, span)| span.end == src.len()) {
        Some(found) => found,
        None => (String::new(), src.len()..src.len()),
    };
    let token_span = scalar.project(decoded_span)?;
    let mut path = path;
    for frame in &stack {
        if let Frame::InlineObject { key: Some(key), in_value: true } = frame {
            path = path.property(key);
        }
    }
    let role = match stack.last() {
        Some(Frame::ArgList { subject, constraint, array_level }) => SchemaCursorRole::Argument {
            subject: subject.clone(),
            constraint: constraint.clone(),
            array_level: *array_level,
        },
        Some(Frame::ConstraintList { subject, array_level }) => SchemaCursorRole::Constraint {
            subject: subject.clone(),
            array_level: *array_level,
        },
        Some(Frame::InlineObject { in_value: false, .. }) => SchemaCursorRole::InlineObjectKey,
        Some(Frame::InlineObject { .. }) | None => match import_name {
            Some(name) => SchemaCursorRole::ImportReference { name },
            None => SchemaCursorRole::Type,
        },
    };

    Some(SchemaCursor {
        path,
        role,
        token: text,
        token_span: shift(token_span, offset),
    })
}

/// Consumes a quoted run that may not be closed yet, returning its authored
/// contents and their span. As in the lexer, `\` escapes the next character in
/// either quote style, so `'a\', b` is still one open string.
fn read_quoted_prefix(src: &str, lex: &mut Lexer<'_>, quote: u8) -> (String, Range<usize>) {
    let start = lex.pos + 1;
    lex.pos += 1;
    while let Some(byte) = lex.peek_byte() {
        if byte == quote {
            let end = lex.pos;
            lex.pos += 1;
            return (src[start..end].to_string(), start..end);
        }
        if byte == b'\\' {
            lex.pos += 1;
            if lex.pos >= src.len() {
                break;
            }
        }
        lex.pos += src[lex.pos..].chars().next().map_or(1, char::len_utf8);
    }
    (src[start..].to_string(), start..src.len())
}

fn shift(span: Range<usize>, offset: usize) -> Range<usize> {
    offset + span.start..offset + span.end
}

/// Whether `path` addresses a union arm at its deepest segment.
///
/// Callers that merge sibling-arm candidates use this to tell an arm position
/// apart from a whole-property position.
pub fn is_union_arm_path(path: &SchemaSourcePath) -> bool {
    matches!(path.segments().last(), Some(SchemaSourcePathSegment::UnionArm(_)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at_end(value: &str) -> SchemaCursor {
        locate_type_definition_cursor(value, 0, value.len()).expect("cursor state")
    }

    #[test]
    fn second_constraint_after_a_nested_paren_constraint() {
        let state = at_end("string(pattern(^a); re");
        assert_eq!(state.token, "re");
        assert_eq!(
            state.role,
            SchemaCursorRole::Constraint {
                subject: Some("string".into()),
                array_level: false,
            }
        );
        assert_eq!(state.token_span, 20..22);
    }

    #[test]
    fn postfix_array_constraint_list_is_array_level() {
        let state = at_end("type-definition[](mi");
        assert_eq!(state.token, "mi");
        assert_eq!(
            state.role,
            SchemaCursorRole::Constraint {
                subject: Some("type-definition".into()),
                array_level: true,
            }
        );
    }

    #[test]
    fn item_constraints_before_the_array_suffix_stay_item_level() {
        let state = at_end("string(min(1))[](un");
        assert_eq!(
            state.role,
            SchemaCursorRole::Constraint {
                subject: Some("string".into()),
                array_level: true,
            }
        );
        let item = at_end("string(mi");
        assert_eq!(
            item.role,
            SchemaCursorRole::Constraint {
                subject: Some("string".into()),
                array_level: false,
            }
        );
    }

    #[test]
    fn partially_authored_inline_object_reports_key_then_value_positions() {
        let key = at_end("{ chi");
        assert_eq!(key.role, SchemaCursorRole::InlineObjectKey);
        assert_eq!(key.token, "chi");

        let value = at_end("{ child: str");
        assert_eq!(value.role, SchemaCursorRole::Type);
        assert_eq!(value.token, "str");
        assert_eq!(value.path, SchemaSourcePath::root().property("child"));
    }

    #[test]
    fn nested_inline_objects_nest_the_structural_path() {
        let state = at_end("{ outer: { inner: str");
        assert_eq!(state.role, SchemaCursorRole::Type);
        assert_eq!(
            state.path,
            SchemaSourcePath::root().property("outer").property("inner")
        );
    }

    #[test]
    fn a_flow_union_arm_carries_its_index() {
        let state = at_end("[string, num");
        assert_eq!(state.token, "num");
        assert_eq!(state.role, SchemaCursorRole::Type);
        assert_eq!(state.path, SchemaSourcePath::root().union_arm(1));
        assert!(is_union_arm_path(&state.path));
    }

    #[test]
    fn a_comma_inside_a_constraint_does_not_open_a_union_arm() {
        let state = at_end("[enum(a, b), str");
        assert_eq!(state.token, "str");
        assert_eq!(state.path, SchemaSourcePath::root().union_arm(1));
    }

    #[test]
    fn a_quote_inside_a_plain_declaration_arm_does_not_hide_the_next_arm() {
        for value in ["[./don't.yaml, ./b", "[./say \"hi.yaml, ./b", "['./it''s.yaml', ./b"] {
            let state = locate_schema_declaration_cursor(value, 0, value.len()).unwrap();
            assert_eq!(state.token, "./b", "{value:?}");
            assert_eq!(state.path, SchemaSourcePath::root().union_arm(1), "{value:?}");
        }
    }

    /// A quote after a content colon (`a:'b`) is plain-scalar content, so it
    /// neither merges arms nor hides the separator before the final arm.
    #[test]
    fn a_quote_after_a_content_colon_does_not_hide_the_next_arm() {
        let rows: &[(&str, usize)] = &[
            ("[ordinary, ", 1),
            ("['a:''b', ", 1),
            ("[\"a:b\", ", 1),
            ("[a:'b, ", 1),
            ("[a:\"b, ", 1),
            ("[a:'b, c:'d, ", 2),
            ("[a:\"b, c:\"d, ", 2),
            ("[{k: a:'b}, ", 1),
            ("[{a:'b: c}, ", 1),
        ];
        let offset = 7;
        for (before, arm) in rows {
            for (token, declaration) in [("./b", true), ("str", false)] {
                let value = format!("{before}{token}");
                let end = offset + value.len();
                let state = if declaration {
                    locate_schema_declaration_cursor(&value, offset, end)
                } else {
                    locate_type_definition_cursor(&value, offset, end)
                }
                .unwrap_or_else(|| panic!("{value:?}"));
                assert_eq!(state.path, SchemaSourcePath::root().union_arm(*arm), "{value:?}");
                assert_eq!(state.role, SchemaCursorRole::Type, "{value:?}");
                assert_eq!(state.token, token, "{value:?}");
                assert_eq!(state.token_span, end - token.len()..end, "{value:?}");
            }
        }
    }

    /// A declaration arm is an opaque YAML scalar, so a parenthesis in it is
    /// content and never hides the `,` before the next arm, while a type
    /// expression keeps `(…)` nesting for constraints such as `enum(a, b)`.
    #[test]
    fn declaration_arms_split_at_commas_between_parentheses() {
        let offset = 5;
        let declarations: &[(&str, usize)] = &[
            ("[./a(b.yaml, ", 1),
            ("[./a(b.yaml, ./c)d.yaml, ", 2),
            ("[./a)b.yaml, ", 1),
            ("['./a(b.yaml', ", 1),
            ("[\"./a(b, c)d.yaml\", ", 1),
            ("[{k: a(b}, ", 1),
            ("[./a.yaml, ", 1),
        ];
        for (before, arm) in declarations {
            let value = format!("{before}./b");
            let end = offset + value.len();
            let state = locate_schema_declaration_cursor(&value, offset, end)
                .unwrap_or_else(|| panic!("{value:?}"));
            assert_eq!(state.path, SchemaSourcePath::root().union_arm(*arm), "{value:?}");
            assert_eq!(state.role, SchemaCursorRole::Type, "{value:?}");
            assert_eq!(state.token, "./b", "{value:?}");
            assert_eq!(state.token_span, end - 3..end, "{value:?}");
        }

        for value in ["[enum(a, b), str", "[string(suggest(a, b)), str"] {
            let end = offset + value.len();
            let state = locate_type_definition_cursor(value, offset, end).unwrap();
            assert_eq!(state.path, SchemaSourcePath::root().union_arm(1), "{value:?}");
            assert_eq!(state.role, SchemaCursorRole::Type, "{value:?}");
            assert_eq!(state.token, "str", "{value:?}");
            assert_eq!(state.token_span, end - 3..end, "{value:?}");
        }
    }

    /// A quote inside a constraint's parentheses opens an expression string, so
    /// punctuation in a quoted argument neither closes the constraint nor
    /// separates alternatives. A quote starting an arm is YAML quoting, and one
    /// outside parentheses is content.
    #[test]
    fn a_quoted_argument_keeps_its_punctuation_inside_its_flow_arm() {
        let rows: &[(&str, usize)] = &[
            // Controls.
            ("[enum(a, b), ", 1),
            ("['enum(\"a)b\", c)', ", 1),
            ("[\"enum('a]b', c)\", ", 1),
            ("['it''s', ", 1),
            ("[a:'b, ", 1),
            ("[string -> it's, ", 1),
            // Closing punctuation inside a quoted argument.
            ("[enum('a)b', c), ", 1),
            ("[enum('a]b', c), ", 1),
            ("[enum('a}b', c), ", 1),
            ("[enum(\"a)b\", c), ", 1),
            ("[enum(\"a]b\", c), ", 1),
            ("[enum(\"a}b\", c), ", 1),
            // Opening punctuation inside a quoted argument.
            ("[enum('a(b', c), ", 1),
            ("[enum('a[b', c), ", 1),
            ("[enum('a{b', c), ", 1),
            ("[enum(\"a(b\", c), ", 1),
            ("[enum(\"a[b\", c), ", 1),
            ("[enum(\"a{b\", c), ", 1),
            // Separators, escapes, and nesting.
            ("[enum('a,b', c), ", 1),
            ("[enum('a\\')b', c), ", 1),
            ("[enum(\"a\\\")b\", c), ", 1),
            ("[string(suggest('x)y', 'p(q')), ", 1),
            ("[{ k: string(suggest('a}b')) }, ", 1),
            ("[enum('a)b', c), enum(\"d(e\", f), ", 2),
            ("['enum(\"a)b\", c)', enum('d(e', f), ", 2),
        ];
        let offset = 9;
        for (before, arm) in rows {
            let value = format!("{before}str");
            let end = offset + value.len();
            let state = locate_type_definition_cursor(&value, offset, end)
                .unwrap_or_else(|| panic!("{value:?}"));
            assert_eq!(state.path, SchemaSourcePath::root().union_arm(*arm), "{value:?}");
            assert_eq!(state.role, SchemaCursorRole::Type, "{value:?}");
            assert_eq!(state.token, "str", "{value:?}");
            assert_eq!(state.token_span, end - 3..end, "{value:?}");
        }
    }

    /// While an argument list is still open, the `,` after a quoted argument
    /// separates enum members rather than alternatives, in a flow union and in
    /// a plain scalar alike.
    #[test]
    fn a_comma_after_a_quoted_argument_stays_in_the_argument_list() {
        let enum_argument = SchemaCursorRole::Argument {
            subject: Some("enum".into()),
            constraint: "enum".into(),
            array_level: false,
        };
        let root = SchemaSourcePath::root();
        let rows: &[(&str, SchemaSourcePath, &str)] = &[
            ("[enum(a, c", root.union_arm(0), "c"),
            ("[enum('a)b', c", root.union_arm(0), "c"),
            ("[enum('a]b', c", root.union_arm(0), "c"),
            ("[enum('a}b', c", root.union_arm(0), "c"),
            ("[enum(\"a)b\", c", root.union_arm(0), "c"),
            ("[enum(\"a]b\", c", root.union_arm(0), "c"),
            ("[enum('a(b', c", root.union_arm(0), "c"),
            ("[enum('a[b', c", root.union_arm(0), "c"),
            ("[enum(\"a(b\", c", root.union_arm(0), "c"),
            ("[enum('a\\')b', c", root.union_arm(0), "c"),
            ("[str, enum('a)b', c", root.union_arm(1), "c"),
            ("['enum(\"a)b\", c)', enum('d)e', c", root.union_arm(1), "c"),
            ("enum('a)b', c", root.clone(), "c"),
            ("enum('a(b', c", root.clone(), "c"),
            ("enum('a]b', c", root.clone(), "c"),
            ("enum(\"a)b\", c", root.clone(), "c"),
            // The cursor is still inside an open string.
            ("[enum('a, c", root.union_arm(0), "a, c"),
            ("enum('a\\', c", root.clone(), "a\\', c"),
        ];
        let offset = 4;
        for (value, path, token) in rows {
            let end = offset + value.len();
            let state = locate_type_definition_cursor(value, offset, end)
                .unwrap_or_else(|| panic!("{value:?}"));
            assert_eq!(&state.path, path, "{value:?}");
            assert_eq!(state.role, enum_argument, "{value:?}");
            assert_eq!(state.token, *token, "{value:?}");
            assert_eq!(state.token_span, end - token.len()..end, "{value:?}");
        }
    }

    #[test]
    fn constraint_arguments_report_their_constraint() {
        let state = at_end("url(scheme(htt");
        assert_eq!(
            state.role,
            SchemaCursorRole::Argument {
                subject: Some("url".into()),
                constraint: "scheme".into(),
                array_level: false,
            }
        );
        assert_eq!(state.token, "htt");
    }

    #[test]
    fn an_import_reference_is_distinguished_from_a_type_keyword() {
        let state = at_end("Post@./typ");
        assert_eq!(
            state.role,
            SchemaCursorRole::ImportReference { name: "Post".into() }
        );
        assert_eq!(state.token, "./typ");
    }

    #[test]
    fn a_fresh_position_reports_an_empty_zero_width_token() {
        for value in ["", "string(", "string(min(1); ", "{ "] {
            let state = at_end(value);
            assert!(state.token.is_empty(), "{value:?} -> {state:?}");
            assert!(state.token_span.is_empty(), "{value:?} -> {state:?}");
            assert_eq!(state.token_span.start, value.len(), "{value:?}");
        }
    }

    #[test]
    fn a_hard_parse_failure_still_yields_a_usable_state() {
        // Every one of these is rejected outright by `parse_type_expr`.
        for value in ["strin", "))(", "string(((", "{ a: [b"] {
            assert!(super::super::grammar::parse_type_expr("p", value).is_err(), "{value:?}");
            assert!(locate_type_definition_cursor(value, 0, value.len()).is_some(), "{value:?}");
        }
    }

    #[test]
    fn quoted_scalars_project_spans_back_through_their_quoting() {
        // Single-quoted, double-quoted, and plain all name the same token.
        for (value, expected) in [
            ("'string(mi", "mi"),
            ("\"string(mi", "mi"),
            ("string(mi", "mi"),
        ] {
            let state = at_end(value);
            assert_eq!(state.token, expected, "{value:?}");
            assert_eq!(&value[state.token_span.clone()], expected, "{value:?}");
        }
    }

    #[test]
    fn double_quoted_escapes_project_to_authored_bytes() {
        let value = "\"enum(caf\\u00e9, be";
        let state = at_end(value);
        assert_eq!(state.token, "be");
        assert_eq!(&value[state.token_span.clone()], "be");
    }

    #[test]
    fn multibyte_content_projects_by_byte() {
        let value = "enum(café, be";
        let state = at_end(value);
        assert_eq!(state.token, "be");
        assert_eq!(&value[state.token_span.clone()], "be");
    }

    #[test]
    fn a_document_offset_shifts_every_reported_span() {
        let value = "string(mi";
        let state = locate_type_definition_cursor(value, 100, 100 + value.len()).unwrap();
        assert_eq!(state.token_span, 107..109);
    }

    #[test]
    fn a_cursor_before_the_end_reads_only_what_precedes_it() {
        let value = "string(min(1); unique)";
        let cursor = value.find("min").unwrap() + 2;
        let state = locate_type_definition_cursor(value, 0, cursor).unwrap();
        assert_eq!(state.token, "mi");
        assert_eq!(
            state.role,
            SchemaCursorRole::Constraint {
                subject: Some("string".into()),
                array_level: false,
            }
        );
    }

    #[test]
    fn a_cursor_outside_the_value_or_mid_character_is_rejected() {
        assert!(locate_type_definition_cursor("string", 10, 5).is_none());
        assert!(locate_type_definition_cursor("string", 0, 7).is_none());
        assert!(locate_type_definition_cursor("café", 0, 4).is_none());
    }

    /// A description and a file reference are read in the grammar's own
    /// modes, so their punctuation neither hides the `,` before the next
    /// alternative nor adds one.
    #[test]
    fn descriptions_and_file_references_keep_the_next_alternative() {
        let rows = [
            // Controls.
            "[string -> plain, ",
            "[string -> (plain), ",
            "[string -> it's fine, ",
            "[Name@./a(b)c.yaml, ",
            "[Name@./a)b.yaml, ",
            "['string -> (it''s fine)', ",
            "[string(suggest('a)b', c)), ",
            // Prose punctuation after `->`.
            "[string -> (it's fine), ",
            "[string -> (say \"hi), ",
            "[{ a: string -> plain ] here }, ",
            "[{ a: string -> (it's fine), b: number }, ",
            // Opening punctuation in an imported filename.
            "[Name@./a(b.yaml, ",
            "[Name@./a[b.yaml, ",
            "[Name@./a{b.yaml, ",
            "[Name@./a('b.yaml, ",
            "[Name@./a(\"b.yaml, ",
            "[{ r: Name@./a(b.yaml }, ",
            "[{ r: Name@./a{b.yaml -> (it's), s: number }, ",
        ];
        let offset = 11;
        for before in rows {
            let value = format!("{before}s");
            let end = offset + value.len();
            let state = locate_type_definition_cursor(&value, offset, end)
                .unwrap_or_else(|| panic!("{value:?}"));
            assert_eq!(state.path, SchemaSourcePath::root().union_arm(1), "{value:?}");
            assert_eq!(state.role, SchemaCursorRole::Type, "{value:?}");
            assert_eq!(state.token, "s", "{value:?}");
            assert_eq!(state.token_span, end - 1..end, "{value:?}");
        }
    }

    /// Inside one scalar, the cursor resumes type context after an
    /// inline-object description ends, keeps the import-reference role across
    /// filename punctuation, and reads a pattern key as one key. Each row is
    /// authored plain, single-quoted, and double-quoted.
    #[test]
    fn a_scalar_cursor_follows_description_reference_and_key_modes() {
        let root = SchemaSourcePath::root();
        let import = |name: &str| SchemaCursorRole::ImportReference { name: name.into() };
        let rows: Vec<(&str, SchemaSourcePath, SchemaCursorRole, &str)> = vec![
            ("Name@./a(b.yaml", root.clone(), import("Name"), "./a(b.yaml"),
            ("Name@./a[b.yaml", root.clone(), import("Name"), "./a[b.yaml"),
            ("Name@./a{b.yaml", root.clone(), import("Name"), "./a{b.yaml"),
            ("Name@./a('b", root.clone(), import("Name"), "./a('b"),
            ("Name@./a(\"b", root.clone(), import("Name"), "./a(\"b"),
            ("Name(required)@./a(b", root.clone(), import("Name"), "./a(b"),
            ("{ r: Name@./a{b.yaml", root.property("r"), import("Name"), "./a{b.yaml"),
            ("{ a: string -> plain, b: str", root.property("b"), SchemaCursorRole::Type, "str"),
            ("{ a: string -> (it's fine), b: str", root.property("b"), SchemaCursorRole::Type, "str"),
            ("{ a: string -> (say \"hi), b: str", root.property("b"), SchemaCursorRole::Type, "str"),
            ("{ a: string -> ({x} it's [fine), b: str", root.property("b"), SchemaCursorRole::Type, "str"),
            ("{ o: { a: string -> x } -> (it's), b: str", root.property("b"), SchemaCursorRole::Type, "str"),
            ("{ o: { a: string -> x, b: str", root.property("o").property("b"), SchemaCursorRole::Type, "str"),
            ("{ r: Name@./a(b.yaml -> it's, b: str", root.property("b"), SchemaCursorRole::Type, "str"),
            ("{ r: Name@./a(b.yaml, b: str", root.property("b"), SchemaCursorRole::Type, "str"),
            ("{ <pattern::^(a,b):c$>: str", root.property("<pattern::^(a,b):c$>"), SchemaCursorRole::Type, "str"),
            ("{ <pattern::^(a", root.clone(), SchemaCursorRole::InlineObjectKey, "<pattern::^(a"),
            (
                "string(pattern(^[{]@x:y$); re",
                root.clone(),
                SchemaCursorRole::Constraint { subject: Some("string".into()), array_level: false },
                "re",
            ),
        ];
        let offset = 6;
        for (decoded, path, role, token) in rows {
            for quote in ["", "'", "\""] {
                let escape = |text: &str| match quote {
                    "'" => text.replace('\'', "''"),
                    "\"" => text.replace('"', "\\\""),
                    _ => text.to_string(),
                };
                let value = format!("{quote}{}", escape(decoded));
                let end = offset + value.len();
                let state = locate_type_definition_cursor(&value, offset, end)
                    .unwrap_or_else(|| panic!("{value:?}"));
                assert_eq!(state.path, path, "{value:?}");
                assert_eq!(state.role, role, "{value:?}");
                assert_eq!(state.token, token, "{value:?}");
                let authored = escape(token);
                assert_eq!(state.token_span, end - authored.len()..end, "{value:?}");
            }
        }
    }

    #[test]
    fn a_cursor_inside_an_inline_object_description_is_in_prose() {
        for value in ["{ a: string -> (it's", "{ a: string -> x, b: string -> y", "{ a: string -> (x, y"] {
            assert!(locate_type_definition_cursor(value, 0, value.len()).is_none(), "{value:?}");
        }
    }

    #[test]
    fn a_description_is_prose_rather_than_grammar() {
        assert!(locate_type_definition_cursor("string -> a note", 0, 16).is_none());
    }
}
