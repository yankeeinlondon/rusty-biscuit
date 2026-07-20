//! The source-first YAML analysis engine.
//!
//! [`analyze_yaml`] is the single entry point: it parses the source exactly
//! once, retains the structured outcome, and generates bounded, local repair
//! candidates only for matching diagnostics. Every candidate is proven
//! against the parser before it is attached to a diagnostic:
//!
//! - **S1 (parse-equivalent)**: source normalization and whitespace cleanup
//!   candidates are accepted only when the patched source reparses to a
//!   `serde_yaml_ng::Value` exactly equal to the original's.
//! - **S3 (parse-recovery)**: when the original does not parse at all, the
//!   reserved-indicator quoting algorithm in [`recover`] applies its own
//!   dedicated proof chain.
//!
//! Documents without diagnostics carry no candidates and are never reparsed.

use std::sync::atomic::{AtomicUsize, Ordering};

use serde_yaml_ng::Value;

use super::analysis::{YamlAnalysis, YamlParseFailure, YamlParseOutcome};
use super::diagnostic::{YamlCertainty, YamlDiagnostic, YamlDiagnosticCode, YamlRepair};
use super::edit_set::apply_edit_set;
use super::recover;
use super::report;
use super::scan::{LineKind, SourceMap};
use crate::span::SourceSpan;

/// Parse counter backing the parse-once instrumentation tests.
static ANALYZE_PARSE_COUNT: AtomicUsize = AtomicUsize::new(0);

/// Number of `serde_yaml_ng` parses the analyzer has performed since the
/// last [`reset_analyze_parse_count`]. Test instrumentation only.
#[doc(hidden)]
#[must_use]
pub fn analyze_parse_count() -> usize {
    ANALYZE_PARSE_COUNT.load(Ordering::Relaxed)
}

/// Resets the analyzer parse counter. Test instrumentation only.
#[doc(hidden)]
pub fn reset_analyze_parse_count() {
    ANALYZE_PARSE_COUNT.store(0, Ordering::Relaxed);
}

/// Every analyzer parse goes through here so the instrumentation counter
/// reflects the true parse count.
pub(super) fn parse_value(source: &str) -> Result<Value, serde_yaml_ng::Error> {
    ANALYZE_PARSE_COUNT.fetch_add(1, Ordering::Relaxed);
    serde_yaml_ng::from_str::<Value>(source)
}

/// Analyzes YAML source text, returning structured diagnostics for both
/// parseable and unparseable input.
///
/// Parseable input is checked for source normalization and parse-equivalent
/// whitespace candidates (S1). Unparseable input enters the bounded
/// reserved-indicator parse-recovery algorithm (S3). Both outcomes then run
/// the report-only layer ([`report`]): duplicate keys, anchor/alias
/// conditions, multiple documents, and the schema-free lints. Report-only
/// diagnostics never carry repairs, so they can never reach edit
/// application.
#[must_use]
pub fn analyze_yaml(source: &str) -> YamlAnalysis {
    let outcome = match parse_value(source) {
        Ok(value) => YamlParseOutcome::Parsed(value),
        Err(error) => YamlParseOutcome::Failed(YamlParseFailure {
            message: error.to_string(),
            location: error.location().map(Into::into),
        }),
    };
    let map = SourceMap::new(source);
    let mut diagnostics = match &outcome {
        YamlParseOutcome::Parsed(value) => {
            let drafts = s1_candidates(source, &map);
            prove_drafts(source, value, drafts)
                .into_iter()
                .map(Draft::into_diagnostic)
                .collect()
        }
        YamlParseOutcome::Failed(failure) => match bom_recovery(source) {
            Some(diagnostic) => vec![diagnostic],
            None => recover::recover(source, &map, failure),
        },
    };
    diagnostics.extend(report::report(source, &map));
    diagnostics.sort_by_key(|diagnostic| (diagnostic.span.start, diagnostic.span.end));
    YamlAnalysis::new(source, outcome, diagnostics)
}

/// A candidate diagnostic plus its single candidate repair, before the
/// safety proof has run.
struct Draft {
    code: YamlDiagnosticCode,
    span: SourceSpan,
    message: String,
    repair: YamlRepair,
}

impl Draft {
    fn new(
        code: YamlDiagnosticCode,
        span: SourceSpan,
        message: &str,
        replacement: &str,
        explanation: &str,
    ) -> Self {
        Self {
            code,
            span: span.clone(),
            message: message.to_string(),
            repair: YamlRepair {
                span,
                replacement: replacement.to_string(),
                explanation: explanation.to_string(),
            },
        }
    }

    fn into_diagnostic(self) -> YamlDiagnostic {
        YamlDiagnostic {
            code: self.code,
            span: self.span,
            classification: YamlCertainty::Deterministic,
            message: self.message,
            repairs: vec![self.repair],
        }
    }
}

/// Generates every S1 candidate for parseable input: source normalization
/// (BOM, line endings, trailing whitespace, final newline) and
/// parse-equivalent whitespace cleanup (mapping colons, sequence markers,
/// flow collections).
fn s1_candidates(source: &str, map: &SourceMap) -> Vec<Draft> {
    let mut drafts = Vec::new();
    bom_candidate(source, &mut drafts);
    line_ending_candidates(source, &mut drafts);
    let superfluous = final_newline_candidates(source, map, &mut drafts);
    trailing_whitespace_candidates(source, map, superfluous.as_ref(), &mut drafts);
    block_whitespace_candidates(source, map, &mut drafts);
    flow_whitespace_candidates(source, map, &mut drafts);
    drafts
}

/// Proves S1 candidates against the ratified safety matrix: the combined
/// edit set must reparse to a value exactly equal to the original's.
///
/// When the combined proof fails (overlaps or value drift), each candidate
/// is proven individually and the surviving set is re-proven combined, so
/// pairwise interactions cannot slip through. Candidates that still fail are
/// dropped entirely — an unproven S1 candidate is never reported.
fn prove_drafts(source: &str, original: &Value, drafts: Vec<Draft>) -> Vec<Draft> {
    if drafts.is_empty() {
        return drafts;
    }
    let repairs: Vec<YamlRepair> = drafts.iter().map(|draft| draft.repair.clone()).collect();
    let combined = apply_edit_set(source, &repairs);
    if combined.audit.rejected.is_empty() && parses_equal(&combined.source, original) {
        return drafts;
    }
    let mut survivors = Vec::new();
    for draft in drafts {
        let single = apply_edit_set(source, std::slice::from_ref(&draft.repair));
        if single.audit.rejected.is_empty() && parses_equal(&single.source, original) {
            survivors.push(draft);
        }
    }
    if survivors.len() <= 1 {
        return survivors;
    }
    let repairs: Vec<YamlRepair> = survivors.iter().map(|draft| draft.repair.clone()).collect();
    let combined = apply_edit_set(source, &repairs);
    if combined.audit.rejected.is_empty() && parses_equal(&combined.source, original) {
        survivors
    } else {
        Vec::new()
    }
}

fn parses_equal(source: &str, expected: &Value) -> bool {
    match parse_value(source) {
        Ok(value) => value == *expected,
        Err(_) => false,
    }
}

/// Recovers a document whose *only* defect is a leading BOM.
///
/// The parser reads a byte-order mark as a document boundary, so a BOM
/// followed by more than one top-level key is rejected as a multi-document
/// stream — a single-key block happens to survive, which is why this only
/// shows up on real frontmatter. Unparseable input never reaches
/// [`s1_candidates`], so without this the mark is not removed *and* neither
/// are the CRLF line endings that accompany it on every Windows-authored
/// file, and `md clean` then fails the document outright.
///
/// The proof is S3's, not S1's: the stripped source parses and the original
/// did not, so removing the mark is deterministic without a `Value`
/// comparison (there is no original value to compare against). Only the BOM
/// edit is emitted; the parse-equivalent tier stays unreachable on this pass
/// and lands on the rescan `md clean` performs once an edit restores
/// parseability.
fn bom_recovery(source: &str) -> Option<YamlDiagnostic> {
    let stripped = source.strip_prefix('\u{FEFF}')?;
    parse_value(stripped).ok()?;
    Some(
        Draft::new(
            YamlDiagnosticCode::Bom,
            0..'\u{FEFF}'.len_utf8(),
            "UTF-8 byte-order mark at stream start",
            "",
            "remove the byte-order mark",
        )
        .into_diagnostic(),
    )
}

/// A single UTF-8 BOM at stream start is removed as a normalization.
fn bom_candidate(source: &str, drafts: &mut Vec<Draft>) {
    if source.starts_with('\u{FEFF}') {
        drafts.push(Draft::new(
            YamlDiagnosticCode::Bom,
            0..3,
            "UTF-8 byte-order mark at stream start",
            "",
            "remove the byte-order mark",
        ));
    }
}

/// CRLF and lone CR line endings each become an LF normalization candidate.
fn line_ending_candidates(source: &str, drafts: &mut Vec<Draft>) {
    let bytes = source.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'\r' {
            if bytes.get(index + 1) == Some(&b'\n') {
                drafts.push(Draft::new(
                    YamlDiagnosticCode::LineEnding,
                    index..index + 2,
                    "CRLF line ending",
                    "\n",
                    "normalize the line ending to LF",
                ));
                index += 2;
            } else {
                drafts.push(Draft::new(
                    YamlDiagnosticCode::LineEnding,
                    index..index + 1,
                    "lone CR line ending",
                    "\n",
                    "normalize the line ending to LF",
                ));
                index += 1;
            }
        } else {
            index += 1;
        }
    }
}

/// Final-newline handling: insert a missing final newline, or collapse
/// superfluous trailing blank lines. Returns the superfluous-run span when
/// one was generated so trailing-whitespace candidates can stay clear of it.
fn final_newline_candidates(
    source: &str,
    map: &SourceMap,
    drafts: &mut Vec<Draft>,
) -> Option<SourceSpan> {
    if !source.is_empty() && !source.ends_with(['\n', '\r']) {
        let end = source.len();
        drafts.push(Draft::new(
            YamlDiagnosticCode::FinalNewline,
            end..end,
            "missing final newline",
            "\n",
            "add a final newline",
        ));
    }
    let lines = map.lines();
    let mut first_blank = lines.len();
    while first_blank > 0 && lines[first_blank - 1].kind == LineKind::Blank {
        first_blank -= 1;
    }
    if first_blank > 0 && first_blank < lines.len() {
        let span = lines[first_blank].span.start..source.len();
        drafts.push(Draft::new(
            YamlDiagnosticCode::FinalNewline,
            span.clone(),
            "superfluous trailing blank lines",
            "",
            "remove the extra trailing blank lines",
        ));
        return Some(span);
    }
    None
}

/// Trailing whitespace on lines outside scalar content. Block-scalar
/// content lines keep their whitespace (it is literal content); whitespace
/// inside quoted scalars and inside the superfluous-final-newline run is
/// left alone.
fn trailing_whitespace_candidates(
    source: &str,
    map: &SourceMap,
    superfluous: Option<&SourceSpan>,
    drafts: &mut Vec<Draft>,
) {
    for line in map.lines() {
        if line.kind == LineKind::BlockContent || line.content.is_empty() {
            continue;
        }
        let end = trim_end(source, &line.content);
        if end == line.content.end {
            continue;
        }
        let span = end..line.content.end;
        if superfluous.is_some_and(|blocked| {
            span.start < blocked.end && blocked.start < span.end
        }) {
            continue;
        }
        if map.quoted_intersects(&span) {
            continue;
        }
        drafts.push(Draft::new(
            YamlDiagnosticCode::TrailingWhitespace,
            span,
            "trailing whitespace",
            "",
            "remove the trailing whitespace",
        ));
    }
}

/// Whitespace around block mapping colons and sequence markers:
/// `key :  value` → `key: value`, `-  item` → `- item`.
fn block_whitespace_candidates(source: &str, map: &SourceMap, drafts: &mut Vec<Draft>) {
    for (index, line) in map.lines().iter().enumerate() {
        if line.kind != LineKind::Content {
            continue;
        }
        let entry = map.entry(index);
        if let (Some(dash), Some(value)) = (entry.dash, &entry.dash_value) {
            let span = dash + 1..value.start;
            if &source[span.clone()] != " " {
                drafts.push(Draft::new(
                    YamlDiagnosticCode::Whitespace,
                    span,
                    "whitespace after sequence marker",
                    " ",
                    "use one space after the sequence marker",
                ));
            }
        }
        if let Some(mapping) = &entry.mapping {
            if mapping.key.end < mapping.colon {
                drafts.push(Draft::new(
                    YamlDiagnosticCode::Whitespace,
                    mapping.key.end..mapping.colon,
                    "whitespace before mapping colon",
                    "",
                    "remove the space before the mapping colon",
                ));
            }
            if let Some(value) = &mapping.value {
                let span = mapping.colon + 1..value.start;
                if &source[span.clone()] != " " {
                    drafts.push(Draft::new(
                        YamlDiagnosticCode::Whitespace,
                        span,
                        "whitespace after mapping colon",
                        " ",
                        "use one space after the mapping colon",
                    ));
                }
            }
        }
    }
}

/// Whitespace inside flow collections: none after `[`/`{`, none before
/// `]`/`}`/`,`, exactly one space after `,` and after a structural `:`.
/// Multi-line runs (containing line breaks) and runs touching comments are
/// left to the line-ending and trailing-whitespace candidates.
fn flow_whitespace_candidates(source: &str, map: &SourceMap, drafts: &mut Vec<Draft>) {
    let bytes = source.as_bytes();
    for region in map.flow_regions() {
        if !region.closed {
            continue;
        }
        // Skip spans nested inside this region: child flow regions, quoted
        // scalars, and comments. Each whitespace run is classified by its
        // own nearest neighbors, exactly once.
        let mut skips: Vec<SourceSpan> = map
            .flow_regions()
            .iter()
            .filter(|other| {
                other.span.start > region.span.start && other.span.end <= region.span.end
            })
            .map(|other| other.span.clone())
            .collect();
        skips.extend(
            map.quoted_scalars()
                .iter()
                .map(|quoted| quoted.span.clone())
                .filter(|span| span.start >= region.span.start && span.end <= region.span.end),
        );
        skips.extend(
            map.comments()
                .iter()
                .filter(|span| span.start >= region.span.start && span.end <= region.span.end)
                .cloned(),
        );
        skips.sort_by_key(|span| (span.start, span.end));

        let inner_start = region.span.start + 1;
        let inner_end = region.span.end - 1;
        let mut skip_index = 0;
        let mut index = inner_start;
        while index < inner_end {
            while skip_index < skips.len() && index >= skips[skip_index].end {
                skip_index += 1;
            }
            if skip_index < skips.len() && index >= skips[skip_index].start {
                index = skips[skip_index].end;
                continue;
            }
            match bytes[index] {
                b' ' | b'\t' => {
                    let run_start = index;
                    while index < inner_end && matches!(bytes[index], b' ' | b'\t') {
                        index += 1;
                    }
                    let run = run_start..index;
                    let previous = if run_start == inner_start {
                        bytes[region.span.start]
                    } else {
                        bytes[run_start - 1]
                    };
                    let next = bytes.get(index).copied();
                    let Some(next_byte) = next else { break };
                    if matches!(next_byte, b'\n' | b'\r') {
                        continue;
                    }
                    let replacement = if matches!(previous, b'[' | b'{')
                        || matches!(next_byte, b']' | b'}' | b',')
                        || (next_byte == b':' && structural_colon(bytes, index))
                    {
                        ""
                    } else if previous == b',' || (previous == b':' && structural_colon(bytes, run_start - 1)) {
                        " "
                    } else {
                        continue;
                    };
                    if &source[run.clone()] != replacement {
                        drafts.push(Draft::new(
                            YamlDiagnosticCode::Whitespace,
                            run,
                            "whitespace in flow collection",
                            replacement,
                            "use canonical spacing in flow collections",
                        ));
                    }
                }
                b',' => {
                    let next = bytes.get(index + 1).copied();
                    if next.is_some_and(|byte| {
                        !matches!(byte, b' ' | b'\t' | b'\n' | b'\r' | b',' | b']' | b'}')
                    }) {
                        drafts.push(Draft::new(
                            YamlDiagnosticCode::Whitespace,
                            index + 1..index + 1,
                            "missing space after comma",
                            " ",
                            "use one space after a comma",
                        ));
                    }
                    index += 1;
                }
                b':' => {
                    let next = bytes.get(index + 1).copied();
                    let quoted_key = index > 0 && matches!(bytes[index - 1], b'"' | b'\'');
                    if quoted_key
                        && next.is_some_and(|byte| {
                            !matches!(byte, b' ' | b'\t' | b'\n' | b'\r' | b',' | b']' | b'}')
                        })
                    {
                        drafts.push(Draft::new(
                            YamlDiagnosticCode::Whitespace,
                            index + 1..index + 1,
                            "missing space after mapping colon",
                            " ",
                            "use one space after the mapping colon",
                        ));
                    }
                    index += 1;
                }
                _ => index += 1,
            }
        }
    }
}

/// A flow-context colon is structural when followed by whitespace, a
/// delimiter, or end of source, or when it immediately follows a closing
/// quote (JSON-style `{"key":1}`).
fn structural_colon(bytes: &[u8], colon: usize) -> bool {
    match bytes.get(colon + 1) {
        None => true,
        Some(b' ' | b'\t' | b'\n' | b'\r' | b',' | b']' | b'}') => true,
        _ => colon > 0 && matches!(bytes[colon - 1], b'"' | b'\''),
    }
}

fn trim_end(source: &str, content: &SourceSpan) -> usize {
    let bytes = source.as_bytes();
    let mut end = content.end;
    while end > content.start && matches!(bytes[end - 1], b' ' | b'\t') {
        end -= 1;
    }
    end
}
