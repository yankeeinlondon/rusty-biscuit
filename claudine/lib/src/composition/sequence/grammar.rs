//! The Claudine sequence-source grammar.
//!
//! A `sequence:` frontmatter value is classified into one of four source
//! variants, and file references additionally carry an optional offset and
//! operator suffix:
//!
//! ```text
//! sequence: <file-ref> [-> <offset.path>] [::<operator>(<args>)]
//! ```
//!
//! The suffix is Claudine's, not an extension to the shared `biscuit-file`
//! reference grammar: the `<file-ref>` portion is handed to
//! [`biscuit_file::FileReference`] untouched, so every reference family (`@`,
//! `!`, `~`, `vault:`, `{{ env }}` interpolation, spaced paths, and paths
//! containing `@`) keeps its usual meaning. Splitting is therefore span-aware —
//! an `->` or `::` inside a quoted operator argument or inside a `{{ … }}`
//! interpolation segment is literal text, not a separator.

use serde_json::Value;

use super::super::error::CompositionError;
use super::super::json_util::json_type_name;

/// A classified `sequence:` frontmatter value, before any I/O or evaluation.
#[derive(Debug, Clone, PartialEq)]
pub enum SequenceSourceSpec {
    /// An authored inline list. Typed items bypass `ListFormat` entirely.
    Inline(Vec<Value>),
    /// A whole-value `{{ … }}` span; evaluated through Darkmatter.
    Expression(String),
    /// A whole-value `$( … )` span; approved and executed, then classified.
    Shell(String),
    /// A file reference plus its optional offset and operator.
    Reference(SequenceReference),
}

/// A parsed file-reference source: the untouched reference text plus the
/// Claudine-owned suffix.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SequenceReference {
    /// The `<file-ref>` portion, verbatim — never string-surgered.
    pub reference: String,
    /// The dot-notation offset introduced by `->`, if present.
    pub offset: Option<String>,
    /// The single `::` operator, if present.
    pub operator: Option<SourceOperator>,
}

/// The one operator a reference may carry. Exactly one per reference in v1.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceOperator {
    /// `map(from, to)` — rename `from` to `to`, removing the original key.
    Map {
        /// The key to rename.
        from: String,
        /// The key it becomes.
        to: String,
    },
    /// `name(from)` — copy `from` into `name`, retaining the original key.
    Name {
        /// The key to copy from.
        from: String,
    },
    /// `template(expr)` — compute `name` per item with a Darkmatter expression.
    Template {
        /// The raw expression source, with the item's fields in scope.
        expr: String,
    },
}

impl SourceOperator {
    /// The operator name as authored, for error messages.
    pub fn verb(&self) -> &'static str {
        match self {
            Self::Map { .. } => "map",
            Self::Name { .. } => "name",
            Self::Template { .. } => "template",
        }
    }
}

/// Classify a raw `sequence:` frontmatter value into a source variant.
///
/// ## Errors
///
/// Returns [`CompositionError::SequenceInvalid`] for a value that is neither a
/// list nor a string, and the typed suffix-syntax variants for a malformed
/// reference suffix.
pub fn classify_source(value: &Value) -> Result<SequenceSourceSpec, CompositionError> {
    match value {
        Value::Array(items) => Ok(SequenceSourceSpec::Inline(items.clone())),
        Value::String(raw) => classify_string_source(raw),
        other => Err(CompositionError::SequenceInvalid(format!(
            "expected a list, a file reference, an expression, or a shell expansion, got {}",
            json_type_name(other)
        ))),
    }
}

/// Classify a string `sequence:` value.
///
/// A value that is *exactly* one `{{ … }}` or `$( … )` span is executable
/// state and produces a list directly; those two forms do not take the
/// offset/operator suffix. Everything else is a file reference.
fn classify_string_source(raw: &str) -> Result<SequenceSourceSpec, CompositionError> {
    let trimmed = raw.trim();

    if let Some(inner) = whole_value_span(trimmed, "{{", "}}") {
        return Ok(SequenceSourceSpec::Expression(inner.trim().to_string()));
    }
    if let Some(inner) = whole_value_span(trimmed, "$(", ")") {
        return Ok(SequenceSourceSpec::Shell(inner.trim().to_string()));
    }

    Ok(SequenceSourceSpec::Reference(parse_reference(trimmed)?))
}

/// Return the inner text when `input` is exactly one `open … close` span — that
/// is, the opener is at the start, the matching closer is at the very end, and
/// no intermediate close ends the span early (so `{{a}} {{b}}` is *not* a
/// whole-value span and stays a file reference).
fn whole_value_span<'a>(input: &'a str, open: &str, close: &str) -> Option<&'a str> {
    let rest = input.strip_prefix(open)?;
    let inner = rest.strip_suffix(close)?;
    if inner.contains(close) {
        return None;
    }
    Some(inner)
}

/// Parse `<file-ref> [-> <offset>] [::<operator>(<args>)]`.
fn parse_reference(input: &str) -> Result<SequenceReference, CompositionError> {
    let (before_operator, operator) = match find_span_aware(input, "::") {
        Some(at) => {
            let (head, tail) = input.split_at(at);
            (head, Some(parse_operator(input, &tail[2..])?))
        }
        None => (input, None),
    };

    let (reference, offset) = match find_span_aware(before_operator, "->") {
        Some(at) => {
            let (head, tail) = before_operator.split_at(at);
            let offset = tail[2..].trim();
            if offset.is_empty() {
                return Err(CompositionError::SequenceSourceSyntax {
                    authored: input.to_string(),
                    problem: "`->` must be followed by a dot-notation offset path".to_string(),
                });
            }
            (head.trim(), Some(offset.to_string()))
        }
        None => (before_operator.trim(), None),
    };

    if reference.is_empty() {
        return Err(CompositionError::SequenceSourceSyntax {
            authored: input.to_string(),
            problem: "missing the file reference before the suffix".to_string(),
        });
    }

    Ok(SequenceReference {
        reference: reference.to_string(),
        offset,
        operator,
    })
}

/// Parse the text following `::` into exactly one operator.
fn parse_operator(full: &str, tail: &str) -> Result<SourceOperator, CompositionError> {
    let tail = tail.trim();

    let open = tail.find('(').ok_or_else(|| CompositionError::SequenceSourceSyntax {
        authored: full.to_string(),
        problem: "an operator must be written `name(args)`".to_string(),
    })?;
    let verb = tail[..open].trim().to_string();

    let close = matching_paren(tail, open).ok_or_else(|| {
        CompositionError::SequenceSourceSyntax {
            authored: full.to_string(),
            problem: format!("operator `{verb}` is missing its closing `)`"),
        }
    })?;

    let trailing = tail[close + 1..].trim();
    if !trailing.is_empty() {
        let problem = if trailing.starts_with("::") {
            "only one operator is allowed per reference".to_string()
        } else {
            format!("unexpected trailing text after the operator: `{trailing}`")
        };
        return Err(CompositionError::SequenceSourceSyntax {
            authored: full.to_string(),
            problem,
        });
    }

    let args_text = &tail[open + 1..close];

    match verb.as_str() {
        // `template` takes one *expression*, not a comma-separated argument
        // list: splitting it would corrupt any expression containing a comma.
        "template" => {
            let expr = args_text.trim();
            if expr.is_empty() {
                return Err(CompositionError::SequenceOperatorArity {
                    operator: "template".to_string(),
                    expected: 1,
                    found: 0,
                });
            }
            Ok(SourceOperator::Template {
                expr: expr.to_string(),
            })
        }
        "map" | "name" => {
            let args = split_operator_args(args_text);
            match (verb.as_str(), args.len()) {
                ("map", 2) => Ok(SourceOperator::Map {
                    from: args[0].clone(),
                    to: args[1].clone(),
                }),
                ("name", 1) => Ok(SourceOperator::Name {
                    from: args[0].clone(),
                }),
                ("map", found) => Err(CompositionError::SequenceOperatorArity {
                    operator: "map".to_string(),
                    expected: 2,
                    found,
                }),
                (_, found) => Err(CompositionError::SequenceOperatorArity {
                    operator: "name".to_string(),
                    expected: 1,
                    found,
                }),
            }
        }
        other => Err(CompositionError::SequenceUnknownOperator(other.to_string())),
    }
}

/// Split an argument list on top-level commas, honoring quotes and nesting,
/// then trim and unquote each argument.
fn split_operator_args(input: &str) -> Vec<String> {
    if input.trim().is_empty() {
        return Vec::new();
    }

    let mut args = Vec::new();
    let mut current = String::new();
    let mut scanner = SpanScanner::default();

    for ch in input.chars() {
        if scanner.is_top_level() && ch == ',' {
            args.push(unquote(current.trim()));
            current.clear();
        } else {
            current.push(ch);
        }
        scanner.consume(ch);
    }
    args.push(unquote(current.trim()));
    args
}

/// Strip one matching pair of surrounding quotes.
fn unquote(input: &str) -> String {
    for quote in ['\'', '"'] {
        if input.len() >= 2 && input.starts_with(quote) && input.ends_with(quote) {
            return input[1..input.len() - 1].to_string();
        }
    }
    input.to_string()
}

/// Find the index of `needle` at top level — outside quotes, `{{ }}`
/// interpolation, and parentheses.
fn find_span_aware(haystack: &str, needle: &str) -> Option<usize> {
    let bytes = haystack.as_bytes();
    let needle_bytes = needle.as_bytes();
    let mut scanner = SpanScanner::default();

    for (index, ch) in haystack.char_indices() {
        if scanner.is_top_level()
            && index + needle_bytes.len() <= bytes.len()
            && &bytes[index..index + needle_bytes.len()] == needle_bytes
        {
            return Some(index);
        }
        scanner.consume(ch);
    }
    None
}

/// Find the `)` matching the `(` at `open`, honoring quotes and nesting.
fn matching_paren(input: &str, open: usize) -> Option<usize> {
    let mut depth = 0usize;
    let mut scanner = SpanScanner::default();

    for (index, ch) in input.char_indices() {
        if index < open {
            scanner.consume(ch);
            continue;
        }
        if !scanner.in_quotes() {
            if ch == '(' {
                depth += 1;
            } else if ch == ')' {
                depth -= 1;
                if depth == 0 {
                    return Some(index);
                }
            }
        }
        scanner.consume(ch);
    }
    None
}

/// Tracks quote, interpolation, and paren nesting while scanning left to right.
///
/// `consume` is called *after* each position is tested, so a character that
/// opens a span is itself still seen at the depth it opened from.
#[derive(Default)]
struct SpanScanner {
    in_single: bool,
    in_double: bool,
    interpolation: usize,
    parens: usize,
    prev: Option<char>,
}

impl SpanScanner {
    fn consume(&mut self, ch: char) {
        match ch {
            '\'' if !self.in_double => self.in_single = !self.in_single,
            '"' if !self.in_single => self.in_double = !self.in_double,
            '{' if !self.in_quotes() && self.prev == Some('{') => self.interpolation += 1,
            '}' if !self.in_quotes() && self.prev == Some('}') && self.interpolation > 0 => {
                self.interpolation -= 1;
            }
            '(' if !self.in_quotes() => self.parens += 1,
            ')' if !self.in_quotes() && self.parens > 0 => self.parens -= 1,
            _ => {}
        }
        // `{{{{` must read as two spans, not one span plus a stray brace, so a
        // consumed brace never pairs with the next one.
        self.prev = if matches!(ch, '{' | '}') && self.prev == Some(ch) {
            None
        } else {
            Some(ch)
        };
    }

    fn in_quotes(&self) -> bool {
        self.in_single || self.in_double
    }

    fn is_top_level(&self) -> bool {
        !self.in_quotes() && self.interpolation == 0 && self.parens == 0
    }
}
