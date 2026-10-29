//! Lexer and parser for the SimplifiedSchema type-and-constraint string.
//!
//! Each property value (after the YAML-shape layer has decided this is a
//! single type expression) is parsed by [`parse_type_expr`] following the
//! EBNF in the schemas spec:
//!
//! ```text
//! type_expr_string := union ( "->" description )?
//! union            := option ( "|" option )*
//! option           := type_expr | group
//! group            := "(" union ")" ( "[]" ( "(" arr_constraints ")" )? )?
//! type_expr        := simple_type
//!                   | inline_object
//!                   | tuple
//!                   | function
//!                   | reference
//! tuple            := "tuple" "(" "[" ( element ( "," element )* ","? )? "]"
//!                     ( ";" item_constraints )? ")"
//!                     ( "[]" ( "(" arr_constraints ")" )? )?
//! element          := union "?"? | "..." option        (spread: an array type)
//! function         := "function" "(" fn_constraint ( ";" fn_constraint )* ";"? ")"
//!                     ( "[]" ( "(" arr_constraints ")" )? )?
//! fn_constraint    := "parameters" "(" ( "void" | parameter ( "," parameter )* ) ")"
//!                   | "returns" "(" union ")" | "category" "(" name ")"
//!                   | "fallible" | "example" "(" ( arglist | call "=>" result
//!                     ( ";" "display-only" ":" reason )? ) ")"
//! parameter        := "..."? identifier "?"? ":" union
//! simple_type      := type_name ( "(" item_constraints ")" )?
//!                                ( "[]" ( "(" arr_constraints ")" )? )?
//! inline_object    := "{" ws* property_list? ws* "}"
//!                     ( "(" item_constraints ")"
//!                     | "[]" ( "(" arr_constraints ")" )?
//!                     )?
//! reference        := identifier ( "(" item_constraints ")" )?
//!                                 ( "[]" ( "(" arr_constraints ")" )? )?
//!                     ( "@" fileref )?
//! property_list    := property_def ( "," ws* property_def )* ","?
//! property_def     := ( identifier | pattern_key ) ws* ":" ws* type_expr_string
//! pattern_key      := "<" pattern_body ">"                   (Feature C)
//! ws               := <whitespace or line-break>
//! identifier       := ( ASCII_ALNUM | "-" | "_" )+
//! type_name        := "string" | "date" | "datetime" | "time" | "number"
//!                   | "numberlike" | "boolean" | "boolish" | "object"
//!                   | "file" | "enum" | "url" | "email" | "yaml" | "json"
//!                   | "literal" | "expression" | "type-definition" | "schema"
//!                   | "unknown" | "any" | "null" | "alphanumeric" | "shell"
//! item_constraints := options ( ";" constraint )*
//!                   | constraint ( ";" constraint )*
//! options          := option ( "," option )*          (alphanumeric, reference)
//! option           := "allow-dashes" | "allow-underscores" | "allow-slashes"
//!                   | "allow-periods" | "allow-whitespace"
//!                   | "block-double-periods" | "block-leading-dash"
//!                   | "block-leading-slash" | "leading-alpha"
//! arr_constraints  := constraint ( ";" constraint )*
//! constraint       := IDENT
//!                   | IDENT "(" arglist ")"
//!                   | "category" "(" name ")"      (name: raw text up to `)`)
//! arglist          := arg ( "," arg )*
//! arg              := NUMBER | BARE_WORD | SQUOTED | DQUOTED
//! fileref          := <biscuit-file file reference> | "this"
//! description      := <rest-of-top-level-field, trimmed>
//! ```
//!
//! A `reference` names a type defined in a schema and is represented as
//! [`TypeExpr::Imported`] until the resolver expands it. With `@fileref` it
//! names a type in that file (`this` is the current schema). Without `@`, an
//! identifier that is a `type_name` is the built-in `simple_type`; any other
//! identifier is a bare local reference, the same value as `Name@this`. So
//! `string` is the built-in and `string@this` is a local type named `string`.
//! A misspelled built-in therefore parses, and fails during resolution unless
//! a local type has that name. A reference's lists mean nothing until its
//! target is known, so the resolver, not this parser, decides what each may
//! carry (only a union accepts a list before `[]`). A reference's leading
//! `alphanumeric` options are kept for the resolver to compare with its
//! target's options.
//!
//! `any` parses as `unknown`. An unknown, repeated, or post-`;` option, and a
//! list without `;` that mixes options and constraints, are grammar errors.
//!
//! `[]` binds tighter than `|`, and a group without `[]` joins its options to
//! the enclosing union, so `(A | B) | C` is `A | B | C` while `(A | B)[]` is
//! one list atom whose type is [`TypeExpr::Union`]. Tuple elements, function
//! parameters, and return types are unions without descriptions; ordering and
//! uniqueness rules for elements and parameters are checked as they are read,
//! and an inline `example(call => result)` is accepted only on a function.
//! Numbers in metadata are admitted through [`crate::markdown::numeric`].
//!
//! A `pattern_key` (`<string>`, `<starting::P>`, `<ending::S>`, `<pattern::RE>`)
//! keys a dictionary/pattern-keyed object; see [`PatternKey`].
//!
//! Inside an `inline_object`, `description` consumes text only until the
//! next top-level comma or closing brace (Decision #9). Inline object
//! nesting is capped at [`MAX_INLINE_OBJECT_DEPTH`] levels (Decision #11).
//!
//! Errors surface as [`SchemaError::Grammar`] with the byte span of the
//! offending token.

use std::{ops::Range, str::FromStr};

use crate::markdown::compose::expression::{SpannedExpr, SpannedExprKind, parse_spanned};
use crate::markdown::numeric::{
    NumberTextError, NumericAdmissionError, admit_number_spelling, format_number, json_number,
};
use crate::markdown::schemas::errors::SchemaError;

use super::alphanumeric::AlphanumericOption;
use super::signature::duplicate_overload;
use super::types::{
    CallExample, Constraint, ExampleOutcome, FunctionParameter, FunctionParameters, FunctionType,
    PatternKey, PatternKeyDef, PropertyAtom, PropertyDef, SchemaShape, SimplifiedType,
    SuggestionCandidate, TupleElement, TupleType, TypeExpr,
};

/// Hard maximum number of nested inline object levels the parser will accept.
/// Exceeding this depth is a grammar error (Decision #11).
pub const MAX_INLINE_OBJECT_DEPTH: usize = 32;

/// Parses a single type-and-constraint string into a [`PropertyAtom`].
///
/// `property` is supplied for error reporting only.
///
/// ## Errors
///
/// [`SchemaError::Grammar`] for invalid grammar, and for a `|` union of more
/// than one option, which is a definition rather than one atom (see
/// [`parse_type_definition`]).
pub fn parse_type_expr(property: &str, input: &str) -> Result<PropertyAtom, SchemaError> {
    match parse_type_definition(property, input)? {
        PropertyDef::Single(atom) => Ok(atom),
        PropertyDef::Union(_) => Err(SchemaError::Grammar {
            property: property.to_string(),
            message: "expected one type, found a `|` union".into(),
            span: 0..input.len(),
        }),
    }
}

/// Parses a type-and-constraint string that may be a `|` union into a
/// [`PropertyDef`].
///
/// `string | number` is the same [`PropertyDef::Union`] as the YAML list
/// `[string, number]`; a single option is a [`PropertyDef::Single`]. A
/// trailing `-> description` describes the last option, as it would in the
/// list form. Two `function` options with the same binding contract are an
/// error.
///
/// ## Errors
///
/// [`SchemaError::Grammar`] with the byte span of the offending text.
pub fn parse_type_definition(property: &str, input: &str) -> Result<PropertyDef, SchemaError> {
    let mut parser = Parser::new(property, input);
    parser.parse_definition()
}

// ── Lexer ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Ident(String),
    Number(f64),
    /// Raw word value (for arg position). Distinct from `Ident` so the parser
    /// can keep the raw lexeme — e.g. enum members.
    Word(String),
    /// Quoted string (single or double).
    Quoted(String),
    LParen,
    RParen,
    LBracket,
    RBracket,
    Comma,
    Semicolon,
    Arrow,
    Eof,
}

#[derive(Debug, Clone)]
struct Token {
    tok: Tok,
    span: Range<usize>,
}

pub(super) struct Lexer<'a> {
    src: &'a str,
    bytes: &'a [u8],
    pub(super) pos: usize,
}

impl<'a> Lexer<'a> {
    pub(super) fn new(src: &'a str) -> Self {
        Self {
            src,
            bytes: src.as_bytes(),
            pos: 0,
        }
    }

    pub(super) fn peek_byte(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    pub(super) fn skip_ws(&mut self) {
        while let Some(b) = self.peek_byte() {
            if b.is_ascii_whitespace() {
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    /// Read a quoted string. The opening quote has already been observed at
    /// `self.pos`. Returns the unquoted contents (with simple `\\`/`\<quote>`
    /// escapes processed) and advances past the closing quote.
    fn read_quoted(&mut self, quote: u8) -> Result<(String, Range<usize>), GrammarBug> {
        let start = self.pos;
        self.pos += 1; // consume opening quote
        let mut out = String::new();
        loop {
            let Some(b) = self.peek_byte() else {
                return Err(GrammarBug {
                    message: format!(
                        "unterminated {} string",
                        if quote == b'\'' {
                            "single-quoted"
                        } else {
                            "double-quoted"
                        }
                    ),
                    span: start..self.pos,
                });
            };
            if b == b'\\' {
                self.pos += 1;
                match self.peek_byte() {
                    Some(escaped) => {
                        out.push(escaped as char);
                        self.pos += 1;
                    }
                    None => {
                        return Err(GrammarBug {
                            message: "trailing backslash in quoted string".into(),
                            span: start..self.pos,
                        });
                    }
                }
            } else if b == quote {
                self.pos += 1;
                return Ok((out, start..self.pos));
            } else {
                let ch = self.src[self.pos..]
                    .chars()
                    .next()
                    .expect("peek_byte established remaining source");
                out.push(ch);
                self.pos += ch.len_utf8();
            }
        }
    }

    /// Read a contiguous run of bare-word characters within a constraint
    /// argument list. Bare words include any byte that is not whitespace, a
    /// quote, or a structural token (`,`, `;`, `(`, `)`). This is permissive
    /// enough to capture regex bodies like `^[a-z]+$` and globs like
    /// `src/**/*.rs`. Patterns containing commas, semicolons, parens, or
    /// whitespace must be quoted.
    ///
    /// `-` is a normal word character except when followed by `>` (the
    /// description arrow), where the `-` terminates the word.
    pub(super) fn read_word(&mut self) -> (String, Range<usize>) {
        let start = self.pos;
        while let Some(b) = self.peek_byte() {
            if matches!(b, b',' | b';' | b'(' | b')' | b'\'' | b'"') || b.is_ascii_whitespace() {
                break;
            }
            if b == b'-' && self.bytes.get(self.pos + 1).copied() == Some(b'>') {
                break;
            }
            self.pos += 1;
        }
        let lex = self.src[start..self.pos].to_string();
        (lex, start..self.pos)
    }

    /// Read an identifier (`[a-z][a-z0-9-]*`). Used in the type-name and
    /// constraint-keyword positions.
    pub(super) fn read_ident(&mut self) -> (String, Range<usize>) {
        let start = self.pos;
        while let Some(b) = self.peek_byte() {
            if b.is_ascii_alphanumeric() || b == b'-' || b == b'_' {
                self.pos += 1;
            } else {
                break;
            }
        }
        (self.src[start..self.pos].to_string(), start..self.pos)
    }

    /// Attempt to lex a number starting at `self.pos`. Recognises
    /// `[-]?digits[.digits]`. Returns `None` if the cursor is not at a number
    /// or at one [`admit_number_spelling`] rejects.
    fn try_read_number(&mut self) -> Option<Token> {
        let start = self.pos;
        let mut cursor = start;
        if self.bytes.get(cursor).copied() == Some(b'-') {
            cursor += 1;
        }
        let digits_start = cursor;
        while let Some(&b) = self.bytes.get(cursor) {
            if b.is_ascii_digit() {
                cursor += 1;
            } else {
                break;
            }
        }
        if cursor == digits_start {
            return None;
        }
        if self.bytes.get(cursor).copied() == Some(b'.') {
            let after_dot = cursor + 1;
            let mut frac = after_dot;
            while let Some(&b) = self.bytes.get(frac) {
                if b.is_ascii_digit() {
                    frac += 1;
                } else {
                    break;
                }
            }
            if frac > after_dot {
                cursor = frac;
            }
        }
        // The next byte (if any) must terminate the number — otherwise this is
        // a bare word that *starts* with digits.
        if let Some(&b) = self.bytes.get(cursor)
            && !(b.is_ascii_whitespace() || matches!(b, b',' | b';' | b')'))
        {
            return None;
        }
        // An inadmissible spelling stays a bare word, so a consumer that needs
        // a number reports why through `arg_to_number` (and `pattern(...)`
        // keeps its digits as text).
        let value = admit_number_spelling(&self.src[start..cursor]).ok()?;
        self.pos = cursor;
        Some(Token {
            tok: Tok::Number(value),
            span: start..cursor,
        })
    }
}

/// Internal lexer error type carrying byte span. The parser converts these
/// into [`SchemaError::Grammar`] with the property name attached.
#[derive(Debug)]
struct GrammarBug {
    message: String,
    span: Range<usize>,
}

// ── Lexical-mode boundaries ──────────────────────────────────────────────
//
// The grammar reads four kinds of text with different rules, and only an
// argument list gives quotes meaning. These functions are the single statement
// of where each non-type text ends; the parser and every structural reader of
// decoded type expressions (`source::scan_expression`, the editor cursor) call
// them rather than restating the rules. Each scans `bytes[start..end]`.

/// Just past the closing quote of the argument string whose opening quote is
/// at `start`, or `None` when it is unterminated. `\` escapes the next byte in
/// either quote style, as [`Lexer::read_quoted`] reads it.
pub(super) fn quoted_end(bytes: &[u8], start: usize, end: usize) -> Option<usize> {
    let quote = bytes[start];
    let mut index = start + 1;
    while index < end {
        match bytes[index] {
            b'\\' => index += 1,
            byte if byte == quote => return Some(index + 1),
            _ => {}
        }
        index += 1;
    }
    None
}

/// The index of the `,` or `}` ending an inline-object property description
/// that starts at `start`, or `end` when neither occurs.
///
/// Description prose nests `{`/`}` and `(`/`)` so a comma or brace inside them
/// does not end it; quotes and brackets are prose. `Err` carries the index of
/// an unbalanced `)`, which the parser rejects.
pub(super) fn inline_description_end(bytes: &[u8], start: usize, end: usize) -> Result<usize, usize> {
    let mut brace_depth = 0usize;
    let mut paren_depth = 0usize;
    for (index, &byte) in bytes.iter().enumerate().take(end).skip(start) {
        match byte {
            b'{' => brace_depth += 1,
            b'}' if brace_depth == 0 => return Ok(index),
            b'}' => brace_depth -= 1,
            b'(' => paren_depth += 1,
            b')' if paren_depth == 0 => return Err(index),
            b')' => paren_depth -= 1,
            b',' if brace_depth == 0 && paren_depth == 0 => return Ok(index),
            _ => {}
        }
    }
    Ok(end)
}

/// The index of the `,`, `}`, `|`, or `->` ending the imported file reference
/// that starts at `start`, or `end` when none occurs. A reference is opaque:
/// no punctuation inside it opens a quote or a nesting level.
pub(super) fn file_reference_end(bytes: &[u8], start: usize, end: usize) -> usize {
    (start..end)
        .find(|&index| match bytes[index] {
            b',' | b'}' | b'|' => true,
            b'-' => index + 1 < end && bytes[index + 1] == b'>',
            _ => false,
        })
        .unwrap_or(end)
}

/// Like [`file_reference_end`] for a reference written inside a group, tuple,
/// or function signature, where `)`, `]`, `;`, and `?` also end it.
fn nested_file_reference_end(bytes: &[u8], start: usize, end: usize) -> usize {
    let top = file_reference_end(bytes, start, end);
    (start..top)
        .find(|&index| matches!(bytes[index], b')' | b']' | b';' | b'?'))
        .unwrap_or(top)
}

/// The index of the `)` closing the constraint argument list whose `(` is at
/// `open`, or `None` when it is unterminated. Quotes, parentheses, brackets,
/// and braces nest, as in an inline example's call and result.
fn argument_list_end(bytes: &[u8], open: usize) -> Option<usize> {
    let mut depth = 0usize;
    let mut index = open;
    while index < bytes.len() {
        match bytes[index] {
            b'\'' | b'"' => {
                index = quoted_end(bytes, index, bytes.len())?;
                continue;
            }
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return (bytes[index] == b')').then_some(index);
                }
            }
            _ => {}
        }
        index += 1;
    }
    None
}

/// The first occurrence of `needle` in `bytes[range]` outside quotes and
/// outside any `(`/`[`/`{` nesting.
fn find_top_level(bytes: &[u8], range: Range<usize>, needle: &[u8]) -> Option<usize> {
    let mut depth = 0usize;
    let mut index = range.start;
    while index < range.end {
        match bytes[index] {
            b'\'' | b'"' => {
                index = quoted_end(bytes, index, range.end)?;
                continue;
            }
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => depth = depth.saturating_sub(1),
            _ if depth == 0 && bytes[index..range.end].starts_with(needle) => return Some(index),
            _ => {}
        }
        index += 1;
    }
    None
}

/// Just past the `>` closing the pattern key whose `<` is at `start`, or
/// `None` when it is unterminated. The key body is opaque up to its first `>`.
pub(super) fn pattern_key_end(bytes: &[u8], start: usize, end: usize) -> Option<usize> {
    (start + 1..end)
        .find(|&index| bytes[index] == b'>')
        .map(|index| index + 1)
}

// ── Parser ───────────────────────────────────────────────────────────────

/// Modes affect which tokens the lexer recognises.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LexMode {
    /// Outside any constraint paren list — type names and structural symbols.
    Outer,
    /// Inside a constraint list — recognise constraint keywords plus `;`.
    ConstraintList,
    /// Inside an arglist — recognise numbers, bare words, and quoted strings.
    ArgList,
}

/// What a constraint list belongs to, which decides how its keywords read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ListOwner {
    /// A built-in type keyword.
    Type(SimplifiedType),
    /// An inline object literal.
    InlineObject,
    /// A named-type reference, whose target is known only after resolution.
    Reference,
    /// A `tuple([...])`.
    Tuple,
    /// A `function(...)` used as an array (`function(...)[]`).
    Function,
    /// A parenthesized union used as an array (`(A | B)[]`).
    Group,
}

impl ListOwner {
    /// Whether the list may open with `alphanumeric` options.
    fn takes_options(self) -> bool {
        matches!(self, Self::Type(SimplifiedType::Alphanumeric) | Self::Reference)
    }

    /// Whether `min`/`max` count characters rather than bound a number.
    fn counts_characters(self) -> bool {
        matches!(self, Self::Type(SimplifiedType::String | SimplifiedType::Alphanumeric))
    }

    fn describe(self, is_array_level: bool) -> String {
        let place = match self {
            Self::Type(ty) => format!("`{}`", ty.as_keyword()),
            Self::InlineObject => "an inline object".to_string(),
            Self::Reference => "a named-type reference".to_string(),
            Self::Tuple => "`tuple`".to_string(),
            Self::Function => "`function`".to_string(),
            Self::Group => "a grouped union".to_string(),
        };
        if is_array_level {
            format!("{place} arrays")
        } else {
            place
        }
    }
}

const MIXED_OPTIONS_MESSAGE: &str = "separate `alphanumeric` options from constraints with `;`, \
     as in `alphanumeric(allow-dashes; min(1))`";

fn unknown_option_message(name: &str) -> String {
    format!(
        "unknown `alphanumeric` option `{name}`; expected one of {}",
        AlphanumericOption::ALL
            .iter()
            .map(|option| format!("`{option}`"))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

/// Whether `name` is a constraint keyword of the grammar, with or without
/// arguments.
fn is_constraint_keyword(name: &str) -> bool {
    matches!(
        name,
        "required"
            | "eager"
            | "not-empty"
            | "integer"
            | "unique"
            | "generated"
            | "deferred"
            | "default"
            | "min"
            | "max"
            | "pattern"
            | "suggest"
            | "match"
            | "scheme"
            | "example"
            | "min-keys"
            | "max-keys"
            | "uncounted-keys"
            | "category"
    )
}

/// The deepest nesting of groups, tuples, and function signatures the parser
/// accepts, so a hostile type expression cannot exhaust the stack.
pub const MAX_TYPE_NESTING: usize = 32;

/// One alternative read by [`Parser::parse_arm`].
enum Arm {
    /// A single type.
    Atom(PropertyAtom),
    /// A parenthesized union without `[]`, whose options join the enclosing
    /// union.
    Group(Vec<PropertyAtom>),
}

/// A union's options as a definition: one option is not a union.
fn arms_to_def(mut arms: Vec<PropertyAtom>) -> PropertyDef {
    if arms.len() == 1 {
        PropertyDef::Single(arms.pop().expect("one arm"))
    } else {
        PropertyDef::Union(arms)
    }
}

struct Parser<'a> {
    property: &'a str,
    src: &'a str,
    lex: Lexer<'a>,
    /// Inline object depth of the type being read.
    object_depth: usize,
    /// Enclosing groups, tuples, and function signatures.
    nesting: usize,
    /// Whether the last union read was one parenthesized group, which a
    /// following `?` applies to as a whole.
    single_group: bool,
}

impl<'a> Parser<'a> {
    fn new(property: &'a str, src: &'a str) -> Self {
        Self {
            property,
            src,
            lex: Lexer::new(src),
            object_depth: 0,
            nesting: 0,
            single_group: false,
        }
    }

    fn enter(&mut self, at: usize) -> Result<(), SchemaError> {
        if self.nesting >= MAX_TYPE_NESTING {
            return self.err(
                format!("type expressions may not nest more than {MAX_TYPE_NESTING} levels deep"),
                at..at + 1,
            );
        }
        self.nesting += 1;
        Ok(())
    }

    fn leave(&mut self) {
        self.nesting -= 1;
    }

    fn starts_with(&self, text: &str) -> bool {
        self.lex.bytes[self.lex.pos..].starts_with(text.as_bytes())
    }

    fn err<T>(&self, message: impl Into<String>, span: Range<usize>) -> Result<T, SchemaError> {
        Err(SchemaError::Grammar {
            property: self.property.to_string(),
            message: message.into(),
            span,
        })
    }

    fn next_token(&mut self, mode: LexMode) -> Result<Token, SchemaError> {
        self.lex.skip_ws();
        let start = self.lex.pos;
        let Some(b) = self.lex.peek_byte() else {
            return Ok(Token {
                tok: Tok::Eof,
                span: start..start,
            });
        };
        // Structural tokens
        match b {
            b'(' => {
                self.lex.pos += 1;
                return Ok(Token {
                    tok: Tok::LParen,
                    span: start..start + 1,
                });
            }
            b')' => {
                self.lex.pos += 1;
                return Ok(Token {
                    tok: Tok::RParen,
                    span: start..start + 1,
                });
            }
            b'[' if !matches!(mode, LexMode::ArgList) => {
                self.lex.pos += 1;
                return Ok(Token {
                    tok: Tok::LBracket,
                    span: start..start + 1,
                });
            }
            b']' if !matches!(mode, LexMode::ArgList) => {
                self.lex.pos += 1;
                return Ok(Token {
                    tok: Tok::RBracket,
                    span: start..start + 1,
                });
            }
            b',' => {
                self.lex.pos += 1;
                return Ok(Token {
                    tok: Tok::Comma,
                    span: start..start + 1,
                });
            }
            b';' => {
                self.lex.pos += 1;
                return Ok(Token {
                    tok: Tok::Semicolon,
                    span: start..start + 1,
                });
            }
            b'-' if self.lex.bytes.get(start + 1).copied() == Some(b'>') => {
                self.lex.pos += 2;
                return Ok(Token {
                    tok: Tok::Arrow,
                    span: start..start + 2,
                });
            }
            b'\'' | b'"' => {
                if matches!(mode, LexMode::ArgList) {
                    let (s, span) = self.lex.read_quoted(b).map_err(|e| SchemaError::Grammar {
                        property: self.property.to_string(),
                        message: e.message,
                        span: e.span,
                    })?;
                    return Ok(Token {
                        tok: Tok::Quoted(s),
                        span,
                    });
                } else {
                    return self.err("unexpected quote", start..start + 1);
                }
            }
            _ => {}
        }

        match mode {
            LexMode::ArgList => {
                if let Some(tok) = self.lex.try_read_number() {
                    return Ok(tok);
                }
                let (word, span) = self.lex.read_word();
                if word.is_empty() {
                    return self.err(
                        format!("unexpected character `{}`", b as char),
                        start..start + 1,
                    );
                }
                Ok(Token {
                    tok: Tok::Word(word),
                    span,
                })
            }
            LexMode::Outer | LexMode::ConstraintList => {
                let (ident, span) = self.lex.read_ident();
                if ident.is_empty() {
                    return self.err(
                        format!("unexpected character `{}`", b as char),
                        start..start + 1,
                    );
                }
                Ok(Token {
                    tok: Tok::Ident(ident),
                    span,
                })
            }
        }
    }

    fn parse_definition(&mut self) -> Result<PropertyDef, SchemaError> {
        let mut arms = self.parse_union_arms()?;
        if let Some(description) = self.parse_top_level_description()? {
            arms.last_mut().expect("a union has an option").description = Some(description);
        }
        if let Some((_, span)) = second_suggestion_span(&arms) {
            return self.err("a property definition may contain at most one `suggest` constraint", span);
        }
        if let Some(message) = duplicate_overload(&arms, None) {
            return self.err(message, 0..self.src.len());
        }
        Ok(arms_to_def(arms))
    }

    /// Reads one or more `|`-separated options. A parenthesized group without
    /// `[]` contributes its own options.
    fn parse_union_arms(&mut self) -> Result<Vec<PropertyAtom>, SchemaError> {
        let mut arms = Vec::new();
        let mut options = 0;
        loop {
            options += 1;
            self.lex.skip_ws();
            let start = self.lex.pos;
            let grouped = match self.parse_arm()? {
                Arm::Atom(atom) => {
                    let categories = atom
                        .property_constraints()
                        .filter(|constraint| matches!(constraint, Constraint::Category(_)))
                        .count();
                    if categories > 1 {
                        return self.err("a type may carry at most one `category`", start..self.lex.pos);
                    }
                    arms.push(atom);
                    false
                }
                Arm::Group(group) => {
                    arms.extend(group);
                    true
                }
            };
            self.lex.skip_ws();
            if self.lex.peek_byte() != Some(b'|') {
                self.single_group = grouped && options == 1;
                return Ok(arms);
            }
            self.lex.pos += 1;
        }
    }

    fn parse_arm(&mut self) -> Result<Arm, SchemaError> {
        self.lex.skip_ws();
        match self.lex.peek_byte() {
            Some(b'{') => {
                let shape = self.parse_inline_object_body(self.object_depth)?;
                // Postfix `()` and `[]` apply to the inline object as a whole.
                let (constraints, is_array, array_constraints) =
                    self.parse_postfix_after_inline_object()?;
                Ok(Arm::Atom(PropertyAtom {
                    ty: TypeExpr::InlineObject(shape),
                    is_array,
                    constraints,
                    array_constraints,
                    description: None,
                }))
            }
            Some(b'(') => self.parse_group(),
            _ => Ok(Arm::Atom(self.parse_type_expr()?)),
        }
    }

    /// Reads `( union )`, optionally followed by `[]` and a container list.
    fn parse_group(&mut self) -> Result<Arm, SchemaError> {
        let open = self.lex.pos;
        self.enter(open)?;
        self.lex.pos += 1;
        let mut arms = self.parse_union_arms()?;
        self.lex.skip_ws();
        self.expect_byte(b')', "expected `)` to close a grouped type")?;
        self.leave();
        self.lex.skip_ws();
        match self.lex.peek_byte() {
            Some(b'(') => {
                return self.err(
                    "a grouped type takes no constraint list; constrain each option, or the list after `[]`",
                    self.lex.pos..self.lex.pos + 1,
                );
            }
            Some(b'[') => {}
            _ => return Ok(Arm::Group(arms)),
        }
        let array_constraints = self.parse_array_suffix(ListOwner::Group)?;
        if arms.len() > 1 {
            return Ok(Arm::Atom(PropertyAtom {
                ty: TypeExpr::Union(arms),
                is_array: true,
                constraints: Vec::new(),
                array_constraints,
                description: None,
            }));
        }
        let mut atom = arms.pop().expect("a group has an option");
        if atom.is_array || (matches!(atom.ty, TypeExpr::InlineObject(_)) && !atom.constraints.is_empty()) {
            return self.err(
                "a list of lists is not supported; group a union of options, as in `(string | number)[]`",
                open..self.lex.pos,
            );
        }
        atom.is_array = true;
        atom.array_constraints = array_constraints;
        Ok(Arm::Atom(atom))
    }

    /// Reads `[]` and its optional container constraint list; the cursor is
    /// at `[`.
    fn parse_array_suffix(&mut self, owner: ListOwner) -> Result<Vec<Constraint>, SchemaError> {
        self.lex.pos += 1;
        self.expect(Tok::RBracket)?;
        self.lex.skip_ws();
        if self.lex.peek_byte() != Some(b'(') {
            return Ok(Vec::new());
        }
        self.lex.pos += 1;
        let constraints = self.parse_constraint_list(owner, true)?;
        self.expect(Tok::RParen)?;
        Ok(constraints)
    }

    /// Reads `tuple( [ elements ] ( ; constraints )? )` after the keyword.
    fn parse_tuple(&mut self, name_span: Range<usize>) -> Result<PropertyAtom, SchemaError> {
        self.lex.skip_ws();
        if self.lex.peek_byte() != Some(b'(') {
            return self.err("`tuple` requires an element list, as in `tuple([string, number])`", name_span);
        }
        self.enter(self.lex.pos)?;
        self.lex.pos += 1;
        self.lex.skip_ws();
        self.expect_byte(b'[', "`tuple(...)` takes a bracketed element list, as in `tuple([string, number])`")?;
        let mut tuple = TupleType::default();
        loop {
            self.lex.skip_ws();
            if self.lex.peek_byte() == Some(b']') {
                self.lex.pos += 1;
                break;
            }
            let element_start = self.lex.pos;
            if tuple.spread.is_some() {
                let message = if self.starts_with("...") {
                    "a tuple may contain only one spread"
                } else {
                    "a spread must be the last tuple element"
                };
                return self.err(message, element_start..element_start + 1);
            }
            if self.starts_with("...") {
                self.lex.pos += 3;
                let mut arms = self.parse_union_arms()?;
                if arms.len() != 1 {
                    return self.err(
                        "a spread names one array type; group a union first, as in `...(string | number)[]`",
                        element_start..self.lex.pos,
                    );
                }
                let atom = arms.pop().expect("one arm");
                if !atom.is_array {
                    return self.err(
                        "a spread must be an array type, as in `...number[]`",
                        element_start..self.lex.pos,
                    );
                }
                if !atom.array_constraints.is_empty() {
                    return self.err(
                        "a spread takes no list constraints; constrain its items, as in `...number(min(0))[]`",
                        element_start..self.lex.pos,
                    );
                }
                self.lex.skip_ws();
                if self.lex.peek_byte() == Some(b'?') {
                    return self.err(
                        "`?` cannot follow a spread: a spread already matches zero or more values",
                        self.lex.pos..self.lex.pos + 1,
                    );
                }
                tuple.spread = Some(Box::new(atom));
            } else {
                self.reject_named_element()?;
                let arms = self.parse_union_arms()?;
                self.lex.skip_ws();
                let optional = self.lex.peek_byte() == Some(b'?');
                if optional {
                    if arms.len() > 1 && !self.single_group {
                        return self.err(
                            "group a union before `?`, as in `(string | number)?`",
                            self.lex.pos..self.lex.pos + 1,
                        );
                    }
                    self.lex.pos += 1;
                    self.lex.skip_ws();
                    if self.lex.peek_byte() == Some(b'|') {
                        return self.err(
                            "group a union before `?`, as in `(string | number)?`",
                            self.lex.pos..self.lex.pos + 1,
                        );
                    }
                } else if tuple.elements.last().is_some_and(|element| element.optional) {
                    return self.err(
                        "a required tuple element cannot follow an optional one",
                        element_start..self.lex.pos,
                    );
                }
                tuple.elements.push(TupleElement {
                    def: arms_to_def(arms),
                    optional,
                });
            }
            self.lex.skip_ws();
            match self.lex.peek_byte() {
                Some(b',') => self.lex.pos += 1,
                Some(b']') => {
                    self.lex.pos += 1;
                    break;
                }
                _ => {
                    return self.err(
                        "expected `,` or `]` in a tuple element list",
                        self.lex.pos..self.lex.pos + 1,
                    );
                }
            }
        }
        self.lex.skip_ws();
        let mut constraints = Vec::new();
        if self.lex.peek_byte() == Some(b';') {
            self.lex.pos += 1;
            constraints = self.parse_constraint_list(ListOwner::Tuple, false)?;
            if let Some(invalid) = constraints.iter().find(|constraint| {
                !matches!(
                    constraint,
                    Constraint::Required
                        | Constraint::Default(_)
                        | Constraint::Generated
                        | Constraint::Deferred
                        | Constraint::Example(_)
                        | Constraint::Category(_)
                        | Constraint::Operands(_)
                )
            }) {
                return self.err(
                    format!("constraint `{}` is not valid on `tuple`", invalid.keyword()),
                    name_span,
                );
            }
        }
        self.lex.skip_ws();
        self.expect_byte(b')', "expected `;` or `)` after a tuple element list")?;
        self.leave();
        self.lex.skip_ws();
        let mut is_array = false;
        let mut array_constraints = Vec::new();
        if self.lex.peek_byte() == Some(b'[') {
            is_array = true;
            array_constraints = self.parse_array_suffix(ListOwner::Tuple)?;
        }
        Ok(PropertyAtom {
            ty: TypeExpr::Tuple(tuple),
            is_array,
            constraints,
            array_constraints,
            description: None,
        })
    }

    /// Rejects `name: T` and `name?: T` as a tuple element.
    fn reject_named_element(&mut self) -> Result<(), SchemaError> {
        let start = self.lex.pos;
        let (name, span) = self.lex.read_ident();
        self.lex.skip_ws();
        if self.lex.peek_byte() == Some(b'?') {
            self.lex.pos += 1;
            self.lex.skip_ws();
        }
        let named = !name.is_empty() && self.lex.peek_byte() == Some(b':');
        self.lex.pos = start;
        if named {
            return self.err("tuple elements cannot be named; a tuple is positional", span);
        }
        Ok(())
    }

    /// Reads `function( constraint ; ... )` after the keyword.
    fn parse_function(&mut self, name_span: Range<usize>) -> Result<PropertyAtom, SchemaError> {
        self.lex.skip_ws();
        if self.lex.peek_byte() != Some(b'(') {
            return self.err(
                "`function` requires a signature, as in `function(parameters(void); returns(string))`",
                name_span,
            );
        }
        self.enter(self.lex.pos)?;
        self.lex.pos += 1;
        let mut parameters = None;
        let mut returns = None;
        let mut fallible = false;
        let mut examples = Vec::new();
        let mut constraints = Vec::new();
        loop {
            self.lex.skip_ws();
            if self.lex.peek_byte() == Some(b')') {
                self.lex.pos += 1;
                break;
            }
            let (keyword, keyword_span) = self.lex.read_ident();
            if keyword.is_empty() {
                return self.err("expected a function constraint", self.lex.pos..self.lex.pos + 1);
            }
            self.lex.skip_ws();
            let has_args = self.lex.peek_byte() == Some(b'(');
            let duplicate = match keyword.as_str() {
                "parameters" => parameters.is_some(),
                "returns" => returns.is_some(),
                "fallible" => fallible,
                "category" => constraints.iter().any(|c| matches!(c, Constraint::Category(_))),
                _ => false,
            };
            if duplicate {
                return self.err(format!("duplicate `{keyword}` constraint on `function`"), keyword_span);
            }
            match (keyword.as_str(), has_args) {
                ("parameters", true) => {
                    self.lex.pos += 1;
                    parameters = Some(self.parse_parameters(keyword_span)?);
                    self.lex.skip_ws();
                    self.expect_byte(b')', "expected `)` to close `parameters`")?;
                }
                ("returns", true) => {
                    self.lex.pos += 1;
                    let arms = self.parse_union_arms()?;
                    self.lex.skip_ws();
                    self.expect_byte(b')', "expected `)` after the return type")?;
                    returns = Some(arms_to_def(arms));
                }
                ("category", true) => {
                    constraints.push(Constraint::Category(self.read_category_name(keyword_span)?));
                }
                ("fallible", false) => fallible = true,
                ("example", true) => match self.parse_example(true, keyword_span)? {
                    ExampleForm::Inline(example) => examples.push(example),
                    ExampleForm::Files(files) => constraints.push(Constraint::Example(files)),
                },
                ("fallible", true) => return self.err("`fallible` does not take arguments", keyword_span),
                ("parameters" | "returns" | "category" | "example", false) => {
                    return self.err(format!("`{keyword}` requires arguments"), keyword_span);
                }
                (other, _) => {
                    return self.err(
                        format!(
                            "unknown constraint `{other}` on `function`; a function takes `parameters`, \
                             `returns`, `category`, `fallible`, and `example`"
                        ),
                        keyword_span,
                    );
                }
            }
            self.lex.skip_ws();
            match self.lex.peek_byte() {
                Some(b';') => self.lex.pos += 1,
                Some(b')') => {
                    self.lex.pos += 1;
                    break;
                }
                _ => {
                    return self.err(
                        "expected `;` or `)` between function constraints",
                        self.lex.pos..self.lex.pos + 1,
                    );
                }
            }
        }
        self.leave();
        let Some(parameters) = parameters else {
            return self.err(
                "a `function` requires `parameters(...)`; write `parameters(void)` for none",
                name_span,
            );
        };
        let Some(returns) = returns else {
            return self.err("a `function` requires `returns(...)`", name_span);
        };
        self.lex.skip_ws();
        let mut is_array = false;
        let mut array_constraints = Vec::new();
        if self.lex.peek_byte() == Some(b'[') {
            is_array = true;
            array_constraints = self.parse_array_suffix(ListOwner::Function)?;
        }
        Ok(PropertyAtom {
            ty: TypeExpr::Function(Box::new(FunctionType {
                parameters,
                returns,
                fallible,
                examples,
            })),
            is_array,
            constraints,
            array_constraints,
            description: None,
        })
    }

    /// Reads the inside of `parameters( ... )`, leaving the cursor at `)`.
    fn parse_parameters(&mut self, keyword_span: Range<usize>) -> Result<FunctionParameters, SchemaError> {
        self.lex.skip_ws();
        if self.lex.peek_byte() == Some(b')') {
            return self.err(
                "`parameters()` is not valid; write `parameters(void)` for a function without parameters",
                keyword_span.start..self.lex.pos + 1,
            );
        }
        let start = self.lex.pos;
        let (word, word_span) = self.lex.read_ident();
        self.lex.skip_ws();
        if word == "void" {
            match self.lex.peek_byte() {
                Some(b')') => return Ok(FunctionParameters::Void),
                Some(b':' | b'?') => {}
                _ => return self.err("`void` cannot be mixed with other parameters", word_span),
            }
        }
        self.lex.pos = start;
        let mut parameters: Vec<FunctionParameter> = Vec::new();
        loop {
            self.lex.skip_ws();
            if self.lex.peek_byte() == Some(b')') && !parameters.is_empty() {
                break;
            }
            let parameter_start = self.lex.pos;
            let rest = self.starts_with("...");
            if rest {
                self.lex.pos += 3;
                self.lex.skip_ws();
            }
            let (name, name_span) = self.lex.read_ident();
            if name.is_empty() {
                return self.err("expected a parameter name", self.lex.pos..self.lex.pos + 1);
            }
            self.lex.skip_ws();
            if name == "void" && !matches!(self.lex.peek_byte(), Some(b':' | b'?')) {
                return self.err("`void` cannot be mixed with other parameters", name_span);
            }
            let optional = self.lex.peek_byte() == Some(b'?');
            if optional {
                if rest {
                    return self.err(
                        "a rest parameter cannot be optional (`?`): it already accepts zero arguments",
                        self.lex.pos..self.lex.pos + 1,
                    );
                }
                self.lex.pos += 1;
                self.lex.skip_ws();
            }
            self.expect_byte(b':', "expected `:` after a parameter name")?;
            let def = arms_to_def(self.parse_union_arms()?);
            if parameters.iter().any(|parameter| parameter.name == name) {
                return self.err(format!("duplicate parameter name `{name}`"), name_span);
            }
            if parameters.last().is_some_and(|parameter| parameter.rest) {
                let message = if rest {
                    "a function may have only one rest parameter"
                } else {
                    "a rest parameter must be the last parameter"
                };
                return self.err(message, parameter_start..self.lex.pos);
            }
            if !optional && !rest && parameters.iter().any(|parameter| parameter.optional) {
                return self.err(
                    "a required parameter cannot follow an optional one",
                    parameter_start..self.lex.pos,
                );
            }
            parameters.push(FunctionParameter { name, def, optional, rest });
            self.lex.skip_ws();
            match self.lex.peek_byte() {
                Some(b',') => self.lex.pos += 1,
                Some(b')') => break,
                _ => {
                    return self.err(
                        "expected `,` or `)` between parameters",
                        self.lex.pos..self.lex.pos + 1,
                    );
                }
            }
        }
        Ok(FunctionParameters::List(parameters))
    }

    /// Reads `( Name )` after `category`: the trimmed raw text up to `)`.
    fn read_category_name(&mut self, keyword_span: Range<usize>) -> Result<String, SchemaError> {
        let open = self.lex.pos;
        let Some(close) = (open + 1..self.src.len()).find(|&index| self.lex.bytes[index] == b')') else {
            return self.err(
                "unterminated `category(...)` (missing `)`)",
                keyword_span.start..self.src.len(),
            );
        };
        let name = self.src[open + 1..close].trim();
        if name.is_empty() {
            return self.err("`category` requires a name", keyword_span.start..close + 1);
        }
        if name.contains('(') {
            return self.err("a category name cannot contain `(`", open + 1..close);
        }
        self.lex.pos = close + 1;
        Ok(name.to_string())
    }

    /// Reads `(<verb>-short)` or `(<verb>-long)` after `operands`.
    fn read_operands_name(&mut self, keyword_span: Range<usize>) -> Result<String, SchemaError> {
        let open = self.lex.pos;
        let Some(close) = (open + 1..self.src.len()).find(|&index| self.lex.bytes[index] == b')') else {
            return self.err(
                "unterminated `operands(...)` (missing `)`)",
                keyword_span.start..self.src.len(),
            );
        };
        let name = self.src[open + 1..close].trim();
        if super::types::operands_verb(name).is_none() {
            return self.err(
                "`operands` takes a generated verb type name, `<verb>-short` or `<verb>-long`",
                keyword_span.start..close + 1,
            );
        }
        self.lex.pos = close + 1;
        Ok(name.to_string())
    }

    /// Reads `( ... )` after `example`: an inline `call => result` example,
    /// which only a function signature's own constraint list allows
    /// (`inline_allowed`), or a list of example files.
    fn parse_example(
        &mut self,
        inline_allowed: bool,
        keyword_span: Range<usize>,
    ) -> Result<ExampleForm, SchemaError> {
        let open = self.lex.pos;
        let bytes = self.lex.bytes;
        let Some(close) = argument_list_end(bytes, open) else {
            return self.err(
                "unterminated `example(...)` (missing `)`)",
                keyword_span.start..self.src.len(),
            );
        };
        let Some(arrow) = find_top_level(bytes, open + 1..close, b"=>") else {
            self.lex.pos += 1;
            let args = self.parse_arglist()?;
            self.expect(Tok::RParen)?;
            if args.is_empty() {
                return self.err("`example` requires at least one file reference", keyword_span);
            }
            return Ok(ExampleForm::Files(args.into_iter().map(|arg| arg.lex).collect()));
        };
        if !inline_allowed {
            return self.err(
                "an inline `example(call => result)` is valid only on a `function` type; \
                 other types take example files, as in `example(./example.yaml)`",
                keyword_span.start..close + 1,
            );
        }
        let call_span = trim_span(self.src, open + 1..arrow);
        let after = arrow + 2..close;
        let (result_range, display_only) = match find_top_level(bytes, after.clone(), b";") {
            None => (after, None),
            Some(semicolon) => {
                let tail = trim_span(self.src, semicolon + 1..close);
                let reason = self.src[tail.clone()]
                    .strip_prefix("display-only")
                    .map(str::trim_start)
                    .and_then(|rest| rest.strip_prefix(':'))
                    .map(str::trim);
                match reason {
                    Some(reason) if !reason.is_empty() => {
                        (after.start..semicolon, Some(reason.to_string()))
                    }
                    _ => {
                        return self.err(
                            "expected `display-only: REASON` after `;` in an inline example",
                            tail,
                        );
                    }
                }
            }
        };
        let expected_span = trim_span(self.src, result_range);
        let call = &self.src[call_span.clone()];
        match parse_spanned(call) {
            Ok(expression) if matches!(expression.kind, SpannedExprKind::FunctionCall { .. }) => {}
            Ok(_) => {
                return self.err(
                    "an example call must be a function call, as in `min(3, 5)`",
                    call_span,
                );
            }
            Err(error) => {
                return self.err(
                    format!("example call `{call}` is not a valid expression: {error}"),
                    call_span,
                );
            }
        }
        let result = &self.src[expected_span.clone()];
        let expected = if result == "error" {
            ExampleOutcome::Error
        } else {
            let value = parse_spanned(result)
                .map_err(|error| error.to_string())
                .and_then(|expression| literal_to_json(&expression, result));
            match value {
                Ok(value) => ExampleOutcome::Value(value),
                Err(reason) => {
                    return self.err(
                        format!(
                            "an example result must be a literal (`3`, `\"3\"`, `[1, 2]`, \
                             `{{a: 1}}`, `null`, `true`) or `error`: {reason}"
                        ),
                        expected_span,
                    );
                }
            }
        };
        self.lex.pos = close + 1;
        Ok(ExampleForm::Inline(CallExample {
            call: call.to_string(),
            call_span,
            expected,
            expected_span,
            display_only,
        }))
    }

    /// Parses the optional `(value_constraints)`, `[]`, and
    /// `[](array_constraints)` suffix that follows an inline object.
    fn parse_postfix_after_inline_object(
        &mut self,
    ) -> Result<(Vec<Constraint>, bool, Vec<Constraint>), SchemaError> {
        let mut value_constraints = Vec::new();
        self.lex.skip_ws();
        if self.lex.peek_byte() == Some(b'(') {
            self.lex.pos += 1;
            value_constraints = self.parse_constraint_list(ListOwner::InlineObject, false)?;
            self.expect(Tok::RParen)?;
        }

        let mut is_array = false;
        let mut array_constraints = Vec::new();
        self.lex.skip_ws();
        if self.lex.peek_byte() == Some(b'[') {
            if !value_constraints.is_empty() {
                return self.err(
                    "inline object array constraints must follow `[]`",
                    self.lex.pos..self.lex.pos + 1,
                );
            }
            self.lex.pos += 1;
            self.expect(Tok::RBracket)?;
            is_array = true;
            self.lex.skip_ws();
            if self.lex.peek_byte() == Some(b'(') {
                self.lex.pos += 1;
                array_constraints = self.parse_constraint_list(ListOwner::InlineObject, true)?;
                self.expect(Tok::RParen)?;
            }
        }
        Ok((value_constraints, is_array, array_constraints))
    }

    /// Top-level `-> description` consumes the rest of the input.
    fn parse_top_level_description(&mut self) -> Result<Option<String>, SchemaError> {
        self.lex.skip_ws();
        let after = self.lex.pos;
        if self.lex.peek_byte() == Some(b'?') {
            return self.err(
                "`?` marks an optional tuple element or function parameter; a property is \
                 optional unless it is `required`",
                after..after + 1,
            );
        }
        let next = self.next_token(LexMode::Outer)?;
        match next.tok {
            Tok::Eof => Ok(None),
            Tok::Arrow => {
                let rest = self.src[self.lex.pos..].trim().to_string();
                if rest.is_empty() {
                    return self.err("`->` must be followed by a description", next.span);
                }
                self.lex.pos = self.src.len();
                Ok(Some(rest))
            }
            other => self.err(
                format!("expected end of expression or `->`, found `{:?}`", other),
                after..next.span.end,
            ),
        }
    }

    /// Parses the body of an inline object literal — the comma-separated
    /// property definitions enclosed by `{` and `}`.
    ///
    /// `depth` is the current nesting level; it must not exceed
    /// [`MAX_INLINE_OBJECT_DEPTH`] (Decision #11).
    fn parse_inline_object_body(&mut self, depth: usize) -> Result<SchemaShape, SchemaError> {
        if depth >= MAX_INLINE_OBJECT_DEPTH {
            return self.err(
                format!(
                    "inline objects may not nest more than {MAX_INLINE_OBJECT_DEPTH} levels deep",
                ),
                self.lex.pos..self.lex.pos,
            );
        }
        // Consume opening `{`.
        self.expect_byte(b'{', "expected `{` to start an inline object")?;
        let mut properties = indexmap::IndexMap::new();
        // Pattern (dictionary) keys are collected separately so literal keys
        // retain precedence during conversion (Feature C).
        let mut pattern_keys: Vec<PatternKeyDef> = Vec::new();
        self.lex.skip_ws();
        // Empty object `{}` is legal.
        if self.lex.peek_byte() == Some(b'}') {
            self.lex.pos += 1;
            return Ok(SchemaShape {
                properties,
                pattern_keys,
                ..Default::default()
            });
        }
        loop {
            self.lex.skip_ws();
            // A key is either a `<...>` pattern key or a literal identifier
            // (unquoted, per Decision #8).
            if self.lex.peek_byte() == Some(b'<') {
                let (key, _key_span) = self.read_inline_pattern_key()?;
                self.lex.skip_ws();
                self.expect_byte(b':', "expected `:` after inline object pattern key")?;
                self.lex.skip_ws();
                let def = self.parse_inline_object_property_value(depth)?;
                pattern_keys.push(PatternKeyDef { key, def });
            } else {
                let (name, name_span) = self.read_inline_object_ident()?;
                if name.is_empty() {
                    return self.err("expected an inline object property name", name_span);
                }
                self.lex.skip_ws();
                self.expect_byte(b':', "expected `:` after inline object property name")?;
                self.lex.skip_ws();
                // Read the property's type expression string (recursively).
                let def = self.parse_inline_object_property_value(depth)?;
                if properties.insert(name.clone(), def).is_some() {
                    return self.err(
                        format!("duplicate property `{name}` in inline object"),
                        name_span,
                    );
                }
            }
            self.lex.skip_ws();
            match self.lex.peek_byte() {
                Some(b',') => {
                    self.lex.pos += 1;
                    self.lex.skip_ws();
                    // Trailing comma is legal: `{ foo: string, }`.
                    if self.lex.peek_byte() == Some(b'}') {
                        self.lex.pos += 1;
                        return Ok(SchemaShape {
                            properties,
                            pattern_keys,
                            ..Default::default()
                        });
                    }
                }
                Some(b'}') => {
                    self.lex.pos += 1;
                    return Ok(SchemaShape {
                        properties,
                        pattern_keys,
                        ..Default::default()
                    });
                }
                Some(other) => {
                    let span = self.lex.pos..self.lex.pos + 1;
                    return self.err(
                        format!(
                            "expected `,` or `}}` between inline object properties, found `{}`",
                            other as char
                        ),
                        span,
                    );
                }
                None => {
                    let span = self.lex.pos..self.lex.pos;
                    return self.err(
                        "unterminated inline object (missing `}`)",
                        span,
                    );
                }
            }
        }
    }

    /// Reads a single inline object property name: ASCII alphanumerics plus
    /// `-` and `_`, including leading digits (Decision #8). Returns an empty
    /// string and the span of the offending byte when the name starts with a
    /// disallowed character; callers decide whether to treat this as a
    /// "expected name" error.
    fn read_inline_object_ident(&mut self) -> Result<(String, Range<usize>), SchemaError> {
        let start = self.lex.pos;
        while let Some(b) = self.lex.peek_byte() {
            if b.is_ascii_alphanumeric() || b == b'-' || b == b'_' {
                self.lex.pos += 1;
            } else {
                break;
            }
        }
        let name = self.src[start..self.lex.pos].to_string();
        Ok((name, start..self.lex.pos))
    }

    /// Reads a `<...>` pattern-key token starting at the current `<` and parses
    /// it into a [`PatternKey`] (Feature C). The body runs to the first `>`.
    fn read_inline_pattern_key(&mut self) -> Result<(PatternKey, Range<usize>), SchemaError> {
        let start = self.lex.pos; // positioned at `<`
        let len = self.lex.bytes.len();
        let Some(pos) = pattern_key_end(self.lex.bytes, start, len) else {
            return self.err("unterminated pattern key (missing `>`)", start..len);
        };
        let raw = &self.src[start..pos];
        let key = PatternKey::parse(raw).map_err(|message| SchemaError::Grammar {
            property: self.property.to_string(),
            message,
            span: start..pos,
        })?;
        self.lex.pos = pos;
        Ok((key, start..pos))
    }

    /// Parses a single property value inside an inline object: the
    /// `type_expr_string` form (a `type_expr` optionally followed by a
    /// `-> description` whose description text terminates at the next
    /// top-level `,` or `}` for the current inline object).
    fn parse_inline_object_property_value(
        &mut self,
        depth: usize,
    ) -> Result<PropertyDef, SchemaError> {
        // The property value is a type expression or `|` union, possibly
        // with a postfix `-> description` that ends at the outer object's
        // next `,` or `}` and describes the last option.
        let saved = self.object_depth;
        self.object_depth = depth + 1;
        let arms = self.parse_union_arms();
        self.object_depth = saved;
        let mut arms = arms?;
        self.lex.skip_ws();
        if self.starts_with("->") {
            self.lex.pos += 2;
            let description = self.read_inline_object_description()?;
            arms.last_mut().expect("a union has an option").description = Some(description);
        }
        if let Some((_, span)) = second_suggestion_span(&arms) {
            return self.err("a property definition may contain at most one `suggest` constraint", span);
        }
        if let Some(message) = duplicate_overload(&arms, None) {
            return self.err(message, 0..self.src.len());
        }
        Ok(arms_to_def(arms))
    }

    /// Reads a description text from the current position up to (but not
    /// consuming) the next `,` or `}` at the current inline-object depth, as
    /// [`inline_description_end`] bounds it.
    ///
    /// Decision #9 says: descriptions terminate at the next top-level comma
    /// or closing brace in the *current* inline object body. The caller is
    /// responsible for ensuring the terminator is actually present.
    fn read_inline_object_description(&mut self) -> Result<String, SchemaError> {
        let start = self.lex.pos;
        let bytes = self.lex.bytes;
        let pos = match inline_description_end(bytes, start, bytes.len()) {
            Ok(pos) => pos,
            Err(pos) => {
                return self.err("unbalanced `)` in inline object description", pos..pos + 1);
            }
        };
        let raw = &self.src[start..pos];
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return self.err(
                "`->` must be followed by a description",
                start..pos,
            );
        }
        self.lex.pos = pos;
        Ok(trimmed.to_string())
    }

    fn expect_byte(&mut self, expected: u8, msg: &str) -> Result<(), SchemaError> {
        let pos = self.lex.pos;
        if self.lex.peek_byte() == Some(expected) {
            self.lex.pos += 1;
            Ok(())
        } else {
            self.err(
                format!(
                    "{msg}, found `{}`",
                    self.lex.peek_byte().map(|b| b as char).unwrap_or('\0')
                ),
                pos..pos,
            )
        }
    }

    fn parse_type_expr(&mut self) -> Result<PropertyAtom, SchemaError> {
        let name_tok = self.next_token(LexMode::Outer)?;
        let (name, name_span) = match name_tok.tok {
            Tok::Ident(s) => (s, name_tok.span),
            Tok::Eof => return self.err("expected a type name", 0..0),
            other => {
                return self.err(
                    format!("expected a type name, found `{:?}`", other),
                    name_tok.span,
                );
            }
        };
        // An explicit `@` reference is detected before the keyword check so
        // that `string@this` names a local type called `string`.
        if self.looks_like_import_suffix() {
            return self.parse_named_reference(name, true);
        }

        match name.as_str() {
            "tuple" => return self.parse_tuple(name_span),
            "function" => return self.parse_function(name_span),
            _ => {}
        }

        // Built-in keywords win; any other identifier is a bare local
        // reference, identical to `Name@this`.
        let Some(ty) = SimplifiedType::from_keyword(&name) else {
            return self.parse_named_reference(name, false);
        };

        // Optional first paren list (item constraints, or value constraints
        // for non-array atoms).
        let mut item_constraints = Vec::new();
        self.lex.skip_ws();
        if self.lex.peek_byte() == Some(b'(') {
            // consume (
            self.lex.pos += 1;
            item_constraints = self.parse_constraint_list(ListOwner::Type(ty), false)?;
            self.expect(Tok::RParen)?;
        }

        // Optional `[]` and array constraints.
        let mut is_array = false;
        let mut array_constraints = Vec::new();
        self.lex.skip_ws();
        if self.lex.peek_byte() == Some(b'[') {
            self.lex.pos += 1;
            self.expect(Tok::RBracket)?;
            is_array = true;
            self.lex.skip_ws();
            if self.lex.peek_byte() == Some(b'(') {
                self.lex.pos += 1;
                array_constraints = self.parse_constraint_list(ListOwner::Type(ty), true)?;
                self.expect(Tok::RParen)?;
            }
        }

        self.validate_semantic_constraints(
            ty,
            &item_constraints,
            false,
            name_span.clone(),
        )?;
        self.validate_semantic_constraints(
            ty,
            &array_constraints,
            true,
            name_span.clone(),
        )?;

        // Enum requires members.
        if matches!(ty, SimplifiedType::Enum)
            && !item_constraints
                .iter()
                .any(|c| matches!(c, Constraint::Members(_)))
        {
            return self.err(
                "`enum` requires a constraint list with at least one member",
                name_span,
            );
        }

        // A `literal` atom without a `LiteralValue` (bare `literal` or
        // `literal()`) is rejected the same way `enum` without members is.
        if matches!(ty, SimplifiedType::Literal)
            && !item_constraints
                .iter()
                .any(|c| matches!(c, Constraint::LiteralValue(_)))
        {
            return self.err("literal requires a value", name_span);
        }

        Ok(PropertyAtom {
            ty: TypeExpr::Primitive(ty),
            is_array,
            constraints: item_constraints,
            array_constraints,
            description: None,
        })
    }

    fn validate_semantic_constraints(
        &self,
        ty: SimplifiedType,
        constraints: &[Constraint],
        is_array_level: bool,
        span: Range<usize>,
    ) -> Result<(), SchemaError> {
        if !matches!(ty, SimplifiedType::TypeDefinition | SimplifiedType::Schema) {
            return Ok(());
        }

        for constraint in constraints {
            match constraint {
                Constraint::Required
                | Constraint::Generated
                | Constraint::Deferred
                | Constraint::Category(_)
                | Constraint::Operands(_) => {}
                Constraint::Default(value) => {
                    let Some(default) = value.as_str() else {
                        return self.err(
                            format!(
                                "`default` value is not valid on `{}`; expected a scalar semantic value",
                                ty.as_keyword()
                            ),
                            span.clone(),
                        );
                    };
                    let value = serde_yaml_ng::Value::String(default.to_string());
                    let result = match ty {
                        SimplifiedType::TypeDefinition => {
                            super::parse_property_definition(self.property, &value).map(|_| ())
                        }
                        SimplifiedType::Schema => {
                            super::parse_schema_declaration(&value).map(|_| ())
                        }
                        _ => unreachable!("semantic type checked above"),
                    };
                    if let Err(error) = result {
                        return self.err(
                            format!(
                                "`default` value is not valid on `{}`: {error}",
                                ty.as_keyword()
                            ),
                            span.clone(),
                        );
                    }
                }
                Constraint::MinItems(_) | Constraint::MaxItems(_) | Constraint::Unique
                    if is_array_level => {}
                other => {
                    return self.err(
                        format!(
                            "constraint `{}` is not valid on `{}`{}",
                            other.keyword(),
                            ty.as_keyword(),
                            if is_array_level { " arrays" } else { "" }
                        ),
                        span.clone(),
                    );
                }
            }
        }
        Ok(())
    }

    /// Pure lookahead: returns `true` when the token stream immediately after
    /// the base identifier matches the import postfix
    /// `('(' … ')')? ('[]' ('(' … ')')?)?` followed by `@`. Does not consume
    /// input and does not interpret constraint contents (so it never raises
    /// spurious errors on non-import expressions like `enum(a, b, c)`).
    fn looks_like_import_suffix(&self) -> bool {
        let bytes = self.lex.bytes;
        let mut pos = self.lex.pos;
        let skip_ws = |pos: &mut usize| {
            while *pos < bytes.len() && bytes[*pos].is_ascii_whitespace() {
                *pos += 1;
            }
        };
        // optional balanced `(...)`, quote-aware; `false` when unbalanced
        let skip_parens = |pos: &mut usize| -> bool {
            if bytes.get(*pos) != Some(&b'(') {
                return true;
            }
            *pos += 1;
            let mut depth = 1usize;
            let mut quote: Option<u8> = None;
            while *pos < bytes.len() && depth > 0 {
                let b = bytes[*pos];
                match quote {
                    Some(q) => {
                        if b == b'\\' {
                            *pos += 1;
                        } else if b == q {
                            quote = None;
                        }
                    }
                    None => match b {
                        b'\'' | b'"' => quote = Some(b),
                        b'(' => depth += 1,
                        b')' => depth -= 1,
                        _ => {}
                    },
                }
                *pos += 1;
            }
            depth == 0
        };
        skip_ws(&mut pos);
        if !skip_parens(&mut pos) {
            return false;
        }
        skip_ws(&mut pos);
        // optional `[]`, then its optional container list
        if bytes.get(pos) == Some(&b'[') {
            if bytes.get(pos + 1) != Some(&b']') {
                return false;
            }
            pos += 2;
            skip_ws(&mut pos);
            if !skip_parens(&mut pos) {
                return false;
            }
            skip_ws(&mut pos);
        }
        bytes.get(pos) == Some(&b'@')
    }

    /// Parses the named-reference postfix after the base `name` has been read:
    /// `('(' constraints ')')? ('[]' ('(' constraints ')')?)?`, followed by
    /// `'@' fileref` when `explicit` is set. Without `explicit` the result is
    /// the local reference [`TypeExpr::local`], the same value `name@this`
    /// produces.
    ///
    /// As on a primitive atom, the list before `[]` lands in `constraints`
    /// and the list after it in `array_constraints`. Both are parsed with a
    /// neutral type because their meaning depends on the target, which is
    /// known only after resolution; the resolver decides which constraints a
    /// list before `[]` may carry. A leading `alphanumeric` option list is
    /// kept as [`Constraint::Options`] for the resolver to compare with the
    /// target's options.
    fn parse_named_reference(
        &mut self,
        name: String,
        explicit: bool,
    ) -> Result<PropertyAtom, SchemaError> {
        let mut constraints = Vec::new();
        self.lex.skip_ws();
        if self.lex.peek_byte() == Some(b'(') {
            self.lex.pos += 1;
            constraints = self.parse_constraint_list(ListOwner::Reference, false)?;
            self.expect(Tok::RParen)?;
        }

        let mut is_array = false;
        let mut array_constraints = Vec::new();
        self.lex.skip_ws();
        if self.lex.peek_byte() == Some(b'[') {
            self.lex.pos += 1;
            self.expect(Tok::RBracket)?;
            is_array = true;
            self.lex.skip_ws();
            if self.lex.peek_byte() == Some(b'(') {
                self.lex.pos += 1;
                array_constraints = self.parse_constraint_list(ListOwner::Reference, true)?;
                self.expect(Tok::RParen)?;
            }
        }

        let ty = if explicit {
            self.lex.skip_ws();
            self.expect_byte(b'@', "expected `@` in an imported type reference")?;
            let reference = self.read_fileref()?;
            TypeExpr::Imported { name, reference }
        } else {
            TypeExpr::local(name)
        };

        Ok(PropertyAtom {
            ty,
            is_array,
            constraints,
            array_constraints,
            description: None,
        })
    }

    /// Reads the file reference to the right of `@`. Runs to the end of the
    /// current sub-expression, as [`file_reference_end`] bounds it: `->`
    /// (description arrow), a `,`, `}`, or `|`, or end of input; inside a
    /// group, tuple, or signature also `)`, `]`, `;`, or `?`.
    fn read_fileref(&mut self) -> Result<String, SchemaError> {
        self.lex.skip_ws();
        let start = self.lex.pos;
        let bytes = self.lex.bytes;
        let pos = if self.nesting > 0 {
            nested_file_reference_end(bytes, start, bytes.len())
        } else {
            file_reference_end(bytes, start, bytes.len())
        };
        let raw = self.src[start..pos].trim();
        if raw.is_empty() {
            return self.err("expected a file reference after `@`", start..pos);
        }
        self.lex.pos = pos;
        Ok(raw.to_string())
    }

    fn expect(&mut self, expected: Tok) -> Result<Token, SchemaError> {
        let tok = self.next_token(LexMode::Outer)?;
        if std::mem::discriminant(&tok.tok) == std::mem::discriminant(&expected) {
            Ok(tok)
        } else {
            self.err(
                format!(
                    "expected `{}`, found `{:?}`",
                    describe_tok(&expected),
                    tok.tok
                ),
                tok.span,
            )
        }
    }

    fn parse_constraint_list(
        &mut self,
        owner: ListOwner,
        is_array_level: bool,
    ) -> Result<Vec<Constraint>, SchemaError> {
        // Special-case enum: the first ident inside the list may be either
        // the start of a positional members list or a constraint keyword
        // followed by `(...)`. We always parse a sequence of `member` items
        // separated by `,` until we either hit `;` (constraint separator), `)`
        // (end of list), or a `(` immediately following an ident (constraint
        // call).
        let mut constraints = Vec::new();
        // Peek to see if this is empty: `()`.
        self.lex.skip_ws();
        if self.lex.peek_byte() == Some(b')') {
            return Ok(constraints);
        }

        // Enum gets the special path that accumulates positional members.
        // Item constraints for an enum are members; the array-level paren
        // list, if present, is just regular array constraints (min/max/etc.).
        if owner == ListOwner::Type(SimplifiedType::Enum) && !is_array_level {
            let (members, mut tail) = self.parse_enum_members()?;
            if !members.is_empty() {
                constraints.push(Constraint::Members(members));
            }
            constraints.append(&mut tail);
            return Ok(constraints);
        }

        // `literal` mirrors the enum delimiter rules but takes exactly one
        // positional (typed) value, then `;`-separated constraints. The
        // array-level paren list is regular array constraints.
        if owner == ListOwner::Type(SimplifiedType::Literal) && !is_array_level {
            return self.parse_literal_value();
        }

        // `alphanumeric` options precede the first `;`; a list that opens
        // with a constraint is constraints only.
        let mut separated = false;
        if owner.takes_options() && !is_array_level {
            let options;
            (options, separated) = self.parse_option_prefix(owner)?;
            if !options.is_empty() {
                constraints.push(Constraint::Options(options));
                if !separated {
                    return Ok(constraints);
                }
            }
        }

        loop {
            self.lex.skip_ws();
            let constraint_start = self.lex.pos;
            let constraint = self.parse_one_constraint(owner, is_array_level)?;
            if matches!(constraint, Constraint::Suggest(_))
                && constraints
                    .iter()
                    .any(|existing| matches!(existing, Constraint::Suggest(_)))
            {
                return self.err(
                    "a property definition may contain at most one `suggest` constraint",
                    constraint_start..self.lex.pos,
                );
            }
            constraints.push(constraint);
            self.lex.skip_ws();
            match self.lex.peek_byte() {
                Some(b';') => {
                    self.lex.pos += 1;
                    separated = true;
                    continue;
                }
                Some(b')') => break,
                Some(b',') if owner.takes_options() && !is_array_level && !separated => {
                    return self.err(MIXED_OPTIONS_MESSAGE, self.lex.pos..self.lex.pos + 1);
                }
                Some(other) => {
                    let span = self.lex.pos..self.lex.pos + 1;
                    return self.err(
                        format!(
                            "expected `;` or `)` between constraints, found `{}`",
                            other as char
                        ),
                        span,
                    );
                }
                None => {
                    let span = self.lex.pos..self.lex.pos;
                    return self.err("unterminated constraint list (missing `)`)", span);
                }
            }
        }
        Ok(constraints)
    }

    /// Parses the `alphanumeric` options that open a constraint list: a
    /// comma-separated list of option keywords ended by `;` or `)`. Returns
    /// the options in authored order and whether the `;` was consumed. When
    /// the list opens with anything but an option keyword, nothing is
    /// consumed and the list is read as constraints only, except that an
    /// unrecognized word in an `alphanumeric` list is an unknown option.
    fn parse_option_prefix(
        &mut self,
        owner: ListOwner,
    ) -> Result<(Vec<AlphanumericOption>, bool), SchemaError> {
        self.lex.skip_ws();
        let start = self.lex.pos;
        let (first, first_span) = self.lex.read_ident();
        self.lex.pos = start;
        if AlphanumericOption::from_keyword(&first).is_none() {
            if owner == ListOwner::Type(SimplifiedType::Alphanumeric)
                && !first.is_empty()
                && !is_constraint_keyword(&first)
            {
                return self.err(unknown_option_message(&first), first_span);
            }
            return Ok((Vec::new(), false));
        }

        let mut options = Vec::new();
        loop {
            self.lex.skip_ws();
            let tok = self.next_token(LexMode::ConstraintList)?;
            let Tok::Ident(name) = tok.tok else {
                return self.err(
                    format!("expected an `alphanumeric` option, found `{:?}`", tok.tok),
                    tok.span,
                );
            };
            match AlphanumericOption::from_keyword(&name) {
                Some(option) if options.contains(&option) => {
                    return self.err(format!("repeated `alphanumeric` option `{name}`"), tok.span);
                }
                Some(option) => options.push(option),
                None if is_constraint_keyword(&name) => {
                    return self.err(MIXED_OPTIONS_MESSAGE, tok.span);
                }
                None => return self.err(unknown_option_message(&name), tok.span),
            }
            self.lex.skip_ws();
            match self.lex.peek_byte() {
                Some(b',') => self.lex.pos += 1,
                Some(b';') => {
                    self.lex.pos += 1;
                    return Ok((options, true));
                }
                Some(b')') => return Ok((options, false)),
                Some(b'(') => {
                    return self.err(
                        format!("`alphanumeric` option `{name}` does not take arguments"),
                        tok.span,
                    );
                }
                Some(other) => {
                    return self.err(
                        format!(
                            "expected `,`, `;`, or `)` after an `alphanumeric` option, found `{}`",
                            other as char
                        ),
                        self.lex.pos..self.lex.pos + 1,
                    );
                }
                None => {
                    return self.err(
                        "unterminated constraint list (missing `)`)",
                        self.lex.pos..self.lex.pos,
                    );
                }
            }
        }
    }

    /// Parses members until we either hit the end of the list or a `;`
    /// separator, in which case we hand off to the regular constraint parser
    /// for any trailing `default(...)` / `required` constraints.
    fn parse_enum_members(&mut self) -> Result<(Vec<String>, Vec<Constraint>), SchemaError> {
        let mut members = Vec::new();
        let mut tail = Vec::new();

        loop {
            self.lex.skip_ws();
            // Look ahead: if the next token is an ident immediately followed
            // by `(`, treat it as a constraint call (e.g. `default(draft)`).
            // Otherwise it's a positional member.
            let saved = self.lex.pos;
            let tok = self.next_token(LexMode::ArgList)?;
            // Determine whether this token is a constraint call.
            let ident_name: Option<&str> = match &tok.tok {
                Tok::Ident(s) => Some(s.as_str()),
                _ => None,
            };
            let is_constraint_call = if let Some(name) = ident_name {
                let lookahead_pos = self.lex.pos;
                self.lex.skip_ws();
                let next_byte = self.lex.peek_byte();
                self.lex.pos = lookahead_pos;
                next_byte == Some(b'(') || name == "required"
            } else {
                false
            };

            if is_constraint_call {
                // Rewind and let the standard constraint parser handle it.
                self.lex.pos = saved;
                let constraint =
                    self.parse_one_constraint(ListOwner::Type(SimplifiedType::Enum), false)?;
                tail.push(constraint);
                self.lex.skip_ws();
                match self.lex.peek_byte() {
                    Some(b';') => {
                        self.lex.pos += 1;
                        // After we've moved into "tail" mode, all subsequent
                        // constraints are parsed normally.
                        return Ok((
                            members,
                            self.parse_remaining_constraints(
                                ListOwner::Type(SimplifiedType::Enum),
                                tail,
                            )?,
                        ));
                    }
                    Some(b')') => return Ok((members, tail)),
                    Some(other) => {
                        let span = self.lex.pos..self.lex.pos + 1;
                        return self.err(
                            format!(
                                "expected `;` or `)` after constraint, found `{}`",
                                other as char
                            ),
                            span,
                        );
                    }
                    None => {
                        let span = self.lex.pos..self.lex.pos;
                        return self.err("unterminated constraint list (missing `)`)", span);
                    }
                }
            }

            // Positional member
            let member = match tok.tok {
                Tok::Ident(s) => s,
                Tok::Word(s) => {
                    if let Err(NumberTextError::Admission(reason)) = admit_number_spelling(&s) {
                        return self.err(format!("enum member `{s}` {reason}"), tok.span);
                    }
                    s
                }
                Tok::Quoted(s) => s,
                Tok::Number(n) => format_number(n),
                other => {
                    return self.err(
                        format!("expected enum member, found `{:?}`", other),
                        tok.span,
                    );
                }
            };
            members.push(member);
            self.lex.skip_ws();
            match self.lex.peek_byte() {
                Some(b',') => {
                    self.lex.pos += 1;
                    continue;
                }
                Some(b';') => {
                    self.lex.pos += 1;
                    return Ok((
                        members,
                        self.parse_remaining_constraints(
                            ListOwner::Type(SimplifiedType::Enum),
                            tail,
                        )?,
                    ));
                }
                Some(b')') => return Ok((members, tail)),
                Some(other) => {
                    let span = self.lex.pos..self.lex.pos + 1;
                    return self.err(
                        format!(
                            "expected `,`, `;`, or `)` in enum members, found `{}`",
                            other as char
                        ),
                        span,
                    );
                }
                None => {
                    let span = self.lex.pos..self.lex.pos;
                    return self.err("unterminated enum members (missing `)`)", span);
                }
            }
        }
    }

    fn parse_remaining_constraints(
        &mut self,
        owner: ListOwner,
        mut acc: Vec<Constraint>,
    ) -> Result<Vec<Constraint>, SchemaError> {
        loop {
            self.lex.skip_ws();
            if self.lex.peek_byte() == Some(b')') {
                return Ok(acc);
            }
            let c = self.parse_one_constraint(owner, false)?;
            acc.push(c);
            self.lex.skip_ws();
            match self.lex.peek_byte() {
                Some(b';') => {
                    self.lex.pos += 1;
                    continue;
                }
                Some(b')') => return Ok(acc),
                Some(other) => {
                    let span = self.lex.pos..self.lex.pos + 1;
                    return self.err(
                        format!(
                            "expected `;` or `)` between constraints, found `{}`",
                            other as char
                        ),
                        span,
                    );
                }
                None => {
                    let span = self.lex.pos..self.lex.pos;
                    return self.err("unterminated constraint list (missing `)`)", span);
                }
            }
        }
    }

    /// Parses a `literal(...)` item-constraint list: exactly one positional
    /// value (typed per the Q1 ruling) followed by optional `;`-separated
    /// constraints. Positioned just after the opening `(` with the leading
    /// whitespace already skipped and a non-`)` byte guaranteed.
    fn parse_literal_value(&mut self) -> Result<Vec<Constraint>, SchemaError> {
        self.lex.skip_ws();
        let tok = self.next_token(LexMode::ArgList)?;
        let value = match tok.tok {
            // Quoted values are always strings (quoting opts out of typing).
            Tok::Quoted(s) => serde_json::Value::String(s),
            // Bare tokens are typed like YAML would type the frontmatter value.
            Tok::Number(_) | Tok::Word(_) => {
                let raw = self.src[tok.span.clone()].to_string();
                match classify_bare_literal(&raw) {
                    BareLiteral::Value(value) => value,
                    BareLiteral::RejectedNull => {
                        return self.err(
                            "bare `null` is not a valid literal value; quote it \
                             (`literal('null')`) or drop the optional key",
                            tok.span,
                        );
                    }
                    BareLiteral::RejectedNumber(reason) => {
                        return self.err(format!("literal value `{raw}` {reason}"), tok.span);
                    }
                }
            }
            other => {
                return self.err(
                    format!("expected a literal value, found `{other:?}`"),
                    tok.span,
                );
            }
        };

        let constraints = vec![Constraint::LiteralValue(value)];
        self.lex.skip_ws();
        match self.lex.peek_byte() {
            Some(b',') => self.err(
                "`literal` takes exactly one value; use `enum(...)` for multiple allowed values",
                self.lex.pos..self.lex.pos + 1,
            ),
            Some(b';') => {
                self.lex.pos += 1;
                // Tail constraints accumulate after the leading `LiteralValue`.
                self.parse_remaining_constraints(ListOwner::Type(SimplifiedType::Literal), constraints)
            }
            Some(b')') => Ok(constraints),
            Some(other) => self.err(
                format!("expected `;` or `)` after the literal value, found `{}`", other as char),
                self.lex.pos..self.lex.pos + 1,
            ),
            None => self.err(
                "unterminated constraint list (missing `)`)",
                self.lex.pos..self.lex.pos,
            ),
        }
    }

    fn parse_one_constraint(
        &mut self,
        owner: ListOwner,
        is_array_level: bool,
    ) -> Result<Constraint, SchemaError> {
        let kw_tok = self.next_token(LexMode::ConstraintList)?;
        let (keyword, kw_span) = match kw_tok.tok {
            Tok::Ident(s) => (s, kw_tok.span),
            other => {
                return self.err(
                    format!("expected a constraint keyword, found `{:?}`", other),
                    kw_tok.span,
                );
            }
        };

        if AlphanumericOption::from_keyword(&keyword).is_some() {
            let message = if owner.takes_options() && !is_array_level {
                format!("`alphanumeric` option `{keyword}` must be written before the first `;`")
            } else {
                format!(
                    "`{keyword}` is an `alphanumeric` option and is not valid on {}",
                    owner.describe(is_array_level)
                )
            };
            return self.err(message, kw_span);
        }

        // Distinguish bare-keyword constraints from those taking arguments.
        self.lex.skip_ws();
        let has_args = self.lex.peek_byte() == Some(b'(');

        let constraint = match (keyword.as_str(), has_args) {
            ("required", false) => Constraint::Required,
            ("eager", false) => Constraint::Eager,
            ("not-empty", false) => Constraint::NotEmpty,
            ("integer", false) => Constraint::Integer,
            ("unique", false) => Constraint::Unique,
            ("generated", false) => Constraint::Generated,
            ("deferred", false) => Constraint::Deferred,
            ("required", true) => {
                return self.err("`required` does not take arguments", kw_span);
            }
            ("eager", true) => {
                return self.err("`eager` does not take arguments", kw_span);
            }
            ("generated", true) => {
                return self.err("`generated` does not take arguments", kw_span);
            }
            ("deferred", true) => {
                return self.err("`deferred` does not take arguments", kw_span);
            }
            ("default", true) => {
                self.lex.pos += 1; // consume (
                let args = self.parse_arglist()?;
                self.expect(Tok::RParen)?;
                if args.len() != 1 {
                    return self.err(
                        format!("`default` takes exactly 1 argument, got {}", args.len()),
                        kw_span,
                    );
                }
                let value = arg_to_json(&args[0])
                    .or_else(|reason| self.err(inadmissible("default", &args[0], reason), args[0].span.clone()))?;
                if owner == ListOwner::Type(SimplifiedType::Null)
                    && !is_array_level
                    && !value.is_null()
                {
                    return self.err(
                        format!(
                            "`default({value})` is not valid on `null`; a `null` default must be null"
                        ),
                        args[0].span.clone(),
                    );
                }
                Constraint::Default(value)
            }
            ("min", true) => {
                self.lex.pos += 1;
                let args = self.parse_arglist()?;
                self.expect(Tok::RParen)?;
                if args.len() != 1 {
                    return self.err(
                        format!("`min` takes exactly 1 argument, got {}", args.len()),
                        kw_span,
                    );
                }
                let n = self.numeric_arg("min", &args[0])?;
                if is_array_level {
                    let n_usize = number_to_usize(n, "min", &args[0], self.property)?;
                    Constraint::MinItems(n_usize)
                } else if owner.counts_characters() {
                    let n_usize = number_to_usize(n, "min", &args[0], self.property)?;
                    Constraint::MinLen(n_usize)
                } else {
                    Constraint::Min(n)
                }
            }
            ("max", true) => {
                self.lex.pos += 1;
                let args = self.parse_arglist()?;
                self.expect(Tok::RParen)?;
                if args.len() != 1 {
                    return self.err(
                        format!("`max` takes exactly 1 argument, got {}", args.len()),
                        kw_span,
                    );
                }
                let n = self.numeric_arg("max", &args[0])?;
                if is_array_level {
                    let n_usize = number_to_usize(n, "max", &args[0], self.property)?;
                    Constraint::MaxItems(n_usize)
                } else if owner.counts_characters() {
                    let n_usize = number_to_usize(n, "max", &args[0], self.property)?;
                    Constraint::MaxLen(n_usize)
                } else {
                    Constraint::Max(n)
                }
            }
            ("pattern", true) => {
                self.lex.pos += 1;
                let args = self.parse_arglist()?;
                self.expect(Tok::RParen)?;
                if args.len() != 1 {
                    return self.err(
                        format!("`pattern` takes exactly 1 argument, got {}", args.len()),
                        kw_span,
                    );
                }
                Constraint::Pattern(args[0].lex.clone())
            }
            ("suggest", true) => {
                self.lex.pos += 1;
                let args = self.parse_arglist()?;
                self.expect(Tok::RParen)?;
                if args.is_empty() {
                    return self.err(
                        "`suggest` requires at least one candidate",
                        kw_span.start..self.lex.pos,
                    );
                }
                // A reference's target is not known yet: its candidates are
                // read as strings until the resolver reinterprets them.
                let target = match owner {
                    ListOwner::Type(
                        ty @ (SimplifiedType::String
                        | SimplifiedType::Number
                        | SimplifiedType::Alphanumeric),
                    ) if !is_array_level => ty,
                    ListOwner::Reference if !is_array_level => SimplifiedType::String,
                    _ => {
                        return self.err(
                            format!("`suggest` is not valid on {}", owner.describe(is_array_level)),
                            kw_span.start..self.lex.pos,
                        );
                    }
                };

                let mut candidates = Vec::with_capacity(args.len());
                for arg in args {
                    let candidate = interpret_suggestion(&arg, target);
                    if candidates.iter().any(|existing: &SuggestionCandidate| {
                        existing.interpreted == candidate.interpreted
                    }) {
                        return self.err("duplicate suggestion candidate", candidate.span);
                    }
                    candidates.push(candidate);
                }
                Constraint::Suggest(candidates)
            }
            ("match", true) => {
                self.lex.pos += 1;
                let args = self.parse_arglist()?;
                self.expect(Tok::RParen)?;
                if args.is_empty() {
                    return self.err("`match` requires at least one glob", kw_span);
                }
                let globs: Vec<String> = args.iter().map(|a| a.lex.clone()).collect();
                if let Some((index, error)) = crate::markdown::schemas::file_match::definition_error(&globs) {
                    let span = index.map_or(kw_span.start..self.lex.pos, |index| args[index].span.clone());
                    return self.err(format!("`match`: {error}"), span);
                }
                Constraint::Match(globs)
            }
            ("scheme", true) => {
                self.lex.pos += 1;
                let args = self.parse_arglist()?;
                self.expect(Tok::RParen)?;
                if args.is_empty() {
                    return self.err("`scheme` requires at least one scheme", kw_span);
                }
                let schemes = args
                    .into_iter()
                    .map(|a| a.lex.to_ascii_lowercase())
                    .collect();
                Constraint::Scheme(schemes)
            }
            ("example", true) => match self.parse_example(false, kw_span)? {
                ExampleForm::Files(refs) => Constraint::Example(refs),
                ExampleForm::Inline(_) => unreachable!("an inline example outside a signature is an error"),
            },
            ("category", true) => Constraint::Category(self.read_category_name(kw_span)?),
            ("operands", true) => Constraint::Operands(self.read_operands_name(kw_span)?),
            ("min-keys", true) => {
                self.lex.pos += 1;
                let args = self.parse_arglist()?;
                self.expect(Tok::RParen)?;
                if args.len() != 1 {
                    return self.err(
                        format!("`min-keys` takes exactly 1 argument, got {}", args.len()),
                        kw_span,
                    );
                }
                let n = self.numeric_arg("min-keys", &args[0])?;
                Constraint::MinKeys(number_to_usize(n, "min-keys", &args[0], self.property)?)
            }
            ("max-keys", true) => {
                self.lex.pos += 1;
                let args = self.parse_arglist()?;
                self.expect(Tok::RParen)?;
                if args.len() != 1 {
                    return self.err(
                        format!("`max-keys` takes exactly 1 argument, got {}", args.len()),
                        kw_span,
                    );
                }
                let n = self.numeric_arg("max-keys", &args[0])?;
                Constraint::MaxKeys(number_to_usize(n, "max-keys", &args[0], self.property)?)
            }
            ("uncounted-keys", true) => {
                self.lex.pos += 1;
                let args = self.parse_arglist()?;
                self.expect(Tok::RParen)?;
                let keys: Vec<String> = args.into_iter().map(|arg| arg.lex).collect();
                if let Err(message) = super::types::check_uncounted_keys(&keys) {
                    return self.err(message, kw_span);
                }
                Constraint::UncountedKeys(keys)
            }
            ("min-keys" | "max-keys" | "uncounted-keys" | "example" | "suggest" | "category" | "operands", false) => {
                return self.err(format!("`{keyword}` requires arguments"), kw_span);
            }
            (other, has_args_) => {
                let suffix = if has_args_ { "(...)" } else { "" };
                return self.err(format!("unknown constraint `{other}{suffix}`"), kw_span);
            }
        };

        Ok(constraint)
    }

    fn parse_arglist(&mut self) -> Result<Vec<Arg>, SchemaError> {
        let mut args = Vec::new();
        self.lex.skip_ws();
        if self.lex.peek_byte() == Some(b')') {
            return Ok(args);
        }
        loop {
            self.lex.skip_ws();
            let tok = self.next_token(LexMode::ArgList)?;
            let arg = match tok.tok {
                Tok::Number(n) => Arg {
                    lex: self.src[tok.span.clone()].to_string(),
                    number: Some(n),
                    quoted: false,
                    span: tok.span,
                },
                Tok::Word(s) | Tok::Ident(s) => Arg {
                    lex: s,
                    number: None,
                    quoted: false,
                    span: tok.span,
                },
                Tok::Quoted(s) => Arg {
                    lex: s,
                    number: None,
                    quoted: true,
                    span: tok.span,
                },
                other => {
                    return self.err(format!("expected argument, found `{:?}`", other), tok.span);
                }
            };
            args.push(arg);
            self.lex.skip_ws();
            match self.lex.peek_byte() {
                Some(b',') => {
                    self.lex.pos += 1;
                    continue;
                }
                Some(b')') => return Ok(args),
                Some(other) => {
                    let span = self.lex.pos..self.lex.pos + 1;
                    return self.err(
                        format!(
                            "expected `,` or `)` in argument list, found `{}`",
                            other as char
                        ),
                        span,
                    );
                }
                None => {
                    let span = self.lex.pos..self.lex.pos;
                    return self.err("unterminated argument list (missing `)`)", span);
                }
            }
        }
    }
}

#[derive(Debug)]
struct Arg {
    lex: String,
    number: Option<f64>,
    quoted: bool,
    span: Range<usize>,
}

/// What `example(...)` held.
enum ExampleForm {
    /// `example(./a.yaml, ./b.yaml)`.
    Files(Vec<String>),
    /// `example(call => result)`.
    Inline(CallExample),
}

/// A constraint argument read as a number: `Ok(None)` for text that is not
/// one, and the admission failure for a number Darkmatter cannot hold.
/// `inf` and `NaN` spellings are reported as not finite rather than as text.
fn arg_to_number(arg: &Arg) -> Result<Option<f64>, NumericAdmissionError> {
    if let Some(number) = arg.number {
        return Ok(Some(number));
    }
    if arg.quoted {
        return Ok(None);
    }
    match admit_number_spelling(&arg.lex) {
        Ok(number) => Ok(Some(number)),
        Err(NumberTextError::Admission(reason)) => Err(reason),
        Err(NumberTextError::NotANumber) => match arg.lex.parse::<f64>() {
            Ok(value) if !value.is_finite() => Err(NumericAdmissionError::NotFinite),
            _ => Ok(None),
        },
    }
}

fn inadmissible(keyword: &str, arg: &Arg, reason: NumericAdmissionError) -> String {
    format!("`{keyword}({})` is not a usable number: {reason}", arg.lex)
}

impl Parser<'_> {
    /// The numeric argument of `keyword`, admitted exactly.
    fn numeric_arg(&self, keyword: &str, arg: &Arg) -> Result<f64, SchemaError> {
        match arg_to_number(arg) {
            Ok(Some(number)) => Ok(number),
            Ok(None) => self.err(
                format!("`{keyword}` requires a numeric argument, got `{}`", arg.lex),
                arg.span.clone(),
            ),
            Err(reason) => self.err(inadmissible(keyword, arg, reason), arg.span.clone()),
        }
    }
}

/// The span of `range` without surrounding whitespace.
fn trim_span(src: &str, range: Range<usize>) -> Range<usize> {
    let text = &src[range.clone()];
    let start = range.start + (text.len() - text.trim_start().len());
    let end = range.end - (text.len() - text.trim_end().len());
    start..end.max(start)
}

/// The index into `arms` and span of a second `suggest(...)` candidate list,
/// which a property definition may not hold.
///
/// The span is the one recorded on that arm's first candidate, so it is
/// relative to whatever text the arm was parsed from.
pub(crate) fn second_suggestion_span(arms: &[PropertyAtom]) -> Option<(usize, Range<usize>)> {
    arms.iter()
        .enumerate()
        .flat_map(|(index, atom)| atom.constraints.iter().map(move |constraint| (index, constraint)))
        .filter_map(|(index, constraint)| match constraint {
            Constraint::Suggest(candidates) => candidates.first().map(|candidate| (index, candidate)),
            _ => None,
        })
        .nth(1)
        .map(|(index, candidate)| (index, candidate.span.clone()))
}

/// Converts the parsed result of an inline example to JSON. Only literals
/// qualify; numbers are admitted from their written spelling in `src`.
fn literal_to_json(expression: &SpannedExpr, src: &str) -> Result<serde_json::Value, String> {
    use serde_json::Value;
    let number = |spelling: &str| -> Result<Value, String> {
        let value = admit_number_spelling(spelling).map_err(|error| format!("`{spelling}` {error}"))?;
        json_number(value)
            .map(Value::Number)
            .ok_or_else(|| format!("`{spelling}` is not finite"))
    };
    match &expression.kind {
        SpannedExprKind::StringLiteral(text) => Ok(Value::String(text.clone())),
        SpannedExprKind::BoolLiteral(flag) => Ok(Value::Bool(*flag)),
        SpannedExprKind::NumberLiteral(_) => number(&src[expression.span.clone()]),
        SpannedExprKind::UnaryMinus(inner) if matches!(inner.kind, SpannedExprKind::NumberLiteral(_)) => {
            number(&format!("-{}", &src[inner.span.clone()]))
        }
        SpannedExprKind::Variable(name) if name == "null" => Ok(Value::Null),
        SpannedExprKind::ArrayLiteral(items) => items
            .iter()
            .map(|item| literal_to_json(item, src))
            .collect::<Result<Vec<_>, _>>()
            .map(Value::Array),
        SpannedExprKind::ObjectLiteral(entries) => entries
            .iter()
            .map(|(key, value)| Ok((key.value.clone(), literal_to_json(value, src)?)))
            .collect::<Result<serde_json::Map<_, _>, String>>()
            .map(Value::Object),
        _ => Err(format!("`{}` is not a literal", &src[expression.span.clone()])),
    }
}

fn interpret_suggestion(arg: &Arg, ty: SimplifiedType) -> SuggestionCandidate {
    interpret_suggestion_parts(&arg.lex, arg.span.clone(), ty)
}

fn interpret_suggestion_parts(
    decoded: &str,
    span: Range<usize>,
    ty: SimplifiedType,
) -> SuggestionCandidate {
    if matches!(ty, SimplifiedType::String | SimplifiedType::Alphanumeric) {
        return SuggestionCandidate {
            decoded: decoded.to_string(),
            interpreted: serde_json::Value::String(decoded.to_string()),
            canonical_decimal: None,
            span,
        };
    }

    let Some(canonical) = normalize_simple_decimal(decoded) else {
        return SuggestionCandidate {
            decoded: decoded.to_string(),
            interpreted: serde_json::Value::String(decoded.to_string()),
            canonical_decimal: None,
            span,
        };
    };
    let interpreted = serde_json::Number::from_str(&canonical)
        .ok()
        .filter(|number| {
            expand_json_number(&number.to_string())
                .and_then(|serialized| normalize_simple_decimal(&serialized))
                .as_deref()
                == Some(canonical.as_str())
        })
        .map(serde_json::Value::Number)
        .unwrap_or_else(|| serde_json::Value::String(canonical.clone()));

    SuggestionCandidate {
        decoded: decoded.to_string(),
        interpreted,
        canonical_decimal: Some(canonical),
        span,
    }
}

pub(crate) fn reinterpret_suggestion_candidates(
    candidates: &mut [SuggestionCandidate],
    ty: SimplifiedType,
    property: &str,
) -> Result<(), SchemaError> {
    for index in 0..candidates.len() {
        let candidate = interpret_suggestion_parts(
            &candidates[index].decoded,
            candidates[index].span.clone(),
            ty,
        );
        if candidates[..index]
            .iter()
            .any(|existing| existing.interpreted == candidate.interpreted)
        {
            return Err(SchemaError::Grammar {
                property: property.to_string(),
                message: "duplicate suggestion candidate".into(),
                span: candidate.span,
            });
        }
        candidates[index] = candidate;
    }
    Ok(())
}

/// Normalizes the feature's simple-decimal grammar without converting through
/// a machine numeric type.
pub fn normalize_simple_decimal(input: &str) -> Option<String> {
    let (negative, unsigned) = match input.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, input),
    };
    let (integer, fraction) = match unsigned.split_once('.') {
        Some((integer, fraction)) => (integer, Some(fraction)),
        None => (unsigned, None),
    };
    if integer.is_empty()
        || !integer.bytes().all(|byte| byte.is_ascii_digit())
        || fraction.is_some_and(|value| {
            value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit())
        })
    {
        return None;
    }

    let integer = integer.trim_start_matches('0');
    let integer = if integer.is_empty() { "0" } else { integer };
    let fraction = fraction.map(|value| value.trim_end_matches('0'));
    let is_zero = integer == "0" && fraction.is_none_or(str::is_empty);
    let mut canonical = String::new();
    if negative && !is_zero {
        canonical.push('-');
    }
    canonical.push_str(integer);
    if let Some(fraction) = fraction.filter(|value| !value.is_empty()) {
        canonical.push('.');
        canonical.push_str(fraction);
    }
    Some(canonical)
}

fn expand_json_number(input: &str) -> Option<String> {
    let Some(exponent_at) = input.find(['e', 'E']) else {
        return Some(input.to_string());
    };
    let (mantissa, exponent) = input.split_at(exponent_at);
    let exponent: i64 = exponent[1..].parse().ok()?;
    let (sign, mantissa) = mantissa
        .strip_prefix('-')
        .map_or(("", mantissa), |rest| ("-", rest));
    let decimal_at = mantissa.find('.').unwrap_or(mantissa.len());
    let digits: String = mantissa.chars().filter(|ch| *ch != '.').collect();
    let decimal_at = i64::try_from(decimal_at).ok()?.checked_add(exponent)?;

    let mut expanded = String::from(sign);
    if decimal_at <= 0 {
        expanded.push_str("0.");
        expanded.extend(std::iter::repeat_n('0', usize::try_from(-decimal_at).ok()?));
        expanded.push_str(&digits);
    } else {
        let decimal_at = usize::try_from(decimal_at).ok()?;
        if decimal_at >= digits.len() {
            expanded.push_str(&digits);
            expanded.extend(std::iter::repeat_n('0', decimal_at - digits.len()));
        } else {
            expanded.push_str(&digits[..decimal_at]);
            expanded.push('.');
            expanded.push_str(&digits[decimal_at..]);
        }
    }
    Some(expanded)
}

fn number_to_usize(
    n: f64,
    constraint: &str,
    arg: &Arg,
    property: &str,
) -> Result<usize, SchemaError> {
    if n < 0.0 || n.fract() != 0.0 {
        return Err(SchemaError::Grammar {
            property: property.to_string(),
            message: format!(
                "`{constraint}` requires a non-negative integer, got `{}`",
                arg.lex
            ),
            span: arg.span.clone(),
        });
    }
    Ok(n as usize)
}

/// A `default(...)` argument as JSON: an admitted number, `true`, `false`,
/// `null`, or text. A bare number spelling Darkmatter cannot hold is an error
/// rather than text.
fn arg_to_json(arg: &Arg) -> Result<serde_json::Value, NumericAdmissionError> {
    if let Some(number) = arg.number {
        return Ok(json_number(number)
            .map(serde_json::Value::Number)
            .unwrap_or_else(|| serde_json::Value::String(arg.lex.clone())));
    }
    if arg.quoted {
        return Ok(serde_json::Value::String(arg.lex.clone()));
    }
    Ok(match arg.lex.as_str() {
        "true" => serde_json::Value::Bool(true),
        "false" => serde_json::Value::Bool(false),
        "null" => serde_json::Value::Null,
        text => {
            if let Err(NumberTextError::Admission(reason)) = admit_number_spelling(text) {
                return Err(reason);
            }
            serde_json::Value::String(text.to_string())
        }
    })
}

/// Classification of an unquoted `literal(...)` value token (Q1 typing rule).
///
/// A quoted value is always a string and never routed through here.
pub(super) enum BareLiteral {
    /// A typed scalar: `Bool`, a JSON `Number`, or a plain `String` (the raw
    /// token spelling that fails the boolean/number shape tests).
    Value(serde_json::Value),
    /// Bare `null` — always an authoring mistake for a literal (an optional
    /// property already accepts `null`), rejected with an actionable error.
    RejectedNull,
    /// A number spelling Darkmatter cannot hold exactly.
    RejectedNumber(NumericAdmissionError),
}

/// Types an unquoted `literal(...)` value token the way YAML would type the
/// frontmatter value it validates: bare `true`/`false` → boolean, a
/// numberlike token → number, everything else → string. Bare `null` is
/// rejected ([`BareLiteral::RejectedNull`]).
pub(super) fn classify_bare_literal(raw: &str) -> BareLiteral {
    match raw {
        "true" => BareLiteral::Value(serde_json::Value::Bool(true)),
        "false" => BareLiteral::Value(serde_json::Value::Bool(false)),
        "null" => BareLiteral::RejectedNull,
        _ => match literal_number(raw) {
            Some(Ok(value)) => BareLiteral::Value(value),
            Some(Err(reason)) => BareLiteral::RejectedNumber(reason),
            None => BareLiteral::Value(serde_json::Value::String(raw.to_string())),
        },
    }
}

/// Returns `true` when a bare (unquoted) string value round-trips through the
/// literal grammar as the *same* string — i.e. it is neither `true`/`false`/
/// `null` nor numberlike. Serialization uses this to decide whether a literal
/// string value must be quoted so it does not re-type on re-parse.
pub(super) fn bare_literal_round_trips_as_string(value: &str) -> bool {
    !matches!(value, "true" | "false" | "null") && literal_number(value).is_none()
}

/// Parses a numberlike-shaped bare token into a typed JSON number admitted
/// exactly: a whole value is a JSON integer, and a whole value no `f64` holds
/// is the admission error. `None` for any spelling that is not
/// `^-?\d+(\.\d+)?$` and for leading-zero integers (`007`), which stay
/// strings (no octal surprises, per the Q1 ruling).
fn literal_number(raw: &str) -> Option<Result<serde_json::Value, NumericAdmissionError>> {
    if !is_literal_numberlike(raw) {
        return None;
    }
    Some(match admit_number_spelling(raw) {
        Ok(value) => json_number(value)
            .map(serde_json::Value::Number)
            .ok_or(NumericAdmissionError::NotFinite),
        Err(NumberTextError::Admission(reason)) => Err(reason),
        Err(NumberTextError::NotANumber) => Err(NumericAdmissionError::NotFinite),
    })
}

/// Mirrors `^-?\d+(\.\d+)?$` (an optional `-`, one or more integer digits, an
/// optional `.` plus one or more fraction digits) and additionally rejects a
/// multi-digit integer part with a leading zero (`007`, `-08`).
fn is_literal_numberlike(s: &str) -> bool {
    let unsigned = s.strip_prefix('-').unwrap_or(s);
    let (integer, fraction) = match unsigned.split_once('.') {
        Some((integer, fraction)) => (integer, Some(fraction)),
        None => (unsigned, None),
    };
    if integer.is_empty() || !integer.bytes().all(|b| b.is_ascii_digit()) {
        return false;
    }
    if integer.len() > 1 && integer.starts_with('0') {
        return false;
    }
    match fraction {
        Some(fraction) => !fraction.is_empty() && fraction.bytes().all(|b| b.is_ascii_digit()),
        None => true,
    }
}

fn describe_tok(t: &Tok) -> &'static str {
    match t {
        Tok::Ident(_) => "identifier",
        Tok::Number(_) => "number",
        Tok::Word(_) => "word",
        Tok::Quoted(_) => "quoted-string",
        Tok::LParen => "(",
        Tok::RParen => ")",
        Tok::LBracket => "[",
        Tok::RBracket => "]",
        Tok::Comma => ",",
        Tok::Semicolon => ";",
        Tok::Arrow => "->",
        Tok::Eof => "<eof>",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(input: &str) -> PropertyAtom {
        parse_type_expr("test", input).expect("parse failed")
    }

    fn parse_err(input: &str) -> SchemaError {
        parse_type_expr("test", input).expect_err("expected parse error")
    }

    #[test]
    fn operands_names_a_generated_verb_type_and_lowers_to_its_annotation() {
        let atom = parse("tuple([number]; operands(number-short))");
        assert_eq!(atom.constraints, [Constraint::Operands("number-short".to_string())]);
        assert_eq!(
            parse("unknown[](max(0); operands(stop_now-long))").array_constraints.last(),
            Some(&Constraint::Operands("stop_now-long".to_string()))
        );
        let schema = to_json_schema_for_test("number(operands(number-short); required)");
        assert_eq!(schema["x-darkmatter-operands"], "number-short");
        // On the typed node, never on an optional property's `null` wrapper.
        let optional = to_json_schema_for_test("tuple([number]; operands(number-short))");
        assert!(optional.get("x-darkmatter-operands").is_none(), "{optional}");
        let typed: Vec<&serde_json::Value> = optional["anyOf"].as_array().expect("a null wrapper").iter().collect();
        assert_eq!(typed[1]["x-darkmatter-operands"], "number-short", "{optional}");

        for bad in ["number(operands(number))", "number(operands(-short))", "number(operands(a b-long))", "number(operands)"] {
            let error = parse_err(bad).to_string();
            assert!(error.contains("operands"), "{bad}: {error}");
        }
    }

    fn to_json_schema_for_test(ty: &str) -> serde_json::Value {
        let yaml: serde_yaml_ng::Value =
            serde_yaml_ng::from_str(&format!("root: \"{ty}\"\n")).expect("the fixture is YAML");
        let schema = crate::markdown::schemas::simplified::parse_yaml_schema(&yaml).expect("the schema parses");
        let json = crate::markdown::schemas::simplified::to_json_schema(&schema).expect("the schema lowers");
        json["properties"]["root"].clone()
    }

    #[test]
    fn parses_bare_type() {
        let atom = parse("string");
        assert_eq!(atom.ty, TypeExpr::Primitive(SimplifiedType::String));
        assert!(!atom.is_array);
        assert!(atom.constraints.is_empty());
        assert!(atom.array_constraints.is_empty());
    }

    #[test]
    fn parses_each_type_keyword() {
        for ty in [
            SimplifiedType::String,
            SimplifiedType::Date,
            SimplifiedType::DateTime,
            SimplifiedType::Time,
            SimplifiedType::Number,
            SimplifiedType::NumberLike,
            SimplifiedType::Boolean,
            SimplifiedType::Boolish,
            SimplifiedType::Object,
            SimplifiedType::File,
            SimplifiedType::Url,
            SimplifiedType::Email,
            SimplifiedType::Yaml,
            SimplifiedType::Json,
            SimplifiedType::Unknown,
            SimplifiedType::Null,
            SimplifiedType::Alphanumeric,
            SimplifiedType::Shell,
        ] {
            let atom = parse(ty.as_keyword());
            assert_eq!(atom.ty, TypeExpr::Primitive(ty));
        }
    }

    #[test]
    fn any_parses_as_unknown_and_new_keywords_take_constraints() {
        assert_eq!(parse("any"), parse("unknown"));
        assert_eq!(parse("any(required)"), parse("unknown(required)"));
        assert_eq!(parse("null(required)").constraints, vec![Constraint::Required]);
        assert_eq!(parse("shell(required)").constraints, vec![Constraint::Required]);
        assert!(parse("shell[]").is_array);
        assert!(parse("unknown[](required)").is_array);
        assert!(parse("null[]").is_array);
    }

    #[test]
    fn keyword_collisions_need_an_explicit_local_reference() {
        for keyword in ["unknown", "null", "alphanumeric", "shell"] {
            assert!(matches!(parse(keyword).ty, TypeExpr::Primitive(_)), "{keyword}");
            assert_eq!(parse(&format!("{keyword}@this")).ty, TypeExpr::local(keyword));
        }
        assert_eq!(
            parse("shell@./shell.yaml").ty,
            TypeExpr::Imported {
                name: "shell".into(),
                reference: "./shell.yaml".into(),
            }
        );
    }

    fn options_of(atom: &PropertyAtom) -> Vec<AlphanumericOption> {
        atom.constraints
            .iter()
            .find_map(|constraint| match constraint {
                Constraint::Options(options) => Some(options.clone()),
                _ => None,
            })
            .unwrap_or_default()
    }

    #[test]
    fn alphanumeric_options_precede_the_separator() {
        let atom = parse("alphanumeric(allow-slashes, allow-periods, block-double-periods; min(1); max(40))");
        assert_eq!(
            atom.constraints,
            vec![
                Constraint::Options(vec![
                    AlphanumericOption::AllowSlashes,
                    AlphanumericOption::AllowPeriods,
                    AlphanumericOption::BlockDoublePeriods,
                ]),
                Constraint::MinLen(1),
                Constraint::MaxLen(40),
            ]
        );
        let only = parse("alphanumeric(leading-alpha, allow-underscores)");
        assert_eq!(
            options_of(&only),
            [AlphanumericOption::LeadingAlpha, AlphanumericOption::AllowUnderscores]
        );
        assert_eq!(only.constraints.len(), 1);
    }

    #[test]
    fn alphanumeric_without_options_reads_constraints_only() {
        assert_eq!(parse("alphanumeric(min(1))").constraints, vec![Constraint::MinLen(1)]);
        assert_eq!(parse("alphanumeric(required)").constraints, vec![Constraint::Required]);
        assert_eq!(
            parse("alphanumeric(required; not-empty)").constraints,
            vec![Constraint::Required, Constraint::NotEmpty]
        );
        assert_eq!(parse("alphanumeric()").constraints, vec![]);
        assert_eq!(parse("alphanumeric").alphanumeric_options(), Some(Default::default()));
    }

    #[test]
    fn alphanumeric_options_on_an_array_bind_to_items() {
        let atom = parse("alphanumeric(allow-dashes)[](min(1))");
        assert!(atom.is_array);
        assert_eq!(options_of(&atom), [AlphanumericOption::AllowDashes]);
        assert_eq!(atom.array_constraints, vec![Constraint::MinItems(1)]);
    }

    #[test]
    fn alphanumeric_option_grammar_errors() {
        let cases = [
            ("alphanumeric(allow-dash)", "unknown `alphanumeric` option `allow-dash`"),
            ("alphanumeric(allow-dashes, nope)", "unknown `alphanumeric` option `nope`"),
            ("alphanumeric(allow-dashes, allow-dashes)", "repeated `alphanumeric` option `allow-dashes`"),
            ("alphanumeric(min(1); allow-dashes)", "must be written before the first `;`"),
            ("alphanumeric(allow-dashes; allow-periods)", "must be written before the first `;`"),
            ("alphanumeric(allow-dashes, min(1))", "separate `alphanumeric` options from constraints with `;`"),
            ("alphanumeric(required, allow-dashes)", "separate `alphanumeric` options from constraints with `;`"),
            ("alphanumeric(allow-dashes(1))", "does not take arguments"),
            ("alphanumeric[](allow-dashes)", "is not valid on `alphanumeric` arrays"),
            ("string(allow-dashes)", "`allow-dashes` is an `alphanumeric` option and is not valid on `string`"),
            ("{ a: string }(allow-dashes)", "not valid on an inline object"),
        ];
        for (input, expected) in cases {
            let message = parse_err(input).to_string();
            assert!(message.contains(expected), "{input}: {message}");
        }
    }

    #[test]
    fn alphanumeric_option_errors_point_at_the_option() {
        let SchemaError::Grammar { span, .. } = parse_err("alphanumeric(allow-dashes, allow-dashes)") else {
            panic!("grammar error expected");
        };
        assert_eq!(span, 27..39);
        let SchemaError::Grammar { span, .. } = parse_err("alphanumeric(min(1); allow-dashes)") else {
            panic!("grammar error expected");
        };
        assert_eq!(span, 21..33);
    }

    #[test]
    fn alphanumeric_takes_string_constraints_and_suggestions() {
        let atom = parse("alphanumeric(allow-dashes; pattern('^a'); suggest(ab, 12); default(ab))");
        let Constraint::Suggest(candidates) = &atom.constraints[2] else {
            panic!("suggest expected: {atom:?}");
        };
        assert_eq!(candidates[1].interpreted, serde_json::json!("12"));
    }

    #[test]
    fn named_reference_keeps_options_for_resolution() {
        let atom = parse("slug(allow-dashes; max(10))");
        assert_eq!(atom.ty, TypeExpr::local("slug"));
        assert_eq!(options_of(&atom), [AlphanumericOption::AllowDashes]);
        assert_eq!(atom.constraints[1], Constraint::Max(10.0));
        assert_eq!(parse("slug(max(10))").constraints, vec![Constraint::Max(10.0)]);
        assert!(
            parse_err("slug(allow-dashes, max(10))")
                .to_string()
                .contains("separate `alphanumeric` options from constraints")
        );
    }

    #[test]
    fn unknown_identifier_parses_as_local_reference() {
        let atom = parse("widget");
        assert_eq!(atom.ty, TypeExpr::local("widget"));
        assert_eq!(atom, parse("widget@this"));
    }

    #[test]
    fn misspelled_keyword_parses_as_local_reference() {
        assert_eq!(parse("strng").ty, TypeExpr::local("strng"));
    }

    #[test]
    fn bare_keyword_stays_primitive_and_explicit_keyword_is_a_reference() {
        assert_eq!(parse("string").ty, TypeExpr::Primitive(SimplifiedType::String));
        assert_eq!(parse("string@this").ty, TypeExpr::local("string"));
    }

    #[test]
    fn bare_reference_postfixes_match_explicit_reference() {
        for (bare, explicit) in [
            ("person", "person@this"),
            ("person[]", "person[]@this"),
            ("person(required)", "person(required)@this"),
            ("person[](min(1))", "person[](min(1))@this"),
            ("person(min(2); max(5))", "person(min(2); max(5))@this"),
            ("person -> An author", "person@this -> An author"),
            ("person[](max(3)) -> People", "person[](max(3))@this -> People"),
        ] {
            let bare_atom = parse(bare);
            assert_eq!(bare_atom, parse(explicit), "{bare} vs {explicit}");
            assert_eq!(bare_atom.ty, TypeExpr::local("person"), "{bare}");
        }
    }

    #[test]
    fn bare_reference_records_constraints_and_array_marker() {
        // As on a primitive atom, the list after `[]` is parsed at the array
        // level and lands in `array_constraints`.
        let atom = parse("person[](min(1))");
        assert!(atom.is_array);
        assert!(atom.constraints.is_empty());
        assert_eq!(atom.array_constraints, vec![Constraint::MinItems(1)]);

        let atom = parse("person(required) -> The author");
        assert!(!atom.is_array);
        assert_eq!(atom.constraints, vec![Constraint::Required]);
        assert_eq!(atom.description.as_deref(), Some("The author"));
    }

    #[test]
    fn bare_reference_defers_suggestion_target() {
        // `suggest` is invalid on most primitives, but a reference's target
        // is unknown until resolution, so both spellings accept it.
        let atom = parse("code(suggest(1, b))");
        assert_eq!(atom, parse("code(suggest(1, b))@this"));
        assert!(matches!(atom.constraints.as_slice(), [Constraint::Suggest(c)] if c.len() == 2));
    }

    #[test]
    fn named_reference_keeps_a_list_before_the_array_marker_apart_in_every_spelling() {
        // The passive grammar accepts either order; the resolver decides what
        // a list before `[]` may carry once the target is known.
        for input in [
            "person(eager)[]",
            "person(eager)[]@this",
            "person(eager)[]@./people.yaml",
        ] {
            let atom = parse(input);
            assert!(atom.is_array, "{input}");
            assert_eq!(atom.constraints, vec![Constraint::Eager], "{input}");
            assert!(atom.array_constraints.is_empty(), "{input}");
        }
        assert_eq!(parse("person(eager)[]"), parse("person(eager)[]@this"));

        let both = parse("person(required; max(2))[](min(1); eager)@./people.yaml");
        assert_eq!(both.constraints, vec![Constraint::Required, Constraint::Max(2.0)]);
        assert_eq!(
            both.array_constraints,
            vec![Constraint::MinItems(1), Constraint::Eager]
        );

        let after = parse("person[](eager)");
        assert!(after.constraints.is_empty());
        assert_eq!(after.array_constraints, vec![Constraint::Eager]);
        assert_ne!(after, parse("person(eager)[]"));
    }

    #[test]
    fn named_reference_rejects_a_second_list_or_a_malformed_array_marker() {
        for input in ["person(eager)[", "person(eager)(required)", "person[](min(1))(eager)"] {
            assert!(
                matches!(parse_err(input), SchemaError::Grammar { .. }),
                "{input}"
            );
        }
    }

    #[test]
    fn bare_reference_works_in_inline_objects() {
        let atom = parse("{ author: person(required) -> The author, tags: tag[] }");
        let shape = inline_shape(&atom);
        let author = prop(shape, "author");
        assert_eq!(author.ty, TypeExpr::local("person"));
        assert_eq!(author.constraints, vec![Constraint::Required]);
        assert_eq!(author.description.as_deref(), Some("The author"));
        let tags = prop(shape, "tags");
        assert_eq!(tags.ty, TypeExpr::local("tag"));
        assert!(tags.is_array);
    }

    #[test]
    fn bare_reference_identifier_characters_match_explicit_reference() {
        for name in ["Person", "postal_code", "x-custom", "v2"] {
            assert_eq!(parse(name).ty, TypeExpr::local(name), "{name}");
            assert_eq!(parse(name), parse(&format!("{name}@this")), "{name}");
        }
    }

    #[test]
    fn parses_required_constraint() {
        let atom = parse("string(required)");
        assert_eq!(atom.constraints, vec![Constraint::Required]);
    }

    #[test]
    fn parses_generated_constraint() {
        let atom = parse("string(generated)");
        assert_eq!(atom.constraints, vec![Constraint::Generated]);
    }

    #[test]
    fn parses_generated_with_required() {
        // `generated` and `required` are orthogonal: `generated` describes
        // ownership/supply semantics; `required` controls type/nullability.
        // Both constraints must survive parsing in either order.
        let atom = parse("string(generated; required)");
        assert_eq!(
            atom.constraints,
            vec![Constraint::Generated, Constraint::Required]
        );
        let atom_rev = parse("string(required; generated)");
        assert_eq!(
            atom_rev.constraints,
            vec![Constraint::Required, Constraint::Generated]
        );
    }

    #[test]
    fn generated_takes_no_args() {
        let err = parse_err("string(generated())");
        let SchemaError::Grammar { message, .. } = err else {
            panic!("expected Grammar error, got {err:?}")
        };
        assert!(message.contains("does not take arguments"));
    }

    #[test]
    fn parses_string_min_max_as_lengths() {
        let atom = parse("string(min(5); max(80))");
        assert_eq!(
            atom.constraints,
            vec![Constraint::MinLen(5), Constraint::MaxLen(80)]
        );
    }

    #[test]
    fn parses_number_min_as_value() {
        let atom = parse("number(min(0); max(100))");
        assert_eq!(
            atom.constraints,
            vec![Constraint::Min(0.0), Constraint::Max(100.0)]
        );
    }

    #[test]
    fn parses_integer_constraint() {
        let atom = parse("number(integer)");
        assert_eq!(atom.constraints, vec![Constraint::Integer]);
    }

    #[test]
    fn parses_pattern_with_quoted_regex() {
        let atom = parse(r#"string(pattern("^[a-z]+$"))"#);
        assert_eq!(
            atom.constraints,
            vec![Constraint::Pattern("^[a-z]+$".into())]
        );
    }

    #[test]
    fn parses_pattern_with_bare_regex() {
        let atom = parse("string(pattern(^[a-z]+$))");
        assert_eq!(
            atom.constraints,
            vec![Constraint::Pattern("^[a-z]+$".into())]
        );
    }

    #[test]
    fn parses_array_with_item_and_array_constraints() {
        let atom = parse("string(pattern(^[a-z]+$))[](min(1); max(5); unique)");
        assert!(atom.is_array);
        assert_eq!(
            atom.constraints,
            vec![Constraint::Pattern("^[a-z]+$".into())]
        );
        assert_eq!(
            atom.array_constraints,
            vec![
                Constraint::MinItems(1),
                Constraint::MaxItems(5),
                Constraint::Unique
            ]
        );
    }

    #[test]
    fn array_level_min_max_are_item_counts_for_numbers_too() {
        let atom = parse("number(min(0); max(100))[](min(1); max(3))");
        assert_eq!(
            atom.constraints,
            vec![Constraint::Min(0.0), Constraint::Max(100.0)]
        );
        assert_eq!(
            atom.array_constraints,
            vec![Constraint::MinItems(1), Constraint::MaxItems(3)]
        );
    }

    #[test]
    fn parses_array_without_constraints() {
        let atom = parse("string[]");
        assert!(atom.is_array);
        assert!(atom.constraints.is_empty());
        assert!(atom.array_constraints.is_empty());
    }

    #[test]
    fn parses_default_string() {
        let atom = parse("string(default(hello))");
        assert_eq!(
            atom.constraints,
            vec![Constraint::Default(serde_json::Value::String(
                "hello".into()
            ))]
        );
    }

    #[test]
    fn parses_default_number() {
        // An integer default keeps integer typing (parsed from the exact source
        // lexeme), so precision survives for values past f64's 2^53 range.
        let atom = parse("number(default(3))");
        assert_eq!(
            atom.constraints,
            vec![Constraint::Default(serde_json::json!(3))]
        );
    }

    #[test]
    fn parses_default_quoted_string() {
        let atom = parse(r#"string(default("hi there"))"#);
        assert_eq!(
            atom.constraints,
            vec![Constraint::Default(serde_json::Value::String(
                "hi there".into()
            ))]
        );
    }

    #[test]
    fn parses_default_boolean() {
        let atom = parse("boolean(default(true))");
        assert_eq!(
            atom.constraints,
            vec![Constraint::Default(serde_json::Value::Bool(true))]
        );
    }

    #[test]
    fn parses_enum_members() {
        let atom = parse("enum(red, green, blue)");
        assert_eq!(
            atom.constraints,
            vec![Constraint::Members(vec![
                "red".into(),
                "green".into(),
                "blue".into()
            ])]
        );
    }

    #[test]
    fn parses_enum_with_default_and_required() {
        let atom = parse("enum(draft, published, archived; default(draft); required)");
        assert_eq!(
            atom.constraints,
            vec![
                Constraint::Members(vec!["draft".into(), "published".into(), "archived".into()]),
                Constraint::Default(serde_json::Value::String("draft".into())),
                Constraint::Required,
            ]
        );
    }

    #[test]
    fn parses_enum_with_quoted_members() {
        let atom = parse("enum('a, b', 'c; d')");
        assert_eq!(
            atom.constraints,
            vec![Constraint::Members(vec!["a, b".into(), "c; d".into()])]
        );
    }

    #[test]
    fn enum_requires_members() {
        let err = parse_err("enum");
        let SchemaError::Grammar { message, .. } = err else {
            panic!("expected Grammar error, got {err:?}")
        };
        assert!(message.contains("requires a constraint list"));
    }

    // ── literal grammar ────────────────────────────────────────────────

    fn literal_value(atom: &PropertyAtom) -> &serde_json::Value {
        atom.constraints
            .iter()
            .find_map(|c| match c {
                Constraint::LiteralValue(v) => Some(v),
                _ => None,
            })
            .expect("expected a LiteralValue constraint")
    }

    #[test]
    fn parses_bare_string_literal() {
        let atom = parse("literal(spec)");
        assert_eq!(atom.ty, TypeExpr::Primitive(SimplifiedType::Literal));
        assert_eq!(literal_value(&atom), &serde_json::json!("spec"));
    }

    #[test]
    fn types_bare_number_literal() {
        assert_eq!(literal_value(&parse("literal(2)")), &serde_json::json!(2));
        assert_eq!(literal_value(&parse("literal(-5)")), &serde_json::json!(-5));
        assert_eq!(literal_value(&parse("literal(2.5)")), &serde_json::json!(2.5));
    }

    #[test]
    fn types_bare_boolean_literal() {
        assert_eq!(literal_value(&parse("literal(true)")), &serde_json::json!(true));
        assert_eq!(literal_value(&parse("literal(false)")), &serde_json::json!(false));
    }

    #[test]
    fn quoted_literal_is_always_string() {
        assert_eq!(literal_value(&parse("literal('2')")), &serde_json::json!("2"));
        assert_eq!(literal_value(&parse(r#"literal("true")"#)), &serde_json::json!("true"));
        assert_eq!(literal_value(&parse("literal('a, b')")), &serde_json::json!("a, b"));
    }

    #[test]
    fn leading_zero_literal_is_string() {
        // `007` fails the numberlike shape test (leading zero) → typed as text.
        assert_eq!(literal_value(&parse("literal(007)")), &serde_json::json!("007"));
    }

    #[test]
    fn literal_with_constraints() {
        let atom = parse("literal(spec; required)");
        assert_eq!(
            atom.constraints,
            vec![
                Constraint::LiteralValue(serde_json::json!("spec")),
                Constraint::Required,
            ]
        );
    }

    #[test]
    fn literal_array_suffix() {
        let atom = parse("literal(auto)[]");
        assert!(atom.is_array);
        assert_eq!(literal_value(&atom), &serde_json::json!("auto"));
    }

    #[test]
    fn bare_literal_requires_value() {
        for input in ["literal", "literal()"] {
            let SchemaError::Grammar { message, .. } = parse_err(input) else {
                panic!("expected Grammar error for {input:?}");
            };
            assert!(
                message.contains("literal requires a value"),
                "unexpected message for {input:?}: {message}"
            );
        }
    }

    #[test]
    fn literal_multiple_values_directs_to_enum() {
        let SchemaError::Grammar { message, .. } = parse_err("literal(a, b)") else {
            panic!("expected Grammar error");
        };
        assert!(message.contains("enum"), "got: {message}");
    }

    #[test]
    fn bare_null_literal_rejected() {
        let SchemaError::Grammar { message, .. } = parse_err("literal(null)") else {
            panic!("expected Grammar error");
        };
        let lower = message.to_lowercase();
        assert!(lower.contains("quote") || lower.contains("drop"), "got: {message}");
    }

    // ── expression grammar ─────────────────────────────────────────────

    #[test]
    fn parses_bare_expression() {
        let atom = parse("expression");
        assert_eq!(atom.ty, TypeExpr::Primitive(SimplifiedType::Expression));
        assert!(atom.constraints.is_empty());
    }

    #[test]
    fn parses_expression_with_required() {
        let atom = parse("expression(required)");
        assert_eq!(atom.constraints, vec![Constraint::Required]);
    }

    #[test]
    fn parameterized_expression_is_rejected() {
        // The reserved future `expression(condition)` form is rejected in v1
        // as an unknown constraint.
        let SchemaError::Grammar { message, .. } = parse_err("expression(condition)") else {
            panic!("expected Grammar error");
        };
        assert!(message.contains("unknown constraint"), "got: {message}");
    }

    #[test]
    fn parses_file_match() {
        let atom = parse("file(match('*.md', '!_*.md'))");
        assert_eq!(
            atom.constraints,
            vec![Constraint::Match(vec!["*.md".into(), "!_*.md".into()])]
        );
    }

    #[test]
    fn parses_file_eager() {
        let atom = parse("file(eager)");
        assert_eq!(atom.ty, TypeExpr::Primitive(SimplifiedType::File));
        assert_eq!(atom.constraints, vec![Constraint::Eager]);
    }

    #[test]
    fn parses_file_eager_with_required_and_match() {
        let atom = parse("file(eager; required; match('**/*review*.md'))");
        assert_eq!(
            atom.constraints,
            vec![
                Constraint::Eager,
                Constraint::Required,
                Constraint::Match(vec!["**/*review*.md".into()]),
            ]
        );
    }

    #[test]
    fn eager_takes_no_args() {
        let err = parse_err("file(eager())");
        let SchemaError::Grammar { message, .. } = err else {
            panic!("expected Grammar error, got {err:?}")
        };
        assert!(message.contains("does not take arguments"));
    }

    #[test]
    fn parses_url_scheme() {
        let atom = parse("url(scheme(https, http))");
        assert_eq!(
            atom.constraints,
            vec![Constraint::Scheme(vec!["https".into(), "http".into()])]
        );
    }

    #[test]
    fn parses_url_scheme_lowercases() {
        let atom = parse("url(scheme(HTTPS))");
        assert_eq!(
            atom.constraints,
            vec![Constraint::Scheme(vec!["https".into()])]
        );
    }

    #[test]
    fn parses_description() {
        let atom = parse("string -> The author's full name");
        assert_eq!(atom.ty, TypeExpr::Primitive(SimplifiedType::String));
        assert_eq!(atom.description.as_deref(), Some("The author's full name"));
    }

    #[test]
    fn parses_constraints_with_description() {
        let atom = parse("string(not-empty; required) -> URL slug");
        assert_eq!(
            atom.constraints,
            vec![Constraint::NotEmpty, Constraint::Required]
        );
        assert_eq!(atom.description.as_deref(), Some("URL slug"));
    }

    #[test]
    fn arrow_alone_errors() {
        let err = parse_err("string -> ");
        let SchemaError::Grammar { message, .. } = err else {
            panic!("expected Grammar error, got {err:?}")
        };
        assert!(message.contains("description"));
    }

    #[test]
    fn whitespace_inside_constraints_is_ignored() {
        let a = parse("string( required )");
        let b = parse("string(required)");
        assert_eq!(a.constraints, b.constraints);
    }

    #[test]
    fn unknown_constraint_errors() {
        let err = parse_err("string(weird)");
        let SchemaError::Grammar { message, .. } = err else {
            panic!("expected Grammar error, got {err:?}")
        };
        assert!(message.contains("unknown constraint"));
    }

    #[test]
    fn unterminated_paren_errors() {
        let err = parse_err("string(required");
        let SchemaError::Grammar { message, .. } = err else {
            panic!("expected Grammar error, got {err:?}")
        };
        assert!(message.contains("unterminated") || message.contains(")"));
    }

    #[test]
    fn min_negative_errors_for_string() {
        let err = parse_err("string(min(-1))");
        let SchemaError::Grammar { message, .. } = err else {
            panic!("expected Grammar error, got {err:?}")
        };
        assert!(message.contains("non-negative"));
    }

    #[test]
    fn min_negative_ok_for_number() {
        let atom = parse("number(min(-5))");
        assert_eq!(atom.constraints, vec![Constraint::Min(-5.0)]);
    }

    // ── Inline object parser tests (Phase 1, Decision #1 — #11) ──────

    use super::super::types::SchemaShape;

    fn inline_shape(atom: &PropertyAtom) -> &SchemaShape {
        match &atom.ty {
            TypeExpr::InlineObject(s) => s,
            other => panic!("expected InlineObject, got {other:?}"),
        }
    }

    fn prop<'a>(shape: &'a SchemaShape, name: &str) -> &'a PropertyAtom {
        match shape.properties.get(name) {
            Some(PropertyDef::Single(a)) => a,
            other => panic!("expected single property `{name}`, got {other:?}"),
        }
    }

    #[test]
    fn parses_bare_inline_object() {
        let atom = parse("{ foo: string, bar: number }");
        let shape = inline_shape(&atom);
        assert_eq!(shape.properties.len(), 2);
        assert_eq!(prop(shape, "foo").ty, TypeExpr::Primitive(SimplifiedType::String));
        assert_eq!(prop(shape, "bar").ty, TypeExpr::Primitive(SimplifiedType::Number));
        assert!(!atom.is_array);
    }

    #[test]
    fn parses_inline_object_array() {
        let atom = parse("{ foo: string }[]");
        let shape = inline_shape(&atom);
        assert_eq!(prop(shape, "foo").ty, TypeExpr::Primitive(SimplifiedType::String));
        assert!(atom.is_array);
    }

    #[test]
    fn parses_required_single_inline_object() {
        let atom = parse("{ host: string }(required)");
        let shape = inline_shape(&atom);
        assert_eq!(prop(shape, "host").ty, TypeExpr::Primitive(SimplifiedType::String));
        assert!(!atom.is_array);
        assert_eq!(atom.constraints, vec![Constraint::Required]);
    }

    #[test]
    fn parses_constrained_inline_object_array() {
        let atom = parse("{ name: string }[](min(1); required)");
        assert!(atom.is_array);
        let shape = inline_shape(&atom);
        assert_eq!(prop(shape, "name").ty, TypeExpr::Primitive(SimplifiedType::String));
        assert_eq!(
            atom.array_constraints,
            vec![Constraint::MinItems(1), Constraint::Required]
        );
    }

    #[test]
    fn rejects_inline_object_constraints_before_array_suffix() {
        for input in [
            "{ foo: string }(required)[]",
            "{ foo: string }(default({}))[]",
            "{ foo: string }(min(1))[]",
        ] {
            let err = parse_err(input);
            let SchemaError::Grammar { message, .. } = err else {
                panic!("expected grammar error");
            };
            assert!(
                message.contains("inline object array constraints must follow `[]`"),
                "unexpected error for {input:?}: {message}"
            );
        }
    }

    #[test]
    fn parses_nested_inline_object() {
        let atom = parse("{ outer: { inner: string } }");
        let outer = inline_shape(&atom);
        assert_eq!(
            prop(outer, "outer").ty,
            TypeExpr::InlineObject({
                let mut s = SchemaShape::new();
                s.properties.insert(
                    "inner".into(),
                    PropertyDef::Single(PropertyAtom::bare(SimplifiedType::String)),
                );
                s
            })
        );
    }

    #[test]
    fn inline_object_at_depth_32_is_accepted() {
        // Build `{a: {a: {a: ... {a: string} ... }}}` with 32 levels.
        // There are 32 inline objects and one innermost `string` value.
        let mut input = String::new();
        for _ in 0..32 {
            input.push_str("{a:");
        }
        input.push_str("string");
        for _ in 0..32 {
            input.push('}');
        }
        let atom = parse(&input);
        let shape = inline_shape(&atom);
        // Walk down 32 levels of `a` and confirm the innermost is a string.
        let mut current = shape;
        for _ in 0..31 {
            let inner = prop(current, "a");
            current = inline_shape(inner);
        }
        // The 32nd `a` is the string value itself.
        assert_eq!(
            prop(current, "a").ty,
            TypeExpr::Primitive(SimplifiedType::String)
        );
    }

    #[test]
    fn inline_object_at_depth_33_is_rejected() {
        // 33 levels should fail the depth check.
        let mut input = String::new();
        for _ in 0..33 {
            input.push_str("{a:");
        }
        input.push_str("string");
        for _ in 0..33 {
            input.push('}');
        }
        let err = parse_err(&input);
        let SchemaError::Grammar { message, .. } = err else {
            panic!("expected Grammar error, got {err:?}")
        };
        assert!(
            message.contains("nesting") || message.contains("deep"),
            "unexpected message: {message}"
        );
    }

    #[test]
    fn whitespace_inside_inline_object_is_ignored() {
        let compact = parse("{ foo: string, bar: number }");
        let padded = parse("{  foo : string , bar : number  }");
        assert_eq!(compact, padded);
    }

    #[test]
    fn multi_line_inline_object_parses() {
        let atom = parse(
            "{\n    url: url(scheme(https); required),\n    method: enum(GET, POST, PUT, DELETE; required),\n    timeout: number(default(30))\n}[]",
        );
        assert!(atom.is_array);
        let shape = inline_shape(&atom);
        assert_eq!(shape.properties.len(), 3);
        assert_eq!(prop(shape, "url").ty, TypeExpr::Primitive(SimplifiedType::Url));
        assert_eq!(prop(shape, "method").ty, TypeExpr::Primitive(SimplifiedType::Enum));
        assert_eq!(prop(shape, "timeout").ty, TypeExpr::Primitive(SimplifiedType::Number));
    }

    #[test]
    fn inline_object_with_property_constraints() {
        let atom = parse("{ foo: string(required; not-empty), bar: number(min(0)) }");
        let shape = inline_shape(&atom);
        let foo = prop(shape, "foo");
        assert_eq!(
            foo.constraints,
            vec![Constraint::Required, Constraint::NotEmpty]
        );
        let bar = prop(shape, "bar");
        assert_eq!(bar.constraints, vec![Constraint::Min(0.0)]);
    }

    #[test]
    fn inline_object_with_descriptions() {
        let atom = parse("{ foo: string(required) -> The foo, bar: number -> The bar }");
        let shape = inline_shape(&atom);
        let foo = prop(shape, "foo");
        assert_eq!(foo.description.as_deref(), Some("The foo"));
        let bar = prop(shape, "bar");
        assert_eq!(bar.description.as_deref(), Some("The bar"));
    }

    #[test]
    fn inline_object_description_with_unescaped_comma_is_not_supported() {
        // The comma ends the description, so `second` is read as the next
        // property name and its missing `:` is a grammar error. Decision #9
        // leaves escaping out of scope.
        let err = parse_err("{ foo: string -> first, second }");
        let SchemaError::Grammar { message, .. } = err else {
            panic!("expected Grammar error, got {err:?}")
        };
        assert!(message.contains("expected `:`"), "unexpected message: {message}");
    }

    #[test]
    fn inline_object_accepts_known_identifier_shapes() {
        for name in ["name", "foo_id", "x-custom", "api2_version", "123abc"] {
            let input = format!("{{ {name}: string }}");
            let atom = parse(&input);
            let shape = inline_shape(&atom);
            assert!(shape.properties.contains_key(name), "missing: {name}");
        }
    }

    #[test]
    fn inline_object_rejects_invalid_identifier_shapes() {
        for name in ["display name", "@type", "x.custom"] {
            let input = format!("{{ {name}: string }}");
            let err = parse_err(&input);
            // All three should produce a Grammar error, but the exact
            // message differs (whitespace is illegal, `@` and `.` are
            // unexpected characters). Just confirm a grammar error.
            assert!(
                matches!(err, SchemaError::Grammar { .. }),
                "expected Grammar error for `{name}`, got {err:?}"
            );
        }
    }

    #[test]
    fn quoted_property_keys_are_rejected() {
        let err = parse_err(r#"{ "x-custom": string }"#);
        assert!(matches!(err, SchemaError::Grammar { .. }));
    }

    #[test]
    fn empty_inline_object_is_accepted() {
        let atom = parse("{}");
        let shape = inline_shape(&atom);
        assert!(shape.properties.is_empty());
        assert!(!atom.is_array);
    }

    #[test]
    fn missing_closing_brace_errors() {
        let err = parse_err("{ foo: string");
        let SchemaError::Grammar { message, .. } = err else {
            panic!("expected Grammar error, got {err:?}")
        };
        assert!(message.contains("`}`") || message.contains("unterminated"));
    }

    #[test]
    fn missing_colon_errors() {
        let err = parse_err("{ foo string }");
        let SchemaError::Grammar { message, .. } = err else {
            panic!("expected Grammar error, got {err:?}")
        };
        assert!(message.contains("`:`") || message.contains("expected"));
    }

    #[test]
    fn trailing_comma_is_accepted() {
        let atom = parse("{ foo: string, }");
        let shape = inline_shape(&atom);
        assert_eq!(shape.properties.len(), 1);
        assert_eq!(
            prop(shape, "foo").ty,
            TypeExpr::Primitive(SimplifiedType::String)
        );
    }

    #[test]
    fn inline_object_at_property_position_in_yaml_shape() {
        // The YAML-shape layer passes the *string* value through to
        // `parse_type_expr`. A YAML mapping is rejected at the YAML layer;
        // the parser itself is only invoked for scalar string values.
        let value = "{ foo: string, bar: number }";
        let atom = parse(value);
        let shape = inline_shape(&atom);
        assert_eq!(shape.properties.len(), 2);
    }
}

/// Phase 1 (TDD scaffolding) for the schema-plus composition primitives
/// (`darkmatter/features/2026-07-08-schema-plus/`).
///
/// Each test pins the target parse behavior for a not-yet-implemented feature
/// and is `#[ignore]`-gated so `just test` stays green until the feature lands.
/// The `#[ignore]` reason names the phase that will implement it; remove the
/// attribute in that phase. Confirm they fail for the expected missing feature
/// with:
///
/// ```text
/// cargo nextest run -p darkmatter --run-ignored ignored-only schema_plus
/// ```
#[cfg(test)]
mod schema_plus_phase1 {
    use super::*;

    // ── Feature A — `example()` constraint ───────────────────────────────

    #[test]
    fn parses_example_constraint_single_file() {
        let atom = parse_type_expr(
            "today",
            "date(generated; required; example(./today-example.yaml))",
        )
        .expect("`example(...)` should parse as a constraint");
        assert_eq!(atom.ty, TypeExpr::Primitive(SimplifiedType::Date));
        assert!(
            atom.constraints
                .contains(&Constraint::Example(vec!["./today-example.yaml".into()])),
            "expected an Example constraint: {:?}",
            atom.constraints
        );
    }

    #[test]
    fn parses_example_constraint_multiple_files() {
        let atom = parse_type_expr("x", "string(example(./a.yaml, ./b.yaml))")
            .expect("`example(...)` should accept comma-separated file references");
        assert!(
            atom.constraints.contains(&Constraint::Example(vec![
                "./a.yaml".into(),
                "./b.yaml".into()
            ])),
            "expected a two-file Example constraint: {:?}",
            atom.constraints
        );
    }

    // ── Feature B — `@` cross-file named-type import ─────────────────────

    #[test]
    fn parses_named_type_import() {
        let atom = parse_type_expr("type", "type@./types.yaml")
            .expect("`type@./types.yaml` should parse as an imported type");
        assert_eq!(
            atom.ty,
            TypeExpr::Imported {
                name: "type".into(),
                reference: "./types.yaml".into(),
            }
        );
        assert!(!atom.is_array);
    }

    #[test]
    fn parses_array_named_type_import() {
        let atom = parse_type_expr("parameters", "parameter[]@./types.yaml")
            .expect("`parameter[]@./types.yaml` should parse");
        assert!(atom.is_array, "postfix `[]` binds to the imported name");
        assert_eq!(
            atom.ty,
            TypeExpr::Imported {
                name: "parameter".into(),
                reference: "./types.yaml".into(),
            }
        );
    }

    #[test]
    fn parses_constrained_self_import() {
        let atom = parse_type_expr("value", "type(required)@this")
            .expect("`type(required)@this` should parse");
        assert_eq!(
            atom.ty,
            TypeExpr::Imported {
                name: "type".into(),
                reference: "this".into(),
            }
        );
        assert_eq!(atom.constraints, vec![Constraint::Required]);
    }

    // ── Feature C — inline-object pattern/dictionary keys ────────────────

    #[test]
    fn parses_inline_catch_all_key() {
        let atom = parse_type_expr("parameter", "{ <string>: any }")
            .expect("`<string>` catch-all key should parse in an inline object");
        let TypeExpr::InlineObject(shape) = &atom.ty else {
            panic!("expected inline object, got {:?}", atom.ty);
        };
        assert!(shape.properties.is_empty(), "catch-all is not a literal key");
        assert_eq!(shape.pattern_keys.len(), 1);
        assert_eq!(shape.pattern_keys[0].key, PatternKey::CatchAll);
    }

    #[test]
    fn parses_inline_regex_pattern_key() {
        let atom = parse_type_expr("dict", "{ <pattern::[0-9]$>: number }")
            .expect("`<pattern::[0-9]$>` pattern key should parse in an inline object");
        let TypeExpr::InlineObject(shape) = &atom.ty else {
            panic!("expected inline object, got {:?}", atom.ty);
        };
        assert_eq!(
            shape.pattern_keys[0].key,
            PatternKey::Pattern("[0-9]$".into())
        );
    }

    // ── Feature D — content-format string types ──────────────────────────

    #[test]
    fn parses_yaml_type_keyword() {
        let atom = parse_type_expr("frontmatter", "yaml").expect("`yaml` should parse as a type");
        assert_eq!(atom.ty, TypeExpr::Primitive(SimplifiedType::Yaml));
    }

    #[test]
    fn parses_json_type_keyword() {
        let atom = parse_type_expr("config", "json").expect("`json` should parse as a type");
        assert_eq!(atom.ty, TypeExpr::Primitive(SimplifiedType::Json));
    }
}
