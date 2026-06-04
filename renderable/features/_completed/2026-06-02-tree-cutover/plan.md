---
phases: 6
created: 2026-06-03
start_phase: 1
source_spec: spec.md
packages:
  - renderable
  - darkmatter
  - biscuit-terminal
related_specs:
  - ../2026-05-26-graphics-policy/spec.md
  - ../2026-06-02-prose-tree/spec.md
  - ../2026-06-02-non-structural/spec.md
  - ../2026-06-02-perf-gate/spec.md
  - ../2026-06-03-browser-perf/spec.md
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2: []
docs_updated_during_phase_2:
  - renderable/features/_completed/2026-05-20-darkmatter-tree/baselines.md
docs_created_during_phase_2:
  - renderable/features/2026-06-02-tree-cutover/implementation-notes.md
skills_files_updated_during_phase_2: []
packages_during_phase_2:
  - darkmatter
  - biscuit-terminal
source_files_during_phase_3:
  - darkmatter/lib/src/markdown/mod.rs
  - darkmatter/lib/src/markdown/types.rs
  - darkmatter/lib/src/markdown/errors/blocks.rs
  - darkmatter/lib/src/markdown/render_tree/mod.rs
  - darkmatter/lib/src/markdown/render_tree/entrypoints.rs
  - darkmatter/lib/src/markdown/render_tree/fold.rs
  - darkmatter/lib/src/layout/page.rs
  - darkmatter/lib/tests/layout_snapshots.rs
  - darkmatter/lib/tests/snapshots/layout_snapshots__zero_config_prose_snapshot.snap
  - darkmatter/lib/tests/snapshots/render_tree_roundtrip__document_json_surface.snap
docs_updated_during_phase_3:
  - renderable/features/2026-06-02-tree-cutover/implementation-notes.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
  - .claude/skills/renderable/SKILL.md
packages_during_phase_3:
  - darkmatter
source_files_during_phase_4:
  - darkmatter/lib/src/markdown/yaml_block.rs
  - biscuit-terminal/lib/src/render_tree/render.rs
  - biscuit-terminal/lib/tests/filesystem_parity.rs
  - darkmatter/lib/tests/snapshots/layout_matrix__YamlBlock__top_margin_2.snap
docs_updated_during_phase_4:
  - renderable/docs/components.md
  - renderable/features/2026-06-02-tree-cutover/implementation-notes.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
  - .claude/skills/renderable/SKILL.md
packages_during_phase_4:
  - darkmatter
  - biscuit-terminal
source_files_during_phase_5:
  - darkmatter/lib/tests/render_comparison.rs
docs_updated_during_phase_5:
  - renderable/features/2026-06-02-tree-cutover/implementation-notes.md
  - renderable/features/_completed/2026-05-20-darkmatter-tree/baselines.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5: []
packages_during_phase_5:
  - darkmatter
source_files_during_phase_6: []
docs_updated_during_phase_6:
  - renderable/features/2026-06-02-tree-cutover/implementation-notes.md
docs_created_during_phase_6: []
skills_files_updated_during_phase_6: []
packages_during_phase_6: []
phase_6_status: blocked
---

# Execution Plan: Tree Cutover

Spec: `spec.md` (this directory)

## Overview

Cut over darkmatter's Markdown document pipeline and the remaining renderable
component holdouts to the shared render tree. The work is ordered so fidelity
gaps close before baselining, entry points flip only after the baseline is
recorded, legacy renderers stay compilable through parity and performance
validation, and bespoke code is deleted only after the acceptance gates pass.

**Success criteria:** `Markdown::as_html`, `Markdown::for_terminal`, and
`DarkmatterPage::render` route through the tree; every component rendered by
the document pipeline is tree-only or explicitly exempt; output is
parity-or-better on all targets; the perf gate in
`../2026-06-02-perf-gate/spec.md` passes; and legacy serializers,
`RuleProcessor`, and component bespoke compatibility hooks are removed.

---

## Phase 1 - Close Fidelity Prerequisites

*Finish the upstream fidelity work before capturing the cutover baseline.*

### Tasks

- [ ] Confirm the graphics-policy implementation is complete and merged into
  the cutover branch: `GraphicsMode { Off, Vector, Rich }`, terminal raster
  gated to `Rich`, `TerminalImageMode::Never -> Off`, browser HR styled SVG at
  `Vector` and above, and Mermaid lowered through promoted `Code` nodes.

- [ ] Verify `Image`, `ThematicBreak`, and `Code{mermaid}` tree node renderers
  own document lowering directly; helper calls into exempt non-structural
  components are allowed only below the node renderer boundary.

- [ ] Confirm strict-style warning behavior remains renderer-agnostic:
  `scan_inline_hr_warnings`, `style:` parsing, and `--strict-style` preflight
  still run independently of whether the legacy or tree renderer is selected.

- [ ] Keep `parse_hr_attribute_block` as the single HR attribute parser used by
  both warning preflight and tree folding.

- [ ] Land the shared Prose prerequisites if they are not already present:
  `TextEmphasis::inverse`, terminal SGR 7/27 lowering, browser inverse CSS
  lowering, and MarkdownPlus inline `Style` lowering.

- [ ] Run the existing render-tree parity and HR snapshot tests and triage every
  diff as either a regression to fix or a documented improvement such as the
  approved `<mark>` recovery.

- [ ] Remove or update any rustdoc or inline comments that describe behavior
  changed by the fidelity work.

### Validation Checkpoint

- [ ] `cargo test -p renderable --lib` passes.

- [ ] Focused darkmatter render-tree parity tests pass, including
  `render_tree_parity` and `render_tree_hr_snapshots`.

- [ ] Focused biscuit-terminal render-tree/style tests pass for inverse and
  graphics-mode behavior.

- [ ] The full fixture corpus produces parity-or-better output on Terminal,
  Browser, Markdown, and MarkdownPlus.

### Parallelizable Work

- [ ] Graphics-mode verification, strict-style warning verification, and Prose
  shared-prerequisite verification can run in parallel once the branch compiles.

---

## Phase 2 - Capture Pre-Cutover Baselines

*Record the authoritative performance baseline before flipping public entry
points.*

### Tasks

- [ ] Run the darkmatter migration parity benchmark with the cutover baseline
  name:
  `cargo bench -p darkmatter --bench migration_parity -- --save-baseline pre-cutover-2026-06-02`.

- [ ] Run the biscuit-terminal render-tree benchmark with the same baseline
  name:
  `cargo bench -p biscuit-terminal --bench render_tree -- --save-baseline pre-cutover-2026-06-02`.

- [ ] If the perf-gate benchmark suite from
  `../2026-06-02-perf-gate/spec.md` is available, run its tree-only baseline
  benches and save the same baseline name.

- [ ] Record middle estimates and fixture ratios in
  `../_completed/2026-05-20-darkmatter-tree/baselines.md`, including the note
  that `biscuit-terminal`'s component comparison group is tree-only where
  components already default to tree.

- [ ] Link the recorded baseline section from the tree-cutover implementation
  notes or PR description so Phase 5 can compare against the exact baseline.

### Validation Checkpoint

- [ ] Baseline files exist under Criterion output for the executed benches.

- [ ] `baselines.md` contains the pre-cutover date, commands, middle estimates,
  and any accepted fixture exceptions from the browser-perf signoff.

- [ ] No public render entry point has been flipped before this phase is
  recorded.

### Parallelizable Work

- [ ] The darkmatter and biscuit-terminal benchmark runs can execute in
  parallel on separate workers if the host has enough CPU capacity and the
  results are recorded with the same command options.

---

## Phase 3 - Flip the Darkmatter Document Pipeline

*Route the public darkmatter Markdown document APIs through the tree while
keeping legacy code available for comparison benches.*

### Tasks

- [ ] Promote the tree document entry points in
  `darkmatter/lib/src/markdown/render_tree/entrypoints.rs` from `pub(crate)` to
  the minimum public visibility needed by `Markdown` and `DarkmatterPage`.

- [ ] Route `Markdown::as_html` through the tree browser document renderer.

- [ ] Route `Markdown::for_terminal` through the tree terminal document
  renderer with the same default layout and terminal option semantics callers
  receive today.

- [ ] Route `DarkmatterPage::render` through the tree terminal document renderer
  and preserve its parity relationship with `Markdown::for_terminal(default)`.

- [ ] Keep `output/html.rs`, `output/terminal.rs`, and `RuleProcessor`
  compilable for `migration_parity` and focused parity comparisons through
  Phase 5.

- [ ] Update snapshots and byte-for-byte tests where tree output is an approved
  deliberate improvement, especially `<mark>` browser recovery and graphics
  policy output.

- [ ] Audit `Markdown::as_terminal`, `as_terminal_with_layout`, CLI render
  paths, and any wrapper APIs so there is no remaining public document path
  that silently selects the legacy serializers.

- [ ] Update rustdoc and comments on flipped APIs so they describe the tree
  renderer and no longer imply event-stream serialization.

### Validation Checkpoint

- [ ] Focused tests for `Markdown::as_html`, `Markdown::for_terminal`,
  `Markdown::as_terminal`, `Markdown::as_terminal_with_layout`, and
  `DarkmatterPage::render` pass.

- [ ] `migration_parity` still compiles and can invoke both legacy and tree
  paths for comparison.

- [ ] Mechanical search shows legacy document serializers are not reachable
  from public darkmatter render entry points outside benchmark or parity-test
  code.

### Parallelizable Work

- [ ] HTML and terminal entry-point rewiring can be implemented in parallel
  after the tree entry points have the required visibility.

- [ ] Snapshot triage can run in parallel with API wrapper audits after the
  first flipped build compiles.

---

## Phase 4 - Flip Remaining Component Holdouts

*Make every component rendered by the document pipeline tree-only, and keep
only documented exemptions for non-structural helpers.*

### Tasks

- [ ] Flip `darkmatter::YamlBlock` default rendering to its tree projection and
  remove any default path that still prefers the old renderer.

- [ ] Complete `biscuit-terminal::components::Prose` collapse onto the shared
  render tree if it is not already landed: parser emits `RenderNode`, target
  renderers delegate to shared tree renderers, and `ProseDocument` is removed.

- [ ] Close `biscuit-terminal::components::FileSystem` terminal parity gaps:
  connector-list `Style` lowering and icon-spacing parity.

- [ ] Flip `FileSystem` terminal rendering to the tree once parity is proven.

- [ ] Review `GraphExpression`, `MermaidDiagram`, `TerminalImage`, `Status`,
  `InlineContent`, `PadLeft`, `PadRight`, `HorizontalRule`,
  `DarkmatterPage`, and `FileTree` against the Exemption Register in
  `../2026-06-02-non-structural/spec.md`.

- [ ] For each exempt component, confirm it is not directly rendered by the
  darkmatter document pipeline and document the justification if the register
  needs an update.

- [ ] Remove component-local `render_bespoke`, `fallback_render`, or old-render
  compatibility hooks for components flipped in this phase unless they are
  still required by Phase 5 comparison benches.

- [ ] Update component rustdoc, README sections, and `.claude/skills/`
  renderable notes if the public rendering behavior or migration workflow
  changed.

### Validation Checkpoint

- [ ] Focused tests for `YamlBlock`, `Prose`, and `FileSystem` pass on all
  targets they support.

- [ ] Mechanical search shows every component the document pipeline renders is
  either tree-only or listed in the Exemption Register with justification.

- [ ] Tree-rendered component snapshots match legacy output or carry an
  explicit approved-difference note.

### Parallelizable Work

- [ ] `YamlBlock`, `Prose`, `FileSystem`, and Exemption Register verification
  can proceed in parallel after Phase 3 lands.

---

## Phase 5 - Validate Cutover Gates

*Prove the flipped tree path meets fidelity and performance acceptance
criteria before deleting legacy code.*

### Tasks

- [ ] Run the full relevant test corpus for `renderable`, `darkmatter`, and
  `biscuit-terminal`, including `render_tree_parity.rs` and
  `render_tree_hr_snapshots.rs`.

- [ ] Re-run `cargo bench -p darkmatter --bench migration_parity` against the
  Phase 2 baseline and calculate the bespoke comparison gate from
  `../2026-06-02-perf-gate/spec.md`: per-target geomean `<= 1.0x` and no
  fixture above the `1.5x` ceiling unless it is an approved exception.

- [ ] Re-run the comprehensive tree-only benchmark suite against the Phase 2
  baseline and verify the baseline-trend guard does not regress by more than
  10%.

- [ ] Confirm the browser-perf accepted exceptions remain limited to the
  signed-off fidelity exceptions and do not hide new structural overhead.

- [ ] Search for all direct construction or use of `RuleProcessor`, legacy HTML
  serializers, legacy terminal serializers, component `render_bespoke`, and
  `fallback_render`.

- [ ] Classify every remaining legacy reference as comparison-bench code,
  parity-test code, deletion-candidate code, or a blocker to fix before
  deletion.

- [ ] Record final validation results, commands, and any accepted localized
  regressions in `baselines.md` or the feature notes.

### Validation Checkpoint

- [ ] Acceptance Criteria 1 through 4 from `spec.md` are all satisfied.

- [ ] No unclassified legacy renderer call sites remain.

- [ ] Test and benchmark commands, dates, baseline names, and results are
  recorded in a reviewable document.

### Parallelizable Work

- [ ] Test runs, benchmark runs, and mechanical legacy-reference searches can
  run in parallel once Phase 4 is complete and the branch compiles.

---

## Phase 6 - Delete Bespoke Renderers

*Remove legacy rendering code only after the Phase 5 gates are green.*

### Tasks

- [ ] Delete darkmatter's legacy `output/html.rs` serializer once no production
  or benchmark path needs it.

- [ ] Delete darkmatter's legacy `output/terminal.rs` serializer once no
  production or benchmark path needs it.

- [ ] Delete `RuleProcessor` and discharge the block-extension spec's legacy
  retention gate.

- [ ] Delete component bespoke render bodies, `render_bespoke`, and
  `fallback_render` compatibility hooks that are dead after Phase 4 and Phase
  5.

- [ ] Remove now-dead support types, imports, feature flags, benchmark arms,
  parity-test branches, snapshots, and fixtures that existed only to compare
  against the deleted bespoke renderers.

- [ ] Keep `parse_hr_attribute_block` and `scan_inline_hr_warnings` intact as
  the single source of truth for HR parsing and strict-style preflight.

- [ ] Update module exports and crate docs so they expose only the tree
  renderer paths.

- [ ] Update `renderable/docs/components.md`, relevant READMEs, and skill files
  if they still describe two render paths or a future cutover.

- [ ] Run a final mechanical search for deleted symbols and old-path language
  such as `RuleProcessor`, `render_bespoke`, `fallback_render`, `legacy
  serializer`, and `event-stream serializer`.

### Validation Checkpoint

- [ ] `cargo test -p renderable --lib` passes.

- [ ] Focused `darkmatter` tests for Markdown HTML, terminal rendering,
  `DarkmatterPage::render`, strict-style preflight, and render-tree parity pass.

- [ ] Focused `biscuit-terminal` component render-tree tests pass.

- [ ] The deleted legacy modules cannot be imported, and no production code
  references deleted renderer symbols.

- [ ] Documentation no longer promises or describes a supported bespoke
  rendering path.

### Parallelizable Work

- [ ] Deletion of darkmatter serializers, component compatibility hooks, and
  documentation cleanup can be split across workers after Phase 5 confirms
  there are no production callers left.
