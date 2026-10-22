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
human_review: true
human_review_items:
    - |-
      **How should gitGraph keep long tags from overlapping once Merman draws them?**

      *Why now:* the plan treats this phase as the point where you decide whether
      `worktree`'s tests may be edited (decision D22). The spec's answer (D36:
      "space commits wider through the gitGraph config") turns out not to exist:
      Merman, like Mermaid.js, hard-codes the distance between commits (50 px) and
      has no setting for it. The next phase (fonts, measurement, terminal palette)
      does not depend on this, but the cutover phase after it does, and it changes
      either the spec or a test.

      *What the spike found:* every gitGraph check in `worktree`'s tests holds on
      Merman's layout data (commit order, lanes, both merge parents, tags, the
      commit-step check) except one: in `gathered_graph_lays_out_long_labels` a very
      long branch-name tag overlaps the `main` and `origin/main` tags on the next
      two commits. Today's renderer avoids it by spacing commits wider (127.6 px);
      Merman would need about 126 px there.

      Options:

      - **A. Ask Merman upstream for a commit-spacing setting** (and, until it
        ships, carry a small pinned fork with it). Keeps D36 and the `worktree`
        tests unchanged. Cons: depends on an outside maintainer or on carrying a
        fork; a non-Mermaid.js setting.
      - **B. Widen spacing ourselves after Merman lays out** (shift each commit's
        x and everything attached to it in our SVG post-process). Keeps the tests
        unchanged with no upstream wait. Cons: rewriting Merman's SVG geometry is
        fragile, couples us to its markup, and grows the post-process a lot.
      - **C. Accept Mermaid.js's behavior (tags may overlap) and edit the one
        `worktree` assertion**, perhaps with `worktree` shortening or moving very
        long tag text itself. Simplest and matches the reference renderer. Cons:
        overlapping labels are a visible regression for long branch names, and
        it drops a behavior Ken previously chose to keep.

      **Recommendation: A**, filing the upstream request now and carrying the
      pinned fork only if upstream has not answered by the cutover phase. It is
      the only option that keeps the agreed behavior without making our
      post-process own Merman's geometry; Merman already lays out from one
      constant, so the change is small and easy to review. If a fork is not
      acceptable, prefer C over B.
message_to_agent: |-
  Phase 1 changed no package code. All 19 rulings are recorded in this log
  under "Rulings"; every one is the plan's suggested default ("assumed")
  except ruling 6 (no OSC 4 on native Windows: biscuit-terminal's non-Unix
  OSC query arm is still a stub returning `OscQueryError::Unsupported`, so
  Windows uses background-derived accents) and ruling 17 (open for Ken).

  Facts the next phases need:
  - Merman's gitGraph commit step is a hard-coded 50 px (`COMMIT_STEP` 40 +
    `LAYOUT_OFFSET` 10 in `merman-render/src/gitgraph.rs`) with no config key,
    so D36's "wider commit spacing via gitGraph config" cannot be built as
    written. Do not start Phase 3 Wave 6 (gitGraph geometry) until Ken has
    answered `human_review_items`. Phase 2 is unaffected.
  - Geometry from `layout_json` works: see
    `spikes/s3-gitgraph-geometry/src/main.rs` for serde structs over
    `layout.GitGraphDiagram` and SVG `tag-label-bkg` polygon parsing (LR only;
    TB/BT polygons carry a rotate transform).
  - Merman requires `serde_json ^1.0.154`; the workspace lock has 1.0.149, so
    adding Merman in Phase 2 will bump it.
  - The ruling-14 Merman feature set (`svg`, `all-diagrams`,
    `layout-cytoscape`, no defaults) pulls no resvg/usvg/png/ELK crate.
  - R-S9: macOS and Windows PNGs are pixel-identical (both Arial); the whole
    Linux gap is DejaVu substitution (0.27% dark, 0.74% light). Pin the golden
    font (ruling 9) and use a tolerance near AE ≤ 0.1%, RMSE ≤ 0.005.
---

# Implementation Log for 2026-10-21-merman (7 phases)

## Phase 1

Phase 1 is rulings and spikes only. No package source changed; the only new
files are `spikes/s3-gitgraph-geometry/` and the R-S9 Windows record under
`spikes/s1-comparison/`.

### Rulings (Wave 2)

Ken could not be asked in this non-interactive run, so every ruling below is
the plan's suggested default, marked **assumed**, unless the spikes changed
the picture (marked **assumed, amended by evidence**). Ken can overrule any
of them; the phase that first depends on each is named.

| # | Topic | Value | Status | Needed by |
| - | ----- | ----- | ------ | --------- |
| 1 | Q5 cache key / backend id | Backend id `merman@<12-char rev>+bv1`. Key covers: hash of the compiled per-family design JSON, content hash of every face file the font chain can return for the stacks plus generic families, post-process version, output format, and the **rewritten** source (after the `\n` rewrite) | assumed | Phase 3 Wave 5 |
| 2 | Q8 CI tiers | SVG goldens and OSC 4 parser tests at L1; PNG comparisons at L1 on Linux/macOS only; `gates = false` promotion is its own change; fixtures added to `source-inputs` only if read through a path CI cannot see | assumed | Phase 6 |
| 3 | Q11 Unicode/ASCII text fallback | Out of scope; a follow-up spec | assumed | — |
| 4 | Q16 benchmark | Release profile; Ken's macOS host is the reference; baseline JSON beside the benchmark; cache bypass with a fresh `BISCUIT_VISUALIZED_CACHE_DIR` per iteration outside the timed region. A Linux/Windows reference host is not scheduled | assumed | Phase 6 |
| 5 | Q18 WCAG "large text" | Measured in SVG user units before scaling: ≥ 24 px, or ≥ 18.66 px bold | assumed | Phase 5/6 |
| 6 | Q19 OSC 4 on native Windows | **Not on native Windows.** The prerequisite's Windows query arm is still a stub (see ruling 19), so carrying OSC 4 there would need new Windows-specific code. Windows uses background-derived accents; documented as such | assumed, amended by evidence | Phase 2 (OSC 4 task) |
| 7 | Q20 derived-palette goldens | Yes, SVG only, for flowchart, sequence, gitGraph: one ANSI-derived and one background-only palette | assumed | Phase 6 |
| 8 | Q21 fixtures location and cap | `biscuit-visualized/src/tests/fixtures/merman/`; the ~20 MB cap applies to `spikes/` and to test fixtures separately | assumed | Phase 3/6 |
| 9 | Q22 pinned golden fonts | Commit one freely licensed family (regular + bold); a test-only internal way to put a font directory ahead of D32 step 1; golden tests set `MERMAID_FONT` to it; mixed-script cases stay out of goldens (oracle test covers them) | assumed | Phase 6 |
| 10 | D32 step 4 rule (R-M8) | Among installed faces whose OS/2 table classifies them as sans-serif, prefer regular weight, non-italic, non-condensed; ties broken by smallest case-folded family name, then file path | assumed | Phase 2 Wave 4 |
| 11 | Fallback-face load order | Chain's resolved face first, then installed faces sorted by case-folded family name; tested per script that has a face on the CI hosts | assumed | Phase 2 Wave 4 |
| 12 | Tag spacing vs cache check | Spacing computed after the cache check; it is a pure function of the rewritten source and font-file hashes, both already in the key | assumed (but see ruling 17: the spacing mechanism itself is open) | Phase 3 |
| 13 | Darkmatter measured face per label (D37) | Additive library option on `mermaid::MermaidDiagram` selecting an HTML-static target; the `biscuit-visualized` post-process rewrites each label's `font-family` and overflow; Darkmatter only calls it and sanitizes; rasterized output never gets either change | assumed | Phase 4 |
| 14 | Merman feature set | `default-features = false`, `["svg", "all-diagrams", "layout-cytoscape"]`; `png`, `native-fonts`, `math` off (math labels show as source). S3 built with exactly this set; see the `png` check below | assumed | Phase 2 Wave 3 |
| 15 | Interim themes at cutover | Phase 3 maps the four `MermaidTheme` values to Merman built-in theme names; Phase 5 replaces this with compiled designs; never ships as final behavior | assumed | Phase 3 |
| 16 | `tag_gap` derivation | Clearance between adjacent measured tags implied by the spacing used, derived from the measurer and cross-checked against layout `pos` (AC-C12). Depends on ruling 17's outcome | assumed, blocked on 17 | Phase 3 Wave 6 |
| 17 | D22 decision point | **Open; needs Ken.** S3 shows all `worktree` gitGraph assertions hold on Merman geometry except one, and that D36's spacing mechanism does not exist in Merman (see R-S8 below). Until Ken decides, AC-C4 ("worktree tests unchanged") stands | **needs Ken** | Phase 3 Wave 6 |
| 18 | Circles `unsupported-paint` | Allow-list that specific known-false diagnostic per family in R-V4's test, with a comment naming it; the general "no `unsupported-*`" assertion is not weakened | assumed | Phase 5/6 |
| 19 | Prerequisite state | `2026-10-13-graphics-where-possible` has **not** delivered native Windows background detection: its log shows Phase 1 of 9 done (component move), and `biscuit-terminal/lib/src/discovery/osc_queries/query.rs` answers every query on non-Unix with `OscQueryError::Unsupported` (the `cfg(not(unix))` arm at line 321). Its Phase 4 "Native Windows backend" task is unchecked. The plan proceeds; the release note must record that Windows release waits on it (D7) | recorded | release |

Ruling 14, `png` check: `cargo tree` of the S3 spike, built with exactly the
ruling-14 feature set, contains no `resvg`, `usvg`, `tiny-skia`, `fontdb`,
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
Merman lacks. Raised for Ken as `human_review_items` (D22 / D36).

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
  basis; Phase 6 should confirm on the golden corpus.
- Two Windows traps found were added to the `os` skill
  (`.claude/skills/os/build-hosts.md`, `BUILD_WIN` row).

### Validation checkpoint 1

- Every ruling has a value or is marked assumed; ruling 17 is explicitly open
  for Ken.
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
