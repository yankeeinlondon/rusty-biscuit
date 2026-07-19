//! Lexer and parser for the SimplifiedSchema type-and-constraint string.
//!
//! Each property value (after the YAML-shape layer has decided this is a
//! single type expression) is parsed by [`parse_type_expr`] following the
//! EBNF in the schemas spec:
//!
//! ```text
//! type_expr_string := type_expr ( "->" description )?
//! type_expr        := simple_type
//!                   | inline_object
//!                   | import
//! simple_type      := type_name ( "(" item_constraints ")" )?
//!                                ( "[]" ( "(" arr_constraints ")" )? )?
//! inline_object    := "{" ws* property_list? ws* "}"
//!                     ( "(" item_constraints ")"
//!                     | "[]" ( "(" arr_constraints ")" )?
//!                     )?
//! import           := identifier ( "[]" )? ( "(" item_constraints ")" )?
//!                     "@" fileref                             (Feature B, O-B1)
//! property_list    := property_def ( "," ws* property_def )* ","?
//! property_def     := ( identifier | pattern_key ) ws* ":" ws* type_expr_string
//! pattern_key      := "<" pattern_body ">"                   (Feature C)
//! ws               := <whitespace or line-break>
//! identifier       := ( ASCII_ALNUM | "-" | "_" )+
//! type_name        := "string" | "date" | "datetime" | "time" | "number"
//!                   | "numberlike" | "boolean" | "boolish" | "object"
//!                   | "file" | "enum" | "url" | "email" | "yaml" | "json"
//!                   | "literal" | "expression" | "type-definition" | "schema"
//!                   | "any"
//! item_constraints := constraint ( ";" constraint )*
//! arr_constraints  := constraint ( ";" constraint )*
//! constraint       := IDENT
//!                   | IDENT "(" arglist ")"
//! arglist          := arg ( "," arg )*
//! arg              := NUMBER | BARE_WORD | SQUOTED | DQUOTED
//! fileref          := <biscuit-file file reference> | "this"
//! description      := <rest-of-top-level-field, trimmed>
//! ```
//!
//! An `import` (`Name@fileref`) inlines a named type from another schema file
//! and is represented as [`TypeExpr::Imported`] until the resolver expands it.
//! A `pattern_key` (`<string>`, `<starting::P>`, `<ending::S>`, `<pattern::RE>`)
//! keys a dictionary/pattern-keyed object; see [`PatternKey`]. Both are
//! additive — a schema using neither parses exactly as before.
//!
//! Inside an `inline_object`, `description` consumes text only until the
//! next top-level comma or closing brace (Decision #9). Inline object
//! nesting is capped at [`MAX_INLINE_OBJECT_DEPTH`] levels (Decision #11).
//!
//! Errors surface as [`SchemaError::Grammar`] with the byte span of the
//! offending token.

use std::{ops::Range, str::FromStr};

use crate::markdown::schemas::errors::SchemaError;

use super::types::{
    Constraint, PatternKey, PatternKeyDef, PropertyAtom, PropertyDef, SchemaShape, SimplifiedType,
    SuggestionCandidate, TypeExpr,
};

/// Hard maximum number of nested inline object levels the parser will accept.
/// Exceeding this depth is a grammar error (Decision #11).
pub const MAX_INLINE_OBJECT_DEPTH: usize = 32;

/// Parses a single type-and-constraint string into a [`PropertyAtom`].
///
/// `property` is supplied for error reporting only.
pub fn parse_type_expr(property: &str, input: &str) -> Result<PropertyAtom, SchemaError> {
    let mut parser = Parser::new(property, input);
    parser.parse()
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
    /// `[-]?digits[.digits]`. Returns `None` if the cursor is not at a number.
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
        let lex = &self.src[start..cursor];
        let value: f64 = lex.parse().ok()?;
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

struct Parser<'a> {
    property: &'a str,
    src: &'a str,
    lex: Lexer<'a>,
    defer_suggestion_target: bool,
}

impl<'a> Parser<'a> {
    fn new(property: &'a str, src: &'a str) -> Self {
        Self {
            property,
            src,
            lex: Lexer::new(src),
            defer_suggestion_target: false,
        }
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

    fn parse(&mut self) -> Result<PropertyAtom, SchemaError> {
        // Inline object literal dispatches before primitive-type parsing.
        self.lex.skip_ws();
        if self.lex.peek_byte() == Some(b'{') {
            let shape = self.parse_inline_object_body(0)?;
            // Postfix `()` and `[]` apply to the inline object as a whole.
            let (constraints, is_array, array_constraints) =
                self.parse_postfix_after_inline_object()?;
            // Top-level description: take the rest of the string (the
            // top-level field has no surrounding `}` to act as a terminator).
            let description = self.parse_top_level_description()?;
            Ok(PropertyAtom {
                ty: TypeExpr::InlineObject(shape),
                is_array,
                constraints,
                array_constraints,
                description,
            })
        } else {
            let atom = self.parse_type_expr()?;
            // Optional description.
            self.lex.skip_ws();
            let after = self.lex.pos;
            let next = self.next_token(LexMode::Outer)?;
            match next.tok {
                Tok::Eof => Ok(atom),
                Tok::Arrow => {
                    let rest = self.src[self.lex.pos..].trim().to_string();
                    if rest.is_empty() {
                        return self.err("`->` must be followed by a description", next.span);
                    }
                    self.lex.pos = self.src.len();
                    Ok(PropertyAtom {
                        description: Some(rest),
                        ..atom
                    })
                }
                other => self.err(
                    format!("expected end of expression or `->`, found `{:?}`", other),
                    after..next.span.end,
                ),
            }
        }
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
            value_constraints = self.parse_constraint_list(SimplifiedType::Any, false)?;
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
                array_constraints = self.parse_constraint_list(SimplifiedType::Any, true)?;
                self.expect(Tok::RParen)?;
            }
        }
        Ok((value_constraints, is_array, array_constraints))
    }

    /// Top-level `-> description` consumes the rest of the input.
    fn parse_top_level_description(&mut self) -> Result<Option<String>, SchemaError> {
        self.lex.skip_ws();
        let after = self.lex.pos;
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
        let mut pos = start + 1;
        while pos < self.lex.bytes.len() && self.lex.bytes[pos] != b'>' {
            pos += 1;
        }
        if pos >= self.lex.bytes.len() {
            return self.err("unterminated pattern key (missing `>`)", start..pos);
        }
        pos += 1; // consume `>`
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
        self.lex.skip_ws();
        // The property value is a single type expression, possibly with a
        // postfix `-> description`. Inline object property values cannot
        // be property-level unions (the YAML-shape layer rejects nested
        // mapping arms at property positions).
        let atom = if self.lex.peek_byte() == Some(b'{') {
            let shape = self.parse_inline_object_body(depth + 1)?;
            let (constraints, is_array, array_constraints) =
                self.parse_postfix_after_inline_object()?;
            // After the closing `}` and any postfix, an inline object
            // property may also carry a description that terminates at
            // the next `,` or `}` in the *outer* object's body.
            let description = if self.lex.peek_byte() == Some(b'-')
                && self.lex.bytes.get(self.lex.pos + 1).copied() == Some(b'>')
            {
                self.lex.pos += 2;
                Some(self.read_inline_object_description()?)
            } else {
                None
            };
            PropertyAtom {
                ty: TypeExpr::InlineObject(shape),
                is_array,
                constraints,
                array_constraints,
                description,
            }
        } else {
            let mut atom = self.parse_type_expr()?;
            self.lex.skip_ws();
            if self.lex.peek_byte() == Some(b'-')
                && self.lex.bytes.get(self.lex.pos + 1).copied() == Some(b'>')
            {
                // Consume the arrow and parse the description with the
                // inline-object terminator.
                self.lex.pos += 2;
                let desc = self.read_inline_object_description()?;
                atom.description = Some(desc);
            }
            atom
        };
        Ok(PropertyDef::Single(atom))
    }

    /// Reads a description text from the current position up to (but not
    /// consuming) the next `,` or `}` at the current inline-object depth.
    /// Tracks nested `{`/`}` and constraint `(`/`)` so commas and braces
    /// inside nested structures do not terminate the description.
    ///
    /// Decision #9 says: descriptions terminate at the next top-level comma
    /// or closing brace in the *current* inline object body. The caller is
    /// responsible for ensuring the terminator is actually present.
    fn read_inline_object_description(&mut self) -> Result<String, SchemaError> {
        let start = self.lex.pos;
        let bytes = self.lex.bytes;
        let mut pos = start;
        let mut brace_depth: usize = 0;
        let mut paren_depth: usize = 0;
        while pos < bytes.len() {
            let b = bytes[pos];
            match b {
                b'{' => brace_depth += 1,
                b'}' => {
                    if brace_depth == 0 {
                        break;
                    }
                    brace_depth -= 1;
                }
                b'(' => paren_depth += 1,
                b')' => {
                    if paren_depth == 0 {
                        return self.err(
                            "unbalanced `)` in inline object description",
                            pos..pos + 1,
                        );
                    }
                    paren_depth -= 1;
                }
                b',' if brace_depth == 0 && paren_depth == 0 => break,
                _ => {}
            }
            pos += 1;
        }
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
        // A cross-file named-type import (`Name@fileref`, Feature B) is
        // detected here: after the base name the grammar is
        // `('[]')? ('(' constraints ')')? '@' fileref` (O-B1). The base name
        // is an arbitrary named type, not a built-in keyword, so import
        // detection runs *before* the `SimplifiedType::from_keyword` check.
        if self.looks_like_import_suffix() {
            return self.parse_import(name);
        }

        let ty = SimplifiedType::from_keyword(&name).ok_or_else(|| SchemaError::Grammar {
            property: self.property.to_string(),
            message: format!("unknown type `{name}`"),
            span: name_span.clone(),
        })?;

        // Optional first paren list (item constraints, or value constraints
        // for non-array atoms).
        let mut item_constraints = Vec::new();
        self.lex.skip_ws();
        if self.lex.peek_byte() == Some(b'(') {
            // consume (
            self.lex.pos += 1;
            item_constraints = self.parse_constraint_list(ty, false)?;
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
                array_constraints = self.parse_constraint_list(ty, true)?;
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
                Constraint::Required | Constraint::Generated => {}
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
    /// `('[]')? ('(' … ')')?` followed by `@`. Does not consume input and does
    /// not interpret constraint contents (so it never raises spurious errors on
    /// non-import expressions like `enum(a, b, c)`).
    fn looks_like_import_suffix(&self) -> bool {
        let bytes = self.lex.bytes;
        let mut pos = self.lex.pos;
        let skip_ws = |pos: &mut usize| {
            while *pos < bytes.len() && bytes[*pos].is_ascii_whitespace() {
                *pos += 1;
            }
        };
        skip_ws(&mut pos);
        // optional `[]`
        if bytes.get(pos) == Some(&b'[') {
            if bytes.get(pos + 1) == Some(&b']') {
                pos += 2;
                skip_ws(&mut pos);
            } else {
                return false;
            }
        }
        // optional balanced `(...)`, quote-aware
        if bytes.get(pos) == Some(&b'(') {
            pos += 1;
            let mut depth = 1usize;
            let mut quote: Option<u8> = None;
            while pos < bytes.len() && depth > 0 {
                let b = bytes[pos];
                match quote {
                    Some(q) => {
                        if b == b'\\' {
                            pos += 1;
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
                pos += 1;
            }
            if depth != 0 {
                return false;
            }
            skip_ws(&mut pos);
        }
        bytes.get(pos) == Some(&b'@')
    }

    /// Parses the import postfix after the base `name` has been read:
    /// `('[]')? ('(' constraints ')')? '@' fileref` (O-B1). Postfix `[]` and
    /// constraints ride on the enclosing [`PropertyAtom`]; the constraints are
    /// re-interpreted against the resolved type in Phase 4, so they are parsed
    /// here with a neutral type.
    fn parse_import(&mut self, name: String) -> Result<PropertyAtom, SchemaError> {
        let mut is_array = false;
        self.lex.skip_ws();
        if self.lex.peek_byte() == Some(b'[') {
            self.lex.pos += 1;
            self.expect(Tok::RBracket)?;
            is_array = true;
        }

        let mut constraints = Vec::new();
        self.lex.skip_ws();
        if self.lex.peek_byte() == Some(b'(') {
            self.lex.pos += 1;
            self.defer_suggestion_target = !is_array;
            constraints = self.parse_constraint_list(SimplifiedType::Any, is_array)?;
            self.defer_suggestion_target = false;
            self.expect(Tok::RParen)?;
        }

        self.lex.skip_ws();
        self.expect_byte(b'@', "expected `@` in an imported type reference")?;
        let reference = self.read_fileref()?;

        Ok(PropertyAtom {
            ty: TypeExpr::Imported { name, reference },
            is_array,
            constraints,
            array_constraints: Vec::new(),
            description: None,
        })
    }

    /// Reads the file reference to the right of `@`. Runs to the end of the
    /// current sub-expression: stops at `->` (description arrow), a top-level
    /// `,` or `}` (inline-object terminators), or end of input.
    fn read_fileref(&mut self) -> Result<String, SchemaError> {
        self.lex.skip_ws();
        let start = self.lex.pos;
        let bytes = self.lex.bytes;
        let mut pos = start;
        while pos < bytes.len() {
            let b = bytes[pos];
            if b == b',' || b == b'}' {
                break;
            }
            if b == b'-' && bytes.get(pos + 1) == Some(&b'>') {
                break;
            }
            pos += 1;
        }
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
        ty: SimplifiedType,
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
        if matches!(ty, SimplifiedType::Enum) && !is_array_level {
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
        if matches!(ty, SimplifiedType::Literal) && !is_array_level {
            return self.parse_literal_value();
        }

        loop {
            self.lex.skip_ws();
            let constraint_start = self.lex.pos;
            let constraint = self.parse_one_constraint(ty, is_array_level)?;
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
                    continue;
                }
                Some(b')') => break,
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
                let constraint = self.parse_one_constraint(SimplifiedType::Enum, false)?;
                tail.push(constraint);
                self.lex.skip_ws();
                match self.lex.peek_byte() {
                    Some(b';') => {
                        self.lex.pos += 1;
                        // After we've moved into "tail" mode, all subsequent
                        // constraints are parsed normally.
                        return Ok((
                            members,
                            self.parse_remaining_constraints(SimplifiedType::Enum, tail)?,
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
                Tok::Word(s) => s,
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
                        self.parse_remaining_constraints(SimplifiedType::Enum, tail)?,
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
        ty: SimplifiedType,
        mut acc: Vec<Constraint>,
    ) -> Result<Vec<Constraint>, SchemaError> {
        loop {
            self.lex.skip_ws();
            if self.lex.peek_byte() == Some(b')') {
                return Ok(acc);
            }
            let c = self.parse_one_constraint(ty, false)?;
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
                self.parse_remaining_constraints(SimplifiedType::Literal, constraints)
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
        ty: SimplifiedType,
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
            ("required", true) => {
                return self.err("`required` does not take arguments", kw_span);
            }
            ("eager", true) => {
                return self.err("`eager` does not take arguments", kw_span);
            }
            ("generated", true) => {
                return self.err("`generated` does not take arguments", kw_span);
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
                Constraint::Default(arg_to_json(&args[0]))
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
                let n = arg_to_number(&args[0]).ok_or_else(|| SchemaError::Grammar {
                    property: self.property.to_string(),
                    message: format!("`min` requires a numeric argument, got `{}`", args[0].lex),
                    span: args[0].span.clone(),
                })?;
                if is_array_level {
                    let n_usize = number_to_usize(n, "min", &args[0], self.property)?;
                    Constraint::MinItems(n_usize)
                } else if matches!(ty, SimplifiedType::String) {
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
                let n = arg_to_number(&args[0]).ok_or_else(|| SchemaError::Grammar {
                    property: self.property.to_string(),
                    message: format!("`max` requires a numeric argument, got `{}`", args[0].lex),
                    span: args[0].span.clone(),
                })?;
                if is_array_level {
                    let n_usize = number_to_usize(n, "max", &args[0], self.property)?;
                    Constraint::MaxItems(n_usize)
                } else if matches!(ty, SimplifiedType::String) {
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
                let deferred_import = self.defer_suggestion_target && matches!(ty, SimplifiedType::Any);
                if is_array_level
                    || (!deferred_import
                        && !matches!(ty, SimplifiedType::String | SimplifiedType::Number))
                {
                    return self.err(
                        format!("`suggest` is not valid on `{}`", ty.as_keyword()),
                        kw_span.start..self.lex.pos,
                    );
                }

                let mut candidates = Vec::with_capacity(args.len());
                for arg in args {
                    let candidate = interpret_suggestion(
                        &arg,
                        if deferred_import {
                            SimplifiedType::String
                        } else {
                            ty
                        },
                    );
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
                let globs = args.into_iter().map(|a| a.lex).collect();
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
            ("example", true) => {
                self.lex.pos += 1;
                let args = self.parse_arglist()?;
                self.expect(Tok::RParen)?;
                if args.is_empty() {
                    return self.err("`example` requires at least one file reference", kw_span);
                }
                let refs = args.into_iter().map(|a| a.lex).collect();
                Constraint::Example(refs)
            }
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
                let n = arg_to_number(&args[0]).ok_or_else(|| SchemaError::Grammar {
                    property: self.property.to_string(),
                    message: format!(
                        "`min-keys` requires a numeric argument, got `{}`",
                        args[0].lex
                    ),
                    span: args[0].span.clone(),
                })?;
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
                let n = arg_to_number(&args[0]).ok_or_else(|| SchemaError::Grammar {
                    property: self.property.to_string(),
                    message: format!(
                        "`max-keys` requires a numeric argument, got `{}`",
                        args[0].lex
                    ),
                    span: args[0].span.clone(),
                })?;
                Constraint::MaxKeys(number_to_usize(n, "max-keys", &args[0], self.property)?)
            }
            ("min-keys" | "max-keys" | "example" | "suggest", false) => {
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
                    span: tok.span,
                },
                Tok::Word(s) => Arg {
                    lex: s,
                    number: None,
                    span: tok.span,
                },
                Tok::Quoted(s) => Arg {
                    lex: s,
                    number: None,
                    span: tok.span,
                },
                Tok::Ident(s) => Arg {
                    lex: s,
                    number: None,
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
    span: Range<usize>,
}

fn arg_to_number(arg: &Arg) -> Option<f64> {
    arg.number.or_else(|| arg.lex.parse().ok())
}

fn interpret_suggestion(arg: &Arg, ty: SimplifiedType) -> SuggestionCandidate {
    interpret_suggestion_parts(&arg.lex, arg.span.clone(), ty)
}

fn interpret_suggestion_parts(
    decoded: &str,
    span: Range<usize>,
    ty: SimplifiedType,
) -> SuggestionCandidate {
    if matches!(ty, SimplifiedType::String) {
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

fn arg_to_json(arg: &Arg) -> serde_json::Value {
    if let Some(n) = arg.number {
        // Parse the exact source spelling first so a large-integer default
        // (`default(9007199254740993)`) keeps i64/u64 precision — the pre-lexed
        // f64 `n` has already rounded values past 2^53. Non-JSON-shaped numeric
        // tokens fall back to that f64.
        if let Ok(num) = serde_json::Number::from_str(&arg.lex) {
            return serde_json::Value::Number(num);
        }
        if let Some(num) = serde_json::Number::from_f64(n) {
            return serde_json::Value::Number(num);
        }
        return serde_json::Value::String(arg.lex.clone());
    }
    match arg.lex.as_str() {
        "true" => serde_json::Value::Bool(true),
        "false" => serde_json::Value::Bool(false),
        "null" => serde_json::Value::Null,
        _ => serde_json::Value::String(arg.lex.clone()),
    }
}

fn format_number(n: f64) -> String {
    if n.fract() == 0.0 && n.is_finite() {
        format!("{}", n as i64)
    } else {
        format!("{n}")
    }
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
        _ => BareLiteral::Value(
            literal_number(raw).unwrap_or_else(|| serde_json::Value::String(raw.to_string())),
        ),
    }
}

/// Returns `true` when a bare (unquoted) string value round-trips through the
/// literal grammar as the *same* string — i.e. it is neither `true`/`false`/
/// `null` nor numberlike. Serialization uses this to decide whether a literal
/// string value must be quoted so it does not re-type on re-parse.
pub(super) fn bare_literal_round_trips_as_string(value: &str) -> bool {
    !matches!(value, "true" | "false" | "null") && literal_number(value).is_none()
}

/// Parses a numberlike-shaped bare token into a typed JSON number, preserving
/// integer vs. float. Rejects any spelling that is not `^-?\d+(\.\d+)?$` and
/// leading-zero integers (`007`), which stay strings (no octal surprises, per
/// the Q1 ruling).
fn literal_number(raw: &str) -> Option<serde_json::Value> {
    if !is_literal_numberlike(raw) {
        return None;
    }
    serde_json::Number::from_str(raw)
        .ok()
        .map(serde_json::Value::Number)
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
            SimplifiedType::Any,
        ] {
            let atom = parse(ty.as_keyword());
            assert_eq!(atom.ty, TypeExpr::Primitive(ty));
        }
    }

    #[test]
    fn rejects_unknown_type() {
        let err = parse_err("widget");
        let SchemaError::Grammar { message, .. } = err else {
            panic!("expected Grammar error, got {err:?}")
        };
        assert!(message.contains("unknown type"));
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
        // as an unknown constraint (never the generic unknown-type error).
        let SchemaError::Grammar { message, .. } = parse_err("expression(condition)") else {
            panic!("expected Grammar error");
        };
        assert!(!message.contains("unknown type"), "got: {message}");
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
        // `second` is treated as the next property start; since `second`
        // is not a known type and the `:` separator is missing, the
        // parser reports a grammar error. Decision #9 leaves escaping
        // out of scope for this feature.
        let err = parse_err("{ foo: string -> first, second }");
        let SchemaError::Grammar { message, .. } = err else {
            panic!("expected Grammar error, got {err:?}")
        };
        // The exact message depends on whether the trailing `}` looks
        // like the end of a property definition or a type-name error.
        assert!(
            message.contains("unknown type")
                || message.contains("expected a type name")
                || message.contains("`:`")
                || message.contains("expected `:`"),
            "unexpected message: {message}"
        );
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
