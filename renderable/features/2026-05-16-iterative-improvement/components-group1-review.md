---
last_updated: "2026-05-16"
reviews: components-group1-spec.md
source_documents:
  - renderable/docs/tree-rendering.md
  - renderable/features/2026-05-16-iterative-improvement/components-group1-spec.md
status: review
---

# Group 1 Tree Rendering Spec — Review

This document reviews `components-group1-spec.md` against the **current code**
in `renderable`, `biscuit-terminal`, and `darkmatter`. Its purpose is to
surface factual mismatches, coverage gaps, under-specified detail, and a
sharper test strategy *before* Phase 0 starts.

The review is grounded in a code survey of:

- `renderable/src/tree/` — `NodeKind`, `RenderNode`, `NodeAttrs`, `Document`,
  `TreeRenderable`, `RenderStrictness`, the Markdown/Browser renderers.
- `biscuit-terminal/lib/src/render_tree/` — `render.rs`, `component.rs`,
  `options.rs`.
- `biscuit-terminal/lib/src/components/` — `section.rs`, `list.rs`, `table/`,
  `two_column.rs`, `progress.rs`, `block_quote.rs`, `renderable.rs`.
- `darkmatter/lib/src/markdown/yaml_block.rs`.

## Verdict

The spec is directionally sound and the phase ordering (foundations →
structural → code/widgets → layout → tables) is the right shape. But it is
written one level of abstraction above the code, and several of its
"Architecture Enhancements" describe machinery that **already partially
exists**, while the single hardest problem — projecting heterogeneous
`dyn TerminalRenderable` content into a tree — is named but never actually
solved. The spec is not yet executable as written: at least three decisions
must be resolved before Phase 0 can produce code.

## 1. Factual corrections — spec vs. current code

These are not opinions; they are mismatches between the spec and what is on
disk today. Each should be reconciled in the spec.

### 1.1 `TerminalRenderContext` already exists

Architecture Enhancement #4 proposes to "Introduce a Width-Aware Terminal
Render Context." It already exists:
`biscuit-terminal/lib/src/render_tree/options.rs` defines

```rust
pub struct TerminalRenderContext { pub width: u32, /* ... */ }
pub struct TerminalRenderOptions { /* ... */ }   // carries strictness + context
```

`render_terminal_node(node, &TerminalRenderOptions)` already threads this.
Reframe #4 as **extend** the existing `TerminalRenderContext` with `indent`,
`available_width` (or rename `width`), and `layout` fields, and make it
**forkable for children** (`fn for_child(&self, indent_delta, width_delta)`).
Do not introduce a new type — that would orphan the existing one.

### 1.2 `TreeComponent` already carries a `Layout`

Architecture Enhancement #3 ("Layout Transfer") is written as if layout is
lost today. `TreeComponent` already has the field:

```rust
pub struct TreeComponent<T: TreeRenderable + Debug> {
    pub inner: T,
    pub layout: Layout,
    pub strictness: RenderStrictness,
}
```

The real gap is narrower and should be stated precisely: (a) `TreeComponent`
does not *read* a layout *off the wrapped component* — the caller must set
`.layout` by hand, and (b) it is unclear whether `render_terminal_node`
applies that layout, or whether nested tree nodes inherit it. Respec #3 as
"populate `TreeComponent.layout` from the component and ensure the terminal
renderer applies it," not "add a layout transfer mechanism."

### 1.3 The terminal tree renderer already delegates back to components

Architecture Enhancement #5 says delegation "becomes circular once those
components implement `TreeRenderable`." Confirmed and worth stating in
stronger terms — this is live today in `render.rs`:

- `NodeKind::Heading` → `Section::new(level, markup)` (`render.rs:144`)
- `NodeKind::List` → `OrderedList::from` / `UnorderedList::from` (`render.rs:166`, `:350`)
- `NodeKind::Table` → `Table::new()` (`render.rs:419`)
- `NodeKind::BlockQuote` → `BlockQuote::from(&str)` (`render.rs:161`)

This makes #5 a **hard ordering gate, not a cleanup step** — see §3.1.

### 1.4 `TreeRenderable` has no default methods

The trait today is exactly:

```rust
pub trait TreeRenderable { fn render_tree(&self) -> RenderNode; }
```

Enhancement #3's `fn tree_layout_hints(&self) -> Option<LayoutHints> { None }`
is a real, additive trait change — fine, but call it out as a trait-surface
change that every existing impl (currently only `BlockQuote`) must remain
compatible with. Prefer the default-method form over a separate `TreeLayout`
trait: a second trait fragments the contract and a `TreeComponent<T>` bound
would then need `T: TreeRenderable + TreeLayout`.

### 1.5 `NodeKind::ListItem` already has `checked`

`NodeKind::ListItem { checked: Option<bool>, children }` already exists.
Enhancement #7 ("Extend List Semantics") never mentions task-list items —
the list-hint design must not collide with the existing `checked` field, and
the parity tests should cover a checked item.

## 2. The unsolved core problem: projecting `dyn TerminalRenderable`

Architecture Enhancement #1 is correctly identified as "the most important
shared prerequisite" — but the spec describes the *interface* and never
resolves the *mechanism*. This is the single biggest blocker.

The content container is:

```rust
pub enum RenderableTerminalContent {
    String(String),
    Component(Rc<dyn TerminalRenderable>),
}
```

`Section`, `OrderedList`, `UnorderedList`, and `TwoColumn` all store content
this way. To project a `Component(Rc<dyn TerminalRenderable>)` into tree
nodes you must answer: **how does a `dyn TerminalRenderable` expose a tree?**
The spec's `TreeContent` trait and `to_tree_nodes()` method both sidestep
this — neither says how the conversion reaches a `TreeRenderable` impl that
lives behind a `TerminalRenderable` trait object. The spec even names the
trap ("fragile downcasts from `dyn TerminalRenderable`") without picking the
alternative.

There are exactly three viable mechanisms. **The spec must pick one in
Phase 0, because every component projection depends on it:**

| Option | Mechanism | Cost |
|--------|-----------|------|
| **A. Optional trait method** | Add `fn render_tree(&self) -> Option<RenderNode> { None }` to `TerminalRenderable`. | Touches the core trait; every component gets it for free; no downcast. **Recommended.** |
| **B. Supertrait + new container** | Define `trait TreeTerminalRenderable: TerminalRenderable + TreeRenderable`; store `Rc<dyn TreeTerminalRenderable>`. | Splits the component ecosystem into "tree-capable" and not; churns `RenderableTerminalContent`. |
| **C. Downcast via `as_any`** | `RenderableTerminalContent` already exposes `as_any`-style access; attempt `downcast_ref` to each known component. | The "fragile downcast" the spec rejects. Closed set, not extensible. Reject. |

Recommendation: **Option A.** It keeps one trait object, needs no change to
`RenderableTerminalContent`, and the `Option` return *is* the contract — a
component that has not adopted the tree returns `None`, and the projection
layer emits `Unsupported` (Strict) or an ANSI-stripped fallback paragraph
(Warn/Lossy), exactly as the spec's contract bullet already wants. Document
A as the decision and rewrite Enhancement #1 around it.

Secondary point: projection must have a **recursion depth guard**. `Section`
can contain a `Section` can contain a `Section`. The spec never bounds this.
Add a `max_depth` to the projection context with a diagnostic on overflow.

## 3. Coverage gaps

### 3.1 Component flips are not isolated — they change darkmatter output

This is the most serious omitted interaction. The spec treats each component
flip as local to that component's parity test. It is not.

The terminal tree renderer (`render.rs`) renders `NodeKind::Heading`,
`List`, and `Table` **by delegating to `Section` / `OrderedList` /
`Table`**. The darkmatter Flow A pipeline (`fold → render_terminal_document`)
goes through that same renderer. Therefore:

> The moment you give `Section` native tree rendering (Enhancement #5) or
> flip its `TerminalRenderable` body, you change what
> `render_terminal_document` emits for **every parsed Markdown heading** —
> which is gated by the darkmatter Flow A parity test
> (`render_tree_parity.rs`), not by any component parity test.

The spec needs an explicit rule: **every component flip in this group must
re-run the darkmatter Flow A parity gate, and "native heading/list/table
rendering" must not regress it.** Add this to the migration rule in §11 and
to each phase's exit criteria. Right now a green component parity test could
ship a darkmatter regression.

### 3.2 Phase 1's parity test is near-tautological until native rendering exists

The spec allows Phase 1 (`Section`) to land `TreeRenderable` while keeping a
"temporary non-recursive bridge." But if native heading rendering does *not*
exist, then `TreeComponent<Section>` → `render_tree()` → `Heading` node →
`render_terminal_node` → **builds a fresh `Section` and renders it**. The
"bespoke vs. tree" parity test then compares `Section`-bespoke against
`Section`-bespoke-via-a-detour. That proves the plumbing runs; it proves
almost nothing about fidelity.

Make native heading/section terminal rendering a **prerequisite for a
meaningful Phase 1 parity test**, not an "or temporary bridge" option. The
temporary bridge is acceptable only as a non-shipping intermediate state, and
the spec should say the parity gate does not count until delegation is gone.

### 3.3 The browser `TreeComponent` adapter is assumed but unscheduled

`tree-rendering.md` roadmap item #2 notes `TreeComponent` "only bridges the
terminal today." Yet the group 1 spec repeatedly promises browser output
("browser renders an idiomatic progress representation", "browser can render
a structural two-column container"). Six components claiming browser output
depend on an adapter that does not exist and is not in any phase.

Add the browser adapter explicitly to **Phase 0** (it is shared
infrastructure, like the terminal one). The spec must also resolve the error
policy `tree-rendering.md` flags: `BrowserRenderable::render_html_fragment`
is infallible, but tree rendering returns `Result`. Decide now: clamp
`Strict`→`Warn` like the terminal adapter does, and document it.

### 3.4 No validation coverage for component-produced trees

The shared rendering contract validates first (`validate` / `ensure_valid`).
Parsed-Markdown trees are well-formed by construction; **component-produced
trees are not** — a buggy `render_tree()` could place an inline node where a
block is expected, or nest a `ListItem` outside a `List`. The spec's test
section never mentions running validation on `render_tree()` output.

Add to Phase 0 / the parity helpers: every component's `render_tree()` output
must pass `validate()` with zero `Error`-severity findings, asserted as a
standalone test independent of any renderer.

### 3.5 Code-render hook wiring through `TreeComponent` is unspecified

Enhancement #9's `CodeRenderer` hook is sound, but `YamlBlock` lives in
`darkmatter` and will be wrapped in `TreeComponent`. `TreeComponent` today
holds only `inner`, `layout`, `strictness` — there is **no slot for a hook**.
A `TreeComponent<YamlBlock>` rendered to the terminal would therefore lose
syntax highlighting. The spec must either (a) add the hook to
`TerminalRenderOptions` *and* give `TreeComponent` a way to pass options
through, or (b) state that `YamlBlock` is rendered only via
`render_terminal_document` (not via `TreeComponent`) until that is resolved.
Pick one in Phase 3's spec text.

## 4. New `NodeKind` variants: resolve before phases, and minimize

The spec leaves "Should `Section` / `Progress` / `Columns` be `NodeKind`
variants?" in **Open Decisions**, yet Phase 1 cannot start without the
`Section` answer and Phase 4 cannot start without the `Progress` answer.
Decisions that gate a phase are not "open" — they are Phase 0 deliverables.

The cost of a new variant is also understated. The whole architecture rests
on an **exhaustive `match` over `NodeKind` in three renderers** plus a
documented serde JSON format plus the compile-time event-inventory tests plus
golden snapshots. Each new variant is a 3-renderer + serde + 2-test-suite
change. Recommended resolution:

- **`Section`** — add as a first-class `NodeKind::Section { depth, heading,
  children }`. It is genuine document structure, the spec's §6 analysis is
  correct that overloading `Heading` flattens body content, and parsed
  Markdown can keep emitting flat `Heading` + siblings (the renderers handle
  both). Worth the variant.
- **`Progress`** — do **not** add a core variant. It is not document
  structure. Project it as a `Paragraph`/`Span` carrying typed render hints
  in `NodeAttrs.data` (`renderable.widget.progress.*`), so the Markdown
  renderer degrades to `Label 75%` with no new match arm and the terminal
  renderer reads the hint. If a node is truly needed later, prefer one
  generic `NodeKind::Widget { widget_type, children }` over many targeted
  ones — one arm, not N.
- **`Columns`** — same as `Progress`: hints first, generic `Widget` if
  forced. `TwoColumn` is explicitly the "semantic projection only" component
  in this group; it does not earn a core variant yet.

Whatever is decided, the spec must state the **Markdown and Browser match
arms** for any new node. Right now it says "Markdown degrades to text" while
also proposing `NodeKind::Progress` — those contradict unless the arm is
specified. An exhaustive match has no implicit fallback.

## 5. Test strategy — make it concrete

The user asked specifically for a clearer test strategy. The spec's §11 lists
helper *categories* but no strategy. Recommended structure:

### 5.1 Test layer placement

State explicitly that all group 1 parity tests are **Level 1** in the
biscuit-terminal Level 1/2/3 vocabulary — they need no real terminal. They
must use `Terminal::new_optimistic(width)` for deterministic, width-pinned
output (this is what `render_tree_component_parity.rs` already does via its
`test_terminal()` helper). No Level 2 (real-terminal) work is in scope for
this group; say so, so nobody adds flaky PTY tests.

### 5.2 Four test tiers per component (not just token parity)

The existing `BlockQuote` gate is **token-presence after ANSI strip**. That
is too weak for layout-sensitive components — a list with the bullet in the
wrong column, or a doubled indent, passes a token-presence check. Each
group 1 component needs four tiers:

1. **Structural snapshot of `render_tree()`** — serialize the `RenderNode` to
   JSON and snapshot it. Cheap, renderer-independent, catches projection
   regressions directly. Flow A has this for documents; Flow B currently has
   nothing equivalent. **Add it.**
2. **Tree validity** — `validate(render_tree())` yields zero errors (§3.4).
3. **Semantic parity** — bespoke vs. tree, token presence after ANSI strip
   (the existing BlockQuote pattern). Keep, but it is the floor, not the bar.
4. **Positional parity** — for lists, sections, two-column, tables: assert
   *where* content lands. Bullet/number column index, hanging-indent column,
   block-child indent depth, two-column gap width, table column boundaries.
   Use visible-width measurement (`strip_escape_codes` + width). This is the
   tier that actually proves layout fidelity and the spec's §11 only gestures
   at it ("line width assertions").

### 5.3 Width matrix

Width-dependent components (`OrderedList` nested, `TwoColumn`, `Table`,
`Progress`) must be tested at a fixed **set** of widths — e.g. 40, 80, 120 —
not one. The spec mentions "narrow and wide" for Table only; make it a
standard matrix helper for the whole group. Ordered-list prefix width
specifically must be tested with items crossing 9→10 and 99→100, and with a
non-default `start`.

### 5.4 Strictness / diagnostics matrix

The spec says "strict/warn/lossy render assertions" without saying what each
component *does* in each mode. Each component spec section must include a
small table: what input triggers a `Diagnostic`, what is an `Error`, what is
accepted lossy. Concretely required cases:

- `Section` / lists with an unprojectable child component → `Unsupported`
  node + diagnostic in Warn; `RenderError` in Strict.
- `TwoColumn` with a `TerminalImage` column → explicit unsupported
  diagnostic in the tree path (the spec already wants this — make it a test).
- `BlockQuote`-style `Prose` flattening → diagnostic recording accepted
  styling loss (the existing pattern; replicate per component).

### 5.5 Cross-flow regression gate

Per §3.1: every phase that adds native heading/list/table rendering or flips
a component must include "darkmatter Flow A parity (`render_tree_parity.rs`)
still green" in its exit criteria. Make this a named, non-optional test step,
not an assumption.

### 5.6 Shared helper crate location

The spec says "shared test helpers" but not where they live. The component
parity helpers belong next to `render_tree_component_parity.rs` in
`biscuit-terminal/lib/tests/`. State this so each phase extends one helper
module rather than copying `normalize`/`strip` per test file.

## 6. Level-of-detail gaps per phase

The phases read as intentions, not specs. Each needs concrete, checkable exit
criteria. Examples of what is currently missing:

- **Phase 0** — "synthetic tests prove ... behave consistently" names no
  types. Specify: a `StubTreeComponent` (implements the chosen Option A
  method), a `StubBespokeOnly` (returns `None`), and assert the projection
  layer's three branches (string → paragraph, tree-capable → subtree,
  bespoke-only → `Unsupported`/fallback + diagnostic).
- **Phase 1** — does not say what `Section`'s `id`/`classes`/`data` carry,
  nor how `HeadingLevel` (the component enum, h1–h6) maps to `HeadingDepth`
  (the tree newtype). Specify the mapping and the out-of-range behavior.
- **Phase 2** — does not state how `indent_children` (a `u32` on
  `OrderedList`, an `Option<u32>` on `UnorderedList` — note the type
  mismatch) is carried. Specify the `renderable.list.*` hint keys and their
  JSON shapes.
- **Phase 6** — `TableCellContent` is a typed enum (Text / Number / Currency
  / Percentage per the code survey). The spec asks "preserved as metadata or
  pre-formatted" but does not decide. Decide: pre-format to a string in the
  cell's `Text` node **and** record the original typed value + alignment in
  `renderable.table.cell.*` hints, so Markdown/Browser get readable text and
  the terminal renderer can right-align numbers.

Recommendation: give every component a short "Tree projection contract"
sub-section — input fields → node kind → child shape → hint keys → lossy
items → diagnostics — so the phase is implementable without re-deriving it.

## 7. Recommended sequencing change: add a Decisions gate

Insert a **Phase −1: Decisions** (or fold into the front of Phase 0) that
*resolves*, with written answers, before any code:

1. `dyn TerminalRenderable` → tree mechanism (§2 — recommend Option A).
2. `NodeKind::Section` yes/no (§4 — recommend yes).
3. `Progress` / `Columns` representation (§4 — recommend hints, no variant).
4. Hint namespace + typed helper API surface (Enhancement #2).
5. Browser `TreeComponent` adapter error policy (§3.3).
6. Code-render hook wiring through `TreeComponent` (§3.5).

These six block Phase 0's deliverables; the current "Open Decisions" section
defers them past the phases that need them. Phase 0 then *implements* against
settled decisions instead of discovering them.

## 8. Minor / editorial

- The frontmatter lists 7 components; the prose says "6 components" in places
  and the title says "Group 1." Pick a count and use it consistently. (The
  task brief that motivated this work says 6; the spec covers 7. Reconcile.)
- §"Components That Should Not Be Included" is misnamed — it argues for
  *including all seven* with different *rigor*. Rename to "Inclusion with
  Differentiated Rigor" or similar.
- Enhancement #10 ("two-pass rendering") is sound and does not violate the
  exhaustive-match contract — a node handler buffering its descendants is
  still one arm. Worth stating that explicitly so a reviewer does not flag it
  as a contract break.
- The spec should note that `Rc<dyn TerminalRenderable>` is `!Send`/`!Sync`;
  projection consumes it into owned `RenderNode`s, so the resulting tree is
  `Send` — a feature worth stating, since it means projected trees can cross
  threads where the components cannot.

## 9. Summary of required spec edits

| # | Change | Severity |
|---|--------|----------|
| 1 | Resolve `dyn TerminalRenderable` → tree mechanism (Option A). | Blocker |
| 2 | Move `Section` / `Progress` / `Columns` node decisions out of "Open Decisions" into a Decisions gate. | Blocker |
| 3 | Add the cross-flow rule: component flips must re-run darkmatter Flow A parity. | Blocker |
| 4 | Schedule the browser `TreeComponent` adapter in Phase 0 with an error policy. | High |
| 5 | Reframe Enhancements #3 and #4 against existing `TreeComponent.layout` / `TerminalRenderContext`. | High |
| 6 | Make native heading/list/table rendering a prerequisite for meaningful parity (not optional). | High |
| 7 | Add the four-tier test strategy (structural snapshot, validity, semantic, positional) + width matrix + strictness matrix. | High |
| 8 | Specify code-hook wiring through `TreeComponent` (Phase 3). | Medium |
| 9 | Add per-component "Tree projection contract" sub-sections with concrete exit criteria. | Medium |
| 10 | Add recursion depth guard to the projection layer. | Medium |
| 11 | Fix the 6-vs-7 component count and rename the "Should Not Be Included" section. | Low |

Once items 1–3 are resolved in writing, the spec is executable; items 4–11
raise it from a direction sketch to an implementable plan.
