//! Upstream hygiene that `2026-10-18-schematic-fidelity-polish` verifies but
//! does not own.

use std::path::Path;

/// The duplicate `schematic/gen/schematic/` tree was a stale copy of the
/// generated crate tracked inside the generator's own directory. Its deletion
/// belongs to spec 1; this check only fails if the tree comes back.
#[test]
fn stale_nested_schematic_tree_is_absent() {
    let stale = Path::new(env!("CARGO_MANIFEST_DIR")).join("schematic");

    assert!(
        !stale.exists(),
        "stale duplicate tree {} exists; delete it rather than regenerate into it",
        stale.display()
    );
}
