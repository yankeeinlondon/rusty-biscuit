---
$schema: feature-review.yaml
ready: false
findings:
    - title: Per-document environment capture and its acceptance tests are not implemented
      priority: high
observations:
    - The scalar ctx example conflicts with the existing composition contract
    - Non-interactive session documentation still names retired harness validation
human_review: true
human_review_items:
    - |-
        The approval decision from the earlier reviews remains unresolved. Choose how a later sequence task uses its newly captured environment when that environment changes previously approved shell command text. Existing approvals cover exact command text; the specification assumes command-template approval, which is not implemented.

        - Retain exact-text approval temporarily and refuse commands whose resolved text changes. Record this temporary departure in the implementation log.
        - Implement command-template approval first, then finish per-document environment capture.
        - Schedule per-document capture separately and explicitly narrow this feature's acceptance requirements before declaring it ready.

        Recommended: retain exact-text approval and refuse changed commands, using the existing enforcement mechanism. This review introduces no new approval decision. The completed namespace migration needs no additional design approval.
has_blocked_findings: true
blocked: true
reviewed_by: codex/gpt-6.1-sol
recurrence: true
created: 2026-10-12T23:11:41-07:00
spec: 2026-10-18-fm-namespace/spec.md
implemented: true
description: A **feature** review of `2026-10-18-fm-namespace/spec.md`
feature: 2026-10-18-fm-namespace/review-3.md
previous: 2026-10-18-fm-namespace/review-2.md
next: 2026-10-18-fm-namespace/review-4.md
log: claudine/features/2026-10-18-fm-namespace/log.md
implemented_by: claude/opus
---

# Review of the `fm` namespace, iteration 3

The feature is **not production ready** under the full specification. The namespace migration remains implemented and the targeted checks pass. Per-document environment capture and its acceptance tests remain deferred behind the same unresolved approval decision. There are no new unblocked findings.

## Previous review and repair disposition

Checked both earlier reviews, the [repair log](log.md), the [implementation inventory](implementation-log.md), and the [plan](plan.md) before reviewing the current code.

| Review 2 item | Verified disposition |
| --- | --- |
| Unblocked findings | Review 2 listed none; there was no independent blocking repair to implement. |
| Blocked environment finding | Not implemented. The repair explicitly deferred the complete connected scope. The plan still requires an approval ruling, the inventory still records it as unresolved, and command-template approval still has no implementation. Nothing establishes that this finding was unblocked before the repair. |
| Hook adapter caller documentation | Addressed: Claudine's dispatch adapter and hook-action skill now name templates, conditions, and matchers. |
| Self-reference documentation | Addressed: Darkmatter's interpolation topic now explains the existing empty-value result and parity between bare, dotted, and bracket reads. The public parity test passes. |
| Scalar `ctx` example | Left for the author because the restriction predates this feature; remains an observation. |

The repair reports **zero disputed findings**. No disputed claim is being re-raised. Review 2's `implemented: true` records the completed repair attempt; its log makes clear that the high finding itself was deferred.

## Unblocked Findings

None. No new high or medium defect was established in the completed namespace migration.

## Blocked Findings

### High — Per-document environment capture and its acceptance tests are not implemented

**Authority: spec.** Decided section 4 requires “Each new document captures its own: each sequence task, each group leg, and each `proxy` target,” before `initialize`, with every environment consumer using that document's map. Acceptance criterion 5 requires supplied-map tests of these entry boundaries, agreement among expressions, references, and approvals, and stability throughout events, loops, and retry/resume. The production routes and acceptance tests still do not implement that contract.

**Defect class:** environment consumers use invocation-owned or live process inputs rather than one map owned by each document lifecycle.

The specification explicitly requires a first document under supplied map A and a later task, group leg, or proxy target under supplied map B. A new epoch currently changes bookkeeping while retaining the invocation environment. Updating only a composition context's environment does not update its file-resolution context or every other consumer. The finding is high because an explicit acceptance criterion remains unmet, independently of how often a parent process's environment changes in ordinary use. A child shell's `export` is not a reproduction: it cannot update the parent environment.

The bounded sweep carries every remaining entry route and consumer from reviews 1 and 2. The same A/B public composition control was executed successfully. Production rows below are **source inspections**, not claimed runtime reproductions: these routes lack the per-document supplied-map input needed to run that control through them. No process environment was mutated to manufacture a demonstration.

| Site | Shape checked | Observed result | Expected result |
| --- | --- | --- | --- |
| Darkmatter public [environment replacement test](../../../darkmatter/lib/tests/l1/request_context_builder.rs:255) | First document and include under A; second request under B | **Clean, executed:** expression output, context environment, and file references agree; the first request remains unchanged | Preserve this replacement capability and connect it to document entry |
| Claudine [invocation capture](../../lib/src/invocation_context.rs:1003) and [epoch entry](../../lib/src/invocation_context.rs:910) | Enter a second document under B | Environment is stored once from the request snapshot; a new epoch retains the invocation and accepts no replacement map | Give each lifecycle its own map while retaining unrelated launch facts |
| Claudine [staged bootstrap and re-preparation](../../cli/src/commands/wrap/composition/staged_boot.rs:338) | Capture before initialize, then reuse | Extends the retained launch context; no document-map capture is introduced | Capture before environment-dependent bootstrap and initialize, then reuse that map |
| Claudine [task preparation and serial member entry](../../cli/src/commands/wrap/sequence/task_run.rs:209) | Successive tasks or serial group legs under A/B | Captures launch context from the invocation; overrides affect the composed context only | Capture each task/leg map and distribute it to all consumers |
| Claudine [parallel group entry](../../cli/src/commands/wrap/sequence/task_run.rs:449) and member preparation in that module | Distinct supplied maps for group legs | Group context still comes from the invocation; no per-leg map replacement | Share repository evidence while keeping each leg's environment independent |
| Claudine [proxy commit](../../lib/src/composition/coordinator/commit.rs:86), [target resolution](../../lib/src/composition/lifecycle/control.rs:291), and wrapper adoption | Source under A; target lifecycle under B | Source resolution uses the retained request context; subsequent preparation retains invocation inputs | Resolve the source's target consistently, then enter the target under its own map |
| Claudine [provider/model selection](../../lib/src/composition/select.rs:155) | Supplied document map differs from process values | Public selection routes still supply `std::env::var` readers | Select using the document map during preparation |
| Claudine [loop settings](../../lib/src/composition/looping/config.rs:18) | Supplied fail-fast, iteration-limit, and pause-margin inputs | Reads live process variables | Read the same document inputs across repeated preparation |
| Claudine [lazy evidence refresh](../../lib/src/invocation_context.rs:1663) | Document environment differs from later process values | Builds evidence with `std::env::vars()` | Environment-derived evidence uses the document map; permitted other observations may refresh |
| File references, `SCHEMAS_DIR`, and approval preparation reached by these routes | Expressions, resolved paths, schemas, and commands under one B map | No complete document-map replacement is routed to every consumer | Use one map consistently and enforce the selected approval contract |
| Claudine [sequence snapshot test](../../lib/src/composition/sequence/tests.rs:588) and [proxy snapshot test](../../lib/src/composition/lifecycle/control/tests.rs:454) | Retained request map versus ambient changes | Existing tests prove the older request contract, not entry of another document under B | Supplied-map acceptance tests through task, group, and proxy entry |
| Released `current_env` classification, refresh scanner, preflight, nesting errors, approval bindings, and editor projection | Present and absent ordinary property; fixed-root catalog | **Clean repaired siblings:** no live/reserved behavior; ordinary-property tests pass; catalog is `fm, ctx, env, current` and late list is `["current"]` | Preserve the completed removal |

**Required verification:** Level 1 tests through the actual document entry routes, using supplied maps without modifying the process environment. Cover capture before initialize, each task, serial and parallel group legs, each proxy target, and reuse through events, approvals, provider preparation, iterations, retry/resume, failure handlers, and finalize. Transclusion must reuse its including document's map. Assert public outputs and resolved references/schema inputs/approved command text, rather than only checking a parser or context helper. The existing Darkmatter replacement control proves feasibility, not these missing routes.

**Blocker:** the plan's rule “Approval ordering needs an explicit ruling” states, “An unresolved ruling blocks completion of the affected environment work.” The repair rechecked this prerequisite and deferred the whole scope. The outstanding options remain in `human_review_items`; this non-interactive review does not select one on the author's behalf. Once resolved, implement the entire table together, or explicitly narrow the specification if the environment work is rescheduled. No additional namespace repair is required by this finding.

## Recurrence

This repeats **“Per-document environment capture and its acceptance tests are not implemented”** from reviews 1 and 2. Those reviews already named invocation ownership, bootstrap, task and serial/parallel group entry, proxy adoption, selectors, loop settings, lazy evidence, references/schema/approval consumers, and supplied-map route tests. The repairs deliberately deferred those siblings; no newly overlooked sibling or expanding edge case caused this recurrence. The released-root removal and terminal-test update were implemented in cycle 1. This review retains the complete outstanding scope as one finding and sets `recurrence: true`.

## Observations

### The scalar ctx example conflicts with the existing composition contract

The specification illustrates `ctx: notes`, but Darkmatter's existing [context merge](../../../darkmatter/lib/src/markdown/compose/context/merge.rs:61) rejects a non-object document `ctx` without override permission. The public namespace tests use a mapping and verify that `fm.ctx` reads document data independently of merged context; lifecycle tests also verify qualified scalar access after state construction. This restriction predates the feature. The author can use a mapping in future examples or schedule a merge-contract change; it is not another repair requirement for this cycle.

### Non-interactive session documentation still names retired harness validation

Claudine's [non-interactive session topic](../../docs/topics/non-interactive-sessions.md:3) still lists the retired harness validation system as a capability. The repair log identifies this separate, unchanged page. The edited dispatch adapter and hook-action skill now describe their actual callers correctly; the older topic sentence can be removed during independent documentation maintenance.

## Verification and scope

| User-facing requirement | Required level and evidence |
| --- | --- |
| `fm` snapshots, dotted/bracket reads, ordering, overrides, missing values, and shell-token selection | Level 1 public composition and lookup tests; targeted runs passed |
| Ordinary `doc`/`current_env` properties and fixed-root precedence | Level 1 public composition/catalog tests; targeted runs passed |
| Lifecycle globals shadow properties; `set` updates later reads and preserves earlier snapshots | Level 1 lifecycle execution; targeted binding tests passed |
| Hook payload rename across templates, conditions, matchers, and reports | Level 1 consumer tests; eight targeted tests passed; inspected retained unprefixed/nested payload behavior |
| Editor completion, hover, ordinary-property treatment, and passive tooling | Level 1 language-server requests and instrumentation; namespace editor tests passed; instrumentation is enabled by the area recipe |
| Frozen clarification prompt migration | Level 1 shipped CLI composition using the frozen fixture; passed |
| Per-document environment capture and lifecycle reuse | Level 1 production-route tests required; missing as described above |
| Retained lazy `current` behavior in an actual terminal | Level 2 required by criterion 8; cycle 1 log records seven executed tmux tests with no skips; not rerun in this documentation-only repair review |

Executed through package recipes and nextest: Darkmatter `just test fm_namespace` (31 passed), `just test fm_` (47 passed), and `just test an_environment_only_replacement` (1 passed); Claudine `just test hook_namespace` (8 passed), `just test binding_contract` (12 passed), `just test fm_` (11 passed), and `just test compose_frozen_clarify_reads_its_ordinary_doc_property` (1 passed). Filters overlap; these are run results, not a count of unique tests. Root `just check-tier-coverage claudine` reports zero stranded tests. Reviewed consolidated L1 declarations and editor instrumentation feature selection. None of the executed tests exceeded five seconds.

The earlier repair log records full L1/lint passes and the required Level 2 run, plus opt-in prompt checks with unrelated existing failures. This repair changed only comments and documentation; this review did not repeat broad gates or open terminal/browser windows. Claudine's existing `__eh_frame` linker warning appeared during targeted runs. Cross-OS evidence remains CI's responsibility and does not determine this readiness result.

Reviewed changed root readers, literal-bracket dependency collection, effective-state frontmatter/context separation, migration searches, hook consumers, editor projections, and the full remaining environment inventory. Dotted `fm` traversal copies only the selected value; no material lookup cost regression was found. The input robustness matrix does not apply: the feature changes expression grammar/lookup and environment ownership, not a file-format or configuration reader. This review changes only review/spec metadata and adds this review document.
