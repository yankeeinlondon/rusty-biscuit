---
$schema: "./_schema.yaml"
created: 2026-07-03
last_updated: 2026-07-03
agent: "codex"
model: "default"
api: true
cli_switch: true
structured_output: true
pty_scrape: true
api_methods:
  - available: true
    endpoint: "GET https://api.kilo.ai/api/profile/balance"
    auth: oauth
    users: "Authenticated Kilo Gateway users; pass X-KILOCODE-ORGANIZATIONID for team balance."
    fields: ["balance"]
    reset_window: "none; current balance only"
    notes: "Used by @kilocode/kilo-gateway fetchBalance; unauthenticated probe returned HTTP 401 with {error:\"Unauthorized\",success:false}."
  - available: true
    endpoint: "GET https://api.kilo.ai/api/trpc/kiloPass.getState?batch=1&input={\"0\":null}"
    auth: oauth
    users: "Authenticated personal Kilo Pass users; source does not document organization applicability."
    fields: ["currentPeriodBaseCreditsUsd", "currentPeriodUsageUsd", "currentPeriodBonusCreditsUsd", "nextBillingAt"]
    reset_window: "nextBillingAt ISO timestamp when present"
    notes: "Used by @kilocode/kilo-gateway fetchKiloPassState; unauthenticated probe returned tRPC UNAUTHORIZED."
  - available: true
    endpoint: "GET http://127.0.0.1:<kilo-server-port>/kilo/profile"
    auth: oauth
    users: "Local Kilo CLI server users authenticated with Kilo Gateway OAuth."
    fields: ["profile.email", "profile.name", "profile.organizations", "balance.balance", "kiloPass.currentPeriodBaseCreditsUsd", "kiloPass.currentPeriodUsageUsd", "kiloPass.currentPeriodBonusCreditsUsd", "kiloPass.nextBillingAt", "currentOrgId"]
    reset_window: "kiloPass.nextBillingAt ISO timestamp when present"
    notes: "Local server endpoint composes profile, balance, Kilo Pass state, and active organization; it returns Unauthorized without OAuth."
cli_methods:
  - available: true
    invocation: "kilo profile --json"
    interactive_only: false
    output_format: "JSON"
    fields: ["name", "email", "team", "organizationId", "balance"]
    notes: "Requires Kilo Gateway OAuth auth. On this host with HOME=/Users/ken it exited 1 and printed 'Not authenticated with Kilo Gateway'."
  - available: true
    invocation: "kilo profile"
    interactive_only: false
    output_format: "human text"
    fields: ["Name", "Email", "Team", "Balance"]
    notes: "Same data as --json, formatted for humans."
  - available: true
    invocation: "kilo stats --days <N> --models [N] --tools [N] --project <id>"
    interactive_only: false
    output_format: "human text"
    fields: ["Sessions", "Messages", "Days", "Total Cost", "Avg Cost/Day", "Avg Tokens/Session", "Median Tokens/Session", "Input", "Output", "Cache Read", "Cache Write", "model messages", "model cost", "tool usage"]
    notes: "Reads local session history, not Kilo Gateway quota. No JSON flag is exposed by help text."
  - available: true
    invocation: "/profile"
    interactive_only: true
    output_format: "TUI dialog"
    fields: ["Name", "Email", "Team", "Balance", "Usage Details URL"]
    notes: "Interactive slash command, aliases /me and /whoami, visible only when connected to Kilo Gateway."
  - available: true
    invocation: "/teams"
    interactive_only: true
    output_format: "TUI selector"
    fields: ["organizations", "currentOrgId"]
    notes: "Does not report usage directly, but selects the personal/team credit pool whose balance is later inspected."
pty_design:
  command: "kilo"
  first_pass_markers: ["Kilo Gateway Profile", "Name:", "Email:", "Team:", "Balance:", "Usage Details:"]
  fuzzy_markers: ["balance", "credits", "kilo pass", "renews", "usage", "team", "personal"]
  fields: ["account_scope", "balance_usd", "usage_url", "kilo_pass_usage_usd", "kilo_pass_base_credits_usd", "kilo_pass_renews"]
  risks: "TUI text has no schema or stability contract; use only after API and CLI JSON fail, and tolerate layout, wording, and color-control drift."
metrics:
  - name: "Gateway balance"
    unit: currency
    window: billing_cycle
    source: "GET /api/profile/balance; kilo profile --json; /profile"
    notes: "Current remaining credit balance, not a short-window counter."
  - name: "Kilo Pass current period usage"
    unit: currency
    window: billing_cycle
    source: "GET /api/trpc/kiloPass.getState; local /kilo/profile; sidebar footer"
    notes: "currentPeriodUsageUsd against currentPeriodBaseCreditsUsd; reset at nextBillingAt when present."
  - name: "Kilo Pass current period base credits"
    unit: currency
    window: billing_cycle
    source: "GET /api/trpc/kiloPass.getState; local /kilo/profile; sidebar footer"
    notes: "Base monthly or subscription-period credit allotment; bonus credits are separate."
  - name: "Free model request limit"
    unit: requests
    window: hourly
    source: "Kilo usage and billing docs"
    notes: "200 free-model requests per hour per IP; no inspected endpoint found for remaining headroom or reset timestamp."
  - name: "Organization per-user daily spend"
    unit: currency
    window: daily
    source: "Kilo usage and billing docs"
    notes: "Daily organization member cap resets at midnight UTC; no inspected endpoint found for the configured cap or current spend."
  - name: "Local session cost"
    unit: currency
    window: other
    source: "kilo stats; /Users/ken/.local/share/kilo/kilo.db session.cost"
    notes: "Retrospective local history, filtered by --days/project; not authoritative for live gateway balance."
  - name: "Local session tokens"
    unit: tokens
    window: other
    source: "kilo stats; /Users/ken/.local/share/kilo/kilo.db session token columns"
    notes: "Input, output, reasoning, cache read, and cache write token totals from local sessions."
limit_states:
  - state: capped
    detectable: true
    source: "Gateway request error"
    markers: ["HTTP 402", "Insufficient balance. Please add credits to continue.", "metadata.buyCreditsUrl"]
    notes: "Docs describe this for zero balance on paid models; the same balance-style failure is used for org daily spend caps."
  - state: no_funds
    detectable: true
    source: "Gateway request error"
    markers: ["HTTP 402", "Insufficient balance", "buyCreditsUrl"]
    notes: "Kilo uses credit balance rather than a separate no-funds code in the inspected docs."
  - state: capped
    detectable: true
    source: "Organization daily spend limit"
    markers: ["balance error", "daily limit", "resets at midnight UTC"]
    notes: "Docs say subsequent requests return a balance error after the per-user daily limit is reached."
  - state: capped
    detectable: true
    source: "Free model rate limit"
    markers: ["HTTP 429", "Rate limit exceeded for free models. Please try again later."]
    notes: "Applies to free models, by IP, 200 requests per hour."
  - state: auth_required
    detectable: true
    source: "kilo profile --json; public endpoint probes"
    markers: ["Not authenticated with Kilo Gateway", "HTTP 401", "Unauthorized", "UNAUTHORIZED"]
    notes: "Observed on this host for CLI profile and unauthenticated /api/profile, /api/profile/balance, and kiloPass probes."
  - state: cap_approaching
    detectable: false
    source: "unknown"
    markers: []
    notes: "No on-demand inspection endpoint or CLI command was found that returns an approaching-cap warning."
docs: "https://kilo.ai/docs/gateway/usage-and-billing"
changes: []
requires_claudine_update: true
reason: "Claudine can add a Kilo usage inspector using kilo profile --json plus optional direct OAuth endpoint support; Kilo is not yet code-supported in Claudine's provider enum."
---

# Kilo Code Usage Inspection

## Introduction to Kilo Code Usage Inspection

Kilo Code uses a credit-balance model through the Kilo AI Gateway, not a Claude-style short-window and long-window subscription cap model. The official billing flow is balance check, request execution, usage tracking, and balance update; costs are based on provider token usage, with free models and BYOK requests tracked at $0 on Kilo's side ([Usage & Billing](https://kilo.ai/docs/gateway/usage-and-billing)). The visible windows found in this research are: hourly free-model request limiting, organization per-user daily spending limits, Kilo Pass subscription-period usage, and local session-history windows selected by `kilo stats --days`.

Kilo officially surfaces usage in the web dashboard, the CLI profile/stats commands, the interactive `/profile` command, and the TUI sidebar wallet. The most useful non-interactive mechanism for Claudine is `kilo profile --json`, which reports current balance as structured JSON. The richer Kilo Pass period fields are available through the local server `/kilo/profile` response and the underlying Kilo Pass tRPC call, but not through the current `kilo profile --json` payload.

## API Call Opportunities

| Mechanism | Endpoint | Auth | Users | Fields | Reset Window |
|---|---|---|---|---|---|
| Public balance API | `GET https://api.kilo.ai/api/profile/balance` | Bearer OAuth token | Authenticated Kilo Gateway users; add `X-KILOCODE-ORGANIZATIONID` for a team balance | `balance` | none |
| Public Kilo Pass API | `GET https://api.kilo.ai/api/trpc/kiloPass.getState?batch=1&input={"0":null}` | Bearer OAuth token | Authenticated Kilo Pass users | `currentPeriodBaseCreditsUsd`, `currentPeriodUsageUsd`, `currentPeriodBonusCreditsUsd`, `nextBillingAt` | `nextBillingAt` ISO timestamp |
| Local Kilo server | `GET http://127.0.0.1:<port>/kilo/profile` | local server auth plus Kilo Gateway OAuth | Authenticated local CLI/server users | `profile`, `balance`, `kiloPass`, `currentOrgId` | `kiloPass.nextBillingAt` ISO timestamp |

Example public balance request:

```bash
curl -H "Authorization: Bearer $KILO_OAUTH_TOKEN" \
  https://api.kilo.ai/api/profile/balance
```

Expected response shape from source:

```json
{
  "balance": 12.34
}
```

`fetchBalance` in `packages/kilo-gateway/src/api/profile.ts` sends the bearer token and optional `x-kilocode-organizationid` header to `/api/profile/balance`, then maps the response into `{ "balance": number }` ([source](https://github.com/Kilo-Org/kilocode/blob/main/packages/kilo-gateway/src/api/profile.ts)). An unauthenticated probe run on 2026-07-03 returned HTTP 401 with `{"error":"Unauthorized","success":false}`.

Example Kilo Pass request:

```bash
curl -H "Authorization: Bearer $KILO_OAUTH_TOKEN" \
  'https://api.kilo.ai/api/trpc/kiloPass.getState?batch=1&input=%7B%220%22%3Anull%7D'
```

Relevant parsed response fields:

```json
{
  "currentPeriodBaseCreditsUsd": 20,
  "currentPeriodUsageUsd": 7.5,
  "currentPeriodBonusCreditsUsd": 0,
  "nextBillingAt": "2026-08-01T00:00:00.000Z"
}
```

`fetchKiloPassState` calls the tRPC endpoint and `parseKiloPassState` extracts subscription period fields from the response wrapper ([source](https://github.com/Kilo-Org/kilocode/blob/main/packages/kilo-gateway/src/api/kilo-pass.ts)). An unauthenticated probe returned HTTP 401 with a tRPC `UNAUTHORIZED` error for `kiloPass.getState`.

Example local-server request:

```bash
curl http://127.0.0.1:4096/kilo/profile
```

The local server endpoint returns:

```json
{
  "profile": {
    "email": "user@example.com",
    "name": "User",
    "organizations": []
  },
  "balance": {
    "balance": 12.34
  },
  "kiloPass": {
    "currentPeriodBaseCreditsUsd": 20,
    "currentPeriodUsageUsd": 7.5,
    "currentPeriodBonusCreditsUsd": 0,
    "nextBillingAt": "2026-08-01T00:00:00.000Z"
  },
  "currentOrgId": null
}
```

The `/kilo/profile` server handler requires local Kilo auth of type `oauth`, then fetches profile, balance, and Kilo Pass state in parallel ([handler source](https://github.com/Kilo-Org/kilocode/blob/main/packages/opencode/src/kilocode/server/httpapi/handlers/kilo-gateway.ts), [schema source](https://github.com/Kilo-Org/kilocode/blob/main/packages/opencode/src/kilocode/server/httpapi/groups/kilo-gateway.ts)). This is a local server API, not a documented public cloud endpoint.

Unknowns:

- Whether `https://api.kilo.ai/api/profile/balance` accepts a Kilo Gateway API key as a bearer token is unknown. The CLI source path uses OAuth tokens for account profile and balance, and `kilo profile --json` rejects non-OAuth local auth.
- No endpoint was found that returns remaining free-model hourly requests, the free-window reset timestamp, organization daily spend used, organization daily cap amount, or a cap-approaching state.

## CLI Switch Opportunities

| Invocation | Non-Interactive | Structured | Output |
|---|---:|---:|---|
| `kilo profile --json` | yes | yes | JSON object with `name`, `email`, `team`, `organizationId`, `balance` |
| `kilo profile` | yes | no | text lines: `Name`, `Email`, `Team`, `Balance` |
| `kilo stats --days <N> --models [N] --tools [N] --project <id>` | yes | no | boxed terminal text with sessions, messages, local cost, token totals, model usage, and tool usage |

`kilo profile --json` is the best CLI path for Claudine because it is non-interactive and machine-parseable. Its source fetches OAuth auth for provider id `kilo`, exits with `Not authenticated with Kilo Gateway` if missing or non-OAuth, then prints `JSON.stringify(info, null, 2)` when `--json` is set ([source](https://github.com/Kilo-Org/kilocode/blob/main/packages/opencode/src/kilocode/cli/cmd/profile.ts)). On this host, `HOME=/Users/ken kilo profile --json` exited 1 with `Not authenticated with Kilo Gateway` because `/Users/ken/.local/share/kilo/auth.json` contains `opencode` and `openrouter` API-key auth records but no `kilo` OAuth record.

`kilo stats` is useful for retrospective local spend, not live quota. The help text observed on this host exposes `--days`, `--tools`, `--models`, and `--project`, but no `--json`. Source aggregation reads the local session database and reports `totalCost`, token buckets, model usage, and tool usage ([source](https://github.com/Kilo-Org/kilocode/blob/main/packages/opencode/src/cli/cmd/stats.ts)). On this host, `HOME=/Users/ken kilo stats --days 7 --models 5 --tools 5` printed zero sessions and zero cost.

The Kilo CLI docs list `/profile` and `/teams` as Kilo Gateway slash commands and document that `/teams` selects an organization for interactive use; non-interactive `kilo run` has no `--org` or `--team` flag and instead uses `KILO_ORG_ID` or the persisted `/teams` selection ([CLI docs](https://kilo.ai/docs/code-with-ai/platforms/cli)).

## Interactive Commands and PTY Scraping

Interactive slash command opportunities:

| Command | Aliases | Display |
|---|---|---|
| `/profile` | `/me`, `/whoami` | Kilo Gateway Profile dialog with name, email, team, balance, and a usage details URL |
| `/teams` | `/team`, `/org`, `/orgs` | team selector; changes which personal or organization credit pool is active |

The `/profile` implementation calls the local server `kilo.profile()` endpoint and renders `DialogKiloProfile`; the dialog displays `Kilo Gateway Profile`, `Name`, `Email`, `Team`, `Balance`, and a usage details link. The usage URL is `https://app.kilo.ai/usage` for personal accounts or `https://app.kilo.ai/organizations/<org-id>/usage-details` for organizations ([command source](https://github.com/Kilo-Org/kilocode/blob/main/packages/opencode/src/kilocode/kilo-commands.tsx), [dialog source](https://github.com/Kilo-Org/kilocode/blob/main/packages/opencode/src/kilocode/components/dialog-kilo-profile.tsx)).

PTY scraping design with `expectrl`:

1. Start `kilo` under a PTY, wait for the TUI input surface, send `/profile\r`, and match exact first-pass markers: `Kilo Gateway Profile`, `Name:`, `Email:`, `Team:`, `Balance:`, `Usage Details:`. Extract `team`, `balance_usd`, and `usage_url`. If the sidebar footer is visible, also match `Balance`, `Personal credits` or `<name> team`, `Kilo Pass`, and `Renews`.
2. If the exact pass fails, capture the screen buffer and fuzzy-search case-insensitive markers: `balance`, `credits`, `kilo pass`, `renews`, `usage`, `team`, `personal`. Parse nearby currency values, ISO-ish dates or short UTC renewal labels, and URLs. Require confidence from at least two markers before returning data.

Scraped TUI text has no schema and no stability contract. It is not versioned, validated, or guaranteed to remain stable, so it is strictly a last resort after `kilo profile --json`, direct API calls, and local-server JSON are unavailable.

## Config and Log Artifacts

Observed host artifacts:

| Path | Evidence | Freshness | Usage/Limit Value |
|---|---|---|---|
| `/Users/ken/.kilo` | no directory or files found | none | none |
| `/Users/ken/.kilocode` | plugin/node_modules-style directory only in inspected depth | stale or install-time | no usage/quota fields observed |
| `/Users/ken/.config/kilo/kilo.jsonc` | contains only `$schema: https://app.kilo.ai/config.json` | config-time; modified 2026-06-13 | no usage/quota fields |
| `/Users/ken/.local/share/kilo/auth.json` | contains `opencode` and `openrouter` API-key records, keys redacted during inspection | auth-change-time; modified 2026-04-14 | no Kilo OAuth account, no usage/quota fields |
| `/Users/ken/.local/share/kilo/kilo.db` | SQLite tables include `session`, `message`, `part`, `account`, `account_state` | live local state; modified 2026-07-03 | session table has `cost` and token columns; observed totals: 2 sessions, 4 messages, 5 parts, cost 0, all token totals 0 |
| `/Users/ken/.local/share/kilo/log/*.log` | CLI run logs | per process | no balance, quota, HTTP 402, HTTP 429, or auth-required markers found in targeted search |
| `/Users/ken/.cache/kilo/models.json` | large provider/model cache | refreshed 2026-07-03 | model pricing and context/output limits, not current consumption |

The local database can support `kilo stats` and retrospective spend analysis, but it is not an authoritative current-quota source. It can be stale, incomplete, project-filtered, or zero even when gateway balance exists.

## Metrics and Windows

| Metric | Unit | Window | Mechanism | Reset Expression |
|---|---|---|---|---|
| Current balance | USD currency | account balance / billing cycle | `/api/profile/balance`, local `/kilo/profile`, `kilo profile --json`, `/profile` | no reset; remaining credit balance |
| Kilo Pass usage | USD currency | subscription period | `/api/trpc/kiloPass.getState`, local `/kilo/profile`, TUI sidebar | `nextBillingAt` ISO timestamp; sidebar formats month/day in UTC |
| Kilo Pass base credits | USD currency | subscription period | `/api/trpc/kiloPass.getState`, local `/kilo/profile`, TUI sidebar | same as Kilo Pass usage |
| Kilo Pass bonus credits | USD currency | subscription period | `/api/trpc/kiloPass.getState`, local `/kilo/profile`, TUI sidebar | same as Kilo Pass usage |
| Free-model rate limit | requests | hourly | docs and gateway errors | no inspected reset timestamp; docs state 200 requests per hour per IP |
| Organization per-user daily spending limit | USD currency | daily | docs and gateway balance errors | midnight UTC |
| Local session cost | USD currency | selected local history range | `kilo stats --days`; SQLite `session.cost` | chosen by `--days`; local timestamps |
| Local session tokens | tokens | selected local history range | `kilo stats --days`; SQLite token columns | chosen by `--days`; local timestamps |

The official docs say usage data is tracked per request with fields including `model`, `provider`, `input_tokens`, `output_tokens`, `cache_write_tokens`, `cache_hit_tokens`, `cost_microdollars`, `time_to_first_token`, and `is_byok`; non-streaming responses include a `usage` field and streaming responses include usage in the final SSE chunk before `[DONE]` ([Usage & Billing](https://kilo.ai/docs/gateway/usage-and-billing)). That request-level stream/body data belongs primarily to the sibling non-interactive-sessions topic unless persisted locally for later inspection.

## Limit States

| State | Mechanism | Markers | Distinguishes |
|---|---|---|---|
| Out of funds / zero balance | Gateway request | HTTP 402, `Insufficient balance. Please add credits to continue.`, `metadata.buyCreditsUrl` | paid model cannot run until credits are added |
| Organization daily cap reached | Gateway request | balance error after daily limit, reset at midnight UTC | per-user org spend cap, not necessarily global no-funds |
| Free-model hourly rate limit | Gateway request | HTTP 429, `Rate limit exceeded for free models. Please try again later.` | free model IP rate limit |
| Auth required | CLI/API | `Not authenticated with Kilo Gateway`, HTTP 401, `Unauthorized`, tRPC `UNAUTHORIZED` | missing or invalid auth |
| Cap approaching | unknown | none found | no on-demand inspection marker found |

The docs state paid model requests are not gateway-rate-limited, although upstream provider rate limits and organization per-user daily spending limits can still apply ([Cost Efficiency & Model Selection](https://kilo.ai/docs/getting-started/rate-limits-and-costs)).

## Sources

- [Kilo Usage & Billing](https://kilo.ai/docs/gateway/usage-and-billing)
- [Kilo Cost Efficiency & Model Selection](https://kilo.ai/docs/getting-started/rate-limits-and-costs)
- [Kilo CLI docs](https://kilo.ai/docs/code-with-ai/platforms/cli)
- [Kilo Setup & Authentication](https://kilo.ai/docs/getting-started/setup-authentication)
- [Kilo Credits and Billing FAQ](https://kilo.ai/docs/getting-started/faq/credits-and-billing)
- [Kilo Gateway profile and balance source](https://github.com/Kilo-Org/kilocode/blob/main/packages/kilo-gateway/src/api/profile.ts)
- [Kilo Gateway Kilo Pass source](https://github.com/Kilo-Org/kilocode/blob/main/packages/kilo-gateway/src/api/kilo-pass.ts)
- [Kilo local server profile handler](https://github.com/Kilo-Org/kilocode/blob/main/packages/opencode/src/kilocode/server/httpapi/handlers/kilo-gateway.ts)
- [Kilo local server profile schema](https://github.com/Kilo-Org/kilocode/blob/main/packages/opencode/src/kilocode/server/httpapi/groups/kilo-gateway.ts)
- [Kilo CLI profile command source](https://github.com/Kilo-Org/kilocode/blob/main/packages/opencode/src/kilocode/cli/cmd/profile.ts)
- [Kilo CLI stats command source](https://github.com/Kilo-Org/kilocode/blob/main/packages/opencode/src/cli/cmd/stats.ts)
- [Kilo interactive profile command source](https://github.com/Kilo-Org/kilocode/blob/main/packages/opencode/src/kilocode/kilo-commands.tsx)
- [Kilo profile dialog source](https://github.com/Kilo-Org/kilocode/blob/main/packages/opencode/src/kilocode/components/dialog-kilo-profile.tsx)
- [Kilo sidebar balance source](https://github.com/Kilo-Org/kilocode/blob/main/packages/opencode/src/kilocode/plugins/sidebar-footer.tsx)
- Observed on host: `/Users/ken/.local/share/kilo/auth.json`, `/Users/ken/.local/share/kilo/kilo.db`, `/Users/ken/.local/share/kilo/log/`, `/Users/ken/.cache/kilo/models.json`, `/Users/ken/.config/kilo/kilo.jsonc`, and negative probes to `https://api.kilo.ai/api/profile`, `https://api.kilo.ai/api/profile/balance`, and `https://api.kilo.ai/api/trpc/kiloPass.getState`.
