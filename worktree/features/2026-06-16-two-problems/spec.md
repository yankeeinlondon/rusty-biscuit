# Two Problems Feature

The worktree CLI's **list** command (`wt list`, the default subcommand) was
investigated for two problems. After measurement, only one is active:

1. **Slowness** — `wt list` takes ~660 ms; ~71% is one stage. **Active.**
2. **Wrong/inconsistent image aspect ratio** — appears to render correctly in
   current runs. **Parked** (see end of doc).

`--perf` (already shipped) is the measurement instrument and the diagnostic that
surfaced both.

---

## Track 1 — Slowness (active)

### Observed baseline

A representative `wt --perf` run on a feature-branch worktree of this monorepo:

```
Performance                                664.3ms  100%
├─ pre-dispatch                              8.6ms    1%
├─ list gather                             470.8ms   71%
├─ table render                              1.0ms   <1%
├─ graph gather                            111.8ms   17%
├─ graph image render (biscuit-terminal)    72.1ms   11%
└─ unattributed                             39.0µs   <1%
```

`list gather` is the target. `graph gather` is secondary. The `graph image
render` figure is a one-off rasterization cache *miss* (normally ~13 ms cached) —
inherent and out of scope. `pre-dispatch` is clap parse + `Terminal::default()`
capability detection (process startup, not git work) — out of scope.

### Root cause (measured, not assumed)

The initial assumption — that the per-worktree `git status` walk is slow — is
**wrong**. Measured on this repo (16 worktrees):

- Per-worktree `git status --porcelain` (the `dirty_status` call): **20–90 ms,
  uniformly cheap.** Not the bottleneck.
- The real cost is the **merge analysis** run for every *divergent* worktree.

`worktree::list_worktrees` (`lib/src/worktree.rs:161-216`) spawns one thread per
worktree. Within each non-main thread:

- `dirty_status` runs on a sub-thread (`git status`, cheap), **concurrently with**
- `ahead_behind` (`git rev-list --left-right --count`, ~45 ms) **→ then
  sequentially →** `check_clean_merge` (`git merge-tree --write-tree`), invoked
  only when `ahead > 0 && behind > 0` (`worktree.rs:183-187`).

In this repo **10 of 15 branches are divergent**, so all 10 run a full 3-way tree
merge against branches 300–4,575 commits behind `main`:

| op | per-call | count | ~total CPU |
| --- | ---: | ---: | ---: |
| `git status` (dirty) | 20–90 ms | 16 | ~0.6 s |
| `rev-list` (ahead/behind) | ~45 ms | 15 | ~0.7 s |
| **`merge-tree --write-tree`** | **40–132 ms** | **10** | **~0.75 s** |

~2 s of CPU work fanned across cores with disk/object-DB contention ≈ the 470 ms
wall-clock. The longest single per-thread chain is `rev-list`→`merge-tree`
(~177 ms) and sets the floor.

The slowness is therefore: **we recompute ahead/behind and a full 3-way merge for
every divergent worktree on every `wt list`, and we serialize those two calls
per thread.**

### Approach (decided: cache + concurrency)

Two prongs — a result cache for warm runs, and concurrency fixes for the cold
(cache-miss) run.

#### 1. SHA-keyed result cache (primary; warm-run win)

Cache the two expensive, deterministic results per branch:

- **Key:** `(default_tip_sha, branch_tip_sha)` plus a cache-format version.
- **Value:** `{ ahead, behind, is_clean }`.

Both `ahead_behind` and merge cleanliness are fully determined by the two commit
tips (the merge-base and both trees follow from the SHA pair), so the SHA pair is
a sound, self-invalidating key: if either tip moves, the key misses and we
recompute. No manual invalidation needed.

- **Cheap key derivation:** the `branch_tip_sha` is the worktree's `HEAD` — already
  present in `git worktree list --porcelain` output (currently skipped in
  `parse_worktree_list`, `worktree.rs:108`). The `default_tip_sha` is one
  `git rev-parse <default>`. So computing the key costs ~one extra git call total,
  not one per worktree.
- **What is NOT cached:** `dirty_status` (working-tree state). It is cheap, changes
  on every file edit, and would need mtime-based invalidation — not worth it. The
  dirty walk still runs live every time.
- **Storage:** a small JSON state file, per-repo (e.g. under the user cache dir
  keyed by repo root, or the git common dir). Stale entries for superseded SHAs
  are harmless; prune opportunistically (e.g. keep only current branches).
- **Effect:** in normal use most worktrees' tips are static between runs, so most
  entries hit and `list gather` collapses toward the live dirty walk only
  (~100 ms). Even during active development only the branch you're committing on
  (and `main`, rarely) misses.

#### 2. Concurrency fixes (cold-run win)

For genuine cache misses (first run, or after tips move):

- **Kill the per-thread `rev-list`→`merge-tree` serialization.** Run them
  concurrently and discard the `merge-tree` result if `ahead == 0 || behind == 0`
  (speculative). Most divergent branches need it anyway; collapses the ~177 ms
  chain toward `max(rev-list, merge-tree)`.
- **Overlap `graph gather` with `list gather`.** `graph gather` needs only the
  default branch + branch *names*, both available right after the cheap
  `git worktree list --porcelain` parse — before any dirty/merge work. Start it
  concurrently so its ~110 ms hides behind `list gather`.

### Scope / non-goals

- Do **not** change the `git status` dirtiness probe — it is already cheap.
- Do **not** change `merge-tree` semantics (the clean/conflict column is core).
- Do **not** cache `dirty_status`.
- Preserve existing subprocess-count guarantees (the `merge-base` / `log` /
  `rev-parse --short` count regression tests) on the cold path.

### Success criteria

- **Warm run** (tips unchanged): `list gather` drops from ~470 ms to roughly the
  live dirty-walk cost. **Target: TBD — candidate `list gather` < 150 ms warm.**
- **Cold run** (all misses): `list gather` improves via de-serialization;
  full-command improves via graph-gather overlap. **Target: TBD — candidate full
  command < 450 ms cold.**
- Output (table + graph) byte-for-byte unchanged.
- Cache correctness: a tip move (commit on a branch, or `main` advancing) yields a
  fresh, correct `(ahead, behind, is_clean)` — verified by test.
- No regression in cold-path subprocess counts.

### Test plan

- **Cache unit tests:** hit returns cached value with zero `rev-list`/`merge-tree`
  calls (recorder-backed); miss on either tip change recomputes; format-version
  bump invalidates.
- **Correctness:** on a controlled fixture, advancing a branch tip flips
  `ahead`/`is_clean` and the cache reflects it (no stale read).
- **Concurrency:** recorder assertion that `graph gather` git calls are issued
  before `list gather` completes (or that wall-clock ≈ max-stage, not sum) — a
  behavioral guarantee, not a timing-flaky check.
- **SLA guard** on a controlled fixture for warm vs cold `list gather`.
- Reuse `temp_repo_with_branches` / `temp_repo_with_feature_branch` fixtures.

### Open question

Set the numeric SLA targets (warm/cold) before implementation, or let the
cache+concurrency implementation report achieved numbers and ratify targets from
there.

---

## Track 2 — Aspect ratio (parked)

Not active work. Recorded so the investigation isn't lost if it recurs.

- **Observation history:** earlier `wt --perf` runs rendered the git-graph image
  squished/too-tall in some worktrees but correct in others (same binary).
  Current runs render correctly — the issue appears intermittent or already
  resolved. No fix is being implemented now.
- **If it recurs, the confirmed facts:**
  - Rendering path: `wt` → `biscuit_terminal::components::mermaid::MermaidDiagram`
    (display: cells→pixels via `term.cell_size()`, terminal image protocol) →
    `biscuit_visualized` (`MermaidDiagram`/`MermaidRenderer`) for SVG generation +
    rasterization. The CLI uses the existing biscuit-terminal component — nothing
    bespoke.
  - The terminal/raster path is faithful (uniform scale, aspect preserved). Any
    mis-proportion originates in `mermaid_rs_renderer::compute_layout`
    (`biscuit-visualized/src/src/mermaid/render.rs:223-225`), which can produce a
    near-square / padded canvas for small (few-commit) gitGraphs.
  - Measured cached gitGraph PNGs: broken cases were all ~square (h/w 0.9–1.3),
    small graphs; healthy graphs were wide-and-short (h/w 0.25–0.45). Branches
    further behind `main` get more horizontal commits → wider → correct, which
    explains the "further behind renders correctly more often" anecdote.
  - **Candidate fix (if needed):** in biscuit-visualized, re-fit the SVG canvas to
    its content bounding box before rasterizing — single chokepoint at
    `raster/png.rs::render_tree_to_pixmap` (covers both the at-width and
    scale rasterization entry points). Note: a content-bbox crop normalizes dead
    space but does **not** widen an intrinsically near-square small graph; making
    small graphs "always wide" is a separate layout-level change. Bump the cache
    backend id (`MERMAID_BACKEND`) on any such change.

---

## Cross-cutting notes

- `--perf` is the measurement tool; no new instrumentation needed beyond optional
  finer per-call attribution inside `list_worktrees` during development.
- Per repo drift conventions: update `worktree/README.md` if `wt list` behavior or
  the new cache file is user-visible; no crate add/remove expected, so
  `docs/dependencies.md` is unaffected (confirm if a serialization/cache crate is
  added).
