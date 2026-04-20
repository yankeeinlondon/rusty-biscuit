# Plan: Consistent Rendering for Compose and Inline-Compose

## Overview

Merge the divergent structured-stream reporting paths in `composition.rs` so
that `compose` and `inline-compose` share a single implementation for all
non-mode-specific logic. Five phases, each leaving the codebase buildable and
tests passing.

---

## Phase 1: Extract `run_structured_composition` + `CompositionStreamResult`

**Goal**: Replace both `run_structured_inline` (composition.rs:1139) and the
structured branch inside `execute_direct_without_harness` (composition.rs:1418)
with a single shared function.

### Step 1.1 — Define `CompositionStreamResult`

Add near the top of `composition.rs` (after existing type aliases, around line
137):

```rust
struct CompositionStreamResult {
    exit_code: i32,
    termination: ProcessTermination,
    assistant_text: String,
    summary: StreamExecutionSummary,
    details: StructuredSummaryDetails,
    had_streamed_assistant: bool,
}
```

### Step 1.2 — Create `run_structured_composition`

New function that consolidates:

- `LiveSemanticSink::with_default_wiring` + `.with_context_extra` (currently
  duplicated at composition.rs:1157-1164 and 1421-1428)
- Parser creation via `create_semantic_parser` (composition.rs:1167-1170 and
  1432-1436)
- `exec::run_child_stream_semantic` call (composition.rs:1171-1186 and
  1437-1452)
- `codex_output.apply_to_summary` (composition.rs:1192-1194 and 1454-1456)
- `had_streamed_assistant` detection — use the **inline** formula
  (`provider != Provider::Codex && !summary.assistant_text.trim().is_empty()`)
  for both modes (composition.rs:1190-1191). The compose path currently gates
  on `provider == Provider::Codex` (composition.rs:1459) which is the logical
  inverse and equivalent.
- Stdout rendering of assistant text when not streamed live, gated by
  `had_streamed_assistant` (composition.rs:1195-1210 and 1459-1476). The
  compose path currently goes through `section_stream.enter_final_stdout()`;
  this should remain compose-specific and NOT be inside the shared function.
  Instead, the shared function returns `had_streamed_assistant` and the caller
  decides whether to route through the section stream.

**Do NOT emit the summary** in this function. Return the `CompositionStreamResult`
so callers control timing.

**Do NOT include the empty-text warning** (composition.rs:1212-1213). It is
removed entirely per spec.

### Step 1.3 — Wire compose caller

Replace the structured branch in `execute_direct_without_harness`
(composition.rs:1418-1491):

1. Call `run_structured_composition(...)` to get a `CompositionStreamResult`.
2. For Codex post-hoc text that wasn't streamed: route through
   `section_stream.enter_final_stdout()` and render to stdout (the existing
   compose-specific behavior at lines 1459-1476, gated by
   `!result.had_streamed_assistant`).
3. Continue calling `emit_stream_summary_with_context` as today (Phase 2
   unifies the emission function).

### Step 1.4 — Wire inline caller

Replace the `run_structured_inline` call in `execute_inline_without_harness`
(composition.rs:928-943):

1. Call `run_structured_composition(...)` to get a `CompositionStreamResult`.
2. Store the result for deferred summary emission (same pattern as current
   `deferred_summary`).
3. Remove the empty-text warning — it is in `run_structured_inline` at
   composition.rs:1212-1213 and will be deleted when that function is removed.

### Step 1.5 — Remove `run_structured_inline`

After both callers are wired to `run_structured_composition`, delete
`run_structured_inline` (composition.rs:1139-1224).

### Step 1.6 — Verify

- `cargo build -p claudine-cli`
- `cargo test -p claudine-cli`
- `rg "run_structured_inline" claudine/` — should return zero matches

---

## Phase 2: Unify summary emission into `emit_composition_summary`

**Goal**: Replace `emit_stream_summary_with_context` and
`emit_stream_summary_no_separator_with_context` (as used by composition) with a
single `emit_composition_summary` function.

### Step 2.1 — Add `emit_composition_summary` to `composition.rs`

```rust
fn emit_composition_summary(
    result: &CompositionStreamResult,
    profile: &dyn WrapperProfile,
    env_context: &EnvironmentContext,
    verbosity: Verbosity,
    verbose: bool,
    dispatch_context: &HashMap<String, serde_json::Value>,
    section_stream: Option<&SectionStream>,
    defer_section_separator: bool,
)
```

Logic:
- If `defer_section_separator` is true (inline-compose): emit directly via
  `eprintln!` using `Prose::render`, matching the current behavior of
  `emit_stream_summary_no_separator_with_context` (mod.rs:3095-3112).
- If `defer_section_separator` is false (compose): call
  `emit_stream_summary_inner` with the section stream, matching the current
  behavior of `emit_stream_summary_with_context` → `emit_stream_summary_inner`
  (mod.rs:2984-3062).
- Both paths write the JSONL summary event.

### Step 2.2 — Update compose caller

In `execute_direct_without_harness`, replace:

```rust
emit_stream_summary_with_context(
    StreamSummaryContext { ... section_stream: Some(&section_stream) },
    dispatch_context,
);
```

with:

```rust
emit_composition_summary(
    &result, profile, env_context, stream_verbosity, detail_requested,
    dispatch_context, Some(&section_stream), false,
);
```

### Step 2.3 — Update inline caller

In `execute_inline_without_harness`, replace:

```rust
emit_stream_summary_no_separator_with_context(
    &summary, profile, env_context, stream_verbosity, detail_requested,
    &details, Some(dispatch_context),
);
```

with:

```rust
emit_composition_summary(
    &result, profile, env_context, stream_verbosity, detail_requested,
    dispatch_context, None, true,
);
```

### Step 2.4 — Clean up imports

Remove `emit_stream_summary_no_separator_with_context` from the
`composition.rs` import block (composition.rs:38) if no longer used.

### Step 2.5 — Verify

- `cargo build -p claudine-cli`
- `cargo test -p claudine-cli`

Note: `emit_stream_summary_no_separator` and
`emit_stream_summary_no_separator_with_context` in `mod.rs` are **not deleted
yet** — Phase 5 handles dead-code cleanup. `emit_stream_summary_no_separator`
is already `#[allow(dead_code)]` (mod.rs:3066) so the build will not break.

---

## Phase 3: Replace `emit_legacy_composition_session_event` with `emit_minimal_composition_summary`

**Goal**: Both legacy (non-structured) branches emit the same stderr summary
and JSONL event that structured runs do, rather than a JSONL-only synthetic
event.

### Step 3.1 — Add `emit_minimal_composition_summary` to `composition.rs`

```rust
fn emit_minimal_composition_summary(
    provider: Provider,
    exit_code: i32,
    profile: &dyn WrapperProfile,
    env_context: &EnvironmentContext,
    dispatch_context: &HashMap<String, serde_json::Value>,
)
```

Logic:
1. Build a `StreamExecutionSummary` with `exit_code` and
   `is_error = exit_code != 0`. All other fields (duration, tokens, cost,
   tool counts, assistant text) remain at defaults.
2. Build a `StructuredSummaryDetails::default()`.
3. Construct a `CompositionStreamResult` from these.
4. Call `emit_composition_summary` with `section_stream: None`,
   `defer_section_separator: true` (no section stream for legacy paths).
5. This renders to stderr AND writes the JSONL event — full parity with
   structured runs.

### Step 3.2 — Update compose legacy branch

In `execute_direct_without_harness` (composition.rs:1507-1511), replace:

```rust
emit_legacy_composition_session_event(provider, result.data, env_context, dispatch_context);
```

with:

```rust
emit_minimal_composition_summary(provider, result.data, profile, env_context, dispatch_context);
```

Note: `profile` is already a parameter of `execute_direct_without_harness`.

### Step 3.3 — Update inline legacy branch

In `execute_inline_without_harness` (composition.rs:1118-1121), replace:

```rust
emit_legacy_composition_session_event(provider, final_exit, env_context, dispatch_context);
```

with:

```rust
emit_minimal_composition_summary(provider, final_exit, profile, env_context, dispatch_context);
```

### Step 3.4 — Delete `emit_legacy_composition_session_event`

Remove the function at composition.rs:1579-1622.

### Step 3.5 — Verify

- `cargo build -p claudine-cli`
- `cargo test -p claudine-cli`
- `rg "emit_legacy_composition_session_event" claudine/` — zero matches

---

## Phase 4: Collapse into `execute_without_harness`

**Goal**: Merge `execute_direct_without_harness` and
`execute_inline_without_harness` into a single function parameterized by mode.

### Step 4.1 — Define `CompositionExecutionMode`

```rust
enum CompositionExecutionMode {
    Direct,
    Inline { closure_plan: InlineClosurePlan },
}
```

### Step 4.2 — Create `execute_without_harness`

Merge the two functions into one. The inline-specific code blocks are guarded
by `if let CompositionExecutionMode::Inline { closure_plan } = &mode` with
comments explaining why they are inline-only:

**Inline-only blocks** (from `execute_inline_without_harness`):
1. **Writability pre-check** (if present in the existing code) — inline
   validates write permission before execution.
2. **Closure validation and file write** (composition.rs:1006-1101) —
   extract_replacement_body, apply_inline_closure, frontmatter merge
   reporting, darkmatter cleanup.
3. **Interrupted-session body report** (composition.rs:1000-1003) —
   report_interruption for partial file content.
4. **Deferred summary timing** — emit the summary after closure validation
   messages, using `defer_section_separator: true`.

**Compose-only behavior**:
1. Summary emitted immediately with `defer_section_separator: false`.
2. Codex stdout text routed through `section_stream.enter_final_stdout()`.

### Step 4.3 — Update call sites in `execute_composition_request_inner`

At composition.rs:814, replace:
```rust
execute_inline_without_harness(...)
```
with:
```rust
execute_without_harness(CompositionExecutionMode::Inline { closure_plan: ... }, ...)
```

At composition.rs:860, replace:
```rust
execute_direct_without_harness(...)
```
with:
```rust
execute_without_harness(CompositionExecutionMode::Direct, ...)
```

### Step 4.4 — Delete old functions

- `execute_direct_without_harness` (composition.rs:1400-1513)
- `execute_inline_without_harness` (composition.rs:903-1125)

### Step 4.5 — Verify

- `cargo build -p claudine-cli`
- `cargo test -p claudine-cli`

---

## Phase 5: Clean up dead code

### Step 5.1 — Delete dead functions and types

| Item | Location | Reason |
|------|----------|--------|
| `run_structured_inline` | composition.rs:1139 | Absorbed into `run_structured_composition` in Phase 1 |
| `run_legacy_inline` | composition.rs:1227 | Absorbed into merged legacy branch in Phase 4 |
| `InlineRunResult` | composition.rs:1127-1136 | Replaced by `CompositionStreamResult` |
| `emit_stream_summary_no_separator` | mod.rs:3067-3083 | Unused (`#[allow(dead_code)]`), only caller was composition |
| `emit_stream_summary_no_separator_with_context` | mod.rs:3086-3126 | Replaced by `emit_composition_summary` in Phase 2 |

### Step 5.2 — Verify no remaining references

```bash
rg "InlineRunResult\|run_structured_inline\|run_legacy_inline\|emit_stream_summary_no_separator\|emit_legacy_composition_session_event" claudine/
```

Should return zero matches in `.rs` files.

### Step 5.3 — Final verification

- `cargo build -p claudine-cli`
- `cargo test -p claudine-cli`
- `cargo fmt --package claudine-cli`
- `cargo clippy --package claudine-cli 2>&1 | head -50`

---

## Intentional Exception Registry

These differences between compose and inline-compose are preserved with
comments in the merged code:

1. **Closure validation and file write** — inline-compose mutates the source
   file; compose does not. Guard: `if let CompositionExecutionMode::Inline {
   closure_plan } = &mode`.

2. **Deferred summary timing** — inline-compose emits summary after closure
   validation via `defer_section_separator: true`. Compose emits immediately
   via `defer_section_separator: false`.

3. **Interrupted-session body report** — inline-compose reports partial body
   content when interrupted (composition.rs:1000-1003). Guarded by inline-mode
   check; compose skips this.

4. **Writability pre-check** — inline-compose validates write permission on
   the target file before execution. Compose has no file write step.

---

## Files Changed Summary

| File | Changes |
|------|---------|
| `claudine/cli/src/commands/wrap/composition.rs` | Add `CompositionStreamResult`, `CompositionExecutionMode`, `run_structured_composition`, `emit_composition_summary`, `emit_minimal_composition_summary`, `execute_without_harness`. Remove `run_structured_inline`, `run_legacy_inline`, `execute_direct_without_harness`, `execute_inline_without_harness`, `emit_legacy_composition_session_event`, `InlineRunResult`. Remove empty-text warning. |
| `claudine/cli/src/commands/wrap/mod.rs` | Remove `emit_stream_summary_no_separator` and `emit_stream_summary_no_separator_with_context` (Phase 5). |

---

## Risks and Mitigations

| Risk | Mitigation |
|------|------------|
| `had_streamed_assistant` formula differs between modes | Both formulas are logically equivalent (`provider != Codex && text_nonempty` vs `provider == Codex`). The shared function uses the inline formula which is the clearer negation. |
| Section-stream routing is compose-only | The shared `run_structured_composition` does not touch the section stream. The compose caller handles `section_stream.enter_final_stdout()` after receiving the result. |
| Legacy Goose path now emits stderr summary | This is intentional per spec — Goose gets the same visible feedback as other providers. If stderr output is unexpected, the summary gracefully degrades to only exit_code and is_error fields. |
| `emit_stream_summary_no_separator_with_context` may have other callers | Verified: only caller is composition.rs:1109. `emit_stream_summary_no_separator` is already `#[allow(dead_code)]`. Neither is used in `sequence.rs`. |
