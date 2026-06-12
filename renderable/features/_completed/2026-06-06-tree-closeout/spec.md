---
status: complete
reviewed: true
date: 2026-06-06
completed: 2026-06-07
owner: ken
parent: renderable/features/_completed/2026-06-04-css-box-architecture/spec.md
depends-on:
    - renderable/features/_completed/2026-06-06-tree-features/spec.md
origin: renderable/features/_completed/2026-06-04-css-box-architecture/review-1.md
---

# Tree Rendering Closeout

Final verification, cleanup, documentation, and scope decisions after
`tree-features` makes the production render tree complete before every target
fold.

## Goal

Close the CSS Box Architecture and tree-rendering migration with evidence that:

```text
source/component
  -> complete typed Document
  -> one target fold
  -> final output
```

is the production architecture. No component-policy decoration, first-class
extension-hint interpretation, or post-render output mutation remains.

## Dependencies

This spec begins only after `tree-features` has:

- introduced alpha-bearing typed paint;
- added ergonomic sparse attr mutation;
- represented width-dependent text behavior with typed hints;
- attached Darkmatter policy during tree construction;
- represented supported browser link/image attributes as typed attrs;
- deleted decorate-time policy and post-render style/attribute rewriting.

## Scope

### Required closeout

1. Audit extension hints and production traversals.
2. Decide and document the `DarkmatterPage` page-frame boundary.
3. Re-baseline reviewed references and run complete verification.
4. Add final architecture assertions and benchmark baselines.
5. Update specifications, public docs, skills, and completion metadata.

### Audit and disposition artifacts

Each audit in this spec — the extension-hint inventory (section 1), the
production-traversal inventory (section 2), and the Biscuit Terminal component
assessment (section 8) — produces a durable record. Write each as a markdown
artifact in this closeout feature directory so the acceptance criteria stay
verifiable after the work lands:

- `extension-hint-inventory.md`;
- `traversal-inventory.md`;
- `component-assessment.md`.

Every row carries its disposition and a one-line rationale. These artifacts are
the evidence for acceptance criteria 1, 3, and 10. A green test run alone does
not satisfy them.

### "Production path" definition

Several scope boundaries below turn on whether a component is on the
**production path**. For this spec that means: reachable from the Darkmatter
document render pipeline (`Markdown -> Document -> target fold`) on Terminal,
Browser, Markdown, or MarkdownPlus. A component that is only constructed by
ad-hoc callers or is a terminal-only utility — for example `FileSystem`, which
the Darkmatter document pipeline never renders — is *not* on the production
path and does not block the parent cutover even if it implements
`TreeRenderable`. Required migrations (section 8) apply only to
production-path components; everything else is an optional, separately
specified follow-up.

### Separate component assessment

Evaluate remaining bespoke Biscuit Terminal components and classify each as:

- migrate to a canonical multi-target tree projection;
- add a typed renderer hook/node because the semantic content is cross-target;
- retain as an intentionally terminal-specific primitive;
- remove or replace if redundant.

This assessment is required. Implementing every resulting migration is required
only where the component is part of a production path covered by the parent
cutover. Other migrations become separately scoped follow-up features.

## Non-Goals

- Reopening the `PaintColor`, CSS box, sparse attr, or construction-policy
  designs completed by preceding specs.
- Requiring every `TerminalRenderable` utility to become multi-target.
- Adding speculative HTML, terminal graphics, or Markdown features.
- Preserving provisional pre-cutover Rust APIs or old render-tree serde shapes.

## 1. Extension-Hint Audit

Inventory every production `NodeAttrs::data` namespace and every
`set_hint`/`get_hint`/`remove_hint` call.

For each entry, record:

- producer;
- consumer;
- supported node kinds;
- whether a shared renderer interprets it;
- whether it affects first-class output behavior;
- disposition.

Apply this rule:

> If a shared renderer interprets the value or it changes first-class
> layout/style/semantic output, promote it to typed attrs. Keep extension hints
> only as opaque package metadata.

At minimum, review:

- `darkmatter.hr`;
- any residual `darkmatter.li`;
- any residual `darkmatter.style`;
- Darkmatter prompt/directive metadata;
- package-specific code, table, graphics, and image namespaces;
- compatibility-only `renderable.*` namespace constants and test access.

Delete stale namespaces and helpers. Update validation so renderable-owned
first-class data cannot regress into the extension bag.

## 2. Production-Traversal Audit

Inventory every recursive tree traversal in production after construction.
Classify it as:

- the target renderer fold;
- validation/diagnostics explicitly requested by the caller;
- a transformation required by a documented public API;
- obsolete policy/output preparation.

Delete obsolete preparation traversals. In particular, verify the absence of:

- `decorate_document` or replacements with equivalent component lookup;
- component-policy `LayoutContext` traversal;
- opacity or attribute sentinel injection;
- post-render HTML opening-tag mutation;
- pre-render link/image text replacement;
- target-width-derived mutation of the source tree.

Validation may remain a separate pass when explicitly requested or required at
an API boundary. Do not disguise policy decoration as validation.

## 3. Page-Frame Decision

Make an explicit final decision for `DarkmatterPage`.

### Option A: retain the slim page frame

Accept it as an assembler outside the document component tree when it is
limited to:

- terminal/page viewport width;
- outer page margin and padding;
- full-page background rows/wrapper;
- max-width centering;
- `PageBackground::Pronounced` code-theme contrast mode;
- browser page wrapper metadata and stylesheet assembly.

It must not carry per-component policy, inspect component node kinds, or mutate
component content.

### Option B: page as root box

Represent page geometry and paint on a typed document/root box and move
remaining page-frame lowering into the standard target folds.

Choose this only if it materially simplifies the retained frame without
compromising viewport-level behavior. Do not require Option B merely for
conceptual purity.

### Trade-offs and recommendation

**Option A — retain the slim page frame.**

- Pros: smallest change; keeps viewport-level concerns (terminal/page width,
  outer page margin, full-page background, max-width centering, code-theme
  contrast) in one assembler that is already correct; no risk to the
  component-tree folds; the page frame is genuinely a different concern from
  per-component box layout.
- Cons: one node-like responsibility (the page) is not a `RenderNode`; readers
  must understand the page frame as a documented exception to "everything is the
  tree."

**Option B — page as root box.**

- Pros: conceptual uniformity — page geometry and paint become ordinary typed
  attrs on a root box; no special-case assembler.
- Cons: forces viewport-only behavior (terminal page width, max-width centering,
  full-page background rows, `PageBackground::Pronounced`, browser page-wrapper
  metadata and stylesheet assembly) through folds that were not designed for
  page-frame concerns; risks regressions in the area the cutover most needs to
  keep stable; "conceptual purity" is the only driver.

**Recommendation: Option A.** The page frame is a viewport-level assembler, not
a component, and Option A already satisfies the parent thesis ("policy is baked
into node attrs; renderers fold") for everything *inside* the document tree.
Adopt Option B only if the audit shows the retained frame cannot be reduced to
the constrained responsibility list above without inspecting component node
kinds or mutating component content. This is the one closeout decision that
should be signed off before the parent is marked complete; see
[Open Questions](#open-questions).

Document the selected boundary in Darkmatter and renderable architecture docs.

## 4. Reference and Behavioral Verification

Review the five stale browser snapshots identified in `review-1.md`:

- `reference_block_quote_width_and_left`;
- `reference_list_left_margin`;
- `reference_page_background_pronounced`;
- `reference_centered_table`;
- `reference_table_max_width`.

If automatic horizontal centering is the intended CSS behavior, accept the
`auto` margin output with a rationale in the reference update.

Review all additional snapshot changes caused by typed alpha, direct policy
attachment, text-layout hints, and direct attribute emission. Characterization
snapshots are references, not immutable byte contracts, but every changed
behavior needs an explicit improvement/regression decision.

Run:

- complete `renderable` Level 1 tests and doctests;
- complete `biscuit-terminal` Level 1 suite;
- complete `darkmatter` and `darkmatter-cli` Level 1 suites without fail-fast
  omissions;
- cutover reference suites;
- applicable browser computed-style/geometry coverage;
- applicable real-terminal Level 2/3 box, width, color, and degradation
  coverage following the repository testing skill;
- MarkdownPlus and portable Markdown degradation coverage for alpha paint,
  typed text-layout, and browser attributes.

Do not declare closeout while the dedicated cutover corpus is red.

## 5. Architecture Assertions

Add tests that prove structure rather than merely snapshotting output:

- a styled Darkmatter source produces a complete initial `Document`;
- terminal and browser consume the same unmodified tree;
- rendering one tree at multiple widths does not mutate it;
- component policy is absent from render-time context;
- first-class rendering performs zero extension-bag accesses;
- browser fragment and streaming output agree for style and attributes;
- final browser output requires no post-render mutation;
- MarkdownPlus stays within its dialect policy (alpha CSS and supported
  attributes only, never a second browser renderer) and portable Markdown drops
  paint, geometry, and browser-only attributes;
- `InheritedStyle` is the sole text-appearance inheritance contract.

Where practical, use compile-time ownership and API removal as the strongest
assertion. Tests should guard behavior that types alone cannot enforce.

## 6. Performance Baseline

Extend the representative production corpus to include:

- alpha foreground/background;
- padding, border, fixed width, fit-content, and max-width;
- tables, block quotes, ordered/unordered lists, and list items;
- links and images with typed browser attrs;
- hyperlink/image/list width-dependent text hints;
- inherited page/component text appearance.

The structural gate remains authoritative:

- zero first-class extension-bag access;
- zero serde round-trips for typed attrs;
- zero per-node formatted hint keys for first-class behavior.

Record Criterion results for trend visibility against the post-tree-cutover
baseline. A timing regression is investigated and documented, but no flaky
wall-clock threshold becomes the acceptance gate.

## 7. Documentation and Metadata

Update:

- `renderable/docs/tree-rendering.md`;
- `renderable/docs/layout-and-style.md`;
- component migration guidance;
- renderable, biscuit-terminal, and Darkmatter skills;
- Darkmatter rendering/style documentation;
- public examples using the replacement `PaintColor` API;
- tree serde documentation to state the same-version-only contract;
- the CSS Box Architecture parent metadata and acceptance checklist.

Repair stale parent/child links so they point to actual active or completed
directories. In particular, the parent's `child_specs` frontmatter still lists
the superseded `2026-06-05-*` IDs and links to directories that no longer exist;
update it to reference the actual `_completed/2026-06-04-*` sub-specs and add
`tree-features` and this closeout as the concluding children. Move completed
feature directories only after verification is green and links are updated
consistently.

The parent closeout must state which behavior is:

- first-class typed tree intent;
- target-specific degradation;
- retained page-frame responsibility;
- intentionally terminal-only.

## 8. Biscuit Terminal Component Assessment

Review at least:

| Component | Assessment question |
|---|---|
| `HorizontalRule` | Should its rich styles project to typed thematic-break hints shared with Darkmatter HRs? |
| `GraphExpression` | Is source/graph semantics cross-target, with SVG/image lowering delegated to renderer hooks? |
| `MermaidDiagram` | Should the tree retain Mermaid source and let terminal/browser choose image/SVG/code fallback? |
| `TerminalImage` | Is this correctly retained as a terminal protocol primitive behind a generic image node/renderer? |
| `Status` | Is it a semantic status node or an intentionally terminal-only inline convenience? |
| `MetricsTree` | Can it project as ordinary structured prose/table/list nodes without losing meaning? |
| `InlineContent` | Is it redundant with structural inline children or `SequenceJoin`? |
| `PadLeft` / `PadRight` | Are these terminal field-formatting utilities superseded by typed text-layout hints? |
| `FileSystem` | Is the bespoke terminal icon path an accepted target specialization of its existing tree projection? |

For each component, write a short disposition and rationale. Create follow-up
specifications only for concrete migrations with user-visible value. Do not
block the parent architecture on terminal-only utilities that are outside
Darkmatter and already have an explicit specialization boundary.

## Acceptance Criteria

1. Every production extension hint is inventoried and classified.
2. No shared renderer consumes extension data for first-class style, layout,
   semantic attributes, or width behavior.
3. Production rendering is one complete tree build followed by one target
   fold, excluding explicit validation and the documented page frame.
4. The `DarkmatterPage` boundary is selected, constrained, tested, and
   documented.
5. No post-render browser style or attribute mutation remains.
6. Structural architecture tests cover the real styled Darkmatter entry points.
7. The structural performance gate passes on the expanded production corpus.
8. All required Level 1, cutover-reference, browser, MarkdownPlus/Markdown
   degradation, and applicable real-terminal verification is green.
9. Parent/child metadata, links, docs, examples, and skills describe the final
   architecture accurately.
10. Every listed bespoke Biscuit Terminal component has a recorded disposition;
    production-blocking migrations are completed and optional migrations are
    separately specified.
11. The CSS Box Architecture parent is marked complete only after criteria
    1-10 are satisfied.

## Sequencing

1. Run extension-hint and production-traversal inventories.
2. Resolve any remaining first-class hint/traversal findings.
3. Decide and document the page-frame boundary.
4. Add architecture assertions and expand the structural performance corpus.
5. Review/re-baseline snapshots and run full verification.
6. Assess remaining Biscuit Terminal components and create justified follow-up
   specs.
7. Update docs, skills, metadata, links, and parent completion status.

## Deliverables

- extension-hint inventory and dispositions (`extension-hint-inventory.md`);
- production-traversal inventory and dispositions (`traversal-inventory.md`);
- documented page-frame decision;
- green verification record;
- post-cutover benchmark record;
- updated architecture and user documentation;
- component assessment with follow-up links where required
  (`component-assessment.md`);
- completed CSS Box Architecture parent specification.

## Open Questions

This is the decision a reviewer should confirm before closeout is implemented.
Record the resolved outcome in the relevant audit artifact.

### Q1 — `DarkmatterPage` page-frame boundary (Option A vs Option B)

Treated in [Page-Frame Decision](#3-page-frame-decision) with a recommendation
of **Option A**, conditional on the audit proving the retained frame stays
within its constrained responsibility list. It is surfaced here because it is
the one choice that gates marking the parent architecture complete.

