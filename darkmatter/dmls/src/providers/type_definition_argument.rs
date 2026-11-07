//! Type-definition hover and completion inside a string-literal argument
//! whose catalog parameter is a `type-definition`, such as `load_data`'s
//! `as`: `{{ load_data("hosts.yaml", "{ name: string }[]") }}`.
//!
//! Which arguments qualify comes from the expression-function catalog's
//! parameter types, never from a function name. The decoded string is read by
//! the semantic routines a frontmatter `type-definition` value uses, and every
//! position maps back through the expression lexer's string escapes and, in
//! frontmatter, through the scalar's YAML projection. Completion tolerates an
//! unfinished string or call; diagnostics are not produced here and stay
//! gated on an expression that parses.
//!
//! Passive: nothing here reads a file, resolves a named-type import, or
//! evaluates the call.

use std::borrow::Cow;
use std::ops::Range;

use darkmatter::markdown::compose::expression::{Lexer, SpannedCallArgument, SpannedExpr, SpannedExprKind, Token};
use darkmatter::markdown::span::SourceSpan;
use lsp_types::{CompletionItem, CompletionTextEdit, Hover};
use simplified_schema::expression::scan::expression_region_at;
use simplified_schema::{
    PropertyAtom, PropertyDef, SimplifiedType, TypeExpr, locate_type_definition_cursor, parse_type_definition_value,
};

use super::DocumentContext;
use super::dsl::body_base;
use super::frontmatter::{self, NamedTypes};
use crate::overlay::expressions::{self, ScalarProjection};

/// Where the expression holding the cursor is authored.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Site {
    /// A `{{ … }}` span in the Markdown body.
    Body,
    /// A frontmatter string scalar's `{{ … }}` span, or an Expression-typed
    /// frontmatter value.
    Frontmatter,
}

/// Type-definition completion at `offset` when it sits in a qualifying
/// string argument at `site`. `Some` claims the cursor even when nothing
/// matches, so the caller offers no expression candidates inside the string.
pub(crate) fn completion(ctx: &DocumentContext, offset: usize, site: Site) -> Option<Vec<CompletionItem>> {
    let argument = locate(ctx, offset, site)?;
    let prefix = &argument.decoded[..argument.cursor];
    // A leading quote would be read as YAML quoting of the definition, which
    // the expression string does not carry.
    if prefix.trim_start().starts_with(['"', '\'']) {
        return Some(Vec::new());
    }
    let Some(state) = locate_type_definition_cursor(&argument.decoded, 0, argument.cursor) else {
        return Some(Vec::new());
    };
    let Some(start) = argument.document_offset(state.token_span.start) else {
        return Some(Vec::new());
    };
    let items = frontmatter::type_definition_completions(ctx, start, offset, &state, NamedTypes::Withheld)
        .into_iter()
        .filter_map(|mut item| {
            if let Some(CompletionTextEdit::Edit(edit)) = &mut item.text_edit {
                edit.new_text = argument.encode(&edit.new_text)?;
            }
            Some(item)
        })
        .collect();
    Some(items)
}

/// Whether `offset` sits in a qualifying string argument at `site`.
pub(crate) fn claims(ctx: &DocumentContext, offset: usize, site: Site) -> bool {
    locate(ctx, offset, site).is_some()
}

/// The type-definition hover for a complete qualifying string argument at
/// `offset`, ranged on the string literal. `None` when the cursor is not in
/// one or its text is not a valid definition, so the expression hover
/// answers instead.
pub(crate) fn hover(ctx: &DocumentContext, offset: usize, site: Site) -> Option<Hover> {
    let argument = locate(ctx, offset, site)?;
    let literal = argument.literal.clone()?;
    let definition =
        parse_type_definition_value(&argument.parameter, &serde_yaml_ng::Value::String(argument.decoded.clone())).ok()?;
    let span = argument.analyzed.project(literal)?;
    frontmatter::markup_hover(
        ctx,
        span,
        frontmatter::meta_schema_definition_hover_body(&argument.parameter, &definition),
    )
}

/// The analyzed text an expression is read in, and how it maps to document
/// bytes: the document itself in the body, a scalar's projection in
/// frontmatter.
struct Analyzed<'t> {
    text: Cow<'t, str>,
    projection: Option<ScalarProjection>,
}

impl Analyzed<'_> {
    fn project(&self, range: Range<usize>) -> Option<SourceSpan> {
        match &self.projection {
            None => Some(range),
            Some(projection) => projection.project(range),
        }
    }

    fn encode(&self, text: &str) -> Option<String> {
        match &self.projection {
            None => Some(text.to_string()),
            Some(projection) => projection.encode_fragment(text),
        }
    }
}

/// A string literal bound to a `type-definition` parameter.
struct TypeArgument<'t> {
    /// The catalog parameter's name, which titles the hover.
    parameter: String,
    /// The string's value under the expression lexer's escape rules; for an
    /// unterminated string, the text authored before the cursor.
    decoded: String,
    /// The analyzed offset where each decoded byte's source starts, then the
    /// end of the decoded source (`decoded.len() + 1` entries).
    raw: Vec<usize>,
    quote: u8,
    /// The analyzed range of the whole literal, quotes included; `None` while
    /// the string is unterminated.
    literal: Option<Range<usize>>,
    /// The cursor as a decoded offset.
    cursor: usize,
    analyzed: Analyzed<'t>,
}

impl TypeArgument<'_> {
    /// The document offset of decoded boundary `decoded`.
    fn document_offset(&self, decoded: usize) -> Option<usize> {
        let raw = *self.raw.get(decoded)?;
        self.analyzed.project(raw..raw).map(|span| span.start)
    }

    /// `text` encoded for this string's quote, then for the enclosing scalar.
    fn encode(&self, text: &str) -> Option<String> {
        let quote = char::from(self.quote);
        let mut escaped = String::with_capacity(text.len());
        for character in text.chars() {
            match character {
                '\\' => escaped.push_str("\\\\"),
                '\n' => escaped.push_str("\\n"),
                '\t' => escaped.push_str("\\t"),
                '\r' => escaped.push_str("\\r"),
                other if other == quote => {
                    escaped.push('\\');
                    escaped.push(other);
                }
                other => escaped.push(other),
            }
        }
        self.analyzed.encode(&escaped)
    }
}

fn locate<'c>(ctx: &DocumentContext<'c>, offset: usize, site: Site) -> Option<TypeArgument<'c>> {
    match site {
        Site::Body => body_argument(ctx, offset),
        Site::Frontmatter => frontmatter_argument(ctx, offset),
    }
}

fn body_argument<'c>(ctx: &DocumentContext<'c>, offset: usize) -> Option<TypeArgument<'c>> {
    let analyzed = Analyzed { text: Cow::Borrowed(ctx.text), projection: None };
    argument_in(analyzed, body_base(ctx.text), offset)
}

/// The argument in the frontmatter scalar holding `offset`: the whole value
/// of an Expression-typed property, else a `{{ … }}` span in a string scalar.
fn frontmatter_argument<'c>(ctx: &DocumentContext<'c>, offset: usize) -> Option<TypeArgument<'c>> {
    let ast = ctx.overlay?.ast.as_deref()?;
    if !ast.contains_offset(offset) {
        return None;
    }
    let index = frontmatter::cursor_scalar(ast, offset)?;
    if let Some(typing) = frontmatter::scalar_typing(ctx, index)
        && let Some(value) = frontmatter::expression_value(ast, index, typing)
    {
        let cursor = value.projection().analyzed_offset(offset)?;
        // The value is bare expression text; reading it after a synthetic
        // opener gives it the scanner's string and escape rules unchanged.
        let analyzed = Analyzed {
            text: Cow::Owned(format!("{{{{{}", value.expression())),
            projection: Some(value.projection().clone()),
        };
        return argument_in(analyzed, 0, cursor + 2).map(|mut argument| {
            argument.raw.iter_mut().for_each(|raw| *raw -= 2);
            argument.literal = argument.literal.map(|literal| literal.start - 2..literal.end - 2);
            argument
        });
    }
    let (projection, text, scan_from) = expressions::interpolation_text(&ast.entries()[index], ctx.text)?;
    let cursor = projection.analyzed_offset(offset)?;
    argument_in(Analyzed { text: Cow::Owned(text), projection: Some(projection) }, scan_from, cursor)
}

/// The qualifying argument holding analyzed offset `cursor`, in an
/// expression whose opener is at or after `from`.
fn argument_in(analyzed: Analyzed<'_>, from: usize, cursor: usize) -> Option<TypeArgument<'_>> {
    let text: &str = &analyzed.text;
    let region = expression_region_at(text, cursor, &[]).filter(|region| region.start >= from)?;
    let quote_at = region.string_start?;
    let body_start = region.start + 2;
    // An open region has no closer; its argument ends with the cursor's line.
    let body_end = region
        .end
        .map_or_else(|| text[cursor..].find('\n').map_or(text.len(), |at| cursor + at), |end| end - 2);
    let bytes = text.as_bytes();
    let quote = bytes[quote_at];
    let mut close = None;
    let mut at = quote_at + 1;
    while at < body_end {
        match bytes[at] {
            b'\\' => at += 2,
            byte if byte == quote => {
                close = Some(at);
                break;
            }
            _ => at += 1,
        }
    }
    let content_end = close.unwrap_or(cursor);
    let (decoded, raw) = decode_string(text, quote_at + 1..content_end, quote)?;
    let expression = &text[body_start..body_end];
    let relative = |offset: usize| offset - body_start;
    let (name, index) = string_argument_call(
        expression,
        relative(quote_at),
        close.map(relative),
        relative(cursor.min(body_end)),
    )?;
    let parameter = type_definition_parameter(&name, index)?;
    let cursor = raw.iter().rposition(|start| *start <= cursor).unwrap_or(0).min(decoded.len());
    Some(TypeArgument {
        parameter,
        decoded,
        raw,
        quote,
        literal: close.map(|close| quote_at..close + 1),
        cursor,
        analyzed,
    })
}

/// Decodes `range` of `text` as a string literal's contents under the
/// expression lexer's escapes (`\n`, `\t`, `\r`, `\\`, and the literal's own
/// quote; any other escape keeps its backslash), with the analyzed offset
/// each decoded byte starts at. `None` when the range ends inside an escape.
fn decode_string(text: &str, range: Range<usize>, quote: u8) -> Option<(String, Vec<usize>)> {
    let contents = text.get(range.clone())?;
    let mut decoded = String::with_capacity(contents.len());
    let mut raw = Vec::with_capacity(contents.len() + 1);
    let mut characters = contents.char_indices().peekable();
    while let Some((at, character)) = characters.next() {
        let start = range.start + at;
        if character != '\\' {
            decoded.push(character);
            raw.extend((0..character.len_utf8()).map(|byte| start + byte));
            continue;
        }
        let (next_at, next) = characters.next()?;
        let escaped = match next {
            'n' => Some('\n'),
            't' => Some('\t'),
            'r' => Some('\r'),
            '\\' => Some('\\'),
            other if other as u32 == u32::from(quote) => Some(other),
            _ => None,
        };
        match escaped {
            Some(value) => {
                decoded.push(value);
                raw.push(start);
            }
            None => {
                decoded.push('\\');
                raw.push(start);
                decoded.push(next);
                raw.extend((0..next.len_utf8()).map(|byte| range.start + next_at + byte));
            }
        }
    }
    raw.push(range.end);
    Some((decoded, raw))
}

/// The function and argument index whose argument is exactly the string
/// literal opened at `quote_at` of `expression`.
///
/// An expression that parses is read as it stands. Otherwise the shared
/// parser reads the text up to the literal's end (or the cursor, closing the
/// string there) with only the missing closers appended, and the literal
/// must end its argument: `"a" + b` is not a literal argument.
fn string_argument_call(
    expression: &str,
    quote_at: usize,
    close: Option<usize>,
    cursor: usize,
) -> Option<(String, usize)> {
    if let Ok(parsed) = expressions::parse(expression) {
        return literal_argument(&parsed, quote_at);
    }
    let mut recovered = match close {
        Some(close) => {
            let rest = expression[close + 1..].trim_start();
            if !(rest.is_empty() || rest.starts_with([',', ')'])) {
                return None;
            }
            expression[..=close].to_string()
        }
        None => {
            let mut prefix = expression.get(..cursor)?.to_string();
            prefix.push(char::from(expression.as_bytes()[quote_at]));
            prefix
        }
    };
    let tokens = Lexer::new(&recovered).tokenize_all().ok()?;
    let mut closing = Vec::new();
    for token in tokens {
        match token {
            Token::LParen => closing.push(')'),
            Token::LBracket => closing.push(']'),
            Token::LBrace => closing.push('}'),
            Token::RParen | Token::RBracket | Token::RBrace => {
                closing.pop();
            }
            _ => {}
        }
    }
    recovered.extend(closing.into_iter().rev());
    literal_argument(&expressions::parse(&recovered).ok()?, quote_at)
}

fn literal_argument(expr: &SpannedExpr, quote_at: usize) -> Option<(String, usize)> {
    let is_literal = |argument: &SpannedExpr| {
        matches!(argument.kind, SpannedExprKind::StringLiteral(_)) && argument.span.start == quote_at
    };
    let found = match &expr.kind {
        SpannedExprKind::FunctionCall { name, args } => {
            args.iter().position(is_literal).map(|index| (name.clone(), index))
        }
        SpannedExprKind::CallbackCall { name, args } => args
            .iter()
            .position(|argument| matches!(argument, SpannedCallArgument::Value(value) if is_literal(value)))
            .map(|index| (name.clone(), index)),
        _ => None,
    };
    found.or_else(|| expressions::children(expr).into_iter().find_map(|child| literal_argument(child, quote_at)))
}

/// The name of the parameter argument `index` of `function` binds to, when
/// any catalog overload types it `type-definition`.
fn type_definition_parameter(function: &str, index: usize) -> Option<String> {
    expressions::function_overloads(function).iter().find_map(|descriptor| {
        let parameters = descriptor.function_type.parameters.as_slice();
        let parameter = parameters
            .get(index)
            .or_else(|| parameters.last().filter(|parameter| parameter.rest))?;
        is_type_definition(&parameter.def).then(|| parameter.name.clone())
    })
}

fn is_type_definition(def: &PropertyDef) -> bool {
    let atoms: &[PropertyAtom] = match def {
        PropertyDef::Single(atom) => std::slice::from_ref(atom),
        PropertyDef::Union(arms) => arms,
    };
    atoms
        .iter()
        .any(|atom| !atom.is_array && matches!(atom.ty, TypeExpr::Primitive(SimplifiedType::TypeDefinition)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_decode_with_the_offset_each_byte_starts_at() {
        let text = r#""a\"b\\c\qé""#;
        let (decoded, raw) = decode_string(text, 1..text.len() - 1, b'"').unwrap();
        assert_eq!(decoded, "a\"b\\c\\qé");
        assert_eq!(raw, [1, 2, 4, 5, 7, 8, 9, 10, 11, 12]);
    }

    #[test]
    fn only_a_catalog_type_definition_parameter_qualifies() {
        assert_eq!(type_definition_parameter("load_data", 1).as_deref(), Some("as"));
        assert_eq!(type_definition_parameter("LOAD_DATA", 1).as_deref(), Some("as"));
        assert_eq!(type_definition_parameter("load_data", 0), None);
        assert_eq!(type_definition_parameter("upper", 0), None);
    }

    #[test]
    fn a_literal_argument_is_found_in_complete_and_unfinished_calls() {
        let complete = r#"map(load_data("h.yaml", "string[]"), h => h)"#;
        let quote = complete.find("\"string").unwrap();
        assert_eq!(string_argument_call(complete, quote, None, quote + 1), Some(("load_data".into(), 1)));
        let open = r#"map(load_data("h.yaml", "str"#;
        let quote = open.rfind('"').unwrap();
        assert_eq!(string_argument_call(open, quote, None, open.len()), Some(("load_data".into(), 1)));
        let operand = r#"load_data("h.yaml", "str" + t"#;
        let quote = operand.find("\"str").unwrap();
        assert_eq!(string_argument_call(operand, quote, Some(quote + 4), quote + 1), None);
    }
}
