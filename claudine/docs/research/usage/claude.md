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
    endpoint: GET https://api.anthropic.com/v1/organizations/usage_report/claude_code?starting_at=YYYY-MM-DD
    auth: enterprise_only
    users: Admin API key for Claude Platform organizations; Claude Enterprise uses a separate Analytics API key path.
    fields:
      - date
      - actor
      - organization_id
      - customer_type
      - terminal_type
      - core_metrics.num_sessions
      - core_metrics.lines_of_code.added
      - core_metrics.lines_of_code.removed
      - model_breakdown[].tokens.input
      - model_breakdown[].tokens.output
      - model_breakdown[].tokens.cache_read
      - model_breakdown[].tokens.cache_creation
      - model_breakdown[].estimated_cost.amount
      - next_page
    reset_window: Daily UTC aggregation; not current five-hour or seven-day subscription reset state.
    notes: Historical Claude Code organization analytics with up to 1-hour freshness delay; not available to individual subscribers.
  - available: true
    endpoint: GET https://api.anthropic.com/v1/organizations/usage_report/messages?starting_at=...&ending_at=...&bucket_width=1h
    auth: enterprise_only
    users: Admin API key for Claude Platform organizations; unavailable for individual accounts.
    fields:
      - bucket
      - group_by dimensions
      - uncached input tokens
      - cache read tokens
      - cache creation tokens
      - output tokens
      - server tool usage
    reset_window: Caller-selected bucket widths of 1m, 1h, or 1d over historical intervals; not current subscription-window reset state.
    notes: Organization API usage and cost reporting; useful for API-key-backed Claude Code, not for personal Max/Pro subscription headroom.
  - available: true
    endpoint: GET https://api.anthropic.com/v1/organizations/rate_limits
    auth: enterprise_only
    users: Admin API key for Claude Platform organizations; unavailable for individual accounts.
    fields:
      - data[].type
      - data[].group_type
      - data[].models
      - data[].limits[].type
      - data[].limits[].value
      - next_page
    reset_window: Configured per-minute API limits only; no live remaining usage or fixed reset timestamp in the response.
    notes: Can identify configured RPM, ITPM, OTPM, and other API limits; combine with usage reporting or response headers for headroom.
  - available: true
    endpoint: GET https://api.anthropic.com/v1/organizations/workspaces/{workspace_id}/rate_limits
    auth: enterprise_only
    users: Admin API key for Claude Platform organizations with workspace access; unavailable for individual accounts.
    fields:
      - data[].type
      - data[].group_type
      - data[].models
      - data[].limits[].type
      - data[].limits[].value
      - data[].limits[].org_limit
      - next_page
    reset_window: Configured workspace overrides only; no live remaining usage or fixed reset timestamp.
    notes: Returns overrides; missing groups inherit organization limits.
  - available: false
    endpoint: unknown Claude.ai subscription usage endpoint
    auth: unknown
    users: Individual Free, Pro, and Max subscribers.
    fields: []
    reset_window: unknown
    notes: Official docs and local CLI help do not expose a supported standalone HTTP API for current personal Claude Code five-hour or seven-day subscription usage; current values are surfaced through UI/status-line mechanisms.
cli_methods:
  - available: true
    invocation: statusLine command configured in ~/.claude/settings.json; Claude Code invokes it with JSON on stdin during an active session.
    interactive_only: true
    output_format: JSON stdin to user command
    fields:
      - rate_limits.five_hour.used_percentage
      - rate_limits.five_hour.resets_at
      - rate_limits.seven_day.used_percentage
      - rate_limits.seven_day.resets_at
      - context_window.used_percentage
      - context_window.remaining_percentage
      - cost.total_cost_usd
      - cost.total_duration_ms
      - cost.total_api_duration_ms
    notes: Observed on host in ~/.claude/statusline.log and documented in Claude Code status-line docs; fresh only while Claude Code is running and status line updates.
  - available: true
    invocation: /usage
    interactive_only: true
    output_format: styled human TUI/dialog
    fields:
      - current session token usage
      - estimated session cost
      - five-hour usage percent
      - weekly usage percent
      - reset labels
      - attribution categories
    notes: Official docs say /usage checks token usage and changelog says it displays plan limits and usage attribution; no documented JSON output.
  - available: true
    invocation: /usage-credits
    interactive_only: true
    output_format: styled human TUI/dialog
    fields:
      - usage credit state
      - monthly spend limit
      - overage enablement
    notes: Pro and Max users can set a monthly spend limit on usage credits; not a general current five-hour or seven-day usage query.
  - available: false
    invocation: claude usage or claude --usage
    interactive_only: false
    output_format: none
    fields: []
    notes: Local claude 2.1.199 --help did not list a usage subcommand or usage-reporting switch; official CLI reference documents flags but no usage subcommand.
pty_design:
  command: expectrl spawn of claude in an interactive PTY, then send /usage and capture the rendered dialog/screen text.
  first_pass_markers:
    - /usage
    - Session
    - Total cost
    - Total duration (API)
    - Total duration (wall)
    - 5-hour
    - weekly
    - Resets
  fuzzy_markers:
    - usage
    - limit
    - session
    - week
    - five hour
    - reset
    - remaining
    - credits
  fields:
    - session_cost
    - session_api_duration
    - session_wall_duration
    - five_hour_used_percentage
    - five_hour_reset
    - weekly_used_percentage
    - weekly_reset
    - attribution_categories
  risks: Scraped TUI text has no schema, no versioned contract, can be clipped or restyled, and may differ by terminal, plan, locale, or VS Code/native dialogs; use only when status-line JSON and official APIs are unavailable.
metrics:
  - name: five_hour_used_percentage
    unit: percent
    window: five_hour
    source: statusLine JSON rate_limits.five_hour.used_percentage; observed in ~/.claude/statusline.log:3986318 and documented in status-line schema.
    notes: Subscriber Claude.ai plan usage; headroom is 100 minus this value.
  - name: five_hour_resets_at
    unit: time
    window: five_hour
    source: statusLine JSON rate_limits.five_hour.resets_at; observed in ~/.claude/statusline.log:3986321.
    notes: Unix epoch seconds; local sample showed 1773961200.
  - name: seven_day_used_percentage
    unit: percent
    window: weekly
    source: statusLine JSON rate_limits.seven_day.used_percentage; observed in ~/.claude/statusline.log:3986323 and documented in status-line schema.
    notes: Subscriber Claude.ai plan usage; headroom is 100 minus this value.
  - name: seven_day_resets_at
    unit: time
    window: weekly
    source: statusLine JSON rate_limits.seven_day.resets_at; observed in ~/.claude/statusline.log:3986325.
    notes: Unix epoch seconds; local sample showed 1774026000.
  - name: context_window_used_percentage
    unit: percent
    window: session
    source: statusLine JSON context_window.used_percentage.
    notes: Current context window, not subscription quota.
  - name: context_window_remaining_percentage
    unit: percent
    window: session
    source: statusLine JSON context_window.remaining_percentage.
    notes: Current context window, not subscription quota.
  - name: session_estimated_cost
    unit: currency
    window: session
    source: /usage Session block and statusLine JSON cost.total_cost_usd.
    notes: Estimated locally and may differ from billing.
  - name: claude_code_daily_user_tokens
    unit: tokens
    window: daily
    source: Admin Claude Code Analytics API model_breakdown[].tokens.
    notes: Historical per-user daily aggregate with up to 1-hour freshness delay.
  - name: api_rate_limit_headers_remaining
    unit: tokens
    window: hourly
    source: Anthropic API response headers anthropic-ratelimit-*-remaining and anthropic-ratelimit-*-reset.
    notes: API-key traffic only; reset fields are RFC 3339.
limit_states:
  - state: cap_approaching
    detectable: true
    source: statusLine JSON and VS Code banner/changelog.
    markers:
      - rate_limits.five_hour.used_percentage >= local warning threshold
      - rate_limits.seven_day.used_percentage >= local warning threshold
      - rate limit warning banner with usage percentage and reset time
    notes: Changelog says a low-usage warning bug was fixed and warning now requires 70% usage; exact warning threshold is not documented as stable.
  - state: capped
    detectable: true
    source: Claude Code error reference and local transcript.
    markers:
      - You've hit your session limit
      - You've hit your weekly limit
      - resets HH:MM (Timezone)
      - error=rate_limit
      - apiErrorStatus=429
    notes: Observed on host in ~/.claude/projects/.../b320b41a-61dd-4aac-8e71-330fbbe7612a.jsonl:162 and :164.
  - state: no_funds
    detectable: true
    source: Claude Code error reference.
    markers:
      - Credit balance is too low
      - Usage credits required for 1M context
      - usage credits prompt
    notes: Distinct from plan cap; fast mode and 1M context can require usage credits.
  - state: auth_required
    detectable: true
    source: Claude Code error reference.
    markers:
      - Not logged in · Please run /login
      - Could not resolve authentication method
      - Invalid API key
      - OAuth token revoked or expired
    notes: Detect before interpreting missing usage fields as zero usage.
  - state: other
    detectable: true
    source: Claude Platform rate-limit docs.
    markers:
      - Request rejected (429)
      - Server is temporarily limiting requests
      - retry-after
      - anthropic-ratelimit-*-remaining
      - anthropic-ratelimit-*-reset
    notes: API transient/server or token-bucket rate limiting can be separate from Claude.ai subscription caps.
docs: https://code.claude.com/docs/en/statusline
changes: []
requires_claudine_update: true
reason: Claudine should add a plan-awareness reader for Claude Code status-line JSON rate_limits, with API/admin paths treated as secondary organization-only mechanisms and PTY scraping as a last resort.
---

# Claude Code Usage Inspection

## Introduction to Claude Code Usage Inspection

Claude Code has a mixed usage model. Claude.ai subscribers see plan-window limits, now surfaced as five-hour and seven-day windows in status-line JSON. API-key and organization deployments use token/cost billing, configured spend limits, API rate limits, and historical organization reporting. Usage credits can add metered overage for features such as fast mode or 1M context, depending on plan and admin enablement.

Officially, current personal plan usage is surfaced inside Claude Code UI surfaces: `/usage`, the status/footer area, VS Code account usage dialogs, and a configurable status line. Organization administrators get web dashboards and Admin APIs for historical Claude Code analytics, API usage/cost reports, configured rate limits, and spend-limit management. The important split for Claudine is that the personal five-hour/seven-day runway is not documented as a standalone `claude usage --json` command or public subscriber HTTP endpoint; the stable structured surface is the status-line JSON delivered during an active session.

## API Call Opportunities

| Mechanism | Endpoint | Auth and users | Fields | Reset/window data | Finding |
| --- | --- | --- | --- | --- | --- |
| Claude Code Analytics Admin API | `GET https://api.anthropic.com/v1/organizations/usage_report/claude_code?starting_at=2025-09-08&limit=20` | Admin API key for Claude Platform organizations; Claude Enterprise uses an Analytics API key; unavailable for individual accounts. | `date`, `actor`, `organization_id`, `customer_type`, `terminal_type`, `core_metrics`, `tool_actions`, `model_breakdown[].tokens`, `estimated_cost`, `next_page`. | Single UTC day from `starting_at`; data can lag up to 1 hour. | Useful for org historical reporting, not current five-hour/seven-day subscriber headroom. Source: [Claude Code Analytics API](https://platform.claude.com/docs/en/manage-claude/claude-code-analytics-api). |
| Usage and Cost Admin API | `GET https://api.anthropic.com/v1/organizations/usage_report/messages?starting_at=2025-01-08T00:00:00Z&ending_at=2025-01-15T00:00:00Z&bucket_width=1d` | Admin API key; unavailable for individual accounts; Claude Platform on AWS does not currently expose these programmatic endpoints. | Token usage by bucket, groupings such as model/workspace/service tier/context window/speed, server tool usage, cost data. | Caller-selected historical buckets: `1m`, `1h`, or `1d`; not fixed subscription resets. | Useful for API-key-backed usage analysis and cost reconciliation. Source: [Usage and Cost API](https://platform.claude.com/docs/en/manage-claude/usage-cost-api). |
| Organization Rate Limits API | `GET https://api.anthropic.com/v1/organizations/rate_limits` | Admin API key; unavailable for individual accounts. | `data[].type`, `group_type`, `models`, `limits[].type`, `limits[].value`, `next_page`. | Configured per-minute limits; no live remaining counters or reset timestamps in the response. | Useful for configured RPM/ITPM/OTPM ceilings. Source: [Rate Limits API](https://platform.claude.com/docs/en/manage-claude/rate-limits-api). |
| Workspace Rate Limits API | `GET https://api.anthropic.com/v1/organizations/workspaces/{workspace_id}/rate_limits` | Admin API key; workspace-aware organization users. | Workspace override entries with `org_limit`; missing groups inherit org limits. | Configured overrides only; no live remaining counters or reset timestamps. | Useful for workspace policy awareness, not current consumption. Source: [Rate Limits API](https://platform.claude.com/docs/en/manage-claude/rate-limits-api). |
| Subscriber current usage endpoint | Unknown | Unknown; no official public endpoint found for individual Free/Pro/Max subscription usage lookup. | Unknown. | Unknown. | Negative finding: local `claude 2.1.199 --help` and official CLI docs expose no standalone usage query, and official docs point current use to `/usage`, status line, web/settings UI, and dashboards. |

API response headers are a separate opportunity for API-key traffic. Anthropic documents `retry-after`, `anthropic-ratelimit-requests-*`, `anthropic-ratelimit-input-tokens-*`, and `anthropic-ratelimit-output-tokens-*` headers, including RFC 3339 reset times and remaining counts. Those headers are not an on-demand inspection endpoint, but they can enrich a tool that already makes API requests. Source: [Claude Platform rate limits](https://platform.claude.com/docs/en/api/rate-limits).

## CLI Switch Opportunities

No non-interactive usage-reporting flag or subcommand was found. On this host, `claude --version` reports `2.1.199 (Claude Code)`, and `claude --help` describes the CLI as interactive by default with `-p/--print` for non-interactive output, but it does not list `usage`, `stats`, `cost`, `--usage`, or a JSON usage switch. The official CLI reference likewise documents flags such as `--print`, `--output-format`, and system-prompt flags but not a usage-reporting subcommand. Source: observed on host with `claude --help`; [Claude Code CLI reference](https://code.claude.com/docs/en/cli-reference).

The closest structured CLI-adjacent mechanism is status-line invocation. Claude Code runs the configured `statusLine.command` and sends JSON on stdin. The official status-line schema includes:

```json
{
  "rate_limits": {
    "five_hour": {
      "used_percentage": 23.5,
      "resets_at": 1738425600
    },
    "seven_day": {
      "used_percentage": 41.2,
      "resets_at": 1738857600
    }
  }
}
```

On this host, `~/.claude/settings.json` configures `statusLine.command` as `~/.claude/statusline.sh`. That script logs each JSON payload to `~/.claude/statusline.log`. The observed log includes:

```json
{
  "rate_limits": {
    "five_hour": {
      "used_percentage": 15,
      "resets_at": 1773961200
    },
    "seven_day": {
      "used_percentage": 23,
      "resets_at": 1774026000
    }
  }
}
```

Observed-on-host reference: `~/.claude/statusline.log:3986318-3986326`. Documentation source: [Customize your status line](https://code.claude.com/docs/en/statusline).

## Interactive Commands and PTY Scraping

`/usage` is the primary interactive usage command. The costs documentation says the Session block at the top of `/usage` shows detailed token usage statistics, estimated cost, API duration, wall duration, and code-change counts; it also says `/usage` checks current token usage. Source: [Manage costs effectively](https://code.claude.com/docs/en/costs).

The installed changelog under `~/.claude/cache/changelog.md` adds current-plan detail: `/usage` displays plan limits, uses `% used`, has progress bars with reset labels, includes categories driving limits usage, and can switch attribution between day and week views. Relevant observed lines include `3996` (`/usage command to see plan limits`), `2300` (status-line `rate_limits`), `770` (per-category breakdown), `367` (24h/7d attribution), `3711` (`% used` progress), and `1137` (weekly reset display). This changelog is an observed local artifact, not a public stability contract.

`/usage-credits` is a separate interactive command. Official docs say Pro and Max users can set a monthly spend limit on usage credits with `/usage-credits`; if the spend limit blocks further usage while credits remain, Claude Code prompts for raising or removing the limit. Source: [Manage costs effectively](https://code.claude.com/docs/en/costs). It is not a full current five-hour/seven-day quota query.

The PTY scrape fallback should be deliberately conservative:

1. Spawn `claude` with `expectrl`, wait for the prompt, send `/usage`, then match exact markers from the current shape: `Session`, `Total cost`, `Total duration (API)`, `Total duration (wall)`, `5-hour`, `weekly`, `Resets`, `% used`, and known attribution labels such as `subagents`, `cache misses`, `long context`, `skills`, `plugins`, or `MCP`.
2. If exact parsing fails, capture the visible dialog text and run fuzzy extraction. Search case-insensitively for nearby terms: `usage`, `limit`, `session`, `week`, `reset`, `remaining`, `credits`, `cost`, `duration`, and percentage/countdown patterns. Return parsed fields with a low-confidence flag and retain the raw capture for diagnostics.

Scraped TUI text has no schema and no stability contract. It can change with version, terminal size, VS Code/native dialogs, plan tier, locale, and feature flags. It should be a last resort after status-line JSON, documented admin APIs, and local artifacts.

Passing `/usage` as a launch prompt was not established as a supported non-interactive preliminary command. The local changelog explicitly notes a fix for `claude agents` sending built-in slash commands such as `/usage` to background sessions as prompt text instead of showing a hint, which is evidence that slash-command execution is tied to interactive command handling rather than a general non-interactive CLI switch. Observed-on-host reference: `~/.claude/cache/changelog.md:141`.

## Config and Log Artifacts

| Path | Fields | Freshness | Notes |
| --- | --- | --- | --- |
| `~/.claude/settings.json` | `statusLine.type`, `statusLine.command`, plus auth/model/settings keys. | Updated when user settings change. | It tells a tool whether status-line JSON can be captured. On this host it points to `~/.claude/statusline.sh`. |
| `~/.claude/statusline.log` | Logged copies of status-line stdin JSON: `rate_limits.five_hour.used_percentage`, `rate_limits.five_hour.resets_at`, `rate_limits.seven_day.used_percentage`, `rate_limits.seven_day.resets_at`, `context_window`, `cost`. | Written every time this host's status-line script runs; fresh only during/after active sessions. | Strongest local structured artifact on this host. It is created by the user's script, not by Claude Code itself, so existence and retention are user-specific. |
| `~/.claude/statusline.sh` | Script reads JSON stdin, computes context/cost display, and appends raw input to `statusline.log`. | Runs live when Claude Code refreshes the status line. | User-installed/generated script. A different user may not log raw JSON. |
| `~/.claude/projects/**/*.jsonl` | Transcript entries include per-message `usage` token fields, `service_tier`, `speed`, and error records such as `error: "rate_limit"` and `apiErrorStatus: 429`. | Written during sessions. | Useful for historical local token accounting and limit-event evidence, but not reliable for current remaining headroom. Observed capped message at `~/.claude/projects/.../b320b41a-61dd-4aac-8e71-330fbbe7612a.jsonl:162`. |
| `~/.claude/stats-cache.json` | `dailyActivity[].date`, `messageCount`, `sessionCount`, `toolCallCount`; `lastComputedDate`. | Stale cache; on this host `lastComputedDate` is `2026-04-22`. | Local usage activity summary, not quota state. It had no `rate_limits`, reset, or credit fields. |
| `~/.claude/sessions/*.json` | Process/session metadata: `pid`, `sessionId`, `cwd`, `startedAt`, `version`, `kind`, `status`, `updatedAt`. | Live-ish session registry. | No quota or usage fields observed. |
| `~/.claude/cache/changelog.md` | Release-note evidence for `/usage`, `/usage-credits`, status-line `rate_limits`, warnings, and error handling. | Updated by Claude Code package/update flow. | Useful for feature history only; not user quota state. |
| `~/.claude/telemetry`, `~/.claude/statsig`, `~/.claude/cache` | No local usage/quota JSON fields found in targeted search, except changelog text. | Unknown. | No inspectable current quota artifact found there on this host. |

## Metrics and Windows

| Metric | Unit | Window | Source | Reset expression |
| --- | --- | --- | --- | --- |
| `rate_limits.five_hour.used_percentage` | Percent | Five-hour subscription window | Status-line JSON | Pair with `rate_limits.five_hour.resets_at`, Unix epoch seconds. |
| `rate_limits.seven_day.used_percentage` | Percent | Seven-day subscription window | Status-line JSON | Pair with `rate_limits.seven_day.resets_at`, Unix epoch seconds. |
| `context_window.used_percentage` / `remaining_percentage` | Percent | Current session context window | Status-line JSON | No reset; changes with context, compaction, and new sessions. |
| `context_window.current_usage.*` | Tokens | Most recent API call/current context window | Status-line JSON | No reset; current context state. |
| `cost.total_cost_usd` | Currency | Current session | Status-line JSON and `/usage` Session block | No reset except session start/clear; estimated locally. |
| `/usage` attribution categories | Percent and category labels | Last 24 hours and seven days | Interactive `/usage`; local changelog | TUI labels; not schema-stable. |
| Claude Code Analytics `model_breakdown[].tokens.*` | Tokens | Daily UTC aggregate | Admin API | `starting_at` selects one UTC date; data freshness up to 1 hour. |
| Claude Code Analytics `estimated_cost.amount` | Currency cents USD | Daily UTC aggregate | Admin API | Same daily aggregate. |
| API response `anthropic-ratelimit-*-remaining` | Requests/tokens | API token-bucket period | API response headers | `anthropic-ratelimit-*-reset` in RFC 3339. |

## Limit States

| State | Markers | Mechanisms | Notes |
| --- | --- | --- | --- |
| Cap approaching | `rate_limits.*.used_percentage` high; warning banner with percentage and reset time. | Status-line JSON; VS Code/UI warning. | Local changelog says warning threshold behavior was fixed and low-usage warnings now require 70% usage, but exact policy is not a stable schema. |
| Capped | `You've hit your session limit`, `You've hit your weekly limit`, reset text, local transcript `error: "rate_limit"`, `apiErrorStatus: 429`. | Interactive run errors, transcripts, non-interactive stream handling. | Detection during a run belongs to the sibling non-interactive-sessions topic; this document records the strings because local artifacts expose them. Source: [Error reference](https://code.claude.com/docs/en/errors). |
| No funds / credits required | `Credit balance is too low`, `Usage credits required for 1M context`, usage-credit prompts. | CLI/UI errors and `/usage-credits`. | Distinct from plan-window exhaustion. Fast mode also draws directly from usage credits when enabled. Sources: [Error reference](https://code.claude.com/docs/en/errors), [Fast mode](https://code.claude.com/docs/en/fast-mode). |
| Auth required | `Not logged in · Please run /login`, `Could not resolve authentication method`, `Invalid API key`, `OAuth token revoked or expired`. | CLI/UI errors. | Must be handled separately from absent quota fields. Source: [Error reference](https://code.claude.com/docs/en/errors). |
| API/server rate limit | `Request rejected (429)`, `Server is temporarily limiting requests`, `retry-after`, `anthropic-ratelimit-*-reset`. | API responses and Claude Code errors. | Can represent token-bucket/acceleration limits, not necessarily subscription cap. Source: [Claude Platform rate limits](https://platform.claude.com/docs/en/api/rate-limits). |

## Sources

- [Claude Code status line](https://code.claude.com/docs/en/statusline)
- [Claude Code costs and `/usage`](https://code.claude.com/docs/en/costs)
- [Claude Code error reference](https://code.claude.com/docs/en/errors)
- [Claude Code CLI reference](https://code.claude.com/docs/en/cli-reference)
- [Claude Code analytics dashboard docs](https://code.claude.com/docs/en/analytics)
- [Claude Code Analytics Admin API](https://platform.claude.com/docs/en/manage-claude/claude-code-analytics-api)
- [Usage and Cost Admin API](https://platform.claude.com/docs/en/manage-claude/usage-cost-api)
- [Rate Limits API](https://platform.claude.com/docs/en/manage-claude/rate-limits-api)
- [Claude Platform rate limits and response headers](https://platform.claude.com/docs/en/api/rate-limits)
- [Claude Code fast mode](https://code.claude.com/docs/en/fast-mode)
- Observed on host: `claude --version` and `claude --help` from Claude Code `2.1.199`.
- Observed on host: `~/.claude/settings.json`, `~/.claude/statusline.sh`, `~/.claude/statusline.log`, `~/.claude/stats-cache.json`, `~/.claude/sessions/*.json`, `~/.claude/projects/**/*.jsonl`, and `~/.claude/cache/changelog.md`.
