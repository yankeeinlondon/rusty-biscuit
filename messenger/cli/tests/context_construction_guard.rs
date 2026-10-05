//! Acceptance Criteria 2, 3, and 5 of `2026-09-30-file-refs-use-magic` for
//! the `messenger` binary: it reads its process once, in `main`, and threads
//! that snapshot to the research commands. Its remaining reads locate its own
//! configuration and secrets, never a document reference, `ctx.*`, or
//! `env.*`. The gates are documented in
//! `darkmatter/cli/tests/common/context_guard.rs`.

#[path = "../../../darkmatter/cli/tests/common/context_guard.rs"]
mod context_guard;

use context_guard::{Allowance, Gate};

const fn allow(gate: Gate, path: &'static str, identifier: &'static str, count: usize, reason: &'static str) -> Allowance {
    Allowance { gate, path, identifier, count, reason }
}

const ALLOWLIST: &[Allowance] = &[
    allow(Gate::Construction, "main.rs", "RequestSnapshot::from_process", 1, "the `messenger` binary's one process read (R8)"),
    allow(Gate::AmbientState, "main.rs", "std::env::var", 2, "`RUST_LOG` tracing filter, and a route secret named by its configured variable"),
    allow(Gate::AmbientState, "config.rs", "dirs::home_dir", 1, "messenger's own configuration file, `~/.messenger.json`"),
    allow(Gate::AmbientState, "config.rs", "std::env::var", 1, "preferred desktop notification helpers"),
    allow(Gate::AmbientState, "desktop_setup.rs", "std::env::var_os", 1, "`%APPDATA%` for Windows notification shortcut setup"),
    allow(Gate::AmbientState, "receipt_store.rs", "dirs::home_dir", 1, "messenger's own receipt store, `~/.messenger/receipts`"),
];

#[test]
fn production_source_builds_contexts_only_through_the_builder() {
    // Spelled for CI's test-input index, which does not count the `#[path]`
    // include above (`source-inputs` in Cargo.toml).
    let _ = include_str!("../../../darkmatter/cli/tests/common/context_guard.rs");
    let _ = include_str!("../../../darkmatter/cli/tests/common/source_scan.rs");
    context_guard::assert_guarded("messenger-cli", &biscuit_test_harness::manifest_dir!().join("src"), ALLOWLIST);
}
