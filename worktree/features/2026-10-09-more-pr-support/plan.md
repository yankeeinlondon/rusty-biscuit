---
total_phases: 4
created: 2026-10-09
phase: 1
agent: claude/sonnet
yolo: true
source_files_during_phase_1:
    - sniff/lib/src/remote/check_status.rs
    - sniff/lib/src/remote/focused.rs
    - sniff/lib/src/remote/blocking.rs
    - sniff/lib/src/remote/mod.rs
    - sniff/lib/tests/l1/pr_check_status.rs
    - sniff/lib/tests/l1/main.rs
    - sniff/lib/tests/fixtures/remote/graphql_check_status/success.json
    - sniff/lib/tests/fixtures/remote/graphql_check_status/passed.json
    - sniff/lib/tests/fixtures/remote/graphql_check_status/partial_errors.json
    - sniff/lib/tests/fixtures/remote/graphql_check_status/request_wide_errors.json
    - sniff/lib/tests/fixtures/remote/graphql_check_status/permission_failure.json
    - sniff/lib/tests/fixtures/remote/graphql_check_status/rate_limit.json
    - sniff/lib/tests/fixtures/remote/graphql_check_status/changed_head.json
    - sniff/lib/tests/fixtures/remote/graphql_check_status/null_rollup.json
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
    - worktree/lib/src/pull_requests.rs
    - worktree/lib/src/list/wait/tests.rs
    - worktree/cli/src/commands/list/tests.rs
    - worktree/cli/src/commands/git_graph/tests.rs
    - worktree/cli/tests/list_table.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
    - .claude/skills/worktree/list-remote.md
packages:
    - sniff
    - worktree
    - worktree-cli
---

# Plan: account for open PRs in `wt list`, and show their check status

## Summary and definition of done

`wt list` already stores the complete same-repository open-PR list and badges
only PRs whose source branch matches a worktree branch. This feature adds two
things:

1. **Accounting.** Every eligible PR no worktree table row displays becomes an
   item in the status list (ascending number, first 10, then an exact
   `and N more open PRs without a worktree row` count).
2. **Check status.** Each table badge and unmatched item carries a
   `failing` / `running` / `passed` / `unknown` marker (`✗` `…` `✓` none),
   derived from GitHub's `statusCheckRollup` count-by-state fields on the PR's
   latest source commit. The status is fetched by batched GraphQL enrichment in
   sniff's `FocusedProviderClient`, carried through `PrSummary` →
   `OpenPullRequest`, and written by the detached worker in PR store **format 7**.

Work spans three crates: `sniff` (lookup + enrichment), `worktree` (store,
matching coverage), `worktree-cli` (renderer, legend, graph height), plus docs
and skills.

**Done means:**

- [ ] A #121-style PR shows an unmatched item with its status glyph, with no
      new table row and no new graph line; table badges show the same glyphs
      on a neutral (not green) fill.
- [ ] Status is exactly as the spec's mapping table; no unknown case is green.
- [ ] One GraphQL batch for 1–100 PRs, two for 101; none for anonymous,
      unsupported, or empty answers; request-wide failure stops later batches;
      REST list is never lost to an enrichment failure or the shared 10 s deadline.
- [ ] Format 7 round-trips; formats 5 and 6 read as before with unknown status;
      4-and-older and 8-and-newer remain misses.
- [ ] Foreground makes no new request or Git process; wait budgets unchanged
      (3 s / 75 s), PR deadline still 10 s.
- [ ] `just test` and `just lint` pass in `sniff/` and `worktree/`; affected L2
      integration tests pass without stealing window focus.
- [ ] Docs and skills listed in the spec are updated in the same change.

## Phase 1: Rulings, sniff status model and enrichment

### Necessary Rules

These need an author ruling (or confirmation of the stated default) before
dependent tasks start. Defaults are what the plan assumes.

- [x] **R1 — Open questions.** Plan assumes the spec's three recommended
      options: keep 10 + exact overflow; neutral fill with contrasting text;
      latest-source-commit only.
- [x] **R2 — Neutral fill value.** The spec leaves the color to the author.
      Default: a mid-gray background (e.g. `bg-slate-700`) with default-bright
      foreground, chosen so `✗` red, `…` yellow, `✓` green stay legible on it;
      verified in light and dark terminals and with `NO_COLOR`.
- [x] **R3 — Opt-in enrichment API.** "Ordinary PR queries must not acquire
      status requests." Default: keep `open_pull_requests` / `_with` unchanged
      in cost and add an explicit opt-in (e.g. `open_pull_requests_with_status`
      or an options argument), which worktree's `SniffOpenPrs` source calls.
      `PrSummary::status` defaults to `Unknown` otherwise.
- [x] **R4 — GraphQL endpoint.** Default: github.com uses
      `https://api.github.com/graphql`; a self-hosted GitHub derives its
      validated GraphQL endpoint from the same approved API base as REST
      (`<api base>/graphql` for GHES's `/api/v3`→`/api/graphql`), else returns
      unknown. Never fall back to github.com with a self-hosted token.
- [x] **R5 — Where the mapping lives.** Default: a pure
      `status_from_rollup_counts` function in sniff (typed enum
      `PrCheckStatus { Failing, Running, Passed, Unknown }`), reused by
      worktree through `PrSummary::status`; worktree owns only its string form.
- [x] **R6 — Deadline sharing.** Default: one `Instant` deadline created in
      `run_with_deadline` and shared by REST pages and GraphQL batches via the
      existing `with_deadline`; on expiry the REST list is returned with
      unresolved statuses unknown (the outer timeout must not discard it).
- [x] **R7 — Count-field names.** The exact GraphQL field names
      (`contexts { checkRunCountsByState { state count } statusContextCountsByState { state count } checkRunCount statusContextCount }`)
      must be confirmed against GitHub's linked reference when the fixture is
      written; the saved fixture, not memory, is the contract. Not a spike:
      it is a doc read.
- [x] **R8 — Legend wording.** Default: extend the existing legend with
      `✗ checks failing`, `… checks running`, `✓ checks passed`, bare `PR #n`
      = checks unknown, shown only when a visible PR marker of that kind is
      present. Confirm exact text.
- [x] **R9 — Credential evidence for GraphQL.** Default: GraphQL requests are
      recorded in the same `SentLog`, so `RequestCredentials` folds REST +
      GraphQL (GraphQL is only sent keyed, so it can never make a keyed answer
      anonymous).
- [x] **R10 — Wider measurements.** None scheduled; the spec says no new
      performance spike or host matrix. If the author wants a benchmark of
      GraphQL batch latency on a large repository, it is a separate decision.

**Spikes:** none. The spec's figures answer the cost question (`ceil(N/100)`
requests) and the field-name question is a documentation read (R7).

### Wave 1 (parallel)

- [x] **Status type** — in `sniff/lib/src/remote/blocking.rs` add
      `PrCheckStatus` and `PrSummary::status` (default `Unknown`; `PrSummary` is
      `#[non_exhaustive]`). Document that it is the latest-source-commit
      evidence, not merge readiness.
- [x] **Rollup mapping** — in `sniff/lib/src/remote/focused.rs` (or a small
      sibling module) write the pure mapping from parsed rollup counts per the
      spec table. Failure wins over running; green needs nonzero, complete,
      consistent counts and only acceptable states; unrecognized state,
      `COMPLETED` without conclusion, missing/malformed counts, null rollup, or
      zero contexts → unknown (never an invented failure).
- [x] **GraphQL fixtures** — save response fixtures (success, partial errors,
      HTTP 200 with request-wide errors, permission failure, rate limit, changed
      head SHA, null rollup) beside the existing stand-in servers.

### Wave 2 (after Wave 1)

- [x] **Batched enrichment** — implement in `FocusedProviderClient`: only for
      GitHub, only with a selected token and a nonempty list; ≤100 aliased
      `pullRequest(number:)` lookups per batch using variables (no provider text
      interpolated); request latest commit + rollup count fields.
      - compare returned commit SHA with the REST record's `source_head_sha`
        (`PullRequestRecord`); mismatch/missing/closed/absent → that PR unknown
      - request-wide failure (HTTP error, rate limit, permission, timeout,
        malformed body, top-level `errors`) stops further batches; per-PR
        errors make only affected PRs unknown
      - same credential selection, host policy, approved endpoint, redirect
        restrictions, and `SentLog` evidence as REST (R4, R9)
      - shares the single deadline (R6)
- [x] **Public entry** — wire the opt-in (R3) through `open_pull_requests_with`
      so `PrSummary::status` is populated; ordinary callers unchanged.

### Validation checkpoint 1

- [x] `just test` and `just lint` in `sniff/`: mapping table tests (every row
      of the spec's evidence table + every unknown case), request-count tests
      (1, 100, 101 PRs; anonymous; unsupported; empty), deadline test (deadline
      after a complete REST list keeps the list), host-restriction test with an
      injected endpoint, self-hosted never contacts github.com.

#### Input robustness matrix: GraphQL response (sniff)

Load-bearing fields: `pullRequest` record, `commit.oid`, `statusCheckRollup`,
count arrays (`state`, `count`), totals. Outcome per shape — one fixture-driven
test walks every cell through the public `PrSummary::status`, with a control row
proving the unedited fixture is `Passed`.

| Shape | Outcome |
|---|---|
| field absent (rollup, counts, oid) | PR unknown; others unaffected |
| explicit `null` (rollup) | unknown (null rollup is documented unknown); `null` counts → unknown, not "zero" |
| wrong type, whole field (`counts: 123`) | unknown |
| wrong type, one element (`[{state:"SUCCESS",count:1},123]`) | unknown (cannot be green; failure/running elsewhere in the array still count if the rest is valid) |
| wrong type, every element | unknown |
| empty (`[]`, total 0) | unknown |
| duplicate key | rejected by strict parse → that response unusable → unknown |
| trailing/invalid content | whole response malformed → request-wide failure, REST list kept |

Smell grep before declaring done: `unwrap_or_default()`, `filter_map(.. as_str())`,
`#[serde(default)]`, `Option<T>` where absent ≠ null, `.ok()` on the count parse.

## Phase 2: Worktree library — status in the store

Depends on Phase 1 checkpoint.

### Wave 1 (parallel)

- [x] **Record field** — add `status` to `OpenPullRequest`
      (`worktree/lib/src/pull_requests.rs`) with a string form
      `failing|running|passed|unknown`; map from `PrSummary::status`
      in the production `OpenPrSource`.
- [x] **Format 7 writer/reader** — bump `PR_STORE_FORMAT_VERSION` 6→7; write all
      fields explicitly. Read 5 and 6 as before, forcing `unknown` status even if
      a `status` field is present (add a status-free reader path for 6, as
      format 5 has `UncredentialedStoreFile`). Formats ≤4 and ≥8 stay misses.
- [x] **Format 7 status leniency** — a missing, null, wrongly typed, or
      unrecognized `status` becomes unknown **for that PR only**; all other
      strictness (malformed JSON, duplicate keys including duplicate `status`,
      identity fields, publication, origin digest, timestamp, credentials) is
      unchanged. Use `strict_json` so duplicate `status` is still detected.

### Wave 2 (after Wave 1)

- [x] **Worker/publish path** — confirm `refresh` publishes list + statuses in
      one atomic write, empty list still publishes, enrichment failure
      publishes unknown statuses, incomplete REST publishes nothing, a later
      answer replaces old statuses, origin recheck still suppresses. Keep the
      PR lock, 10 s deadline, publication ID semantics.
- [x] **Duplicate numbers** — ensure `PrListing` consumers tolerate duplicate
      PR records (dedupe at coverage time, Phase 3).
- [x] **Library docs** — update `///` on `OpenPullRequest`, the store module doc
      and `PR_STORE_FORMAT_VERSION` history.

### Validation checkpoint 2

- [x] `just test` / `just lint` in `worktree/`. Extend
      `the_store_reader_walks_the_input_robustness_matrix` and the format-5 test;
      add format 5/6/7 status cases and unsupported formats.

#### Input robustness matrix: PR store (format 7)

| Shape | `status` | Other load-bearing fields (identity, publication, origin digest, fetched_at, credentials) |
|---|---|---|
| absent | unknown, PR kept | publication unusable (miss) |
| explicit null | unknown, PR kept | miss |
| wrong type, whole field | unknown, PR kept | miss |
| wrong type, one element (a PR's `status: 7` among valid PRs) | that PR unknown, others keep status | one PR with bad identity → miss |
| wrong type, every element | all unknown, all PRs kept | miss |
| empty (`""`, unrecognized string, `[]` PR list) | `""`/unrecognized → unknown; empty list is a valid answer | n/a |
| duplicate key | miss (including duplicate `status`) | miss |
| trailing content | miss | miss |

Formats 5/6: `status` present or not → unknown, other rules as today. Control
row: unedited fixture yields the stored status. One test per format; grep for
`#[serde(default)]` and `unwrap_or_default()` on the identity path afterward.

## Phase 3: CLI rendering — markers, neutral badge, unmatched items

Depends on Phase 2. Rendering uses biscuit-terminal `Prose` / `UnorderedList` /
`TerminalRenderable`; escape all provider text (`Prose::escape_text`). Snapshot
tests go in integration tests (`cli/tests/`), not shared-module unit tests.

### Wave 1 (parallel)

- [ ] **Marker helper** — one shared helper in `worktree/cli/src/commands/list_table.rs`
      next to `pr_badge`: glyph prefix + optional color from status, and a
      named neutral badge style constant replacing
      `bg-emerald-800` (R2). Used by both table badges and unmatched items.
- [ ] **Table coverage** — compute the set of PR numbers actually placed on a
      worktree row, by walking the same row rules that place badges in the
      default-target cell, parent cell, and beside the branch name. Parent-only
      rows, bare branch refs, detached/unusable-branch rows, and graph tags do
      **not** count as coverage. Must be derived from the real placement walk,
      not a parallel re-implementation of matching.
- [ ] **Unmatched selection** — from `PrListing`, eligible = both source repo
      identities present and equal (`for_branch`'s comparison); drop covered
      numbers; dedupe by number; sort ascending; take 10; overflow `N` =
      remaining distinct eligible. Forks and unknown-source PRs never counted.

### Wave 2 (after Wave 1)

- [ ] **Status list items** — extend `render_status` / `pr_status_markup`
      order: existing age/failure item, unmatched items + overflow line,
      refresh hint. List is nonempty when any unmatched PR exists. Format
      `- ✗ PR #121 fix/windows-mega-fix → main  (no worktree row)`;
      link only with a URL and OSC 8; branch names always visible. Cleared by
      an empty successful answer; absent for ignored/unsupported/miss; kept with
      the age/failure item for a stale answer; dropped on origin change.
- [ ] **Legend** — extend `legend_markup` per R8, only when a matching marker
      is visible; unknown = bare `PR #n`; keep glyphs distinct from the current
      worktree circles.
- [ ] **Graph height** — account for wrapped status items in the height
      reservation (`max_graph_rows` at `list.rs`); graph tags and lines are
      unchanged; all output remains on stderr, wrapper stdout contract intact.

### Validation checkpoint 3

- [ ] Integration/snapshot tests (`cli/tests/list_table.rs`, `list_prs.rs`):
      #121 case; all three placements; several PRs on one branch with different
      targets; fork/unknown-source exclusion (including a fork branch sharing a
      local branch name); duplicate numbers; PR already tagged on a graph line;
      0/1/10/11 unmatched with an unrelated badge and an excluded fork;
      long/markup-like branch names; OSC 8 on/off; missing URL; light/dark;
      color/no-color; graph height reservation. Any terminal L2 coverage uses
      non-focusing backends only.

## Phase 4: Notices, docs, and end-to-end validation

### Wave 1 (parallel)

- [ ] **Keyless clause** — in the keyless notice add
      `An API key is also required to fetch PR check statuses.` only for GitHub
      when an anonymous PR answer from this run was accepted
      (`observed_keyless` / worker-recorded evidence; an anonymous head check
      alone does not qualify). Preserve warning precedence; other providers
      keep their text. Tests: `list::tests::observations::keyless::*`,
      `list_prs::*keyless*`.
- [ ] **Worker/store e2e** — through the worker/store path: successful status
      replacement, successful empty publication, stale-answer fallback, API
      suppression (`--ignore-api`), origin change, publication-ID recognition
      without waiting for a receipt. Reuse existing request counters and
      foreground Git-call assertions to prove no foreground request or Git
      process was added; preserve 3 s / 75 s / 10 s and lock contention.
- [ ] **Docs** — `worktree/docs/cli/list.md` (scope, markers, legend, neutral
      style, status rules, unmatched items/overflow, keyless clause, cached
      status age; graph keeps existing tags — no feature/fix names or paths in
      docs); `worktree/README.md`; `sniff/docs/cli/repo_remote.md` + public API
      comments + affected sniff topic docs. Write for a reader with no repo
      experience, with a compact example per rule and a Mermaid diagram for the
      fetch/publish flow.
- [ ] **Skills** — `.claude/skills/worktree/list.md` and `list-remote.md`
      (format 7, legacy readers, optional enrichment, displayed-PR accounting,
      marker rendering) and `.claude/skills/sniff/remote-and-repository.md`
      (focused lookup status + request-evidence contract).

### Wave 2 (after Wave 1)

- [ ] **Drift pass** — review `///`/`//!` and inline comments in every touched
      symbol; fix drift toward the code. Check `docs/dependencies.md` — no
      crates should be added.
- [ ] **Smell greps** — the matrix smell list over both readers.

### Final validation checkpoint

- [ ] `just test`, `just lint` in `sniff/` and `worktree/`; `just test-l2` in
      `worktree/` for the touched integration tests; no window focus gained.
- [ ] `just ci-local --plan` reviewed before pushing.
- [ ] Do **not** move the feature to `_completed` or run `just complete`; the
      terminal state is "implementation complete, ready for review". Do not run
      `cargo fmt` or commit unless instructed.
