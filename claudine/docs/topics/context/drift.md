# Drift Control

The `claudine context` reports document a moving target: Darkmatter's context
variables, expression functions, and side-effect capabilities all evolve. This
doc explains the mechanism that keeps the CLI output faithful to the
implementation, exactly where drift can still occur, and what could be done to
close those gaps.

## The pattern

Every report is built from a **typed descriptor catalog** that Darkmatter
exports, and the CLI renders that catalog directly:

| Subsystem | Catalog (Darkmatter) | Accessor |
|-----------|----------------------|----------|
| Context variables | `CONTEXT_VARIABLE_DESCRIPTORS` | `context_variable_descriptors()` |
| Expression functions | authored `expression-functions.yaml` catalog | `expression_function_descriptors()` |
| Expression semantics | `OPERATOR_DESCRIPTORS`, `TRUTHINESS_DESCRIPTORS`, `MODE_DESCRIPTORS`, `NULL_PROPAGATION_DESCRIPTORS`, `UNARY_OPERATOR_DESCRIPTORS`, `COMPARISON_OPERATOR_DESCRIPTORS`, `ARITHMETIC_OPERATOR_DESCRIPTORS`, `VARIABLE_ACCESS_DESCRIPTORS` | accessors in `expression::semantics` |
| Side-effect capabilities | `EFFECT_DESCRIPTORS` | `effect_descriptors()` |

Each catalog is static data — reading it does no I/O and captures no context.
The CLI imports the accessor, groups the descriptors, and folds them into
biscuit-terminal tables. It keeps **no parallel list of its own**.

Every descriptor also implements the shared [`Described`] trait from
`darkmatter::catalog`, so consumers can perform exact lookups, fuzzy
nearest-match suggestions, and plain-text error enrichment without depending on
any specific catalog type.

## Three layers of drift

Think of drift as three gaps between "what the CLI prints" and "what the code
does." The design closes the first two and most of the third.

### Layer 1 — CLI ↔ catalog: closed structurally

The report *is* the catalog. There is no second list to forget to update. Add a
descriptor in Darkmatter and it appears in the CLI on the next build, grouped
and styled, with zero CLI edits. This gap cannot open as long as the CLI keeps
reading the accessor rather than hard-coding rows.

### Layer 2 — catalog ↔ runtime: closed structurally

Each subsystem proves, in-crate, that its catalog matches the real runtime
surface — in both directions:

| Subsystem | Parity test(s) | What it pins |
|-----------|----------------|--------------|
| Context | `descriptor_name_set_equals_captured_runtime_key_set` | descriptor **names** == captured `ctx.*` keys |
| Context examples | `capture_value_shape_matches_display_type`, `context_example_results_are_type_consistent` | declared `display_type` matches captured JSON shape |
| Expression functions | registration invariants + `every_descriptor_overload_is_dispatchable_at_its_declared_arity` | descriptors and handlers share one registration (overload-aware), and each is callable at its arity |
| Expression semantics | `operator_precedence_matches_parser` + per-catalog `*_examples_evaluate_correctly` | precedence table matches parser; every example evaluates to declared result |
| Side effects | `verb_signature_set_equals_descriptor_signature_set` + `every_verb_maps_to_a_reachable_method` | each descriptor **signature** is backed by a real, reachable `EffectEngine` method |

Expression descriptors and runtime handlers are fields of the same domain-owned
registration. Dispatch and catalog projection therefore read the same data, and
invariant tests reject name, alias, and signature collisions.

A separate, related guard protects the **documentation-only** contract: the
`effects-instrumentation` counters and the CLI test
`metadata_reports_construct_no_engine_and_attempt_no_network` ensure the reports
never construct an effect engine or hit the network (see
[Side Effects](side-effects.md#documentation-only-guarantee)).

### Layer 3 — catalog *content* ↔ actual behavior: mostly closed

Layers 1 and 2 pin **structure** — the set of names and signatures. Layer 3 pins
**content** as well, through a shared `example` field on every descriptor type.

- **Examples are verified.** Every descriptor can carry an `Example` with an
  `invocation` and a declared `result`. In-crate tests run each example through
  the real evaluator or effect engine and assert the rendered output matches.
  A stale description or signature therefore breaks the build.

- **Context type labels are checked.** The context catalog's `display_type` is
  validated against the JSON shape produced by `ComposeContext::capture`,
  including `Nullable(T)` semantics.

- **Expression language semantics are catalog-driven.** Operator precedence,
  truthiness, comparison/arithmetic rules, variable access, parse modes, and
  null propagation all live in typed descriptor catalogs anchored to the parser
  (`operator_precedence_matches_parser`) and evaluator (`*_examples_evaluate_correctly`).
  The `--expressions` report renders these catalogs directly; no hand-written
  literal arrays remain in the CLI.

- **Side-effect orphan methods are explicitly reviewed.** Public `EffectEngine`
  methods that are deliberately outside the capability surface are listed in
  `INTENTIONALLY_UNCATALOGUED` in `effects/catalog.rs`. The list is currently
  empty, which is itself an asserted state: every public mutating method is
  either catalogued or explicitly reviewed and documented as excluded.

The remaining content drift surfaces are minor:

- **Prose descriptions.** The human-readable `description` on each descriptor is
  still free text. It is, however, anchored by the verified example: a
  description that drifts from behavior will typically be noticed when the
  example is read or updated.

- **The darkmatter narrative doc.** `darkmatter/docs/topics/darkmatter-expressions.md`
  contains narrative prose, but its function table is generated from
  `expression_function_descriptors()` by `just darkmatter regen-expr-doc` and is
  guarded by the parity test `narrative_doc_function_table_matches_catalog`.

## Next steps

Concrete ways to push the remaining drift surfaces toward fully closed:

1. **Snapshot the rendered `--expressions` and `--side-effects` reports.** Add a
   test that renders the reports at representative widths and asserts the output
   matches a checked-in snapshot. This would catch layout drift or accidental
   column changes in addition to content drift.

2. **Generate more narrative prose from catalogs.** Where possible, derive the
   darkmatter topic doc's language sections (precedence, truthiness, etc.) from
  the same catalogs that drive the CLI, leaving only higher-level explanatory
  prose to be maintained by hand.

3. **Keep `INTENTIONALLY_UNCATALOGUED` reviewed.** Any new public `EffectEngine`
   method must either earn a descriptor or be added to the allow-list with a
   written rationale. Treat the allow-list as a code-review checklist.

## Source map

| Concern | Location |
|---------|----------|
| Shared descriptor framework | `darkmatter/lib/src/catalog/mod.rs` |
| Context catalog + parity tests | `darkmatter/lib/src/markdown/compose/context/catalog.rs` |
| Context capture | `darkmatter/lib/src/markdown/compose/context/capture.rs` |
| Expression catalog + parity tests | `darkmatter/lib/src/markdown/compose/expression/catalog.rs` |
| Expression semantics catalogs | `darkmatter/lib/src/markdown/compose/expression/semantics.rs` |
| Expression runtime registry | `darkmatter/lib/src/markdown/compose/expression/functions.rs` |
| Effects catalog + verb harness | `darkmatter/lib/src/effects/catalog.rs` |
| Effects instrumentation | `darkmatter/lib/src/effects/mod.rs` |
| CLI report rendering + CLI-level tests | `claudine/cli/src/commands/context.rs`, `context_render.rs` |
