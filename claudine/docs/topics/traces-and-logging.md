# Traces and Logging

Claudine produces structured JSONL logs for every wrapped session and dispatched event. These logs live under `~/.claudine/logs/` with one file per calendar day (`YYYY-MM-DD.jsonl`).

## Wrapper Environment Variables

When Claudine operates as a wrapper (`claudine <provider>`), it injects several environment variables into the provider process before spawn:

| Variable | Description |
|----------|-------------|
| `AGENT` | Provider name (e.g., `codex`, `claude`) |
| `YOLO` | `true` or `false` — whether auto-approval mode is active |
| `INTERACTIVE` | `true` or `false` — whether the session is interactive |
| `AGENT_PARAMS` | JSON array of provider-specific argv tokens |
| `CLAUDINE_SESSION_ID` | UUID for the current wrapped session |
| `CLAUDINE_PID` | Claudine's own process ID at wrapper startup |
| `PACKAGE_AREA` | Monorepo package area, when resolvable |
| `PACKAGE` | Monorepo package name, when resolvable |

### `CLAUDINE_PID`

`CLAUDINE_PID` is Claudine's own process ID. It is set in the provider environment **before** spawn so the provider and downstream consumers (logs, reports) can correlate back to the wrapper process.

### `AGENT_PID`

`AGENT_PID` is **not** injected into the provider environment. It represents the immediate child PID returned by `std::process::Command::spawn()` and is only available to Claudine-controlled contexts **after** a successful spawn. The provider process is not required to receive this value.

## PID Fields in JSONL Logs

Wrapped session lifecycle records carry two PID-related fields:

- **`env.claudine_pid`** — present on wrapper-emitted records. Derived from `CLAUDINE_PID` or the wrapper's own process ID.
- **`agent_pid`** — present only after a successful provider spawn. Omitted from raw JSONL when no child PID is available.

Report and query outputs expose a stable nullable `agent_pid` field, where `null` means no provider child PID was available for that row.
