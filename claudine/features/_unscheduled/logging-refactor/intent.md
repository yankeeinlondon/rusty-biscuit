# Intent: Logging Refactor

This document reverse-engineers the original "intent" for Claudine's logging system, identifies the current implementation gaps, and defines the target architecture for a unified event logging service.

## 1. Original Intent (Reverse-Engineered)

Based on existing type definitions (`LogTarget`, `default_log_target`) and discarded comments, the intent for Claudine's logging system was:

1. **Structured Event Auditing:** Every lifecycle event (SessionStart, ToolCall, etc.) should be capturable as a structured JSON record.
2. **Flexible Destinations:** Users should be able to log to:
    * **Local Files:** JSONL format with optional daily rotation.
    * **Remote Servers:** Structured POST requests to an HTTP endpoint for centralized auditing/monitoring.
3. **Configurable Granularity:**
    * **Global Logging:** A default target that captures all enabled events.
    * **Per-Event Logging:** The ability to add a `log` action to specific event bindings with custom targets.
4. **Fire-and-Forget Execution:** Logging should happen in the background without blocking the agent's turn or the tool execution pipeline.

## 2. Current Problems

The current implementation is "riddled with problems" because it is mostly a collection of disconnected boilerplate and dead code:

### 2.1. The "Log" Action is Missing
While `LogTarget` exists in the codebase, the `HookAction` enum (which defines what Claudine can actually *do* when an event fires) is missing a `Log` variant. Consequently, it is currently impossible to configure a logging action in `config.json`.

### 2.2. Dead Config Fields
`GlobalSettings` contains a `default_log_target` field, but it is never consumed by the dispatch runtime. It exists as a serialized field that "does nothing."

### 2.3. Dual Config Models
The codebase is in the middle of a migration from `HookerConfig` (per-provider) to `ClaudineConfig` (canonical/flat). 

* `HookerConfig` has `default_log_target` (structural but unused).
* `ClaudineConfig` has a simple `logging: bool` (semantic but insufficient).
Neither model correctly integrates with a functional logging service.

### 2.5. Conceptual Confusion: Native vs. Claudine Logging
The codebase also contains `LoggingCapabilities` within the `AgentCapabilities` struct (`lib/src/agents/model.rs`). This refers to the *native* logging capabilities of the agents Claudine wraps (e.g., where Claude Code stores its session logs). This is distinct from, and often confused with, Claudine's own *event logging* service. A refactor must clearly decouple these two concepts.

## 3. Proposed Architecture

The logging refactor should align the codebase with the original intent:

### 3.1. Unified Logging Service
Implement a `LoggingService` (likely in `claudine-lib`) that manages a pool of `LogTarget` writers. This service should be initialized once at the start of a `DispatchRuntimeContext`.

### 3.2. Explicit "Log" Action
Add `Log` to `HookAction`:

```rust
pub enum HookAction {
    // ... existing ...
    Log {
        /// Optional override for the target. If None, uses default_log_target.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        target: Option<LogTarget>,
    },
}
```

### 3.3. Update `ClaudineConfig`
Replace `logging: bool` with a more expressive configuration:

```rust
pub struct ClaudineConfig {
    // ...
    #[serde(default)]
    pub logging: LoggingSettings,
}

pub struct LoggingSettings {
    pub enabled: bool,
    pub default_target: Option<LogTarget>,
    /// Automatically log all events to the default target even without explicit actions.
    pub auto_log_all: bool,
}
```

### 3.4. Background Dispatch
When a `Log` action is executed (or `auto_log_all` is enabled), the event metadata should be cloned and sent to a background task (using `tokio::mpsc` or similar) to ensure that slow file I/O or HTTP timeouts never affect the agent's performance.

### 3.5. Standardized Payload
All loggers should emit a standardized JSON object based on `EventMeta`, including:

* Timestamp (ISO 8601)
* Provider & Event type
* Session ID & CWD
* Tool name & input/output (truncated if necessary)
* Environment context (Git branch, repo name)
