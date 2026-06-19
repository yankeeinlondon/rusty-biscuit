---
ready: false
agent: codex
model: ""
---

# Review: `sniff repo version`

## Findings

### High: `Package.version` still misses Cargo workspace inheritance

The spec explicitly says Cargo `version.workspace = true` resolution belongs in the shared library version-resolution path used by both detection and aggregation. The implementation added `cargo_package_version_with_source` for `aggregate_versions`, but `create_package` still populates `Package.version` through `resolve_package_version`, which calls the literal-only `cargo_package_version`.

Code reference: `sniff/lib/src/filesystem/repo/detection.rs:733`

That means `sniff repo version` can report inherited versions correctly, but other public package-catalog surfaces still serialize inheriting Cargo packages with no `version`. For example, `repo structure --json` and any library consumer of `RepoInfo.packages[].version` remain wrong for inherited Cargo workspaces. This also creates two divergent definitions of "package version" inside the same library.

Fix: make detection-time `resolve_package_version` use the same Cargo inheritance resolver, or factor a shared helper that returns at least the resolved string for detection and `(version, source)` for attribution.

Strongest verification present: Level 1 aggregation/CLI tests cover inherited output for the focused command, but there is no Level 1 test proving the package catalog's `Package.version` is populated for `version.workspace = true`.

### High: Explicit scope overrides are skipped outside `info.is_monorepo`

`handle_repo_version` only calls `resolve_scope_with_overrides` inside the `info.is_monorepo && packages non-empty` branch. For a synthesized single-package repo, or a repo detected from a subdirectory as a single package, all overrides are silently ignored and the code falls back to `resolve_directory_version`.

Code reference: `sniff/cli/src/commands/repo.rs:672`

This violates the override contract: `--package <NAME>` / `--package-area <NAME>` should validate named targets against the catalog, and unknown names should error clearly. Today a single-package project can accept `sniff repo version --package ghost` and still print the local version. That is a user-visible correctness bug.

Fix: when `info.packages` is present, resolve overrides regardless of `is_monorepo`; only use the direct directory fallback when there is no catalog at all. Unknown package/area tests should include a synthesized single-package repo.

Strongest verification present: Level 1 CLI tests cover unknown overrides only in a monorepo fixture. They do not exercise the synthesized single-package path.

### Medium: `--base` pointing inside a monorepo does not discover the containing repo

The handler treats any explicit `--base` as the detection root:

Code reference: `sniff/cli/src/commands/repo.rs:656`

So `--base <repo>/pkg-a/lib repo version --all --json` analyzes only `pkg-a/lib` as a synthesized single-package repo instead of finding the enclosing repo and applying `--all` across all packages. The current test at `sniff/cli/tests/cli.rs:2396` uses exactly this shape, but it only asserts the collapsed version string, so it passes even if `pkg-b` is omitted.

Fix: either define `--base` for this scoped command as the CWD-equivalent path and discover the repo root from it, or tighten the tests and docs if `--base` is intentionally "analysis root". Given the spec says `--all` selects scope independent of CWD, the former is the expected behavior.

Strongest verification present: Level 1 CLI test exists, but it is too weak because uniform versions mask the missing packages. Assert the package list includes all repo packages, or use differing versions.

## Test Rigor

The user-observable behavior here is command output, JSON shape, scope selection, and exit status. Level 1 CLI/in-process testing is appropriate; no terminal emulator rendering, keyboard input, mouse, paste, or scrolling behavior requires Level 2 or Level 3. The missing coverage is not a level mismatch, it is untested cases in the Level 1 suite.

## Verification Run

- `cargo check --color=never -p sniff -p sniff-cli` passed.
- `cargo test --color=never -p sniff filesystem::repo::aggregate::tests::aggregate_versions --lib` passed: 10 tests.

## Production Readiness

Not ready. The focused command works for the main happy path, but the implementation still leaves the shared package catalog inconsistent for inherited Cargo versions and skips explicit-scope validation in non-monorepo/single-package paths.
