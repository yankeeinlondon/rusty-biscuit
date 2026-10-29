//! Schema-trigger envelope parsing.
//!
//! A schema trigger is a standalone YAML file claiming the
//! `kind: schema-trigger` envelope — the same envelope-claiming pattern as
//! `kind: schema`. The envelope carries:
//!
//! - `match:` (required) — the match-expression payload parsed by
//!   [`super::grammar`].
//! - `$schema:` (optional) — the payload reference layered onto matching
//!   documents. This module records the raw authored value without
//!   resolving it; a trigger without one is matcher-only and layers nothing.
//!
//! `kind` decides whether a file claims the envelope. A missing `kind` or any
//! other string is ordinary data; `trigger-schema`, the spelling before the
//! rename, is a [`SchemaError::TriggerRenamed`]; and a null or non-string
//! `kind` is an error rather than ordinary data. Unknown top-level keys are
//! rejected (never silently ignored). The envelope
//! performs no I/O: it validates shape, parses the match grammar, and runs
//! the vacuous-trigger lint, then hands the parsed result back to the caller.
//!
//! See the "Repository Trigger Schemas" section of
//! `darkmatter/docs/topics/schemas/definition.md`.

use serde_yaml_ng::Value as YamlValue;

use crate::markdown::schemas::errors::SchemaError;

use super::grammar::{MatchConditions, parse_match_conditions};
use super::lint;

/// The `kind:` value that claims a schema-trigger envelope.
pub const ENVELOPE_KIND: &str = "schema-trigger";

/// The `kind:` spelling before the rename, rejected with no alias.
const RENAMED_KIND: &str = "trigger-schema";

/// The recognized top-level keys of a schema-trigger envelope.
const KNOWN_KEYS: &[&str] = &["kind", "match", "$schema"];

/// A parsed `kind: schema-trigger` envelope.
///
/// The match payload is fully parsed and linted. The `$schema:` payload
/// reference is recorded raw — resolution (and merge-compatibility gating)
/// happens in effective-schema assembly.
#[derive(Debug, Clone, PartialEq)]
pub struct TriggerEnvelope {
    /// The parsed, lint-clean top-level match conditions (AND'd).
    pub conditions: MatchConditions,
    /// The raw `$schema:` value; `None` for a matcher-only trigger.
    pub payload: Option<YamlValue>,
}

/// Classifies and parses a `kind: schema-trigger` envelope from YAML source.
///
/// Returns `Ok(None)` when the document does not claim the envelope: it is
/// not a mapping, or its `kind` is absent or a string other than
/// `schema-trigger` and `trigger-schema`. Once the envelope is claimed, a
/// malformed payload (missing `match:`, bad match grammar, vacuous trigger,
/// unknown top-level keys, a null or non-reference `$schema`) is a hard
/// [`SchemaError`].
///
/// Does **not** resolve the `$schema:` payload.
///
/// ## Errors
///
/// [`SchemaError::TriggerRenamed`] for `kind: trigger-schema` or `any:`;
/// [`SchemaError::TriggerMatch`] naming the field for a null or non-string
/// `kind` and every other envelope or grammar failure.
pub fn parse_trigger_envelope(value: &YamlValue) -> Result<Option<TriggerEnvelope>, SchemaError> {
    let Some(map) = value.as_mapping() else {
        return Ok(None);
    };

    match map.get(YamlValue::String("kind".into())) {
        None => return Ok(None),
        Some(YamlValue::String(kind)) if kind == ENVELOPE_KIND => {}
        Some(YamlValue::String(kind)) if kind == RENAMED_KIND => return Err(renamed_kind()),
        Some(YamlValue::String(_)) => return Ok(None),
        // A null `kind` is not ordinary data: it reads as a declaration whose
        // value was dropped, and treating it as unclaimed would hide a trigger.
        Some(YamlValue::Null) => {
            return Err(field_error("kind", "must be a string such as `schema-trigger`, got null".into()));
        }
        Some(other) => {
            return Err(field_error(
                "kind",
                format!("must be a string such as `schema-trigger`, got {}", yaml_kind(other)),
            ));
        }
    }

    // Reject unknown top-level keys. A non-string key (`42: extra`) is never
    // a known key, so it is reported too, rendered as its YAML text.
    let unknown: Vec<String> = map
        .keys()
        .filter(|key| key.as_str().is_none_or(|key| !KNOWN_KEYS.contains(&key)))
        .map(describe_key)
        .collect();
    if !unknown.is_empty() {
        return Err(SchemaError::TriggerMatch {
            message: format!(
                "schema-trigger envelope supports only `kind`, `match`, and `$schema`; found \
                 unknown keys: {}",
                unknown.join(", ")
            ),
        });
    }

    // `match:` is required.
    let match_value = map
        .get(YamlValue::String("match".into()))
        .ok_or_else(|| field_error("match", "a schema-trigger envelope requires a `match:` key".into()))?;

    let conditions = parse_match_conditions(match_value)?;
    lint::lint(&conditions)?;

    let payload = match map.get(YamlValue::String("$schema".into())) {
        None => None,
        Some(YamlValue::Null) => {
            return Err(field_error(
                "$schema",
                "an explicit null is not a payload; omit `$schema` for a matcher-only trigger".into(),
            ));
        }
        Some(payload @ (YamlValue::String(_) | YamlValue::Mapping(_) | YamlValue::Sequence(_))) => {
            Some(payload.clone())
        }
        Some(other) => {
            return Err(field_error(
                "$schema",
                format!("must be a schema reference or a schema mapping, got {}", yaml_kind(other)),
            ));
        }
    };

    Ok(Some(TriggerEnvelope {
        conditions,
        payload,
    }))
}

/// The error for `kind: trigger-schema`.
fn renamed_kind() -> SchemaError {
    SchemaError::TriggerRenamed {
        field: "kind".into(),
        found: RENAMED_KIND.into(),
        replacement: ENVELOPE_KIND.into(),
    }
}

/// A [`SchemaError::TriggerMatch`] about envelope field `field`.
fn field_error(field: &str, message: String) -> SchemaError {
    SchemaError::TriggerMatch { message: format!("`{field}`: {message}") }
}

fn yaml_kind(value: &YamlValue) -> &'static str {
    match value {
        YamlValue::Null => "null",
        YamlValue::Bool(_) => "boolean",
        YamlValue::Number(_) => "number",
        YamlValue::String(_) => "string",
        YamlValue::Sequence(_) => "sequence",
        YamlValue::Mapping(_) => "mapping",
        YamlValue::Tagged(_) => "tagged-value",
    }
}

/// A top-level key as the unknown-key message shows it: a string as-is,
/// anything else as its compact YAML text.
fn describe_key(key: &YamlValue) -> String {
    match key {
        YamlValue::String(text) => text.clone(),
        other => serde_yaml_ng::to_string(other)
            .map(|text| text.trim_end().replace('\n', " "))
            .unwrap_or_else(|_| format!("{other:?}")),
    }
}

/// Convenience wrapper that parses YAML source first.
///
/// Source that is not valid YAML is classified by [`claims_envelope`] before
/// the parse failure is treated as fatal: an unclaimed file is `Ok(None)`, so
/// an unrelated mid-edit YAML file in a schema root never fails trigger
/// discovery. A claimed file returns [`SchemaError::TriggerEnvelopeYaml`], or
/// [`SchemaError::TriggerRenamed`] when its claim uses `trigger-schema`.
///
/// An initial UTF-8 BOM is removed and CRLF line endings become LF before
/// parsing, because the YAML parser rejects a BOM followed by CRLF lines as a
/// multi-document stream. Reported line and column are those of the
/// normalized source: identical to the original's, except that a first-line
/// column does not count the BOM.
pub fn parse_trigger_envelope_from_str(source: &str) -> Result<Option<TriggerEnvelope>, SchemaError> {
    let source = source.strip_prefix('\u{feff}').unwrap_or(source);
    let source: std::borrow::Cow<'_, str> =
        if source.contains('\r') { source.replace("\r\n", "\n").into() } else { source.into() };
    match serde_yaml_ng::from_str::<YamlValue>(&source) {
        Ok(value) => parse_trigger_envelope(&value),
        Err(err) => match claims_envelope(&source) {
            None => Ok(None),
            Some(Claim::Renamed) => Err(renamed_kind()),
            Some(Claim::Current) => Err(yaml_error(&err)),
        },
    }
}

/// A claimed envelope's YAML failure, with the location moved out of the text.
fn yaml_error(err: &serde_yaml_ng::Error) -> SchemaError {
    let location = err.location();
    let mut message = err.to_string();
    if let Some(location) = &location {
        // The parser's text repeats the location the variant carries.
        let suffix = format!(" at line {} column {}", location.line(), location.column());
        if let Some(at) = message.find(&suffix) {
            message.replace_range(at..at + suffix.len(), "");
        }
    }
    SchemaError::TriggerEnvelopeYaml {
        message,
        line: location.as_ref().map(|location| location.line()),
        column: location.as_ref().map(|location| location.column()),
    }
}

/// Which spelling a provable declaration line uses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Claim {
    /// `kind: schema-trigger`.
    Current,
    /// `kind: trigger-schema`, rejected with the rename error.
    Renamed,
}

/// Whether source that failed to parse as YAML still declares the envelope.
///
/// A declaration is a line at column 0 reading `kind`, optional spaces, `:`,
/// then exactly `schema-trigger` or `trigger-schema` (plain, single-, or
/// double-quoted) and at most a trailing `#` comment; the first such line
/// decides. This is error recovery, not a YAML grammar: a line counts only
/// when it is provably a top-level block-mapping entry, so lines inside a
/// multi-line quoted scalar, a block scalar, or an open flow collection never
/// claim, and anything this scan cannot place is unclaimed.
fn claims_envelope(source: &str) -> Option<Claim> {
    let mut scan = ClaimScan::default();
    source.lines().find_map(|line| scan.line(line))
}

/// Cross-line state of [`claims_envelope`]'s scan.
#[derive(Default)]
struct ClaimScan {
    /// The quote character of a quoted scalar still open at the end of the
    /// previous line.
    quote: Option<char>,
    /// The indentation of the node owning an open block scalar; content lines
    /// are blank or indented further. `None` with `root_block` set means the
    /// document itself is a block scalar.
    block_parent: Option<usize>,
    root_block: bool,
    /// Depth of a flow collection still open at the end of the previous line.
    flow_depth: usize,
}

impl ClaimScan {
    /// Consumes one line and reports the declaration it provably is, if any.
    fn line(&mut self, line: &str) -> Option<Claim> {
        let indent = line.len() - line.trim_start_matches(' ').len();
        let blank = line.trim().is_empty();
        if self.root_block {
            return None;
        }
        if let Some(parent) = self.block_parent {
            if blank || indent > parent {
                return None;
            }
            self.block_parent = None;
        }
        let provably_top_level = self.quote.is_none() && self.flow_depth == 0;
        let claim = if provably_top_level { claim_line(line) } else { None };
        self.advance(line, indent);
        claim
    }

    /// Advances quote, flow, and block-scalar state across one line.
    fn advance(&mut self, line: &str, indent: usize) {
        let mut chars = line.chars().peekable();
        let mut at_scalar_start = true;
        let mut previous_space = true;
        let mut escaped = false;
        while let Some(ch) = chars.next() {
            if let Some(open) = self.quote {
                if escaped {
                    escaped = false;
                } else if open == '"' && ch == '\\' {
                    escaped = true;
                } else if ch == open {
                    if open == '\'' && chars.peek() == Some(&'\'') {
                        chars.next();
                    } else {
                        self.quote = None;
                    }
                }
                previous_space = false;
                continue;
            }
            if ch == '#' && previous_space {
                break;
            }
            match ch {
                '\'' | '"' if at_scalar_start => self.quote = Some(ch),
                '[' | '{' if at_scalar_start || self.flow_depth > 0 => self.flow_depth += 1,
                ']' | '}' if self.flow_depth > 0 => self.flow_depth -= 1,
                _ => {}
            }
            let next_is_space = chars.peek().is_none_or(|next| next.is_whitespace());
            at_scalar_start = (at_scalar_start && ch.is_whitespace())
                || (matches!(ch, ':' | '-' | '?') && next_is_space)
                || (self.flow_depth > 0 && matches!(ch, ',' | '[' | '{'));
            previous_space = ch.is_whitespace();
        }
        // A block scalar header (`|`, `>-`, `|2+`) closes the line's node.
        if self.quote.is_none() && self.flow_depth == 0 && is_block_scalar_header(line) {
            let trimmed = line.trim_start();
            if indent == 0 && (trimmed.starts_with(['|', '>']) || trimmed.starts_with("---")) {
                self.root_block = true;
            } else {
                self.block_parent = Some(indent);
            }
        }
    }
}

/// Whether the line's last value token (before any comment) is a block scalar
/// header following a value or sequence indicator, or opening the document.
fn is_block_scalar_header(line: &str) -> bool {
    let content = strip_comment(line).trim_end();
    let start = content.rfind(char::is_whitespace).map_or(0, |at| at + 1);
    let token = &content[start..];
    let mut rest = token.chars();
    if !matches!(rest.next(), Some('|' | '>')) || !rest.all(|ch| matches!(ch, '+' | '-' | '1'..='9')) {
        return false;
    }
    let before = content[..start].trim_end();
    before.is_empty() || before == "---" || before.ends_with(':') || before.ends_with('-') || before.ends_with('?')
}

/// The line up to a `#` comment that follows whitespace or starts the line.
fn strip_comment(line: &str) -> &str {
    let mut previous_space = true;
    for (at, ch) in line.char_indices() {
        if ch == '#' && previous_space {
            return &line[..at];
        }
        previous_space = ch.is_whitespace();
    }
    line
}

/// The declaration `line` is when it is exactly a column-0
/// `kind: schema-trigger` or `kind: trigger-schema` line.
fn claim_line(line: &str) -> Option<Claim> {
    let rest = line.strip_prefix("kind")?;
    let rest = rest.trim_start_matches(' ').strip_prefix(':')?;
    if !rest.starts_with([' ', '\t']) {
        return None;
    }
    let rest = rest.trim_start();
    [(ENVELOPE_KIND, Claim::Current), (RENAMED_KIND, Claim::Renamed)].into_iter().find_map(|(kind, claim)| {
        let after = [
            rest.strip_prefix(kind),
            rest.strip_prefix(&format!("'{kind}'")),
            rest.strip_prefix(&format!("\"{kind}\"")),
        ];
        after
            .into_iter()
            .flatten()
            .any(|after| {
                let trailing = after.trim_start();
                trailing.is_empty() || (after.len() > trailing.len() && trailing.starts_with('#'))
            })
            .then_some(claim)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ignores_non_trigger_document() {
        let v: YamlValue = serde_yaml_ng::from_str("$schema:\n  title: string").unwrap();
        assert!(parse_trigger_envelope(&v).unwrap().is_none());
    }

    #[test]
    fn ignores_kind_schema() {
        let v: YamlValue =
            serde_yaml_ng::from_str("kind: schema\ntypes:\n  title: string").unwrap();
        assert!(parse_trigger_envelope(&v).unwrap().is_none());
    }

    #[test]
    fn parses_minimal_envelope() {
        let v: YamlValue =
            serde_yaml_ng::from_str("kind: schema-trigger\nmatch:\n  prompt: string(required)")
                .unwrap();
        let env = parse_trigger_envelope(&v).unwrap().unwrap();
        assert_eq!(env.conditions.0.len(), 1);
        assert!(env.payload.is_none());
    }

    #[test]
    fn records_payload_raw() {
        let v: YamlValue = serde_yaml_ng::from_str(
            "kind: schema-trigger\nmatch:\n  prompt: string(required)\n$schema: claudine.yaml",
        )
        .unwrap();
        let env = parse_trigger_envelope(&v).unwrap().unwrap();
        assert_eq!(env.payload.as_ref().and_then(YamlValue::as_str), Some("claudine.yaml"));
    }

    #[test]
    fn rejects_missing_match() {
        let v: YamlValue =
            serde_yaml_ng::from_str("kind: schema-trigger\n$schema: claudine.yaml").unwrap();
        let err = parse_trigger_envelope(&v).unwrap_err();
        assert!(matches!(err, SchemaError::TriggerMatch { .. }));
    }

    #[test]
    fn rejects_unknown_key() {
        let v: YamlValue = serde_yaml_ng::from_str(
            "kind: schema-trigger\nmatch:\n  prompt: string(required)\nfoo: bar",
        )
        .unwrap();
        let err = parse_trigger_envelope(&v).unwrap_err();
        assert!(matches!(err, SchemaError::TriggerMatch { .. }));
    }

    #[test]
    fn rejects_non_string_top_level_keys_by_their_yaml_text() {
        for (source, shown) in [
            ("kind: schema-trigger\nmatch:\n  prompt: string(required)\n42: extra", "42"),
            ("kind: schema-trigger\nmatch:\n  prompt: string(required)\ntrue: extra", "true"),
            ("kind: schema-trigger\nmatch:\n  prompt: string(required)\n~: extra", "null"),
            ("kind: schema-trigger\nmatch:\n  prompt: string(required)\n[a, b]: extra", "- a - b"),
            ("{\"kind\": \"schema-trigger\", \"match\": {\"prompt\": \"string(required)\"}, 42: \"x\"}", "42"),
        ] {
            let v: YamlValue = serde_yaml_ng::from_str(source).unwrap();
            match parse_trigger_envelope(&v) {
                Err(SchemaError::TriggerMatch { message }) => assert!(
                    message.contains("unknown keys") && message.ends_with(shown),
                    "{source:?}: {message}"
                ),
                other => panic!("{source:?} was accepted: {other:?}"),
            }
        }
    }

    #[test]
    fn rejects_vacuous_trigger() {
        let v: YamlValue =
            serde_yaml_ng::from_str("kind: schema-trigger\nmatch:\n  maybe: string").unwrap();
        let err = parse_trigger_envelope(&v).unwrap_err();
        assert!(matches!(err, SchemaError::TriggerVacuousMatch));
    }

    #[test]
    fn the_old_kind_is_a_rename_error_parsed_or_not() {
        for source in [
            "kind: trigger-schema\nmatch:\n  prompt: string(required)",
            "kind: trigger-schema\nmatch:\n  prompt: string(required)\nnotes: a: b",
        ] {
            match parse_trigger_envelope_from_str(source) {
                Err(SchemaError::TriggerRenamed { field, found, replacement }) => {
                    assert_eq!((field.as_str(), found.as_str(), replacement.as_str()), ("kind", "trigger-schema", "schema-trigger"));
                }
                other => panic!("{source:?}: {other:?}"),
            }
        }
    }

    #[test]
    fn a_null_or_non_string_kind_is_an_error_and_other_strings_are_data() {
        for (source, shown) in [("kind: null", "got null"), ("kind: 42", "got number"), ("kind: [schema-trigger]", "got sequence")] {
            let v: YamlValue = serde_yaml_ng::from_str(source).unwrap();
            match parse_trigger_envelope(&v) {
                Err(SchemaError::TriggerMatch { message }) => {
                    assert!(message.starts_with("`kind`:") && message.ends_with(shown), "{source}: {message}");
                }
                other => panic!("{source:?}: {other:?}"),
            }
        }
        for source in ["kind: ''", "kind: schema", "title: x"] {
            let v: YamlValue = serde_yaml_ng::from_str(source).unwrap();
            assert!(parse_trigger_envelope(&v).unwrap().is_none(), "{source}");
        }
    }
}
