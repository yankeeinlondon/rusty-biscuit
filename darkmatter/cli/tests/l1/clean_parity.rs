//! The shared `clean_document` parity cases through `md clean`: printed
//! output, `--save`, and `--json`'s outcome fields match the library
//! transaction (and DMLS `textDocument/formatting`, which asserts the same
//! cases) byte for byte.

use crate::common;

use common::CliProcessFixture;
use serde_json::Value;
use std::fs;

#[path = "../../../lib/tests/common/clean_parity/mod.rs"]
mod cases;

use cases::{CASES, ParityCase, ParityOutcome};

fn assert_case(case: &ParityCase) {
    let fixture = CliProcessFixture::named(case.name);
    let path = fixture.write_file(format!("cwd/{}.md", case.name), case.input);

    let printed = fixture.command().arg("clean").arg(&path).output().unwrap();
    let json = fixture.command().arg("clean").arg(&path).arg("--json").output().unwrap();
    let report: Value = serde_json::from_slice(&json.stdout)
        .unwrap_or_else(|error| panic!("{}: not JSON ({error})", case.name));
    assert!(json.stderr.is_empty(), "{}: {}", case.name, String::from_utf8_lossy(&json.stderr));
    assert_eq!(report["changed"], case.outcome == ParityOutcome::Changed, "{}: {report:#}", case.name);
    if case.outcome != ParityOutcome::Changed {
        assert_eq!(report["applied"], serde_json::json!([]), "{}", case.name);
    }

    if case.outcome == ParityOutcome::Invalid {
        assert_eq!(printed.status.code(), Some(1), "{}", case.name);
        assert_eq!(json.status.code(), Some(1), "{}", case.name);
    } else {
        assert!(printed.status.success(), "{}: {printed:?}", case.name);
        assert!(json.status.success(), "{}", case.name);
        assert_eq!(String::from_utf8_lossy(&printed.stdout), case.expected, "{}", case.name);
    }
    let declined = report["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .any(|entry| entry["code"] == "clean.fidelity-fallback");
    assert_eq!(declined, case.outcome == ParityOutcome::Declined, "{}: {report:#}", case.name);

    let saved = fixture.command().arg("clean").arg(&path).arg("--save").output().unwrap();
    assert_eq!(saved.status.success(), case.outcome != ParityOutcome::Invalid, "{}", case.name);
    assert_eq!(fs::read_to_string(&path).unwrap(), case.expected, "{}: saved bytes", case.name);
}

macro_rules! parity_case_tests {
    ($($test:ident => $case:literal),+ $(,)?) => {
        const TESTED_CASES: &[&str] = &[$($case),+];

        $(
            #[test]
            fn $test() {
                let _ = include_str!("../../../lib/tests/common/clean_parity/mod.rs");
                let case = CASES.iter().find(|case| case.name == $case).expect("shared parity case");
                assert_case(case);
            }
        )+
    };
}

parity_case_tests! {
    repairable_frontmatter => "repairable-frontmatter",
    repair_disabled => "repair-disabled",
    repair_disabled_invalid_yaml => "repair-disabled-invalid-yaml",
    unrepairable_frontmatter => "unrepairable-frontmatter",
    bare_interpolation_frontmatter => "bare-interpolation-frontmatter",
    empty_frontmatter => "empty-frontmatter",
    bom_without_frontmatter => "bom-without-frontmatter",
    bom_before_frontmatter => "bom-before-frontmatter",
    crlf => "crlf",
    crlf_frontmatter => "crlf-frontmatter",
    mixed_line_endings => "mixed-line-endings",
    missing_final_newline => "missing-final-newline",
    reference_definitions => "reference-definitions",
    unpaired_directive_after_repairable_frontmatter => "unpaired-directive-after-repairable-frontmatter",
}

#[test]
fn every_shared_parity_case_has_one_cli_test() {
    let _ = include_str!("../../../lib/tests/common/clean_parity/mod.rs");
    let shared: Vec<_> = CASES.iter().map(|case| case.name).collect();
    assert_eq!(shared, TESTED_CASES);
    let unique: std::collections::BTreeSet<_> = TESTED_CASES.iter().collect();
    assert_eq!(unique.len(), TESTED_CASES.len(), "each case must be scheduled exactly once");
}
