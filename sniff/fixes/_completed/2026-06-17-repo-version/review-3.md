---
ready: false
agent: codex
model: ""
---

# Review: `sniff repo version` iteration 3

## Findings

### High: `sniff-cli` test build does not compile

The implementation fixed the iteration-2 runtime bug by changing the bare aggregate helper to derive its root from `RepoInfo`:

- `sniff/cli/src/output/repo_json.rs:918` now defines `aggregate_repo_version(repo: Option<&RepoInfo>)`.

But three unit tests in the same file still call the old two-argument form:

- `sniff/cli/src/output/repo_json.rs:2820`
- `sniff/cli/src/output/repo_json.rs:2828`
- `sniff/cli/src/output/repo_json.rs:2877`

`cargo test --color=never -p sniff-cli --no-run` fails with `E0061: this function takes 1 argument but 2 arguments were supplied`, so the CLI test target cannot compile. This blocks production readiness even though `cargo check` passes, because the acceptance criteria require CLI integration/unit coverage for the new JSON collapse behavior and scope behavior.

Fix: update those assertions to call `aggregate_repo_version(Some(&repo))` / `aggregate_repo_version(Some(&empty))`. Since the helper now intentionally reads `repo.root`, keep the test fixtures' `RepoInfo.root` values meaningful rather than passing a separate root argument.

Strongest verification present: Level 1 tests are present in source, but they are currently uncompilable for `sniff-cli`, so they provide no executable verification.

## Test Rigor

The user-observable requirements here are JSON shape, text rendering, scope selection, source attribution, and exit status. Level 1 unit and CLI tests are the appropriate verification level; no requirement depends on real terminal rendering, terminal input encoding, mouse/paste/IME behavior, or OS keyboard injection, so Level 2 and Level 3 tests are not required.

The current gap is not a tier mismatch. It is a Level 1 build failure: the test target for the CLI cannot compile.

## Verification Run

- `cargo check --color=never -p sniff -p sniff-cli` passed.
- `cargo test --color=never -p sniff filesystem::repo::aggregate::tests::aggregate_versions --lib` passed: 10 tests.
- `cargo test --color=never -p sniff-cli --no-run` failed with the `aggregate_repo_version` stale-call errors above.

## Production Readiness

Not ready. The remaining issue is mechanical but release-blocking: the `sniff-cli` test target must compile before the feature can be considered production-ready.
