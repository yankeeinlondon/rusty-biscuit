//! Acceptance Criteria 2, 3, and 5 of `2026-09-30-file-refs-use-magic` for
//! `claudine-gen`: it reads its process once, in `main`, and every area
//! resolves through a context built from that snapshot. Its other reads only
//! decide whether output is colored. The gates are documented in
//! `darkmatter/cli/tests/common/context_guard.rs`.

#[path = "../../../../darkmatter/cli/tests/common/context_guard.rs"]
mod context_guard;

use context_guard::{Allowance, Gate};

const ALLOWLIST: &[Allowance] = &[
    Allowance {
        gate: Gate::Construction,
        path: "main.rs",
        identifier: "RequestSnapshot::from_process",
        count: 1,
        reason: "the `claudine-gen` binary's one process read (R8)",
    },
    Allowance {
        gate: Gate::AmbientState,
        path: "report.rs",
        identifier: "std::env::var_os",
        count: 3,
        reason: "`FORCE_COLOR`, `CLICOLOR_FORCE`, `NO_COLOR` decide output color (rendering)",
    },
];

#[test]
fn production_source_builds_contexts_only_through_the_builder() {
    // Spelled for CI's test-input index, which does not count the `#[path]`
    // include above (`source-inputs` in Cargo.toml).
    let _ = include_str!("../../../../darkmatter/cli/tests/common/context_guard.rs");
    let _ = include_str!("../../../../darkmatter/cli/tests/common/source_scan.rs");
    context_guard::assert_guarded("claudine-gen", &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src"), ALLOWLIST);
}
