---
$schema: feature-review.yaml
ready: true
findings: []
observations:
    - Root-union namespace wording conflicts with retained behavior
    - Conflict messages omit the conflicting values
human_review: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: "2026-10-11T21:51:57-07:00"
spec: 2026-10-13-named-type-presence-and-local-names/spec.md
implemented: false
description: "A **feature** review of `2026-10-13-named-type-presence-and-local-names/spec.md`"
feature: 2026-10-13-named-type-presence-and-local-names/review-1.md
---

# Review 1

The feature is production ready. No high, medium, or low findings were identified. Presence inheritance, local-name resolution, constraint conjunction, editor assistance, and Claudine completion have appropriate Level-1 verification. No human-only validation or unresolved design decision is required.

Reviewed the implementation from base commit `f1d031f7a` through `b2cc72a00`, including the implementation log, current documentation, test declarations, and the existing uncommitted completion metadata. Production code was not changed during this review.

## Findings

None.

## Requirements and verification

All requirements below need Level 1: they concern schema results, hermetic filesystem behavior, command invocation, or language-server protocol responses. None requires terminal rendering or OS keyboard input. Editor tests send actual language-server requests through the in-memory protocol harness; CLI tests invoke the shipped command with isolated fixtures.

| Spec requirement | Strongest relevant verification reviewed | Level |
|---|---|---|
| Required and generated presence through bare, explicit local, and cross-file references; authored versus effective validation | `schemas_named_type_merge::required_and_generated_survive_every_reference_spelling`, CLI `schema_validate` and `schema_validate_phase` tests | 1 |
| Required expression at the top level and inside a mapping type | `required_expression_type_is_required_at_top_level_and_inside_a_mapping_type` | 1 |
| Presence combinations, optional definitions, multi-hop references, nested objects, arrays, and union arms | `each_presence_combination_is_inherited_as_declared`, `multi_hop_chains_inherit_presence_locally_and_across_files`, required-inner-field, required-array, and union-arm tests | 1 |
| Bounds, flags, patterns, schemes, defaults, examples, suggestions, and conflict origins | `schemas_named_type_merge`, `schemas_conjunctive_lowering`, and CLI conflict/pattern validation tests; assertions inspect both lowered output and actual validation | 1 |
| Bare/explicit equivalence, collisions, forward references, missing names, defining-file resolution, and cycles | Named-type resolver tests and grammar tests, including typed error assertions | 1 |
| Completion, hover, definition navigation, diagnostic ranges and related locations, imported-file refresh | `dmls::l1::named_type_references`, including editing an imported definition without reopening its consumer | 1 |
| Test-suite and shipped-schema audits | Recorded file dispositions in the implementation log; five `shipped_schema_corpus` tests; no remaining assertion requiring presence stripping found in affected source/test trees | 1 |
| Package tests and lint | Review runs below; implementation log additionally records complete Claudine and messenger runs | 1 |
| Serialization, descriptions, target-directed bounds, passive parsing, normal CLI invocation | Serializer/grammar round trips, source-projection tests, named-type equivalence tests, shipped corpus, and `md schema validate` fixtures | 1 |
| Existing target restrictions and unchanged lowering; grouped file matches, exclusions, empty intersections, and single-schema passivity | Resolver restriction tests, lowering equivalence tests, grouped-match validation tests, and Claudine completion tests | 1 |

The constraint review covered string length, numeric bounds, array item counts, object key counts in both mapping blocks and postfixes, patterns, schemes, defaults, suggestions, and examples. The file-match review followed the grouped representation through Darkmatter lowering, root-union validation, completion metadata, and Claudine's completion and supplied-file paths. No incomplete sibling implementation was identified.

The input robustness matrix applies to the changed `$constraints` mapping behavior, not to the type-expression text grammar. The YAML and JSON-form matrix tests cover the mapping itself plus `min-keys`, `max-keys`, `required`, and `generated`. They exercise absent, null, wrong types, invalid collection members, empty values, duplicate keys, and invalid/trailing content where applicable. Positive controls and public authored/effective validation results make these fields demonstrably load-bearing. Both inline and imported-object paths are checked. No silently accepted malformed cell was identified.

New test files are declared in their consolidated `l1/main.rs` targets and were selected by the review runs. The instrumentation-dependent corpus checks use the existing CI feature; the implementation log records a separate run with that feature enabled. Cross-platform evidence remains CI's responsibility and does not affect this verdict.

## Review execution

- Darkmatter `just test`: **9,299 passed, 11 skipped**, 39.7 seconds. The skipped tests belong to tiers outside this Level-1 invocation.
- Darkmatter focused schema/editor/CLI run: **137 passed**.
- Claudine `just test schema_completion composition::schema compose_schema_cli schema_interactive`: **256 passed**. The existing macOS linker warning about unwind-table size remained nonblocking.
- Darkmatter `just lint`: **passed**, including the Zed extension target check.
- `just check-tier-coverage darkmatter` and `just check-tier-coverage claudine`: **passed**, with no stranded tests.

No formatting command, commit, lifecycle-directory move, terminal window, or browser window was used.

## Observations

### Root-union namespace wording conflicts with retained behavior

The spec's local-name rules say inline root unions retain an empty local namespace. In fact, the Darkmatter [root-union resolver](../../../darkmatter/lib/src/markdown/schemas/resolve.rs) already resolved explicit local names against each arm's own entries on the base commit, and the feature retains that behavior for bare names. The current [local-reference documentation](../../../darkmatter/docs/topics/schemas/local-type-references.md) accurately describes the per-arm namespace; entries are never shared across arms. The implementation log records this historical mismatch. This is an observation about the spec's description of existing behavior, not a request to remove working per-arm resolution in this cycle.

### Conflict messages omit the conflicting values

Darkmatter's [schema error formatter](../../../darkmatter/lib/src/markdown/schemas/errors.rs) names conflicting constraint kinds and their source locations but omits values such as `min(10)` and `max(5)`. Editor diagnostics retain related locations showing the authored constraints, so the required provenance is available and this does not block readiness. Including the values in a future diagnostic improvement would make CLI errors easier to understand without opening both source files; no new protection layer or constraint solver is warranted.
