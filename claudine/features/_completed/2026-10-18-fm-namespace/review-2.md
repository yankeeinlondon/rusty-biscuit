---
$schema: feature-review.yaml
ready: false
findings:
    - title: Per-document environment capture and its acceptance tests are not implemented
      priority: high
observations:
    - The scalar ctx example conflicts with the existing composition contract
    - Self-reference preserves empty-value behavior rather than a cycle error
    - Hook adapter documentation names a caller that does not exist
human_review: true
human_review_items:
    - |-
        The approval decision requested in review 1 is still unresolved. Choose how later sequence tasks may use a newly captured environment when its values change previously approved command text. Existing approval records cover exact command text; the spec assumes command-template approval, which is scheduled later.

        - Keep exact-text approval temporarily and refuse a task if its environment changes approved command text. Record this temporary departure in the implementation log.
        - Implement command-template approval first, then finish per-document environment capture as specified.
        - Separate per-document environment capture from this feature and schedule it after command-template approval. Explicitly narrow this feature's acceptance requirements before declaring it ready.

        Recommended: retain exact-text approval and refuse changed commands. This uses the existing enforcement mechanism. No new decision was made in this review, and the completed namespace work needs no further design approval.
has_blocked_findings: true
blocked: true
reviewed_by: codex/gpt-6.1-sol
recurrence: true
created: 2026-10-12T23:00:02-07:00
spec: 2026-10-18-fm-namespace/spec.md
implemented: true
log: claudine/features/2026-10-18-fm-namespace/log.md
implemented_by: claude/opus
description: A **feature** review of `2026-10-18-fm-namespace/spec.md`
feature: 2026-10-18-fm-namespace/review-2.md
previous: 2026-10-18-fm-namespace/review-1.md
next: 2026-10-18-fm-namespace/review-3.md
---

# Review of the `fm` namespace, iteration 2

The feature is **not production ready** under the full specification. The unblocked namespace migration and removal of the live `current_env` root are implemented. The remaining high finding concerns per-document environment ownership and its acceptance tests; the repair explicitly deferred this connected work pending the approval decision already requested in review 1.

## Previous review and repair disposition

[Review 1](review-1.md) put both findings under `Findings`, with the environment finding marked partially blocked. It did not contain separate `Unblocked Findings` and `Blocked Findings` sections. This review checked the actual dispositions, the [repair log](log.md), and the [implementation inventory](implementation-log.md).

| Previous finding or portion | Repair status verified in this review |
| --- | --- |
| Namespace rename, editor support, and artifact migration | Implemented: Darkmatter uses `fm`; Claudine hook dispatch uses `hook`; ordinary `doc` properties resolve independently; authored catalogs, clarification prompt copies, editor projections, docs, and skills are migrated. Targeted public composition, lifecycle, hook, and language-server tests pass. |
| Removal of reserved/live `current_env` behavior | Implemented: the catalog is exactly `fm, ctx, env, current`; refresh machinery is removed; Claudine's late-namespace list is `["current"]`; ordinary present/absent properties and approval use are covered. |
| Existing Level 2 lazy-root test | Updated to `current` only. The repair log records seven actual tmux test executions with no skips. |
| Per-document environment capture and consistent consumers | Explicitly deferred; the production routes still use invocation-owned or live process inputs. |
| Approval decision blocking the connected environment work | Still unresolved. Neither the repair log nor the plan records an author ruling or implementation of command-template approval. |

The repair reports **zero disputed findings**. Its notes about scalar `ctx` and self-reference are considered below with the existing-code observation rule; neither is silently promoted into a new blocking requirement.

## Unblocked Findings

None. No new high or medium defect was established in the repaired namespace migration. Independent work from the previous review is addressed. The remaining environment implementation can be designed and prepared, but completing its integrated contract requires the outstanding approval disposition.

## Blocked Findings

### High — Per-document environment capture and its acceptance tests are not implemented

**Authority: spec.** Decided section 4 requires “Each new document captures its own: each sequence task, each group leg, and each `proxy` target,” before `initialize`, and requires every environment consumer of one document to use that captured map. Acceptance criterion 5 requires supplied-map tests of these entry boundaries, agreement among expressions, file references, and approvals, and stable inputs across events, loops, and retry/resume. These requirements remain unimplemented and untested through the production entry routes.

**Defect class:** environment ownership is still invocation-wide or read from the live process, instead of consistently captured at each document entry.

The specification's own realistic scenario is map A for one task and map B for the next document. A later task must see B through expressions, context values, references, schema selection, and approval preparation, then keep B for its entire lifecycle. Creating another `DocumentEpoch` currently changes run bookkeeping without replacing the invocation's environment. Applying target overrides only to a composed context is also insufficient to establish one map shared by all consumers.

The bounded sweep below includes all entry routes and consumers inventoried in review 1. The supplied-map public API control was executed; production rows are source inspections, not claimed runtime reproductions. The missing entry seam prevents supplying A/B through those routes, and this review introduced no process-environment mutation to simulate it.

| Site | Shape checked | Observed result | Required result |
| --- | --- | --- | --- |
| Darkmatter public request test, [`an_environment_only_replacement_enters_a_document_under_another_map`](../../../darkmatter/lib/tests/l1/request_context_builder.rs:255) | Supplied A in the first document and its include; explicit replacement with B for another document | **Clean control, executed:** expressions, context environment, and references agree; the first request remains unchanged | Retain this API capability and wire replacement into production document entry |
| Claudine [`InvocationContext`](../../lib/src/invocation_context.rs:1003) and [`begin_document_epoch_in`](../../lib/src/invocation_context.rs:910) | Stored map versus a newly entered epoch | Invocation stores `snapshot.env()` once; the epoch retains the same invocation without a document environment input | Separate document-owned environment from unrelated launch-owned facts |
| Direct/wrapper bootstrap and post-initialize preparation, [`staged_boot`](../../cli/src/commands/wrap/composition/staged_boot.rs:338) | Initial context and subsequent extension | Extends the retained epoch/launch context; no new document-map contract is established | Capture before initialize, then reuse that document's map on re-preparation |
| Sequence tasks and serial group legs, [`prepare_task_context`](../../cli/src/commands/wrap/sequence/task_run.rs:209) and `open_member_run` in the same module | Successive documents under A/B | Capture from the same invocation; target overrides update the composed context only | Each task/leg gets its own supplied or CLI-captured map, distributed to every consumer |
| Parallel group legs, `open_group_run` and member preparation in [`task_run`](../../cli/src/commands/wrap/sequence/task_run.rs:449) | Different maps for distinct legs | Group capture uses the invocation; no per-leg environment replacement | Preserve shared repository evidence while giving each leg its own environment |
| Proxy resolution/adoption, [`commit_proxy`](../../lib/src/composition/coordinator/commit.rs:86), [`resolve_proxy_target`](../../lib/src/composition/lifecycle/control.rs:291), and wrapper preparation | Source map A, entered target map B | Existing request context selects the target; subsequent epochs retain invocation environment | Define the source-selection map and capture B for the target lifecycle without refreshing unrelated request facts |
| Provider/model selectors in [`select.rs`](../../lib/src/composition/select.rs:161) | Supplied document map differs from process values | Public selectors still use `std::env::var` closures | Read the document map during preparation |
| Loop environment settings in [`looping/config.rs`](../../lib/src/composition/looping/config.rs:19) | Fail-fast, iteration limit, and pause margin | Still read `std::env::var` | Read stable document inputs through repeated loop preparation |
| Lazy agent/model evidence in [`refresh_evidence`](../../lib/src/invocation_context.rs:1663) | Captured document map versus later process values | Still builds evidence with `std::env::vars()` | Environment-derived facts use the document map; permitted non-environment facts may refresh |
| File-reference, schema-root, and shell-approval consumers reached by these routes | Same document map for references, `SCHEMAS_DIR`, expressions, and command text | Existing request snapshot is retained; no complete document replacement is routed to these consumers | One shared document map, with changed command text refused or checked under the chosen approval contract |
| Retained sequence/proxy snapshot tests in [`sequence/tests.rs`](../../lib/src/composition/sequence/tests.rs:588) and [`control/tests.rs`](../../lib/src/composition/lifecycle/control/tests.rs:454) | Snapshot differs from ambient environment | Test the older request-snapshot contract; they do not exercise entry of a second document | Supplied-map acceptance tests through actual task, group, and proxy entry routes |
| Released `current_env`, catalog, refresh scanner, preflight, nesting diagnostics, approval declarations, and editor classification | Ordinary property present/absent; no built-in classification | **Clean repaired siblings:** present value resolves from frontmatter; absence renders empty with the ordinary advisory; catalog and approval lists exclude the retired root | Preserve these completed repairs |

**Required verification level:** Level 1 supplied-map tests through document entry and reuse routes. A context-replacement feasibility test does not prove those routes perform replacement. Cover capture before initialize, each task/group/proxy entry, and reuse through every event, provider preparation, loop iteration, retry/resume, handlers, finalize, and transclusion. Assert same-map outputs and resolved paths, including schema and approval consumers, without mutating the real process environment. The retained `current` terminal behavior separately has the specified Level 2 evidence.

**Blocker and remedy:** the [plan's approval rule](plan.md) says an unresolved ruling blocks completion of the affected environment work. The repair invoked that rule and deferred the connected routing and consumer changes together. This review preserves that blocker rather than authorizing a new approval contract. After the author chooses an option listed in `human_review_items`, implement the complete bounded list above, or explicitly narrow the specification if this work is rescheduled. Do not mark the full feature ready while criterion 5 still requires it.

## Recurrence

This is the same class as review 1's **“Per-document environment capture and its acceptance tests are not implemented.”** The prior sweep already named invocation capture, task and serial/parallel group entry, proxy entry, provider/model selection, loop configuration, lazy environment-derived evidence, and the supplied-map acceptance tests. Those sites remain unresolved; the repair deliberately deferred them rather than overlooking a newly discovered sibling. The namespace removal and Level 2 siblings were fixed. This review carries the complete remaining list together, so the next repair should not address only one consumer. `recurrence: true` records the outstanding class, not an expanding requirement or a new edge case introduced by the repair.

## Observations

### The scalar ctx example conflicts with the existing composition contract

Decided section 2 illustrates `ctx: notes`, but Darkmatter's unchanged [`merge_ctx`](../../../darkmatter/lib/src/markdown/compose/context/merge.rs:61) rejects a non-object document `ctx` unless override is enabled. The new public composition test uses a mapping and proves that `fm.ctx` reads the document value rather than merged runtime context; the lifecycle test also proves scalar qualified lookup after state construction. The scalar composition restriction predates this feature. Recommend changing the explanatory example to a mapping, or scheduling an explicit change to the merge contract; do not infer a new scalar-input policy from this rename.

### Self-reference preserves empty-value behavior rather than a cycle error

The spec says self-reference “keeps the existing cycle error,” but the existing bare-name behavior has no such error. The executed public [`self_reference_through_fm_matches_bare_self_reference`](../../../darkmatter/lib/tests/l1/fm_namespace_contract.rs:172) test gives `[x/tail]` for `a: "x{{ a }}/tail"` and the dotted/bracket `fm` equivalents. This preserves the old result. Record the limitation and correct future explanations; adding cycle rejection would change existing interpolation behavior beyond a namespace rename.

### Hook adapter documentation names a caller that does not exist

Claudine's [`dispatch::expression`](../../lib/src/dispatch/expression.rs:13) module still says its adapter serves harness validation. The implementation inventory and current callers show templates, matchers, and hook conditions, with no separate harness-validation caller. Report-message tests exercise the shared template path. The comment predates this change and should describe those actual consumers; there is no missing independent harness-message path to implement or test.

## Verification and scope

Reviewed the specification, both implementation logs, review 1, production root readers and their callers, dependency collectors, editor completion/hover and advisory classification, authored catalogs, current-document migration searches, and the complete environment inventory above. No production rename defect or material dotted-lookup cost increase was found. Qualified `fm` traversal copies the selected value; bare `fm` takes a snapshot.

| User-facing requirement | Appropriate level and evidence |
| --- | --- |
| `fm` reads, literal-bracket ordering, overrides, absent values, and ordinary released names | Level 1 public composition tests; targeted runs passed |
| Lifecycle globals shadow document properties; `set` updates later reads while earlier snapshots remain unchanged | Level 1 lifecycle executor tests; targeted binding-contract run passed |
| Hook templates, conditions, matchers, missing fields, and report messages | Level 1 adapter and consumer tests; targeted hook-namespace run passed; inspected nested payload and unchanged bare-field coverage |
| Editor completion, hover, ordinary-property diagnostics, and passive behavior | Level 1 language-server requests and effects instrumentation; targeted editor/advisory runs passed |
| Frozen clarification prompt and authored schema migration | Level 1 normal CLI composition and schema-corpus coverage; frozen prompt test rerun in this review |
| New document environment capture and whole-lifecycle reuse | Level 1 route coverage required; incomplete as described in the high finding |
| Retained lazy `current` behavior in a real terminal | Level 2; repair reports seven executed tmux tests, no skips; not rerun here |

Commands run in this review use nextest through the package recipes: Darkmatter `just test fm_namespace` (31 passed), `just test fm_` (47 passed), `just test an_environment_only_replacement` (1 passed), and `just test undeclared_property` (12 passed); Claudine `just test hook_namespace` (8 passed), `just test fm_` (11 passed), `just test binding_contract` (12 passed), and `just test compose_frozen_clarify_reads_its_ordinary_doc_property` (1 passed). Root `just check-tier-coverage claudine` reports zero stranded tests. The new public namespace and editor tests are declared by their consolidated L1 targets; their names select L1, and the editor instrumentation feature is enabled by the area recipe.

The repair log separately records full Darkmatter/Claudine L1 and lint passes, the Level 2 run, and opt-in prompt checks with pre-existing unrelated prompt failures. Those full gates were not rerun for this documentation-only review. Cross-OS execution evidence is left to CI and does not affect this readiness decision. No terminal/browser window was opened by this review, and no production code or tests were changed.

The input robustness matrix does not apply: this change alters expression lookup and in-memory environment ownership, not a file-format/configuration deserializer. Authored schema key/description migrations do not change schema-loader input handling.
