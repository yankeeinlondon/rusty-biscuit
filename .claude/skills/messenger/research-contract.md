# Provider Research Contract

Platform research is becoming typed, reviewed metadata (feature
`messenger/features/2026-09-17-research-metadata-pipeline/`). It never changes
`CapabilitySet` or delivery behavior.

- **Roster** `messenger/docs/platforms.yaml`: five platforms, seven sending
  interfaces whose `adapters` are exactly `ProviderKind::as_str()`, research-only
  companions (`adapters: []`), curated sources (cap 10 per platform), and
  explicit exclusions (email, desktop, APNs, FCM).
- **Schema v1 (frozen)** `docs/research/platforms/_schema.yaml`, with every named
  type in `_types.yaml` (shared by the roster, overrides, and
  `docs/research/implementation/_schema.yaml` mappings schemas). Rules the schema
  cannot express, and their Rust owners, are in `_rules.md`.
- **Fleet prompt** `docs/research/platforms/_fleet.md`: the shared three-pass
  instructions. Each platform document's `prompt` only delegates to it.
- **Fixtures** `lib/tests/fixtures/research/` and the passive corpus test
  `lib/tests/research_corpus.rs` (Darkmatter is a dev-dependency only).

## Gotchas

SimplifiedSchema types must be one line and regex groups need
`pattern('…')`. YAML data files need `---` fences for `md schema validate`,
but fenced files cannot be read by `biscuit_file::Yaml` (Claudine sequence
sources). Darkmatter coerces `"2000"` to a number, so strict scalars are a Rust
check.
