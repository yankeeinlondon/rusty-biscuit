---
ready: false
agent: codex
model: ""
---

# Review - Iteration 2

## Findings

### High - Mermaid helper exemption is signed off even though the tree promotion boundary does not exist

The non-structural spec makes the helper exemption conditional: `NodeKind::Code { lang: "mermaid", .. }` must stay as a code node only until a Mermaid-aware renderer promotes it according to `MermaidMode` and `GraphicsMode`, with any `MermaidDiagram` call below that boundary (`renderable/features/2026-06-02-non-structural/spec.md:151`). It also requires confirming before bespoke deletion that the tree renderer produces the `Code{mermaid}` document output self-contained (`spec.md:136`).

The implementation documents the opposite state as complete. `phase-5-notes.md:83` says the tree renderer does not key on `lang == "mermaid"` and that the promotion boundary does not exist, but the checklist still marks the Mermaid assertion complete at `phase-5-notes.md:154`. The Phase 3 notes make the same point explicitly: Mermaid is rendered as a normal highlighted code block, and promotion is a documented gap (`phase-3-notes.md:134`, `phase-3-notes.md:144`). The code matches that: `TerminalCodeRenderer::render_terminal_code` and `render_browser_code` reconstruct code metadata and call the normal syntax-highlighting renderers without branching on `lang == "mermaid"` (`darkmatter/lib/src/markdown/render_tree/code_renderer.rs:77`, `darkmatter/lib/src/markdown/render_tree/code_renderer.rs:157`). The only parity test covers `MermaidMode::Off`, and its comments state `Text` and `Image` are not covered because the experimental tree path has no Mermaid adapter (`darkmatter/lib/tests/render_tree_parity.rs:1271`).

That is a blocker for production readiness of this verification. Either implement the Mermaid-aware `CodeRenderer` promotion from the graphics-policy plan (`renderable/features/2026-05-26-graphics-policy/plan.md:23`) and add tests for the promoted modes, or narrow the non-structural sign-off so `Code{mermaid}` remains pending rather than checked off. As written, the catalog also overstates the current state by saying the tree calls `MermaidDiagram` as its rasterizer (`renderable/docs/components.md:131`).

Verification level: current strongest coverage is Level 1/static inspection plus an L1 parity test for `MermaidMode::Off` only. There is no verification for the user-observable promoted Mermaid behavior. Once promotion is implemented, terminal diagram rendering should have Level 2 coverage for the real terminal output path; OS keyboard injection is not relevant here.

## Test Coverage Notes

The iteration-1 public-pipeline routing issue has been corrected in the docs: `phase-5-notes.md` and `components.md` now explicitly say `Markdown::as_html`, `as_terminal`, `as_terminal_with_layout`, and `DarkmatterPage::render` still use the legacy serializers and that public cutover remains pending.

The remaining coverage gap is specifically Mermaid promotion. The existing `MermaidMode::Off` test is useful, but it does not verify the spec's promoted `Text` / `Image` behavior or the helper boundary.

## Readiness

Not ready for production. The exemption register is mostly coherent, but the Mermaid helper condition is marked complete before the required tree promotion path exists.
