//! Library positions for the shared `position_fixtures` documents, which the
//! CLI and DMLS suites check against the same expected starts.

use biscuit_file::FileResolutionContext;
use darkmatter::markdown::Markdown;
use darkmatter::markdown::schemas::{DarkmatterSchemas, ValidationProblemKind};

use crate::position_fixtures::{self, ExpectedStart};

fn kind_name(kind: &ValidationProblemKind) -> &'static str {
    match kind {
        ValidationProblemKind::Missing => "missing",
        ValidationProblemKind::Type => "type",
        ValidationProblemKind::Invalid => "invalid",
    }
}

#[test]
fn every_shared_case_reports_its_expected_starts() {
    let _ = include_str!("../position_fixtures/mod.rs");
    let api = DarkmatterSchemas::new(FileResolutionContext::new(std::env::temp_dir()));

    for case in position_fixtures::CASES {
        for line_end in ["\n", "\r\n"] {
            let md: Markdown = case.document.replace('\n', line_end).into();
            let report = api.validate(&md).expect("validate");

            assert!(!report.valid, "{}", case.name);
            assert_eq!(report.problems.len(), case.expected.len(), "{}: {:#?}", case.name, report.problems);
            for &ExpectedStart { path, kind, offending_property, line, column } in case.expected {
                let problem = report
                    .problems
                    .iter()
                    .find(|p| p.path == path && kind_name(&p.kind) == kind)
                    .unwrap_or_else(|| panic!("{}: no {kind} problem at {path:?}", case.name));
                assert_eq!(problem.offending_property.as_deref(), offending_property, "{}", case.name);
                assert_eq!(
                    (problem.line, problem.column),
                    (Some(line), Some(column)),
                    "{} {path:?} with {line_end:?} line ends",
                    case.name
                );
            }
        }
    }
}

/// The expected starts address the authored tokens: an unknown key's start
/// begins that key, and the shared byte-offset helper agrees with the
/// line/column it was given on LF and CRLF text alike.
#[test]
fn expected_starts_address_the_authored_tokens() {
    let _ = include_str!("../position_fixtures/mod.rs");
    for case in position_fixtures::CASES {
        for line_end in ["\n", "\r\n"] {
            let text = case.document.replace('\n', line_end);
            for expected in case.expected {
                let offset = position_fixtures::byte_offset(&text, expected.line, expected.column);
                if let Some(key) = expected.offending_property {
                    assert!(text[offset..].starts_with(&format!("{key}:")), "{} {key}", case.name);
                }
            }
        }
    }
    let acceptance = position_fixtures::ACCEPTANCE.document;
    assert_eq!(position_fixtures::byte_offset(acceptance, 5, 9), 48);
    assert_eq!(position_fixtures::byte_offset(&acceptance.replace('\n', "\r\n"), 5, 9), 52);
}
