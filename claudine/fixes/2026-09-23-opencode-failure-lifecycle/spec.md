---
created: 2026-09-23
status: implemented
clarified: false
reviewed: false
implemented: true
implemented_by: claude/opus-5.5
$schema:
    status: |-
        enum(
            draft-spec,
            finalized-spec,
            planned,
            implemented,
            review-findings,
            human-in-the-loop,
            completed,
            on-hold,
            abandoned
        ) -> an indicator of progress for this specification
    reviewed: boolean -> indicates whether the specification file has been reviewed by another agent from the one which created the spec
    clarified: boolean -> indicates whether the specification was built -- _in part_ -- with the 'clarify.md' prompt
    implemented: boolean -> indicates whether this spec's plan has been implemented
    implemented_by: string -> the agent who implemented the plan
area: claudine
packages:
    - claudine
    - claudine-cli
---

# The `failure` lifecycle is skipped when an OpenCode run is interrupted

## Report

A composed prompt run through OpenCode failed and its `failure` lifecycle
event (the `say`, `sad-trombone` effect, and messaging `message` of
`prompts/_implement/implement-plan.md`) never fired. The operator's output:

```text
󰀨 ProviderModelNotFoundError: Model not found: kimi-for-coding/k3. (ref err_16b12165)
┃ API Error
┃ Unexpected server error. Check server logs for details.
^C
 User interrupted compose operation in prompts/implement.md^C
⚠ second interrupt — force-exiting compose
```

A second report from the same evening (a `usage_limit_reached` failure on
iteration 3 of the same prompt) was investigated alongside it. It is **not**
part of this defect: Claudine's JSONL log records that session ending at
`01:52:29.009Z`, and Playa's journal shows the `failure` event's sad-trombone
and speech queued 7 ms later and played to completion. That event fired; its
only terminal-visible surface is the messaging route.

## Root cause

One chain, exposed by the OpenCode 1.18.x upgrade installed 2026-09-21.

1. **OpenCode leaves a process in its group.** Run on its own, OpenCode
   1.18.32 exits 1 in under a second for an unknown model. Through Claudine
   the same run took 10.9 s.
2. **The post-exit teardown slept blind.** `kill_process_group`
   (`cli/src/commands/wrap/exec/mod.rs`) sent `SIGTERM` to the child's process
   group and then slept the whole `kill_grace` (10 s) whenever the signal
   reached *any* member, even one that died at once. A stack sample during the
   dead time showed `run_child_stream_semantic → thread::sleep` with no
   OpenCode process alive. The run looked hung.
3. **Ctrl+C in that window could only force-exit.** No child wait loop is
   active during the teardown, so the compose-scoped SIGINT guard treated a
   repeat press as "wedged, no ladder to defer to" and called `_exit(130)`,
   skipping `failure` and `finalize`. The operator's first press printed only
   the notice (no `press again to escalate`, which a wait loop would print);
   the second force-exited.

A third, separate symptom of the same upgrade: OpenCode 1.18 no longer puts
the concrete cause on stdout. The NDJSON carries only
`{"type":"error","error":{"name":"UnknownError","data":{"message":"Unexpected server error. Check server logs for details.","ref":"err_…"}}}`,
and the cause appears only in the stderr record
`level=ERROR message=failed ref=err_… error="ProviderModelNotFoundError: …"`.
The stdout parser's existing rule (keep a concrete error over a later generic
one) had nothing concrete to keep, so `err.msg` read "Unexpected server error".

Pre-launch model validation is intentionally not part of the fix: the
2026-07-06 Phase F ruling makes the provider the authority on model ids and the
catalog a drift signal only.

## Fixes

### F1 — Teardown ends when the group is empty and yields to Ctrl+C

`kill_process_group` polls the group (`kill(-pgid, 0)`) every 50 ms after the
`SIGTERM` and returns as soon as it is empty; a user interrupt cuts the grace
short to `SIGKILL`. For its whole duration it holds `WaitLoopActiveGuard`, so a
repeat press defers instead of force-exiting past the lifecycle tail.

### F2 — A repeat Ctrl+C during a terminal lifecycle event gets 500 ms

`claudine::interrupt::TerminalLifecycleScope` marks `success`, `blocked`,
`failure`, and `finalize` while they run (entered in
`LifecycleRunGuard::run_event_stack`, the single execution choke point). A
repeat press in that window takes the new `PressRung::GraceExit`: it announces
the grace and arms a `TERMINAL_LIFECYCLE_EXIT_GRACE` (500 ms) deadline; a run
that finishes sooner exits normally, otherwise the wrapper force-exits with
130. On Unix the handler writes one byte to a process-lifetime self-pipe and a
watcher thread enforces the deadline; on Windows the console-handler thread
waits itself. Later presses do not extend it.

The deadline exists only after a second press. Without Ctrl+C, and after a
single press, lifecycle events run for as long as they need.

### F3 — A generic OpenCode error takes its cause from the matching stderr record

The stdout parser records `error.data.ref` as
`StreamExecutionSummary::error_reference`. The stderr bridge records each
backstopped `ERROR` record's headline by its `ref`
(`SharedStderrState::failure_causes`). `merge_stderr_state_into_summary`
replaces a generic server error with the cause sharing its `ref` and sets
`error_kind` to the JS error class, so lifecycle `err.msg` reads
`ProviderModelNotFoundError: Model not found: kimi-for-coding/k3.` and
`err.variant` reads `ProviderModelNotFoundError`.

## Verification

| Check | Result |
|---|---|
| Real OpenCode 1.18.32, unknown model, installed build | 10.9 s, `err.msg` generic |
| Same, fixed build | 1.2 s, `err.msg`/`err.variant` name the cause |
| `compose_sigint_during_orphan_teardown_runs_failure_lifecycle` | passes; hangs to the nextest timeout on the pre-F1 code |
| `compose_second_sigint_lets_a_short_failure_stack_finish` | passes; fails with the grace rung disabled |
| `compose_second_sigint_force_exits_a_failure_stack_that_outlasts_the_grace` | passes (exit 130 at ~0.5 s); fails with the grace rung disabled |
| `opencode_unknown_model_surfaces_its_cause_as_the_failure_error` | passes |
| Lib: `ref` join (matching and unrelated `ref`), parser `ref` extraction | pass |

## Follow-ups

- `UserInterruptGuard` never unregisters its Unix `signal_hook` handler
  (`SigId` has no `Drop`). Its doc comment claimed otherwise and was corrected;
  whether the handler *should* be unregistered is a separate question.
- Only the stderr backstop's headline is joined. A future OpenCode that drops
  `ref` from either surface falls back to the generic message.
