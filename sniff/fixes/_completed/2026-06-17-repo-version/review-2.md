---
ready: false
agent: codex
model: ""
---

# Review: `sniff repo version` iteration 2

## Findings

### High: bare `repo --json` version collapse uses the invocation directory as the repo root

The spec requires the consolidated `sniff repo --json` top-level `version` to be the repo-scope collapse: exactly one distinct package version across all packages emits that string; zero or multiple distinct versions emits `null`.

The implementation wires the aggregate field through `aggregate_repo_version(repo, dir)` at `sniff/cli/src/output/repo_json.rs:774`, and that helper passes `dir` directly to `bare_aggregate_version` at `sniff/cli/src/output/repo_json.rs:913`. For bare aggregate output, `dir` is the CLI base/CWD, not necessarily the repository root. This matters because `bare_aggregate_version` re-resolves manifest sources and `cargo_package_version_with_source` looks for inherited Cargo workspace versions at `repo_root.join("Cargo.toml")` (`sniff/lib/src/filesystem/repo/cargo.rs:241`).

I reproduced this with a pure-virtual Cargo workspace containing one member whose manifest uses `version.workspace = true`. From inside the member scope:

```text
sniff --base <workspace>/member repo --json
```

reported:

```json
{ "version": null, "package_count": 1 }
```

The expected top-level `version` is the workspace `[workspace.package].version` because repo-scope aggregation finds exactly one package version. The focused command path avoids this by discovering the enclosing root before calling `aggregate_versions`; the bare aggregate path needs the same root discipline, likely by using `repo.root` (or `repo_root(dir)`) rather than `dir` for `bare_aggregate_version`.

Strongest verification present: Level 1 unit tests cover `aggregate_repo_version` only when the supplied path is already the repo root, and Level 1 CLI tests cover the focused `repo version` command. There is no Level 1 CLI/integration test for `sniff --base <package> repo --json` with Cargo workspace-inherited versions, so this acceptance criterion is currently unguarded.

## Test Rigor

The user-observable requirements for this feature are JSON shape, text output, scope selection, source attribution, and exit status. Level 1 CLI/in-process coverage is the right level; no requirement here depends on a real terminal renderer or OS keyboard input, so Level 2/Level 3 tests are not required.

The gap above is a Level 1 coverage gap, not a tier mismatch: add a test that builds a pure-virtual Cargo workspace with `[workspace.package].version`, runs bare `repo --json` from a member directory via `--base`, and asserts the top-level `version` is the inherited string.

## Verification Run

- `cargo test -p sniff-cli --test cli test_repo_version_verbose_named_workspace_inheritance -- --nocapture` passed.
- Manual reproduction for bare `repo --json` from a package subdirectory in an inherited-version workspace produced `version: null`, confirming the finding.

## Production Readiness

Not ready. The focused `sniff repo version` behavior is in good shape, but the spec also changes bare `sniff repo --json`; that aggregate still fails a documented workspace-inheritance case when invoked from inside the repo.
