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

use cases::{CASES, ParityOutcome};

#[test]
fn every_parity_case_matches_through_md_clean() {
    let _ = include_str!("../../../lib/tests/common/clean_parity/mod.rs");
    let fixture = CliProcessFixture::named("clean-parity-every-case");
    for case in CASES {
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
}
