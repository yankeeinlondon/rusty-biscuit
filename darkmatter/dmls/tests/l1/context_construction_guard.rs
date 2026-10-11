//! Acceptance Criteria 2, 3, and 5 of `2026-09-30-file-refs-use-magic` for
//! `dmls`: the server reads its process once, in `main`, and every document
//! context comes from the builder (`RepositoryContexts`). A document without a
//! context is a `ContextFailure`, never an `Option`. The gates are documented
//! in `darkmatter/cli/tests/common/context_guard.rs`.

#[path = "../../../cli/tests/common/context_guard.rs"]
mod context_guard;

use context_guard::{Allowance, Gate};

const ALLOWLIST: &[Allowance] = &[Allowance {
    gate: Gate::Construction,
    path: "main.rs",
    identifier: "RequestSnapshot::from_process",
    count: 1,
    reason: "the server binary's one process read (R8)",
}];

#[test]
fn production_source_builds_contexts_only_through_the_builder() {
    // Spelled for CI's test-input index, which does not count the `#[path]`
    // include above (`source-inputs` in Cargo.toml).
    let _ = include_str!("../../../cli/tests/common/context_guard.rs");
    let _ = include_str!("../../../cli/tests/common/source_scan.rs");
    context_guard::assert_guarded("dmls", &biscuit_test_harness::manifest_dir!().join("src"), ALLOWLIST);
}
