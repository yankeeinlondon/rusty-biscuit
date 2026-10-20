---
$schema: feature-review.yaml
ready: true
findings: []
observations: []
human_review: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: 2026-10-12T15:07:56-07:00
spec: 2026-10-16-core-schema-types/spec.md
implemented: false
description: "A **feature** review of `2026-10-16-core-schema-types/spec.md`"
feature: 2026-10-16-core-schema-types/review-1.md
---

# Core Schema Types — Review 1

**Production ready: yes.** No high findings. Two medium findings concern
unusual uses of the public serialization and subtype APIs; neither blocks
readiness. No human decision is needed to close this review.

Reviewed the specification, implementation log, changed schema code and its
consumers in Darkmatter, DMLS, and Claudine. The review leaves implementation
code unchanged. The specification's review count and completion flag are
updated; the author retains control of its lifecycle directory.

## Findings

### Medium — Serializing an unresolved reference can erase its option-set check

**Authority: spec.** Acceptance criterion 13 requires that a serializer round
trip “preserves meaning and emits canonical options.” The named-type decision
also requires that “writing a different option set at a use site” is a schema
error. An unusual reference with only redundant restriction options changes
from a rejected declaration to an accepted declaration after serialization.

**Defect class:** canonicalization discards an explicit option-set assertion
before resolving the named type against which that assertion must be checked.

In the Darkmatter package,
[`serialize_property_atom`](../../../darkmatter/lib/src/markdown/schemas/simplified/serialize.rs)
converts a parsed type back to a type expression. Its `render_constraints`
helper canonicalizes options for both primitive types and unresolved
references, then removes the original option constraint. With this catalog:

```yaml
kind: schema
types:
  slug: alphanumeric(allow-dashes)
```

`slug(block-leading-dash)@./types.yaml` correctly fails resolution: its
canonical option set is empty, which differs from the named definition's
`allow-dashes` set. Serialization produces `slug@./types.yaml`, which loads
successfully because it no longer asserts an option set. Retaining an ordinary
constraint does not help: `slug(block-leading-dash; max(10))@./types.yaml`
becomes `slug(max(10))@./types.yaml` and also loads.

The sweep used the public parser, serializer, and `resolve_yaml_schema` API
against a temporary copy of the catalog shape used by the existing named-option
tests. Every reference row below was checked separately with
`block-leading-dash`, `block-leading-slash`, and `block-double-periods` as the
sole option; all three reproduced the same result.

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| Primitive scalar | `alphanumeric(<restriction>)` | Both original and serialized declarations load with the same meaning | Same; clean |
| Primitive array | `alphanumeric(<restriction>)[]` | Both load with the same meaning | Same; clean |
| Bare local reference | `slug(<restriction>)` | Original fails; serialized `slug` loads | Preserve the conflict |
| Local array reference | `slug(<restriction>)[]` | Original fails; serialized `slug[]` loads | Preserve the conflict |
| Imported scalar reference | `slug(<restriction>)@./types.yaml` | Original fails; serialized reference loads | Preserve the conflict |
| Imported array reference | `slug(<restriction>)[]@./types.yaml` | Original fails; serialized reference loads | Preserve the conflict |
| Reference retaining a constraint | `slug(<restriction>; max(10))@./types.yaml` | Original fails; serialized reference loads | Preserve the conflict |
| Inline object, local child | `{ child: slug(<restriction>) }` | Original fails; serialized object loads | Preserve the conflict |
| Inline object, imported child | `{ child: slug(<restriction>)@./types.yaml }` | Original fails; serialized object loads | Preserve the conflict |
| Inline object array, imported child | `{ child: slug(<restriction>)@./types.yaml }[]` | Original fails; serialized array loads | Preserve the conflict |
| Matching option-set control | `slug(allow-dashes)@./types.yaml` | Both load with the same meaning | Same; clean |

The primitive, named-reference, and recursive inline-object serializer branches
are covered by this sweep. Resolved union display output is explicitly outside
the serializer's parseable round-trip contract. Existing production consumers
serialize inferred primitive types or already resolved types, so the failing
unresolved assertion does not currently reach their normal output.

Keep this nonblocking. A focused improvement would preserve authored options
on unresolved references until resolution supplies their identity; if canonical
output for unresolved references is retained, document this limitation rather
than introducing a new serialized assertion syntax just for this case. Add a
round-trip conflict test if this behavior is repaired.

### Medium — Structured subtype identity compares authored details

**Authority: spec.** The subtype decision says “Ordinary constraints
(`min`, `max`, `pattern`, …) are ignored” and “Two identical types (the same
type with the same canonical options) are always subtypes of each other,
whatever the type.” Acceptance criterion 6 requires the corresponding identity
and constraint-independent checks. These rules work for primitive atoms but
not when equivalent atoms occur inside an object or resolved union.

**Defect class:** structured type identity uses raw syntax-tree equality
instead of the canonical, constraint-independent identity used for primitives.

In the Darkmatter package,
[`is_subtype`](../../../darkmatter/lib/src/markdown/schemas/simplified/subtype.rs)
is the passive public function intended for later command-approval consumers.
`Identity::Structured` compares `TypeExpr` values directly (line 133). Those
values include child constraints and authored option/member order. For example,
the public parser and subtype API report `false` for:

```text
{ p: alphanumeric(allow-dashes, allow-periods) }
    ⊆ { p: alphanumeric(allow-periods, allow-dashes) }
```

The character sets are identical. The same problem occurs when a child merely
adds `min(1)`, or when an equivalent union is expanded through a named reference.

The sweep exercised both structured variants and their array forms through
the public API. Named unions were resolved before comparison; option-bearing
YAML union arms were quoted so commas stayed inside their type expressions.

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| Primitive scalar | `alphanumeric(allow-dashes; min(1))` vs. `alphanumeric(allow-dashes)` | `true` | `true`; clean |
| Primitive array | `string(min(1))[]` vs. `string[]` | `true` | `true`; clean |
| Primitive enum | `enum(a,b)` vs. `enum(b,a)` | `true` | `true`; clean |
| Inline object | Child changes only `min(1)` | `false` | `true` after ignoring ordinary constraints |
| Inline object | Child alphanumeric options change only order | `false` | `true` for the same canonical options |
| Inline object | Child base alphanumeric adds redundant `block-leading-dash` | `false` | `true` for the same canonical options |
| Inline object | Child enum members change only order | `false` | `true`, as for the primitive enum control |
| Inline object control | Identical `literal(3)` child | `true` | `true`; clean |
| Inline object array | Child changes only `min(1)` | `false` | `true` for identical underlying types |
| Inline object array control | Only outer `[](min(1))` differs | `true` | `true`; clean |
| Resolved scalar union | Alphanumeric arm changes only option order | `false` | `true` for identical underlying types |
| Resolved union array | Same option-order edit | `false` | `true` for identical underlying types |
| Resolved scalar union | String arm changes only `min(1)` | `false` | `true` after ignoring ordinary constraints |
| Resolved union array | Same constraint-only edit | `false` | `true` after ignoring ordinary constraints |
| Resolved union controls | Identical `[string, number]`, scalar and array | `true` for both | `true`; clean |

No production call site currently invokes this function; the planned approval
consumer compares scalar string-like types, where the implementation is correct.
This is therefore medium rather than a release blocker. Document the structured
identity limitation for now. A recursive identity model would be relatively
large compared with its current benefit; revisit it when a consumer needs these
comparisons, and keep that work separate from general structural subtyping.

## Verification

All feature requirements need **Level 1** verification: parser and validator
results, hermetic CLI invocations, composition, or language-server protocol
responses. No requirement depends on terminal rendering or keyboard encoding,
so there is no Level 2 or Level 3 mismatch.

| Spec requirement | Strongest relevant verification inspected and run |
|---|---|
| Phase A validation table, lowering, nullable unions, native YAML null (criteria 1–4) | L1 `schema_validate_core_types`, `schemas_effective_presence`, core grammar tests, and converter tests |
| Alphanumeric options, constraints, no scalar coercion, exported-schema agreement (criteria 5 and 13) | L1 `schemas_core_type_values`, CLI `schema_robustness_matrix`, grammar conflicts, and serializer round trips |
| Scalar subtype rules and identical array types (criterion 6) | L1 subtype unit tests and public-API test; additional public-API probes exposed the structured limitation above |
| Passive shell values and closed lifecycle action alternatives (criterion 7) | L1 library validation/composition, shipped CLI validation, and DMLS lifecycle diagnostics; existing Claudine initialization tests ran |
| Trigger and side-effect migration (criterion 8) | L1 shipped-schema corpus, `side_effect_partial`, CLI trigger selection including malformed `steps`, and shipped-artifact CLI validation |
| Completion, hover, option diagnostics, descriptor parity (criteria 9–10) | L1 DMLS `core_schema_types` over unsaved buffers, exact ranges and related locations; CLI `schema_about` and descriptor parity tests |
| Presence across validation, triggers, composition, launch/completion, and raw JSON controls (criterion 12) | L1 `schemas_effective_presence`, `schemas_runtime_parity`, and Claudine `compose_presence_parity` |
| Null defaults, local/imported definitions, meta-types, and native unknown values (criterion 14) | L1 core grammar/value tests, CLI declaration/instance robustness matrix, and DMLS native-null diagnostics |
| Required package gates, corpus, ordinary invocation path, test placement (criterion 11) | Gates below; new modules declared in consolidated L1 targets; no stranded Darkmatter tier tests |

The YAML robustness tests cover declaration and instance columns, positive
controls, absence, explicit null, wrong whole-field and element types, empty
values, duplicate keys, and invalid trailing content. The implementation log
records the matrix's deliberate departures from the plan. No additional
matrix finding was identified. The two findings above concern a serializer
and a type relation, not configuration-reader matrix cells.

| Review-time command | Result on macOS |
|---|---|
| `cd darkmatter && just test` | Exit 0; 9,625 passed, 11 skipped |
| `cd darkmatter && just lint` | Exit 0, including the Zed WASM extension check |
| `cd claudine && just test` | Exit 0; 8,694 passed, 9 skipped |
| `just check-tier-coverage darkmatter` | Exit 0; no stranded tests |

Temporary public-API probes used `parse_type_expr`, `serialize_property_atom`,
`resolve_yaml_schema`, and `is_subtype`; the probe source was removed afterward.
The implementation log's previous gate results were not substituted for these
review-time runs. Cross-OS evidence remains CI's responsibility and does not
block this review.

## Observations

None.
