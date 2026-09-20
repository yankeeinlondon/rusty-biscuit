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
