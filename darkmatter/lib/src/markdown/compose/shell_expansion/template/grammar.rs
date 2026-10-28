//! The authored template grammar: the resolved-byte tokenizer's quoting and
//! curated redirections, with each complete `{{ … }}` span recognized first
//! and kept as a placeholder inside its containing argument.

use super::{
    ArgPart, CommandTemplate, Placeholder, RedirectOp, TemplateArg, TemplateGrammarError, TemplateSegment,
    HoleType, TypeProvenance, project_hole, project_hole_expr,
};
use crate::markdown::compose::expression::{DeclaredTypes, ExpressionFinder, PreparedInterpolationView, TypeScope};
use crate::markdown::compose::shell_expansion::types::ChainOperator;
use std::ops::Range;

/// Parses an authored shell command into a [`CommandTemplate`].
///
/// Each `{{ … }}` span is found with Darkmatter's interpolation scanner before
/// any quote, whitespace, or operator is interpreted, so a span's own spaces,
/// quotes, and `||` never split or chain the command. A placeholder stays
/// inside the argument that contains it, quoted or not. Placeholders are typed
/// as if the document had no schema and no guard; call
/// [`CommandTemplate::project_types`] with the site's type scope.
///
/// ## Examples
///
/// ```
/// use darkmatter::markdown::compose::shell_expansion::template::{ArgPart, parse_authored};
///
/// let template = parse_authored(r#"just clean --out "{{ out || 'a b' }}/build""#).unwrap();
/// let out = &template.segments[0].args[2];
/// assert!(matches!(&out.parts[..], [ArgPart::Hole(_), ArgPart::Literal(rest)] if rest == "/build"));
/// ```
///
/// ## Errors
///
/// Returns a [`TemplateGrammarError`] for a placeholder in a program word or
/// redirection target, and for every construct template mode does not run:
/// pipes, `;`, a lone `&`, `<`, `>>`, redirection to a file, `$(`, unterminated
/// quotes, interpolation literals, and literal control characters other than
/// newline, carriage return, and tab.
pub fn parse_authored(text: &str) -> Result<CommandTemplate, TemplateGrammarError> {
    let scan = ExpressionFinder::scan_plain(text);
    if !scan.literals.is_empty() || !scan.tokens.is_empty() {
        return Err(TemplateGrammarError::InterpolationLiteral);
    }
    let holes: Vec<(Range<usize>, String)> = scan
        .expressions
        .into_iter()
        .map(|location| (location.start..location.end, location.expression))
        .collect();
    let declared = SchemaFree::new();
    let scope = declared.scope();
    build(text, holes, &|_, expression| project_hole(expression, &scope))
}

/// Parses the authored command a prepared view covers, typing each
/// placeholder from the syntax the view already retains.
///
/// Matches [`parse_authored`] result for result, but parses nothing: a
/// placeholder whose retained expression failed keeps the fallback type, as
/// the text path gives an expression that does not parse. Holes are matched to
/// retained occurrences by span, so repeated text stays distinct.
///
/// ## Errors
///
/// The same [`TemplateGrammarError`]s as [`parse_authored`].
pub fn parse_authored_prepared(view: PreparedInterpolationView<'_>) -> Result<CommandTemplate, TemplateGrammarError> {
    if view.has_literal_syntax() {
        return Err(TemplateGrammarError::InterpolationLiteral);
    }
    let holes: Vec<(Range<usize>, String)> = view
        .placeholders()
        .map(|placeholder| (placeholder.span(), placeholder.text().to_string()))
        .collect();
    let declared = SchemaFree::new();
    let scope = declared.scope();
    build(view.text(), holes, &|span, _| {
        view.placeholder_at(span)
            .and_then(|placeholder| placeholder.expr())
            .map_or_else(
                || HoleType::fallback(TypeProvenance::NotBareProperty),
                |expr| project_hole_expr(expr, &scope),
            )
    })
}

/// The schema-free, guard-free types a template's holes have until
/// `project_types` retypes them at their site.
struct SchemaFree<'s>(DeclaredTypes<'s>);

impl SchemaFree<'_> {
    fn new() -> Self {
        Self(DeclaredTypes::new(None))
    }

    fn scope(&self) -> TypeScope<'_> {
        TypeScope::unguarded(&self.0)
    }
}

/// The shared grammar body; `typing` gives a hole its initial type from its
/// whole-placeholder span and expression text.
fn build(
    text: &str,
    holes: Vec<(Range<usize>, String)>,
    typing: &dyn Fn(&Range<usize>, &str) -> HoleType,
) -> Result<CommandTemplate, TemplateGrammarError> {
    let mut parser = Parser {
        text,
        pos: 0,
        holes,
        typing,
        next_hole: 0,
        segments: Vec::new(),
        words: Vec::new(),
        word: None,
        redirects: Vec::new(),
        operator: ChainOperator::None,
    };
    parser.run()?;
    Ok(CommandTemplate {
        authored: text.to_string(),
        segments: parser.segments,
    })
}

struct Parser<'a> {
    text: &'a str,
    pos: usize,
    holes: Vec<(Range<usize>, String)>,
    typing: &'a dyn Fn(&Range<usize>, &str) -> HoleType,
    next_hole: usize,
    segments: Vec<TemplateSegment>,
    words: Vec<Vec<ArgPart>>,
    /// The word being built; `Some` once any character, quote, or hole starts it.
    word: Option<Vec<ArgPart>>,
    redirects: Vec<RedirectOp>,
    operator: ChainOperator,
}

impl Parser<'_> {
    fn run(&mut self) -> Result<(), TemplateGrammarError> {
        while let Some(ch) = self.peek() {
            if self.take_hole() {
                continue;
            }
            match ch {
                ' ' | '\t' | '\n' | '\r' => {
                    self.bump();
                    self.end_word();
                }
                '\'' => {
                    self.bump();
                    self.start_word();
                    self.single_quoted()?;
                }
                '"' => {
                    self.bump();
                    self.start_word();
                    self.double_quoted()?;
                }
                '\\' => {
                    self.bump();
                    let next = self.bump().ok_or(TemplateGrammarError::Unterminated)?;
                    match next {
                        ' ' | '\'' | '"' | '\\' => self.push_char(next)?,
                        _ => {
                            self.push_char('\\')?;
                            self.push_char(next)?;
                        }
                    }
                }
                '&' => {
                    self.bump();
                    if self.peek() != Some('&') {
                        return Err(TemplateGrammarError::Background);
                    }
                    self.bump();
                    self.end_segment(ChainOperator::And)?;
                }
                '|' => {
                    self.bump();
                    if self.peek() != Some('|') {
                        return Err(TemplateGrammarError::Pipe);
                    }
                    self.bump();
                    self.end_segment(ChainOperator::Or)?;
                }
                ';' => return Err(TemplateGrammarError::Semicolon),
                '<' => return Err(TemplateGrammarError::InputRedirect),
                '>' => {
                    self.bump();
                    self.end_word();
                    self.redirect_after_stdout()?;
                }
                '$' if self.text[self.pos..].starts_with("$(") => {
                    return Err(TemplateGrammarError::CommandSubstitution);
                }
                '2' if self.word.is_none() && self.text[self.pos..].starts_with("2>") => {
                    self.pos += 2;
                    self.redirect_after_stderr()?;
                }
                _ => {
                    self.bump();
                    self.push_char(ch)?;
                }
            }
        }
        self.end_segment(ChainOperator::None)?;
        if self.segments.is_empty() {
            return Err(TemplateGrammarError::Empty);
        }
        Ok(())
    }

    fn peek(&self) -> Option<char> {
        self.text[self.pos..].chars().next()
    }

    fn bump(&mut self) -> Option<char> {
        let ch = self.peek()?;
        self.pos += ch.len_utf8();
        Some(ch)
    }

    /// Consumes a placeholder starting at the cursor into the current word.
    fn take_hole(&mut self) -> bool {
        let Some((span, expression)) = self.holes.get(self.next_hole) else {
            return false;
        };
        if span.start != self.pos {
            return false;
        }
        let placeholder = Placeholder {
            expression: expression.clone(),
            span: span.clone(),
            typing: (self.typing)(span, expression),
            timing: super::HoleTiming::ApprovalTime,
        };
        self.pos = span.end;
        self.next_hole += 1;
        self.start_word();
        self.word.as_mut().expect("started").push(ArgPart::Hole(placeholder));
        true
    }

    fn start_word(&mut self) {
        self.word.get_or_insert_with(Vec::new);
    }

    fn push_char(&mut self, ch: char) -> Result<(), TemplateGrammarError> {
        if ch.is_control() && !matches!(ch, '\n' | '\r' | '\t') {
            return Err(TemplateGrammarError::ControlCharacter);
        }
        self.start_word();
        let word = self.word.as_mut().expect("started");
        match word.last_mut() {
            Some(ArgPart::Literal(text)) => text.push(ch),
            _ => word.push(ArgPart::Literal(ch.to_string())),
        }
        Ok(())
    }

    fn end_word(&mut self) {
        if let Some(word) = self.word.take() {
            self.words.push(word);
        }
    }

    fn single_quoted(&mut self) -> Result<(), TemplateGrammarError> {
        loop {
            if self.take_hole() {
                continue;
            }
            match self.bump().ok_or(TemplateGrammarError::Unterminated)? {
                '\'' => return Ok(()),
                ch => self.push_char(ch)?,
            }
        }
    }

    fn double_quoted(&mut self) -> Result<(), TemplateGrammarError> {
        loop {
            if self.take_hole() {
                continue;
            }
            match self.bump().ok_or(TemplateGrammarError::Unterminated)? {
                '"' => return Ok(()),
                '\\' => match self.bump().ok_or(TemplateGrammarError::Unterminated)? {
                    next @ ('\\' | '"' | ' ') => self.push_char(next)?,
                    next => {
                        self.push_char('\\')?;
                        self.push_char(next)?;
                    }
                },
                ch => self.push_char(ch)?,
            }
        }
    }

    /// After `>`: `>&2`, or `> /dev/null`.
    fn redirect_after_stdout(&mut self) -> Result<(), TemplateGrammarError> {
        match self.peek() {
            Some('>') => Err(TemplateGrammarError::ArbitraryRedirect),
            Some('&') => {
                self.bump();
                if self.peek() == Some('2') {
                    self.bump();
                    self.redirects.push(RedirectOp::StdoutToStderr);
                    Ok(())
                } else {
                    Err(TemplateGrammarError::ArbitraryRedirect)
                }
            }
            _ => {
                self.dev_null_target()?;
                self.redirects.push(RedirectOp::StdoutNull);
                Ok(())
            }
        }
    }

    /// After `2>`: `2>&1`, or `2> /dev/null`.
    fn redirect_after_stderr(&mut self) -> Result<(), TemplateGrammarError> {
        if self.peek() == Some('&') {
            self.bump();
            if self.peek() == Some('1') {
                self.bump();
                self.redirects.push(RedirectOp::StderrToStdout);
                return Ok(());
            }
            return Err(TemplateGrammarError::ArbitraryRedirect);
        }
        self.dev_null_target()?;
        self.redirects.push(RedirectOp::StderrNull);
        Ok(())
    }

    fn dev_null_target(&mut self) -> Result<(), TemplateGrammarError> {
        while matches!(self.peek(), Some(' ' | '\t')) {
            self.bump();
        }
        let rest = &self.text[self.pos..];
        let target_len = rest
            .find(|ch: char| ch.is_whitespace() || "&|;<>'\"`\\$".contains(ch))
            .unwrap_or(rest.len());
        // The next unconsumed hole starts at or after the cursor; one that
        // starts inside the target word puts a value in the target.
        if self
            .holes
            .get(self.next_hole)
            .is_some_and(|(span, _)| span.start < self.pos + target_len)
        {
            return Err(TemplateGrammarError::RedirectHole);
        }
        if &rest[..target_len] != "/dev/null" {
            return Err(TemplateGrammarError::ArbitraryRedirect);
        }
        self.pos += target_len;
        Ok(())
    }

    fn end_segment(&mut self, next: ChainOperator) -> Result<(), TemplateGrammarError> {
        self.end_word();
        let mut words = std::mem::take(&mut self.words).into_iter();
        let redirects = std::mem::take(&mut self.redirects);
        let Some(program) = words.next() else {
            // Nothing at all is fine only at the very end of an empty input,
            // which `run` reports; anything else is a dangling operator.
            if next == ChainOperator::None && self.operator == ChainOperator::None && redirects.is_empty() {
                return Ok(());
            }
            return Err(TemplateGrammarError::Empty);
        };
        let program = match TemplateArg::normalized(program).parts.as_slice() {
            [ArgPart::Literal(text)] if !text.is_empty() => text.clone(),
            [ArgPart::Literal(_)] => return Err(TemplateGrammarError::Empty),
            _ => return Err(TemplateGrammarError::ProgramWordHole),
        };
        self.segments.push(TemplateSegment {
            operator: self.operator,
            program,
            args: words.map(TemplateArg::normalized).collect(),
            redirects,
        });
        self.operator = next;
        Ok(())
    }
}
