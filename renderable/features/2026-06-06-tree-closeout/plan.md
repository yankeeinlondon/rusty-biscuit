---
status: ready for implementation
date: 2026-06-06
owner: ken
spec: renderable/features/2026-06-06-tree-closeout/spec.md
depends_on: renderable/features/2026-06-06-tree-features/plan.md
total_phases: 6
packages:
    - renderable
    - biscuit-terminal
    - darkmatter
    - darkmatter-cli
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

- [ ] Inventory every production `set_hint`, `get_hint`, and `remove_hint`
  call, grouped by namespace, producer, consumer, node placement, output effect,
  and disposition.
- [ ] Confirm no shared renderer reads extension data for style, layout,
  semantic browser attributes, or width behavior.
- [ ] Inventory every recursive production traversal after tree construction:
  renderer fold, validation/diagnostics, documented transformation, or obsolete
  preparation.
- [ ] Use all four Darkmatter targets in the reachability definition:
  Terminal, Browser, Markdown, and MarkdownPlus.
- [ ] Add explicit negative searches for deleted mechanisms:
  `decorate_document`, `component_for`, `darkmatter.li`, `darkmatter.style`,
  sentinel prefixes, style/attribute merge functions, and component-policy
  render contexts.
- [ ] Record one-line rationales for every retained extension hint/traversal.

**Exit condition:** The first two artifacts are complete enough to identify any
remaining architecture violations; the component artifact has the agreed table
shape and production-path column.

## Phase 2: Resolve Audit Findings and Finalize the Page Frame

Fix any remaining blocker found by Phase 1 and make the page-frame decision.

- [ ] Promote any renderer-interpreted extension value to typed attrs, or delete
  it if stale.
- [ ] Delete any obsolete preparation traversal missed by `tree-features`.
- [ ] Inspect `DarkmatterPage` and `LayoutContext` against the constrained
  Option A responsibility list.
- [ ] Adopt **Option A, the slim page frame**, unless the audit proves it still
  inspects component kinds or mutates component content.
- [ ] Ensure the retained frame owns only viewport/page concerns:
  terminal/page width, outer margin/padding, full-page background,
  max-width centering, pronounced-background code-theme contrast, browser page
  wrapper metadata, and stylesheet assembly.
- [ ] Add focused tests proving the frame carries no component policy and does
  not traverse/mutate document components.
- [ ] Record the signed-off decision and rationale in
  `traversal-inventory.md`.

**Verification:**

- targeted Darkmatter page/layout tests
- `cargo check -p darkmatter -p darkmatter-cli`
- repeat all negative `rg` checks

**Exit condition:** No first-class behavior remains in extension hints or
preparation traversals, and the page-frame exception is constrained and proven.

## Phase 3: Add Final Architecture and Performance Assertions

Turn the intended architecture into durable tests.

- [ ] Add a production-entry structural test proving a styled source's initial
  `Document` already contains layout, paint, text layout, and browser attrs.
- [ ] Render the same cloned/uncloned tree through Terminal, Browser, Markdown,
  and MarkdownPlus and assert the input tree is unchanged.
- [ ] Render one tree at multiple terminal widths and assert width-dependent
  output changes without tree mutation.
- [ ] Assert browser fragment and streaming paths emit identical style and
  attributes.
- [ ] Assert portable Markdown drops paint/geometry/browser attrs and
  MarkdownPlus remains within its documented HTML policy.
- [ ] Assert `InheritedStyle` is the only text-appearance inheritance path.
- [ ] Expand the structural performance corpus with every feature listed in the
  spec and prove:
  - zero first-class extension-bag access;
  - zero typed-attr serde round-trips;
  - zero per-node formatted hint keys.
- [ ] Update `render_pipeline_steps` and relevant Criterion corpora so measured
  paths are the real production entry points.
- [ ] Record short benchmark results and comparison rationale in a closeout
  artifact such as `performance-record.md`.

**Verification:**

- targeted architecture tests
- `cargo test -p biscuit-terminal --test perf_gate`
- benchmark compile checks and short non-gating runs

**Exit condition:** Tests and the structural gate enforce the final topology,
and performance trend data exists for the production corpus.

## Phase 4: Review References and Run Behavioral Verification

- [ ] Review and re-baseline the five named stale browser snapshots with an
  explicit rationale for CSS `auto` centering where accepted.
- [ ] Review every remaining snapshot change from alpha, direct policy,
  text-layout, and browser-attribute work.
- [ ] Run complete Level 1 suites without fail-fast omissions:
  - `just -f renderable/justfile test`
  - `just -f biscuit-terminal/justfile test`
  - `just -f darkmatter/justfile test`
- [ ] Run doctests for all three package areas.
- [ ] Run browser coverage only through:
  - `just -f darkmatter/justfile test-browser`
- [ ] Run applicable real-terminal coverage only through:
  - `just -f biscuit-terminal/justfile test-l2`
  - `just -f darkmatter/justfile test-l2`
- [ ] If a harness is unavailable, record the clean skip and available-backend
  results; use the required environment flags only in an environment expected
  to provide those harnesses.
- [ ] Run Markdown/MarkdownPlus degradation tests explicitly.
- [ ] Create `verification-record.md` with commands, counts, skips, retries, and
  reviewed snapshot decisions.

**Exit condition:** Dedicated references, Level 1, doctests, browser, applicable
terminal, and dialect-degradation coverage are green or have documented
environmental skips permitted by the test policy.

## Phase 5: Complete the Biscuit Terminal Component Assessment

Finish `component-assessment.md`.

- [ ] Assess every component named by the spec:
  `HorizontalRule`, `GraphExpression`, `MermaidDiagram`, `TerminalImage`,
  `Status`, `MetricsTree`, `InlineContent`, `PadLeft`, `PadRight`, and
  `FileSystem`.
- [ ] For each, record:
  - whether it is reachable from the four-target Darkmatter production path;
  - current tree projection/renderer support;
  - target-specific behavior that cannot be shared;
  - disposition and one-line rationale;
  - blocking versus optional status.
- [ ] Implement any production-path migration required to satisfy the parent
  architecture, with targeted tests.
- [ ] For valuable non-blocking migrations, create separate feature specs and
  link them; do not expand closeout implementation scope silently.
- [ ] Explicitly record accepted specializations such as FileSystem terminal
  icon selection and terminal image protocols.

**Exit condition:** Every component has a durable disposition, all blockers are
resolved, and optional work is separately scoped.

## Phase 6: Documentation, Metadata, and Parent Completion

- [ ] Update architecture and user documentation:
  - `renderable/docs/tree-rendering.md`;
  - `renderable/docs/layout-and-style.md`;
  - component migration guidance;
  - Darkmatter rendering/style docs;
  - replacement API examples;
  - same-version-only serde contract.
- [ ] Update renderable, biscuit-terminal, and Darkmatter skills to match the
  final implementation and page-frame boundary.
- [ ] Repair the CSS Box Architecture parent:
  - replace stale `2026-06-05-*` child IDs;
  - link the actual completed `2026-06-04-*` specs;
  - add `tree-features` and `tree-closeout`;
  - update status, acceptance checklist, and architecture summary.
- [ ] Review all changed rustdoc/module comments for drift.
- [ ] Run final lint/check commands without running `cargo fmt` directly.
- [ ] Re-run the highest-signal architecture tests after documentation/metadata
  moves and repair links.
- [ ] Move feature directories to `_completed` only after all verification is
  green and every relative link is updated.
- [ ] Mark the parent complete only when all eleven closeout acceptance criteria
  are demonstrably satisfied by code, tests, and artifacts.

**Exit condition:** Documentation and skills describe the actual architecture,
all links resolve, audit/verification artifacts are durable, and the parent CSS
Box Architecture is legitimately complete.

