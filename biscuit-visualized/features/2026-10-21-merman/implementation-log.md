---
spec: /Volumes/coding/wt/rusty-biscuit/feat-merman/biscuit-visualized/features/2026-10-21-merman/spec.md
plan: biscuit-visualized/features/2026-10-21-merman/plan.md
implemented_by: claude/opus
started_phase: 1
source_files_during_phase_1:
    - biscuit-visualized/features/2026-10-21-merman/spikes/s3-gitgraph-geometry/Cargo.toml
    - biscuit-visualized/features/2026-10-21-merman/spikes/s3-gitgraph-geometry/Cargo.lock
    - biscuit-visualized/features/2026-10-21-merman/spikes/s3-gitgraph-geometry/src/main.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1:
    - biscuit-visualized/features/2026-10-21-merman/spikes/s3-gitgraph-geometry/FINDINGS.md
    - biscuit-visualized/features/2026-10-21-merman/spikes/s1-comparison/windows-leg.md
skills_files_updated_during_phase_1:
    - .claude/skills/os/build-hosts.md
packages: []
message_to_agent: |-
  Phase 1 changed no package code. Ken ruled every item on 2026-10-22
  (spec D40-D69); the "Rulings" table below shows each ruling, its spec
  decision, and, where it differed, what Phase 1 had recorded. Nothing is
  open. Ruling 17 is closed by D43 (fixed rewrite rule for worktree's
  gitGraph tests) and D56 (a `commitSpacing` gitGraph key on Ken's Merman
  fork). The plan's Phase 2 Stream C does the fork patch before any gitGraph
  geometry work.

  Facts the next phases need:
  - Stock Merman's gitGraph commit step is a hard-coded 50 px (`COMMIT_STEP`
    40 + `LAYOUT_OFFSET` 10 in `merman-render/src/gitgraph.rs`) with no
    config key; D56 adds `commitSpacing` (default 50) on the fork.
  - Geometry from `layout_json` works: see
    `spikes/s3-gitgraph-geometry/src/main.rs` for serde structs over
    `layout.GitGraphDiagram` and SVG `tag-label-bkg` polygon parsing (LR only;
    TB/BT polygons carry a rotate transform).
  - Merman requires `serde_json ^1.0.154`; the workspace lock has 1.0.149, so
    adding Merman in Phase 2 bumps it (pre-authorized).
  - The D58 Merman feature set (`svg`, `all-diagrams`, `layout-cytoscape`,
    no defaults) pulls no resvg/usvg/png/ELK crate.
  - R-S9: macOS and Windows PNGs are pixel-identical (both Arial); the whole
    Linux gap is DejaVu substitution (0.27% dark, 0.74% light). Goldens use
    the bundled Inter (D40) with AE <= 0.1%, RMSE <= 0.005 on every CI
    environment (D42).
---

# Implementation Log for 2026-10-21-merman (7 phases)

## Phase 1

Phase 1 is rulings and spikes only. No package source changed; the only new
files are `spikes/s3-gitgraph-geometry/` and the R-S9 Windows record under
`spikes/s1-comparison/`.

### Rulings (Wave 2; ruled by Ken 2026-10-22)

Phase 1 ran non-interactively and first recorded the plan's defaults. Ken
ruled every item on 2026-10-22; the spec records each as a decision. The
"Ruling" column is Ken's; the last column shows what Phase 1 had recorded,
only where it differed.

| # | Topic | Ruling | Status | Spec | Phase 1 had assumed (where different) |
| - | ----- | ------ | ------ | ---- | ------------------------------------- |
| 1 | Q5 cache key / backend id | Backend id `merman@<12-char rev>+bv1`. Key: source as rewritten by step 3b, output format, hash of the compiled per-family design JSON, once-per-process content hash of every face file the chain can return (bundled faces included), and the identity (content hash + face index) of the face actually resolved for each requested family and fallback | ruled by Ken 2026-10-22 | D41, D63 | Same, without the resolved-face identity |
| 2 | Q8 CI tiers | SVG goldens, PNG goldens (AE ≤ 0.1%, RMSE ≤ 0.005), and OSC 4 parser tests at L1 on every CI environment; this work promotes `biscuit-visualized` to gating (`gates = true`, exclusion fields removed); fixtures in `source-inputs` only if read through a path CI cannot see | ruled by Ken 2026-10-22 | D42 | PNG comparisons on Linux/macOS only; `gates = false` promotion left to its own change |
| 3 | Q11 Unicode/ASCII text fallback | Out of scope; follow-up `2026-10-22-merman-text-fallback` | ruled by Ken 2026-10-22 | D48 | — |
| 4 | Q16 benchmark | Release profile; reference host is the Linux build host; baseline JSON beside the benchmark; fresh `BISCUIT_VISUALIZED_CACHE_DIR` per iteration outside the timed region | ruled by Ken 2026-10-22 | D44 | Ken's macOS host as the reference |
| 5 | Q18 contrast | Every text element meets 4.5:1; no large-text exception | ruled by Ken 2026-10-22 | D45 | Large text (≥ 24 px, or ≥ 18.66 px bold, in SVG user units) at 3:1 |
| 6 | Q19 OSC 4 on native Windows | Not carried: the non-Unix OSC query arm is a stub, so native Windows uses background-derived accents, documented | ruled by Ken 2026-10-22 | D46 | — (Phase 1 reached the same outcome from the evidence) |
| 7 | Q20 derived-palette goldens | Six SVG goldens: flowchart, sequence, gitGraph × {full ANSI set, background only} | ruled by Ken 2026-10-22 | D47 | — |
| 8 | Q21 fixtures location and cap | `biscuit-visualized/src/tests/fixtures/merman/`; ~20 MB cap separately for `spikes/` and fixtures; bundled fonts are library source and do not count | ruled by Ken 2026-10-22 | D49 | — |
| 9 | Q22 golden font | Bundled Inter (regular, bold, OFL 1.1) in `biscuit-visualized/src/assets/fonts/inter/`, compiled in and registered as `Inter`; goldens set `MERMAID_FONT=Inter` | ruled by Ken 2026-10-22 | D40 | A committed test-only font directory put ahead of D32 step 1 by an internal test hook |
| 10 | D32 step 4 rule (R-M8) | Among installed OS/2 sans-serif faces: regular and bold present, then widest Latin coverage, then proportional over monospace, then alphabetically first family name | ruled by Ken 2026-10-22 | D50 | Regular, non-italic, non-condensed; ties by case-folded family name, then file path |
| 11 | Fallback-face order | Resolved face, then installed faces by case-folded family name, then (Linux builds only) the bundled Noto Emoji, last | ruled by Ken 2026-10-22 | D59, D65 | Same, without the Linux-only Noto Emoji |
| 12 | Tag spacing vs cache check | Pre-parse step 3b, before the cache check, so the key covers the rewritten source | ruled by Ken 2026-10-22 | D55 | Spacing computed after the cache check |
| 13 | Darkmatter measured face per label | Additive "HTML static" target on `mermaid::MermaidDiagram`; `biscuit-visualized` rewrites `font-family` and allows overflow; Darkmatter selects and sanitizes. Sanitizer admits `<filter>` and `<feDropShadow>` only (number-or-color attributes besides `id`; `dx`, `dy`, `stdDeviation` ≤ 20; `flood-opacity` 0–1); other `fe*` dropped; local-only `url(#…)` unchanged | ruled by Ken 2026-10-22 | D60, D64 | Same target design; `filter`/`fe*` admission left to the sanitizer change |
| 14 | Merman feature set | `default-features = false`, `["svg", "all-diagrams", "layout-cytoscape"]`; `png`, `native-fonts`, ELK, `math` off | ruled by Ken 2026-10-22 | D58 | — (same set; `math` off now ruled) |
| 15 | Interim themes at cutover | The four themes map to Merman built-in themes until the palette phase replaces them | ruled by Ken 2026-10-22 | D61 | — |
| 16 | `tag_gap` derivation | Clearance between adjacent tags produced by the `commitSpacing` written in step 3b, from measured tag widths and layout `pos` | ruled by Ken 2026-10-22 | D56, AC-C12 | Clearance implied by "wider spacing via gitGraph config", blocked on ruling 17 |
| 17 | D22 decision point | Closed. Fixed rule: topology assertions pass unchanged; renderer-mechanics assertions may be rewritten, each logged with category and reason. `commitSpacing` on Ken's Merman fork keeps `tag_overlaps()` unchanged | ruled by Ken 2026-10-22 | D43, D56 | Open, left for Ken; AC-C4 ("tests unchanged") standing meanwhile |
| 18 | Circles `unsupported-paint` | Allow-listed per family with a comment naming it; general assertion not weakened | ruled by Ken 2026-10-22 | D62 | — |
| 19 | Prerequisite state | `2026-10-13-graphics-where-possible` has **not** delivered native Windows detection: its log shows Phase 1 of 9 done, and `biscuit-terminal/lib/src/discovery/osc_queries/query.rs` answers every non-Unix query with `OscQueryError::Unsupported` (the `cfg(not(unix))` arm at line 321). Implementation proceeds; release waits on it | recorded fact; ruled by Ken 2026-10-22 | D7, D46 | — |
| 20 | Upstream drafts | Six issue drafts (`upstream-issues/01-…md` to `06-…md`; 6 = ER `<br/>` comment clip) and the `commitSpacing` pull request draft; Ken files and opens them during the cutover; agents never post | ruled by Ken 2026-10-22 | D52, D56, D66 | Four issue drafts, filed on Ken's go-ahead |
| 21 | Emoji fallback font | Noto Emoji (monochrome, regular, TrueType, OFL 1.1), Linux builds only, beside Inter, last fallback; Linux glyph test; goldens exclude emoji | ruled by Ken 2026-10-22 | D65 | — (not covered in Phase 1) |
| 22 | Benchmark placement | Non-member crate at `biscuit-visualized/bench/` (empty `[workspace]`), no CI cell; `just bench` runs it directly instead of delegating to `_bench` (item 11's exception); baseline JSON there | ruled by Ken 2026-10-22 | D67, R-V6, AC-V2 | — (the plan left the placement and the recipe open) |
| 23 | Measurer crates | `rustybuzz` and `ttf-parser` as direct dependencies of `biscuit-visualized`, pinned to the versions usvg 0.47 uses (one copy each); pre-authorized in item 1 | ruled by Ken 2026-10-22 | D68, R-D5 | — (the plan added them without pre-authorization) |
| 24 | Clean-room check | At final verification a reviewer confirms no Zed copyright or GPL header and no Zed-specific identifiers (`zed-accent`, `mermaid_render`, `inject_css`, `accent_colors`, at least) in the changed code; logged | ruled by Ken 2026-10-22 | D69, R-T10, AC-T9 | — (R-T10 had no verifying AC) |

Ruling 14 (D58), `png` check: `cargo tree` of the S3 spike, built with exactly
the ruling-14 feature set, contains no `resvg`, `usvg`, `tiny-skia`, `fontdb`,
`png`, or ELK crate (the only name match is `palette_math`, a color crate), so
no umbrella feature in that set pulls `png` or a second usvg.

### R-S8 / AC-S6: gitGraph geometry spike

New standalone project `spikes/s3-gitgraph-geometry/` (empty `[workspace]`,
`Cargo.lock` seeded from the workspace lock, then `serde_json` bumped to
1.0.154 because Merman requires `^1.0.154`; Merman at
`482fcde7faffb05a466f198085d615e604075d97`). Full write-up:
`spikes/s3-gitgraph-geometry/FINDINGS.md`; raw rows:
`spikes/s3-gitgraph-geometry/out/results.json`.

**Which copy ran.** Neither route in the spec could run: Cargo `[patch]`
cannot replace `worktree-cli`'s **path** dependency on `biscuit-visualized`,
and `git_graph/tests.rs` is a private unit-test module of the `wt` binary
that cannot compile outside it. Instead the real tests were run once with
`geometry_of` temporarily instrumented to dump its Mermaid input and today's
geometry (`S3_CAPTURE_DIR`), 76/76 passed, and the instrumentation was
reverted with `git checkout` (verified: `git status` shows no change outside
the feature directory). The 31 captured calls from 14 tests are in
`spikes/s3-gitgraph-geometry/inputs/`, and the spike replays them through
Merman, checking every property a `worktree` geometry assertion reads.

**Per-assertion result** (S = ids/order/lanes/parents/tag texts; O =
`tag_overlaps()` empty; P = the line-2193 `commit_step` check):

| Assertion group | Inputs | Result on Merman geometry |
| --------------- | ------ | ------------------------- |
| S, all 14 tests | 31 | **pass** (both parents of every merge kept, no repair; `+N` square ids kept exactly) |
| O, 10 tests | 27 | **pass** except `gathered_graph_lays_out_long_labels` (both viewports): `fix/very-long-exact-branch-reference-beta` overlaps `main` and `origin/main` at Merman's fixed step |
| P, line 2193 | 1 | **pass**: derived step 50 = Merman default 50 (that graph has no colliding tags) |

**commit_step.** Derivable: consecutive `pos` differ by exactly 50 in every
input (`COMMIT_STEP` 40 + `LAYOUT_OFFSET` 10).

**Widened commit spacing (D36).** The required diagram-wide step is 125.9 for
`long_labels` and 64.2 for `observation_1`/`observation_2`; every other
input keeps 50. **But Merman cannot be given a step**: it is a compile-time
constant in `merman-render/src/gitgraph.rs`, and the only gitGraph config keys
Merman reads are `rotateCommitLabel`, `showCommitLabel`, `showBranches`,
`diagramPadding`, `parallelCommits`, `titleTopMargin` (Mermaid.js has no such
key either). D36's "wider commit spacing in the generated Mermaid (gitGraph
config)" is not implementable at this rev. If spacing were applied by some
other route, line 2193 still holds (no widening for that graph) and O passes
for `long_labels`.

**Natural size.** Merman's gitGraph width is a median 1.15× today's (range
0.52–1.17×). `worktree`'s planner trims to the viewport using natural size, so
its generated Mermaid will change at cutover; the replay used today's plans,
so that effect is not covered here and is first visible in Phase 3.

**Go/no-go for "worktree tests unchanged": no-go as specified.** One
assertion (`long_labels`' no-overlap) cannot hold without a spacing mechanism
Merman lacks. Raised for Ken (D22 / D36); Ken closed it on 2026-10-22 with
D43 (fixed rewrite rule) and D56 (`commitSpacing` on his Merman fork).

### R-S9 / AC-S8: Windows PNG leg

Run by a subagent; full record in `spikes/s1-comparison/windows-leg.md`. One
diagram (`upstream_cypress_sequencediagram_spec_example_001`, Merman column,
S1 settings: resvg 0.45, 2× natural width capped at 1000 px), built from the
same tree on all three hosts in this run (Linux re-rendered fresh, not taken
from the gallery). Metric: ImageMagick `compare` AE share (fuzz 0) and
normalized RMSE, same-size pairs only (`scripts/cross_os_diff.py`).

| Pair | Theme | AE share | RMSE |
| ---- | ----- | -------- | ---- |
| macOS–Windows | dark | 0.000% | 0.0000 |
| Linux–Windows | dark | 0.268% | 0.0365 |
| macOS–Linux | dark | 0.268% | 0.0365 |
| macOS–Windows | light | 0.000% | 0.0000 |
| Linux–Windows | light | 0.735% | 0.0661 |
| macOS–Linux | light | 0.735% | 0.0661 |

- All three hosts produce 1000×1298 and a byte-identical SVG; every difference
  is rasterization.
- macOS and Windows both draw the labels in Arial and match pixel for pixel.
  The whole Linux difference is font substitution (DejaVu Sans; no Arial).
- With real-font measurement (`merman-real`), macOS–Windows differ by at most
  11 px (RMSE 0.0008) from different Arial files.
- **Input to D13:** with the pinned golden font of ruling 9, a tolerance of
  AE share ≤ 0.1% and RMSE ≤ 0.005 leaves a wide margin over the worst
  same-font difference seen. Without a pinned font, the gap is a different
  typeface (0.74% here, up to 3.2% across S1's corpus), and a tolerance that
  loose would hide regressions. This supports ruling 9 (pin the font) and
  ruling 2 (PNG comparisons on Linux/macOS only). One diagram is a thin
  basis; Phase 6 should confirm on the golden corpus. *(Ken's rulings of
  2026-10-22: the bundled Inter, D40, and PNG goldens on every CI
  environment, D42.)*
- Two Windows traps found were added to the `os` skill
  (`.claude/skills/os/build-hosts.md`, `BUILD_WIN` row).

### Validation checkpoint 1

- Every ruling had a value; ruling 17 was left for Ken. All were ruled by
  Ken on 2026-10-22 (see Rulings); ruling 17 is closed by D43 and D56.
- AC-S6 recorded (R-S8 above) with a go/no-go: **no-go** for "worktree tests
  unchanged" as specified. AC-S8 recorded (R-S9 above).
- `git status` (below) shows changes only under `spikes/`, this log, the plan,
  and one `os` skill line; no package source changed. The `os` skill edit is
  a skill update required by the repository's rule to record new OS facts in
  the same change, not package code.

### Requirement-to-test mapping

Phase 1 adds no package behavior, so no package test was added or changed.
The spike evidence is the check: S3's replay of all 31 captured
`worktree` geometry inputs (`out/results.json`) and the R-S9 comparison.
`worktree`'s own `git_graph::tests` (76) passed on the unchanged tree during
capture.

### Gates

- Scope check: `git status --porcelain --untracked-files=all | awk '{print
  $NF}' | xargs python3 scripts/ci/affected_scope.py` gave `packages:
  ["darkmatter"]`, `change_class: package`, reason "changed test input read
  by package(s) darkmatter": darkmatter L1 tests walk `.claude/skills/`, which
  the `os` skill edit touched. Gates were therefore **not** skipped.
- `just test` (biscuit-visualized): 294 passed, 3 skipped.
- `just lint` (biscuit-visualized): clean.
- Narrowed darkmatter run for the skill-tree readers: `cargo nextest run -p
  darkmatter --test l1 -E
  'test(/shipped_schema_corpus::|current_root_migration_guard::/)'`: 9 passed.
  `shipped_schema_corpus::loadable_shipped_schemas_resolve_and_lower` took
  5.5 s and was flagged slow; it reads the whole shipped corpus and is not
  affected by a one-line skill edit (pre-existing, recorded only).
- `worktree-cli` `git_graph::tests`: 76 passed during capture, on the
  unchanged tree.
- No cross-OS check: no package code changed. The Windows host was exercised
  for R-S9 itself.
