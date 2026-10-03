//! The entry-point parity matrix's chooser runner: the candidates Claudine's
//! missing-property chooser offers for a `file(match(...))` property
//! ([`file_candidate_paths`]), from each launch directory, against the shared
//! glob rows. The chooser itself needs a terminal, so this calls its walk
//! here, in the binary's unit tests; `claudine __complete` covers the TAB walk
//! in `tests/l1/entry_point_parity.rs`.

#[path = "../../../../../darkmatter/lib/tests/common/entry_point_parity/mod.rs"]
mod matrix;

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
