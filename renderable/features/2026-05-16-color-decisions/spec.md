---
title: "CodeRenderer Terminal Color Context"
created: "2026-05-16"
status: draft
confidence: high
related:
  - renderable/features/2026-05-16-iterative-improvement/components-group1-plan.md
  - .ai/plans/2026-05-16.execution-of-components-group1.md
precursor_to: "Darkmatter tree-rendering migration"
---

# CodeRenderer Terminal Color Context

## 1. Summary

The `CodeRenderer` hook's terminal method, `render_terminal_code`, currently
receives only a `width: u32`. It must be widened to carry the terminal's
**color depth** and **color mode** so that a code-renderer implementation can
make the same capability-aware color decisions the rest of the tree renderer
makes — without breaking the crate dependency graph.

This is a small, additive change. It is specified as a **standalone
precursor** because it must land **before** darkmatter implements
`CodeRenderer`: widening the trait is free now and a breaking change to
darkmatter once darkmatter depends on the current signature.

## 2. Background & Context

### 2.1 The render tree and tree renderers

`renderable` defines a canonical, target-agnostic render tree (`RenderNode` /
`NodeKind`). Three tree renderers fold that tree into output:

- the **Markdown** and **Browser** renderers live in `renderable`
  (`renderable/src/tree/render/`),
- the **Terminal** renderer lives in `biscuit-terminal`
  (`biscuit-terminal/lib/src/render_tree/render.rs`).

The dependency direction is fixed: `biscuit-terminal` depends on `renderable`.
`renderable` must never depend on `biscuit-terminal`.

### 2.2 What `CodeRenderer` is

`CodeRenderer` (`renderable/src/tree/render/mod.rs`) is an optional hook trait.
A `NodeKind::Code` node renders, by default, as a plain dim/indented panel with
no syntax highlighting. `CodeRenderer` lets a caller inject richer rendering
(syntax highlighting, chrome) for code blocks:

```rust
pub trait CodeRenderer {
    fn render_terminal_code(
        &self,
        lang: Option<&str>,
        value: &str,
        attrs: &NodeAttrs,
        width: u32,
    ) -> Option<String>;

    fn render_browser_code(
        &self,
        lang: Option<&str>,
        value: &str,
        attrs: &NodeAttrs,
    ) -> Option<BrowserFragment<Ready>>;
}
```

`Some(_)` is used verbatim; `None` falls back to the built-in plain renderer.
The hook is carried as `Option<Rc<dyn CodeRenderer>>` on `TreeComponent` and
`TerminalRenderOptions`, and consulted by `Writer::render_code_node`
(`render.rs:629`).

### 2.3 How the hook came to take `width: u32`

`CodeRenderer` was introduced in Phase 3 of the Group 1 tree-rendering work
(see the related plan). The original plan sketch had `render_terminal_code`
take `&TerminalRenderContext`. That is **not possible**: `TerminalRenderContext`
is defined in `biscuit-terminal` and references `biscuit-terminal`-only types
(`ColorDepth`, `ColorMode`, `ImageSupport`, `Terminal`). A trait in
`renderable` cannot name it.

The Phase 3 resolution passed a primitive `width: u32`; the terminal renderer
extracts it from `context.available_width` at the call site. The trait's
doc-comment records this as a deliberate architecture note.

### 2.4 Current state

- The `CodeRenderer` **plumbing is complete**: the trait, the
  `Option<Rc<dyn CodeRenderer>>` slots on `TreeComponent` /
  `TerminalRenderOptions`, the `render_code_node` call site, and the browser
  hook all exist and are tested.
- There is **no production `CodeRenderer` implementation**. Phase 3 used a
  deterministic **test stub** only.
- `YamlBlock::render_tree_node()` projects to a `NodeKind::Code` node with
  `CodeRenderHints { header_row, language_label, highlight }`, but no real
  highlighting hook is installed in production.

So today the `width`-only signature has **zero practical impact**.

### 2.5 Why this matters now

The **next** body of work migrates darkmatter onto the tree renderer. As part
of that, darkmatter will supply a real `CodeRenderer` that performs syntax
highlighting (the capability `YamlBlock` and Markdown ``` ```yaml ``` fences
have today via bespoke rendering). At that point the `width`-only signature
becomes a constraint that must be resolved — and resolving it cleanly is
cheaper before darkmatter writes `impl CodeRenderer` than after.

## 3. Problem Statement

A syntax-highlighting `CodeRenderer` needs more than `width` to make correct,
**consistent** color decisions. Two concrete problems:

### 3.1 Consistency gap

Every other part of the terminal tree renderer honors the
`TerminalRenderContext` capability snapshot (color depth, color mode, OSC8,
image support, Unicode). A `width`-only code hook cannot. Its only options are:

- self-detect capabilities from the ambient environment
  (`COLORTERM`, `COLORFGBG`, `NO_COLOR`, terminfo), or
- assume a fixed capability set.

Self-detection diverges from the render context whenever the two disagree —
for example a test terminal, a forced light/dark mode, or a capability
override built via `Terminal::builder()`. The rest of the document would obey
the context; the code block would obey the ambient environment. That is a
latent correctness bug for tests and composed/non-interactive rendering.

### 3.2 Darkmatter genuinely needs color depth and color mode

This is not hypothetical. Darkmatter's existing code-block rendering already
depends on both:

- **Color mode** (light/dark) — `darkmatter::markdown::highlighting`
  defines its own `ColorMode` and a `detect_color_mode()`; `CodeHighlighter`
  is constructed with it to pick a syntect theme.
- **Color depth** — `darkmatter::markdown::output::terminal` defines its own
  `ColorDepth` (`None`, `Colors256`, `TrueColor`, …) with an `auto_detect()`,
  and `TerminalOptions.color_depth` drives downsampling of 24-bit SGR for
  256/16-color terminals.

A darkmatter `CodeRenderer` that receives only `width` would have to
re-derive both by ambient detection — re-introducing the consistency gap of
§3.1 for the one node kind that most needs faithful color handling.

### 3.3 Change-ordering

Widening `CodeRenderer::render_terminal_code` is a **breaking change** to the
trait. Performed **now**, no implementor exists outside a test stub, so the
cost is updating the stub and the single call site. Performed **after**
darkmatter ships `impl CodeRenderer`, it additionally forces a coordinated
edit to darkmatter. Doing it first is strictly cheaper.

## 4. Goals and Non-Goals

### 4.1 Goals

- Give `render_terminal_code` enough terminal capability context to make
  color decisions consistent with the rest of the tree renderer.
- Keep the `CodeRenderer` trait in `renderable`, with **no** dependency on
  `biscuit-terminal`.
- Make the change additive and complete **before** the darkmatter migration
  begins, so darkmatter implements against the final signature.

### 4.2 Non-Goals

- Migrating darkmatter onto the tree renderer (the subsequent body of work).
- Implementing a production darkmatter `CodeRenderer` (subsequent work).
- Changing `render_browser_code` — the browser target has no terminal
  capability concept; it stays as-is.
- Unifying the three separate color-capability enums
  (`biscuit-terminal::discovery::detection::{ColorDepth, ColorMode}`,
  `darkmatter`'s own `ColorDepth` / `ColorMode`) beyond what this hook needs.
  A full consolidation may be desirable later but is out of scope here.
- Adding OSC8 hyperlink or image-protocol information to the code hook — code
  blocks emit neither.

## 5. Requirements

### 5.1 Functional requirements

- **FR-1** — `renderable` MUST define terminal color-capability descriptor
  types usable by `CodeRenderer` without depending on `biscuit-terminal`.
  At minimum this covers **color depth** and **color mode**.
- **FR-2** — `CodeRenderer::render_terminal_code` MUST receive, in addition to
  `lang`, `value`, and `attrs`: the available render **width**, the terminal
  **color depth**, and the terminal **color mode**.
- **FR-3** — The terminal tree renderer (`biscuit-terminal`) MUST populate the
  new context by mapping from its `TerminalRenderContext`
  (`available_width`, `color_depth`, `color_mode`) at the `render_code_node`
  call site. No ambient re-detection in the renderer.
- **FR-4** — The mapping from `biscuit-terminal`'s `ColorDepth`
  (`None`, `Minimal`, `Basic`, `Enhanced`, `TrueColor`) and `ColorMode`
  (`Light`, `Dark`, `Unknown`) onto the `renderable` descriptors MUST be
  total and lossless enough that a code renderer can choose between, at least:
  no color, 16-color, 256-color, and true-color output, and between
  light / dark / unknown backgrounds.
- **FR-5** — `render_browser_code` MUST be unchanged.
- **FR-6** — The built-in plain code renderer
  (`Writer::render_code`) MUST be unaffected; it already renders through the
  full `TerminalRenderContext`.
- **FR-7** — The `width`-only behavior MUST NOT be retained as a second code
  path. There is one terminal code hook signature.

### 5.2 Compatibility and migration requirements

- **CR-1** — The Phase 3 test stub `CodeRenderer` MUST be updated to the new
  signature.
- **CR-2** — All existing tests in `renderable`, `biscuit-terminal`, and
  `darkmatter` MUST remain green. In particular the `yaml_block_parity` suite
  and Group 1 parity suites MUST stay green.
- **CR-3** — No behavior change for any current user: because no production
  `CodeRenderer` exists, the only observable change is the trait signature.

### 5.3 Quality requirements

- **QR-1** — `cargo clippy --all-targets` MUST be warning-free for
  `renderable`, `biscuit-terminal`, and `darkmatter`.
- **QR-2** — The new `renderable` descriptor types MUST be documented per the
  repo rustdoc convention (no H1 in `///`, `## Examples` etc.) and MUST
  derive the same standard traits as sibling color types
  (`Debug, Clone, PartialEq`, serde where consistent with the module).
- **QR-3** — The architecture note on the `CodeRenderer` trait MUST be updated
  to describe the new context type instead of the `width: u32` rationale.

## 6. Design Decisions

### 6.1 Settled

- **D-1 — The trait stays in `renderable`.** Homing `CodeRenderer` in
  `biscuit-terminal` would block the browser renderer (which lives in
  `renderable`) from sharing the trait. The dependency direction is not
  negotiable.
- **D-2 — The hook carries a context value, not the full
  `TerminalRenderContext`.** `renderable` cannot name that type; a purpose-built
  `renderable`-owned value is the only option that respects D-1.
- **D-3 — Only width + color depth + color mode are in scope.** Code blocks
  do not emit hyperlinks or images; Unicode support is already reflected in
  how `value` is projected. Adding unused fields is rejected (simplicity).

### 6.2 Recommended (pending confirmation)

- **D-4 — A small struct over loose primitives.** Pass a single
  `TerminalCodeContext` value rather than three positional arguments.
  Rationale: future-proof (new fields are additive without re-threading call
  sites), self-documenting, and consistent with how `renderable` already
  groups render configuration.

  Illustrative shape (final naming/placement subject to Open Questions):

  ```rust
  // renderable
  pub struct TerminalCodeContext {
      /// Available render width in columns.
      pub width: u32,
      /// Color depth the terminal advertises.
      pub color_depth: ColorDepth,
      /// Light / dark / unknown background.
      pub color_mode: ColorMode,
  }

  fn render_terminal_code(
      &self,
      lang: Option<&str>,
      value: &str,
      attrs: &NodeAttrs,
      context: &TerminalCodeContext,
  ) -> Option<String>;
  ```

- **D-5 — Color-capability descriptors live in `renderable::color`.**
  `renderable::color` already owns the color *value* system (`Color`,
  `WebColor`, `BasicColor`, RGB, HDR). Color *capability* descriptors
  (`ColorDepth`, `ColorMode`) are a natural fit there, and placing them in the
  lowest crate lets `biscuit-terminal` and `darkmatter` both map onto a single
  canonical pair instead of each crate keeping its own.

## 7. Open Questions

- **OQ-1 — Naming.** `ColorDepth` and `ColorMode` already exist as distinct
  types in both `biscuit-terminal::discovery::detection` and `darkmatter`.
  Adding `renderable::color::ColorDepth` / `ColorMode` creates three
  same-named types. Acceptable (different modules), or should the `renderable`
  types take disambiguating names (e.g. `TerminalColorDepth`)? Recommendation:
  use plain `ColorDepth` / `ColorMode` in `renderable::color` and let the
  other crates re-export or alias.
- **OQ-2 — Variant set for `renderable::color::ColorDepth`.** Mirror
  `biscuit-terminal`'s five-variant enum (`None`, `Minimal`, `Basic`,
  `Enhanced`, `TrueColor`) for a lossless map, or a coarser set
  (`None`, `Ansi16`, `Ansi256`, `TrueColor`) sufficient for highlighter
  downsampling? FR-4 only requires the coarser distinctions.
- **OQ-3 — Should `biscuit-terminal` and `darkmatter` eventually *replace*
  their own `ColorDepth`/`ColorMode` with the `renderable` types**, or only
  map at the `CodeRenderer` boundary? Full replacement is out of scope (§4.2)
  but the decision affects whether the new types are designed as a superset.
- **OQ-4 — Context value name.** `TerminalCodeContext` vs `CodeTerminalContext`
  vs `CodeRenderContext`. Should not collide with `TerminalRenderContext`.

## 8. Affected Code

| Crate | File | Change |
|-------|------|--------|
| `renderable` | `src/color/` (`mod.rs` + new) | New `ColorDepth`, `ColorMode` descriptors |
| `renderable` | `src/tree/render/mod.rs` | New `TerminalCodeContext`; widen `render_terminal_code`; update architecture note |
| `renderable` | `src/tree/mod.rs` | Re-export new public types |
| `biscuit-terminal` | `src/render_tree/render.rs` | `render_code_node` builds `TerminalCodeContext` from `TerminalRenderContext` |
| `biscuit-terminal` | `src/discovery/detection/color.rs` | Map `biscuit-terminal` `ColorDepth`/`ColorMode` → `renderable` descriptors |
| `biscuit-terminal` | tree-render tests | Update the Phase 3 stub `CodeRenderer` |

Darkmatter is **not** touched by this spec — it consumes the final signature
during the subsequent migration.

## 9. Acceptance Criteria

- [ ] `renderable` defines color-depth and color-mode descriptors with no
      `biscuit-terminal` dependency (FR-1).
- [ ] `CodeRenderer::render_terminal_code` receives width, color depth, and
      color mode (FR-2); the `width`-only signature is gone (FR-7).
- [ ] The terminal renderer populates the context from `TerminalRenderContext`
      with no ambient re-detection (FR-3); the `biscuit-terminal` →
      `renderable` capability map is total (FR-4).
- [ ] `render_browser_code` and the built-in plain code renderer are
      unchanged (FR-5, FR-6).
- [ ] The Phase 3 test stub compiles against the new signature (CR-1).
- [ ] `renderable`, `biscuit-terminal`, and `darkmatter` build, test green
      (incl. `yaml_block_parity` and Group 1 parity suites), and are
      clippy-clean (CR-2, QR-1).
- [ ] The `CodeRenderer` architecture note reflects the new context (QR-3).

## 10. Out of Scope / Future Work

- The darkmatter tree-rendering migration itself, including darkmatter's
  production `impl CodeRenderer` that maps `TerminalCodeContext` onto
  `CodeHighlighter` + `TerminalOptions.color_depth`.
- Consolidating the three `ColorDepth` / `ColorMode` enums across the
  workspace onto the `renderable` canonical pair (tracked via OQ-3).
- Extending the code hook with any non-color capability signal.
