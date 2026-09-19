---
created: 2026-09-12
total_phases: 12
phase: 7
agent: claude/opus
yolo: "true"
area: claudine
spec: ./spec.md
source_files_during_phase_1: []
docs_updated_during_phase_1:
    - claudine/docs/research/mcp/codex.md
docs_created_during_phase_1:
    - claudine/fixes/2026-09-12-shadow-home/audit.md
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
    - claudine/catalog-types/src/lib.rs
    - claudine/catalog-types/src/provider_overlay.rs
    - claudine/gen/src/emit/identity_paths.rs
    - claudine/gen/src/emit/mod.rs
    - claudine/gen/src/generate/coerce/mod.rs
    - claudine/gen/src/registry.rs
    - claudine/gen/src/registry/tests.rs
    - claudine/gen/tests/fixtures/generated-artifact-baseline.json
    - claudine/gen/tests/pipeline.rs
    - claudine/gen/tests/registry_coverage.rs
    - claudine/lib/src/provider/antigravity/data.rs
    - claudine/lib/src/provider/claude/data.rs
    - claudine/lib/src/provider/codex/data.rs
    - claudine/lib/src/provider/gemini/data.rs
    - claudine/lib/src/provider/goose/data.rs
    - claudine/lib/src/provider/kilo/data.rs
    - claudine/lib/src/provider/kimi/data.rs
    - claudine/lib/src/provider/methods.rs
    - claudine/lib/src/provider/mod.rs
    - claudine/lib/src/provider/opencode/data.rs
    - claudine/lib/src/provider/overlay.rs
    - claudine/lib/src/provider/pi/data.rs
    - claudine/lib/src/provider/qwen/data.rs
    - claudine/lib/src/provider/tests.rs
docs_updated_during_phase_2:
    - claudine/docs/providers/catalog.json
    - claudine/docs/providers/dispatch-inventory.json
    - claudine/docs/providers/facts/antigravity.yaml
    - claudine/docs/providers/facts/claude.yaml
    - claudine/docs/providers/facts/codex.yaml
    - claudine/docs/providers/facts/gemini.yaml
    - claudine/docs/providers/facts/goose.yaml
    - claudine/docs/providers/facts/kilo.yaml
    - claudine/docs/providers/facts/kimi.yaml
    - claudine/docs/providers/facts/opencode.yaml
    - claudine/docs/providers/facts/pi.yaml
    - claudine/docs/providers/facts/qwen.yaml
    - claudine/fixes/2026-09-12-shadow-home/audit.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
    - claudine/cli/src/commands/wrap/composition/pipeline.rs
    - claudine/cli/src/commands/wrap/env/mod.rs
    - claudine/cli/src/commands/wrap/env/sanitize.rs
    - claudine/cli/src/commands/wrap/env/tests.rs
    - claudine/cli/src/commands/wrap/mod.rs
    - claudine/lib/src/invocation_context.rs
    - claudine/lib/src/invocation_context/tests.rs
docs_updated_during_phase_3:
    - claudine/docs/providers/dispatch-inventory.json
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
    - claudine/cli/src/commands/wrap/profile/codex.rs
    - claudine/cli/src/commands/wrap/profile/gemini.rs
    - claudine/cli/src/commands/wrap/profile/mod.rs
    - claudine/cli/src/commands/wrap/profile/tests/overlay_strategy.rs
    - claudine/cli/src/commands/wrap/repo_home.rs
    - claudine/cli/src/commands/wrap/repo_home/tests.rs
    - claudine/cli/tests/error_guards.rs
    - claudine/lib/src/diagnostics/registry.rs
    - claudine/lib/src/error.rs
    - claudine/lib/src/invocation_context.rs
    - claudine/lib/src/lib.rs
    - claudine/lib/src/provider_overlay/mod.rs
    - claudine/lib/src/provider_overlay/plan.rs
    - claudine/lib/src/provider_overlay/selector.rs
    - claudine/lib/src/provider_overlay/tests.rs
docs_updated_during_phase_4:
    - claudine/docs/providers/dispatch-inventory.json
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
source_files_during_phase_5:
    - claudine/cli/Cargo.toml
    - claudine/cli/src/commands/exec_prep/mod.rs
    - claudine/cli/src/commands/wrap/composition/pipeline.rs
    - claudine/cli/src/commands/wrap/env/mod.rs
    - claudine/cli/src/commands/wrap/mod.rs
    - claudine/cli/src/commands/wrap/profile/codex.rs
    - claudine/cli/src/commands/wrap/profile/mod.rs
    - claudine/cli/src/commands/wrap/provider_overlay.rs
    - claudine/cli/src/commands/wrap/provider_overlay/tests.rs
    - claudine/cli/src/commands/wrap/repo_home.rs
    - claudine/cli/src/commands/wrap/repo_home/tests.rs
    - claudine/cli/tests/dispatch_inventory.rs
    - claudine/cli/tests/wrap_opencode.rs
docs_updated_during_phase_5:
    - claudine/docs/providers/dispatch-inventory.json
docs_created_during_phase_5: []
skills_files_updated_during_phase_5: []
source_files_during_phase_6:
    - claudine/cli/src/commands/exec_prep/mod.rs
    - claudine/cli/src/commands/wrap/composition/pipeline.rs
    - claudine/cli/src/commands/wrap/env/mod.rs
    - claudine/cli/src/commands/wrap/env/tests.rs
    - claudine/cli/src/commands/wrap/exec/spawn/setup.rs
    - claudine/cli/src/commands/wrap/mod.rs
    - claudine/cli/src/commands/wrap/profile/claude.rs
    - claudine/cli/src/commands/wrap/profile/tests/overlay_strategy.rs
    - claudine/cli/src/commands/wrap/provider_overlay.rs
    - claudine/cli/src/commands/wrap/provider_overlay/tests.rs
    - claudine/cli/src/commands/wrap/tests.rs
    - claudine/cli/src/commands/wrap/wrapper_mcp.rs
    - claudine/cli/src/commands/wrap/wrapper_stages.rs
    - claudine/cli/tests/dispatch_inventory.rs
    - claudine/cli/tests/error_guards/transport-allow.toml
    - claudine/cli/tests/level1_provider_overlay_home.rs
    - claudine/cli/tests/propagated_context_fixtures.rs
    - claudine/cli/tests/wrap_basics.rs
    - claudine/cli/tests/wrap_compose_exec.rs
docs_updated_during_phase_6:
    - claudine/docs/providers/dispatch-inventory.json
    - claudine/fixes/2026-09-12-shadow-home/audit.md
docs_created_during_phase_6: []
skills_files_updated_during_phase_6:
    - .claude/skills/os/build-hosts.md
source_files_during_phase_7:
    - claudine/cli/src/commands/wrap/composition/pipeline.rs
    - claudine/cli/src/commands/wrap/launch_plan.rs
    - claudine/cli/src/commands/wrap/wrapper_mcp.rs
    - claudine/cli/tests/level1_provider_overlay_home.rs
    - claudine/lib/src/mcp/inject.rs
    - claudine/lib/src/provider_overlay/plan.rs
docs_updated_during_phase_7:
    - claudine/docs/providers/dispatch-inventory.json
    - claudine/fixes/2026-09-12-shadow-home/audit.md
docs_created_during_phase_7: []
skills_files_updated_during_phase_7: []
source_files_during_phase_8:
    - claudine/cli/src/commands/wrap/composition/pipeline.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/target_launch.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/target_launch/tests.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/retry_resume.rs
    - claudine/cli/src/commands/wrap/harness_orch/session_key.rs
    - claudine/cli/src/commands/wrap/harness_orch/session_key/tests.rs
    - claudine/cli/src/commands/wrap/launch_plan.rs
    - claudine/cli/src/commands/wrap/launch_plan/tests.rs
    - claudine/cli/src/commands/wrap/profile/codex.rs
    - claudine/cli/src/commands/wrap/profile/mod.rs
    - claudine/cli/src/commands/wrap/provider_overlay.rs
    - claudine/cli/src/commands/wrap/provider_overlay/tests.rs
    - claudine/cli/src/commands/wrap/wrapper_stages.rs
    - claudine/cli/src/commands/wrap/wrapper_stages/tests.rs
    - claudine/cli/tests/dispatch_inventory.rs
    - claudine/cli/tests/level1_provider_overlay_home.rs
    - claudine/lib/src/composition/coordinator/active.rs
    - claudine/lib/src/composition/coordinator/tests.rs
docs_updated_during_phase_8:
    - claudine/docs/providers/dispatch-inventory.json
docs_created_during_phase_8: []
skills_files_updated_during_phase_8: []
source_files_during_phase_9:
    - claudine/cli/src/commands/compose/mod.rs
    - claudine/cli/src/commands/wrap/composition/pipeline.rs
    - claudine/cli/src/commands/wrap/env/mod.rs
    - claudine/cli/src/commands/wrap/flags.rs
    - claudine/cli/src/commands/wrap/tests.rs
    - claudine/cli/src/commands/wrap/wrapper_stages.rs
    - claudine/cli/src/output/error_walker/tests.rs
    - claudine/cli/src/output/mod.rs
    - claudine/cli/src/perf/mod.rs
    - claudine/cli/src/perf/report.rs
    - claudine/cli/src/perf/tests/perf_tree.rs
    - claudine/cli/tests/level1_provider_overlay_home.rs
    - claudine/lib/src/composition/coordinator/invocation.rs
    - claudine/lib/src/composition/types.rs
docs_updated_during_phase_9:
    - claudine/cli/README.md
    - claudine/docs/providers/dispatch-inventory.json
    - claudine/docs/topics/composition.md
docs_created_during_phase_9: []
skills_files_updated_during_phase_9: []
source_files_during_phase_10:
    - claudine/cli/src/commands/wrap/env/tests.rs
    - claudine/cli/tests/level1_provider_overlay_home.rs
    - claudine/cli/tests/propagated_context_fixtures.rs
    - claudine/cli/tests/wrap_basics.rs
docs_updated_during_phase_10:
    - claudine/docs/providers/dispatch-inventory.json
docs_created_during_phase_10:
    - claudine/fixes/2026-09-12-shadow-home/test-map.md
skills_files_updated_during_phase_10:
    - .claude/skills/os/build-hosts.md
source_files_during_phase_11:
    - claudine/cli/Cargo.toml
    - claudine/cli/src/commands/wrap/provider_overlay.rs
    - claudine/cli/src/commands/wrap/provider_overlay/tests.rs
    - claudine/cli/tests/level1_provider_overlay_home.rs
    - claudine/cli/tests/level2_provider_overlay_capture.rs
docs_updated_during_phase_11:
    - claudine/docs/providers/dispatch-inventory.json
    - claudine/fixes/2026-09-12-shadow-home/test-map.md
docs_created_during_phase_11: []
skills_files_updated_during_phase_11:
    - .claude/skills/os/build-hosts.md
    - .claude/skills/rust-testing/SKILL.md
source_files_during_phase_12:
    - claudine/cli/src/commands/wrap/composition/pipeline.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/target_launch.rs
    - claudine/cli/src/commands/wrap/provider_overlay/tests.rs
    - claudine/cli/tests/dispatch_inventory.rs
    - claudine/cli/tests/level1_provider_overlay_home.rs
    - claudine/gen/src/registry.rs
    - claudine/lib/src/provider/mod.rs
    - claudine/lib/src/provider/system_prompt.rs
    - claudine/lib/src/provider_overlay/tests.rs
docs_updated_during_phase_12:
    - claudine/README.md
    - claudine/cli/README.md
    - claudine/docs/pipeline.md
    - claudine/docs/research/acp/antigravity.md
    - claudine/docs/research/mcp/codex.md
    - claudine/docs/topics/building-an-agent-wrapper.md
    - claudine/docs/topics/composition.md
    - claudine/docs/topics/execution-flow.md
    - claudine/docs/topics/how-to-create-a-new-provider.md
    - claudine/docs/topics/mcp-mode.md
    - claudine/docs/topics/provider-metadata.md
    - claudine/docs/topics/repo-isolation.md
    - claudine/docs/topics/system-prompt.md
    - claudine/docs/topics/wrapped-execution-switches.md
    - claudine/fixes/2026-09-12-shadow-home/implementation-log.md
    - claudine/lib/README.md
docs_created_during_phase_12:
    - claudine/fixes/2026-09-12-shadow-home/acceptance.md
skills_files_updated_during_phase_12:
    - .claude/skills/claudine/SKILL.md
    - .claude/skills/claudine/architecture.md
    - .claude/skills/claudine/cli-reference.md
    - .claude/skills/claudine/composition.md
    - .claude/skills/claudine/mcp-mode.md
    - .claude/skills/claudine/system-prompt.md
    - .claude/skills/claudine/timeline.md
source_code:
    - claudine/catalog-types/src/lib.rs
    - claudine/catalog-types/src/provider_overlay.rs
    - claudine/cli/Cargo.toml
    - claudine/cli/src/commands/compose/mod.rs
    - claudine/cli/src/commands/exec_prep/mod.rs
    - claudine/cli/src/commands/wrap/composition/pipeline.rs
    - claudine/cli/src/commands/wrap/env/mod.rs
    - claudine/cli/src/commands/wrap/env/sanitize.rs
    - claudine/cli/src/commands/wrap/env/tests.rs
    - claudine/cli/src/commands/wrap/exec/spawn/setup.rs
    - claudine/cli/src/commands/wrap/flags.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/target_launch.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/target_launch/tests.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/retry_resume.rs
    - claudine/cli/src/commands/wrap/harness_orch/session_key.rs
    - claudine/cli/src/commands/wrap/harness_orch/session_key/tests.rs
    - claudine/cli/src/commands/wrap/launch_plan.rs
    - claudine/cli/src/commands/wrap/launch_plan/tests.rs
    - claudine/cli/src/commands/wrap/mod.rs
    - claudine/cli/src/commands/wrap/profile/claude.rs
    - claudine/cli/src/commands/wrap/profile/codex.rs
    - claudine/cli/src/commands/wrap/profile/gemini.rs
    - claudine/cli/src/commands/wrap/profile/mod.rs
    - claudine/cli/src/commands/wrap/profile/tests/overlay_strategy.rs
    - claudine/cli/src/commands/wrap/provider_overlay.rs
    - claudine/cli/src/commands/wrap/provider_overlay/tests.rs
    - claudine/cli/src/commands/wrap/tests.rs
    - claudine/cli/src/commands/wrap/wrapper_mcp.rs
    - claudine/cli/src/commands/wrap/wrapper_stages.rs
    - claudine/cli/src/commands/wrap/wrapper_stages/tests.rs
    - claudine/cli/src/output/error_walker/tests.rs
    - claudine/cli/src/output/mod.rs
    - claudine/cli/src/perf/mod.rs
    - claudine/cli/src/perf/report.rs
    - claudine/cli/src/perf/tests/perf_tree.rs
    - claudine/cli/tests/dispatch_inventory.rs
    - claudine/cli/tests/error_guards.rs
    - claudine/cli/tests/error_guards/transport-allow.toml
    - claudine/cli/tests/level1_provider_overlay_home.rs
    - claudine/cli/tests/level2_provider_overlay_capture.rs
    - claudine/cli/tests/propagated_context_fixtures.rs
    - claudine/cli/tests/wrap_basics.rs
    - claudine/cli/tests/wrap_compose_exec.rs
    - claudine/cli/tests/wrap_opencode.rs
    - claudine/gen/src/emit/identity_paths.rs
    - claudine/gen/src/emit/mod.rs
    - claudine/gen/src/generate/coerce/mod.rs
    - claudine/gen/src/registry.rs
    - claudine/gen/src/registry/tests.rs
    - claudine/gen/tests/fixtures/generated-artifact-baseline.json
    - claudine/gen/tests/pipeline.rs
    - claudine/gen/tests/registry_coverage.rs
    - claudine/lib/src/composition/coordinator/active.rs
    - claudine/lib/src/composition/coordinator/invocation.rs
    - claudine/lib/src/composition/coordinator/tests.rs
    - claudine/lib/src/composition/types.rs
    - claudine/lib/src/diagnostics/registry.rs
    - claudine/lib/src/error.rs
    - claudine/lib/src/invocation_context.rs
    - claudine/lib/src/invocation_context/tests.rs
    - claudine/lib/src/lib.rs
    - claudine/lib/src/mcp/inject.rs
    - claudine/lib/src/provider/antigravity/data.rs
    - claudine/lib/src/provider/claude/data.rs
    - claudine/lib/src/provider/codex/data.rs
    - claudine/lib/src/provider/gemini/data.rs
    - claudine/lib/src/provider/goose/data.rs
    - claudine/lib/src/provider/kilo/data.rs
    - claudine/lib/src/provider/kimi/data.rs
    - claudine/lib/src/provider/methods.rs
    - claudine/lib/src/provider/mod.rs
    - claudine/lib/src/provider/opencode/data.rs
    - claudine/lib/src/provider/overlay.rs
    - claudine/lib/src/provider/pi/data.rs
    - claudine/lib/src/provider/qwen/data.rs
    - claudine/lib/src/provider/system_prompt.rs
    - claudine/lib/src/provider/tests.rs
    - claudine/lib/src/provider_overlay/mod.rs
    - claudine/lib/src/provider_overlay/plan.rs
    - claudine/lib/src/provider_overlay/selector.rs
    - claudine/lib/src/provider_overlay/tests.rs
source_code_deleted:
    - claudine/cli/src/commands/wrap/repo_home.rs
    - claudine/cli/src/commands/wrap/repo_home/tests.rs
documentation:
    - claudine/README.md
    - claudine/cli/README.md
    - claudine/docs/pipeline.md
    - claudine/docs/providers/catalog.json
    - claudine/docs/providers/dispatch-inventory.json
    - claudine/docs/providers/facts/antigravity.yaml
    - claudine/docs/providers/facts/claude.yaml
    - claudine/docs/providers/facts/codex.yaml
    - claudine/docs/providers/facts/gemini.yaml
    - claudine/docs/providers/facts/goose.yaml
    - claudine/docs/providers/facts/kilo.yaml
    - claudine/docs/providers/facts/kimi.yaml
    - claudine/docs/providers/facts/opencode.yaml
    - claudine/docs/providers/facts/pi.yaml
    - claudine/docs/providers/facts/qwen.yaml
    - claudine/docs/research/acp/antigravity.md
    - claudine/docs/research/mcp/codex.md
    - claudine/docs/topics/building-an-agent-wrapper.md
    - claudine/docs/topics/composition.md
    - claudine/docs/topics/execution-flow.md
    - claudine/docs/topics/how-to-create-a-new-provider.md
    - claudine/docs/topics/mcp-mode.md
    - claudine/docs/topics/provider-metadata.md
    - claudine/docs/topics/repo-isolation.md
    - claudine/docs/topics/system-prompt.md
    - claudine/docs/topics/wrapped-execution-switches.md
    - claudine/fixes/2026-09-12-shadow-home/acceptance.md
    - claudine/fixes/2026-09-12-shadow-home/audit.md
    - claudine/fixes/2026-09-12-shadow-home/implementation-log.md
    - claudine/fixes/2026-09-12-shadow-home/test-map.md
    - claudine/lib/README.md
completed_phase: "12"
implemented: true
packages:
    - claudine
    - claudine-cli
    - claudine-gen
human_review: true
human_review_items:
    - "Native Windows L2 cannot be hermetic under today's home authority (unchanged from Phase 11). The overlay's source and storage roots come from HomeBaseline::resolved(), which is dirs::home_dir() and reads the known-folder profile on native Windows, ignoring USERPROFILE and HOME, so an end-to-end `claudine codex --repo` on Windows reads and writes the real %USERPROFILE%\\.claudine and .codex. Decide: (a) move the overlay home to the environment-first std::env::home_dir(); (b) accept Phase 5's copy-mode unit tests as native Windows materialization evidence and drop the Windows L2 variant; or (c) run the Windows L2 on a throwaway account. acceptance.md criterion 9 stays unmet until then."
    - "Windows and WSL2 evidence is still missing (acceptance.md criterion 9). build-win-native's W: volume was full and the WSL guest (VHDX on W:) reset SSH at key exchange; Phase 12 did not re-attempt. CI's claudine-cli windows-latest/L2 and wsl2-ubuntu/L2 cells are accepted gaps. Someone must free W: and run the L2 there before the fix can be called fully accepted."
    - "Behavior change made in Phase 11, for veto (unchanged): a missing DEFAULT provider source root builds an empty overlay instead of refusing; a missing EXPLICIT root still refuses with provider.overlay_failed. Phase 12 documented this in docs/topics/repo-isolation.md -> When the Overlay Cannot Be Built and docs/topics/mcp-mode.md; a veto must update both."
    - "Carried forward, unanswered: the undocumented CLAUDE_SECURESTORAGE_CONFIG_DIR pin for Claude overlays. Phase 12 documented it as observed-but-undocumented in docs/topics/repo-isolation.md -> Claude Special Case."
    - "Checkpoint 12's grep does not return only _completed hits. The remainder is provider research (docs/research/system-prompt/*), the `shadow_home_file` research-schema enum that claudine-gen still maps, the ShadowHomeFile variant name, one historical module comment, one negative test assertion, and the timeline entry. None describes current behavior (list in acceptance.md -> Checkpoint grep). Decide whether renaming the research enum/variant is wanted; it is a schema + code change, not documentation."
    - "just test-l2 is red only on level2_lifecycle_control::level2_shipped_implement_plan_{supplied_commit_message_runs_exact_commit_branch, unset_commit_message_reaches_provider_and_auto_branch}, caused by the uncommitted shipped_implement_route/_implement/implement-plan.md fixture edit from other in-flight work (bare {{area}}). Not this fix, but acceptance criterion 12 cannot be marked met while it is red."
message_to_agent: |
    Phase 12 was the final phase. Nothing remains to implement; the fix is
    "implementation complete, ready for review", with the unmet acceptance items
    named in human_review_items. Do not move the fix directory to _completed.

    For a reviewer or follow-up agent:

    1. acceptance.md is the verdict: 10 met, 2 unmet (criterion 9: native Windows
       and WSL2 evidence; criterion 12: two unrelated L2 failures).
    2. docs/topics/repo-isolation.md is now the authoritative user-facing
       contract. Its masking table must match `repo_isolated_resources`
       (lib/src/provider_overlay/plan.rs); both table tests hard-code the same
       rows, and Pi was added to both in Phase 12. The `.goose`/`.opencode` arms
       of `repo_isolated_resources` are dead (both providers refuse --repo) and
       were left in place.
    3. Kilo's `mcp` verdict is `composable_injection`, but Kilo has no runtime
       injector, so the verdict is inert. Docs say only OpenCode injects inline.
    4. If the Phase 11 missing-default-root behavior is vetoed, update
       repo-isolation.md and mcp-mode.md (docs + skill mirror) with the code.
---

# Execution Plan — Preserve Provider Overlays Without Replacing the User Home

Converts `spec.md` into an ordered, observable execution plan.

## Reading This Plan

- Phases run in order. Tasks inside a phase run in order unless marked
  **‖ parallel**, which means they touch disjoint files and may run
  concurrently with the other **‖ parallel** tasks in the same group.
- Each phase ends with a **Checkpoint** — a concrete, runnable verification.
  Do not start the next phase until its checkpoint passes.
- `just` recipes run from the `claudine/` package area unless stated otherwise.

## Naming Decisions Fixed Up Front

These names are used consistently by every phase. Change them here, once, if
the implementation team disagrees — do not diverge mid-plan.

| Concept | Name | Home |
|---|---|---|
| Immutable launch home snapshot | `HomeBaseline` | `claudine/lib/src/invocation_context.rs` |
| Immutable launch env snapshot (raw `OsString`) | `EnvBaseline` | `claudine/lib/src/invocation_context.rs` |
| Typed overlay plan | `OverlayPlan` | `claudine/lib/src/provider_overlay/plan.rs` |
| Why an overlay is needed | `OverlayReason::{RepoResources, RepoPrompt, Mcp}` | same |
| Per-(provider, reason) verdict | `OverlayCapability::{NativeRoot, ComposableInjection, Unsupported}` | `claudine/lib/src/provider/mod.rs` (generated) |
| Provider-owned env/arg patch | `OverlaySelector` | `claudine/lib/src/provider_overlay/selector.rs` |
| Pre-spawn typed failure | `ClaudineError::ProviderOverlayUnsupported` / `::ProviderOverlayFailed` | `claudine/lib/src/error.rs` |

The existing on-disk path `~/.claudine/<agent-offset>` is **retained** where a
provider's selector accepts that shape (Design → Documentation and Migration).
No destructive migration is authorized in any phase.

---

## Phase 1 — Audit and Capability Ground Truth

Produces the evidence the rest of the plan depends on. **No production code
changes in this phase.** Output is a committed audit document plus per-provider
research/facts updates.

The single highest-risk unknown: some selectors name the provider's own config
directory (`CODEX_HOME` → `.codex` itself) and others name a *parent* under
which the provider creates its normal directory. Getting this backwards silently
produces `~/.claudine/.codex/.codex/config.toml` and a provider that reads the
user's real config. Every selector must be observed, not inferred.

- [x] Create `claudine/fixes/2026-09-12-shadow-home/audit.md` with one row per
      (provider, activation reason) pair — 10 providers × 3 reasons
      (`repo_resources`, `repo_prompt`, `mcp`) — and columns: selector name,
      path shape (`ProviderDir` | `ParentOfProviderDir` | `Inline` | `None`),
      classes relocated (config / auth / sessions / cache / state), additive-vs-
      exclusive discovery, verdict, and the evidence that establishes it.
- [x] Read the existing evidence already in the repo before probing anything:
      `claudine/docs/providers/facts/*.yaml`, `claudine/docs/research/mcp/*.md`,
      `claudine/docs/research/system-prompt/*.md`, and the current
      `original_home()` special case for `CODEX_HOME` in
      `claudine/cli/src/commands/wrap/repo_home.rs:34-42`. Record which rows are
      already answered so probing is limited to genuine gaps.
- [x] **‖ parallel** Establish the path shape for `CODEX_HOME` and confirm the
      pre-overlay `CODEX_SQLITE_HOME` resolution chain
      (`repo_home.rs::codex_sqlite_home`) is unaffected by the shape finding.
- [x] **‖ parallel** Establish the path shape and relocated classes for
      `GEMINI_CLI_HOME`. This is load-bearing for L1 test 6 (Gemini MCP
      injection must write where `GEMINI_CLI_HOME` selects, not where `HOME`
      implies) and for the `mcp-server-enablement.json` /
      `mcp-oauth-tokens.json` copies in
      `claudine/lib/src/mcp/inject.rs:296-297`.
- [x] **‖ parallel** Establish shape and class coverage for `CLAUDE_CONFIG_DIR`,
      `GOOSE_PATH_ROOT`, `KIMI_CODE_HOME`, `QWEN_HOME`, `PI_CODING_AGENT_DIR`,
      and Kilo's config overlays.
- [x] **‖ parallel** Establish whether `OPENCODE_CONFIG_DIR` is additive. The
      spec is explicit that an additive directory **cannot** claim
      `repo_resources` isolation until the remaining user discovery paths are
      disabled or filtered. If it is additive and cannot be closed, OpenCode's
      `repo_resources` verdict is `Unsupported`; its `mcp` verdict stays
      `ComposableInjection` via the existing `OPENCODE_CONFIG_CONTENT` path.
- [x] **‖ parallel** Confirm Antigravity's position. `claudine/docs/providers/
      facts/antigravity.yaml:239` already records that changing `HOME` breaks
      keyring auth and that no safe runtime-injection surface exists. Unless a
      verified selector is found, every Antigravity reason is `Unsupported`.
- [x] For every row whose answer is not already in the repo, add or update the
      authoritative research doc under `claudine/docs/research/<topic>/<slug>.md`
      and/or `claudine/docs/providers/facts/<slug>.yaml`. **Never hand-edit a
      generated `lib/src/provider/<slug>/data.rs`.**
- [x] Record in `audit.md` the explicit list of (provider, reason) pairs that
      will become pre-spawn refusals, so Phase 6 and Phase 10 test the same set
      and the behavior tightening is visible in one place.

**Checkpoint 1.** `audit.md` has no blank verdict cells; each `NativeRoot`
verdict cites a path shape and the evidence that established it; each
`Unsupported` verdict states what would have to be verified to change it. Run
`just lint` to confirm the repo is still clean (no code changed yet).

---

## Phase 2 — Provider Metadata: Facts → Codegen → `ProviderInfo`

Moves the audit's verdicts into generated, drift-checked provider metadata so
the planner is table-driven and the dispatch-inventory guard stays clean.

- [x] Add facts keys to `claudine/docs/providers/facts/<slug>.yaml` for all 10
      providers: `overlay_selector` (record: `env_var`, `shape`,
      `relocates` string array, `additive` bool; or null) and
      `overlay_capabilities` (record keyed by the three reasons, each a member of
      the `OverlayCapability` vocabulary).
- [x] Register both keys in `claudine/gen/src/registry.rs` beside the existing
      `repo_home_root_files` entry (`registry.rs:616-624`), with
      `DeclaredSource::Facts`, the matching `SchemaExpectation`, and new
      `Coercion` variants.
- [x] Add the `OverlayCapability` and `OverlaySelectorShape` enums to
      `claudine/catalog-types` so `gen` and `lib` share one vocabulary, matching
      how `ResumeSupport` is shared today.
- [x] Emit the new fields from `claudine/gen/src/emit/` (follow
      `models_offerings.rs:262`, which emits `repo_home_root_files`) and add the
      corresponding `pub` fields to `ProviderInfo` in
      `claudine/lib/src/provider/mod.rs` near `repo_home_root_files`
      (`mod.rs:301`).
- [x] Add accessors on `Provider` in `claudine/lib/src/provider/methods.rs`
      (mirroring `agent_offset()` at `methods.rs:212`) —
      `overlay_selector()` and `overlay_capability(reason)`.
- [x] Update `claudine/gen/tests/registry_coverage.rs` (the field allow-list at
      `registry_coverage.rs:25-50`) and `claudine/gen/tests/pipeline.rs` for the
      new fields.
- [x] Regenerate: `claudine providers generate` from the `claudine/` area, then
      confirm every `lib/src/provider/<slug>/data.rs` diff contains only the two
      new fields.
- [x] Extend `claudine/lib/src/provider/tests.rs` with invariants: a
      `NativeRoot` verdict for any reason requires a non-null `overlay_selector`;
      an `additive: true` selector may not carry a `NativeRoot` verdict for
      `repo_resources`; `Unsupported` is legal with or without a selector.

**Checkpoint 2.** `just test-gen` and `just test-library` pass. `just test`
passes, including the `dispatch_inventory` drift test — confirming no new
`match Provider` site was introduced. `git diff` on `data.rs` files shows only
additive generated fields.

---

## Phase 3 — Capture One Immutable Launch Baseline

Satisfies Design → "Capture one immutable launch baseline" and Invariant 2.
`InvocationContext` already owns `home_dir` (`invocation_context.rs:711`) and a
lossy `HashMap<String, String>` environment (`:656-694`); neither preserves the
raw `OsString` presence/absence the spec requires.

- [x] Add `HomeBaseline` to `claudine/lib/src/invocation_context.rs`: the
      resolved user home plus the exact presence-or-absence and raw `OsString`
      value of `HOME`, and on Windows `USERPROFILE`, `HOMEDRIVE`, `HOMEPATH`,
      plus `HOME` when the caller supplied it. Model absence as `Option`, never
      as an empty string.
- [x] Add `EnvBaseline` — the full launch environment as raw
      `Vec<(OsString, OsString)>` — captured in `capture_with_observation`
      alongside the existing lossy `environment` map. Keep the existing map; do
      not break its consumers.
- [x] Expose `InvocationContext::home_baseline()` and `env_baseline()`. Reuse the
      existing `biscuit_file::home_dir()` authority already imported at
      `invocation_context.rs:8` — do **not** add a `sniff` host-discovery call.
- [x] Change `sanitize_process_env` in
      `claudine/cli/src/commands/wrap/env/sanitize.rs:36-64` to iterate the
      supplied `EnvBaseline` instead of `std::env::vars_os()`. Thread the
      baseline through `build_child_env_with_launch`
      (`env/mod.rs:197`) and `build_child_env` (`env/mod.rs:160`).
- [x] Update the two production callers to pass the invocation's baseline:
      `claudine/cli/src/commands/wrap/mod.rs:555` and
      `claudine/cli/src/commands/wrap/composition/pipeline.rs:473`. Both already
      hold the `InvocationContext`.
- [x] Add unit tests in `claudine/lib/src/invocation_context/tests.rs` (extend
      the existing serialized env-guard pattern at `tests.rs:41-104`): mutating
      `HOME` after capture does not move the baseline; an absent `HOME` is
      captured as absent and is **not** synthesized from `USERPROFILE`; a
      non-UTF-8 `HOME` survives round-trip on Unix.

**Checkpoint 3.** `just test` passes. A new test proves `sanitize_process_env`
ignores a post-capture ambient mutation. No behavior change is user-visible yet
— `HOME` is still being overwritten downstream.

---

## Phase 4 — The Typed Overlay Plan and the Profile Seam

Replaces the boolean `force_shadow_home` / `needs_shadow_home` / raw
`Option<PathBuf>` triple (`env/mod.rs:170,287-294`) with one typed plan.

- [x] Create `claudine/lib/src/provider_overlay/` with `mod.rs`, `plan.rs`,
      `selector.rs`. Register it in `claudine/lib/src/lib.rs`.
- [x] Define `OverlayReason` (`RepoResources`, `RepoPrompt`, `Mcp`) as a set —
      one launch can carry several reasons at once, which is exactly the case
      `--repo` plus `--mcp` produces today.
- [x] Define `OverlayPlan` carrying every field the spec enumerates: the reason
      set; the selected provider; the pre-overlay provider **source root**; the
      overlay **storage root** and the **provider-visible root** the selector
      expects; the provider-owned env/argument patch; resource classes to exclude
      or materialize; state that must remain at its pre-overlay location; and
      whether the requested reason set is supported.
- [x] Define `OverlaySelector` — the (env var, shape, value) triple plus the
      `provider_visible_root(storage_root)` function that applies the shape.
      `ProviderDir` yields the provider directory itself; `ParentOfProviderDir`
      yields its parent. This function is the *only* place the shape distinction
      is expressed.
- [x] Add `OverlayPlanner::plan(provider, reasons, source_root, baseline) ->
      Result<OverlayPlan, OverlayRefusal>`. It reads
      `provider.overlay_capability(reason)` and `provider.overlay_selector()`
      from Phase 2's generated metadata — table-driven, no `match Provider`.
- [x] Resolve the **source root** before the selector is applied, per Design →
      "Preserve user intent and provider state": an explicit user-supplied
      provider root (e.g. an ambient `CODEX_HOME`) is the source the overlay is
      built *from*, never the destination and never replaced by a hard-coded
      default. This generalizes `repo_home.rs::original_home()`.
- [x] Add `WrapperProfile::overlay_strategy(&self, plan: &mut OverlayPlan)` to
      `claudine/cli/src/commands/wrap/profile/mod.rs` (default: no-op) for
      provider-specific *path construction and side effects only* — Codex prompt
      materialization, Gemini token-file copies. Policy stays in metadata.
- [x] Implement `overlay_strategy` in `profile/codex.rs` (prompt overlay +
      SQLite state pinned outside the overlay) and `profile/gemini.rs`
      (`mcp-server-enablement.json` / `mcp-oauth-tokens.json`). Leave the other
      eight profiles on the default.
- [x] Implement `OpenCode`'s `Mcp` reason as `ComposableInjection` producing a
      plan with **no filesystem overlay and no storage root** — Design is
      explicit that `OPENCODE_CONFIG_CONTENT` must not acquire a filesystem or
      home override for uniformity.
- [x] Add `ClaudineError::ProviderOverlayUnsupported { provider, reason,
      selector, next_action }` and `::ProviderOverlayFailed { provider, reason,
      stage, source }` to `claudine/lib/src/error.rs`, and register both in
      `claudine/lib/src/diagnostics/registry.rs`. Neither may carry a credential
      value or secret file content (Invariant 8).
- [x] Unit-test the planner in `claudine/lib/src/provider_overlay/tests.rs`:
      shape application for both shapes; an explicit source root is carried as
      source and not as destination; an `Unsupported` verdict returns a refusal
      rather than a weakened plan; a multi-reason request where one reason is
      unsupported refuses **only** that reason set, leaving a no-overlay launch
      available.

**Checkpoint 4.** `just test` passes. The planner is fully unit-tested and
compiles, but nothing calls it yet. `just lint` passes, including
`lint-transport` (the new typed errors must not collapse to prose).

---

## Phase 5 — Overlay Materialization

Rewrites `claudine/cli/src/commands/wrap/repo_home.rs` (473 lines) into
`claudine/cli/src/commands/wrap/provider_overlay.rs`, driven by `OverlayPlan`.
This phase changes *how* the overlay is built; it does not yet change `HOME`.

- [x] Move and rename the module. Keep `codex_sqlite_home()` behavior
      byte-for-byte (`repo_home.rs:219-238`) — it is a completed contract
      (spec → Current Triggers and Contracts) and must keep resolving from the
      **pre-overlay** environment.
- [x] Replace `sync_shadow_home`'s indiscriminate `read_dir` mirror
      (`repo_home.rs:71-139`) with classification driven by the plan's
      exclude/materialize/keep-in-place sets:
      - stable settings and credential references → materialized;
      - `--repo`-selected resource classes → omitted, with repo-scoped
        replacements materialized where needed;
      - live state (databases, journals, lock files, sockets) → **never**
        materialized; a provider-native external state selector is used instead.
- [x] Keep `purge_volatile_state` (`repo_home.rs:179-201`) and
      `is_volatile_state_file` (`:206-212`) intact — legacy linked SQLite state
      must still be swept, and existing regular legacy databases under the old
      overlay must remain (spec: "not deleted by this fix").
- [x] Fix the native-Windows materialization. The current `#[cfg(not(unix))]`
      branch (`repo_home.rs:119-134`) hard-links only one directory level deep
      and hard-links files unconditionally. Replace with a recursive,
      provider-safe **copy** (or a supported link where the entry is proven
      stable), handling nested directories to arbitrary depth. Directory hard
      links do not exist; mutable secret or state files are never hard-linked to
      make Windows imitate Unix symlinks.
- [x] Keep the Unix symlink path for stable entries (`repo_home.rs:111-115`,
      `link_or_copy_file:432-456`).
- [x] Route every overlay write through the existing Claudine config-write
      contracts (`claudine::config::atomic::atomic_write`) for atomic
      replacement and cleanup, as Design requires.
- [x] Migrate the existing tests in
      `claudine/cli/src/commands/wrap/repo_home/tests.rs` to the new module and
      add: recursive nested-directory materialization; a mutable state file is
      neither linked nor copied; an explicit source root is read from, not
      written to.
- [x] Update the `repo_home` re-export path used by
      `claudine/cli/src/commands/exec_prep/mod.rs:135` and
      `composition/pipeline.rs:527`.

**Checkpoint 5.** `just test` passes. Windows materialization has a test that
fails against the old one-level implementation — run it and confirm it is
non-vacuous before moving on (repo rule: prove new tests non-vacuous).

---

## Phase 6 — Direct Wrapper Wiring and Fail-Closed Diagnostics

The behavior change lands here: `HOME` stops moving.

- [x] In `claudine/cli/src/commands/wrap/env/mod.rs`, replace the
      `force_shadow_home: bool` parameter (`:170`, `:206`) and the
      `needs_shadow_home` boolean (`:288-294`) with an `OverlayReason` set
      computed by the caller and an `Option<OverlayPlan>` on `EnvPlan`
      (replacing `shadow_home_path`, `:125`).
- [x] **Delete the `/dev/null` fallback** at `env/mod.rs:317-322`. Overlay
      planning or materialization failure now returns
      `ProviderOverlayFailed`/`ProviderOverlayUnsupported` **before** the
      provider is spawned. Remove the "failed to create shadow HOME" warning
      string entirely.
- [x] Remove the `env.insert("HOME", shadow_home_root)` write at
      `repo_home.rs:309` (now in `provider_overlay.rs`). The child env instead
      receives the plan's provider-owned selector patch. The `HOME` /
      `USERPROFILE` / `HOMEDRIVE` / `HOMEPATH` values projected from the Phase 3
      baseline pass through untouched.
- [x] Update the direct wrapper caller at
      `claudine/cli/src/commands/wrap/mod.rs:551-565` to compute the reason set
      (`--repo` → `RepoResources`; Codex repo-prompt discovery → `RepoPrompt`;
      `--mcp`/`--use` → `Mcp`) and pass it through. The Codex/Gemini hard-coded
      `matches!(provider, Provider::Codex | Provider::Gemini)` at `:551-552`
      becomes a metadata lookup — one fewer dispatch site.
- [x] Update `claudine/cli/src/commands/exec_prep/mod.rs:116-146`
      (`ensure_shadow_home` → `ensure_provider_overlay`), including its doc
      comment, which currently documents the `HOME=/dev/null` degradation
      (`:123`). That comment is now stale and must be rewritten, not deleted
      silently.
- [x] Add a `debug_assert`-backed guard (or a small pure function all env
      assembly routes through) enforcing Invariant 1: no code path may write a
      provider-overlay path, `/dev/null`, `NUL`, or an equivalent sentinel into
      `HOME`, `USERPROFILE`, `HOMEDRIVE`, or `HOMEPATH`.
- [x] Wire the refusal so a normal wrapper launch needing no overlay remains
      available: refuse only the requested mode, never the whole command.

**Checkpoint 6.** `just test` passes. Manually verify with a fake provider that
`claudine codex --repo` in a scratch repo leaves `HOME` at the launch value, and
that `claudine antigravity --repo` refuses before spawn with a typed diagnostic
that does not mention credentials. `grep -rn "dev/null" claudine/cli/src` returns
no production env-assembly hit.

---

## Phase 7 — MCP Injection on the Provider-Owned Root

`claudine/lib/src/mcp/inject.rs` currently takes `shadow_home: Option<&Path>`
and derives the config dir by joining the agent offset (`:155`, `:290`) — a
`HOME`-shaped assumption.

- [x] Change the `McpInjector::inject` signature (`inject.rs:34-42`) from
      `shadow_home: Option<&Path>` to `config_root: Option<&Path>`, meaning the
      directory that directly contains `config.toml` / `settings.json`.
- [x] Update `CodexInjector` (`inject.rs:144-166`) to use `config_root`
      directly instead of `home.join(".codex")`, and `GeminiInjector`
      (`inject.rs:281-297`) instead of `home.join(".gemini")`. The caller
      supplies the value from `OverlaySelector::provider_visible_root`, so the
      `ProviderDir` vs `ParentOfProviderDir` distinction from Phase 1 is honored
      in exactly one place.
- [x] Confirm Gemini's `mcp-server-enablement.json` and `mcp-oauth-tokens.json`
      copies (`inject.rs:296-297`) land under the `GEMINI_CLI_HOME`-selected
      root, satisfying L1 test 6.
- [x] Leave `OpenCodeInjector` (`inject.rs:50-63`) taking `None` — it already
      ignores the parameter. Ensure no caller now materializes an overlay for
      OpenCode MCP (L1 test 7).
- [x] Update the injector error text (`inject.rs:150-153`, `:287-290`) from
      "requires a shadow HOME" to the provider-overlay vocabulary, and make it a
      typed `ProviderOverlayFailed` rather than a bare `ConfigValidation` string.
- [x] Update call sites: `claudine/cli/src/commands/wrap/wrapper_mcp.rs:204`,
      `claudine/cli/src/commands/wrap/launch_plan.rs:943` (and the
      `McpRebuildInputs.shadow_home` field at `:483`), and
      `composition/pipeline.rs:660-673`.
- [x] Update the existing injector unit tests (`inject.rs:563`, `:604`, `:666`)
      to the new parameter, renaming
      `file_backed_injectors_require_shadow_home` accordingly.

**Checkpoint 7.** `just test` passes. `claudine codex --mcp` and
`claudine gemini --mcp` against fake providers write their config to the
selector-designated root, and the launch's `HOME` is unchanged.

---

## Phase 8 — Composition, Sequence, Proxy, Retry, and Resume

Design → "Preserve the user home for descendants" and Invariant 7. Everything
here shares Phase 4's single planning path.

- [x] In `claudine/cli/src/commands/wrap/composition/pipeline.rs:460-482`,
      replace `needs_mcp_shadow_home` / `needs_repo_shadow_home` with the same
      `OverlayReason` set the direct wrapper computes, calling the same planner.
      The hard-coded `matches!(provider, Provider::Codex | Provider::Gemini)` at
      `:461` becomes a metadata lookup.
- [x] Replace the ad hoc `pre_provider_env` restoration at
      `pipeline.rs:524-551` — which today special-cases exactly two keys,
      `CODEX_SQLITE_HOME` and `HOME` — with a generic rule: on a provider
      transition, **every** selector owned by any provider is removed from the
      baseline, and an explicit ambient value is restored when one existed. This
      is what closes Invariant 7 for the eight providers the current code does
      not name.
- [x] Keep the `CODEX_SQLITE_HOME` restore semantics identical
      (`pipeline.rs:534-541`): remove when absent ambiently, restore the explicit
      value when present.
- [x] Delete the now-obsolete comment block at `pipeline.rs:543-546` describing
      `HOME` as "the one provider-shaped key written before this point" — it is
      the exact drift the spec's Code Comment Quality rule targets.
- [x] Add an `overlay` facet to `SessionCompatibilityKey`
      (`claudine/lib/src/composition/`, consumed by
      `claudine/cli/src/commands/wrap/harness_orch/session_key.rs:105-124`). Its
      value folds the complete overlay plan — reason set, selector name and
      value, provider-visible root, exclude set — so the recorded key cannot
      describe a different config/resource view than the child receives.
- [x] Extend the module doc at `session_key.rs:13-54` ("Which facets are
      authoritative") with the new facet and its derivation. The doc is a
      maintained contract; leaving it un-updated is drift.
- [x] Confirm the rebuild path (`harness_orch/loop_control.rs:1806-1862`) passes
      the rebuilt plan's overlay, not the invocation-opening one — a retry spawns
      under the refreshed plan.
- [x] Verify the `exec_prep::ensure_provider_overlay` pre-materialization at
      `pipeline.rs:653-658` still runs when MCP is in play for *any* provider, so
      a refreshed document that moves to a filesystem-backed injector at a retry
      boundary finds the overlay on disk.
- [x] Extend `harness_orch/loop_control/tests/retry_resume.rs` with a refusal
      case where only the overlay facet moved.

**Checkpoint 8.** `just test` passes, including the retry/resume suites. Add and
run a focused test proving a Codex→OpenCode proxy transition leaves no
`CODEX_HOME`/`CODEX_SQLITE_HOME` in the OpenCode child env.

---

## Phase 9 — Diagnostics, Output, and Perf Surface

- [x] Rewrite `repo_flag_info_message` in
      `claudine/cli/src/output/mod.rs:415-435`. It currently prints "A shadow
      HOME has been created at … to preserve authentication." That sentence is
      now wrong in two ways: the concept is a provider overlay, and it is no
      longer a HOME. Name the selector and the provider-visible root instead.
- [x] Update the caller at
      `claudine/cli/src/commands/wrap/wrapper_stages.rs:233-240`.
- [x] Rename the perf substage `"shadow home sync"` → `"provider overlay"` and
      its child `"repo root detect"` in `env/mod.rs:391-410` and the docs at
      `env/mod.rs:126-133`, `:211-216`. Check
      `claudine/cli/src/perf/report.rs` and
      `claudine/cli/src/perf/tests/perf_tree.rs` for pinned substage names.
- [x] Audit `claudine/cli/tests/level2_perf_capture.rs` and any golden capture
      that pins the old substage label; update the expectations.
- [x] Confirm every new diagnostic renders through the effective-diagnostic walk
      (`claudine/lib/src/diagnostics/`) rather than a bespoke print, and that no
      message can be read as "your credentials are invalid".
- [x] Verify the sensitive-environment filter is untouched: a missing API-key env
      var and a home-context mismatch must remain distinguishable diagnostics.
      No change to the allow-list is in scope.

**Checkpoint 9.** `just test` passes. Run `claudine codex --repo --perf` against
a fake provider and confirm the substage tree and the info line both read in the
new vocabulary with no "shadow HOME" string anywhere in user-facing output.

---

## Phase 10 — L1 Contract Tests

The spec's numbered list, made concrete. All tests use `CliProcessFixture`
(`claudine/cli/tests/common/mod.rs`) per the L1 spawn contract, hermetic child
fixtures, and platform-neutral path construction. Environment-mutating unit
tests use the existing serialized env guards.

- [x] **‖ parallel** (1)(2) `claudine/cli/tests/level1_provider_overlay_home.rs`
      — for **every** activation reason, the launch baseline's home variables
      reach the child unchanged; and no launch path inserts `/dev/null`, `NUL`,
      or an overlay path into a global home variable.
- [x] **‖ parallel** (3) Explicit provider-root overrides (ambient `CODEX_HOME`,
      `GEMINI_CLI_HOME`, …) are captured as **source** roots, and the Claudine
      selector points at the correct provider-visible overlay shape.
- [x] **‖ parallel** (4) Repo resources remain isolated to each provider's
      previously documented level — assert against the exclusion table in
      `claudine/docs/topics/repo-isolation.md:40-49`, which Phase 12 must keep in
      sync.
- [x] **‖ parallel** (5) Codex prompt overlay still works and SQLite state
      remains outside it. Extend
      `claudine/cli/tests/wrap_basics.rs:192`
      (`codex_wrapper_uses_shadow_home_for_repo_prompt_overlay_without_repo_flag`)
      and rename it to the overlay vocabulary.
- [x] **‖ parallel** (6) Gemini MCP injection writes the location selected by
      `GEMINI_CLI_HOME`, not a `HOME`-derived path.
- [x] **‖ parallel** (7) OpenCode inline MCP injection creates **no** overlay —
      assert the storage root does not exist on disk after the run.
- [x] **‖ parallel** (8) Unsupported (provider, reason) combinations fail before
      the fake provider records a spawn. Drive this from Phase 1's refusal list;
      at minimum `claudine antigravity --repo`.
- [x] **‖ parallel** (9) Materialization failure produces the typed diagnostic
      and never falls back to a null home. Induce failure with an unwritable
      storage root.
- [x] **‖ parallel** (10) Simulated nested `git`, `gpg`, and `gh` stubs on the
      fixture `PATH` observe the original home variables. Use stubs that record
      their environment — **never** invoke a real credential store, and never
      read the developer's real `$HOME`.
- [x] **‖ parallel** (11) Proxy/retry/resume provider transitions remove the
      prior selector, restore an explicit ambient value when one existed, and
      apply only the target provider's plan.
- [x] Cover the edge matrix across the above: absent variables; non-UTF-8 Unix
      values where supported; paths containing spaces; native Windows path forms.
- [x] Add a `SPAWN_ALLOWLIST` entry only if a new test genuinely needs a named
      escape, with the required reasoned comment. Prefer `command()`.
- [x] Prove each new test non-vacuous: confirm it fails against the pre-fix
      behavior (stash the fix commit range or temporarily reintroduce the `HOME`
      write locally — do **not** commit that).

**Checkpoint 10.** `just test` passes on macOS. Every one of the spec's eleven
numbered contracts maps to at least one named test; record the mapping in
`claudine/fixes/2026-09-12-shadow-home/test-map.md`.

---

## Phase 11 — L2 and Cross-Platform Evidence

Required evidence covers macOS, Linux, native Windows, and WSL2. Load the `os`
repo skill before claiming an OS cannot be tested from this host — it is the
authority on which hosts produce which evidence.

- [x] Add `claudine/cli/tests/level2_provider_overlay_capture.rs` driving the
      real `claudine` binary with fake providers. The fixture uses a private home
      and config tree, must **not** open or focus a terminal window, and must not
      read the developer's real credentials.
- [ ] Each environment proves, at minimum: (a) the home-preservation contract and
      (b) one filesystem-backed overlay end to end.
- [ ] Native Windows additionally proves recursive overlay materialization
      without directory hard links — the Phase 5 fix. Watch the known
      `USERPROFILE` known-folder trap and the portable path-spelling rules from
      the `os` skill.
- [x] Keep provider-specific real-CLI coverage in the existing opt-in
      `real-tests` tier (`just test-real`). Do **not** add real-provider tests to
      ordinary L1.
- [ ] Run `just test-l2` locally on macOS. Reuse qualifying passing CI evidence
      for the other three environments where it exists; run what does not.
- [x] Run `just ci-local --plan` before pushing, per repo convention, and review
      the plan rather than assuming scope.

**Checkpoint 11.** `just test-l2` green on macOS. Linux, native Windows, and
WSL2 evidence collected (CI counts as verification). Record the evidence
locations in `test-map.md`. If any environment is red, fix it — a one-environment
red is exactly the signal the `os` skill exists to diagnose.

---

## Phase 12 — Documentation, Skill Mirror, and Acceptance Sweep

Documentation must distinguish four things the old text conflated: provider
overlay, user-home identity, provider authentication preservation, and
environment credential admission.

- [x] **‖ parallel** Rewrite `claudine/docs/topics/repo-isolation.md`. Steps 2,
      3, and 5 of "What `--repo` Actually Does Today" (`:24-28`) describe shadow
      home and `HOME=~/.claudine`; the `HOME=/dev/null` paragraph (`:31`) is
      removed outright. Keep and update the per-provider exclusion table
      (`:40-49`) — Phase 10 test (4) asserts against it. Update the Codex
      special-case section (`:72-88`) to the overlay vocabulary while preserving
      the SQLite contract statement.
- [x] **‖ parallel** Update `claudine/docs/topics/mcp-mode.md` and
      `claudine/docs/topics/mcp-catalog.md` where they describe shadow-HOME
      injection.
- [x] **‖ parallel** Update `claudine/docs/topics/system-prompt.md` and
      `claudine/docs/topics/composition.md` shadow-home references.
- [x] **‖ parallel** Update `claudine/docs/topics/wrapped-execution-switches.md`
      and any CLI reference text describing `--repo` as changing `HOME`.
- [x] **‖ parallel** Add a short "Provider Overlay" section to
      `claudine/docs/topics/provider-metadata.md` documenting the two new facts
      keys and the capability vocabulary, so the next provider onboarding knows
      to fill them.
- [x] **‖ parallel** Update `claudine/docs/topics/how-to-create-a-new-provider.md`
      with the overlay-capability step.
- [x] Mirror into the skill snapshots — `.claude/skills/claudine/SKILL.md`
      (the "MCP Support" paragraph states "Codex/Gemini inject via a shadow HOME
      under `~/.claudine`"), `architecture.md`, `composition.md`,
      `system-prompt.md`, `cli-reference.md`, and `mcp-mode.md`.
- [x] Update `.claude/skills/claudine/timeline.md` with this change.
- [x] Refresh each edited skill document's `hash:` frontmatter with
      `md hash <file>`.
- [x] Check `claudine/docs/dependencies.md` — update only if a crate was added
      or removed (none is expected).
- [x] Walk the spec's Acceptance Criteria list one item at a time and record the
      satisfying evidence (test name, doc section, or file:line) in
      `claudine/fixes/2026-09-12-shadow-home/acceptance.md`. Any criterion
      without concrete evidence is not done.
- [x] Set `implemented: true` in `spec.md` frontmatter and refresh its hash.
- [x] Do **not** move the fix directory to `_completed` — archiving is Ken's
      explicit call.

**Checkpoint 12 (final).** From `claudine/`:

```
just lint
just test
just test-l2
```

All three green. `grep -rni "shadow home\|shadow_home" claudine/docs claudine/cli/src claudine/lib/src .claude/skills/claudine` returns only historical `features/_completed` and `fixes/_completed` references. Real-provider tests remain opt-in behind `just test-real`.

---

## Risk Register

| # | Risk | Phase | Mitigation |
|---|---|---|---|
| R1 | A selector's path shape is inferred wrong, silently reading the user's real config while appearing to isolate | 1 | Shape is observed, recorded in `audit.md` with evidence, and applied in exactly one function (`provider_visible_root`). Phase 10 test (3) asserts the resulting shape. |
| R2 | An additive config dir (OpenCode, Kilo) is granted a `repo_resources` verdict it cannot honor | 1, 2 | Phase 2 adds a generated-metadata invariant: `additive: true` may not carry `NativeRoot` for `repo_resources`. |
| R3 | Threading `EnvBaseline` through `sanitize_process_env` touches signatures broadly and destabilizes unrelated tests | 3 | Keep the existing lossy `environment` map intact; add the raw baseline alongside. Only two production callers change. |
| R4 | The Windows recursive-copy rewrite is only testable on the Windows host and lands late | 5, 11 | Phase 5 adds the unit test immediately and proves it non-vacuous on macOS via a path-shape fixture; Phase 11 collects real Windows evidence. Load the `os` skill first. |
| R5 | Behavior tightening (pre-spawn refusals) surprises existing usage | 1, 6 | The refusal set is fixed and published in `audit.md` in Phase 1, before any code changes, and asserted in Phase 10 test (8). Spec authorizes the tightening explicitly. |
| R6 | New `match Provider` sites regrow while routing per-provider overlay behavior | 2, 4, 6, 8 | Policy lives in generated metadata; side effects live in `WrapperProfile`. The `dispatch_inventory` guard is a checkpoint in Phases 2 and 6 — Phases 6 and 8 each *remove* an existing `matches!(provider, Codex \| Gemini)` site. |
| R7 | A stale comment survives a behavior change | 5, 6, 8, 9 | Named comment-deletion tasks at `exec_prep/mod.rs:123`, `pipeline.rs:543-546`, `env/mod.rs:126-133`, `session_key.rs:13-54`. Repo rule: code is correct, comment is wrong. |

## Out of Scope (do not drift into these)

Changing the sensitive-environment allow-list; guaranteeing credentials are
present or authorized; starting login/key-import/credential-repair flows;
turning `--repo` into a filesystem sandbox; broadening any provider's documented
resource-isolation classes; deleting legacy overlay state; relocating existing
worktrees.
