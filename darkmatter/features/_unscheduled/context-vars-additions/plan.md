---
agent: "open_code"
phases: 6
created: "2026-06-10"
start_phase: 1
hash: ""
---

# Execution Plan: Context Variables and Expression Function Additions

Derived from `darkmatter/features/_unscheduled/context-vars-additions/spec.md`.

## Overview

Add two new runtime context variables (`agent`, `model`) and twenty new expression functions across four categories (Filesystem, Type Predicates, String Mutations, Context).

---

## Phase 1: Context Variables — `agent` and `model`

**Goal:** Surface the executing agent name and model name as `ctx.agent` and `ctx.model`.

**Dependency:** None (foundation work).

### Tasks

- [ ] Add `ContextGroup::Agent` variant to the demand-driven capture group enum in `darkmatter/lib/src/markdown/compose/context/capture.rs`.
- [ ] Register `agent` and `model` in `ContextGroup::for_key` mapping.
- [ ] Add `ContextVariableDescriptor` entries for `agent` and `model` in `darkmatter/lib/src/markdown/compose/context/catalog.rs` under a new "Agent" category.
- [ ] Implement `populate_agent(values: &mut Map<String, Value>)` in `capture.rs` that reads `std::env::var("AGENT")` and `std::env::var("MODEL")`, inserting them as `Value::String`; `model` defaults to `"default"` when absent or unknown.
- [ ] Wire `populate_agent` into `capture_runtime_context_for_groups` when `ContextGroup::Agent` is present.
- [ ] Update `ContextGroup::all()` to include `Agent`.
- [ ] Add unit tests in `capture.rs` verifying:
  - `AGENT=opencode` → `ctx.agent == "opencode"`
  - `MODEL=gpt-4o` → `ctx.model == "gpt-4o"`
  - Missing `MODEL` → `ctx.model == "default"`
  - Missing `AGENT` → `ctx.agent` is present (empty string or null per existing env conventions)
- [ ] Verify `descriptor_name_set_equals_captured_runtime_key_set` test passes after catalog additions.

### Validation Checkpoint

- [ ] `cargo test -p darkmatter descriptor_name_set_equals_captured_runtime_key_set` passes.
- [ ] `cargo test -p darkmatter capture::` tests for agent/model pass.

---

## Phase 2: Filesystem Expression Functions

**Goal:** Add ten path-shape utilities that resolve through `FileReference` and operate on the resolved/normalized path string without existence checks.

**Dependency:** Phase 1 (not strictly required, but foundation should be in place first).

**Parallelizable:** Yes — each function is independent after the shared `FileReference` resolution helper is established.

### Tasks

- [ ] Add a shared `resolve_file_ref_arg(name: &str, value: &Value, ctx: &ResolutionContext) -> Result<String, String>` helper in `functions.rs` that:
  - Requires a string argument
  - Parses through `biscuit_file::FileReference::new(...)` and resolves to absolute path
  - Returns the resolved path string on success
  - Returns error for non-string inputs or malformed file references
- [ ] Implement `is_indexed_file_fn(args, ctx)` → `Value::Bool`:
  - Resolves file through `FileReference`, checks if stem matches `{base}-{digits}` before extension
  - Examples: `review-1.md` → `true`, `review1.md` → `false`, `review_1.md` → `false`
- [ ] Implement `file_index_fn(args, ctx)` → `Value::Number`:
  - Returns index number for indexed files; returns `-1` for non-indexed files
- [ ] Implement `increment_file_index_fn(args, ctx)` → `Value::String`:
  - Increments index: `review-1.md` → `review-2.md`
  - Preserves zero-padding: `review-001.md` → `review-002.md`
  - Non-indexed files start at index `2`: `review.md` → `review-2.md` (no padding)
- [ ] Implement `decrement_file_index_fn(args, ctx)` → `Value::String`:
  - Decrements index; clamps at `0` (further decrements stay at `0`)
  - Preserves zero-padding when present
- [ ] Implement `basename_fn(args, ctx)` → `Value::String`:
  - Returns filename with extension (e.g., `foo/bar/baz.md` → `baz.md`)
- [ ] Implement `basename_without_index_fn(args, ctx)` → `Value::String`:
  - Returns basename with index removed for indexed files; unchanged for non-indexed
  - `foo/review-1.md` → `review.md`
- [ ] Implement `dir_fn(args, ctx)` → `Value::String`:
  - Returns directory portion of resolved path
- [ ] Implement `ext_fn(args, ctx)` → `Value::String`:
  - Returns file extension or empty string when none
- [ ] Implement `file_trailing_fn(args, ctx)` → `Value::String`:
  - Returns last directory + basename: `foo/bar/baz/test.md` → `baz/test.md`
- [ ] Implement `dir_leading_fn(args, ctx)` → `Value::String`:
  - Returns all but last directory: `foo/bar/baz/test.md` → `foo/bar`
- [ ] Register all ten functions in `FS_FUNCTIONS` table in `functions.rs` with canonical names and aliases.
- [ ] Add `ExpressionFunctionDescriptor` entries in `catalog.rs` under category "Filesystem".
- [ ] Add unit tests in `functions.rs` for each function covering:
  - Indexed file patterns (with/without padding)
  - Non-indexed file patterns
  - FileReference resolution (relative, `@repo-root`, etc.)
  - Null propagation
  - Type mismatch errors (non-string input)
  - Missing extension cases

### Validation Checkpoint

- [ ] `cargo test -p darkmatter fn_filesystem` passes.
- [ ] `cargo test -p darkmatter descriptor_signature_set_equals_dispatchable_signature_set` passes.

---

## Phase 3: Type Predicate Expression Functions

**Goal:** Add `is_positive`, `is_negative`, and `is_integer` to the Type Predicates category.

**Dependency:** Phase 1.

**Parallelizable:** Yes — all three are independent pure functions.

### Tasks

- [ ] Implement `is_positive_fn(args: &[Value])` → `Value::Bool`:
  - Uses existing numeric coercion rules (`to_number`)
  - `0` is neither positive nor negative → `false`
  - Errors on uncoercible values
- [ ] Implement `is_negative_fn(args: &[Value])` → `Value::Bool`:
  - Same coercion and zero semantics as `is_positive` but inverted
- [ ] Implement `is_integer_fn(args: &[Value])` → `Value::Bool`:
  - Strict: only `Value::Number` where `fract() == 0.0` returns `true`
  - String `"1"` → `false`; JSON number `1` → `true`
  - Errors on uncoercible values (or returns `false` per type-predicate convention; check existing `is_number` behavior)
- [ ] Register all three in `PURE_FUNCTIONS` table.
- [ ] Add `ExpressionFunctionDescriptor` entries in `catalog.rs` under "Type Predicates" category.
- [ ] Add unit tests covering:
  - Positive/negative/zero numbers
  - Coerced strings (`"5"`, `"-3"`)
  - Non-coercible values (`"abc"`, `[]`, `{}`, `true`, `null`)
  - Integer strictness (`1` vs `"1"`, `1.5` vs `"1.5"`)

### Validation Checkpoint

- [ ] `cargo test -p darkmatter fn_type_predicates` passes (new assertions).
- [ ] `cargo test -p darkmatter descriptor_signature_set_equals_dispatchable_signature_set` passes.

---

## Phase 4: String Mutation Expression Functions

**Goal:** Add `without_date`, `ensure_leading`, `ensure_trailing`, `link`, and `terminal`.

**Dependency:** Phase 1.

**Parallelizable:** Partially — `without_date`, `ensure_leading`, `ensure_trailing` are independent. `link` depends on understanding `FileReference` resolution. `terminal` depends on `biscuit_terminal::Prose`.

### Tasks

- [ ] Implement `without_date_fn(args: &[Value])` → `Value::String`:
  - Requires JSON string; errors on non-string
  - Removes strict `YYYY-MM-DD` calendar-date substrings via regex
  - Does NOT remove full datetimes as a single token
  - Does NOT remove compact/ordinal dates
  - Returns unchanged string when no matches
- [ ] Implement `ensure_leading_fn(args: &[Value])` → `Value::String | Value::Number`:
  - Accepts string or number for both `var` and `prefix`
  - Rejects arrays, objects, booleans, null
  - Returns number only when `var` is JSON number AND `prefix` is numeric or numberlike string
  - Otherwise returns string
  - Examples: `ensure_leading("foobar", "foo")` → `"foobar"`; `ensure_leading("bar", "foo")` → `"foobar"`; `ensure_leading(123, 4)` → `4123`; `ensure_leading("123", "abc")` → `"abc123"`
- [ ] Implement `ensure_trailing_fn(args: &[Value])` → `Value::String | Value::Number`:
  - Same type rules as `ensure_leading` but operates on suffix
  - Returns number only when `var` is JSON number AND `postfix` is numeric or numberlike string
- [ ] Implement `link_fn` as a context-aware function in `FS_FUNCTIONS`:
  - One-arg `link(file)`: resolves through `FileReference`, errors on HTTP(S) URLs, description is relative path, destination is absolute path
  - Two-arg `link(target, desc)`: accepts file reference or HTTP(S) URL; description is explicit arg
  - Renders as markdown `[desc](destination)`
  - Escapes `[` and `]` in descriptions
  - Escapes destinations for spaces, `)`, etc. using angle-bracket form or percent-encoding
- [ ] Implement `terminal_fn(args: &[Value])` → `Value::String`:
  - Requires JSON string; errors on non-string
  - Converts string to terminal-escaped output using `biscuit_terminal::components::prose::Prose`
  - Returns the rendered ANSI string
- [ ] Register `without_date`, `ensure_leading`, `ensure_trailing`, `terminal` in `PURE_FUNCTIONS`.
- [ ] Register `link` in `FS_FUNCTIONS`.
- [ ] Add descriptors in `catalog.rs` under "String Mutations" category.
- [ ] Add unit tests covering:
  - Date removal with overlapping and edge-case strings
  - Leading/trailing with string and number combinations
  - Link one-arg vs two-arg, file vs URL, markdown escaping
  - Terminal rendering of styled text
  - Type mismatch errors for all functions

### Validation Checkpoint

- [ ] `cargo test -p darkmatter fn_string_mutations` passes.
- [ ] `cargo test -p darkmatter fn_filesystem` passes (for link tests).
- [ ] `cargo test -p darkmatter descriptor_signature_set_equals_dispatchable_signature_set` passes.

---

## Phase 5: Context Expression Functions

**Goal:** Add `has_skill` and `has_local_skill`.

**Dependency:** Phase 1 (needs `ctx.agent` to derive skill roots). May need skill root discovery logic.

### Tasks

- [ ] Investigate and document known skill root paths per agent:
  - Claude: `~/.claude/skills/`, `{repo}/.claude/skills/`
  - OpenCode: `~/.config/opencode/skill/`, `{repo}/.opencode/skill/`
  - Generic: `{repo}/.agents/skills/`
  - Confirm against actual directory structures in the repo
- [ ] Implement skill root resolution helper:
  - Derives user-scoped and local-scoped roots from `ctx.agent` value
  - Returns ordered list of paths to check
- [ ] Implement `has_skill_fn(args: &[Value])` → `Value::Bool`:
  - Accepts skill name string
  - Checks all known skill roots (user-scoped + local-scoped) for the executing agent
  - A skill exists when a direct child directory basename exactly matches the name
  - Returns `false` when agent is unknown or no roots match
- [ ] Implement `has_local_skill_fn(args: &[Value])` → `Value::Bool`:
  - Same as `has_skill` but checks only local-scoped roots (repo-relative)
- [ ] Register both in `PURE_FUNCTIONS` (they don't need filesystem resolution context, just the agent name from environment).
- [ ] Add `ExpressionFunctionDescriptor` entries in `catalog.rs` under "Context" category.
- [ ] Add unit tests with temporary directory trees:
  - Skill exists in local root → `has_skill(true)`, `has_local_skill(true)`
  - Skill exists only in user root → `has_skill(true)`, `has_local_skill(false)`
  - Skill does not exist → both `false`
  - Wrong directory name (nested, suffix) → `false`

### Validation Checkpoint

- [ ] `cargo test -p darkmatter has_skill` passes.
- [ ] `cargo test -p darkmatter descriptor_signature_set_equals_dispatchable_signature_set` passes.

---

## Phase 6: Integration, Regression, and Documentation

**Goal:** Ensure all new surface works end-to-end and is documented.

**Dependency:** Phases 1–5 complete.

### Tasks

- [ ] Add end-to-end regression tests in `darkmatter/lib/tests/expression_regression.rs` for:
  - `ctx.agent` and `ctx.model` in interpolation expressions
  - Each new filesystem function in real document compose pipeline
  - Each new type predicate in ternary expressions
  - `link()` rendering markdown links correctly in composed output
  - `terminal()` producing ANSI output in composed output
  - `has_skill()` / `has_local_skill()` with mocked skill directories
- [ ] Update `darkmatter/docs/topics/darkmatter-expressions.md`:
  - Add new context variables to the context-variables reference
  - Add new expression functions to the function catalog with signatures and examples
- [ ] Update `darkmatter/docs/topics/context-variables.md`:
  - Document `agent` and `model` variables with examples
- [ ] Run full darkmatter test suite: `cargo test -p darkmatter`
- [ ] Run clippy: `cargo clippy -p darkmatter --all-targets --all-features`
- [ ] Verify no compiler warnings in changed files.

### Validation Checkpoint

- [ ] `cargo test -p darkmatter` passes (full suite).
- [ ] `cargo clippy -p darkmatter --all-targets --all-features` is clean.
- [ ] Documentation renders correctly (`cargo doc -p darkmatter --no-deps`).

---

## Parallelization Summary

| Phase | Parallel Within Phase? | Notes |
|-------|----------------------|-------|
| 1 | Yes | `agent` and `model` capture are independent. |
| 2 | Yes | All 10 filesystem functions are independent once the shared `resolve_file_ref_arg` helper exists. |
| 3 | Yes | All 3 type predicates are independent pure functions. |
| 4 | Partial | `without_date`, `ensure_leading`, `ensure_trailing` parallel; `link` and `terminal` each have unique dependencies. |
| 5 | Yes | `has_skill` and `has_local_skill` share a helper but can be implemented together. |
| 6 | No | Sequential validation and documentation. |

## Rollback Strategy

If any phase introduces test failures:
1. Identify the failing function/context variable.
2. Comment out its registration in `PURE_FUNCTIONS`, `FS_FUNCTIONS`, or `ContextGroup::for_key`.
3. Fix in isolation before re-enabling.
4. The catalog parity tests will fail if a function is registered but not described (or vice versa) — this is the intended safety net.
