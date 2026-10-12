//! The shared parity cases (`lib/tests/common/clean_parity`) against the
//! library transaction itself; the CLI and DMLS suites assert the same cases.

use std::convert::Infallible;

use darkmatter::markdown::cleanup::rules::MarkdownRuleOverrides;
use darkmatter::markdown::cleanup::{CleanOutcome, CleanRequest, clean_document};
use darkmatter::markdown::schemas::CleanSchemaConfig;

#[path = "../common/clean_parity/mod.rs"]
mod cases;

use cases::{CASES, ParityOutcome};

#[test]
fn every_parity_case_matches_the_library_transaction() {
    let _ = include_str!("../common/clean_parity/mod.rs");
    for case in CASES {
        let overrides = MarkdownRuleOverrides::default();
        let request = CleanRequest { source: case.input, overrides: &overrides, workspace: None, document_path: None };
        let result = clean_document(&request, || {
            Ok::<_, Infallible>(CleanSchemaConfig::new(crate::request_support::cwd_context()))
        })
        .expect("clean runs");
        let outcome = match result.outcome {
            CleanOutcome::Changed => ParityOutcome::Changed,
            CleanOutcome::Unchanged => ParityOutcome::Unchanged,
            CleanOutcome::Declined => ParityOutcome::Declined,
            CleanOutcome::Invalid => ParityOutcome::Invalid,
        };
        assert_eq!(outcome, case.outcome, "{}: {:?}", case.name, result.diagnostics);
        assert_eq!(result.text, case.expected, "{}", case.name);
        if outcome == ParityOutcome::Changed {
            assert_ne!(case.input, case.expected, "{}", case.name);
        } else {
            assert_eq!(case.input, case.expected, "{}", case.name);
            assert!(result.applied.is_empty(), "{}", case.name);
        }
        // Each emitted text is a fixed point.
        let again = CleanRequest { source: &result.text, ..request };
        let second = clean_document(&again, || {
            Ok::<_, Infallible>(CleanSchemaConfig::new(crate::request_support::cwd_context()))
        })
        .expect("clean runs");
        if outcome == ParityOutcome::Changed {
            assert_eq!(second.outcome, CleanOutcome::Unchanged, "{}: {:?}", case.name, second.text);
        }
    }
}

