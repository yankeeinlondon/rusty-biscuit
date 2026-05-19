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
