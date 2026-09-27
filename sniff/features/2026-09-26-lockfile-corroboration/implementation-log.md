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
source_files_during_phase_4:
    - sniff/cli/src/output/repo_json.rs
    - sniff/cli/src/output/mod.rs
    - sniff/cli/src/output/filesystem/mod.rs
    - sniff/cli/src/output/filesystem/repo.rs
    - sniff/cli/src/output/filesystem/lockfile.rs
    - sniff/cli/tests/l1/main.rs
    - sniff/cli/tests/l1/lockfile_cli.rs
    - sniff/cli/tests/l1/cli.rs
    - sniff/cli/tests/l1/snapshots.rs
    - sniff/cli/tests/l1/snapshots/l1__snapshots__cargo_monorepo_structure_text.snap
    - sniff/cli/tests/l1/snapshots/l1__snapshots__cargo_pnpm_monorepo_structure_text.snap
    - sniff/cli/tests/l1/snapshots/l1__snapshots__pnpm_nx_monorepo_structure_text.snap
    - sniff/cli/tests/l1/snapshots/l1__snapshots__repo_aggregate_json.snap
docs_updated_during_phase_4:
    - sniff/docs/cli/repo_structure.md
    - sniff/cli/README.md
    - sniff/features/2026-09-26-lockfile-corroboration/plan.md
    - sniff/features/2026-09-26-lockfile-corroboration/implementation-log.md
    - sniff/features/2026-09-26-lockfile-corroboration/spec.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
    - .claude/skills/sniff/cli.md
packages:
    - sniff
    - sniff-cli
    - claudine
    - claudine-cli
source_files_during_phase_5:
    - sniff/lib/src/filesystem/repo/lockfile/cargo.rs
    - sniff/lib/src/filesystem/repo/lockfile/membership.rs
    - sniff/lib/src/filesystem/repo/lockfile/tests.rs
    - sniff/lib/src/performance/counters.rs
    - sniff/lib/tests/l1/lockfile_isolation.rs
    - sniff/lib/tests/l1/lockfile_provenance.rs
    - sniff/lib/tests/l1/main.rs
    - sniff/cli/tests/l1/cli.rs
docs_updated_during_phase_5:
    - sniff/lib/README.md
    - sniff/cli/README.md
    - sniff/docs/sniff-library-architecture.md
    - sniff/features/2026-09-26-lockfile-corroboration/measurements.md
    - sniff/features/2026-09-26-lockfile-corroboration/plan.md
    - sniff/features/2026-09-26-lockfile-corroboration/implementation-log.md
    - sniff/features/2026-09-26-lockfile-corroboration/spec.md
docs_created_during_phase_5:
    - sniff/fixes/_unscheduled/npm-dot-slash-workspace-patterns/spec.md
skills_files_updated_during_phase_5:
    - .claude/skills/sniff/SKILL.md
    - .claude/skills/sniff/architecture.md
    - .claude/skills/sniff/performance.md
    - .claude/skills/sniff/remote-and-repository.md
    - .claude/skills/sniff/testing.md
    - .claude/skills/os/SKILL.md
source_code:
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
    - Cargo.lock
    - sniff/lib/Cargo.toml
    - sniff/lib/src/filesystem/repo/jsonc.rs
    - sniff/lib/src/filesystem/repo/nested.rs
    - sniff/lib/src/filesystem/repo/lockfile/standalone.rs
    - sniff/cli/src/output/filesystem/mod.rs
    - claudine/cli/src/commands/wrap/env/tests.rs
    - sniff/cli/src/output/mod.rs
    - sniff/cli/src/output/filesystem/repo.rs
    - sniff/cli/src/output/filesystem/lockfile.rs
    - sniff/cli/tests/l1/main.rs
    - sniff/cli/tests/l1/lockfile_cli.rs
    - sniff/cli/tests/l1/snapshots.rs
    - sniff/cli/tests/l1/snapshots/l1__snapshots__cargo_monorepo_structure_text.snap
    - sniff/cli/tests/l1/snapshots/l1__snapshots__cargo_pnpm_monorepo_structure_text.snap
    - sniff/cli/tests/l1/snapshots/l1__snapshots__pnpm_nx_monorepo_structure_text.snap
    - sniff/lib/tests/l1/lockfile_isolation.rs
documentation:
    - sniff/features/2026-09-26-lockfile-corroboration/spec.md
    - sniff/features/2026-09-26-lockfile-corroboration/plan.md
    - sniff/features/2026-09-26-lockfile-corroboration/accepted-versions.md
    - sniff/features/2026-09-26-lockfile-corroboration/spike-s2-jsonc.md
    - sniff/features/2026-09-26-lockfile-corroboration/spike-s3-parsers.md
    - sniff/features/2026-09-26-lockfile-corroboration/spike-s4-evidence-audit.md
    - sniff/features/2026-09-26-lockfile-corroboration/measurements.md
    - sniff/features/2026-09-26-lockfile-corroboration/implementation-log.md
    - sniff/fixes/_unscheduled/package-manager-uv-label/spec.md
    - sniff/lib/README.md
    - sniff/cli/README.md
    - sniff/docs/dependencies.md
    - docs/dependencies.md
    - sniff/docs/cli/repo_structure.md
    - sniff/docs/sniff-library-architecture.md
    - sniff/fixes/_unscheduled/npm-dot-slash-workspace-patterns/spec.md
completed_phase: 5
implemented: true
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

## Phase 4

One agent did Wave 1 (JSON projection, human output, CLI tests) and the Wave 2
checkpoint. No library source changed; the CLI only projects library
observations.

### JSON projection (`cli/src/output/repo_json.rs`, `cli/src/output/mod.rs`)

- New `repo_json::repo_info_value` serializes a `RepoInfo` and removes an
  empty `standalone_lockfiles` through `omit_empty_standalone_lockfiles`
  (ruling R2). It is used by `repo structure --json` (unfiltered and
  filtered) and the unspecialized fallback. `sniff --json` (`/filesystem/repo`)
  and `sniff filesystem --json` (`/repo`) call the omission helper on the
  nested object. A populated list is emitted unchanged, in the flat wire shape.
- The bare `sniff repo --json` aggregate's `structure` gained
  `standalone_lockfiles`, always an array, because the aggregate always
  carries `monorepo_layers` too ("the same rule"). It is a projection of the
  captured `RepoInfo`; no new observation.
- Layers already carried `lockfile` everywhere because every site serializes
  `MonorepoLayer` directly. No CLI code rediscovers or reinterprets lockfiles.

### Human output (`cli/src/output/filesystem/lockfile.rs`, new)

- `render_lockfile_section` renders a "Lockfiles" section with `Prose` and
  `UnorderedList`: one headline per layer (label, root relative to the
  repository root, status, plain-language explanation), then one line per
  selected lockfile, per `missing` member, and per `extra` member. Standalone
  entries follow the layers (tool label, root `.` for the repository root).
  Paths and labels go through `Prose::escape_text`.
- The explanation derives from the reason first, then the status. Every R5
  pairing has its own text, including `unsupported_layout` (Rush managers,
  subspaces, variants, unreadable Rush config), `no_lockfile_source`,
  `ambiguous_membership`, and Cargo's `subset_only` statuses.
- Wired into `render_repo_section` (both single-package and monorepo
  branches, after the package list) and `render_filesystem_section`. The
  section is empty when there is no layer and no standalone entry, so
  single-package output is unchanged.
- `--plain` strips the styling; the section has no glyph-only content, so the
  text fallback reads the same.

### Deviation: no stderr "use the full request" hint

Every CLI command that displays layers (`repo structure`, `sniff`,
`sniff filesystem`, and the bare `repo --json` aggregate, which opts in with
`with_lockfile_provenance(true)`) corroborates. No CLI command can show a
`not_requested` layer, and there is no CLI flag that toggles corroboration.
So the plan's stderr hint naming a flag would be unreachable code. The
`not_requested` headline states the fact ("a lockfile exists, but this request
did not compare it") and names no flag, which keeps CLI hints off stdout. The
renderer unit test covers it. If a structure-tier command ever renders
layers, add the stderr hint there.

### Other observations

- `sniff repo structure` on a single Poetry, PDM, or Composer project
  reports no repository (the CLI uses workspace detection, not
  `detect_repo_with_request_or_root_package`), so standalone entries reach
  the CLI only inside a workspace. That is pre-existing CLI scope, not
  changed here.
- Styled output piped to a file still carries escape codes on this host
  because `COLORTERM=truecolor` is set; that is `Terminal::default()`
  behavior shared by the whole CLI, not new. `--plain` output is clean.

### Requirement-to-test mapping

| Behavior | Tests |
|---|---|
| Empty `standalone_lockfiles` omitted from `repo structure --json` (unfiltered, filtered) and the fallback; the omitted form reads back as empty | `repo_json::tests::monorepo_topology::repo_info_projections_omit_an_empty_standalone_lockfile_list` |
| Populated list kept in the flat wire shape; write/read/write round trip is stable | `repo_json::tests::monorepo_topology::repo_info_projections_keep_a_populated_standalone_lockfile_list` |
| `sniff --json` and `sniff filesystem --json` omit an empty list and keep a populated one | `output::tests::standalone_lockfiles::{an_empty_list_is_omitted, a_populated_list_is_kept}` |
| Aggregate `structure` always carries `standalone_lockfiles` (empty and populated) beside layer `lockfile` | `repo_json::tests::monorepo_topology::aggregate_structure_always_carries_standalone_lockfiles`; `snapshots::repo_aggregate_json_snapshot` (projection now includes the key); L1 `cli::repo_aggregate_json_reports_cargo_subset_evidence_for_matching_lockfile`, `lockfile_cli::aggregate_json_carries_the_layer_lockfile_and_standalone_list` |
| Phase 3 handoff: `expected_lockfile_structure_json()` no longer pins `"standalone_lockfiles": []` | L1 `cli::repo_structure_json_reports_cargo_subset_evidence_for_matching_lockfile` |
| Shipped CLI on real pnpm 10.32.1 fixture copies: `match`, `mismatch` (missing), `mismatch` (extra), `unreadable` (`parse_failed`, malformed trailing content), `unreadable` (`read_failed`, a directory where the lockfile goes), `absent`; exactly one JSON document on stdout, empty stderr, exit 0; provenance upgraded only on `match` for the layer and every package | L1 `lockfile_cli::structure_json_carries_each_layer_lockfile_observation` |
| The same six cases in `--plain` human mode: layer, status, explanation, lockfile path, missing and extra members; not JSON; empty stderr; exit 0 | L1 `lockfile_cli::structure_human_output_names_each_layer_status_paths_and_members` |
| Standalone Poetry lockfile at a package root in JSON and human output | L1 `lockfile_cli::a_standalone_lockfile_is_reported_in_json_and_human_output` |
| `not_requested` layer and standalone entry (no CLI command produces one) say the request skipped them and print no flag | `output::filesystem::lockfile::tests::not_requested_layers_and_standalone_lockfiles_say_the_request_skipped_them` |
| Each missing and extra member on its own line; a multi-file Gradle group lists every file on its own line; no section without layers or standalone entries | `lockfile::tests::{a_mismatch_lists_each_missing_and_extra_member, a_multi_file_lockfile_group_lists_each_file_on_its_own_line, nothing_is_rendered_without_a_layer_or_standalone_lockfile}` |
| No two R5 status/reason pairings share an explanation | `lockfile::tests::each_status_and_reason_pairing_has_its_own_explanation` |
| Existing text snapshots now end with the Lockfiles section (`absent` layers) | `snapshots::{cargo_monorepo_structure_text_snapshot, cargo_pnpm_monorepo_structure_text_snapshot, pnpm_nx_monorepo_structure_text_snapshot}` |

Regression check: with the omission helper neutered, four of these tests
fail (`an_empty_list_is_omitted`,
`repo_info_projections_omit_an_empty_standalone_lockfile_list`,
`structure_json_carries_each_layer_lockfile_observation`, and
`repo_structure_json_reports_cargo_subset_evidence_for_matching_lockfile`).

Placement: every new test is L1 and has no tier marker in any path segment.
Unit tests live in the `sniff-cli` lib target. The new integration module
`tests/l1/lockfile_cli.rs` is declared in `tests/l1/main.rs`
(`test_layout::every_test_source_is_compiled_by_a_declared_target` passes). It
reads the library's fixtures through
`Path::new(env!("CARGO_MANIFEST_DIR")).parent()...join("lib/tests/fixtures/lockfiles")`,
a form `docs/cicd/test-inputs.md` indexes. The fixtures are copied into a
tempdir and no package-manager binary runs.

### Snapshot and expectation changes

- `l1__snapshots__{cargo_monorepo,cargo_pnpm_monorepo,pnpm_nx_monorepo}_structure_text.snap`:
  added the Lockfiles section. Headers kept as they were; only bodies
  changed.
- `l1__snapshots__repo_aggregate_json.snap`: `structure` gained
  `"standalone_lockfiles": []`; `snapshots.rs`'s stable projection now copies
  that key.
- `cli.rs`: `expected_lockfile_structure_json()` dropped
  `"standalone_lockfiles": []`; the aggregate test asserts
  `structure.standalone_lockfiles == []`.

### Docs and skill

- `sniff/docs/cli/repo_structure.md`: the Lockfiles section, the layer
  `lockfile` object in the JSON example, and `standalone_lockfiles`.
- `sniff/cli/README.md`: the `repo structure` JSON row mentions
  `standalone_lockfiles` omission and the aggregate.
- `.claude/skills/sniff/cli.md`: a "Lockfile observations" section (the
  omission chokepoint every serializer site must use, and why no CLI command
  shows `not_requested`).
- The full documentation pass remains Phase 5.

### Gates

- `just test` (sniff): 3038 passed, 32 skipped.
- `just lint` (sniff): clean.
- `cargo clippy -p sniff -p sniff-cli --all-targets -- -D warnings`: clean,
  and also clean with `--all-features`. The first run flagged two
  `needless_borrows_for_generic_args` in the new unit tests, which were fixed.
- Validation checkpoint 4: `sniff --base <copy of pnpm-10.32.1/workspace-edited-stale-extra> repo structure --json | jq`
  showed the `mismatch` object with `extra: ["packages/gamma"]` and no
  `standalone_lockfiles` key. Human output was reviewed on stdout and
  redirected to a file (exit 0, 0 bytes on stderr).
- `just check-tier-coverage sniff`: 0 stranded tests.
- `just cross-check sniff --os all`: linux pass (2138), windows pass (2127),
  wsl pass (2138, archive mode). That recipe's package argument selects the
  `sniff` lib only, so the CLI was checked separately:
  `just cross-check sniff-cli --os all`: linux pass (877), windows pass (873),
  wsl pass (877). All 17 new or changed Phase 4 tests passed on each of the
  three, including the `unreadable-directory` CLI case on native Windows.
- Skipped: nothing. No pre-existing failures.

## Phase 5

One orchestrating agent wrote the tests and ran the gates. Two background
agents ran concurrently in Wave 1: one for measurements and the passive corpus
pass (`measurements.md` only), and one for documentation (docs, skill files,
and a comment-only source fix).

### Bug found and fixed: Cargo `[workspace].exclude` reported as missing

The corpus pass ran on this repository and got a false `members_missing` for
`darkmatter/dmls/zed-dmls`, which the root `Cargo.toml` lists under
`[workspace].exclude`. The Cargo detector's raw seeds hold an included seed and
an excluded seed for a directory that `members` and `exclude` both match; they
merge only later, in `seed.rs`. `lockfile::cargo::compare` treated every seed
as a member. Cargo never locks an excluded directory as a member, so the
lockfile was right and the observation was wrong. An excluded directory with
an unparseable manifest also turned the result into
`incomplete_manifest_discovery`.

- **Fix:** `sniff/lib/src/filesystem/repo/lockfile/cargo.rs` skips every path
  that has an excluded seed. Checking `is_excluded` on each seed is not enough,
  because the included twin remains.
- **Regression test (written first, failed before the fix):**
  `lockfile_isolation::a_cargo_workspace_exclude_is_not_a_missing_member`. It
  covers a legacy directory and an unparseable excluded manifest, under both
  structure and full requests, and a real missing member still reported
  afterward.
- **After the fix,** `sniff repo structure --json` on this repository reports
  `members_present`.
- `pnpm`, `npm`, `yarn`, `bun`, and `uv` detectors never set `is_excluded`,
  so `compare_members` is unchanged.
- **Docs:** `sniff/lib/README.md` (Cargo paragraph) and
  `.claude/skills/sniff/architecture.md`.

### Deviations and observations

- **No workers to propagate into.** `ManifestStore` uses `RefCell` and is not
  `Sync`, so all lockfile probing, reading, and parsing happens on the
  detecting thread. The counter-propagation task has nothing to change.
- **`unknown_standard`, `metadata_failed`, `ambiguous_membership`,
  `incomplete_manifest_discovery`, and `invalid_member_path` are not in the L1
  complete-JSON matrix.**
  - `Unknown` never owns a layer (ruling R7). The orchestrator-only cases show
    its inferred standard entry and that `monorepo_layers` is absent.
  - The metadata seam is `#[cfg(test)]` and crate-private (R9).
  - All five are pinned by the library's unit tests: `lockfile::tests` and the
    `detection` tests.
- **The new complete-JSON matrix is structure-tier.** The existing
  Cargo/pnpm/uv matrix still covers the full tier, and the added cases stay
  independent of host tools.
- **Leaf-marker member order is filesystem-dependent.** Bazel, Pants, and
  Buck2 list layer members in directory-walk order: `app, lib` on APFS and
  NTFS, `lib, app` on ext4. The first Linux and WSL run of the new matrix was
  red for this reason only.
  - The normalizer sorts those lists, and only those.
  - The fact is recorded in the `os` skill.
  - This ordering predates the feature and was not changed.
- **Pre-existing, not changed:**
  - A Cargo layer's `packages` lists a directory twice when `members` and
    `exclude` both match it (visible in the regression test's debug output).
    Ownership and catalogs are unaffected.
  - npm workspace patterns written `"./dir/*"` match no members. Recorded as
    `sniff/fixes/_unscheduled/npm-dot-slash-workspace-patterns/spec.md`.
- **A dangling lockfile symlink is `absent`.** Metadata follows the link and
  gets `NotFound`, which is absence under the same rule as a missing parent
  directory. This is pinned by `symlinked_lockfiles_and_roots_are_followed`.
- **A stale test comment was fixed.**
  `create_cli_monorepo_with_matching_cargo_lock` in
  `sniff/cli/tests/l1/cli.rs` still said a matching `Cargo.lock` "reports
  lockfile provenance"; Cargo reports `members_present`. Comment only.

### Requirement-to-test mapping

| Behavior (plan task / AC) | Tests |
|---|---|
| Nested layers with different authorities read only their own lockfile, compare members relative to their own root, never borrow an ancestor's lockfile, and upgrade only owned packages (AC3) | L1 `lockfile_isolation::nested_layers_with_different_authorities_observe_only_their_own_lockfile` (root Cargo, nested pnpm `web/` match, nested pnpm `docs/` `absent` despite a decoy root `pnpm-lock.yaml`, 2 reads) |
| Overlapping layers at one root (Yarn and npm) never share another authority's lockfile | L1 `overlapping_layers_at_one_root_never_share_another_authority_lockfile`; complete JSON in `lockfile_provenance::every_other_authority_reports_its_complete_repository_result` ("Yarn match beside an overlapping lockfile-less npm layer") |
| Two layers sharing one lockfile: one probe, read, and parse, for success and a cached parse failure | unit `lockfile::tests::layers_sharing_one_lockfile_read_and_parse_it_once`; existing `detection::tests::cargo_lock_is_shared_by_corroboration_and_dependency_enrichment` |
| Repeat calls with a cheaper request inherit no upgrade; no cross-request cache hides an edited file | L1 `repeat_calls_with_a_cheaper_request_inherit_no_upgrade` (corroborate, then `structure`, then `full` declined, then edit and corroborate) |
| Filename precedence: a failed or unsupported selected file is never retried with a lower-priority file | L1 `a_failed_selected_lockfile_is_never_retried_with_a_lower_priority_file` (malformed, v1, and directory `npm-shrinkwrap.json` beside a matching `package-lock.json`; malformed `bun.lock` beside `bun.lockb`); existing `lockfile::tests::only_the_first_present_candidate_is_selected` |
| Directory where the lockfile should be (R9) | the same L1 test (directory case), matrix case "npm directory in place of the lockfile"; existing `detection::tests::a_directory_in_place_of_the_lockfile_is_a_read_failure`, CLI `lockfile_cli` `unreadable-directory` |
| Metadata-failure seam (R9) | existing `lockfile::tests::{a_metadata_failure_is_not_skipped_for_a_lower_priority_candidate, a_metadata_failure_is_unreadable_even_when_declined}`, `detection::tests::a_metadata_failure_is_unreadable_and_reads_nothing`, the Gradle and standalone seam tests |
| Symlinked lockfile and root spellings, including the macOS `/var` to `/private/var` temporary directory; a dangling link | L1 `symlinked_lockfiles_and_roots_are_followed` (`#[cfg(unix)]`); portable L1 `a_root_spelled_with_dot_components_reports_the_same_observation` |
| `.hidden` members keep their leading dots in match and mismatch | L1 `hidden_members_keep_their_leading_dots` (a `./.tools/lint/` spelling matches; stale `.cache/old` is `extra`); every Node and Rush real fixture's `.tools/hidden` in the matrix; existing unit `membership::tests::normalize_member_keeps_leading_dots_in_names_and_case` |
| Windows-native separator conversion | portable unit `membership::tests::normalize_member_converts_windows_separators` (existing); `#[cfg(windows)]` unit `membership::tests::manifest_member_converts_native_windows_paths` (new, including `\\?\` verbatim roots); `#[cfg(windows)]` L1 `a_windows_native_root_spelling_yields_slash_separated_members` (new: `\` root, lower-case drive, `\` in the glob) |
| Disabled structure request: 0 reads, 0 parses, probes > 0, probe cost independent of presence, no added walk (AC4) | L1 `a_declining_structure_request_probes_without_reading_or_walking` (walk, read-dir, nested-marker, and glob counters equal across absent, declined, and corroborated) |
| Enabled request reads and parses each selected file once; a cached failure is not retried by the full request's later consumers | L1 `an_enabled_request_reads_and_parses_each_selected_lockfile_once` (Cargo, malformed npm, pnpm; structure and full: 3 reads, 3 parses) |
| Full request with corroboration off still reads `Cargo.lock` for dependency versions, reports `not_requested`, never upgrades; a malformed lock is read once | L1 `a_full_request_declining_corroboration_still_reads_cargo_lock_for_versions` |
| Complete serialized `RepoInfo` for every other authority and every reachable status: npm (match, mismatch extra, mismatch missing, parse_failed, unsupported_version, read_failed, absent, not_requested), Yarn plus overlapping npm, Bun text and binary, Rush match and unsupported_layout, Go (unverifiable, absent), Gradle, Bazel, Maven, .NET, Pants, Buck2 (not_applicable), and Nx-, Turborepo-, and Lerna-only (no layer, `unknown` standard) (AC1) | L1 `lockfile_provenance::every_other_authority_reports_its_complete_repository_result` (24 cases, hand-written documents, per-case read/parse counts, typed round trip); existing Cargo/pnpm/uv matrix |
| Cargo `[workspace].exclude` is not a missing member (regression) | L1 `lockfile_isolation::a_cargo_workspace_exclude_is_not_a_missing_member` |

Regression checks:

- The exclude test failed before the fix, first with
  `incomplete_manifest_discovery` and then with the per-seed-only version of
  the fix.
- Mutating one expected provenance in the matrix (Rush subspaces `explicit`
  to `globbed`) makes it fail.

Placement:

- Every new test is L1, and no path segment carries a tier marker.
- `tests/l1/lockfile_isolation.rs` is declared in `tests/l1/main.rs`.
- The matrix reads fixtures through
  `biscuit_test_harness::manifest_dir!().join("tests/fixtures/lockfiles")`.
- `just check-tier-coverage sniff` finds 0 stranded tests.

### Measurements and corpus (`measurements.md`)

- Release-mode production parsers, measured through the public API with a
  throwaway harness at `/tmp/lockfile-measure`: 200 members and 20,000
  packages per format, 1 warm-up and 7 timed runs, warm page cache.
  - Added time with corroboration on:
    - npm 5.9 ms
    - Bun 5.9 ms
    - pnpm 61.8 ms
    - Yarn 80.6 ms
    - uv 69.1 ms
    - Cargo 24.0 ms
  - Peak: 1.1–1.3x the input for JSON and JSONC, 10–13x for YAML, and 36–41x
    for TOML, the same as a generic `toml::Value` parse.
  - Nothing is retained after the call.
- Corpus pass over 15 local checkouts: no `unreadable` result. The two defects
  it found are handled above (the Cargo exclude fix and the unscheduled npm
  spec). The other unexpected rows are real stale lockfiles and one Yarn 3.1
  `unsupported_version`.

### Documentation and skills

- `sniff/lib/README.md`: a "Lockfile Corroboration" section with the breaking
  note, the status, reason, and source tables, root exclusion, Cargo semantics
  (now including `exclude`), provenance, and request cost. Also Key Types and a
  `jsonc-parser` dependency row.
- `sniff/cli/README.md`: a "Lockfile Observations" section. It names no flag
  and does not describe standalone single projects as visible.
- `sniff/docs/sniff-library-architecture.md`: a Lockfile Corroboration
  section.
- `sniff/docs/cli/repo_structure.md` was verified unchanged. `just docs_cli`
  was not needed, because no clap help text changed.
- `sniff/docs/dependencies.md` and `docs/dependencies.md` already list
  `jsonc-parser` (verified).
- `.claude/skills/sniff/`:
  - `architecture.md`: a lockfile pipeline section and the Cargo exclude rule.
  - `performance.md`: probing stops at the first present candidate.
  - `remote-and-repository.md`: the layer `lockfile` and
    `standalone_lockfiles`.
  - `SKILL.md`: trimmed to 199 lines by moving the testing guidance verbatim
    into the new `testing.md`.
- `.claude/skills/os/SKILL.md`: the ext4 `read_dir` order fact.
- `sniff/lib/src/performance/counters.rs`: the `REPO_LOCKFILE_PROBES` doc
  comment now describes the precedence-ordered probing and the standalone
  probes (comment only).
- Final `rg lockfile_match` sweep (outside `_completed` and this feature's own
  documents): only the documented breaking-change notes remain, plus the L1
  assertion that the key is gone.

### Gates

- `just test` (sniff): 3051 passed, 32 skipped.
- `just lint` (sniff): clean.
- `cargo clippy -p sniff -p sniff-cli --all-targets -- -D warnings`: clean,
  and clean with `--all-features`. One `type_complexity` hit in a new test was
  fixed by a named case struct.
- `cargo check --all-targets` on all 22 direct reverse dependencies of
  `sniff`, from `cargo tree -i sniff --depth 1`: clean.
- `cargo nextest run -p claudine`: 4358 passed.
- `claudine-cli` `wrap::env`: 28 passed.
- Darkmatter `lazy_roots` (the `2026-09-20-repo-perf` boundary tests): 20
  passed.
- `just cross-check sniff --os all`, on the final tree:

  | OS | Result |
  |---|---|
  | Linux | 2151 passed |
  | Windows | 2141 passed |
  | WSL | 2151 passed (archive mode) |

  The +13 (Linux) and +14 (Windows) against Phase 4 are exactly this phase's
  new tests, which shows that the `#[cfg(windows)]` cases ran on Windows.
- `just cross-check sniff-cli --os all`:

  | OS | Result |
  |---|---|
  | Linux | 877 passed |
  | Windows | 873 passed |
  | WSL | 877 passed |

- Pre-existing Windows warnings, not from this change: an unused
  `KEY_ALL_ACCESS` in `programs/windows_apps.rs`, unused `index` bindings in
  lib tests, and an unused `stage_raw_path` in `git_parity.rs`.
- No CI matrix cells were added and no event scheduling changed.
- Skipped: nothing. No pre-existing failures.
- Not done by this agent: commits, signing, and `git verify-commit`. The
  session was told not to stage or commit. The breaking-change commit needs
  the `!` marker (R11) and must list the changed expectations recorded in the
  Phase 2–4 logs.

### Success criteria evidence (plan "Success looks like")

1. `lockfile_match` is gone. See the sweep above and the L1 assertion at
   `lockfile_fixtures.rs:372`.
2. The every-authority complete-JSON matrix (24 cases) and the
   Cargo/pnpm/uv × state matrix, with orchestrator-only cases.
3. Real-tool fixture tests (Phases 2 and 3, `lockfile_fixtures`) cover every
   `accepted-versions.md` row and the negative variants.
4. `lockfile_isolation` plus the unit tests mapped above.
5. The counter tests mapped above.
6. `lockfile_cli` (Phase 4) runs JSON and human modes through the shipped
   binary.
7. `measurements.md` Phase 5 sections, and the gates above on all four OSes.
8. The READMEs, `sniff/docs`, skills, and dependency docs listed above.
