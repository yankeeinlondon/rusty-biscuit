---
$schema: feature-review.yaml
ready: false
findings:
  - "R1-F1 (high): referenced groups and internal task adapters retain the caller's authoring path"
  - "R1-F2 (high): Claudine runs file actions after launch repository discovery fails"
  - "R1-F3 (high): root sequence events and loop gates omit refusal diagnostics"
  - "R1-F4 (high): L1 tests exceed the timing limit under concurrent execution"
  - "R1-F5 (medium): containment diagnostics truncate the target or effective root"
human_review: false
human_in_the_loop: null
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: 2026-11-15T16:15:48-08:00
spec: 2026-11-14-lifecycle-mutation-root/spec.md
implemented: true
description: "A **fix** review of `2026-11-14-lifecycle-mutation-root/spec.md`"
fix: 2026-11-14-lifecycle-mutation-root/review-1.md
log: /Volumes/coding/wt/rusty-biscuit/fix-sequences/darkmatter/fixes/2026-11-14-lifecycle-mutation-root/log.md
implemented_by: claude/opus
next: 2026-11-14-lifecycle-mutation-root/review-2.md
---

# Review 1

The fix is not production ready. The shared target resolver and most launch-boundary wiring work, but some execution paths retain the wrong authoring document, Claudine ignores failed launch discovery, and several failure-reporting paths lose a refusal. The L1 timing gate also remains red. No finding requires an author decision before repair.

## Implementation decisions (informational)

The two requested independent readers examined the spec and implementation log in parallel, including `decisions_for_review`. The spec supplies no exception allowing required behavior to be deferred; its Open Questions section says the author rulings settle the behavior. The following inventory includes choices flagged for review and work left unfinished. None warrants interrupting the author: ordinary implementation choices are reasonable, and the defects have repairs within the existing contract.

- Moving Claudine's old root-relative test expectations into Phase 1 kept intermediate dependent suites usable. Keep that choice.
- Preserving `EditError::NoLocalTarget` for an unmatched recursive editor reference avoids an unnecessary public error variant. The internal effect error can remain more specific.
- Preserving the file-reference contract's own relative-tree escape error is appropriate. A relative reference rejected by that contract need not be rewritten as a different containment error.
- Shortened CLI assertions accommodated existing status-message truncation, while library tests checked complete paths. This explains the assertion choice but leaves the product defect in R1-F5; restore full CLI assertions when repaired.
- Rejecting broken `.git` files in Sniff is necessary to avoid selecting an enclosing repository as a guessed boundary. Its broader consumer impact is justified by the requirement and the recorded dependent checks.
- Deferring Claudine's launch-discovery error reporting to `2026-11-15-launch-discovery-failure-reporting` is not acceptable completion of this spec. R1-F2 recommends checking the retained observation before lifecycle execution, without making every unrelated Claudine command fail at capture. This needs no new ruling.
- Deriving referenced-container contexts through `InvocationContext::derive_source`, retaining launch authority, and using `Cow` to satisfy the context-construction guard are reasonable internal choices. The implementation is incomplete because the source path does not travel with every context; see R1-F1. The fallback to the caller's context when derivation is unavailable must not silently change the authoring base.
- Correcting the corrupt-repository fixtures was appropriate: an invalid `.git` directory alone does not necessarily constitute a failed discovery. Valid repository structure with corrupt metadata, and a separately broken gitfile, exercise the intended cases.
- Centralizing the detailed write-location contract in Darkmatter's lifecycle topic, with summaries and links elsewhere, reduces duplication. Updating the additional Claudine sequences page and Darkmatter caller-skill passage corrected relevant drift. Caller-skill publication is recorded as complete.
- Correcting two Windows test expectations for escaped backslashes and native separators was appropriate. Cross-OS evidence is left to CI and is not a readiness finding.
- The implementation explicitly omitted dedicated internal `kind: task` adapter verification without a reason. The public execution probe reproduced an incorrect destination; this omission is part of R1-F1. Omitting a separate mutation test for the step guard is reasonable because that guard cannot dispatch file actions.
- The foreign-sequence fixture added a `failure:` stack to print the refusal. That exposed, but did not repair, the root reporting defect in R1-F3. Users must not need to author such a stack to learn why a command failed.
- Optimizing the two existing Darkmatter test families without dropping assertions was reasonable. Choosing eight test workers under host contention was also reasonable. Calling remaining over-limit tests merely load-sensitive does not clear the review's timing gate; see R1-F4.
- No input robustness matrix is required. The changes select local paths and inspect repository entries; they do not add or change a configuration/file-format parser or field validator. Lifecycle Markdown grammar is also excluded by the review instructions.
- Editor L2 verification and documentation were deferred between implementation phases, then completed. They are not outstanding work.

## Findings

### R1-F1 (high): referenced groups and internal task adapters retain the caller's authoring path

**Authority — spec:** “a referenced sequence or group uses its own document,” and “Every execution path in the opening inventory is in scope, including internal task stacks still reachable through library adapters.” The spec also requires passing the same resolved target to the operation and live-frontmatter update.

**Class:** a document handoff replaces part of the resolution context while retaining the caller's source identity, so actions use the wrong authoring base or reject a valid target.

In Claudine's [group executor](../../../claudine/lib/src/composition/sequence/task/group.rs), `run_group_container` installs `own_context` in `private_stack` but inherits `source_path` from the calling sequence. Its event stacks and gate inherit that path again. Group members separately install their task source paths, which is why the existing member-only fixture passes. The internal task adapter's [side-effect dispatch](../../../claudine/lib/src/composition/sequence/task/mod.rs) similarly uses `self.stack` even when the task came from another file.

Reproduction uses a caller repository containing `flows/root.md` and `groups/group.yaml`. The root references the group; the group appends its event name to `events.log` in `initialize`, `start`, `success`, `finalize`, and `loop.gate`. Its one member appends to `member.log`. Run `claudine sequence --claude flows/root.md`. No provider is needed for these file-only steps. Canonical fixture paths reproduce the error independently of macOS path aliases.

| Instance | Site | Shape tested | Observed | Expected |
|---|---|---|---|---|
| R1-F1.1 | Referenced group event stacks | Missing relative `events.log`; four events | Exit 0; all four lines in `flows/events.log` | `groups/events.log` |
| R1-F1.2 | Referenced group gate | Same operand in one-pass loop | Gate line also in `flows/events.log` | `groups/events.log` |
| R1-F1.3 | Group selected from a catalog | `selected@../groups/catalog.yaml`, relative event target | `flows/catalog.log` | `groups/catalog.log` |
| R1-F1.4 | Foreign referenced group | Absolute caller-root target in `initialize` | Exit 1; no file; source-tree/context mismatch | Absolute in-root write succeeds |
| R1-F1.5 | Internal external-task adapter, primary | `tasks/task.yaml`, primary `ensure_file: note.md`; public `TaskExecution::run` | Task succeeds; file beside sequence | File beside task |
| R1-F1.10 | Internal external-task adapter, setup | Same fixture; setup creates `setup.md` | File beside sequence | File beside task |
| R1-F1.11 | Internal external-task adapter, teardown | Same fixture; teardown creates `teardown.md` | File beside sequence | File beside task |
| R1-F1.6 | Referenced sequence events and gate | Same four events, gate, and relative marker | Marker beside referenced sequence | Clean |
| R1-F1.7 | Referenced group member | Relative member marker | `groups/member.log` | Clean |
| R1-F1.8 | Inline group events and member | Relative markers in containing document | Both beside containing document | Clean |
| R1-F1.9 | Inline sequence origin | Inline array in containing document | Preflight retains that document as `origin_path`; no source handoff | Clean by origin invariant |

The internal-adapter probes reused the existing task fixture and public execution seam. The primary-only probe failed in 0.147 seconds with `expected beside task=false, wrote beside sequence=true`. The stage sweep failed in 0.182 seconds: setup, primary, and teardown all wrote beside the sequence, with no corresponding files beside the task. Both temporary review tests were restored afterward.

Carry the authoring source and its matching file-resolution context together at these handoffs, retaining the captured launch boundary. Follow that identity through event/gate expressions and active-document mirroring as well as file dispatch. Add group event/gate and adapter-stage tests with the caller and authoring file in different directories; retain the member, inline, and referenced-sequence controls. A second resolver or a mutable engine base is unnecessary.

### R1-F2 (high): Claudine runs file actions after launch repository discovery fails

**Authority — spec:** “A discovery failure must be reported before file actions run; only a successful observation of no working tree uses the launch-directory fallback.”

**Class:** a failed observation is projected as repository absence, and every downstream boundary consumer accepts the fallback without checking the retained failure.

Claudine's [invocation capture](../../../claudine/lib/src/invocation_context.rs) retains `RepositoryEntry::failure`, but constructs launch scope from its absent repository identity. The changed hosts consume `local_root()` without checking `InvocationContext::launch_repository().failure()`. The existing test only proves that a target above the launch directory is refused; it does not prove that discovery failure prevents writes below that directory.

Reproduction: create a healthy caller repository with launch directory `repo/package`, and a prompt in a separate healthy library repository. Give its actions absolute targets under `repo/package`. Verify success, remove the markers, then replace the caller's `.git/config` with `[core` followed by invalid configuration text. Repeat the same command. Claudine still exits 0 and writes the markers, without reporting discovery failure.

| Instance | Boundary consumer | Shape tested | Observed | Expected |
|---|---|---|---|---|
| R1-F2.1 | Staged initialization, `compose/prep.rs` → `staged_boot.rs` | Healthy/corrupt launch config; `initialize` marker | Both write and exit 0 | Corrupt case fails before write |
| R1-F2.2 | Wrapper pipeline, `composition/pipeline.rs` | Same fixture; `start` marker | Both write | Corrupt case fails before write |
| R1-F2.3 | Inline composition | Same fixture; initialization/start; fake provider rewrites body | Corrupt case exits 0 and writes both | No lifecycle write on failed discovery |
| R1-F2.4 | Loop preparation, `compose/prep.rs` | One-pass document loop, initialization marker | Corrupt case writes | No write |
| R1-F2.5 | Harness loop, `harness_orch/loop_control.rs` | Same loop's start/gate markers | Corrupt case writes both and exits 0 | No write |
| R1-F2.6 | Root sequence engine, `sequence/mod.rs` | Sequence initialization marker | Corrupt case writes and exits 0 | No write |
| R1-F2.7 | Task engine, `sequence/task_run.rs` | Same sequence's absolute task target | Corrupt case writes | No write |
| R1-F2.8 | Step-guard engine, `sequence/task_run.rs` | Retained scope consumer inspected | Also accepts unchecked scope; guard itself cannot dispatch file actions | Validate authority before subsequent execution |
| R1-F2.9 | Darkmatter CLI host | Same corrupt launch config and external prompt | Exit 1; launch-resolution diagnostic; no marker | Clean |

Check the retained launch observation before entering lifecycle execution, using one shared check across the host paths. This can remain scoped to lifecycle runs; changing every Claudine command's capture behavior is unnecessary. Complete the requirement here and reconcile the duplicate follow-up record. No human ruling is needed.

### R1-F3 (high): root sequence events and loop gates omit refusal diagnostics

**Authority — spec:** “An outside-root error identifies the target and effective root so the author can correct the path.” The spec includes early initialization, loop events, and failure/finalization paths while preserving the existing failure classification and recovery semantics.

**Class:** an executor records a refused action, but the host's outcome/report projection does not publish the stored diagnostic.

Claudine's [root event driver](../../../claudine/cli/src/commands/wrap/sequence/container.rs:110) stores the primary error and returns `Finished`; the [sequence entry point](../../../claudine/cli/src/commands/wrap/sequence/mod.rs) folds that into an integer without reporting the error. A plain sequence whose own `initialize` contains an outside `ensure_file` exits 1 with completely empty stdout and stderr. Adding `failure: [{info: "{{ err.msg }}"}]` exposes the reason, which explains the implementation fixture's workaround.

The sibling [loop-gate runner](../../../claudine/lib/src/composition/looping/engine.rs:561) forwards evaluation errors and control actions, but does not report `action_error`. Unlike setup failures, loop dispatch failures intentionally leave the composition outcome unchanged; that existing contract must remain intact. The missing diagnostic is the defect, not exit 0.

For root events, run a file-only sequence with an absolute outside target in the named event. For gates, use `until: "loop.count >= 1"`, `max: 2`, and `gate: [{ensure_file: /absolute/outside/refused.md}]`. The same gate matrix was run through all four consumers, with a fake provider for document loops.

| Instance | Site | Shape tested | Observed | Expected |
|---|---|---|---|---|
| R1-F3.1 | Root `initialize` | Outside effect, no failure stack | Exit 1; zero output bytes | Refusal names target/root |
| R1-F3.2 | Root `start` | Same target | Exit 1; preflight notes only | Refusal names target/root |
| R1-F3.3 | Root `success` | Same target, successful file-only child | Exit 1; successful child summary, no refusal | Root event refusal reported |
| R1-F3.4 | Root `finalize` | Same target, successful child | Successful child summary, no refusal | Finalization refusal reported |
| R1-F3.5 | Root `failure` | Child refuses one target; failure stack refuses another | Child error reported; second refusal absent | Preserve/report root event diagnostic too |
| R1-F3.6 | Document loop gate | Outside `ensure_file`, then outside `edit` | Exit 0; neither refusal reported | Report refusal; preserve per-phase outcome policy |
| R1-F3.7 | Root sequence loop gate | Same two actions | Same | Same |
| R1-F3.8 | Referenced sequence gate | Same two actions | Same | Same |
| R1-F3.9 | Referenced group gate | Same two actions, canonical in-repo source | Same | Same |
| R1-F3.10 | All four gate consumers, positive control | In-root `ensure_file` | Exit 0; target created | Clean |
| R1-F3.11 | All four gate consumers, suppression control | Outside effect with `no_error: true` | Exit 0; no write or refusal diagnostic | Clean; suppression is intentional |
| R1-F3.12 | All four gate consumers, evaluation control | Division-by-zero operand | Exit 1; evaluation error reported | Clean |
| R1-F3.13 | Referenced sequence event | Outside target in nested `initialize` | Step reports target/root for short paths | Clean |
| R1-F3.14 | Foreign referenced group event | Same target in `initialize` | Source/context mismatch before intended refusal | R1-F1; repeat after repair |
| R1-F3.15 | Canceled file actions | Cancellation checked before dispatch | Existing cancellation contract prevents fresh mutation | Clean invariant; do not launch an editor |

All seven frontmatter verbs, both `ensure_file` overloads, directory creation, both append verbs, and `edit` feed the shared `action_error` channel; the executor's every-writer refusal test verifies that producer side. Follow that channel to both the root completion reporter and loop-gate consumer rather than repairing one verb.

Report the stored refusal once through the shared output/emitter path, retaining secondary root event failures without repeating an already-rendered child error. Preserve terminal-phase outcome policy, `no_error`, and cancellation. Users should not need to author a diagnostic-printing failure stack. This is a small reporting repair, not a change to catch semantics.

### R1-F4 (high): L1 tests exceed the timing limit under concurrent execution

**Authority — review verification rule:** “a test at 10 s or longer fails, whatever it takes alone,” and a test that is fast alone but over the allowed bar in its package run is a shared-resource contention defect. The implementation's recorded runs and this review's canonical run exceed that limit.

**Class:** tests pass their assertions but exceed the L1 execution budget when they compete for shared resources; isolated success is being used to dismiss a failing concurrent run.

`cd claudine && just test-cli lifecycle_mutation` ran 18 cases, with 14 slow and three above ten seconds. Host load averages immediately afterward were 48.27/60.45/69.01 on the recorded 16-core host. A serial measurement, `just test-cli lifecycle_mutation -j 1`, put every case below two seconds. The three over-ten-second cases were also selected one at a time and took 1.085, 2.594, and 2.471 seconds, respectively; load averages near these checks were 32.79/44.84/53.41. The serial/isolated evidence clears the loaded-run cases between five and ten seconds, but does not clear the three cases above ten seconds.

| Instance | Test / recorded run | Concurrent or recorded failing time | Isolated/serial evidence | Status |
|---|---|---|---|---|
| R1-F4.1 | `lifecycle_mutation_root::a_foreign_sequence_cannot_write_beside_itself_from_initialize_or_a_task` | Review: 11.088 s | Serial: 1.300 s; alone: 1.085 s | Concurrent execution remains failing |
| R1-F4.2 | `lifecycle_mutation_lifetime::every_pass_and_gate_of_a_document_loop_writes_beside_the_document` | Review: 10.389 s | Serial: 1.951 s; alone: 2.594 s | Same |
| R1-F4.3 | `lifecycle_mutation_lifetime::referenced_containers_write_beside_themselves_and_inline_ones_beside_their_container` | Review: 10.106 s | Serial: 0.734 s; alone: 2.471 s | Same |
| R1-F4.4 | `ctx_per_run::a_sequence_task_observes_an_earlier_tasks_staging` | Implementation full run: 10.524 s | Log: 0.9–1.5 s alone | No compliant subsequent full run recorded |
| R1-F4.5 | `claudine_schemas_embedded::embedded_yaml_action_initialize_bundle::wrong_type_one_element` | Latest implementation full run: 10.984 s | Log: 1.6–1.7 s alone | Same |
| R1-F4.6 | Same family, `wrong_type_every_element` | Same full run: 10.425 s | Log: 1.6–1.7 s alone | Same |
| R1-F4.7 | `commands::git_graph::tests::gathered_graph_lays_out_sparse_lanes_at_56` | Recorded worktree run: 15.368 s | Log: graph cases 3.1–4.2 s alone | Existing defect, still above limit |
| R1-F4.8 | Same graph family, `gathered_graph_lays_out_sparse_lanes_at_120` | Same run: 12.384 s | Same isolated range | Same |
| R1-F4.9 | Same graph family, `the_observed_graph_keeps_recent_commits_on_every_lane_at_200x60` | Same run: 12.323 s | Same isolated range | Same |
| R1-F4.10 | `providers::frontmatter::sequence_tests::diagnostics_then_point_requests_reuse_one_snapshot` | Implementation log: 17.9 s solo spike | Later: 2.3 s | Record identifies an over-limit run; later solo speed does not explain/remove contention |
| R1-F4.11 | Other eleven slow cases in the review's mutation run | 6.508–9.668 s | Every serial case below 2 s | Cleared under loaded-host rule |
| R1-F4.12 | Darkmatter executor, compose-lifecycle, passive-compose cases | Review maxima below 1.3 s | 52 + 62 + 3 cases | Clean |
| R1-F4.13 | Optimized differential and base-schema test families | Earlier timed-out runs | Log records subsequent complete Darkmatter run with neither family slow | Cleared by implementation's subsequent run |

The exact full-run evidence is in `target/slow-tests/runs.jsonl`: `1f6fb303-9de3-43aa-b548-71d928682895` (Claudine), `842131b6-9828-4f83-85fa-7970e5f44cbb` (Claudine), and `5dd7a088-0345-4564-9328-baf945a15c79` (worktree). The second Claudine record contains 63 slow tests, not the log's original 61. The log has been corrected, including its claim that no tests failed the gate.

Address the actual contention or repeated work, using the repository's timing guidance; do not rename cases into a slower tier, add retries, or raise timeouts. Re-measure individually and in the relevant canonical package run. Keep the previous isolated timings as diagnostic evidence rather than proof that the concurrent failure is cleared. No workspace-wide rerun is requested.

### R1-F5 (medium): containment diagnostics truncate the target or effective root

**Authority — spec:** “An outside-root error identifies the target and effective root so the author can correct the path.” The implementation knowingly accepts only a prefix of the root in new CLI assertions. Legal longer project paths lose the information the spec requires.

**Class:** a notification-sized summary replaces the complete actionable error when the failure is projected to terminal and machine output.

Darkmatter's [failure projection](../../lib/src/markdown/compose/report_diagnostics.rs:277) publishes `failure.msg`; the [message constructor](../../lib/src/lifecycle/envelope.rs:45) limits it to 240 characters. Both CLI hosts use that shortened text. With a fixture's ordinary system-temp prefix plus a 50-character project component, an effect's root is cut off; editor failures add enough context to cut off the filename as well. This is an unusual path-length case, not an outside-write bypass.

| Instance | Site | Shape tested | Observed | Expected |
|---|---|---|---|---|
| R1-F5.1 | `md compose`, effect, Markdown/JSON | Short outside `ensure_file` | Complete target/root | Clean control |
| R1-F5.2 | `md compose`, effect, JSON | Longer outside `ensure_file` | Root truncated; no other root field | Complete target/root available |
| R1-F5.3 | `md compose`, frontmatter writer, JSON | Same path with `set_frontmatter` | Same truncation | Same |
| R1-F5.4 | `md compose`, editor, Markdown/JSON | Short and longer outside `edit` | Root truncated even in short fixture; long target also truncated | Same |
| R1-F5.5 | Claudine composition, effect/frontmatter | Same longer targets, both verbs | Root truncated | Same |
| R1-F5.6 | Claudine composition, editor | Same longer `edit` target | Target and root truncated | Same |
| R1-F5.7 | Claudine nested sequence report | Short outside effect | Complete target/root after joining wrapped lines | Clean control |
| R1-F5.8 | Other effect writers | Seven frontmatter writers and five filesystem overloads | Shared `resolve_effect_path` → action failure → same summary constructor; every-writer refusal tests confirm channel | Apply repair to shared projection |

Keep the existing compact message for speech/notifications. Preserve the full containment detail for the command's actionable error output, through the shared reporter, and strengthen the CLI assertions. There is no need to remove the summary limit globally or invent a separate error parser.

## Verification and scope

- Read the spec, implementation log/plan, implementation commits and uncommitted changes, host constructors, resolver, engine read/write guards, identity comparison, affected docs/skills, and declared test targets.
- Ran `darkmatter/just test lifecycle::executor::tests`: 52 passed, none slow; `just test compose_lifecycle`: 62 passed, none slow; `just test lifecycle_passive_compose`: 3 passed, none slow.
- Ran `claudine/just test-cli lifecycle_mutation`: all 18 assertions passed, but the timing gate failed. The serial rerun passed all 18 in 0.189–1.951 seconds each. Both results are retained above.
- Ran isolated shipped-CLI reproductions with healthy/corrupt repositories, an external prompt repository, fake provider, file-only sequences/groups, and absolute outside targets. No real editor, provider, terminal window, or browser window was launched.
- Two temporary internal-adapter probes invoked `TaskExecution::run` through the existing fixture and reproduced R1-F1, including setup, primary, and teardown. They were removed after measurement; no implementation test source is part of this review change.
- `just check-tier-coverage darkmatter` and `just check-tier-coverage claudine` both reported zero stranded tests. The affected L1 files are declared by their consolidated `main.rs` modules, and the canonical commands selected them.
- All requirements changed by this fix concern captured filesystem context, deterministic file effects, and command results. L1 is the appropriate verification level. The implementation also records two passing focus-preserving L2 editor tests; this fix adds no new keyboard-encoder or real-terminal rendering promise requiring L3.
- Cross-OS evidence is not a readiness gate for this review. No commit, formatter, lifecycle completion move, or production-code repair was performed.

## Implementation record corrections

The implementation log was corrected in place with a dated note: referenced-group context derivation did not finish the source handoff; the latest Claudine full run recorded 63 slow tests; and over-limit runs cannot be described as passing the L1 timing gate. These record corrections are informational. The underlying code/test defects are inventoried in the findings above.
