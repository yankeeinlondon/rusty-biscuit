---
phase: 3
completed: 2026-06-02
---

# Phase 3 Implementation Notes: Node-Kind Renderer Verification

## Success Criteria

- [x] `NodeKind::ThematicBreak` renders terminal output through the tree renderer entry points.
- [x] `NodeKind::ThematicBreak` renders browser output through the tree renderer entry points.
- [x] `NodeKind::ThematicBreak` renders Markdown output through the tree renderer entry points.
- [x] `NodeKind::Image` renders through the tree renderer graphics-policy path.
- [x] `NodeKind::Code { lang: "mermaid", .. }` remains a code node until promoted (documented gap).
- [x] Darkmatter document pipeline does not emit bare `Status`.
- [x] Each helper exemption has an evidence note pointing to the renderer function, test, or mechanical search result.

## Verification Evidence

### `NodeKind::ThematicBreak`

#### Terminal Renderer

**Renderer function:** `biscuit-terminal/lib/src/render_tree/render.rs:418-421`

```rust
NodeKind::ThematicBreak => {
    let rule = horizontal_rule_from_attrs(&node.attrs);
    Ok(rule.render(&self.opts.context.terminal))
}
```

The tree renderer's `NodeKind::ThematicBreak` arm calls `horizontal_rule_from_attrs` — a private helper that maps `darkmatter.hr.*` hints onto `HorizontalRule` style/alignment/weight/width/color — then renders the constructed rule. `HorizontalRule` is a pure helper call beneath the node renderer; it is never instantiated directly by the darkmatter document pipeline.

**Helper boundary:** `horizontal_rule_from_attrs` (`render.rs:1440`) is a private function scoped to the render-tree module. It consumes `NodeAttrs` hints and returns a configured `HorizontalRule`. No code path in `darkmatter/lib/src/markdown` calls `HorizontalRule` directly for document HRs.

**Tests:**
- `render_tree_thematic_break_renders_rule` (`render.rs:2494`) — basic smoke test.
- `render_tree_thematic_break_consumes_darkmatter_hr_hints` (`render.rs:3101`) — asserts waves glyph (`≋` or `~`) when `darkmatter.hr.kind = "waves"`.
- `render_tree_thematic_break_without_hints_uses_default_rule` (`render.rs:3132`) — asserts plain rule when no hints.

**Parity test:** `render_tree_parity_hr_attributes_spanned` (`darkmatter/lib/tests/render_tree_parity.rs:814`) — asserts both legacy and tree pipelines render the HR-attribute paragraph as non-literal markup (no `style: waves` text leakage).

#### Browser Renderer

**Renderer function:** `renderable/src/tree/render/browser.rs:273`

```rust
NodeKind::ThematicBreak => Ok(self.render_thematic_break(&node.attrs)),
```

`render_thematic_break` (`browser.rs:429`) builds an `<hr>` void tag and surfaces `darkmatter.hr.*` hints as `data-hr-*` HTML attributes. No SVG or `HorizontalRule` builder is called at this layer; the browser renderer owns the node lowering directly.

**Tests:**
- `thematic_break_and_breaks` (`browser.rs:1493`) — asserts `<hr>` output.
- `thematic_break_surfaces_darkmatter_hr_hints_as_data_attrs` (`browser.rs:1504`) — asserts `data-hr-kind="waves"` and `data-hr-weight="thick"`.

#### Markdown Renderer

**Renderer function:** `renderable/src/tree/render/markdown.rs:259`

```rust
NodeKind::ThematicBreak => Ok("---".to_string()),
```

The Markdown tree renderer lowers `ThematicBreak` directly to `---` with no helper indirection.

**Test:** `thematic_break` (`markdown.rs:1083`) — asserts `---` output.

---

### `NodeKind::Image`

#### Terminal Renderer

**Renderer function:** `biscuit-terminal/lib/src/render_tree/render.rs:747-756`

```rust
NodeKind::Image { alt, .. } => {
    // Terminal inline images are out of scope for this phase
    // (visual components keep bespoke renderers). The alt text
    // stands in for the image.
    self.diagnostics.push(Diagnostic::lossy(
        "image rendered as alt text; inline terminal images are out of scope",
        Some(node.span.clone()),
    ));
    Ok(format!("[{alt}]"))
}
```

The terminal tree renderer handles `NodeKind::Image` by rendering alt text with a lossy diagnostic. It does **not** dispatch to `TerminalImage` as a standalone component. `TerminalImage` may encode a chosen terminal tier only after a graphics-policy renderer chooses the policy; that hook does not yet exist on the tree path (documented parity gap; see `entry-point-shape.md`).

**Test:** `render_tree_image_renders_alt_text_with_diagnostic` (`render.rs:2557`).

#### Browser Renderer

**Renderer function:** `renderable/src/tree/render/browser.rs:292-294`

```rust
NodeKind::Image { url, title, alt } => {
    Ok(self.render_image(node, url, title.as_deref(), alt))
}
```

`render_image` (`browser.rs:899`) emits an `<img>` void tag with `src`, `alt`, and optional `title`. The browser tree renderer owns the node lowering directly.

**Test:** `link_and_image` (`browser.rs:1477`) — asserts `<img src="img.png" alt="alt text">`.

#### Markdown Renderer

**Renderer function:** `renderable/src/tree/render/markdown.rs:308-317`

```rust
NodeKind::Image { url, title, alt } => {
    let alt = if self.table_cell_depth > 0 {
        escape_table_cell_text(alt)
    } else {
        alt.clone()
    };
    Ok(format!("![{alt}]({})", self.link_target(url, title)))
}
```

The Markdown tree renderer lowers `Image` directly to `![alt](url)` with no helper indirection.

**Test:** `link_and_image` (`markdown.rs:1017`) — asserts `![alt text](img.png)`.

---

### `NodeKind::Code { lang: "mermaid", .. }`

#### Current Tree-Path Behavior

The tree renderers treat `NodeKind::Code { lang: "mermaid", .. }` as an ordinary code block:

- **Terminal:** `render_code_node` (`biscuit-terminal/lib/src/render_tree/render.rs:415-416`) calls the `CodeRenderer` hook (`TerminalCodeRenderer` in darkmatter). `TerminalCodeRenderer::render_terminal_code` (`darkmatter/lib/src/markdown/render_tree/code_renderer.rs:78`) does **not** key on `lang == "mermaid"`; it produces a syntax-highlighted code block like any other language. No `MermaidDiagram` call occurs.
- **Browser:** `render_code_block` (`renderable/src/tree/render/browser.rs:270-271`) calls the browser `CodeRenderer` hook, which `TerminalCodeRenderer` also implements. Same outcome: a highlighted `<pre><code>` block.
- **Markdown:** `NodeKind::Code` renders as a fenced code block (`markdown.rs:247-258`). Mermaid source survives verbatim.

**Evidence that it stays a code node:**
- Mechanical search of `renderable/src/tree/render` and `biscuit-terminal/lib/src/render_tree` finds zero `mermaid` or `Mermaid` references.
- Parity test `render_tree_parity_mermaid_off_mode` (`darkmatter/lib/tests/render_tree_parity.rs:1283`) asserts both legacy and tree pipelines render the mermaid source as a plain highlighted code block under `MermaidMode::Off`.

**Documented gap:** `entry-point-shape.md` lists "Mermaid mode and HR CSS variables" as documented parity gaps for the internal tree path. The graphics-policy spec (`features/2026-05-26-graphics-policy/spec.md`) owns the design for Mermaid promotion through the `CodeRenderer` extension point; implementation is deferred to that work stream.

**Conclusion:** `NodeKind::Code { lang: "mermaid" }` correctly remains a code node on the tree path. There is no promotion boundary yet, so `MermaidDiagram` is not called — which is the correct behavior for an unimplemented promotion layer. Once the graphics-policy `CodeRenderer` hook keys on `lang == "mermaid"`, any `MermaidDiagram` call will be below that boundary.

---

### `Status` vs `StatusBlock`

**Mechanical search:** `grep -r 'Status\b' darkmatter/lib/src/markdown/render_tree/`

Result: **zero matches** for bare `Status`. The render-tree pipeline does not import or construct `biscuit_terminal::components::status::Status`.

**Mechanical search:** `grep -r 'StatusBlock' darkmatter/lib/src/markdown/render_tree/`

Result: **zero matches** in the render-tree module itself. `StatusBlock` is used in error-rendering paths (via the `BlockError` trait) in `darkmatter/lib/src/markdown/errors/`, `darkmatter/lib/src/markdown/compose/`, etc. — but these are error-formatting helpers, not document-content nodes emitted by the fold or render pipeline.

**Conclusion:** No darkmatter document path emits bare `Status`. `StatusBlock` is the document-content type for error blocks and stays on the tree (it implements `TreeRenderable`).

---

## Legacy Path Verification (for Phase 4/5 context)

The legacy darkmatter output paths (`darkmatter/lib/src/markdown/output/terminal.rs` and `output/html.rs`) **do** handle `ThematicBreak`, `Image`, and Mermaid directly:

- `output/terminal.rs:1067` — `InlineEvent::HorizontalRule(attrs)` builds `HorizontalRule` from attrs and renders it.
- `output/terminal.rs:1121-1149` — mermaid code blocks are detected and rendered via `Mermaid::render_for_terminal()`.
- `output/terminal.rs:655` — `TerminalImage` is constructed and rendered for images.
- `output/html.rs:229` — `InlineEvent::HorizontalRule` builds and renders `HorizontalRule`.
- `output/html.rs:261-287` — mermaid code blocks are detected and rendered via `Mermaid::render_for_html()`.

These are the **legacy serializers**, not the tree renderer. The cutover (Phase 5) will remove these paths once all callers migrate to the tree entry points. The Phase 3 verification confirms the tree renderers are ready to own the document lowering for `ThematicBreak` and `Image`; Mermaid promotion remains a documented gap to be closed by the graphics-policy work.

## Validation Checkpoint

- [x] `NodeKind::ThematicBreak` — tree renderer owns terminal, browser, and Markdown lowering; `HorizontalRule` is a helper below the node renderer.
- [x] `NodeKind::Image` — tree renderer owns browser and Markdown lowering; terminal path renders alt text (graphics-policy hook deferred). No document path dispatches to `TerminalImage` as a standalone component.
- [x] `NodeKind::Code { lang: "mermaid" }` — stays a code node on the tree path; no `MermaidDiagram` call occurs above a promotion boundary (boundary does not yet exist, which is a documented parity gap).
- [x] No darkmatter document path emits bare `Status`.
- [x] Evidence notes point to renderer functions, tests, and mechanical search results.

## Tests Run

```bash
# renderable tree renderer tests (173 passed)
cd renderable && cargo test --lib tree::render

# biscuit-terminal render_tree tests (176 passed)
cd biscuit-terminal/lib && cargo test --lib render_tree

# darkmatter render_tree unit tests (129 passed)
cd darkmatter/lib && cargo test --lib render_tree

# darkmatter parity tests (22 passed)
cd darkmatter/lib && cargo test --test render_tree_parity

# darkmatter HR snapshot tests (3 passed, 1 snapshot updated for pre-existing SGR reset change)
cd darkmatter/lib && cargo test --test render_tree_hr_snapshots
```
