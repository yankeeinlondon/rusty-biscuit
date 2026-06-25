---
ready: false
agent: codex
model: ""
---

# Review: Scope-Complete JSON for `sniff repo`

## Findings

### Medium: Public docs and help still misstate the repo JSON/key contract

- [sniff/cli/src/args/mod.rs:1079](/Users/ken/.claudine/worktrees/rusty-biscuit/sniff/sniff/cli/src/args/mod.rs:1079) says `sniff repo` shows repository structure, but the spec explicitly preserves bare text dispatch as the repository name and makes only `sniff repo --json` the aggregate.
- [sniff/cli/README.md:867](/Users/ken/.claudine/worktrees/rusty-biscuit/sniff/sniff/cli/README.md:867) documents `jq -e '.is_monorepo'` for the new aggregate, but the implemented and specified key is kebab-case: `is-monorepo`. That example now always misses the aggregate field.
- [sniff/docs/cli/repo.md:26](/Users/ken/.claudine/worktrees/rusty-biscuit/sniff/sniff/docs/cli/repo.md:26), [sniff/docs/cli/repo.md:27](/Users/ken/.claudine/worktrees/rusty-biscuit/sniff/sniff/docs/cli/repo.md:27), and [sniff/docs/cli/repo.md:28](/Users/ken/.claudine/worktrees/rusty-biscuit/sniff/sniff/docs/cli/repo.md:28) link to `repo_is-monorepo.md`, `repo_package-count.md`, and `repo_version.md`, but those files are not present under `sniff/docs/cli`.
- [sniff/docs/cli/repo.md:7](/Users/ken/.claudine/worktrees/rusty-biscuit/sniff/sniff/docs/cli/repo.md:7) says the listed flags apply to all repo subcommands, while [sniff/docs/cli/repo.md:14](/Users/ken/.claudine/worktrees/rusty-biscuit/sniff/sniff/docs/cli/repo.md:14) and [sniff/docs/cli/repo.md:15](/Users/ken/.claudine/worktrees/rusty-biscuit/sniff/sniff/docs/cli/repo.md:15) list command-local network/package-registry opt-ins. The spec calls out that these are command-local and must not be treated as parent aggregate inputs.

The feature changes public behavior and the spec explicitly requires README/help/docs updates. These mismatches will send users to stale behavior, broken docs links, and the wrong JSON key.

**Suggested fix:** Update the top-level help text to say bare `sniff repo` prints the repository name, fix README examples to use kebab-case keys (`jq -e '."is-monorepo"'`), either add the three missing per-leaf docs or stop linking to them, and describe `--latest-versions` / `--refresh-remotes` as command-local rather than all-subcommand flags.

### Medium: New leaf commands are missing end-to-end integration coverage

The spec asks for `sniff repo is-monorepo --json`, `sniff repo package-count --json`, and `sniff repo version --json` to be covered as single-key JSON leaves, including the `version: null` exit-code-`1` path. Current coverage has unit tests for the value helpers at [sniff/cli/src/output/repo_json.rs:1502](/Users/ken/.claudine/worktrees/rusty-biscuit/sniff/sniff/cli/src/output/repo_json.rs:1502), [sniff/cli/src/output/repo_json.rs:1509](/Users/ken/.claudine/worktrees/rusty-biscuit/sniff/sniff/cli/src/output/repo_json.rs:1509), and [sniff/cli/src/output/repo_json.rs:1516](/Users/ken/.claudine/worktrees/rusty-biscuit/sniff/sniff/cli/src/output/repo_json.rs:1516), plus aggregate tests that only verify the parent object and parser acceptance. I did not find integration tests that invoke the three new leaves and assert exact stdout JSON, text output, and exit code behavior.

This is a Level 1 test gap, not a Level 2/Level 3 mismatch. The requirements are non-interactive JSON/stdout/exit-code behavior, so Level 1 process tests are the appropriate rigor. The missing part is that the tests do not exercise the actual CLI dispatch/output contract for the new user-facing commands.

**Suggested fix:** Add L1 `assert_cmd` tests for:

- `sniff repo is-monorepo --json` has exactly one key, `is-monorepo`, and exits `0`.
- `sniff repo package-count --json` has exactly one key, `package-count`, and exits `0`.
- `sniff repo version --json` returns `{ "version": null }` and exits `1` in a fixture repo with no root manifest version, plus `--no-error` exits `0`.
- Text output for the three leaves matches the script-friendly contract.

## Test Rigor Assessment

- Aggregate JSON shape, excluded network/parameterized keys, key presence, stable file-list shapes, and leaf unwrapping have Level 1 integration coverage in [sniff/cli/tests/cli.rs:299](/Users/ken/.claudine/worktrees/rusty-biscuit/sniff/sniff/cli/tests/cli.rs:299) through [sniff/cli/tests/cli.rs:522](/Users/ken/.claudine/worktrees/rusty-biscuit/sniff/sniff/cli/tests/cli.rs:522). Level 1 is appropriate for JSON shape and command dispatch.
- The “aggregate is offline” requirement is partly covered by Level 1 tests and by code inspection: the `OutputFilter::Repo` plan uses `without_network()` before detection. The test at [sniff/cli/tests/cli.rs:499](/Users/ken/.claudine/worktrees/rusty-biscuit/sniff/sniff/cli/tests/cli.rs:499) is indirect because it asserts excluded keys rather than instrumenting network calls, but the implementation path itself is offline.
- `repo name -v` and bare `repo -v` terminal-subset behavior is Level 1 stdout coverage. That is appropriate here because the requirement is field inclusion/exclusion, not terminal emulator rendering, glyph widths, or keyboard behavior.
- No Level 2 or Level 3 coverage is required by this feature as specified.

## Positive Notes

- Bare `repo` and explicit `repo name` are now distinct actions, which matches the spec's dispatch requirement.
- `sniff repo --json` is routed through a dedicated aggregate builder and the detection plan avoids network detection.
- Manual smoke checks showed:
  - `target/debug/sniff repo is-monorepo --json` emits `{ "is-monorepo": true }` and exits `0`.
  - `target/debug/sniff repo package-count --json` emits `{ "package-count": 65 }` and exits `0`.
  - `target/debug/sniff repo version --json` emits `{ "version": null }` and exits `1` in this worktree.

## Verification

- Passed: `cargo test -p sniff-cli repo_aggregate_json -- --nocapture`

## Production Readiness

Not ready. The core implementation appears close, but the feature should not ship with stale public help/docs and without Level 1 end-to-end coverage for the three new public leaf commands.
