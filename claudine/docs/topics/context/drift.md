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
| Expression functions | `EXPRESSION_FUNCTION_DESCRIPTORS` | `expression_function_descriptors()` |
| Side-effect capabilities | `EFFECT_DESCRIPTORS` | `effect_descriptors()` |

Each catalog is a pure `const` — reading it does no I/O and captures no context.
The CLI imports the accessor, groups the descriptors, and folds them into
biscuit-terminal tables. It keeps **no parallel list of its own**.

## Three layers of drift

Think of drift as three gaps between "what the CLI prints" and "what the code
does." The design closes the first two and leaves the third partly open.

### Layer 1 — CLI ↔ catalog: closed structurally

The report *is* the catalog. There is no second list to forget to update. Add a
descriptor in Darkmatter and it appears in the CLI on the next build, grouped
and styled, with zero CLI edits. This gap cannot open as long as the CLI keeps
reading the accessor rather than hard-coding rows.

### Layer 2 — catalog ↔ runtime: closed by parity tests

Each subsystem proves, in-crate, that its catalog matches the real runtime
surface — in both directions:

| Subsystem | Parity test(s) | What it pins |
|-----------|----------------|--------------|
| Context | `descriptor_name_set_equals_captured_runtime_key_set` | descriptor **names** == captured `ctx.*` keys |
| Expression | `descriptor_signature_set_equals_dispatchable_signature_set` + `every_descriptor_overload_is_dispatchable_at_its_declared_arity` | descriptor **signatures** == dispatchable signatures (overload-aware), and each is callable at its arity |
| Side effects | `verb_signature_set_equals_descriptor_signature_set` + `every_verb_maps_to_a_reachable_method` | each descriptor **signature** is backed by a real, reachable `EffectEngine` method |

Add an item to one side only — the catalog or the runtime — and the build fails.
For the expression engine the runtime side is itself a single source of truth
(`PURE_FUNCTIONS` / `FS_FUNCTIONS` carry their own `signatures`), so the
dispatcher and the parity check read the same data.

A separate, related guard protects the **documentation-only** contract: the
`effects-instrumentation` counters and the CLI test
`metadata_reports_construct_no_engine_and_attempt_no_network` ensure the reports
never construct an effect engine or hit the network (see
[Side Effects](side-effects.md#documentation-only-guarantee)).

### Layer 3 — catalog *content* ↔ actual behavior: partly open

Layers 1 and 2 pin **structure** — the set of names and signatures. They do not
pin **content**: the human-readable `description`, the `display_type`, and any
prose the CLI prints alongside the catalog are still maintained by hand and have
no automated tie to behavior. This is where drift can still creep in.

The open gaps, by subsystem:

- **Context — type labels.** `descriptor_name_set_equals_captured_runtime_key_set`
  proves a variable *exists* but not that its declared `display_type` matches the
  JSON shape `capture.rs` actually produces. The recent `Nullable(String)` /
  `Nullable(Integer)` correction was made by hand, not surfaced by a failing
  test. A descriptor could claim `Integer` while capture inserts a `String` and
  nothing would complain.

- **Expression — language-semantics prose.** The `--expressions` *function table*
  is catalog-driven and safe, but the language sections (operator precedence,
  truthiness, unary/comparison/arithmetic semantics, mode table, null
  propagation) are **hand-written literal arrays in `context.rs`**. They mirror
  the lexer/parser/evaluator but are derived from no Darkmatter type, so a change
  to operator precedence or truthiness in the evaluator would not update — or
  break — this prose.

- **Side effects — orphan methods.** The verb harness cannot detect a public
  `EffectEngine` method that was never given a descriptor (Rust can't enumerate
  a type's method surface at compile time). Such a method is *intentionally*
  outside the capability surface, but the choice is silent rather than asserted.

- **Descriptions everywhere.** Every `description` field — and the darkmatter
  narrative doc `darkmatter/docs/topics/darkmatter-expressions.md`, a fourth,
  fully manual surface — is free text with no behavioral check.

## Next steps

Concrete ways to push Layer 3 toward closed, roughly in order of value:

1. **Type-parity test for context variables.** Add a capture-driven test that,
   for each descriptor, captures a `ComposeContext` and asserts the `Value`
   shape matches `display_type` — treating `Nullable(T)` as "`null` or a `T`".
   This would have caught the bare-`Nullable` bug automatically and is the
   single highest-value gap to close.

2. **Promote expression semantics into typed catalogs.** Move operator
   precedence, the truthiness table, the comparison/arithmetic rules, and the
   mode table out of `context.rs` literals and into Darkmatter descriptor
   catalogs (e.g. an operator catalog) the way functions already are. Then the
   `--expressions` semantics sections become catalog-driven (Layer 1) and can be
   parity-checked against the lexer/evaluator (Layer 2). Alternatively, snapshot
   the rendered semantics and assert representative cases against real
   `evaluate()` behavior.

3. **Assert intentional non-catalogued effect methods.** Maintain an explicit
   allow-list of `EffectEngine` methods deliberately excluded from the catalog,
   and a test that every other public method has a descriptor. This converts the
   silent orphan-method gap into a reviewed decision.

4. **Generate the darkmatter narrative doc from the catalog.** Render
   `darkmatter-expressions.md`'s function tables from
   `EXPRESSION_FUNCTION_DESCRIPTORS` (or check it in CI) so the prose reference
   and the typed catalog cannot diverge.

5. **Example-bearing descriptors.** Give descriptors an optional `example`
   field and doctest it against the real evaluator / engine, so descriptions are
   anchored to observable behavior rather than trusted prose.

## Source map

| Concern | Location |
|---------|----------|
| Context catalog + parity test | `darkmatter/lib/src/markdown/compose/context/catalog.rs` |
| Context capture | `darkmatter/lib/src/markdown/compose/context/capture.rs` |
| Expression catalog + parity tests | `darkmatter/lib/src/markdown/compose/expression/catalog.rs` |
| Expression runtime registry | `darkmatter/lib/src/markdown/compose/expression/functions.rs` |
| Effects catalog + verb harness | `darkmatter/lib/src/effects/catalog.rs` |
| Effects instrumentation | `darkmatter/lib/src/effects/mod.rs` |
| CLI report rendering + CLI-level tests | `claudine/cli/src/commands/context.rs`, `context_render.rs` |
