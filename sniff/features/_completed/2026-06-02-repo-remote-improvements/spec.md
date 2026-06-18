# `sniff repo remote` — PR count + recent CI/CD runs

**Date:** 2026-06-02
**Area:** `sniff` (lib + cli)
**Command:** `sniff repo remote`

## Problem

The `sniff repo remote` output has two gaps:

1. **Open PR count is never surfaced.** The stats line shows stars, forks, and
   open issues, but no pull-request count. The `Pull Requests` section is
   hidden entirely when the open-PR list is empty, so a repo with zero open PRs
   shows nothing about PRs at all.
2. **CI/CD is detection-only.** `detect_cicd` checks for the *presence* of
   `.github/workflows/` and hardcodes `status: "detected"`. It never reports
   actual workflow runs, even though the schema and `CiCdInfo` type are already
   shaped to carry them.

## Goal

- Always show an open-PR count in the stats line (including `0`).
- Show the most recent CI/CD workflow runs (any branch) as a compact table.

Success: running `sniff repo remote` against a GitHub repo shows `… ⇄ N PRs`
in the stats line and a `CI/CD (last N runs)` table with per-run
conclusion, workflow name, branch, event, and relative time. Non-GitHub
providers and repos without runs are unaffected (graceful fallback).

## Existing building blocks (no schema work required)

- `schematic_schema::github::ListWorkflowRunsRequest`
  → `WorkflowRunsResponse { workflow_runs: Vec<WorkflowRun> }`, re-exported via
  `schematic_schema::github::*`. `Response = WorkflowRunsResponse` is already
  wired (`schematic/schema/src/github/requests.rs:1238`).
- `WorkflowRun` fields used: `name`, `status`, `conclusion`, `event`,
  `head_branch`, `html_url`, `created_at`.
- `CiCdInfo` already has `provider`, `config_path`, `name`, `status`,
  `conclusion`, `html_url`, `started_at`.
- `fetch_report` already fetches open PRs into `report.pull_requests`
  (`PullRequestState::Open`).
- `chrono = "0.4"` is already a dependency of both `sniff/lib` and `sniff/cli`.

## Design

### 1. Data model — `sniff/lib/src/remote/types.rs`

Add two optional fields to `CiCdInfo` (additive, serde-compatible):

```rust
/// The branch or tag ref that triggered the run.
pub head_branch: Option<String>,
/// The event that triggered the run (e.g. "push", "pull_request").
pub event: Option<String>,
```

All four existing `CiCdInfo` construction sites
(`github.rs`, `gitlab.rs`, `gitea.rs`, `bitbucket.rs` `detect_cicd`) gain
`head_branch: None, event: None`.

### 2. Library — fetch workflow runs

**`sniff/lib/src/remote/provider.rs`**

New trait method with a default implementation (non-breaking for the three
non-GitHub providers):

```rust
/// List recent CI/CD workflow runs, most recent first.
///
/// Default implementation returns an empty list for providers that do not
/// implement workflow-run inspection.
async fn list_workflow_runs(
    &self,
    _owner: &str,
    _repo: &str,
    _limit: usize,
) -> Result<Vec<CiCdInfo>, SniffError> {
    Ok(Vec::new())
}
```

Update `fetch_report` CI/CD population:

```rust
let runs = self
    .list_workflow_runs(owner, repo, 5)
    .await
    .unwrap_or_default();
let ci_cd = if runs.is_empty() {
    // Fall back to presence detection (configured-but-no-runs, or non-Actions).
    self.detect_cicd(owner, repo).await.unwrap_or(None).into_iter().collect()
} else {
    runs
};
```

Note: in the common case (a repo with runs) this makes one `actions/runs`
call and skips the two tree fetches `detect_cicd` previously always issued.

**`sniff/lib/src/remote/github.rs`**

- Extract a pure, unit-testable mapping function:

  ```rust
  fn map_workflow_run(run: WorkflowRun) -> CiCdInfo {
      CiCdInfo {
          provider: "GitHub Actions".to_string(),
          config_path: None,
          name: run.name.unwrap_or_else(|| "workflow".to_string()),
          status: run.status.unwrap_or_default(),
          conclusion: run.conclusion,
          html_url: run.html_url,
          started_at: run.created_at,
          head_branch: run.head_branch,
          event: run.event,
      }
  }
  ```

- Implement `list_workflow_runs` using
  `ListWorkflowRunsRequest::new(owner, repo).with_per_page(limit as i64)`,
  reusing the existing authenticated → anonymous fallback pattern from
  `list_pull_requests` (retry on `MissingCredential` / `AuthenticationRequired`).
  Map results with `map_workflow_run`, `take(limit)`.

### 3. CLI rendering — `sniff/cli/src/output/remote.rs`

**Stats line** (`render_remote_text`):

- Relabel the issues stat `◎ N open` → `◎ N issues`.
- Append a PR count from `report.pull_requests.len()`, always rendered
  (including `0`):

  ```
  ★ 2  ⑂ 0  ◎ 0 issues  ⇄ 0 PRs
  ```

  PR glyph: `⇄` (dim).

**CI/CD section** (`render_cicd`):

- When entries are run-shaped (`started_at.is_some()`), render a table —
  columns: conclusion glyph, workflow name, branch, event, relative time.
  Heading: `CI/CD (last N runs)`.
- Otherwise (presence-only fallback / non-Actions providers), keep the
  existing bulleted presence list unchanged.

Conclusion glyph mapping:

| conclusion / status | glyph | style |
|---------------------|-------|-------|
| `success`           | `✓`   | green |
| `failure`           | `✗`   | red   |
| `cancelled`, `skipped` | `⊘` | dim   |
| in-progress / queued (no conclusion) | status text | dim |

**Relative time** — new pure helper with injected `now` for deterministic tests:

```rust
fn relative_time(iso: &str, now: DateTime<Utc>) -> String  // e.g. "8m ago"
```

Render path passes `Utc::now()`; tests pass a fixed instant.

## Tests

- **lib** (`github.rs`): `map_workflow_run` maps name/status/conclusion/branch/event
  correctly, including `None` fallbacks.
- **cli** (`output/remote.rs`):
  - `relative_time` with a fixed `now` (minutes / hours / days).
  - Stats line includes `PRs` and shows `0 PRs` when the PR list is empty.
  - CI/CD table renders the conclusion glyph, workflow name, and branch for
    run-shaped `CiCdInfo` entries.
  - Presence-only fallback still renders the bulleted list.

## Out of scope

- Increasing PR pagination beyond GitHub's default 30. The PR list display
  already caps at 10; the stats count is accurate up to 30, which suffices for
  this repo. Documented as a known cap.
- Workflow-run fetching for GitLab / Gitea / Bitbucket — the default empty
  trait impl leaves them unchanged; can be added later.

## Known caveats (documented, not fixed)

- GitHub's `open_issues_count` (the `◎` number) includes pull requests, so on
  repos with open PRs the issues count overlaps the PR count. The number shown
  is GitHub's canonical repo metadata value.
- The open-PR count is derived from the fetched open-PR list and is therefore
  capped at GitHub's default page size (30).
