# Arg-Forwarding Spec: `--` Boundary Bug + Cross-Provider Pipeline Issues

**Date**: 2026-04-15
**Scope**: `claudine/cli/src/commands/wrap/` — `mod.rs`, `profile.rs`, `composition.rs`
**Status**: Specification for fix — no changes made

---

## Issue 1: Post-`--` Flag Neutralization (Critical)

### Summary

In `run_provider_wrapper_inner` (`mod.rs`), the pipeline applies flags to `child_args` in a fixed order: `prompt_delivery` → `apply_entrypoint` → `apply_model` → `apply_structured_stream`. Any provider whose `prompt_delivery` inserts a `--` separator (currently only OpenCode) has all subsequent flag insertions rendered invisible to the child process's argument parser, because everything after `--` is positional.

### Affected providers

| Provider | `prompt_delivery` shape | Flags after `--`? | Impact |
|---|---|---|---|
| **OpenCode** | `AppendArgs(["--", prompt])` | `--model`, `--format json`, `--dangerously-skip-permissions`, `--system`, `--output-format` | **Critical** — all pushed flags are positionalized |
| Claude | `Stdin(prompt)` / `AppendArgs([prompt])` | No | None |
| Codex | `Stdin(prompt)` / `InsertArgs` | No | None |
| Gemini | `AppendArgs(["--prompt", prompt])` | No | None |
| Kimi | `Stdin(prompt)` / `AppendArgs(["--prompt", prompt])` | No | None |
| Qwen | `AppendArgs(["--prompt", prompt])` | No | None |
| Goose | `InsertArgs` / `AppendArgs(["run", "-t", prompt])` | No | None |

### Root cause in `mod.rs` ordering

```
Line 738:  prompt_delivery       → child_args = ["--", "my prompt"]
Line 748:  apply_yolo_for_mode   → pushes --dangerously-skip-permissions AFTER "--"
Line 768:  apply_entrypoint      → inserts "run" at index 0 (before "--")
Line 770:  apply_non_interactive_flags
Line 780:  model resolution      → pushes --model AFTER "--"
Line 858:  apply_output_format   → pushes --format AFTER "--"
Line 1213: apply_structured_stream → pushes --format json AFTER "--"
```

### Why this is a class of bug, not just an OpenCode bug

Any future provider (or any modification to an existing provider) that uses `--` in `prompt_delivery` will silently break in the same way. The pipeline has no guard against inserting flags after a `--` boundary, and there is no assertion or test that validates the final argv is parseable.

### Trace: `claudine opencode --model some-model "my prompt"`

```
1. Clap parses --model → args.model = Some("some-model")
2. child_args = ["my prompt"]
3. extract_prompt_source → child_args = []
4. prompt_delivery → child_args = ["--", "my prompt"]
5. apply_entrypoint → child_args = ["run", "--", "my prompt"]
6. apply_model → child_args = ["run", "--", "my prompt", "--model", "some-model"]
7. apply_structured_stream → child_args = ["run", "--", "my prompt", "--model", "some-model", "--format", "json"]
```

Final argv: `opencode run -- "my prompt" --model some-model --format json`

`--model` and `--format` are positionalized. The `MODEL` env var partially compensates for `--model`, but `--format json` has no env var fallback.

### Why `--` exists

OpenCode's `prompt_delivery` uses `--` because composed prompts commonly start with `-`-prefixed tokens (bullet lists like `- some item`), which yargs rejects as unknown options. This is a legitimate concern and the separator must remain.

### Composition path is NOT affected

`composition.rs` correctly places `prompt_delivery` (line 557) AFTER `apply_structured_stream` (line 535) and all other flag insertions. The comment at `composition.rs:546-550` explicitly documents this ordering rationale.

### Proposed fix

**Option A (Recommended): Reorder the pipeline in `mod.rs` to match `composition.rs`.**

Move `prompt_delivery` to after all `apply_*` methods have completed. This makes the direct-wrap path consistent with the composition path and eliminates the boundary class of bug entirely:

```
1. apply_yolo_for_mode
2. apply_entrypoint
3. apply_non_interactive_flags
4. model resolution / apply_model
5. apply_output_format
6. apply_system_prompt
7. apply_structured_stream
8. prompt_delivery              ← moved to last
```

This requires `prompt_delivery` implementations to account for a populated arg list (which they already do — `InsertArgs` scans for entrypoint position, `AppendArgs` appends to end).


- **DECISION:** use Option A

**Option B: Insert flags before `--` in each provider.**

Each provider with a `--` boundary would need to scan for the `--` and insert before it. This is fragile and per-provider.

---

## Issue 2: `apply_structured_stream` Is Post-`--` for All Append-Based Providers

### Summary

Even if Issue 1 is fixed for OpenCode specifically, the general pattern of appending flags to the end of `child_args` is risky for any provider. Every `apply_structured_stream` implementation pushes flags to the end of the arg list. If a provider's `prompt_delivery` is ever changed to include `--`, the same class of bug appears.

### Current `apply_structured_stream` implementations

| Provider | Flags pushed | Position risk |
|---|---|---|
| Claude | `--print`, `--verbose`, `--output-format stream-json` | Low (no `--` in delivery) |
| Codex | `--json` | Low (no `--` in delivery) |
| Gemini | `--output-format stream-json` | Low (no `--` in delivery) |
| Kimi | `--print`, `--output-format stream-json` | Low (no `--` in delivery) |
| Qwen | `--output-format stream-json` | Low (no `--` in delivery) |
| OpenCode | `--format json` | **High** (after `--`) |
| Goose | N/A (no structured stream support) | N/A |

### Proposed fix

Pipeline reordering (Issue 1, Option A) resolves this as a natural consequence.

---

## Issue 3: `apply_model` Default Trait Implementation Is Append-Only

### Summary

The default `WrapperProfile::apply_model` (line 336-345) pushes `--model <value>` to the end of `child_args`. This is the same append pattern that causes Issue 1 for OpenCode. Two providers override this (OpenCode and Goose), but the default is risky for any provider that might add `--` in prompt delivery.

### Default implementation

```rust
fn apply_model(&self, args: &mut Vec<String>, ..., model: &str) -> Option<String> {
    args.push("--model".to_string());
    args.push(model.to_string());
    None
}
```

Providers using the default: Claude, Codex, Gemini, Kimi, Qwen. None currently use `--` in prompt delivery, so this is latent rather than active.

### Proposed fix

After pipeline reordering (Issue 1, Option A), append is safe because `prompt_delivery` runs last. No additional change needed beyond the reorder.

---

## Issue 4: OpenCode Model Resolution Has Duplicated Code Paths

### Summary

OpenCode model resolution is handled inline in both `mod.rs` (lines 776-848) and `composition.rs` (lines 373-446) with nearly identical logic. Both paths:

1. Call `resolve_opencode_model(cli_model)` 
2. Match on `CliSwitch`, `OpenCodeModelEnv`, `ConfigDefault`
3. Push `--model` to `child_args` if not already present
4. Set `MODEL` env var
5. Validate that a model exists for non-interactive mode

This is ~70 lines of duplicated code across two call sites.

### Proposed fix

Extract an `apply_opencode_model_resolution` function into `profile.rs` that takes `(child_args, env_overrides, cli_model, non_interactive)` and encapsulates the resolution + validation. Both `mod.rs` and `composition.rs` call it instead of inlining the logic.

---

## Issue 5: Goose `prompt_delivery` Has an Inconsistency

### Summary

Goose's `prompt_delivery` uses `InsertArgs` when `"run"` is found in the existing args but falls back to `AppendArgs(["run", "-t", prompt])` when it isn't. However, `apply_entrypoint` already inserts `"run"` at index 0 for non-interactive mode. In the `mod.rs` pipeline, `prompt_delivery` runs BEFORE `apply_entrypoint`, so the `"run"` entrypoint is NOT yet in `child_args`. This means Goose always takes the `AppendArgs` fallback path in the direct-wrap pipeline.

The result: if Goose's passthrough args somehow contain `"run"` from user input (not from `apply_entrypoint`), the insert position is calculated correctly. But in the normal flow, `prompt_delivery` always appends `["run", "-t", prompt]`, and then `apply_entrypoint` inserts another `"run"` at index 0.

**Actual argv**: `goose run run -t "my prompt"` — the `run` subcommand appears twice.

### Affected scenarios

This is masked because Goose doesn't currently support structured streaming and the `run` command may tolerate the duplication, but it's a latent correctness issue.

### Proposed fix

Pipeline reordering (Issue 1, Option A) resolves this: `apply_entrypoint` runs before `prompt_delivery`, so Goose's `prompt_delivery` will find `"run"` already in the arg list and use `InsertArgs` correctly.

---

## Issue 6: YOLO Flag Insertion Is Also Post-`--` for OpenCode

### Summary

`apply_yolo_for_mode` (line 751) runs before `apply_entrypoint` in `mod.rs`, which means `--dangerously-skip-permissions` is pushed to `child_args` after `prompt_delivery` has placed `--`. For OpenCode non-interactive:

```
prompt_delivery → child_args = ["--", "my prompt"]
apply_yolo_for_mode → child_args = ["--", "my prompt", "--dangerously-skip-permissions"]
apply_entrypoint → child_args = ["run", "--", "my prompt", "--dangerously-skip-permissions"]
```

`--dangerously-skip-permissions` is after `--` and thus positionalized. OpenCode may still honor it via other means, but it's the same class of bug.

### Proposed fix

Pipeline reordering (Issue 1, Option A).

---

## Issue 7: Pipeline Ordering Differs Between `mod.rs` and `composition.rs`

### Summary

The two execution paths have different orderings for the same logical operations:

| Operation | `mod.rs` line | `composition.rs` line |
|---|---|---|
| YOLO | 751 | 348 |
| Entrypoint | 768 | 365 |
| Non-interactive flags | 770 | 367 |
| Model resolution | 776 | 373 |
| Output format | 856 | 454 |
| System prompt | 863 | 480 |
| Structured stream | 1213 | 535 |
| **Prompt delivery** | **738** | **557** |

In `mod.rs`, prompt delivery is FIRST (before everything). In `composition.rs`, it's LAST (after everything). The composition path has the correct ordering; the direct-wrap path does not.

### Proposed fix

Unify the ordering. `composition.rs` should be the canonical ordering, and `mod.rs` should be refactored to match. Consider extracting the pipeline into a shared builder function in `profile.rs`.

---

## Issue 8: No Validation of Final Argv Correctness

### Summary

Neither path validates that the final `child_args` are well-formed. There is no check that:
- Flags intended as options appear before any `--` separator
- Required flags are not accidentally positionalized
- The child command will actually parse the args as intended

### Proposed fix

Add a debug-mode assertion or lint that, when `RUST_LOG=claudine=debug`, logs the final argv and warns if flags are detected after `--`. This catches regressions early without runtime overhead in production.

---

## DRY Opportunities (Cross-Provider)

### 1. Unified pipeline builder

Extract the shared pipeline logic from `mod.rs` and `composition.rs` into a `PipelineBuilder` struct in `profile.rs`:

```rust
struct PipelineBuilder<'a> {
    profile: &'a dyn WrapperProfile,
    child_args: Vec<String>,
    env_overrides: Vec<(String, String)>,
    // ... other state
}

impl<'a> PipelineBuilder<'a> {
    fn apply_yolo(&mut self, ...) -> Result<()> { ... }
    fn apply_entrypoint(&mut self, ...) { ... }
    fn apply_model(&mut self, ...) { ... }
    fn apply_output_format(&mut self, ...) { ... }
    fn apply_structured_stream(&mut self, ...) { ... }
    fn deliver_prompt(&mut self, ...) -> Result<()> { ... }
    fn build(self) -> (Vec<String>, Vec<(String, String)>) { ... }
}
```

Both call sites construct a `PipelineBuilder` and call the same methods in the same order. This eliminates the ordering divergence and the duplicated OpenCode model resolution.

### 2. `PromptDelivery::AppendArgs` with pre-boundary insertion

For providers that need `--` (currently only OpenCode), `PromptDelivery` could carry a hint about whether the provider uses a `--` separator. The pipeline could then insert flags before the `--` rather than after. This is less preferred than full pipeline reordering but would be a targeted fix.

### 3. Common `apply_structured_stream` patterns

Multiple providers push the same `--output-format stream-json` pair:

- Gemini: `["--output-format", "stream-json"]`
- Kimi: `["--print", "--output-format", "stream-json"]`
- Qwen: `["--output-format", "stream-json"]`

These could use a shared helper or a default `apply_structured_stream` implementation that takes the flag name and value.

### 4. Common `prompt_delivery` patterns

Several providers share identical `prompt_delivery` logic:

- **Stdin-when-non-interactive, append-when-interactive**: Claude, Kimi
- **AppendArgs with `--prompt`/`--prompt-interactive`**: Gemini, Qwen
- **Positional-after-entrypoint**: Codex, OpenCode

These could be expressed as reusable functions rather than per-provider methods with duplicated logic.

---

## Summary of Issues by Severity

| # | Issue | Severity | Fix |
|---|---|---|---|
| 1 | OpenCode post-`--` flag neutralization | **Critical** | Reorder pipeline |
| 5 | Goose double `run` subcommand | Medium | Reorder pipeline |
| 6 | OpenCode YOLO post-`--` | Medium | Reorder pipeline |
| 7 | Pipeline ordering divergence | Medium | Shared pipeline builder |
| 2 | `apply_structured_stream` append pattern | Low (latent) | Reorder pipeline |
| 3 | `apply_model` default append | Low (latent) | Reorder pipeline |
| 4 | OpenCode model resolution duplication | Low (DRY) | Extract shared function |
| 8 | No final argv validation | Low (DX) | Debug-mode assertion |

The pipeline reorder (Issue 1, Option A) resolves issues 1, 2, 3, 5, and 6 simultaneously. Issue 7 is then addressed by extracting the reordered pipeline into a shared builder (DRY opportunity 1).

---

## Proposed Implementation Order

1. **Reorder `mod.rs` pipeline** — move `prompt_delivery` to after all `apply_*` methods (fixes Issues 1, 2, 3, 5, 6)
2. **Extract OpenCode model resolution** into `profile.rs` (fixes Issue 4)
3. **Extract shared pipeline builder** (fixes Issue 7, DRY opportunity 1)
4. **Add debug-mode argv validation** (fixes Issue 8)
5. **Extract common delivery/structured-stream helpers** (DRY opportunities 3, 4)
