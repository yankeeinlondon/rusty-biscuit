---
$schema: feature-review.yaml
ready: false
findings:
  - "high: Cancellation schema validation stops at fixed nesting depths"
observations:
  - "Provider-wide dry-run test exceeds the Level-1 time budget"
  - "Configuration-shape matrices do not cover every supported format"
human_review: false
has_blocked_findings: false
reviewed_by: codex/gpt-6.1-sol
recurrence: true
created: "2026-10-15T15:53:19-07:00"
spec: "2026-10-24-sequence-containers/spec.md"
implemented: true
description: "A **feature** review of `2026-10-24-sequence-containers/spec.md`"
feature: "2026-10-24-sequence-containers/review-2.md"
previous: "2026-10-24-sequence-containers/review-1.md"
next: "2026-10-24-sequence-containers/review-3.md"
log: "claudine/features/2026-10-24-sequence-containers/log.md"
implemented_by: "claude/opus"
---

# Review 2

The feature is **not production ready** under the current specification. Two previous findings are fixed; the schema finding is partially fixed. One high finding carries the remaining instances of that class. No implementation code was changed during this review.

## Previous unblocked findings

The previous review has no separate “Unblocked Findings” section. All three findings are unblocked according to its metadata, so all three were checked first.

| Previous finding | Result | Verification |
| --- | --- | --- |
| Source rereads change the frozen container graph | Fixed | Preparation retains discovered sequence children and group legs; root retries retain their first graph. All 11 new CLI regressions passed, covering root, inline, referenced, catalog, external-task, and retry paths. Existing refusal tests still reject newly introduced commands. |
| Referenced sequence events lose their own frontmatter | Fixed | Referenced documents now supply their authored state to discovery, events, and children, with invocation setters and runtime writes above it. All 21 new CLI regressions passed, including Markdown/YAML/JSON/JSON5, caller collisions, group placement, external tasks, conditions, setters, and shell approval/execution. |
| Container schemas accept forbidden cancellation actions | Partially fixed | Direct task/group fields and their directly typed children now reject forbidden actions. Deeper child lists and deeply nested conditional bodies still accept them, as detailed below. |

The previous review's `implemented` flag is advanced as requested; the table above records the actual outcome rather than treating that flag as proof that every finding is closed.

## Findings

### High: Cancellation schema validation stops at fixed nesting depths

**Authority — spec:** requirement 5 requires `cancelled` to be “typed with the section 4 action set, on every primitive” and explicitly says, “do not defer validation of nested containers to a later feature.” Section 4 makes `retry`, `resume`, `proxy`, `prep`, `skip`, `error`, `break`, and `defer` typed parse errors in cancellation stacks. This is an explicit schema acceptance requirement without a container-depth exemption; runtime rejection alone does not meet it.

**Defect class:** schema expansion replaces restricted child/action types with unrestricted collections at a fixed depth, allowing forbidden cancellation actions below that boundary.

In Claudine, [the task schema's deeper sequence list](../../schemas/lifecycle.yaml:387) and [the deeper group leg list](../../schemas/lifecycle.yaml:405) are `unknown[]`. All cancellation fields beneath those lists bypass the restricted action type. Independently, [the third cancellation conditional's branches](../../schemas/lifecycle.yaml:327) use `loose-body`, which permits the same forbidden actions even on a root document. These are two cutoffs in the same validation class, rather than separate findings for each container combination or verb.

The new Claudine [grandchild schema test](../../lib/tests/l1/claudine_schemas.rs:1401) explicitly asserts that an invalid cancellation stack is valid. It documents the implementation limit but cannot establish compliance with the current requirement.

Reproduce by copying the shipped `action.yaml` and `lifecycle.yaml` beside this Markdown file:

```yaml
---
$schema:
  nested: task@./lifecycle.yaml
nested:
  name: outer
  sequence:
    - name: middle
      sequence:
        - name: inner
          sequence: []
          cancelled: [{retry: 2}]
---
```

Run the shipped command:

```sh
md schema validate --no-baseline-schema --no-trigger-schemas --format json example.md
```

It exits 0 with `valid: true`. Moving that cancellation field to `outer` or `middle` rejects it, proving that the imported schema is active. The real-tool sweep used private copies of the shipped schema files, replaced only the cancellation stack, and checked all eight well-formed forbidden actions plus `[stop]` and `[]` at every site below. Both positive controls passed everywhere.

In this table, an arrow means a container directly contains the next container; the last container owns the tested `cancelled` field. “All eight” refers to the eight verbs named above.

| Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- |
| Document cancellation type | Each forbidden action directly in the stack | All eight rejected | Rejected; clean |
| Inline sequence task | Same stack on the task | All eight rejected | Rejected; fixed |
| Group | Same stack on the group object | All eight rejected | Rejected; fixed |
| Sequence → sequence | Same stack on the inner sequence | All eight rejected | Rejected; fixed |
| Sequence → group | Same stack on the inner group | All eight rejected | Rejected; clean |
| Group → sequence | Same stack on the leg sequence | All eight rejected | Rejected; fixed |
| Group → group | Same stack on the leg group | All eight rejected | Rejected; clean |
| Sequence → sequence → sequence | Same stack on the innermost sequence | All eight accepted | Rejected |
| Sequence → sequence → group | Same stack on the innermost group | All eight accepted | Rejected |
| Sequence → group → sequence | Same stack on the innermost sequence | All eight accepted | Rejected |
| Sequence → group → group | Same stack on the innermost group | All eight accepted | Rejected |
| Group → sequence → sequence | Same stack on the innermost sequence | All eight accepted | Rejected |
| Group → sequence → group | Same stack on the innermost group | All eight accepted | Rejected |
| Group → group → sequence | Same stack on the innermost sequence | All eight accepted | Rejected |
| Group → group → group | Same stack on the innermost group | All eight accepted | Rejected |
| Shared cancellation conditional, one and two nested conditions | Each forbidden action in `then`, then separately in `else`; string expressions `true`/`false` select the branch | All eight rejected at both depths and branches | Rejected; clean |
| Shared cancellation conditional, three nested conditions | Same edit in `then`, then separately in `else` | All eight accepted in both branches | Rejected |

The shared conditional family is used by the document and all four typed container definitions (`task`, `group`, `task-2`, `group-2`); its unrestricted branch body is a common sibling site, not a separate implementation per primitive. The existing imported-task, imported-group, and imported-group-leg regressions also passed their direct `retry` checks. Referenced data files are checked by the runtime task parser rather than these Markdown schema exports; the runtime grandchild rejection test passed. This finding concerns the schema result, not execution of a forbidden recovery action.

The repair must either preserve the restricted cancellation action set throughout supported nesting or follow an explicit author revision of the schema contract. Increasing the unrolled depth by one would only move the boundary and repeat this cycle. The implementation log reports substantial schema expansion cost and a lack of recursive named-type support. A general recursive schema rewrite is large relative to the harm here: runtime already refuses these actions. Prefer documenting and explicitly agreeing to a bounded schema guarantee over adding that machinery solely for this editor check. The docs now describe a bound, but that does not amend the specification; readiness remains false until the stated contract is met or the author changes it. No new human approval is required to implement the existing contract.

## Recurrence

This is the same class as review 1's **“Container schemas accept forbidden cancellation actions.”** That repair fixed its listed shallow cases but stopped at `task-2.sequence` and `group-2.tasks`; it also left `cancelled-conditional-3.then` and `.else` unrestricted. A complete sweep should have followed the restricted type through those child lists and conditional branches. This review enumerates all eight sequence/group combinations reaching the container cutoff, both conditional branches, and the clean shallower controls. `recurrence` is therefore `true`.

## Verification and requirement coverage

Executed on macOS during this review:

- Claudine `just test sequence_frozen_graph sequence_referenced_state sequence_containers container cancelled grandchild claudine_schemas::`: **159 passed**.
- Claudine `just test sequence group container_yaml_matrix`: **1,359 passed**, with three tests exceeding five seconds; this is not a clean timing result.
- Claudine `just test sequence_sources_cli sequence_composition_dry_run_for_every_provider`: **24 passed**, with the provider-wide test still exceeding five seconds. The other two previously slow tests finished below five seconds on this narrower run.
- Claudine `just test-l2 level2_sequence_container_ctrl_c`: **one real tmux test passed**; the generator package had zero matching tests. No window gained focus.
- Darkmatter `just test schema_embedded_trigger_bundles triggers::assemble claudine_schema_discovery`: **44 passed**, including the new disk/embedded cache reuse and invalidation cases.
- Messenger `just test the_runner`: **five passed**.
- `just check-tier-coverage claudine`: **zero stranded tests**.
- The public schema CLI sweeps described above, including positive controls and all eight forbidden verbs at every listed boundary.
- `git diff --check`: passed.
- The saved review passed `md schema validate --no-baseline-schema --format json` against `feature-review.yaml`.

The following table maps the spec's acceptance criteria to the verification level present. The numbered criteria can be found in the specification's “Acceptance criteria” section; descriptions here identify the behavior without requiring that lookup. Passing focused tests overlap; their counts must not be added together as unique coverage.

| Acceptance criteria and behavior | Strongest relevant verification present | Assessment |
| --- | --- | --- |
| 1: nesting, ordered results, reference cycles, concurrent legs | Level 1 library and CLI | Appropriate; focused sequence/group suites passed |
| 2: removed grammar and options | Level 1 parser and CLI | Appropriate |
| 3–4: container event order, failure propagation, retry, no-session errors | Level 1 deterministic runners and CLI | Appropriate; referenced-state regression tests passed |
| 5–6: fail-fast, sibling cancellation, timeouts | Level 1 controlled runners and subprocess termination | Appropriate |
| 7: terminal Ctrl-C, descendant cleanup, exit 130 | Level 2 headless tmux foreground interruption | Appropriate; passed during this review |
| 8–11: generated values, reserved names, lexical scope | Level 1 expression, parser, library, and CLI | Appropriate |
| 12–14: body ownership, native result publication, lift-ready parser | Level 1 execution tests and source guards | Appropriate |
| 15: ledger removal and Messenger command generation | Level 1 CLI/completion and Messenger tests | Appropriate; focused runner tests passed |
| 16–19: proxy cleanup, shipped examples, edge cases, atomic publication | Level 1 execution and corpus tests | Appropriate; opt-in live prompt checks remain separate |
| 20–21: retry retention, source rereads, isolation, approval positions | Level 1 deterministic runners and CLI | Appropriate; frozen-graph repair tests passed |
| 22: Messenger locking, interrupted/stale/legacy records | Level 1 public API and CLI | Appropriate |
| 23: container retry parameters | Level 1 library and CLI | Appropriate |
| Requirement 5: cancellation schema restrictions throughout nesting | Level 1 schema API tests and public schema CLI | Correct test level; failing contract detailed above |

The new CLI files are declared in the consolidated L1 target, and the schema regressions are compiled by the library's declared L1 target. The tier audit confirms a live selection route. These are execution/state/schema requirements; they do not depend on a terminal's keyboard encoder. The Ctrl-C criterion explicitly calls for Level 2 and has that coverage.

Input robustness inspection included the sequence Markdown/YAML matrix, group/group-catalog YAML matrices, runtime cancellation-shape checks, and Messenger's run-record matrix. They distinguish missing, null, wrong-type, empty, duplicate, and invalid-content cases where applicable. This review found no new silent-default defect in those paths. The format-coverage limitation is recorded below rather than represented as an exhaustive format-by-field proof.

Full package suites and lint were not rerun in this review; the implementation log records their earlier runs. Cross-OS proof remains a CI responsibility and is not a readiness blocker here.

## Observations

### Provider-wide dry-run test exceeds the Level-1 time budget

The existing Claudine CLI [provider-wide sequence dry-run test](../../cli/tests/l1/wrap_sequence_composition.rs:20) took 6.826 seconds in the broader run and 5.833 seconds in the narrower run. It loops over seven process invocations in one test. That exceeds the repository's five-second budget, but the test itself predates this feature and this review did not establish a feature-induced regression. It is information for a separate test-maintenance decision, not another repair requested by this cycle. Two sequence-source matrix tests also exceeded five seconds in the broader run and finished below it on the narrower run.

### Configuration-shape matrices do not cover every supported format

The Claudine [sequence field matrix](../../lib/src/composition/sequence/tests.rs:2352) covers Markdown/YAML, and the [group/catalog field matrix](../../lib/src/composition/sequence/preflight/tests.rs:2391) covers YAML. JSON/JSON5 have successful source and referenced-state tests, but these matrices do not repeat every malformed new container field through those formats. The common post-load grammar provides useful shared coverage, and no realistic wrong result was demonstrated. This is a limit on the robustness evidence, not an additional production defect or a request for a synthetic payload matrix in this repair cycle.
