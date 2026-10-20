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
        Choose how a later sequence task uses its newly captured environment when that environment changes previously approved shell command text. Existing approvals cover exact command text; the specification assumes command-template approval, which is not implemented. This is the same unresolved decision from the earlier reviews.

        - Retain exact-text approval temporarily and refuse commands whose resolved text changes. Record this temporary departure in the implementation log.
        - Implement command-template approval first, then finish per-document environment capture.
        - Schedule per-document capture separately and explicitly narrow this feature's acceptance requirements before declaring it ready.

        Recommended: retain exact-text approval and refuse changed commands using the existing enforcement mechanism. Another unchanged review/repair cycle cannot resolve this decision. The completed namespace migration needs no additional design approval.
has_blocked_findings: true
blocked: true
reviewed_by: codex/gpt-6.1-sol
recurrence: true
created: 2026-10-12T23:16:25-07:00
spec: 2026-10-18-fm-namespace/spec.md
implemented: true
description: A **feature** review of `2026-10-18-fm-namespace/spec.md`
feature: 2026-10-18-fm-namespace/review-4.md
previous: 2026-10-18-fm-namespace/review-3.md
log: claudine/features/2026-10-18-fm-namespace/log.md
implemented_by: claude/opus
next: 2026-10-18-fm-namespace/review-5.md
---

# Review of the `fm` namespace, iteration 4

The feature is **not production ready** under the full specification. The namespace migration remains implemented, and the targeted tests pass. One high finding remains: per-document environment capture and its acceptance tests were deferred again because the required approval decision is unresolved. No new unblocked finding was established.

## Previous review and repair disposition

Read the specification, all three earlier reviews, the [repair log](log.md), the [implementation inventory](implementation-log.md), and the [plan](plan.md). Review 3 had no unblocked findings. Its repair changed only the log and review metadata; it did not claim to implement the blocked finding.

| Review 3 item | Verified disposition |
| --- | --- |
| Unblocked findings | None existed, so none required implementation. |
| Blocked environment finding | Still deferred. The plan requires an explicit approval decision before this work; the inventory still records that decision as unresolved. No evidence establishes that the finding was unblocked before the last repair. |
| Scalar `ctx` observation | Unchanged; the restriction predates this feature. |
| Retired harness-validation documentation | Unchanged; the page is outside this feature's edits. |

The repair reports **zero disputed findings**. The recurring finding retains its original specification authority; no disputed claim is being re-raised. Review 3's `implemented: true` records the completed repair attempt, not completion of its deferred work.

## Unblocked Findings

None.

## Blocked Findings

### High — Per-document environment capture and its acceptance tests are not implemented

**Authority: spec.** Decided section 4 requires “Each new document captures its own: each sequence task, each group leg, and each `proxy` target,” before `initialize`, and requires every environment consumer of that document to read the same map. Acceptance criterion 5 requires supplied-map tests proving these boundaries and stability across events, iterations, and retry/resume. Those routes and tests remain unimplemented.

**Defect class:** environment consumers use invocation-owned or live process inputs instead of one map owned by each document lifecycle.

The specified test input is a first document under map A and a later document under map B, with expressions and file references agreeing within each document. Darkmatter's public replacement control passes, but Claudine's document entry retains the invocation environment. This is high because an explicit acceptance criterion remains unmet and untested through the required routes. A child shell's `export` cannot change the parent process and is not a reproduction of this contract.

The table sweeps the full connected class from the earlier reviews. The Darkmatter control was executed through public composition with supplied maps; the missing Claudine routes were inspected in source. They do not expose the per-document supplied-map input required to run the same control, so these rows are not presented as executed reproductions. No process environment was mutated for this review.

| Site | Shape checked | Observed result | Expected result |
| --- | --- | --- | --- |
| Darkmatter public [environment replacement control](../../../darkmatter/lib/tests/l1/request_context_builder.rs:255) | First document and include under A; second document under B | **Clean, executed:** expressions, context environment, and file references agree; A remains unchanged | Preserve this capability and route it through document entry |
| Claudine [invocation capture](../../lib/src/invocation_context.rs:1003) and [document epoch entry](../../lib/src/invocation_context.rs:910) | Second lifecycle under B | Environment remains invocation-owned; epoch entry accepts no replacement map | Own a map per lifecycle while retaining unrelated launch facts |
| Claudine [staged bootstrap and re-preparation](../../cli/src/commands/wrap/composition/staged_boot.rs:338) | Capture before initialize, then reuse | Extends launch context and applies overrides; no document-map capture | Capture before bootstrap/initialize and reuse throughout the lifecycle |
| Claudine [task preparation](../../cli/src/commands/wrap/sequence/task_run.rs:209) and serial member entry in that module | Successive tasks and serial legs under A/B | Captures launch context from the invocation; context overrides do not replace every consumer's map | Capture each task/leg map and distribute it consistently |
| Claudine [parallel group entry](../../cli/src/commands/wrap/sequence/task_run.rs:449) and member preparation | Independently supplied leg maps | Group context retains invocation inputs; no per-leg replacement route | Share repository evidence while keeping each leg's map independent |
| Claudine [proxy commit](../../lib/src/composition/coordinator/commit.rs:86), [target resolution](../../lib/src/composition/lifecycle/control.rs:291), and wrapper adoption | Source A; target lifecycle B | Resolution retains the source request context; adoption retains invocation inputs | Resolve the source target consistently, then capture B for its lifecycle |
| Claudine [provider/model selectors](../../lib/src/composition/select.rs:155) | Supplied map differs from process values | Production entry points still pass live environment readers | Select from the document map |
| Claudine [loop settings](../../lib/src/composition/looping/config.rs:18) | Supplied fail-fast, iteration limit, and pause margin | Reads process variables | Read the document map consistently across re-preparation |
| Claudine [lazy evidence refresh](../../lib/src/invocation_context.rs:1663) | Document values differ from later process values | Rebuilds evidence with `std::env::vars()` | Environment-derived evidence uses the document map; other permitted facts may refresh |
| File references, `SCHEMAS_DIR`, and shell approval reached through [preparation](../../lib/src/composition/prepare.rs:394) and the entry routes above | Expressions, paths, schemas, and command text under B | No complete document-map replacement reaches all consumers | One map for all consumers, with the selected approval contract enforced |
| Claudine [sequence snapshot test](../../lib/src/composition/sequence/tests.rs:588) and [proxy snapshot test](../../lib/src/composition/lifecycle/control/tests.rs:454) | Request snapshot versus changed ambient values | Tests retain the older request-wide contract; neither enters a new lifecycle under supplied B | Supplied-map tests through task, group, and proxy entry |
| Released `current_env` catalog, refresh, preflight, errors, approval bindings, and editor classification | Declared/absent ordinary property and fixed-root precedence | **Clean repaired siblings:** ordinary-property tests pass; fixed catalog is `fm, ctx, env, current`; late list is `["current"]` | Preserve the completed removal |

**Required verification:** Level 1 tests through the actual task, serial/parallel group-leg, and proxy entry routes, using supplied maps. Assert capture before initialize, map agreement among expressions/references/schema inputs/approved commands, reuse through lifecycle events, provider preparation, iterations, retry/resume, failure handlers, and finalize, and transclusion sharing its including document's map. A helper replacement test alone does not prove these entry routes.

**Blocker:** the [plan](plan.md), under “Approval ordering needs an explicit ruling,” explicitly states: “An unresolved ruling blocks completion of the affected environment work.” The [implementation inventory](implementation-log.md), under “Approval ruling,” still records it as unresolved. Command-template approval remains a finalized specification rather than an implemented prerequisite. This review does not choose a different approval contract on the author's behalf. The options are preserved in `human_review_items`; once decided, implement the entire table together or explicitly narrow the feature's acceptance requirements.

## Recurrence

This repeats **“Per-document environment capture and its acceptance tests are not implemented”** from reviews 1, 2, and 3. Every earlier review already named invocation ownership, bootstrap, task and serial/parallel leg entry, proxy adoption, selectors, loop settings, lazy evidence, references/schema/approval consumers, and route tests. The repairs deliberately deferred all of those siblings; none was newly overlooked by a partial repair. The completed `current_env` removal stays clean. This review carries the complete remaining class and sets `recurrence: true`.

The requirement has a defined boundary; recurrence here comes from an unresolved prerequisite, not new machinery producing new edge cases. Another unchanged repair cycle cannot complete it without the recorded decision or an explicit scope change.

## Observations

### The scalar ctx example conflicts with the existing composition contract

The specification's `ctx: notes` example conflicts with Darkmatter's existing [context merge](../../../darkmatter/lib/src/markdown/compose/context/merge.rs:61), which rejects scalar document `ctx` without override permission. The namespace tests use a mapping and verify that `fm.ctx` reads document data independently of merged context. This restriction predates the feature; a future example can use a mapping, or the author can schedule a merge-contract change separately.

### Non-interactive session documentation still names retired harness validation

Claudine's unchanged [non-interactive session topic](../../docs/topics/non-interactive-sessions.md:3) still lists retired harness validation as a capability. The edited dispatch adapter correctly names templates, conditions, and matchers. The older sentence belongs to separate documentation maintenance, not this repair cycle.

## Verification and scope

| Requirement | Appropriate level and evidence |
| --- | --- |
| `fm` snapshots, dotted/bracket reads, interpolation ordering, missing values, overrides, and shell-token selection | Level 1 public composition and lookup tests; freshly passed |
| Ordinary `doc`/`current_env`, reserved-root precedence | Level 1 public composition/catalog tests; freshly passed |
| Lifecycle global precedence and `set` visibility with stable earlier snapshots | Level 1 lifecycle execution; freshly passed |
| Hook payload in templates, conditions, matchers, and reports | Level 1 consumer tests; freshly passed |
| Editor completion/hover and passive behavior | Level 1 language-server requests with effects instrumentation; freshly passed; area recipe enables instrumentation |
| Frozen clarification prompt migration | Level 1 shipped CLI with frozen fixture; prior review records a passing run; last repair changed no fixture or behavior |
| Per-document environment capture and reuse | Level 1 production-route tests required; missing as listed in the finding |
| Retained lazy `current` behavior in a terminal | Level 2 required by acceptance criterion 8; cycle 1 log records seven executed tmux tests with no skips; unchanged by the last repair |

Fresh runs through package recipes and nextest: Darkmatter `just test fm_namespace` (31 passed), `just test fm_` (47 passed, including editor requests and passive instrumentation), and `just test an_environment_only_replacement` (1 passed); Claudine `just test hook_namespace` (8 passed) and `just test binding_contract` (12 passed). Filters overlap; these are run results rather than a unique test count. None exceeded five seconds. Root `just check-tier-coverage claudine` reports zero stranded tests; the new namespace test module is declared in the consolidated L1 target.

Full L1/lint gates, opt-in prompt checks, and the required Level 2 run are recorded in the earlier repair/review evidence. They were not repeated for a repair that changed only log/review metadata. Claudine emitted its existing macOS `__eh_frame` linker warning during targeted runs. No terminal/browser window was opened. Cross-OS evidence does not determine this readiness result.

Dotted `fm` lookup still traverses the effective map and clones only the selected value; no material performance issue was established. The input robustness matrix does not apply: this feature changes expression grammar/lookup and environment ownership, not a configuration or file-format reader. This review writes only its output and the requested prior-review/spec metadata; it leaves implementation and spec body unchanged.
