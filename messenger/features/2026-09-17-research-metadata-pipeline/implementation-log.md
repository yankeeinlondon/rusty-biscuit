---
spec: /Volumes/coding/wt/rusty-biscuit/feat-better-static-analysis/messenger/features/2026-09-17-research-metadata-pipeline/spec.md
plan: messenger/features/2026-09-17-research-metadata-pipeline/plan.md
implemented_by: claude/opus
started_phase: 1
source_files_during_phase_1:
    - messenger/features/2026-09-17-research-metadata-pipeline/spikes/publication/prototype/.gitignore
    - messenger/features/2026-09-17-research-metadata-pipeline/spikes/publication/prototype/Cargo.toml
    - messenger/features/2026-09-17-research-metadata-pipeline/spikes/publication/prototype/src/fault.rs
    - messenger/features/2026-09-17-research-metadata-pipeline/spikes/publication/prototype/src/fsutil.rs
    - messenger/features/2026-09-17-research-metadata-pipeline/spikes/publication/prototype/src/generations.rs
    - messenger/features/2026-09-17-research-metadata-pipeline/spikes/publication/prototype/src/journal.rs
    - messenger/features/2026-09-17-research-metadata-pipeline/spikes/publication/prototype/src/lib.rs
    - messenger/features/2026-09-17-research-metadata-pipeline/spikes/publication/prototype/src/model.rs
    - messenger/features/2026-09-17-research-metadata-pipeline/spikes/publication/prototype/tests/common/mod.rs
    - messenger/features/2026-09-17-research-metadata-pipeline/spikes/publication/prototype/tests/generations_interruption.rs
    - messenger/features/2026-09-17-research-metadata-pipeline/spikes/publication/prototype/tests/journal_interruption.rs
    - messenger/features/2026-09-17-research-metadata-pipeline/spikes/publication/prototype/tests/open_handles.rs
    - messenger/features/2026-09-17-research-metadata-pipeline/spikes/orchestration/probe_budget.py
    - messenger/features/2026-09-17-research-metadata-pipeline/spikes/orchestration/probe_tree_windows.py
docs_updated_during_phase_1:
    - messenger/features/2026-09-17-research-metadata-pipeline/spikes/orchestration/findings.md
docs_created_during_phase_1:
    - messenger/features/2026-09-17-research-metadata-pipeline/architecture.md
    - messenger/features/2026-09-17-research-metadata-pipeline/fixture-matrix.md
    - messenger/features/2026-09-17-research-metadata-pipeline/spikes/surface-inventory/inventory.md
    - messenger/features/2026-09-17-research-metadata-pipeline/spikes/schema-pilots/findings.md
    - messenger/features/2026-09-17-research-metadata-pipeline/spikes/publication/findings.md
    - messenger/features/2026-09-17-research-metadata-pipeline/spikes/orchestration/probe-results.json
    - messenger/features/2026-09-17-research-metadata-pipeline/spikes/schema-pilots/_schema.yaml
    - messenger/features/2026-09-17-research-metadata-pipeline/spikes/schema-pilots/_types.yaml
    - messenger/features/2026-09-17-research-metadata-pipeline/spikes/schema-pilots/_mappings.schema.yaml
    - messenger/features/2026-09-17-research-metadata-pipeline/spikes/schema-pilots/_mappings.types.yaml
    - messenger/features/2026-09-17-research-metadata-pipeline/spikes/schema-pilots/mappings.pilot.yaml
    - messenger/features/2026-09-17-research-metadata-pipeline/spikes/schema-pilots/discord.md
    - messenger/features/2026-09-17-research-metadata-pipeline/spikes/schema-pilots/telegram.md
    - messenger/features/2026-09-17-research-metadata-pipeline/spikes/schema-pilots/slack.md
    - messenger/features/2026-09-17-research-metadata-pipeline/spikes/schema-pilots/signal.md
    - messenger/features/2026-09-17-research-metadata-pipeline/spikes/schema-pilots/negative/
    - messenger/features/2026-09-17-research-metadata-pipeline/spikes/schema-pilots/probes/
skills_files_updated_during_phase_1:
    - .claude/skills/os/windows.md
source_files_during_phase_2:
    - messenger/lib/Cargo.toml
    - messenger/lib/tests/research_corpus.rs
    - messenger/lib/tests/fixtures/research/contract/
    - messenger/lib/tests/fixtures/research/interaction/
    - messenger/lib/tests/fixtures/research/diagnostics/
    - messenger/lib/tests/fixtures/research/negative/schema/
    - messenger/lib/tests/fixtures/research/negative/semantic/
    - messenger/docs/platforms.yaml
    - messenger/docs/platforms.schema.yaml
    - messenger/docs/research/platforms/_schema.yaml
    - messenger/docs/research/platforms/_types.yaml
    - messenger/docs/research/platforms/_overrides.schema.yaml
    - messenger/docs/research/implementation/_schema.yaml
docs_updated_during_phase_2:
    - messenger/docs/research/platforms/discord.md
    - messenger/docs/research/platforms/slack.md
    - messenger/docs/research/platforms/telegram.md
    - messenger/docs/research/platforms/whatsapp.md
    - messenger/docs/research/platforms/signal.md
    - docs/dependencies.md
docs_created_during_phase_2:
    - messenger/docs/research/platforms/_fleet.md
    - messenger/docs/research/platforms/_rules.md
    - messenger/lib/tests/fixtures/research/README.md
skills_files_updated_during_phase_2:
    - .claude/skills/messenger/SKILL.md
    - .claude/skills/messenger/research-contract.md
source_files_during_phase_3:
    - messenger/lib/Cargo.toml
    - messenger/justfile
    - Cargo.lock
    - messenger/lib/src/lib.rs
    - messenger/lib/src/research/mod.rs
    - messenger/lib/src/research/assess.rs
    - messenger/lib/src/research/canonical.rs
    - messenger/lib/src/research/diagnostics.rs
    - messenger/lib/src/research/error.rs
    - messenger/lib/src/research/load.rs
    - messenger/lib/src/research/paths.rs
    - messenger/lib/src/research/model/mod.rs
    - messenger/lib/src/research/model/common.rs
    - messenger/lib/src/research/model/document.rs
    - messenger/lib/src/research/model/roster.rs
    - messenger/lib/src/research/model/overrides.rs
    - messenger/lib/src/research/model/mappings.rs
    - messenger/lib/src/research/validate/mod.rs
    - messenger/lib/src/research/validate/identity.rs
    - messenger/lib/src/research/validate/constraints.rs
    - messenger/lib/src/research/validate/bindings.rs
    - messenger/lib/src/research/validate/interaction.rs
    - messenger/lib/src/research/validate/errors.rs
    - messenger/lib/src/research/validate/coverage.rs
    - messenger/lib/src/research/validate/replay.rs
    - messenger/lib/tests/research_corpus.rs
    - messenger/lib/tests/research_validation.rs
    - messenger/lib/tests/fixtures/research/contract/overrides-valid.yaml
    - messenger/lib/tests/fixtures/research/contract/pilot-discord.md
    - messenger/lib/tests/fixtures/research/contract/pilot-signal.md
    - messenger/lib/tests/fixtures/research/contract/pilot-telegram.md
    - messenger/lib/tests/fixtures/research/negative/semantic/sr-coverage--surface-uncovered.md
    - messenger/lib/tests/fixtures/research/negative/semantic/sr-enforceable--unit-unspecified-executable.md
    - messenger/lib/tests/fixtures/research/negative/semantic/sr-evidence--secondary-only-known.md
    - messenger/lib/tests/fixtures/research/negative/semantic/sr-gap--gap-placeholder.md
    - messenger/lib/tests/fixtures/research/negative/semantic/sr-interactivity--callback-only-claims-text.md
    - messenger/lib/tests/fixtures/research/negative/semantic/sr-mapping--adapter-platform-mismatch.yaml
    - messenger/lib/tests/fixtures/research/negative/semantic/sr-mapping--proposed-as-implemented.yaml
    - messenger/lib/tests/fixtures/research/negative/semantic/sr-mapping--unassessed-without-reason.yaml
    - messenger/lib/tests/fixtures/research/negative/semantic/sr-origin-phase--sdk-phase-certainty-mismatch.md
    - messenger/lib/tests/fixtures/research/negative/semantic/sr-override--expired-override.yaml
    - messenger/lib/tests/fixtures/research/negative/semantic/sr-override--stale-override.yaml
    - messenger/lib/tests/fixtures/research/negative/semantic/sr-ref--error-dangling-constraint.md
    - messenger/lib/tests/fixtures/research/negative/semantic/sr-roster--missing-roster-interface.md
    - messenger/lib/tests/fixtures/research/negative/semantic/sr-roster--research-only-with-adapter.md
    - messenger/lib/tests/fixtures/research/negative/semantic/sr-roster--roster-adapter-mapped-twice.yaml
docs_updated_during_phase_3:
    - messenger/docs/research/platforms/_rules.md
    - messenger/lib/tests/fixtures/research/README.md
    - messenger/lib/README.md
    - docs/dependencies.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
    - .claude/skills/messenger/SKILL.md
    - .claude/skills/messenger/research-contract.md
    - .claude/skills/os/macos.md
packages:
    - messenger
---

# Implementation Log for 2026-09-17-research-metadata-pipeline (8 phases)

## Phase 1

- Phase 1 is discovery and architecture: its outputs are spike evidence and
  design records under `spikes/` and `architecture.md`, not product behavior.
- Wave 1 tasks run as four parallel agents with disjoint output directories:
  `spikes/surface-inventory/`, `spikes/schema-pilots/`, `spikes/publication/`,
  and `spikes/orchestration/` (extended).
- Drafted `fixture-matrix.md` while Wave 1 ran: all 37 acceptance criteria map
  to named fixtures, a proof boundary, and a landing phase; every row is L1.
- Finding: `just test`/`just lint` build the `messenger` library with only
  `desktop`, so research code needs explicit `--features research` and
  `--all-features` checks (recorded in the matrix; no new CI cell — CI already
  runs `all-features`).
- Finding: `cargo tree -p messenger --all-features` already reaches
  `biscuit-file` and YAML crates through `sniff` → `schematic-*`; the
  "ordinary send build has no maintenance deps" check must therefore use
  no-default, default, and `desktop` feature sets (all clean on 2026-09-17).
- Wave 1 Surface Inventory complete → `spikes/surface-inventory/inventory.md`
  (revision `d57faf7e8`). Key outcomes: adapter IDs reuse the persisted
  `ProviderKind::as_str()` strings; interface IDs are separate snake_case
  names; `Summarized` bodies, location-with-text, and plain text behave very
  differently per adapter; `supports_location` is `true` for all seven even
  where only a text fallback exists; error detail (codes, subcodes, field
  errors, correlation IDs, success warnings) is lost in most adapters. Three
  suspected runtime defects (Slack webhook response parsing, Signal group
  method name, Telegram 429 `retry_after` loss) are recorded as research
  priorities, not fixed here — this feature must not change delivery behavior.
- Wave 1 Publication Spike complete → `spikes/publication/findings.md` and a
  standalone prototype crate (`spikes/publication/prototype/`, own empty
  `[workspace]`, not a monorepo member). Chosen: committed manifest
  `messenger/docs/research/publication.json` as the single selection point,
  local journal + staged/backup copies under gitignored
  `messenger/.research-state/publication/`, and a hash-guarded, idempotent
  roll-back/roll-forward recovery. All 17 prototype tests pass on macOS
  (re-run by the orchestrator: 2 + 9 + 6 tests OK), build-linux, and
  build-win-native. **Gap:** WSL2 not run (SSH to build-win reset twice; W:
  nearly full). The subagent could not write its report file; the orchestrator
  saved its content verbatim.
- New measured Windows fact added to `.claude/skills/os/windows.md`:
  `std::fs::rename` succeeds over a std-opened reader while
  `tempfile::persist` fails with error 5.
- Wave 1 Budget Spike complete → new section in
  `spikes/orchestration/findings.md` plus `probe_budget.py`,
  `probe_tree_windows.py`, `probe-results.json`. Built committed HEAD
  `d57faf7e8` from a `git archive` snapshot because the working tree's
  Claudine crates currently fail to compile due to another effort's
  uncommitted edits (not touched). Key outcomes: worker cwd is the repo root
  (prepared scratch cwd needed for discovery); inline-compose exposes prior
  prose by design (never use it for discovery); Unix timeout kills only the
  child's process group (a `setsid` grandchild survives; wrapper death
  orphans the worker); Windows Job Object kills the tree; lifecycle retries
  launch inside one Claudine command, so admission must hook inside
  Claudine's launch path (`execute_attempt_phase` before
  `execute_harness_attempt`); Claudine persists no run state today.
  GitNexus impact was unreliable (index mid-update by another session) —
  Phase 5 must re-run impact on a fresh index. **Gap:** WSL2 unreachable;
  native Windows evidence is OS-level (Job Object) only.
- Wave 1 Schema Pilots complete → `spikes/schema-pilots/` (pilot schema,
  four pilot documents, mappings pilot, negative corpus, probes). The subagent
  could not write `findings.md`; the orchestrator saved it and added a note
  that production IDs/fingerprints follow `architecture.md`. Independently
  re-verified with the installed `md`: the pilots exit 0, all 13
  `schema-rejects/` samples exit 1, and all 9 `schema-accepts-rust-must-reject/`
  samples exit 0 (as designed). Thirteen SimplifiedSchema limitations were
  recorded (L1–L13); the most consequential are root closure only through a
  pattern key (L1), type leakage requiring a separate `_types.yaml` (L2), no
  arrays of unions (L3–L5), and multi-line types failing to load from
  standalone files (L6, likely a Darkmatter bug).
- Wave 2 complete: `architecture.md` (IDs, module/feature graph, schema/type
  boundaries, artifacts, local state, snapshot protocol, fingerprints,
  CLI/recipes, budget boundary, cleanup rules, resolution of every spec
  planning item) and `fixture-matrix.md`. Confirmed that Darkmatter already
  exposes `DarkmatterSchemas::validate` as a library API, and that its
  `effects-instrumentation` feature can prove loader passivity.
- Decision recorded (not escalated): downtime after a crash is not charged.
  The run enters `interrupted`, an operator-resumption state, which the spec's
  active-clock ruling already excludes; time through the last heartbeat plus
  one interval stays charged, so no consumed allowance is refunded.

### Verification

- Phase 1 changes no product behavior: no workspace crate, public API,
  provider, or `CapabilitySet` changed. The requirement-to-test mapping for
  the whole feature is `fixture-matrix.md`; Phase 1's own claims are backed by
  spike evidence rather than workspace tests:
  - snapshot protocol: 17 prototype tests (interruption at every injection
    point, abort-based crashes, Windows open-handle cases), passing on
    macOS (re-run by the orchestrator), build-linux, and build-win-native;
  - schema expressiveness: `md schema validate` over pilots and negative
    samples (re-run by the orchestrator);
  - budget semantics: `probe_budget.py` on macOS and build-linux,
    `probe_tree_windows.py` on build-win-native (`probe-results.json`).
- Gates: `just test` in `messenger/` — 532 run, 532 passed, 2 skipped
  (pre-existing skips); `just lint` in `messenger/` — clean.
- Skipped / gaps: WSL2 evidence for both spikes (host unreachable); the
  Claudine binary was not run on native Windows; `just cross-check` was not
  used because no workspace package changed.
- Plan checkpoint item "obtain maintainer review" is left unchecked: it needs
  a human. `human_review: true` is set on the spec with two items (extra
  lifecycle subcommands; artifact-layout additions).
- Pre-existing: the working tree's Claudine crates did not compile during
  this phase because of another effort's uncommitted edits (not touched).

## Phase 2

- Proceeded under the architecture record's recommended option A for both
  pending human-review items (five explicit lifecycle subcommands; the layout
  additions). Phase 2 depends only on the layout item: the schema split into
  `_schema.yaml` + `_types.yaml` is unavoidable (pilot L2). No CLI surface was
  implemented, and `_fleet.md` names only the spec-required
  `messenger research validate`, so the subcommand decision stays open.
- Wave 3 **Roster Contract** → `messenger/docs/platforms.yaml` +
  `messenger/docs/platforms.schema.yaml`. Five active platforms, seven sending
  interfaces (adapter IDs = `ProviderKind::as_str()`), six research-only
  companions (`discord_gateway`, `discord_interactions`, `slack_events_api`,
  `slack_socket_mode`, `slack_interactivity`, `whatsapp_webhooks` — Telegram
  and signal-cli receive on their own interface, so they need none), and
  explicit exclusions for email, desktop, APNs, FCM. The curated-source cap is
  a schema ceiling (`max(10)` on the array, `curated_source_cap ≤ 10`); Rust
  enforces a lower configured cap (SR-CURATED). Seed curated lists come from
  URLs already cited in the prose research and the spec's spot checks, all
  marked `review: proposed` because every curated-list change needs human
  approval.
- Finding: the roster cannot be both an `md`-validated file and a raw
  Claudine sequence source. `md schema validate` only sees data inside
  `---` fences (pilot L10), but `biscuit_file::Yaml` (Claudine's sequence
  loader) rejects a fenced file as a two-document stream. Kept the fences;
  the fleet reaches roster data through Messenger's prepared per-run inputs
  (architecture "Local state area"), not by pointing `claudine sequence` at
  the roster. Recorded for Phase 6.
- Wave 3 **Metadata Schema** → `docs/research/platforms/_schema.yaml` +
  `_types.yaml` (schema version 1), `_overrides.schema.yaml`,
  `docs/research/implementation/_schema.yaml`, and `_rules.md`. One types
  file serves all four schemas (cross-directory `Name@../platforms/_types.yaml`
  imports were probed and work). Vocabulary beyond the pilot: dedicated
  `attachment_bindings`, `addressing`, `receipts`, `attribution_bindings`,
  `location_bindings`, `author_geolocation`, `expression_bindings`,
  `delivery_controls`, `eligibility`, `rate_limits`, `interaction_fixtures`;
  a per-interface `coverage` matrix with all 16 categories as required keys
  (each cell `researched` / `not_applicable` / `gap`), so an omitted category
  fails the schema and every category can express an investigated gap;
  `html_subset`; text-binding `packaging`/`visibility`; image
  `multiple_images`/`ordering`/`atomic`/`receipts`; question `native`,
  `cancellation`, `responses`; `link_button`, `free_text_interpretation`;
  form `external_form`/`sequential_application` containers; a shared
  `Lifecycle` with units in field names; operations constrained to
  snake_case; xxh64 fingerprints (`xxh64:` + 16 hex) instead of the pilot's
  SHA-256. The pilot's generic `capabilities` record is dropped.
- `_rules.md` lists 30 Rust-owned rules (the pilot's SR-* list plus
  SR-CURATED, SR-ATTRIBUTION, SR-LOCATION, SR-EXPRESSION, SR-SANITIZED,
  SR-OVERRIDE) with their planned `messenger::research` owner modules.
- Wave 4 fixtures → `messenger/lib/tests/fixtures/research/`:
  `contract/` (45: the four pilots ported to v1, the shipped roster copy,
  and one fixture per representation), `interaction/` (13), `diagnostics/`
  (6), `negative/schema/` (28, each with an `# expect-problem:` pointer), and
  `negative/semantic/` (79, schema-valid by design, stem = rule code). All
  171 checked with the installed `md`: every positive and semantic negative
  is valid; every schema negative fails at its declared pointer. Payloads use
  placeholder IDs (`USER_ID`, `MESSAGE_ID`); hosts are `*.example.com`.
- Pilots ported to v1 as `contract/pilot-{discord,telegram,slack,signal}.md`:
  interface IDs renamed to the production snake_case form, operations
  snake_cased (`sendMessage` → `send_message`, `chat.postMessage` →
  `chat_post_message`), new required fields filled conservatively
  (`unknown` or a `gap.<platform>.pilot_scope` gap), and the three generic
  capabilities converted to attribution, author-geolocation, and attachment
  records. All four pass the v1 schema.
- The fixtures were generated once by a throwaway script (not committed) and
  are now ordinary hand-maintained files. `contract/roster.yaml` was dropped:
  a copy of the shipped roster would drift, and the corpus test validates the
  shipped roster directly.
- Wave 3 **Fleet Prompt** → `docs/research/platforms/_fleet.md`: role limits,
  access policy (preauthorized versus approval-required), evidence rules,
  contract guidance, 15 category research questions, the spec's
  per-interface required investigations, and the three passes with their
  inputs and outputs (discovery report with `suggested_sources`; candidate
  plus source-check record; source-list proposal that never edits the
  roster). The five platform documents' duplicated inline prompts were
  replaced with a short delegating prompt: a single-document refresh is
  Pass 2 only, runs against a candidate copy, and never runs discovery
  through inline-compose. The prompts use repository-root paths, because a
  candidate copy lives elsewhere and relative `./_fleet.md` would break. The
  documents otherwise stay untouched legacy prose (no `$schema` until the
  Phase 7 migration).
- Not done in Phase 2, deliberately: `claudine sequence` frontmatter in
  `_fleet.md`. Running a sequence launches real agents (forbidden here), and
  its inputs depend on the Phase 5 budget ledger and the Phase 6 prepared-run
  layout.
- Passive corpus test → `messenger/lib/tests/research_corpus.rs` (14 tests)
  with `darkmatter` (`effects-instrumentation`) as a **dev-dependency only**,
  following `claudine-cli`'s precedent. It validates the shipped roster and
  every fixture through `DarkmatterSchemas::validate` (the `md schema
  validate` code path), checks the roster against `ProviderKind::as_str()`,
  checks that legacy documents delegate to the fleet, checks that every
  semantic rule code appears in `_rules.md`, proves passivity (effect-engine
  and network counters unchanged, validated bytes unchanged, including a
  `prompt` carrying `{{ shell(...) }}`), and scans the corpus for
  credentials, webhook URLs with concrete IDs, bot-token and phone shapes,
  control characters, escaped ESC sequences, and payloads over 1 KiB. The
  scanner has its own positive and negative samples so it cannot pass
  vacuously.
- Performance: Darkmatter re-resolves the schema for every document (about
  100 ms per document in a debug build). A shared `DarkmatterSchemas` did
  not help. Tests are split per directory and rule family, and the passivity
  test uses a representative sample, so the slowest test takes about 5 s,
  well inside the default 30 s termination.
- The test reads `CARGO_MANIFEST_DIR` at run time, falling back to `env!`,
  because the WSL2 CI leg runs a nextest archive whose compile-time path
  names the ubuntu-latest builder (os skill, "Red only on wsl2-ubuntu").
- Mutation checks (reverted): an unknown key in `contract/minimal-valid.md`
  failed `contract_fixtures_validate`; eleven Telegram curated sources failed
  both roster tests at the schema (`has more than 10 items`) and in Rust.

### Requirement-to-test mapping (Phase 2)

| Behavior changed | Test / evidence |
|---|---|
| Roster: 5 platforms, 7 adapters = `ProviderKind::as_str()`, companions without adapters, exclusions (criterion 1) | `shipped_roster_validates_against_its_schema`, `shipped_roster_maps_every_chat_adapter_exactly_once`; `negative/semantic/sr-roster--*.yaml` |
| Curated cap 10 and explained contributions (criterion 27) | `negative/schema/roster-cap-11.yaml`, `roster-retained-without-contribution.yaml`, `roster-cap-above-ceiling.yaml`; `contract/roster-cap-10.yaml`; `sr-curated--*` |
| Closed schema: unknown keys/enums, missing arrays/interfaces/knowledge, types, locators, incomplete matrices, version literal (criterion 6) | `schema_negative_fixtures_fail_at_their_declared_problem` (28 fixtures, each asserting its pointer) |
| Representations: field/recommended/aggregate/bytes/count/bridge-conditional bounds, units and stages, states, versions, bindings, images, attribution, location, expression, controls, gaps (criteria 4, 5, 15-19, 25, 28) | `contract_fixtures_validate` over `contract/*` |
| Interactivity (criteria 20-22) | `interaction_fixtures_validate` over `interaction/*` |
| Diagnostics (criteria 12, 13) | `diagnostic_fixtures_validate` over `diagnostics/*` |
| Rust-owned rules stay out of the schema and are documented | `semantic_negative_fixtures_*` (79), `rules_document_lists_every_rule_the_semantic_corpus_names` |
| Sanitized fixtures (criterion 14) | `research_corpus_is_sanitized`, `sanitization_scanner_rejects_known_unsafe_shapes` |
| Passive validation (Phase 2 checkpoint) | `validation_is_passive` |
| v1 freeze gate: four pilots pass and every category can hold an investigated gap | `freeze_gate_fixtures_validate` |
| Fleet delegation replaces duplicated prompts | `shipped_platform_documents_validate_or_delegate_to_the_fleet` |
| Real shipped artifacts through the normal path | `md schema validate` over the roster, all 170 fixtures, and the five documents (installed binary); the corpus test uses the same library entry point |

Phase 2 has no persisted read/write/read round trip: it writes no values
programmatically. The Phase 4 publication tests own round trips.

### Gates (macOS)

- `just test` in `messenger/`: 546 run, 546 passed, 2 skipped (pre-existing
  skips).
- `just lint` in `messenger/`: clean (covers the new test target).
- `cargo nextest run -p messenger --all-features --test research_corpus`:
  14/14.
- `cargo tree -p messenger -e normal` with `--no-default-features`, default
  features, and `--features desktop`: no `darkmatter`, `biscuit-file`, or
  YAML crate (the dev-dependency does not leak).
- `md schema validate` sweep: 0 unexpected results over 170 fixtures.

### Cross-OS evidence (`just cross-check messenger … research_corpus`)

None of the three remote legs produced a test result. All failures are host
availability problems, not test failures:

- **windows** (`build-win-native`): `scp` to `W:` failed, then `git fetch`
  failed with `No space left on device`. The `W:` volume is full (Phase 1
  noted it was nearly full). Not cleaned: freeing space on a shared host
  deletes other people's data and needs a human decision (see the
  `storage-strategy` skill).
- **wsl** (`build-win`): `kex_exchange_identification: Connection reset by
  peer`, the same SSH reset Phase 1 hit.
- **linux** (`build-linux`): queued behind a stale host lock held since
  2026-09-14T18:25Z by `reward-20260914-c3e60d0` (`nightly-reward-spike`,
  branch `feat-nightly-perf`). The script never removes a foreign lock, and
  neither did this phase; the queued waiter was stopped after about 25
  minutes.

OS-risk assessment for what Phase 2 changed: the only code is a test that
reads repository files. It uses a runtime `CARGO_MANIFEST_DIR` (for the WSL
archive leg), `Path::join` with relative segments (portable), and `str::lines`
(which also strips `\r`). `.gitattributes` forces `eol=lf`. Darkmatter's
relative `$schema` and `Name@../platforms/_types.yaml` resolution on native
Windows is the one unproven path; CI's `windows-latest` leg covers it, and
the next phase with host access should run `just cross-check messenger --os
windows research_corpus`.

## Phase 3

- Added the opt-in `research` feature to `messenger` (optional `darkmatter`,
  `biscuit-file` with only `file-reference`, `biscuit-hash`,
  `serde_path_to_error`) and the feature-gated module
  `messenger/lib/src/research/`: `model` (authored DTOs), `load` (passive
  loader), `diagnostics`, `error`, `paths` (`Workspace`, `RepoPath`),
  `canonical` (canonical JSON + xxh64), `validate` (six rule families plus
  `replay`), and `assess` (mappings and fingerprint reuse).
- Local L1 now covers the feature: `local-features = ["desktop", "research"]`
  and the package-area `test`/`sanity`/`lint` recipes build
  `messenger --features desktop,research`. Without this, `just test` and
  `just lint` would never compile the new module. CI is unchanged (it already
  runs `all-features`).
- Loader decisions:
  - One `FileResolutionContext` is captured per `Loader`, anchored at an
    explicit repository root (`FileResolutionContext::new(root)
    .with_repository_root(root)`). No sniff or Git probing, so no process
    spawn. Darkmatter shares the same context.
  - `$schema` must resolve (through `FileReference`) to the shipped schema
    for the file's kind; anything else is `SR-SCHEMA-BINDING` and schema
    validation is skipped (a document bound to another schema proves nothing).
  - Darkmatter validation problems become `SCHEMA` findings (`SR-TOP-LEVEL`
    for an unknown root key). Only a schema-clean file is version-gated
    (`SR-VERSION`: the raw value must be the integer 1) and then
    deserialized from the *authored* frontmatter, so Darkmatter's coercion
    cannot hide a quoted number or a float version (`SR-STRICT-SCALARS`
    comes from serde's `invalid type` errors, with a JSON Pointer from
    `serde_path_to_error`).
  - Host paths never reach findings: every finding carries a `/`-separated
    `RepoPath`; third-party messages are scrubbed of the repository root.
  - Manifests (`publication.json`) are not loaded yet: the manifest format
    is defined by the Phase 4 snapshot writer, which will add its reader.
- Validation scopes (the SR-GAP baseline-mode decision the Phase 2 handoff
  asked for): `Scope::Fragment` for fixtures, pilots, and candidates;
  `Scope::Accepted` for accepted research and baselines, which additionally
  requires every active roster interface, coverage and unknown facts resting
  only on investigated gaps, an image-role matrix for researched images, and
  an author-geolocation answer for researched location. Roster validation in
  `Accepted` scope also requires all five platforms and all seven adapters.
- Ruling: a known bound backed only by `secondary` sources is valid but
  ineligible (`IneligibleReason::SecondaryEvidenceOnly`), not rejected, because
  the spec says such claims "remain visible". Same treatment as
  `sr-enforceable--*`.
- Ruling: `recommended_max` is never executable (`Advisory`): no enforcing
  consumer exists for a recommendation.
- Ruling: the SR-UNIQUE / SR-APPLICABILITY scope key includes unit and
  measurement stage. 2000 Unicode scalars and 4000 UTF-8 bytes on one field
  are two simultaneous bounds, not a duplicate (`contract/units-ambiguous.md`
  relies on this).
- Ruling: `interactivity` coverage for an interface counts records on its
  related companions in either direction (Gateway, Interactions, Events API),
  plus question/form bindings naming it as `companion_interface`.
- Ruling: changes may name gap IDs as well as facts; a `removed` change must
  name an ID that is no longer present (removed IDs are never reused).
- Error replay (`validate::replay`): signatures are explicit conjunctions; a
  numeric or string native code is read at `code_locator`, and for `warning`
  outcomes also from `warnings_locator`. A discriminator may read a JSON
  Pointer beneath a defined envelope pointer (`/` is authored as the root).
  `no_match` and `unknown` both mean "no executable signature matches".
- Interaction replay: only `expect: answer` fixtures on question bindings are
  replayed (confirmation mapping, option IDs, strings). Timeout, cancellation,
  and stale-option outcomes are lifecycle facts that no payload proves.
- Fixture corrections. Holding every semantic fixture to "each finding
  carries the expected rule" exposed Phase 2 defects. The code was treated as
  correct and the fixtures were fixed:
  - `sr-coverage--surface-uncovered.md` gave both interfaces a matrix (so
    nothing was uncovered); the second matrix was removed.
  - `sr-roster--research-only-with-adapter.md` and
    `sr-interactivity--callback-only-claims-text.md` had a duplicated
    coverage matrix; `sr-roster--roster-adapter-mapped-twice.yaml` attributed
    Slack curated sources to `discord_bot_api`;
    `sr-ref--error-dangling-constraint.md` marked errors `researched` with no
    record; `sr-origin-phase--sdk-phase-certainty-mismatch.md` bound an SDK
    error to a service envelope (now an `sdk_error` envelope with an
    `sdk_error_variant` signature).
  - `pilot-discord.md`: dangling `alt_text_binding`, and
    `requires_messenger_update: true` without the required gap (added
    `gap.discord.embed_budget_preflight`). `pilot-signal.md`: operand-less
    bridge-version conditions without a gap (the pilot prose predated
    SR-CONDITION; now `gap: gap.signal.versions`), and a change naming the
    removed `cap.signal.attachments` (now `att.signal.send`).
    `pilot-telegram.md`: a prose selector default (now
    `default_state: absent`) and a gap naming the old `cap.*` ID.
  - Cross-file fixtures now name `c.fx.content.service_max`, a fact in the
    companion `contract/constraints-field.md`. `overrides-valid.yaml` and
    `sr-override--expired-override.yaml` carry the real target and schema
    fingerprints, so each negative fails for exactly one cause.
  - New headers: `# validate-scope: accepted` (missing roster interface, gap
    placeholder) and `# expect-ineligible: <fact> <reason>` (enforceable,
    secondary-only evidence). README updated.
- Performance: every `DarkmatterSchemas::validate` call re-resolves the
  schema's imports. With the typed corpus added, two tests hit nextest's 30 s
  termination in the full `--all-features` run. Two fixes: `Loader` now caches
  the resolved `EffectiveSchema` per contract kind (sound because binding
  pins each kind to one schema file), and the Phase 2 corpus helper caches by
  canonical `$schema` path. Both research suites dropped from about 18 s to
  3 s wall time.
- `_rules.md` updated: two new load-stage codes (`SR-SCHEMA-BINDING`,
  `SR-TOP-LEVEL`), the two scopes, and the rulings above (SR-UNIQUE key,
  SR-EVIDENCE, SR-ENFORCEABLE advisory, SR-COVERAGE counting, SR-GAP scope,
  SR-CHANGE gaps and removals, SR-ENVELOPE root pointer, and SR-MAPPING
  outcome versus finding).

### Requirement-to-test mapping (Phase 3)

| Behavior | Test(s) |
|---|---|
| Typed DTOs mirror schema v1 exactly (no lost, renamed, or extra field) | `research_validation::{contract,interaction,diagnostic}_documents_round_trip_through_the_schema` (load → serialize → Darkmatter schema → deserialize, twice for determinism) |
| Roster, overrides, mappings typed load | `typed::shipped_roster_is_a_complete_accepted_roster`, `typed::positive_roster_overrides_and_mappings_fixtures_are_clean` |
| Adapter IDs = `ProviderKind::as_str()` | `typed::adapter_ids_are_the_persisted_provider_kind_spellings` |
| SR-VERSION independent of schema coercion (`'1'`) | `a_quoted_schema_version_is_unsupported_even_though_the_schema_coerces_it` |
| SR-STRICT-SCALARS: quoted number, numeric operand, float version, with pointer | `strict_scalars_reject_authored_type_mismatches_at_their_pointer` + the two `sr-strict-scalars--*` fixtures |
| SR-TOP-LEVEL | `an_unknown_top_level_key_is_rejected_by_rule` |
| SR-SCHEMA-BINDING: missing or foreign `$schema`; shipped legacy docs | `the_schema_must_be_the_contract_for_the_file_kind`, `shipped_legacy_documents_are_reported_as_unbound` |
| Typed errors with repo-relative paths, no host paths | `unreadable_files_are_errors_with_repository_relative_paths`; every semantic finding's path is checked portable in `assert_semantic_rules` |
| All 30 SR rules reject their fixtures, with only that rule | `typed::{identity,constraint,coverage,binding,interaction,error}_rules_reject_their_fixtures`; `every_semantic_fixture_is_exercised_by_a_rule_family` |
| Positive corpus clean | `typed::{contract,interaction,diagnostic}_documents_pass_the_semantic_rules` |
| Eligibility keeps UTF-8 / scalar / UTF-16 / grapheme / parsed / serialized distinctions | `executable_constraints_keep_their_researched_unit_and_stage` |
| Ambiguous, unknown, conflicting, not-applicable, advisory, and secondary-only facts are never executable | `ambiguous_unknown_and_advisory_constraints_never_become_executable`; `expect-ineligible` fixtures |
| Unresolved condition makes a known bound ineligible (with and without the gap) | `an_unresolved_condition_makes_an_otherwise_known_bound_ineligible` |
| Applicability: co-holding vs exclusive vs identical conditions | `only_conditions_that_can_hold_together_make_two_values_ambiguous` |
| Missing interface, category, or platform never means unrestricted | `an_interface_without_a_matrix_reports_missing_not_unrestricted`, `accepted_scope_requires_the_whole_fleet_and_every_roster_interface` |
| Stale but valid research stays inspectable | `stale_or_invalid_research_remains_inspectable` |
| Deterministic, sorted findings | `findings_are_deterministic_and_sorted` |
| Overrides: expired (boundary day), orphaned, stale target, stale schema | `overrides_fail_precisely_when_expired_orphaned_or_stale` |
| Assessment reuse only on fingerprint equality; revision and unrelated files ignored; CRLF equal; code or fact change → `unassessed` + gap; removed fact; proposed never a claim | `assessments_are_reused_only_while_every_fingerprint_matches` |
| Error replay: match, near miss, unknown code, warnings, plain text, header locators | `diagnostics/*` fixtures via the positive corpus; `sr-fixtures--*`, `sr-match-overlap--*`; `replay::tests::*` |
| Canonical JSON and fingerprints | `canonical::tests::*` |
| Passivity of typed load and validation | `typed::typed_loading_and_validation_are_passive` (effect-engine and network counters, bytes unchanged) |
| Normal send build free of maintenance deps | `cargo tree -e normal` gate below |

There is no persisted read/write/read round trip in this phase: Phase 3
writes nothing. The DTO round trip through the schema stands in for it, and
Phase 4 owns persistence round trips.

### Gates (macOS)

- `just test` in `messenger/` (now `desktop,research`): 587 run, 587 passed,
  2 skipped (pre-existing skips).
- `just lint` in `messenger/`: clean.
- `cargo nextest run -p messenger --all-features`: 509 run, 509 passed,
  2 skipped (the first run had two corpus timeouts, fixed as described above).
- `cargo clippy -p messenger --all-features --all-targets -- -D warnings`: clean.
- `cargo nextest run -p messenger --features research` (default providers
  plus research, no desktop): 236 run, 236 passed, 2 skipped.
- `cargo tree -p messenger -e normal` with `--no-default-features`, default,
  and `--features desktop`: no `darkmatter`, `biscuit-file`, `biscuit-hash`,
  `serde_path_to_error`, YAML, or Claudine crate. `--features research` adds
  exactly those four, and no Claudine.
- Pre-existing, not from this phase: `cargo nextest run -p messenger
  --no-default-features` does not compile the lib's unit tests
  (`src/tests/validation.rs:447` uses `Target::discord_channel` without the
  `discord` feature). The same happens with or without `research`; the
  `--no-default-features` library itself compiles, including `research`.

### Cross-OS evidence

- **Linux:** Docker Desktop (`rust:1`, rustc 1.97.1, `linux/arm64`) on a copy
  of the worktree: `research_corpus` 28/28, `research_validation` 19/19,
  research lib unit tests 8/8. `just cross-check --os linux` is still queued
  behind the stale `build-linux` lock held since 2026-09-14 by
  `reward-20260914-c3e60d0` (`nightly-reward-spike`); the foreign lock was
  not removed.
- **Windows:** compile evidence only. `cargo check -p messenger --features
  research --target x86_64-pc-windows-gnu --tests` passed.
  `just cross-check --os windows` failed again because `W:` on
  `build-win-native` is full ("No space left on device"); it was not cleaned,
  because that is a human decision on a shared host.
- **WSL:** `just cross-check --os wsl` failed with an SSH connection reset
  to `build-win`, as in Phases 1–2.
- A first attempt passed `-E 'binary(…) | binary(…)'` to `just cross-check`,
  which breaks its argument line (already documented in the `os` skill's
  `build-hosts.md`). New fact added to `os/macos.md`: Docker bind mounts under
  `/tmp` fail on this host, so Linux containers need source and target dirs
  under `$HOME`.
- OS risk review: repository paths are built from `Path::components` and
  joined with `/`; schema binding compares canonicalized paths (verbatim
  `\\?\` on both sides); fingerprints normalize CRLF; tests read
  `CARGO_MANIFEST_DIR` at run time for the WSL archive leg. The macOS `/var`
  symlinked tempdir already exercises the non-canonical workspace-root path.
