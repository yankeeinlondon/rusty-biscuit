# Implementation Plan: Arg-Forwarding Fixes

**Date**: 2026-04-15
**Scope**: `claudine/cli/src/commands/wrap/` — `mod.rs`, `profile.rs`, `composition.rs`
**Depends on**: `spec.md` (issues 1-8), `findings.md`

---

## Overview

The core fix is a pipeline reorder in `mod.rs` to match the already-correct ordering in `composition.rs`. This single change resolves issues 1, 2, 3, 5, and 6. Follow-up work extracts duplicated code into shared helpers (issues 4, 7) and adds a debug-mode argv validation guard (issue 8).

## Step 0: Add Regression Tests Before Any Changes

**Why first**: The current code has zero unit tests for the full arg-pipeline ordering. Without tests, we can't verify the reorder doesn't break anything.

**File**: `claudine/cli/src/commands/wrap/profile.rs` (test module at line 1993)

**Tests to add**:

1. **`test_opencode_non_interactive_args_order`** — Simulate the full direct-wrap pipeline steps in sequence for OpenCode non-interactive with `--model` and verify `--model` and `--format` appear BEFORE any `--` separator.

2. **`test_goose_non_interactive_no_duplicate_run`** — Simulate the pipeline for Goose non-interactive and verify `"run"` appears exactly once in the final argv.

3. **`test_all_providers_flags_before_double_dash`** — For every provider that uses `AppendArgs` with `--` in `prompt_delivery`, verify that after applying all `apply_*` methods + `prompt_delivery`, no flag (starting with `-`) appears after the first `--` in the final argv.

4. **`test_pipeline_mod_rs_matches_composition_rs`** — A meta-test: extract the ordered list of pipeline operations from both paths and assert they match (this will initially FAIL, and pass after step 1).

**Verification**: Run `cargo test -p claudine-cli test_opencode_non_interactive_args_order` etc. The ordering test should FAIL (confirming the bug exists), the others may need initial adjustment.

---

## Step 1: Reorder Pipeline in `mod.rs` (Fixes Issues 1, 2, 3, 5, 6)

**Confidence**: HIGH — `composition.rs` already uses this exact ordering and works correctly.

**File**: `claudine/cli/src/commands/wrap/mod.rs`
**Function**: `run_provider_wrapper_inner`

### Current ordering (lines 738-1196):

```
1. prompt_delivery          (line 738)  ← BUG: places -- before flags
2. require_prompt_present   (line 746)
3. reject_direct_yolo       (line 748)
4. apply_yolo_for_mode      (line 751)  ← flag after --
5. apply_entrypoint         (line 768)
6. apply_non_interactive    (line 770)
7. model resolution         (line 776)  ← flag after --
8. output format            (line 832)  ← flag after --
9. system prompt            (line 839)
10. operation env            (line 873)
11. sandbox                  (line 879)
12. env_plan build           (line 889)
13. MCP injection            (line 914)
14. structured stream        (line 1195) ← flag after --
```

### Target ordering (matches `composition.rs`):

```
1. reject_direct_yolo
2. apply_yolo_for_mode
3. apply_entrypoint
4. apply_non_interactive
5. model resolution
6. output format
7. system prompt
8. operation env
9. sandbox
10. env_plan build
11. MCP injection
12. structured stream
13. prompt_delivery          ← MOVED TO LAST
14. require_prompt_present   ← after delivery, same as composition
```

### Exact changes in `mod.rs`:

1. **Move the `prompt_delivery` block** (lines 733-744) to just before the `wrapper_harness` block (~line 1205). Place it after `apply_structured_stream` (line 1196) and the `StructuredCodexOutput::prepare` block.

2. **Move `require_prompt_present`** (line 746) to just after the relocated `prompt_delivery` block.

3. Keep `reject_direct_yolo` (line 748) where it is — it inspects child_args but prompt_delivery hasn't run yet, so args are still empty/clean at that point. Actually, since we're moving prompt_delivery to the end, `reject_direct_yolo` should stay before the apply_* methods. It currently runs on the args list after prompt_delivery, but since prompt_delivery was the one adding content, and we're moving it, reject_direct_yolo will now run on an empty-ish list. **Check needed**: verify `reject_direct_yolo` doesn't need the prompt args. Looking at implementations — most are no-ops or check for `--dangerously-skip-permissions`. OpenCode's is a no-op. This is safe to keep in its current position.

4. **No other methods need reordering** — `apply_yolo_for_mode`, `apply_entrypoint`, etc. stay in their current relative positions; the only change is removing the early `prompt_delivery`.

### Constraints to verify:

- `prompt_delivery` implementations receive the fully-populated `child_args` at that point. `AppendArgs` appends to end (safe). `InsertArgs` scans for position (safe — entrypoint is already inserted). `Stdin` doesn't touch args (safe).
- The `stdin_seed` variable from `prompt_delivery` is used later in `wrapper_harness` (line 1206-1209). After the move, `stdin_seed` is still in scope. Verify the variable is still accessible at the `wrapper_harness` usage point.
- The `prompt_source` variable (line 738 area) is used after the move point too. It's defined earlier and immutable, so this is safe.

### Expected argv after fix:

```
opencode run --dangerously-skip-permissions --model some-model --format json -- "my prompt"
```

All flags are before `--`. The prompt is the only positional after `--`.

---

## Step 2: Extract OpenCode Model Resolution into Shared Function (Fixes Issue 4)

**Confidence**: HIGH — pure extraction, no logic change.

**File**: `claudine/cli/src/commands/wrap/profile.rs`

### Current duplication:

- `mod.rs` lines 776-824: resolve model, push to args/env, validate
- `composition.rs` lines 373-448: same logic with minor env_plan API differences

### New function in `profile.rs`:

```rust
pub(crate) fn apply_opencode_model_resolution(
    child_args: &mut Vec<String>,
    env_setter: &mut dyn FnMut(String, String),
    cli_model: Option<&str>,
    non_interactive: bool,
) -> Result<Option<OpenCodeModelSource>>
```

The `env_setter` callback abstracts the difference between `Vec<(String, String)>` (mod.rs) and `HashMap<OsString, OsString>` (composition.rs).

### Changes:

1. Add `apply_opencode_model_resolution` to `profile.rs` with the body from `mod.rs` lines 776-824.
2. Replace inline code in `mod.rs` with a call to the new function.
3. Replace inline code in `composition.rs` with a call to the new function.
4. The validation error (no model provided) path: in `mod.rs` it uses `error_report::AgentErrorReport` and `std::process::exit(1)`; in `composition.rs` it returns `Err(...)`. Unify to return `Err` in both cases, or add an `on_no_model` callback. **Recommendation**: use `Err` in the shared function and let `mod.rs` catch and render the error report.

---

## Step 3: Extract Shared Pipeline Builder (Fixes Issue 7, DRY Opportunity 1)

**Confidence**: MEDIUM — this is a larger refactor. Can be deferred or done incrementally.

**File**: `claudine/cli/src/commands/wrap/profile.rs`

### Approach:

Create a `PipelineBuilder` struct that encapsulates the shared pipeline steps:

```rust
pub(crate) struct PipelineBuilder<'a> {
    profile: &'a dyn WrapperProfile,
    child_args: Vec<String>,
    env_overrides: Vec<(String, String)>,
}

impl<'a> PipelineBuilder<'a> {
    pub fn new(profile: &'a dyn WrapperProfile) -> Self;
    pub fn apply_yolo(&mut self, ...) -> Result<...>;
    pub fn apply_entrypoint(&mut self, ...);
    pub fn apply_model_resolution(&mut self, ...);
    pub fn apply_output_format(&mut self, ...);
    pub fn apply_system_prompt(&mut self, ...);
    pub fn apply_structured_stream(&mut self, ...);
    pub fn deliver_prompt(&mut self, ...) -> Result<...>;
    pub fn build(self) -> (Vec<String>, Vec<(String, String)>);
}
```

### Concerns:

- The two call sites have different surrounding context (env_plan structure, warning collection, MCP injection timing). The builder can't encapsulate everything without over-abstracting.
- **Recommendation**: Extract only the common arg-mutating steps into builder methods. Keep env_plan assembly, MCP injection, and warning collection in the caller. The builder's `build()` returns `(child_args, env_overrides)` for the caller to integrate.

---

## Step 4: Add Debug-Mode Argv Validation (Fixes Issue 8)

**Confidence**: HIGH — additive, no existing behavior change.

**File**: `claudine/cli/src/commands/wrap/profile.rs` (or `mod.rs` near the exec point)

### Implementation:

Add a function `validate_argv_flags_before_separator` that:

1. Finds the index of the first `--` in `child_args`.
2. Scans elements after `--` for any that start with `-`.
3. If found, logs a warning at `log::warn!` level: `"Flag {:?} appears after -- separator in argv: {:?}", flag, child_args`.

Call this function in both `mod.rs` and `composition.rs` just before exec, gated behind a `log::log_enabled!(log::Level::Warn)` check so there's zero overhead in normal operation.

```rust
pub(crate) fn validate_argv_order(binary: &str, args: &[String]) {
    if let Some(pos) = args.iter().position(|a| a == "--") {
        for arg in &args[pos + 1..] {
            if arg.starts_with('-') {
                log::warn!(
                    "Flag {:?} appears after -- separator in {} argv: {:?}",
                    arg, binary, args
                );
            }
        }
    }
}
```

---

## Step 5: Extract Common Delivery/Structured-Stream Helpers (DRY Opportunities 3, 4)

**Confidence**: LOW priority — these are cosmetic cleanups, not bugs.

### 5a: Common structured stream patterns

Multiple providers push identical flag pairs:
- Gemini, Qwen: `--output-format stream-json`
- Kimi: `--print --output-format stream-json`

These could use a shared helper:

```rust
fn push_stream_json_flags(args: &mut Vec<String>, extra: &[&str]) {
    for flag in extra {
        args.push((*flag).to_string());
    }
    args.push("--output-format".to_string());
    args.push("stream-json".to_string());
}
```

### 5b: Common prompt_delivery patterns

- **Stdin-when-non-interactive, append-when-interactive**: Claude, Kimi — could share a function.
- **AppendArgs with `--prompt`/`--prompt-interactive`**: Gemini, Qwen — could share a function.

These are straightforward extractions but don't fix any bugs. Defer to a separate cleanup pass.

---

## Risk Assessment

| Step | Risk | Mitigation |
|---|---|---|
| Step 0 (tests) | Low | Tests are additive |
| Step 1 (reorder) | Medium | composition.rs already proves the ordering works; tests from step 0 guard regressions |
| Step 2 (extract model resolution) | Low | Pure extraction; different error handling paths need attention |
| Step 3 (pipeline builder) | Medium | Over-abstract risk; defer if time-constrained |
| Step 4 (argv validation) | Low | Additive; only logs in debug mode |
| Step 5 (DRY helpers) | Low | Cosmetic; can skip entirely |

## Recommended Execution Order

1. **Step 0** — Write tests, confirm the opencode ordering test FAILS (proves bug exists)
2. **Step 1** — Reorder `mod.rs` pipeline, confirm all tests pass
3. **Step 4** — Add argv validation (quick, high value)
4. **Step 2** — Extract model resolution (reduces duplication)
5. **Step 3** — Pipeline builder (optional, larger refactor)
6. **Step 5** — DRY helpers (optional cleanup)

## Files Modified

| File | Steps | Change Type |
|---|---|---|
| `claudine/cli/src/commands/wrap/mod.rs` | 1, 4 | Pipeline reorder + validation call |
| `claudine/cli/src/commands/wrap/composition.rs` | 2, 4 | Use shared model resolution + validation call |
| `claudine/cli/src/commands/wrap/profile.rs` | 0, 2, 3, 4, 5 | Tests + shared functions + builder |

## Validation

After each step:

```bash
cargo test -p claudine-cli -- wrap
cargo test -p claudine-cli -- profile
cargo build -p claudine-cli
just -f claudine/justfile lint
```

Manual smoke test after step 1:

```bash
claudine opencode --model <some-model> "my prompt"
# Verify: flags appear before -- in the spawned argv
# Check: RUST_LOG=claudine=debug claudine opencode --model <some-model> "my prompt"
```
