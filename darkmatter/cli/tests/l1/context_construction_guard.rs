//! Acceptance Criteria 2, 3, and 5 of `2026-09-30-file-refs-use-magic` for
//! `md`: the binary reads its process once, in `main`, and its remaining
//! environment reads configure output, never file resolution, `ctx.*`, or
//! `env.*`. This crate owns the shared engine
//! (`tests/common/context_guard.rs`), so the engine's self-tests live here.

#[path = "../common/context_guard.rs"]
mod context_guard;

use context_guard::{Allowance, Gate};

const fn allow(gate: Gate, path: &'static str, identifier: &'static str, count: usize, reason: &'static str) -> Allowance {
    Allowance { gate, path, identifier, count, reason }
}

const ALLOWLIST: &[Allowance] = &[
    allow(Gate::Construction, "main.rs", "RequestSnapshot::from_process", 1, "the `md` binary's one process read (R8)"),
    allow(Gate::AmbientState, "main.rs", "std::env::var", 1, "`RUST_LOG` tracing filter"),
    allow(Gate::AmbientState, "artifact.rs", "std::env::var", 1, "`MD_DRY_RUN` test hook that skips launching a viewer"),
    allow(
        Gate::AmbientState,
        "commands/compose.rs",
        "std::env::var",
        1,
        "`DARKMATTER_NO_BASELINE_SCHEMA` switches the embedded baseline schema off (a feature switch, not a path)",
    ),
    allow(Gate::AmbientState, "commands/hash.rs", "std::env::var", 2, "`HASH_PROPERTY` / `HASH_IGNORE_PROPERTIES` hash configuration"),
    allow(Gate::AmbientState, "render.rs", "std::env::var", 1, "`TERMINAL_IMAGES` image rendering mode (rendering)"),
];

#[test]
fn production_source_builds_contexts_only_through_the_builder() {
    context_guard::assert_guarded("darkmatter-cli", &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src"), ALLOWLIST);
}

/// Every gate rejects its seeded violation, test code, comments, and literals
/// are out of scope, and each kind of stale allowlist entry is rejected, so
/// the guards cannot pass vacuously.
#[test]
fn the_engine_rejects_planted_violations_and_stale_entries() {
    let root = tempfile::tempdir().unwrap();
    let write = |relative: &str, text: &str| {
        let path = root.path().join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    };
    write(
        "lib.rs",
        r#"// FileResolutionContext::new in a comment is not a construction.
fn construct() {
    let a = biscuit_file::FileResolutionContext :: new(dir);
    let b = FileResolutionContext::from_snapshot(dir, home, env);
    let s = RequestSnapshot::from_process();
}
fn optional(a: Option< &'a mut FileResolutionContext >, b: Option<biscuit_file::FileResolutionContext>) {}
use std::env::{self, var_os, vars};
fn ambient() {
    std::env::var("X");
    env::current_dir();
    dirs::home_dir();
    home::home_dir();
    biscuit_file::home_dir();
    capture_env();
    reference.resolve();
    reference.resolve_from(dir);
    reference.resolve_target();
    PortablePath::from_reference(reference).file_reference();
}
fn allowed() {
    context.home_dir();
    reference.resolve_in_context(&context);
    other.resolve(&input);
    let s = "std::env::var";
    PortablePath::from_path(path).with_ctx(&context).file_reference();
    let derived = context.for_cwd(dir);
}
fn home_dir() -> Option<PathBuf> { None }
#[cfg(test)]
mod tests {
    fn t() { std::env::var("Y"); }
}
#[cfg(test)]
mod hidden;
"#,
    );
    write("hidden.rs", "fn z() { FileResolutionContext::new(dir); }\n");
    write("bin.rs", "fn main() { std::env::var(\"Z\"); }\n");

    let allowlist = [
        allow(Gate::AmbientState, "lib.rs", "std::env::var", 1, "matches"),
        allow(Gate::AmbientState, "lib.rs", "std::env::var", 1, "duplicate"),
        allow(Gate::AmbientState, "lib.rs", "dirs::home_dir", 2, "moved count"),
        allow(Gate::AmbientState, "gone.rs", "std::env::vars", 1, "unused"),
        allow(Gate::OptionalContext, "lib.rs", "Option<FileResolutionContext>", 1, "not allowed"),
        allow(Gate::Construction, "lib.rs", "RequestSnapshot::from_process", 1, " "),
    ];
    let mut problems = context_guard::problems(root.path(), &allowlist);
    problems.sort();
    let unlisted = |gate: &str, variant: &str, identifier: &str, path: &str, lines: &str, count: usize| {
        format!(
            "unlisted {gate} site `{identifier}` in {path} at lines {lines}; remove it, or (if the gate allows) add \
             Allowance {{ gate: Gate::{variant}, path: \"{path}\", identifier: \"{identifier}\", count: {count}, reason: \"..\" }}"
        )
    };
    let mut expected = vec![
        "allowlist entry for construction `RequestSnapshot::from_process` in lib.rs needs a reason and a non-zero count".to_string(),
        "allowlist entry for optional-context `Option<FileResolutionContext>` in lib.rs: the optional-context gate takes no allowlist"
            .to_string(),
        "duplicate allowlist entry: ambient-state `std::env::var` in lib.rs".to_string(),
        "moved count for ambient-state `dirs::home_dir` in lib.rs: allowlisted 2, found 1 at lines [12]".to_string(),
        "unused allowlist entry: ambient-state `std::env::vars` in gone.rs no longer occurs".to_string(),
        unlisted("construction", "Construction", "FileResolutionContext::new", "lib.rs", "[3]", 1),
        unlisted("construction", "Construction", "FileResolutionContext::from_snapshot", "lib.rs", "[4]", 1),
        unlisted("optional-context", "OptionalContext", "Option<&FileResolutionContext>", "lib.rs", "[7]", 1),
        unlisted("ambient-state", "AmbientState", "std::env::var_os", "lib.rs", "[8]", 1),
        unlisted("ambient-state", "AmbientState", "std::env::vars", "lib.rs", "[8]", 1),
        unlisted("ambient-state", "AmbientState", "std::env::current_dir", "lib.rs", "[11]", 1),
        unlisted("ambient-state", "AmbientState", "home::home_dir", "lib.rs", "[13]", 1),
        unlisted("ambient-state", "AmbientState", "biscuit_file::home_dir", "lib.rs", "[14]", 1),
        unlisted("ambient-state", "AmbientState", "capture_env()", "lib.rs", "[15]", 1),
        unlisted("ambient-state", "AmbientState", ".resolve()", "lib.rs", "[16]", 1),
        unlisted("ambient-state", "AmbientState", ".resolve_from(..)", "lib.rs", "[17]", 1),
        unlisted("ambient-state", "AmbientState", ".resolve_target()", "lib.rs", "[18]", 1),
        unlisted("ambient-state", "AmbientState", context_guard::PORTABLE_WITHOUT_CTX, "lib.rs", "[19]", 1),
        unlisted("ambient-state", "AmbientState", "std::env::var", "bin.rs", "[1]", 1),
    ];
    expected.sort();
    assert_eq!(problems, expected);
}
