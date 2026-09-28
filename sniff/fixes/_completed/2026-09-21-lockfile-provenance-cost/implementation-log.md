---
fix: 2026-09-21-lockfile-provenance-cost
spec: sniff/fixes/2026-09-21-lockfile-provenance-cost/spec.md
plan: sniff/fixes/2026-09-21-lockfile-provenance-cost/plan.md
deferred_perf_measurement: false
implementation_1: "2026-09-26T12:57:31-07:00"
implementation_2: "2026-09-26T15:30:33-07:00"
---

# Implementation log — Lockfile provenance cost

## Implementation of Review Findings #1

> **started at:** 2026-09-26T12:57:31-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/fix-sniff/sniff/fixes/2026-09-21-lockfile-provenance-cost/review-1.md'
- this is iteration 1 of the review-to-implement cycle
- orchestration notes
        - the review found no implementation at all, so its four findings together cover the whole spec; `plan.md` in this directory breaks that work into phases and is followed here
        - findings run serially in dependency order: the consumer audit (finding 4) first, because it decides the structure default; then the mechanism and gate (finding 1); then the typed Cargo parser (finding 2); then verification and performance (finding 3)
        - the worktree holds unrelated uncommitted changes from other streams (`nested.rs` worker-cap work, `2026-09-20-repo-perf` docs, other specs); none of them are touched or staged
        - host at start: 16 logical cores, 1-minute load average 5.43
- starting the work on 'Public compatibility policy has no consumer audit or author decision' at 12:58:00
        - scope of the audit: text search over every Rust crate, `scripts/`, `just/`, every package-area `justfile`, and `.github/`, for `.provenance`, `lockfile_match`, `PackageProvenance`, every `detect_repo*` entry point, every `RepoRequest::{structure,focused,full}()` plan, and every `sniff repo ... --json` invocation; read-only, no source changed
        - no production code anywhere in the workspace reads `MonorepoLayer::provenance`, `MonorepoLayer::lockfile_match`, or `Package::provenance`; the only readers are Sniff's own tests, and `PackageSeed::provenance` is read only to copy it into `Package` (`detection.rs:1706`) and in seed merging (`seed.rs:134`)
        - the only outputs that expose the values are serde serializations of `MonorepoLayer` / `Package` inside a complete `RepoInfo` (or its `monorepo_layers`), all in the Sniff CLI
        - consumer audit (table below)

| Consumer | Location | Request tier | Reads/serializes | Depends on old value |
|---|---|---|---|---|
| Darkmatter ambient repository capture | `darkmatter/lib/src/markdown/compose/context/capture/snapshot.rs:447` | direct `detect_repo_structure` | neither; reads root, packages, and package areas for the scope catalog and `Repo` ctx keys | no |
| `md compose` launch and source package area | `darkmatter/cli/src/commands/compose.rs:200`, `:258` | direct `detect_repo_structure` | neither; `package_area_label_for_dir` only | no |
| Darkmatter empty-package-area test | `darkmatter/lib/tests/l1/empty_package_area.rs:43` | direct `detect_repo` (full) | neither | no |
| Claudine composition resolve | `claudine/lib/src/composition/resolve.rs:73`, `:146` | direct `detect_repo_structure` | neither; package area and scope catalog | no |
| Claudine lifecycle control | `claudine/lib/src/composition/lifecycle/control.rs:316` | direct `detect_repo_structure` | neither; package area | no |
| Claudine completion scopes | `claudine/cli/src/completion/scopes.rs:218` | direct `detect_repo_structure` | neither; stores `RepoInfo` in memory, never serialized | no |
| Claudine `LaunchContext::from_cwd` | `claudine/lib/src/system_prompt/context.rs:46` | `RepoRequest::structure()` plan | neither; projects to its own context type | no |
| Claudine `detect_environment_fast` | `claudine/lib/src/events/environment.rs:388` (`.repo` at `:396`) | `RepoRequest::structure()` plan | neither; `EnvironmentContext` projects `authority`, `orchestrators`, and package names only (`:275-300`) | no |
| Claudine `detect_environment` | `claudine/lib/src/events/environment.rs:365` | default `FilesystemRequest` → `RepoRequest::full()` | neither; same projection | no |
| Claudine invocation context topology | `claudine/lib/src/invocation_context.rs:1771`, `:1817` | `RepoRequest::structure()` plan | neither; `RepoInfo` and `SniffResult` held in memory and projected, never serialized | no |
| Claudine repo-root and package-area probes | `claudine/lib/src/linking/paths.rs:338`, `composition/sequence/source.rs:106`, `claudine/cli/src/commands/mcp/mod.rs:206`, `wrap/env/package_context.rs:27`, `:162` | direct `detect_repo` (full) | neither; root, `is_monorepo`, package area | no |
| `scripts/drift.rs` | `scripts/drift.rs:763` | direct `detect_repo` (full) | neither; package areas | no |
| Sniff `detect_repo_identity` | `sniff/lib/src/filesystem/repo/identity.rs:102` | direct `detect_repo_structure` | neither; `is_monorepo`, package count | no |
| Sniff `detect_repo_packages` (docs) | `sniff/lib/src/filesystem/docs.rs:174` | direct `detect_repo_structure` | neither; names and paths | no |
| Sniff recent-commits attribution | `sniff/lib/src/filesystem/git/recent_commits/collect.rs:73` | direct `detect_repo_structure` | neither; package catalog | no |
| Sniff package-area, blast radius | `sniff/lib/src/filesystem/repo/area.rs:50`, `blast_radius.rs:101` | direct `detect_repo` (full) | neither | no |
| Sniff filesystem request default | `sniff/lib/src/filesystem/mod.rs:518` | `structure()` only when a plan has no repo request | neither | no |
| Sniff `detect_repo_aggregate` / bare `sniff repo --json` | `sniff/lib/src/filesystem/repo/aggregate_view.rs:170`, `sniff/cli/src/commands/mod.rs:1243`; serialized at `sniff/cli/src/output/repo_json.rs:791-799` | `RepoRequest::focused(RepoDetailRequest::all())` | **serializes** `structure.monorepo_layers[]` with `provenance` and `lockfile_match`; on this checkout today it emits `pnpm-workspaces` `"lockfile"` / `true` | not a structure-tier consumer, but the spec's `focused(...)` default of `false` changes this output; it can keep the old value by opting in explicitly in `aggregate_request()` |
| `sniff repo structure --json` and other unspecialized `repo` actions | `sniff/cli/src/commands/mod.rs:1298` (fallback plan); `repo_json.rs:586`, `:598`, `:986` | default `FilesystemRequest` → `RepoRequest::full()` | **serializes** complete `RepoInfo` | no; full keeps corroboration under option A |
| `sniff --json`, `sniff filesystem --json` | `sniff/cli/src/output/mod.rs:702`, `:731` | default plan → `RepoRequest::full()` | **serializes** complete `FilesystemInfo.repo` | no; full keeps corroboration |
| `sniff repo is-monorepo --json` | `sniff/cli/src/commands/mod.rs:780`; `repo_json.rs:375` | direct `detect_repo_structure` | neither; `authority`, `orchestrators` | no |
| `sniff repo packages` / `package-areas` / `version` / `package-count` | `sniff/cli/src/commands/repo.rs:54`, `:139`, `:675`; `commands/mod.rs:832`, `:250-375` | direct `detect_repo_structure[_or_root_package]` | neither; names, areas, versions, count | no |
| `sniff repo package-dependencies` / `package-manager` / `test-runner` | `sniff/cli/src/commands/repo.rs:303`, `:373`, `:546` | `RepoRequest::focused(...)` | neither; hand-built allowlists (`repo_json.rs:462`) | no |
| `sniff repo git-status --package` | `sniff/cli/src/commands/mod.rs:1288` | `RepoRequest::structure()` plan | neither; scope paths | no |
| CI planner tests | `scripts/ci/test_resolved_plan.py:122`, `:561` | `sniff repo package-area --json`, `package-areas --json` | neither; names only | no |
| justfile recipes | `just/devops.just:2711`, `:437`, `:2557`, `:2906`; `just/{plan,review,spec,coverage}.just`; `homelab/justfile:235`; `claudine/justfile:376`; `queue/justfile`, `biscuit-file/justfile` | text-mode `package-area`, `package-areas`, `packages`, `root`, `git-status`, `staged-files`, `package-dependencies` | neither | no |
| "provenance" hits in CI | `scripts/ci/schema.py`, `.github/ci/schemas/contract.json`, `.github/workflows/{ci,_package-ci}.yml`, `just/devops.just:535-667`, `scripts/ci-rollup-tests.rs`, `scripts/ci-build-archive*.rs` | n/a | unrelated CI-evidence, skip-policy, and build-archive provenance; none invokes Sniff for repository topology | no |

        - S1 load sites and their gating (all five lockfile read paths funnel through `ManifestStore`; no Sniff code reads a lockfile outside it)
            - `ManifestStore::cargo_lock` (`detection.rs:397`) → `CargoLockVersions::parse` (`manifest_index.rs:28`, reached only through `cargo_lock`; it does its own `read_to_string` at `:30` rather than calling `read_counted_lockfile`, and duplicates that helper's `FS_FILE_OPENS` / `FS_BYTES_READ` / `REPO_LOCKFILE_PARSES` increments, so the new read-attempt counter must be added in both places)
            - `ManifestStore::pnpm_lock` (`detection.rs:407`) and `uv_lock` (`:419`) → `read_counted_lockfile` (`:476`, whose only callers are these two)
            - caller 1: `synthesize_root_package_repo_with_store` (`detection.rs:174-178`) → `cargo_lock`, gated on `request.wants_dependencies()`; this is the root-package synthesis path for both `detect_repo_structure_or_root_package` (via `synthesize_root_package_repo`, which passes `RepoRequest::structure()`) and the in-detection `include_root_package` branch (`:703`); it runs only when no workspace outcome exists, so it never reaches `upgrade_provenance_with_lockfile`
            - caller 2: `lock_versions_for_seed` (`detection.rs:1674`, called at `:777-781`) → `cargo_lock`, gated on `request.wants_dependencies()`
            - caller 3 (ungated): `upgrade_provenance_with_lockfile` (`detection.rs:747-750`, `:890`) → `pnpm_lockfile_matches` (`:949` `pnpm_lock`), `uv_lockfile_matches` (`:996` `uv_lock`), `cargo_lockfile_matches` (`:1041` `cargo_lock`, plus `ManifestStore::cargo` member-manifest reads); it runs once per layer, including nested layers
            - nested and polyglot paths: `nested.rs` and `polyglot.rs` call no lockfile accessor; nested workspaces become ordinary layers and reach lockfiles only through caller 3; `npm.rs:159`, `:249`, `:411-420` and `detection.rs:1434-1435` only `probe_exists` a lockfile (existence probe, no open or parse)
            - `cargo.rs:96`, `:129` receive `Option<&CargoLockVersions>` from the callers above and never load one
            - conclusion: the spec's claim holds; in the structure tier `upgrade_provenance_with_lockfile` is the only path that reads or parses a lockfile, and no new ungated site exists, so no extra Phase 3 task is needed
        - test expectations that pin the old structure-tier values (these are test expectations, not consumers)
            - `sniff/lib/tests/l1/integration.rs:728` `test_pnpm_lockfile_parity_upgrades_provenance`: `detect_repo_structure` → layer `Lockfile`, `lockfile_match == Some(true)`, and every package `Lockfile`
            - `sniff/lib/tests/l1/integration.rs:759` `test_pnpm_lockfile_drift_records_mismatch`: `lockfile_match == Some(false)` (layer `Globbed` still holds)
            - `sniff/lib/tests/l1/integration.rs:776` `test_pnpm_lockfile_superset_is_recorded_as_mismatch`: `lockfile_match == Some(false)`
            - `sniff/lib/tests/l1/integration.rs:803` `test_uv_lockfile_parity_upgrades_provenance`: layer `Lockfile`, `Some(true)`
            - `sniff/lib/tests/l1/integration.rs:819` `test_uv_lockfile_superset_is_recorded_as_mismatch`: `Some(false)`
            - `sniff/lib/tests/l1/integration.rs:997` `test_rusty_biscuit_repo_topology_parity`: live-checkout `detect_repo_structure`; asserts `homelab/server/frontend` has `PackageProvenance::Lockfile` (`:1042-1046`)
            - `sniff/lib/src/filesystem/repo/detection.rs:3079` `inherited_cargo_root_manifest_is_parsed_once_per_detection`: structure tier (`detect_repo_inner(.., true)`); `FS_FILE_OPENS == MEMBERS + 2` counts "the one absent Cargo.lock observation"; that file-open count is recorded before the lockfile is known to be absent, so it drops to `MEMBERS + 1` once structure declines corroboration
            - `sniff/lib/src/filesystem/repo/detection.rs:3108` `cargo_lock_is_shared_by_provenance_and_dependency_enrichment`: `focused(dependencies)`; `REPO_LOCKFILE_PARSES == 1` still holds through the dependency read, but the name and premise ("shared by provenance") no longer hold with focused corroboration off by default; either opt this test in or rename it
            - not affected: the CLI insta snapshots `l1__snapshots__{cargo_monorepo,cargo_pnpm_monorepo,pnpm_nx_monorepo}_structure_json.snap` and `l1__snapshots__repo_aggregate_json.snap` (fixtures have no lockfile, so they already pin `"globbed"` and omit `lockfile_match`), `repo_json.rs` unit tests at `:2832` and `:2926` (hand-built `Globbed` values), Claudine `environment.rs:546` (hand-built layer), and the zero-parse structure assertions at `detection.rs:3229`, `recent_commits.rs:836`, and `aggregate_view.rs:931` (lockfile-free fixtures; they become stronger, not wrong)
        - R1 conclusion: **Option A stands.** No structure-tier consumer (direct `detect_repo_structure` / `detect_repo_structure_or_root_package` call or `RepoRequest::structure()` plan) reads or serializes `provenance` or `lockfile_match`; Darkmatter and Claudine project only root, packages, areas, authority, and orchestrators
            - every public output that serializes a complete `RepoInfo` (`sniff repo structure --json`, `sniff --json`, `sniff filesystem --json`) runs on `RepoRequest::full()`, which keeps corroboration under A, so their output is unchanged
            - the one output that changes is not structure-tier: bare `sniff repo --json` (`detect_repo_aggregate`) uses `RepoRequest::focused(RepoDetailRequest::all())` and serializes `structure.monorepo_layers[].provenance` and `lockfile_match`; under the spec's focused default (`false`) it would stop reporting `"lockfile"` and `lockfile_match`; this is movable to an explicit request (`aggregate_request()` opts in), so it does not force option B; whether the aggregate should pay for the parse or drop the field is a smaller policy question for the author; the default recommendation is to opt it in so existing output is unchanged
            - required test updates under A: the six `integration.rs` tests above move to an explicit corroborating request (or assert the new manifest-derived values), and the two `detection.rs` counter tests are adjusted as noted
        - orchestrator decision: adopt **Option A** provisionally, per the plan's ruling R1 (`yolo: true`), because the audit found no structure-tier consumer that depends on the old value; the author's `human_review` decision in `review-1.md` is still open, and the default is confined to the constructors in `request.rs`, so the author can reverse it without undoing the mechanism
        - orchestrator decision for bare `sniff repo --json`: `aggregate_request()` opts in to lockfile provenance explicitly, so that serialized output stays byte-stable
- work completed for 'Public compatibility policy has no consumer audit or author decision' at 13:03:24
- starting the work on 'Structure detection still reads lockfiles without a request for provenance' at 13:03:24
        - discovered: `upgrade_provenance_with_lockfile` is the only lockfile read on the structure path (matches the S1 audit above); `CargoLockVersions::parse` does its own read, so the read-attempt counter goes in both read sites; the performance counter catalog is `performance/counters.rs` alone (`perf_tree.rs` fixtures are generic captured reports, not a catalog), so no other registration was needed
        - request setting (`sniff/lib/src/request.rs`): public `RepoRequest::lockfile_provenance: bool` with `#[serde(default = "lockfile_provenance_default")]` (`true`) and no `skip_serializing_if`; `structure()` and `focused(..)` set `false`, `full()` sets `true`; added `with_lockfile_provenance(self, bool) -> Self` and `wants_lockfile_provenance()`; field and constructor docs state that the setting controls corroboration only and that a dependency request still reads `Cargo.lock`
        - counter: `REPO_LOCKFILE_READS = "filesystem.repo.lockfile_reads"` in `performance/counters.rs`, incremented before the read in `read_counted_lockfile` (`detection.rs`) and `CargoLockVersions::parse` (`manifest_index.rs`, parsing strategy untouched); `REPO_LOCKFILE_PARSES` unchanged
        - gate: the `upgrade_provenance_with_lockfile` loop in `detect_repo_inner_with_shared_request_and_ownership` runs only when `request.wants_lockfile_provenance()`; dependency-version reads (`lock_versions_for_seed`, root-package synthesis) stay gated on `wants_dependencies()` only
        - `aggregate_request()` (`aggregate_view.rs`) opts in with `.with_lockfile_provenance(true)`, so bare `sniff repo --json` keeps its `provenance` / `lockfile_match` output (the `repo_aggregate_json` snapshot is unchanged)
        - docs: `detect_repo`, `detect_repo_structure`, `detect_repo_with_request`, `detect_repo_structure_or_root_package` (`types.rs`); `MonorepoLayer::provenance` and `lockfile_match` (`standard.rs`) now list the three meanings of `None` (not requested, absent or unparseable lockfile, authority without corroboration); `sniff/lib/README.md` topology JSON; `sniff/cli/README.md` `repo structure` row; `.claude/skills/sniff/performance.md` (new "Lockfile corroboration is a request option" subsection, appended after the existing uncommitted edits, which were kept); `.claude/skills/sniff/SKILL.md` request-cost table; `.claude/skills/sniff/architecture.md` package-topology paragraph
        - comment/code drift found and resolved (code taken as correct)
            - `MonorepoLayer::lockfile_match` said "`None` when no lockfile was parsed"; rewritten for the request-driven meaning
            - `detect_repo_structure` carried an unused `[RepoRequest::structure]` link definition while linking an undefined `RepoRequest::focused`; both are now defined and used
            - `inherited_cargo_root_manifest_is_parsed_once_per_detection` asserted `MEMBERS + 2` file opens "plus the one absent Cargo.lock observation"; structure now reads no lockfile, so it asserts `MEMBERS + 1` and `REPO_LOCKFILE_READS == 0`
            - `cargo_lock_is_shared_by_provenance_and_dependency_enrichment` no longer exercised provenance under the new focused default; it now opts in explicitly, asserts exactly 1 read and 1 parse, and asserts the layer is `Lockfile` / `Some(true)`, so its name matches what it proves
        - files changed: `sniff/lib/src/request.rs`, `sniff/lib/src/performance/counters.rs`, `sniff/lib/src/filesystem/repo/detection.rs`, `sniff/lib/src/filesystem/repo/manifest_index.rs`, `sniff/lib/src/filesystem/repo/aggregate_view.rs`, `sniff/lib/src/filesystem/repo/types.rs`, `sniff/lib/src/filesystem/repo/standard.rs`, `sniff/lib/tests/l1/integration.rs`, `sniff/lib/README.md`, `sniff/cli/README.md`, `.claude/skills/sniff/performance.md`, `.claude/skills/sniff/SKILL.md`, `.claude/skills/sniff/architecture.md`
        - tests added (unit)
            - `request.rs`: `legacy_repo_request_json_without_lockfile_provenance_corroborates`, `repo_request_constructors_set_lockfile_provenance_defaults`, `lockfile_provenance_builder_opts_in_and_out`, `lockfile_provenance_is_always_serialized`; extended `legacy_repo_request_json_omits_and_defaults_focused_details` (declined and opted-in round trips) and `detection_plan_serialization_roundtrip`
            - `detection.rs` (`observation_index` module): `absent_cargo_lock_counts_one_read_attempt_and_no_parse` (1 read, 0 parses, `lockfile_match: None`); `declined_corroboration_still_resolves_dependency_versions_from_cargo_lock` (focused dependencies, 1 read, 1 parse, `Globbed`, `None`, `serde` resolved to `1.0.0`); `declining_structure_request_reads_and_parses_no_lockfile` and `opted_in_structure_request_corroborates_every_lockfile_once` on a new `three_lockfile_workspace` fixture (Cargo + pnpm + uv, each with a matching lockfile): declined → both counters 0 and absent from the report, no `Lockfile` provenance, every `lockfile_match` `None`; opted in → 3 reads, 3 parses, every layer `Lockfile` / `Some(true)`
        - tests updated (L1 `integration` module): `test_pnpm_lockfile_parity_upgrades_provenance`, `test_pnpm_lockfile_drift_records_mismatch`, `test_pnpm_lockfile_superset_is_recorded_as_mismatch`, `test_uv_lockfile_parity_upgrades_provenance`, `test_uv_lockfile_superset_is_recorded_as_mismatch`, and `test_rusty_biscuit_repo_topology_parity` now detect through `detect_repo_with_request` with `RepoRequest::structure().with_lockfile_provenance(true)` (unchanged expectations), and each also asserts that plain `detect_repo_structure` reports manifest-derived provenance with `lockfile_match: None` (new helpers `detect_structure_with_lockfile_provenance` and `assert_plain_structure_skips_corroboration`)
        - results
            - `just test` in `sniff/`: 2883 passed, 32 skipped, 0 failed
            - `just lint` in `sniff/`: exit 0
            - `cargo clippy -p sniff --all-targets -- -D warnings` and `cargo clippy -p sniff-cli --all-targets -- -D warnings`: clean
            - `cargo check -p darkmatter -p claudine`: clean (no `RepoRequest { .. }` literal exists outside `request.rs`)
            - `cargo doc -p sniff --no-deps` with intra-doc-link warnings: no new warnings in changed items; pre-existing broken links (for example `SniffError::Git`, `FrontmatterOnly`, `standard.rs:408` `Package::relative`) are unrelated and left alone
            - no pre-existing test failures were observed, including with the uncommitted `nested.rs` work present
- work completed for 'Structure detection still reads lockfiles without a request for provenance' at 13:12:50
- starting the work on 'Cargo lockfiles still use the generic TOML parser' at 13:12:50
        - discovered: the old `CargoLockVersions::parse` deserialized into `toml::Value` and took `package` only via `get(..).as_array()`, and each entry's `name`/`version` via `get(..).as_str()`; so a non-table entry, a non-string field (integer, table, array, or datetime), a missing `package`, and a non-array `package` (string, integer, datetime, or a `[package]` table) were all skipped or empty, never `None`; only invalid TOML (including duplicate keys) yielded `None`
        - design (`sniff/lib/src/filesystem/repo/manifest_index.rs`)
            - `parse` keeps the read, the `REPO_LOCKFILE_READS` and `REPO_LOCKFILE_PARSES` counters, and the byte counters exactly where they were, then calls a new private `from_lock_str(&str)`
            - `from_lock_str` deserializes, through `biscuit_file::toml_crate` (toml 1.x), into a private `CargoLockDocument { package: LockedPackages }` whose entries are `LockedPackage { name: Option<String>, version: Option<String> }`; unknown keys at both levels go to serde's `IgnoredAny`, so `source`, `checksum`, `dependencies`, `metadata`, and the top-level `version` are never allocated
            - leniency comes from three hand-written `deserialize_any` visitors: `string_or_none` (string → `Some`; scalars → `None`; arrays and maps, which include TOML datetimes, are drained with `IgnoredAny` → `None`), `LockedPackageEntry` (a table → a `LockedPackage` via `MapAccessDeserializer`; anything else → skipped), and `LockedPackages` (an array → entries in order; anything else → empty)
            - entries are folded into the same `HashMap<String, Vec<String>>` in lockfile order, so `resolve` still returns the first recorded version
            - the old body is kept as `#[cfg(test)] fn parse_reference(&str)`, the parity oracle
            - `parse` docs now state the retained-fields contract, the skip rules, and that `None` means an unreadable file or invalid TOML only
        - files changed: `sniff/lib/src/filesystem/repo/manifest_index.rs` only
        - tests added (unit, `manifest_index::tests`); each compares the typed and reference results with an independently written expected map
            - `cargo_lock_typed_parse_keeps_duplicate_name_versions_in_lockfile_order`: `syn` at three versions, interleaved with another package, plus `source`/`checksum`/`dependencies`/`[metadata]`; order is `2.0.100, 1.0.109, 2.0.50` and `resolve("syn")` is `2.0.100`
            - `cargo_lock_typed_parse_skips_malformed_entries_like_reference`: missing name, integer version, integer name, table version, date version, a string/integer/array/datetime element, and missing version; only `kept` survives
            - `cargo_lock_typed_parse_treats_non_array_package_as_empty_like_reference`: missing, string, integer, datetime, and `[package]` table forms all give `Some(empty)`
            - `cargo_lock_typed_parse_rejects_invalid_toml_like_reference`: unterminated header, duplicate key, unterminated array all give `None`
            - `cargo_lock_typed_parse_matches_reference_on_workspace_lockfile`: full map equality on this checkout's `Cargo.lock`, read through `include_str!("../../../../../Cargo.lock")` so CI's test-input index schedules it when the lockfile changes
        - informal timing (not protocol evidence): a temporary `#[ignore]`d unit test run with `--release` on this Mac, 5 warm-up rounds then 50 alternating rounds on the 396,787-byte `Cargo.lock`; typed min 1.74 ms / median 1.82 ms, reference min 2.35 ms / median 2.44 ms (about 25% less parse time); 1-minute load average 8.46 before and 9.00 after (`sysctl -n vm.loadavg`); the scratch test was deleted afterwards
        - results
            - `just test` in `sniff/`: 2888 passed, 32 skipped, 0 failed (the previous run's 2883 plus the 5 new tests)
            - `just lint` in `sniff/`: exit 0
            - `cargo clippy -p sniff --all-targets -- -D warnings`, the same with `--features remote`, and `cargo clippy -p sniff-cli --all-targets -- -D warnings`: clean
            - `rustfmt --check --edition 2024` on `manifest_index.rs` (read-only, no rewrite): clean
- work completed for 'Cargo lockfiles still use the generic TOML parser' at 13:18:53
- starting the work on 'Requested behavior and performance have no new verification' at 13:18:53
        - functional verification
            - AC3 provenance fixture matrix: new module `sniff/lib/tests/l1/lockfile_provenance.rs`, declared in the consolidated `l1` target (`tests/l1/main.rs`); temp-dir fixtures for Cargo (`crates/*`), pnpm (`packages/*`), and uv (`py/*` plus the always-counted root), each × matching, extra lockfile member, missing lockfile member, absent, and unparseable lockfile (15 cells)
                - `corroborating_requests_report_hand_written_provenance_for_every_lockfile_state`: every cell under `RepoRequest::structure().with_lockfile_provenance(true)` (via `detect_repo_with_request`) and under `detect_repo` (full); asserts against a hand-written table `is_monorepo`, `root`, the detected standards, the complete `MonorepoLayer` (root, authority, orchestrators, `provenance`, `lockfile_match`, `root_is_package`, packages), and per-package relative path, path, name, package area, standard, ecosystem, and provenance; with a fresh `PerformanceCollector`, `REPO_LOCKFILE_READS == 1` in every cell and `REPO_LOCKFILE_PARSES == 1`, or `0` when the lockfile is absent
                - expected table: matching gives `Lockfile` / `Some(true)` for all three; missing member gives `Globbed` / `Some(false)`; absent and unparseable give `Globbed` / `None`; extra member gives `Lockfile` / `Some(true)` for Cargo but `Globbed` / `Some(false)` for pnpm and uv
                - `extra_lockfile_member_corroborates_cargo_but_not_pnpm_or_uv`: the explicit assertion pair for that asymmetry
                - `declining_requests_report_manifest_provenance_and_read_no_lockfile`: every cell under plain `RepoRequest::structure()` and `detect_repo_structure`; the same complete assertions with `Globbed` layer and package provenance and `lockfile_match: None`, and zero lockfile reads and parses
                - paths compare as `PathBuf` built component by component from the temp root, and layer and package relative paths are normalized from `\` to `/`, the same normalization the existing lockfile tests use, so the assertions hold on Windows
                - scope: the existing request round-trip, counter, and parser parity unit tests and the six updated `integration.rs` lockfile tests were not duplicated
            - CLI JSON tests (`sniff/cli/tests/l1/cli.rs`, `l1` target), on `create_cli_monorepo` plus a committed `Cargo.lock` naming both members
                - `repo_structure_tier_json_is_json_only_with_a_lockfile_present`: `sniff repo is-monorepo --json` stdout is exactly `{"authority":"cargo-workspace","is_monorepo":true}` and `sniff repo packages --json` stdout is exactly `["pkg-a","pkg-b"]`; neither structure-tier projection serializes provenance
                - `repo_structure_json_reports_lockfile_provenance_for_matching_lockfile`: `sniff repo structure --json` (full request) reports the Cargo layer with `"provenance": "lockfile"`, `lockfile_match: true`, and both packages with `"lockfile"` provenance
                - `repo_aggregate_json_reports_lockfile_provenance_for_matching_lockfile`: bare `sniff repo --json` reports `structure.monorepo_layers[0]` with `"lockfile"` and `lockfile_match: true`
                - each test parses the whole stdout as one JSON document; no insta snapshot changed
            - Darkmatter (AC5): `just test --no-fail-fast` in `darkmatter/` ran 8498 tests: 8496 passed, 2 failed, 12 skipped
                - the three observation-boundary tests passed: `markdown::compose::tests::lazy_roots::ambient_repository::the_observation_is_fixed_at_request_creation`, `...::a_child_reads_the_observation_fixed_at_request_creation`, and `...::an_older_constructor_request_is_fixed_at_the_root_entry_not_by_the_child`
                - `git diff -G REPOSITORY_DISCOVERY_COUNT` is empty, and no Darkmatter or Claudine file is modified
                - pre-existing, not caused by this change: `feature_review_incident::the_shipped_feature_review_composes_and_names_its_spec` and `feature_review_incident::the_typo_is_one_identifier_under_the_current_grammar` fail with `File not found: ../_writing-clearly.md`; commit `6c682a7fd` added a `::file ../_writing-clearly.md` transclusion to `prompts/_reviews/feature-review.md`, and the test's `repository()` fixture copies only `_senior-reviewer.md`, `_ready.md`, and `_test-tiers.md`; left unfixed as out of scope
            - Claudine (AC5): `cargo nextest run -p claudine -p claudine-cli --features claudine-cli/test-fixtures -E 'test(/completion::scopes::|composition::resolve::|invocation_context|events::environment::/)'` ran 97 tests, all passed (`composition::resolve::tests`, `events::environment::tests`, `invocation_context::tests`, and `claudine-cli` `completion::scopes::tests`)
            - Sniff: `just test` 2894 passed, 32 skipped, 0 failed (the previous 2888 plus 3 matrix and 3 CLI tests); `just lint` exit 0; `cargo clippy -p sniff --all-targets -- -D warnings`, the same with `--features remote`, and `cargo clippy -p sniff-cli --all-targets -- -D warnings`: clean; `rustfmt --check` is clean for the new module and the added CLI tests
        - performance verification
            - trees (R8)
                - baseline: detached worktree `/tmp/lpc-baseline` at `HEAD` `ea73a87aa`, plus the uncommitted `nested.rs` change applied from `git diff HEAD`
                - changed: this worktree, which differs from the baseline only by this fix
                - separate `CARGO_TARGET_DIR` per tree; binary sha256s in `evidence/environment.md` and `evidence/binaries.sha256`
                - the baseline worktree was removed afterwards
            - harness
                - a scratch `sniff/lib/examples/lpc_bench.rs` in each tree, deleted immediately after its build; no source file remains in either tree
                - `CargoLockVersions` is `pub(crate)`, so each tree's `parse` was copied verbatim by `sed` into the example (`evidence/{baseline,after}/lpc_bench-parser.rs.txt`)
                - compose used `md` built from each tree, with cwd set to the checkout root and `evidence/trivial.md` as the document; this goes through the launch-repository and ambient `Repo` capture calls to `detect_repo_structure`
                - driver `evidence/measure.py.txt`, summarizer `evidence/summarize.py.txt`
            - protocol
                - 3 warmup rounds and 20 samples per case
                - rotating alternation of baseline and changed runs
                - median and range, labeled warmed-cache
                - 1-minute load recorded before and after every set
            - headline medians (warmed cache; ranges and loads in `results.md`)

                | Case | Profile | Baseline | Changed | Load before → after |
                |---|---|---|---|---|
                | parser (`Cargo.lock` read + parse) | release | 2.540 ms | 1.890 ms (−25.6%) | 7.49 → 7.49 |
                | parser | debug | 24.526 ms | 22.393 ms (−8.7%) | 7.38 → 6.95 |
                | `detect_repo_structure`, declined vs. baseline | release | 39.591 ms | 31.601 ms (−20.2%) | 7.49 → 7.68 |
                | same, provenance requested | release | 39.591 ms | 39.162 ms (−1.1%) | 7.49 → 7.68 |
                | `detect_repo_structure`, declined vs. baseline | debug | 130.875 ms | 75.015 ms (−42.7%) | 6.95 → 6.09 |
                | same, provenance requested | debug | 130.875 ms | 128.707 ms (−1.7%) | 6.95 → 6.09 |
                | ambient `md compose` (whole process) | release | 110.276 ms | 91.835 ms (−16.7%) | 9.18 → 8.85 |
                | ambient `md compose` (whole process) | debug | 342.856 ms | 234.017 ms (−31.7%) | 8.85 → 8.31 |

            - load: the 1-minute load stayed between 6.09 and 9.18 at every set boundary; no set was deferred
            - discarded run: the driver's first campaign gave both compose sides one label, so their samples merged; that run was discarded (see `evidence/environment.md` § Run history), and only the compose sets were measured again with distinct labels
            - not measured: suite-level wall clock, other operating systems, and `upgrade_provenance_with_lockfile` in isolation (the detection requested − declined difference stands in for it)
            - files written: `results.md`, `evidence/environment.md`, `evidence/summary-table.md`, `evidence/binaries.sha256`, `evidence/measure.log`, `evidence/measure.py.txt`, `evidence/summarize.py.txt`, `evidence/lpc_bench-common.rs.txt`, `evidence/trivial.md`, `evidence/baseline/*.json`, `evidence/after/*.json`, `evidence/{baseline,after}/lpc_bench-parser.rs.txt`
        - cross-OS evidence (orchestrator; `just cross-check` ships the uncommitted local tree)
            - `just cross-check sniff --os linux`: 2011 passed, 30 skipped
            - `just cross-check sniff --os windows` (native Windows): 2000 passed, 23 skipped, including the new `lockfile_provenance` fixture matrix with its path comparisons
            - macOS: local `just test` in `sniff/` (see the functional-verification entries above)
            - WSL2 was not run locally; CI's nightly schedule covers it
        - orchestrator verified the two Darkmatter failures (`feature_review_incident::*`, `File not found: ../_writing-clearly.md`) are unrelated: commit `6c682a7fd` added `::file ../_writing-clearly.md` to `prompts/_reviews/feature-review.md`, and this fix touches no Darkmatter or prompt file
- work completed for 'Requested behavior and performance have no new verification' at 13:59:08

### Successful Completion

The implementation of review cycle 1 has completed successfully in 1h 02m. During this implementation all 4 review findings were evaluated to see if they could be fixed as a part of this implementation cycle: 4 were fixed, 0 were deferred (see reasons below):

- no finding was deferred, and no performance measurement was deferred: every sample set ran with a 1-minute load between 6.09 and 9.18 on the 16-core host
- one item still needs the author: Option A was adopted provisionally under the plan's ruling R1, based on the consumer audit; the `human_review` decision in `review-1.md` is still open. Reversing it to Option B means changing the `structure()` default in `sniff/lib/src/request.rs` and moving Darkmatter's capture to an explicit declining request
- outstanding items outside this fix's scope
        - Darkmatter `just test` has 2 pre-existing failures (`feature_review_incident::*`) caused by the unrelated prompt change in `6c682a7fd`
        - nothing was committed; the changes remain uncommitted in the worktree next to other streams' edits (`just/devops.just`, `nested.rs`, `.claude/skills/os/build-hosts.md`), so staging must use explicit paths

The files changed by this cycle are:

- `sniff/lib/src/request.rs`, `sniff/lib/src/performance/counters.rs`, `sniff/lib/src/filesystem/repo/{detection,manifest_index,aggregate_view,types,standard}.rs`
- `sniff/lib/tests/l1/{integration,lockfile_provenance,main}.rs`, `sniff/cli/tests/l1/cli.rs`
- `sniff/lib/README.md`, `sniff/cli/README.md`, `.claude/skills/sniff/{SKILL,architecture,performance}.md`
- `sniff/fixes/2026-09-21-lockfile-provenance-cost/{implementation-log,results}.md` and `evidence/`

## Implementation of Review Findings #2

> **started at:** 2026-09-26T15:30:33-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/fix-sniff/sniff/fixes/2026-09-21-lockfile-provenance-cost/review-2.md'
- this is iteration 2 of the review-to-implement cycle
- orchestration notes
        - the review has 3 findings: 2 unblocked (complete-result verification, isolated corroboration measurement) and 1 blocked on the author (structure-request default)
        - the author's inline note on the blocked finding ("DO THE WORK") is taken as an instruction to resolve the default here instead of re-escalating it, recording the recommendation with its pros and cons so the author can still reverse it
        - findings run serially: the default decision first (it fixes the expectations the other two rely on), then the complete-result fixture, then the isolated measurement
        - the worktree still holds unrelated uncommitted changes from other streams (`just/devops.just`, `nested.rs`, `.claude/skills/os/build-hosts.md`); none are touched or staged
        - host at start: 1-minute load average 5.69
- starting the work on 'The structure-request default still awaits the author's decision' at 15:31:20
        - handled by the orchestrator directly: the code already implements Option A, and no source, doc, or skill file calls it provisional (only `plan.md` R1 did), so the work is recording the decision, not changing behavior
        - decision: **Option A (opt-in) is final**, and bare `sniff repo --json` keeps opting in so its lockfile-confirmed output is unchanged
        - context for the author, with examples
                - who changes under A: only callers of `detect_repo_structure`, `detect_repo_structure_or_root_package`, `RepoRequest::structure()`, or `RepoRequest::focused(..)` without `.with_lockfile_provenance(true)`. On this checkout they now see the `pnpm-workspaces` layer as `"provenance": "globbed"` with no `lockfile_match`, where they used to see `"lockfile"` / `true`. The audit found no such caller that reads either field (Darkmatter and Claudine read root, packages, package areas, authority, and orchestrators)
                - who is unchanged: `sniff repo structure --json`, `sniff --json`, `sniff filesystem --json`, `detect_repo`, and any `RepoRequest::full()` plan; also legacy serialized request plans, which deserialize with corroboration on
                - measured saving under A (review cycle 1, `results.md`): release `detect_repo_structure` 39.6 ms → 31.6 ms (−20%); ambient release `md compose` 110.3 ms → 91.8 ms (−17%); debug compose 342.9 ms → 234.0 ms (−32%)
        - options weighed
                - **A — opt-in (chosen).** Pro: every structure caller, Darkmatter included, gets the saving with no code change; "structure" means topology, and lockfile corroboration is not topology; matches the spec's recommendation. Con: a public result changes for structure-tier callers, and `lockfile_match: None` now also means "not requested"; mitigated by the audit (no reader), the field docs listing all three meanings of `None`, and the explicit `with_lockfile_provenance(true)` escape hatch
                - **B — opt-out.** Pro: no serialized output changes for anyone. Con: every structure caller keeps paying for a value none of them reads; Darkmatter and each Claudine call site would need migration to `detect_repo_with_request` with a declining request, and the convenience function `detect_repo_structure` would have no way to decline
                - for bare `sniff repo --json`: opting in (chosen) keeps the aggregate's serialized `provenance` / `lockfile_match` byte-stable at the cost of one lockfile parse per invocation, which is a small share of a command that already runs full Git and dependency detection; declining would silently change a documented JSON field for a user-facing command
        - reversibility: switching to B means changing the `lockfile_provenance` value in `RepoRequest::structure()` and `RepoRequest::focused(..)` (`sniff/lib/src/request.rs`) and updating the declining-request tests; the setting, gate, and counter stay
        - files changed: `spec.md` (a dated "Decision" note under the open question), `plan.md` (R1 marked adopted, "provisionally" removed)
        - no tests were needed: the existing L1 matrix (`declining_requests_report_manifest_provenance_and_read_no_lockfile`), the request constructor tests, and the CLI aggregate test already pin both halves of the decision
- work completed for 'The structure-request default still awaits the author's decision' at 15:31:20
- starting the work on 'Complete repository results are not verified against the pre-change contract' at 15:31:24
        - discovered
                - every serialized `RepoInfo` in the matrix carries one host-dependent field: `monorepo_standards[].binary`, the acting-binary lookup on the host `PATH` (e.g. `/Users/ken/.cargo/bin/cargo` here, `null` where the tool is absent). No fixture file controls it, and it is independent of the lockfile, so the normalization replaces it with a placeholder. Before doing so it checks that any binary found is the authority's tool
                - a pre-existing quirk, unrelated to this fix: `Package::configuration` puts the package's relative path in front of a path that is already repository-relative, e.g. `crates/alpha/crates/alpha/Cargo.toml` (and `pkg-a/lib/pkg-a/lib/Cargo.toml` in the CLI). `file_associations[].files` has the correct `crates/alpha/Cargo.toml`. The pre-change code (HEAD `ea73a87aa`) produces the same value, so the expectation pins it as the pre-change contract and says in a comment that it is not endorsed. Library behavior is unchanged. It should become its own fix
                - in full detection, the uv root package lists `uv.lock` in `configuration` and `file_associations` whenever the lockfile exists, including the unparseable and extra-member states. Cargo and pnpm roots are not packages, so their lockfiles appear in no package
                - the CLI's `repo structure --json` reports the canonical root (`/private/var/...` on macOS), while the library reports the root as passed in (`/var/...`)
                - no comment drift found in the touched code
        - files changed
                - `sniff/lib/tests/l1/lockfile_provenance.rs`
                        - new `Tier`, `Shape`/`shape()` (one per-authority table of fixture facts), `expected_json()`, `normalized_json()`, and `assert_complete_json()`
                        - the module doc now mentions the complete-result assertion and the pre-change check
                - `sniff/cli/tests/l1/cli.rs`: new `normalized_structure_json()` and `expected_lockfile_structure_json()`
        - tests added (no new test functions and no extra detections: each existing detection now also gets a complete-document assertion)
                - `lockfile_provenance::corroborating_requests_report_hand_written_provenance_for_every_lockfile_state`: for all 15 cells (Cargo / pnpm / uv × matching, extra member, missing member, absent, unparseable), the complete `serde_json::to_value(RepoInfo)` is compared with the hand-written document under `RepoRequest::structure().with_lockfile_provenance(true)` (structure tier) and under `detect_repo` (full tier)
                - `lockfile_provenance::declining_requests_report_manifest_provenance_and_read_no_lockfile`: the same complete-document assertion for `RepoRequest::structure()` and `detect_repo_structure`, using `globbed` and no `lockfile_match`. This shows that declining changes only the provenance fields
                - how the expectation is built: a `serde_json::json!` document from `shape()` plus each cell's `(provenance, lockfile_match)`, taken from the existing `CORROBORATED` table. It covers every top-level field, every `DetectedStandard` and `MonorepoLayer` field, and every emitted `Package` field. Full tier adds `file_associations`, `configuration`, `package_managers`, `test_runners`, and the uv root's `nested_packages`. Nothing in it is copied from detection output
                - normalization: the temp root becomes `<root>`, `\` becomes `/`, and `binary` becomes `<host PATH lookup>`. It does not sort anything, so the order of packages and layer members is asserted as reported
                - `cli::repo_structure_json_reports_lockfile_provenance_for_matching_lockfile` also asserts the complete `sniff repo structure --json` document against a hand-written expectation, with no extra spawn. The expectation includes `languages`, `file_associations`, `primary_language`, and so on. The reported root must canonicalize to the fixture root before it is replaced by `<root>`
                - why a CLI assertion even though the library one exists: the CLI runs its own request plan on a Git fixture with source files, which produces fields the library matrix does not reach (languages). Checking the whole document adds one assertion to an existing spawn, and it pins the CLI plan itself, not only the serialization
                - hand-written provenance assertions (`assert_repo`, `CORROBORATED`, `extra_lockfile_member_corroborates_cargo_but_not_pnpm_or_uv`) are kept unchanged
                - mutation check: renaming one expected key made the corroborating test fail; the change was then reverted
        - pre-change comparison: **matches**
                - method: temporary detached worktree of HEAD `ea73a87aa` under `/tmp` with its own `CARGO_TARGET_DIR`. The new fixture, expectation, and normalization were copied into its `lockfile_provenance.rs`, and a probe test ran `detect_repo_structure` (which corroborated by default at HEAD) and `detect_repo` for all 15 cells
                - result: all 30 complete documents (15 cells × structure/full) equal the expected JSON after normalization. The CLI whole-document assertion was copied into HEAD's `cli.rs` and also passes there
                - the worktree was removed with `git worktree remove --force`, its target directory was deleted, and neither tree kept scratch files
        - results
                - `just test` in `sniff/`: 2894 passed, 32 skipped, 0 failed
                - `just lint` in `sniff/`: clean
                - `cargo clippy -p sniff --all-targets -- -D warnings` and `cargo clippy -p sniff-cli --all-targets -- -D warnings`: clean
                - `rustfmt --check --edition 2024` on `lockfile_provenance.rs`: clean. On `cli.rs`, the new code is clean, but standalone rustfmt reports pre-existing diffs elsewhere in the file (HEAD's copy shows 34), which were left alone
                - not verified on Windows or Linux from this host. The normalization handles `\` separators and the CLI root spelling by canonicalizing both sides
        - cross-OS evidence (orchestrator; `just cross-check` ships the uncommitted local tree)
                - `just cross-check sniff --os linux`: 2011 passed, 30 skipped
                - `just cross-check sniff --os windows` (native Windows): 2000 passed, 23 skipped, including the complete-document assertions with their `\\` → `/` and temp-root normalization
                - WSL2 was not run locally; CI's nightly schedule covers it
        - out-of-scope defect surfaced (not fixed here): `Package::configuration` doubles a package's relative path (for example `crates/alpha/crates/alpha/Cargo.toml`), and HEAD `ea73a87aa` produces the same value. The expected JSON pins today's value, and a comment in the test says it is not an endorsed value. This should become its own fix
- work completed for 'Complete repository results are not verified against the pre-change contract' at 15:44:45
- starting the work on 'The required isolated corroboration measurement is missing' at 15:44:45
        - 1-minute load average at start: 6.12
        - harness: a scratch `#[doc(hidden)] pub fn __lpc_bench` plus one inserted `lpc_capture(...)` line placed right before the corroboration loop in `detection.rs`, byte-identical in both trees, driven by a scratch example `lpc_corrob`
                - the process runs `detect_repo_structure(root)` once, untimed. Each timed call then gets fresh clones of the layers and seeds and a fresh `ManifestStore` copy whose lockfile caches are empty, so the lockfile read and parse are included
                - cases: `all`, `cargo`, `pnpm`, and `*-manifests-cached` (the Cargo members' `Cargo.toml` files are parsed into the copy before the timer starts, which gives the step's marginal cost to detection)
                - copies saved as `evidence/lpc_corrob-detection.patch.txt`, `evidence/lpc_corrob.rs.txt`, and `evidence/lpc_pnpm_probe.rs.txt`. The driver is `evidence/measure-corrob.py.txt` and the fixture generator is `evidence/pnpm-fixture.py.txt`
        - trees
                - baseline: a fresh detached worktree at `ea73a87aa` plus the `nested.rs` patch (sha256 `8d41071d…ef1f9d`, the same as cycle 1). It was removed with `git worktree remove --force`, and both `/tmp` target directories were deleted
                - changed: this worktree. `git diff HEAD -- sniff/lib/src` had sha256 `8d3dec4d…fe9a7d` before the harness was added, after it was removed, and after `just test`/`just lint`
        - protocol: 3 warmup rounds and 20 measured samples per case, with alternating rotated order and 3 in-process warmups per process. The 1-minute load was recorded before and after every set, with a limit of 12
                - 14 sets were measured and none was deferred. The load ranged from 4.23 to 5.27, and the readings are in every JSON and in `evidence/measure-corrob.log`
        - counter check (throwaway collector example): declined and requested structure detection both do 83 manifest parses, and requested adds only 2 lockfile reads and 2 lockfile parses. At the corroboration point, 74 member manifests are not yet parsed, and corroboration parses them first
        - headline medians for the isolated step on the checkout root, before (generic) → after (typed), with ranges in `results.md` § 4

                | Profile | All | All, manifests cached | Cargo, manifests cached | pnpm (control) |
                |---|---|---|---|---|
                | release | 9.408 → 8.702 ms (−7.5%) | 7.281 → 6.801 ms (−6.6%) | 3.082 → 2.537 ms (−17.7%, no range overlap) | 4.159 → 4.202 ms (+1.0%) |
                | debug | 65.461 → 63.920 ms (−2.4%) | 54.668 → 52.510 ms (−3.9%) | 25.423 → 23.377 ms (−8.0%) | 29.510 → 29.404 ms (−0.4%) |

                - the step's change equals the parse change from § 1 (release −0.55 vs −0.65 ms; debug −2.05 vs −2.13 ms). The isolated marginal step (6.80 / 52.5 ms) agrees with the § 2 proxy of requested minus declined (7.56 / 53.7 ms)
                - pnpm is about half of the step even on this Cargo-authoritative checkout
                - in both trees the Cargo layer reports `lockfile_match = false`, because the member `darkmatter/dmls/zed-dmls`, which the root `Cargo.toml` excludes, is still listed as a layer member. This is pre-existing, is not touched here, and should become its own fix
        - pnpm-authoritative check (changed tree only)
                - fixtures: `checkout-shape`, whose lockfile is 287,979 bytes, and `large`, a synthetic lockfile of 3,055,582 bytes with 40 members
                - release step: 4.259 ms and 44.246 ms; the lock parse alone is 4.295 ms and 44.825 ms, so the parse is the whole step
                - release detection declined → requested: 1.673 → 6.097 ms and 4.805 → 51.758 ms
                - per byte, the pnpm parse costs about 14.8 ns/byte in release, against 4.8 ns/byte for the typed Cargo parse
                - a typed probe that reads only the importer keys is 27–29% faster in release and 44–45% faster in debug
        - pnpm decision: leave pnpm parsing as it is in this fix. R6's literal threshold (≥10% of the step) is met, but it cannot fail on a pnpm-authoritative fixture
                - reasons: the cost is now paid only by opt-in (full-tier) requests; a typed serde parse keeps 71–73% of the release parse time; a large gain needs a stop-early reader, which is out of scope
                - recommended follow-up: an `_unscheduled` fix for pnpm lockfile corroboration cost
                - if the author restores the old structure default, this decision should be revisited
        - files written
                - `results.md`: new §§ 4 and 5. The "Deferred sets" line, the § 2 breakdown sentence, and "What was not measured" were updated; the isolation gap was removed and two remaining gaps were added. The cells for §§ 1–3 are unchanged
                - `evidence/environment.md`: new section "Corroboration-step campaign"
                - `evidence/summary-table.md`: regenerated with `summarize.py.txt`. The rows for §§ 1–3 are unchanged
                - `evidence/binaries-corroboration.sha256` and `evidence/measure-corrob.log`, plus 10 new JSON files in `evidence/baseline/` and 34 in `evidence/after/`, named `{profile}-{set}-{label}.json`
        - results
                - `just test` in `sniff/`: 2894 passed, 32 skipped
                - `just lint` in `sniff/`: clean
        - orchestrator check: during this cycle the branch HEAD moved from `ea73a87aa` to `4442c8234` (a merge of `main` into `fix/sniff`, made outside this session); `git diff --stat ea73a87aa HEAD -- sniff Cargo.lock` is empty, so the baseline tree and the measured corpus are unaffected
- work completed for 'The required isolated corroboration measurement is missing' at 16:02:32

### Successful Completion

The implementation of review cycle 2 has completed successfully in 33m. During this implementation all 3 review findings were evaluated to see if they could be fixed as a part of this implementation cycle: 3 were fixed, 0 were deferred (see reasons below):

- no finding was deferred, and no performance measurement was deferred: every new sample set ran with a 1-minute load between 4.23 and 5.27 on the 16-core host
- the blocked finding was resolved as the author's inline note asked: Option A is recorded as final in `spec.md` and `plan.md`, with the options, pros and cons, and the one-line reversal path in this log
- items for the author to confirm
        - pnpm parser: left unchanged. Plan rule R6's 10% threshold is met, but a typed parse keeping only `importers` recovers only 27–29% of release parse time, and the cost is now paid only by opted-in requests (see `results.md` § 5). An unscheduled follow-up fix is recommended if the author wants that saving
        - revisit the pnpm decision if the structure default is ever reversed to Option B
- pre-existing defects surfaced, out of scope and not fixed (each should become its own fix)
        - `Package::configuration` doubles a package's relative path (for example `crates/alpha/crates/alpha/Cargo.toml`); the new complete-result expectations pin the pre-change value, with a comment in the test saying it is not an endorsed value
        - on this checkout the Cargo layer reports `lockfile_match = false`: `darkmatter/dmls/zed-dmls` is in the root `Cargo.toml` `exclude` list but still appears as a layer member (same at `ea73a87aa`)
- nothing was committed; changes stay uncommitted next to other streams' edits, so staging must use explicit paths

The files changed by this cycle are:

- `sniff/lib/tests/l1/lockfile_provenance.rs`, `sniff/cli/tests/l1/cli.rs` (complete-result assertions; no library source changed)
- `sniff/fixes/2026-09-21-lockfile-provenance-cost/{spec,plan,implementation-log,results,review-2}.md`
- `sniff/fixes/2026-09-21-lockfile-provenance-cost/evidence/` (`environment.md`, `summary-table.md`, new harness copies, logs, and sample JSON)

## Implementation of Review Findings #3

- finding: "The measured pnpm lockfile cost meets the plan's parser trigger, but the parser remains unchanged" (medium)
        - implemented first: a typed `importers`-keys parse in `ManifestStore`, with a test-only `serde_yaml_ng::Value` reference and Level 1 parity tests (matching, extra, missing, malformed, and this checkout's `pnpm-lock.yaml`)
                - a mutation check (string `importers` treated as an empty mapping) made the malformed-lockfile test fail
                - one divergence was found and handled: the generic parse rejects duplicate mapping keys anywhere, so the typed parse now rejects duplicate importer keys too. It cannot detect duplicates inside the skipped `packages:`/`snapshots:` sections; a test pins that behavior
        - owner decision (2026-09-26): corroborating only Cargo, pnpm, and uv reflects this monorepo's ecosystems, not Sniff's scope, and was never approved. Corroboration is redesigned in `2026-09-26-lockfile-corroboration`
                - the parser and its tests were removed from this fix's tree and saved as that feature's `pnpm-typed-parser.patch`, which applies cleanly to the current `detection.rs`
                - `spec.md` and `plan.md` (R6 and the Wave 7 item) record the replacement acceptance decision that review 3 asked for
        - after the revert, `cargo test -p sniff --lib pnpm` passes (5 tests) and the library test count is back to its pre-parser total

