---
agent: opencode/zai-coding-plan/glm-5.2
total_phases: 4
created: 2026-07-09
phase: 4
yolo: "true"
source_files_during_phase_1:
  - darkmatter/dmls/src/overlay/expressions.rs
  - darkmatter/dmls/src/providers/dsl.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - darkmatter/dmls/src/overlay/expressions.rs
  - darkmatter/dmls/src/providers/dsl.rs
  - darkmatter/dmls/src/providers/frontmatter.rs
  - darkmatter/lib/src/markdown/compose/expression/catalog.rs
  - darkmatter/lib/src/markdown/schemas/resolve.rs
  - darkmatter/lib/src/markdown/render_tree/style_tree_parity_tests.rs
  - darkmatter/lib/tests/snapshots/cutover_reference__ref_centered_table_terminal.snap
  - darkmatter/lib/tests/snapshots/cutover_reference__ref_table_max_width_terminal.snap
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
  - .claude/skills/darkmatter/SKILL.md
source_files_during_phase_3:
  - darkmatter/dmls/src/capabilities.rs
  - darkmatter/dmls/src/providers/dsl.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
  - .claude/skills/darkmatter/SKILL.md
source_files_during_phase_4:
  - darkmatter/dmls/tests/level2_lsp_session.rs
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
source_code:
  - darkmatter/dmls/src/overlay/expressions.rs
  - darkmatter/dmls/src/providers/dsl.rs
  - darkmatter/dmls/src/providers/frontmatter.rs
  - darkmatter/dmls/src/capabilities.rs
  - darkmatter/dmls/tests/level2_lsp_session.rs
  - darkmatter/lib/src/markdown/compose/expression/catalog.rs
  - darkmatter/lib/src/markdown/schemas/resolve.rs
  - darkmatter/lib/src/markdown/render_tree/style_tree_parity_tests.rs
  - darkmatter/lib/tests/snapshots/cutover_reference__ref_centered_table_terminal.snap
  - darkmatter/lib/tests/snapshots/cutover_reference__ref_table_max_width_terminal.snap
documentation: []
packages:
  - darkmatter
  - dmls
---

# DMLS Interpolation Assistance (Hover + Completion) Plan

This is the **editor-facing** consumer of the completed
[single-sourcing-schema](../_completed/2026-07-08-single-sourcing-schema/spec.md)
work. It owns no library, schema, expression-evaluator, or context-capture
change. It consumes the single-sourced `context_variable_descriptors()` and
`EXPRESSION_FUNCTION_DESCRIPTORS` catalogs to improve DMLS hover and completion
inside `{{ ... }}` interpolation.

## Scope summary (by design decision)

| Decision | Surface | Phase |
|----------|---------|-------|
| D1 — One semantic adapter in `overlay::expressions` (stop lossy tuple projection; add shared `ctx.*` hover formatter) | adapter foundation | 1 |
| D2 — Enrich interpolation `ctx.*` hover (type + description + compose-time note) | `providers::dsl` hover | 2 |
| D5 — Catalog-backed function-call hover in v1 (typed signature + description) | `providers::dsl` hover | 2 |
| D3 — `ctx.*` completion metadata in correct LSP fields (`detail` = type, `documentation` = eager Markdown, eager `textEdit`); advertise `.` trigger | `providers::dsl` completion + `capabilities` | 3 |
| D4 — Typed function descriptors for completion (`detail` = `typed_signature()`, eager `documentation`, bare-name insertion) | `providers::dsl` completion | 3 |
| D6 — Do **not** add array-rendering hints in v1 | constraint (non-action) | — |

## Files touched

| File | Role |
|------|------|
| `darkmatter/dmls/src/overlay/expressions.rs` | D1 adapter — replaces lossy tuple accessors with descriptor-returning ones; owns shared `ctx.*` hover formatter |
| `darkmatter/dmls/src/providers/dsl.rs` | D2/D5 hover (`interpolation_hover`), D3/D4 completion (`interpolation_completions`, `text_edit_item`) |
| `darkmatter/dmls/src/providers/frontmatter.rs` | D1/D2 — routes `ctx_hover` through the shared formatter |
| `darkmatter/dmls/src/capabilities.rs` | D3 — adds `.` to `trigger_characters` |
| `darkmatter/dmls/tests/level2_lsp_session.rs` | L2 integration tests for hover/completion shapes |
| `darkmatter/dmls/tests/no_side_effects.rs` | Regression — must continue to pass (acceptance criterion 7/8) |

## Phase 1 — Shared Catalog Adapter (D1)

**Goal:** Replace the lossy `(name, description)` / `(signature, description)`
tuple projections in `dmls::overlay::expressions` with borrowed descriptor
accessors, and add the shared Markdown formatter for the catalog-backed
`ctx.*` hover block. This phase introduces no user-visible behavior change on
its own — it is the foundation every subsequent phase consumes.

- [x] Add a descriptor-returning context-variable lookup to `overlay::expressions`: `ctx_descriptor(name: &str) -> Option<&'static ContextVariableDescriptor>` that finds a descriptor by its bare tail name (e.g. `"today"`), backed by `context_variable_descriptors()`.
- [x] Add a descriptor-returning expression-function lookup: `function_descriptor(name: &str) -> Option<&'static ExpressionFunctionDescriptor>` that matches by the bare function name (the leading identifier of `signature`), backed by `EXPRESSION_FUNCTION_DESCRIPTORS`.
- [x] Remove or rewrite the lossy tuple accessors `ctx_names()` and `function_signatures()` so DMLS no longer has a code path that discards type information. Call sites migrate to the new descriptor accessors. (`is_ctx_name` and `function_description` may be kept as thin wrappers over the new lookups if their call sites still need only that datum, but must not be the primary access path for new work.)
- [x] Add a shared Markdown formatter in `overlay::expressions` that renders the catalog-backed portion of a `ctx.*` hover from one `ContextVariableDescriptor`: the qualified name (`ctx.<name>`), rendered `display_type`, the read-only/Darkmatter-owned ownership note, and the description. This formatter is the single authority for that block — both `providers::dsl` and `providers::frontmatter` consume it so the two surfaces can never drift.
- [x] Add a shared Markdown formatter for the function-call catalog block from one `ExpressionFunctionDescriptor`: the `typed_signature()` and the description. Used by D5 hover and available for D4 completion documentation.
- [x] Add unit tests in `overlay::expressions` proving the adapter returns catalog descriptors and preserves rendered array types (e.g. `ctx.packages` → `string[]`) and typed function signatures (e.g. a fallible function yields the `| error` suffix).
- [x] Validation checkpoint: `just test` passes in the `dmls` package area; the adapter compiles with no remaining lossy tuple accessor as the primary descriptor path.

> **Parallelizable within Phase 1:** the two descriptor accessors and the two
> formatters are independent units of work once the import surface is agreed.

## Phase 2 — Enriched Interpolation Hover: `ctx.*` (D2) + Function-Call (D5)

**Goal:** Both hover enrichments land together because they modify the same
`interpolation_hover` function in `providers::dsl` and share the Phase 1
formatters. Depends on **Phase 1**.

- [x] In `interpolation_hover` (`providers::dsl`), when the parsed expression root is a `ctx.<name>` variable (explicit `ctx.` prefix required), look up the descriptor via the Phase 1 adapter and render the shared catalog-backed block (name, type, ownership, description).
- [x] Append the DMLS-owned passive compose-time note — the "evaluated at _compose_ time (rather than now)" wording (exact string in spec § "Concurrent DMLS changes to preserve" / `providers::dsl`) — after the shared block. This note is interpolation-specific and must not appear in the frontmatter `ctx_hover`.
- [x] Enforce the D2 classification rule: only an explicitly `ctx.`-qualified root receives context-variable metadata. A bare interpolation such as `{{ today }}` is treated as a frontmatter variable even when `today` is a known context-variable tail; an unknown `ctx.<name>` retains the generic expression hover and borrows no metadata from a similarly named bare key.
- [x] Route the frontmatter provider's `ctx_hover` (`providers::frontmatter`) through the same shared Phase 1 formatter so its catalog-backed block is byte-identical to the interpolation hover's catalog-backed block for the same variable. Remove the inline formatting currently in `ctx_hover`.
- [x] Add function-call hover (D5): when the cursor is inside an interpolation and the deepest `FunctionCall` AST node whose `span` contains the cursor offset names a known catalog function, render that descriptor's typed signature and description (via the Phase 1 function formatter). The hover range remains the complete `{{ ... }}` expression (no AST span change). Unknown functions retain the generic parsed-expression hover.
- [x] Add a helper to walk the spanned expression AST (`SpannedExpr`) and find the deepest `FunctionCall` whose `span` contains a given offset, returning its name. This is the D5 cursor-to-function resolver.
- [x] Add unit tests proving interpolation and frontmatter `ctx.*` hover share the same catalog-backed name/type/ownership/description block.
- [x] Add unit tests proving interpolation hover appends the passive compose-time note and never evaluates `ctx.*`.
- [x] Add unit tests proving a bare key whose name matches a `ctx.*` tail is treated as frontmatter, not as generated context.
- [x] Add unit tests for function hover covering a formatting function (e.g. `as_csv`), a pre-existing function (e.g. `length`), and an unknown function (generic hover retained).
- [x] Validation checkpoint: `just test` and `just lint` pass in the `dmls` package area.

> **Parallelizable with Phase 3** after Phase 1 lands, since hover and
> completion touch different functions in `providers::dsl` (`interpolation_hover`
> vs `interpolation_completions`). Coordinate only on the shared `text_edit_item`
> helper if Phase 3 extends it.

## Phase 3 — Catalog-Backed Completion (D3 + D4) + `.` Trigger

**Goal:** Put completion metadata in the correct LSP fields using the Phase 1
descriptor accessors, and advertise `.` as a completion trigger so `ctx.`
auto-completes. Depends on **Phase 1**. Parallelizable with Phase 2.

- [x] Extend the `text_edit_item` helper in `providers::dsl` (or add a sibling helper) to accept an eager Markdown `documentation` field (`CompletionItemDocumentation::MarkupContent` with `MarkupKind::Markdown`), so completion items can carry both `detail` and `documentation`.
- [x] Update `ctx.*` completion items in `interpolation_completions`: `label` and inserted text are the fully qualified `ctx.<name>`; `kind` remains `VARIABLE`; `detail` is the descriptor's rendered `display_type` (e.g. `string[]`); `documentation` is eager Markdown containing the descriptor description; `textEdit` eagerly replaces the current interpolation token (preserving the existing Zed-safe no-snippet behavior).
- [x] Update expression-function completion items: the existing untyped `signature` remains the label (e.g. `as_csv(list)`); insertion remains the bare function name with no snippet or synthesized parentheses; `detail` is `ExpressionFunctionDescriptor::typed_signature()`; `documentation` is the descriptor description as eager Markdown.
- [x] Keep completion matching prefix-based and case-sensitive. `{{ ctx.pa }}` offers matching `ctx.*` variables; it does not offer removed `*_list` aliases. Existing top-level-frontmatter and expression-function candidates remain available in their current contexts.
- [x] Add `"."` to `CompletionOptions::trigger_characters` in `capabilities.rs`, alongside the existing `/`, `(`, and `#` triggers (do not drop any existing trigger).
- [x] Keep the completion provider's open-interpolation guard: a period in ordinary prose (outside an open `{{ ... }}`) produces no DSL completion items. The `completion_partial` function already verifies the cursor is inside an open interpolation.
- [x] Add unit tests proving `ctx.*` completion sets `detail`, eager Markdown `documentation`, and the eager token-replacing `textEdit` from the catalog descriptor.
- [x] Add unit tests proving function completion sets the untyped label, typed-signature `detail`, eager documentation, and bare-name insertion from one descriptor.
- [x] Add unit tests proving all six formatting functions (`as_csv`, `as_tsv`, `as_space_separated`, `as_line_separated`, `as_unordered_list`, `as_ordered_list`) are present in completion, and at least one fallible typed signature asserts the `| error` suffix in `detail`.
- [x] Add unit tests proving completion after a period outside an open interpolation returns no DSL candidates.
- [x] Validation checkpoint: `just test` and `just lint` pass in the `dmls` package area.

> **Parallelizable with Phase 2** after Phase 1 lands. If both phases extend the
> shared `text_edit_item` helper, land that extension first (or in Phase 1) to
> avoid a merge conflict.

## Phase 4 — Capability Advertisement + L2 Integration Tests

**Goal:** End-to-end validation through an in-memory LSP session, capability
advertisement checks, and the no-side-effects regression. Depends on **Phases 2
and 3**.

- [x] Add an L2 test in `tests/level2_lsp_session.rs` proving the `initialize` response advertises `.` as a completion trigger without dropping `/`, `(`, or `#`.
- [x] Add an L2 test verifying interpolation `ctx.*` hover response shape: the catalog-derived type and description are present, the passive compose-time note is appended, and the catalog-backed block matches the frontmatter `ctx_hover` for the same variable.
- [x] Add an L2 test verifying `ctx.*` completion response shape: `detail` carries the rendered type, `documentation` is eager Markdown, and the `textEdit` range/edit are correct.
- [x] Add an L2 test verifying function completion response shape: `detail` is the typed signature (including `| error` for a fallible function), `documentation` is eager Markdown, and insertion is the bare function name.
- [x] Add an L2 test verifying function-call hover shows the typed signature and description for a known function, and the generic hover for an unknown function.
- [x] In at least one L2 hover/completion test, include an astral Unicode character (e.g. an emoji) before the interpolation so the negotiated UTF-16 position path is exercised and the `textEdit`/hover ranges are correct under wide characters.
- [x] Add an L2 test proving a period trigger outside an open interpolation produces no DSL completion items.
- [x] Confirm the existing `no_side_effects` test continues to pass unchanged — editor assistance does not execute directives, expressions, or commands (acceptance criterion 7).
- [x] Validation checkpoint: `just test`, `just test-l2`, and `just lint` all pass in the `dmls` package area. Confirm the change introduces no platform-specific path or terminal behavior (macOS, Windows, Linux portable).

> **Parallelizable within Phase 4:** the capability test and the response-shape
> tests are independent once Phases 2–3 have landed.

## Acceptance criteria mapping

| Criterion | Phase(s) |
|-----------|----------|
| 1. `{{ ctx.<name> }}` hover shows catalog type + description + compose-time note; matches frontmatter | 1, 2, 4 |
| 2. Only explicitly qualified `ctx.*` receives context-variable metadata | 2, 4 |
| 3. `ctx.*` completion exposes type in `detail`, description in eager `documentation`, correct eager `textEdit` | 1, 3, 4 |
| 4. `.` is an advertised trigger; triggering it outside interpolation yields no DSL items | 3, 4 |
| 5. Every function completion derives label/typed-detail/documentation/insertion from one descriptor; six formatting functions present with fallibility | 1, 3, 4 |
| 6. Hovering a known function shows typed signature + description; unknown keeps generic hover | 1, 2, 4 |
| 7. No DMLS-side re-declaration of semantic data; no editor request evaluates/executes content | 1, 2, 3, 4 |
| 8. L1, L2, lint, and no-side-effects suites pass; macOS/Windows/Linux portable | 4 |
