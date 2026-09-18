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
- **Typed layer** `messenger::research` (feature `research`, off by default;
  pulls in `darkmatter`, `biscuit-file`, `biscuit-hash`, `serde_path_to_error`).
  `Loader` binds `$schema` to the shipped schema for the file kind, validates
  through Darkmatter's library, version-gates, then deserializes the *authored*
  frontmatter; `validate_document` runs the `SR-*` rules in `Scope::Fragment`
  (fixtures, candidates) or `Scope::Accepted` (baselines: full roster, only
  investigated gaps). Only a `ValidatedDocument` yields executable constraints
  (`eligibility()`); `assess::evaluate` decides fingerprint reuse.
- **Fixtures** `lib/tests/fixtures/research/`; `lib/tests/research_corpus.rs`
  runs the corpus through Darkmatter and the typed rules;
  `lib/tests/research_validation.rs` pins targeted behavior with variants
  written into a temp workspace that copies the shipped schemas.

## Gotchas

SimplifiedSchema types must be one line and regex groups need
`pattern('…')`. YAML data files need `---` fences for `md schema validate`,
but fenced files cannot be read by `biscuit_file::Yaml` (Claudine sequence
sources). Darkmatter coerces `"2000"` to a number, so strict scalars are a Rust
check.

Resolve schemas once: `DarkmatterSchemas::validate` re-resolves imports per
document (~100 ms debug), which pushed corpus tests past nextest's 30 s
termination under full-suite load. `Loader` caches the effective schema per
contract kind; test helpers cache by canonical `$schema` path.

Semantic fixtures are held to "every finding carries the expected rule", which
exposed several Phase 2 fixture defects (duplicated matrices, dangling IDs).
When adding a fixture, run the typed corpus test, not only `md schema
validate`. Header conventions: `# expect-rule`, `# validate-scope: accepted`,
`# expect-ineligible: <fact> <reason>`.
