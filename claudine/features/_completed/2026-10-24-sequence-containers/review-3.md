---
$schema: feature-review.yaml
ready: false
findings:
  - "high: Messenger timeouts release run protection while sequence children keep running"
observations:
  - "Existing Level-1 tests exceed the time budget"
  - "Configuration-shape matrices do not cover every supported format"
  - "Cancellation conditionals lose editor branch completion"
human_review: false
has_blocked_findings: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: "2026-10-16T09:53:04-07:00"
spec: "2026-10-24-sequence-containers/spec.md"
implemented: true
description: "A **feature** review of `2026-10-24-sequence-containers/spec.md`"
feature: "2026-10-24-sequence-containers/review-3.md"
previous: "2026-10-24-sequence-containers/review-2.md"
next: "2026-10-24-sequence-containers/review-4.md"
log: "claudine/features/2026-10-24-sequence-containers/log.md"
implemented_by: "claude/opus"
---

# Review 3

The feature is **not production ready**. The previous review's finding is resolved under the amended schema contract. One new high finding affects the Messenger run protection required when removing the budget ledger. No production code was changed during this review.

## Previous unblocked findings

Review 2 has no separate “Unblocked Findings” section; its single finding is unblocked in its metadata. It was checked first. The earlier review 1 findings were also checked for regression.

| Previous finding | Result | Verification |
| --- | --- | --- |
| Review 2: Cancellation schema validation stops at fixed nesting depths | Resolved under the amended requirement 5 | The schema rejects forbidden cancellation actions through all five supported conditional levels. It types two container levels; deeper containers are checked by the parser. All eight sequence/group combinations at the third container level, two fourth-level chains, and the two CLI checks that reject malformed inline containers before root initialization passed. |
| Review 1: Source rereads change the frozen container graph | Remains fixed | All 11 frozen-graph CLI regressions passed, including root and nested retries, inline and referenced containers, catalogs, and external tasks. |
| Review 1: Referenced sequence events lose their own frontmatter | Remains fixed | All 21 referenced-state CLI regressions passed, covering Markdown/YAML/JSON/JSON5, event expressions, shell discovery/execution, caller collisions, setters, and runtime writes. |
| Review 1: Container schemas accept forbidden cancellation actions | Resolved under the amended requirement 5 | Direct document/container stacks, nested typed containers, dictionary stacks, conditional branches, and the sixth-level rejection tests passed. |

The schema amendment explicitly permits parser-only checking beyond two container levels and shape-only schema checks for other container events. Those limits are no longer findings against this specification. The docs describe them. Referenced files are loaded and checked during static preflight; inline malformed container events are rejected during plan normalization.

## Findings

### High: Messenger timeouts release run protection while sequence children keep running

**Authority — spec:** section 10 requires that the Messenger-owned runner hold an OS-backed fleet lock, “one research run at a time,” “for the whole execution,” and that “Cleanup and promotion are refused while a run is live.” Acceptance criterion 22 requires tests of that protection. A sequence running a shell command or an agent when its elapsed-time limit expires is ordinary research-run input, not malformed configuration.

**Defect class:** timeout settlement treats termination of the immediate sequence process as termination of all its work, releasing the ownership lock while descendant processes can still execute and write files.

In Messenger, [the research runner's timeout branch](../../../messenger/lib/src/research/refresh/runner.rs:164) calls `Child::kill()` and waits only for that child. It creates no process-tree ownership boundary and does not request Claudine's graceful cancellation. On macOS this kills Claudine without giving it a chance to terminate its running tasks. [The public runner then records the stopped outcome](../../../messenger/lib/src/research/refresh/runner.rs:69) and returns, dropping the fleet lock. A surviving task can continue writing into the research workspace while another run or cleanup is admitted.

Reproduced through Messenger's public `runner::execute_with` using the existing `Repo::published()` research fixture, a prepared run with `max_seconds: 2`, the actual built Claudine executable, a private home/PATH/audio spool, and a fake Goose executable. The sequence contained:

```yaml
sequence:
  - name: work
    shell: /usr/bin/python3 child.py
```

The child writes a readiness file, waits for a release file with a bounded deadline, and then writes a completion file. After the timeout returned `RunnerOutcome::Stopped`, the readiness file existed, the completion file did not, and the fleet lock was available. A second prepared run completed through the runner. Creating the release file afterward caused the first run's child to write its completion file. Thus actual sequence work survived both the timeout and admission of another run. The fixture's bounded child exited before the probe finished; no provider, GUI, or audio playback was used.

The sweep covered the shared runner's normal settlement paths and every public liveness consumer in the affected research workflow. Each timeout-consumer check used the same still-running child before releasing it:

| Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- |
| Messenger runner timeout | Actual Claudine sequence with a running shell child reaches its elapsed-time limit | Returns `stopped`; child continues and later writes | Settle the owned work before returning and releasing run protection |
| Messenger runner success | Command waits for its child, then exits 0 | Child finished before return; records `finished` | Same; clean control |
| Messenger runner failure | Same command exits 1 after its child finishes | Child finished before return; records `stopped` | Same; clean control |
| Messenger runner interruption | Same command exits 130 after its child finishes | Child finished before return; records `interrupted` | Same; clean control |
| `StateArea::fleet_lock_held` | Read while the timed-out sequence's child is waiting | `false` | Run protection must still cover live owned work |
| A second `runner::execute_with` invocation | Another prepared run starts before the first child is released | Accepted and completes | No overlap with the first run's surviving work |
| Cleanup preview and application | Call `cleanup::plan`, then `cleanup::apply`, while that child waits | Both accepted | Refuse while owned work remains live, or first guarantee that no child remains |
| Promotion | Attempt promotion of the stopped run while its child waits | Passes liveness check, then rejects the record with `WrongStatus` | Preserve live-work protection; the separate eligibility check is clean |
| Explicit stale recovery | Attempt `cleanup::recover` on the stopped run while its child waits | Passes liveness check, then rejects with `WrongStatus` | Preserve live-work protection; stopped-record rejection is clean |
| Claudine's ordinary Ctrl-C path | Existing headless tmux test interrupts both opted-in and plain descendants | Descendants terminate; cleanup runs innermost first; exit 130 | Same; clean control showing the cleanup path bypassed by the runner's hard kill |

The normal runner controls used a small command that spawns and waits for a child; the timeout was first reproduced with that same command, then confirmed with actual Claudine. `runner::execute` and the Messenger `research run` CLI delegate to `execute_with`; neither supplies another termination mechanism. The CLI's active-run note also uses `fleet_lock_held`; a timeout changes the record to failed, so the note does not warn about the surviving child. The exceptional `try_wait` error branch also returns without cleanup; it was inspected but no OS wait failure was manufactured. It should share the same ownership/settlement rule rather than acquire separate recovery machinery.

Repair the runner's process ownership and termination path, keeping the fleet lock until owned descendants settle. Prefer existing cross-platform process-control facilities and Claudine's cancellation cleanup rather than independent changes to each liveness reader. Add a deterministic runner regression with a real sequence child and a barrier; assert both that no post-timeout write occurs and that another run cannot overlap surviving work. [The existing timeout test](../../../messenger/lib/tests/research_refresh.rs:1203) checks only the saved status and reason, which is why it passes while descendants survive. This protection is an explicit contract and the failure occurs at a normal deadline, so documenting the limitation alone would not satisfy it.

## Recurrence

No finding repeats an earlier class. Review 1 covered frozen task membership, referenced-document state, and cancellation schema restrictions; review 2 continued the schema restriction class. This finding concerns ownership and settlement of subprocesses in Messenger's replacement run protection. `recurrence` is `false`.

## Verification and requirement coverage

Executed on macOS during this review:

- Claudine `just test claudine_schemas deep_cancelled container_grammar sequence_frozen_graph sequence_referenced_state`: **219 passed**, with 13 tests reported slow.
- Claudine `just test sequence group container_yaml_matrix`: **1,372 passed**, with 24 tests reported slow. Counts overlap with the first run.
- Claudine `just test-l2 level2_sequence_container_ctrl_c`: **one real headless tmux test passed**; the generator had zero matching tests. No window gained focus.
- Messenger `just test the_runner a_live_runner stale_active a_ledger_era cleanup_and_promotion the_run_record_matrix`: **11 passed**.
- Four temporary Messenger public-API probes: **four passed**, confirming timeout survival and normal success/failure/interruption controls. A further probe with actual Claudine and the liveness consumers above also passed, demonstrating the defect. Temporary probes were compiled by the existing declared research test target and selected by L1. The source file was restored byte-for-byte afterward.
- Public `md schema validate --no-baseline-schema --no-trigger-schemas --format json` against private copies of the shipped schemas: cancellation `retry` rejected and `stop` accepted in both branches at conditional levels 1, 3, and 5 (**12 checks**).
- Claudine `just test research_sequence_artifacts::`: **six passed**, all below five seconds in this focused run.
- Claudine `just lint`: passed.
- `just check-tier-coverage claudine`: **zero stranded tests**.
- Repository search for removed ledger identifiers outside feature/fix snapshots and timelines: no hits. Removed-grammar documentation hits describe rejection or migration; the performance-testing `--fail-fast` wording is unrelated.
- `git diff --check`: passed.
- Saved review validation against `feature-review.yaml`: passed with no problems.

The acceptance criteria below are in the specification's “Acceptance criteria” section. Grouping describes the behavior as well as its number; it does not imply every combination was rerun separately.

| Criteria and behavior | Strongest relevant verification | Assessment |
| --- | --- | --- |
| 1–2: nesting, reference cycles, ordered results, removed grammar | Level 1 parser and CLI | Appropriate; focused suites passed |
| 3–4: container events, preparation failure, failure propagation, retry, no-session errors | Level 1 deterministic execution and CLI | Appropriate; focused suites passed |
| 5–6: fail-fast, opted-in cancellation, timeout as failure | Level 1 controlled runners and subprocess tests | Appropriate for Claudine's behavior |
| 7: terminal Ctrl-C and descendant cleanup | Level 2 headless tmux | Appropriate; executed and passed |
| 8–11: group/sequence values, reserved names, rename errors, lexical scope | Level 1 expressions, parser, execution, and CLI | Appropriate; focused suites passed |
| 12–14: body lifecycle ownership, result publication, lift-ready parser | Level 1 execution and source guards | Appropriate coverage present |
| 15: ledger removal and Messenger command generation | Level 1 CLI/completion/research tests plus source search | Appropriate; replacement run protection has the finding above |
| 16–19: proxy cleanup, shipped examples, empty/skipped results, atomic state | Level 1 execution, CLI, and fixture checks | Appropriate; focused suites passed |
| 20–21: retry retention, frozen graphs, source rereads, isolation, approval position | Level 1 deterministic execution and CLI | Appropriate; repair regressions passed |
| 22: Messenger live/stale/interrupted/legacy run protection | Level 1 public API and CLI | Correct level, but the timeout test misses surviving descendants; reproduced above |
| 23: container retry parameters | Level 1 execution and CLI | Appropriate; focused suites passed |
| Requirement 5: bounded cancellation schema restrictions | Level 1 schema API and public schema CLI | Meets the amended boundary |

No requirement here needs a terminal keyboard encoder beyond the explicitly requested Level-2 interrupt test. The new CLI regression files are declared by the consolidated L1 target; the tier audit found no stranded tests. Cross-OS execution evidence is left to CI and does not affect readiness. Full package suites and Messenger lint were not rerun; their earlier results are recorded in the implementation log.

Input robustness inspection covered the Markdown/YAML sequence matrix, YAML group/catalog matrix, cancellation action shapes, and JSON run-record matrix. They exercise absent, null, wrong-type, empty, duplicate, and invalid-content cases where applicable, through plan/preflight/public load results. No new silent-default defect was found. The format-coverage limitation from review 2 remains below; these checks are not an exhaustive format-by-field proof.

## Observations

### Existing Level-1 tests exceed the time budget

The focused runs reported existing schema, sequence-source, and CLI tests above five seconds. For example, the provider-wide dry-run test took 9.018 seconds and the sequence-source format test took 8.424 seconds in the broader run. Nextest reported passing assertions, but those cases fail the repository's separate timing requirement. The earlier review and implementation log already identify this debt; this review did not establish a new feature-induced slowdown. It warrants separate maintenance, not another blocking finding in this cycle. The shipped-research tests added by this feature also ran slowly in the broad run, so all six were rerun together: each passed below five seconds, with the slowest dropping from 12.926 seconds to 2.546 seconds. That narrower result does not establish a new persistent timing defect in those tests.

### Configuration-shape matrices do not cover every supported format

The sequence matrix covers Markdown/YAML and the group/catalog matrix covers YAML; successful JSON/JSON5 reference tests do not repeat every malformed container field. This is the same evidence limitation recorded by review 2. Shared post-load parsing helps, and no realistic incorrect result was demonstrated. No additional synthetic matrix is requested by this review.

### Cancellation conditionals lose editor branch completion

The schema repair represents `then` and `else` using one pattern key to avoid excessive schema expansion. As documented in [Claudine's schema guide](../../docs/topics/schemas.md:285), the editor no longer completes those branch names inside `cancelled` conditionals. The implementation log also records loss of nested expression support below those keys. This is an editor ergonomics tradeoff, not a failure of the amended action-validation contract. Keep the limitation visible when improving recursive schema support rather than expanding this repair into a separate editor redesign.
