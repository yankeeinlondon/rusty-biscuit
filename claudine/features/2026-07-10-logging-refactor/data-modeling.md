# Data Modeling

The canonical data model — the CRDT document taxonomy (fact logs / state registers / multi-writer), the single-writer rule, the ephemeral presence layer, document addressing, and the DuckDB star-schema projection — lives in the shared design doc:

→ [rendezvous data model](../../rendezvous/docs/crdt.md) (RATIFIED 2026-07-12)

## Canonical Log Envelope (`ClaudineAgenticLog`) — RATIFIED 2026-07-12 (D1)

The **lean envelope**: the POC `Entry` core plus typed optional context fields that
reports filter on. Everything provider-specific rides in `metadata`; a field is
promoted to a typed column only when a report actually needs it.

| Field | Type | Notes |
|-------|------|-------|
| `sequence` | `u64` | per-session monotonic, daemon-assigned |
| `created_at_unix_ms` | `i64` | producer's claimed event time |
| `received_at_unix_ms` | `i64` | daemon clock at ingestion (S5 clock-skew mitigation: reports choose which one orders) |
| `source` | `string` | producer id (`claudine-cli`, `agent-tail`, `process-monitor`) |
| `level` | `string` | severity |
| `event_kind` | `string` | e.g. `session_start`, `tool_call`, `hook_event`, `log_line`, `process_observed` |
| `session_id` | `option<string>` | whatever identifier the producer natively has (D3: no write-time merging; correlation happens in the projection) |
| `agent` | `option<string>` | provider slug |
| `model` | `option<string>` | |
| `repo` | `option<string>` | canonical remote-URL form (S4: one shared canonicalization) |
| `message` | `string` | human-readable body |
| `metadata` | `json` | everything else, per producer |

Notes:

- `received_at_unix_ms` is the one addition beyond the ratified preview's spirit of
  "fields reports filter on" — it was already called for by spec S5 (cross-host clock
  skew) and costs one column.
- The DuckDB `session_entries` fact table mirrors these fields as typed columns plus
  `metadata_json`; the `UNIQUE(chunk_id, sequence)` idempotency key is unchanged.
- Correlation identifiers (D3): each producer fills `session_id` with what it natively
  knows (Claudine wrapper session id, provider conversation id, or none for a raw
  process observation, which instead carries `pid`/`process_started_at` in `metadata`).
  Stitching happens in the projection layer — see spike S3 in [spec.md](./spec.md).
