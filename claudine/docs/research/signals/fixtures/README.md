# Signal-Detection Fixture Corpus

Evidence corpus for the provider-metadata signal catalog (spec.md "Evidence corpus and
mechanical verification"; design ruling in
`features/2026-07-02-provider-metadata/design/signal-detection.md`).

- Detection records in the per-provider signals research docs cite these files via
  `records[].evidence` (`file(required)` — SimplifiedSchema rejects `eager` inside
  inline-object rows, so fixture existence is enforced by `claudine signals check`
  instead; see the note in `../_schema.yaml`).
- `claudine signals check` replays every fixture through the **production** signal
  engine: each record's `match` must fire on its evidence fixture and each `extract`
  path must produce a value of the declared unit/type. Overlapping records in one
  provider-by-source group are also asserted NOT to trip each other's fixtures.
- The same lines double as test vectors for the hand-written stream/log parsers —
  one corpus, two consumers.

Signal-kind names below follow the canonical `SignalKind` taxonomy in
`claudine/catalog-types/src/signal.rs` (`provider_version`, `usage_capped`,
`model_resolved`, `generation_retried`, ...).

## Curation rules

1. **Scenario-scoped.** One signal story per file, not bulk session dumps. Stream
   payloads are `.jsonl` (one JSON object per line); stderr logs are `.txt`.
2. **Byte-faithful.** A fixture line is exactly what the parser sees. Lines are never
   reformatted, re-serialized, or prettified. The only permitted mutation is scrubbing
   (below), and every scrub is recorded in the provenance table.
3. **Scrubbed.** No API keys/tokens, no personal home-directory paths, no emails, no
   personal free-text content. Home paths are rewritten `/Users/ken` -> `/Users/user`
   when a line is otherwise worth keeping. Random session/message identifiers
   (`ses_*`, UUID `session_id`s, `chatcmpl-*`) from already-committed test data are
   retained: they are opaque identifiers carrying no personal information, and
   altering them would break byte-fidelity for no privacy gain.
4. **Human-reviewed before commit.** Every file in this tree was line-verified during
   curation (2026-07-05 seed pass); future additions (including harvest promotions
   from `~/.claudine/harvest/`) require the same review.
5. **No fabrication — provenance is first-class.** Every fixture carries exactly one
   of four provenance classes, recorded in [`provenance.yaml`](provenance.yaml) (the
   CI-enforced machine record; `gen/tests/fixtures_provenance.rs` asserts the
   file↔entry bijection, the class vocabulary, and a non-empty source per entry):
   - `capture` — real wire/live-run bytes from an actual provider session. New
     capture-class candidates arrive via the opt-in unmatched-event harvest
     (`harvest_unmatched` config flag / `CLAUDINE_HARVEST` env), which files
     scrubbed candidates under `~/.claudine/harvest/`.
   - `test_vector` — originates from claudine's committed parser test data (inline
     Rust test payloads unescaped to the real wire line, committed fixture files, or
     curated lines from committed session captures — the seed pass).
   - `source_shape` — synthesized byte-shape verified **verbatim** against pinned
     provider source. Legitimate ONLY when verbatim-verified against a pinned
     commit/tag AND labeled as `source_shape` in the manifest.
   - `docs_example` — verbatim from official provider documentation.

   Never author payload bytes from memory — a fixture you typed is not evidence.
   Shapes that cannot be evidenced under one of the four classes are recorded as
   gaps to capture live, never synthesized.

Sources below are relative to the repo root; `errors.rs` means
`claudine/lib/src/stream/logs/opencode/errors.rs`, `protocol/` and `providers/` mean
`claudine/lib/src/stream/protocol/` and `claudine/lib/src/stream/providers/`.
Line numbers are as of the 2026-07-05 seed pass.

## claude/ (9 files)

| Fixture | Source | Signal(s) evidenced | Scrub |
| --- | --- | --- | --- |
| `rate-limit-throttled-message.jsonl` | protocol/claude.rs:676 (also providers/claude.rs:1321,1920) | rate limited (throttled, `retry_after_ms`, message) | clean, verified |
| `rate-limit-info-approaching.jsonl` | protocol/claude.rs:694 (also providers/claude.rs:1341) | rate-limit warning (`rate_limit_info` nested form, `status: approaching_limit`, nested `resetsAt`, `rateLimitType: usage`) | clean, verified |
| `rate-limit-info-allowed-warning-seven-day.jsonl` | providers/claude.rs:1386 | rate-limit warning (`status: allowed_warning`, `rateLimitType: seven_day`) | clean, verified |
| `rate-limit-not-throttled.jsonl` | providers/claude.rs:1421 | negative case: `is_throttled: false`, no warning emitted | clean, verified |
| `error-billing.jsonl` | protocol/claude.rs:604 | billing error (top-level `error` envelope) | clean, verified |
| `assistant-error-rate-limit.jsonl` | protocol/claude.rs:616 | rate limited (`assistant.error` envelope variant) | clean, verified |
| `billing-error-synthetic-result.jsonl` | lib/tests/fixtures/providers/claude.ndjson:6-7 (live capture) | billing error via synthetic assistant (`model: "<synthetic>"`, `error: billing_error`) + `result` with `is_error: true` and zeroed usage | clean, verified (session UUIDs retained, see rule 3) |
| `result-usage-cost-fields.jsonl` | protocol/claude.rs:628,651 | token usage + cost on `result`; both cost spellings (`cost_usd` legacy line 1, `total_cost_usd` line 2 — fallback expresses as two priority-ordered records) | clean, verified |
| `init-model.jsonl` | protocol/claude.rs:502 | model resolved (`init` with `model`, `apiKeySource`) | clean, verified |

## codex/ (7 files)

| Fixture | Source | Signal(s) evidenced | Scrub |
| --- | --- | --- | --- |
| `error-rate-limit.jsonl` | protocol/codex.rs:774 (also providers/codex.rs:1057) | rate limited (top-level `error`, `error_type: rate_limit`) | clean, verified |
| `turn-failed-usage-limit.jsonl` | providers/codex.rs:1128,1133 | usage capped ("You've hit your usage limit."); the `turn.failed` + `error` pair also evidences duplicate-terminal-error dedup | clean, verified |
| `stream-error-network.jsonl` | protocol/codex.rs:786 | transport/stream error (`stream.error`, `type: network`) | clean, verified |
| `turn-completed-usage.jsonl` | protocol/codex.rs:758 | token usage (`turn.completed` with `cached_input_tokens`, `duration_ms`) | clean, verified |
| `turn-completed-usage-live.jsonl` | lib/tests/fixtures/providers/codex.ndjson:64 (live capture) | token usage, live field shape (no `duration_ms`/`status`) | clean, verified |
| `thread-started-docs.jsonl` | developers.openai.com/codex/noninteractive (official JSONL example) | session resumable (`thread.started.thread_id`) | clean, verified |
| `turn-completed-usage-reasoning-docs.jsonl` | developers.openai.com/codex/noninteractive (official JSONL example) | token usage, current documented `reasoning_output_tokens` field | clean, verified |

## gemini/ (6 files)

| Fixture | Source | Signal(s) evidenced | Scrub |
| --- | --- | --- | --- |
| `init-model-resolved.jsonl` | lib/tests/fixtures/providers/gemini.ndjson:1 (live capture) | model resolved (`init` with `model: auto-gemini-3`, timestamp) | clean, verified (session UUID retained) |
| `error-warning-loop.jsonl` | protocol/gemini.rs:279 (also providers/gemini.rs:671) | non-fatal warning (`severity: warning`, loop detection) | clean, verified |
| `error-fatal.jsonl` | providers/gemini.rs:686 | fatal error (`severity: fatal`) | clean, verified |
| `result-turn-limit.jsonl` | protocol/gemini.rs:327 (line 1), providers/gemini.rs:730 (line 2) | turn limit reached; both observed error-type spellings (`FatalTurnLimitedError`, `FatalTurnLimited`) | clean, verified |
| `result-usage-stats.jsonl` | protocol/gemini.rs:290 | token usage (`result.stats`: totals, cached, duration, tool calls) | clean, verified |
| `result-usage-per-model-live.jsonl` | lib/tests/fixtures/providers/gemini.ndjson:78 (live capture) | token usage with per-model breakdown (`stats.models{...}`) — also evidences which models actually served the session | clean, verified |

## goose/ (8 files)

Goose has no stream parser today — Claudine integrates it hook/adapter-only — so its
corpus had no seed-pass rows. The fleet pass added 8 `source_shape` fixtures,
re-derived 2026-07-06 per the provenance ruling (the originals were fabricated and
replaced) against `block/goose` commit `65eed515559af22dde2ba965335e331422f60c26`.
Per-file provenance lives in `provenance.yaml` and the detection records in
`../goose.md`; no per-file table is kept here. Live captures should supersede these
via harvest.

## kilo/ (12 files)

Fleet-added, all `source_shape`: byte-shapes verified against pinned Kilo Code source
`Kilo-Org/kilocode` commit `1fc8f066fd263455d77fed269a8bcfcd57309a55` (package
version 7.4.1). Per-file provenance is in `provenance.yaml`; the detection records in
`../kilo.md` carry the signal mapping.

## kimi/ (7 files)

| Fixture | Source | Signal(s) evidenced | Scrub |
| --- | --- | --- | --- |
| `wire-auth-expired.jsonl` | protocol/fixtures/kimi/wire-auth-expired.jsonl (verbatim copy) | provider version (init `protocol_version: 1.9`, server 1.38.0) + auth expired (JSON-RPC error `-32004`) | clean, verified |
| `wire-protocol-110.jsonl` | protocol/fixtures/kimi/wire-protocol-110.jsonl (verbatim copy) | provider version (protocol 1.10, server 1.47.0), generation retried (`StepRetry` with `error_type`, `status_code: 500`, backoff), token usage (`StatusUpdate` with `token_usage` + `mcp_status`), notification (`task.completed`) | clean, verified |
| `wire-cancelled-interrupt.jsonl` | protocol/fixtures/kimi/wire-cancelled.jsonl lines 2,3,82,94,95,96 (curated subset) | interrupt/cancellation (cancel result, `TurnEnd`, prompt result `status: cancelled`) | clean, verified; line 1 of the source (init dump embedding the capturing user's full skill catalog) deliberately excluded |
| `status-update-token-usage.jsonl` | protocol/fixtures/kimi/wire-greet.jsonl line 53 | token usage (`StatusUpdate`, `mcp_status: null` variant — complements the populated `mcp_status` in `wire-protocol-110.jsonl`) | clean, verified |

Fleet-added `source_shape` fixtures (verified verbatim against `MoonshotAI/kimi-cli`
commit `2c34efb`, tag 1.48.0 — see `provenance.yaml`):

- `wire-max-steps-reached.jsonl` — `source_shape` (wire typed models)
- `wire-question-request.jsonl` — `source_shape` (wire typed models)
- `step-retry-rate-limit.jsonl` — `source_shape` (wire typed models); a genuine
  live capture is still wanted (see Gaps)

`wire-greet.jsonl`, `wire-subagent.jsonl`, and `wire-tool-shell.jsonl` were evaluated
and skipped as tool/content traffic: their only signal-shaped lines are the init
version announcement (already covered twice above, and their init lines embed the
capturing user's skill catalog) and `StatusUpdate` (extracted separately).

## opencode/ (17 files, `.txt` stderr-log lines)

OpenCode changed its stderr stream-error format in 1.17.8: the legacy format is
`ERROR <ts> +<ms> service=llm ... error={<JSON envelope>}`; the 1.17.8 format is
`timestamp=<iso> level=ERROR run=<id> message="stream error" ... error.error="<flat string>"`
(dotted key, flat-string payload, no `service=` tag). Both formats are represented.
The five `classify_llm_failure` branches (errors.rs:323) map as noted below.

| Fixture | Source | Signal(s) evidenced | Scrub |
| --- | --- | --- | --- |
| `usage-cap-legacy-retry-wrapped.txt` | lib/tests/fixtures/logs/opencode-rate-limit.txt (verbatim copy) | branch 1: usage capped (legacy format, ZAI code 1308, reset time, retry-wrapped envelope, full `data`/`lastError` shape) | clean, verified (ses_* id retained) |
| `stream-error-legacy-usage-cap.txt` | errors.rs:875 | branch 1: usage capped (legacy format, minimal envelope, reset time) | clean, verified |
| `stream-error-1178-usage-cap.txt` | errors.rs:903 | branch 1: usage capped (**1.17.8 format**: `message="stream error"`, dotted `error.error` flat string, reset time) | clean, verified (ses_* id retained) |
| `usage-cap-kimi-403-billing-cycle.txt` | errors.rs:1493 | branch 1: usage capped, Kimi dialect (HTTP 403 `permission_error`, "billing cycle" phrasing — real status code preserved, not a 429 sentinel) | clean, verified |
| `usage-cap-exceeded-quota.txt` | errors.rs:1512 | branch 1: usage capped (`exceeded_current_quota_error` vocabulary) | clean, verified |
| `usage-cap-wins-over-retries.txt` | errors.rs:1552 | branch 1 beats branch 2: cap and retry-exhaustion markers on one line -> usage capped (priority-ordering evidence) | clean, verified |
| `429-retry-exhausted.txt` | errors.rs:1468 | branch 2: retries exhausted (`AI_RetryError`/`maxRetriesExceeded` wrapping a 429, no cap vocabulary) | clean, verified |
| `usage-cap-advisory-no-error-tag.txt` | errors.rs:1531 | branch 3: advisory cap phrase without an error tag -> non-fatal ApiFailure | clean, verified |
| `429-overload.txt` | lib/tests/fixtures/logs/opencode-429-overload.txt (verbatim copy; same line at errors.rs:1430) | branch 4: overloaded (429 + "engine is currently overloaded") | clean, verified |
| `429-plain-rate-limited.txt` | errors.rs:1449 | branch 5: plain 429 -> transient rate limited | clean, verified |
| `stream-start-1178.txt` | errors.rs:937 | negative twin for the 1.17.8 group: `message=stream` (INFO) must classify as LLM-call start, never as a stream error — the `stream` vs `"stream error"` message split is the only discriminator | clean, verified |
| `auth-failure-invalid-key.txt` | errors.rs:1018 | auth failure (`AuthenticationError`, invalid API key) | clean, verified |
| `api-failure-500.txt` | errors.rs:958 | generic API failure (500, legacy format) — outside the five 429/cap branches | clean, verified |
| `error-new-format-inline-json.txt` | errors.rs:1304 | generic API failure (500) in the new `timestamp=`/`service=` format with inline JSON `error=` tag | clean, verified |
| `uncaught-error-fatal.txt` | lib/tests/fixtures/logs/opencode-uncaught-error.txt (verbatim copy) | uncaught fatal runtime error (`fatal` keyword, stack trace, ANSI user-facing banner line) | clean, verified (`/$bunfs/...` bundler paths, not personal) |
| `version-announcement.txt` | lib/tests/fixtures/logs/opencode-new-format-lifecycle.txt:1 and opencode-new-format-serviceless.txt:1 | provider version (`version=1.14.48` boot banner) in both new-format dialects (`service=default` and serviceless `run=`) | clean, verified |
| `config-asset-load-failed.txt` | lib/tests/fixtures/logs/opencode-malformed-assets.txt:1,5,6 | config asset load failure (command/skill/agent variants, ENOENT) | **scrubbed**: `/Users/ken` -> `/Users/user` |

`opencode-mixed.txt` was evaluated and skipped: its signal lines duplicate fixtures
above (uncaught fatal, legacy usage cap, asset load failure) and it carries a personal
home path; the mixed-format interleaving it exercises is parser robustness, not a
signal story.

## pi/ (15 files)

Fleet-added, all `source_shape`: byte-shapes verified against pinned
`earendil-works/pi` commit `2e4ad6a09423002f58b9a5dc2749f7db7929d0f0`. The two auth
fixtures (`stream-auth-invalid-no-api-key.jsonl`, `stream-auth-invalid-oauth.jsonl`)
and `exit-no-models-available.json` carry verbatim source message templates;
`stream-usage-capped-quota.jsonl`, `stream-no-funds-billing.jsonl`,
`stream-rate-limited-message.jsonl`, `stream-provider-overloaded-message.jsonl`, and
`rpc-extension-ui-request.jsonl` carry exemplar (non-verbatim) provider message text
inside a verified event shape. Per-file provenance is in `provenance.yaml`; the
detection records in `../pi.md` carry the signal mapping.

## qwen/ (8 files)

| Fixture | Source | Signal(s) evidenced | Scrub |
| --- | --- | --- | --- |
| `error-rate-limit.jsonl` | providers/qwen.rs:511 | rate limited (`error` envelope, `type: rate_limit`) | clean, verified |
| `init-model.jsonl` | protocol/qwen.rs:286 | model resolved (`init` shape) | clean, verified |
| `system-session-start-model.jsonl` | protocol/qwen.rs:296 | model resolved (`system`/`session_start` shape — second spelling of the same signal) | clean, verified |
| `result-usage.jsonl` | protocol/qwen.rs:353 | token usage (`result.usage`) | clean, verified |
| `summary-token-usage.jsonl` | protocol/qwen.rs:386 (also providers/qwen.rs:475) | token usage (`summary.token_usage` — alternate event + key spelling) | clean, verified |

Fleet-added `source_shape` fixtures (verbatim strings verified against
`QwenLM/qwen-code` v0.19.6 — see `provenance.yaml`):

- `result-auth-missing-api-key.jsonl` — `source_shape`
- `result-loop-detected.jsonl` — `source_shape`
- `system-init-model-version.jsonl` — `source_shape`

## Gaps (to capture, not synthesize)

- **qwen exit codes 53/55/130** — these failure modes bypass the `result` event
  entirely and surface as stderr text + process exit code only. No committed fixture
  exists for any of them; capturing them requires a live run (harvest or manual).
  Recorded here instead of fabricating payloads.
- **goose** — corpus is `source_shape` only (hook/adapter-only, no stream parser);
  live captures should supersede it, and the error-then-`complete` taint scenario
  in particular still wants live-capture confirmation.
- **claude top-level reset spellings** — `ClaudeRateLimit` supports top-level
  `resetsAt` and `reset_at` (seconds) fallbacks (protocol/claude.rs:338-341,
  `resolved_reset_at`), but no committed test payload exercises either top-level
  spelling; only the nested `rate_limit_info.resetsAt` form has evidence. To capture.
- **claude system.init version** — the live `system`/`init` line carries
  `claude_code_version` (a provider-version signal), but the committed capture
  (claude.ndjson:5) embeds personal paths and a full local tool/skill inventory;
  a scrubbed live capture is needed before it can join the corpus.
- **codex auth failure** — `classify_error` maps `auth_error` to a configuration
  failure, but the tests pass bare kind strings; no full auth-failure wire line
  exists. To capture. Codex also announces no model in `thread.started`, so there is
  no model-resolved fixture.
- **gemini rate limit / auth** — no rate-limit-shaped or auth-shaped payload exists
  in any gemini test or fixture. To capture.
- **kimi rate limit / usage cap** — `step-retry-rate-limit.jsonl` now covers the
  rate-limit retry shape as `source_shape` (verified against kimi-cli source), but a
  genuine live capture is still wanted, and no usage-cap JSON-RPC error has been
  captured yet.
- **opencode 1.17.8 coverage** — the dotted 1.17.8 format is only evidenced for the
  usage-cap branch and the stream-start negative (matching what the tests cover);
  retries-exhausted, overload, plain-429, advisory-cap, and auth shapes exist only in
  the legacy format. To capture post-1.17.8 examples via harvest.
