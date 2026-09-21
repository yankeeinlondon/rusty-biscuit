---
kind: spike-record
feature: 2026-09-19-direct-cell-execution
created: 2026-09-20
plan_phase: 1
status: complete
---

# S3 — Offline capacity and budget model

Purely offline: every plan below is one of the eight corpus documents in
`plans/` (recipes in `plans/README.md`), captured at tree head `1266e5fc9`.
Row-set definitions follow the design: **test** = executing cells with gate
in `{L1, L2, browser}` on a native environment; **wsl** = the same on
`wsl2-ubuntu`; **check** / **lint** = executing cells of that gate. Rows are
serialized as R1 orders them — `[package, gate, environment, runner]`, runner
from the plan's environment table (`wsl2-ubuntu` → `windows-latest`).

## Per-plan capacity

| Plan | Executing cells | Areas w/ rows | Largest row set (area) | Largest per-area payload | Total row-set payload | Builds | `job_estimate` |
|---|---:|---:|---|---|---:|---:|---:|
| `all` | 374 | 29 | 21 (homelab, native test) | 2,140 B | 21,783 B | 204 | 440 |
| `dispatch` | 374 | 29 | 21 (homelab) | 2,140 B | 21,783 B | 204 | 440 |
| `nightly` | 66 | 28 | 7 (homelab) | 452 B | 4,699 B | 66 | 132 |
| `pr` | 4 | 1 | 2 (biscuit-hash) | 257 B | 257 B | 2 | 4 |
| `all-reused` | 71 | 29 | 7 (homelab) | 528 B | 5,204 B | 3 | 71 |
| `mixed` | 308 | 29 | 21 (homelab) | 1,729 B | 18,232 B | 204 | 308 |
| `prohibited` | 308 | 29 | 21 (homelab) | 1,729 B | 18,232 B | 204 | 308 |
| `gap-only` | 0 | 0 | — | — | 0 B | 0 | 22 |

Against GitHub's published limits — 256 jobs per matrix, ten nesting levels,
50 unique reusable workflows per top-level caller — the worst case anywhere in
the corpus is **21 of 256 rows** in one matrix and a **4-level** chain using
**3** called workflows. Capacity is not a design constraint here; the guard
Phase 3 adds exists against future growth.

## `all` plan, per area (the design's worst case)

| Area | test | check | lint | wsl | payload (B) |
|---|---:|---:|---:|---:|---:|
| homelab | 21 | 0 | 7 | 7 | 2,119 |
| claudine | 19 | 1 | 5 | 5 | 1,697 |
| unchained-ai | 15 | 0 | 5 | 5 | 1,443 |
| schematic | 14 | 0 | 4 | 4 | 1,303 |
| darkmatter | 14 | 1 | 3 | 3 | 1,113 |
| biscuit-terminal | 9 | 1 | 2 | 2 | 879 |
| root | 9 | 1 | 3 | 2 | 789 |
| claudine/rendezvous | 9 | 1 | 3 | 3 | 968 |
| sniff | 8 | 1 | 2 | 2 | 673 |
| biscuit-icon | 8 | 0 | 2 | 2 | 708 |
| biscuit-tui | 8 | 0 | 2 | 2 | 696 |
| worktree | 8 | 1 | 2 | 2 | 712 |
| tree-hugger | 8 | 1 | 2 | 2 | 751 |
| biscuit-file | 6 | 1 | 2 | 2 | 650 |
| biscuit-hash | 6 | 1 | 2 | 2 | 650 |
| biscuit-speaks | 6 | 1 | 2 | 2 | 672 |
| biscuit-location | 6 | 0 | 2 | 2 | 634 |
| messenger | 6 | 0 | 2 | 2 | 564 |
| model-citizen | 6 | 0 | 2 | 2 | 604 |
| playa | 6 | 0 | 2 | 2 | 524 |
| queue | 6 | 0 | 2 | 2 | 524 |
| reaper | 6 | 0 | 2 | 2 | 534 |
| research | 6 | 1 | 2 | 2 | 606 |
| biscuit-contract | 3 | 0 | 1 | 1 | 326 |
| agent-sandbox | 3 | 0 | 1 | 1 | 331 |
| tabby | 3 | 0 | 1 | 1 | 256 |
| tools | 3 | 0 | 1 | 0 | 257 |
| visualizer | 3 | 0 | 1 | 1 | 296 |
| darkmatter/dmls | 3 | 0 | 1 | 1 | 306 |

(29 areas carry rows; `gap-only` areas aside, the `all` plan's other two
areas own no cells at all.) Row-set payloads are roughly **1/38th** of the
per-area `area_matrix` records they replace (2,119 B vs 7,908 B for homelab)
and about **27%** in aggregate (21,783 B vs 79,929 B).

## Scope-job output budget (today vs the design)

Today's `scope` job emits the legacy projection among ~25 outputs; the
largest, serialized:

| Output | Bytes |
|---|---:|
| `area_matrix` | 79,929 |
| `matrix` | 79,140 |
| `build_slices` | 63,067 |
| `build_owners` | 56,130 |
| `build_runners` | 14,107 |
| `policy` | 13,355 |
| `build_artifacts` | 10,843 |
| (everything else) | ~6,500 |
| **total scope.json** | **323,063** |

Under the design the per-area row sets (21,783 B across all areas, indexed
per area exactly as `area_matrix` is) **replace** `area_matrix`'s 79,929 B in
the `with:` path while the policy/build projections stay (Phase 8 Wave 2
keeps `policy`, `build_owners`, `build_slices`, `build_artifacts`,
`build_runners`). GitHub's documented `workflow_dispatch` input payload
ceiling is 65,535 characters per invocation — the largest single area's
payload is ~3% of that, and each `with:` entry travels per area call.

## Job estimate including area overhead

For the `all` plan, the whole-run job count under the row-driven design:

| Component | Jobs |
|---|---:|
| `validation`, `scope`, `ci-gate`, `ci-reporting` | 4 |
| `preflight` (full scope → 3 OSes) | 3 |
| `build` owners | 3 |
| `area-ci` matrix legs | 29 (+2 areas that schedule an audit with no rows) |
| per area: `coverage-audit` | 29–31 |
| per area: `accepted-gaps` (11 gap areas) | 11 |
| `_package-ci.yml` calls (one per area with rows) | 29 |
| producer rows: test 228 + check 12 + lint 68 + wsl 66 | 374 |
| wsl2 delegator legs + guest jobs | 66 + 66 |
| `area-drift` (when flagged) | 0–1 |
| **total** | **≈ 600** |

Today's shape produces the same producer-leg count (374 cells, wsl2 cells
costing two jobs — `job_estimate` 440 counts exactly that) plus 74
`package-ci` delegator legs instead of 29, so the row design **reduces**
delegator legs while leaving runner demand essentially unchanged. The one
behavioral change is R3's: L2/browser rows start alongside L1 instead of
behind it, raising **peak concurrent** runners per area by at most its L2+L2
row count (largest: homelab has no L2; the largest L2 area is far below its
L1 width). Phase 7 Wave 1 re-records this table from the shipped adapter
before the trial.

## Budget constants for Phase 7 to enforce (R7)

| Constant | Value | Rationale |
|---|---:|---|
| `MATRIX_LIMIT` | 256 | GitHub's per-matrix job ceiling; already defined in `affected_scope.py:436` |
| Row-set ceiling per area per job | 256 | same ceiling, enforced per row set before emit |
| Per-area serialized row-set payload budget | **16,384 B (16 KB)** | ~7.7× the worst measured area (2,140 B); a quarter of the 65,535-char input-payload ceiling, leaving room for the other three sets in one `with:` block |
| Total row-set payload across areas | **512 KB** | ~23× the worst measured total (21,783 B); guards the scope job's aggregate output |

The planner fails with a named error before emitting a plan that exceeds
either budget; it never truncates or silently splits an area (R7). At current
measurements no area is within an order of magnitude of any budget, so no
partitioning design is warranted.

## Phase 7 re-measurement (shipped adapter)

Re-measured on 2026-09-20 from the shipped planner, row adapter, scope
projection, and workflows (working tree on `888c19872` plus the uncommitted
Phases 3–7), offline, with
`python3 features/2026-09-19-direct-cell-execution/spikes/capacity.py`. No
hosted run produced any number below. Test gates for these shapes are pinned by
`scripts/ci/test_affected_scope.py::CapacityGuardTests`.

### Matrix cardinality and output sizes

Sizes are the `jq -c` form `ci.yml` writes to `$GITHUB_OUTPUT`.

| Plan | Executing cells | `area-ci` matrix | Largest row set | `build` matrix | `preflight` matrix | Largest area's rows | All `area_rows` | Scope document |
|---|---:|---:|---|---:|---:|---:|---:|---:|
| `--all` (no event) | 374 | 29 | 21 (homelab, test) | 3 | 3 | 3,625 B (homelab) | 39,911 B | 364,106 B |
| `--all --event push` | 308 | 29 | 21 (homelab, test) | 3 | 3 | 2,935 B (homelab) | 33,748 B | 346,987 B |
| `--all --event workflow_dispatch` | 374 | 29 | 21 (homelab, test) | 3 | 3 | 3,625 B (homelab) | 39,911 B | 364,106 B |
| `--all --event schedule` (nightly) | 66 | 29 | 7 (homelab, wsl) | 1 | 2 | 819 B (homelab) | 10,618 B | 171,233 B |

Row counts by set, full workspace: test 228, check 12, lint 68, wsl 66.
Push to `main` (no WSL2): the same minus the 66 wsl rows. Nightly: wsl 66 only.

**Correction to the Phase 1 model.** The shipped row is an object
(`{"package", "gate", "environment", "runner"}`, key order kept for R1's
labels), and each area document carries four `has_*_rows` flags. S3 modeled
bare arrays, so the real payloads are ~1.8× the Phase 1 figures (homelab
3,625 B, not 2,140 B; all areas 39,911 B, not 21,783 B). Every size and count ceiling still
has more than 4× headroom.

**Phase 8 re-measurement (2026-09-20, same script and tree plus Phase 8).**
Retiring `matrix` and `area_matrix` changes no row, count, or job: every column
above except the last is identical. The scope document shrinks to 204,970 B
(`--all` and `workflow_dispatch`), 193,263 B (push), and 77,421 B (nightly),
about 44% smaller on the full workspace. The largest remaining `scope`-job
output is `area_rows` at 39,911 B.

### Against the ceilings

| Ceiling | Enforced by | Worst measured | Headroom |
|---|---|---:|---:|
| 256 jobs per matrix (GitHub) | `MATRIX_LIMIT`, every row set and the package list | 29 (`area-ci`), 21 (a row set) | 8.8× |
| 16 KB per area's rows | `AREA_ROW_SET_BUDGET` | 3,625 B | 4.5× |
| 512 KB for all areas' rows together | `TOTAL_ROW_SET_BUDGET` (**new in Phase 7**) | 39,911 B | 13× |
| 1 MB per job output (GitHub) | the two budgets above, for `area_rows` | 39,911 B (`area_rows`; `area_matrix`'s 79,950 B retired in Phase 8) | 26× |
| Reusable-workflow depth | `the_reusable_workflow_chain_stays_within_githubs_four_levels` | 4 (`ci.yml` → `_area-ci.yml` → `_package-ci.yml` → `_wsl-ci.yml`) | 0 under a 4-level reading |
| Unique reusable workflows per caller | — (3 of 50) | 3 | 16× |

The aggregate budget was recorded in the Phase 1 table above but never
implemented; Phase 3 enforced only the per-area one. It matters because
`ci.yml` carries every area's rows as **one** output (`area_rows`) that each
`area-ci` leg indexes, so the sum, not any one area, is what meets GitHub's
per-output ceiling.

**Depth is the only ceiling without headroom.** The repository's contract and
the `_area-ci.yml` header treat four levels as GitHub's maximum; the Phase 1
table above cites ten. The chain satisfies both readings, but nothing may be
inserted into it under the stricter one, and this re-measurement does not
settle which one is current.

### Job estimate, including area overhead

Runner jobs occupy a hosted runner. Call jobs (`uses:`) do not. S3's ≈600
double-counted the WSL2 leg: `_package-ci.yml`'s `wsl2` job is a call into
`_wsl-ci.yml`, and only the guest job it calls occupies a runner.

| Component | `--all` | push | nightly |
|---|---:|---:|---:|
| `validation`, `scope`, `ci-gate`, `ci-reporting` | 4 | 4 | 4 |
| `preflight` | 3 | 3 | 2 |
| `build` owners | 3 | 3 | 1 |
| `area-drift` (only when flagged) | 0 | 0 | 0 |
| `coverage-audit` (one per scheduled area) | 29 | 29 | 29 |
| `accepted-gaps` (areas with a governed gap) | 11 | 9 | 11 |
| producer rows (one runner job per executing cell) | 374 | 308 | 66 |
| **runner jobs** | **424** | **356** | **113** |
| call jobs: `area-ci` / `package-ci` / `wsl2` | 29 / 29 / 66 | 29 / 29 / 0 | 29 / 28 / 66 |

The plan's own `job_estimate` (440 / 308 / 132) counts producer work only, with
a WSL2 cell as two jobs. It is a scheduling hint, not this table.

### Peak concurrency (ruling R3)

L2 and browser rows now start beside L1 instead of staging behind it. The
largest per-area rise is **5 runners** (darkmatter's L2 and browser rows on a
full-workspace plan). Nightly has none. This is well inside the per-matrix
ceiling, so R3's fallback (a second `needs:`-ordered job) is not warranted on
these numbers. Queue time on a real run is the evidence that would change that.

### The guard, proven

- Unit: `test_an_over_limit_row_set_fails_planning_with_a_named_error`,
  `test_an_oversized_area_payload_fails_planning_with_a_named_error` (Phase 3),
  and `CapacityGuardTests::test_the_combined_scope_output_budget_refuses_many_small_areas`
  (Phase 7). The last one also pins the boundary: one area fewer passes.
- Through the normal invocation path:
  `CapacityGuardTests::test_an_over_budget_area_fails_before_dispatch_and_emits_nothing`
  runs `affected_scope.main()` over the real workspace with `--all --plan-out`
  and only the per-area budget lowered to 256 B. The planner exits non-zero,
  names the area and the budget, prints no scope document, and writes no plan
  file. The same invocation at the shipped budget emits both.
