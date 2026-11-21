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
packages:
    - claudine-cli
decisions_for_review:
    - |-
        Phase 1 ends with nine new regression tests failing on purpose. The plan treats them as a baseline to record before moving straight to Phase 2, but the phase's completion rule also asks for every test to pass. I kept them failing and visible rather than hiding them with #[ignore], a feature gate, or assertions inverted to match today's behavior. Each of those would leave a test that cannot catch the defect and would have to be undone in Phase 2. The trade-off is that the area's full test run is red between Phase 1 and Phase 2. If a reviewer wants every phase green, the nine tests would need to be marked or split out, and the marks removed when Phase 2 lands.
    - |-
        Four existing library tests (sequence group framing) fail whenever the shell's LANG is a non-UTF-8 locale such as LANG=C. The header glyph then becomes ">" instead of the arrow these tests look for. I did not fix them: they are in code this fix never touches, they pass under a UTF-8 locale, and changing how the locale picks glyphs (or forcing one in the tests) is a separate decision. I ran the full gate under LANG=C.UTF-8 instead. A reviewer may prefer making those tests independent of the locale; that is a small, separate change in claudine/lib/src/composition/sequence/task/tests.rs.
    - |-
        The plan asked for a retry-and-resume recovery case modeled on existing tests. I added only the retry process test (document_loop_retry_does_not_replace_startup_source) and left resume to its existing coverage in ctx_per_run.rs and lifecycle_traces.rs, because both recoveries take the same fresh-read path the retry test pins. If a reviewer wants resume pinned against the retained source too, it is one more test in loop_startup_source.rs using a provider that reports a session (resume needs one).
message_to_agent: |-
    Phase 1 changed no production code. Nine new tests fail on purpose and are Phase 2's acceptance targets. Fix production code until all of them pass; do not edit their assertions to match today's behavior. The failing tests are loop_startup_source::{staged_proxy_loop_retains_its_first_validated_root, staged_single_uses_the_rewritten_lifecycle_context, delegated_proxy_loop_retains_the_harness_read, ordinary_direct_pass_uses_retained_root_after_removal, document_loop_retry_does_not_replace_startup_source}, loop_cli::inline_compose_loop_refuses_a_later_pass_that_leaves_the_deliverable_unchanged, and loop_control::tests::overlay_layering::{stabilized_harness_events_use_the_active_epoch, stabilized_harness_events_use_the_active_epoch_after_target_rebuild, a_stabilized_plan_failure_routes_through_the_rewritten_catch_and_context}. The log's "Baseline checkpoint" section gives each failure message and the route it proves. Things to know: (1) this host's shell has LANG=C, which makes four unrelated library group_framing tests fail, so run the full gate as `LANG=C.UTF-8 just test` from claudine/. (2) `just test` stops at the first failure; while baseline tests are still red, add `--max-fail=30` to see the whole suite. (3) Test code under cli/src counts toward dispatch_inventory: never name a `Provider::` variant in new test code there (the harness tests take it from `intent.fallback_provider`). (4) The plan's Phase 2 step "extend the harness guard test after adding the feedback field" belongs in overlay_layering.rs, next to the shared run_staged_boot driver. (5) The stale comment above compose_initialize_staged_boot::a_sequence_task_proxy_target_initialize_creates_a_file_its_body_includes is scheduled for Phase 3.
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
  environment shows `LANG=C` alone causes them: a non-UTF-8 locale renders the
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
