//! Signals compilation stage: detection records in the signals research
//! corpus → the committed `lib/src/signals/generated.rs` tables.
//!
//! Follows the `catalog.json` single-artifact precedent (always regenerated
//! full-scope with its own check fn), not the per-provider confirm flow:
//! one committed file spans every signals doc, including the roster-only
//! providers (kilo, pi) whose tables compile but stay dormant — reachable
//! only through the lib's slug lookup, never a `Provider` variant.
//!
//! Malformed input is a typed generation error naming the doc (and record),
//! never a silent skip. Evidence-file EXISTENCE is deliberately not checked
//! here — the sidecar defers that to the fixture-replay `signals check`.
//!
//! Determinism contract: providers emit in [`SIGNAL_SLUGS`] (alphabetical)
//! order; records within a provider sort by (source declaration order,
//! priority); extractions keep document order within their record; the file
//! ends in exactly one trailing newline and nothing is timestamped.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use claudine_catalog_types::{DetectionMode, MatchOp, SignalKind, SignalSource, Unit, Zone};
use serde_json::Value;
use strum::IntoEnumIterator;

mod emit;

use crate::emit::{indent, pascal, render_slice};
use emit::emit_file;
use crate::errors::GenError;
use crate::generate::{CheckOutcome, diff_lines};
use crate::inputs::{area_resolution_context, load_validated_frontmatter};
use biscuit_file::FileResolutionContext;
use darkmatter::markdown::compose::RequestSnapshot;

/// Every signals research doc, alphabetical (= emission order). A superset
/// of [`crate::generate::PROVIDER_SLUGS`]: kilo and pi are roster-only.
pub const SIGNAL_SLUGS: &[&str] = &[
    "antigravity", "claude", "codex", "gemini", "goose", "kilo", "kimi", "opencode", "pi", "qwen",
];

/// The committed generated signals path under an area root.
pub fn signals_path(area: &Path) -> PathBuf {
    area.join("lib/src/signals/generated.rs")
}

/// Builds the deterministic `generated.rs` text from the full signals
/// corpus under `area`, resolving every document's `$schema` through one
/// context built from `snapshot` rebased at the area.
pub fn build_signals(area: &Path, snapshot: &RequestSnapshot) -> Result<String, GenError> {
    let context = area_resolution_context(area, snapshot)?;
    let mut tables = Vec::with_capacity(SIGNAL_SLUGS.len());
    for slug in SIGNAL_SLUGS {
        tables.push(load_doc(area, slug, &context)?);
    }
    Ok(emit_file(&tables))
}

/// Byte-compares [`build_signals`] output against the committed file — the
/// code path shared by the CLI `check` subcommand and the drift test.
pub fn check_signals(area: &Path, snapshot: &RequestSnapshot) -> Result<CheckOutcome, GenError> {
    let generated = build_signals(area, snapshot)?;
    let path = signals_path(area);
    if !path.is_file() {
        return Ok(CheckOutcome::MissingCommitted { path });
    }
    let committed = std::fs::read_to_string(&path).map_err(|source| GenError::Io {
        path: path.clone(),
        source,
    })?;
    if committed == generated {
        return Ok(CheckOutcome::Clean);
    }
    Ok(CheckOutcome::Drift {
        details: diff_lines(&committed, &generated),
    })
}

// ---------------------------------------------------------------------------
// Owned parse rows
// ---------------------------------------------------------------------------

/// One provider's parsed, validated table (records already in emission
/// order).
struct DocTable {
    slug: &'static str,
    records: Vec<RecordRow>,
}

struct RecordRow {
    id: String,
    kind: SignalKind,
    source: SignalSource,
    mode: DetectionMode,
    priority: u16,
    match_path: Option<String>,
    op: Option<MatchOp>,
    value: Option<String>,
    values: Vec<String>,
    since: Option<String>,
    until: Option<String>,
    extractions: Vec<ExtractionRow>,
}

struct ExtractionRow {
    field: String,
    source: ExtractSource,
    unit: Option<Unit>,
    zone: Option<Zone>,
}

/// Gen-side mirror of `claudine_catalog_types::ExtractStrategy`. Research
/// authoring: `path:` (the common form), or `literal:`, or `regex:`+`path:`,
/// or `start:`+`stop:`+`path:`.
enum ExtractSource {
    Path(String),
    Literal(String),
    Regex { path: String, pattern: String },
    StartStopTokens { path: String, start: String, stop: String },
}

fn doc_err(path: &Path, message: impl Into<String>) -> GenError {
    GenError::SignalDocInvalid {
        path: path.to_path_buf(),
        message: message.into(),
    }
}

fn record_err(path: &Path, record: &str, message: impl Into<String>) -> GenError {
    GenError::SignalRecordInvalid {
        path: path.to_path_buf(),
        record: record.to_string(),
        message: message.into(),
    }
}

// ---------------------------------------------------------------------------
// Loading + validation
// ---------------------------------------------------------------------------

/// Loads one sidecar-validated signals doc and runs every generate-time
/// gate: path grammar, regex compilation, op/value consistency, priority
/// uniqueness, the exact-duplicate subsumption gate, and join integrity.
fn load_doc(
    area: &Path,
    slug: &'static str,
    context: &FileResolutionContext,
) -> Result<DocTable, GenError> {
    let doc_path = area.join(format!("docs/research/signals/{slug}.md"));
    let sidecar = area.join("docs/research/signals/_schema.yaml");
    if !sidecar.is_file() {
        return Err(GenError::SidecarMissing { path: sidecar });
    }
    let frontmatter = load_validated_frontmatter(&doc_path, context)?;

    let record_values = frontmatter
        .get("records")
        .and_then(Value::as_array)
        .ok_or_else(|| doc_err(&doc_path, "frontmatter has no `records:` list"))?;
    let mut records = Vec::with_capacity(record_values.len());
    for value in record_values {
        records.push(parse_record(&doc_path, value)?);
    }

    // Unique ids are a precondition of the extraction join.
    let mut index_of: BTreeMap<String, usize> = BTreeMap::new();
    for (index, record) in records.iter().enumerate() {
        if index_of.insert(record.id.clone(), index).is_some() {
            return Err(record_err(&doc_path, &record.id, "duplicate record id"));
        }
    }
    check_groups(&doc_path, &records)?;

    // Extractions join flat onto records[].id, keeping document order.
    if let Some(extractions) = frontmatter.get("extractions") {
        let extractions = extractions
            .as_array()
            .ok_or_else(|| doc_err(&doc_path, "`extractions:` is not a list"))?;
        for value in extractions {
            let target = require_str(&doc_path, "<extraction>", value, "record")?;
            let row = parse_extraction(&doc_path, &target, value)?;
            match index_of.get(&target) {
                Some(&index) => records[index].extractions.push(row),
                None => {
                    return Err(doc_err(
                        &doc_path,
                        format!(
                            "extraction for field `{}` references record `{target}`, which \
                             does not exist in records[]",
                            row.field
                        ),
                    ));
                }
            }
        }
    }

    // Emission order: (source declaration order, priority). The sort is
    // total because check_groups proved priorities unique per group.
    records.sort_by_key(|record| (source_rank(record.source), record.priority));
    Ok(DocTable { slug, records })
}

fn parse_record(doc: &Path, value: &Value) -> Result<RecordRow, GenError> {
    let id = require_str(doc, "<record>", value, "id")?;
    let kind = parse_member::<SignalKind>(
        doc,
        &id,
        "SignalKind",
        &require_str(doc, &id, value, "signal")?,
    )?;
    let source = parse_member::<SignalSource>(
        doc,
        &id,
        "SignalSource",
        &require_str(doc, &id, value, "source")?,
    )?;
    let mode = parse_member::<DetectionMode>(
        doc,
        &id,
        "DetectionMode",
        &require_str(doc, &id, value, "detection")?,
    )?;

    let priority_value = value
        .get("priority")
        .ok_or_else(|| record_err(doc, &id, "missing `priority`"))?;
    let priority = priority_value
        .as_u64()
        .and_then(|n| u16::try_from(n).ok())
        .ok_or_else(|| {
            record_err(
                doc,
                &id,
                format!("priority `{priority_value}` is not a u16 (0..=65535)"),
            )
        })?;

    let match_path = optional_str(doc, &id, value, "match_path")?;
    if let Some(expr) = &match_path {
        parse_signal_path(expr)
            .map_err(|reason| record_err(doc, &id, format!("match_path `{expr}`: {reason}")))?;
    }
    let op = match optional_str(doc, &id, value, "match_op")? {
        Some(member) => Some(parse_member::<MatchOp>(doc, &id, "MatchOp", &member)?),
        None => None,
    };
    let match_value = optional_str(doc, &id, value, "match_value")?;
    let match_values = optional_str_list(doc, &id, value, "match_values")?;

    check_match_terms(doc, &id, mode, &match_path, op, &match_value, &match_values)?;

    Ok(RecordRow {
        kind,
        source,
        mode,
        priority,
        match_path,
        op,
        value: match_value,
        values: match_values,
        since: optional_str(doc, &id, value, "since")?,
        until: optional_str(doc, &id, value, "until")?,
        extractions: Vec::new(),
        id,
    })
}

/// The op/value consistency gate (design "Record grammar"):
/// - declarative records require `match_path` + `match_op`;
/// - a present `match_op` requires `match_path` even on bespoke records;
/// - `eq`/`regex`/`substring_ci` require `match_value` and forbid
///   `match_values`; `in` is the reverse; `exists` carries neither;
/// - `regex` values must compile at generate time.
fn check_match_terms(
    doc: &Path,
    id: &str,
    mode: DetectionMode,
    match_path: &Option<String>,
    op: Option<MatchOp>,
    value: &Option<String>,
    values: &[String],
) -> Result<(), GenError> {
    if mode == DetectionMode::Declarative && (match_path.is_none() || op.is_none()) {
        return Err(record_err(
            doc,
            id,
            "declarative records require both `match_path` and `match_op`",
        ));
    }
    let Some(op) = op else {
        // A bespoke record with no `match_op` is a cross-record/temporal
        // detector — its firing lives in the behavior half, so there are no
        // match terms to gate here.
        return Ok(());
    };
    if match_path.is_none() {
        return Err(record_err(doc, id, "`match_op` requires `match_path`"));
    }
    match op {
        MatchOp::Eq | MatchOp::Regex | MatchOp::SubstringCi => {
            if value.is_none() {
                return Err(record_err(
                    doc,
                    id,
                    format!("`match_op: {}` requires `match_value`", op_wire(op)),
                ));
            }
            if !values.is_empty() {
                return Err(record_err(
                    doc,
                    id,
                    format!(
                        "`match_op: {}` takes a single `match_value`, not `match_values`",
                        op_wire(op)
                    ),
                ));
            }
            if op == MatchOp::Regex {
                let pattern = value.as_deref().expect("checked above");
                regex::Regex::new(pattern).map_err(|err| {
                    record_err(doc, id, format!("regex `{pattern}` does not compile: {err}"))
                })?;
            }
        }
        MatchOp::In => {
            if values.is_empty() {
                return Err(record_err(
                    doc,
                    id,
                    "`match_op: in` requires a non-empty `match_values` list",
                ));
            }
            if value.is_some() {
                return Err(record_err(
                    doc,
                    id,
                    "`match_op: in` takes `match_values`, not `match_value`",
                ));
            }
        }
        MatchOp::Exists => {
            if value.is_some() || !values.is_empty() {
                return Err(record_err(
                    doc,
                    id,
                    "`match_op: exists` carries no `match_value`/`match_values`",
                ));
            }
        }
    }
    Ok(())
}

/// Per provider×source group: priorities must be unique, and no two
/// records carrying match terms may share an identical
/// (match_path, op, value, values, since, until) tuple — the
/// exact-duplicate half of the subsumption gate (finer overlap analysis is
/// `signals check`'s fixtures-based job). The version window is part of a
/// record's identity: version-drift twins with equal match terms but
/// disjoint `since`/`until` ranges are legitimate (e.g. the OpenCode
/// 1.17.8 stream-error format drift) and admitted at runtime by
/// version-range selection, not by match terms alone.
fn check_groups(doc: &Path, records: &[RecordRow]) -> Result<(), GenError> {
    let mut priorities: BTreeMap<(usize, u16), &str> = BTreeMap::new();
    let mut terms: BTreeMap<String, &str> = BTreeMap::new();
    for record in records {
        let rank = source_rank(record.source);
        if let Some(other) = priorities.insert((rank, record.priority), &record.id) {
            return Err(record_err(
                doc,
                &record.id,
                format!(
                    "priority {} duplicates record `{other}` in the same `{}` source group",
                    record.priority,
                    source_wire(record.source),
                ),
            ));
        }
        if record.op.is_some() {
            let key = format!(
                "{rank}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}",
                record.match_path,
                record.op.map(op_wire),
                record.value,
                record.values,
                record.since,
                record.until,
            );
            if let Some(other) = terms.insert(key, &record.id) {
                return Err(record_err(
                    doc,
                    &record.id,
                    format!(
                        "match terms exactly duplicate record `{other}` in the same `{}` \
                         source group",
                        source_wire(record.source),
                    ),
                ));
            }
        }
    }
    Ok(())
}

fn parse_extraction(doc: &Path, record: &str, value: &Value) -> Result<ExtractionRow, GenError> {
    let field = require_str(doc, record, value, "field")?;
    let source = parse_extract_source(doc, record, &field, value)?;
    let unit = match optional_str(doc, record, value, "unit")? {
        Some(member) => Some(parse_member::<Unit>(doc, record, "Unit", &member)?),
        None => None,
    };
    let zone = match optional_str(doc, record, value, "zone")? {
        Some(member) => Some(parse_member::<Zone>(doc, record, "Zone", &member)?),
        None => None,
    };
    Ok(ExtractionRow {
        field,
        source,
        unit,
        zone,
    })
}

/// Select the extraction strategy from the authored keys, validating any path.
fn parse_extract_source(
    doc: &Path,
    record: &str,
    field: &str,
    value: &Value,
) -> Result<ExtractSource, GenError> {
    let validate_path = |path: &str| -> Result<(), GenError> {
        parse_signal_path(path)
            .map(|_| ())
            .map_err(|reason| {
                record_err(doc, record, format!("extraction `{field}` path `{path}`: {reason}"))
            })
    };
    if let Some(literal) = optional_str(doc, record, value, "literal")? {
        return Ok(ExtractSource::Literal(literal));
    }
    if let Some(pattern) = optional_str(doc, record, value, "regex")? {
        let path = require_str(doc, record, value, "path")?;
        validate_path(&path)?;
        return Ok(ExtractSource::Regex { path, pattern });
    }
    let start = optional_str(doc, record, value, "start")?;
    let stop = optional_str(doc, record, value, "stop")?;
    if start.is_some() || stop.is_some() {
        let path = require_str(doc, record, value, "path")?;
        validate_path(&path)?;
        let (start, stop) = (
            start.ok_or_else(|| record_err(doc, record, format!("extraction `{field}` has `stop` but no `start`")))?,
            stop.ok_or_else(|| record_err(doc, record, format!("extraction `{field}` has `start` but no `stop`")))?,
        );
        return Ok(ExtractSource::StartStopTokens { path, start, stop });
    }
    let path = require_str(doc, record, value, "path")?;
    validate_path(&path)?;
    Ok(ExtractSource::Path(path))
}

// ---------------------------------------------------------------------------
// Small parse helpers
// ---------------------------------------------------------------------------

fn require_str(doc: &Path, record: &str, value: &Value, key: &str) -> Result<String, GenError> {
    match value.get(key) {
        Some(Value::String(text)) => Ok(text.clone()),
        Some(other) => Err(record_err(
            doc,
            record,
            format!("`{key}` must be a string, got `{other}`"),
        )),
        None => Err(record_err(doc, record, format!("missing `{key}`"))),
    }
}

fn optional_str(
    doc: &Path,
    record: &str,
    value: &Value,
    key: &str,
) -> Result<Option<String>, GenError> {
    match value.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(text)) => Ok(Some(text.clone())),
        Some(other) => Err(record_err(
            doc,
            record,
            format!("`{key}` must be a string, got `{other}`"),
        )),
    }
}

fn optional_str_list(
    doc: &Path,
    record: &str,
    value: &Value,
    key: &str,
) -> Result<Vec<String>, GenError> {
    let Some(list) = value.get(key) else {
        return Ok(Vec::new());
    };
    let items = list.as_array().ok_or_else(|| {
        record_err(doc, record, format!("`{key}` must be a string list, got `{list}`"))
    })?;
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        match item {
            Value::String(text) => out.push(text.clone()),
            other => {
                return Err(record_err(
                    doc,
                    record,
                    format!("`{key}` must contain strings, got `{other}`"),
                ));
            }
        }
    }
    Ok(out)
}

/// snake_case wire member → catalog-types variant, via strum's static-str
/// mirror. Sidecar enum validation normally rejects unknown members first;
/// this is the loud second line of defense against sidecar↔Rust drift.
fn parse_member<T>(doc: &Path, record: &str, what: &str, member: &str) -> Result<T, GenError>
where
    T: IntoEnumIterator + Copy,
    &'static str: From<T>,
{
    T::iter()
        .find(|variant| <&'static str>::from(*variant) == member)
        .ok_or_else(|| record_err(doc, record, format!("`{member}` is not a {what} member")))
}

/// One parsed segment of the restricted JSONPath subset.
#[derive(Debug, PartialEq, Eq)]
enum PathSegment {
    Key(String),
    Index(u64),
}

/// Parses `a.b[0].c` into segments: dot-separated identifiers, each
/// optionally followed by numeric bracket indices. Everything else —
/// wildcards, filters, recursive descent, quoted keys — is outside the
/// subset and rejected.
fn parse_signal_path(expr: &str) -> Result<Vec<PathSegment>, String> {
    if expr.is_empty() {
        return Err("path is empty".to_string());
    }
    let mut segments = Vec::new();
    for part in expr.split('.') {
        let (ident, brackets) = match part.find('[') {
            Some(at) => part.split_at(at),
            None => (part, ""),
        };
        let mut chars = ident.chars();
        let head_ok = matches!(chars.next(), Some(c) if c.is_ascii_alphabetic() || c == '_');
        if !head_ok || !chars.all(|c| c.is_ascii_alphanumeric() || c == '_') {
            return Err(format!("segment `{part}` is not a bare identifier"));
        }
        segments.push(PathSegment::Key(ident.to_string()));
        let mut rest = brackets;
        while !rest.is_empty() {
            let Some(after_open) = rest.strip_prefix('[') else {
                return Err(format!("segment `{part}` has trailing text after a bracket"));
            };
            let Some(close) = after_open.find(']') else {
                return Err(format!("segment `{part}` has an unterminated bracket"));
            };
            let digits = &after_open[..close];
            if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
                return Err(format!(
                    "bracket index `[{digits}]` is not numeric — wildcards, filters, and \
                     recursive descent are outside the restricted subset"
                ));
            }
            let index: u64 = digits
                .parse()
                .map_err(|_| format!("bracket index `[{digits}]` overflows u64"))?;
            segments.push(PathSegment::Index(index));
            rest = &after_open[close + 1..];
        }
    }
    Ok(segments)
}

fn source_rank(source: SignalSource) -> usize {
    SignalSource::iter()
        .position(|candidate| candidate == source)
        .expect("iter covers every variant")
}

fn op_wire(op: MatchOp) -> &'static str {
    op.into()
}

fn source_wire(source: SignalSource) -> &'static str {
    source.into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signal_path_accepts_the_restricted_subset() {
        assert_eq!(
            parse_signal_path("message.content[0].text").unwrap(),
            vec![
                PathSegment::Key("message".into()),
                PathSegment::Key("content".into()),
                PathSegment::Index(0),
                PathSegment::Key("text".into()),
            ]
        );
        assert!(parse_signal_path("apiKeySource").is_ok());
        assert!(parse_signal_path("params.payload.token_usage.output").is_ok());
        assert!(parse_signal_path("choices[0].finish_reason").is_ok());
        assert!(parse_signal_path("a[1][2]").is_ok());
    }

    #[test]
    fn signal_path_rejects_everything_outside_the_subset() {
        for bad in [
            "",
            ".leading",
            "trailing.",
            "a..b",
            "a[*]",
            "a[-1]",
            "$.a",
            "a[?(@.b)]",
            "a..b",
            "a[0]x",
            "a[0",
            "a['key']",
            "9lives",
            "a b",
        ] {
            assert!(parse_signal_path(bad).is_err(), "`{bad}` must be rejected");
        }
    }

    #[test]
    fn pascal_round_trips_signal_members() {
        // Spot-check the wire→variant naming the emitter relies on.
        assert_eq!(pascal(SignalKind::UsageCapApproaching.into()), "UsageCapApproaching");
        assert_eq!(pascal(SignalSource::StderrPromoted.into()), "StderrPromoted");
        assert_eq!(pascal(MatchOp::SubstringCi.into()), "SubstringCi");
        assert_eq!(pascal(Unit::Iso8601.into()), "Iso8601");
        assert_eq!(pascal(Zone::EmbeddedOffset.into()), "EmbeddedOffset");
    }
}
