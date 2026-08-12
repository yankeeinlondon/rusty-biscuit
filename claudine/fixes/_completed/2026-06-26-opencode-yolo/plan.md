---
agent: open_code/zai-coding-plan/glm-5.2
phases: 5
created: 2026-06-27
start_phase: 1
yolo: "true"
source_files_during_phase_1:
  - claudine/lib/src/opencode_config.rs
  - claudine/lib/src/lib.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - claudine/lib/src/mcp/inject.rs
  - claudine/cli/src/commands/wrap/wrapper_mcp.rs
  - claudine/cli/src/commands/wrap/env/mod.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - claudine/cli/src/commands/wrap/wrapper_stages.rs
  - claudine/cli/src/commands/wrap/mod.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
  - claudine/lib/src/permissions/providers/opencode.rs
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
source_files_during_phase_5:
  - claudine/cli/src/commands/wrap/wrapper_stages.rs
docs_updated_during_phase_5:
  - .claude/skills/claudine/cli-reference.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
  - .claude/skills/claudine/cli-reference.md
source_code:
  - claudine/lib/src/opencode_config.rs
  - claudine/lib/src/lib.rs
  - claudine/lib/src/mcp/inject.rs
  - claudine/lib/src/permissions/providers/opencode.rs
  - claudine/cli/src/commands/wrap/wrapper_mcp.rs
  - claudine/cli/src/commands/wrap/env/mod.rs
  - claudine/cli/src/commands/wrap/wrapper_stages.rs
  - claudine/cli/src/commands/wrap/mod.rs
documentation:
  - .claude/skills/claudine/cli-reference.md
packages:
  - claudine
---

# Execution Plan — OpenCode YOLO Subagent Permission Bypass

Source spec: [`spec.md`](./spec.md) — *OpenCode YOLO Does Not Bypass Permissions for Subagent Sessions*.

## Problem summary (one paragraph)

`--dangerously-skip-permissions` only auto-approves the **parent** OpenCode session.
Subagent (Task) sessions fall back to the `"ask"` default for `external_directory`
/ `doom_loop`, and with no TTY in a non-interactive `opencode run` they block
forever. The durable fix is a **config-level `permission` block** delivered through
`OPENCODE_CONFIG_CONTENT`, which OpenCode applies session-wide (parent **and**
children). That same env var is currently written by two independent producers
(MCP injector, system-prompt append) that **overwrite** each other — a latent
clobber that adding a third writer (YOLO) makes mandatory to fix.

## Architecture facts that drive this plan

Verified against current source:

- **Stage order** in `run_provider_wrapper_inner`
  (`claudine/cli/src/commands/wrap/mod.rs`):
  - Stage 6 (`mod.rs:352`) — `apply_yolo_for_mode`: pushes argv only today;
    `env_overrides` param is unused (`profile/opencode.rs:30`).
  - Stage 8 (`mod.rs:469`) — `resolve_and_apply_system_prompt` →
    `OpencodeWrapper::apply_system_prompt` pushes
    `("OPENCODE_CONFIG_CONTENT", {"instructions":[…]})` into `app.env`
    (`profile/opencode.rs:64-70`).
  - Stage 9 (`mod.rs:493`) — `build_child_env_with_launch` folds `env_overrides`
    into `env_plan.env: HashMap<OsString, OsString>` via plain last-write-wins
    `set_added_env` (`env/mod.rs:284-286`).
  - Stage 10 (`mod.rs:534`) — `compose_mcp_session` → `OpenCodeInjector::inject`
    writes `{"mcp":…}` into a fresh `string_env`, then
    `env_plan.env.insert(k, v)` (`wrapper_mcp.rs:167-175`, `mcp/inject.rs:101-103`)
    — **clobbering** the system-prompt value folded in Stage 9.
- **Net**: MCP (Stage 10) silently drops system-prompt `instructions` today.
  Adding YOLO as a third writer without a merge would drop whichever ran first.
- **Policy backend** is a *separate* code path:
  `claudine/lib/src/permissions/providers/opencode.rs` —
  `parse_cli_overrides` recognizes only `--yolo` (`:167`),
  `build_one_shot_plan` emits `--yolo` for `AutoApprove` (`:587`) and overwrites
  `OPENCODE_CONFIG_CONTENT` (`:600-604`).
- **Error enum** (`claudine/lib/src/error.rs`): `ClaudineError::ConfigValidation(String)`
  is the surgical home for the redacted "existing config is not a JSON object"
  failure (spec §acceptance #8).
- **Validation commands** (claudine area, per `claudine/justfile` + AGENTS.md):
  `just test-library`, `just test-cli`, `just lint`, `just doctest`, `just check`.
  Nextest is the runner (`just _test`).

## Design decision (decisive)

- **One shared library helper** owns all `OPENCODE_CONFIG_CONTENT` assembly:
  deep-merge + redacted parse + YOLO permission-block builder. Every runtime
  write site calls it; direct `env.insert("OPENCODE_CONFIG_CONTENT", …)` writes
  are the bug to remove (spec §"OPENCODE_CONFIG_CONTENT must be merged").
- **YOLO permission overlay is applied as a dedicated step AFTER MCP**
  (new Stage 10.5 in the orchestrator), gated on `yolo_enabled && non_interactive`,
  so it is the last Claudine overlay and cannot be weakened by MCP/system-prompt.
  `apply_yolo_for_mode` keeps its argv-only responsibility
  (`--dangerously-skip-permissions`) — the trait method is unchanged in signature.
- **Merge, never overwrite**, at the two runtime folds (Stage 9 env_overrides
  fold; Stage 10 MCP fold) and the new Stage 10.5, all via the helper. This also
  preserves a user-supplied `OPENCODE_CONFIG_CONTENT` (acceptance #8).

---

## Phase 1 — Shared OpenCode inline-config merge helper (library foundation)

**Goal**: land the single utility every other phase depends on, with full unit
coverage, before touching any producer. Nothing runtime-visible changes yet.

**Dependencies**: none. **Blocks**: Phases 2, 3, 4.

- [x] Create new library module `claudine/lib/src/opencode_config.rs` and register
      it as `pub mod opencode_config;` in `claudine/lib/src/lib.rs`. This is the
      shared home for `OPENCODE_CONFIG_CONTENT` assembly (consumed by lib MCP
      injector and CLI wrapper profile/orchestrator), named for the provider so
      it is not mistaken for an MCP-only utility.
- [x] Implement `pub fn deep_merge(base: &mut Value, overlay: Value)` — recursive
      object merge: objects merge key-by-key; arrays and scalars are replaced by
      the overlay (spec: "Arrays are replaced by the later overlay … needed for
      `instructions`").
- [x] Implement `pub fn yolo_permission_block() -> Value` returning the exact,
      path-free, cross-platform-stable object from spec §"Permission block shape":
      `{"permission":{"*":"allow","external_directory":"allow","doom_loop":"allow"}}`.
- [x] Implement the workhorse
      `pub fn merge_overlay(current: Option<&str>, overlay: Value) -> Result<String>`
      — parse `current` as a JSON object (`{}` when `None`); on parse failure or
      non-object, return `ClaudineError::ConfigValidation` with a **redacted**
      message that names `OPENCODE_CONFIG_CONTENT` but never echoes the raw value;
      deep-merge `overlay`; serialize once to a compact JSON string.
- [x] Add unit tests in the module's `#[cfg(test)]` block:
  - [x] `deep_merge` merges nested objects; later array replaces earlier.
  - [x] `yolo_permission_block()` shape is exactly the three keys under
        `permission` (serde round-trip equality).
  - [x] `merge_overlay(None, …)` starts from `{}`.
  - [x] `merge_overlay(Some(obj), …)` deep-merges and preserves unrelated keys.
  - [x] `merge_overlay` on malformed JSON returns an error whose `to_string()`
        contains `OPENCODE_CONFIG_CONTENT` and does **not** contain the raw value.
  - [x] `merge_overlay` on a non-object JSON (`"42"`, `[1,2]`) returns an error.
  - [x] `yolo_permission_block()` serializes identically regardless of map
        insertion order (cross-platform check, acceptance #7).

**Validation checkpoint**: `just test-library` (nextest) green for the new module;
`just check` clean. No other crate changed yet.

---

## Phase 2 — Fix the MCP ↔ system-prompt `OPENCODE_CONFIG_CONTENT` clobber

**Goal**: eliminate the latent two-writer clobber so `instructions` and `mcp`
coexist. This is required groundwork for acceptance #4 and is a correctness fix
in its own right.

**Dependencies**: Phase 1. **Parallelizable with**: Phase 4 (disjoint files).

- [x] MCP producer — in `claudine/lib/src/mcp/inject.rs`, stop writing
      `OPENCODE_CONFIG_CONTENT` directly. Build the `{"mcp": mcp_config}` overlay
      `Value`, then obtain the existing value from the passed-in `env` map and
      call `opencode_config::merge_overlay(existing, overlay)`; insert the
      returned string. (Note: the injector's `env: &mut HashMap<String,String>`
      is currently seeded empty by `compose_mcp_session`, so also handle the fold
      point below.)
- [x] MCP fold — in `claudine/cli/src/commands/wrap/wrapper_mcp.rs`
      (`compose_mcp_session`, the `for (k, v) in string_env` loop at `:173-175`),
      route the `OPENCODE_CONFIG_CONTENT` key through
      `opencode_config::merge_overlay` against the current `env_plan.env` value
      instead of a blind `env_plan.env.insert`. Other keys keep plain insert.
- [x] System-prompt fold — in
      `claudine/cli/src/commands/wrap/env/mod.rs::build_child_env_with_launch`
      (the `env_overrides` fold at `:284-286`), when the key is
      `OPENCODE_CONFIG_CONTENT`, merge via the helper against the current `env`
      map value (which may carry a user-supplied config from sanitize —
      acceptance #8); otherwise keep `set_added_env`.
- [x] Add/extend unit tests:
  - [x] `inject.rs` (`opencode_injects_env_var`): after injection, the env value
        parses to an object containing `mcp`; and when the injector is handed a
        map that already has an `OPENCODE_CONFIG_CONTENT` object, the merge
        preserves prior keys alongside `mcp`.
  - [x] New test: MCP overlay + a pre-existing `instructions` key in the env map
        yields a single config object containing **both** `mcp` and
        `instructions` (direct regression for the clobber).
  - [x] New test: a user-supplied non-empty `OPENCODE_CONFIG_CONTENT` object is
        preserved (not replaced) when MCP is injected.

**Validation checkpoint**: `just test-library && just test-cli` green; a focused
test asserts `instructions` + `mcp` coexist after both producers run. `just check`
clean.

---

## Phase 3 — Inject the YOLO permission overlay for non-interactive subagents (the core fix)

**Goal**: make YOLO mean YOLO session-wide. A gated, post-MCP merge of the
permission block into `env_plan.env`, applied last so it is authoritative.

**Dependencies**: Phase 1. **Parallelizable with**: Phase 4. **Note**: the full
acceptance-#4 coexistence test requires Phase 2 to also be merged; sequence the
combined assertion after both land.

- [x] Add a new orchestrator stage "Stage 10.5 — OpenCode YOLO config overlay" in
      `claudine/cli/src/commands/wrap/mod.rs`, placed immediately after
      `compose_mcp_session` (Stage 10) and before `child_cwd` is read
      (`mod.rs:545`). Gate it on
      `provider == Provider::OpenCode && yolo_enabled && non_interactive_requested`.
      When gated-on, call
      `opencode_config::merge_overlay(current, opencode_config::yolo_permission_block())`
      against `env_plan.env["OPENCODE_CONFIG_CONTENT"]` and write the result back.
      When gated-off, do nothing (no `permission` key — acceptance #3).
      `yolo_enabled` is already the single source of truth for "YOLO took effect"
      (`mod.rs:365`) and already reflects OpenCode interactive `not_applied`, so
      no extra interactive guard is needed.
- [x] Factor the stage into a small, unit-testable helper in
      `claudine/cli/src/commands/wrap/wrapper_stages.rs` (e.g.
      `apply_opencode_yolo_config_overlay(provider, yolo_enabled, non_interactive,
      env_plan) -> Result<()>`) and call it from the orchestrator. Keeping it
      isolated makes the gating unit tests (acceptance #2/#3) trivial and keeps
      the orchestrator readable.
- [x] Leave `OpencodeWrapper::apply_yolo_for_mode`
      (`profile/opencode.rs:27-47`) argv-only (`--dangerously-skip-permissions`).
      Do **not** change its signature or push config into `env_overrides` — the
      config overlay is orchestrator-owned so it lands after MCP. Confirm
      existing `apply_yolo` tests still pass unchanged.
- [x] Add unit tests for the new helper (`wrapper_stages` test module):
  - [x] OpenCode + YOLO + non-interactive → `env_plan.env`'s
        `OPENCODE_CONFIG_CONTENT` parses to an object whose `permission.*`,
        `permission.external_directory`, `permission.doom_loop` are all
        `"allow"` (acceptance #2).
  - [x] OpenCode + YOLO + **interactive** → no `permission` key is added
        (mirrors `YoloOutcome::not_applied`).
  - [x] OpenCode + **not** YOLO + non-interactive → no `permission` key
        (acceptance #3); existing env value untouched.
  - [x] Non-OpenCode provider + YOLO + non-interactive → helper is a no-op
        (scope gate; never widens other providers).
  - [x] When `env_plan.env` already carries `instructions` + `mcp` (simulate
        Phase 2 output), the YOLO merge yields one object containing
        `instructions`, `mcp`, **and** `permission` — none dropped (acceptance
        #4 end-to-end; run this assertion once Phase 2 is merged).
- [x] Add an integration-level test for the assembled spawn spec: building the
      OpenCode child for a non-interactive YOLO run yields argv containing
      `--dangerously-skip-permissions` (acceptance #5) **and** an
      `OPENCODE_CONFIG_CONTENT` whose parsed JSON carries the permission block.
      Place alongside existing wrap integration tests
      (`claudine/cli/src/commands/wrap/tests.rs` or a focused new test).

**Validation checkpoint**: `just test-cli` green; the gating matrix
(YOLO×interactive×provider) is covered; the spawn-spec test asserts argv + env
together. `just lint` clean.

---

## Phase 4 — Policy backend consistency (no native `opencode run --yolo`)

**Goal**: the `PolicyEngine` OpenCode path must agree with the wrapper —
auto-approve means `--dangerously-skip-permissions` and/or the merged permission
overlay, never `opencode run --yolo` (which the installed 1.17.11 `run`
subcommand does not expose). Spec §"Policy metadata cleanup" + acceptance #9.

**Dependencies**: Phase 1 (reuse `merge_overlay` + `yolo_permission_block`).
**Parallelizable with**: Phases 2 and 3 (disjoint file:
`claudine/lib/src/permissions/providers/opencode.rs`).

- [x] `parse_cli_overrides` (`:163-173`): in the `CliPolicyInput::Argv` arm, also
      recognize `--dangerously-skip-permissions` (set the same `yolo = true`
      approval flag the `--yolo` branch sets). Keep recognizing `--yolo` for
      backward compatibility of parsed inputs, but the canonical flag is now the
      dangerous-skip variant.
- [x] `build_one_shot_plan` (`:578-612`): for
      `SetApprovalMode(CanonicalApprovalMode::AutoApprove)`, push
      `--dangerously-skip-permissions` (not `--yolo`) onto `argv`, and merge the
      YOLO permission block into the `OPENCODE_CONFIG_CONTENT` overlay via
      `opencode_config::merge_overlay` instead of the current bare
      `env.insert(...)` overwrite (`:600-604`). This makes one-shot auto-approve
      and the wrapper path produce identical, merge-safe env.
- [x] Update the existing test `opencode_one_shot_plan_uses_env_overlay`
    (`:697-719`) which currently asserts `argv.contains("--yolo")`: flip it to
      assert `--dangerously-skip-permissions` is present, `--yolo` is **absent**,
      and the overlay env carries the `permission` block.
- [x] Add unit tests:
  - [x] `parse_cli_overrides` on `["--dangerously-skip-permissions"]` yields the
        auto-approve CLI override (parity with `--yolo`).
  - [x] `build_one_shot_plan` for `AutoApprove` emits `--dangerously-skip-permissions`
        and a merged `OPENCODE_CONFIG_CONTENT` whose JSON has the three `allow`
        permission keys; it does **not** emit `--yolo`.
  - [x] A one-shot plan that also carries a path/command operation merges both
        the operation's permission pattern and the YOLO block into one
        `OPENCODE_CONFIG_CONTENT` (no clobber within the policy path either).
- [x] Confirm the provider catalog stays the authority: do **not** add an
      OpenCode `--yolo` argv path. `YoloSupport::NonInteractiveOnly {
      non_interactive_flag: "--dangerously-skip-permissions" }` remains unchanged
      (spec §"Policy metadata cleanup").

**Validation checkpoint**: `just test-library` green; the flipped one-shot test
and new parity tests pass; `--yolo` is no longer emitted by the policy path.

---

## Phase 5 — Cross-cutting validation, acceptance matrix, and docs

**Goal**: prove the whole fix against every acceptance criterion, then close out
doc/comment drift per AGENTS.md.

**Dependencies**: Phases 2, 3, 4 all merged.

- [x] Walk spec §"Acceptance criteria" 1–9 and add/confirm a test (or an explicit
      observable) for each. Record the mapping in a checklist comment or test
      name so reviewers can trace criterion → test:
  - [x] #1 subagent external path auto-allowed (covered by the permission-block
        presence tests + the spawn-spec test).
  - [x] #2 three `allow` keys present under YOLO non-interactive.
  - [x] #3 no `permission` key when not YOLO.
  - [x] #4 `instructions` + `mcp` + `permission` coexist (Phase 3 combined test).
  - [x] #5 `--dangerously-skip-permissions` still on argv.
  - [x] #6 `doom_loop` auto-allowed.
  - [x] #7 permission JSON byte-identical cross-platform (Phase 1 order-independence
        test + confirm no path content).
  - [x] #8 existing user config merged or redacted-rejected (Phase 1 + Phase 2
        user-value tests).
  - [x] #9 policy path emits no native `--yolo` (Phase 4).
- [x] Run the full local gate from the claudine area:
      `just check && just lint && just doctest && just test`.
- [x] Sanity-check `--dry-run` output for an OpenCode non-interactive YOLO launch
      shows the merged `OPENCODE_CONFIG_CONTENT` (observable, no real spawn) —
      manual or snapshot assertion.
- [x] Comment/doc pass (AGENTS.md "Comment Quality" + Rustdoc convention):
  - [x] At the new Stage 10.5 and the merge helper, document the **WHY**
        (parent-vs-subagent asymmetry; merge-not-overwrite invariant) — a
        contract/invariant comment, not HOW-narration.
  - [x] Remove any now-stale `//` or doc line that claims
        `OPENCODE_CONFIG_CONTENT` is single-writer or that `--yolo` is the
        OpenCode auto-approve flag.
  - [x] Ensure no `# H1` inside `///`; use `## H2` sections per repo Rustdoc
        convention.
- [x] Verify scope gates held: no permission block injected for interactive YOLO
      (already `not_applied`) or for non-OpenCode providers; `.env` protection
      retained for non-YOLO runs (spec §decision 2).

**Validation checkpoint (final)**: `just check && just lint && just doctest &&
just test` all green from the claudine area; every acceptance criterion maps to a
passing test; no behavior change for non-YOLO or non-OpenCode runs (regression
guard from spec §"Test plan → regression").

---

## Parallelism map

- **Phase 1** — serial foundation; starts immediately.
- After Phase 1 lands, **Phases 2, 3, 4** can proceed in parallel (disjoint
  files: Phase 2 → `mcp/inject.rs` + `wrapper_mcp.rs` + `env/mod.rs`;
  Phase 3 → `wrap/mod.rs` + `wrapper_stages.rs`; Phase 4 →
  `permissions/providers/opencode.rs`).
- The **acceptance-#4 coexistence assertion** (Phase 3) and the **final gate**
  (Phase 5) require Phases 2 + 3 (+ 4 for #9) to all be merged.

## Out of scope (per spec)

- 30-minute `step_timeout` watchdog grace tuning.
- Interactive TUI YOLO for OpenCode.
- YOLO for any other provider.
- Steering subagents off `/tmp`.
