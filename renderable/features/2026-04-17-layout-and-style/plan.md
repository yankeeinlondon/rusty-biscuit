---
phases: 5
created: 2026-05-18
start_phase: 1
---

# Plan: The `Style` Primitive (Spec B)

This plan implements the `Style` primitive in `renderable` as the appearance sibling to `Layout` (Spec A). It focuses on the terminal target first, migrating `biscuit-terminal` components from bespoke styling to declared styles.

## Phase 1: Foundation — Primitives & Serialization

Establish the core types and ensure the existing leaf primitives are serializable.

- [ ] **Extend existing style primitives**
    - [ ] Add `Serialize` / `Deserialize` (with `snake_case`) to `TextEmphasis`, `UnderlineStyle`, and `EmphasisLayer` in `renderable/src/style.rs`.
- [ ] **Implement `PerMode<T>`**
    - [ ] Define `PerMode<T>` enum (`Universal`, `Adaptive { light, dark }`).
    - [ ] Add convenience constructors (`universal`, `adaptive`).
    - [ ] Implement `resolve(mode: ColorMode) -> T`.
- [ ] **Implement `Border` and `Fill`**
    - [ ] Define `Border` struct and supporting enums (`BorderWeight`, `BorderLineStyle`, `BorderSides`).
    - [ ] Define `Fill` struct and supporting enums (`FillIntensity`, `FillBand`).
- [ ] **Implement the `Style` struct**
    - [ ] Define `Style` with fields for `color`, `background`, `emphasis`, `border`, and `fill`.
    - [ ] Implement `Default` (all-none/all-false).
    - [ ] Add `Serialize` / `Deserialize` with `snake_case`.
- [ ] **Validation**
    - [ ] Add JSON sample to `Style` rustdoc.
    - [ ] Add round-trip tests for `Style`, `Border`, `Fill`, and `PerMode`.

## Phase 2: Tree Integration — `NodeAttrs` & Namespaces

Wire `Style` into the render tree machinery.

- [ ] **Namespace registration**
    - [ ] Add `STYLE` (`renderable.style`) to `HintNamespace` in `renderable/src/tree/mod.rs` (or equivalent).
- [ ] **`NodeAttrs` wiring**
    - [ ] Add `set_style(Style)` and `style() -> Option<Style>` methods to `NodeAttrs`.
    - [ ] Ensure `Style` serializes into the `renderable.style` hint map.
- [ ] **Inheritance logic**
    - [ ] Implement limited inheritance for `color` and `emphasis` during tree traversal.
    - [ ] Verify `background`, `border`, and `fill` do *not* inherit.

## Phase 3: Terminal Application — Tree Renderer

Teach the terminal tree renderer to apply `Style` during the fold.

- [ ] **Color & Emphasis lowering**
    - [ ] Implement lowering of `Style.color` and `Style.background` to ANSI SGR, applying capability-aware degradation and `PerMode` resolution.
    - [ ] Reuse `TextEmphasis::sgr_ops` for emphasis lowering.
- [ ] **Box painting (`Border` & `Fill`)**
    - [ ] Implement terminal emission for `Border` using box-drawing characters.
    - [ ] Implement terminal emission for `Fill` using background bands.
- [ ] **Tree integration**
    - [ ] Update the terminal tree renderer fold to apply the resolved `Style` to the output stream.

## Phase 4: Component Migration — `biscuit-terminal`

Migrate components from bespoke fields to `Style`.

- [ ] **`Section` migration**
    - [ ] Move hard-coded heading emphasis (bold/italic) to a declared `Style`.
- [ ] **`BlockQuote` migration**
    - [ ] Move `text_color`, `bg_color`, and `left_block_color` fields to `Style`.
    - [ ] Implement compatibility shims for existing builder methods.
- [ ] **`Table` migration**
    - [ ] Define `TableStyle` typed slot struct.
    - [ ] Move row-striping colors to `TableStyle`.
- [ ] **`Progress` migration**
    - [ ] Define `ProgressStyle` typed slot struct.
    - [ ] Move track/bracket colors and glyphs to `ProgressStyle`.
- [ ] **Validation**
    - [ ] Verify `cargo test` passes for `biscuit-terminal`.
    - [ ] Verify terminal output parity with current bespoke output.

## Phase 5: Closure — Validation & Drift

Verify success criteria and clean up.

- [ ] **Target isolation**
    - [ ] Confirm Markdown output remains unchanged (styles are ignored).
    - [ ] Confirm Browser target still builds (Style is ignored/deferred).
- [ ] **Drift verification**
    - [ ] Run `just drift-report` and verify `Styling`-facet slice is reduced.
- [ ] **Documentation**
    - [ ] Update `README.md` and module-level docs to reflect the new `Style` primitive.
    - [ ] Mark migrated bespoke fields as `#[deprecated]`.
