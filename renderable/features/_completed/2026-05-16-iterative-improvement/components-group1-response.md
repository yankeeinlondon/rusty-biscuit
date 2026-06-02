---
last_updated: "2026-05-16"
responds_to: renderable/features/2026-05-16-iterative-improvement/components-group1-review.md
updates: renderable/features/2026-05-16-iterative-improvement/components-group1-spec.md
status: complete
---

# Group 1 Spec Review Response

## Summary

The spec was revised substantially in response to the architect review. The
largest change is that the spec now resolves blocker-level decisions up front
instead of leaving them as open questions. It also reframes proposed
architecture changes against the current code, adds concrete projection
contracts for all seven components, and defines a more rigorous Level 1 test
strategy.

## Changes Made

### Resolved `dyn TerminalRenderable` Projection

The spec now chooses the review's recommended Option A: add an optional tree
method to `TerminalRenderable`. This lets `RenderableTerminalContent` project
`Rc<dyn TerminalRenderable>` without downcasting and without replacing the
content container with a new trait object type.

The projection contract now includes:

- tree-capable components return `Some(RenderNode)`
- bespoke-only components return `None`
- `Strict` treats unsupported projection as an error
- `Warn` / `Lossy` can emit fallback content with diagnostics
- projection uses a recursion depth guard

### Moved Phase-Blocking Decisions Up Front

The old "Open Decisions" section deferred choices that blocked implementation.
The spec now has a **Settled Decisions** section covering:

- optional `TerminalRenderable` tree projection method
- first-class `NodeKind::Section`
- no `NodeKind::Progress` or `NodeKind::Columns` in group 1
- table cells as readable text plus typed metadata hints
- Phase 0 browser adapter with `Warn` default behavior
- code-render hook wiring through `TreeComponent`

### Reframed Existing Infrastructure Accurately

The spec now acknowledges that:

- `TreeComponent` already owns a `Layout`; the gap is layout population from the
  wrapped component and application through the renderer
- `TerminalRenderContext` already exists; the work is to extend and fork it for
  child rendering
- terminal tree rendering already delegates to structural components, making
  native rendering a hard ordering gate for meaningful parity

### Strengthened Cross-Flow Regression Rules

The spec now states that component parity is not sufficient when native tree
rendering changes shared Markdown node handling. Any phase that changes native
heading, list, or table rendering, or flips a component to tree-backed terminal
rendering, must also keep the darkmatter Flow A parity gate green.

This rule is repeated in the architecture section, test strategy, and phase exit
criteria.

### Added Browser Adapter to Phase 0

The browser `TreeComponent` adapter is now explicitly scheduled in Phase 0. The
spec adopts a default `Warn` policy because `BrowserRenderable` is infallible
while tree rendering is fallible.

### Added Concrete Test Strategy

The previous helper list was replaced with a concrete Level 1 testing strategy:

- structural JSON snapshots of `render_tree()`
- `validate()` with zero error-severity findings
- semantic parity after ANSI stripping
- positional parity for layout-sensitive components
- width matrix tests using `Terminal::new_optimistic`
- strictness and diagnostics matrix
- darkmatter Flow A parity as a named regression gate

The spec also places shared helper work in `biscuit-terminal/lib/tests/`.

### Added Component Projection Contracts

Each component now has a "Projection Contract" subsection describing:

- node shape
- hint keys
- lossy cases
- diagnostics behavior
- exit criteria

The contracts cover `Section`, `UnorderedList`, `OrderedList`, `YamlBlock`,
`Progress`, `TwoColumn`, and `Table`.

### Updated Sequencing

The sequence now includes **Phase -1: Decisions Gate** before Phase 0. Phase 0
then implements shared infrastructure against settled decisions.

The remaining order is:

1. shared foundations
2. `Section`
3. `UnorderedList` and `OrderedList`
4. `YamlBlock`
5. `Progress`
6. `TwoColumn`
7. `Table`

This preserves the original ordering intent while making the blocker decisions
explicit.

### Renamed Inclusion Section

The old "Components That Should Not Be Included" section was misleading because
the spec recommends including all seven. It is now **Inclusion With
Differentiated Rigor**.

## Intentional Choices

### Progress and TwoColumn Use Hints, Not New Node Kinds

The review recommended not adding core variants for `Progress` and `Columns`.
The updated spec adopts that approach. Both components use ordinary fallback
structure plus typed widget hints in group 1. A future generic `Widget` node is
left as a remaining open question after the hint-based approach has been tested.

### Section Gets a New Node Kind

The updated spec accepts the review recommendation to add `NodeKind::Section`.
This is the one new core node kind selected for group 1 because it represents
real document structure and avoids overloading `Heading` with body content.

### Table Metadata Starts in Hints

The spec chooses a pragmatic table projection: cells get readable formatted text
and table metadata is carried in hints. This keeps Markdown and browser output
useful while giving the terminal renderer enough information for alignment and
width planning. Moving table metadata into typed `NodeKind::Table` fields is
left as a future decision if hints become too large.

### Browser Adapter Uses Warn Semantics by Default

The spec chooses a default `Warn` policy for the browser adapter instead of
trying to force infallible browser rendering into strict tree semantics. Strict
callers should use the lower-level fallible browser tree renderer.

## Notable Follow-Up Work

The revised spec still leaves a few non-blocking questions:

- whether a generic `NodeKind::Widget` is needed after group 1 proves widget
  hints
- whether `Prose` needs a full structural projection before `Table` is flipped
- whether table metadata should eventually move from hints into typed node
  fields
- what additional strictness controls browser adapters should expose
