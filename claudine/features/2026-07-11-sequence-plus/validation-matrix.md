# Sequence Plus — Acceptance Validation Matrix

Phase 13 handoff artifact. Each row maps one [`spec.md`](spec.md) acceptance
criterion to the tests that prove it and the command that runs them. Skips are
recorded with a reason rather than omitted.

## Evidence provenance

Review-5 finding 6 asked for one thing: every claim in this file must name the
**revision** it was measured against, the **host**, the **verification level**,
the **command**, and the **result** — and historical evidence must not sit
undifferentiated beside current-tree evidence.

Three revision anchors are used throughout. Cite one by name in any new claim.

| Anchor | Revision | Round | Date |
|---|---|---|---|
| **R0** | `be2d100a6` + uncommitted working tree | review 5 | 2026-07-19 |
| **R-1** | `be2d100a6` (`432fd25ab` for the gate record) | review 4 | 2026-07-18 |
| **R-2** | pre-review-4 tree | review 3 | earlier |

**R0 contains no commits from the review-5 round.** Findings 1–5 were
implemented into the working tree and nothing was committed; `git status` at the
time of writing shows 24 modified files and 6 untracked ones. Anything below
attributed to R0 is therefore evidence about a tree that exists only in a working
copy.

Hosts: **R0 and R-1** are macOS 26.5.2 / Darwin 25.5.0 / arm64, 16 logical cores,
on branch `error-prop-and-file-resolution`. The Linux subsection is a
`rust:latest` Docker container on the same host and is **R-2**.

Two words are used precisely and are not interchangeable:

- **Executed** — a command ran to completion on a real host at the named
  revision, and its result is recorded.
- **Type-checked** — `cargo check` accepted the source. Nothing linked, nothing
  ran. This is never evidence of behavior.

## Commands

| Command | Scope |
|---------|-------|
| `just test` (from `claudine/`) | L1 — lib unit + CLI integration |
| `just test` (from `biscuit-file/`) | L1 — `ListFormat` |
| `just test` (from `darkmatter/`) | L1 — `set(…)`, name coercion, `last(list)` |
| `just test-l2 --no-fail-fast` (from `claudine/`) | L2 — real terminal (tmux) |
| `just lint` (each of the three areas) | clippy |
| `just test-l3` (from `claudine/`) | L3 — real OS keyboard via WezTerm + cliclick; refuses to run unattended |
| `just check-windows` (from `claudine/`) | Windows **type-check** of lib + CLI test targets — the only gate that compiles the Windows-only suites, and the only one; it does not link or run them |

The bare `just test-l2` recipe fail-fasts at the first failure; `--no-fail-fast`
is required to get real coverage of the 144-case L2 suite.

Gate results live under "Gate runs" below, split by revision anchor: R0
(2026-07-19) is the current tree; R-1 (2026-07-18) has its full verbatim record
in [`gate-run-2026-07-18.md`](gate-run-2026-07-18.md). There is no run of any
kind behind `just check-windows` — it type-checks and stops.

## Test inventory

Counts are `#[test]` occurrences **at R0** unless a row says otherwise.

| File | Level | `#[test]` count |
|------|-------|-----------------|
| `claudine/lib/src/composition/sequence/tests.rs` | L1 | 104 |
| `claudine/lib/src/composition/sequence/task/tests.rs` | L1 | 88 (was 73 at R-1; +15 from review-5 findings 1–2) |
| `claudine/lib/src/composition/sequence/preflight/tests.rs` | L1 | 37 |
| `claudine/lib/src/render/task_stream/tests.rs` | L1 | 24 |
| `claudine/lib/src/composition/runtime_state/tests.rs` | L1 | 13 |
| `claudine/cli/src/commands/wrap/exec/termination/coordinator/tests.rs` | L1 | 17 |
| `claudine/cli/src/commands/compose/interrupt.rs` | L1 | 7 |
| `claudine/cli/src/commands/wrap/exec/termination/handle.rs` | L1 | 4 |
| `claudine/cli/tests/test_placement.rs` | L1 (guard) | 11 |
| `claudine/cli/src/commands/wrap/sequence/tests.rs` | L1 | 7 |
| `claudine/cli/src/commands/wrap/sequence/jit/tests.rs` | L1 | 6 |
| `claudine/cli/tests/sequence_errors_cli.rs` | CLI E2E | 29 |
| `claudine/cli/tests/sequence_cli.rs` | CLI E2E | 28 |
| `claudine/cli/tests/sequence_sources_cli.rs` | CLI E2E | 22 |
| `claudine/cli/tests/sequence_groups.rs` | CLI E2E | 20 |
| `claudine/cli/tests/composition_outputs.rs` | CLI E2E | 14 |
| `claudine/cli/tests/sequence_jit.rs` | CLI E2E | 13 |
| `claudine/cli/tests/sequence_overlay_pty.rs` | L1 (PTY) | 7 |
| `claudine/lib/src/composition/sequence/task/shell.rs` | L1 | 5 — `Utf8Stream` decoder units (new at R0) |
| `claudine/cli/tests/level2_sequence_task_stream_capture.rs` | L2 | 9 (was 7 at R-1; +2 from review-5 finding 2) |
| `claudine/cli/tests/level2_windows_sequence_ctrl_c.rs` | L2 (Windows host) | 1 — type-checked by `just check-windows`, **never executed** |
| `claudine/cli/src/commands/wrap/exec/termination/windows.rs` | L1 (Windows host) | 2 — type-checked by `just check-windows`, **never executed** |
| `claudine/cli/tests/level3_sequence_ctrl_c.rs` | L3 (macOS) | 1 — **never executed at R0** |
| `claudine/cli/tests/level3_linux_sequence_ctrl_c.rs` | L3 (Linux X11) | 1 — new at R0, **never executed on any host** |
| `claudine/cli/tests/level3_windows_sequence_ctrl_c.rs` | L3 (Windows) | 1 — new at R0, **never executed on any host** |
| `biscuit-file/lib/src/list_format.rs` | L1 | 22 (+3 doctests) |

Four entries are new or grew at the review-4 round, and each exists to close a
specific finding:

- **`termination/coordinator/tests.rs` (17) and `termination/handle.rs` (4)** —
  the host-independent half of the Windows interrupt machinery: press ladder,
  per-registration escalation, flag fan-out, weak-flag pruning, handler
  refcount edges, concurrent registration, and handle drop order. Deliberately
  generic over the child payload so it runs where Win32 does not, which is why
  it is the only Windows-shaped code in this feature with executed evidence.
- **`compose/interrupt.rs` (7)** — review-4 finding 8. Cross-platform tests over
  `press_rung` and `classify_console_interrupt`, the two decision points of the
  now-real Windows body of `install_user_interrupt_guard`. They run on every
  host; the Win32 handler registration they feed does not.
- **`wrap/sequence/tests.rs`** and **`sequence/task/tests.rs`** — review-4
  finding 2. `run_was_interrupted` is now driven directly with non-`130` exit
  codes (`137`, `1`, `0xC000013A`) against the shared flag, and
  `an_interrupted_provider_is_recorded_as_interrupted_whatever_its_exit_code`
  plus `exit_130_without_the_interrupt_flag_is_an_ordinary_failure_here` pin
  both directions of the derivation.
- **`level2_windows_sequence_ctrl_c.rs` (1)** and **`termination/windows.rs`
  (2)** — the Windows-host suites. `just check-windows` type-checks them; no
  host has ever run them. See "Windows: what compiles versus what has run".

## Criterion → evidence

### AC1 — Retained behavior stays covered

Scalar/object steps, document-level inline-compose, fail-fast precedence,
missing-property aggregation, dry-run, Ctrl+C exit `130`.

- `sequence/tests.rs`: `inline_scalar_list_normalizes_correctly`,
  `inline_object_list_requires_name`, `fail_fast_false_from_frontmatter`,
  `fail_fast_wrong_type_fails`
- `clean_break::characterize_current_overlay_keys` pins the post-break overlay
  key set (Phase 1 characterization, updated in Phase 3)
- `sequence_cli.rs` — dry-run target behavior, aggregate missing properties
- Ctrl+C `130`: `task/tests.rs::parallel_groups` signal fan-out cases +
  `level3_wrap_ctrl_c.rs` (pre-existing) + `level3_sequence_ctrl_c.rs`
  (review-2 finding 5 — see "Level-3 sequence Ctrl+C fan-out"). **The
  OS-keyboard leg of this criterion has not been executed since R-2.** It was
  guard-blocked at R-1 (see [`gate-run-2026-07-18.md`](gate-run-2026-07-18.md)
  § Gate 4) and not attempted at R0. AC1's keyboard evidence therefore rests on
  R-2's green — two trees behind the one the L1/L2 gates certify, and behind
  review-5 finding 1's rewrite of the termination path it exercises. Linux and
  Windows L3 fixtures exist as of R0 but have **never run**. The procedure that
  discharges all three is [`l3-ctrl-c-runbook.md`](l3-ctrl-c-runbook.md).
- The exit-code derivation itself is now covered rather than assumed
  (review-4 finding 2). `run_was_interrupted(exit_code, interrupted)` in
  `wrap/sequence/mod.rs` is the single decision point for both the step path
  (`iterate.rs`) and the group prompt-task path (`task_run.rs`), and it is
  driven at L1 with `137`, `1`, and `0xC000013A` against the shared flag — the
  exits the Unix `SIGKILL` rung and the two Windows rungs actually produce.
  `130` remains sufficient but is no longer necessary.

### AC2 — Typed errors for every rejected construct

- `sequence_errors_cli.rs` (29 cases, **ungated**) — executable exclusivity,
  reserved writes, invalid formal states, empty static lists, suffix grammar,
  offsets/operators, cycles, nesting, write-back collisions, `max_parallel`,
  timeout, group `loop`
- `sequence/tests.rs`: `object_step_rejects_two_executables`,
  `shell_step_rejects_prompt_only_task_option`,
  `object_step_extracts_state_and_rejects_reserved_state_key`,
  `external_list_shape_rejected`, `group_loop_rejected`, `empty_list_fails`
- `preflight/tests.rs` — `SequenceReferenceCycle` full chains,
  `SequenceNestedSequence`, `SequenceUnsupportedConstruct`
- Reserved-key **writes**: `composition_outputs.rs`
- Typed `#[source]` causes (not stringified) forced by the `error_guards` suite

### AC3 — Source/list-form coverage

- `biscuit-file/lib/src/list_format.rs` — 22 unit tests: every `ListFormat`
  variant, ambiguous precedence, quoted CSV/TSV, CRLF, Unicode, scalar,
  whitespace-only
- `sequence/tests.rs::dynamic_sources` (11), `::grammar_tests` (21) — typed
  expression arrays bypass classification; numeric/boolean coercion; `null`
  rejection; dynamic empty-list success vs. static empty-list error

### AC4 — File sources and `FileReference`

- `sequence_sources_cli.rs` (22 cases) — YAML/JSON/JSON5/JSONL/NDJSON asserted
  **equal to each other** rather than five independent expectations, so a
  format-specific divergence cannot hide
- `sequence/tests.rs::source_resolution` (17) — plain/`@`/`!`/`~`/`vault:`/
  env-interpolated references, paths containing a space and an `@`, quoted
  suffix arguments
- `yaml_json_and_json5_offsets_produce_equivalent_plans`
- Resolution goes through `biscuit_file::FileReference::resolve_from`; no
  duplicated path rules (verified by source audit — see AC-notes below)

### AC5 — JIT semantics

- `sequence_jit.rs` (13) + `jit/tests.rs` (6) — runtime `set` and prior
  `outputs` visible to later serial tasks; reserved overlay retains highest
  precedence
- `preflight/tests.rs` — preflight-approved shell bytes **equal** executed
  bytes; `SHELL_UNAVAILABLE_ROOTS` rejects late-binding roots
- Live-disk re-read boundaries: `sequence_jit.rs` + the Phase 12 direct-YAML
  reload regression (`composition::load_yaml_document`)

### AC6 — Group semantics

- `task/tests.rs::parallel_groups` (14) — inverted completion delays for
  scheduling caps, snapshot isolation, mutation merge + duplicate-key warnings,
  declaration-ordered nested outputs, all-child completion after failure, no
  interactivity, signal fan-out
- `repeated_runs_produce_identical_results_regardless_of_completion_order`
- `parallel_execution_leaves_process_env_and_cwd_untouched`
- `sequence_groups.rs` (20) — real overlap through the real `claudine sequence`
  path (three 1s tasks < 2.5s; four capped at 2 ≥ 1.9s)

### AC7 — Rendering

- `render/task_stream/tests.rs` (24) — narrow widths, wrapping, Unicode, no
  color, palette cycling, invisible-bar alignment, stdout/stderr split,
  concurrent writes, no torn escapes. **Column assertions count characters, not
  bytes** (`│` is one column, three UTF-8 bytes).
- `task/tests.rs::group_framing` (6);
  `concurrent_siblings_never_split_one_frame_group`
- Geometry parity pinned at both levels:
  `a_serial_frame_and_a_parallel_frame_share_one_left_edge` (L1),
  `serial_and_parallel_group_frames_share_one_left_edge` (E2E), and
  `level2_serial_and_parallel_share_one_left_edge_at_narrow_width_in_tmux`
  (L2 — the only one that can see a frame overflow a real pane)
- **At R0 this criterion's color assertions are no longer ambient-dependent.**
  Review-5 finding 4 pinned the palette tests to an explicitly color-capable
  terminal and replaced 13 vacuous L2 escape-byte guards with row-anchored ones.
  Two of the L1 tests were passing vacuously before that change. See "Review-5
  round (R0)" § finding 4.

### AC8 — Cross-platform

- `task/shell.rs` is the only spawner: `cfg(windows)` `cmd /C` vs `sh -c`,
  `try_wait` polling (no `wait4`/signals). **Termination is no longer
  `child.kill()`** — review-5 finding 1 replaced it with tree-scoped ownership,
  and the two platforms are *deliberately* not identical there. See "Review-5
  round (R0)" § finding 1, including the open success-path asymmetry.
- CRLF handled twice independently: `ListFormat::normalize_newlines` inbound,
  `trim_transport_newline` outbound
- Durations via `harness::parse_timeout`; paths via `FileReference`
- Every `#[cfg(unix)]` in the sequence suites exists for exactly one reason — a
  `#!/bin/sh` provider stub. The nine ungated witness tests found in Phase 12
  were gated, and the six blocked-construct **message contracts** they assert
  gained ungated counterparts in `sequence_errors_cli.rs`, so Windows keeps the
  message contract and only the zero-launch witness is gated.

**AC8 is unblocked, not met.** That distinction is the single most important
statement in this document and should not be softened anywhere it appears.

macOS is the host. Linux is proven by a real-kernel `cargo check` + L1 run under
Docker. Windows evidence, stated at full strength, is exactly this:

| Windows claim | Status |
|---|---|
| Production lib + bin compile for `x86_64-pc-windows-gnu` | **Executed** — `cargo check`, exit `0` |
| Lib **and CLI test targets** type-check for `x86_64-pc-windows-gnu` | **Executed** — `just check-windows`, exit `0` (new at review 4) |
| Platform-sensitive production paths audited by reading | **Executed** — see "Windows source audit" |
| Any Windows code *running* | **Never.** No host, no emulation, no run |

`cargo check --tests` does not link and does not execute. It proves signatures,
types, and the absence of drift between a test and the API it calls. It proves
nothing whatever about `GenerateConsoleCtrlEvent`, `TerminateJobObject`, or
kill-on-close at runtime.

The "identical semantics on all three platforms" claim above holds for the
**task spawner**, where both branches are ordinary `Command` construction. For
**interruption** the honest statement is three-part: the Windows path is **fixed
in source, type-checked, and never executed.**
`wrap/exec/termination/windows.rs` routes `wait_with_signal_handling` through
`windows_wait_loop`, `register_sequence_interrupt_flag` gives `execute_sequence`'s
shared `interrupted` flag a Windows producer, and `register_compose_interrupt_handler`
gives the compose path the same treatment (review-4 finding 8). Every one of
those is design-complete and machine-checked for type correctness. None has run.

Only a native Windows run of `termination/windows.rs`'s Job-object regressions
and `cli/tests/level2_windows_sequence_ctrl_c.rs` closes AC8. That is review-4
finding 1 step 3, and it is out of reach from this host.

### AC9 — Test placement

Follows the Claudine placement contract: inline unit tests by default, sibling
`tests.rs` modules past the size threshold (`sequence/`, `task/`, `preflight/`,
`task_stream/`, `runtime_state/`), CLI integration tests for orchestration and
output contracts, L2 only where a real terminal is required.

Two tier corrections landed with review-2 finding 4. `level2_sequence_overlay_pty.rs`
was renamed to `sequence_overlay_pty.rs`: it drives an `expectrl` pseudo-TTY,
which `test_toolkit::Level::L1` defines as Level 1, so the `level2_` prefix
claimed a tier it never occupied. Its `require_level!` now names `Level::L1` and
its tests are `pty_sequence_*`. No tier prefix is used, deliberately — `_test`
and `_sanity` both filter out `level2_`/`level3_`/`browser_`/`real_`/`slow_`, so
any prefix would have left the binary running in no canonical recipe.

### AC10 — Documentation

- `claudine/docs/topics/flow-control/sequences.md` — full rewrite (the prior
  document was largely aspirational: `groups:`/`members:`, `command:`,
  `variables:`, `until`/`while`, list-shaped `template`, `parameters:` — none
  were ever implemented). The two meanings of `prompt` now lead the document.
- `.claude/skills/claudine/`: `SKILL.md`, `architecture.md` (new §Sequences),
  `cli-reference.md`, `timeline.md`
- No dependency changes → no `docs/dependencies.md` edit

### AC11 — Verification commands

See "Commands" above and "Known failures and skips" below. Executed results are
under "Gate runs", **current-first**: R0 (2026-07-19) is the tree as it stands;
R-1 and everything after it in that section is historical.

The spec's stated gates are `just test`, `just test-l2` where terminal or
concurrency behavior requires it, and `just lint`, from the `claudine` package
area. All three are green at R0 — `test-l2` modulo 4 named pre-existing
failures — and `just doctest` and `just check-windows` are green alongside them.
Formatting was checked read-only only; no `cargo fmt` write mode was used, and
`cargo fmt --check`'s diffs in untouched files are the known local-rustfmt drift,
not a gate result.

## Review-5 round (R0) — what landed and what it proves

Findings 1–5 of [`review-5.md`](review-5.md) were implemented into the working
tree on 2026-07-19. Finding 6 is this section and the provenance convention
above. **Nothing was committed**, so every claim here is about R0's working copy.

| Finding | Source state | Strongest evidence | Executed on |
|---|---|---|---|
| 1 — shell process-tree ownership | Implemented | L1 real-process (Unix) | macOS only |
| 2 — live parallel shell output | Implemented | L1 + L2 tmux | macOS only |
| 3 — Ctrl+C L3 evidence | Code added | **none — compile only** | nowhere |
| 4 — `NO_COLOR` gate | Implemented | L1 + L2 A/B | macOS only |
| 5 — preflight status ordering | Implemented | L1 + L2 | macOS only |

### Finding 1 — shell timeout and interruption now own the process tree

`SystemTaskShell` places each command in a tree it owns and tears the whole tree
down, rather than killing the immediate child:

- **Unix** — `CommandExt::process_group(0)`, then `kill(-pid, SIGTERM)`, a
  `TREE_KILL_GRACE` of 250 ms, then an unconditional `kill(-pid, SIGKILL)`.
- **Windows** — `CREATE_NEW_PROCESS_GROUP` plus a Job Object carrying
  `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`; `TerminateJobObject` on an explicit kill.

Output volume is bounded by **reusing** `runaway::CaptureVolumeCap` rather than
inventing a second cap, so the sequence path inherits the existing 50k-line /
32 MiB defaults; a trip raises the new `SequenceTaskShellRunaway` error. The
reader's shutdown wait is bounded by a `sync_channel` + `recv_timeout` against
`READER_SHUTDOWN_GRACE` (2 s), and the `JoinHandle` is **dropped, never
`join()`ed** — joining is precisely the unbounded wait the finding named.

Evidence, all **L1, executed on macOS at R0**: 5 `#[cfg(unix)]` real-process
tests in `task/tests.rs::shell_tasks` — backgrounded descendant holding stdout,
pipeline capture, nested-tree timeout, interrupt of a running tree, and stdout
flood tripping the volume cap — plus 3 `#[cfg(windows)]` `cmd`-based twins
(descendant, pipeline, nested timeout).

**The Windows twins have never run.** `just check-windows` type-checks them at
R0 and that is all. There is no Linux runtime evidence for this finding either;
the Unix tests would run there but no one has run them.

#### Two open judgment calls for Ken

1. `SequenceTaskShellRunaway` carries `task` and `command` but **not** the
   tripped line/byte counters, so the error says a cap was hit without saying
   which one or by how much.
2. **Unix and Windows disagree on the success path.** A shell command that exits
   `0` having deliberately backgrounded a daemon leaves that daemon alive on
   Unix, and has it killed at Job-Object drop on Windows. Either answer is
   defensible; the platforms disagreeing is worse than both. This needs a
   decision before ship — it is a user-visible semantic difference, not an
   implementation detail.

### Finding 2 — parallel shell output is live and attributed

stdout goes to the data channel (`TaskLiveOutput::append`) and stderr to the
status channel (`append_status`), so stderr is displayed and task-attributed but
**never enters `outputs`** — the spec's split preserved rather than widened.

Streaming composes with finding 1's chunked reader: the same read loop, one
extra push. Two decisions are load-bearing and worth recording because they are
not obvious from the diff:

- A separate `pending_status` fragment buffer keeps a partial stderr line from
  splicing into a stdout frame. Without it, two channels sharing one fragment
  buffer produce a frame that is half one stream and half the other.
- A new `Utf8Stream` decoder holds back incomplete multi-byte sequences across
  the 8 KiB chunk boundary. Without it, a multi-byte character straddling a
  chunk renders as replacement characters — a rendering defect that only appears
  at specific output lengths.

Evidence at R0, **executed on macOS**: 6 L1 real-process tests in
`task/tests.rs::shell_streaming` (channel split, first line before completion,
two-task arrival-order interleave, no torn lines at the sink, trailing-fragment
flush, multibyte survival), 5 `Utf8Stream` unit tests in `shell.rs`, and 2 new
L2 tmux tests — color and no-color.

### Finding 3 — Level-3 Ctrl+C evidence: **still open**

Code landed; **evidence did not**. No Level-3 test was executed on any platform
this session, on any tree. What exists at R0 is compile and filterset evidence
only.

New at R0:

| Artifact | What it is |
|---|---|
| `biscuit-test-harness/src/xdotool.rs` | Linux X11 XTEST key injection |
| `biscuit-test-harness/src/win_input.rs` | Windows `SendKeys`/`SendInput` injection |
| `claudine/cli/tests/level3_linux_sequence_ctrl_c.rs` | Linux L3 fixture |
| `claudine/cli/tests/level3_windows_sequence_ctrl_c.rs` | Windows L3 fixture |
| [`l3-ctrl-c-runbook.md`](l3-ctrl-c-runbook.md) | The attended procedure that produces the evidence |

Three things remain outstanding, and none of them is closed by the above:

1. The **macOS** L3 re-run against R0 — the existing green is R-2, an older tree.
2. A **Linux** L3 run. Never performed.
3. A **Windows** L3 run. Never performed.

The runbook exists precisely because only an attended desktop session can
discharge these. **Finding 3 is not closed.**

One asymmetry is deliberate and recorded rather than silently left: the macOS
fixture lacks the descendant-cleanup assertion its Linux and Windows siblings
carry. It was not added because it could not be re-verified from this session,
and an unverifiable assertion in a test that never runs is worse than its
absence.

### Finding 4 — the `NO_COLOR` gate, and why the exposure was nondeterministic

**L1.** The palette test `a_parallel_group_gives_each_task_its_own_palette_entry`
is pinned to `Terminal::new_forced()` instead of inheriting the ambient
environment, and the explicit no-color assertion builds a terminal with
`ColorDepth::None`. A third test,
`every_body_line_carries_its_own_tasks_bar`, was found **passing vacuously** on
a colorless host — it asserted over bar prefixes that were all empty strings —
and now also uses `new_forced()`.

**L2 spillover**, closed in the same round: `clear_no_color()` was hoisted into
`claudine/cli/tests/common/mod.rs` (it did not exist anywhere at R-1), **15** L2
capture files now call it, and **13** call sites of the new row-anchored
`assert_row_is_styled` replaced vacuous `frame.raw.contains(ESC)` guards — a
check that passes on any frame containing an escape byte anywhere, including one
emitted by tmux's own status line.

**The nuance that matters for anyone re-testing this.** `biscuit-test-harness`
already scrubbed `NO_COLOR` in `apply_color_forcing_env`, and a **warm tmux
server swallows ambient `NO_COLOR` entirely** — it is not in tmux's
`update-environment`, so a server started before the variable was exported never
sees it. The exposure was therefore nondeterministic and ordering-dependent, not
a clean always-red. **Any A/B on this must use a cold tmux server**; against a
warm one, both legs pass and prove nothing.

### Finding 5 — preflight status ordering

"Starting pre-flight checks" now renders **before** `run_phase_1c_with_schema`
rather than after it. The L2 frame-slicing helpers that used the status as a
post-preflight boundary are re-anchored on `PREFLIGHT_DONE_MARKER`, deliberately
just `"Preflight:"` — the full sentence folds in a 44-column pane, and a marker
that folds is a marker that fails to match. 2 new L1 ordering tests, executed on
macOS at R0.

**Partly open.** The *earlier* static preflight pass
(`build_preflight_graph` / `approve_preflight_graph`) still runs ahead of the
status and performs its own shell approval, so the finding's underlying
complaint — slow or interactive approval happening with no progress feedback —
partly survives. Moving the status earlier still would place it ahead of the
agent-selection picker, which is a UX call rather than a correctness one. **Left
for Ken.**

## Level-2 task-stream coverage

`claudine/cli/tests/level2_sequence_task_stream_capture.rs` — 7 tmux tests,
gated by `require_level!(Level::L2, TmuxHarness::available(), "tmux")` so the
suite skips cleanly on a host without tmux.

Six of the seven finish in ~3 s each. The seventh,
`level2_prompt_idle_flush_keeps_the_task_bar_in_tmux`, costs `78.128s` on its own
— 72.7% of the whole L2 tier — and is its critical path. See "The L2 tier's cost
profile changed this round" under "Gate runs".

L2 **complements** the L1 tests; it does not replace them. The exact-byte and
SGR assertions for frame atomicity stay in `sequence_groups.rs` and
`render::task_stream::tests`. The L2 file makes no byte-equality assertion:
tmux collapses and re-emits SGR, so color is checked by extracting the
`38;2;R;G;B` triple that paints the bar and comparing *triples*. Each fixture
is sized to fit the visible pane, because `capture()` never sees scrollback.

| Test | What only a real pane can prove |
|------|----------------------------------|
| `level2_parallel_task_bodies_carry_their_own_bar_color_in_tmux` | L1 compares a header from the stderr *pipe* against a body from the stdout *pipe*. A reader has neither. Here both lines emerge from one pane, interleaved in arrival order, and the body's bar color still matches its own header's — with the two tasks' colors distinct. |
| `level2_serial_and_parallel_share_one_left_edge_at_narrow_width_in_tmux` | Separated pipes have no width to overflow. In a 44-column pane, a frame that reaches the edge was folded by the *terminal*, which does not re-emit the gutter. Asserts nothing overflows, both bodies fold, continuations keep their gutter, and serial/parallel content starts at the same column. |
| `level2_no_color_pane_keeps_textual_attribution_in_tmux` | `NO_COLOR` chosen by a real capability handshake rather than a constructed `Terminal`: no bar is painted, and every task still announces and closes by name. |
| `level2_non_utf8_locale_uses_the_ascii_header_glyph_in_tmux` | `supports_unicode` is locale-derived, so only a process that actually inherits `LC_ALL=C` exercises the ASCII (`>`) fallback. |
| `level2_zero_step_sequence_renders_a_styled_notice_in_tmux` | The "resolved to 0 steps" notice carries styling and exits `0` when rendered by a real terminal. |
| `level2_parallel_prompt_streams_keep_task_attribution_in_tmux` | A `prompt:` task's provider text never passes through `TaskLiveOutput`; it is framed at the stream end instead. Stubs `claude` (not `goose`, which has no `stream_protocol`) so the semantic spawn actually runs, then requires the bar set painting each assistant/reasoning/tool marker to equal the task-header bar set. |
| `level2_prompt_idle_flush_keeps_the_task_bar_in_tmux` | An assistant block held open past the silence window is flushed by the idle ticker, not by `close()`. Only a post-stall marker discriminates the two, and only a real pane advances the ticker — which is why the test must actually wait. |

### Defect this coverage found

The narrow-width test failed on first run and exposed a real defect in
`TaskStreamFrame::quote()`. `TaskBar::Invisible` takes `BlockQuote`'s
custom-prefix path, which folds nothing on its own, and `Layout::default()`
ships `word_wrap: None` — so only the *default-border tree* path was wrapping.
A serial body line therefore ran past the pane edge and let the terminal wrap
it, dropping the two-column gutter and restarting the continuation at column 0:
exactly the sideways lurch the invisible bar exists to prevent. Parallel work,
on the tree path, folded correctly — so the two modes did not share one left
edge after all.

Fixed by opting the invisible branch's `Prose` into `WordWrap::default()`. The
colored branch is untouched, so the existing exact-byte L1 assertions still
hold. Guarded at both levels: `long_content_wraps_inside_the_invisible_bar_too`
(L1 unit) and the L2 narrow-pane test, which was confirmed to fail against the
pre-fix binary.

## Level-3 sequence Ctrl+C fan-out

Added for review-2 finding 5. The spec's concurrency section makes four
user-keyboard claims that no existing test discharged: a Ctrl+C during a
blocking parallel group must terminate every running child, prevent the next
step from launching, return control to the shell, and exit `130`. The prior
evidence was Level 1 (a manufactured `SIGINT` or a directly-set interrupt flag)
plus `level3_wrap_ctrl_c.rs`, which drives a standalone `claudine compose` —
one child, no later step, no group. It proves the shared wrapper can receive an
OS chord; it proves nothing about sequence scheduling or fan-out.

`claudine/cli/tests/level3_sequence_ctrl_c.rs` carries one test,
`level3_sequence_ctrl_c_fans_out_to_parallel_children`. It runs a real
`claudine sequence` in a focused, foreground WezTerm window, injects a genuine
macOS Quartz Ctrl+C chord via `cliclick`, and asserts all four claims from that
single keystroke.

**The fixture is deliberately SIGINT-immune**, and as of R0 it is so twice over.
A plain `sleep 300` task would risk sharing claudine's foreground process group
and being killed directly by the tty's SIGINT even if the fan-out were completely
broken — the test would pass on a regression. Two independent things now rule
that out: review-5 finding 1 gave `SystemTaskShell` `process_group(0)`, so task
children are no longer in the group the terminal broadcasts to; and each task
additionally runs `trap '' INT` before looping forever, publishing its pid first.
Only claudine's own machinery — the `signal_hook` handler setting the shared
flag, and each task's wait loop driving `ProcessTree::terminate` (SIGTERM, then
unignorable SIGKILL) — can end it. A dead pid is positive evidence that the
fan-out reached *that specific task*.

> Before R0 this paragraph credited the fixture's immunity solely to `trap ''
> INT`, because `process_group(0)` did not yet exist on the shell path. That
> sentence was true when written and is now superseded.

### Gating

`require_level!(Level::L3, WezTermHarness::available() && cliclick::available(),
…)`. Enabled by **`RUN_LEVEL3=1`** (plus WezTerm reachable via
`WEZTERM_UNIX_SOCKET`, and `cliclick` on `PATH`); `BISCUIT_TEST_LEVEL_REQUIRED=3`
turns a missing backend into a hard failure. Run via `just test-l3`. Verified
skip-clean with `RUN_LEVEL3` unset: `skipping: set RUN_LEVEL3=1 to enable Level 3
(WezTerm + cliclick)`. The `level3_` prefix keeps it out of both the `just test`
(L1) and `just test-l2` filtersets — confirmed by `cargo nextest list`, which
matches it under `test(/level3_/)` and not under the L1 expression.

### Execution status — last green at R-2; **not re-run at R-1 or R0**

`just test-l3` was guard-blocked on 2026-07-18: `_test_l3` refuses to start
without a TTY unless `BISCUIT_L3_TAKE_FOCUS=1` authorizes it, and the override
was deliberately not used — it hijacks an active desktop. See
[`gate-run-2026-07-18.md`](gate-run-2026-07-18.md) § Gate 4 for the verbatim
refusal. The guard doing its job is a passing observation about
`just/devops.just::_test_l3`, and a **not-run gate** for this feature.

So the record below stands as the most recent green, but it was obtained at
**R-2** — before the review-4 finding-2 interrupt-derivation change, the
finding-5 cfg gate, and the finding-8 compose guard, and long before review-5
finding 1 rewrote the very termination path this test exercises. AC1's
OS-keyboard leg is therefore **two rounds behind** the tree the L1 and L2 gates
certify, and the gap widened rather than narrowed at R0.

Observed green at R-2 on the authoring host (macOS, WezTerm + cliclick): all three
children reported `interrupted`, `step 1/2 interrupted by Ctrl+C` printed,
`later-step-ran.txt` absent, and the pane's exit marker read `L3SEQ_0rc=130`.

This supersedes an earlier record in this file stating the test had never
passed. That was accurate when written — the `cliclick` chord failed to reach
WezTerm across four consecutive runs, matching the focus-transfer limit
documented in `level3_wrap_ctrl_c.rs`. Two `biscuit-test-harness` changes fixed
delivery:

- `wezterm.rs` — `focus_spawned_pane` now polls until a WezTerm process is
  actually the frontmost macOS application (`wait_until_wezterm_frontmost`,
  5s budget) instead of sleeping a fixed 200ms and hoping. `AXRaise` returns
  before the WindowServer has necessarily made the window key.
- `cliclick.rs` — `click_then_ctrl_chord` / `click_then_alt_chord` now issue the
  chord through AppleScript `keystroke … using <modifier> down` rather than
  cliclick's `kd:ctrl t:c ku:ctrl`. cliclick cannot express a modified letter as
  one event, so the modifier flag raced the letter: measured 7/10 delivery
  versus 24/24 for the System Events path.

**Both changes are committed**, together, as `1fbf0d0b1` *"fix(biscuit-test-harness):
route modified chords through AppleScript and wait for focus"* (2026-07-18) — an
ancestor of R0, present on `error-prop-and-file-resolution` and **not yet on
`main`**. The green run depends on them; a checkout without them reproduces the
original failure, which is now only reachable by checking out `main`.

> A prior revision of this file claimed both changes were "still uncommitted on
> this branch." That was review-5 finding 6, and it was wrong: verified against
> `git log -- biscuit-test-harness/src/{wezterm,cliclick}.rs` and
> `git merge-base --is-ancestor 1fbf0d0b1 HEAD`. The working tree's only
> `biscuit-test-harness` changes at R0 are the two new L3 injector modules and
> the `lib.rs` lines that declare them.

The behavior is independently corroborated one tier down. Driving the same
fixture through `tmux send-keys C-c`, claudine reports each task `interrupted`,
prints `step 1/2 interrupted by Ctrl+C`, leaves `later-step-ran.txt` absent,
kills every published pid, and exits `130`.

Deadlines are sized (30s readiness, 15s termination) so a full run finishes in
~23–25s — inside nextest's 30s termination budget — meaning a chord that fails
to land reports as a clean assertion failure with a full pane dump rather than
an opaque `TMT`.

### Focus-stealing containment

This test raises a GUI terminal over every other window and injects real OS
keystrokes into whatever holds focus. Two guards bound that blast radius:

- `test_placement.rs::focus_stealing_apis_stay_in_keyboard_tier_files` (L1)
  fails if `SpawnVisibility::Foreground` or `focus_spawned_pane` appears as
  executable code in any Claudine test file not named `level3_*`, keeping the
  APIs behind the `level3_` filterset and the `RUN_LEVEL3=1` opt-in. Module docs
  naming the APIs do not trip it — the scan runs on comment-stripped bytes.
- `just/devops.just::_test_l3` refuses to start unattended: without a TTY it
  exits non-zero unless `BISCUIT_L3_TAKE_FOCUS=1` authorizes the run, so an
  agent, hook, or CI job cannot hijack an active desktop. From a terminal it
  prompts before proceeding.

## Known failures and skips

| Item | Status | Reason |
|------|--------|--------|
| `level2_context_{default,values,side_effects}_at_140_fills_cap_in_tmux` | **Pre-existing fail (3) — real, not a load artifact** | `claudine context` renders 140 visible cells on this host where the contract wants 138–139. Unrelated to sequences; Phase 2's checkpoint recorded the identical three failures, including the untouched default report. The 2026-07-18 run pins them as deterministic: all three fail on **all four** attempts, in `1.1`–`1.5 s`, with an identical off-by-one cell count. Do not excuse them as load — "pre-existing" is not "green", and the L2 tier does not return `0` while they stand. |
| Windows runtime execution | **Not run — nothing Windows has ever executed** | No Windows host and no emulation available. The strongest Windows evidence is a `--target x86_64-pc-windows-gnu` *compile* of lib+bin, a `--tests` *type-check* of both crates, and a source audit. None of that is a run. See "Windows: what compiles versus what has run". |
| Windows **test suites** | **Type-checked, never executed** | Both Windows suites — `termination/windows.rs`'s `#[cfg(all(test, windows))]` Job-object regressions and `cli/tests/level2_windows_sequence_ctrl_c.rs` — type-check under `just check-windows`. That closes the "invisible to every gate" half of review-4 finding 1: a typo or signature drift is now caught. It does **not** make them evidence of behavior. `cargo check --tests` neither links nor runs. |
| Windows **test-target** compilation | **Fixed — verified green** | Was 7 errors (Unix-only APIs in `#[cfg(test)]` code) plus a `duckdb-sys`/mingw wall. Both cleared at review 4; `just check-windows` type-checks lib **and** CLI test targets for `x86_64-pc-windows-gnu`, exit `0`, and the gate was probed to bite. See "Windows test targets: how the wall came down". |
| `level2_non_utf8_locale_uses_the_ascii_header_glyph_in_tmux` | **Pre-existing fail (1) at R0 — cold-server artifact** | New to this table at R0 only because R0's L2 run used a **cold** tmux server (required by finding 4's A/B). It passes on a warm server. An isolation artifact of the run configuration, not a sequence-plus regression — but recorded rather than dropped, because "passes on a warm server" is a caveat, not a green. |
| Level-2 suite | **Run at R0, green modulo 4 pre-existing fails** | `just test-l2 --no-fail-fast` on macOS at R0: **146 run, 142 passed, 4 failed**. The 4 are the `level2_context_*_at_140` trio plus the non-UTF-8 locale row above. No sequence-plus L2 test failed. R-1's reading was `144 tests run: 141 passed (6 slow), 3 failed`, exit `100` ([`gate-run-2026-07-18.md`](gate-run-2026-07-18.md)); the tier gained 2 tests from finding 2. |
| L2 for task-stream rendering | **Added** (review-2 finding 4) | See "Level-2 task-stream coverage" above. The prior "deliberately omitted" rationale was wrong in one respect: the contract is *not* a pure function of capability flags, because the flags themselves are chosen by a real handshake and the pane — not the renderer — decides what folding failure looks like. |
| `level3_sequence_ctrl_c_fans_out_to_parallel_children` (macOS) | **Last green at R-2; not re-run at R-1 or R0** | Green on the authoring host at R-2: every child interrupted, next step suppressed, exit `130`. `just test-l3` was **guard-blocked** at R-1 and simply not attempted at R0 — both sessions non-interactive. Two rounds stale, and it is the developer's step. See [`l3-ctrl-c-runbook.md`](l3-ctrl-c-runbook.md). |
| `level3_linux_sequence_ctrl_c.rs`, `level3_windows_sequence_ctrl_c.rs` | **Never executed on any host** | Added at R0 for review-5 finding 3. Compile and filterset evidence only. Neither Linux nor Windows has ever run an L3 sequence-interruption test. Finding 3 is **open**. |
| `#[ignore]`d perf harnesses | **Pre-existing** | `compose_ttff_perf.rs`, `completion_perf.rs`, `system_prompt_perf_bench.rs` — diagnostic, not gates. |

## Gate runs (review-2 finding 6)

Recorded on the authoring host (macOS, Apple Silicon). Nothing in this section is
reported green unless a command was run to completion.

**This section is ordered current-first.** "Current tree (R0)" is the tree a
reader checking out this branch gets today. Everything under "Historical" was
measured against a tree that no longer exists and is retained for the
discrimination it supports, not as a statement about R0.

| Subsection | Anchor | Host | Re-run at R0? |
|---|---|---|---|
| Current tree (R0) | R0 | macOS | — |
| macOS — R-1 | R-1 | macOS | superseded by R0 |
| Linux — real kernel via Docker | R-2 | Docker on macOS | **no** |
| Windows: what compiles versus what has run | R-1 | macOS cross-compile | type-check only |

Per the repository's drift-bracket convention every timed gate in the **R-1**
subsections carries the `uptime` bracket it was measured within. That is not
decoration on this host: the R-1 session's 1-minute load ranged from `8.46` to
`197.32` on 16 cores — a 23× spread — and one full L1 run went red at the top of
that range with 12 timeouts that did not reproduce at the bottom of it. **A
cross-run timing comparison without a bracket is not a measurement here.**

**The R0 table carries no brackets**, which is a real limitation of it: its
verdicts (exit codes, pass/fail counts) are sound, but nothing in it supports a
timing comparison against R-1. Treat the R0 row counts as verdicts, not
measurements.

### Status legend

- **Green** — ran to completion, no failures.
- **Green modulo known** — ran to completion; the only failures are pre-existing
  or environment-limited, each named individually.
- **Not run / blocked** — did not execute, with the exact blocker.

### Current tree (R0) — macOS, 2026-07-19

Run from `claudine/` on macOS 26.5.2 (Darwin 25.5.0, arm64) against
`be2d100a6` + the review-5 working tree, after findings 1–5 landed.

| Gate | Level | Verdict | Result |
|------|-------|---------|--------|
| `just test` | L1 | **Green** | exit `0`; ~6,156 passing across the 5 binaries |
| `just lint` | clippy | **Green** | exit `0` |
| `just check-windows` | type-check | **Green — nothing ran** | exit `0`. Type-check only; no Windows code executed |
| `just doctest` | L1 | **Green** | exit `0` |
| `just test-l2 --no-fail-fast` | L2 | **Green modulo 4 known** | 146 run, 142 passed, 4 failed |
| `just test-l3` | L3 | **Not run** | Not attempted — session non-interactive; the focus guard requires attended authorization |

**The L2 verdict is identical with `NO_COLOR=1` and with it unset**, against a
**cold** tmux server. That A/B is the executed evidence for review-5 finding 4,
and the cold server is what makes it meaningful — see finding 4 above for why a
warm server would have made both legs pass vacuously.

The 4 L2 failures are pre-existing and none belongs to this work: the three
`level2_context_*_at_140_fills_cap_in_tmux` (`expected 138..=139 visible cells;
got 140` — the known `claudine context` table-width drift) plus
`level2_non_utf8_locale_uses_the_ascii_header_glyph_in_tmux`, a cold-server
isolation artifact that passes warm. Both rows are in "Known failures and skips".

Two caveats that will cost the next person time if not recorded:

- **`just test-l2` fail-fasts.** The bare recipe stops at test 39 of 146 and
  never reaches the sequence tests at all. `--no-fail-fast` is not a preference
  here; without it the tier being measured is not the tier being reported.
- **`cargo fmt --check` reports diffs in files nobody touched.** That is the
  known local-rustfmt-versus-`main` drift documented in the root `CLAUDE.md`, not
  a finding. `just lint` is the formatting-adjacent gate of record, and it is
  green.

Unlike the R-1 subsection below, this table does not quote verbatim summary
lines: these results were recorded by the session that ran the gates and relayed
here, not captured into a transcript. Counts are reported to the precision they
were reported at. For verbatim gate output, R-1's
[`gate-run-2026-07-18.md`](gate-run-2026-07-18.md) remains the model.

---

**Everything from here to the end of this section is historical.** No subsection
below was measured against R0.

### macOS — R-1, 2026-07-18

Run 2026-07-18 against the review-4 tree — **after** findings 2, 5, 6, 8 and
finding 1's steps 1–2 all landed, and **before** any review-5 work. Superseded
by the R0 table above for `lint`, `test`, and `test-l2`; retained for its load
diagnosis and its verbatim summary lines. Full record in
[`gate-run-2026-07-18.md`](gate-run-2026-07-18.md).

| Gate | Verdict | Load bracket | Verbatim |
|------|---------|--------------|----------|
| `just lint` | **Green** | `17.70`→`17.64` | `JUST_LINT_EXIT=0`; all five crates `Finished dev profile`. Includes the `lifecycle-doc-facets` guard: `✅ lifecycle-doc-facets guard: lifecycle docs use the faceted err.* contract.` |
| `just test --no-fail-fast` | **Green** | `11.79`→`81.94` | `JUST_TEST_EXIT=0`, `191 s` wall — `claudine-catalog-types`: `Summary [   0.016s] 21 tests run: 21 passed, 0 skipped` · `claudine`: `Summary [  35.585s] 3775 tests run: 3775 passed (3 slow), 7 skipped` · `claudine-contract`: `Summary [   0.104s] 47 tests run: 47 passed, 5 skipped` · `claudine-cli`: `Summary [ 146.590s] 2160 tests run: 2160 passed (93 slow, 2 flaky), 172 skipped` · `claudine-gen`: `Summary [   2.796s] 152 tests run: 152 passed, 4 skipped` |
| `just test-l2 --no-fail-fast` | **Green modulo known** | `9.37`→`10.35` | `Summary [ 107.491s] 144 tests run: 141 passed (6 slow), 3 failed, 2188 skipped` → `JUST_L2_EXIT=100`; **110 s wall clock** |
| `just test-l3` | **Not run — guard-blocked** | `8.46`→`8.46` | `_test_l3` refuses an unattended run: no TTY on stdin and `BISCUIT_L3_TAKE_FOCUS` unset. Deliberately not overridden. Last recorded result is the review-3 round's green **against an older tree**; see "Level-3 sequence Ctrl+C fan-out". **Outstanding for this round; it is the developer's step.** |

**One L1 run went red and was diagnosed, not excused.** An earlier attempt at
load `16.87`→`197.32` returned `JUST_TEST_EXIT=1` with 12 timeouts. All four
discriminators point one way: every timeout landed at `30.02`–`30.09 s` against a
30 s cap (the cap, not a duration); 9 further tests in the same
compose/`inline-compose`/`opencode`/`perf` family went `FLAKY`, passing on retry
at `23.5`–`29.6 s`, i.e. one continuous population straddling the cap; an
isolated re-run of exactly those 12 returned `12 passed` at `15.1`–`17.1 s` each;
and the clean-bracket full-tier re-run above is green with zero timeouts, the
`claudine-cli` wall clock falling `308.100s → 146.590s` with no code change.
**Load artifact, confirmed.** The standing fragility is worth recording: that
family consumes ~55% of its cap even in isolation on a busy host, so it is the
first thing to time out anywhere. This gate certifies the L1 tier on a quiet
host, not on a loaded one.

**Two flaky tests in the green run, both benign.**
`claudine-cli::sequence_schema sequence_unsupported_shape_surfaces_typed_error_under_tty_pref`
is the known spurious nextest `LKFAIL` — a test binary that spawns a CLI child
needs a leak timeout — retried green in `0.185 s`; a leak failure is not a test
failure. `commands::compose::tests::sigint_during_prep_sets_interrupt_flag_and_renders_notice`
is a SIGINT-timing test that passed on the second attempt.

**The 3 `test-l2` failures are exactly the pre-existing trio**, confirmed by
name: `level2_context_default_at_140_fills_cap_in_tmux`,
`level2_context_values_at_140_fills_cap_in_tmux`,
`level2_context_side_effects_at_140_fills_cap_in_tmux`. They are **not** load
artifacts — deterministic across all four attempts, `1.1`–`1.5 s`, identical
off-by-one cell count (`expected 138..=139 visible cells; got 140`). No
sequence-plus L2 test failed. Note the bare `just test-l2` fail-fasts at the
first of these, which is why `--no-fail-fast` is the recipe of record.

**The L2 tier's cost profile changed this round** (review-4 finding 3, now
measured). The tier went from `142 tests / 43.629s` to `144 tests / 107.491s` —
**2.46×** — from two added tests, at a bracket of `9.37`→`10.35`. Effectively all
of it is one test: `level2_prompt_idle_flush_keeps_the_task_bar_in_tmux` at
**`PASS [  78.128s]`**, which is **72.7% of the tier on its own**. It stalls by
construction (a hardcoded 30 s `SILENCE_WINDOW` at
`cli/src/commands/wrap/exec/watchdog/spawn.rs:58` plus a ticker of the same
cadence, cleared only on the *second* tick) and trips both the `> 30.000s` and
`> 60.000s` SLOW thresholds. The next-longest L2 test in the whole suite is
`6.424 s`, and the other 143 overlap within the remaining ~29 s — so this one
test now sets the tier's floor, and no amount of parallelism brings `just
test-l2` below ~80 s again.

The bespoke `.config/nextest.toml` grant (`period = "30s", terminate-after = 6`
→ a 180 s cap) is **adequate with margin**: measured `78.128 s` consumes 43.4%,
leaving 2.30×. The test's own `run_in_pane_within` deadline of 150 s binds first,
which is the correct ordering — a blown deadline surfaces as an assertion failure
with a pane dump rather than an opaque `TMT`. Two caveats: the ~80 s floor is
structural and will not shrink on a fast host while the surrounding pane setup
*will* grow on a loaded one, and 2.30× headroom is not obviously enough at the
load this session peaked at (197). Budget for that before adding another
stall-shaped L2 test.

**Every sequence-plus L2 test passed**, including both new this round:
`level2_parallel_prompt_streams_keep_task_attribution_in_tmux` (`PASS [ 3.315s]`)
and `level2_prompt_idle_flush_keeps_the_task_bar_in_tmux` (`PASS [ 78.128s]`).

### Linux — real kernel via Docker

**Not re-run at review 4.** Recorded at the review-3 round; no product change
since then is Linux-specific.

Host is macOS; Linux evidence obtained under `rust:latest` on
`Linux 6.12.76-linuxkit aarch64 GNU/Linux`, `cargo 1.97.1 (c980f4866 2026-06-30)`,
`rustc 1.97.1 (8bab26f4f 2026-07-14)`. This is a real Linux kernel, not emulation.

| Gate | Verdict | Verbatim |
|------|---------|----------|
| `cargo check -p claudine -p claudine-cli` | **Green** | `Finished dev profile … in 1m 53s` → `EXIT_LIB_BIN=0` |
| `cargo check -p claudine --tests` | **Green** | `Finished dev profile … in 4m 18s` → `EXIT_CLAUDINE_TESTS=0` |
| `cargo test -p claudine --lib` (L1) | **Green modulo 2 container artifacts** | `test result: FAILED. 3675 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out; finished in 15.30s` → `EXIT_L1=101` |
| `cargo check -p claudine-cli --tests` | **Not run** | The container did not complete this leg, producing no output past the header. `claudine-cli`'s dev-dependency chain reaches `duckdb` via `rendezvous-daemon`; that build is the heavy element and the likely cause, but this was not isolated further. Recorded as not-run rather than green. |

Both Linux L1 failures are **artifacts of the container, not Linux defects**, and
each was falsified individually rather than assumed:

- `composition::sequence::task::tests::group_framing::a_parallel_group_gives_each_task_its_own_palette_entry`
  — failed with `three tasks shared a color: ["│", …] left: 1 right: 3`, i.e. no
  bar was painted at all. The container exports no `TERM`, so capability
  detection disables color. **Re-run with `TERM=xterm-256color`: `ok`.**
- `composition::resolve::tests::validate_permissions_readonly_file` — failed with
  `called Result::unwrap_err() on an Ok value`. The container runs as **root**
  (`whoami` → `root`), and root bypasses DAC write permission. Verified directly:
  `chmod 444 f` then appending as root succeeds. The test's premise cannot hold
  as root. Still fails with `TERM` set, as expected, since the cause is the uid.

### Windows: what compiles versus what has run

Everything in this subsection is a **compile-time** result obtained by
cross-compiling from macOS. Read no runtime claim into any of it: `cargo check`
type-checks, `cargo check --tests` additionally type-checks test code, and
neither links a binary nor executes one instruction. **Zero Windows code in this
feature has ever run.**

The `duckdb-sys` + mingw blocker recorded in prior notes was re-tested rather
than inherited, and it turned out to be a missing assembler flag rather than an
upstream wall — so the `--tests` legs that review-4 finding 1 recorded as
unreachable are now green. Steps 1 and 2 of that finding are closed; step 3, a
native Windows *run*, remains open and is the only thing that can close AC8.

| Gate | Verdict | Verbatim |
|------|---------|----------|
| `cargo check -p claudine -p claudine-cli --target x86_64-pc-windows-gnu` | **Green** | `Finished dev profile [unoptimized + debuginfo] target(s) in 2m 43s`; re-verified at review 4 in `54.84s` |
| `just check-windows` (lib **and** CLI, `--tests`) | **Green — type-check only** *(new at review 4)* | `JUST_CHECK_WINDOWS_EXIT=0`, `Finished dev profile [unoptimized + debuginfo] target(s) in 15.44s`. This is the gate that reaches the Windows-only suites. It does not link or run them. |
| `x86_64-pc-windows-msvc` | **Not run** | Target *is* installed, but `cargo check` runs dependency build scripts, and `aws-lc-sys` needs an MSVC-targeting C compiler this host lacks: `error occurred in cc-rs: … "cc" … "--target=x86_64-pc-windows-msvc"`. Reachable with `cargo-xwin` (downloads the MSVC SDK); not attempted. |

Two host-configuration traps had to be cleared first, and are recorded so the
next run does not mistake them for product breakage:

1. `~/.cargo/config.toml` sets `rustc-wrapper = "kache"`, which leaks into
   cc-rs as a compiler launcher and makes every C build fail with
   `error: unrecognized subcommand 'x86_64-w64-mingw32-gcc'`. Cross-compiles
   need `RUSTC_WRAPPER=""`.
2. With the wrapper disabled, kache's read-only cached artifacts in the shared
   `target/` cause `error: output file … is not writeable`. Use a separate
   `CARGO_TARGET_DIR`.

### Windows test targets: how the wall came down

Both blockers recorded above turned out to be softer than they read, and
removing them exposed two real defects that no gate could previously see.
Closed at review 4 (finding 1, steps 1–2); step 3, a native Windows *run*,
remains open.

**1. The 7 Unix-only test APIs — fixed, and not by deleting tests.**
`mappers.rs` used `ExitStatusExt::from_raw` in 4 tests. Gating the module
`#[cfg(unix)]` would have silently dropped those 4 from Windows, so instead a
local `exit_status(code)` helper absorbs the platform difference — Unix takes a
`wait(2)` status (code in the second byte), Windows takes the exit code itself —
so the tests are *compiled* on both and would run on both. They have run on
Unix only. The two `std::os::unix::fs::symlink` sites
(`protect/path.rs`, `protect/service/tests.rs`) *are* gated `#[cfg(unix)]`,
because creating a symlink on Windows needs `SeCreateSymbolicLinkPrivilege`,
which a test host cannot assume. The behavior under test is cross-platform; only
the fixture is not.

**2. The duckdb/mingw block fell to two changes, and the load-bearing one is the
dependency gate.** The prior note called COFF's `too many sections` limit "an
upstream toolchain limit"; it is really a *default*, since mingw's `as` supports
a big-object format that lifts the 32767-section cap. So
`check-windows` exports `CFLAGS_x86_64_pc_windows_gnu` /
`CXXFLAGS_x86_64_pc_windows_gnu` as `-Wa,-mbig-obj`. But that flag has to reach
`duckdb-sys`' own build script, which is not this repo's to configure — the
comment at `cli/Cargo.toml:113` records exactly that.

What actually made the CLI test targets reachable is the dependency gate:
`rendezvous-daemon` now sits under
`[target.'cfg(not(all(windows, target_env = "gnu")))'.dev-dependencies]`, with
matching `#[cfg]`s at the two fixture sites (`session_report/tests.rs`,
`dashboard/tests.rs`). The gate is on the **ABI, not the OS** — MSVC's assembler
has no section cap, so `x86_64-pc-windows-msvc` keeps the daemon fixtures and
only the `-gnu` cross-check target drops them. That target exists solely to
type-check Windows sources.

The reason a gate was unavoidable rather than merely convenient: a
`#![cfg(unix)]` inside a test *file* still builds every dev-dependency, so
nothing short of removing the dependency on this target lets
`cargo check -p claudine-cli --tests --target x86_64-pc-windows-gnu` reach the
Windows suites at all.

**3. Behind duckdb sat a real Windows compile error in `rendezvous-daemon`.**
`local_transport/windows.rs` handed `NamedPipeServer` straight to
`serve_local_incoming`, whose bound requires `tonic::transport::server::Connected`
— which tonic implements for `TcpStream` and `UnixStream` but not for named
pipes. **The Windows local control plane could not compile at all**, and the
duckdb wall had been hiding it. Fixed with a `PipeConnection` newtype
implementing `Connected` + `AsyncRead` + `AsyncWrite` by delegation. Its
`ConnectInfo` is `()` deliberately: a pipe has no peer address, nothing in the
daemon reads connection identity, and the local threat boundary is the DACL
applied at instance creation.

**4. And one in `claudine-cli` itself.** `commands::init` is a `#[cfg(test)]`
module whose reference helpers call `shellexpand`, declared only under
`[target.'cfg(unix)'.dependencies]` — so the Windows *test* target failed while
the bin target passed. Fixed by adding `shellexpand` as a dev-dependency, the
same remedy the neighbouring `url` entry already documents.

**The gate was verified to bite, not merely to pass.** A deliberate type error
(`let _: u32 = "not a u32";`) appended to `level2_windows_sequence_ctrl_c.rs` is
caught by `just check-windows` — `error[E0308]: mismatched types` — and the file
was restored immediately (`git diff --stat` clean). Without that probe, "exit 0"
would be indistinguishable from "the file was skipped", which is precisely the
failure mode finding 1 named.

`cargo tree -i duckdb` still resolves to
`duckdb → rendezvous-daemon → [dev-dependencies] → claudine-cli`: neither
`claudine` nor `claudine-cli` depends on duckdb to build or ship.

### The two Windows Low findings — one recorded, one fixed

- **Interrupt feedback bypasses the synchronized render sink** (finding 7) —
  **recorded as a deliberate choice, not changed.** `emit_interrupt_feedback`
  writes straight to stderr because the console handler is a context-free
  `extern "system" fn(u32)` Windows invokes on its own thread, while
  `StreamOutput` is per-run state behind an `Arc<Mutex<…>>` in the call chain.
  Reaching it would mean parking a global handle to the live run's sink for the
  sake of one static byte string. The cost is bounded and is not a torn line: a
  single newline-terminated `write_all` under `Stderr`'s internal lock, so bytes
  cannot interleave — only the sink's cursor bookkeeping misses the row. The
  reasoning now lives at the function rather than in this file, which is the
  review's stated acceptable resolution.
- **`install_user_interrupt_guard` was a no-op on Windows** (finding 8) —
  **fixed in source, type-checked, never executed.** The guard now has a real
  Windows body built on the sequence-plus coordinator: a shared `press_rung`
  ladder used by both hosts, a `HandlerGuard<ConsoleHandler>` over the new
  `ProcessHandler` trait, and `register_compose_interrupt_handler` for
  refcounted registration alongside sequence and wait-loop registrations.
  `on_console_interrupt` resolves a press through `classify_console_interrupt`,
  marks `USER_INTERRUPTED`, and force-exits via `ExitProcess` on the second
  press when no wait loop owns the ladder. So `claudine compose` /
  `inline-compose` on Windows is no longer behind the sequence path.

  Seven cross-platform L1 tests in `compose/interrupt.rs` cover the decision
  points — the rung ladder, inert presses with no run registered, publication
  and withdrawal of the notice, deferral while a wait loop is active, and the
  process-marking edges. Those tests execute everywhere.

  **The machine-checkable proof that the Windows body is real** is the
  disappearance of the two `never used` warnings at `cli/src/output/mod.rs:618`
  and `:644` from the `x86_64-pc-windows-gnu` target: `mark_user_interrupted`
  and `wait_loop_active` now have a Windows caller. That is a compile-time
  observation. Nothing here has run on Windows.

### Windows source audit

Because no Windows run is possible, the platform-sensitive paths this feature
touched were audited in production code (`#[cfg(test)]` excluded).

- **Path handling — clean.** No hardcoded separators, `/tmp` literals, or
  concatenated paths in changed production code; construction goes through
  `Path::join`/`PathBuf`. The literal `/` uses that remain are protocol-mandated
  (JSON Pointer in `composition/error/render/mod.rs:126`; `.gitignore` needle in
  `wrap/system_prompt.rs:172`) and correct on every host.
- **Newline handling — clean.** All `.lines()` uses are CRLF-safe by
  construction. The `split('\n')` in `wrap/stream_io.rs:87` and the
  `trim_end_matches('\n')` calls in `output/error_walker.rs:47` and
  `render/task_stream.rs:260,369` operate only on strings the program itself just
  rendered, never on file or child-process input.
- **Unix-only APIs — clean.** Every occurrence is gated with a Windows
  counterpart or documented degradation: `repo_home.rs:98/104` and `:393/398/406`,
  `spawn/setup.rs:74/79`, `wrap/sequence/mod.rs:162/171`,
  `compose/interrupt.rs:15/22/56/93`. `linking/symlink.rs:119` degrades to a typed
  `LinkingError` ("symlink creation is only supported on Unix").
- **Process spawn/termination — gated correctly.** The three defects this audit
  found are all fixed in source (below). `termination/mod.rs` gates `mod unix` on
  `#[cfg(unix)]` and `mod windows` on `#[cfg(windows)]`, with a
  `#[cfg(not(any(unix, windows)))] compile_error!` for anything else; the
  platform-independent `coordinator` and `handle` modules compile everywhere
  under `#[cfg_attr(unix, allow(dead_code))]` so their bookkeeping stays
  unit-testable off Windows. `spawn/setup.rs:73-84` pairs `cfg(unix)`
  `process_group(0)` with `cfg(windows)` `CREATE_NEW_PROCESS_GROUP`;
  `Cargo.toml` scopes `libc`/`signal-hook` to `cfg(unix)` and `windows` 0.62 to
  `cfg(windows)`.

#### Windows defects found by this audit — all three fixed; two still unexecuted

These are **product** findings surfaced while closing the gate. All three are
fixed. Their evidence is not equal, and the distinction is the point of this
subsection:

- Defects **1 and 2** are *fixed in source, type-checked, never executed*. Their
  Windows-host regressions now compile under `just check-windows`, which is a
  real advance over review 3 — a signature drift can no longer hide — but
  compiling a test is not running it. Their status is deliberately neither
  "open" nor "closed" until a native Windows run exists.
- Defect **3** is *fixed and verified*, because what it asserts is itself a
  compile-time property.

1. **Windows Ctrl+C was a no-op on the simple wait path** — *fixed in source,
   unverified.* Was: `wait_with_signal_handling` was a bare `child.wait()` that
   unconditionally returned `ProcessTermination::Completed`, while the child sat in
   `CREATE_NEW_PROCESS_GROUP` and never saw the terminal's Ctrl+C. Now: it runs the
   same `windows_wait_loop` as the channel-driven paths, and
   `register_sequence_interrupt_flag` gives `execute_sequence`'s shared flag a
   Windows producer. Fixed at review 3.
2. **Job-object handle leak** — *fixed in source, unverified.* Was: `CreateJobObjectW`
   yielded a bare `HANDLE` no path closed, leaking one per wrapped child (one per
   step in a sequence). Now: a `HandleCloser`-parameterised `OwnedRawHandle` owns
   it, so kill-on-close fires at the wait scope's end. Drop order is pinned
   cross-platform by `handle.rs::a_later_declared_guard_releases_before_the_handle_closes`,
   which does execute. The two Windows-host regressions in `termination/windows.rs`
   — the ones that would prove `TerminateJobObject` and kill-on-close actually fire
   — now type-check but have never run. Fixed at review 3.
3. **Module gate vs dependency gate mismatch** — *fixed.* Was: `mod windows` gated
   on `#[cfg(not(unix))]` while the `windows` crate is a
   `[target.'cfg(windows)'.dependencies]` entry, so a target that is neither failed
   on an unresolved crate rather than a stated non-support. Now: `mod windows` is
   gated on `#[cfg(windows)]`, and a
   `#[cfg(not(any(unix, windows)))] compile_error!` names the unsupported platform
   directly. Fixed at review 4 (finding 5). Unlike 1 and 2 this one **is** verified:
   both `cargo check -p claudine-cli` (macOS) and
   `--target x86_64-pc-windows-gnu` are green after the change.

### Dispatch inventory

`claudine-cli::dispatch_inventory dispatch_inventory_matches_committed_file` had
been red since 146f35a90. Regenerated with `CLAUDINE_UPDATE_INVENTORY=1`; the
diff is **two line numbers and nothing else** —
`wrap/harness_orch/attempt.rs` 257→264 and `wrap/wrapper_exec.rs` 179→183, both
keeping the same `path`, `form`, `dispatch_class`, `providers`, and
`exempt_candidate`. Total site count is unchanged at **1358 before and after**, so
no dispatch site appeared or disappeared and there is no architecture regression
to escalate. Verified green *without* the env var:
`Summary [   1.101s] 1 test run: 1 passed, 11 skipped`.

## Out-of-scope confirmations

Verified absent by grep over `claudine/lib/src` + `claudine/cli/src`:

- **No deprecation aliases** — no `#[deprecated]` added anywhere in the feature
  range; retired overlay names (`previous_state`, `next_state`, `step`,
  `total_steps`) appear only in the test that asserts their absence and in
  prose describing the removal. `SequenceRunSummary::total_steps` is an
  unrelated run-report field, not a frontmatter overlay key.
- **No nested sequences/groups** — rejected at preflight via
  `SequenceNestedSequence` / `SequenceUnsupportedConstruct`.
- **No group-loop semantics** — group `loop` is rejected, not interpreted.
- **No checkpoint/resume** — no persistence code exists.
- **No unapproved process-global mutation** — `set_current_dir` / `set_var`
  appear only in test files; production sequence and task paths never touch
  process-global env or CWD, pinned by
  `parallel_execution_leaves_process_env_and_cwd_untouched`.
