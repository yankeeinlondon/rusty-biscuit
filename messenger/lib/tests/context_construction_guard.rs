//! Acceptance Criteria 2, 3, and 5 of `2026-09-30-file-refs-use-magic` for
//! the `messenger` library: research documents resolve through a context the
//! caller built from its snapshot, and the library reads the process only for
//! platform integration. The gates are documented in
//! `darkmatter/cli/tests/common/context_guard.rs`.

#[path = "../../../darkmatter/cli/tests/common/context_guard.rs"]
mod context_guard;

use context_guard::{Allowance, Gate};

const ALLOWLIST: &[Allowance] = &[Allowance {
    gate: Gate::AmbientState,
    path: "provider/desktop/windows.rs",
    identifier: "std::env::var",
    count: 1,
    reason: "`%APPDATA%` locates the Start Menu shortcut Windows notifications need",
}];

#[test]
fn production_source_builds_contexts_only_through_the_builder() {
    // Spelled for CI's test-input index, which does not count the `#[path]`
    // include above (`source-inputs` in Cargo.toml).
    let _ = include_str!("../../../darkmatter/cli/tests/common/context_guard.rs");
    let _ = include_str!("../../../darkmatter/cli/tests/common/source_scan.rs");
    context_guard::assert_guarded("messenger", &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src"), ALLOWLIST);
}
