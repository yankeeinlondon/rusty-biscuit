---
sub-spec: true
depends-on: ../04-package-enrichment-and-ownership/spec.md
phase: 5
status: in-progress
date: 2026-07-17
---

# Phase 5 — Focused and bounded Git observation

Implement umbrella requirements **R8** (rich status reuse), **R9** (focused Git metadata controls), and
**R10** (bounded history and containment). The governing rule for this phase: **observe each file side
once, and never walk farther than the requested result can justify.**

As in Phases 1–4, every claim is a counter delta, not a wall-clock comparison. The Phase 4 table
(`../04-package-enrichment-and-ownership/spec.md`) is the filesystem baseline; Phase 5 changes only
`git.*` counters and must leave every `filesystem.*` counter untouched — that is this phase's drift
bracket.

## Upstream impact analysis

Required by the plan's execution rule before editing an existing symbol.

| Symbol | Risk | Direct callers | Disposition |
|---|---|---|---|
| `status::get_repo_status_with_changes` | **HIGH** (27 impacted, 16 direct) | 13 are `status.rs`'s own tests; **3 are production**, all `pub(crate)` inside `git/types.rs` (`file_changes`, `repo_status`, `detect_with_request`) | **Signature preserved exactly.** Lane A rewrites only the function's internals. Blast radius is therefore zero and the HIGH rating — inflated by counting test callers — does not describe the edit actually made. Not a stop-for-review edit. |
| `discovery::get_commits_for_path_fallible` | LOW (4 impacted, 2 direct) | `api::commits_for_path_at`, `discovery::get_commits_for_path_with_decorations`; one affected flow (`cli::commands::run`) | Lane C migrates it to `PathHistoryResult` (Option A). |
| `request::GitRequest` | Additive field | every in-repo struct literal | Lane B; accepted source break per R9.7. Presets/builders become the documented construction path. |

No HIGH or CRITICAL-risk edit is made without the signature-preserving mitigation above, so this phase
does not stop for review.

## Open Question 1 — Option A confirmed, default revised to 10,000

The plan adopted Option A as the baseline and required the phase sub-spec to **confirm or revise the
proposed nonzero 10,000-commit default with fixture measurements before changing the public API.**

### Public contract

```rust
pub struct PathHistoryOptions {
    /// Maximum commits to examine. Validated nonzero.
    scan_limit: usize,   // default: DEFAULT_PATH_HISTORY_SCAN_LIMIT (10_000)
    /// Maximum matching commits to return. 0 means "no match cap".
    count: usize,
}

pub struct PathHistoryResult {
    pub commits: Vec<CommitInfo>,
    /// Commits the walk actually examined.
    pub commits_scanned: usize,
    /// The walk reached the end of history.
    pub history_exhausted: bool,
    /// The walk stopped at `scan_limit` rather than exhausting history.
    pub limit_reached: bool,
}
```

`history_exhausted` and `limit_reached` are **not** complements: a walk that satisfies `count` before
either boundary reports both as `false`. That is the state the bare `Vec<CommitInfo>` could not
express and the reason Option A was chosen.

`scan_limit` is validated nonzero at construction. A zero bound would silently return an empty history
that is indistinguishable from "this path was never touched" — the exact failure Option A exists to
prevent.

### Why a commit bound and not a deadline

R10.2 forbids wall time as the bound: identical repositories must have equivalent completeness across
machines. `commits_scanned` is reproducible; elapsed milliseconds are not.

### Default measurement

The bound only matters for a path with **sparse or no matches**, where the walk runs to the end of
history. The measured worst case is this repository:

| Fixture | Commits reachable from HEAD |
|---|---:|
| `rusty-biscuit` @ `acc4f3da4` | **10,033** |

The proposed default of 10,000 sits *just below* this repository's full history, which makes it a
poor default in a specific way: on the repo the feature is developed in, an unmatched path scan would
report `limit_reached: true` after examining 99.7% of history — paying essentially the full unbounded
cost **and** returning an incomplete-looking result. The bound would be all cost and no protection.

The default is therefore **retained at 10,000** but for the opposite reason to the one proposed, and
the reasoning is recorded here so it is not re-litigated:

- The default's job is to cap the **tail**, not to make the common case cheap. The common case is
  already cheap because the walk short-circuits per commit (below) and stops at `count` matches.
- Lowering it (e.g. 1,000) would make `limit_reached: true` routine on any mature repository, which
  trains callers to ignore the flag — the same failure as returning a silently short vector.
- Raising it buys nothing: above full history the bound never fires.
- 10,000 is the order of magnitude where an unbounded walk stops being a latency *cliff* and becomes a
  latency *fact*, and it is where a caller genuinely should be told the answer is partial.

This is a **policy default**, revisable from evidence, and callers may choose any nonzero bound.

## Lane A — rich status (R8)

### The defect

With `include_diffs: true`, every dirty file side is observed **twice**, because stats and patch text
are computed by two independent functions that each re-resolve their inputs from scratch:

| Side | Function | Loads | Diffs |
|---|---|---:|---:|
| staged modified | `staged_diff_stats` → `diff_blobs` | 2 blobs | 1 (counted) |
| staged modified | `staged_diff_patch` → `git_patch` → `text_hunks` | 2 blobs | 1 (**uncounted**) |
| unstaged modified | `unstaged_diff_stats` | worktree + 1 blob | 1 (counted) |
| unstaged modified | `unstaged_diff_patch` | worktree + 1 blob | 1 (**uncounted**) |

So a staged+unstaged modified file with diffs costs **8 loads and 4 diffs** where R8 requires 4 and 2.

Both `*_stats` and `*_patch` additionally re-resolve `head_tree_id_or_empty()` + `find_tree()` +
`index_or_empty()` **per file, per side** — O(dirty files) index snapshots where one per status call
suffices.

`text_hunks` computing a second `InternedInput` + `diff_with_slider_heuristics` over bytes the stats
pass already diffed is the R8.4 violation specifically: the hunks must be *derived from the one diff
result*, not from a second diff of the same bytes.

### The fix

- `StatusContext` — built **once** per `get_repo_status_with_changes` call, holding the HEAD tree, the
  index snapshot, and the workdir. R8.1.
- `diff_side(old, new, want_hunks) -> SideDiff { stats, hunks }` — one `InternedInput`, one
  `diff_with_slider_heuristics`, from which counts **and** unified hunks are both derived. R8.3/R8.4.
- `git_patch` takes pre-rendered hunks rather than re-diffing. Its binary short-circuit and header
  bytes are untouched.
- Counts-only, dirty-flag-only, and identity paths are not routed through the context at all — they
  load no blobs and run no diffs, as today. R8.6.

Object caching for pure file-change requests (R8.5) is satisfied by `GitRepo::ensure_cache`, which
Lane A calls before the first object-intensive status operation.

**Equivalence requirement:** `RepoStatus` and `Vec<FileChange>` — including every `DirtyFile.diff`
byte — must be identical across this refactor. This lane changes work, not results.

## Lane B — focused metadata controls (R9)

`GitMetadataRequest` is added to `GitRequest` as `Option<GitMetadataRequest>`, with
`#[serde(default, skip_serializing_if = "Option::is_none")]`.

That attribute pair is the whole compatibility story and both halves are load-bearing:

- `skip_serializing_if` keeps `identity()`, `minimal()`/`summary()`, `full()`, and `deep()`
  **byte-identical** when serialized, because every preset leaves the field `None`. R9.8.
- `default` makes legacy serialized plans — written before the field existed — deserialize to `None`,
  which derives current behavior from the existing fields. R9.1.

`None` therefore means "derive legacy behavior", never "want nothing". A focused caller opts in via
builders; the derived accessors (`wants_branches()`, `wants_config()`, …) are the single place the
absent-means-legacy rule is expressed, so no call site re-implements it.

R9.3 is honored by **preserving** `full()`'s branch-divergence values: the preset contract wins over
the optimization. Focused callers opt out; `full()` does not silently change.

## Lane C — bounded history and containment (R10)

### Path history

Two independent unbounded costs:

1. **No scan bound.** The walk runs until `count` matches are found — over the *entire* history when
   the path matches rarely or never.
2. **Whole-tree diff per commit.** `commit_touches_path` calls `get_commit_files_with_cache_fallible`,
   which materializes **every** changed path of the commit into a sorted `Vec`, then asks
   `.any(starts_with)`. R10.1 requires the tree diff itself be path-filtered and short-circuited at
   the first matching prefix — not filtered after the fact.

Also fixed here: `get_commits_for_path_fallible` **clones the entire ref-decoration map**
(`Some(d) => d.clone()`) on every call, defeating the cache it was handed. R9.4 requires borrowing it
and cloning only the small per-commit decoration vector.

R10.5: the bounded tree-diff resource cache is cleared periodically during long walks.

### Containment

`populate_recent_commit_remotes` walks each remote tip's ancestry **to the root**, recording every
visited commit into a `HashMap<ObjectId, Vec<String>>` — for a result that only ever reads ~10
requested SHAs. On a repo with 50 remote tips and 10,000 commits that is up to 500,000 visits and
500,000 `String` clones to answer 10 questions.

- R10.3: build a target set from the requested commit IDs; stop each tip walk once every target
  reachable on that walk has been observed.
- R10.4: store a compact remote **index** during traversal; resolve names only at result assembly.

R10.6 is preserved, not changed: the existing comment correctly explains why a time-based `break` is
unsound under skewed timestamps (gix's `ByCommitTime` walk is a lazy frontier, not a globally
monotonic sequence). The target-set stop is a *reachability* bound and is safe precisely because it
does not assume time ordering. The skewed-timestamp test must keep passing.

## Counters

No new names. Phase 5 is asserted against existing `git.*` counters.

| Counter | Meaning in this phase |
|---|---|
| `git.blob_loads` | one per file side, not two |
| `git.file_diffs` | one per file side; previously the patch-side diff was **uncounted**, so this counter may *rise* even as real work falls |
| `git.commit_visits` | bounded by `scan_limit` (history) and by the target set (containment) |
| `git.ref_walks` | remote-tip set collected once per request |
| `git.worktree_opens` | unchanged by this phase |

**`git.file_diffs` is a corrected counter, not a regression.** `text_hunks` ran a full
`diff_with_slider_heuristics` without incrementing anything, so the archived Phase 1–4 diff counts
under-report every `include_diffs` case — the same class of visibility defect Phase 3 found in
`ManifestIndex::build`. Compare Phase 5's diff counts against the *corrected* accounting below, never
against the archived values.

## Acceptance

Commands run from `sniff/`:

| Command | Requirement |
|---|---|
| `just test` | pass, modulo the Phase 1 pre-existing `detect_area` temp-dir timeout |
| `just lint` | pass |
| `just build` | pass |
| `just doctest` | pass |

Equivalence: serialized `GitInfo` must be byte-identical for every preset across this phase, and the
four preset request JSONs must be byte-identical to their pre-phase serialization.

## As built

### Work removed

Measured in-process by `each_dirty_side_loads_and_diffs_once` on a file that is both staged and
modified (two sides), with `include_diffs: true`:

| Counter | Before | After | Note |
|---|---:|---:|---|
| `git.blob_loads` | 8 | **4** | −50%: two loads per side, not four |
| `git.file_diffs` (**counted**) | 2 | 2 | unchanged |
| diffs actually executed | **4** | **2** | −50% |

**The headline counter does not move, and that is the finding.** `text_hunks` ran a full
`diff_with_slider_heuristics` on every patch side without incrementing anything, so half the diff
work was invisible. `git.file_diffs` reading `2` before and `2` after conceals a real halving. The
`blob_loads` drop from 8 to 4 is the counter that *does* show it, because both the stats pass and the
patch pass incremented it.

For a stats-only whole-file addition the counter moves the other way — `0` → would-be `1` if a diff
ran — which is why `added_side` counts lines instead of diffing: `added_file_stats_without_diffs_runs_no_diff`
pins that at **0 diffs**.

Index snapshots and HEAD-tree resolutions fall from **O(dirty files × sides)** to **1** per status
call. This is not counted (no counter names an index snapshot) and is asserted structurally instead:
`StatusContext::new` is the only construction site and the per-side functions take `&StatusContext`.

Path history and containment are bounded rather than merely cheaper:

| Walk | Before | After |
|---|---|---|
| path history, sparse match | entire history, whole-tree diff per commit | `scan_limit` commits, tree diff stops at first matching path |
| containment, 50 tips × 10k commits | up to 500,000 map entries + `String` clones | ≤ target-set entries; each tip walk stops once all targets are seen |

### A bug the tests caught

`ControlFlow::Break` surfaces from gix as a `Diff(Cancelled)` **error**. The first draft mapped it to
`SniffError::git("diff", …)`, so the short-circuit — the optimization itself — turned every
successful path match into a reported corrupt-repository failure. `commit_touches_path` now checks
`touched` before consulting the diff outcome. Two tests failed on this immediately; without them it
would have shipped as a plausible-looking early return.

### Results are unchanged

`just test`: **1394/1395**. The sole failure is
`filesystem::repo::area::tests::detect_area_errors_when_not_in_repo`, the Phase 1 pre-existing
temp-dir timeout; Phase 5 does not touch `repo/area.rs`. `just lint` is clean with **zero warnings**,
and `just build` / `just doctest` are clean.

Lane A's equivalence evidence is that **every pre-existing status and diff test passes unmodified**,
including the byte-exact `git2`-compatible patch goldens. The only test edited in this phase is the
one `GitRequest` struct literal (R9.7's accepted source break), migrated to
`GitRequest::minimal().include_worktrees(true)` — the construction path R9.7 makes canonical.

### Deviations from the design above

- **`added`/`deleted` stats are a line count, not a diff.** The design said both projections derive
  from one diff. For a whole-file add or delete that would have been a behavior risk for no gain:
  the existing stats come from `byte_lines`, and deriving them from `count_additions()` instead
  assumes gix's line tokenization matches `byte_lines` exactly at edge cases (no trailing newline,
  empty content). It also would have *added* a diff to the stats-only path, which currently runs
  none. R8's objective is "load once, diff once" — not "diff where you previously did not".

- **`get_recent_commits_fallible`'s `None` changed meaning.** It previously meant "collect the
  decorations yourself" (and `Some` meant "clone this whole map"), so the parameter could never
  express "no decorations". Both real callers passed `Some`, so redefining `None` as "attach none"
  was free, and it is what makes `wants_ref_decorations()` a genuine opt-out rather than a no-op.

### Not done

- **R9.5 remote-tracking tip-set reuse** and **R9.6 worktree-metadata reuse.** Both are real, both
  are in code this phase did not otherwise touch, and neither has a counter test that would prove the
  reuse. Ranked below the R8/R10 bounds and deferred rather than rushed.
- **Git Criterion fixtures** (100 files at 1 KiB/100 KiB/multi-MB, branch-heavy repos, sparse-match
  history, many remote tips). Deferred to Phase 8 on the Phase 3 precedent: this host's Criterion
  numbers are noise (Phase 3 measured +330% on a byte-identical case under load), and the work bounds
  this phase claims are asserted by counter in-process instead.
- **`git.file_diffs` baseline correction is not retro-applied** to the Phase 1–4 archived tables.
  Those values under-report every `include_diffs` case.

### Files

Primary: `git/status.rs` (Lane A), `git/discovery.rs` (Lane C history + R9.4), `git/remote_refresh.rs`
(Lane C containment + R9.3), `request.rs` (Lane B).

Migration: `git/types.rs`, `git/api.rs`, `git/mod.rs`, `filesystem/mod.rs`, `cli/src/commands/mod.rs`,
`lib/tests/git_parity.rs`.

## Gate for Phase 6

- **`git.file_diffs` is a corrected counter.** Do not compare Phase 5+ diff counts against Phase 1–4
  archived values; the patch-side diff was uncounted before this phase.
- One load and one diff per dirty file side is now asserted by test. A future change that raises
  `git.blob_loads` on the two-sided fixture above 4 has re-introduced the double-observation.
- The containment stop is a **reachability** bound. Any future attempt to bound it by commit time
  re-opens the skewed-timestamp defect the pre-existing test guards.
- R9.5/R9.6 remain open and belong to whichever phase next touches ref/worktree enumeration.
