---
title: Sentrux Quality Remediation Plan — claudine package area
agent: claude
phases: 6
start_phase: 1
created: '2026-05-04T15:47:23'
source_review: review-1.md
quality_signal_baseline: 0.5372
suggestions_total: 29
suggestions_critical: 6
suggestions_urgent: 11
source_files_during_phase_1:
  - claudine/lib/src/harness/failure.rs
  - claudine/lib/src/harness/mod.rs
  - claudine/lib/src/harness/error.rs
  - claudine/lib/src/harness/model.rs
  - claudine/lib/src/lib.rs
  - claudine/lib/src/provider_id.rs
  - claudine/lib/src/provider/mod.rs
  - claudine/lib/src/provider/identity.rs
  - claudine/lib/src/provider/output_format.rs
  - claudine/lib/src/provider/registry.rs
  - claudine/lib/src/provider/claude.rs
  - claudine/lib/src/provider/codex.rs
  - claudine/lib/src/provider/gemini.rs
  - claudine/lib/src/provider/goose.rs
  - claudine/lib/src/provider/kimi.rs
  - claudine/lib/src/provider/opencode.rs
  - claudine/lib/src/provider/qwen.rs
  - claudine/lib/src/provider/roo.rs
  - claudine/lib/src/stream/mod.rs
  - claudine/lib/src/stream/badges.rs
  - claudine/lib/src/stream/claude_semantic.rs
  - claudine/lib/src/stream/codex_semantic.rs
  - claudine/lib/src/stream/gemini_semantic.rs
  - claudine/lib/src/stream/kimi_semantic.rs
  - claudine/lib/src/stream/opencode_semantic.rs
  - claudine/lib/src/stream/qwen_semantic.rs
  - claudine/lib/src/stream/reporting.rs
  - claudine/lib/src/stream/semantic.rs
  - claudine/lib/src/stream/stderr.rs
  - claudine/lib/src/stream/summary.rs
  - claudine/lib/src/stream/logs/opencode.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - claudine/lib/src/dispatch/runner.rs
  - claudine/lib/src/dispatch/runner/mod.rs
  - claudine/lib/src/dispatch/runner/speak.rs
  - claudine/lib/src/dispatch/runner/bash.rs
  - claudine/lib/src/dispatch/runner/report.rs
  - claudine/lib/src/dispatch/runner/mappers.rs
  - claudine/lib/src/dispatch/runner/protect.rs
  - claudine/lib/src/dispatch/runner/meta_json.rs
  - claudine/lib/src/dispatch/runner/decisions.rs
  - claudine/lib/src/dispatch/loader.rs
  - claudine/lib/src/dispatch/deps.rs
  - claudine/lib/src/dispatch/mod.rs
  - claudine/lib/src/harness/parse.rs
  - claudine/lib/src/harness/parse/mod.rs
  - claudine/lib/src/harness/parse/validations.rs
  - claudine/lib/src/harness/parse/handlers.rs
  - claudine/lib/src/harness/parse/overlays.rs
  - claudine/lib/src/harness/parse/frontmatter.rs
  - claudine/lib/src/harness/parse/shapes.rs
  - claudine/lib/src/harness/parse/span.rs
  - claudine/lib/src/harness/validate.rs
  - claudine/lib/src/harness/validate/mod.rs
  - claudine/lib/src/harness/validate/fs.rs
  - claudine/lib/src/harness/validate/git.rs
  - claudine/lib/src/harness/validate/compare.rs
  - claudine/lib/src/harness/validate/render.rs
  - claudine/lib/src/config/claudine_config.rs
  - claudine/lib/src/config/mod.rs
  - claudine/lib/src/config/tts.rs
  - claudine/lib/src/config/messaging_block.rs
  - claudine/lib/src/config/merge.rs
  - claudine/lib/src/stream/mod.rs
  - claudine/lib/src/stream/claude_semantic.rs
  - claudine/lib/src/stream/codex_semantic.rs
  - claudine/lib/src/stream/gemini_semantic.rs
  - claudine/lib/src/stream/kimi_semantic.rs
  - claudine/lib/src/stream/opencode_semantic.rs
  - claudine/lib/src/stream/qwen_semantic.rs
  - claudine/lib/src/stream/providers/mod.rs
  - claudine/lib/src/stream/providers/claude.rs
  - claudine/lib/src/stream/providers/codex.rs
  - claudine/lib/src/stream/providers/gemini.rs
  - claudine/lib/src/stream/providers/kimi.rs
  - claudine/lib/src/stream/providers/opencode.rs
  - claudine/lib/src/stream/providers/qwen.rs
  - claudine/lib/src/stream/logs/opencode.rs
  - claudine/lib/src/stream/logs/opencode/mod.rs
  - claudine/lib/src/stream/logs/opencode/events.rs
  - claudine/lib/src/stream/logs/opencode/errors.rs
  - claudine/lib/src/stream/logs/opencode/reasoning.rs
  - claudine/lib/src/linking/skills.rs
  - claudine/lib/src/linking/skills/mod.rs
  - claudine/lib/src/linking/skills/portable.rs
  - claudine/lib/src/linking/skills/partial.rs
  - claudine/lib/src/linking/skills/native.rs
  - claudine/lib/src/linking/compatibility.rs
  - claudine/lib/src/linking/compatibility/mod.rs
  - claudine/lib/src/linking/compatibility/table.rs
  - claudine/lib/src/reporting/queries.rs
  - claudine/lib/src/reporting/queries/mod.rs
  - claudine/lib/src/reporting/queries/common.rs
  - claudine/lib/src/reporting/queries/today.rs
  - claudine/lib/src/reporting/queries/week.rs
  - claudine/lib/src/reporting/queries/month.rs
  - claudine/lib/src/reporting/queries/sessions.rs
  - claudine/lib/src/reporting/queries/tools.rs
  - claudine/lib/src/reporting/queries/errors.rs
  - claudine/lib/src/reporting/queries/repos.rs
  - claudine/lib/src/reporting/queries/trends.rs
  - claudine/lib/src/reporting/queries/sync.rs
  - claudine/lib/src/services/mod.rs
  - claudine/lib/src/services/protect/catalog.rs
  - claudine/lib/src/services/protect/config.rs
  - claudine/lib/src/services/protect/decision.rs
  - claudine/lib/src/services/protect/matcher.rs
  - claudine/lib/src/services/protect/mod.rs
  - claudine/lib/src/services/protect/observe.rs
  - claudine/lib/src/services/protect/path.rs
  - claudine/lib/src/services/protect/report.rs
  - claudine/lib/src/services/protect/service.rs
  - claudine/lib/src/protect/mod.rs
  - claudine/lib/src/protect/catalog.rs
  - claudine/lib/src/protect/config.rs
  - claudine/lib/src/protect/decision.rs
  - claudine/lib/src/protect/matcher.rs
  - claudine/lib/src/protect/observe.rs
  - claudine/lib/src/protect/path.rs
  - claudine/lib/src/protect/report.rs
  - claudine/lib/src/protect/service.rs
  - claudine/lib/src/lib.rs
  - claudine/lib/src/provider/mod.rs
  - claudine/lib/src/provider/behavior.rs
  - claudine/lib/src/provider/claude.rs
  - claudine/lib/src/provider/codex.rs
  - claudine/lib/src/provider/gemini.rs
  - claudine/lib/src/provider/kimi.rs
  - claudine/lib/src/provider/opencode.rs
  - claudine/lib/src/provider/qwen.rs
  - claudine/lib/src/provider/tests.rs
  - claudine/cli/src/commands/wrap/subagent_watchdog.rs
  - claudine/cli/src/commands/wrap/wire_io.rs
  - claudine/cli/src/commands/config_tui/mod.rs
  - claudine/cli/src/commands/config_tui/app.rs
  - claudine/cli/src/commands/config_tui/tabs/services.rs
  - claudine/cli/src/commands/init/mod.rs
  - claudine/cli/src/commands/init_wizard.rs
  - claudine/lib/benches/runtime_hot_paths.rs
  - claudine/lib/src/composition/mod.rs
docs_updated_during_phase_2:
  - claudine/docs/topics/composition.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - claudine/cli/src/commands/wrap/profile/mod.rs
  - claudine/cli/src/commands/wrap/profile/claude.rs
  - claudine/cli/src/commands/wrap/profile/codex.rs
  - claudine/cli/src/commands/wrap/profile/gemini.rs
  - claudine/cli/src/commands/wrap/profile/goose.rs
  - claudine/cli/src/commands/wrap/profile/kimi.rs
  - claudine/cli/src/commands/wrap/profile/opencode.rs
  - claudine/cli/src/commands/wrap/profile/qwen.rs
  - claudine/cli/src/commands/wrap/live_semantic_sink/mod.rs
  - claudine/cli/src/commands/wrap/live_semantic_sink/sections.rs
  - claudine/cli/src/commands/wrap/live_semantic_sink/spacing.rs
  - claudine/cli/src/commands/wrap/live_semantic_sink/tool_calls.rs
  - claudine/cli/src/commands/wrap/live_semantic_sink/thinking.rs
  - claudine/cli/src/commands/wrap/live_semantic_sink/errors.rs
  - claudine/cli/src/commands/wrap/live_semantic_sink/heartbeat.rs
  - claudine/cli/src/commands/wrap/exec/mod.rs
  - claudine/cli/src/commands/wrap/exec/spawn.rs
  - claudine/cli/src/commands/wrap/exec/watchdog.rs
  - claudine/cli/src/commands/wrap/exec/termination.rs
  - claudine/cli/src/commands/wrap/exec/timeouts.rs
  - claudine/cli/src/commands/wrap/exec/exit.rs
  - claudine/cli/src/commands/wrap/exec/wiring.rs
  - claudine/cli/src/commands/wrap/exec/subagent_watchdog.rs
  - claudine/cli/src/commands/wrap/mod.rs
  - claudine/cli/src/commands/wrap/flags.rs
  - claudine/cli/src/commands/wrap/prompt_source.rs
  - claudine/cli/src/commands/wrap/overlay.rs
  - claudine/cli/src/commands/wrap/resume.rs
  - claudine/cli/src/commands/wrap/harness_orch.rs
  - claudine/cli/src/commands/wrap/inline.rs
  - claudine/cli/src/commands/wrap/policy.rs
  - claudine/cli/src/commands/wrap/wire_io.rs
  - claudine/cli/src/commands/wrap/subagent_watchdog.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
packages:
  - claudine
  - claudine-cli
---

# Sentrux Quality Remediation Plan — claudine

This plan operationalizes the 29 suggestions in [`review-1.md`](./review-1.md). The
work is overwhelmingly **structural refactoring** — file splits, cycle breaks,
namespace re-shaping — with **no public-API change** intended. The plan
sequences high-leverage / blocking work first (cycle breaks → lib splits → CLI
splits) and ends with a Sentrux re-scan to verify and surface the next round.

## Scope and goals

**In scope:** the `claudine/lib/` and `claudine/cli/` crates only. All files
verified on disk per review-1 § "Method note". The plan touches **no other
workspace member**.

**Goals (acceptance criteria for the entire plan):**

1. `cycle_count` drops from **2 → 0** (Phase 1).
2. `god_file_count` drops from **1 → 0** and every file ≥ 2,000 lines is
   either split or has a tracked exception (Phases 2–4).
3. `cross_module_edges` ratio drops below **40%** (Phases 2 & 5).
4. `complex_fn_count` (`133`) drops by ≥ 30% via reduced-context simplifications
   (Phase 6 follow-ups).
5. `cargo build`, `cargo test`, `cargo clippy --all-targets -- -D warnings`,
   and `just lint` for both `claudine/lib` and `claudine/cli` pass at every
   phase boundary.
6. No behavioral change in `claudine` CLI output, hook dispatch, composition
   pipelines, watchdog, MCP injection, or messenger redaction (verified by
   the existing inline test suites moving with their subject modules).

## Common per-task checklist

Every task in this plan is a structural refactor and follows the same
discipline. Apply this checklist on every sub-task:

1. Create the new module/directory layout *first* (empty `mod.rs` /
   re-export skeleton).
2. Move code blocks **without modification** into the new files.
   Inline `#[cfg(test)] mod tests` blocks travel with their subject module.
3. Update `mod`/`pub mod` declarations in the parent.
4. Narrow `use` statements at the top of each new file to only what that
   file needs (this is the single biggest cross-module-edge reduction lever).
5. Update **internal** importers; preserve `pub use` at module boundaries
   so the **public API is unchanged**.
6. Run, in order: `cargo build -p <crate>`, `cargo test -p <crate>`,
   `cargo clippy -p <crate> --all-targets -- -D warnings`,
   then the area `justfile` recipes (`just test|lint|build`).
7. Confirm the inline test count is preserved (the review already cites
   per-file test counts — they are the regression baseline).
8. **Do not** rename public types, change visibility upward (`pub(crate)` →
   `pub`), or alter behavior. If a refactor *requires* a behavioral change,
   stop and surface it — that is a separate task.

## Risk register

| Risk | Mitigation |
|---|---|
| Merge conflicts between parallel splits in the same `wrap/` directory | Serialize Phase 3 sub-waves; commit after each split lands |
| Hidden public-API leakage when narrowing `pub use` chains | Audit `pub use` lines before/after with `cargo public-api` or a manual grep; require Phase 6 re-scan to confirm |
| Behavioral regression masked by tests moving with code | Run `cargo test --workspace` (not just `-p`) at end of each phase |
| Compilation cascade from `Provider` enum move (Phase 1.2) | Add temporary `pub use` shim, ship the shim removal as the last sub-task of Phase 1 |
| `output.rs` vs `output/` collision (Phase 5.5) | Convert via `git mv`; Rust forbids the dual form once `output/mod.rs` exists |

---

## Phase 1 — Break Import Cycles (foundational)

**Why first.** Both `critical`-tier cycles in review-1 must be resolved before
later splits land — Phase 2.5 (stream parser grouping) directly depends on the
`Provider` enum being relocated out of `provider/mod.rs`. After Phase 1,
`cycle_count` should already read `0`.

**Parallelizable.** 1.1 and 1.2 touch disjoint files and may run concurrently.

### 1.1 — Break `harness::error` ⇄ `harness::model` cycle

**Files:**
- `claudine/lib/src/harness/error.rs` (importer of `ValidationFailure`)
- `claudine/lib/src/harness/model.rs` (importer of `HarnessError`)
- `claudine/lib/src/harness/mod.rs` (mod declarations)

**Steps:**
1. Create `claudine/lib/src/harness/failure.rs` (new leaf module).
2. Move the `ValidationFailure` struct (and any directly-coupled types it
   carries) from `model.rs` into `failure.rs`. The move must be byte-equivalent
   except for `use` statements at the top.
3. Add `pub mod failure;` to `harness/mod.rs` **before** `pub mod error;` and
   `pub mod model;` (declaration order doesn't matter to rustc but reads
   bottom-up the layering).
4. Both `error.rs` and `model.rs` switch to
   `use crate::harness::failure::ValidationFailure;`.
5. Re-export from `harness::error::ValidationFailure` and
   `harness::model::ValidationFailure` for one compatibility cycle if any
   external callers of the lib crate consume those paths (verify with grep
   across `claudine/cli/`).

**Verification:** `cargo build -p claudine` then `cargo test -p claudine
harness::`.

### 1.2 — Break `provider` ⇄ `stream` cycle

**Files:**
- `claudine/lib/src/provider/{claude,codex,gemini,kimi,opencode,qwen,behavior,mod}.rs`
- `claudine/lib/src/stream/{mod,summary,semantic,badges,claude_semantic,codex_semantic,gemini_semantic,kimi_semantic,opencode_semantic,qwen_semantic}.rs`

**Steps:**
1. Create `claudine/lib/src/provider_id.rs` (a flat leaf module — **not**
   `provider/id.rs`, which would re-trigger the cycle).
2. Move the `Provider` enum, `provider_info()`, `PROVIDERS_DISPLAY_ORDER`, and
   `OutputFormatSelector` from `provider/mod.rs` into `provider_id.rs`.
3. Add `pub mod provider_id;` to `claudine/lib/src/lib.rs`.
4. Add a temporary shim `pub use crate::provider_id::Provider;` in both
   `provider/mod.rs` and `stream/mod.rs` so existing import paths keep working.
5. Update every file in `stream/` to switch from
   `use crate::provider::Provider;` to `use crate::provider_id::Provider;`.
   The `provider/` files keep their `use crate::stream::*` imports.
6. Verify cycle is gone: `cargo build -p claudine` succeeds with no rustc cycle
   warning.
7. **Last sub-task:** delete the shim re-exports added in step 4 once all
   internal call sites have been migrated.

**Verification:** `cargo test -p claudine` and a manual grep for
`crate::provider::Provider` returning zero hits inside `claudine/lib/src/stream/`.

---

## Phase 2 — Library structural splits

All Phase 2 tasks are independent file-level splits within `claudine/lib/`.
They can run in **parallel sub-waves**, but a single phase boundary keeps the
verification gate consistent. 2.5 has a soft dependency on 1.2 having landed
(or its compatibility shim still being in place).

### 2.1 — Split `dispatch::runner` (2,326 lines, 84 fns)

**Target layout:** `claudine/lib/src/dispatch/runner/{mod,speak,bash,report,
mappers,protect,meta_json,decisions}.rs` per review-1.

Each new file inherits its 49 inline tests from the parent. `mod.rs` keeps
only the orchestration entry point and `pub use` re-exports for the public
surface (the dispatch loop callers).

### 2.2 — Split `harness::parse` (2,317 lines, 89 fns)

**Target layout:** `claudine/lib/src/harness/parse/{mod,validations,handlers,
overlays,frontmatter,shapes}.rs` per review-1. Helpers become
`pub(super) fn`. The 56 inline tests move with their subject.

### 2.3 — Split `harness::validate` (1,848 lines)

**Target layout:** `claudine/lib/src/harness/validate/{mod,<one file per
validation kind>,render}.rs`. The Darkmatter-backed render helper lives in
`render.rs` so it is independently unit-testable (per the area skill's
2026-04-29 expression-bridge note).

### 2.4 — Split `config::claudine_config` (1,955 lines)

**Target layout:** `claudine/lib/src/config/{claudine_config,tts,
messaging_block,merge}.rs`. The merge logic is large enough to deserve its
own file. Importers (13 cited in review-1) update to narrower paths.

### 2.5 — Group stream parsers under `stream::providers::`

**Depends on:** Phase 1.2 complete (or its shim still in place).

**Files moved:**
`stream/{claude,codex,gemini,kimi,opencode,qwen}_semantic.rs` →
`stream/providers/{claude,codex,gemini,kimi,opencode,qwen}.rs`.

**Trait extraction:** introduce `trait SemanticParser` and a
`SemanticParser::for_provider(p: Provider) -> Box<dyn SemanticParser>`
factory. Concrete provider files in `claudine/lib/src/provider/` then import
the **trait** instead of six per-provider parser types. This is the single
biggest cross-module-edge reduction in Phase 2.

### 2.6 — Split `stream/logs/opencode` (2,066 lines)

**Target layout:** `stream/logs/opencode/{mod,events,reasoning,errors}.rs`.
The typed `Reasoning` variant routing (per area skill 2026-04-16) lands in
`reasoning.rs`; `SemanticErrorKind` classification lives in `errors.rs`.

### 2.7 — Split `linking::skills` (1,543) and `linking::compatibility` (1,202)

**Target layout:**
- `linking/skills/{mod,portable,partial,native}.rs` (per portability class).
- `linking/compatibility/{mod,table}.rs` — the 8-provider × N-feature matrix
  becomes a `const` table in `table.rs`; lookup/diff stays in `compatibility.rs`.

### 2.8 — Split `reporting::queries` (1,581 lines)

**Target layout:** `reporting/queries/{mod,today,week,month,sessions,tools,
errors,repos,trends,sync,common}.rs`. Each file holds the SQL fragments and
result mapping for its `claudine logs` subcommand. **Pairs with Phase 5.1**
(`commands::logs` split) — same file-name pattern on the CLI side.

### 2.9 — Flatten `services::protect::*` to top-level `protect/`

`services/` contains nothing else (the area skill confirms `ProtectService` is
its only inhabitant). Promote `claudine/lib/src/services/protect/*` →
`claudine/lib/src/protect/*`, delete the `services/` namespace, and update the
12 cited importers across `dispatch`, `adapters`, `events`, `config`,
`composition`, `harness`. This single change drops one path segment from every
protect-related edge — directly addressing the `max_depth: 8` baseline.

### 2.10 — Introduce `dispatch::deps` façade for the loader

`dispatch/loader.rs` (1,526 lines) imports from 13 distinct `crate::*` paths.
Create `claudine/lib/src/dispatch/deps.rs` re-exporting only the narrow surface
the loader actually uses, and switch the loader to a single
`use crate::dispatch::deps::*;` line. Verify the loader's inbound edge count
drops via a re-scan in Phase 6.

### 2.11 — Add `MessengerRoute` trait + per-route files

**Files:** `claudine/lib/src/messaging/send.rs` (1,185 lines) and
`claudine/lib/src/messaging/mod.rs`.

**Steps:**
1. Define `pub trait MessengerRoute { fn send(&self, payload: &Payload) ->
   Result<SendReceipt> }`.
2. Move each provider implementation into
   `messaging/routes/{discord_bot,discord_webhook,slack_bot,slack_webhook,
   signal,whatsapp}.rs`.
3. Make `redact_webhook_urls` a single shared decorator (it is currently
   re-implemented per route — see area skill's "Messenger Webhook Redaction
   Invariants").
4. Desktop notifications (`execute_notification`) stay separate per the area
   skill — they are zero-config and not a `MessengerRoute`.

**Pairs with Phase 4.3** (CLI `tabs::messenger` split).

### 2.12 — Audit `composition/mod.rs` re-exports

**Goal:** confirm no `pub use` chain from `composition::mod` re-exports
something declared `mod` (private) — that defeats modularity. Where a private
type genuinely needs external reach, hoist it into `composition::api` rather
than widening the parent's privacy.

**Output:** a one-paragraph note in `claudine/docs/topics/composition.md`
documenting the public surface (or a confirmation that no change was needed).

---

## Phase 3 — CLI critical god-file splits

These are the **four `critical`-tier CLI items**. Together they are the single
"god file" the baseline counts (and three peers that nearly qualify). Each
target file is materially independent in concern; the **directory** is shared
(`cli/src/commands/wrap/`), so be careful with merge ordering.

**Sub-wave A (parallelizable, fully independent):**
- 3.1 `commands::wrap::profile`
- 3.2 `commands::wrap::live_semantic_sink`
- 3.3 `commands::wrap::exec` (absorbs `wire_io` and `subagent_watchdog` —
  see Phase 4 § 4.x for the freed names)

**Sub-wave B (after A, because 3.4 orchestrates the others):**
- 3.4 `commands::wrap::mod`

### 3.1 — Split `commands::wrap::profile` (3,347 lines, 105 tests)

Per review-1: one file per provider under
`cli/src/commands/wrap/profile/{mod,claude,codex,gemini,goose,kimi,opencode,
qwen}.rs`. `mod.rs` keeps the trait `WrapperProfile` and the dispatcher.
Each provider's tests follow it.

### 3.2 — Split `commands::wrap::live_semantic_sink` (4,269 lines, 124 fns, 78 tests)

**Target layout:** `cli/src/commands/wrap/live_semantic_sink/{mod,sections,
spacing,tool_calls,thinking,errors,heartbeat}.rs`.

The 9-section model emitters land in `sections.rs`; the spacing state machine
(strictly enforced per area skill 2026-04-14) is its own file in `spacing.rs`
because the spacing rules are shared across all section emitters. The
`SemanticErrorKind` colored BlockQuote rendering (per area skill 2026-04-16)
lives in `errors.rs`. The 30-second flush_if_idle / heartbeat thread (per
2026-04-16 fix) lives in `heartbeat.rs`.

### 3.3 — Split `commands::wrap::exec` and absorb `wire_io` + `subagent_watchdog`

**Files merged into the new directory:**
- `cli/src/commands/wrap/exec.rs` (3,091 lines, 79 fns, 38 tests) — splits
- `cli/src/commands/wrap/wire_io.rs` (1,611 lines) — becomes `exec/wiring.rs`
- `cli/src/commands/wrap/subagent_watchdog.rs` (1,204 lines) — becomes
  `exec/subagent_watchdog.rs`

**Target layout:** `cli/src/commands/wrap/exec/{mod,spawn,watchdog,termination,
timeouts,exit,wiring,subagent_watchdog}.rs`.

The unified-watchdog change from area skill 2026-05-03 (the four env vars
`CLAUDINE_TIMEOUT`, `CLAUDINE_STEP_TIMEOUT`, `CLAUDINE_KILL_GRACE`,
`CLAUDINE_WATCHDOG_INTERVAL`) all stay in `watchdog.rs` + `timeouts.rs`. The
`WatchdogTermination` channel and SIGTERM→SIGKILL grace logic are in
`termination.rs`. **Do not** alter the precedence chain (CLI > frontmatter >
env > built-in default) — that is a behavior contract.

This sub-task subsumes review-1's `urgent` § "Split `commands::wrap::wire_io`".

### 3.4 — Split `commands::wrap::mod` (4,641 lines, 95 fns, 44 tests)

**Schedule after 3.1–3.3.** This file orchestrates the others; doing it last
minimizes merge conflict surface.

**Target layout:** `cli/src/commands/wrap/{mod,flags,prompt_source,overlay,
resume,harness_orch,inline,policy}.rs` per review-1.

`mod.rs` retains `pub use` re-exports + `run_provider_wrapper` only. The 44
inline tests move to whichever sibling file owns the function under test.

---

## Phase 4 — CLI urgent splits

Four further CLI splits. All independent and parallelizable.

### 4.1 — Split `commands::wrap::composition` (2,452 lines)

**Target layout:** `cli/src/commands/wrap/composition/{mod,structured,summary,
inline_guards,legacy_goose}.rs` per review-1.

`CompositionExecutionMode::{Direct, Inline}` stays in `mod.rs`. The four
inline-only guarded behaviors documented in the area skill 2026-04-16 fix
(closure validation/file write, deferred summary timing, interrupted-session
partial body report, writability pre-check) all land in `inline_guards.rs`
with comments preserved verbatim.

### 4.2 — Split `cli/src/argv.rs` (1,605 lines)

**Target layout:** `cli/src/argv/{mod,rule1_provider_bool,rule2_canonicalize,
rule3_separator,rule4_help_hoist,flag_surface}.rs` per review-1.

**Critical invariant to preserve:** Rule 4 must run before Rule 3. Document
this as a comment block in `mod.rs` and keep the `composition_flags_with_value_
matches_clap_surface` drift-detection test in `flag_surface.rs`.

### 4.3 — Split `commands::config_tui::tabs::messenger` (1,770 lines)

**Pairs with Phase 2.11.**

**Target layout:** `cli/src/commands/config_tui/tabs/messenger/{mod,
masked_input,test_connection,redaction,routes/{discord_bot,slack_bot,signal,
whatsapp,discord_webhook,slack_webhook}.rs}` per review-1.

**Invariants to preserve verbatim** (called out in area skill):
- Inline webhook URLs render as `webhook: ********` — never raw.
- Secret input buffers display bullets/asterisks during modal entry.
- All error messages run through `redact_webhook_urls` before display.
- Test-connection failure status redacts URLs.

### 4.4 — Split `completion::composition` (1,489 lines)

**Target layout:** `cli/src/completion/composition/{mod,compose,inline_compose,
sequence,magic_at,setter_value}.rs` per review-1.

The performance strategy described in the area skill (root-menu rules,
per-mode pipelines) is the contract — the split is purely structural.

---

## Phase 5 — Important splits and small cleanup

### 5.1 — Split `commands::logs` (1,300 lines)

**Pairs with Phase 2.8** (`reporting::queries` split). One CLI file per
subcommand: `cli/src/commands/logs/{mod,today,week,month,sessions,tools,
errors,repos,trends,sync}.rs`. Each pairs 1:1 with its
`reporting/queries/<name>.rs` counterpart.

### 5.2 — Split `commands::hooks` (1,237 lines)

**Target layout:** `cli/src/commands/hooks/{mod,support,mapping,describe,
variables,list}.rs` (one file per display mode plus the per-provider listing).

### 5.3 — Split `commands::mcp` (1,209 lines)

**Target layout:** `cli/src/commands/mcp/{mod,list,init,show,default,alias,
remove,sync}.rs` (one file per subcommand). Cross-link to
[`claudine/docs/mcp-support.md`](../../docs/mcp-support.md) in each file's
header comment so future contributors land in the right doc.

### 5.4 — Split `commands::config_tui::tabs::actions` (1,474 lines)

Same recipe as Phase 4.3 — one file per action type under
`config_tui/tabs/actions/`.

### 5.5 — Resolve `output.rs` vs `output/` duplication

Convert `cli/src/output.rs` (1,173 lines) into `cli/src/output/mod.rs` and
split its contents into `output/{prose,tables,hyperlinks,…}.rs`. The two
existing children (`error_report.rs`, `error_walker.rs`) sit naturally as
siblings.

**Method note:** use `git mv` so history is preserved on the file rename.

---

## Phase 6 — Verification, complexity reduction, and documentation

### 6.1 — Re-run Sentrux baseline

Run `sentrux scan claudine/` and the `dsm` / `test_gaps` MCP tools (which
were unavailable for review-1 generation per its method note). Capture a new
`.sentrux/baseline.json` and diff against the 2026-05-04 snapshot.

**Required outcomes:**
- `cycle_count` == 0
- `god_file_count` == 0
- `cross_module_edges` ratio < 40%
- `quality_signal` > 0.65

### 6.2 — Address remaining complex functions

After Phases 1–5, re-examine `complex_fn_count` (currently 133). Most
functions in the split files will already drop below the configured
cyclomatic threshold simply because their surrounding context narrowed. For
the remaining `≥ 15`-complexity functions, apply targeted simplification
(extract helpers, replace nested matches with table dispatch, hoist guards).

**Target:** ≥ 30% reduction in `complex_fn_count`. This is the only Phase 6
sub-task that may require behavior-aware editing — every change must be
covered by an existing inline test.

### 6.3 — Confirm `composition/mod.rs` audit (Phase 2.12) is reflected in docs

Spot-check that `claudine/docs/topics/composition.md` is consistent with the
Phase 2.12 audit outcome.

### 6.4 — Update area skill and CLAUDE.md if architecture summary drifted

The `claudine/.claude/skills/claudine/SKILL.md` description mentions a
`services` namespace; after Phase 2.9 promotes `protect/` to top level, that
sentence needs an update. Mirror any other module-rename in:
- `claudine/.claude/skills/claudine/SKILL.md` (top-level module list)
- `claudine/CLAUDE.md` (only if the package layout summary references the
  renamed paths)
- `claudine/docs/topics/protect-service.md` (path references)

---

## Dependency graph

```
Phase 1.1 ──┐
            ├──► Phase 2 (parallel sub-waves) ──┐
Phase 1.2 ──┘                                    │
                                                 ├──► Phase 6
Phase 3 wave A (3.1, 3.2, 3.3) ──► Phase 3.4 ───┤
                                                 │
Phase 4 (parallel) ──────────────────────────────┤
                                                 │
Phase 5 (parallel; 5.1 pairs with 2.8) ─────────┘
```

Phase 1 is hard prerequisite for Phase 2.5. Phases 3, 4, 5 do not depend on
each other or on Phase 2 — they are separate crates' internal organization.
Phase 6 depends on every prior phase.

## Out of scope (intentional deferrals)

- **Reporting query filters in expression-bridge form** — area skill notes
  this is deferred follow-up because in-memory filtering can conflict with
  SQL aggregation efficiency. Not addressed by this plan.
- **Resource-linking filter expression rewrite** — deferred for the same
  reason; prefix/suffix/negation linking syntax is shared across
  skills/commands/agents and warrants its own design pass.
- **Adding ACP support** — explicit non-goal per area skill.
- **Renaming public types** — every refactor in this plan preserves the
  public API surface.
