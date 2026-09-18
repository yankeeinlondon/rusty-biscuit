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

## Publication and consumers

- **Snapshot** = the five accepted documents, `docs/research/platforms/catalog.json`,
  and the generated region of `docs/research/summary/platforms.md`, selected by
  the committed manifest `docs/research/publication.json` (`research::publish`,
  a port of the Phase 1 manifest + journal spike). Journal and staging live in
  the gitignored `messenger/.research-state/publication/`.
- `research::generate::{generate, check}` drive it: fleet load →
  `Fleet::validate` (Accepted scope) → `Fleet::snapshot` → `publish`. With a
  published snapshot, documents come **only** from verified bytes or explicit
  `updates` (partial refresh); without one, the fixed paths are the initial
  baseline. Refusal, interruption, or a pending journal leaves the previous
  snapshot selected; writers refuse with `RecoveryRequired` until `recover`.
- `research::project` builds the catalog (serialization-only DTOs; sorted by
  spelled IDs; no clock, no host paths; `refresh_due` recorded as a date so
  staleness is judged by readers). `research::report` reads the published
  catalog through `CatalogView` (reason codes only, so no executable type can be
  deserialized) and stays available when today's validation fails.
  `research::delta` is the mechanical fact-level comparison with the six fixed
  flags. `report::emitted_surfaces` is Messenger's own surface list per
  adapter and must change with the adapter code.
- CLI: `messenger research validate|generate [--check]|report|recover`
  (`--root`, `--today`, `--json`; exit 0 / 1 findings-drift-refusal / 2 usage /
  3 cannot run). Tests: `lib/tests/research_publication.rs` (fault injection at
  every step), `lib/tests/research_lifecycle.rs` (delta),
  `cli/tests/research_cli.rs` (real binary). Baseline fixture:
  `lib/tests/fixtures/research/lifecycle/fleet/`.

## Refresh budget (Claudine)

- A platform refresh is one `claudine sequence --budget-ledger <ledger>` run.
  Messenger cannot link Claudine (`claudine` depends on `messenger`), so it uses
  the CLI only: `claudine budget init <ledger> --run-id --platform
  --max-seconds --max-invocations [--exclusive-lock ../../fleet.lock]` (both
  limits required, no defaults), then `show --json` / `suspend` / `resume` /
  `grant`. Contract: [`claudine/docs/cli/budget.md`](../../../claudine/docs/cli/budget.md).
- Put every charged activity inside the sequence (validation and delta as
  `shell:` steps). Time outside a budgeted Claudine run is not observable.
- A successful run ends `suspended` ("awaiting human review"). Exit `76` means
  exhausted: the ledger keeps the incomplete `stage`, and this is never a
  finished or unknown result. Exit `77` means blocked: suspended, interrupted,
  or the fleet lock is held. A killed runner leaves `active`, and the next
  command recovers it to `interrupted`.
- The ledger does not persist sequence progress. A restarted sequence re-runs
  from step 1 and consumes more allowance. Resuming a candidate mid-run is
  Messenger's job (Phase 6).

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

The summary artifact is hashed by its generated region only
(`ArtifactScope::GeneratedRegions`), so maintainers may edit the prose around
it; editing inside the markers is a verification failure. Any other artifact
is whole-file. Manifest and journal paths are checked with
`paths::is_portable` before use: a hostile manifest could otherwise make
publication remove files outside the repository.

`Loader::load_document_text` judges a candidate's text *as if* it sat at the
accepted path, so a relative `$schema` resolves the way it will after
publication. Load candidates this way, not from their state-area location.

Dense `Table`s cannot render at narrow widths ("Table could not be rendered in
N columns"); the human report uses word-wrapped `UnorderedList`s instead, and
`UnorderedList` items are plain text (do not `Prose::escape_text` them).
