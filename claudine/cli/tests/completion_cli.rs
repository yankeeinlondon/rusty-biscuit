//! End-to-end integration tests for the Claudine completion engine.
//!
//! These tests spawn the compiled `claudine` binary with the hidden
//! `__complete` subcommand that generated bash/zsh/fish scripts shell out
//! to on every `<TAB>`. The CLI contract is:
//!
//! ```text
//! claudine __complete --current <INDEX> -- <argv...>
//! ```
//!
//! where `<argv...>` starts with the binary name at position 0, the
//! subcommand at position 1, and so on. `<INDEX>` is the 0-based position
//! of the token being completed. The engine returns one candidate per line
//! on stdout, or nothing when the cursor is not at a targeted argument
//! position.
//!
//! Coverage focus:
//!
//! - Composition positional (`compose`) — curated scope matching, path
//!   resets, substring/subsequence hits, non-markdown exclusion,
//!   `.gitignore` honored.
//! - Unsupported slots — wrapper subcommands, setter-shaped tokens, and
//!   unsupported prefix forms must all emit zero candidates so the
//!   shell's native completion takes over.

use std::fs;
use std::path::Path;

mod common;
use common::completion::run_complete as run_complete_trailing;
use common::{TestWorkspace, init_git_repo};

/// Seed a fake `.git` directory plus a few markdown fixtures under a
/// `prompts/` curated directory.
fn seed_curated_fixtures(root: &Path) {
    fs::create_dir_all(root.join(".git")).unwrap();
    let prompts = root.join("prompts");
    fs::create_dir_all(&prompts).unwrap();
    fs::write(prompts.join("plain.md"), "# plain\n").unwrap();
    fs::write(prompts.join("with-prompt.md"), "# with prompt\n").unwrap();
    fs::write(prompts.join("seq.md"), "# sequence\n").unwrap();
    // Non-markdown companions — every scenario must drop them.
    fs::write(prompts.join("notes.txt"), "not markdown\n").unwrap();
}

/// Seed a real git workspace (so `sniff::detect_repo_structure` returns a
/// usable `RepoInfo`) with a minimal Cargo workspace. This is required for
/// the package-root and package-area curated scopes; a bare `.git` is not
/// enough.
fn seed_repo_with_package(root: &Path) {
    assert!(
        init_git_repo(root),
        "unable to initialize git repo at {root:?}",
    );
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
}

// ---------------------------------------------------------------------
// Acceptance criterion 1 — empty input, inside a repo
// ---------------------------------------------------------------------

#[test]
fn empty_input_lists_curated_markdown_inside_repo() {
    let workspace = TestWorkspace::named("supp-empty-input");
    let root = workspace.path();
    seed_repo_with_package(root);
    let prompts = root.join("prompts");
    fs::create_dir_all(&prompts).unwrap();
    fs::write(prompts.join("plain.md"), "# p\n").unwrap();

    let got = run_complete_trailing(root, &["compose", ""]);

    assert!(
        got.iter().any(|c| c == "prompts/plain.md"),
        "empty input should surface curated repo markdown; got: {got:?}",
    );
    // `.txt` siblings must never appear.
    fs::write(prompts.join("notes.txt"), "t").unwrap();
    let retry = run_complete_trailing(root, &["compose", ""]);
    assert!(
        !retry.iter().any(|c| c.ends_with(".txt")),
        "empty-input completion leaked a non-markdown file: {retry:?}"
    );
}

// ---------------------------------------------------------------------
// Acceptance criterion 2 — `@pr` two-char curated match
// ---------------------------------------------------------------------

#[test]
fn two_char_curated_match_finds_multiple_files_by_substring() {
    let workspace = TestWorkspace::named("supp-two-char");
    let root = workspace.path();
    seed_curated_fixtures(root);
    // Additional match for the mid-filename case below.
    fs::write(root.join("prompts").join("suppress.md"), "# suppress\n").unwrap();
    fs::write(root.join("prompts").join("my-prompt.md"), "# my prompt\n").unwrap();

    let got = run_complete_trailing(root, &["compose", "@pr"]);

    // Magic mode keeps the `@` and inserts the filename only; matching is
    // fuzzy-subsequence on the filename stem.
    assert!(
        got.iter().any(|c| c == "@with-prompt.md"),
        "@pr should match `with-prompt.md`; got: {got:?}",
    );
    assert!(
        got.iter().any(|c| c == "@suppress.md"),
        "@pr should match `suppress.md` (subsequence); got: {got:?}",
    );
    assert!(
        got.iter().any(|c| c == "@my-prompt.md"),
        "@pr should match `my-prompt.md` (subsequence); got: {got:?}",
    );
    // `plain.md` and `seq.md` filenames do NOT have `p` before `r`; they
    // must NOT appear.
    assert!(
        !got.iter().any(|c| c.ends_with("plain.md")),
        "@pr matching must be on filename, not path; got: {got:?}",
    );
    assert!(
        !got.iter().any(|c| c.ends_with("seq.md")),
        "@pr matching must be on filename, not path; got: {got:?}",
    );
}

// ---------------------------------------------------------------------
// Acceptance criterion 3 — `@pro` three-char broad scan
// ---------------------------------------------------------------------

#[test]
fn non_curated_paths_never_surface_regardless_of_typed_length() {
    let workspace = TestWorkspace::named("supp-no-broad-scan");
    let root = workspace.path();
    seed_curated_fixtures(root);
    // Put a matching markdown file OUTSIDE the curated directories; it
    // must never surface — the engine only walks prompts/ and sequences/.
    fs::create_dir_all(root.join("random")).unwrap();
    fs::write(root.join("random").join("process.md"), "# process\n").unwrap();

    for partial in ["@pr", "@pro", "@process", "process"] {
        let got = run_complete_trailing(root, &["compose", partial]);
        assert!(
            !got.iter().any(|c| c.contains("random/process.md")),
            "non-curated path leaked for {partial:?}: {got:?}"
        );
    }
}

// ---------------------------------------------------------------------
// Acceptance criterion 4 — `@prompts/` path-separator reset
// ---------------------------------------------------------------------

#[test]
fn path_separator_reset_enumerates_inside_prompts_directory() {
    let workspace = TestWorkspace::named("supp-path-reset");
    let root = workspace.path();
    seed_curated_fixtures(root);

    // `@prompts/` committed-directory form — new engine strips the `@`
    // sigil on selection and delegates to the committed-directory walker.
    let got = run_complete_trailing(root, &["compose", "prompts/"]);

    // Every file inside the repo `prompts/` directory should surface,
    // regardless of filename, because the segment after `/` is empty
    // (0 meaningful chars).
    assert!(
        got.iter().any(|c| c == "prompts/plain.md"),
        "path reset should surface plain.md: {got:?}",
    );
    assert!(
        got.iter().any(|c| c == "prompts/with-prompt.md"),
        "path reset should surface with-prompt.md: {got:?}",
    );
    assert!(
        got.iter().any(|c| c == "prompts/seq.md"),
        "path reset should surface seq.md: {got:?}",
    );
    // The non-markdown `notes.txt` must still be dropped.
    assert!(
        !got.iter().any(|c| c.ends_with(".txt")),
        "path reset leaked a non-markdown file: {got:?}",
    );
}

// ---------------------------------------------------------------------
// Acceptance criterion 5 — implicit-relative `prompts/`
// ---------------------------------------------------------------------

#[test]
fn implicit_relative_prompts_stays_repo_only() {
    let workspace = TestWorkspace::named("supp-implicit-relative");
    let root = workspace.path();
    seed_curated_fixtures(root);

    let got = run_complete_trailing(root, &["compose", "prompts/"]);

    assert!(
        got.iter().any(|c| c == "prompts/plain.md"),
        "implicit-relative should surface repo-local matches: {got:?}",
    );
    // Implicit-relative matches must NOT gain the `@` prefix.
    for c in &got {
        if c.contains("prompts/plain.md") || c.contains("prompts/seq.md") {
            assert!(
                !c.starts_with('@'),
                "implicit-relative repo match must not gain an @ prefix; got: {c}"
            );
        }
    }
}

// ---------------------------------------------------------------------
// Wrapper / composition --asp & --rsp — the current engine explicitly
// leaves these slots to the shell's native file completion. We assert
// empty stdout so a future accidental re-introduction of a curated
// completer here would fail loud.
// ---------------------------------------------------------------------

#[test]
fn wrapper_asp_flag_value_slot_emits_no_candidates() {
    let workspace = TestWorkspace::named("comp-wrapper-asp");
    let root = workspace.path();
    seed_repo_with_package(root);
    let prompts = root.join("prompts");
    fs::create_dir_all(&prompts).unwrap();
    fs::write(prompts.join("plain.md"), "# p\n").unwrap();

    let got = run_complete_trailing(root, &["claude", "--asp", ""]);
    assert!(
        got.is_empty(),
        "wrapper --asp flag value slot must defer to clap/shell default \
         file completion: {got:?}",
    );
}

#[test]
fn wrapper_replace_system_prompt_flag_value_slot_emits_no_candidates() {
    let workspace = TestWorkspace::named("comp-wrapper-rsp");
    let root = workspace.path();
    seed_repo_with_package(root);
    let prompts = root.join("prompts");
    fs::create_dir_all(&prompts).unwrap();
    fs::write(prompts.join("plain.md"), "# p\n").unwrap();

    let got = run_complete_trailing(root, &["codex", "--replace-system-prompt", "@"]);
    assert!(
        got.is_empty(),
        "wrapper --replace-system-prompt value slot must defer to the \
         shell's default file completion: {got:?}",
    );
}

#[test]
fn compose_asp_flag_value_slot_emits_no_candidates() {
    let workspace = TestWorkspace::named("comp-compose-asp");
    let root = workspace.path();
    seed_repo_with_package(root);
    let prompts = root.join("prompts");
    fs::create_dir_all(&prompts).unwrap();
    fs::write(prompts.join("plain.md"), "# p\n").unwrap();

    let got = run_complete_trailing(root, &["compose", "file.md", "--asp", ""]);
    assert!(
        got.is_empty(),
        "compose --asp value slot must defer to the shell's default file \
         completion: {got:?}",
    );
}

// ---------------------------------------------------------------------
// Acceptance criterion 8 — non-markdown files are never offered
// ---------------------------------------------------------------------

#[test]
fn non_markdown_files_never_surface() {
    let workspace = TestWorkspace::named("supp-non-markdown");
    let root = workspace.path();
    seed_curated_fixtures(root);
    fs::write(root.join("prompts").join("code.rs"), "fn main(){}\n").unwrap();

    let got = run_complete_trailing(root, &["compose", "@"]);

    assert!(
        !got.iter().any(|c| c.ends_with(".txt")),
        "non-markdown .txt candidate leaked: {got:?}"
    );
    assert!(
        !got.iter().any(|c| c.ends_with(".rs")),
        "non-markdown .rs candidate leaked: {got:?}"
    );
}

// ---------------------------------------------------------------------
// Acceptance criterion 9 — gitignored files excluded from broad scan
// ---------------------------------------------------------------------

#[test]
fn gitignored_files_excluded_from_curated_scan() {
    let workspace = TestWorkspace::named("supp-gitignore");
    let root = workspace.path();
    fs::create_dir_all(root.join(".git")).unwrap();
    fs::write(root.join(".gitignore"), "prompts/ignored/\n").unwrap();
    fs::create_dir_all(root.join("prompts").join("ignored")).unwrap();
    fs::write(
        root.join("prompts").join("ignored").join("secret.md"),
        "# s\n",
    )
    .unwrap();
    fs::write(root.join("prompts").join("kept.md"), "# k\n").unwrap();

    let got = run_complete_trailing(root, &["compose", "@"]);

    // `@` with empty active segment — magic mode keeps `@` and emits
    // `@<basename>` candidates.
    assert!(
        got.iter().any(|c| c == "@kept.md"),
        "non-ignored curated match must surface: {got:?}"
    );
    assert!(
        !got.iter().any(|c| c.contains("secret")),
        "gitignored curated file must NOT appear: {got:?}"
    );
}

// ---------------------------------------------------------------------
// Acceptance criterion 10 — mid-filename substring match
// ---------------------------------------------------------------------

#[test]
fn mid_filename_substring_match_fires() {
    let workspace = TestWorkspace::named("supp-mid-filename");
    let root = workspace.path();
    seed_curated_fixtures(root);
    // `prompt.md` contains `omp` in the middle, so `@omp<tab>` must
    // match.
    fs::write(root.join("prompts").join("prompt.md"), "# p\n").unwrap();

    let got = run_complete_trailing(root, &["compose", "@omp"]);

    // Magic mode keeps `@` and inserts the filename; fuzzy-subsequence
    // matching on the stem matches `omp` against `prompt`.
    assert!(
        got.iter().any(|c| c == "@prompt.md"),
        "mid-filename subsequence match failed: {got:?}"
    );
}

// ---------------------------------------------------------------------
// Non-targeted positions → engine emits nothing
// ---------------------------------------------------------------------

#[test]
fn non_targeted_subcommand_emits_no_candidates() {
    let workspace = TestWorkspace::named("supp-non-targeted");
    let root = workspace.path();
    seed_curated_fixtures(root);

    // `hooks` is not a composition or wrapper subcommand; the engine
    // must return empty stdout so the shell falls back to its default
    // completion.
    let got = run_complete_trailing(root, &["hooks", ""]);
    assert!(
        got.is_empty(),
        "non-targeted subcommand must return empty stdout: {got:?}"
    );
}

#[test]
fn setter_shaped_token_suppresses_completion() {
    let workspace = TestWorkspace::named("supp-setter");
    let root = workspace.path();
    seed_curated_fixtures(root);

    // `key=val` at the positional slot is a setter, not a file
    // reference; the engine must suppress completion.
    let got = run_complete_trailing(root, &["compose", "key="]);
    assert!(
        got.is_empty(),
        "setter-shaped token must suppress completion: {got:?}"
    );
}

#[test]
fn unsupported_prefix_returns_no_candidates() {
    let workspace = TestWorkspace::named("supp-unsupported");
    let root = workspace.path();
    seed_curated_fixtures(root);

    for prefix in [
        "!pkg",
        "vault:x",
        "./local",
        "/abs/path",
        "%recursive",
        "{{HOME}}",
    ] {
        let got = run_complete_trailing(root, &["compose", prefix]);
        assert!(
            got.is_empty(),
            "unsupported prefix `{prefix}` must return no candidates: {got:?}"
        );
    }
}
