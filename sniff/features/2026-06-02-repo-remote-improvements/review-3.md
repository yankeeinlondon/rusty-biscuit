---
agent: codex
model: ""
ready: false
---

# Review: `sniff repo remote` PR count + CI/CD runs

## Findings

### High: `sniff repo remote` still does not fetch workflow runs through the normal provider path

The GitHub-specific implementation exists in `GitHubRemote::list_workflow_runs`, but the production CLI path does not call `GitHubRemote` directly. `sniff repo remote` constructs a `GitRemote` enum via `GitRemote::from_url` / `GitRemote::from_shorthand` and then calls `remote.fetch_report(...)` on that enum (`sniff/cli/src/commands/remote.rs:18-19`, `sniff/cli/src/commands/remote.rs:42-44`).

`fetch_report` calls `self.list_workflow_runs(owner, repo, 5)` before falling back to `detect_cicd` (`sniff/lib/src/remote/provider.rs:173-177`). However, the `RemoteRepoProvider for GitRemote` impl forwards `detect_cicd`, `list_pull_requests`, etc., but does not override `list_workflow_runs` (`sniff/lib/src/remote/mod.rs:257-375`). That means calls on `GitRemote` use the trait default from `sniff/lib/src/remote/provider.rs:120-126`, which always returns an empty vector. The result is that the actual `sniff repo remote` user flow always falls back to CI/CD presence detection and never renders the new `CI/CD (last N runs)` table.

This is the primary success criterion in the spec, so the feature is not production-ready.

Verification level present: Level 1 tests cover `GitHubRemote::list_workflow_runs` directly (`sniff/lib/tests/remote_providers.rs:580-620`) and fake-provider `fetch_report` selection (`sniff/lib/src/remote/provider.rs:374-421`), but there is no Level 1 test for `GitRemote::GitHub(...).fetch_report(...)` or the CLI-equivalent enum dispatch path. Add enum forwarding and a regression test that mounts a mocked Actions response, calls `fetch_report` on `GitRemote::GitHub(provider)`, and asserts `report.ci_cd` contains run-shaped entries.

### Medium: The Level 2 fixture is packaged as a real `sniff-cli` binary

The new `sniff/cli/src/bin/render_cicd_fixture.rs` helper is under Cargo's auto-discovered binary directory, so it is part of the package's binary surface. `cargo metadata` reports both `sniff` and `render_cicd_fixture` as `sniff-cli` bin targets. This is test scaffolding, but it will be built by `cargo build -p sniff-cli --bins` and can be installed/distributed as an extra executable.

That is not part of the feature design and makes the CLI package less ergonomic. Prefer moving the fixture into an integration-test helper, gating it behind a test-only feature, or making the Level 2 test invoke the main binary with a fixture mode that is not exposed in release/install builds.

Verification level present: `cargo check --color=never -p sniff-cli --bins` passed, which confirms the helper compiles as a binary target. This is a packaging/API surface concern rather than a terminal-behavior verification gap.

## Verification-Level Summary

- Stats line `⇄ N PRs`, including `0 PRs`: Level 1 renderer tests are present and appropriate for this static output requirement.
- CI/CD table content, branch/event/time, and relative-time formatting: Level 1 renderer tests are present and appropriate for the non-terminal-specific parts.
- CI/CD status glyph colors/styles: a Level 2 tmux capture test is present (`sniff/cli/tests/level2_cicd_styling.rs`) and is the right level for terminal-rendered SGR/glyph behavior.
- Actual workflow-run fetching in `sniff repo remote`: strongest meaningful coverage is incomplete because direct provider tests bypass the enum used by the CLI. This is the production-blocking gap.

## Checks Run

```text
cargo check --color=never -p sniff-cli --bins
cargo test --color=never -p sniff --features remote list_workflow_runs_success
cargo test --color=never -p sniff-cli test_render_cicd_run_shaped_table
cargo metadata --no-deps --format-version 1
```

Note: earlier attempts using multiple Cargo test filters and `--exact` ran zero tests; the focused substring-filter reruns above executed the intended tests.
