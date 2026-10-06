---
created: 2026-10-06
phase: 1
total_phases: 6
agent: claude/sonnet
yolo: true
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages: []
---

# Plan: a slow stream reader is reported as what it was

## Summary and definition of done

The spec (`2026-10-06-stream-reader-join-timeout`) records a successful agent
run reported as "Stream parser thread panicked". The checkout already has typed
join outcomes (`JoinOutcome`, `ReaderStall`, `ReaderProgress`), the 5 s / 250 ms
/ 120 s `ReaderBudget`, a shared `readers_since` clock, and `settle_parser`
(`cli/src/commands/wrap/exec/reader_join.rs`). The semantic path therefore
already distinguishes panic from timeout. What remains:

| Gap | Where | Requirement |
| --- | --- | --- |
| Captured and inherited paths still use fallback-only `join_with_timeout_or` (`exec/mod.rs`, `spawn/captured.rs`) | cli | R1, R2, R3 |
| Reader panic/timeout is not visible on stderr without tracing on those paths | cli | R1 |
| A stall *inside* a parser callback before the final line is unrecoverable: the reader holds the only parser while a render callback blocks on terminal I/O | cli + lib | R3 |
| Trailer, summary, warnings, ticker shutdown and lock acquisition still write synchronously through the `StreamOutput` lock (`stream_io.rs`, `policy.rs::emit_stream_summary`) | cli | R4 |
| No explanation of the WezTerm stall in `docs/` | docs | R5 |
| `format_internal_duration` prints 60 s..3599 s as a bare number | cli | R6 |
| `FinalMessage::render` ignores the supplied terminal's width and uses default `TerminalOptions`, so the sequence gutter inset only reaches the plain fallback | lib | R7 |
| `an_early_wait_error_still_reaps_the_whole_tree` failed once under load | lib | R8 |

**Done when:**

- every R1 to R8 acceptance item has a test named in Phase 6's checklist, passing under `just test` in `claudine`;
- a permanently blocked output sink cannot stop the wrapper from finishing a run, firing lifecycle events, and returning through `shutdown::finish`;
- an exit-0 run with a parsed result keeps its exit code and result when output stalls, and shows a warning on stderr;
- the `docs/topics` pages (timeouts, signal-handling, composition) and affected READMEs describe the shipped behavior;
- the work stops at "implementation complete, ready for review" (no move to `_completed`, no commit unless asked).

Input Robustness Matrix: not applicable. No parser, manifest, or config reader changes.

## Phase 1 — Rulings, baseline, and the one spike

### Necessary Rules

These need an author ruling before Phase 4. Each lists the recommendation the plan assumes if the author does not object.

- [x] **Ruling A — delivery strategy.** The spec's Open Question. Plan assumes **option 1**: one bounded output worker per invocation; after a stall, terminal frames for that invocation are rejected and the loss is recorded in the session record. Accepting it means terminal visibility is lost for the rest of the invocation on the affected sink. Options 2 and 3 are out of scope.
- [x] **Ruling B — what "invocation" spans.** A `claudine compose` loop runs many iterations in one process. Assumed: the worker and its disabled flag live per *iteration* run (so one stalled iteration cannot write into the next), but a sink found stalled stays silent for the remaining iterations of the process; no new writer thread is created for a sink already marked stalled. This is what "no unbounded new writer per iteration" requires.
- [x] **Ruling C — one worker, two sinks.** Assumed: a single worker owns both stdout and stderr frames, in order, so warnings, trailer, and summary share one stall decision. A stderr-only stall therefore also disables stdout delivery. Alternative (worker per sink) doubles threads; not assumed.
- [x] **Ruling D — queue bound and overflow.** Assumed: bounded by frame count (the spec requires bounded memory, not a number; pick a constant, e.g. 256 frames, plus a byte cap). Overflow drops *terminal presentation frames only*, counts them, and reports the loss once; provider bytes, semantic events, and the result are never dropped. In a JSON stdout mode only whole frames are dropped so the stream stays valid JSON.
- [x] **Ruling E — output budget for main-thread writes.** Assumed: trailer, summary, warnings, and ticker shutdown wait on the worker against the same 120 s clock that started at reader cleanup (the spec's "output budget"), clamped to the time left; once delivery is disabled they return immediately. Lifecycle actions and messaging drain keep their own deadlines.
- [x] **Ruling F — inherited-path failure kind.** Assumed: no new `error_kind` beyond `stream_reader_timeout` and `parse_failure`; a failed forward on the inherited path is a stderr warning plus a session-record field, never a provider failure.
- [x] **Ruling G — spike scope.** The R5 sample runs on one host (this macOS host, WezTerm). Linux and Windows terminal behavior is not measured here; a wider campaign is for the author to commission separately.

### Tasks

- [x] **Confirm rulings.** Record the author's answers to A to G in an "Implementation log" section at the end of this file. If A is rejected, stop; the design in Phase 4 changes.
- [x] **Baseline.** From `claudine/`, run `just test` and `just lint`; note pre-existing failures so later phases are not blamed for them. Load `rust-testing`, `os`, and `rust-devops` skills only if a task below needs them.
- [x] **Spike: R5 boundary sample.** Run once, before Phase 4.
    - on this host under WezTerm, run a fake agent emitting a long stream (large final message, escape sequences, hyperlinks) while the terminal pane is paused or scrolled-back, and sample where the reader blocks: pipe read, render, `StreamOutput` lock, or `StdoutWriter::write`
    - capture with `sample`/`lldb` thread backtraces or temporary `tracing` at those boundaries (do not commit instrumentation)
    - record: confirmed observations, leading explanation, unresolved cause
    - exit criterion: a paragraph usable verbatim in the timeouts doc. An unreproduced terminal defect does not block Phases 2 to 5. A confirmed Claudine contribution gets its own task appended to Phase 4.

**Checkpoint:** rulings recorded, baseline green or known-red list written, spike paragraph written.

## Phase 2 — Independent fixes (R6, R7, R8)

These touch disjoint files and can start as soon as Phase 1's baseline is done.

### Wave 1 (parallel)

- [ ] **Unit suffix (R6).**
    - in `cli/src/commands/wrap/exec/timeouts.rs`, make `format_internal_duration` always emit a unit: seconds below 60, `Nm` (and `NmSs` if remainder is nonzero) from 60 to 3599, `NhMm` from 3600
    - table test for 59, 60, 120, 3599, 3600 asserting exact strings; check callers' messages and any snapshot or doc text that quotes the old bare form
    - drift pass over the function's `///` docs
- [ ] **Gutter inset in Markdown (R7).**
    - in `lib/src/render/final_message.rs`, make `FinalMessage::render` derive the Markdown width from the supplied `Terminal` (`term.width()` honoring `fixed_width`) rather than independently detected defaults, so cached `with_options` options and default options both honor the inset; keep capability fallback and `NO_COLOR` behavior
    - check how `new_assistant_stream_inset` (`cli/src/commands/wrap/exec/mod.rs`) and `AssistantStream` pass options, so the streaming and final paths agree
    - tests: prose and a fenced code block inside an inset-2 terminal, a terminal narrower than the inset, cached options (`with_options`), and plain (non-TTY) output unchanged
    - use biscuit-terminal components; do not hand-roll wrapping
- [ ] **Reap flake (R8).** Investigate `an_early_wait_error_still_reaps_the_whole_tree` (`lib/src/composition/sequence/task/tests.rs`).
    - reproduce: run it under load (`just test` for lib alongside a CPU hog, or nextest with high `--test-threads`), capture the 32 s LEAK-FAIL path
    - decide whether the leak is real (production reap code in the task shell) or the fixture's timing (`BackgroundedDescendant`, `REAP_DEADLINE`, the `sleep 1` marker in the body)
    - fix at the cause; if the fixture is at fault prefer a descendant-ready handshake. Do not weaken `assert_reaped` or the marker assertion, and add no retries
    - record the finding in the implementation log

**Checkpoint:** focused tests for each pass; R8's test passes repeatedly (say 20 runs) under load.

## Phase 3 — Reader outcomes on every spawn path (R1, R2)

Depends on Phase 1 rulings F; independent of Phase 2.

### Wave 2 (sequential; shared helpers first)

- [ ] **Shared outcome reporting.** Add to `reader_join.rs` a reporter that turns a `JoinOutcome` into (a) an error kind (`parse_failure` for panics with the payload string, or an explicit non-string-payload message; `stream_reader_timeout` for timeouts with the observed state) and (b) a stderr diagnostic written without tracing. Confirm `panic_message` covers non-string payloads and say so in its message.
- [ ] **Retire fallback-only joins.** Remove `join_with_timeout_or`/`join_with_timeout` from `exec/mod.rs`; migrate their callers.

### Wave 3 (parallel after Wave 2)

- [ ] **Captured path** (`spawn/captured.rs`).
    - track both readers with `ReaderProgress` and join them with `join_reader` under one `ReaderBudget` and one `since` clock started after process-tree teardown
    - keep bytes already collected: a timed-out reader's partial buffer must be readable (shared buffer rather than the thread's return value) and the result flagged as incomplete capture
- [ ] **Inherited path** (`spawn/inherited.rs`).
    - same tracked joins and shared clock; a forwarding stall or failure is a warning plus session-record field and never a provider failure (Ruling F)
- [ ] **Semantic path** (`spawn/semantic.rs`).
    - use the shared reporter for the stderr reader and the `settle_parser` warning so all three paths format identically
- [ ] **Boundary tests** with injected short `ReaderBudget` and synchronization signals (no real 120 s waits), in `reader_join/tests.rs` and `spawn/tests/{captured,inherited}.rs`:
    - real panic with a string payload and a non-string payload vs each timeout state (`PipeOpen`, `Processing`)
    - held-open pipe: short cap applies only after a continuous 250 ms read wait and at or after 5 s; a reader between buffered lines does not count
    - shared stdout/stderr deadline: the two joins together wait no longer than one limit; a state change does not reset it
    - completion observed before declaring timeout at the boundary
    - stderr visibility asserted from captured stderr, tracing off

**Checkpoint:** `just test` for claudine-cli passes; all three paths report the same kinds and messages.

## Phase 4 — Terminal delivery separated from parsing (R3, R4)

Depends on Phase 1 Rulings A to E and the spike. This is the structural phase; do not parallelize inside it except where marked.

### Wave 4 (design and core type)

- [ ] **Output worker.** Add a CLI-owned `OutputWorker` (own module under `cli/src/commands/wrap/`, next to `stream_io.rs`):
    - one thread, bounded queue of whole frames (Ruling D), carrying stdout and stderr frames in order (Ruling C)
    - `submit` never blocks and never holds a run-state lock; full queue drops presentation frames and counts them
    - `finish(deadline)` waits on completion up to the shared deadline; on expiry sets a *disabled* flag, abandons the thread (no new writer is ever created for that sink, Ruling B), and records an incomplete-output diagnostic
    - after disabling, `submit` rejects instantly; a late in-progress write may finish but cannot be followed by more
    - portable: no Unix-only calls; synthetic sink behind a trait for tests
- [ ] **Result snapshot.** Add a lock-free-with-respect-to-terminal-I/O snapshot of parser state (final result, error kind, exit data) that is published *before* a rendered frame is submitted, so `settle_parser` can read it even when the reader holds the parser inside a callback. Provider parsing, result semantics, and Markdown rendering stay in the library; only the coordination lives in the CLI.

### Wave 5 (wiring; sequential)

- [ ] **Reader wiring.** Route `OutputTextCallback`/`ReasoningCallback` and `AssistantStream` writes through the worker (`spawn/semantic.rs`, `StdoutWriter`/`StreamOutput` in `stream_io.rs`). Ordering and content for an unblocked sink must be byte-identical to today; keep the biscuit-terminal component path, capability fallbacks, and `NO_COLOR`.
- [ ] **Main-thread writes.** Move the ticker shutdown, warnings, trailer, summary rendering, `StreamOutput` lock acquisition, and panic reporting through the worker with the Ruling E deadline (`emit_stream_summary` in `policy.rs`). No main-thread write may reach the terminal directly.
- [ ] **Frozen summary.** Freeze the summary and publish exactly one `session_end` before lifecycle completion. After cutoff a late reader must not mutate the summary, emit semantic events for the completed run, or write into the next iteration (generation token checked by the sink and the semantic emitter).
- [ ] **Settle on the snapshot.** Change `settle_parser` to use the snapshot where the slot is empty because the reader stalled mid-line (see Phase 5 for the outcome rules).
- [ ] **Spike follow-up.** If the spike confirmed a Claudine-side contributor (write size, escape output, queries), fix it here.
    - confirmed contributor from the Phase 1 spike: per-block Markdown rendering rebuilds `Terminal::default()` (darkmatter `terminal_options_from_terminal_options`), and `biscuit_terminal::discovery::fonts::font_name` (and its `fallback_font_name_scan` → `query_iterm2_font_name`) is uncached, spawning a `defaults read com.googlecode.iterm2` subprocess per Markdown block on the reader thread. Cache or hoist the detection so a large final message does not multiply subprocess spawns while the reader holds the parser.

### Wave 6 (tests)

- [ ] **Blocked sink tests** with a controllable sink: block permanently; assert the run completes, lifecycle events fire, `shutdown::finish` returns, and warnings, tickers, trailer, and summary do not hang. Release the sink at teardown and assert no late event or output reaches a following iteration.
- [ ] **Slow line, unread result.** Block delivery on a middle line with the final result line still unread; parsing still produces the complete successful result.
- [ ] **Queue bound.** Overflow drops and counts presentation frames, memory stays bounded, provider bytes and the result are intact; JSON stdout stays valid.
- [ ] **No detached-writer growth.** Across several simulated iterations on a stalled sink the writer-thread count does not grow.

**Checkpoint:** focused tests green on macOS; sink tests written for portability (no Unix-only gating).

## Phase 5 — Success preservation matrix (R3)

Depends on Phase 4's snapshot.

- [ ] **Outcome rules in `settle_parser`/policy.** Implement exactly:
    - parsed successful result plus child exit 0: keep exit code and result, warn that output may be incomplete, at any stall point
    - no complete result at cutoff: `stream_reader_timeout` as an incomplete-stream failure; never invent a result from unread lines
    - preserve an already-established provider failure, nonzero exit, signal interruption, or provider timeout; the reader warning never replaces its primary cause
    - a real panic stays `parse_failure`
    - a stderr-reader failure alone stays a warning unless its provider-specific bridge is needed for the outcome (check Codex app-server, Pi RPC, and OpenCode bridges under `exec/`)
- [ ] **Matrix test.** One table-driven test across {exit 0 + result, exit 0 no result, result says failure with exit 0, nonzero exit, interrupted, provider timeout} x {reader timeout, panic, rendering stalled}: assert resulting exit code, `error_kind`, and warning presence. None becomes success because rendering timed out.
- [ ] **Per-path semantics.** Captured: partial bytes remain available and incomplete capture is flagged. Inherited: forwarding failure is distinguished from provider failure. Include Codex's slow-reader case (6.5 s in the logs) as a fixture with injected budgets.

**Checkpoint:** matrix passes; `just test` for claudine-cli passes.

## Phase 6 — Docs, drift pass, and verification

### Wave 7 (parallel)

- [ ] **Topic docs** (audience: a developer new to the repo; lead with what the reader can do, a compact example per rule, a Mermaid diagram for the post-exit flow: teardown, shared clock, reader joins, worker `finish`, `session_end`):
    - `docs/topics/timeouts.md`: the post-exit wait as an internal cleanup budget separate from the two provider timeout rules; the three kinds of outcome; the R5 findings; correct the claim that a bounded join alone prevents a blocked terminal from hanging the wrapper; step-timeout durations with units. No link to the fix by name.
    - `docs/topics/signal-handling.md`: interaction of delivery shutdown with Ctrl+C and termination
    - `docs/topics/composition.md`: sequence gutter inset reaching rendered Markdown
    - affected READMEs (`claudine/README.md`, `cli/README.md`, `lib/README.md`) where they describe these behaviors
    - the claudine skill (`.claude/skills/claudine/`) if the topic summaries there change
- [ ] **Comment drift pass.** For every symbol whose behavior changed, update `///` and `//!` docs and inline comments; delete HOW-narration; report any drift found and how it was resolved.
- [ ] **Implementation log.** Record departures from the spec, the rulings, the R8 finding, and the spike result at the end of this file.

### Wave 8 (sequential)

- [ ] **Verification.** From `claudine/`: focused tests per phase, then `just test`, then `just lint`. Check a repo-wide grep for `join_with_timeout_or`, bare `thread::sleep` readers' state assumptions, and `unwrap_or` on join results. Confirm the portable synthetic tests carry no `cfg(unix)`; Unix-only process-tree fixtures keep honest scope. Do not run `cargo fmt`; no extra CI cells.
- [ ] **Acceptance checklist.** Tick one test per item: R1 panic vs timeout and stderr visibility; R2 cap, settle, shared deadline; R3 matrix and slow-line case; R4 blocked sink and no cross-iteration leak; R5 doc paragraph; R6 five boundaries; R7 prose, code, narrow, cached, plain; R8 reliable under load.
- [ ] **Hand-off.** State "implementation complete, ready for review". Do not move the spec to `_completed` and do not commit unless explicitly asked.

## Implementation log

### Phase 1 rulings (recorded 2026-10-06)

The implementation session is non-interactive, so the author could not be
consulted. Rulings A to G are adopted as the plan's documented defaults (the
recommendation each ruling names), and are flagged here for the author to
confirm or reject at review; a rejection of A reopens Phase 4's design.

- **A:** option 1 — one bounded output worker per invocation; after a stall,
  terminal frames for that invocation are rejected and the loss is recorded in
  the session record.
- **B:** worker and disabled flag live per iteration run; a sink found stalled
  stays silent for the process's remaining iterations; no new writer thread is
  created for a sink already marked stalled.
- **C:** one worker owns both stdout and stderr frames, in order; a stderr-only
  stall disables stdout delivery too.
- **D:** bounded by frame count (constant, e.g. 256 frames, plus a byte cap);
  overflow drops terminal presentation frames only, counted and reported once;
  provider bytes, semantic events, and the result are never dropped; JSON
  stdout mode drops only whole frames.
- **E:** trailer, summary, warnings, and ticker shutdown wait on the worker
  against the 120 s clock started at reader cleanup, clamped to the time left;
  once delivery is disabled they return immediately.
- **F:** no new `error_kind`; a failed forward on the inherited path is a
  stderr warning plus a session-record field, never a provider failure.
- **G:** the R5 sample ran on this macOS host under WezTerm only.

### Phase 1 baseline (recorded 2026-10-06)

From the worktree's `claudine/`: `just lint` exit 0; `just test` 8373 passed,
9 skipped, 0 failures. No known-red list. (An earlier accidental run against
the main checkout — a relative `cd` hijacked by this host's `CDPATH`, a trap
already recorded in the `os` skill's macos.md — showed 8366 passed there; the
difference of 7 is Implementation 1's `reader_join` tests.)

### Phase 1 spike result (R5, recorded 2026-10-06)

Reproduced the incident shape on this host under WezTerm using XOFF (Ctrl-S,
IXON flow control) on the pane as a stand-in for a terminal that stops
accepting output, a fake `claude` emitting stream-json with a 52 KB final
assistant message, and `sample(1)` thread backtraces. Confirmed observations:

1. The stdout reader blocks in `write(2)` inside `StdoutWriter::write`
   (`stream_io.rs:245`), called from the `OutputTextCallback`
   (`semantic.rs:382`) *inside* `ClaudeSemanticStreamParser::feed_line` →
   `handle_assistant_message` → `flush_assistant_text` →
   `LiveSemanticSink::on_semantic_event` — mid-line, holding the only parser.
   This is the incident's line-816 shape.
2. During the join the main thread waits in `join_reader`
   (`reader_join.rs:208`); the reader's state is `Processing`, so the 120 s
   drain limit applies.
3. After the 120 s join timeout the main thread blocks indefinitely in
   `StreamOutput::emit_stderr_line` (`stream_io.rs:217`) on
   `__psynch_mutexwait`, because the abandoned reader still holds the
   `StreamOutput` lock. The wrapper hung ~87 s past the join timeout and
   would hang forever: R4's gap is confirmed live.
4. On XON both threads completed within the same second, matching the
   incident's "resumes within microseconds" evidence. Because the reader then
   fed the result line and filled the parser slot before `settle_parser` ran,
   the run kept exit 0 with `exit_reason: stream_reader_timeout` in the
   session record; had the slot still been empty it would have been the
   failure path.
5. Confirmed Claudine-side contributor: per-block Markdown rendering rebuilds
   `Terminal::default()` via darkmatter's
   `terminal_options_from_terminal_options`, and
   `biscuit_terminal::discovery::fonts::font_name` (plus its
   `fallback_font_name_scan` → `query_iterm2_font_name`) is uncached and
   spawns a `defaults read com.googlecode.iterm2` subprocess per Markdown
   block on the reader thread. 889 of 2217 reader-thread samples sat in this
   detection path (262 blocked in `poll(2)` on the spawned process). A large
   final message therefore keeps the reader busy for seconds even with a
   healthy terminal, holding the parser throughout. Fix task appended to
   Phase 4's spike follow-up.

Unresolved cause: what made the user's real WezTerm stop accepting output is
not established; XOFF was a controlled stand-in, not a reproduction of a
WezTerm defect. No WezTerm defect is asserted.

Spike paragraph for the timeouts doc (usable verbatim):

> When the agent has exited but its output stream is still being drained, the
> wrapper's reader thread can stall inside the terminal write itself: it
> renders each stream line and writes the rendered frame to the terminal from
> inside the stream parser's callback, holding the parser while it does so. A
> terminal that stops accepting output (observed locally by applying XOFF flow
> control to a WezTerm pane) then blocks the reader in the kernel write, with
> the result line still unread. The bounded reader join (120 s for a reader
> caught processing) caps that wait, but after the join times out the main
> thread still writes warnings, the trailer, and the summary through the same
> output lock the abandoned reader holds — so a permanently blocked terminal
> can still hang the wrapper after the join. Separately, rendering a large
> final message is slowed further because each Markdown block rebuilds the
> detected terminal options, and font detection can spawn a subprocess per
> block on the reader thread.
