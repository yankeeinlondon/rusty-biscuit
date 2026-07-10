---
$schema: ./_schema.yaml
created: 2026-07-08
last_updated: 2026-07-08
agent: codex
model: default
api: true
cli_switch: false
structured_output: true
pty_scrape: true
api_methods:
  - available: true
    endpoint: "POST https://127.0.0.1:<connect-port>/exa.language_server_pb.LanguageServerService/GetUserStatus"
    auth: session_cookie
    users: "Users with a running Antigravity IDE or Antigravity CLI language server; the caller must recover the local CSRF token from the running language-server process."
    fields:
      - "userStatus.email"
      - "userStatus.isAuthenticated"
      - "userStatus.planStatus.availablePromptCredits"
      - "userStatus.planStatus.planInfo.monthlyPromptCredits"
      - "userStatus.cascadeModelConfigData.clientModelConfigs[].modelOrAlias.model"
      - "userStatus.cascadeModelConfigData.clientModelConfigs[].quotaInfo.remainingFraction"
      - "userStatus.cascadeModelConfigData.clientModelConfigs[].quotaInfo.resetTime"
    reset_window: "Per-model resetTime values, expressed as timestamps; window length is not explicitly named."
    notes: "Undocumented local Connect RPC endpoint. Observed in antigravity-usage source and compatible with local Antigravity server design; requires Connect-Protocol-Version: 1 and X-Codeium-Csrf-Token."
  - available: true
    endpoint: "POST https://daily-cloudcode-pa.googleapis.com/v1internal:loadCodeAssist"
    auth: oauth
    users: "Consumer OAuth users and enterprise users after Antigravity login; observed on host logs with authMethod=consumer."
    fields:
      - "codeAssistEnabled"
      - "planInfo.monthlyPromptCredits"
      - "planInfo.planType"
      - "availablePromptCredits"
      - "cloudaicompanionProject"
      - "currentTier"
      - "paidTier"
      - "allowedTiers"
    reset_window: "No reset timestamp in the observed/reverse-engineered parser; yields billing-cycle-style prompt-credit counts."
    notes: "Internal Cloud Code endpoint used by Antigravity CLI on this host; antigravity-usage uses production cloudcode-pa.googleapis.com while observed CLI logs used daily-cloudcode-pa.googleapis.com."
  - available: true
    endpoint: "POST https://daily-cloudcode-pa.googleapis.com/v1internal:fetchAvailableModels"
    auth: oauth
    users: "Authenticated Antigravity users with a Cloud Code companion project; model fetch may return 403 for some accounts or permissions."
    fields:
      - "models.<model_id>.displayName"
      - "models.<model_id>.label"
      - "models.<model_id>.quotaInfo.remainingFraction"
      - "models.<model_id>.quotaInfo.resetTime"
      - "models.<model_id>.quotaInfo.isExhausted"
      - "defaultAgentModelId"
    reset_window: "Per-model resetTime values, expressed as timestamps; window length is inferred from plan/quota policy, not carried as a stable field."
    notes: "Internal model/quota endpoint observed in Antigravity CLI logs and antigravity-usage source. The third-party client treats 403 as possible and continues with prompt credits only."
cli_methods:
  - available: false
    invocation: "agy quota --json"
    interactive_only: false
    output_format: "unknown"
    fields: []
    notes: "No official Antigravity CLI non-interactive quota/status subcommand or JSON switch was found in official README/changelog or on-host CLI artifacts."
  - available: true
    invocation: "/usage"
    interactive_only: true
    output_format: "styled TUI text"
    fields:
      - "model quota usage"
      - "remaining quotas"
      - "real-time consumption statistics"
      - "reset countdowns, based on public reports and quota-page redesign"
    notes: "Official changelog says /usage and /quota force real-time reload of model configuration and remaining quotas; later changelog says the Models & Quota page replaced the legacy usage page."
  - available: true
    invocation: "/quota"
    interactive_only: true
    output_format: "styled TUI text"
    fields:
      - "model quota usage"
      - "remaining quotas"
      - "real-time consumption statistics"
    notes: "Official changelog documents /quota alongside /usage. No stable schema or non-TTY form was found."
  - available: true
    invocation: "status line"
    interactive_only: true
    output_format: "styled TUI text"
    fields:
      - "quota usage"
      - "execution mode"
    notes: "Official changelog for 1.0.8 says quota usage and execution mode are displayed in the status line."
pty_design:
  command: "expectrl spawn of `agy`, then send `/usage` first and `/quota` as fallback"
  first_pass_markers:
    - "Models & Quota"
    - "Remaining"
    - "Resets In"
    - "%"
    - "EXHAUSTED"
  fuzzy_markers:
    - "quota"
    - "usage"
    - "remaining"
    - "reset"
    - "credits"
    - "exhausted"
    - "disabled"
  fields:
    - "model label"
    - "model id when visible"
    - "remaining percentage"
    - "reset timestamp or countdown"
    - "disabled/exhausted state"
    - "plan tier when visible"
    - "prompt credits when visible"
  risks: "The slash-command display is a TUI surface with no schema, no versioned contract, no stable field names, and no guarantee that text remains visible or parseable across releases."
metrics:
  - name: "model_remaining_fraction"
    unit: percent
    window: unknown
    source: "local GetUserStatus and Cloud Code fetchAvailableModels"
    notes: "Field is quotaInfo.remainingFraction; displayed as remaining percentage. Reset timestamp exists, but the response does not name the window length."
  - name: "model_reset_time"
    unit: time
    window: unknown
    source: "local GetUserStatus and Cloud Code fetchAvailableModels"
    notes: "Field is quotaInfo.resetTime; parsers convert it to milliseconds until reset."
  - name: "prompt_credits_available"
    unit: credits
    window: monthly
    source: "Cloud Code loadCodeAssist and local GetUserStatus planStatus"
    notes: "availablePromptCredits and planInfo.monthlyPromptCredits produce available, monthly, used percentage, and remaining percentage."
  - name: "status_line_quota_usage"
    unit: percent
    window: unknown
    source: "Antigravity CLI 1.0.8 changelog"
    notes: "Interactive status-line text only; exact fields are not structured."
limit_states:
  - state: auth_required
    detectable: true
    source: "local Antigravity CLI logs and Cloud Code client error handling"
    markers:
      - "You are not logged into Antigravity"
      - "failed to get load code assist response: error getting token source"
      - "HTTP 401"
      - "HTTP 403"
    notes: "Observed in local logs before silent keyring auth completed; third-party Cloud Code client maps 401/403 to an authentication error."
  - state: capped
    detectable: true
    source: "local/Cloud Code model quota fields and interactive display"
    markers:
      - "quotaInfo.isExhausted=true"
      - "quotaInfo.remainingFraction=0"
      - "isExhausted=true"
      - "EXHAUSTED"
    notes: "Structured fields can distinguish exhausted model quota when available; TUI text can only be scraped."
  - state: cap_approaching
    detectable: true
    source: "local/Cloud Code model quota fields"
    markers:
      - "quotaInfo.remainingFraction below Claudine threshold"
      - "remaining percentage in low range"
    notes: "No provider-specified warning threshold was found for inspection; an inspecting tool must choose its own threshold."
  - state: no_funds
    detectable: true
    source: "G1 credits support and prompt-credit fields"
    markers:
      - "availablePromptCredits=0"
      - "planStatus.availablePromptCredits=0"
      - "G1 credits unavailable or disabled"
    notes: "The CLI changelog documents G1 credits and a /credits panel, but no stable no-funds error schema was found in inspection surfaces."
  - state: unknown
    detectable: false
    source: "official documentation"
    markers: []
    notes: "Google does not publish a stable usage-inspection schema for Antigravity CLI as of this research."
docs: https://antigravity.google/docs/cli/credits
changes: []
requires_claudine_update: true
reason: "Claudine will need a new Antigravity usage adapter that can query undocumented JSON surfaces and fall back to slash-command PTY scraping when the local/API routes are unavailable."
---

# Antigravity Usage Inspection

## Introduction to Antigravity Usage Inspection

Antigravity uses a mixed plan and quota model. Official Antigravity documentation pages exist for [AI Credits](https://antigravity.google/docs/cli/credits) and [Plans](https://antigravity.google/docs/plans), and the public docs/search index describes AI Premium credits, usage quotas, five-hour refresh behavior for paid CLI users, and weekly rate limits for free users. The official CLI repository says Antigravity CLI authenticates through the system keyring and Google Sign-In, with enterprise users connecting a GCP project during onboarding ([google-antigravity/antigravity-cli README](https://github.com/google-antigravity/antigravity-cli)).

Google does not currently publish a stable, documented usage-inspection API or a non-interactive `agy` usage command. The practical inspection surfaces are internal: Cloud Code `v1internal` endpoints, a local Connect RPC service exposed by a running Antigravity language server, interactive `/usage` and `/quota` TUI pages, and logs/config files under `/Users/ken/.gemini/antigravity-cli`. The requested `/Users/ken/.antigravity` directory exists on this host, but it contains app shell state and extensions, not observed quota state.

## API Call Opportunities

### Local Connect RPC: `GetUserStatus`

When Antigravity is running, a local language-server process exposes Connect RPC endpoints on loopback. The reverse-engineered working request is:

```http
POST https://127.0.0.1:<connect-port>/exa.language_server_pb.LanguageServerService/GetUserStatus
Content-Type: application/json
Accept: application/json
Connect-Protocol-Version: 1
X-Codeium-Csrf-Token: <csrf-token>

{
  "metadata": {
    "ideName": "antigravity",
    "extensionName": "antigravity",
    "locale": "en"
  }
}
```

The third-party `antigravity-usage` project documents that REST-looking probes such as `/quota`, `/status`, `/api/v1/user/status`, and `/v1/user/status` are wrong and produce failures such as `403 Invalid CSRF token`; it identifies the correct endpoint as `LanguageServerService/GetUserStatus` with `X-Codeium-Csrf-Token` and `Connect-Protocol-Version: 1` ([solution.md](https://github.com/skainguyen1412/antigravity-usage/blob/main/docs/solution.md)). Its source extracts `planStatus.availablePromptCredits`, `planStatus.planInfo.monthlyPromptCredits`, and `cascadeModelConfigData.clientModelConfigs[].quotaInfo.remainingFraction/resetTime` from that response ([connect-client.ts](https://github.com/skainguyen1412/antigravity-usage/blob/main/src/local/connect-client.ts)).

Auth is local session/process auth, not an API key. The caller needs the running process port and CSRF token. The same third-party implementation probes HTTPS first and falls back to HTTP on the extension server port if TLS/protocol detection fails ([service.ts](https://github.com/skainguyen1412/antigravity-usage/blob/main/src/quota/service.ts)).

### Cloud Code Internal Endpoints

Local Antigravity CLI logs on this host show calls to:

```http
POST https://daily-cloudcode-pa.googleapis.com/v1internal:loadCodeAssist
Authorization: Bearer <oauth-token>
Content-Type: application/json

{
  "metadata": {
    "ideType": "ANTIGRAVITY",
    "platform": "PLATFORM_UNSPECIFIED",
    "pluginType": "GEMINI"
  }
}
```

Observed-on-host evidence:

- `/Users/ken/.gemini/antigravity-cli/log/cli-20260708_113052.log` records `https://daily-cloudcode-pa.googleapis.com/v1internal:loadCodeAssist` and then `...:fetchAvailableModels`.
- `/Users/ken/.gemini/antigravity-cli/log/cli-20260708_114233.log` records `quota_manager.go:72] quotaRefreshLoop: starting reload (force=true)` after those calls.
- `/Users/ken/.gemini/antigravity-cli/log/cli-20260708_114540.log` records the same endpoint pair in non-interactive print mode.

The reverse-engineered Cloud Code client in `antigravity-usage` uses `https://cloudcode-pa.googleapis.com/v1internal:loadCodeAssist` with an OAuth bearer token and parses `planInfo.monthlyPromptCredits`, `planInfo.planType`, `availablePromptCredits`, tier fields, and `cloudaicompanionProject` ([cloudcode.ts](https://github.com/skainguyen1412/antigravity-usage/blob/main/src/google/cloudcode.ts), [parser.ts](https://github.com/skainguyen1412/antigravity-usage/blob/main/src/google/parser.ts)). The endpoint accepts OAuth/session bearer auth, not a standalone API key.

For model quotas, Antigravity calls:

```http
POST https://daily-cloudcode-pa.googleapis.com/v1internal:fetchAvailableModels
Authorization: Bearer <oauth-token>
Content-Type: application/json

{
  "project": "<cloudaicompanion-project-id>"
}
```

The parsed response fields are `models.<model_id>.displayName`, `label`, `quotaInfo.remainingFraction`, `quotaInfo.resetTime`, `quotaInfo.isExhausted`, and `defaultAgentModelId` ([cloudcode.ts](https://github.com/skainguyen1412/antigravity-usage/blob/main/src/google/cloudcode.ts), [parser.ts](https://github.com/skainguyen1412/antigravity-usage/blob/main/src/google/parser.ts)). The `antigravity-usage` service treats `fetchAvailableModels` failures as possible permission failures and continues with prompt credits only, which is useful negative evidence for Claudine: a 403 on model quota does not necessarily mean `loadCodeAssist` cannot return plan credit data ([service.ts](https://github.com/skainguyen1412/antigravity-usage/blob/main/src/quota/service.ts)).

## CLI Switch Opportunities

No official Antigravity CLI subcommand or switch was found that prints quota in non-interactive mode. The public README only documents starting the CLI with `agy` and authenticating/signing out via `/logout` ([README](https://github.com/google-antigravity/antigravity-cli)). The changelog documents a `models` subcommand in 1.0.5, but not a non-interactive usage or quota subcommand ([CHANGELOG.md](https://github.com/google-antigravity/antigravity-cli/blob/main/CHANGELOG.md)).

The best structured CLI path found is not Google’s CLI: `antigravity-usage quota --json` prints a JSON `QuotaSnapshot` with `timestamp`, `method`, `email`, `planType`, `promptCredits`, and `models` ([README](https://github.com/skainguyen1412/antigravity-usage), [types.ts](https://github.com/skainguyen1412/antigravity-usage/blob/main/src/quota/types.ts)). Because this is a third-party tool, it should be treated as an implementation reference or optional helper, not as an Antigravity provider contract.

## Interactive Commands and PTY Scraping

Antigravity CLI has interactive quota commands. The official changelog for 1.0.1 says `/usage` and `/quota` were improved to force a real-time reload of model configuration and remaining quotas, allowing updated real-time consumption statistics to be seen immediately ([CHANGELOG.md](https://github.com/google-antigravity/antigravity-cli/blob/main/CHANGELOG.md)). The 1.0.8 changelog says the “Models & Quota” page replaced the legacy usage page, disabled quota buckets are displayed as `Disabled`, and quota usage plus execution mode appear in the status line ([CHANGELOG.md](https://github.com/google-antigravity/antigravity-cli/blob/main/CHANGELOG.md)).

A Claudine PTY scraper should be a last resort:

1. Spawn `agy` with `expectrl`, wait for the input prompt, send `/usage`, and fall back to `/quota` if `/usage` is unavailable. The first pass should match exact current markers: `Models & Quota`, `Remaining`, `Resets In`, `%`, `EXHAUSTED`, and `Disabled`. It should capture rows as model label, remaining percentage, reset countdown/timestamp, and disabled/exhausted state.
2. If exact parsing fails, run a fuzzy pass over the visible screen buffer for quota-shaped labels: `quota`, `usage`, `remaining`, `reset`, `credits`, `exhausted`, `disabled`, model names, percentages, and time/countdown tokens.

Scraped TUI text carries no schema and no stability contract. Unlike JSON returned from `GetUserStatus` or `fetchAvailableModels`, the TUI has no versioned field names and no validator, so drift is expected. This path is viable only because the official CLI exposes `/usage` and `/quota`; it should never outrank API/JSON inspection.

Whether a slash command can be passed as a preliminary command at launch is unknown. No documented `agy --command /usage`, `agy -p /usage`, or equivalent was found in the official README or changelog. Local logs show “Print mode” for ordinary prompt execution, but not slash-command execution (`/Users/ken/.gemini/antigravity-cli/log/cli-20260708_114540.log`).

## Config and Log Artifacts

Observed local files under `/Users/ken/.antigravity`:

| Path | Usage/Limit Finding | Freshness |
| --- | --- | --- |
| `/Users/ken/.antigravity/argv.json` | No usage/quota fields; app shell arguments and crash reporter ID only. | Static app configuration. |
| `/Users/ken/.antigravity/extensions/` | Extension payloads; no Antigravity quota state found in targeted search. | Extension install/update state, not usage state. |

Observed Antigravity CLI files under `/Users/ken/.gemini/antigravity-cli`:

| Path | Usage/Limit Finding | Freshness |
| --- | --- | --- |
| `/Users/ken/.gemini/antigravity-cli/settings.json` | Selected model: `Gemini 3.1 Pro (High)`; no quota counters. | Updated when settings change. |
| `/Users/ken/.gemini/antigravity-cli/jetski_state.pbtxt` | Onboarding includes `POST_ONBOARDING_STEP_TYPE_USAGE_MODE`; no counters. | Written during onboarding. |
| `/Users/ken/.gemini/antigravity-cli/cache/onboarding.json` | Consumer onboarding complete, enterprise onboarding false; no counters. | Written during onboarding. |
| `/Users/ken/.gemini/antigravity-cli/log/cli-20260708_*.log` | Auth state, quota refresh loop state, Cloud Code endpoint URLs, and auth-required failures. Logs do not contain refreshed quota values in the inspected files. | Written live per CLI session. |
| `/Users/ken/.gemini/antigravity-cli/conversations/*.db` | SQLite conversation records with binary metadata; table names do not expose quota state. | Per conversation/session. |
| `/Users/ken/.gemini/antigravity-cli/conversation_summaries.db` | `conversation_summaries` only; no quota table found. | Updated as conversations change. |

Observed local logs contain useful markers:

- Auth-required: `failed to get load code assist response: error getting token source: You are not logged into Antigravity`.
- Successful auth: `applyAuthResult: email=<email>, authMethod=consumer, quotaProject=`.
- Inspection calls: `URL: https://daily-cloudcode-pa.googleapis.com/v1internal:loadCodeAssist` and `URL: https://daily-cloudcode-pa.googleapis.com/v1internal:fetchAvailableModels`.
- Refresh manager: `quotaRefreshLoop: starting reload (force=true)` and `quotaRefreshLoop: skipped (throttled)`.

These artifacts are diagnostic, not a fresh usage source. They prove the mechanisms Antigravity uses, but an inspecting tool should query the local/API JSON surfaces rather than parse historical logs.

## Metrics and Windows

| Mechanism | Metric | Unit | Window | Reset Expression |
| --- | --- | --- | --- | --- |
| Local `GetUserStatus` | `quotaInfo.remainingFraction` per model | Percent/fraction | Unknown; likely model quota window | `quotaInfo.resetTime` timestamp when present |
| Local `GetUserStatus` | `planStatus.availablePromptCredits` and `planInfo.monthlyPromptCredits` | Credits | Monthly/billing-style prompt credits | No reset timestamp observed |
| Cloud `loadCodeAssist` | `availablePromptCredits`, `planInfo.monthlyPromptCredits`, `planInfo.planType` | Credits and plan tier | Monthly/billing-style prompt credits | No reset timestamp observed |
| Cloud `fetchAvailableModels` | `models.<id>.quotaInfo.remainingFraction`, `isExhausted` | Percent/fraction and boolean | Unknown; likely per model family/window | `quotaInfo.resetTime` timestamp |
| `/usage` or `/quota` | Remaining quota and real-time consumption statistics | Styled human text, likely percent/countdown | Unknown | Textual countdown or reset indicator, no schema |
| Status line | Quota usage | Styled human text | Unknown | Unknown |

The public plan model includes five-hour refresh behavior for paid CLI subscribers and weekly limits for free users, but the inspected JSON fields carry reset timestamps rather than explicit “five-hour” or “weekly” labels. Claudine should store the window as `unknown` unless it can map the model/account tier externally.

## Limit States

| State | Detection | Markers |
| --- | --- | --- |
| Auth required | Structured/API and logs | HTTP 401/403 in Cloud Code client; `You are not logged into Antigravity`; local logs showing `failed to get load code assist response: error getting token source`. |
| Capped | Structured/API or TUI | `quotaInfo.isExhausted=true`, `quotaInfo.remainingFraction=0`, parsed `isExhausted=true`, TUI `EXHAUSTED`. |
| Cap approaching | Structured/API with Claudine threshold | `quotaInfo.remainingFraction` below a Claudine-defined warning threshold; no provider threshold found. |
| No funds / credits exhausted | Prompt-credit fields, possibly `/credits` panel | `availablePromptCredits=0` or `planStatus.availablePromptCredits=0`; changelog documents G1 credits and a `/credits` panel but no stable no-funds schema. |
| Permission-limited model fetch | API negative probe | `fetchAvailableModels` can fail with 403 while `loadCodeAssist` still yields prompt credits, based on the third-party service fallback. |

## Sources

- [Google Antigravity CLI product page](https://antigravity.google/product/antigravity-cli)
- [Google Antigravity AI Credits documentation](https://antigravity.google/docs/cli/credits)
- [Google Antigravity Plans documentation](https://antigravity.google/docs/plans)
- [google-antigravity/antigravity-cli README](https://github.com/google-antigravity/antigravity-cli)
- [google-antigravity/antigravity-cli CHANGELOG](https://github.com/google-antigravity/antigravity-cli/blob/main/CHANGELOG.md)
- [antigravity-usage README](https://github.com/skainguyen1412/antigravity-usage)
- [antigravity-usage local Connect client](https://github.com/skainguyen1412/antigravity-usage/blob/main/src/local/connect-client.ts)
- [antigravity-usage Cloud Code client](https://github.com/skainguyen1412/antigravity-usage/blob/main/src/google/cloudcode.ts)
- [antigravity-usage quota parser](https://github.com/skainguyen1412/antigravity-usage/blob/main/src/google/parser.ts)
- [antigravity-usage quota service](https://github.com/skainguyen1412/antigravity-usage/blob/main/src/quota/service.ts)
- [antigravity-usage quota types](https://github.com/skainguyen1412/antigravity-usage/blob/main/src/quota/types.ts)
- [antigravity-usage Connect RPC solution note](https://github.com/skainguyen1412/antigravity-usage/blob/main/docs/solution.md)
- Observed on host: `/Users/ken/.antigravity/argv.json`
- Observed on host: `/Users/ken/.gemini/antigravity-cli/settings.json`
- Observed on host: `/Users/ken/.gemini/antigravity-cli/jetski_state.pbtxt`
- Observed on host: `/Users/ken/.gemini/antigravity-cli/log/cli-20260708_113052.log`
- Observed on host: `/Users/ken/.gemini/antigravity-cli/log/cli-20260708_114233.log`
- Observed on host: `/Users/ken/.gemini/antigravity-cli/log/cli-20260708_114540.log`
