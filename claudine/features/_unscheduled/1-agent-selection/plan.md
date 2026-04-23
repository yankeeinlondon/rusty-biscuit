---
phases: 7
created: 2026-04-19
start_phase: 1
---

# Agent Selection Execution Plan

Source documents:

- `claudine/features/_unscheduled/agent-selection/spec.md`
- `claudine/features/_unscheduled/agent-selection/tech-design.md`

Validated current seams:

- `claudine/lib/src/composition/prepare.rs` still stores raw `effective_agent_hint: Option<Value>`.
- `claudine/lib/src/composition/select.rs` still auto-selects a single installed provider and only understands singular `agent`.
- `claudine/cli/src/commands/wrap/composition.rs` owns both favorite-agent lookup and the interactive picker.
- `claudine/cli/src/commands/wrap/sequence.rs` front-loads shell approvals, but does not front-load per-step agent/model review.
- `claudine/lib/src/config/claudine_config.rs` still requires `preferred_agent: Provider`.
- `claudine config` is currently a TUI entrypoint, not a setter command surface.

## Phase Index

| Phase | Outcome | Depends on |
| --- | --- | --- |
| 1 | Config model and mutation surfaces support optional favorite agent and model overrides | none |
| 2 | Preparation layer emits typed `agent` and `model` hints | 1 |
| 3 | Pure provider/model resolver matches TTY and non-TTY rules | 2 |
| 4 | Model catalog cache and override service validate frontmatter models | 1, 3 |
| 5 | `compose` and `inline-compose` use the new resolver end to end | 3, 4 |
| 6 | `sequence` front-loads step resolution and review before execution | 3, 4, 5 |
| 7 | Documentation and regression sweep land with the feature | 1-6 |

## Phase 1: Config Foundation

Outcome: Claudine can represent "no favorite agent", can store per-provider model overrides, and exposes a supported way to update the favorite agent after init.

Files:

- `claudine/lib/src/config/claudine_config.rs`
- `claudine/lib/src/dispatch/loader.rs`
- `claudine/cli/src/commands/init/mod.rs`
- `claudine/cli/src/commands/init_wizard.rs`
- `claudine/cli/src/commands/config_tui/**`
- `claudine/cli/src/args.rs`
- `claudine/cli/src/main.rs`

Steps:

- [ ] Change `ClaudineConfig.preferred_agent` from `Provider` to `Option<Provider>`, add `#[serde(default, alias = "favorite_agent")]`, and add `models: HashMap<Provider, ProviderModelOverride>`.
- [ ] Add `ProviderModelOverride` and `ModelOverrideMode` in the config layer with additive-list and explicit-replace forms.
- [ ] Update `Default`, config validation, and dispatch-loader merge behavior so user config may omit `preferred_agent`, while repo config still rejects `preferred_agent`, `favorite_agent`, and `models`.
- [ ] Update `init` and `init_wizard` so first-run setup can write an optional favorite agent and does not hard-fail when no providers are installed.
- [ ] Update the config TUI preferences tab so favorite agent is optional and can be cleared.
- [ ] Extend the `config` command surface to support `claudine config set favorite-agent <provider>` while preserving bare `claudine config` as the TUI entrypoint.

Parallelizable:

- Steps 4 and 5 can run in parallel after Step 1 lands.
- Step 6 can run in parallel with Steps 4 and 5 once the config schema is stable.

Validation checkpoint:

- `cargo test -p claudine config::claudine_config`
- `cargo test -p claudine dispatch::loader`
- `cargo test -p claudine-cli init`
- Manual smoke check:
  - `claudine config`
  - `claudine config set favorite-agent codex`

## Phase 2: Typed Selection Hints

Outcome: effective frontmatter is parsed once into typed agent/model hints, and wrong frontmatter types fail during preparation instead of during launch.

Files:

- `claudine/lib/src/composition/prepare.rs`
- `claudine/lib/src/composition/types.rs`
- `claudine/lib/src/composition/error.rs`

Steps:

- [ ] Add `EffectiveSelectionHints`, `AgentHint`, and `ModelHint` to `composition/types.rs`.
- [ ] Replace `PreparedComposition.effective_agent_hint` with `PreparedComposition.selection_hints`.
- [ ] Parse `agent` in `prepare_direct` and `prepare_inline` as either a single provider or an ordered provider list, preserving author order and fuzzy-matching through the existing `Provider` matcher.
- [ ] Parse `model` in `prepare_direct` and `prepare_inline` as either a single string or an ordered string list.
- [ ] Add early errors for unknown provider names and wrong JSON types via new `CompositionError` variants instead of late resolver failures.
- [ ] Keep `effective_frontmatter` unchanged so harness, MCP, lifecycle, and closure paths continue to read the composed document state they already expect.

Validation checkpoint:

- `cargo test -p claudine composition::prepare`
- `cargo test -p claudine composition::error`

## Phase 3: Pure Resolution Engine

Outcome: library code can resolve provider and model without terminal IO and can also produce picker/review plans for CLI callers.

Files:

- `claudine/lib/src/composition/select.rs`
- `claudine/lib/src/composition/types.rs`
- `claudine/lib/src/composition/error.rs`
- `claudine/lib/src/composition/mod.rs`

Steps:

- [ ] Add `ResolutionMode`, `InstalledProviderSnapshot`, `ResolvedExecutionTarget`, `ProviderResolutionReason`, `ModelResolutionReason`, and the picker/review plan structs described in the design.
- [ ] Replace the current `select_provider(...)` flow with explicit TTY and non-TTY resolution paths that remove the old `SingleInstalled` shortcut.
- [ ] Keep explicit `--<provider>` flags higher priority than exclusions and higher priority than all other signals.
- [ ] Implement picker-plan construction using `PROVIDERS_DISPLAY_ORDER`, frontmatter ordering, and optional favorite-agent defaulting rules.
- [ ] Implement model precedence exactly as designed: CLI `--model`, provider-specific env, `MODEL`, frontmatter `model`, provider default.
- [ ] Add the OpenCode non-TTY hard error when the final model chain still resolves to `None`.
- [ ] Extend `CompositionExecutionRequest` with `resolved_target: Option<ResolvedExecutionTarget>` and update resolver-facing reason enums so downstream logging can report how the target was chosen.

Parallelizable:

- Picker-plan construction and model-resolution work can proceed in parallel after the new core structs are added.

Validation checkpoint:

- `cargo test -p claudine composition::select`
- Focused assertions for:
  - no single-installed auto-selection
  - TTY picker ranking/default index
  - non-TTY frontmatter list resolution
  - favorite-agent absence
  - provider env precedence
  - OpenCode no-model hard error

## Phase 4: Model Catalog Service

Outcome: frontmatter model hints are validated against cached provider catalogs with user overrides and graceful fallback when a source is unavailable.

Files:

- `claudine/lib/src/model_catalog/mod.rs`
- `claudine/lib/src/model_catalog/cache.rs`
- `claudine/lib/src/model_catalog/config.rs`
- `claudine/lib/src/model_catalog/provider_sources.rs`
- `claudine/lib/src/model_catalog/service.rs`
- `claudine/lib/src/config/claudine_config.rs`

Steps:

- [ ] Add the `model_catalog` module tree and a service API that answers one question: "is this model valid for this provider right now?"
- [ ] Implement cache read/write and stale-cache fallback behavior under `~/.claudine/cache/models/<provider>.json`.
- [ ] Implement override merging for additive and replace modes using the new config types from Phase 1.
- [ ] Add Codex and Claude sources by reading generated model enums already shipped in `unchained-ai/lib`.
- [ ] Add OpenCode model sourcing by shelling out to `opencode models` and parsing the result into normalized model IDs.
- [ ] Add Qwen catalog sourcing by filtering and normalizing the same OpenCode output.
- [ ] Leave Gemini, Kimi, and Goose on "user overrides only" in v1, and make missing catalog data cause frontmatter `model` to be skipped rather than treated as an error.
- [ ] Wire the resolver from Phase 3 to use the merged catalog when evaluating frontmatter `model`.

Parallelizable:

- Codex/Claude source work and OpenCode/Qwen source work are parallelizable after the cache/service contract exists.

Validation checkpoint:

- `cargo test -p claudine model_catalog`
- Focused assertions for:
  - additive override merge
  - replace override merge
  - stale cache read when refresh fails
  - OpenCode parser coverage
  - Qwen filtered catalog coverage

## Phase 5: Compose And Inline Wiring

Outcome: `compose` and `inline-compose` resolve provider/model once, prompt correctly in TTY mode, never prompt in non-TTY mode, and launch wrappers with fully resolved targets.

Files:

- `claudine/cli/src/commands/wrap/composition.rs`
- `claudine/cli/src/commands/wrap/selection_ui.rs`
- `claudine/cli/src/commands/compose.rs`
- `claudine/lib/src/composition/types.rs`

Steps:

- [ ] Extract picker rendering from `wrap/composition.rs` into a new `selection_ui.rs` helper that accepts typed picker plans from the library.
- [ ] Build the installed-provider snapshot once at command start using `InstalledAiClients::is_installed(provider.sniff_ai_cli())`.
- [ ] Replace the current `select_provider(...)` plus fallback `interactive_select(...)` flow with the new mode-aware resolver and picker-plan flow.
- [ ] Pass `ResolvedExecutionTarget` through `CompositionExecutionRequest` so launch no longer depends on wrapper-local provider/model guessing.
- [ ] Extend `composition_dispatch_context(...)` with `provider_selection_reason`, `resolved_model`, `model_selection_reason`, and `selection_mode`.
- [ ] Remove the current ad hoc `load_config_favorite(...) -> Some(config.preferred_agent)` assumption so missing favorites remain valid.

Parallelizable:

- UI extraction and dispatch-context/reporting changes can run in parallel once request/target types are finalized.

Validation checkpoint:

- `cargo test -p claudine-cli wrap_commands -- --nocapture`
- Focused CLI coverage for:
  - `compose` non-TTY structured error text
  - `inline-compose` non-TTY structured error text
  - explicit provider bypassing picker
  - exclusions only affecting automatic selection, not explicit flags

## Phase 6: Sequence Front-Loading And Review

Outcome: `sequence` resolves every step's agent/model before shell preflight or execution, supports one interactive review/sign-off in TTY mode, and fails the whole run up front in non-TTY mode.

Files:

- `claudine/cli/src/commands/wrap/sequence.rs`
- `claudine/cli/src/commands/wrap/selection_ui.rs`
- `claudine/lib/src/composition/select.rs`
- `claudine/lib/src/composition/error.rs`
- `claudine/lib/src/composition/types.rs`

Steps:

- [ ] Reorder sequence preparation into: build overlays, prepare each step, resolve provider/model for each step, review or validate, shell-preflight finalized steps, then execute.
- [ ] Add `SequenceStepDraft` and `SequenceSelectionFailure` support so unresolved steps can be surfaced before execution starts.
- [ ] Add a non-TTY aggregate failure path using `CompositionError::SequenceSelectionFailed { failures }`.
- [ ] Build the TTY review MVP: static summary, row selection, provider edit via `inquire::Select`, model edit via `Select` or free-text input, and one final confirmation.
- [ ] Enforce locks when an explicit provider flag or explicit model flag was supplied on the CLI.
- [ ] Carry the reviewed or resolved per-step target into `CompositionExecutionRequest.resolved_target` so execution reuses the front-loaded decision instead of resolving again mid-run.

Parallelizable:

- The aggregate failure type work and the interactive review UI can proceed in parallel once per-step draft types exist.

Validation checkpoint:

- `cargo test -p claudine-cli sequence_cli`
- Focused assertions for:
  - no step executes when any non-TTY step fails selection
  - per-step resolved target propagation
  - provider/model lock behavior
- Manual smoke check with a multi-step sequence in both TTY and non-TTY modes.

## Phase 7: Documentation And Regression Sweep

Outcome: the shipped behavior is documented, test coverage reflects the new semantics, and the repo’s Claudine skill docs stay current.

Files:

- `claudine/docs/topics/composition.md`
- `claudine/cli/README.md`
- `claudine/.claude/skills/claudine/SKILL.md`
- `claudine/features/_unscheduled/agent-selection/plan.md`

Steps:

- [ ] Update user-facing composition docs to call out the TTY vs non-TTY split, frontmatter `agent` and `model`, OpenCode's non-TTY model requirement, and Roo's continued exclusion from composition.
- [ ] Update CLI help and README examples to show the new favorite-agent semantics and the new `config set favorite-agent` path.
- [ ] Update the Claudine skill doc so future agents have the right selection workflow and config semantics.
- [ ] Run a full regression sweep across library and CLI tests after the docs are aligned with the final behavior.

Validation checkpoint:

- `cargo test -p claudine`
- `cargo test -p claudine-cli`
- Manual smoke matrix:
  - `compose` in TTY with no explicit provider
  - `compose` in non-TTY with frontmatter `agent`
  - `compose` in non-TTY with only favorite agent
  - `compose` OpenCode in non-TTY with and without `OPENCODE_MODEL`
  - `sequence` in TTY with per-step edits
  - `sequence` in non-TTY with one intentionally unresolved step
