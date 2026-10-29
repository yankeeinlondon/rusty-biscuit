//! Acceptance Criteria 2, 3, and 5 of `2026-09-30-file-refs-use-magic` for
//! the `darkmatter` library: contexts are built only by the builder, no
//! context is optional, and the process is read only by
//! `RequestSnapshot::from_process` and the reads listed below, none of which
//! feeds file resolution, `ctx.*`, or `env.*`. The gates and their scope rule
//! are documented in `cli/tests/common/context_guard.rs`.

#[path = "../../../cli/tests/common/context_guard.rs"]
mod context_guard;

use context_guard::{Allowance, Gate};

const fn allow(gate: Gate, path: &'static str, identifier: &'static str, count: usize, reason: &'static str) -> Allowance {
    Allowance { gate, path, identifier, count, reason }
}

const ALLOWLIST: &[Allowance] = &[
    allow(
        Gate::Construction,
        "markdown/compose/context/request.rs",
        "FileResolutionContext::from_snapshot",
        1,
        "the builder, `build_resolution_context_with_catalog`",
    ),
    // `RequestSnapshot::from_process`, the only process reader (R2).
    allow(Gate::AmbientState, "markdown/compose/context/request.rs", "std::env::current_dir", 1, "RequestSnapshot::from_process"),
    allow(Gate::AmbientState, "markdown/compose/context/request.rs", "biscuit_file::home_dir", 1, "RequestSnapshot::from_process"),
    allow(Gate::AmbientState, "markdown/compose/context/request.rs", "biscuit_file::capture_env", 1, "RequestSnapshot::from_process"),
    allow(Gate::AmbientState, "editor/mod.rs", "std::env::var", 2, "`$EDITOR` / `$VISUAL` choose the program `md` opens a file in"),
    allow(Gate::AmbientState, "markdown/compose/remote.rs", "std::env::var", 1, "`DARKMATTER_REMOTE_CONCURRENCY` fetch concurrency limit (tuning)"),
    allow(
        Gate::AmbientState,
        "markdown/compose/shell_expansion/alias.rs",
        "std::env::var",
        1,
        "`$SHELL` for the public `resolve_alias` helper; composition reads the request's `SHELL` instead (shell execution, not resolution)",
    ),
    allow(
        Gate::AmbientState,
        "markdown/highlighting/themes.rs",
        "std::env::var",
        6,
        "`THEME`, `CODE_THEME`, `NO_COLOR`, `COLORFGBG` select a highlighting theme (rendering)",
    ),
    allow(
        Gate::AmbientState,
        "markdown/render_tree/code_renderer.rs",
        "std::env::var",
        2,
        "`CODE_THEME` / `THEME` select a code-block theme (rendering)",
    ),
    allow(Gate::AmbientState, "render/metadata_codec.rs", "std::env::var", 1, "image metadata policy variable (rendering)"),
    allow(
        Gate::AmbientState,
        "markdown/schemas/validate.rs",
        "std::env::var",
        1,
        "`DARKMATTER_SCHEMA_CACHE_SIZE` validator cache capacity (tuning)",
    ),
    allow(
        Gate::AmbientState,
        "markdown/output/terminal.rs",
        "std::env::current_dir",
        1,
        "terminal image base when `ImageRenderer::new` is given none (rendering after composition; no production caller passes none)",
    ),
    allow(
        Gate::AmbientState,
        "style/bespoke.rs",
        "std::env::current_dir",
        1,
        "`style.page.stylesheet` of a document with no path reads beside the launch directory (rendering after composition, not a file reference)",
    ),
];

#[test]
fn production_source_builds_contexts_only_through_the_builder() {
    // Spelled for CI's test-input index, which does not count the `#[path]`
    // include above (`source-inputs` in Cargo.toml).
    let _ = include_str!("../../../cli/tests/common/context_guard.rs");
    let _ = include_str!("../../../cli/tests/common/source_scan.rs");
    context_guard::assert_guarded("darkmatter", &biscuit_test_harness::manifest_dir!().join("src"), ALLOWLIST);
}

/// The collection sites compose inside the request's own context: none builds
/// a resolution context, takes an optional one, or reads the process, and the
/// allowlist above names no collection file.
#[test]
fn collection_sources_need_no_context_allowance() {
    assert!(
        ALLOWLIST.iter().all(|allowance| !allowance.path.contains("collection_blocks")),
        "a collection file was allowlisted; route it through the request context instead"
    );
    let src = biscuit_test_harness::manifest_dir!().join("src");
    let mut sources = std::collections::BTreeMap::new();
    for entry in std::fs::read_dir(src.join("markdown/compose/collection_blocks")).expect("collection sources") {
        let path = entry.expect("directory entry").path();
        if path.extension().is_some_and(|extension| extension == "rs") {
            let name = path.file_name().expect("file name").to_string_lossy().into_owned();
            sources.insert(format!("markdown/compose/collection_blocks/{name}"), std::fs::read_to_string(&path).expect("read source"));
        }
    }
    sources.insert(
        "markdown/compose/inline/collection_blocks.rs".to_string(),
        std::fs::read_to_string(src.join("markdown/compose/inline/collection_blocks.rs")).expect("inline collection source"),
    );
    assert!(sources.len() >= 8, "the scan read only {} collection source(s)", sources.len());
    let problems = context_guard::check(&context_guard::scan(&sources), &[]);
    assert!(problems.is_empty(), "{problems:#?}");
}
