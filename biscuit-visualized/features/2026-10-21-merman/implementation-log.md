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
packages:
    - biscuit-visualized
    - biscuit-terminal
message_to_agent: |-
  Phase 2 is complete. Everything below the Merman render path exists, and
  Mermaid diagrams still render through mermaid-rs-renderer. Read the log's
  "## Phase 2" section, in particular "Facts the next phase needs".

  - Merman is a git dependency on Ken's fork: `yankeeinlondon/merman`, rev
    `6e6b1e6554e8e75d468c1f991652c1c53a85bace` (branch
    `gitgraph-commit-spacing`). The fork adds a gitGraph `commitSpacing` key
    (px, default 50); it is read from site config, `%%{init}%%` (allow-listed
    in merman-core's preprocess), and frontmatter. The backend id of D41 uses
    the first 12 characters of that rev.
  - Always pass a theme or site config to Merman. Without one, Merman draws
    text in "Recursive Variable" while asking our measurer about the
    trebuchet stack, and labels clip by up to 14.6 px on macOS.
  - Font API (pub(crate), `image` feature), in `biscuit-visualized/src/src/fonts/`:
      - `default_chain()` and `FontChain::{resolve, fallback, database}`
      - D63 hooks: `face_identity`, `query_identities`, `font_set_hash`
      - `raster::usvg_options(&chain)`
      - `LabelMeasurer::text_measurer(&chain)`, which returns
        `Arc<dyn merman::svg::TextMeasurer + Send + Sync>` built through
        `DeterministicTextMeasurer::with_width_callback`.
  - `mermaid::newline::rewrite_label_newlines(&str) -> Cow<str>` is the D34
    lexer, with module-level `#[allow(dead_code)]`. Phase 3 calls it in
    pipeline step 3b and adds the end-to-end "fitting box" check of AC-M6.
  - Cache key: `font_set_hash()` hashes about 1.7 GB on macOS (0.6–1 s).
    Ken is asked to rule on this (human_review_items) before you wire D41/D63.
    Follow his ruling.
  - D46's premise is stale. biscuit-terminal has a Windows console query
    backend (`discovery/tty_query/windows.rs`), so OSC 4 could run on Windows
    with no new code. It is kept Unix-only (`ASKS_PALETTE = cfg!(unix)`)
    until Ken re-decides. This matters for Phase 5, not Phase 3.
  - The palette hook for Phase 5 is
    `biscuit_terminal::discovery::osc_queries::ansi_palette() -> [Option<RgbValue>; 16]`
    (`#[doc(hidden)]`).
  - `MERMAID_FONT` now also applies to graphs, because every rasterized SVG
    goes through the chain.
  - `biscuit-terminal`'s lib tests do not compile with `--all-features`. This
    predates Phase 2 (`horizontal_rule`); use `--features image`, as its
    justfile does.
source_files_during_phase_2:
    - biscuit-visualized/src/Cargo.toml
    - Cargo.lock
    - biscuit-terminal/lib/Cargo.toml
    - biscuit-visualized/src/src/artifact.rs
    - biscuit-visualized/src/src/cache/file_cache.rs
    - biscuit-visualized/src/src/lib.rs
    - biscuit-visualized/src/src/fonts/mod.rs
    - biscuit-visualized/src/src/fonts/bundled.rs
    - biscuit-visualized/src/src/fonts/chain.rs
    - biscuit-visualized/src/src/fonts/query.rs
    - biscuit-visualized/src/src/fonts/metadata.rs
    - biscuit-visualized/src/src/fonts/matching.rs
    - biscuit-visualized/src/src/fonts/shaper.rs
    - biscuit-visualized/src/src/fonts/measure.rs
    - biscuit-visualized/src/src/mermaid/mod.rs
    - biscuit-visualized/src/src/mermaid/newline.rs
    - biscuit-visualized/src/src/raster/mod.rs
    - biscuit-visualized/src/src/raster/png.rs
    - biscuit-visualized/src/src/tests/mod.rs
    - biscuit-visualized/src/src/tests/bundled_font_tests.rs
    - biscuit-visualized/src/src/tests/font_chain_tests.rs
    - biscuit-visualized/src/src/tests/measure_tests.rs
    - biscuit-visualized/src/src/tests/newline_tests.rs
    - biscuit-visualized/src/src/tests/raster_tests.rs
    - biscuit-visualized/src/src/tests/fixtures/merman/wrap/wrap_class.mmd
    - biscuit-visualized/src/src/tests/fixtures/merman/wrap/wrap_er.mmd
    - biscuit-visualized/src/src/tests/fixtures/merman/wrap/wrap_flowchart.mmd
    - biscuit-visualized/src/src/tests/fixtures/merman/wrap/wrap_mindmap.mmd
    - biscuit-visualized/src/src/tests/fixtures/merman/wrap/wrap_sequence.mmd
    - biscuit-visualized/src/src/tests/fixtures/merman/wrap/wrap_state.mmd
    - biscuit-visualized/src/src/tests/fixtures/merman/mixed/mixed_class.mmd
    - biscuit-visualized/src/src/tests/fixtures/merman/mixed/mixed_flowchart.mmd
    - biscuit-visualized/src/src/tests/fixtures/merman/mixed/mixed_mindmap.mmd
    - biscuit-visualized/src/src/tests/fixtures/merman/mixed/mixed_sequence.mmd
    - biscuit-visualized/src/assets/fonts/inter/Inter-Regular.ttf
    - biscuit-visualized/src/assets/fonts/inter/Inter-Bold.ttf
    - biscuit-visualized/src/assets/fonts/noto-emoji/NotoEmoji-Variable.ttf
    - biscuit-terminal/lib/examples/discovery_probe.rs
    - biscuit-terminal/lib/src/discovery/osc_queries/mod.rs
    - biscuit-terminal/lib/src/discovery/osc_queries/parse.rs
    - biscuit-terminal/lib/src/discovery/osc_queries/query.rs
    - biscuit-terminal/lib/src/discovery/startup_exchange.rs
    - biscuit-terminal/lib/tests/common/pty.rs
    - biscuit-terminal/lib/tests/l1/level1_graphics_probe.rs
    - biscuit-terminal/lib/tests/l1/level1_osc4_palette.rs
    - biscuit-terminal/lib/tests/l1/main.rs
docs_updated_during_phase_2:
    - docs/dependencies.md
    - biscuit-visualized/docs/dependencies.md
    - biscuit-visualized/docs/components/mermaid_diagram.md
    - biscuit-visualized/docs/components/graph_expression.md
    - biscuit-terminal/docs/dependencies.md
    - biscuit-terminal/docs/discovery/color.md
    - biscuit-terminal/docs/discovery/terminal-queries.md
docs_created_during_phase_2:
    - biscuit-visualized/docs/merman.md
    - biscuit-visualized/features/2026-10-21-merman/upstream-issues/pr-01-gitgraph-commit-spacing.md
    - biscuit-visualized/src/assets/fonts/inter/README.md
    - biscuit-visualized/src/assets/fonts/inter/OFL.txt
    - biscuit-visualized/src/assets/fonts/noto-emoji/README.md
    - biscuit-visualized/src/assets/fonts/noto-emoji/OFL.txt
    - biscuit-visualized/src/src/tests/fixtures/merman/README.md
skills_files_updated_during_phase_2:
    - .claude/skills/biscuit-visualized/SKILL.md
    - .claude/skills/biscuit-visualized/artifacts-caching.md
    - .claude/skills/biscuit-visualized/rasterization.md
    - .claude/skills/biscuit-terminal/discovery.md
human_review: true
human_review_items:
    - |-
      **The font hash in the render cache key costs about a second per process on macOS. Decide how to identify fonts before Phase 3 builds the cache key.**

      *Why now:* Phase 3 builds the render cache key. The spec says that key includes a hash of the **contents** of every font file the font chain can return (D41), plus the content hash of each face it actually resolved, fallback faces included (D63). Fallback can reach every installed face, so on this Mac that means reading and hashing 2,986 font files, about 1.7 GB. That took 0.57 s and 0.95 s per process with the files already in the OS file cache, and longer from a cold disk. The same spec caps a one-shot render in a fresh process at 500 ms (D35). Built as written, every `bt`/Darkmatter process that renders a diagram on macOS pays about a second before its first cache lookup, and the 500 ms rule cannot be met. Linux hosts have far fewer fonts, so it barely shows there.

      **Options**

      - **A. Hash contents only for the faces the stacks resolve to; identify every other font file by path, size, and modification time.** Read the bytes only of the few faces the chain resolves for the design's stacks and the generic families, plus the bundled Inter and Noto Emoji. Identify the remaining fallback candidates by file path, size, modification time, and face index.
          - Pros: well under the 500 ms budget. Still invalidates on any font installed, removed, or upgraded, because an upgrade changes the size or modification time. Simple.
          - Cons: departs from "contents" for fallback faces; a font file replaced in place with the same size and a preserved timestamp would be missed (rare).
      - **B. Keep full content hashing, but remember it on disk.** Store each file's content hash under the cache directory, keyed by path, size, and modification time, and re-hash only changed files.
          - Pros: keeps the spec's content semantics exactly.
          - Cons: the first render on each host still pays 1–2 s; adds a persistent index with its own invalidation and concurrency handling.
      - **C. Build it as specified and relax D35's cold rule (for example to 1.5 s on macOS).**
          - Pros: no design change.
          - Cons: every diagram-rendering process on macOS is about a second slower, permanently; the cold-render rule stops protecting users.

      **Recommendation: A.** It keeps the property the key exists for, that a cached image is never reused after fonts change, at a cost that fits D35. The only case it gives up, a byte-different font with identical size and timestamp, is not one a user produces by installing or upgrading fonts. B buys exactness for that corner case with a persistent index and a slow first run; C makes every render slower to protect against the same corner case.
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

## Phase 2

Phase 2 built everything below the Merman render path: the `commitSpacing`
patch on Ken's Merman fork, the resvg/usvg 0.47 upgrade, the Merman git
dependency, the bundled fonts, one font-resolution chain shared by the
rasterizer and a rustybuzz measurer, the interim `\n` lexer, and the internal
OSC 4 query. Mermaid diagrams still render through `mermaid-rs-renderer`; the
rasterizer (graphs and today's Mermaid output alike) now draws through the
chain.

Work was split across five sub-agents (fork patch; usvg/fonts; OSC 4 in an
isolated worktree; font chain + `FontResolver`; measurer + oracle tests); the
`\n` lexer, the Merman dependency, the planned docs pages, and the gates were
done by the orchestrator.

### Stream C: Merman fork patch (D56, AC-C2a)

- Ken had **no GitHub fork** of `Latias94/merman` (`gh repo view
  yankeeinlondon/merman` failed; not among the upstream's forks). Item 8
  pre-authorizes pushing to "Ken's GitHub fork", so the fork was created as
  the minimal precondition: `gh repo fork Latias94/merman --clone=false
  --default-branch-only` → https://github.com/yankeeinlondon/merman.
- Local clone `/Users/ken/coding/forks/merman`: remote `ken` =
  `git@github.com:yankeeinlondon/merman.git` (the https push failed because
  the clone's credential helper points at a missing `/usr/bin/gh`; SSH with
  `BatchMode=yes` worked). Branch `gitgraph-commit-spacing` from
  `482fcde7faffb05a466f198085d615e604075d97`; commit
  **`6e6b1e6554e8e75d468c1f991652c1c53a85bace`**, signed (key
  `06385819D2659A03`, `git verify-commit` good), no attribution trailers.
  `git ls-remote ken gitgraph-commit-spacing` → that SHA. Nothing pushed to
  `origin`; no PR or issue opened.
- Patch (3 files, +195/−7):
    - `merman-render/src/gitgraph.rs`: `gitGraph.commitSpacing` (px, default
      50) replaces `COMMIT_STEP + LAYOUT_OFFSET` as the step; read like
      `diagramPadding` (numbers and numeric strings); zero, negative,
      non-numeric, `null`, and boolean values fall back to 50. Under
      `parallelCommits`, steps from a parent are `commitSpacing −
      LAYOUT_OFFSET`. Other `LAYOUT_OFFSET` uses (first-commit offset, BT axis)
      unchanged.
    - **Departure from the plan's description:** `merman-core/src/preprocess/mod.rs`
      gained a one-entry `MERMAN_EXTENSION_CONFIG_KEYS = ["commitSpacing"]`
      allow-list, because Merman (like Mermaid 11.16) drops `%%{init}%%` keys
      that Mermaid's default config does not know, which would have silently
      removed `commitSpacing` from directives.
    - Tests: 4 unit (default 50 and 120 across LR/TB/BT with and without
      `parallelCommits`; first-commit offset; numeric strings and invalid
      values), 1 SVG test (commit `cx` via default, site config, init
      directive, frontmatter), 1 directive-filter test.
- Fork test runs: `merman-render --lib gitgraph` 55 passed;
  `gitgraph_svg_test` 66 passed; `layout_snapshots_test` 3 passed (every
  layout golden, gitGraph included, unchanged at the default);
  `merman-core --lib` 499 passed; clippy clean on the touched targets.
- PR draft: `upstream-issues/pr-01-gitgraph-commit-spacing.md`.

### Stream A: dependencies and bundled fonts

- **resvg/usvg 0.47 (R-M3, AC-M5).** Direct users were `biscuit-terminal/lib`
  and `biscuit-visualized/src` only. Ken committed the first bump mid-phase as
  `81a1838bd` ("chore(deps): update SVG rendering dependencies": resvg 0.47 in
  both, png 0.18); everything else is uncommitted. resvg/usvg 0.47 needed no
  source change; png 0.18 renamed `Compression::Default` → `Balanced` (same
  zlib level 6) and `set_adaptive_filter` → `set_filter(Filter::Adaptive)`.
  `PNG_ENCODING` is now `png@0.18:compression=balanced,filter=adaptive` and
  `RASTERIZER` `resvg@0.47` (the PNG cache key changes, so no old PNG is
  served). tiny-skia 0.12's own encoder now equals ours byte for byte, so
  `raster_tests::the_encoding_is_lossless_and_no_larger_than_tiny_skias` now
  asserts `<=` (was `<`; failed at 11553 = 11553).
- **Versions:** resvg/usvg 0.47.0, fontdb 0.23.0, tiny-skia 0.12.0, rustybuzz
  0.20.1, ttf-parser 0.25.1, fontique 0.12.0, unicode-bidi 0.3.18,
  unicode-script 0.5.8, serde_json 1.0.154. `cargo tree -d --workspace
  --target all` shows one copy each of usvg, resvg, fontdb, tiny-skia,
  rustybuzz, ttf-parser, fontique (png 0.17 survives only under `ico` and
  `tauri-codegen`).
- **fontique** with `["system", "fontconfig-dlopen"]` (orchestrator's
  change): without `fontconfig-dlopen` fontique links libfontconfig at build
  time, so every Linux build would need the fontconfig dev package; with it
  the library is loaded at run time, and a host without it gets an empty
  fontconfig backend (`LIB_RESULT.is_err()` → default), so step 3 is skipped
  and step 4 still resolves.
- **Merman dependency (R-C2, D58, D10):** `merman = { git =
  "https://github.com/yankeeinlondon/merman.git", rev =
  "6e6b1e6554e8e75d468c1f991652c1c53a85bace", default-features = false,
  features = ["svg", "all-diagrams", "layout-cytoscape"], optional = true }`
  under `image`; `mermaid-rs-renderer` stays until Phase 3. Resolving it
  required raising locked minimums to Merman's requirements: thiserror
  2.0.18 → 2.0.21, serde 1.0.228 → 1.0.229, rustc-hash 2.1.2 → 2.1.3,
  unicode-segmentation 1.13.2 → 1.13.3, indexmap 2.14.0 → 2.14.2+,
  data-encoding 2.10.0 → 2.11.1, pulldown-cmark 0.13.3 → 0.13.5 (each by
  `cargo update -p <crate>@<locked>` with Merman temporarily removed, one
  crate per resolver conflict). `cargo tree -p merman` has no png, usvg,
  resvg, tiny-skia, fontdb, or ELK crate; `cargo tree -i usvg` shows only
  0.47 via our resvg.
- **Inter (D40, R-M12, AC-M9):** Inter v4.1
  (`https://github.com/rsms/inter/releases/download/v4.1/Inter-4.1.zip`,
  `extras/ttf/`): `Inter-Regular.ttf` 411,640 bytes, `Inter-Bold.ttf`
  420,428 bytes, `OFL.txt` 4,380 bytes; SHA-256s in
  `src/assets/fonts/inter/README.md`. **`OFL.txt` declares no Reserved Font
  Name**: its copyright line is only "Copyright (c) 2016 The Inter Project
  Authors (https://github.com/rsms/inter)" (input to
  `2026-10-21-mermaid-embedded-font` D8). Inter's OS/2 `sFamilyClass` is 0,
  not 8.
- **Noto Emoji (D65, R-M14):** no static regular TTF is published
  (googlefonts/noto-emoji has none; googlefonts/emoji-bw 404s), so the
  **variable** font (`wght` 300–700, default instance Regular 400) from
  google/fonts `ofl/notoemoji/NotoEmoji[wght].ttf` at commit
  `8b0a1d0f5983c89bc2b93f1b5fb55f9e252744b5` is bundled as
  `NotoEmoji-Variable.ttf`, **exactly 1,982,596 bytes** (above the spec's
  1–1.5 MB estimate). Compiled in only under `#[cfg(target_os = "linux")]`
  (`fonts/bundled.rs`, constant defined inside that cfg).

### Stream B: OSC 4 (D19, D46, R-P1, R-P2, R-P7)

- Done in an isolated worktree; its base `f657c9148` is an ancestor of this
  branch and every touched file was identical between the two, so the files
  were copied back verbatim.
- Entry point `#[doc(hidden)] pub fn
  biscuit_terminal::discovery::osc_queries::ansi_palette() -> [Option<RgbValue>; 16]`
  (the one cross-crate hook for Phase 5's palette); everything behind it is
  `pub(crate)`. The sixteen `ESC]4;N;?BEL` queries ride the existing
  once-per-process OSC 10/11 startup exchange (after OSC 11, before the
  Kitty query and the DA1 sentinel), share its `DIRECT` cache, timeout
  (`DEFAULT_TIMEOUT`, 1 s) and silence budget, and use the color gate
  unchanged (no TTY, `CI`, `TMUX`/`ZELLIJ`/`STY`). Every outcome is cached,
  including all-unknown. Only `?` queries are sent.
- Parser `parse_osc4_palette`: `rgb:` with 1–4 hex digits per component
  (scaled from its own width), `#rrggbb`, BEL or ST; any order; mixed with
  other replies; truncated, malformed, or out-of-range replies leave the slot
  `None`; the first usable reply per index wins.
- **D46, restated (AC-P6), with a finding.** Native Windows sends no OSC 4:
  `ASKS_PALETTE = cfg!(unix)`, so its palette is all-unknown and Phase 5's
  diagrams there use background-derived accents; the docs say so. **But D46's
  recorded premise is stale on this branch:** the OSC query path is no longer
  a stub on non-Unix. `biscuit-terminal/lib/src/discovery/tty_query/windows.rs`
  exists and already answers OSC 10/11 in the startup exchange, so carrying
  OSC 4 there would need no Windows-specific code, which is D46's own
  condition for running it. The conservative outcome (not carried) was kept
  because it is unproven on Windows; turning it on is a one-line change plus a
  Windows proof. For Ken to re-decide before Phase 5 consumes the palette (not
  needed for Phase 3).
- Gate note: the color gate's `is_tty()` is stdout **or** stderr, not stdout
  alone as R-P7's wording says; unchanged.
- Unrelated pre-existing defect seen: `biscuit-terminal`'s lib test target
  does not compile with `--all-features` (28 errors in
  `components/horizontal_rule/mod.rs`, a `horizontal_rule_svg` argument-count
  change); `just test` uses `--features image` and is unaffected. Also the
  OSC 10/11 parser reads `rgb:ff/ff/ff` as 16-bit (≈1); a test asserts it;
  untouched.

### Wave 4–5: font chain, rasterizer, measurer

- **Chain** (`biscuit-visualized/src/src/fonts/{chain,query,metadata,matching}.rs`,
  `pub(crate)`, `image` feature): `FontChain::resolve`/`resolve_with_step`
  (D32 steps 1–4), `fallback(c, already_tried)` (usvg's contract:
  `already_tried[0]` is the resolved face), `fallback_order`, `database()`
  (the same `Arc<fontdb::Database>` usvg gets), `default_chain()` (system
  fonts loaded and `MERMAID_FONT` read once per process; fontique and the
  step-4 scan only when first needed). D63 hooks for Phase 3:
  `face_identity`, `query_identities`, `font_set_hash` (not yet in the key).
- Decisions inside the spec's room:
    - D50 Latin blocks: U+0000–007F, 0080–00FF, 0100–017F, 0180–024F,
      1E00–1EFF, 2C60–2C7F, A720–A7FF, AB30–AB6F, 10780–107BF, 1DF00–1DFFF;
      "regular" = weight 400 upright, "bold" = 700 upright; a variable font
      counts its default instance only.
    - Fallback order: installed faces by case-folded first family name;
      within a family, the face CSS matching picks for the resolved face's
      weight/style/stretch first, then PostScript name, face index, file
      path; then (Linux only) Noto Emoji. Bundled Inter is not in the
      fallback list. On macOS, hidden families (`.Apple Color Emoji UI`)
      sort first because of the leading `.`; documented.
    - Face matching within a family ports fontdb 0.23's private
      `find_best_match`, because fontdb's `query` matches names
      case-sensitively and cannot exclude the bundled faces.
    - S2's "mono/courier → monospace" name heuristic was dropped (a catalog of
      names, which D32 forbids); step 3 is the OS sans-serif answer, since a
      stack generic was already answered at step 2. A stack asking only for
      `courier` therefore gets sans-serif when Courier is not installed
      (`docs/merman.md` says so).
    - The `MERMAID_FONT` warning fires when a chain is built, behind a
      process-wide `AtomicBool`.
    - **Behavior change:** `MERMAID_FONT` now applies to every rasterized
      SVG, graphs and `GraphExpression::with_font_family()` included.
    - `available_font_families()` lists installed families plus `Inter`,
      not Noto Emoji.
- **Rasterizer (R-M4):** `raster::usvg_options(&Arc<FontChain>)` sets the
  chain's fontdb and a `usvg::FontResolver` calling `resolve`/`fallback`;
  `rasterize_svg_to_png_bytes_with` takes an explicit chain (tests).
- **Measurer (D31, R-M1, R-M2):** `fonts/shaper.rs` (usvg 0.47 text mirror)
  and `fonts/measure.rs` (`LabelMeasurer`; `text_measurer(chain) -> Arc<dyn
  merman::svg::TextMeasurer + Send + Sync>` through
  `DeterministicTextMeasurer::with_width_callback`; `width`, `faces`,
  `shape`). Strings are shaped exactly as given (unlike S2, `\n`/`\r` are
  not stripped). Three spike-vs-usvg mismatches fixed: usvg turns weight,
  stretch, italic, oblique into `wght`/`wdth`/`ital`/`slnt` variations;
  usvg empties a letter-spaced cluster when its advance is not > 0 (spike
  used ≥); usvg skips a face its `load_font` rejects. **No difference from
  usvg found: 0 px worst case on every compared label.**
- **Fixtures (D49):** `src/src/tests/fixtures/merman/{wrap,mixed}/` (10
  `.mmd`, 3,991 bytes total), written for the S2 spike; README says no
  third-party license applies.

### Facts the next phase needs

- **Merman's default font family.** Without a site config/theme, Merman draws
  in `"Recursive Variable"` while asking the measurer about `"trebuchet ms",
  …`; on macOS that clipped class labels by up to 14.6 px. With an explicit
  theme the two agree. **The Phase 3 render path must always pass a theme.**
- Merman sometimes passes `font_family` with a trailing CSS `;`; the measurer
  strips it. For some strings Merman also measures with plain `sans-serif`
  (Helvetica on macOS while drawn in Trebuchet); harmless (the wrap test shows
  no clip) but the face test accepts a match on any style Merman asked with.
- `font-kerning="none"` on `<text>` did not switch kerning off in usvg's
  output; `kerning="0"` does. Not investigated.
- **Font-set hash cost (D41/D63 vs D35).** On this Mac the once-per-process
  content hash of every file the chain can return covers 2,986 files, 3,419
  faces, 1,713,371,560 bytes: **571 ms and 950 ms** on two runs with the
  files already in the OS cache (cold disk will be slower). System font load
  ≈170–180 ms; first fontique answer ≈235–285 ms. Wired as specified, this
  alone exceeds D35's 500 ms cold-render rule on macOS. Raised for Ken
  (`human_review_items`).

### Departures from the plan/spec

- Fork creation on Ken's account (not in the plan; precondition of item 8).
- The Merman `%%{init}%%` allow-list for `commitSpacing` (above).
- Noto Emoji is the variable font, 1,982,596 bytes (no static regular face
  is published).
- fontique uses `fontconfig-dlopen`.
- D46's premise is stale (above); outcome kept as recorded.

### Requirement-to-test mapping

| Requirement / AC | Tests (all L1, `biscuit-visualized` unless noted) |
| --- | --- |
| AC-C2a (fork) | Fork: 4 gitgraph unit tests, 1 SVG test, 1 directive-filter test; `git ls-remote ken`; PR draft file |
| R-M3/AC-M5 (one usvg 0.47) | `cargo tree -d` / `cargo tree -i usvg` (above); `raster_tests::*` |
| D40/AC-M9 (Inter) | `bundled_font_tests::inter_regular_is_upright_400_in_family_inter`, `inter_bold_is_upright_700_in_family_inter`; `font_chain_tests::mermaid_font_inter_on_an_empty_host_resolves_regular_and_bold` |
| D65/R-M14/AC-M11 (Noto Emoji) | `bundled_font_tests` Linux parse test; `font_chain_tests::bundled_noto_emoji_is_last_and_never_resolved` (Linux), `no_noto_emoji_face_is_registered_off_linux`; `measure_tests::linux_emoji_without_an_installed_emoji_face_draws_bundled_noto_emoji`, `linux_installed_emoji_face_wins_over_bundled_noto_emoji` (Linux) |
| AC-M3 (chain order, warning) | `font_chain_tests::mermaid_font_names_bundled_inter_in_any_casing`, `mermaid_font_names_an_installed_family_ignoring_case`, `unset_empty_or_unknown_mermaid_font_falls_through_to_the_stack`, `stack_names_match_ignoring_case_in_stack_order`, `a_stack_generic_takes_the_os_answer_for_that_generic`, `no_installed_stack_name_takes_the_os_sans_serif_answer`, `no_generic_answer_takes_the_installed_sans_serif_rule`, `bundled_inter_is_matched_by_name_only`, `an_uninstalled_mermaid_font_warns_exactly_once_per_process`, `unset_or_empty_mermaid_font_never_warns` |
| AC-M3 (measured face = drawn face) | `measure_tests::measured_faces_are_drawn_faces_for_the_wrap_corpus_with_inter`, `..._on_host_fonts` (76 / 75 drawn lines) |
| AC-M4 (non-stack host) | `font_chain_tests::a_host_of_only_non_stack_fonts_resolves_every_query`, `..._paints_text`, control `an_empty_host_without_mermaid_font_paints_no_text` |
| AC-M8 (D50) | four `font_chain_tests::sans_rule_*` preference tests (each tied above the preference under test), `sans_rule_considers_only_sans_serif_classed_faces`, `metadata_of_inter_regular_matches_its_tables` |
| AC-M10/D59 (fallback) | `font_chain_tests::a_missing_character_falls_back_by_case_folded_family_name` (resolver's `select_fallback` = `fallback()`), `fallback_order_lists_installed_faces_only_then_last_resort`, `fallback_within_a_family_prefers_the_matching_weight` |
| R-M4 (rasterizer uses chain) | `font_chain_tests::rasterizer_paints_a_lowercase_mermaid_stack_in_bundled_inter`, `available_font_families_include_bundled_inter`, `family_names_list_installed_families_and_bundled_inter` |
| D63 hooks | `font_chain_tests::face_identities_follow_content_and_resolution`, `font_set_hash_ignores_loading_order_and_follows_content` |
| R-V8/AC-M1 (oracle) | `measure_tests::oracle_widths_match_usvg_on_bundled_faces` (deterministic: Latin, weights, italic, kerning on/off, combining marks, letter/word spacing incl. negative, small caps, emoji fallback, RTL/CJK as `.notdef` on both sides), `oracle_widths_match_usvg_on_host_fonts`, `oracle_widths_match_usvg_for_the_mixed_script_corpus` |
| AC-M2 (Merman wraps) | `measure_tests::wrapped_labels_fit_their_boxes_with_inter`, `..._on_host_fonts` (allow-list: only `wrap_er` "when it was created" / "and stored", upstream issue 6, ≈6.0–6.2 px; the test fails if they stop clipping); `measurer_measures_candidates_without_collapsing_whitespace`, `merman_measurer_reports_the_shaped_width`, `measurer_queries_usvg_default_family_for_a_style_without_one` |
| R-M10/R-V9/AC-M6 (`\n` lexer, string level) | `newline_tests::*` (13): one positive test per family (class, ER, mindmap, requirement, `classDiagram-v2`); per-family inputs with `\n` in every excluded construct (frontmatter, single- and multi-line `%%{init}%%`, `%%` comments, `click`/`link`/`callback`, `classDef`/`style`/`cssClass`, `accTitle`, `accDescr` line and block) plus one label that must change; 15 non-listed families from the S2 corpus unchanged and borrowed; header detection past comments/frontmatter; escaped `\\n`; CRLF. AC-M6's "fitting box" half is end-to-end and lands with the Phase 3 render path that calls the lexer |
| AC-P1 (OSC 4 parser) | `biscuit-terminal` `osc4_tests::*` (14) |
| AC-P2 (timeout, no re-send) | `biscuit-terminal` `level1_osc4_palette::silent_terminal_palette_returns_within_its_timeout_and_is_never_resent`, `da1_only_terminal_palette_is_unknown_and_never_resent`, `answering_terminal_palette_rides_the_startup_exchange` |
| AC-P5/R-P7 (no query) | `level1_osc4_palette::ci_sends_no_palette_query`, `redirected_output_sends_no_palette_query`, `multiplexer_sends_no_palette_query` |
| AC-P6/D46 | `only_unix_asks_for_the_palette`; docs |

The input-robustness matrix does not apply: Phase 2 adds no file or
configuration reader (the OSC 4 reply is a terminal protocol, per the plan's
scope note; `MERMAID_FONT` is a single string whose shapes are tested above).

### Gates

- `just test` (biscuit-visualized): **375 passed, 3 skipped**.
- `just lint` (biscuit-visualized): clean.
- `just test` (biscuit-terminal): **3881 passed, 55 skipped**.
- `just lint` (biscuit-terminal): clean.
- Dependents (narrowed, serially):
    - `claudine-cli --test l1 -E 'test(/pty/)'` (its PTY harness fakes the
      startup exchange): 122 passed. One test over 5 s,
      `sequence_task_providers::empty_hint_nested_sequence_list_fails_its_body_step_before_any_task_launches`
      (selected only because "empty" contains "pty"), passed alone in 3.06 s
      at host load ≈22–28 from other sessions: load-sensitive, unrelated.
    - `darkmatter -E 'test(/(?i)mermaid|diagram|svg|png|image|graph|raster/)'`:
      299 passed.
    - `worktree-cli -E 'test(/git_graph/)'`: 76 passed.
    - `sniff-cli -E 'test(/(?i)graph|ui|render|dependenc/)'`: 129 passed.
- Cross-OS (font chain and the shared startup exchange carry OS risk):
    - `just cross-check biscuit-visualized --os windows --all-features`:
      377 passed, 1 skipped.
    - `just cross-check biscuit-terminal --os windows`: 3483 passed, 64
      skipped.
    - `just cross-check biscuit-visualized --os linux --all-features`: 380
      passed, 1 skipped (the Linux-only Noto Emoji tests included).
    - `cargo check --target x86_64-pc-windows-gnu` clean for both crates.
- Scope check: see below.

### Scope check

`git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs
python3 scripts/ci/affected_scope.py`, run after every edit of the phase
(this paragraph excepted): `packages: ["biscuit-terminal",
"biscuit-visualized", "test-toolkit", "darkmatter", "sniff"]`,
`change_class: package`. `source_packages` are the two packages changed;
darkmatter and sniff are "changed test input read by package(s)", and
test-toolkit is "changed suite input owned by package(s) test-toolkit".
The gates were therefore **not** skipped. The commit `81a1838bd`, made
mid-phase, is not in the working tree, so the calculator did not see it; its
three files (`Cargo.lock`, both `Cargo.toml`s) are covered by the runs above.

Narrowed runs for the test-input readers:

- `cargo nextest run -p test-toolkit`: 478 passed, 2 skipped.
- `cargo nextest run -p darkmatter --test l1 -E
  'test(/shipped_schema_corpus::|current_root_migration_guard::|skill/)'`:
  13 passed.
- `cargo nextest run -p sniff -p sniff-cli -E
  'test(/(?i)repo|lock|package|skill|cargo|workspace/)'`: 1315 passed, **4
  failed, pre-existing**: `sniff-cli::l1
  snapshots::{cargo_monorepo_structure_json_snapshot,
  cargo_pnpm_monorepo_structure_json_snapshot,
  pnpm_nx_monorepo_structure_json_snapshot, repo_aggregate_json_snapshot}`.
  The new output adds a `"monorepo_standards"` field that sniff emits since
  `cf9361989` ("feat(sniff): expose package identity and ownership", already
  on this branch). The snapshots were last updated in `e4e1bafd0`, before
  that commit, and they read temp-dir fixtures, not repository files. Phase 2
  changes no sniff file, and no dependency bump can add a JSON field. Left
  for the sniff owners. Not caused by this phase.
