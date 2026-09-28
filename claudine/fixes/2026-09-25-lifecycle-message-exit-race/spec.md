---
created: 2026-09-25
status: draft-spec
clarified: false
reviewed: false
review_iterations: 0
implemented: false
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

Every outbound message and desktop notification that Claudine starts is
delivered or reported as failed before the process exits. None disappear
without a trace. That holds for a `message` in the last lifecycle event of a
run (`success`, `failure`, `finalize`) and for a hook `message` action under
`claudine handle`. It also holds when the process exits with an error.

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
   `message` goes through `execute_resolved_message`
   (`lib/src/messaging/send.rs:178`), called from
   `DefaultLifecycleEmitter::emit_message`
   (`lib/src/composition/lifecycle/mod.rs:436`). Hook `message` actions go
   through `execute_message` (`send.rs:128`, called from
   `lib/src/dispatch/runner/mod.rs:373`). Desktop notifications go through
   `execute_notification` (`send.rs:304`). All three start a Tokio task,
   return at once, and discard the task's handle. Nothing ever awaits those
   tasks.
2. **Every command ends with `std::process::exit`.** `compose` and
   `inline-compose` (`cli/src/commands/compose/mod.rs:648`, `:662`),
   `sequence` (`sequence.rs:70`), `wrap` (`wrap/mod.rs:245`, `:441`),
   `handle` (`handle.rs:95`, `:109`), and the top-level error path
   (`main.rs:198`) all end this way. `std::process::exit` does not wait for
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
that plays only sometimes is a separate issue. It may be a race when handing
off to the worker, and it is out of scope here (see Non-goals).

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

Any future delivery helper that starts a task must register it with the same
tracker. The module docs state this, and R6's guard enforces it.

### R2. Drain before every normal exit

The CLI gets one exit function, used everywhere a command ends. It waits for
in-flight deliveries (R3), flushes stdout and stderr, and then calls
`std::process::exit`. Every call site listed in Cause item 2 switches to it.

Direct `std::process::exit` / `_exit` / `ExitProcess` calls remain only at
these sites:

- the audio worker path in `main.rs` (`run_audio_worker_if_requested`), which
  never starts a delivery;
- the forced-exit paths in `compose/interrupt.rs`, where the user has
  explicitly asked to stop now (R4);
- the generated child-program source in `wrap/exec/termination/windows.rs`,
  which is test fixture text, not CLI code.

A guard test fails if a direct exit call appears anywhere else in
`claudine/cli/src`. It follows the pattern of `spawn_site_guard.rs` and uses
an exact allowlist.

### R3. The wait is bounded, and running out of time is reported

- The drain waits at most **10 seconds in total**, however many deliveries
  are pending. A healthy webhook post finishes in well under a second. The
  cap exists only so an unreachable route cannot hang a finished run. The
  value is a named constant with no configuration surface.
- Under `claudine handle`, the drain uses whatever remains of the handler's
  existing deadline (`resolve_deadline`), capped at the 10-second limit. The
  drain must not push a hook handler past the deadline that protects the
  agent session.
- Any delivery still pending when time runs out is reported. A `Warning`
  status on stderr names each unfinished route and states that its message
  may not have been delivered. The exit code does not change: a failed
  notification never turns a successful run into a failure.
- When nothing is pending, the drain returns immediately. A run that sends
  no messages exits no later than it does today.

### R4. A forced exit skips the wait

A second Ctrl+C still wins. That covers the `PressRung::GraceExit` grace
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
  `message`, sent over a `discord_webhook` route whose URL points at a local
  HTTP listener. `parse_webhook_url` in
  `messenger/lib/src/provider/discord_webhook.rs` does not pin the host.
  Confirm that it also accepts an `http://127.0.0.1:<port>/api/v10/webhooks/1/tok`
  URL. If it rejects plain HTTP or a loopback host, stop and raise that as
  an open question rather than widening the parser. On today's code the
  listener must receive no request. Record that failing result in the plan.
- The same test on the fixed code: the listener receives exactly one POST
  whose body contains the rendered `success` text, before the process exits.
- A listener that answers `400` causes the "Failed to send lifecycle message"
  status line to appear on stderr, and the exit code stays `0`.
- A listener that accepts the connection but never replies makes the run
  exit after about 10 seconds, prints the R3 warning, and keeps exit code
  `0`. This test is a `slow_` test if it needs the full budget. The plan may
  instead add a test-only override for the constant, but must not add a
  user-facing one.
- `claudine handle` with a hook `message` action: the listener receives the
  POST before the handler exits.
- A `sequence` whose last step's `success` event sends a message: the
  listener receives it.
- The R2 exit-site guard, and a guard that every task started in
  `messaging/send.rs` goes through the tracker (R1).

### R7. Documentation

- `lib/src/messaging/send.rs`: the module docs and the `execute_message`,
  `execute_resolved_message`, and `execute_notification` docs currently say
  "fire-and-forget". Rewrite them to say sends are tracked and drained
  before exit (R1–R3).
- `docs/topics/flow-control/lifecycle.md`: add a sentence to the communication-properties
  section (near the `message` row) stating that a message sent from a
  terminal event is delivered, or reported as failed, before the process
  exits, with a 10-second limit.
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
