---
$schema: feature-review.yaml
ready: false
findings:
  - 'R1-F1 (high): Tuple examples bypass read approval and lose their defining-file base'
  - 'R1-F2 (high): Import lookup requires grants for unused search candidates'
  - 'R1-F3 (medium): Nested grouped union arms are never substituted'
human_review: true
human_review_items:
  - 'After repairs, confirm the corrected terminology search and exclusion of unrelated uses and recorded transcripts; recommended: retain.'
  - 'Confirm using “option value” for CLI flags and “setter value” for name=value; recommended: retain.'
  - 'Confirm checking substituted tuple elements without changing static tuple acceptance; recommended: retain.'
  - 'Confirm refusing template types in function narrowing targets and nested function-signature positions; recommended: retain.'
  - 'Confirm preserving interpolated root file paths while refusing root type slots; recommended: retain.'
  - 'Confirm allowing grouped property unions and array item unions; recommended: retain and repair R1-F3.'
  - 'Confirm the explicit pending requiredness query alongside the compatibility boolean; recommended: retain.'
  - 'Confirm proof-based “quote the slot” hints, including envelope conversion failures; recommended: retain.'
  - 'Confirm recording returned import dependencies as imports and searched candidates; recommended: retain.'
  - 'Confirm separate expansion per slot origin, allowing equivalent schemas to differ in inline versus reference placement; recommended: retain.'
  - 'Confirm checking lowering during substitution to report invalid returned types and suffixes immediately; recommended: retain.'
  - 'Confirm gating returned examples and checking returned pattern keys; recommended: retain and complete the traversal in R1-F1.'
  - 'Confirm deduplicated admission and the public refuse-all helper; recommended: retain, but replace advance approval of unused candidates as R1-F2 describes.'
  - 'Confirm classifying wrong result shapes as invalid type definitions with their underlying errors; recommended: retain.'
  - 'Confirm the six distinct template-type diagnostic codes; recommended: retain.'
  - 'Confirm treating returned-import grant failures as deferrable and excluding them from failure caching; recommended: retain.'
  - 'Confirm the existing pre-read automatic-grant check without a second committed-byte check for returned imports; no change requested for the narrow concurrent-edit window.'
  - 'Confirm reporting the first using property for a shared named-type slot; recommended: retain.'
  - 'Confirm separate schema-file resolution, including unused slot-bearing named types and types-only files; recommended: retain.'
  - 'Confirm leaving template types inside load_data contract schemas unsupported and failing closed; no extension requested by this review.'
  - 'Confirm resolving recognized concrete schema files after their shape check; recommended: retain.'
  - 'Confirm testing prompted approval through the CLI handler seam in process; recommended: retain.'
  - 'Confirm reusing assignment conversion order and omitting grant-choice details from returned-import refusal records; recommended: retain the typed code and location.'
  - 'Confirm passive recovery of literal editor schema information after resolution failure; recommended: retain.'
  - 'Confirm showing every union arm while pending and preserving existing static hover output; recommended: retain.'
  - 'Confirm letting editor suggestions fall through to independent static union values; recommended: retain.'
  - 'Confirm ordering Claudine property names by the authored required suffix; recommended: retain.'
  - 'Confirm behavioral proof that name completion stays passive without enabling work counters; recommended: retain.'
  - 'Confirm sharing static property completion through the public root-property helper; recommended: retain.'
  - 'Confirm fresh volatile evaluation and silent URL fallback in completion; recommended: retain.'
  - 'Confirm keeping the existing lack of trigger-only completion when the document schema is pending; no additional trigger-discovery scope requested.'
  - 'Confirm preserving fixture names that describe template types rather than reference this feature snapshot; recommended: retain.'
  - 'Confirm splitting the simplified-schema skill into linked topic pages; recommended: retain.'
  - 'Confirm resolving the whole effective template for a templated completion request; recommended: retain, as the spec states.'
  - 'Other open editor buffers are not merged by the standalone schema-file helper; retain the current CLI-specific behavior, while editor jobs continue using their captured buffer snapshots.'
  - 'Cross-platform execution was deferred after a host connection failed; CI evidence remains outside this review’s readiness decision.'
  - 'R1-F2 repair (iteration 1): confirm the new public biscuit-file `FileReference::resolve_detailed_admitted`, which admits each candidate just before its containment check and probe (a recursive `%` reference admits every root before its walk); alternative was re-implementing biscuit-file’s candidate search inside simplified-schema.'
  - 'R1-F2 repair (iteration 1): confirm that a refused sibling-fallback check returns the refusal instead of reporting the name as missing.'
  - 'R1-F3 repair (iteration 1): confirm `requiredness()` is Pending (and `is_required()` false) for a slot inside an array item union, since a returned `(required)` item hoists onto the property; `has_pending_type()` keeps its top-level-only meaning for DMLS hover.'
  - 'R1-F1 repair (iteration 1): confirm no separate defensive gate in the final JSON example loader; every example-bearing position is gated before lowering instead.'
human_in_the_loop: null
has_blocked_findings: false
blocked: false
recurrence: false
reviewed_by: codex/gpt-6.1-sol
created: 2026-11-09T13:15:35-08:00
spec: 2026-11-08-template-types/spec.md
implemented: true
description: 'A **feature** review of `2026-11-08-template-types/spec.md`'
feature: 2026-11-08-template-types/review-1.md
log: /Volumes/coding/wt/rusty-biscuit/feat-better-research/darkmatter/features/2026-11-08-template-types/log.md
implemented_by: claude/opus
next: 2026-11-08-template-types/review-2.md
---

# Review 1: Template types

The feature is **not production ready**. Three defects need repair: tuple examples escape the slot read boundary, file lookup demands approval for paths it never needs, and nested grouped union arms remain unresolved. Each has a recommended repair within the existing spec. No human ruling is needed.

The review covered the spec and implementation log, the changed parser and schema consumers, returned-type expansion, import and example reads, schema validation, editor projection, and Claudine completion. Two parallel readers independently extracted implementation decisions before the code review. This is the first review; there are no previous findings to carry forward.

## Findings

### R1-F1 (high): Tuple examples bypass read approval and lose their defining-file base

**Authority — spec, “Imports, reads, and resolution”:** “Every read caused by the returned definition belongs to the originating slot’s approval boundary. This includes direct and transitive schema imports, referenced examples, and path probes.” The same section requires imports to retain their defining-file base. **Defect:** example references inside returned tuple definitions are read without approval, and examples inside imported tuple definitions resolve from the outer schema instead of the imported file.

**Class:** the recursive example-reference traversal skips tuple elements and spreads, leaving downstream loading to consume unresolved relative references outside the slot gate.

In simplified-schema, [rehome_atom_examples](../../../simplified-schema/lib/src/resolve.rs:3438) admits and relocates examples on atoms, union arms, object properties, and pattern-key properties, but never descends into tuples. The later [resolve_examples_in_json](../../../simplified-schema/lib/src/resolve.rs:4538) does descend into the lowered tuple and opens the example without that slot gate. Darkmatter consequently receives a successful schema without having approved or observed the example as a slot read.

Reproduction through the built `md schema validate case.yaml --output json`, outside a Git repository with an isolated home and no grants:

```yaml
kind: schema
$schema:
  status: '{{ "tuple([string(example(./ex.yaml))])" }}'
```

Use this valid `ex.yaml` beside it:

```yaml
kind: example
invocation: "x"
returns: "val"
description: d
```

The command exits 0 and reports `valid: true`. Changing only the returned definition to `string(example(./ex.yaml))` produces the expected exit 1 and `dm.schema.slot_approval_required`. Changing the tuple's example file to invalid content produces an example-envelope error, and removing it produces a missing-file error: these controls prove the ungranted file is actually consumed.

The sweep used that same example and no example grant at each site. Imported cases grant only `lib/types.yaml`, whose `Person` type holds the listed definition, and return `Person@./lib/types.yaml`.

| Child ID | Site and shape tested | Observed result | Expected result |
| --- | --- | --- | --- |
| R1-F1.1 | Returned scalar `string(example(...))` | Approval required; clean | Approval required |
| R1-F1.2 | Returned array atom `string[](example(...))` | Approval required; clean | Approval required |
| R1-F1.3 | Returned ordinary object property | Approval required; clean | Approval required |
| R1-F1.4 | Returned union arm | Approval required; clean | Approval required |
| R1-F1.5 | Returned pattern-key property `{ <pattern::a>: string(example(...)) }` | Approval required; clean | Approval required |
| R1-F1.6 | Returned fixed tuple element | Exit 0, valid; ungranted example read | Approval required |
| R1-F1.7 | Returned tuple spread `tuple([number, ...string(example(...))[]])` | Exit 0, valid; ungranted example read | Approval required |
| R1-F1.8 | Imported scalar definition | Refused at its example; clean | Approval required |
| R1-F1.9 | Imported array definition | Refused at its example; clean | Approval required |
| R1-F1.10 | Imported ordinary-object property | Refused at its example; clean | Approval required |
| R1-F1.11 | Imported union arm | Refused at its example; clean | Approval required |
| R1-F1.12 | Imported fixed tuple element | Exit 0, valid; ungranted example read | Approval required |
| R1-F1.13 | Imported tuple spread | Exit 0, valid; ungranted example read | Approval required |
| R1-F1.14 | Transitive imported pattern-key property | Refused at its example; clean | Approval required |
| R1-F1.15 | Transitive imported tuple element | Exit 0, valid; ungranted example read | Approval required |
| R1-F1.16 | Transitive imported tuple spread | Exit 0, valid; ungranted example read | Approval required |
| R1-F1.17 | Imported tuple element, granted folder, valid `lib/ex.yaml` and invalid outer `ex.yaml` | Rejects the outer file | Read `lib/ex.yaml` and succeed |
| R1-F1.18 | Imported tuple spread with that same base-directory control | Rejects the outer file | Read `lib/ex.yaml` and succeed |
| R1-F1.19 | Function parameter and return | Function value lowering refuses the definition; clean for this read class | No unguarded example consumption |
| R1-F1.20 | `property(...)` declaration | `type-definition` grammar refuses `example`; clean for this read class | No unguarded example consumption |
| R1-F1.21 | Final JSON example loader | Ungated tuple reference reaches file loading | Consume only examples already admitted at the originating slot |
| R1-F1.22 | Darkmatter result classification | Ungranted tuple example produces a valid result | Refuse the ungranted read before loading |

**Recommended repair:** extend the existing relocation/admission traversal to tuple elements and spreads. Keep the same defining-file base, gate, and observations; no new security layer or syntax is needed. Add public-resolution tests for both tuple positions, a denied example, a distinct imported-file base, and the resulting read observations. Cover a transitive imported tuple with the same traversal.

### R1-F2 (high): Import lookup requires grants for unused search candidates

**Authority — spec, “Verification and completion criteria”:** “Implementation is complete when a template type resolves to the same schema and validation behavior as the equivalent static definition,” with all introduced reads gated. **Defect:** a returned bare-name import fails even when its first search root contains a valid, granted match, because a later root that will never be searched is ungranted.

**Class:** eager admission of every possible lookup candidate changes ordered, first-match resolution into an all-roots permission requirement.

In simplified-schema, [select_schema_file_checked](../../../simplified-schema/lib/src/resolve/select.rs:144) calls `admit` for the entire planned candidate list before invoking selection. The ordinary selector stops at the first match. With project schemas followed by home schemas, this can request an unnecessary home-directory grant or make non-prompting validation/completion fail.

Reproduced in process through the public `resolve_yaml_schema_with_roots` API and the returned `PendingSchema::resolve` API, adapting the existing `template_type_imports` fixture. Supply two existing schema-root directories, `first` and `second`; put `types.yaml` exporting `Person: string` in `first`. The host returns `Person@types.yaml` and admits only paths under `first`. The all-static control resolves. The template refuses `second/types.yaml` even though `first/types.yaml` is the selected file. No concurrent edit or malformed input is needed.

The same public test used the following sweep. All successful controls used the same first-root grant and valid files; errors retained the test host's refusal code.

| Child ID | Site and shape tested | Observed result | Expected result |
| --- | --- | --- | --- |
| R1-F2.1 | Direct returned import `Person@types.yaml` | Refused at unused `second/types.yaml` | Resolve granted first-root type |
| R1-F2.2 | Explicit returned import `Person@./first/types.yaml` | Resolves; clean | Resolve |
| R1-F2.3 | Returned example `string(example(example.yaml))` with a valid first-root artifact | Refused at unused `second/example.yaml` | Resolve granted first-root artifact |
| R1-F2.4 | Explicit returned example `string(example(./first/example.yaml))` | Resolves; clean | Resolve |
| R1-F2.5 | Explicit returned import whose type transitively imports `Person@types.yaml` | Refused at unused `second/types.yaml` | Resolve granted first-root dependency |
| R1-F2.6 | Ordinary static bare-name selection with identical root ordering | Resolves first-root file; clean | Resolve first match |
| R1-F2.7 | Pending resolution's admission decision | Turns unused-candidate refusal into whole-resolution refusal | Ask only immediately before a candidate is actually needed |

**Recommended repair:** run the same ordered selector with admission immediately before each actual probe, stopping after the first match. Preserve denial before access; do not solve this by relaxing grants or probing first. Keep a missing-first-root case to prove a genuinely needed second probe still needs approval. Exercise the existing implicit-relative fallback branch as well as bare-name roots when moving the gate.

### R1-F3 (medium): Nested grouped union arms are never substituted

**Authority — spec, “Grammar and result contract”:** a slot may occupy a property's type or property-level union arm, with the suffix an imported named type accepts; “A returned union retains normal union semantics; an array suffix makes an array of that union.” **Defect:** a grouped array union containing a slot works as a property's whole type but fails when used as an arm of another union, including inside objects and tuples.

**Class:** substitution handles a union at the current definition but only traverses, rather than substitutes, grouped union atoms nested in its arms.

In simplified-schema, [TypeSubstitution::nested](../../../simplified-schema/lib/src/slots/template_type.rs:187) recurses into union arms without running their grouped-union substitution. [splice](../../../simplified-schema/lib/src/slots/template_type.rs:239) replaces only arms whose immediate type is a slot. Nested grouped unions survive both passes with their slot still present.

For example, `md schema validate case.yaml --output json` fails with `dm.schema.slot_failed` and “1 unresolved” for:

```yaml
kind: schema
$schema:
  status: "number | ({{ 'string' }} | boolean)[]"
```

The same command succeeds when the slot is replaced with `string`. The slot expression is a constant, so approval cannot explain the failure. These are less common nested type expressions, hence medium priority.

| Child ID | Site and shape tested | Observed result | Expected result |
| --- | --- | --- | --- |
| R1-F3.1 | Bare `{{ 'string' }}` | Resolves; clean | Static equivalent |
| R1-F3.2 | Bare slot with `[]` | Resolves; clean | Static equivalent |
| R1-F3.3 | Whole-property `({{ 'string' }} \| boolean)[]` | Resolves; clean | Static equivalent |
| R1-F3.4 | Direct union arm `number \| {{ 'string' }}` | Resolves; clean | Static equivalent |
| R1-F3.5 | Union arm `number \| ({{ 'string' }} \| boolean)[]` | Unresolved slot | Static equivalent |
| R1-F3.6 | Nested group `(number \| ({{ 'string' }} \| boolean)[])[]` | Unresolved slot | Static equivalent |
| R1-F3.7 | Ordinary object property `{ a: {{ 'string' }} }` | Resolves; clean | Static equivalent |
| R1-F3.8 | Object property `{ a: number \| ({{ 'string' }} \| boolean)[] }` | Unresolved slot | Static equivalent |
| R1-F3.9 | Direct tuple element `tuple([number, {{ 'string' }}])` | Resolves; clean | Static equivalent |
| R1-F3.10 | Tuple element `tuple([number \| ({{ 'string' }} \| boolean)[]])` | Unresolved slot | Static equivalent |
| R1-F3.11 | Named `State` body containing the nested union, used by two properties | Unresolved slot; static control succeeds | Static equivalent |
| R1-F3.12 | Final lowering for the failed shapes | Refuses unresolved type; clean failure containment | Receive fully substituted type |
| R1-F3.13 | CLI classification for the failed shapes | Exit 1 with unresolved-slot failure | Successful schema validation |

Every row also ran with the slot replaced by `string`; every static control succeeded.

**Recommended repair:** substitute grouped union atoms at every allowed nesting depth, retaining array constraints and hoisted property markers and checking invariants after replacement. Extend the existing public equivalence tests with the failing shapes; do not add a separate parser or textual rewrite.

## Verification and coverage

All required behavior here is library, command, or editor-protocol behavior. Level 1 is appropriate; the spec explicitly requires no focused terminal or browser windows. No keyboard encoding or terminal styling requirement was inferred.

| Requirement | Verification inspected or run | Level |
| --- | --- | --- |
| Grammar, allowed/refused positions, result shapes, unions, suffixes, nullable presence, passive queries/export, source positions | `template_types_passive`, `template_type_results`, `template_type_refusals`, `quote_the_slot` source review; template tests run | L1 public API and parser |
| Import bases, cycles, reachable-slot refusal, patterns, read admission, shared reads | `template_type_imports`, Darkmatter template-type tests; additional reproductions above | L1 public API |
| Trigger matching and replacement without evaluation | Darkmatter template-trigger tests run | L1 public API |
| Schema-file and document validation, approvals, data output, assignment conversion | Darkmatter template-type CLI tests run; scripted approval-handler tests inspected | L1 binary and handler seam |
| Dependency invalidation, warm read observations, editor buffers, stale results, cancellation | Darkmatter template-type tests and editor slot suite run | L1 public API and language-server protocol |
| Claudine passive names, static values, granted template values, silent refusal, composition modes | Committed-prompt completion suite and completion implementation inspected | L1 binary |
| Shipped artifacts and static behavior | Minimal-feature suite includes shipped corpus and static-schema tests | L1 |

Review runs:

- `simplified-schema: just test template_type`: 111 selected tests passed; the recipe also ran all 1,548 minimal-feature tests successfully.
- `darkmatter: just test dynamic_schema_slot_template`: 36 passed.
- `darkmatter: just test schema_slots_editor`: 52 passed.
- `claudine: just test-cli completion_committed_prompt_schema`: 13 passed.
- Additional CLI reproductions produced the failures and clean controls in the instance tables. A temporary public-API test reproduced the unnecessary-candidate refusals; its source was restored after collecting the evidence. No production implementation was changed.

The new integration test modules are included by their declared `l1/main.rs` targets and use ordinary L1 names. Existing tests passing does not cover the three missing cases above. The linker emitted its existing macOS unwind-table size warning during the command builds; it did not prevent execution. This review did not rerun full lint or cross-platform builds; the implementation log records the full package checks, and cross-platform evidence is expressly outside readiness here.

The input robustness matrix does not apply: the feature changes the template/type text grammar and YAML error hints, not a configuration loader or a configuration field contract. Returned-result shape coverage was nevertheless inspected: null, mapping, number, boolean, empty string/list, mixed invalid list members, and all-invalid list members are rejected, while supported string/list definitions resolve through the public API.

## Implementation decisions (informational)

The items below consolidate both independent decision-reader lists and all entries in `decisions_for_review`. They require no author action before repair. The corresponding frontmatter items let the author confirm or reverse them after this cycle. Existing spec rulings already authorize rejecting reachable slots, allowing trigger payloads, allowing tuple elements only, the terminology rename, resolving completion value requests, and resolving every slot during schema-file validation; those decisions are not reopened.

1. **Terminology search:** correct the plan's character-class typo; leave unrelated “value position” prose and historical agent transcripts intact. This respects the spec's semantic rename boundary.
2. **CLI wording:** distinguish option values from setter values. The distinction is accurate and cheap to change.
3. **Tuple checks:** check substituted elements without applying additional checks to static tuples. Preserve the explicit static-behavior contract.
4. **Function positions:** also refuse narrowing targets and nested slots inside refused function signatures. Such slots have no supported resolution path.
5. **Root refusal:** preserve interpolated file references such as `{{ dir }}/schema.yaml`; refuse complete root type slots. This avoids removing existing composition behavior.
6. **Grouped unions:** allow slots in grouped property unions and array items. Keep that interpretation and fix incomplete nesting under R1-F3.
7. **Requiredness query:** expose an explicit pending answer and return false from the compatibility boolean while pending. Pending-aware consumers use the explicit query.
8. **Quote hints:** require evidence that quoting improves the decoding error and also handle envelope conversion. This avoids blaming unrelated mapping errors.
9. **Dependency fields:** classify returned imports as import edges and searched candidates, with examples in their existing field. These are all cache inputs.
10. **Per-slot expansion:** preserve each slot's defining namespace independently, allowing different inline/`$ref` placement from an all-static export. Validation equivalence matters; byte-identical JSON layout is unnecessary.
11. **Early lowering:** reject unusable returned types and suffix conflicts during resolution. This prevents apparent success followed by export failure.
12. **Returned examples and patterns:** gate examples, resolve inline examples against the document, reject inline `example(this)`, and check pattern keys. Finish the missing tuple traversal under R1-F1.
13. **Read-hook design:** deduplicate admission per slot/path/kind, allow repeated set-like observations, fail unplannable targets closed, and expose a refuse-all helper. Keep these choices, but fix the advance admission of unused candidates under R1-F2.
14. **Shape errors:** report invalid type-definition results with the underlying grammar error instead of a separate wrong-shape variant. Typed causes remain available.
15. **Diagnostic names:** retain the six specific result codes, which distinguish invalid syntax, nested/reachable slots, local references, suffix conflicts, and invalid expansion.
16. **Refusal classification:** make returned-import grant failures deferrable during reference checks and avoid caching grant-dependent failures. This preserves grant behavior across sessions.
17. **Automatic-grant byte recheck deferred:** imports receive the pre-read clean-committed check without the data reader's second hash check. The log explicitly identifies the concurrent-edit window. No finding is raised for that narrow timing case; the confirmed unconditional tuple bypass is R1-F1.
18. **Pending property context:** report the first property using a shared named-type slot, and put that context on per-slot diagnostics. This is an adequate, deterministic location choice.
19. **Schema-file helper:** resolve exports plus unused slot-bearing named types and types-only files, using the provided text as an open buffer. This fulfills “every slot in the file.”
20. **Contract templates deferred:** template types inside `load_data` contract schemas remain unsupported and fail closed. The spec's enumerated document-selected schema surfaces do not clearly require expanding this separate contract path; no new scope is imposed.
21. **Concrete schema files:** resolve every recognized schema file after shape validation, including concrete files. This also detects ordinary resolution failures.
22. **Prompt verification:** use the CLI's approving/denying handler seam in process. It directly proves the grant decision and batching; no terminal-input behavior is part of this feature.
23. **Assignments and refusal detail:** reuse resolution-before-assignment conversion; returned-import refusal records retain their typed code and location without `grants_needed` choices. The spec does not require that additional payload.
24. **Failed editor resolution:** recover literal authored schema information by passive assembly while preserving failed status. This retains useful independent information without validating stale types.
25. **Pending hover:** show all arms when pending while retaining existing first-arm hover for concrete unions. The change preserves static sibling information.
26. **Suggestion fallback:** let an empty pending-only suggestion result fall through to static sibling values. This is consistent with passive sibling information.
27. **Name ordering:** use the authored required suffix for Claudine name ordering, as expressly required.
28. **Passivity evidence:** prove name completion behavior without work-counter feature changes. An unresolved, ungranted fixture is an adequate negative control.
29. **Shared completion helper:** use a public root-property helper for static values in pending templates. This avoids evaluating unrelated slots.
30. **Volatile/remote completion:** evaluate volatile expressions freshly; offer no values for URL reads without a remote runtime. No new cache or network policy is needed.
31. **Trigger-only completion deferred:** do not add value candidates for matched-trigger-only properties while the document schema is pending. The current completion loader does not enable trigger discovery even for concrete schemas; this review does not turn that existing scope into a new requirement.
32. **Fixture names:** preserve directories named for the template-type concept. They are not links to a movable feature snapshot.
33. **Skill organization:** split the simplified-schema skill into linked topics to keep its entry file short. This is reversible documentation structure.
34. **Whole-template completion:** resolve the effective template, rather than only the requested property's slots. This matches the spec; an unrelated ungranted slot can therefore suppress values for a templated property.
35. **Other editor buffers deferred in the schema-file helper:** supplied schema-file text replaces the active buffer snapshot when necessary. Normal editor jobs use their separate captured-buffer path; no editor regression was established here.
36. **Cross-platform evidence deferred:** the implementation's host connection failed. This is recorded, but cross-platform evidence does not block this review's readiness decision.

No choice meets the stopping test without a recommendation that stays inside the spec and existing rulings. All three findings can be repaired directly.
