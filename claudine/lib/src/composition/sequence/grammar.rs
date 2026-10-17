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
use super::super::types::{LoopLocation, LoopOwner};

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

    // A literal token is stored data. Read as `{{ … }}` it would become an
    // expression, and decoded it would be data choosing the run's steps.
    if crate::composition::closure::opens_stored_token(trimmed) {
        return Err(CompositionError::SequenceInvalid(
            "`sequence` holds a stored literal token (`{{!data:…}}`), which is data; \
             author the list, file reference, expression, or shell expansion directly"
                .to_string(),
        ));
    }

    if let Some(inner) = whole_value_span(trimmed, "{{", "}}") {
        return Ok(SequenceSourceSpec::Expression(inner.trim().to_string()));
    }
    if let Some(inner) = whole_value_span(trimmed, "$(", ")") {
        return Ok(SequenceSourceSpec::Shell(inner.trim().to_string()));
    }
    // A frontmatter suffix (`::ok`, `::result`, `::timeout:5`, …) turns a value
    // into something other than a list, or tunes a frontmatter command; a
    // sequence source is neither, so name that rather than misreading the
    // suffix as a reference operator.
    if let Some(shell) = darkmatter::markdown::compose::parse_frontmatter_shell_value_spanned(trimmed)
        && let Some(suffix) = trimmed.get(shell.close_span.end..)
        && suffix.starts_with("::")
    {
        return Err(CompositionError::SequenceInvalid(format!(
            "`sequence` shell source `{trimmed}` takes no suffix: `{suffix}` applies to a \
             frontmatter value, and a sequence source must produce a list"
        )));
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

/// Container event keys carried by inline sequences and group definitions.
pub const CONTAINER_EVENT_KEYS: &[&str] = &["initialize", "start", "success", "failure", "blocked", "cancelled", "finalize"];

/// Validate task grammar before preparation can execute any work.
///
/// Inline containers are validated transitively, an inline group's events and
/// legs included, so a nested container's events fail here, before the root
/// `initialize` runs, at any depth. Referenced containers are judged when
/// static preflight loads them. `property` is the task's authored path in
/// `origin` (`sequence[1]`, or empty for a file whose root is the task); it
/// locates the task `when`/`loop` diagnostics.
pub fn validate_task_grammar(map: &serde_json::Map<String, Value>, origin: &std::path::Path, property: &str) -> Result<(), CompositionError> {
    for key in ["setup", "teardown", "execution", "fail_fast"] {
        if let Some(value) = map.get(key) {
            return Err(removed_task_key(key, value, origin));
        }
    }
    let inline = matches!(map.get("sequence"), Some(Value::Array(_)));
    if !inline && let Some(key) = CONTAINER_EVENT_KEYS.iter().find(|key| map.contains_key(**key)) {
        return Err(CompositionError::RemovedGrammar {
            source_path: origin.to_path_buf(), path: (*key).into(), removed: (*key).into(),
            replacement: "container events belong on an inline `sequence:` list or inside a `group:` object; put referenced sequence events in its target document".into(),
        });
    }
    validate_container_events(map, origin)?;
    task_when(map, origin, property)?;
    task_loop(map, origin, property)?;
    if let Some(Value::Object(group)) = map.get("group") {
        let group_property = child_property(property, "group");
        validate_container_events(group, origin)?;
        group_object_grammar(group, origin, &group_property)?;
        for (index, leg) in group.get("tasks").and_then(Value::as_array).into_iter().flatten().enumerate() {
            let Value::Object(leg) = leg else {
                // Same wording as `build_group`, which judges a group file's
                // legs; here the file is the one declaring the inline group.
                return Err(CompositionError::SequenceGroupInvalid {
                    group: group.get("name").and_then(Value::as_str).unwrap_or("<inline>").to_string(),
                    origin: origin.to_path_buf(),
                    problem: format!("task {} must be an object, got {}", index + 1, json_type_name(leg)),
                });
            };
            validate_task_grammar(leg, origin, &format!("{group_property}.tasks[{index}]"))?;
        }
    }
    if let Some(value) = map.get("sequence") {
        match value {
            Value::Array(items) => {
                super::normalize::normalize_plan_at(
                    items, super::model::SequenceSource::Inline, origin, true, &child_property(property, "sequence"),
                )?;
            }
            Value::String(_) => {
                if !matches!(classify_source(value)?, SequenceSourceSpec::Reference(_)) {
                    return Err(CompositionError::SequenceInvalid("task `sequence` must be a list or file reference; dynamic sources belong on the sequence document".into()));
                }
            }
            other => return Err(CompositionError::SequenceInvalid(format!("task `sequence` must be a list or file reference, got {}", json_type_name(other)))),
        }
    }
    Ok(())
}

/// Join an authored property path and a child key; an empty parent is the
/// document root.
pub(crate) fn child_property(parent: &str, child: &str) -> String {
    if parent.is_empty() { child.to_string() } else { format!("{parent}.{child}") }
}

/// Validate a task's `when:` guard.
///
/// The guard must be a non-empty string that parses with the same condition
/// grammar as a lifecycle `when`. It is evaluated at the task's turn.
///
/// ## Errors
///
/// [`CompositionError::WhenMisplaced`] on a step without an executable, and
/// [`CompositionError::TaskWhenInvalid`] for a non-string, empty, or
/// unparsable guard.
fn task_when(map: &serde_json::Map<String, Value>, origin: &std::path::Path, property: &str) -> Result<(), CompositionError> {
    let Some(value) = map.get("when") else { return Ok(()) };
    let property = child_property(property, "when");
    if !super::reserved::EXECUTABLE_KEYS.iter().any(|key| map.contains_key(*key)) {
        return Err(CompositionError::WhenMisplaced {
            source_path: origin.to_path_buf(), property,
            hint: "`when` guards a task and goes beside its executable; a step that runs the document body takes no guard".into(),
        });
    }
    let invalid = |message: String, source| CompositionError::TaskWhenInvalid {
        source_path: origin.to_path_buf(), property: property.clone(), message, source,
    };
    match value {
        Value::String(text) if text.trim().is_empty() => {
            return Err(invalid("`when` must not be an empty expression; remove the key to run the task unconditionally".into(), None));
        }
        Value::String(text) => {
            darkmatter::markdown::compose::expression::parse_condition(text).map_err(|source| {
                invalid(format!("`when` is not a valid expression: {source}"), Some(source))
            })?;
        }
        other => return Err(invalid(format!("`when` must be an expression string, got {}", json_type_name(other)), None)),
    }
    Ok(())
}

/// Place a task-level `loop:`: an inline nested sequence's loop is a sequence
/// loop and goes to [`container_loop`]; on any other task it is misplaced, and
/// the error names where that task's loop belongs.
fn task_loop(map: &serde_json::Map<String, Value>, origin: &std::path::Path, property: &str) -> Result<(), CompositionError> {
    if !map.contains_key("loop") {
        return Ok(());
    }
    if matches!(map.get("sequence"), Some(Value::Array(_))) {
        return container_loop(map, LoopOwner::Sequence, &LoopLocation { source_path: origin.to_path_buf(), property: child_property(property, "loop") }).map(drop);
    }
    let executable = super::reserved::EXECUTABLE_KEYS.iter().copied().find(|key| map.contains_key(*key));
    let hint = match executable {
        Some("group") if map.get("group").is_some_and(Value::is_object) => {
            "a group's loop goes on its group object, beside `tasks:` (`group: { loop: …, tasks: […] }`)"
        }
        Some("group") => "this task references a group; put `loop:` in that group file's object, beside its `tasks:`",
        Some("sequence") => "this task references a sequence document; put `loop:` at that document's root, beside its `sequence:`",
        Some("prompt") => "a prompt task repeats through its document's own frontmatter `loop:`; or wrap the task in an inline `sequence:` and put `loop:` beside that",
        Some("task") => "this task references a task file; wrap it in an inline `sequence:` and put `loop:` beside that",
        Some(_) => "shell and side-effect tasks repeat only inside a looping container; wrap the task in an inline `sequence:` and put `loop:` beside that",
        None => "a step that runs the document body has no loop of its own; put `loop:` at the sequence document's root, or on an inline `sequence:` or a `group:` object",
    };
    Err(CompositionError::TaskLoopMisplaced { source_path: origin.to_path_buf(), property: child_property(property, "loop"), hint: hint.into() })
}

/// Check a group object's own keys: `loop:` goes to [`container_loop`], and
/// `when:` is a task key, not a group key.
///
/// ## Errors
///
/// [`CompositionError::WhenMisplaced`] for `when`, and the loop parser's
/// errors for `loop`.
pub(crate) fn group_object_grammar(group: &serde_json::Map<String, Value>, origin: &std::path::Path, property: &str) -> Result<(), CompositionError> {
    if group.contains_key("when") {
        return Err(CompositionError::WhenMisplaced {
            source_path: origin.to_path_buf(), property: child_property(property, "when"),
            hint: "`when` is a task key; put it beside `group:` on the task that runs the group".into(),
        });
    }
    container_loop(group, LoopOwner::Group, &LoopLocation { source_path: origin.to_path_buf(), property: child_property(property, "loop") }).map(drop)
}

/// Check the root keys of a sequence document (Markdown frontmatter, a
/// standalone YAML file, or a referenced formal document) that belong to this
/// grammar rather than to the lifecycle parser.
///
/// `when:` is rejected: the document body is not a step, so it has nothing to
/// guard. `loop:` is a sequence loop, validated by the loop parser
/// (`container_loop`). Run this before the root's lifecycle events are
/// parsed, so a malformed root loop gets the loop parser's diagnostic rather
/// than a lifecycle one.
///
/// ## Errors
///
/// [`CompositionError::WhenMisplaced`] for a root `when`, and the loop
/// parser's errors for a root `loop`.
pub fn validate_sequence_root(root: &serde_json::Map<String, Value>, origin: &std::path::Path) -> Result<(), CompositionError> {
    if root.contains_key("when") {
        return Err(CompositionError::WhenMisplaced {
            source_path: origin.to_path_buf(), property: "when".into(),
            hint: "a sequence document's body is not a step, so its root takes no guard; `when` is a task key, written beside a step's executable".into(),
        });
    }
    container_loop(root, LoopOwner::Sequence, &LoopLocation::document_root(origin)).map(drop)
}

/// Parse the container `loop:` in `map`, if any, with `owner`'s grammar, so
/// an invalid loop is its own located error.
pub(crate) fn container_loop(
    map: &serde_json::Map<String, Value>, owner: LoopOwner, location: &LoopLocation,
) -> Result<Option<super::super::looping::LoopBlock>, CompositionError> {
    map.get("loop").map(|value| super::super::looping::parse_loop_block(value, owner, location)).transpose()
}

/// Parse each container event `map` carries, one at a time.
fn validate_container_events(map: &serde_json::Map<String, Value>, origin: &std::path::Path) -> Result<(), CompositionError> {
    for key in CONTAINER_EVENT_KEYS {
        if let Some(value) = map.get(*key) {
            let mut root = serde_json::Map::new();
            root.insert((*key).into(), value.clone());
            super::super::lifecycle::parse_lifecycle_config(&Value::Object(root), origin)?;
        }
    }
    Ok(())
}

pub(crate) fn removed_task_key(key: &str, value: &Value, origin: &std::path::Path) -> CompositionError {
    let replacement = match key {
        "setup" => "use a preceding `side_effect:` task for an action or `shell:` task for a command",
        "teardown" => "use a group's `finalize:`; wrap a single task in a group of one",
        "execution" if value.as_str() == Some("serial") => "use `sequence: [{name: first, shell: command}]` for ordered work; nest that sequence as a group leg when needed",
        "execution" => "groups run concurrently; omit `execution:` (including `execution: parallel`)",
        "fail_fast" => "`fail_fast` belongs on a `group:` object only; use a group of one with `fail_fast: false` to continue after failure",
        _ => unreachable!(),
    };
    CompositionError::RemovedGrammar { source_path: origin.to_path_buf(), path: key.into(), removed: key.into(), replacement: replacement.into() }
}
