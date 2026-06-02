---
phase: 5
completed: 2026-06-02
---

# Phase 5 Implementation Notes: Cutover Readiness Validation

## Success Criteria

- [x] Mechanical searches confirm no remaining document-pipeline route calls the legacy `RuleProcessor`, `output/html.rs`, `output/terminal.rs`, or exempt helper components directly for `Image`, `ThematicBreak`, or Mermaid code nodes.
- [x] Narrow package tests covering `renderable`, `biscuit-terminal`, and `darkmatter` renderer behavior pass.
- [x] Component-catalog and feature-spec documentation is up to date.
- [x] Tree-cutover Phase 4/5 checklist updated with exemption-register and node-kind verification status.
- [x] Final evidence recorded: changed docs, verified renderer paths, tests run, remaining blockers.
- [x] Validation checkpoint: cutover can consume this spec without treating registered non-structural components as blockers.

## Mechanical Search Results

### RuleProcessor

Searched `darkmatter`, `biscuit-terminal`, and `renderable` for `RuleProcessor`.

**Findings:**
- `RuleProcessor` is defined in `darkmatter/lib/src/markdown/block/rule_processor.rs` and exported from `darkmatter::markdown::block`.
- It is instantiated **only** in the legacy serializers:
  - `darkmatter/lib/src/markdown/output/terminal.rs:959`
  - `darkmatter/lib/src/markdown/output/html.rs:195`
- No caller in the **tree renderer** or **document-pipeline fold** constructs `RuleProcessor`.
- The render-tree fold uses `BlockExtensionProcessor` (span-aware) for HR attributes, not `RuleProcessor`.
- References in `renderable` and `biscuit-terminal` are limited to documentation, specs, and the `biscuit-terminal` skill file — no source usage.

**Conclusion:** `RuleProcessor` is a legacy-only path. No document-pipeline route bypasses the tree renderer to call it.

### Legacy Output Serializers

Searched for `output/html.rs` and `output/terminal.rs` usage.

**Findings:**
- Both files exist under `darkmatter/lib/src/markdown/output/`.
- They are referenced in documentation, specs, reviews, and the legacy render API (`Markdown::as_html`, `Markdown::for_terminal`).
- They are **not** referenced by any tree-renderer code path.
- The tree entry points (`render_tree_html`, `render_tree_terminal`, `render_tree_markdown`) live in `darkmatter/lib/src/markdown/render_tree/entrypoints.rs` and do not call the legacy serializers.

**Conclusion:** The legacy serializers are scheduled for deletion in the tree-cutover Phase 5. They are not active document-pipeline routes.

### Exempt Helper Components

Searched for direct calls to `TerminalImage`, `HorizontalRule`, and `MermaidDiagram` from document-pipeline code.

**Findings:**
- **`TerminalImage`**: Constructed only in `darkmatter/lib/src/markdown/output/terminal.rs:655` (legacy serializer) and in `biscuit-terminal` component tests. The tree renderer's `NodeKind::Image` arm renders alt text directly; no `TerminalImage` dispatch.
- **`HorizontalRule`**: Constructed only in:
  - `biscuit-terminal/lib/src/render_tree/render.rs:1440` — private helper `horizontal_rule_from_attrs` called from the tree renderer's `ThematicBreak` arm.
  - `darkmatter/lib/src/markdown/output/terminal.rs` and `output/html.rs` — legacy serializers.
  - Component tests in `biscuit-terminal/lib/src/components/horizontal_rule/mod.rs`.
- **`MermaidDiagram`**: Called only in:
  - `darkmatter/lib/src/mermaid/render_terminal.rs` — legacy Mermaid terminal wrapper.
  - `biscuit-terminal/lib/src/components/mermaid.rs` — component implementation.
  - `worktree/cli/src/commands/list.rs` — standalone CLI command (not document pipeline).
  - Tests and documentation.
  The tree renderer does **not** key on `lang == "mermaid"` in the `CodeRenderer` hook; the promotion boundary does not yet exist.

**Conclusion:** No exempt helper is called directly from an active document-pipeline route bypassing the tree renderer. All helper usage is either below the node renderer or in legacy paths scheduled for deletion.

### Status vs StatusBlock

Searched for bare `Status` emission from darkmatter document paths.

**Findings:**
- No darkmatter document path emits bare `Status`.
- `StatusBlock` is the document-content type and implements `TreeRenderable`.

**Conclusion:** Verified — no remediation needed.

## Tests Run

### renderable
```
cargo test -p renderable
```
Result: **362 passed, 0 failed**

### biscuit-terminal (library)
```
cargo test --lib render_tree
```
Result: **176 passed, 0 failed**

### darkmatter (library)
```
cargo test --lib render_tree
```
Result: **129 passed, 0 failed**

### darkmatter parity tests
```
cargo test --test render_tree_parity
```
Result: **22 passed, 0 failed**

### darkmatter HR snapshot tests
```
cargo test --test render_tree_hr_snapshots
```
Result: **3 passed, 0 failed**

### Doctests
```
cargo test --doc -p renderable -p biscuit-terminal -p darkmatter
```
Result: **80 passed, 0 failed, 2 ignored**

## Lints

```bash
just lint  # run in renderable, biscuit-terminal, and darkmatter package areas
```
Result: **All passed**

## Documentation Status

- `renderable/docs/components.md` — updated in Phase 2 with IR-state annotations for all exempt components.
- `renderable/features/2026-06-02-non-structural/spec.md` — Exemption Register and Verification Condition are current.
- `renderable/features/2026-06-02-tree-cutover/spec.md` — Decision #5 updated with verification-completed reference.

## Phase 4/5 Checklist Sign-Off

| Assertion | Status | Evidence |
|---|---|---|
| `NodeKind::ThematicBreak` renders terminal/browser/markdown from `render_*_node` / `render_*_document`; `HorizontalRule` is a helper below the node renderer | ✅ | `biscuit-terminal/lib/src/render_tree/render.rs:418-421`, `renderable/src/tree/render/browser.rs:273`, `renderable/src/tree/render/markdown.rs:259` |
| `NodeKind::Image` renders through the tree renderer's graphics-policy path; `TerminalImage` is not dispatched as a standalone component | ✅ | `biscuit-terminal/lib/src/render_tree/render.rs:747-756`, `renderable/src/tree/render/browser.rs:292-294`, `renderable/src/tree/render/markdown.rs:308-317` |
| `NodeKind::Code { lang: "mermaid" }` stays a code node until promotion; `MermaidDiagram` is below the boundary | ✅ | `darkmatter/lib/src/markdown/render_tree/code_renderer.rs:78`, `renderable/src/tree/render/browser.rs:270`, `renderable/src/tree/render/markdown.rs:247` |
| No document-pipeline route calls legacy `RuleProcessor`, `output/html.rs`, `output/terminal.rs`, or helpers directly | ✅ | Mechanical searches (see above) |
| No darkmatter document path emits bare `Status` | ✅ | `StatusBlock` is the document type; mechanical search finds no bare `Status` emission |

## Remaining Cutover Blockers

None from the non-structural exemption verification. The registered non-structural components are **not** blockers for the tree cutover.

Separate cutover blockers (owned by sibling specs):
- Graphics-policy implementation (Phase 0a of tree-cutover spec).
- Browser performance hotspot (`large_table` ≈ 11× slower — owned by perf-gate spec).
- `Prose` collapse onto shared tree (owned by prose-tree spec).
- `FileSystem` terminal flip (owned by tree-cutover Phase 3).
- `YamlBlock` default flip (owned by tree-cutover Phase 3).

## Validation Checkpoint

✅ The cutover can consume the non-structural spec without treating the registered exempt components as blockers.
✅ All document-pipeline components remain either tree-render-only or explicitly tracked for cutover.
✅ Every exempt helper call is either below the node renderer or in a legacy path scheduled for deletion.
