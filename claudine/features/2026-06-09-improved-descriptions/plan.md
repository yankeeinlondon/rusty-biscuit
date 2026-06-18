---
agent: open_code/kimi-for-coding/k2p6
phases: 6
created: 2026-06-12
start_phase: 1
yolo: true
source_files_during_phase_1:
  - darkmatter/lib/src/catalog/mod.rs
  - darkmatter/lib/src/lib.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - darkmatter/lib/src/catalog/mod.rs
  - darkmatter/lib/src/markdown/compose/context/catalog.rs
  - darkmatter/lib/src/markdown/compose/expression/catalog.rs
  - darkmatter/lib/src/effects/catalog.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
packages:
  - darkmatter
---

# Catalog Drift Control & Runtime-Accessible Descriptions — Execution Plan

## Phase 1 — Framework core

Build the shared descriptor module with no runtime coupling to the evaluator or `EffectEngine`.

- [x] Create `darkmatter/lib/src/catalog/mod.rs` containing:
  - `Described` trait with `key`, `description`, `category`, `order`, and `example` methods.
  - `Example` struct with `invocation` and `result` static strings.
  - `describe(catalog, key)` exact lookup.
  - `suggest(catalog, key, max)` fuzzy nearest-match lookup.
  - `describe_for_error(d)` plain-text formatter.
- [x] Implement in-crate Levenshtein distance in `catalog/mod.rs` (~30 lines, no new dependencies).
- [x] Add unit tests for `describe` exact matches and misses across a small test catalog.
- [x] Add unit tests for `suggest` ranking, tie-breaking, `max == 0`, parenthesis stripping, and `ctx.` prefix stripping.
- [x] Add unit tests for `describe_for_error` output shape.
- [x] Wire `catalog` module into `darkmatter/lib/src/lib.rs`.
- [x] Run `cargo test` for `darkmatter/lib` and confirm all new tests pass.
- [x] Run `cargo check` for the workspace and confirm no new warnings from the module.

**Validation checkpoint:** `cargo test -p darkmatter-lib catalog` passes with 100% of the new unit tests green.

**Parallelizable:** The unit-test fixture catalogs can be written in parallel, but the module itself is sequential.

---

## Phase 2 — Adopt + anchor the three catalogs

Add `example` fields and `impl Described` to the existing context, expression, and effect descriptors, plus executable anchors.

### Context variables

- [x] Add `example: Option<Example>` to `ContextVariableDescriptor` in `darkmatter/lib/src/markdown/compose/context/catalog.rs`.
- [x] Implement `Described` for `ContextVariableDescriptor`.
- [x] Add one illustrative `Example` per descriptor where the `result` string is type-consistent with `display_type`.
- [x] Write `cfg(test)` `capture_value_shape_matches_display_type` that captures one `ComposeContext` and asserts every descriptor's JSON value shape matches `display_type` (including `Nullable(T)` semantics).
- [x] Write `cfg(test)` `context_example_results_are_type_consistent` that asserts each `Example.result` string matches the `display_type` shape rules.
- [x] Run `cargo test` for context catalog tests and confirm green.

### Expression functions

- [x] Add `example: Option<Example>` to `ExpressionFunctionDescriptor` in `darkmatter/lib/src/markdown/compose/expression/catalog.rs`.
- [x] Implement `Described` for `ExpressionFunctionDescriptor`.
- [x] Add executable `Example` entries for every pure function.
- [x] Mark filesystem functions as display-only if their examples cannot be pinned to a tempdir fixture; document the reason beside each such descriptor.
- [x] Prepare a `cfg(test)` tempdir fixture with a known `.md` file and a known plain file for fs-function examples.
- [x] Add executable `Example` entries for fs functions referencing the tempdir fixture.
- [x] Mark date-relative functions as display-only when their only useful example depends on wall-clock time; document the reason.
- [x] Write `cfg(test)` `every_example_evaluates_to_its_declared_result` that parses each `example.invocation`, runs it through `evaluate`, and asserts rendered output equals `example.result`.
- [x] Run `cargo test` for expression catalog tests and confirm green.

### Side effects

- [x] Add `example: Option<Example>` to `EffectDescriptor` in `darkmatter/lib/src/effects/catalog.rs`.
- [x] Implement `Described` for `EffectDescriptor`.
- [x] Add one display `Example` per effect descriptor.
- [x] Reconcile with the existing in-code contract: `effects/mod.rs` + `effects/catalog.rs` already document the descriptor catalog as the authoritative capability surface (a method without a descriptor is intentionally outside the surface). Either reference that contract or, if a discoverable list still earns its keep, define `const INTENTIONALLY_UNCATALOGUED: &[&str]` naming `EffectEngine` methods deliberately outside the capability surface.
- [x] Note: `EffectVerb`/`EFFECT_VERBS` is now `cfg(test)` and the no-probe counters live behind the off-by-default `effects-instrumentation` cargo feature — enable that feature for any test asserting "no `EffectEngine` constructed".
- [x] Update existing `verb_signature_set_equals_descriptor_signature_set` to additionally assert every descriptor carries an example.
- [x] Run `cargo test` for effects catalog tests and confirm green.

### Closure

- [x] Run full `cargo test -p darkmatter-lib` and confirm no regressions.

**Validation checkpoint:** All three catalogs implement `Described`; every catalog has examples; all three executable-anchor tests pass.

**Parallelizable:** Context, expression, and effect descriptor work can be done in parallel once Phase 1 is merged.

---

## Phase 3 — Promote expression semantics

Move the literal semantics arrays out of `claudine/cli/src/commands/context.rs` into typed Darkmatter catalogs anchored to the parser.

- [ ] Create `darkmatter/lib/src/markdown/compose/expression/semantics.rs` with:
  - `OperatorDescriptor` struct and `OPERATOR_DESCRIPTORS` catalog.
  - `TRUTHINESS_DESCRIPTORS`, `MODE_DESCRIPTORS`, `NULL_PROPAGATION_DESCRIPTORS` catalogs.
  - Unary operator, comparison, arithmetic, and variable-access semantics catalogs.
  - `impl Described` for every semantics descriptor type.
- [ ] Expose the parser's precedence table as `pub(crate) const` in `darkmatter/lib/src/markdown/compose/expression/parser.rs`.
- [ ] Write `cfg(test)` `operator_precedence_matches_parser` asserting `OPERATOR_DESCRIPTORS` precedence equals the parser's precedence table.
- [ ] Write `cfg(test)` `semantics_examples_evaluate_correctly` executing curated examples for precedence, truthiness, mode behavior, and null propagation.
- [ ] Refactor `claudine/cli/src/commands/context.rs` to render precedence, truthiness, mode, null propagation, unary/comparison/arithmetic, and variable-access sections by iterating the new catalogs instead of using literal arrays.
- [ ] Keep presentation helpers and section order local to the CLI; remove any parallel semantic row literals.
- [ ] Add `Example` column to the expression-function and semantics reports where layout permits, staying within the 140-column layout contract.
- [ ] Run `cargo test` for `darkmatter-lib` and `claudine-cli`.
- [ ] Run `claudine context` (or equivalent) and visually confirm reports still render correctly.

**Validation checkpoint:** No literal semantics arrays remain in `context.rs`; `operator_precedence_matches_parser` passes; `claudine context` reports look correct.

**Parallelizable:** The `semantics.rs` catalog definitions and the CLI rendering refactor can be drafted in parallel, but they must integrate before the phase closes.

---

## Phase 4 — Error-message enrichment

Wire did-you-mean + verified example into expression evaluation and context compose-time diagnostics.

### Expression evaluation errors

- [ ] In `darkmatter/lib/src/markdown/compose/expression/mod.rs` `evaluate`, update the `Unknown function: <name>` path to call `suggest(...)` on `EXPRESSION_FUNCTION_DESCRIPTORS`.
- [ ] Append the nearest match's signature, description, and verified example using `describe_for_error`.
- [ ] Update arity errors to call `describe(...)` on the matched function and append correct signature + example.
- [ ] Add unit tests for unknown-function did-you-mean (e.g., `uper` → `upper(x)`).
- [ ] Add unit tests for arity error enrichment.

### Context typo diagnostic

- [ ] Add a non-fatal diagnostic type/API in Darkmatter's composition preparation path for unknown `ctx.<name>` references.
- [ ] Implement parser-aware detection (not regex-only) so `{{ "ctx.toady" }}` does not warn while `{{ ctx.toady }}` does.
- [ ] Use `suggest(...)` on `CONTEXT_VARIABLE_DESCRIPTORS` to produce nearest-match warnings with `describe_for_error`.
- [ ] Expose the diagnostic list to Claudine for rendering.
- [ ] In `claudine-cli`, render the diagnostics to stderr via `biscuit-terminal` components on normal composition preparation paths.
- [ ] Respect existing non-fatal warning suppression flags (`--silent` or equivalent).
- [ ] Add tests proving silent-null evaluation semantics are unchanged.
- [ ] Add tests proving quoted-string/code-fence false positives are excluded.

### Side effects report

- [ ] Add an `Example` column to the `--side-effects` report in the CLI.
- [ ] Render examples from `EFFECT_DESCRIPTORS` via `describe_for_error` formatting rules.

### Closure

- [ ] Run full `cargo test -p darkmatter-lib -p claudine-cli` and confirm green.
- [ ] Run a manual end-to-end test invoking an unknown function and a ctx typo to verify enriched output.

**Validation checkpoint:** Unknown-function and arity errors show did-you-mean + verified example; ctx-typo diagnostic warns without breaking silent-null; `--side-effects` report includes examples.

**Parallelizable:** Expression error wiring, context diagnostic wiring, and side-effects report column can be developed in parallel.

---

## Phase 5 — Narrative-doc parity (cuttable)

Optionally close the final manual surface by generating the function table in `darkmatter/docs/topics/darkmatter-expressions.md` from the catalog.

- [ ] Decide whether to keep or cut this phase based on team bandwidth.
- [ ] If kept, mark generated regions in `darkmatter/docs/topics/darkmatter-expressions.md` with clear start/end markers.
- [ ] Write a generator script (or hidden CLI flag) that emits the function table markdown from `EXPRESSION_FUNCTION_DESCRIPTORS`.
- [ ] Add a `just` recipe `just darkmatter regen-expr-doc` that runs the generator.
- [ ] Write `cfg(test)` `narrative_doc_function_table_matches_catalog` that compares only the generated region against the catalog output.
- [ ] Run the generator and commit the regenerated doc region.
- [ ] Run `cargo test` and confirm the parity test passes.

**Validation checkpoint:** `just darkmatter regen-expr-doc` regenerates the doc region; `narrative_doc_function_table_matches_catalog` passes.

**Parallelizable:** The generator script and the doc-region markers can be prepared in parallel.

---

## Phase 6 — Recast drift model and docs

Update documentation to reflect the closed gaps and the new shared framework.

- [ ] Update `claudine/docs/topics/context/drift.md` with the new drift-layer model (Layer 3 closed for context/expression semantics, reviewed allow-list for effects).
- [ ] Update the three `context/` topic docs to mention runtime-accessible descriptions and the enriched error surfaces.
- [ ] Document the `INTENTIONALLY_UNCATALOGUED` allow-list and the review process for adding methods to it.
- [ ] Add a short README/note in `darkmatter/lib/src/catalog/` explaining when to use `Described` and how to add a new catalog.
- [ ] Run `cargo doc -p darkmatter-lib --no-deps` and confirm no broken intra-doc links.
- [ ] Run `cargo test` one final time across the touched crates.
- [ ] Run `claudine context` and `--side-effects` reports for a final visual regression check.

**Validation checkpoint:** Docs accurately describe the new state; doc tests pass; build is green; reports render within the 140-column contract.

**Parallelizable:** Doc updates for drift.md, topic docs, and catalog README can be written in parallel.

---

## Cross-cutting concerns

- [ ] Every phase boundary must leave `cargo test` green for `darkmatter-lib` and `claudine-cli`.
- [ ] No new external dependencies (Levenshtein stays in-crate).
- [ ] All error-path text remains plain text in Darkmatter; Claudine owns terminal styling.
- [ ] Keep the existing 140-column layout contract for `claudine context` reports.
- [ ] Preserve null-propagation semantics for unknown `ctx.*` access.
