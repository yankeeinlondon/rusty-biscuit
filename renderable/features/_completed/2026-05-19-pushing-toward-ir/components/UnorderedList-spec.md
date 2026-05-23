# UnorderedList — IR Rendering Design Specification

## Component Status

| Field      | Value                                                                              |
|------------|------------------------------------------------------------------------------------|
| Name       | UnorderedList                                                                      |
| Kind       | Block                                                                              |
| Location   | `biscuit-terminal/lib/src/components/list.rs`                                      |
| Terminal   | ✅ bespoke `TerminalRenderable`                                                     |
| Browser    | ❌ native impl; possible through `BrowserTreeComponent` after `TreeRenderable`       |
| Markdown   | ❌ native impl; possible through tree renderer after projection helper is canonical  |
| Tree       | ⚠️ terminal hook exists; canonical `TreeRenderable` impl is still required          |
| IR State   | projection exists, but old terminal render path still owns behavior                 |
| bt CLI     | bespoke (via `bt list`)                                                            |

UnorderedList holds a `Vec<RenderableTerminalContent>` and renders items with a
configurable bullet prefix (default: `- `). It owns a `Layout` for margins,
alignment, and word-wrap, plus `hanging_indent: bool` (default: true) and an
optional `indent_children: Option<u32>` that controls the indentation of nested
block-level children (defaults to the visible width of the bullet).

A terminal compatibility projection already exists via `render_tree_node()`, producing a
`NodeKind::List { ordered: false, start: None, children }` with each item
projected into `NodeKind::ListItem` nodes. The projection seeds the component's
`Layout` onto the root node and records `ListRenderHints` with the bullet,
hanging-indent flag, and any explicit `indent_children`. However, the default
`TerminalRenderable::render()` still uses the bespoke `render_content()` path.

Important: `render_tree_node()` is a `TerminalRenderable` compatibility hook,
not the canonical render-tree producer contract. Cross-target adapters such as
`TreeComponent<T>` and `BrowserTreeComponent<T>` require
`renderable::tree::TreeRenderable`. The implementation must therefore factor
the existing projection into one private helper and make both
`TerminalRenderable::render_tree_node()` and `TreeRenderable::render_tree()`
delegate to it.

The terminal tree renderer (`render_tree::render`) already has **native list
rendering** — it handles `NodeKind::List` and `NodeKind::ListItem` directly,
producing bulleted output with hanging indent and block-child indentation
without delegating back to the bespoke `UnorderedList` component.

---

## Design Steps

### Terminal IR Implementation

- The **UnorderedList** component does not currently have an IR-based rendering solution
- This section will describe what is required to ensure that the **UnorderedList** component:
    - has an IR implementation
    - the IR implementation drives the TerminalRenderable contract
    - the IR implementation is what is used by the bt CLI (note if **UnorderedList** doesn't yet have bt CLI subcommand then it will be designed below in the bt CLI section)

#### Tree Projection Status

UnorderedList already has a working tree projection via `render_tree_node()` at
`list.rs:616`. The projection:

1. Iterates `self.items`, calling `project_list_items()` on each.
2. Each item is projected via `RenderableTerminalContent::to_tree_nodes()` into
   a `NodeKind::ListItem`.
3. The list node is `RenderNode::list(false, None, children)` — unordered, no
   start index.
4. `ListRenderHints` are set: `bullet` (omitted for default `- `), `hanging_indent`,
   and `indent_children`.
5. Layout is seeded if non-default.

The native terminal tree renderer handles this structure at
`render_tree::render.rs:762` (`render_list`) and `:809` (`render_list_item`).

Before switching render paths, refactor the projection into a private helper
that returns a concrete `RenderNode` rather than an `Option<RenderNode>`:

```rust
impl UnorderedList {
    fn to_render_tree_node(&self) -> RenderNode {
        let children = project_list_items(&self.items);
        let mut node = RenderNode::list(false, None, children);
        let bullet = if self.bullet == "- " {
            None
        } else {
            Some(self.bullet.clone())
        };
        node.attrs.set_list_hints(&ListRenderHints {
            bullet,
            hanging_indent: self.hanging_indent,
            indent_children: self.indent_children,
        });
        if self.layout != Layout::default() {
            node.attrs.set_layout(&self.layout);
        }
        node
    }
}

impl renderable::tree::TreeRenderable for UnorderedList {
    fn render_tree(&self) -> RenderNode {
        self.to_render_tree_node()
    }
}
```

`TerminalRenderable::render_tree_node()` should then become
`Some(self.to_render_tree_node())`. This avoids drift between the terminal hook
and the canonical tree producer, and it makes `BrowserTreeComponent` usable.

#### Switching the Default Render Path

The goal is to make the IR path the default for both `TerminalRenderable::render()`
and the bt CLI, while **retaining the bespoke path** for parity testing.

The switch follows the same approach as the `OrderedList` spec: UnorderedList's
`TerminalRenderable::render()` and `render_optimistic()` delegates are changed
to project the tree and call `render_terminal_node()`.

**Implementation approach:**

```rust
impl TerminalRenderable for UnorderedList {
    fn render_optimistic(&self, term_width: Option<u32>) -> String {
        let width = term_width.unwrap_or(80);
        let term = Terminal::new_optimistic(width);
        self.render_via_tree(&term)
    }

    fn render(&self, term: &Terminal) -> String {
        self.render_via_tree(term)
    }

    // ... layout, is_block_level, as_any unchanged ...

    fn render_tree_node(&self) -> Option<RenderNode> {
        Some(self.to_render_tree_node())
    }
}

impl UnorderedList {
    fn render_via_tree(&self, term: &Terminal) -> String {
        let node = self.to_render_tree_node();
        let opts = TerminalRenderOptions::new(term, RenderStrictness::Warn);
        match render_terminal_node(&node, &opts) {
            Ok(rendered) => rendered.output,
            Err(error) => format!("[render-tree error: {error}]"),
        }
    }

    /// Retained for parity testing. Renders via the pre-tree bespoke path.
    fn render_bespoke(&self, term: &Terminal) -> String {
        let width = term.width();
        let available = self.layout.available_width(width);
        let content = self.render_content(Some(term), available);
        self.layout.apply_block_layout(&content, width)
    }
}
```

#### Layout Mapping

UnorderedList's `Layout` already seeds onto the projected `RenderNode` via
`node.attrs.set_layout(&self.layout)`. The terminal tree renderer's
`render_with_layout` applies it during rendering. No additional Layout
parameters are needed.

The `indent_children` parameter is carried via `ListRenderHints` on the node
attributes, which the native terminal renderer reads at `render.rs:776`:
`hints.indent_children.unwrap_or(default_indent)` — for unordered lists, the
default is `visible_width(&bullet)`.

#### Custom Bullet Handling

The custom bullet is carried via `ListRenderHints::bullet`. The tree renderer
reads it at `render.rs:770`: `hints.bullet.unwrap_or_else(|| "- ".to_string())`.
The projection normalizes the default `- ` to `None` to keep the tree
canonical:

```rust
let bullet = if self.bullet == "- " {
    None
} else {
    Some(self.bullet.clone())
};
```

This mapping is already in place and needs no change.

#### Hanging Indent Handling

The `hanging_indent` flag is carried via `ListRenderHints::hanging_indent` and
consumed by the native renderer at `render.rs:875`. The bespoke path's
word-wrap-with-hanging-indent behavior for string items (via `wrap_lines`) is
replaced by the tree renderer's approach at `render_list_text` (which creates a
`Prose` with `WrapProse(None, Some(prefix_width))`).

This is a known difference in code path that must be covered by the parity
tests. The functional output should be identical.

#### Style Considerations

UnorderedList has no visual Style of its own (no color, border, fill, or
emphasis). It is a structural container. No `Style` is seeded on the projected
node.

#### Parity Test Strategy

Critical test variants for the IR vs bespoke comparison:

| Variant                                                      | Validates                                                                               |
|--------------------------------------------------------------|-----------------------------------------------------------------------------------------|
| Empty UnorderedList                                          | Both paths produce `""`                                                                 |
| Single string item                                           | Output is `"- Item\n"` in both paths                                                    |
| Three string items                                           | Bullet prefix preserved on all lines                                                    |
| Long string item requiring word wrap                         | Hanging indent on continuation lines matches or is recorded in `KNOWN_DRIFT`            |
| Custom bullet (`"→ "`)                                       | Custom bullet used in both paths                                                        |
| Custom bullet with different visible width (`"*) "`)         | Width computation correct; hanging indent adjusted                                      |
| Disable hanging indent                                       | Continuation lines have no extra indent                                                 |
| String item + Prose component item                           | Content survives both paths; any Prose styling loss in tree path is documented          |
| String item + nested UnorderedList (block child)             | Block child is indented by `indent_children`; bullets at correct indent                 |
| String item + nested OrderedList (block child)               | Block child renders with numbered prefix at correct indent                              |
| Three-level nesting (UL > UL > UL)                           | Indent compounds: 2 + 2 + 2 spaces; no width overflow                                  |
| Empty nested UnorderedList                                   | Empty child list behavior is explicit and covered by parity/structural tests            |
| UnorderedList with left/right margins                        | Layout is applied via `set_layout`; margins narrow the available width                  |
| UnorderedList with alignment                                 | Block alignment applied as a unit                                                       |
| UnorderedList with custom `indent_children`                  | Block children indented by specified amount                                             |
| Mixed inline + block children                                | First child gets prefix, block children get indent, no prefix                           |
| `From<Vec<RenderableTerminalContent>>` with hanging indent   | Component items receive automatic hanging indent configuration                          |
| `From<Prose>` conversion                                     | Single Prose item renders correctly as a one-item list                                  |
| Very narrow terminal width (e.g. 10)                         | Content remains present; accepted wrap drift is captured rather than hidden             |
| Bullet changed after construction (`with_bullet`)            | Bullet and hanging indent updated on existing items                                     |

Parity should use two levels:

1. **Semantic invariants**: ANSI-stripped token presence, item ordering, marker
   presence, and no dropped children. These are required for every case.
2. **Exact or facet parity where feasible**: compare stripped output, indent
   shape, blank-line shape, and line widths. Any accepted difference goes into
   the existing `KNOWN_DRIFT` ledger in `render_comparison.rs` with a verdict.

Known accepted divergences should be documented in `KNOWN_DRIFT`:

- **Prose styling loss**: Items that are Prose components lose styling in the
  tree path because Prose's `render_tree_node()` returns `None`, triggering
  ANSI-stripped fallback. Content is preserved; styling is not.
- **Hanging indent code path**: The bespoke path uses `wrap_lines` with
  `WordWrap::WrapProse(None, Some(indent))` directly, while the tree renderer's
  `render_list_text` creates a temporary `Prose` and calls `render_in_width`.
  Both should produce the same wrapping, but the intermediate representations
  differ.
- **Width handling**: The bespoke path manually computes `child_width =
  term_width - bullet_width` for each item. The tree renderer's width
  management may produce slightly different line breaks in edge cases at very
  narrow widths.
- **Top/bottom margin behavior**: the tree renderer applies vertical margins
  from `Layout`; some bespoke layout paths historically ignored them. If the
  bespoke output is behind the tree behavior, classify it as `BespokeBehind`
  rather than changing the tree renderer to match the old behavior.
- **Canonical default bullet vs CLI default bullet**: the component's canonical
  default bullet is `"- "`, while `bt list` currently defaults to `"• "`.
  The CLI default is user-facing terminal presentation. Markdown output must
  still use standard `- ` syntax.

#### Feature Requests for Tree Rendering

No feature requests are needed, so there are no render-tree implementation
requests to approve or deny for this component.

The existing tree renderer already has native
support for unordered lists with:

- Configurable bullet prefix via `ListRenderHints::bullet`
- Hanging indent for wrapped continuation lines via `ListRenderHints::hanging_indent`
- Block-child indentation via `ListRenderHints::indent_children`
- Mixed inline/block child handling in `render_list_item`
- Layout application via `set_layout`
- Width-aware wrapping per item

The `NodeKind::List { ordered: false }` node kind and the native rendering in
`render_tree::render.rs` are a direct match for UnorderedList's semantics.

#### Tree Renderer Fit Assessment

The existing tree renderer is an **excellent fit** for UnorderedList. The
component's entire rendering logic — configurable bullet prefix, hanging
indent, block-child indentation, and Layout application — has a corresponding
native implementation in the tree renderer. The `NodeKind::List` /
`NodeKind::ListItem` node types were designed to represent exactly this kind of
component.

UnorderedList's tree projection already exists and is tested (see
`unordered_list_render_tree_node_carries_layout_when_margins_set` at `list.rs:893`).
The remaining work is switching the default render path and adding comprehensive
parity tests.

`will_use_tree_renderer`: **true** — the existing tree renderer handles
UnorderedList's needs without any feature additions.

`will_use_tree_renderer_with_features`: **true** — no features requested, so
this is the same as above.

---

### Browser IR Implementation

- In this section we will provide a design specification for the **UnorderedList** component's browser output through the render-tree adapter

UnorderedList does not currently have a bespoke browser rendering implementation.
Since Terminal IR is designed first and UnorderedList already projects to a
`NodeKind::List { ordered: false }` render tree node, the browser path should be
handled by the existing `BrowserTreeComponent<T>` adapter in
`biscuit-terminal/lib/src/render_tree/browser_adapter.rs` **after**
`UnorderedList` implements `TreeRenderable`.

The browser tree renderer already handles `NodeKind::List` at
`renderable/src/tree/render/browser.rs:363`:

- Unordered lists (`ordered: false`) produce `<ul>`.
- Each `NodeKind::ListItem` produces `<li>` with optional checkbox for task
  items.
- Child content (text, paragraphs, nested lists) is rendered recursively.

UnorderedList gains `BrowserRenderable` by wrapping itself in the adapter:

```rust
use biscuit_terminal::render_tree::BrowserTreeComponent;
use renderable::browser::BrowserRenderable;

let ul = UnorderedList::new(vec!["First", "Second", "Third"]);
let component = BrowserTreeComponent::new(ul);
let fragment = component.render_html_fragment();
let html = fragment.render();
// Produces: <ul><li>First</li><li>Second</li><li>Third</li></ul>
```

Do not implement a separate bespoke browser serializer for `UnorderedList`
unless the tree adapter proves structurally unable to represent list semantics.
The adapter's infallible error policy is acceptable for the trait boundary:
structural errors become a visible fallback fragment, while normal list output
is rendered through `render_browser_node`.

#### Layout to CSS Mapping

UnorderedList's `Layout` maps to CSS via the existing `layout_to_css` lowering in
`renderable/src/tree/render/browser.rs`:

- Margins → `margin-*` properties on the `<ul>` wrapper
- Alignment → `margin-left:auto` / `margin-right:auto` when `max_width` is present
- `max_width` → `max-width` CSS property

No additional CSS mapping is needed beyond what the tree renderer already provides.

#### Key Test Variants

| Variant                                      | Asserts                                                                          |
|----------------------------------------------|----------------------------------------------------------------------------------|
| Empty UnorderedList                          | Produces `<ul></ul>` with no children                                            |
| Single string item                           | HTML contains `<ul><li>Item</li></ul>`                                           |
| Three string items                           | Three `<li>` elements inside `<ul>` with correct text                           |
| Custom bullet (`"→ "`)                       | HTML is unaffected (bullet is a terminal rendering concern)                     |
| Nested UnorderedList (block child)           | HTML contains nested `<ul>` inside an `<li>`                                    |
| Nested OrderedList (block child)             | HTML contains `<ol>` inside an `<li>`                                           |
| String + Prose item                          | HTML contains text content (Prose renders as fallback text since no tree)        |
| Layout with margins                          | `<ul>` wrapper has `margin-left` / `margin-right` CSS                           |
| Layout with alignment and max-width          | `<ul>` wrapper has `max-width` and auto margin CSS for block alignment           |
| Custom `indent_children`                     | No effect on HTML output (indent is a terminal concern)                          |
| Long text item                               | Content is present in the `<li>`; wrapping is CSS-driven                        |
| Invalid projected tree via adapter fixture   | Adapter emits visible fallback fragment instead of panicking                     |

---

### Markdown IR Implementation

#### Markdown vs MarkdownPlus for UnorderedList

UnorderedList is a structural container with no color, border, fill, or visual
styling of its own. This means:

- **Both Markdown and MarkdownPlus produce identical output** for UnorderedList.
- An unordered list is natively representable in CommonMark: `- Item`.
- There is no situation where UnorderedList's own structure would diverge between
  the two targets.

The only potential divergence would come from child components that have color
or style (e.g., a Prose with `<red>error</red>`). Since the tree-based Markdown
renderer ignores `Style` entirely (locked by regression test), and UnorderedList
adds no style of its own, the two outputs are identical through the tree path.

Note: the **custom bullet** is a terminal-only concern. The Markdown renderer
always produces `- Item` regardless of the bullet set on the component. This is
intentional — Markdown's unordered list syntax uses `- ` as the standard marker
and has no facility for custom bullets.

#### Markdown Rendering Design

The Markdown tree renderer already handles `NodeKind::List` at
`renderable/src/tree/render/markdown.rs:308`:

- Unordered lists produce `- First\n- Second\n- Third`
- Existing continuation lines are indented to align under the marker
- Block children within list items are indented correctly

The Markdown renderer does **not** perform width-based word wrapping. Long item
text remains a single Markdown line unless the projected child content already
contains line breaks. This is intentional because Markdown output is structural
source text, not terminal layout.

UnorderedList projects to `NodeKind::List { ordered: false, start: None }`, so
the Markdown renderer will produce standard bullet list syntax.

```rust
use renderable::tree::render::{render_markdown_node, MarkdownRenderOptions};

let ul = UnorderedList::new(vec!["First", "Second", "Third"]);
let node = ul.to_render_tree_node();
let rendered = render_markdown_node(&node, &MarkdownRenderOptions::default());
// Produces: "- First\n- Second\n- Third"
```

Layout is ignored by the Markdown renderer (by design — locked by test).

UnorderedList can implement `MarkdownRenderable` by projecting its tree and
calling `render_markdown_node`:

```rust
impl MarkdownRenderable for UnorderedList {
    fn render_markdown(&self) -> String {
        let node = self.to_render_tree_node();
        render_markdown_node(&node, &MarkdownRenderOptions::default())
            .map(|r| r.output)
            .unwrap_or_default()
    }

    fn render_markdown_plus(&self) -> String {
        // Identical for UnorderedList — no styling divergence
        self.render_markdown()
    }
}
```

#### Key Test Variants

| Variant                                  | Asserts                                                                           |
|------------------------------------------|-----------------------------------------------------------------------------------|
| Empty UnorderedList                      | Produces `""`                                                                     |
| Single item                              | Markdown is `"- Item"`                                                            |
| Three items                              | Markdown is `"- First\n- Second\n- Third"`                                       |
| Long item                                | Remains one Markdown line unless source content contains line breaks              |
| Item containing explicit newline         | Continuation lines are indented to marker width                                   |
| Nested UnorderedList                     | Markdown contains indented bullet sublist                                         |
| Nested OrderedList                       | Markdown contains indented numbered sublist                                       |
| Mixed string + component items           | Content appears in Markdown; components without tree render as plain text         |
| UnorderedList with Layout                | Layout has no effect on Markdown output (regression test)                         |
| Markdown equals MarkdownPlus             | Both methods produce identical output                                             |
| Item with inline styling (Prose)         | Styled text is degraded to plain text in both Markdown and MarkdownPlus           |
| Custom bullet                            | Markdown uses standard `- ` regardless of custom bullet                           |
| Invalid list structure fixture           | Strict rendering fails validation rather than silently dropping content            |

---

### `bt` CLI

- This specification will ensure that the **UnorderedList** component:
    - has a 'bt' CLI subcommand for rendering this component
    - that the '--md' and '--html' CLI switches are available to render to Markdown and HTML targets respectively (the default render is always for the Terminal)
    - that the '--example' CLI switch is in place to provide a thoughtful example of how this command should be used with the CLI (see other working examples for a template)

#### Current State

| Aspect              | Status                                                           |
|---------------------|------------------------------------------------------------------|
| CLI command exists  | Yes — `bt list` renders `UnorderedList`                          |
| Render method       | Bespoke — calls `UnorderedList::render()` directly               |
| Has `--md` switch   | No                                                               |
| Has `--html` switch | No                                                               |
| Has `--example`     | Yes (`bt list --example`)                                        |

The existing `bt list` command (`biscuit-terminal/cli/src/commands/list.rs`)
creates an `UnorderedList` with a configurable bullet (default `• `). Each item
is parsed through `Prose` for styled text support. It supports `--example`,
`--bullet`, `--no-hanging-indent`, and shared `LayoutArgs`.

The render path currently calls `list.render(&term)` directly (bespoke). It does
not support `--md` or `--html` switches.

#### Specification Design

Extend the existing `bt list` command to add `--md`, `--md-plus`, and `--html`
switches following the pattern established by `bt prose`. Switch the terminal
render path to use the tree renderer (consistent with the Terminal IR
implementation above).

**CLI structure:**

```
bt list [OPTIONS] [ITEMS]...

bt list --example
bt list "First item" "Second item" "Third item"
bt list --bullet "→ " "Option A" "Option B"
bt list --md "First" "Second" "Third"
bt list --html "First" "Second" "Third"
bt list --md-plus "First" "Second" "Third"
```

**Updated args:**

| Flag                          | Type           | Description                                                       |
|-------------------------------|----------------|-------------------------------------------------------------------|
| `ITEMS`                       | `Vec<String>`  | Positional list items (required unless `--example`)               |
| `--example` / `-e`            | `bool`         | Render example and show command                                   |
| `--bullet` / `-b`             | `String`       | Bullet character for unordered lists (default: `• `)              |
| `--no-hanging-indent`         | `bool`         | Disable hanging indent on wrapped lines                           |
| `--html`                      | `bool`         | Render to HTML fragment (conflicts with `--md`, `--md-plus`)      |
| `--md`                        | `bool`         | Render to portable Markdown (conflicts with `--html`, `--md-plus`)|
| `--md-plus`                   | `bool`         | Render to MarkdownPlus (conflicts with `--html`, `--md`)          |
| `[command(flatten)]`          | `LayoutArgs`   | Shared margin/alignment flags                                     |

**Render path:**

1. Parse items (unescape shell escapes, wrap as Prose for styled text).
2. Build `UnorderedList::from(items).with_bullet(&bullet)`.
3. Apply `--no-hanging-indent` if set.
4. Apply `LayoutArgs` to the component's layout.
5. **Terminal** (default): Render via `render(&term)` (now tree-based after IR switch).
6. **HTML** (`--html`): Wrap in `BrowserTreeComponent` → `render_html_fragment()`.
7. **Markdown** (`--md`): Project tree → `render_markdown_node()`.
8. **MarkdownPlus** (`--md-plus`): Same as `--md` for lists (outputs are identical).

For `--md` and `--md-plus`, layout flags should not change the Markdown body.
This follows the render-tree contract: Markdown ignores `Layout`. If the CLI
keeps the broader `bt prose` convention of serializing layout as frontmatter,
that must be an explicit CLI-layer addition and covered by tests; it should not
be attributed to the tree Markdown renderer.

For `--html`, layout flags should be visible as inline CSS on the rendered
`<ul>` through `layout_to_css`; avoid wrapping the fragment in an extra
layout-only `<div>` unless a browser parity test proves the adapter cannot put
the style on the list node.

**Example definitions** (unchanged from existing):

```rust
const LIST_EXAMPLE: &[&str] = &[
    "<b>Plan</b> the change",
    "<green>Run</green> focused tests",
    "Ship the smallest useful fix",
];
const LIST_EXAMPLE_CMD: &str = r#"bt list "<b>Plan</b> the change" "<green>Run</green> focused tests" "Ship the smallest useful fix""#;
```

**Module changes**: Update `biscuit-terminal/cli/src/commands/list.rs` to import
the multi-target rendering helpers and add the `--html`, `--md`, `--md-plus`
flags with their rendering logic (following the pattern from `prose.rs`).

The HTML path wraps the UnorderedList in `BrowserTreeComponent` and calls
`render_html_fragment()`. The Markdown path calls `render_markdown_node()` on the
projected tree. Both follow the existing adapter patterns.

---

## Acceptance Criteria Summary

- [ ] `UnorderedList`'s `TerminalRenderable::render()` delegates to the tree path by default
- [ ] `UnorderedList` has one private projection helper used by both tree-related traits
- [ ] `UnorderedList` implements `renderable::tree::TreeRenderable`
- [ ] Bespoke render path retained as `render_bespoke()` for parity testing
- [ ] `BrowserRenderable` achieved via `BrowserTreeComponent<UnorderedList>`
- [ ] `MarkdownRenderable` implemented on `UnorderedList` via tree renderer's Markdown path
- [ ] `bt list` (terminal default) renders through the tree renderer
- [ ] `bt list --md` renders Markdown output (`- First\n- Second\n- Third`)
- [ ] `bt list --html` renders HTML output (`<ul><li>...</li></ul>`)
- [ ] `bt list --md-plus` renders MarkdownPlus output (identical to `--md` for lists)
- [ ] `bt list --example` renders example with command display (unchanged)
- [ ] Parity tests (bespoke vs tree) cover all variants listed in Terminal IR section
- [ ] `KNOWN_DRIFT` ledger documents accepted divergences
- [ ] `bt list --bullet`, `--no-hanging-indent`, and `LayoutArgs` continue to work unchanged
