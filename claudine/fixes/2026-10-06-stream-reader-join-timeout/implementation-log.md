---
fix: 2026-10-06-stream-reader-join-timeout
deferred_perf_measurement: false
implementation_1: "2026-10-06"
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
    - claudine/cli/src/commands/wrap/exec/timeouts.rs
    - claudine/cli/src/commands/wrap/exec/timeouts/tests.rs
    - claudine/lib/src/render/final_message.rs
    - claudine/lib/src/composition/sequence/task/tests.rs
    - claudine/lib/tests/l1/assistant_stream_width.rs
    - claudine/lib/tests/l1/main.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
    - claudine/cli/src/commands/wrap/exec/reader_join.rs
    - claudine/cli/src/commands/wrap/exec/reader_join/tests.rs
    - claudine/cli/src/commands/wrap/exec/mod.rs
    - claudine/cli/src/commands/wrap/exec/spawn/captured.rs
    - claudine/cli/src/commands/wrap/exec/spawn/inherited.rs
    - claudine/cli/src/commands/wrap/exec/spawn/semantic.rs
    - claudine/cli/src/commands/wrap/exec/spawn/tests/captured.rs
    - claudine/cli/src/commands/wrap/exec/spawn/tests/inherited.rs
    - claudine/cli/src/commands/wrap/exec/wiring/session.rs
docs_updated_during_phase_3:
    - claudine/docs/topics/timeouts.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
packages:
    - claudine
    - claudine-cli
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

## Phase 1
Phase 1 is rulings, baseline, and the R5 spike. No source files, docs, or
skill files were changed; the phase touched only this fix's `plan.md`,
`implementation-log.md`, and `spec.md`.

### Rulings

- the session is non-interactive, so rulings A to G were adopted as the
  plan's documented defaults (the recommendation each ruling names) and
  recorded verbatim in the plan's Implementation log section for the author
  to confirm or reject at review. A rejection of ruling A reopens Phase 4's
  design; this is called out in `message_to_agent` on the spec.

### Baseline

- first attempt ran `just test` / `just lint` against the **main checkout**
  by accident: this host exports a `CDPATH` containing
  `/Users/ken/coding/personal/rusty-biscuit`, so a relative `cd claudine`
  from the worktree root jumps to the main checkout (a trap already recorded
  in the `os` skill's `macos.md`, rediscovered here the hard way). All later
  commands use absolute paths or `./`-relative paths, which bypass CDPATH.
- corrected baseline, from the worktree's `claudine/`:
    - `just lint` — exit 0
    - `just test` — 8373 passed, 9 skipped, 0 failures (the accidental
      main-checkout run had shown 8366; the difference is Implementation 1's
      seven `reader_join` tests)
- no known-red list; the R8 flake
  (`an_early_wait_error_still_reaps_the_whole_tree`) did not appear in this
  run.

### Spike: R5 boundary sample (WezTerm, this macOS host)

Setup: a fake `claude` on an isolated PATH emitting claude stream-json —
init, two small assistant lines, a 52 KB final assistant message (Markdown
prose, code fences, links), then the `result` line and exit 0 — wrapped by
the worktree `claudine` in a real WezTerm pane with an isolated HOME. The
pane was frozen with XOFF (`0x13` via `wezterm cli send-text --no-paste`,
relying on the PTY line discipline's IXON) mid-stream; thread backtraces
were captured with `sample(1)`. A control experiment first proved the
mechanism: under XOFF a pane child blocks in `write(2)`
(`__write_nocancel`) and resumes on XON. Artifacts under
`/tmp/claudine-r5-spike/` (not committed).

Confirmed observations:

1. **Stall site.** The stdout reader blocked in `write(2)` inside
   `StdoutWriter::write` (`stream_io.rs:245`), reached from the
   `OutputTextCallback` (`semantic.rs:382`) inside
   `ClaudeSemanticStreamParser::feed_line` → `handle_assistant_message` →
   `flush_assistant_text` → `LiveSemanticSink::on_semantic_event`
   (`event_sink.rs:191`). The reader was mid-line — the large assistant
   message — with the `result` line unread, holding the only parser. This is
   exactly the incident's line-816 shape and R3's unprotected case.
2. **During the join.** The main thread waited in `join_reader`
   (`reader_join.rs:208`); the reader's tracked state was `Processing`, so
   the 120 s drain limit (not the 5 s pipe cap) applied.
3. **After the join timeout (R4 confirmed live).** Once the 120 s drained,
   the main thread proceeded to the warning write and blocked indefinitely
   in `StreamOutput::emit_stderr_line` (`stream_io.rs:217`) on
   `__psynch_mutexwait`: the abandoned reader still held the `StreamOutput`
   lock. The wrapper hung ~87 s past the join timeout and would have hung
   forever; no warning reached the pane. A bounded join alone does not
   prevent a blocked terminal from hanging the wrapper.
4. **Release symmetry.** On XON, both threads completed within the same
   second (session record `duration_ms: 222098`), matching the incident's
   "reader resumes within microseconds of `session_end`" evidence. Because
   the reader then fed the `result` line and filled the parser slot before
   `settle_parser` ran, this run kept exit code 0 with
   `exit_reason: stream_reader_timeout` recorded in `session_end`; a slower
   reader would have taken the failure path with an empty slot.
5. **Confirmed Claudine-side contributor.** 889 of 2217 reader-thread
   samples sat in per-block Markdown rendering rebuilding
   `Terminal::default()` (darkmatter `render_tree_terminal_with_context` →
   `terminal_options_from_terminal_options` →
   `biscuit_terminal::Terminal::default`), and 262 of those were blocked in
   `poll(2)` waiting on a subprocess spawned by
   `biscuit_terminal::discovery::fonts::font_name` →
   `fallback_font_name_scan` → `query_iterm2_font_name` (`defaults read
   com.googlecode.iterm2`). Font detection is uncached, so every Markdown
   block of a large final message can spawn a subprocess on the reader
   thread while it holds the parser. Implementation 1's read-only survey saw
   the `Terminal::default()` rebuild but judged it milliseconds and missed
   the uncached subprocess spawn. A fix task was appended to Phase 4's spike
   follow-up in the plan.

Leading explanation: the wrapper's reader performs terminal I/O from inside
the parser callback while holding both the parser and the shared output
lock; any sustained terminal backpressure (or unusually slow per-block
rendering) stalls it mid-line. What caused the user's real WezTerm to stop
accepting output is **not** established — XOFF was a controlled stand-in,
and no WezTerm defect is asserted.

### Requirement-to-test mapping for Phase 1

Phase 1 changes no behavior, so it adds no tests. Its verification is
observational: the spike backtraces above (kept under
`/tmp/claudine-r5-spike/out/`), the corrected green baseline, and the
recorded rulings. Behavior-changing phases (2 onward) carry the test
requirements.

## Phase 2

Independent fixes R6, R7, R8. Three source areas, all behind focused tests;
full-area `just test` and `just lint` green at the end of the phase.

### R6 — step-timeout durations always carry a unit

- `cli/src/commands/wrap/exec/timeouts.rs::format_internal_duration` printed
  60–3599 s as a bare number (`120` → `"2"`). It now renders `Ns` below a
  minute, `Nm` / `NmSs` (leftover seconds only when nonzero) up to an hour,
  and `NhMm` beyond it. The function's only caller is the legacy
  `detect_step_timeout` (dead in production, kept for its own tests); the
  live watchdog path uses `watchdog/breach.rs::format_duration` ("3m 0s"),
  which was already unit-ful and is unchanged. Grep found no test, snapshot,
  or doc quoting the old bare form (the "30m" fixtures in
  `termination/tests` and `composition/error/tests` are hand-written
  strings, and `docs/topics/timeouts.md` quotes the breach formatter).
- drift pass: the function's `///` doc now states the format contract.
- tests: `format_internal_duration_always_carries_a_unit` (rstest table over
  0, 59, 60, 61, 120, 3599, 3600, 3661, 7200 — covers all five plan
  boundaries) and
  `detect_step_timeout_message_includes_the_unit_suffixed_silence`, which
  asserts the downstream message ("no stream activity for 2m;") so the unit
  is verified in the composed output, not only at the helper.

### R7 — the sequence gutter inset reaches rendered Markdown

- root cause: `FinalMessage::render` passed `TerminalOptions` to darkmatter
  with `max_width: None`, so darkmatter's
  `terminal_options_from_terminal_options` re-detected the host terminal and
  used the *real* width. The narrowed `Terminal` that
  `new_assistant_stream_inset` builds (`fixed_width = width - inset`) only
  reached the `Prose` fallback. Fix: `render` now fills an unset `max_width`
  from the supplied `term.width()` (which honors `fixed_width`), clamped to
  1..=u16::MAX. An explicit `max_width` pin still wins. Both callers —
  `AssistantStream::render_markdown` (cached options, streaming path) and
  `emit_final_message` (default options, final path) — carry no pin, so both
  now honor the inset and agree with each other. `NO_COLOR` and capability
  fallback are untouched: production renders only run on a TTY, where
  darkmatter keeps the detected terminal and only overrides its width.
- discovery (observational scratch run, not committed): fenced code blocks
  pad their panel to the pinned width but long code lines overflow unwrapped
  — pre-existing darkmatter behavior, unchanged by this fix; the inset
  signal for code is the panel width, not line clipping.
- tests, all in the claudine library:
    - `render::final_message::tests::markdown_prose_honors_the_supplied_terminal_width`
      (prose wraps at a 40-column terminal; detection would give 80)
    - `render::final_message::tests::markdown_code_block_honors_the_supplied_terminal_width`
      (panel rows ≤ 40; pre-fix they pad to the detected 80)
    - `render::final_message::tests::a_terminal_narrower_than_the_inset_still_renders`
      (one-column terminal: no panic, no lost characters)
    - `render::final_message::tests::cached_options_without_max_width_honor_the_supplied_terminal`
      (the `with_options` streaming shape)
    - `render::final_message::tests::an_explicit_max_width_wins_over_the_supplied_terminal`
      (caller pin stays authoritative)
    - `l1 assistant_stream_width::plain_stream_output_is_byte_exact_raw_text`
      (plain non-TTY path unchanged, byte-exact through append+close)
    - `l1 assistant_stream_width::markdown_stream_honors_the_narrowed_terminal_width`
      (streaming and final paths agree at width 40)
- sabotage proof: with the pre-fix `render` body restored, the three width
  tests fail (`got [80, 80, 80, 80]` for the code panel); with the fix they
  pass. Pre-fix file stashed and restored; restored file byte-identical to
  the fixed version.
- placement fallout: the two `AssistantStream` tests first landed in the
  inline `mod tests`, pushing `assistant_stream.rs` past the 300-line
  inline-test budget — `test_placement::repository_test_placement` failed
  under the full suite. They moved to `lib/tests/l1/assistant_stream_width.rs`
  (declared in `main.rs`, public API only); `assistant_stream.rs` is
  byte-identical to before the phase. The placement gates pass.

### R8 — `an_early_wait_error_still_reaps_the_whole_tree` flake

- reproduction attempt: 60 focused runs under a concurrent full-area
  `just test` plus a 14-way `sha256sum` CPU hog — no failure. The single
  observed failure remains a once-in-a-full-suite event.
- finding (fixture at fault, not the production reap). The descendant's
  survivor tell was `sleep 1; echo late > marker`: the descendant wrote its
  marker one second after starting, and the test asserted the group SIGKILL
  from `ProcessTree::drop` landed first. Warm, the kill chain — command
  shell observes the pid file, `echo started` arms the injection, capture
  thread reads the byte, 20 ms wait poll notices, `drop(tree)` broadcasts —
  takes ~100 ms. Under full-suite load every link is scheduling-bound and
  the chain can exceed the one-second fuse: the *working* reap then arrives
  after the marker, and the test reports a leak that never happened. The
  32 s timeline fits a load-slowed fixture (each pid-wait-loop iteration
  forks `/bin/sleep`; the descendant's own exec competes for the same
  scheduler). A genuine 10 s survivor of a delivered group SIGKILL has no
  mechanism; the leak is not in `shell.rs`.
- fix (descendant-ready handshake, per the plan's preference):
  `BackgroundedDescendant::survivor_action` replaces the fuse. The
  descendant body is now
  `while kill -0 $PPID 2>/dev/null; do sleep 0.1; done; <action>; sleep 30`:
  it acts only after its parent — the command shell — is reaped. The reap
  kills parent and descendant in one broadcast, so a reaped descendant
  never observes the death at any load (the success path carries no timing
  assumption at all); a genuine survivor observes the parent's reap, acts,
  and lingers 30 s so `assert_reaped`'s pid watch names it by pid (it exits
  on its own, so a failed run leaves nothing). The `$PPID` zombie window
  (`kill -0` succeeds until `child.wait`) is what orders the observation
  after the epilogue. Neither `assert_reaped` nor the marker assertion was
  weakened; no retries added.
- hardening, same fixture, same class: the pid-wait loop's `exit 90` budget
  went from 1000 to 6000 `sleep 0.01` iterations (~10 s → ~60 s floor).
  Under load the loop's wall-clock and the descendant's publication delay
  stretch together; a short budget can give up on a merely starved
  descendant and fail on fixture timing.
- sabotage proof: with `ProcessTree::drop` changed to signal only the direct
  child (never the group), both reap tests fail deterministically as
  `FAIL + LEAK` — "the backgrounded descendant (pid N) outlived its command
  by more than 10s" — the same failure class as the incident, now
  deterministic and pid-named. Production code restored and verified by
  grep/git.
- the parent-death gate was verified by hand on this host (a descendant
  whose parent exits without the group kill writes the marker and lingers;
  cleaned up).
- all four `BackgroundedDescendant` call sites converted (the two marker
  tests and the two `printf late` stream tests; the stream contracts stay
  discriminating — a leaked descendant's print lands in the still-parked
  pipe reader).
- validation: the four reap tests × 25 iterations under concurrent
  full-area `just test` + CPU hog: 0 failures. Linux evidence via
  `just cross-check claudine --os linux <four filters>`: 4/4 pass (the
  fragment relies on `$PPID`, builtin `kill -0`, fractional `sleep` —
  portable across bash and dash). Windows is untouched: these tests are
  `#[cfg(unix)]` and the Windows twin has no pid fixture.

### Requirement-to-test mapping for Phase 2

- R6: `format_internal_duration_always_carries_a_unit` (boundary table) +
  `detect_step_timeout_message_includes_the_unit_suffixed_silence`
  (downstream message). Both L1 unit tests in claudine-cli, selected by
  `just test`.
- R7: five `render::final_message::tests::*` unit tests (lib target) + two
  `claudine::l1 assistant_stream_width::*` integration tests. All L1,
  compiled by declared targets (lib unit tests; the declared `l1` test
  binary via `tests/l1/main.rs`), selected by `just test`.
- R8: the pre-existing four reap tests now carry the deterministic
  handshake; no new test was needed — the contract and its assertions are
  unchanged, the fixture no longer races them. Sabotage discrimination
  proven for both the marker and pid assertions.
- broader gates: full-area `just test` 8390 passed / 9 skipped / 0 failed;
  `just lint` exit 0; Linux cross-check of the reap tests green. No skipped
  or pre-existing failures observed in this phase.

## Phase 3

Reader outcomes on every spawn path (R1, R2). All in claudine-cli.

### Changes

- `exec/reader_join.rs`: shared reporter. `ReaderStream` (`Output`, `Stdout`,
  `Stderr`), `ReaderFailure { error_kind, message }`, `reader_failure`,
  `reader_warning_line`, `eprint_reader_warning` (direct `eprintln!`, so it
  shows with tracing off). Panic wording is `Stream {parser|stdout reader|
  stderr reader} thread panicked: {payload}`; timeouts reuse
  `timeout_message`. `settle_parser` now builds its panic and timeout messages
  through `reader_failure`; its stdout wording is unchanged. `panic_message`
  already covered non-string payloads (says "the panic payload was not a
  string"); a doc line and a test now pin that.
- Retired `join_with_timeout`/`join_with_timeout_or` (`exec/mod.rs`) and the
  untracked `join_within` and `ReaderStall::Unknown`, which only they used.
- Captured path: both readers tracked by `ReaderProgress`, joined by
  `join_captures` under one `ReaderBudget` and one clock started after
  process-group teardown. The capture buffer is an `Arc<Mutex<String>>` shared
  with the reader, so a panicked or abandoned reader's partial output is
  kept. `CapturedChildOutput.incomplete` flags it.
- Inherited path: forwarders are tracked and joined by `join_forwarders`
  (same budget and clock). `forward_lines` keeps draining after a write
  error and returns the first `io::Error`; a failed forward is a warning,
  never a provider failure (Ruling F). Stdout/stderr forwarding is generic
  over `Write` so tests inject a failing and a stalled sink.
- Semantic path: stderr reader outcome uses `reader_failure`; warnings render
  through `reader_warning_line`. Its stderr panic wording changed from "The
  agent's stderr reader panicked" to the shared wording.
- `ProcessResult.reader_warnings` carries the warnings.

### Departures and open items

- The session-record field for captured/inherited incomplete output is **not**
  written yet. `ProcessResult.reader_warnings` and
  `CapturedChildOutput.incomplete` are populated but carry
  `#[allow(dead_code)]` (the existing `agent_pid` precedent). Plumbing them
  into `session_end` crosses the attempt tuple in `harness_orch/attempt.rs`
  and `policy.rs`, and Ruling A/B's output-loss record in Phase 4 is the
  natural owner of that field. Recorded for Phase 4.
- Lint first failed the `error_guards` scan on `error.to_string()`; the
  forwarder now returns the typed `io::Error` and formats it at the join.

### Requirement-to-test mapping

All L1, in claudine-cli (`just test`), injected millisecond budgets, no real
5 s / 120 s waits:

- panic vs timeout, string and non-string payload, all streams:
  `reader_join::tests::reader_failure_names_each_outcome_with_the_same_wording_for_every_stream`,
  `spawn::tests::captured::capture_readers_report_panics_apart_from_timeouts`
- held-open pipe, partial buffer kept, incomplete:
  `spawn::tests::captured::a_held_open_capture_pipe_keeps_the_partial_buffer_and_warns`
- shared deadline, state change does not reset it:
  `spawn::tests::captured::both_capture_joins_share_one_deadline_and_a_new_line_does_not_reset_it`
- completion observed before declaring timeout:
  `reader_join::tests::a_reader_that_finished_is_joined_even_when_the_clock_ran_out_long_ago`,
  `spawn::tests::captured::a_finished_capture_reader_is_joined_even_past_the_deadline`
- pipe-cap-after-settle and between-lines (Implementation 1, unchanged):
  `reader_join::tests::a_reader_back_on_a_held_pipe_after_a_slow_line_times_out_at_the_pipe_cap`,
  `a_reader_slow_on_a_line_before_eof_gets_the_drain_limit`
- inherited: `spawn::tests::inherited::a_failed_forward_is_a_warning_and_the_pipe_is_still_drained`,
  `a_stalled_terminal_times_out_the_forwarder_as_a_warning`
- stderr visibility: `reader_join::tests::the_warning_line_carries_the_message_through_the_status_component`
  asserts the exact line `eprint_reader_warning` prints; the direct `eprintln!`
  itself is not captured by a test (honest gap).

### Gates

`just lint` exit 0; `just test` 8399 passed, 9 skipped, 0 failed. Docs:
`docs/topics/timeouts.md` gained the captured/inherited paragraph. Not run:
cross-OS rigs (tests use channels and in-memory sinks, no `cfg(unix)`).
