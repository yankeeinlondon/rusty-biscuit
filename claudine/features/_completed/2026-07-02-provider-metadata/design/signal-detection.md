# Supplemental Design: Signal Catalog Detection Semantics

> **Status:** draft for Ken's review. Refines spec.md "Signal Catalog" (taxonomy,
> detection records, runtime engine, evidence corpus) and Phase 2s.
> Ratified input: F3 (record priority; cross-record state is bespoke-by-definition).

## Record grammar (the compiled surface)

- **Path:** a restricted JSONPath subset — dot segments and numeric bracket indices
  only (`error.responseBody.code`, `choices[0].finish_reason`). No wildcards, filters,
  or recursive descent in v1. Malformed paths are generation errors (as the spec
  requires — now checkable because the grammar exists).
- **Operators:** `eq` (typed value), `in` (value set), `substring_ci`, `regex`
  (anchored RE, compiled at generate time). These four cover every existing
  hand-written detection surveyed (Claude rate-limit headers, OpenCode vocab matching
  via `contains_any_ci`).
- **Priority (F3, ratified):** every record carries `priority: u16`, unique within
  its provider×source group; the engine evaluates the group as a priority-ordered
  first-match-wins list. This expresses the OpenCode 429 cascade's ordering (cap beats
  429-sentinel beats overload beats plain rate-limit) declaratively.
- **`distinguish` stays prose** (documentation for humans), but the machine guarantee
  it gestures at becomes real: generation errors if two records in one provider×source
  group have match-term sets where one subsumes the other with no differing term and
  no priority separation. Overlap is caught mechanically; prose never load-bears.

## The declarative/bespoke boundary (F3, ratified)

**Single-payload matching is declarative; anything needing cross-record or temporal
state is bespoke.** Applied to the existing inventory:

| Existing extraction | Disposition |
| --- | --- |
| Claude rate-limit headers + `resolved_reset_at()` field-fallback (`protocol/claude.rs`) | declarative — fallback expresses as two records with priorities |
| OpenCode 429/usage-cap vocab cascade (`stream/logs/opencode/errors.rs::classify_llm_failure`) | declarative — the five branches become priority-ordered records; the envelope-vs-responseBody *shape walk* stays a bespoke locator helper if the path grammar can't reach it (decide per record at migration) |
| Retry-exhaustion counting | bespoke (cross-event state) |
| Stalled-generation backstop (`stream/progress.rs`) | bespoke, permanently |
| Runaway guards (`runaway/`) | bespoke, permanently |

Temporal guards ARE named in the taxonomy (`generation_stalled`, `runaway_repetition`,
`runaway_volume`, …) with `detection: bespoke` recorded — so the catalog documents
them, `signals check` covers them, and consumers see one vocabulary.

**Typed-event binding is glue mode = bespoke** in v1. Records bind to raw JSON only;
where a provider's stream is already deserialized, a small bespoke shim re-emits. This
keeps the compiled grammar closed; revisit only if shims proliferate.

## Sink event type and consumer contract

- The sink emits a **`SignalEvent`** enum defined in `claudine/catalog-types` (F1) —
  taxonomy-typed payloads with explicit `unit`/`zone`.
- `LogClassification`/`ProviderLimitKind` become **consumer-side projections** of
  `SignalEvent`: policy facts (transient vs terminal, retry-worthiness) live in the
  projection where they are universal per taxonomy kind (e.g. `usage_capped` ⇒
  terminal), and in the consumer where they are situational. Existing consumers
  migrate by swapping their input type, not their logic.
- **Multi-source dedup:** the sink dedups on (session, signal kind, correlation
  window); first source wins, duplicates increment an occurrence counter on the
  emitted event. This handles the spec's own stream+JSONL double-observation example.

## Version-range selection at runtime

When the running provider version is unknown (the common case), the engine evaluates
the **union** of version-scoped records, priority as tiebreak. `provider_version` is
itself a taxonomy signal; once observed in-session, subsequent selection narrows to
records whose bounds admit it. No version probing is added to the wrapper.

## `signals check` semantics

- **Coverage:** every record — declarative AND bespoke — requires ≥1 positive fixture
  (`evidence: file(eager)`, fixing the spec's plain-`file` slip). Bespoke is checkable
  because bespoke emits through the same sink: the harness feeds the fixture through
  engine + bespoke registry and asserts the emitted `SignalEvent`.
- **Negative assertions:** for every overlapping record pair (same provider×source
  group), each record's fixtures are asserted NOT to trip the other — the
  `distinguish` promise, mechanically enforced.
- **Fixture home + posture:** fixtures live in the repo under the signals topic;
  `signals check` is a dev/CI-facing subcommand requiring a checkout (documented as
  such). The shipped binary carries only the compiled `&'static` tables.

## Shared vocab enums

`unit` / `zone` / `confidence` live canonically in `catalog-types` (F1). Sidecar
schemas carry mirrored member lists; a test regenerates/verifies sidecar members
against the Rust variants (three-copy drift closed). Amendments folded in:
`unit` gains `unix_nanos` (spike finding the spec missed); `confidence` ordering is
`source_code > observed > documented > inferred` (placing `observed`, which the spec's
prose ranking omitted).

## Harvest (scoped down for v1)

The "signal-adjacent shapes" recognizer implied by the spec is a second, looser
catalog nobody has designed. v1 harvest captures only **error/warning-class events
that matched no detection record** — no shape heuristics. Scrub rules co-located with
the protect catalog (as decided); retention capped (size + age) under
`~/.claudine/harvest/`. The looser recognizer is future work, gated on real gaps.
