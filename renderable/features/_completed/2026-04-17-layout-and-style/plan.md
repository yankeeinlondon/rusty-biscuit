---
phases: 5
created: 2026-05-18
start_phase: 1
source_files_during_phase_1:
  - renderable/src/style.rs
  - renderable/src/prelude.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - renderable/src/style.rs
  - renderable/src/tree/attrs.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
  - .claude/skills/renderable/tree.md
source_files_during_phase_3:
  - biscuit-terminal/lib/src/render_tree/style.rs
  - biscuit-terminal/lib/src/render_tree/mod.rs
  - biscuit-terminal/lib/src/render_tree/render.rs
  - biscuit-terminal/lib/src/components/prose/styles.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
  - biscuit-terminal/lib/src/components/section.rs
  - biscuit-terminal/lib/src/render_tree/render.rs
  - biscuit-terminal/lib/src/components/block_quote.rs
  - biscuit-terminal/lib/src/components/table/types.rs
  - biscuit-terminal/lib/src/components/table/mod.rs
  - biscuit-terminal/lib/src/components/table/table.rs
  - biscuit-terminal/lib/src/components/progress.rs
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
source_files_during_phase_5: []
docs_updated_during_phase_5:
  - renderable/README.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
  - .claude/skills/renderable/SKILL.md
  - .claude/skills/renderable/style.md
packages:
  - renderable
  - biscuit-terminal
---

# Plan: The `Style` Primitive (Spec B)

This plan implements the `Style` primitive in `renderable` as the appearance sibling to `Layout` (Spec A). It focuses on the terminal target first, migrating `biscuit-terminal` components from bespoke styling to declared styles.

## Phase 1: Foundation — Primitives & Serialization

Establish the core types and ensure the existing leaf primitives are serializable.

- [x] **Extend existing style primitives**
    - [x] Add `Serialize` / `Deserialize` (with `snake_case`) to `TextEmphasis`, `UnderlineStyle`, and `EmphasisLayer` in `renderable/src/style.rs`.
- [x] **Implement `PerMode<T>`**
    - [x] Define `PerMode<T>` enum (`Universal`, `Adaptive { light, dark }`).
    - [x] Add convenience constructors (`universal`, `adaptive`).
    - [x] Implement `resolve(mode: ColorMode) -> T`.
- [x] **Implement `Border` and `Fill`**
    - [x] Define `Border` struct and supporting enums (`BorderWeight`, `BorderLineStyle`, `BorderSides`).
    - [x] Define `Fill` struct and supporting enums (`FillIntensity`, `FillBand`).
- [x] **Implement the `Style` struct**
    - [x] Define `Style` with fields for `color`, `background`, `emphasis`, `border`, and `fill`.
    - [x] Implement `Default` (all-none/all-false).
    - [x] Add `Serialize` / `Deserialize` with `snake_case`.
- [x] **Validation**
    - [x] Add JSON sample to `Style` rustdoc.
    - [x] Add round-trip tests for `Style`, `Border`, `Fill`, and `PerMode`.

## Phase 2: Tree Integration — `NodeAttrs` & Namespaces

Wire `Style` into the render tree machinery.

- [x] **Namespace registration**
    - [x] Add `STYLE` (`renderable.style`) to `HintNamespace` in `renderable/src/tree/mod.rs` (or equivalent).
- [x] **`NodeAttrs` wiring**
    - [x] Add `set_style(Style)` and `style() -> Option<Style>` methods to `NodeAttrs`.
    - [x] Ensure `Style` serializes into the `renderable.style` hint map.
- [x] **Inheritance logic**
    - [x] Implement limited inheritance for `color` and `emphasis` during tree traversal.
    - [x] Verify `background`, `border`, and `fill` do *not* inherit.

## Phase 3: Terminal Application — Tree Renderer

Teach the terminal tree renderer to apply `Style` during the fold.

- [x] **Color & Emphasis lowering**
    - [x] Implement lowering of `Style.color` and `Style.background` to ANSI SGR, applying capability-aware degradation and `PerMode` resolution.
    - [x] Reuse `TextEmphasis::sgr_ops` for emphasis lowering.
- [x] **Box painting (`Border` & `Fill`)**
    - [x] Implement terminal emission for `Border` using box-drawing characters.
    - [x] Implement terminal emission for `Fill` using background bands.
- [x] **Tree integration**
    - [x] Update the terminal tree renderer fold to apply the resolved `Style` to the output stream.

## Phase 4: Component Migration — `biscuit-terminal`

Migrate components from bespoke fields to `Style`.

- [x] **`Section` migration**
    - [x] Move hard-coded heading emphasis (bold/italic) to a declared `Style`.
- [x] **`BlockQuote` migration**
    - [x] Move `text_color`, `bg_color`, and `left_block_color` fields to `Style`.
    - [x] Implement compatibility shims for existing builder methods.
- [x] **`Table` migration**
    - [x] Define `TableStyle` typed slot struct.
    - [x] Move row-striping colors to `TableStyle`.
- [x] **`Progress` migration**
    - [x] Define `ProgressStyle` typed slot struct.
    - [x] Move track/bracket colors and glyphs to `ProgressStyle`.
- [x] **Validation**
    - [x] Verify `cargo test` passes for `biscuit-terminal`.
    - [x] Verify terminal output parity with current bespoke output.

## Phase 5: Closure — Validation & Drift

Verify success criteria and clean up.

- [x] **Target isolation**
    - [x] Confirm Markdown output remains unchanged (styles are ignored).
    - [x] Confirm Browser target still builds (Style is ignored/deferred).
- [x] **Drift verification**
    - [x] Run `just drift-report` and verify `Styling`-facet slice is reduced.
- [x] **Documentation**
    - [x] Update `README.md` and module-level docs to reflect the new `Style` primitive.
    - [x] Mark migrated bespoke fields as `#[deprecated]` — **N/A by phase 4
      design.** Phase 4 *removed* the bespoke fields outright (replacing them
      with declared `Style` / typed slot structs), a stronger outcome than
      deprecation, so no fields remain to annotate. The surviving builder
      methods are intentionally kept as compatibility shims (their rustdoc
      says so); annotating them `#[deprecated]` was rejected because the
      workspace lints with `-D warnings`, so it would cascade build failures
      across ~10 downstream crates (`claudine`, `darkmatter`, `sniff`,
      `playa`, `homelab`, `biscuit-terminal-cli`) that legitimately still
      call them.
