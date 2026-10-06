---
created: 2026-10-06
status: draft-spec
clarified: false
reviewed: false
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
    clarified: boolean -> indicates whether the specification was built -- _in part_ -- with the 'clarify.md' prompt
    implemented: boolean -> indicates whether this spec's plan has been implemented
    implemented_by: string -> the agent who implemented the plan
area: claudine
packages:
    - claudine-cli
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

## Root cause

1. **The join lied about panics.** After the agent exits,
   `run_child_stream_semantic` (`cli/src/commands/wrap/exec/spawn/semantic.rs`)
   joined the stdout reader with `join_with_timeout_or(handle, 5 s,
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
3. **Both threads waited on the terminal.** The main thread writes the
   trailer through the same `StreamOutput` lock and stderr just before it
   builds the `session_end` row (`emit_stream_summary`, `policy.rs`). Two
   threads released within microseconds of each other point to a shared
   blocked resource: the terminal not accepting output, while the reader
   held the lock inside a stdout write (`StdoutWriter::write`,
   `stream_io.rs`). The reader stalled before its pipe reached EOF. In the
   2026-10-06 run it was handling line 816 (the final assistant message)
   with line 817 still unread.
4. **The slack that hid it went away.** `kill_process_group` used to sleep the
   whole 10 s `kill_grace` before the join. That gave a slow reader time to
   finish. F1 of `2026-09-23-opencode-failure-lifecycle` correctly made the
   teardown return as soon as the group is empty, so only the 5 s join was
   left.

## Requirements

### R1 — A reader failure is reported as what it was

- A reader thread that panics is reported as `parse_failure`, and the message
  includes the panic payload.
- A reader that does not finish in time is reported as a distinct kind,
  `stream_reader_timeout`, never as a panic. Its message names the cause: the
  output pipe is still held open, or output is still being processed.
- Every reader panic or timeout is visible on stderr without enabling
  tracing. This covers the semantic, captured, and inherited spawn paths.

### R2 — The short join cap applies only to a held-open pipe

- The 5 s cap exists for a descendant that keeps the agent's output pipe open.
  It applies only while the reader is blocked reading that pipe, and only
  after a short settle period (250 ms). Without the settle period, a reader
  that is briefly between two buffered lines would count as blocked.
- A reader that is parsing, rendering, or writing gets a much longer limit,
  but a bounded one: 120 s, twice the longest stall in the logs.
- The stdout and stderr readers share one deadline, so the total wait after
  the agent exits stays within one limit.

### R3 — A successful agent run is never failed by a slow reader

If the agent exited 0 and produced a result, a reader timeout must not turn
the run into a failure. The run keeps the agent's exit code and result, and
shows a warning that output may be incomplete. This holds wherever the reader
stalled, including inside the handling of a line, with lines still unread.
Today a reader stuck mid-line holds the only copy of the parser's summary.

### R4 — A terminal that never accepts output cannot hang the wrapper

After the reader join, the main thread still writes the trailer and summary
through the same output lock and stderr. A terminal that has stopped accepting
output blocks those writes indefinitely. The wrapper must finish the run,
run its lifecycle events, and exit within a bounded time even then.

### R5 — The WezTerm stall is explained

The evidence says WezTerm stopped accepting the wrapper's output for up to
about a minute. The cause must be identified and recorded in Claudine's
`docs/`. If Claudine's output contributes to it, that must be fixed. Possible
contributors include output volume, escape sequences, terminal queries, or
the size of a single write.

### R6 — Step-timeout durations always carry a unit

`format_internal_duration` (`exec/timeouts.rs`) prints 60 s to 3599 s as a
bare number: 120 s reads as `2`. Every duration in a step-timeout message must
include its unit.

### R7 — A sequence task's gutter inset reaches rendered Markdown

`new_assistant_stream_inset` sets a fixed width on the terminal it builds.
`FinalMessage::render` ignores that terminal and detects its own width. As a
result, the inset applies only to the plain-text fallback. Rendered Markdown
in a sequence task must honor the same inset.

### R8 — `an_early_wait_error_still_reaps_the_whole_tree` is reliable

`claudine composition::sequence::task::tests::shell_tasks::an_early_wait_error_still_reaps_the_whole_tree`
failed once with LEAK-FAIL after 32 s under full-suite load and passed on its
own three times. It must pass reliably under full-suite load. Either fix the
leak or the test's timing assumption, whichever the investigation shows is
at fault.
