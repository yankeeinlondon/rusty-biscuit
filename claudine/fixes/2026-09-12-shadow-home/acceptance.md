# Acceptance Sweep — Preserve Provider Overlays Without Replacing the User Home

Each item from [`spec.md`](./spec.md) → Acceptance Criteria, walked one at a
time on 2026-09-16 (Phase 12). Paths are relative to `claudine/`.
`L1` = `cli/tests/level1_provider_overlay_home.rs`. The full test inventory
per contract is in [`test-map.md`](./test-map.md).

**Result: 10 met, 2 unmet.** The unmet items are the cross-platform evidence
(native Windows and WSL2) and the gate item, which is red only because of two
L2 failures unrelated to this fix. Neither criterion was amended.

| # | Criterion | Status |
|---|---|---|
| 1 | No production path replaces a global home variable | **met** |
| 2 | `HOME=/dev/null` fallback removed; failures typed and pre-spawn | **met** |
| 3 | Every compiled provider has a test-backed verdict per reason | **met** |
| 4 | Verified selectors with the correct shape; unsupported refused explicitly | **met** |
| 5 | Repo masking, Codex prompt overlay, and runtime MCP intact where supported | **met** |
| 6 | Codex SQLite obeys the separate-state contract | **met** |
| 7 | All launch routes share one planning path and restore provider env | **met** |
| 8 | Nested ordinary tools observe the original home | **met** |
| 9 | macOS, Linux, native Windows, WSL2 evidence | **unmet** (Windows, WSL2) |
| 10 | Diagnostics never infer broken credentials or expose secrets | **met** |
| 11 | Repo-isolation, MCP, architecture, CLI, and skill docs describe the contract | **met** |
| 12 | `just test`, `just test-l2`, `just lint` pass; real-provider tests opt-in | **unmet** (2 unrelated L2 failures) |

## 1. No production wrapper or composition path replaces a global user-home variable

- **Writer:** `OverlayPlan::env_patch` holds only the provider selector and
  state pins. `cli/src/commands/wrap/provider_overlay/tests.rs::an_overlay_patch_never_writes_a_home_variable`.
- **Runtime guard:** `cli/src/commands/wrap/exec/spawn/setup.rs::debug_assert_child_env`
  calls `provider_overlay::home_identity_violation` on every debug spawn and
  panics on `/dev/null`, `NUL`, or a `.claudine` path in `HOME`,
  `USERPROFILE`, `HOMEDRIVE`, or `HOMEPATH`.
- **End to end:** `L1::every_activation_reason_leaves_the_launch_home_variables_unchanged`
  (every reason, direct and `compose`), `L2::unix::level2_tmux_codex_repo_overlay_keeps_the_user_home_in_an_interactive_launch`.
- **Source scan (Phase 12):** the only production `"HOME"` sites outside the
  baseline constant are the two guards that stop a system-prompt producer from
  repointing it (`launch_plan.rs:822`, `composition/pipeline.rs:982`) and
  presence checks (`exec/wiring/session.rs:41`, `exec/spawn/setup.rs:55`).
  `repo_home.rs` and `build_repo_home_env` are deleted.

## 2. The `HOME=/dev/null` fallback is removed and overlay failures are typed, pre-spawn failures

- `ClaudineError::ProviderOverlayUnsupported` / `::ProviderOverlayFailed`
  (`lib/src/error.rs:427-428`), registered as `provider.overlay_unsupported` /
  `provider.overlay_failed` (`lib/src/diagnostics/registry.rs:183,197`).
- `L1::a_failed_overlay_stops_the_launch_without_a_null_home`,
  `L1::an_unwritable_storage_root_stops_the_launch_with_the_typed_diagnostic`,
  `L1::a_missing_explicit_source_root_stops_the_launch_before_building_storage`:
  each asserts the fake provider recorded no spawn.
- `build_overlay`'s contract (`cli/src/commands/wrap/provider_overlay.rs`,
  "Either way nothing has been spawned, and there is no fallback launch").
- Documented exception, not a fallback: a missing *default* source root builds
  an empty overlay (Phase 11, still in `human_review_items`).
  `L1::a_missing_default_source_root_launches_with_an_empty_overlay`.

## 3. Every compiled provider has a test-backed capability verdict for each activation reason

- Generated `overlay_capabilities` for all ten providers
  (`docs/providers/facts/*.yaml` → `lib/src/provider/<slug>/data.rs`).
- `lib/src/provider/tests.rs::overlay_capability_matrix_matches_the_audit`
  pins all 30 cells against `audit.md` → Verdict Matrix. Structural
  invariants: `additive_selectors_cannot_claim_repo_resource_isolation`,
  `repo_resource_isolation_requires_a_single_source_root`.
- Behavior per live cell: contract 1 rows (supported), contract 8 rows
  (refused), and `L1::every_refused_provider_and_reason_fails_before_the_provider_is_spawned`
  also asserts the inert `claude --mcp` verdict keeps its export guidance.

## 4. Supported providers use only verified provider-owned selectors with the correct path shape; unsupported combinations are refused explicitly

- Selector evidence: `audit.md` F2, F3, D2, D3.
- Shape: `L1::an_explicit_provider_root_is_the_overlay_source_and_the_selector_names_the_overlay`
  (all six `native_root` providers, including Gemini's parent shape),
  `lib/src/provider_overlay/tests.rs::{an_explicit_selector_value_is_the_source_and_never_the_destination, an_explicit_parent_shaped_value_resolves_through_its_child_segment}`,
  `lib/src/provider/tests.rs::selector_shapes_and_source_roots_are_well_formed`.
- Refusal: `L1::every_refused_provider_and_reason_fails_before_the_provider_is_spawned`
  (Antigravity, OpenCode, Kilo, Goose × `--repo`, direct and `compose`),
  `L1::antigravity_refuses_only_the_repo_mode`,
  `lib/src/provider_overlay/tests.rs::every_published_refusal_pair_refuses_before_a_plan_exists`.

## 5. Existing repo-resource masking, Codex prompt overlay, and runtime MCP behavior remain intact for supported combinations

- Masking: `L1::a_repo_overlay_hides_exactly_the_documented_resource_classes`
  (Claude, Codex, Gemini, Kimi, **Pi — added in Phase 12**, Qwen) against
  `docs/topics/repo-isolation.md` → What Is Actually Masked;
  `lib/src/provider_overlay/tests.rs::the_repo_isolation_table_matches_the_documented_set`.
- Codex prompts: `cli/tests/wrap_basics.rs::codex_wrapper_uses_a_provider_overlay_for_repo_prompts_without_repo_flag`.
- MCP: `L1::codex_mcp_injects_servers_into_the_config_codex_home_names`,
  `L1::gemini_mcp_injects_servers_under_the_gemini_cli_home_root`,
  `L1::compose_gemini_mcp_injects_servers_under_the_gemini_cli_home_root`,
  `L1::opencode_mcp_injects_inline_without_an_overlay`,
  `L1::codex_mcp_without_a_codex_root_injects_into_an_empty_overlay`.
- "Intact" is stronger than before for MCP: `audit.md` F1 records that Codex
  and Gemini runtime injection previously wrote to a doubly nested path and was
  a no-op. Those tests failed against the old path.

## 6. Codex SQLite state continues to obey the completed separate-state contract

- `CodexWrapper::overlay_strategy` pins `CODEX_SQLITE_HOME` from the launch
  baseline (`cli/src/commands/wrap/profile/codex.rs`; `audit.md` D4).
- `cli/src/commands/wrap/provider_overlay/tests.rs::{codex_overlay_uses_real_sqlite_directory_and_preserves_legacy_state, a_mutable_state_file_is_neither_linked_nor_copied}`;
  L2 asserts SQLite is absent from the overlay and `CODEX_SQLITE_HOME` is
  `~/.codex`. Non-vacuity: mirroring SQLite fails the L2 (Phase 11).
- Documented: `docs/topics/repo-isolation.md` → Codex Special Case.

## 7. Direct, composition, sequence, retry, resume, and proxy launches share one overlay-planning path and restore provider-owned environment correctly

- One planner entry in production: `provider_overlay::build_overlay` →
  `OverlayPlanner::plan`. Callers: `wrap/env/mod.rs:318` (direct wrapper and
  the composition/sequence child environment), `exec_prep/mod.rs:158`
  (composition MCP after server resolution), `launch_plan.rs:976`
  (`rebuild_overlay` for proxy/retry/resume). The other two
  `OverlayPlanner::new` sites are `#[cfg(test)]`.
- Restoration: `L1::{a_codex_to_opencode_transition_leaves_no_codex_selector_in_the_opencode_child, a_codex_to_opencode_transition_restores_explicit_ambient_codex_selectors, a_codex_to_gemini_transition_injects_into_the_gemini_overlay}`
  (proxy and retry each); `launch_plan::tests::{a_replay_onto_codex_applies_the_codex_overlay_plan, a_replay_away_from_codex_removes_every_codex_overlay_variable, a_same_provider_replay_keeps_the_invocation_overlay}`;
  `loop_control::tests::retry_resume::a_resume_whose_only_moved_facet_is_the_overlay_is_refused`.

## 8. Nested ordinary tools observe the original user-home environment

- Contract 10 in `test-map.md`: every `L1` launch runs recorder stubs for
  `git`, `gpg`, and `gh` from a `PATH` only the provider sees, and
  `assert_home_preserved` checks all four home variables for each.
- Non-UTF-8 and absent values: `L1::absent_and_non_utf8_home_variables_pass_through_an_overlay_launch_verbatim`.
- Interactive terminal launch: the L2 test (macOS and Linux).

## 9. macOS, Linux, native Windows, and WSL2 evidence satisfies the platform matrix, including native Windows directory materialization — **unmet**

| Environment | Status | Evidence or blocker |
|---|---|---|
| macOS | met | Phase 11: L2 overlay test 20/20 stress; all L1 contracts. Phase 12 gates below. |
| Linux | met | Phase 11, `build-linux` scratch clone: L2 overlay test pass; overlay L1/unit filter 123/123. |
| WSL2 | **unmet** | `build-win` reset SSH at key exchange (VHDX on the full `W:`); CI `claudine-cli/wsl2-ubuntu/L2` is an accepted gap. L1 runs in CI. |
| Native Windows | **unmet** | `build-win-native` `W:` had 0 GB free. The Windows L2 is compile-verified only; it now names its Codex source (`CODEX_HOME`) and overlay storage (`CLAUDINE_OVERLAY_DIR`) inside the fixture, so it no longer depends on the known-folder home (review-2 finding 2), but it has not yet run natively. Phase 5 copy-mode unit tests (26 passed on `build-win-native`) are the only native materialization evidence. Needs a native run once `W:` has space. |

Not re-attempted in Phase 12: the blockers are host capacity and a design
decision, neither of which a documentation phase can clear.

## 10. User-facing diagnostics never infer broken credentials from an isolated provider/config lookup and never expose secret values

- `L1::a_removed_api_key_and_an_overlay_failure_are_distinct_diagnostics`: the
  removed-API-key notice names the variable and `--include` and omits the
  secret; the overlay failure mentions neither the key, `--include`, nor the
  word "credential".
- `cli/src/output/error_walker/tests.rs::provider_overlay_diagnostics_render_through_the_effective_walk`
  rejects misleading wording in both rendered codes.
- The unsupported code projects the selector *name* only
  (`lib/src/diagnostics/registry.rs`, Invariant 8 comment).

## 11. Repo-isolation, MCP, architecture, CLI, and Claudine skill documentation describe the new provider-overlay contract

Phase 12, detailed in the implementation log:

- `docs/topics/repo-isolation.md`: rewritten. Adds the four-concern
  distinction table, failure behavior, the selector/shape/source-root masking
  table, the refused-provider table, and the Codex and Claude special cases.
- MCP: `docs/topics/mcp-mode.md` → Runtime Injection (and skill mirror);
  `docs/research/mcp/codex.md` → Runtime Injection corrected.
- Architecture: skill `architecture.md` (module tree, overlay paragraph),
  `docs/topics/provider-metadata.md` → Provider Overlay,
  `docs/topics/how-to-create-a-new-provider.md`, `docs/topics/execution-flow.md`,
  `docs/pipeline.md`, `docs/topics/building-an-agent-wrapper.md`.
- CLI: `--repo` help text (Phase 9), skill `cli-reference.md`,
  `docs/topics/wrapped-execution-switches.md`, `cli/README.md`, `lib/README.md`,
  `README.md`.
- Skill: `SKILL.md`, `composition.md`, `system-prompt.md`, `mcp-mode.md`,
  `timeline.md`.
- Checkpoint grep: see "Checkpoint grep" below.

## 12. `just test`, `just test-l2`, and `just lint` pass in the `claudine` package area, with real-provider tests remaining opt-in

**unmet**, only because of two L2 failures outside this fix. Run on macOS
from `claudine/` after every Phase 12 change:

| Gate | Result |
|---|---|
| `just test --no-fail-fast` | **7140 passed, 9 skipped**, exit 0 |
| `just lint` | exit 0, no warnings |
| `just test-l2 --no-fail-fast` (claudine-cli, self-spawn `-j 14`) | **224/226**. All overlay L2 tests pass, including `level2_provider_overlay_capture` and both MCP resume rows. |
| `BISCUIT_L2_THREADS=4 just _test_l2 claudine-gen --features terminal-tests` | 3/3. Run separately because the recipe stops after the cli half fails. |

The two failures are
`level2_lifecycle_control::{level2_shipped_implement_plan_supplied_commit_message_runs_exact_commit_branch,
level2_shipped_implement_plan_unset_commit_message_reaches_provider_and_auto_branch}`:
"subtree-compose strict mode: unknown root". They come from the uncommitted
`cli/tests/fixtures/shipped_implement_route/_implement/implement-plan.md` edit
from other in-flight work, which still uses a bare `{{area}}`. No overlay is
involved. Same tests, same cause as Phase 11 on macOS and Linux.

Real-provider tests: the overlay suite adds none to L1 or L2. Live-provider
targets stay behind the `real-tests` feature (`docs/dependencies.md` →
Rendezvous Local IPC, last bullet).

## Checkpoint grep

`grep -rni "shadow home\|shadow_home" claudine/docs claudine/cli/src claudine/lib/src .claude/skills/claudine`
does **not** return only `_completed` references. Every remaining hit was read
and left deliberately:

- `docs/research/system-prompt/{antigravity,gemini,kimi,pi,_fleet}.md` and
  `_schema.yaml`: provider research describing hypothetical wrapper options,
  and `shadow_home_file`, a research-schema enum value that `claudine-gen`
  still maps (`gen/src/emit/execution_prompting.rs:208`). Changing it is a
  research-schema change, not documentation.
- `docs/topics/system-prompt.md`, `lib/src/provider/system_prompt.rs`, and the
  skill `system-prompt.md`: the `ShadowHomeFile` variant name, which now
  documents that it is retained only for that enum and never delivers.
- `lib/src/provider_overlay/plan.rs:4`: describes what the plan replaced.
- `lib/src/mcp/inject.rs:696`: asserts a message does *not* say "shadow HOME".
- Skill `timeline.md`: the history entry for this fix.
