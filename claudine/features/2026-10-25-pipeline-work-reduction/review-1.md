---
$schema: feature-review.yaml
ready: false
findings:
    - "R1-F1 (high): Lifecycle and body loop bindings still copy the whole previous result"
    - "R1-F2 (medium): Property-union rewriting loses the caller's file-match origin"
human_review: true
human_review_items:
    - "Informational: timing used a moderately loaded host; retain the recorded uncertainty and treat small timing changes as inconclusive. No response is required."
    - "Informational: count all five shell-site placeholder analysis passes when applying the follow-up threshold; the estimated time share is 6.4%."
    - "Informational: retain the mechanical fixes to pre-existing lint and portable test-path failures."
    - "Informational: enable work counters in normal Darkmatter test recipes and CI test features, while keeping default builds uninstrumented."
    - "Informational: the implementer limited loop-read optimization to conditions; R1-F1 recommends completing the lifecycle and body paths within this spec."
    - "Informational: preserve the existing condition lookup behavior for nested loop.last_output paths rather than introduce new expression semantics."
    - "Informational: skip sequence-value construction for tasks without a condition."
    - "Informational: compare output-copy counts across history sizes instead of fixing their exact numeric values."
    - "Informational: remove a named-type memo entry after a failed read or parse; restoring the old content then requires one parse."
    - "Informational: leave the pre-existing second parse of failing imports unchanged; it occurs only while reporting an error."
    - "Informational: the completion design uses per-alternative traversal masks and exact root spellings to preserve existing results."
    - "Informational: completion code and any biscuit-file API change for per-file canonicalization remain future work; this spec requires a candidate design only."
    - "Informational: use a test-build file report to verify CLI branch lifetimes instead of enabling CLI work counters."
    - "Informational: branch counts exclude the enclosing container; each admitted branch still copies its immutable group-start seed."
    - "Informational: count provider-expression state construction per task, sharing state between its agent and model hints."
    - "Informational: preserve task-wide provider deferral when either hint depends on runtime writes; splitting dependencies per hint would change behavior."
    - "Informational: property-union caller origins were deliberately left unchanged; R1-F2 recommends correcting the demonstrated normalization defect."
    - "Informational: retain the root-union origin correction so its rewrite verdict agrees with validation."
    - "Informational: use the existing process-wide validator cache in the public eager-rewrite helper."
    - "Informational: include all root shared definitions in each arm; pruning unrelated definitions is unnecessary for this change."
    - "Informational: return report and schema presence as a tuple; the existing timing recorder supplies preparation metadata without recording it for every caller."
    - "Informational: derive diagnostic token locations from completed problem codes and properties instead of changing public problem records."
    - "Informational: retain positions for pre-shell problems that composition later filters; that earlier report actually contains problems."
    - "Informational: implement alphanumeric caching in its actual conversion module rather than the file named in the plan."
    - "Informational: the alphanumeric compile counter can overcount simultaneous cache misses; this affects instrumentation, not validation results."
    - "Informational: leave the repeated scope-join timing slowdown unchanged because its executed source and direct work were unchanged."
    - "Informational: retain the small documentation corrections discovered during reconciliation."
    - "Informational: keep before/after anchor results in the implementation log, as the acceptance criterion requests."
    - "Informational: retain full snapshots where whole state is required; move enclosing sequence outputs during lexical scope changes."
    - "Informational: bracketed loop access continues through whole-root expression evaluation; no new bracket-access semantics are requested."
    - "Informational: prove preparation counts at the library boundary and CLI wiring with subprocess behavior tests."
    - "Informational: formatting was limited to newly created tests; existing files were not reformatted."
    - "Informational: close pre-warm reuse and the other excluded optimizations as measured, no change; create the two qualifying follow-up specs automatically."
    - "Informational: use macOS for local measurement and existing CI for other operating systems; cross-OS evidence does not block this review."
    - "Implementer decision (R1-F1): added Darkmatter `RuntimeBinding::Members { member, whole }` so dotted `loop.<member>` reads skip the whole object; a host-globals set holding one fingerprints by cache pointer like `Lazy`, so equal values built separately can miss the per-run compose cache (never a wrong hit). `loop.last_output.k` still copies `last_output`, which the contract allows."
    - "Implementer decision (R1-F2): `CallerOrigins` gained a whole-instance form; a property-level union arm compiles with its owning top-level property's origin and is retained per (arm, origin), rather than receiving the full property-keyed origin map."
human_in_the_loop: null
has_blocked_findings: false
blocked: false
recurrence: false
reviewed_by: codex/gpt-6.1-sol
created: 2026-10-27T15:13:49-07:00
spec: 2026-10-25-pipeline-work-reduction/spec.md
implemented: true
log: /Volumes/coding/wt/rusty-biscuit/feat-work-reduction/claudine/features/2026-10-25-pipeline-work-reduction/log.md
implemented_by: claude/opus
description: "A **feature** review of `2026-10-25-pipeline-work-reduction/spec.md`"
feature: 2026-10-25-pipeline-work-reduction/review-1.md
next: 2026-10-25-pipeline-work-reduction/review-2.md
---

# Review 1: pipeline work reduction

The feature is **not production ready**. One approved optimization remains incomplete, and a file-origin defect affects eager normalization through recursive unions. Both have a recommended repair within the existing scope; neither waits for an author decision.

Reviewed the implementation against `spec.md`, the implementation log, the completion candidate design, the refreshed spike report, and the production/test diff from `d7ea96a1f` through `076a54516`. Two parallel readers independently inventoried the implementer's choices before the implementation comparison. There is no earlier implementation review to carry forward.

## Findings

### R1-F1 (high): Lifecycle and body loop bindings still copy the whole previous result

**Authority: spec**, “Hook and loop leaf reads (C2, C4)” requires: “Resolve the requested member before constructing a whole root object. Scalars must not clone a large sibling”; design rule 3 likewise says to materialize the whole object only when the whole root is requested. Acceptance item 5 requires a deterministic regression for the removed work. The implementation optimizes the condition lookup, but leaves ordinary lifecycle and document-body bindings eager.

**Defect class:** eager construction of a namespace root copies unrelated large values before a scalar consumer can select its member.

In the `claudine` package, [runtime_bindings_enclosed](../../lib/src/composition/lifecycle/bindings.rs:311) constructs `loop` through `EnclosingLoop::value`, and [composition_host_globals](../../lib/src/composition/lifecycle/bindings.rs:358) does the same for composition. Both call `LoopAmbient::to_value`, which copies `last_output`. A loop whose previous pass produced a large report therefore still copies that report when a lifecycle message or the next prompt uses only `{{ loop.count }}`. Binding construction copies it even before an expression reads anything.

Reproduction: reuse the existing `loop_leaf_reads.rs` one-megabyte ambient fixture. Snapshot `Work::LoopAmbientMaterialize` around `runtime_bindings_in_loop(LifecycleScope::Event(LifecycleSignal::Success), LifecycleValues::default(), Some(&ambient))`, then around `composition_host_globals(EnclosingLoop::Iteration(&ambient), json!({}), None)`. An isolated nextest probe observed `(1, 1)` instead of `(0, 0)`, before any whole-root request. The existing condition and hook tests provide the corresponding clean controls.

| Child ID | Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- | --- |
| R1-F1.1 | Lifecycle bindings, including task/container events through `StackExecutionContext::bindings` | One-megabyte previous result; scalar-only consumer | One whole-loop materialization during binding construction | No whole-loop materialization for a scalar read |
| R1-F1.2 | Composition host globals, used by body/frontmatter composition | Same ambient; scalar-only consumer | One whole-loop materialization during host-global construction | No whole-loop materialization for a scalar read |
| R1-F1.3 | `LoopExpressionLookup::get` condition path | Same large sibling; dotted scalar members | Clean: zero whole-loop materializations | Zero |
| R1-F1.4 | `EventMetaExpressionLookup::get` hook path | Large response; dotted scalar/nested members | Clean: zero whole-hook materializations | Zero |
| R1-F1.5 | Explicit bare `loop` and bare `hook` reads | Whole root requested | Clean: one materialization, preserving the root value | One |

The binding producers above are the two remaining production callers of the loop object's constructor outside the explicit whole-root condition path. Their consumers retain no opportunity to avoid the copy. The new loop tests exercise only the condition path, so their passing result does not cover these siblings.

**Recommended repair:** complete member-aware resolution through the existing host-global binding boundary, preserving whole-root representation, scope errors, and current-pass values. Merely making the whole root lazy still copies the large sibling on the first scalar read and does not meet the contract. Keep the change confined to this boundary; add L1 work-count and value tests for lifecycle and body/frontmatter consumers. Update the looping documentation's condition-only description when the broader implementation lands. This requires a wider interface change than the implementer chose, but the spec already authorizes the outcome; it does not require another scope decision.

### R1-F2 (medium): Property-union rewriting loses the caller's file-match origin

**Authority: defect**, in the `darkmatter` package's changed [ArmValidators::new](../../../darkmatter/lib/src/markdown/schemas/rewrite.rs:183): the property-arm cache explicitly replaces caller origins with an empty map. A caller-supplied file inside a recursive union can pass the effective schema's file-match validation, then be judged from the document directory during eager rewriting and remain unnormalized. This is also relevant to the spec's union-validator contract requiring document/caller-origin coverage. The log acknowledges that this difference predates the optimization; that does not remove the demonstrated defect from review scope.

**Defect class:** dropping origin information between validation and union-arm selection causes the later normalization decision to reject an already accepted file.

Reproduction uses the existing `schemas_rewrite_arm_validators.rs` temp-file fixture and the recursive schema shape already present in `recursive_schema_caller_files.rs`. Create `a.md` in the document directory and another in its `launch/` child. Supply `spec` as the absolute path of `launch/a.md`, with a `CallerInputRecord` captured at `launch/`. Resolve this schema through `DarkmatterSchemas::effective_for`, then call `EffectiveSchema::validate` and `normalize_frontmatter`:

```yaml
$schema:
  - node: "file(eager; match(./*.md)) | { children: node[] }"
    spec: node(required)
  - spec: number(required)
```

Validation succeeds. Normalization leaves the absolute path unchanged; the eager-file projection should produce `launch/a.md`. A direct root-arm file declaration produces that expected result with the same file and caller origin. This is an unusual combination of caller input, recursion, and a directory-relative glob, so priority is medium. The practical harm is inconsistent, machine-specific path output, rather than selecting the wrong physical file in this reproduction.

The isolated public-API probe varied only the `spec` schema shape and matching value container. All ten cases validated successfully:

| Child ID | Site / shape | Origin tested | Observed normalized `spec` | Expected |
| --- | --- | --- | --- | --- |
| R1-F2.1 | Root arm: direct `file(eager; required; match(./*.md))` | Caller | Clean: `launch/a.md` | `launch/a.md` |
| R1-F2.2 | Reference to recursive union: `node(required)` | Caller | Absolute launch path retained | `launch/a.md` |
| R1-F2.3 | Additional property union: `node \| number` | Caller | Absolute launch path retained | `launch/a.md` |
| R1-F2.4 | Array-item traversal: `node[]` | Caller | Array retains absolute launch path | `["launch/a.md"]` |
| R1-F2.5 | Inline-object traversal: `{ path: node }` | Caller | Object retains absolute launch path | `{ "path": "launch/a.md" }` |
| R1-F2.6 | Root arm: direct eager matched file | Document | Clean: `a.md` | `a.md` |
| R1-F2.7 | Reference to recursive union | Document | Clean: `a.md` | `a.md` |
| R1-F2.8 | Additional property union | Document | Clean: `a.md` | `a.md` |
| R1-F2.9 | Array-item traversal | Document | Clean: `["a.md"]` | `["a.md"]` |
| R1-F2.10 | Inline-object traversal | Document | Clean: `{ "path": "a.md" }` | `{ "path": "a.md" }` |

The value's downstream decisions were checked, not just the cache constructor: effective validation accepts it; root-arm selection accepts it; standalone property-arm selection loses its top-level origin; reference/array/object traversal consequently declines the rewrite; the public normalization result exposes the unchanged absolute value. The free rewrite helper has no caller-origin argument and retains its document-context contract, so it is not an additional failing caller-origin site.

**Recommended repair:** retain the owning top-level property's origin when compiling and testing a standalone property arm, including traversal through references and containers. Include that origin identity in the invocation-local retained-validator key; an arm address alone cannot distinguish uses under properties supplied from different origins. Preserve the existing cache and fresh file checks. Add permanent public-API tests for the table above; do not introduce a filesystem-result cache or change glob semantics. The current origin test only uses direct root arms, which explains why it passes despite this defect.

## Implementation decisions (informational)

No item requires stopping for a human. The spec pre-authorizes routine choices and automatic threshold classification. Where a choice leaves a required behavior incomplete, the findings above supply a repair recommendation rather than asking the author to choose again. The frontmatter lists each choice for optional confirmation after repair; no response is needed to continue.

| Implementer's choice or deferred work | Review assessment |
| --- | --- |
| Take the baseline and comparison under moderate host load after waiting for other work | Accept the measurement method with its explicit limitation. Deterministic counts and allocation evidence remain useful; small timing differences are inconclusive. Do not restart the measurement campaign. |
| Attribute all five shell-site parsing passes to placeholder preparation | Recommend retaining the broader attribution: all five run inside the operation being measured. The 6.4% share is explicitly an estimate. |
| Fix the existing eight-argument lint and three compile-time manifest paths | Retain the mechanical remedies. They do not change the measured production paths. |
| Enable Darkmatter counters in normal test recipes and CI feature lists | Required to keep the counting regressions live; default builds remain separate. |
| Restrict loop optimization to conditions because host bindings need a wider change | Incomplete against the approved leaf-read contract; repair under R1-F1. |
| Keep nested `loop.last_output.<key>` absent in conditions | Preserve the previous exact-member lookup semantics under the public-behavior preservation requirement. This review requests no new expression language. |
| Return early for tasks without `when:` | Retain: no condition needs a sequence object. |
| Compare copied-output counts at two history sizes rather than pinning exact totals | Retain: proves history independence without coupling to incidental event counts. |
| Remove a memo entry after read or parse failure | Retain: freshness and failure behavior are correct; restoring good text costs one new parse. |
| Leave failing imports' discovery/expansion double parse | Accept unchanged error-path work; this does not defeat the successful-revision memo contract. |
| Use exact consumer masks and exact root spellings in the completion design | Retain: preserves each alternative's pruning and displayed path spelling. These remain reversible design choices. |
| Leave completion implementation and possible biscuit-file canonicalization API changes for later work | Required design-only scope; the candidate discusses the remaining per-file cost rather than claiming it is removed. |
| Verify CLI branch lifetimes through a test-build file seam | Retain: tests observe the shipped scheduler without adding a CLI counter feature. |
| Exclude container state from leg counts and copy the seed per admitted leg | Retain: the bound is on active leg branches, not all memory or total constructions. Further storage sharing is unnecessary here. |
| Count provider state construction per task, shared by both hints | Retain: literal hints still construct zero states and expression tasks get fresh state. |
| Preserve task-wide dependency deferral rather than split it per hint | Retain existing prediction behavior; per-key deferral would be a separate behavioral change. |
| Preserve the old property-arm origin mismatch | The demonstrated wrong normalization is R1-F2; repair it without another author ruling. |
| Correct caller origins for root-union arms | Retain: aligns arm selection with validation, within the approved caller-origin contract. |
| Use the shared validator cache in the public rewrite helper | Retain: bounded existing cache, fresh judgments, no new cache category. |
| Include all shared definitions instead of pruning to reachable ones | Retain: complete schemas are correct; unrelated-definition invalidation does not defeat the measured per-item compilation reduction. |
| Return `(report, summary)` with only `has_schema`, using the existing timing recorder | Accept as an internal packaging choice that preserves the requested one-preparation timing and output behavior. The recorder avoids charging uninstrumented callers for metadata collection. |
| Select diagnostic token locations from completed problem codes and properties | Retain: avoids expanding public problem records; location behavior has L1 coverage. |
| Keep pre-shell position maps for reports composition later filters | Accept: the earlier validation report has actual problems; eliminating later filtering work would extend the lazy-provider lifetime beyond this change's requirement. |
| Put alphanumeric caching in `simplified/convert.rs` | Retain: that is where the check actually lives. |
| Accept possible concurrent overcount in the alphanumeric compile counter | Accept as an instrumentation limitation; isolated nextest regressions remain deterministic and validation results are unaffected. |
| Leave the repeated scope-join timing slowdown unmodified | Accept: direct work, allocations, and executed source are unchanged; no demonstrated attributable regression requires repair. |
| Fix existing fences, allocator ownership wording, and the provider-deferral example | Retain the narrow documentation corrections. |
| Record anchor comparisons only in the implementation log | Meets acceptance item 7; duplicating the table in the spike report is unnecessary. |
| Keep full snapshots for whole-state operations and move lexical outputs | Retain: narrows reads without weakening coherent multi-field reads or restoration. |
| Keep bracketed loop access on the whole-root evaluator path | Preserve existing evaluation semantics; R1-F1 concerns eager host binding even for ordinary dotted scalar reads. |
| Assert preparation counts in library tests and CLI behavior in subprocess tests | Retain: one boundary proves counts, the other proves wiring. |
| Format new test files only | No existing-file formatting churn or `cargo fmt` run was found. |
| Close pre-warm reuse and excluded items; create trigger/placeholder follow-up specs | Pre-authorized by the measured threshold rule. Both dated active specs exist; closed items need no deferred implementation here. |
| Rely on CI for other operating systems | Accepted evidence boundary; cross-OS proof is not a readiness condition for this review. |

## Verification and requirement coverage

All changed behavior is computation, filesystem resolution, or scheduler lifetime behavior. L1 is appropriate; this feature adds no terminal-rendering or keyboard-encoding promise requiring new L2/L3 evidence. The implementation log records the existing areas' L2 and default-build checks; this review did not repeat those broad gates.

| Requirement | Strongest relevant verification inspected / run | Assessment |
| --- | --- | --- |
| Group-start isolation, admission/release bounds, declaration-order folding, retries and cancellation | L1 `branch_admission` plus CLI `group_branch_lifetime` and existing container regressions | Library and both focused CLI cases pass |
| Hook leaf equality, reserved roots, freshness, avoided payload construction | L1 `hook_leaf_reads` | Pass |
| Loop leaf reads and current-pass values | L1 `loop_leaf_reads`; additional binding-construction probe | Condition path passes; host paths fail R1-F1 |
| Coherent narrow projections, restoration and history-independent copying | L1 runtime-state tests and `runtime_projections` | Focused projection cases pass |
| Literal prediction, precedence, late/runtime dependencies and expression failures | L1 `literal_hints` | Pass |
| Cached complete union schemas, uniqueness, cold/warm use, changed schema/file state and caller origins | L1 `schemas_rewrite_arm_validators`; public-API origin sweep | Existing cases pass; recursive caller forms fail R1-F2 |
| One final schema preparation, assignments, pending values, errors and output compatibility | L1 `schemas_validation_summary`, CLI summary tests, existing output-format tests | Library and both focused CLI cases pass |
| Lazy positions and diagnostic preservation | L1 `lazy_positions`, existing position/provenance tests | Focused cases pass |
| Latest named-type revision, more than 256 paths, edit/reuse, malformed/deleted files and held tables | L1 memo and named-import tests | Focused schema run passes |
| Cached alphanumeric constraints and diagnostics | L1 `schemas_alphanumeric_check_cache` | Pass |
| Duration units, all three forms, lexical/semantic differences and shipped schema | L1 `content_policy_duration_parity` | All four tests pass |
| Conditional-item terminal outcomes and completion candidate | Report, both follow-up specs, candidate design | Delivered |

Review runs used nextest through repository recipes: 371 tests selected by `schemas_` in Darkmatter; 10 lazy-position tests; 4 duration-parity tests; 10 Claudine leaf tests; 8 admission tests; 4 projection tests; 8 literal-hint tests; 2 Darkmatter CLI summary tests; and 2 Claudine CLI branch-lifetime tests all passed. Both `just check-tier-coverage claudine` and `just check-tier-coverage darkmatter` reported zero stranded tests. New modules are declared in their consolidated targets and counter tests are selected by enabled test features.

Two temporary diagnostic probes confirmed the findings through in-process APIs and process-wide counters. They were removed after the observations were recorded; this review leaves no source/test edits. No production fixes, commits, formatting commands, or focus-taking terminal tests were performed.

## Input robustness

The changed file-reading path is the named-type revision memo. It retains the existing standalone-schema classifier rather than adding a second parser. Read, UTF-8, classification and parse failures remain errors; none serves an old memo entry. Successful reads still hash current bytes before reuse.

Inspected the public named-import tests and classifier delegation: positive tagged/pure-envelope and JSON-compatible YAML controls; malformed YAML; duplicate keys; trailing content; a second YAML document; implicit-null and empty `types`; invalid sibling entries; non-schema targets; oversized numeric admission; and invalid UTF-8. Located errors and CLI no-launch wiring are tested. Grepped the reader for permissive defaults, optional fields, filtered elements, and discarded parse errors; the memo's changed path introduces none. Other matches in the large resolver are existing diagnostic or schema-consumer paths, not new memo fallback behavior. No new configuration field validator is introduced, so a second field-shape matrix is unnecessary.

## Readiness

There are two new findings: R1-F1 is an incomplete performance contract; R1-F2 is a demonstrated normalization defect on an unusual input combination. There are no carried findings, blocked findings, or pending human rulings. Implementation choices remain available for optional author review after the repair cycle.
