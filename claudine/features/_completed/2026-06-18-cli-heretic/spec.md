---
created: 2026-06-18
reviewed: true
status: ready for planning and implementation
area:
  - claudine-cli
---

# CLI Heretic — Dismantling the 15 God Files

## Problem

`hug god-files --high-risk` reports **15 high-risk files** in `claudine-cli`. They
fall into two structurally different shapes, and the report's per-file detail
makes the shape obvious:

1. **Mega-function files** — one function holds 30–70% of the file. The worst
   offenders:
   - `composition/mod.rs` → `execute_composition_request_inner` **908 SLOC**
   - `wrap/mod.rs` → `run_provider_wrapper_inner` **862 SLOC**
   - `wrap/sequence.rs` → `execute_sequence` **550 SLOC** (50% of file)
   - `wrap/harness_orch.rs` → `run_harness_loop` **509 SLOC** + `execute_harness_attempt` **298 SLOC**
   - `commands/compose.rs` → `run_inline_compose_inner` **369** + `run_compose_inner` **299**
2. **Wide-surface files** — many cohesive-but-distinct top-level symbols plus a
   large inline `#[cfg(test)] mod tests`. The report flags these with "N
   unrelated top-level symbols — split by responsibility" and high import
   fan-out (`live_semantic_sink/mod.rs` 39 imports, `wrap/mod.rs` 90,
   `profile/mod.rs` WrapperProfile trait with 27 members).
3. **Pure test files** — `tests/wrap_commands.rs` is **5986 SLOC** across 166
   test functions; `tests/sequence_cli.rs` (1250) and
   `tests/level2_schema_prompt_pty.rs` (1134) are smaller versions of the same
   thing.

A meaningful fraction of the *source* god-files' bulk is **inline test
modules**, not production logic. `live_semantic_sink/mod.rs` and `perf.rs` are
roughly half tests; `composition/mod.rs`, `compose.rs`, `env.rs`, `wiring.rs`,
and `watchdog.rs` each carry 200–650 lines of inline tests. That observation
drives the lowest-risk, highest-yield lever below.

> Reproduce the live report any time with `hug god-files --high-risk`.

> Review note: the command above is repo-wide and currently includes many
> unrelated package areas. Use `hug god-files claudine/cli --high-risk --plain`
> for the scoped claudine-cli gate. Physical line counts from `wc -l` are larger
> than `hug`'s effective SLOC because `hug` excludes blanks/comments; use `hug`
> values when deciding whether a file is out of the high-risk band.

## How "high risk" is actually measured (verified against tree-hugger)

The risk band is **purely a function of effective SLOC** — not imports, depth,
or symbol count:

```rust
// tree-hugger/lib/src/god_files/constants.rs
pub const MODERATE_MIN_SLOC: usize = 400;
pub const HIGH_MIN_SLOC:     usize = 1000;
```

`RiskBand::from_sloc` bands a file High at ≥ 1000 effective SLOC and Moderate at
≥ 400. The "24 imports", "depth 5", and "166 unrelated top-level symbols" lines
are **advisory refactor hints** attached after banding; they do not change the
band. Effective SLOC counts inline `#[cfg(test)]` modules.

This gives a concrete, measurable exit gate:

- **Mandatory:** every one of the 15 files drops below **1000** effective SLOC
  (out of the high band).
- **Preferred (where a natural cohesive split exists):** below **400** (off the
  report entirely).
- **Anti-gaming guardrail:** moving 1500 lines of tests into a *single* sibling
  file that is itself ≥ 1000 SLOC just relocates a god-file. Splits must land
  every produced file under the relevant threshold, and must be by
  responsibility — not by mechanically spreading lines to satisfy the metric.

## Goals

1. All 15 reported files drop out of the **high-risk** band (< 1000 effective
   SLOC), with no newly-created file entering it.
2. The refactor is **behavior-preserving**: a pure restructure. No change to CLI
   output, exit codes, timeout semantics, provider wiring, or stream rendering.
3. Splits follow the module/responsibility seams the report already names, and
   reuse the submodule directories that **already exist** under
   `wrap/exec/`, `wrap/composition/`, `wrap/profile/`, and
   `wrap/live_semantic_sink/`.
4. The existing test suite stays green at every phase; each phase leaves the
   build compiling and independently shippable.
5. The final verification is scoped and reproducible:
   `hug god-files claudine/cli --high-risk --plain` reports zero files.

## Non-Goals

- **No behavior changes.** Not a bug-fix or feature pass. If a latent bug is
  found mid-refactor, file it separately; do not fix it inside a "move" commit
  (Rule 3 — surgical; and the repo's scope-discipline rule for refactor vs.
  behavior commits).
- **No `cargo fmt` / `rustfmt` write-mode.** `main` is the formatting authority;
  match surrounding style by hand when moving code (repo convention; ad-hoc fmt
  poisons branch↔main merges).
- **No change to tree-hugger's thresholds** to make the report pass. The metric
  is the gate, not a thing we tune to win.
- **No new public API.** Extractions are crate-internal (`pub(crate)` / private
  module items). The `claudine-cli` binary surface is unchanged.
- **No reach into the `claudine` library or `claudine-contract`.** This is a
  CLI-crate-only restructure.

## Guiding constraints

- **Cross-OS.** Several targets carry `cfg(unix)` / `cfg(not(unix))` branches
  (e.g. `wiring.rs::install_sigint_forwarder`). Preserve the cfg split verbatim
  when relocating; the crate must still compile on macOS, Windows, and Linux.
- **Test relocation pattern is already sanctioned.** The library uses
  `#[cfg(test)] mod tests;` in `mod.rs` pointing at a sibling `tests.rs`
  (`lib/src/provider/tests.rs`, `lib/src/agents/tests.rs`). Reuse exactly this
  pattern — it keeps private-item access (unit tests stay in-crate) while moving
  the bytes out of the production file. No `#[path]` gymnastics required.
  For non-`mod.rs` modules such as `env.rs`, `watchdog.rs`, `wiring.rs`,
  `sequence.rs`, `harness_orch.rs`, and `perf.rs`, the child test module lives
  under the module-name directory (`env/tests.rs`, `exec/watchdog/tests.rs`,
  `exec/wiring/tests.rs`, `sequence/tests.rs`, `harness_orch/tests.rs`,
  `perf/tests.rs`). Do not create ad-hoc `*_tests.rs` siblings.
- **Integration tests are separate binaries.** Files under `cli/tests/` each
  compile as their own test binary, so splitting one into themed files is
  free of shared-state hazards — but it raises link count. `cli/tests/common/`
  already exists; extend it deliberately instead of creating a second helper
  convention or copy-pasting fixtures.
- **Child module placement follows Rust's file-module rules.** If a module is
  currently a file (`sequence.rs`, `harness_orch.rs`, `commands/compose.rs`),
  its extracted children belong under the matching directory
  (`sequence/step.rs`, `harness_orch/attempt.rs`, `commands/compose/looping.rs`)
  while the original file remains the parent orchestrator. Promoting a file to
  `mod.rs` is allowed only when it reduces net wiring churn; it is not required
  for child modules.
- **`hug` is the gate.** Re-run `hug god-files claudine/cli --high-risk --plain`
  after each file to confirm the band drop and that nothing regressed into high.
  Run `just build`, `just lint`, `just test-cli`, and the relevant `just test-l2`
  filter for PTY-touched files before declaring a phase done.

## Two levers

**Lever A — extract inline tests to a child `tests.rs` module (or themed test
modules).** Pure relocation, zero production-logic risk. For files whose inline
test module is < ~600 lines, one child `tests.rs` drops the production file
under threshold. For files that are ~half tests (`live_semantic_sink/mod.rs`,
`perf.rs`), split the tests into **themed** child modules so no single test file
re-enters the high band.

**Lever B — split production by responsibility / decompose mega-functions.**
Move cohesive symbol groups into submodules (most parent directories already
exist), and break the mega-functions into named, sequential stage functions that
read as a pipeline. Highest care, lands last.

Most files need both levers; a few need only one. Lever A is sequenced first
because it is the cheapest SLOC reduction and may by itself drop several files
out of high-risk, shrinking the Lever-B scope before the risky work begins.

## Per-file plan

Effective-SLOC figures are from the cited `hug god-files --high-risk` run; line
numbers will have drifted — re-derive seams from the live report and code.

Review refresh, 2026-06-18: `hug god-files claudine/cli --high-risk --plain`
still reports the same 15 claudine-cli files and the same effective-SLOC values
listed below. `wc -l` shows larger physical files in the working tree; that does
not change the gate.

### Group A — pure test files (Lever A only; split by theme)

| File | SLOC | Plan |
|------|------|------|
| `tests/wrap_commands.rs` | 5986 | Split 166 tests into themed binaries: `wrap_structured_stream.rs`, `wrap_watchdog_timeout.rs`, `wrap_inline_compose.rs`, `wrap_sigint.rs`, `wrap_opencode.rs`, plus a residual. Hoist shared spawn/fixture helpers into the existing `tests/common/`. |
| `tests/sequence_cli.rs` | 1250 | Split by concern: fail-fast propagation, magic-reference resolution, schema/required aggregation, per-step `step_timeout`, prompt-property inline. |
| `tests/level2_schema_prompt_pty.rs` | 1134 | Split schema-prompt PTY tests from sequence-overlay PTY tests; share the PTY harness setup via a common helper module. |

Each produced file must land < 1000 SLOC; aim < 600 so they stay clearly off the
high band.

### Group B — mega-function files (Lever A + Lever B)

- **`wrap/composition/mod.rs` (2274 SLOC; `execute_composition_request_inner` 908).**
  Extract the inner pipeline into named stages under the existing
  `wrap/composition/` dir (`prep_context.rs`, `dry_run.rs` already exist): target
  resolution, header emission, prepare, dispatch, outcome assembly. Move inline
  tests to `composition/tests.rs` (or themed siblings). The report's "depth 5"
  hint resolves as the stages flatten.
- **`wrap/mod.rs` (1257 SLOC; `run_provider_wrapper_inner` 862, 90 imports).**
  The wrap pipeline already has `exec/`, `env.rs`, `repo_home.rs`, `overlay.rs`
  submodules — push the inner stages into them: startup detection,
  `bootstrap_mcp_state`, env assembly (→ `env.rs`), spawn/stream/exit (→
  `exec/`). `run_provider_wrapper_inner` should shrink to an orchestrator that
  calls named stages. Move inline tests to `wrap/tests.rs`. The 90-import
  fan-out drops naturally as logic leaves the file.
- **`wrap/sequence.rs` (1092 SLOC; `execute_sequence` 550).** Extract the
  per-step iteration loop, the result/reporting assembly, and the Phase-1c
  schema path into child modules under `wrap/sequence/` (`step.rs`,
  `reporting.rs`, `phase1c.rs`) while keeping `wrap/sequence.rs` as the parent
  orchestrator unless promotion to `sequence/mod.rs` clearly reduces churn.
  Inline tests (~125) → `sequence/tests.rs`.
- **`wrap/harness_orch.rs` (1164 SLOC; `run_harness_loop` 509 +
  `execute_harness_attempt` 298, no inline tests).** Pure Lever B — needs real
  decomposition. Split into child modules under `wrap/harness_orch/`: loop
  control, attempt execution, prompt materialization
  (`materialize_harness_prompt` 88), launch building. This file has no test
  bytes to shed, so the function split is the only path under threshold.
- **`commands/compose.rs` (1458 SLOC; `run_inline_compose_inner` 369 +
  `run_compose_inner` 299).** The two `*_inner` functions share substantial prep
  and loop scaffolding — factor the shared prepare/loop into helpers (or a
  `commands/compose/` child module) and keep each entrypoint thin. Inline tests
  (~370, starting ~line 1700 in the physical file) → `commands/compose/tests.rs`.
- **`wrap/exec/wiring.rs` (1356 SLOC; `run_kimi_wire_session` 187, 55 imports).**
  Kimi wire-mode is a self-contained protocol. Extract into a `wrap/exec/wire/`
  submodule: session lifecycle, request dispatch (`handle_request_dispatch`,
  `dispatch_hook_request`), the `WireWriter`, and exit handling. Preserve the
  `cfg(unix)`/`cfg(not(unix))` `install_sigint_forwarder` pair. Inline tests
  (~650) → themed `wire/tests/*` or `wire/tests.rs`.

### Group C — wide-surface files (Lever A + targeted Lever B)

- **`wrap/live_semantic_sink/mod.rs` (3070 SLOC; ~half tests; 39 imports).**
  Submodules already exist (`sections.rs`, `spacing.rs`, `thinking.rs`,
  `tool_calls.rs`, `heartbeat.rs`, `errors.rs`). Move `render_event` (127),
  `on_semantic_event` (116), and `summarize_provider_payload` (79) into focused
  submodules. Split the large inline test module into **themed** sibling test
  files (combined-section golden, OpenCode acceptance/replay, glyph rendering)
  so none re-enters high. This file is the single biggest win for Lever A.
- **`cli/src/perf.rs` (2035 SLOC; ~half tests; 37 symbols).** Split production
  into a `perf/` submodule: report model + `into_report_with_elapsed`, tree
  assembly (`build_perf_tree`), and rendering. Split the test half into themed
  siblings (render snapshot/golden, accumulator merge, tree assembly).
- **`wrap/profile/mod.rs` (2019 SLOC; `WrapperProfile` trait 258 / 27 members).**
  Provider impls already live in `profile/{claude,codex,…}.rs`. Keep the trait
  declaration in `mod.rs`; move the large default-method bodies (`apply_yolo`,
  `apply_output_format`, `apply_entrypoint`, …) and free helpers
  (`apply_opencode_model_resolution`, `find_positional_prompt_index`) into a
  `profile/apply.rs` / `profile/resolve.rs`. Inline tests → `profile/tests.rs`
  (themed if needed).
- **`wrap/exec/watchdog.rs` (1771 SLOC; 34 imports).** Split into: timeout
  evaluation (`evaluate_timeout_tick`), breach-message formatting
  (`format_step_timeout_breach_message`, `maybe_emit_step_timeout_warn`), and
  the monitor spawners (`spawn_prompt_timing_monitor`,
  `spawn_flush_if_idle_ticker`). Inline tests → themed siblings (timeout-rule
  tests vs. OpenCode breach-diagnostic tests).
- **`wrap/env.rs` (1123 SLOC; 20 symbols).** Split: child-env assembly
  (`build_child_env*`), monorepo package-context resolution
  (`resolve_monorepo_package_context`, package-area selection), and
  sanitize/redact (`sanitize_process_env`, `redact_sensitive_args`). Inline
  tests → `env/tests.rs` or themed children under `env/tests/`.
- **`config_tui/tabs/messenger/mod.rs` (1374 SLOC; depth 5).** Split the tab
  into: `render` (243), key/input handling (`handle_messenger_input_modal` 215,
  `handle_messenger_select_modal` 90, `handle_key`), leaving `mod.rs` as the
  wiring. Inline tests → `messenger/tests.rs` or themed children under
  `messenger/tests/`. Preserve the webhook-redaction invariants exactly (masked
  input, `redact_webhook_urls` on every error path).

## Phasing

Each phase ends green (`just build`, `just lint`, `just test-cli`, and targeted
`just test-l2` when PTY tests or PTY harness code moved) and re-runs
`hug god-files claudine/cli --high-risk --plain` to re-prioritize. The full
`just all` gate is the final pass; it includes library and contract checks even
though the intended code changes stay in `claudine-cli`.

1. **Phase 0 — baseline.** Capture the current `hug god-files --high-risk`
   output, the scoped `hug god-files claudine/cli --high-risk --plain` output,
   and a full green `just all` run as the reference. No code changes.
2. **Phase 1 — test extraction (Lever A) across all source god-files.** Pure
   relocation via the sanctioned `#[cfg(test)] mod tests;` sibling pattern. Touch
   no production logic. Re-run `hug` after each file — several files
   (`live_semantic_sink/mod.rs`, `perf.rs`, `env.rs`, `watchdog.rs`,
   `wiring.rs`, `composition/mod.rs`, `compose.rs`) should drop band or shrink
   substantially, pruning later scope.
3. **Phase 2 — pure test-file splits (Group A).** `wrap_commands.rs` and the two
   smaller test files, with shared helpers hoisted to `tests/common/`.
4. **Phase 3 — wide-module responsibility splits (Group C).** Mechanical moves of
   cohesive symbol groups into existing/new submodules. Lower risk than Group B
   because the symbols are already independent; the report calls them "unrelated
   top-level symbols".
5. **Phase 4 — mega-function decomposition (Group B).** Highest risk; lands last
   with the most review. Extract sequential named stages; the orchestrator
   function becomes a readable call-list. `harness_orch.rs` is the hardest
   (no test bytes to shed — the function split is the whole job).

## Risks & mitigations

- **Silent behavior drift during a "move".** Mitigate: behavior-preserving
  extraction only; full nextest (L1) + the relevant L2/PTY suites must pass
  unchanged after each file. No logic edits inside relocation commits.
- **Merge poisoning from accidental reformat.** Mitigate: never run `cargo fmt`;
  move code verbatim and match surrounding style by hand. Keep diffs to pure
  cut/paste + `mod` wiring where possible.
- **Relocating a god-file instead of dismantling it** (Lever A into one giant
  sibling). Mitigate: the < 1000 gate applies to *every produced file*; test-
  heavy files split by theme.
- **Link-time growth from many new integration-test binaries.** Mitigate: prefer
  a handful of themed binaries over one-per-test; share helpers via
  `tests/common/`. Run with nextest.
- **cfg-gated code dropped during a move** (e.g. the unix/non-unix sigint pair).
  Mitigate: grep for `cfg(` in each target before/after the move; the post-move
  file must contain the same cfg arms.
- **Accidental helper API drift while splitting tests.** Mitigate: move shared
  integration-test helpers into the existing `tests/common/` only when at least
  two new test binaries need them; keep helper names behavior-oriented rather
  than themed around the old file.
- **Module privacy breakage during production extraction.** Mitigate: start
  extracted modules private, widen only to `pub(super)` / `pub(crate)` as needed,
  and avoid making new public `claudine-cli` APIs. If a move needs broad
  visibility just to compile, revisit the split.

## Success criteria

- `hug god-files claudine/cli --high-risk --plain` reports **0** high-risk
  files (every one of the original 15 is < 1000 effective SLOC), and
  `hug god-files claudine/cli --plain` shows **no new file** in the high band.
- No newly-created file is itself ≥ 1000 SLOC; test files split by theme each sit
  comfortably below threshold.
- The full claudine nextest suite (L1 + applicable L2/PTY) is green and
  unchanged in behavior; CLI output, exit codes, and timeout semantics are
  byte-for-byte the same as the Phase-0 baseline for representative commands.
- No `cargo fmt` was run; diffs are relocation + `mod` wiring, not reformatting.
- The crate still compiles for macOS, Windows, and Linux targets (cfg arms
  preserved).
- No change to the `claudine-cli` public/binary surface, the `claudine` library,
  or `claudine-contract`.

## Resolved review decisions

1. **Hard target — band-drop is mandatory; moderate cleanup is opportunistic.**
   Requiring every target to drop below 400 SLOC would turn this from a bounded
   de-risking pass into a broad architecture rewrite. Recommended decision:
   enforce < 1000 for every original and newly-created file, and pursue < 400
   only where the split is natural and behavior-preserving.

2. **Test-file granularity — themed binaries plus shared common helpers.**
   `tests/common/` already exists, so the implementation should extend it for
   spawn/fixture utilities used by multiple new binaries. Avoid a one-test-file
   explosion; each themed binary should stay under 1000 SLOC and preferably
   under 600 SLOC.

3. **Submodule shape — keep current file modules as parents first.** For
   `sequence.rs`, `compose.rs`, `harness_orch.rs`, `env.rs`, `watchdog.rs`,
   `wiring.rs`, and `perf.rs`, add child modules under the matching directory
   while the current file remains the orchestrator. Promote to `mod.rs` only if
   the implementation proves that file-module children create more churn than
   they save.

4. **Execution model — parallelize only the low-coupling phases.** Phase 1
   test extraction and Phase 2 integration-test splits can fan out by file.
   Phases 3 and 4 should serialize inside the `wrap/` tree because
   `wrap/mod.rs`, `composition`, `sequence`, `exec`, and harness code share
   imports, visibility, and wrapper pipeline contracts.

## Open questions

None. The remaining choices are implementation planning details, not design
blockers.
