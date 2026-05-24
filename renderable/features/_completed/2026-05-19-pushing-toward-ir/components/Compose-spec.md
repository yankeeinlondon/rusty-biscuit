# Compose — IR Rendering Design Specification

## Component Status

| Field      | Value                                                                 |
|------------|-----------------------------------------------------------------------|
| Name       | Compose                                                               |
| Kind       | Sequence container; currently reports `is_block_level() -> false`     |
| Location   | `biscuit-terminal/lib/src/components/compose.rs`                      |
| Terminal   | bespoke `TerminalRenderable`                                          |
| Browser    | none                                                                  |
| Markdown   | none                                                                  |
| Tree       | none                                                                  |
| IR State   | needs a tree projection plus one approved render-tree sequence feature |
| bt CLI     | none                                                                  |

Compose holds a `Vec<RenderableTerminalContent>` and concatenates each part's
rendered output with no automatic separators. Parts are either plain `String`s
or `Rc<dyn TerminalRenderable>` components (Prose, Section, Table, List, etc.).
It owns a `Layout` for margins, alignment, max-width, and word-wrap.

## Review Corrections

The original draft said Compose could project to `NodeKind::Root` with one
flattened child per part and still preserve Compose's no-separator behavior.
That is incorrect for the current terminal and Markdown tree renderers.

`NodeKind::Root` is rendered as a block sequence. Terminal and Markdown join
root children with a blank line (`\n\n`). Therefore:

- `Compose::new(["foo", "bar"])` would become `foo\n\nbar` instead of `foobar`
  if it projected to `Root([Text("foo"), Text("bar")])`.
- `String + Section` would gain a blank line before the heading, while bespoke
  Compose simply concatenates the string and the section output.
- Plain root-level `Text` works only when there is a single child; it is not a
  general sequence model for Compose.

Compose can only flip its `TerminalRenderable` implementation to the tree path
after the render tree has an explicit target-agnostic sequence/concatenation
contract. That request is approved below.

## Design Steps

### Terminal IR Implementation

Compose should implement `render_tree_node()` and `TreeRenderable`, but its
projection must not rely on normal `Root` block joining for the no-separator
case.

#### Tree Projection Design

Compose's projection should produce a container node that carries explicit
sequence semantics:

- children are rendered in order;
- no separator is inserted by default;
- each child keeps its own structural node when available;
- child fallback behavior continues to use `RenderableTerminalContent::to_tree_nodes`;
- Compose layout is seeded onto the sequence container, not onto individual
  children.

Until the approved render-tree functionality is implemented, the nearest
transitional projection is:

```rust
fn render_tree_node(&self) -> Option<RenderNode> {
    let mut children = Vec::new();
    for part in &self.parts {
        let mut ctx = TreeProjectionContext::default();
        let result = part.to_tree_nodes(&mut ctx);
        children.extend(result.nodes);
    }

    let mut root = RenderNode::root(children);
    root.attrs.data.insert(
        "renderable.sequence.join".into(),
        serde_json::Value::String("none".into()),
    );
    if self.layout != Layout::default() {
        root.attrs.set_layout(&self.layout);
    }
    Some(root)
}
```

The exact API for the sequence marker should be chosen during render-tree
implementation. A typed `NodeAttrs` accessor is preferred over ad hoc string
lookups in component code, for example:

```rust
node.attrs.set_sequence_join(SequenceJoin::None);
```

The renderer behavior must be defined in the render tree, not inside Compose,
so Browser, Markdown, MarkdownPlus, and Terminal all share the same ordering and
separator semantics.

#### Projection Per Part

| Part type                         | Projection                                                                 |
|-----------------------------------|----------------------------------------------------------------------------|
| `String(s)`                       | `RenderNode::text(s)`                                                      |
| `Component(c)` with tree support  | include `c.render_tree_node()` directly                                    |
| `Component(c)` without tree       | use `to_tree_nodes` fallback according to `RenderStrictness`                |
| Projection recursion overflow     | preserve `Unsupported` plus diagnostic behavior from `to_tree_nodes`        |

Projection diagnostics are currently discarded by `render_tree_node()` because
the trait returns `Option<RenderNode>`. Tests should still cover the fallback
nodes that appear under Warn/Lossy projection.

#### Layout Mapping

Compose already owns a `Layout`. The projection seeds it onto the sequence
container via `attrs.set_layout(&self.layout)` when the layout is non-default.

Terminal layout must be applied by `render_with_layout`. Unlike the older
layout-and-style narrative, current terminal tree rendering does apply
`max_width` as a content-width cap, so Compose layout parity tests must include:

- left/right margins;
- center/right alignment;
- `max_width`;
- word-wrap behavior for text-heavy content.

Markdown continues to ignore layout. Browser lowers layout to inline CSS:
margins become `margin-*`, `max_width` becomes `max-width`, and center/right
alignment is represented with `auto` margins when `max_width` is present. The
browser renderer does not lower Compose block alignment to `text-align`.

#### Style Considerations

Compose itself has no `Style`; it is a pure sequence container. Child component
styles remain attached to child nodes. Compose must not introduce foreground
color, background, border, fill, or emphasis on its container unless a future
public Compose API adds those fields.

#### Semantic Change: `is_block_level()`

Current Compose reports `is_block_level() -> false`. That is meaningful in
legacy terminal composition because Compose behaves like a concatenating inline
fragment.

The tree-backed `TerminalRenderable` implementation should not silently flip
`Compose::is_block_level()` to `true`. Doing so would alter list, column, and
parent-component behavior that branch on `is_block_level()`. If a tree adapter
such as `TreeComponent<Compose>` reports block-level, parity tests should record
that as adapter drift, not as the desired Compose component behavior.

Recommendation: keep `Compose::is_block_level() -> false` unless a separate
behavioral migration explicitly changes the component contract.

#### Parity Test Strategy

Parity is asserted on ANSI-stripped output for content, with targeted exact
string assertions for separator-sensitive cases. Compose's defining behavior is
concatenation, so tests must not reduce everything to token-presence checks.

| Variant                                  | Validates                                                                    |
|------------------------------------------|------------------------------------------------------------------------------|
| Empty Compose                            | Both paths produce `""`                                                      |
| Single string part                       | Plain text survives both paths                                               |
| Two string parts                         | Exact output is `foobar`; no blank line or space is inserted                 |
| Three string parts with explicit newline | Only caller-provided newlines appear                                         |
| String + Prose                           | Content survives; Prose fallback/style loss is documented                    |
| String + Section                         | No implicit blank line is inserted before the heading                        |
| Section + string                         | No implicit blank line is inserted after the section                         |
| String + Table                           | Table data survives; separator drift is rejected unless caller supplied it   |
| String + UnorderedList                   | List items and bullets survive in order                                      |
| Multiple Prose parts                     | Concatenated content is identical after ANSI stripping                       |
| Compose with Layout                      | Tree layout matches legacy layout behavior for margin, alignment, max-width  |
| Nested Compose                           | Inner sequence semantics are preserved recursively                           |
| Unicode content                          | Multi-byte and wide content survives; width-sensitive layout is sane         |
| Many parts (100+)                        | No truncation, stack overflow, or quadratic behavior in normal use           |
| Unsupported child in Warn mode           | Fallback text appears and a lossy diagnostic can be observed where exposed   |
| Unsupported child in Strict mode         | Strict tree render fails visibly rather than silently dropping content       |

Known accepted divergences should be documented in a `KNOWN_DRIFT` ledger:

- **Prose styling loss**: Parts that are Prose components lose some styling in
  the generic terminal-content projection until Prose has full tree projection
  coverage. Content must survive.
- **Adapter block-level reporting**: `TreeComponent<Compose>` may report
  `true`; `Compose` itself should continue to report `false`.
- **Heading escape ordering**: Bespoke headings and tree headings may emit
  different SGR sequences. Stripped heading content must match.
- **HTML wrapper**: `render_browser_node` on a root sequence emits a wrapper
  `<div>`, while `render_browser_document` renders root children as page body
  fragments. CLI tests should assert the selected entry point explicitly.

#### Feature Requests for Tree Rendering

##### Request RT-COMPOSE-001: Add explicit no-separator sequence rendering

**APPROVED**

This feature request has been approved and WILL be included as part of the render-tree implementation BEFORE you are asked to implement this solution. Always refer to the @renderable/docs/tree-rendering.md and @renderable/docs/layout-and-style.md documents as the definitive guide.

Why: Compose's public contract is ordered concatenation with no automatic
separators. The current `Root` rendering contract is ordered block rendering
with blank-line separators in Terminal and Markdown. Treating Compose as a
plain `Root` would change observable output for basic inputs like `["foo",
"bar"]`. The render tree needs an explicit sequence/fragment join policy so a
component can preserve target-agnostic structural children without inheriting
document-block spacing.

Required behavior:

- Add a typed render-tree representation for sequence joining. This may be a
  dedicated node kind or a typed `NodeAttrs` hint on `Root`; prefer the smallest
  change that keeps exhaustive renderer handling explicit.
- Support at least `SequenceJoin::None`, meaning render children in order with
  no renderer-inserted separator.
- Terminal, Markdown, MarkdownPlus, and Browser renderers must honor the same
  child order and no-separator semantics.
- Normal document `Root` behavior must remain unchanged unless the sequence
  marker is present.
- Validation must reject sequence semantics in structurally invalid positions
  if the chosen representation can appear outside a block/container context.
- Tests must cover root/document behavior unchanged, Compose-style no-separator
  behavior, nested sequences, and mixed inline/block children.

#### Tree Renderer Fit Assessment

The existing tree renderer is a partial fit for Compose. Projection and child
fallback are already mostly available through
`RenderableTerminalContent::to_tree_nodes`, and layout can ride on the
container node. The missing piece is not child projection; it is the renderer's
lack of a no-separator sequence contract.

`will_use_tree_renderer`: **false** until RT-COMPOSE-001 is implemented. The
current renderer would introduce blank lines in terminal and Markdown output.

`will_use_tree_renderer_with_features`: **true**. With RT-COMPOSE-001,
Compose can use the tree renderer for terminal, browser, Markdown, and
MarkdownPlus outputs.

## Browser IR Implementation

Compose does not currently have a bespoke browser rendering implementation.
After Compose implements `TreeRenderable`, it should gain browser output through
a direct `BrowserRenderable` impl that follows `BrowserTreeComponent`'s error
policy:

```rust
impl BrowserRenderable for Compose {
    fn render_html_fragment(&self) -> BrowserFragment<Ready> {
        let tree = self.render_tree();
        let opts = BrowserRenderOptions {
            strictness: RenderStrictness::Warn,
            ..Default::default()
        };
        match render_browser_node(&tree, &opts) {
            Ok(rendered) => rendered.output,
            Err(error) => BrowserFragment::new()
                .define_as_text_fragment(format!("[render-tree error: {error}]"))
                .finalize(),
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
```

This avoids cloning Compose just to satisfy the existing owning
`BrowserTreeComponent<T>` adapter. If a borrowed tree adapter is added later,
the impl may delegate to that helper instead.

### Browser Rendering Behavior

| Part type                         | Browser output                                                       |
|-----------------------------------|----------------------------------------------------------------------|
| `String(s)`                       | escaped text in sequence order                                       |
| `Component` with tree node        | tree renderer converts the node to HTML                              |
| `Component` without tree          | ANSI-stripped fallback text                                          |
| Compose root sequence             | wrapper `<div>` when rendered with `render_browser_node`              |
| Compose document sequence         | body fragments when rendered with `render_browser_document`           |

Compose layout maps through the existing browser layout lowering. Tests must
assert `margin-*`, `max-width`, and `auto` margin alignment behavior where
applicable. They should not expect `text-align` for Compose alignment.

### Browser Test Variants

| Variant                         | Asserts                                                            |
|---------------------------------|--------------------------------------------------------------------|
| Empty Compose                   | Produces an empty wrapper or empty body for the selected entry     |
| Single string                   | HTML contains escaped text                                         |
| Two strings                     | Text nodes appear in order with no inserted textual separator      |
| String + Section                | Heading appears after the string in DOM order                      |
| String + Table                  | HTML contains `<table>` with expected headers and cells            |
| String + List                   | HTML contains `<ul>` or `<ol>` with list items                     |
| Compose with margins            | Wrapper has `margin-left` / `margin-right` CSS                     |
| Compose with max-width/alignment| Wrapper has `max-width` and appropriate `auto` margins             |
| Nested Compose                  | Inner sequence does not introduce extra text separators            |
| Prose fallback                  | Prose content appears as plain fallback text where no tree exists  |

## Markdown IR Implementation

### Markdown vs MarkdownPlus for Compose

Compose is a pure sequence container. It has no color, border, fill, or visual
styling of its own. Through the tree path, Markdown and MarkdownPlus should be
identical unless a child node or future renderer feature explicitly introduces
target-specific MarkdownPlus behavior.

The current tree Markdown renderer ignores `Layout` and `Style`, so Compose's
own layout does not affect Markdown output.

### Markdown Rendering Design

Compose projects to the approved sequence representation, and
`render_markdown_node` walks it.

| Part type                         | Markdown output                                                     |
|-----------------------------------|---------------------------------------------------------------------|
| `String(s)`                       | literal text `s`                                                    |
| `Component` with tree node        | Markdown for that structural node                                   |
| `Component` without tree          | ANSI-stripped fallback text                                         |
| Sequence join `None`              | no renderer-inserted blank line, space, or paragraph separator      |

### Markdown Test Variants

| Variant                       | Asserts                                                            |
|-------------------------------|--------------------------------------------------------------------|
| Empty Compose                 | Produces `""`                                                      |
| Single string                 | Markdown is the literal text                                       |
| Two strings                   | Exact Markdown is concatenated with no inserted separator          |
| String + Section              | No implicit blank line appears before `# Title`                    |
| Section + string              | No implicit blank line appears after the section                   |
| String + Table                | GFM table appears in order without extra leading separator         |
| String + List                 | List Markdown appears in order                                     |
| Compose with Layout           | Layout has no effect on Markdown output                            |
| Nested Compose                | Inner sequence semantics are preserved                             |
| Markdown equals MarkdownPlus  | Outputs are identical for mixed children without MarkdownPlus style |

## `bt` CLI

Compose should add a `bt compose` subcommand only after the tree-backed
component behavior is implemented. The command is useful because Compose is
primarily a composition primitive and the CLI can demonstrate mixed target
output.

### Current State

| Aspect              | Status                          |
|---------------------|---------------------------------|
| CLI command exists  | No                              |
| Render method       | N/A                             |
| Has `--md` switch   | No                              |
| Has `--html` switch | No                              |
| Has `--example`     | No                              |

### Specification Design

Add a `bt compose` subcommand that accepts ordered content parts and renders
through the tree path.

```
bt compose [OPTIONS] [ITEMS]...

bt compose --example
bt compose "Hello, " --prose "<bold>world</bold>!" " and more"
bt compose --md "Hello, " --prose "<bold>world</bold>!"
bt compose --html --heading 1 "Title" --text "Body text"
```

Arguments:

| Flag                       | Type          | Description                                                |
|----------------------------|---------------|------------------------------------------------------------|
| `ITEMS`                    | `Vec<String>` | Positional plain text items appended in order              |
| `--example` / `-e`         | `bool`        | Render example and show command                            |
| `--md`                     | `bool`        | Render portable Markdown                                   |
| `--md-plus`                | `bool`        | Render MarkdownPlus                                        |
| `--html`                   | `bool`        | Render HTML fragment                                       |
| `--text <TEXT>`            | repeatable    | Add a plain text part                                      |
| `--prose <TEXT>`           | repeatable    | Add a Prose-styled part                                    |
| `--heading <LVL> <TITLE>`  | repeatable    | Add a heading part; level must be 1 through 6              |
| `--list <ITEMS>...`        | repeatable    | Add an unordered list part; use `--` to end values         |
| `--ordered-list <ITEMS>...`| repeatable    | Add an ordered list part; use `--` to end values           |
| `--table <COLS> <ROWS>...` | repeatable    | Add a table part, e.g. `"Name,Age" "Alice,30"`             |
| `[command(flatten)]`       | `LayoutArgs`  | Shared margins (`--margin-left/right/top/bottom`) and `--alignment` flags |

The output target flags `--md`, `--md-plus`, and `--html` must be mutually
exclusive. Terminal remains the default target.

> **Deferred:** `--max-width` and `--word-wrap` are intentionally not part of
> `LayoutArgs` as of the Compose migration. `Compose`'s in-code `Layout` API
> already exposes both via `.max_width(..)` / `.word_wrap(..)`, and the tree
> renderers honor them — only the CLI surface is deferred. Extending
> `LayoutArgs` ripples through ~12 unrelated subcommands and would expose
> flags that silently do nothing until each consumer wires them through. The
> lessons-learned log captures the rationale; revisit once the bulk of the
> migration is complete and the rollout can land coherently across the
> shared arg struct.

Render path:

1. Build a `Compose` from the provided items in command-line order.
2. Apply `LayoutArgs` to the Compose layout.
3. Terminal: project tree, render through `render_terminal_node`.
4. HTML: project tree, render through the browser tree adapter.
5. Markdown: project tree, render through `render_markdown_node`.
6. MarkdownPlus: use the MarkdownPlus dialect/options if available; output is
   expected to match Markdown for Compose itself.

Example command:

```rust
const COMPOSE_EXAMPLE_CMD: &str = r#"bt compose --heading 1 "Project Status" --text "Build: " --prose "<green>passing</green>" --list "Unit tests" "Integration tests""#;
```

The `--example` flag renders this example and prints the command.

Registration in `args.rs`:

```rust
#[command(display_order = 19)]
Compose(compose::ComposeArgs),
```

Module file: `biscuit-terminal/cli/src/commands/compose.rs`

The module should follow the same target-switching pattern as `prose.rs`, but
all Compose output must route through the tree renderer after RT-COMPOSE-001
has landed.

## Acceptance Criteria Summary

- [ ] RT-COMPOSE-001 is implemented in the render tree before Compose flips to tree rendering
- [ ] Normal document `Root` blank-line behavior is unchanged without the sequence marker
- [ ] `Compose` implements `render_tree_node()` returning `Some(RenderNode)`
- [ ] `Compose` implements `TreeRenderable` (`render_tree()`)
- [ ] Compose projection preserves no-separator semantics for adjacent strings
- [ ] Compose projection preserves no-separator semantics for mixed text/block children
- [ ] Compose layout is seeded on the sequence container
- [ ] `Compose::is_block_level()` remains `false` unless a separate migration changes the public contract
- [ ] `Compose`'s `TerminalRenderable` delegates to the tree path by default after parity passes
- [ ] `Compose` implements or exposes `BrowserRenderable` through the tree adapter
- [ ] `Compose` implements `MarkdownRenderable` through the tree renderer
- [ ] `bt compose` subcommand is registered in `args.rs`
- [ ] `bt compose --md` renders Markdown output
- [ ] `bt compose --md-plus` renders MarkdownPlus output
- [ ] `bt compose --html` renders HTML output
- [ ] `bt compose --example` renders an example with command display
- [ ] `--md`, `--md-plus`, and `--html` are mutually exclusive
- [ ] Parity tests cover all variants listed above
- [ ] `KNOWN_DRIFT` ledger documents accepted divergences
