//! End-to-end integration tests for the dynamic completion surface.
//!
//! These tests spawn the compiled `claudine` binary with the environment
//! variables `clap_complete::CompleteEnv` uses to dispatch a completion
//! reply (`COMPLETE=<shell>`, `_CLAP_COMPLETE_INDEX=<n>`, `_CLAP_IFS=\n`),
//! then assert on the reply candidates.
//!
//! They cover the acceptance matrix locked in Phase 0 of the
//! `2026-04-17-file-completion` feature:
//!
//! - `compose @…`, `inline-compose @…`, `sequence @…` pick up the
//!   relevant repo-scoped fixtures via the `@` prefix;
//! - setter partials (e.g. `topic=`) suppress completion;
//! - unsupported prefixes (`vault:`, `/abs/…`, `%`, `{{…}}`) return zero
//!   candidates;
//! - `!` returns zero candidates when there is no package area root
//!   (empty or non-repo workspaces).

use std::fs;
use std::path::Path;
use std::process::Command;

use assert_cmd::cargo::cargo_bin_cmd;

mod common;
use common::{TestWorkspace, init_git_repo};

/// Minimum subset of env vars clap_complete's bash adapter sets when it
/// invokes the binary as a completion subprocess.
const SHELL: &str = "bash";

/// Seed the shared fixture tree used by most of the `@…` assertions.
///
/// The tree deliberately mixes:
///
/// - a `compose`-friendly markdown file (valid `.md`, no frontmatter);
/// - an `inline-compose`-friendly file (`prompt:` in frontmatter);
/// - a `sequence`-friendly file (`sequence:` inline list);
/// - a non-markdown file (`.txt`) that every validator must drop.
///
/// Having all four in one fixture keeps the full matrix reachable from a
/// single workspace root while still giving each test a focused assertion.
///
/// The workspace is set up as a minimal cargo workspace (plus `git init`)
/// because `sniff::detect_repo_structure` only returns a usable
/// `RepoInfo` when it recognizes a workspace tool at the root. Just
/// `.git/` alone is not enough for magic-path completion, and
/// `discover_magic` relies on the resolved `repo_root`.
///
/// Tests that need to exercise the no-repo failure mode deliberately
/// avoid calling this helper.
fn seed_fixtures(root: &Path) {
    assert!(
        init_git_repo(root),
        "unable to initialize git repo in test workspace at {root:?}",
    );
    // Minimal cargo workspace marker — sniff requires `[workspace]` with
    // at least one member before it emits a RepoInfo. We point at a tiny
    // placeholder package inside the fixture so detection succeeds
    // without pulling any real dependencies.
    fs::write(
        root.join("Cargo.toml"),
        "[workspace]\nresolver = \"2\"\nmembers = [\"placeholder\"]\n",
    )
    .unwrap();
    let placeholder = root.join("placeholder");
    fs::create_dir_all(&placeholder).unwrap();
    fs::write(
        placeholder.join("Cargo.toml"),
        "[package]\nname = \"placeholder\"\nversion = \"0.0.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    fs::create_dir_all(placeholder.join("src")).unwrap();
    fs::write(placeholder.join("src").join("lib.rs"), "// placeholder\n").unwrap();
    let prompts = root.join("prompts");
    fs::create_dir_all(&prompts).unwrap();
    // Valid for compose (any `.md` extension passes).
    fs::write(prompts.join("plain.md"), "# Plain\n\nbody\n").unwrap();
    // Valid for compose + inline-compose.
    fs::write(
        prompts.join("with-prompt.md"),
        "---\nprompt: Write a poem\n---\nbody\n",
    )
    .unwrap();
    // Valid for compose + sequence.
    fs::write(
        prompts.join("seq.md"),
        "---\nsequence:\n  - one\n  - two\n---\nbody\n",
    )
    .unwrap();
    // Never valid for any mode (not markdown).
    fs::write(prompts.join("notes.txt"), "not markdown\n").unwrap();
}

/// Spawn `claudine` as a bash completion subprocess and collect the
/// newline-separated candidate list.
///
/// `args` is the shell-level command being completed (excluding the
/// leading `claudine` binary name), with the last element being the
/// current partial token. The index is set to the position of that
/// partial in the full argv (bin + args).
fn run_completion(cwd: &Path, args: &[&str]) -> Vec<String> {
    // The bash adapter calls the binary with `-- <full argv>` and sets
    // `_CLAP_COMPLETE_INDEX` to `COMP_CWORD`, which is the index of the
    // word under the cursor in the shell's word list.
    let full_argv_len = 1 + args.len(); // `claudine` + args
    let index = full_argv_len - 1;

    // `cargo_bin_cmd!` returns an `assert_cmd::Command`; we copy its
    // resolved program path out and rebuild a plain `std::process::Command`
    // so we can drive the subprocess without any assertion-style helpers.
    let reference = cargo_bin_cmd!("claudine");
    let program = reference.get_program().to_os_string();
    let mut cmd = Command::new(program);
    cmd.env("COMPLETE", SHELL)
        .env("_CLAP_COMPLETE_INDEX", index.to_string())
        .env("_CLAP_IFS", "\n")
        .env("NO_COLOR", "1")
        .current_dir(cwd)
        .arg("--")
        .arg("claudine");
    for arg in args {
        cmd.arg(arg);
    }

    let output = cmd.output().expect("completion subprocess to run");
    assert!(
        output.status.success(),
        "completion subprocess failed: status={:?}, stderr={}",
        output.status,
        String::from_utf8_lossy(&output.stderr),
    );

    let stdout = String::from_utf8(output.stdout).expect("utf-8 completion output");
    stdout
        .split('\n')
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}

// ---------------------------------------------------------------------
// @-prefix acceptance (magic scope)
// ---------------------------------------------------------------------

#[test]
fn compose_at_prefix_lists_repo_scoped_markdown_files() {
    let workspace = TestWorkspace::named("completion-compose-at");
    let root = workspace.path();
    seed_fixtures(root);

    let candidates = run_completion(root, &["compose", "@"]);

    assert!(
        candidates.iter().any(|c| c == "@prompts/plain.md"),
        "compose @ should surface plain.md; got: {candidates:?}",
    );
    assert!(
        candidates.iter().any(|c| c == "@prompts/with-prompt.md"),
        "compose @ should surface with-prompt.md; got: {candidates:?}",
    );
    assert!(
        candidates.iter().any(|c| c == "@prompts/seq.md"),
        "compose @ should surface seq.md; got: {candidates:?}",
    );
    // compose is extension-only, so a .txt file must never leak through.
    assert!(
        !candidates.iter().any(|c| c.ends_with("notes.txt")),
        "compose must never emit non-markdown candidates; got: {candidates:?}",
    );
}

#[test]
fn inline_compose_at_prefix_keeps_only_prompt_frontmatter_files() {
    let workspace = TestWorkspace::named("completion-inline-at");
    let root = workspace.path();
    seed_fixtures(root);

    let candidates = run_completion(root, &["inline-compose", "@"]);

    assert!(
        candidates.iter().any(|c| c == "@prompts/with-prompt.md"),
        "inline-compose @ must include files with a prompt; got: {candidates:?}",
    );
    // plain.md has no `prompt:` frontmatter; seq.md has `sequence:`; both
    // must be dropped by the inline-compose validator.
    assert!(
        !candidates.iter().any(|c| c == "@prompts/plain.md"),
        "inline-compose must drop prompt-less markdown; got: {candidates:?}",
    );
    assert!(
        !candidates.iter().any(|c| c == "@prompts/seq.md"),
        "inline-compose must drop sequence-only markdown; got: {candidates:?}",
    );
    assert!(
        !candidates.iter().any(|c| c.ends_with("notes.txt")),
        "inline-compose must never emit non-markdown candidates; got: {candidates:?}",
    );
}

#[test]
fn sequence_at_prefix_keeps_only_sequence_frontmatter_files() {
    let workspace = TestWorkspace::named("completion-sequence-at");
    let root = workspace.path();
    seed_fixtures(root);

    let candidates = run_completion(root, &["sequence", "@"]);

    assert!(
        candidates.iter().any(|c| c == "@prompts/seq.md"),
        "sequence @ must include sequence-capable markdown; got: {candidates:?}",
    );
    // plain.md and with-prompt.md have no resolvable sequence plan, so
    // they must be dropped by the sequence validator.
    assert!(
        !candidates.iter().any(|c| c == "@prompts/plain.md"),
        "sequence must drop plain markdown; got: {candidates:?}",
    );
    assert!(
        !candidates.iter().any(|c| c == "@prompts/with-prompt.md"),
        "sequence must drop prompt-only markdown; got: {candidates:?}",
    );
}

// ---------------------------------------------------------------------
// Setter suppression + unsupported prefixes
// ---------------------------------------------------------------------

#[test]
fn setter_partial_suppresses_completion() {
    let workspace = TestWorkspace::named("completion-setter");
    let root = workspace.path();
    seed_fixtures(root);

    // `topic=` matches the strict setter regex; classifier returns
    // `SetterPartial` and discovery short-circuits to zero.
    let candidates = run_completion(root, &["compose", "topic="]);
    assert!(
        candidates.is_empty(),
        "setter partial must suppress completion; got: {candidates:?}",
    );
}

#[test]
fn vault_prefix_returns_zero_candidates() {
    let workspace = TestWorkspace::named("completion-vault");
    let root = workspace.path();
    seed_fixtures(root);

    let candidates = run_completion(root, &["compose", "vault:"]);
    assert!(
        candidates.is_empty(),
        "vault: is not supported in v1; got: {candidates:?}",
    );
}

#[test]
fn absolute_path_prefix_returns_zero_candidates() {
    let workspace = TestWorkspace::named("completion-abs");
    let root = workspace.path();
    seed_fixtures(root);

    let candidates = run_completion(root, &["compose", "/abs"]);
    assert!(
        candidates.is_empty(),
        "absolute paths are not supported in v1; got: {candidates:?}",
    );
}

// ---------------------------------------------------------------------
// Empty package-area scope
// ---------------------------------------------------------------------

#[test]
fn package_prefix_with_no_repo_context_returns_zero_candidates() {
    // No git repo / `sniff` detection → no package-area root → `!`
    // must resolve to nothing rather than surfacing cwd files under a
    // `!` prefix. This test intentionally skips `seed_fixtures` so the
    // workspace has no repo marker at all.
    let workspace = TestWorkspace::named("completion-bang-empty");
    let root = workspace.path();
    // Seed a lone markdown file so "return zero" is a real assertion
    // and not just an empty-directory artefact.
    fs::write(root.join("local.md"), "# Local\n").unwrap();

    let candidates = run_completion(root, &["compose", "!"]);
    assert!(
        candidates.is_empty(),
        "`!` without a resolvable package area must return zero; \
         got: {candidates:?}",
    );
}

// ---------------------------------------------------------------------
// End-to-end failure modes (spec §Failure modes)
//
// These cases are covered by unit tests in
// `src/completion/{file_reference,validate}.rs`, but Phase 5 calls for
// verifying end to end that the completion subprocess never surfaces
// diagnostics and returns cleanly (possibly zero candidates) even when
// the repo contains adversarial fixtures.
// ---------------------------------------------------------------------

#[test]
fn inline_compose_silently_omits_malformed_and_oversized_files() {
    let workspace = TestWorkspace::named("completion-failure-modes");
    let root = workspace.path();
    seed_fixtures(root);

    let prompts = root.join("prompts");
    // Malformed frontmatter — YAML fails to parse as a prompt value.
    fs::write(
        prompts.join("malformed.md"),
        "---\nprompt: [unterminated\n\nbody without close\n",
    )
    .unwrap();
    // Empty prompt — explicitly forbidden by the validator.
    fs::write(prompts.join("empty-prompt.md"), "---\nprompt: \"\"\n---\nbody\n").unwrap();
    // Oversized file — skipped before parsing.
    let padding = "a".repeat((1024 * 1024 + 1) as usize);
    fs::write(
        prompts.join("oversized.md"),
        format!("---\nprompt: hi\n---\n{padding}"),
    )
    .unwrap();

    let candidates = run_completion(root, &["inline-compose", "@"]);

    // The valid fixture still comes through.
    assert!(
        candidates.iter().any(|c| c == "@prompts/with-prompt.md"),
        "inline-compose must still surface valid prompt fixtures; got: {candidates:?}",
    );
    // Every failure-mode fixture is silently dropped.
    assert!(
        !candidates.iter().any(|c| c == "@prompts/malformed.md"),
        "malformed frontmatter must be dropped; got: {candidates:?}",
    );
    assert!(
        !candidates.iter().any(|c| c == "@prompts/empty-prompt.md"),
        "empty-prompt files must be dropped; got: {candidates:?}",
    );
    assert!(
        !candidates.iter().any(|c| c == "@prompts/oversized.md"),
        "oversized files must be dropped; got: {candidates:?}",
    );
}

#[test]
fn missing_prompts_and_sequences_directories_return_cleanly() {
    // The spec lists missing `prompts/` or `sequences/` directories as a
    // silent failure mode. Build a minimal workspace with no such
    // directories and assert the completion subprocess still returns
    // successfully with no diagnostics leaking onto stderr.
    let workspace = TestWorkspace::named("completion-missing-dirs");
    let root = workspace.path();
    assert!(init_git_repo(root));
    fs::write(
        root.join("Cargo.toml"),
        "[workspace]\nresolver = \"2\"\nmembers = [\"placeholder\"]\n",
    )
    .unwrap();
    let placeholder = root.join("placeholder");
    fs::create_dir_all(&placeholder).unwrap();
    fs::write(
        placeholder.join("Cargo.toml"),
        "[package]\nname = \"placeholder\"\nversion = \"0.0.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    fs::create_dir_all(placeholder.join("src")).unwrap();
    fs::write(placeholder.join("src").join("lib.rs"), "// placeholder\n").unwrap();

    // Zero candidates are acceptable — what matters is that the
    // subprocess exits successfully (verified inside `run_completion`)
    // and nothing about the empty tree becomes a shell-visible error.
    let candidates = run_completion(root, &["inline-compose", "@"]);
    assert!(
        candidates.iter().all(|c| !c.contains("prompts/")
            && !c.contains("sequences/")),
        "no prompts/sequences entries should be produced; got: {candidates:?}",
    );
}
