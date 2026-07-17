//! Executable boundary guard for the Darkmatter ↔ Claudine error boundary.
//!
//! `integrated-design.md` §10 requires that typed `MarkdownError`/`BlockError`
//! (and the Claudine error surfaces wrapping them) cross into Claudine by
//! `#[from]`/`#[source]`, never flattened via `.to_string()`. Two named sites
//! used to flatten a typed error into a `String`:
//!
//! 1. `composition/closure.rs` flattened the typed `ClaudineError` from
//!    `atomic_write` into `CompositionError::AtomicWriteFailed(String)`.
//! 2. `composition/lifecycle/control.rs::resolve_proxy_target` flattened the
//!    typed `HarnessError` from `resolve_harness_path` via `map_err(|e|
//!    e.to_string())`.
//!
//! Both were converted to carry the typed source. These string assertions are
//! the executable guardrail that those two specific conversions cannot silently
//! regress.
//!
//! ## Notes
//!
//! Scope is deliberately limited to these two named sites. The feature spec
//! permits some legacy `String` error variants during migration, and other
//! sites (e.g. `lifecycle/executor.rs`) still legitimately use
//! `map_err(|e| e.to_string())` and are out of scope for this finding. A
//! repo-wide ban would false-positive on those, so the guard matches the exact
//! converted constructions only.

use std::path::Path;

/// `CARGO_MANIFEST_DIR` for a lib integration test is the lib crate root
/// (`claudine/lib`), so source paths resolve regardless of the invoking CWD.
fn read_source(relative: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(relative);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()))
}

#[test]
fn closure_does_not_flatten_atomic_write_error() {
    let src = read_source("src/composition/closure.rs");

    // The typed struct form is `AtomicWriteFailed { path, source }`; the old
    // tuple form `AtomicWriteFailed(...)` flattened the typed `ClaudineError`.
    assert!(
        !src.contains("AtomicWriteFailed("),
        "closure.rs reintroduced the tuple-style `AtomicWriteFailed(` construction, \
         which flattens the typed atomic-write error (integrated-design §10)"
    );
    assert!(
        !src.contains("CompositionError::AtomicWriteFailed(e.to_string())"),
        "closure.rs reintroduced flattening the atomic-write error to a string \
         (integrated-design §10)"
    );
}

#[test]
fn resolve_proxy_target_does_not_flatten_harness_error() {
    let src = read_source("src/composition/lifecycle/control.rs");

    // `resolve_proxy_target` must propagate the typed `HarnessError` from
    // `resolve_harness_path` with `?`, not flatten it with `.to_string()`.
    assert!(
        !src.contains("resolve_harness_path(target, &ctx).map_err(|e| e.to_string())"),
        "lifecycle/control.rs reintroduced flattening the `resolve_harness_path` \
         error to a string instead of propagating the typed HarnessError \
         (integrated-design §10)"
    );
}

/// `CARGO_MANIFEST_DIR` for the lib crate is `claudine/lib`; the CLI proxy
/// routes live one directory over, under `claudine/cli`.
fn read_cli_source(relative: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../cli")
        .join(relative);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()))
}

/// Every proxy route must funnel through the shared, existence-checking
/// `resolve_proxy_target` (D5/D6) and wrap its typed `HarnessError` in
/// `CompositionError::InvalidFileReference` — never resolve the reference
/// directly or flatten the failure into an `eyre!` string. The three routes
/// are the two harness-loop dispatch paths and the setup-phase pipeline.
#[test]
fn every_proxy_route_uses_the_shared_resolver_and_typed_error() {
    let routes = [
        "src/commands/wrap/harness_orch/loop_control/proxy.rs",
        "src/commands/wrap/harness_orch/loop_control/control_dispatch.rs",
        "src/commands/wrap/composition/pipeline.rs",
    ];
    for route in routes {
        let src = read_cli_source(route);
        assert!(
            src.contains("resolve_proxy_target"),
            "{route} no longer resolves proxy targets through the shared \
             `resolve_proxy_target` (D5)"
        );
        assert!(
            src.contains("InvalidFileReference"),
            "{route} no longer wraps the proxy resolution failure in the typed \
             `CompositionError::InvalidFileReference` (integrated-design §10)"
        );
    }
}

/// The terminal-control dispatch route once called `resolve_harness_path`
/// directly, bypassing the existence probe and swapping `source_path` to a path
/// it never checked (the live latent bug this feature fixed). Guard that it
/// stays converged on `resolve_proxy_target`.
#[test]
fn control_dispatch_does_not_bypass_the_existence_check() {
    let src = read_cli_source("src/commands/wrap/harness_orch/loop_control/control_dispatch.rs");
    assert!(
        !src.contains("resolve_harness_path(&target"),
        "control_dispatch.rs reintroduced the direct `resolve_harness_path` call \
         that bypasses `resolve_proxy_target`'s existence check"
    );
}
