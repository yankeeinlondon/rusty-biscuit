//! Trigger-schema envelope parsing.
//!
//! A trigger schema is a standalone YAML file claiming the
//! `kind: trigger-schema` envelope — the same envelope-claiming pattern as
//! `kind: schema`. The envelope carries:
//!
//! - `match:` (required) — the match-expression payload parsed by
//!   [`super::grammar`].
//! - `$schema:` (optional, deferred) — the payload reference layered onto
//!   matching documents. Resolution is deferred to Phase 4; this module
//!   records the raw authored value without resolving it.
//!
//! Unknown top-level keys are rejected (never silently ignored). The envelope
//! performs no I/O: it validates shape, parses the match grammar, and runs
//! the vacuous-trigger lint, then hands the parsed result back to the caller.
//!
//! See the "Repository Trigger Schemas" section of
//! `darkmatter/docs/topics/schemas/definition.md`.

use serde_yaml_ng::Value as YamlValue;

use crate::markdown::schemas::errors::SchemaError;

use super::grammar::{MatchArms, parse_match_arms};
use super::lint;

/// The `kind:` value that claims a trigger-schema envelope.
pub const ENVELOPE_KIND: &str = "trigger-schema";

/// The recognized top-level keys of a trigger-schema envelope.
const KNOWN_KEYS: &[&str] = &["kind", "match", "$schema"];

/// A parsed `kind: trigger-schema` envelope.
///
/// The match payload is fully parsed and linted. The `$schema:` payload
/// reference is recorded raw — resolution (and merge-compatibility gating)
/// happens in Phase 4's effective-schema assembly.
#[derive(Debug, Clone, PartialEq)]
pub struct TriggerEnvelope {
    /// The parsed, lint-clean match arms (OR'd).
    pub match_arms: MatchArms,
    /// The raw `$schema:` value, when present. Deferred to Phase 4.
    pub payload: Option<YamlValue>,
}

/// Classifies and parses a `kind: trigger-schema` envelope from YAML source.
///
/// Returns `Ok(None)` when the document does not claim the trigger-schema
/// envelope (no `kind: trigger-schema`). Once the envelope is claimed, a
/// malformed payload (missing `match:`, bad match grammar, vacuous trigger,
/// unknown top-level keys) is a hard [`SchemaError`].
///
/// Does **not** resolve the `$schema:` payload — that is Phase 4's job.
pub fn parse_trigger_envelope(value: &YamlValue) -> Result<Option<TriggerEnvelope>, SchemaError> {
    let Some(map) = value.as_mapping() else {
        return Ok(None);
    };

    let kind_value = map
        .get(YamlValue::String("kind".into()))
        .and_then(YamlValue::as_str);
    if kind_value != Some(ENVELOPE_KIND) {
        return Ok(None);
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
                "trigger-schema envelope supports only `kind`, `match`, and `$schema`; found \
                 unknown keys: {}",
                unknown.join(", ")
            ),
        });
    }

    // `match:` is required.
    let match_value = map
        .get(YamlValue::String("match".into()))
        .ok_or_else(|| SchemaError::TriggerMatch {
            message: "trigger-schema envelope requires a `match:` key".into(),
        })?;

    let match_arms = parse_match_arms(match_value)?;
    // Vacuous-trigger lint: one vacuous arm makes the whole trigger vacuous.
    lint::lint(&match_arms)?;

    let payload = map
        .get(YamlValue::String("$schema".into()))
        .cloned();

    Ok(Some(TriggerEnvelope {
        match_arms,
        payload,
    }))
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
/// discovery. A claimed file returns [`SchemaError::TriggerEnvelopeYaml`].
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
        Err(_) if !claims_envelope(&source) => Ok(None),
        Err(err) => {
            let location = err.location();
            let mut message = err.to_string();
            if let Some(location) = &location {
                // The parser's text repeats the location the variant carries.
                let suffix = format!(" at line {} column {}", location.line(), location.column());
                if let Some(at) = message.find(&suffix) {
                    message.replace_range(at..at + suffix.len(), "");
                }
            }
            Err(SchemaError::TriggerEnvelopeYaml {
                message,
                line: location.as_ref().map(|location| location.line()),
                column: location.as_ref().map(|location| location.column()),
            })
        }
    }
}

/// Whether source that failed to parse as YAML still declares the envelope.
///
/// A declaration is a line at column 0 reading `kind`, optional spaces, `:`,
/// then exactly `trigger-schema` (plain, single-, or double-quoted) and at
/// most a trailing `#` comment. This is error recovery, not a YAML grammar:
/// a line counts only when it is provably a top-level block-mapping entry, so
/// lines inside a multi-line quoted scalar, a block scalar, or an open flow
/// collection never claim, and anything this scan cannot place is unclaimed.
fn claims_envelope(source: &str) -> bool {
    let mut scan = ClaimScan::default();
    source.lines().any(|line| scan.line(line))
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
    /// Consumes one line and reports whether it is a provable declaration.
    fn line(&mut self, line: &str) -> bool {
        let indent = line.len() - line.trim_start_matches(' ').len();
        let blank = line.trim().is_empty();
        if self.root_block {
            return false;
        }
        if let Some(parent) = self.block_parent {
            if blank || indent > parent {
                return false;
            }
            self.block_parent = None;
        }
        let provably_top_level = self.quote.is_none() && self.flow_depth == 0;
        let claim = provably_top_level && is_claim_line(line);
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

/// Whether `line` is exactly a column-0 `kind: trigger-schema` declaration.
fn is_claim_line(line: &str) -> bool {
    let Some(rest) = line.strip_prefix("kind") else {
        return false;
    };
    let Some(rest) = rest.trim_start_matches(' ').strip_prefix(':') else {
        return false;
    };
    if !rest.starts_with([' ', '\t']) {
        return false;
    }
    let rest = rest.trim_start();
    let after = [
        rest.strip_prefix(ENVELOPE_KIND),
        rest.strip_prefix("'trigger-schema'"),
        rest.strip_prefix("\"trigger-schema\""),
    ];
    after.into_iter().flatten().any(|after| {
        let trailing = after.trim_start();
        trailing.is_empty() || (after.len() > trailing.len() && trailing.starts_with('#'))
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
            serde_yaml_ng::from_str("kind: trigger-schema\nmatch:\n  prompt: string(required)")
                .unwrap();
        let env = parse_trigger_envelope(&v).unwrap().unwrap();
        assert_eq!(env.match_arms.0.len(), 1);
        assert!(env.payload.is_none());
    }

    #[test]
    fn records_payload_raw() {
        let v: YamlValue = serde_yaml_ng::from_str(
            "kind: trigger-schema\nmatch:\n  prompt: string(required)\n$schema: claudine.yaml",
        )
        .unwrap();
        let env = parse_trigger_envelope(&v).unwrap().unwrap();
        assert_eq!(env.payload.as_ref().and_then(YamlValue::as_str), Some("claudine.yaml"));
    }

    #[test]
    fn rejects_missing_match() {
        let v: YamlValue =
            serde_yaml_ng::from_str("kind: trigger-schema\n$schema: claudine.yaml").unwrap();
        let err = parse_trigger_envelope(&v).unwrap_err();
        assert!(matches!(err, SchemaError::TriggerMatch { .. }));
    }

    #[test]
    fn rejects_unknown_key() {
        let v: YamlValue = serde_yaml_ng::from_str(
            "kind: trigger-schema\nmatch:\n  prompt: string(required)\nfoo: bar",
        )
        .unwrap();
        let err = parse_trigger_envelope(&v).unwrap_err();
        assert!(matches!(err, SchemaError::TriggerMatch { .. }));
    }

    #[test]
    fn rejects_non_string_top_level_keys_by_their_yaml_text() {
        for (source, shown) in [
            ("kind: trigger-schema\nmatch:\n  prompt: string(required)\n42: extra", "42"),
            ("kind: trigger-schema\nmatch:\n  prompt: string(required)\ntrue: extra", "true"),
            ("kind: trigger-schema\nmatch:\n  prompt: string(required)\n~: extra", "null"),
            ("kind: trigger-schema\nmatch:\n  prompt: string(required)\n[a, b]: extra", "- a - b"),
            ("{\"kind\": \"trigger-schema\", \"match\": {\"prompt\": \"string(required)\"}, 42: \"x\"}", "42"),
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
            serde_yaml_ng::from_str("kind: trigger-schema\nmatch:\n  maybe: string").unwrap();
        let err = parse_trigger_envelope(&v).unwrap_err();
        assert!(matches!(err, SchemaError::TriggerVacuousArm));
    }
}

