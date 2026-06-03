---
phases: 4
created: 2026-06-02
start_phase: 1
source_files_during_phase_1:
  - renderable/src/tree/mod.rs
  - renderable/src/tree/render/browser.rs
  - biscuit-terminal/lib/src/render_tree/options.rs
  - darkmatter/lib/src/markdown/render_tree/entrypoints.rs
  - darkmatter/lib/benches/migration_parity.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - renderable/src/tree/graphics.rs
  - renderable/src/tree/mod.rs
  - renderable/src/tree/render/browser.rs
  - biscuit-terminal/lib/src/components/horizontal_rule/mod.rs
  - biscuit-terminal/lib/src/render_tree/render.rs
  - darkmatter/lib/tests/snapshots/render_tree_hr_snapshots__html_waves.snap
  - darkmatter/lib/tests/snapshots/render_tree_hr_snapshots__html_kind_waves.snap
  - darkmatter/lib/tests/snapshots/render_tree_hr_snapshots__html_all_attributes.snap
  - darkmatter/lib/tests/snapshots/render_tree_hr_snapshots__html_mark_dim_hr.snap
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - renderable/src/tree/render/browser.rs
  - biscuit-terminal/lib/src/components/mermaid.rs
  - biscuit-terminal/lib/src/render_tree/render.rs
  - biscuit-terminal/lib/tests/render_tree_code_context.rs
  - darkmatter/lib/src/markdown/render_tree/code_renderer.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
  - darkmatter/lib/src/markdown/render_tree/entrypoints.rs
  - darkmatter/lib/tests/render_tree_parity.rs
  - darkmatter/lib/benches/migration_parity.rs
  - biscuit-terminal/lib/src/render_tree/render.rs
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
packages:
  - renderable
  - biscuit-terminal
  - darkmatter
---

# Execution Plan: Graphics Policy

## Phase 1: Context and Policy Foundation
- [ ] Add `GraphicsMode` enum (`Off`, `Vector`, `Rich`) to `renderable::tree`.
- [ ] Add `BrowserMermaidMode` enum (`Code`, `StaticSvg`, `Interactive`) to `renderable::tree`.
- [ ] Add `graphics_mode` and `mermaid_mode` fields to `BrowserRenderOptions`, defaulting to `Rich` and `Code` respectively.
- [ ] Add `graphics_mode` and `force_graphics` fields to `TerminalRenderContext`, defaulting to `Rich` and `false` respectively.
- [ ] Update all construction sites (`from_terminal`, entry points, bench harness) to support the new fields.
- [ ] **Validation Checkpoint**: Compilation succeeds across the workspace. Behavior remains unchanged for existing callers.

## Phase 2: Horizontal Rule (HR) Lowering and SVG Fidelity
- [x] Refactor HR component to use lazy lowering: Move the rasterization decision out of `HorizontalRule::render` to the renderer. HR must honor `graphics_mode ∧ capability`.
- [x] Add a dependency-correct shared helper in `renderable` (e.g., `renderable::tree::graphics::horizontal_rule_svg`) for pure styled-HR SVG construction.
- [x] Wire the shared HR SVG helper into `render_thematic_break` (`renderable/src/tree/render/browser.rs`) for `GraphicsMode::Vector` and `Rich`. Ensure `GraphicsMode::Off` retains the plain `<hr>`.
- [x] **Validation Checkpoint**: Add parity coverage ensuring byte-parity tests against current `biscuit-terminal` browser HR SVG output pass.

## Phase 3: Mermaid Promotion via CodeRenderer
- [x] Implement adapter-owned Mermaid promotion using the Mermaid-aware `CodeRenderer` path.
- [x] Configure Terminal code renderer to rasterize Mermaid diagrams at the `Rich` tier.
- [x] Configure Browser code renderer to emit static `<svg>` at `Vector` and `Rich` tiers (or interactive if `mermaid_mode` requests it).
- [x] Ensure graceful degradation: Render the original code block when capped by `GraphicsMode::Off`, lacking opt-in/capability, or upon promotion failure.
- [x] [Parallelizable] Validate `Code` node metadata preservation. Ensure the inherited code-block meta (title, line-numbering, highlight) is fully preserved when promotion is disabled or fails.
- [x] **Validation Checkpoint**: Verify renderable core has NOT acquired `biscuit-visualized` or `biscuit-terminal` dependencies.

## Phase 4: Entry-point Mapping, TerminalImage Routing & Parity
- [x] Update `darkmatter` entry points to map legacy `TerminalImageMode` and `MermaidMode` to the new `GraphicsMode` and `BrowserMermaidMode`.
- [x] Fix the `image_mode`-dropping bug at the `TerminalOptions -> TerminalRenderOptions` mapping boundary.
- [x] Route `TerminalImage` component's alt-text vs. image choice through `graphics_mode` (`Off`/`Vector` -> alt text, `Rich` -> image protocol).
- [x] [Parallelizable] Ensure darkmatter frontmatter composition works as intended (e.g., `style: waves` drops graphic at `Off`, honors structure).
- [x] **Validation Checkpoint**: Execute `migration_parity` tests to verify:
  - `TerminalImageMode::Never` suppresses Mermaid `Image`.
  - `mark_dim_hr` avoids rasterization under `Off` mode.
  - Browser SVG output reaches parity with legacy rendering at `Rich` tier.
