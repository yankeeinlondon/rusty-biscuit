---
implementation_1: "2026-10-14T09:23:57-07:00"
implementation_2: "2026-10-14T10:00:10-07:00"
---

# TTS Pre-warm Log

## Implementation of Review Findings #1

> **started at:** 2026-10-14T09:23:57-07:00

- this implementation is attempting to implement _all_ of the review findings found in 'claudine/features/2026-10-22-tts-prewarm/review-1.md'
- this is iteration 1 of the review-to-implement cycle

### Finding Ledger

| ID      | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| ------- | --------------------------------------------------------- | ----------- | -------------------------------- |
| R1-F1   | high: Deferred task setup and teardown phrases are never reconsidered | fixed | all child rows F1.1–F1.9 verified; `just test` 9151 passed |
| R1-F1.1 | Top-level task setup: `env`, `ctx`, interpolated `derived` phrases submitted once task inputs exist | fixed | CLI `deferred_phrases_of_a_top_level_task_setup_are_prepared`: `task env Ada`, `task ctx <utc date>`, `task derived Widget ready` once each with task playback config; fails with submission disabled |
| R1-F1.2 | Top-level task teardown: same | fixed | CLI `deferred_phrases_of_a_top_level_task_teardown_are_prepared`; same assertions |
| R1-F1.3 | Serial group-member setup: same | fixed | CLI `deferred_phrases_of_a_serial_member_setup_are_prepared` (`open_member_run`) |
| R1-F1.4 | Serial group-member teardown: same | fixed | CLI `deferred_phrases_of_a_serial_member_teardown_are_prepared` |
| R1-F1.5 | Parallel group-member setup: same | fixed | CLI `deferred_phrases_of_a_parallel_member_setup_are_prepared` (`open_leg_run`) |
| R1-F1.6 | Parallel group-member teardown: same | fixed | CLI `deferred_phrases_of_a_parallel_member_teardown_are_prepared` |
| R1-F1.7 | Prompt document lifecycle control: `env`/`ctx` phrases prepared | clean | CLI `a_prompt_documents_env_and_ctx_phrases_are_prepared_resolved`: `doc env Ada` and `doc ctx <date>` prepared; also asserted in every row |
| R1-F1.8 | Group container setup/teardown: rejected by grammar, no speech site | clean | CLI `a_group_container_with_setup_is_rejected_before_any_preparation`: grammar error, no launch, no preparation |
| R1-F1.9 | Task scope preserved: sequence-injected keys stay late, `initialize` excluded, invocation handle reused | fixed | lib `task_stacks_resolve_with_the_tasks_captured_inputs`: overlay key `state` and handle-injected `topic` stay late, `initialize` never prepared, literal queued once (preflight 1, task submit 6 new); CLI reuses `invocation.prewarm()` handle, `literal control` appears once per row |
| R1-F2   | high: Task phrases are prepared with different settings from playback | fixed | all child rows F2.1–F2.11 verified; `just test` 9151 passed |
| R1-F2.1 | Top-level task setup: preparation config equals playback config | fixed | lib `task_stacks_use_task_playback_and_prompt_lifecycles_use_the_user_configuration` + CLI `a_mixed_sequence_prepares_task_stacks_and_prompts_with_their_playback_configuration`: `top setup` config = `TtsConfig::new()` (task playback); both fail when task stacks use user settings |
| R1-F2.2 | Top-level task teardown: same | fixed | same two tests, `top teardown` |
| R1-F2.3 | Serial group-member setup: same | fixed | same two tests, `serial member setup` |
| R1-F2.4 | Serial group-member teardown: same | fixed | same two tests, `serial member teardown` |
| R1-F2.5 | Parallel group-member setup: same | fixed | same two tests, `parallel member setup` |
| R1-F2.6 | Parallel group-member teardown: same | fixed | same two tests, `parallel member teardown` |
| R1-F2.7 | Compose document lifecycle control: Kokoro + `af_heart` both sides | clean | `collect_and_submit` passes the document's user settings as gate and playback; existing compose CLI pre-warm tests pass |
| R1-F2.8 | Sequence prompt document lifecycle control: same | clean | mixed-sequence lib and CLI tests: prompt phrases prepared with kokoro + `af_heart` |
| R1-F2.9 | Group container: rejected by grammar; clean | clean | group-container stacks rejected by grammar (see R1-F1.8); no walk site |
| R1-F2.10 | Worker carries per-candidate config; de-dup key uses that config; enablement separate | fixed | worker queues `(text, TtsConfig)`; key per candidate config; gates read only user `tts_prewarm`; lib `each_candidate_is_prepared_with_its_own_playback_configuration`, `prewarm_false_holds_when_the_first_submission_plays_with_default_settings` |
| R1-F2.11 | Mixed sequence test: task and prompt preparation each match their publication config | fixed | CLI `a_mixed_sequence_prepares_task_stacks_and_prompts_with_their_playback_configuration` |
| R1-F3   | Docs/skill updated alongside repairs (AC 11) | fixed | lifecycle.md (per-path voice table, task stacks reconsidered at task context), testing.md (recorder line format), claudine SKILL.md |
| R1-N1   | nonblocking (discovered): a `setup:`/`teardown:` nested inside the `group:` object is accepted and silently ignored; it never runs, so it is not a speech site | deferred | pre-existing grammar leniency outside this spec's code and not a pre-warm defect; next action: author decides on a fix spec |

### Progress

- order decision: R1-F2 is implemented before R1-F1 because F1's new task-stack submissions must carry task playback settings, which needs F2's per-candidate configuration seam
- starting the work on 'R1-F2 per-candidate playback configuration' at 09:24:24
        - worker queues `(text, TtsConfig)` per candidate; `PrewarmKey` built from each candidate's own config; gates read only the user settings' `tts_prewarm`
        - `PrewarmHandle::submit(user, playback, phrases, inputs)`; `collect_and_submit` passes the document's user settings as both
        - new `composition::sequence::task::task_playback_settings()` used by `run_step_task` and by preflight task-stack submission; prompt lifecycles keep the bridged user settings
        - recorder seam lines are now `text<TAB>config-json`; docs (lifecycle.md, testing.md) and claudine SKILL.md updated
        - tests added: 2 worker, 1 sequence preflight, 1 CLI mixed sequence; both new sequence tests fail when task stacks use the user settings (mutation checked, restored)
        - `just test` 9142 passed, 9 skipped; `just test-cli tts_prewarm::` 11 passed; `just lint` passed
        - defect class: preparation fixed one configuration per invocation instead of the configuration of the speech path that later plays each candidate
        - submission sites swept: compose loop bootstrap, staged boot, pipeline document prep, harness re-materialization (user settings; correct), sequence preflight (task stacks fixed to task playback settings)
- work completed for 'R1-F2 per-candidate playback configuration' at 09:36:37
- starting the work on 'R1-F1 deferred task setup/teardown reconsidered' at 09:36:37
        - new library entry point `prewarm::submit_task_stacks(handle, user, task, frontmatter, context)`; shares `parsed_task_stacks` with preflight's `Walk::task` (same `parse_task_action_stack`, scopes, no `initialize`); inputs = step effective frontmatter + task's captured `ComposeContext` (`DocumentContext::Captured`), `ROOT_OVERLAY_KEYS` late plus the step handle's injected keys (overlay, `--set`, runtime writes); playback = `task_playback_settings()`
        - CLI: `TaskStackPrewarm` (`cli/src/commands/wrap/sequence/task_prewarm.rs`) loads the bridged user settings once per step task only when the handle is live; submitted from `run_step_task` (after `prepare_task_context`), `open_member_run`, `open_leg_run`; member stacks evaluate against the same step frontmatter (`with_group` only adds the late `group` global)
        - `open_group_run`: step-level `setup:` on a group task is rejected ("not valid for a `group` task"); clean. Observation: a `setup:` nested inside the `group:` object is silently accepted and never runs or speaks (pre-existing grammar leniency, not a speech site)
        - sweep: proxy target, step prep (`jit::step_prepare_options`), compose loop iteration, harness re-materialization already submit via `collect_and_submit`; approval epoch and wrapped-memory overlay carry no lifecycle speech; no other `TaskExecution` constructor in production
        - tests: lib `task_stacks_resolve_with_the_tasks_captured_inputs`; CLI 6 macro rows `deferred_phrases_of_{a_top_level_task,a_serial_member,a_parallel_member}_{setup,teardown}_are_prepared`, `a_prompt_documents_env_and_ctx_phrases_are_prepared_resolved`, `a_group_container_with_setup_is_rejected_before_any_preparation`; all 6 rows fail with submission disabled (mutation checked, restored)
        - docs: lifecycle.md pre-warm section (diagram node, submission list, "left for their step" example); claudine SKILL.md pre-warm paragraph
        - `cargo nextest run -p claudine prewarm` 56 passed; `just test-cli tts_prewarm::` 19 passed; `just test` 9151 passed, 9 skipped; `just lint` passed
- work completed for 'R1-F1 deferred task setup/teardown reconsidered' at 09:50:20
- orchestrator reconciliation at 09:53:39
        - inspected combined diff: `task_playback_settings()` is the single source for `run_step_task` playback, preflight task stacks, and `submit_task_stacks`; `TaskStackPrewarm` loads user settings with the same `load_claudine_config(None, repo_root)` + `bridge_tts_settings` as preflight, only when the handle is live
        - `just test` (claudine): 9151 passed, 9 skipped
        - `PLAYA_DRY_RUN=1 just lint` (claudine): exit 0, no warnings or errors
        - `just test-cli tts_prewarm::`: all passing (19)
        - `git diff --check`: clean
        - OS: library changes are platform-neutral; the new CLI tests follow the file's existing Unix-only stub-provider pattern; no `cfg`/path code touched, so no cross-check run; CI covers Linux/macOS, nightly covers Windows/WSL2

### Completion Report

- **outcome:** complete
- **elapsed:** about 30 minutes (09:23:57 → 09:54)
- **parent findings:** total 3 = fixed 3 + already fixed 0 + blocked 0 + deferred 0 + unresolved 0 + disputed 0
        - R1-F1 fixed (child rows F1.1–F1.6 fixed, F1.7–F1.8 clean, F1.9 fixed)
        - R1-F2 fixed (child rows F2.1–F2.6, F2.10, F2.11 fixed; F2.7–F2.9 clean)
        - R1-F3 (docs alongside repairs, AC 11) fixed
- **nonblocking follow-up:** R1-N1 (group-object `setup:` silently ignored) is deferred to the author; it is not a defect against this spec
- **verified repairs**
        - each pre-warm candidate carries the playback `TtsConfig` of its speech path and is de-duplicated on it; the gate is decided only from the user's bridged settings
        - task `setup:`/`teardown:` stacks are prepared with `task_playback_settings()`, the same settings task playback uses
        - task stacks are resubmitted with resolved inputs when a top-level task, serial member, or parallel member establishes its context (`submit_task_stacks`)
- **changed files**
        - `claudine/lib/src/composition/lifecycle/prewarm/{worker.rs,worker/tests.rs,mod.rs,sequence.rs,sequence/tests.rs,fixture.rs}`, `claudine/lib/src/composition/lifecycle/tests/audio_emission.rs` (R1-F2, R1-F1)
        - `claudine/lib/src/composition/sequence/task/mod.rs` (R1-F2)
        - `claudine/cli/src/commands/wrap/sequence/{task_run.rs,task_prewarm.rs (new),mod.rs}` (R1-F1, R1-F2)
        - `claudine/cli/tests/l1/tts_prewarm.rs` (R1-F1, R1-F2)
        - `claudine/docs/topics/flow-control/lifecycle.md`, `claudine/docs/topics/testing.md`, `.claude/skills/claudine/SKILL.md` (R1-F3)
- **final verification**
        - `just test` in `claudine/`: 9151 passed, 9 skipped (all findings)
        - `PLAYA_DRY_RUN=1 just lint` in `claudine/`: passed (all findings)
        - `cargo nextest run -p claudine prewarm`: 56 passed (R1-F2.10, R1-F1.9)
        - `just test-cli tts_prewarm::`: 19 passed (R1-F1.1–F1.8, R1-F2.1–F2.8, F2.11)

## Implementation of Review Findings #2

> **started at:** 2026-10-14T10:00:10-07:00

- this implementation is attempting to implement _all_ of the review findings found in 'claudine/features/2026-10-22-tts-prewarm/review-2.md'
- this is iteration 2 of the review-to-implement cycle
- review `implemented` confirmed `false` before repairs

### Finding Ledger

| ID      | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| ------- | --------------------------------------------------------- | ----------- | -------------------------------- |
| R2-F1   | high: Sequence preflight coverage passes with preflight preparation disabled | fixed | all children F1.1–F1.8 verified; mutation fails the strengthened test; `just test` 9152 passed |
| R2-F1.1 | Three-prompt CLI test, recording mode: must fail when preflight submission is bypassed (later prompts' phrases observed before step one's provider exits) | fixed | `a_three_prompt_sequence_submits_every_document_before_step_one`: step-one stub polls the recorder (≤200 × 0.05 s) for `Beta failed` and `Gamma failed` and writes `prepared`/`timeout`; passes in 1.1 s; with the reviewer's `if true \|\| !handle.is_enabled()` bypass it fails `left: "timeout" right: "prepared"` (re-run by the orchestrator; source restored, no `if true` left) |
| R2-F1.2 | Three-prompt CLI test, blocking mode: still proves launch does not wait on a blocked preparation | fixed | split into `a_blocked_preparation_does_not_delay_any_sequence_step`: three launches, recorder `["setting up"]`, no spool; comment no longer claims preflight ordering |
| R2-F1.3 | Stale comment "only preflight collects it" updated with the assertion | fixed | comment removed; new doc comment says why only preflight can satisfy the in-run check; `docs/topics/testing.md` gains the what-vs-when recorder note |
| R2-F1.4 | Sibling: mixed-configuration sequence test (no before-first-step claim) | clean | its doc describes only per-path playback configuration; no ordering claim or assertion |
| R2-F1.5 | Sibling: six deferred task tests and prompt-context control (later phase by design) | clean | macro doc states phrases are prepared once the task context is established; the literal control asserts once-only, holds whoever submits |
| R2-F1.6 | Sibling: compose blocked-preparation test | clean | compose has no preflight; it claims and asserts only one launch plus one never-finishing preparation |
| R2-F1.7 | Sibling: library three-prompt preflight helper test | clean | `every_early_phrase_of_three_prompts_is_queued_before_any_step_runs` calls `submit_sequence_preflight` directly with no step running; six queued candidates prove helper ordering |
| R2-F1.8 | Sibling: remaining CLI tests (gates, proxy, retry, init rereads, invalid group) make no ordering claim | clean | grep of `preflight`, `before step`, `step one` in `tts_prewarm.rs` hits only the two sequence tests and the deferred helpers; `lifecycle.md` lines describe product behavior, still true |

### Progress

- starting the work on 'R2-F1 sequence preflight ordering test' at 10:00:31
        - defect class: a test that asserts only on the command's final state lets a later preparation phase stand in for the earlier phase it claims to prove
        - delegated to one foreground subagent (rust, rust-testing, claudine skills); it reports log items back to the orchestrator
        - test-only repair (subagent): new helper `write_three_prompt_sequence`; the two-mode loop split into a record-mode ordering test and a block-mode launch test
        - discovery: under the bypass the old after-the-run checks (four phrases, three launches) still pass; only an in-run observation distinguishes preflight from task-context and per-step submission
        - discovery: block mode records `setting up` with or without preflight because task-context submission queues step one's `setup:` first, so block mode can never prove ordering
        - discovery: under the regression the failing test waits out its ~10 s cap (14 s total); a normal run takes 1.1 s
        - orchestrator re-ran the mutation independently: FAIL `left: "timeout"`, then restored `cli/src/commands/wrap/sequence/mod.rs` byte for byte from a copy
        - `PLAYA_DRY_RUN=1 just test-cli tts_prewarm::`: 20 passed (was 19)
- work completed for 'R2-F1 sequence preflight ordering test' at 10:03:30

### Completion Report

- **outcome:** complete; elapsed about 9 minutes (10:00:10 → 10:09)
- parent findings: total 1 = fixed 1 + already fixed 0 + blocked 0 + deferred 0 + unresolved 0 + disputed 0
- child rows: 3 fixed, 5 clean; none open
- verified repair: R2-F1, the three-prompt sequence test now observes from inside step one's provider that steps two and three were prepared while step one ran, and fails when sequence preflight submission is bypassed; the blocked-preparation proof is a separate test
- open findings: none
- no performance measurement was required
- changed files:
        - `claudine/cli/tests/l1/tts_prewarm.rs` (R2-F1.1–F1.3)
        - `claudine/docs/topics/testing.md` (R2-F1.3, recorder what-vs-when note)
- final verification:
        - `PLAYA_DRY_RUN=1 just test` in `claudine/`: 9152 passed, 9 skipped (all findings)
        - `PLAYA_DRY_RUN=1 just lint` in `claudine/`: exit 0; only the pre-existing `__eh_frame` linker warning
        - mutation check with the reviewer's bypass: the strengthened test fails, 19 of 20 pass (R2-F1.1)
        - `git diff --check`: passed
        - cross-OS: no OS-specific code changed; the CLI file is `cfg(unix)` with `/bin/sh` stubs as before, so WSL2/Linux run it in CI and native Windows does not compile it
