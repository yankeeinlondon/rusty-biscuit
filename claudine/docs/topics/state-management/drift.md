# Drift Control

**Documentation drift** happens when a description no longer matches the
software. For example, a report might list a function that was removed, show
the wrong number of arguments, or promise a result the function no longer
returns.

Claudine reduces this risk by building its `context` reports from descriptions
provided by Darkmatter, the library that implements the capabilities. Tests
check those descriptions against parts of the implementation. **The checks
cover names, types, signatures, and selected behavior; explanatory prose and
some omissions still need human review.**

This page explains those guarantees for contributors maintaining
[context variables](context-variables.md), [expressions](expression-engine.md),
and [side effects](side-effects.md).

## One catalog feeds each report

Darkmatter publishes a **descriptor catalog**: structured records containing
names, descriptions, type or argument information, and other metadata. Claudine
reads those records to build the report tables.

For example, adding an expression function to Darkmatter's catalog makes its
entry available to `claudine context --expressions` on the next build. There is
no separate list of function rows in the CLI to update.

| Report content | Where its descriptions come from |
|---|---|
| Context variables | The embedded base schema's `ctx` definitions, with presentation groups supplied in Rust. |
| Expression functions | The authored `expression-functions.yaml` catalog. |
| Expression language rules | Rust descriptor catalogs for operators, truthiness, access rules, and parse modes. |
| Side effects | The `EFFECT_DESCRIPTORS` catalog in Rust. |

Reading these catalogs does not capture host context or execute the capabilities
they describe. Some catalogs are derived from embedded data on first access;
they are not all simple compile-time arrays.

Sharing a catalog prevents a separately maintained CLI list from going stale.
It does not prove the catalog is complete, its prose is accurate, or the
renderer displays every field correctly.

## What the automated checks establish

### Context variables: schema, descriptions, and captured values agree

The context catalog is derived from the base schema. Tests check that the
projection preserves the schema's names, types, descriptions, flags, defaults,
and ordering. They also check that every variable has a presentation group.

Separate tests compare catalog names with the keys in a captured context and
check the general JSON shapes of the resulting values. For example, an array
variable should produce an array, and an optional value may be `null`.

These checks use captured test environments. They do not demonstrate that every
variable is correct on every machine. Context descriptors currently carry no
example records, so there is no catalog-example execution guarantee for them.

### Expressions: documented functions can be called

Function descriptions and executable handlers are maintained separately, then
joined by canonical name in the registry. Registry checks reject duplicate
names, alias collisions, and names missing from either side.

Behavior tests go further: they call documented signatures with the declared
number of arguments and evaluate function examples against their expected
results. Language-rule tests also compare operator precedence with the parser
and evaluate examples for rules such as truthiness and missing-value handling.

This catches a missing handler or a broken example. It does not automatically
catch a misleading sentence whose associated example still passes.

### Side effects: catalog entries have reachable methods

A test table pairs each documented side-effect signature with a call to a real
`EffectEngine` method. Tests compare that table with the catalog and exercise
the calls in a sandbox. For HTTP, the test checks that the default host policy
refuses the request, without sending it.

The catalog's displayed examples are marked `DisplayOnly`. The tests require
an example to exist, but do not execute each displayed invocation and compare
its result. The separate method-call tests establish reachability.

There is also a coverage gap: **a public method omitted from both the catalog
and the test table is invisible to these checks**. The
`INTENTIONALLY_UNCATALOGUED` list records deliberate exclusions, but its being
empty does not prove that no method was accidentally omitted. Reviewers must
check new public mutating methods.

### Generated documentation matches its source

The function table in
[Darkmatter Expressions](../../../../darkmatter/docs/topics/darkmatter-expressions.md)
is generated from the function catalog. A test checks that the committed table
matches that output. Regenerate it with:

```sh
just darkmatter regen-expr-doc
```

The surrounding explanations are maintained by hand.

## The reports must not execute side effects

This is a related but separate guarantee: looking up a capability must not run
it. The default, expression, and side-effect reports are documentation; only
`--values` captures live context.

A CLI test enables Darkmatter's optional effects instrumentation and checks that
rendering the three documentation reports neither constructs an `EffectEngine`
nor enters `http_post`. The latter counter includes requests refused by the
host policy. This is a targeted check of the effects engine, rather than a
process-wide audit of every possible I/O path.

See [Side Effects: Documentation-only guarantee](side-effects.md#documentation-only-guarantee)
for the instrumentation details. A separate test checks that the values report
captures context exactly once.

## What still needs review

When changing a capability, check its description alongside its implementation.
In particular:

- Does the prose explain the current behavior, including missing values,
  restrictions, and return values?
- Do examples cover the behavior that changed, rather than only a case that
  still happens to pass?
- Does a new public side-effect method have a descriptor and test-table entry,
  or a documented reason for exclusion?
- Does the report remain readable at supported terminal widths?

The shared catalog removes duplicate maintenance. Tests provide evidence for
specific contracts. Neither replaces review of the reader-facing explanation.

## Next steps

Possible improvements include broader behavior coverage for side-effect
examples and generating more reference tables from the language-rule catalogs.
Both would reduce the amount that must be checked manually.

Rendered-output snapshots at representative widths could supplement the
existing layout tests by making column and presentation changes easy to review.
A snapshot would capture a change in output, not prove that the output is
correct.

For now, keep reviewing `INTENTIONALLY_UNCATALOGUED` whenever a public mutating
method is added. Intentional exclusions need a written rationale; accidental
omissions cannot be found by the current two-table comparison.

## Implementation reference

### Main checks

| Area | Tests or checks |
|---|---|
| Context schema projection | `projected_descriptors_match_base_schema`, `grouping_map_is_total` |
| Context capture | `every_descriptor_has_a_captured_runtime_key`, `capture_shape_matches_projected_type` |
| Expression registration and calls | Registry invariants; `every_descriptor_overload_is_dispatchable_at_its_declared_arity` |
| Function examples | `every_example_evaluates_to_its_declared_result` |
| Language rules | `operator_precedence_matches_parser`; semantics `*_examples_evaluate_correctly` tests |
| Side-effect catalog and calls | `verb_signature_set_equals_descriptor_signature_set`, `every_verb_maps_to_a_reachable_method` |
| Generated function table | `narrative_doc_function_table_matches_catalog` |
| Documentation reports | `metadata_reports_construct_no_engine_and_attempt_no_network` |
| Live report capture | `values_report_captures_context_exactly_once` |

### Source map

| Concern | Location |
|---|---|
| Shared descriptors and example verification modes | [Catalog framework](../../../../darkmatter/lib/src/catalog/mod.rs) |
| Context schema | [Embedded base schema](../../../../darkmatter/docs/schemas/darkmatter.yaml) |
| Context descriptors and checks | [Context catalog](../../../../darkmatter/lib/src/markdown/compose/context/catalog.rs) |
| Runtime context values | [Capture modules](../../../../darkmatter/lib/src/markdown/compose/context/capture/) |
| Expression descriptions and checks | [Expression catalog](../../../../darkmatter/lib/src/markdown/compose/expression/catalog/) |
| Language-rule descriptors | [Expression semantics](../../../../darkmatter/lib/src/markdown/compose/expression/semantics.rs) |
| Executable expression functions | [Function modules](../../../../darkmatter/lib/src/markdown/compose/expression/functions/) |
| Side-effect descriptions and test calls | [Effects catalog](../../../../darkmatter/lib/src/effects/catalog.rs) |
| Effects instrumentation | [Effects engine](../../../../darkmatter/lib/src/effects/mod.rs) |
| Report rendering and tests | [Context command](../../../cli/src/commands/context/) |
| Shared report layout | [Context renderer](../../../cli/src/commands/context_render.rs) |
