---
$schema: feature-review.yaml
ready: false
findings:
  - "R1-F1 (high): Passive schema consumers skip narrowing semantic checks"
  - "R1-F2 (high): Compound narrowing targets admit unresolved named types"
  - "R1-F3 (high): Invalid predicate calls still grant narrowing facts"
  - "R1-F4 (high): Narrowing failures lose required diagnostic provenance"
human_review: true
human_review_items:
  - "For later confirmation: accepting self-contained grouped-union array targets is reasonable, but the implementation log overstates support because registration currently rejects them. No ruling is needed for repair."
  - "For later confirmation: artifact comparison and fixture regeneration moved from phase 3 to phase 2 to keep removal of the old setter compilable; recommend retaining that sequencing."
  - "For later confirmation: two file_exists assertions changed from string to file, as the spec requires; recommend retaining them."
  - "For later confirmation: presence-only true-iff remains supported, as the spec explicitly requires; recommend retaining it."
  - "For later confirmation: alias coverage uses existing fileexists and isstring aliases rather than adding an is_file_type alias; recommend retaining that choice."
  - "For later confirmation: existing whole-artifact failure tests were reused alongside new per-function mismatch cases; the focused review run passed them."
  - "For later confirmation: the compose skill topic was left unchanged because it contained no obsolete string-narrowing claim; recommend retaining that choice."
  - "For later confirmation: dependent tests were not repeated after the final documentation-only source edit; earlier dependent checks were reported in the implementation log."
  - "For later confirmation: cross-OS execution was left to CI; it does not block this review's readiness decision."
  - "For awareness: the implementer left an existing CLI Clippy too_many_arguments failure unresolved; this review did not rerun lint or identify it as a runtime defect."
  - "Implementer decision (R1-F1.7): grouped-union arrays such as `(string | number)[]` are accepted as one-way narrowing targets by both the parser and registration; `true-iff` still rejects them. Alternative was rejecting them everywhere."
  - "Implementer decision (R1-F3): only a call the passive checker proves rejected withholds its fact; arguments typed Unknown/Uncertain/NullableMatch keep it. Alternative (require Compatible) would drop valid narrowing, since the subject itself is Unknown."
  - "Implementer decision (R1-F4): a missing-fact overload mismatch is located at the declaring clause, with related information spanning the overload that declares nothing."
human_in_the_loop: null
has_blocked_findings: false
blocked: false
recurrence: false
reviewed_by: codex/gpt-6.1-sol
created: "2026-10-27T21:52:13-07:00"
spec: 2026-10-27-schema-narrowing-metadata/spec.md
implemented: true
description: "A **feature** review of `2026-10-27-schema-narrowing-metadata/spec.md`"
feature: 2026-10-27-schema-narrowing-metadata/review-1.md
next: 2026-10-27-schema-narrowing-metadata/review-2.md
log: /Volumes/coding/wt/rusty-biscuit/feat-schema-narrowing-and-extraction/darkmatter/features/2026-10-27-schema-narrowing-metadata/log.md
implemented_by: claude/opus
---

# Review 1: Schema narrowing metadata

The feature is **not production ready**. The catalog migration, derived artifact comparison, built-in file narrowing, and normal composition example work in the selected tests. Four contract gaps remain. Every repair has a recommendation within the spec; none waits for a human decision.

## Findings

### R1-F1 (high): Passive schema consumers skip narrowing semantic checks

**Authority — spec:** Validation requires “one shared fact compiler used by catalogs, runtime registration, and DMLS,” with declaration checks in schema-owned code. The final acceptance criterion requires passive meta-type validation and DMLS authoring to use “the same declaration checks and located errors.”

**Class:** declaration validity depends on which public consumer reads it.

In Darkmatter, [compile_declared_fact](../../lib/src/markdown/compose/expression/call_binding/registration.rs:438) lives under runtime registration. The schema grammar checks clause shape, parameter existence, and boolean returns, but never calls the semantic checks for exact-family targets or agreement between overloads. Catalog loading adds those checks later. Consequently, an author can receive a successful schema validation and no editor diagnostic for a declaration the host cannot register.

Reproduce with the shipped `project.yaml` host fixture, changing only `narrows(value: file; true-implies)` to `narrows(value: file; true-iff)`. The host loader rejects it. Copy the same signature into a `kind: schema` type definition: standalone parsing accepts it, the `type-definition` meta-type validates it, and DMLS reports no definition diagnostic. The same discrepancy occurs for constrained exact-family targets and conflicting overloads.

The bounded sweep covered these sites and decisions:

| Instance | Site | Shape tested | Observed | Expected |
| --- | --- | --- | --- | --- |
| R1-F1.1 | Public type parser | `file`, `string(min(3))`, and `enum(a,b)` with `true-iff` | All accepted | Reject unsupported exact-family facts |
| R1-F1.2 | Standalone schema parser | The same three targets; overloads differing by absence, target, kind, or parameter position | Accepted | Reject every invalid declaration |
| R1-F1.3 | Runtime registration | Those targets and all four overload disagreement cases | Rejected; clean | Preserve rejection |
| R1-F1.4 | Host catalog loader | The three invalid exact-family targets | Rejected; clean | Preserve rejection |
| R1-F1.5 | Public meta-type validation | `string` control, then `file` and `string(min(3))` with `true-iff` | All report `valid: true` | Control valid; other two invalid |
| R1-F1.6 | DMLS standalone authoring | Valid file implication control, invalid file `true-iff`, mixed clause presence across overloads | No diagnostics for any | Diagnose the latter two |
| R1-F1.7 | Parser versus registration, array target | `(string | number)[]` with `true-implies` | Parser accepts; registration rejects as a union | One consistent supported-target decision |
| R1-F1.8 | Overload agreement control | Two distinct binding contracts with the same fact | Parser and registration accept; clean | Preserve acceptance |

Unknown kinds, unknown/rest parameters, non-boolean returns, top-level unions, and duplicate clauses already have rejection tests. The omission is semantic parity, not another grammar-shape matrix.

**Recommended repair:** move declaration validation into schema-owned code and have the parser, meta-types, catalog loading, and registration consume it. Keep runtime lowering separate from schema validity. Use the existing source mapping for the resulting errors, including the provenance repair in R1-F4. For grouped arrays, preserve the spec's supported self-contained targets and make parsing and registration agree; do not treat the implementation log's claimed support as proof that registration accepts them. Add public API and DMLS regressions for the missed semantic cases.

### R1-F2 (high): Compound narrowing targets admit unresolved named types

**Authority — spec:** Validation says to “reject unsupported named or constrained targets rather than accepting an annotation the analyzer cannot honor.” The selected scope explicitly rejects “named/imported references and targets that require a definition table.”

**Class:** target validation checks only the outer type and misses references nested inside a compound target.

Darkmatter's [narrows_target_problem](../../lib/src/markdown/schemas/simplified/types.rs:449) rejects only a top-level imported/name node. Its wildcard accepts an inline object or tuple containing the same node. The registration compiler then stores that target without translating or resolving it through the owning signature machinery.

Starting from the shipped host fixture, change the target `file` to `{a: Name}` or `tuple([Name])`. Both public schema parsing and host catalog loading succeed, and the resulting signature registers even with no definition of `Name`. A host author describing an object or tuple using a reusable helper type is a plausible input. Accepting it here bypasses the explicit scope boundary.

| Instance | Site | Shape tested | Observed | Expected |
| --- | --- | --- | --- | --- |
| R1-F2.1 | Shared outer-target validator via parser, registration, and host loader | `{a: Name}` | Accepted by all three | Located unsupported-target error |
| R1-F2.2 | Same consumers, tuple element | `tuple([Name])` | Accepted by all three | Located unsupported-target error |
| R1-F2.3 | Same consumers, direct reference control | `Name[]` | Rejected; clean | Preserve rejection |
| R1-F2.4 | Same consumers, grouped array containing a name | `(Name | string)[]` | Parser accepts; compiler rejects only because the outer node is a union | Reject the nested reference consistently |
| R1-F2.5 | Same consumers, self-contained controls | `{a: string}`, `tuple([string])`, `string(min(3))`, `enum(a,b)` with one-way implication | Accepted; clean | Preserve supported one-way targets |
| R1-F2.6 | Fact lowering and application | Accepted object/tuple target containing `Name` | Compiler stores the unresolved atom; `Shape::suitable_for` inserts that atom into an unknown subject's shape | Invalid target must fail before a usable fact exists |

The last row follows the value into its consumer: [fact compilation](../../lib/src/markdown/compose/expression/call_binding/registration.rs:483) clones the target, and [Shape::suitable_for](../../lib/src/markdown/compose/expression/scope/shape.rs:409) uses it with the document's definitions, not a retained host definition table. Neither step establishes that the name belongs to the catalog or can be honored. This is a code invariant, rather than a claim based on agreement between two readers.

**Recommended repair:** recursively reject definition-dependent nodes throughout targets, using schema-owned traversal across objects, tuples, union arms, and their children. Include source-free programmatic targets in the same check. Preserve self-contained supported targets; implementing named-target resolution is unnecessary and outside the selected scope.

### R1-F3 (high): Invalid predicate calls still grant narrowing facts

**Authority — spec:** “Facts apply only after a call is accepted by the normal call checker. A binding failure or a thrown error proves neither branch.” Acceptance also explicitly requires “invalid calls ... add no fact.”

**Class:** fact application checks arity but omits argument-binding validity.

Darkmatter's [predicate dispatch in TypeScope](../../lib/src/markdown/compose/expression/scope/facts.rs:50) asks registration for a fact based only on argument count. It never checks the call's argument types before refining the subject. This code predates the feature, but the feature expressly requires this behavior and its new optional-argument tests do not cover binding failures.

Register this description through the public registration API, without executing its handler:

```text
function(parameters(value: unknown, count: number);
         returns(boolean); narrows(value: file; true-implies))
```

With `foo: unknown`, `probe_pred(foo, {})` has an `Incompatible` second argument according to the normal call checker, yet assuming its true branch changes `foo` to `file`. A subsequent `frontmatter(foo, 'title')` is then classified compatible. Runtime rejection of the bad guard does not make this public analysis result satisfy the stated contract.

| Instance | Site | Shape tested | Observed | Expected |
| --- | --- | --- | --- | --- |
| R1-F3.1 | Public scope assumption | `probe_pred(foo, 1)` | True side is `file`; clean | Preserve valid narrowing |
| R1-F3.2 | Public scope assumption | `probe_pred(foo, {})` | Checker says incompatible; true side is still `file` | Retain `unknown` |
| R1-F3.3 | Negated guard | `!probe_pred(foo, {})` | False side becomes `file` | Retain `unknown` |
| R1-F3.4 | Short-circuit call checking | Bad guard followed by `frontmatter(foo, 'title')` | File argument classified compatible | No fact from the invalid guard |
| R1-F3.5 | Conditional block scope | The same bad guard in `::block when="..."` | Body's file argument classified compatible | No fact from the invalid guard |
| R1-F3.6 | Arity controls | Missing count; extra third argument | Neither branch refines `foo`; clean | Preserve no-fact behavior |
| R1-F3.7 | Omitted optional subject | Existing optional-fact test | No fact; clean | Preserve behavior |
| R1-F3.8 | DMLS branch type projection | Shared TypeScope result | DMLS `local_type` explicitly delegates to `branch_scope`/TypeScope, with no independent acceptance gate | Shared result must exclude invalid-call facts |

The projection row is bounded by DMLS's stated delegation invariant in [local_type](../../dmls/src/overlay/expressions.rs:1204); it is not a separate executed editor reproduction. All three implication kinds pass through the same unchecked dispatch before their polarity-specific application, so repair the shared acceptance point rather than one kind or consumer.

**Recommended repair:** gate fact application on the existing passive call check's rejection result in the current scope. Retain conservative handling of unknown arguments and avoid running handlers or duplicating binding rules. Add regressions for invalid supplied arguments, negation, and propagation into expression and block scopes.

### R1-F4 (high): Narrowing failures lose required diagnostic provenance

**Authority — spec:** “Duplicate clauses point to the second clause with related information for the first; mismatched overloads name both declarations.” It also requires located errors through imports, quoted strings, and block scalars, and the same typed errors for source-free declarations.

**Class:** declaration errors flatten source provenance into message text instead of carrying structured locations.

In Darkmatter's [duplicate-clause branch](../../lib/src/markdown/schemas/simplified/grammar.rs:1096), the first location becomes the text `(first at byte 51)`. The error contains only the second token's span. DMLS cannot turn that text into a related location through YAML decoding or imported files. Separately, [InvalidFact construction](../../lib/src/markdown/compose/expression/call_binding/registration.rs:442) discards the clause's stored spans, and catalog loading wraps it into a plain message. Fixing R1-F1's acceptance checks alone would still leave this independent provenance requirement unmet.

| Instance | Site | Shape tested | Observed | Expected |
| --- | --- | --- | --- | --- |
| R1-F4.1 | Grammar duplicate rejection | Two `narrows` clauses | Span at second keyword; first offset only in message | Structured primary and related locations |
| R1-F4.2 | DMLS quoted scalar | Same duplicate signature in a quoted YAML value | Correct second-token range, no `relatedInformation` | Related range at first declaration |
| R1-F4.3 | DMLS block scalar | Same duplicate signature in `|-` | Correct second-token range, no `relatedInformation` | Related range at first declaration |
| R1-F4.4 | Host catalog semantic rejection | `file; true-iff` | Message names file/property, with no structured target range | Located target failure |
| R1-F4.5 | Registration overload rejection | Missing/different facts across overloads | `InvalidFact` carries function and message only, including overload numbers | Typed failure retaining both declaration origins when available |
| R1-F4.6 | Existing single-token errors | Unknown implication and unknown parameter | DMLS block-scalar regression passes; clean | Preserve precise token ranges |
| R1-F4.7 | Source-free programmatic errors | Existing later-overload missing-parameter regression | Typed `InvalidFact`, no fabricated file range; clean for source-free behavior | Preserve this behavior while adding optional provenance |

A plain YAML control with spaces after parameter colons is invalid YAML before the grammar runs; it was excluded from the provenance conclusions. For imported declarations, the structural limitation is decisive: the first duplicate origin and semantic-error spans have already been discarded before any source projection can use them. No separate import lookup can recover them reliably.

**Recommended repair:** carry structured primary/related spans through schema-owned validation and the existing source-mapping path; retain source-free errors without invented ranges. Add quoted/block-scalar and imported-source assertions for both duplicate clauses and conflicting overloads. This needs no new diagnostic wire format or escaping layer.

## Verification and test rigor

I compared the implementation with the spec and inspected the changed grammar, representation, serializer, signature comparison, registration, catalog loading, artifact rebuilding, built-in declarations, documentation, and tests. Both decision-gathering agents independently read the spec and implementation log.

Executed through nextest-backed package recipes:

- `just test narrows`: 43 selected tests passed.
- `just test review_narrowing host_function flow_sensitive_narrowing compose_file_predicates context_function_contracts shipped_schema_corpus schema_yaml_spellings meta_schema_phase1 --success-output immediate`: 117 tests passed, comprising 113 existing tests and four temporary review probes.
- A final focused binding probe confirmed both expression and block propagation. All temporary probe edits were removed afterward; this review changes only this file and the spec's review count.

An attempted combined `-E` expression was rejected by the recipe's shell argument interpolation before tests launched. The successful command above used positional name filters instead. Lint was not rerun; the implementation log reports an existing CLI lint failure.

| Requirement group | Appropriate level | Evidence and limitation |
| --- | --- | --- |
| Grammar, serialization, overload identity, meta-types, passive errors | L1 | Existing tests pass; semantic parity and provenance gaps are R1-F1/R1-F4 |
| Built-in original-value implications, positive/negative branches, aliases | L1 | Predicate behavior and scope tests pass |
| Stable subjects, compound guards, lifecycle invalidation | L1 | Existing flow-sensitive suite passes; invalid-call acceptance gap is R1-F3 |
| Target support and passive rejection | L1 | Primitive/constrained controls pass; compound reference gap is R1-F2 |
| Artifact generation, drift, absence, per-function versus whole-file failure | L1 | Public reader matrix and DMLS host diagnostics pass |
| Normal guarded file call through `md compose` | L1 hermetic CLI | Existing shipped-binary composition test passes |
| Function types remain description-only | L1 | Existing JSON Schema boundary cases inspected; no runtime function-value support added |

No requirement here needs terminal-emulator rendering or OS keyboard injection. No terminal or browser focus was used. Cross-OS evidence is left to CI, as requested, and does not affect readiness.

## Input robustness matrix

The text grammar is excluded from the configuration matrix. The changed artifact comparison uses the existing strict JSON loader and field validators. Default invalid-shape behavior is rejection: whole-file errors make the catalog incomplete; entry errors make only that function incomplete; neither grants narrowing. Explicit `null` remains distinct from absence. `facts: []` is valid only when the catalog declares no fact.

The passing 75-cell public reader matrix covers absent, null, wrong whole-field types, wrong element types, empty values, unknown fields, duplicate keys, invalid/trailing JSON, stale hashes, and missing artifacts. New comparison cases cover missing declared facts, artifact-only facts, kind, parameter, position, type, and presence-marker disagreement. The `required`, `required_in`, typed array/string accessors, and strict loader do not silently filter malformed values. A code-smell inspection found no added permissive default or filtered parse on these fields. No independent configuration-shape defect was found.

## Implementation decisions (informational)

The following items need no human ruling before repair. The frontmatter lists them for later confirmation or reversal.

- **Grouped-union arrays:** accepting a self-contained array target is a reasonable scope interpretation. However, the log's statement that `(a | b)[]` is accepted describes parsing only: registration rejects it. R1-F1 records the inconsistency. Nested named references remain excluded under R1-F2.
- **Artifact work moved earlier:** comparison and regeneration moved into phase 2 because removing the old setter otherwise broke compilation. Recommend retaining the result; phase ordering has no remaining product effect.
- **File suitability expectations:** updating two assertions from `string` to `file` implements the spec directly. Recommend keeping them.
- **Presence-only equivalence:** retaining `narrows(x; true-iff)` implements the spec's explicit three-kind presence contract. Rejection is not a pending author choice.
- **Alias selection:** tests use the aliases the registry actually offers and do not add one for `is_file_type`. Recommend keeping that choice.
- **Artifact test reuse:** existing whole-file failures were reused while new mismatch cases exercised per-function failures. The focused review run passed both groups.
- **Unchanged skill topic:** leaving `compose.md` untouched after checking for drift is reasonable; other relevant skill topics were updated.
- **Dependent verification:** not repeating dependent tests after the final source change, a doc comment, is reasonable. Earlier dependent checks are reported in the log, not independently rerun here.
- **Cross-OS verification:** leaving it to CI is consistent with review readiness rules.
- **Existing lint failure:** the log leaves CLI `too_many_arguments` unresolved. This is a reported verification limitation, not an independently established wrong-result defect in this review.

No feature work is recorded as deferred after phase 5. Earlier deferrals of documentation and editor tests were completed. Richer named/recursive targets and collection callback/type propagation remain outside this feature by the spec's own scope. No human-only evidence, irreversible unresolved choice, or scope transfer prevents repair of any finding.
