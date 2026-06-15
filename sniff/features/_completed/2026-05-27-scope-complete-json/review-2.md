---
ready: false
agent: codex
model: ""
---

# Review: Scope-Complete JSON for `sniff repo` - Iteration 2

## Findings

### Medium: Public help and docs still describe stale `repo` behavior

- [sniff/cli/src/args/mod.rs:300](/Users/ken/.claudine/worktrees/rusty-biscuit/sniff/sniff/cli/src/args/mod.rs:300) is the clap summary rendered by `sniff repo --help`; it still says `repo` shows repository/monorepo structure. The spec preserves bare text dispatch as the repository name and only makes `sniff repo --json` the aggregate.
- [sniff/cli/src/args/mod.rs:1079](/Users/ken/.claudine/worktrees/rusty-biscuit/sniff/sniff/cli/src/args/mod.rs:1079) still says the top-level `sniff repo` command shows repository structure.
- [sniff/cli/src/args/repo.rs:628](/Users/ken/.claudine/worktrees/rusty-biscuit/sniff/sniff/cli/src/args/repo.rs:628) still says `repo name -v` includes version and language/monorepo info, but the spec moved that rich one-liner to the parent `repo -v`. This stale text is visible in `sniff repo --help`.
- [sniff/docs/cli/repo.md:7](/Users/ken/.claudine/worktrees/rusty-biscuit/sniff/sniff/docs/cli/repo.md:7) says the listed flags apply to all repo subcommands, then lists command-local `--latest-versions` and `--refresh-remotes` at [sniff/docs/cli/repo.md:14](/Users/ken/.claudine/worktrees/rusty-biscuit/sniff/sniff/docs/cli/repo.md:14) and [sniff/docs/cli/repo.md:15](/Users/ken/.claudine/worktrees/rusty-biscuit/sniff/sniff/docs/cli/repo.md:15). The spec is explicit that these are command-local and are not parent aggregate inputs.
- [sniff/docs/cli/repo.md:35](/Users/ken/.claudine/worktrees/rusty-biscuit/sniff/sniff/docs/cli/repo.md:35) still labels `structure` as the default.
- [sniff/docs/cli/repo.md:118](/Users/ken/.claudine/worktrees/rusty-biscuit/sniff/sniff/docs/cli/repo.md:118) says path and exit-code subcommands always produce plain text or no output, but this feature includes JSON for exit-code leaves such as `is-monorepo`, `version`, and boolean leaves in the aggregate.
- [sniff/cli/README.md:867](/Users/ken/.claudine/worktrees/rusty-biscuit/sniff/sniff/cli/README.md:867) still uses `jq -e '.is_monorepo'` against `sniff repo --json`; the implemented and specified aggregate key is kebab-case, so the example should use `jq -e '."is-monorepo"'`.
- [sniff/docs/cli/repo.md:26](/Users/ken/.claudine/worktrees/rusty-biscuit/sniff/sniff/docs/cli/repo.md:26), [sniff/docs/cli/repo.md:27](/Users/ken/.claudine/worktrees/rusty-biscuit/sniff/sniff/docs/cli/repo.md:27), and [sniff/docs/cli/repo.md:28](/Users/ken/.claudine/worktrees/rusty-biscuit/sniff/sniff/docs/cli/repo.md:28) link to `repo_is-monorepo.md`, `repo_package-count.md`, and `repo_version.md`, but those files are still missing.

The core behavior may be usable, but this is public contract drift in the exact areas the spec called out for documentation updates.

**Suggested fix:** Update the clap summaries, top-level after-help, repo subcommand help, README example, and repo CLI docs to match the implemented parent-vs-leaf split. Add the three missing per-leaf doc pages or remove the links.

### Medium: The three new public leaves still lack end-to-end Level 1 tests

The spec asks for `sniff repo is-monorepo --json`, `sniff repo package-count --json`, and `sniff repo version --json` to be verified as single-key public leaves, including the `version: null` exit-code-`1` path. Current coverage includes value helper/unit coverage and aggregate/parser coverage, but I did not find integration tests that invoke those three commands and assert exact stdout JSON, text output, and exit-code behavior.

Manual smoke checks in this worktree showed the implementation behaves correctly:

- `target/debug/sniff repo is-monorepo --json` emitted `{ "is-monorepo": true }` and exited `0`.
- `target/debug/sniff repo package-count --json` emitted `{ "package-count": 65 }` and exited `0`.
- `target/debug/sniff repo version --json` emitted `{ "version": null }` and exited `1`.

That should be captured as Level 1 process coverage. Level 1 is the right rigor here because these are non-interactive stdout/stderr/exit-code contracts; no real terminal rendering or OS input encoder behavior is involved.

**Suggested fix:** Add `assert_cmd` integration tests for the three leaves covering exact JSON key sets, text output, `version` absent with exit `1`, and `version --no-error` with exit `0`.

### Low: Internal JSON-builder comments still describe the retired behavior

[sniff/cli/src/output/repo_json.rs:10](/Users/ken/.claudine/worktrees/rusty-biscuit/sniff/sniff/cli/src/output/repo_json.rs:10), [sniff/cli/src/output/repo_json.rs:77](/Users/ken/.claudine/worktrees/rusty-biscuit/sniff/sniff/cli/src/output/repo_json.rs:77), and [sniff/cli/src/output/repo_json.rs:101](/Users/ken/.claudine/worktrees/rusty-biscuit/sniff/sniff/cli/src/output/repo_json.rs:101) still say bare `sniff repo` / the default branch preserves the full `RepoInfo` JSON behavior. The code now routes `RepoAction::Default` through [sniff/cli/src/commands/mod.rs:1507](/Users/ken/.claudine/worktrees/rusty-biscuit/sniff/sniff/cli/src/commands/mod.rs:1507) and builds the aggregate at [sniff/cli/src/output/repo_json.rs:607](/Users/ken/.claudine/worktrees/rusty-biscuit/sniff/sniff/cli/src/output/repo_json.rs:607).

Per the repo comment-quality guidance, this is behavior-comment drift. The code appears to be the authority; the comments should be corrected or deleted.

## Test Rigor Assessment

- Aggregate JSON shape, required keys, excluded network/parameterized keys, file-list stable shapes, and leaf unwrapping have Level 1 process coverage in [sniff/cli/tests/cli.rs:299](/Users/ken/.claudine/worktrees/rusty-biscuit/sniff/sniff/cli/tests/cli.rs:299) through [sniff/cli/tests/cli.rs:522](/Users/ken/.claudine/worktrees/rusty-biscuit/sniff/sniff/cli/tests/cli.rs:522). Level 1 is appropriate for JSON shape and command dispatch.
- `repo name -v` and `repo -v` field inclusion/exclusion have Level 1 stdout coverage at [sniff/cli/tests/cli.rs:549](/Users/ken/.claudine/worktrees/rusty-biscuit/sniff/sniff/cli/tests/cli.rs:549) and [sniff/cli/tests/cli.rs:604](/Users/ken/.claudine/worktrees/rusty-biscuit/sniff/sniff/cli/tests/cli.rs:604). Level 1 is appropriate because the requirement is not terminal emulator rendering, glyph widths, or keyboard behavior.
- The new leaf commands need Level 1 process coverage. No Level 2 or Level 3 coverage is required by this feature as specified.

## Positive Notes

- Bare `repo` and explicit `repo name` are now distinct actions, matching the dispatch requirement.
- `sniff repo --json` uses a dedicated aggregate path after detection and excludes `remote`, `pr`, and `hash`.
- The new leaves manually smoke-tested with the expected JSON and exit codes.

## Verification

- Passed: `cargo test -p sniff-cli repo_aggregate_json -- --nocapture`
- Passed: `cargo test -p sniff-cli repo_name_verbose_is_name_only -- --nocapture`
- Passed: `cargo test -p sniff-cli repo_default_verbose_is_rich_oneliner -- --nocapture`
- Passed manual smoke checks for `repo is-monorepo --json`, `repo package-count --json`, and `repo version --json`.

## Production Readiness

Not ready. The implementation is close, but the public help/docs still contain user-facing contract drift and the three new public leaves need end-to-end Level 1 integration coverage before this should be marked production-ready.
