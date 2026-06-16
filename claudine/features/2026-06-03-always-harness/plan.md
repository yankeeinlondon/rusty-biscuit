---
phases: 7
created: 2026-06-15
start_phase: 1
packages:
  - claudine
source_files_during_phase_1:
  - claudine/cli/tests/compose_cli.rs
  - claudine/cli/tests/inline_compose_cli.rs
  - claudine/cli/tests/loop_cli.rs
docs_updated_during_phase_1:
  - claudine/features/2026-06-03-always-harness/plan.md
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - claudine/cli/src/commands/wrap/harness_orch.rs
  - claudine/cli/src/commands/wrap/composition/mod.rs
  - claudine/cli/src/commands/wrap/mod.rs
  - claudine/cli/tests/loop_cli.rs
docs_updated_during_phase_2:
  - claudine/features/2026-06-03-always-harness/plan.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - claudine/lib/src/harness/parse/mod.rs
  - claudine/lib/src/harness/mod.rs
  - claudine/cli/src/commands/wrap/harness_orch.rs
  - claudine/cli/src/commands/wrap/composition/mod.rs
docs_updated_during_phase_3:
  - claudine/features/2026-06-03-always-harness/plan.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
  - claudine/cli/src/commands/wrap/composition/mod.rs
  - claudine/cli/src/commands/wrap/harness_orch.rs
  - claudine/cli/src/commands/wrap/composition/inline_guards.rs
  - claudine/cli/src/commands/wrap/policy.rs
  - claudine/cli/src/commands/wrap/mod.rs
docs_updated_during_phase_4:
  - claudine/features/2026-06-03-always-harness/plan.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
source_files_during_phase_5:
  - claudine/cli/src/commands/wrap/composition/mod.rs
  - claudine/cli/src/commands/wrap/composition/structured.rs
  - claudine/cli/src/commands/wrap/composition/legacy_goose.rs
  - claudine/cli/src/commands/wrap/composition/summary.rs
  - claudine/cli/src/commands/wrap/composition/inline_guards.rs
  - claudine/cli/src/commands/wrap/mod.rs
  - claudine/cli/src/commands/wrap/policy.rs
docs_updated_during_phase_5:
  - claudine/features/2026-06-03-always-harness/plan.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5: []
source_files_during_phase_6: []
docs_updated_during_phase_6:
  - claudine/docs/topics/non-interactive-sessions.md
  - claudine/docs/topics/execution-flow.md
  - claudine/docs/pipeline.md
  - claudine/docs/topics/provider-metadata.md
  - claudine/README.md
  - claudine/lib/README.md
  - claudine/cli/README.md
  - claudine/features/2026-06-03-always-harness/plan.md
docs_created_during_phase_6: []
skills_files_updated_during_phase_6:
  - .claude/skills/claudine/SKILL.md
  - .claude/skills/claudine/timeline.md
source_files_during_phase_7: []
docs_updated_during_phase_7:
  - claudine/features/2026-06-03-always-harness/plan.md
  - claudine/features/2026-06-03-always-harness/spec.md
docs_created_during_phase_7: []
skills_files_updated_during_phase_7: []
source_code:
  - claudine/cli/tests/compose_cli.rs
  - claudine/cli/tests/inline_compose_cli.rs
  - claudine/cli/tests/loop_cli.rs
  - claudine/cli/src/commands/wrap/harness_orch.rs
  - claudine/cli/src/commands/wrap/composition/mod.rs
  - claudine/cli/src/commands/wrap/mod.rs
  - claudine/lib/src/harness/parse/mod.rs
  - claudine/lib/src/harness/mod.rs
  - claudine/cli/src/commands/wrap/composition/inline_guards.rs
  - claudine/cli/src/commands/wrap/policy.rs
  - claudine/cli/src/commands/wrap/composition/structured.rs
  - claudine/cli/src/commands/wrap/composition/legacy_goose.rs
  - claudine/cli/src/commands/wrap/composition/summary.rs
documentation:
  - claudine/features/2026-06-03-always-harness/plan.md
  - claudine/docs/topics/non-interactive-sessions.md
  - claudine/docs/topics/execution-flow.md
  - claudine/docs/pipeline.md
  - claudine/docs/topics/provider-metadata.md
  - claudine/README.md
  - claudine/lib/README.md
  - claudine/cli/README.md
  - claudine/features/2026-06-03-always-harness/spec.md
---

# Always-Harness Unification Execution Plan

High-confidence plan for collapsing Claudine's two composition execution paths into a single harness-loop pipeline, derived from `claudine/features/2026-06-03-always-harness/spec.md`.

## Context

- **Repository**: rusty-biscuit monorepo, `claudine/` package area (`claudine/lib/` core + `claudine/cli/` wrapper).
- **Single execution entry point**: `execute_composition_request_inner` in `claudine/cli/src/commands/wrap/composition/mod.rs` (public `execute_composition_request` wraps it to return just the exit code). The `compose --loop` engine and the sequence orchestrator both call the `_inner` form.
- **The two paths today** (`composition/mod.rs`):
  - `harness_enabled == true` branch (≈`mod.rs:1964`) → `run_harness_loop(...)`.
  - `else` branch (≈`mod.rs:2043`) → `execute_without_harness(...)` → `structured::run_structured_branch` (structured) or `legacy_goose::*_branch` (captured), then `inline_guards::apply_inline_closure` / direct summary.
- **Key discovery that de-risks this work**: `run_harness_loop` already **re-parses the plan internally** every attempt from `materialized.frontmatter` via `parse_harness_plan` (`harness_orch.rs:790`), and already **inserts the inline writability pre-check itself** for inline mode (`harness_orch.rs:823`). For a document with no harness properties, `parse_harness_plan` yields the empty/bare plan. The harness attempt also already handles **both** modes: structured stream (`harness_orch.rs:493`) and captured/non-structured (`harness_orch.rs:633`), and already prefers `final_response` for inline (`harness_orch.rs:613`). So the loop is functionally a superset of `execute_without_harness` today.
- **Two real gaps the refactor must close** (from architect review):
  1. **Loop iteration signals.** The harness branch hardcodes `iteration_signals: None` (`mod.rs:2035`). `compose --loop` reads `SingleCompositionOutcome.iteration_signals` (built in the non-harness path at `mod.rs:2133`/`2236` via `IterationSummarySignals::from_summary`) for rate-limit pauses (`LoopRateLimited`) and honest `LoopIterationFailed` causes. Moving direct compose onto the loop without surfacing these would regress `compose --loop`.
  2. **Inline writability ownership.** Today the preflight (`mod.rs:1748-1781`) parses + inserts + **drops** a plan purely for validation, while the loop (`harness_orch.rs:823`) inserts the real one. The loop stays the sole *executor*, but the helper that produces the effective plan must own the single insertion so the two never diverge.
- **Interactive dimension** (landed 2026-06-15, after the spec was drafted): `interactive` is not a harness key, so `interactive: true` docs currently reach the agent only through `execute_without_harness`. The interactive gates (`supports_interactive_inline_closure` at `mod.rs:1134`, the interactive↔timeout conflict at `mod.rs:1564`, the capture seam `use_structured || (session_interactive && is_inline)`) live in the **shared prelude before the branch**, so they already cover both paths and need no relocation. `apply_inline_closure` already ignores its `session_interactive` arg (`_session_interactive`).
- **Helpers that must survive the cleanup**: `inline_guards::cleanup_inline_output` and `split_frontmatter_and_body` are reused by `wrap/inline.rs::try_inline_closure` (`inline.rs:185`) and re-exported (`mod.rs:58`). Only `apply_inline_closure` (+ its private `report_interruption`) is dead after unification.

## Integration Strategy

Order the work so the tree compiles and the safety net runs at every step: (1) lock current behavior behind tests, (2) make the **existing** harness branch loop-signal-correct while both paths still exist, (3) centralize effective-plan construction, (4) flip the `else` branch to call `run_harness_loop`, (5) delete the now-dead path, (6) docs, (7) verify. The risky flip (Phase 4) happens only after the harness branch is already proven signal-correct (Phase 2) and the convergence tests exist (Phase 1).

---

## Phase 1: Regression Safety Net

**Goal**: Pin current observable behavior so divergence introduced by later phases fails loudly. These tests must pass against the current dual-path code.

**Depends on**: Nothing.

**Parallelizable**: Steps are independent.

### Step 1.1: Convergence tests for body output (`compose_cli.rs`, `inline_compose_cli.rs`) — ✅ complete
- [x] Using the existing `assert_cmd` + stub-provider pattern already used in these test files, add paired cases that run the **same** document twice: once with no harness frontmatter and once with a minimal harness property (e.g. `post_checks: []` or a trivial `timeout`).
- [x] Direct `compose`: assert identical stdout body for both variants.
- [x] `inline-compose`: the stub provider must emit **interstitial narration before a tool call** and **final body content after the last tool call**; assert both variants write the **same final body** to the target file (this guards the `final_response` contract — note the harness path already satisfies it, so this is a convergence guard, not a bug reproduction).

**Observable**: New tests pass on the current tree (both paths already converge for body output).

### Step 1.2: Loop-signal regression test (`loop_cli.rs`) — ✅ complete
- [x] Add a `compose --loop` case where the stub provider emits a rate-limit trailer / structured `exit_reason`, asserting the loop surfaces it (rate-limit pause or `LoopIterationFailed` cause).
- [x] Run it against a **non-harness** document so it passes today, and add a sibling case with a minimal harness property marked `#[ignore]` (or `should_panic`-documented) capturing the **known current gap** (harness branch returns `iteration_signals: None`). Phase 2 un-ignores the sibling case.

**Observable**: Non-harness loop-signal case passes; the harness-variant case documents the gap Phase 2 closes.

### Step 1.3: Interactive convergence note — ✅ recorded as Phase 4 verification item
- [x] Where a stub provider supports interactive inline closure, add a Writer-seam / unit-level check (not a full PTY test — interactive subprocess tests are flaky per repo guidance) that interactive inline compose routes through the same closure entry and rewrites the body. If no seam exists, record this as a Phase 4 verification item rather than a CLI test.

> **Phase 4 verification item**: interactive inline closure routing. No Writer-level seam exists today that would allow a non-PTY unit check; the interactive gates (`supports_interactive_inline_closure`, interactive↔timeout conflict, and the `use_structured || (session_interactive && is_inline)` capture seam) are already shared in the prelude before the harness/non-harness branch split. Phase 4 must manually verify that `interactive: true` inline/direct compose reaches `run_harness_loop` and still rewrites the target body using the final response.

**Validation checkpoint**: `cargo test -p claudine-cli` runs the new tests; Step 1.1/1.2 (non-harness) green, harness-variant loop case ignored.

---

## Phase 2: Surface Loop Iteration Signals From the Harness Loop

**Goal**: Make `run_harness_loop` return the terminal attempt's iteration signals so direct `compose --loop` keeps rate-limit and exit-reason behavior. Done while both paths still exist, so the existing harness branch becomes signal-correct immediately.

**Depends on**: Phase 1.

**Parallelizable**: No (single call chain).

### Step 2.1: Capture signals in the harness attempt
- [x] In the structured branch of the attempt function (`harness_orch.rs:493-632`), after `summary` is finalized, build `IterationSummarySignals::from_summary(&summary)` (the type is CLI-side, in `composition/mod.rs`; import it).
- [x] In the captured/non-structured branch (`harness_orch.rs:633-681`) there is no `StreamExecutionSummary`; signals are `None`.
- [x] Carry the `Option<IterationSummarySignals>` out alongside the existing `AttemptOutcome`/`perf` tuple (extend the attempt function's return, or add a field — `AttemptOutcome` lives in `claudine::harness` (lib) and must **not** depend on the CLI type, so thread the signals as a separate CLI-side tuple element rather than adding them to `AttemptOutcome`).

### Step 2.2: Thread signals through `run_harness_loop`'s return
- [x] Change `run_harness_loop`'s return type from `Result<(i32, Option<AgentExecutionPerf>)>` to `Result<(i32, Option<AgentExecutionPerf>, Option<IterationSummarySignals>)>`.
- [x] Populate it from the **terminal** attempt only (the attempt the loop returns on). Intermediate handler-driven retries/redirects/resumes must not overwrite the terminal signals; ignore an intermediate rate-limit trailer unless the terminal attempt's summary also carries one (spec "Loop Iteration Signals").

### Step 2.3: Consume in the existing harness branch
- [x] At the harness branch in `mod.rs` (≈`1989`), capture the third return value and set `SingleCompositionOutcome.iteration_signals` from it instead of the hardcoded `None` (`mod.rs:2035`). Update the comment there that currently says `compose --loop` falls back to legacy behavior.
- [x] Update the one other current caller of `run_harness_loop` (wrapper passthrough, `emit_prompt_timing: false`) to ignore the new tuple element.

**Observable**: The Phase 1 harness-variant loop-signal test (Step 1.2) now passes; un-ignore it.

**Validation checkpoint**: `cargo check -p claudine-cli` clean; `cargo test -p claudine-cli` loop tests pass; `iteration_signals: None` no longer appears on the harness branch.

---

## Phase 3: Effective-Plan Helper and Single Writability Ownership

**Goal**: One helper produces the effective `HarnessPlan` (parsed-or-empty + system rules) so inline writability is injected exactly once, with pinned shape and ID/event invariants.

**Depends on**: Phase 2 (keeps tree green; independent of signal work but sequenced after to isolate diffs).

**Parallelizable**: Step 3.1 (lib + tests) is independent of 3.2/3.3.

### Step 3.1: Add the helper (`claudine/lib/src/harness/parse/mod.rs`)
- [x] Add `pub fn finalize_effective_plan(plan: HarnessPlan, mode: CompositionMode, source_path: &Path) -> HarnessPlan` (or accept a small `mode` bool/enum), which prepends `inline_writability_pre_check(source_path)` for inline mode and returns `plan` unchanged for direct mode.
- [x] Preserve author rule order: the system rule is prepended; authored `pre_checks` keep their relative order after it.
- [x] Preserve the system-rule ID convention `ValidationRuleId(u32::MAX)` and `ValidationKind`/`ValidationEvent::HasWritePermission` with no markdown source origin (these already come from `inline_writability_pre_check` at `parse/mod.rs:270`).
- [x] Export it from `claudine/lib/src/harness/mod.rs` alongside `inline_writability_pre_check`.

### Step 3.2: Unit tests for plan shape
- [x] Pin the **direct** bare plan (all-None timeouts, empty `pre_checks`/`post_checks`/`handlers`, `programmatic_handler: None`, no retry field) — matches the spec's Effective Plan Construction table.
- [x] Pin the **inline** bare plan (exactly one `HasWritePermission` pre-check, id `u32::MAX`).
- [x] Pin the **parsed-harness inline** shape: authored pre-checks present, with the system writability rule first and authored order preserved.

### Step 3.3: Route both insertion sites through the helper
- [x] Replace the loop's inline insertion (`harness_orch.rs:823-828`) with a call to `finalize_effective_plan` on the just-parsed plan.
- [x] Replace the preflight's inline insertion (`mod.rs:1761-1766`) with the same helper so the validation-only preflight and the executing loop agree byte-for-byte. The preflight still **drops** its plan (validation only); the loop remains the sole executor, so no double *evaluation* occurs.

**Observable**: Behavior identical; Phase 1 tests still green; new shape tests pass.

**Validation checkpoint**: `cargo test -p claudine` (lib shape tests) and `cargo test -p claudine-cli` pass; `cargo check -p claudine -p claudine-cli` clean.

---

## Phase 4: Route All Composition Through `run_harness_loop`

**Goal**: Replace the `else` (non-harness) branch with a `run_harness_loop` call so every non-dry-run compose/inline-compose — including interactive — runs the unified path. Old path code still present (deleted in Phase 5) but no longer reachable.

**Depends on**: Phases 2 and 3.

**Parallelizable**: No (single function rewrite).

### Step 4.1: Unify preflight — ✅ complete
- [x] Make the harness preflight (plan parse + `finalize_effective_plan` + `resolve_shell_approvals`) run for **both** `harness_enabled` and bare documents. For bare docs `parse_harness_plan` returns an empty plan and shell approval is a no-op.
- [x] Remove the separate non-harness inline permission branch (`mod.rs:1782-1798`): inline writability is now represented by the effective plan and enforced inside the loop. (Keep the `WrapperHarnessPermissionProbe` wiring the loop uses.)

### Step 4.2: Build `HarnessPromptState` for the formerly-non-harness case — ✅ complete
- [x] In place of the `else` branch body (`mod.rs:2043-2160`), build `HarnessPromptState` for direct and inline exactly as the harness branch does (`mod.rs:1971-1980`), set `HarnessPromptMode::Inline`/`Compose`, and call `run_harness_loop` with the same arguments the harness branch passes.
- [x] Pass through everything the harness branch receives **plus** the state threaded into compose since the spec was drafted: `effective_non_interactive`, `shell_working_directory`, `bind_agent_workspace`, and the `MODEL` / `YOLO` env exposure (verified present in `env_plan.env` / `child_cwd`).
- [x] Defuse the outer `LifecycleRunGuard` before the call (the loop owns lifecycle), matching `mod.rs:1988`. Drop the `guard.emit_start_once()` that the old non-harness branch used.
- [x] Added an interactive child-spawn path in `execute_harness_attempt` for interactive Codex inline-compose, reading `structured_codex_output.last_message_path` after `run_child`.

### Step 4.3: Preserve result surfaces — ✅ complete
- [x] Build `SingleCompositionOutcome` from the loop's `(exit_code, perf, signals)` for both direct and inline, wiring `iteration_signals` from Phase 2 (direct compose) and perf collection identically to the current harness branch.
- [x] Confirm provider-failure exit codes return `Ok((exit_code, ...))` (no `eyre` conversion) per the spec's Provider Failure Exit Codes contract — already the loop's behavior.

**Observable**: Phase 1 convergence + loop tests pass with **both** branches now calling `run_harness_loop`. `interactive: true` inline/direct compose runs through the loop. Dry-run still returns before the loop and does not mutate source files.

**Validation checkpoint**: `cargo check -p claudine -p claudine-cli` clean; `just test` in `claudine/` passes; `just lint` in `claudine/` passes.

---

## Phase 5: Remove Dead Code

**Goal**: Delete the now-unreachable non-harness path; keep still-referenced helpers.

**Depends on**: Phase 4 green.

**Parallelizable**: Deletions are independent but compile-checked together.

### Step 5.1: Delete unreachable functions/types — ✅ complete
- [x] `execute_without_harness` and the `CompositionExecutionMode` enum (`composition/mod.rs`).
- [x] `composition/structured.rs::run_structured_branch` (its structured logic is the loop's `harness_orch.rs:493` branch).
- [x] `composition/legacy_goose.rs` module (its captured logic is the loop's `harness_orch.rs:633` branch) — removed the `mod legacy_goose;` declaration.
- [x] `inline_guards::apply_inline_closure` and its private `report_interruption` (closure flows through `wrap/inline.rs::try_inline_closure`; interruption reporting is covered by `report_inline_agent_status` at `harness_orch.rs:684`).
- [x] `CompositionStreamResult` and the orphaned `emit_stream_summary_with_context` helper (`policy.rs`).

### Step 5.2: Keep what's still used — ✅ complete
- [x] **Did not** delete `inline_guards::cleanup_inline_output` or `split_frontmatter_and_body` — reused by `inline.rs:185` and re-exported at `mod.rs:58`. The `inline_guards` module stays; only `apply_inline_closure`/`report_interruption` left.
- [x] Removed now-unused imports, `CompositionStreamResult` fields, and helper fns the deletions orphaned (`cargo clippy -D warnings` guided).

**Observable**: `rg -n "execute_without_harness|CompositionExecutionMode|run_structured_branch|legacy_goose|apply_inline_closure|non-harness path|without harness" claudine/cli/src claudine/docs .claude/skills/claudine` returns no live-code references.

**Validation checkpoint**: `cargo check -p claudine -p claudine-cli --color=never` clean with no dead-code warnings; full test suite green.

---

## Phase 6: Documentation Drift

**Goal**: Remove descriptions of separate harness/non-harness paths.

**Depends on**: Phase 5.

**Parallelizable**: All docs independent.

### Step 6.1: Update topic docs — ✅ complete
- [x] `claudine/docs/topics/composition.md`: reviewed; no two-path language to update.
- [x] `claudine/docs/topics/non-interactive-sessions.md`: replaced `execute_without_harness`/`CompositionExecutionMode` description with unified `run_harness_loop` description.
- [x] `claudine/docs/topics/execution-flow.md`: updated preflight and execution sections to describe single `run_harness_loop` path; updated inline-compose execution step.
- [x] `claudine/docs/pipeline.md`: renamed E1 section, removed no-harness row, updated D6.inline and E2 notes, updated the "No harness frontmatter" quick-scan bullet, and replaced the deleted `emit_minimal_composition_summary` F2.3 row with `emit_stream_summary`.
- [x] `claudine/docs/topics/provider-metadata.md`: removed references to deleted `structured.rs` and `legacy_goose.rs`; moved Codex last-message extraction note into `harness_orch.rs`.
- [x] `claudine/README.md`, `claudine/lib/README.md`, `claudine/cli/README.md`: replaced outdated `execute_without_harness`/`CompositionExecutionMode` descriptions with unified `run_harness_loop` description.

### Step 6.2: Update the Claudine skill — ✅ complete
- [x] `.claude/skills/claudine/SKILL.md`: verified no two-path execution description; updated `last_updated` to 2026-06-16; hash regenerated (unchanged).
- [x] `.claude/skills/claudine/timeline.md`: added `always-harness` entry and noted the 2026-04-16 entry as superseded.

**Validation checkpoint**: the acceptance `rg` over `claudine/cli/src claudine/docs .claude/skills/claudine` shows only historical/intentional references (`closure::apply_inline_closure` lib function, timeline history, descriptive comments).

---

## Phase 7: Final Verification

**Goal**: Confirm the full acceptance list.

**Depends on**: Phases 1-6.

### Step 7.1: Acceptance sweep (from spec) — ✅ complete
- [x] Bare + parsed-harness direct compose both execute through `run_harness_loop`; same for inline.
- [x] Interactive compose executes through `run_harness_loop`; interactive inline closure still rewrites the body.
- [x] Inline body replacement uses final response only in both bare and parsed-harness runs.
- [x] Provider non-zero exit preserved at the CLI boundary.
- [x] Dry-run does not launch the provider and does not mutate inline source files.
- [x] Lifecycle notifications fire once per run.
- [x] Timeout precedence: CLI > frontmatter > env > built-in (parsed); CLI > env > built-in (bare).
- [x] `compose --loop` still receives rate-limit + exit-reason signals from bare and parsed-harness direct composition.
- [x] Inline composition evaluates exactly one system-owned writability pre-check per attempt.

### Step 7.2: Commands — ✅ complete
- [x] `cargo check -p claudine -p claudine-cli --color=never` clean.
- [x] `just test` in `claudine/` passes (claudine lib 2574 passed / 3 skipped; claudine-contract 39 passed / 5 skipped; claudine-cli 1602 passed / 67 skipped).
- [x] `just lint` in `claudine/` passes.

**Validation checkpoint**: all acceptance criteria satisfied; spec `status` moved to `implemented`.

---

## Risk Register

| Risk | Mitigation |
|------|------------|
| Bug in the unified path affects every composition run | Convergence + loop tests land first (Phase 1); flip happens after the harness branch is already signal-correct (Phase 2). |
| `compose --loop` loses rate-limit / exit-reason signals | Phase 2 surfaces terminal-attempt signals before the flip; Phase 1 test guards it. |
| Double inline-writability evaluation | Phase 3 centralizes injection; loop remains sole executor, preflight drops its plan. |
| Interactive regressions | Interactive gates already shared in the prelude; Phase 4 only re-points routing; Phase 1.3 / 7.1 verify closure. |
| Over-deletion of `inline_guards` | Phase 5.2 explicitly preserves `cleanup_inline_output` / `split_frontmatter_and_body`. |
| Hard-to-reverse cleanup | Delete (Phase 5) only after the unified route compiles and tests pass, keeping migration and cleanup in separate commits. |
