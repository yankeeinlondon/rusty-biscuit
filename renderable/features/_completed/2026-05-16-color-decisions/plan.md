# Plan: CodeRenderer Terminal Color Context

## Overview

Widen `CodeRenderer::render_terminal_code` from `width: u32` to `TerminalCodeContext` (width + color_depth + color_mode) in `renderable`, with `biscuit-terminal` providing boundary `From` conversions and populating the context at the call site. This is a breaking trait change that must land before darkmatter implements `CodeRenderer`.

## Prerequisites

- [ ] Confirm no other crate outside `renderable` and `biscuit-terminal` implements `CodeRenderer` (grep for `impl CodeRenderer`)
- [ ] Review current `renderable::color` module structure to determine where new capability descriptors fit

## Phase 1: renderable — New Types

**Goal:** Define `ColorDepth`, `ColorMode`, and `TerminalCodeContext` in `renderable`.

1. **Create `renderable/src/color/capability.rs`**
   - Define `ColorDepth` enum with variants: `None`, `Minimal`, `Basic`, `Enhanced`, `TrueColor`
   - Derive `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`
   - Add serde derives if consistent with `renderable::color` module conventions
   - Document per rustdoc convention (no H1, H2 sections: `Examples`, `Notes`)
   
2. **Add `ColorMode` enum to same file**
   - Variants: `Light`, `Dark`, `Unknown`
   - Same derives as `ColorDepth`
   - Document `Unknown` semantics per D-6

3. **Define `TerminalCodeContext` struct**
   - Fields: `width: u32`, `color_depth: ColorDepth`, `color_mode: ColorMode`
   - Derive `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`
   - Provide `new(width, color_depth, color_mode)` constructor
   - Document as terminal capability context for code rendering

4. **Update `renderable/src/color/mod.rs`**
   - Add `pub mod capability;` or equivalent
   - Re-export `ColorDepth`, `ColorMode`, `TerminalCodeContext`
   - Update module docs per QR-4: clarify these are **capability descriptors**, distinct from color *value* types, and that ANSI emission lives in `biscuit-terminal`

5. **Update `renderable/src/tree/mod.rs`**
   - Re-export `TerminalCodeContext` and the new color types for tree-render consumer convenience

## Phase 2: renderable — Trait Update

**Goal:** Widen `CodeRenderer::render_terminal_code` signature.

1. **Update `renderable/src/tree/render/mod.rs`**
   - Change `render_terminal_code` signature: replace `width: u32` with `context: TerminalCodeContext`
   - Update architecture note doc comment to describe `TerminalCodeContext` rationale
   - Add no-color contract documentation (FR-10): 
     - Implementors SHOULD treat `ColorDepth::None` as "emit no ANSI styling"
     - If unable to honor supplied context, SHOULD return `None`
     - Implementors MUST NOT run ambient capability detection

2. **Verify `render_browser_code` is untouched** (FR-7)

## Phase 3: biscuit-terminal — Boundary Conversions

**Goal:** Implement `From` conversions and update call site.

1. **Add `From` impls in `biscuit-terminal/src/discovery/detection/color.rs`**
   - `impl From<discovery::detection::ColorDepth> for renderable::color::ColorDepth`
   - `impl From<discovery::detection::ColorMode> for renderable::color::ColorMode`
   - Map variants 1:1 per D-7 and D-8
   - Note: Use `renderable::color::ColorDepth as RenderColorDepth` alias if name collision

2. **Update `biscuit-terminal/src/render_tree/render.rs`**
   - Locate `Writer::render_code_node` (line ~629)
   - Build `TerminalCodeContext` from `TerminalRenderContext`:
     - `width`: use `context.available_width` (NOT root `width`) per FR-3
     - `color_depth`: map via `From` impl
     - `color_mode`: map via `From` impl
   - Pass `TerminalCodeContext` to `render_terminal_code`
   - No ambient re-detection

## Phase 4: Tests & Compatibility

**Goal:** Update test stub, add pass-through tests, ensure all suites green.

1. **Update Phase 3 test stub** (CR-1)
   - Find test stub `CodeRenderer` in `biscuit-terminal` tree-render tests
   - Update `render_terminal_code` signature to accept `TerminalCodeContext`
   - Ensure stub compiles

2. **Add pass-through tests** (§9 acceptance criteria)
   - Test (a): `available_width` vs root width — build nested/indented context where they differ, assert stub receives `available_width`
   - Test (b): `ColorDepth` pass-through — manually build `TerminalRenderContext` with specific `ColorDepth`, assert stub receives mapped value
   - Test (c): `ColorMode` pass-through including `Unknown` — same pattern
   - Test (d): No ambient influence — set conflicting env vars (`COLORTERM`, `NO_COLOR`, etc.) while providing explicit context, assert stub receives context values

3. **Run test suites**
   - `cargo test -p renderable`
   - `cargo test -p biscuit-terminal`
   - `cargo test -p darkmatter` (verify `yaml_block_parity` and Group 1 parity suites)
   - Fix any failures

4. **Lint check**
   - `cargo clippy --all-targets -p renderable`
   - `cargo clippy --all-targets -p biscuit-terminal`
   - `cargo clippy --all-targets -p darkmatter`
   - Address all warnings

## Phase 5: Documentation & Sign-off

**Goal:** Complete documentation requirements and verify acceptance criteria.

1. **Verify rustdoc for new types** (QR-2)
   - `ColorDepth` / `ColorMode`: examples, notes, derive list
   - `TerminalCodeContext`: constructor documented

2. **Verify module docs** (QR-4)
   - `renderable::color` module doc clarifies capability descriptors vs value types

3. **Verify trait docs** (QR-3)
   - Architecture note updated
   - No-color contract present
   - No ambient detection mandate present

4. **Final acceptance check against §9 criteria**
   - [ ] `renderable::color` defines `ColorDepth` and `ColorMode`
   - [ ] `TerminalCodeContext` defined with `new()` constructor
   - [ ] `CodeRenderer::render_terminal_code` takes `TerminalCodeContext`
   - [ ] `biscuit-terminal` has `From` impls
   - [ ] `render_code_node` populates context correctly
   - [ ] Pass-through tests cover (a)-(d)
   - [ ] `render_browser_code` and plain renderer unchanged
   - [ ] Docs complete
   - [ ] Test stub compiles
   - [ ] All crates build, test green, clippy-clean

## Risk Mitigation

- **Breaking change surface:** Minimal — only test stub implements trait today. Confirm with `grep -r "impl CodeRenderer"` across workspace before starting.
- **Name collisions:** `ColorDepth`/`ColorMode` exist in `biscuit-terminal` and `darkmatter`. Use use-site aliasing (D-9); verify with compiler.
- **Test regressions:** Run full darkmatter parity suites early in Phase 4 to catch issues.

## Dependencies

- No external dependencies beyond existing workspace crates.
- Must complete before darkmatter tree-rendering migration begins.

## Estimation

- Phase 1: 1-2 hours
- Phase 2: 30 minutes
- Phase 3: 1-2 hours
- Phase 4: 2-3 hours (tests + debugging)
- Phase 5: 30 minutes
- **Total: 5-8 hours**
