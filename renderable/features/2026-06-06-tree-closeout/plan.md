---
status: complete
date: 2026-06-06
completed: 2026-06-07
owner: ken
spec: renderable/features/2026-06-06-tree-closeout/spec.md
depends_on: renderable/features/_completed/2026-06-06-tree-features/plan.md
total_phases: 6
packages:
    - renderable
    - biscuit-terminal
    - darkmatter
    - darkmatter-cli
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1:
    - renderable/features/2026-06-06-tree-closeout/extension-hint-inventory.md
    - renderable/features/2026-06-06-tree-closeout/traversal-inventory.md
    - renderable/features/2026-06-06-tree-closeout/component-assessment.md
skills_files_updated_during_phase_1: []
packages_touched_during_phase_1: []
source_files_during_phase_2:
    - renderable/src/tree/attrs.rs
    - renderable/src/tree/mod.rs
    - renderable/src/tree/validate.rs
    - renderable/src/tree/render/browser.rs
    - renderable/src/tree/graphics.rs
    - biscuit-terminal/lib/src/render_tree/render.rs
    - biscuit-terminal/lib/tests/perf_gate.rs
    - darkmatter/lib/src/markdown/render_tree/fold.rs
    - darkmatter/lib/src/markdown/render_tree/build_context.rs
    - darkmatter/lib/src/markdown/render_tree/entrypoints.rs
    - darkmatter/lib/src/markdown/render_tree/structural_gate.rs
    - darkmatter/lib/src/markdown/render_tree/inventory.rs
    - darkmatter/lib/src/markdown/block/hr_builder.rs
    - darkmatter/lib/src/layout/page.rs
    - darkmatter/lib/tests/browser_render.rs
    - darkmatter/lib/tests/level2_render_tree_terminal.rs
docs_updated_during_phase_2:
    - renderable/features/2026-06-06-tree-closeout/extension-hint-inventory.md
    - renderable/features/2026-06-06-tree-closeout/traversal-inventory.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
packages_touched_during_phase_2:
    - renderable
    - biscuit-terminal
    - darkmatter
source_files_during_phase_3:
    - biscuit-terminal/lib/tests/perf_gate.rs
    - darkmatter/lib/src/markdown/render_tree/structural_gate.rs
    - darkmatter/lib/benches/render_pipeline_steps.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3:
    - renderable/features/2026-06-06-tree-closeout/performance-record.md
skills_files_updated_during_phase_3: []
packages_touched_during_phase_3:
    - biscuit-terminal
    - darkmatter
source_files_during_phase_4:
    - darkmatter/justfile
    - .config/nextest.toml
docs_updated_during_phase_4: []
docs_created_during_phase_4:
    - renderable/features/2026-06-06-tree-closeout/verification-record.md
skills_files_updated_during_phase_4: []
packages_touched_during_phase_4:
    - renderable
    - biscuit-terminal
    - darkmatter
    - darkmatter-cli
source_files_during_phase_5: []
docs_updated_during_phase_5:
    - renderable/features/2026-06-06-tree-closeout/component-assessment.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5: []
packages_touched_during_phase_5: []
source_files_during_phase_6:
    - darkmatter/lib/tests/tree_features_characterization.rs
docs_updated_during_phase_6:
    - renderable/docs/tree-rendering.md
    - renderable/docs/layout-and-style.md
    - renderable/README.md
    - renderable/features/_completed/2026-06-04-css-box-architecture/spec.md
docs_created_during_phase_6: []
skills_files_updated_during_phase_6:
    - .claude/skills/renderable/SKILL.md
    - .claude/skills/darkmatter/SKILL.md
    - .claude/skills/biscuit-terminal/render-tree.md
packages_touched_during_phase_6:
    - renderable
    - biscuit-terminal
    - darkmatter
# Phase 6 also moved three feature directories into `_completed/`
# (2026-06-04-css-box-architecture, 2026-06-06-tree-features,
# 2026-06-06-tree-closeout) and swept all absolute references to insert
# `_completed/` consistently across .md/.rs files.
---

# Tree Rendering Closeout Plan

**Goal:** Produce durable evidence that the CSS Box Architecture is complete:
Darkmatter builds one complete typed tree, every target performs one fold, the
remaining page-frame boundary is explicit, all required verification is green,
and optional Biscuit Terminal component work is classified rather than hidden.

**Precondition:** Complete every phase and acceptance criterion in
`../2026-06-06-tree-features/plan.md`.

## Phase 1: Produce the Architecture Inventories

Create the three durable audit artifacts required by the spec.

**Create:**

- `extension-hint-inventory.md`
- `traversal-inventory.md`
- `component-assessment.md` (initial scaffold; completed in Phase 5)

- [x] Inventory every production `set_hint`, `get_hint`, and `remove_hint`
  call, grouped by namespace, producer, consumer, node placement, output effect,
  and disposition.
- [x] Confirm no shared renderer reads extension data for style, layout,
  semantic browser attributes, or width behavior.
- [x] Inventory every recursive production traversal after tree construction:
  renderer fold, validation/diagnostics, documented transformation, or obsolete
  preparation.
- [x] Use all four Darkmatter targets in the reachability definition:
  Terminal, Browser, Markdown, and MarkdownPlus.
- [x] Add explicit negative searches for deleted mechanisms:
  `decorate_document`, `component_for`, `darkmatter.li`, `darkmatter.style`,
  sentinel prefixes, style/attribute merge functions, and component-policy
  render contexts.
- [x] Record one-line rationales for every retained extension hint/traversal.

**Exit condition:** The first two artifacts are complete enough to identify any
remaining architecture violations; the component artifact has the agreed table
shape and production-path column.

## Phase 2: Resolve Audit Findings and Finalize the Page Frame

Fix any remaining blocker found by Phase 1 and make the page-frame decision.

- [x] Promote any renderer-interpreted extension value to typed attrs, or delete
  it if stale. *(`darkmatter.hr.*` → typed `renderable::tree::ThematicBreakAttrs`
  on `NodeAttrs::thematic_break`; both producers and all three shared-renderer
  consumers converted; zero production extension-bag access remains.)*
- [x] Delete any obsolete preparation traversal missed by `tree-features`.
  *(None survives — confirmed by the negative `rg` set; only doc/test-comment
  references to the deleted mechanisms remain.)*
- [x] Inspect `DarkmatterPage` and `LayoutContext` against the constrained
  Option A responsibility list.
- [x] Adopt **Option A, the slim page frame**, unless the audit proves it still
  inspects component kinds or mutates component content.
- [x] Ensure the retained frame owns only viewport/page concerns:
  terminal/page width, outer margin/padding, full-page background,
  max-width centering, pronounced-background code-theme contrast, browser page
  wrapper metadata, and stylesheet assembly.
- [x] Add focused tests proving the frame carries no component policy and does
  not traverse/mutate document components.
  *(`page_frame_chrome_ignores_component_policy_content`,
  `page_frame_vertical_margin_only_wraps_component_body`.)*
- [x] Record the signed-off decision and rationale in
  `traversal-inventory.md`.

**Verification:**

- targeted Darkmatter page/layout tests
- `cargo check -p darkmatter -p darkmatter-cli`
- repeat all negative `rg` checks

**Exit condition:** No first-class behavior remains in extension hints or
preparation traversals, and the page-frame exception is constrained and proven.

## Phase 3: Add Final Architecture and Performance Assertions

Turn the intended architecture into durable tests.

- [x] Add a production-entry structural test proving a styled source's initial
  `Document` already contains layout, paint, text layout, and browser attrs.
  *(`structural_gate::styled_corpus_populates_every_typed_group`, now asserting
  a typed `Layout` alongside paint, text-layout, and browser attrs.)*
- [x] Render the same cloned/uncloned tree through Terminal, Browser, Markdown,
  and MarkdownPlus and assert the input tree is unchanged.
  *(`structural_gate::rendering_does_not_mutate_the_tree_across_targets_and_widths`
  extended to fold both Markdown dialects.)*
- [x] Render one tree at multiple terminal widths and assert width-dependent
  output changes without tree mutation. *(same test: 40 vs 100 cols.)*
- [x] Assert browser fragment and streaming paths emit identical style and
  attributes. *(`structural_gate::styled_browser_fragment_and_streaming_paths_agree`;
  also pinned on the synthetic corpus in `renderable`.)*
- [x] Assert portable Markdown drops paint/geometry/browser attrs and
  MarkdownPlus remains within its documented HTML policy.
  *(`structural_gate::markdown_dialects_degrade_within_policy`.)*
- [x] Assert `InheritedStyle` is the only text-appearance inheritance path.
  *(`structural_gate::inherited_style_is_the_sole_text_appearance_path`, plus the
  `renderable::tree::inherit` field-level unit tests.)*
- [x] Expand the structural performance corpus with every feature listed in the
  spec and prove:
  - zero first-class extension-bag access;
  - zero typed-attr serde round-trips;
  - zero per-node formatted hint keys.
  *(Both corpora expanded — `perf_gate::styled_corpus_document` and
  `structural_gate::STYLED_CORPUS`; the three properties collapse onto the
  single `renderable.*` bag-access observable, derived in the `structural_gate`
  module doc.)*
- [x] Update `render_pipeline_steps` and relevant Criterion corpora so measured
  paths are the real production entry points. *(New `styled_production` group
  benches `DarkmatterPage::render` / `render_to_browser` over the styled corpus.)*
- [x] Record short benchmark results and comparison rationale in a closeout
  artifact such as `performance-record.md`. *(Created.)*

**Verification:**

- targeted architecture tests
- `cargo test -p biscuit-terminal --test perf_gate`
- benchmark compile checks and short non-gating runs

**Exit condition:** Tests and the structural gate enforce the final topology,
and performance trend data exists for the production corpus.

## Phase 4: Review References and Run Behavioral Verification

- [x] Review and re-baseline the five named stale browser snapshots with an
  explicit rationale for CSS `auto` centering where accepted. *(Already
  re-baselined in `tree-features` with the accepted `margin: 0ch auto 0ch auto`
  centering rationale documented in `cutover_reference.rs`; Phase 4 re-ran and
  confirmed all five pass against the accepted baselines — see
  `verification-record.md` §1.)*
- [x] Review every remaining snapshot change from alpha, direct policy,
  text-layout, and browser-attribute work. *(No pending `*.snap.new` and no
  modified committed `*.snap`; the full reference/characterization corpora pass —
  §1, §7.)*
- [x] Run complete Level 1 suites without fail-fast omissions:
  - `just -f renderable/justfile test` *(490 passed)*
  - `just -f biscuit-terminal/justfile test` *(357 passed)*
  - `just -f darkmatter/justfile test` *(lib 3858 + cli 415 passed; recipe fixed
    to be Level-1-only — §2)*
- [x] Run doctests for all three package areas. *(renderable 98, biscuit-terminal
  191, darkmatter 161 — §3.)*
- [x] Run browser coverage only through:
  - `just -f darkmatter/justfile test-browser` *(59 passed after browser-tier
    leak-timeout fix — §4.)*
- [x] Run applicable real-terminal coverage only through:
  - `just -f biscuit-terminal/justfile test-l2` *(68 passed)*
  - `just -f darkmatter/justfile test-l2` *(55 passed — §5.)*
- [x] If a harness is unavailable, record the clean skip and available-backend
  results; use the required environment flags only in an environment expected
  to provide those harnesses. *(Host provides WezTerm/tmux/kitty/Chrome; all
  tiers ran for real, no skips required — §5.)*
- [x] Run Markdown/MarkdownPlus degradation tests explicitly. *(renderable 38,
  darkmatter 6 — §6.)*
- [x] Create `verification-record.md` with commands, counts, skips, retries, and
  reviewed snapshot decisions. *(Created.)*

**Exit condition:** Dedicated references, Level 1, doctests, browser, applicable
terminal, and dialect-degradation coverage are green or have documented
environmental skips permitted by the test policy.

## Phase 5: Complete the Biscuit Terminal Component Assessment

Finish `component-assessment.md`.

- [x] Assess every component named by the spec:
  `HorizontalRule`, `GraphExpression`, `MermaidDiagram`, `TerminalImage`,
  `Status`, `MetricsTree`, `InlineContent`, `PadLeft`, `PadRight`, and
  `FileSystem`.
- [x] For each, record:
  - whether it is reachable from the four-target Darkmatter production path;
  - current tree projection/renderer support;
  - target-specific behavior that cannot be shared;
  - disposition and one-line rationale;
  - blocking versus optional status.
- [x] Implement any production-path migration required to satisfy the parent
  architecture, with targeted tests. *(None required — every component is
  disposition **R** (retained); no production-path migration was needed.)*
- [x] For valuable non-blocking migrations, create separate feature specs and
  link them; do not expand closeout implementation scope silently. *(No candidate
  met the user-visible-value bar; two optional candidates recorded, not specced.)*
- [x] Explicitly record accepted specializations such as FileSystem terminal
  icon selection and terminal image protocols.

**Exit condition:** Every component has a durable disposition, all blockers are
resolved, and optional work is separately scoped.

## Phase 6: Documentation, Metadata, and Parent Completion

- [x] Update architecture and user documentation:
  - `renderable/docs/tree-rendering.md`; *(same-version serde contract +
    Option A page-frame boundary added)*
  - `renderable/docs/layout-and-style.md`; *(page-frame gap reframed as the
    signed-off Option A decision)*
  - component migration guidance; *(the deleted `migrate-component-to-ir.md`
    flip-guide's two live broken links repointed to `components.md` / README)*
  - Darkmatter rendering/style docs; *(already accurate post-tree-features;
    confirmed)*
  - replacement API examples; *(alpha `PaintColor` examples already in
    `layout-and-style.md` §6 / §7)*
  - same-version-only serde contract. *(stated in `tree-rendering.md` and the
    `tree.md` skill)*
- [x] Update renderable, biscuit-terminal, and Darkmatter skills to match the
  final implementation and page-frame boundary.
- [x] Repair the CSS Box Architecture parent:
  - replace stale `2026-06-05-*` child IDs;
  - link the actual completed `2026-06-04-*` specs;
  - add `tree-features` and `tree-closeout`;
  - update status, acceptance checklist, and architecture summary.
- [x] Review all changed rustdoc/module comments for drift. *(No library source
  changed in Phase 6; the one `.rs` edit was a `//!` doc-comment in
  `tree_features_characterization.rs` repointed to the moved path.)*
- [x] Run final lint/check commands without running `cargo fmt` directly.
  *(`just lint` clean for renderable / biscuit-terminal / darkmatter.)*
- [x] Re-run the highest-signal architecture tests after documentation/metadata
  moves and repair links. *(darkmatter `structural_gate` 7/7; biscuit-terminal
  `perf_gate` 2/2.)*
- [x] Move feature directories to `_completed` only after all verification is
  green and every relative link is updated. *(Moved the three active dirs; swept
  all absolute references; relative sibling links resolve.)*
- [x] Mark the parent complete only when all eleven closeout acceptance criteria
  are demonstrably satisfied by code, tests, and artifacts.

**Exit condition:** Documentation and skills describe the actual architecture,
all links resolve, audit/verification artifacts are durable, and the parent CSS
Box Architecture is legitimately complete.

