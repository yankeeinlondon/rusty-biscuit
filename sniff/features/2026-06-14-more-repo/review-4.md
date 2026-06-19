---
ready: false
agent: codex
model: ""
---

# Review: More Repo

## Findings

### High: single-package repos fail the new package-manager and dependencies contracts

The spec requires `sniff repo package-manager` to report a singular value for a non-monorepo, and `sniff repo dependencies` to report external dependencies declared by repo packages (`spec.md:101`, `spec.md:115`). The current implementation only works for monorepo/workspace structures. In a plain Cargo package with `[package]` and a `serde` dependency, `repo package-manager --json` returns `{ "package_manager": null }`, and `repo dependencies --json` exits with `Error: Not inside a recognized repository`.

The issue is in the focused handlers: `handle_repo_dependencies` treats `detect_repo_structure(&root)? == None` as a hard error, while `handle_repo_package_manager` maps the same `None` to `AggregateResult::Empty` (`sniff/cli/src/commands/repo.rs:278`, `sniff/cli/src/commands/repo.rs:285`, `sniff/cli/src/commands/repo.rs:345`, `sniff/cli/src/commands/repo.rs:353`, `sniff/cli/src/commands/repo.rs:382`). `detect_repo_structure` does not create a `RepoInfo` for a normal single Cargo package, because `detect_cargo_workspace` returns `None` unless `[workspace].members` exists. `repo test-runner` has a direct non-monorepo fallback; these two commands need the same class of library-owned fallback, ideally by producing root-package metadata once rather than ad hoc CLI parsing.

This also leaves the bare `repo --json` aggregate wrong for single-package repos: repo-wide `package_manager`, `dependencies`, and `package_dependencies` are empty/null even though the root manifest has usable facts.

Verification level: Level 1 CLI integration is appropriate. Add single-package fixtures for Cargo, Node, and Python that assert `repo package-manager`, `repo dependencies`, and bare `repo --json` report root-package facts.

### High: `repo branches --json` does not emit the specified nullable fields

The branch JSON contract is explicit: every branch object has `sha: string | null`, `upstream: string | null`, `ahead: number | null`, and `behind: number | null` (`spec.md:87`). The implementation serializes `BranchInfo` directly (`sniff/cli/src/commands/repo.rs:223`), but the type skips `upstream` when it is `None` and forces `ahead`/`behind` to `usize` (`sniff/lib/src/filesystem/git/types.rs:219`). A newly initialized repo with no upstream emits no `upstream` key at all and reports `ahead: 0`, `behind: 0`, so consumers cannot distinguish "no upstream/tracking data" from "configured upstream and exactly even."

Fix the public JSON shape by making `upstream` serialize as `null` when absent and by modeling `ahead`/`behind` as nullable when no configured upstream is available. If the library type cannot change, add a focused JSON projection for `repo branches --json` and the aggregate `branches` field.

Verification level: Level 1 CLI integration is appropriate. Extend `test_repo_branches_json_shape` to assert all contract keys are present and that a no-upstream branch serializes `upstream`, `ahead`, and `behind` as `null`.

## Test Rigor Notes

- These findings are data-contract and CLI behavior issues; Level 1 unit/CLI integration coverage is the correct verification level.
- I did not find feature requirements that require Level 2 real-terminal capture or Level 3 OS keyboard injection. The terminal rendering here is ordinary command output, not key input, emulator encoding, or real-terminal layout behavior.
- Current tests cover many workspace and aggregate paths, but they miss ordinary single-package repos for the new package-manager/dependency commands and only partially assert the branch JSON object shape.

## Verification

- Ran targeted Level 1 tests:
  - `cargo test -p sniff-cli --test cli test_repo_aggregate_dependencies_are_cwd_invariant --color=never`
  - `cargo test -p sniff-cli --test cli test_repo_package_manager_variant_list_uses_unique_values --color=never`
- Manually reproduced the single-package Cargo failures with `target/debug/sniff --base "$tmp" repo package-manager --json` and `target/debug/sniff --base "$tmp" repo dependencies --json`.
- Manually reproduced the missing branch fields with `target/debug/sniff --base "$tmp" repo branches --json`.
- Did not run the full suite.
