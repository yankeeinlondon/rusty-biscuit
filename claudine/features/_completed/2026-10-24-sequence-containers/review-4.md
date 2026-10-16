---
$schema: feature-review.yaml
ready: true
findings: []
observations:
  - "Existing Level-1 tests exceed the time budget"
  - "Configuration-shape matrices do not cover every supported format"
  - "Cancellation conditionals lose editor branch completion"
human_review: false
has_blocked_findings: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: "2026-10-16T10:34:35-07:00"
spec: "2026-10-24-sequence-containers/spec.md"
implemented: false
description: "A **feature** review of `2026-10-24-sequence-containers/spec.md`"
feature: "2026-10-24-sequence-containers/review-4.md"
previous: "2026-10-24-sequence-containers/review-3.md"
---

# Review 4

The feature is **production ready under the amended specification**. The previous review's blocking timeout defect is fixed, and this review found no new high, medium, or low findings. The three earlier observations remain nonblocking. This review changed only review documents and the requested specification metadata.

## Previous unblocked findings

Review 3 has no separate “Unblocked Findings” section. Its single finding is unblocked in its metadata and was checked first. Reviews 1 and 2 were also read to check for recurring defects.

| Previous finding | Result | Verification |
| --- | --- | --- |
| Review 3: Messenger timeouts release run protection while sequence children keep running | Fixed | Timeout and wait-error paths now share process-tree shutdown while the fleet lock remains held. Both new timeout regressions passed: a waiting grandchild is dead before return and cannot write after release; a sequence command's interrupt handler runs before forced termination. |
| Review 1: Source rereads change the frozen container graph | Remains fixed | All 11 frozen-graph CLI tests passed, covering inline, referenced, catalog, external-task, and retry paths. Preparation retains discovered children and order; root retries retain the first graph. |
| Review 1: Referenced sequence events lose their own frontmatter | Remains fixed | All 21 referenced-state CLI tests passed, covering Markdown/YAML/JSON/JSON5, events, children, shell approval/execution, caller collisions, setters, and mutations. |
| Reviews 1 and 2: Container cancellation schemas lose restrictions below fixed depths | Resolved under the amended requirement | Schema and deep-parser tests passed. The schema checks two container levels and all five supported conditional levels; deeper containers are checked by the parser before execution. This is the specification's explicit interim boundary. |

For the Messenger repair, the review checked the complete ownership path rather than treating a saved `stopped` status as proof of termination:

| Site | Check and result |
| --- | --- |
| Timeout shutdown | The public runner's barrier regression passed: the grandchild started, a competing run was refused, the deadline produced `stopped`, the grandchild was gone before return, and releasing its barrier produced no write. |
| Graceful cleanup | The Unix regression passed: the command's interrupt handler wrote its cleanup marker before shutdown finished. |
| OS wait error | Inspected: calls the same shutdown function as timeout. No artificial OS wait failure was introduced. |
| Normal completion, failure, and interruption | Existing runner controls passed, preserving `finished`, `stopped`, and `interrupted`. Normal exits do not kill detached lifecycle audio workers. |
| Lock ownership and competing runs | The lock's scope includes shutdown, descendant settlement, and outcome persistence. Both the existing competing-run test and the timeout regression passed. |
| Cleanup, promotion, and explicit stale recovery | Their existing liveness checks still use the same fleet lock. Live-run refusal and explicit stale-recovery tests passed. |
| Public command entry points | Inspected: Messenger's research CLI and default runner delegate to the repaired explicit-command runner; there is no separate timeout path. |

Messenger's [process tracking](../../../messenger/lib/src/research/refresh/owned_tree.rs) records descendant process IDs and start times and kills observed descendants deepest first after graceful shutdown. Its new tests use actual shell processes through the public API, rather than launching Claudine itself. Claudine's descendant cleanup is separately exercised by the real-terminal test below. These are complementary checks, not a claim of a new end-to-end Messenger-to-Claudine timeout test.

The documented sampling limitation for rapidly detached processes remains. Settlement also has a bounded wait and warns if observed processes survive it; it is not an unconditional OS termination guarantee. No ordinary sequence input was found that defeats the repaired path. Extending this into stronger containment solely for unusual detachment or OS kill failures would add machinery beyond the demonstrated defect; retain the documented limit.

## Findings

None.

## Recurrence

No finding repeats an earlier class. The frozen-graph, referenced-state, bounded schema, and timeout regressions passed. `recurrence` is `false`.

## Verification and requirement coverage

Executed on macOS during this review:

- Claudine `just test sequence group container_yaml_matrix claudine_schemas deep_cancelled`: **1,533 passed**, with three existing tests above five seconds.
- Messenger `just test a_timed_out_run the_runner a_live_runner stale_active cleanup_and_promotion the_run_record_matrix`: **11 passed**, all below five seconds; the new descendant and graceful-cleanup tests took 2.611 and 2.295 seconds.
- Claudine `just test-l2 level2_sequence_container_ctrl_c`: **one Level-2 test passed**, using headless tmux. No window gained focus. The generator had zero matching tests.
- Claudine and Messenger `just lint`: **passed**.
- `just check-tier-coverage claudine` and `just check-tier-coverage messenger`: **zero stranded tests**.
- Repository-wide search for removed ledger identifiers, excluding historical feature/fix snapshots and timelines: **no remaining hits**. Remaining `CLAUDINE_FAIL_FAST` reads belong to the separate loop behavior or tests/documentation asserting that sequences no longer inject it.
- Review validation against `feature-review.yaml` and `git diff --check`: **passed**.

The following numbers refer to the specification's “Acceptance criteria” section. Descriptions identify the behavior without requiring the reader to look up those numbers.

| Criteria and behavior | Strongest relevant verification | Assessment |
| --- | --- | --- |
| 1–2: nested containers, ordered results, reference cycles, removed grammar | Level 1 parser, library, and CLI | Appropriate; focused suites passed |
| 3–4: event order, preparation failure, failure propagation, retry, recovery without a session | Level 1 deterministic execution and CLI | Appropriate; focused suites passed |
| 5–6: atomic groups, opted-in sibling cancellation, timeout as failure | Level 1 controlled execution and subprocess tests | Appropriate; focused suites passed |
| 7: terminal Ctrl-C, descendant cleanup, innermost-first finalization, exit 130 | Level 2 headless tmux | Appropriate; executed and passed |
| 8–11: generated values, reserved names, legacy-name errors, lexical scope | Level 1 expressions, parser, execution, and CLI | Appropriate; focused suites passed |
| 12–14: body lifecycle ownership, native results, parser independence | Level 1 execution and source-guard tests | Appropriate coverage present |
| 15: ledger removal and Messenger command generation | Level 1 command/research tests and repository search | Appropriate coverage present; removal search clean |
| 16–19: proxy cleanup, shipped examples, empty/skipped results, atomic publication | Level 1 execution, CLI, and artifact checks | Appropriate coverage present |
| 20–21: retry retention, frozen membership, rereads, isolation, approval position | Level 1 deterministic execution and CLI | Appropriate; repair regressions passed |
| 22: Messenger live, interrupted, stale, and legacy run protection | Level 1 public API and CLI | Appropriate; timeout and liveness checks passed; legacy coverage inspected |
| 23: container retry parameters | Level 1 execution and CLI | Appropriate; focused suites passed |
| Requirement 5: bounded cancellation schema validation | Level 1 schema and parser tests | Meets the amended boundary |

No requirement depends on a terminal keyboard encoder beyond the explicitly requested Level-2 interrupt check. No additional Level-3 test is needed. Cross-OS evidence is left to CI and does not affect readiness. Full package suites were not rerun; this review reports the focused runs above and inspected the earlier full-suite results in the implementation log.

The new CLI regressions are declared by the consolidated Level-1 target. Messenger's timeout regressions are compiled by its research integration target, selected by the Level-1 recipe with the `research` feature enabled, and actually ran above.

Input robustness inspection covered sequence Markdown/YAML fields, YAML group/catalog fields, cancellation action shapes, and the JSON run-record matrix through public plan/preflight/load results. No new silent-default defect was found. The format limitation below remains; successful source-format tests do not establish a complete malformed-field matrix for every format. The timeout repair adds no new configuration field or file-format reader.

## Observations

### Existing Level-1 tests exceed the time budget

The existing Claudine source-format tests took 6.368 and 6.068 seconds, and the provider-wide dry-run test took 7.199 seconds. Their assertions passed, but their durations exceed the repository's five-second Level-1 budget. Earlier reviews already recorded these cases; this review found no new feature-induced slowdown. Keep this as separate test maintenance. The two new Messenger timeout regressions remained below the budget.

### Configuration-shape matrices do not cover every supported format

The sequence matrix covers Markdown/YAML, and the group/catalog matrix covers YAML. Successful JSON/JSON5 reference tests do not repeat every malformed container field. Shared parsing provides useful coverage, but does not prove every format-by-field combination. This carries the earlier evidence limitation forward; no realistic incorrect public result was demonstrated, and no additional synthetic matrix is requested in this cycle.

### Cancellation conditionals lose editor branch completion

The schema represents `then` and `else` with a shared pattern to limit expansion. As [Claudine's schema guide](../../docs/topics/schemas.md:285) explains, editor completion does not suggest those branch names inside cancellation conditionals. This remains an editor ergonomics tradeoff, not a violation of the amended cancellation-validation requirement.
