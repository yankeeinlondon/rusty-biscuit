# Response Refinement Review 3

## Findings

1. **High: the section model is still only partially implemented, so the spec's sink-level spacing contract is not actually enforced for final stdout or trailer metadata.**
   `LiveSemanticSink` explicitly says `FinalStdout` and trailer wiring are still pending in [claudine/cli/src/commands/wrap/live_semantic_sink.rs](../../cli/src/commands/wrap/live_semantic_sink.rs#L443), and `OutputText` still bypasses the section tracker entirely via the raw callback path in [live_semantic_sink.rs](../../cli/src/commands/wrap/live_semantic_sink.rs#L571). The wrapped execution path still has open TODOs to route trailer lines through `sink.emit_trailer_line()` in [wrap/mod.rs](../../cli/src/commands/wrap/mod.rs#L1369) and [wrap/mod.rs](../../cli/src/commands/wrap/mod.rs#L1798), while `emit_stream_summary_inner` still does its own ad hoc newline management with direct `eprint!/eprintln!` calls in [wrap/mod.rs](../../cli/src/commands/wrap/mod.rs#L2966). That means Child 3 is not actually finished as specified: the “9-section model” is not the single authority for stdout/stderr spacing yet.

2. **High: unknown tool argument/result shapes still lose information instead of falling back to wrapped raw JSON.**
   The spec requires “never lose information” and says unknown tool shapes should fall back to raw JSON. The implementation does the opposite: `extract_tool_summary()` only returns strings from a small key allowlist or the first top-level string, then returns `None` for everything else in [claudine/lib/src/stream/tool_display.rs](../../lib/src/stream/tool_display.rs#L168). `ToolCallDisplay::from_call()` and `from_result()` then propagate that `None` directly in [tool_display.rs](../../lib/src/stream/tool_display.rs#L244). The unit test at [tool_display.rs](../../lib/src/stream/tool_display.rs#L237) locks this behavior in by asserting `None` for an object with no strings. In practice this regresses back to opaque `🔧 → Tool` / `🔧 ← Tool` lines for any tool whose most relevant context is numeric, nested, or otherwise not in the hard-coded key list.

3. **Medium: the canonical tool-line styling contract is only half implemented.**
   The spec calls for the summary/status slot to be dim-italic by default, with only the word `error` additionally forced red+bold. The current formatter only applies prose markup in the error branch. Success, pending, and summary text are emitted as plain strings in [claudine/cli/src/commands/wrap/live_semantic_sink.rs](../../cli/src/commands/wrap/live_semantic_sink.rs#L307), and those paths go through `Status::new(...)` instead of `Status::from_prose(...)` in [live_semantic_sink.rs](../../cli/src/commands/wrap/live_semantic_sink.rs#L293). That means the rendered output does not match the canonical format the spec asks for, even though the content is mostly there.

4. **Medium: the tests prove parser pieces and sink pieces, but not the full OpenCode wrapper regression that motivated the feature.**
   The OpenCode work is covered at the parser level in [claudine/lib/src/stream/opencode_semantic.rs](../../lib/src/stream/opencode_semantic.rs#L696), at the sink callback level in [claudine/cli/src/commands/wrap/live_semantic_sink.rs](../../cli/src/commands/wrap/live_semantic_sink.rs#L1165), and at argv-forwarding level in [claudine/cli/tests/wrap_direct_argv.rs](../../cli/tests/wrap_direct_argv.rs#L18). I could not find an equivalent `wrap_commands.rs` end-to-end regression test that actually runs `claudine opencode ...`, asserts the assistant response reaches stdout, asserts there is no synthesized outgoing tool line, and asserts the trailer spacing/order after real streamed output. By contrast, Codex and Gemini do have wrapper-level stdout/trailer tests in [claudine/cli/tests/wrap_commands.rs](../../cli/tests/wrap_commands.rs#L1113) and [wrap_commands.rs](../../cli/tests/wrap_commands.rs#L1293). Given that the original bug was in the wrapped pipeline rather than just the parser, that missing integration coverage is still a real risk.

5. **Low: the current spacing tests mostly validate stderr snapshots, not the spec's combined stdout+stderr emission-order acceptance criterion.**
   The fixture replay test that claims to cover blank-line normalization only checks `stderr_lines` in [claudine/cli/src/commands/wrap/live_semantic_sink.rs](../../cli/src/commands/wrap/live_semantic_sink.rs#L1980). That is weaker than the spec, which defines the invariant against the combined rendered output in emission order. This matters because the current implementation still routes final stdout and final metadata outside the shared section tracker.

## Additional Suggestions

- Cache the `Terminal` inside `LiveSemanticSink` instead of rebuilding it on every status/thinking render. `wrap_terminal()` constructs a fresh terminal object for each render path in [claudine/cli/src/commands/wrap/live_semantic_sink.rs](../../cli/src/commands/wrap/live_semantic_sink.rs#L53). That is unnecessary work in long-running structured sessions with many tool/info events.

## Validation

- `cargo test -p claudine gemini_semantic --lib`
- `cargo test -p claudine opencode_semantic --lib`
- `cargo test -p claudine-cli live_semantic_sink`

All three passed locally.
