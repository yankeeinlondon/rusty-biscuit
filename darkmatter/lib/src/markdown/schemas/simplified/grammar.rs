//! Lexer and parser for the SimplifiedSchema type-and-constraint string.
//!
//! Each property value (after the YAML-shape layer has decided this is a
//! single type expression) is parsed by [`parse_type_expr`] following the
//! EBNF in the schemas spec:
//!
//! ```text
//! type_expr_string := type_expr ( "->" description )?
//! type_expr        := type_name ( "(" item_constraints ")" )?
//!                                ( "[]" ( "(" arr_constraints ")" )? )?
//! type_name        := "string" | "date" | "datetime" | "time" | "number"
//!                   | "numberlike" | "boolean" | "boolish" | "object"
//!                   | "file" | "enum" | "url" | "email" | "any"
//! item_constraints := constraint ( ";" constraint )*
//! arr_constraints  := constraint ( ";" constraint )*
//! constraint       := IDENT
//!                   | IDENT "(" arglist ")"
//! arglist          := arg ( "," arg )*
//! arg              := NUMBER | BARE_WORD | SQUOTED | DQUOTED
//! description      := <rest-of-string, trimmed>
//! ```
//!
//! Errors surface as [`SchemaError::Grammar`] with the byte span of the
//! offending token.

use std::ops::Range;

use crate::markdown::schemas::errors::SchemaError;

use super::types::{Constraint, PropertyAtom, SimplifiedType};

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

struct Lexer<'a> {
    src: &'a str,
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Lexer<'a> {
    fn new(src: &'a str) -> Self {
        Self {
            src,
            bytes: src.as_bytes(),
            pos: 0,
        }
    }

    fn peek_byte(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn skip_ws(&mut self) {
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
                out.push(b as char);
                self.pos += 1;
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
    fn read_word(&mut self) -> (String, Range<usize>) {
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
    fn read_ident(&mut self) -> (String, Range<usize>) {
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
}

impl<'a> Parser<'a> {
    fn new(property: &'a str, src: &'a str) -> Self {
        Self {
            property,
            src,
            lex: Lexer::new(src),
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

        Ok(PropertyAtom {
            ty,
            is_array,
            constraints: item_constraints,
            array_constraints,
            description: None,
        })
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

        loop {
            self.lex.skip_ws();
            let constraint = self.parse_one_constraint(ty, is_array_level)?;
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
            ("not-empty", false) => Constraint::NotEmpty,
            ("integer", false) => Constraint::Integer,
            ("unique", false) => Constraint::Unique,
            ("required", true) => {
                return self.err("`required` does not take arguments", kw_span);
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
                    lex: format_number(n),
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
        assert_eq!(atom.ty, SimplifiedType::String);
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
            SimplifiedType::Any,
        ] {
            let atom = parse(ty.as_keyword());
            assert_eq!(atom.ty, ty);
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
        let atom = parse("number(default(3))");
        assert_eq!(
            atom.constraints,
            vec![Constraint::Default(serde_json::json!(3.0))]
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

    #[test]
    fn parses_file_match() {
        let atom = parse("file(match('*.md', '!_*.md'))");
        assert_eq!(
            atom.constraints,
            vec![Constraint::Match(vec!["*.md".into(), "!_*.md".into()])]
        );
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
        assert_eq!(atom.ty, SimplifiedType::String);
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
}
