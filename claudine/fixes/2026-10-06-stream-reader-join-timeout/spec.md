---
created: 2026-10-06
clarified: false
reviewed: true
reviewed_by: codex/gpt-6.1-sol
reviewed_on: 2026-10-06
review_iterations: 0
completed: false
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
    - claudine-cli
human_review: false
message_to_agent: |-
    Phase 1 is complete (rulings, baseline, spike). Key context for the next phases:

    1. Rulings A-G were adopted as the plan's documented defaults because the
       session was non-interactive and the author could not be consulted. They
       are recorded verbatim in the plan's "Implementation log" section. Phase
       4's design depends on ruling A; if the author rejects it at review,
       Phase 4 must be re-planned.
    2. Baseline from the worktree's claudine/: `just lint` exit 0, `just test`
       8373 passed / 9 skipped / 0 failed. No known-red tests.
    3. HOST HAZARD: this host exports CDPATH containing the main checkout. A
       relative `cd claudine` from the worktree root jumps to the MAIN
       checkout, not the worktree. Always `cd` with an absolute path or a
       `./`-prefixed path, and verify with `pwd`. (Already recorded in the os
       skill's macos.md.)
    4. Spike (R5) confirmed the incident shape live: reader blocked in
       write(2) inside StdoutWriter::write from the OutputTextCallback inside
       feed_line, holding the parser mid-line; main thread blocked after the
       120 s join timeout on __psynch_mutexwait acquiring the StreamOutput
       lock in emit_stderr_line — the wrapper hangs forever (R4 gap confirmed,
       not just theoretical).
    5. Spike found a confirmed Claudine-side contributor, and a fix task was
       appended to Phase 4's "Spike follow-up": per-block Markdown rendering
       rebuilds Terminal::default() (darkmatter terminal_options_from_terminal_options),
       and biscuit_terminal::discovery::fonts::font_name is UNCACHED — its
       fallback_font_name_scan -> query_iterm2_font_name spawns a `defaults
       read com.googlecode.iterm2` subprocess per Markdown block on the reader
       thread. 889/2217 reader samples were in this path. Cache/hoist the
       detection.
    6. Spike artifacts (sample outputs, fake agent, pane transcripts) are under
       /tmp/claudine-r5-spike/ if a later phase wants the raw evidence.
    7. Phase 1 changed no source files, docs, or skills; only the fix's
       plan.md, implementation-log.md, and spec.md frontmatter.
---

# A successful agent run is reported as a stream parser panic

## Report

A `claudine compose` loop iteration whose Claude agent exited 0 sometimes
ended with:

```text
 Stream parser thread panicked
 CompositionError: parse_failure
┃ Iteration 6 exited with code 1.
```

The terminal was WezTerm, run directly with no multiplexer. Nothing had
panicked.

## Incident evidence and likely cause

1. **The join lied about panics.** After the agent exits,
   the claudine-cli package's
   [`run_child_stream_semantic`](../../../cli/src/commands/wrap/exec/spawn/semantic.rs),
   which runs a provider with structured output, joined the stdout reader with `join_with_timeout_or(handle, 5 s,
   ErrorParser)`. That returned the fallback both for a real panic (whose
   payload `unwrap_or` discarded) and for a timeout (which it only reported
   through `tracing::warn!`, off by default). `ErrorParser` always reported
   `parse_failure` / "Stream parser thread panicked".
2. **The reader was slow, not dead.** `~/.claudine/logs/2026-10-06.jsonl` row
   6845 (agent_pid 8848) is the `session_end` with `exit_code: 0`,
   `exit_reason: parse_failure`. The same reader then logged the run's final
   `output_text` and `turn_complete` (stream line 817, the `result`) 21 µs and
   208 µs later. The reader had stopped after line 815, at 08:21:13, and
   resumed only after the join had already timed out. In ten of the eleven
   such failures since 2026-09-25, the reader's events resume within 1 ms of
   the `session_end` row (one Codex run: 6.5 s).
3. **Terminal output is the leading hypothesis.** The main thread writes the
   trailer through the same `StreamOutput` lock and stderr just before it
   builds the `session_end` row (`emit_stream_summary`, `policy.rs`). Two
   threads released within microseconds of each other suggest a shared
   blocked resource, potentially the terminal not accepting output while
   the reader held the lock inside a stdout write (`StdoutWriter::write`,
   `stream_io.rs`). The reader stalled before its pipe reached EOF. In the
   2026-10-06 run it was handling line 816 (the final assistant message)
   with line 817 still unread.
4. **The slack that hid it went away.** `kill_process_group` used to sleep the
   whole 10 s `kill_grace` before the join. That gave a slow reader time to
   finish. The process-group teardown change in
   `2026-09-23-opencode-failure-lifecycle` correctly made the
   teardown return as soon as the group is empty, so only the 5 s join was
   left.

## Current implementation and scope

The incident describes the earlier implementation. The current checkout already
has typed join outcomes, reader-state tracking, the 5 s / 120 s bounds, and a
parser handoff before final rendering in the claudine-cli package's
[reader join helpers](../../../cli/src/commands/wrap/exec/reader_join.rs).
That handoff protects a stall after parsing every line; it does not protect a
stall inside a parser callback before the final result line is read. Captured
and inherited runs still use fallback-only joins. These are remaining gaps,
not reasons to replace the completed work.

The claudine library is also in scope: it owns Markdown rendering and the
sequence shell-task regression. Keep provider parsing, result semantics, and
rendering behavior in the library; the CLI owns pipe/thread coordination and
terminal delivery. This fix adds no public timeout flag or environment variable.
The post-exit wait is an internal cleanup budget, separate from the two
provider timeout rules documented in [Timeouts](../../../docs/topics/timeouts.md).

## Requirements

### R1 — A reader failure is reported as what it was

- A reader thread that panics is reported as `parse_failure`, and the message
  includes a string panic payload, or an explicit non-string-payload fallback.
- A reader that does not finish in time is reported as a distinct kind,
  `stream_reader_timeout`, never as a panic. Its message names the cause: the
  observed state: waiting for pipe data, or processing output. A descendant
  holding the pipe and a terminal refusing output are possible explanations,
  not causes that reader-state tracking proves.
- Every reader panic or timeout is visible on stderr without enabling
  tracing when stderr accepts output. This covers the semantic, captured, and
  inherited spawn paths. If stderr is blocked, bounded shutdown takes priority
  over visible delivery; retain the diagnostic in the session record where
  that record exists. See Open Questions for the delivery design.

### R2 — The short join cap applies only to a held-open pipe

- The 5 s cap exists for a descendant that keeps the agent's output pipe open.
  It applies only while the reader is blocked reading that pipe, and only
  after a continuous read wait of at least 250 ms. This is a heuristic, not
  proof that no buffered bytes remain. Without the settle period, a reader
  that is briefly between two buffered lines would count as blocked.
- A reader that is parsing, rendering, or writing gets a much longer limit,
  but a bounded one: 120 s, twice the longest stall in the logs.
- The stdout and stderr readers share one deadline, so the total wait after
  reader cleanup begins stays within one limit. Start the shared monotonic
  clock after process-tree teardown and before joins; neither reader nor a
  change of state resets it. At or after 5 s, a continuous 250 ms read wait
  permits the short cutoff; 120 s is the absolute ceiling for every state.
  Observe completion before declaring timeout at a boundary. Process-tree
  teardown has its existing separate budget.

### R3 — A successful agent run is never failed by a slow reader

If the agent exited 0 and produced a result, a reader timeout must not turn
the run into a failure. The run keeps the agent's exit code and result, and
shows a warning that output may be incomplete. This holds wherever the reader
stalled, including inside the handling of a line, with lines still unread.
The current reader can hold the only parser while a rendering callback blocks.

**Design decision:** terminal delivery must not block provider parsing. Publish
semantic state before submitting rendered output, and retain a result snapshot
that can be read without taking a lock held across terminal I/O. Queue capacity
must be bounded; a full rendering queue must not block parsing or grow memory
without limit. Discarding terminal presentation may be necessary after the
output budget expires; report that loss. This does not authorize dropping
provider bytes or silently truncating the result.

Only a parsed successful provider result plus child exit 0 qualifies for
success preservation. Do not invent a result from unread lines. If no complete
result is available at cutoff, record `stream_reader_timeout` as an incomplete
stream failure. Preserve an already established provider failure, nonzero exit,
signal interruption, or provider timeout; a reader warning must not replace its
primary cause. A real parser panic remains `parse_failure`. A stderr-reader
failure alone remains a warning unless its provider-specific bridge is needed
to determine the outcome.

Freeze the summary and publish exactly one `session_end` before lifecycle
completion. After cutoff, readers must not mutate that summary, emit late
semantic events for the completed run, or write into the next loop iteration.
Captured runs must preserve bytes already collected and flag incomplete capture;
inherited runs must distinguish failed forwarding from a provider failure.

### R4 — A terminal that never accepts output cannot hang the wrapper

After the reader join, the main thread still writes the trailer and summary
through the same output lock and stderr. A terminal that has stopped accepting
output blocks those writes indefinitely. The wrapper must finish the run,
run its lifecycle events, and finish output cleanup within a bounded time even
then. This includes ticker shutdown, warnings, trailers, summary rendering,
output-lock acquisition, and panic reporting, not just the reader joins.
Lifecycle actions keep their existing independent deadlines; the 120 s output
budget is not a new deadline for arbitrary lifecycle actions or messaging drain.
Do not bypass the CLI's ordinary shutdown path to force an exit.

A detached blocked writer must be isolated from future iterations: merely
dropping its thread handle does not stop it or release its locks. Do not wait
indefinitely for that writer, allow an unbounded new writer per iteration, or
hold run-state locks during writes. The concrete delivery and abandonment
strategy requires the author decision in Open Questions.

### R5 — The WezTerm stall is explained

The logs show a reader stall of up to about a minute during a WezTerm run;
they do not establish that WezTerm itself caused it. Inspect the blocked read,
render, lock, and write boundaries on one host with a quick targeted sample.
Record confirmed observations, the leading explanation, and any unresolved
cause in Claudine's `docs/`. Fix a confirmed Claudine contribution; an
unreproduced terminal defect must not prevent the bounded-cleanup fix. Possible
contributors include output volume, escape sequences, terminal queries, or
the size of a single write.

### R6 — Step-timeout durations always carry a unit

The claudine-cli package's
[`format_internal_duration`](../../../cli/src/commands/wrap/exec/timeouts.rs),
which formats timeout diagnostic durations, prints 60 s to 3599 s as a
bare number: 120 s reads as `2`. Every duration in a step-timeout message must
include its unit.

### R7 — A sequence task's gutter inset reaches rendered Markdown

The claudine-cli package's
[`new_assistant_stream_inset`](../../../cli/src/commands/wrap/exec/mod.rs)
constructs the narrower terminal used for sequence-task output. The claudine
library's [`FinalMessage::render`](../../../lib/src/render/final_message.rs),
which renders the completed assistant message, uses independently detected
Markdown options rather than consistently honoring the supplied terminal. As a
result, the inset applies only to the plain-text fallback. Rendered Markdown
in a sequence task must honor the same inset.

### R8 — `an_early_wait_error_still_reaps_the_whole_tree` is reliable

The claudine library's
[`an_early_wait_error_still_reaps_the_whole_tree`](../../../lib/src/composition/sequence/task/tests.rs)
checks that an injected shell wait failure still cleans up descendants. It
failed once with LEAK-FAIL after 32 s under full-suite load and passed on its
own three times. It must pass reliably under full-suite load. Either fix the
leak or the test's timing assumption, whichever the investigation shows is
at fault.


## Verification and documentation

Use short injected budgets and synchronization signals for reader tests rather
than waiting 120 s or relying on sleeps to establish a reader's state. Cover:

- A real panic, including a non-string payload, versus each timeout state.
- A slow line followed by unread final-result data; parsing still produces the
  complete successful result while terminal delivery is blocked.
- A held-open pipe, an incomplete result, and a shared stdout/stderr deadline.
- A result indicating failure despite exit 0, a nonzero child exit, interruption,
  and provider timeout; none becomes success because rendering timed out.
- A permanently blocked output sink; warnings, tickers, trailer, and summary do
  not prevent completion. Release the controlled sink during test teardown and
  check that no late event or output enters a subsequent iteration.
- Semantic, captured, and inherited paths; partial captured bytes remain
  available and normal unblocked output retains its existing order and content.
- Duration boundaries at 59 s, 60 s, 120 s, 3599 s, and 3600 s, with units.
- Markdown prose and code blocks inside a sequence gutter, including narrow
  terminals, cached rendering options, and plain output.

Keep terminal output on the existing biscuit-terminal component path, including
capability fallbacks and `NO_COLOR`. Warnings belong on stderr; any supported
JSON stdout mode must retain its existing valid JSON contract.

Run focused tests followed by the relevant package-area L1 suite. Investigate
the shell cleanup failure under full-suite load without weakening the descendant
reap assertion or hiding failures with retries. Prefer a descendant-ready
handshake if fixture startup timing is responsible. Portable synthetic reader
and sink tests must work on macOS, Linux, native Windows, and WSL2; existing
Unix-only process-tree fixtures retain their honest platform scope. Real
WezTerm observation is terminal integration evidence, not a portable unit test.
No additional CI cells or performance benchmark campaign are required here.

Update the timeout, signal-handling, and composition topic pages and affected
README descriptions alongside implementation. Correct the current timeout
page's claim that a bounded join alone prevents a blocked terminal from hanging
the wrapper. Record implementation departures in the implementation log rather
than rewriting this snapshot after the review closes.

## Open Questions

### How should Claudine stop delivery to a terminal that never drains?

A synchronous write can remain blocked after the reader budget expires. The
wrapper needs a delivery strategy that preserves parsing, bounds queued memory,
and prevents an old writer from corrupting a later loop iteration. Select this
before implementing the output separation; the choice affects output-loss and
shutdown guarantees.

1. **One bounded output worker per invocation, with delivery disabled after a
   stall (recommended).** Parsing and lifecycle state never depend on this
   worker. On cutoff, reject later terminal frames for the invocation and retain
   the incomplete-output diagnostic in session records. Pros: bounded thread
   count and memory, small change to coordination, no old writer competing with
   a new iteration's writer. Cons: one in-progress write may finish late, and
   later iterations must stay silent on the affected sink. The author must
   decide whether that loss of terminal visibility is acceptable.
2. **Platform-specific cancellable or nonblocking terminal delivery.** Pros:
   can recover delivery and stop outstanding writes where the OS supports it.
   Cons: substantially larger macOS/Linux/Windows implementation and testing
   burden; cancellation semantics differ, and partial escape sequences need
   handling. Choose this only if continued visible loop output is required.
3. **Isolate terminal delivery in a helper process.** Pros: a stuck writer can
   be terminated without leaving a blocked thread inside the wrapper. Cons:
   extra process ownership, framing, startup, and shutdown complexity; queued
   and partially written output can still be lost.

The first option best matches the immediate goal of reliable result reporting
with a small, bounded repair. It deliberately sacrifices presentation after a
permanent stall. Approval of that tradeoff is needed before treating it as the
implemented contract; a fresh detached writer per run is not an acceptable
substitute.
