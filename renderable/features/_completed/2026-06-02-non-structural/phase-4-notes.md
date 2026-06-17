---
phase: 4
completed: 2026-06-02
---

# Phase 4 Implementation Notes: Remediation and Focused Tests

## Success Criteria

- [x] All remediation tasks either completed with tests or explicitly marked not needed with evidence from Phase 3.
- [x] Focused tests exist for all verified tree-renderer paths.
- [x] Comments and docs updated for any changed behavior.
- [x] Validation checkpoint passed.

## Remediation Assessment

### ThematicBreak Tree Renderer Path

**Status: No remediation needed.**

Phase 3 verified that `NodeKind::ThematicBreak` renders through the tree renderer in all three targets:

- **Terminal:** `biscuit-terminal/lib/src/render_tree/render.rs:418-421` calls `horizontal_rule_from_attrs` (a private helper) which constructs `HorizontalRule` from node attributes. `HorizontalRule` is a pure helper call beneath the node renderer.
- **Browser:** `renderable/src/tree/render/browser.rs:273` calls `render_thematic_break` directly; no helper indirection.
- **Markdown:** `renderable/src/tree/render/markdown.rs:259` lowers directly to `---`.

No document path bypasses the tree renderer. The legacy `output/terminal.rs` and `output/html.rs` serializers handle `ThematicBreak` directly, but these are legacy paths scheduled for removal in Phase 5 of the cutover, not active tree-renderer bypasses.

### Image Tree Renderer Path

**Status: No remediation needed.**

Phase 3 verified that `NodeKind::Image` renders through the tree renderer:

- **Terminal:** `biscuit-terminal/lib/src/render_tree/render.rs:747-756` renders alt text with a lossy diagnostic. It does **not** dispatch to `TerminalImage` as a standalone component. The graphics-policy hook for terminal inline images is a documented parity gap (see `entry-point-shape.md`).
- **Browser:** `renderable/src/tree/render/browser.rs:292-294` calls `render_image` directly.
- **Markdown:** `renderable/src/tree/render/markdown.rs:308-317` lowers directly to `![alt](url)`.

No document path dispatches to `TerminalImage` as a standalone component.

### Mermaid Code Block Tree Renderer Path

**Status: No remediation needed.**

Phase 3 verified that `NodeKind::Code { lang: "mermaid", .. }` stays a code node on the tree path:

- **Terminal:** `render_code_node` calls the `TerminalCodeRenderer` hook, which does not key on `lang == "mermaid"`.
- **Browser:** `render_code_block` calls the browser `CodeRenderer` hook with the same behavior.
- **Markdown:** Renders as a fenced code block with Mermaid source verbatim.

No `MermaidDiagram` call exists above a promotion boundary because the promotion boundary does not yet exist. This is a documented parity gap owned by the graphics-policy spec (`features/2026-05-26-graphics-policy/spec.md`). Once the `CodeRenderer` extension point keys on `lang == "mermaid"`, any `MermaidDiagram` call will be below that boundary.

### Status vs StatusBlock

**Status: No remediation needed.**

Phase 3 mechanically confirmed no darkmatter document path emits bare `Status`. `StatusBlock` is the document-content type and stays on the tree (it implements `TreeRenderable`).

## Code Changes

The only code change in Phase 4 was fixing a pre-existing clippy lint error:

- `biscuit-terminal/lib/src/render_tree/render.rs:1631-1632` — changed `let mut style = Style::default(); style.emphasis = emphasis;` to `let style = Style { emphasis, ..Default::default() };` to satisfy `clippy::field_reassign_with_default`.

This change is purely structural and does not affect behavior.

## Tests

All existing tests continue to pass, covering the verified tree-renderer paths:

### Terminal Tests (biscuit-terminal)
- `render_tree_thematic_break_renders_rule` — basic HR rendering
- `render_tree_thematic_break_consumes_darkmatter_hr_hints` — HR with `darkmatter.hr.*` hints
- `render_tree_thematic_break_without_hints_uses_default_rule` — default HR path
- `render_tree_image_renders_alt_text_with_diagnostic` — image alt-text fallback

### Browser Tests (renderable)
- `thematic_break_and_breaks` — `<hr>` output
- `thematic_break_surfaces_darkmatter_hr_hints_as_data_attrs` — `data-hr-*` attributes
- `link_and_image` — `<img>` tag output

### Markdown Tests (renderable)
- `thematic_break` — `---` output
- `link_and_image` — `![alt](url)` output

### Parity Tests (darkmatter)
- `render_tree_parity_hr_attributes_spanned` — HR attributes in both pipelines
- `render_tree_parity_mermaid_off_mode` — mermaid as code block in both pipelines
- `render_tree_parity_links_images` — image rendering parity

### HR Snapshot Tests (darkmatter)
- `render_tree_hr_markdown_snapshots`
- `render_tree_hr_terminal_snapshots`
- `render_tree_hr_html_snapshots`

## Validation Checkpoint

- [x] `NodeKind::ThematicBreak` — tree renderer owns all three targets; `HorizontalRule` is a helper below the node renderer. No remediation needed.
- [x] `NodeKind::Image` — tree renderer owns browser and Markdown lowering; terminal alt-text is a documented gap. No `TerminalImage` standalone dispatch. No remediation needed.
- [x] `NodeKind::Code { lang: "mermaid" }` — stays a code node on the tree path; promotion boundary does not yet exist (documented gap). No `MermaidDiagram` call above boundary. No remediation needed.
- [x] No darkmatter document path emits bare `Status`. No remediation needed.
- [x] All tests pass (renderable: 173, biscuit-terminal: 176, darkmatter: 129, parity: 22, HR snapshots: 3).
- [x] All lints pass (`just lint` for all three package areas).

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

# darkmatter HR snapshot tests (3 passed)
cd darkmatter/lib && cargo test --test render_tree_hr_snapshots

# lints (all passed)
cd renderable && just lint
cd biscuit-terminal && just lint
cd darkmatter && just lint
```
