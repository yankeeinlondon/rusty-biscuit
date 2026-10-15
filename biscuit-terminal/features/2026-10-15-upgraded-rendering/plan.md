---
total_phases: 6
created: 2026-10-15
phase: 1
agent: opencode/zai-coding-plan/glm-5.3
yolo: "true"
---

# Upgraded Diagram Rendering — Implementation Plan

## Summary and Success Criteria

Rendering has two stages — Mermaid interpretation/layout to SVG (`mermaid-rs-renderer` 0.3.1 in `biscuit-visualized`), and SVG rasterization to PNG (`resvg`/`usvg` 0.45 in both `biscuit-visualized` and `biscuit-terminal`). This feature upgrades the rasterizer to 0.48.1 in both consumers, evaluates a pinned upstream Mermaid revision (`3726ccbffe0e8032361eb9668694b24f77858060`, 24 commits past v0.3.1) against the released renderer on the upgraded rasterizer, makes cached artifacts respond to both stages, and documents real syntax support.

The work breaks into three strands plus surrounding duties:

1. **Rasterizer upgrade** — bump `resvg` to 0.48.1 in `biscuit-visualized/src/Cargo.toml` and `biscuit-terminal/lib/Cargo.toml`, adapt font-database/parsing/render calls in `src/src/raster/`, `terminal_image`, and `horizontal_rule`, and prove behavior preservation (aspect ratio, `target_width` precedence, scale, themes, transparency, premultiplied-to-straight RGBA conversion).
2. **Mermaid evaluation** — freeze a small provenance-tracked corpus (flowcharts, radar, state fork/join, requirement, existing gitGraph merge/tag regressions), run two attributed comparison legs (existing renderer old/new rasterizer; then released vs pinned renderer on the new rasterizer), adopt or explicitly retain 0.3.1 on the evidence, reevaluate local corrections, and test the real `compute_layout` integration path.
3. **Cache identity** — make `RASTERIZER` (`resvg@0.48.1`) an input to Mermaid and graph **PNG** cache keys through the shared `FileCache::cache_key` path (SVG keys unchanged), update `MERMAID_BACKEND` to the exact selected version or full SHA, and prove miss-then-hit invalidation from seeded old entries.

Supporting duties: implementation log beside the spec, docs/skills drift maintenance, and all-OS compile evidence through existing CI workflows only.

**Successful completion looks like:**

- Both direct rasterizer consumers resolve to exactly one `resvg` 0.48.1 (compatible `usvg`), compile with and without their `image` features on the pinned 1.98.1 toolchain, and no second rasterizer enters through `mermaid-rs-renderer` (its `png`/`scene` features stay off).
- The implementation log names the selected Mermaid version or full Git revision and justifies adoption or retention with recorded before/after SVG and PNG evidence for every investigated defect, with failures categorized as parse, SVG geometry, PNG rasterization, or terminal display.
- Focused behavioral regressions (labels/data preserved, non-overlapping bounds, dimensions, alpha; no font-dependent pixel snapshots) pass, including cache invalidation proofs and measured-dimensions-equal-viewBox after gitGraph corrections.
- `just test` and `just lint` pass in both areas; `just test-l2` passes where terminal integration is affected; compile evidence covers macOS, Linux, native Windows, and WSL2 through existing workflows with no CI scope expansion.
- Public APIs, CLI flags, defaults, fallback paths, and metadata shapes are compatible (cache-key changes shift artifact paths, as intended); docs and skills state verified support and remaining limitations without referencing this feature snapshot.

**Notes:** The Input Robustness Matrix does not apply — this feature changes no file-format or configuration reader; the cache-key change is generation-side, and corpus fixtures are test inputs, not configuration. The spec explicitly excludes engine replacement, browser/Node runtimes, terminal graphics-protocol changes, and performance studies; unresolved fidelity problems are recorded for the separate `2026-10-15-mermaid-renderer-comparison` experiment rather than investigated here.

## Phase 1 — Rulings, Dependency Spike, Corpus, and Baseline

### Necessary Rules

Rulings needed from the author before or during Wave 1; record each decision in `implementation-log.md`.

- **R1 — Corpus and evidence location.** Proposed: frozen corpus inputs live as files under `biscuit-visualized/src/tests/fixtures/` (repository reads spelled per the `rust-testing` skill) so regression tests consume the same sources; comparison evidence (per-leg SVG/PNG outputs) lives under `biscuit-terminal/features/2026-10-15-upgraded-rendering/evidence/`. Rule on whether binary evidence is committed or recorded by content hash with local retention. Confirm or amend.
- **R2 — Backend spelling on retention.** If the candidate is rejected and 0.3.1 is retained, may `MERMAID_BACKEND` remain `mermaid-rs-renderer@0.3.x+bv3` unchanged (recommended: yes — the backend did not change, and respelling `0.3.x` as `0.3.1` would invalidate SVG caches for no rendering change), with the spec's exact-version/SHA spelling rule taking effect the next time the backend identity actually changes?
- **R3 — Adoption authority.** Unambiguous evidence (candidate fixes corpus defects and passes existing regression checks) → the implementer adopts or retains per the spec's criteria. Mixed evidence (some fixes, some regressions) → escalate to the author before Wave 10 records the decision.
- **R4 — Comparison host and fonts.** Confirm this macOS host is the comparison host for every leg (identical fonts across legs), and that "selected families" means the font families usvg resolves for each corpus render, recorded per leg. Wider cross-host font measurement is out of scope unless the author requests it.
- **R5 — User-failure provenance.** No concrete user-failing diagrams accompany this feature; the corpus therefore starts from upstream regression examples with an explicit log note that the user's particular failures have not yet been reproduced. Confirm, or supply the user inputs to add.

### Spikes

- **S1 — Dependency-graph and toolchain spike** (Wave 1, runs once). In a scratch checkout outside the workspace manifests, set `resvg = "0.48.1"` in both consumer manifests and `mermaid-rs-renderer` to Git revision `3726ccbffe0e8032361eb9668694b24f77858060` (`default-features = false`); prove the complete graph resolves and `cargo check` passes on the pinned 1.98.1 toolchain; record any crate in the selected graph whose declared `rust-version` exceeds it; confirm exactly one `resvg` and one `usvg` resolve (`cargo tree -i resvg`, `cargo tree -i usvg`) with the Mermaid crate's optional `png`/`scene` paths off. One host, a quick sample — the spec already gives resvg 0.48.1's MSRV (1.85.0) but not the unreleased revision's graph, which is exactly what this spike measures.

Dropped spikes (ruled, not scheduled): a performance spike — the spec explicitly excludes a performance study; a rasterizer-fidelity spike — the spec's own comparison legs answer it and are scheduled implementation work below.

#### Wave 1

- [ ] **Record rulings** —
  - Present R1–R5 to the author; capture decisions and any new constraints in the implementation-log scaffold.
  - Flag any ruling that materially changes later waves (corpus location gates Wave 2; R3 gates Wave 10).
- [ ] **Run dependency spike** —
  - Execute S1 as described; record resolution output, MSRV findings, and single-rasterizer confirmation in the log.
  - If the spike fails on the pinned toolchain, stop and report — the upgrade decision changes before any workspace manifest is touched.
- [ ] **Scaffold implementation log** —
  - Create `biscuit-terminal/features/2026-10-15-upgraded-rendering/implementation-log.md` with sections: dependency choices, corpus and provenance, comparison legs, validation evidence, departures from spec.

#### Wave 2

Requires the corpus-location ruling (R1).

- [ ] **Freeze corpus** —
  - One file per input with a provenance header (upstream commit or local regression test of origin) and expected behavior: flowchart(s) including a routing case, radar, state fork/join, requirement diagram, plus the existing gitGraph labeled-merge and tag-spacing sources reused from `biscuit-visualized/src/src/tests/gitgraph_tests.rs` and the `biscuit-terminal` git_graph tests — reuse, do not invent a gallery.
  - Record the R5 note that the user's particular failures are not yet reproduced; radar and requirement may fail to parse on 0.3.1 — record that as their expected baseline category.
- [ ] **Capture baseline leg A** —
  - With current dependencies (Mermaid 0.3.1 + resvg 0.45), render every corpus input to SVG and PNG under one documented comparison configuration: same themes (default and dark), same scale/`target_width` choices, same fonts, identical requested dimensions across all future legs.
  - Isolate each leg's cache via `BISCUIT_VISUALIZED_CACHE_DIR` (or `FileCache::with_base_dir`) so preexisting output cannot substitute for a render; record resolved font families per R4.
  - Store evidence per R1; categorize each input as parse failure / SVG geometry defect / PNG rasterization defect / terminal display defect.

Phase exit: rulings logged; spike outcome logged; corpus frozen with provenance; leg-A SVG+PNG evidence and font record exist; log scaffolded.

## Phase 2 — Shared Rasterizer Upgrade to resvg 0.48.1

#### Wave 3

One task: the workspace has one lockfile, so the bump is inherently serial.

- [ ] **Bump rasterizer deps** —
  - Set the `resvg` range to the 0.48.1-compatible release in `biscuit-visualized/src/Cargo.toml` and `biscuit-terminal/lib/Cargo.toml`, keeping both optional under their existing `image` features; update the workspace lockfile; leave `mermaid-rs-renderer` at 0.3.1 with `default-features = false` for this phase.
  - Verify with `cargo tree` (features resolved) that exactly one `resvg` and one `usvg` version appear workspace-wide, the Mermaid crate pulls no `png`/`scene`/`usvg` path, and `biscuit-terminal/cli` has no direct `resvg` dependency.
  - Continue using `resvg`'s `usvg` re-export; add no direct `usvg` dependency.

#### Wave 4

Concurrent — the three tasks touch separate file surfaces.

- [ ] **Adapt visualized raster** —
  - Port `biscuit-visualized/src/src/raster/{mod,png}.rs` to the 0.48 APIs (font-database handling, `usvg::Options`/`Tree` parsing, render call) per the resvg 0.48 changelog.
  - Preserve the lazy `OnceLock` system-font initialization, uniform-scale `target_width` rendering with exact aspect ratio and minimum-one pixel sizing, the scale path, `available_font_families`, and both PNG entry points' public signatures.
- [ ] **Adapt terminal SVG paths** —
  - Port `biscuit-terminal/lib/src/components/terminal_image/mod.rs` `load_svg` and the `horizontal_rule` SVG rendering to the 0.48 APIs; preserve the premultiplied-to-straight RGBA demultiply loop and the default-options behavior (no system-font loading — that existing limitation stands and is out of scope to fix).
  - Preserve zero-dimension and parse-failure error paths.
- [ ] **Verify feature matrix** —
  - `cargo check` on the pinned toolchain for both packages with and without their `image` features, plus a default CLI build; record results in the log.

#### Wave 5

Concurrent — test writing and the comparison run are independent.

- [ ] **Rasterizer regressions** —
  - Focused behavioral tests, not font-dependent pixel snapshots: PNG output exactly `target_width` wide with height within integer pixel rounding of the aspect ratio; scale ≥ 1 behavior; transparent vs opaque background alpha; graph artifacts (expression and DOT) still render with preserved labels and correct dimensions.
  - Terminal: a partially transparent shape SVG demultiplied without dark fringes (assert interior pixel color and per-channel alpha math through the public image result); `horizontal_rule` SVG still renders; `target_width` precedence over `scale` and minimum-one sizing preserved in the component paths.
- [ ] **Comparison leg B** —
  - Re-render the corpus on Mermaid 0.3.1 + resvg 0.48.1 with the identical configuration, fonts, and isolated cache as leg A; categorize each input (parse / SVG geometry / PNG rasterization / unchanged) so improvements attribute to the rasterizer stage.
  - Attach before/after SVG+PNG evidence per investigated defect to the log (a successful render alone is not proof of fidelity); record remaining rasterization defects for the comparison experiment.

Phase exit: acceptance criterion 1 met (both consumers on 0.48.1, compatible `usvg`, existing feature configurations compile, disabled included); dimension/alpha regressions green; leg B recorded with evidence.

## Phase 3 — Cache Identity for Rasterization

Independent of Phase 4 once Phase 2 lands; the two phases may proceed concurrently. Comparisons in Phase 4 bypass caches anyway, so ordering between them is free.

#### Wave 6

- [ ] **Add rasterizer to PNG keys** —
  - Set `RASTERIZER` to `resvg@0.48.1` in `biscuit-visualized/src/src/cache/file_cache.rs` and fold it into `FileCache::cache_key`'s key string only when the output format is PNG — the shared path means both Mermaid and graph PNG keys include it and callers cannot omit it.
  - Preserve the public `cache_key` signature and the existing `biscuit-hash` xxHash pipeline; leave `GRAPH_BACKEND` untouched (a rasterizer-only upgrade must not change graph backend identity); update the cache tests that assert backend-identity constants.

#### Wave 7

- [ ] **Cache invalidation tests** —
  - In an isolated cache (`FileCache::with_base_dir` or `CACHE_DIR_ENV`), compute a pre-upgrade key by embedding the old constant in the test and seed an old PNG entry; assert the upgraded request misses the seeded entry, writes its own entry, and hits on repetition — prove invalidation, not constant strings.
  - Cover both Mermaid and graph PNG outputs; assert SVG keys for identical inputs are unchanged by the rasterizer bump; assert a Mermaid backend-identity change invalidates both Mermaid formats.
  - No cache migration, global deletion, or cleanup service: old files remain for the existing temporary-directory lifecycle.

Phase exit: rasterizer-only change provably shifts PNG keys only; miss-then-hit covered for both kinds; SVG key stability proven.

## Phase 4 — Mermaid Renderer Evaluation and Adoption

#### Wave 8

- [ ] **Pin candidate revision** —
  - Switch `mermaid-rs-renderer` to Git revision `3726ccbffe0e8032361eb9668694b24f77858060`, `default-features = false`, never a moving branch; re-verify exactly one `resvg`/`usvg` in the resolved graph (its optional PNG and scene paths pull their own `usvg`/`resvg` and must stay off).
  - Compile both packages on the pinned toolchain; record the full SHA and the relevant upstream fixes (radar data/labels, state fork/join, requirement text placement, flowchart routing) in the log.

#### Wave 9

Concurrent — the comparison render must exist before analysis, so run leg C first if not parallelizable in practice; corrections reevaluation consumes leg C output.

- [ ] **Comparison leg C** —
  - Render the corpus on the candidate + resvg 0.48.1 with the identical configuration, fonts, and isolated cache; compare against leg B (0.3.1 on the same rasterizer) so changes attribute to the Mermaid stage; categorize every result and attach before/after SVG+PNG evidence per investigated defect.
- [ ] **Reevaluate corrections** —
  - Against the candidate's output, reevaluate the labeled-merge repair, gitGraph tag spacing, and SVG post-processing (legend baseline, pie contrast, quadrant fills, circle radius, `git_inv_colors` theme mirror): retain each correction whose removal breaks or degrades a pinned behavior; remove one only with regression evidence that upstream replaces it; if a correction's assumptions shift against the revision, fix it and advance the `+bv<n>` suffix (bump `MERMAID_BACKEND` only for our-layer changes).
  - Record a per-correction verdict in the log.

#### Wave 10

- [ ] **Adoption decision** —
  - Before deciding, check crates.io for a published release containing the motivating fixes; prefer it over the Git pin if the corpus verifies it (exact version in the backend id; `0.3.x` cannot distinguish releases or revisions).
  - Adopt only if the candidate demonstrates relevant corpus improvements and passes existing regression checks on the integration path; set `MERMAID_BACKEND` to `mermaid-rs-renderer@<exact-version-or-full-SHA>+bv<n>` for the adoption, advancing the suffix only if correction behavior changed (per R2 if retaining).
  - If rejected: revert to 0.3.1, keep the completed rasterizer upgrade, record the unresolved Mermaid limitations and the candidate's failure; never describe the retained renderer as upgraded. Mixed evidence escalates per R3.

#### Wave 11

Concurrent — both consume the chosen stack from Wave 10.

- [ ] **Integration verification** —
  - Test the real `compute_layout` path (parse → merge repair → theme with `%%{init}%%` overrides → layout → `render_svg`), not upstream's convenience `render()` API: configuration precedence and explicit source theme overrides preserved; fixes confined to upstream's high-level API are not silently assumed inherited.
  - Assert `natural_size()` still equals the emitted SVG's viewBox after gitGraph corrections (terminal fitting depends on it) by parsing the emitted SVG; verify the SVG artifact remains standalone and parseable with visible labels and equivalent theme handling (browser surface).
  - Confirm the terminal package's fallible render path still returns errors on bad input and the infallible `TerminalRenderable` path still emits the source code block; public component APIs, CLI flags, defaults, and metadata shape stay compatible.
- [ ] **Defect regressions** —
  - For each corpus defect the chosen stack fixes (e.g. radar data/labels, state fork/join geometry, requirement text placement, flowchart routing): one focused behavioral regression asserting preserved labels/data, non-overlapping bounds, or dimensions — via the existing proof APIs (`gitgraph_geometry()`, `natural_size()`), never Mermaid text snapshots or exact pixel comparisons.

Phase exit: acceptance criteria 2, 3, 4, and 7 met — selected version/revision recorded with rationale, corpus evidence complete, focused regressions green, fallback and API compatibility confirmed.

## Phase 5 — Documentation, Skills, and Implementation Log

#### Wave 12

Concurrent — separate documents; all consume the Phase 4 outcome.

- [ ] **Update docs trees** —
  - `biscuit-terminal/docs/components/mermaid_diagram.md`: replace the unqualified "supports all Mermaid diagram types" claim with verified support and limitations (including radar and requirement status under the chosen stack).
  - Document the direct-SVG loader's font limitation in the terminal-image docs if it remains; update `biscuit-visualized/docs/mermaid-gitgraph.md` version references and backend-id spelling, and the graph docs only if behavior changed.
  - No `docs/` page references this feature snapshot; README overviews change only if public descriptions actually change.
- [ ] **Update dependency records** —
  - Root `docs/dependencies.md` and the per-area dependency pages: the resvg 0.45 → 0.48.1 move and the Mermaid renderer selection (release version or Git SHA), noting the single-rasterizer invariant and any MSRV finding from S1.
- [ ] **Update agent skills** —
  - `.opencode/skill/biscuit-visualized/` (`SKILL.md` dependency table, `mermaid-rendering.md`, `artifacts-caching.md` — PNG cache keys now include `RASTERIZER`, `rasterization.md`) and `.opencode/skill/biscuit-terminal/` (`mermaid-diagrams.md`, `image-rendering.md`); check the `.claude/skills/` tree per `biscuit-terminal/skills-lock.json` conventions.
  - Grep both skill trees and docs for stale `0.45` / `0.3.1` / `@0.3.x` strings before declaring done.
- [ ] **Consolidate implementation log** —
  - Final record: dependency choices, all three legs with font families, corpus results, validation evidence, departures from spec, and unresolved fidelity problems flagged (by name) for `2026-10-15-mermaid-renderer-comparison`.

Phase exit: docs and skills state current behavior with no stale version claims; log complete enough to serve as the review record.

## Phase 6 — Cross-Platform Validation and Acceptance

#### Wave 13

- [ ] **Local full validation** —
  - `just test` and `just lint` in `biscuit-visualized` and `biscuit-terminal` (nextest); feature-disabled compile checks repeated; `just test-l2` in `biscuit-terminal` since terminal image integration is affected — terminal/browser windows must not gain focus; CLI builds.
- [ ] **CI evidence** —
  - Review `just ci-local --plan` before pushing; push to run PR CI (Linux and macOS); native Windows and WSL2 compile evidence flows through the existing nightly over the changed packages, or pre-prove those cells with `just ci-seed` on the test rigs — load the `os` skill before claiming any OS cannot be tested from here.
  - Add no CI cell for image comparison; local artifacts serve visual investigation; respect existing evidence-reuse rules.

#### Wave 14

- [ ] **Acceptance review** —
  - Walk spec acceptance criteria 1–8 against the implementation log; verify public API/CLI/defaults compatibility with artifact paths shifted only via cache identity; confirm every task in this plan is checked off.
  - End state: implementation complete, ready for review — moving the feature to `_completed` remains the author's responsibility.

Phase exit: all eight acceptance criteria met with recorded evidence; the feature is ready for the review cycle.
