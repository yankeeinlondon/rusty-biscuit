---
$schema: ./_schema.yaml
created: 2026-07-03
last_updated: 2026-07-03
agent: codex
model: default
api: true
cli_switch: false
structured_output: true
pty_scrape: true
api_methods:
  - available: true
    endpoint: https://chatgpt.com/backend-api/wham/usage
    auth: oauth
    users: ChatGPT Plus/Pro users authenticated through Pi's openai-codex /login OAuth; OpenAI API keys do not expose this subscription quota.
    fields:
      - rate_limit.primary_window.used_percent
      - rate_limit.primary_window.reset_at
      - rate_limit.primary_window.reset_after_seconds
      - rate_limit.secondary_window.used_percent
      - rate_limit.secondary_window.reset_at
      - rate_limit.secondary_window.reset_after_seconds
      - rate_limit.allowed
      - rate_limit.limit_reached
      - rate_limit_reached_type
      - plan_type
      - email
    reset_window: Primary 5-hour window and secondary weekly window; reset timestamps and reset-after seconds are parsed when present.
    notes: Private ChatGPT Codex subscription endpoint used by Pi quota extensions, not documented as a stable Pi API.
  - available: true
    endpoint: https://api.anthropic.com/api/oauth/usage
    auth: oauth
    users: Claude Pro/Max OAuth users authenticated through Pi /login; Anthropic API-key users may be billed per token and may not receive subscription windows.
    fields:
      - five_hour.utilization
      - five_hour.resets_at
      - seven_day.utilization
      - seven_day.resets_at
      - seven_day_sonnet.utilization
      - seven_day_sonnet.resets_at
      - seven_day_opus.utilization
      - seven_day_opus.resets_at
    reset_window: 5-hour and seven-day windows, plus model-family weekly windows when present; reset timestamps are parsed from resets_at.
    notes: Requires the anthropic-beta oauth-2025-04-20 header in observed extension code.
  - available: true
    endpoint: https://api.z.ai/api/monitor/usage/quota/limit
    auth: api_key
    users: Z.ai Coding Plan users with a ZAI_API_KEY or Pi auth.json zai credential.
    fields:
      - data.limits[].type
      - data.limits[].percentage
      - data.limits[].nextResetTime
      - success
      - msg
    reset_window: Unknown named window; the TOKENS_LIMIT record carries nextResetTime in Unix milliseconds when present.
    notes: Used by the @harms-haus/pi-zai-usage extension; the package calls it the official Z.ai API.
  - available: true
    endpoint: https://openrouter.ai/api/v1/credits
    auth: api_key
    users: OpenRouter API-key users.
    fields:
      - data.total_credits
      - data.total_usage
    reset_window: No reset window for credits in the observed extension.
    notes: "@porche/pi-usage also probes /key for per-key limit fields."
  - available: true
    endpoint: https://openrouter.ai/api/v1/key
    auth: api_key
    users: OpenRouter API-key users.
    fields:
      - data.limit
      - data.limit_remaining
      - data.limit_reset
    reset_window: limit_reset when present.
    notes: Used as a companion to /credits by @porche/pi-usage.
  - available: true
    endpoint: https://api.github.com/copilot_internal/user
    auth: oauth
    users: GitHub Copilot users with a Pi github-copilot credential, GITHUB_TOKEN/GH_TOKEN, or a gh CLI token; may require token exchange first.
    fields:
      - quota_snapshots.*.entitlement
      - quota_snapshots.*.remaining
      - quota_reset_date
      - monthly_quotas
      - limited_user_quotas
      - quota_windows[]
    reset_window: Monthly or provider-labeled quota reset fields when returned.
    notes: "@porche/pi-usage first tries https://api.github.com/copilot_internal/v2/token to exchange a token, then calls this usage endpoint."
cli_methods:
  - available: false
    invocation: pi --help
    interactive_only: false
    output_format: styled human text
    fields: []
    notes: Stock Pi CLI help and official CLI docs list no usage/quota/reporting subcommand or switch.
  - available: true
    invocation: /session
    interactive_only: true
    output_format: styled human text
    fields:
      - session file
      - session ID
      - messages
      - tokens
      - cost
    notes: Built-in interactive command; useful for session consumption, not provider account quota.
  - available: true
    invocation: pi --mode json "<prompt>"
    interactive_only: false
    output_format: JSONL event stream
    fields:
      - message.usage.input
      - message.usage.output
      - message.usage.cacheRead
      - message.usage.cacheWrite
      - message.usage.totalTokens
      - message.usage.cost.total
    notes: Non-interactive structured session usage only; it requires starting a run and does not inspect account quota before a run.
  - available: true
    invocation: /quota
    interactive_only: true
    output_format: styled human text and custom session message
    fields:
      - provider/model
      - remaining percent
      - reset
      - source
      - dimension
      - freshness
    notes: Provided by the optional pi-quota-status extension, not stock Pi.
  - available: true
    invocation: /quota debug
    interactive_only: true
    output_format: styled human text and custom session message
    fields:
      - config path
      - state path
      - active model
      - matched adapter
      - subscription auth
      - context usage
      - quota source
    notes: Provided by the optional pi-quota-status extension.
  - available: true
    invocation: /usage [limits|local|openai-codex|anthropic|github-copilot|openrouter]
    interactive_only: true
    output_format: styled human text
    fields:
      - session token usage
      - account quota remaining percent
      - reset time
      - local 24h usage
      - local 7d usage
      - local 30d usage
    notes: Provided by the optional @porche/pi-usage extension, not stock Pi.
pty_design:
  command: pi with an installed usage extension, then send /quota or /usage through an expectrl-controlled PTY after startup settles.
  first_pass_markers:
    - Account limits
    - 5h window
    - Weekly
    - left
    - Resets
    - provider/model
    - remaining
    - reset
  fuzzy_markers:
    - quota
    - usage
    - limit
    - remaining
    - left
    - reset
    - 5h
    - weekly
    - tokens
    - cost
  fields:
    - provider
    - model
    - short_window_remaining_percent
    - short_window_reset
    - weekly_remaining_percent
    - weekly_reset
    - tokens
    - cost
  risks: TUI and extension text has no schema, versioning, or stability contract; colors, progress bars, labels, and extension behavior can drift without a machine-readable failure.
metrics:
  - name: session_input_tokens
    unit: tokens
    window: session
    source: Pi JSONL sessions, JSON event stream, /session
    notes: Assistant message usage.input.
  - name: session_output_tokens
    unit: tokens
    window: session
    source: Pi JSONL sessions, JSON event stream, /session
    notes: Assistant message usage.output.
  - name: session_cache_tokens
    unit: tokens
    window: session
    source: Pi JSONL sessions, JSON event stream, /session
    notes: cacheRead plus cacheWrite.
  - name: session_cost
    unit: currency
    window: session
    source: Pi JSONL sessions, JSON event stream, /session
    notes: usage.cost.total when the provider/model reports cost.
  - name: context_usage_percent
    unit: percent
    window: session
    source: Extension ctx.getContextUsage and pi-quota-status debug output
    notes: Current context usage, not account quota.
  - name: openai_codex_5h_remaining
    unit: percent
    window: five_hour
    source: chatgpt.com/backend-api/wham/usage via optional extensions
    notes: Computed as 100 - primary_window.used_percent.
  - name: openai_codex_weekly_remaining
    unit: percent
    window: weekly
    source: chatgpt.com/backend-api/wham/usage via optional extensions
    notes: Computed as 100 - secondary_window.used_percent.
  - name: anthropic_5h_remaining
    unit: percent
    window: five_hour
    source: api.anthropic.com/api/oauth/usage via optional extensions
    notes: Computed as 100 - utilization percent.
  - name: anthropic_weekly_remaining
    unit: percent
    window: weekly
    source: api.anthropic.com/api/oauth/usage via optional extensions
    notes: seven_day, seven_day_sonnet, and seven_day_opus may be present.
  - name: zai_token_quota_used
    unit: percent
    window: unknown
    source: api.z.ai/api/monitor/usage/quota/limit via @harms-haus/pi-zai-usage
    notes: TOKENS_LIMIT.percentage is used percent; resetTimeMs comes from nextResetTime.
  - name: openrouter_credits
    unit: currency
    window: billing_cycle
    source: openrouter.ai /credits and /key via @porche/pi-usage
    notes: total_credits, total_usage, limit_remaining, and limit_reset when present.
limit_states:
  - state: cap_approaching
    detectable: true
    source: pi-quota-status and @porche/pi-usage output
    markers:
      - remaining percent below warning threshold
      - warning color/status below 25 percent in pi-quota-status
      - yellow traffic-light band between 30 and 70 percent in @porche/pi-usage
    notes: Thresholds are extension policy, not Pi protocol.
  - state: capped
    detectable: true
    source: Provider endpoint responses and extension output
    markers:
      - rate_limit.limit_reached=true
      - rate_limit_reached_type
      - remaining=0
      - Req 0%
      - statusline error background when either Codex quota window is exhausted
    notes: OpenAI Codex carries the clearest observed markers.
  - state: no_funds
    detectable: true
    source: OpenRouter /credits and /key
    markers:
      - total_credits - total_usage <= 0
      - limit_remaining=0
    notes: Pi core has no provider-agnostic no-funds enum for inspection.
  - state: auth_required
    detectable: true
    source: Pi auth file and extension error strings
    markers:
      - No openai-codex OAuth in ~/.pi/agent/auth.json. Run /login.
      - No Anthropic auth found. Use /login or set ANTHROPIC_API_KEY.
      - No OPENROUTER_API_KEY
      - No GitHub Copilot token found.
      - auth.json is empty object
    notes: Observed host auth.json is {}, so no local Pi account quota probe could be authenticated.
  - state: unknown
    detectable: true
    source: pi-quota-status output
    markers:
      - quota n/a (sub)
      - No provider quota data
      - No tracked quota data yet.
    notes: Distinguishes missing/unavailable quota from exhausted quota.
docs: https://pi.dev/docs/latest/usage
changes: []
requires_claudine_update: true
reason: "Claudine would need Pi-specific plan-awareness support for three distinct mechanisms: structured session JSONL parsing, provider-specific private quota endpoints, and optional extension slash-command/TUI scraping. Pi does not expose one stable first-party usage API or non-interactive CLI usage command."
---

# Pi Usage Inspection

## Introduction to Pi Usage Inspection

Pi is a multi-provider coding-agent harness, not a single subscription service. Its official provider model mixes `/login` OAuth subscription providers, API-key providers, environment-variable credentials, and custom provider registration. Official docs say Pi supports subscription-based providers through OAuth and API-key providers through environment variables or `auth.json`; subscription `/login` currently covers ChatGPT Plus/Pro Codex, Claude Pro/Max, and GitHub Copilot, with credentials stored in `~/.pi/agent/auth.json` and auto-refreshed when expired ([Pi Providers](https://pi.dev/docs/latest/providers)).

Pi officially surfaces session consumption, not universal account quota. The stock interactive `/session` command shows the session file, ID, messages, tokens, and cost, and sessions are stored as JSONL under `~/.pi/agent/sessions/` ([Using Pi](https://pi.dev/docs/latest/usage), [Session File Format](https://pi.dev/docs/latest/session-format)). Account quota/headroom is provider-specific today: it can be polled through provider endpoints if credentials are available, or displayed by optional Pi extension packages such as `pi-quota-status`, `@porche/pi-usage`, `@llblab/pi-codex-usage`, and `@harms-haus/pi-zai-usage` ([pi-quota-status](https://pi.dev/packages/pi-quota-status), [@porche/pi-usage](https://pi.dev/packages/%40porche/pi-usage), [@llblab/pi-codex-usage](https://pi.dev/packages/%40llblab/pi-codex-usage), [@harms-haus/pi-zai-usage](https://pi.dev/packages/%40harms-haus/pi-zai-usage)).

On this host, the installed Pi resolves its agent directory to `/Users/ken/.claudine/.pi/agent` because the process home is `/Users/ken/.claudine`. I inspected that provider config directory. It contains `auth.json` with `{}`, mode `0600`, and an empty `sessions` tree; no Pi session JSONL files and no quota-extension state files were present. This means local artifacts could not authenticate live provider quota probes and did not contain cached usage or quota observations (observed on host: `/Users/ken/.claudine/.pi/agent/auth.json`, `/Users/ken/.claudine/.pi/agent/sessions`, `/Users/ken/.claudine/.pi/agent/pi-quota-status` absent).

## API Call Opportunities

There is no documented Pi-owned HTTP endpoint that returns the current user's universal Pi usage or plan limits. The API opportunities are provider-specific and, in several cases, private or extension-discovered.

| Provider/scope | Endpoint | Auth | Users | Fields and reset data | Evidence |
|---|---|---|---|---|---|
| OpenAI Codex subscription | `GET https://chatgpt.com/backend-api/wham/usage` | Bearer OAuth token from Pi `openai-codex` credential; `ChatGPT-Account-Id` when extractable from token | ChatGPT Plus/Pro Codex subscription users; OpenAI API keys do not expose these quotas | `rate_limit.primary_window.used_percent`, `reset_at`, `reset_after_seconds`; `secondary_window` for weekly; `allowed`, `limit_reached`, `rate_limit_reached_type`, `plan_type`, `email` | `pi-quota-status` source parses this endpoint, and its package page says it polls the ChatGPT Codex usage endpoint for 5-hour and weekly limits ([pi-quota-status](https://pi.dev/packages/pi-quota-status)); `@llblab/pi-codex-usage` says OpenAI API keys are not ChatGPT Codex subscription auth ([pi-codex-usage](https://pi.dev/packages/%40llblab/pi-codex-usage)). |
| Anthropic subscription | `GET https://api.anthropic.com/api/oauth/usage` | Bearer OAuth token; observed header `anthropic-beta: oauth-2025-04-20` | Claude Pro/Max OAuth users; API-key billing may not yield plan windows | `five_hour.utilization`, `five_hour.resets_at`, `seven_day.utilization`, `seven_day.resets_at`, plus `seven_day_sonnet`/`seven_day_opus` when present | Pi docs note Anthropic subscription auth and extra-usage billing ([Pi Providers](https://pi.dev/docs/latest/providers)); `pi-quota-status` and `@porche/pi-usage` source parse this endpoint. |
| Z.ai Coding Plan | `GET https://api.z.ai/api/monitor/usage/quota/limit` | Bearer Z.ai API key from Pi model registry or `ZAI_API_KEY` | Z.ai plan users | Response shape observed in extension code: `success`, `data.limits[]`; `TOKENS_LIMIT.percentage`; `TOKENS_LIMIT.nextResetTime` in Unix ms | `@harms-haus/pi-zai-usage` documents the endpoint, `TOKENS_LIMIT`, `percentage`, and `resetTimeMs` status payload ([pi-zai-usage](https://pi.dev/packages/%40harms-haus/pi-zai-usage)). |
| OpenRouter credits | `GET https://openrouter.ai/api/v1/credits` | Bearer OpenRouter API key | OpenRouter API-key users | `data.total_credits`, `data.total_usage`; no reset | `@porche/pi-usage` source uses `/credits` and documents OpenRouter support ([pi-usage](https://pi.dev/packages/%40porche/pi-usage)). |
| OpenRouter key limits | `GET https://openrouter.ai/api/v1/key` | Bearer OpenRouter API key | OpenRouter API-key users | `data.limit`, `data.limit_remaining`, `data.limit_reset` | `@porche/pi-usage` source uses `/key` as a companion quota call. |
| GitHub Copilot | `GET https://api.github.com/copilot_internal/user`, optionally after `GET https://api.github.com/copilot_internal/v2/token` | Pi `github-copilot` OAuth, `GITHUB_TOKEN`, `GH_TOKEN`, or `gh auth token`; token exchange may be needed | GitHub Copilot users | `quota_snapshots`, `monthly_quotas`, `limited_user_quotas`, `quota_windows`, reset fields such as `quota_reset_date` | `@porche/pi-usage` documents GitHub Copilot support and `gh auth token` fallback ([pi-usage](https://pi.dev/packages/%40porche/pi-usage)). |

Example requests, redacted:

```bash
curl -sS https://chatgpt.com/backend-api/wham/usage \
  -H "Authorization: Bearer $OPENAI_CODEX_OAUTH_ACCESS_TOKEN" \
  -H "Accept: application/json" \
  -H "User-Agent: codex-cli" \
  -H "ChatGPT-Account-Id: $CHATGPT_ACCOUNT_ID"
```

```bash
curl -sS https://api.anthropic.com/api/oauth/usage \
  -H "Authorization: Bearer $ANTHROPIC_OAUTH_ACCESS_TOKEN" \
  -H "Accept: application/json" \
  -H "Content-Type: application/json" \
  -H "anthropic-beta: oauth-2025-04-20"
```

```bash
curl -sS https://api.z.ai/api/monitor/usage/quota/limit \
  -H "Authorization: Bearer $ZAI_API_KEY" \
  -H "Accept-Encoding: identity"
```

Negative probe finding: on this host, `auth.json` is `{}`, so authenticated subscription probes cannot be run without prompting for `/login`. This is an auth-required state, not evidence that the endpoints are unavailable (observed on host: `/Users/ken/.claudine/.pi/agent/auth.json`).

## CLI Switch Opportunities

Stock Pi has no non-interactive `usage`, `quota`, `limits`, or `status` CLI switch. Official CLI docs list model, session, tool, resource, prompt, version, and help options, but no usage-reporting command; `pi --help` on this host likewise showed package commands and options but no usage/quota command ([Using Pi](https://pi.dev/docs/latest/usage); observed on host: `pi --help`, 2026-07-03).

Structured stock output exists for run events, not pre-run quota inspection:

```bash
pi --mode json "List files" 2>/dev/null | jq -c 'select(.type == "message_end")'
```

Pi's JSON mode emits JSON lines, including assistant messages with `usage` fields, but only while running an agent session ([JSON Event Stream Mode](https://pi.dev/docs/latest/json)). It is useful for consumption accounting after a run starts, not for deciding whether a run is worth starting.

Optional extension commands:

| Invocation | Provider | Interactive? | Structured? | What it reports | Evidence |
|---|---|---:|---:|---|---|
| `/quota` | `pi-quota-status` | Yes | No | Tracked `/login` subscription model rows: provider/model, remaining percent, reset, source, dimension, freshness | Package docs list `/quota`, `/quota config`, `/quota reload`, `/quota debug` ([pi-quota-status](https://pi.dev/packages/pi-quota-status)). |
| `/quota debug` | `pi-quota-status` | Yes | No | Config/state paths, active model, matched adapter, subscription auth, context usage, quota source | Package docs and observed source. |
| `/usage` | `@porche/pi-usage` | Yes | No | Session token usage and provider account limits for the active provider | Package docs list `/usage` commands and provider support ([pi-usage](https://pi.dev/packages/%40porche/pi-usage)). |
| `/usage local` | `@porche/pi-usage` | Yes | No | Local 24h, 7d, and 30d session usage from Pi session files | Package docs and observed source. |

Unknown: no installed extension on this host provided a standalone `pi-usage --json`, `pi-quota-status --json`, or equivalent structured CLI. The package pages identify these as Pi extensions, not standalone CLIs.

## Interactive Commands and PTY Scraping

Stock interactive command:

- `/session` shows the current session file, ID, messages, tokens, and cost. This is session consumption only, not account quota ([Using Pi](https://pi.dev/docs/latest/usage)).

Optional extension commands:

- `/quota` from `pi-quota-status` shows provider/model, remaining percent, reset, source, dimension, and freshness for tracked `/login` subscription models. `/quota debug` adds config/state paths, active model, adapter status, subscription auth, context usage, and quota source ([pi-quota-status](https://pi.dev/packages/pi-quota-status)).
- `/usage` from `@porche/pi-usage` shows live session token usage, provider account limits, and fallback local historical usage. `/usage limits`, `/usage <provider>`, and `/usage local` narrow the report ([pi-usage](https://pi.dev/packages/%40porche/pi-usage)).
- `@llblab/pi-codex-usage` does not require commands; it writes a compact statusline for OpenAI Codex subscription models. It encodes 5-hour and weekly windows into a dual quota bar and displays countdowns when reset timestamps are available ([pi-codex-usage](https://pi.dev/packages/%40llblab/pi-codex-usage)).
- `@harms-haus/pi-zai-usage` publishes a status payload under `zai-usage` with `percentage` and `resetTimeMs`; display usually depends on `pi-powerline` ([pi-zai-usage](https://pi.dev/packages/%40harms-haus/pi-zai-usage)).

Two-pass `expectrl` scraping design:

1. Start `pi` in a PTY with extensions disabled or enabled according to the target mechanism. For stock session usage, send `/session`. For provider quotas, install or load the chosen extension and send `/quota`, `/quota debug`, or `/usage`.
2. First pass: match exact known markers. For `/usage`, parse lines containing `Account limits`, `5h window`, `Weekly`, `left`, and `Resets`. For `/quota`, parse table headings and fields such as `provider/model`, `remaining`, `reset`, `source`, `dimension`, and `freshness`. For statusline extensions, capture footer/status text and parse labels such as `codex`, `spark`, `5h/7d`, `n/a`, and `error`.
3. Second pass: run only when exact markers fail. Strip ANSI, normalize box/table glyphs and progress-bar characters, then fuzzy-search for quota terms: `quota`, `usage`, `limit`, `remaining`, `left`, `reset`, `5h`, `weekly`, `tokens`, `cost`, `n/a`, `error`. Associate nearby percentages, countdowns, timestamps, and provider/model labels.
4. Emit confidence with every parsed field. Exact marker matches can be medium confidence; fuzzy matches are low confidence and should trigger telemetry or a diagnostic requesting API/JSON support.

Caveat: scraped TUI text has no schema and no stability contract. Nothing versions or validates extension display text, footer layout, colors, progress bars, or human labels. PTY scraping is strictly a last resort behind API JSON, session JSONL, and extension state files.

## Config and Log Artifacts

Official and observed artifacts:

| Artifact | Freshness | Fields | Finding |
|---|---|---|---|
| `~/.pi/agent/auth.json` or resolved `getAgentDir()/auth.json` | Live credential store; updated by `/login`, `/logout`, token refresh, and API-key storage | Provider keys, `type: "api_key"` or `type: "oauth"`, OAuth `access`, `refresh`, `expires`, possible account metadata | Official docs describe auth storage and `0600` permissions ([Pi Providers](https://pi.dev/docs/latest/providers)). On this host the resolved file is `/Users/ken/.claudine/.pi/agent/auth.json`, mode `0600`, value `{}`. |
| `~/.pi/agent/sessions/**.jsonl` or resolved `getAgentDir()/sessions/**.jsonl` | Written as sessions run; stale for pre-run quota but authoritative for historical session consumption | Assistant `message.usage.input`, `output`, `cacheRead`, `cacheWrite`, `totalTokens`, `cost.total`, timestamps | Official session format documents JSONL storage and usage fields ([Session File Format](https://pi.dev/docs/latest/session-format)). On this host the sessions directory exists but contains no JSONL files. |
| `~/.pi/agent/pi-quota-status/config.json` | User-editable extension config; absent means defaults at runtime | Adapter config, generic header mappings, fallback quotas, thresholds | `pi-quota-status` documents this file ([pi-quota-status](https://pi.dev/packages/pi-quota-status)). It is absent on this host. |
| `~/.pi/agent/pi-quota-status/state.json` | Extension cache/state; updated after subscription polling, provider headers, or fallback deduction | Parsed quota observations only; no raw headers, prompts, responses, or tokens | `pi-quota-status` documents state storage and privacy behavior ([pi-quota-status](https://pi.dev/packages/pi-quota-status)). It is absent on this host. |
| Extension status slots, for example `ctx.ui.setStatus("pi-quota-status", ...)` and `ctx.ui.setStatus("zai-usage", { percentage, resetTimeMs })` | Live UI state, not necessarily persisted | Remaining percent, reset time, compact display | Pi extension docs expose `ctx.ui.setStatus`; package docs describe specific status payloads ([Pi Extensions](https://pi.dev/docs/latest/extensions), [pi-zai-usage](https://pi.dev/packages/%40harms-haus/pi-zai-usage)). |

No Pi log, cache, statsig-style telemetry, or quota state artifact containing provider usage was present under the inspected provider config directory on this host.

## Metrics and Windows

| Mechanism | Metrics | Window | Reset expression |
|---|---|---|---|
| Stock `/session` | Tokens and cost | Session | None |
| JSON mode | `message.usage.input`, `output`, `cacheRead`, `cacheWrite`, `totalTokens`, `cost.total` | Session/run | None |
| Session JSONL | Same `Usage` shape persisted in assistant messages | Session/history | None |
| `ctx.getContextUsage()` | Context tokens, context window, percent | Current session context | None |
| OpenAI Codex `/wham/usage` | Used percent and computed remaining percent | 5-hour primary, weekly secondary, additional buckets when present | `reset_at` timestamp and/or `reset_after_seconds` countdown |
| Anthropic `/api/oauth/usage` | Utilization and computed remaining percent | 5-hour, seven-day, model-family weekly windows | `resets_at` timestamp |
| Z.ai `/api/monitor/usage/quota/limit` | Token-quota used percent | Unknown provider-defined token quota window | `nextResetTime` Unix milliseconds |
| OpenRouter `/credits` | Credits total, credits used, credits remaining | Billing/credit balance | None |
| OpenRouter `/key` | Key limit, remaining limit, reset | Provider key quota window | `limit_reset` |
| GitHub Copilot internal endpoints | Monthly quota snapshots, remaining, unlimited flags, overage | Monthly or provider-defined quota windows | `quota_reset_date`, `quota_reset_date_utc`, `limited_user_reset_date`, or per-window reset fields |

Reset times are mixed: provider JSON may return ISO timestamps, Unix milliseconds, provider strings, or reset-after seconds; extension displays localize them into human text and countdowns. `@porche/pi-usage` documents timezone selection through `PI_USAGE_TZ`, `TZ`, runtime timezone, then UTC fallback ([pi-usage](https://pi.dev/packages/%40porche/pi-usage)).

## Limit States

| State | How to distinguish | Mechanisms | Caveats |
|---|---|---|---|
| cap-approaching | Remaining percent below extension thresholds; warning/critical colors; traffic-light bands | `pi-quota-status`, `@porche/pi-usage`, statusline extensions | Thresholds are extension UI policy, not a Pi schema. |
| capped | `limit_reached=true`, `rate_limit_reached_type`, remaining `0`, `Req 0%`, `error` statusline when a quota window is exhausted | OpenAI Codex endpoint and quota extensions | Provider fields vary. |
| out of funds | OpenRouter credit balance `total_credits - total_usage <= 0`, or key `limit_remaining=0` | OpenRouter `/credits` and `/key` | Other providers may express no-funds as an API error during a run, which belongs to the sibling non-interactive-sessions topic. |
| auth required | Empty/missing `auth.json` provider entry, missing env key, extension strings such as `Run /login`, `No OPENROUTER_API_KEY`, `No GitHub Copilot token found` | Local auth file and extension preflight | Observed host is auth-required for Pi subscription quota because `auth.json` is `{}`. |
| unknown/unavailable | `quota n/a (sub)`, `No provider quota data`, `No tracked quota data yet`, `No limits data`, `Unavailable` | Extension output and absent local state | Treat as unknown, not as safe headroom. |

## Sources

- [Pi official usage docs](https://pi.dev/docs/latest/usage)
- [Pi official providers docs](https://pi.dev/docs/latest/providers)
- [Pi official session format docs](https://pi.dev/docs/latest/session-format)
- [Pi official JSON event stream docs](https://pi.dev/docs/latest/json)
- [Pi official RPC docs](https://pi.dev/docs/latest/rpc)
- [Pi official extension docs](https://pi.dev/docs/latest/extensions)
- [pi-quota-status package page](https://pi.dev/packages/pi-quota-status)
- [@porche/pi-usage package page](https://pi.dev/packages/%40porche/pi-usage)
- [@llblab/pi-codex-usage package page](https://pi.dev/packages/%40llblab/pi-codex-usage)
- [@harms-haus/pi-zai-usage package page](https://pi.dev/packages/%40harms-haus/pi-zai-usage)
- Observed on host: `/Users/ken/.claudine/.pi/agent/auth.json`, `/Users/ken/.claudine/.pi/agent/sessions`, absence of `/Users/ken/.claudine/.pi/agent/pi-quota-status/`
- Observed package source unpacked from npm on 2026-07-03: `@earendil-works/pi-coding-agent@0.80.3`, `pi-quota-status@0.3.0`, `@porche/pi-usage@0.3.5`, `@llblab/pi-codex-usage@0.8.0`, `@harms-haus/pi-zai-usage@0.1.0`
