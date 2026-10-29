---
$schema: feature-review.yaml
ready: true
findings: []
human_review: false
human_in_the_loop: null
has_blocked_findings: false
blocked: false
recurrence: false
reviewed_by: codex/gpt-6.1-sol
created: "2026-10-27T23:35:16-07:00"
spec: 2026-10-27-schema-narrowing-metadata/spec.md
implemented: false
description: "A **feature** review of `2026-10-27-schema-narrowing-metadata/spec.md`"
feature: 2026-10-27-schema-narrowing-metadata/review-2.md
previous: 2026-10-27-schema-narrowing-metadata/review-1.md
---

# Review 2: Schema narrowing metadata

The feature is **production ready** within the specification's selected scope. All four previous findings are verified fixed, including every required instance in the repair ledger. No new findings or decisions requiring a human were identified.

## Previous Finding Verification

I read `review-1.md` and the repair's `### Finding Ledger` in `log.md` before reviewing the repairs. The previous review has informational human-review items but no author rulings. These outcomes come from the implementation and independently executed tests, rather than the ledger's completion flags.

Test names below identify assertions in the Darkmatter library's `meta_schema_phase1`, `flow_sensitive_narrowing`, and `host_functions` test modules, or DMLS's `schema_yaml_spellings`, `named_type_references`, and `expression_scopes` modules. All these modules were selected in the passing review run.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| R1-F1: Passive schema consumers skip narrowing semantic checks | R1-F1.1–R1-F1.8, checked individually below | verified fixed | Schema-owned declaration validation is shared by parsing and registration; catalog and editor paths consume those checks. | Nothing |
| R1-F1.1: Public type parser | Reject `file`, `string(min(3))`, and `enum(a,b)` with `true-iff` | verified fixed | `public_type_parser_rejects_true_iff_narrows_on_a_refinement` checks rejection at the target and accepts one-way controls. | Nothing |
| R1-F1.2: Standalone parser | Reject those targets and overload disagreement by absence, target, kind, and position | verified fixed | `standalone_schema_parser_rejects_invalid_narrows_declarations` exercises scalar and YAML-list overload spellings. | Nothing |
| R1-F1.3: Registration | Preserve invalid-target and overload-disagreement rejection | verified fixed | `registration_rejects_invalid_facts` and `registration_rejects_overloads_that_disagree_on_the_fact` test text and programmatic declarations. | Nothing |
| R1-F1.4: Host catalog | Preserve invalid exact-target rejection | verified fixed | `registration_rejects_collisions_parity_gaps_and_invalid_facts_before_evaluation` mutates the shipped host fixture for all three targets. | Nothing |
| R1-F1.5: Meta-types | Accept exact-family control; reject invalid refinements | verified fixed | `type_definition_rejects_narrows_facts_registration_rejects` checks controls and failures; `schema_consumers_reject_named_types_nested_in_narrows_targets` also checks the `schema` meta-type with a file refinement. | Nothing |
| R1-F1.6: DMLS authoring | Diagnose invalid `true-iff` and missing overload facts; keep valid control clean | verified fixed | `a_narrows_fact_registration_rejects_is_diagnosed_while_authoring` exercises the language-server protocol path. | Nothing |
| R1-F1.7: Grouped array parity | Make parser and registration agree on `(string \| number)[]` | verified fixed | Parser and host controls accept it; `a_grouped_union_array_is_a_one_way_target` verifies registration and scope application. Shared validation still rejects it for `true-iff`. | Nothing |
| R1-F1.8: Agreeing overload control | Preserve distinct binding contracts with the same fact | verified fixed | Registration, standalone scalar/list parsing, and meta-type controls accept matching facts. | Nothing |
| R1-F2: Compound narrowing targets admit unresolved named types | R1-F2.1–R1-F2.6, checked individually below | verified fixed | Target validation exhaustively visits every type-expression variant before a fact can be compiled. | Nothing |
| R1-F2.1: Object member | Reject `{a: Name}` in parser, registration, and host loading | verified fixed | `public_type_parser_rejects_named_types_nested_in_narrows_targets`, `registration_rejects_named_types_nested_in_a_target`, and `a_host_catalog_rejects_named_types_nested_in_a_fact_target` check the same shape. | Nothing |
| R1-F2.2: Tuple element | Reject `tuple([Name])` in the same consumers | verified fixed | The same three tests include this target; standalone and meta-type tests also reject it. | Nothing |
| R1-F2.3: Direct reference control | Keep rejecting `Name[]` | verified fixed | Parser, registration, and host target matrices retain this rejection. | Nothing |
| R1-F2.4: Grouped array reference | Reject `(Name \| string)[]` because of its nested name | verified fixed | Public parser, registration, host loader, standalone parser, and meta-type tests reject it with a named-type error. | Nothing |
| R1-F2.5: Supported controls | Preserve self-contained objects, tuples, constrained strings, and enums | verified fixed | Parser, registration, and host matrices accept `{a: string}`, `tuple([string])`, `string(min(3))`, and `enum(a,b)` as one-way targets. | Nothing |
| R1-F2.6: Fact lowering/application | Prevent unresolved targets from becoming usable facts, including programmatic declarations | verified fixed | Programmatic registration rejects nested names. The exhaustive traversal also covers dictionary values, tuple spread, union arms, imported references, and nested function types. Registration validates before constructing a fact, preventing invalid targets from reaching scope application. | Nothing |
| R1-F3: Invalid predicate calls still grant narrowing facts | R1-F3.1–R1-F3.8, checked individually below | verified fixed | The single predicate-fact dispatch point checks the passive call verdict before applying any implication kind. | Nothing |
| R1-F3.1: Valid call control | Preserve `probe_pred(foo, 1)` narrowing to `file` | verified fixed | `a_rejected_call_grants_no_fact_on_either_side` checks the valid call and its negation. | Nothing |
| R1-F3.2: Rejected supplied argument | `probe_pred(foo, {})` must retain `unknown` | verified fixed | The same test asserts both the incompatible call verdict and unchanged subject type on both branches. | Nothing |
| R1-F3.3: Negation | Rejected calls must grant no fact through `!` | verified fixed | The same test checks negated and parenthesized forms; `every_implication_kind_withholds_a_rejected_calls_fact` adds the other implication kinds. | Nothing |
| R1-F3.4: Short-circuit call checking | Rejected guards must not make later file arguments compatible | verified fixed | `a_rejected_guard_does_not_narrow_later_operands` checks `&&`, `and(...)`, and both ternary arrangements. | Nothing |
| R1-F3.5: Conditional blocks | Rejected block guards must not narrow their bodies | verified fixed | `a_rejected_block_guard_does_not_narrow_its_body` checks valid and invalid guards, including a compound guard. | Nothing |
| R1-F3.6: Arity controls | Missing and excess arguments must grant no fact | verified fixed | `a_wrong_argument_count_grants_no_fact` asserts rejection and unchanged types for both polarities. | Nothing |
| R1-F3.7: Optional subject omitted | An omitted optional subject must grant no fact | verified fixed | `an_omitted_optional_fact_argument_narrows_nothing` checks omission and a supplied-subject control. | Nothing |
| R1-F3.8: DMLS projection | Editor branch types must share the corrected scope result | verified fixed | `local_type_takes_no_fact_from_a_rejected_guard_call` exercises DMLS's type projection with instrumentation enabled by the area recipe. | Nothing |
| R1-F4: Narrowing failures lose required diagnostic provenance | R1-F4.1–R1-F4.7 and the repair's additional sibling R1-F4.8, checked below | verified fixed | Structured primary and related spans survive grammar errors, YAML projection, registration, host loading, and editor diagnostics. | Nothing |
| R1-F4.1: Duplicate clause | Locate the second keyword and relate the first clause | verified fixed | `public_parser_relates_a_duplicate_narrows_clause_to_the_first` asserts exact structured spans and the absence of a byte-offset message substitute. | Nothing |
| R1-F4.2: Quoted YAML | Preserve both duplicate-clause locations through quoted scalars | verified fixed | `a_duplicate_narrows_clause_relates_the_first_while_authoring` asserts protocol ranges. | Nothing |
| R1-F4.3: Block YAML and imports | Preserve both locations through block scalars and referenced files | verified fixed | The same DMLS test covers block scalars. `a_rejected_narrows_declaration_in_an_imported_file_is_related_into_that_file` checks duplicate and conflicting-overload locations in referenced/imported files. | Nothing |
| R1-F4.4: Host semantic rejection | Return a structured target location | verified fixed | `a_host_catalog_locates_a_rejected_fact_in_its_source` checks the target, duplicate clause, and overload disagreement against source bytes. | Nothing |
| R1-F4.5: Overload disagreement | Retain both declaration origins where source exists | verified fixed | `public_parsers_relate_disagreeing_narrows_overloads_to_both_declarations`, registration tests, and `disagreeing_narrows_overloads_relate_both_declarations_while_authoring` check scalar/list forms and missing/different facts. | Nothing |
| R1-F4.6: Single-token controls | Preserve unknown-kind and unknown-parameter ranges | verified fixed | `an_invalid_narrows_clause_in_a_block_scalar_is_ranged_at_its_token` still passes. | Nothing |
| R1-F4.7: Source-free errors | Keep typed failures without fabricated file ranges | verified fixed | The later-overload missing-parameter case returns `InvalidFact` naming overload index 1; disagreement names both indexes. This error variant carries no file range. | Nothing |
| R1-F4.8: Additional duplicate-constraint sibling | Preserve related locations for repeated function constraints and suggestions | verified fixed | The generic duplicate-keyword branch retains the first keyword for `parameters`, `returns`, `fallible`, and `category`; the duplicate-return regression executes it. Scalar/list suggestion paths retain the first candidate's structured span. | Nothing |

The repair ledger also records R1-X1, the previous review's informational CLI lint failure. `just lint` now passes, independently confirming that limitation is cleared. It was not an additional finding in the previous review.

## Unblocked Findings

None.

## Blocked Findings

None. No unresolved prerequisite or human decision prevents completion.

## Spec coverage and test rigor

I inspected the declaration representation, parsing and serialization, overload identity, schema-owned validity checks, runtime lowering, built-in declarations, fact application, host artifact comparison, diagnostic source projection, documentation, and changed tests. The review preserves the selected scope: self-contained targets, existing scope operations, and description-only function types. Named/recursive target support and collection callback propagation remain outside this feature.

The shared validity rules live in the Darkmatter library's [narrowing module](../../lib/src/markdown/schemas/simplified/narrowing.rs). [Registration](../../lib/src/markdown/compose/expression/call_binding/registration.rs) rechecks programmatic declarations through those rules and lowers valid declarations. [Scope fact application](../../lib/src/markdown/compose/expression/scope/facts.rs) uses the existing passive call checker to withhold facts for rejected calls. No predicate handler or second narrowing engine is used to validate declarations.

| User-facing requirement | Appropriate verification level | Evidence |
| --- | --- | --- |
| All implication kinds and presence-only forms survive repeated serialization | L1 | Grammar and serializer `narrows` tests check canonical placement, absence, clause ordering, and repeated round trips. |
| Invalid clauses and disagreeing overloads fail passively and at the correct location | L1 | Public parser, registration, meta-type, host catalog, and DMLS tests listed above; handler counters assert passive paths execute no handler. |
| Both file predicates prove file suitability on true without false-side exclusion of missing paths | L1 | `declared_builtin_facts` branch assertions and `context_function_contracts` runtime input cases. |
| Primitive and presence predicates retain branch behavior and original-value meaning | L1 | `the_migrated_primitives_keep_their_branch_shapes` and `migrated_predicates_answer_about_the_original_value`, including convertible values. |
| Stable binding identity, explicit local aliases, separate evaluations, shadowing, and lifecycle invalidation | L1 | `flow_sensitive_narrowing` tests exercise subject identity, repeated calls, guarded scopes, writes, and event boundaries. |
| Invalid calls and omitted subjects add no facts; negation and compound guards remain correct | L1 | `rejected_calls`, optional-subject, alias, short-circuit, and conditional-block tests. Arguments that may bind conservatively retain their facts; only a proven rejection withholds them. |
| Host artifact data agrees with catalog-derived facts without running a host | L1 | `host_functions` checks deterministic generation, missing/extra/mismatched fact data, malformed/stale/missing artifacts, and per-function versus whole-catalog failure. DMLS host-function tests verify incomplete-analysis reporting. |
| Unknown values reach file-typed calls after a guard through the normal CLI | L1, hermetic subprocess | `compose_file_predicates` invokes the shipped `md compose` path using the existing CLI fixture. |
| Shipped declarations remain valid; duplicate binding contracts remain errors | L1 | `shipped_schema_corpus` and grammar/signature tests, including parameter-name/annotation insensitivity and equivalent target spelling controls. |
| Function types remain description-only | L1 | Existing function-conversion rejection tests and inspected JSON Schema conversion boundary; no runtime function-value representation was added. |
| Imported, quoted, and block-scalar diagnostics retain primary/related ranges | L1, language-server protocol | DMLS duplicate-clause, overload-disagreement, and imported-definition tests assert protocol locations. These are source-analysis requirements, not terminal rendering requirements. |

No requirement needs real-terminal capture or OS keyboard injection. No terminal or browser window was opened or focused. Cross-OS evidence remains a CI responsibility and does not affect this review's readiness decision.

## Input robustness matrix

The SimplifiedSchema text grammar is excluded from the configuration matrix. For the host facts artifact, the default invalid-shape outcome remains rejection without granting narrowing: a whole-file failure makes the catalog incomplete; an entry failure makes only that function incomplete. Absence and explicit null remain distinct. The deliberate exceptions are nullable operational fields and `facts: []` when the catalog declares no fact.

The passing public host-reader matrix starts from the committed artifact and a positive narrowing control. It exercises absent fields, nulls, wrong field and element types, empty collections, unknown fields, duplicate keys, invalid/trailing JSON, hash drift, and missing artifacts. Fact comparison checks absence in both directions, parameter, position, kind, target, and marker disagreement. Runtime and passive reader controls agree.

I checked the strict JSON loader and shared field accessors for permissive defaults, silently filtered elements, and swallowed parse failures. The integer conversion's `.ok()` is followed by a returned error when conversion fails; it does not default the position. No additional configuration reader or format was introduced by the repair.

## Independent verification

Executed from this worktree:

- `just test narrows host_function flow_sensitive_narrowing meta_schema_phase1 schema_yaml_spellings named_type_references expression_scopes shipped_schema_corpus schema_narrowing context_function_contracts compose_file_predicates` in `darkmatter/`: **202 passed**, 12 binaries, 4.978 seconds. No selected test exceeded five seconds.
- `just lint` in `darkmatter/`: passed for the library, CLI, DMLS, Zed CLI, and the Zed WASM extension check.
- `just check-tier-coverage darkmatter`: passed; no stranded tests. Changed test modules are declared by their consolidated L1 targets, and the recipe enables the instrumentation feature used by the DMLS regression.
- `git diff --check`: passed before writing this review.

The test build emitted a macOS linker warning about compact unwind-table size; it did not prevent compilation or execution. I did not repeat the implementation log's full-area or dependent-package test runs and do not treat those claims as independently reproduced evidence.

This review changes only this review file and the requested review/spec frontmatter. It does not modify the implementation, relocate the feature, or create a commit.

## Recurrence

No open or new finding repeats an earlier defect class. All previous findings are closed by the verification above; no partial repair or newly found sibling remains.
