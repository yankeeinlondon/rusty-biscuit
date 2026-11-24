---
spec: "/home/build/wt/rusty-biscuit/fix-biscuit-terminal/claudine/fixes/2026-11-20-loop-reread-context-drift/spec.md"
plan: "claudine/fixes/2026-11-20-loop-reread-context-drift/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
source_files_during_phase_1:
    - claudine/cli/tests/l1/loop_startup_source.rs
    - claudine/cli/tests/l1/main.rs
    - claudine/cli/tests/l1/loop_cli.rs
    - claudine/cli/tests/bin/fake_goose/main.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/overlay_layering.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/target_launch.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/target_launch/tests.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
    - claudine/lib/src/composition/prepare.rs
    - claudine/lib/src/composition/prepare/entry.rs
    - claudine/lib/src/composition/prepare/service.rs
    - claudine/lib/src/composition/prepare/service/tests.rs
    - claudine/cli/src/commands/compose/mod.rs
    - claudine/cli/src/commands/compose/prep.rs
    - claudine/cli/src/commands/wrap/composition/mod.rs
    - claudine/cli/src/commands/wrap/composition/pipeline.rs
    - claudine/cli/src/commands/wrap/composition/runner.rs
    - claudine/cli/src/commands/wrap/composition/staged_boot.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/mod.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/overlay_layering.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/requeue.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/retry_resume.rs
    - claudine/cli/src/commands/wrap/harness_orch/mod.rs
    - claudine/cli/src/commands/wrap/harness_orch/prompt.rs
    - claudine/cli/src/commands/wrap/harness_orch/prompt/tests.rs
    - claudine/cli/src/commands/wrap/harness_orch/types.rs
    - claudine/cli/src/commands/wrap/wrapper_stages.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
packages:
    - claudine-cli
    - claudine
decisions_for_review:
    - |-
        Phase 1 ends with nine new regression tests failing on purpose. The plan treats them as a baseline to record before moving straight to Phase 2, but the phase's completion rule also asks for every test to pass. I kept them failing and visible rather than hiding them with #[ignore], a feature gate, or assertions inverted to match today's behavior. Each of those would leave a test that cannot catch the defect and would have to be undone in Phase 2. The trade-off is that the area's full test run is red between Phase 1 and Phase 2. If a reviewer wants every phase green, the nine tests would need to be marked or split out, and the marks removed when Phase 2 lands.
    - |-
        Four existing library tests (sequence group framing) fail when the effective locale is non-UTF-8, such as LANG=C with no UTF-8 LC_ALL or LC_CTYPE override. The header glyph then becomes ">" instead of the arrow these tests look for. I did not fix them: they are in code this fix never touches, they pass under a UTF-8 locale, and changing how the locale picks glyphs (or forcing one in the tests) is a separate decision. I ran the full gate under LANG=C.UTF-8 instead. A reviewer may prefer making those tests independent of the locale; that is a small, separate change in claudine/lib/src/composition/sequence/task/tests.rs.
    - |-
        The plan asked for a retry-and-resume recovery case modeled on existing tests. I added only the retry process test (document_loop_retry_does_not_replace_startup_source) and left resume to its existing coverage in ctx_per_run.rs and lifecycle_traces.rs, because both recoveries take the same fresh-read path the retry test pins. If a reviewer wants resume pinned against the retained source too, it is one more test in loop_startup_source.rs using a provider that reports a session (resume needs one).
    - |-
        After the provider harness rereads a directly run document following its initialize, the new lifecycle actions are installed with the context that reread prepared, exactly as it is (the same thing the old retry path did). I did not additionally widen that context for context groups named only in the effective frontmatter, nor lay the launch environment overrides on top, as the command pipeline does for a guard it owns. The reread already captures every group the document text names, and all tests agree. If a reviewer wants both paths to build the context identically, the harness install would call the same extension the pipeline uses; nothing else would change.
    - |-
        The plan says only a successful first-attempt stabilization hands its read to the loop. I hand it on from every kind of first-attempt boot, including an adopted proxy target's full boot, not only the direct document's stabilize-only boot. Only an owning loop reads the value, and an adopted target never runs inside a loop that would read it, so the effect is the same today. Limiting it to the stabilize-only boot is a one-line condition if a reviewer prefers the narrower rule.
    - |-
        The preparation an attempt reports to the loop's gate is recorded after the boot or recovery install and before the harness plan is parsed, rather than immediately before start. A plan-parse failure ends the attempt with an error either way, so the gate never reads it then. Moving the record to just before start would only matter if a later change let a plan failure continue the loop.
    - |-
        In the looping guide, the existing inline-compose example that ended a loop with a bare `until: "done == true"` no longer described real behavior: a later pass composes the document the loop kept at startup, so a bare `done` reads that kept copy and never sees the agent's `done: true` (the run reaches the cap instead). The plan and spec do not mention this example. I changed the example to read the file explicitly (`frontmatter('task.md', 'done')`), said why next to it, and pinned both cases with tests (the explicit read ends after one pass; the bare read reaches the cap). Eager documents without `initialize` already behaved this way before this fix, so this corrects the documentation and is not a new regression. The alternative is to make an inline loop's condition read the live deliverable's frontmatter. That would keep the old example working, but the gate would then see a different document than its own pass's lifecycle saw. Choosing it would mean changing the loop gate's inputs for inline passes, reverting the example and the caller-skill sentence, and inverting `a_bare_inline_sentinel_reads_the_kept_document`.
source_files_during_phase_3:
    - claudine/cli/tests/bin/fake_goose/main.rs
    - claudine/cli/tests/l1/loop_startup_source.rs
    - claudine/cli/tests/l1/compose_initialize_staged_boot.rs
docs_updated_during_phase_3:
    - claudine/docs/topics/flow-control/looping.md
    - claudine/docs/pipeline.md
    - claudine/docs/topics/composition.md
    - claudine/docs/topics/flow-control/lifecycle.md
    - claudine/README.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
    - .claude/skills/claudine/SKILL.md
    - claudine/docs/use-skills/use-claudine/lifecycle.md
source_code:
    - claudine/cli/tests/l1/loop_startup_source.rs
    - claudine/cli/tests/l1/main.rs
    - claudine/cli/tests/l1/loop_cli.rs
    - claudine/cli/tests/bin/fake_goose/main.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/overlay_layering.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/target_launch.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/target_launch/tests.rs
    - claudine/lib/src/composition/prepare.rs
    - claudine/lib/src/composition/prepare/entry.rs
    - claudine/lib/src/composition/prepare/service.rs
    - claudine/lib/src/composition/prepare/service/tests.rs
    - claudine/cli/src/commands/compose/mod.rs
    - claudine/cli/src/commands/compose/prep.rs
    - claudine/cli/src/commands/wrap/composition/mod.rs
    - claudine/cli/src/commands/wrap/composition/pipeline.rs
    - claudine/cli/src/commands/wrap/composition/runner.rs
    - claudine/cli/src/commands/wrap/composition/staged_boot.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/mod.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/requeue.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/retry_resume.rs
    - claudine/cli/src/commands/wrap/harness_orch/mod.rs
    - claudine/cli/src/commands/wrap/harness_orch/prompt.rs
    - claudine/cli/src/commands/wrap/harness_orch/prompt/tests.rs
    - claudine/cli/src/commands/wrap/harness_orch/types.rs
    - claudine/cli/src/commands/wrap/wrapper_stages.rs
    - claudine/cli/tests/l1/compose_initialize_staged_boot.rs
documentation:
    - claudine/docs/topics/flow-control/looping.md
    - claudine/docs/pipeline.md
    - claudine/docs/topics/composition.md
    - claudine/docs/topics/flow-control/lifecycle.md
    - claudine/README.md
    - .claude/skills/claudine/SKILL.md
    - claudine/docs/use-skills/use-claudine/lifecycle.md
completed_phase: 3
implemented: true
---

# Implementation Log for 2026-11-20-loop-reread-context-drift (3 phases)

## Phase 1

Phase 1 adds regression tests only; no production path changed. Each new
test pins the behavior the spec requires, so the ones that pin the defect fail
until Phase 2 lands. Those failures are the phase's deliverable (the plan's
"Baseline checkpoint"), not regressions.

### Fixture additions

- `cli/tests/bin/fake_goose/main.rs` (plan mode): with both
  `FAKE_FIRST_REWRITE_PATH` and `FAKE_FIRST_REWRITE_FROM` set, launch 1 copies
  the `FROM` bytes over `PATH` after recording its prompt. Marker mode and runs
  with neither variable are unchanged.
- `cli/tests/l1/loop_startup_source.rs` (new, registered in `tests/l1/main.rs`
  under `test-fixtures`): a committed Git fixture with the native fake goose,
  the native fake editor as `code` (reached only through `EDITOR`) and as
  `rewrite-source` on `PATH`, prompt recordings under the fixture workspace
  (outside the observed repository), lifecycle markers in a git-ignored
  `events.log`. No sleeps, no shell scripts for the provider.
- `loop_control/target_launch.rs`: `mod tests` became `pub(super) mod tests`
  under `#[cfg(test)]`, and `intent()`, `FIXTURE_MODEL`,
  `without_ambient_model()` became `pub(in crate::commands::wrap::harness_orch)`
  so the harness guard tests reuse the target-launch fixture identity.

### Requirement → test mapping

| Spec / plan requirement | Test | Boundary, fidelity, tier |
| --- | --- | --- |
| Staged proxy loop keeps the post-`initialize` read; `initialize` once; condition reads stabilized frontmatter (`required_passes`); caller and `proxy.with` values both passes; new `ctx.hostname` group in `start`; pre-init observation kept on pass 1, fresh tree on pass 2; provider rewrite (body, frontmatter, start/failure, schema, loop) seen only by a new run | `loop_startup_source::staged_proxy_loop_retains_its_first_validated_root` | CLI process + filesystem + Git, F3, L1 |
| Delegated (harness `StabilizeOnly`) read is the retained source, distinct from both adjacent disk versions | `loop_startup_source::delegated_proxy_loop_retains_the_harness_read` | CLI process, F3, L1 |
| Staged single: rewritten `start` sees the new `ctx.branch` group with the composition's context and keeps the pre-init file-change observation, fires once | `loop_startup_source::staged_single_uses_the_rewritten_lifecycle_context` | CLI process, F3, L1 |
| Staged single: valid read then denied lifecycle shell audit routes through the rewritten `blocked`/`finalize` with the new group; no launch; early catch shell never runs | `loop_startup_source::staged_single_denied_audit_routes_through_the_rewritten_catch_context` | CLI process, F3, L1 |
| Later ordinary direct pass needs no root on disk; a new run does | `loop_startup_source::ordinary_direct_pass_uses_retained_root_after_removal` | CLI process, F3, L1 |
| Each sequence prompt task invocation reads its own source at entry | `loop_startup_source::sequence_task_reopens_root_between_invocations` | CLI process, F3, L1 |
| Retry reads disk with the same pass number; the next ordinary pass returns to the startup source; `initialize` once | `loop_startup_source::document_loop_retry_does_not_replace_startup_source` | CLI process, F3, L1 |
| Harness `StabilizeOnly`: prompt and guard-rendered `start` share branch/model/earlier file list; one `file_changes` and one `git` observation; `Lifecycle` consumer on the stabilized epoch | `loop_control::tests::overlay_layering::stabilized_harness_events_use_the_active_epoch` | in-process real guard, F2, L1 |
| Same through the adopted route that calls `rebuild_target_launch` | `…::stabilized_harness_events_use_the_active_epoch_after_target_rebuild` | in-process, F2, L1 |
| Setup failure after installation (`timeout: 1s`, `step_timeout: 2s` fails `parse_harness_plan`) routes through rewritten `blocked`/`finalize` with the new context, no `start` | `…::a_stabilized_plan_failure_routes_through_the_rewritten_catch_and_context` (StabilizeOnly), `…::a_stabilized_plan_failure_after_target_rebuild_routes_through_the_rewritten_catch` (adopted) | in-process, F2, L1 |
| Earlier stabilized shell-audit failure routes through the bootstrap catch with the bootstrap context | `…::a_stabilized_shell_audit_failure_routes_through_the_bootstrap_catch_and_context` | in-process, F2, L1 |
| Approved body `::shell` first seen on the stabilized read runs; an early catch shell never reaches the runner | `…::an_approved_stabilized_body_shell_runs_but_an_early_catch_shell_never_does` | in-process, F2, L1 |
| Inline: pass 2 succeeding without an edit gets the unchanged-body verdict; pass 1's deliverable kept byte for byte | `loop_cli::inline_compose_loop_refuses_a_later_pass_that_leaves_the_deliverable_unchanged` (existing positive `inline_compose_loop_runs_iterations` kept) | CLI process + filesystem, F3, L1 |

The input robustness matrix does not apply (plan, "Validation scope"): no
reader, field validator, or grammar changes.

### Baseline checkpoint

Command (from `claudine/`): `just test-cli loop_startup_source overlay_layering loop_cli`.
The recipe is fail-fast, so the three filters were also run separately to
get every named outcome. Host load average about 0.7 on 16 cores.

**Fail at baseline, on the intended defect (9):**

- `staged_proxy_loop_retains_its_first_validated_root` — exit 1 before the
  second launch: `start[0].append_line` raised "interpolation of
  `ctx.hostname` failed: context variable `ctx.hostname` belongs to the Os
  capture group, which this composition request did not capture". This is the
  reported defect on the command staged route (proxy target with
  `initialize`). Pass 1's prompt was the stabilized body
  (`STABILIZED-BODY pass=1| caller=caller| proxy=proxy| host=…`), so the
  composition used the new read while the event used the old context.
- `staged_single_uses_the_rewritten_lifecycle_context` — same failure for
  `ctx.branch` (Git group) on the direct staged single route; prompt was
  `REWRITTEN-BODY branch=main| …`.
- `delegated_proxy_loop_retains_the_harness_read` — the prompts happen to
  match (the original body's `::shell rewrite-source` re-ran on pass 2 and
  recreated the harness version before the harness reread it), but the
  editor ran twice and the provider's third version was overwritten on disk.
  Route evidence: pass 2 composed the caller-resolved original (with the
  directive), not the harness-validated read.
- `ordinary_direct_pass_uses_retained_root_after_removal` — iteration 2 exits
  1: "source file does not exist" from the harness existence check, after
  composing `RETAINED-BODY pass=2`.
- `document_loop_retry_does_not_replace_startup_source` — events were
  `initialize, start ORIGINAL pass=1, start ORIGINAL pass=1, start RECOVERY pass=2`
  and prompts `ORIGINAL pass=1, RECOVERY pass=1, RECOVERY pass=2`: the retry
  read the new body but ran the old `start`, and the next ordinary pass
  adopted the recovery read as its source.
- `inline_compose_loop_refuses_a_later_pass_that_leaves_the_deliverable_unchanged`
  — pass 2 exits 0: pass 1's edit counted as pass 2's change.
- `overlay_layering::stabilized_harness_events_use_the_active_epoch`
  (StabilizeOnly) — guard-rendered `start` raised the `ctx.branch` Git-group
  error; the prompt rendered `branch=main model=fixture/opencode-model
  files=["early.txt"]`. Observations were already `{file_changes: 1, git: 1}`;
  no `Lifecycle` consumer.
- `overlay_layering::stabilized_harness_events_use_the_active_epoch_after_target_rebuild`
  — prompt and `start` agree (the rebuild installs the new context), but no
  `Lifecycle` consumer is recorded on the stabilized epoch (consumers
  `{body: 2, effective-frontmatter: 3, preflight: 3}`); only
  `install_reentry_lifecycle_context` records one, and only on retry/resume.
- `overlay_layering::a_stabilized_plan_failure_routes_through_the_rewritten_catch_and_context`
  — `blocked` and `finalize` raise the `ctx.branch` error, so the plan error
  is replaced by a lifecycle evaluation error.

**Pass at baseline (controls and already-correct routes):**

- `staged_single_denied_audit_routes_through_the_rewritten_catch_context` —
  the command staged single route already routes a denied stabilized audit
  through the rewritten `blocked`/`finalize` with the new context, and the
  catch shell does not run. Kept as the command-route evidence for that
  boundary; the harness-route twin is the failing plan-failure case above.
- `sequence_task_reopens_root_between_invocations` — each prompt task's
  invocation already reads its own source; pins that Phase 2 must not move
  this boundary.
- `overlay_layering::a_stabilized_plan_failure_after_target_rebuild_routes_through_the_rewritten_catch`,
  `…::a_stabilized_shell_audit_failure_routes_through_the_bootstrap_catch_and_context`,
  `…::an_approved_stabilized_body_shell_runs_but_an_early_catch_shell_never_does`
  — the adopted post-rebuild catch, the bootstrap catch on an earlier audit
  failure, and the shell-free early catch are already correct.
- All pre-existing tests in `overlay_layering` (31) and `loop_cli` (17) pass,
  including the positive two-edit inline loop.
- The existing control `compose_initialize_staged_boot::a_file_still_missing_after_initialize_blocks_once_without_launching`
  is unchanged.

### Stale comment identified

`compose_initialize_staged_boot.rs` above
`a_sequence_task_proxy_target_initialize_creates_a_file_its_body_includes`
says a sequence task's `initialize` proxy "is adopted inside the provider
harness". A prompt task carries a task ledger through `task_run.rs`, so the
committed handoff returns to the command coordinator; the passing test is
command-route evidence. The plan schedules the comment fix in Phase 3
("Repo guidance").

### Deviations from the plan's test wording

- The staged proxy test renders `ctx.hostname` in the stabilized body too,
  so the expected host value is read from the numbered prompt recording (as
  the plan asks) rather than from the host.
- The harness approved-shell case reaches an early catch through the
  `timeout`/`step_timeout` plan failure, because no other early catch runs
  after a successful stabilized read; its `blocked` marker sits before the
  shell action because the locked shell aborts the rest of that stack.
- The adopted harness variants do not exercise a `rebuild_target_launch`
  failure: no cheap fixture makes that rebuild fail.
- Only the retry recovery is covered by a new process test; resume keeps its
  existing coverage in `ctx_per_run.rs` and `lifecycle_traces.rs`.

### Phase gates

Run from `claudine/` after all edits, with no subagent running (load average
under 1 on 16 cores):

- `just test` (default environment, `LANG=C`): stopped after 2,323 tests on
  four **pre-existing** library failures,
  `claudine::composition::sequence::task::tests::group_framing::{a_cap_of_one_preserves_the_group_bar_at_the_same_left_edge, a_members_body_output_lands_on_the_data_channel_not_the_status_one, every_member_task_opens_and_closes_exactly_one_stream, every_body_line_carries_its_own_tasks_bar}`.
  They fail alone too, and pass with a scrubbed environment. Bisecting the
  environment showed a non-UTF-8 effective locale causes them: it renders the
  group header glyph `▶` as `>`, and these tests look for `▶`. This phase does
  not touch the library, so they are unrelated to it.
- `LANG=C.UTF-8 just test`: stopped on
  `dispatch_inventory::dispatch_inventory_matches_committed_file`. The new
  harness test named `Provider::Goose` twice under `cli/src`, which moved the
  site count from 1863 to 1865. Fixed by taking the provider from the fixture
  intent (`intent.fallback_provider`), so the committed inventory is
  unchanged. That test passes alone now.
- `LANG=C.UTF-8 just test --max-fail=30` (a failure budget, so the run
  continues past the known baseline failures): **12,671 tests run, 12,662
  passed, 9 failed, 15 skipped**. The 9 failures are exactly the baseline
  regressions listed above. `🐢 L1 run recorded: 12671 test(s), none at or
  above 5.0s`.
- `just lint`: exit 0, no warnings.
- `just check-tier-coverage claudine`: nothing stranded.
- Dependent packages: no workspace package depends on `claudine-cli`, and
  this phase changed only its tests and a test-fixture binary, so there are no
  downstream tests to run.

The `claudine` skill needed no change: no architecture or workflow changed.
No local OS test rigs (`BUILD_LINUX`, `BUILD_WIN`, `BUILD_WSL`, `BUILD_MACOS`)
are configured on this host. The new tests use native fixture binaries and
`biscuit_test_harness::bin_exe!`, and `loop_startup_source` is compiled under
`test-fixtures` only, with no `cfg(unix)` gate. A body `::shell` runs through
`cmd` on Windows, so CI's Windows leg is the first real evidence for
`delegated_proxy_loop_retains_the_harness_read`.

## Phase 2

Phase 2 changes production code in the `claudine` library and `claudine-cli`
so that every Phase 1 baseline failure passes without editing its assertions.

### What changed

**Library — one canonical later-pass stage**
(`lib/src/composition/prepare/service.rs`, `prepare.rs`, `prepare/entry.rs`)

- New `SchemaStage::SkipLaunchVerdict`. Its branch sets
  `PrepareOptions::defer_schema_verdict = false` (so `resolve_launch_schema`
  keeps its typed errors) and calls the new `prepare_loop_iteration`, which
  passes a private skip flag to `prepare_direct_on` / `prepare_inline_on`.
  There the Darkmatter verdict-suppression option is set from
  `options.defer_schema_verdict || skip_launch_verdict`, so defaults and
  coercion still apply but no verdict is reached. The result carries
  `schema_verdict_deferred == false` (set from the stage). Every existing
  caller passes `false` / `None`.
- Inline later passes read the deliverable once, before any composition
  effect, and use those bytes for `InlineClosurePlan::original_document_text`,
  the baseline rehearsal, and the `MdHashKind::Simple` hash. Prompt,
  lifecycle, provenance, and schema stay on the retained source. An
  unreadable deliverable fails with `InlineArtifactUnreadable` naming the path.
- `DocumentEntryReason::LoopIteration`, `SourceBasis::StampedStructuralPlan`,
  and `PreparationStages::full_validation` docs now say a later pass composes
  the retained source, reaches no verdict, and still audits its own commands
  deny-only; retry/resume are named as the fresh-read entries.

**Harness** (`wrap/harness_orch/{types,mod,prompt,loop_control}.rs`)

- New CLI-private `RetainedLoopSource { source, approvals }` and
  `LoopPassPreparation { context, frontmatter, document_epoch }`, carried on
  `HarnessPromptState` (`stabilized_source`, `pass_preparation`) and moved by
  `runner.rs` into `SingleCompositionOutcome`. All other state initializers
  and outcome constructors set `None`.
- `materialize_harness_prompt_selecting` returns the materialization plus the
  owned source and `input_layers.pre_approved_commands` (absent → empty). The
  test-only `materialize_harness_prompt` still returns only the prompt.
- `AttemptDocument.retained`: `None` for the seed, `Some` for a fresh read.
- `bootstrap_adopted_document_phase` keeps the previous materialization until
  the stabilized lifecycle audit passes (a refusal is caught by the bootstrap
  actions, frontmatter, context, and epoch), then installs the stabilized
  actions and context together through `staged_boot::install_prepared_lifecycle`
  before the adopted target's launch rebuild can fail. On a successful rebuild
  the rebuilt context updates both `materialized.compose_context` and the
  guard. On attempt 1 (non-passthrough) the retained read is saved to
  `stabilized_source` before `start`.
- Recovery: new `install_recovery_preparation` audits a retry/resume read's
  lifecycle shells under the frozen harness policy, then installs its actions
  and context before the harness plan is parsed, so its `start`, `blocked`, and
  `finalize` use the recovery preparation. The start-only
  `install_reentry_lifecycle_context` is removed (and `run_start_lifecycle_event`
  lost its now-unused `prompt_state` parameter; the two `retry_resume.rs` tests
  install through the shared helper as production does).
- `publish_pass_preparation` records the guard's installed context with the
  same materialization's frontmatter and epoch, after boot/recovery
  installation and before the plan parse; passthrough publishes nothing.
- Recovery failure routes (fresh-document preflight, failed materialization,
  refused recovery audit) keep the empty frontmatter view but take its epoch
  from the last installed `pass_preparation` (`recovery_failure_view`).
- `prepare_harness_plan_phase` skips the source-existence check only for
  `HarnessPromptMode::Compose` with entry `LoopIteration`.

**Command** (`compose/prep.rs`, `compose/mod.rs`, `wrap/composition/{staged_boot,pipeline,mod}.rs`)

- `staged_boot::install_prepared_lifecycle(guard, config, context, epoch,
  invocation)` sets config and context without touching the event ledger and
  records one `Lifecycle` consumer (epoch, else invocation).
- `build_and_run_loop` keeps one `Option<RetainedLoopSource>`: eager pass 1
  that owns the verdict retains the caller-resolved source with its prepared
  approvals; staged pass 1 retains its post-`initialize` reread; a delegated
  pass 1 consumes `outcome.stabilized_source` once. The two later-pass arms
  collapsed into one: it requires retention (else `LoopIterationFailed`,
  exit 1, reason `startup preparation did not return a retained source`),
  clears the pending-verdict flag, begins a fresh epoch from the retained
  root's requirements (or clears inherited context/epoch without an
  invocation), reapplies the startup approvals, audits template commands with
  the frozen policy, prepares with `LoopIteration` + `SkipLaunchVerdict`, then
  audits lifecycle commands. `CompositionKind::prepare_without_schema` is
  removed. The staged-only source fallback is gone.
- Every pass installs its preparation through the shared helper before the
  pipeline. The staged loop's initialization guard and reread base are
  `staged.bootstrap.compose_context`; `run_staged_single` seeds its reread with
  `StagedLifecycleRuntime.context` and installs through the helper.
- `pipeline.rs`: in the external-guard branch the guard context is set from
  `LifecyclePhase.context` (missing-group extension and target env included)
  before the runner; the pipeline records `Lifecycle` only for a guard it owns.
- `LoopIterationOutput.context`/`frontmatter` come from the returned
  `pass_preparation`, else the locally prepared pair (including the
  downgraded-iteration route). `LoopCondition` is recorded on the epoch used
  for that result; the bootstrap record before templated `max` stays.
- Comments around `build_and_run_loop`, `execute_composition_attempt`, the
  stabilized-boot rebuild ("second capture"), and the harness epoch docs that
  still described a harness-side "loop refresh" were corrected.

### Sequence scope

No sequence-specific path was added. `task_run.rs` sends an owning document
loop through `run_looping_document` → `prepare_and_run_active_document` →
`build_and_run_loop`, so it gets the same retention; each prompt-task
invocation still resolves and reads its own source
(`loop_startup_source::sequence_task_reopens_root_between_invocations` and
`sequence_referenced_state` pass).

### Requirement → test mapping (Phase 2 additions)

| Requirement | Test | Boundary, fidelity, tier |
| --- | --- | --- |
| Later pass reaches no verdict, is not marked deferred, keeps the schema and defaults; `Validate` control still refuses | `composition::prepare::service::tests::loop_iteration_skips_the_launch_verdict_without_deferring_it` | library public API, F2, L1 |
| Later inline pass baselines on the live deliverable (text and hash), prompt from the retained source; unreadable deliverable → `InlineArtifactUnreadable` with path | `composition::prepare::service::tests::a_later_inline_pass_baselines_on_the_live_deliverable` | library public API + filesystem, F2, L1 |
| Harness stabilized retention: stored source is the rewritten root; exact approval set is `echo approved-by-harness` | `…::overlay_layering::an_approved_stabilized_body_shell_runs_but_an_early_catch_shell_never_does` (extended) | in-process real guard, F2, L1 |

The nine Phase 1 baseline failures are the process/harness acceptance tests
for the rest; all nine pass now with unedited assertions.

The input robustness matrix does not apply: no reader, field validator, or
grammar changed.

### Implementation checkpoint (focused paths)

From `claudine/`, `LANG=C.UTF-8`:

- `just test-library --features work-counters composition::prepare::service`:
  24 passed (both new tests included).
- `just test-cli loop_startup_source`: 7 passed (the five Phase 1 baseline
  failures in this module now pass). `just test-cli loop_cli`: 18 passed
  (`inline_compose_loop_refuses_a_later_pass_that_leaves_the_deliverable_unchanged`
  included). `just test-cli stabilized`: 8 passed (the three harness baseline
  failures included). `overlay_layering retry_resume loop_control`: 173 passed.
- `ctx_per_run` 30, `lifecycle_traces` 47, `loop_primitives` 17,
  `loop_retry_passes` 3, `sequence_referenced_state` 101,
  `sequence_frozen_graph` 25, `inline_prompt_shell_preflight` 16,
  `ctx_launch_anchor` 10, `compose_initialize_staged_boot` 21: all passed,
  including `ctx_per_run::a_later_loop_iteration_observes_the_previous_iterations_staging`.

The canonical-boundary evidence that a later pass has no pending verdict and
takes the no-verdict branch is
`loop_iteration_skips_the_launch_verdict_without_deferring_it` (the same
source refuses under `Validate`). The process complement is
`delegated_proxy_loop_retains_the_harness_read`: the editor directive runs
once, so pass 2 neither recomposes the original body nor stabilizes again.
No raw `harness_materialize_prompt` span count was used.

### Phase gates

Run from `claudine/` after all edits, with no subagent running (no subagents
were used this phase):

- `LANG=C.UTF-8 just lint`: exit 0, no warnings.
- `LANG=C.UTF-8 just test --max-fail=30`: **12,673 run, 12,673 passed, 15
  skipped**, `🐢 L1 run recorded: 12673 test(s), none at or above 5.0s`, exit 0.
  Load average during the run rose to about 16 on a 16-core host.
- `just check-tier-coverage claudine`: nothing stranded.
- `git diff --check`: clean.
- Dependent packages: `claudine-contract` is the only other dependent of the
  `claudine` library and is part of the area's own `just test`; nothing
  depends on `claudine-cli`.

The `claudine` skill is left to Phase 3, whose "Repo guidance" task updates
`.claude/skills/claudine/SKILL.md` with the startup source and epoch handoff.
No local OS test rigs (`BUILD_LINUX`, `BUILD_WIN`, `BUILD_WSL`, `BUILD_MACOS`)
are configured on this host; cross-OS evidence is left to CI. The changes add
no path comparison, `cfg`, or OS-specific code.

## Phase 3

Phase 3 documents the read boundaries Phase 2 implemented, pins the
documentation's examples with process tests, and runs the full gates. No
production code changed.

### Documentation

- `docs/topics/flow-control/looping.md`: new section **What each pass
  reads**. It has a table of what a later pass sees and an example that runs
  as written. In the example, `set` carries `last_pass` into the next pass, an
  explicit `frontmatter('progress.md', 'done')` read ends the loop, and the
  agent's edit to the root reaches only the next run. A Mermaid flow shows
  startup, kept read, pass, gate, and recovery. The section also covers retry
  and resume, proxy, sequence tasks, and the `inline-compose` deliverable read,
  and cross-links the existing `loop.action` example (`#actions`).
- **Drift found and fixed in the same file:** the `loop.last_exit_code`
  section's `inline-compose` sentinel example looped on a bare
  `until: "done == true"`. Under the retained-source rule a bare property
  reads the kept document, so the loop never sees the agent's `done: true`.
  Reproduced by hand: four launches, then `loop limit exceeded … cap is 2/4`.
  The example now reads `frontmatter('task.md', 'done')` and says why. Both
  shapes are pinned by tests (below). Recorded in `decisions_for_review`.
- `docs/topics/composition.md`:
  - *Loop Execution* names the kept read.
  - *Documents That Declare `initialize`* says the stabilized reread is the
    read a loop keeps, and that its actions are installed together with its
    context, including for `blocked`/`finalize` when its checks fail.
  - The entry matrix row now reads "the document the loop kept at startup".
    Its paragraph says no verdict is reached and none is left pending, while
    defaults and the completion schema stay.
  - New subsection **Which read a loop keeps** has the per-route table
    (eager / staged / delegated proxy target), what the kept read carries, the
    deny-only audit, and the fact that the root file need not exist. It also
    gives the read boundary of retry/resume, proxy, sequence tasks, and
    includes/explicit reads, and the context/epoch handoff, plus a Mermaid
    sequence diagram based on the spec's diagram.
  - *Launch-time baseline check* gains **A later loop iteration baselines on
    the live file**: rehearsal, hash, and closure use live bytes; retry/resume
    keep the pass's baseline; unchanged → *document not updated*, earlier edit
    kept; unreadable → `InlineArtifactUnreadable`.
- `docs/pipeline.md` B4: "Later loop iterations reuse the audited structural
  plan" is replaced by a **Which read a loop keeps** block: the three routes,
  what a later iteration does, recovery, the inline live baseline, and the
  install-together rule. There is a Mermaid flowchart of startup and
  iteration, and B5.3 is cross-referenced.
- `docs/topics/flow-control/lifecycle.md`:
  - The staged-boot paragraph no longer says "structural plan".
  - The frontmatter-lifetime sentence used to say a `set_frontmatter` "between
    loop iterations is visible to the next iteration". It now says the write
    is visible to later actions and events of the same attempt (the
    per-attempt live frontmatter cell). Values for the next iteration belong
    in `set`/`loop.action`, and a disk write to the root is seen by the next
    run, retry/resume, or a new sequence task.
  - Pre-warm submission points: an ordinary pass warms from its own
    preparation of the kept document.
  - *Limits*: a `set_frontmatter` rewrite is warmed again only where Claudine
    prepares from disk (after `initialize`, retry/resume, next run or task).
  - *Loop lifecycle concerns* no longer claims iterations skip shell
    pre-flight.
- `README.md`: a short retention summary that links to the looping guide.

### Repo guidance

- `.claude/skills/claudine/SKILL.md`: new **Document-loop read boundary**
  paragraph after "One loop for every primitive". It covers
  `RetainedLoopSource` per route, the one later-pass arm (`LoopIteration` +
  `SkipLaunchVerdict`, fresh epoch, audits), the root-existence bypass, the
  inline live baseline, recovery installation, `install_prepared_lifecycle`,
  `pass_preparation`, and test and doc pointers. Every named symbol was
  checked to exist.
- Stale comment fixed: `cli/tests/l1/compose_initialize_staged_boot.rs` above
  `a_sequence_task_proxy_target_initialize_creates_a_file_its_body_includes`
  now says the prompt task's ledger surfaces the proxy to the command-level
  coordinator, instead of saying it "is adopted inside the provider harness".

### Caller guidance

- `docs/use-skills/use-claudine/lifecycle.md`: under the `loop:` example, a
  **What a later pass reads** bullet. It states the retained-root rule, the
  recovery reads, the explicit-read advice (which works on the document's own
  file too), and the inline deliverable rule. It points to
  `claudine/docs/topics/flow-control/looping.md#what-each-pass-reads`, written
  as a code path, which is the caller skills' existing convention, since an
  installed skill cannot follow a relative link into the repository.
- `just publish-skills "$(pwd)/../target/claudine-loop-skills"` from
  `claudine/`: exit 0. The emitted `use-claudine/lifecycle.md` keeps the
  existing `info: "pass {{ loop.count }} done"` expression example verbatim
  and contains the new rule paragraph. `git status` afterwards shows no
  tracked or untracked publication artifacts (the output is under
  `target/`).

### Tests added (requirement → test)

| Requirement | Test | Boundary, fidelity, tier |
| --- | --- | --- |
| The looping guide's example runs as written: `set` carries into pass 2, the gate's `frontmatter('progress.md', 'done')` read ends the loop after pass 2, pass 2 composes the kept root rather than pass 1's rewrite, and a new run composes the rewrite | `loop_startup_source::looping_guide_example_keeps_the_root_and_carries_set_values` (reads `looping.md` with `include_str!` inside the test) | CLI process + filesystem + Git fixture, F3, L1 |
| The corrected inline sentinel example: an explicit read of the document's own file sees the agent's `done: true` and ends after one pass | `loop_startup_source::looping_guide_inline_sentinel_reads_the_live_file` (extracts the example from `looping.md`) | CLI process + filesystem, F3, L1 |
| Negative control: a bare `done` reads the kept document, so the loop reaches its cap (`loop limit exceeded`) despite the agent's edits | `loop_startup_source::a_bare_inline_sentinel_reads_the_kept_document` | CLI process + filesystem, F3, L1 |

Fixture change: `claudine-fake-goose` plan mode gains
`FAKE_SECOND_REWRITE_PATH`/`FAKE_SECOND_REWRITE_FROM`, which act on launch 2
the way the `FIRST` pair acts on launch 1. Runs with neither pair set behave
as before. `LoopRun::second_launch_rewrites` wraps the new pair.

The input robustness matrix does not apply: no reader or validator changed.

### Review material (from Phases 1–2, unchanged in Phase 3)

- Startup root replacement trace:
  `staged_proxy_loop_retains_its_first_validated_root` (two launches, one
  target initialize, the stabilized body/lifecycle on both passes, the
  init-created file only in pass 2's working-tree view, the third root used
  only by the fresh run) and `delegated_proxy_loop_retains_the_harness_read`.
- Recovery trace: `document_loop_retry_does_not_replace_startup_source`
  (events `initialize`, `start ORIGINAL pass=1`, `start RECOVERY pass=1`,
  `start ORIGINAL pass=2`).
- Inline preservation:
  `loop_cli::inline_compose_loop_refuses_a_later_pass_that_leaves_the_deliverable_unchanged`
  (exact first-pass bytes kept).
- Context counters: `overlay_layering::stabilized_harness_events_use_the_active_epoch{,_after_target_rebuild}`
  (`DocumentEpoch::work_snapshot`: one `file_changes` and one `git`
  observation, a `Lifecycle` consumer on the stabilized epoch).

### Phase gates

Run from `claudine/` after all edits, with no subagent running (none were
used this phase). The load average at the start of the suite was 4.1 on 16
cores.

- `LANG=C.UTF-8 just lint`: exit 0, no warnings.
- `LANG=C.UTF-8 just test --max-fail=30`: **12,676 run, 12,676 passed, 15
  skipped**, `🐢 L1 run recorded: 12676 test(s), none at or above 5.0s`, exit
  0. `LANG=C.UTF-8` is used because a non-UTF-8 effective locale breaks four
  unrelated group-framing tests (see Phase 1).
- `just check-tier-coverage claudine`: nothing stranded.
- `git diff --check`: clean.
- Scope calculator (repository root, after every edit including plan, log,
  and spec frontmatter):
  `git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`
  gives `packages: [claudine, claudine-cli, test-toolkit, darkmatter]` and
  `change_class: package`. The working tree still holds Phase 2's uncommitted
  changes, so this covers both phases. Running it file by file shows why each
  extra package appears:
  - `darkmatter` appears because every changed Markdown file selects one
    narrowed L1 cell. Its filter is
    `schema_roots::no_doc_or_skill_names_the_singular_schema_variable`,
    `shipped_schema_corpus::shipped_markdown_inline_schemas_parse_passively`,
    and `shipped_schema_corpus::shipped_markdown_named_type_references_resolve`.
    Ran exactly that filter with `cargo nextest run -p darkmatter -E …`:
    3 passed.
  - `test-toolkit` appears because a claudine source change selects its lint
    companion `archive-path-guard`.
    `cd tools/test-toolkit && just archive-path-guard`: 2 passed.
- Dependent packages: `claudine-contract`, the only reverse dependency, is
  part of the area's own `just test`. Nothing depends on `claudine-cli`.

No local OS test rigs (`BUILD_LINUX`, `BUILD_WIN`, `BUILD_WSL`,
`BUILD_MACOS`) are configured on this host, so cross-OS evidence is left to
CI. The new tests use the native fixture binaries and portable file
rewrites, with no `cfg` gate.

## Review correction — 2026-11-21

The statements that `LANG=C` alone always breaks the four group-framing
tests were too broad. `LANG=C just test-library group_framing` passed all
12 tests during review because this session inherited `LC_ALL=C.UTF-8`
and `LC_CTYPE=C.UTF-8`. Setting all three explicitly with
`LANG=C LC_ALL=C LC_CTYPE=C just test-library group_framing --max-fail=10`
reproduced exactly the four recorded failures (8 passed, 4 failed).
The statements above now name the effective locale; the original run
outcomes remain historical evidence. The test defect is recorded in
review-1.md for repair and requires no author decision.
