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

The bare `just test-l2` recipe fail-fasts at the first failure; `--no-fail-fast`
is required to get real coverage of the 142-case L2 suite.

## Test inventory

| File | Level | `#[test]` count |
|------|-------|-----------------|
| `claudine/lib/src/composition/sequence/tests.rs` | L1 | 103 |
| `claudine/lib/src/composition/sequence/task/tests.rs` | L1 | 66 |
| `claudine/lib/src/composition/sequence/preflight/tests.rs` | L1 | 37 |
| `claudine/lib/src/render/task_stream/tests.rs` | L1 | 18 |
| `claudine/lib/src/composition/runtime_state/tests.rs` | L1 | 13 |
| `claudine/cli/src/commands/wrap/sequence/jit/tests.rs` | L1 | 6 |
| `claudine/cli/tests/sequence_errors_cli.rs` | CLI E2E | 29 |
| `claudine/cli/tests/sequence_cli.rs` | CLI E2E | 28 |
| `claudine/cli/tests/sequence_sources_cli.rs` | CLI E2E | 22 |
| `claudine/cli/tests/sequence_groups.rs` | CLI E2E | 18 |
| `claudine/cli/tests/composition_outputs.rs` | CLI E2E | 14 |
| `claudine/cli/tests/sequence_jit.rs` | CLI E2E | 13 |
| `claudine/cli/tests/sequence_overlay_pty.rs` | L1 (PTY) | 7 |
| `claudine/cli/tests/level2_sequence_task_stream_capture.rs` | L2 | 5 |
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
  `level3_wrap_ctrl_c.rs` (pre-existing) +
  `level3_sequence_ctrl_c.rs` (review-2 finding 5 — see "Level-3 sequence
  Ctrl+C fan-out" below for its honest, not-yet-passing status)

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
**task spawner**, but not for **interruption**: `wrap/exec/termination/windows.rs`
has no Ctrl+C handling on the simple wait path, so AC1's Ctrl+C/exit-`130`
behavior is currently Unix-only. Recorded as an open Windows defect in the gate-runs
section rather than treated as satisfied.

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

`claudine/cli/tests/level2_sequence_task_stream_capture.rs` — 5 tmux tests,
~6 s wall clock, gated by `require_level!(Level::L2, TmuxHarness::available(),
"tmux")` so the suite skips cleanly on a host without tmux.

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

### Execution status — NOT yet observed passing

Run for real on the authoring host (macOS, WezTerm + cliclick both present).
Every stage up to and including the keystroke works: the pane spawns, claudine
launches, the group announces all three children, and the pids publish. The
`cliclick` chord then fails to reach WezTerm, no SIGINT is delivered, and the
test fails honestly at its 15s termination assertion (4 consecutive attempts).

This is the focus-transfer reliability limit already documented in
`level3_wrap_ctrl_c.rs`, **not** a fan-out defect. Control experiment: the
pre-existing `level3_ctrl_c_terminates_wrapped_child` fails the same way on the
same host in the same session, echoing a bare `c` into its pane — while its
sibling `…_with_timeout_configured` passed, showing the chord lands
intermittently rather than never.

The behavior under test *is* confirmed one tier down. Driving this exact fixture
through `tmux send-keys C-c`, claudine reports each task `interrupted`, prints
`step 1/2 interrupted by Ctrl+C`, leaves `later-step-ran.txt` absent, kills every
published pid, and exits `130`. So the fixture and the product path are sound;
what remains unproven from this host is only the OS-keyboard leg.

The test is **not** loosened to force a pass. Its deadlines are sized (30s
readiness, 15s termination) so the whole run finishes in ~23–25s — inside
nextest's 30s termination budget — meaning a chord that never lands reports as a
clean assertion failure with a full pane dump rather than an opaque `TMT`.

## Known failures and skips

| Item | Status | Reason |
|------|--------|--------|
| `level2_context_{default,values,side_effects}_at_140_fills_cap_in_tmux` | **Pre-existing fail (3)** | `claudine context` renders 140 visible cells on this host where the contract wants 138–139. Unrelated to sequences; Phase 2's checkpoint recorded the identical three failures, including the untouched default report. |
| Windows runtime execution | **Not run** | No Windows host and no emulation available. Windows evidence is a successful `--target x86_64-pc-windows-gnu` *compile* of lib+bin plus a source audit — not an executed run. See "Gate runs (review-2 finding 6)". |
| Windows **test-target** compilation | **Pre-existing fail** | `cargo check -p claudine --tests --target x86_64-pc-windows-gnu` fails with 7 errors, all Unix-only APIs inside `#[cfg(test)]` code. All three sites exist identically on `main` (`git grep os::unix main -- …`), so this is pre-existing, not sequence-plus work. |
| Level-2 suite | **Run, green modulo the 3 pre-existing `context` fails** | `just test-l2 --no-fail-fast`: `142 tests run: 139 passed (4 slow), 3 failed`. The 3 are the `level2_context_*_at_140` row above. The prior "Level-2 omitted" framing is superseded. |
| L2 for task-stream rendering | **Added** (review-2 finding 4) | See "Level-2 task-stream coverage" below. The prior "deliberately omitted" rationale was wrong in one respect: the contract is *not* a pure function of capability flags, because the flags themselves are chosen by a real handshake and the pane — not the renderer — decides what folding failure looks like. |
| `level3_sequence_ctrl_c_fans_out_to_parallel_children` | **Added, not yet passing** | The cliclick chord does not reach WezTerm on this host, so no SIGINT is delivered. Environment limit, not a product defect — the pre-existing `level3_ctrl_c_terminates_wrapped_child` fails identically here, and the same fixture driven by `tmux send-keys C-c` exits `130` with every child interrupted and the next step suppressed. See "Level-3 sequence Ctrl+C fan-out". |
| `#[ignore]`d perf harnesses | **Pre-existing** | `compose_ttff_perf.rs`, `completion_perf.rs`, `system_prompt_perf_bench.rs` — diagnostic, not gates. |

## Gate runs (review-2 finding 6)

Recorded 2026-07-18 on the authoring host (macOS, Apple Silicon). **Host load was
80–160 throughout** — every timing-shaped result below is bracketed accordingly.
Nothing in this section is reported green unless a command was run to completion
and its summary line is quoted verbatim.

### Status legend

- **Green** — ran to completion, no failures.
- **Green modulo known** — ran to completion; the only failures are pre-existing
  or environment-limited, each named individually.
- **Not run / blocked** — did not execute, with the exact blocker.

### macOS (host) — `claudine/`

| Gate | Verdict | Verbatim |
|------|---------|----------|
| `just lint` | **Green** | `JUST_LINT_EXIT=0`; all five crates `Finished dev profile`. Includes the `lifecycle-doc-facets` guard: `✅ lifecycle-doc-facets guard: lifecycle docs use the faceted err.* contract.` |
| `just test --no-fail-fast` | **Green modulo load flake** | `Summary [ 125.121s] 3773 tests run: 3771 passed (146 slow), 2 timed out, 7 skipped` → `JUST_TEST_EXIT=1` |
| `just test-l2 --no-fail-fast` | **Green modulo known** | `Summary [  43.629s] 142 tests run: 139 passed (4 slow), 3 failed, 2156 skipped` → `JUST_L2_EXIT=100` |
| `just test-l3` | **Not run here** | Environment-limited; see "Level-3 sequence Ctrl+C fan-out". |

**The 2 `just test` timeouts are load artifacts, not failures.** Both are
`TRY 4 TMT [ 30.0s]`:
`composition::interpolation_conformance::loop_and_lifecycle_agree_on_shared_syntax`
and
`composition::looping::engine::tests::seed_state::seeded_loop_repro_runs_to_completion_with_live_derived_variable`.
Re-run in isolation at load 158.83 both pass:
`Summary [  16.938s] 2 tests run: 2 passed (2 slow), 3778 skipped`.
The `146 slow` count in the same run — with ordinarily sub-second tests taking
7 s — corroborates load, not a product regression.

**The 3 `test-l2` failures are exactly the pre-existing trio**, confirmed by
name: `level2_context_default_at_140_fills_cap_in_tmux`,
`level2_context_values_at_140_fills_cap_in_tmux`,
`level2_context_side_effects_at_140_fills_cap_in_tmux`. No sequence-plus L2 test
failed. Note the bare `just test-l2` fail-fasts at the first of these
(`38/142 tests run`), which is why `--no-fail-fast` is the recipe of record.

### Linux — real kernel via Docker

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

| Gate | Verdict | Verbatim |
|------|---------|----------|
| `cargo check -p claudine -p claudine-cli --target x86_64-pc-windows-gnu` | **Green** | `Finished dev profile [unoptimized + debuginfo] target(s) in 2m 43s` |
| `cargo check -p claudine --tests --target …-gnu` | **Pre-existing fail** | `error: could not compile claudine (lib test) due to 7 previous errors` |
| `cargo check -p claudine -p claudine-cli --tests --target …-gnu` | **Blocked** | `duckdb-sys` unity build: `x86_64-w64-mingw32-as: … too many sections (54084)` / `Fatal error: … file too big` |
| `x86_64-pc-windows-msvc` | **Not run** | Target not installed; requires the MSVC toolchain and Windows SDK, neither available on this host. |

Two host-configuration traps had to be cleared first, and are recorded so the
next run does not mistake them for product breakage:

1. `~/.cargo/config.toml` sets `rustc-wrapper = "kache"`, which leaks into
   cc-rs as a compiler launcher and makes every C build fail with
   `error: unrecognized subcommand 'x86_64-w64-mingw32-gcc'`. Cross-compiles
   need `RUSTC_WRAPPER=""`.
2. With the wrapper disabled, kache's read-only cached artifacts in the shared
   `target/` cause `error: output file … is not writeable`. Use a separate
   `CARGO_TARGET_DIR`.

**The 7 Windows test-compile errors are pre-existing.** All are Unix-only APIs
inside test code: `std::os::unix::process::ExitStatusExt` /
`ExitStatus::from_raw` at `claudine/lib/src/dispatch/runner/mappers.rs:146,153,166,182,198`,
and `std::os::unix::fs::symlink` at `claudine/lib/src/protect/path.rs:402` and
`claudine/lib/src/protect/service/tests.rs:318`. `mappers.rs` and `protect/path.rs`
are **byte-identical to `main`** (`git diff main...HEAD` empty); the third is a
Phase-13 test *extraction* whose Unix call already existed at
`main:claudine/lib/src/protect/service.rs:515`. Sequence-plus introduced none of them.

**The `--tests` duckdb block is a dev-dependency artifact, not product code.**
`cargo tree -i duckdb` resolves to
`duckdb → rendezvous-daemon → [dev-dependencies] → claudine-cli`. Neither
`claudine` nor `claudine-cli` depends on duckdb to build or ship; it is reachable
only when compiling `claudine-cli`'s test targets. The failure is COFF's
`too many sections` limit against duckdb's unity build under mingw — an upstream
toolchain limit, unrelated to this feature.

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
- **Process spawn/termination — gated correctly, with one behavioral gap (below).**
  `termination/mod.rs:28-50` gates `unix`/`windows` modules; `spawn/setup.rs:73-84`
  pairs `cfg(unix)` `process_group(0)` with `cfg(windows)`
  `CREATE_NEW_PROCESS_GROUP`; `Cargo.toml` scopes `libc`/`signal-hook` to
  `cfg(unix)` and `windows` 0.62 to `cfg(windows)`.

#### Open Windows defects found by this audit (not fixed here)

These are **product** findings surfaced while closing the gate. They are recorded
rather than silently folded into a green claim.

1. **Windows Ctrl+C is a no-op on the simple wait path.**
   `wrap/exec/termination/windows.rs:24-34` — `wait_with_signal_handling` is a
   bare `child.wait()`: no console handler, no press tracking, and it
   unconditionally returns `ProcessTermination::Completed`. The Unix counterpart
   (`unix.rs:78-135`) runs the full SIGINT → SIGTERM → SIGKILL ladder. This path
   is reachable in production — `spawn/semantic.rs:523` selects it whenever
   `needs_advanced_wait` is false. Because the child is spawned into
   `CREATE_NEW_PROCESS_GROUP`, it does not receive the terminal's Ctrl+C either,
   so on Windows the chord terminates nothing and an interrupted run is reported
   as `Completed`. This qualifies AC1's Ctrl+C/exit-`130` claim on Windows.
2. **Job-object handle leak.** `windows.rs:195` —
   `CreateJobObjectW` yields a bare `HANDLE` (no `Drop`; the RAII wrapper is
   `windows::core::Owned<HANDLE>`), and no path calls `CloseHandle`. One handle
   leaks per wrapped child, so a `sequence` leaks one per step. The doc comment at
   `windows.rs:192-194` is correspondingly wrong: it claims descendants die when
   the handle closes "by normal Drop", which never happens.
3. **Module gate vs dependency gate mismatch.** `termination/mod.rs:31` gates
   `mod windows` on `#[cfg(not(unix))]`, but `claudine/cli/Cargo.toml:62` supplies
   the `windows` crate only under `[target.'cfg(windows)'.dependencies]`. Real
   Windows is fine; a target that is neither unix nor windows fails on the missing
   crate.

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
