---
phases: 7
created: 2026-05-06
start_phase: 1
source_files_during_phase_1:
  - claudine/lib/src/composition/error.rs
  - claudine/lib/src/composition/loop_config.rs
  - claudine/lib/src/composition/mod.rs
  - claudine/lib/src/composition/types.rs
docs_updated_during_phase_1:
  - claudine/features/2026-05-06-looping/plan.md
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - claudine/lib/src/composition/loop_actions.rs
  - claudine/lib/src/composition/mod.rs
docs_updated_during_phase_2:
  - claudine/features/2026-05-06-looping/plan.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
  - .claude/skills/claudine/SKILL.md
source_files_during_phase_3:
  - claudine/lib/src/composition/loop_expression.rs
  - claudine/lib/src/composition/mod.rs
docs_updated_during_phase_3:
  - claudine/features/2026-05-06-looping/plan.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
  - .claude/skills/claudine/SKILL.md
packages:
  - claudine
---

# Looping Feature Execution Plan

High-confidence plan for implementing per-prompt looping in Claudine, derived from `claudine/features/2026-05-06-looping/spec.md`.

## Context

- **Repository**: rusty-biscuit monorepo, `claudine/` package area
- **Architecture**: `claudine/lib/` (core library) + `claudine/cli/` (CLI wrapper)
- **Existing patterns**: Sequence feature (`claudine/lib/src/composition/sequence.rs`, `claudine/cli/src/commands/wrap/sequence.rs`) provides the closest precedent for iteration orchestration
- **Frontmatter pipeline**: Darkmatter parses YAML frontmatter; composition layer resolves templates, transclusions, and shell expansions
- **Expression engine**: Darkmatter's `darkmatter::markdown::compose::expression` module provides boolean condition evaluation via the `EvaluationLookup` trait
- **Template interpolation**: `claudine/lib/src/dispatch/template.rs` provides `interpolate()` with `EventMetaExpressionLookup`
- **Single execution entry points**: `claudine/cli/src/commands/compose.rs` (`run_compose_inner`, `run_inline_compose_inner`) → `execute_composition_request()` in `claudine/cli/src/commands/wrap/composition/mod.rs`
- **Error types**: `CompositionError` in `claudine/lib/src/composition/error.rs`
- **CLI args**: `SharedComposeArgs` in `claudine/cli/src/commands/compose.rs` is shared across `compose`, `inline-compose`, and `sequence`

## Integration Strategy

The loop is implemented at the **composition library layer** (`claudine/lib`) as a new orchestrator module, similar to how `sequence.rs` builds `SequencePlan`. The CLI layer detects loops and delegates to the loop orchestrator, which wraps the existing single-composition execution pipeline. This keeps loop logic testable independently of CLI child-process spawning and reuses all existing composition, harness, and provider-selection machinery.

---

## Phase 1: Core Types and Parsing

**Goal**: Define the data model for loop configuration and validate it from frontmatter.

**Depends on**: Nothing.

**Parallelizable**: Internal steps are sequential; no external dependencies.

### Step 1.1: Add loop types to `claudine/lib/src/composition/types.rs`
- Add `LoopConfig` struct with `condition: LoopCondition`, `actions: Vec<LoopAction>`, `max_iterations: Option<usize>`, `fail_fast: Option<bool>`
- Add `LoopCondition` enum with `While(String)` and `Until(String)` variants
- Add `LoopAction` enum with `Increment(String)`, `Decrement(String)`, `Set { prop: String, value: serde_json::Value }`, `Append { prop: String, value: serde_json::Value }`, `Prepend { prop: String, value: serde_json::Value }`, `Merge { prop: String, value: serde_json::Value }`
- Add `AmbientVariable` enum or constants for `iteration`, `is_first`, `is_last`, `last_output`, `last_exit_code`

**Observable**: Types compile; unit tests for struct construction pass.

### Step 1.2: Create `claudine/lib/src/composition/loop_config.rs`
- Implement `resolve_loop_config(source: &ResolvedCompositionSource) -> Result<Option<LoopConfig>, CompositionError>`
- Parse the `loop` frontmatter key from `source.markdown.frontmatter().as_map()`
- Validate `while`/`until` are mutually exclusive and are strings
- Validate `actions` is either a scalar (string or object) or a list of strings/objects
- Parse DSL-string actions: `increment(prop)`, `decrement(prop)`, `set(prop, value)`, `append(prop, value)`, `prepend(prop, value)`, `merge(prop, value)`
- Parse structured-object actions: `{ op: "increment", prop: "counter" }`, etc.
- Validate `loop.max` is a positive integer if present
- Validate `loop.fail_fast` is a boolean if present
- Return `CompositionError::LoopInvalid` with descriptive messages for any validation failure

**Observable**: Unit tests for all three syntax forms (scalar DSL, list of DSL, structured objects) pass; error cases return expected `CompositionError` variants.

### Step 1.3: Add loop error variants to `claudine/lib/src/composition/error.rs`
- Add `LoopInvalid(String)`
- Add `LoopLimitExceeded { cap: usize, prompt_path: PathBuf, iteration: usize }`
- Add `InvalidAction { iteration: usize, action_index: usize, total_actions: usize, message: String }`
- Add `InvalidIncrementType { iteration: usize, action_index: usize, property: String, found: String }`
- Add `InvalidDecrementType { iteration: usize, action_index: usize, property: String, found: String }`
- Ensure all error messages include the 1-based iteration index and action index as specified

**Observable**: New variants compile; `BlockError` impl covers them with appropriate `StatusBlock` rendering.

**Validation checkpoint**: `cargo test -p claudine` for new unit tests passes; `cargo check -p claudine` is clean.

---

## Phase 2: Action Engine

**Goal**: Implement all action mutators with atomic staging semantics.

**Depends on**: Phase 1.

**Parallelizable**: Steps 2.1–2.3 can be done in parallel after Step 1.3 is complete.

### Step 2.1: Implement numeric actions (`increment`, `decrement`)
- Create `claudine/lib/src/composition/loop_actions.rs`
- Implement `apply_increment(fm: &mut serde_json::Map<String, Value>, prop: &str) -> Result<(), CompositionError>`
  - If property is null/undefined/missing, set to `1`
  - If property is numeric (or string representation of number), increment by 1
  - Otherwise return `InvalidIncrementType`
- Implement `apply_decrement` with symmetric logic (default to `-1`)

**Observable**: Unit tests for null→1, "5"→6, 5→6, "abc"→error pass.

### Step 2.2: Implement property actions (`set`, `append`, `prepend`, `merge`)
- `set`: Direct assignment to frontmatter property. Reject `prop == "loop" || prop == "replace" || is_ambient(prop)` with `InvalidAction`.
- `append`: Append value to string property with rules for numeric/boolean coercion, JSON object serialization (prepend `\n`), JSON array serialization (prepend `\n`), empty/null preservation for JSONL pattern
- `prepend`: Same as append but `\n` goes after the new line instead of before
- `merge`: Shallow merge of object into property. Reject if property is non-null and not an object, or if value is not an object. On key collision, new value wins. Arrays are replaced, not concatenated.

**Observable**: Unit tests for each action type with edge cases (empty property, type mismatches, JSONL formation) pass.

### Step 2.3: Implement action atomicity (staging)
- Implement `ActionStaging` struct that holds a cloned copy of the frontmatter map
- Implement `Stage::apply_action(action: &LoopAction) -> Result<(), CompositionError>` which mutates the cloned copy
- Implement `Stage::commit() -> serde_json::Value` which returns the mutated copy
- Any error during `apply_action` discards the staged copy; no partial mutations are persisted
- Error messages include `at iteration {n}, action {i} of {total}`

**Observable**: Unit test shows that failure on action 2 of 3 leaves frontmatter unchanged from pre-action state.

**Validation checkpoint**: All action engine unit tests pass; `cargo check -p claudine` is clean.

---

## Phase 3: Condition Evaluation

**Goal**: Evaluate `while`/`until` boolean expressions against frontmatter + ambient variables using Darkmatter's expression engine.

**Depends on**: Phase 1.

**Parallelizable**: Can proceed in parallel with Phase 2 after Phase 1 completes.

### Step 3.1: Build frontmatter expression lookup adapter
- Create `claudine/lib/src/composition/loop_expression.rs`
- Implement `LoopExpressionLookup` struct holding references to the current frontmatter state and ambient variables
- Implement `EvaluationLookup` for `LoopExpressionLookup`
  - Resolution order: `env.*` → ambient variables (`iteration`, `is_first`, `is_last`, `last_output`, `last_exit_code`) → frontmatter properties (top-level and nested via `.`)
  - Ambient variables shadow same-named frontmatter keys
  - Convert frontmatter values to `serde_json::Value` for the evaluator

**Observable**: Unit tests resolve `iteration == 1`, `counter < 5`, `env.HOME != ""` correctly.

### Step 3.2: Integrate condition evaluation into loop flow
- Implement `evaluate_condition(condition: &LoopCondition, lookup: &LoopExpressionLookup) -> Result<bool, CompositionError>`
- Parse the condition string using `darkmatter::markdown::compose::expression::Parser::with_mode(..., ParseMode::Condition)`
- Evaluate with `darkmatter::markdown::compose::expression::evaluate()`
- Handle parse errors and evaluation errors, wrapping them in `LoopInvalid` with context

**Observable**: Unit tests for `while: "counter < 5"` (true/false) and `until: "done"` (true/false) pass.

**Validation checkpoint**: Condition evaluation unit tests pass; `cargo check -p claudine` is clean.

---

## Phase 4: Loop Orchestrator

**Goal**: Build the main loop execution engine that ties together conditions, actions, ambient variables, safety caps, and composition execution.

**Depends on**: Phases 1, 2, 3.

**Parallelizable**: Steps 4.1–4.3 are sequential; Step 4.4 can start after 4.2.

### Step 4.1: Create `claudine/lib/src/composition/loop_engine.rs`
- Define `LoopIterationContext` holding current frontmatter, ambient variables, and iteration state
- Define `LoopExecutionResult` holding final exit code, final frontmatter, iteration count, and optional error

### Step 4.2: Implement the core loop algorithm
```
1. Parse loop config from source frontmatter
2. Determine max_iterations (CLI flag → env → loop.max → default 100)
3. Determine fail_fast (CLI flag → env → loop.fail_fast → default true)
4. Initialize frontmatter = source frontmatter
5. Initialize last_output = "", last_exit_code = 0
6. For iteration = 1..=max_iterations:
   a. Build effective state = frontmatter + ambient variables
   b. Evaluate condition against effective state
   c. If condition says stop, break
   d. Prepare composition with effective frontmatter (ambient vars as set_overrides or env)
   e. Execute composition, capture (output, exit_code)
   f. If execution failed and fail_fast, break with error
   g. last_output = output; last_exit_code = exit_code
   h. If loop has actions:
      i. Create staged copy of frontmatter
      ii. Apply each action in order to staged copy
      iii. If any action fails and fail_fast, break with error
      iv. If fail_fast is false and action fails, discard staged; continue with pre-action frontmatter
      v. On full success, commit staged copy as new frontmatter
   i. Continue to next iteration
7. If max_iterations exceeded, return LoopLimitExceeded
```

**Observable**: Integration test runs a 3-iteration loop with increment and verifies final frontmatter state.

### Step 4.3: Implement `is_last` lookahead
- To compute `is_last`, speculatively apply actions to a copy of the state, evaluate the condition against that speculative state
- If the speculative condition says stop OR iteration + 1 == max_iterations, then `is_last = true`
- The user-visible state is unchanged by this lookahead

**Observable**: Unit test shows `is_last` is true on the final iteration and false otherwise, including when max_iterations is the stopping condition.

### Step 4.4: Handle `last_exit_code` under `fail_fast: false`
- When a prompt run fails (non-zero exit) and `fail_fast: false`, `last_exit_code` must reflect the failing iteration's exit code
- The next iteration's condition can branch on `last_exit_code == 0`

**Observable**: Unit test shows a retry loop with `until: "last_exit_code == 0"` continues past failures.

**Validation checkpoint**: All loop engine unit tests pass; integration test with mocked composition execution passes.

---

## Phase 5: CLI Integration

**Goal**: Wire loop detection and execution into the CLI, add new flags and env vars, and handle the `FAIL_FAST` migration.

**Depends on**: Phase 4.

**Parallelizable**: Steps 5.1–5.3 are sequential; Step 5.4 can be done in parallel with 5.2–5.3.

### Step 5.1: Add `--max-iterations` to `SharedComposeArgs`
- In `claudine/cli/src/commands/compose.rs`, add `max_iterations: Option<usize>` to `SharedComposeArgs`
- Add `#[arg(long = "max-iterations", value_name = "N")]`
- Add validation: must be > 0

**Observable**: `claudine compose --help` shows the new flag.

### Step 5.2: Add env var support for `CLAUDINE_MAX_ITERATIONS` and `CLAUDINE_FAIL_FAST`
- In the loop engine, read `CLAUDINE_MAX_ITERATIONS` from `std::env::var` if CLI flag not present
- In the sequence executor (`claudine/cli/src/commands/wrap/sequence.rs`), change `FAIL_FAST` env override to `CLAUDINE_FAIL_FAST`
- In the compose execution path, inject `CLAUDINE_FAIL_FAST` into env_overrides when a loop is present

**Observable**: Setting `CLAUDINE_MAX_ITERATIONS=10` limits loops to 10 iterations; `CLAUDINE_FAIL_FAST=false` allows continuation past failures.

### Step 5.3: Implement `FAIL_FAST` → `CLAUDINE_FAIL_FAST` deprecation
- Add a `static std::sync::Once` deprecation guard in `claudine/lib/src/composition/loop_config.rs` (or a shared env-reading utility)
- When `FAIL_FAST` is read and `CLAUDINE_FAIL_FAST` is not set, emit `tracing::warn!` once per process with migration message
- Use the `FAIL_FAST` value as fallback for one release

**Observable**: Integration test captures the deprecation warning in stderr/logs when `FAIL_FAST=true` is set.

### Step 5.4: Wire loop execution into compose and inline-compose
- In `run_compose_inner` and `run_inline_compose_inner` (in `claudine/cli/src/commands/compose.rs`):
  - After `composition::prepare_direct` / `composition::prepare_inline`, check if the effective frontmatter contains a `loop` property
  - If yes, build a `LoopConfig` and invoke the loop engine instead of the single `execute_composition_request` call
  - The loop engine should receive a callback/closure that executes one iteration via the existing `execute_composition_request_inner` path
  - Pass the `--max-iterations` CLI value and `CLAUDINE_FAIL_FAST` env value to the loop engine
- Ensure the loop's per-iteration composition uses the mutated frontmatter from the previous iteration
- Ensure ambient variables are available in the prompt body via `{{...}}` interpolation (they should be injected as `set_overrides` or env vars in the composition options)

**Observable**: A real prompt file with `loop: { while: "counter < 3", actions: ["increment(counter)"] }` runs 3 times when invoked with `claudine compose file.md`.

**Validation checkpoint**: Manual CLI test with example prompt files works; `cargo test -p claudine-cli` passes.

---

## Phase 6: Testing

**Goal**: Comprehensive test coverage for parsing, actions, conditions, integration, and edge cases.

**Depends on**: Phases 1–5.

**Parallelizable**: Test modules are independent and can be written in parallel.

### Step 6.1: Unit tests for loop config parsing (`loop_config.rs`)
- All three syntax forms (scalar DSL, list of DSL strings, structured objects)
- Missing `while`/`until` → error
- Both `while` and `until` present → error
- Invalid action syntax → error
- `loop.max` negative or zero → error
- `loop.fail_fast` non-boolean → error

### Step 6.2: Unit tests for action mutators (`loop_actions.rs`)
- `increment` on null, number, string-number, string, boolean
- `decrement` on null, number, string-number, string, boolean
- `set` on existing/new property; rejection of `loop`, `replace`, ambient names
- `append` on string, number, boolean, object (JSONL), array, empty/null
- `prepend` with same coverage as append
- `merge` on null, object, string, array; shallow merge semantics; array replacement on collision
- Atomicity: action 2 fails → frontmatter unchanged; action 3 succeeds after fix

### Step 6.3: Unit tests for condition evaluation (`loop_expression.rs`)
- `while: "counter < 5"` with counter = 3 (true), 5 (false)
- `until: "done"` with done = false (continue), true (stop)
- Ambient variable resolution: `iteration == 1`, `is_first == true`
- Frontmatter property resolution: `stage == "review"`
- Env variable resolution: `env.HOME != ""`
- Shadowing: frontmatter key `iteration` is shadowed by ambient `iteration`

### Step 6.4: Integration tests for loop engine (`loop_engine.rs`)
- Simple counter loop: 5 iterations, increment each time
- `until` loop with `set(done, true)` on iteration 3
- `append` accumulating a log across iterations
- `is_last` true only on final iteration
- `last_output` and `last_exit_code` propagation
- `max_iterations` cap exceeded → `LoopLimitExceeded`
- `fail_fast: true` with failing prompt → halts immediately
- `fail_fast: false` with failing prompt → continues, `last_exit_code` reflects failure
- Action failure with `fail_fast: false` → rolls back, continues from pre-action state

### Step 6.5: CLI integration tests
- `claudine compose loop.md` runs a loop file correctly
- `claudine compose --max-iterations 5 loop.md` overrides default
- `CLAUDINE_MAX_ITERATIONS=5 claudine compose loop.md` overrides default
- `FAIL_FAST=true` emits deprecation warning and still works
- Sequence command continues to use `CLAUDINE_FAIL_FAST` correctly

**Validation checkpoint**: `cargo test -p claudine` and `cargo test -p claudine-cli` both pass with new tests.

---

## Phase 7: Validation and Cleanup

**Goal**: Ensure the feature is complete, documented, and does not regress existing functionality.

**Depends on**: Phases 1–6.

**Parallelizable**: Steps 7.1–7.3 are sequential; Step 7.4 can run in parallel with 7.3.

### Step 7.1: Run full test suite
- `cargo test -p claudine`
- `cargo test -p claudine-cli`
- `cargo test -p darkmatter` (ensure expression engine changes don't break)

### Step 7.2: Run lint and typecheck
- `cargo clippy -p claudine -- -D warnings`
- `cargo clippy -p claudine-cli -- -D warnings`
- `cargo fmt -- --check`

### Step 7.3: Manual end-to-end validation
- Create a test prompt file with looping frontmatter
- Run with `claudine compose` and verify iterations
- Run with `--max-iterations` override
- Run with `fail_fast: false` and a failing command
- Verify `FAIL_FAST` deprecation warning appears once

### Step 7.4: Update spec and create example prompts
- Update `claudine/features/2026-05-06-looping/spec.md` with any implementation adjustments discovered
- Add example prompt files to `claudine/features/2026-05-06-looping/examples/`
  - `counter-loop.md` — basic while/increment
  - `retry-until-success.md` — until with fail_fast false
  - `accumulate-log.md` — append action building JSONL

**Validation checkpoint**: All tests pass, clippy is clean, manual validation succeeds, examples run correctly.

---

## Dependency Graph

```
Phase 1 (Types + Parsing)
  │
  ├──→ Phase 2 (Action Engine) ──┐
  │                              │
  └──→ Phase 3 (Condition Eval) ─┤
                                 │
                                 ▼
                         Phase 4 (Loop Orchestrator)
                                 │
                                 ▼
                         Phase 5 (CLI Integration)
                                 │
                                 ▼
                         Phase 6 (Testing)
                                 │
                                 ▼
                         Phase 7 (Validation)
```

Phases 2 and 3 are parallelizable after Phase 1. Phase 4 depends on both 2 and 3. Phases 5–7 are sequential.

## Risk Register

| Risk | Mitigation |
|------|------------|
| Darkmatter expression evaluator does not support all needed operators for loop conditions | Early spike in Phase 3.1 to confirm `ParseMode::Condition` handles comparisons, logical ops, and helpers. If gaps exist, implement custom pre-processing or extend Darkmatter. |
| Frontmatter mutation between iterations breaks Darkmatter composition caching/invariants | Loop engine creates fresh `ComposeOptions` with mutated frontmatter for each iteration; do not reuse `PreparedComposition` across iterations. |
| Action atomicity staging is expensive for large frontmatter objects | Staging clones the frontmatter `Map` only; if frontmatter is very large, this is acceptable for the 100-iteration default cap. Document this trade-off. |
| `is_last` lookahead duplicates action application work | Acceptable overhead per spec; the lookahead uses a disposable copy and does not mutate user-visible state. |
| `FAIL_FAST` deprecation conflicts with sequence's existing usage | Sequence already reads `FAIL_FAST`; the deprecation layer reads both names, preferring `CLAUDINE_FAIL_FAST`, and warns once if only the legacy name is present. |
