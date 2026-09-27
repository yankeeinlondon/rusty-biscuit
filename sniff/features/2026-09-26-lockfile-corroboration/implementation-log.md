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
packages:
    - sniff
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
