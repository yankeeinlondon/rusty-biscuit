//! The library reads the process in one place: `schema/capture.rs`, the
//! raw-path factories' single context capture. No other production code
//! reads the working directory, home, or environment, and the only other
//! context it builds is content's empty one, so pure content parsing,
//! validation, coercion, lookup, and export can never capture. The gates and their scope rule are Darkmatter's
//! shared engine (`darkmatter/cli/tests/common/context_guard.rs`).
//!
//! One departure from the engine's rules: its optional-context gate takes no
//! allowlist, and the library has exactly one optional context, the
//! `SchemaSource` field that is `None` for
//! a raw path until the factory captures. That site is pinned by count here
//! instead, so a second optional context anywhere still fails.

#[path = "../../../../darkmatter/cli/tests/common/context_guard.rs"]
mod context_guard;
#[allow(clippy::duplicate_mod)]
#[path = "../../../../darkmatter/cli/tests/common/source_scan.rs"]
mod source_scan;

use context_guard::{Allowance, Gate};

const fn allow(gate: Gate, path: &'static str, identifier: &'static str, count: usize, reason: &'static str) -> Allowance {
    Allowance { gate, path, identifier, count, reason }
}

const CAPTURE: &str = "the raw-path factories' one capture (`request_context`)";

const ALLOWLIST: &[Allowance] = &[
    allow(Gate::Construction, "schema/capture.rs", "FileResolutionContext::from_snapshot", 1, CAPTURE),
    allow(
        Gate::Construction,
        "schema/layer.rs",
        "FileResolutionContext::from_snapshot",
        1,
        "`content_context`: an empty snapshot (no directory, home, or environment) for a schema with no file; reads no process state",
    ),
    allow(Gate::AmbientState, "schema/capture.rs", "std::env::current_dir", 1, CAPTURE),
    allow(Gate::AmbientState, "schema/capture.rs", "biscuit_file::home_dir", 1, CAPTURE),
    allow(Gate::AmbientState, "schema/capture.rs", "biscuit_file::capture_env", 1, CAPTURE),
    allow(
        Gate::AmbientState,
        "validate.rs",
        "std::env::var",
        1,
        "`DARKMATTER_SCHEMA_CACHE_SIZE` sizes the shared validator cache (tuning; feeds no resolution)",
    ),
];

/// The one optional context: `SchemaSource::context`, `None` for a raw path.
const OPTIONAL_CONTEXT: (&str, usize) = ("schema/source.rs", 1);

#[test]
fn only_the_capture_module_builds_a_context_or_reads_the_process() {
    // Spelled for CI's test-input index, which does not count the `#[path]`
    // includes above (`source-inputs` in Cargo.toml).
    let _ = include_str!("../../../../darkmatter/cli/tests/common/context_guard.rs");
    let _ = include_str!("../../../../darkmatter/cli/tests/common/source_scan.rs");
    let sources = source_scan::production_sources(&biscuit_test_harness::manifest_dir!().join("src"));
    assert!(sources.len() > 50, "the scan read only {} source file(s)", sources.len());
    assert!(sources.contains_key("schema/capture.rs"), "the capture module was not scanned");

    let (optional, sites): (Vec<_>, Vec<_>) =
        context_guard::scan(&sources).into_iter().partition(|site| site.gate == Gate::OptionalContext);
    let optional_paths: Vec<&str> = optional.iter().map(|site| site.path.as_str()).collect();
    assert_eq!(
        optional_paths,
        vec![OPTIONAL_CONTEXT.0; OPTIONAL_CONTEXT.1],
        "an optional context outside `SchemaSource`: {optional:#?}"
    );
    let problems = context_guard::check(&sites, ALLOWLIST);
    assert!(problems.is_empty(), "simplified-schema context guard:\n  {}", problems.join("\n  "));
}

/// The gates fire on the library's own spellings: a capture outside the
/// sanctioned module is an unlisted site, not a pass.
#[test]
fn a_capture_outside_the_capture_module_is_detected() {
    let mut sources = std::collections::BTreeMap::new();
    sources.insert(
        "schema/view.rs".to_string(),
        "fn f() { let cwd = std::env::current_dir(); let c = FileResolutionContext::from_snapshot(&cwd, None, env); }"
            .to_string(),
    );
    let problems = context_guard::check(&context_guard::scan(&sources), &[]);

    assert_eq!(problems.len(), 2, "{problems:#?}");
    assert!(problems.iter().all(|problem| problem.contains("unlisted") && problem.contains("schema/view.rs")), "{problems:#?}");
}
