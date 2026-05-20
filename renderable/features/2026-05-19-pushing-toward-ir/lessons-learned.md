# Lessons Learned

> This page is a place to write down novel or surprising things we've encountered (as well as how we were able to address them) as they relate to designing and implementing components through an intermediate IR

## BlockQuote: semantic tree vs. compatibility prefix

`BlockQuote::with_border()` exposes a terminal-specific compatibility API that accepts an arbitrary prefix string, while the render tree represents block quotes semantically and styles borders through typed `Style::Border`. That arbitrary prefix should not be promoted into `NodeKind::BlockQuote`; otherwise the canonical tree would gain a component-specific terminal presentation detail.

The migration pattern for this kind of API is to route the normal/default component through the tree renderer and keep a small bespoke fallback for compatibility-only knobs that are not target-agnostic.

## Compose: document roots are not concatenation containers

`Compose` looks like it can project to a `NodeKind::Root` with one child per
part, but current Terminal and Markdown root rendering treats children as
document blocks and joins them with blank lines. That breaks Compose's core
contract: adjacent parts concatenate with no automatic separator.

The migration pattern is to add an explicit sequence/fragment join contract to
the render tree instead of overloading normal document-root behavior. Normal
Markdown document spacing should remain unchanged; Compose needs a deliberate
no-separator sequence marker or node.

## FileSystem: visual trees need marker policy, not pre-rendered connectors

`FileSystem` looks like a nested `List` / `ListItem` tree, but its terminal
output depends on connector geometry (`├──`, `└──`, `│`) that is presentation,
not document structure. Baking those connectors into `Text` nodes would make
the canonical tree terminal-specific and would degrade Browser and Markdown
outputs.

The migration pattern is to keep the tree semantic, attach typed `Style` for
terminal appearance, keep `fs-*` classes as browser/MarkdownPlus hooks, and add
a typed list marker policy for renderers that need custom marker presentation.

## OrderedList: projection hooks are not canonical tree adoption

`OrderedList` already exposes `TerminalRenderable::render_tree_node()`, and its
native tree shape is a clean `NodeKind::List { ordered: true }`. That looked
like complete tree adoption at first glance, but the cross-target adapters
(`TreeComponent` and `BrowserTreeComponent`) require the canonical
`TreeRenderable` trait instead of the terminal compatibility hook.

The migration pattern is to factor one private projection helper, make both
`TreeRenderable::render_tree()` and `TerminalRenderable::render_tree_node()`
delegate to it, then route Terminal, Browser, and Markdown output through the
tree renderers. This avoids drift between the old component hook and the
canonical render-tree producer contract.

## UnorderedList: terminal bullets are not Markdown markers

`UnorderedList` has two marker concepts that look similar but should stay
separate. The component and `bt list` can use a custom terminal bullet such as
`"• "` or `"→ "`, but the canonical Markdown renderer should still emit normal
CommonMark list syntax with `- ` markers. Custom bullets belong in typed list
render hints for targets that can present them, not in the Markdown structural
output.

The migration pattern is to carry custom bullets through `ListRenderHints` for
terminal rendering, ignore them for Browser and Markdown structural output, and
test that `bt list --md` remains valid portable Markdown even when `--bullet`
is set.

## Progress: compatibility projection hooks are not enough

`Progress` already had `TerminalRenderable::render_tree_node()` and `bt
progress` already used the tree renderer, so it looked close to fully migrated.
The missing piece is the canonical `TreeRenderable` trait: without it,
`TreeComponent`, `BrowserTreeComponent`, and future cross-target adapters
cannot consume the component directly.

The migration pattern is to factor one private projection helper, make both
`TreeRenderable::render_tree()` and the compatibility `render_tree_node()` hook
delegate to it, then route Terminal, Browser, Markdown, and MarkdownPlus
through the shared tree renderers.

## Section: layout must be seeded into the projected node

`TreeRenderable::tree_layout()` looks like the natural place to expose a
component's layout, but the current `TreeComponent` and `BrowserTreeComponent`
adapters render the `RenderNode` returned by `render_tree()` and do not apply
that optional hook. A component that only implements `tree_layout()` would lose
layout when rendered through today's adapters.

The migration pattern is to seed non-default layout directly onto the projected
root node's `NodeAttrs`, matching `BlockQuote`, and avoid carrying a second
adapter-level layout for the same component. Revisit `tree_layout()` separately
if the adapters are later changed to apply it.

## StatusBlock: default visuals can be semantic while escape hatches stay bespoke

`StatusBlock` initially looked like it needed either a custom callout node or a
terminal-specific border prefix in the tree because its default body border is
`"┃ "`. The existing `Style::Border` model already covers that default with a
thick left border, so the canonical projection can stay target-agnostic for the
normal path.

The migration pattern is to map defaults and typed overrides into `Style`, but
keep arbitrary terminal compatibility strings such as `StatusBlock::border()`
out of the tree. A narrow bespoke fallback is preferable to teaching the
render tree about target-specific prefix text that Browser and Markdown cannot
use semantically.

## Table: a tree-shaped hook is not the same as canonical tree adoption

`Table` already projects to `NodeKind::Table` through
`TerminalRenderable::render_tree_node()`, and `bt table` already renders that
projection through the terminal tree renderer. That looks complete until the
cross-target adapters are considered: `TreeComponent` and
`BrowserTreeComponent` consume `TreeRenderable`, not the terminal compatibility
hook.

The migration pattern is to factor one private projection helper, implement
`TreeRenderable::render_tree()` from it, and keep
`TerminalRenderable::render_tree_node()` as a compatibility delegate. This
prevents the terminal CLI path from drifting away from Browser and Markdown
rendering.

## Table: Markdown cells need a table-cell serialization mode

The Markdown tree renderer currently renders text as raw Markdown and joins
table cells with pipe delimiters. That is fine for simple fixtures, but it
breaks as soon as a cell contains `|` or a literal newline. Table components
support arbitrary text and multi-line cells, so a generic text renderer is not
enough inside `NodeKind::TableCell`.

The migration pattern is to keep the tree semantic and teach the Markdown
renderer a table-cell context: escape literal pipes, normalize soft breaks to
spaces, and normalize hard breaks or literal newlines to `<br>` so GFM table
structure remains valid.

## TextBlock: stored style fields can reveal dormant behavior

`TextBlock` looked like a straightforward parity migration because its public
fields map directly to `Style`, but the current bespoke terminal renderer only
applies italic and `FontWeight`. Foreground color, background color,
underline, strikethrough, and blink are stored by the component but inert in
`render()` / `render_optimistic()`.

The migration pattern is to separate legacy parity tests from activated-field
tests. Parity should prove the tree preserves existing behavior for fields the
bespoke path really renders, while the newly active stored fields should be
tested and documented as an intentional public behavior fix.

## Todo: checked state is not task state

`Todo` looks like a direct GFM task-list item because `NodeKind::ListItem`
already has `checked: Option<bool>`, but that field only captures checked,
unchecked, or ordinary-list semantics. Todo has five public states, and using
`checked: None` for intermediate states would accidentally render ordinary
bullets in Markdown instead of unchecked task items.

The migration pattern is to keep the tree semantically valid as a one-item
`List` with a `ListItem`, use `checked: Some(false)` for every non-completed
state, and carry the richer state in typed task hints. CSS classes remain useful
browser hooks, but renderer behavior should not depend on parsing class names.

## TwoColumn: compatibility hooks can hide missing canonical adoption

`TwoColumn` looked tree-ready because it already had
`TerminalRenderable::render_tree_node()` and renderer tests for
`ColumnsHints`. That hook is still a terminal compatibility surface, not the
canonical `TreeRenderable` trait that `TreeComponent` and
`BrowserTreeComponent` consume.

The migration pattern is to factor the projection into one private helper,
make `TreeRenderable::render_tree()` and the compatibility hook delegate to it,
then flip terminal/browser/Markdown output through the shared renderers. Tests
should include a serialized-node parity check between the two hooks so the
compatibility path cannot drift.

## TwoColumn: a structural carrier can be right while target lowering is incomplete

`ColumnsHints` on a `BlockQuote` carrier are enough to keep TwoColumn out of a
dedicated node kind, and all three renderers already recognize the hint. The
surprise is that recognition is not the same as preserving the full component
contract: Browser currently emits class hooks without width/gap CSS, and
plain Markdown necessarily loses side-by-side layout.

The migration pattern is to keep the tree shape stable, approve renderer
lowering only where the target can represent the semantics, and explicitly
document target loss. Browser and MarkdownPlus should preserve width/gap with
HTML/CSS; portable Markdown should stay sequential.

## SequenceJoin: a Root-only hint reuses the layout namespace and validation gate

`SequenceJoin` looked like it might need a dedicated `NodeKind` variant or a
new hint namespace, but the smallest faithful change is a typed hint on the
existing `Root` node stored under the `renderable.layout` namespace. The
surprise was how cheap the "structurally invalid positions" requirement turned
out to be: tree validation already rejects a nested `Root`, so the only extra
rule needed is "sequence-join hint must be on a `Root`." A nested sequence is
therefore expressible only as the top-level sequence — a test that nests one
`Root`-with-sequence-join inside another fails validation, and the correct
"nested sequence" test is a sequence whose children are ordinary blocks that
keep their own internal spacing.

Browser needed no Root change at all: HTML block elements never receive
renderer-inserted blank-line separators, so a sequence-join `Root` and a
document `Root` already render identically. The no-separator contract only
diverges on the Terminal and Markdown targets, which do insert `\n\n` between
document blocks.

## Table title: a hint on Table, not a NodeKind field, keeps the JSON shape stable

Adding `title: Option<String>` to `NodeKind::Table` would have changed the
serialized tree shape for every table. A namespaced `renderable.table.title`
hint with typed `set_table_title` / `table_title` accessors carries the same
information with no variant change. The renderers each place it differently —
`<caption>` before `<thead>` for Browser, a line above the top border for
Terminal, escaped plain text before the table for Markdown — but they share one
rule: an empty or whitespace-only title is ignored at render time, not at
storage time, so a component can set a title unconditionally.

## Table-cell Markdown escaping: a Writer depth counter, not a node-kind check

GFM table cells need pipe/newline escaping, but a node-kind check ("am I inside
a `TableCell`?") cannot see the ancestor chain during a depth-first fold. The
working pattern is a `table_cell_depth` counter on the Markdown `Writer`,
incremented around the `TableCell` arm and decremented after. Every leaf arm
(`Text`, `InlineCode`, `SoftBreak`, `HardBreak`) then branches on
`table_cell_depth > 0`. The subtle part is inline code: a literal `|` inside
`` `code` `` still breaks the pipe-delimited row, so the pipe must be escaped
*inside* the backticks even though escaping code content normally feels wrong.

## Browser Style lowering: emphasis splits into semantic wrappers and CSS

`Style` browser lowering is not a single CSS string. The spec deliberately
splits emphasis: bold/italic/strikethrough become semantic elements
(`<strong>`/`<em>`/`<s>`), while underline/dim/blink and color/background
become inline CSS. That forced a two-part implementation — `node_attributes`
gained style CSS declarations, and a separate `wrap_style_emphasis` post-pass
wraps the finalized fragment in the semantic tags. Splitting `render` into a
thin wrapper plus `render_kind` was the clean seam: `render_kind` does the
kind dispatch, `render` applies the emphasis wrappers exactly once per node.
`UnderlineStyle::css_declaration()` returns human-spaced CSS
(`text-decoration: underline`); the compact inline-style convention used
elsewhere needed a `": " → ":"` and `"; " → ";"` normalization pass.

## TreeConnectors: connector geometry is a recursive walk, not a per-item prefix

The terminal `render_list` builds one prefix per item, which is enough for
bullets and ordinals but not for `├──`/`└──`/`│` geometry. Connector geometry
needs depth, last-child state, and an accumulated ancestor continuation string
— information that only a recursive walk over the nested `List`/`ListItem`
structure carries. The working shape is a dedicated
`render_tree_connector_list(children, ancestor_prefix)` that splits each
item's own inline content (rides the branch line) from its nested `List`
children (recurse with `ancestor_prefix + "│   "` or `+ "    "`). Browser and
Markdown deliberately do *not* reimplement this: they degrade `TreeConnectors`
to a native nested list (Browser adds `list-style:none`; Markdown emits a
strictness-gated lossy diagnostic), because terminal box-drawing text has no
place in HTML or portable Markdown output.

## Shared lowering helpers prevent two targets from drifting

Progress HTML and two-column flex CSS are emitted by both the Browser renderer
and the MarkdownPlus path of the Markdown renderer. Rather than copy the HTML
shape twice, a small `tree::render::shared` module owns `progress_html`,
`columns_container_css`, `left_column_css`, and `color_to_css`. The browser
passes its layout CSS into `progress_html`'s `outer_style` parameter; Markdown
passes an empty string because Markdown ignores layout by contract. The single
shared definition is what makes "the same semantic progress HTML shape used by
the browser renderer" a checkable property rather than an aspiration.

## BlockQuote flip: the KNOWN_DRIFT ledger is part of the contract

Flipping a component's `TerminalRenderable` to the tree path retires the
component's entire `KNOWN_DRIFT` block in `render_comparison.rs` — once the
bespoke path delegates to the tree, every cell of the matrix yields identical
output by construction. The `render_matches_bespoke` test enforces this with a
`FIXED — remove from KNOWN_DRIFT` diagnostic that names every now-clean
`(component, scenario, facet, verdict)` row. The migration step is therefore
not just "flip and re-snapshot" but also "delete the now-fixed drift entries";
leaving them in fails the gate with a perfectly explicit error message. A
brief comment in `KNOWN_DRIFT` recording why the entries were retired keeps
the historical context discoverable when a later reader wonders whether the
ledger was ever exercised for this component.

## BlockQuote flip: side-by-side snapshots become tautological

The `layout_matrix` snapshot pairs each component's bespoke output above its
tree output, with the explicit goal of making divergences visible during
migration. After the BlockQuote flip both rows are produced by the same tree
renderer (`quote.render(&term)` now routes through `render_tree`), so the
snapshot still guards the rendered output but the *side-by-side* comparison no
longer demonstrates anything. That is acceptable as long as the snapshots are
regenerated and the bespoke/tree harness keeps exercising the non-default
`with_border()` compatibility path separately — which it does not, today.
The bespoke fallback is covered only by unit tests inside
`block_quote.rs`. If a future change wants to keep the matrix meaningful for
flipped components, the harness needs an explicit "compatibility-fallback
scenario" knob; otherwise the row reads as informational once a component
flips.

## BlockQuote inline Prose styling: project the IR, don't flatten ANSI

The first BlockQuote tree projection collapsed a `Prose` content component
through `plain_text()` — it rendered the prose optimistically and stripped
ANSI to derive a single `Text` node. That kept Markdown and Browser output
clean (their semantic projection of "quote a paragraph" doesn't need
terminal SGR), but it silently dropped inline emphasis on the terminal
target. `bt quote "<b>foo</b> ..."` rendered `foo` plain, contradicting
the documented example that advertises bold.

The fix is to project `Prose`'s private `ProseDocument` IR into
structured inline `RenderNode`s (`Strong` / `Emphasis` / `Delete`
wrappers for pure semantic emphasis; `Span` with a `Style` attribute for
color/dim/blink/underline; `Link` and `Code` for the matching kinds).
The renderable terminal tree renderer already lowers those inline kinds
through `text_appearance_sgr`, so terminal output gets full SGR back
"for free". Markdown and Browser fold the same structured nodes into
semantic wrappers (`<strong>` / `<em>` / `<s>`) — better than flattened
plain text — and Markdown still ignores color slots that have no peer.

A guard test (`test_prose_bold_inline_styling_survives_terminal_tree_render`)
pins the SGR byte-level expectation so a future regression that re-introduces
ANSI stripping inside the projection fails loudly. The semantic
parity-token comparison in `render_tree_component_parity.rs` is now too
coarse to catch this regression on its own — both paths emit the same
*words* — which is why an explicit byte-level positive pin matters.

## BlockQuote render-tree path: errors must not become in-band text

The first cut of `render_via_tree` surfaced a `RenderError` as a
`[render-tree error: …]` literal in the terminal output stream. That
satisfies the infallible `TerminalRenderable::render` trait contract but
pollutes user output with diagnostic strings. The right shape is to log
the failure through `tracing::error!` (which observability tooling
picks up) and fall back to an empty string. Panicking is also wrong:
`TerminalRenderable::render` is infallible by contract, and a panic
inside an in-band CLI render would crash the host process for what is
usually a transient render-tree validation failure.

## Compose flip: sequence-join Text nodes must bypass Prose wrapping

`SequenceJoin::None` was approved as the explicit no-separator render-tree
contract, and the renderers were taught to honor it on the `Root` node. But
the terminal renderer's default top-level rendering of an inline kind
(`Text`, `Emphasis`, `Strong`, `Span`, …) wraps the markup in a fresh `Prose`
instance via `render_prose` — and `Prose::render` strips trailing whitespace
to keep prose paragraphs tidy. That meant a Compose part like
`"Results:\n"` lost its caller-supplied newline when concatenated with a
following `Table`, breaking the literal-concatenation contract.

The fix is to special-case `render_sequence` in the terminal renderer: a
`NodeKind::Text` child renders its literal `value` verbatim (no Prose
instance), and other inline kinds run through `render_inline_node` to
lower styling tokens but then bypass `render_prose`'s whitespace trim.
Block kinds keep the normal block render path so a `Section`, `Table`, or
`List` inside Compose still gets its proper layout.

Markdown didn't have the same problem because the Markdown renderer
already treats `Text` nodes verbatim (modulo table-cell escaping). Browser
didn't have it either, because HTML block layout has no implicit
blank-line separators between siblings.

## Compose flip: nested Compose must inline its Root children

`Compose::render_tree()` returns a `NodeKind::Root` carrying
`SequenceJoin::None`. When a nested `Compose` appears as a child of an
outer Compose, the projection layer treats it as a tree-renderable
component and naively includes its `Root` node. The tree validator
correctly rejects nested `Root` nodes — `Root` is only valid as the
top-level container — so the outer tree would fail validation.

The projection in `Compose::project_part` flattens any nested `Root` into
its children before adding them to the outer sequence. This preserves
Compose's recursive no-separator semantics without breaking the
"`Root` only at top level" invariant: the inner sequence's contents
become siblings in the outer sequence, which is the same observable
output Compose's bespoke `render` produced.

## Compose flip: bespoke-only children need the caller's terminal

The shared `RenderableTerminalContent::to_tree_nodes` fallback uses
`Terminal::new_optimistic(80)` for the ANSI-stripping rerender when a
component lacks `render_tree_node`. That terminal advertises
`ImageSupport::Kitty`, which sent `HorizontalRule` straight to its Kitty
image tier and produced a base64 PNG blob — a frame full of binary that
survived ANSI stripping because Kitty graphics are APC, not CSI. The
sibling test (HR + plain prose inside a Compose) failed in a confusing
way: the expected `╌` glyph never appeared.

The pragmatic fix is to thread the caller's actual `Terminal` through
`Compose::render(term)`'s projection step. When a child has no
`render_tree_node` and a terminal context is available, render it through
the real target, strip ANSI, and embed the result as a `Text` node — so a
text-only terminal keeps HR on its Unicode tier. For pure `render_tree()`
calls (no terminal in scope) the shared default still applies, which is
the right policy for cross-target adapters.

## Compose flip: bt CLI needs ordered subarg parsing, not interleaved flags

The spec example `bt compose --heading 1 "Project Status" --text "Build:
" --prose ...` reads like a CLI that preserves the exact declaration
order across distinct flag types. Clap's derive macro doesn't natively
preserve that order: each repeatable flag collects into its own `Vec`,
and the order between, say, `--text` and `--prose` is lost. `ArgMatches`
does expose per-occurrence indices, but the derive surface doesn't make
that ergonomic.

The pragmatic compromise is to use a fixed, documented assembly order
(`heading → text → prose → list → ordered-list → table → positional
ITEMS`) and keep within-kind order intact via clap's default `Vec`
collection. Real `bt compose` invocations typically use one or two part
kinds, so the spec example is reproducible under this policy. A future
revision that needs strict interleaving can switch to manual
`ArgMatches::indices_of` parsing without changing the public surface.

## Compose flip: Table's `with_title` needed `set_table_title` on the projection

The Table component already has a public `with_title` that the bespoke
terminal renderer emits above the top border. The render-tree path for
Compose runs Table through `Table::render_tree_node`, which builds the
`NodeKind::Table` from columns/rows/hints — but it didn't carry the
title hint. RT-TABLE-001 is approved and the typed `NodeAttrs::set_table_title`
helper exists; Table's projection just needed one extra line to wire it
up. This is a small surgical fix outside the Compose component itself
but discovered by a Compose-level parity test (`test_add_table_with_title`).
The fix belongs in Table because the gap is in Table's tree projection;
Compose only revealed it.

## BlockQuote flip: Browser Style color lowering is already partially wired

`layout-and-style.md` §6 still says "Browser `Style` lowering of `Style` to
CSS is designed but not yet wired." That is true of `border`, `fill`, and the
emphasis-as-semantic-wrappers split, but the `color` slot is in fact lowered
to an inline `style="color:#…"` declaration on the element today. An
assertion like `html.contains("<blockquote>")` (with the closing `>`) on a
styled quote will therefore fail because the element actually renders as
`<blockquote style="color:#…">`. Tests for styled browser output should match
on `<blockquote` (no closing `>`) and `</blockquote>` separately, and the
status doc should be updated when color/background/emphasis lowering lands in
full.

## Compose polish: structural `is_empty` beats render-output probing

A first cut of the `bt compose` CLI used `compose.render_markdown().is_empty()`
as an "any parts provided?" probe. That works only because an empty Compose
projects to an empty `Root`, but it couples the CLI's argument-validation
path to renderer behavior and runs an entire Markdown pass to answer a
constant-time question.

The polish-pass migration is to expose a structural `Compose::is_empty()`
that inspects the in-memory `parts` slice. Callers in tests and CLIs get a
clearer intent line, and a future renderer change (e.g. emitting a placeholder
for an empty document) cannot silently break the emptiness check.

## Compose polish: LayoutArgs is a shared surface, not a per-command knob

The Compose spec calls for `LayoutArgs` to expose `--max-width` and
`--word-wrap`. Extending that struct would ripple through ~12 unrelated
subcommands (`flowchart`, `xy_chart`, `pie`, `quote`, `quadrant`, `dir`, …)
and surface flags that silently do nothing until each consumer wires them
through its own `apply_layout_args`. That is a worse UX than not having the
flags at all.

The deferred path is to keep the in-code `Layout` API authoritative (Compose
already honors `.max_width(..)` / `.word_wrap(..)` via `layout_mut()` and the
tree renderers), document the deferral in the Compose spec, and revisit the
`LayoutArgs` extension once the bulk of the migration is complete so the
rollout can land coherently across the shared arg struct. Surgical-change
discipline beats local optimization when the surface is genuinely shared.

## Compose polish: per-child clone in `render_sequence` is a hot-path tax

The sequence-join hot path in `Writer::render_sequence` originally did
`NodeKind::Text { value } => value.clone()` for every text child. Compose
documents typically hold many small text parts (UI labels, prefixes,
separators), so each render performed `n_text_parts` `String` allocations
that could simply push into the output buffer.

Inlining the push (`output.push_str(value)`) preserves the same behavior
without the per-child heap traffic. The lesson: the `match` arm pattern that
binds an owned `String` from a value-collecting match is a clone trap; if
the result is destined for `output.push_str(&part)` two lines later, push the
borrowed `&str` directly.

## FileSystem flip: `Layout::default()` already pins `WordWrap::None`

Forcing `word_wrap = WordWrap::None` on the projected `Root` looks like a
required guard against connector wrapping, but
`renderable::layout::Layout::default()` already pins `word_wrap` to `None` —
explicitly chosen for block components in the layout module's own default.
That means a FileSystem with no layout customizations produces a layout
equal to `Layout::default()` after the override, so the `if layout !=
Layout::default()` gate skips seeding the layout hint entirely. The first
parity test for the seeded hint failed not because the override was missing
but because the no-customization case correctly avoided a redundant hint.

The migration pattern is to split the test into two: one that proves a
non-default layout (`Alignment::Center`) is seeded *and* has `WordWrap::None`
applied, and one that proves the default layout intentionally omits the
hint. The implementation is correct as-is — the test was the artifact that
needed adjusting.

## FileSystem flip: two `BrowserRenderable::as_any` ambiguities

A component that implements both `TerminalRenderable` and `BrowserRenderable`
gains two `fn as_any(&self) -> &dyn Any` methods. Existing unit tests that
call `fs.as_any()` compile cleanly *until* the `BrowserRenderable` impl
lands; the second trait makes the call site ambiguous (E0034) and the test
must disambiguate with `TerminalRenderable::as_any(&fs)` (or
`<FileSystem as BrowserRenderable>::as_any`). This is a one-line fix per
call site, but it is invisible until the second trait is added — worth
checking when migrating any component that already had `as_any` exercised in
tests.

## FileSystem flip: `MetricConfig` is `pub(super)`, not `pub(crate)`

`MetricConfig` (the per-metric enable/threshold/filter struct) is declared
`pub(super)` in `filesystem/metrics.rs`, so it is reachable from `mod.rs`
but not from a sibling-of-`filesystem` projection module. The first cut of
this migration tried to extract the projection into
`filesystem/tree_projection.rs` and immediately had to either widen the
visibility (a non-surgical change) or reach the field through unsafe
casts (a worse mistake). The right call is to land the projection inside
`mod.rs` next to the existing `TerminalRenderable` impl — the file is large
but the projection is logically part of the same component surface, and
sibling modules in `filesystem/` do not get crate-local access to
`pub(super)` fields without bumping their visibility.

## FileSystem markdown: `RenderStrictness::Lossy` is the right default for `TreeConnectors`

The render-tree Markdown renderer raises a lossy diagnostic when it
degrades `ListMarkerPolicy::TreeConnectors` to a native nested list under
`RenderStrictness::Warn` (the default). For a component-driven Markdown
adapter like `FileSystem`'s, that diagnostic is purely informational —
the component *knows* the target lacks terminal box-drawing characters
and chose this projection anyway — so propagating it would clutter the
CLI output stream without giving the user new information. Setting
`strictness: RenderStrictness::Lossy` on the per-render `MarkdownRenderOptions`
suppresses the diagnostic at the renderer level, matching the component's
deliberate Markdown contract. Browser rendering takes the same default
because `<ul>`/`<li>` with `list-style: none` is the right baseline; the
diagnostic is similarly redundant.

## FileSystem: project portable icons separately from terminal/Plus icons

The Unicode fallback icons (📂, 📄, ⚠, 📁) are correct for *terminal and
MarkdownPlus and Browser* rendering — they're a portable contract that
survives even when Nerd Fonts are absent. They are *not* correct for
plain Markdown: the spec asks for icons to be omitted there entirely so
the output round-trips through renderers that lack emoji-capable fonts.

The clean projection pattern is to keep the icon as a classed `Span`
(class `fs-icon`) in the canonical tree, then have the plain-Markdown
adapter walk the tree and strip every `fs-icon` span (plus the literal
" " separator that follows it) immediately before handing the tree to
the Markdown renderer. MarkdownPlus and Browser keep the spans because
they have CSS hooks to control whether to render them.

This is cleaner than threading a "for_plain_markdown" flag through the
projection itself — projection stays single-purpose, the adapter owns
its dialect-specific transformations, and the in-tree class survives as
a CSS hook in the dialects that want it.

## FileSystem: file:// links need a canonicalized base, threaded once

When `FileSystem::new(".")` projects entries, each entry computes its
`file://` URL as `current_path.join(name)`. If `current_path` is the
*raw* root_path (e.g. `.`), the URL embeds `./` — producing `file://./src`
instead of `file:///abs/path/src`. The bespoke ANSI renderer already
solved this by canonicalizing `root_path` once at the top of
`render()` and threading the canonical `base_path` through
`render_nodes`; the canonical-tree projection has to mirror that exactly,
both for the root header *and* for every entry. Two separate fixes (one
in `fs_project_root_header`, one in `fs_render_tree_inner` threading
into `fs_project_tree_node`) are required — they cannot share state
because the root header is a sibling of the entries' parent list, not
an ancestor.

## FileSystem: metric threshold highlight requires structural projection

The projection's first attempt at metrics was to call
`format_metrics(.., is_tty=false)` and embed the resulting plain-text
string in a single `fs-metrics` span with a dim Style. This works for
labels but flattens away the "value exceeds threshold → bold yellow"
signal entirely, because `format_metric_pair` only emits the bold-yellow
SGR sequence when `is_tty=true`.

The fix is to project metrics as a structured inline-span sequence
instead: dim `<span>label:</span>` nodes for each label, a plain
`Text(value)` for unhighlighted values, and a
`<span class="fs-metric-highlight" style="bold yellow">value</span>` for
threshold-exceeded values. The Style now survives into Terminal, Browser,
and MarkdownPlus rendering; plain Markdown drops the Style (as expected)
but keeps the value text intact.

## OrderedList flip: switching the default `render()` reshapes unit-test expectations

Flipping `OrderedList::render(&term)` from the bespoke renderer to the
canonical render tree turned several in-source unit tests into byte-level
mismatches even though semantic content was unchanged. The bespoke path
emitted a trailing `\n` after the final item and a true blank line (`""`)
between the previous item and a following item when an empty nested list
sat between them. The tree renderer omits the trailing newline and pads
the blank line up to the indent width (`"    "`) so it visually lines up
with sibling items.

The cleanest migration is to update the in-source unit tests to assert
the tree-renderer's output (the user-visible canonical behavior after
the flip) and document the byte-level divergences as accepted
`KNOWN_DRIFT` in comments. Integration parity tests in `tests/` then
compare `OrderedList::render_bespoke(&term)` against
`OrderedList::render(&term)` under ANSI-stripped, whitespace-normalized
equality — they should agree on items, numbering, indentation, and order
without being sensitive to the documented trailing-newline / padded-blank
differences. Tightening unit tests to byte equality with the tree output
is the right move because that *is* now the user-facing contract; the
bespoke path is preserved only as a `#[doc(hidden)] pub fn render_bespoke`
for parity tests, not as a behavior anyone should depend on.

## OrderedList flip: Prose styling in items is a documented loss, not a regression

The first parity-test attempt for an `OrderedList` containing a styled
`Prose` item asserted that the bold SGR survives the tree path
("`\x1b[1m`...`\x1b[22m`"). That fails today because
`Prose::render_tree_node()` returns `None`, so the
`RenderableTerminalContent::to_tree_nodes` fallback renders Prose to ANSI
and then strips it before embedding into the projected tree as plain
`Text`. The OrderedList spec explicitly lists this as a `KNOWN_DRIFT`:
"Prose styling loss".

Recovering Prose styling inside ordered-list items needs the same
downcast pattern that `BlockQuote` and `Compose` use — convert
`RenderableTerminalContent::Component` to a structured inline node
sequence via `Prose::to_render_nodes()` before falling back to the
generic projection. That is intentionally outside this migration's
scope to keep the change surgical: `OrderedList` already had a working
`render_tree_node()` projection, and the spec's acceptance criteria
focus on flipping the default render path, not on rescuing per-item
inline styling. The parity test should therefore assert that Prose
*content* survives the tree path (which it does), not that *styling*
does.

## OrderedList flip: cross-target CLI flags require cross-target impls

The spec's CLI section asks for `--md`, `--md-plus`, and `--html` switches
on `bt list`. Adding those switches to a single `bt list` subcommand that
serves *both* `OrderedList` and `UnorderedList` looks symmetric, but only
`OrderedList` implements `MarkdownRenderable` and `BrowserRenderable`
after this migration — `UnorderedList`'s own IR flip is still pending.
The pragmatic shape is to keep one subcommand with the `--ordered` /
`-o` switch, and reject `--md` / `--md-plus` / `--html` *without*
`--ordered` at runtime with a clear error message that names the missing
prerequisite. The CLI surface stays consistent (no per-list-kind
subcommand split), the failure mode is explicit, and the unordered IR
migration can later flip a single error site to "do the cross-target
render" without renaming flags or breaking compatibility.

## OrderedList: third Prose-downcast site → consolidate the pattern next

`OrderedList::project_list_items` is the **third** container that needed the
`as_any().downcast_ref::<Prose>().to_render_nodes()` pattern after
`BlockQuote::paragraph_children` and `Compose::project_part`. Each migration
in this set initially accepted "Prose styling loss" as `KNOWN_DRIFT`, then
reversed the decision after review pointed out that other containers had
already established the precedent.

The migration pattern is to **promote the downcast to a shared helper** the
next time a container needs it — for example a `pub(crate)` function
`project_terminal_content_with_prose(content) -> Vec<RenderNode>` in
`biscuit-terminal/lib/src/render_tree/projection.rs`. The helper would
encapsulate the `Prose` downcast plus the generic
`RenderableTerminalContent::to_tree_nodes` fallback, so a fourth container
(e.g. `TwoColumn`, `StatusBlock`, `Section`, `Table`) does not have to
re-derive the pattern. Doing the refactor now would require re-running
BlockQuote and Compose suites to confirm no regression, which is out of
scope for surgical follow-ups but should be the *first* step of the next
container migration that would otherwise add a fourth duplicate.

## OrderedList: inline siblings must share one `Paragraph`

When projecting a `Prose` item into a `ListItem`, returning
`prose.to_render_nodes()` directly as the item's children produces
**multiple top-level inline nodes** (e.g. `Strong("Bold")` + `Text(" item")`).
The terminal list renderer (`render_list_item`) only attaches the prefix to
the *first* inline child and treats every subsequent sibling as a block
child — which then gets `indent_children` applied and lands on its own
indented line. The visible failure for `<b>Bold</b> item` is:

```
1. Bold
     item
```

The migration pattern is to wrap the inline projection in a single
`Paragraph`: `RenderNode::list_item(None, vec![RenderNode::paragraph(prose.to_render_nodes())])`.
`BlockQuote::paragraph_children` already does this implicitly because its
caller wraps in `RenderNode::paragraph(...)`. `Compose::project_part` does
not, because Compose's outer container is a `Root` that treats children as
separate blocks — exactly the behavior Compose wants. The lesson is that
the wrapping decision is **caller-dependent** and the shared downcast helper
proposed above should not pre-wrap; each caller wraps as appropriate for
its container semantics.

## OrderedList: tree-path CSS already lowers `Layout` — don't wrap twice

When the CLI dispatches `bt list -o --html --margin-left N`, the tree path
already emits `margin-left:Nch` on the `<ol>` element via
`layout_to_css` in `renderable/src/tree/render/browser.rs`. Wrapping the
fragment in an additional `<div style="margin-left: Nch">` doubles the
property's effective value and quietly breaks layouts.

The migration pattern is to **only wrap the fragment for properties the
tree path cannot express on the component's own root element**. For lists
today that is `text-align` from `--alignment` *without* a `max_width` — the
tree path emits `margin-left:auto` / `margin-right:auto` for centering only
when `max_width` is present, so an alignment-without-max-width still needs
a surrounding `<div style="text-align: …">`. Every other `LayoutArgs`
property has an `<ol>`-level peer and should be applied via
`list.layout_mut()` only, not re-applied on a wrapper.

A regression test that asserts `stdout.matches("margin-left").count() == 1`
makes this drift impossible to land silently.

## UnorderedList flip: the Prose-downcast helper, finally consolidated

The OrderedList migration explicitly called for promoting the
`as_any().downcast_ref::<Prose>().to_render_nodes()` pattern to a shared
helper "the next time a container needs it." UnorderedList was that next
container, and the consolidation landed as
`project_renderable_content(content, ProjectionMode)` in
`biscuit-terminal/lib/src/render_tree/projection.rs`. The single helper now
serves four migrated callers — `BlockQuote`, `Compose`, `OrderedList`, and
`UnorderedList` — with one source of truth for the Prose-downcast escape
hatch.

The non-obvious shape of the API is `ProjectionMode` with two variants
rather than a bool flag, because the "what does a non-Prose component
become?" question has genuinely different answers:

- `ProjectionMode::InlineOnly` — every non-Prose component flattens to an
  ANSI-stripped `Text` node, matching `BlockQuote`'s wrap-in-`Paragraph`
  semantics.
- `ProjectionMode::Structural { terminal_hint }` — block-capable
  containers (`Compose`, `OrderedList`, `UnorderedList`) keep their
  child's `render_tree_node()` when available; an optional `terminal_hint`
  lets `Compose` thread its caller's real terminal through for capability-
  sensitive bespoke-only children (e.g. `HorizontalRule`'s text tier vs
  image tier).

The wrapping decision (`Paragraph` for list items, none for Compose, one
big `Paragraph` for BlockQuote) is intentionally **left to the caller** —
the helper returns inline nodes and trusts each container to know whether
its outer kind treats children as block siblings or inline runs.

## UnorderedList flip: cross-target CLI flags become per-list-kind impls

The OrderedList CLI lesson called out that `--md`/`--html`/`--md-plus`
returned a runtime error when used without `--ordered`, because
UnorderedList did not yet implement `MarkdownRenderable` and
`BrowserRenderable`. UnorderedList's IR flip retired that error gate: both
list kinds now implement all three target traits, and the CLI dispatches
to whichever target is selected without an `--ordered`-only prerequisite.

The implementation pattern is a `render_terminal<L: TerminalRenderable>`
helper that works for either list kind, and two `dispatch_*_render`
methods (`dispatch_ordered_render` / `dispatch_unordered_render`) that pick
the right list type at the boundary. The two methods are *not* one generic
function, because `OrderedList` and `UnorderedList` do not share a
"renders to all three targets" trait — adding one would be premature
abstraction for two callers.

The user-visible surface is also more consistent: a custom terminal
`--bullet` is honored on the terminal target only, and the Markdown and
HTML paths emit `- ` / `<ul>`/`<li>` regardless. The CLI tests
(`test_list_unordered_md_ignores_custom_bullet`,
`test_list_unordered_html_ignores_custom_bullet`) pin this contract so a
future renderer change cannot leak the bullet into a non-terminal target.

## UnorderedList review: "verbatim duplication" claims need a pre-flight diff

The review of the UnorderedList migration flagged `render_html_with_layout`
/ `render_markdown_with_layout_frontmatter` / `layout_style_frontmatter` /
`wrapper_only_css` as "defined verbatim" in both `cli/src/commands/prose.rs`
and `cli/src/commands/list.rs`, recommending promotion to a shared helper.
A line-by-line diff showed only the Markdown pair (`render_markdown_with_layout_frontmatter`
and `layout_style_frontmatter`) is byte-identical between the two files.

The HTML pair is **deliberately different**:

- `prose.rs::render_html_with_layout` wraps the fragment in a `<div>` and
  applies every `LayoutArgs` property as inline CSS (margins on all four
  sides plus `text-align`), because Prose's HTML fragment is a single
  `<span>` and has no element of its own to receive layout.
- `list.rs::render_html_with_layout` calls a `wrapper_only_css` helper that
  emits **only** `text-align` (when `--alignment` is present without a
  `max_width`), because the tree path already lowers margins onto the
  component's own `<ol>` / `<ul>` via `layout_to_css` in
  `renderable::tree::render::browser`. Wrapping in a `<div>` that also
  carries `margin-left` would double-apply the property — a regression
  that landed and got reverted in the OrderedList migration.

The lesson: when a review highlights duplication, run the diff before
promoting. Pure code duplication is a code-smell; deliberate
shape-divergence dressed up as duplication is a footgun if you collapse it
into one helper. Promoting only the truly-identical helpers
(`render_markdown_with_layout_frontmatter` + `layout_style_frontmatter`)
to `cli/src/commands/shared.rs` is the correct surgical fix; the
component-specific HTML wrappers stay where they are, with a comment
explaining why one wraps full layout and the other does not.

## UnorderedList review: thread `terminal_hint` through every list projection

The Compose migration was the first container to thread the caller's real
`Terminal` into `project_renderable_content`, so bespoke-only children
(notably `HorizontalRule`) honor the actual target's capability tier
instead of the projection layer's optimistic default. `project_list_items`
was originally added with `terminal_hint: None` and never updated, leaving
the two list kinds inconsistent with Compose for the same fallback
scenario.

The fix is a private `to_render_tree_node_with_terminal(Option<&Terminal>)`
on both `OrderedList` and `UnorderedList`, mirroring Compose's
`render_tree_with_terminal`. `TreeRenderable::render_tree` continues to
pass `None` (it is target-agnostic and has no terminal in hand), while
`render_via_tree` — the bridge from the `TerminalRenderable` contract
into the tree renderer — passes `Some(term)`. `project_list_items`
itself now takes `terminal_hint: Option<&Terminal>` and forwards it
straight to `ProjectionMode::Structural { terminal_hint }`.

This is asymmetry-by-omission, not by intent. The migration playbook
should record: **any container that calls `project_renderable_content`
needs a `terminal_hint` plumbing pass when its `render_via_tree` runs.**
The shared `ProjectionMode::Structural { terminal_hint }` field is the
only documented place this affordance lives; helpers that paper over it
with `None` silently degrade the capability story.

## UnorderedList review: `pub(crate)` symbols are not doctest-runnable

A doctest fenced with bare ` ``` ` on the `pub(crate)` helper
`project_renderable_content` compiled fine in isolation (it lives inside
the crate's lib) but **fails when rustdoc compiles it as an external
crate** with `error[E0603]: enum 'ProjectionMode' is private`. Rustdoc
builds doctests as if they were downstream consumers, so any item touched
inside a doctest must be `pub`, not `pub(crate)`.

The fix for crate-internal helpers is to fence the example with
` ```ignore ` and add a one-line note that the snippet illustrates
in-crate usage. The alternative — promoting the helper to `pub` purely
to satisfy a doctest — would over-expose a deliberately-internal API.
The lesson: every newly-added doctest on a `pub(crate)` item needs an
`ignore` (or `no_run`/`compile_fail`) marker.

## Progress flip: top-margin snapshot was lying before the flip

The `layout_matrix` `Progress__top_margin_2` snapshot was committed in a
shape where the BESPOKE half showed no leading newlines and the TREE half
showed two. That looked like the bespoke renderer correctly skipping
zero-content top-margin output and the tree renderer (incorrectly)
emitting blank rows — but in reality the bespoke `apply_block_layout`
path never honored `margin.top` on a single-line block. The snapshot was
a faithful record of a renderer-divergence bug.

Re-pointing `TerminalRenderable::render` at `render_via_tree` made both
halves identical (`\n\n` + `Loading [...] 75%`), which is the correct
top-margin behavior. The snapshot needed regeneration; the *user-visible
fix* is that top margins on a `Progress` finally render. The migration
playbook should record: a snapshot that "passes" under bespoke can still
encode a behavior gap the flip will close, so snapshot diffs during the
flip are evidence of the fix, not regressions.

## Progress flip: the spec's `render_optimistic` shape needs `Terminal::new_optimistic`

The Progress spec's `render_via_tree` example renders against the
supplied `&Terminal`, and its note about `render_optimistic` says
"construct `Terminal::new_optimistic(width)` and then call
`render_via_tree()`." The trap is that the pre-flip `render_optimistic`
called `render_bar(ColorDepth::TrueColor)` directly (no terminal), so
re-routing through the tree requires materializing a terminal with
`TrueColor` depth — and `Terminal::new_optimistic(width)` is exactly
that. Using `Terminal::default()` instead would silently downgrade color
behavior on environments where defaults differ. The lesson: when the
spec says "construct an optimistic terminal," it really means
`new_optimistic(width)`, not `default()`. The other migrated components
(`OrderedList`, `UnorderedList`, `Compose`) follow this same pattern,
which made the right choice obvious in context.

## Progress flip: `BrowserRenderable` re-export, not the renderable crate path

`biscuit-terminal::components::renderable` re-exports
`renderable::browser::BrowserRenderable`, and the existing tree-flipped
components (`OrderedList`, `UnorderedList`, `Compose`, `BlockQuote`) all
import it from the local `crate::components::renderable` path. The
renderable crate's own `BrowserRenderable` is the same trait — but
mixing the two import paths leaks an extra dependency surface into
biscuit-terminal that isn't there in the other migrated components.
Following the established import convention keeps the per-component
header consistent and avoids accidentally implementing the wrong
"BrowserRenderable" if the workspace ever splits the traits.

## Progress flip: every clap subcommand needs an `--example` flag, even cross-target ones

The `bt` `test_every_subcommand_help_exposes_example_flag` smoke test
iterates a fixed list of subcommands and asserts each one exposes
`--example` in its help. Adding `--html`, `--md`, and `--md-plus` to
`bt progress` doesn't change that contract, but a forgetful version of
the migration that wired the new flags as a non-overlapping
`OutputArgs`-style enum could remove the per-command `--example` knob.
The Progress CLI keeps `--example` exactly where it was, and the
mutual-exclusion groups (`html`/`md`/`md_plus`) are independent of
`--example` — so `--example --md` correctly renders the example values
through the Markdown path. The lesson: `--example` is per-command UX
and stays on every subcommand even when other cross-target flags are
added.
