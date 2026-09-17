---
created: 2026-09-15
area: claudine
spec: ./spec.md
plan: ./plan.md
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
implementation_2: "2026-09-16T15:30:56-07:00"
---

# Implementation Log — Preserve Provider Overlays Without Replacing the User Home

## Phase 1

**Scope: audit and capability ground truth. No production code changed.**

Output is `audit.md` (30 rows: 10 providers × 3 activation reasons) plus one
research-document correction.

### Test design requirements

This phase changes no behavior, so the "map every changed behavior to a
concrete test" requirement maps to an empty set. The audit's *purpose* is to
produce the input for the tests Phases 10 and 11 write, and the refusal list is
published here specifically so those phases test the same set.

One requirement did apply and was met: plan → Phase 1 requires that selector
shape be **observed, not inferred**. Where the repo's evidence was ambiguous, it
was resolved against a running binary rather than by reading — see the probe
below.

### The probe

`docs/research/mcp/codex.md`'s `runtime_injection` prose said `CODEX_HOME`
should point at "a temporary directory containing a generated
`.codex/config.toml`" — i.e. `ParentOfProviderDir` shape. The same document's
`config_files` records, and four other research documents, said
`$CODEX_HOME/config.toml` — i.e. `ProviderDir`. That is exactly the
backwards-shape hazard plan → Phase 1 names as the highest-risk unknown, so it
was settled empirically.

Setup: scratch `HOME` under `/tmp`, a `.codex/config.toml` and `.gemini/settings.json`
in it, a minimal MCP catalog at `<home>/.claudine/mcp/catalog.json`, fake `codex`
and `gemini` stubs on `PATH`, `env -i` with only `HOME`, `PATH`, `NO_COLOR=1`,
`CLAUDINE_RENDEZVOUS_REPORT=false`, `PLAYA_DRY_RUN=1`, and a private
`PLAYA_SPOOL_DIR`. No real credentials were read and no terminal window was
opened.

Running `claudine codex --use probe -p hi` and `claudine gemini --use probe -p hi`,
Claudine's own pre-flight MCP report printed:

```
• files=<scratch-home>/.claudine/.codex/.codex/config.toml
• files=<scratch-home>/.claudine/.gemini/.gemini/settings.json
```

Both doubly nested. This is audit finding **F1**: runtime MCP injection writes
one directory level below where the provider reads, so `claudine codex --mcp`
and `claudine gemini --mcp` have been silently running against the user's
unmodified configuration. The injected file is also deleted by the normal
temp-file cleanup after the run, which is why the defect leaves no trace on
disk and why no existing test caught it — the injector's own unit tests call
`inject` with a `TempDir` root, encoding the intended contract rather than the
value production passes (`inject.rs:562-580,601-625`), and no L1 test drives
runtime MCP injection end to end through the wrapper.

Code path, confirmed by reading after the fact: `repo_home.rs:25` makes the
storage root `~/.claudine/<agent_offset>`; `repo_home.rs:322` and
`env/mod.rs:314` put that value — the agent directory itself — on
`EnvPlan::shadow_home_path`; `repo_home.rs:309` sets the child's `HOME` to its
*parent*; then `wrapper_mcp.rs:204` hands `shadow_home_path` to
`McpInjector::inject`, which joins the offset a second time at `inject.rs:155`
and `inject.rs:293`.

Phase 7's `shadow_home: Option<&Path>` → `config_root: Option<&Path>` change is
the fix. Phase 10 tests (5) and (6) will be non-vacuous by construction, since
they fail against today's behavior.

### Evidence read before probing (plan task 2)

`docs/providers/facts/*.yaml` (all ten), and the `mcp/`, `system-prompt/`,
`agent-cli/`, `subagents/`, `plugins/`, `agent-logging/`,
`non-interactive-sessions/`, `skills/`, `resume/`, and `agent-permissions/`
research fleets. Twenty-six of the thirty rows were answerable from the repo
without probing. The `original_home()` `CODEX_HOME` special case
(`repo_home.rs:34-42`) and `codex_sqlite_home()` (`repo_home.rs:219-238`) were
read and corroborate the `ProviderDir` verdict — neither joins anything onto
the variable's value.

### Shape results

| Selector | Shape | Note |
|---|---|---|
| `CLAUDE_CONFIG_DIR` | `ProviderDir` | Relocates the `.claude` tree; not `~/.claude.json` |
| `CODEX_HOME` | `ProviderDir` | `$CODEX_HOME/config.toml` |
| `GEMINI_CLI_HOME` | **`ParentOfProviderDir`** | "creates or uses a `.gemini` folder inside it" |
| `GOOSE_PATH_ROOT` | `ParentOfProviderDir` | Child segments are `config/`, `data/`, `state/`, `.agents/` — not `.goose` |
| `KIMI_CODE_HOME` | `ProviderDir` | Default `~/.kimi-code` |
| `OPENCODE_CONFIG_DIR` | `ProviderDir`, **additive** | Cannot mask user resources |
| `KILO_CONFIG_DIR` | `ProviderDir`, **additive** | Same |
| `QWEN_HOME` | `ProviderDir` | `$QWEN_HOME/agents/` |
| `PI_CODING_AGENT_DIR` | `ProviderDir` | Replaces `~/.pi/agent` |
| Antigravity | `None` | No provider-scoped selector exists |

`GEMINI_CLI_HOME` is the fleet's only true `ParentOfProviderDir`. Usefully, the
existing on-disk layout already satisfies both shapes: `CODEX_HOME=~/.claudine/.codex`
and `GEMINI_CLI_HOME=~/.claudine` both resolve to the directory Claudine already
materializes, so **no migration is needed** (audit F2).

### Verdicts and the refusal list

Three pairs become pre-spawn refusals, all reached only via `--repo`:
Antigravity, OpenCode, and Kilo `repo_resources`. Every other `Unsupported`
cell is inert — the reason is never raised for that provider — so no currently
working command begins to fail. OpenCode and Kilo keep working `--mcp`/`--use`
because their MCP path is `ComposableInjection` and carries no overlay.

### Secondary findings

- **F3** — `agent_offset` is not the provider's config root for Goose,
  OpenCode, Kilo, Kimi, Pi, or Antigravity. Today's `--repo` overlay for those
  providers mirrors the wrong directory and points the provider at an empty
  relocated root: the launch is not isolated, it is blank. Phase 4's source-root
  resolution needs a new provider-owned fact.
- **F4** — no `OPENCODE_DISABLE_*` or `KILO_DISABLE_*` variable closes the
  global user config directory, confirming the spec's prediction that additive
  directories cannot claim `repo_resources`.
- **F5/D4** — `codex_sqlite_home()` is unaffected by the shape finding, but once
  Phase 4 writes `CODEX_HOME` into the child env it must resolve from the
  Phase 3 `EnvBaseline` rather than the assembled child env.
- **F6** — `~/.claude.json` does not follow `CLAUDE_CONFIG_DIR`. Once `HOME`
  stops moving, Claude reads it in place, which is strictly better under
  Invariant 6. `repo_home_root_files` (Claude is its only user) becomes dead;
  Phases 5/6 should confirm rather than port it forward.

### Research correction

`docs/research/mcp/codex.md` → `runtime_injection.mechanism` said `CODEX_HOME`
takes "a temporary directory containing a generated `.codex/config.toml`". That
contradicts the same document's `config_files` records and four other research
documents, and is the likely origin of F1. Corrected to state that `CODEX_HOME`
names the Codex directory itself, with a pointer to the `config_files` records.
`last_updated` bumped to 2026-09-15. No facts YAML and no generated `data.rs`
was touched — the two new facts keys are Phase 2's work, and this audit is
their input.

### Gates run

`just lint` from the `claudine/` package area — clean, as expected for a phase
that changed only Markdown. `just test` was not run: no code changed, and plan →
Checkpoint 1 asks only for `lint`.

### Raised for human review

Two items, recorded in `plan.md` frontmatter:

1. **D3** — whether Phase 2 lands an OS-aware `source_root` fact (keeping
   Goose/Kimi/Pi at `NativeRoot`) or downgrades Goose to `Unsupported`. The
   second option adds a fourth entry to the refusal list, which risk R5 exists
   to keep fixed before code changes begin.
2. **F1** — the live MCP no-op. Phase 7 fixes it incidentally, but it affects
   shipped behavior today and may warrant an earlier standalone fix.

## Phase 2

**Scope: provider metadata — facts → codegen → `ProviderInfo`.** The audit's
verdicts become generated, drift-checked data so the Phase 4 planner is
table-driven and no new `match Provider` site is needed. No runtime behavior
changed: nothing reads the new fields yet.

### The decision Phase 2 could not defer

Audit D3 said the source-root question is "the one decision Phase 2 cannot make
silently". This session is non-interactive, so it was made explicitly and
written into `audit.md` rather than left open.

Reading the Goose evidence closely changed the shape of the choice D3
anticipated. D3 framed it as "land an OS-aware `source_root` fact, or defer
OS-dependent roots and downgrade Goose". But a per-OS record would not have
helped: Goose has no single pre-overlay root on *any* OS. `GOOSE_PATH_ROOT`
collapses `config/`, `data/`, and `state/` under one root; unset, those are
three separate directories (`~/.config/goose`, `~/.local/share/goose`,
`~/.local/state/goose` on Linux;
`~/Library/Application Support/Block/goose/{config,data,state}` on macOS).
Building that overlay needs a Goose `overlay_strategy` in the wrapper profile,
which plan → Phase 4 explicitly withholds ("Leave the other eight profiles on
the default").

So: `source_root` landed as a fact, Kimi (`~/.kimi-code`) and Pi
(`~/.pi/agent`) cleared their `‡` markers and keep `NativeRoot`, and **Goose
`repo_resources` is downgraded to `Unsupported`**, joining the refusal list as
row 4.

Risk R5 wanted the refusal list fixed before code changes. It grew by one, and
that is recorded rather than absorbed. The mitigating fact is that
`claudine goose --repo` is not a working command today: audit F3 established
that it mirrors `~/.goose`, which is not Goose's config root, and points Goose
at an empty relocated tree. The launch is blank, not isolated. A visible
pre-spawn refusal replaces a silent one. Raised for human review regardless.

The constraint is now structural rather than a matter of judgment — the lib
invariant `repo_resource_isolation_requires_a_single_source_root` fails any
`NativeRoot` `repo_resources` verdict whose selector has no `source_root`.

### Design decisions worth carrying forward

**The vocabulary went to `claudine-catalog-types`, including `OverlayReason`.**
The plan asked for `OverlayCapability` and `OverlaySelectorShape` there.
`OverlayReason` joined them because the `overlay_capabilities` facts record is
*keyed* by its members, and the generator validates the required keys against
`OverlayReason::VARIANTS` — adding a reason therefore makes the facts key
mandatory instead of silently defaulting to a permissive verdict. Phase 4 must
re-export it from `provider_overlay/plan.rs`, not redefine it (the
`resume_support.rs` re-export is the precedent).

**`relocates` is a typed enum, not the string array the plan text names.** The
YAML authoring form is still a string array; the Rust type is
`&'static [OverlayResourceClass]` over the five classes the plan's own Phase 1
column vocabulary defines (config / auth / sessions / cache / state). Phase 5
classifies overlay entries by resource class, and string-matching there is the
kind of debt this repo's conventions ask to avoid. Provider-specific resource
kinds (agents, commands, modes, skills, plugins, prompts) fold into `Config`
because they live under the provider's config tree; `State` is the class that
must never be mirrored with per-file links.

**The catalog type is `OverlaySelectorSpec`.** The plan's naming table reserves
`OverlaySelector` for Phase 4's *runtime* type, which is the spec plus a
resolved value plus `provider_visible_root`. Two different things needed two
names; the catalog half took the `Spec` suffix so the plan's name stays free.

**`source_root` is `Option<PathTemplate>`, enforced home-relative at
generation.** Reusing the existing `PathTemplate` gives Phase 4
`resolve_with_home` for free. The `~/` requirement is a loud generation error,
not a convention: an absolute or drive-qualified root would not be portable
across the four environments this repo must support.

**Fields were appended at the end of `ProviderInfo`** (orders 47, 48) so no
existing field renumbered — every `data.rs` diff is purely additive.

### Facts values, and where each came from

All ten providers, from `audit.md`'s full matrix. Selector spellings, shapes,
and relocated classes are pinned by tests so a typo cannot ship quietly.

| Provider | Selector | Shape | `additive` | `source_root` |
|---|---|---|---|---|
| Claude | `CLAUDE_CONFIG_DIR` | `provider_dir` | no | `~/.claude` |
| Codex | `CODEX_HOME` | `provider_dir` | no | `~/.codex` |
| Gemini | `GEMINI_CLI_HOME` | `parent_of_provider_dir{.gemini}` | no | `~/.gemini` |
| Goose | `GOOSE_PATH_ROOT` | `parent_of_provider_dir{config}` | no | *null* |
| Kilo | `KILO_CONFIG_DIR` | `provider_dir` | **yes** | *null* |
| Kimi | `KIMI_CODE_HOME` | `provider_dir` | no | `~/.kimi-code` |
| OpenCode | `OPENCODE_CONFIG_DIR` | `provider_dir` | **yes** | *null* |
| Pi | `PI_CODING_AGENT_DIR` | `provider_dir` | no | `~/.pi/agent` |
| Qwen | `QWEN_HOME` | `provider_dir` | no | `~/.qwen` |
| Antigravity | — | — | — | — |

Claude's `relocates` deliberately excludes `auth`: `CLAUDE_CONFIG_DIR` does not
move the macOS Keychain account, and leaving credential lookup on the real user
identity is exactly the outcome Invariant 4 wants.

OpenCode and Kilo record their *additive* directory selectors even though both
are `Unsupported` for `repo_resources` — that is what gives the
`additive: true` invariant a live negative case, per audit D5. Their MCP path
stays `composable_injection` and acquires no storage root.

### Requirement → test mapping

Phase 2 changes no user-visible behavior, so the mapping is
"generated data and its shape gates", not "launch outcomes". Codegen is a
parser/schema change, so the corpus and malformed-input requirements apply in
full.

| Behavior / requirement | Test | Level |
|---|---|---|
| Facts → `data.rs` for the real shipped corpus (all 10 providers, both keys) | `claudine-gen::drift committed_data_matches_regenerated_inputs` | passive corpus, pre-existing, now covers the new fields |
| Committed generated bytes are pinned | `claudine-gen::drift committed_generated_artifacts_match_phase_1_byte_baseline` (11 hashes refreshed) | passive corpus |
| Registry covers every serialized field, in order | `claudine-gen::registry_coverage` (3 tests, field list +2) | unit |
| Registry source counts | `claudine_gen::registry::tests::registry_matches_matrix_source_counts` (22→24 facts, 43→45 total) | unit |
| `ProviderDir` selector round-trips through the real claude facts | `claudine-gen::pipeline overlay_facts_emit_the_selector_and_capability_records` | end-to-end through `generate_for_area` |
| `ParentOfProviderDir` carries its child segment | `…parent_of_provider_dir_shape_carries_the_child_segment` | end-to-end |
| Payload-less parent shape (the F1-shaped authoring slip) | `…parent_of_provider_dir_without_a_child_is_rejected` | negative |
| Multi-segment child | `…multi_segment_child_is_rejected` | negative |
| Unknown shape member | `…unknown_selector_shape_is_rejected` | negative |
| Absolute (non-portable) source root | `…absolute_source_root_is_rejected` | negative |
| Null source root (Goose / OpenCode / Kilo shape) | `…null_source_root_emits_none` | representation variant |
| Null selector (Antigravity shape) | `…null_overlay_selector_emits_none` | representation variant |
| Unknown resource class | `…unknown_relocated_resource_class_is_rejected` | negative |
| Unknown capability verdict | `…unknown_capability_verdict_is_rejected` | negative |
| Missing reason key must not default | `…missing_capability_reason_is_rejected` | negative |
| Absent capability record | `…absent_overlay_capabilities_is_rejected` | negative |
| Wire forms / externally tagged shape | `claudine-catalog-types provider_overlay::tests` (3 tests) | unit |
| `NativeRoot` requires a selector | `claudine provider::tests::native_root_verdicts_require_a_selector` | unit over shipped catalog |
| `additive` forbids `repo_resources` isolation (R2, audit D5) | `…additive_selectors_cannot_claim_repo_resource_isolation` | unit, anti-vacuity guarded |
| `NativeRoot` `repo_resources` requires a source root (D3) | `…repo_resource_isolation_requires_a_single_source_root` | unit, anti-vacuity guarded |
| `Unsupported` legal with and without a selector | `…unsupported_verdicts_are_legal_with_and_without_a_selector` | unit |
| Shape / source-root spelling well-formedness | `…selector_shapes_and_source_roots_are_well_formed` | unit, anti-vacuity guarded |
| The published verdict matrix (Phase 6 / Phase 10 authority) | `…overlay_capability_matrix_matches_the_audit` | unit, all 30 cells |
| Selector env-var spellings | `…recorded_selectors_match_the_audit` | unit, all 10 |
| Source root is not `agent_offset` (F3) | `…source_roots_are_independent_of_the_agent_offset` | unit |
| `Provider` accessors read generated data | `…provider_overlay_accessors_read_generated_metadata` | unit, all 10 × 3 |
| Relocated classes (Gemini sidecars, Claude Keychain exclusion) | `…relocated_resource_classes_match_the_audit` | unit |
| No new provider-dispatch site | `claudine-cli::dispatch_inventory` (both tests) | repo guard |

Round-trip: `generate → check` reconvergence is covered by the existing
`claudine-gen::generate_ux yes_writes_all_drift_and_check_reconverges`, which
now exercises the new fields, and by regenerating twice locally to confirm the
second run reported no drift.

### Non-vacuity, proved rather than asserted

Three of the invariants were run against deliberately corrupted facts before
being accepted:

- Goose `repo_resources: native_root` → both
  `repo_resource_isolation_requires_a_single_source_root` and
  `overlay_capability_matrix_matches_the_audit` fail, each naming Goose.
- OpenCode `repo_resources: native_root` → both
  `additive_selectors_cannot_claim_repo_resource_isolation` (naming
  `OPENCODE_CONFIG_DIR`) and
  `unsupported_verdicts_are_legal_with_and_without_a_selector` fail.

Both facts files were restored and everything regenerated; the working tree
diff for `docs/providers/facts/` and `lib/src/provider/` is additive only.

The remaining invariants carry in-test anti-vacuity guards (`additive_seen`,
`without_source_root > 0`, `parent_shapes > 0`) so they cannot pass by finding
nothing to check. The gen-side negative tests are non-vacuous by construction —
each asserts a specific `GenError` variant and field.

### One guard that needed handling

`claudine-cli::dispatch_inventory` counts every place code names a provider,
split into `reference` (harmless) and `conditional` (governed, burn-down
tracked). The invariant tables were first written as
`[(Provider::Claude, …), …]` arrays, which the classifier reads as the
`tuple-array` form and counts as **conditional** — three new governed sites in
a phase whose checkpoint exists to prove no new dispatch appeared.

Rewritten to key on the facts-file slug and resolve through a local
`provider_by_slug` helper. `conditional` stays at 61 and `match-provider` at 7;
only `direct-ref`/`reference` grew (1453 → 1457), from four ordinary
`provider_info(Provider::X)` lookups of the same kind the file already
contains. The tables are also now keyed the way their source of truth is, so a
row and its `docs/providers/facts/<slug>.yaml` read side by side.

The committed inventory was regenerated with `CLAUDINE_UPDATE_INVENTORY=1`.
The JSON diff shows several apparent line-number changes in files this phase
did not touch (`cli/src/commands/exec_prep/mod.rs`,
`lib/src/dispatch/runner/tests.rs`); those are diff misalignment from inserting
records into a sorted array, not drift. Verified by removing the new tests and
confirming the *committed* inventory still passes against unmodified sources.

### Gates run

From the `claudine/` package area:

- `just test` — **7046 passed, 9 skipped, 0 failed** (includes `test-gen`,
  `test-library`, `test-cli`, the drift suite, and `dispatch_inventory`).
- `just lint` — clean.
- `cargo clippy -p claudine -p claudine-gen -p claudine-catalog-types
  --all-targets` — clean (the package `lint` recipe does not build test
  targets; run separately so the new test code is linted too).

No test was skipped or left failing. No pre-existing failure was encountered.
`just test-l2` was not run: Phase 2 changes no runtime behavior and plan →
Checkpoint 2 does not ask for it.

### Not done, deliberately

- `docs/topics/provider-metadata.md` and
  `docs/topics/how-to-create-a-new-provider.md` are **not** updated here — plan
  → Phase 12 owns both, by name. Nothing in the existing text drifted (checked:
  no field-count or facts-key inventory appears in `docs/` or the skill
  snapshots).
- `repo_home_root_files` was left in place. Audit F6 predicts it goes dead once
  `HOME` stops moving, but confirming that is Phase 5/6's task.
- `claudine/gen/src/scaffold.rs` needed no change: the facts skeleton is derived
  from `REGISTRY`, so new providers now scaffold both keys automatically.

## Phase 3

**Scope: capture one immutable launch baseline.** `InvocationContext` now owns
the launch home and the launch environment in raw `OsString` form, and the
child-environment assembly reads that snapshot instead of the wrapper's own
process. No user-visible behavior changed: `HOME` is still overwritten
downstream by the Phase 5/6 code this baseline exists to replace.

### What landed

`HomeBaseline` and `EnvBaseline` live in `claudine/lib/src/invocation_context.rs`
beside the existing lossy `environment` map, which is untouched and still has
every one of its consumers.

- **`HOME_VARIABLES` is all four names on every platform** — `HOME`,
  `USERPROFILE`, `HOMEDRIVE`, `HOMEPATH` — not `HOME` on Unix and the Windows
  three on Windows. The plan's wording allowed either, and a per-OS split would
  have been wrong twice: an MSYS, Git-Bash, or WSL-interop launch carries both
  sets at once, and Phase 6's Invariant-1 guard has to refuse a sentinel written
  into *any* of the four regardless of which host it runs on. Recording all four
  everywhere means no consumer has to decide which names its host "should" have
  had.
- **Absence is `Option::None` and an empty value is `Some("")`.** Both states
  are captured distinctly, and the resolved home is kept separately from the
  variables, because `biscuit_file::home_dir()` (→ `dirs::home_dir()`) resolves
  from `$HOME` on Unix but from the Windows known-folder API — not
  `%USERPROFILE%` — on native Windows. Conflating the two would have made the
  baseline's meaning platform-dependent.
- **`env_names_match` compares case-insensitively on Windows only.** Windows
  resolves environment-variable names case-insensitively, so a byte-equality
  lookup would miss a `UserProfile`/`USERPROFILE` spelling the OS itself treats
  as one variable. Used by both `HomeBaseline::variable` and `EnvBaseline::get`.
- **The lossy map is now derived from the raw baseline** rather than captured
  by a second `std::env::vars()` call. This was not cosmetic:
  `std::env::vars()` **panics** on a non-UTF-8 variable, so before this change
  a user whose environment carried one crashed every `InvocationContext`
  capture. Deriving with `filter_map` skips what has no lossless `String` form
  and guarantees the two views cannot disagree about a variable that moved
  between two reads.
- `InvocationInner::home_dir` was **replaced** by `home_baseline` rather than
  sitting beside it, so the resolved home has exactly one owner;
  `InvocationContext::home_dir()` keeps its signature and its behavior.

On the CLI side, `sanitize_process_env` takes the `EnvBaseline` as its first
argument and iterates it instead of `std::env::vars_os()`. The baseline is
threaded through `build_child_env_with_launch` and `build_child_env`, and both
production callers pass the invocation's snapshot:
`wrap/mod.rs` (`invocation.env_baseline()`) and `composition/pipeline.rs`.

The composition pipeline's `request.invocation_context` is an `Option`, so that
caller falls back to `EnvBaseline::capture()` when no invocation owner exists —
which is the same environment an owner would have recorded, and is exactly the
pre-existing behavior for that path.

### Requirement → test mapping

The changed behavior is internal plumbing that crosses one crate boundary
(`claudine` → `claudine-cli`) and no CLI, filesystem, terminal, or persistence
boundary — `just test`'s L1 output is byte-identical before and after. Per
`rust-testing`'s level rules that puts the verification at unit level in both
crates; there is no observable CLI surface for an L1 binary test to assert yet.
Phase 10 owns the end-to-end contracts, and it can only write them after Phase 6
stops moving `HOME`.

| Behavior / requirement | Test | Level |
|---|---|---|
| A post-capture `HOME` mutation does not move the baseline (all four variables, plus the resolved home and the lossy map) | `claudine invocation_context::tests::home_and_env_baselines_do_not_follow_a_post_capture_mutation` | unit, anti-vacuity guarded |
| An absent `HOME` is captured as absent and is **not** synthesized from `USERPROFILE` | `…an_absent_home_is_captured_as_absent_and_is_not_synthesized` | unit, negative |
| An empty `HOME` is a present empty value, not an absent one | `…an_empty_home_is_captured_as_a_present_empty_value` | unit, representation variant (Unix only — Windows deletes a variable assigned `""`) |
| A non-UTF-8 `HOME` round-trips through the raw baseline while the lossy map drops it | `…a_non_utf8_home_survives_the_raw_baseline_round_trip` | unit, representation variant |
| **Checkpoint 3:** `sanitize_process_env` ignores a post-capture ambient mutation | `claudine-cli commands::wrap::env::tests::sanitize_process_env_reads_the_supplied_baseline_not_the_ambient_process` | unit, anti-vacuity guarded |
| Sanitation verdicts are computed over the supplied snapshot: strip, `--include`, profile allow-list, and the "requested but not set" warning | `…sanitize_process_env_classifies_the_baselines_keys` | unit, all four return values asserted |
| A non-UTF-8 value reaches the child as its original bytes | `…sanitize_process_env_preserves_non_utf8_values` | unit, representation variant |
| Downstream state: the assembled `EnvPlan` inherits the baseline and not the ambient process | `…build_child_env_inherits_the_supplied_baseline` | unit, end of the changed path |
| The resolved home and file-resolution context stay pinned to the capture | `…captured_process_state_is_immutable_for_later_projections` | unit, pre-existing, still green |
| No new provider-dispatch site | `claudine-cli::dispatch_inventory` (both tests) | repo guard |

### Non-vacuity, proved rather than asserted

- Reverting the lossy map to `std::env::vars().collect()` makes
  `a_non_utf8_home_survives_the_raw_baseline_round_trip` **panic** inside
  `std::env`, naming the offending value. Restored and re-run green.
- Reverting `sanitize_process_env` to `std::env::vars_os()` fails **both**
  `sanitize_process_env_reads_the_supplied_baseline_not_the_ambient_process`
  ("the post-capture value leaked into the child environment") and
  `build_child_env_inherits_the_supplied_baseline`. Restored and re-run green.
- The two ambient-mutation tests carry in-test anti-vacuity assertions
  (`std::env::var_os("HOME")` really moved; a second `EnvBaseline::capture()`
  really sees `mutated`/`late`), so neither can pass because the mutation
  silently failed.

### Gates run

From the `claudine/` package area, on macOS:

- `just test` — **7054 passed, 9 skipped, 0 failed.**
- `just lint` — clean.
- `cargo clippy -p claudine -p claudine-cli --all-targets` — clean (the package
  `lint` recipe does not build test targets, so the new test code is linted
  separately, as in Phase 2).

No test was skipped or left failing, and no pre-existing failure was
encountered. `just test-l2` was not run: plan → Checkpoint 3 does not ask for
it and no L2-observable behavior changed.

One gate needed handling. `dispatch_inventory` failed on first run with
`sites: 1518 → 1520`: the new `build_child_env_inherits_the_supplied_baseline`
test names `Provider::Claude` twice, in the same plain-argument form as the
tests already in that file. Regenerated with `CLAUDINE_UPDATE_INVENTORY=1` and
verified the classification rather than the count — `conditional` stays at
**61** and `match-provider` at **7**; only `direct-ref`/`reference` grew
(1457 → 1459). No new dispatch appeared.

### Cross-OS evidence

The only platform-conditional code added is `env_names_match`'s `#[cfg(windows)]`
arm, which no macOS build ever compiles. Verified on the native Windows rig:

- `just cross-check claudine --os windows baseline …` — compiled `claudine` on
  `build-win-native` and ran `home_and_env_baselines_do_not_follow_a_post_capture_mutation` → **PASS**.
- `just cross-check claudine --os windows an_absent_home …` → **PASS**.

Two notes for whoever runs this next. `just cross-check` interpolates its
arguments into a shell unquoted, so a nextest **filterset** (`-E 'test(x)'`)
dies on the parentheses; pass plain positional substring filters instead. And
the script runs `cargo nextest run -p <package>`, so `cross-check claudine`
reaches the **library** only — the `claudine-cli` unit tests need
`cross-check claudine-cli`. That second run was not made: the CLI half of this
phase contains no `cfg`, no path handling, and no new API — it threads one
reference through three signatures — so the Windows risk was confined to the
library arm that is now verified. Phase 11 owns the full four-environment
sweep.

### Not done, deliberately

- **`ambient_sensitive_env()` still reads `std::env::vars_os()`.** The plan
  names `sanitize_process_env` alone, and this function is the invocation-*neutral*
  credential snapshot a per-attempt rebuild re-sanitizes for a different
  provider — a different contract from the child's inherited environment. It is
  captured at the top of the composition pipeline, before any wrapper stage
  mutates anything, so today it observes the same values. Worth converting when
  Phase 8 touches the rebuild path, so the invocation has exactly one
  environment authority.
- **The `sanitize_env_for_test` / `sanitize_env_for_test_with_auto` helpers in
  `env/tests.rs` were left alone.** They *re-implement* the sanitizer rather
  than calling it — which was previously unavoidable, because
  `sanitize_process_env` read the ambient process and could not be given an
  input. That is no longer true, and the four tests built on those helpers
  could now drive the real function. Rewriting them is a test-only cleanup with
  no bearing on this fix, so it was left out of a behavior-change diff; the new
  tests above call the real function directly.
- The `"shadow home sync"` perf substage names and the `EnvPlan::shadow_home_path`
  field keep their old vocabulary. Phases 6 and 9 own those renames by name.

## Phase 4

**Scope: the typed overlay plan and the profile seam.** `claudine::provider_overlay`
now answers, as one value, every question the wrapper answers today with two
booleans and a bare `Option<PathBuf>`. Nothing calls it: the planner compiles,
is unit-tested end to end, and is wired to no launch path. `HOME` is still
overwritten downstream by the Phase 5/6 code this plan exists to replace.

### What landed

`claudine/lib/src/provider_overlay/` — `mod.rs`, `selector.rs`, `plan.rs`,
`tests.rs`.

- **`OverlayReasons`** is a `Copy` three-bit set over `OverlayReason`, with
  `OVERLAY_REASONS` as the public "every reason" array. A launch carries several
  reasons at once (`--repo` plus `--mcp`), and a refusal has to be able to name
  exactly one of them.
- **`OverlayPlan`** carries the reason set, the provider, the pre-overlay source
  root (plus whether the user named it explicitly), the storage root, the
  provider-visible root, the selector, the per-reason verdicts, the
  materialization list, the pinned external-state entries, and the `--repo`
  exclusion set. `env_patch()` is the derived child patch; `excluded_resources()`
  is the derived omission set.
- **`OverlaySelector` / `OverlayStorage`** in `selector.rs`, with the shape
  distinction expressed in exactly two adjacent functions — see the next
  section, which is the one design call worth reading.
- **`OverlayPlanner`** reads `Provider::overlay_capability` and
  `Provider::overlay_selector` from Phase 2's generated metadata. It introduces
  no `match Provider`: `dispatch_inventory` `conditional` stayed at **61** and
  `match-provider` at **7**.
- **`ClaudineError::ProviderOverlayUnsupported` / `::ProviderOverlayFailed`**,
  mapped to two new locked catalog codes, `provider.overlay_unsupported`
  (unrecoverable/caller — nothing generic resolves it, only dropping the mode
  does) and `provider.overlay_failed` (correctable/environment, carrying the
  `io::Error` as a typed `#[source]`).

### The one design call worth reading: which root is "storage"

Plan → Phase 4 describes `provider_visible_root(storage_root)` as applying the
shape, and in the same sentence says `ParentOfProviderDir` "yields its parent" —
which reads backwards against the function's own name. The audit's F2 table adds
a third reading by listing Gemini's storage root as `~/.claudine/.gemini`.

Settled as: **the storage root is the selector's value**, and
`provider_visible_root(shape, storage_root)` is the forward function — identity
for `ProviderDir`, `join(child)` for `ParentOfProviderDir`. That makes the
selector value unambiguous (it is always the storage root), gives Phase 7 the
`config_root` it needs from one call, and keeps the shape in one expression.

The inverse — choosing the storage root so the *visible* root keeps its
historical spelling — is `OverlayStorage::new`, deliberately placed in the same
module, three lines away, with the pairing stated in both doc comments. The
result reproduces audit F2's on-disk layout exactly, so **no migration is
needed**:

| Provider | Storage root (selector value) | Provider-visible root |
|---|---|---|
| Codex | `~/.claudine/.codex` | `~/.claudine/.codex` |
| Gemini | `~/.claudine` | `~/.claudine/.gemini` |

`overlay_storage_keeps_the_existing_on_disk_layout_under_both_shapes` pins it.

### Three deviations from the plan's literal text

All three are recorded rather than absorbed.

1. **`OverlayPlanner::new(&HomeBaseline, &EnvBaseline)` + `plan(provider,
   reasons)`**, not the plan's `plan(provider, reasons, source_root, baseline)`.
   The source root is *resolved* by the planner from the baseline and the
   generated `source_root` fact; no caller in Phases 6 or 8 has a source root to
   supply, so the fourth parameter would have been an input nobody has. The
   planner needs both the home and the environment baseline — `resolved()` is
   not `variable("HOME")`, per Phase 3's note — so the pair became the
   planner's state rather than two more arguments on every call.
2. **`WrapperProfile::overlay_strategy(&self, plan: &mut OverlayPlan, env:
   &EnvBaseline)`** takes a second parameter the plan text does not name. Audit
   D4 requires `CODEX_SQLITE_HOME` to resolve from the Phase 3 `EnvBaseline` and
   never from the assembled child environment — otherwise, once `CODEX_HOME`
   names the overlay, the SQLite home recurses into it and breaches Invariant 6.
   The profile cannot honor that without the baseline.
3. **The `--repo` exclusion table moved to the library** as
   `provider_overlay::repo_isolated_resources`, and
   `repo_home.rs::excluded_dirs` now delegates to it. Phase 5 reads the set off
   the plan, so the alternative was two copies of the same policy for one
   phase — a divergence neither side's tests would have caught. The non-`--repo`
   branch (Codex `prompts`) stayed in the CLI: it is an artifact of prompt
   materialization, not an isolation rule.

One addition: **`HomeBaseline::from_parts`**, the twin of the existing
`EnvBaseline::from_entries`. Without it there is no way to plan against a stated
home, so every planner test would have had to mutate the process environment
under a serial guard to test a pure function.

### Requirement → test mapping

Phase 4 changes no user-visible behavior — `just test`'s L1 surface is identical
before and after — so per `rust-testing`'s level rules the verification is unit
level in both crates. The one changed *production* behavior is the exclusion
table's new owner, and it is covered on both sides of the move.

| Behavior / requirement | Test | Level |
|---|---|---|
| Shape application, all three shapes | `claudine provider_overlay::tests::provider_visible_root_applies_the_selector_shape` | unit |
| Both shapes keep the existing on-disk layout (audit F2, no migration) | `…overlay_storage_keeps_the_existing_on_disk_layout_under_both_shapes` | unit |
| An inline selector has no storage at all | `…an_inline_selector_has_no_storage` | unit, negative |
| Reason set semantics: idempotent insert, membership, order, `FromIterator` | `…overlay_reasons_behave_as_a_set` | unit |
| A `ProviderDir` plan: selector, value, visible root, source root, verdicts, env patch, exclusions | `…a_codex_repo_plan_points_the_selector_at_the_overlay` | unit, downstream outputs asserted |
| A `ParentOfProviderDir` plan offsets the value by the child segment | `…a_gemini_plan_offsets_the_selector_value_by_the_child_segment` | unit |
| **Invariant 1 at the planning layer**, swept over all 10 × 3 cells | `…no_plan_patches_a_home_variable` | unit, anti-vacuity guarded |
| Every requested reason records its verdict | `…a_multi_reason_plan_records_every_requested_verdict` | unit |
| An empty reason set plans no overlay | `…an_empty_reason_set_plans_no_overlay` | unit, boundary |
| **An explicit source root is carried as source, never as destination** | `…an_explicit_selector_value_is_the_source_and_never_the_destination` | unit |
| An explicit parent-shaped value resolves through its child segment | `…an_explicit_parent_shaped_value_resolves_through_its_child_segment` | unit |
| Present-but-empty selector value falls back to the documented default | `…an_empty_selector_value_falls_back_to_the_documented_default` | unit, representation variant |
| Absent selector value uses the documented default | `…a_codex_repo_plan_points_the_selector_at_the_overlay` | unit, representation variant |
| Another provider's selector does not move this provider's source root | `…a_foreign_selector_value_does_not_move_another_providers_source_root` | unit, negative |
| No resolvable home ⇒ refusal, not a guessed path | `…a_host_without_a_resolvable_home_cannot_plan_a_filesystem_overlay` | unit, negative |
| **`Unsupported` refuses instead of weakening the plan** — all four published refusal pairs | `…every_published_refusal_pair_refuses_before_a_plan_exists` | unit, negative |
| **A multi-reason request refuses only the unsupported reason**, leaving the launch available | `…a_multi_reason_request_refuses_only_the_unsupported_reason` | unit |
| OpenCode/Kilo MCP acquires no storage root, no selector, no env patch | `…an_inline_injection_plan_acquires_no_storage_root` | unit |
| A materialized entry is excluded from the mirror | `…a_materialized_entry_is_excluded_from_the_source_mirror` | unit |
| A nested destination does not hide its ancestor | `…a_nested_materialization_does_not_exclude_its_ancestor` | unit, boundary |
| Pinned external state joins the env patch after the selector | `…pinned_external_state_joins_the_env_patch_after_the_selector` | unit |
| A refusal becomes a typed, catalog-shaped pre-spawn error | `…a_refusal_becomes_a_typed_pre_spawn_error` | unit, key-set parity |
| **Invariant 8**: the diagnostic names the selector, never its value or a neighbouring secret | `…an_overlay_diagnostic_names_the_selector_but_never_its_value` | unit, negative |
| A materialization failure projects its stage and publishes its `io::Error` | `…a_materialization_failure_projects_its_stage_and_publishes_its_cause` | unit |
| The `--repo` exclusion table's exact contents | `…the_repo_isolation_table_matches_the_documented_set` | unit, all 8 rows |
| Codex materializes prompts and pins SQLite state outside the overlay | `claudine-cli profile::tests::overlay_strategy::codex_materializes_its_prompts_and_pins_sqlite_state_outside_the_overlay` | unit |
| Audit D4: explicit `CODEX_SQLITE_HOME` > explicit `CODEX_HOME` > default, and never the overlay | `…codex_prefers_an_explicit_sqlite_home_over_the_source_root` | unit, 4 representation variants |
| A relative state root is refused | `…codex_refuses_a_relative_sqlite_home` | unit, negative |
| Gemini sidecars land under the `.gemini` child, not beside the selector value | `…gemini_materializes_its_oauth_sidecars_under_the_provider_visible_root` | unit |
| An explicit Gemini root is the materialization *source* only | `…gemini_reads_an_explicit_provider_root_as_the_materialization_source` | unit |
| The other eight profiles keep the no-op strategy | `…every_other_profile_keeps_the_no_op_overlay_strategy` | unit |
| An inline plan gains no materialization | `…an_inline_plan_gains_no_materialization` | unit, negative |
| **The exclusion table's move preserves the mirror's behavior**, both spellings, with and without `--repo` | `claudine-cli repo_home::tests::repo_only_exclusions_come_from_the_shared_isolation_table` | unit, all 10 providers |
| The new codes project a catalog-shaped detail on the label-only path | `claudine-cli::error_guards` corpus (+2 entries) | passive corpus |
| No new provider-dispatch site | `claudine-cli::dispatch_inventory` (both tests) | repo guard |

Passive corpus coverage: the two new codes are now in `error_guards::corpus`,
which checks every registered code's detail projection against the catalog, and
the pre-existing registry sweeps (`null_detail_for_agrees_with_the_catalog_for_every_code`,
`from_code_projects_a_catalog_shaped_detail_for_every_registered_code`) cover
them automatically. There is no end-to-end shipped-artifact test in this phase
because nothing calls the planner yet — Phase 10 owns that, and it can only be
written once Phase 6 stops moving `HOME`.

### Non-vacuity, proved rather than asserted

- Corrupting the library's Codex row (`["skills", "agents", "prompts"]` →
  `["skills", "agents"]`) failed `the_repo_isolation_table_matches_the_documented_set`,
  naming `.codex`. The first version of the CLI delegation test **passed**
  against the same corruption, because it only checks that every table entry is
  excluded. That is a real hole, so the test gained an exact
  `excluded_dirs(true) == repo_isolated_resources(offset)` assertion before
  being accepted. Restored and re-run green.
- `no_plan_patches_a_home_variable` counts the patches it inspected and fails if
  the sweep found nothing to check; `every_published_refusal_pair_refuses_before_a_plan_exists`
  and `the_repo_isolation_table_matches_the_documented_set` assert non-empty
  expectations before iterating.
- The gen-side and error-catalog negative tests are non-vacuous by construction:
  each asserts a specific variant, key set, or error kind.

### Gates run

From the `claudine/` package area, on macOS:

- `just test` — **7086 passed, 9 skipped, 0 failed.**
- `just lint` — clean (includes `lint-transport`; the two new typed errors carry
  their provider, reason, stage/selector, and — for `ProviderOverlayFailed` — a
  typed `#[source]`, so nothing collapses to prose).
- `cargo clippy -p claudine -p claudine-cli --all-targets` — clean.

No test was skipped or left failing, and no pre-existing failure was
encountered. The 9 skips are the pre-existing L2-gated set, unchanged since
Phase 2. `just test-l2` was not run: plan → Checkpoint 4 does not ask for it and
no L2-observable behavior changed.

Two gates needed handling, both expected:

- `error_guards::the_corpus_covers_every_code_a_diagnostic_can_return` failed on
  the two new codes, which is exactly its job. Both were added to
  `corpus::all()`.
- `dispatch_inventory` drifted (1520 → 1558 sites). Regenerated with
  `CLAUDINE_UPDATE_INVENTORY=1` and verified the **classification**, not the
  count: `conditional` **61** and `match-provider` **7** are unchanged; only
  `direct-ref`/`reference` grew (1459 → 1497). Two draft sites that would have
  added governed dispatch were rewritten first — a `[(Provider::X, reason)]`
  fact table in the library tests (the `tuple-array` form Phase 2 hit) became a
  sweep over `PROVIDERS_DISPLAY_ORDER` × `OVERLAY_REASONS`, which is better
  coverage anyway, and a `provider == Provider::Codex` in the CLI test became
  `provider.agent_offset() == ".codex"`, which is how the table is keyed.

### Cross-OS

No `#[cfg]` code was added. One real Windows hazard was found and fixed **in the
tests** before it could reach CI: `Path::is_absolute` is `false` for a rootless
`/x` path on native Windows, and `codex_sqlite_home` requires an absolute state
root — so three Codex fixtures written with POSIX roots would have passed on
macOS and failed on Windows for a reason unrelated to the behavior under test.
The profile test file now builds every absolute fixture through a small
`absolute()` helper that prefixes the host's own root. Everything else compares
`Path` values built by the same joins, which is separator-agnostic on Windows.

A second Windows hazard was found **by the rig, not by reading**, and would
have shipped otherwise: a `PathBuf` comparison is separator-agnostic on Windows,
but an `OsString` comparison is byte-exact. Two `env_patch()` assertions built
their expectation as `home.join(".claudine/.codex")` and compared it against a
value production had built with two `join`s, so the first Windows run failed
with `"…\\.claudine/.codex"` against `"…\\.claudine\\.codex"` — green on
macOS, red on Windows, for a reason with nothing to do with the behavior. Both
test files now build every path expectation one component at a time.

Evidence collected on `build-win-native`:

| Run | Result |
|---|---|
| `just cross-check claudine --os windows provider_overlay` (first attempt) | **2 failed** — the `OsString` separator defect above |
| `just cross-check claudine --os windows provider_overlay` (after the fix) | **25 passed** |
| `just cross-check claudine-cli --os windows overlay_strategy` | **7 passed** |

Linux and WSL2 were not run: no code added in this phase is `cfg`-conditional,
and the two environments compile the same arm macOS does, which is green. Phase
11 owns the four-environment sweep.

### Not done, deliberately

- **Nothing calls the planner.** Plan → Checkpoint 4 is explicit about this, so
  `WrapperProfile::overlay_strategy` and `profile/codex.rs`'s `codex_sqlite_home`
  carry a single `#[allow(dead_code)]` with a reason (verified necessary: without
  it the package build warns, and `just lint` does not build test targets).
- **`repo_home.rs::codex_sqlite_home` was left alone.** It still reads the
  ambient process, which is what Phase 5 owns. The profile's baseline-driven
  twin is six lines and exists because audit D4 requires it; Phase 5 should
  collapse the two when it moves the module.
- **No documentation or skill change.** Phase 12 owns the vocabulary rewrite by
  name, and nothing in the existing text drifted — no doc describes the planner,
  and no user-facing string moved.
- `repo_home_root_files` is still in place (audit F6 predicts it goes dead);
  Phase 5/6 owns confirming that.

## Phase 5

**Scope: overlay materialization.** `claudine/cli/src/commands/wrap/repo_home.rs`
is gone. `claudine/cli/src/commands/wrap/provider_overlay.rs` builds the
overlay from an `OverlayPlan` (planned, then shaped by the profile's
`overlay_strategy`) rather than from `agent_offset` and a hard-coded mirror.
The child still receives `HOME=<overlay parent>`. Phase 6 owns that.

### What landed

- **`build_overlay_env(provider, reasons, cwd, perf, effective_root, home,
  env_baseline)`** replaces `build_repo_home_env`. It plans, runs the profile
  strategy, materializes, and returns the env entries, the provider-visible
  root, and timings. Its env output is unchanged: `HOME`, plus the plan's
  pinned external state (`CODEX_SQLITE_HOME`). The plan's own selector
  (`CODEX_HOME`, `GEMINI_CLI_HOME`, …) is deliberately **withheld**. Applying
  it is Phase 6's behavior change, and applying it early would override an
  explicit ambient value the child still reads.
- **`overlay_reasons(provider, cwd, repo_only, mcp_overlay, effective_root)`**
  replaces `needs_shadow_home`. `RepoResources` for `--repo`; `RepoPrompt` when
  the provider has a non-`Unsupported` prompt verdict **and** repo prompts
  exist; `Mcp` only when the caller's MCP flag is set **without** `--repo`,
  because the composition path passes `mcp || repo` through the one flag.
  Raising `Mcp` under `--repo` would refuse `claudine claude --repo` for a reason
  the user never asked for (audit D1). Pinned by
  `overlay_reasons_infer_mcp_only_without_repo`.
- **`materialize(plan, repo_root, MirrorMode)`** classifies each source entry:
  - names in `plan.excluded_resources()`, bare or dot-prefixed, are omitted,
    so the Codex `prompts` branch Phase 4 flagged is gone, not ported;
  - **live state** (`is_live_state`: the SQLite family, `*.lock`, and anything
    that is not a file, directory, or symlink, such as sockets and FIFOs) is
    never linked or copied, at any depth for the copy;
  - everything else is mirrored with `MirrorMode::native()`: `Link` on Unix
    (the existing per-entry symlink, overlay-owned real files preserved) and
    `Copy` elsewhere;
  - materializations: `State` is never placed; a `repo_scoped` directory
    rebuilds the prompt overlay (user prompts unless `RepoResources`, then repo
    prompts win); other directories are copied; `File` entries (Gemini
    sidecars) are always **real copies**, replacing any legacy symlink.
- **`purge_volatile_state` / `is_volatile_state_file` are unchanged** apart
  from returning `io::Result`. Regular legacy databases stay:
  `codex_overlay_uses_real_sqlite_directory_and_preserves_legacy_state`.
- **Failures are typed.** Every stage maps to
  `ClaudineError::ProviderOverlayFailed { stage }`: a relative or missing
  source root is `SourceRoot` (checked before any storage is created), creating
  the visible root is `StorageRoot`, and the rest is `Materialization`. A
  planner refusal is `ProviderOverlayUnsupported`.
- **Every copied file goes through `claudine::config::atomic::atomic_write`.**
  Symlink creation is not a content write and stays `std::os::unix::fs::symlink`.
- **`codex_sqlite_home` collapsed.** The ambient copy is deleted.
  `profile::codex_launch_sqlite_home(home, env)` plans Codex, runs
  `CodexWrapper::overlay_strategy`, and reads the pinned value, so the
  composition pipeline (`pipeline.rs`, pre-transition capture) records exactly
  the value an overlay pins, from the invocation's baselines. Audit D4's
  semantics hold: explicit `CODEX_SQLITE_HOME` > explicit `CODEX_HOME` >
  `~/.codex`, relative rejected.
- **`exec_prep::ensure_shadow_home`** takes the `EnvBaseline` and is a no-op for
  a provider whose `Mcp` verdict is not `NativeRoot` (audit D1).
- `profile/mod.rs`: `overlay_strategy`'s `#[allow(dead_code)]` is removed, now
  that it has a caller.

### Native Windows: the copy strategy

The old `#[cfg(not(unix))]` branch hard-linked files, including mutable
secrets, and descended exactly one level. A directory two levels down was
passed to `fs::hard_link` and failed the whole overlay, so on native Windows
`--repo` for Claude (`projects/<dir>/…`) would have degraded to `HOME=/dev/null`.

`copy_entry` recurses to any depth, skips live state at every level, follows
directory links with a canonical-path `visited` set so a link cycle ends, and
rewrites a file only when the destination:

1. is not a regular file;
2. has a **link count > 1**, meaning a hard link the old mirror left behind;
3. differs in length; or
4. is older than the source.

Rule 2 is load-bearing, not defensive. A hard link shares its length and
mtime with the source, so rules 3 and 4 alone would keep it forever, and every
provider write would keep landing in the user's file. Replacing the entry via
`atomic_write` (temp + rename) swaps the directory entry without writing
through the link. On Windows the count comes from `GetFileInformationByHandle`,
which needed the `Win32_Storage_FileSystem` feature on `claudine-cli`'s
existing `windows` dependency (no new crate, so `docs/dependencies.md` is
unaffected).

Known cost, recorded rather than solved: on native Windows every overlay
launch stat-walks the mirrored tree, and the first launch copies it,
including `~/.claude/projects`. A directory junction would match Unix
semantics at O(1), but the plan authorizes "copy, or a supported link where
the entry is proven stable", and a session-log directory is not stable. See
`human_review_items`.

### Deviation: the four refusal pairs now refuse in Phase 5

The plan puts "wire the refusal" in Phase 6. It had to land here. Once
materialization is plan-driven, `claudine {opencode,kilo,goose,antigravity}
--repo` has no plan, and the only other outcomes were:

- the legacy fallback, `HOME=/dev/null`. Spec: deleted, never reintroduced; and
  `wrap_opencode`'s own premise guard (`the launch degraded to the null-home
  fallback`) failed on exactly this; or
- launching with no overlay, which silently weakens requested isolation
  (Invariant 5).

So `env/mod.rs` now returns a `ProviderOverlayUnsupported` error before
spawn. Every **other** overlay error still takes the legacy warning +
`HOME=/dev/null` branch, which Phase 6 deletes. The rendered diagnostic, from a
manual run against a fake provider:

```
⤫ ClaudineError: provider.overlay_unsupported
┃ OpenCode cannot satisfy repo_resources through a provider-owned
┃ selector: run OpenCode without --repo; repository-scoped isolation has
┃ no verified provider-owned mechanism for this provider
```

Exit 1, and the fake provider's args file was never written.

Consequence for tests: `opencode_repo_isolation_delivers_the_configured_default_model`
and its `compose_` twin asserted a successful `--repo` launch, which the audit's
refusal list (row 2) makes unreachable. They are now
`opencode_repo_isolation_is_refused_before_spawn` and
`compose_opencode_repo_isolation_is_refused_before_spawn`, covering both routes:
non-zero exit, the typed code, the mode named, no "credential" wording, no spawn,
and (direct route) no `~/.claudine/.opencode` materialized. The
configured-default-model-on-argv coverage *under `--repo`* goes with them. That
scenario can no longer happen for OpenCode. `RECORD_MODEL_DELIVERY` and
`assert_configured_model_delivered` had no other users and were removed.

### Deviation: Claude-family MCP sessions no longer build a pre-transition overlay

`pipeline.rs:647` called `ensure_shadow_home` for **any** provider with MCP
servers, "so a retry that moves onto a shadow-HOME injector finds one on disk".
A Claude plan with `Mcp` is refused, so building one is impossible. The gate
makes it a no-op instead. That drops a `HOME=~/.claudine` write for
`compose --claude --mcp` (a Phase 6-direction change) and loses nothing that
worked: the overlay it built was Claude's, and a Codex rebuild onto it joins
`.codex` again (audit F1) and writes nowhere Codex reads. Phase 7/8 must
give a transition onto Codex/Gemini its own target-provider plan.

### Requirement → test mapping

| Behavior / requirement | Test | Level |
|---|---|---|
| Exclusions come from the shared table ∪ materialized names, both spellings; non-`--repo` Codex hides only `prompts` | `provider_overlay::tests::repo_only_exclusions_come_from_the_shared_isolation_table` | unit, migrated + tightened |
| A `--repo` overlay omits isolated classes on disk and keeps settings/history, both modes | `…a_repo_overlay_omits_isolated_resources_and_keeps_settings` | unit, filesystem |
| **A mutable state file is neither linked nor copied** (sqlite, WAL, `.lock`, Unix socket; nested for copy) | `…a_mutable_state_file_is_neither_linked_nor_copied` | unit, both modes |
| **Recursive nested-directory materialization** (depth 5), real files, no write-through | `…copy_mode_materializes_nested_directories_to_arbitrary_depth` | unit |
| A legacy hard-linked destination is replaced, not written through | `…copy_mode_replaces_a_hard_linked_destination_instead_of_writing_through_it` | unit, regression shape |
| Copy refresh picks up a changed source; overlay-owned files survive | `…copy_mode_refreshes_a_changed_source_and_keeps_overlay_owned_files` | unit, repeated write/read |
| **An explicit source root is read from, not written to** (byte snapshot, path with a space) | `…an_explicit_source_root_is_read_from_not_written_to` | unit, both modes |
| Explicit `CODEX_HOME` pins SQLite to it; the selector is withheld from the child | `…an_explicit_codex_home_pins_sqlite_state_to_the_source_root` | unit, entry point |
| Missing source root → `ProviderOverlayFailed{SourceRoot, NotFound}`, no storage created | `…a_missing_source_root_fails_with_a_typed_error_before_building_storage` | unit, negative |
| Unsupported reason → `ProviderOverlayUnsupported`, nothing written | `…an_unsupported_reason_is_refused_before_anything_is_written` | unit, negative |
| Gemini sidecars are real copies under `.gemini`, replacing a legacy link | `…gemini_sidecars_are_copied_rather_than_linked` | unit, both modes |
| Gemini returns the provider-visible root (Phase 7's injector input), `HOME` unchanged in shape | `…gemini_overlay_reports_the_provider_visible_root` | unit |
| Codex `CODEX_SQLITE_HOME` = real dir; legacy regular DBs preserved | `…codex_overlay_uses_real_sqlite_directory_and_preserves_legacy_state` | unit, migrated off `EnvGuard` |
| Non-Codex gets no `CODEX_SQLITE_HOME` even with one ambient | `…non_codex_overlay_does_not_receive_codex_sqlite_home` | unit, negative |
| `codex_sqlite_home` semantics (default / `CODEX_HOME` / explicit / relative rejected) | `…codex_sqlite_home_*` (3 tests) | unit, migrated to baselines |
| Prompt merge / `--repo` repo-only / repo switch (Unix links) | `…prompt_overlay_*` (3 tests) | unit, migrated |
| Prompt merge as real files (copy mode) | `…prompt_overlay_copy_mode_writes_real_files` | unit |
| Reasons: effective root drives Codex detection; `--repo` short-circuit; MCP inference | `…overlay_reasons_*` (3 tests) | unit |
| Effective root vs cwd; nested-cwd git fallback | `…build_overlay_env_uses_supplied_effective_root_not_cwd`, `…_fallback_resolves_repo_root_from_nested_cwd` | unit, migrated |
| Claude root-level `.claude.json` still linked while `HOME` moves | `…materialize_root_level_state_*` (2 tests) | unit, migrated |
| Purge semantics intact | `…purge_volatile_state_*`, `…volatile_state_files_match_live_dbs_only` | unit, unchanged |
| `ensure_shadow_home` is a no-op without a root-requiring injector | `exec_prep::tests::ensure_shadow_home_is_a_no_op_without_a_root_requiring_injector` | unit |
| **OpenCode `--repo` refuses before spawn**, direct and composition | `wrap_opencode::opencode_repo_isolation_is_refused_before_spawn`, `…compose_opencode_repo_isolation_is_refused_before_spawn` | L1 integration, real binary + fake provider |
| Codex prompt overlay end to end through the real binary (unchanged, still green) | `wrap_basics::codex_wrapper_uses_shadow_home_for_repo_prompt_overlay_without_repo_flag`, `propagated_context_fixtures::isolated_fixture_can_opt_in_to_shadow_home_repo_resources` | L1 integration, pre-existing |
| Child-cwd vs source-repo split through `build_child_env_with_launch` | `env::tests::build_child_env_codex_shadow_home_uses_child_cwd_not_source_repo_root` | unit, pre-existing |
| No new governed dispatch; two removed | `dispatch_inventory` (both tests) | repo guard |

Every migrated `build_*_env` test now states a `HomeBaseline`/`EnvBaseline`
instead of mutating the process `HOME` under `#[serial]`. The file no longer
uses `serial_test` or `EnvGuard`.

Passive corpus: no parser, schema, template, or shipped artifact changed in
this phase, so there is no shipped corpus to sweep. The two diagnostic codes
the refusal/failure paths emit were already in `error_guards::corpus` from
Phase 4.

### Non-vacuity, proved by corruption

Each check was run against a deliberately broken implementation, confirmed red,
then restored (the file diffed identical to the backup) and confirmed green:

| Corruption | Test | Result |
|---|---|---|
| Copy only one level (skip subdirectories), the old Windows shape | `copy_mode_materializes_nested_directories_to_arbitrary_depth` | **FAIL** |
| Drop the link-count rule from `copy_is_current` | `copy_mode_replaces_a_hard_linked_destination_instead_of_writing_through_it` | **FAIL** |
| Drop `*.lock` from `is_live_state` | `a_mutable_state_file_is_neither_linked_nor_copied` | **FAIL** |
| Re-route the refusal into the `HOME=/dev/null` fallback | both `*_repo_isolation_is_refused_before_spawn` | **2 FAIL** |

Checkpoint 5's specific demand, a Windows materialization test that fails
against the old one-level implementation, is the first row. The test runs in
`MirrorMode::Copy` on every OS, so it was provable on macOS, and it is also
green on `build-win-native` (below).

### Gates run

From `claudine/`, on macOS:

- `just test`: **7100 passed, 9 skipped, 0 failed.** The 9 skips are the
  pre-existing L2-gated set.
- `just lint`: clean (includes `lint-transport`).
- `cargo clippy -p claudine-cli --all-targets`: clean.

Two gate failures were handled on the way, both expected:

- `dispatch_inventory`: `cli_dispatch_guard_holds_the_line` reported the two
  `repo_home.rs` `GuardEntry` rows (`matches!` Codex prompt predicate,
  `== Provider::Codex` prompt materialization) as **stale**, because the sites
  no longer exist. Both rows were removed from `GUARD_ALLOWLIST`, and the
  inventory was regenerated with `CLAUDINE_UPDATE_INVENTORY=1`. Classification
  verified, not just the count: `conditional` **61 → 59**, `eq-comparison`
  22 → 20, `match-provider` 7 unchanged. The prompt predicate is now
  `provider.overlay_capability(RepoPrompt)`, and SQLite pinning lives in the
  Codex profile.
- `wrap_opencode` (2 rows): see the refusal deviation above.

`just test-l2` was not run. Checkpoint 5 asks for `just test`, and no L2 row
covers overlay materialization.

### Cross-OS

| Run | Result |
|---|---|
| `just cross-check claudine-cli --os windows provider_overlay` (`build-win-native`) | **26 passed**, no compile warnings: exercises `GetFileInformationByHandle`, recursive copy, and hard-link replacement natively |
| `just cross-check claudine-cli --os linux provider_overlay` | **not run**: `build-linux` lock held by an unrelated job (`nightly-reward-spike`, owner `reward-20260914-c3e60d0`, since 2026-09-14T18:25Z); timed out waiting |
| `just cross-check claudine-cli --os wsl provider_overlay` (`build-win`), attempt 1 | the test profile **compiled cleanly** (`Finished test profile in 3m 03s`); nextest then failed to extract its own archive, so **no tests ran** (harness failure, no JUnit) |
| same, attempt 2 | SSH `kex_exchange_identification: Connection reset by peer` to the rig; stopped retrying |

So the Linux evidence is a clean `cfg(unix)` Linux compile and nothing more.
The Unix-arm tests (symlink mirror, socket fixture) ran only on macOS. Phase 11
owns the four-environment sweep, and CI's Linux/WSL legs will run the file.

Windows-safety notes, all in the tests: every expectation is built by
per-component `join` (the `OsString` separator trap from Phase 4);
`MirrorMode::Link` tests and the socket fixture are `cfg(unix)`; `mirror_modes()`
yields only `Copy` on non-Unix hosts.

### Not done, deliberately

- **`HOME` still moves**, and `materialize_root_level_state` still links
  `~/.claude.json` beside the overlay. Both are needed while `HOME` points
  there. Audit F6 has both, and `repo_home_root_files`, going dead the moment
  Phase 6 applies the selector instead.
- **The `HOME=/dev/null` fallback and the "failed to create shadow HOME"
  warning remain** for non-refusal failures (Phase 6 deletes them).
- **`HomeBaseline` is still captured ambiently** in `env/mod.rs` and
  `exec_prep`. `build_child_env_with_launch` has no home baseline parameter;
  threading the invocation's is Phase 6's signature change. The composition
  pipeline's SQLite capture already uses the invocation's.
- No documentation or skill change. No living doc or skill names
  `repo_home.rs`; every remaining mention is a historical plan/review record.
  The perf label `shadow home sync` and the `shadow_home_path` field keep their
  names (Phase 9 / Phase 12 vocabulary).

## Phase 6

**Scope: direct wrapper wiring and fail-closed diagnostics.** `HOME` stops
moving; the child receives the plan's provider-owned selector patch.

### What landed

- **`HOME` no longer moves.** `provider_overlay::build_overlay` (was
  `build_overlay_env`) returns the `OverlayPlan` itself. The one writer,
  `apply_overlay_env`, applies `plan.env_patch()` whole: the selector
  (`CODEX_HOME`, `GEMINI_CLI_HOME`, `CLAUDE_CONFIG_DIR`, `KIMI_CODE_HOME`,
  `PI_CODING_AGENT_DIR`, `QWEN_HOME`) plus pinned external state. The Phase 5
  "withheld selector" filter and the `HOME` insert are gone.
- **Typed plan on `EnvPlan`.** `shadow_home_path: Option<PathBuf>` became
  `overlay: Option<OverlayPlan>`, with `EnvPlan::overlay_visible_root()` for the
  three consumers (`wrapper_mcp.rs`, `wrapper_stages.rs`, `pipeline.rs`).
  `build_child_env{,_with_launch}` take `overlay_reasons: OverlayReasons` and
  the invocation's `&HomeBaseline` in place of `repo: bool, force_shadow_home:
  bool`. The ambient `HomeBaseline::capture()` in `env/mod.rs` is gone. The
  direct wrapper threads `invocation.home_baseline()`.
- **Reasons are computed by the caller.** The new signature is
  `overlay_reasons(provider, repo_resources, mcp_requested, repo_root)`. `Mcp` is
  raised when `--mcp`/`--use` is set **and** the provider's `Mcp` verdict is not
  `Unsupported` (audit D1). That metadata lookup replaces
  `matches!(provider, Provider::Codex | Provider::Gemini)` in **both**
  `wrap/mod.rs` and `pipeline.rs` (plan R6: two dispatch sites removed). `--repo
  --mcp` for Codex now plans both reasons. The Phase 5 "Mcp only without
  `--repo`" fold is gone, because the composition path no longer ORs the two
  flags into one boolean.
- **Fail closed.** The `failed to create shadow HOME` warning and
  `HOME=/dev/null` arm are deleted. Every `build_overlay` error propagates with
  `?` before spawn, as `ProviderOverlayUnsupported` or `ProviderOverlayFailed`.
  Its `error_guards/transport-allow.toml` `format_context` entry was removed with
  it.
- **`exec_prep::ensure_shadow_home` → `ensure_provider_overlay`.** It takes
  `&HomeBaseline`, keeps an existing plan's `RepoResources`, and applies the
  patch through `apply_overlay_env`. Its doc, which described the `HOME=/dev/null`
  degradation, was rewritten: the stage is now a no-op on the direct path and
  does work only for a composition session planned without `Mcp`.
- **Invariant 1 guard, two layers.** `apply_overlay_env` refuses to write any
  `HOME_VARIABLES` name (`debug_assert!`, skipped in release). The pure
  `home_identity_violation(env)` flags a home variable holding `/dev/null`,
  `NUL` (case-insensitive), or a path with a `.claudine` component. It is
  `debug_assert!`ed in `exec/spawn/setup.rs::debug_assert_child_env`, which
  all three spawn modes call, so every L1 test that spawns (debug `claudine`
  binary) enforces it.
- **Composition restore, translated 1:1.** The `pre_provider_env` block that
  restored ambient `HOME` for an MCP-only overlay now restores the plan's
  selector variable from the `EnvBaseline`, under the same condition (plan
  carries `Mcp`, no `--repo`). Under `--repo`, or for a prompt-only overlay, the
  selector stays in `pre_provider_env` exactly as `HOME` did, so a same-provider
  retry keeps its overlay. The generic every-provider rule is Phase 8's.
  A drifted comment above the first `ensure_provider_overlay` call ("a refreshed
  document can move the provider to a shadow-HOME injector…") had been wrong
  since Phase 5's gate. The code wins, and the comment was rewritten.

### Finding: audit F6 was wrong, and Claude needs two extra pieces

Applying `CLAUDE_CONFIG_DIR` instead of moving `HOME` would have broken
`claudine claude --repo` authentication on macOS. The Phase 5 note was to delete
`materialize_root_level_state` per F6. Before doing that, I inspected the shipped
Claude Code bundle read-only (`strings` on
`~/.local/share/claude/versions/2.1.273`; no credential store touched):

```js
function Zyn(){let t=`.claude${i2()}.json`;return I(process.env.CLAUDE_CONFIG_DIR||Dt(),t)}
function RS(){let n=process.env.CLAUDE_SECURESTORAGE_CONFIG_DIR;
  if(n!==void 0)return(n||l(o(),".claude")).normalize("NFC");return we()}
function xP(n=""){let e=process.env.CLAUDE_SECURESTORAGE_CONFIG_DIR,
  t=e!==void 0?!e:!process.env.CLAUDE_CONFIG_DIR, r=e!==void 0?e.normalize("NFC"):we(),
  c=t?"":`-${a("sha256").update(r).digest("hex").substring(0,8)}`;
  return`Claude Code${Kt().OAUTH_FILE_SUFFIX}${n}${c}`}
```

1. `.claude.json` lives at `$CLAUDE_CONFIG_DIR/.claude.json` when the selector
   is set. **`repo_home_root_files` is not dead.**
   `materialize_root_level_state(plan, mode)` now places `~/.claude.json` inside
   the provider-visible root (`~/.claudine/.claude/.claude.json`), linked on
   Unix and copied elsewhere, as before. It is skipped when the source root is
   explicit (an ambient `CLAUDE_CONFIG_DIR` holds its own copy, which the mirror
   carries).
2. Setting `CLAUDE_CONFIG_DIR` suffixes the keychain service with a hash of the
   directory and moves the credentials directory. `ClaudeWrapper::overlay_strategy`
   (new) pins `CLAUDE_SECURESTORAGE_CONFIG_DIR` to:
   - an ambient `CLAUDE_SECURESTORAGE_CONFIG_DIR`, verbatim; else
   - a non-empty ambient `CLAUDE_CONFIG_DIR` (same hash the user's own launch
     computes); else
   - `""`, which selects the default `Claude Code-credentials` entry and
     `~/.claude`. Claude's own env propagation deliberately preserves an empty
     value for this key.

`audit.md` F6 carries a correction block. The variable is **undocumented**,
so this is raised for human review.

### Requirement → test mapping

| Behavior / requirement | Test | Level |
|---|---|---|
| `codex --repo`: `HOME` and `USERPROFILE` (value with a space) unchanged in the provider **and** a nested `gh` stub; `CODEX_HOME` = overlay; `CODEX_SQLITE_HOME` = `~/.codex`; overlay carries user `config.toml` | `level1_provider_overlay_home::codex_repo_overlay_uses_codex_home_and_leaves_the_user_home` | L1, real binary + fake provider |
| `claude --repo`: home preserved; `CLAUDE_CONFIG_DIR` = overlay; `CLAUDE_SECURESTORAGE_CONFIG_DIR=""`; `$CLAUDE_CONFIG_DIR/.claude.json` is the user's | `…::claude_repo_overlay_keeps_state_and_credential_store_reachable` | L1 |
| No reasons → no selector, no overlay dir, home preserved (absent-selector variant) | `…::a_launch_without_overlay_reasons_sets_no_selector` | L1 |
| Refusal is per mode: `antigravity --repo` refuses before spawn (typed, names `--repo`, no "credential"); `antigravity` alone spawns with the user `HOME` | `…::antigravity_refuses_only_the_repo_mode` | L1 |
| Materialization failure (storage root blocked by a file) → `provider.overlay_failed`, no spawn, no `/dev/null`, no "shadow HOME", no "credential" | `…::a_failed_overlay_stops_the_launch_without_a_null_home` | L1, negative |
| Missing source root → same pre-spawn failure; no storage created | `…::a_missing_source_root_stops_the_launch_before_building_storage` | L1, negative |
| Codex prompt overlay without `--repo`: `HOME` = launch value, prompt reachable via `$CODEX_HOME` | `wrap_basics::codex_wrapper_uses_shadow_home_for_repo_prompt_overlay_without_repo_flag`, `propagated_context_fixtures::isolated_fixture_can_opt_in_to_shadow_home_repo_resources` (updated; names are Phase 10's rename) | L1 |
| `compose --codex --mcp`: `HOME` unchanged, `CODEX_HOME` = overlay | `wrap_compose_exec::compose_supports_mcp_runtime_and_tag_cleanup` (updated) | L1, composition route |
| Env assembly through `build_child_env_with_launch`: prompt overlay follows `child_cwd`; `HOME` stays at the baseline; `CODEX_HOME` set. Now baseline-driven, no process-env mutation or `#[serial]` | `env::tests::build_child_env_codex_overlay_uses_child_cwd_not_source_repo_root` | unit |
| Selector applied, no home var in patch: Codex (explicit `CODEX_HOME` is source, selector = overlay), Codex MCP, Gemini (`GEMINI_CLI_HOME` = parent of `.gemini`), Claude (config dir + secure storage) | `provider_overlay::tests::{an_explicit_codex_home_pins_sqlite_state_to_the_source_root, codex_overlay_uses_real_sqlite_directory_and_preserves_legacy_state, gemini_overlay_reports_the_provider_visible_root, non_codex_overlay_does_not_receive_codex_sqlite_home}` | unit |
| `.claude.json` placed in the visible root (both mirror modes, stale copy replaced, not at the old `HOME` spot); not taken from home under explicit `CLAUDE_CONFIG_DIR` (path with a space) | `…::a_default_claude_overlay_carries_the_home_root_state_file`, `…::an_explicit_claude_config_dir_does_not_take_the_home_root_state_file` | unit, filesystem |
| Secure-storage pin: absent / empty `CLAUDE_CONFIG_DIR` → `""`; explicit dir → that dir; explicit secure-storage value wins | `profile::tests::overlay_strategy::claude_keeps_its_credential_store_at_the_pre_overlay_location` | unit |
| Reasons: `RepoResources` for every provider; `Mcp` only with a verdict (Gemini, OpenCode yes; Claude no, even with `--repo`); `--repo --mcp` Codex → both; prompt detection at the supplied root | `…::overlay_reasons_{raise_repo_resources_for_every_provider, raise_mcp_only_for_a_provider_with_an_mcp_verdict, detect_codex_prompts_at_the_supplied_repo_root}` | unit |
| Invariant 1 writer: a profile-pinned `HOME` is never written (panics in debug) | `…::an_overlay_patch_never_writes_a_home_variable` | unit |
| Invariant 1 check: `/dev/null`, `NUL`/`nul`, overlay paths flagged across `HOME`/`USERPROFILE`/`HOMEPATH`; real home, selector vars, non-home `/dev/null`, absent vars pass; non-UTF-8 home passes | `…::home_identity_violation_flags_null_devices_and_overlay_paths`, `…::home_identity_violation_accepts_a_non_utf8_home` | unit |
| `ensure_provider_overlay` no-op off the gate or for Claude; keeps an existing plan without touching disk | `exec_prep::tests::{ensure_provider_overlay_is_a_no_op_off_the_gate_or_without_a_root_requiring_injector, ensure_provider_overlay_keeps_an_existing_overlay}` | unit |
| Two `matches!(Codex \| Gemini)` sites gone | `dispatch_inventory` (both tests; allowlist rows removed, inventory regenerated) | repo guard |

Passive corpus / shipped-artifact rule: no parser, schema, template, prompt, or
shipped artifact changed. The two diagnostic codes were already in
`error_guards::corpus`.

### Non-vacuity, proved by corruption

Each corruption was applied from a backup, run, and restored. The restored file
diffed identical to the backup.

| Corruption | Result |
|---|---|
| Re-insert `HOME=<overlay parent>` after the patch, spawn guard disabled | `codex_repo_overlay_…` and `claude_repo_overlay_…` **FAIL** on "provider saw a moved HOME" |
| Same, spawn guard enabled | same two **FAIL**; claudine panics before spawn: `child env "HOME" is ".../home/.claudine" — a provider overlay must never move the user home` |
| Re-introduce the `failed to create shadow HOME` + `HOME=/dev/null` arm (guard disabled) | `a_failed_overlay_…` and `a_missing_source_root_…` **FAIL** |
| Drop the `CLAUDE_SECURESTORAGE_CONFIG_DIR` pin | L1 `claude_repo_overlay_…`, unit `claude_keeps_its_credential_store_…`, unit `non_codex_overlay_…` **FAIL** |
| Drop the `materialize_root_level_state` call | L1 `claude_repo_overlay_…` **FAIL** ("must carry the user's .claude.json") |

### Gates run (macOS, from `claudine/`)

- `just test`: **7109 passed, 9 skipped, 0 failed**. The 9 skips are the
  pre-existing L2-gated set.
- `just lint`: clean (includes `lint-transport`).
- `cargo clippy -p claudine-cli --all-targets`: clean.
- Checkpoint 6 greps: `rg "dev/null" cli/src` (non-test) finds only shell
  completion scripts and the guard's own literal in `home_identity_violation`.
  No production env-assembly write. `rg "failed to create shadow"` finds nothing.

Expected failures handled on the way:

- `dispatch_inventory`: two stale `GuardEntry` rows (the `matches!` Codex/Gemini
  MCP predicates in `pipeline.rs` and `wrap/mod.rs`) removed. Inventory
  regenerated. Classification checked: `conditional` 59 → 57, `matches-macro`
  15 → 13, `direct-ref` +3 (exempt references in tests and `profile/claude.rs`).
  Total sites 1569 → 1570 for that reason.
- `error_guards`: stale `format_context` allow entry for the deleted warning.
- `test_placement`: `exec_prep/mod.rs` inline tests passed 300 lines. The
  tests were compacted (two no-op cases share one table test), not relocated.
- Three pre-existing L1 tests pinned `HOME=~/.claudine` (`wrap_basics`,
  `propagated_context_fixtures`, `wrap_compose_exec`). They now assert the
  launch `HOME` plus the selector.

### Cross-OS

| Run | Result |
|---|---|
| `just cross-check claudine-cli --os windows provider_overlay overlay_strategy exec_prep wrap::env::tests` | **not run**: compile died with `There is not enough space on the disk. (os error 112)` on `W:`. `W:` had 143 MB free. `W:\ci-verification` holds 161 GB and `W:\WSL` 131 GB. Triggering `RustyBiscuit-CargoSweep` reclaimed nothing (it is rooted at the `C:` checkout). Nothing was deleted (storage rule 2). The fact is recorded in `.claude/skills/os/build-hosts.md`. |
| `just cross-check claudine-cli --os linux …` | **not run**: `build-linux` lock still held by `nightly-reward-spike` / `reward-20260914-c3e60d0` since 2026-09-14T18:25Z (same as Phase 5) |
| WSL | not attempted: the guest's VHDX lives on the full `W:` volume |

A first attempt passed `'a|b|c'` as one filter. The recipe's unquoted argument
line ran the pieces as shell commands (`command not found`). Positional
substrings are the working form; the skill's gotcha now names `|`.

Windows-relevant code, reviewed by hand: `is_home_variable` compares
case-insensitively only under `cfg(windows)`. `home_identity_violation` uses
`Path::components`. The Claude/Codex unit fixtures build paths per component.
The L1 file is `#![cfg(unix)]` (shell stubs). The copy-mode `.claude.json`
placement goes through `atomic_write`, which the Phase 5 Windows run already
exercised.

### Not done, deliberately

- **Perf label `shadow home sync`**, the `McpRebuildInputs.shadow_home` field,
  `repo_flag_info_message`'s "A shadow HOME has been created…", the `--repo`
  help text "via a shadow HOME", and the injector error strings keep the old
  vocabulary: Phases 7, 9, and 12. `repo_flag_info_message` is now factually
  wrong for `--repo` launches. Phase 9 rewrites it.
- **MCP injection is still a no-op for Codex/Gemini (F1)**: the injector
  receives the visible root and joins the offset again. Phase 7.
- **Cross-provider selector restoration** on a proxy/retry transition is still
  only the 1:1 translation above. Phase 8.
- Test renames (`…uses_shadow_home…`) are Phase 10's.

## Phase 7

**Scope: MCP injection on the provider-owned root.** This fixes audit F1:
Codex and Gemini runtime MCP injection wrote one directory below where the
provider reads, so it silently did nothing.

### What landed

- **`McpInjector::inject(servers, env, config_root: Option<&Path>)`.**
  `config_root` is the directory that directly contains `config.toml` /
  `settings.json`. The trait method now documents that contract. Callers pass
  `EnvPlan::overlay_visible_root()`, which is `OverlayPlan::provider_visible_root()`.
  The shape translation stays in `provider_overlay::selector` alone
  (`~/.claudine/.codex` for `CODEX_HOME`, `~/.claudine/.gemini` under
  `GEMINI_CLI_HOME=~/.claudine`).
- **`CodexInjector`** writes `config_root/config.toml` and **`GeminiInjector`**
  writes `config_root/settings.json`. Neither joins `.codex` / `.gemini` any
  more. Gemini's `mcp-server-enablement.json` / `mcp-oauth-tokens.json` copies
  now land in the same `GEMINI_CLI_HOME`-selected directory.
- **Typed refusal.** Both file-backed injectors share
  `require_config_root(provider, config_root)`, which returns
  `ClaudineError::ProviderOverlayFailed { reason: Mcp, stage: McpInjection,
  source: io NotFound }` instead of `ConfigValidation("… requires a shadow
  HOME")`. It uses a new `OverlayStage::McpInjection` (`"mcp_injection"`). The
  diagnostic code stays `provider.overlay_failed`, whose declared detail keys
  (`provider`, `reason`, `stage`, `message`) are unchanged, so the registry
  and the code count (49) did not change.
- **`OpenCodeInjector`** still ignores the parameter (`_config_root`). Its plan
  has no storage root, so every caller passes `None`.
- **Call sites:** `wrapper_mcp.rs` and `composition/pipeline.rs` (local renamed
  `shadow` → `config_root`), plus `launch_plan.rs::rebuild_mcp`.
  **`McpRebuildInputs.shadow_home` → `config_root`.** Its doc claimed the root
  was "materialized whenever MCP is in play — including for providers whose
  injector does not need one". That has been false since Phase 6's
  `ensure_provider_overlay` capability gate, so I rewrote it to say the root
  belongs to the invocation provider only.

### Requirement → test mapping

| Behavior / requirement | Test | Level |
|---|---|---|
| F1, Codex (`ProviderDir`): `codex --mcp` writes the server into `$CODEX_HOME/config.toml`, keeps the user's `model`, leaves `~/.codex/config.toml` byte-identical, and creates no `.codex/.codex`. `HOME`/`USERPROFILE` (spaced) are unchanged in the provider and a nested `gh`. This runs **twice** (read/write/read round trip across the first launch's cleanup). | `level1_provider_overlay_home::codex_mcp_injects_servers_into_the_config_codex_home_names` | L1, real binary + fake provider |
| F1, Gemini (`ParentOfProviderDir`), L1 test 6: `GEMINI_CLI_HOME=~/.claudine`. `$GEMINI_CLI_HOME/.gemini/settings.json` holds `mcpServers.linear` and the user's `theme`, and argv carries `--allowed-mcp-server-names linear`. Both sidecars are regular-file copies in that directory, and the user's originals and `settings.json` are unchanged. There is no `.gemini/.gemini`. Home is preserved. | `…::gemini_mcp_injects_servers_under_the_gemini_cli_home_root` | L1 |
| Composition route passes the same root | `…::compose_gemini_mcp_injects_servers_under_the_gemini_cli_home_root` | L1, `compose` |
| L1 test 7: OpenCode `--mcp` injects inline (`OPENCODE_CONFIG_CONTENT` carries the server). `OPENCODE_CONFIG_DIR` is unset, no dot-prefixed overlay dir appears under `~/.claudine`, and `HOME` is unchanged. | `…::opencode_mcp_injects_inline_without_an_overlay` | L1 |
| Injector writes directly in `config_root` (no re-joined offset), both shapes | `mcp::inject::tests::{codex_writes_config_toml_directly_in_the_config_root, gemini_writes_settings_json_directly_in_the_config_root}` (renamed from `…_in_shadow_home`) | unit |
| Existing settings / sidecar contents preserved at the new root | `…::{codex_preserves_existing_non_mcp_settings, gemini_preserves_existing_settings_and_sidecars}` (updated to pass the provider dir) | unit |
| Linked sidecars become private copies, user files untouched (`cfg(not(unix))` branch pre-seeds copies) | `…::gemini_sidecars_become_private_copies_in_the_config_root` (new) | unit, filesystem |
| Negative: `None` → `provider.overlay_failed`, `reason=mcp`, `stage=mcp_injection`, correct provider, no "shadow HOME" / "credential" text, env untouched, for both injectors | `…::file_backed_injectors_refuse_a_missing_config_root_with_a_typed_overlay_failure` (renamed from `file_backed_injectors_require_shadow_home`) | unit |

The `None` path cannot be reached through the binary. `ensure_provider_overlay`
builds the overlay before either injector runs, so the typed refusal is covered
at unit level only.

Passive corpus / shipped-artifact rule: no parser, schema, template, prompt, or
shipped artifact changed. `provider.overlay_failed` was already in the
`error_guards` corpus, and its detail keys did not change.

### Non-vacuity

Before the fix, the three new Codex/Gemini L1 tests **failed** on the F1
assertion ("Codex must read the injected server from $CODEX_HOME", Gemini's
`settings.json` without `linear`). The OpenCode test passed from the start. It
guards against a regression.

After the fix, each corruption below was applied from a backup and then
restored. `cmp` confirmed the files were identical to the backup.

| Corruption | Tests that failed |
|---|---|
| Codex joins `.codex` again | unit `codex_writes_…`, `codex_preserves_…`; L1 `codex_mcp_…` |
| Gemini joins `.gemini` again | unit `gemini_writes_…`, `gemini_preserves_…`, `gemini_sidecars_…`; L1 `gemini_mcp_…`, `compose_gemini_mcp_…` |
| Drop the Gemini sidecar copies from the injector | unit `gemini_sidecars_…` only. The L1 still sees copies because `GeminiWrapper::overlay_strategy` (Phase 4) already materializes them. The injector copy is a second, redundant layer. |
| `overlay_reasons` also raises `RepoResources` for an MCP launch | L1 `opencode_mcp_injects_inline_without_an_overlay`; `mcp_cli::gemini_and_opencode_wrapper_mcp_dry_run_…` |

### Gates run (macOS, from `claudine/`)

- `just test`: **7114 passed, 9 skipped, 0 failed**. The 9 skips are the
  pre-existing L2-gated set.
- `just lint`: exit 0 (includes `lint-transport`).
- `cargo clippy -p claudine -p claudine-cli --all-targets`: no warnings.

Expected failures handled on the way:

- `dispatch_inventory`: regenerated. The change is +2 `direct-ref` / `reference`
  sites (the `Provider::Codex` / `Provider::Gemini` arguments to
  `require_config_root`) plus line shifts. There is no new conditional dispatch
  (`conditional` stays at 57).
- `test_placement`: `inject.rs` inline tests reached 310 lines, over the
  300-line cap. The sidecar and refusal tests were tightened to 300 lines rather
  than relocating the module.

### Cross-OS

| Run | Result |
|---|---|
| `just cross-check claudine --os windows mcp::inject` | **pass**, 13/13 (includes the `cfg(not(unix))` sidecar branch). `W:` has been freed since Phase 6. |
| `just cross-check claudine --os linux mcp::inject` | **not run**: waited 25 min on the `build-linux` lock, still held by `nightly-reward-spike` / `reward-20260914-c3e60d0` (since 2026-09-14T18:25Z, as in Phases 5 and 6), then my `timeout` ended the wait. Only the lock owner may clear it. |
| WSL | not run. The change is path joins with no WSL-specific surface. |

The L1 file is `#![cfg(unix)]` (shell stubs). No Windows L1 evidence exists for
end-to-end injection. Phase 11 owns that.

### Findings for later phases

- **The injection now takes effect, so the shared overlay is live.** Before
  F1 was fixed the injected file was never read. Now, in `Link` mirror mode,
  `atomic_write` replaces the overlay's `config.toml` / `settings.json` symlink
  with a regular file, which is correct: the user's file is not written through.
  Two consequences are inherited from the original design and are not new code:
  1. every launch shares `~/.claudine/.codex`, so a concurrent non-MCP `codex
     --repo` launch sees another launch's injected servers until its cleanup
     runs;
  2. if cleanup never runs (crash, `SIGKILL`), `mirror_source_root` treats the
     leftover regular file as "overlay-owned" and keeps it. Later launches then
     read a stale snapshot of the user's config plus stale servers until the
     file is removed.

  Neither is in the spec's scope. Both are recorded here for Phase 8
  (retry/resume) and Phase 11 (evidence).
- **Rebuild on a provider transition.** `rebuild_mcp` injects into
  `McpRebuildInputs.config_root`, which is the *invocation* provider's root. A
  retry that moves Codex → Gemini would write `settings.json` into
  `~/.claudine/.codex`. Before the fix it wrote `~/.claudine/.codex/.gemini`,
  so it was equally wrong. That transition needs the target provider's own plan,
  which is Phase 8's generic rule.

## Phase 8

**Scope: composition, sequence, proxy, retry, and resume.** Every attempt starts
from the immutable launch baseline, drops every overlay variable a previous
provider wrote, and applies only its own provider's plan. The session key folds
the whole plan.

### What landed

- **Generic restore rule (Invariant 7).** New
  `provider_overlay::restore_overlay_selectors(env, written, baseline)` sets
  every provider-owned overlay variable to its `EnvBaseline` value, or removes
  it when absent. The names are every provider's metadata selector
  (`all_providers()`) plus every name the invocation's plan patched. The second
  source covers the profile-pinned state selectors (`CODEX_SQLITE_HOME`,
  `CLAUDE_SECURESTORAGE_CONFIG_DIR`), which are not metadata. The composition
  pipeline applies it to `pre_provider_env`. That replaces the two special
  cases:
  - the Codex-only `CODEX_SQLITE_HOME` block, which also read the *live
    process* through `std::env::var_os` rather than the baseline;
  - the Phase 6 MCP-only-without-`--repo` selector restore.

  The drifted "the one provider-shaped key written before this point" comment
  went with them. The Codex `CODEX_SQLITE_HOME` semantics are unchanged: removed
  when absent at launch, and an explicit value restored verbatim.
- **Why `--repo` no longer keeps the selector in the base.** The Phase 6
  translation left `CODEX_HOME` in the rebuild base under `--repo`, so that a
  same-provider retry kept its overlay. That is exactly how a Codex → OpenCode
  retry leaked `CODEX_HOME`. Instead, **the replay re-applies an overlay plan on
  every rebuild** (`launch_plan::replay`):
  - provider unchanged: re-apply the invocation's recorded plan
    (`RecordedLaunch.overlay`), with no re-materialization;
  - provider moved: `rebuild_overlay` plans and materializes the target's
    overlay. It calls `provider_overlay::overlay_reasons` + `build_overlay` from
    `LaunchPlanInputs.overlay: Option<OverlayRebuildInputs { repo_resources,
    mcp_requested, home, env }>` and `workspace_cwd`, which are the same inputs
    and single planning path the direct and composition launches use.

  The patch goes through the new `overlay_env_entries`, the entry form of
  `apply_overlay_env` with the same home-variable guard. A refused or failed
  target keeps its typed `provider.overlay_unsupported` / `provider.overlay_failed`
  diagnostic through `LaunchPlanError::producer_report`.
- **Phase 7's `rebuild_mcp` finding fixed.** `rebuild_mcp` now takes
  `config_root` from the rebuilt plan's `provider_visible_root()`.
  `McpRebuildInputs.config_root` (the invocation provider's root) is deleted. A
  Codex → Gemini retry now writes `settings.json` under `GEMINI_CLI_HOME`, not
  into `~/.claudine/.codex`.
- **`codex_sqlite_home` replay slice deleted.** `LaunchPlanInputs.codex_sqlite_home`
  and `EnvironmentPhase.codex_sqlite_home` were a shadow-HOME-era copy of what
  the Codex plan's patch already carries. `profile::codex::launch_sqlite_home`
  lost its only production caller. It is now `#[cfg(test)]` (and so is its
  re-export), because the three SQLite-contract tests in
  `provider_overlay/tests.rs` pin through it.
- **`overlay` facet.** `SessionCompatibilityKey.overlay: String` (lib) is named
  `"provider overlay"` by `incompatibilities`. `session_key::overlay_facet`
  folds `provider`, the reason set, the sorted provider-owned env patch (selector
  and pinned state), the provider-visible root, and the excluded resources.
  `"none"` means no plan. The facet is read from the plan, not from the child
  env, because reasons and exclusions never reach the env. `LaunchPlan.overlay`
  carries the invocation plan on the verbatim shortcut and the rebuilt plan on a
  replay. `RebuiltLaunchIdentity.overlay` passes it to `session_compat_key` in
  `loop_control.rs`, so a retry/resume key describes the plan the child actually
  receives. The direct wrapper records its plan through
  `LaunchPlanInputs::recorded_only(…, overlay)`. The `session_key.rs` module doc
  gained an **Overlay-derived** facet entry.
- **`ensure_provider_overlay` pre-materialization (plan task 8).** It still runs
  whenever servers resolve, gated to the invocation provider's `NativeRoot` MCP
  verdict. The plan's worry (a retry moving onto a file-backed injector finds no
  overlay) is now answered by `rebuild_overlay` materializing the target's
  overlay itself, so no speculative cross-provider pre-materialization was
  added. The comment above the call now says so.
- **Drift fixed** in `launch_plan.rs`:
  - the module doc ("MCP shadow-HOME materialization", plus a new `## Provider
    overlays` section);
  - `provider_env_baseline` and `patch_with_baseline_restores` docs;
  - the `HOME` skip comment in system-prompt re-application;
  - `pipeline.rs`'s "shadow-HOME materialization" comment.

  Two test fixtures modelled the removed shadow `HOME` as a provider-shaped key
  (`target_launch::tests::a_provider_switch_clears_the_opening_providers_environment`,
  `launch_plan::tests::a_replay_restores_rather_than_deletes_a_key_that_had_a_prior_value`).
  The code is correct, so the fixtures moved to an explicit ambient `CODEX_HOME`.

### Finding: a proxy does not replay, it re-enters the pipeline

The first L1 attempt only used a `failure` **proxy**. Corruption showed it was
vacuous for the replay: reusing the invocation overlay on a provider move (C1)
or skipping the restore (C2) still passed. A proxy hand-off is re-prepared by
`compose::prep::prepare_and_run_active_document`, which is a full pipeline run
that plans from the immutable baseline. Only **retry/resume** with a refreshed
`agent:` reaches `launch_plan::replay`. The L1 tests therefore run both routes:
`Transition::Proxy` and `Transition::Retry`. In the retry route, the fake Codex
rewrites `agent:` and fails.

The first attempt also showed that `compose --repo` Codex → OpenCode now
**refuses the OpenCode hop before spawn** with `provider.overlay_unsupported`,
because OpenCode has no verified `--repo` mechanism. The retry route fails
closed the same way (`a_replay_onto_a_provider_that_cannot_honor_repo_refuses_typed`).
Previously the target launched without isolation, which the spec forbids
(Invariant 5). The L1 transition tests use `--mcp`, a reason both providers
support.

### Requirement → test mapping

| Behavior / requirement | Test | Level |
|---|---|---|
| Checkpoint 8 / spec L1 test 11: Codex → OpenCode (`--mcp`), by **proxy and by retry**. Codex saw `CODEX_HOME` = overlay and `CODEX_SQLITE_HOME` = `~/.codex`. OpenCode sees `CODEX_HOME`, `CODEX_SQLITE_HOME`, and `OPENCODE_CONFIG_DIR` unset, with servers inline in `OPENCODE_CONFIG_CONTENT`. `HOME` is unchanged in both. | `level1_provider_overlay_home::a_codex_to_opencode_transition_leaves_no_codex_selector_in_the_opencode_child` | L1, real binary + fake providers |
| Explicit ambient selectors restored, not deleted: ambient `CODEX_HOME` / `CODEX_SQLITE_HOME` (paths with a space). Codex sees the overlay plus the explicit SQLite home; OpenCode gets both user values back verbatim (proxy and retry). | `…::a_codex_to_opencode_transition_restores_explicit_ambient_codex_selectors` | L1 |
| Phase 7 finding: Codex → Gemini (proxy and retry) injects into Gemini's overlay. `GEMINI_CLI_HOME=~/.claudine` with `mcpServers.linear` and the user's `theme`. No `settings.json` in `~/.claudine/.codex`, no Codex selector in Gemini, `HOME` unchanged. Codex read its own injected server. | `…::a_codex_to_gemini_transition_injects_into_the_gemini_overlay` | L1 |
| Replay onto Codex (Claude → Codex, `--repo`): Claude's `CLAUDE_CONFIG_DIR` and `CLAUDE_SECURESTORAGE_CONFIG_DIR` removed; Codex selector and SQLite state set; no home variable in the patch; `LaunchPlan.overlay` is Codex's; the target overlay is materialized on disk (home path with a space) | `launch_plan::tests::a_replay_onto_codex_applies_the_codex_overlay_plan` | unit, filesystem |
| Replay away from Codex removes every Codex overlay variable and records no overlay | `…::a_replay_away_from_codex_removes_every_codex_overlay_variable` | unit, filesystem |
| Same-provider replay keeps the invocation overlay (no baseline strip under `--repo`) | `…::a_same_provider_replay_keeps_the_invocation_overlay` | unit, filesystem |
| Negative: replay onto Antigravity under `--repo` refuses with `provider.overlay_unsupported`, and the message never mentions credentials | `…::a_replay_onto_a_provider_that_cannot_honor_repo_refuses_typed` | unit |
| Restore rule: invocation selector and pinned state removed; other providers' explicit values (and a stale overwrite) return to the baseline; unrelated keys untouched; no home variable | `provider_overlay::tests::restoring_overlay_selectors_returns_every_overlay_variable_to_the_baseline` | unit |
| Restore rule, non-UTF-8 ambient value restored byte-exact | `…::restoring_overlay_selectors_keeps_a_non_utf8_ambient_value` (`cfg(unix)`) | unit |
| Overlay facet folds the whole plan: `none`; reasons, selector value, root, and exclusions appear; `--repo` vs MCP-only (same selector/root) differ; identical plans compatible; paths built per component | `session_key::tests::the_overlay_facet_folds_the_complete_plan` | unit |
| Facet reads the plan, not a selector already in the child env | `…::the_overlay_facet_ignores_a_selector_the_plan_does_not_carry` | unit |
| Plan task 9: a resume where **only** the overlay moved is refused naming `provider overlay` alone, routed as `failure` with `err` | `loop_control::tests::retry_resume::a_resume_whose_only_moved_facet_is_the_overlay_is_refused` | unit, loop routing |
| `incompatibilities` names the new facet in isolation | `composition::coordinator::tests::session_compatibility_key::each_facet_change_is_named_in_isolation` (row added) | unit, lib |
| Two `eq-comparison` Codex dispatch sites removed (plan R6) | `dispatch_inventory` (stale `GuardEntry` rows removed, inventory regenerated) | repo guard |

Passive corpus / shipped-artifact rule: no parser, schema, template, prompt, or
shipped artifact changed, and no new diagnostic code was added. The
round-trip rule is covered by the existing Phase 7 repeated-launch L1. This
phase persists nothing new.

### Non-vacuity, proved by corruption

Each corruption was applied from a backup, run, and restored. `cmp` confirmed
every file identical to its backup.

| Corruption | Tests that failed |
|---|---|
| C1: replay reuses the invocation overlay on a provider move | all three L1 transition tests (retry route); unit `a_replay_onto_codex_…`, `a_replay_away_from_codex_…`, `…_refuses_typed` |
| C2: pipeline skips `restore_overlay_selectors` | all three L1 transition tests (retry route) |
| C3: restore ignores the plan's pinned-state names | unit `restoring_overlay_selectors_returns_every_…` |
| C4: overlay facet always `none` | unit `the_overlay_facet_folds_the_complete_plan`, `a_resume_whose_only_moved_facet_is_the_overlay_is_refused` |

C1 and C2 initially did **not** fail the proxy-only L1 tests; that is the
finding above.

### Gates run (macOS, from `claudine/`)

- `just test`: **7124 passed, 9 skipped, 0 failed** (the 9 are the pre-existing
  L2-gated set).
- `just lint`: exit 0 (includes `lint-transport`).
- `cargo clippy -p claudine -p claudine-cli --all-targets`: no warnings.
- GitNexus `impact` before editing: `replay` **HIGH** (callers
  `build_launch_plan` → `rebuild_launch_identity` → `rebuild_target_launch`,
  i.e. every retry/resume/proxy rebuild), `rebuild_mcp` LOW. Five came back
  `UNKNOWN`: `SessionCompatibilityKey`, `session_compat_key`,
  `LaunchPlanInputs`, `McpRebuildInputs`, `apply_overlay_env`. Their callers
  were confirmed by text search, and every construction site was updated. The
  retry/resume/proxy suites pass. `detect-changes --scope all`: not partial or
  truncated; it reports the cumulative uncommitted Phases 1–8 diff (90 files,
  259 symbols, risk high).

Expected failures handled on the way: `dispatch_inventory` had two stale
`GuardEntry` rows (the `CODEX_SQLITE_HOME` `provider == Provider::Codex` sites
in `pipeline.rs` and `launch_plan.rs`). Both rows were removed and the inventory
was regenerated: sites 1572 → 1581 (new `Provider::…` references in tests),
`eq-comparison` −2.

### Cross-OS

> **Phase 12 note:** Phase 8 left this section as an unfilled
> `CROSS_OS_PLACEHOLDER`, so Phase 8 recorded no cross-OS evidence of its own.
> The fix's cross-platform evidence is in `## Phase 11` and
> [`test-map.md`](./test-map.md) → L2 and cross-platform evidence.

### Not done, deliberately

- Perf label `shadow home sync`, `repo_flag_info_message`, and `--repo` help
  text: Phase 9 / 12.
- Phase 7 finding 3 (shared `~/.claudine/<offset>` in `Link` mode, stale
  injected file after a crash) is **not made worse**. A same-provider retry
  re-uses the recorded plan with no new materialization. A provider move
  materializes the target's overlay in the same shared location a direct
  launch of that provider already uses. Still recorded for Phase 11.
- The second `ensure_provider_overlay` call inside the injector branch of
  `pipeline.rs` is redundant with the first (identical gate). Left as is; it is
  not Phase 8 scope.

## Phase 9

Scope: the vocabulary and diagnostics surface. That is the `--repo` info line,
the perf substage label, and an audit of how the overlay diagnostics render
and what they claim.

### Pre-edit impact (GitNexus CLI)

- `build_env_perf_substages`: **LOW** (one direct caller,
  `build_child_env_with_launch`).
- `emit_preflight_preamble` (the only caller of `repo_flag_info_message`):
  **LOW**, reaches `async_main`.
- `repo_flag_info_message`: **UNKNOWN** (not in the index). A text search
  confirms one caller, `wrapper_stages.rs:236`, and no test or golden pins its
  text.

### What changed

1. **`--repo` info line** (`cli/src/output/mod.rs`). `repo_flag_info_message`
   now takes `Option<&OverlayPlan>` instead of a bare path. When the plan has a
   selector **and** a filesystem storage root, it appends
   `<SELECTOR> points the provider at its overlay in <provider-visible root>;
   your home directory is unchanged.` An inline or absent plan adds no clause.
   The root is the *provider-visible* root, not the selector's value. For
   Gemini (`ParentOfProviderDir`), that is `~/.claudine/.gemini`, not
   `~/.claudine`. The old "shadow HOME … to preserve authentication" sentence is
   gone. The caller (`wrapper_stages.rs::emit_preflight_preamble`) passes
   `env_plan.overlay.as_ref()`.
2. **Perf substage** `shadow home sync` → `provider overlay`
   (`env/mod.rs::build_env_perf_substages`). The local `shadow_breakdown` became
   `overlay_breakdown`. **Decision:** the child `repo root detect` keeps its name
   because it is not shadow-home vocabulary and still measures exactly that. I
   read the plan's "and its child" as locating the child, not as renaming it.
   The doc comments that named the label were updated: `env/mod.rs` (two sites),
   `perf/mod.rs`, `perf/report.rs`, and `composition/pipeline.rs`. The
   `build_env_perf_substages` doc was also corrected: the substage appears only
   when an overlay is *materialized*. An inline OpenCode MCP plan returns no
   timings.
3. **`--repo` help text** (handed over by Phase 8). The clap doc in
   `wrap/flags.rs` and `compose/mod.rs` changed from "via a shadow HOME" to "via a
   provider overlay". So did the matching field docs in
   `lib/src/composition/coordinator/invocation.rs` and
   `lib/src/composition/types.rs`, and the row in `cli/README.md`. The live help
   snapshot (`wrap_basics__wrapper_help_includes_expected_flags.snap`) does not
   pin `--repo`, so no snapshot changed.
4. **Docs drift from the label rename:** the perf example tree and the
   attribution bullet in `docs/topics/composition.md`. The new label has the
   same width, so column alignment is kept. The rest of `composition.md`'s
   shadow-home prose belongs to Phase 12.

### Audits (no code change needed)

- **Golden captures:** `cli/tests/level2_perf_capture.rs` and every snapshot
  under `cli/tests/snapshots/` pin neither label. The only pin was the synthetic
  tree in `perf/tests/perf_tree.rs`, which was renamed for consistency. That
  test exercises tree roles, not the production label; the new L1 below is
  what pins the label.
- **Diagnostics walk:** `provider.overlay_unsupported` and
  `provider.overlay_failed` are `ClaudineError` variants with registry
  `CodeSpec`s, and no overlay path prints them bespoke. A grep of
  `provider_overlay.rs`, `profile/codex.rs`, `lib/src/provider_overlay/`, and
  `exec_prep` found no print/warn calls. A real-binary probe in a scratch home
  rendered both as `⤫ ClaudineError: provider.overlay_*` blocks. Neither
  `next_action` text (`plan.rs::refuse`) nor either `#[error]` string mentions
  credentials or authentication.
- **Sensitive-env filter:** `git diff main` over `env/sanitize.rs`,
  `profile/`, and `lib/src` shows no allow-list or `is_sensitive_key` change.
  Phase 3 only changed the input from `std::env::vars_os()` to the
  `EnvBaseline`. Generated `allowed_env_keys` are unchanged.

### Requirement → test mapping

| Behavior | Test | Level |
|---|---|---|
| Checkpoint 9: `claudine codex --repo --perf` via the real binary + fake Codex. The info line reads `CODEX_HOME points the provider at its overlay in <home>/.claudine/.codex; your home directory is unchanged.`, the perf tree shows `provider overlay` → `repo root detect`, the output contains no `shadow` / `preserve authentication` / `credential`, and `HOME`/`USERPROFILE` are unchanged for the provider and a nested `gh` | `level1_provider_overlay_home::codex_repo_perf_output_names_the_provider_overlay_not_a_home` | L1 |
| Info line names the selector and the **provider-visible** root: Codex (`ProviderDir`) and Gemini (`ParentOfProviderDir`, root ≠ selector value), with a home path containing a space; no `shadow`/`authentication`/`credential` | `commands::wrap::tests::repo_flag_info_message_names_the_selector_and_the_provider_visible_root` | unit |
| Negative: no plan, and an inline plan (OpenCode MCP), add no overlay clause and never name `OPENCODE_CONFIG_*` | `commands::wrap::tests::repo_flag_info_message_omits_the_overlay_clause_without_a_filesystem_overlay` | unit |
| Both overlay diagnostics are the effective diagnostic (selection and snapshot agree on the code) and still render their own block under an added `wrap_err` context; no credential/authentication/login/token/shadow wording | `output::error_walker::tests::provider_overlay_diagnostics_render_through_the_effective_walk` | unit |
| A removed API key and an overlay failure stay distinct. The launch reports `ANTHROPIC_API_KEY` as removed with the `--include` remedy, separately from the `CLAUDE_CONFIG_DIR` overlay line. The overlay failure is `provider.overlay_failed` with no mention of the key, `--include`, or credentials, and nothing is spawned. The secret value is never printed | `level1_provider_overlay_home::a_removed_api_key_and_an_overlay_failure_are_distinct_diagnostics` | L1 |
| Perf tree roles under the renamed label | `perf::tests::perf_tree::perf_tree_child_env_build_breakdown_nests_without_breaking_reconciliation` (renamed fixture label) | unit |

Passive corpus / shipped-artifact rule: no parser, schema, template, prompt, or
shipped artifact changed. Nothing new is persisted. The perf label is display
only, and no perf JSON consumer reads substage names (repo-wide grep).

### Non-vacuity, proved by corruption

Each corruption was applied from a `/tmp` backup and restored; `cmp` confirmed
the files identical.

| Corruption | Tests that failed |
|---|---|
| C1: info line renders the selector *value* instead of the provider-visible root | `repo_flag_info_message_names_the_selector_…` (Gemini row) |
| C2: perf label reverted to `shadow home sync` | `codex_repo_perf_output_names_the_provider_overlay_not_a_home` |
| C3: overlay clause dropped | `repo_flag_info_message_names_the_selector_…`, `codex_repo_perf_output_…` |
| C4: a clause is emitted even without a filesystem overlay | `repo_flag_info_message_omits_the_overlay_clause_…` |

The walker test and the API-key L1 are guard tests over behavior that was
already correct (the audits above). They pin it rather than fix it.

### Gates run (macOS, from `claudine/`)

- First `just test`: 1 failure, `dispatch_inventory_matches_committed_file`
  (sites 1581 → 1585). The cause is the new `Provider::…` references in the
  Phase 9 unit tests; no production dispatch site was added. It was
  regenerated with `CLAUDINE_UPDATE_INVENTORY=1 cargo nextest run -p
  claudine-cli --test dispatch_inventory`.
- `just test` (rerun): **7129 passed, 9 skipped, 0 failed**. The 9 are the
  pre-existing L2-gated set.
- `just lint`: exit 0.
- `cargo clippy -p claudine -p claudine-cli --all-targets`: no warnings.
- Checkpoint 9 was run for real: `claudine codex --repo --perf -- --version`
  against a fake Codex in the L1 fixture (first mapping row; the output was
  inspected by hand during development). The info line and the perf tree both
  use the new vocabulary, and no `shadow` string appears.

### Cross-OS

None run. Phase 9 changes display strings, a perf label, and doc comments. The
new L1s live in the existing `#![cfg(unix)]` file. The new unit tests build
paths with `Path::join` and compare through `biscuit_file::to_portable_string`,
which renders rooted Windows paths with `/`. The planner does no absoluteness
check, so `/home/user name` plans identically on Windows. There is no OS-level
risk that a local rig would answer better than Phase 11's evidence run.

### Not done, deliberately

- `docs/topics/*.md` shadow-home prose (other than the perf lines), the
  `lib/src/provider/{mod.rs,system_prompt.rs}` field docs, and the skill mirror
  belong to Phase 12.
- `claudine-contract`'s own ephemeral shadow HOME (`contract/src/home.rs`) is a
  separate, intentional isolation for the read-only inference contract. It is
  outside this spec.
- An orphaned snapshot at the doubled path
  `cli/claudine/cli/tests/snapshots/wrap_commands__wrapper_help_includes_expected_flags.snap`
  still says "via a shadow HOME". No test reads it; it was left for Ken to
  decide (noted in `message_to_agent`).

## Phase 10

**Scope: L1 contract tests.** Every numbered L1 contract in the spec now maps
to named tests. The full map is in [`test-map.md`](./test-map.md). This phase
changed tests only; no production code changed.

### Starting point

Phases 6–9 had already landed tests for contracts 6, 7, 9, and 11. Contracts 1,
2, 3, 5, 8, and 10 were partly covered: Codex and Claude `--repo` only, only a
nested `gh`, explicit roots only at unit level, and only the Antigravity refusal
row. Contract 4 had unit coverage but no end-to-end proof. This phase filled
those gaps and did not duplicate what was there.

### What landed

- **Recorder rework** (`level1_provider_overlay_home.rs`). The nested `gh` in
  the fixture `bin` became three stubs, `git`, `gpg`, and `gh`, in a
  `nested-bin` directory. Only the provider script adds that directory to its
  `PATH`, through a `run-nested` helper. **Why:** Claudine itself shells out to
  `git` (`overlay.rs`, `prep_context.rs`, `system_prompt/context.rs`), so a
  `git` stub on Claudine's own `PATH` would record Claudine's view rather than a
  descendant's. The stubs record `HOME`, `USERPROFILE`, `HOMEDRIVE`, and
  `HOMEPATH`. `recording_command` / `recorded` are shared by every launch, and
  `assert_home_preserved` checks the provider and all three tools.
- **`RECORD_OVERLAY`**, a provider-neutral recorder. It prints every home
  variable and every provider-owned selector, the provider-visible root's
  entries, a seeded `marker`, and which seeded `<class>/user.md` files the
  provider can reach.
- **New L1 tests:**
  - `every_activation_reason_leaves_the_launch_home_variables_unchanged`
    covers contracts 1, 2, and 10. `repo_resources` runs for all six
    native-root providers (Claude, Codex, Gemini, Kimi, Pi, Qwen), plus Codex
    `repo_prompt`, `mcp` for Codex, Gemini, and OpenCode (inline), and
    `compose --codex --repo`. Each row asserts that no home variable holds
    `/dev/null`, `NUL`, or a `.claudine` component, that absent
    `HOMEDRIVE`/`HOMEPATH` stay absent, and that the selector value is exact.
  - `an_explicit_provider_root_is_the_overlay_source_and_the_selector_names_the_overlay`
    covers contract 3 for all six providers. It includes Gemini's parent shape
    and Claude's `CLAUDE_SECURESTORAGE_CONFIG_DIR` following the explicit
    directory. The explicit root must not be written to.
  - `a_repo_overlay_hides_exactly_the_documented_resource_classes` covers
    contract 4 for Claude, Codex, Gemini, Kimi, and Qwen against
    `docs/topics/repo-isolation.md`. Settings stay visible, and classes the
    table does not list stay reachable.
  - `every_refused_provider_and_reason_fails_before_the_provider_is_spawned`
    covers contract 8 for audit refusal rows 1–4, on both the direct and
    `compose` routes. It also checks that no overlay storage is created and
    that `claude --mcp` keeps its export guidance (audit D1).
  - `an_unwritable_storage_root_stops_the_launch_with_the_typed_diagnostic`
    covers contract 9 with a `0o555` storage root. It checks its premise and
    skips on a runner that ignores the mode, such as root.
  - `absent_and_non_utf8_home_variables_pass_through_an_overlay_launch_verbatim`
    covers the edge matrix. An absent `USERPROFILE` is not synthesized, and a
    non-UTF-8 value reaches the provider and nested `git` byte-exact.
- **Extended tests:**
  - `codex_mcp_injects_servers_into_the_config_codex_home_names`: contract 5
    for MCP. It seeds `state_5.sqlite` and asserts `CODEX_SQLITE_HOME=~/.codex`
    with no `*.sqlite*` inside `$CODEX_HOME`, on both attempts.
  - `opencode_mcp_injects_inline_without_an_overlay` now uses the shared
    recorder, so nested tools are checked. It also asserts that no
    `.claudine/.opencode` or `.claudine/.config` storage exists.
- **Rename and extension, contract 5:**
  `wrap_basics::codex_wrapper_uses_shadow_home_for_repo_prompt_overlay_without_repo_flag`
  → `codex_wrapper_uses_a_provider_overlay_for_repo_prompts_without_repo_flag`.
  It now asserts `CODEX_SQLITE_HOME` and that no SQLite database or sidecar
  (`.sqlite`, `-wal`, `-shm`) is in the overlay.
  `propagated_context_fixtures::isolated_fixture_can_opt_in_to_shadow_home_repo_resources`
  → `…_to_provider_overlay_repo_resources`, with its fixture and capture names
  updated.
- **Native Windows path forms**, at unit level:
  `env::tests::build_child_env_carries_native_windows_home_forms_through_an_overlay_launch`.
  A drive-letter and a UNC `USERPROFILE` (each with a space) and split
  `HOMEDRIVE`/`HOMEPATH` pass through `build_child_env_with_launch` for a Codex
  `--repo` overlay byte-exact. An absent `HOME` is not synthesized. The test is
  platform-neutral, because the values are carried and never parsed.
- **No `SPAWN_ALLOWLIST` entry.** Every new launch goes through
  `fixture.command()`. `env_remove("USERPROFILE")` is not a guarded
  isolation site.

### Test assumptions corrected on the way (not product bugs)

- **Codex `prompts` exists under `--repo`.** The directory is Codex's
  repo-scoped prompt overlay, so isolation is judged by whether the user's own
  file is reachable, not by whether the directory exists.
- **`claude --mcp` outside `--dry-run` stops before spawn** with the existing
  "does not support runtime MCP injection … `claudine mcp export claude
  --apply`" error. That predates this spec and is not an overlay refusal. The
  guard now asserts the export guidance, that `provider.overlay_unsupported` is
  absent, and that no `.claudine/.claude` storage is created.

### Requirement → test mapping

See [`test-map.md`](./test-map.md). In short:

| Contract | Primary new or changed test |
|---|---|
| 1, 2, 10 | `every_activation_reason_leaves_the_launch_home_variables_unchanged`, plus nested `git`/`gpg`/`gh` in every `recording_command` launch |
| 3 | `an_explicit_provider_root_is_the_overlay_source_and_the_selector_names_the_overlay` |
| 4 | `a_repo_overlay_hides_exactly_the_documented_resource_classes` |
| 5 | `wrap_basics::codex_wrapper_uses_a_provider_overlay_for_repo_prompts_without_repo_flag`, `codex_mcp_injects_servers_into_the_config_codex_home_names` (SQLite added) |
| 6 | existing `gemini_mcp_injects_servers_under_the_gemini_cli_home_root` (Phase 7) |
| 7 | `opencode_mcp_injects_inline_without_an_overlay` (storage-absence and nested-tool checks added) |
| 8 | `every_refused_provider_and_reason_fails_before_the_provider_is_spawned` |
| 9 | `an_unwritable_storage_root_stops_the_launch_with_the_typed_diagnostic`, plus the existing Phase 6 tests |
| 11 | existing Phase 8 transition tests (proxy and retry) |
| Edge matrix | `absent_and_non_utf8_home_variables_pass_through_an_overlay_launch_verbatim`, `build_child_env_carries_native_windows_home_forms_through_an_overlay_launch` |

Passive corpus / shipped-artifact rule: no parser, schema, template, prompt, or
shipped artifact changed. Nothing new is persisted. The Codex MCP L1's two
launches in a row are the existing read/write/read round trip.

### Non-vacuity, proved by corruption

A script (`/tmp/p10-corrupt/run.py`) backed up each target file, applied one
corruption, and ran the Phase 10 filter: the whole L1 binary plus the renamed
and new unit tests. It then restored every file, and `cmp` reported each one
identical to its backup. C1 and C2 also disabled the spawn-time
`home_identity_violation` guard, so the tests themselves had to catch the
regression rather than a debug panic.

| Corruption (pre-fix behavior) | Tests that failed |
|---|---|
| C1: `apply_overlay_env` moves `HOME` to the overlay parent | 14, including `every_activation_reason_…`, `an_explicit_provider_root_…`, `absent_and_non_utf8_…`, `build_child_env_carries_native_windows_home_forms_…`, `wrap_basics::codex_wrapper_uses_a_provider_overlay_…`, `propagated_context_fixtures::…_provider_overlay_repo_resources`, and every earlier home-preserving L1 |
| C2: overlay error → launch with `HOME=/dev/null` | `an_unwritable_storage_root_…`, `every_refused_provider_and_reason_…`, `antigravity_refuses_only_the_repo_mode`, `a_failed_overlay_…`, `a_missing_source_root_…`, `a_removed_api_key_and_an_overlay_failure_…` |
| C3: an ambient selector is ignored as the source root | `an_explicit_provider_root_is_the_overlay_source_…` |
| C4: Qwen's `--repo` exclusions drop `commands` | `a_repo_overlay_hides_exactly_the_documented_resource_classes` |
| C5: sanitation synthesizes `USERPROFILE` from `HOME` and back | `absent_and_non_utf8_…`, `build_child_env_carries_native_windows_home_forms_…` |
| C6: sanitation stores values lossily | `absent_and_non_utf8_…`, `sanitize_process_env_preserves_non_utf8_values` |
| C7: SQLite files are no longer live state (mirrored) | `wrap_basics::codex_wrapper_uses_a_provider_overlay_…`, `codex_mcp_injects_servers_…` |

No corruption failed the nested-tool assertion alone. C1 trips the provider
line first. The nested check is the same assertion over the tools' own
records, and no realistic fault moves a descendant's home without also moving
the provider's.

### Gates run (macOS, from `claudine/`)

- First `just test`: 1 failure, `dispatch_inventory_matches_committed_file`
  (sites 1585 → 1587). The cause is the two `Provider::Codex` references in the
  new `env::tests` unit test; no production dispatch site changed. I
  regenerated the inventory with `CLAUDINE_UPDATE_INVENTORY=1 cargo nextest
  run -p claudine-cli --test dispatch_inventory`.
- `just test` (rerun): **7136 passed, 9 skipped, 0 failed** (7129 in Phase 9,
  plus 6 new L1 tests and 1 new unit test). The 9 skips are the pre-existing
  L2-gated set.
- `just lint`: exit 0.
- `cargo clippy -p claudine-cli --all-targets`: exit 0, no warnings.

### Cross-OS

| Run | Result |
|---|---|
| `just cross-check claudine-cli --os windows native_windows_home_forms sanitize_process_env build_child_env` | **not run.** The patch upload failed (`scp: write remote "W:/ci-verification/cross-check-….patch": Failure`), then `error: unable to write file …` and `fatal: Could not reset index file`, which points to write failures on `W:` (free space not measured). Nothing was deleted. The symptom is now recorded in `.claude/skills/os/build-hosts.md`. |
| `just cross-check claudine-cli --os linux <new test names>` | **not run**: waited on the `build-linux` lock, still held by `nightly-reward-spike` / `reward-20260914-c3e60d0` since 2026-09-14T18:25Z (same as Phases 5–8), then stopped. Only the lock owner may clear it. |
| WSL | not attempted: the guest's VHDX is on the same `W:` volume. |

Code relevant to Windows, reviewed by hand. The new Windows-forms unit test
touches no OS-specific branch. `plan.env` keys are the baseline's own spelling,
so the `HashMap` lookup is exact on Windows too. The overlay directory is
built with `Path::join` and compared as an `OsString`. `home_identity_violation`
does not treat `\\fileserver\homes` as a sentinel. The new L1s are Unix-only
(shell stubs). Their only non-POSIX risk is the `0o555` premise, which skips on
a runner that ignores the mode.

### Not done, deliberately

- **L2 and the four-environment evidence** belong to Phase 11. `test-map.md`
  has a placeholder section for them.
- **Pi is absent from the contract 4 table.** `docs/topics/repo-isolation.md`
  documents no Pi row, and `repo_isolated_resources(".pi")` falls through to the
  default arm. Contracts 1 and 3 cover Pi. Phase 12 should decide whether the
  doc gains a Pi row.
- **The doc table still lists Goose and OpenCode rows**, but both refuse
  `--repo` now (refusal rows 2 and 4). Phase 12's doc sync applies.
- **The Phase 8 log still holds a literal `CROSS_OS_PLACEHOLDER`** under its
  `### Cross-OS`. It was left as found, because it is Phase 8's record.

## Phase 11

**Scope: L2 and cross-platform evidence.** This phase added one L2 test, fixed
one production regression that `just test-l2` exposed, and recorded evidence per
environment in [`test-map.md`](./test-map.md). macOS and Linux are met. Native
Windows and WSL2 are unmet, for the reasons below (both in `human_review_items`).

### Hosts at the start

- `BUILD_LINUX=build-linux`, `BUILD_WIN=build-win-native`, `BUILD_WSL=build-win`
  were declared.
- `build-win-native`: `Get-PSDrive W` reported **0.00 GB free** (C: 25.75 GB).
- `build-linux`: the standing-clone lock was still held by
  `nightly-reward-spike` / `reward-20260914-c3e60d0` (since 2026-09-14T18:25Z).
- `build-win` (WSL guest): SSH failed at key exchange with `Connection reset by
  peer`, twice. Its VHDX is on the full `W:`.
- No CI evidence can be reused: HEAD is not contained in any remote branch.

### What landed

- **`cli/tests/level2_provider_overlay_capture.rs`** (registered with
  `required-features = ["terminal-tests"]`). **Why L2:** the L1 suite drives
  Claudine with piped stdio, which only reaches the non-interactive launch. A
  user types `claudine codex --repo` at a terminal with no prompt, and the
  provider inherits that terminal. The test runs that launch in a real terminal:
  - **Unix** (`unix::level2_tmux_codex_repo_overlay_keeps_the_user_home_in_an_interactive_launch`):
    a detached tmux session (no window, no focus). A launcher script starts
    `claudine` under `env -i` with only fixture variables, so an ambient
    `CODEX_HOME` or `CLAUDINE_*` in the pane's shell cannot leak in. Shell
    recorders match the L1 file's.
  - **Windows** (`windows::level2_wezterm_windows_codex_repo_overlay_copies_nested_directories_and_keeps_the_user_home`):
    a WezTerm `cmd.exe` pane with a `setlocal` launcher that clears ambient
    `CLAUDINE_*` and Codex selectors. One rustc-built recorder serves every
    role by file stem, since a `.cmd` provider has batch argument limits and
    Rust's `Command` resolves only `.exe` for the nested tools. It adds a
    by-copy proof: the provider appends to the overlay's nested file, and the
    user's file must be unchanged.
  - Shared assertions: the provider's stdin and stdout are TTYs, and its banner
    is in the pane. For the provider and nested `git`/`gpg`/`gh`: `HOME`
    and the spaced `USERPROFILE` are unchanged, `HOMEDRIVE`/`HOMEPATH` are
    absent, and no home variable is `/dev/null`, `NUL`, or under `.claudine`.
    `CODEX_HOME` is the overlay and `CODEX_SQLITE_HOME` is `~/.codex`. Settings
    and `rules/team/nested/deep.md` (two levels below a mirrored entry) are
    reachable, `skills` is hidden, SQLite is not in the overlay, and the user's
    `config.toml` is untouched. Unix also asserts that `rules` is a symlink to
    the user's entry.
- **Windows premise guard.** Before launching anything, the Windows test
  asserts that `HomeBaseline::capture().resolved()` lies inside the fixture. On
  native Windows it cannot: `dirs::home_dir()` reads the known-folder profile
  (`os` skill, windows.md item 2). The overlay's source *and* storage roots
  come from it (`OverlayPlanner::{source_root, overlay_home}`), so a launch
  would read and write the real `%USERPROFILE%\.claudine`. The guard fails with
  that explanation instead of touching the real profile or skipping silently.
  This is a decision for the author (human review item 1). The Windows arm is
  compile-verified only.

### Production regression found and fixed

The full macOS `just test-l2` was red in
`level2_lifecycle_control::{level2_lifecycle_resume_refuses_when_refresh_changes_mcp_server_set,
level2_lifecycle_resume_refuses_when_refresh_changes_an_interpolated_mcp_tag}`.
No earlier phase had run `just test-l2`. Reproduced directly:

```
HOME=<tmp>/home (no .codex)  claudine codex --mcp "hello"
⤫ ClaudineError: provider.overlay_failed
┃ provider overlay for Codex failed at source_root (mcp): provider source root does not exist
```

Phase 5's `materialize` refused any source root that was not a directory, so a
user who has never run Codex could no longer use `--mcp`, and `claude --repo`
without `~/.claude` refused too. The old shadow home built an empty overlay in
both cases. The spec's Outcome keeps MCP injection working and never requires
the source root to exist, and the log records no rationale for the refusal.

**Fix** (`cli/src/commands/wrap/provider_overlay.rs::materialize`), based on
`fs::metadata(source_root)`:

- directory → mirror as before;
- missing and **default** → create the visible root and skip the mirror
  (materializations still run; each already checks its own source);
- missing and **explicit** (`plan.source_root_is_explicit()`) →
  `ProviderOverlayFailed { SourceRoot, NotFound }`, because a named root that
  isn't there would otherwise drop the user's configuration without a word;
- a file → `SourceRoot`, `NotADirectory`;
- any other metadata error → `SourceRoot` with the real error.

The doc comment on `materialize` now states the default/explicit contract.
Impact: GitNexus returned `Target 'materialize' not found` (the file is new and
not in the index), so callers were confirmed by text search. The only
production caller is `build_overlay`, which serves the direct wrapper
(`env/mod.rs`), `exec_prep`, and the proxy/retry rebuild (`launch_plan.rs`). All
three routes get the same rule. No docs described the old refusal.

This is a behavior change inside an authorized phase checkpoint ("if any
environment is red, fix it"). It is listed in `human_review_items` so it can
be vetoed.

### Requirement → test mapping (this phase)

| Requirement | Test | Level |
|---|---|---|
| Interactive terminal launch keeps home variables for provider and nested tools; no sentinel | `L2::unix::level2_tmux_codex_repo_overlay_keeps_the_user_home_in_an_interactive_launch` | L2 (tmux) |
| Filesystem-backed overlay end to end: settings, depth-2 directory, `--repo` exclusion, SQLite outside, source untouched | same | L2 |
| Native Windows recursive materialization by copy, not link or junction | `L2::windows::level2_wezterm_windows_codex_repo_overlay_copies_nested_directories_and_keeps_the_user_home` | L2 (WezTerm), **compile-only**, blocked |
| Original failing input: `codex --mcp` with no `~/.codex` injects into `$CODEX_HOME` | `L1::codex_mcp_without_a_codex_root_injects_into_an_empty_overlay`; the two `level2_lifecycle_control` MCP resume rows | L1 + L2 |
| Missing default root → empty overlay, selector set, user root not created, no home variable (Codex `mcp`, Claude `repo_resources`) | `provider_overlay::tests::a_missing_default_source_root_builds_an_empty_overlay`, `L1::a_missing_default_source_root_launches_with_an_empty_overlay` | unit + L1 |
| Missing explicit root still refuses before storage; named root not created | `provider_overlay::tests::a_missing_explicit_source_root_fails_with_a_typed_error_before_building_storage` (renamed), `L1::a_missing_explicit_source_root_stops_the_launch_before_building_storage` (renamed; now names a nonexistent `CLAUDE_CONFIG_DIR`) | unit + L1 |
| Root that is a file refuses | `provider_overlay::tests::a_source_root_that_is_a_file_fails_with_a_typed_error` | unit, negative |

Passive corpus / shipped-artifact rule: no parser, schema, template, prompt, or
shipped artifact changed. Nothing new is persisted.

### Non-vacuity

**L2 test.** Each corruption was applied alone and run through `just _test_l2`:

| Corruption | Result |
|---|---|
| `apply_overlay_env` sets `HOME` to the overlay parent, spawn guard off | FAIL: "provider saw a moved HOME" |
| `--repo` exclusions ignored in `mirror_source_root` | FAIL: "--repo must hide the user's skills" |
| SQLite not treated as live state | FAIL: "SQLite state was mirrored into the overlay" |
| `run_child` gives the provider `Stdio::null()` stdin | FAIL: "the provider did not inherit the terminal" |

**Missing-root fix.**

| Corruption | Tests that failed |
|---|---|
| Default missing root refuses again | `a_missing_default_source_root_builds_an_empty_overlay`, `L1::a_missing_default_source_root_launches_with_an_empty_overlay`, `L1::codex_mcp_without_a_codex_root_injects_into_an_empty_overlay` |
| Explicit missing root also starts empty | `a_missing_explicit_source_root_fails_with_…` (unit), `L1::a_missing_explicit_source_root_stops_…` |

**A mistake in my own proof, caught and corrected.** The first corruption
script restored files with `shutil.copy2`, which keeps the backup's older
mtime. Cargo never rebuilt, so later runs used the last corruption's binary
(provider stdin `/dev/null`). A `cmp` against the backup still passed. Symptoms:
the new test failed 40 of 40 `--stress-count` iterations with `STDIN_TTY=no`,
and two full `just test-l2` runs were invalid (a WezTerm spawn error under
`-j 14` in the first, the stdin failure in the second). After `touch` and a
rebuild, the test passed **20 of 20** stress iterations at `-j 14`. Both invalid
runs were discarded. The fact is now in the `rust-testing` skill. The
missing-root proof restored with a plain write plus `os.utime`.

An earlier false alarm, also a fixture bug: the Unix recorder first tested
`[ -t 1 ]` inside `{ … } > "$CLAUDINE_ENV_FILE"`, which tests the redirect. It
now samples both TTY states before the redirect.

### Gates run

**macOS, from `claudine/`:**

- `just test --no-fail-fast`: first run, 7139/7140. The failure was
  `dispatch_inventory_matches_committed_file`, because the new unit test's
  provider table adds `Provider::` references. Regenerated with
  `CLAUDINE_UPDATE_INVENTORY=1 cargo nextest run -p claudine-cli --test dispatch_inventory`
  (sites 1587 → 1589, both `tuple-array`, no production dispatch site). Rerun:
  **7140 passed, 9 skipped** (the pre-existing L2-gated set).
- `just lint`: **exit 0** (run after the production fix).
- `just test-l2 --no-fail-fast` (claudine-cli, parallel self-spawn `-j 14`):
  **224/226**. The overlay L2 and both MCP resume rows pass. Failing, and
  unrelated:
  `level2_lifecycle_control::level2_shipped_implement_plan_{supplied_commit_message_runs_exact_commit_branch,
  unset_commit_message_reaches_provider_and_auto_branch}`. The uncommitted
  shipped `implement-plan.md` fixture (other in-flight work) still uses bare
  `{{area}}`: "subtree-compose strict mode: unknown root 'area'". Because the
  cli half failed, the recipe did not reach `claudine-gen`, so that ran
  separately: `just _test_l2 claudine-gen --features terminal-tests` (tmux
  required, self-spawn) → **3/3 passed**.
- Stress: `--stress-count 20` of the new L2 test at `-j 14` → **20/20**.
- Windows compile: `cargo check -p claudine-cli --tests --features terminal-tests --target x86_64-pc-windows-gnu`
  (the `check-windows` recipe's env) → finished, no warnings or errors. That
  scratch target was deleted afterward.
- `just ci-local --plan` (read-only): 126 changed paths. Cells include
  `claudine-cli/{ubuntu-latest,macos-latest}/L2`,
  `claudine-cli/{windows-latest,wsl2-ubuntu}/L2 = omit / accepted-gap`, and L1 on
  all four environments. CI therefore supplies no Windows or WSL2 L2 evidence.

**Linux (`build-linux`).** A scratch clone was used, because the standing
clone's lock is stale. It was built with `git clone --shared` of the standing
clone plus a bundle for the missing base commit and a temporary-index binary
patch; the standing clone was not modified. Recorded in the `os` skill.

- Before the fix: the new L2 test passed (tmux required), and the overlay
  L1/unit filter passed 108/108.
- After the fix: overlay L1/unit filter (`provider_overlay overlay_strategy
  wrap::env::tests … source_root empty_overlay without_a_codex_root`)
  **123/123**. L2 `level2_provider_overlay_capture` + `level2_lifecycle_control`
  with `BISCUIT_TEST_REQUIRED_BACKENDS=tmux`: **95/97**, the same two unrelated
  `implement_plan` rows.
- The scratch clone and remote temp files were deleted; host usage returned to
  50 G.

**WSL2:** not run. The guest is unreachable (see hosts above).
**Native Windows:** not run. `W:` is full, and the test's hermetic premise fails
by design (human review item 1).

### Plan tasks left unchecked, deliberately

- "Each environment proves (a) and (b)": met on macOS and Linux, unmet on
  native Windows and WSL2.
- "Native Windows additionally proves recursive materialization": the test
  exists, but it is compile-only and blocked.
- "Run `just test-l2` locally on macOS … run what does not": macOS and Linux
  ran; Windows and WSL2 could not.

Checkpoint 11 is therefore **not** fully met. Everything missing is named in
`human_review_items`.

## Phase 12

**Scope: documentation, skill mirror, and acceptance sweep.** Entries are
appended as each task lands.

### `docs/topics/repo-isolation.md` rewrite

- New `## Four Things That Are Not the Same` table: provider overlay,
  user-home identity, provider authentication preservation, and environment
  credential admission (spec → Documentation and Migration).
- "What `--repo` Actually Does Today" rewritten: plan → refuse or build under
  `~/.claudine/<agent-offset>` from the source root → mirror (link on Unix,
  recursive copy on native Windows, live state never mirrored) → provider-owned
  selector, home variables untouched. The `HOME=~/.claudine` step and the
  `HOME=/dev/null` paragraph are gone. New `### When the Overlay Cannot Be
  Built` covers both diagnostic codes, the four `OverlayStage` names, and the
  Phase 11 default/explicit/file missing-root rule.
- The exclusion table now carries selector, shape, and source root per
  provider. **Goose and OpenCode rows removed** (both refuse `--repo`); **Pi row
  added** (`skills`, `commands`, `agents`, `hooks`, the `repo_isolated_resources`
  default arm). A new `### Providers That Refuse --repo` table lists
  Antigravity, Goose, OpenCode, Kilo with the audit's reason for each.
- Codex section moved to overlay vocabulary. The SQLite contract statement is
  kept and now says the value resolves from the launch-time environment (D4).
- New `## Claude Special Case`: `.claude.json` placement and the
  `CLAUDE_SECURESTORAGE_CONFIG_DIR` pin, marked as undocumented upstream.
- "Preserved Authentication Is King" names the incident in one sentence.

**Tests for the Pi row (Phase 10 message item 5).** The doc now claims a Pi
row, so it was made test-backed rather than asserted:

- `cli/tests/level1_provider_overlay_home.rs::a_repo_overlay_hides_exactly_the_documented_resource_classes`
  gains `("pi", ["skills", "commands", "agents", "hooks"])`.
- `lib/src/provider_overlay/tests.rs::the_repo_isolation_table_matches_the_documented_set`
  gains `(".pi", …)`.
- Both pass. Non-vacuity: adding `".pi" => &["skills"]` to
  `repo_isolated_resources` fails the L1 with "pi: user `commands` reachability
  differs from the documented table". Restored with a plain write plus `touch`;
  `cmp` against the backup is clean.
- The unit table still lists `.goose` and `.opencode` arms. They are dead
  (both providers refuse before planning) but removing them is a code change
  outside this phase; left as is.

### MCP docs

- `docs/topics/mcp-catalog.md`: contains no injection or home text. No change.
- `docs/topics/mcp-mode.md` described no mechanism at all, so it gained a short
  `## Runtime Injection` section: Codex/Gemini overlay via `CODEX_HOME` /
  `GEMINI_CLI_HOME` (parent shape), home variables unchanged, SQLite pinned,
  OpenCode inline with no overlay, empty overlay for a never-run provider, and
  pre-spawn `provider.overlay_failed` with no fallback. Mirrored verbatim into
  `.claude/skills/claudine/mcp-mode.md` (the two files were identical before and
  are identical after).
- Caught while writing: Kilo's `mcp` verdict is `ComposableInjection`, but Kilo
  has no `runtime_injector` (only Codex, Gemini, OpenCode implement one), so the
  verdict is inert. The first draft said Kilo injects inline; corrected before
  moving on.

### `system-prompt.md`, `composition.md`, and drifted code comments

- `docs/topics/composition.md` and the skill copy: the dry-run seam list says
  "provider-overlay planning and materialization" (was "MCP shadow-HOME
  materialization"; the seam also precedes `--repo` overlays, confirmed at
  `pipeline.rs:238` vs `:640`), and the rebuild paragraph says "the provider
  overlay plan" (was "shadow HOME").
- `docs/topics/system-prompt.md` and the skill copy: `ShadowHomeFile` was
  described as "retained for providers that require it". The code says
  otherwise: no provider data selects it and `wrap/system_prompt.rs` warns
  "no longer implemented". **Drift detected, code taken as correct:** the line
  now says it exists only so `shadow_home_file` research records still generate.
- Comment-only drift fixes, in files whose behavior this fix changed (no
  non-comment line changed except two registry description strings):
  - `cli/src/commands/wrap/composition/pipeline.rs` dry-run seam comment;
  - `cli/src/commands/wrap/harness_orch/loop_control/target_launch.rs` `//!`;
  - `cli/src/commands/wrap/provider_overlay/tests.rs` Gemini test doc;
  - `lib/src/provider/system_prompt.rs` `ShadowHomeFile` variant docs;
  - `lib/src/provider/mod.rs` `agent_offset` and `repo_home_root_files` field
    docs. `agent_offset`'s only production consumers are overlay storage and
    the isolation-table key (text search; `provider.agent_offset()` in
    `plan.rs:452,460` and `selector.rs:76`);
  - `gen/src/registry.rs` field descriptions for the same two fields. The
    strings appear nowhere else (not in `catalog.json` or any baseline).
  - Left alone as historical: `lib/src/provider_overlay/plan.rs:4` (describes
    what the plan replaced), `lib/src/mcp/inject.rs:696` (a negative assertion
    that a message does *not* say "shadow HOME"), and `fixes/2026-09-12-shadow-home`
    path citations.

### Switches, CLI reference, READMEs, and flow docs

- `--repo` help text (`wrap/flags.rs`, `compose/mod.rs`, `composition/types.rs`)
  already says "via a provider overlay" (Phase 9). No change.
- `docs/topics/wrapped-execution-switches.md` had no `--repo` text. It lists the
  variables the wrapper injects, so it gained `### Home and Provider Overlay
  Variables`: the four home variables are never set; selectors and state pins
  are set only when an overlay is needed.
- `.claude/skills/claudine/cli-reference.md` `--repo` row: "via a shadow HOME"
  → provider overlay, `HOME` unchanged, and the four refusing providers.
- `claudine/cli/README.md`: MCP injection bullet, and the module tree entry
  `repo_home.rs → Shadow HOME` (that file was deleted) → `provider_overlay.rs`.
- `claudine/lib/README.md` `inject` bullet; `claudine/README.md` topic link.
- `docs/topics/execution-flow.md` 6d/6e; `docs/pipeline.md` C3.1/C3.2 and the
  skip rule (now names `overlay_reasons`, the pre-spawn refusal codes, and
  that home variables are never written).
- `docs/topics/building-an-agent-wrapper.md`: `agent_offset()` bullet, and the
  Gemini append-mode bullet, which claimed Claudine builds a shadow `HOME` and
  overrides `HOME=`. **Drift detected, code taken as correct:** Gemini data
  uses `EnvVarFile { GEMINI_SYSTEM_MD }` for append too, and
  `apply_system_prompt_via_spec` prefixes `~/.gemini/GEMINI.md` when present.
  The line now says that. (My first rewrite of this line was also wrong and
  was corrected after reading `profile/gemini.rs`.)

### `provider-metadata.md` and `how-to-create-a-new-provider.md`

- `provider-metadata.md` gained `## Provider Overlay`: both facts keys
  (`overlay_selector` sub-keys `env_var`/`shape`/`relocates`/`additive`/
  `source_root`; `overlay_capabilities` per reason), the three-verdict
  vocabulary, the D1 "reason raised only when needed / inert verdicts" rule,
  the five invariant tests by name, and that side effects belong in
  `WrapperProfile::overlay_strategy`. Spellings were checked against
  `catalog-types/src/provider_overlay.rs` and `facts/gemini.yaml`
  (`parent_of_provider_dir: { child: .gemini }`).
- **Drift detected, code taken as correct:** the field count said 42. HEAD's
  catalog already had 43; `SERIALIZED_PROVIDER_INFO_FIELDS` now has 45. The
  prose says 45 and the typed-data list names the two new fields.
- "Governed-site census" said the remaining guard entries include
  "shadow-HOME mechanics". No `GUARD_ALLOWLIST` entry is about home or
  overlays any more (Phases 6 and 8 removed them). Removed from this doc, the
  skill's `architecture.md`, and the allow-list's doc comment in
  `cli/tests/dispatch_inventory.rs` (comment only; the inventory scans
  `lib/src` and `cli/src`, not tests).
- `how-to-create-a-new-provider.md`: a Phase 1 research question for the
  selector (shape, additivity, per-OS default root, "observe; a `*_HOME` name
  proves nothing"); `overlay_selector`/`overlay_capabilities` rows and a
  corrected `repo_home_root_files` row in Typed Catalog Data; CLI checklist
  items for `overlay_strategy` and `repo_isolated_resources` (with the doc row
  and both table tests); three invariant tests in Key Invariant Tests.

### Skill mirror, timeline, hashes, dependencies

- Skill mirror: `SKILL.md` MCP paragraph (shadow HOME → provider overlay via
  `CODEX_HOME`/`GEMINI_CLI_HOME`, home never changed, "pre-shadow" →
  "pre-overlay"); `architecture.md` gains `invocation_context.rs` and
  `provider_overlay/` in the module tree plus a **Provider overlays never move
  the home** paragraph after the child-process environment guard (claims
  checked: `debug_assert_child_env` calls `home_identity_violation`;
  `restore_overlay_selectors` exists); `composition.md`, `system-prompt.md`,
  `cli-reference.md`, `mcp-mode.md` as logged above.
- `timeline.md`: new `2026-09-12 — shadow-home` entry, dated by the fix
  directory like its neighbors.
- Hashes, computed with `md hash`, written, and recomputed to confirm they are
  stable: skill `cli-reference.md`, `composition.md`, `timeline.md` (each also
  `last_updated: 2026-09-16`), and `docs/topics/composition.md` and
  `execution-flow.md`, which carry `hash:` too. No other edited file has a
  `hash:` key. **`spec.md` has no `hash:` key**, so "refresh its hash" had
  nothing to refresh; none was added.
- `docs/dependencies.md`: unchanged. No crate was added or removed across the
  fix. `claudine-cli`'s existing `windows` dependency gained the
  `Win32_Storage_FileSystem` feature in Phase 5, with its rationale in a
  `Cargo.toml` comment. The dependencies doc has never listed that
  dependency's features (only `rendezvous-daemon`'s), and the plan limits this
  task to crate additions and removals.

### Research documents that described Claudine's own behavior

Research docs describe provider behavior and are otherwise out of scope; their
generic "shadow HOME" wording (e.g. `system-prompt/gemini.md`, the
`shadow_home_file` enum in `system-prompt/_schema.yaml`) is left alone. Two
passages made claims about **Claudine** that are now false:

- `docs/research/mcp/codex.md` `## Runtime Injection`. Phase 1 corrected the
  `runtime_injection.mechanism` record but the prose kept the same error
  (step 2: "Write `.codex/config.toml` inside it", the F1 doubly nested shape).
  Step 2 now says `config.toml` directly. "This is the mechanism Claudine
  uses" plus "the shadow home does not inherit saved authentication" wrongly
  described Claudine's overlay; it now says Claudine uses the selector with a
  mirroring overlay and a `CODEX_SQLITE_HOME` pin, and the limitations apply
  to a bare temporary `CODEX_HOME`.
- `docs/research/acp/antigravity.md`: "Claudine's wrapped HOME/shadow HOME
  patterns can accidentally point it at an empty state directory" → Claudine
  never changes `HOME` and refuses `antigravity --repo`.

### Carried items closed

- Phase 10 message item 5: Pi row added and test-backed; Goose/OpenCode rows
  removed in favor of the refusal table (see the repo-isolation entry above).
  The Phase 8 log's `CROSS_OS_PLACEHOLDER` is replaced by a clearly marked
  Phase 12 note pointing to the Phase 11 evidence. The note says the
  placeholder was never filled and does not invent Phase 8 evidence.

### Acceptance sweep

[`acceptance.md`](./acceptance.md) walks all 12 criteria: **10 met, 2 unmet**.

- Criterion 9 (four-OS evidence) is **unmet**: native Windows and WSL2 are
  blocked by the same host capacity and home-authority decision as Phase 11.
  Not re-attempted, and the criterion was not amended.
- Criterion 12 (gates) is **unmet** only because of the two unrelated
  `level2_shipped_implement_plan_*` failures.
- Evidence gathered fresh in this phase, not copied from `test-map.md`:
  - A production source scan for home-variable writes. Only guards and
    presence checks remain (`launch_plan.rs:822`, `pipeline.rs:982`,
    `session.rs:41`, `setup.rs:55`).
  - The single planning path. `build_overlay` is called from `env/mod.rs:318`,
    `exec_prep/mod.rs:158`, and `launch_plan.rs:976`; both direct
    `OverlayPlanner::new` uses are `#[cfg(test)]`.
  - The credential-inference and secret assertions in
    `L1::a_removed_api_key_and_an_overlay_failure_are_distinct_diagnostics`.

**Checkpoint grep deviation.** The checkpoint expects only `_completed` hits.
It also returns provider research (`docs/research/system-prompt/*`, including
the `shadow_home_file` research-schema enum that `claudine-gen` still maps),
the `ShadowHomeFile` identifier and its docs, one historical module comment,
one negative assertion, and this fix's timeline entry. Each was read, and none
describes current Claudine behavior. The full list is in `acceptance.md` →
Checkpoint grep. Renaming the enum or research vocabulary would be a schema
and code change, which is outside a documentation phase.

### GitNexus

No production function's behavior was edited. The function bodies touched are
two test tables, plus two description strings in the `claudine-gen` `REGISTRY`
const table. `impact` on both test functions returned `Target … not found`
(`UNKNOWN`). Text search confirms they have no callers. The registry strings
are read by the JSON registry listing (`registry.rs:844`) and scaffold TODO
comments. They appear in no committed artifact, and the gen baseline tests
passed. `detect-changes` was not run, because nothing is committed in this
phase.

### Requirement → test mapping (this phase)

| Requirement | Test | Level |
|---|---|---|
| The new Pi row in `repo-isolation.md` is true | `L1::a_repo_overlay_hides_exactly_the_documented_resource_classes` (Pi added); `lib/src/provider_overlay/tests.rs::the_repo_isolation_table_matches_the_documented_set` (`.pi` added) | L1 + unit |
| Every other doc claim | Read against code, with no new test: `repo_isolated_resources`, generated `data.rs` selectors, `OverlayStage::as_str`, `injector_for_provider` (so Kilo has no injector), the Gemini profile's `EnvVarFile` delivery, `SERIALIZED_PROVIDER_INFO_FIELDS` (45), `debug_assert_child_env`, and `restore_overlay_selectors` | — |

Passive corpus and shipped-artifact rule: no parser, schema, template, prompt,
or shipped artifact changed. No values are persisted.

Non-vacuity: `".pi" => &["skills"]` fails the L1 (logged above).

### Gates (macOS, from `claudine/`, after all Phase 12 edits)

- `just test --no-fail-fast`: **7140 passed, 9 skipped**, exit 0. This
  includes `dispatch_inventory_matches_committed_file`, so no inventory
  regeneration was needed: the new test rows are strings, not `Provider::`
  references.
- `just lint`: exit 0, no warnings.
- `just test-l2 --no-fail-fast`: **224/226**. The two failures are the
  unrelated `level2_lifecycle_control::level2_shipped_implement_plan_{supplied_commit_message_runs_exact_commit_branch,
  unset_commit_message_reaches_provider_and_auto_branch}`, with the same
  "subtree-compose strict mode: unknown root" cause from the uncommitted
  `shipped_implement_route/_implement/implement-plan.md` fixture edit. The
  overlay L2 and both MCP resume rows pass.
- `BISCUIT_L2_THREADS=4 just _test_l2 claudine-gen --features terminal-tests`:
  3/3.
- Cross-OS: not run. Phase 12 changed documentation, comments, two test
  tables, and two registry description strings. None of these is OS-sensitive.
  The Windows and WSL2 blockers from Phase 11 are unchanged.


## Implementation of Review Findings #1

> **started at:** 2026-09-16T08:58:43-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/feat-better-static-analysis/claudine/fixes/2026-09-12-shadow-home/review-1.md'
- this is iteration 1 of the review-to-implement cycle
- starting the work on 'Finding 1: hermetic L1 fixture scrubs provider selectors' at 08:59:23
    - discovery: provider metadata carries exactly one selector per provider (`ProviderInfo::overlay_selector.env_var`: `CLAUDE_CONFIG_DIR`, `CODEX_HOME`, `GEMINI_CLI_HOME`, `GOOSE_PATH_ROOT`, `KIMI_CODE_HOME`, `OPENCODE_CONFIG_DIR`, `KILO_CONFIG_DIR`, `QWEN_HOME`, `PI_CODING_AGENT_DIR`); the names outside metadata are the profile-pinned external state (`CODEX_SQLITE_HOME`, `CLAUDE_SECURESTORAGE_CONFIG_DIR`) and the inline MCP selectors from the audit (`OPENCODE_CONFIG_CONTENT`, `KILO_CONFIG_CONTENT`)
    - GitNexus impact on `inherited_scrub_keys` and `write_probe_stub` returned `UNKNOWN` (test-binary helpers are not indexed); text search confirms each has a single private caller (`child_environment`, `probe_fixture`), so the blast radius is the claudine-cli L1 test binaries only
    - change: `cli/tests/common/mod.rs` — new `provider_selector_vars()` (metadata selectors chained with a four-entry `PROFILE_OWNED_SELECTOR_VARS`), appended to `inherited_scrub_keys()`; module docs now list five scrubbed families
    - change: `cli/tests/cli_process_fixture.rs` — the probe stub (Unix and Windows) records every `provider_selector_vars()` name, generated rather than hand-listed; new test `both_command_surfaces_drop_every_inherited_provider_selector` exports every selector with a sentinel, asserts `command()` and `command_std()` record identical environments with each selector empty, and asserts a selector set after `build()` still arrives
    - ambient-restoration tests in `level1_provider_overlay_home.rs` already set their values on the built command (`.env` after `fixture.command()`), so none needed changes
    - docs: `.claude/skills/rust-testing/SKILL.md` fixture-scrub paragraph now names third-party tool selectors; the claudine skill and `docs/topics/testing.md` do not enumerate scrubbed families, so they were left alone
    - verification with a parent exporting all 13 selectors (`CODEX_SQLITE_HOME CODEX_HOME GEMINI_CLI_HOME CLAUDE_CONFIG_DIR CLAUDE_SECURESTORAGE_CONFIG_DIR GOOSE_PATH_ROOT KIMI_CODE_HOME OPENCODE_CONFIG_DIR OPENCODE_CONFIG_CONTENT KILO_CONFIG_DIR KILO_CONFIG_CONTENT QWEN_HOME PI_CODING_AGENT_DIR`): `just test-cli --test level1_provider_overlay_home --test cli_process_fixture --no-fail-fast` → 48/48 passed (23 overlay + 25 fixture)
    - same targets with no selectors in the parent → 48/48 passed
    - non-vacuity: with the new `keys.extend(provider_selector_vars())` line commented out and all selectors exported, 18/48 failed — 17 overlay contracts (including the five named in the review) plus the new policy test; line restored and the file touched for a fresh mtime
    - the default `target/` directory was writable on this host; no alternate `CARGO_TARGET_DIR` was needed
    - `just test --no-fail-fast` (claudine area): 7141 passed, 9 skipped, exit 0
    - `just lint` (claudine area): exit 0, no warnings
- work completed for 'Finding 1: hermetic L1 fixture scrubs provider selectors' at 09:08:30
- starting the work on 'Finding 2: per-launch isolated overlay roots' at 09:09:06
    - GitNexus upstream impact: `build_overlay` **CRITICAL** (16 symbols; Launch_plan, Wrap, Exec_prep, Composition, Env), `mirror_source_root` and `purge_volatile_state` **CRITICAL** (6; every direct, composition, and replay route reaches them through `materialize`), `copy_entry` LOW (4), `overlay_facet` LOW (1, `session_compat_key`); `OverlayStorage` and `OverlayPlanner` returned `UNKNOWN` — text search confirms their callers are `OverlayPlanner::plan`, `build_overlay`, and unit tests only
    - design: each `build_overlay` call plans a fresh launch root `~/.claudine/overlays/<provider-slug>/<pid>-<nanos>-<counter>` (`OverlayPlanner` generates it; `with_launch_id` pins it for deterministic unit fixtures) and materializes into it through a new `claudine::provider_overlay::OverlayLease`; the plan holds the lease in an `Arc`, so every clone (the invocation's recorded launch, a retry's replay) keeps the root and dropping the last one removes it
    - design: liveness is an exclusive OS file lock (`std::fs::File::lock`, stable since 1.89) on a sibling `<root>.lock`, not a PID — the OS drops it on `_exit`, a crash, or a second Ctrl+C, where `Drop` never runs; `sweep_abandoned_overlays` runs before each materialization and removes only roots whose lock it can take (a lease locks before it creates its root, so a starting launch is never swept); `remove_dir_all` does not follow the Unix mirror's links into the user's source
    - design: same-provider retry/resume re-applies the invocation's own plan (already the replay rule), so a resume sees the same root the opening attempt wrote into; a provider-moving replay builds its own root, dropped with that attempt
    - design: the session-compatibility overlay facet spells the launch root `<overlay>` (env patch values and the visible root), so two launches of the same plan key equal; provider, reasons, patch shape, and exclusions still move the key
    - legacy storage: `~/.claudine/<agent_offset>` (and Gemini's old `GEMINI_CLI_HOME=~/.claudine`) is outside `overlays/` and is no longer read, written, purged, or removed by any launch; decision: it is not read as migration input either, because its real files would re-introduce exactly the stale excluded/deleted content this finding removes. The old "real content in the overlay is overlay-owned" mirror rule, `purge_volatile_state`, the copy freshness check, and the Windows hard-link probe (`link_count`, plus the `Win32_Storage_FileSystem` feature in `cli/Cargo.toml`) only existed to reconcile a reused root, so they were deleted with their unit tests
    - trade-off (persistence): on Unix a provider write to a mirrored top-level entry still reaches the user's source through its link (history, sessions, `auth.json` refreshed in place); a write that *replaces* a link by rename, a new top-level entry the source did not have, and every write on native Windows (recursive copy) now ends with the launch. Before this change such writes lingered in the shared overlay and shadowed the source on later launches. Codex SQLite is unaffected (pinned to the pre-overlay root). Native-Windows OAuth refresh-token rotation written only into the copy is the sharpest consequence and is recorded as a follow-up rather than guessed at here
    - trade-off: an orphaned provider that outlives a killed Claudine loses its root at the next launch's sweep, because the lock died with Claudine
    - changes (lib): new `provider_overlay/lease.rs` (`OverlayLease`, `sweep_abandoned_overlays`); `selector.rs` adds `OVERLAY_LAUNCHES_DIR` and `OverlayStorage::new(launch_root, shape)` (was `overlay_home, agent_offset, shape`); `plan.rs` adds the launch id, `OverlayPlanner::with_launch_id`, `OverlayPlan::hold_lease`; `mod.rs` re-exports and documents the ownership model
    - changes (cli): `wrap/provider_overlay.rs` — `materialize` sweeps, acquires the lease, builds into the fresh root, and returns the lease (`build_overlay` attaches it); reuse-reconciliation code deleted; `harness_orch/session_key.rs::overlay_facet` spells the launch root `<overlay>`; `cli/Cargo.toml` drops `Win32_Storage_FileSystem`
    - tests (lib unit): `every_plan_gets_its_own_launch_root_outside_legacy_storage`, `overlay_storage_is_the_launch_root_under_both_shapes` (replaces the legacy-layout test), and `lease::{a_lease_creates_its_root_and_dropping_it_removes_root_and_lock, dropping_a_lease_does_not_follow_links_into_the_source, a_name_already_in_use_is_refused_and_left_alone, a_sweep_removes_only_roots_without_a_live_owner, a_sweep_of_a_missing_directory_removes_nothing}`
    - tests (cli unit): `concurrent_launches_of_one_provider_each_see_only_their_own_plan` (both mirror modes: non-repo → repo, source deletion, overlapping lifetimes), `dropping_the_last_plan_clone_removes_the_launch_root`, `a_launch_reclaims_roots_abandoned_by_ended_launches`, `codex_overlay_uses_real_sqlite_directory_and_leaves_legacy_storage_untouched` (was `…preserves_legacy_state`); session-key test now proves two launch roots of one plan key equal and the Gemini child segment is kept; deleted `purge_volatile_state_*` (3), `copy_mode_replaces_a_hard_linked_destination…`, `copy_mode_refreshes_a_changed_source…`; path expectations in profile/launch-plan/env/info-line tests updated to the launch root
    - tests (L1, `level1_provider_overlay_home.rs`): new `a_repo_launch_after_a_non_repo_launch_cannot_reach_the_earlier_skills` (also snapshots legacy `~/.claudine/.codex` unchanged), `a_source_entry_deleted_between_launches_is_absent_from_the_next_view`, `overlapping_launches_in_two_repositories_each_see_only_their_own_prompts`, `overlapping_launches_with_two_mcp_server_sets_each_keep_their_own_config`, `a_launch_ending_inside_another_launchs_lifetime_leaves_that_overlay_intact` (overlapping fake providers: the outer one blocks on a release file while the inner launch runs to completion, then records its view at exit); the shared `launch_root` helper makes every existing overlay test assert the selector names a launch root under `overlays/<slug>/` and that the root is gone after the launch — direct, compose, proxy, and retry routes all pass it
    - tests (L2, `level2_provider_overlay_capture.rs`): the fake providers (tmux shell script and Windows recorder) now record SQLite presence and the `rules` entry kind while running, since the root no longer exists afterwards; the Windows premise check from review finding 3 was not touched
    - non-vacuity: emulated the shared persistent root (fixed launch id, lease with no lock/removal, sweep disabled, mirror skipping existing entries, root-removed assertion disabled) → all five new L1 regressions failed plus the existing two-launch Codex MCP test; restored from `/tmp` copies and `touch`ed for a fresh mtime (a first emulation that left the sweep on passed the two sequential tests, because the unlocked shared root was swept — which is itself the fix working)
    - docs: `docs/topics/repo-isolation.md` (new "One Launch, One Overlay Root" section: lifetime, concurrency, retry/resume keying, sweep, persistence trade-off, legacy storage; drifted table row, step 3, parent-shape paragraph, and legacy-SQLite paragraph fixed), `docs/topics/mcp-mode.md` + skill mirror, `docs/topics/building-an-agent-wrapper.md` (`agent_offset` no longer names storage), `cli/README.md`, skill `SKILL.md`, `architecture.md`, `timeline.md`; research docs quoting historical host paths were left as observations
    - first `just test --no-fail-fast`: 7146/7150 passed, 9 skipped; 4 failures were mine and are fixed: `dispatch_inventory_matches_committed_file` (14 new test-only `direct-ref` sites in `cli/src` unit tests, all `exempt_candidate`; regenerated `docs/providers/dispatch-inventory.json` with `CLAUDINE_UPDATE_INVENTORY=1`), and three L1 tests pinning the legacy `~/.claudine/.codex` path (`mcp_cli::codex_wrapper_mcp_dry_run_shows_cleaned_prompt_and_shadow_file`, `wrap_basics::codex_wrapper_uses_a_provider_overlay_for_repo_prompts_without_repo_flag`, `wrap_compose_exec::compose_supports_mcp_runtime_and_tag_cleanup`) now assert the launch-root prefix
    - final `just test --no-fail-fast` (claudine area): **7149 passed, 1 failed, 9 skipped**; the failure is unrelated: `shipped_prompt_contract::shipped_prompts_have_parseable_schemas_and_expressions` rejects `prompts/_reviews/feature-review.md` (`Expected ')', found ':' at position 241`), a repo-root prompt modified by someone else at 09:34:44, after my first run in which that test passed
    - `just lint` (claudine area): exit 0, no warnings (run twice, after the last edits)
    - `level1_provider_overlay_home`: 28/28 (23 existing + 5 new); `claudine` lib `provider_overlay` unit tests 31/31; `BISCUIT_TEST_REQUIRED_BACKENDS=tmux just _test_l2 claudine-cli --features terminal-tests --test level2_provider_overlay_capture`: 1/1 passed
    - cross-OS: `just check-windows` (x86_64-pc-windows-gnu, `--tests`, claudine + claudine-cli) exit 0 with no warnings — compile evidence only; `level2_provider_overlay_capture` also checked for the Windows target with `terminal-tests`, and the edited Windows recorder source compiled with `rustc --edition=2024` on macOS. Linux and native-Windows execution (`just cross-check`) not run; the file-lock liveness path (`LockFileEx` on Windows) is therefore unexercised off macOS
    - the default `target/` directory was writable; no alternate `CARGO_TARGET_DIR` was needed (the Windows check uses the recipe's own `target/windows-check`)
    - follow-ups: native-Windows persistence of provider writes (OAuth refresh rotation in a copied `auth.json` or Gemini token file) needs a decision — write-back for credential-class entries, or a documented limitation; Windows/Linux runtime evidence for the lease and sweep
- work completed for 'Finding 2: per-launch isolated overlay roots' at 09:38:43
- starting the work on 'Finding 2 follow-up: guarded overlay write-back' at 09:39:41

## Implementation of Review Findings #2

> **started at:** 2026-09-16T15:30:56-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/feat-better-static-analysis/claudine/fixes/2026-09-12-shadow-home/review-2.md'
- this is iteration 2 of the review-to-implement cycle

- starting the work on 'Finding 1: failed write-back keeps a recoverable copy' at 15:31:37
    - discovery: `OverlayLease` is held as `Arc<OverlayLease>` inside `OverlayPlan` (`hold_lease`), shared by every plan clone including retry/resume records, so the last clone's drop is the only point where no later attempt can reuse the root; a wrapper-side "after the child exits" call would run before a same-provider retry reuses the root
    - GitNexus impact (upstream): `WriteBack::apply_entry` LOW (1 direct, `apply`); `reclaim` LOW (1 direct, `sweep_abandoned_overlays`); `WriteBackOutcome`, `OverlayLease`, `sweep_abandoned_overlays` UNKNOWN (no resolved callers) — confirmed by text search: consumers are `cli/src/commands/wrap/provider_overlay.rs` (`materialize`, `lease.write_back`, `sweep_abandoned_overlays`), `lib/src/provider_overlay/plan.rs` (`hold_lease`), and unit tests; no caller reads `WriteBackOutcome` fields
    - design: failures are data, not only a warning — `WriteBackOutcome.failed: Vec<WriteBackFailure>` (entry name, source path, overlay path, error text; never contents); `OverlayLease::release()` is the explicit, idempotent finalization returning a typed `OverlayRelease` (`Removed` / `Retained`), which `Drop` calls and reports on stderr; a root holding unpersisted state is kept with a sibling `<root>.retained` marker listing what to recover, and `sweep_abandoned_overlays` never reclaims a marked root
    - changes:
        - `lib/src/provider_overlay/write_back.rs`: `WriteBackOutcome.failed` + new `WriteBackFailure` (name, source, overlay, error text); `apply` records every read/write error there instead of only logging; `WriteBack::provider()` accessor; module/`apply` docs updated
        - `lib/src/provider_overlay/lease.rs`: new `OverlayRelease` (`Removed(outcome)` / `Retained { provider, root, marker, outcome }`) with `recovery_notice()`; explicit `OverlayLease::release(self)`; `Drop` runs the same finalization once and prints the notice on stderr (plus `tracing::error!`); a failed write-back keeps the root and writes `<root>.retained` holding the notice; `reclaim` (the sweep) skips any root with a marker (an unreadable marker counts as present); module/type docs updated
        - `lib/src/provider_overlay/mod.rs`: re-export `OverlayRelease`, `WriteBackFailure`; module doc names retention
        - `cli/src/commands/wrap/provider_overlay.rs`: module doc names retention (no code change)
        - docs: `claudine/docs/topics/repo-isolation.md` gains a "Write-back failed" bullet; `.claude/skills/claudine/architecture.md` mirror updated
    - tests added:
        - lib `provider_overlay::tests::lease::an_explicit_release_that_writes_back_removes_the_root` (typed `Removed` outcome, `written == ["auth.json"]`, no notice)
        - lib `...::a_failed_write_back_retains_the_root_with_a_marker_the_sweep_honors` (`#[cfg(unix)]`; read-only source dir; asserts `Retained`, exact failure entry, source untouched, overlay keeps rotated bytes, marker == notice, notice names overlay + source paths and no contents, sweep removes 0) — Windows skip documented: a read-only directory attribute does not stop file creation there
        - lib `...::a_sweep_never_reclaims_a_root_marked_retained` (cross-platform)
        - L1 `level1_provider_overlay_home::a_rotated_token_that_cannot_be_written_back_stays_recoverable` (real `claudine codex --repo` with a fake provider rotating `auth.json`, read-only `~/.codex`; asserts source keeps old token, kept overlay copy holds the rotated token, stderr reports "could not be written back" and names the kept path, stderr has no token content, and a second launch's sweep leaves the kept copy); privileged-runner probe mirrors the existing `an_unwritable_storage_root_...` precedent; confirmed with `--no-capture` it ran, not skipped
    - non-vacuity:
        - neutered retention (`match (&self.write_back, true)`) and the sweep marker check together: lib `a_failed_write_back_retains...` and `a_sweep_never_reclaims...` FAIL; L1 FAIL "the rotated token was deleted (No such file or directory)"
        - neutered only the sweep marker check: L1 FAIL at the final post-sweep read (line 1702), proving the second-launch assertion is load-bearing
        - restored from backup with a plain write + `touch`, `cmp` identical; all 8 lease tests and the L1 test pass again
    - first `just lint` failed on `error_guards::production_sources_pass_every_scan_backed_guard` (`no_unallowlisted_typed_error_collapses`: `WriteBackFailure.error` was built from `error.to_string()`); fixed by keeping the typed `io::Error` (dropping the unused `Clone`/`PartialEq`/`Eq` derives on `WriteBackOutcome`, `WriteBackFailure`, `OverlayRelease`) rather than adding an allowlist entry
    - gates (from `claudine/`):
        - `just lint`: exit 0, no warnings
        - `just test`: exit 0 — 7161 passed, 9 skipped (the `shipped_prompt_contract` failure did not occur)
        - `just test-cli --test level1_provider_overlay_home --no-fail-fast`: 30/30 passed
        - `just check-windows` not run: no `#[cfg(windows)]` code was touched; the only platform gate added is `#[cfg(unix)]` on one lib test
    - deferred:
        - no separate wrapper call site for `release()`: the lease is shared by `Arc` across plan clones (retry/resume records reuse the root), so the last clone's drop is the one correct finalization point; `release()` is the explicit, typed path that `Drop` runs, and its outcome reaches the user on stderr and in the marker
        - the launch exit code is unchanged (the child already exited successfully and the notice is emitted from the lease drop); the launch no longer claims persistence silently, but it does not turn the run red
        - a *refused* write-back (source changed during the launch) still discards the overlay copy as documented; retaining it too would be a policy change outside this finding
- work completed for 'Finding 1: failed write-back keeps a recoverable copy' at 15:51:07

- starting the work on 'Finding 2: native-Windows L2 test reaches the launch' at 15:51:50
    - discovery: the overlay's *source* root already honors an explicit ambient provider selector (`OverlayPlanner::resolve_source_root` reads `CODEX_HOME` from the `EnvBaseline` before the home), so `CODEX_HOME=<fixture>` keeps the source out of the real profile; the *storage* root (`overlay_home` → `HomeBaseline::resolved()/.claudine`) had no override, and `HomeBaseline::capture` is `dirs::home_dir()` (known folder on native Windows)
    - no Claudine data-dir override existed; the closest existing pattern is `CLAUDINE_RENDEZVOUS_FALLBACK_DIR` (a single-purpose directory override documented for test isolation and power users)
    - options weighed: (a) move `HomeBaseline` to environment-first `std::env::home_dir()` — rejected, it changes the production home authority for every overlay while ~60 other `~/.claudine` sites (logs, harvest, prompt change-state) stay on `dirs::home_dir()`, and review-2 asked for injection without relying on home variables; (b) a disposable Windows profile — not constructible inside a nextest process; (c) a single-purpose storage override read from the launch baseline — chosen
    - impact: `node .gitnexus/run.cjs impact "overlay_home" --direction upstream` → LOW, 1 direct caller (`OverlayPlanner::plan`), 0 processes; `OverlayPlanner` → UNKNOWN (ambiguous impl match), confirmed by text search: `OverlayPlanner::new` is called from `wrap/provider_overlay.rs`, `wrap/profile/codex.rs` (test helper), and an `exec_prep` unit test; the stale-root sweep derives its directory from the planned storage root, so it follows the override without change
    - change (lib): `OVERLAY_DIR_ENV = "CLAUDINE_OVERLAY_DIR"` in `provider_overlay/selector.rs` (re-exported); `OverlayPlanner::overlay_home` replaced by `launches_dir`, which uses an absolute override from the `EnvBaseline` and otherwise `~/.claudine/overlays`; a relative override is unplannable (typed refusal) rather than silently falling back to the home; moves storage only, never a source root or a home variable
    - change (tests): unit `an_absolute_overlay_dir_override_moves_only_the_launch_roots`, `a_relative_overlay_dir_override_refuses_instead_of_falling_back_to_the_home`; L1 `explicit_codex_home_and_overlay_dir_keep_the_overlay_out_of_the_home` (the exact explicit-root launch the Windows L2 uses, through the real binary)
    - change (L2): the Windows test drops the `HomeBaseline` premise assertion; its launcher sets `CODEX_HOME=<fixture home>\.codex` and `CLAUDINE_OVERLAY_DIR=<workspace>\overlay-launches` (after the existing `CLAUDINE_*`/`CODEX_HOME` clears), keeps fixture `HOME`/`USERPROFILE` so env-first readers (user config) stay in the fixture, and asserts they reach the child unchanged; `assert_overlay_launch` takes the expected launches directory (Unix still asserts the default `~/.claudine/overlays`); the fixed 1 s sleep became `wait_for_cmd_prompt`, polling the pane for a `>` prompt with a 30 s deadline
    - residual (not changed, noted): other `dirs::home_dir()` sites (logs, signal harvest, system-prompt change-state) would still resolve to the known folder on Windows if a launch reached them; the interactive `codex --repo` launch with no repository prompt is not expected to write there, but this has not been observed natively
    - docs: `docs/topics/repo-isolation.md`, skill `architecture.md` and `cli-reference.md` (env var table), `os` skill `windows.md` (known-folder item), `test-map.md` and `acceptance.md` Native Windows rows (the "premise fails by design" text was stale)
    - macOS (Darwin 27, local): `just test-library provider_overlay` → 36 passed; `just test-cli --test level1_provider_overlay_home explicit_codex_home` → passed, and with the planner's env lookup neutered it failed at the overlay-parent assertion (restored with a fresh write + touch)
    - macOS: `BISCUIT_TEST_REQUIRED_BACKENDS=tmux just _test_l2 claudine-cli --features terminal-tests --test level2_provider_overlay_capture` → 1 passed, backend-proof `tmux run=1`
    - Windows compile evidence (macOS host, `x86_64-pc-windows-gnu`): `just check-windows` → clean, 0 warnings; plus `cargo check -p claudine-cli --tests --features terminal-tests --target x86_64-pc-windows-gnu` (the recipe omits `terminal-tests`, so the L2 file needs this) → clean
    - blocker (native Windows execution): `build-win-native` `W:` reported 53 KB free (`W:\ci-verification` 161.3 GB — `rusty-biscuit` 99.3 GB, `rb-pr66` 62.0 GB last written 2026-08-30; `W:\WSL\Ubuntu-26.04\ext4.vhdx` 130.8 GB). The Cargo target is pinned to `W:` and must not be overridden, and the storage-strategy rules forbid an agent deleting artifacts it did not create, so `just cross-check claudine-cli --os windows` was not started; listed for the author. Also observed: the WezTerm socket file exists at `C:\Users\ken\.local\share\wezterm\sock` but no `wezterm-mux-server` process was running, and `cross-check` does not forward `WEZTERM_UNIX_SOCKET`, so a native run additionally needs a mux server and the socket variable in the remote session
    - macOS: first `just test` failed only `dispatch_inventory::dispatch_inventory_matches_committed_file` (1608 → 1611 reference sites): two new `Provider::Codex` fixture references in this finding's planner unit tests and one in Finding 1's lease test (`provider_overlay/tests.rs`), all `direct-ref`/`reference`, `exempt_candidate`; regenerated with `CLAUDINE_UPDATE_INVENTORY=1 cargo nextest run -p claudine-cli --test dispatch_inventory` (documented remedy), which now covers both findings' sites
    - macOS: `just test` → 7164 passed, 9 skipped; `just lint` → passed; `shipped_prompt_contract` did not fail in this run
- work completed for 'Finding 2: native-Windows L2 test reaches the launch' at 16:14:03
- starting the work on 'Finding 3: Kilo MCP capability agrees with runtime' at 16:14:41
    - discovery: facts (`docs/providers/facts/kilo.yaml`) publish `overlay_capabilities.mcp: composable_injection`; the planner already agrees (`provider_overlay/tests.rs::an_inline_injection_plan_acquires_no_storage_root` plans Kilo MCP with no storage root, `provider/tests.rs` pins the verdict); only the runtime disagrees — `KiloProvider` inherits `McpBehavior::runtime_injector() -> None`, so `wrapper_mcp::compose_mcp_session` and `composition/pipeline.rs` hit "does not support runtime MCP injection" and `launch_plan::rebuild_mcp` returns `NoMcpInjector`
    - discovery: `docs/research/mcp/kilo.md` verifies the mechanism — `runtime_injection.supported: true` via `KILO_CONFIG_CONTENT` carrying an inline `mcp` map (tagged `local`, above project config), and `server_shape` lists `transports: [local, remote]`, `command_fields: [type, command, environment, enabled, timeout]`, `http_fields: [type, url, headers, oauth, enabled, timeout]` with `command` as a single argv array — field-for-field the shape `OpenCodeInjector` already emits for `OPENCODE_CONFIG_CONTENT`; the audit row (`audit.md:335`) cites the same source
    - decision: implement the inline Kilo injector rather than mark MCP unsupported. The JSON shape is verified by research, not guessed, and the OpenCode emitter is reusable by parameterizing the env variable and provider-override slug; marking unsupported would discard a verified capability and still require regenerating facts and every pinned verdict
    - impact: `OpenCodeInjector` UNKNOWN (dispatch boundary through `McpInjector`; text search: only `opencode/behavior.rs` constructs it, plus `inject.rs` tests); `KiloProvider` UNKNOWN (dispatch boundary; constructed only as `KILO_PROVIDER` static in its behavior module); `injector_for_provider` HIGH (7: `wrapper_mcp`, `composition/pipeline`, `launch_plan::rebuild_mcp` and their callers) — behavior changes only for `Provider::Kilo`, which previously errored; `merge_injected_env_into_plan` HIGH (13, 11 direct, mostly its own unit tests plus the two MCP folds) — OpenCode handling must stay byte-identical; `rebuild_mcp` LOW (3)
    - discovery: the child env inherits the ambient environment (`env/sanitize.rs` reads `vars_os`), so a user-exported `KILO_CONFIG_CONTENT` reaches the plan; a plain set would clobber it, so the Kilo value merges like OpenCode's in the direct and composition folds and, on a launch-plan rebuild, onto the value captured before any provider stage (mirroring `opencode_config_base`)
    - impact warning: `injector_for_provider` and `merge_injected_env_into_plan` report HIGH; the change is confined to the Kilo key (OpenCode keeps `merge_overlay`'s exact messages via the new `merge_named_overlay`), confirmed by the unchanged OpenCode unit and L1 tests passing
    - change (lib): `opencode_config.rs` gains `OPENCODE_CONFIG_CONTENT`/`KILO_CONFIG_CONTENT` constants, `INLINE_CONFIG_ENV_VARS`, and `merge_named_overlay(name, …)` (errors name the variable); `merge_overlay` delegates with the OpenCode name
    - change (lib): `mcp/inject.rs` moves the OpenCode emitter into `inject_inline_config(provider, key, …)`, reading per-server overrides from the provider's own slug; new `KiloInjector` targets `KILO_CONFIG_CONTENT`; `provider/kilo/behavior.rs` returns it from `runtime_injector()` (module doc no longer says MCP is unwired; `supported()` stays `false` because import/sync/export are still not wired)
    - change (cli): `wrapper_mcp::merge_injected_env_into_plan` merges every `INLINE_CONFIG_ENV_VARS` key (was OpenCode only), so direct and composition launches keep a user-exported `KILO_CONFIG_CONTENT`; `LaunchPlanInputs::kilo_config_base` (captured in `composition/pipeline.rs` beside `opencode_config_base`) lets `launch_plan::replay` merge a rebuilt Kilo server set onto the same base
    - docs: `docs/topics/mcp-mode.md` + skill mirror, `.claude/skills/claudine/SKILL.md` "MCP Support" rollout (Kilo added to runtime injection), `docs/topics/building-an-agent-wrapper.md`, `docs/topics/execution-flow.md`, `lib/README.md`, `cli/README.md`; `repo-isolation.md` makes no Kilo MCP claim and was left alone; facts and generated `data.rs` unchanged (the published verdict already matched the new runtime), so no regeneration
    - tests (unit): `mcp::inject::tests::kilo_injects_the_researched_inline_mcp_shape` pins the exact `KILO_CONFIG_CONTENT` document for a `local` and a `remote` server, merge over an existing value, and `kilo`-slug-only overrides; `supported_providers_return_some` now includes Kilo; `wrapper_mcp::tests::merge_injected_env_merges_a_user_exported_kilo_config`
    - tests (L1, `cli/tests/level1_provider_overlay_home.rs`): `kilo_mcp_injects_inline_without_an_overlay` (direct `claudine kilo --mcp` against a fake `kilo`: exact JSON merged over a user-exported config, no `KILO_CONFIG_DIR`, no overlay storage, home preserved) and `a_codex_to_kilo_transition_injects_inline_kilo_config` (proxy → composition fold; retry → launch-plan rebuild); the recording scripts now also print `KILO_CONFIG_DIR`/`KILO_CONFIG_CONTENT`
    - non-vacuity (sources restored with plain writes for fresh mtimes, `cmp`-verified, then both tests re-passed): (A) `runtime_injector` returning `None` → both L1 tests FAIL; (B) rebuild merging onto `None` instead of `kilo_config_base` → the transition test FAILS, the direct test passes; (C) Kilo dropped from `INLINE_CONFIG_ENV_VARS` → both FAIL
    - docs: `.claude/skills/claudine/timeline.md` shadow-home entry notes Kilo inline MCP is now wired
    - docs: also `cli/README.md` and skill `cli-reference.md` (non-interactive `#tag` stripping now applies to Kilo, since it is not provider-gated once an injector exists) and `docs/pipeline.md` C3.11
    - gate discovery: first `just test` failed `dispatch_inventory_matches_committed_file` (1611 → 1613: the two `Provider::Kilo` direct-refs in `KiloInjector`); regenerated with `CLAUDINE_UPDATE_INVENTORY=1 cargo nextest run -p claudine-cli --test dispatch_inventory`
    - gate discovery: the next run failed `test_placement::repository_test_placement` — `lib/src/mcp/inject.rs` inline tests went from 299 to 349 lines (limit 300); moved the whole test module verbatim (dedented) to `lib/src/mcp/inject/tests.rs`, the layout `mcp/import/tests.rs` already uses; the relocated file adds 10 `exempt_candidate` test sites to the inventory (now 1623), regenerated again; all 14 `mcp::inject` unit tests pass
    - macOS: `just test --no-fail-fast` → 7168 passed, 9 skipped; `shipped_prompt_contract` did not fail in this run
    - macOS: `just lint` → exit 0, no warnings
    - deferred: Kilo MCP import/sync/export (`McpBehavior::supported()` stays `false`) — out of this finding's scope, which covers only the published runtime verdict; native-Windows/WSL2 evidence left to CI (no `#[cfg]` code changed; the new L1 tests live in a `#![cfg(unix)]` binary like their OpenCode siblings)
- work completed for 'Finding 3: Kilo MCP capability agrees with runtime' at 16:30:22

### Successful Completion

The implementation of review cycle 2 has completed successfully in 59m 45s (15:30:56 → 16:30:41). During this implementation all 3 review findings were evaluated to see if they could be fixed as a part of this implementation cycle: 3 were fixed, 0 were deferred (see reasons below):

- no finding was deferred
- residual items recorded against fixed findings (not deferrals of the findings themselves):
        - Finding 1: a launch whose write-back fails still exits `0`; the failure is reported on stderr and in the durable `<root>.retained` marker, and the retained root survives later sweeps
        - Finding 2: the native-Windows Level 2 test now reaches launch by design (`CODEX_HOME` source plus the new `CLAUDINE_OVERLAY_DIR` storage override, and a bounded readiness poll) and cross-compiles cleanly with `terminal-tests`, but it was **not executed** on `build-win-native`: the rig's `W:` volume had 53 KB free (`W:\ci-verification` holds 161.3 GB from other sessions), no `wezterm-mux-server` was running, and `just cross-check` does not forward `WEZTERM_UNIX_SOCKET`; run it with `BISCUIT_TEST_REQUIRED_BACKENDS=wezterm` once space is reclaimed
        - Finding 3: Kilo MCP import/sync/export remain unwired (`McpBehavior::supported()` stays `false`); runtime injection through `KILO_CONFIG_CONTENT` is now implemented and agrees with the published verdict
- the files changed by this cycle:
        - library: `claudine/lib/src/provider_overlay/{write_back,lease,plan,selector,mod,tests}.rs`, `claudine/lib/src/mcp/inject.rs`, `claudine/lib/src/mcp/inject/tests.rs`, `claudine/lib/src/opencode_config.rs`, `claudine/lib/src/provider/kilo/behavior.rs`
        - CLI: `claudine/cli/src/commands/wrap/{provider_overlay,wrapper_mcp,launch_plan}.rs`, `claudine/cli/src/commands/wrap/composition/pipeline.rs`, `claudine/cli/tests/level1_provider_overlay_home.rs`, `claudine/cli/tests/level2_provider_overlay_capture.rs`
        - docs and skills: `claudine/docs/topics/{repo-isolation,mcp-mode,building-an-agent-wrapper,execution-flow}.md`, `claudine/docs/pipeline.md`, `claudine/docs/providers/dispatch-inventory.json`, `claudine/{lib,cli}/README.md`, `.claude/skills/claudine/{SKILL,architecture,cli-reference,mcp-mode,timeline}.md`, `.claude/skills/os/windows.md`, `claudine/fixes/2026-09-12-shadow-home/{test-map,acceptance}.md`
- final gates (macOS, from `claudine/`, after Finding 3): `just test --no-fail-fast` → 7168 passed, 9 skipped; `just lint` → clean

## Implementation of Review Findings #3

> **started at:** 2026-09-16T19:38:20-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/feat-better-static-analysis/claudine/fixes/2026-09-12-shadow-home/review-3.md'
- this is iteration 3 of the review-to-implement cycle
- starting the work on 'Finding 1: recovery survives metadata, source-read, and marker-write failures' at 19:38:36
        - GitNexus impact (upstream): `WriteBack::apply_entry` LOW (1 direct, `WriteBack::apply`); `reclaim` LOW (1 direct, `sweep_abandoned_overlays`); `finish`, `apply`, and `OverlayRelease` ambiguous/UNKNOWN by name — confirmed by text search: `WriteBack::apply` is called only by `OverlayLease::finish`, and no code outside `lib/src/provider_overlay/` destructures `OverlayRelease` (the CLI only calls `OverlayLease::acquire`, `write_back`, and `sweep_abandoned_overlays`)
        - discovery: `apply_entry` has three error collapses — `let Ok(..) = symlink_metadata(overlay) else { return Ok(None) }` (every error treated as removal), `fs::read(source).is_ok_and(..)` (a failed read is "different"), and `Fingerprint::of(source).ok()` (a failed fingerprint is a source conflict, so `refused`); each leaves `failed` empty and the root is deleted
        - discovery: the sweep's only keep signal is a successfully created `<root>.retained` file, and `finish` removes `<root>.lock` in every branch; `reclaim` also *creates* a missing lock file and reclaims a lockless root, so a marker-less retained root is always swept. A full disk, the named failure case, blocks the marker's creation as readily as the write-back
        - discovery: the WSL2 CI leg drops to an unprivileged `biscuit` user for tests (`.github/workflows/_wsl-ci.yml` "Create the unprivileged test user") and the hosted Linux/macOS runners are not root, so asserting a Unix permission premise does not turn any configured CI leg red
