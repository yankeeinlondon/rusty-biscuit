//! Report-only YAML diagnostics.
//!
//! Every detector in this module is lexical: findings come from the
//! [`SourceMap`] and never reparse the source, which keeps the analyzer's
//! parse-once instrumentation intact. Two certainty classifications are
//! produced, and both are report-only — no diagnostic here ever carries a
//! repair, so nothing in this module can reach edit application:
//!
//! - `DeterministicFindNonDeterministicSolution`: the problem is certain
//!   (duplicate keys, undeclared/forward/misspelled/duplicate anchors,
//!   multiple documents) but every repair choice needs an intent decision.
//! - `NonDeterministicFind`: a suspected smell (unused anchors, ambiguous
//!   scalars, suspicious empty values, block-scalar smells, comment
//!   truncation, style inconsistency, similar keys). Heuristic thresholds
//!   are biased toward quiet: common intentional YAML produces nothing.

use std::collections::HashMap;

use super::diagnostic::{YamlCertainty, YamlDiagnostic, YamlDiagnosticCode};
use super::scan::{
    AnchorKind, BlockScalar, FlowKind, KeyOccurrence, LineKind, MarkerKind, PathSegment, QuoteStyle,
    SourceMap,
};
use crate::span::SourceSpan;

/// Runs every report-only detector over `source`, returning all findings.
/// Findings are returned detector-grouped; the engine sorts the combined
/// set into stable source order.
pub(super) fn report(source: &str, map: &SourceMap) -> Vec<YamlDiagnostic> {
    let mut diagnostics = Vec::new();
    duplicate_keys(source, map, &mut diagnostics);
    anchor_alias_conditions(source, map, &mut diagnostics);
    multiple_documents(map, &mut diagnostics);
    ambiguous_scalars(source, map, &mut diagnostics);
    suspicious_empty_values(source, map, &mut diagnostics);
    block_scalar_smells(source, map, &mut diagnostics);
    comment_truncation(source, map, &mut diagnostics);
    style_inconsistency(source, map, &mut diagnostics);
    similar_keys(source, map, &mut diagnostics);
    diagnostics
}

fn finding(
    code: YamlDiagnosticCode,
    span: SourceSpan,
    classification: YamlCertainty,
    message: String,
) -> YamlDiagnostic {
    YamlDiagnostic {
        code,
        span,
        classification,
        message,
        repairs: Vec::new(),
    }
}

/// 1-indexed line number of `byte`, for human-readable cross-references.
fn line_number(map: &SourceMap, byte: usize) -> usize {
    map.line_at_byte(byte).map_or(0, |line| line + 1)
}

/// Document discriminator for scope-sensitive detectors: the number of
/// document markers before `byte`. Duplicate keys and anchor scopes never
/// cross a document boundary, and every boundary carries a marker, so equal
/// indices imply the same document.
fn document_index(map: &SourceMap, byte: usize) -> usize {
    map.markers()
        .iter()
        .filter(|marker| marker.span.start < byte)
        .count()
}

// ===== Duplicate mapping keys =====

/// Detects duplicate mapping keys at every block nesting level plus inside
/// flow mappings. `serde_yaml_ng` rejects the document, so the parser alone
/// cannot report both conflicting entries; this detector reports every
/// occurrence with its own span and never selects a repair.
fn duplicate_keys(source: &str, map: &SourceMap, diagnostics: &mut Vec<YamlDiagnostic>) {
    let occurrences = map.key_occurrences(source);
    let mut groups: HashMap<(usize, Vec<PathSegment>), Vec<&KeyOccurrence>> = HashMap::new();
    for occurrence in &occurrences {
        let document = document_index(map, occurrence.key_span.start);
        groups
            .entry((document, occurrence.path.clone()))
            .or_default()
            .push(occurrence);
    }
    let mut conflicts: Vec<(SourceSpan, String)> = Vec::new();
    for occurrences in groups.values() {
        if occurrences.len() > 1 {
            report_key_conflict(map, occurrences, &mut conflicts);
        }
    }
    for region in map.flow_regions() {
        if region.kind != FlowKind::Mapping || !region.closed {
            continue;
        }
        let keys = flow_mapping_keys(source, map, region.span.clone());
        let mut seen: HashMap<String, SourceSpan> = HashMap::new();
        let mut flow_conflicts: Vec<(String, SourceSpan, SourceSpan)> = Vec::new();
        for (text, span) in keys {
            if let Some(first) = seen.get(&text) {
                flow_conflicts.push((text, first.clone(), span));
            } else {
                seen.insert(text, span);
            }
        }
        for (text, first, later) in flow_conflicts {
            conflicts.push((
                first.clone(),
                format!(
                    "mapping key `{text}` is redefined at line {}",
                    line_number(map, later.start)
                ),
            ));
            conflicts.push((
                later,
                format!(
                    "duplicate mapping key `{text}` (first defined at line {})",
                    line_number(map, first.start)
                ),
            ));
        }
    }
    conflicts.sort_by_key(|(span, _)| (span.start, span.end));
    for (span, message) in conflicts {
        diagnostics.push(finding(
            YamlDiagnosticCode::DuplicateKey,
            span,
            YamlCertainty::DeterministicFindNonDeterministicSolution,
            message,
        ));
    }
}

/// Emits both spans of a block-level key conflict: the first definition
/// (marked as redefined later) and every duplicate (marked with the first
/// definition's line).
fn report_key_conflict(
    map: &SourceMap,
    occurrences: &[&KeyOccurrence],
    conflicts: &mut Vec<(SourceSpan, String)>,
) {
    let first = occurrences[0];
    let first_line = line_number(map, first.key_span.start);
    let second_line = line_number(map, occurrences[1].key_span.start);
    let text = &first.key_text;
    conflicts.push((
        first.key_span.clone(),
        format!("mapping key `{text}` is redefined at line {second_line}"),
    ));
    for occurrence in &occurrences[1..] {
        conflicts.push((
            occurrence.key_span.clone(),
            format!("duplicate mapping key `{text}` (first defined at line {first_line})"),
        ));
    }
}

/// Tokenizes the top-level entries of a closed flow mapping region into
/// `(key text, key span)` pairs. Nested flow regions, quoted scalars, and
/// comments inside the region are skipped; entries whose key is empty,
/// compound, or not representable are skipped (the parser diagnostic covers
/// them).
fn flow_mapping_keys(source: &str, map: &SourceMap, region: SourceSpan) -> Vec<(String, SourceSpan)> {
    let bytes = source.as_bytes();
    let mut skips: Vec<SourceSpan> = map
        .flow_regions()
        .iter()
        .filter(|other| other.span.start > region.start && other.span.end <= region.end)
        .map(|other| other.span.clone())
        .collect();
    skips.extend(
        map.quoted_scalars()
            .iter()
            .map(|quoted| quoted.span.clone())
            .filter(|span| span.start >= region.start && span.end <= region.end),
    );
    skips.extend(
        map.comments()
            .iter()
            .filter(|span| span.start >= region.start && span.end <= region.end)
            .cloned(),
    );
    skips.sort_by_key(|span| (span.start, span.end));

    let mut keys = Vec::new();
    let inner_start = region.start + 1;
    let inner_end = region.end - 1;
    let mut entry_start = inner_start;
    let mut index = inner_start;
    let mut skip_index = 0;
    while index <= inner_end {
        while skip_index < skips.len() && index >= skips[skip_index].end {
            skip_index += 1;
        }
        if skip_index < skips.len() && index >= skips[skip_index].start {
            index = skips[skip_index].end;
            continue;
        }
        if index == inner_end || bytes[index] == b',' {
            if let Some(key) = flow_entry_key(source, map, entry_start, index) {
                keys.push(key);
            }
            entry_start = index + 1;
        }
        index += 1;
    }
    keys
}

/// Extracts the key of one top-level flow entry (`start..end`): the trimmed
/// text before the first structural colon, when it is a plain or simply
/// quoted scalar.
fn flow_entry_key(
    source: &str,
    map: &SourceMap,
    start: usize,
    end: usize,
) -> Option<(String, SourceSpan)> {
    let bytes = source.as_bytes();
    let mut key_start = start;
    while key_start < end && matches!(bytes[key_start], b' ' | b'\t' | b'\n' | b'\r') {
        key_start += 1;
    }
    let mut colon = None;
    for index in key_start..end {
        if map.quoted_scalars().iter().any(|quoted| {
            quoted.span.start < index && index < quoted.span.end && quoted.span.start >= start
        }) {
            continue;
        }
        if bytes[index] == b':' && flow_structural_colon(bytes, index) {
            colon = Some(index);
            break;
        }
    }
    let colon = colon?;
    let mut key_end = colon;
    while key_end > key_start && matches!(bytes[key_end - 1], b' ' | b'\t' | b'\n' | b'\r') {
        key_end -= 1;
    }
    if key_end <= key_start {
        return None;
    }
    let text = &source[key_start..key_end];
    let quoted = text.len() >= 2
        && ((text.starts_with('\'') && text.ends_with('\''))
            || (text.starts_with('"') && text.ends_with('"')));
    let unquoted = if quoted { &text[1..text.len() - 1] } else { text };
    if unquoted.is_empty() || unquoted.contains(['[', ']', '{', '}', ',']) {
        return None;
    }
    Some((unquoted.to_string(), key_start..key_end))
}

/// A flow-context colon is structural when followed by whitespace, a
/// delimiter, or end of source, or when it immediately follows a closing
/// quote (JSON-style `{"key":1}`). Mirrors the S1 whitespace classifier.
fn flow_structural_colon(bytes: &[u8], colon: usize) -> bool {
    match bytes.get(colon + 1) {
        None => true,
        Some(b' ' | b'\t' | b'\n' | b'\r' | b',' | b']' | b'}') => true,
        _ => colon > 0 && matches!(bytes[colon - 1], b'"' | b'\''),
    }
}

// ===== Anchor and alias conditions =====

/// Detects undeclared, forward, misspelled, duplicate, and unused
/// anchor/alias conditions. Anchor scopes never cross document boundaries.
/// Every finding preserves graph-sensitive source: none carries a repair.
fn anchor_alias_conditions(source: &str, map: &SourceMap, diagnostics: &mut Vec<YamlDiagnostic>) {
    let anchors: Vec<_> = map
        .anchors()
        .iter()
        .filter(|reference| reference.kind == AnchorKind::Anchor)
        .collect();
    let aliases: Vec<_> = map
        .anchors()
        .iter()
        .filter(|reference| reference.kind == AnchorKind::Alias)
        .collect();
    // Anchors offered as a misspelling candidate are presumed referenced —
    // the misspelling finding supersedes an unused finding for them.
    let mut candidate_anchors: Vec<SourceSpan> = Vec::new();

    for alias in &aliases {
        let document = document_index(map, alias.full.start);
        let name = &source[alias.name.clone()];
        let declared: Vec<_> = anchors
            .iter()
            .filter(|anchor| {
                document_index(map, anchor.full.start) == document
                    && &source[anchor.name.clone()] == name
            })
            .collect();
        if declared.is_empty() {
            let candidate = anchors
                .iter()
                .filter(|anchor| document_index(map, anchor.full.start) == document)
                .filter(|anchor| near_key_match(name, &source[anchor.name.clone()]))
                .min_by_key(|anchor| {
                    (
                        edit_distance(name, &source[anchor.name.clone()]),
                        alias.full.start.abs_diff(anchor.full.start),
                    )
                });
            if let Some(candidate) = candidate {
                let candidate_name = &source[candidate.name.clone()];
                candidate_anchors.push(candidate.name.clone());
                diagnostics.push(finding(
                    YamlDiagnosticCode::AnchorMisspelled,
                    alias.name.clone(),
                    YamlCertainty::DeterministicFindNonDeterministicSolution,
                    format!(
                        "alias `*{name}` matches no declared anchor; did you mean `&{candidate_name}` (declared at line {})?",
                        line_number(map, candidate.full.start)
                    ),
                ));
            } else {
                diagnostics.push(finding(
                    YamlDiagnosticCode::AnchorUndeclared,
                    alias.name.clone(),
                    YamlCertainty::DeterministicFindNonDeterministicSolution,
                    format!("alias `*{name}` references an anchor that is never declared"),
                ));
            }
        } else if declared.iter().all(|anchor| anchor.full.start > alias.full.start) {
            diagnostics.push(finding(
                YamlDiagnosticCode::AnchorForward,
                alias.name.clone(),
                YamlCertainty::DeterministicFindNonDeterministicSolution,
                format!(
                    "alias `*{name}` references anchor `&{name}` declared later at line {}",
                    line_number(map, declared[0].full.start)
                ),
            ));
        }
    }

    let mut by_name: HashMap<(usize, &str), Vec<_>> = HashMap::new();
    for anchor in &anchors {
        let document = document_index(map, anchor.full.start);
        by_name
            .entry((document, &source[anchor.name.clone()]))
            .or_default()
            .push(anchor);
    }
    let mut duplicates: Vec<_> = by_name
        .values()
        .filter(|declarations| declarations.len() > 1)
        .flat_map(|declarations| declarations[1..].to_vec())
        .collect();
    duplicates.sort_by_key(|anchor| anchor.full.start);
    for anchor in duplicates {
        let name = &source[anchor.name.clone()];
        diagnostics.push(finding(
            YamlDiagnosticCode::AnchorDuplicate,
            anchor.name.clone(),
            YamlCertainty::DeterministicFindNonDeterministicSolution,
            format!(
                "anchor `&{name}` is declared more than once; the later declaration shadows the earlier one"
            ),
        ));
    }

    for anchor in &anchors {
        if candidate_anchors.contains(&anchor.name) {
            continue;
        }
        let document = document_index(map, anchor.full.start);
        let name = &source[anchor.name.clone()];
        let used = aliases.iter().any(|alias| {
            document_index(map, alias.full.start) == document
                && &source[alias.name.clone()] == name
        });
        if !used {
            diagnostics.push(finding(
                YamlDiagnosticCode::AnchorUnused,
                anchor.name.clone(),
                YamlCertainty::NonDeterministicFind,
                format!("anchor `&{name}` is never referenced by an alias"),
            ));
        }
    }
}

// ===== Multiple documents =====

/// Detects streams containing more than one YAML document and reports each
/// extra document's opening span. The incompatibility is certain for
/// single-document analysis, but selecting, splitting, or rewriting a
/// document is an intent decision — no repair is ever attached.
fn multiple_documents(map: &SourceMap, diagnostics: &mut Vec<YamlDiagnostic>) {
    let openings = document_openings(map);
    if openings.len() <= 1 {
        return;
    }
    let total = openings.len();
    for (index, opening) in openings.iter().enumerate().skip(1) {
        diagnostics.push(finding(
            YamlDiagnosticCode::MultiDocument,
            opening.clone(),
            YamlCertainty::DeterministicFindNonDeterministicSolution,
            format!(
                "stream contains {total} YAML documents; this starts document {} and single-document analysis cannot select, split, or rewrite it",
                index + 1
            ),
        ));
    }
}

/// Every document opening in the stream, in source order: an implicit
/// opening for content before the first `---`, each `---` marker, and each
/// content run after a `...` marker with no intervening `---`.
fn document_openings(map: &SourceMap) -> Vec<SourceSpan> {
    let mut openings = Vec::new();
    let starts: Vec<_> = map
        .markers()
        .iter()
        .filter(|marker| marker.kind == MarkerKind::Start)
        .collect();
    let first_start = starts.first().map(|marker| marker.span.start);
    if let Some(line) = map.lines().iter().find(|line| {
        line.kind == LineKind::Content && first_start.is_none_or(|start| line.span.start < start)
    }) {
        openings.push(line.content.clone());
    }
    openings.extend(starts.iter().map(|marker| marker.span.clone()));
    for end in map
        .markers()
        .iter()
        .filter(|marker| marker.kind == MarkerKind::End)
    {
        let next_start = starts
            .iter()
            .find(|marker| marker.span.start > end.span.start)
            .map(|marker| marker.span.start);
        if let Some(line) = map.lines().iter().find(|line| {
            line.kind == LineKind::Content
                && line.span.start > end.span.end
                && next_start.is_none_or(|start| line.span.start < start)
        }) {
            openings.push(line.content.clone());
        }
    }
    openings.sort_by_key(|span| (span.start, span.end));
    openings
}

// ===== Plain scalar enumeration and core-schema resolution =====

/// Where a plain scalar was found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PlainKind {
    /// A block mapping's inline value.
    MappingValue,
    /// A block sequence entry's inline value.
    DashValue,
    /// A block mapping's key.
    MappingKey,
}

/// A plain scalar occurrence eligible for the value lints.
#[derive(Debug, Clone)]
struct PlainValue {
    span: SourceSpan,
    kind: PlainKind,
}

/// Enumerates plain scalars in block mapping values, sequence entries, and
/// mapping keys. Quoted scalars, flow content, anchored/tagged values, block
/// scalar headers, and values continued on the next line are excluded —
/// their resolution is not cleanly visible at the lexeme level.
fn plain_values(source: &str, map: &SourceMap) -> Vec<PlainValue> {
    let mut values = Vec::new();
    for (index, line) in map.lines().iter().enumerate() {
        if line.kind != LineKind::Content || map.in_flow(line.content.start) {
            continue;
        }
        if map
            .block_scalars()
            .iter()
            .any(|scalar| scalar.header_line == index)
        {
            continue;
        }
        let entry = map.entry(index);
        if let Some(mapping) = &entry.mapping {
            if !map.quoted_intersects(&mapping.key) {
                values.push(PlainValue {
                    span: mapping.key.clone(),
                    kind: PlainKind::MappingKey,
                });
            }
            if let Some(value) = &mapping.value
                && is_plain_scalar_value(source, map, index, value)
            {
                values.push(PlainValue {
                    span: value.clone(),
                    kind: PlainKind::MappingValue,
                });
            }
        }
        if entry.dash.is_some()
            && entry.mapping.is_none()
            && let Some(value) = &entry.dash_value
            && is_plain_scalar_value(source, map, index, value)
        {
            values.push(PlainValue {
                span: value.clone(),
                kind: PlainKind::DashValue,
            });
        }
    }
    values
}

/// A value region holds a lintable plain scalar when it intersects no
/// quoted scalar, flow region, or anchor, does not open a tag/anchor/block
/// construct, and is not continued on the next line.
fn is_plain_scalar_value(
    source: &str,
    map: &SourceMap,
    line: usize,
    span: &SourceSpan,
) -> bool {
    if map.quoted_intersects(span) || map.flow_intersects(span) {
        return false;
    }
    if map.anchors().iter().any(|anchor| {
        span.start < anchor.full.end && anchor.full.start < span.end
    }) {
        return false;
    }
    let text = &source[span.clone()];
    if text.starts_with(['|', '>', '!', '&', '*']) {
        return false;
    }
    !has_plain_continuation(map, line)
}

/// A plain scalar value continues on the next line when the following
/// content line is more indented and carries no sequence or mapping entry
/// of its own.
fn has_plain_continuation(map: &SourceMap, line: usize) -> bool {
    let indent = map.lines()[line].indent;
    for candidate in line + 1..map.lines().len() {
        let candidate_line = &map.lines()[candidate];
        match candidate_line.kind {
            LineKind::Blank | LineKind::Comment => continue,
            LineKind::Content => {
                let entry = map.entry(candidate);
                return candidate_line.indent > indent
                    && entry.dash.is_none()
                    && entry.mapping.is_none();
            }
            _ => return false,
        }
    }
    false
}

/// YAML 1.2 core-schema resolution of a plain scalar, mirroring
/// `serde_yaml_ng` (verified by cross-check tests). Purely lexical — this
/// never invokes the parser, keeping the analyzer's parse-once contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PlainResolution {
    Null,
    Bool(bool),
    Int,
    Float,
    Text,
}

fn resolve_plain(text: &str) -> PlainResolution {
    match text {
        "" | "~" | "null" | "Null" | "NULL" => return PlainResolution::Null,
        "true" | "True" | "TRUE" => return PlainResolution::Bool(true),
        "false" | "False" | "FALSE" => return PlainResolution::Bool(false),
        _ => {}
    }
    let unsigned = text.strip_prefix(['-', '+']).unwrap_or(text);
    if !unsigned.is_empty()
        && unsigned.bytes().all(|byte| byte.is_ascii_digit())
        && (unsigned.len() == 1 || !unsigned.starts_with('0'))
    {
        return PlainResolution::Int;
    }
    for (prefix, radix) in [("0x", 16u32), ("0o", 8u32), ("0b", 2u32)] {
        if let Some(digits) = unsigned.strip_prefix(prefix)
            && !digits.is_empty()
            && digits.bytes().all(|byte| matches!(radix, 16) && byte.is_ascii_hexdigit()
                || matches!(radix, 8) && matches!(byte, b'0'..=b'7')
                || matches!(radix, 2) && matches!(byte, b'0' | b'1'))
        {
            return PlainResolution::Int;
        }
    }
    let lower = unsigned.to_ascii_lowercase();
    if lower == ".inf" || lower == ".nan" {
        return PlainResolution::Float;
    }
    if is_float_form(unsigned) {
        return PlainResolution::Float;
    }
    PlainResolution::Text
}

/// The core-schema float form: digits with an optional fraction, or a
/// leading-dot fraction, with an optional exponent.
fn is_float_form(text: &str) -> bool {
    let bytes = text.as_bytes();
    let mut index = 0;
    let digits_before = consume_digits(bytes, &mut index);
    let mut has_dot = false;
    let mut fraction = 0;
    if bytes.get(index) == Some(&b'.') {
        has_dot = true;
        index += 1;
        fraction = consume_digits(bytes, &mut index);
    }
    if !has_dot && digits_before == 0 {
        return false;
    }
    if digits_before == 0 && fraction == 0 {
        return false;
    }
    let mut has_exponent = false;
    if matches!(bytes.get(index), Some(b'e') | Some(b'E')) {
        has_exponent = true;
        index += 1;
        if matches!(bytes.get(index), Some(b'-') | Some(b'+')) {
            index += 1;
        }
        if consume_digits(bytes, &mut index) == 0 {
            return false;
        }
    }
    (has_dot || has_exponent) && index == bytes.len()
}

fn consume_digits(bytes: &[u8], index: &mut usize) -> usize {
    let start = *index;
    while bytes.get(*index).is_some_and(u8::is_ascii_digit) {
        *index += 1;
    }
    *index - start
}

// ===== Ambiguous scalars =====

/// Warns about plain scalars whose resolved type or cross-dialect behavior
/// is surprising, and about non-string mapping keys. Canonical `true`,
/// `false`, `null`, plain integers, and plain floats with lossless
/// spellings are common intentional YAML and stay quiet as *values*; every
/// non-string resolution is surprising as a *key*. Flow-collection scalars
/// are out of scope (documented boundary).
fn ambiguous_scalars(source: &str, map: &SourceMap, diagnostics: &mut Vec<YamlDiagnostic>) {
    for value in plain_values(source, map) {
        let text = &source[value.span.clone()];
        let message = if value.kind == PlainKind::MappingKey {
            ambiguous_key_reason(text)
                .map(|reason| format!("mapping key `{text}` {reason}; quote it if a string key was intended"))
        } else {
            ambiguous_value_reason(text)
                .map(|reason| format!("plain scalar `{text}` {reason}; quote it if a string was intended"))
        };
        if let Some(message) = message {
            diagnostics.push(finding(
                YamlDiagnosticCode::AmbiguousScalar,
                value.span.clone(),
                YamlCertainty::NonDeterministicFind,
                message,
            ));
        }
    }
}

/// Why a mapping key is ambiguous: every non-string resolution is
/// surprising as a key; string keys get only the cross-dialect and
/// portability notes.
fn ambiguous_key_reason(text: &str) -> Option<String> {
    match resolve_plain(text) {
        PlainResolution::Text => ambiguous_text_reason(text),
        PlainResolution::Null => Some("parses as null".to_string()),
        PlainResolution::Bool(value) => Some(format!("parses as the boolean {value}")),
        PlainResolution::Int => Some(format!("parses as the number {}", rendered_number(text))),
        PlainResolution::Float => Some(format!(
            "parses as the number {}",
            rendered_number(text)
        )),
    }
}

/// Why a plain scalar *value* is ambiguous, or `None` when it is common
/// intentional YAML.
fn ambiguous_value_reason(text: &str) -> Option<String> {
    match resolve_plain(text) {
        // Canonical null/bool spellings are overwhelmingly intentional.
        PlainResolution::Null if text == "null" => None,
        PlainResolution::Bool(_) if matches!(text, "true" | "false") => None,
        PlainResolution::Null => Some("parses as null".to_string()),
        PlainResolution::Bool(value) => Some(format!("parses as the boolean {value}")),
        PlainResolution::Int => {
            let unsigned = text.strip_prefix(['-', '+']).unwrap_or(text);
            if unsigned.starts_with("0x") || unsigned.starts_with("0o") || unsigned.starts_with("0b")
            {
                Some(format!(
                    "parses as the number {} (non-decimal form)",
                    rendered_number(text)
                ))
            } else {
                // Plain decimal integers are common intentional YAML.
                None
            }
        }
        PlainResolution::Float => {
            let lower = text.to_ascii_lowercase();
            let unsigned = lower.strip_prefix(['-', '+']).unwrap_or(&lower);
            if unsigned == ".inf" {
                Some("parses as infinity".to_string())
            } else if unsigned == ".nan" {
                Some("parses as NaN (not a number)".to_string())
            } else if text.contains(['e', 'E']) {
                Some(format!(
                    "parses as the number {} (scientific notation)",
                    rendered_number(text)
                ))
            } else {
                let fraction = text.split('.').nth(1).unwrap_or("");
                if fraction.is_empty() || fraction.ends_with('0') {
                    Some(format!(
                        "parses as the number {}; the source spelling loses trailing digits",
                        rendered_number(text)
                    ))
                } else {
                    // Canonical floats such as `1.5` are common intentional YAML.
                    None
                }
            }
        }
        PlainResolution::Text => ambiguous_text_reason(text),
    }
}

/// Portability notes for scalars that resolve to strings here: YAML 1.1
/// boolean spellings, leading-zero digit strings, and timestamp shapes.
fn ambiguous_text_reason(text: &str) -> Option<String> {
    let lower = text.to_ascii_lowercase();
    if matches!(lower.as_str(), "yes" | "no" | "on" | "off") {
        return Some("parses as a string here but as a boolean in YAML 1.1 tools".to_string());
    }
    let unsigned = text.strip_prefix(['-', '+']).unwrap_or(text);
    if unsigned.len() > 1
        && unsigned.starts_with('0')
        && unsigned.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Some(
            "parses as a string here but as a number in many YAML tools (leading zero)"
                .to_string(),
        );
    }
    if is_timestamp_shaped(text) {
        return Some("parses as a string here but as a timestamp in many YAML tools".to_string());
    }
    None
}

/// The parsed value's shortest rendering for diagnostic messages.
fn rendered_number(text: &str) -> String {
    if let Ok(integer) = text.parse::<i64>() {
        return integer.to_string();
    }
    let unsigned = text.strip_prefix(['-', '+']).unwrap_or(text);
    let parsed = if let Some(digits) = unsigned.strip_prefix("0x") {
        i64::from_str_radix(digits, 16).ok()
    } else if let Some(digits) = unsigned.strip_prefix("0o") {
        i64::from_str_radix(digits, 8).ok()
    } else if let Some(digits) = unsigned.strip_prefix("0b") {
        i64::from_str_radix(digits, 2).ok()
    } else {
        None
    };
    if let Some(integer) = parsed {
        let signed = if text.starts_with('-') { -integer } else { integer };
        return signed.to_string();
    }
    if let Ok(float) = text.parse::<f64>() {
        return float.to_string();
    }
    text.to_string()
}

/// Timestamp-shaped text: `YYYY-MM-DD` optionally followed by a time part.
/// Date detection is shape-only (month/day ranges are not validated).
fn is_timestamp_shaped(text: &str) -> bool {
    let bytes = text.as_bytes();
    if bytes.len() < 10 {
        return false;
    }
    let digits_at = |positions: &[usize]| {
        positions
            .iter()
            .all(|position| bytes[*position].is_ascii_digit())
    };
    digits_at(&[0, 1, 2, 3, 5, 6, 8, 9]) && bytes[4] == b'-' && bytes[7] == b'-'
}

// ===== Suspicious empty values =====

/// Reports mapping keys and sequence entries with no inline value and no
/// nested block, which resolve to null. Container keys (a more-indented
/// block follows) are intentional structure and stay quiet. Entries whose
/// value position holds a *tight* comment (`#` followed by non-space) defer
/// to the comment-truncation detector's more specific finding; a spaced
/// prose comment does not suppress — a null placeholder with a note is
/// still a possible accidental null.
fn suspicious_empty_values(source: &str, map: &SourceMap, diagnostics: &mut Vec<YamlDiagnostic>) {
    for (index, line) in map.lines().iter().enumerate() {
        if line.kind != LineKind::Content || map.in_flow(line.content.start) {
            continue;
        }
        let entry = map.entry(index);
        if let Some(mapping) = &entry.mapping
            && mapping.value.is_none()
            && !has_nested_block(map, index)
            && !tight_comment_between(source, map, mapping.colon + 1, line.content.end)
        {
            let key = &source[mapping.key.clone()];
            diagnostics.push(finding(
                YamlDiagnosticCode::SuspiciousEmptyValue,
                mapping.key.clone(),
                YamlCertainty::NonDeterministicFind,
                format!(
                    "key `{key}` has no value and resolves to null; add a value or an explicit empty string if null was not intended"
                ),
            ));
        }
        if let Some(dash) = entry.dash
            && entry.dash_value.is_none()
            && entry.mapping.is_none()
            && !has_nested_block(map, index)
            && !tight_comment_between(source, map, dash + 1, line.content.end)
        {
            diagnostics.push(finding(
                YamlDiagnosticCode::SuspiciousEmptyValue,
                dash..dash + 1,
                YamlCertainty::NonDeterministicFind,
                "sequence entry has no value and resolves to null".to_string(),
            ));
        }
    }
}

/// A line parents a nested block when the next non-blank, non-comment line
/// is a more-indented content line.
fn has_nested_block(map: &SourceMap, line: usize) -> bool {
    let indent = map.lines()[line].indent;
    for candidate in line + 1..map.lines().len() {
        let candidate_line = &map.lines()[candidate];
        match candidate_line.kind {
            LineKind::Blank | LineKind::Comment => continue,
            LineKind::Content => return candidate_line.indent > indent,
            _ => return false,
        }
    }
    false
}

/// A tight comment (`#` followed by a non-space, non-`#` byte) starts
/// strictly inside `start..end`.
fn tight_comment_between(source: &str, map: &SourceMap, start: usize, end: usize) -> bool {
    map.comments().iter().any(|comment| {
        comment.start >= start
            && comment.start < end
            && source
                .as_bytes()
                .get(comment.start + 1)
                .is_some_and(|byte| !matches!(byte, b' ' | b'\t' | b'#'))
    })
}

// ===== Block scalar smells =====

/// Warns when a folded (`>`) block scalar appears to hold line-oriented
/// content: shell commands, scripts, PEM material, patches, or templates.
/// Literal (`|`) scalars are never flagged. A folded scalar stays quiet
/// unless it has at least two non-blank content lines (a single folded line
/// is prose-shaped), no content line ends with sentence punctuation (prose
/// folds intentionally), and at least one line carries a structural signal:
/// a shell prompt, shebang, PEM armor, diff marker, shell operator,
/// template marker, or a leading word from a small curated set of
/// unambiguous shell and build-tool invocations (unrecognized commands stay
/// quiet — the vocabulary boundary is deliberate).
fn block_scalar_smells(source: &str, map: &SourceMap, diagnostics: &mut Vec<YamlDiagnostic>) {
    for scalar in map.block_scalars() {
        if source.as_bytes().get(scalar.indicator) != Some(&b'>') {
            continue;
        }
        let lines = block_content_lines(source, map, scalar);
        if lines.len() < 2 {
            continue;
        }
        let prose = lines.iter().any(|text| {
            text.ends_with(['.', ',', ';', '?', '!'])
        });
        if prose {
            continue;
        }
        if !lines.iter().any(|text| has_line_oriented_signal(text)) {
            continue;
        }
        diagnostics.push(finding(
            YamlDiagnosticCode::BlockScalarSmell,
            scalar.indicator..scalar.indicator + 1,
            YamlCertainty::NonDeterministicFind,
            format!(
                "folded scalar (`>`) joins these {} lines into one line; use `|` if they are meant to stay separate",
                lines.len()
            ),
        ));
    }
}

/// A line carries a line-oriented-content signal.
fn has_line_oriented_signal(text: &str) -> bool {
    if text.starts_with("$ ")
        || text == "$"
        || text.starts_with("#!")
        || text.starts_with("-----BEGIN ")
        || text.starts_with("diff --git ")
        || text.starts_with("@@ ")
        || text.contains("{{")
        || text.contains(" && ")
        || text.ends_with(" &&")
        || text.contains(" || ")
        || text.ends_with(" ||")
    {
        return true;
    }
    let first_word = text.split_whitespace().next().unwrap_or("");
    COMMAND_WORDS.contains(&first_word)
}

/// Unambiguous shell and build-tool invocations. Vocabulary-based detection
/// is a deliberate, bounded heuristic: words outside this list never
/// trigger the block-scalar smell on their own.
const COMMAND_WORDS: &[&str] = &[
    "echo", "cd", "set", "export", "source", "sh", "bash", "zsh", "env", "eval", "exec", "cargo",
    "npm", "pnpm", "yarn", "git", "curl", "wget", "docker", "kubectl", "sudo", "apt", "apt-get",
    "brew", "pip", "pip3", "python", "python3", "node", "make", "just", "rm", "cp", "mv", "mkdir",
    "cat", "chmod", "tar", "ssh", "scp", "rsync", "xargs", "grep", "sed", "awk", "touch", "ln",
];

/// Trimmed text of every non-blank content line of a block scalar.
fn block_content_lines<'a>(source: &'a str, map: &SourceMap, scalar: &BlockScalar) -> Vec<&'a str> {
    if scalar.content.is_empty() {
        return Vec::new();
    }
    map.lines()
        .iter()
        .filter(|line| {
            line.kind == LineKind::BlockContent
                && line.span.start >= scalar.content.start
                && line.span.end <= scalar.content.end
        })
        .filter_map(|line| {
            let text = source[line.content.clone()].trim();
            if text.is_empty() { None } else { Some(text) }
        })
        .collect()
}

// ===== Comment truncation and indicator smells =====

/// Warns when a comment occupies or trails a value position and its text
/// resembles value content. Only *tight* comments qualify — `#` followed
/// immediately by a non-space, non-`#` byte — because spaced comments read
/// as intentional prose (the yamllint comment-spacing convention). Also
/// warns when a double-quoted Windows path contains YAML escapes that
/// silently change its content. Quoted values followed by comments are left
/// quiet: the quotes already mark the value boundary.
fn comment_truncation(source: &str, map: &SourceMap, diagnostics: &mut Vec<YamlDiagnostic>) {
    let bytes = source.as_bytes();
    for (index, line) in map.lines().iter().enumerate() {
        if line.kind != LineKind::Content || map.in_flow(line.content.start) {
            continue;
        }
        let entry = map.entry(index);
        let comments: Vec<_> = map
            .comments()
            .iter()
            .filter(|comment| {
                comment.start >= line.content.start && comment.start < line.content.end
            })
            .collect();
        for comment in comments {
            let Some(&next) = bytes.get(comment.start + 1) else {
                continue;
            };
            if matches!(next, b' ' | b'\t' | b'#') {
                continue;
            }
            let comment_text = &source[comment.clone()];
            if let Some(mapping) = &entry.mapping {
                if mapping.value.is_none()
                    && comment.start > mapping.colon
                    && bytes[mapping.colon + 1..comment.start]
                        .iter()
                        .all(|byte| matches!(byte, b' ' | b'\t'))
                {
                    let key = &source[mapping.key.clone()];
                    diagnostics.push(finding(
                        YamlDiagnosticCode::CommentTruncation,
                        comment.clone(),
                        YamlCertainty::NonDeterministicFind,
                        format!(
                            "the value of `{key}` is null; `{comment_text}` reads as a comment — quote it if it was meant as the value"
                        ),
                    ));
                    continue;
                }
                if let Some(value) = &mapping.value
                    && is_plain_scalar_value(source, map, index, value)
                    && comment.start >= value.end
                {
                    let key = &source[mapping.key.clone()];
                    let shown = &source[value.clone()];
                    diagnostics.push(finding(
                        YamlDiagnosticCode::CommentTruncation,
                        comment.clone(),
                        YamlCertainty::NonDeterministicFind,
                        format!(
                            "`{comment_text}` starts a comment, so the value of `{key}` is only `{shown}` — quote the whole token if the `#` was meant as content"
                        ),
                    ));
                }
                continue;
            }
            if entry.dash.is_some()
                && entry.mapping.is_none()
                && let Some(dash) = entry.dash
            {
                match &entry.dash_value {
                    None if comment.start > dash
                        && bytes[dash + 1..comment.start]
                            .iter()
                            .all(|byte| matches!(byte, b' ' | b'\t')) =>
                    {
                        diagnostics.push(finding(
                            YamlDiagnosticCode::CommentTruncation,
                            comment.clone(),
                            YamlCertainty::NonDeterministicFind,
                            format!(
                                "the sequence entry is null; `{comment_text}` reads as a comment — quote it if it was meant as content"
                            ),
                        ));
                    }
                    Some(value)
                        if is_plain_scalar_value(source, map, index, value)
                            && comment.start >= value.end =>
                    {
                        let shown = &source[value.clone()];
                        diagnostics.push(finding(
                            YamlDiagnosticCode::CommentTruncation,
                            comment.clone(),
                            YamlCertainty::NonDeterministicFind,
                            format!(
                                "`{comment_text}` starts a comment, so the entry is only `{shown}` — quote the whole token if the `#` was meant as content"
                            ),
                        ));
                    }
                    _ => {}
                }
            }
        }
    }
    windows_path_escape_smells(source, map, diagnostics);
}

/// Warns when a double-quoted scalar spelling a Windows path (`X:\...`)
/// contains YAML escape sequences that silently change its content, such as
/// `\n` becoming a newline. Single-quoted or plain paths stay quiet.
fn windows_path_escape_smells(source: &str, map: &SourceMap, diagnostics: &mut Vec<YamlDiagnostic>) {
    for quoted in map.quoted_scalars() {
        if quoted.style != QuoteStyle::Double || !quoted.closed {
            continue;
        }
        let text = &source[quoted.span.clone()];
        let inner = &text[1..text.len() - 1];
        let drive_path = inner.len() > 2
            && inner.as_bytes()[0].is_ascii_alphabetic()
            && inner.as_bytes()[1] == b':'
            && inner.as_bytes()[2] == b'\\';
        if !drive_path {
            continue;
        }
        let surprising: Vec<char> = inner
            .chars()
            .collect::<Vec<_>>()
            .windows(2)
            .filter(|pair| pair[0] == '\\' && matches!(pair[1], 'n' | 't' | 'r' | 'f' | 'b' | '0'))
            .map(|pair| pair[1])
            .collect();
        if surprising.is_empty() {
            continue;
        }
        diagnostics.push(finding(
            YamlDiagnosticCode::CommentTruncation,
            quoted.span.clone(),
            YamlCertainty::NonDeterministicFind,
            format!(
                "`{text}` contains YAML escape sequences (`\\{}` changes the path); use single quotes for Windows paths",
                surprising[0]
            ),
        ));
    }
}

// ===== Style and indentation inconsistency =====

/// Detects two maintenance smells with strong signals: mixed block
/// indentation widths (one summary diagnostic naming the widths), and
/// mixed boolean spellings for the same value (one diagnostic per
/// non-majority spelling). Quoting-style mixing is deliberately not
/// reported: escapes legitimately force double quotes, so mixed quote
/// styles are idiomatic.
fn style_inconsistency(source: &str, map: &SourceMap, diagnostics: &mut Vec<YamlDiagnostic>) {
    indentation_widths(map, diagnostics);
    boolean_spellings(source, map, diagnostics);
}

/// Reports the first line whose indentation step differs from the
/// document's majority step. Steps are measured between nested mapping or
/// sequence lines; flow content, block scalar content, quoted scalar
/// continuations, and plain scalar continuations never contribute a step.
fn indentation_widths(map: &SourceMap, diagnostics: &mut Vec<YamlDiagnostic>) {
    let mut steps: Vec<(usize, usize)> = Vec::new();
    for (index, line) in map.lines().iter().enumerate() {
        if line.kind != LineKind::Content || line.indent == 0 || map.in_flow(line.content.start) {
            continue;
        }
        if inside_multiline_quoted(map, line.content.start) {
            continue;
        }
        let entry = map.entry(index);
        if entry.dash.is_none() && entry.mapping.is_none() {
            continue;
        }
        let Some(parent) = structural_parent(map, index) else {
            continue;
        };
        steps.push((index, line.indent - map.lines()[parent].indent));
    }
    let majority = majority_step(&steps);
    let Some(majority) = majority else {
        return;
    };
    let distinct: Vec<usize> = {
        let mut widths: Vec<usize> = steps.iter().map(|(_, step)| *step).collect();
        widths.sort_unstable();
        widths.dedup();
        widths
    };
    if distinct.len() < 2 {
        return;
    }
    if let Some((line, step)) = steps.iter().find(|(_, step)| *step != majority) {
        let span = map.lines()[*line].content.clone();
        let widths = distinct
            .iter()
            .map(|width| width.to_string())
            .collect::<Vec<_>>()
            .join(" and ");
        diagnostics.push(finding(
            YamlDiagnosticCode::StyleInconsistency,
            span,
            YamlCertainty::NonDeterministicFind,
            format!(
                "indentation step of {step} spaces here, but most of the document uses {majority}; mixed widths ({widths}) make nesting easy to misread"
            ),
        ));
    }
}

/// The nearest previous content line that can parent `line`: smaller
/// indent, a sequence or mapping entry, outside flow regions and multiline
/// quoted scalars.
fn structural_parent(map: &SourceMap, line: usize) -> Option<usize> {
    let indent = map.lines()[line].indent;
    for candidate in (0..line).rev() {
        let candidate_line = &map.lines()[candidate];
        if candidate_line.kind != LineKind::Content || candidate_line.indent >= indent {
            continue;
        }
        if map.in_flow(candidate_line.content.start)
            || inside_multiline_quoted(map, candidate_line.content.start)
        {
            continue;
        }
        let entry = map.entry(candidate);
        if entry.dash.is_none() && entry.mapping.is_none() {
            continue;
        }
        return Some(candidate);
    }
    None
}

/// The byte falls inside a quoted scalar that started on an earlier line.
fn inside_multiline_quoted(map: &SourceMap, byte: usize) -> bool {
    map.quoted_scalars().iter().any(|quoted| {
        quoted.span.start < byte
            && byte < quoted.span.end
            && map.line_at_byte(quoted.span.start) != map.line_at_byte(byte)
    })
}

/// The most frequent indentation step; ties resolve to the step that
/// appears first in the document.
fn majority_step(steps: &[(usize, usize)]) -> Option<usize> {
    let mut best: Option<(usize, usize, usize)> = None; // (step, count, first index)
    for (position, (_, step)) in steps.iter().enumerate() {
        let count = steps.iter().filter(|(_, other)| other == step).count();
        let replace = match best {
            None => true,
            Some((_, best_count, best_position)) => {
                count > best_count || (count == best_count && position < best_position)
            }
        };
        if replace {
            best = Some((*step, count, position));
        }
    }
    best.map(|(step, _, _)| step)
}

/// Reports each boolean value whose spelling differs from the document's
/// majority spelling for that value (`true` versus `True`, and so on).
/// Ties resolve to the canonical lowercase spelling.
fn boolean_spellings(source: &str, map: &SourceMap, diagnostics: &mut Vec<YamlDiagnostic>) {
    let values = plain_values(source, map);
    let spellings: Vec<&PlainValue> = values
        .iter()
        .filter(|value| value.kind != PlainKind::MappingKey)
        .filter(|value| {
            matches!(
                resolve_plain(&source[value.span.clone()]),
                PlainResolution::Bool(_)
            )
        })
        .collect();
    for canonical in ["true", "false"] {
        let group: Vec<&PlainValue> = spellings
            .iter()
            .filter(|value| source[value.span.clone()].eq_ignore_ascii_case(canonical))
            .copied()
            .collect();
        let mut distinct: Vec<String> = group
            .iter()
            .map(|value| source[value.span.clone()].to_string())
            .collect();
        distinct.sort();
        distinct.dedup();
        if distinct.len() < 2 {
            continue;
        }
        let mut majority: Option<String> = None;
        for spelling in &distinct {
            let count = group
                .iter()
                .filter(|value| &source[value.span.clone()] == spelling)
                .count();
            let replace = match &majority {
                None => true,
                Some(current) => {
                    let current_count = group
                        .iter()
                        .filter(|value| &source[value.span.clone()] == current)
                        .count();
                    count > current_count || (count == current_count && spelling == canonical)
                }
            };
            if replace {
                majority = Some(spelling.clone());
            }
        }
        let Some(majority) = majority else { continue };
        for value in group
            .iter()
            .filter(|value| source[value.span.clone()] != majority)
        {
            let text = &source[value.span.clone()];
            diagnostics.push(finding(
                YamlDiagnosticCode::StyleInconsistency,
                value.span.clone(),
                YamlCertainty::NonDeterministicFind,
                format!(
                    "boolean spelling `{text}` differs from `{majority}` used elsewhere in the document; pick one spelling"
                ),
            ));
        }
    }
}

// ===== Similar and misplaced keys =====

/// Compares mapping keys within one scope and across sibling scopes (scopes
/// sharing the same parent path, such as `development:` and `production:`)
/// and reports near-matching pairs: small edit distances, transpositions,
/// and case/separator variants. Exact repeats across sibling scopes are
/// normal configuration shapes and stay quiet; exact duplicates within one
/// scope are the duplicate-key detector's finding.
fn similar_keys(source: &str, map: &SourceMap, diagnostics: &mut Vec<YamlDiagnostic>) {
    let occurrences = map.key_occurrences(source);
    let mut reported: Vec<(SourceSpan, String)> = Vec::new();
    for (position, occurrence) in occurrences.iter().enumerate() {
        let document = document_index(map, occurrence.key_span.start);
        for other in &occurrences[position + 1..] {
            if document_index(map, other.key_span.start) != document {
                continue;
            }
            if !scopes_comparable(&occurrence.path, &other.path) {
                continue;
            }
            if near_key_match(&occurrence.key_text, &other.key_text)
                && !reported.iter().any(|(span, _)| *span == other.key_span)
            {
                let earlier_line = line_number(map, occurrence.key_span.start);
                reported.push((
                    other.key_span.clone(),
                    format!(
                        "key `{}` is similar to `{}` at line {earlier_line}; possible typo or misplaced key",
                        other.key_text, occurrence.key_text
                    ),
                ));
            }
        }
    }
    reported.sort_by_key(|(span, _)| (span.start, span.end));
    for (span, message) in reported {
        diagnostics.push(finding(
            YamlDiagnosticCode::SimilarKey,
            span,
            YamlCertainty::NonDeterministicFind,
            message,
        ));
    }
}

/// A key path's scope: every segment but the final key.
fn scope_of(path: &[PathSegment]) -> &[PathSegment] {
    &path[..path.len().saturating_sub(1)]
}

/// Two key paths live in comparable scopes: the same scope, or sibling
/// scopes sharing a parent path (their scope paths differ only in the final
/// segment).
fn scopes_comparable(first: &[PathSegment], second: &[PathSegment]) -> bool {
    let first_scope = scope_of(first);
    let second_scope = scope_of(second);
    if first_scope == second_scope {
        return true;
    }
    if first_scope.len() != second_scope.len() || first_scope.is_empty() {
        return false;
    }
    scope_of(first_scope) == scope_of(second_scope)
}

/// Two distinct key spellings are suspiciously close. Lengths under three
/// never match (too much noise); lengths three and four tolerate one edit;
/// longer keys tolerate two. Case and `-`/`_` separator variants always
/// match. Keys equal after full normalization are caught here; identical
/// keys are other detectors' findings.
fn near_key_match(first: &str, second: &str) -> bool {
    if first == second {
        return false;
    }
    let length = first.chars().count().max(second.chars().count());
    if length < 3 {
        return false;
    }
    let normalize = |text: &str| text.to_lowercase().replace('_', "-");
    if normalize(first) == normalize(second) {
        return true;
    }
    let distance = edit_distance(first, second);
    (length >= 5 && distance <= 2) || distance <= 1
}

/// Optimal string alignment (Damerau with unit costs): insertions,
/// deletions, substitutions, and adjacent transpositions.
fn edit_distance(first: &str, second: &str) -> usize {
    let a: Vec<char> = first.chars().collect();
    let b: Vec<char> = second.chars().collect();
    let (rows, cols) = (a.len() + 1, b.len() + 1);
    let mut distances = vec![vec![0usize; cols]; rows];
    for (row, cell) in distances.iter_mut().enumerate().take(rows) {
        cell[0] = row;
    }
    for (col, cell) in distances[0].iter_mut().enumerate().take(cols) {
        *cell = col;
    }
    for row in 1..rows {
        for col in 1..cols {
            let cost = usize::from(a[row - 1] != b[col - 1]);
            distances[row][col] = (distances[row - 1][col] + 1)
                .min(distances[row][col - 1] + 1)
                .min(distances[row - 1][col - 1] + cost);
            if row > 1 && col > 1 && a[row - 1] == b[col - 2] && a[row - 2] == b[col - 1] {
                distances[row][col] = distances[row][col].min(distances[row - 2][col - 2] + 1);
            }
        }
    }
    distances[rows - 1][cols - 1]
}
