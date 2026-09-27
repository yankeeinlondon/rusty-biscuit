---
spec: "/Volumes/coding/wt/rusty-biscuit/fix-sniff/sniff/features/2026-09-26-lockfile-corroboration/spec.md"
plan: "sniff/features/2026-09-26-lockfile-corroboration/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
source_files_during_phase_1: []
fixtures_created_during_phase_1:
    - sniff/lib/tests/fixtures/lockfiles/
    - sniff/lib/tests/fixtures/lockfiles/.ignore
docs_updated_during_phase_1:
    - sniff/features/2026-09-26-lockfile-corroboration/spec.md
    - sniff/features/2026-09-26-lockfile-corroboration/plan.md
docs_created_during_phase_1:
    - sniff/features/2026-09-26-lockfile-corroboration/accepted-versions.md
    - sniff/features/2026-09-26-lockfile-corroboration/spike-s2-jsonc.md
    - sniff/features/2026-09-26-lockfile-corroboration/spike-s3-parsers.md
    - sniff/features/2026-09-26-lockfile-corroboration/spike-s4-evidence-audit.md
    - sniff/features/2026-09-26-lockfile-corroboration/measurements.md
    - sniff/features/2026-09-26-lockfile-corroboration/implementation-log.md
    - sniff/fixes/_unscheduled/package-manager-uv-label/spec.md
skills_files_updated_during_phase_1:
    - .claude/skills/sniff/SKILL.md
source_files_during_phase_2:
    - sniff/lib/src/filesystem/repo/lockfile/mod.rs
    - sniff/lib/src/filesystem/repo/lockfile/sources.rs
    - sniff/lib/src/filesystem/repo/lockfile/membership.rs
    - sniff/lib/src/filesystem/repo/lockfile/pnpm.rs
    - sniff/lib/src/filesystem/repo/lockfile/uv.rs
    - sniff/lib/src/filesystem/repo/lockfile/cargo.rs
    - sniff/lib/src/filesystem/repo/lockfile/npm.rs
    - sniff/lib/src/filesystem/repo/lockfile/yarn.rs
    - sniff/lib/src/filesystem/repo/lockfile/bun.rs
    - sniff/lib/src/filesystem/repo/lockfile/rush.rs
    - sniff/lib/src/filesystem/repo/lockfile/fallback.rs
    - sniff/lib/src/filesystem/repo/lockfile/tests.rs
    - sniff/lib/src/filesystem/repo/detection.rs
    - sniff/lib/src/filesystem/repo/manifest_index.rs
    - sniff/lib/src/filesystem/repo/mod.rs
    - sniff/lib/src/filesystem/repo/npm.rs
    - sniff/lib/src/filesystem/repo/standard.rs
    - sniff/lib/src/filesystem/repo/topology.rs
    - sniff/lib/src/filesystem/repo/types.rs
    - sniff/lib/src/filesystem/repo/aggregate_view.rs
    - sniff/lib/src/performance/counters.rs
    - sniff/lib/src/request.rs
    - sniff/lib/tests/fixtures.rs
    - sniff/lib/tests/l1/main.rs
    - sniff/lib/tests/l1/integration.rs
    - sniff/lib/tests/l1/lockfile_provenance.rs
    - sniff/lib/tests/l1/lockfile_fixtures.rs
    - sniff/cli/src/output/repo_json.rs
    - sniff/cli/tests/l1/cli.rs
    - sniff/cli/tests/l1/snapshots/l1__snapshots__repo_aggregate_json.snap
    - claudine/lib/src/events/environment.rs
docs_updated_during_phase_2:
    - sniff/lib/README.md
    - sniff/cli/README.md
    - sniff/features/2026-09-26-lockfile-corroboration/accepted-versions.md
    - sniff/features/2026-09-26-lockfile-corroboration/plan.md
    - sniff/features/2026-09-26-lockfile-corroboration/implementation-log.md
    - sniff/features/2026-09-26-lockfile-corroboration/spec.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
    - .claude/skills/sniff/SKILL.md
    - .claude/skills/sniff/performance.md
source_files_during_phase_3:
    - Cargo.lock
    - sniff/lib/Cargo.toml
    - sniff/lib/src/filesystem/repo/jsonc.rs
    - sniff/lib/src/filesystem/repo/mod.rs
    - sniff/lib/src/filesystem/repo/detection.rs
    - sniff/lib/src/filesystem/repo/nested.rs
    - sniff/lib/src/filesystem/repo/npm.rs
    - sniff/lib/src/filesystem/repo/types.rs
    - sniff/lib/src/filesystem/repo/lockfile/mod.rs
    - sniff/lib/src/filesystem/repo/lockfile/npm.rs
    - sniff/lib/src/filesystem/repo/lockfile/yarn.rs
    - sniff/lib/src/filesystem/repo/lockfile/bun.rs
    - sniff/lib/src/filesystem/repo/lockfile/rush.rs
    - sniff/lib/src/filesystem/repo/lockfile/fallback.rs
    - sniff/lib/src/filesystem/repo/lockfile/standalone.rs
    - sniff/lib/src/filesystem/repo/lockfile/tests.rs
    - sniff/lib/tests/l1/lockfile_fixtures.rs
    - sniff/lib/tests/l1/lockfile_provenance.rs
    - sniff/cli/src/output/filesystem/mod.rs
    - sniff/cli/src/output/repo_json.rs
    - sniff/cli/tests/l1/cli.rs
    - claudine/cli/src/commands/wrap/env/tests.rs
docs_updated_during_phase_3:
    - sniff/lib/README.md
    - sniff/docs/dependencies.md
    - docs/dependencies.md
    - sniff/features/2026-09-26-lockfile-corroboration/accepted-versions.md
    - sniff/features/2026-09-26-lockfile-corroboration/plan.md
    - sniff/features/2026-09-26-lockfile-corroboration/implementation-log.md
    - sniff/features/2026-09-26-lockfile-corroboration/spec.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
    - .claude/skills/sniff/SKILL.md
    - .claude/skills/sniff/performance.md
packages:
    - sniff
    - sniff-cli
    - claudine
    - claudine-cli
---

# Implementation Log for 2026-09-26-lockfile-corroboration (5 phases)

## Phase 1

### Rulings recorded (task "Record rulings")

- Added `## Owner rulings` (R1–R11) to `spec.md`, marked "adopted default —
  override before Phase 2". Spec `status` stays `draft-spec`.
- Contract tables updated: status table gained `members_present` and
  `members_missing` (Cargo only); `extra`/`missing` field notes updated; a new
  `### Repository-level standalone lockfiles` table defines
  `RepoInfo.standalone_lockfiles`; the Cargo and Rush coverage rows now point at
  R1 and R3. Each open question now carries a pointer to its ruling.

### Unscheduled fix (R4)

- Created `sniff/fixes/_unscheduled/package-manager-uv-label/spec.md`. It also
  records the analogous Bun-labeled-as-npm gap found while reading
  `detect_package_managers` (`detection.rs:1431`).

### Fixture placement checks

- `sniff/lib/tests/fixtures/` is a data directory the layout gate
  (`tools/test-toolkit/src/test_layout.rs`, `DATA_DIRECTORIES`) skips, so Cargo
  fixture `.rs` files there do not violate the consolidated-binary layout.
- `git check-ignore` confirmed that every lockfile name and `.yarnrc.yml`,
  `.venv`, `vendor`, and `common/` paths under the fixture root are trackable;
  only `node_modules/` and `target/` are ignored, and fixtures must not contain
  them anyway.
- Nested manifests under `tests/fixtures` have precedent
  (`scripts/ci/fixtures/*/Cargo.toml`, `darkmatter/lib/tests/fixtures/validate`).

### Spike S1: real-tool fixtures

- 67 fixture directories (582 files, about 367 KiB) under
  `sniff/lib/tests/fixtures/lockfiles/<tool>-<version>/<case>/`, each with a
  `PROVENANCE.md` (tool version, commands, host, date, expected membership).
  Generated in `/tmp` scratch directories, never inside the repo; no
  `node_modules`, `.git`, `target`, `.venv`, `.gradle`, `build`, or `common/temp`.
- Tools: pnpm 8.15.9 / 9.15.9 / 10.32.1; npm 6.14.18 / 11.6.4 (v1, v2, v3,
  shrinkwrap); Yarn 1.22.22 / 3.8.7 / 4.0.2 / 4.18.1; Bun 1.2.0 / 1.3.3 (text,
  binary, both); uv 0.9.5 (workspace, virtual root, root-only workspace,
  single project); cargo 1.52.0 / 1.77.2 / 1.98.1 (v2, v3, v4; docker for the
  older two); Rush 5.179.0 on pnpm 9.15.9; Go 1.27.1; Gradle 5.6.4 / 8.14.5
  (root and subproject-only layouts); Bazel 8.4.2; Poetry 2.5.1; PDM 2.29.2;
  Composer 2.10.3.
- Hand-edited `-edited-<variant>` copies (stale-extra, missing,
  malformed-trailing, unknown-version, duplicate-key, missing-required-field)
  for pnpm 10, npm 11, Yarn 4.18, Bun 1.3.3, uv, and Cargo v4, plus a Bun
  comments-and-trailing-commas variant.
- Beyond the plan's list, I added pnpm 8 (`'6.0'`), Yarn 3.8.7 and 4.0.2
  (metadata versions 6 and 8), Bun 1.2.0 (no `configVersion`), a uv root-only
  workspace, and root-level Gradle locks. Without them, common real versions
  would be `unsupported_version`, and the Gradle root candidates would have
  no fixture.
- Deviation: one subagent made a throwaway local commit inside its `/tmp`
  Rush scratch repo (Rush requires git). It never touched this repository.

### Spike S2: JSONC crate

- `jsonc-parser` 0.33.2 (`serde` feature) with explicit strict `ParseOptions`.
  It streams (0.03 MiB peak on 15.7 MiB) and rejects trailing garbage. It is a
  new direct `sniff` dependency, so Phase 5 updates the dependency docs.
  `json-five`, already in the graph through `biscuit-file`, was rejected
  because it builds a full AST (149 MiB peak). See `spike-s2-jsonc.md` and
  `measurements.md`.

### Spike S3: parser feasibility

- Written up in `spike-s3-parsers.md`, with timings in `measurements.md`. Key
  findings, now folded into the Phase 2/3 tasks in `plan.md` as "Phase 1
  finding" bullets:
  - `serde_yaml_ng` coerces non-string YAML keys to strings, so pnpm
    importer keys must be read as `Value` to reject them;
  - npm records a `file:` dependency exactly like a member, so filter by the
    locked `packages[""].workspaces` globs;
  - Yarn Classic must be detected by signature before the YAML parse;
  - uv omits `[manifest]` for a root-only workspace, so absence means "root
    only", not `parse_failed`;
  - a Cargo lock without `[[package]]` is `parse_failed` for corroboration
    only.
- **Pre-existing bug:** real `rush.json` files contain comments, and
  `parse_rush_project_folders` uses strict `serde_json`, so Sniff detects no
  Rush layer on the real Rush 5.179.0 fixture. Stripping comments makes the
  layer appear. This is added as a prerequisite to the Phase 3 Rush task.
- **Parser cost:** `serde_yaml_ng` buffers every event and `toml` builds its
  own document tree, so typed YAML/TOML parses still peak at about 17–22x the
  input. For TOML the peak equals the generic parse's. Only the JSON parsers
  stream.

### Spike S4: evidence audit

- `spike-s4-evidence-audit.md`: the call-site table, `ManifestStore` today, and
  the R10 signals. None of the three signals exists; it recommends a
  `pub(crate) membership_incomplete` on `DetectorOutcome` and a `Result`-based
  Cargo manifest cache. It notes that
  `absent_cargo_lock_counts_one_read_attempt_and_no_parse` (`detection.rs`)
  changes meaning once presence gates reads.

### Accepted-version matrix

- `accepted-versions.md`: compared-format rows (accepted values, producer,
  fixture, field path, unsupported versions, signatures), an expected-result
  row for every fixture and variant, fallback and standalone rows, and the
  JSONC choice.

### Fixture discovery problem and `.ignore`

- The first `just test` failed:
  `integration::test_rusty_biscuit_repo_topology_parity` found
  `sniff/lib/tests/fixtures/lockfiles/bun-1.2.0/workspace/.tools/hidden` as a
  `BunWorkspaces` package. The in-tree `sniff repo --json` reported 63 layers
  (61 of them fixtures). Cause: the nested-marker walk (`nested.rs`) has no
  fixture-directory exclusion, unlike the manifest index
  (`is_fixture_manifest`).
- Phase 1 must not change production code, so I added
  `sniff/lib/tests/fixtures/lockfiles/.ignore` (`/*/`). Sniff's `ignore`-crate
  walkers honor it and git does not, so all fixtures stay tracked. The
  in-tree `sniff repo --json` is back to the 2 real layers. Side effect: `rg`
  and Grep skip the fixtures unless `--no-ignore` is given. The permanent fix
  (a fixture exclusion in the nested walk) is an owner decision, noted in the
  spec's `message_to_agent`.

### Index anomaly

- Part of this phase's output appeared staged (`A`) in the git index mid-phase:
  the plan, log, and spike docs, plus fixtures created before the uv/Cargo
  set. Neither I nor any subagent brief ran `git add`; something outside this
  session staged them. I left the index untouched.

### Gates

- `just test` (sniff): 2909 passed, 32 skipped (after `.ignore`).
- `just lint` (sniff): clean.
- `cargo clippy -p sniff -p sniff-cli --all-targets -- -D warnings`: clean.
- Longest fixture path is 129 characters, shorter than the repo's existing
  177-character maximum, so there is no new Windows MAX_PATH risk. No code
  changed, so no cross-OS run was needed.

## Phase 2

### Design decisions (made before code)

- **Module:** `sniff/lib/src/filesystem/repo/lockfile/` is a crate-private
  module. `filesystem::repo` re-exports `LockfileObservation`,
  `LockfileStatus`, and `LockfileReason`.
- **Layer to seed ownership:** the engine finds the `DetectorOutcome` that
  built each layer by `(root, authority)` and uses that outcome's seeds as the
  manifest-side member set. `layer.packages` is copied from those same seeds,
  so R10(a), "a declared member with no resolved seed", can occur only when no
  outcome matches the layer. That case reports `incomplete_manifest_discovery`.
  Provenance on `match` upgrades only flat seeds with the same `key`,
  `standard`, and `owner_root` as one of the outcome's seeds. That is ownership,
  not a bare `relative` match.
- **Presence probe:** one `std::fs::metadata` call per logical probe. It
  follows links, `NotFound`/`NotADirectory` are `Absent`, and any other error
  is `Failed(kind)`. That is one syscall and one `FS_METADATA_PROBES`, the
  same as `probe_exists`. The plan's `symlink_metadata`-then-follow has the
  same outcome with two syscalls. The presence cache is keyed on the lexical
  `normalize_path`, so a probe pays no hidden `canonicalize`, as S4 §1.5
  recommends. The content cache keeps `normalized_key`.

### Wave 1: engine (single agent)

- **Types** (`lockfile/mod.rs`): public `LockfileObservation`,
  `LockfileStatus`, and `LockfileReason` with the R5 snake-case vocabulary.
  `MonorepoLayer.lockfile` is required, with no `#[serde(default)]`, so old
  result JSON fails to deserialize. The public constructor is
  `LockfileObservation::not_applicable(reason)`, used by `topology.rs` as a
  placeholder that detection always overwrites, and by the Claudine and CLI
  test literals.
- **Table** (`lockfile/sources.rs`): an exhaustive `match` over all 18
  variants. Nx, Turborepo, and Lerna map to `NotApplicable(NoLockfileSource)`
  only so the match stays exhaustive (R7: they never own a layer).
- **Store** (`detection.rs`): `pnpm_locks` and `uv_locks` are gone.
  - `lockfile_presence(path)` caches the tri-state probe, keyed lexically.
  - `lockfile(path, format)` caches
    `Result<Rc<ParsedLockfile>, LockfileFailure>`, and every read is gated on
    presence.
  - `cargo_lock(path)` / `cargo_lock_outcome` cache one `CargoLock`, which
    carries both the lenient `CargoLockVersions` and the strict
    corroboration view from a single TOML parse, so enrichment and
    corroboration share one read and one parse.
  - `read_counted_lockfile` is now the only lockfile read site;
    `CargoLockVersions::parse(path)` was deleted.
- **Shared probe sites** (S4): `has_bun_lockfile` and `detect_yarn_workspace`
  now probe through `lockfile_presence`. Consequence:
  `filesystem.repo.lockfile_probes` also counts those detector marker probes,
  including the nested dispatch at every `package.json` directory. Those
  probes predate this feature (they were already `FS_METADATA_PROBES`), and
  the only new metadata work is one probe per layer candidate. The
  `has_workspace_marker` pre-gate, `detect_package_managers`, and
  `resolve_js_package_manager` still use `probe_exists`, per S4.
- **R9 seam**: `lockfile::test_seam::fail_metadata(path, kind)` is a
  `#[cfg(test)]` per-thread seam checked by `probe_presence`, so it works
  through full detection in unit tests.
- **R10**: signal (a) is structural: the engine uses the layer's own outcome
  seeds, and a missing outcome is `incomplete_manifest_discovery`. Signal (b)
  covers Cargo only, because only Cargo parses member manifests for
  corroboration: an unreadable or unparseable member `Cargo.toml`, or an
  unresolvable name or version, is `incomplete_manifest_discovery`. Signal (c)
  does not apply: S4 found the glob expander has no bound to report. Its
  silently dropped walk errors are an owner decision (S4 §3c) and are not
  flagged.
- **Rush** (`Source::Configured`, `lockfile/rush.rs`): a Phase 2 placeholder.
  It probes Rush's three manager lockfiles under `common/config/rush/` and
  reports `absent`, `not_requested`, or `unverifiable` +
  `unsupported_layout`, reading no configuration. Phase 3 replaces it.
- **Fallback** (`lockfile/fallback.rs`): Go `go.work.sum`, Gradle root
  `gradle.lockfile`, and Bazel `MODULE.bazel.lock`, metadata only. The Gradle
  legacy lock directory and Bazel's `MODULE.bazel` gate are Phase 3.
- **Stubs**: `npm.rs`, `yarn.rs`, and `bun.rs` return `UnsupportedVersion`
  after one counted read, so those layers are `unverifiable` +
  `unsupported_version` until Phase 3. `bun.lockb` is already
  `unverifiable` + `no_membership_data` with no read, from the table's
  `Format::BunBinary`.
- Deleted `upgrade_provenance_with_lockfile`, `pnpm_lockfile_matches`,
  `uv_lockfile_matches`, `cargo_lockfile_matches`,
  `normalize_layer_package_relative`, and `layer_relative_path`.

### Wave 2: parsers (done sequentially by the same agent)

- **pnpm** (`lockfile/pnpm.rs`): from the patch, with three fixes.
  - `lockfileVersion` is captured as an untagged string or number. The quoted
    value must be exactly `6.0` or `9.0`; a bare number compares by value.
  - Importer keys are read as `serde_yaml_ng::Value`, and a non-string or
    duplicate key is `parse_failed`.
  - The importers visitor records invalidity instead of erroring, so an
    unsupported version is classified before any shape error.
  - Duplicate keys in skipped sections are tolerated, and a test documents
    that divergence from the `#[cfg(test)]` generic `Value` reference. Parity
    with the reference is checked on every pnpm fixture, the Rush pnpm lock,
    and the repository's own `pnpm-lock.yaml`.
- **uv** (`lockfile/uv.rs`):
  - Accepts `version = 1`, `revision = 3`.
  - Maps each `[manifest].members` name to exactly one
    `editable`/`virtual` source path; zero or several paths is
    `ambiguous_membership`.
  - An absent `[manifest]` is the root alone.
  - When the typed parse fails, the version fields are re-read from a
    two-field header struct, so an unsupported version with an unfamiliar
    layout is `unsupported_version`, not `parse_failed`. This second parse
    runs only on the failure path.
  - `#[serde(flatten)]` was rejected: it buffers the whole document into
    serde's generic `Content` tree.
- **Cargo** (`lockfile/cargo.rs` + `manifest_index.rs`):
  - `CargoLockDocument` gained a lenient `version` (`LockVersion`),
    `metadata` presence, per-entry `source` presence (`Option<IgnoredAny>`),
    and `LockedPackages { is_array, skipped }`.
  - `CargoLockVersions::from_document` keeps the old lenient results. The
    parity test compares it with `parse_reference` on every Cargo fixture and
    the repository's own `Cargo.lock`.
  - R1 compares each non-root member's `(name, resolved version)` against
    the source-less entries, with `version.workspace = true` resolved and an
    omitted version treated as Cargo's `0.0.0`.

### Findings and expectation changes (call out in the commit message, R11 `feat(sniff)!:`)

- **Breaking:** `MonorepoLayer.lockfile_match` is removed and replaced by the
  required `lockfile` object. The CLI JSON gained the object automatically,
  because layers serialize through serde. The aggregate snapshot
  (`l1__snapshots__repo_aggregate_json.snap`) gained a `lockfile` block
  (`absent`) and nothing else.
- **Cargo:** the old "extra entry still matches" expectation is now
  `members_present` + `subset_only`, and the provenance stays `globbed`. That
  covers `lockfile_provenance.rs`, the `detection.rs` unit tests, and the
  CLI's `repo structure` / bare `repo` tests, whose names changed from
  `..._reports_lockfile_provenance_...` to
  `..._reports_cargo_subset_evidence_...`.
- **uv:** the synthetic `[workspace].members` fixtures (`fixtures.rs`,
  `lockfile_provenance.rs`, `detection.rs`) were rewritten in uv 0.9's real
  `[manifest]` + `source` layout, and comparison excludes the root.
- **Counters:** an absent lockfile is now 1 probe and 0 reads, down from 1
  read. So `absent_cargo_lock_counts_one_read_attempt_and_no_parse` became
  `absent_cargo_lock_is_one_probe_and_no_read`, and two enrichment tests
  dropped one `FS_FILE_OPENS`.
- **Pre-existing test typo fixed:**
  `create_pnpm_workspace_with_stale_lockfile` had a stray literal `\n`
  importer key (`"  \\n"`). The test only asserted `Some(false)`, so the
  bogus key was invisible. It would now appear in `extra`, so it was removed.
- **Unreachable matrix row:** a uv workspace with `members = []` produces no
  layer (`uv.rs` detector returns `None`), so
  `uv-0.9.5/root-only-workspace` → `match` holds only at the parser and
  engine level. `accepted-versions.md` is annotated, and an L1 test pins that
  no layer exists.

### Requirement-to-test mapping

| Behavior | Tests |
|---|---|
| Wire vocabulary and always-serialized object, repeated JSON round trip | `lockfile::tests::the_wire_names_are_the_frozen_snake_case_vocabulary`, `the_observation_serializes_every_field_and_round_trips`; L1 `lockfile_fixtures::the_serialized_layer_carries_the_complete_lockfile_object` |
| Exhaustive candidate table (17 + `Unknown`) | `lockfile::sources::tests::every_standard_has_the_specified_source` |
| Precedence: not_applicable → metadata failure → absent → not_requested → fallback → parse | `lockfile::tests::*` (not-applicable without probing, absent for every sourced authority, not_requested without reads, fallback metadata-only, bun.lockb never read, R6 first-present selection, metadata failure not skipped, metadata failure beats a declined request) |
| R9 metadata and read failures | `detection::observation_index::a_metadata_failure_is_unreadable_and_reads_nothing` (seam), `a_directory_in_place_of_the_lockfile_is_a_read_failure` (passes on macOS, Linux, Windows, WSL) |
| Request-local caching of presence and failures | `the_store_caches_lockfile_presence_and_failures` |
| Path normalization (`.`, trailing `/`, `..` against a base, leading dots, case, `\`, absolute/drive/NUL/non-UTF-8/other-drive) | `lockfile::membership::tests::*` (the Windows drive test ran on `build-win-native`) |
| Set comparison and sorted extra/missing, no intersection | `membership::tests::compare_*`; L1 fixture stale-extra and missing rows |
| R10 incomplete discovery | `lockfile::tests::a_layer_without_its_detector_outcome_is_incomplete`, `detection::…::an_unparseable_cargo_member_is_incomplete_manifest_discovery` |
| Ownership-scoped provenance (never across authorities) | `opted_in_structure_request_corroborates_every_lockfile_once` (Cargo seeds stay `globbed` beside matched pnpm and uv); L1 `assert_rows` checks every `lockfile` package belongs to the matched authority |
| Disabled structure request: 0 reads, 0 parses, N probes, `not_requested` | `declining_structure_request_probes_but_reads_and_parses_no_lockfile`; L1 `lockfile_provenance::declining_requests_*`, `lockfile_fixtures` declined half |
| Full dependency request with corroboration off still reads Cargo.lock once, no upgrade | `declined_corroboration_still_resolves_dependency_versions_from_cargo_lock`, `cargo_lock_is_shared_by_corroboration_and_dependency_enrichment` |
| pnpm parser (versions, non-string/duplicate keys, missing sections, unsupported never mismatch, ignored-section duplicate tolerance, parity with the generic reference) | `lockfile::pnpm::tests::*`, including the passive corpus over every pnpm fixture, the Rush lock, and the repository's `pnpm-lock.yaml` |
| uv parser (version/revision, manifest mapping, absent manifest, ambiguity, non-member local deps, unsupported layout) | `lockfile::uv::tests::*`, including the passive corpus over every uv fixture |
| Cargo R1 (source-less name+version, registry same-name, local nonmember, v1/v2/v3/v4, missing `[[package]]`) and `resolve` parity | `lockfile::cargo::tests::*`, including the corpus and `version_index_is_unchanged_by_the_shared_parse` (fixtures plus the repository's `Cargo.lock`); the existing `manifest_index` parity tests |
| End to end through the public API on real-tool fixtures | L1 `lockfile_fixtures::{pnpm,uv,cargo}_tool_fixtures_*` (every Phase 2 fixture and variant, corroborating and declined) |
| Complete serialized `RepoInfo` matrix | L1 `lockfile_provenance::*` |
| CLI JSON carries the object (Phase 4 extends) | `sniff-cli::l1 cli::repo_structure_json_reports_cargo_subset_evidence_for_matching_lockfile`, `repo_aggregate_json_reports_cargo_subset_evidence_for_matching_lockfile`, `snapshots::repo_aggregate_json_snapshot` |

All new tests are L1: no tier marker appears in any path segment. Unit tests
live in the lib target, and `lockfile_fixtures` is declared in
`tests/l1/main.rs`; `test_layout` passes. Fixture reads use `include_str!`
(unit) or `manifest_dir!().join("tests/fixtures/lockfiles")` (L1), so CI
test-input narrowing sees them.

### Gates

- `just test` (sniff): 2970 passed, 32 skipped. The first run failed only on
  `snapshots::repo_aggregate_json_snapshot` (new `lockfile` block); after
  accepting it, everything passes.
- `just lint` (sniff): clean.
- `cargo clippy -p sniff -p sniff-cli --all-targets -- -D warnings`: clean,
  and also clean with `--all-features`.
- `cargo check --all-targets` on all 21 reverse dependencies of `sniff`,
  including claudine, darkmatter, worktree, and messenger: clean. I did not
  run a workspace-wide check, per the repo rule. Claudine's
  `from_sniff_result_populates_monorepo_topology`: passes.
- `./scripts/cross-check.sh sniff --os all`: linux pass (2084), windows pass
  (2073, including the Windows-only
  `manifest_member_rejects_a_member_on_another_drive`), wsl pass (archive
  mode). No receipt was published because the tree is uncommitted.
- Skipped: nothing. No pre-existing failures.

## Phase 3

All six Wave 1 tasks were done sequentially by one agent. Each format stayed
in its own `lockfile/*.rs` file. The engine (`lockfile/mod.rs`) changed in
two small ways only: its parsed-outcome `match` became a shared `classify`
helper, which Rush reuses with the `common/temp` base, and `rush::observe` now
receives the layer and its owned seeds.

### New dependency

- `jsonc-parser` 0.33.2 (`serde` feature), per spike S2. The new
  `filesystem::repo::jsonc` module is its only caller. It passes a literal
  strict `ParseOptions` (comments and trailing commas only). Dependency docs
  were updated now rather than in Phase 5, because the repo's drift rule ties
  them to the change that adds the crate: `sniff/docs/dependencies.md` and
  root `docs/dependencies.md` (a catalog entry plus a recent-notes line).

### Parsers

- **npm** (`lockfile/npm.rs`): typed `serde_json` with a `packages` map
  visitor.
  - The visitor keeps only the root record's `workspaces`, every record path
    without a `node_modules` component, and the `resolved` target of each
    `link: true` record.
  - The locked set is the paths and link targets that the locked declarations
    match, through a pure string `globset` matcher. `*` stops at `/`, `**`
    spans it, and a leading `!` excludes, as in the manifest-side expander.
    `./` prefixes and trailing `/` are normalized away.
  - A duplicate `packages` key is `parse_failed`, and so is a missing
    `packages` object or root record at v2/v3.
  - v1 and any other version is `unsupported_version`. A non-integer version
    counts as a recognizable unsupported version.
  - A root record without `workspaces` is `ambiguous_membership` when any
    local package path exists, and an empty set otherwise. An invalid
    declaration glob is also `ambiguous_membership`.
  - When the typed body parse fails, the version is re-read from a header
    struct, as uv does. So an unsupported version with an unfamiliar body is
    `unsupported_version`, not `parse_failed`. Yarn and Bun follow the same
    pattern.
- **Yarn** (`lockfile/yarn.rs`):
  - The Classic signature is checked in the leading comment block, before
    parsing.
  - Berry `__metadata.version` 6, 8, and 10 are accepted.
  - Each entry's `resolution` is split at the last `@workspace:`. A name
    containing `:` (a protocol such as `patch:`) is rejected, so `link:`,
    `portal:`, and patch descriptors never count.
  - Two entries resolving to one path collapse. Two names for one path, a
    duplicate entry key, a missing or mistyped `resolution`, and no `.` root
    workspace are `parse_failed`.
- **Bun** (`lockfile/bun.rs`): `jsonc` into a typed struct with a
  `workspaces` key visitor. `lockfileVersion` 1 is accepted. A missing or
  non-object `workspaces`, or a duplicate key, is `parse_failed`. `bun.lockb`
  was already metadata-only from Phase 2.

### Rush (ruling R3)

- **Pre-existing bug fixed:** `parse_rush_project_folders` (strict JSON) is
  replaced by a typed `RushJson` model (`npm.rs`) parsed through `jsonc`.
  Fields of an unexpected type are kept as `Lenient::Other`, so an odd
  setting never hides the projects. The model is cached in `ManifestStore`
  (`rush_json`), so detection and corroboration share one read and one
  manifest parse. `detect_rush_workspace` now takes the store. That touched
  its two call sites (`detection.rs` and `nested.rs`). An unreadable
  `rush.json` still fails detection, as before; an unparseable one still
  means "no Rush layer". Regression tests: `npm::tests::the_real_rush_json_is_read_through_its_comments`
  and L1 `rush_tool_fixture_reports_the_accepted_version_matrix` (before the
  fix, the real fixture produced no layer).
- **`lockfile/rush.rs`:**
  - Exactly one of `pnpmVersion`/`npmVersion`/`yarnVersion` picks the manager
    and its `common/config/rush/*` lockfile. None or several is
    `unverifiable` + `unsupported_layout` with `paths: []`.
  - Precedence:
    1. A metadata failure is `unreadable`.
    2. npm and Yarn managers are `not_requested` when their lockfile is
       present and the request declines, and `unverifiable` +
       `unsupported_layout` otherwise.
    3. For pnpm with a declining request, `not_requested` or `absent`.
    4. For pnpm with a corroborating request, the layout is classified first:
       variants in `rush.json`, a `common/config/rush/variants` directory,
       `subspacesEnabled: true`, or `useWorkspaces` not `true` makes it
       `unverifiable` + `unsupported_layout`, with the lockfile listed when
       present. Only then is a missing lockfile `absent`.
  - `pnpm-config.json` wins. Without it, `rush.json`'s legacy
    `pnpmOptions.useWorkspaces` decides, and Rush's default is `false`.
  - Configuration that cannot be read or parsed is `unsupported_layout`, not
    `unreadable`, because the lockfile is not what failed. That decision is
    recorded in `accepted-versions.md`.
  - The supported layout parses through `Format::Pnpm` (the store cache) and
    drops the synthetic `.` importer. It then compares with the base
    `["common", "temp"]`, so `../../packages/alpha` becomes `packages/alpha`.
  - Config reads go through the new `detection::read_counted_config`, which
    counts `FS_FILE_OPENS`, `FS_BYTES_READ`, and `REPO_MANIFEST_PARSES` (R8).
    They happen only on a corroborating request. `rush.json` costs nothing
    extra, because the detector already cached it.

### Fallback sources

- **Gradle:** reports the root `gradle.lockfile` plus the direct `*.lockfile`
  children of the root `gradle/dependency-locks/`. That is one `read_dir`
  (`FS_READ_DIRS`), with no recursion. Directories named `*.lockfile` are
  skipped; a symlink counts when it resolves to a file. Both groups are listed
  when both exist.
  - A file where the directory should be holds no lockfiles. This is decided
    by a metadata check on the error path only, because `read_dir`'s error
    kind for a file differs by OS.
  - A metadata failure on the directory is `unreadable` + `metadata_failed`.
- **Bazel:** without `MODULE.bazel`, Bzlmod does not apply, so there is no
  lockfile source: `not_applicable` + `no_lockfile_source`, with no lockfile
  probe. The Phase 2 "absent for every sourced authority" test dropped Bazel
  for this reason. It is covered by the new
  `a_bazel_root_without_module_bazel_has_no_lockfile_source`.
- The Maven, .NET, Pants, Buck2, and `Unknown` `not_applicable` table tests
  already existed from Phase 2
  (`sources::tests::every_standard_has_the_specified_source` and
  `tests::standards_without_a_lockfile_source_are_not_applicable_without_probing`).

### Standalone observations (ruling R2)

- New public `StandaloneLockfileObservation { root, tool, #[serde(flatten)]
  observation }` and `StandaloneLockfileTool` (`composer`, `pdm`, `poetry`),
  re-exported from `filesystem::repo`.
- `RepoInfo.standalone_lockfiles` is always serialized. It has
  `#[serde(default)]`, so older `RepoInfo` JSON without the field still
  deserializes. Only the replaced `lockfile_match` was meant to break.
- It is filled on both result paths: the workspace path (root plus every
  final package root) and the root-package path (root only).
- Cost: three cached `lockfile_presence` probes per unique root, on every
  request, and never a read. That is `3 × (1 + packages)` extra
  `lockfile_probes`.
- **Expectation changes:** two Phase 2 unit tests pin exact probe counts.
  `absent_cargo_lock_is_one_probe_and_no_read` went from 4 to 13 (three
  roots). `declining_structure_request_probes_but_reads_and_parses_no_lockfile`
  went from 12 to 33 (seven roots). Their comments now itemize the standalone
  probes.
- The L1 complete-result matrix (`lockfile_provenance.rs`) gained
  `"standalone_lockfiles": []`.
- **CLI (Phase 4 handoff):** `sniff repo structure --json` serializes
  `RepoInfo` directly, so it now emits `"standalone_lockfiles": []`.
  `cli::repo_structure_json_reports_cargo_subset_evidence_for_matching_lockfile`'s
  expected JSON was updated to pin today's output. R2 says the CLI omits an
  empty list; that projection belongs to the Phase 4 "JSON projection" task,
  which must also flip this expectation. No CLI source changed in this phase
  except test-only `RepoInfo` literals.
- **Constructor fallout:** 16 test-only `RepoInfo` literals gained
  `standalone_lockfiles: Vec::new()`: `types.rs` (2),
  `cli/src/output/filesystem/mod.rs` (2), `cli/src/output/repo_json.rs`
  (10), and
  `claudine/cli/src/commands/wrap/env/tests.rs` (1). Claudine lib and
  Darkmatter already used `..Default::default()`.
- **Composer gap (pre-existing, not changed):** Sniff recognizes no PHP-only
  package, so a Composer-only project has no `RepoInfo` and no observation.
  The L1 test adds a `package.json` to the Composer fixture.

### Requirement-to-test mapping

| Behavior | Tests |
|---|---|
| Strict JSONC: comments and trailing commas only; rejects quotes, unquoted keys, missing commas, hex, unary plus, trailing content, an unterminated comment, empty input | `repo::jsonc::tests::*` |
| npm membership from the locked declarations, `file:` dependency excluded, link records, negation, `./` and trailing `/`, object-form declarations, `node_modules` exclusion | `lockfile::npm::tests::*` (corpus over every npm fixture, parity with a generic `serde_json::Value` reference) |
| npm v1, unknown, and odd versions never yield membership; malformed, duplicate, missing, and mistyped documents are `parse_failed`; `ambiguous_membership` cases | `npm::tests::{unsupported_versions_never_yield_membership, rejects_malformed_documents, undeclared_local_paths_are_ambiguous_but_an_empty_lock_is_not, an_invalid_declaration_glob_is_ambiguous}` |
| Yarn Berry 6/8/10, Classic by signature, scoped names split at the last marker, `link:`/`portal:`/`patch:` excluded, conflicting identities, a missing root, duplicate keys | `lockfile::yarn::tests::*` (corpus plus a generic `Value` reference) |
| Bun JSONC with comments and trailing commas (strict JSON rejects real output), version 1, duplicate or missing `workspaces` | `lockfile::bun::tests::*` (corpus) |
| Real-tool fixtures end to end (corroborating and declined, 1 read and 1 parse, provenance only on `match`) | L1 `lockfile_fixtures::{npm,yarn,bun}_tool_fixtures_report_the_accepted_version_matrix` |
| npm v1 through detection; shrinkwrap beats a disagreeing `package-lock.json` (R6) | L1 `an_npm_v1_lockfile_is_an_unsupported_version_through_detection`, `npm_shrinkwrap_takes_precedence_over_package_lock` |
| `bun.lockb` never read (both request kinds); `bun.lock` wins over `bun.lockb` | L1 `a_binary_bun_lockfile_is_unverifiable_without_a_read`; `precedence-both` row |
| `rush.json` comments regression (the pre-existing bug), lenient fields, manager selection | `repo::npm::tests::*`; L1 `rush_tool_fixture_reports_the_accepted_version_matrix` |
| Rush importer base translation, synthetic `.` excluded, config reads only when corroborating (manifest parse counts 3 vs 1) | `lockfile::tests::{rush_importers_resolve_against_common_temp, a_declined_rush_request_reads_no_configuration}` |
| Every R3 unsupported layout (subspaces, unreadable configs, `useWorkspaces` false/omitted/absent, variants directory and declaration, npm/Yarn/no/several managers), never reading the lockfile; unsupported-without-lockfile is not `absent`; legacy `pnpmOptions` | `lockfile::tests::{unsupported_rush_layouts_are_unverifiable_without_reading_the_lockfile, an_unsupported_rush_layout_without_a_lockfile_is_not_absent, the_legacy_rush_pnpm_option_enables_the_workspace_layout, rush_managers_other_than_pnpm_are_unsupported_layouts}`; L1 `rush_tool_fixture_variants_follow_ruling_r3` (a renamed importer gives `mismatch`; subspaces, `useWorkspaces`, and variants give `unsupported_layout`) |
| Gradle legacy group one level deep, sorted beside the root lockfile, empty or misplaced directory, metadata failure | `lockfile::tests::{legacy_gradle_locks_are_listed_one_level_deep, an_empty_or_misplaced_legacy_gradle_lock_directory_is_absent, a_metadata_failure_on_the_legacy_gradle_directory_is_unreadable}` |
| Bazel gate | `lockfile::tests::a_bazel_root_without_module_bazel_has_no_lockfile_source` |
| Fallback real fixtures (Go, Gradle root, 14-file legacy group, subproject-only `absent` ×2, Bazel), both request kinds, 0 reads and 0 parses | L1 `fallback_tool_fixtures_are_reported_from_metadata_alone` |
| An empty text lockfile is `parse_failed` after exactly one read and one parse, for all six text formats (replaces the Phase 2 stub test) | `lockfile::tests::an_empty_lockfile_is_a_parse_failure_after_one_read` |
| Standalone: root and package roots only (no `vendor/` or `.venv` search), dedupe, sorting, probe count, declined, absent, metadata failure, flat wire shape with a repeated round trip | `lockfile::standalone::tests::*` |
| Standalone on real Poetry/PDM/Composer fixtures, both request kinds, 0 reads, JSON `tool`; a workspace package root; `[]` always serialized | L1 `standalone_tool_fixtures_are_repository_level_observations`, `standalone_lockfiles_at_workspace_package_roots_are_reported`, `an_empty_standalone_list_is_still_serialized` |

Every new test is L1: no tier marker appears in any path segment. Unit tests
are in the lib target. `lockfile_fixtures` was already declared in
`tests/l1/main.rs`. Fixture reads use `include_str!` (unit) or
`manifest_dir!().join("tests/fixtures/lockfiles")` (L1).

### Gates

- `just test` (sniff): 3024 passed, 32 skipped. The first run failed only on
  `cli::repo_structure_json_reports_cargo_subset_evidence_for_matching_lockfile`
  (the new `"standalone_lockfiles": []`; see the Phase 4 handoff above).
- `just lint` (sniff): clean.
- `cargo clippy -p sniff -p sniff-cli --all-targets -- -D warnings`: clean,
  and also clean with `--all-features`.
- `cargo check --all-targets` on all 23 direct reverse dependencies of
  `sniff` (`cargo tree -i sniff --depth 1`): clean after the one
  `claudine-cli` test literal. I did not run a workspace-wide check.
  Claudine's `from_sniff_result_populates_monorepo_topology` and
  `claudine-cli`'s `wrap::env` tests (28) pass.
- `just cross-check sniff --os all`: linux pass (2138), windows pass (2127),
  wsl pass (2138, archive mode). One warning appeared on Windows, an unused
  `KEY_ALL_ACCESS` import at `sniff/lib/src/programs/windows_apps.rs:291`. It
  predates this change, which does not touch that file.
- Validation checkpoint 3: every accepted-version row, including the Phase 2
  rows, has a passing real-fixture L1 test. Every edited variant produces its
  specified status. Every unknown-version fixture and generated case is
  `unverifiable`, never `mismatch`.
- Skipped: nothing. No pre-existing failures.
