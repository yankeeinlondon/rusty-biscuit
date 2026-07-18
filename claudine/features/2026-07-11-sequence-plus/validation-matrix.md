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
| `just test-l2 --no-fail-fast` (from `claudine/`) | L2 — real-terminal PTY |
| `just lint` (each of the three areas) | clippy |

The bare `just test-l2` recipe fail-fasts at the first failure; `--no-fail-fast`
is required to get real coverage of the 144-case L2 suite.

## Test inventory

| File | Level | `#[test]` count |
|------|-------|-----------------|
| `claudine/lib/src/composition/sequence/tests.rs` | L1 | 103 |
| `claudine/lib/src/composition/sequence/task/tests.rs` | L1 | 66 |
| `claudine/lib/src/composition/sequence/preflight/tests.rs` | L1 | 37 |
| `claudine/lib/src/render/task_stream/tests.rs` | L1 | 17 |
| `claudine/lib/src/composition/runtime_state/tests.rs` | L1 | 13 |
| `claudine/cli/src/commands/wrap/sequence/jit/tests.rs` | L1 | 6 |
| `claudine/cli/tests/sequence_errors_cli.rs` | CLI E2E | 29 |
| `claudine/cli/tests/sequence_cli.rs` | CLI E2E | 28 |
| `claudine/cli/tests/sequence_sources_cli.rs` | CLI E2E | 22 |
| `claudine/cli/tests/sequence_groups.rs` | CLI E2E | 18 |
| `claudine/cli/tests/composition_outputs.rs` | CLI E2E | 14 |
| `claudine/cli/tests/sequence_jit.rs` | CLI E2E | 13 |
| `claudine/cli/tests/level2_sequence_overlay_pty.rs` | L2 | 7 |
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
  `level3_wrap_ctrl_c.rs` (pre-existing)

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

- `render/task_stream/tests.rs` (17) — narrow widths, wrapping, Unicode, no
  color, palette cycling, invisible-bar alignment, stdout/stderr split,
  concurrent writes, no torn escapes. **Column assertions count characters, not
  bytes** (`│` is one column, three UTF-8 bytes).
- `task/tests.rs::group_framing` (6);
  `concurrent_siblings_never_split_one_frame_group`
- Geometry parity pinned at both levels:
  `a_serial_frame_and_a_parallel_frame_share_one_left_edge` (L1) and
  `serial_and_parallel_group_frames_share_one_left_edge` (E2E)

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

### AC9 — Test placement

Follows the Claudine placement contract: inline unit tests by default, sibling
`tests.rs` modules past the size threshold (`sequence/`, `task/`, `preflight/`,
`task_stream/`, `runtime_state/`), CLI integration tests for orchestration and
output contracts, L2 only where a real terminal is required.

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

## Known failures and skips

| Item | Status | Reason |
|------|--------|--------|
| `level2_context_{default,values,side_effects}_at_140_fills_cap_in_tmux` | **Pre-existing fail (3)** | `claudine context` renders 140 visible cells on this host where the contract wants 138–139. Unrelated to sequences; Phase 2's checkpoint recorded the identical three failures, including the untouched default report. |
| Windows runtime execution | **Skipped** | Cross-compilation from this host is blocked upstream (`duckdb-sys` + mingw). Windows correctness rests on source audit and test gating, not a compiled run. |
| L2 for task-stream rendering | **Deliberately not added** | The contract is a pure function of `Terminal` capability flags taken as data, so a constructed `Terminal` exercises every branch more precisely than a captured pane. A real-terminal capture collapses SGR and would *weaken* the no-torn-escape assertion. |
| `#[ignore]`d perf harnesses | **Pre-existing** | `compose_ttff_perf.rs`, `completion_perf.rs`, `system_prompt_perf_bench.rs` — diagnostic, not gates. |

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
