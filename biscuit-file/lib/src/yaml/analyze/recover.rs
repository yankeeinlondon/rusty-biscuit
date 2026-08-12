//! S3 parse-recovery: quoting plain scalars that begin with a reserved YAML
//! indicator when the original document fails to parse.
//!
//! The bounded grammar (decision D3) accepts only:
//!
//! - a block-mapping value (`key: <lexeme>`) or block-sequence entry
//!   (`- <lexeme>`) at any nesting depth, where `<lexeme>` is a single-line
//!   scalar whose first byte is a reserved indicator;
//! - the parser's structured error location landing on or immediately after
//!   that lexeme;
//! - a lexeme with no ` #` (comment/content ambiguity), no `"` or `\`
//!   (double-quoting must reproduce the lexeme byte-for-byte), and no flow
//!   collection intersection.
//!
//! The dedicated proof substitutes for the unavailable `Value` equality of
//! the S1 gate: the candidate must parse, the parsed node at the lexically
//! scanned context path must be a string byte-equal to the original lexeme,
//! and multi-error documents iterate one repair per round (bounded at 8) at
//! a strictly advancing error offset. Exhaustion or any failed proof leaves
//! the document in its original bytes with all findings report-only — a
//! partial recovery chain is never emitted.

use serde_yaml_ng::Value;

use super::analysis::YamlParseFailure;
use super::diagnostic::{YamlCertainty, YamlDiagnostic, YamlDiagnosticCode, YamlRepair};
use super::engine::parse_value;
use super::scan::{LineKind, PathSegment, SourceMap};
use crate::span::SourceSpan;

/// Maximum number of single-error repair rounds before the chain is
/// abandoned (decision D3).
const MAX_ROUNDS: usize = 8;

/// Indicator characters that may start an S3-repairable lexeme (D3).
const INDICATORS: &[u8] = b"`@%&*!|>'\",[]{}#:?-";

/// One accepted (or provisionally accepted) repair round.
struct RoundEdit {
    /// Byte span of the lexeme in the *original* source.
    original_span: SourceSpan,
    /// Quoted replacement text.
    replacement: String,
    /// Lexical context path used by the parse-recovery proof.
    path: Vec<PathSegment>,
    /// The original lexeme text.
    lexeme: String,
    /// First byte of the lexeme (the reserved indicator).
    first_byte: u8,
}

/// Runs the S3 parse-recovery chain for unparseable `source`.
///
/// On success, returns one `Deterministic` [`YamlDiagnosticCode::ReservedIndicator`]
/// diagnostic per repaired lexeme (spans in original-source coordinates).
/// On any failure, returns report-only diagnostics: the identified
/// reserved-indicator findings (without repairs) plus the parse diagnostic.
pub(super) fn recover(
    source: &str,
    map: &SourceMap,
    failure: &YamlParseFailure,
) -> Vec<YamlDiagnostic> {
    let mut edits: Vec<RoundEdit> = Vec::new();
    let mut current = source.to_string();
    let mut current_failure = failure.clone();
    // Bytes added to the source by accepted edits so far; every accepted
    // edit adds exactly two quote bytes, so original spans are current spans
    // minus this delta.
    let mut delta = 0usize;

    for round in 0..MAX_ROUNDS {
        let round_map;
        let active_map = if round == 0 {
            map
        } else {
            round_map = SourceMap::new(&current);
            &round_map
        };
        let Some(error_byte) = current_failure.location.map(|location| location.byte) else {
            return fail(source, failure, &edits);
        };
        let Some(found) = find_lexeme(&current, active_map, error_byte) else {
            return fail(source, failure, &edits);
        };
        let lexeme_text = current[found.lexeme.clone()].to_string();
        let replacement = format!("\"{lexeme_text}\"");
        // End of the repaired region in candidate coordinates (the lexeme
        // plus its two quote bytes).
        let repaired_end = found.lexeme.end + 2;
        let mut candidate = current.clone();
        candidate.replace_range(found.lexeme.clone(), &replacement);
        let original_span = found.lexeme.start - delta..found.lexeme.end - delta;
        if let Some(previous) = edits.last()
            && original_span.start < previous.original_span.end
        {
            return fail(source, failure, &edits);
        }
        let edit = RoundEdit {
            original_span,
            replacement,
            path: found.path,
            lexeme: lexeme_text,
            first_byte: found.first_byte,
        };
        match parse_value(&candidate) {
            Ok(value) => {
                edits.push(edit);
                if proven(&value, &edits) {
                    return success(edits);
                }
                return fail(source, failure, &edits);
            }
            Err(error) => {
                let Some(next) = error.location().map(|location| location.index()) else {
                    return fail(source, failure, &edits);
                };
                // The repair must advance the parser strictly past the
                // repaired lexeme.
                if next <= repaired_end {
                    return fail(source, failure, &edits);
                }
                edits.push(edit);
                delta += 2;
                current = candidate;
                current_failure = YamlParseFailure {
                    message: error.to_string(),
                    location: error.location().map(Into::into),
                };
            }
        }
    }
    fail(source, failure, &edits)
}

/// A lexeme matched by the bounded grammar.
struct FoundLexeme {
    path: Vec<PathSegment>,
    lexeme: SourceSpan,
    first_byte: u8,
}

/// Applies the D3 bounded grammar at the parser's error location.
fn find_lexeme(source: &str, map: &SourceMap, error_byte: usize) -> Option<FoundLexeme> {
    let line_index = map.line_at_byte(error_byte)?;
    let line = &map.lines()[line_index];
    if line.kind != LineKind::Content {
        return None;
    }
    let context = map.block_value_context(source, line_index)?;
    let lexeme = context.lexeme;
    // The parser error must land on or immediately after the scalar.
    if error_byte < lexeme.start || error_byte > line.content.end {
        return None;
    }
    // Flow-collection contexts are report-only.
    if map.flow_intersects(&lexeme) {
        return None;
    }
    let text = &source[lexeme.clone()];
    let first_byte = *text.as_bytes().first()?;
    if !INDICATORS.contains(&first_byte) {
        return None;
    }
    // Unterminated quoted scalars follow a different, unratified grammar.
    if first_byte == b'\'' || first_byte == b'"' {
        return None;
    }
    // Double-quoting must reproduce the lexeme byte-for-byte: no escapes.
    if text.contains('"') || text.contains('\\') {
        return None;
    }
    // Comment-versus-content ambiguity stays report-only.
    if text.contains(" #") {
        return None;
    }
    Some(FoundLexeme {
        path: context.path,
        lexeme,
        first_byte,
    })
}

/// The parse-recovery proof: every repaired lexeme must navigate, via its
/// lexically scanned context path, to a parsed string byte-equal to the
/// original lexeme text.
fn proven(value: &Value, edits: &[RoundEdit]) -> bool {
    edits.iter().all(|edit| {
        let mut node = value;
        for segment in &edit.path {
            let next = match segment {
                PathSegment::Key(key) => node.get(key.as_str()),
                PathSegment::Index(index) => node.get(*index),
            };
            match next {
                Some(inner) => node = inner,
                None => return false,
            }
        }
        matches!(node, Value::String(text) if text == &edit.lexeme)
    })
}

/// Successful chain: one deterministic diagnostic per repaired lexeme.
fn success(edits: Vec<RoundEdit>) -> Vec<YamlDiagnostic> {
    edits
        .into_iter()
        .map(|edit| {
            let indicator = edit.first_byte as char;
            YamlDiagnostic {
                code: YamlDiagnosticCode::ReservedIndicator,
                span: edit.original_span.clone(),
                classification: YamlCertainty::Deterministic,
                message: format!(
                    "plain scalar begins with the reserved YAML indicator `{indicator}` and does not parse"
                ),
                repairs: vec![YamlRepair {
                    span: edit.original_span,
                    replacement: edit.replacement,
                    explanation:
                        "quote the scalar so the indicator is treated as string content"
                            .to_string(),
                }],
            }
        })
        .collect()
}

/// Failed or abandoned chain: every finding is report-only and no repair is
/// attached, so the document stays in its original bytes.
fn fail(source: &str, failure: &YamlParseFailure, edits: &[RoundEdit]) -> Vec<YamlDiagnostic> {
    let mut diagnostics: Vec<YamlDiagnostic> = edits
        .iter()
        .map(|edit| {
            let indicator = edit.first_byte as char;
            YamlDiagnostic {
                code: YamlDiagnosticCode::ReservedIndicator,
                span: edit.original_span.clone(),
                classification: YamlCertainty::DeterministicFindNonDeterministicSolution,
                message: format!(
                    "plain scalar begins with the reserved YAML indicator `{indicator}` and does not parse"
                ),
                repairs: Vec::new(),
            }
        })
        .collect();
    let span = failure.location.map_or(0..0, |location| {
        let byte = location.byte.min(source.len());
        byte..(byte + 1).min(source.len())
    });
    diagnostics.push(YamlDiagnostic {
        code: YamlDiagnosticCode::Parse,
        span,
        classification: YamlCertainty::DeterministicFindNonDeterministicSolution,
        message: failure.message.clone(),
        repairs: Vec::new(),
    });
    diagnostics
}
