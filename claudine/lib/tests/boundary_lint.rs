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
//! It also guards the **file-reference grammar** boundary: the file-resolution
//! feature (spec D1/D3, AC1/AC6) makes `biscuit_file::FileReference` the single
//! syntax authority across Claudine-executed Darkmatter compose surfaces. The
//! `darkmatter_*` tests below broaden coverage from the three CLI proxy routes
//! to the Darkmatter modules Claudine composition drives, so a reintroduced
//! private prefix classifier or source-first `join` fallback fails loudly.
//!
//! ## Notes
//!
//! Scope is deliberately limited to the named sites and the exact removed
//! constructions. The feature spec permits some legacy `String` error variants
//! during migration, and other sites (e.g. `lifecycle/executor.rs`) still
//! legitimately use `map_err(|e| e.to_string())` and are out of scope for this
//! finding. A repo-wide ban would false-positive on those, so the guard matches
//! the exact converted constructions only.

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

/// Provider-attempt routes may only construct typed handoff requests; the
/// coordinator commit is the sole resolution and typed-failure boundary.
///
/// This is stronger than requiring every route to call the shared resolver:
/// that older shape still allowed several callers to resolve and mutate active
/// document identity independently. The coordinator now owns the one atomic
/// resolve/check/commit operation, while the command pipeline may only invoke
/// that operation and route its concrete failure.
#[test]
fn every_proxy_route_uses_the_shared_resolver_and_typed_error() {
    let attempt_routes = [
        "src/commands/wrap/harness_orch/loop_control/proxy.rs",
        "src/commands/wrap/harness_orch/loop_control/control_dispatch.rs",
    ];
    for route in attempt_routes {
        let src = read_cli_source(route);
        assert!(
            src.contains("EvaluatedProxyRequest::new"),
            "{route} no longer produces the typed request consumed by the coordinator"
        );
        assert!(
            !src.contains("resolve_proxy_target"),
            "{route} bypasses coordinator ownership by resolving a handoff in the attempt harness"
        );
    }

    let commit = read_source("src/composition/coordinator/commit.rs");
    assert!(
        commit.contains("resolve_proxy_target(&target, &source_path, repo_root)"),
        "the coordinator commit no longer owns the shared existence-checking resolver"
    );
    assert!(
        commit.contains("ProxyCommitError::Resolution"),
        "the coordinator commit no longer preserves resolution failures as a typed source"
    );

    let pipeline = read_cli_source("src/commands/wrap/composition/pipeline.rs");
    assert!(
        pipeline.contains("commit_proxy("),
        "the composition coordinator no longer enters the atomic handoff commit"
    );
    assert!(
        !pipeline.contains("resolve_proxy_target"),
        "the composition pipeline resolves outside the coordinator commit"
    );
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

/// `CARGO_MANIFEST_DIR` for the lib crate is `claudine/lib`; the
/// Claudine-executed Darkmatter compose surfaces live two directories over,
/// under `darkmatter/lib`.
fn read_darkmatter_source(relative: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../darkmatter/lib")
        .join(relative);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()))
}

/// Every Claudine-executed Darkmatter compose surface must parse document
/// references through `biscuit_file::FileReference` — the single grammar
/// authority (spec D1, AC1/AC6) — rather than a bespoke prefix classifier. This
/// broadens the proxy-route boundary above from the three CLI routes to the
/// Darkmatter modules Claudine composition actually drives, so a new private
/// grammar cannot land silently on one of them.
#[test]
fn darkmatter_compose_surfaces_route_through_file_reference() {
    for module in [
        "src/markdown/compose/expression/functions/mod.rs",
        "src/markdown/compose/expression/resolve_ctx.rs",
        "src/markdown/compose/link_resolve.rs",
        "src/markdown/compose/transclusion/resolver.rs",
        "src/markdown/schemas/resolve.rs",
    ] {
        let src = read_darkmatter_source(module);
        assert!(
            src.contains("FileReference"),
            "{module} no longer routes references through the shared \
             `FileReference` grammar authority (spec D1)"
        );
    }
}

/// Path-shape expression functions must take a missing target's shape from the
/// shared candidate plan, never a re-parsed prefix branch plus a source-first
/// `ctx.base_dir.join` (spec Current Drift #5, D3). Guards the exact private
/// fallback this finding removed.
#[test]
fn darkmatter_path_shape_functions_do_not_source_join_a_missing_reference() {
    let src = read_darkmatter_source("src/markdown/compose/expression/functions/mod.rs");
    assert!(
        src.contains("resolve_document_file_ref_shape"),
        "expression functions no longer shape missing paths through the shared \
         candidate plan (spec D3)"
    );
    for removed in ["ctx.base_dir.join(rest)", "ctx.base_dir.join(path)"] {
        assert!(
            !src.contains(removed),
            "expression functions reintroduced the private source-first path \
             fallback `{removed}` (spec Current Drift #5)"
        );
    }
}

/// Local Markdown link resolution must absolutize a missing reference through
/// the shared candidate plan (source-first for implicit), never a
/// source-first `dir.join(raw)` that bypasses classification (spec Current
/// Drift #8, D2). Guards the exact fallback this finding removed.
#[test]
fn darkmatter_link_resolve_does_not_source_join_after_a_miss() {
    let src = read_darkmatter_source("src/markdown/compose/link_resolve.rs");
    assert!(
        src.contains("candidate_plan"),
        "link_resolve no longer absolutizes a missing reference through the \
         shared candidate plan (spec Current Drift #8)"
    );
    assert!(
        !src.contains("let joined = dir.join(raw)"),
        "link_resolve reintroduced the source-first `dir.join(raw)` fallback \
         that bypasses shared classification (spec Current Drift #8)"
    );
}
