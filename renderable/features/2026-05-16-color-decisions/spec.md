---
title: "CodeRenderer Terminal Color Context"
created: "2026-05-16"
revised: "2026-05-16"
revision: 2
status: draft
confidence: high
related:
  - renderable/features/2026-05-16-iterative-improvement/components-group1-plan.md
  - .ai/plans/2026-05-16.execution-of-components-group1.md
review: renderable/features/2026-05-16-color-decisions/review.md
precursor_to: "Darkmatter tree-rendering migration"
---

# CodeRenderer Terminal Color Context

> **Revision 2** incorporates the architect review
> (`review.md`): the `TerminalCodeContext` struct is now settled, the
> `ColorDepth` variant set and `ColorMode::Unknown` semantics are settled,
> a no-color hook contract and boundary conversion helpers are specified,
> pass-through tests are added to acceptance criteria, the over-claim about
> current darkmatter downsampling is corrected, and code-block metadata plus
> true depth downsampling are captured as darkmatter-migration future work.

## 1. Summary

The `CodeRenderer` hook's terminal method, `render_terminal_code`, currently
receives only a `width: u32`. It must be widened to carry the terminal's
**color depth** and **color mode** so that a code-renderer implementation can
make the same capability-aware color decisions the rest of the tree renderer
makes — without breaking the crate dependency graph.

The widened parameter is a settled, `renderable`-owned `TerminalCodeContext`
value. This is a small, additive change. It is specified as a **standalone
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
  defines its own `ColorMode` (`Light` / `Dark`) and a `detect_color_mode()`;
  `CodeHighlighter` is constructed with it to pick a syntect theme.
- **Color depth** — `darkmatter::markdown::output::terminal` defines its own
  `ColorDepth` (`None`, `Colors256`, `TrueColor`, …) with an `auto_detect()`,
  and `TerminalOptions.color_depth` is part of darkmatter's terminal options.

**Accurate scope of current darkmatter `color_depth` behavior.** Today
darkmatter uses `color_depth` primarily for the **no-color early return**
(`ColorDepth::None`) and for constructing the shared `biscuit-terminal::Terminal`.
It is **not** the case that darkmatter consistently downsamples 24-bit SGR to
256/16-color output for code-heavy content: code-block and prose paths still
emit many `38;2` / `48;2` true-color sequences regardless of the advertised
256/16-color depth. True depth-aware downsampling is a **goal of the darkmatter
tree-rendering migration** (see §10), not an existing guarantee — but the
context this precursor adds is the prerequisite that makes it implementable
consistently.

A darkmatter `CodeRenderer` that receives only `width` would have to re-derive
both color depth and color mode by ambient detection — re-introducing the
consistency gap of §3.1 for the one node kind that most needs faithful color
handling.

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
- Implementing true depth-aware SGR downsampling. It **is** a goal of the
  darkmatter migration (§10) but is out of scope for this precursor, which
  only delivers the capability context that makes it possible.
- Changing `render_browser_code` — the browser target has no terminal
  capability concept; it stays as-is.
- Replacing the existing `ColorDepth` / `ColorMode` enums in
  `biscuit-terminal` and `darkmatter` with the new `renderable` types. This
  precursor only introduces the `renderable` types and maps **at the
  `CodeRenderer` boundary**. Workspace-wide consolidation is deferred (§7,
  OQ-1).
- Adding OSC8 hyperlink or image-protocol information to the code hook — code
  blocks emit neither.
- Carrying code-block metadata beyond color (title, line numbering, line
  highlighting). Captured as darkmatter-migration future work (§10).

## 5. Requirements

### 5.1 Functional requirements

- **FR-1** — `renderable` MUST define terminal color-capability descriptor
  types — `ColorDepth` and `ColorMode` — in `renderable::color`, usable by
  `CodeRenderer` without depending on `biscuit-terminal`.
- **FR-2** — `renderable` MUST define a `TerminalCodeContext` value type
  carrying `width: u32`, `color_depth: ColorDepth`, and
  `color_mode: ColorMode`. `CodeRenderer::render_terminal_code` MUST receive
  it in addition to `lang`, `value`, and `attrs`.
- **FR-3** — The terminal tree renderer (`biscuit-terminal`) MUST populate the
  `TerminalCodeContext` by mapping from its `TerminalRenderContext`, using
  `available_width` (NOT root `width`), `color_depth`, and `color_mode`, at
  the `render_code_node` call site. No ambient re-detection in the renderer.
- **FR-4** — `renderable::color::ColorDepth` MUST mirror `biscuit-terminal`'s
  five-variant enum — `None`, `Minimal`, `Basic`, `Enhanced`, `TrueColor` —
  so the boundary mapping is total and lossless, including 8-color `Minimal`.
- **FR-5** — `renderable::color::ColorMode` MUST have the variants `Light`,
  `Dark`, and `Unknown`, mirroring `biscuit-terminal`. `Unknown` MUST mean
  "the terminal renderer could not determine the background"; see §6 D-6 for
  resolution rules.
- **FR-6** — A boundary conversion API MUST exist so call sites do not inline
  `match` expressions. It MUST be implemented as
  `impl From<biscuit_terminal::discovery::detection::ColorDepth> for
  renderable::color::ColorDepth` (and the same for `ColorMode`), located in
  `biscuit-terminal` (the crate that can name both types).
- **FR-7** — `render_browser_code` MUST be unchanged.
- **FR-8** — The built-in plain code renderer (`Writer::render_code`) MUST be
  unaffected; it already renders through the full `TerminalRenderContext`.
- **FR-9** — The `width`-only behavior MUST NOT be retained as a second code
  path. There is one terminal code hook signature.
- **FR-10** — The `CodeRenderer` trait documentation MUST state the no-color
  contract: an implementor SHOULD treat `ColorDepth::None` as "emit no ANSI
  styling", and if it cannot honor the supplied capability context it SHOULD
  return `None` so the built-in plain renderer handles the block. Implementors
  MUST NOT run ambient capability detection to override the supplied context.

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
- **QR-2** — The new `renderable` descriptor and context types MUST be
  documented per the repo rustdoc convention (no H1 in `///`, `## Examples`
  etc.). `ColorDepth` / `ColorMode` MUST derive `Debug, Clone, Copy,
  PartialEq, Eq` (fieldless enums) and serde where consistent with the
  `renderable::color` module. `TerminalCodeContext` MUST derive `Debug, Clone,
  Copy, PartialEq, Eq` and provide a `new(width, color_depth, color_mode)`
  constructor.
- **QR-3** — The architecture note on the `CodeRenderer` trait MUST be updated
  to describe `TerminalCodeContext` instead of the `width: u32` rationale, and
  MUST include the no-color contract (FR-10).
- **QR-4** — `renderable::color`'s module documentation MUST be extended to
  state that `ColorDepth` / `ColorMode` are **terminal capability
  descriptors**, distinct from the color-*value* types in the module, and that
  terminal ANSI emission still lives in `biscuit-terminal`.

## 6. Design Decisions

All decisions below are **settled** for implementation.

- **D-1 — The trait stays in `renderable`.** Homing `CodeRenderer` in
  `biscuit-terminal` would block the browser renderer (which lives in
  `renderable`) from sharing the trait. The dependency direction is not
  negotiable.
- **D-2 — The hook carries a `TerminalCodeContext` value, not the full
  `TerminalRenderContext`.** `renderable` cannot name that type; a
  purpose-built `renderable`-owned value is the only option that respects D-1.
- **D-3 — Scope is width + color depth + color mode only.** Code blocks do not
  emit hyperlinks or images; Unicode support is already reflected in how
  `value` is projected. Adding unused fields is rejected (simplicity).
- **D-4 — `TerminalCodeContext` is a `Copy` struct passed by value.** All
  fields are `Copy`, so the hook takes `context: TerminalCodeContext` by value
  — no lifetimes for implementors. It provides a
  `new(width, color_depth, color_mode)` constructor.

  ```rust
  // renderable::tree::render  (re-exported from renderable::tree)
  #[derive(Debug, Clone, Copy, PartialEq, Eq)]
  pub struct TerminalCodeContext {
      /// Available render width in columns (post-indent).
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
      context: TerminalCodeContext,
  ) -> Option<String>;
  ```

- **D-5 — Color-capability descriptors live in `renderable::color`.**
  `renderable::color` already owns the color *value* system (`Color`,
  `WebColor`, `BasicColor`, RGB, HDR). Color *capability* descriptors
  (`ColorDepth`, `ColorMode`) are a natural fit there, and placing them in the
  lowest crate lets `biscuit-terminal` and `darkmatter` map onto a single
  canonical pair. Module docs MUST clarify these are capability descriptors,
  not terminal-rendering APIs (QR-4).
- **D-6 — `ColorMode::Unknown` resolution rules.**
  - `renderable::color::ColorMode::Unknown` means "the terminal renderer could
    not determine the background." It is a faithful signal, not an error.
  - A `CodeRenderer` MUST NOT run ambient detection to resolve `Unknown`
    (that would re-introduce the §3.1 consistency gap).
  - When a code renderer must pick a concrete light/dark treatment, it
    resolves `Unknown` against its own configured option (e.g. darkmatter's
    `TerminalOptions.color_mode`); if no such option is configured at the
    tree-rendering entry point, it defaults to `Dark`. This rule is recorded
    here so the darkmatter implementation does not invent an ad-hoc fallback.
- **D-7 — Five-variant `ColorDepth` (resolves former OQ-2).** The enum mirrors
  `biscuit-terminal` exactly (`None`, `Minimal`, `Basic`, `Enhanced`,
  `TrueColor`) so the boundary map is lossless. A consumer that is internally
  coarser (e.g. darkmatter's `None` / `Colors256` / `TrueColor`) maps the
  surplus variants *consciously* on its own side rather than losing them at
  the boundary.
- **D-8 — Boundary conversion via `From` impls in `biscuit-terminal`
  (resolves former Rec-5).** `impl From<…detection::ColorDepth> for
  renderable::color::ColorDepth` and the `ColorMode` equivalent live in
  `biscuit-terminal`, where both source and target types are nameable. This
  keeps `render_code_node` focused on rendering and the conversion reusable.
- **D-9 — Plain type names with use-site aliasing (resolves former OQ-3 /
  Rec-10).** The new types are named `ColorDepth` / `ColorMode` (consistent
  with `renderable::color` owning color types). Because identically named
  types exist in `biscuit-terminal::discovery::detection` and `darkmatter`,
  crates that import more than one MUST alias at the use site, e.g.
  `use renderable::color::ColorDepth as RenderColorDepth;` — matching the
  existing precedent (`darkmatter` already does
  `use biscuit_terminal::discovery::detection::ColorDepth as TerminalColorDepth`).

## 7. Open Questions

- **OQ-1 — Workspace-wide consolidation.** Should `biscuit-terminal` and
  `darkmatter` eventually *replace* their own `ColorDepth` / `ColorMode` with
  the `renderable` canonical pair, rather than only mapping at the
  `CodeRenderer` boundary? This is **deferred** — out of scope for this
  precursor (§4.2). D-7 already designs the `renderable` enum as a faithful
  superset/mirror, so a later consolidation is not blocked.

## 8. Affected Code

| Crate | File | Change |
|-------|------|--------|
| `renderable` | `src/color/` (`mod.rs` + new module) | New `ColorDepth`, `ColorMode` capability descriptors; module-doc clarification (QR-4) |
| `renderable` | `src/tree/render/mod.rs` | New `TerminalCodeContext`; widen `render_terminal_code`; update architecture note + no-color contract |
| `renderable` | `src/tree/mod.rs` | Re-export `TerminalCodeContext` and the new color types |
| `biscuit-terminal` | `src/discovery/detection/color.rs` | `From` impls mapping `biscuit-terminal` `ColorDepth`/`ColorMode` → `renderable` descriptors (D-8) |
| `biscuit-terminal` | `src/render_tree/render.rs` | `render_code_node` builds `TerminalCodeContext` from `TerminalRenderContext` (`available_width`, `color_depth`, `color_mode`) |
| `biscuit-terminal` | tree-render tests | Update the Phase 3 stub `CodeRenderer`; add context pass-through tests (§9) |

Darkmatter is **not** touched by this spec — it consumes the final signature
during the subsequent migration.

## 9. Acceptance Criteria

- [ ] `renderable::color` defines `ColorDepth` (five variants, D-7) and
      `ColorMode` (`Light`/`Dark`/`Unknown`), with no `biscuit-terminal`
      dependency (FR-1, FR-4, FR-5).
- [ ] `renderable` defines `Copy` `TerminalCodeContext` with a `new(...)`
      constructor (FR-2, D-4, QR-2).
- [ ] `CodeRenderer::render_terminal_code` takes `TerminalCodeContext` by
      value; the `width`-only signature is gone (FR-2, FR-9).
- [ ] `biscuit-terminal` provides `From` impls for both capability types
      (FR-6, D-8).
- [ ] `render_code_node` populates the context from `TerminalRenderContext`
      with no ambient re-detection (FR-3).
- [ ] **Pass-through tests** in `biscuit-terminal`: a stub `CodeRenderer` is
      installed and asserted to receive
      (a) `available_width`, not root `width` (verified via a nested/indented
      context where the two differ);
      (b) the `ColorDepth` configured on the manually built
      `TerminalRenderContext`;
      (c) the `ColorMode` configured on the context, including `Unknown`;
      (d) values matching the manually built context with no influence from
      conflicting ambient env vars.
- [ ] `render_browser_code` and the built-in plain code renderer are
      unchanged (FR-7, FR-8).
- [ ] The `CodeRenderer` docs state the no-color / return-`None` contract and
      forbid ambient detection (FR-10, QR-3); `renderable::color` module docs
      mark the new types as capability descriptors (QR-4).
- [ ] The Phase 3 test stub compiles against the new signature (CR-1).
- [ ] `renderable`, `biscuit-terminal`, and `darkmatter` build, test green
      (incl. `yaml_block_parity` and Group 1 parity suites), and are
      clippy-clean (CR-2, QR-1).

## 10. Out of Scope / Future Work

These belong to the **darkmatter tree-rendering migration** spec, not this
precursor. They are recorded here so the next spec does not lose them.

- **Darkmatter `impl CodeRenderer`.** A production code renderer that maps
  `TerminalCodeContext` onto `CodeHighlighter` + `TerminalOptions`, with:
  - `TerminalCodeContext.color_depth` → `TerminalOptions.color_depth` **with
    no `auto_detect()` call** — the supplied context is authoritative;
  - `TerminalCodeContext.color_mode` → darkmatter's `highlighting::ColorMode`
    using the explicit `Unknown` fallback rule (D-6);
  - an acceptance test that sets **conflicting ambient env vars and render
    context values** and verifies code blocks follow the render context.
- **True depth-aware SGR downsampling — confirmed goal.** The darkmatter
  migration MUST downsample 24-bit `38;2` / `48;2` sequences to 256-color
  (`Enhanced`), 16-color (`Basic`), 8-color (`Minimal`), or strip them
  entirely (`None`) according to `TerminalCodeContext.color_depth`. This is a
  separate acceptance criterion for that migration; this precursor only
  supplies the depth signal that makes it implementable.
- **Code-block metadata beyond color.** Color context alone is not sufficient
  for full darkmatter code-block parity. Markdown fences carry richer info
  strings (`CodeBlockMeta`: title, line numbering, line highlighting). Note
  that `NodeKind::Code` already has an unused `meta: Option<String>` field
  that can carry the raw info string; the migration must design how that field
  (or structured `CodeRenderHints` equivalents) preserves `CodeBlockMeta`
  before tree-rendered darkmatter code blocks can be called parity-complete.
- **Workspace-wide `ColorDepth` / `ColorMode` consolidation** (OQ-1).

## 11. Revision History

- **r2 (2026-05-16)** — Incorporated architect review (`review.md`): settled
  `TerminalCodeContext` (D-4); settled five-variant `ColorDepth` (D-7);
  settled `ColorMode::Unknown` semantics (D-6); added boundary `From` impls
  (D-8, FR-6); settled type naming / aliasing (D-9); added no-color hook
  contract (FR-10); added module-doc clarification requirement (QR-4);
  corrected the over-claim about current darkmatter downsampling (§3.2);
  added context pass-through acceptance tests (§9); captured darkmatter
  mapping tests, confirmed true downsampling goal, and code-block metadata as
  future work (§10).
- **r1 (2026-05-16)** — Initial draft.
