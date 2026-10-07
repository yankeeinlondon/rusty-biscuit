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
source_files_during_phase_4:
    - claudine/cli/src/commands/wrap/output_worker.rs
    - claudine/cli/src/commands/wrap/output_worker/tests.rs
    - claudine/cli/src/commands/wrap/run_scope.rs
    - claudine/cli/src/commands/wrap/run_scope/tests.rs
    - claudine/cli/src/commands/wrap/stream_io.rs
    - claudine/cli/src/commands/wrap/stream_io/tests.rs
    - claudine/cli/src/commands/wrap/mod.rs
    - claudine/cli/src/commands/wrap/section.rs
    - claudine/cli/src/commands/wrap/policy.rs
    - claudine/cli/src/commands/wrap/wrapper_exec.rs
    - claudine/cli/src/commands/wrap/harness_orch/attempt.rs
    - claudine/cli/src/commands/wrap/live_semantic_sink/event_sink.rs
    - claudine/cli/src/commands/wrap/exec/reader_join.rs
    - claudine/cli/src/commands/wrap/exec/reader_join/tests.rs
    - claudine/cli/src/commands/wrap/exec/spawn/semantic.rs
    - claudine/cli/src/output/assistant.rs
    - claudine/cli/src/shutdown.rs
    - biscuit-terminal/lib/src/discovery/fonts/mod.rs
docs_updated_during_phase_4:
    - claudine/docs/topics/timeouts.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
    - .claude/skills/claudine/SKILL.md
source_files_during_phase_5:
    - claudine/cli/src/commands/wrap/exec/reader_join.rs
    - claudine/cli/src/commands/wrap/exec/reader_join/matrix.rs
docs_updated_during_phase_5:
    - claudine/docs/topics/timeouts.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5: []
source_files_during_phase_6: []
docs_updated_during_phase_6:
    - claudine/docs/topics/timeouts.md
    - claudine/docs/topics/signal-handling.md
    - claudine/docs/topics/composition.md
docs_created_during_phase_6: []
skills_files_updated_during_phase_6: []
source_code:
    - claudine/cli/src/commands/wrap/exec/timeouts.rs
    - claudine/cli/src/commands/wrap/exec/timeouts/tests.rs
    - claudine/lib/src/render/final_message.rs
    - claudine/lib/src/composition/sequence/task/tests.rs
    - claudine/lib/tests/l1/assistant_stream_width.rs
    - claudine/lib/tests/l1/main.rs
    - claudine/cli/src/commands/wrap/exec/reader_join.rs
    - claudine/cli/src/commands/wrap/exec/reader_join/tests.rs
    - claudine/cli/src/commands/wrap/exec/mod.rs
    - claudine/cli/src/commands/wrap/exec/spawn/captured.rs
    - claudine/cli/src/commands/wrap/exec/spawn/inherited.rs
    - claudine/cli/src/commands/wrap/exec/spawn/semantic.rs
    - claudine/cli/src/commands/wrap/exec/spawn/tests/captured.rs
    - claudine/cli/src/commands/wrap/exec/spawn/tests/inherited.rs
    - claudine/cli/src/commands/wrap/exec/wiring/session.rs
    - claudine/cli/src/commands/wrap/output_worker.rs
    - claudine/cli/src/commands/wrap/output_worker/tests.rs
    - claudine/cli/src/commands/wrap/run_scope.rs
    - claudine/cli/src/commands/wrap/run_scope/tests.rs
    - claudine/cli/src/commands/wrap/stream_io.rs
    - claudine/cli/src/commands/wrap/stream_io/tests.rs
    - claudine/cli/src/commands/wrap/mod.rs
    - claudine/cli/src/commands/wrap/section.rs
    - claudine/cli/src/commands/wrap/policy.rs
    - claudine/cli/src/commands/wrap/wrapper_exec.rs
    - claudine/cli/src/commands/wrap/harness_orch/attempt.rs
    - claudine/cli/src/commands/wrap/live_semantic_sink/event_sink.rs
    - claudine/cli/src/output/assistant.rs
    - claudine/cli/src/shutdown.rs
    - biscuit-terminal/lib/src/discovery/fonts/mod.rs
    - claudine/cli/src/commands/wrap/exec/reader_join/matrix.rs
documentation:
    - claudine/docs/topics/timeouts.md
    - .claude/skills/claudine/SKILL.md
    - claudine/docs/topics/signal-handling.md
    - claudine/docs/topics/composition.md
completed_phase: 6
implemented: true
packages:
    - claudine
    - claudine-cli
    - biscuit-terminal
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

## Phase 4

Terminal delivery is now separated from parsing (R3, R4).

### What changed

- `wrap/output_worker.rs`: `OutputWorker`, one thread per coordinator, a
  bounded queue of whole frames (4096 frames, 8 MiB) carrying stdout and stderr
  frames in order (Ruling C). `submit` never blocks. `drain(deadline)` waits
  for the queue; on expiry it sets the *disabled* flag, discards the queue and
  abandons the thread. After that `submit` rejects at once, and the worker loop
  returns before taking another frame, so a late in-progress write may finish
  but nothing follows it. No code path starts a second writer (the thread is
  spawned once, lazily). `FrameSink` is the test seam; `OutputLoss` counts
  dropped, rejected, and stalled.
- `wrap/stream_io.rs`: every `StreamOutput` emission (`stdout_writer`,
  `emit_stderr_line`, `emit_stderr_frames`, `emit_stdout_frames`,
  `emit_stdout_line`, new `emit_stderr_undecorated`) builds a byte frame and
  queues it. Cursor bookkeeping stays at queue time, under the same lock, so
  frame order and the injected newline are as before; it moves only when the
  frame was actually queued. `StreamOutput::shared()` is the one process-wide
  worker, which is what makes Ruling B hold (a stalled sink stays silent for
  later iterations; no per-iteration writer). Tests moved to
  `stream_io/tests.rs` because inline tests passed the 300-line placement cap.
- `wrap/run_scope.rs`: a thread-local `RunScope` the stdout reader enters.
  It carries a `closed` flag and a `ResultSnapshot`. `LiveSemanticSink`
  returns early from `on_semantic_event` when the scope is closed (no render,
  no lifecycle dispatch, no event log), and publishes a completed turn or
  terminal error into the snapshot *before* rendering. The output layer checks
  the same flag, so a reader past its cutoff cannot write into a following
  iteration (replaces the plan's "generation token"; a thread-local reaches the
  sink and the writers without changing the parser-builder signature).
- `spawn/semantic.rs`: the reader enters the scope; after the stdout join the
  run settles from the snapshot and closes the scope; the main-thread
  `eprintln!` of captured stderr is now queued; after the warnings the run sets
  the output deadline `readers_since + drain_limit` and drains against it
  (Ruling E).
- `settle_parser` takes the snapshot. With the slot empty and the reader
  stalled: published terminal error is kept; a published completed turn with
  exit 0 is kept as success (no assistant text, only the parser had it);
  anything else stays `stream_reader_timeout`. `FallbackParser` now holds a
  whole summary. Phase 5 owns the full matrix.
- `wrapper_exec.rs`, `harness_orch/attempt.rs`: `section_stream.drain_final()`
  after `emit_stream_summary`, which consumes the run's stored deadline (so a
  stale clock cannot cut off the next iteration), falling back to 120 s.
- `shutdown.rs::flush_streams`: drains the worker (10 s fallback) before the
  exit flush and skips the flush when delivery was disabled, because the
  abandoned writer may hold the stdout lock and `stdout().flush()` would wait
  on it forever.
- `output/assistant.rs`: `emit_final_message` with a section stream writes
  through the worker instead of `std::io::stdout()`.
- `policy.rs`: `emit_stream_summary_inner` adds `extra.output_incomplete`
  (`dropped_frames`, `rejected_frames`, `stalled`) to the `session_end` row
  when the run lost output; the loss is read from `SectionStream::output_loss`
  against a mark taken when the stream was built (Ruling A's "recorded in the
  session record"). The plan said the field crosses the attempt tuple; reading
  it from the section stream avoided that.
- Spike follow-up: `biscuit_terminal::discovery::fonts::font_name` is cached in
  a `OnceLock`. `Terminal::default()` calls it, so this removes the per-block
  `defaults read com.googlecode.iterm2` subprocess in darkmatter's Markdown
  render path. The fix is in the lower layer that owns the call, not a hoist in
  darkmatter, which would have needed a cached `Terminal`.
- Docs: `docs/topics/timeouts.md` gained "A terminal that stops accepting
  output" and the corrected "join alone" claim; the claudine skill's Timeouts
  line names the worker.

### Departures and open items

- A delivered-late frame problem was found and fixed while writing tests: a
  rejected write used to move the newline cursor, which would have injected a
  stray `\n` into the next iteration.
- Not routed through the worker, by choice: the stderr reader's raw
  passthrough (`std::io::stderr().lock()`; a stall there is bounded by its
  join and reported as a stderr-reader warning), the captured/inherited paths'
  own forwarding, the Kimi wire session, and the `eprintln!` fallbacks used
  only when no section stream exists (tests, legacy). A terminal stall there
  does not hold the parser, but it also is not covered by the worker's
  disabled flag. Flagged for review.
- The output-loss warning is not shown on the terminal, by design: a stalled
  terminal cannot show it. It is in the session record and `tracing::warn!`.
- `ProcessResult.reader_warnings` and `CapturedChildOutput.incomplete` remain
  `#[allow(dead_code)]` (Phase 3 note). The `output_incomplete` field covers the
  semantic path; the captured and inherited paths do not use the worker.
- No test drives the real `run_child_stream_semantic` with a blocked sink: the
  semantic path has no existing process-level test harness and a portable
  provider fixture does not exist. The blocked-sink behavior is covered by
  simulating the reader (parser rendering through `StdoutWriter`, `join_reader`,
  `settle_parser`, `drain`) in `stream_io/tests.rs`. A real-process test is a
  candidate for Phase 5/6.
- The font cache has no direct test (subprocess spawns are not observable from
  a unit test); existing biscuit-terminal tests pass.

### Requirement-to-test mapping

All L1, `just test` in `claudine`:

- blocked sink: run completes, no emission, trailer wait, or exit drain hangs:
  `stream_io::tests::a_blocked_terminal_stalls_no_caller`,
  `output_worker::tests::a_permanently_blocked_sink_never_blocks_submit_or_drain_past_its_deadline`
- no late write after disabling:
  `output_worker::tests::the_write_in_progress_may_finish_but_nothing_follows_it_after_disabling`
- late reader cannot write into the next iteration:
  `stream_io::tests::a_closed_run_cannot_write_into_the_next_iteration`,
  `run_scope::tests::closing_a_scope_closes_it_for_the_thread_that_entered_it`
- slow line, unread result:
  `stream_io::tests::a_reader_behind_a_blocked_terminal_still_parses_the_unread_result`
- queue bound (frames, bytes, whole frames, provider data untouched):
  `output_worker::tests::a_full_queue_drops_whole_frames_and_counts_them`,
  `the_byte_cap_bounds_queued_memory`
- no writer-thread growth:
  `output_worker::tests::a_stalled_sink_never_gains_a_second_writer_thread`,
  `stream_io::tests::iterations_on_a_stalled_terminal_share_one_writer`
- unblocked output unchanged (order, newline injection):
  `output_worker::tests::an_unblocked_sink_receives_every_frame_in_order`,
  `stream_io::tests::queued_frames_keep_cursor_order_for_an_unblocked_sink`,
  and the pre-existing `stream_io` cursor tests
- stale deadline cannot cut off the next iteration:
  `stream_io::tests::the_drain_deadline_is_consumed_by_the_final_drain`
- snapshot settle: `reader_join::tests::a_stalled_reader_with_a_published_turn_and_exit_zero_keeps_the_result`,
  `a_stalled_reader_keeps_the_provider_error_it_had_published`,
  `a_stalled_reader_with_no_published_result_is_an_incomplete_stream`,
  `a_published_turn_does_not_turn_a_nonzero_exit_into_success`;
  scope publishing: `run_scope::tests::leaving_the_scope_stops_publishing_into_it`

### Gates

`just test` (claudine): 8419 passed, 9 skipped, 0 failed. `just lint`: exit 0.
`just test` in biscuit-terminal: 3566 passed. The sink tests use channels,
condvars, and in-memory sinks and carry no `cfg(unix)`; the cross-OS rigs were
not run. Not run: `cargo fmt`.

## Phase 5

Success preservation matrix (R3).

### Finding

Phase 4's snapshot-aware `settle_parser` already met every outcome rule, so
Phase 5 changed no behavior; it proves the rules and documents them.

- exit 0 with a published turn, stalled anywhere: result and exit kept, warning
- published terminal error: kept as the run's error (also with exit 0)
- nothing published: `stream_reader_timeout`, never an invented result
- nonzero exit or interruption: exit code kept
- provider timeout / rate limit: applied by the caller after settling, so the
  reader warning cannot replace it
- real panic: `parse_failure`
- stderr-reader failure: warning only. The stderr bridges (Codex app-server,
  Pi RPC, OpenCode) accumulate into shared state, not the reader's return
  value, so a timed-out stderr reader does not change the bridge outcome.

### Tests

- `reader_join::matrix` (new file): 6 run outcomes x {stall before the parser
  is handed back (`Processing` and `PipeOpen`), stall in the final render,
  panic, joined in time}, asserting exit code, `error_kind`, `is_error`, and
  warning presence. Provider timeout is applied with the real
  `apply_early_termination_to_summary`.
- `a_slow_but_progressing_reader_is_joined_with_its_real_parser_and_no_warning`:
  the Codex 6.5 s case with scaled budgets.
- Per-path (captured partial bytes and incomplete flag, inherited forward
  failure is a warning) are the Phase 3 tests in `spawn/tests/`.

### Departures

- the plan asked for "rendering stalled" as a third reader failure; it is the
  slot-holding timeout column. The `Interrupted` and `ProviderTimeout` rows
  model the post-settle caller behavior rather than driving a process.

### Docs

`docs/topics/timeouts.md`: outcome table for a stalled reader. `settle_parser`
docs note the panic and early-termination rules.

### Gates

`just test` (claudine): 8423 passed, 9 skipped, 0 failed (before the final
matrix test additions; the new matrix tests pass focused). `just lint`: exit 0.
Not run: `cargo fmt`, cross-OS rigs (tests are portable, no `cfg(unix)`).

## Phase 6

Docs, drift pass, and verification.

### Docs

- `docs/topics/timeouts.md`: framed the post-exit wait as an internal cleanup
  budget separate from the two provider rules, added a sequence diagram of the
  post-exit flow (teardown, shared clock, joins, worker drain, `session_end`),
  added the R5 findings ("What the stall looked like"), and stated that
  step-timeout durations always carry a unit. The earlier claim that a bounded
  join alone keeps a blocked terminal from hanging the wrapper was already
  corrected in Phase 4.
- `docs/topics/signal-handling.md`: new "Terminal output at exit" section on
  the bounded exit-time output wait and its relation to Ctrl+C and wrapper
  termination.
- `docs/topics/composition.md`: new "Agent output inside a sequence task"
  section (gutter inset reaches rendered Markdown, saturation, non-TTY).
- READMEs and the claudine skill: checked, no change needed. None describes
  these behaviors, and the skill's timeouts summary already names the worker.

### Drift pass

No drifted comments found in the changed symbols. A repo grep for
`join_with_timeout_or` and `unwrap_or` on join results finds only
`exec/wiring/session.rs:314`, the Kimi wire stderr join, which is outside this
fix (flagged in the spec's `human_review_items`). The portable sink tests carry
no `cfg(unix)`.

### Requirement-to-test mapping

- R1: `reader_join::tests` (panic vs timeout, stderr visibility), `spawn/tests/{captured,inherited}.rs`
- R2: `reader_join::tests` (caps, shared deadline, completion before timeout)
- R3: `reader_join::matrix`, slow-reader fixture, `stream_io` slow-line tests
- R4: `output_worker::tests`, `stream_io::tests`, `run_scope::tests`
- R5: timeouts.md paragraph above
- R6: `format_internal_duration_always_carries_a_unit`
- R7: `render::final_message` tests, `tests/l1/assistant_stream_width.rs`
- R8: `an_early_wait_error_still_reaps_the_whole_tree` (Phase 2)

### Gates

`just test` (claudine): 8424 passed, 9 skipped, 0 failed. `just lint`: exit 0.
Not run: `cargo fmt`, the cross-OS rigs (this phase changed docs only), and
`check-tier-coverage` (the recipe does not exist in this checkout).

## Review 1: direct terminal writes

### Departure

R4 lists warnings among the writes that must stay bounded on a terminal that
never drains. Every wrapper-owned write made during or right after a provider
run now goes through the output worker: reader warnings, inherited forwarding,
the semantic and Kimi wire stderr passthrough, and the captured response and
stderr. Status lines written through `crate::log` (61 call sites under
`commands/wrap/`, plus the step/stall-timeout warnings and the interrupt status
in `harness_orch`) are not routed through the worker: that would move the
CLI-wide `crate::log` channel onto it and change stdout/stderr ordering for
every command. They stay direct writes behind a process-wide terminal gate
(`cli/src/terminal_gate.rs`) instead. When the process-terminal worker is
abandoned it closes the gate, and every direct writer checks it first and skips
the write rather than waiting on the stdout/stderr lock the abandoned write
still holds. A skipped `crate::log` line or `--perf` report counts in the
worker's `rejected_frames`. The `harness_orch` `eprintln!` sites now write
through `crate::log::message` (same bytes), and the same gate covers the
`--perf` report, `--debug` tracing, and the Ctrl+C notices written from the
Unix signal handlers and the Windows console handler. Before a stall the gate
is open and nothing changes; a direct write made while the worker is stuck but
before the drain deadline has disabled it can still block, as before.

## Review 1: timeout snapshots

### Departure

R3 asks for a result snapshot published before rendered output is submitted.
The coarse snapshot (completed turn, terminal error) still is, from the event
sink. The full snapshot, which carries the answer, session, usage, and the
provider's finalization verdict (Claude's task ledger), cannot be: the sink
runs inside `feed_line` while the reader holds the parser mutably. The claudine
library therefore gained `SemanticStreamParser::snapshot`, implemented by every
provider parser as the summary `finish` would return, and the reader publishes
it after every line it finishes once a result or terminal error was reported
(`run_scope::feed_line`). It is finalized at exit 0 because the exit code is
not yet known; settlement keeps the real exit code for a failure and never
preserves success for a nonzero exit. A reader that stalls inside the result
line's own handling still leaves only the coarse snapshot, so such a run keeps
success without its answer text, as before. Since rendering is queued to the
output worker, only non-terminal sink work (lifecycle dispatch, logging) can
stall there.

## Review 1: sequence Markdown in a real pane

### Departure

The R7 record above says both rendering paths honor the gutter inset. The
Level-2 rows added for this finding showed that two did not. Streamed text
that arrives mid-line was shown raw, before Markdown rendering, so nothing
folded it, and the terminal wrapped it back to column 0 outside the gutter. The
claudine library's `AssistantStream` gained `holding_partial_lines`, which
`new_assistant_stream_inset` turns on for a nonzero inset on a terminal. The
task's `TaskFrameWriter` already held an incomplete line, so this causes no
visible delay. Separately, Codex's final message (the `--output-last-message`
answer) bypassed the task's frame writer and used the full-width terminal. In a
sequence task, `execute_harness_attempt` now writes it through
`emit_framed_final_message`, which uses the gutter and the inset width. Not
changed: a status line wider than the pane, such as the run trailer with token
counts, and a code line wider than the panel are still folded by the terminal
and lose the gutter.

## Review 2: direct diagnostics and panic reporting

### Departure

The review-1 departure above kept `crate::log`, `--perf`, tracing, and the
Ctrl+C notices as direct writes behind the terminal gate. That left them able
to block before any drain had found the stall, and a write already blocked was
not released when the gate closed. It is superseded: once a wrapped run creates
the process-wide output worker (`StreamOutput::shared`), every diagnostic is a
frame on it through `crate::terminal_gate::write`, which never blocks. The
ordering concern that motivated the earlier departure is met by the trigger:
commands that never create the worker, and anything a wrapped command writes
before its first run (preflight, interactive prompts), still write directly.
Details that depart from or extend the spec's wording:

- The library gained `claudine::render::console` (`set_console_writer`,
  `write_stderr_line`, `write_stdout_line`) so its lifecycle-action, harness
  report, messaging, overlay-lease, and hook-report lines can join the CLI's
  worker. Without an installed writer it prints exactly as before.
- The panic hook is color-eyre's report, written through
  `terminal_gate::write` instead of color-eyre's `eprintln!`
  (`terminal_gate::install_report_hooks`).
- The Unix wrapper SIGINT handler no longer writes its feedback line. The wait
  loop shows one line per counted press each 75 ms poll, so the line can trail
  the press by up to 75 ms; the signal still reaches the child at once.
  `wait_with_signal_handling` on Unix now polls `try_wait` every 75 ms instead
  of blocking in `wait`, so it can show that line.
- The Unix compose handler posts one byte per press to a relay thread (the
  former grace-exit watcher) and never writes to the terminal. The relay shows
  the notice and, for an exit rung, gives it 250 ms to reach the terminal
  before `_exit(130)`. If the relay could not be started an exit rung exits
  immediately without its notice, where it used to write the notice from the
  handler.
- A diagnostic queued while an interactive provider shares the terminal is
  written by the worker as soon as the terminal accepts it, rather than by the
  thread that produced it; with a healthy terminal the difference is the
  worker's scheduling delay.

## Review 2: completion handling

### Resolution of the Review 1 timeout-snapshot departure

The Review 1 departure above ("a reader that stalls inside the result line's
own handling still leaves only the coarse snapshot, so such a run keeps success
without its answer text") is resolved; that entry is left as written. The
reader now separates parsing a line from the sink work it causes: every parser
emits into a `run_scope::DeferredSink`, which, while the reader is inside
`run_scope::feed_line`, holds each event instead of handing it to the
`LiveSemanticSink`. A result or terminal error marks the scope reported the
moment the parser emits it. After the parser has handled the whole line,
`feed_line` publishes `SemanticStreamParser::snapshot(0)` and only then
delivers the held events (rendering, logging, hooks). Outside `feed_line` (the
Kimi wire session, which feeds its parser directly, and anything `finish`
emits) events are delivered at once, as before.

The coarse completed-turn marker (`ResultSnapshot::turn_complete`,
`TurnResult`) and the settlement branch that built success from it without the
answer are removed. A stdout reader cannot hold an unparsed result line with
sink work outstanding, so that case is unreachable; a reader that stalls before
the parser emits a result settles as `stream_reader_timeout`. The terminal
error published by `LiveSemanticSink` is kept, because a stderr bridge (Codex,
OpenCode) reports through it on a thread that has no parser line.

### Departures

- The events of one line now reach the sink after the parser has handled the
  whole line rather than as each is emitted. Providers emit one JSON record per
  line, so the visible difference is the time to parse one line.

## Review 3: terminal write errors

The output worker now counts each frame its sink refuses with an I/O error in
`OutputLoss::failed_writes`, before it clears `writing`, so a drain that finds
the queue idle also sees the failure. Every consumer already read
`OutputLoss`, so the count reaches all of them through that one type:

- inherited forwarding adds `failed_writes` to the counts that raise its
  forwarding-loss warning;
- `deliver_capture` returns the loss since a reading taken before it queued
  anything, instead of `Drained`, and `captured_output_status` takes that
  loss, so a drain that completed with failed writes is incomplete delivery;
- structured attempts and the session-end record read
  `SectionStream::output_loss`, which now includes the count; the record's
  `output_incomplete` object gains `failed_writes` (additive).

### Departures

- The spec does not say whether a sink that returned an error should be
  written again. The worker keeps writing later frames and counts each failure,
  without retrying the failed frame: a write error returns at once, so it
  cannot stall the run, and a closed stdout says nothing about stderr. A
  stalled sink is still disabled as before.
- The loss counters stay process-wide rather than per stream, as before; the
  record and statuses report that output was lost, not on which stream.

## Incident evidence audit — 2026-10-07

Rechecked the daily Claudine JSONL files against the claims in Implementation
1, matching both `agent_pid` and `claudine_pid`. File order is not timestamp
order: duplicate hook/semantic records can be appended after `session_end`
with timestamps from before it. A record later in the file does not by itself
establish that processing resumed after the session-end timestamp.

For the original eleven incidents through agent PID 8848, all session-end
records have child exit code 0. Six have a semantic record timestamped strictly
after session end within 1 ms; four have their first such record at about
16.5 ms, 41.3 ms, 6.49 s, and 48.54 s. The remaining Codex run (agent PID
76722) logged `turn_complete` 0.745 ms before session end and has no subsequent
semantic record under that process identity in the daily file. The earlier
claim that ten of eleven resumed within 1 ms is not supported by this audit.
These are processing/logging times, not terminal-delivery or child-write times.

For the October 6 incident, correlated Claude's native transcript
(`8079a339-1346-41d5-b021-80d0b69590d3`, under the host's Claude project logs)
with `/Users/ken/.claudine/logs/2026-10-06.jsonl`:

| Boundary | UTC timestamp | Evidence |
| --- | --- | --- |
| Last tool result logged by Claudine | 08:21:13.864267 | Daily log row 6838, stream line 815 |
| Final answer recorded by Claude | 08:21:29.979 | Native transcript row 595, 4838 characters |
| Claude stop hook completed | 08:21:30.172 | Native transcript row 598; hook duration 181 ms, no errors, continuation not prevented |
| Claudine reported the parser panic | 08:22:26.640994 | Daily log row 6845, session end, child exit 0 |
| Claudine logged the final answer | 08:22:26.641015 | Daily log row 6846, stream line 816; identical to the native answer plus a trailing newline |
| Claudine logged the result | 08:22:26.641202 | Daily log row 6847, stream line 817 |

The final answer existed in the provider transcript about 56.662 s before
Claudine logged it. This supports delayed wrapper processing of an already
produced answer, rather than continued answer generation after the reported
panic. It does not timestamp the provider's stdout write, entry into rendering,
the reader-join deadline, or a terminal write's return. The semantic logger
runs after output rendering/delivery callbacks, and `session_end` is generated
after summary presentation, so neither timestamp marks the onset of its stall
or the instant the join timed out.

The native transcript correlation was absent from the earlier recorded
investigation. The controlled XOFF experiment proves a possible mechanism,
not the cause of these live incidents. Establishing that cause still requires
correlation of the other native transcripts and, for a recurrence, timed
observations at pipe read, render, output-lock, write, and join boundaries.
No production behavior or author decision was changed by this audit.

### Caller exit, provider identity, and answer length

Traced the code preceding the reader-join repair (`edd641291`'s parent).
After child exit, `run_child_stream_semantic` waited 5 s for the stdout reader
and substituted `ErrorParser` if the wait expired. That parser also served
real panics and always produced `is_error: true`, `parse_failure`, and
"Stream parser thread panicked", retaining the child's native exit code.
`classify_failure` treated the error flag as a failure despite native exit 0;
the harness's terminal failure branch returned 1 for that combination. The
session record's exit 0 therefore did not establish Claudine's shell exit 0.
Without lifecycle recovery, a shell chain using `&&` or an enclosing loop
could stop after an already displayed answer.

The reader rendered before logging semantic events and could keep running
after its handle was abandoned. Output could appear before the failure banner
even with the corresponding JSONL answer record after `session_end`. Codex
also recovered and rendered its last-message file independently of the
fallback parser's error flag; finding an answer did not clear that flag.

Using stream session-start and semantic provider identities, the original
eleven cases are nine Claude runs and two Codex runs. Some Codex session-end
rows say `provider: claude`, so that top-level field alone is unsuitable for
counting providers in these historical fallback records.
The old fallback conflated timeout and panic; continued reader events establish
that many of these were timeouts, but the message alone cannot prove that for
every case, particularly the Codex run with no later logged semantic events.

| Agent PID | Provider | Final answer characters in Claudine's semantic log |
| --- | --- | --- |
| 22643 | Claude | No final answer/result pair identified |
| 208 | Claude | 4923 |
| 17960 | Claude | No final answer/result pair identified |
| 54219 | Claude | 3379 |
| 69203 | Claude | 5421 |
| 76722 | Codex | Final answer uses the last-message file, not `output_text` events |
| 23924 | Claude | 1416 |
| 11587 | Claude | No final answer/result pair identified |
| 88503 | Codex | Final answer uses the last-message file, not `output_text` events |
| 24541 | Claude | 4151 |
| 8848 | Claude | 4839 |

For the six identifiable Claude final-answer/result pairs, the last output-text
event has median length 4495 characters, ranging from 1416 to 5421. In 459
successful Claude session records in the same daily files and before the
October 6 incident, the last output-text event has median length 3339, a
90th-percentile order statistic of 4972, and maximum 6876. These are Unicode
character counts of one presentation segment, including its trailing newline,
not entire-invocation output sizes. The controls exclude errored records and
are not matched for prompt, output formatting, CPU load, or terminal state.
The sample supports somewhat longer answers in these six incidents, but not
length as a sufficient cause. Three Claude cases lack a logged final pair and
cannot be included honestly in that comparison.

All nine corresponding native Claude transcripts are available. The last
native text block is not necessarily the final answer: for example, agent PID
17960's transcript ends its text well before the reader's remaining tool and
result records. Native finality and result semantics must be checked before
treating every incident as the same delayed-final-answer shape.

Longer Markdown, repeated per-block font discovery, and CPU scheduling delays
can postpone reader completion past a fixed 5 s wait. Per-block font discovery
was measured in the controlled sample and is now cached. Historical logs have
no CPU or blocked-stack observations that establish CPU contention or terminal
backpressure in the live incidents. In particular, one Codex completion was
already logged before the reported panic, showing that the old fallback could
discard parsed completion while the reader finished its remaining work. That
case's logs do not distinguish a timeout from a panic after completion.

Verified the current result-preservation paths with
`just test-cli commands::wrap::exec::reader_join commands::wrap::output_worker commands::wrap::run_scope`:
46 tests passed. The suite includes Claude/Codex successful-result preservation,
all nine structured provider identities with held completion callbacks,
nonzero/provider-error controls, distinct real panics, and bounded output loss.
This is synthetic regression evidence, not historical CPU or terminal evidence.

Corrected the timeout topic and the reader-budget WHY comment: the logs do not
prove that 120 s is twice a measured maximum post-exit terminal stall. No
timeout constant, exit policy, or terminal-abandonment behavior was changed.

### Parsing and rendering computation spike — 2026-10-07

Measured six native Claude answers corresponding to the identifiable final
answer/result pairs above. Reconstructed minimal assistant and successful
result JSON envelopes around each answer; these are representative inputs,
not the original full wire records. Added synthetic 10x and 100x repetitions
of the October 6 answer to inspect scaling. The result envelope also includes
the answer, so each parsing operation processes it in two JSON records.

The temporary example used the current production Claude parser with
`NullSemanticSink`, checked its success and nonempty answer, and constructed
both its snapshot and final summary. A combined stage additionally decoded
each line to `serde_json::Value`, approximating the reader's extra decode for
signal observation; actual signal matching was excluded. Rendering used
`FinalMessage` (Darkmatter Markdown, with `Prose` as its fallback) into a
string with cached options, images disabled, and fixed width 80. No terminal
output delivery, live semantic sink, logging, hooks, or child process ran in
the timed operations.

Each stage ran in its own process, with one cold operation, five warmups,
and an adaptive batch targeting roughly 350 ms, bounded to 5–20000 iterations.
Measured operation wall time with `Instant`; collected whole-process user and
system CPU with the parent Python process's child-resource accounting. CPU
totals include startup, input loading, cold work, and warmups, so they are not
per-operation CPU measurements. This is an unoptimized Cargo `dev` build on
this macOS host, with no deliberately induced CPU contention, no affinity
controls, and no cross-OS claim. It is a baseline computation spike, not a
reproduction of historical host load or terminal backpressure.

| Work per answer/result pair | Six incident answers, mean wall time range |
| --- | --- |
| Generic JSON decoding | 0.017–0.042 ms |
| Provider parsing, snapshot, and final summary | 0.029–0.055 ms |
| Generic decoding plus provider parsing and summaries | 0.046–0.095 ms |
| Markdown rendering of the answer into a string | 1.24–3.18 ms |

| October 6 answer size | Combined decoding/parsing/summaries | Markdown rendering |
| --- | --- | --- |
| 4838 characters | 0.089 ms | 2.97 ms |
| 48380 characters (10x synthetic) | 0.63 ms | 28.73 ms |
| 483800 characters (100x synthetic) | 5.97 ms | 284.46 ms |

The baseline does not support ordinary final-answer JSON decoding alone as
the explanation for delays of tens of seconds. Current rendering is also
fast at the real incident sizes; this measurement does not reproduce the
previous uncached per-block font discovery or a blocked terminal write.
Host scheduling, callbacks, signal matching, logging, accumulated parser
state, and the original full records remain outside the measurement.

There is a possible CPU optimization in sharing the reader's generic JSON
decode with provider decoding, but that decode costs only tens of microseconds
at these answer sizes. A refactor is not justified as a remedy for this
incident by the measurements. Timed observations at the real reader boundaries
are the better next investigation. The current 120 s processing budget was
retained; the 5 s settled-pipe-wait budget is separate. No production code or
timeout constant changed in this spike.

Artifacts on this host:

- `/tmp/claudine-completion-cpu-spike.rs`: temporary benchmark source, removed
  from the package's examples directory after execution;
- `/tmp/claudine-completion-cpu-inputs.json`: local private answer fixtures;
- `/tmp/claudine-completion-cpu-results.json`: all 32 stage measurements;
- `/tmp/claudine-completion-cpu-build.log`: development-build result.
