//! The entry-point parity matrix's chooser runner: the candidates Claudine's
//! missing-property chooser offers for a `file(match(...))` property
//! ([`file_candidate_paths`]), from each launch directory, against the shared
//! glob rows. The chooser itself needs a terminal, so this calls its walk
//! here, in the binary's unit tests; `claudine __complete` covers the TAB walk
//! in `tests/l1/entry_point_parity.rs`. A focused test adds the directory
//! case-alias fixtures the shared matrix does not carry.

#[path = "../../../../../darkmatter/lib/tests/common/entry_point_parity/mod.rs"]
mod matrix;
#[path = "../../../tests/common/fs_capability.rs"]
mod fs_capability;

use matrix::{EntryPoint, Observed, Owner, ParityFixture, ParityReport, Row, rows_for};

use super::file_candidate_paths;
use crate::completion::scopes::ScopeContext;

#[test]
fn chooser_offers_every_glob_row_in_native_order() {
    // The coupling CI's test-input index reads; the `#[path]` include alone is
    // not counted.
    let _ = include_str!("../../../../../darkmatter/lib/tests/common/entry_point_parity/mod.rs");
    let root = tempfile::TempDir::new().expect("fixture root");
    let fixture = ParityFixture::create(root.path());
    let mut report = ParityReport::new(Owner::ClaudineCliChooser);
    for row in rows_for(Owner::ClaudineCliChooser) {
        let Row::GlobValue(cell) = row else {
            panic!("the chooser runs Table 2 glob rows only: {row:?}");
        };
        assert_eq!(cell.entry, EntryPoint::ClaudineChooser, "{row:?}");
        let launch = fixture.launch_dir(cell.launch);
        let paths = file_candidate_paths(&cell.patterns(), &ScopeContext::discover_from(&launch));
        let observed = if paths.is_empty() { Observed::Unresolved } else { Observed::Files(paths) };
        report.record(&fixture, &row, &fixture.expected_glob_value(&cell), &observed);
    }
    report.assert_parity();
}

/// Criterion 26 for the chooser's walk: absolute and bare patterns spelling a
/// directory with a case alias the filesystem folds (`DOCS` for `docs`, `ς`
/// for `Σ`, `ß` for `SS`) offer nothing, and the stored spelling offers the
/// file. Where an alias does not open (a case-sensitive filesystem), the
/// checks still run.
#[test]
fn chooser_rejects_absolute_directory_case_aliases() {
    for (stored, authored) in [("docs", "DOCS"), ("Σ", "ς"), ("SS", "ß")] {
        let tmp = tempfile::TempDir::new().expect("fixture root");
        std::fs::create_dir_all(tmp.path().join(".git")).unwrap();
        std::fs::create_dir_all(tmp.path().join(stored)).unwrap();
        std::fs::write(tmp.path().join(stored).join("a.md"), "# A\n").unwrap();
        fs_capability::probe_directory_alias(tmp.path(), stored, authored);
        let root = biscuit_file::to_portable_string(tmp.path());
        let ctx = ScopeContext::discover_from(tmp.path());
        let offered = |pattern: String| file_candidate_paths(&[pattern], &ctx);

        for mismatched in [format!("{root}/{authored}/*.md"), format!("{authored}/*.md")] {
            assert_eq!(offered(mismatched.clone()), Vec::<std::path::PathBuf>::new(), "`{mismatched}`");
        }
        for exact in [format!("{root}/{stored}/*.md"), format!("{stored}/*.md")] {
            let got = offered(exact.clone());
            assert!(got.iter().any(|path| path.ends_with(format!("{stored}/a.md"))), "`{exact}`: {got:?}");
        }
    }
}
