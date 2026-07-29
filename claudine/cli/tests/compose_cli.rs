//! Integration tests for `claudine compose`.
//!
//! Phase 1 of the `2026-06-03-always-harness` feature. Pins current observable
//! behavior so divergence introduced by later phases fails loudly.

use std::fs;
use tempfile::tempdir;
mod common;
use common::{augmented_path, init_git_repo, strip_ansi, write, write_executable};

// ============================================================================
// Phase 1: convergence between non-harness and harness-enabled direct compose
// ============================================================================

/// Build a fake `opencode` binary that emits a short structured stream and
/// exits successfully.
fn stage_opencode_body_writer(path_dir: &std::path::Path) {
    write_executable(
        &path_dir.join("opencode"),
        r##"#!/bin/sh
if [ "$1" = "models" ]; then
  printf '%s\n' '["test-model"]'
  exit 0
fi
printf '%s\n' '{"type":"init","session_id":"conv","model":"test-model"}'
printf '%s\n' '{"type":"step_start","sessionID":"conv"}'
printf '%s\n' '{"type":"text","text":"# Generated Plan\n\nThis is the generated body."}'
printf '%s\n' '{"type":"finish","sessionID":"conv"}'
exit 0
"##,
    );
}

#[cfg(unix)]
#[test]
fn compose_direct_produces_expected_stdout_body() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();

    stage_opencode_body_writer(&path_dir);

    let bare_file = workspace.path().join("bare.md");
    fs::write(
        &bare_file,
        "---\n---\nCompose this document.\n",
    )
    .unwrap();

    let output = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .env("OPENCODE_MODEL", "test-model")
        .current_dir(workspace.path())
        .args(["compose", "--opencode", bare_file.to_str().unwrap()])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    let expected = "# Generated Plan\n\nThis is the generated body.\n";
    assert_eq!(
        stdout, expected,
        "direct compose stdout body mismatch; got: {stdout:?}"
    );
}

// ============================================================================
// Whole-value frontmatter interpolation regression
// (2026-06-19-invalid-parsing-state)
// ============================================================================

/// A frontmatter value that is exactly one malformed `{{ … }}` interpolation is
/// executable state, not text. Composition must abort during preparation with a
/// parse error that names the offending key — and must never leak the raw
/// template through `--dry-run` as a successful effective-frontmatter result.
///
/// `{{ dirname(review) + '/spec.md') }}` carries an unbalanced paren: the exact
/// malformed shape shipped in `prompts/implement-suggestions.md` that motivated
/// this fix.
#[cfg(unix)]
#[test]
fn compose_dry_run_malformed_whole_value_spec_path_aborts_without_leaking() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    let count_path = workspace.path().join("call-count.txt");

    let md_file = workspace.path().join("plan.md");
    fs::write(
        &md_file,
        "---\n\
         review: features/2026-06-19-review-findings/review-2.md\n\
         spec_path: \"{{ dirname(review) + '/spec.md') }}\"\n\
         ---\n\
         Implement against {{ spec_path }}.\n",
    )
    .unwrap();

    // Provider stub records every invocation so we can prove it was never
    // launched — preparation must abort before the dry-run provider seam.
    write_executable(
        &path_dir.join("goose"),
        &format!(
            "#!/bin/sh\necho touched >> {count}\nexit 0\n",
            count = count_path.display()
        ),
    );

    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .current_dir(workspace.path())
        .args([
            "compose",
            "--goose",
            md_file.to_str().unwrap(),
            "--dry-run",
        ])
        .assert()
        .failure();

    let output = assert.get_output();
    let plain_err = strip_ansi(&String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        plain_err.contains("spec_path"),
        "error must name the offending key; stderr:\n{plain_err}"
    );
    assert!(
        plain_err.contains("parse error"),
        "error must report the typed interpolation parse failure; stderr:\n{plain_err}"
    );
    // The original leak: the raw malformed template surfacing as a successful
    // effective-frontmatter result on stdout. It must be gone.
    assert!(
        !stdout.contains("dirname(review)"),
        "raw malformed template leaked to stdout; stdout:\n{stdout}"
    );
    assert!(
        !count_path.exists(),
        "no provider session should have been launched; stub recorded a call"
    );
}

// ============================================================================
// Cross-surface repository-first parity (2026-07-13-file-resolution, AC6)
// ============================================================================

/// A Darkmatter `::file` transclusion run by Claudine resolves a bare implicit
/// reference **repository-first** on a real collision.
///
/// This is the cross-surface half of the file-resolution feature (AC6): the
/// lifecycle-proxy surface is proven end-to-end by the L2 file-resolution
/// capture; this proves the *transclusion* surface obeys the identical
/// repository-first contract when Claudine composes the document.
///
/// The fixture is a genuine collision — the transcluded basename exists at
/// **both** the repository root and the source document's directory — so the
/// outcome discriminates precedence rather than mere existence. A CLI spawn (not
/// a terminal capture) is the right level: the assertion is on the *content*
/// delivered to the provider, which needs the real binary + filesystem but no
/// TTY. The provider stub records everything it receives (argv, any file-valued
/// argument, and stdin), so the composed prompt is captured regardless of the
/// wrapper's delivery mechanism.
#[cfg(unix)]
#[test]
fn compose_transclusion_resolves_repository_first_on_collision() {
    let workspace = tempdir().unwrap();
    let root = workspace.path();
    assert!(
        init_git_repo(root),
        "repository-first transclusion needs a real git worktree root"
    );

    let path_dir = root.join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    let capture = root.join("delivered-prompt.txt");
    // Capture argv, file-valued args (the wrapper may pass the prompt as a
    // tmpfile), and stdin — the composed prompt lands in one of them.
    write_executable(
        &path_dir.join("claude"),
        r#"#!/bin/sh
{
  for a in "$@"; do
    if [ -f "$a" ]; then cat "$a"; else printf '%s\n' "$a"; fi
  done
  cat
} >> "$CLAUDINE_PROMPT_CAPTURE" 2>/dev/null
exit 0
"#,
    );

    // Genuine collision: same basename at the repository root and beside the
    // source document. Repository-first must transclude the root copy.
    write(&root.join("shared.md"), "REPO_ROOT_TRANSCLUSION_MARKER\n");
    write(
        &root.join("prompts/shared.md"),
        "SOURCE_LOCAL_TRANSCLUSION_MARKER\n",
    );
    let doc = root.join("prompts/doc.md");
    write(&doc, "---\n---\n::file shared.md\n");

    assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", root)
        .env("PATH", augmented_path(&path_dir))
        .env("CLAUDINE_PROMPT_CAPTURE", &capture)
        .current_dir(root)
        .args(["compose", "--claude", doc.to_str().unwrap()])
        .assert()
        .success();

    let delivered = fs::read_to_string(&capture).unwrap_or_default();
    assert!(
        delivered.contains("REPO_ROOT_TRANSCLUSION_MARKER"),
        "the bare `::file shared.md` transclusion must resolve repository-first \
         to <repo>/shared.md.\ndelivered prompt:\n{delivered}"
    );
    assert!(
        !delivered.contains("SOURCE_LOCAL_TRANSCLUSION_MARKER"),
        "repository-first must win the collision — the source-local copy must \
         not be transcluded.\ndelivered prompt:\n{delivered}"
    );
}

// ============================================================================
// Cross-repository source re-anchoring (Finding 1 — D2/D10/AC12)
// ============================================================================

/// A nested bare `::file` transclusion authored inside a top-level document from
/// repository A must resolve against repository A even when the binary is
/// launched from an unrelated repository B.
///
/// Prior to the two-phase re-anchor (provisional launch-time context →
/// definitive source-anchored context), repository discovery was driven by
/// the launch CWD, so launching from B hijacked every nested reference in A.
/// This test pins the post-fix behavior: the source document's repository
/// wins regardless of where the user invoked `claudine compose` from.
#[cfg(unix)]
#[test]
fn compose_transclusion_uses_source_doc_repository_not_launch_cwd() {
    let workspace = tempdir().unwrap();

    // --- Primary repo: source doc + correct transclusion target ------------
    let repo_root = workspace.path().join("repo");
    fs::create_dir_all(&repo_root).unwrap();
    assert!(
        init_git_repo(&repo_root),
        "source-doc re-anchoring needs a real git worktree root for repo_root"
    );
    // The transclusion target lives at the primary repo's root.
    write(
        &repo_root.join("snippet.md"),
        "PRIMARY_REPO_TRANSCLUSION_MARKER\n",
    );
    write(
        &repo_root.join("nested.md"),
        "::file snippet.md\n",
    );

    // --- Unrelated repo: decoy transclusion target --------------------------
    let decoy_root = workspace.path().join("decoy");
    fs::create_dir_all(&decoy_root).unwrap();
    // `git init` on the decoy too: with no git root, biscuit-file's ambient
    // gix walk could find repo_root by walking up, so we give decoy its own
    // worktree root to make the test deterministic when the launch CWD lives
    // inside it.
    assert!(
        init_git_repo(&decoy_root),
        "decoy launch repo needs a real git worktree root"
    );
    write(
        &decoy_root.join("snippet.md"),
        "DECOY_REPO_TRANSCLUSION_MARKER\n",
    );
    write(
        &decoy_root.join("nested.md"),
        "::file snippet.md\n",
    );

    let prompts_dir = repo_root.join("prompts");
    fs::create_dir_all(&prompts_dir).unwrap();
    let doc = prompts_dir.join("uses_snippet.md");
    // Bare transclusion resolves repository-first: with the fix, the source's
    // repo (repo_root) wins, not the launch CWD's repo (decoy_root).
    write(&doc, "---\n---\n::file nested.md\n");

    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    let capture = workspace.path().join("delivered-prompt.txt");
    write_executable(
        &path_dir.join("claude"),
        r#"#!/bin/sh
{
  for a in "$@"; do
    if [ -f "$a" ]; then cat "$a"; else printf '%s\n' "$a"; fi
  done
  cat
} >> "$CLAUDINE_PROMPT_CAPTURE" 2>/dev/null
exit 0
"#,
    );

    // Launch from the decoy repo while targeting the document in repo_root.
    // If repository discovery were still CWD-driven, the decoy's snippet.md
    // would win and the delivered prompt would contain the decoy marker.
    assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .env("CLAUDINE_PROMPT_CAPTURE", &capture)
        .current_dir(&decoy_root)
        .args(["compose", "--claude", doc.to_str().unwrap()])
        .assert()
        .success();

    let delivered = fs::read_to_string(&capture).unwrap_or_default();
    assert!(
        delivered.contains("PRIMARY_REPO_TRANSCLUSION_MARKER"),
        "the `::file snippet.md` transclusion must resolve against the source \
         document's repository (repo_root), not the launch CWD's repository \
         (decoy_root).\ndelivered prompt:\n{delivered}"
    );
    assert!(
        !delivered.contains("DECOY_REPO_TRANSCLUSION_MARKER"),
        "the decoy repo's snippet must not leak into a document authored in \
         repo_root.\ndelivered prompt:\n{delivered}"
    );
}
