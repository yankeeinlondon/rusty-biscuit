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
created: 2026-10-27T16:21:53-07:00
spec: 2026-10-25-pipeline-work-reduction/spec.md
implemented: false
description: "A **feature** review of `2026-10-25-pipeline-work-reduction/spec.md`"
feature: 2026-10-25-pipeline-work-reduction/review-2.md
previous: 2026-10-25-pipeline-work-reduction/review-1.md
---

# Review 2: pipeline work reduction

The feature is **production ready** against the specification. Both previous findings are verified fixed, including every child instance in the repair ledger. No new findings or unresolved decisions remain.

Read the specification, previous review, repair `log.md` and its finding ledger, original `implementation-log.md`, spike report, and completion-walk design. Reviewed the current implementation and uncommitted repair diff; completion flags were not treated as evidence. The previous review contains no human rulings. Its accepted implementation choices remain accepted.

## Previous Finding Verification

Child IDs below are the required instances in `log.md` → “Finding Ledger.” A control was already correct; its outcome means the behavior remains verified, rather than that the repair changed it.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| R1-F1: Lifecycle and body loop bindings still copy the whole previous result | R1-F1.1–R1-F1.7 | verified fixed | All seven instances checked below. Both binding producers resolve individual members, and ambient construction shares the previous result. | Nothing |
| R1-F1.1: Lifecycle bindings | Construction, scalar reads, task/container events | verified fixed | Claudine's [binding producer](../../lib/src/composition/lifecycle/bindings.rs) uses member-aware bindings. Passing `loop_member_reads` tests check construction and scalar interpolation beside a one-megabyte result, including the real executor event path: zero whole-loop materializations. | Nothing |
| R1-F1.2: Composition host globals | Construction, body/frontmatter, inherited globals | verified fixed | The host-global producer uses the same binding. Passing `scalar_loop_reads_in_body_and_frontmatter_build_no_loop_object` uses `prepare_direct` and checks values and zero materializations; Darkmatter's `host_globals_compose` also checks transclusion. | Nothing |
| R1-F1.3: Condition lookup | Dotted scalar members beside a large previous result | verified fixed | Control: `loop_leaf_reads` passes; direct member lookup remains in Claudine's [loop expression adapter](../../lib/src/composition/looping/expression.rs). | Nothing |
| R1-F1.4: Hook lookup | Scalar/nested, reserved and missing members | verified fixed | Control: all six `hook_leaf_reads` tests pass. The closed member resolver still excludes `hook.env` and prevents frontmatter fallback. | Nothing |
| R1-F1.5: Explicit whole-root reads | Bare loop/hook, bracket access, nested projections and missing members | verified fixed | Loop and hook controls pass. Bare loop reads build one object per session; bracket access uses that path. Darkmatter's seven `member_aware` cases verify nested/indexed reads, null/missing values, shadowing, whole-root reuse and unavailable bindings. | Nothing |
| R1-F1.6: Ambient construction | Repeated frame-to-ambient projections | verified fixed | Claudine's [loop frame](../../lib/src/composition/looping/frame.rs) and ambient hold `Arc<Value>`. Passing `every_ambient_shares_the_frame_result_without_copying_it` checks pointer equality. Pass recording/retry retains the previous revision. | Nothing |
| R1-F1.7: Binding consumers | Resolution, kind/debug, host identity, declaration agreement, subtree registration | verified fixed | Inspected every production enum match in Darkmatter's [binding module](../../../darkmatter/lib/src/markdown/compose/expression/binding.rs) and [subtree adapter](../../../darkmatter/lib/src/markdown/compose/subtree.rs). All handle member bindings; registration invokes no provider. Identity follows the existing lazy-binding strategy. Binding and host-global tests pass. | Nothing |
| R1-F2: Property-union rewriting loses the caller's file-match origin | R1-F2.1–R1-F2.16 | verified fixed | All sixteen instances checked below. Property arms retain validators by arm and owning origin; public validation and normalization agree. | Nothing |
| R1-F2.1: Caller-origin direct root file | Direct eager file with `match(./*.md)` | verified fixed | Control: `caller_origin_direct_root_arm_file` validates and normalizes to `launch/a.md`. | Nothing |
| R1-F2.2: Caller-origin recursive reference | `node(required)` | verified fixed | `caller_origin_reference_to_recursive_union` validates and normalizes to `launch/a.md`. | Nothing |
| R1-F2.3: Caller-origin property union | `node \| number` | verified fixed | `caller_origin_property_union_with_reference` validates and normalizes to `launch/a.md`. | Nothing |
| R1-F2.4: Caller-origin array traversal | `node[]` | verified fixed | `caller_origin_list_of_recursive_union` validates and normalizes to `["launch/a.md"]`. | Nothing |
| R1-F2.5: Caller-origin object traversal | `{ path: node }` | verified fixed | `caller_origin_inline_object_of_recursive_union` validates and normalizes to `{ "path": "launch/a.md" }`. | Nothing |
| R1-F2.6: Document-origin direct root file | Direct eager matched file | verified fixed | Control: `document_origin_direct_root_arm_file` yields `a.md`. | Nothing |
| R1-F2.7: Document-origin recursive reference | `node(required)` | verified fixed | Control: `document_origin_reference_to_recursive_union` yields `a.md`. | Nothing |
| R1-F2.8: Document-origin property union | `node \| number` | verified fixed | Control: `document_origin_property_union_with_reference` yields `a.md`. | Nothing |
| R1-F2.9: Document-origin array traversal | `node[]` | verified fixed | Control: `document_origin_list_of_recursive_union` yields `["a.md"]`. | Nothing |
| R1-F2.10: Document-origin object traversal | `{ path: node }` | verified fixed | Control: `document_origin_inline_object_of_recursive_union` yields `{ "path": "a.md" }`. | Nothing |
| R1-F2.11: Retention distinguishes origins | Two properties, two origins, twenty items each | verified fixed | `caller_origin_and_document_origin_properties_keep_their_own_arm_validators` passes: correct paths, four cold compilations, zero warm compilations. | Nothing |
| R1-F2.12: One shared arm address | Same arm reached from two property origins | verified fixed | Passing unit test `a_shared_arm_is_judged_from_each_owning_property_origin`; inspected retention by both address and origin in Darkmatter's [rewrite module](../../../darkmatter/lib/src/markdown/schemas/rewrite.rs). | Nothing |
| R1-F2.13: Root-union validators | Complete instance with property-keyed origins | verified fixed | Control: root-arm compilation retains the effective schema's caller-origin map; direct-root and separate-origin tests pass. | Nothing |
| R1-F2.14: Free rewrite helper | Document context without caller records | verified fixed | Control: the helper accepts no caller origins and starts with document ownership; rewrite tests pass. | Nothing |
| R1-F2.15: Effective validation | Verdict before normalization | verified fixed | Every public origin row asserts `EffectiveSchema::validate` accepts the instance before checking normalized output. All seventeen rewrite-arm tests pass. | Nothing |
| R1-F2.16: Other origin-sensitive decisions | Compose file-mode selection, root applicability, coercion | verified fixed | Inspected Darkmatter's [composition schema adapter](../../../darkmatter/lib/src/markdown/compose/schema_validation.rs): file-mode selection removes `match` and uses the supplied record context; root applicability passes caller origins; structural coercion makes no origin-sensitive file judgment. These contracts rule out the dropped-origin defect at these sites. | Nothing |

The repair updates canonical looping/expression documentation to describe member resolution and schema documentation to describe owning-property origins. Both agree with the implementation. No partial repairs or newly discovered siblings remain.

## Unblocked Findings

None.

## Blocked Findings

None. No item requires an author decision before repair.

## Recurrence

None. Both previously reported classes are repaired across the required sites; this review adds no finding in either class.

## Specification coverage

These requirements concern computation, scheduling lifetimes, schema preparation and filesystem judgments. L1 is the appropriate verification level. The feature adds no keyboard, terminal-rendering, mouse or browser behavior requiring new L2/L3 evidence. Existing broad area, L2 and default-build gate results are recorded in the implementation logs; this review independently reran the focused checks below. Cross-OS proof remains CI evidence, not a readiness condition here.

| Requirement | Verification level and evidence | Assessment |
| --- | --- | --- |
| Group-start isolation, admission/release bounds and declaration-order results | L1 `branch_admission` and CLI `group_branch_lifetime`; inspected private-branch lifetime and result folding | Pass; covers capped/uncapped groups, nested work, retry, interruption and fail-fast |
| Hook/loop leaf reads, current values and unchanged whole roots | L1 hook, loop binding/condition/event/body, frame sharing, member binding and transclusion tests | Pass; closes R1-F1 |
| Coherent narrow runtime reads, retry clearing and lexical restoration | L1 `runtime_projections` and loop/retry cases; inspected lock-scoped projections and moved sequence scopes | Pass |
| Literal provider hints, precedence, runtime dependencies and launch-time authority | L1 `literal_hints`; inspected classification before lazy state construction and retained dependency analysis | Pass |
| Complete cached union validators, unique accepting arm, fresh file judgments and caller origins | L1 `schemas_rewrite_arm_validators` and shared-arm origin test, including changed schema/file controls | Pass; closes R1-F2 |
| One final schema preparation, separate assignment preparation and preserved output/errors | L1 `schemas_validation_summary` and CLI `schema_validate_summary`; inspected CLI consumption of the shared operation | Pass |
| Lazy positions with unchanged diagnostics and source mapping | L1 `lazy_positions`, including baseline, unions, aliases, CRLF, original/constructed sources and shared definitions | Pass |
| Latest named-type revision without whole-cache clearing or stale failure results | L1 `schemas_named_type_memo` and named-import errors; inspected read/hash/parse/update and held ownership | Pass; includes 300 paths, edit/reuse, malformed/deleted files and earlier held results |
| Structural alphanumeric-validator reuse and constraint/provenance preservation | L1 `schemas_alphanumeric_check_cache` | Pass |
| Duration lexical parity across units and baseline forms | L1 `content_policy_duration_parity`, with semantic exceptions, roster guards and shipped schema validation | Pass |
| Measurements, terminal classifications and completion candidate | Reviewed anchor comparison/uncertainty, refreshed report, qualifying follow-up specs and completion design | Delivered |

The completion candidate remains a design deliverable. It preserves each alternative's pruning and ordering, separates distinct roots/policies, keeps observations request-local, and accounts for per-file canonicalization and its remaining cost. No completion implementation is claimed. The trigger-discovery and placeholder-parsing follow-up specs exist; pre-warm reuse and excluded candidates have terminal no-change classifications. The original log records all anchor comparisons and timing uncertainty; no new measurement campaign is required.

Counters remain feature-gated. Area recipes and CI test features enable the counting regressions; default library/CLI builds are separately recorded in the repair log. New tests belong to compiled unit or consolidated L1 targets. Both package areas report zero stranded tests.

## Input robustness

The changed configuration-reading path remains the named-type revision memo. It delegates to the existing standalone-schema classifier; this repair adds neither a parser nor a field validator. Origin records and runtime bindings are typed internal data and introduce no new file-format matrix.

Successful memo reads hash current bytes before reuse. Read, UTF-8, classification and parse failures return errors rather than an old table. The public memo tests retain positive controls and check malformed-after-good, cold malformed, deletion, repair and held revisions. The named-import error suite checks duplicate keys, trailing content, multiple YAML documents, null/empty type declarations and malformed sibling entries. The earlier review's accepted classifier contract is unchanged.

Repeated the permissive-parse smell search in the resolver. The changed memo loading functions contain no `serde(default)`, optional-field deserializer, `filter_map`, `unwrap_or_default`, or discarded parse error. Matches elsewhere belong to existing diagnostic/consumer paths; the memo adds no empty-table or stale-revision fallback.

## Verification performed

Ran nextest through each area's `just test` recipe from the absolute session worktree directory:

- Claudine: `loop_` (598), `hook_leaf_reads` (6), `every_ambient_shares` (1), `branch_admission` (8), `runtime_projections` (4), `literal_hints` (8), `group_branch_lifetime` (2).
- Darkmatter: `schemas_rewrite` (17), `member_aware` (7), `host_globals_compose` (8), `a_shared_arm_is_judged` (1), `schemas_named_type_memo` (5), `schemas_alphanumeric_check_cache` (4), `schemas_validation_summary` (9), `lazy_positions` (10), `content_policy_duration_parity` (4), `schema_validate_summary` (2), `schemas_named_import_errors` (9).
- Root: `just check-tier-coverage claudine` and `just check-tier-coverage darkmatter`, both with zero stranded tests.

All passed: 703 test executions, with overlap between selections. An initial combined filter was interpreted by the recipe shell as pipelines and failed before testing; the single-filter runs above replaced it. The pre-existing macOS linker warning about `__eh_frame` appeared during test builds.

No production fixes, test-source changes, formatting commands, commits or focus-taking terminal tests were performed. Only this review and the requested review/spec metadata were written.
