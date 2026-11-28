# `agent-errors` ↔ `signals` topic boundary

This note records the deliberate coordination between the `agent-errors`
(rendering/summary vocabulary) and `signals` (wire-level detection) research
topics, per spec `2026-07-11-provider-errors-as-data` D9. It is the
`agent-errors` counterpart to `signals/_overlap-exclusions.yaml`.

## The boundary

The two topics touch the same provider error text but own different layers, and
neither should encode the other's records:

| Topic | Owns | Artifact | Record type |
|---|---|---|---|
| `signals/` | **Detection** — wire-level payloads that fire a `SignalKind` (usage caps, rate-limit extraction, exit-code mapping) | `signals/generated.rs` | detection records (`match_path` / `match_op` / `match_value`) |
| `agent-errors/` (this topic) | **Rendering/summary** — the `SemanticErrorKind` classification of an error's kind/message text | `stream/providers/vocabulary.rs` | ordered keyword/code buckets (needles) |

Rule: when research for one topic surfaces a record that belongs to the other,
**cite it, do not duplicate it**. `agent-errors` documents carry the citation in
prose (e.g. codex.md's Rate-Limit and Capacity sections cite
`signals/codex.md`); they never re-encode a detection record as a needle.

## Codex overlap (pilot, B2)

The signal-assurance workstream researched Codex's overload/usage-cap surface
first (`signals/codex.md`, `signals/_overlap-exclusions.yaml`). The relevant
shared surface:

- **`you've hit your usage limit`** — Codex's `UsageLimitReachedError` copy.
  Owned by `signals/` as the `usage_capped` **detection** records
  (`stream-usage_capped-turn_failed-message`, `stream-usage_capped-error-message`).
  `agent-errors/codex.md` does **not** add this as a needle; the seeded
  `rate limit` needle already renders the `api_remote` summary family, and the
  detection concern stays in `signals/`.
- **`error_type: rate_limit`** — owned by `signals/` as the `rate_limited`
  detection record. Not an `agent-errors` needle.
- **`overloaded` (OpenAI 503)** — proposed by `agent-errors/codex.md` as a
  *rendering* needle (message branch, `api_remote`). This is a
  `SemanticErrorKind` classification concern, not a `SignalKind` detection
  record, so it lives here and not in `signals/`. No overlap exclusion is
  needed — the two artifacts classify different things about the same prose.

## Why no `_overlap-exclusions.yaml` here

`signals/_overlap-exclusions.yaml` exists because the signals check replays
evidence fixtures and must declare benign cross-record firings. The
`agent-errors` deterministic gate (`claudine-gen agent-errors check`) does not
replay fixtures across providers — its checks are per-document hygiene rules
(seed preservation, provenance, motivating-class). There is therefore no
fixture-overlap surface to exclude; the coordination is purely the
cite-don't-duplicate boundary documented above. If a future check does replay
cross-topic fixtures, add a machine-readable exclusions file mirroring the
signals precedent.
