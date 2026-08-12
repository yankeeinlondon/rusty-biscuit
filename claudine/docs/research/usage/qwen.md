---
$schema: ./_schema.yaml
created: 2026-07-03
last_updated: 2026-07-03
agent: codex
model: default
api: false
cli_switch: true
structured_output: true
pty_scrape: true
api_methods:
  - available: false
    endpoint: ""
    auth: unknown
    users: "unknown"
    fields: []
    reset_window: "No documented Qwen CLI or Coding Plan HTTP endpoint reports current usage, remaining quota, or reset times on demand."
    notes: "Alibaba documents viewing Coding Plan usage on the console page, not through an API."
cli_methods:
  - available: true
    invocation: "qwen -p '/stats daily'"
    interactive_only: false
    output_format: "human text"
    fields: ["daily token totals", "model", "auth type", "source", "api duration"]
    notes: "Official current docs list /stats daily and /stats day [YYYY-MM-DD]; installed 0.15.6 on this host does not yet include this subcommand."
  - available: true
    invocation: "qwen -p '/stats monthly'"
    interactive_only: false
    output_format: "human text"
    fields: ["monthly token totals", "model", "auth type", "source", "api duration"]
    notes: "Official current docs list /stats monthly and /stats month [YYYY-MM]; installed 0.15.6 on this host does not yet include this subcommand."
  - available: true
    invocation: "qwen -p '/stats export <daily|monthly> [date|month] --format json --output <path>'"
    interactive_only: false
    output_format: "JSON"
    fields: ["aggregate token summaries", "model grouping", "auth type grouping", "source grouping"]
    notes: "Best structured path in current docs. Exports aggregate summaries, not raw transcripts."
  - available: true
    invocation: "/stats or /usage"
    interactive_only: true
    output_format: "styled TUI text"
    fields: ["session duration", "prompts", "API requests", "prompt tokens", "output tokens", "tool calls", "files changed"]
    notes: "Available as an interactive dashboard; installed 0.15.6 also supports /stats in non-interactive command mode after auth is configured."
  - available: true
    invocation: "/stats model"
    interactive_only: false
    output_format: "human text"
    fields: ["model", "prompt tokens", "output tokens", "cached tokens"]
    notes: "Installed 0.15.6 source shows non-interactive text lines of the form model: prompt=..., output=..., cached=...."
  - available: true
    invocation: "/status"
    interactive_only: true
    output_format: "styled TUI text"
    fields: ["version", "auth method", "model", "path", "memory usage"]
    notes: "Status is orientation/auth context, not consumption or quota headroom."
  - available: false
    invocation: "qwen auth status"
    interactive_only: false
    output_format: "human text"
    fields: ["configured authentication method"]
    notes: "Observed in installed 0.15.6, but official current docs say the standalone qwen auth command has been removed; it reports auth status only, not usage or quota."
pty_design:
  command: "spawn qwen in a PTY, send /stats, then /stats model, then /status, and exit"
  first_pass_markers: ["Session Stats", "Model Usage", "API Requests", "Tokens", "Prompt", "Output", "Auth Method"]
  fuzzy_markers: ["stats", "usage", "requests", "tokens", "model", "auth", "quota", "remaining", "reset"]
  fields: ["session_duration", "api_requests", "prompt_tokens", "output_tokens", "cached_tokens", "model", "auth_method", "context_window_used_percent"]
  risks: "TUI text has no schema or stability contract; use only when /stats export JSON and local JSONL are unavailable, and treat missing or renamed labels as drift rather than failure of the provider."
metrics:
  - name: "Coding Plan 5-hour request quota"
    unit: requests
    window: five_hour
    source: "Alibaba Cloud Coding Plan FAQ"
    notes: "The FAQ documents a 5-hour request quota and says exhaustion resets automatically after 5 hours; no on-demand API response fields were found."
  - name: "Coding Plan weekly request quota"
    unit: requests
    window: weekly
    source: "Alibaba Cloud Coding Plan FAQ"
    notes: "The FAQ says the weekly request quota resets at 00:00:00 UTC+8 on Monday."
  - name: "Coding Plan monthly request quota"
    unit: requests
    window: monthly
    source: "Alibaba Cloud Coding Plan FAQ"
    notes: "The FAQ says the monthly request quota resets at 00:00:00 UTC+8 on the corresponding day of the next subscription month."
  - name: "Session API requests"
    unit: requests
    window: session
    source: "Qwen CLI /stats"
    notes: "Installed 0.15.6 source sums per-model totalRequests for the current session."
  - name: "Session prompt tokens"
    unit: tokens
    window: session
    source: "Qwen CLI /stats"
    notes: "Installed 0.15.6 source reports prompt tokens accumulated in the current session."
  - name: "Session output tokens"
    unit: tokens
    window: session
    source: "Qwen CLI /stats"
    notes: "Installed 0.15.6 source reports candidate/output tokens accumulated in the current session."
  - name: "Daily token usage"
    unit: tokens
    window: daily
    source: "Qwen CLI /stats daily and local usage JSONL"
    notes: "Current design persists content-free records under usage/token-usage-YYYY-MM.jsonl and aggregates them with /stats daily."
  - name: "Monthly token usage"
    unit: tokens
    window: monthly
    source: "Qwen CLI /stats monthly and local usage JSONL"
    notes: "Current design aggregates the same local JSONL records by month."
  - name: "Context window usage"
    unit: percent
    window: session
    source: "Qwen status line input"
    notes: "Installed source computes used and remaining percentage from last prompt tokens versus context window; this is context capacity, not subscription quota."
limit_states:
  - state: cap_approaching
    detectable: false
    source: "Qwen CLI docs and installed source"
    markers: []
    notes: "No pre-run or on-demand cap-approaching marker was found."
  - state: capped
    detectable: true
    source: "Alibaba Cloud Coding Plan FAQ and installed Qwen source"
    markers: ["hour allocated quota exceeded", "week allocated quota exceeded", "month allocated quota exceeded", "usage allocated quota exceeded", "Rate limit exceeded. Try again later."]
    notes: "These are runtime failure strings rather than on-demand inspection fields; sibling non-interactive-sessions should own live detection."
  - state: no_funds
    detectable: true
    source: "Installed Qwen source"
    markers: ["HTTP 402", "HTTP 403", "billing", "quota"]
    notes: "Installed source maps 402/403 and messages containing billing/quota to payment/quota failure handling; no pre-run balance endpoint was found."
  - state: auth_required
    detectable: true
    source: "Observed on host with qwen -p /stats and qwen auth status"
    markers: ["No auth type is selected", "No authentication method configured", "Use Qwen Code CLI to authenticate first."]
    notes: "Non-interactive slash-command attempts fail before stats when no auth type is configured."
docs: https://qwenlm.github.io/qwen-code-docs/en/users/features/commands/
changes: []
requires_claudine_update: true
reason: "Qwen now documents local daily/monthly usage inspection and JSON export through /stats; Claudine should add a version-gated Qwen usage provider that prefers /stats export JSON or local usage JSONL and falls back to session /stats scraping only when necessary."
---

# Qwen CLI Usage Inspection

## Introduction to Qwen CLI Usage Inspection

Qwen Code supports multiple billing and quota modes. The discontinued Qwen OAuth free tier no longer provides a reliable quota path; official authentication docs say the free tier ended on 2026-04-15 and direct users to Coding Plan, OpenRouter, Fireworks, or custom API-key providers. Alibaba Cloud Coding Plan is a fixed monthly subscription with request quotas, including 5-hour, weekly, and monthly windows. General API-key and third-party providers may expose their own billing or quota model outside Qwen Code.

Official usage visibility is split. Alibaba says Coding Plan consumption is visible on the Coding Plan console page and that token consumption/model-specific usage are not supported for the plan page because Coding Plan quota is based on model-call counts. Qwen Code itself now documents `/stats daily`, `/stats monthly`, and `/stats export` for local token-usage inspection; this is useful for local consumption history but is not the authoritative remaining Coding Plan quota or reset state.

## API Call Opportunities

No documented HTTP endpoint reports current Qwen Code or Coding Plan quota consumption, remaining headroom, or reset timestamps on demand.

| Opportunity | Result | Auth | Users | Evidence |
|---|---|---|---|---|
| Coding Plan console usage | Web console only; no public endpoint documented | Alibaba Cloud console session | Coding Plan subscribers | Alibaba says usage information is viewable on the Coding Plan page and model-specific/token consumption is not supported there. |
| Coding Plan OpenAI-compatible model endpoint, such as `POST https://coding-intl.dashscope.aliyuncs.com/v1/chat/completions` | Not a quota-inspection endpoint; chat responses may include per-request usage only | `Authorization: Bearer sk-sp-...` | Coding Plan subscribers using supported coding tools | Alibaba documents dedicated plan API keys and base URLs for calls, but quota exhaustion is reported as errors and plan usage is viewed in the console. |
| Qwen OAuth usage endpoint | No current path found | OAuth/session token | Unknown; OAuth free tier discontinued | Qwen docs state OAuth free tier was discontinued on 2026-04-15 and issue #3302 requested a `/quota` command because users had no way to check remaining OAuth quota. |

Example negative probe:

```sh
qwen -p /stats
```

Observed on this host with Qwen Code 0.15.6 and no configured Qwen auth:

```text
No auth type is selected. Please configure an auth type (e.g. via settings or `--auth-type`) before running in non-interactive mode.
```

This proves non-interactive slash-command inspection requires configured auth in the installed binary. It does not establish a provider quota endpoint.

## CLI Switch Opportunities

Current official docs list these slash-command inspection paths:

| Invocation | Works Non-Interactively | Structured | Fields |
|---|---:|---:|---|
| `qwen -p '/stats daily'` | Yes, when auth is configured | No | Daily token usage statistics |
| `qwen -p '/stats day YYYY-MM-DD'` | Yes, when auth is configured | No | Daily token usage for a selected local date |
| `qwen -p '/stats monthly'` | Yes, when auth is configured | No | Monthly token usage statistics |
| `qwen -p '/stats month YYYY-MM'` | Yes, when auth is configured | No | Monthly token usage for a selected month |
| `qwen -p '/stats export daily 2026-07-03 --format json --output usage.json'` | Yes, when auth is configured | Yes | Aggregate token summaries grouped by total, model, auth type, model/auth type, and source |
| `qwen -p '/stats model'` | Yes, when auth is configured | No | Per-model token breakdown and estimated cost in current docs; installed 0.15.6 reports `prompt`, `output`, and `cached` counts |
| `qwen -p '/stats tools'` | Yes, when auth is configured | No | Tool call totals |

The installed Homebrew package on this host is Qwen Code 0.15.6 at `/opt/homebrew/Cellar/qwen-code/0.15.6`. Its compiled source defines `/stats` with alias `/usage`, `/stats model`, and `/stats tools`, including non-interactive text output. It does not contain the current `/stats daily`, `/stats monthly`, or `/stats export` implementation, so Claudine should version-gate the newer strategy.

Observed `qwen auth status` on this host prints authentication state, not usage:

```text
=== Authentication Status ===
No authentication method configured.
```

Current official docs say the standalone `qwen auth` command has been removed and legacy invocations print migration guidance, so `qwen auth status` is not a stable current usage-inspection path.

## Interactive Commands and PTY Scraping

Qwen Code ships interactive slash commands that expose local usage-like metrics:

| Command | Display |
|---|---|
| `/stats` or `/usage` | Interactive usage statistics dashboard with Session, Activity, and Efficiency tabs in current docs. Installed 0.15.6 source renders a `Session Stats` panel with session id, tool calls, success rate, wall time, agent active time, API time, tool time, and model usage when model data exists. |
| `/stats model` | Model stats panel. Installed 0.15.6 source renders `Model Stats For Nerds` with requests, errors, average latency, total tokens, prompt tokens, cached tokens, thought tokens when present, and output tokens. |
| `/stats tools` | Tool stats panel with call counts, success rate, average duration, and user agreement. |
| `/status` or `/about` | Version and environment status. Current docs describe `/status` as version information and `/status paths` as current session file/log paths; installed source and localization strings include labels such as Auth Method and Memory Usage. |

Mini-design for `expectrl` scraping:

1. First pass: spawn `qwen` in a PTY, wait for the prompt, send `/stats`, `/stats model`, and `/status`, then capture the screen. Parse exact labels known today: `Session Stats`, `Model Usage`, `Model Stats For Nerds`, `API Requests`, `Tokens`, `Prompt`, `Output`, `Cached`, `Auth Method`, and `Memory Usage`. Convert numbers with thousands separators and preserve model labels exactly.
2. Second pass: when exact markers fail, run a fuzzy search over the captured screen for lowercase tokens near each other: `stats`, `usage`, `requests`, `tokens`, `model`, `auth`, `quota`, `remaining`, `reset`, `prompt`, `output`, and `cached`. Extract nearby numeric values only when the label/value relationship is unambiguous; otherwise return a drift diagnostic with the raw captured text redacted for obvious secrets.

PTY scraping is strictly a last resort. TUI text has no schema, no semantic version, and no validation contract. Current `/stats export --format json` and local JSONL records are better because they are explicitly designed for aggregation and machine consumption.

## Config and Log Artifacts

Observed host state on 2026-07-03:

| Path | Result | Freshness | Usage/Quota Fields |
|---|---|---|---|
| `~/.qwen` | Does not exist on this host | None | None |
| `<repo>/.qwen` | Exists, but contains only symlinked `agents`, `commands`, and `skills` | Repo resource links, not runtime state | None |
| `$TMPDIR/.qwen`, `/tmp/.qwen` | No files found | None | None |
| `~/Library/Application Support`, `~/Library/Logs`, `~/Library/Caches` Qwen/DashScope/ModelStudio search | No matching Qwen usage artifacts found | None | None |
| `~/.claudine/cache/models/qwen_code.json` | Claudine model cache only | Written by Claudine model discovery | Provider id, model ids, fetched timestamp; no usage |

Installed Qwen 0.15.6 source defines these storage locations:

| Path | Meaning | Usage Relevance |
|---|---|---|
| `~/.qwen/settings.json` | User settings | May contain auth/provider configuration; absent on this host |
| `<project>/.qwen/settings.json` | Project settings | No such file in this worktree |
| `~/.qwen/oauth_creds.json` | OAuth credentials | Absent on this host; OAuth free tier is discontinued |
| runtime dir from `QWEN_RUNTIME_DIR` or `~/.qwen` | Runtime output base | Current design stores usage JSONL here, but no such local usage files exist on this host |
| `<runtime>/debug/<session_id>.txt` | Debug logs when `QWEN_DEBUG_LOG_FILE` is enabled | Debug logging is opt-in; no local debug logs were found |

Current Qwen design docs define a newer artifact: each API response appends a content-free usage record to `usage/token-usage-YYYY-MM.jsonl` under the runtime directory. The documented record dimensions are local date, month, session id, model, auth type, source, token counters, and API duration; prompts, responses, tool content, project paths, prompt ids, and response ids are intentionally excluded. No such artifact exists on this host, consistent with the installed 0.15.6 binary being older than the current docs.

## Metrics and Windows

| Mechanism | Metric | Window | Reset Expression |
|---|---|---|---|
| Alibaba Coding Plan console | Overall plan consumption and remaining quota | 5-hour, weekly, monthly/subscription | 5-hour quota resets after 5 hours; weekly resets Monday 00:00:00 UTC+8; monthly resets at 00:00:00 UTC+8 on the corresponding next subscription-month day |
| `/stats` | Session duration, prompt count, API requests, prompt tokens, output tokens, tool calls, changed lines | Current session | No reset; session-scoped |
| `/stats model` | Per-model prompt, output/candidate, cached, thoughts, total tokens; requests/errors/latency in the TUI | Current session | No reset; session-scoped |
| `/stats daily` | Local token usage summary | Daily local date | Calendar day; exact timezone is the local date stored in Qwen's JSONL design |
| `/stats monthly` | Local token usage summary | Calendar month | Calendar month based on stored month |
| `/stats export --format json` | Aggregate daily/monthly summaries grouped by total, model, auth type, model/auth type, and source | Daily or monthly | Same as selected daily/monthly summary |
| Status line input | Context window used and remaining percentage | Current prompt/session context | No subscription reset; recalculated from last prompt token count and model context window |

Important distinction: Coding Plan quota is counted in model calls, not tokens. Qwen local `/stats` token counters can estimate local usage patterns but cannot authoritatively tell whether the 5-hour, weekly, or monthly Coding Plan request quota has remaining headroom.

## Limit States

| State | On-Demand Inspection | Runtime Markers | Notes |
|---|---|---|---|
| Cap approaching | Unknown | None found | No documented field, local artifact, or CLI display gives a warning threshold before the cap. |
| Capped, 5-hour | Not inspectable before a call through a documented CLI/API path | `hour allocated quota exceeded` | Alibaba says this means the 5-hour request quota is exhausted and resets automatically after 5 hours. |
| Capped, weekly | Not inspectable before a call through a documented CLI/API path | `week allocated quota exceeded` | Alibaba says this resets Monday 00:00:00 UTC+8. |
| Capped, monthly | Not inspectable before a call through a documented CLI/API path | `month allocated quota exceeded` | Alibaba says this resets on the next subscription-month corresponding day at 00:00:00 UTC+8. |
| Temporary resource throttle | Not inspectable before a call through a documented CLI/API path | `usage allocated quota exceeded. please try again later.` | Alibaba describes this as temporary throttling from high short-period resource consumption. |
| No funds / billing problem | Unknown as preflight | HTTP 402/403 or messages containing `billing` or `quota` in installed source | Installed Qwen source maps these to payment/quota style handling, but no balance lookup was found. |
| Auth required | Yes, via `qwen auth status` in installed 0.15.6, or by attempting non-interactive slash commands | `No auth type is selected`, `No authentication method configured`, `Use Qwen Code CLI to authenticate first.` | Current docs removed the standalone `qwen auth` command; prefer `/doctor` or configured settings in newer versions. |

## Sources

- [Qwen Code overview](https://qwenlm.github.io/qwen-code-docs/en/users/overview/)
- [Qwen Code commands](https://qwenlm.github.io/qwen-code-docs/en/users/features/commands/)
- [Qwen Code authentication](https://qwenlm.github.io/qwen-code-docs/en/users/configuration/auth/)
- [Qwen Code settings and usage statistics](https://qwenlm.github.io/qwen-code-docs/en/users/configuration/settings/)
- [Issue #4479 token usage stats coordination](https://qwenlm.github.io/qwen-code-docs/en/design/issue-4479-token-usage-stats-coordination/)
- [Alibaba Cloud Coding Plan FAQ](https://www.alibabacloud.com/help/en/model-studio/coding-plan-faq)
- [Alibaba Cloud Coding Plan overview](https://help.aliyun.com/en/model-studio/coding-plan)
- [GitHub issue #3302: Add /quota command](https://github.com/QwenLM/qwen-code/issues/3302)
- Observed on host: `qwen --version` returned `0.15.6`; installed package source at `/opt/homebrew/Cellar/qwen-code/0.15.6/libexec/lib/node_modules/@qwen-code/qwen-code/cli.js`
- Observed on host: local Qwen artifact search found no `~/.qwen`, no Qwen usage JSONL, and no macOS Application Support/Logs/Caches usage artifact
