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

## Refresh and review (`research::refresh`)

- **Commands**: `prepare` (selection + limits + run dir + printed Claudine
  commands; `--dry-run`, `--resume RUN_ID`), `check-run RUN_ID [--through
  validation|review]` (the sequence's `shell:` steps), `runs`, `promote
  RUN_ID… --approved-by NAME | --renewal`, `reject`, `cleanup [--apply]`.
  Messenger never spawns Claudine; `prepare` prints `claudine budget init …`
  and `claudine sequence --yolo --budget-ledger …`.
- **Run layout** `messenger/.research-state/runs/<platform>/<run_id>/`:
  `run.json`, `budget.json` (Claudine's), `run.md`, `inputs/{discovery,
  reconcile,sources,review}/`, `outputs/{discovery.md, suggested-sources.json,
  source-proposal.json, evidence-review.json}`, `candidate/<platform>.md` (with
  schema copies so `md schema validate` works there), `source-checks.json`,
  `validation.json`, `delta.json`. Renewal records go to `renewals/`.
- **Selection reasons**: `missing`, `expired`, `forced`, `schema_invalid`,
  `prompt_changed`, `schema_changed`, `no_review_record`, `version_changed`
  (`--observed-version IFACE=VER`). What a document was "researched under"
  comes from the newest published review record or local renewal record; a
  generate-only snapshot has neither, so every platform reads
  `no_review_record`.
- **Integrity rules** (in `check`, beyond validation): same platform, same
  `created`, every chronology ID kept, removed IDs listed in `changes`,
  `last_updated` moves only with a successful check, and a curated source's
  `retrieved` changes only with a successful check on that exact date.
- **Approval**: `approval::decide` → `Ineligible` / `Renewal` /
  `HumanRequired`. Renewal = frontmatter equal after dropping `last_updated`,
  `agent`, `model`, and `sources[*].retrieved`; same body hash; no curated
  change; same prompt and schema fingerprints as the last review/renewal; no
  inaccessible check; every URL source rechecked on its recorded date.
- **History**: a human promotion adds `docs/research/reviews/{date}-{platform}-
  {run_id}.json` to the same publication; `generate` carries every review
  record forward and renders `CHANGELOG.md` from them (rule `SR-REVIEW`).
- **Initial baseline**: publication needs every active platform, so the first
  promotion passes all five run IDs to one `promote`. One run alone is refused
  (`GenerateError::Refused` with `missing`) and nothing is written.
- **Recovery**: `prepare --resume` reruns from the first incomplete stage
  (reconcile when validation failed), at most twice, with the same ledger. It
  refuses an exhausted, suspended, interrupted, or active ledger until the
  operator acts in Claudine.
- **Effective status** (`state::apply_ledger`, applied by `runs`, selection,
  `check-run`, `resume`, `promote`, and `reject`): an `active` run takes the
  ledger's resting state. `exhausted` → exhausted, `interrupted` →
  interrupted, and `stopped` → `failed` **only when the ledger's `runs`
  exceeds `RunRecord::ledger_runs`**, the count recorded at prepare/resume. A
  sequence that stopped before `check-run` decided (a failed step, Ctrl+C, or
  `AgentResolutionFailed`) is resumable or rejectable, not stuck `active`. A
  ledger still `stopped`/`initialized` (never launched, or Claudine rejected
  the arguments before opening a run) leaves the run `active`: its printed
  `claudine sequence` command is still the way forward.
- **Recipes** (`messenger/justfile`): `research-validate|generate|check|
  report|runs|cleanup` wrap the offline commands; `research-publish NAME
  RUN…` is `promote --approved-by`. `research-refresh SECONDS INVOCATIONS
  [prepare args] [-- sequence args]` builds both CLIs, puts the target's
  `debug/` first on PATH (via `cygpath -u` on Git Bash), runs `prepare
  --json` from the repository root, then each run's `budget init` and
  `sequence` in turn. It continues past a failed platform and exits 1 at
  the end. Needs `jq`; bash 3.2-safe (no `mapfile`). Workflow docs:
  `messenger/docs/user-guide.md#provider-research`.
- **Launching a live run** (checked 2026-09-18 on the shipped roster):
  - Put this worktree's `target/debug` first on PATH. An older installed
    `claudine` has no `budget` subcommand, and the `shell:` steps call
    `messenger research check-run`.
  - `run.md` has no `agent` hint. With no terminal, `claudine sequence` fails
    with `AgentResolutionFailed` before the first step. With a terminal, it
    opens a picker, and that wait is charged to the budget. Pass a provider
    flag (for example `--claude`) to the printed sequence command, or after
    `--` to `just research-refresh`.
  - `sequence --dry-run` still runs the `shell:` steps. `check-run` then marks
    the run `failed` because it has no outputs. Rehearse only in a throwaway
    Git repository holding a copy of `messenger/docs`, never on a real run.

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

`delta::compare` asserts both documents describe one platform; a worker can
write another platform's document into the candidate slot, so `check` skips
the delta on a platform mismatch and reports it as a finding. A candidate
whose frontmatter does not parse is a validation finding, never a hard error.

`promote` finishes a run whose candidate is already the published document
(an interrupted publication rolled forward by `recover`) instead of refusing
it as a stale baseline.
