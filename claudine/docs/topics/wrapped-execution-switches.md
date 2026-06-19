# CLI Switches in Wrapped Execution

The Claudine _wrapper_ commands like `claudine claude`, `claudine opencode`, etc. all have a common set of CLI switches and inject a standard set of environment variables into the provider process.

## Environment Variables

The wrapper sanitizes the parent environment and injects the following variables before spawning the provider:

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

### PID Distinction

- **`CLAUDINE_PID`** — Claudine's own process ID, available to the provider environment before spawn.
- **`AGENT_PID`** — The immediate child PID returned by the spawn operation. This is **not** injected into the provider environment; it is only available to Claudine-controlled contexts (logs, reports, hooks) after a successful spawn.
