//! The `claudine` binary keeps no second lifecycle implementation
//! (`2026-10-27-darkmatter-lifecycle-ownership`, acceptance 4).
//!
//! The CLI's drivers call Darkmatter's lifecycle kernel through the library's
//! adapters. This guard scans `claudine/cli/src` with the scan the library's
//! guard of the same name uses on `claudine/lib/src`, so a change to the CLI
//! alone runs it. The scan's rules and self-tests live with the scan, in
//! `claudine/lib/tests/common/lifecycle_ownership_scan.rs` and the library
//! guard.

#[path = "../../../../darkmatter/cli/tests/common/source_scan.rs"]
mod source_scan;
// The CLI needs no reviewed adapter today, so the scan's allowlist builder is
// unused here.
#[allow(dead_code)]
#[path = "../../../lib/tests/common/lifecycle_ownership_scan.rs"]
mod scan;

use scan::{Allowed, Owned, violations};
use source_scan::{module_tree_sources, production_sources};

/// Reviewed adapters in `claudine/cli/src`, keyed relative to `claudine/`.
/// An entry that matches nothing fails the guard.
const ALLOWED: &[Allowed] = &[];

#[test]
fn the_claudine_binary_keeps_no_second_lifecycle_implementation() {
    // Spelled for CI's test-input index, which does not count the `#[path]`
    // includes above (`source-inputs` in Cargo.toml).
    let _ = include_str!("../../../../darkmatter/cli/tests/common/source_scan.rs");
    let _ = include_str!("../../../lib/tests/common/lifecycle_ownership_scan.rs");
    // The twin variants come from the library's `From<LifecycleError>`
    // conversion (`scan::TWIN_SOURCE`), which this guard does not scan.
    let _ = include_str!("../../../lib/src/composition/error/lifecycle_authoring.rs");
    let errors = production_sources(&biscuit_test_harness::manifest_dir!().join("../lib/src/composition/error"));
    // Followed from the crate root, so a test module a test file declares by
    // `#[path]` is never read as production.
    let area = biscuit_test_harness::manifest_dir!().join("..");
    let (sources, problems) = module_tree_sources(&area, "claudine", &["cli/src/main.rs".to_string()]);
    assert!(problems.is_empty(), "{problems:#?}");
    assert!(sources.contains_key("cli/src/commands/wrap/composition/pipeline.rs"), "the CLI's drivers were not read");
    let darkmatter = production_sources(&biscuit_test_harness::manifest_dir!().join("../../darkmatter/lib/src/lifecycle"));
    let owned = Owned::read(&darkmatter, &errors["lifecycle_authoring.rs"]);
    assert!(owned.seams.contains("LifecycleRunHost") && owned.types.contains("LifecycleRunGuard"), "Darkmatter's lifecycle source was not read");
    assert!(owned.twins.contains("LifecycleRenamedEvent"), "the twin conversion was not read");

    let violations = violations(&sources, &owned, ALLOWED);

    assert!(violations.is_empty(), "lifecycle ownership violations:\n{}", violations.join("\n"));
}
