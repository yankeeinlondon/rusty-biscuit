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
//! single-quoted, and double-quoted scalars, text on a plain scalar's
//! continuation line, CRLF input, and multibyte content all report authored
//! document ranges rather than decoded ones. Nothing here
//! reads a file, expands an import, or evaluates anything.

use std::ops::Range;

use super::grammar::{self, Lexer};
use super::source::{
    ExpressionContext, SchemaSourcePath, SchemaSourcePathSegment, quoted_flow_scalar_end,
    scan_expression, split_flow_entries,
};
use super::yaml_scalar::{DecodedScalar, decode_partial_scalar_at};

/// The structural role a cursor occupies inside a partially authored value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchemaCursorRole {
    /// A type keyword, named-type reference (bare or before `@`), or
    /// whole-definition scaffold position.
    Type,
    /// A constraint keyword inside a `(…)` constraint list.
    Constraint {
        /// The type keyword or named-type reference the constraint list
        /// attaches to, when one was typed. A reference's target is unknown
        /// here, so its name is reported as authored.
        subject: Option<String>,
        /// Whether this is the postfix `[](…)` array-level list, whose accepted
        /// constraints are the array set rather than the item set.
        array_level: bool,
        /// The list's leading group while no `;` has been typed in it: the
        /// entries authored before the cursor's own token, in authored order.
        /// `alphanumeric` options can only be written in this group, so
        /// `None` (a `;` precedes the cursor) means only constraints may
        /// follow.
        leading_group: Option<Vec<String>>,
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
    /// A composition operand: the text after `...` in an inline object, or
    /// a `$extends` value or sequence item read by
    /// [`locate_composition_operand_cursor`]. Only a named object type can
    /// be written here, never a type keyword or a constraint.
    CompositionOperand {
        /// Which half of `Name@reference` the cursor is in.
        part: CompositionOperandPart,
        /// The authored document range of the operand from its first byte
        /// up to the cursor (after `...` and any blank before the name), so
        /// a completion can replace the whole operand typed so far.
        /// [`SchemaCursor::token_span`] covers only the active half.
        operand_span: Range<usize>,
    },
}

/// The half of a composition operand a cursor is in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompositionOperandPart {
    /// The type name, before any `@`.
    Name,
    /// The reference after `@`: `this`, a file path, or an embedded name.
    Reference {
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
/// use simplified_schema::{
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

/// Locates the authoring state of `cursor` inside a plain scalar already
/// decoded from `source`, as [`locate_type_definition_cursor`] does for raw
/// value text.
///
/// `scalar` is the result of
/// [`decode_plain_scalar_node`](super::decode_plain_scalar_node) on `source`,
/// so a value that continues onto later lines is read as YAML folds it and any
/// anchor or tag before it stays outside every reported range. Use it when the
/// value cannot be read from a parsed document, such as a definition still
/// being typed.
///
/// ## Returns
///
/// `None` when `cursor` is before the scalar or not on a character boundary,
/// when it is past the scalar's last authored byte with anything other than
/// spaces or tabs between them on that line, and in the cases
/// [`locate_type_definition_cursor`] lists.
pub fn locate_type_definition_cursor_in_scalar(
    source: &str,
    scalar: &DecodedScalar,
    cursor: usize,
) -> Option<SchemaCursor> {
    scan_decoded(&scalar.prefix_at(source, cursor)?, 0, SchemaSourcePath::root())
}

/// [`locate_schema_declaration_cursor`] for a plain scalar already decoded
/// from `source`; see [`locate_type_definition_cursor_in_scalar`].
pub fn locate_schema_declaration_cursor_in_scalar(
    source: &str,
    scalar: &DecodedScalar,
    cursor: usize,
) -> Option<SchemaCursor> {
    whole_decoded(&scalar.prefix_at(source, cursor)?, 0, SchemaSourcePath::root())
}

/// Locates the authoring state of a cursor inside one partially authored
/// `$extends` operand: a scalar `$extends` value, or one item of its
/// sequence, which the caller has already framed.
///
/// `value_source` is the operand's authored text (quoted or plain, never
/// line-ending-normalized), `value_offset` its byte offset in the caller's
/// document, and `cursor` a document byte offset at or inside it. The role
/// is always [`SchemaCursorRole::CompositionOperand`], and its ranges are
/// authored document ranges.
///
/// ## Examples
///
/// ```
/// use simplified_schema::{CompositionOperandPart, SchemaCursorRole, locate_composition_operand_cursor};
///
/// let value = "'audit@./sha";
/// let state = locate_composition_operand_cursor(value, 10, 10 + value.len()).unwrap();
/// assert_eq!(state.token, "./sha");
/// assert_eq!(state.token_span, 17..22);
/// assert!(matches!(
///     state.role,
///     SchemaCursorRole::CompositionOperand { part: CompositionOperandPart::Reference { .. }, .. }
/// ));
/// ```
///
/// ## Returns
///
/// `None` when `cursor` is outside `value_source` or not on a character
/// boundary, and when the text before the cursor is not the start of an
/// operand: a postfix, description, or slot after the name, or blank text
/// after a complete name.
pub fn locate_composition_operand_cursor(
    value_source: &str,
    value_offset: usize,
    cursor: usize,
) -> Option<SchemaCursor> {
    let relative = cursor.checked_sub(value_offset)?;
    if relative > value_source.len() || !value_source.is_char_boundary(relative) {
        return None;
    }
    let (scalar, _) = decode_partial_scalar_at(&value_source[..relative], 0);
    operand_decoded(&scalar, value_offset)
}

/// [`locate_composition_operand_cursor`] for a plain scalar already decoded
/// from `source`; see [`locate_type_definition_cursor_in_scalar`].
pub fn locate_composition_operand_cursor_in_scalar(
    source: &str,
    scalar: &DecodedScalar,
    cursor: usize,
) -> Option<SchemaCursor> {
    operand_decoded(&scalar.prefix_at(source, cursor)?, 0)
}

/// Reads a whole decoded operand prefix with the grammar's identifier rule.
fn operand_decoded(scalar: &DecodedScalar, offset: usize) -> Option<SchemaCursor> {
    let src = scalar.decoded();
    let mut lex = Lexer::new(src);
    lex.skip_ws();
    let operand_start = lex.pos;
    let (name, name_span) = lex.read_ident();
    let (part, token, token_span) = if lex.pos == src.len() {
        (CompositionOperandPart::Name, name, name_span)
    } else {
        lex.skip_ws();
        if name.is_empty() || lex.peek_byte() != Some(b'@') {
            return None;
        }
        let reference = lex.pos + 1;
        let lead = reference + src[reference..].len() - src[reference..].trim_start().len();
        (CompositionOperandPart::Reference { name }, src[lead..].to_string(), lead..src.len())
    };
    Some(SchemaCursor {
        path: SchemaSourcePath::root(),
        role: SchemaCursorRole::CompositionOperand {
            part,
            operand_span: shift(scalar.project(operand_start..src.len())?, offset),
        },
        token,
        token_span: shift(scalar.project(token_span)?, offset),
    })
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
    whole_decoded(&scalar, offset, path)
}

fn whole_decoded(scalar: &DecodedScalar, offset: usize, path: SchemaSourcePath) -> Option<SchemaCursor> {
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
    /// `spreads` counts the `...` operands seen so far, and `spread` is the
    /// decoded offset just past the `...` of the member being typed, when it
    /// is one.
    InlineObject {
        key: Option<String>,
        in_value: bool,
        spreads: usize,
        spread: Option<usize>,
    },
    /// Inside a `(…)` constraint list attached to `subject`. `group` holds
    /// the entries of the list's leading group with their spans, until a `;`
    /// ends that group (`None`).
    ConstraintList {
        subject: Option<String>,
        array_level: bool,
        group: Option<Vec<(String, Range<usize>)>>,
    },
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
    scan_decoded(&scalar, offset, path)
}

/// [`scan_scalar`] over text already decoded up to the cursor.
fn scan_decoded(scalar: &DecodedScalar, offset: usize, path: SchemaSourcePath) -> Option<SchemaCursor> {
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
        // `...` in key position starts a composition operand, read as a type
        // name and an optional `@` reference like any other.
        if let Some(Frame::InlineObject { in_value: false, spreads, spread, .. }) = stack.last_mut()
            && spread.is_none()
            && src[start..].starts_with("...")
        {
            lex.pos += 3;
            *spreads += 1;
            *spread = Some(lex.pos);
            last_word = None;
            token = None;
            continue;
        }
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
                Some(Frame::ConstraintList { group: Some(group), .. }) => {
                    group.push((word.clone(), span.clone()));
                }
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
                stack.push(Frame::InlineObject { key: None, in_value: false, spreads: 0, spread: None });
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
                    Some(Frame::ConstraintList { subject, array_level, .. }) => Frame::ArgList {
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
                        group: Some(Vec::new()),
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
                if let Some(Frame::InlineObject { key, in_value, spread, .. }) = stack.last_mut() {
                    *key = None;
                    *in_value = false;
                    *spread = None;
                    subject = None;
                    saw_array = false;
                }
                token = None;
            }
            b';' => {
                lex.pos += 1;
                if let Some(Frame::ConstraintList { group, .. }) = stack.last_mut() {
                    *group = None;
                }
                token = None;
            }
            b':' => {
                lex.pos += 1;
                if let Some(Frame::InlineObject { key, in_value, .. }) = stack.last_mut() {
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
    let token_span = scalar.project(decoded_span.clone())?;
    let mut path = path;
    for frame in &stack {
        if let Frame::InlineObject { key: Some(key), in_value: true, .. } = frame {
            path = path.property(key);
        }
    }
    if let Some(Frame::InlineObject { in_value: false, spreads, spread: Some(_), .. }) = stack.last() {
        path = path.spread(spreads - 1);
    }
    let role = match stack.last() {
        Some(Frame::ArgList { subject, constraint, array_level }) => SchemaCursorRole::Argument {
            subject: subject.clone(),
            constraint: constraint.clone(),
            array_level: *array_level,
        },
        Some(Frame::ConstraintList { subject, array_level, group }) => {
            SchemaCursorRole::Constraint {
                subject: subject.clone(),
                array_level: *array_level,
                leading_group: group.as_ref().map(|group| {
                    group
                        .iter()
                        .filter(|(_, span)| *span != decoded_span)
                        .map(|(word, _)| word.clone())
                        .collect()
                }),
            }
        }
        Some(Frame::InlineObject { in_value: false, spread: Some(after_dots), .. }) => {
            let operand_start = after_dots + src[*after_dots..].len() - src[*after_dots..].trim_start().len();
            SchemaCursorRole::CompositionOperand {
                part: match import_name {
                    Some(name) => CompositionOperandPart::Reference { name },
                    None => CompositionOperandPart::Name,
                },
                operand_span: shift(scalar.project(operand_start..src.len())?, offset),
            }
        }
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
                leading_group: None,
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
                leading_group: Some(Vec::new()),
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
                leading_group: Some(Vec::new()),
            }
        );
        let item = at_end("string(mi");
        assert_eq!(
            item.role,
            SchemaCursorRole::Constraint {
                subject: Some("string".into()),
                array_level: false,
                leading_group: Some(Vec::new()),
            }
        );
    }

    /// The leading group is what `alphanumeric` options may be written in:
    /// it lists the entries before the cursor's own token until a `;` ends
    /// it, and an argument list's words never enter it.
    #[test]
    fn a_constraint_list_reports_its_leading_group_until_the_first_separator() {
        let group = |value: &str| match at_end(value).role {
            SchemaCursorRole::Constraint { leading_group, .. } => leading_group,
            other => panic!("{value:?} -> {other:?}"),
        };
        let words = |list: &[&str]| Some(list.iter().map(|word| word.to_string()).collect::<Vec<_>>());
        assert_eq!(group("alphanumeric("), words(&[]));
        assert_eq!(group("alphanumeric(allow-da"), words(&[]));
        assert_eq!(group("alphanumeric(allow-dashes, "), words(&["allow-dashes"]));
        assert_eq!(
            group("alphanumeric(allow-dashes, leading-alpha, allow-p"),
            words(&["allow-dashes", "leading-alpha"])
        );
        assert_eq!(group("alphanumeric(allow-dashes; "), None);
        assert_eq!(group("alphanumeric(allow-dashes; min(1); re"), None);
        assert_eq!(group("alphanumeric(pattern(a, b), "), words(&["pattern"]));
        assert_eq!(group("[string, alphanumeric(allow-dashes, "), words(&["allow-dashes"]));
        // A nested list keeps its own group.
        assert_eq!(group("{ id: alphanumeric(leading-alpha; max(3)), b: alphanumeric("), words(&[]));
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
        for value in ["strin(", "))(", "string(((", "{ a: [b"] {
            assert!(super::super::grammar::parse_type_expr("p", value).is_err(), "{value:?}");
            assert!(locate_type_definition_cursor(value, 0, value.len()).is_some(), "{value:?}");
        }
    }

    fn decoded_plain(source: &str, holder: &str, parent_indent: usize) -> DecodedScalar {
        let start = source.find(holder).expect("holder") + holder.len();
        super::super::decode_plain_scalar_node(source, start, parent_indent)
            .expect("plain scalar")
            .0
    }

    #[test]
    fn a_decoded_continuation_reads_the_cursor_without_its_anchor_or_tag() {
        for (source, holder) in [
            ("t: &e string(min(1);\n  ma\n", "t: "),
            ("t: !!str string(min(1);\n  ma\n", "t: "),
            ("t: &e !!str string(min(1);\r\n\r\n  ma\r\n", "t: "),
        ] {
            let scalar = decoded_plain(source, holder, 0);
            let cursor = source.find("ma").unwrap() + 2;
            let state = locate_type_definition_cursor_in_scalar(source, &scalar, cursor)
                .expect("cursor state");
            assert_eq!(state.token, "ma", "{source:?}");
            assert_eq!(state.token_span, cursor - 2..cursor, "{source:?}");
            assert!(
                matches!(
                    &state.role,
                    SchemaCursorRole::Constraint { subject: Some(subject), leading_group: None, .. }
                        if subject == "string"
                ),
                "{source:?}: {:?}",
                state.role
            );
            // The same token read from raw text that starts after the anchor.
            let raw = locate_type_definition_cursor(source, 0, cursor);
            assert_eq!(raw.map(|raw| raw.token), Some("ma".to_string()));
        }
    }

    #[test]
    fn a_decoded_scalar_accepts_only_cursors_it_authored_or_trailing_blanks() {
        let source = "t: &e string(min(1);\n  max(2); \nnext: x\n";
        let scalar = decoded_plain(source, "t: ", 0);
        let end = source.find("max(2);").unwrap() + "max(2);".len();
        // At the final byte, and after trailing blanks: a fresh position.
        let state = locate_type_definition_cursor_in_scalar(source, &scalar, end).unwrap();
        assert_eq!((state.token.as_str(), state.token_span), ("", end..end));
        let state = locate_type_definition_cursor_in_scalar(source, &scalar, end + 1).unwrap();
        assert_eq!(state.token_span, end + 1..end + 1);
        // Before the scalar (inside the anchor), and on the next line.
        let anchor = source.find("&e").unwrap() + 1;
        assert!(locate_type_definition_cursor_in_scalar(source, &scalar, anchor).is_none());
        let sibling = source.find("next").unwrap() + 2;
        assert!(locate_type_definition_cursor_in_scalar(source, &scalar, sibling).is_none());
        // A comment after the scalar is not part of it.
        let source = "t: string(min(1);\n  max(2) # note\n";
        let scalar = decoded_plain(source, "t: ", 0);
        let inside = source.find("note").unwrap();
        assert!(locate_type_definition_cursor_in_scalar(source, &scalar, inside).is_none());
    }

    #[test]
    fn a_decoded_declaration_arm_is_one_token_without_its_anchor() {
        let source = "- &a ./types\n  .yaml\n";
        let scalar = decoded_plain(source, "- ", 0);
        let cursor = source.len() - 1;
        let state = locate_schema_declaration_cursor_in_scalar(source, &scalar, cursor).unwrap();
        assert_eq!(state.token, "./types .yaml");
        assert_eq!(state.token_span, source.find("./").unwrap()..cursor);
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
                leading_group: Some(Vec::new()),
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
                SchemaCursorRole::Constraint {
                    subject: Some("string".into()),
                    array_level: false,
                    leading_group: None,
                },
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

    /// The role at `cursor` inside `value`.
    fn role_at(value: &str, cursor: usize) -> SchemaCursorRole {
        locate_type_definition_cursor(value, 0, cursor).expect("cursor state").role
    }

    #[test]
    fn a_bare_reference_name_is_a_type_position() {
        let state = at_end("pers");
        assert_eq!(state.role, SchemaCursorRole::Type);
        assert_eq!(state.token, "pers");

        let inline = at_end("{ author: pers");
        assert_eq!(inline.role, SchemaCursorRole::Type);
        assert_eq!(inline.path, SchemaSourcePath::root().property("author"));

        let arm = at_end("[string, pers");
        assert_eq!(arm.role, SchemaCursorRole::Type);
        assert!(is_union_arm_path(&arm.path));
    }

    #[test]
    fn bare_reference_constraint_positions_match_explicit_reference_positions() {
        let constraint = |array_level| SchemaCursorRole::Constraint {
            subject: Some("person".into()),
            array_level,
            leading_group: Some(Vec::new()),
        };
        for (bare, explicit, cursor, expected) in [
            ("person(re)", "person(re)@this", 9, constraint(false)),
            ("person[](mi)", "person[](mi)@this", 11, constraint(true)),
            ("person[]", "person[]@./people.yaml", 8, SchemaCursorRole::Type),
            ("person(ea)[]", "person(ea)[]@./people.yaml", 9, constraint(false)),
            ("person(eager)[](mi)", "person(eager)[](mi)@this", 18, constraint(true)),
            (
                "person(min(1))",
                "person(min(1))@this",
                12,
                SchemaCursorRole::Argument {
                    subject: Some("person".into()),
                    constraint: "min".into(),
                    array_level: false,
                },
            ),
        ] {
            assert_eq!(role_at(bare, cursor), expected, "{bare}");
            assert_eq!(role_at(explicit, cursor), expected, "{explicit}");
        }
    }

    #[test]
    fn a_bare_reference_in_an_inline_object_keeps_its_constraint_role() {
        let state = at_end("{ author: person(requ");
        assert_eq!(state.path, SchemaSourcePath::root().property("author"));
        assert_eq!(
            state.role,
            SchemaCursorRole::Constraint {
                subject: Some("person".into()),
                array_level: false,
                leading_group: Some(Vec::new()),
            }
        );
    }

    // ── Composition operands ────────────────────────────────────────────

    fn operand(part: CompositionOperandPart, operand_span: Range<usize>) -> SchemaCursorRole {
        SchemaCursorRole::CompositionOperand { part, operand_span }
    }

    fn reference(name: &str) -> CompositionOperandPart {
        CompositionOperandPart::Reference { name: name.into() }
    }

    /// `(value, cursor, token, token text in value, role)` for a cursor at
    /// `cursor` bytes into `value`; spans are checked against the value text.
    fn assert_spread(value: &str, cursor: usize, token: &str, role: SchemaCursorRole, path: SchemaSourcePath) {
        let state = locate_type_definition_cursor(value, 0, cursor).unwrap_or_else(|| panic!("{value:?}"));
        assert_eq!(state.role, role, "{value:?} at {cursor}");
        assert_eq!(state.token, token, "{value:?} at {cursor}");
        assert_eq!(&value[state.token_span.clone()], token, "{value:?} at {cursor}");
        assert_eq!(state.path, path, "{value:?} at {cursor}");
    }

    #[test]
    fn composition_cursor_after_an_ellipsis_names_an_operand() {
        let root = SchemaSourcePath::root;
        // Empty, right after `...`, and after a blank.
        assert_spread("{ ...", 5, "", operand(CompositionOperandPart::Name, 5..5), root().spread(0));
        assert_spread("{ ... ", 6, "", operand(CompositionOperandPart::Name, 6..6), root().spread(0));
        // A partial name, and the second spread of an object.
        assert_spread("{ ...ba", 7, "ba", operand(CompositionOperandPart::Name, 5..7), root().spread(0));
        assert_spread(
            "{ ...a, id: string, ...common-att",
            33,
            "common-att",
            operand(CompositionOperandPart::Name, 23..33),
            root().spread(1),
        );
        // Before and after `@`: the name half, then the reference half.
        assert_spread("{ ...base@", 10, "", operand(reference("base"), 5..10), root().spread(0));
        assert_spread("{ ...base@th", 12, "th", operand(reference("base"), 5..12), root().spread(0));
        assert_spread(
            "{ ...audit@./types/sha",
            22,
            "./types/sha",
            operand(reference("audit"), 5..22),
            root().spread(0),
        );
        // A cursor inside the name of a complete object, before its postfix:
        // only the text up to the cursor is read.
        let value = "{ ...base, id: string }(closed)";
        assert_spread(value, 8, "bas", operand(CompositionOperandPart::Name, 5..8), root().spread(0));
        let value = "{ ...base@./a.yaml }(min-keys(1)) -> described";
        assert_spread(value, 13, "./a", operand(reference("base"), 5..13), root().spread(0));
    }

    #[test]
    fn composition_cursor_follows_nested_objects_arrays_and_union_arms() {
        let root = SchemaSourcePath::root;
        // A spread inside an object nested in an array of objects.
        assert_spread(
            "{ rows: { ...ro",
            15,
            "ro",
            operand(CompositionOperandPart::Name, 13..15),
            root().property("rows").spread(0),
        );
        assert_spread(
            "{ rows: { ...row }[], more: { a: string, ...ot",
            46,
            "ot",
            operand(CompositionOperandPart::Name, 44..46),
            root().property("more").spread(0),
        );
        // A flow-sequence union arm, and a `|` union option.
        assert_spread(
            "[string, { ...ba",
            16,
            "ba",
            operand(CompositionOperandPart::Name, 14..16),
            root().union_arm(1).spread(0),
        );
        assert_spread("string | { ...ba", 16, "ba", operand(CompositionOperandPart::Name, 14..16), root().spread(0));
        // A spread after a completed one returns to key position after `,`.
        let state = at_end("{ ...base, na");
        assert_eq!(state.role, SchemaCursorRole::InlineObjectKey);
        assert_eq!(state.token, "na");
    }

    #[test]
    fn composition_cursor_projects_through_quoted_and_multiline_scalars() {
        // Quoted scalars: the ranges are authored bytes.
        for value in ["'{ a: string, ...ba", "\"{ a: string, ...ba"] {
            let state = at_end(value);
            assert_eq!(state.token, "ba", "{value:?}");
            assert_eq!(&value[state.token_span.clone()], "ba", "{value:?}");
            let SchemaCursorRole::CompositionOperand { operand_span, part } = &state.role else {
                panic!("{value:?}: {:?}", state.role);
            };
            assert_eq!(part, &CompositionOperandPart::Name);
            assert_eq!(&value[operand_span.clone()], "ba", "{value:?}");
        }
        // An escape before the operand shifts the authored range.
        let value = "\"{ a: string -> caf\\u00e9, ...audit@./sh";
        let state = at_end(value);
        assert_eq!(state.token, "./sh");
        assert_eq!(&value[state.token_span.clone()], "./sh");
        // A plain scalar continued on the next line (a plain scalar cannot
        // start with `{`, which opens a YAML flow mapping, or hold `: `).
        let source = "t: string | { ...base,\n  ...comm\n";
        let scalar = decoded_plain(source, "t: ", 0);
        let cursor = source.find("comm").unwrap() + 4;
        let state = locate_type_definition_cursor_in_scalar(source, &scalar, cursor).unwrap();
        assert_eq!(state.token, "comm");
        assert_eq!(state.token_span, cursor - 4..cursor);
        assert_eq!(state.role, operand(CompositionOperandPart::Name, cursor - 4..cursor));
        assert_eq!(state.path, SchemaSourcePath::root().spread(1));
        // A folded operand: the name and reference on separate lines.
        let source = "t: string | { ...base@\n  ./ty\n";
        let scalar = decoded_plain(source, "t: ", 0);
        let cursor = source.find("./ty").unwrap() + 4;
        let state = locate_type_definition_cursor_in_scalar(source, &scalar, cursor).unwrap();
        assert_eq!(state.token, "./ty");
        assert_eq!(state.token_span, cursor - 4..cursor);
        assert!(matches!(
            state.role,
            SchemaCursorRole::CompositionOperand { part: CompositionOperandPart::Reference { ref name }, .. } if name == "base"
        ));
    }

    #[test]
    fn composition_cursor_ignores_keys_descriptions_and_document_data() {
        // A bare literal key and a key after a completed spread.
        assert_eq!(at_end("{ na").role, SchemaCursorRole::InlineObjectKey);
        assert_eq!(at_end("{ ...base, ").role, SchemaCursorRole::InlineObjectKey);
        // An ellipsis in a description is prose: inside it there is no
        // state, and after its `,` the cursor is at a key.
        assert_eq!(locate_type_definition_cursor("{ a: string -> wait...", 0, 22), None);
        assert_eq!(locate_type_definition_cursor("string -> and so on...", 0, 22), None);
        assert_eq!(at_end("{ a: string -> wait..., b").role, SchemaCursorRole::InlineObjectKey);
        // Outside an object, and in a property value, `...` is not a spread.
        for value in ["...data", "{ a: ...b"] {
            assert!(
                !matches!(at_end(value).role, SchemaCursorRole::CompositionOperand { .. }),
                "{value:?}: {:?}",
                at_end(value).role
            );
        }
        // An argument that contains an ellipsis stays an argument.
        assert!(matches!(at_end("{ a: enum(x, ...y").role, SchemaCursorRole::Argument { .. }));
    }

    #[test]
    fn composition_cursor_reads_a_whole_extends_operand() {
        let at = |value: &str, cursor: usize| locate_composition_operand_cursor(value, 0, cursor);
        for (value, cursor, token, part) in [
            ("", 0, "", CompositionOperandPart::Name),
            ("ev", 2, "ev", CompositionOperandPart::Name),
            ("events@", 7, "", reference("events")),
            ("events@thi", 10, "thi", reference("events")),
            ("audit@./a.yaml", 9, "./a", reference("audit")),
            ("'audit@./sh", 11, "./sh", reference("audit")),
            ("\"audit@./sh", 11, "./sh", reference("audit")),
            ("  audit @ ./sh", 14, "./sh", reference("audit")),
        ] {
            let state = at(value, cursor).unwrap_or_else(|| panic!("{value:?}"));
            assert_eq!(state.token, token, "{value:?}");
            assert_eq!(&value[state.token_span.clone()], token, "{value:?}");
            assert_eq!(state.path, SchemaSourcePath::root(), "{value:?}");
            let SchemaCursorRole::CompositionOperand { part: found, operand_span } = &state.role else {
                panic!("{value:?}: {:?}", state.role);
            };
            assert_eq!(found, &part, "{value:?}");
            assert_eq!(operand_span.end, cursor, "{value:?}");
            assert!(value[operand_span.clone()].ends_with(token), "{value:?}");
        }
        // The document offset shifts every range.
        let state = locate_composition_operand_cursor("base", 40, 43).unwrap();
        assert_eq!(state.token_span, 40..43);
        assert_eq!(state.role, operand(CompositionOperandPart::Name, 40..43));
        // Postfixes, a description, a slot, and blank text after a complete
        // name are not an operand being typed; a cursor outside is refused.
        for (value, cursor) in [("base(", 5), ("base[]", 6), ("base -> d", 9), ("{{ x", 4), ("base ", 5)] {
            assert_eq!(at(value, cursor), None, "{value:?}");
        }
        assert_eq!(locate_composition_operand_cursor("base", 10, 9), None);
        assert_eq!(locate_composition_operand_cursor("base", 0, 5), None);
        // A multiline plain scalar, read from its decoded node.
        // The anchor stays outside every range, and the folded line break
        // is a blank between `@` and the reference.
        let source = "$extends: &e common-attributes@\n  ./sha\n";
        let scalar = decoded_plain(source, "$extends: ", 0);
        let cursor = source.find("./sha").unwrap() + 5;
        let state = locate_composition_operand_cursor_in_scalar(source, &scalar, cursor).unwrap();
        assert_eq!(state.token, "./sha");
        assert_eq!(state.token_span, cursor - 5..cursor);
        let name_start = source.find("common").unwrap();
        assert_eq!(state.role, operand(reference("common-attributes"), name_start..cursor));
    }
}
