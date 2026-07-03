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
    endpoint: POST https://cloudcode-pa.googleapis.com/v1internal:retrieveUserQuota
    auth: oauth
    users: Google-account Code Assist auth used by Gemini CLI; not documented as a public subscriber API.
    fields:
      - buckets[].modelId
      - buckets[].remainingAmount
      - buckets[].remainingFraction
      - buckets[].resetTime
    reset_window: Per-bucket resetTime when returned by the internal service.
    notes: Observed in installed Gemini CLI 0.46.0 source; Config.refreshUserQuota calls CodeAssistServer.retrieveUserQuota and caches bucket quota fields.
  - available: false
    endpoint: Public Gemini CLI usage/quota endpoint
    auth: unknown
    users: unknown
    fields: []
    reset_window: unknown
    notes: Official Gemini CLI docs document /stats model, AI Studio, Google Cloud dashboards, and Code Assist quota pages, but no supported standalone HTTP endpoint for current Gemini CLI usage inspection.
  - available: false
    endpoint: Gemini API quota inspection endpoint
    auth: api_key
    users: Gemini API key users.
    fields: []
    reset_window: Requests per day reset at midnight Pacific time per Gemini API docs, but no current-consumption endpoint was found.
    notes: API docs expose rate-limit dimensions and dashboard links, not a machine endpoint for remaining usage.
cli_methods:
  - available: false
    invocation: gemini stats|usage|quota --help
    interactive_only: false
    output_format: none
    fields: []
    notes: Local Gemini CLI 0.46.0 help listed mcp, extensions, skills, hooks, gemma, and query mode only; probes for stats, usage, and quota subcommands fell through to generic help.
  - available: true
    invocation: /stats model
    interactive_only: true
    output_format: styled TUI text
    fields:
      - per-session model token counts
      - quota model or tier labels
      - percent used
      - usage limit
      - reset countdown
      - reset label
    notes: Official command reference documents model token counts and quota information; installed source renders percent used, usage limit, daily reset text, and model quota rows.
  - available: true
    invocation: /stats
    interactive_only: true
    output_format: styled TUI text
    fields:
      - session duration
      - tool calls
      - performance metrics
    notes: Default stats view is session-scoped and not enough for provider quota headroom; useful only as adjacent session telemetry.
  - available: true
    invocation: gemini -p PROMPT --output-format json
    interactive_only: false
    output_format: JSON
    fields:
      - response
      - stats
      - error
    notes: Headless JSON produces usage statistics for the run that was just executed, not on-demand preflight quota headroom.
  - available: true
    invocation: gemini -p PROMPT --output-format stream-json
    interactive_only: false
    output_format: JSONL
    fields:
      - init
      - message
      - tool_use
      - tool_result
      - error
      - result.stats
      - result per-model token usage
    notes: Structured run-output path; not an outside-run usage inspection command.
pty_design:
  command: expectrl spawn of gemini in an interactive PTY, send /stats model, wait for Model usage and quota rows, then send /exit.
  first_pass_markers:
    - /stats model
    - Model Usage
    - Usage limit
    - Usage limits span all sessions and reset daily.
    - Limit resets in
    - "Resets:"
    - Pro
    - Flash
  fuzzy_markers:
    - stats
    - model
    - quota
    - usage
    - limit
    - remaining
    - reset
    - used
    - percent
  fields:
    - model_or_tier
    - used_percentage
    - remaining_amount
    - limit
    - reset_time_or_countdown
    - token_counts
  risks: Scraped TUI text has no schema, version, or stability contract; colors, layout, aliases, terminal width, plan, locale, and future UI rewrites can break exact parsing, so fuzzy fallback and explicit confidence flags are required.
metrics:
  - name: request limit
    unit: requests
    window: daily
    source: Official Gemini CLI quota docs and Code Assist quota docs.
    notes: Daily request limits vary by auth and plan; examples include 1000, 1500, 2000, and 250 requests per user per day.
  - name: used percentage
    unit: percent
    window: daily
    source: /stats model display and internal QuotaStatsInfo rendering.
    notes: Computed as 100 minus remaining divided by limit; rendered as whole-percent text.
  - name: usage limit
    unit: requests
    window: daily
    source: /stats model QuotaStatsInfo and retrieveUserQuota bucket remainingAmount/remainingFraction.
    notes: When remainingAmount exists, CLI derives limit from remainingAmount and remainingFraction; otherwise it normalizes fraction onto a 100-point scale.
  - name: quota reset
    unit: time
    window: daily
    source: retrieveUserQuota buckets[].resetTime and /stats model reset text.
    notes: Internal value is a resetTime field; TUI formats it as countdown text such as "Limit resets in ..." or row text beginning "Resets:".
  - name: per-session tokens
    unit: tokens
    window: session
    source: Local ~/.gemini/tmp/*/chats/*.jsonl and headless output stats.
    notes: Observed local chat JSONL records include tokens.input, output, cached, thoughts, tool, and total plus model; these are stale after write and not quota headroom.
  - name: API rate dimensions
    unit: other
    window: other
    source: Gemini API rate-limit docs.
    notes: API limits are measured in RPM, TPM, RPD, and sometimes TPD or spend-based rolling windows; docs do not expose current remaining usage via CLI.
  - name: AI credits
    unit: credits
    window: unknown
    source: Installed Gemini CLI source overage and credit code paths.
    notes: Source has G1 credit balance and insufficient-credit markers, but no local cache with fresh credit balance was observed on this host.
limit_states:
  - state: cap_approaching
    detectable: true
    source: /stats model percent used and internal color thresholds.
    markers:
      - usedPercentage >= warning threshold
      - yellow quota text
      - high percent used
    notes: Installed source colors quota rows at warning and critical thresholds, but no stable machine-readable approaching field was found.
  - state: capped
    detectable: true
    source: /stats model and quota error classification source.
    markers:
      - Limit reached
      - remaining === 0
      - You have exhausted your daily quota on this model.
      - QUOTA_EXHAUSTED
      - RATE_LIMIT_EXCEEDED
      - HTTP 429
    notes: /stats model can show a reached limit; run-time quota errors are owned by the sibling non-interactive-sessions topic.
  - state: no_funds
    detectable: true
    source: Installed Gemini CLI source.
    markers:
      - INSUFFICIENT_G1_CREDITS_BALANCE
      - emptyWalletRequest
      - creditBalance
      - Newly purchased AI credits may take a few minutes to update. Run /stats to check your balance.
    notes: Distinct from daily quota exhaustion when AI credits are required or selected for overage.
  - state: auth_required
    detectable: true
    source: Local settings and CLI auth behavior.
    markers:
      - missing oauth_creds.json
      - expired OAuth credentials
      - selectedType requires unavailable credential
      - /auth prompt
    notes: Inspect selected auth in ~/.gemini/settings.json before interpreting missing quota fields.
  - state: unknown
    detectable: false
    source: Public documentation.
    markers: []
    notes: No official structured state taxonomy for Gemini CLI usage inspection was found.
docs: https://geminicli.com/docs/resources/quota-and-pricing/
changes: []
requires_claudine_update: true
reason: Claudine should add a Gemini usage inspector that prefers the internal Code Assist quota API for OAuth sessions, falls back to /stats model PTY scraping, and treats local chat token logs as session-only stale telemetry.
---

# Gemini CLI Usage Inspection

## Introduction to Gemini CLI Usage Inspection

Gemini CLI uses a mixed quota model. For Google-account and Workspace Code Assist usage, the documented limit is daily model requests per user, with requests aggregated across Gemini CLI and Gemini Code Assist agent mode; one prompt can consume multiple model requests. For Gemini API key and Vertex AI modes, the model is pay-as-you-go or project quota based, with Gemini API limits measured across requests per minute, input tokens per minute, requests per day, and related dimensions. Officially, Gemini CLI surfaces current CLI usage through the interactive `/stats model` command; API-key and Vertex users are pointed toward AI Studio or Google Cloud quota dashboards rather than a documented standalone CLI usage API.

Sources: [Gemini CLI quota and pricing](https://geminicli.com/docs/resources/quota-and-pricing/), [Gemini CLI command reference](https://geminicli.com/docs/reference/commands/), [Gemini Code Assist quotas](https://developers.google.com/gemini-code-assist/resources/quotas), [Gemini API rate limits](https://ai.google.dev/gemini-api/docs/rate-limits).

## API Call Opportunities

### Internal Code Assist Quota API

The installed Gemini CLI 0.46.0 source includes a structured quota path for Google-account Code Assist auth:

```http
POST https://cloudcode-pa.googleapis.com/v1internal:retrieveUserQuota
Content-Type: application/json

{"project":"<code-assist-project-id>"}
```

The request is made by `Config.refreshUserQuota()`, which calls `CodeAssistServer.retrieveUserQuota({ project })`. The `CodeAssistServer` base URL is `https://cloudcode-pa.googleapis.com/v1internal`; method URLs are formed as `${baseUrl}:${method}`. The response is expected to include `buckets[]`; the CLI reads `bucket.modelId`, `bucket.remainingAmount`, `bucket.remainingFraction`, and `bucket.resetTime`, then caches per-model `remaining`, `limit`, and `resetTime`.

Auth is OAuth through the Code Assist content generator. The same source constructs `CodeAssistServer` only for `oauth-personal` and `compute-default-credentials`; API-key and Vertex paths use other content generators and do not expose this Code Assist quota call. Users are therefore Google-account or ADC Code Assist users, not every Gemini API key user. This endpoint is not documented as a public subscriber API, so Claudine should treat it as best-effort and version-sensitive.

Observed-on-host citations:

- `/Users/ken/.nvm/versions/node/v22.20.0/lib/node_modules/@google/gemini-cli/bundle/chunk-6ZHP2EJW.js:308061` defines `CODE_ASSIST_ENDPOINT = "https://cloudcode-pa.googleapis.com"` and `CODE_ASSIST_API_VERSION = "v1internal"`.
- `/Users/ken/.nvm/versions/node/v22.20.0/lib/node_modules/@google/gemini-cli/bundle/chunk-6ZHP2EJW.js:308282` defines `retrieveUserQuota(req)` as `requestPost("retrieveUserQuota", req)`.
- `/Users/ken/.nvm/versions/node/v22.20.0/lib/node_modules/@google/gemini-cli/bundle/chunk-6ZHP2EJW.js:380586` through `:380650` parses `quota.buckets[]` into `modelQuotas`.
- `/Users/ken/.nvm/versions/node/v22.20.0/lib/node_modules/@google/gemini-cli/bundle/chunk-6ZHP2EJW.js:309330` through `:309354` shows the Code Assist server is available only when the wrapped content generator is a `CodeAssistServer`.

Reset information: `buckets[].resetTime` is carried when returned by the service. The TUI formats that value as a countdown or reset label.

### Public API Status

No supported public HTTP endpoint for current Gemini CLI usage headroom was found. The official quota page says to use `/stats model`; the API rate-limit page links to active rate limits in AI Studio and says API requests per day reset at midnight Pacific time, but it does not describe a current-remaining endpoint. This is a negative finding, not a proof that no private endpoint exists.

### Negative Probes

Local CLI probes on Gemini CLI 0.46.0:

| Probe | Result |
| --- | --- |
| `gemini stats --help` | Generic top-level help; no `stats` subcommand. |
| `gemini usage --help` | Generic top-level help; no `usage` subcommand. |
| `gemini quota --help` | Generic top-level help; no `quota` subcommand. |

Observed-on-host citation: `gemini --help` listed only `mcp`, `extensions`, `skills`, `hooks`, `gemma`, and query mode; `gemini --version` returned `0.46.0`.

## CLI Switch Opportunities

There is no documented non-interactive `gemini usage`, `gemini quota`, or `gemini stats` switch that reports current plan headroom.

Gemini CLI does have structured headless output for a run:

```bash
gemini -p "..." --output-format json
gemini -p "..." --output-format stream-json
```

The official headless-mode docs say JSON output contains `response`, `stats`, and optional `error`, while streaming JSON emits JSONL events ending in a `result` with aggregated statistics and per-model token usage. This is machine-parseable, but it is not a preflight quota lookup because it requires starting a run and reports the run that just occurred.

Source: [Gemini CLI headless mode](https://geminicli.com/docs/features/headless/). Observed-on-host local bundled doc: `/Users/ken/.nvm/versions/node/v22.20.0/lib/node_modules/@google/gemini-cli/bundle/docs/cli/headless.md`.

## Interactive Commands and PTY Scraping

The official inspection command is:

```text
/stats model
```

The command reference says `/stats` displays detailed statistics for the current session, and its `model` subcommand shows model-specific usage statistics, including token counts and quota information. The quota page says `/stats model` checks current token usage and applicable limits and provides a snapshot of current-session token usage plus information about limits associated with the current quota.

Installed source shows the current reporting shape:

- `QuotaStatsInfo` renders `"Limit reached"` when remaining is zero.
- Otherwise it renders a whole percent used and optionally `"Limit resets in ..."`.
- Detail lines include `"Usage limit: <limit>"` and `"Usage limits span all sessions and reset daily."`
- `ModelQuotaDisplay` renders rows grouped by model tier or model ID with percent used and optional `"Resets: ..."` text.

Observed-on-host citations:

- `/Users/ken/.nvm/versions/node/v22.20.0/lib/node_modules/@google/gemini-cli/bundle/interactiveCli-NKTBHB7O.js:8534` through `:8563` renders `QuotaStatsInfo`.
- `/Users/ken/.nvm/versions/node/v22.20.0/lib/node_modules/@google/gemini-cli/bundle/interactiveCli-NKTBHB7O.js:16460` through `:16590` renders `ModelQuotaDisplay` rows.
- `/Users/ken/.nvm/versions/node/v22.20.0/lib/node_modules/@google/gemini-cli/bundle/interactiveCli-NKTBHB7O.js:32606` through `:32684` wires quota updates into UI state.

No evidence was found that `/stats model` can be passed as a preliminary launch command. `gemini [query..]` and `-p/--prompt` are model prompts, not slash-command injection surfaces. Therefore PTY scraping means starting an interactive session, waiting for the input prompt, sending `/stats model`, capturing the rendered output, and exiting.

### Two-Pass `expectrl` Design

First pass:

1. Spawn `gemini` in a PTY with a controlled width, for example 120 columns.
2. Wait for the interactive input prompt.
3. Send `/stats model\r`.
4. Capture until the screen contains exact markers: `Model Usage`, `Usage limit`, `Limit resets in`, `Usage limits span all sessions and reset daily.`, `Resets:`, or `No API calls have been made in this session.`
5. Parse rows for model/tier, percent used, limit, remaining-derived values if present, and reset text.

Second pass:

1. Run only if exact markers fail.
2. Strip ANSI/OSC and box-drawing noise.
3. Fuzzy-search for nearby terms: `stats`, `model`, `quota`, `usage`, `limit`, `remaining`, `reset`, `used`, `%`.
4. Extract candidate percentages, limits, and reset phrases from the same line and neighboring lines.
5. Return lower-confidence structured data with the raw excerpt attached for diagnostics.

Caveat: scraped TUI text has no schema and no stability contract. It is not versioned, validated, or guaranteed across terminal sizes, themes, plans, locales, or releases. It is a last resort after structured API inspection fails.

## Config and Log Artifacts

This host has read access to `~/.gemini`, which resolves to a Claudine-managed tree at `/Users/ken/.claudine/.gemini`.

| Path | Observed fields | Freshness | Usage value |
| --- | --- | --- | --- |
| `~/.gemini/settings.json` | `security.auth.selectedType`, UI and tool settings | Live config | Auth selection only; no usage or quota state. |
| `~/.gemini/oauth_creds.json` | OAuth token fields and expiry | Credential cache | Auth evidence only; no quota fields. Tokens were not copied into this document. |
| `~/.gemini/google_accounts.json` | Active account email | Credential/account cache | Account identity only; no usage fields. |
| `~/.gemini/state.json` | `tipsShown` | UI state | No usage fields. |
| `~/.gemini/projects.json` | Project path aliases | Persistent project registry | No usage fields. |
| `~/.gemini/history/*/.project_root` | Project roots | Persistent history metadata | No usage fields. |
| `~/.gemini/tmp/*/logs.json` | User messages and slash-command entries | Per-session/project logs | No authoritative quota fields found; can contain prompts and exits. |
| `~/.gemini/tmp/*/chats/*.jsonl` | `tokens.input`, `tokens.output`, `tokens.cached`, `tokens.thoughts`, `tokens.tool`, `tokens.total`, `model`, tool calls | Written during sessions | Useful for stale per-session token history; not current quota headroom or reset state. |

Observed local scans found no `quota`, `remainingFraction`, `remainingAmount`, or `resetTime` records under `~/.gemini` JSON/JSONL artifacts. They did find token usage in chat JSONL records, for example `~/.gemini/tmp/claudine-2/chats/session-2026-06-10T16-20-99967bca.jsonl` contained `tokens` and `model` fields. That artifact is stale historical session telemetry and should not be used as plan runway.

## Metrics and Windows

| Mechanism | Metrics | Window | Reset expression |
| --- | --- | --- | --- |
| Internal `retrieveUserQuota` | `modelId`, `remainingAmount`, `remainingFraction`, derived `remaining`, derived `limit`, `resetTime` | Daily or provider-defined bucket; source UI states daily | Raw `resetTime` field when present. |
| `/stats model` | Session token counts, percent used, usage limit, model/tier rows, reset label | Session tokens plus all-session daily quota | Human countdown or `Resets:` label. |
| Headless JSON | `stats` and per-model token usage for the completed run | Session/run | None for plan quota. |
| Local chat JSONL | Per-response token counts and model IDs | Session/history | None. |
| Gemini API docs | RPM, TPM, RPD, spend-based limits, sometimes TPD/IPM | Minute, day, rolling 10-minute spend windows, other model-specific windows | RPD resets at midnight Pacific time; other reset details are dashboard/API-error dependent. |
| Code Assist docs | Requests per user per day | Daily | Daily reset; exact timestamp not documented on the page. |

Unknowns:

- The exact public stability of `retrieveUserQuota` is unknown.
- Whether all Google AI Pro/Ultra, Workspace, and Code Assist tiers receive the same bucket field set is unknown.
- Whether reset windows can be shorter than daily for future Gemini CLI plans is unknown; public docs currently emphasize daily request limits for Gemini CLI/Code Assist, while the user-facing Gemini web app has separate five-hour/weekly behavior outside this CLI evidence set.

## Limit States

| State | Detection markers | Mechanism | Notes |
| --- | --- | --- | --- |
| Cap approaching | High `usedPercentage`; warning-colored quota display | `/stats model` or internal bucket math | Installed source has warning/critical color thresholds, but no stable structured approaching field. |
| Capped | `Limit reached`, `remaining === 0`, `You have exhausted your daily quota on this model.`, `QUOTA_EXHAUSTED`, `RATE_LIMIT_EXCEEDED`, HTTP 429 | `/stats model`, internal quota cache, run-time error parser | Run-time event detection belongs to the sibling non-interactive-sessions topic. |
| No funds | `INSUFFICIENT_G1_CREDITS_BALANCE`, `emptyWalletRequest`, `creditBalance`, "Newly purchased AI credits may take a few minutes to update. Run /stats to check your balance." | Installed source | Indicates AI-credit exhaustion rather than included daily quota exhaustion. |
| Auth required | Missing or expired credentials, selected auth type without usable credential, `/auth` prompt | Config and interactive UI | Check `~/.gemini/settings.json` and credential files before interpreting missing quota data. |
| Unknown | No marker | Public docs | No official structured state taxonomy for inspection was found. |

Observed-on-host limit-state citations:

- `/Users/ken/.nvm/versions/node/v22.20.0/lib/node_modules/@google/gemini-cli/bundle/chunk-6ZHP2EJW.js:304827` through `:304940` classifies quota failures, daily exhaustion, per-minute retryability, `QUOTA_EXHAUSTED`, `RATE_LIMIT_EXCEEDED`, and `INSUFFICIENT_G1_CREDITS_BALANCE`.
- `/Users/ken/.nvm/versions/node/v22.20.0/lib/node_modules/@google/gemini-cli/bundle/interactiveCli-NKTBHB7O.js:25271` through `:25321` renders overage, empty-wallet, and credit-update UI paths.

## Sources

- [Gemini CLI quota and pricing](https://geminicli.com/docs/resources/quota-and-pricing/)
- [Gemini CLI command reference](https://geminicli.com/docs/reference/commands/)
- [Gemini CLI headless mode](https://geminicli.com/docs/features/headless/)
- [Gemini CLI configuration reference](https://geminicli.com/docs/reference/configuration/)
- [Gemini Code Assist quotas and limits](https://developers.google.com/gemini-code-assist/resources/quotas)
- [Gemini API rate limits](https://ai.google.dev/gemini-api/docs/rate-limits)
- [Gemini CLI GitHub repository](https://github.com/google-gemini/gemini-cli)
- Observed on host: `gemini --version` returned `0.46.0`.
- Observed on host: `/Users/ken/.nvm/versions/node/v22.20.0/lib/node_modules/@google/gemini-cli/bundle/chunk-6ZHP2EJW.js`.
- Observed on host: `/Users/ken/.nvm/versions/node/v22.20.0/lib/node_modules/@google/gemini-cli/bundle/interactiveCli-NKTBHB7O.js`.
- Observed on host: `~/.gemini/settings.json`, `~/.gemini/oauth_creds.json`, `~/.gemini/google_accounts.json`, `~/.gemini/state.json`, `~/.gemini/projects.json`, `~/.gemini/tmp/*/logs.json`, and `~/.gemini/tmp/*/chats/*.jsonl`.
