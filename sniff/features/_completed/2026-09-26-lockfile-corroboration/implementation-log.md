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
implementation_1: "2026-09-27T06:32:42-07:00"
implementation_2: "2026-09-27T09:08:49-07:00"
implementation_3: "2026-09-27T10:08:47-07:00"
implementation_4: "2026-09-27T10:49:55-07:00"
implementation_5: "2026-09-27T11:05:31-07:00"
implementation_6: "2026-09-27T11:25:40-07:00"
implementation_7: "2026-09-27T12:09:19-07:00"
implementation_8: "2026-09-27T12:34:16-07:00"
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

## Implementation of Review Findings #1

> **started at:** 2026-09-27T06:32:42-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/fix-sniff/sniff/features/2026-09-26-lockfile-corroboration/review-1.md'
- this is iteration 1 of the review-to-implement cycle
- starting the work on 'Incomplete manifest discovery can produce an exact lockfile match' at 06:33:23
        - discovery: `expand_membership_globs` silently dropped a Cargo pattern outside Cargo's subset, any pattern `globset` could not parse (include or `!` exclude), walk errors (`filter_map(Result::ok)`), and glob-free members naming a missing path; the shared observation walk also discarded its errors before handing `manifest_dirs` to the expander
        - discovery: the expander has no count bound, and the shared walk keeps collecting manifests past the inventory cap, so the R10 "glob expander reported its bound" case has no bound to report; walk failures stand in for it
        - decision: the expander now returns `MembershipExpansion { seeds, patterns_resolved, missing_literal }`; `DetectorOutcome` gains a crate-private `incomplete` flag set by each detector, and detection passes an incomplete outcome to `observe_layer_lockfile` as `None`, the existing "manifest-side set incomplete" channel, so incompleteness blocks only the comparison step and `absent`/`not_requested`/`unreadable`/`unsupported_version`/`ambiguous_membership` keep precedence
        - decision (per detector): a missing glob-free member is incomplete for Cargo and Rush, which reject it; for npm, pnpm, Yarn, Bun, and uv it is a glob that matched nothing, as the tools themselves treat it; a glob matching no directory is never incompleteness; a Cargo `exclude` naming a missing path is harmless, but an unparseable exclude pattern is incomplete
        - decision: a member's own manifest is checked at comparison time (`member_manifest_resolves` in `lockfile/mod.rs`): `package.json` for npm, pnpm, Yarn, Bun, and Rush, `pyproject.toml` for uv; missing or unparseable is incomplete, matching Cargo's existing `resolved_identity` rule, whose parse-failure test is unchanged; the check parses through the request's `ManifestStore`, so full detection's enrichment reuses it, and only an opted-in structure request pays one parse per member
        - decision: shared-walk failures are recorded as `FilesystemSystemView::manifest_walk_errors` / `RepoEvidence::manifest_walk_errors`, and only a failure inside or above a glob's walk root makes that expansion incomplete; only I/O-rooted `ignore::Error`s count (`walk_failure_path`), so an unparseable ignore rule hides nothing
        - discovery (not changed, out of scope): a malformed member `package.json` fails the whole detection today, because nested discovery dispatches the npm detector at every member and its `required_npm` propagates the parse error; no JS layer is observed at all, so the false `match` is unreachable through the public API for JS authorities; the L1 JS cases accept that error or, if it is ever relaxed, require the full `incomplete_manifest_discovery` observation; the engine-level unit test covers the JS comparison path directly
        - files changed: `sniff/lib/src/filesystem/repo/{glob.rs, topology.rs, detection.rs, cargo.rs, npm.rs, uv.rs, nx_turbo.rs, dotnet.rs, go.rs, gradle.rs, maven.rs, polyglot.rs, lockfile/mod.rs, lockfile/tests.rs}`, `sniff/lib/src/filesystem/system_view.rs`, `sniff/lib/tests/l1/{lockfile_isolation.rs, lockfile_provenance.rs}` (comment only), `.claude/skills/sniff/architecture.md`
        - tests added (L1, `lockfile_isolation`): `a_malformed_uv_member_pyproject_is_incomplete_manifest_discovery`, `a_dropped_workspace_glob_is_incomplete_manifest_discovery` (pnpm, unparseable `tools/[`), `an_unresolved_cargo_member_pattern_is_incomplete_manifest_discovery` (brace pattern, missing literal), `a_missing_rush_project_folder_is_incomplete_manifest_discovery` (with a `mismatch` control), `a_malformed_pnpm_member_manifest_never_matches`, `a_malformed_npm_member_manifest_never_matches`, and the negative control `patterns_matching_nothing_still_match_the_lockfile`; each asserts the full observation under a corroborating structure request and a full request, plus layer and package provenance
        - tests added (unit): `lockfile::tests::a_member_without_a_parseable_manifest_is_incomplete` (pnpm and npm, malformed and missing member manifest, with a `match` control); in `glob.rs`, `a_fully_understood_expansion_is_complete`, `a_dropped_pattern_is_reported_unresolved`, `a_missing_literal_member_is_reported_separately`, `an_observed_walk_failure_is_unresolved_only_where_a_glob_walks`, `only_io_walk_errors_have_a_failure_path`
        - mutation check: with the flag and the manifest check disabled, the uv, dropped-glob, Cargo, and engine tests fail; the two JS L1 cases pass either way, because detection errors first (see above)
        - changed expectations: none; every existing test passed unchanged
        - docs: `observe_layer_lockfile` doc updated for the new `None` meaning and precedence; the sniff skill's architecture notes describe the R10 signals; the `lockfile_provenance.rs` coverage comment now points to the new L1 cases; the README reason table was already accurate
        - results: `just test` in `sniff/` passed 3064 tests (32 skipped); `just lint` was clean; `cargo clippy -p sniff -p sniff-cli --all-targets -- -D warnings` was clean
- work completed for 'Incomplete manifest discovery can produce an exact lockfile match' at 06:45:35
- starting the work on 'The uv parser treats missing membership data as an empty set' at 06:46:12
        - discovery: the real-tool `single-project` and `root-only-workspace` fixtures both omit `[manifest]` and carry exactly one local package, the root at `source = { virtual = "." }`; that is the only shape the document itself establishes as root-only
        - discovery: through detection a root-only uv workspace has no layer (the detector requires a non-empty `members` array), but a workspace whose `members = ["py/*"]` matches nothing does, with an empty discovered set once the root is excluded; that is the setup that exposed the false `match`
        - decision: a present `[manifest]` without `members` is a parse error (`unreadable`/`parse_failed`); `UvManifest::members` is now `Option<Vec<String>>` instead of `#[serde(default)]`
        - decision: an absent `[manifest]` is the empty set only when the document's local packages (`editable`, `virtual`, or `directory` sources) are exactly one, at `.`; any other shape (no packages, no root record, root plus another local package, including a `directory` path dependency) is the new crate-private `ParsedLockfile::NoMembershipData`, classified `unverifiable`/`no_membership_data`; counting `directory` sources is deliberately conservative, and costs nothing observable today because a single project with a path dependency is not a uv workspace layer
        - files changed: `sniff/lib/src/filesystem/repo/lockfile/{uv.rs, mod.rs}`, `sniff/lib/tests/l1/{lockfile_fixtures.rs, lockfile_isolation.rs}`, `sniff/lib/tests/fixtures/lockfiles/uv-0.9.5/workspace-edited-missing-required-field/PROVENANCE.md` (appended a "Superseded" note under the Phase 1 ruling), `sniff/lib/README.md` (lockfile table row), `accepted-versions.md` (uv accepted-layout cell and the `workspace-edited-missing-required-field` expected row)
        - tests added (L1, `lockfile_isolation`): `a_uv_manifest_without_members_is_a_parse_failure` and `an_absent_uv_manifest_beside_other_local_packages_has_no_membership_data`, each run against an empty and a one-member discovered set, plus the control `an_absent_uv_manifest_with_only_the_root_package_matches_an_empty_member_set`; each asserts the full observation, layer provenance, and every uv package's provenance under a corroborating structure request and a full request
        - tests added (unit, `lockfile::uv`): `an_absent_manifest_is_root_only_only_when_the_root_is_the_sole_local_package` (virtual and editable root), `an_absent_manifest_with_other_local_packages_has_no_membership_data` (six shapes), and a "manifest without members" row in `rejects_malformed_accepted_documents`
        - changed expectations: `uv-0.9.5/workspace-edited-missing-required-field` moves from `mismatch` with `missing` = `.tools/hidden`, `packages/alpha`, `packages/beta` to `unverifiable`/`no_membership_data` with empty `extra`/`missing` (L1 `lockfile_fixtures`), and from `Members([])` to `NoMembershipData` in the parser's fixture matrix
        - mutation check: with the old defaults restored, both new L1 membership tests and the fixture matrix fail; restored afterward
        - docs: the `uv.rs` module and `parse` docs describe the root-only rule and the new error; the `UvSource` doc notes why `directory` is read
        - results: `just test` in `sniff/` passed 3069 tests (32 skipped); `just lint` was clean; `cargo clippy -p sniff -p sniff-cli --all-targets -- -D warnings` was clean
- work completed for 'The uv parser treats missing membership data as an empty set' at 06:50:44
- starting the work on 'Rush silently drops invalid lockfile member paths' at 06:51:12
        - discovery: the real-tool `rush-5.179.0/pnpm-workspace` lockfile writes the synthetic `common/temp` project as the importer key spelled exactly `.`; `without_synthetic_project` instead dropped every key whose `normalize_member(key, &[])` failed or was empty, so absolute, drive-prefixed, and NUL-containing importers never reached the shared validator
        - decision: drop only the key that is exactly `.`; every other key, including other root spellings such as `./` or `""`, goes to `recorded_member_set`, so an invalid key yields `unverifiable`/`invalid_member_path` and any other root spelling is compared (as `common/temp`) rather than hidden
        - files changed: `sniff/lib/src/filesystem/repo/lockfile/rush.rs` (filter, doc comment, removed the `normalize_member` import, new unit tests), `sniff/lib/src/filesystem/repo/lockfile/tests.rs`, `sniff/lib/tests/l1/lockfile_isolation.rs`
        - tests added (L1, `lockfile_isolation`): `an_invalid_rush_importer_is_an_invalid_member_path`, with absolute (`/opt/elsewhere/pkg`), drive (`C:/elsewhere/pkg`), and NUL-escaped importers beside otherwise matching members; asserts the full observation (`unverifiable`, `invalid_member_path`, paths `common/config/rush/pnpm-lock.yaml`, empty extra/missing), `explicit` layer and package provenance, exactly one lockfile read and one parse despite a matching decoy at `common/temp/pnpm-lock.yaml`, and a control where removing the invalid key gives `match` with `lockfile` provenance
        - tests added (unit): `lockfile::tests::an_absolute_rush_importer_is_an_invalid_member_path`; in `lockfile::rush::tests`, `only_the_synthetic_dot_importer_is_dropped`, `invalid_importers_survive_for_the_shared_validator`, `other_root_spellings_are_not_synthetic`, `non_member_parses_pass_through`
        - mutation check: with the old filter restored, the L1 test, the engine test, and two of the rush unit tests fail; restored afterward
        - changed expectations: none; every existing Rush test and the real-tool fixture matrix pass unchanged
        - results: `just test` in `sniff/` passed 3075 tests (32 skipped); `just lint` was clean; `cargo clippy -p sniff -p sniff-cli --all-targets -- -D warnings` was clean
- work completed for 'Rush silently drops invalid lockfile member paths' at 06:54:55
- starting the work on 'Styled terminal output has no real-terminal verification' at 06:55:35
        - discovery: the sniff-cli L2 suite is the `level2` target (`tests/level2/main.rs`, `required-features = ["test-fixtures"]`, `autotests = false`); `just test-l2` in `sniff/` runs it through `_test_l2 sniff-cli --features test-fixtures`; `level2_perf_tree_rendering` is the precedent for running the real `sniff` binary in an owned, resized tmux pane
        - discovery: in a TTY the Lockfiles section uses the default `- ` bullet at both levels, hangs a wrapped headline two columns in, and nests the detail items four columns in
        - decision: an owned tmux session (`TmuxHarness::new()` + `resize(56, 40)`) rather than the broker's shared pane, because width is the contract under test; the binary is resolved with `bin_exe!("sniff")`, and HOME/XDG config/cache point into a `SniffCliFixture`
        - decision: the fixture is written inline (pnpm 9.0 lockfile with importers `.`, `alpha`, `beta`, stale `gamma`; manifest members `alpha`, `beta`, `delta`), so the test reads no repository file and needs no test-input declaration
        - files changed: `sniff/cli/tests/level2/level2_lockfile_rendering.rs` (new), `sniff/cli/tests/level2/main.rs` (`mod level2_lockfile_rendering`)
        - tests added (L2, tmux): `level2_lockfile_rendering::level2_lockfile_mismatch_renders_wrapped_and_styled_in_tmux` asserts, at 56 columns: no ESC, SGR fragment, or markup tag is visible; the top-level `- pnpm workspaces (.): mismatch` item wraps, each continuation row hangs two columns in, and the joined text ends with the full explanation (not truncated); the nested `- ` items at four columns are exactly the selected path, the missing member, and the extra member, in that order; no row exceeds the pane width; in raw, `mismatch` and both member labels are red, the layer label is bold, and the selected-path row is not red
        - verification: a deliberately wrong expected explanation made the test fail with the captured pane in the message, so it executed; with `BISCUIT_TEST_REQUIRED_BACKENDS=tmux` a skip would have failed
        - results: `just test-l2` in `sniff/` ran 7 tests, 7 passed, including the new one (executed, not skipped, on this macOS host via tmux); `just check-tier-coverage sniff` and `sniff-cli` reported nothing stranded; `just test` passed 3075 tests (32 skipped); `just lint` was clean; `cargo clippy -p sniff -p sniff-cli --all-targets -- -D warnings` and the same for `sniff-cli --features test-fixtures` were clean
- work completed for 'Styled terminal output has no real-terminal verification' at 07:01:04
- cross-OS verification of the combined review changes (orchestrator, 07:02–07:11)
        - finding 1 adds path comparison between walk errors and glob search directories, so both affected packages were cross-checked
        - `just cross-check sniff --os all`: linux pass, windows pass, wsl pass
        - `just cross-check sniff-cli --os all`: linux pass, windows pass, wsl pass
        - no receipt was published because the tested tree is uncommitted; these runs are local evidence, not reusable CI receipts
- follow-up notes for the reviewer (not deferrals; each finding is fully fixed)
        - a malformed nested member `package.json` still makes npm-family and Rush detection fail as a whole before corroboration runs; the L1 tests accept that error or require `incomplete_manifest_discovery`, and whether detection should tolerate the malformed file is a candidate follow-up fix
        - the spec's "glob expander reported its bound" signal has no source: the expander has no size bound, so walk errors and dropped patterns are the signals used
        - uv: a lone `directory` source beside the root also rules out root-only membership; this is stricter than necessary but cannot happen through detection today

### Successful Completion

The implementation of review cycle 1 has completed successfully in 39 minutes. During this implementation all 4 review findings were evaluated to see if they could be fixed as a part of this implementation cycle: 4 were fixed, 0 were deferred (see reasons below):

- no findings were deferred

The files changed during this cycle are:

- library source: `sniff/lib/src/filesystem/repo/{glob,topology,detection,cargo,npm,uv,nx_turbo,dotnet,go,gradle,maven,polyglot}.rs`, `sniff/lib/src/filesystem/repo/lockfile/{mod,uv,rush}.rs`, `sniff/lib/src/filesystem/system_view.rs`
- library tests: `sniff/lib/src/filesystem/repo/lockfile/tests.rs`, `sniff/lib/tests/l1/{lockfile_isolation,lockfile_fixtures,lockfile_provenance}.rs`, `sniff/lib/tests/fixtures/lockfiles/uv-0.9.5/workspace-edited-missing-required-field/PROVENANCE.md`
- CLI tests: `sniff/cli/tests/level2/level2_lockfile_rendering.rs` (new), `sniff/cli/tests/level2/main.rs`
- docs: `sniff/lib/README.md`, `.claude/skills/sniff/architecture.md`, `sniff/features/2026-09-26-lockfile-corroboration/accepted-versions.md`

## Implementation of Review Findings #2

> **started at:** 2026-09-27T09:08:49-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/fix-sniff/sniff/features/2026-09-26-lockfile-corroboration/review-2.md'
- this is iteration 2 of the review-to-implement cycle
- the review contains one finding: 'Malformed Node member manifests abort repository detection instead of yielding an incomplete observation' (high)
- starting the work on 'Malformed Node member manifests abort repository detection instead of yielding an incomplete observation' at 09:09:10
        - delegated to a `rust-developer` subagent using the `rust`, `rust-testing`, and `sniff` skills
        - discovery: nested discovery dispatches the npm detector at every nested `package.json` and parses it before checking for a lockfile, so a malformed member failed detection for npm, pnpm, Yarn, Bun, and Rush workspaces alike (Yarn and Bun reach the error through the npm dispatch at the same directory)
        - decision: at a nested directory, a `package.json` that fails to parse declares no workspace and is not treated as an npm, Yarn, or Bun root; incompleteness is reported through the existing round-1 `member_manifest_resolves` check, so no new flag was added
        - decision: only a parse failure is tolerated at a nested candidate; an unreadable nested `package.json` and a malformed workspace root manifest remain fatal
        - decision: a malformed `package.json` that belongs to no workspace is now skipped with a debug log instead of failing detection
        - files changed:
                - `sniff/lib/src/filesystem/repo/detection.rs`: crate-private `ManifestStore::npm_unless_malformed` (parse failure yields `Ok(None)`, read failure stays an error; shares the request cache, so parse count is unchanged)
                - `sniff/lib/src/filesystem/repo/nested.rs`: skip before dispatching the npm, Yarn, or Bun detector when the nested `package.json` is malformed
                - `sniff/lib/tests/l1/lockfile_isolation.rs`: shared helper `assert_malformed_js_member_never_matches`; the two tests that accepted the error were replaced
                - `.claude/skills/sniff/architecture.md`: paragraph describing the old fatal behavior replaced
        - tests replaced (L1): `lockfile_isolation::a_malformed_pnpm_member_manifest_never_matches` and `lockfile_isolation::a_malformed_npm_member_manifest_never_matches` became `a_malformed_pnpm_member_manifest_is_incomplete_manifest_discovery` and `a_malformed_npm_member_manifest_is_incomplete_manifest_discovery`; each now requires the complete observation (`unverifiable`, `incomplete_manifest_discovery`, the lockfile path, empty `extra` and `missing`) under structure and full requests, unchanged layer and package provenance, both members listed, and `match` after the member is repaired
        - tests added (L1):
                - `lockfile_isolation::a_malformed_yarn_member_manifest_is_incomplete_manifest_discovery`
                - `lockfile_isolation::a_malformed_bun_member_manifest_is_incomplete_manifest_discovery`
                - `lockfile_isolation::a_malformed_rush_member_manifest_is_incomplete_manifest_discovery`
                - `lockfile_isolation::a_malformed_workspace_root_manifest_fails_detection` (npm and pnpm roots, both requests)
        - tests added (unit): `filesystem::repo::detection::tests::npm_unless_malformed_declines_only_a_parse_failure`
        - mutation check: with the nested skip disabled, all five member tests failed while the root-malformed and uv tests passed
        - not changed (possible follow-up): a malformed nested `Cargo.toml`, `pyproject.toml`, or `pnpm-workspace.yaml` under a different kind of root still fails detection through the required-manifest readers
        - verification (from `sniff/`): `just test lockfile_isolation::` 26/26 passed; `just test` 3080 passed, 32 skipped; `just lint` clean; `cargo clippy -p sniff -p sniff-cli --all-targets -- -D warnings` clean
        - cross-OS (orchestrator): `just cross-check sniff --os all` passed on linux, windows, and wsl (2180 tests on the last leg); no receipt was published because the tested tree is uncommitted
- work completed for 'Malformed Node member manifests abort repository detection instead of yielding an incomplete observation' at 09:20:30

### Successful Completion

The implementation of review cycle 2 has completed successfully in 12 minutes. During this implementation all 1 review findings were evaluated to see if they could be fixed as a part of this implementation cycle: 1 were fixed, 0 were deferred (see reasons below):

- no findings were deferred

The files changed during this cycle are:

- library source: `sniff/lib/src/filesystem/repo/detection.rs`, `sniff/lib/src/filesystem/repo/nested.rs`
- library tests: `sniff/lib/tests/l1/lockfile_isolation.rs`
- docs: `.claude/skills/sniff/architecture.md`

## Implementation of Review Findings #3

> **started at:** 2026-09-27T10:08:47-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/fix-sniff/sniff/features/2026-09-26-lockfile-corroboration/review-3.md'
- this is iteration 3 of the review-to-implement cycle
- the review contains two findings: 'PHP-only Composer projects lose their standalone lockfile observation' (high) and 'A root-only uv workspace has no layer to report its lockfile match' (high)
- starting the work on 'PHP-only Composer projects lose their standalone lockfile observation' at 10:08:57
        - discovery: `synthesize_root_package_repo_with_store` (`sniff/lib/src/filesystem/repo/detection.rs`) gated on `detect_package_ecosystem(root) != Unknown`, which knows only Cargo, Node, Python (`pyproject.toml`/`requirements.txt`), and Go markers, so a `composer.json` root synthesized nothing and the R2 probe never ran
        - discovery: Poetry and PDM single projects are not affected, since both carry a `pyproject.toml` (a Python marker); the existing fixture test already proves them without edits
        - discovery: even with a library result, no shipped plain-output command showed a single-package root's standalone lockfile: `repo structure` runs filesystem detection, which synthesizes a root package only when the repo request carries `details` (the aggregate's focused request), so `repo structure` printed nothing and `repo structure --json` printed `{}` for every single-package root, Poetry included
        - decision: a `composer.json` root is synthesized as the one root package, like any other single-package root: `ecosystem` stays `unknown` (no new `PackageEcosystem` variant), `package_managers` stays empty (`detect_package_managers` unchanged, R4), provenance `manifest-scan`, no layer and no standard
        - decision: the root package is named from `composer.json` `name` when present (fixture: `fixture/fixture-root`), falling back to the relative path as for any unnamed manifest; the read is confined to a PHP-only root so workspace members keep their existing names
        - decision: `sniff repo structure` now uses the full repo request plus `RepoDetailRequest::all()`, which only switches on root-package synthesis; single-package roots of every ecosystem now render a `Repository` / `Type: Single-package` section with their standalone lockfiles instead of empty output
        - discovery: `sniff/docs/cli/repo_structure.md` already documented a "Single-Package Repository" summary that `repo structure` never printed; the CLI change makes the code match it (the doc now lists which root manifests qualify)
        - consequence: a degenerate Cargo workspace (`[workspace] members = []`) now renders `Type: Single-package` under `repo structure`, as it already did for the aggregate and `repo packages`; the text snapshot `degenerate_cargo_structure_text` was replaced by whitespace-collapsed assertions (the wrapped root path length differs by OS) and the test renamed to `degenerate_cargo_structure_text_is_single_package`; the JSON test still asserts no topology keys plus `is_monorepo: false`
        - files changed: `sniff/lib/src/filesystem/repo/detection.rs` (synthesis gate, `composer_package_name`, doc fixes), `sniff/lib/src/filesystem/repo/types.rs` (doc), `sniff/cli/src/commands/mod.rs` (`repo structure` plan), `sniff/lib/README.md`, `sniff/cli/README.md`, `sniff/docs/cli/repo_structure.md`, `.claude/skills/sniff/architecture.md`, `.claude/skills/sniff/cli.md`, `accepted-versions.md` (Composer detection note)
        - tests changed: `lockfile_fixtures::standalone_tool_fixtures_are_repository_level_observations` now uses the unmodified Composer fixture and also asserts no monorepo, no standards, and one root package, for full and structure requests
        - tests added: `lockfile_fixtures::a_php_only_root_package_is_named_by_its_composer_manifest` (name, `unknown` ecosystem, no package managers; the mixed PHP-plus-Node case keeps one package and one Composer entry), `lockfile_fixtures::a_composer_lockfile_without_its_manifest_is_no_repository`, `lockfile_cli::a_php_only_project_reports_its_composer_lockfile` (`repo structure --json` and `--plain repo structure`: exit 0, empty stderr, one JSON document, observation present, layers omitted, no layer line)
        - tests changed (CLI): `snapshots::degenerate_cargo_structure_text_is_empty` became `degenerate_cargo_structure_text_is_single_package`; its snapshot file was deleted
        - mutation check: with the `composer.json` gate removed, both new PHP-only tests and the fixture-matrix test failed
        - verification (from `sniff/`): `just test` 3083 passed, 32 skipped; `just lint` clean; `cargo clippy -p sniff -p sniff-cli --all-targets -- -D warnings` clean
        - comment drift fixed: the root-package synthesis docblock claimed a `Cargo.toml` needed `[package]`, but the code (correct) synthesizes any root manifest without workspace members, degenerate workspaces included; the docblock now says so
        - orchestrator review: accepted the `repo structure` request widening in `sniff/cli/src/commands/mod.rs` because the CLI still only reports what the library produces; flagged for the reviewer because it changes `repo structure` output for a degenerate Cargo workspace (`members = []`) from empty to single-package and removed the `degenerate_cargo_structure_text` snapshot
- work completed for 'PHP-only Composer projects lose their standalone lockfile observation' at 10:19:48
- starting the work on 'A root-only uv workspace has no layer to report its lockfile match' at 10:19:48
        - discovery: `uv_workspace_members_from_value` collapsed an absent `members` and an explicit `members = []` into one empty vector, and `detect_uv_workspace` returned no outcome for both; the lockfile engine already matches the root-only fixture (both sets empty once the root is excluded)
        - discovery: the uv detector always lists the root among the layer's `packages`, but `membership_resolves_non_degenerately` counted a lone uv package as non-degenerate (`RootMembership::Always => true`), so admitting the root-only layer would have flipped `is_monorepo` to `true`; the old rule (`test_degenerate_uv_workspace_is_not_a_monorepo`) says a root-only uv workspace is not a monorepo
        - decision: `uv_workspace_members_from_value` returns `Option<Vec<String>>`; only an explicit `members` array declares a workspace, so a missing `[tool.uv.workspace]` table, a table without `members` (e.g. only `exclude`), or a non-array `members` still forms no layer. uv itself treats a members-less table as root-only, but the conservative reading keeps the change to what the finding names
        - decision: `RootMembership::Always` with one package is now degenerate, because for uv that package is the root alone. Side effect: a uv workspace whose `members` globs match nothing (root-only in practice) is also no longer a monorepo, which the old predicate got wrong the same way. Cargo is untouched: its root is never listed in `packages`, and a Cargo `[workspace] members = []` still forms no layer (kept scoped to uv per the finding)
        - decision: root package provenance follows the existing uv member fixtures, where the root seed is among the layer's owned seeds and is upgraded on `match`: under a corroborating request the layer and the root package are `lockfile`; under a declining request both stay `globbed`
        - CLI effect (checked on a copy of the fixture): `sniff repo structure` reports `Type: Single-package` plus a `uv workspace (.)` lockfile `match`; its JSON has `is_monorepo: false`, one `uv-workspace` layer with `packages: [""]`, and `inferred` standard confidence; the aggregate `sniff repo --json` stays `is_monorepo: false` with the one package. No CLI test or snapshot changed
        - files changed: `sniff/lib/src/filesystem/repo/uv.rs` (Option-returning members reader, unit tests), `sniff/lib/src/filesystem/repo/standard.rs` (predicate, its docblock, and unit test), `sniff/lib/tests/l1/lockfile_fixtures.rs`, `sniff/lib/tests/l1/integration.rs`, `sniff/lib/README.md`, `.claude/skills/sniff/architecture.md`, `accepted-versions.md` (expected-results row described the omission)
        - tests: replaced `a_root_only_uv_workspace_has_no_layer_to_corroborate` with `a_root_only_uv_workspace_matches_its_root_only_lockfile` (full request: complete `lockfile` JSON `match`/`["uv.lock"]`/`null`/`[]`/`[]`, layer and root package `lockfile` provenance, one read and one parse, `is_monorepo` false; structure request: `not_requested`/`request_disabled`, `globbed` provenance, zero reads and parses); `test_degenerate_uv_workspace_is_not_a_monorepo` now expects one root-only layer instead of none; uv.rs unit tests `members_absent_when_table_or_key_absent` and `an_explicit_empty_members_array_declares_a_root_only_workspace`; standard.rs `single_member_resolves_only_when_root_counts` now asserts a lone uv package is degenerate and two are not. `lockfile_provenance.rs` matrix is unaffected (no root-only uv case)
        - verification (from `sniff/`): `just test` 3084 passed, 32 skipped; `just lint` clean; `cargo clippy -p sniff -p sniff-cli --all-targets -- -D warnings` clean
- work completed for 'A root-only uv workspace has no layer to report its lockfile match' at 10:28:00
- cross-OS (orchestrator): `just cross-check sniff --os all` (2183 tests on the last leg) and `just cross-check sniff-cli --os all` (878 tests on the last leg) passed on linux, windows, and wsl; no receipt was published because the tested tree is uncommitted

### Successful Completion

The implementation of review cycle 3 has completed successfully in 29 minutes. During this implementation all 2 review findings were evaluated to see if they could be fixed as a part of this implementation cycle: 2 were fixed, 0 were deferred (see reasons below):

- no findings were deferred
- items for the reviewer's attention:
        - `sniff repo structure` now requests repository details, so every single-package root prints a `Single-package` summary with its standalone lockfiles; a degenerate Cargo workspace (`members = []`) changed from empty output to a single package, and its text snapshot was replaced
        - the `is_monorepo` rule in `standard.rs` now treats a lone uv package as degenerate, so a root-only uv layer does not make a repository a monorepo
        - a Cargo `[workspace] members = []` still forms no layer, while uv's explicit empty declaration now does

The files changed during this cycle are:

- library source: `sniff/lib/src/filesystem/repo/detection.rs`, `sniff/lib/src/filesystem/repo/types.rs`, `sniff/lib/src/filesystem/repo/uv.rs`, `sniff/lib/src/filesystem/repo/standard.rs`
- CLI source: `sniff/cli/src/commands/mod.rs`
- library tests: `sniff/lib/tests/l1/lockfile_fixtures.rs`, `sniff/lib/tests/l1/integration.rs`
- CLI tests: `sniff/cli/tests/l1/lockfile_cli.rs`, `sniff/cli/tests/l1/snapshots.rs` (snapshot `l1__snapshots__degenerate_cargo_structure_text.snap` deleted)
- docs: `sniff/lib/README.md`, `sniff/cli/README.md`, `sniff/docs/cli/repo_structure.md`, `.claude/skills/sniff/architecture.md`, `.claude/skills/sniff/cli.md`, `sniff/features/2026-09-26-lockfile-corroboration/accepted-versions.md`

## Implementation of Review Findings #4

> **started at:** 2026-09-27T10:49:55-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/fix-sniff/sniff/features/2026-09-26-lockfile-corroboration/review-4.md'
- this is iteration 4 of the review-to-implement cycle
- starting the work on 'Invalid workspace member entries can produce a false lockfile match' at 10:50:05
        - discovery: the uv, pnpm, `package.json` (array and `{packages: [...]}` forms), and Cargo `members`/`exclude` readers used `filter_map(as_str)`, so a non-string entry vanished and the shortened set could `match`; Rush's `project_folders` silently skipped a project without a string `projectFolder` the same way (R10: "a declared member has no resolved package"), so it is fixed too
        - discovery: R10's existing crate-private `DetectorOutcome::incomplete` flag already routes a layer to `unverifiable` + `incomplete_manifest_discovery` for every authority, Cargo included (`lockfile::cargo::compare` receives no owned seeds), and blocks provenance upgrades; no new mechanism was needed
        - decision: an invalid entry is recorded, not dropped: new `glob::DeclaredPatterns { patterns, has_invalid }` (built via `FromIterator<Option<&str>>`) is returned by every reader; detectors expand the valid patterns and OR `has_invalid` into `incomplete`
        - decision (all-invalid list): the layer is kept, reported `unverifiable` + `incomplete_manifest_discovery`, with no provenance upgrade; `DeclaredPatterns::is_empty` is false when any entry exists, so `members = [123]` no longer suppresses the layer (uv keeps its root-only layer; Cargo/pnpm/Node/Rush keep a layer with no packages); an absent or empty declaration still forms no layer as before
        - decision: a non-array `members`/`packages`/`workspaces` value (e.g. `members = "x"`) is left unchanged (reads as no declaration); flagged for reviewer
        - files changed: `lib/src/filesystem/repo/glob.rs`, `uv.rs`, `cargo.rs`, `npm.rs`, `detection.rs` (`collect_default_workspace_patterns` reads `.patterns`), `lib/tests/l1/lockfile_fixtures.rs`, `cli/tests/l1/lockfile_cli.rs`, `lib/README.md` (reason table example), `.claude/skills/sniff/architecture.md` (R10 flag sources)
        - tests added: L1 library `lockfile_fixtures::invalid_workspace_member_entries_are_incomplete_manifest_discovery` (12 cases on disposable copies of real-tool fixtures, editing only the manifest: uv, pnpm, npm array, npm object form, Cargo members, Cargo exclude, Rush; one-invalid and all-invalid lists), asserting status, reason, empty differences, and non-lockfile layer and package provenance; unit tests `uv::tests::a_non_string_member_is_recorded_as_invalid_rather_than_dropped`, `npm::tests::package_json_non_string_workspaces_are_recorded_as_invalid`, `npm::tests::pnpm_non_string_packages_are_recorded_as_invalid`; renamed `rush_json_skips_projects_without_a_string_folder` to `rush_json_records_projects_without_a_string_folder_as_invalid`; CLI L1 case `unverifiable-invalid-member` in the `lockfile_cli` JSON and human matrices
        - verification: `just test` 3088 passed, 32 skipped; `just lint` clean; `cargo clippy -p sniff -p sniff-cli --all-targets -- -D warnings` clean; `just check-tier-coverage sniff` 0 stranded
- work completed for 'Invalid workspace member entries can produce a false lockfile match' at 10:55:51
- orchestrator check: `just test invalid_` re-run independently, 29 passed (including the new 12-case library test); cross-OS checks were not run this cycle because the change only affects how manifest values are classified and adds no path or OS-specific code; CI and the earlier cycle's cross-check cover OS variance

### Successful Completion

The implementation of review cycle 4 has completed successfully in 7 minutes. During this implementation all 1 review findings were evaluated to see if they could be fixed as a part of this implementation cycle: 1 were fixed, 0 were deferred (see reasons below):

- no findings were deferred
- items for the reviewer's attention:
        - Rush had the same silent-drop bug (a project without a string `projectFolder`) and was fixed in the same way, although the review did not name it
        - an all-invalid member list now keeps a layer reported `unverifiable` + `incomplete_manifest_discovery` instead of producing no layer; an absent or empty declaration still produces no layer
        - a non-array `members`/`packages`/`workspaces` value (e.g. `members = "x"`) is unchanged and still reads as no declaration
        - Rush inconsistency: when every declared folder is missing on disk, the layer still disappears (existing behavior), but when every entry is invalid, the layer is kept

The files changed during this cycle are:

- library source: `sniff/lib/src/filesystem/repo/glob.rs`, `sniff/lib/src/filesystem/repo/uv.rs`, `sniff/lib/src/filesystem/repo/cargo.rs`, `sniff/lib/src/filesystem/repo/npm.rs`, `sniff/lib/src/filesystem/repo/detection.rs`
- library tests: `sniff/lib/tests/l1/lockfile_fixtures.rs`
- CLI tests: `sniff/cli/tests/l1/lockfile_cli.rs`
- docs: `sniff/lib/README.md`, `.claude/skills/sniff/architecture.md`

## Implementation of Review Findings #5

> **started at:** 2026-09-27T11:05:31-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/fix-sniff/sniff/features/2026-09-26-lockfile-corroboration/review-5.md'
- this is iteration 5 of the review-to-implement cycle
- starting the work on 'Wrong-type workspace member fields silently remove lockfile observations' at 11:05:40
        - discovery: the uv `members`, Cargo `members`/`exclude`, pnpm `packages`, and `package.json` `workspaces` readers mapped a present non-array value to an empty declaration, so the detector formed no layer and the lockfile vanished; Rush had the same gap one level up: `projects: Vec<...>` failed to deserialize for `"projects": 123`, `rush.json` read as unparseable (`Ok(None)`), and the Rush layer disappeared silently, so it is fixed too
        - decision: a present member field of the wrong type is an invalid declaration: new `glob::DeclaredPatterns::invalid()` (no patterns, `has_invalid = true`) is returned by every reader, so `is_empty` is false, the layer is kept, and the existing `DetectorOutcome::incomplete` route yields `unverifiable` + `incomplete_manifest_discovery` with no provenance upgrade; no detector code changed
        - decision: an absent field still declares nothing (no layer), as before; a null pnpm `packages` (pnpm itself ignores a falsy `packages`), a null `package.json` `workspaces`, a null `"packages"` in the object form, and a null Rush `projects` also read as absent, preserving prior behavior for null
        - decision: the `package.json` object form without a `packages` key (e.g. only `nohoist`) stays "no declaration"; only a present, non-null, non-array `packages` is invalid; npm/Yarn/Bun authority selection is unchanged because each detector only tests `is_empty`, which is now false for the invalid declaration
        - decision: Rush `projects` became `Option<Lenient<Vec<Lenient<RushProject>>>>`, matching the existing `Lenient` pattern for other `rush.json` fields, so a wrong-type `projects` no longer hides the manager/variants settings either
        - discovery: `detection.rs` `collect_default_workspace_patterns` reads only `.patterns`, so an invalid field contributes no default Nx/Turbo patterns, consistent with iteration 4; no change needed
        - not changed: a wrong-type parent table (e.g. `[workspace]` or `tool.uv.workspace` that is not a table) still reads as no declaration; out of scope for this finding
        - files changed: `lib/src/filesystem/repo/glob.rs` (`DeclaredPatterns::invalid`, doc note), `uv.rs`, `cargo.rs`, `npm.rs` (pnpm, `package.json`, and Rush readers plus reader docs), `lib/tests/l1/lockfile_fixtures.rs`, `cli/tests/l1/lockfile_cli.rs`, `lib/README.md` (reason table), `.claude/skills/sniff/architecture.md` (R10 flag sources)
        - tests added: 7 wrong-type cases in L1 library `lockfile_fixtures::invalid_workspace_member_entries_are_incomplete_manifest_discovery` (uv `members = "packages/*"`, pnpm `packages: 123`, npm `"workspaces": 123`, npm `{"packages": "packages/*"}`, Cargo `members = "crates/*"`, Cargo `exclude = "local-lib"`, Rush `"projects": 123`), confirmed failing when `invalid()` is neutered; unit tests `uv::tests::a_non_array_members_value_is_recorded_as_invalid_rather_than_absent`, new `cargo::tests` module (`an_absent_key_declares_nothing`, `a_non_array_value_is_recorded_as_invalid_rather_than_absent`), `npm::tests::package_json_non_array_workspaces_are_recorded_as_invalid`, `npm::tests::package_json_null_or_packageless_workspaces_declare_nothing`, `npm::tests::pnpm_non_sequence_packages_are_recorded_as_invalid`, `npm::tests::pnpm_absent_or_null_packages_declare_nothing`, `npm::tests::rush_json_non_array_projects_are_recorded_as_invalid`; CLI L1 case `unverifiable-wrong-type-members` (review 5's `packages: 123` reproduction) in the `lockfile_cli` JSON and human matrices
        - verification: `just test` 3096 passed, 32 skipped; `just lint` clean; `cargo clippy -p sniff -p sniff-cli --all-targets -- -D warnings` clean; `just check-tier-coverage sniff` 0 stranded
- work completed for 'Wrong-type workspace member fields silently remove lockfile observations' at 11:10:55
- orchestrator check: re-ran `just test invalid_` (31 passed, including the extended library case), `just test lockfile_cli` (5 passed, including the `unverifiable-wrong-type-members` JSON and human cases), `just test non_array` (5 passed), and `just test non_sequence` (1 passed); no cross-OS check was run because the change only reclassifies manifest value types and adds no path or OS-specific code

### Successful Completion

The implementation of review cycle 5 has completed successfully in 7 minutes. During this implementation all 1 review findings were evaluated to see if they could be fixed as a part of this implementation cycle: 1 were fixed, 0 were deferred (see reasons below):

- no findings were deferred
- items for the reviewer's attention:
        - Rush had the same gap one level up (`"projects": 123` made `rush.json` unparseable and the layer vanished); it is fixed with the existing `Lenient` pattern, although the review did not name it
        - a null member field (pnpm `packages: null`, `package.json` `workspaces: null`, object-form `"packages": null`, Rush `projects: null`) still reads as absent, which keeps the prior behavior
        - a wrong-type parent table (for example a `[workspace]` or `tool.uv.workspace` value that is not a table) still reads as no declaration and is out of scope
        - the CLI case's check that every package has globbed provenance may pass trivially, because `packages: 123` resolves no member packages; the library L1 cases assert the provenance directly

The files changed during this cycle are:

- library source: `sniff/lib/src/filesystem/repo/glob.rs`, `sniff/lib/src/filesystem/repo/uv.rs`, `sniff/lib/src/filesystem/repo/cargo.rs`, `sniff/lib/src/filesystem/repo/npm.rs`
- library tests: `sniff/lib/tests/l1/lockfile_fixtures.rs`
- CLI tests: `sniff/cli/tests/l1/lockfile_cli.rs`
- docs: `sniff/lib/README.md`, `.claude/skills/sniff/architecture.md`

## Implementation of Review Findings #6

> **started at:** 2026-09-27T11:25:40-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/fix-sniff/sniff/features/2026-09-26-lockfile-corroboration/review-6.md'
- this is iteration 6 of the review-to-implement cycle
- starting the work on 'An npm lockfile without workspace declarations reports a false mismatch' at 11:26:10
        - discovery: `lockfile/npm.rs` returned `Members([])` when the root record had no `workspaces` and `packages` held no non-root, non-`node_modules/` path, so the comparison invented an empty locked set and reported every current member as `missing`; a link-only lockfile without declarations took the same path, because the check ignored `link: true` targets
        - discovery: every npm 11.6.4 real-tool workspace fixture (v3, v2, shrinkwrap, and the edited variants) copies the manifest declarations into `packages[""].workspaces`, so an absent field records no member set rather than an empty one
        - decision: `unverifiable` + `no_membership_data` (the new `ParsedLockfile::NoMembershipData` branch), not `unreadable` + `parse_failed`, because the document is valid JSON at an accepted version and `workspaces` is not a required field; this matches uv's absent-`[manifest]` classification
        - decision: an undeclared lockfile that records a local package path or a `link: true` target stays `ambiguous_membership` (either could be a workspace or a `file:` dependency); only the link-target half is new; a present `workspaces: []` stays a recorded empty member set
        - files changed: `sniff/lib/src/filesystem/repo/lockfile/npm.rs` (branch and `parse` docs), `sniff/lib/tests/l1/lockfile_fixtures.rs`, `sniff/lib/README.md` (`no_membership_data` reason row and the npm comparison row), `sniff/features/2026-09-26-lockfile-corroboration/accepted-versions.md` (drifted "npm without locked declarations" note said "an empty member set otherwise")
        - tests added: unit tests `npm::tests::a_missing_root_declaration_without_local_records_has_no_membership_data`, `npm::tests::an_empty_root_declaration_records_an_empty_member_set`, and `npm::tests::undeclared_local_paths_and_link_targets_are_ambiguous` (renamed from `undeclared_local_paths_are_ambiguous_but_an_empty_lock_is_not`, whose installed-only assertion encoded the bug); L1 `lockfile_fixtures::an_npm_lockfile_without_workspace_declarations_has_no_membership_data` copies `npm-11.6.4/workspace`, rewrites `package-lock.json` to the root record without `workspaces`, and asserts status, reason, paths, empty `extra`/`missing`, one parse, and unchanged layer and package provenance
        - verified failing without the fix: the L1 test reported `Mismatch` with `missing: [".tools/hidden", "packages/alpha", "packages/beta"]` (the review's reproduction), and the two changed unit tests reported `Ok(Members([]))`
        - verification: `just test` 3099 passed, 32 skipped; `just lint` clean; `cargo clippy -p sniff -p sniff-cli --all-targets -- -D warnings` clean; `just check-tier-coverage sniff` 0 stranded
- work completed for 'An npm lockfile without workspace declarations reports a false mismatch' at 11:29:54
- starting the work on 'Shipped CLI fixture tests use a build-host path in archived runs' at 11:30:12
        - discovery: `lockfile_cli::lockfile_fixtures` rooted the library's fixtures at `Path::new(env!("CARGO_MANIFEST_DIR"))`, the producer's checkout; the repository archive-path guard (`cargo nextest run -p test-toolkit --test archive_path_guard`, full-tree mode) already flagged it before the fix: `sniff/cli/tests/l1/lockfile_cli.rs:19 — use biscuit_test_harness::manifest_dir!()`
        - discovery: this was the only `env!("CARGO_…")` path read in `sniff/cli/tests` (and `sniff/lib/tests` has none); `sniff-cli` already had `biscuit-test-harness` as a dev-dependency, so no dependency or dependency-doc change
        - decision: replace only the root with `biscuit_test_harness::manifest_dir!()`, keeping `.parent().expect(…).join("lib/tests/fixtures/lockfiles")` in the same expression, a form `scripts/ci/test_inputs.py` resolves
        - files changed: `sniff/cli/tests/l1/lockfile_cli.rs`
        - verification: archive-path guard passes after the fix (2 passed)
        - verification: `test_inputs.scan` over `cargo metadata` with candidate `sniff/lib/tests/fixtures/lockfiles/pnpm-10.32.1/workspace/pnpm-lock.yaml` selects `sniff-cli` at `lockfile_cli.rs:22` and `:26` with unit `binary_id(sniff-cli::l1)` (the read is in a helper, so the whole binary is scheduled)
        - verification: archived run: `cargo nextest archive -p sniff-cli`, then `cargo nextest run --archive-file … --workspace-remap /tmp/fixsniff-remap` against a copy of `Cargo.toml`, `Cargo.lock`, `.config`, and `sniff/` passed all 5 `lockfile_cli` tests; with the fixtures deleted from the remapped copy only, every test failed with `missing fixture /private/tmp/fixsniff-remap/sniff/lib/tests/fixtures/lockfiles/…` although the build-host checkout still held them, proving the run-time directory is used
        - verification: `just test` 3099 passed, 32 skipped; `just lint` clean; `cargo clippy -p sniff -p sniff-cli --all-targets -- -D warnings` clean; `just check-tier-coverage sniff` 0 stranded
- work completed for 'Shipped CLI fixture tests use a build-host path in archived runs' at 11:32:42
- orchestrator check: re-ran `just test no_membership_data` (4 passed, including the new L1 npm case); no `just cross-check` run, because finding 1 changes only JSON-field classification and finding 2 was proven with the archive-path guard and a remapped archived run, neither adding OS-specific code

### Successful Completion

The implementation of review cycle 6 has completed successfully in 8 minutes. During this implementation all 2 review findings were evaluated to see if they could be fixed as a part of this implementation cycle: 2 were fixed, 0 were deferred (see reasons below):

- no findings were deferred
- items for the reviewer's attention:
        - npm always writes `workspaces` into the root record of a workspace lockfile (every npm 11.6.4 fixture does), so a missing field now reads as `unverifiable` + `no_membership_data`; `workspaces: []` remains a legitimate empty member set
        - a missing `workspaces` with only link targets now reports `ambiguous_membership`, consistent with the existing local-path case; before, it also produced the false empty set
        - `accepted-versions.md` carried the drifted "an empty member set otherwise" note, which was corrected to match the code
        - the old `CARGO_MANIFEST_DIR` spelling was already selected by the test-inputs index, so the defect was only the build-time path, not CI scheduling

The files changed during this cycle are:

- library source: `sniff/lib/src/filesystem/repo/lockfile/npm.rs`
- library tests: `sniff/lib/tests/l1/lockfile_fixtures.rs`
- CLI tests: `sniff/cli/tests/l1/lockfile_cli.rs`
- docs: `sniff/lib/README.md`, `sniff/features/2026-09-26-lockfile-corroboration/accepted-versions.md`

## Implementation of Review Findings #7

> **started at:** 2026-09-27T12:09:19-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/fix-sniff/sniff/features/2026-09-26-lockfile-corroboration/review-7.md'
- this is iteration 7 of the review-to-implement cycle
- starting the work on 'An explicit null npm workspace declaration is treated as missing evidence' at 12:09:30
        - discovery: in `NpmLockDocument`, `packages: null` and `lockfileVersion: null` were already errors; the absent/`null` conflation affected only the root record's `workspaces` and `PackageRecord.link` (`"link": null` read as "not a link" while `"link": "yes"` was already rejected)
        - discovery (sibling parsers): yarn (`resolution`, `__metadata.version`), bun (`workspaces`), and pnpm (`importers`) already treat a missing value as a parse error, so `null` is already `unreadable`; `uv.lock` and `Cargo.lock` are TOML, which has no `null`; Rush's `useWorkspaces`/`subspacesEnabled` are layout configuration, not lockfile membership fields
        - decision: added a generic `present` deserializer to `npm.rs`, applied as `#[serde(default, deserialize_with = "present")]` to `RootRecord.workspaces` and `PackageRecord.link`; absent stays `None`, explicit `null` is a parse error (`unreadable` + `parse_failed`); `workspaces: []` stays a legitimate empty set
        - decision (not changed, flagged for the reviewer): `resolved: null` on a `link: true` record still drops that link target, because `resolved` is also an ordinary ignored field on non-link records
        - decision: the public-result test builds its inputs in a tempdir from the real `npm-11.6.4/workspace` fixture, following the review-6 `no_membership_data` test, instead of adding an edited fixture directory
                - case 1: the workspace fixture with `workspaces: null` (the reviewer's `ambiguous_membership` repro)
                - case 2: a root-only lockfile with `workspaces: null` (the `no_membership_data` repro)
        - files changed:
                - `sniff/lib/src/filesystem/repo/lockfile/npm.rs`: the fix, the `parse` `## Errors` rustdoc, unit test `an_explicit_null_declaration_is_malformed_not_missing`, and four new `rejects_malformed_documents` cases
                - `sniff/lib/tests/l1/lockfile_fixtures.rs`: `an_npm_lockfile_with_null_workspace_declarations_is_unreadable` (full observation, `REPO_LOCKFILE_PARSES == 1`, no provenance upgrade), in the declared `l1` binary
                - `sniff/features/2026-09-26-lockfile-corroboration/accepted-versions.md`, `sniff/lib/README.md`: npm notes now cover explicit `null`
                - `.claude/skills/sniff/architecture.md`: the npm lockfile parser is stricter than the manifest side, where `package.json` `workspaces: null` still declares nothing
        - red proof: both new tests failed with the attribute removed from `workspaces`; the fix was then restored
        - verification: `cargo nextest run -p sniff --features remote -E 'test(npm)'` 88 passed; `just test` 3101 passed, 32 skipped; `just lint` clean; `cargo clippy -p sniff -p sniff-cli --all-targets -- -D warnings` clean; `just check-tier-coverage sniff` 0 stranded
- work completed for 'An explicit null npm workspace declaration is treated as missing evidence' at 12:13:30
- orchestrator check: re-ran `cargo nextest run -p sniff --features remote -E 'test(null)'` (9 passed, including the new L1 public-result test); no `just cross-check` run, because the change only classifies JSON field values and adds no OS-specific code

### Successful Completion

The implementation of review cycle 7 has completed successfully in 5 minutes. During this implementation all 1 review findings were evaluated to see if they could be fixed as a part of this implementation cycle: 1 were fixed, 0 were deferred (see reasons below):

- no findings were deferred
- items for the reviewer's attention:
        - the same defect was also fixed for `PackageRecord.link: null`
        - `resolved: null` on a link record was deliberately left tolerant (see above)

The files changed during this cycle are:

- library source: `sniff/lib/src/filesystem/repo/lockfile/npm.rs`
- library tests: `sniff/lib/tests/l1/lockfile_fixtures.rs`
- docs: `sniff/lib/README.md`, `sniff/features/2026-09-26-lockfile-corroboration/accepted-versions.md`, `.claude/skills/sniff/architecture.md`

## Implementation of Review Findings #8

> **started at:** 2026-09-27T12:34:16-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/fix-sniff/sniff/features/2026-09-26-lockfile-corroboration/review-8.md'
- this is iteration 8 of the review-to-implement cycle 
- starting the work on 'A null npm link target can falsely corroborate a workspace' at 12:34:22
        - discovery: `PackageRecord.resolved` was a bare `Option<String>`, so `"resolved": null` read as absent; a mistyped non-null `resolved` (e.g. a number) was already a parse error on every record
        - discovery: `Packages.invalid` is returned as `Err` from `parse`, which the lockfile pipeline maps to `unreadable` + `parse_failed`; no provenance upgrade follows
        - decision: `resolved` is now tri-state `Option<Option<String>>` through the existing `present` deserializer (`None` absent, `Some(None)` explicit `null`, `Some(Some(_))` target); the `Packages` visitor records `invalid = "null link target"` only when the fully read record has `link: true` and `Some(None)`, so JSON key order does not matter
        - decision: `resolved: null` on a non-link record (no `link`, or `link: false`) stays tolerated as an ignored field; `link: true` with `resolved` omitted is unchanged (still dropped), because the finding is scoped to a present target
        - files changed:
                - `sniff/lib/src/filesystem/repo/lockfile/npm.rs`: the fix; the `parse` `## Errors` rustdoc, `present` doc, and `resolved` field doc; two `rejects_malformed_documents` cases (a link-only null target, and a null target masked by its `packages/web` record, with `link` before `resolved`); unit test `a_null_resolved_outside_a_link_record_is_ignored`
                - `sniff/lib/tests/l1/lockfile_fixtures.rs`: `an_npm_lockfile_with_a_null_link_target_is_unreadable` (tempdir copy of `npm-11.6.4/workspace` with `node_modules/@fixture/beta` `resolved: null`; full observation, `REPO_LOCKFILE_PARSES == 1`, no layer or package provenance upgrade), in the declared `l1` binary
                - `sniff/features/2026-09-26-lockfile-corroboration/accepted-versions.md`, `sniff/lib/README.md`, `.claude/skills/sniff/architecture.md`: npm explicit-`null` notes now cover a `link: true` record's `resolved`
        - red proof: with the `invalid` assignment disabled, `rejects_malformed_documents` (npm) and `an_npm_lockfile_with_a_null_link_target_is_unreadable` both failed; the non-link tolerance test passed either way, as a guard; the fix was then restored
        - verification: `just test null` 13 passed; `just test` 3103 passed, 32 skipped; `just lint` clean; `cargo clippy -p sniff -p sniff-cli --all-targets -- -D warnings` clean; `just check-tier-coverage sniff` 0 stranded
- work completed for 'A null npm link target can falsely corroborate a workspace' at 12:38:18
- orchestrator check: re-ran `cargo nextest run -p sniff --features remote -E 'test(null)'` (11 passed, including `an_npm_lockfile_with_a_null_link_target_is_unreadable`); no `just cross-check` run, because the change only classifies JSON field values and adds no OS-specific code

### Successful Completion

The implementation of review cycle 8 has completed successfully in 4 minutes. During this implementation all 1 review findings were evaluated to see if they could be fixed as a part of this implementation cycle: 1 were fixed, 0 were deferred (see reasons below):

- no findings were deferred
- items for the reviewer's attention:
        - a `link: true` record with `resolved` omitted entirely is still dropped rather than rejected; the finding covered only a present but invalid target

The files changed during this cycle are:

- library source: `sniff/lib/src/filesystem/repo/lockfile/npm.rs`
- library tests: `sniff/lib/tests/l1/lockfile_fixtures.rs`
- docs: `sniff/lib/README.md`, `sniff/features/2026-09-26-lockfile-corroboration/accepted-versions.md`, `.claude/skills/sniff/architecture.md`
