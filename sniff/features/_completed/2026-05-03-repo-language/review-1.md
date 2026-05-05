---
ready: false
agent: codex
model: ""
---

# Review: `repo language`

## Findings

### Critical: `sniff repo language` performs ~5x more work than necessary

`sniff repo language` currently takes **~0.65s wall time** (0.96s user) because the `DetectionPlan` for `RepoAction::Language` uses `FilesystemRequest::new()` with all its expensive defaults:

- **`GitRequest::full()`** (default) — discovers 10 commits, per-file change stats, worktrees, and unified diffs. None of this is consumed by `render_repo_language`, which only needs `result.filesystem.languages.primary`.
- **`RepoRequest::full()`** (default) — builds a manifest index, discovers nested packages, and runs per-package language scanning. The skill docs note this is "10-50x slower than `structure()`". `render_repo_language` never touches `result.filesystem.repo`.
- **`include_file_inventory: true`** (default) — this is the *only* work that matters, because `primary_language_name()` reads `result.filesystem.languages`, which is produced by the file-inventory summarizer.

**Fix applied:** changed the `RepoAction::Language` branch in `commands.rs` to:

```rust
FilesystemRequest::new()
    .git(GitRequest::summary())   // only need repo-root discovery, not commits/diffs
    .without_repo()               // skip manifest index + nested package scan entirely
    .without_docs()
    .without_formatting()
```

**Result:** wall time drops from **~0.65s → ~0.13s** (4.7× faster), user time from **~0.96s → ~0.09s** (10× faster). Non-git directories run in **~0.01s**.

Relevant code: [commands.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/sniff/sniff/cli/src/commands.rs:705).

### High: `sniff repo language` has no direct Level 1 verification

The feature adds a new user-facing command, but I could not find tests that spawn the CLI for `sniff repo language`, assert the text output, assert the JSON shape, or assert that `--base` targets another repo for this subcommand. The existing tests around `sniff/cli/tests/cli.rs:736` cover the older top-level `sniff language`, not `repo language`; `rg` finds no CLI tests for `RepoSubcommand::Language`, `repo language`, `render_repo_language`, or the `{ "language": "Rust" }` contract.

Requirement verification:

- New command `sniff repo language` returns the primary language: no direct test found. Required level: Level 1. Present level: none.
- `sniff repo language --json` returns focused JSON: no direct test found. Required level: Level 1. Present level: none.
- `--base <dir>` works with repo subcommands, including this command and nested/global placement: no direct `repo --base ... language` or `repo language --base ...` test found. Required level: Level 1. Present level: none.

This does not need Level 2 or Level 3 because the output is plain text/JSON and the interaction is argv parsing, not terminal rendering or OS keyboard input. Level 1 integration tests with a temporary git repo are the right verification level here.

Relevant implementation points: [args.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/sniff/sniff/cli/src/args.rs:227), [commands.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/sniff/sniff/cli/src/commands.rs:705), [output/mod.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/sniff/sniff/cli/src/output/mod.rs:551), [repo_json.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/sniff/sniff/cli/src/output/repo_json.rs:180).

### Medium: No-language text mode silently succeeds with empty output

`render_repo_language` documents that it returns an empty string when no primary language is known so callers can decide how to surface absence, but the caller just appends that string and exits successfully. In a repo with no language files, `sniff repo language` exits `0` with no stdout, while JSON emits `{ "language": null }`.

That is ambiguous for scripting: an empty successful stdout is indistinguishable from a formatting or plumbing bug. The spec asks for the primary programming language found in the repo, and the implementation plan explicitly called out handling the "no primary language" case. Either text mode should exit non-zero on absence, or it should emit a stable sentinel/message and have tests locking that behavior down.

Relevant code: [filesystem.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/sniff/sniff/cli/src/output/filesystem.rs:2083), [output/mod.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/sniff/sniff/cli/src/output/mod.rs:551).

### Low: Repo help omits the new subcommand

The repo-specific after-help says `sniff repo --help` is where users can see repo commands, but the curated examples do not mention `sniff repo language`. This is a small discoverability gap rather than a runtime bug.

Relevant code: [args.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/sniff/sniff/cli/src/args.rs:1447).

## Notes

Manual smoke checks against temporary git repos showed the happy path currently works:

- Rust repo: `sniff repo language` prints `Rust` and exits `0`.
- Rust repo JSON: `sniff repo language --json` emits `{ "language": "Rust" }`.
- External Python repo: `sniff --base <repo> repo language`, `sniff repo --base <repo> language`, and `sniff repo language --base <repo>` all print `Python`.
- Empty git repo: text mode exits `0` with empty stdout; JSON emits `{ "language": null }`.

## Suggested Fixes

Add focused Level 1 integration tests in `sniff/cli/tests/cli.rs` using temporary git repos:

- `repo language` text output is exactly `Rust\n` for a repo containing `src/main.rs`.
- `repo language --json` is valid JSON and exactly has `language: "Rust"`.
- `--base` works for `sniff --base <repo> repo language`, `sniff repo --base <repo> language`, and `sniff repo language --base <repo>`.
- Empty/no-language repo behavior is explicit and locked down for both text and JSON.

Then add `sniff repo language` to `REPO_AFTER_HELP`.

## Production Readiness

Not ready. The happy-path implementation appears functional, but the new user-visible command and the `--base` regression fix are not verified at the appropriate Level 1 boundary, and the no-language text behavior needs an explicit product decision plus tests.
