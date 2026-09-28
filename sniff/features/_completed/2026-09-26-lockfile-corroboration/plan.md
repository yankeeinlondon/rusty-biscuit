---
total_phases: 5
created: 2026-09-26
phase: 5
agent: "claude/opus"
yolo: "true"
feature: 2026-09-26-lockfile-corroboration
spec: sniff/features/2026-09-26-lockfile-corroboration/spec.md
related:
    - 2026-09-21-lockfile-provenance-cost
    - 2026-09-20-repo-perf
planned_packages:
    - sniff
    - sniff-cli
    - claudine
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

# Lockfile corroboration for every workspace standard: implementation plan

This plan implements `2026-09-26-lockfile-corroboration`. It replaces
`MonorepoLayer::lockfile_match: Option<bool>` with a required, structured
per-layer `lockfile` observation. It extends corroboration from Cargo, pnpm,
and uv to every standard whose lockfile records workspace membership, and gives
every other standard a truthful fallback status.

## Work Summary and Success Definition

### Where the work lands today

- **Corroboration logic:** `sniff/lib/src/filesystem/repo/detection.rs`.
  `upgrade_provenance_with_lockfile` (around line 888) dispatches to
  `pnpm_lockfile_matches`, `uv_lockfile_matches`, and `cargo_lockfile_matches`.
  Each returns `Option<bool>`, and `None` hides several different reasons. The
  gate is `request.wants_lockfile_provenance()` (around line 747).
- **Request-local cache:** `ManifestStore` in `detection.rs` (around line 196).
  The `pnpm_locks` and `uv_locks` fields hold generic `Value` trees, and
  `cargo_locks` holds `CargoLockVersions` (`manifest_index.rs`). All three cache
  `None` on failure, so the cause of a failure is lost.
- **Public type:** `MonorepoLayer` in `sniff/lib/src/filesystem/repo/standard.rs`
  (around line 370). `lockfile_match` is skipped in JSON when `None`.
  `PackageProvenance::Lockfile` is documented as coming from a "committed"
  lockfile, which is inaccurate: the implementation reads the working tree.
- **Constructors and consumers of the old field:** `topology.rs`, `types.rs`,
  `standard.rs` (tests), `aggregate_view.rs`, `sniff/cli/src/output/repo_json.rs`
  (4 fixtures), `sniff/cli/tests/l1/cli.rs`, `sniff/lib/tests/l1/{integration,
  lockfile_provenance}.rs`, and `claudine/lib/src/events/environment.rs`, the
  only consumer outside Sniff.
- **Human rendering:** `sniff/cli/src/output/filesystem/repo.rs`.
  `format_monorepo_layer` and the multi-layer list currently show no lockfile
  state.
- **Parsers available:** `toml`, `serde_yaml_ng`, and `serde_json` are already
  available. `json-five` is already in the workspace graph through
  `biscuit-file` and is a candidate for JSONC. There is no Bun, npm, Yarn, or
  Rush parser.

### Summary of required work

1. **Observation type (breaking).** Add a public `LockfileObservation` with
   `status`, `paths`, `reason`, `extra`, and `missing`, plus closed `status` and
   `reason` enums. It replaces `lockfile_match` and is always serialized.
2. **Selection engine.** For each authority, declare the candidate files and
   their precedence. Probe metadata through a store cache that distinguishes
   present, absent, and error. Then apply the spec's precedence rules:
   `not_applicable`, then `unreadable` for metadata errors, then `absent`, then
   `not_requested`, then fallback `unverifiable`, then parse. Probes run even
   when corroboration is disabled, but nothing reads file contents.
3. **Typed parsers.** Each parser keeps only what membership and version
   recognition need, validates the whole document, and returns a typed
   `Parsed | Unsupported | Failed` outcome that is cached per request. Formats:
   pnpm (from the patch, fixed), uv (the real `[manifest]` layout), npm v2/v3,
   Yarn Berry, Bun text (JSONC), Rush on the ordinary pnpm-workspace layout, and
   Cargo (keeping `source` for the Cargo ruling).
4. **Membership comparison.** Compare paths relative to the layer root, with
   the root excluded. Normalize paths by component, reject absolute or
   unrepresentable paths, and never intersect the two sets before comparing.
   Only an exact `match` upgrades provenance, and never across layers owned by
   another authority.
5. **Fallback sources.** Go (`go.work.sum`), Gradle (root `gradle.lockfile` or
   legacy `gradle/dependency-locks/*.lockfile`), and Bazel (`MODULE.bazel.lock`)
   report `unverifiable` from metadata alone. Maven, .NET, Pants, and Buck2
   report `not_applicable`, and `Unknown` reports `unknown_standard`.
6. **Repository-level fallback.** Add repository-level observations for
   standalone Poetry, PDM, and Composer lockfiles (Ruling R2).
7. **Request cost.** Add the lockfile probe/read/parse counters described in
   Ruling R8. A structure request with corroboration disabled reads and parses
   0 lockfiles, and no new descendant walk is added.
8. **CLI.** The JSON projections carry the object everywhere layers appear.
   Human output shows each layer's status, paths, and extra/missing members,
   with plain-language explanations, through `biscuit-terminal` components.
9. **Evidence and docs.** Add an accepted-version matrix with real-tool
   fixtures, parser measurements, READMEs, public docs, the Sniff skill, and
   dependency docs. Verify on macOS, Linux, native Windows, and WSL2.

### Success looks like

Each item maps to one of the spec's acceptance criteria (AC).

1. `MonorepoLayer` has no `lockfile_match`. Every serialized layer carries a
   `lockfile` object with all five fields present. `rg lockfile_match` finds no
   hits outside `_completed` specs and the documented breaking-change note.
2. An L1 fixture matrix in `sniff/lib/tests/l1/lockfile_provenance.rs` (or a
   sibling module) asserts complete serialized `RepoInfo` for every authority
   and every applicable status, including `Unknown`. It also covers
   orchestrator-only repositories, where no synthetic layer may appear (AC1).
3. Every accepted format/version row in `accepted-versions.md` has at least one
   checked-in fixture written by the recorded real tool version. It also has
   tests for stale extra members, missing members, unrelated local dependencies,
   malformed trailing content, unsupported versions, root-only or virtual
   roots, scoped names, duplicate keys, and missing required fields. Every
   unknown version is proven unable to produce `mismatch` (AC2).
4. Tests for nested and overlapping layers, shared lockfiles, repeat
   cheaper-request calls, filename precedence, metadata and read failures,
   symlink spellings, `.hidden` paths, and Windows-native path conversion pass.
   The failure tests are deterministic on every OS (AC3).
5. Work counters show that a disabled structure request performs 0 lockfile
   reads and 0 parses. Enabled requests read and parse each selected file once,
   including cached failures. A full request with corroboration disabled but
   dependency enrichment on still reads `Cargo.lock` and never upgrades
   provenance (AC4).
6. The shipped CLI, run on disposable repositories, emits exactly one valid
   JSON document on stdout, with hints on stderr only. The exit status does not
   change for `mismatch` or `unreadable`. Human output names each layer, its
   status, paths, and extra/missing members (AC5).
7. `measurements.md` records release-mode parse time and allocation behavior
   for large JSON, JSONC, YAML, and TOML fixtures under the `2026-09-20-repo-perf`
   protocol. The following all pass:
   - `just test` and `just lint` in `sniff/`;
   - `cargo clippy -p sniff -p sniff-cli --all-targets -- -D warnings`;
   - the affected Claudine and Darkmatter tests;
   - dependency-derived compile checks.

   macOS, Linux, native Windows, and WSL2 evidence exists under repository
   policy (AC6).
8. The READMEs, `sniff/docs`, `.claude/skills/sniff/`, and dependency docs
   describe the new object, the removal of `lockfile_match`, root exclusion, and
   the Cargo semantics.

## Phase 1: Rulings, spikes, and accepted-version matrix

This phase settles every open question and produces the pinned evidence that
later phases test against. No production code changes.

### Necessary Rules

The spec's **Open questions** must be settled before implementation. Because
this plan runs with `yolo: true`, each ruling below records the spec's
recommended option as the **adopted default**. The owner may override any
ruling before Phase 2 begins. An override that changes the wire contract (R1,
R2, R5) must update this plan and the spec before Phase 2 starts.

- **R1: Cargo reports partial evidence under separate statuses.** Adopt the
  spec's recommended option and give it a concrete wire shape:
  - Add two statuses: `members_present` and `members_missing`. `match` and
    `mismatch` keep their exact set-equality meaning on every ecosystem.
  - A member counts as present when `Cargo.lock` has a `[[package]]` entry with
    the member's name, the resolved manifest version (including
    `version.workspace = true` inheritance), and **no** `source`.
  - `members_missing` lists the absent members, as layer-relative paths, in
    `missing`. `extra` is always `[]` for Cargo because stale extra members
    cannot be recovered. `reason` is `subset_only` on both statuses so a
    consumer cannot read either one as equality.
  - Neither status upgrades provenance. Only `match` upgrades to `Lockfile`.
  - Parsing `Cargo.lock` now retains `source`. The dependency-version lookup
    (`CargoLockVersions::resolve`) must keep its current results byte for byte,
    which a parity test proves.
  - A `Cargo.lock` without a numeric `version` key is v1/v2. Classify it from
    its format signature. The accepted versions come from spike S1.
- **R2: Standalone Python/PHP lockfiles become repository-level observations.**
  - **Wire location:** `RepoInfo.standalone_lockfiles: Vec<StandaloneLockfileObservation>`.
    It is always serialized in the library, `[]` when empty. The CLI JSON
    projection follows the existing rule for omitting empty arrays, the same
    rule it applies to `monorepo_layers`.
  - **Entry fields:** `root` (repo-relative, `/`-separated, `""` for the
    repository root), `tool` (`poetry` | `pdm` | `composer`), plus the same
    `status`/`paths`/`reason`/`extra`/`missing` object as layers. `paths` is
    relative to `root`.
  - **Roots probed:** the requested project root and every already-discovered
    package root. Reuse manifest-index evidence where it exists, add no
    descendant walk, and add no `vendor/` or `.venv` search.
  - **Candidates:** `poetry.lock`, `pdm.lock`, `composer.lock`.
  - **Absence semantics:** an absent file produces **no entry**. Absence at an
    arbitrary package root is not an observation worth reporting.
  - **Present file:** `not_requested` when corroboration is disabled, otherwise
    `unverifiable` with `no_membership_data`. A metadata error produces
    `unreadable`.
  - **Deduplication:** entries are unique by `(root, tool)`. A root that is
    also a workspace layer root still gets a standalone entry, because no layer
    authority covers these tools.
  - Go standalone modules (`go.sum`) are out of scope. Decision 2 covers Go
    only through `GoWorkspace` layers.
- **R3: Rush compares only the ordinary single pnpm workspace layout.**
  - The supported layout requires all of the following:
    - `rush.json` declares `pnpmVersion`;
    - subspaces are not enabled (`common/config/rush/subspaces.json` is absent
      or has `subspacesEnabled: false`);
    - the pnpm config does not disable workspaces (`useWorkspaces`, confirmed
      by spike S1);
    - no variant is involved.
  - In that layout, the candidate is `common/config/rush/pnpm-lock.yaml`.
    Importer keys resolve against `common/temp`.
  - **Every other layout** reports `unverifiable` with `unsupported_layout`
    and any confidently identified paths. That covers `npmVersion` and
    `yarnVersion` managers, enabled subspaces, the legacy non-workspace install,
    and any variant directory under `common/config/rush/variants/`.
  - The Rush configuration files are manifests, not lockfiles. They are read
    only when corroboration is enabled, and each read is counted as a manifest
    parse.
- **R4: Package-manager detection stays separate.** Record the uv-reported-as-pip
  gap as a new `_unscheduled` fix, `sniff/fixes/_unscheduled/package-manager-uv-label/spec.md`
  (task 1.4). No change to `detect_package_managers` in this feature.
- **R5: Status and reason vocabulary (frozen wire contract).**
  - **`status`** (snake_case): `match`, `mismatch`, `members_present`,
    `members_missing`, `unverifiable`, `unreadable`, `absent`,
    `not_applicable`, `not_requested`.
  - **`reason`** (snake_case):

    | Reason | Used with |
    |---|---|
    | `request_disabled` | `not_requested` |
    | `no_lockfile_source` | `not_applicable` |
    | `unknown_standard` | `not_applicable` |
    | `unsupported_version` | `unverifiable` |
    | `unsupported_layout` | `unverifiable` |
    | `no_membership_data` | `unverifiable`, including fallback formats and binary `bun.lockb` |
    | `ambiguous_membership` | `unverifiable`, when the name-to-path mapping is ambiguous |
    | `incomplete_manifest_discovery` | `unverifiable` |
    | `invalid_member_path` | `unverifiable`, for absolute or unrepresentable paths |
    | `metadata_failed` | `unreadable` |
    | `read_failed` | `unreadable`, including a directory where a file is expected and a file that vanished between probe and read |
    | `parse_failed` | `unreadable`, for invalid syntax or invalid required membership fields |
    | `subset_only` | Cargo `members_present` and `members_missing` |

  - `reason` is `null` for `match`, `mismatch`, and `absent`.
  - No human-readable diagnostic field is added to the wire object. Failure
    detail (the candidate path and the error kind, never file contents) goes to
    `tracing::debug!` only. The spec makes the diagnostic optional, and leaving
    it out keeps the contract small.
- **R6: `paths` contains only the selected source.**
  - Precedence: `npm-shrinkwrap.json` beats `package-lock.json`, and `bun.lock`
    beats `bun.lockb`. Only the winning file appears in `paths`.
  - Legacy Gradle is the one multi-file group. It lists each concrete
    `*.lockfile` directly under the root `gradle/dependency-locks/`.
  - If the selected file fails or has an unsupported version, no
    lower-priority file is tried.
- **R7: Authority selection is not revisited.** Corroboration uses
  `layer.authority` as already selected. Nx, Turborepo, and Lerna never
  produce a layer of their own.
- **R8: Counter vocabulary.**
  - Add `filesystem.repo.lockfile_probes` for lockfile metadata probes.
  - Keep the existing `REPO_LOCKFILE_READS` (content opens attempted) and
    `REPO_LOCKFILE_PARSES` (parser invocations after a successful read), with
    unchanged meanings.
  - `FS_FILE_OPENS`, `FS_BYTES_READ`, and `FS_METADATA_PROBES` stay at their
    shared increment points.
  - Rush configuration reads count as manifest parses.
- **R9: Deterministic failure injection.**
  - Read failure: a directory in place of the lockfile, which fails on every
    OS.
  - Metadata failure: a `#[cfg(test)]` probe seam in the store, since Unix
    mode bits are not portable.
  - Unix permission tests may be added as extras, but none may be the only
    proof of a behavior.
- **R10: Signal for incomplete manifest discovery.**
  - A layer is incomplete when any of the following holds:
    - a `layer.packages` entry has no resolved seed;
    - a member manifest failed to parse;
    - the glob expander reported its bound.
  - If spike S4 finds no existing signal for one of these, add a
    `pub(crate)` flag on the layer outcome. Do not infer completeness.
- **R11: Breaking-change commit convention.** Library and CLI commits that
  remove `lockfile_match` use the `!` conventional-commit marker, for example
  `feat(sniff)!:`, so release-plz bumps the version correctly.

### Spikes

- **S1: Real-tool fixture generation.** This is the biggest risk. Parsers
  cannot be written without real layouts.
- **S2: JSONC parser choice.** Candidates are `json-five` (already in the
  graph) and `jsonc-parser` with its serde feature. Criteria:
  - accepts comments and trailing commas;
  - supports typed serde deserialization with `IgnoredAny`, so no whole-tree
    allocation;
  - rejects trailing garbage;
  - behaves predictably on duplicate keys;
  - release-mode throughput on a large generated `bun.lock`.

  Pick one and record the result in `accepted-versions.md`, or in
  `measurements.md` for the timings.
- **S3: Typed parser feasibility.**
  - Confirm that a real Berry `yarn.lock` (with its `# THIS IS AN
    AUTOGENERATED FILE` header and `__metadata`) parses as YAML through
    `serde_yaml_ng`.
  - Confirm that typed `serde_json` with `IgnoredAny` handles a large
    `package-lock.json` in one pass.
  - Confirm that the `pnpm-typed-parser.patch` approach extends to reject
    non-string importer keys and to capture `lockfileVersion`.
- **S4: Probe-evidence and completeness audit.** List every existing lockfile
  `probe_exists` call site: `npm.rs:158-420`, `detection.rs:1439-1440`, and
  others found by search. For each site, record whether it can reach
  `ManifestStore`. Then decide how the new tri-state probe reuses that evidence
  without double counting. Also confirm which signals from R10 already exist.

### Tasks

**Wave 1** (all concurrent)

- [x] **Record rulings**
  - Copy R1–R11 into the spec as an `## Owner rulings` section, marked
    "adopted default — override before Phase 2". The spec status stays
    `draft-spec` until the owner confirms.
  - Add the `members_present`/`members_missing` statuses and the repo-level
    `standalone_lockfiles` shape to the spec's contract tables.
- [x] **Spike S1: fixtures**
  - Using installed or ephemeral tools (`npx`, `uvx`, `bunx`; none installed
    globally), generate minimal workspaces with 2–3 members, one scoped name,
    one `.hidden` member, and one unrelated local (`file:`/`link:`/path)
    dependency. Formats:
    - pnpm 10 (lockfile 9.0) and pnpm 9;
    - npm 11 (v3) and npm with `--lockfile-version 2`, plus a hand-kept npm
      v1 sample from npm 6 via `npx npm@6`;
    - Yarn Berry 4 and Yarn Classic 1.22;
    - Bun 1.3 (`bun.lock`) and `bun.lockb` from `--save-binary` or an older Bun;
    - uv 0.9 workspace, single-project, and virtual-root cases;
    - Cargo lock v3/v4 with a local non-member path dependency, a registry
      crate sharing a member's name, and an inherited workspace version;
    - Rush 5 on pnpm workspaces, via `npx @microsoft/rush`;
    - `go.work.sum` from a `go work` example;
    - Gradle `gradle.lockfile` and a legacy lock directory, generated or
      transcribed from the Gradle docs with a noted provenance;
    - `MODULE.bazel.lock`;
    - `poetry.lock` and `pdm.lock` via `uvx`, and `composer.lock` via
      Composer 2.10.
  - Check fixtures in under `sniff/lib/tests/fixtures/lockfiles/<tool>-<version>/<case>/`.
    Each directory has a `PROVENANCE.md` giving the exact tool version,
    command, host OS, and date.
  - Trim fixtures to minimal size but never hand-edit their membership data.
    Place hand-edited negative variants (stale extra, missing, malformed
    trailing content, unknown version) beside them with `-edited` suffixes.
  - Follow the `rust-testing` skill's spelling for repository reads so CI's
    test-input narrowing (`docs/cicd/test-inputs.md`) sees these fixtures.
- [x] **Spike S2: JSONC crate**
  - Benchmark and assess the candidates against the S2 criteria. Record the
    choice and whether a new dependency is added (and so whether the
    dependency docs change in Phase 5).
- [x] **Spike S3: parser feasibility**
  - Write throwaway `#[cfg(test)]` probes, not merged, against the S1
    fixtures. Record per-format field paths:
    - npm: `lockfileVersion`, `packages[""].workspaces`, and records with
      `link: true`;
    - Yarn: `__metadata.version`, and entries whose `resolution` is
      `<name>@workspace:<path>`;
    - Bun: `lockfileVersion` and `workspaces` keys;
    - uv: `version`, `revision`, `[manifest].members`, and `package[].source.
      {editable,virtual}`;
    - Rush: the importer base.
- [x] **Spike S4: evidence audit**
  - Produce the call-site table and the completeness-signal findings from the
    S4 description.
- [x] **Unscheduled fix**
  - Create `sniff/fixes/_unscheduled/package-manager-uv-label/spec.md`
    describing the uv-labeled-as-pip gap (R4), with
    `related: [2026-09-26-lockfile-corroboration]`.

**Wave 2** (after Wave 1)

- [x] **Accepted-version matrix**
  - Write `sniff/features/2026-09-26-lockfile-corroboration/accepted-versions.md`.
    Each row gives the format, the accepted version values, the producer tool
    and version, the fixture path, the membership field path, the behavior of
    known-unsupported versions (`unsupported_version`), and the format signature
    for historical versions without a numeric version.
  - Rows cover pnpm, npm, Yarn, Bun, uv, Cargo, and Rush, plus the fallback
    formats with the metadata-only note.
- [x] **Validation checkpoint 1**
  - Every R-ruling is either reflected in the spec or overridden by the owner.
  - Every matrix row has a real fixture.
  - Spike findings are recorded under the feature directory.
  - `git status` shows no production code change.

## Phase 2: Observation type, selection engine, and migration of existing formats

This phase lands the breaking API change and a framework that every later
format plugs into. It keeps workspace compile green, including Claudine, and
ports pnpm, uv, and Cargo to the new contract.

**Wave 1** (sequential, one agent; it touches the shared core)

- [x] **Observation types**
  - In a new `sniff/lib/src/filesystem/repo/lockfile/mod.rs`, add
    `LockfileObservation`, `LockfileStatus`, and `LockfileReason`, using the
    serde `rename_all = "snake_case"` values from R5.
  - Re-export them from `filesystem::repo`.
  - Replace `MonorepoLayer::lockfile_match` with a required
    `pub lockfile: LockfileObservation` that is always serialized, with no
    `#[serde(default)]`, so old result JSON fails to deserialize by design.
  - Add a `LockfileObservation::not_applicable(reason)` constructor for the
    layer-building sites.
- [x] **Fix every constructor and consumer**
  - Update `topology.rs`, `types.rs`, the `standard.rs` tests,
    `aggregate_view.rs` (its comment references `lockfile_match`),
    `sniff/cli/src/output/repo_json.rs` (4 fixtures), and
    `claudine/lib/src/events/environment.rs`.
  - Update the `standard.rs` docs so `PackageProvenance::Lockfile` and
    `MonorepoLayer::provenance` no longer say "committed", and describe the
    exact-`match` rule.
- [x] **Candidate table**
  - In `lockfile/sources.rs`, map `MonorepoStandard` to either `Source::Candidates`
    (an ordered list of `(relative path, Format)`), `Source::Fallback`
    (metadata only), `Source::NotApplicable(reason)`, or `Source::Configured`
    (Rush: the path comes from configuration).
  - This table is the single source of truth. Every one of the 17 standards
    plus `Unknown` must be matched exhaustively, with no wildcard arm.
- [x] **Tri-state probe and typed store cache**
  - Extend `ManifestStore` with `lockfile_presence(path) -> Present | Absent |
    Failed(kind)`, cached. It uses `symlink_metadata` and then follows links as
    today, and treats a directory as present, so the read fails later with
    `read_failed`.
  - Increment `REPO_LOCKFILE_PROBES` and `FS_METADATA_PROBES` on each miss.
  - Replace `pnpm_locks`, `uv_locks`, and `cargo_locks` with a typed outcome
    cache: `Result<Rc<Parsed<Format>>, LockfileFailure>`, where `Parsed`
    carries either `Membership` or `Unsupported { version }`. Failures are
    cached too.
  - Wire the existing lockfile probe sites that S4 found reachable to share
    this cache.
  - Add the R9 `#[cfg(test)]` metadata-failure seam.
- [x] **Selection state machine**
  - `observe_layer_lockfile(layer, request, store, seeds) -> LockfileObservation`
    implements the precedence rules in steps 1–5 of the spec, exactly in that
    order.
  - It is called for every layer regardless of the request, so presence is
    always probed. It reads contents only when
    `request.wants_lockfile_provenance()`.
  - Replace the gated loop at `detection.rs:747` with an unconditional loop.
- [x] **Path normalization and comparison**
  - `lockfile/membership.rs` does component-wise normalization:
    - `.` and trailing separators are removed, leading dots in names are kept,
      `..` resolves against a per-format base, and case is preserved;
    - absolute or non-UTF-8 paths produce `invalid_member_path`;
    - output uses `/` via the OS skill guidance;
    - the root is excluded from both sets;
    - duplicates are deduplicated, while conflicting identities or duplicate
      required keys produce `parse_failed`;
    - no intersection before comparison;
    - sorted `extra` and `missing` sets are computed.
  - An incomplete manifest-side set (R10) short-circuits to `unverifiable`.
- [x] **Provenance upgrade**
  - On `match` only, set `layer.provenance` and the provenance of seeds this
    layer owns (by ownership, not by bare `relative` match) to `Lockfile`.
  - Nested or overlapping layers never overwrite another authority's seeds.
  - Delete `upgrade_provenance_with_lockfile`, `pnpm_lockfile_matches`,
    `uv_lockfile_matches`, and `cargo_lockfile_matches`. Also delete
    `normalize_layer_package_relative` and `layer_relative_path` if nothing
    else uses them.
- [x] **Stub every format**
  - Create `lockfile/{pnpm,npm,yarn,bun,uv,cargo,rush,fallback}.rs`, each
    exposing `parse(content) -> Outcome`. Formats not yet implemented return
    `Unsupported` so the engine yields `unverifiable`.
  - This lets the Phase 3 tasks edit only their own file.

**Wave 2** (concurrent; each task owns one format file and its tests)

- [x] **pnpm parser**
  - Apply `pnpm-typed-parser.patch` into `lockfile/pnpm.rs` and fix three
    things:
    - capture `lockfileVersion` and classify it against the matrix;
    - reject non-string or duplicate importer keys with `parse_failed`
      instead of filtering them;
    - return a typed outcome instead of `Option`.
  - Keep the ignored-section duplicate tolerance, with a test documenting that
    it differs from the generic reference.
  - _Phase 1 finding:_ `serde_yaml_ng` coerces the plain scalars `1`, `~`, and
    `true` to strings when a key is deserialized as `String`. Deserialize
    importer keys as `serde_yaml_ng::Value` and reject every non-`String`
    variant. Accept `lockfileVersion` as a string or a number
    (`spike-s3-parsers.md`).
  - Add parity against a `#[cfg(test)]` generic `serde_yaml_ng::Value`
    reference on the S1 fixtures.
- [x] **uv parser**
  - Implement the layout confirmed by S3 in `lockfile/uv.rs`:
    - check `version` and `revision`;
    - map `[manifest].members` names to paths through `package[].source.
      editable|virtual`;
    - handle single-member and root cases explicitly, with the root excluded;
    - report an unmapped or ambiguous name as `ambiguous_membership`;
    - never count a non-member local dependency.
  - Delete the synthetic `[workspace].members` expectation.
  - _Phase 1 finding:_ uv omits `[manifest]` when the root is the only
    member, so an absent `[manifest]` is the root-only set (empty after root
    exclusion), not `parse_failed`. Accept `version = 1`, `revision = 3`.
- [x] **Cargo partial evidence**
  - In `manifest_index.rs`/`lockfile/cargo.rs`, make the typed `Cargo.lock`
    parse keep `name`, `version`, and whether `source` is present. Keep one
    parse per request shared with `CargoLockVersions`, as the spec requires.
  - Implement R1's `members_present`/`members_missing` using resolved member
    name and version.
  - Add a parity test proving `CargoLockVersions::resolve` output is
    unchanged. Add tests for the registry-same-name false positive and the
    local-nonmember case.
  - _Phase 1 finding:_ a lock with no `[[package]]` array is `parse_failed`
    for corroboration only; `CargoLockVersions` keeps its empty-index result.
    v2 is accepted by signature (no `version` key and no `[metadata]` table);
    v1 is `unsupported_version`. See `accepted-versions.md`.
- [x] **Validation checkpoint 2**
  - `cargo check --workspace --all-targets` compiles, including Claudine.
  - `just test` in `sniff/` passes after updating `lockfile_provenance.rs`,
    `integration.rs`, and the `detection.rs` unit tests. Cargo's old
    extra-entry expectation becomes `members_present`, and uv's
    root-inclusion expectation becomes root-excluded. Each change is called
    out in the commit message.
  - `cargo clippy -p sniff -p sniff-cli --all-targets -- -D warnings` passes.
  - A declining structure request on the three-lockfile fixture shows 0 reads,
    0 parses, and N probes, with `not_requested` on each layer.

## Phase 3: New formats, fallbacks, and standalone observations

Every task here edits its own `lockfile/*.rs` file or a new module. None of
them edits the Phase 2 engine except to register one table entry.

**Wave 1** (all concurrent)

- [x] **npm parser** (`lockfile/npm.rs`)
  - Select `npm-shrinkwrap.json` before `package-lock.json`, typed `serde_json`
    with `IgnoredAny`.
  - Build the locked workspace set from the root `packages[""].workspaces`
    declarations together with `link: true` records that resolve to workspace
    paths. Deleted members come from the locked declarations. Local `file:`
    links that are not declared workspaces are excluded.
  - v1 (no `packages`) → `unsupported_version`.
  - A version that cannot tell a workspace from a local link →
    `ambiguous_membership`.
  - Never inspect `node_modules` on disk.
  - _Phase 1 finding:_ a `file:` dependency is recorded exactly like a member
    (`packages["local-lib"]` plus a `link: true` record). Filter `packages`
    keys by the locked `packages[""].workspaces` globs, using a pure string
    glob matcher; `glob.rs` walks the filesystem and cannot be reused as is.
    Duplicate `packages` keys need a seen-set visitor (`serde_json` is
    last-wins).
- [x] **Yarn parser** (`lockfile/yarn.rs`)
  - Detect Berry by `__metadata.version`. Classic, identified by its
    `# yarn lockfile v1` signature, → `unsupported_version`.
  - Take workspace paths from `resolution: "<name>@workspace:<path>"`, parsing
    scoped names by splitting at the last `@workspace:` marker. Do not split at
    the first `@`.
  - Exclude `link:` and `portal:` entries, and handle combined descriptors
    such as `workspace:^`.
  - _Phase 1 finding:_ check the Classic signature before the YAML parse
    (Classic is not YAML and would otherwise be `parse_failed`). Accept
    `__metadata.version` 6, 8, and 10. No `@workspace:.` root resolution →
    `parse_failed`.
- [x] **Bun parser** (`lockfile/bun.rs`)
  - Parse `bun.lock` with the JSONC crate chosen in S2 using a typed partial
    struct that reads `lockfileVersion` and the `workspaces` keys.
  - The `""` key is the root and is excluded.
  - `bun.lockb` → `unverifiable` + `no_membership_data` from metadata alone,
    with no read. Never invoke Bun.
- [x] **Rush layout** (`lockfile/rush.rs`)
  - **Prerequisite (pre-existing bug found in Phase 1):** real `rush.json`
    files contain comments, and `parse_rush_project_folders` (`npm.rs`) uses
    strict `serde_json`, so Sniff detects no Rush layer for any real Rush repo
    today (verified against `rush-5.179.0/pnpm-workspace`). Parse `rush.json`
    with the S2 JSONC parser first, with a regression test on the real
    fixture; otherwise Rush corroboration can never run.
  - Read `rush.json`, the subspace config, and the pnpm config to classify the
    layout under R3.
  - For the supported layout, reuse `lockfile/pnpm.rs` for syntax and
    translate importer keys from the `common/temp` base (`../../apps/x` →
    `apps/x`).
  - Every other layout → `unsupported_layout` with the known paths.
  - Test against the S1 Rush fixture.
- [x] **Fallback sources** (`lockfile/fallback.rs`)
  - Go `go.work.sum`, Gradle, and Bazel `MODULE.bazel.lock` (only when
    `MODULE.bazel` exists). Each is metadata-only `unverifiable` +
    `no_membership_data`.
  - For Gradle, enumerate only the root `gradle.lockfile` or the direct
    `*.lockfile` children of `gradle/dependency-locks/`, never recursively.
  - Add explicit `not_applicable` table tests for Maven, .NET, Pants, Buck2,
    and `Unknown`.
- [x] **Standalone observations** (`lockfile/standalone.rs`)
  - Add `RepoInfo.standalone_lockfiles` and `StandaloneLockfileObservation`
    per R2.
  - Probe the root and discovered package roots through the Phase 2 store,
    with no descendant walk. Report present files only.
  - Update the `RepoInfo` constructors and consumers this touches.

**Wave 2**

- [x] **Validation checkpoint 3**
  - Every accepted-version row has a passing real-fixture test.
  - Every edited negative variant produces its specified status.
  - Every unknown-version fixture yields `unverifiable`, never `mismatch`.
  - `just test` and all-target clippy pass for `sniff`.

## Phase 4: CLI projection and CLI tests

This phase may start once Phase 2 is complete, concurrently with Phase 3 Wave 1:
the wire types are frozen by then. Its final checkpoint waits for Phase 3.

**Wave 1** (concurrent)

- [x] **JSON projection**
  - `sniff/cli/src/output/repo_json.rs`: make sure every place layers are
    serialized carries `lockfile`, including consolidated repository JSON and
    the aggregate views. Project `standalone_lockfiles` with the existing
    rule for omitting empty arrays.
  - Nothing in the CLI rediscovers or reinterprets lockfiles.
- [x] **Human output**
  - In `sniff/cli/src/output/filesystem/repo.rs`, render a lockfile line per
    layer: status, selected paths, and `extra`/`missing` lists.
  - Give fallback, `not_requested`, `not_applicable`, and `members_*` statuses
    a plain-language explanation derived from `reason`. For example,
    `not_requested` says to rerun with the full request, and the line tells
    the user which flag to use.
  - Add a standalone-lockfiles section.
  - Use `biscuit-terminal` components (`Prose`, `UnorderedList`) and keep the
    plain-text and terminal fallbacks.
  - Repository facts go to stdout. Any "use the full request" hint goes to
    stderr and is suppressed under `--json`.
- [x] **CLI tests**
  - In `sniff/cli/tests/l1/cli.rs`, replace the `lockfile_match` assertions
    (around lines 1038, 1177, 1196) with object assertions.
  - Add disposable-repository cases using the existing fixture harness:
    `match`, `mismatch` (with `missing` populated), `unreadable`, and
    `not_requested` (from a structure-only command, if one exists). Each case
    runs in both `--json` and human modes.
  - Assert exactly one valid JSON document on stdout, an empty or hint-only
    stderr, and exit code 0 for `mismatch` and `unreadable`.
  - No live checkout and no package-manager binaries.

**Wave 2**

- [x] **Validation checkpoint 4**
  - `just test` in `sniff/` passes for the lib and CLI.
  - A manual run of `sniff repo structure --json | jq .` against a Phase 1
    fixture shows the object.
  - Human output was reviewed in a real terminal and piped to a file.

## Phase 5: Hardening, measurements, docs, and cross-OS verification

**Wave 1** (concurrent)

- [x] **Isolation and caching tests**
  - Add L1 tests for:
    - nested and overlapping layers with different authorities;
    - two layers sharing one lockfile (read once);
    - repeat calls with a cheaper request (no inherited upgrade);
    - filename precedence, including a failed selected file with no retry;
    - a directory where the lockfile should be;
    - the metadata-failure seam;
    - symlinked lockfile and root spellings (use `canonicalize` for the macOS
      temp dir per the `os` skill);
    - `.hidden` members;
    - Windows-native separator conversion (a `#[cfg(windows)]` case plus a
      portable unit test of the normalizer).
- [x] **Counter tests**
  - Disabled structure request: 0 reads, 0 parses, probes > 0, no additional
    walk counter.
  - Enabled request: each selected file read and parsed once, and a cached
    failure is not retried.
  - Full request with corroboration disabled: still reads `Cargo.lock` for
    dependency versions, reports `not_requested`, and never upgrades
    provenance.
  - Propagate collection into workers, if any exist, per the counters
    contract.
- [x] **Complete-result matrix**
  - Extend `sniff/lib/tests/l1/lockfile_provenance.rs` to assert full
    serialized `RepoInfo` for every authority and every applicable status,
    plus orchestrator-only fixtures that produce no synthetic layer.
- [x] **Measurements**
  - Use release-mode timing and peak/retained allocation (dhat or the
    existing perf harness) on large generated JSON (npm), JSONC (Bun), YAML
    (pnpm and Yarn), and TOML (uv and Cargo) fixtures.
  - Record input size, tool version, request shape, host, repetitions, and
    cache state in `measurements.md` under the `2026-09-20-repo-perf`
    protocol.
  - Run a passive corpus pass over local checkouts and record statuses only;
    this is not a test dependency. Tests use counters, not wall-clock
    thresholds.
- [x] **Documentation**
  - Update `sniff/lib/README.md` and `sniff/cli/README.md`: the breaking
    removal of `lockfile_match`, the new object, the status and reason tables,
    root exclusion, Cargo `members_*` semantics, and `standalone_lockfiles`.
  - Update `sniff/docs/` (the architecture and CLI docs), and regenerate CLI
    docs with `just docs_cli` if the help text changed.
  - Update `.claude/skills/sniff/` (`architecture.md`, `performance.md` for
    probe cost and counters, and `remote-and-repository.md` if it describes
    layers).
  - If S2 added a crate, update `sniff/docs/dependencies.md` and the root
    `docs/dependencies.md`.
  - Do a final `rg lockfile_match` sweep and fix any stale "committed lockfile"
    comments.

**Wave 2** (sequential)

- [x] **Local gates**
  - Run `just test` and `just lint` in `sniff/`.
  - Run `cargo clippy -p sniff -p sniff-cli --all-targets -- -D warnings`.
  - Run `just test` in `claudine/` for the affected packages. The Claudine
    change is constructor-only, so run `cargo nextest` filtered to the
    `claudine` library package if the full area recipe is heavy.
  - Run the Darkmatter tests named in `2026-09-20-repo-perf`, plus
    `cargo check --all-targets` for the direct reverse dependencies of
    `sniff`: model-citizen, playa, worktree, unchained-ai, messenger,
    biscuit-terminal, biscuit-speaks, claudine, darkmatter, research, and
    scripts.
  - Do not run workspace-wide gates.
- [x] **Cross-OS verification**
  - Following the `os` skill, reuse qualifying evidence first, then run the
    `sniff` and `sniff-cli` L1 suites on native Windows and WSL2 hosts, in
    addition to macOS and Linux.
  - Pay particular attention to path normalization and the directory-as-file
    failure test on Windows.
  - Add no CI matrix cells and change no event scheduling.
- [x] **Implementation log and close**
  - Write `implementation-log.md` covering the ruling outcomes, deviations,
    and the evidence locations.
  - Set the spec's frontmatter to `status: implemented` and
    `implemented: true`.
  - Stop at "implementation complete, ready for review". Do not move the spec
    to `_completed` and do not run `just complete`.
- [x] **Final checkpoint**
  - Every Success item 1–8 above is checked with a pointer to its evidence.
  - Every commit is signed with the author identity and has no agent
    attribution trailers, and `git verify-commit` passes for each one.
  - _Phase 5 note:_ the Success 1–8 evidence is in the "Success criteria
    evidence" list of `implementation-log.md` `## Phase 5`. The implementing
    session was told not to stage or commit, so the signing and
    `git verify-commit` checks belong to the separate commit step.

## Dependency and parallelism summary

| Phase | Depends on | Parallel opportunity |
|---|---|---|
| 1 | — | Wave 1: rulings, S1–S4, and the unscheduled fix are all concurrent |
| 2 | 1 (rulings, fixtures, S3/S4) | Wave 1 is one agent (shared core); Wave 2 runs pnpm, uv, and Cargo concurrently |
| 3 | 2 Wave 1 (engine and stubs) | Six format/fallback/standalone tasks run concurrently, one file each |
| 4 | 2 (frozen types) | Runs alongside Phase 3; its checkpoint waits for Phase 3 |
| 5 | 3, 4 | Wave 1 runs tests, measurements, and docs concurrently; Wave 2 is sequential gates |

**Main risks:**

- Real-tool fixture availability (Rush, Poetry, PDM, Gradle, Bazel, and old npm
  and Yarn are not installed on the dev Mac). Mitigation: use ephemeral
  `npx`/`uvx` runs, with transcribed documentation samples where no tool can
  run, noted in `PROVENANCE.md`.
- uv's unstable lockfile format. Mitigation: pinned fixtures, and unknown
  versions report `unverifiable`.
- Edit contention in `detection.rs`. Mitigation: Phase 2 Wave 1 is a single
  agent, and later waves edit only their own `lockfile/*.rs` files.
