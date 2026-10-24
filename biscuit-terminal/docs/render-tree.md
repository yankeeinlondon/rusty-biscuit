# Render-Tree Hooks

`biscuit_terminal::render_tree` folds a `renderable` render tree into terminal
text. It knows nothing about syntax highlighting or diagrams; a program adds
both through two optional hooks on `TerminalRenderOptions`. This page says what
each hook receives and what the renderer does with its answer. The shared
concepts (graphics ceiling, Mermaid modes) are explained in
[`renderable/docs/tree-rendering.md`](../../renderable/docs/tree-rendering.md#terminal-mermaid-promotion-and-the-diagram-hook).

| Hook | Builder | Consulted for |
| --- | --- | --- |
| `code_renderer` | `with_code_renderer(Rc<dyn CodeRenderer>)` | every code node |
| `diagram_renderer` | `with_diagram_renderer(Rc<dyn DiagramRenderer>)` | a `mermaid` code node, when promotion is on (below) |

## The code renderer

Every `NodeKind::Code` node is offered to the code renderer with its language,
body, info-string `meta`, attributes, and a `TerminalCodeContext` (available
width, color depth, color mode, page colors, code theme, line numbers). A
`Some(String)` answer is printed verbatim; `None`, or no hook, prints the
built-in unhighlighted code block.

This includes code nodes a diagram hook returns: a text drawing (`txt`) and a
source fallback (`mermaid`) are highlighted by whatever code renderer the
program installed. `md` installs darkmatter's; a program that installs none
gets plain code blocks.

## The diagram hook

The renderer offers a `mermaid` fence to the diagram hook, ahead of the code
renderer, when `context.mermaid_mode` and `context.graphics_mode` allow it:

| `mermaid_mode` | `Rich` | `Vector` | `Off` |
| --- | --- | --- | --- |
| `Code` (default) | no | no | no |
| `Image` | yes | no | no |
| `Automatic` | yes | yes | no |

`DiagramRenderRequest` carries `source`, `attrs`, the capability snapshot
`terminal` (with `force_graphics` applied), `available_width` (the fence's
columns, narrowed inside list items and quotes), `graphics_mode`, the
ceiling the hook must not exceed, and `mermaid_mode`, the mode that offered the
fence (`Image` asks for an image only; `Automatic` may be answered with text).
The hook answers:

| Answer | What the renderer prints |
| --- | --- |
| `Ok(DiagramOutcome::Rendered(s))` | `s`, verbatim |
| `Ok(DiagramOutcome::Nodes(fragment))` | the fragment, laid out where the fence stood at its indent and width; its code nodes go through the code renderer, and a `mermaid` node in it is never offered to the diagram hook again |
| `Ok(DiagramOutcome::Declined)` | the ordinary code block, with no diagnostic under any strictness |
| `Err(e)` | `Strict`: the render fails with `LossyRejected`. `Warn`: a lossy diagnostic, then the code block. `Lossy`: the code block |

A minimal hook that shows the source unless images are allowed:

```rust
use biscuit_terminal::render_tree::{
    DiagramOutcome, DiagramRenderError, DiagramRenderRequest, DiagramRenderer,
};
use renderable::tree::{GraphicsMode, RenderNode};

struct TextOnly;

impl DiagramRenderer for TextOnly {
    fn render_terminal_diagram(
        &self,
        request: &DiagramRenderRequest<'_>,
    ) -> Result<DiagramOutcome, DiagramRenderError> {
        if request.graphics_mode == GraphicsMode::Rich {
            return Ok(DiagramOutcome::Declined);
        }
        Ok(DiagramOutcome::Nodes(vec![RenderNode::code(
            Some("txt".to_string()),
            None,
            format!("[diagram, {} columns]", request.available_width),
        )]))
    }
}
```

`biscuit-visualized`'s `TerminalDiagramRenderer` is the hook this repository
ships. The contract above is pinned by
`biscuit-terminal/lib/tests/l1/render_tree_diagram_hook.rs`.
