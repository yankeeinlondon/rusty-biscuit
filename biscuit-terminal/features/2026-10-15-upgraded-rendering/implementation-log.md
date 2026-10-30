---
spec: "/Volumes/coding/wt/rusty-biscuit/feat-better-research/biscuit-terminal/features/2026-10-15-upgraded-rendering/spec.md"
plan: "biscuit-terminal/features/2026-10-15-upgraded-rendering/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
source_files_during_phase_1:
    - biscuit-terminal/features/2026-10-15-upgraded-rendering/evidence/capture/capture.rs
    - biscuit-terminal/features/2026-10-15-upgraded-rendering/evidence/capture/run-leg.sh
docs_updated_during_phase_1: []
docs_created_during_phase_1:
    - biscuit-visualized/src/tests/fixtures/mermaid-corpus/PROVENANCE.md
skills_files_updated_during_phase_1: []
packages:
    - biscuit-visualized
    - biscuit-terminal
human_review: true
human_review_items:
    - |-
      **Do you have Mermaid diagrams that actually failed for you?** (decide before Phase 2)

      The comparison set ("corpus") currently holds 8 diagrams taken from the upstream renderer's own regression examples plus two of our gitGraph tests. None of them is a diagram you saw fail. The first comparison ("leg A") was captured with today's libraries; Phase 2 upgrades the image library, after which re-capturing leg A needs a separate checkout of the old code.

      - **Option A — add your failing diagrams now.** Drop each into `biscuit-visualized/src/tests/fixtures/mermaid-corpus/` and re-run `run-leg.sh A` before Phase 2. Pro: the upgrade is judged against the problems you actually hit. Con: a short delay.
      - **Option B — proceed with the upstream examples.** Pro: no delay. Con: the evaluation may "pass" without proving your diagrams improved; the log says so explicitly.
      - **Option C — add them later.** Pro: no delay now. Con: leg A must then be re-run from a checkout of commit `958664cde` (possible, but slower and easy to forget).

      **Recommendation: A** if you have them at hand, otherwise B. The whole point of the feature is "some newer diagrams fail"; we have not yet reproduced a single one of your failures.
    - |-
      **Keep large PNG evidence in git?** (confirm or change; not blocking)

      Each comparison run produces about 4.6 MB of PNG images and 144 KB of SVG. I chose to commit the SVGs, a report with a content hash of every image, and only the PNGs of the defects actually investigated (about 800 KB per run); the rest are ignored by git and can be regenerated.

      - **Option A — as chosen (SVGs + report + investigated PNGs).** Pro: small repo growth, still reviewable. Con: the full PNG set lives only on this machine.
      - **Option B — commit every PNG.** Pro: complete record. Con: about 14 MB across the three runs.
      - **Option C — commit nothing binary, hashes only.** Pro: smallest. Con: reviewers cannot see the defects without re-running.

      **Recommendation: A** — it keeps the before/after images that the spec requires as evidence without bloating the repository.
message_to_agent: |-
  Phase 1 notes for Phase 2 (rasterizer upgrade):

  - The S1 spike showed both packages compile UNCHANGED against resvg 0.48.1 (and the pinned Mermaid rev) on rustc 1.98.1. Expect Wave 4 to be verification plus tests, not an API port; still read the resvg 0.48 changelog for behavior changes (text shaping moved from rustybuzz to harfrust/skrifa).
  - After the bump, mermaid-rs-renderer 0.3.1 keeps fontdb 0.23 for text MEASUREMENT while usvg 0.48 uses fontdb 0.24 for DRAWING. Two databases, two versions. Watch for measured-vs-drawn width differences in leg B.
  - Leg B: run `biscuit-terminal/features/2026-10-15-upgraded-rendering/evidence/capture/run-leg.sh B` from the repo after the lockfile bump. It renders the frozen corpus with the same configuration as leg A, into `evidence/leg-B/`. Copy the same five investigated PNGs (same filenames as `evidence/investigated/leg-A/`) into `evidence/investigated/leg-B/` and compare report.md hashes/sizes against leg A.
  - The leg-A `gitgraph_tag_spacing` defect (`PR #104 → main` text overruns its tag box) is suspected to be rasterization/font related. Leg B is where it gets attributed.
  - Rulings R1–R5 are provisional (headless run); see the log's Rulings section and the human_review_items.
---

# Implementation Log for 2026-10-15-upgraded-rendering (6 phases)

## Rulings

This run is headless, so R1–R5 could not be put to the author. Each is recorded
below as a **provisional ruling** taken from the plan's own recommendation; the
author can amend any of them (see `human_review_items`).

- **R1 — corpus and evidence location (provisional, amended).** Corpus inputs live in
  `biscuit-visualized/src/tests/fixtures/mermaid-corpus/` with provenance in
  `PROVENANCE.md` beside them. Evidence lives in `evidence/` beside this log:
  per leg, `leg-<X>/report.md` (outcome, natural size, viewBox, PNG size, corner
  alpha, and an xxh64 of every artifact via `biscuit-hash`), `versions.txt`,
  and every SVG (≈144 KB per leg) are kept; full PNG sets (≈4.6 MB per leg) are
  git-ignored and identified by hash; the PNGs of investigated defects are
  copied to `evidence/investigated/leg-<X>/` (≈800 KB for leg A) and kept.
- **R2 — backend spelling on retention: yes (provisional).** If 0.3.1 is retained,
  `MERMAID_BACKEND` stays `mermaid-rs-renderer@0.3.x+bv3`.
- **R3 — adoption authority (provisional).** Unambiguous evidence → implementer
  decides; mixed evidence → escalate before Wave 10 records the decision.
- **R4 — comparison host and fonts (provisional).** This macOS host
  (Darwin 27.2.0, rustc 1.98.1) is the comparison host for every leg; "selected
  families" means the face fontdb resolves for each `font-family` list the SVG
  names, recorded in each leg's report.
- **R5 — user-failure provenance (provisional).** No user-failing diagrams
  accompany the feature; the corpus is upstream regression inputs plus local
  gitGraph regressions, and **the user's particular failures have not yet been
  reproduced**. Rulings that gate later waves: R1 gated Wave 2 (taken as above);
  R3 gates Wave 10.

## Dependency Choices

### S1 — dependency-graph and toolchain spike (2026-10-16)

Run in a detached scratch worktree (`/tmp/ur-spike/repo`, HEAD `958664cde`,
removed afterward) with `resvg = "0.48.1"` in both consumer manifests and
`mermaid-rs-renderer` at Git `rev = "3726ccbffe0e8032361eb9668694b24f77858060"`,
`default-features = false`.

- `cargo update -p resvg -p mermaid-rs-renderer` resolved: resvg/usvg
  0.45.1 → 0.48.1, tiny-skia 0.11.4 → 0.12.0, kurbo 0.11.3 → 0.13.1, svgtypes
  0.15.3 → 0.16.1, imagesize 0.13 → 0.15; added fontdb 0.24.0, harfrust 0.12.0,
  skrifa 0.44.0, read-fonts 0.41.0, font-types 0.12.7, roxmltree 0.21.1;
  removed rustybuzz 0.20.1 (the new shaping stack replaces it).
- **Exactly one `resvg` and one `usvg`** (0.48.1) resolve workspace-wide
  (`cargo tree -i resvg|usvg --workspace --all-features`); usvg's only
  dependent is resvg, and resvg's direct dependents are `biscuit-terminal` and
  `biscuit-visualized`. The Mermaid crate's resolved features are only
  `default-features = false` (no `png`, `scene`, or `cli`); its direct deps are
  anyhow, fontdb, json5, once_cell, regex, serde, serde_json, thiserror,
  ttf-parser — no usvg/resvg.
- **Finding — a second `fontdb`.** `mermaid-rs-renderer` depends on `fontdb`
  0.23 directly for text measurement, while usvg 0.48.1 uses `fontdb` 0.24. With
  resvg 0.45 both used 0.23, so the graph now carries two fontdb versions (and
  fontdb 0.23's `fontconfig-parser` brings roxmltree 0.20 next to usvg's
  0.21.1). This is not a second rasterizer, but measurement (Mermaid) and
  rasterization (usvg) now resolve fonts through two separately loaded
  databases of different versions. Leg B must check that measured and drawn
  text agree.
- **Finding — the Git revision still reports version `0.3.1`** in its manifest,
  so only the full SHA can name it in `MERMAID_BACKEND`.
- `cargo check` on rustc 1.98.1 passed **with no source changes**:
  `biscuit-visualized` (`--all-features` and default), `biscuit-terminal`
  (`--features image` and `--no-default-features`), and `biscuit-terminal-cli`.
  The existing `usvg::Options`/`fontdb_mut`/`Tree::from_str`/`resvg::render`
  calls compile unchanged against 0.48.1, so Phase 2's port is expected to be
  verification rather than rewriting.
- MSRV: no crate in the resolved graph declares a `rust-version` above 1.98.1.
  Highest declarations: resvg/usvg 1.85.0, harfrust/skrifa/read-fonts/kurbo
  1.85; `mermaid-rs-renderer` declares none.
- Upstream history: `v0.3.1` is `2f993bd79a55235eb59a34d807852276ba25bea7`; 24
  commits lead to `3726ccb` (radar `a560e0c`, `b783fbb`, `43d6444`; state
  `ac37239`; requirement `1da35e5`, `75b0963`; flowchart `3e431ec`, `90a1bfc`,
  `7578b7b`; `%%{init}%%` in library entry points `f07ebff`; native scene API
  `3726ccb`).

## Corpus and Provenance

Eight inputs in `biscuit-visualized/src/tests/fixtures/mermaid-corpus/`, each
byte-identical to its origin; `PROVENANCE.md` there records origin, the change
each exercises, and expected behavior. Upstream inputs: `flowchart_decision_fan`,
`flowchart_cycles`, `radar_long_names`, `radar_short_curve`, `state_fork_join`,
`requirement_basic`. Local regressions reused from
`biscuit-visualized/src/src/tests/gitgraph_tests.rs`: `gitgraph_labeled_merge`,
`gitgraph_tag_spacing`. The `biscuit-terminal` gitGraph tests carry only a
one-commit source, already covered by these two, so none was copied.
The user's particular failures have not been reproduced (R5).

## Comparison Legs

All legs use `evidence/capture/run-leg.sh <leg> [repo-root]`, which builds
`capture.rs` as a throwaway crate outside the workspace, pinned by a copy of
the repo's `Cargo.lock`, and renders through the public
`MermaidDiagram::render` (the real `compute_layout` path) with
`BISCUIT_VISUALIZED_CACHE_DIR` set to a fresh empty directory; it asserts every
render is a cache miss. Fixed configuration: themes `default` and `dark`; SVG;
PNG `target_width = 1200`, opaque; PNG `scale = 2`, transparent.

### Leg A — mermaid-rs-renderer 0.3.1 + resvg 0.45.1 (baseline)

Evidence: `evidence/leg-A/` (`report.md`, `versions.txt`, SVGs; PNGs by hash)
and `evidence/investigated/leg-A/`. All 48 renders succeeded; every input
parses on 0.3.1, radar and requirement included. Natural size equals the
emitted viewBox size for every input and theme; transparent PNGs have alpha 0
in the corner, opaque PNGs 255. Fonts: the gitGraph/radar list
`trebuchet ms,verdana,arial,…` resolves to **Arial**; the flowchart/state/
requirement list `Inter,ui-sans-serif,…` resolves to **Inter**.

| Input | Category | Observation (default theme) |
| ----- | -------- | --------------------------- |
| `requirement_basic` | SVG geometry | `<<Requirement>>` header drawn above the box top; divider strikes through the `ID:` row; the `<<satisfies>>` label sits on the arrowhead (matches upstream `1da35e5`/`75b0963`) |
| `flowchart_decision_fan` | SVG geometry | both `Decision` branches leave the bottom vertex and cross before reaching their targets (upstream issue #129); `Let me think` places `H` (declared second) left of `G` |
| `radar_long_names` | SVG geometry | legend truncates curve names (`Enterprise Platform`, `Customer Success And`); data and axis labels correct |
| `gitgraph_tag_spacing` | PNG rasterization (suspected) | the `PR #104 → main` text overruns its tag box on the right; tags do not overlap each other. Suspected measured-vs-drawn font width mismatch (`→` glyph); leg B attributes it |
| `state_fork_join` | SVG geometry (minor) | fork/join bars and start/end markers correct; arrowheads draw as open chevrons with a visible stem |
| `radar_short_curve` | none | `Short` pads D and E to the center as expected |
| `flowchart_cycles` | none | back edges and the subgraph cycle route cleanly; labels visible |
| `gitgraph_labeled_merge` | none | `M` merges `S2` into `main` with its `origin/main` tag |

No terminal display defect is in scope for this leg: it captures artifacts,
not terminal output.

## Validation Evidence

Phase 1: see the gates recorded under Phase 1 below.

## Departures from Spec

- Plan Wave 2 asked for a provenance header in each corpus file; provenance is
  instead recorded in `PROVENANCE.md` beside the inputs so each input stays
  byte-identical to its origin (a `%%` comment could itself change parsing on
  one renderer version and contaminate the comparison).
- Rulings R1–R5 were recorded provisionally from the plan's recommendations
  because the session could not reach the author.

## Phase 1

- Scaffolded this log; rulings recorded provisionally (headless run).
- S1 spike passed: single resvg/usvg 0.48.1, Mermaid `png`/`scene` off, no
  MSRV above 1.98.1, both packages compile unchanged; second `fontdb` (0.23 via
  Mermaid, 0.24 via usvg) noted.
- Corpus frozen (8 inputs + `PROVENANCE.md`); capture tool written
  (`evidence/capture/`); leg A captured and categorized; evidence retention
  set by `evidence/.gitignore`.
- No library source or test changed in this phase: Phase 1 changes no
  behavior, so no regression test was added. The corpus is not yet read by
  any test (Phase 4 adds the defect regressions that consume it).
- Leg A cannot be re-run from the working tree after Phase 2 bumps the
  lockfile; to regenerate it, run `run-leg.sh A <worktree-at-958664cde>`.
- Gates (2026-10-16, macOS): `just test` in `biscuit-terminal` — 3605 passed,
  57 skipped; 6 pre-existing slow L1 tests over the 5 s budget
  (`layout_matrix::*` ×5 at 20–22 s, `render_comparison::via_render_matches_via_tree_direct`
  at 14 s), unrelated to this phase. `just lint` in `biscuit-terminal` passed.
  No cross-OS run: nothing that compiles changed.
- Requirement-to-test mapping: none this phase; no shipped behavior changed.
  The `biscuit-terminal` skill needed no update.
