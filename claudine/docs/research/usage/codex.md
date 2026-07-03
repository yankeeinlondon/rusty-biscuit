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
    endpoint: "codex app-server JSON-RPC: account/rateLimits/read"
    auth: oauth
    users: "ChatGPT-authenticated Codex users; API-key auth is rejected by the local app-server request processor."
    fields: ["rateLimits.limitId", "rateLimits.primary.usedPercent", "rateLimits.primary.windowDurationMins", "rateLimits.primary.resetsAt", "rateLimits.secondary.usedPercent", "rateLimits.secondary.windowDurationMins", "rateLimits.secondary.resetsAt", "rateLimits.credits.hasCredits", "rateLimits.credits.unlimited", "rateLimits.credits.balance", "rateLimits.individualLimit", "rateLimits.planType", "rateLimits.rateLimitReachedType", "rateLimitsByLimitId", "rateLimitResetCredits.availableCount"]
    reset_window: "primary and secondary windows include windowDurationMins and resetsAt Unix seconds"
    notes: "Observed live on this host through `codex app-server --stdio`; returned codex primary 300-minute and secondary 10080-minute windows."
  - available: true
    endpoint: "codex app-server JSON-RPC: account/usage/read"
    auth: oauth
    users: "ChatGPT-authenticated Codex users; API-key auth is rejected by the local app-server request processor."
    fields: ["summary.lifetimeTokens", "summary.peakDailyTokens", "summary.longestRunningTurnSec", "summary.currentStreakDays", "summary.longestStreakDays", "dailyUsageBuckets[].startDate", "dailyUsageBuckets[].tokens"]
    reset_window: "none; daily buckets are dated but do not carry reset timestamps"
    notes: "Observed live on this host through `codex app-server --stdio`; returned token activity summary and dated daily token buckets."
  - available: true
    endpoint: "GET https://chatgpt.com/backend-api/wham/usage"
    auth: oauth
    users: "ChatGPT-authenticated Codex users via the Codex backend client; unauthenticated requests return 401."
    fields: ["plan_type", "rate_limit.primary_window.used_percent", "rate_limit.primary_window.limit_window_seconds", "rate_limit.primary_window.reset_after_seconds", "rate_limit.primary_window.reset_at", "rate_limit.secondary_window", "credits", "spend_control", "additional_rate_limits", "rate_limit_reached_type", "rate_limit_reset_credits.available_count"]
    reset_window: "backend payload carries limit_window_seconds, reset_after_seconds, and reset_at"
    notes: "The Codex app-server maps this backend response into RateLimitSnapshot records."
  - available: true
    endpoint: "GET https://chatgpt.com/backend-api/wham/profiles/me"
    auth: oauth
    users: "ChatGPT-authenticated Codex users via the Codex backend client."
    fields: ["summary.lifetimeTokens", "summary.peakDailyTokens", "summary.longestRunningTurnSec", "summary.currentStreakDays", "summary.longestStreakDays", "dailyUsageBuckets[].startDate", "dailyUsageBuckets[].tokens"]
    reset_window: "none"
    notes: "The Codex app-server maps this backend response into account/usage/read."
  - available: false
    endpoint: "GET https://chatgpt.com/backend-api/wham/usage with OpenAI API-key auth"
    auth: api_key
    users: "unknown; probe with a syntactically API-key-shaped bearer token returned 401 invalid_api_key, and source rejects non-ChatGPT auth before this app-server method runs."
    fields: []
    reset_window: "none"
    notes: "Negative probe on 2026-07-03: unauthenticated request returned 401 Unauthorized; invalid API-key bearer returned 401 invalid_api_key."
cli_methods:
  - available: true
    invocation: "printf JSON-RPC lines into `codex app-server --stdio`: initialize, initialized, account/rateLimits/read, account/usage/read"
    interactive_only: false
    output_format: "JSONL JSON-RPC responses"
    fields: ["rateLimits", "rateLimitsByLimitId", "rateLimitResetCredits", "summary", "dailyUsageBuckets"]
    notes: "Best current non-interactive mechanism. It is a local app-server protocol, not a one-shot `codex usage --json` command."
  - available: false
    invocation: "codex login status"
    interactive_only: false
    output_format: "human text"
    fields: ["auth mode only"]
    notes: "Installed CLI help exposes login status but no usage or limit fields."
  - available: false
    invocation: "codex doctor --json"
    interactive_only: false
    output_format: "JSON"
    fields: ["auth storage", "config", "network reachability", "state paths"]
    notes: "Structured and useful for diagnostics, but observed output had no usage, quota, or reset fields."
  - available: true
    invocation: "/status"
    interactive_only: true
    output_format: "styled TUI text"
    fields: ["current session configuration", "token usage", "remaining limits"]
    notes: "Official pricing docs say `/status` shows remaining limits during an active CLI session; source describes it as current session configuration and token usage."
  - available: true
    invocation: "/usage"
    interactive_only: true
    output_format: "styled TUI text"
    fields: ["account usage", "usage limit reset controls"]
    notes: "Source description says it views account usage or uses a usage-limit reset; gated by token_activity_command_enabled."
  - available: true
    invocation: "/statusline"
    interactive_only: true
    output_format: "styled TUI setup"
    fields: ["five-hour-limit", "weekly-limit", "context used", "context remaining", "used tokens"]
    notes: "Source status surface preview includes FiveHourLimit and WeeklyLimit display items; useful for visible status, not ideal for machine parsing."
pty_design:
  command: "codex --no-alt-screen"
  first_pass_markers: ["/usage", "Usage", "5h", "weekly", "resets", "%", "credits"]
  fuzzy_markers: ["primary", "secondary", "five hour", "5-hour", "weekly", "window", "reset", "credits", "limit", "usage"]
  fields: ["primary.used_percent", "primary.resets_at_or_countdown", "secondary.used_percent", "secondary.resets_at_or_countdown", "credits.balance", "reset_credits.available_count", "plan_type"]
  risks: "TUI text has no schema or stability contract; use only when app-server JSON-RPC is unavailable, and prefer fuzzy extraction only after exact markers fail."
metrics:
  - name: "primary window used"
    unit: percent
    window: five_hour
    source: "account/rateLimits/read rateLimits.primary.usedPercent"
    notes: "Observed primary windowDurationMins 300."
  - name: "primary reset time"
    unit: time
    window: five_hour
    source: "account/rateLimits/read rateLimits.primary.resetsAt"
    notes: "Unix timestamp in seconds."
  - name: "secondary window used"
    unit: percent
    window: weekly
    source: "account/rateLimits/read rateLimits.secondary.usedPercent"
    notes: "Observed secondary windowDurationMins 10080."
  - name: "secondary reset time"
    unit: time
    window: weekly
    source: "account/rateLimits/read rateLimits.secondary.resetsAt"
    notes: "Unix timestamp in seconds."
  - name: "credit balance"
    unit: credits
    window: unknown
    source: "account/rateLimits/read rateLimits.credits.balance"
    notes: "String balance plus hasCredits and unlimited booleans."
  - name: "earned reset credits"
    unit: other
    window: unknown
    source: "account/rateLimits/read rateLimitResetCredits.availableCount"
    notes: "Snapshot-only; refetch after consuming a reset."
  - name: "lifetime tokens"
    unit: tokens
    window: other
    source: "account/usage/read summary.lifetimeTokens"
    notes: "Account token-activity total, not remaining quota."
  - name: "daily token buckets"
    unit: tokens
    window: daily
    source: "account/usage/read dailyUsageBuckets"
    notes: "Dated usage history; no reset time."
  - name: "local thread tokens used"
    unit: tokens
    window: session
    source: "~/.codex/state_5.sqlite threads.tokens_used"
    notes: "Local persisted aggregate per thread; stale for quota decisions and lacks reset information."
limit_states:
  - state: cap_approaching
    detectable: true
    source: "account/rateLimits/read usedPercent"
    markers: ["primary.usedPercent near 100", "secondary.usedPercent near 100"]
    notes: "No explicit approaching state was observed; tools must choose thresholds."
  - state: capped
    detectable: true
    source: "account/rateLimits/read rateLimitReachedType"
    markers: ["rate_limit_reached", "workspace_owner_usage_limit_reached", "workspace_member_usage_limit_reached"]
    notes: "Also likely paired with usedPercent at or near 100."
  - state: no_funds
    detectable: true
    source: "account/rateLimits/read rateLimitReachedType and credits"
    markers: ["workspace_owner_credits_depleted", "workspace_member_credits_depleted", "credits.hasCredits=false", "credits.balance=0"]
    notes: "Credit depletion is separate from included-window exhaustion."
  - state: auth_required
    detectable: true
    source: "app-server account request errors and HTTP probes"
    markers: ["codex account authentication required to read rate limits", "chatgpt authentication required to read rate limits", "HTTP 401 Unauthorized", "invalid_api_key"]
    notes: "App-server source rejects missing auth and non-Codex-backend auth before fetching limits."
  - state: unknown
    detectable: false
    source: "local SQLite and logs"
    markers: []
    notes: "Local artifacts do not persist authoritative quota state beyond per-thread token counters."
docs: https://developers.openai.com/codex/pricing
changes: []
requires_claudine_update: true
reason: "Claudine can avoid TUI scraping by adding a Codex usage inspector that speaks the local app-server JSON-RPC protocol and parses account/rateLimits/read plus account/usage/read."
---

# Codex CLI Usage Inspection

## Introduction to Codex CLI Usage Inspection

Codex CLI uses two different usage models depending on authentication mode. When signed in with ChatGPT, Codex usage counts against the user's ChatGPT plan and shared agentic usage allowance; OpenAI documents a five-hour window for local messages and cloud tasks, notes that additional weekly limits may apply, and says credits can extend work after included limits are reached. When signed in with an API key, Codex uses API token pricing instead of ChatGPT subscription windows. OpenAI officially points users to the Codex usage dashboard for current limits and says `/status` can show remaining limits during an active Codex CLI session. The installed CLI and upstream source additionally expose a local app-server JSON-RPC account surface that can query the same state in structured form.

Sources: [Codex pricing](https://developers.openai.com/codex/pricing), [Using Codex with your ChatGPT plan](https://help.openai.com/en/articles/11369540-using-codex-with-your-chatgpt-plan), [Codex app-server README](https://github.com/openai/codex/blob/da4c8ca57d40b074bdc1b5b1218851100150c56b/codex-rs/app-server/README.md).

## API Call Opportunities

### Local App-Server JSON-RPC

The best on-demand inspection path is the local app-server protocol:

```json
{"method":"initialize","id":1,"params":{"clientInfo":{"name":"claudine","title":"Claudine","version":"0.0.0"}}}
{"method":"initialized"}
{"method":"account/rateLimits/read","id":2}
{"method":"account/usage/read","id":3}
```

Run it over:

```bash
codex app-server --stdio
```

The protocol is newline-delimited JSON over stdio and requires an `initialize` request followed by an `initialized` notification before account requests. The app-server README lists `account/rateLimits/read` as the method for ChatGPT rate limits, optional monthly credit limit, and earned reset count, and lists `account/usage/read` as token-activity summary plus daily buckets. The generated protocol schema includes `GetAccountRateLimitsResponse`, `RateLimitSnapshot`, `RateLimitWindow`, `CreditsSnapshot`, `RateLimitResetCreditsSummary`, and `GetAccountTokenUsageResponse`.

Observed on this host on 2026-07-03 with `codex-cli 0.142.5`, `account/rateLimits/read` returned:

```json
{
  "rateLimits": {
    "limitId": "codex",
    "primary": { "usedPercent": 4, "windowDurationMins": 300, "resetsAt": 1783110919 },
    "secondary": { "usedPercent": 41, "windowDurationMins": 10080, "resetsAt": 1783394406 },
    "credits": { "hasCredits": false, "unlimited": false, "balance": "0" },
    "individualLimit": null,
    "planType": "prolite",
    "rateLimitReachedType": null
  },
  "rateLimitResetCredits": { "availableCount": 4 }
}
```

The same response included `rateLimitsByLimitId.codex_bengalfox` for `GPT-5.3-Codex-Spark`, with independent primary and secondary windows. `account/usage/read` returned `summary.lifetimeTokens`, `summary.peakDailyTokens`, `summary.longestRunningTurnSec`, streak fields, and `dailyUsageBuckets[]` with `startDate` and `tokens`.

Auth is ChatGPT/Codex-backend auth, not API-key auth. The app-server request processor returns `"codex account authentication required to read rate limits"` when no Codex auth exists and `"chatgpt authentication required to read rate limits"` when the configured auth is not the Codex backend. The same guard exists for token usage. Source: [account processor](https://github.com/openai/codex/blob/da4c8ca57d40b074bdc1b5b1218851100150c56b/codex-rs/app-server/src/request_processors/account_processor.rs), [protocol account types](https://github.com/openai/codex/blob/da4c8ca57d40b074bdc1b5b1218851100150c56b/codex-rs/app-server-protocol/src/protocol/v2/account.rs), [generated TypeScript schema](https://github.com/openai/codex/tree/da4c8ca57d40b074bdc1b5b1218851100150c56b/codex-rs/app-server-protocol/schema/typescript/v2).

### ChatGPT Backend HTTP Routes

The app-server uses the `codex_backend_client` crate. With the ChatGPT backend base URL, it maps rate-limit inspection to:

```http
GET https://chatgpt.com/backend-api/wham/usage
```

and token activity to:

```http
GET https://chatgpt.com/backend-api/wham/profiles/me
```

The rate-limit payload model contains `plan_type`, `rate_limit`, `credits`, `spend_control`, `additional_rate_limits`, and `rate_limit_reached_type`. The window model carries `used_percent`, `limit_window_seconds`, `reset_after_seconds`, and `reset_at`. The app-server normalizes those into `usedPercent`, `windowDurationMins`, and `resetsAt`. Source: [backend client](https://github.com/openai/codex/blob/da4c8ca57d40b074bdc1b5b1218851100150c56b/codex-rs/backend-client/src/client.rs), [rate-limit reset client](https://github.com/openai/codex/blob/da4c8ca57d40b074bdc1b5b1218851100150c56b/codex-rs/backend-client/src/client/rate_limit_resets.rs), [backend OpenAPI models](https://github.com/openai/codex/tree/da4c8ca57d40b074bdc1b5b1218851100150c56b/codex-rs/codex-backend-openapi-models/src/models).

Negative probes on 2026-07-03:

| Probe | Result |
| --- | --- |
| `curl -i https://chatgpt.com/backend-api/wham/usage` | HTTP 401, body `{"detail":"Unauthorized"}` |
| `curl -i -H 'Authorization: Bearer sk-test' https://chatgpt.com/backend-api/wham/usage` | HTTP 401 with `x-openai-ide-error-code: invalid_api_key` |

This supports the source finding: the useful route is ChatGPT session/OAuth-backed, not a general OpenAI API-key endpoint. Public docs do mention an Enterprise Analytics API and Compliance API for Codex logs, but those are enterprise/admin analytics surfaces, not the per-user live quota lookup Claudine needs. Source: [Help Center plan article](https://help.openai.com/en/articles/11369540-using-codex-with-your-chatgpt-plan).

## CLI Switch Opportunities

There is no installed one-shot `codex usage`, `codex quota`, or `codex limits` subcommand in `codex-cli 0.142.5`. Observed `codex --help` listed `exec`, `review`, `login`, `logout`, `mcp`, `plugin`, `app-server`, `doctor`, `debug`, and other commands, but no direct usage command. `codex login status` exists but reports login state rather than quota. `codex doctor --json` is structured and useful for config/auth/path diagnostics, but the observed JSON had no usage, quota, or reset-window fields.

The practical non-interactive CLI path is therefore:

```bash
codex app-server --stdio
```

with JSON-RPC requests for `account/rateLimits/read` and `account/usage/read`. This yields machine-parseable JSONL and worked without a TTY in the local probe. It is more stable than scraping a TUI, but it is still an app-server protocol surface rather than a purpose-built `codex usage --json` command. Source: [Codex app-server README](https://github.com/openai/codex/blob/da4c8ca57d40b074bdc1b5b1218851100150c56b/codex-rs/app-server/README.md), [installed CLI help observed on host, 2026-07-03].

`codex exec --json` is not an on-demand usage command. It streams events for a run and can include turn token usage while executing, which belongs to the sibling non-interactive-sessions topic rather than pre-run inspection. Source: [exec JSONL event processor](https://github.com/openai/codex/blob/da4c8ca57d40b074bdc1b5b1218851100150c56b/codex-rs/exec/src/event_processor_with_jsonl_output.rs).

## Interactive Commands and PTY Scraping

Codex has both `/status` and `/usage` slash commands. Source declares `/status` as "show current session configuration and token usage" and `/usage` as "view account usage or use a usage limit reset". The pricing docs additionally say `/status` can show remaining limits during an active CLI session. `/usage` supports inline args and is gated by `token_activity_command_enabled`; `/statusline` can configure visible status-line items, and source preview labels include `FiveHourLimit` and `WeeklyLimit`. Source: [slash command enum](https://github.com/openai/codex/blob/da4c8ca57d40b074bdc1b5b1218851100150c56b/codex-rs/tui/src/slash_command.rs), [slash command filtering](https://github.com/openai/codex/blob/da4c8ca57d40b074bdc1b5b1218851100150c56b/codex-rs/tui/src/bottom_pane/slash_commands.rs), [status surface preview](https://github.com/openai/codex/blob/da4c8ca57d40b074bdc1b5b1218851100150c56b/codex-rs/tui/src/bottom_pane/status_surface_preview.rs), [Codex pricing](https://developers.openai.com/codex/pricing).

These commands are interactive TUI commands, not normal shell subcommands. There is no evidence that the installed CLI accepts `/status` or `/usage` as a preliminary command argument at launch. Because the app-server JSON-RPC path is available, PTY scraping should be a last resort.

Mini-design with `expectrl`:

1. Start `codex --no-alt-screen` in a PTY so scrollback remains visible, wait for the composer prompt, send `/usage\r`, and capture the visible pane. The first pass should match exact current markers: `Usage`, `5h` or `5-hour`, `weekly`, `resets`, `%`, `credits`, plus labels for primary/secondary if present. Extract percentages, reset timestamps or countdowns, credit balance, reset-credit count, and plan label.
2. If exact markers fail, send `/status\r` and run a fuzzy pass over the screen buffer using case-insensitive markers: `primary`, `secondary`, `five hour`, `weekly`, `window`, `reset`, `credits`, `limit`, `usage`. Prefer numbers close to the matching label on the same line; if the UI shifts to a table, prefer column headers and row labels over fixed positions.

Caveat: scraped TUI text has no schema and no stability contract. It is not versioned, not validated, and can drift with copy, layout, terminal width, feature flags, or styling. Treat it as a degraded fallback only after app-server JSON-RPC fails.

## Config and Log Artifacts

The prompt required inspection of `~/.codex`. On this host, `~/.codex` exists and contains config, auth, history, sessions, SQLite state, model cache, and logs. The active `CODEX_HOME` reported by `codex doctor --json` and `codex app-server --stdio` was `/Users/ken/.claudine/.codex`, which symlinks most user-facing files back to `~/.codex` while keeping its own SQLite files. Both homes were inspected.

Relevant observed artifacts:

| Path | Evidence | Freshness | Usage value |
| --- | --- | --- | --- |
| `~/.codex/auth.json` | keys include `auth_mode`, `OPENAI_API_KEY`, `tokens`, `last_refresh`; values were not copied | updated when auth refreshes | proves ChatGPT auth exists, but no usage or limit fields |
| `~/.codex/config.toml` | model, reasoning effort, feature flags, trusted projects | user-edited config | no usage or limit fields |
| `~/.codex/state_5.sqlite` | `threads.tokens_used` plus timestamps; observed 1,860 rows and aggregate token counters | updated as local threads are persisted | local per-thread token accounting only; no plan windows, headroom, or resets |
| `~/.codex/logs_2.sqlite` | `logs` table with `feedback_log_body`; keyword search for usage/quota/limit/rate returned 0 rows | live logging DB | no usage/limit state observed |
| `~/.codex/goals_1.sqlite` | `thread_goals.tokens_used`, `token_budget`, `status` | goal bookkeeping | local Codex goal budgets, not provider quota |
| `~/.codex/memories_1.sqlite` | `stage1_outputs.usage_count`, `last_usage` | memory pipeline bookkeeping | memory use metadata, not quota |
| `~/.codex/models_cache.json` | model catalog fields, including service tiers and capabilities | refreshed from server | no usage windows; can explain model/fast-mode consumption differences |
| `~/.codex/history.jsonl` | prompt history | append-only history | no authoritative quota fields |

No local artifact under `~/.codex` or `/Users/ken/.claudine/.codex` persisted the current five-hour or weekly quota snapshot observed from `account/rateLimits/read`. The closest local field is `threads.tokens_used`, which is useful for local history but stale and incomplete for headroom decisions. Observed-on-host references: `sqlite3 ~/.codex/state_5.sqlite '.schema threads'`, `sqlite3 ~/.codex/logs_2.sqlite '.schema logs'`, `codex doctor --json`, and redacted key inspection of `~/.codex/auth.json` on 2026-07-03.

## Metrics and Windows

| Mechanism | Metric | Unit | Window | Reset expression |
| --- | --- | --- | --- | --- |
| `account/rateLimits/read` | `primary.usedPercent` | percent | five-hour in observed data | `primary.resetsAt` Unix seconds |
| `account/rateLimits/read` | `primary.windowDurationMins` | time | five-hour in observed data | duration was `300` minutes |
| `account/rateLimits/read` | `secondary.usedPercent` | percent | weekly in observed data | `secondary.resetsAt` Unix seconds |
| `account/rateLimits/read` | `secondary.windowDurationMins` | time | weekly in observed data | duration was `10080` minutes |
| `account/rateLimits/read` | `credits.balance`, `hasCredits`, `unlimited` | credits | unknown | no reset |
| `account/rateLimits/read` | `rateLimitResetCredits.availableCount` | count | unknown | snapshot only |
| `account/usage/read` | `summary.lifetimeTokens` | tokens | lifetime | no reset |
| `account/usage/read` | `dailyUsageBuckets[].tokens` | tokens | daily bucket history | date only |
| `~/.codex/state_5.sqlite` | `threads.tokens_used` | tokens | session/thread | no reset |

OpenAI's pricing page describes credit usage as token-based: input tokens, cached input tokens, and output tokens are converted to credits using model-specific rates. That pricing explanation is distinct from the app-server `usedPercent` windows, which are already normalized percentages. Source: [Codex pricing](https://developers.openai.com/codex/pricing), [RateLimitSnapshot protocol type](https://github.com/openai/codex/blob/da4c8ca57d40b074bdc1b5b1218851100150c56b/codex-rs/protocol/src/protocol.rs).

## Limit States

| State | Best marker | Mechanism | Notes |
| --- | --- | --- | --- |
| Cap approaching | `primary.usedPercent` or `secondary.usedPercent` near a Claudine threshold | `account/rateLimits/read` | No explicit approaching state was observed in the response; choose a local threshold such as 80-90%. |
| Capped | `rateLimitReachedType=rate_limit_reached`, `workspace_owner_usage_limit_reached`, or `workspace_member_usage_limit_reached` | `account/rateLimits/read` | Source enum defines these backend states. |
| Out of funds | `workspace_owner_credits_depleted`, `workspace_member_credits_depleted`, or credits with `hasCredits=false` and `balance=0` | `account/rateLimits/read` | Credit depletion is separate from subscription-window exhaustion. |
| Auth required | app-server JSON-RPC error text or HTTP 401 | app-server and backend HTTP | Missing auth and API-key/non-ChatGPT auth are distinguishable in app-server source. |
| Unknown/stale local state | only local token counters exist | SQLite artifacts | Local DBs should not drive quota decisions without an app-server refresh. |

The enum values come from the protocol and generated backend models: `rate_limit_reached`, `workspace_owner_credits_depleted`, `workspace_member_credits_depleted`, `workspace_owner_usage_limit_reached`, and `workspace_member_usage_limit_reached`. Source: [protocol type](https://github.com/openai/codex/blob/da4c8ca57d40b074bdc1b5b1218851100150c56b/codex-rs/protocol/src/protocol.rs), [backend OpenAPI rate-limit payload](https://github.com/openai/codex/blob/da4c8ca57d40b074bdc1b5b1218851100150c56b/codex-rs/codex-backend-openapi-models/src/models/rate_limit_status_payload.rs).

## Sources

- [Codex pricing](https://developers.openai.com/codex/pricing)
- [Codex CLI](https://developers.openai.com/codex/cli)
- [Using Codex with your ChatGPT plan](https://help.openai.com/en/articles/11369540-using-codex-with-your-chatgpt-plan)
- [Codex app-server README](https://github.com/openai/codex/blob/da4c8ca57d40b074bdc1b5b1218851100150c56b/codex-rs/app-server/README.md)
- [Codex backend client](https://github.com/openai/codex/blob/da4c8ca57d40b074bdc1b5b1218851100150c56b/codex-rs/backend-client/src/client.rs)
- [Codex rate-limit reset client](https://github.com/openai/codex/blob/da4c8ca57d40b074bdc1b5b1218851100150c56b/codex-rs/backend-client/src/client/rate_limit_resets.rs)
- [Codex account processor](https://github.com/openai/codex/blob/da4c8ca57d40b074bdc1b5b1218851100150c56b/codex-rs/app-server/src/request_processors/account_processor.rs)
- [Codex protocol rate-limit types](https://github.com/openai/codex/blob/da4c8ca57d40b074bdc1b5b1218851100150c56b/codex-rs/protocol/src/protocol.rs)
- [Codex app-server account protocol](https://github.com/openai/codex/blob/da4c8ca57d40b074bdc1b5b1218851100150c56b/codex-rs/app-server-protocol/src/protocol/v2/account.rs)
- [Codex slash commands](https://github.com/openai/codex/blob/da4c8ca57d40b074bdc1b5b1218851100150c56b/codex-rs/tui/src/slash_command.rs)
- [Observed on host: `codex --help`, `codex app-server --stdio`, `codex doctor --json`, `~/.codex` SQLite schemas and redacted auth/config inspection, 2026-07-03]
