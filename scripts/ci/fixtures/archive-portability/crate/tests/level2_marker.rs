//! L2-marker tier of the archive-portability fixture.
//!
//! Named to match the repository's L2 filterset (`test(/(^|::)level2_/)`) so an
//! archive-mode `just _test_l2` selects it. It drives no terminal backend: the
//! fixture proves archive plumbing, and a red result here must never be read as
//! "no tmux on this host".

#[test]
fn level2_marker_runs_from_the_archive() {
    assert_eq!(
        archive_portability::dylib_marker(),
        "archive-portability-dylib"
    );
}
