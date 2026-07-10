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
    endpoint: "GET http://<opencode-server>/session/{sessionID}"
    auth: unknown
    users: "Any local or remote OpenCode server user who can connect to the server; remote servers may require HTTP Basic auth."
    fields:
      - "cost"
      - "tokens.input"
      - "tokens.output"
      - "tokens.reasoning"
      - "tokens.cache.read"
      - "tokens.cache.write"
      - "time.created"
      - "time.updated"
    reset_window: "none; returns session/message usage, not plan-window reset timestamps"
    notes: "Structured local-server API for stored session usage. It does not expose Go/Zen plan quota, remaining headroom, or reset time."
  - available: true
    endpoint: "GET http://<opencode-server>/session/status"
    auth: unknown
    users: "Any local or remote OpenCode server user who can connect to the server; remote servers may require HTTP Basic auth."
    fields:
      - "session status map"
    reset_window: "none"
    notes: "Useful for active/idle/completed state, but not usage or plan quota."
  - available: false
    endpoint: "GET /auth/usage"
    auth: unknown
    users: "unknown"
    fields: []
    reset_window: "unknown"
    notes: "Proposed in GitHub issue #8911, but absent from the current generated OpenAPI spec and current CLI help."
cli_methods:
  - available: true
    invocation: "opencode stats [--days N] [--project] [--tools N] [--models N]"
    interactive_only: false
    output_format: "styled human text"
    fields:
      - "sessions"
      - "messages"
      - "days"
      - "total cost"
      - "average cost per day"
      - "average tokens per session"
      - "median tokens per session"
      - "input tokens"
      - "output tokens"
      - "cache read tokens"
      - "cache write tokens"
      - "per-model usage when requested"
      - "per-tool usage when requested"
    notes: "Native non-interactive CLI usage summary over stored sessions. No JSON flag was exposed by installed OpenCode 1.17.13, and it does not report plan limits or reset windows."
  - available: true
    invocation: "opencode session list --format json"
    interactive_only: false
    output_format: "JSON"
    fields:
      - "session id"
      - "title"
      - "created"
      - "updated"
      - "projectId"
      - "directory"
    notes: "Structured discovery of session IDs; combine with local storage or server session APIs to inspect token/cost fields."
  - available: true
    invocation: "opencode run --format json <prompt>"
    interactive_only: false
    output_format: "JSON event stream"
    fields:
      - "raw run events"
    notes: "Structured during-run events, not an on-demand quota query. Mentioned only because it can expose fresh per-run usage after a run."
  - available: true
    invocation: "/status"
    interactive_only: true
    output_format: "TUI dialog text"
    fields:
      - "system status"
      - "MCP/LSP status"
    notes: "Native TUI status command. It is advertised in the footer, but source inspection did not show plan quota fields."
  - available: false
    invocation: "opencode auth usage"
    interactive_only: false
    output_format: "unknown"
    fields: []
    notes: "Proposed in GitHub issue #8911, but not present in installed OpenCode 1.17.13 help."
pty_design:
  command: "opencode"
  first_pass_markers:
    - "/status"
    - "Input"
    - "Output"
    - "Cache Read"
    - "Cache Write"
    - "$"
  fuzzy_markers:
    - "usage"
    - "quota"
    - "limit"
    - "remaining"
    - "reset"
    - "tokens"
    - "cost"
  fields:
    - "tokens"
    - "cost"
    - "context percent"
    - "quota percent"
    - "reset countdown"
    - "limit state"
  risks: "Native TUI text has no schema or stability contract. Scraping is last-resort only; prefer server/local JSON for token/cost usage and official console/API if OpenCode publishes plan quota endpoints."
metrics:
  - name: "Go rolling usage"
    unit: currency
    window: five_hour
    source: "OpenCode Go official docs and console support source"
    notes: "Official docs state a $12 5-hour limit; support source labels the rolling window and formats reset countdowns."
  - name: "Go weekly usage"
    unit: currency
    window: weekly
    source: "OpenCode Go official docs and console support source"
    notes: "Official docs state a $30 weekly limit; support source computes weekly usage and reset."
  - name: "Go monthly usage"
    unit: currency
    window: monthly
    source: "OpenCode Go official docs and console support source"
    notes: "Official docs state a $60 monthly limit; support source computes monthly usage and reset."
  - name: "Zen workspace/member usage"
    unit: currency
    window: monthly
    source: "OpenCode Zen official docs"
    notes: "Monthly spend limits are configurable in Zen; no local CLI/API query for the current value was found."
  - name: "Stored session token usage"
    unit: tokens
    window: session
    source: "Local OpenCode storage and session API"
    notes: "Assistant messages store input, output, reasoning, cache read, and cache write token fields."
  - name: "Stored session cost"
    unit: currency
    window: session
    source: "Local OpenCode storage, stats CLI, and session API"
    notes: "Cost is stored per assistant message/session and aggregated by opencode stats."
  - name: "Context consumption"
    unit: percent
    window: session
    source: "TUI footer source"
    notes: "TUI computes current-message token total divided by model context limit; this is context-window occupancy, not subscription quota."
limit_states:
  - state: cap_approaching
    detectable: false
    source: "native OpenCode inspection"
    markers: []
    notes: "No native on-demand API/CLI/TUI inspection marker for approaching Go/Zen plan caps was found. Community plugin surfaces thresholds, but that is not native OpenCode."
  - state: capped
    detectable: true
    source: "OpenCode Go docs and console support source"
    markers:
      - "rate-limited"
      - "[limited]"
      - "resets in"
    notes: "Internal support formatting distinguishes rate-limited Go rows; ordinary users are officially directed to the console, not a documented endpoint."
  - state: no_funds
    detectable: true
    source: "provider error normalization and observed GitHub issue text"
    markers:
      - "insufficient_quota"
      - "Quota exceeded. Check your plan and billing details."
      - "Free usage exceeded, add credits"
    notes: "This is primarily during-run failure detection, not pre-run inspection."
  - state: auth_required
    detectable: true
    source: "OpenCode CLI docs and local auth files"
    markers:
      - "~/.local/share/opencode/auth.json missing provider entry"
      - "401 Unauthorized"
      - "try running `opencode auth login <your provider URL>`"
    notes: "Auth status can be inferred from provider credentials and normalized API errors; it is not quota headroom."
  - state: unknown
    detectable: false
    source: "native OpenCode inspection"
    markers: []
    notes: "Current ordinary-user reset/headroom state for Go/Zen is unknown outside the hosted console."
docs: https://opencode.ai/docs/go/
changes: []
requires_claudine_update: true
reason: "Claudine can add an OpenCode usage inspector for stored session token/cost data via local JSON/server APIs and opencode stats, but plan quota/headroom must be marked unavailable unless a future OpenCode console/API endpoint is discovered."
---

# OpenCode CLI Usage Inspection

## Introduction to OpenCode CLI Usage Inspection

OpenCode has a mixed usage model. OpenCode Go is a subscription for selected open coding models with dollar-denominated limits: $12 per 5 hours, $30 per week, and $60 per month; the docs say current usage is tracked in the hosted console, and Go can fall back to Zen balance when the user enables **Use balance**. OpenCode Zen is pay-as-you-go balance with optional monthly spend limits for a workspace and individual members. Separately, the OpenCode CLI records historical session token and cost usage locally, and can aggregate that history with `opencode stats`. Native OpenCode inspection therefore splits into two tiers: historical session usage is available locally, but current Go/Zen plan headroom and reset state is not exposed through a documented native CLI or ordinary-user HTTP API.

## API Call Opportunities

| Mechanism | Endpoint | Auth and eligibility | Response fields | Reset-window data | Evidence |
| --- | --- | --- | --- | --- | --- |
| Local/OpenCode server session status | `GET http://<host>:<port>/session/status` | Local server access; remote `opencode serve` can be protected with HTTP Basic auth via `OPENCODE_SERVER_PASSWORD` and username defaults to `opencode`. | Map of session IDs to `SessionStatus`; active/idle/completed state, not usage counters. | None. | Server docs say the TUI talks to a server and the server exposes an OpenAPI 3.1 spec at `/doc` ([server docs](https://opencode.ai/docs/server/)); generated OpenAPI defines `/session/status` as "Get session status" ([source](https://github.com/anomalyco/opencode/blob/a09447bc9bd604f43767fa9ebbe1a6cc0afb8a53/packages/sdk/openapi.json#L5512-L5568)). |
| Local/OpenCode server session read | `GET http://<host>:<port>/session/{sessionID}` | Same server access as above. | Session fields include `cost`, `tokens.input`, `tokens.output`, `tokens.reasoning`, `tokens.cache.read`, `tokens.cache.write`, and timestamps. | None. | Session mapping reads stored token/cost columns into session info ([source](https://github.com/anomalyco/opencode/blob/a09447bc9bd604f43767fa9ebbe1a6cc0afb8a53/packages/opencode/src/session/session.ts#L78-L117)); OpenAPI exposes session get ([source](https://github.com/anomalyco/opencode/blob/a09447bc9bd604f43767fa9ebbe1a6cc0afb8a53/packages/sdk/openapi.json#L5577-L5588)). |
| Hosted OpenCode Go/Zen console | Unknown public endpoint; official surface is the web console. | OpenCode account/session in the hosted console. Ordinary-user API auth is unknown. | Current Go usage and Zen balance/monthly limits are visible in console per docs; exact response schema unknown. | Console has enough state to show current Go usage and enforce rolling/weekly/monthly windows, but no documented API response was found. | Go docs state "You can track your current usage in the console" after listing 5-hour, weekly, and monthly limits ([Go docs](https://opencode.ai/docs/go/)); Zen docs describe balance auto-reload and monthly limits ([Zen docs](https://opencode.ai/docs/zen/)). |
| Proposed OAuth usage API | `GET /auth/usage` | Unknown; proposal says OAuth providers. | Proposed: account status, success/failure counts, cooldown, Anthropic percentages. | Proposed, not established. | GitHub issue #8911 proposes `GET /auth/usage` and `opencode auth usage` ([issue](https://github.com/anomalyco/opencode/issues/8911)); current source probe found no `/auth/usage`, `/usage`, `/billing`, `/subscription`, or `/balance` routes in `packages/sdk/openapi.json` at commit `a09447bc9bd604f43767fa9ebbe1a6cc0afb8a53` (observed on host with `rg`). |

Example local server request:

```sh
opencode serve --port 4096
curl http://localhost:4096/session/status
curl http://localhost:4096/session/ses_...
```

The server/SDK route is useful for stored session usage but does not answer "how much Go/Zen quota remains?" The negative probe is important: the current generated OpenAPI spec has no native `/auth/usage` or `/usage` endpoint, despite those names appearing in feature requests.

## CLI Switch Opportunities

| Invocation | Structured | Non-interactive | What it prints | Evidence |
| --- | --- | --- | --- | --- |
| `opencode stats [--days N] [--project] [--tools N] [--models N]` | No native JSON observed. | Yes. | Styled tables: sessions, messages, days, total cost, average cost/day, average/median tokens per session, input/output/cache tokens, and optional model/tool breakdowns. | Installed OpenCode 1.17.13 help says `opencode stats` "show token usage and cost statistics" (observed on host); source command description matches and aggregates `SessionTable` plus stored message/session tokens and cost ([source](https://github.com/anomalyco/opencode/blob/a09447bc9bd604f43767fa9ebbe1a6cc0afb8a53/packages/opencode/src/cli/cmd/stats.ts#L50-L79)). |
| `opencode session list --format json` | Yes. | Yes. | JSON array containing session IDs and metadata. It does not include token/cost counters directly. | Observed on host: `opencode session list --format json --max-count 2` returned JSON session rows. Source switches between JSON and table output based on `args.format` ([source](https://github.com/anomalyco/opencode/blob/a09447bc9bd604f43767fa9ebbe1a6cc0afb8a53/packages/opencode/src/cli/cmd/session.ts#L91-L118)). |
| `opencode run --format json <prompt>` | Yes. | Yes. | Raw JSON events for a run. Useful after a run, not a pre-run inspection API. | CLI docs document `--format default|json` for `opencode run` ([CLI docs](https://opencode.ai/docs/cli/)); source also describes JSON event support ([source](https://github.com/anomalyco/opencode/blob/a09447bc9bd604f43767fa9ebbe1a6cc0afb8a53/packages/opencode/src/cli/cmd/run.ts#L13-L18)). |
| `opencode auth usage` | No. | Unknown. | Unknown; not present in installed help. | Feature request #8911 proposes it, but installed OpenCode 1.17.13 `opencode --help` and `opencode auth --help` did not expose it (observed on host), and source/OpenAPI probes did not find the endpoint. |

Native CLI inspection can answer historical "what have I spent in stored sessions?" but not "how much plan window remains?" `opencode stats` is the closest native command, and it is human-text only in the installed version.

## Interactive Commands and PTY Scraping

OpenCode advertises `/status` in the TUI footer. Source inspection shows a `DialogStatus` route registered as `opencode.status`, and footer/tips text prompts users to run `/status` for system status ([source](https://github.com/anomalyco/opencode/blob/a09447bc9bd604f43767fa9ebbe1a6cc0afb8a53/packages/tui/src/app.tsx#L761-L765), [source](https://github.com/anomalyco/opencode/blob/a09447bc9bd604f43767fa9ebbe1a6cc0afb8a53/packages/tui/src/routes/session/footer.tsx#L70-L86)). Source inspection did not show native `/usage` or `/quota` commands in OpenCode itself. Community plugin `@slkiser/opencode-quota` adds `/quota`, `/quota_status`, `/tokens_today`, `/tokens_weekly`, a `opencode-quota show --json` command, and TUI quota surfaces, but that is not a native OpenCode mechanism ([plugin README](https://github.com/slkiser/opencode-quota)).

Mini-design for a last-resort `expectrl` scraper:

1. Start `opencode` in a PTY with a known project directory and a clean environment. Wait for the prompt/footer marker, then send `/status` and Enter. If a community quota plugin is intentionally installed, also support `/quota` and `/quota_status`; otherwise treat those commands as unavailable.
2. First pass: match exact known markers from native status and footer text. For native OpenCode, useful exact markers are `/status`, `Input`, `Output`, `Cache Read`, `Cache Write`, currency strings, and context percentages in the footer. Capture nearby numeric fields only when labels match exactly.
3. Second pass: if exact markers fail, fuzzy-search the visible screen buffer for `usage`, `quota`, `limit`, `remaining`, `reset`, `tokens`, `cost`, `context`, and `%`. Use proximity scoring to associate numbers with labels, and mark every field as low-confidence unless both a metric label and unit/window label are present.
4. Emit provenance with every parsed value: command, screen line, parser pass, confidence, and raw snippet.

Caveat: scraped TUI text carries no schema and no stability contract. Unlike JSON from local storage/server APIs, nothing versions or validates the text. PTY scraping should be a last resort for display hints, not a gating source for deciding whether a long Claudine run is safe to start.

## Config and Log Artifacts

| Artifact | Observed on host | Relevant fields | Freshness | Usage-inspection value |
| --- | --- | --- | --- | --- |
| `/Users/ken/.config/opencode/config.json` and `/Users/ken/.config/opencode/opencode.jsonc` | Yes. | Model/provider/plugin configuration. One config contained a provider model `limit.context` and `limit.output`; no plan usage counters. | Updated when user changes config. | Model limits help interpret context-window capacity, not subscription quota. |
| `/Users/ken/.claudine/.config/opencode/opencode.jsonc` | Yes, because this shell's `HOME` is `/Users/ken/.claudine`. | `$schema` only. | Static config. | No usage state. |
| `/Users/ken/.local/share/opencode/auth.json` and `account.json` | Yes. | Provider credential records: `type`, API-key or OAuth token fields, `expires`, provider IDs, account IDs. | Updated by auth/login flows and token refresh. | Auth-required inference only; no usage/quota counters. Secrets must not be copied into reports. |
| `/Users/ken/.local/share/opencode/storage/message/<session>/<message>.json` | Yes. | Assistant messages store `role`, `providerID`, `modelID`, `cost`, `tokens.input`, `tokens.output`, `tokens.reasoning`, `tokens.cache.read`, `tokens.cache.write`, and completion timestamps. | Written as sessions run and finish. | Best local structured artifact for historical token/cost usage. It is per-message/session history, not provider plan headroom. |
| `/Users/ken/.local/share/opencode/storage/session/<project>/<session>.json` | Yes. | Session metadata and timestamps; sampled files did not contain aggregated token/cost values, though source schema supports them in the DB-backed/session model. | Written during session lifecycle. | Useful for session discovery/timestamps; message files carry the observed usage fields. |
| `/Users/ken/.cache/opencode/models.json` | Yes. | Model catalog cache. | Refreshed by model loading or `opencode models --refresh`. | Gives pricing/context metadata, not user usage. |
| `/Users/ken/.opencode/bin/opencode` | Yes. | Installed OpenCode binary, version 1.17.13. | Updated by OpenCode install/upgrade. | Command source for observed help; no state. |

No native local artifact containing current Go/Zen quota percentage, remaining dollars, or reset timestamp was found under `/Users/ken/.config/opencode`, `/Users/ken/.claudine/.config/opencode`, `/Users/ken/.local/share/opencode`, `/Users/ken/.cache/opencode`, or `/Users/ken/.opencode`.

## Metrics and Windows

| Metric | Unit | Window | Mechanisms | Reset expression |
| --- | --- | --- | --- | --- |
| Go rolling usage | Dollars | 5 hours | Hosted console; internal support/admin code. | Support source formats as countdown text such as `now`, `1h 20m`, or `2d`; no ordinary-user API response was found ([source](https://github.com/anomalyco/opencode/blob/a09447bc9bd604f43767fa9ebbe1a6cc0afb8a53/packages/console/support/src/lib/lookup.ts#L391-L407)). |
| Go weekly usage | Dollars | Weekly | Hosted console; internal support/admin code. | Countdown to weekly reset in support source. |
| Go monthly usage | Dollars | Monthly | Hosted console; internal support/admin code. | Countdown/monthly analysis in support source; exact user-facing timestamp format unknown. |
| Zen balance | Dollars | Billing/balance | Hosted console. | Not a reset window; balance decreases and may auto-reload below $5 per docs. |
| Zen monthly limits | Dollars | Monthly | Hosted console. | Unknown from local artifacts; docs say workspace/member monthly limits can be set. |
| Stored message/session cost | Dollars | Session/history | `opencode stats`, local message JSON, session API. | None. It is historical usage, not a reset quota. |
| Stored message/session tokens | Tokens | Session/history | `opencode stats`, local message JSON, session API, TUI footer. | None. |
| Context consumption | Percent | Current message/session | TUI footer. | None; computed against model context limit, not subscription period. |

## Limit States

| State | Distinguishing markers | Mechanisms | Notes |
| --- | --- | --- | --- |
| Cap approaching | Unknown natively. | No native OpenCode CLI/API/TUI inspection marker found. | Community quota plugin can provide thresholds, but native OpenCode does not expose a pre-run warning query. |
| Capped | `rate-limited`, `[limited]`, `resets in ...` in internal support formatting. | Hosted console/support code; no documented ordinary-user API. | Support source formats Go usage rows with a `rate-limited` status and countdown ([source](https://github.com/anomalyco/opencode/blob/a09447bc9bd604f43767fa9ebbe1a6cc0afb8a53/packages/console/support/src/lib/lookup.ts#L391-L394)). |
| No funds / quota exhausted | `insufficient_quota`, "Quota exceeded. Check your plan and billing details.", "Free usage exceeded, add credits". | During-run provider errors; GitHub issue examples. | OpenCode normalizes `insufficient_quota` stream errors to a non-retryable API error ([source](https://github.com/anomalyco/opencode/blob/a09447bc9bd604f43767fa9ebbe1a6cc0afb8a53/packages/opencode/src/provider/error.ts#L117-L123)). |
| Auth required | Missing provider entry in `auth.json`; 401/403 normalized messages; `opencode auth login`. | Local credential files, provider errors, CLI docs. | CLI docs say credentials are stored in `~/.local/share/opencode/auth.json` ([CLI docs](https://opencode.ai/docs/cli/)). |
| Unknown | No native quota record or endpoint. | Local config/cache/log inspection. | Use explicit `unknown` rather than inferring headroom from historical token/cost data. |

## Sources

- [OpenCode Go docs](https://opencode.ai/docs/go/)
- [OpenCode Zen docs](https://opencode.ai/docs/zen/)
- [OpenCode CLI docs](https://opencode.ai/docs/cli/)
- [OpenCode server docs](https://opencode.ai/docs/server/)
- [OpenCode SDK docs](https://opencode.ai/docs/sdk/)
- [OpenCode generated OpenAPI spec in source](https://github.com/anomalyco/opencode/blob/a09447bc9bd604f43767fa9ebbe1a6cc0afb8a53/packages/sdk/openapi.json)
- [OpenCode stats command source](https://github.com/anomalyco/opencode/blob/a09447bc9bd604f43767fa9ebbe1a6cc0afb8a53/packages/opencode/src/cli/cmd/stats.ts)
- [OpenCode session source](https://github.com/anomalyco/opencode/blob/a09447bc9bd604f43767fa9ebbe1a6cc0afb8a53/packages/opencode/src/session/session.ts)
- [OpenCode provider error normalization source](https://github.com/anomalyco/opencode/blob/a09447bc9bd604f43767fa9ebbe1a6cc0afb8a53/packages/opencode/src/provider/error.ts)
- [OpenCode TUI footer source](https://github.com/anomalyco/opencode/blob/a09447bc9bd604f43767fa9ebbe1a6cc0afb8a53/packages/tui/src/routes/session/footer.tsx)
- [OpenCode support lookup source](https://github.com/anomalyco/opencode/blob/a09447bc9bd604f43767fa9ebbe1a6cc0afb8a53/packages/console/support/src/lib/lookup.ts)
- [GitHub issue #8911: proposed OAuth usage API](https://github.com/anomalyco/opencode/issues/8911)
- [GitHub issue #9281: unified usage tracking request](https://github.com/anomalyco/opencode/issues/9281)
- [Community OpenCode Quota plugin](https://github.com/slkiser/opencode-quota)
- Observed on host: `opencode --version` returned `1.17.13`; `opencode stats --help`, `opencode stats --days 1`, and `opencode session list --format json --max-count 2` were run on 2026-07-03.
- Observed on host: local OpenCode state under `/Users/ken/.config/opencode`, `/Users/ken/.claudine/.config/opencode`, `/Users/ken/.local/share/opencode`, `/Users/ken/.cache/opencode`, and `/Users/ken/.opencode` was inspected on 2026-07-03.
