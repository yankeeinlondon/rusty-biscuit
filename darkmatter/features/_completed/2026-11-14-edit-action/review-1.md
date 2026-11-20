---
$schema: feature-review.yaml
ready: false
findings:
  - "R1-F1 (high): Recorded planner tests exceed the L1 time budget"
  - "R1-F2 (low): Sequence edit test describes an obsolete template limitation"
human_review: false
human_in_the_loop: null
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: 2026-11-18T08:17:42-08:00
spec: 2026-11-14-edit-action/spec.md
implemented: true
description: "A **feature** review of `2026-11-14-edit-action/spec.md`"
feature: 2026-11-14-edit-action/review-1.md
log: /Volumes/coding/wt/rusty-biscuit/fix-sequences/darkmatter/features/2026-11-14-edit-action/log.md
implemented_by: claude/opus
next: 2026-11-14-edit-action/review-2.md
---

# Edit action review 1

The feature is not ready under this review's rules. The editor workflows checked during this review work; the outstanding items are the recorded slow tests and an obsolete explanation in a workflow test. Neither finding needs an author decision.

The comparison uses the original specification together with the author rulings recorded at the end of the implementation log. In particular, ordinary scalar coercion, null flags, the caller-repository write boundary, the accepted timeout gap, and refusal of Claudine's unsupported `--output` flag supersede the original requirements where they differ. Cross-platform execution evidence remains CI's responsibility.

## Findings

### R1-F1 (high): Recorded planner tests exceed the L1 time budget

**Authority: defect, and the review's explicit testing gate:** “10 s or longer fails, whatever it takes alone” and “any slow test ... in the implementer's recorded runs ... each is a finding.” The implementation log's Phase 1 gates record two ordinary Test Toolkit tests taking 15.0 and 15.5 seconds in a package run. Their isolated times, 2.9 and 0.17 seconds, do not clear that threshold. Leaving them for follow-up because they predate the feature does not meet this review's readiness rule.

**Defect class:** planner contract tests perform enough shared repository/process work to exceed the L1 time budget under package concurrency.

The two sites are in [Test Toolkit's planner contract tests](../../../../tools/test-toolkit/tests/ci_workflow_contracts.rs:5291). The first named below invokes the shipped planner three times in one test, including its full-scope plan; the deletion case invokes it once. They operate on the real checkout. This is ordinary package-test execution, not an adversarial document input. The implementation record's diagnosis of contention is plausible, but this review does not claim to have established its precise resource bottleneck.

The sweep includes both planner consumers implicated by the record and all four neighboring tests that the same record identified as slow. The clean rows are part of the sweep, not additional repair requests.

| Child ID | Site | Shape tested | Observed result | Expected result |
|---|---|---|---|---|
| R1-F1.1 | `ci_workflow_contracts::the_shipped_planner_emits_plans_the_guards_reader_accepts` | Shipped documentation-only, full-scope, and changed-file plans read through the public guard reader | Recorded package run 15.0 s; recorded alone 2.9 s. Review package runs 5.916 s and 6.338 s; review isolated run 3.187 s | No package execution reaches 10 s under load; isolated execution stays within 5 s |
| R1-F1.2 | `ci_workflow_contracts::the_shipped_planner_omits_deletions_and_the_reader_refuses_an_unexpected_absence` | Modified source plus deleted source, followed by the guard's public missing-file refusal | Recorded package run 15.5 s; recorded alone 0.17 s; review isolated run 0.188 s | Same time budget while retaining the deletion/missing-file assertions |
| R1-F1.3 | `archive_path_guard::every_allowlist_entry_still_names_a_live_site` | Package run and isolated source-guard execution | Recorded 7.5 s at load about 40 on 16 cores; isolated 3.6 s. Clean under the loaded-host rule | Under 10 s loaded and at most 5 s alone |
| R1-F1.4 | `archive_path_guard::no_archive_executed_target_bakes_in_a_producer_path` | Package run and isolated source-guard execution | Recorded 9.1 s loaded; isolated 4.8 s. Later recorded 5.4 s loaded and 4.86 s alone. Clean | Same loaded-host allowance |
| R1-F1.5 | `kache_init_contracts::ensure_is_idempotent_on_a_second_run` | Repeated initialization in its fixture | Recorded 5.2 s loaded; isolated 2.5 s. Review package runs 4.488 s and 4.929 s. Clean | Same time budget |
| R1-F1.6 | `kache_host_contracts::report_resolves_the_worktree_base_by_the_wt_contract` | Worktree-base fixture | Recorded 6.1 s loaded; isolated 3.1 s. Current implementation splits the fixture shapes into separate tests. Clean | Same time budget |

The review observed load averages of 47.23, 26.64, and 18.65 on the 16-core host while builds and package tests were active. The isolated 3.187-second measurement clears the current 5–10 second results under the loaded-host allowance; it does not excuse the recorded 15-second executions. Later faster runs establish that assertions can pass, but do not identify a repair for the recorded contention.

**Recommended repair:** keep the public planner-to-reader checks, but remove unnecessary repeated repository discovery from their fixtures. Split the multi-plan test into independently scheduled cases with a guard retaining all three cases; investigate the single-plan deletion test's shared work as well. Use the smallest representative workspace that still tests the real planner/reader boundary where possible. Measure both repaired sites alone and in the package run. Do not add a slow-tier marker, serialize the suite, drop the full-scope/deletion cases, or extend a timeout. This is a test-cost repair; no new CI cell is needed.

### R1-F2 (low): Sequence edit test describes an obsolete template limitation

**Authority: defect:** [the Claudine workflow test](../../../../claudine/cli/tests/l1/lifecycle_edit_action.rs:307) says “A sequence document's own values are step data, not templates, so the path is spelled where it is used.” That statement is false in the current implementation. The sequence runner prepares its root values before initialization.

A direct invocation of the built CLI in an isolated repository used the specification's `fqn: "{{ ctx.cwd }}/specs/spec.md"`, `when: file_exists(fqn)`, and `edit: { file: "{{ fqn }}", create_new: true }`, followed by an informational message and `skip`. The compiled fake editor opened the expected file; the message printed and the command exited 0. The workaround in the existing test still passes, but its explanation teaches a limitation that no longer exists and avoids the motivating workflow's computed property.

**Recommended repair:** remove the obsolete comment and make this existing sequence-root test use the computed `fqn` property from the motivating example. Keep its existing assertions for the new-file and existing-file branches, exact target, and no provider/task execution. No new test abstraction or additional process matrix is necessary.

## Implementation decisions (informational)

Two independent agents read the spec and implementation record in parallel before the code comparison. The following inventory includes the choices they found, including temporary choices that were later replaced. I recommend the retained implementation choices; none warrants interrupting the author or blocking a repair.

| Implementer choice | Review disposition |
|---|---|
| Sniff identifies one whole program path, without splitting its argument on spaces; the resolver splits configured command lines | Keep the API boundary. It permits identification of a program path whose directory contains spaces without promising shell parsing |
| Test Toolkit has its own fake-editor binary and Unix `libc` dependency | Keep; process-tree and signal behavior need a real child process |
| Fake-editor replacement content comes from a file and saves through a temporary file and rename | Keep; exercises normal editor replacement saves and arbitrary bytes |
| Fake-editor line input precedes its JSON standard-stream report, read by `read_stdio_report` | Keep; the report can include the input actually read |
| Held fake editors self-expire after 120 seconds, and spawned children signal readiness over a private pipe | Keep; bounds leaked fixtures and avoids publishing an unready child |
| Added a paused-editor fixture mode for changing targets before exit | Keep; tests post-exit type and containment checks without changing production timing |
| Windows normal-exit release removes Job kill-on-close so a waiting client's GUI window can survive | Keep; matches the specified ownership boundary. Current process-tree ownership lives in Sniff |
| Resolver tests use an injected private lookup core, with real process/environment wiring tests separately | Keep; deterministic precedence/probe tests avoid depending on installed editors |
| Exclusive creation uses a two-thread primitive test and preoccupied service targets; the CLI check/edit race uses an authored `ensure_file` between the predicate and edit | Keep; real file creation proves the race contract without a production test hook |
| Cancellation and deletion reuse nested `EditorError` variants; `EditError::is_cancelled` recognizes cancellation | Keep; avoids duplicate error variants |
| Added typed `EditError::NoLocalTarget` rather than inventing a file-reference cause | Keep; preserves the actual failure class |
| Long-form operand and flag diagnostics point at their individual keys | Keep; more precise authored locations |
| Temporary schema exemptions, generic Claudine error classification, and schema-gap test wrappers bridged implementation phases | Completed and removed/replaced; not deferred work |
| Schema generation refuses function/effect names that collide with lifecycle-only verbs | Keep; prevents accidentally exposing an effectful `edit()` expression |
| Failure messages put the reason first and append the target only when absent | Author accepted; current complete lifecycle diagnostics also preserve long messages |
| Initially retained host-specific write roots and tested sibling prompt libraries as refused | Superseded by the author's common caller-repository boundary and document-relative targets; current code implements that boundary |
| Cancellation before launch and cancellation while editing have different messages | Keep; explains whether an editor was started |
| Darkmatter's L2 recipe enables `test-fixtures` alongside `terminal-tests` | Keep; local and CI runs build the fake-editor target |
| No new lifecycle-wide timeout was introduced | Author explicitly accepted the gap. Interrupt and fail-fast sibling cancellation remain supported; no new human ruling is needed |
| Silent sequence-root initialization failures were initially deferred as a presentation decision | Already repaired in current sequence reporting; verified directly and corrected in the implementation record |
| Claudine's edit failure is environment-origin and correctable, with a boxed typed cause | Keep; fits existing error ownership and ordinary editor failures |
| Inline sequence start/finalize ordering substitutes for removed serial-group and setup/teardown syntax | Keep; tests the current grammar's serial execution surface |
| Claudine initially asserted empty stdout rather than a nonexistent structured envelope | Superseded by the author's refusal of the unsupported `--output` flag; current tests cover its argument positions |
| Originally added strict parser-owned target/flag checks and non-null schema markers | Superseded by the author's coercion/null rules. Current native numeric/boolean targets convert to text, Boolean-like flag strings convert, and null flags take their absent defaults |
| Retained parser-owned passive checks for remaining invalid target and literal-flag shapes | Keep; schema validation, DMLS, and runtime use the same shape rules |
| Added `edit` to cancellation bundles and both loop schema arms, beyond the plan's enumerated sites | Keep; registration matches parser availability, while cancellation eligibility is enforced when reached |
| Long-form edit has an operand marker for passive template scanning | Keep; malformed long-form expressions remain visible to tooling |
| DMLS may retain an enclosing union summary alongside a precise operand diagnostic | Keep; the summary can describe another problem in the same action |
| Initially edited installed caller skills | Superseded by the author's versioned canonical sources and `just publish-skills` workflow |
| Generated shipped schemas once, using standing freshness checks rather than a second identical generation | Author accepted; no repeat approval needed |
| Did not rewrite a landed commit merely to split an editor I/O hint from comment cleanup | Author explicitly dropped that history rewrite |
| Left final cross-OS verification to CI and limited early-phase test scope | Repository policy already decides this; not a human-review item |
| Deferred the two recorded tests over 10 seconds because they were unchanged code | Not accepted by this review's explicit timing rules; repair is R1-F1, without an author decision |

The rulings initially marked mutation-root unification, coercion/null handling, and shared race-free process trees as pending work owned by later specs. Those implementations are present now. This review does not reopen their approved scope assignment. The only continuing functional deferral is lifecycle timeout coverage, which the author explicitly accepted and the canonical Claudine documentation explains.

## Requirement and verification levels

| User-facing requirement | Strongest relevant evidence | Assessment |
|---|---|---|
| Motivating create/edit/skip workflow, existing-file branch, external prompt location, and occupied-name race | L1 process tests in both CLIs; direct sequence-root computed-property run | Correct behavior observed; update the stale sequence test explanation under R1-F2 |
| Scalar/list/long forms, flags, arity, unknown fields, located errors, and conditional branches | L1 parser, schema parity, DMLS protocol, and CLI tests | Appropriate; later author coercion/null rulings applied |
| Local-reference resolution, containment, Unicode/metacharacter paths, exclusive creation, preserved bytes, and atomic replacement | L1 library/service/process tests | Appropriate; no real editor or graphical application needed |
| Shared editor precedence, matching surface, blocking flags, and rejected non-blocking editors | L1 resolver, Sniff catalog, and CLI dispatch tests | Appropriate |
| Eligibility, launch/exit/deletion/type failures, and `no_error` continuation | L1 service and CLI tests | Appropriate |
| Passive analysis, `md compose --shell`, dry-run, untaken branches, and no effectful expression registration | L1 recorder/no-side-effect tests and catalog checks | Appropriate |
| Initialize/later-event/container ordering and parallel-member eligibility | L1 Claudine and Darkmatter workflow tests | Appropriate |
| Terminal input, blocking handoff, screen routing, and return to subsequent actions | L2 detached tmux pane tests in both CLIs | Appropriate; input goes only to the detached pane |
| Structured `md compose` output, GUI stream routing, and refusals before creation | L1 CLI envelope and standard-stream recorder tests | Appropriate; Claudine envelope requirement superseded by author ruling |
| Cancellation, saved files, tree termination/escalation, and no continuation | L1 process-tree/host tests plus L2 interrupt-and-return tests | Appropriate; sequence-timeout trigger explicitly waived |
| Terminal/foreground restoration on success, nonzero exit, and cancellation | L1 controlling-PTY launcher test covers all three; L2 covers success and interruption through real tmux | Appropriate; no terminal input-encoding or modifier-badge behavior was introduced that needs L3 |
| Shipped artifacts, host schema availability, completion, hover, and passive diagnostics | Existing freshness/parity checks and L1 DMLS tests | Appropriate |

The input-shape audit covers the new action's target and shared literal-flag validator: absent/null/empty operands, scalar coercion, wrong whole values and list elements, unknown fields, duplicate inner/sibling flags, and templates in flags. Accepted controls and invalid cases are exercised through public lifecycle parsing and schema validation, with CLI wiring controls and located refusals. This feature does not add a file-format loader; existing YAML document loading is unchanged, so no new duplicate-key/trailing-document loader matrix is required. Markdown/interpolation grammar and editor output are outside that matrix.

New tests are declared by the consolidated test binaries, fake-editor targets are enabled by the CLI's CI feature metadata, and `just check-tier-coverage darkmatter` found no stranded tests.

## Verification performed

- Darkmatter `just test editor`: 334 passed; no test over five seconds.
- Darkmatter `just test edit_action`: 46 passed; no test over five seconds.
- Darkmatter `just test lifecycle_edit_schema_parity`: five passed; slowest 0.334 seconds.
- Claudine `just test lifecycle_edit_action`: 77 passed; slowest 2.950 seconds.
- Claudine `just test lifecycle_refusal_reports`: 34 passed; no test over five seconds, including sequence-root and referenced-container edit refusals.
- Root `just test test-toolkit`: 566 assertion passes, four skips, one slow test at 5.916 seconds. A second package run recorded the same multi-plan test at 6.338 seconds. These are timing evidence, not an unconditional green test gate.
- Direct current-CLI reproductions confirmed a reported sequence-root edit refusal and the original computed-property create/edit/skip workflow, using an isolated repository and compiled fake editor.
- Darkmatter `just test-l2 level2_edit_action`: completed successfully; both detached-tmux editor tests executed and passed, at 0.920 and 0.950 seconds. The library and language-server package legs selected no matching tests.
- Isolated nextest measurement of the multi-plan contract: 3.187 seconds, clearing the current loaded-host 5–10 second readings but not the earlier executions over 10 seconds.
- Isolated nextest measurement of the deletion contract: 0.188 seconds.
- The review file passes the current CLI's `md schema validate` against `feature-review.yaml`.

Some overlapping local recipes wrote to the same default nextest report path; one slow-test recording step reported a missing XML file and another recorded zero tests after a different run replaced it. Counts and timings above come from each invocation's own nextest output, not from the shared current report. This review does not claim a complete fresh package-wide slow-test receipt from those overlapping runs.

## Implementation record corrections

The implementation log was corrected in place, with dated notes, for two claims contradicted by direct execution: sequence-root initialization failures are now reported, and computed root properties now resolve before initialization. The original spec was not rewritten. These corrections are informational, not findings or human-review items; the separate obsolete comment in a test is R1-F2.
