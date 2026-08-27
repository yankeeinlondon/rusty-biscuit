---
name: biscuit-terminal
description: Expert knowledge for the biscuit-terminal Rust library - terminal capability detection, rich terminal rendering, terminal components, inline image rendering, Mermaid/graph adapters, `TerminalRenderable`, `Prose`, and the terminal renderer for the renderable tree. Use when building CLI terminal output, using terminal-aware components, rendering images or diagrams, detecting color/underline/italics support, or folding `renderable::tree` nodes to terminal text.
hash: a552a21dc0a66232-95f568f032e66a02
---

# biscuit-terminal

`biscuit-terminal` owns terminal capability detection and terminal-facing
rendering. It defines `TerminalRenderable`, provides the reusable component
library, and renders `renderable::tree` nodes to terminal output.

## Start Here

- Detect capabilities with `Terminal::new()` or `Terminal::new_optimistic`.
- Prefer components from `biscuit_terminal::prelude` for rich terminal output.
- Use `Prose` for styled rich text instead of hand-written escape codes.
- For multi-target components, project through `renderable::tree` and share a
  helper between `TreeRenderable::render_tree` and
  `TerminalRenderable::render_tree_node`.
- Do not send raw escape codes from app code when a component or render-tree
  style can express the intent.

## Core Principles

1. Detect before rendering.
2. Gracefully fall back when a terminal lacks image, color, underline, or glyph
   support.
3. Keep static terminal facts on `Terminal`; query dynamic width/height through
   methods.
4. Validate image paths and apply app-level policy through
   `TerminalImageOptions`.
5. Keep nested components structural by projecting them into the render tree
   instead of degrading them to ANSI-stripped text.

## Common Entry Points

```rust
use biscuit_terminal::prelude::{Prose, TerminalRenderable};
use biscuit_terminal::terminal::Terminal;

let term = Terminal::new();
let output = Prose::new("Hello <b>world</b>").render(&term);
```

```rust
use biscuit_terminal::render_tree::{render_terminal_node, TerminalRenderOptions};
use renderable::tree::RenderNode;

let node = RenderNode::paragraph(vec![RenderNode::text("Hello")]);
let rendered = render_terminal_node(&node, &TerminalRenderOptions::default())?;
```

## Render-Tree Style Status

The terminal tree renderer consumes `renderable::layout::Layout` and
`renderable::style::Style`:

- layout margins/alignment/`max_width`/padding/width are applied around block nodes
- `Style` color/background/emphasis lower to SGR with terminal capability
  degradation
- `Style` border/fill render as terminal box glyphs and painted bands
- `bt block`, `bt progress`, and `bt table` provide CLI coverage for
  render-tree style behavior

The full per-component, per-target support contract is published in the
[Style Everywhere matrix](../../../renderable/features/2026-06-30-style-everywhere/matrix.md).

Browser and Markdown lowering for the same render tree are owned by
`renderable`, not this crate.

## Progressive Disclosure

Open only the topic file needed for the task:

| Topic | File |
|-------|------|
| Terminal struct and capability fields | `terminal-struct.md` |
| Component catalog | `components.md` |
| Render-tree terminal renderer | `render-tree.md` |
| Inline images | `image-rendering.md` |
| Mermaid diagrams | `mermaid-diagrams.md` |
| Colors | `color-system.md` |
| Detection functions | `discovery.md` |
| OS/environment detection | `os-environment.md` |
| Escape analysis | `escape-codes.md` |
| Prose/styled text | `styling.md` |
| `bt` CLI commands | `cli.md` |

Use the `renderable` skill for shared IR/layout/style definitions. Use the
`darkmatter` skill when Markdown parsing or `style:` frontmatter is involved.

## Testing Notes

- Level 1 tests use PTY-based library probes.
- Level 2 tests use `biscuit-test-harness` against real terminals such as
  WezTerm, Kitty, tmux, and Apple Terminal, and skip cleanly when unavailable.
- For HTML/CSS render output, use computed-style assertions when possible.
- For image protocols, assert terminal-specific capture behavior; WezTerm often
  requires debug/meta output while Kitty can preserve graphics protocol bytes.
- Internal `terminal-tests` and `browser-tests` features expose only their
  corresponding integration targets and harness dependencies. Ordinary local
  L1 keeps both disabled; tier recipes and CI enable them explicitly.

See `biscuit-test-harness/README.md` and the `cli` skill for the full testing
workflow.
