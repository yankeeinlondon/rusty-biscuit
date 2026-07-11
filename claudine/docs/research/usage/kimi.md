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
    endpoint: https://api.kimi.com/coding/v1/usages
    auth: oauth
    users: Kimi Code platform users with a valid Kimi Code OAuth-resolved bearer token; unknown whether Moonshot Open Platform API keys are accepted because the CLI gates this endpoint to the managed Kimi Code platform.
    fields:
      - usage
      - usage.name
      - usage.title
      - usage.used
      - usage.remaining
      - usage.limit
      - usage.reset_at
      - usage.resetAt
      - usage.reset_time
      - usage.resetTime
      - usage.reset_in
      - usage.resetIn
      - usage.ttl
      - limits
      - limits[].detail
      - limits[].window.duration
      - limits[].window.timeUnit
    reset_window: Successful responses may carry absolute reset timestamps or countdown seconds on each usage row; limits may also carry window.duration and window.timeUnit for labels such as 5h or 7d.
    notes: Current upstream source constructs this endpoint from the Kimi Code platform base URL and sends Authorization Bearer. A local probe on 2026-07-03 reached the endpoint but returned HTTP 401 unauthenticated with REASON_INVALID_AUTH_TOKEN because the stored OAuth token was stale.
cli_methods:
  - available: false
    invocation: kimi usage
    interactive_only: false
    output_format: none
    fields: []
    notes: No current non-interactive usage subcommand or switch was found in docs, --help output, or source.
  - available: true
    invocation: /usage
    interactive_only: true
    output_format: styled terminal text
    fields:
      - label
      - percent_left
      - progress_bar
      - reset_hint
    notes: Slash command alias /status. Works only when the selected model belongs to the Kimi Code platform.
pty_design:
  command: kimi
  first_pass_markers:
    - API Usage
    - "% left"
    - resets in
    - Usage is available on Kimi Code platform only.
    - Authorization failed. Please check your API key.
  fuzzy_markers:
    - usage
    - quota
    - left
    - remaining
    - reset
    - limit
    - weekly
    - 5h
    - 7d
  fields:
    - label
    - percent_left
    - reset_hint
    - auth_error
    - endpoint_unavailable
  risks: Interactive scraping has no schema or stability contract; rich progress bars, colors, aliases, and wording can change without versioning, so this must remain a last resort behind the JSON API.
metrics:
  - name: remaining quota percentage
    unit: percent
    window: unknown
    source: GET https://api.kimi.com/coding/v1/usages and /usage display
    notes: Computed as (limit - used) / limit, or from remaining when the API returns remaining instead of used.
  - name: used quota
    unit: unknown
    window: unknown
    source: GET https://api.kimi.com/coding/v1/usages
    notes: The endpoint parser treats used and limit as integers but does not define whether they represent requests, credits, tokens, or an internal Kimi Code unit.
  - name: quota limit
    unit: unknown
    window: unknown
    source: GET https://api.kimi.com/coding/v1/usages
    notes: Integer denominator used for remaining percentage.
  - name: short-window quota
    unit: unknown
    window: five_hour
    source: GET https://api.kimi.com/coding/v1/usages limits[].window
    notes: Inferred from parser support for duration 300 and timeUnit MINUTE labels; exact unit and live payload not confirmed on this host because auth was stale.
  - name: long-window quota
    unit: unknown
    window: weekly
    source: GET https://api.kimi.com/coding/v1/usages usage
    notes: The parser labels top-level usage as Weekly limit by default; exact successful payload not confirmed on this host because auth was stale.
  - name: per-turn token usage
    unit: tokens
    window: session
    source: /Users/ken/.kimi-code/sessions/**/wire.jsonl
    notes: Local wire logs contain usage.record events with inputOther, output, inputCacheRead, and inputCacheCreation. This is historical session accounting, not current quota headroom.
  - name: context token count
    unit: tokens
    window: session
    source: /Users/ken/.kimi/sessions/**/context.jsonl
    notes: Legacy local context logs contain role _usage records with token_count. This is stale session context accounting, not current quota headroom.
limit_states:
  - state: cap_approaching
    detectable: true
    source: /usage renderer
    markers:
      - remaining ratio <= 0.3
      - yellow progress bar
      - "% left"
    notes: The renderer colors rows yellow at 30% or less remaining; this is a UI threshold, not a provider schema field.
  - state: capped
    detectable: true
    source: /usage renderer and GET /usages fields
    markers:
      - 0% left
      - remaining equals 0
      - used >= limit
      - red progress bar
    notes: The renderer clamps exhausted or overused rows to 0% left.
  - state: no_funds
    detectable: false
    source: unknown
    markers: []
    notes: No on-demand usage-inspection field or CLI command distinguishing no funds from quota exhaustion was found. Print mode treats quota exhaustion as permanent exit code 1 during a run, but that belongs to run-time failure detection.
  - state: auth_required
    detectable: true
    source: GET /usages and /usage renderer
    markers:
      - HTTP 401
      - unauthenticated
      - REASON_INVALID_AUTH_TOKEN
      - Authorization failed. Please check your API key.
    notes: Observed on host on 2026-07-03 when probing /usages with the stored OAuth token.
docs: https://moonshotai.github.io/kimi-cli/en/reference/slash-commands.html#usage
changes: []
requires_claudine_update: true
reason: Claudine can add a Kimi plan-awareness probe that calls the Kimi Code /usages JSON endpoint with the OAuth-resolved bearer token before falling back to PTY scraping of /usage.
---

# Kimi Code CLI Usage Inspection

## Introduction to Kimi Code CLI Usage Inspection

Kimi Code CLI has two distinct account surfaces. The Kimi Code platform is configured at `https://api.kimi.com/coding/v1` and is the only platform for which the CLI's `/usage` command is documented to work; Moonshot Open Platform endpoints such as `https://api.moonshot.ai/v1` use the broader API platform pricing and rate-limit model. The official Kimi API platform describes cumulative-recharge-based API rate limits in terms of concurrency, RPM, TPM, and TPD, but Kimi Code CLI's usage inspection code queries a separate Kimi Code `/usages` endpoint and renders quota rows as progress bars and percent remaining. [Kimi Code CLI provider docs](https://moonshotai.github.io/kimi-cli/en/configuration/providers.html), [slash command docs](https://moonshotai.github.io/kimi-cli/en/reference/slash-commands.html#usage), [Kimi API rate-limit docs](https://platform.kimi.ai/docs/pricing/limits)

Officially, the provider surfaces Kimi Code usage through the interactive `/usage` slash command, with `/status` as an alias. The current CLI source reveals the underlying JSON endpoint used by that command, but there is no documented standalone `kimi usage` subcommand or documented `--json` switch for usage inspection. Local host artifacts under `/Users/ken/.kimi-code` and `/Users/ken/.kimi` contain auth, config, logs, and session token accounting, but no fresh quota-cache artifact was found. [Slash command docs](https://moonshotai.github.io/kimi-cli/en/reference/slash-commands.html#usage); observed on host: `/Users/ken/.kimi-code/config.toml`, `/Users/ken/.kimi-code/sessions/**/wire.jsonl`, `/Users/ken/.kimi/sessions/**/context.jsonl`.

## API Call Opportunities

| Opportunity | Finding |
| --- | --- |
| Endpoint | `GET https://api.kimi.com/coding/v1/usages`. Current upstream source builds this as `f"{base_url}/usages"` after verifying the selected model is from the managed Kimi Code platform. [usage.py](https://github.com/MoonshotAI/kimi-cli/blob/main/src/kimi_cli/ui/shell/usage.py) |
| Request | `curl -H "Authorization: Bearer $KIMI_CODE_TOKEN" -H "Accept: application/json" https://api.kimi.com/coding/v1/usages`. Source sends `Authorization: Bearer {api_key}` and parses JSON. [usage.py](https://github.com/MoonshotAI/kimi-cli/blob/main/src/kimi_cli/ui/shell/usage.py) |
| Auth | Bearer token. In normal Kimi Code login, the CLI resolves `provider.api_key` through the provider OAuth reference before calling the endpoint. Local config has `[providers."managed:kimi-code".oauth] storage = "file", key = "oauth/kimi-code"` and token files under `/Users/ken/.kimi-code/credentials/kimi-code.json`; values were inspected but not copied. [usage.py](https://github.com/MoonshotAI/kimi-cli/blob/main/src/kimi_cli/ui/shell/usage.py); observed on host: `/Users/ken/.kimi-code/config.toml`, `/Users/ken/.kimi-code/credentials/kimi-code.json`. |
| Users | Kimi Code platform users. The CLI returns "Usage is available on Kimi Code platform only" before calling the endpoint when the selected model is not from `managed:kimi-code`. [usage.py](https://github.com/MoonshotAI/kimi-cli/blob/main/src/kimi_cli/ui/shell/usage.py), [slash command docs](https://moonshotai.github.io/kimi-cli/en/reference/slash-commands.html#usage) |
| Response fields | The parser accepts top-level `usage` and `limits`. Each row may contain `name` or `title`, `used` or `remaining`, `limit`, and reset fields `reset_at`, `resetAt`, `reset_time`, `resetTime`, `reset_in`, `resetIn`, `ttl`, or `window`. Each limit may contain nested `detail` and `window.duration` plus `window.timeUnit`. [usage.py](https://github.com/MoonshotAI/kimi-cli/blob/main/src/kimi_cli/ui/shell/usage.py) |
| Reset information | Absolute reset timestamps are parsed as ISO-like UTC strings and displayed as `resets in ...`; countdown-like fields are interpreted as seconds. Window labels are inferred from duration plus time unit, including minutes, hours, and days. [usage.py](https://github.com/MoonshotAI/kimi-cli/blob/main/src/kimi_cli/ui/shell/usage.py) |
| Negative probe | On 2026-07-03, a direct host probe to `GET https://api.kimi.com/coding/v1/usages` with the stored `/Users/ken/.kimi-code/credentials/kimi-code.json` access token returned HTTP 401 with `code: unauthenticated` and debug reason `REASON_INVALID_AUTH_TOKEN`. No static `managed:kimi-code` API key was present in `/Users/ken/.kimi-code/config.toml`, so a successful live payload could not be captured. Observed on host. |
| Negative source behavior | The CLI maps HTTP 401 to "Authorization failed. Please check your API key." and HTTP 404 to "Usage endpoint not available. Try Kimi for Coding." [usage.py](https://github.com/MoonshotAI/kimi-cli/blob/main/src/kimi_cli/ui/shell/usage.py) |

Example successful shape, based on the current parser rather than a successful host response:

```json
{
  "usage": {
    "name": "Weekly limit",
    "used": 100,
    "limit": 1000,
    "resetAt": "2026-07-06T00:00:00Z"
  },
  "limits": [
    {
      "window": { "duration": 300, "timeUnit": "MINUTE" },
      "detail": {
        "used": 20,
        "limit": 200,
        "resetIn": 3600
      }
    }
  ]
}
```

The exact successful live response remains unknown from this host because local auth was stale.

## CLI Switch Opportunities

No non-interactive CLI switch or subcommand equivalent to `kimi usage`, `kimi usage --json`, or `kimi --usage` was found in the installed `0.14.0` help output, current docs, or current source. The installed host binary reports `0.14.0`, while current upstream docs and source are much newer; this version skew is relevant because `/usage` was added and refined after early releases. Observed on host: `/Users/ken/.kimi-code/bin/kimi --version`, `/Users/ken/.kimi-code/bin/kimi --help`; [current print-mode docs](https://moonshotai.github.io/kimi-cli/en/customization/print-mode.html), [current CLI source](https://github.com/MoonshotAI/kimi-cli).

Print mode supports `--output-format=stream-json`, but that structured output is for agent messages and tool messages, not account usage inspection. It does not expose a usage command in non-interactive mode. [Print mode docs](https://moonshotai.github.io/kimi-cli/en/customization/print-mode.html)

| Invocation | Works non-interactively | Structured | Finding |
| --- | --- | --- | --- |
| `kimi usage` | No | No | Not found. |
| `kimi --usage` | No | No | Not found. |
| `kimi --print --output-format=stream-json` | Yes | JSONL | Structured agent output exists, but it is not an account usage inspection path. |

## Interactive Commands and PTY Scraping

`/usage` displays API usage and quota information with progress bars and remaining percentages. `/status` is an alias. The docs state that it only works with the Kimi Code platform, and source enforces that by checking the managed platform before calling `/usages`. [Slash command docs](https://moonshotai.github.io/kimi-cli/en/reference/slash-commands.html#usage), [usage.py](https://github.com/MoonshotAI/kimi-cli/blob/main/src/kimi_cli/ui/shell/usage.py)

The visible display is a Rich panel titled `API Usage`. Each row includes a label, a progress bar whose completed length represents remaining quota, a `NN% left` string, and an optional reset hint such as `resets in 1h`. The renderer computes remaining as `limit - used`, clamps unusual values, and colors rows green, yellow, or red at remaining-ratio thresholds. [usage.py](https://github.com/MoonshotAI/kimi-cli/blob/main/src/kimi_cli/ui/shell/usage.py), [test_usage.py](https://github.com/MoonshotAI/kimi-cli/blob/main/tests/ui/test_usage.py)

PTY scraping with `expectrl` should use two passes:

1. First pass: spawn `kimi`, wait for the prompt, send `/usage\r`, and match exact current markers: `API Usage`, `% left`, optional `resets in`, `Authorization failed. Please check your API key.`, `Usage endpoint not available. Try Kimi for Coding.`, and `Usage is available on Kimi Code platform only.` Extract row labels, percentages, reset hints, and auth or endpoint errors from the visible screen.
2. Second pass: only after exact matching fails, capture the current screen buffer and fuzzy-search for quota vocabulary: `usage`, `quota`, `remaining`, `left`, `reset`, `limit`, `weekly`, `5h`, and `7d`. This pass should produce lower-confidence fields and include the raw matched line for diagnostics.

Scraped TUI text carries no schema and no stability contract. The Rich panel title, colors, progress-bar glyphs, aliases, row labels, and wording can change without a versioned response format. PTY scraping is therefore strictly a last resort behind the JSON endpoint and should tolerate drift by returning partial data plus confidence metadata.

## Config and Log Artifacts

| Artifact | Observed fields | Freshness | Usage value |
| --- | --- | --- | --- |
| `/Users/ken/.kimi-code/config.toml` | `default_model`, `providers."managed:kimi-code".base_url = "https://api.kimi.com/coding/v1"`, provider OAuth reference, search and fetch service URLs. Sensitive values redacted during inspection. | Updated when login/model/config changes. | Identifies the endpoint base and OAuth storage key; does not store current quota. |
| `/Users/ken/.kimi-code/credentials/kimi-code.json` | `access_token`, `refresh_token`, `expires_at`, `expires_in`, `scope`, `token_type`. | OAuth credential cache; can become stale. | Auth source for `/usages`; the current access token returned 401 on 2026-07-03. |
| `/Users/ken/.kimi-code/oauth/kimi-code` | OAuth token file exists. | OAuth credential cache; can become stale. | Auth source; not a usage cache. |
| `/Users/ken/.kimi-code/sessions/**/wire.jsonl` | `usage.record` events with `model`, `usage.inputOther`, `usage.output`, `usage.inputCacheRead`, `usage.inputCacheCreation`, `usageScope: "turn"`, and millisecond `time`. | Written during session turns. | Useful for historical per-turn token accounting; not current quota headroom and no reset windows. |
| `/Users/ken/.kimi/sessions/**/context.jsonl` | Legacy `_usage` records with `token_count`. | Written during legacy session context updates. | Stale session context accounting; not quota headroom. |
| `/Users/ken/.kimi-code/logs/kimi-code.log` and `/Users/ken/.kimi/logs/kimi.log` | Startup, config, model, update, and tool-load messages. | Written during CLI startup and runtime events. | No current quota fields found in inspected logs. |
| `/Users/ken/.kimi-code/session_index.jsonl` | `sessionDir`, `sessionId`, `workDir`. | Updated as sessions are created/indexed. | No usage or quota fields. |
| `/Users/ken/.kimi/kimi.json` | `work_dirs`. | Legacy metadata. | No usage or quota fields. |

Official docs describe the default data directory as `~/.kimi/`, with config, credentials, sessions, `context.jsonl`, `wire.jsonl`, `state.json`, user history, and logs. This host also has a newer `/Users/ken/.kimi-code` directory created by the standalone Kimi Code install/migration path; local inspection found both directories. [Data locations docs](https://moonshotai.github.io/kimi-cli/en/configuration/data-locations.html); observed on host: `/Users/ken/.kimi`, `/Users/ken/.kimi-code`.

## Metrics and Windows

| Mechanism | Metrics | Window | Reset expression | Notes |
| --- | --- | --- | --- | --- |
| `GET /coding/v1/usages` | `used`, `remaining`, `limit`, computed percent left, labels | Unknown; parser supports top-level weekly-style summary and multiple limit rows | Absolute timestamp fields or countdown seconds | Best Claudine target because it is JSON, but the successful live payload could not be confirmed on this host. |
| `/usage` or `/status` | Label, progress bar, percent left, optional reset hint | Same as endpoint rows | Human text such as `resets in ...` | Human-only Rich output. |
| `wire.jsonl` `usage.record` | `inputOther`, `output`, `inputCacheRead`, `inputCacheCreation` | Session turn | None | Historical local accounting, not quota. |
| Legacy `context.jsonl` `_usage` | `token_count` | Session context | None | Historical context accounting, not quota. |
| Kimi API platform docs | Concurrency, RPM, TPM, TPD | Minute/day depending on metric | Unknown from docs page | Applies to broader API platform; not the same as Kimi Code `/usage` quota display. [Rate-limit docs](https://platform.kimi.ai/docs/pricing/limits) |

The units used by Kimi Code `/usages` are unknown. Current source treats `used`, `remaining`, and `limit` as integers but does not name them as requests, credits, tokens, or currency. The API platform docs define RPM, TPM, and TPD for Open Platform rate limits, but those docs do not define the Kimi Code `/usages` row unit. [usage.py](https://github.com/MoonshotAI/kimi-cli/blob/main/src/kimi_cli/ui/shell/usage.py), [rate-limit docs](https://platform.kimi.ai/docs/pricing/limits)

## Limit States

| State | Detection | Markers | Caveat |
| --- | --- | --- | --- |
| Cap approaching | `/usage` renderer threshold | Remaining ratio `<= 0.3` yields yellow; display still says `% left`. | UI threshold, not a provider schema field. [usage.py](https://github.com/MoonshotAI/kimi-cli/blob/main/src/kimi_cli/ui/shell/usage.py) |
| Capped | Endpoint row or `/usage` renderer | `used >= limit`, `remaining = 0`, `0% left`, red progress bar. | Does not distinguish membership expiration from ordinary quota exhaustion. |
| Out of funds | Unknown for on-demand inspection | Unknown | No usage-inspection field or local artifact was found that separates no funds from exhausted quota. API platform rate-limit and recharge docs are separate from Kimi Code `/usage`. |
| Auth required | Endpoint or `/usage` error handling | HTTP 401; `unauthenticated`; `REASON_INVALID_AUTH_TOKEN`; "Authorization failed. Please check your API key." | Observed locally against `/usages` with stale OAuth token on 2026-07-03. |
| Endpoint unavailable/wrong platform | CLI precheck or endpoint response | "Usage is available on Kimi Code platform only."; HTTP 404 maps to "Usage endpoint not available. Try Kimi for Coding." | Useful to distinguish Open Platform models from Kimi Code platform usage. [usage.py](https://github.com/MoonshotAI/kimi-cli/blob/main/src/kimi_cli/ui/shell/usage.py) |

## Sources

- [Kimi Code CLI slash commands: `/usage`](https://moonshotai.github.io/kimi-cli/en/reference/slash-commands.html#usage)
- [Kimi Code CLI providers and Kimi Code base URL](https://moonshotai.github.io/kimi-cli/en/configuration/providers.html)
- [Kimi Code CLI data locations](https://moonshotai.github.io/kimi-cli/en/configuration/data-locations.html)
- [Kimi Code CLI print mode and stream JSON](https://moonshotai.github.io/kimi-cli/en/customization/print-mode.html)
- [Kimi API platform rate limits](https://platform.kimi.ai/docs/pricing/limits)
- [Upstream `usage.py`](https://github.com/MoonshotAI/kimi-cli/blob/main/src/kimi_cli/ui/shell/usage.py)
- [Upstream `test_usage.py`](https://github.com/MoonshotAI/kimi-cli/blob/main/tests/ui/test_usage.py)
- [Upstream `platforms.py`](https://github.com/MoonshotAI/kimi-cli/blob/main/src/kimi_cli/auth/platforms.py)
- Observed on host, 2026-07-03: `/Users/ken/.kimi-code/config.toml`, `/Users/ken/.kimi-code/credentials/kimi-code.json`, `/Users/ken/.kimi-code/oauth/kimi-code`, `/Users/ken/.kimi-code/sessions/**/wire.jsonl`, `/Users/ken/.kimi/sessions/**/context.jsonl`, `/Users/ken/.kimi-code/logs/kimi-code.log`, `/Users/ken/.kimi/logs/kimi.log`.
