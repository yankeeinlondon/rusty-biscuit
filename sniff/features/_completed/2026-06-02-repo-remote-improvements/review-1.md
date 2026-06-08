---
ready: false
agent: codex
model: ""
---

# Review: `sniff repo remote` PR count + CI/CD runs

## Findings

### High: `0 PRs` is not rendered, contrary to the primary success criterion

The spec requires the stats line to always show an open-PR count, including zero:

- Goal: "Always show an open-PR count in the stats line (including `0`)."
- CLI rendering: "Append a PR count from `report.pull_requests.len()`, always rendered (including `0`)."

The implementation only appends the PR stat when the fetched PR list is non-empty:

- `sniff/cli/src/output/remote.rs:62`

```rust
if !report.pull_requests.is_empty() {
    stats_parts.push(format!(
        "<dim>⇄</dim> {} PRs",
        format_number(report.pull_requests.len())
    ));
}
```

The added unit test locks in the incorrect behavior:

- `sniff/cli/src/output/remote.rs:1003`

```rust
fn test_render_remote_text_no_pr_count_when_empty() {
    let report = make_test_report();
    let rendered = render_remote_text(&report, None);
    assert!(!rendered.contains("⇄"));
}
```

This is a direct functionality gap in the feature's main user-facing requirement. The test should expect `⇄ 0 PRs`, and the renderer should push the PR stat unconditionally.

Verification level present: Level 1 renderer unit test, but it verifies the wrong behavior. For this text-output requirement, Level 1 is sufficient once corrected; no Level 2/Level 3 terminal validation is required by the spec.

### Medium: CI/CD table heading omits the run count required by the spec

The spec calls for a heading of `CI/CD (last N runs)` when workflow-run entries are rendered. The implementation always renders the generic `CI/CD` heading before deciding whether it is rendering run-shaped data or the presence-only fallback:

- `sniff/cli/src/output/remote.rs:203`

```rust
writeln!(out).unwrap();
write!(out, "{}", Prose::new("<b><u>CI/CD</u></b>").display(term)).unwrap();
```

That means a repo with runs does not surface whether the table is showing the last 1, 5, or 10 runs. This is smaller than the missing `0 PRs` issue, but it is still a designed user-facing output requirement that was not implemented.

Verification level present: Level 1 renderer tests check only that the generic `CI/CD` heading appears. They do not verify `CI/CD (last N runs)`. Level 1 is sufficient for this output requirement once the expected heading is asserted.

### Medium: The CI/CD run fetch path has no provider-level test coverage

The spec asks for GitHub workflow runs to be fetched with `ListWorkflowRunsRequest`, authenticated-to-anonymous fallback, and mapping via `map_workflow_run`. The implementation adds mapper unit tests, but there is no test that exercises `GitHubRemote::list_workflow_runs` or `RemoteRepoProvider::fetch_report` selecting runs before falling back to `detect_cicd`.

Relevant implementation:

- `sniff/lib/src/remote/github.rs:537`
- `sniff/lib/src/remote/provider.rs:173`

The risk is that request construction, fallback behavior, and aggregation can regress while the mapper tests still pass. Existing remote provider fixtures already cover provider behavior elsewhere; this feature should add a mocked GitHub workflow-runs response test or a fake `RemoteRepoProvider` test for `fetch_report` proving:

- non-empty workflow runs populate `RemoteReport.ci_cd`,
- empty workflow runs fall back to presence detection,
- workflow-run request failures fall back gracefully as designed.

Verification level present: Level 1 mapper tests only. For API aggregation and fallback behavior, Level 1 mocked/provider tests are the appropriate minimum and are currently missing.

### Low: Relative time output is not the compact format described in the spec

The spec's helper example uses compact relative time such as `"8m ago"`, and the table is described as compact. The implementation emits long phrases like `"5 minutes ago"`:

- `sniff/cli/src/output/remote.rs:441`
- `sniff/cli/src/output/remote.rs:455`

This may be acceptable if intentionally chosen, but it does not match the written spec and will make the CI/CD table wider than designed. Either update the implementation/tests to compact units (`8m ago`, `2h ago`, `3d ago`) or revise the spec before accepting the change.

Verification level present: Level 1 helper tests validate the long format. Level 1 is enough for this pure helper, but the expected strings should align with the intended UX.

## Test Rigor

No requirements in this feature involve keyboard input, modifier visibility, paste/IME, mouse behavior, scrolling, or terminal-emulator input encoding, so Level 3 is not applicable.

The user-observable terminal output requirements are plain rendered text and table layout. The current Level 1 renderer tests are an acceptable minimum for the stats line and helper formatting, but they need to assert the specified behavior. If exact glyph width, fallback rendering, or SGR styling becomes part of the acceptance criteria, add Level 2 real-terminal capture tests.

## Validation

I ran:

```bash
cargo test --color=never -p sniff github::tests::test_map_workflow_run
```

Result: passed. This compiled `sniff` and ran the two workflow mapper tests.

I also ran:

```bash
cargo test --color=never -p sniff-cli remote
```

Result: passed. This includes `test_render_remote_text_no_pr_count_when_empty`, which currently passes because it asserts the spec-incompatible behavior called out above.
