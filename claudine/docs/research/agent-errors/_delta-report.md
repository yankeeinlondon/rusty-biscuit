# Vocabulary Delta Report — live research vs. Phase-A seeds

Increment **C1** of spec `2026-07-11-provider-errors-as-data`. This is the
order-aware reconciliation of the live 2026-07-14 research against the
immutable Phase-A baselines. Ken accepted the complete disposition on
2026-07-14 and authorized C2/C3.

## Method

For each provider, the ordered `kind_buckets`, `msg_buckets`, and
`code_buckets` research projections were compared with
`_seeds/<slug>.yaml`. Provenance is research-only and therefore is not itself
a runtime delta. A row is classified under D8 only when its executable value,
branch, semantic kind, or position differs from the seed projection.

The deterministic gate independently verifies seed preservation, kind and
position identity, needle hygiene, provenance, invented-seed claims, empirical
capture requirements, and motivating-class coverage.

## Completeness

| Provider | Seed rows | Research rows | Additions | Disposition |
|---|---:|---:|---:|---|
| claude | 27 | 39 | 12 | accepted |
| codex | 25 | 27 | 2 | accepted |
| gemini | 24 | 30 | 6 | accepted |
| goose | 0 | 0 | 0 | research-only |
| kimi | 21 | 25 | 4 | accepted |
| opencode | 29 | 35 | 6 | accepted |
| kilo | 29 | 34 | 5 | accepted |
| pi | 18 | 36 | 18 | accepted |
| qwen | 23 | 25 | 2 | accepted |
| antigravity | 18 | 18 | 0 | unchanged |
| **Total** | **214** | **269** | **55** | **accepted** |

All 214 seeds are preserved. There are zero removals, re-kinds, reorderings,
or exact duplicates.

## Accepted additions

| Provider | Branch and kind | Appended rows |
|---|---|---|
| claude | kind · configuration | `oauth_org_not_allowed`, `invalid_request`, `model_not_found` |
| claude | message · api_remote | `server is temporarily limiting requests`, `request rejected (429)`, `is temporarily unavailable, so auto mode cannot determine` |
| claude | message · configuration | `not logged in`, `invalid api key`, `could not resolve authentication method`, `oauth token revoked`, `oauth token has expired`, `login expired` |
| codex | message · api_remote | `overloaded`, `selected model is at capacity` |
| gemini | kind · configuration | `forbidden`, `unauthorized` |
| gemini | kind · agent_native | `fatalturnlimitederror` |
| gemini | message · api_remote | `overloaded`, `resource_exhausted`, `no capacity available for model` |
| kilo | message · api_remote | `server error`, `response decompression failed` |
| kilo | message · configuration | `please reauthenticate with the copilot provider`, `unauthorized:`, `forbidden:` |
| kimi | code · agent_native | `-32000` (`INVALID_STATE`) |
| kimi | code · configuration | `-32001` (`LLM_NOT_SET`), `-32002` (`LLM_NOT_SUPPORTED`) |
| kimi | code · api_remote | `-32003` (`CHAT_PROVIDER_ERROR`) |
| opencode | message · api_remote | `server error`, `connection reset by server`, `provider response headers timed out`, `response decompression failed` |
| opencode | message · configuration | `unauthorized:`, `forbidden:` |
| pi | message · api_remote | `insufficient_quota`, `out of budget`, `quota exceeded`, `too many requests`, `service unavailable`, `server error`, `internal error`, `provider returned error`, `network error`, `connection refused`, `fetch failed`, `reset before headers`, `socket hang up`, `websocket closed`, `websocket error`, `stream ended before message_stop`, `http2 request did not get a response`, `resourceexhausted` |
| qwen | message · configuration | `no auth type is selected` |
| qwen | message · agent_native | `loop detection halted the run` |

## D8 classification

| Class | Count | Result |
|---|---:|---|
| R1 sticky-seed conflict | 0 | no seed removed or re-kinded |
| R2 evidence-backed addition | 55 | accepted |
| R3 ordering change | 0 | every row or new bucket is appended |
| R4 kind reassignment | 0 | no existing row changes kind |
| Exact duplicate | 0 | none within any branch |
| New cross-kind substring overlap | 0 | none |

Six accepted additions are same-kind refinements whose broader seed already
wins: Claude `oauth_org_not_allowed` behind `auth`, `invalid api key` behind
`api key`, and `could not resolve authentication method` behind
`authentication`; Gemini `unauthorized` behind `auth`; and Pi
`insufficient_quota` / `quota exceeded` behind `quota`. They are retained as
source-attested vocabulary without changing the winning semantic kind.

## Collision and precedence disposition

No accepted row creates a cross-kind substring overlap. New structured kinds
remain ahead of message classification, Kimi's exact codes remain ahead of all
text, and additions inside existing buckets follow every seed. OpenCode and
Kilo's `server error` message rows precede their later configuration message
buckets by the existing branch contract; mixed prose therefore keeps the
earlier `api_remote` winner.

Broad additions have representative near-miss controls. Agent-native additions
also assert explicit generated-bucket membership because their classification
result is otherwise indistinguishable from the cascade fallback.

## C1 adjudication — accepted

Ken accepted all 55 additions and the retrospective live-pilot checkpoint on
2026-07-14. C2 regenerated
`lib/src/stream/providers/vocabulary.rs` from research and added Level-1 tests
covering every accepted row, precedence, exact-code behavior, and near misses.
Unresolved `gaps` remain non-executable in the provider research documents.
