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

use super::grammar::Lexer;
use super::source::{SchemaSourcePath, SchemaSourcePathSegment};
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
/// The root-union layer is shared with
/// [`locate_type_definition_cursor`], so `[./a.yaml, ./b` reports arm 1.
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
        let (arm, arm_start) = flow_union_arm(&prefix[body_start..]);
        return locate_in(
            arm,
            offset + body_start + arm_start,
            path.union_arm(flow_arm_index(&prefix[body_start..])),
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

/// The trailing arm of a partially authored flow sequence, plus its byte offset
/// within `body`. Splitting is depth- and quote-aware, so a `,` inside an
/// `enum(a, b)` constraint or a `{ … }` literal never starts a new arm.
fn flow_union_arm(body: &str) -> (&str, usize) {
    let start = flow_arm_starts(body).last().copied().unwrap_or(0);
    let arm = &body[start..];
    let lead = arm.len() - arm.trim_start().len();
    (&arm[lead..], start + lead)
}

fn flow_arm_index(body: &str) -> usize {
    flow_arm_starts(body).len()
}

/// The byte offset just past each top-level `,` in `body`.
fn flow_arm_starts(body: &str) -> Vec<usize> {
    let mut starts = Vec::new();
    let bytes = body.as_bytes();
    let mut quote = None;
    let mut escaped = false;
    let mut depth = 0usize;
    for (index, byte) in bytes.iter().copied().enumerate() {
        if escaped {
            escaped = false;
            continue;
        }
        match (quote, byte) {
            (Some(b'"'), b'\\') => escaped = true,
            (Some(active), current) if active == current => quote = None,
            (None, b'\'' | b'"') => quote = Some(byte),
            (None, b'(' | b'[' | b'{') => depth += 1,
            (None, b')' | b']' | b'}') => depth = depth.saturating_sub(1),
            (None, b',') if depth == 0 => starts.push(index + 1),
            _ => {}
        }
    }
    starts
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
fn scan_scalar(prefix: &str, offset: usize, path: SchemaSourcePath) -> Option<SchemaCursor> {
    let (scalar, _) = decode_partial_scalar_at(prefix, 0);
    let src = scalar.decoded().to_string();
    let mut lex = Lexer::new(&src);
    let mut stack: Vec<Frame> = Vec::new();
    // The type keyword at the current type position, and whether a `[]` has
    // been typed since it — together they decide whether the *next* `(` opens
    // the item-level or the array-level constraint list.
    let mut subject: Option<String> = None;
    let mut saw_array = false;
    let mut last_word: Option<String> = None;
    let mut after_at: Option<String> = None;
    let mut token: Option<(String, Range<usize>)> = None;

    loop {
        lex.skip_ws();
        let start = lex.pos;
        let Some(byte) = lex.peek_byte() else { break };
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
                after_at = None;
                token = None;
            }
            b';' => {
                lex.pos += 1;
                after_at = None;
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
                after_at = None;
                token = None;
            }
            b'@' => {
                lex.pos += 1;
                after_at = last_word.clone().or(subject.clone());
                token = None;
            }
            b'\'' | b'"' => {
                let (text, span) = read_quoted_prefix(&src, &mut lex, byte);
                token = Some((text, span));
            }
            b'-' if src.as_bytes().get(start + 1) == Some(&b'>') => {
                // Everything past the top-level `->` is a human description,
                // not grammar the cursor API can speak to.
                return None;
            }
            _ => {
                let (word, span) = if matches!(stack.last(), Some(Frame::ArgList { .. }))
                    || after_at.is_some()
                {
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
                    Some(Frame::ArgList { .. }) => {}
                    Some(Frame::ConstraintList { .. }) => {}
                    Some(Frame::InlineObject { in_value: false, .. }) => {}
                    _ if after_at.is_none() => {
                        subject = Some(word.clone());
                        saw_array = false;
                    }
                    _ => {}
                }
                last_word = Some(word.clone());
                token = Some((word, span));
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
        Some(Frame::InlineObject { .. }) | None => match after_at {
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

/// Consumes a quoted run that may not be closed yet, returning its contents and
/// the span of those contents.
fn read_quoted_prefix(src: &str, lex: &mut Lexer<'_>, quote: u8) -> (String, Range<usize>) {
    let start = lex.pos + 1;
    lex.pos += 1;
    while let Some(byte) = lex.peek_byte() {
        if byte == quote {
            let end = lex.pos;
            lex.pos += 1;
            return (src[start..end].to_string(), start..end);
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

    #[test]
    fn a_description_is_prose_rather_than_grammar() {
        assert!(locate_type_definition_cursor("string -> a note", 0, 16).is_none());
    }
}
