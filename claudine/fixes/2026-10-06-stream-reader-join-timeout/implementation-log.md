---
fix: 2026-10-06-stream-reader-join-timeout
deferred_perf_measurement: false
implementation_1: "2026-10-06"
---

# Implementation Log

## Implementation 1

- read the evidence again before coding. All eleven `session_end` rows with
  `error: "Stream parser thread panicked"` since 2026-09-25 have
  `exit_code: 0`. In ten of them, the same `agent_pid`'s next semantic event
  comes within 1 ms of the `session_end` row: 21 µs and 208 µs for agent_pid
  8848 on 2026-10-06. The exception is a Codex run, at 6.5 s.
    - discovery: the reader in the 2026-10-06 run stalled **before EOF**. Its
      last logged event was line 815 at 08:21:13. Line 816 (the final
      assistant message) was being handled when the join gave up, and line
      817 (`result`) had not been read. A bound keyed on EOF alone, as the
      task first proposed, would still have applied the 5 s cap. The bound
      is therefore keyed on whether the reader is **blocked on its pipe**.
      That is the only state in which a descendant holding the pipe can be
      the cause.
    - discovery: the main thread and the reader resume within microseconds of
      each other. That is consistent with both waiting on the terminal (the
      reader inside `StdoutWriter::write` holding the `StreamOutput` lock,
      the main thread in `emit_stream_summary`). It is not consistent with a
      slow render or a hook action, which would not hold up the main thread.
- added `cli/src/commands/wrap/exec/reader_join.rs`:
    - `JoinOutcome { Joined, Panicked(String), TimedOut(ReaderStall) }`
    - `panic_message`
    - `ReaderProgress` / `TrackedLines`, an atomic that each line pull sets
      to waiting, processing, or EOF
    - `join_reader(handle, progress, budget, since)` and the untracked
      `join_within`
    - `ReaderBudget` with `READER_PIPE_CAP` (5 s) and `READER_DRAIN_LIMIT`
      (120 s)
    - `ParserSlot`, `FallbackParser` (replaces `ErrorParser`), and
      `settle_parser`
- chose 120 s for the drain limit. The longest reader stall in the logs
  is about a minute (2026-10-01: the reader's `turn_complete` came 48.5 s
  after a `session_end` that was itself 5 s after the join started). 120 s
  doubles that. No `TimeoutConfig` knob fits. `kill_grace` is the
  SIGTERM-to-SIGKILL interval, and `step_timeout` measures the child's
  silence (default 30 min), so neither is about the wrapper draining its
  own output. The WHY comment is on the constant.
- `spawn/semantic.rs`:
    - both readers iterate through `ReaderProgress::track`
    - the stdout reader puts its parser in the slot before `drain_close`
    - the joins share one `since` instant, so the combined wait stays within
      one drain limit
    - warnings render as `Status` with `StatusState::Warning` through
      `stream_output.emit_stderr_line`, like the early-termination and
      tool-survivor warnings beside it
- `exec/mod.rs`: `join_with_timeout`/`join_with_timeout_or` (captured and
  inherited paths) now go through `join_within` and log a panic, with its
  payload, separately from a timeout. Their 5 s cap is unchanged; those paths
  were not part of this report.
- F3 in the spec: the parser is recoverable only once every line has been
  fed. A reader stuck inside `feed_line` (the sink renders and writes from
  there) holds the parser, so the 2026-10-06 shape could not keep its summary
  after a timeout. F2 is what fixes that case: it would have joined about 6 s
  after the exit. Sharing parser state any deeper would mean splitting the
  summary accumulation out of every provider parser. That is too invasive for
  this fix.
- tests: seven unit tests in `exec/reader_join/tests.rs`, all with injected
  millisecond budgets, synchronized on the reader's recorded state rather
  than on sleeps.
    - discovery: the panic test first failed as `stream_reader_timeout`. The
      default panic hook's report took longer than the 20 ms test cap while
      the reader still counted as waiting on its pipe. The panic is now raised
      while a line is being handled, as a renderer panic would be.
- docs: added "Waiting for the stream readers after the agent exits" to
  `claudine/docs/topics/timeouts.md`, and pointed the `claudine` skill's
  Timeouts entry at it.
- stall-site investigation (read-only): findings are below. Nothing was
  changed.
- first `just test` run: one failure,
  `a_reader_slow_on_a_line_before_eof_gets_the_drain_limit`, reported
  `TimedOut(PipeOpen)`. Once the slow line was released, the reader's next
  pull marked it as waiting on its pipe for an instant before the source
  returned EOF, and a poll landed in that window. The same race exists in
  production: a pull that returns a buffered line at once still passes
  through "waiting". Fixed in the code, not the test:
    - `ReaderProgress` now records *when* the reader started waiting
    - the pipe cap applies only after `READER_PIPE_SETTLE` (250 ms) of
      continuous blocking, a constant with a WHY comment
    - `ReaderBudget` gained `pipe_settle`
- 25 consecutive runs of the `reader_join` tests passed.
- sabotage proof: with the pipe cap applied regardless of the reader's
  state, four tests fail, the panic, post-EOF, before-EOF, and
  kept-summary tests. Source restored and byte-compared with its backup.
- verification:
    - `just test`, second run: 8373 passed, 9 skipped
    - the run before it stopped on
      `claudine composition::sequence::task::tests::shell_tasks::an_early_wait_error_still_reaps_the_whole_tree`
      (LEAK-FAIL after 32 s under full-suite load)
        - that test is in the `claudine` library, which this fix does not
          touch
        - it passed three times when run alone
        - recorded as a pre-existing flaky test
    - `just lint` first failed on clippy `never_loop` in the panic test. The
      test now uses `if let … .next()`. After that `just lint` passes, and
      the seven `reader_join` tests pass.

### Requirement status after Implementation 1

The spec was rewritten on 2026-10-06 as requirements only (R1-R8).
- Implementation 1 satisfies R1 and R2.
- It satisfies R3 only for a reader that has fed every line. A reader that
  times out mid-line still fails the run.
- R4-R8 are not started.

### Stall-site investigation (read-only)

On the render path, nothing besides a write blocks for seconds once the
process is warm. Per-block Markdown rendering rebuilds the `Terminal` with
full detection (`Terminal::default()` in darkmatter's render tree). Its OSC
color query is cached for the process and bounded at 1 s. The rest of the
detection costs milliseconds. Images and Mermaid are off
(`image_mode = Never`). No network fetch is reachable. Syntax sets load once.

Hook dispatch runs synchronously on the reader thread (`handle.block_on`).
A configured `call` action could hold it for up to 60 s. The user's config
binds none to these events, and a hook stall would not release the main
thread at the same moment. The terminal not accepting output remains the
best explanation. None of these is a clear local defect behind the stall, so
none was changed.
