# Tree Rendering

This document describes the render-tree architecture introduced into the
`renderable`, `darkmatter`, and `biscuit-terminal` crates, how it relates to
the rendering paths that already exist, what is and is not covered by tests,
and a roadmap for adopting it.

It is a status-and-direction document. It is deliberately honest about what has
been *proven* versus what has only been *wired up*.

> **Status update — darkmatter document cutover complete (2026-06-02).** This
> document was written mid-migration. The darkmatter Markdown *document*
> pipeline has since fully cut over: `Markdown::as_html`, `Markdown::as_terminal`,
> and `DarkmatterPage::render` / `render_to_browser` now route through the
> render-tree document entry points (`render_tree_html` / `render_tree_terminal`
> / `render_tree_markdown` in `darkmatter/.../render_tree/entrypoints.rs`, all
> `pub`), and the legacy event-stream serializers (`output::as_html`,
> `output::for_terminal`) plus the `RuleProcessor` iterator adapter have been
> **deleted**. Span-aware folding of `==mark==`, dim, and HR-with-attributes
> landed with it. §2, §4, and §5 below have been updated to match; see
> [`renderable/features/2026-06-02-tree-cutover/`](../features/2026-06-02-tree-cutover/)
> and [`components.md`](./components.md). The **per-component** `render()` cutover
> (§3) is a separate track and remains partly in progress.

## 1. The tree-rendering architecture

The render tree is a **canonical, owned, target-agnostic representation** of a
document. Content sources produce it; render targets consume it. The slogan is
*parse once, build one tree, walk it per target*.

### Core model — `renderable::tree`

`renderable` owns the model so that every other crate can depend on it without
a dependency cycle.

- `RenderNode { kind: NodeKind, span: SourceSpan, attrs: NodeAttrs }` — the node
  envelope. `span` carries provenance (and an optional byte range); `attrs`
  carries identity (`id`), semantic `classes`, namespaced extension `data`, and
  an optional block-level `Layout` (margins, alignment, max-width, wrapping —
  see `layout-and-style.md`).
- `NodeKind` — a 25-variant payload enum covering document structure: `Root`,
  `Heading`, `Paragraph`, `BlockQuote`, `List`, `ListItem`, `Code`,
  `ThematicBreak`, `Table`, `TableRow`, `TableCell`, `FootnoteDefinition`,
  `Text`, `Emphasis`, `Strong`, `Delete`, `Span`, `InlineCode`, `Link`, `Image`,
  `FootnoteReference`, `SoftBreak`, `HardBreak`, `Html`, and `Unsupported`.
- `Document { sources: SourceRegistry, metadata: DocumentMetadata, root }` — the
  full document wrapper, with a source registry and frontmatter slot.
- `HeadingDepth`, `ColumnAlign` — constrained newtypes.

The whole public surface is `serde`-serializable to its own documented JSON
format (not MDAST-compatible).

### Producers

A tree has two kinds of producer, and the design treats them symmetrically:

1. **The fold** (in `darkmatter`) — turns a parsed document into a `Document`.
2. **`TreeRenderable` components** — a component implements
   `TreeRenderable { fn render_tree(&self) -> RenderNode }` to project itself
   into a document-structural subtree.

### Consumers — the renderers

Three renderers walk a `RenderNode` / `Document` with an **exhaustive `match`**
over `NodeKind` (no default-recursing visitor — adding a variant must break
every renderer until it makes a deliberate decision):

| Target   | Entry points                                          | Crate             |
|----------|-------------------------------------------------------|-------------------|
| Markdown | `render_markdown_node` / `render_markdown_document`   | `renderable`      |
| Browser  | `render_browser_node` / `render_browser_document`     | `renderable`      |
| Terminal | `render_terminal_node` / `render_terminal_document`   | `biscuit-terminal`|

The browser target adds `render_browser_document_html(doc, opts)`: a direct
`Document` → final HTML `String` path that streams the whole tree into one
buffer instead of building a `BrowserFragment` per node. Its bytes are identical
to `render_browser_document(doc, opts)?.output.render()` — same validation,
diagnostics, page options, head ordering, raw-HTML policy, and code-renderer
hooks. Reach for it when a caller already owns a `Document` and only needs the
final string (the cutover path and the browser perf benches); keep
`render_browser_document` for callers that compose through `HtmlPage` /
`BrowserFragment<Ready>`.

The Terminal renderer lives in `biscuit-terminal` because a meaningful terminal
renderer needs `Terminal`, `Layout`, color depth, and OSC8 — types `renderable`
cannot depend on. `renderable` gains **no** `pulldown-cmark` or
`biscuit-terminal` dependency; the dependency direction
(`darkmatter` → `biscuit-terminal` → `renderable`) is preserved.

### Shared rendering contract

Every renderer follows the same shape:

- It **validates first** (`validate` / `ensure_valid`). A structural `Error`
  fails the render regardless of strictness; warnings become diagnostics.
- It honors a `RenderStrictness` mode — `Strict` (any loss is an error),
  `Warn` (best-effort output plus diagnostics), `Lossy` (documented degrade).
- It returns `Result<Rendered<T>, RenderError>`, where `Rendered<T>` bundles the
  output with any non-fatal `Diagnostic`s.

`Unsupported` is a real, visible node — never a silent drop.

### The component adapter

`biscuit-terminal` provides `TreeComponent<T: TreeRenderable>`, which wraps a
`TreeRenderable` and supplies an (infallible) `TerminalRenderable` impl by
calling `render_tree()` then `render_terminal_node`. It is the bridge that lets
a tree-producing component render to the terminal. A sibling
`BrowserTreeComponent<T>` (`render_tree::browser_adapter`) provides the same
bridge for the Browser target via `BrowserRenderable`.

## 2. The darkmatter rendering path (now on the tree)

The darkmatter document pipeline routes through the render tree:

- `Markdown::as_html(HtmlOptions)` and `Markdown::as_terminal(TerminalOptions)`
  fold the parsed document into a `Document` and lower it through the
  render-tree browser / terminal renderers (`render_tree_html_from_document` /
  `render_tree_terminal`). The hand-written **`pulldown-cmark` event → string**
  serializers they used to call (`output::as_html`, `output::for_terminal`) have
  been **deleted**.
- `as_ast` (built on the `markdown` crate, producing MDAST) still exists as an
  independent structural-export feature; it is not part of the render path and
  nothing renders from its MDAST.
- The parser is built with darkmatter's shared parse options and the
  **span-aware** fold (`fold_markdown_spanned_with_frontmatter`), which preserves
  the custom `==mark==` / dim inline styles and `--- { … }` HR-attribute
  directives (parsed via `block::hr_parser`) along with their source offsets.
  The old offset-discarding `RuleProcessor` iterator adapter is gone; `hr_parser`
  survives as the attribute-parsing helper the fold calls.
- `compose/` transformations (transclusion, interpolation, TOC linking) are
  **still** implemented as string preprocessing and stream-mutating iterator
  adapters that run before the fold; moving them onto the tree is future work
  (see §5).

## 3. The structural-component rendering path

Twelve structural components in `biscuit-terminal` — `BlockQuote`, `Compose`,
`FileSystem`, `OrderedList`, `UnorderedList`, `Progress`, `Section`,
`StatusBlock`, `Table`, `TextBlock`, `Todo`, and `TwoColumn` — now implement
`TreeRenderable`, `MarkdownRenderable`, and `BrowserRenderable`. The default
`TerminalRenderable::render` path on each of them (with one caveat below)
projects to a `RenderNode` and lowers through `render_terminal_node`. The
Markdown and Browser targets render exclusively through the tree.

The shared shape per component is:

- A private projection helper (`to_render_node()` or equivalent) builds the
  canonical subtree.
- `TreeRenderable::render_tree` calls that helper.
- `TerminalRenderable::render` calls the helper, lowers via the tree
  renderer, and falls back to a `render_bespoke` companion only for the
  compatibility escape hatches the component still owns (e.g. a custom
  `BlockQuote` border prefix, `Table::prefer_cursor_alignment`, the
  `TwoColumn` image overlay).
- `TerminalRenderable::render_tree_node` is overridden to return the same
  projection, so when the component appears *inside* another component's
  tree the projector emits the structural subtree rather than falling
  back to "render to ANSI, strip, wrap in `Text`".

**Caveat — `FileSystem` keeps its bespoke terminal path (Stage 4
deferral).** Its Browser and Markdown targets route through the tree, and
its `render_tree_node` override is in place so cross-target adapters
consume it structurally. However `FileSystem::render` still calls the
bespoke directory-tree renderer because the connector geometry (`├──`,
`└──`, `│`) plus per-entry `Style` lowering through
`render_tree_connector_list` is not yet at parity. Stage 3 resolved this
as a Stage 4 deferral, and the named follow-on criteria live in
[`stage1-and-2/lessons-learned.md`](../features/2026-05-19-pushing-toward-ir/stage1-and-2/lessons-learned.md)
("Stage 4 acceptance criterion" under the FileSystem decision).

**Stage 3 closed the structural-projection gap.** After Stage 3 every
one of the twelve components — including `BlockQuote`, `StatusBlock`, and
`FileSystem` — implements `TerminalRenderable::render_tree_node` and
returns the same node as `TreeRenderable::render_tree`. Containers that
hold a non-`Prose` child component (`BlockQuote`, `Compose`,
`OrderedList`, `UnorderedList`) now project that child structurally
through `project_renderable_content(.., ProjectionMode::Structural)`
instead of falling back to ANSI-stripped text. The `RenderStrictness::Warn`
fallback in `RenderableTerminalContent::to_tree_nodes` and in the
terminal-hint short-circuit of `project_renderable_content` emit a
`tracing::warn!` (then `tracing::debug!`) keyed on
`TerminalRenderable::type_name` so a future component that forgets the
override is observable in logs and CI.

Stage 1 also landed eleven render-tree-functionality additions consumed
by the Stage 2 component impls — `SequenceJoin::None` (Compose),
`ListMarkerPolicy` (FileSystem), Browser and MarkdownPlus `ProgressHints`,
`set_table_title` and Markdown-safe cell escaping (Table), Browser `Style`
lowering for color, emphasis, underline, dim, and blink (TextBlock and
all subsequent consumers), `TaskState` / `TaskHints` (Todo), and
`ColumnsHints` lowering to Browser CSS and MarkdownPlus HTML (TwoColumn).
See
[`approved-render-tree-functionality.md`](../features/2026-05-19-pushing-toward-ir/approved-render-tree-functionality.md)
for the full RT-* ledger.

Inherently visual components (`TerminalImage`, `GraphExpression`) and
simple non-block helpers (`PadLeft`, `PadRight`, `InlineContent`,
`HorizontalRule`, `Status`) remain bespoke by design — they are out of
scope for the tree. `Prose` parses its bracket-tag grammar directly into
`RenderNode`, implements `TreeRenderable`, and renders every target through
the shared tree renderers; it has no component-local IR.

## 4. Testing coverage — and the gaps

The render tree supports two distinct flows, and they are **not** equally
tested.

### Flow A — parsed document → tree → render (well covered)

This is the darkmatter use case, and it is exercised end-to-end against real
input:

- **Event inventory** — compile-time exhaustive-match tests pin every
  `pulldown-cmark` 0.13 `Event` / `Tag` / `TagEnd` variant to a disposition; a
  parser enum change breaks the build.
- **Fold unit tests** — folding of every Milestone 1 construct plus footnotes,
  HTML-block grouping, and superscript/subscript.
- **Golden round trips** (`render_tree_roundtrip.rs`) — 11 real Markdown
  fixtures folded, structurally asserted, rendered back through the Markdown
  renderer, and snapshotted; plus a serialized `Document` JSON-surface snapshot.
- **Parity gates** *(historical)* — during the migration, `render_tree_parity.rs`
  ran the fold pipeline against real input and compared it, on semantic
  invariants, to the legacy `as_html` / `for_terminal` output. That test and the
  `migration_parity` bench were **deleted** at cutover, since the tree is now the
  only render path; behavior is covered by the fold / round-trip / snapshot
  suites and the per-target integration tests
  (`horizontal_rule_integration.rs`, `render_tree_hr_snapshots.rs`) instead.
- **Benchmarks** — fold + render stress benchmarks in both `darkmatter` and
  `biscuit-terminal` (tree-only, baseline-tracked; the bespoke-vs-tree
  comparison benches are gone).

Flow A is genuinely proven at the test level, not theoretical.

### Flow B — component → tree → render (twelve components, parity-gated)

This is the `TreeRenderable` use case. After Stage 2 of the IR push,
twelve `biscuit-terminal` components are adopted and each has a parity gate.
See
[`renderable/features/2026-05-19-pushing-toward-ir/lessons-learned.md`](../features/2026-05-19-pushing-toward-ir/lessons-learned.md)
for the per-component ledger.

- `BlockQuote`, `Compose`, `FileSystem`, `OrderedList`, `UnorderedList`,
  `Progress`, `Section`, `StatusBlock`, `Table`, `TextBlock`, `Todo`, and
  `TwoColumn` each implement `TreeRenderable` (plus `MarkdownRenderable` and
  `BrowserRenderable`). For all of them except `FileSystem`, the default
  `TerminalRenderable::render` is the **tree** path; the bespoke renderer
  is retained as a `render_bespoke` companion for parity comparison and
  for the compatibility escape hatches the component still owns.
- `TreeComponent` is unit-tested with synthetic stub types.
- **Per-component parity gates** live in
  `biscuit-terminal/lib/tests/`: `ordered_list_parity.rs`,
  `unordered_list_parity.rs`, `list_parity.rs`, `progress_parity.rs`,
  `section_parity.rs`, `status_block_parity.rs`, `table_parity.rs`,
  `text_block_parity.rs`, `todo_parity.rs`, `two_column_parity.rs`. The
  `BlockQuote` parity gate is in `render_tree_component_parity.rs`.
  `Compose` and `FileSystem` carry their parity / projection tests
  in-module rather than in a dedicated parity file.
- Each parity test renders the component *both* ways — the bespoke
  `render_bespoke` versus the tree path through `render_terminal_node` —
  and asserts semantic invariants (token presence after ANSI stripping,
  structural `NodeKind` for nested cases). Accepted divergences (e.g.
  border treatment, attribution placement, flattened nested non-`Prose`
  styling) are documented in the test bodies.

Flow B is **parity-gated across the twelve adopted components**, with the
discipline encoded as a separate test per component. The pattern that
worked once on `BlockQuote` has now been applied to every component
flipped to the tree.

### Known gaps

- **`FileSystem`'s terminal path is still bespoke.** Browser and Markdown
  flow through the tree; `FileSystem::render` does not. Stage 3 deferred
  the terminal flip to Stage 4 because connector-list style lowering and
  icon-name spacing still need parity work.
- **Fallback projection still exists for non-migrated components.** The
  Stage 3-adopted components now provide structural `render_tree_node`
  overrides, but a future component that returns `None` from that hook can
  still degrade to an ANSI-stripped text fallback under `Warn` / `Lossy`.
  The fallback now logs with `TerminalRenderable::type_name` so the missing
  projection is observable.
- **Lossy projection fidelity is characterized, not eliminated.** Text
  extraction from a `Prose` component remains lossy (styling flattened);
  the parity tests assert content survives and document styling loss
  as accepted.

## 5. Roadmap for integration

This is a direction sketch, not a detailed plan. Each step should land as its
own feature with its own parity gate.

### Done in Stage 1–2 — component adoption (`biscuit-terminal`)

1. **Component-side parity discipline established.** `BlockQuote`'s
   `render_tree_component_parity.rs` set the pattern: render both ways,
   assert semantic equivalence, document accepted divergences. Every
   subsequent adoption added its own parity test file (or in-module
   parity tests for `Compose` and `FileSystem`) before the component
   was flipped.
2. **Browser and Markdown adapters wired.** Both targets render the
   twelve adopted components through `render_browser_node` /
   `render_markdown_node`. The `BrowserRenderable` impls call the
   shared projection and lower via the tree; the same applies to
   `MarkdownRenderable`.
3. **Twelve structural components flipped:** `BlockQuote`, `Compose`,
   `FileSystem`, `OrderedList`, `UnorderedList`, `Progress`, `Section`,
   `StatusBlock`, `Table`, `TextBlock`, `Todo`, `TwoColumn`. For all
   except `FileSystem`, `TerminalRenderable::render` runs the tree
   path by default and `render_bespoke` is retained for parity and
   for the documented escape hatches.
4. **Inherently visual components left bespoke** by design:
   `TerminalImage`, `GraphExpression`. The simple non-block helpers
   (`PadLeft`, `PadRight`, `InlineContent`, `HorizontalRule`, `Status`)
   are likewise out of scope for the tree.
5. **Stage 1 RT-\* additions landed.** Eleven render-tree-functionality
   features (`SequenceJoin::None`, `ListMarkerPolicy`, `ProgressHints`
   in Browser and MarkdownPlus, table title and Markdown-safe cell
   escaping, Browser `Style` lowering, `TaskState` / `TaskHints`,
   Browser CSS and MarkdownPlus HTML lowering for `ColumnsHints`)
   gave Stage 2 the renderer vocabulary it needed.

### Done in Stage 3 — structural-projection completion

See [`stage3-spec.md`](../features/2026-05-19-pushing-toward-ir/stage3-spec.md)
for the full plan. The completed work:

- Added the missing `render_tree_node` overrides on `BlockQuote`,
  `StatusBlock`, and `FileSystem`.
- Deferred `FileSystem`'s terminal `render` flip to Stage 4 with explicit
  parity criteria.
- Tightened container nested-component parity tests from "text survives" to
  "structural `NodeKind` survives" for migrated children.
- Strengthened the `to_tree_nodes` fallback policy and added a stable
  `type_name()` hook to `TerminalRenderable` so missing overrides are
  observable in logs and CI.

### darkmatter migration

Steps 6–8 are **done** (2026-06-02 tree cutover):

6. ✅ **Deferred fold work resolved.** The span-aware fold
   (`fold_markdown_spanned_with_frontmatter`) folds `==mark==`, dim, and
   HR-with-attributes with source offsets preserved; the offset-destroying
   `RuleProcessor` adapter was removed and `block::hr_parser` survives as the
   attribute-parsing helper.
7. ✅ **Parity reached, then the harness retired.** The tree pipeline reached
   accepted parity with the legacy serializers across the corpus (including the
   raw-block-HTML the legacy `for_terminal` silently dropped); the parity harness
   and `migration_parity` bench were deleted once the tree became the only path.
8. ✅ **Public render paths migrated.** `as_html` routes through
   `render_tree_html_from_document` and `as_terminal` through
   `render_tree_terminal`; `DarkmatterPage::render` / `render_to_browser` route
   through the same entry points. The bespoke serializers are deleted.

Still open:

9. **Re-home `compose/`** transformations (transclusion, interpolation, TOC
   linking) as composable tree-rewrite passes; the node model already reserves
   the hooks (`SourceSpan` provenance, `NodeAttrs`, `DocumentMetadata`). These
   still run as pre-fold source transformations today.
10. **Retire or adapt `as_ast`** — either drop the MDAST path or implement a
    dedicated `Document → MDAST JSON` adapter if external consumers need it. It
    remains an independent, non-rendering export today.

### Guiding principle

Every migration step — component or darkmatter — should be gated by a parity
test against the renderer it replaces, the same discipline Phase 11 applied to
the darkmatter pipeline. The tree is adopted *because a parity gate proved it
faithful*, never on the assumption that it is.
