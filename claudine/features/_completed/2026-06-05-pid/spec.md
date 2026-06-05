# PID Capture for Wrapped Agentic CLIs

## Functional Requirements

- When Claudine starts, it captures its own process ID.
- When Claudine operates as a wrapper for an agentic CLI, it exposes Claudine's process ID as `CLAUDINE_PID`.
- `CLAUDINE_PID` is set in the wrapped provider process environment before the provider process is spawned.
- When Claudine successfully spawns the wrapped agentic CLI process, it captures the immediate child process ID returned by the spawn operation.
- `AGENT_PID` means the immediate child process ID returned by the wrapper spawn operation, such as Rust's `Command::spawn()` child ID.
- `AGENT_PID` is available only after a successful spawn, because the child process ID is not known before spawn.
- `AGENT_PID` is exposed in Claudine-controlled contexts after successful spawn.
- PID capture applies when Claudine is operating as a wrapper for an agentic CLI, regardless of whether the wrapped session is interactive or non-interactive.

## Environment Variables

### `CLAUDINE_PID`

- Value: Claudine's own process ID.
- Availability: set in the provider process environment before spawn.

### `AGENT_PID`

- Value: the immediate child process ID returned by the wrapper spawn operation.
- Availability: available to Claudine-controlled contexts after successful spawn.
- The provider process is not required to receive `AGENT_PID` in its environment.

## Structured Logs and Reports

- Structured logs and reports for wrapped agentic CLI sessions must include PID information on wrapper session lifecycle records and Claudine-controlled hook, action, log, and report contexts.
- Provider stream records are not required to carry duplicate PID fields unless they are also emitted as Claudine-controlled hook, action, log, or report context records.
- Structured logs and reports use snake_case field names:
    - `claudine_pid` maps to the `CLAUDINE_PID` environment/context variable.
    - `agent_pid` maps to the `AGENT_PID` context variable.
- When Claudine has captured its own PID, required structured records include `claudine_pid`.
- When the wrapped provider process has been spawned successfully, required structured records include `agent_pid`.
- Raw structured logs omit `agent_pid` when no provider child PID is available for that record.
- Report and query outputs expose stable nullable `agent_pid` fields or columns. In those outputs, `agent_pid: null` means no provider child PID was available for that output row.

## Non-Requirements

- `AGENT_PID` is not required to resolve through provider shims, shells, launchers, or wrapper scripts to find a final provider executable PID.
- `AGENT_PID` is not a process group ID or session leader PID.
- `AGENT_PID` does not have provider-specific semantics.
- Claudine is not required to use an additional shim only to make `AGENT_PID` visible inside the provider process environment.

## Acceptance Criteria

- In wrapper mode, the provider process receives `CLAUDINE_PID` in its environment before spawn.
- After a successful provider spawn, Claudine records the spawned child process ID as `AGENT_PID`.
- Wrapper session lifecycle records and Claudine-controlled hook, action, log, and report contexts include `claudine_pid` when available.
- Wrapper session lifecycle records and Claudine-controlled hook, action, log, and report contexts include `agent_pid` after successful provider spawn.
- Provider stream records are not required to include PID fields solely because they occur during a wrapped provider session.
- Raw structured logs omit unavailable `agent_pid` values.
- Report and query outputs expose a stable nullable `agent_pid` field or column, with null meaning no provider child PID was available for that output row.
