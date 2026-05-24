---
ready: true
agent: ${env.AGENT}
---

# Review: Kimi Code CLI Wire Mode Implementation

The implementation of the Kimi Code CLI wire mode is highly thorough and robust. It effectively addresses the silent-drop bug by switching to Kimi's structured JSON-RPC 2.0 `--wire` protocol and achieves strong feature parity with other structured providers.

## Positive Observations

- **Comprehensive Protocol Modeling**: `claudine/lib/src/stream/protocol/kimi.rs` provides a detailed and type-safe model for Kimi's JSON-RPC envelopes, including notifications, requests, and responses.
- **Robust Semantic Parsing**: `claudine/lib/src/stream/kimi_semantic.rs` correctly maps a wide range of Kimi wire events to the unified `SemanticEvent` model, supporting over 20 distinct event types (exceeding the 16 core lifecycle events).
- **Tested Bidirectional Communication**: The new `wire_io.rs` module manages the bidirectional session with appropriate auto-responses for initialize, approval, and question requests, ensuring the agent doesn't hang waiting for client input.
- **Strong Test Coverage**: The implementation includes exhaustive unit tests for protocol parsing, semantic mapping, and the wire IO loop, as well as integration tests for round-trip fidelity.
- **Signal Handling**: Correct implementation of SIGINT forwarding to send a JSON-RPC `cancel` request before falling back to hard SIGTERM/SIGKILL escalation.

## Identified Gaps & Suggestions

### 1. Missing Timing Monitor (Feature Parity)
In `claudine/cli/src/commands/wrap/mod.rs`, the `run_kimi_wire_session` call currently ignores the `prompt_timing` context.
- **Impact**: Kimi runs in composition mode (`compose`, `inline-compose`, `sequence`) do not emit the periodic timing headers (`t=0`, `t=10m`, etc.) or the two-stage timeout warnings that other providers (like Claude and OpenCode) support.
- **Recommendation**: Make `exec::spawn_prompt_timing_monitor` and `exec::stop_timing_ticker` available to the `wire_io` module (change visibility to `pub(crate)`) and wire them into the `run_kimi_wire_session` loop.

### 2. Stderr Bridge Ignored
The `stderr_bridge` is also ignored in the `run_kimi_wire_session` branch.
- **Impact**: While Kimi's primary events are now on stdout, any out-of-band diagnostics or rate-limit information emitted by the Kimi CLI to stderr will bypass the classification logic.
- **Recommendation**: Integrate the `stderr_bridge` into `run_kimi_wire_session` to ensure all stderr output is correctly classified and merged into the final execution summary.

### 3. Logic Duplication in Launch Paths
There is some duplication of setup logic (building sinks, plumbing, and dispatch context) in the harness attempt loop between the `wire_prompt` branch and the default `run_child_stream_semantic` branch.
- **Suggestion**: Consider refactoring the launch dispatch in `claudine/cli/src/commands/wrap/mod.rs` to unify how these resources are passed to the underlying execution functions, reducing the risk of drift between the two transport modes.

## Production Readiness

The feature is **ready for production**. The identified gaps are primarily around secondary telemetry (timing headers) and edge-case stderr classification, which do not affect the core correctness of the assistant text delivery or tool-calling functionality. The switch to wire mode significantly improves the reliability and observability of Kimi-based workflows.
