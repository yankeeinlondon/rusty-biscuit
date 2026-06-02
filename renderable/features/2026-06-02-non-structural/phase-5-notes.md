---
phase: 5
completed: 2026-06-02
---

# Phase 5 Implementation Notes: Cutover Readiness Validation

## Scope of This Verification

This validates the **node-kind builder/helper exemption** condition from the
spec: that the tree node renderers and `pub(crate)` tree entry points own the
document lowering for `Image`, `ThematicBreak`, and `Code{mermaid}`, and that
every exempt-helper call sits below the node renderer (helper calls are fine).

For `Code{mermaid}`, "own the lowering" currently means rendering it as a plain
highlighted code block: the tree path has **no** `lang == "mermaid"` branch and
makes **no** `MermaidDiagram` call. The promotion boundary that would lower a
mermaid fence to a diagram (and call `MermaidDiagram` below it) does not yet
exist; it is owned by the graphics-policy spec and tracked as ⏳ Pending in the
Phase 4/5 Checklist Sign-Off below. The exemption holds either way — the helper
is not reached by any document-pipeline route — but the promoted behavior is
unverified because it is unimplemented.

It does **not** verify the public document-pipeline cutover. The public APIs
`Markdown::as_html`, `Markdown::as_terminal`, and `as_terminal_with_layout`
still delegate to the legacy serializers (`output::as_html`,
`output::for_terminal`, `output::terminal::for_terminal_with_layout`), and
`DarkmatterPage::render` still calls `as_terminal_with_layout`. Routing those
public APIs through the tree entry points is **tree-cutover condition #1** and
remains **pending**. The mechanical-search "no remaining document-pipeline
route calls the legacy serializers" state is only reached *after* that cutover
deletes the serializers — see the spec's Verification Condition, which is
explicitly framed "before bespoke deletion (cutover Phase 5)."

## Success Criteria

- [x] Mechanical searches confirm the tree entry points and node renderers do not call the legacy `RuleProcessor`, `output/html.rs`, `output/terminal.rs`, or exempt helper components directly for `Image`, `ThematicBreak`, or Mermaid code nodes; every exempt-helper call sits below the node renderer or in a legacy path scheduled for deletion. (The public `Markdown::as_html` / `as_terminal` / `as_terminal_with_layout` pipeline still routes through the legacy serializers — that public cutover is tree-cutover condition #1 and remains pending. See [Scope](#scope-of-this-verification).)
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

**Conclusion:** `RuleProcessor` is reached only from the legacy serializers. No
*tree* route constructs it; the tree fold uses `BlockExtensionProcessor`
instead. The legacy serializers that do call it are still the active public
document-pipeline routes (`Markdown::as_html` / `as_terminal`) and are
scheduled for deletion at tree-cutover Phase 5 once condition #1 lands.

### Legacy Output Serializers

Searched for `output/html.rs` and `output/terminal.rs` usage.

**Findings:**
- Both files exist under `darkmatter/lib/src/markdown/output/`.
- They are still the active public document-pipeline routes: `Markdown::as_html` delegates to `output::as_html` (`darkmatter/lib/src/markdown/mod.rs:595`), `Markdown::as_terminal` to `output::for_terminal` (`:620`), and `as_terminal_with_layout` to `output::terminal::for_terminal_with_layout` (`:626`); `DarkmatterPage::render` calls `as_terminal_with_layout` (`darkmatter/lib/src/layout/page.rs:867`).
- They are **not** referenced by any tree-renderer code path.
- The tree entry points (`render_tree_html`, `render_tree_terminal`, `render_tree_markdown`) live in `darkmatter/lib/src/markdown/render_tree/entrypoints.rs`, are `pub(crate)` and reached only from tests/benchmarks, and do not call the legacy serializers.

**Conclusion:** The legacy serializers remain the active public document-pipeline routes and are scheduled for deletion at tree-cutover Phase 5, once condition #1 routes the public APIs through the tree entry points. The tree entry points themselves are verified clean of legacy-serializer calls; the public cutover that retires the serializers is pending (owned by the tree-cutover spec).

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
| `NodeKind::Code { lang: "mermaid" }` is rendered as a plain highlighted code block by the tree; no `MermaidDiagram` dispatch occurs | ✅ | `darkmatter/lib/src/markdown/render_tree/code_renderer.rs:77` (no `lang == "mermaid"` branch), `renderable/src/tree/render/browser.rs:270`, `renderable/src/tree/render/markdown.rs:247` |
| Mermaid promotion boundary (`Code{mermaid}` → diagram per `MermaidMode`/`GraphicsMode`, with any `MermaidDiagram` call below it) exists and is tested for the promoted `Text`/`Image` modes | ⏳ Pending | Boundary does not yet exist (`phase-5-notes.md` mechanical search; `code_renderer.rs:77` has no Mermaid branch). Owned by graphics-policy spec (`../2026-05-26-graphics-policy/spec.md`). Only `MermaidMode::Off` parity is covered (`darkmatter/lib/tests/render_tree_parity.rs:1283`) |
| No *tree* route calls legacy `RuleProcessor`, `output/html.rs`, `output/terminal.rs`, or helpers directly; every exempt-helper call sits below the node renderer | ✅ | Mechanical searches (see above) |
| Public `Markdown::as_html` / `as_terminal` / `as_terminal_with_layout` route through the tree (retiring the legacy serializers) | ⏳ Pending | Owned by tree-cutover condition #1; public APIs still delegate to `output::*` (`darkmatter/lib/src/markdown/mod.rs:595,620,626`) |
| No darkmatter document path emits bare `Status` | ✅ | `StatusBlock` is the document type; mechanical search finds no bare `Status` emission |

## Remaining Cutover Blockers

None from the non-structural exemption verification. The registered non-structural components are **not** blockers for the tree cutover.

Separate cutover blockers (owned by sibling specs):
- Graphics-policy implementation (Phase 0a of tree-cutover spec) — includes the
  Mermaid promotion boundary (`Code{mermaid}` → diagram per `MermaidMode`/
  `GraphicsMode`). Until it lands the tree renders mermaid as a plain code block
  and the promoted `Text`/`Image` behavior is unverified.
- Browser performance hotspot (`large_table` ≈ 11× slower — owned by perf-gate spec).
- `Prose` collapse onto shared tree (owned by prose-tree spec).
- `FileSystem` terminal flip (owned by tree-cutover Phase 3).
- `YamlBlock` default flip (owned by tree-cutover Phase 3).

## Validation Checkpoint

✅ The cutover can consume the non-structural spec without treating the registered exempt components as blockers.
✅ All document-pipeline components remain either tree-render-only or explicitly tracked for cutover.
✅ Every exempt helper call is either below the node renderer or in a legacy path scheduled for deletion.
✅ The tree node renderers and `pub(crate)` entry points own the document lowering for `Image` and `ThematicBreak` (helper calls below the node renderer). `Code{mermaid}` currently lowers to a plain highlighted code block with no `MermaidDiagram` dispatch; the Mermaid promotion boundary that would call the rasterizer is **pending** (owned by the graphics-policy spec) — see the `⏳ Pending` row in the Phase 4/5 Checklist Sign-Off.

⏳ **Not verified here (out of scope):** the public document-pipeline cutover. `Markdown::as_html` / `as_terminal` / `as_terminal_with_layout` and `DarkmatterPage::render` still route through the legacy serializers. Retiring those public routes is tree-cutover condition #1 and remains pending; only after it lands does the "no remaining document-pipeline route calls the legacy serializers" state hold for the public surface.
