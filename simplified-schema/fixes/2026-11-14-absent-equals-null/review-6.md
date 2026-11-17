---
$schema: feature-review.yaml
ready: true
findings: []
human_review: false
human_in_the_loop: null
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: "2026-11-15T10:45:17-08:00"
spec: 2026-11-14-absent-equals-null/spec.md
implemented: false
description: "A **fix** review of `2026-11-14-absent-equals-null/spec.md`"
fix: 2026-11-14-absent-equals-null/review-6.md
previous: 2026-11-14-absent-equals-null/review-5.md
---

# Review 6: Absent equals null

The fix is **production ready within the spec's stated boundary**. R5-F1 is
verified fixed across all 27 required instances and controls. No new finding,
partial repair, regression, or newly found sibling remains. Earlier findings
remain fixed. No decision requires a human.

## Previous Finding Verification

Read review 5, the repair's `log.md` Finding Ledger, both original implementation
records, and the earlier reviews' findings and verification dispositions. No
earlier review contains a Human rulings section, and no earlier finding remains
blocked or open. The author's November 15 spec ruling bounds the used-default
check; its explicitly excluded conditional evaluation and parent satisfiability
checks are accepted limits, not unfinished findings.

The latest ledger groups some numbered children. The table below expands all
27 children so each original reproduction and clean control has a disposition.
Earlier closed findings are included to show their current regression evidence.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| R1-F1: Pattern-keyed objects skip default preparation | All 19 children | verified fixed | Both library builds pass selector, nested/defaulted-parent, recursive, conflict and positional controls; Darkmatter passes conditional-body default reporting | None |
| R1-F2: Lifecycle validation accepts inputs the parser rejects | All 23 children | verified fixed | Full host suites pass flag/placement agreement, bundles, nested bodies, overlays and completed-provenance regressions; shared passive checks still call the parser's placement rules | None |
| R2-F1: Dictionary selectors still skip or split default preparation | All 20 children | verified fixed | Both library builds pass `entry_defaults`, `combined_selectors` and raw selector tests, including required names and one-pass stability | None |
| R3-F1: Used defaults omit constraints from other applicable definitions | All 18 children | verified fixed | `complete_default_checks`, nullable-wrapper, union-policy and declaration-check equivalence tests pass in both builds; shipped-corpus tests remain below five seconds | None |
| R3-F2: Failure guidance omits runtime dictionary-default conflicts | All 11 children | verified fixed | Read validation, diagnostics, global constraints, both defaults skills and Rust API docs; they distinguish declaration-time conflicts from per-key runtime conflicts | None |
| R4-F1: Used defaults skip constraints in independent unions | All 22 children | verified fixed | Both builds pass `independent_union_checks`, including every original object-only route, direct preparation, `Prepared`, one-arm and authored-value controls; hosts delegate to the same engine | None |
| R4-F2: Claudine L1 tests exceed the timing budget | All 152 named cases and shared-work check | verified fixed | Full canonical run passed; all 152 original cases matched by package/test name, maximum 4.263 s; no test in the suite reached five seconds | None |
| R5-F1: Union alternatives that cannot contain a key bypass default checks | All 27 children below | verified fixed | Both builds' public-result regressions and 772 additional public-result checks per build; code, guidance and host consumers checked | None |
| R5-F1.1: SimplifiedSchema string shorthand | Named prefix/suffix fixture; both modes | verified fixed | `a_simplified_string_shorthand_refuses_the_default` returns `InvalidDefault` at `x-settings.options` | None |
| R5-F1.2: Raw overlapping patterns | Both union keywords and every excluding alternative | verified fixed | Every-route regression refuses before child completion | None |
| R5-F1.3: Literal plus matching pattern | Same alternatives and modes | verified fixed | Every-route regression refuses at `x-settings.options` | None |
| R5-F1.4: Literals across `allOf` | Same alternatives and modes | verified fixed | Every-route regression refuses at `x-settings.options` | None |
| R5-F1.5: Pattern plus another member's catch-all | Same alternatives and modes | verified fixed | Every-route regression refuses at `x-settings.options` | None |
| R5-F1.6: Separate catch-alls | Same alternatives and modes | verified fixed | Every-route regression refuses at `x-settings.options` | None |
| R5-F1.7: Local references | Joined parent and union references | verified fixed | Every-route regression refuses at `options` | None |
| R5-F1.8: Joined array items | Existing `[{}]` | verified fixed | Every-route regression refuses at `0.options` | None |
| R5-F1.9: Joined tuple positions | Existing first object; empty-tuple control | verified fixed | Every-route regression refuses; original positional controls insert no item | None |
| R5-F1.10: Common root default | Root property beside independent union | verified fixed | Every-route regression refuses at `options` | None |
| R5-F1.11: Two independent unions | Default on one union, required child on another | verified fixed | Every-route regression preserves grouping and refuses the parent default | None |
| R5-F1.12: Static alternative forms | Scalar/array/null types, type array excluding objects, scalar `const`/`enum`, false schema, references to string/null | verified fixed | Every-route regression checks all 13 excluding alternatives; added probes verify nested conjunctions/unions and references | None |
| R5-F1.13: Scalar default | Text `"3"` versus numeric property and string shorthand | verified fixed | `a_scalar_default_beside_a_string_member_is_refused_before_conversion`, both keywords and modes | None |
| R5-F1.14: Public default preparation | `DefaultPlan::compile` then `prepare_defaults` | verified fixed | Public preparation regression and extra absent/null probes refuse the uncompleted default at `options` | None |
| R5-F1.15: Public strict validation | Direct `Prepared` | verified fixed | Public verdict regression and extra probes return `Unprepared(InvalidDefault)` | None |
| R5-F1.16: Public soft validation | Direct `Prepared` | verified fixed | Same refusal before conversion or child completion | None |
| R5-F1.17: Property-local union control | Union inside defaulted property's own definition | verified fixed | Original public regression refuses, both keywords and modes | None |
| R5-F1.18: Direct conjunction controls | Original R3-F1 routes | verified fixed | `complete_default_checks` passes in both builds | None |
| R5-F1.19: Independent object-only union controls | Original R4-F1 routes | verified fixed | `independent_union_checks` passes in both builds | None |
| R5-F1.20: Default's own union controls | One-arm defaults and nested loose/strict alternatives | verified fixed | `required_defaults::unions` preserves the accepted one-arm policy | None |
| R5-F1.21: Direct null alternative | All ten routes, both keywords and modes | verified fixed | Every-route regression includes `type: null`; nullable-wrapper controls also pass | None |
| R5-F1.22: Property shapes and authored values | Absent/null, authored `{}`, false, number, empty text and array | verified fixed | Additional public probes refuse only used invalid defaults; authored objects gain the child default, false stays false, other types give data failures; second soft pass is stable | None |
| R5-F1.23: Object-admitting controls | Open object, type array with object, object reference and admitting pattern | verified fixed | Existing positive test plus extra probes on all ten routes, both keywords/modes; final `oneOf` exclusivity remains validation's decision | None |
| R5-F1.24: Comments and guidance | Module/helper Rust docs, validation/unions/diagnostics topics, defaults skill | verified fixed | Read against implementation: static object/key admission is described; validation topic states the bounded approximation; diagnostics retains correct conflict/error guidance | None |
| R5-F1.25: Downstream decisions | Facade, Darkmatter completion, Claudine composition | verified fixed | Facade probes pass; read host delegation and ran both full host suites | None |
| R5-F1.26: Boolean catch-all admission | Direct/referenced closed alternatives on all ten routes | verified fixed | Every-route regression includes both forms, both keywords and modes | None |
| R5-F1.27: Boolean property/pattern controls | Explicit false definitions on all ten routes | verified fixed | Existing controls plus extra route probes refuse at the expected path for both representations, keywords and modes | None |

Tests named in the R5-F1 rows live in
[pattern_defaults.rs](../../lib/tests/l1/pattern_defaults.rs), principally
`independent_union_checks::members_that_cannot_hold_the_key`. The every-route
test checks the public facade's strict and soft errors, including their paths,
for ten routes, two union keywords and 13 excluding alternatives. Its measured
times were 0.175 seconds with features and 0.157 seconds without default features.

## Unblocked Findings

None.

## Blocked Findings

None.

## Recurrence

No recurring finding is reported. R5-F1 repaired the sibling found in review 5;
the earlier selector and independent-union reproductions remain fixed. The
author's explicit boundary prevents this review from expanding the requirement
into general schema satisfiability analysis.

## Implementation assessment

In the simplified-schema package,
[`Fill::accepts`](../../lib/src/defaults.rs) now asks `admits_key` before counting
a union member with no property definition as accepting a default. That helper
reads Boolean schemas, object admission by `type`/`const`/`enum`, closed-object
key admission, local references, conjunctions and nested unions. The check is
static; it does not choose an arm by validating the unfinished parent.

The compiler continues recording union choices using its existing null-wrapper
rule. That private implementation choice preserves the one-arm default policy,
and the independent-union check rejects the reviewed non-object alternatives
where their empty selection would otherwise accept. The public route probes
and prior one-arm controls found no remaining wrong verdict. Changing this
internal representation is unnecessary for the bounded requirement.

Darkmatter's `EffectiveSchema::complete` still calls the library's
`prepare_defaults`, and Claudine uses that effective-schema path. There is no
second host default-validity walk to repair. Current guidance describes the
same boundary as the spec: conditional keywords and whether a parent's other
constraints can be satisfied are excluded from the preliminary default check;
validation still judges the completed value in full.

## Verification levels and input robustness

All requirements here concern data processing, passive diagnostics, or
hermetic command/execution wiring. L1 is appropriate: no acceptance criterion
requires terminal rendering, keyboard encoding, mouse input or a browser.

| Requirement | Verification level and evidence | Result |
| --- | --- | --- |
| Presence-only required keys, null admission, optional/required defaults, falsey values, borrowed/owned reports, unchanged input and stable second pass | L1 `required_presence`, `required_defaults`, public probes | Pass |
| Existing/missing/null parents, lists/tuples, recursive/imported types, pending values, default validity and union conflicts | L1 parent, nesting, pending, union and selector tests | Pass within the explicit default-check boundary |
| Raw schemas and exported JSON Schema; phase projections; trigger matching; prepared/uncached parity and shipped artifact identities | L1 raw/export, phase, matcher, envelope, identity and freshness tests | Pass |
| Envelope metadata/carriers, inheritance, keyed configuration payloads and default-sensitive key counts | L1 envelope/configuration matrices in library and host suites | Pass |
| Lifecycle flag/operand conversion, mandatory targets, placement, bundles, generated actions, `set` and authored provenance | L1 schema/parser agreement and fake-service tests in both hosts | Pass |
| `md schema validate`, composition, whole YAML and DMLS; declared-but-unwritten parent/child behavior and source preservation | L1 host, CLI, editor and absent-property regressions | Pass |
| `not-empty` by type, unsupported-type errors, list bound equivalence and empty YAML interpolation | L1 `not_empty_values` and host frontmatter tests | Pass |

The original parser/configuration changes retain their public-result robustness
matrices. Invalid field shapes and malformed documents produce located errors.
The explicit exceptions remain the spec's null-as-unset fields, optional-null
admission, allowed soft scalar conversions, and raw-schema defaults. A used
invalid default is a schema-preparation error; an unused default is not judged.

Read and executed the controls in `required_defaults::input_shapes`,
`envelope_null_carriers`, `input_robustness`, raw schema file/classification
tests, host lifecycle matrices, Claudine configuration matrices and CLI loader
wiring. These cover absent/null, wrong whole-field and element types, empty
values, duplicate keys, invalid/trailing text and format-specific syntax at
their shared validators/loaders. The latest repair adds no file loader.

Searched the relevant readers for `serde(default)`, `Option`, filtered elements,
`unwrap_or_default()` and `.ok()`. Claudine's typed null-as-unset adapters are
intentional inheritance rules; wrong types still fail deserialization. Schema
array filtering and pattern parsing are followed by schema-validator
compilation, which refuses malformed definitions. No additional silent
field-loss defect was established.

## Commands and results

| Command | Observed result |
| --- | --- |
| `simplified-schema/`: `just test` | 2,332 feature-enabled and 1,973 minimal tests passed; none at or above five seconds |
| `darkmatter/`: `just test` | 16,730 passed, 10 tier/recipe skips; none at or above five seconds |
| `claudine/`: `just test` | 12,170 passed, 15 tier/recipe skips; none at or above five seconds |
| `just lint` in each of the three areas | Exit 0 each; Darkmatter includes the Zed WASM check, Claudine includes transport and lifecycle-doc guards |
| `simplified-schema/`: `just doctest` | 40 passed, three existing ignored examples |
| Additional public probes, default and minimal builds | 772 checks passed in each build |
| Root `just check-tier-coverage`, separately for each area | Exit 0 for all three areas; zero stranded tests |
| Review metadata and whitespace checks | YAML frontmatter parses; readiness agrees with empty inventory; previous implementation claims preserved; `git diff --check` clean |

Additional temporary examples copied the existing required-child fixture and
route builders and called only public library APIs. In each library build,
252 checks covered facade/`Prepared`/direct preparation, absent/null values,
authored empty objects, false, wrong types, repeat stability and nested static
forms; another 520 checks covered Boolean property/pattern denials and positive
and negative static admission across every route. Both examples were removed.
No production source or permanent test was changed by this review.

The existing package tests compile through their declared consolidated L1
targets. The new module is declared by `tests/l1/main.rs`, uses no higher-tier
name prefix, and runs with both the CI-declared features and the minimal build.
Cross-OS proof remains with CI and is not a readiness finding.

## Implementation record correction

Corrected the current behavior summary in `implementation.md`, which still
said review 5's non-object and closed-object alternatives bypassed the default
check. The repair now refuses those defaults. Added a dated note naming the
two library runs and public probes that verify the correction. Historical
phase results remain historical evidence; no false current behavior statement
was established in `implementation-log.md`. This correction is not a finding
or a human-review item. Only the requested review metadata changes the spec.
