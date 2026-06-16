# Two Problems Feature

The worktree CLI's **list** command (`wt list`, the default subcommand) has two
independent problems:

1. It's too slow.
2. It renders git-graph images at the wrong aspect ratio — and inconsistently,
   from the *same* binary, depending on which worktree it's run in.

These are tracked as **two independent workstreams in one spec**. They touch
different code (worktree-cli git orchestration vs. biscuit-visualized SVG
sizing), have no ordering dependency, and may land and ship separately.

The `--perf` flag (already shipped) is the measurement instrument for Track 1
and the diagnostic that surfaced both problems. Reference screenshot:
`wt --perf` (incorrect aspect ratio, with the perf breakdown) vs. `wt` in a
different worktree (correct aspect ratio).

---

## Track 1 — Slowness

### Observed baseline

A representative `wt --perf` run on a feature-branch worktree of this monorepo:

| Stage | Time | Share |
| --- | ---: | ---: |
| pre-dispatch | 12.3 ms | 2% |
| **list gather** | **421.7 ms** | **67%** |
| table render | 30.0 ms | 5% |
| **graph gather** | **156.7 ms** | **25%** |
| graph image render (biscuit-terminal) | 13.1 ms | 2% |
| unattributed | 49.0 µs | <1% |
| **total** | **633.9 ms** | 100% |

The two "gather" stages are ~92% of wall-clock and are the focus.

### What each stage is (resolves the "what is pre-dispatch?" question)

The pipeline is sequential in `cli/src/commands/list.rs::run_pipeline`.

- **pre-dispatch** — everything *before* the pipeline begins: `clap`
  completion-env check + `Cli::parse()` + the `Terminal::default()` capability
  probe constructed in `list::run`. It is **not** a git/worktree stage; it is
  process startup overhead. The bulk is terminal capability detection. Minor
  and largely unavoidable; out of scope except to document.
- **list gather** — `worktree::list_worktrees()`
  (`lib/src/worktree.rs:161`). Already one-thread-per-worktree, and within each
  thread `dirty_status` runs concurrently with `ahead_behind`. The dominant
  cost is per-worktree `git status --porcelain` (`dirty_status`,
  `worktree.rs:280`) walking a large monorepo checkout, across ~18 worktrees.
- **table render** — builds + renders the status table to stderr.
- **graph gather** — git work for the graph image. On a feature branch:
  `git_graph::gather_branch` (`cli/src/commands/git_graph.rs:73`) — one
  `merge-base` + three **sequential** `git log` queries. On `main`:
  `gather_base_graph` — already parallel per-branch.
- **graph image render** — `MermaidDiagram::render`. The 13.1 ms here is a cache
  hit; a cache *miss* (rasterization) would be far larger. See Track 2.

### Root cause

Two structural inefficiencies, neither about a single slow git call:

1. **The two big stages run sequentially even though `graph gather` does not
   depend on `list gather`'s expensive part.** `graph gather` needs only the
   default branch and the worktree *branch names* — both available immediately
   after the cheap `git worktree list --porcelain` parse, *before* any
   dirty-status work. Today we fully finish `list gather` → render table →
   *then* start `graph gather`.
2. **`gather_branch`'s three `git log` queries are sequential** despite being
   independent once the `merge-base` is known. In a large repo each git
   invocation is tens of ms, so this serialization is most of the 156 ms.

### Approach (decided: overlap + parallelize, structural)

Restructure the pipeline so the two independent git-work stages overlap, and
parallelize the independent work inside graph gathering. Do **not** change the
`git status` dirtiness probe in this track (the 421 ms `list gather` floor is
the harder, riskier lever; deferred — see Non-goals).

1. **Stage overlap.** Split `list_worktrees()` so the cheap
   `git worktree list --porcelain` parse + `default_branch()` resolution happen
   first, then run two things concurrently:
   - per-worktree dirty/ahead-behind status gathering (the current 421 ms), and
   - `graph gather` (which now has everything it needs).

   The table still renders as soon as status gathering completes; the graph
   renders as soon as graph gathering completes. Expected wall-clock: the graph
   gather (~156 ms) hides behind list gather (~421 ms).
2. **Intra-`gather_branch` parallelism.** After `merge-base` resolves, run the
   `default_context`, `default_after_base`, and `branch_after_base` log queries
   on concurrent threads (mirror the `std::thread::scope` pattern already used
   in `gather_base_graph`). Preserve the single-`merge-base` guarantee that
   existing tests assert.

Both changes must preserve the existing subprocess-count guarantees (the
`merge-base`-counting and `rev-parse --short`-counting regression tests).

### Success criteria

- Full `wt list` warm-cache wall-clock on this monorepo drops materially from
  the ~634 ms baseline. **Target: TBD — set a numeric SLA before implementation
  (candidate: < 450 ms warm, i.e. graph gather fully hidden).**
- `graph gather` for a feature branch drops from ~157 ms toward ~`merge-base` +
  one parallel-log round (candidate: < 80 ms).
- No regression in subprocess counts: existing `merge-base` / `log` /
  `rev-parse --short` count assertions still pass.
- Output (table + graph) is byte-for-byte unchanged from the sequential version.

### Test plan

- Extend the existing `--perf`-backed and `recorder`-backed tests
  (`list.rs` / `git_graph.rs` test modules) with a wall-clock SLA guard for the
  overlapped pipeline on a controlled fixture.
- Add a recorder assertion that graph-gather git calls are issued *before*
  status-gather completes (or simply that total wall-clock ≈ max(stages), not
  sum) — phrase as a behavioral guarantee, not a timing-flaky check.
- Reuse `temp_repo_with_branches` / `temp_repo_with_feature_branch` fixtures.

---

## Track 2 — Wrong / inconsistent aspect ratio

### Observation (and why it is per-worktree)

The same compiled `wt` binary renders the git graph correctly in some worktrees
and squished/too-tall in others. The anecdote "worktrees further behind main
render correctly more often" is **real and now explained** (see root cause).

### Evidence

The git graph is a Mermaid `gitGraph` → SVG (`mermaid_rs_renderer`) → PNG
(resvg, `biscuit-visualized/src/src/raster/png.rs`) → inline terminal image.
Measuring the dimensions of the globally-cached gitGraph PNGs
(`$TMPDIR/biscuit-visualized/v1/mermaid/png/`) shows almost all graphs are
wide-and-short (height/width ≈ 0.25–0.45), with a tight cluster of broken
outliers — **all 320 px wide (= 160 px SVG before the 2× raster scale), with
ratio 0.92–1.33 (square or taller-than-wide):**

```
320 x 427  ratio 1.33
320 x 374  ratio 1.16
320 x 363  ratio 1.13
320 x 301  ratio 0.94
320 x 297  ratio 0.92  (×3)
```

### Root cause

A `gitGraph` lays commits left-to-right; the vertical extent (branch lanes +
labels) is roughly fixed, while horizontal extent scales with commit count.
`mermaid_rs_renderer::compute_layout` (`mermaid/render.rs:223-225`) appears to
clamp small graphs to a **minimum canvas width (~160 px SVG)** while keeping the
fixed vertical extent, yielding a near-square canvas with the actual commits
bunched in the **top-left** and large empty space — exactly the left screenshot.

The terminal display path is **not** at fault: resvg rasterizes with a uniform
scale (`png.rs:64-68`, `render_tree_to_pixmap`), and the Kitty width-only path
preserves the PNG's aspect ratio. The image faithfully reproduces a
mis-proportioned SVG.

Why per-worktree, same binary: the graph *content* differs per worktree.
- On a feature branch, `worktree_graph` shows `main`-after-merge-base commits
  (up to 5) + branch commits. A branch **further behind main** has more
  `main`-after-base commits → more horizontal commits → wider canvas → correct.
  A branch close to main → few commits → hits the min canvas → square/wrong.
- Hence "further behind main renders correctly more often." Confirmed.

The dimensions originate upstream in `mermaid_rs_renderer`; **biscuit-visualized
does not control `compute_layout`** and rasterizes whatever canvas the SVG
declares.

### Approach (decided: fix in biscuit-visualized)

Since biscuit-visualized cannot change `compute_layout`, the fix is to
**re-fit the SVG canvas to its actual content bounding box before
rasterizing**, so the rasterized aspect ratio always matches the drawn content
regardless of any upstream minimum-canvas padding.

Two candidate mechanisms (implementation to pick, both inside biscuit-visualized):

- **A — Raster-boundary crop (preferred).** In the raster layer, after parsing
  with usvg, query the content bounding box (e.g. `tree.root()` layer/abs
  bounding box) and size the pixmap to that bbox (plus a small uniform margin),
  translating the render transform by `-bbox.origin`. General, robust, no SVG
  string surgery. Affects all diagram types.
- **B — SVG `viewBox`/`width`/`height` rewrite.** Post-process the SVG string
  (as `apply_svg_overrides` already does) to tighten the declared canvas. More
  fragile; keeps a single SVG artifact whose dimensions are correct for any
  later consumer.

Either way:
- Apply a small, uniform margin so the crop does not clip strokes/labels.
- This changes rendered output for existing diagrams → **bump the cache backend
  identifier** (`MERMAID_BACKEND` in `cache/file_cache.rs`) so stale cached PNGs
  are not reused, and **purge or invalidate** the existing cache for this key
  family.

### Scope / non-goals

- Fix lives in **biscuit-visualized** (cross-package change; consumers include
  biscuit-terminal and therefore `wt`). The `wt`/worktree-cli side
  (`default_graph_width`, `MermaidDiagram::with_width`) is **not** changed in
  this track — it was a candidate workaround, explicitly not chosen.
- We are **not** modifying the upstream `mermaid_rs_renderer` crate.
- Non-gitGraph diagram types must be verified for no visual regression (the crop
  must be content-faithful, not gitGraph-specific).

### Open question to resolve during implementation

Confirm the minimum-canvas behavior is in `mermaid_rs_renderer::compute_layout`
(vs. `LayoutConfig::default()` being tunable). If `LayoutConfig` exposes a
relevant minimum-width / padding knob, adjusting it may be simpler than a
bbox crop — but the bbox crop is the robust, content-agnostic fix and is the
default plan.

### Success criteria

- A `gitGraph` with few commits (e.g. 2–3) rasterizes to a wide-and-short PNG
  with height/width comparable to many-commit graphs (no near-square / >0.9
  ratio outliers), and the commits fill the canvas (no large top-left dead
  space).
- The same `wt` binary produces visually consistent aspect ratios across
  worktrees regardless of commit count / behind-ness.
- No visual regression for other diagram types (flowchart, pie, sequence, etc.).
- Cache key bumped; no stale-PNG reuse across the change.

### Test plan

- biscuit-visualized: a raster/sizing test asserting that a minimal `gitGraph`
  (2 commits, 2 branches) produces a PNG whose height/width ratio is below a
  threshold (e.g. < 0.6) and that content bbox ≈ canvas (within margin).
- Regression: a multi-commit `gitGraph` and at least one non-gitGraph diagram
  produce dimensions consistent with pre-change content bounds (allowing for the
  margin), proving the crop is faithful.
- Cache test: changed backend id yields a fresh cache key (no reuse of
  pre-change artifacts).

---

## Cross-cutting notes

- `--perf` is the measurement tool for Track 1 and the evidence source that
  surfaced both problems; no new instrumentation needed.
- Per repo drift conventions: update biscuit-visualized README / docs if the
  rasterization/sizing contract changes, and `docs/dependencies.md` is unaffected
  (no crate add/remove expected).
