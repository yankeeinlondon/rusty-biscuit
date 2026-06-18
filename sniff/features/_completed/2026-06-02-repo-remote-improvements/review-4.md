---
ready: true
agent: codex
model: ""
---

# Review: `sniff repo remote` PR count + CI/CD runs

## Findings

No production-blocking findings.

The iteration-3 gap is addressed: `RemoteRepoProvider for GitRemote` now forwards `list_workflow_runs`, so the production CLI path that constructs a `GitRemote` enum can reach `GitHubRemote::list_workflow_runs` instead of always using the trait default.

One small follow-up is worth considering but does not block production: `GitHubRemote::list_workflow_runs` relies on `per_page=limit` for bounding results and does not apply the spec's final `.take(limit)` after mapping (`sniff/lib/src/remote/github.rs:561`). GitHub should honor `per_page`, and the rendering path also caps displayed rows, so this is low risk. A small regression test with an over-limit fixture would pin the invariant more directly.

## Requirement Coverage

- Data model: `CiCdInfo` includes `head_branch` and `event`, and provider presence-detection construction sites populate them with `None`.
  Verification level: Level 1 via compile coverage and provider/renderer fixture construction.

- GitHub workflow-run fetching: `GitHubRemote::list_workflow_runs` calls `/actions/runs` with `per_page`, maps name/status/conclusion/branch/event/url/timestamp, and supports missing-token anonymous fallback.
  Verification level: Level 1 via mocked provider tests in `sniff/lib/tests/remote_providers.rs`.

- Production enum dispatch: `GitRemote::GitHub(...).fetch_report(...)` includes run-shaped CI/CD entries.
  Verification level: Level 1 via `fetch_report_via_enum_includes_workflow_runs`.

- Stats line: the output relabels issues as `issues` and always includes `⇄ N PRs`, including `0 PRs`.
  Verification level: Level 1 renderer tests in `sniff/cli/src/output/remote.rs`.

- CI/CD run table: run-shaped entries render `CI/CD (last N runs)` with status, workflow name, branch, event, and relative time; presence-only entries still render the fallback list.
  Verification level: Level 1 renderer tests.

- CI/CD glyph/color rendering: success/failure/skipped/active status cells render with the expected glyphs and SGR styling through a real terminal capture.
  Verification level: Level 2 via `sniff/cli/tests/level2_cicd_styling.rs`. Level 3 is not required because this feature has no OS-keyboard-input behavior.

## Verification Run

```text
cargo test --color=never -p sniff --features remote list_workflow_runs_success
cargo test --color=never -p sniff --features remote fetch_report_via_enum_includes_workflow_runs
cargo test --color=never -p sniff-cli remote
just test-l2 level2_cicd_status_cells_render_styled_in_tmux
```

All passed. `cargo test -p sniff-cli remote` emitted two pre-existing unused-variable warnings in `sniff/cli/tests/cli.rs`; they are unrelated to this feature.
