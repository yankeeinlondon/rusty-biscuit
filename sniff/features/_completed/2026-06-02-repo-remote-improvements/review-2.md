---
ready: false
agent: codex
model: ""
---

# Review: `sniff repo remote` PR count + CI/CD runs

## Findings

### High: CI/CD run status rendering does not meet the specified glyph/style contract

The spec defines user-visible conclusion rendering:

- `success` -> `✓`, green
- `failure` -> `✗`, red
- `cancelled`, `skipped` -> `⊘`, dim
- in-progress / queued with no conclusion -> status text, dim

The implementation returns unstyled raw strings and uses different output for several cases:

- `sniff/cli/src/output/remote.rs:263`
- `sniff/cli/src/output/remote.rs:272`

`success` and `failure` are not wrapped with green/red rendering markup, `cancelled` is not dimmed, `skipped` renders `⊝` instead of the required `⊘`, and active runs render symbolic glyphs (`⟳`, `◌`) instead of dim status text. The Level 1 tests currently lock in those mismatches:

- `sniff/cli/src/output/remote.rs:1035`
- `sniff/cli/src/output/remote.rs:1053`
- `sniff/cli/src/output/remote.rs:1077`

This is a direct user-observable behavior gap. Update `cicd_conclusion_glyph` (or replace it with a richer status-cell helper) so the returned table cell includes the specified glyph/text and `biscuit-terminal` markup for color/dim styling.

Verification level present: Level 1 unit tests on raw glyph strings. Because the spec includes specific colors/styles in terminal output, this needs Level 2 real-terminal capture coverage for at least success, failure, skipped/cancelled, and queued/in-progress rows. Per the requested rigor rules, Level 1-only verification is not sufficient for this styled terminal-output requirement.

### Medium: GitHub workflow-run request construction and fallback remain untested

The feature now has useful tests for `RemoteRepoProvider::fetch_report` selection/fallback with a fake provider, and `map_workflow_run` is covered. What is still missing is a test that exercises `GitHubRemote::list_workflow_runs` itself:

- `sniff/lib/src/remote/github.rs:537`
- `sniff/lib/src/remote/github.rs:543`
- `sniff/lib/src/remote/github.rs:549`

That method is where the actual GitHub endpoint, `per_page` query parameter, response deserialization, and authenticated-to-anonymous fallback are wired. A mocked `wiremock` test should verify that `GET /repos/{owner}/{repo}/actions/runs?per_page=5` is issued, maps returned runs into `CiCdInfo`, and retries anonymously on missing/auth-required credentials. The fake provider tests in `sniff/lib/src/remote/provider.rs` do not catch regressions in request construction or schematic response wiring.

Verification level present: Level 1 unit/fake-provider tests. Level 1 mocked HTTP coverage is the appropriate minimum for this requirement and is currently missing.

### Low: Workflow-run fallback values do not match the specified mapper

The spec's mapper uses `run.name.unwrap_or_else(|| "workflow".to_string())` and `run.status.unwrap_or_default()`. The implementation emits `"Unknown"` and `"unknown"` instead:

- `sniff/lib/src/remote/github.rs:238`
- `sniff/lib/src/remote/github.rs:239`

This is a small user-visible mismatch for runs with omitted names/statuses. Either align the implementation with the spec or update the spec if `"Unknown"` is intentional.

## Test Rigor

No Level 3 requirements apply: this feature has no keyboard, paste/IME, mouse, or terminal input-encoder behavior.

The PR count, relative-time helper, presence-only fallback, and unstyled table content are adequately testable at Level 1, and the current Level 1 tests cover the previously reported `0 PRs`, heading count, and compact relative-time issues.

The CI/CD status colors/styles are terminal-rendered user-visible behavior, so the strongest current coverage is at the wrong level. Add Level 2 capture tests or remove styling from the spec before calling this production-ready.

## Validation

I ran:

```bash
cargo test --color=never -p sniff-cli remote
cargo test --color=never -p sniff --features remote test_fetch_report
cargo test --color=never -p sniff --features remote test_map_workflow_run
```

Result: all passed.

I also ran:

```bash
cargo test --color=never -p sniff --features remote --test remote_providers github
```

Result: failed in existing GitHub fixture tests with `MissingCredentials` in several non-workflow paths (`list_documents_success`, `list_issues_success`, `detect_cicd_github_actions`, `get_tags_and_releases_success`, and two error-mapping tests). I did not treat that as a feature-specific finding, but it means the broader GitHub remote-provider suite is not green in this non-interactive environment.
