# Tech Design: Consistent Rendering for Compose and Inline-Compose

## Problem

`claudine compose` and `claudine inline-compose` use different code paths for
structured stream reporting in non-interactive sessions, producing divergent
user feedback despite performing equivalent work. The divergence exists in
four places within `composition.rs`:

1. Two separate execution functions (`execute_direct_without_harness` and
   `execute_inline_without_harness`) each contain their own structured-stream
   setup, LiveSemanticSink construction, and summary emission logic.
2. Compose uses `emit_stream_summary_with_context`; inline-compose uses
   `emit_stream_summary_no_separator_with_context`.
3. Inline-compose has a deferred-summary pattern that delays emission until
   after closure validation, while compose emits immediately.
4. Both legacy (non-structured) paths call `emit_legacy_composition_session_event`
   which creates a synthetic `SessionEnd` event manually instead of using the
   modern summary pipeline.

## Spec Requirements

1. All reporting for compose and inline-compose during non-interactive sessions
   should be nearly identical and should use the same code.
2. Any exceptions must include clear comments describing why the exception is
   important.
3. All calls to `emit_legacy_composition_session_event` should be changed to
   use the modern alternative.
4. Once all calls are removed, delete `emit_legacy_composition_session_event`.

## Current Architecture

### Entry Points

```
compose.rs:run_compose_inner()      → CompositionMode::ChainedDocument
compose.rs:run_inline_compose_inner() → CompositionMode::InlineFrontmatterPrompt
```

Both build a `CompositionExecutionRequest` and call
`composition.rs:execute_composition_request()`.

### Branching in execute_composition_request

```
execute_composition_request_inner()
├── harness_enabled == true
│   └── run_harness_loop()  ← shared for both modes; no changes needed
├── is_inline == true
│   └── execute_inline_without_harness()
│       ├── structured: run_structured_inline()
│       │   └── returns deferred (summary, details, had_streamed_assistant)
│       │   └── later: emit_stream_summary_no_separator_with_context()
│       └── legacy: run_legacy_inline()
│           └── emit_legacy_composition_session_event()
└── is_inline == false (compose)
    └── execute_direct_without_harness()
        ├── structured: inline LiveSemanticSink + emit_stream_summary_with_context()
        └── legacy: exec::run_child() + emit_legacy_composition_session_event()
```

### Key Differences Today

| Aspect | compose | inline-compose | Unify? |
|--------|---------|----------------|--------|
| Structured sink construction | Inside `execute_direct_without_harness` | Inside `run_structured_inline` | Yes (Phase 1) |
| Summary emission function | `emit_stream_summary_with_context` | `emit_stream_summary_no_separator_with_context` | Yes (Phase 2) |
| Section stream | Passed to emit function | Not used | Preserved as intentional difference (tied to deferred summary timing) |
| Summary timing | Immediately after execution | After closure validation | Preserved as intentional exception (documented) |
| Empty-text warning | None | `"the agent did not provide a summarized message"` | **Remove from both** |
| `had_streamed_assistant` | N/A | Tracked, gates stdout re-render | Yes — track and gate stdout re-render in both modes |
| Codex post-hoc text render | In `execute_direct_without_harness` | In `run_structured_inline` | Yes (Phase 1 shared helper) |
| Legacy event emission | `emit_legacy_composition_session_event` | `emit_legacy_composition_session_event` | Replaced by `emit_minimal_composition_summary` (Phase 3), which renders the same stderr summary as structured runs — affects Goose only, since it is the only provider without structured stream support |

## Design

### Phase 1: Extract shared structured-stream execution

Create a single struct `CompositionStreamResult` and a single function
`run_structured_composition` that replaces both the structured path inside
`execute_direct_without_harness` and `run_structured_inline`.

```rust
/// Result of running a provider child process with structured stream parsing.
struct CompositionStreamResult {
    exit_code: i32,
    termination: ProcessTermination,
    assistant_text: String,
    summary: StreamExecutionSummary,
    details: StructuredSummaryDetails,
    had_streamed_assistant: bool,
}

/// Run a provider child with structured stream parsing.
///
/// Returns the full result including deferred summary data so the caller
/// can decide when to emit the summary (immediately for compose, after
/// closure validation for inline-compose).
fn run_structured_composition(
    provider: Provider,
    profile: &dyn WrapperProfile,
    binary_path: &Path,
    child_args: &[String],
    child_env: &HashMap<OsString, OsString>,
    child_cwd: &Path,
    stdin_seed: Option<&str>,
    structured_codex_output: Option<&StructuredCodexOutput>,
    stderr_noise: &[&str],
    stream_verbosity: Verbosity,
    env_context: &EnvironmentContext,
    dispatch_context: &HashMap<String, serde_json::Value>,
    child_spawned: &mut bool,
) -> Result<CompositionStreamResult>
```

This function consolidates:
- LiveSemanticSink construction (identical in both paths today)
- Parser creation via `create_semantic_parser`
- `exec::run_child_stream_semantic` invocation
- Codex post-hoc text application (`codex_output.apply_to_summary`)
- `had_streamed_assistant` detection (used by both modes)
- Stdout rendering of assistant text when not streamed live, gated by
  `had_streamed_assistant` in both modes

The function does **not** emit the summary. It returns all the data needed
for the caller to decide when and how to emit.

The empty-text warning (`"the agent did not provide a summarized message"`)
is **removed entirely** — not migrated into this function. The `SessionEnd`
JSONL event already records empty assistant text; no stderr warning is
needed in either mode.

### Phase 2: Unify summary emission

Both callers emit through the same function. The only difference is timing
(and therefore separator handling):

```rust
/// Emit the composition stream summary to stderr and JSONL.
///
/// `defer_section_separator`: when true, skips the automatic section-stream
/// separator. Inline-compose sets this to true because closure validation
/// output appears between the stream and the summary, so the caller manages
/// its own spacing.
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

Internally this calls `emit_stream_summary_inner` (the existing shared
implementation in `mod.rs`). The `defer_section_separator` flag selects
between the section-stream path and the direct-stderr path, replacing the
current two-function split of `emit_stream_summary_with_context` vs
`emit_stream_summary_no_separator_with_context`.

### Phase 3: Eliminate legacy paths

Both `execute_direct_without_harness` and `execute_inline_without_harness`
have legacy (non-structured) else branches that call
`emit_legacy_composition_session_event`. Replace these with a lightweight
structured summary that constructs a `StreamExecutionSummary` from the raw
`exec::run_child` result and emits via the same `emit_composition_summary`
function.

In practice this path is only reachable for the Goose provider, which is
the only provider whose `WrapperProfile::supports_structured_stream()`
returns `false`. All other composition-eligible providers (Claude, Codex,
Gemini, Kimi, Qwen, OpenCode) take the structured path. Goose compositions
therefore receive the same visible stderr summary as other providers after
this phase, rather than the current JSONL-only silence.

The replacement for `emit_legacy_composition_session_event`:

```rust
/// Build a minimal `StreamExecutionSummary` from a raw child exit code
/// and emit it through the standard summary pipeline.
fn emit_minimal_composition_summary(
    provider: Provider,
    exit_code: i32,
    profile: &dyn WrapperProfile,
    env_context: &EnvironmentContext,
    dispatch_context: &HashMap<String, serde_json::Value>,
)
```

This function:
1. Constructs a `StreamExecutionSummary` with `exit_code` and
   `is_error = exit_code != 0` populated. All other fields (duration,
   tokens, cost, tool counts, assistant text) stay at their defaults so the
   rendered summary degrades gracefully to the fields that are actually
   knowable for a non-structured run.
2. Calls `emit_stream_summary_inner` (or `emit_composition_summary` with
   `section_stream: None`) to render to stderr **and** write the JSONL
   event. Both surfaces are produced — this is the full-parity behavior,
   not a JSONL-only fallback.

Once both call sites are converted, delete `emit_legacy_composition_session_event`.

### Phase 4: Collapse execute_direct_without_harness and execute_inline_without_harness

After phases 1-3, the two functions share enough logic to merge into a
single `execute_without_harness` function parameterized by an enum:

```rust
enum CompositionExecutionMode {
    Direct,
    Inline { closure_plan: InlineClosurePlan },
}
```

The merged function:

```rust
fn execute_without_harness(
    mode: CompositionExecutionMode,
    provider: Provider,
    profile: &dyn WrapperProfile,
    binary_path: &Path,
    child_args: &[String],
    child_env: &HashMap<OsString, OsString>,
    child_cwd: &Path,
    stdin_seed: Option<&str>,
    session_interactive: bool,
    resolved_path: &Path,
    use_structured: bool,
    structured_codex_output: Option<&StructuredCodexOutput>,
    stdout_noise: &[&str],
    stderr_noise: &[&str],
    stream_verbosity: Verbosity,
    detail_requested: bool,
    show_checks: bool,
    env_context: &EnvironmentContext,
    dispatch_context: &HashMap<String, serde_json::Value>,
    term: &Terminal,
    child_spawned: &mut bool,
) -> Result<i32>
```

The inline-specific steps — closure plan extraction, writability check
emission, frontmatter merge reporting, darkmatter cleanup — remain
inside an `if let CompositionExecutionMode::Inline { .. } = mode` block,
with comments explaining why they are inline-only:

```rust
// Inline-only: closure validation replaces the file body with the
// agent's response. Compose has no file mutation step, so this
// validation and reporting is exclusive to inline-compose.
if let CompositionExecutionMode::Inline { closure_plan } = &mode {
    // ... closure validation, file write, frontmatter merge ...
}
```

### Phase 5: Clean up dead code

After the merge:

- Delete `run_structured_inline` (absorbed into `run_structured_composition`)
- Delete `run_legacy_inline` (absorbed into the merged legacy branch)
- Delete `execute_direct_without_harness` (absorbed into `execute_without_harness`)
- Delete `execute_inline_without_harness` (absorbed into `execute_without_harness`)
- Delete `emit_stream_summary_no_separator` and `emit_stream_summary_no_separator_with_context`
  if no other callers remain (check `sequence.rs` and `mod.rs` harness loop)
- Delete `emit_legacy_composition_session_event`
- Delete the `InlineRunResult` type alias

## Call-Site Map

| Location | Current | After |
|----------|---------|-------|
| `composition.rs:814` | `execute_inline_without_harness(...)` | `execute_without_harness(Inline { closure_plan }, ...)` |
| `composition.rs:860` | `execute_direct_without_harness(...)` | `execute_without_harness(Direct, ...)` |
| `composition.rs:1121` | `emit_legacy_composition_session_event(...)` | `emit_minimal_composition_summary(...)` then delete |
| `composition.rs:1509` | `emit_legacy_composition_session_event(...)` | `emit_minimal_composition_summary(...)` then delete |

## Exception Registry

These differences between compose and inline-compose are **intentional** and
must be preserved with comments:

1. **Closure validation and file write** — inline-compose mutates the source
   file; compose does not. The closure extraction, frontmatter merge, and
   darkmatter cleanup are exclusive to inline.

2. **Deferred summary timing** — inline-compose must emit the summary after
   closure validation messages so the user sees file-write status before
   metadata. Compose emits immediately because there is no intermediate
   output. The `defer_section_separator` flag on
   `emit_composition_summary` selects between these timings.

3. **Interrupted-session body report** — inline-compose reports partial body
   content when interrupted (the file may have been partially filled).
   Compose does not write files so this is irrelevant.

4. **Writability pre-check** — inline-compose validates write permission on
   the target file before execution. Compose does not write files.

Note: the empty-text warning (`"the agent did not provide a summarized
message"`) is **not** on this list — it is being removed entirely from both
modes, not preserved as an inline-only exception.

## Files Changed

| File | Change |
|------|--------|
| `claudine/cli/src/commands/wrap/composition.rs` | Main refactor target: merge functions, delete legacy code |
| `claudine/cli/src/commands/wrap/mod.rs` | Possibly remove `emit_stream_summary_no_separator*` if no other callers |
| `claudine/cli/src/commands/wrap/sequence.rs` | Verify no dependency on removed functions; update if needed |

## Testing

- Existing tests in `composition.rs` (`split_frontmatter_*`, `cleanup_*`)
  remain unchanged — they test helper functions that are not affected.
- **Unit tests** on the new shared helpers:
  - `run_structured_composition` with a mocked child stream, asserting the
    returned `CompositionStreamResult` captures `had_streamed_assistant`,
    applies Codex post-hoc text, and does not emit the summary.
  - `emit_composition_summary` with `section_stream: None` and
    `section_stream: Some(...)`, asserting spacing and JSONL event shape.
  - `emit_minimal_composition_summary` asserting that a non-structured
    run with exit code 0 and exit code N each produce the expected stderr
    block and JSONL event.
- **Golden stderr comparison test**: run both `compose` and `inline-compose`
  against a mock provider (with no closure validation work — i.e., the
  inline-compose closure plan is a no-op so the deferred-summary timing
  does not produce divergence) and assert their stderr summary output is
  byte-identical. The only permitted divergence is the intentional
  exceptions documented above (file-write messages, closure validation,
  interrupted-body report, writability pre-check) — none of which should
  appear in this fixture's output.
- Verify `emit_legacy_composition_session_event` removal by confirming no
  grep matches remain in the codebase.
- Run `cargo test -p claudine-cli` after each phase.

## Delivery Model

- Multi-phased plan. Each phase is a logical checkpoint and should leave
  the codebase in a buildable, test-passing state before the next phase
  begins.
- All git work (commits, branches, PR creation, merges) is handled
  externally to this plan — the phase structure here is about
  implementation ordering, not about PR boundaries.
