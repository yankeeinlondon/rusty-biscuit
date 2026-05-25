# Migrating a Component to the Render Tree IR

This document is the canonical recipe for taking a component into the
render-tree intermediate representation (IR). It is the onward-path companion
to [`tree-rendering.md`](./tree-rendering.md), which describes the architecture
as a status-and-direction document; this file is prescriptive.

If you are adding a brand-new component, jump to
[Variant B](#variant-b-born-on-the-tree). If you are taking an existing
component that already has a hand-written terminal renderer and routing it
through the tree, follow [Variant A](#variant-a-flip-from-bespoke).

## Overview

The render tree is `renderable::tree`: a target-agnostic, owned, serde-
serializable `RenderNode` graph. The slogan is *parse once, build one tree,
walk it per target*.

Every structural component should project into the tree so that:

- A single, audited projection serves the Terminal, Browser, and Markdown
  targets — no per-target divergence and no per-target maintenance.
- Containers that hold the component as nested content (a `BlockQuote`
  inside a `TwoColumn`, a `Compose` inside a `Section`) can pull the
  component's structural subtree instead of falling back to
  render-then-strip-ANSI text.
- A new render target can be added by walking `NodeKind` exhaustively
  rather than by re-implementing every component.

The IR is **not** an AST — it is a render-target projection. See the
[Tree Module skill page](../../.claude/skills/renderable/tree.md) and
[`renderable/README.md`](../README.md) for the trait surface.

## Variant A: flip from bespoke

A component currently has a hand-written `TerminalRenderable::render` (a
"bespoke" path) that emits ANSI directly. The goal is to retire that path
in favor of a tree projection that the shared terminal renderer lowers to
ANSI.

The component must already implement, or be ready to implement,
`TreeRenderable` (in `renderable`) and `TerminalRenderable` (in
`biscuit-terminal`).

1. **Factor a single private projection helper.** Add (or extract) one
   `fn to_render_node(&self) -> RenderNode` on the component, building the
   canonical subtree. This helper is the single source of truth for the
   component's structural shape.

2. **Implement `TreeRenderable::render_tree`** to delegate to that helper:

   ```rust
   impl TreeRenderable for MyComponent {
       fn render_tree(&self) -> RenderNode {
           self.to_render_node()
       }
   }
   ```

3. **Override `TerminalRenderable::render_tree_node`** to delegate to the
   same helper. This is the hook the projection layer calls when the
   component appears nested inside another component's tree:

   ```rust
   fn render_tree_node(&self) -> Option<RenderNode> {
       Some(self.to_render_node())
   }
   ```

   Both `TreeRenderable::render_tree` and
   `TerminalRenderable::render_tree_node` go through the same helper so
   the two cannot drift.

4. **Refactor `render` to route through the tree.** The established
   pattern is a per-component private `fn render_via_tree(&self, term:
   &Terminal) -> String` that calls `self.to_render_node()` then
   `render_terminal_node(&node, &TerminalRenderOptions::new(term,
   RenderStrictness::Warn))`. `TerminalRenderable::render` and
   `render_optimistic` delegate to it. There is no shared cross-component
   helper named `render_via_tree`; each component owns its own copy so
   that diagnostics, fallback strings, and `tracing` labels stay local.

5. **Add a parity test before flipping the public path.** The standing
   discipline (see
   [`renderable/features/2026-05-19-pushing-toward-ir/stage1-and-2/lessons-learned.md`](../features/2026-05-19-pushing-toward-ir/stage1-and-2/lessons-learned.md))
   is to add `biscuit-terminal/lib/tests/<component>_parity.rs` that
   renders the component both ways — the bespoke output versus the
   tree path — and asserts semantic invariants (token presence after ANSI
   stripping, structural `NodeKind` for nested cases). Document any
   accepted divergences in the test body. Once parity is reached, flip
   `render` to the tree path.

6. **Retire the bespoke renderer.** If the component has no terminal-only
   capability the tree cannot express, delete the `render_bespoke`
   helper, drop any private helpers it pulled in, and collapse the
   parity test to pin tree-path behavior only. See Stage 3 Task 3c.1
   for examples (`OrderedList`, `UnorderedList`, `Progress`, `Section`,
   `TextBlock`, `Todo`).

7. **Keep `render_bespoke` only when an escape hatch is required.** See
   [Escape-hatch rules](#escape-hatch-rules) below.

## Variant B: born on the tree

A new component should not gain a bespoke path it then has to retire.

1. **Define one private projection helper** as in Variant A step 1.

2. **Implement `TreeRenderable::render_tree`** to delegate to it.

3. **Implement `TerminalRenderable`** with `render` routing directly
   through the tree:

   ```rust
   impl TerminalRenderable for MyComponent {
       fn render(&self, term: &Terminal) -> String {
           let node = self.to_render_node();
           let opts = TerminalRenderOptions::new(term, RenderStrictness::Warn);
           match render_terminal_node(&node, &opts) {
               Ok(rendered) => rendered.output,
               Err(error) => {
                   tracing::error!(
                       component = "MyComponent",
                       error = %error,
                       "render_terminal_node failed; emitting empty output"
                   );
                   String::new()
               }
           }
       }
       fn render_tree_node(&self) -> Option<RenderNode> {
           Some(self.to_render_node())
       }
       /* ... layout, as_any, etc. */
   }
   ```

4. **Implement `BrowserRenderable` and `MarkdownRenderable`** by lowering
   the same projection through `render_browser_node` and
   `render_markdown_node`. The component never hand-writes ANSI, HTML,
   or Markdown.

5. **No `render_bespoke`.** Born-on-the-tree components have no
   pre-existing escape hatch and should not invent one. Use the
   `Unsupported` node if a presentation cannot be expressed yet; the
   renderers will surface it visibly.

## Escape-hatch rules

A `render_bespoke` hook is allowed to remain after migration only when
the component genuinely cannot lower its full capability through the
render tree.

When you keep one:

- **Capability gap is real and documented.** The rustdoc on
  `render_bespoke` must name the specific terminal-only capability that
  the canonical tree cannot express today (arbitrary border prefix,
  cursor-positioned image overlay, etc.) and state that removing the
  hook without first adding that capability to the tree is a regression.
- **Mark it `#[doc(hidden)] pub`.** `pub` so the parity tests and any
  in-crate sanctioned caller can reach it; `#[doc(hidden)]` so it is not
  part of the public surface.
- **The active `render` path stays on the tree.** `render` routes to
  `render_via_tree` for the common case and falls back to
  `render_bespoke` only when the escape-hatch condition is met (see the
  three sanctioned hooks below).
- **Cover it with an integration parity test.** A
  `biscuit-terminal/lib/tests/<component>_parity.rs` file (or a
  dedicated in-module test) that exercises the escape-hatch branch and
  pins its observable output, so a future tree-side improvement that
  lets the capability migrate cannot silently regress.

### `render_bespoke` retention after Stage 3

These are the only sanctioned escape hatches as of Stage 3:

| Component | File | Capability the tree cannot express |
|---|---|---|
| `StatusBlock` | `biscuit-terminal/lib/src/components/status_block.rs:339` | Arbitrary string `border` prefix that `Style::Border` cannot represent (the typed `Border` enum is closed). |
| `Table` | `biscuit-terminal/lib/src/components/table/table.rs:1581` | `prefer_cursor_alignment` knob with the TTY cursor-positioning render path. |
| `TwoColumn` | `biscuit-terminal/lib/src/components/two_column.rs:567` and `:586` | Inline `TerminalImage` overlay via cursor positioning; the tree's projection of an image inside a column degrades to `Unsupported`. |

Every other Stage 2 component flipped to the tree had its
`render_bespoke` retired in Stage 3 Task 3c.1. `FileSystem` is the only
adopted component that has not flipped yet — it keeps its bespoke
terminal `render` body without a `render_bespoke` hook, and the Stage 4
spec gates the flip on connector-list `Style` lowering and icon-name
spacing reconciliation.

## `project_renderable_content` and `ProjectionMode`

Container components in `biscuit-terminal` use the in-crate helper
`project_renderable_content(content, mode)` (defined in
`biscuit-terminal/lib/src/render_tree/projection.rs`) to project a
`RenderableTerminalContent` child — which may be a `String`, a `Prose`,
or any other `TerminalRenderable` — into a `Vec<RenderNode>`.

The helper and `ProjectionMode` are `pub(crate)`; this guidance therefore
applies to container components inside `biscuit-terminal`, not to
external crates.

Choose the mode based on what the container's child slot is allowed to
contain:

- **`ProjectionMode::Structural { terminal_hint }`** — for containers
  whose children may be block-level. The child's `render_tree_node` is
  used directly when present, so a nested `BlockQuote`, `Table`, or
  `Section` survives as its own `NodeKind`. When the child has no tree
  projection and `terminal_hint` is set, the helper renders through that
  terminal and wraps the ANSI-stripped output as a single `Text` node
  so capability-sensitive fallbacks (e.g. text-only HR tier vs. image
  tier) reflect the real target. Used by `Section`, `OrderedList`,
  `UnorderedList`, `Compose`.
- **`ProjectionMode::InlineOnly`** — for inline-prose containers where
  nested block content must flatten to text inside an inline run. Used
  by `BlockQuote::paragraph_children` so the quote's children stay in a
  single `Paragraph` and adjacent runs share one block. Any non-`Prose`
  child becomes a single ANSI-stripped `Text` node via its optimistic
  render, regardless of whether the child has a `render_tree_node`.

`Prose` children are always projected through `Prose::to_render_nodes`
so bold, italic, and colored runs survive as structured inline nodes
under both modes.

## CLI helper guidance

CLI commands in `biscuit-terminal/cli/` should construct the component,
detect the terminal once, and render via the component's
`TerminalRenderable` methods.

The standard pattern (see
[`biscuit-terminal/cli/src/commands/shared.rs`](../../biscuit-terminal/cli/src/commands/shared.rs)):

```rust
use crate::commands::shared::detect_terminal_honoring_force_color;

let term = detect_terminal_honoring_force_color();
println!("{}", my_component.render(&term));
```

`detect_terminal_honoring_force_color()` honors the conventional
color-forcing env vars (`FORCE_COLOR=1`, `CLICOLOR_FORCE=1`); for
inline rendering with a default terminal use `Terminal::default()` or
`Terminal::new()` directly.

Use `.display(&term)` when you need a guaranteed trailing newline (it
delegates to `render` and appends `\n` if absent — see the
`TerminalRenderable::display` rustdoc). Avoid bare `render_optimistic(None)`
for end-user CLI output: it hardcodes 80 columns and does no capability
detection — it is intended only for composition contexts where no real
terminal is available.

`NO_COLOR` is honored at the shared color-detection layer (see Stage 3
Task 3e in `lessons-learned.md`); CLI commands do not need to strip SGR
codes manually after rendering.

## Target fallback behavior

When a `NodeKind` cannot be rendered for a target, every renderer follows
the same shape: a structural `Error` fails the render regardless of
strictness, an `Unsupported` node yields a warning-severity validation
finding that escalates to an error in `Strict`, surfaces a visible
sentinel in `Warn`, and degrades silently in `Lossy`.

| Target | `Warn` output for `NodeKind::Unsupported { label }` | Source |
|---|---|---|
| Terminal | `<dim>[unsupported: LABEL]</dim>` via `Prose` | `biscuit-terminal/lib/src/render_tree/render.rs:1362` |
| Markdown / MarkdownPlus | `<!-- unsupported: LABEL -->` HTML comment | `renderable/src/tree/render/markdown.rs:621` |
| Browser | `<!-- unsupported: LABEL -->` raw HTML | `renderable/src/tree/render/browser.rs:846` |

All three also push a `Diagnostic::unsupported` onto the `Rendered<T>`
return, so callers in `Warn` mode see both the rendered fallback and
the diagnostic trail.

`Unsupported` is a real, visible node — never a silent drop. Use it
intentionally when a component encounters a presentation the tree
cannot express today.

## Documentation-update obligations

When you migrate a component, you must also update:

- **`biscuit-terminal/docs/components/<name>.md`** — describe the new
  tree-routed `render` path, list any retained `render_bespoke` hook and
  why, and document any newly-overridden `render_tree_node` behavior.
- **`.claude/skills/biscuit-terminal/`** and any skill files that
  reference the component, so the LLM-facing catalog stays current.
- **Any spec or feature doc** that mentions the component's bespoke path
  (search the relevant `renderable/features/.../*.md` set).
- **The component ledger** in
  [`renderable/docs/tree-rendering.md`](./tree-rendering.md) §3, if you
  flipped a previously-bespoke component or retired an escape hatch.
- **The README list of IR-aware components** in
  [`renderable/README.md`](../README.md) when the set of components that
  project through the tree changes.

## See also

- [`tree-rendering.md`](./tree-rendering.md) — the render-tree
  architecture as a status-and-direction document.
- [`components.md`](./components.md) — the component-side overview.
- [`renderable/features/2026-05-19-pushing-toward-ir/stage1-and-2/lessons-learned.md`](../features/2026-05-19-pushing-toward-ir/stage1-and-2/lessons-learned.md)
  — the per-component lessons learned during Stage 1, 2, and 3.
