//! Parity between the library's validation-problem starts and DMLS's
//! diagnostic range starts over the shared `position_fixtures` documents.
//!
//! Both sides are reduced to byte offsets into the same complete Markdown
//! text, so the comparison holds in either negotiated encoding; the
//! non-ASCII case proves the conversion is load-bearing. Each DMLS range's
//! covered text is pinned too, so parity cannot pass by narrowing a range.

use biscuit_file::FileResolutionContext;
use darkmatter::markdown::Markdown;
use darkmatter::markdown::schemas::{DarkmatterSchemas, ValidationProblem, ValidationProblemKind};
use lsp_types::{Diagnostic, NumberOrString, Position};

use super::tests::diagnostics_with;
use crate::config::DmlsConfig;
use crate::diagnostics::codes::code;
use crate::source_map::{PositionEncoding, SourceMap};

#[path = "../../../../lib/tests/position_fixtures/mod.rs"]
mod position_fixtures;

use position_fixtures::{ExpectedStart, PositionCase};

/// What DMLS currently highlights for each fixture problem, keyed by
/// `(case, pointer)`, written with LF line ends.
const DMLS_RANGED_TEXT: &[(&str, &str, &str)] = &[
    ("acceptance", "/start/0/then/1", "sya"),
    ("block_list", "/tags/1", "oops"),
    ("block_list", "/items/1", "name: b\n"),
    ("flow", "/m", "b"),
    ("flow", "/n/a", "x"),
    ("flow", "/s/1", "two"),
    ("non_ascii_prefix", "/m", "zz"),
    ("non_ascii_prefix", "/n/b", "x"),
    ("root_missing", "", "other: x\n$schema:\n  title: string(required)\n"),
];

const ENCODINGS: [PositionEncoding; 2] = [PositionEncoding::Utf8, PositionEncoding::Utf16];
const LINE_ENDS: [&str; 2] = ["\n", "\r\n"];

fn kind_name(kind: &ValidationProblemKind) -> &'static str {
    match kind {
        ValidationProblemKind::Missing => "missing",
        ValidationProblemKind::Type => "type",
        ValidationProblemKind::Invalid => "invalid",
    }
}

/// The DMLS code an expected problem surfaces as.
fn dmls_code(expected: &ExpectedStart) -> &'static str {
    match (expected.kind, expected.offending_property) {
        (_, Some(_)) => code::SCHEMA_UNKNOWN_KEY,
        ("missing", None) => code::SCHEMA_MISSING_REQUIRED,
        ("type", None) => code::SCHEMA_TYPE_MISMATCH,
        (kind, None) => panic!("no DMLS code mapped for fixture kind {kind:?}"),
    }
}

fn code_of(diagnostic: &Diagnostic) -> &str {
    match &diagnostic.code {
        Some(NumberOrString::String(code)) => code,
        other => panic!("diagnostic without a string code: {other:?}"),
    }
}

fn strict_config() -> DmlsConfig {
    let mut config = DmlsConfig::default();
    config.schema.strict = true;
    config
}

/// The library problem for `expected`, after asserting its start addresses
/// the fixture's expected byte offset.
fn library_problem<'r>(
    problems: &'r [ValidationProblem],
    text: &str,
    case: &PositionCase,
    expected: &ExpectedStart,
) -> (&'r ValidationProblem, usize) {
    let problem = problems
        .iter()
        .find(|p| {
            p.path == expected.path
                && kind_name(&p.kind) == expected.kind
                && p.offending_property.as_deref() == expected.offending_property
        })
        .unwrap_or_else(|| panic!("{}: no library problem for {expected:?} in {problems:#?}", case.name));
    let (Some(line), Some(column)) = (problem.line, problem.column) else {
        panic!("{}: library problem {:?} has no position", case.name, expected.path);
    };
    let offset = position_fixtures::byte_offset(text, line, column);
    assert_eq!(
        offset,
        position_fixtures::byte_offset(text, expected.line, expected.column),
        "{} {:?}: library start",
        case.name,
        expected.path
    );
    (problem, offset)
}

/// Converts an LSP position to a byte offset from the source line alone,
/// independently of [`SourceMap`], counting `character` in `encoding`.
fn independent_byte_offset(text: &str, position: Position, encoding: PositionEncoding) -> usize {
    let line_start: usize = text
        .split_inclusive('\n')
        .take(position.line as usize)
        .map(str::len)
        .sum();
    let line = text[line_start..].split('\n').next().unwrap_or_default();
    let mut units = 0;
    for (index, ch) in line.char_indices() {
        if units == position.character as usize {
            return line_start + index;
        }
        units += match encoding {
            PositionEncoding::Utf8 => ch.len_utf8(),
            PositionEncoding::Utf16 => ch.len_utf16(),
        };
    }
    assert_eq!(units, position.character as usize, "position past the end of its line");
    line_start + line.len()
}

/// The byte offset of `position`, from [`SourceMap::lsp_to_byte`], checked
/// against an independent conversion over the source line.
fn start_byte(sm: &SourceMap, text: &str, position: Position) -> usize {
    let offset = sm.lsp_to_byte(position).expect("valid LSP position");
    assert_eq!(offset, independent_byte_offset(text, position, sm.encoding()), "{position:?}");
    offset
}

/// Runs both validators over every case in `encoding` and `line_end`, and
/// asserts each DMLS diagnostic starts at the library's byte offset and
/// covers the recorded text. Missing-required problems surface only under
/// `strict`.
fn assert_parity(encoding: PositionEncoding, line_end: &str, strict: bool) {
    let config = if strict { strict_config() } else { DmlsConfig::default() };
    let api = DarkmatterSchemas::new(FileResolutionContext::new(std::env::temp_dir()));

    for case in position_fixtures::CASES {
        let text = case.document.replace('\n', line_end);
        let markdown: Markdown = text.clone().into();
        let report = api.validate(&markdown).expect("library validation runs");
        let context = format!("{} {encoding:?} {line_end:?} strict={strict}", case.name);

        diagnostics_with(&text, encoding, &config, |diagnostics, sm| {
            let schema_diagnostics: Vec<&Diagnostic> =
                diagnostics.iter().filter(|d| code_of(d).starts_with("dm.schema.")).collect();
            let surfaced: Vec<&ExpectedStart> =
                case.expected.iter().filter(|e| strict || e.kind != "missing").collect();
            assert_eq!(schema_diagnostics.len(), surfaced.len(), "{context}: {diagnostics:#?}");

            for expected in surfaced {
                let (problem, library_offset) = library_problem(&report.problems, &text, case, expected);
                let matches: Vec<&&Diagnostic> = schema_diagnostics
                    .iter()
                    .filter(|d| code_of(d) == dmls_code(expected) && d.message == problem.message)
                    .collect();
                let [diagnostic] = matches.as_slice() else {
                    panic!("{context} {:?}: expected one DMLS diagnostic, got {matches:#?}", expected.path);
                };

                let start = start_byte(sm, &text, diagnostic.range.start);
                let end = start_byte(sm, &text, diagnostic.range.end);
                assert_eq!(start, library_offset, "{context} {:?}: DMLS start", expected.path);

                let (_, _, ranged) = DMLS_RANGED_TEXT
                    .iter()
                    .find(|(name, path, _)| *name == case.name && *path == expected.path)
                    .unwrap_or_else(|| panic!("no recorded DMLS range for {context} {:?}", expected.path));
                assert_eq!(
                    &text[start..end],
                    ranged.replace('\n', line_end),
                    "{context} {:?}: DMLS range",
                    expected.path
                );
            }
        });
    }
}

#[test]
fn dmls_starts_match_library_starts_in_utf8() {
    let _ = include_str!("../../../../lib/tests/position_fixtures/mod.rs");
    for strict in [false, true] {
        assert_parity(PositionEncoding::Utf8, "\n", strict);
    }
}

#[test]
fn dmls_starts_match_library_starts_in_utf16() {
    let _ = include_str!("../../../../lib/tests/position_fixtures/mod.rs");
    for strict in [false, true] {
        assert_parity(PositionEncoding::Utf16, "\n", strict);
    }
}

#[test]
fn dmls_starts_match_library_starts_with_crlf_line_ends() {
    let _ = include_str!("../../../../lib/tests/position_fixtures/mod.rs");
    for encoding in ENCODINGS {
        assert_parity(encoding, "\r\n", true);
    }
}

/// The non-ASCII prefix puts the same byte start at different UTF-8 and
/// UTF-16 columns, so the parity above depends on converting each encoding.
#[test]
fn non_ascii_prefix_columns_differ_between_encodings() {
    let _ = include_str!("../../../../lib/tests/position_fixtures/mod.rs");
    let case = position_fixtures::NON_ASCII_PREFIX;
    for line_end in LINE_ENDS {
        let text = case.document.replace('\n', line_end);
        let starts: Vec<Vec<(u32, u32)>> = ENCODINGS
            .iter()
            .map(|&encoding| {
                diagnostics_with(&text, encoding, &DmlsConfig::default(), |diagnostics, _| {
                    let mut starts: Vec<(u32, u32)> = diagnostics
                        .iter()
                        .map(|d| (d.range.start.line, d.range.start.character))
                        .collect();
                    starts.sort_unstable();
                    starts
                })
            })
            .collect();
        // The 4-byte emoji is 2 UTF-16 units, so each UTF-16 column is 2 less.
        assert_eq!(starts[0], vec![(1, 15), (2, 18)], "{line_end:?} UTF-8");
        assert_eq!(starts[1], vec![(1, 13), (2, 16)], "{line_end:?} UTF-16");
    }
}
