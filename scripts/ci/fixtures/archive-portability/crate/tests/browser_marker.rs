//! Browser-marker tier of the archive-portability fixture.
//!
//! Named to match the repository's browser filterset
//! (`test(/(^|::)browser_/)`). It launches no browser — see the L2 marker for
//! why the fixture stays free of production backends.

#[test]
fn browser_marker_runs_from_the_archive() {
    let path = archive_portability::repository_fixture();
    assert!(path.is_file(), "{} must exist", path.display());
}
