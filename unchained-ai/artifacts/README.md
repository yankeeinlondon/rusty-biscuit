# Model Catalog Artifacts

`models-catalog.json` (with its JSON Schema in `models-catalog.schema.json`) is
the **entire cross-area boundary** between unchained-ai's model catalog and
claudine: claudine's generation consumes the committed file by
workspace-relative path and never links unchained-ai code. Design:
[`claudine/features/2026-07-02-provider-metadata/design/model-catalog-boundary.md`](../../claudine/features/2026-07-02-provider-metadata/design/model-catalog-boundary.md).

The artifact carries every catalog offering with its parsed identity
(`vendor/family`, version, variants, date pin, serving tags), a normalized
identity key, per-offering metadata, a `vendor/family` index (release-ordered
members, `latest`, rolling aliases), cross-source duplicate groups, and a gap
list of identity-less ids.

## Regeneration

- `just generate-models` (manual, needs provider API keys) refreshes the
  catalog **data** — the generated enums and metadata under
  `lib/src/rigging/providers/models/`. Goal cadence: weekly.
- `just artifact` (`cargo run -p unchained-ai-gen --bin emit-catalog`)
  re-emits both files here, fully **offline** — run it after `generate-models`
  or after touching the catalog schema types or curation tables. A drift test
  in `gen/tests/catalog_drift.rs` fails CI when the committed files are stale.

## Staleness contract

`generated_at` records when the catalog data was generated (the max
`//! Generated:` header of the committed provider files), not when the
artifact was emitted. Consumers enforce a max age against it — 30 days by
default, `on_stale: warn` — because family-latest answers are only correct
relative to that timestamp.

## `schema_version` compatibility

Consumers must fail loudly on a `schema_version` mismatch. Breaking artifact
changes bump it — including curation-table changes (variant vocabulary, vendor
aliases, identity-bearing serving tags) that would move existing identity
keys.
