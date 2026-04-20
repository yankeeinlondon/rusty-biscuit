# Defensive Programming & Observability Review: Claudine

This document outlines the findings and suggestions from a defensive programming and observability review of the `claudine` library and CLI. The focus is on preventing silent failures, avoiding hangs, and ensuring high-fidelity tracing and metrics.

## 1. Defensive Programming: Preventing Hangs

### Findings
- **Hook Action Timeouts:** `HookAction::Call` in `lib/src/dispatch/runner.rs` correctly implements a 60-second default timeout using `tokio::time::timeout`.
- **Process Escalation:** The wrapper's `run_child` and `run_child_stream` in `cli/src/commands/wrap/exec.rs` implement robust signal escalation (SIGINT -> SIGTERM -> SIGKILL) and optional timeouts.
- **Pipe Deadlock Protection:** `run_child_stream` uses dedicated threads for `stdout` and `stderr` processing, effectively preventing deadlocks when the child process fills its output pipes.
- **Synchronous I/O in Async Contexts:** Several areas perform synchronous I/O or SQLite operations within `async` functions without using `tokio::task::spawn_blocking`.
    - `claudine/cli/src/commands/logs.rs` calls `ReportingStore::sync` and other query methods directly.
    - `claudine/lib/src/dispatch/mod.rs` calls `loader::load_runtime_config` which performs file I/O and regex compilation synchronously.

### Suggestions
- **[High] Wrap Synchronous Store Calls:** In `claudine/cli/src/commands/logs.rs`, all calls to `ReportingStore` (especially `sync`) should be wrapped in `tokio::task::spawn_blocking`. While typically fast on local SSDs, these operations can block the Tokio reactor during large syncs or on slow filesystems.
- **[Medium] Async Config Loading:** Consider moving `load_runtime_config` to use `tokio::fs` or wrapping it in `spawn_blocking`. Since it also performs regex compilation, it can be computationally expensive if many hooks are defined.
- **[Low] Stdin Safety:** The `handle` command correctly bails if stdin is a TTY and empty. Ensure other commands that consume stdin (like `compose`) implement similar checks to avoid hanging in non-interactive sessions.

## 2. Defensive Programming: Preventing Silent Failures

### Findings
- **Failure Isolation:** Claudine follows a strong "failure isolation" strategy where non-critical action failures (TTS, sound effects, log server posts) are logged as `warn!` but do not propagate errors that would stop the agent's execution.
- **Call Mapper Resiliency:** `HookAction::Call` logs a `warn!` if a mapper fails, ensuring the agent session continues even if a single hook produces malformed output.
- **Reporting Sync:** `best_effort_sync` in the CLI logs warnings for parse failures but continues the sync process.

### Suggestions
- **[Medium] Standardized Exit Codes:** Align with the Gemini CLI standard for exit codes:
    - `41`: Auth/Permission errors.
    - `44`: Sandbox/Environment errors.
    - `52`: Configuration errors.
    - Currently, Claudine uses `2` for "Stop Session" and standard `1`/`0` elsewhere.
- **[Medium] Atomic Write Validation:** Ensure that `atomic_write` in `claudine/lib/src/config/atomic.rs` (if used for all config mutations) includes a verification step to ensure the written file is semantically valid before replacing the original.
- **[Low] I/O Error Context:** Many `Io` errors are wrapped via `thiserror` but lack path context. Using `anyhow` or `eyre` style context, or including the `PathBuf` in custom error variants, would improve debuggability.

## 3. Observability: Tracing

### Findings
- **Internal Tracing:** `tracing` is used effectively for internal spans (e.g., `dispatch_event`, `hook_action`, `stream_parse`).
- **Span Metadata:** Spans correctly capture `session_id`, `child_pid`, and `provider`.

### Suggestions
- **[High] OpenTelemetry Integration:** As planned in `claudine/docs/research/agent-observability/integration-strategies.md`, implement "Phase 1" by adding `tracing-opentelemetry`. This will allow Claudine to export traces to OTLP-compatible backends (Langfuse, Jaeger, etc.).
- **[Medium] GenAI Semantic Conventions:** Adopt [OpenTelemetry GenAI semantic conventions](https://github.com/open-telemetry/semantic-conventions/blob/main/docs/gen-ai/gen-ai-metrics.md) for span attributes (e.g., `gen_ai.operation.name`, `gen_ai.request.model`).
- **[Medium] Distributed Tracing:** Ensure that `CLAUDINE_SESSION_ID` is propagated as a trace parent or linked span when wrapping child processes to maintain a single distributed trace.

## 4. Observability: Metrics

### Findings
- **Custom Aggregate Metrics:** Metrics are currently computed post-hoc via JSONL-to-SQLite aggregation. This is excellent for historical reporting but lacks real-time operational visibility.
- **Reporting Index:** The reporting index captures detailed usage (tokens, cost) and performance (duration).

### Suggestions
- **[Medium] Real-time Metrics:** Integrate a metrics library (e.g., `metrics` crate) to emit real-time counters and histograms for:
    - Event dispatch counts (per provider/event).
    - Hook execution latency.
    - Protection service hit rates and outcomes.
    - Child process start/exit counts.
- **[Medium] Standardized Token Metrics:** Ensure all providers' token usage is mapped to the standard GenAI metric names: `gen_ai.client.token.usage`.

## 5. Summary of Recommended Actions

1.  **Refactor `claudine/cli/src/commands/logs.rs`** to use `spawn_blocking` for all `ReportingStore` interactions.
2.  **Add `tracing-opentelemetry` and `opentelemetry-otlp`** to `claudine/lib/Cargo.toml` and initialize the OTLP exporter in the CLI.
3.  **Audit Exit Codes** across the CLI to match the standard 40/50-series codes for specific failure categories.
4.  **Enhance `Permissions` and `MCP` logging** with `debug!` and `trace!` level instrumentation to assist in troubleshooting complex configuration merges.
