# Phase 1 — Baseline, blast radius, and characterization

This is the frozen pre-refactor record for Sequence Plus. It captures the blast
radius of the HIGH-risk sequence symbols, the replacement seams between library
normalization and CLI orchestration, the retained-behavior characterization
mapping, the clean-break/blocked regression tests, and the L1 baseline result.

> **GitNexus note.** The repo-root `CLAUDE.md` GitNexus block is currently in a
> merge-conflict state with an empty `HEAD` side, so GitNexus guidance is not
> active on this branch and its index may be stale. Rather than depend on a
> possibly-stale MCP index in a non-interactive session, the impact/seam
> analysis below was produced with static call-graph analysis (ripgrep over
> `lib/src` and `cli/src`). It records the same substance the tasks require:
> direct callers and the exact replacement seams. Re-run GitNexus
> `impact`/`detect_changes` in Phase 13 once the index and the `CLAUDE.md`
> conflict are resolved.

## Blast radius — direct callers of HIGH-risk symbols

| Symbol | Defined in | Direct callers | Risk |
|---|---|---|---|
| `resolve_sequence_plan` | `lib/src/composition/sequence.rs` | re-exported `lib/src/composition/mod.rs:111`; called `cli/src/commands/sequence.rs:225` | HIGH — the single library entry that detects/normalizes a sequence |
| `build_step_overlay` | `lib/src/composition/sequence.rs` | re-exported `mod.rs:111`; called `cli/src/commands/wrap/sequence/phase1c.rs:214` | HIGH — emits the seven-key overlay the clean break replaces |
| `execute_sequence` | `cli/src/commands/wrap/sequence/mod.rs` | `cli/src/commands/sequence.rs:257` (command entry) | HIGH — top-level orchestrator |
| `run_phase_1c_with_schema` | `cli/src/commands/wrap/sequence/phase1c.rs` | `cli/src/commands/wrap/sequence/mod.rs:406` | HIGH — the eager all-step preparation the JIT model retires |
| `run_sequence_steps` | `cli/src/commands/wrap/sequence/iterate.rs` | `cli/src/commands/wrap/sequence/mod.rs:460` | MEDIUM — Phase 2 serial execution loop |
| `execute_composition_request_inner_with_guard` | `cli/src/commands/wrap/composition/mod.rs` | `cli/src/commands/wrap/composition/mod.rs:147,171`; documented from `looping/engine.rs:391` | HIGH — the shared wrapper-grade executor used by `compose`, `inline-compose`, and every sequence step |

Owner warning (recorded for the implementer): every symbol above is HIGH-risk
except `run_sequence_steps` (MEDIUM). `execute_composition_request_inner_with_guard`
is shared with `compose`/`inline-compose`; any change to it must keep those two
commands' behavior identical (guarded by their own integration suites).

## Current sequence execution flow and replacement seams

Flow (command → orchestrator → prepare → execute → shared executor):

1. `commands::sequence::run_sequence` — arg parse, interactive rejection, source
   resolve, `resolve_sequence_plan` (`sequence.rs:225`) → `execute_sequence`.
2. `wrap::sequence::execute_sequence` (`mod.rs`) — fail-fast resolution, shared
   approval cache, interrupt flag, provider/model resolution (Phase 1a/1b).
3. `run_phase_1c_with_schema` (`phase1c.rs`) — per step: `build_step_overlay`
   (`:214`) → schema pre-validate → shell approval → prepare `PreparedComposition`.
   **Eager**: all steps prepared before any provider launches.
4. `iterate::run_sequence_steps` — Phase 2 serial loop dispatching each prepared
   step through `execute_composition_request_inner`.
5. `wrap::composition::…::execute_composition_request_inner_with_guard` — the
   shared executor (also drives `compose`/`inline-compose`).

Replacement seams for later phases (where new code slots in without disturbing
`compose`/`inline-compose`):

- **Library normalization** — `resolve_sequence_plan` / `build_step_overlay` /
  `SequenceStepOverlay` (Phases 3–4: typed states, `previous`/`next`,
  `index`/`count`, source variants, `list:` removal).
- **Provider/model resolution** — Phase 1a/1b in `execute_sequence` (retained;
  Phase 5 keeps one stable per-task target vector).
- **Preparation** — `run_phase_1c_with_schema` (Phase 8 replaces the stored
  `PreparedComposition` vector with immutable preflight nodes + a runtime cell).
- **Execution/reporting** — `run_sequence_steps` + `SequenceRunSummary`
  (Phases 6–11: runtime layers, `outputs`, groups, concurrent rendering).
- **Shared composition executor** — `execute_composition_request_inner_with_guard`
  (Phase 6 extends its outcome with captured undecorated final stdout).

There is a known **stale comment** in `cli/src/commands/wrap/sequence/mod.rs`
claiming each step re-reads the live file and sees the prior rewritten body; the
eager Phase-1c architecture does not do this. Its deletion is scheduled for
Phase 12 (per `current-state.md`), not Phase 1.

## Retained-behavior characterization mapping

Phase 1 freezes retained behavior. Existing coverage already pins each retained
behavior; the mapping below is the guardrail set the refactor must keep green
(new Phase-1 additions are marked ★).

| Retained behavior | Test(s) |
|---|---|
| Inline scalar steps | `lib …sequence::tests::inline_scalar_list_normalizes_correctly` |
| Inline object steps (required `name`) | `…inline_object_list_requires_name`, `…inline_object_step_missing_name_fails`, `…inline_object_step_name_wrong_type_fails` |
| Current 7-key overlay shape ★ | `…sequence::tests::clean_break::characterize_current_overlay_keys` |
| Overlay neighbor/first/last/step values | `…overlay_for_single_step_sequence`, `…overlay_for_middle_step`; `types::tests` reserved-key precedence |
| Document-level inline-compose mode | `cli/tests/sequence_prompt_property.rs` (inline selection, body write-back, non-string reject) |
| Fail-fast precedence (CLI > doc > default) | `cli/tests/sequence_cli.rs` (fail-fast true/false + CLI precedence); `…fail_fast_false_from_frontmatter`, `…fail_fast_wrong_type_fails` |
| Aggregate missing properties | `cli/tests/sequence_schema.rs` (cross-step aggregation, setter satisfaction) |
| Dry-run target behavior | `cli/tests/wrap_sequence_composition.rs` (dry-run composition, unresolved agent rendering, no launch) |
| Shell approval sharing | `cli/tests/sequence_cli.rs` (shell whitelist reuse); orchestrator preflight tests |
| Ctrl+C exit `130` | interrupt coverage inherited through the wrapper executor; `level2_sequence_overlay_pty.rs` review-cancel path |

## Clean-break / blocked regression tests (target behavior, `#[ignore]`d)

These encode the deliberate removals and the blocked construct. They are
`#[ignore]`d — not deleted — so the phase harness (which gates each phase on a
passing `just test`) stays green while the target assertions are preserved
verbatim. Each is confirmed to **fail today** under `--run-ignored=all`, proving
it captures not-yet-implemented behavior. The phase that lands the change
un-ignores its test (and updates the paired characterization test).

| Test (`lib …sequence::tests::clean_break::`) | Target | Un-ignore in |
|---|---|---|
| `legacy_overlay_names_removed` | overlay no longer emits `previous_state`/`next_state`/`step`/`total_steps` | Phase 3 |
| `external_list_shape_rejected` | `kind: sequence` + `list:` YAML rejected | Phase 4 |
| `group_loop_rejected` | a step's `group.loop` rejected with a typed error | Phase 9 |

## L1 baseline (`just test` in `claudine/`, pre-refactor)

Run on branch `error-prop-and-file-resolution`, all packages green:

| Package | Result |
|---|---|
| `claudine-catalog-types` | 21 passed, 0 skipped |
| `claudine` (lib) | 3553 passed (10 slow), 10 skipped |
| `claudine-contract` | 47 passed, 5 skipped |
| `claudine-cli` | 1997 passed (92 slow, 1 flaky), 170 skipped |
| `claudine-gen` | 152 passed, 4 skipped |

Notes:

- The lib count includes the new `characterize_current_overlay_keys`; the three
  ignored `clean_break` tests are counted among the lib "skipped".
- The one flaky CLI test — `argv_normalization::headline_compose_with_trailing_help_renders_help`
  — is pre-existing and unrelated to sequence work; it passed on nextest retry.
