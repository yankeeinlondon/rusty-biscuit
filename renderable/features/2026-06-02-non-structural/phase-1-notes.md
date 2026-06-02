---
phase: 1
completed: 2026-06-02
---

# Phase 1 Implementation Notes: Non-Structural Component Exemptions

## Success Criteria

- [x] Exemption criterion documented (document-pipeline participation).
- [x] Exemption Register consumable by the cutover.
- [x] Node-kind helper verification inventory completed.

## Related References Reviewed

| Reference | File | Key Findings |
|---|---|---|
| Cutover spec | `features/2026-06-02-tree-cutover/spec.md` | AC#2 narrowed; Decision #5 resolved by this spec. Component holdouts table lists exempt components as `no changes`. |
| Graphics-policy spec | `features/2026-05-26-graphics-policy/spec.md` | Owns `Image` / `ThematicBreak` / `Code{mermaid}` node renderers. Defines `GraphicsMode` and promotion boundaries. |
| Perf-gate spec | `features/2026-06-02-perf-gate/spec.md` | Defines benchmark suite; exempt components excluded from fixture design. |
| Component catalog | `docs/components.md` | IR-state column tracks component render path. Currently lists all exempt components as `no changes`. |

## Exempt Component Inventory

### Terminal Layout Primitives / UI Affordances (permanently exempt)

| Component | Crate | Tree Projection | Render Paths | Evidence Location |
|---|---|---|---|---|
| `PadLeft` | biscuit-terminal | No (`render_tree_node` returns `None`) | Terminal only | `biscuit-terminal/lib/src/components/pad.rs:34,78` |
| `PadRight` | biscuit-terminal | No | Terminal only | `biscuit-terminal/lib/src/components/pad.rs:135,179` |
| `InlineContent` | biscuit-terminal | No | Terminal only | `biscuit-terminal/lib/src/components/inline_content.rs:80,275` |
| `Status` | biscuit-terminal | No | Terminal only | `biscuit-terminal/lib/src/components/status.rs:422,533` |

### Standalone Graphics/Viz Widgets (exempt now)

| Component | Crate | Tree Projection | Render Paths | Evidence Location |
|---|---|---|---|---|
| `GraphExpression` | biscuit-terminal | No | Terminal + Browser | `biscuit-terminal/lib/src/components/graph_expression.rs:73,372,391` |
| `FileTree` | darkmatter | No | Terminal only | `darkmatter/lib/src/markdown/reference/file_tree/mod.rs:124,268` |

### Node-Kind Builder/Helpers (exempt from separate projection)

| Component | Crate | Tree Projection | Render Paths | Document Node Kind | Evidence Location |
|---|---|---|---|---|---|
| `HorizontalRule` | biscuit-terminal | No | Terminal + Browser | `NodeKind::ThematicBreak` | `biscuit-terminal/lib/src/components/horizontal_rule/mod.rs:38,152,221` |
| `TerminalImage` | biscuit-terminal | No | Terminal only | `NodeKind::Image` | `biscuit-terminal/lib/src/components/terminal_image/mod.rs:195,227` |
| `MermaidDiagram` | biscuit-terminal | No | Terminal only | `NodeKind::Code { lang: "mermaid" }` | `biscuit-terminal/lib/src/components/mermaid.rs:378,573` |

### Page Frame

| Component | Crate | Tree Projection | Render Paths | Evidence Location |
|---|---|---|---|---|
| `DarkmatterPage` | darkmatter | No (`render_tree_node` returns `None`) | Terminal only (minimal browser path planned per perf-gate) | `darkmatter/lib/src/layout/page.rs:65,1194` |

## Node-Kind Semantics That Stay On-Tree

| Node Kind | Terminal Renderer | Browser Renderer | Markdown Renderer | Evidence |
|---|---|---|---|---|
| `NodeKind::ThematicBreak` | `biscuit-terminal/lib/src/render_tree/render.rs:418` | `renderable/src/tree/render/browser.rs:273,429` | `renderable/src/tree/render/markdown.rs:259` | Tree renderers own the document lowering; helper calls (`HorizontalRule`) are below the node renderer. |
| `NodeKind::Image` | `biscuit-terminal/lib/src/render_tree/render.rs:747` | `renderable/src/tree/render/browser.rs:292,899` | `renderable/src/tree/render/markdown.rs:308` | Tree renderers own the document lowering; `TerminalImage` encodes chosen terminal tier below the graphics-policy boundary. |
| `NodeKind::Code { lang: "mermaid", .. }` | `biscuit-terminal/lib/src/render_tree/render.rs:415` (via `render_code_node`) | `renderable/src/tree/render/browser.rs:270` | `renderable/src/tree/render/markdown.rs:247` | Stays a code node until Mermaid-aware renderer promotes it; `MermaidDiagram` called only below promotion boundary. |

## Status vs StatusBlock Verification

- `StatusBlock` implements `TreeRenderable` and is on the tree (`biscuit-terminal/lib/src/components/status_block.rs:445`).
- Bare `Status` does **not** implement `TreeRenderable` and is UI chrome only.
- Mechanical search of `darkmatter/lib/src/markdown` finds **no** document path emitting bare `Status`. All error and diagnostic paths emit `StatusBlock`.

## Off-Tree Components Not in Register

All components found without tree projection are accounted for:

1. **In the Exemption Register** (10 components): `PadLeft`, `PadRight`, `InlineContent`, `Status`, `GraphExpression`, `FileTree`, `HorizontalRule`, `TerminalImage`, `MermaidDiagram`, `DarkmatterPage`.
2. **Already tree-render-only** (1 component): `Prose` (`biscuit-terminal/lib/src/components/prose/tree.rs:54`).
3. **Tree projection exists, default path still bespoke** (2 components): `FileSystem` (terminal path), `YamlBlock`. These are tracked by the cutover spec, not exempt.

**No unclassified blockers found.**

## Validation Checkpoint

- [x] Every off-tree component found by mechanical search is either listed in the register, already tree-render-only, or explicitly recorded as a blocker needing classification before Phase 5.
- [x] All document-pipeline node kinds (`ThematicBreak`, `Image`, `Code{mermaid}`) have confirmed tree renderer ownership.
- [x] No darkmatter document path emits bare `Status`.
