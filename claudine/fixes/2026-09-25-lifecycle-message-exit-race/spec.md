---
created: 2026-09-25
status: draft-spec
clarified: false
reviewed: true
reviewed_by: codex/gpt-6-sol
reviewed_on: 2026-09-27
review_iterations: 0
implemented: false
human_review: false
message_to_agent: |-
    Phase 1 is done; read `implementation-log.md` → Phase 1 before starting.
    - Rules 7, 8, and 10 in `plan.md` were amended (look for "*Amended ...*").
      Rule 7: `track` must warn and return when `Handle::try_current()` fails,
      for all three helpers; today `execute_message` and
      `execute_resolved_message` call bare `tokio::spawn`, which panics on a
      thread with no runtime (parallel sequence-group member threads).
      Rule 8: for `sequence` and provider wrappers, `finish` installs the
      compose ladder (`install_user_interrupt_guard`) during a non-empty drain,
      because on Unix their leftover flag-only / wait-loop SIGINT handlers
      swallow every press. compose/inline-compose hand over their
      `UserInterruptGuard`; `handle` hands over nothing.
    - The reproduction test
      `lifecycle_message_drain::compose_success_message_is_delivered_before_exit`
      carries `#[ignore = "red until the CLI drains deliveries before exit; ..."]`
      so each phase ends green. Phase 3 deletes that attribute only. Run it
      meanwhile with `--run-ignored all`. The control test
      `compose_start_message_reaches_the_listener_during_the_run` must stay green.
    - The loopback fixture is `cli/tests/common/webhook_listener.rs`
      (`WebhookListener`, `ListenerMode`, `write_webhook_route`,
      `apply_route_env`). Phase 2's library embedder test needs its own
      in-process listener; this one lives in the CLI test tree.
    - Phase 4's `sequence` test must use an agent step: sequence shell-task
      stacks are hard-coded to no messaging route (`wrap/sequence/task_run.rs`).
    - On macOS a native desktop notification runs `osascript` synchronously on
      a Tokio worker. `abort()` cannot stop it, but `finish` exits the process
      without waiting for worker threads, so the drain deadline still holds.
    - `just test-cli -E "..."` breaks the `_test` recipe's shell quoting; call
      `cargo nextest run -p claudine-cli --features test-fixtures --test l1 -E '...'`
      directly for filtered runs.
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
    reviewed_by: string -> the agent and model used in the spec review
    reviewed_on: date -> the date the spec was reviewed
    review_iterations: number -> the number of implementation reviews have taken place in the review/fix cycle
    clarified: boolean -> indicates whether the specification was built -- _in part_ -- with the 'clarify.md' prompt
    implemented: boolean -> indicates whether this spec's plan has been implemented
    implemented_by: string -> the agent who implemented the plan
area: claudine
packages:
    - claudine
    - claudine-cli
related:
    - 2026-09-03-tts-not-finishing
    - 2026-09-23-opencode-failure-lifecycle
---

# Outbound messages sent near process exit are silently dropped

## Outcome

Every outbound message and desktop notification that the Claudine CLI starts
either finishes or is reported as unfinished before an ordinary process exit.
An unfinished send may already have reached its destination, so the warning
must not claim it failed or invite an automatic retry. This applies to a
`message` in the last lifecycle event of a run (`success`, `failure`,
`finalize`) and to a hook `message` action under `claudine handle`, including
error exits. Explicit forced exits, crashes, and external termination remain
outside this guarantee.

## Report

`prompts/_reviews/feature-review.md` declares a `message` in its `start`
event and in each branch of its `success` stack. The `start` message arrives
in Discord every time. The `success` message never arrives, and nothing is
printed to say it failed. The other actions in the same stack do fire: the
`warn:` line prints and the `sad-trombone` effect plays, although not every
time.

```text
✓ 256s · 1.4M input tokens · 7K output tokens · 1.3M cached tokens · 27 tool calls
󰀨 fix review 5 of `2026-09-24-ux-improvements` in the worktree package area has
  completed successfully but not production ready: worktree/fixes/2026-09-24-ux-improvements/review-5.md
```

A Discord message was expected after that line and none was sent.

## Cause

This cause comes from reading the code. The plan's first task is to reproduce
it (see R6).

1. **Sends are started in the background and never awaited.** Lifecycle
   `message` goes through [execute_resolved_message](../../lib/src/messaging/send.rs),
   called by [DefaultLifecycleEmitter](../../lib/src/composition/lifecycle/mod.rs)
   in the `claudine` library. Hook `message` actions use
   [execute_message](../../lib/src/messaging/send.rs) from the library's hook
   runner. Desktop notifications use
   [execute_notification](../../lib/src/messaging/send.rs). All three start a Tokio task,
   return at once, and discard the task's handle. Nothing ever awaits those
   tasks.
2. **The affected command paths end with `std::process::exit`.** `compose`,
   `inline-compose`, `sequence`, provider wrappers, `handle`, and the top-level
   error path all call it. `std::process::exit` does not wait for
   runtime tasks. Any send still in flight is killed partway through its
   request.
3. **So a message is delivered only if the run keeps going long enough.** A
   `start` message has the agent's whole run to finish its Discord request.
   A `success`, `failure`, or `finalize` message has only the time between
   the lifecycle stack returning and the exit call. That is far shorter than
   the DNS lookup, TLS handshake, and HTTP round trip a webhook post needs.
4. **The failure report dies with the send.** `report_send_failure` runs
   inside the same task. A send that fails near exit is therefore never
   reported, which is why this failure produces no output at all.

Audio does not have this problem in its primary form. `2026-09-03-tts-not-finishing`
moved speech and effects into a detached spool worker process that outlives
Claudine (`lib/src/composition/lifecycle/audio.rs`, `enqueue_speech` and
`enqueue_effect`). Messaging was never given an equivalent. The sound effect
that plays only sometimes is a separate observation; its cause is not
established here (see Non-goals).

`2026-09-23-opencode-failure-lifecycle` reported that the `failure` event's
`message` never fired. That fix addressed an interrupt path that skipped the
whole lifecycle. With that path fixed, the `failure` message still loses to
the race described here.

## Requirements

### R1. One process-wide registry of in-flight deliveries

`execute_message`, `execute_resolved_message`, and `execute_notification`
register each task they start with a single process-wide tracker owned by
`claudine::messaging`. None of them discards a task handle any more. The
tracker records which route each task belongs to, so an undelivered message
can be named (R3).

The tracker uses the route name for a message and a safe `desktop notification`
label for a notification; it never prints a URL, token, image path, or message
body. Registration and drain coordination must not lose a task that finishes
quickly or starts while draining. Remove completed entries, including failures
and panics, instead of retaining them for a long-running wrapper. Never hold
the registry lock across an await or while rendering a warning. Report a task
panic as a delivery failure without changing the command's exit code.

Any future delivery helper that starts a task must register it with the same
tracker. The module docs state this, and R6's guard enforces it.

### R2. Drain before every normal exit

The CLI gets one ordinary shutdown path. Command paths return their exit code
to the async dispatch layer, which drains deliveries (R3) while the Tokio
runtime is still running, flushes stdout and stderr, and exits with the
original code. Top-level errors must be rendered and drained before `run()`
drops its runtime. Do not create another runtime or synchronously block a
Tokio worker to perform the drain. `claudine handle` keeps its machine-readable
response complete and flushed before exit. Every affected call site in Cause
item 2 uses this path, including errors that currently return through `?`.

Direct `std::process::exit` / `_exit` / `ExitProcess` calls remain only at
these sites:

- the centralized ordinary-exit path, after it has drained and flushed;
- the audio worker path in `main.rs` (`run_audio_worker_if_requested`), which
  never starts a delivery;
- the forced-exit paths in `compose/interrupt.rs`, where the user has
  explicitly asked to stop now (R4);
- the generated child-program source in `wrap/exec/termination/windows.rs`,
  which is test fixture text, not CLI code.

Clap's help and parse-error exits occur before any delivery can start and stay
outside the drain; document them in the allowlist. The `claudine` library's
public send helpers still return immediately when embedded in another program.
Expose a drain operation for embedding applications to opt in; the CLI owns
the ordinary-exit guarantee.

A guard test fails if a direct exit call appears anywhere else in
`claudine/cli/src`. It follows the pattern of `spawn_site_guard.rs` and uses
an exact allowlist.

### R3. The wait is bounded, and running out of time is reported

- The drain waits at most **10 seconds in total**, however many deliveries
  are pending. A healthy webhook post finishes in well under a second. The
  cap exists only so an unreachable route cannot hang a finished run. The
  value is a named constant with no configuration surface.
- Under `claudine handle`, start the existing overall deadline before
  `run_inner` and apply its remaining time, capped at 10 seconds, to the
  drain. If handler work reaches that deadline, retain exit code `124` and
  report pending deliveries without another wait. The drain must not push a
  hook handler past the deadline that protects the agent session.
- Any delivery still pending when time runs out is reported. A `Warning`
  status on stderr names each unfinished route or the desktop notification
  label and states that delivery is unknown. Cancel and remove unfinished
  tasks before leaving the runtime. The exit code does not change: a failed
  delivery never turns a successful run into a failure.
- When nothing is pending, the drain returns immediately. A run that sends
  no messages exits no later than it does today.

### R4. A forced exit skips the wait

A second Ctrl+C still wins. Keep the compose interrupt registration alive
through the drain, or transfer its equivalent to shutdown; today it drops
when the command returns. That covers the `PressRung::GraceExit` grace
deadline and the immediate `_exit(130)` rungs from
`2026-09-23-opencode-failure-lifecycle`. If the user presses Ctrl+C again
during the drain, the process exits through the existing forced-exit path
without waiting further. A drain in progress must never make interrupt
handling slower than it is today.

### R5. Failures that finish during the drain are reported

A send that fails while the drain is waiting reports through the existing
`report_send_failure` / `report_notification_failure` path, with the same
redaction. Because the process now outlives the task, that stderr line
appears. This requirement adds no new code; it exists so a test pins the
behavior.

### R6. Verification

- **Reproduce first.** Before changing behavior, add an L1 CLI test that
  runs `claudine compose` against a stub `claude` provider (an executable
  that prints one line and exits `0`). The document declares a `success`
  `message`, sent over an environment-backed `discord_webhook` route whose
  test-only URL points at a local HTTP listener. Claudine's inline URL
  validator requires a production Discord host, while
  [parse_webhook_url](../../../messenger/lib/src/provider/discord_webhook.rs)
  accepts a loopback HTTP URL with the usual `/webhooks/{id}/{token}` path.
  Use the environment-backed route to preserve production validation, and
  use a dummy token because errors can include the loopback URL. Keep the
  listener from completing its reply after it receives the request. On the
  old code, assert the child exits while the reply is withheld; on the fixed
  code, assert the child remains running, release the reply, then assert it
  exits. If the old child exits before a request arrives, that also reproduces
  the loss. Record the observed failure in the implementation log. Do not
  assert that no request reached the listener; the old task may send a request
  before it is killed. Give both child and listener a watchdog timeout.
- The same test on the fixed code: the listener receives exactly one POST
  whose body contains the rendered `success` text, before the process exits.
- A listener that answers `400` causes the "Failed to send lifecycle message"
  status line to appear on stderr, and the exit code stays `0`.
- A listener that accepts the connection but never replies makes the run
  exit within the total 10-second drain budget, prints the R3 warning, and
  keeps exit code `0`. Mark this a `slow_` test if it needs the full budget;
  avoid adding a production override solely for testing. Verify that multiple
  pending deliveries share one budget and that the warning does not contain
  the dummy token or message body.
- `claudine handle` with a hook `message` action: the listener receives the
  POST before the handler exits. A stalled send consumes only the remainder
  of the handler's overall deadline, and a timed-out handler still exits `124`.
- A `sequence` whose last step's `success` event sends a message: the
  listener receives it.
- A terminal `notify` action is tracked and drained. A failed desktop
  notification uses its existing warning; an unfinished one gets R3's safe
  label. Keep the test silent and independent of the host's notification UI.
- Verify a top-level error after a send still drains, and a second Ctrl+C
  during the drain takes the existing forced-exit path on macOS, Linux, and
  Windows. These child-process tests must not give a terminal or browser
  window focus.
- The R2 exit-site guard, and a guard that every task started in
  `messaging/send.rs` goes through the tracker (R1).

### R7. Documentation

- `lib/src/messaging/send.rs`: the module docs and the `execute_message`,
  `execute_resolved_message`, and `execute_notification` docs currently say
  "fire-and-forget". Rewrite them to say sends are tracked and drained
  before exit (R1–R3).
- `docs/topics/flow-control/lifecycle.md`: add a sentence to the communication-properties
  section (near the `message` row) stating that a message sent from a
  terminal event finishes or is reported as unfinished before an ordinary
  process exit, with a 10-second limit. State that a timeout leaves delivery
  uncertain.
- `docs/topics/messaging.md`: explain the CLI drain, the handler's shared
  overall deadline, the unchanged exit code, and the library caller's
  opt-in drain contract. Also correct its claim that hook messages already
  have a dedicated three-second timeout; the present hook runner starts a
  send task without such a timeout.
- The `claudine` skill, if it describes messaging as fire-and-forget.

## Design Notes

**Why drain at exit instead of awaiting each send in place.** Awaiting at
emit time is the smaller change, but it has two costs. A `start` message
would delay the agent's launch by a webhook round trip. A slow route would
also stall every lifecycle stack and every hook action, and hooks sit on the
agent's latency path. Draining at exit keeps sends running alongside the
work and pays the wait once, only when something is still pending.

**Why not a detached worker like audio.** The audio spool worker exists
because playback lasts seconds past exit and must survive it. A webhook post
is a sub-second request whose failure the user should see in this terminal.
Moving it to a detached process would mean passing route secrets (webhook
URLs are bearer credentials) to a child process, and failures would land
where nobody reads them. A bounded in-process drain gives the delivery
guarantee without either cost.

## Non-goals

- Discord content truncation (`2026-09-06-truncation` in messenger). It is
  still needed separately: once this fix lands, a message over 2,000
  characters, such as `feature-review.md`'s "not production ready" findings
  list, will fail with a visible error instead of vanishing.
- The occasional missing `sad-trombone` effect. It belongs to the audio
  spool handoff. If R6's reproduction work shows it shares this cause, file
  a separate fix.
- Retrying failed sends, or storing messages durably across a crash.
