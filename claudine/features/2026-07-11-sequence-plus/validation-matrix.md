# Sequence Plus — Acceptance Validation Matrix

Phase 13 handoff artifact. Each row maps one [`spec.md`](spec.md) acceptance
criterion to the tests that prove it and the command that runs them. Skips are
recorded with a reason rather than omitted.

## Commands

| Command | Scope |
|---------|-------|
| `just test` (from `claudine/`) | L1 — lib unit + CLI integration |
| `just test` (from `biscuit-file/`) | L1 — `ListFormat` |
| `just test` (from `darkmatter/`) | L1 — `set(…)`, name coercion, `last(list)` |
| `just test-l2 --no-fail-fast` (from `claudine/`) | L2 — real terminal (tmux) |
| `just lint` (each of the three areas) | clippy |
| `just check-windows` (from `claudine/`) | Windows type-check of lib + CLI **test** targets — the only gate that compiles the Windows-only suites |

The bare `just test-l2` recipe fail-fasts at the first failure; `--no-fail-fast`
is required to get real coverage of the 144-case L2 suite.

## Test inventory

| File | Level | `#[test]` count |
|------|-------|-----------------|
| `claudine/lib/src/composition/sequence/tests.rs` | L1 | 103 |
| `claudine/lib/src/composition/sequence/task/tests.rs` | L1 | 71 |
| `claudine/lib/src/composition/sequence/preflight/tests.rs` | L1 | 37 |
| `claudine/lib/src/render/task_stream/tests.rs` | L1 | 18 |
| `claudine/lib/src/composition/runtime_state/tests.rs` | L1 | 13 |
| `claudine/cli/src/commands/wrap/exec/termination/coordinator.rs` | L1 | 15 |
| `claudine/cli/src/commands/wrap/exec/termination/handle.rs` | L1 | 4 |
| `claudine/cli/tests/test_placement.rs` | L1 (guard) | 11 |
| `claudine/cli/src/commands/wrap/sequence/tests.rs` | L1 | 7 |
| `claudine/cli/src/commands/wrap/sequence/jit/tests.rs` | L1 | 6 |
| `claudine/cli/tests/sequence_errors_cli.rs` | CLI E2E | 29 |
| `claudine/cli/tests/sequence_cli.rs` | CLI E2E | 28 |
| `claudine/cli/tests/sequence_sources_cli.rs` | CLI E2E | 22 |
| `claudine/cli/tests/sequence_groups.rs` | CLI E2E | 18 |
| `claudine/cli/tests/composition_outputs.rs` | CLI E2E | 14 |
| `claudine/cli/tests/sequence_jit.rs` | CLI E2E | 13 |
| `claudine/cli/tests/sequence_overlay_pty.rs` | L1 (PTY) | 7 |
| `claudine/cli/tests/level2_sequence_task_stream_capture.rs` | L2 | 7 |
| `claudine/cli/tests/level2_windows_sequence_ctrl_c.rs` | L2 (Windows host) | 1 — type-checked by `just check-windows`, **never executed** |
| `claudine/cli/src/commands/wrap/exec/termination/windows.rs` | L1 (Windows host) | 2 — type-checked by `just check-windows`, **never executed** |
| `claudine/cli/tests/level3_sequence_ctrl_c.rs` | L3 | 1 |
| `biscuit-file/lib/src/list_format.rs` | L1 | 22 (+3 doctests) |

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
  (review-2 finding 5 — passing; see "Level-3 sequence Ctrl+C fan-out")

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
- `sequence_groups.rs` (18) — real overlap through the real `claudine sequence`
  path (three 1s tasks < 2.5s; four capped at 2 ≥ 1.9s)

### AC7 — Rendering

- `render/task_stream/tests.rs` (18) — narrow widths, wrapping, Unicode, no
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

### AC8 — Cross-platform

- `task/shell.rs` is the only spawner: `cfg(windows)` `cmd /C` vs `sh -c`,
  `try_wait` polling (no `wait4`/signals), `child.kill()` — identical semantics
  on all three platforms
- CRLF handled twice independently: `ListFormat::normalize_newlines` inbound,
  `trim_transport_newline` outbound
- Durations via `harness::parse_timeout`; paths via `FileReference`
- Every `#[cfg(unix)]` in the sequence suites exists for exactly one reason — a
  `#!/bin/sh` provider stub. The nine ungated witness tests found in Phase 12
  were gated, and the six blocked-construct **message contracts** they assert
  gained ungated counterparts in `sequence_errors_cli.rs`, so Windows keeps the
  message contract and only the zero-launch witness is gated.

**Compile evidence and one qualification.** macOS is the host. Linux is proven by
a real-kernel `cargo check` + L1 run under Docker, and Windows by a successful
`x86_64-pc-windows-gnu` compile of lib+bin — see "Gate runs (review-2 finding 6)"
for the verbatim results and for the pre-existing Windows *test*-target failures.

The "identical semantics on all three platforms" claim above holds for the
**task spawner**. For **interruption** it is now design-complete on both hosts
but unequally evidenced: `wrap/exec/termination/windows.rs` routes
`wait_with_signal_handling` through `windows_wait_loop`, and
`register_sequence_interrupt_flag` gives `execute_sequence`'s shared
`interrupted` flag a Windows producer. Neither has ever executed — see "Gate
runs" for what Windows evidence exists (production cross-compile plus source
audit) and review-4 finding 1 for why the Windows test suites cannot yet be
compiled.

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

See "Commands" above and "Known failures and skips" below.

## Level-2 task-stream coverage

`claudine/cli/tests/level2_sequence_task_stream_capture.rs` — 7 tmux tests,
gated by `require_level!(Level::L2, TmuxHarness::available(), "tmux")` so the
suite skips cleanly on a host without tmux.

Six of the seven finish in ~2–3 s each. The seventh,
`level2_prompt_idle_flush_keeps_the_task_bar_in_tmux`, costs `77.744s` on its
own and is the whole L2 tier's critical path — see "The L2 tier's cost profile
changed this round" under "Gate runs".

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

**The fixture is deliberately SIGINT-immune.** `SystemTaskShell` spawns each
task's `sh -c` *without* `process_group(0)`, so a plain `sleep 300` task would
share claudine's foreground process group and be killed directly by the tty's
SIGINT even if the fan-out were completely broken. Each task therefore runs
`trap '' INT` and then loops forever, publishing its pid first. Only claudine's
own machinery — the `signal_hook` handler setting the shared flag, and each
task's wait loop calling `child.kill()` (SIGKILL) — can end it. A dead pid is
positive evidence that the fan-out reached *that specific task*.

### Gating

`require_level!(Level::L3, WezTermHarness::available() && cliclick::available(),
…)`. Enabled by **`RUN_LEVEL3=1`** (plus WezTerm reachable via
`WEZTERM_UNIX_SOCKET`, and `cliclick` on `PATH`); `BISCUIT_TEST_LEVEL_REQUIRED=3`
turns a missing backend into a hard failure. Run via `just test-l3`. Verified
skip-clean with `RUN_LEVEL3` unset: `skipping: set RUN_LEVEL3=1 to enable Level 3
(WezTerm + cliclick)`. The `level3_` prefix keeps it out of both the `just test`
(L1) and `just test-l2` filtersets — confirmed by `cargo nextest list`, which
matches it under `test(/level3_/)` and not under the L1 expression.

### Execution status — PASSING (2026-07-18)

Observed green on the authoring host (macOS, WezTerm + cliclick): all three
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

**Both changes are still uncommitted on this branch.** The green run depends on
them; a checkout without them reproduces the original failure.

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
| `level2_context_{default,values,side_effects}_at_140_fills_cap_in_tmux` | **Pre-existing fail (3)** | `claudine context` renders 140 visible cells on this host where the contract wants 138–139. Unrelated to sequences; Phase 2's checkpoint recorded the identical three failures, including the untouched default report. |
| Windows runtime execution | **Not run** | No Windows host and no emulation available. Windows evidence is a successful `--target x86_64-pc-windows-gnu` *compile* of lib+bin plus a source audit — not an executed run. See "Gate runs (review-2 finding 6)". |
| Windows **test suites** | **Compiled, not executed** | Both Windows suites — `termination/windows.rs`'s `#[cfg(all(test, windows))]` Job-object regressions and `cli/tests/level2_windows_sequence_ctrl_c.rs` — now type-check under `just check-windows`. That closes the "invisible to every gate" half of review-4 finding 1: a typo or signature drift is now caught. It does **not** make them executed evidence; only a native Windows run does. |
| Windows **test-target** compilation | **Fixed — green** | Was 7 errors (Unix-only APIs in `#[cfg(test)]` code) plus a `duckdb-sys`/mingw wall. Both cleared at review 4; `just check-windows` now type-checks lib **and** CLI test targets for `x86_64-pc-windows-gnu`, exit `0`. See "Windows test targets: how the wall came down". |
| Level-2 suite | **Run, green modulo the 3 pre-existing `context` fails** | `just test-l2 --no-fail-fast`: `144 tests run: 141 passed (2 slow), 3 failed` (re-run at review 4). The 3 are the `level2_context_*_at_140` row above. The prior "Level-2 omitted" framing is superseded. |
| L2 for task-stream rendering | **Added** (review-2 finding 4) | See "Level-2 task-stream coverage" below. The prior "deliberately omitted" rationale was wrong in one respect: the contract is *not* a pure function of capability flags, because the flags themselves are chosen by a real handshake and the pane — not the renderer — decides what folding failure looks like. |
| `level3_sequence_ctrl_c_fans_out_to_parallel_children` | **Passing** | Green on the authoring host: every child interrupted, next step suppressed, exit `130`. Depends on the (still uncommitted) `biscuit-test-harness` focus and chord-delivery fixes — see "Execution status". |
| `#[ignore]`d perf harnesses | **Pre-existing** | `compose_ttff_perf.rs`, `completion_perf.rs`, `system_prompt_perf_bench.rs` — diagnostic, not gates. |

## Gate runs (review-2 finding 6)

Recorded on the authoring host (macOS, Apple Silicon). Nothing in this section is
reported green unless a command was run to completion and its summary line is
quoted verbatim.

The macOS table below was **re-run at review 4** (2026-07-18, load 8–16) against
the tree that carries the review-3 Windows rebuild, the two new L2 tests, the
coordinator/handle L1 suites, and the review-4 finding-2 interrupt-derivation
fix. It supersedes the review-3 run (load 80–160), whose two timeouts and
`146 slow` count were load artifacts that do not reproduce at this bracket. The
Linux and Windows subsections still date from the review-3 round and are **not**
re-run here; each is marked accordingly.

### Status legend

- **Green** — ran to completion, no failures.
- **Green modulo known** — ran to completion; the only failures are pre-existing
  or environment-limited, each named individually.
- **Not run / blocked** — did not execute, with the exact blocker.

### macOS (host) — `claudine/`

Re-run 2026-07-18 at review 4, against the final tree — **after** findings 2, 5,
6 and finding 1's steps 1–2 all landed. Load bracket: `12.56` entering L2,
`13.02` on L2 exit; L1 ran at `18`–`67` (the upper end is the build, not the
tests). Two earlier runs of the same gates, at load `8`–`12` and mid-way through
the findings, returned the same verdicts.

| Gate | Verdict | Verbatim |
|------|---------|----------|
| `just lint` | **Green** | `JUST_LINT_EXIT=0`; all five crates `Finished dev profile`. Includes the `lifecycle-doc-facets` guard: `✅ lifecycle-doc-facets guard: lifecycle docs use the faceted err.* contract.` |
| `just test --no-fail-fast` | **Green** | `JUST_TEST_EXIT=0`, all five crates — `claudine-catalog-types`: `Summary [   0.017s] 21 tests run: 21 passed, 0 skipped` · `claudine`: `Summary [  38.230s] 3773 tests run: 3773 passed (6 slow), 7 skipped` · `claudine-contract`: `Summary [   0.098s] 47 tests run: 47 passed, 5 skipped` · `claudine-cli`: `Summary [ 118.901s] 2151 tests run: 2151 passed (74 slow), 172 skipped` · `claudine-gen`: `Summary [   1.768s] 152 tests run: 152 passed, 4 skipped` |
| `just test-l2 --no-fail-fast` | **Green modulo known** | `Summary [ 103.815s] 144 tests run: 141 passed (2 slow), 3 failed, 2179 skipped` → `JUST_L2_EXIT=100`; **106 s wall clock** |
| `just test-l3` | **Not run / blocked** | `_test_l3` refuses an unattended run: no TTY on stdin and `BISCUIT_L3_TAKE_FOCUS` unset. Deliberately not overridden — the override hijacks an active desktop. Last recorded result is the review-3 round's green; see "Level-3 sequence Ctrl+C fan-out". **This gate is the developer's step and is outstanding for this round.** |

**The two `just test` timeouts from the review-3 run did not reproduce.** At this
load the whole tier is green with zero timeouts, and the L1 wall clock for the
`claudine` crate fell from `125.121s` to `38.230s` — confirming the earlier
`2 timed out` / `146 slow` reading was the host, not the product.

**Two spurious `LKFAIL`s appeared in the earlier of the two review-4 runs and not
in the final one**, which is the expected signature of the nextest leak-timeout
artifact rather than behavior:
`claudine composition::schema::tests::top_level_pointer_segment_handles_escaped_keys`
and `claudine-cli::sequence_prompt_property sequence_rejects_interactive_true_frontmatter_via_cli`,
each `TRY 1 LKFAIL` then `FLAKY 2/4` with the retry passing in ~0.03 s. A test
binary that spawns a CLI child needs a leak timeout. Both runs returned
`JUST_TEST_EXIT=0`.

**The 3 `test-l2` failures are exactly the pre-existing trio**, confirmed by
name: `level2_context_default_at_140_fills_cap_in_tmux`,
`level2_context_values_at_140_fills_cap_in_tmux`,
`level2_context_side_effects_at_140_fills_cap_in_tmux`. No sequence-plus L2 test
failed. Note the bare `just test-l2` fail-fasts at the first of these, which is
why `--no-fail-fast` is the recipe of record.

**The L2 tier's cost profile changed this round** (review-4 finding 3). The tier
went from `142 tests / 43.629s` to `144 tests / 103.815s` — a 2.4× wall-clock
increase from two added tests. Effectively all of it is one test:
`level2_prompt_idle_flush_keeps_the_task_bar_in_tmux` at **`PASS [  77.744s]`**,
which stalls by construction (a 30 s `SILENCE_WINDOW` and ticker cadence, cleared
only on the *second* tick) and trips both the `> 30.000s` and `> 60.000s` SLOW
thresholds. It is the tier's critical path; the remaining 143 tests overlap
within the other ~26 s. The bespoke `.config/nextest.toml` `terminate-after = 6`
grant is what keeps it from being killed. The expense is real and now measured —
budget for it before adding another stall-shaped L2 test.

**Every sequence-plus L2 test passed**, including both new this round:
`level2_parallel_prompt_streams_keep_task_attribution_in_tmux` (`PASS [ 2.992s]`)
and `level2_prompt_idle_flush_keeps_the_task_bar_in_tmux` (`PASS [ 77.744s]`).

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

### Windows — cross-compile from macOS

The `duckdb-sys` + mingw blocker recorded in prior notes was re-tested rather
than inherited, and it **does not apply to claudine's lib or binary**.

**Partially re-run at review 4.** The production cross-compile was repeated
against the finding-2 fix:
`RUSTC_WRAPPER="" CARGO_TARGET_DIR=… cargo check -p claudine-cli --target x86_64-pc-windows-gnu`
→ `Finished dev profile [unoptimized + debuginfo] target(s) in 54.84s`, exit `0`,
with the same 2 pre-existing `never used` warnings at `cli/src/output/mod.rs:618,644`
(review-4 finding 8). The `--tests` legs below were **not** re-run; they remain
blocked for the reasons stated, which is review-4 finding 1.

| Gate | Verdict | Verbatim |
|------|---------|----------|
| `cargo check -p claudine -p claudine-cli --target x86_64-pc-windows-gnu` | **Green** | `Finished dev profile [unoptimized + debuginfo] target(s) in 2m 43s`; re-verified at review 4 in `54.84s` |
| `just check-windows` (lib **and** CLI, `--tests`) | **Green** *(new at review 4)* | `JUST_CHECK_WINDOWS_EXIT=0`, `Finished dev profile [unoptimized + debuginfo] target(s) in 15.44s`. Supersedes the two rows below, which recorded this as blocked. |
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
and the tests now run on both. The two `std::os::unix::fs::symlink` sites
(`protect/path.rs`, `protect/service/tests.rs`) *are* gated `#[cfg(unix)]`,
because creating a symlink on Windows needs `SeCreateSymbolicLinkPrivilege`,
which a test host cannot assume. The behavior under test is cross-platform; only
the fixture is not.

**2. The duckdb/mingw block was a missing assembler flag, not an upstream wall.**
The prior note called COFF's `too many sections` limit "an upstream toolchain
limit". It is a *default*: mingw's `as` supports a big-object COFF format that
lifts the 32767-section cap, and duckdb's unity-build blobs need it. Setting
`CFLAGS_x86_64_pc_windows_gnu` / `CXXFLAGS_x86_64_pc_windows_gnu` to
`-Wa,-mbig-obj` builds `libduckdb-sys` cleanly — 0 section errors. **The
dev-dependency never needed breaking or feature-gating.** The flag now lives in
the `check-windows` recipe rather than in one person's shell history.

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

### Two Windows gaps deliberately left open

Both are review-4 Low findings, and both are recorded rather than fixed.

- **Interrupt feedback bypasses the synchronized render sink** (finding 7).
  `emit_interrupt_feedback` writes straight to stderr because the console handler
  is a context-free `extern "system" fn` on a Windows-owned thread while
  `StreamOutput` is per-run state behind an `Arc<Mutex<…>>`. Reaching it would
  mean a global handle to the live run's sink for the sake of one static byte
  string. The cost is bounded — a single newline-terminated `write_all` under
  `Stderr`'s internal lock, so no torn line, only cursor bookkeeping that misses
  one row. **Closed as a deliberate choice, now recorded at the function.**
- **`install_user_interrupt_guard` is a no-op on Windows** (finding 8). Real gap:
  `claudine compose` gets no `USER_INTERRUPTED` marking and no second-press
  force-exit there. Split out to
  [`fixes/_unscheduled/1-windows-compose-interrupt-guard/spec.md`](../../fixes/_unscheduled/1-windows-compose-interrupt-guard/spec.md)
  because it is on the *compose* path rather than the sequence path, needs new
  coordinator surface rather than reuse, and cannot be runtime-verified from this
  host — closing a Low finding by adding more never-executed Windows code would
  deepen finding 1.

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
  found are all fixed in source (below).
  `termination/mod.rs` gates `unix`/`windows` modules; `spawn/setup.rs:73-84`
  pairs `cfg(unix)` `process_group(0)` with `cfg(windows)`
  `CREATE_NEW_PROCESS_GROUP`; `Cargo.toml` scopes `libc`/`signal-hook` to
  `cfg(unix)` and `windows` 0.62 to `cfg(windows)`.

#### Windows defects found by this audit — all three now fixed in source

These are **product** findings surfaced while closing the gate. All three have
since been fixed, but **no fix has executed on a Windows host**: each is "fixed in
source, no executed evidence" until review-4 finding 1 makes the Windows test
targets compilable. The status below is deliberately neither "open" nor "closed".

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
   cross-platform by `handle.rs::a_later_declared_guard_releases_before_the_handle_closes`;
   the two Windows-host regressions in `termination/windows.rs` remain uncompilable.
   Fixed at review 3.
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
