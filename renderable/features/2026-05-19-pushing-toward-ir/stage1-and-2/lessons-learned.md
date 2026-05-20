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

## Progress polish: infallible-trait fallbacks must log, not emit sentinels

`Progress`'s tree-routed render paths originally split error-fallback policy
across targets: the Terminal path silently emitted an empty string while the
Browser path embedded an in-band `[render-tree error: …]` text fragment. Both
trait contracts (`TerminalRenderable::render`, `BrowserRenderable::render_html_fragment`)
are infallible, so the right response to a tree-render failure is the same on
both targets — log the error via `tracing::error!` (with `component`, `error`,
and the relevant dialect) and return an empty output. The Markdown path's
`.unwrap_or_default()` had the right shape but no diagnostic; replacing it with
an explicit `match` arm and an `error!` call closes the observability gap
without changing the public contract. The lesson: when a trait is infallible
but the implementation delegates to a fallible renderer, the fallback policy
must be uniform across targets (empty output + structured log), never an
in-band sentinel that pollutes user-visible output.

## Progress polish: `bracket_color` has no semantic CSS slot — preserve as `data-*`

The Progress widget's filled and empty colors lower to inline
`background-color` declarations on the `progress-filled` and `progress-track`
spans, but the bracket glyphs sit inside the percentage text node and have no
dedicated element to paint. The spec sanctioned either dropping `bracket_color`
or preserving it. Preserving it as a `data-bracket-color="#rrggbb"` attribute
on the outer `progress` span (only when the slot is set) mirrors how
non-default bracket *glyphs* are already preserved as `data-left-bracket` /
`data-right-bracket`, keeps the lossless-when-possible posture of the
MarkdownPlus/Browser shared HTML, and lets consumers repaint brackets in
post-processing without inventing a synthetic CSS class. The lesson: when a
component slot has no semantic element to paint, preserve it as a `data-*`
attribute alongside the other lossless hints — don't drop it, and don't invent
a sham element to paint it on.

## Section flip: heading-only sections are byte-identical across the flip

`Section`'s bespoke `render_content` already lowered the heading through the
shared `apply_style` helper (the same helper the tree renderer's
`render_heading_line` uses), so for an empty section the bespoke and tree
paths produce **byte-identical** output (`\x1b[1m# Title\x1b[0m` for h1). The
divergences only appear once body content is added, and they reduce to the
documented blank-line difference between heading and body (`\n` for bespoke,
`\n\n` for tree). Section's `render_bespoke` parity test therefore lands as
exact-bytes for empty sections and token-list equality for sections with
content — the same split the broader `KNOWN_DRIFT` ledger documented before
the flip retired those entries.

## Section flip: with_layout is a default trait method, not a missing builder

`Section` exposes no `with_layout` builder of its own, but layout-matrix call
sites such as `section.with_layout(s.layout.clone())` already compile. That
is because `with_layout` is a default method on the `TerminalRenderable`
trait that operates through `layout_mut()`, so any component implementing
`layout_mut` inherits the builder for free. The migration pattern is to use
the trait default unless a component needs to wrap a non-`Layout` value
during construction, in which case a bespoke builder shadows the trait
default — but Section, like the other flipped components, has no such need.

## Section polish: parity coverage must include Prose and nested-component children, not just strings

A `Section` parity suite that only exercises `push("string")` content cannot
catch the most consequential drift mode for the IR flip — the
`project_renderable_content` projection branch. With strings only, both the
bespoke and tree paths funnel through `RenderableTerminalContent::String`
and never touch the inline-vs-flat-text decision that Prose triggers or
the block-structure-preservation decision that a nested block component
triggers. The spec lists "Section with Prose content" and "Section with
nested Component content" as critical variants for exactly this reason:
without them, a future regression that re-introduces ANSI-strip-and-flatten
inside the projection (the same trap the BlockQuote and OrderedList
migrations called out separately) would land green.

The migration pattern is to pin **two structural assertions** per polish
pass:

1. A Prose child must project to **at least one non-`Text` inline node**
   (`Strong`/`Emphasis`/`Span` — assert "not flat text" rather than
   pinning a specific kind, so the test survives a future canonical
   choice between `Strong` and `Span` for pure bold), and the terminal
   render must contain a bold-open SGR (`\x1b[1m`) so the inline lowering
   actually fires.
2. A nested block component (e.g. another `Section`) must appear in the
   parent's projected `children` as **a structured node of its own kind**
   (here, `NodeKind::Section { depth, heading, .. }` with the nested
   depth preserved), and all three targets (Terminal, Markdown, Browser)
   must render both headings with their respective prefixes/tags.

Together these two tests close the loophole that string-only parity coverage
leaves open, and they cost roughly fifty lines per component — well below
the cost of debugging the regression they catch.

## Section polish: shared `LayoutArgs` masks per-command vertical-margin asymmetry

`bt section --html --margin-top 2` silently ignores the top margin because
the cross-target branches in `SectionArgs::run` `return Ok(())` before
`emit_vertical_margins` runs — terminal output is the only branch that
threads the layout closure through that helper. The behavior is correct
(HTML margins are emitted as CSS on the `<section>` itself via the tree
path's `layout_to_css`, and Markdown has no portable vertical-margin
primitive), but it is invisible at the flag's docstring because
`LayoutArgs` is a *shared* struct used by ~12 subcommands.

The polish-pass fix is to document the asymmetry in the **consuming
command's** docstring rather than mutating `LayoutArgs` itself. Touching the
shared struct would either (a) duplicate the note across every command
that flatten-imports `LayoutArgs` or (b) push misleading semantics onto
commands that *do* honor vertical margins on every target. A
`#[command(flatten)] pub layout: LayoutArgs` field already has its own
docstring slot in clap — that is the right place for "this command only
honors `--margin-top`/`--margin-bottom` on the terminal target," and it
matches the surgical-change discipline the Compose polish lesson called
out for the same struct.

## StatusBlock flip: portable icon ≠ Nerd Font icon ≠ existing component constant

`StatusBlock` projects each `StatusState` to a stable portable Unicode glyph
at tree-projection time so Markdown and Browser output remain identical
across environments. The trap is that "portable Unicode glyph" is *not* the
same as the existing `Status` component's `FB_*` constants: the
StatusBlock spec specifies `⤫` (U+292B, RISING DIAGONAL CROSSING FALLING
DIAGONAL) for `Error`, while `components::status::FB_FAILURE` uses
`⨫` (U+2A2B, MINUS WITH FALLING DOTS). Both render as a small "x" in many
fonts but are different code points — a bespoke `render_bespoke` path that
goes through `Status::from_prose(...).state(...)` will emit `⨫` on a
non-Nerd terminal while the canonical tree path emits `⤫`.

The migration pattern is to **own the canonical icon in the projecting
component**, not to thread the existing `Status::FB_*` constants into the
projection. Parity tests then compare tokens with the leading icon glyph
*dropped* — a `drop_icons` helper that strips a leading non-ASCII char per
token is enough — so bespoke/tree parity can hold over visible content
without forcing one path to adopt the other's icon choice. The spec's icon
table is the contract; the existing `Status` constants are an unrelated
bespoke detail.

## StatusBlock flip: composite component projection vs bespoke wrap

`StatusBlock` is a composite of three sub-renderables (`Status` header,
`BlockQuote` body, `Prose` hint). The bespoke renderer composes those
component instances and joins their output with `\n`. Reusing that same
composition inside the tree projection (e.g. `Status::render_tree_node()`
+ `BlockQuote::render_tree_node()` + `Prose::to_render_nodes()`) initially
seemed appealing because each sub-component already has a tree path — but
this composes *block* trees inside the parent `Root`, and each child block
brings its own layout/style/word-wrap defaults that conflict with
StatusBlock's intended layout (right margin 5, `WrapProse(8, None)`).

The cleaner shape is to project StatusBlock directly into `Root` →
`Paragraph` / `BlockQuote` / `Paragraph` children, with severity color
mapped via `Style.color` on the header and `Style.border` on the body,
*without* delegating to sub-component tree projections. Header and hint
text flatten through a dedicated `prose_plain_text` helper that uses a
deliberately uncolored, non-Nerd terminal so bracketed markup like
`<b>Bold</b>` collapses to `Bold` rather than leaking into Markdown or
Browser output. This keeps StatusBlock's layout authoritative on the
projected `Root` and avoids the impedance mismatch between sub-component
defaults and the composite's intent.

## Table flip: cursor-positioning unit tests must point at `render_bespoke`

`Table`'s `prefer_cursor_alignment` mode is a terminal-specific compatibility
knob that the canonical render tree does **not** lower today — the tree
renderer's `render_table` has no awareness of the `TableTerminalHints
.prefer_cursor_alignment` flag, even though the projection carries it. Seven
in-source unit tests pinned byte-level `\x1b[NG` (column-move) escapes by
calling `Table::render_optimistic(Some(80))`. After the flip those tests
silently lost their cursor escapes because `render_optimistic` now routes
through the tree path, which falls back to space-based padding.

The migration pattern is to redirect *bespoke-only* byte-level tests at the
preserved `Table::render_bespoke(&Terminal::new_optimistic(80))` entry point
rather than relax the assertions. The bespoke `render_bespoke` still keys on
`term.is_tty` for cursor positioning, and `Terminal::new_optimistic` sets
`is_tty: true`, so the cursor path stays exercised. The block-level test
file gets a one-paragraph comment explaining that those specific tests
document bespoke behavior the tree renderer has not yet learned, and the
flip is correct as-is — the tests are the artifact that needed adjusting,
not the implementation.

This is the same shape as OrderedList's "switching the default `render()`
reshapes unit-test expectations" lesson, but it is sharper for Table
because `prefer_cursor_alignment` is a discrete tree-renderer feature gap
rather than a cosmetic byte-level divergence; the tree renderer cannot
produce these escapes at all today.

## Table flip: layout-matrix `top_margin_2` snapshot was lying before the flip

Identical to the Progress lesson: the `Table__top_margin_2` and
`Table__bottom_margin_2` `layout_matrix` snapshots were committed with the
BESPOKE half showing no top-/bottom-margin newlines and the TREE half
showing the correct margin. After the flip both halves match the tree
output (the bespoke renderer never honored vertical margins on a Table),
so the snapshot needed `INSTA_UPDATE=always` regeneration and the four
matching `KNOWN_DRIFT` entries per axis retired with a comment in
`render_comparison.rs` recording the retirement reason. The migration
playbook should treat a `BespokeBehind` ledger entry on a vertical-margin
facet as evidence that the flip will close a user-visible behavior gap,
not as a parity regression.

## StatusBlock flip: arbitrary border survives at the TerminalRenderable boundary

The `StatusBlock::border(String)` compatibility knob is documented as
terminal-only and explicitly excluded from the canonical tree. The
implementation matches BlockQuote's pattern exactly: a `has_default_border()`
predicate gates `render(&term)` between `render_via_tree` (default path) and
`render_bespoke` (compatibility fallback). The two paths share no code below
that gate — `render_bespoke` composes `Status` + `BlockQuote` instances
verbatim, while `render_via_tree` builds the canonical projection — and the
`BrowserRenderable` / `MarkdownRenderable` impls *always* use the tree path,
so a custom prefix can never leak into a non-terminal target.

A regression test that asserts `!html.contains("!! ")` and
`!md.contains("!! ")` for a `with_border("!! ")` block makes this gate
explicit and prevents a future "make the bespoke path the canonical source"
refactor from accidentally pulling the prefix into structural output.

## Table flip: cursor-positioning is a TTY-gated escape hatch, not a tree hint

The `prefer_cursor_alignment` builder on `Table` is documented as a terminal
escape hatch (`CSI N G` column-move bytes) used by 30+ production CLI call
sites (`claudine`, `sniff`, `model-citizen`, `messenger`, …). The
canonical render-tree does *not* yet lower this hint, so a naive flip of
`TerminalRenderable::render` to `render_via_tree` silently degraded every
opted-in consumer to space-padded columns without any test catching it.

The correct gate is **two-condition, not one**: `prefer_cursor_alignment &&
term.is_tty`. The `is_tty` half matters because the bespoke path itself
already gates internally — emitting cursor-move bytes into a pipe or file
capture corrupts the captured output. Tests must pin all three quadrants
explicitly so a future refactor cannot silently re-introduce the
regression:

- TTY + opt-in → bespoke path, `CSI N G` bytes appear
- TTY + opt-out (or non-TTY + opt-in) → tree path, no `CSI N G` bytes
- non-TTY + opt-in → tree path (the bespoke guard wins, not the caller)

The component-spec migration playbook should treat any documented
"terminal-only escape hatch" attribute (`prefer_cursor_alignment`,
`StatusBlock::border`, …) as requiring an explicit gate in `render()` even
after the tree flip, until the tree renderer lowers the corresponding
terminal hint.

## Table flip: SoftBreak in a cell is a SPACE, not a `<br>`

Reviewers naturally expect "all four break sources" (literal `|`, literal
`\n`, `SoftBreak`, `HardBreak`) to map identically inside a Markdown table
cell. The actual implementation in
`renderable/src/tree/render/markdown.rs` is asymmetric and intentional:

- literal `\n` between adjacent `Text` nodes → `<br>` (cell-text escape)
- `SoftBreak` node inside a cell → single space (not `<br>`)
- `HardBreak` node inside a cell → `<br>`
- literal `|` anywhere → `\|`

The space-mapping for SoftBreak is what makes GFM tables look right when
source Markdown wraps cell prose across lines — collapsing to a space
preserves the prose flow. A regression test that hand-builds a
`RenderNode::table_cell` with all four siblings is the only way to pin
this divergence; the public `TableCellContent::Text` API only exposes
literal characters, so the SoftBreak/HardBreak path is otherwise
unreachable from a tree built by `Table::to_render_tree_node`.

## Table flip: SGR bytes in cell content survive as backslash-escaped text

Cells containing pre-styled ANSI SGR sequences (e.g.
`"\x1b[32mActive\x1b[0m"`) round-trip through the *terminal* target with
the `\x1b` byte rendered as the literal text `\[…` rather than passing
through live. This is a deliberate consequence of treating cell content
as data, not control: it keeps the box-drawing borders uncorrupted when
a typed cell is mis-classified, at the cost of losing live color in cells
the caller specifically pre-styled.

Pin the contract as "visible text reaches the user; the color token
appears (live or escaped)" rather than "live SGR survives" — the latter
would force the renderer to allow arbitrary cell content to escape the
table structure.

## TextBlock flip: the bespoke italic SGR open is malformed (`\x1b[3` not `\x1b[3m`)

`utils::styling::Stylist::wrap` emits its open as `ESC + set_char + content
+ ESC + reset_char + TERMINAL`, which for `Style::Italic` (whose `set_char`
returns `"3"`) yields `\x1b[3<content>\x1b[23m` — the trailing `m` on the
open SGR is missing. Most terminals are still permissive enough to apply
italic, but a byte-equality check for `\x1b[3m` against the bespoke output
fails. The tree path emits the canonical `\x1b[3m...\x1b[23m`.

The migration pattern for any italic-touching parity test is to grep for the
SGR open *prefix* `\x1b[3` against the bespoke side and the canonical
`\x1b[3m` against the tree side. Treat the divergence as accepted
`KNOWN_DRIFT`: fixing the bespoke `wrap` would touch a shared helper that
many other components still depend on, and the bespoke path is `#[doc(hidden)]
pub fn render_bespoke` after the flip anyway. A future cleanup that
retires bespoke renderers entirely retires the bug along with the helper.

## TextBlock flip: block-level Style emphasis lowers to CSS, not semantic wrappers

The RT-TEXTBLOCK-001 spec asks for emphasis lowering "to semantic wrappers
(`<strong>`, `<em>`, `<s>`) **or equivalent valid HTML** that preserves
nesting." The browser tree renderer takes that latitude and splits behavior
by element category: an **inline** node lowers emphasis to semantic wrappers
(`<strong>`/`<em>`/`<s>`), while a **block** node lowers the same emphasis
to inline CSS on the element itself (`font-weight:bold`, `font-style:italic`,
`text-decoration-line:line-through`). `TextBlock` projects to a block
`Paragraph`, so its `<p>` element carries CSS — `<p style="font-weight:bold">Hello</p>`
rather than `<p><strong>Hello</strong></p>`.

Both shapes are spec-conformant; the parity test must accept either. The
clean assertion shape is "bold semantics preserved" — `html.contains("<strong")
|| html.contains("font-weight:bold")` — rather than pinning a specific
wrapper choice. A future renderer change that flips block emphasis to
semantic wrappers should not require updating every block-component parity
test in lockstep.

## TextBlock flip: `Color` is `Copy` — drop the `.as_ref().clone()` boilerplate

`renderable::color::Color` derives `Copy`, so the projection's
`Option<Color>` -> `Option<TargetValue<PerMode<Color>>>` lowering can be
`self.fg_color.map(|c| TargetValue::universal(PerMode::universal(c)))`
without an intermediate `.as_ref()` and without `.clone()`. The first cut of
the TextBlock migration mirrored the `.as_ref().clone()` shape that other
migrations use for non-`Copy` `Style` / `Layout` values, and clippy flagged
three `clone_on_copy` warnings. The shape that compiles AND satisfies clippy
for a `Copy` payload is the bare `Option::map` chain.

## TextBlock flip: `UnderliningRequest` carries optional underline color with no Style slot

The pre-IR `TextBlock` exposed underline color via `UnderliningRequest::*(Some(color))`,
but the canonical `Style::emphasis.underline` is `Option<UnderlineStyle>` —
shape only, no color slot. The TextBlock spec sanctions dropping the
underline color at the projection boundary rather than smuggling it through
a `data-underline-color` hint, because (a) the bespoke renderer never
emitted underline at all, so there is nothing to be parity-with, and (b) a
colored-underline feature should be designed renderer-wide rather than
preserved as a `TextBlock`-only escape hatch.

The migration pattern for any future component that exposes a component-only
sub-slot with no `Style` peer is to drop the slot at the projection
boundary, document the drop, and pin a structural test
(`underline_color_is_dropped_from_projection`) that the projection records
shape only. The drop is intentional and should not be papered over.

## TextBlock review: `--example` injection must respect mutually-exclusive sibling flags

The TextBlock CLI uses an `--example` flag that injects a representative
style set (`bold = true`, `--fg green`, etc.) so the rendered output matches
the documented command. The first cut wrote `effective.bold = true`
unconditionally, which created an in-memory state with both `bold = true`
and `dim = true` whenever the user combined `--example --dim`. clap's
`conflicts_with = "bold"` on `--dim` only fires when both flags come from
the command line; the implicit `--example` injection bypasses that gate
entirely. The fix is to guard the injection with `if !self.dim` so `--dim`
cleanly wins.

The general pattern: any flag that injects values into clap state outside
the normal parse path must mirror the `conflicts_with` graph manually for
every pair it can disturb. Pin a test (`test_text_block_example_dim_wins_over_implicit_bold`)
that checks the resulting render reflects the user's flag, not the
injection's default. When asserting on `--example` output, split off the
`print_example_command` trailer before searching for SGR — the trailer is
intentionally styled bold and would mask any `\x1b[1m` assertion on the
body.

## TextBlock review: underline-color drop test should be parameterised over the enum

The TextBlock projection drops underline color (`UnderliningRequest::*(Some(c))`)
because `Style::emphasis.underline` is shape-only. The first cut only pinned
the drop on `Curly`, which left four other variants unprotected against a
future regression that smuggled underline color through a data hint. The
fix is to parameterise the test over all five variants (`Straight`,
`Double`, `Curly`, `Dotted`, `Dashed`) and assert (a) the projected
`UnderlineStyle` matches the variant shape and (b) no `underline-color` /
`underline_color` key appears in the serialised `NodeAttrs`. The
serialise-then-grep check is a cheap structural assertion that any future
"smuggle the color back in" attempt — whether as a `data-*` hint or an
ad-hoc attrs field — will trip immediately.

## TextBlock review: `render_optimistic` activates stored fields on the call-path

A subtle implication of the TextBlock IR flip that's easy to miss: `render_optimistic`
now routes through `render_via_tree`, which means every `Style` field the
component stored — foreground color, background color, underline,
strikethrough, blink — now emits SGR on the optimistic path too, even when
the caller does not supply a real `Terminal`. Callers that previously held
a `TextBlock` with `with_foreground_color(...)` set and called
`render_optimistic(Some(80))` got plain text out (the bespoke path stored
the color but never rendered it). The same call now emits a 30/90-series
or TrueColor SGR.

This is the intentional public-behavior fix — the dormant stored fields
were always meant to render — but it's worth surfacing prominently because
unit tests that asserted on stripped/plain output of `render_optimistic`
may have silently relied on the inertness. The migration pin is
`fg_color_via_tree_is_now_active` in `components/text_block.rs`'s unit
tests and `fg_basic_color_activates_through_tree` in `text_block_parity.rs`.
Any caller surprised by new ANSI in `render_optimistic` output is the
expected outcome of the flip, not a regression.

## Todo flip: list-rendering path bypasses ListItem Style

The Todo spec called for setting `Style` on the projected `ListItem` so the
terminal renderer would lower dim + strikethrough SGR for `Cancelled`. The
test discovery was that `render_list_item` directly dispatches the item's
children through the inline pipeline — it never calls `self.render(node)`
on the `ListItem`, so the `Style` attribute attached there is silently
ignored on terminal output. The `style` attribute *is* read by the browser
renderer (it lowers to `opacity:0.6;text-decoration-line:line-through` on
the `<li>` element), so the attribute is not dead weight, just
target-specific.

The migration pattern is to keep the `Style` on the `ListItem` for browser
CSS lowering AND carry the same emphasis on an inline `Span` inside the
item's `Paragraph`. The `Span` flows through `render_inline_node`, which
*does* honor the declared style, so dim + strikethrough finally surface as
SGR on the terminal target. For Cancelled the structural shape is:

    Paragraph → Span (Style: dim+strikethrough)
              → Delete
              → Text

The `Delete` is what serializes to GFM `~~text~~` in Markdown and `<s>` in
the browser; the `Span` Style is the terminal-only carrier for dim. None
of the layers fight: Markdown ignores `Style`; browser CSS dedupes
line-through; terminal sees dim from the Span and strikethrough from the
Delete-rendered Prose tag.

## Todo flip: layout belongs on the List, not the ListItem

A natural reading of the spec ("seed Layout onto the ListItem because the
item is the visible block") fails in practice: the list-rendering path
constructs its own prefix-and-indent geometry for each item and never
consults the item's own `Layout`. The component's `Layout` only takes
effect when it sits on a node whose `render()` call passes through
`render_with_layout` — for a one-item list, that node is the top-level
`List` itself.

The migration pattern is to set Layout on the `List` node (the component's
visible block) when the component's `Layout` is non-default. This is the
same pattern Section uses for its top-level container and matches every
other component whose layout is honored on the terminal target. The
parity test pin is `non_default_layout_is_recorded_on_list` /
`layout_left_margin_applies_through_tree`.

## Todo flip: browser Delete lowers to `<s>`, not `<del>`

The Todo spec listed `<del>` as the browser-side strikethrough wrapper for
Cancelled items, but the browser tree renderer lowers `NodeKind::Delete`
to a `<s>` element (`self.block(BlockTag::S, ...)`), not `<del>`. The two
elements carry the same semantic ("content is no longer accurate") and
both pick up `text-decoration: line-through` styling by default, so the
difference is purely a question of which tag the renderer happens to
emit. Tests that pinned `<del>` are wrong; they must assert on `<s>` (and
optionally on the `<li>` carrying `text-decoration-line:line-through`
CSS from the Style on the `ListItem`).

## Todo flip: NoColor terminals still see strikethrough SGR via Delete

The bespoke `Todo` renderer stripped strikethrough for Cancelled items
when the terminal advertised `ColorDepth::None`. The tree path does not:
the projected `Delete` node always lowers to a `<strikethrough>` Prose
tag inside `render_inline_node`, and the Prose pipeline emits
`\x1b[9m`/`\x1b[29m` regardless of color depth. The behavior is
documented as a KNOWN_DRIFT for Todo cancelled items only — every other
state preserves the bespoke "no ANSI in NoColor" contract because their
projections carry no inline strikethrough/dim markup.

The parity test pin is
`no_color_no_nerd_non_cancelled_emits_no_ansi_escapes`, which loops over
the four non-Cancelled states and asserts the output contains no `\x1b`,
while `no_color_no_nerd_cancelled_state_uses_ascii_fallback` only asserts
that the ASCII marker `[-]` and the description text are present
post-strip. The drift is acceptable because the strikethrough SGR pair
(`\x1b[9m...\x1b[29m`) is a visual no-op on terminals that lack ANSI
support at all, and a graceful fallback on terminals that have ANSI but
no color.

## TwoColumn flip: image-overlay fallback is not a tree-render error path

`Progress`, `Table`, and friends fall back to `render_bespoke()` when
`render_terminal_node()` returns `Err`. For `TwoColumn` the
terminal-image overlay scenario does not surface as an error — the
projection itself returns a structurally-valid `NodeKind::Unsupported`
node. `render_terminal_node` happily folds an `Unsupported` node into a
diagnostic placeholder, so the tree path returns `Ok`, the `match` on the
result never enters the fallback arm, and the user sees the placeholder
instead of the image overlay.

The fix is a kind check before the tree call:

```rust
let node = self.to_render_node();
if matches!(node.kind, NodeKind::Unsupported { .. }) {
    return self.render_bespoke(term);
}
let opts = TerminalRenderOptions::new(term, RenderStrictness::Warn);
match render_terminal_node(&node, &opts) { ... }
```

`Err`-only fallback would have produced a regression visible only when
an image column is present; the parity tests
`render_with_image_column_uses_bespoke_path_not_unsupported_placeholder`
and `render_with_image_column_in_right_uses_bespoke_path` pin both
sides of the flip.

## TwoColumn flip: layout-matrix snapshot lies were the most numerous yet

The `Progress` and `Table` flips each retired a single
`top_margin_2` snapshot whose BESPOKE half was lying — the pre-flip
bespoke renderer was dropping the top margin entirely, so the snapshot
recorded "BESPOKE: no margin / TREE: margin". `TwoColumn` retires
*three* such snapshots (`top_margin_2`, `bottom_margin_2`, and
`max_width_40`), plus twelve `KNOWN_DRIFT` ledger entries on the
`render_matches_bespoke` matrix. The migration pattern stays the same:
accept the new snapshot, drop the ledger entries, and prefer asserting
parity at `render_bespoke` vs. `render(&term)` rather than trusting the
pre-flip side-by-side harness.

The deeper lesson: the layout-matrix harness records the bespoke path's
*shape*, not its *correctness*. Every flip surfaces fresh "lies" because
the bespoke shape never matched the layout contract; only the tree path
does. Once the final component is flipped, the harness's side-by-side
view becomes informational across all components.

## Closing observation: a tree, not twelve components

Twelve components were migrated to the canonical render tree under this
feature: `BlockQuote`, `Compose`, `FileSystem`, `OrderedList`,
`UnorderedList`, `Progress`, `Section`, `StatusBlock`, `Table`,
`TextBlock`, `Todo`, and now `TwoColumn`. The pattern that emerged is
almost mechanical, but the leverage it gives is not. Each flip swapped
one bespoke renderer for one shared lowering, and each one tightened
the contract that every other renderer now plays against.

By the time `TwoColumn` landed, the renderable surface looked very
different from the per-target traits it started with: the render tree
is no longer "another target," it is the *origin* the targets fold
from. Bespoke renderers became fallbacks. Compatibility hooks became
delegates. Snapshots that recorded bespoke shape were retired in
favour of bespoke-vs-tree parity at `render_bespoke`. The
`KNOWN_DRIFT` ledger went from a real audit surface to a closed book —
each entry was either retired with a flip comment or actively pinned a
documented loss (Markdown list markers, Markdown table HTML escapes,
Markdown progress widget loss, the Todo Cancelled strikethrough SGR).

Three larger truths surfaced repeatedly:

1. **Stored fields hide dormant behavior.** `TextBlock` stored an
   `italic` field that no terminal path ever rendered; `Todo` stored
   `Layout` on each item but the bespoke renderer only honored the
   outer one; `TwoColumn` stored `gap` and `left_width` that Browser
   simply dropped. Every flip is also a chance to walk the struct and
   ask "what isn't this field doing today?"
2. **Recognition is not preservation.** Every renderer recognised
   `ColumnsHints` long before the canonical projection adopted them.
   Recognition gave a working pixel; preservation kept the contract.
   The same gap appeared for Browser-side `BlockQuote` border color,
   Markdown table escaping, and `Progress` `bracket_color`. The
   reverse — preserving without recognising — produced silent loss
   diagnostics that only fire under `Strict`.
3. **The bespoke path is the past, not the truth.** Twelve flips in,
   every snapshot that compared "BESPOKE vs TREE" turned out to
   record bespoke quirks the tree path was correct to abandon: top
   margins that were silently dropped, max-widths that were ignored,
   line wraps that were the wrong policy. The harness is useful as a
   *change indicator*, not as an *oracle*. After every flip, expect
   to update the snapshot; before every flip, expect to find the lie.

The tree was always there. The migration was just the long process of
asking each component to admit it.

## Stage 2 complete: all twelve components flipped

The TwoColumn flip closes Stage 2 of the IR push. With it, every one of
`BlockQuote`, `Compose`, `FileSystem`, `OrderedList`, `Progress`,
`Section`, `StatusBlock`, `Table`, `TextBlock`, `Todo`, `TwoColumn`, and
`UnorderedList` now reaches the terminal, browser, and markdown targets
through the canonical render tree. What started as twelve independent
migrations converged on a single, repeatable recipe.

### The recipe that emerged

Each flip — by the time it landed — looked the same:

1. **One private projection helper.** Every component now has a private
   `fn build_<x>_node(&self) -> RenderNode` (often `build_tree_node`,
   `render_columns_tree`, etc.) that owns the entire `Component → tree`
   lowering. Both `TreeRenderable::render_tree()` *and* the legacy
   `TerminalRenderable::render_tree_node()` delegate to it. There is
   exactly one source of truth per component, and parity test 25 pins
   them as serialization-equal.
2. **`render_via_tree` with `tracing::error!` + bespoke fallback.** The
   public `render(&term)` is rewritten to call the projection helper,
   fold the resulting node through `render_terminal_node`, and *log*
   any failure via `tracing::error!` before falling back to the bespoke
   path (or an empty string for components that have no bespoke
   fallback). Silent fallbacks would have masked regressions; logging
   makes them visible without breaking the infallible `render` contract.
3. **`#[doc(hidden)] pub fn render_bespoke()` retained.** The old
   renderer is preserved as a doc-hidden public method so parity tests
   can compare the two paths and so escape-hatch scenarios (notably
   `TwoColumn`'s cursor-overlay image rendering) can opt out of the
   tree. The visibility is `pub` for the test crate boundary but
   `#[doc(hidden)]` so the public docs surface only `render`.
4. **Direct `BrowserRenderable` and `MarkdownRenderable` impls.** Each
   component now implements both trait targets directly on the
   component type. Browser routes through `BrowserTreeComponent::new`;
   Markdown routes through `MarkdownTreeComponent::new` (or the
   equivalent fold). No component re-derives lowering rules — they
   ride the tree renderers' shared CSS lowering, GFM emission, and
   strictness gates.
5. **`bt <x> --md / --md-plus / --html / --example` with
   `conflicts_with_all`.** Every flipped component's CLI command grows
   the three target flags, each declared with `conflicts_with_all =
   [...]` so clap rejects mixed targets at parse time, and an
   `--example` flag that composes with each. `bt columns --example
   --md-plus`, `bt table --html`, `bt todo --md`, etc., all behave
   identically.
6. **Dedicated `*_parity.rs` test file using `render_bespoke()`.**
   Each component owns a dedicated `lib/tests/<x>_parity.rs` that
   compares `strip_ansi(component.render_bespoke(&term))` against
   `strip_ansi(component.render(&term))` across the spec's full test
   variant list (typically variants 1–12, 15–17). The harness's
   side-by-side view is now informational; the parity file is the
   oracle.

### Notable cross-cutting refactors

- **Shared `project_renderable_content` helper.** Multiple components
  needed to project a `RenderableTerminalContent` (which may be a
  string or an arbitrary nested component) into either inline-only
  spans or structural block children. That logic was lifted into a
  shared helper with a `ProjectionMode::{InlineOnly, Structural {
  terminal_hint }}` discriminant so each call site picks the
  appropriate flattening policy without duplicating the inline-run /
  paragraph-wrap dance.
- **`ColumnsHints` block-carrier pattern.** `TwoColumn` resisted a
  dedicated `NodeKind` and instead rides a `BlockQuote` carrier with
  typed `ColumnsHints` plus a `left_count` split index. The three
  tree renderers special-case the hint so the quote border never
  renders. This pattern — *attach typed hints to a generic carrier
  rather than minting a new kind* — also turned out to fit the
  `HorizontalRule` width/weight knobs and is a strong candidate for
  future composite layouts.

### Notable behavioral changes

- **Activated dormant `TextBlock` style fields.** `TextBlock` stored
  italic/style fields that no terminal path ever consulted. The flip
  surfaced them as `Style::TextEmphasis` on the projected node, and
  the terminal renderer now respects them. This was a behavioral
  *gain*, not a regression — the bespoke path had been dropping the
  field on the floor for years.
- **Cursor-positioning TTY gate.** Several flips revealed bespoke
  renderers that emitted cursor save/restore sequences regardless of
  whether stdout was a TTY. The tree renderer gates cursor-positioning
  output on `Terminal::is_tty`, so piping `bt columns ...` to a file
  no longer leaks `\x1b[s` / `\x1b[u` into the captured bytes.
- **Layout-matrix snapshots regenerated for 5+ components.**
  `Progress`, `Table`, `Section`, `Todo`, and `TwoColumn` each retired
  snapshots whose BESPOKE half had been recording a layout violation
  (silently dropped top/bottom margins, ignored `max_width`, wrong
  word-wrap policy). The new snapshots record both halves agreeing
  through the tree path. The `KNOWN_DRIFT` ledger went from a real
  audit surface to a closed book.

### Nested non-`Prose` block components flatten to text

The Stage 2 closeout uncovered one accepted limitation worth recording
explicitly. When a container component (`TwoColumn`, `BlockQuote`,
`Section`, …) holds a `RenderableTerminalContent::Component(c)` whose
`c` is *not* a `Prose`, the projection path
(`RenderableTerminalContent::to_tree_nodes` → `Component(c)` arm) calls
`c.render_tree_node()`. That method defaults to `None`, so unless the
inner component overrides it, the projector falls back — under
`RenderStrictness::Warn` (the public default) — to rendering `c` to a
plain terminal string, stripping ANSI, and wrapping the result in a
`Text` node.

The visible effect is that a `BlockQuote` nested inside a `TwoColumn`
column projects to a `Paragraph { Text("│ quoted inside") }` rather
than a structural `BlockQuote` child of the columns carrier. The text
content (and its styling, once rendered) survives end-to-end, but the
tree loses the `BlockQuote` *kind*. This matters only for downstream
consumers that walk the tree structurally — every current tree
renderer treats the flattened paragraph correctly.

`Prose` escapes this trap because the `Component` arm of
`to_tree_nodes` checks for the `Prose` type and projects its rich
inline children directly, so styled emphasis/strong/spans survive
through the tree.

The Stage 2 parity test
`render_via_tree_preserves_nested_block_content` (in
`biscuit-terminal/lib/tests/two_column_parity.rs`) pins the *current*
contract: nested block-component text reaches the column carrier's
subtree. A future Stage 3 task is to teach `project_renderable_content`
to downcast `BlockQuote` and `Section` the same way it downcasts
`Prose` so they project to block render-nodes rather than flattened
text — at which point the test can be tightened to assert the
structural `BlockQuote { … }` child.

### Is the recipe ready for extraction?

Yes — with one caveat. The six recipe points above can be turned into
a documented "IR migration checklist" today; every Stage 2 flip
followed them and no flip needed a new step. The caveat: components
with no bespoke fallback (e.g. a future component that exists *only*
as a tree projection) skip points 3 and partially point 2. The
checklist should document both shapes: "flip-from-bespoke" (the
twelve we just shipped) and "born-on-the-tree" (the path forward).

A useful next deliverable is a `migrate-component-to-ir.md` checklist
under `renderable/docs/` that codifies these six points plus the
escape-hatch knobs (`render_bespoke`, `Unsupported` node, strictness
behavior) so the next contributor does not have to re-derive the
pattern from twelve worked examples.

## Stage 3, Task 3a.2 — render_tree_node parity audit (2026-05-20)

Audited the nine pre-existing `TerminalRenderable::render_tree_node`
overrides for explicit parity coverage against
`TreeRenderable::render_tree`. Eight already had a structural-equality
assertion; only `Compose` lacked dedicated coverage — fixed in this
task by adding `biscuit-terminal/lib/tests/compose_parity.rs` (empty,
text-only, and mixed-parts cases).

Per-component result:

| Component     | Override (src)                                         | Parity test                                                              | Result |
|---------------|--------------------------------------------------------|--------------------------------------------------------------------------|--------|
| Compose       | `components/compose.rs:157`                            | `tests/compose_parity.rs` (3 new tests)                                  | ADDED  |
| OrderedList   | `components/list.rs:935`                               | `tests/ordered_list_parity.rs:71` (`tree_renderable_and_compat_hook_…`)  | PASS   |
| Progress      | `components/progress.rs:371`                           | `tests/progress_parity.rs:361` (`tree_renderable_and_render_tree_node_…`)| PASS   |
| Section       | `components/section.rs:393` (→ private `to_render_node`)| `tests/section_parity.rs:221` (`tree_renderable_and_render_tree_node_…`) | PASS   |
| Table         | `components/table/table.rs:1677`                       | `tests/table_parity.rs` (shared-projection coverage via `render_tree_node` asserts) | PASS |
| TextBlock     | `components/text_block.rs:355` (→ private `to_render_node`)| `tests/text_block_parity.rs:175` (`tree_renderable_and_render_tree_node_…`)| PASS |
| Todo          | `components/todo.rs:549`                               | `tests/todo_parity.rs:243` (`tree_renderable_and_render_tree_node_share_projection`) | PASS |
| TwoColumn     | `components/two_column.rs:624`                         | `tests/two_column_parity.rs:337` (`tree_renderable_matches_terminal_render_tree_node`) | PASS |
| UnorderedList | `components/list.rs:377`                               | `tests/unordered_list_parity.rs:68` (`tree_renderable_and_compat_hook_…`)| PASS   |

Notes:

- `Section` and `TextBlock` both delegate through a private
  `to_render_node` helper called by both trait entry points; the
  parity tests still assert byte-equal `RenderNode`s, so structural
  drift in either path will trip the gate.
- `Compose`'s override is a one-liner delegate to
  `<Self as TreeRenderable>::render_tree(self)`; the added tests pin
  this invariant against future refactors that might insert
  post-processing in only one path.

## Stage 3, Task 3a.3 — `FileSystem::render` decision (2026-05-20)

### Decision: outcome (iii) — defer to Stage 4

The bespoke `FileSystem::render` path is kept. The flip to
`render_via_tree` is deferred to Stage 4 behind a single, named
acceptance criterion (see "Stage 4 acceptance criterion" below).

This is the same posture the FileSystem spec already documented as
the recommendation; Stage 3a.3 confirms it with parity evidence and
pins the present-day gap as `#[test]`s in
`biscuit-terminal/lib/tests/filesystem_parity.rs` so the decision is
observable rather than tribal.

### Why not outcome (i)

The canonical render-tree path cannot reproduce the bespoke output
today. Concretely, `TerminalRenderer::render_tree_connector_list`
(`biscuit-terminal/lib/src/render_tree/render.rs:905`) renders each
list-item paragraph with `self.render_inline(children,
&Style::default())` — discarding the per-item paragraph `Style`
attribute set by `FileSystem::fs_project_tree_node` (via
`fs_entry_style`). The projection correctly carries bold/blue, cyan,
italic, dim, and red intent on the paragraph; the connector-list
renderer simply does not lower it. This is provable from inspection,
and the new fixtures observe the symptom directly.

Bespoke also formats Unicode icons as `📂name` (no separator) while
the tree projection emits `📂 name` because the projection inserts a
literal `" "` text node between the icon span and the name span. Even
if the SGR gap were closed, the plain-text content would still differ
until either the projection drops the literal space or the bespoke
renderer adds one.

### Why not outcome (ii)

Outcome (ii) would mean "bespoke forever". The spec already approves
`ListMarkerPolicy::TreeConnectors` and the projection already targets
it; the only thing standing between the bespoke and tree paths is the
connector-list renderer (a generic renderer concern, not a
FileSystem-specific one) and the icon-spacing presentation choice.
Both are tractable in Stage 4.

### Fixture parity table

| Fixture | Bespoke | Tree | Parity | Notes |
|---|---|---|---|---|
| connector geometry | `├── 📄a.txt` / `└── 📄b.txt` | `├── 📄 a.txt` / `└── 📄 b.txt` | N | Tree inserts a literal space between icon span and name; connector glyphs themselves match. |
| gitignore styling | no dim SGR | no dim SGR | Y (today) | `is_ignored` is hardcoded `false` (Phase 8 placeholder); neither path exercises dim today. When Phase 8 lands, tree will fail until the Style lowering gap is closed. |
| errors and permissions | red SGR on error node (when sandbox honors `0o000`) | no SGR | N | When the scanner records `has_error: true`, bespoke emits `\x1b[31m`; tree drops it via the connector-list Style gap. Sandbox-dependent. |
| depth limit | only depth-1 entries visible | only depth-1 entries visible | Y | Both paths honor `max_depth`. |
| highlight precedence | red SGR on TODO-dir (bold suppressed) | no SGR | N | Same Style-lowering gap as dotfile/symlink. |
| metric annotations | `( file size: 42 B )` | `( file size: 42 B )` | Y | Metric pair text matches; per-pair SGR styling (dim label / bold-yellow value) is a follow-on assertion to add when the Style gap is closed. |
| dotfile italic | italic SGR (`\x1b[3m`) | no SGR | N | Canonical proof: projection sets `emphasis.italic = true`, the connector-list renderer ignores it. |
| symlink styling | cyan SGR (when scanner flags symlink) | no SGR | N | Scanner-and-renderer dependent; identical root cause as dotfile/highlight. |
| link behavior (OSC8) | `\x1b]8;;file://...` | `\x1b]8;;file://...` | Y | Both paths emit OSC8 hyperlinks with `file://` URLs when `with_file_links()` is set. |

Six of nine fixtures have parity (Y) today; three fail because the
tree path cannot lower per-item paragraph `Style`, and one (gitignore)
is latent — it will fail the same way once Phase 8 actually marks
entries as ignored. The icon-spacing divergence affects every line but
is a single source-edit away in either path.

### Stage 4 acceptance criterion

Flip `FileSystem::render` to `self.render_via_tree(term)` once **all**
of the following hold:

1. **Connector-list Style lowering.**
   `TerminalRenderer::render_tree_connector_list` lowers each
   `ListItem`'s child paragraph `Style` (and the paragraph children's
   own styles) into the inline rendering call, instead of always
   passing `&Style::default()`. The fix likely mirrors what
   `render_list_item` does for the default-marker path.

2. **Icon-name spacing reconciled.** Either:
   - the canonical projection drops the literal `" "` text node it
     inserts between the icon span and the name span (and the bespoke
     renderer continues to emit `📂name`), or
   - the bespoke renderer adds a single space between the Unicode icon
     and the name (and the projection keeps the explicit space).

   Pick one. Either choice is fine; the parity fixture will pin it.

3. **Parity fixtures flip from "GAP" to byte-for-byte equality** in
   `biscuit-terminal/lib/tests/filesystem_parity.rs`. The three
   currently-failing-by-design fixtures
   (`fixture_dotfile_italic_records_divergence`,
   `fixture_highlight_precedence_records_divergence`,
   `fixture_errors_and_permissions_records_divergence`) must invert
   from `assert!(!styled_in_tree, …)` to `assert!(styled_in_tree, …)`
   plus a stripped-ANSI equality check between bespoke and tree
   output.

4. **Layout matrix.** Add `FileSystem` to the default rows of the
   layout-matrix harness (Stage 3c work; tracked separately as Task
   3c.3).

Acceptance is satisfied when `cargo test -p biscuit-terminal --test
filesystem_parity` passes with every fixture asserting equality rather
than divergence, and `FileSystem::render`'s body is the same one-line
`self.render_via_tree(term)` as the other twelve flipped components.

### What did NOT move

- `FileSystem::render` still calls the bespoke renderer.
- `FileSystem::render_tree_node` continues to delegate to
  `TreeRenderable::render_tree` (Stage 3a.1 contract — required so
  cross-target adapters consume `FileSystem` structurally).
- No layout-matrix work was done here; Task 3c.3 will pick that up iff
  outcome (i) is reached in Stage 4.

### Evidence

- Fixtures: `biscuit-terminal/lib/tests/filesystem_parity.rs` (36
  tests, all passing; nine of them are §3a.3 decision-gate fixtures).
- Bespoke renderer:
  `biscuit-terminal/lib/src/components/filesystem/mod.rs:1610`.
- Projection:
  `biscuit-terminal/lib/src/components/filesystem/mod.rs:2745`.
- Connector-list renderer (gap source):
  `biscuit-terminal/lib/src/render_tree/render.rs:905`.

### Final action (Task 3c.3, 2026-05-20)

Per the §3a.3 outcome (iii) branch of the Stage 3 plan:

- **Kept bespoke.** `FileSystem::render` is unchanged and still calls
  the bespoke renderer at
  `biscuit-terminal/lib/src/components/filesystem/mod.rs:1610`.
- **No public `render_bespoke` hook added.** `FileSystem` deliberately
  does not expose a `pub fn render_bespoke`; the bespoke output is
  reachable only via `<FileSystem as TerminalRenderable>::render`. This
  matches the §3c.2 escape-hatch policy: only `StatusBlock`, `Table`,
  and `TwoColumn` retain sanctioned `render_bespoke` hooks.
- **Excluded from default layout-matrix parity.** The Task 3c layout
  matrix harness must omit `FileSystem` from its default
  `via_render == via_tree_direct` rows for the duration of Stage 3,
  because the tree path cannot reproduce the bespoke output until the
  Stage 4 acceptance criteria above are met. Task 3c picks this up.
- **Stage 4 flip is gated by the acceptance criteria above** (connector-
  list Style lowering, icon-name spacing reconciliation, parity-fixture
  flip, and re-adding `FileSystem` to the layout matrix). No new gates
  are introduced here.

## Stage 3 Task 3c.1 — retired bespoke hooks

- `OrderedList::render_bespoke` retired (no escape-hatch knob); `list.rs` private `render_content` helper removed alongside it.
- `UnorderedList::render_bespoke` retired; `list.rs` second `render_content` helper removed alongside it.
- `Progress::render_bespoke` retired; private `render_bar` helper removed (only caller was the bespoke path; `paint_fg` kept — still used by the tree renderer).
- `Section::render_bespoke` retired; private `render_content` helper and the in-source `render_bespoke_still_available_for_parity` test removed.
- `TextBlock::render_bespoke` retired; private `to_terminal` helper and the in-source `render_bespoke_still_available_for_parity` test removed; `styling::{Style, Stylist}` imports orphaned and dropped.
- `Todo::render_bespoke` retired; private `to_terminal` helper and six in-source `test_*_todo` tests (which only exercised the bespoke `to_terminal`) removed; `discovery::detection::ColorDepth`, `components::prose::Prose`, and `styling::{FontWeight, Style, Stylist}` imports re-pruned.
- Six `*_parity.rs` files collapsed: bespoke-vs-tree comparison tests deleted (tautological once `render_bespoke` is gone); tests that pin tree-path behaviour kept and rephrased; `assert_semantic_match` helper imports dropped where unused.

## Stage 3 Task 3e — NO_COLOR is enforced in the shared color-detection layer, not per-command

Two unrelated NO_COLOR enforcement paths existed before Stage 3:

1. **Per-command stripping** in `bt prose` (`biscuit-terminal/cli/src/commands/prose.rs:119-123`) — calls `strip_sgr_sequences` after rendering when `NO_COLOR` is set.
2. **Shared detection** — `color_depth()` in `biscuit-terminal/lib/src/discovery/detection/color.rs` never read `NO_COLOR`, so any tree-rendered command other than `prose` (e.g. `bt quote`, `bt status-block`) would emit SGR codes despite the env var.

The Stage 3 directive is single-source-of-truth: the *shared layer* honors `NO_COLOR`, components do not need per-command stripping. `color_depth()` now returns `ColorDepth::None` when `NO_COLOR` is set to a non-empty value, unless `FORCE_COLOR` or `CLICOLOR_FORCE` is also set (which acts as the conventional override, per the [NO_COLOR spec](https://no-color.org) and the `supports-color`/`chalk`/`clap` convention).

`bt quote "Test"` under `NO_COLOR=1` (with `FORCE_COLOR` unset) now emits `│ Test\n` with no SGR bytes. Verified by `test_tree_rendered_quote_respects_no_color` in `biscuit-terminal/cli/tests/integration_test.rs`. The existing per-command stripping in `bt prose` is now redundant defense-in-depth, not the load-bearing path — it can be removed in a future cleanup but was left alone here per the Rule 3 "surgical changes" guideline.

## Stage 3 complete (2026-05-20)

Stage 3 of the IR push is complete. The structural-projection gap closed
in Stage 2 is now closed across every adopted component, the
projection-fallback path is observable in production, and the migration
recipe is published so future component authors can flip-to-tree or
born-on-tree confidently.

Key deliverables:

- Three missing `render_tree_node` overrides added on `BlockQuote`,
  `StatusBlock`, and `FileSystem` (§3a.1).
- Nine existing overrides audited and parity-tested; nested-component
  tests tightened to assert structural `NodeKind`s and all
  `TODO(stage-3)` markers removed (§3a.2).
- `FileSystem::render` decision recorded as outcome (iii) — defer the
  terminal flip to Stage 4, behind the connector-list `Style` lowering
  and icon-name spacing reconciliation acceptance criteria (§3a.3,
  §3c.3).
- Projection fallback observable via `TerminalRenderable::type_name()`
  with a warn-once-then-debug `tracing` surface (§3b).
- Bespoke compatibility hooks retired on six components — `OrderedList`,
  `UnorderedList`, `Progress`, `Section`, `TextBlock`, `Todo` (§3c.1).
- Three sanctioned `render_bespoke` hooks retained and documented —
  `StatusBlock` (arbitrary border), `Table` (`prefer_cursor_alignment` +
  TTY path), `TwoColumn` (image overlay) — each marked
  `#[doc(hidden)] pub` with rustdoc naming the capability gap (§3c.2).
- Layout-matrix harness simplified: the right column compares
  `TreeRenderable::render_tree` directly (§3c).
- `NO_COLOR` honored at the shared color-detection layer rather than
  per-command (§3e).
- Migration recipe published at
  [`renderable/docs/migrate-component-to-ir.md`](../../../docs/migrate-component-to-ir.md)
  and linked from `renderable/README.md` and
  `.claude/skills/renderable/SKILL.md` (§3d).

The migration recipe is the canonical onward-path document for any
future component flip — both the *flip-from-bespoke* (Variant A) and
*born-on-the-tree* (Variant B) paths are prescribed there, alongside
the escape-hatch rules and the documentation-update obligations.

Outstanding follow-up for Stage 4:

- **`FileSystem::render` flip.** The Browser and Markdown targets
  already route through the tree; the terminal `render` body still
  calls the bespoke directory-tree renderer. The Stage 4 acceptance
  criteria are encoded in §3c.3 (connector-list `Style` lowering,
  icon-name spacing reconciliation, parity-fixture flip, and re-adding
  `FileSystem` to the default layout matrix).
