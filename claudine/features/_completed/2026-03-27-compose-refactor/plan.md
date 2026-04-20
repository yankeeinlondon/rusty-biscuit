# Compose Refactor Implementation Plan

## Context

Composition currently exists behind six partially overlapping entrypoints that have drifted apart architecturally. Top-level `claudine compose` uses a reduced execution path (`run_provider_composition`) that bypasses harness, MCP, structured streaming, handler-driven recovery, and wrapper reporting. Meanwhile `claudine <agent> --compose` has full wrapper-grade behavior but reads harness properties from raw source frontmatter instead of effective composed frontmatter. The result is two incompatible execution implementations and a broken invariant documented in the spec, tech design, and drift analysis.

This plan reduces composition to two canonical commands sharing one wrapper-grade execution pipeline, eliminating the drift permanently.

**Primary inputs:**
- `claudine/features/2026-03-27-compose-refactor/spec.md`
- `claudine/features/2026-03-27-compose-refactor/tech-design.md`

---

## Phase 1: Expand Library Types and Prepare Functions

**Goal:** Add `PreparedComposition` with `effective_frontmatter` and closure state. New types are additive -- existing callers unchanged.

### `claudine/lib/src/composition/types.rs`

Add alongside existing types (do not remove `PreparedPrompt` yet):

```rust
pub struct PreparedComposition {
    pub mode: CompositionMode,
    pub resolved_path: PathBuf,
    pub prompt: String,
    pub effective_frontmatter: serde_json::Value,
    pub effective_agent_hint: Option<serde_json::Value>,
    pub closure: CompositionClosurePlan,
}

pub enum CompositionClosurePlan {
    Direct,
    Inline(InlineClosurePlan),
}

pub struct InlineClosurePlan {
    pub original_document_text: String,
    pub original_frontmatter_hash: u64,
    pub original_body_hash: u64,
    pub managed_fields: BTreeSet<String>,
}

pub struct CompositionExecutionRequest {
    pub mode: CompositionMode,
    pub file_ref: String,
    pub prepared: PreparedComposition,
    pub explicit_provider: Option<Provider>,
    pub excluded: BTreeSet<Provider>,
    pub session_interactive: bool,
    pub silent: bool,
}
```

### `claudine/lib/src/composition/prepare.rs`

Add two new functions (keep old ones for now):

- `prepare_direct_composition(source: &ResolvedCompositionSource) -> Result<PreparedComposition>`
  - Compose entire document via Darkmatter
  - Extract `effective_frontmatter` from composed `Markdown.frontmatter()` -> `serde_json::Value`
  - Extract `effective_agent_hint` from composed frontmatter `agent` key
  - Set `closure: CompositionClosurePlan::Direct`

- `prepare_inline_composition(source: &ResolvedCompositionSource, repo_root: Option<&Path>) -> Result<PreparedComposition>`
  - Require string `prompt` in frontmatter
  - Build temp markdown (frontmatter + prompt as body), compose through Darkmatter
  - Extract `effective_frontmatter` and `effective_agent_hint` from composed state
  - Capture hashes via `source.markdown.hash_frontmatter(false)` and `source.markdown.hash_body(false)`
  - Build `InlineClosurePlan` with `managed_fields: BTreeSet::from(["last_updated".into()])`
  - Append updated guardrails (new contract: return body only, don't edit file)

### `claudine/lib/src/composition/guardrails.rs`

Update `DEFAULT_GUARDRAILS` to new inline prompt contract:
- "Return the replacement Markdown body content only"
- "Do not include frontmatter delimiters or frontmatter content"
- "Do not edit the source file directly"

### `claudine/lib/src/composition/mod.rs`

Add re-exports for all new types and functions.

### Test boundary
- Unit tests for both new prepare functions verifying `effective_frontmatter` is from composed state (not raw)
- Existing prepare tests still pass

---

## Phase 2: Update Provider Selection Logic

**Goal:** New precedence rules. Remove `AGENT` env. Add single-installed-provider and config favorite.

### `claudine/lib/src/composition/select.rs`

Add `select_provider_v2` with new precedence:
1. `explicit_provider` -> `SelectionReason::ExplicitProvider`
2. Single installed candidate after exclusion -> new `SelectionReason::SingleInstalled`
3. `prepared.effective_agent_hint` (from composed frontmatter) -> `FrontmatterHint`
4. `favorite: Option<Provider>` from config -> new `SelectionReason::ConfigFavorite`
5. Return `Err(InteractiveSelectionRequired)`

Config favorite loaded from: `HookerConfig.settings.linking` (`Option<LinkingSettings>`) -> `.preference: Vec<Provider>` (first entry = favorite). Defined in `claudine/lib/src/events/config.rs:83-91`. Loaded via `claudine::dispatch::loader::load_config(None, repo_root)`.

Keep old `select_provider`/`select_provider_with_env` for now.

### `claudine/lib/src/composition/types.rs`

Add `SelectionReason::SingleInstalled` and `SelectionReason::ConfigFavorite`.

### `claudine/lib/src/composition/error.rs`

- Update `InteractiveSelectionRequired` message (remove "set AGENT env" wording)
- Add `InlineInteractiveUnsupported` variant
- Add `InvalidInlineResponse(String)` variant

### Test boundary
- Unit tests for all precedence levels, exclusion filtering, ambiguous hints
- Old selection tests still pass

---

## Phase 3: Wrapper-Grade Composition Executor

**Goal:** Create `wrap/composition.rs` -- the single execution pipeline both commands delegate to. This is the core architectural change.

### Create `claudine/cli/src/commands/wrap/composition.rs`

One public function:
```rust
pub(crate) fn execute_composition_request(
    request: CompositionExecutionRequest,
    verbose: u8,
) -> Result<i32>
```

Implementation sequence:
1. Detect installed providers (`InstalledAiClients::new()`)
2. Load config favorite from `load_config(None, repo_root)` -> `settings.linking.preference[0]`
3. Select provider via `select_provider_v2`. Handle `InteractiveSelectionRequired` and `AgentHintAmbiguous` with inquire chooser. Fail cleanly without TTY.
4. For `inline-compose -i`: check provider can capture final assistant message. Bail with `InlineInteractiveUnsupported` if not.
5. Acquire wrapper profile: `profile::profile_for_provider(selected.provider)`
6. Resolve binary: `clients.path(provider.sniff_ai_cli())`
7. Build child args: `apply_prompt_body`, conditionally `apply_non_interactive` + `apply_non_interactive_defaults`
8. Build env: `env::build_child_env(...)`
9. `profile.validate_final_args(...)`
10. Harness detection using `request.prepared.effective_frontmatter` (THE key fix):
    ```rust
    if harness::has_harness_properties(&request.prepared.effective_frontmatter) {
        let plan = harness::parse_harness_plan_with_shell(
            &request.prepared.effective_frontmatter, ...
        )?;
        // evaluate pre_checks, capture snapshot
    }
    ```
11. MCP composition (reuse existing pattern from `wrap/mod.rs`)
12. Structured streaming decision
13. Execute: harness loop OR inline+closure OR direct child
14. For inline closure: extract body from provider result, call `closure::apply_inline_closure`

### Modify `claudine/cli/src/commands/wrap/mod.rs`

- Add `pub(crate) mod composition;`
- Make these `pub(crate)`: `run_harness_loop` (18 params), `HarnessPromptState`, `HarnessPromptMode`, `MaterializedHarnessPrompt`, `AttemptLaunch`, `LiveStreamSink`, `StructuredCodexOutput`, `StructuredSummaryDetails`, `WrapperHarnessPermissionProbe`, `CachedHarnessLoopContext`
- Make helper functions `pub(crate)`: `rewrite_markdown_preserving_frontmatter`, `split_frontmatter_parts`, `upsert_last_updated_in_frontmatter`, stream/reporting helpers as needed

### Test boundary
- Integration test: `claudine compose file.md` through new pipeline using mock provider
- Verify harness is reachable (frontmatter with `post_checks`)

---

## Phase 4: Thin Command Entrypoints

**Goal:** Replace `compose.rs` internals with thin request builders delegating to `execute_composition_request`.

### Rewrite `claudine/cli/src/commands/compose.rs`

New `ComposeArgs`:
```rust
pub struct ComposeArgs {
    #[arg(short = 'i', long)]
    pub interactive: bool,
    #[arg(long = "exclude", value_name = "PROVIDER")]
    pub exclude: Vec<String>,
    #[arg(long)]
    pub silent: bool,
    #[arg(long, group = "provider_select")]
    pub claude: bool,
    #[arg(long, group = "provider_select")]
    pub codex: bool,
    #[arg(long, group = "provider_select")]
    pub gemini: bool,
    #[arg(long, group = "provider_select")]
    pub opencode: bool,
    #[arg(long, group = "provider_select")]
    pub qwen: bool,
    #[arg(value_name = "FILE")]
    pub file: String,
}
```

New `InlineComposeArgs` -- same shape.

`run_compose(args, verbose)`:
- Resolve -> `prepare_direct_composition` -> build `CompositionExecutionRequest` -> `execute_composition_request`

`run_inline_compose(args, verbose)`:
- Resolve -> validate permissions -> `prepare_inline_composition` -> build request -> execute

Remove entirely: `run_compose_inner`, `execute_composition`, `run_provider_composition`, `load_provider_preferences`, `interactive_select`, `interactive_select_from`, `ComposeSubcommand`, `InlineArgs`, old `ComposeInlineArgs`.

### Modify `claudine/cli/src/args.rs`

```rust
Compose(commands::compose::ComposeArgs),
#[command(name = "inline-compose")]
InlineCompose(commands::compose::InlineComposeArgs),
```

Remove `ComposeInline` variant and `compose-inline` command name.

### Modify `claudine/cli/src/main.rs`

Route `Commands::InlineCompose(args) => commands::compose::run_inline_compose(args, cli.verbose)`

### Test boundary
- `claudine compose file.md` works end-to-end
- `claudine inline-compose file.md` works end-to-end
- `claudine compose --codex file.md` selects Codex explicitly
- `claudine compose-inline` -> unknown command error

---

## Phase 5: Remove Wrapper Composition Flags

**Goal:** Remove `--compose`, `--frontmatter-prompt`, `--prompt-file` from `WrapperArgs`. Canonical commands are live; old flags are dead.

### Modify `claudine/cli/src/commands/wrap/mod.rs`

Remove from `WrapperArgs`:
- `prompt_file: Option<String>` / `-p`
- `frontmatter_prompt: Option<String>` / `--fp`
- `compose: Option<String>`

Remove all code blocks gated on these flags:
- Prompt-file pipeline (~lines 799-862)
- Frontmatter-prompt pipeline (~lines 864-917)
- Chained composition pipeline (~lines 919-953)
- `has_composition_prompt` variable and conditionals
- `prompt_file_dry_run` variable and paths
- `inline_composition_source` variable and entire inline post-execution block (~200 lines)
- `chained_composition` / `chained_composition_source` variables
- `HarnessPromptMode::Inline`, `HarnessPromptMode::Compose` variants (if no composition prompts remain in wrapper, simplify or remove `HarnessPromptState` from the wrapper path)

### Delete `claudine/cli/src/commands/wrap/prompt_file.rs`

Entire file -- `--prompt-file` is removed per spec.

Remove `pub(crate) mod prompt_file;` from `wrap/mod.rs`.

### Modify `claudine/cli/src/output.rs`

Remove `PromptFileDryRunInfo` import and all `prompt_file_info` / `log_prompt_file_dry_run` code.

### Test boundary
- `claudine codex --compose file.md` -> unknown flag error
- `claudine claude --frontmatter-prompt file.md` -> unknown flag error
- `claudine claude -p file.md` -> unknown flag error
- Normal wrapper passthrough still works

---

## Phase 6: Clean Up Old Types and Aliases

**Goal:** Remove deprecated types, old functions, old command aliases.

### `claudine/lib/src/composition/types.rs`
- Remove `PreparedPrompt` (replaced by `PreparedComposition`)
- Remove old `CompositionRequest` (replaced by `CompositionExecutionRequest`)
- Remove `SelectionReason::EnvironmentOverride` and `SelectionReason::PreferenceFallback`

### `claudine/lib/src/composition/select.rs`
- Remove `select_provider` and `select_provider_with_env`
- Rename `select_provider_v2` -> `select_provider`

### `claudine/lib/src/composition/prepare.rs`
- Remove `prepare_inline_prompt` and `prepare_chained_prompt`
- Rename `prepare_direct_composition` -> `prepare_direct`
- Rename `prepare_inline_composition` -> `prepare_inline`

### `claudine/lib/src/composition/error.rs`
- Remove `ProviderRunFailed` (provider failure after launch no longer triggers selection fallback)

### `claudine/lib/src/composition/mod.rs`
- Update all re-exports

### `claudine/cli/src/commands/compose.rs`
- Remove any remaining stubs (`run_compose_inline`, `ComposeSubcommand`, `InlineArgs`, old `ComposeInlineArgs`)

### Test boundary
- Library tests updated to new names
- `cargo build -p claudine -p claudine-cli` clean with no dead code warnings

---

## Phase 7: Move Inline Rewrite to `composition::closure`

**Goal:** Centralize closure logic in the library for independent testing.

### Create `claudine/lib/src/composition/closure.rs`

Move from `wrap/mod.rs`:
- `rewrite_markdown_preserving_frontmatter` -> rename `rewrite_inline_document`
- `split_frontmatter_parts` (private helper)
- `upsert_last_updated_in_frontmatter` (private helper)
- `detect_newline`, `rewrite_last_updated_line`, `trim_line_ending` (private helpers)

Add new public functions:

```rust
/// Strip accidental frontmatter from provider output, validate non-empty.
pub fn extract_replacement_body(
    provider_output: &str,
) -> Result<String, CompositionError>

/// Validate replacement body, reconstruct document, write atomically.
pub fn apply_inline_closure(
    plan: &InlineClosurePlan,
    replacement_body: &str,
    target_path: &Path,
) -> Result<(), CompositionError>
```

`apply_inline_closure`:
1. Validate non-empty body -> `InvalidInlineResponse` if empty
2. Hash replacement body, compare with `plan.original_body_hash` -> error if unchanged
3. Call `rewrite_inline_document(plan.original_document_text, replacement_body, today)`
4. Atomic write via `claudine::config::atomic::atomic_write`

### Update `claudine/cli/src/commands/wrap/composition.rs`
- Replace inline closure logic with calls to `claudine::composition::closure::*`

### Remove from `claudine/cli/src/commands/wrap/mod.rs`
- The moved private functions and their unit tests

### `claudine/lib/src/composition/mod.rs`
- Add `mod closure;` and re-exports

### Test boundary
- Unit tests in `closure.rs` for body extraction, frontmatter preservation, unchanged body rejection
- Migrate existing `wrap/mod.rs` rewrite tests
- `claudine inline-compose` end-to-end still works

---

## Phase 8: Documentation and Tests

**Goal:** Docs stop describing old behavior. Integration tests cover all acceptance criteria.

### Rewrite `claudine/docs/topics/composition.md`
- Two canonical commands only
- Retirement of wrapper switches
- Provider selection precedence (explicit -> single-installed -> frontmatter hint -> config favorite -> interactive -> error)
- `--interactive` controls provider session mode, not selection
- Both commands use wrapper-grade execution
- Inline closure: Claudine rewrites the file

### Update `claudine/README.md`
- New canonical commands in composition section
- Remove references to `--compose`, `--frontmatter-prompt`, `--prompt-file`

### Update CLI help text in `claudine/cli/src/args.rs`

### Migrate tests in `claudine/cli/tests/wrap_commands.rs`

Rewrite existing (they use retired `--frontmatter-prompt` and `--compose` flags):
1. `codex_frontmatter_prompt_validates_agent_file_update` -> `inline_compose_validates_file_update`
2. `codex_frontmatter_prompt_restores_original_frontmatter_layout_after_tamper` -> `inline_compose_preserves_frontmatter`
3. `codex_frontmatter_prompt_does_not_overwrite_file_on_failure` -> `inline_compose_no_overwrite_on_failure`
4. `codex_frontmatter_prompt_retries_inline_recovery` -> `inline_compose_harness_retry`
5. `codex_compose_response_validation_uses_captured_legacy_output` -> `compose_uses_wrapper_grade_execution`

Add new:
6. `compose_effective_frontmatter_enables_harness` -- raw frontmatter has no harness keys, composed adds `post_checks` via Darkmatter
7. `compose_interactive_preserves_composition_first` -- `-i` still prepares prompt first
8. `explicit_provider_flag_bypasses_chooser` -- `--codex` selects directly
9. `ambiguous_hint_fails_without_tty` -- `agent: "c"` matching multiple, no TTY -> error
10. `no_cross_provider_retry_after_launch` -- non-zero exit, no automatic rerun

---

## Acceptance Criteria Mapping

| Criterion | Phase |
|-----------|-------|
| Only `compose` and `inline-compose` remain | 4, 5, 6 |
| No reduced execution implementation | 3, 4 |
| Both modes inherit wrapper features | 3 |
| Harness uses effective composed frontmatter | 1, 3 |
| Inline rewrite is deterministic, preserves frontmatter | 1, 7 |
| Provider selection is deterministic | 2 |
| No automatic cross-provider retry after launch | 2, 3 |
| Docs and tests reflect new contract | 8 |

## Key Risk: Effective Frontmatter Threading

The single most important invariant: once `PreparedComposition` is built, **all** downstream code reads `prepared.effective_frontmatter`. No function may re-read source file frontmatter for harness detection, provider selection, MCP resolution, or handler parsing. If any step falls back to raw source state, the drift returns.

## Critical Files

| File | Action |
|------|--------|
| `claudine/lib/src/composition/types.rs` | Expand |
| `claudine/lib/src/composition/prepare.rs` | Expand |
| `claudine/lib/src/composition/select.rs` | Rewrite |
| `claudine/lib/src/composition/error.rs` | Expand |
| `claudine/lib/src/composition/guardrails.rs` | Update |
| `claudine/lib/src/composition/closure.rs` | **Create** |
| `claudine/lib/src/composition/mod.rs` | Update exports |
| `claudine/cli/src/commands/compose.rs` | Rewrite |
| `claudine/cli/src/commands/wrap/composition.rs` | **Create** |
| `claudine/cli/src/commands/wrap/mod.rs` | Reduce (remove ~500 lines) |
| `claudine/cli/src/commands/wrap/prompt_file.rs` | **Delete** |
| `claudine/cli/src/args.rs` | Update commands |
| `claudine/cli/src/main.rs` | Update dispatch |
| `claudine/cli/src/output.rs` | Remove prompt_file output |
| `claudine/docs/topics/composition.md` | Rewrite |
| `claudine/README.md` | Update |
| `claudine/cli/tests/wrap_commands.rs` | Migrate + add tests |

## Verification

After all phases:
```bash
cd claudine && just test      # all unit + integration tests pass
cd claudine && just lint       # no warnings, no dead code
cd claudine && just build      # clean build
claudine compose --help        # shows new flags
claudine inline-compose --help # shows new flags
claudine codex --help          # no --compose, --frontmatter-prompt, --prompt-file
```
