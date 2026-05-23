---
created: "2026-05-15T16:18:41"
review: "root/@renderable/features/2026-05-14-kickoff/review-1.md"
spec: "root/@renderable/features/2026-05-14-kickoff/spec.md"
phases: 6
current_phase: 1
---

# Project 1: Stable Base Implementation Plan

## Goal

Establish `renderable` as the shared home for cross-target rendering vocabulary while preserving existing behavior in `biscuit-terminal` and `darkmatter`.

This plan covers the stable-base work identified by the review: rename the terminal rendering trait, move browser rendering vocabulary into `renderable`, extract stylesheet data from `darkmatter`, extract layout and color data from `biscuit-terminal`, and validate the migration with targeted build, test, doc-test, and dependency checks.

## Success Criteria

- [ ] `Renderable` is renamed to `TerminalRenderable` and `RenderableContent` is renamed to `RenderableTerminalContent` across `biscuit-terminal` and `darkmatter`.
- [ ] `BrowserRenderable` is defined by `renderable` and re-exported by `biscuit-terminal` for compatibility.
- [ ] Stylesheet data types live in `renderable` with terminal-specific behavior kept in `biscuit-terminal`.
- [ ] `Layout`, `Color`, and their target-agnostic satellite types live in `renderable`; terminal-only rendering behavior stays in `biscuit-terminal`.
- [ ] `renderable` remains a leaf crate with no dependency on `biscuit-terminal` or `darkmatter`.
- [ ] `renderable`, `biscuit-terminal`, and `darkmatter` build, test, and doc-test successfully.
- [ ] Downstream workspace packages that consume terminal layout/color APIs continue compiling through compatibility re-exports.

## Planning Assumptions

- The implementation team should treat `renderable/features/2026-05-14-kickoff/spec.md`, `stylesheet-extraction.md`, `layout-and-color-move.md`, and `decisions.md` as source material for detailed symbol-level behavior.
- The requested review path is recorded in frontmatter. The file was not present in this checkout during plan creation, so this plan is grounded in the available kickoff spec, adjacent extraction notes, decisions, and existing project-plan material.
- Work should land phase-by-phase. Each phase has an explicit validation checkpoint and should leave the workspace in a compiling state before the next phase starts.
- Avoid root-wide `cargo build` or `cargo test` unless a phase explicitly calls for downstream verification. Prefer scoped package commands.

## Dependency Order

- Phase 1 must happen first because it establishes the actual current symbol inventory.
- Phase 2 must happen before Phase 3 because the `BrowserRenderable` move should preserve the renamed terminal vocabulary.
- Phase 4 must happen before Phase 5 because the layout/color move depends on stylesheet color types already living in `renderable`.
- Phase 6 happens last and validates the full stable-base surface.

## Phase 1: Baseline and Inventory

Purpose: establish a clean starting point, confirm exact package names, and identify current references before changing code.

- [ ] Run `cargo metadata --no-deps --format-version 1` and record the exact package names for `renderable`, `biscuit-terminal`, `biscuit-terminal-cli`, and `darkmatter`.
- [ ] Search for current terminal-rendering identifiers with `rg '\b(Renderable|RenderableContent)\b' biscuit-terminal darkmatter` and confirm all expected call sites are covered.
- [ ] Search for current browser-rendering identifiers with `rg '\bBrowserRenderable\b' renderable biscuit-terminal darkmatter` and identify whether `renderable` still has a placeholder browser trait.
- [ ] Inspect `renderable/features/2026-05-14-kickoff/decisions.md` and confirm decisions that constrain Project 1, especially BrowserRenderable coexistence, Scheme A stylesheet naming, and the decision not to wire `Layout` into `PageOptions`.
- [ ] Inspect `stylesheet-extraction.md` and identify the data-only stylesheet types, terminal-coupled behavior, darkmatter consumers, and required new dependencies.
- [ ] Inspect `layout-and-color-move.md` and identify the move order for `TermColor`, layout data, color data, and compatibility shims.
- [ ] Validation checkpoint: confirm no edits have been made yet, package names are known, and the implementation team has a symbol inventory for all later phases.

Parallelizable work:

- [ ] A second implementer can inspect `stylesheet-extraction.md` while another inspects `layout-and-color-move.md`, because both are read-only planning tasks.
- [ ] A second implementer can inventory downstream layout/color consumers while the primary implementer inventories render trait consumers.

## Phase 2: Rename Terminal Rendering Vocabulary

Purpose: make terminal rendering explicitly terminal-scoped before moving shared browser and style vocabulary into `renderable`.

- [ ] In `biscuit-terminal/lib/src/components/renderable.rs`, rename `Renderable` to `TerminalRenderable`.
- [ ] In `biscuit-terminal/lib/src/components/renderable.rs`, rename `RenderableContent` to `RenderableTerminalContent`.
- [ ] Update trait docs, enum docs, intra-doc links, and doc-test examples in `renderable.rs` to use the new names.
- [ ] Update `biscuit-terminal` imports, impl blocks, trait-object types, generic bounds, and qualified calls from `Renderable` to `TerminalRenderable`.
- [ ] Update `biscuit-terminal` imports and enum references from `RenderableContent` to `RenderableTerminalContent`.
- [ ] Update `biscuit-terminal/lib/src/prelude.rs` so the prelude exports `TerminalRenderable` and `RenderableTerminalContent`.
- [ ] Update `darkmatter` imports, impl blocks, trait-object types, generic bounds, and qualified calls to use `TerminalRenderable`.
- [ ] Update `darkmatter` references to use `RenderableTerminalContent` where applicable.
- [ ] Run `rg '\bRenderable\b' biscuit-terminal darkmatter` and confirm every remaining match is either `BrowserRenderable`, `TerminalRenderable`, documentation about the rename, or an intentionally preserved historical reference.
- [ ] Run `rg '\bRenderableContent\b' biscuit-terminal darkmatter` and confirm every remaining match is `RenderableTerminalContent` or intentionally preserved documentation.
- [ ] Build `biscuit-terminal`, `biscuit-terminal-cli`, and `darkmatter` with scoped `cargo build -p ...` commands.
- [ ] Run tests for `biscuit-terminal` and `darkmatter` with `cargo nextest run -p ...`, falling back to `cargo test -p ...` if nextest is unavailable.
- [ ] Run doc-tests for `biscuit-terminal` and `darkmatter`.
- [ ] Validation checkpoint: renamed symbols compile and tests pass without changing trait behavior.

Parallelizable work:

- [ ] One implementer can update `biscuit-terminal` consumers while another updates `darkmatter` consumers after the definition-site rename is complete.
- [ ] Test execution for `biscuit-terminal` and `darkmatter` can run in parallel once both crates build.

## Phase 3: Move `BrowserRenderable` to `renderable`

Purpose: make browser rendering available without depending on `biscuit-terminal`.

- [ ] Add a `renderable::browser::renderable` module that defines the legacy `BrowserRenderable` trait shape: `render_to_browser`, `render_to_browser_with_inline_variables`, and `as_any`.
- [ ] Export `BrowserRenderable` from `renderable::browser`.
- [ ] If `renderable` contains a placeholder `BrowserRenderable` trait for `render_html_fragment`, remove or reconcile it so there is only one `BrowserRenderable` trait in Project 1.
- [ ] Update `renderable` browser composition types so any `dyn BrowserRenderable` references resolve to `renderable::browser::BrowserRenderable`.
- [ ] Add `renderable` as a dependency of `biscuit-terminal`.
- [ ] Remove the local `BrowserRenderable` definition from `biscuit-terminal` and replace it with a compatibility re-export from `renderable::browser`.
- [ ] Remove any imports in `biscuit-terminal` that were only needed by the old local browser trait definition.
- [ ] Confirm existing `darkmatter` imports through `biscuit_terminal::components::renderable::BrowserRenderable` still compile through the re-export.
- [ ] Build `renderable`, `biscuit-terminal`, `biscuit-terminal-cli`, and `darkmatter`.
- [ ] Run `cargo tree -p renderable -i biscuit-terminal` and `cargo tree -p renderable -i darkmatter`; confirm `renderable` does not depend on either package.
- [ ] Run tests and doc-tests for `renderable`, `biscuit-terminal`, and `darkmatter`.
- [ ] Validation checkpoint: `BrowserRenderable` is owned by `renderable`, compatibility imports still work, and dependency direction is acyclic.

Parallelizable work:

- [ ] One implementer can update the `renderable` module and trait export while another prepares the `biscuit-terminal` dependency and re-export patch.
- [ ] Dependency checks and doc-tests can run in parallel after the build passes.

## Phase 4: Extract Stylesheet Data to `renderable`

Purpose: move cross-target CSS data and serialization out of `darkmatter` while keeping terminal-only behavior out of `renderable`.

- [ ] Read `renderable/features/2026-05-14-kickoff/stylesheet-extraction.md` in full before editing stylesheet code.
- [ ] Move stylesheet property types into `renderable`, including `CssProp`, sizing properties, color properties, integer properties, custom properties, and typed-property support.
- [ ] Move stylesheet value types into `renderable`, including `CssUnit`, sizing values, color values, raw values, value kinds, and conversion traits.
- [ ] Move `StylesheetError` into `renderable` with its target-agnostic error behavior.
- [ ] Move target-agnostic emitters into `renderable`, including CSS and JSON rendering behavior.
- [ ] Keep terminal-coupled behavior out of `renderable`, including `to_terminal_string`, `to_terminal`, and `BlockError` integration.
- [ ] Add a `biscuit-terminal` extension trait or free functions that restore terminal rendering behavior for stylesheet values.
- [ ] Apply Scheme A naming: use `CssStyle` for declaration blocks, `CssRule` for selector/block pairs, and `Stylesheet` for the rule collection.
- [ ] Leave `HtmlClassDefinition` and `ClassDefinition` names unchanged.
- [ ] Update `darkmatter` stylesheet consumers to import stylesheet data from `renderable::stylesheet`.
- [ ] Update `darkmatter` error downcasts and tests to use the moved `StylesheetError`.
- [ ] Add only the required `renderable` dependencies for the moved data model, such as `serde`, `serde_json`, and `thiserror` if not already present.
- [ ] Add focused tests in `renderable` for `CssStyle`, `CssRule`, `Stylesheet`, CSS emission, JSON emission, and error behavior.
- [ ] Add or preserve compatibility tests in `darkmatter` for YAML/browser output that depends on stylesheet rendering.
- [ ] Build `renderable`, `biscuit-terminal`, and `darkmatter`.
- [ ] Run tests and doc-tests for `renderable`, `biscuit-terminal`, and `darkmatter`.
- [ ] Add a sanity test that a stylesheet containing `CssColor::Rgb(51, 102, 153)` renders to the expected CSS and round-trips through any supported serialization path.
- [ ] Run `cargo tree -p renderable -i biscuit-terminal` and confirm the stylesheet move did not introduce a terminal dependency.
- [ ] Validation checkpoint: stylesheet data is target-agnostic in `renderable`, terminal behavior is restored through `biscuit-terminal`, and darkmatter output remains stable.

Parallelizable work:

- [ ] One implementer can move the data model into `renderable` while another updates darkmatter consumer imports after the public module path is established.
- [ ] Tests for pure stylesheet serialization can be added in parallel with terminal extension tests because they exercise separate crates.

## Phase 5: Extract Layout and Color to `renderable`

Purpose: move shared layout and color data into `renderable` while preserving terminal ANSI behavior and existing public paths through re-exports.

- [ ] Read `renderable/features/2026-05-14-kickoff/layout-and-color-move.md` in full before editing layout or color code.
- [ ] Confirm Phase 4 is complete and `CssColor` lives in `renderable`.
- [ ] Move `TermColor` into its own `biscuit-terminal` module before moving color data.
- [ ] Build and test `biscuit-terminal` after the `TermColor` preparation step to prove behavior is unchanged.
- [ ] Move layout data into `renderable`, including `Alignment`, `Margin`, `RowFill`, `MaxWidth`, `Layout`, and `WordWrap`.
- [ ] Keep terminal layout application behavior in `biscuit-terminal` through a terminal extension trait or equivalent compatibility layer.
- [ ] Add `biscuit-terminal` re-exports for moved layout symbols so existing callers keep compiling.
- [ ] Build and test `renderable` and `biscuit-terminal` after the layout move.
- [ ] Move color data into `renderable`, including `Color`, `BasicColor`, `RgbColor`, `HdrColor`, `WebColor`, `Tailwind`, `Octet`, and related errors.
- [ ] Preserve serialized enum variant names for moved color types.
- [ ] Keep ANSI rendering behavior and `TermColor` impls in `biscuit-terminal`.
- [ ] Add `biscuit-terminal` re-exports for moved color symbols so existing callers keep compiling.
- [ ] Do not add `Layout` to `PageOptions`; preserve the decision that page-level browser styling flows through stylesheet/page APIs.
- [ ] Add focused tests in `renderable` for layout defaults, margin resolution, color serialization, and color CSS conversion where applicable.
- [ ] Add terminal smoke tests proving ANSI output remains byte-identical for representative color and layout usage.
- [ ] Build `renderable`, `biscuit-terminal`, and `darkmatter`.
- [ ] Run tests and doc-tests for `renderable`, `biscuit-terminal`, and `darkmatter`.
- [ ] Update `docs/dependencies.md`, affected per-area dependency docs, and `.claude/skills/biscuit-terminal/SKILL.md` if the move changes architecture or workflow documentation.
- [ ] Validation checkpoint: layout and color data live in `renderable`, terminal rendering behavior lives in `biscuit-terminal`, and existing callers continue to compile through compatibility paths.

Parallelizable work:

- [ ] Layout data tests and color data tests can be developed in parallel once module boundaries are agreed.
- [ ] Documentation updates can happen in parallel with final downstream verification after public paths stabilize.

## Phase 6: Workspace Validation and Handoff

Purpose: prove the stable base is complete and hand implementation teams a clear final state.

- [ ] Run `cargo build -p renderable -p biscuit-terminal -p biscuit-terminal-cli -p darkmatter`, or equivalent scoped package builds if the installed Cargo version does not accept multiple `-p` values.
- [ ] Run `cargo nextest run -p renderable`, `cargo nextest run -p biscuit-terminal`, and `cargo nextest run -p darkmatter`; fall back to `cargo test -p ...` where needed.
- [ ] Run `cargo test -p renderable --doc`, `cargo test -p biscuit-terminal --doc`, and `cargo test -p darkmatter --doc`.
- [ ] Run `cargo doc -p renderable -p biscuit-terminal --no-deps` and resolve new rustdoc warnings or broken intra-doc links.
- [ ] Run `rg '\bRenderable\b' biscuit-terminal darkmatter` and confirm there are no stray uses of the old terminal trait name.
- [ ] Run `rg '\bRenderableContent\b' biscuit-terminal darkmatter` and confirm there are no stray uses of the old content enum name.
- [ ] Run `rg '\bHtmlStyleSheet\b' renderable darkmatter biscuit-terminal` and confirm the old collection name is gone if Scheme A has been fully applied in Project 1.
- [ ] Run `cargo tree -p renderable -i biscuit-terminal` and `cargo tree -p renderable -i darkmatter`; confirm no back-dependencies exist.
- [ ] Check downstream consumers named in `layout-and-color-move.md`, including `biscuit-tui`, `biscuit-visualized`, `claudine`, `messenger`, `model-citizen`, `playa`, `sniff`, `schematic`, and `unchained-ai`.
- [ ] For each downstream consumer, run the lightest scoped check that proves compatibility, starting with `cargo check -p <package>` and adding tests where the package has focused layout/color coverage.
- [ ] Confirm `renderable` exports `BrowserRenderable`, stylesheet types, layout types, and color types from stable public module paths.
- [ ] Confirm `biscuit-terminal` exports `TerminalRenderable`, `RenderableTerminalContent`, terminal stylesheet extensions, terminal layout extensions, `TermColor` behavior, and compatibility re-exports.
- [ ] Record any intentionally deferred items for Project 2 or Project 3, especially new `BrowserRenderable` API shape, `MarkdownRenderable`, `AstRenderable`, and caller migration to browser fragment/page APIs.
- [ ] Validation checkpoint: all success criteria are checked, the dependency graph is clean, and Project 1 is ready for implementation handoff or completion review.

Parallelizable work:

- [ ] Downstream package checks can be split across implementers once Phase 5 has landed.
- [ ] Documentation review and cargo/rustdoc verification can run in parallel after all code checks are green.
