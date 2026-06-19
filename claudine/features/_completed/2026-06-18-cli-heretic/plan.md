---
agent: codex/
phases: 5
created: 2026-06-18
start_phase: 1
yolo: "true"
---

# CLI Heretic — Execution Plan

Derived from [`spec.md`](spec.md). This is a behavior-preserving `claudine-cli`
restructure to dismantle the 15 high-risk god files reported by
`hug god-files claudine/cli --high-risk --plain`.

Success means every original target and every newly-created file stays below
1000 effective SLOC, no public CLI behavior changes, and the scoped `hug` gate
reports zero high-risk files.

## Phase 1 — Baseline and source test extraction

Capture the reference state first, then apply the lowest-risk Lever A work:
move inline unit tests out of production modules using the sanctioned
`#[cfg(test)] mod tests;` pattern. This phase is mostly parallelizable by file
after the baseline is captured.

- [ ] Capture baseline `hug god-files --high-risk` output for comparison, without changing code.
- [ ] Capture baseline `hug god-files claudine/cli --high-risk --plain` output and save the original 15-file target list.
- [ ] Run the baseline validation suite (`just build`, `just lint`, `just test-cli`, and relevant existing L2/PTY filters if already known) so later phases can distinguish refactor breakage from pre-existing failures.
- [ ] Move inline tests from `wrap/composition/mod.rs` to `wrap/composition/tests.rs` or themed child test modules, preserving private-item access.
- [ ] Move inline tests from `wrap/mod.rs` to `wrap/tests.rs`, preserving wrapper behavior and imports.
- [ ] Move inline tests from `wrap/sequence.rs` to `wrap/sequence/tests.rs`.
- [ ] Move inline tests from `commands/compose.rs` to `commands/compose/tests.rs`.
- [ ] Move inline tests from `wrap/exec/wiring.rs` to `wrap/exec/wiring/tests.rs` or themed child modules under the existing module hierarchy.
- [ ] Move inline tests from `wrap/live_semantic_sink/mod.rs` into themed sibling test modules so no produced test file reaches 1000 effective SLOC.
- [ ] Move inline tests from `cli/src/perf.rs` into themed `perf` child test modules.
- [ ] Move inline tests from `wrap/profile/mod.rs` to `wrap/profile/tests.rs` or themed children if one file would be too large.
- [ ] Move inline tests from `wrap/exec/watchdog.rs` into themed watchdog test modules.
- [ ] Move inline tests from `wrap/env.rs` to `wrap/env/tests.rs` or themed children.
- [ ] Move inline tests from `config_tui/tabs/messenger/mod.rs` to `config_tui/tabs/messenger/tests.rs` or themed children.
- [ ] After each file move, run `hug god-files claudine/cli --high-risk --plain` and record which files dropped out of the high-risk band.
- [ ] Verify no newly-created test module is reported at or above 1000 effective SLOC.

**Parallelizable:** the per-file test moves can fan out once baseline output is
captured. Avoid parallel edits inside the same module directory.

**Validation checkpoint:** `just build`, `just lint`, `just test-cli`, targeted
unit tests for every moved test module, and
`hug god-files claudine/cli --high-risk --plain` all run after the phase.

---

## Phase 2 — Split integration test god files

Split the pure integration-test files by behavior theme. Keep shared fixtures in
the existing `cli/tests/common/` convention instead of duplicating helpers.
This phase is parallelizable by original test file.

- [ ] Audit `cli/tests/wrap_commands.rs` and classify the 166 tests by theme before moving any code.
- [ ] Hoist spawn helpers, fixture builders, and repeated assertions from `wrap_commands.rs` into `cli/tests/common/` only when at least two new binaries need them.
- [ ] Split `wrap_commands.rs` into themed binaries such as `wrap_structured_stream.rs`, `wrap_watchdog_timeout.rs`, `wrap_inline_compose.rs`, `wrap_sigint.rs`, `wrap_opencode.rs`, plus a residual file if needed.
- [ ] Verify every new `wrap_*` integration test file is below 1000 effective SLOC, preferably below 600.
- [ ] Split `cli/tests/sequence_cli.rs` by concern: fail-fast propagation, magic-reference resolution, schema aggregation, per-step `step_timeout`, and prompt-property inline behavior.
- [ ] Split `cli/tests/level2_schema_prompt_pty.rs` into schema-prompt PTY coverage and sequence-overlay PTY coverage.
- [ ] Move shared PTY harness setup from the split PTY tests into `cli/tests/common/` without changing PTY semantics.
- [ ] Run each newly-created integration test binary directly or with the narrowest available `just test-cli` / nextest filter.
- [ ] Re-run `hug god-files claudine/cli --high-risk --plain` and confirm the three pure test files no longer appear.

**Parallelizable:** `wrap_commands.rs`, `sequence_cli.rs`, and
`level2_schema_prompt_pty.rs` can be split by separate implementers after common
helper ownership is agreed.

**Validation checkpoint:** all split integration test binaries pass; PTY tests
run through the existing L2/PTY recipe; the scoped `hug` report shows no
high-risk files created under `cli/tests/`.

---

## Phase 3 — Wide-surface responsibility splits

Move cohesive, already-independent symbol groups into focused submodules. These
are production moves, so serialize edits within the `wrap/` tree and keep each
parent as the orchestrator unless promoting to `mod.rs` clearly reduces churn.

- [ ] Split `wrap/live_semantic_sink/mod.rs` by moving `render_event`, `on_semantic_event`, and `summarize_provider_payload` into focused existing submodules such as `sections`, `tool_calls`, `thinking`, `heartbeat`, or `errors`.
- [ ] Verify live semantic sink golden/replay tests still pass and combined-section, OpenCode, and glyph rendering behavior is unchanged.
- [ ] Split `cli/src/perf.rs` into `perf/` child modules for report model conversion, tree assembly, and rendering.
- [ ] Verify perf accumulator, tree assembly, and render snapshot/golden tests still pass.
- [ ] Split `wrap/profile/mod.rs` by keeping the `WrapperProfile` trait declaration in place and moving large default-method bodies into `profile/apply.rs` and resolution helpers into `profile/resolve.rs`.
- [ ] Verify provider profile tests still cover `apply_yolo`, output format handling, entrypoint handling, OpenCode model resolution, and positional prompt detection.
- [ ] Split `wrap/exec/watchdog.rs` into timeout evaluation, breach-message formatting, and monitor spawner modules.
- [ ] Verify watchdog timeout-rule tests and OpenCode breach-diagnostic tests still pass.
- [ ] Split `wrap/env.rs` into child-env assembly, monorepo package-context resolution, and sanitize/redact modules.
- [ ] Verify environment redaction tests and package-context resolution tests still pass on macOS and preserve cross-OS path assumptions.
- [ ] Split `config_tui/tabs/messenger/mod.rs` into rendering, key/input handling, and modal-selection modules.
- [ ] Verify webhook redaction invariants: raw webhook URLs never render, secret buffers stay masked, and all webhook errors pass through `redact_webhook_urls`.
- [ ] Run `hug god-files claudine/cli --high-risk --plain` after each wide-surface file and confirm it drops below 1000 effective SLOC.

**Parallelizable:** `perf.rs` and `config_tui/tabs/messenger/mod.rs` can proceed
in parallel with one selected `wrap/` split. Serialize overlapping `wrap/`
imports and visibility changes.

**Validation checkpoint:** `just build`, `just lint`, `just test-cli`, targeted
tests for each split module, and the scoped `hug` report all pass.

---

## Phase 4 — Mega-function decomposition

Decompose the highest-risk orchestration functions into named sequential stages.
This is the most behavior-sensitive work and should land after test extraction
and wide-surface moves have reduced file size pressure.

- [ ] Decompose `wrap/composition/mod.rs::execute_composition_request_inner` into stages for target resolution, header emission, prepare, dispatch, and outcome assembly under `wrap/composition/`.
- [ ] Keep `execute_composition_request_inner` as a readable orchestrator and verify dry-run, prepare, dispatch, and schema-validation behavior remains unchanged.
- [ ] Decompose `wrap/mod.rs::run_provider_wrapper_inner` by moving startup detection, MCP bootstrap, environment assembly, spawn/stream handling, and exit handling into existing `wrap/exec/`, `wrap/env.rs`, `repo_home`, and overlay modules.
- [ ] Verify provider wrapper CLI output, exit codes, MCP injection behavior, timeout behavior, and stream rendering match the baseline.
- [ ] Decompose `wrap/sequence.rs::execute_sequence` into child modules for per-step iteration, result/reporting assembly, and Phase-1c schema behavior.
- [ ] Verify fail-fast propagation, shared shell approval cache, schema aggregation, magic references, and step timeout behavior remain unchanged.
- [ ] Decompose `wrap/harness_orch.rs` into modules for loop control, attempt execution, prompt materialization, and launch building.
- [ ] Verify `run_harness_loop`, `execute_harness_attempt`, and `materialize_harness_prompt` behavior with existing harness and composition tests.
- [ ] Decompose `commands/compose.rs` so `run_compose_inner` and `run_inline_compose_inner` share extracted prepare/loop scaffolding while each entrypoint remains thin.
- [ ] Verify compose and inline-compose output, file mutation behavior, `last_updated` handling, and exit codes match the baseline.
- [ ] Extract Kimi wire-mode protocol code from `wrap/exec/wiring.rs` into `wrap/exec/wire/` modules for session lifecycle, request dispatch, `WireWriter`, and exit handling.
- [ ] Preserve the `cfg(unix)` and `cfg(not(unix))` `install_sigint_forwarder` arms verbatim when relocating wiring code.
- [ ] Before and after the wiring move, grep the target files for `cfg(` and confirm every cross-OS branch still exists.
- [ ] Run `hug god-files claudine/cli --high-risk --plain` after each mega-function file and confirm each drops below 1000 effective SLOC.

**Parallelizable:** limited. `commands/compose.rs` can proceed separately from
`wrap/harness_orch.rs`; the remaining `wrap/` pipeline files should be
serialized because they share imports, visibility, and wrapper contracts.

**Validation checkpoint:** `just build`, `just lint`, `just test-cli`, targeted
wrapper/composition/sequence/harness tests, relevant L2/PTY filters, and the
scoped `hug` gate all pass.

---

## Phase 5 — Final gate, documentation, and release readiness

Close the refactor by proving the metric, behavior, and repository conventions
all hold. Documentation changes are only required if implementation discovers a
real workflow or architecture update; this refactor should not change public
behavior.

- [ ] Run `hug god-files claudine/cli --high-risk --plain` and confirm it reports zero files.
- [ ] Run `hug god-files claudine/cli --plain` and confirm no newly-created file is in the high-risk band.
- [ ] Compare final high-risk output against the Phase 1 baseline and confirm all original 15 files are below 1000 effective SLOC.
- [ ] Run `just build` and confirm `claudine-cli` compiles.
- [ ] Run `just lint` and confirm no new lint failures.
- [ ] Run `just test-cli` and confirm the CLI suite is green.
- [ ] Run the relevant `just test-l2` filters for PTY-touched files and confirm PTY behavior is green.
- [ ] Run `just all` as the final repository gate if time and machine constraints allow; record any pre-existing unrelated failures separately.
- [ ] Verify no `cargo fmt` / `rustfmt` write-mode was run and the diff is relocation plus module wiring rather than broad formatting churn.
- [ ] Verify no public `claudine-cli` command, flag, output format, exit code, timeout semantic, provider wiring behavior, or stream-rendering behavior changed.
- [ ] Verify no changes reached into the `claudine` library or `claudine-contract` except for unavoidable compile fallout; if any did, document why and review scope.
- [ ] Update README/topic docs, `.claude/skills/`, or this feature documentation only if the refactor changed architecture, workflows, or public behavior.

**Parallelizable:** final validation commands are mostly sequential because each
gate depends on the completed tree. Documentation review can run in parallel
with non-mutating validation.

**Validation checkpoint:** scoped `hug` reports zero high-risk claudine-cli
files, no new high-risk files exist, and the full requested validation suite is
green or has explicitly documented pre-existing failures.

## Cross-phase guardrails

- [ ] Treat every phase as behavior-preserving; if a latent bug appears, file it separately and do not fix it inside a move/decomposition task.
- [ ] Keep extracted modules private by default and widen only to `pub(super)` or `pub(crate)` when required by the module boundary.
- [ ] Preserve cross-OS `cfg` branches exactly during moves, especially in wiring and process/session code.
- [ ] Do not run `cargo fmt` or `rustfmt` write-mode.
- [ ] Use `hug` effective SLOC, not `wc -l`, to determine whether the high-risk gate is satisfied.
- [ ] Apply the <1000 effective-SLOC gate to every produced file, including test files.
