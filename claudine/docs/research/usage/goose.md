---
$schema: ./_schema.yaml
created: 2026-07-03
last_updated: 2026-07-03
agent: codex
model: default
api: true
cli_switch: true
structured_output: true
pty_scrape: true
api_methods:
  - available: true
    endpoint: "GET /sessions/{session_id}"
    auth: api_key
    users: "Users running the local Goose server with the server secret; not a hosted subscriber quota endpoint."
    fields: ["usage.total_tokens", "usage.input_tokens", "usage.output_tokens", "usage.cache_read_input_tokens", "usage.cache_write_input_tokens", "accumulated_usage.total_tokens", "accumulated_usage.input_tokens", "accumulated_usage.output_tokens", "accumulated_cost"]
    reset_window: "none; local session lifetime only"
    notes: "Returns local session token and cost fields from Goose session storage; does not report subscription limits, headroom, or reset time."
  - available: true
    endpoint: "GET /sessions"
    auth: api_key
    users: "Users running the local Goose server with the server secret; not a hosted subscriber quota endpoint."
    fields: ["sessions[].id", "sessions[].name", "sessions[].updated_at", "sessions[].message_count"]
    reset_window: "none"
    notes: "Lists sessions so an inspecting tool can choose a session ID before calling GET /sessions/{session_id}; usage fields are on the per-session response."
  - available: false
    endpoint: "unknown hosted usage/quota endpoint"
    auth: unknown
    users: "unknown"
    fields: []
    reset_window: "unknown"
    notes: "No official Goose CLI documentation or inspected source path exposes provider-plan quota windows, remaining headroom, or reset timestamps."
cli_methods:
  - available: true
    invocation: "goose run --text '<prompt>' --output-format json"
    interactive_only: false
    output_format: "json"
    fields: ["metadata.total_tokens", "metadata.input_tokens", "metadata.output_tokens", "metadata.status"]
    notes: "Structured output after a run; reports local session token usage, not provider quota windows."
  - available: true
    invocation: "goose run --text '<prompt>' --output-format stream-json"
    interactive_only: false
    output_format: "newline-delimited JSON events"
    fields: ["type=complete.total_tokens", "type=complete.input_tokens", "type=complete.output_tokens"]
    notes: "Structured stream-complete event after a run; reports local session token usage, not provider quota windows."
  - available: true
    invocation: "goose run --text '<prompt>' --stats"
    interactive_only: false
    output_format: "human text on stderr"
    fields: ["time_to_first_token", "tokens_per_second", "output_tokens", "draft_accept_rate"]
    notes: "Performance and output-token statistics for the just-finished run; no limit or reset information."
  - available: true
    invocation: "goose session export --session-id <id> --format json"
    interactive_only: false
    output_format: "json"
    fields: ["usage", "accumulated_usage", "accumulated_cost", "messages"]
    notes: "Structured local session export preserving usage fields; requires a known session ID."
  - available: true
    invocation: "goose session list --format json"
    interactive_only: false
    output_format: "json"
    fields: ["sessions"]
    notes: "Structured session discovery; useful before session export or local-server session lookup."
  - available: true
    invocation: "goose term info"
    interactive_only: false
    output_format: "compact human text"
    fields: ["context_usage_dots", "model"]
    notes: "Reads AGENT_SESSION_ID and prints a five-dot context-usage indicator plus model name; no numeric JSON and no provider quota."
  - available: true
    invocation: "interactive prompt context line before each input"
    interactive_only: true
    output_format: "human TTY text"
    fields: ["context_usage_percent", "total_tokens", "context_limit", "estimated_cost_if_enabled"]
    notes: "Interactive sessions display context usage before each prompt and may display estimated cost when GOOSE_CLI_SHOW_COST is enabled."
  - available: false
    invocation: "/status"
    interactive_only: true
    output_format: "unknown"
    fields: []
    notes: "The help text advertises /status, but the inspected slash-command parser does not implement a /status branch."
pty_design:
  command: "goose session --resume --session-id <id>"
  first_pass_markers: ["% ", "/", "Cost:", "> "]
  fuzzy_markers: ["context", "tokens", "model", "Cost", "USD", "●", "○"]
  fields: ["context_usage_percent", "total_tokens", "context_limit", "estimated_cost", "model"]
  risks: "TTY text has no schema or compatibility contract; /status appears documented but unimplemented in inspected source, so scraping must target the prompt-adjacent context line and tolerate wording, glyph, color, and width drift."
metrics:
  - name: "session total tokens"
    unit: tokens
    window: session
    source: "GET /sessions/{session_id}; session export; run JSON metadata; sessions.db"
    notes: "Local Goose session usage; not a subscription window."
  - name: "session input tokens"
    unit: tokens
    window: session
    source: "GET /sessions/{session_id}; session export; run JSON metadata; sessions.db"
    notes: "Local Goose session usage."
  - name: "session output tokens"
    unit: tokens
    window: session
    source: "GET /sessions/{session_id}; session export; run JSON metadata; sessions.db"
    notes: "Local Goose session usage."
  - name: "cache read and write input tokens"
    unit: tokens
    window: session
    source: "GET /sessions/{session_id}; session export; sessions.db"
    notes: "Stored when providers report cache token breakdowns."
  - name: "accumulated session cost"
    unit: currency
    window: session
    source: "GET /sessions/{session_id}; session export; sessions.db"
    notes: "Optional local estimate or accumulated value; not provider billing-account spend."
  - name: "context usage percentage"
    unit: percent
    window: session
    source: "interactive context line; goose term info approximation"
    notes: "Percent of model context window consumed by the current session; reset is session compaction/clear/new session, not time."
  - name: "provider subscription quota headroom"
    unit: unknown
    window: unknown
    source: "not found"
    notes: "No on-demand Goose mechanism found for 5-hour, weekly, billing-cycle, credit, or remaining-headroom windows."
limit_states:
  - state: cap_approaching
    detectable: false
    source: "on-demand inspection"
    markers: []
    notes: "No inspected API, CLI, slash command, or local artifact reports approaching provider-plan caps."
  - state: capped
    detectable: true
    source: "provider error during a run"
    markers: ["ProviderError::RateLimitExceeded", "Rate limit exceeded:", "telemetry_type=rate_limit"]
    notes: "Runtime detection belongs to the sibling non-interactive-sessions topic; no preflight usage inspection path was found."
  - state: no_funds
    detectable: true
    source: "provider error or system notification during a run"
    markers: ["ProviderError::CreditsExhausted", "Credits exhausted:", "SystemNotificationType::CreditsExhausted", "top_up_url"]
    notes: "Runtime detection only; no preflight balance endpoint found."
  - state: auth_required
    detectable: true
    source: "goose info --check; local server API"
    markers: ["Auth: FAILED", "Authentication error:", "HTTP 401", "X-Secret-Key"]
    notes: "Distinguishes missing or invalid provider auth and local server API secret failures, but not usage cap state."
docs: https://block.github.io/goose/docs/guides/logs
changes: []
requires_claudine_update: true
reason: "Claudine can add Goose session/context token inspection from local JSON/API/SQLite surfaces, but should explicitly model provider quota windows as unsupported instead of trying to infer 5-hour or weekly runway."
---

# Goose CLI Usage Inspection

## Introduction to Goose CLI Usage Inspection

Goose CLI does not define its own hosted subscription or quota model. It is an open-source local agent that sends model calls to the configured provider, so billing limits, credits, rate limits, and reset windows belong to Anthropic, OpenAI, Google, OpenRouter, Ollama, or another selected provider rather than to Goose itself. Goose does track local session token usage, context-window consumption, optional cost estimates, telemetry counters, and provider error classes.

Official Goose surfaces today are local rather than account-plan oriented: the CLI can print or export session token usage, the local Goose server can return session records over authenticated HTTP, diagnostics can bundle session/config/log data, and the local SQLite session database stores token fields. No inspected official documentation or source path exposed an on-demand Goose command, API, or file that reports 5-hour, weekly, billing-cycle, credit, remaining-headroom, or reset-window values.

## API Call Opportunities

| Mechanism | Endpoint | Auth | Users | Usage Fields | Reset Window |
|-----------|----------|------|-------|--------------|--------------|
| Local Goose server session lookup | `GET /sessions/{session_id}` | `X-Secret-Key` local server secret | Any user running Goose server with the secret | `usage`, `accumulated_usage`, `accumulated_cost`, messages | None; session lifetime |
| Local Goose server session listing | `GET /sessions` | `X-Secret-Key` local server secret | Any user running Goose server with the secret | Session IDs and metadata for follow-up lookup | None |
| Hosted Goose quota API | unknown | unknown | unknown | unknown | unknown |

Example local session request:

```sh
curl -sS \
  -H "X-Secret-Key: $GOOSE_SERVER__SECRET_KEY" \
  http://127.0.0.1:3284/sessions/20260703_1
```

The local server authentication middleware bypasses auth for `/status`, `/mcp-app-proxy`, and `/mcp-app-guest`, but all other routes require the `X-Secret-Key` header to match the configured server secret. The source documents `/sessions/{session_id}` as returning a `Session`, and the `Session` type contains `usage`, `accumulated_usage`, and `accumulated_cost` fields. The generated OpenAPI document also marks `/sessions` and `/sessions/{session_id}` as `api_key` protected and documents `401` for invalid or missing API key.

Example response shape, distilled from the source schemas:

```json
{
  "id": "20260703_1",
  "usage": {
    "input_tokens": 12000,
    "output_tokens": 900,
    "total_tokens": 12900,
    "cache_read_input_tokens": 8000,
    "cache_write_input_tokens": 500
  },
  "accumulated_usage": {
    "input_tokens": 48000,
    "output_tokens": 3200,
    "total_tokens": 51200
  },
  "accumulated_cost": 0.42
}
```

These endpoints are useful for local session accounting but are not plan-limit APIs. They do not carry reset timestamps, window lengths, remaining requests, remaining credits, subscription tier, or billing-cycle state. A negative probe is therefore part of the finding: the local Goose server API has `/status`, `/system_info`, `/diagnostics/{session_id}`, and session-management routes, but no inspected route named usage, quota, billing, credits, limits, or reset.

## CLI Switch Opportunities

| Invocation | Non-Interactive | Structured | What It Yields | Limitations |
|------------|-----------------|------------|----------------|-------------|
| `goose run --text '<prompt>' --output-format json` | Yes | Yes | Final JSON with `metadata.total_tokens`, `metadata.input_tokens`, `metadata.output_tokens`, and `status` | Requires making a run; no quota windows |
| `goose run --text '<prompt>' --output-format stream-json` | Yes | Yes | JSON events ending in `type: "complete"` with token fields | Requires making a run; no quota windows |
| `goose run --text '<prompt>' --stats` | Yes | No | Time to first token, tokens/sec, output tokens, speculative draft stats when present | Performance stats only |
| `goose session list --format json` | Yes | Yes | Session inventory for choosing a session ID | Session discovery only |
| `goose session export --session-id <id> --format json` | Yes | Yes | Full session JSON, including usage fields | Requires existing session ID |
| `goose session diagnostics --session-id <id>` | Yes | Yes | Diagnostics JSON with session, config, logs, prompts, and errors | Troubleshooting bundle; may include sensitive conversation data |
| `goose term info` | Yes, if `AGENT_SESSION_ID` is set | No | Five-dot context usage indicator and model name, for shell prompts | Coarse and glyph-based |

The CLI command guide documents `session list --format json`, `session export --format json`, `session diagnostics`, and `run --output-format text|json|stream-json`. The CLI parser source confirms those switches and adds `--stats` to `goose run`. The implementation of JSON and stream-JSON run output emits accumulated or current session token fields in `metadata` and `complete` events.

`goose term info` is a lightweight shell-prompt command. It reads `AGENT_SESSION_ID`, loads that session, computes `total_tokens / context_limit`, and prints five filled/empty dots plus the shortened model name, for example:

```text
●○○○○ sonnet
```

This is context-window consumption, not subscription quota. It has no JSON flag.

## Interactive Commands and PTY Scraping

Interactive Goose sessions display context usage before each input prompt. The code calls `display_context_usage()` at the top of the interactive loop, which renders a bar, a percentage, and a `used/limit` token pair. When `GOOSE_CLI_SHOW_COST=true`, it also prints an estimated cost line based on Goose's canonical model price data.

The help text advertises:

```text
/status - Show session status: model, provider, mode, and token usage.
```

However, the inspected slash-command parser has no `/status` constant or match arm. Unknown slash input falls through as a normal message. This is a documented/source drift: `/status` should not be treated as a reliable usage-inspection command until verified in an installed Goose version.

PTY scraping design with `expectrl`:

| Pass | Strategy | Markers | Fields |
|------|----------|---------|--------|
| First pass | Spawn `goose session --resume --session-id <id>` in a PTY, wait for the prompt-adjacent context line and then the `> ` input prompt. | percent marker `%`, token separator `/`, optional `Cost:`, final prompt `> ` | `context_usage_percent`, `total_tokens`, `context_limit`, optional `estimated_cost` |
| Second pass | If the exact parse fails, search the visible screen buffer for nearby usage terms and numeric patterns. | `context`, `tokens`, `model`, `Cost`, `USD`, `●`, `○` | Same fields when recoverable; otherwise return unknown with captured evidence |

The scraper must treat TUI text as a last resort. It has no schema, no version, no stability contract, and no validation layer. Color, glyphs, line wrapping, width, provider-specific cost availability, and the apparent `/status` help drift can all break exact matching. A robust Claudine implementation should prefer JSON/session APIs or SQLite reads and reserve PTY scraping for context-window display only.

## Config and Log Artifacts

| Artifact | Path | Fields | Freshness | Host Observation |
|----------|------|--------|-----------|------------------|
| Config YAML | `~/.config/goose/config.yaml` on documented Unix-like paths; source uses platform app strategy with backward-compatible `Block/goose` directories | Provider/model config, telemetry setting, context and display settings | Updated by `goose configure` or env overrides | Missing under `/Users/ken/.config/goose` and `/Users/ken/Library/Application Support/Block/goose` |
| Sessions database | `~/.local/share/goose/sessions/sessions.db` on documented Unix-like paths; source constant `sessions/sessions.db` under Goose data dir | `total_tokens`, `input_tokens`, `output_tokens`, cache token fields, accumulated token fields, `accumulated_cost`, provider/model config | Updated during and after sessions | Missing on this host |
| CLI/server logs | `~/.local/state/goose/logs/cli/`, `~/.local/state/goose/logs/server/` | Operational events, session IDs, errors, tool activity | Written live and retained for a limited window | Missing on this host |
| LLM request logs | `~/.local/state/goose/logs/llm_request.*.jsonl` | Model configuration, request payload, response data, token usage information | Rotating request logs; 10 most recent completed requests | Missing on this host |
| Diagnostics JSON | Produced by `goose session diagnostics --session-id <id>` or Desktop diagnostics | `system`, `session`, `config`, `logs`, `prompts`, `schedule`, `errors` | Generated on demand | No local Goose installation/artifacts available to generate |

On this host, no Goose binary was found with `command -v goose`, and the checked provider directories under `/Users/ken` were absent. The directories checked were:

- `/Users/ken/.config/goose`
- `/Users/ken/.local/share/goose`
- `/Users/ken/.local/state/goose`
- `/Users/ken/Library/Application Support/Block/goose`
- `/Users/ken/Library/Application Support/goose`
- `/Users/ken/Library/Application Support/Goose`
- `/Users/ken/Library/Logs/goose`
- `/Users/ken/Library/Logs/Block/goose`

Because no local artifact exists to inspect, source-defined schemas and official docs are the evidence for fields. The sessions database is the most valuable artifact when present because it is structured, local, and queryable without launching an agent run.

## Metrics and Windows

| Metric | Unit | Window | Sources | Reset Expression |
|--------|------|--------|---------|------------------|
| `total_tokens` | Tokens | Session | Local server session API, session export JSON, run JSON, SQLite | New session, clear/truncate/compact behavior, or DB update |
| `input_tokens` | Tokens | Session | Same | Same |
| `output_tokens` | Tokens | Session | Same | Same |
| `cache_read_input_tokens` | Tokens | Session | Local server session API, session export JSON, SQLite | Same |
| `cache_write_input_tokens` | Tokens | Session | Local server session API, session export JSON, SQLite | Same |
| `accumulated_total_tokens` | Tokens | Session | Local server session API, session export JSON, SQLite | Same |
| `accumulated_cost` | Currency | Session | Local server session API, session export JSON, SQLite | Same |
| Context usage | Percent and tokens | Session context window | Interactive prompt line, `goose term info` approximation | Reset by compaction, truncation, clearing, or new session |
| Provider subscription quota | Unknown | Unknown | Not found | Unknown |
| 5-hour window | Unknown | Unknown | Not found | Unknown |
| Weekly window | Unknown | Unknown | Not found | Unknown |
| Billing-cycle credits | Unknown | Unknown | Not found | Unknown |

Reset times are not expressed for local session metrics. Goose context usage is a ratio against the model context limit, not a time-window quota, so there is no timestamp or countdown.

## Limit States

| State | On-Demand Detectable | Markers | Notes |
|-------|----------------------|---------|-------|
| Cap approaching | No | None found | Goose has context-window percentages, but no provider-plan cap-approaching state for subscription windows. |
| Capped/rate limited | No for inspection; yes during runs | `ProviderError::RateLimitExceeded`, message prefix `Rate limit exceeded:`, telemetry type `rate_limit` | Runtime stream/error handling belongs to `non-interactive-sessions`. |
| Out of funds | No for inspection; yes during runs | `ProviderError::CreditsExhausted`, `SystemNotificationType::CreditsExhausted`, optional `top_up_url` | Runtime-only signal; no balance endpoint found. |
| Auth required | Yes for setup/API access | `goose info --check` prints `Auth: FAILED`; local server returns HTTP `401` without valid `X-Secret-Key`; provider errors use `ProviderError::Authentication` | Distinguishes authentication failures from quota limits. |

For preflight plan-awareness, Goose can tell Claudine whether a local server/session is readable and how full the current context window is. It cannot answer whether a 5-hour or weekly subscription budget has enough remaining runway.

## Sources

- [Goose documentation: Logging System](https://block.github.io/goose/docs/guides/logs)
- [Goose documentation: CLI Commands](https://block.github.io/goose/docs/guides/goose-cli-commands)
- [Goose documentation: Diagnostics and Reporting](https://block.github.io/goose/docs/troubleshooting/diagnostics-and-reporting)
- [Goose documentation: Anonymous Usage Data](https://block.github.io/goose/docs/guides/usage-data)
- [Goose documentation: Environment Variables](https://block.github.io/goose/docs/guides/environment-variables)
- [Goose source: CLI parser](https://github.com/aaif-goose/goose/blob/main/crates/goose-cli/src/cli.rs)
- [Goose source: CLI session loop and JSON output](https://github.com/aaif-goose/goose/blob/main/crates/goose-cli/src/session/mod.rs)
- [Goose source: slash-command parser](https://github.com/aaif-goose/goose/blob/main/crates/goose-cli/src/session/input.rs)
- [Goose source: terminal integration](https://github.com/aaif-goose/goose/blob/main/crates/goose-cli/src/commands/term.rs)
- [Goose source: session manager and SQLite schema](https://github.com/aaif-goose/goose/blob/main/crates/goose/src/session/session_manager.rs)
- [Goose source: config/data/state paths](https://github.com/aaif-goose/goose/blob/main/crates/goose/src/config/paths.rs)
- [Goose source: local server session routes](https://github.com/aaif-goose/goose/blob/main/crates/goose-server/src/routes/session.rs)
- [Goose source: local server auth middleware](https://github.com/aaif-goose/goose/blob/main/crates/goose-server/src/auth.rs)
- [Goose source: provider error types](https://github.com/aaif-goose/goose/blob/main/crates/goose-provider-types/src/errors.rs)
- Observed on host: no `goose` binary on `PATH`; no Goose config, data, state, or log directories under `/Users/ken/.config/goose`, `/Users/ken/.local/share/goose`, `/Users/ken/.local/state/goose`, or checked macOS Application Support/Logs variants on 2026-07-03.
