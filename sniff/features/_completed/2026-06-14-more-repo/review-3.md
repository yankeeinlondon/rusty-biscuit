---
ready: false
agent: codex
model: ""
---

# Review: More Repo

## Findings

### High: bare `repo --json` dependencies are cwd-scoped instead of repo-wide

The spec classifies `dependencies` as a repo-wide aggregate fact that must be identical regardless of where the command is invoked in the tree (`spec.md:195`). The implementation wires the aggregate field through `aggregate_external_dependencies(repo, dir)` (`sniff/cli/src/output/repo_json.rs:787`), and that helper calls `resolve_scope(repo, dir)` whenever packages exist (`sniff/cli/src/output/repo_json.rs:906`). Inside a package or package-area, bare `sniff repo --json` will therefore emit only that local dependency subset while sibling repo-wide fields such as `packages`, `package_dependencies`, `package_manager`, and `test_runner` are still repo-scoped.

That breaks the redesigned aggregate contract and makes the same no-arg JSON payload depend on cwd outside the explicit `context` object. Use `AggregateScope::Repo` for the aggregate `dependencies` field and keep cwd scoping only on the focused `sniff repo dependencies` command.

Verification level: Level 1 CLI integration is sufficient. Add a fixture with two workspace packages that declare different external dependencies, run bare `repo --json` from the repo root and from one package directory, and assert `.dependencies.dependencies` is identical in both outputs.

### High: missing `repo version` exits nonzero even though the spec says absence is not an error

The version fix explicitly says ecosystems with no modeled version source should return `null`, and “A missing version is not an error” (`spec.md:149`, `spec.md:155`). 

> **SPEC CHANGE**: this should be changed ... a missing version number should return a error exit code but we should include a `--no-error` CLI flag which removes the error exit code (but still returns `null` on STDOUT). Please update spec and ensure implementation follows this update.


This affects valid repos such as Go projects with no explicit version source. `sniff repo version` and `sniff repo version --json` should exit 0 with empty text / `{ "version": null }` when the manifest has no version, reserving nonzero exits for actual detection failures.

Verification level: Level 1 CLI integration is sufficient. Update the current null-version tests to assert exit 0 for both text and JSON on a versionless repo.

### Medium: `software test-runners` is not wired like the accepted 9th program category

The accepted test-runner strategy says the host surface should follow the established category pattern and explicitly calls for `define_program_action!(TestRunnerAction, ...)` in the CLI (`test-runner-strategy.md:83`, `test-runner-strategy.md:89`). The implemented clap tree makes `SoftwareSubcommand::TestRunners` a leaf (`sniff/cli/src/args/mod.rs:317`), unlike editors/utilities/etc., so `sniff software test-runners install ...` and `install-plan` are rejected before dispatch. The report path exists, but the category is not feature-complete against the accepted pattern.

Either add `TestRunnerAction` with the same install/install-plan support as the other software categories, or amend the spec/strategy to say test runners are intentionally report-only.

Verification level: Level 1 clap/CLI tests are sufficient. Add parse and CLI failure/success tests for `software test-runners install --help` and `software test-runners install-plan <runner>` if the action is supported.

## Test Rigor Notes

- These findings are CLI/library JSON semantics and command parsing; Level 1 coverage is the appropriate verification level.
- I did not find requirements in this feature that need Level 2 real-terminal rendering or Level 3 OS keyboard injection. The terminal-output requirements are ordinary table/list/prose output, not real-emulator styling, width, or input-encoder behavior.
- Existing Level 1 aggregate tests cover shape, snake_case keys, JSON validity, size reduction, and old-key removal, but they do not cover cwd-invariance for repo-wide aggregate facts.

## Verification

- Reviewed the spec, test-runner strategy, staged implementation, CLI tests, and Sniff/rust-testing skill guidance.
- Did not run the full test suite; this was a code review pass focused on contract gaps.
