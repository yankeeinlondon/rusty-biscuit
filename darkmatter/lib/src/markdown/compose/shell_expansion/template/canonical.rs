//! The one-line `template <shape>` rule text.
//!
//! The writer emits a literal run bare when every character is safe and
//! otherwise double-quotes it, escaping `\\`, `\"`, `\{`, `\}`, `\n`, `\r`, and
//! `\t`. A hole is `{{ <type> }}`. The reader accepts both literal forms and
//! decodes escapes before it looks for holes, so an escaped `{{` is never a
//! hole. Rules compare as parsed shapes, never as text.

use super::{
    ApprovalType, RedirectOp, ShapeArg, ShapePart, ShapeSegment, StringLikeType, TemplateGrammarError,
    TemplateShape,
};
use crate::markdown::compose::shell_expansion::types::ChainOperator;
use crate::markdown::schemas::simplified::grammar::parse_type_expr;
use crate::markdown::schemas::{Constraint, SimplifiedType, TypeExpr};
use std::fmt::Write;

/// Why a `template` rule line was rejected. The loader adds the file and line.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TemplateRuleError {
    #[error("{0}")]
    Grammar(TemplateGrammarError),
    #[error("unknown escape '\\{0}' in a quoted literal")]
    InvalidEscape(char),
    #[error("'{0}' must be written inside a double-quoted literal")]
    UnquotedCharacter(char),
    #[error("a literal brace '{0}' inside quotes must be escaped as '\\{0}'")]
    UnescapedBrace(char),
    #[error("unterminated {{{{ placeholder")]
    UnterminatedHole,
    #[error("placeholder type '{text}' does not parse: {message}")]
    InvalidType { text: String, message: String },
    #[error("placeholder type '{0}' carries a constraint; rules hold only a type and its options")]
    Constraint(String),
    #[error("placeholder type '{0}' carries a description")]
    Annotation(String),
    #[error("placeholder type '{0}' names an unresolved named type")]
    NamedType(String),
    #[error("placeholder type '{0}' is not a supported string-like type")]
    UnsupportedType(String),
}

impl From<TemplateGrammarError> for TemplateRuleError {
    fn from(error: TemplateGrammarError) -> Self {
        Self::Grammar(error)
    }
}

impl TemplateShape {
    /// The rule text after the `template ` keyword.
    ///
    /// ## Examples
    ///
    /// ```
    /// use darkmatter::markdown::compose::shell_expansion::template::{TemplateShape, parse_authored};
    ///
    /// let template = parse_authored("just clean --out {{ out }}/build").unwrap();
    /// let shape = TemplateShape::from_template(&template);
    /// assert_eq!(shape.canonical_text(), "just clean --out {{ string }}/build");
    /// assert_eq!(TemplateShape::parse_rule(&shape.canonical_text()).unwrap(), shape);
    /// ```
    pub fn canonical_text(&self) -> String {
        let mut out = String::new();
        for segment in &self.segments {
            match segment.operator {
                ChainOperator::None => {}
                ChainOperator::And => out.push_str(" && "),
                ChainOperator::Or => out.push_str(" || "),
            }
            write_literal(&mut out, &segment.program);
            for arg in &segment.args {
                out.push(' ');
                for part in &arg.parts {
                    match part {
                        ShapePart::Literal(text) => write_literal(&mut out, text),
                        ShapePart::Hole(ty) => {
                            let _ = write!(out, "{{{{ {} }}}}", ty.text());
                        }
                    }
                }
            }
            for redirect in &segment.redirects {
                out.push(' ');
                out.push_str(redirect.as_str());
            }
        }
        out
    }

    /// Parses rule text written by [`canonical_text`](Self::canonical_text) or
    /// by hand.
    ///
    /// ## Errors
    ///
    /// A [`TemplateRuleError`] for malformed text, a program-word hole, and a
    /// placeholder type that is not one resolved, supported string-like type
    /// with only its type options: constraints (with or without `;`),
    /// descriptions, named types, arrays, unions, and other types are refused.
    pub fn parse_rule(text: &str) -> Result<Self, TemplateRuleError> {
        RuleParser { text, pos: 0 }.parse()
    }
}

impl super::CommandTemplate {
    /// The template as an approval prompt shows it: literal runs in the
    /// canonical rule spelling and each hole as `{{ <expression>: <type> }}`,
    /// with the validator disclosed when it is narrower than the approval
    /// type (see [`HoleType::display_type`](super::HoleType::display_type)).
    ///
    /// ## Examples
    ///
    /// ```
    /// use darkmatter::markdown::compose::shell_expansion::template::parse_authored;
    ///
    /// let template = parse_authored("just clean --out {{ out }}/build").unwrap();
    /// assert_eq!(template.display_text(), "just clean --out {{ out: string }}/build");
    /// ```
    pub fn display_text(&self) -> String {
        let mut out = String::new();
        for segment in &self.segments {
            match segment.operator {
                ChainOperator::None => {}
                ChainOperator::And => out.push_str(" && "),
                ChainOperator::Or => out.push_str(" || "),
            }
            write_literal(&mut out, &segment.program);
            for arg in &segment.args {
                out.push(' ');
                for part in &arg.parts {
                    match part {
                        super::ArgPart::Literal(text) => write_literal(&mut out, text),
                        super::ArgPart::Hole(placeholder) => {
                            let _ = write!(
                                out,
                                "{{{{ {}: {} }}}}",
                                placeholder.expression,
                                placeholder.typing.display_type()
                            );
                        }
                    }
                }
            }
            for redirect in &segment.redirects {
                out.push(' ');
                out.push_str(redirect.as_str());
            }
        }
        out
    }
}

/// Characters a bare literal run may hold. Anything else is quoted.
fn is_bare_safe(ch: char) -> bool {
    !(ch.is_whitespace() || ch.is_control() || "\"'\\{}&|;<>$`#".contains(ch))
}

/// Writes one literal run. Operators and redirections hold unsafe characters,
/// so a literal that looks like one is always quoted and re-reads as a word.
fn write_literal(out: &mut String, text: &str) {
    if !text.is_empty() && text.chars().all(is_bare_safe) {
        out.push_str(text);
        return;
    }
    out.push('"');
    for ch in text.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '{' => out.push_str("\\{"),
            '}' => out.push_str("\\}"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(ch),
        }
    }
    out.push('"');
}

struct RuleParser<'a> {
    text: &'a str,
    pos: usize,
}

enum RuleToken {
    Word(Vec<ShapePart>),
    Operator(ChainOperator),
    Redirect(RedirectOp),
}

impl RuleParser<'_> {
    fn parse(mut self) -> Result<TemplateShape, TemplateRuleError> {
        let mut segments = Vec::new();
        let mut words: Vec<Vec<ShapePart>> = Vec::new();
        let mut redirects = Vec::new();
        let mut operator = ChainOperator::None;
        while let Some(token) = self.next_token()? {
            match token {
                RuleToken::Word(parts) => words.push(parts),
                RuleToken::Redirect(op) => redirects.push(op),
                RuleToken::Operator(next) => {
                    segments.push(segment(operator, std::mem::take(&mut words), std::mem::take(&mut redirects))?);
                    operator = next;
                }
            }
        }
        segments.push(segment(operator, words, redirects)?);
        Ok(TemplateShape { segments })
    }

    fn rest(&self) -> &str {
        &self.text[self.pos..]
    }

    fn next_token(&mut self) -> Result<Option<RuleToken>, TemplateRuleError> {
        let skipped = self.rest().len() - self.rest().trim_start_matches([' ', '\t']).len();
        self.pos += skipped;
        let rest = self.rest();
        if rest.is_empty() {
            return Ok(None);
        }
        for (spelling, token) in [
            ("&&", RuleToken::Operator(ChainOperator::And)),
            ("||", RuleToken::Operator(ChainOperator::Or)),
            ("2>&1", RuleToken::Redirect(RedirectOp::StderrToStdout)),
            (">&2", RuleToken::Redirect(RedirectOp::StdoutToStderr)),
            ("2> /dev/null", RuleToken::Redirect(RedirectOp::StderrNull)),
            ("> /dev/null", RuleToken::Redirect(RedirectOp::StdoutNull)),
        ] {
            let ends_word = |tail: &str| tail.chars().next().is_none_or(|ch| ch == ' ' || ch == '\t');
            if rest.strip_prefix(spelling).is_some_and(ends_word) {
                self.pos += spelling.len();
                return Ok(Some(token));
            }
        }
        self.word().map(|parts| Some(RuleToken::Word(parts)))
    }

    fn word(&mut self) -> Result<Vec<ShapePart>, TemplateRuleError> {
        let mut parts = Vec::new();
        let mut literal = String::new();
        while let Some(ch) = self.rest().chars().next() {
            if ch == ' ' || ch == '\t' {
                break;
            }
            if ch.is_control() {
                return Err(TemplateGrammarError::ControlCharacter.into());
            }
            if self.rest().starts_with("{{") {
                if !literal.is_empty() {
                    parts.push(ShapePart::Literal(std::mem::take(&mut literal)));
                }
                parts.push(ShapePart::Hole(self.hole()?));
                continue;
            }
            self.pos += ch.len_utf8();
            match ch {
                '"' => self.quoted(&mut literal)?,
                _ if is_bare_safe(ch) => literal.push(ch),
                _ => return Err(TemplateRuleError::UnquotedCharacter(ch)),
            }
        }
        // A quoted empty run is a present, empty literal.
        parts.push(ShapePart::Literal(literal));
        Ok(parts)
    }

    fn quoted(&mut self, literal: &mut String) -> Result<(), TemplateRuleError> {
        let mut chars = self.rest().char_indices();
        while let Some((offset, ch)) = chars.next() {
            match ch {
                '"' => {
                    self.pos += offset + 1;
                    return Ok(());
                }
                '\\' => {
                    let (_, escaped) = chars.next().ok_or(TemplateGrammarError::Unterminated)?;
                    literal.push(match escaped {
                        '\\' | '"' | '{' | '}' => escaped,
                        'n' => '\n',
                        'r' => '\r',
                        't' => '\t',
                        other => return Err(TemplateRuleError::InvalidEscape(other)),
                    });
                }
                '{' | '}' => return Err(TemplateRuleError::UnescapedBrace(ch)),
                _ if ch.is_control() => return Err(TemplateGrammarError::ControlCharacter.into()),
                _ => literal.push(ch),
            }
        }
        Err(TemplateGrammarError::Unterminated.into())
    }

    /// A `{{ type }}` hole. The closing `}}` is the first one outside a quoted
    /// string, so a quoted `enum` member may hold braces.
    fn hole(&mut self) -> Result<ApprovalType, TemplateRuleError> {
        let body_start = self.pos + 2;
        let mut quote: Option<char> = None;
        let mut chars = self.text[body_start..].char_indices().peekable();
        while let Some((offset, ch)) = chars.next() {
            match (quote, ch) {
                (Some(open), _) if ch == open => quote = None,
                (Some(_), _) => {}
                (None, '\'' | '"') => quote = Some(ch),
                (None, '}') if chars.peek().is_some_and(|(_, next)| *next == '}') => {
                    let body = &self.text[body_start..body_start + offset];
                    self.pos = body_start + offset + 2;
                    return parse_rule_type(body.trim());
                }
                _ => {}
            }
        }
        Err(TemplateRuleError::UnterminatedHole)
    }
}

fn segment(
    operator: ChainOperator,
    words: Vec<Vec<ShapePart>>,
    redirects: Vec<RedirectOp>,
) -> Result<ShapeSegment, TemplateRuleError> {
    let mut words = words.into_iter().map(ShapeArg::normalized);
    let program = match words.next() {
        None => return Err(TemplateGrammarError::Empty.into()),
        Some(word) => match word.as_literal() {
            Some("") => return Err(TemplateGrammarError::Empty.into()),
            Some(text) => text.to_string(),
            None => return Err(TemplateGrammarError::ProgramWordHole.into()),
        },
    };
    Ok(ShapeSegment {
        operator,
        program,
        args: words.collect(),
        redirects,
    })
}

/// A rule placeholder's type: `string`, or one resolved, supported
/// string-like type with no constraints and no description.
fn parse_rule_type(text: &str) -> Result<ApprovalType, TemplateRuleError> {
    let atom = parse_type_expr("template", text).map_err(|error| TemplateRuleError::InvalidType {
        text: text.to_string(),
        message: error.to_string(),
    })?;
    if atom.description.is_some() {
        return Err(TemplateRuleError::Annotation(text.to_string()));
    }
    if matches!(atom.ty, TypeExpr::Imported { .. }) {
        return Err(TemplateRuleError::NamedType(text.to_string()));
    }
    let identity_only = atom
        .constraints
        .iter()
        .chain(&atom.array_constraints)
        .all(|constraint| matches!(constraint, Constraint::Options(_) | Constraint::Members(_) | Constraint::LiteralValue(_)));
    if !identity_only {
        return Err(TemplateRuleError::Constraint(text.to_string()));
    }
    if !atom.is_array && atom.ty == TypeExpr::Primitive(SimplifiedType::String) {
        return Ok(ApprovalType::BroadString);
    }
    StringLikeType::from_atom(&atom)
        .map(ApprovalType::StringLike)
        .ok_or_else(|| TemplateRuleError::UnsupportedType(text.to_string()))
}
