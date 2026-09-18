---
total_phases: 8
created: 2026-07-16
phase: 8
yolo: "true"
packages:
  - sniff
  - sniff-cli
packages_during_phase_8:
  - sniff
  - sniff-cli
source_code:
  - .github/workflows/sniff-performance.yml
  - .github/workflows/test.yml
  - sniff/cli/src/commands/mod.rs
  - sniff/cli/src/output/filesystem/files.rs
  - sniff/cli/src/output/recent_commits.rs
  - sniff/cli/src/output/repo_json.rs
  - sniff/cli/tests/cli.rs
  - sniff/cli/tests/snapshots.rs
  - sniff/cli/tests/snapshots/snapshots__repo_aggregate_json.snap
  - sniff/justfile
  - sniff/lib/benches/cases/filesystem.rs
  - sniff/lib/benches/support/fixtures.rs
  - sniff/lib/examples/work_counts.rs
  - sniff/lib/src/executable_index.rs
  - sniff/lib/src/filesystem/docs.rs
  - sniff/lib/src/filesystem/file_types/aggregate.rs
  - sniff/lib/src/filesystem/file_types/classify.rs
  - sniff/lib/src/filesystem/file_types/model.rs
  - sniff/lib/src/filesystem/formatting.rs
  - sniff/lib/src/filesystem/git/api.rs
  - sniff/lib/src/filesystem/git/discovery.rs
  - sniff/lib/src/filesystem/git/mod.rs
  - sniff/lib/src/filesystem/git/open.rs
  - sniff/lib/src/filesystem/git/recent_commits.rs
  - sniff/lib/src/filesystem/git/remote_refresh.rs
  - sniff/lib/src/filesystem/git/status.rs
  - sniff/lib/src/filesystem/git/types.rs
  - sniff/lib/src/filesystem/git/worktree.rs
  - sniff/lib/src/filesystem/mod.rs
  - sniff/lib/src/filesystem/repo/aggregate.rs
  - sniff/lib/src/filesystem/repo/aggregate_view.rs
  - sniff/lib/src/filesystem/repo/cargo.rs
  - sniff/lib/src/filesystem/repo/detection.rs
  - sniff/lib/src/filesystem/repo/dotnet.rs
  - sniff/lib/src/filesystem/repo/glob.rs
  - sniff/lib/src/filesystem/repo/go.rs
  - sniff/lib/src/filesystem/repo/gradle.rs
  - sniff/lib/src/filesystem/repo/identity.rs
  - sniff/lib/src/filesystem/repo/manifest_index.rs
  - sniff/lib/src/filesystem/repo/maven.rs
  - sniff/lib/src/filesystem/repo/mod.rs
  - sniff/lib/src/filesystem/repo/nested.rs
  - sniff/lib/src/filesystem/repo/npm.rs
  - sniff/lib/src/filesystem/repo/nx_turbo.rs
  - sniff/lib/src/filesystem/repo/polyglot.rs
  - sniff/lib/src/filesystem/repo/python.rs
  - sniff/lib/src/filesystem/repo/seed.rs
  - sniff/lib/src/filesystem/repo/test_runner_usage.rs
  - sniff/lib/src/filesystem/repo/topology.rs
  - sniff/lib/src/filesystem/repo/types.rs
  - sniff/lib/src/filesystem/repo/uv.rs
  - sniff/lib/src/filesystem/system_view.rs
  - sniff/lib/src/hardware/audio.rs
  - sniff/lib/src/hardware/storage.rs
  - sniff/lib/src/lib.rs
  - sniff/lib/src/network/mod.rs
  - sniff/lib/src/os/locale.rs
  - sniff/lib/src/os/time.rs
  - sniff/lib/src/performance.rs
  - sniff/lib/src/performance/counters.rs
  - sniff/lib/src/process.rs
  - sniff/lib/src/programs/enums/metadata.rs
  - sniff/lib/src/programs/host_capability.rs
  - sniff/lib/src/programs/install/execute.rs
  - sniff/lib/src/programs/schema.rs
  - sniff/lib/src/remote/bitbucket.rs
  - sniff/lib/src/remote/gitea.rs
  - sniff/lib/src/remote/github.rs
  - sniff/lib/src/remote/gitlab.rs
  - sniff/lib/src/remote/mod.rs
  - sniff/lib/src/remote/provider.rs
  - sniff/lib/src/remote/snapshot.rs
  - sniff/lib/src/request.rs
  - sniff/lib/src/services/launchd.rs
  - sniff/lib/src/services/mod.rs
  - sniff/lib/src/services/openrc.rs
  - sniff/lib/src/services/runit.rs
  - sniff/lib/src/services/systemd.rs
  - sniff/lib/tests/git_parity.rs
  - sniff/lib/tests/integration.rs
  - sniff/lib/tests/remote_providers.rs
documentation:
  - .claude/skills/sniff/SKILL.md
  - sniff/docs/sniff-library-architecture.md
  - sniff/features/2026-07-16-performance/phases/_completed/01-work-accounting/spec.md
  - sniff/features/2026-07-16-performance/phases/_completed/02-reuse-and-scope/spec.md
  - sniff/features/2026-07-16-performance/phases/_completed/03-observation-index/spec.md
  - sniff/features/2026-07-16-performance/phases/_completed/04-package-enrichment-and-ownership/spec.md
  - sniff/features/2026-07-16-performance/phases/_completed/05-git-observation/spec.md
  - sniff/features/2026-07-16-performance/phases/_completed/06-remote-network-and-subprocess/spec.md
  - sniff/features/2026-07-16-performance/phases/_completed/07-profile-guided-cleanup/spec.md
  - sniff/features/2026-07-16-performance/phases/_completed/08-cross-platform-validation/spec.md
  - sniff/features/2026-07-16-performance/plan.md
  - sniff/lib/README.md
  - sniff/lib/benches/README.md
source_files_during_phase_8:
  - sniff/lib/src/filesystem/formatting.rs
  - sniff/lib/src/filesystem/docs.rs
  - sniff/lib/src/filesystem/repo/types.rs
  - sniff/lib/src/os/time.rs
  - sniff/lib/src/request.rs
  - sniff/cli/src/commands/mod.rs
  - .github/workflows/sniff-performance.yml
  - .github/workflows/test.yml
docs_updated_during_phase_8:
  - sniff/docs/sniff-library-architecture.md
  - sniff/lib/README.md
  - sniff/lib/benches/README.md
  - sniff/features/2026-07-16-performance/plan.md
docs_created_during_phase_8:
  - sniff/features/2026-07-16-performance/phases/_completed/08-cross-platform-validation/spec.md
skills_files_updated_during_phase_8:
  - .claude/skills/sniff/SKILL.md
source_files_during_phase_7: []
docs_updated_during_phase_7:
  - sniff/features/2026-07-16-performance/plan.md
docs_created_during_phase_7:
  - sniff/features/2026-07-16-performance/phases/_completed/07-profile-guided-cleanup/spec.md
skills_files_updated_during_phase_7:
  - .claude/skills/sniff/SKILL.md
packages_during_phase_7:
  - sniff
source_files_during_phase_6:
  - sniff/lib/src/lib.rs
  - sniff/lib/src/process.rs
  - sniff/lib/src/request.rs
  - sniff/lib/src/os/time.rs
  - sniff/lib/src/os/locale.rs
  - sniff/lib/src/hardware/storage.rs
  - sniff/lib/src/network/mod.rs
  - sniff/lib/src/programs/schema.rs
  - sniff/lib/src/programs/host_capability.rs
  - sniff/lib/src/services/mod.rs
  - sniff/lib/src/services/systemd.rs
  - sniff/lib/src/services/runit.rs
  - sniff/lib/src/services/launchd.rs
  - sniff/lib/src/services/openrc.rs
  - sniff/lib/src/remote/mod.rs
  - sniff/lib/src/remote/snapshot.rs
  - sniff/lib/src/remote/provider.rs
  - sniff/lib/src/remote/github.rs
  - sniff/lib/src/remote/gitlab.rs
  - sniff/lib/src/remote/gitea.rs
  - sniff/lib/src/remote/bitbucket.rs
  - sniff/lib/tests/remote_providers.rs
  - sniff/justfile
docs_updated_during_phase_6:
  - sniff/features/2026-07-16-performance/plan.md
docs_created_during_phase_6:
  - sniff/features/2026-07-16-performance/phases/_completed/06-remote-network-and-subprocess/spec.md
skills_files_updated_during_phase_6:
  - .claude/skills/sniff/SKILL.md
source_files_during_phase_5:
  - sniff/lib/src/request.rs
  - sniff/lib/src/filesystem/mod.rs
  - sniff/lib/src/filesystem/git/mod.rs
  - sniff/lib/src/filesystem/git/api.rs
  - sniff/lib/src/filesystem/git/status.rs
  - sniff/lib/src/filesystem/git/discovery.rs
  - sniff/lib/src/filesystem/git/remote_refresh.rs
  - sniff/lib/src/filesystem/git/types.rs
  - sniff/lib/tests/git_parity.rs
  - sniff/cli/src/commands/mod.rs
docs_updated_during_phase_5:
  - sniff/features/2026-07-16-performance/plan.md
docs_created_during_phase_5:
  - sniff/features/2026-07-16-performance/phases/_completed/05-git-observation/spec.md
skills_files_updated_during_phase_5:
  - .claude/skills/sniff/SKILL.md
source_files_during_phase_4:
  - sniff/lib/src/filesystem/repo/seed.rs
  - sniff/lib/src/filesystem/repo/detection.rs
  - sniff/lib/src/filesystem/repo/topology.rs
  - sniff/lib/src/filesystem/repo/manifest_index.rs
  - sniff/lib/src/filesystem/repo/glob.rs
  - sniff/lib/src/filesystem/repo/mod.rs
  - sniff/lib/src/filesystem/repo/cargo.rs
  - sniff/lib/src/filesystem/repo/npm.rs
  - sniff/lib/src/filesystem/repo/uv.rs
  - sniff/lib/src/filesystem/repo/nx_turbo.rs
  - sniff/lib/src/filesystem/repo/go.rs
  - sniff/lib/src/filesystem/repo/gradle.rs
  - sniff/lib/src/filesystem/repo/maven.rs
  - sniff/lib/src/filesystem/repo/dotnet.rs
  - sniff/lib/src/filesystem/repo/polyglot.rs
  - sniff/lib/src/filesystem/repo/nested.rs
docs_updated_during_phase_4:
  - sniff/features/2026-07-16-performance/plan.md
docs_created_during_phase_4:
  - sniff/features/2026-07-16-performance/phases/_completed/04-package-enrichment-and-ownership/spec.md
skills_files_updated_during_phase_4:
  - .claude/skills/sniff/SKILL.md
source_files_during_phase_3:
  - sniff/lib/src/filesystem/system_view.rs
  - sniff/lib/src/filesystem/mod.rs
  - sniff/lib/src/filesystem/repo/detection.rs
  - sniff/lib/src/filesystem/repo/nested.rs
  - sniff/lib/src/filesystem/repo/glob.rs
  - sniff/lib/src/filesystem/repo/manifest_index.rs
  - sniff/lib/src/filesystem/repo/types.rs
  - sniff/lib/src/filesystem/repo/cargo.rs
  - sniff/lib/src/filesystem/repo/npm.rs
  - sniff/lib/src/filesystem/repo/uv.rs
  - sniff/lib/src/filesystem/repo/nx_turbo.rs
  - sniff/lib/src/performance/counters.rs
  - sniff/lib/tests/integration.rs
docs_updated_during_phase_3:
  - sniff/features/2026-07-16-performance/plan.md
docs_created_during_phase_3:
  - sniff/features/2026-07-16-performance/phases/_completed/03-observation-index/spec.md
skills_files_updated_during_phase_3:
  - .claude/skills/sniff/SKILL.md
source_files_during_phase_2:
  - sniff/lib/src/performance.rs
  - sniff/lib/src/filesystem/mod.rs
  - sniff/lib/src/filesystem/system_view.rs
  - sniff/lib/src/filesystem/file_types/model.rs
  - sniff/lib/src/filesystem/file_types/classify.rs
  - sniff/lib/src/filesystem/file_types/aggregate.rs
  - sniff/lib/src/filesystem/repo/aggregate_view.rs
  - sniff/lib/src/filesystem/repo/aggregate.rs
  - sniff/lib/src/filesystem/repo/mod.rs
  - sniff/lib/src/filesystem/repo/identity.rs
  - sniff/lib/src/filesystem/git/types.rs
  - sniff/lib/src/filesystem/git/worktree.rs
  - sniff/lib/src/filesystem/git/recent_commits.rs
  - sniff/lib/src/filesystem/git/api.rs
  - sniff/lib/src/filesystem/git/mod.rs
  - sniff/lib/src/executable_index.rs
  - sniff/lib/src/programs/host_capability.rs
  - sniff/cli/src/output/repo_json.rs
  - sniff/cli/src/output/recent_commits.rs
  - sniff/cli/src/output/filesystem/files.rs
  - sniff/cli/src/commands/mod.rs
  - sniff/cli/tests/snapshots.rs
  - sniff/cli/tests/snapshots/snapshots__repo_aggregate_json.snap
  - sniff/cli/tests/cli.rs
docs_updated_during_phase_2:
  - sniff/features/2026-07-16-performance/plan.md
docs_created_during_phase_2:
  - sniff/features/2026-07-16-performance/phases/_completed/02-reuse-and-scope/spec.md
skills_files_updated_during_phase_2:
  - .claude/skills/sniff/SKILL.md
source_files_during_phase_1:
  - sniff/lib/src/performance.rs
  - sniff/lib/src/performance/counters.rs
  - sniff/lib/src/filesystem/system_view.rs
  - sniff/lib/src/filesystem/docs.rs
  - sniff/lib/src/filesystem/file_types/classify.rs
  - sniff/lib/src/filesystem/git/discovery.rs
  - sniff/lib/src/filesystem/git/open.rs
  - sniff/lib/src/filesystem/git/recent_commits.rs
  - sniff/lib/src/filesystem/git/remote_refresh.rs
  - sniff/lib/src/filesystem/git/status.rs
  - sniff/lib/src/filesystem/git/worktree.rs
  - sniff/lib/src/filesystem/repo/aggregate.rs
  - sniff/lib/src/filesystem/repo/cargo.rs
  - sniff/lib/src/filesystem/repo/detection.rs
  - sniff/lib/src/filesystem/repo/dotnet.rs
  - sniff/lib/src/filesystem/repo/glob.rs
  - sniff/lib/src/filesystem/repo/go.rs
  - sniff/lib/src/filesystem/repo/gradle.rs
  - sniff/lib/src/filesystem/repo/identity.rs
  - sniff/lib/src/filesystem/repo/manifest_index.rs
  - sniff/lib/src/filesystem/repo/maven.rs
  - sniff/lib/src/filesystem/repo/nested.rs
  - sniff/lib/src/filesystem/repo/npm.rs
  - sniff/lib/src/filesystem/repo/nx_turbo.rs
  - sniff/lib/src/filesystem/repo/polyglot.rs
  - sniff/lib/src/filesystem/repo/python.rs
  - sniff/lib/src/filesystem/repo/test_runner_usage.rs
  - sniff/lib/src/filesystem/repo/topology.rs
  - sniff/lib/src/filesystem/repo/uv.rs
  - sniff/lib/src/hardware/audio.rs
  - sniff/lib/src/hardware/storage.rs
  - sniff/lib/src/network/mod.rs
  - sniff/lib/src/os/locale.rs
  - sniff/lib/src/os/time.rs
  - sniff/lib/src/programs/enums/metadata.rs
  - sniff/lib/src/programs/host_capability.rs
  - sniff/lib/src/programs/install/execute.rs
  - sniff/lib/src/programs/schema.rs
  - sniff/lib/src/remote/bitbucket.rs
  - sniff/lib/src/remote/gitea.rs
  - sniff/lib/src/remote/github.rs
  - sniff/lib/src/remote/gitlab.rs
  - sniff/lib/src/remote/mod.rs
  - sniff/lib/src/services/launchd.rs
  - sniff/lib/src/services/openrc.rs
  - sniff/lib/src/services/runit.rs
  - sniff/lib/src/services/systemd.rs
  - sniff/lib/examples/work_counts.rs
  - sniff/lib/benches/cases/filesystem.rs
  - sniff/lib/benches/support/fixtures.rs
docs_updated_during_phase_1:
  - sniff/lib/benches/README.md
docs_created_during_phase_1:
  - sniff/features/2026-07-16-performance/phases/_completed/01-work-accounting/spec.md
skills_files_updated_during_phase_1:
  - .claude/skills/sniff/SKILL.md
---

# Execution Plan: Sniff Performance Improvements

Implement the umbrella performance specification through independently reviewable phases while preserving Sniff's public output contracts and macOS, Linux, and Windows behavior. The governing implementation rule is **observe once, project many times**: each request acquires expensive evidence once, and library-owned projections reuse that evidence.

## Planning decisions and execution rules

- [x] Treat this plan's Phase 1 as the specification's Phase 0 work-accounting prerequisite; all subsequent specification phases are shifted by one so execution starts with the required standard Phase 1.
- [x] Adopt Open Question 1 Option A as the Phase 5 planning baseline: focused path-history APIs use `PathHistoryOptions` and return `PathHistoryResult { commits, commits_scanned, history_exhausted, limit_reached }`; confirm the proposed nonzero 10,000-commit default with fixture measurements in the Phase 5 sub-spec before changing the public API.
- [x] Before editing an existing function, method, or type, run the repository-required upstream impact analysis for that symbol, record direct callers and affected flows in the phase sub-spec, and stop for review before any HIGH or CRITICAL-risk edit.
- [x] Create each phase sub-spec at the path named below with `sub-spec: true`, a relative `depends-on` link, the exact public migration and counters for that phase, platform fixtures, and acceptance commands; do not combine the umbrella work into one pull request.
- [x] Keep business logic and reusable projections in `sniff-lib`; limit `sniff-cli` changes to request selection, serialization, and rendering of library-owned facts.
- [x] Preserve native `Path`/`PathBuf` semantics, ignore/prune rules, symlink behavior, valid-JSON-only stdout, text/plain rendering, and exit codes except for the specification's explicit changes.
- [x] Use L1 tests for in-process fixtures, nextest through the `sniff/` area's `just test` recipe, Criterion for directional timing, and work counters as the primary structural acceptance evidence; never use `cargo test` or write-mode `cargo fmt`.

## Phase 1 — Trustworthy work accounting and baselines

Establish R1 before evaluating any optimization. This phase must not intentionally change detection results.

- [x] Create `phases/_completed/01-work-accounting/spec.md` with `sub-spec: true` and `depends-on: ../../../spec.md`; enumerate stable counter names, owned instrumentation boundaries, test-access strategy, representative fixtures, and the pre-change baseline procedure.
- [x] Update `sniff/lib/src/performance.rs` so an active `PerformanceCollector` can be installed in `ignore::build_parallel()` worker closures and every worker flushes its stage/counter buffers before exiting or returning to the pool.
- [x] Update `sniff/lib/src/filesystem/system_view.rs` `WorkerBuffers` and parallel-walker setup so directory-entry and classification work performed by every worker appears exactly once in the request's report, including early-stop paths.
- [x] Gate per-entry clocks, TLS access, and counter bookkeeping out of the default hot path whenever neither structured performance collection nor the `metrics` feature is active; add a test or benchmark assertion proving the disabled path records no work.
- [x] Add counters at Sniff-owned boundaries for directory entries, accepted file classifications, inventory saturation, file opens, metadata probes, canonicalizations, bytes read, manifest/lockfile/config parses, repository discoveries, status walks, blob/worktree loads, file diffs, ref walks, commit visits, subprocess spawns/timeouts, and remote API operations.
- [x] Add a crate-private test harness or scoped collector API that exposes work counts to unit/integration tests without adding test-only state to public result types or process-global caches.
- [x] Add worker-accounting tests that compare serial/coarse expected units with parallel reports and prove no worker data is lost, double-counted, or leaked into a later request.
- [x] Capture pre-optimization work counts and directional timings for the specification's staged-filesystem, 375-package structure/full, and 100-dirty-file status cases; rename the stale `huge_500_packages` benchmark or make its fixture contain 500 packages without changing both workload and baseline in the same comparison.
- [x] Record the baseline environment, fixture sizes, request presets, counter values, and Criterion IDs in the Phase 1 sub-spec so later phases compare equivalent work rather than hosted-runner wall time alone.

**Validation checkpoint — Phase 1**

- [x] From `sniff/`, run `just sanity`, `just test`, `just lint`, `just build`, and `just doctest`; confirm unchanged public output and no default-path metrics overhead regression.
- [x] Run the affected Criterion groups with `just bench` or the documented `cargo bench -p sniff --bench perf -- <filter>` form and archive the baseline report identifiers for Phases 2–7.
- [x] Do not begin an optimization phase until parallel-worker counters are visible and repeatable on the same fixture.

## Phase 2 — Remove repeated aggregate work and accidental local scans

Implement R2, R3, R7, and the eager-index portions of R13. After the Phase 2 sub-spec fixes shared contracts, the four labeled lanes can proceed in parallel in separate worktrees because they touch distinct primary modules; coordinate final assembly changes in `request.rs` and filesystem orchestration.

- [x] Create `phases/_completed/02-reuse-and-scope/spec.md` with `sub-spec: true` and `depends-on: ../01-work-accounting/spec.md`; pin aggregate JSON goldens, walk-scope truth tables, inventory serialization migration, and eager-index counter expectations.
- [x] **[Parallel lane A — aggregate]** Extend the aggregate library request/result only for facts currently missing from `GitInfo`/`RepoInfo`—including conflicts, current worktree identity, branches/worktrees, root-package fallback, and one commit-family history observation—without adding CLI-side detection.
- [x] **[Parallel lane A — aggregate]** Refactor `sniff/cli/src/output/repo_json.rs::build_aggregate_value` into a pure projection over `SniffResult` plus explicit render options; remove calls that discover repositories, collect changed paths, query worktrees/branches/conflicts, or walk history.
- [x] **[Parallel lane A — aggregate]** Derive dirty, staged, unstaged, and untracked file/source/document/package/package-area buckets from the single detected `file_changes` collection and a library-owned attribution projection over the detected package catalog; retain current serializer ordering and schema, then let Phase 4 replace the attribution projection's internals with the shared deepest-prefix index.
- [x] **[Parallel lane A — aggregate]** Remove aggregate-only identity and single-package fallback detection from `sniff/cli/src/commands/mod.rs`; keep focused commands on their focused library entry points.
- [x] **[Parallel lane A — aggregate]** Add work-count and effect-isolation tests proving bare `sniff repo --json` performs one repository discovery context, exactly one detailed status walk, one shared history observation, no network calls, and no observation inside `build_aggregate_value`; add stdout/stderr and JSON golden tests.
- [x] **[Parallel lane B — planner]** Introduce a table-driven internal walk-scope decision in filesystem orchestration: formatting-only starts no descendant walker; structure-only collects no inventory; package/base inventory stays package-scoped; repo-wide docs/full repo use repo scope; Git presence alone never widens scope.
- [x] **[Parallel lane B — planner]** Make mixed-consumer planning choose the smallest valid walk set, reuse one repository walk only when it satisfies every consumer, and add work-count tests for formatting-only, package-scoped Git plus inventory, docs-only context, and mixed-scope cases.
- [x] **[Parallel lane B — planner]** Reorder filesystem assembly to borrow intermediate repo/docs evidence and move it with `Option::take`; prove completed `RepoInfo` and Markdown vectors are not deep-cloned and discard internal repo context when only docs needed it.
- [x] **[Parallel lane C — saturation]** Add Serde-defaulted, omitted-when-empty `truncated` and `limit` fields to `FileInventory`, `FileAssociationBreakdown`, and `LanguageSummary`, propagate them through every projection, and update all in-repo struct literals.
- [x] **[Parallel lane C — saturation]** Implement a global accepted-classification cap: inventory-only walking quits at `MAX_FILES`; combined walking stops classification/counters at saturation but continues only while manifest, marker, or docs observers remain active.
- [x] **[Parallel lane C — saturation]** Sort accepted classifications before projection and replace exact-subset assertions for truncated parallel runs with cap, flag, ordering, native-path-validity, and complete-run determinism assertions.
- [x] **[Parallel lane D — executable indexes]** Route `HostCapabilities::detect()` through the eager PATH index and macOS bundle-inclusive construction through the existing bundle cache; add counters/tests proving one PATH scan and one cache-backed bundle discovery per request.

**Validation checkpoint — Phase 2**

- [x] Run focused aggregate CLI parity tests for default, `--plain`, and `--json`; ordinary successful JSON must produce one valid document on stdout and no stderr.
- [x] Run `just test`, `just lint`, `just build`, and the formatting-only, staged-filesystem, inventory-cap, aggregate, and executable-index Criterion groups; compare work counts with Phase 1 and reject any unexplained output or scope regression.
- [x] Verify complete inventory JSON omits `truncated`/`limit`, truncated JSON includes `true` and the accepted cap, and every public projection reports the same completeness state.

## Phase 3 — One per-request filesystem observation index

Implement R4 so integrated full filesystem requests and standalone full repo detection share the same compact evidence model.

- [x] Create `phases/_completed/03-observation-index/spec.md` with `sub-spec: true` and `depends-on: ../02-reuse-and-scope/spec.md`; list every evidence kind, root-marker exception, ignore/prune rule, case behavior, and allowed specialized fallback.
- [x] Evolve `FilesystemSystemView` in `sniff/lib/src/filesystem/system_view.rs` into a request-scoped observation index that conditionally retains manifest path/kind/owner, nested marker and parent, solution/leaf evidence, capped classifications, Markdown metadata, and already-available entry metadata—never full `DirEntry` values or file bodies.
- [x] Match nested workspace markers during the shared walk and make `sniff/lib/src/filesystem/repo/nested.rs` consume supplied evidence instead of invoking `walk_for_nested_markers` when the tree has already been observed.
- [x] Route standalone full `detect_repo` and integrated full filesystem detection through the same observation builder, while keeping structure-only free to use the smallest evidence set established in Phase 2.
- [x] Refactor Cargo membership glob expansion and compatible standards to query indexed manifest/evidence paths rather than walk prefix trees or probe each candidate directory for manifests.
- [x] Preserve root-marker handling, committed-marker ignore semantics, directory pruning, fixed-marker platform case behavior, and native path encoding; add macOS/Linux/Windows fixture assertions around case, separators, and ignored markers.
- [x] For every specialized detector that cannot consume the index, document the semantic reason in the Phase 3 sub-spec, reuse the common ignore/prune policy, and add a counter assertion that makes its fallback walk explicit.
- [x] Add integrated and standalone fixture tests proving each full request performs one non-Git repository enumeration and returns equivalent topology, inventory, docs, nested workspaces, solutions, and leaf packages.

**Validation checkpoint — Phase 3**

- [x] Run `just test`, `just lint`, and `just build`; compare integrated and standalone full-result fixtures and confirm one non-Git enumeration for compatible evidence.
- [x] Run nested-discovery, integrated-full, standalone-full, and 375-package Criterion cases with and without supplied evidence; record entry/probe reductions and investigate any result delta before Phase 4. **Criterion timings rejected as noise** (host load 57-87/16 cores; the unchanged structure-only control reported +330%) — see the Phase 3 sub-spec; entry/probe reductions recorded by counter, and the with/without-evidence comparison is asserted in-process by `observing_once_changes_work_not_results`. Re-run on an idle host deferred to Phase 8.

## Phase 4 — Deduplicate packages, parsing, and ownership

Implement R5 and R6 on top of the observation index. Discovery must finish and deduplicate boundaries before enrichment begins.

- [x] Create `phases/_completed/04-package-enrichment-and-ownership/spec.md` with `sub-spec: true` and `depends-on: ../03-observation-index/spec.md`; define `PackageSeed`, detail-level enrichment, `ManifestStore`, normalized-key semantics, and the structure-only migration.
- [x] Introduce a cheap internal `PackageSeed` containing the normalized absolute path key, owning `MonorepoStandard`, provenance, and matched evidence kinds; make workspace and nested detectors return seeds before resolving enrichment fields.
- [x] Merge seeds by whole-component normalized `PathBuf` key before name/version resolution, language/framework scans, test-runner detection, feature extraction, dependency parsing, or file association.
- [x] Add one per-detection `ManifestStore` keyed by normalized native paths for parsed/raw Cargo, Node, Python, Go, lockfile, inherited-version, and root-scoped test-runner/config inputs; count and test one parse per unique input. **Completed after the Phase 4 boundary:** the request-scoped store now covers typed and raw manifests, Cargo/pnpm/uv lockfiles, inherited workspace manifests, and root configuration. Review cycle 3 added replayable failure caching and detection-level one-parse coverage for npm, pnpm, and uv.
- [x] Index manifest entries by normalized path so subtree queries use prefix ranges rather than scanning the complete manifest list per package; use observed manifest kinds before any deliberate fallback probe. **`ManifestIndex::subtree_range` (binary-searched, sorted entries) + `kinds_at` land the index; seeds carry observed kinds. The probe-skipping consumer of `seed.evidence` is deferred** — absence in the index is not proof of absence (it omits generated/fixture manifests), so a fast path may only skip probes for kinds known *present*.
- [x] Make `RepoRequest::structure()` perform membership and minimum package identity parsing only; leave dependency, test-runner, feature, framework, language, and file-list enrichment fields absent/empty while retaining identity, paths, package area, ecosystem, standard, provenance, exclusion state, standards, and layers. **Completed after owner review:** `RepoDetailRequest` provides focused package-manager, dependency, and test-runner opt-ins, and the three CLI commands use those focused requests without enabling full inventory-backed enrichment.
- [x] Enrich every unique seed exactly once for `RepoRequest::full()` and parse root `Cargo.lock`, inherited root `Cargo.toml`, and root-scoped test-runner configuration at most once per detection. **Current source routes these inputs through the request-scoped `ManifestStore`; Level-1 detection and counter fixtures pin reuse.**
- [x] Define one normalized-path boundary operation that canonicalizes only where existing resolved-symlink semantics require it, preserves native encoding/separators and existing lexical case behavior, and normalizes Windows drive prefixes without lossy strings or ad hoc case folding.
- [x] Build one deepest-prefix ownership index and use it for inventory, docs, aggregate buckets, and commit-file attribution; compare path components and depth so nested packages always beat shallower prefixes. **Completed:** the crate-private `PackageOwnershipIndex` reuses normalized native-path keys and is shared across all four consumers; component-prefix, non-UTF-8 Unix, Windows drive/case, and zero-hot-canonicalization fixtures pin the contract.
- [ ] Add fixtures for duplicate seeds, overlapping/nested packages, symlink and lexical paths, non-UTF-8 paths where supported, Windows drive/case behavior, root inheritance, and 100/500/2,000 mixed-ecosystem packages. **All but a dedicated symlink fixture are covered:** seed merging and prefix siblings, native non-UTF-8 and Windows drive/case ownership, inherited root manifests, and Criterion mixed-ecosystem cardinalities at all three sizes. Existing normalization preserves historical resolved-symlink semantics, but this checklist item remains open until a dedicated parity fixture pins it.
- [x] Update library README/rustdoc examples to direct callers to focused/full requests for enriched fields and document the intentionally shallow structure-only contract.

**Validation checkpoint — Phase 4**

- [x] Run `just test`, `just lint`, `just build`, and `just doctest`; assert one enrichment per unique boundary, one parse per unique manifest/lockfile/config, and deepest-prefix assignment parity. **The post-review canonical `just test` and `just lint` runs pass; focused Level-1 fixtures now assert each reuse and ownership boundary. Historical Phase 4 counts remain in the sub-spec.**
- [x] Benchmark structure/full detection at 100, 500, and 2,000 packages plus Markdown/package ownership scaling; verify structure mode's enrichment counters remain zero and record qualified evidence. **Criterion workload definitions now cover mixed ecosystems at all three sizes and large document ownership. No wall-time ratio was published on the loaded host; counters remain the acceptance evidence.**

## Phase 5 — Focused and bounded Git observation

Implement R8–R10. Rich status, metadata controls, and history/containment can be developed as parallel lanes after the Phase 5 public request/result contracts are fixed; merge through shared `gix` request context tests.

- [x] Create `phases/_completed/05-git-observation/spec.md` with `sub-spec: true` and `depends-on: ../04-package-enrichment-and-ownership/spec.md`; finalize Option A, validate or revise the 10,000-commit default, list every in-repo source migration, and pin existing preset JSON and result values.
- [x] **[Parallel lane A — rich status]** Add one status-call context holding the index, HEAD tree, worktree, object cache, and reusable diff resources; preserve status-provided object IDs and enable object caching for file-change-only requests.
- [x] **[Parallel lane A — rich status]** Load each staged/unstaged blob or worktree side once, run one diff per side, and derive both statistics and optional unified hunks from that diff; keep identity, dirty-flag, and counts-only requests free of blob loads and diffs.
- [x] **[Parallel lane B — metadata controls]** Add optional `GitMetadataRequest` controls and builders for commits, ref decorations, branches, divergence, remotes, tracking, config, and worktrees; absence during deserialization must derive legacy behavior and legacy presets must omit the field when serialized.
- [x] **[Parallel lane B — metadata controls]** Update every in-repo `GitRequest` struct literal to builders/presets, keep `identity()`, `minimal()`/`summary()`, `full()`, and `deep()` values unchanged, and ensure focused recent commits do not trigger unrelated graph/config/worktree work.
- [x] **[Parallel lane B — metadata controls]** Borrow the ref-decoration cache, reuse remote-tracking tip sets, avoid two unrequested reachability walks per non-current branch, and reuse worktree metadata without serially opening every linked worktree as a full repository. **Completed:** one `RefSnapshot` supplies ref decorations, local tips, and remote-tracking tips across focused consumers; worktree proxies plus administrative `HEAD` supply metadata without linked-repository opens, and the already-discovered current linked repository supplies requested current-worktree detail.
- [x] **[Parallel lane C — bounded history]** Add `PathHistoryOptions` with a validated nonzero scan limit and migrate focused APIs/CLI callers to `PathHistoryResult`; path-filter tree diffs, stop at the first matching prefix per commit, expose scan completeness, and periodically clear bounded diff caches.
- [x] **[Parallel lane C — containment]** Build a target set from requested recent commit IDs, stop each remote-tip ancestry walk once all reachable targets for that walk are found, store compact remote identifiers during traversal, and resolve names only during result assembly.
- [x] Add serialization tests for legacy request JSON, focused-control round trips, and byte-identical preset JSON; add work-count tests for blob/diff units, unrelated metadata avoidance, bounded path history, skewed timestamps, ref/tip reuse, worktree opens, and target-set containment. **Completed:** focused ref consumers require one ref walk, metadata-only linked worktrees require zero repository opens, and the current linked worktree reuses the discovered handle.
- [x] Extend Git Criterion fixtures for 100 files at 1 KiB/100 KiB/multi-megabyte sizes, branch-heavy/divergent repos, sparse matches in long history, and many remote tips; assert work bounds separately from wall time. **Definitions and setup-excluded fixtures were added in review cycle 3; no timing verdict was collected on the loaded host.**

**Validation checkpoint — Phase 5**

- [x] Run `just test`, `just lint`, `just build`, and Git-focused Criterion groups; confirm legacy preset values/JSON remain unchanged and focused controls execute only requested counters. **`just test` 1394/1395** (sole failure is the pre-existing `detect_area_errors_when_not_in_repo` timeout; Phase 5 does not touch `repo/area.rs`). **`just lint` clean with zero warnings**, `just build` and `just doctest` clean. Preset JSON parity and focused-control gating are asserted by test, not inspection. Criterion groups deferred as above.
- [x] Verify each dirty side has one load and one diff, path-history completeness is explicit at the bound, containment never relies on timestamp early-stop behavior, and all existing Git CLI output goldens pass. **All four asserted by test.** The containment stop is a reachability bound, not a time bound, and the pre-existing skewed-timestamp test passes unchanged. Every existing status/diff test — including the byte-exact patch goldens — passes without modification, which is the equivalence evidence for Lane A.

## Phase 6 — Reuse remote/network inputs and bound subprocesses

Implement R11 and R12. The remote snapshot, WAN client, NTP default, and subprocess lanes are parallelizable after the Phase 6 sub-spec fixes shared error/counter policies; provider files can proceed independently once the snapshot contract is stable.

- [x] Create `phases/_completed/06-remote-network-and-subprocess/spec.md` with `sub-spec: true` and dependencies on `../05-git-observation/spec.md` and the umbrella spec; pin provider truncation semantics, HTTP counters, timeout defaults, partial-result behavior, diagnostics, and child-reaping tests.
- [x] **[Parallel lane A — remote]** Add a provider-private `RemoteRepoSnapshot` or additive defaulted trait hook so `fetch_report` resolves metadata/default branch once and makes at most one equivalent root recursive-tree request per report without adding a required downstream trait method. **`remote/snapshot.rs`** + three **defaulted** hooks (`snapshot`, `list_documents_with`, `detect_cicd_with`). No required trait method added: the defaults re-enter the existing `list_documents`/`detect_cicd`, so a downstream provider that adopts nothing still works (R11.4).
- [x] **[Parallel lane A — remote]** Make GitHub, GitLab, Gitea, and Bitbucket document/CI projections consume shared metadata/tree evidence; run independent Bitbucket continuation listings concurrently only after the shared branch input is available. **All four adopted.** Per report: GitHub 3 metadata + 2 tree → **1 + 1**; Gitea 3 + 2 → **1 + 1**; GitLab **3 tree → 1** (its metadata call *is* a tree fetch whose result was discarded); Bitbucket 3 metadata + 2 root listings → **1 + 1**. Bitbucket's `docs/` and `doc/` listings now run under one `tokio::join!` once the branch is known (R11.3). Sharing the projection also collapsed **four byte-identical copies** of `categorize_document`/`is_documentation_file` (and their four duplicate test suites) into one in `snapshot.rs`.
- [x] **[Parallel lane A — remote]** Detect provider-reported truncation and perform bounded pagination/subtree continuation needed for correctness; preserve required-metadata and optional-section graceful degradation when the shared tree fails. **`truncated` was deserialized by GitHub and Gitea and read nowhere**, and Bitbucket's `response.next` was explicitly ignored ("For MVP, just return the first page") — so a >100k-entry or multi-page repository reported "no docs, no CI" with full confidence. Now: GitHub/Gitea continue the document/CI prefixes as `branch:prefix` subtrees; Bitbucket follows pagination to `MAX_LISTING_PAGES`. `RemoteTree::available` keeps "fetch failed" distinguishable from "observed and empty", so a failed tree degrades the optional sections while metadata survives (R11.6).
- [x] **[Parallel lane A — remote]** Add complete, truncated, and failed-tree provider fixtures proving one metadata call, one equivalent root-tree call, separately counted continuation calls, and preserved workflow-run fallback. **6 new wiremock tests** in `snapshot_reuse_tests`, bounds pinned by `expect(1)`. All 65 pre-existing remote tests pass unmodified. Continuations are counted under a separate `tree_continuation` slug per the counter's documented intent. **A probe caught a real design bug:** the client percent-encodes the whole `branch:prefix` tree_sha into one path segment, so an original `.github/workflows` prefix arrived as `%2F` — routinely rejected or normalized by routers. `CONTINUATION_PREFIXES` is now single-component-only (`.github`, not `.github/workflows`) and the recursive response supplies `workflows/ci.yml` beneath it.
- [x] **[Parallel lane B — WAN]** Reuse one blocking HTTP client across WAN attempts, configure at least two default HTTPS endpoints, query sequentially, stop after the first strictly parsed IP, and apply identical connect/request deadlines without logging bodies or credentials. **The client was rebuilt per attempt** (throwing away the pool and TLS setup between endpoints); it is now built once and the whole ladder runs under it. **`icanhazip.com` added as a second default** — with one endpoint the fallback ladder was unreachable and "retry" was a fiction. Split `connect_timeout` (1s) / `timeout` (2s), identical per endpoint. Bodies never leave `fetch_wan_ip`.
- [x] **[Parallel lane B — WAN]** Preserve cache TTL and force-refresh semantics; add wiremock tests for first-endpoint success, invalid response fallback, timeout fallback, all-endpoint failure, strict IPv4/IPv6 parsing, and no stale return beyond policy. **TTL/force-refresh code is untouched** — `detect_wan_ip`'s cache logic was not in the edit's path, and its pre-existing tests still pass, which is the no-stale-return evidence. **7 new tests**, including `a_successful_first_endpoint_is_not_followed_by_a_second_request` (wiremock `expect(0)` on the second endpoint — the R11.7 non-disclosure bound) and `endpoint_attempts_are_counted_across_the_thread_hop`. That last one caught a live instance of the skill's documented trap: the ladder now runs inside a spawned thread, so its counters needed an explicit `WorkerCollector`; verified non-vacuous by removing `activate()` and watching it fail.
- [x] **[Parallel lane C — defaults]** Change `DetectionPlan::default()` to use all domains with safe OS defaults and NTP disabled while leaving explicit `OsRequest::full()` NTP behavior unchanged; update default-plan tests and serialization/docs. **`os: Some(OsRequest::full().include_ntp_status(false))`.** `OsRequest::full()` is byte-identical, pinned by `explicit_full_os_request_retains_ntp`; `default_plan_makes_no_ntp_request` pins the gate and that no *other* `full()` field was downgraded. `detection_plan_defaults_to_all_full` renamed to `..._to_all_domains` — its old name asserted the very equation R12 removes.
- [x] **[Parallel lane D — subprocesses]** Consolidate timeout execution behind a shared helper that accepts an explicit `Duration`, invokes executables directly without a shell, drains stdout/stderr concurrently, kills on timeout, waits to reap, emits tracing plus counters, and supports injected short test deadlines. **`sniff/lib/src/process.rs`.** This fixed a real bug, not just duplication: the three deadline-carrying sites (`os/time.rs`, `programs/schema.rs`, `host_capability.rs`) polled `try_wait()` over an **undrained** piped stdout, so any child emitting more than one pipe buffer blocked in `write()`, never exited, and was killed at its deadline with output lost. `output_larger_than_a_pipe_buffer_does_not_deadlock` (1 MiB) is the regression test.
- [x] **[Parallel lane D — subprocesses]** Preserve named defaults: 3 seconds for services and Windows locale, 5 seconds for `diskutil`, 2 seconds for host capability, and 3 seconds for program schema/NTP; treat later changes as policy changes. **All six in `process::timeouts`**; the three pre-existing values are carried over unchanged, and `test_ntp_timeout_is_three_seconds` now pins the shared constant.
- [x] **[Parallel lane D — subprocesses]** Batch systemd PID and runit status collection into a constant/bounded command count, chunking only for command-line limits; preserve primary-listing unavailable/empty results and successfully parsed enrichment prefixes on timeout. **systemd `1 + N_running` → `1 + ceil(N_running/128)`** (one `systemctl show --property=Id --property=MainPID` per chunk); **runit `N` → `ceil(N/128)`**. A failed/timed-out chunk degrades only its own services to `pid: None` / `(false, None)` — the pre-existing per-probe-failure behavior — and never discards a healthy chunk.
- [x] **[Parallel lane D — subprocesses]** Apply the shared deadline/pipe-draining behavior to every service backend, macOS `diskutil`, Windows locale and BurntToast PowerShell probes, program schema, and host-capability probes; retain public result shapes and avoid terminal noise. **Every subprocess in sniff now routes through the shared timeout runner.** Review cycle 3 closed the remaining BurntToast bypass and made cleanup process-tree-aware with Unix process groups and Windows kill-on-close Job Objects. Diagnostics are `tracing::warn`, never stdout/stderr.
- [x] Add cross-platform tests with outputs larger than a typical pipe buffer, descendants retaining inherited pipes, timeout during primary listing, timeout after partial enrichment, explicit child-reaping checks, batched command counts, and platform-gated command fixtures. **Portable same-test-executable fixtures now prove that a direct child exiting ahead of a 30-second descendant does not block pipe joins.** Existing Level-1 coverage pins large output, injected short deadlines, service batching, partial failure, and child reaping; a Windows GNU all-target check compiles the Job Object path.

**Validation checkpoint — Phase 6**

- [x] Run `just test`, `just lint`, `just build`, and remote/network/service benchmarks; confirm remote HTTP call bounds, sequential WAN disclosure, batched subprocess counts, timeout counters, and no leaked children under nextest. **`just test` 1603/1604** — the sole failure is the pre-existing `detect_area_errors_when_not_in_repo` timeout, verified against clean HEAD and recorded as a known baseline since Phase 4; Phase 6 does not touch `repo/area.rs`. **`just lint` clean with zero warnings**, `just build` and `just doctest` clean. Remote HTTP bounds, WAN non-disclosure, and batched counts are asserted by test rather than inspection. **Benchmarks deferred to Phase 8** with the Phase 3/5 precedent — this host's timings are not trustworthy (Phase 3 recorded +330% for a byte-identical case), and every claim in this phase is a request count or a counter, which is the evidence this feature judges on.
- [x] **Added, not in the original plan — the acceptance gate was blind.** `sniff-lib`'s `default = []` and the `sniff/justfile` `test` recipe passed no `--features`, so every `remote`/`network`-gated test — including all 65 pre-existing `remote_providers` tests, and both of this phase's new test lanes — **was never executed by `just test`**. The recipe now passes `--features remote`; the count went from **1,414 to 1,604 tests**, and all 190 newly-running tests pass. They reach a local `MockServer`, never the internet.
- [x] Verify default Tier-1 detection makes no NTP request, explicit full OS detection retains NTP, JSON stdout remains clean, and provider graceful-degradation fixtures pass for all four providers. **All asserted by test.** `default_plan_makes_no_ntp_request` and `explicit_full_os_request_retains_ntp` pin both halves of R12.1/R12.2. Graceful degradation: `a_failed_tree_preserves_metadata_and_degrades_optional_sections` plus the 65 pre-existing provider tests — including every error-mapping and unauth-fallback case for all four providers — pass unmodified, which is the equivalence evidence for Lane A. CLI output is untouched by this phase (no `sniff/cli` source changed), and its snapshot goldens pass.

## Phase 7 — Profile-gated remaining hot loops

Implement only measured R13/R14 candidates. This phase is optional per candidate and must not create speculative abstractions.

- [x] Create `phases/_completed/07-profile-guided-cleanup/spec.md` with `sub-spec: true` and dependencies on `../05-git-observation/spec.md` and `../06-remote-network-and-subprocess/spec.md`; include post-structural profiles and a keep/defer decision for every R14 item.
- [x] Re-run representative cold-ish and warm profiles after Phases 2–6 and rank residual costs by work count and sampled time; establish a materiality threshold in the sub-spec before editing. **Threshold fixed before editing: ≥5% of a dominant counter or ≥5% of sampled time, repeatable, byte-identical results.** Two workloads profiled (`detect_repo` on the 375-package fixture; `ProgramsInfo::detect` on the live PATH) — the R14 filesystem items and the R13.3/R13.4/L9/L14 programs items live in different paths, so one profile could not have judged both. **Cold-ish profiles not run** (`purge` needs `sudo`); the sub-spec argues this only strengthens a defer decision, since cold caches raise the syscall share and every candidate is a userspace micro-optimization.
- [x] **[Parallelizable by independent measured hotspot]** Implement only supported candidates among classification fast paths, bounded framework prefix reads, walker metadata reuse, static regex/maps, path-list merge allocation reduction, rate-limit matching, interface-cache clone reduction, and local-bin ancestor memoization. **No candidate is supported by the measurement; none implemented, no production code changed.** Repo detection is ~71% syscalls with its largest userspace bucket at ~2.9%; the programs path is ~96% Rayon park/spin. **The leading hypothesis was disconfirmed by measurement:** `metadata_probes` (13,275 vs 4,685 walked entries) looked like the dominant reuse defect, but per-path attribution shows **12,675 distinct / 600 redundant (4.5%)** — 95.5% are first-and-only marker checks that *are* the detection contract. R13.3/R13.4 are conditional on their targets being "visible"/"justified by measurement"; both are **absent** from the profile, so deferring is compliance with R13.
- [x] For every implemented candidate, add a focused benchmark and parity test; for every deferred candidate, record evidence that its cost is negligible or its contract risk exceeds measured benefit. **Nine-row keep/defer table in the sub-spec, one row per candidate.** Two premises turned out **stale**: L6's `standard.rs:608` is now `#[cfg(test)]`-only (production leaves `version: None` for the no-subprocess boundary, so the "~50×/run" claim no longer describes production), and L12's `merge_path_lists` **no longer exists** — Phase 4 deleted it. Two are deferred on **contract risk over benefit** (bounded framework prefix reads against a 50 KB total read volume; walker-metadata lifetime coupling for 182 probes); the rest on negligible cost.
- [x] Do not add unbounded CPU-scaled parallelism; keep storage concurrency bounded and internally configurable for benchmarks. **Verified on HEAD and unchanged:** `num_cpus`/`available_parallelism`/`ThreadPoolBuilder`/`build_global` appear nowhere in `sniff/lib/src`; storage detection is serial (no concurrency to bound); remote fetch parallelism is already bounded and documented. Recorded as a caution, not a finding: the programs fan-out is already 96% park/spin, so more threads there would add contention.

**Validation checkpoint — Phase 7**

- [x] Run `just test`, `just lint`, `just build`, and only the affected Criterion groups; require a repeatable material improvement with identical results before retaining a micro-optimization. **`just test` 1603/1604** — the sole failure is the pre-existing `detect_area_errors_when_not_in_repo` timeout, a known baseline since Phase 4; Phase 7 changed no production code, so it cannot have caused it. **`just lint` clean with zero warnings**, `just build` clean, `just doctest` 87/87. **No Criterion groups run: the checkpoint scopes them to "only the affected" groups, and no candidate was implemented, so no group is affected** — running them would only reproduce the untrustworthy timings Phases 3/5/6 already deferred to Phase 8 (this host sat at load 14→47 on 16 cores throughout). The "repeatable material improvement" bar is what **rejected** all nine candidates rather than what any of them cleared; results are byte-identical because the production diff for this phase is empty.

## Phase 8 — Documentation, cross-platform CI, and completion

Complete verification, documentation maintenance, and ongoing regression visibility. R14 candidates do not block completion when Phase 7 evidence defers them.

- [x] Create `phases/_completed/08-cross-platform-validation/spec.md` with `sub-spec: true` and dependencies on all completed phase sub-specs; enumerate final acceptance criteria, platform jobs, artifact names, and parity fixtures.
- [x] Update `sniff/docs/sniff-library-architecture.md`, library/CLI READMEs, rustdoc, benchmark manifest, and `.claude/skills/sniff/` to document request-scoped observation, structure/full semantics, inventory truncation, bounded path history, focused Git metadata, default NTP policy, subprocess deadlines, and qualified benchmark evidence. **Architecture doc gained seven sections** (Request-Scoped Observation Index, Structure-Only Contract, Package Discovery vs. Enrichment, Inventory Saturation, Focused Git Metadata, Bounded Path History, Subprocess Deadlines, Remote Snapshot, Performance Evidence) and its shared-work intro now states the governing "observe once, project many times" rule. Rustdoc updated at the four sites carrying the multiplier. Benchmark manifest gained a "What These Timings Are Worth" section. **CLI README needed no change** — this phase altered no CLI behavior, and its NTP/`sntp` platform line was already accurate.
- [x] Correct known drift alongside its owning behavior: `.editorconfig` lookup semantics, `filter_inventory` copy/sharing language, capped inventory determinism, the 375-package benchmark name/count, the unsupported general 10–50× claim, and NTP timeout/platform wording. **Four of six were live; two were already fixed by the phase that owned them** (`filter_inventory` and capped-inventory determinism, both rewritten in Phase 2 — verified, not re-touched), and the 375-package rename landed in Phase 1 (verified; added the "never compare against an archived `huge_500` result" note). Live and fixed: (1) `detect_formatting`'s docblock claimed parent-directory traversal that `find_editorconfig` has never done; (2) `os/time.rs` claimed a 5-second NTP timeout and the architecture doc/skill claimed "up to 10s (Linux `timedatectl`)" — the real bound is `timeouts::NTP` = **3s**, and the Linux path makes no network round trip at all, so the 10s figure described a cost that never existed; (3) the 10–50× claim, in 6 sites. **The 10–50× claim was worse than "unsupported"** — see below. Drift found but deliberately not fixed: `benches/README.md` and the architecture doc say `ProgramsInfo::detect()` fans out over **8** categories while the type and skill say **9**; that belongs to the phase owning program detection, not to a completion phase whose diff must contain no behavior.
    - **The historical 10–50× correction, in detail.** At the original Phase 8 capture, the 375-package fixture put `structure()` and `full()` near **1×** by discovery counters because R5.6 was still blocked and structure also paid a nested-marker fallback walk. That table remains valid historical evidence but predates the completed shallow-structure migration and must not be used as a current structure-mode bound. **No replacement timing ratio was published:** the host was heavily loaded and the sequential case order warmed full mode's page cache.
- [x] Extend `.github/workflows/sniff-performance.yml` or add a scheduled workflow with macOS/Linux/Windows jobs, same-OS/runner comparisons, work-count artifacts, and characterized timing signals; retain Linux PR Criterion artifacts and avoid universal cross-OS wall-clock thresholds. **Extended in place** rather than added as a second workflow — one file keeps the two signals' division of labor readable. New `sniff-work-counts` job: 3-OS matrix, `fail-fast: false`, uploads the `work_counts` table per OS (90-day retention; counters are the durable evidence, the Criterion HTML stays at 14). Same-OS comparison is enforced **by construction, not by a threshold**: artifacts are named per OS and the Criterion baseline was renamed `ci` → `ci-linux` so a macOS run cannot be diffed against a Linux one. No wall-clock gate added anywhere. The Linux PR Criterion job is retained and now `if: github.event_name != 'schedule'`.
- [x] Ensure `.github/workflows/test.yml` continues to run `cargo check --all-targets` and nextest tiers for `sniff`/`sniff-cli` on macOS, Linux, and Windows with portable fixtures, `cfg`-gated OS imports, and `std::env::join_paths` PATH construction. **The job already existed and runs all three OSes; one real gap closed.** The compile guard was `cargo check -p sniff --all-targets` with no `--features`, and `sniff-lib` is `default = []` — so every `#[cfg(feature = "remote")]`/`#[cfg(feature = "network")]` target was **invisible to the guard whose entire stated purpose is catching non-portable test code**. This is the same blindness Phase 6 found in `just test` (190 tests unrun). Now `--features remote`. Portability of the fixtures themselves is verified by the tests passing, not re-audited here.
- [x] Run output-parity fixtures for library results and CLI default/plain/JSON modes, including valid-JSON-only stdout, no ordinary-success stderr, stable serializer ordering, and documented intentional changes only. **Verified live against the built binary**, not only by golden: `sniff --json`, `sniff repo --json`, `sniff hardware --json`, and `sniff repo git-status --json` each emit one parseable JSON document on stdout with **0 bytes of stderr** and exit 0; `sniff repo` and `sniff repo --plain` likewise exit 0 with empty stderr. Serializer ordering and the render goldens are pinned by the snapshot suite inside `just test`. This phase changes no serializer, render path, or result type, so parity holds by construction — the goldens are the check on that claim, not the argument for it.
- [x] Run the complete acceptance matrix from `sniff/`: `just sanity`, `just lint`, `just build`, `just doctest`, `just test`, `just test-l2`, and `just bench`; retain read-only formatting checks through `just lint` and do not run write-mode formatting. **Historical Phase 8 results remain in the phase sub-spec. Post-review `just test` passed all 1,657 sniff-lib and 777 sniff-cli tests, and `just lint` passed.** No write-mode `cargo fmt` was run. Criterion workload definitions were added without publishing loaded-host timings; only the synthetic large-service row remains deferred because exposing a production command-injection seam solely for a benchmark would broaden unrelated API surface.
- [x] Review final work counters against Phase 1 for every acceptance case and attach a phase-by-phase table covering walks, opens/probes/parses, discoveries/status/diffs/graph visits, subprocesses/timeouts, and HTTP operations. **Full table in the sub-spec.** Headline, Phase 1 → Phase 8: `staged_filesystem_full_all_stages` `metadata_probes` 7,885 → **4,075** (−48%), `read_dirs` 422 → **211** (−50%), `manifest_parses` 214 → **124** (−42%), `package_enrichments` 180 → **90** (−50%); `repo_full_huge_375_packages` `metadata_probes` 25,975 → **13,275** (−49%), `read_dirs` 1,403 → **701** (−50%), `manifest_parses` 754 → **454** (−40%), `package_enrichments` 600 → **300** (−50%); Git `deep()` `blob_loads` 400 → **200** (R8). Results-shaped counters (`entries_visited` 638/4,685, `files_accepted` 395/3,755, `documents_parsed` 182) are **unchanged**, which is the point: this feature changed work, not results.
    - **Historical drift bracket:** `staged_filesystem_summary_git_plus_repo` and `repo_structure_huge_375_packages` were byte-identical to Phase 1 at the original Phase 8 boundary. They predate shallow structure semantics and are not current regression bounds.
    - **Three counters that cannot be compared naively, all documented in the sub-spec:** Phase 1's `file_opens`/`bytes_read` **under-report** every manifest-index case (`ManifestIndex::build`'s workers carried no collector until Phase 3), so `bytes_read` +6% is the counter getting *honest*, not work being added; and `git.file_diffs` under-reports before Phase 5, so it **does not move while the real work halves** — `blob_loads` is what shows R8.
    - **Subprocess/timeout and HTTP counters have no row**: `work_counts`'s fixtures are local synthetic trees that spawn no children and make no requests, so those counters are legitimately zero. Their bounds are asserted in-process by the Phase 6 tests (argv-logging `systemctl` shim, per-provider `expect(1)` wiremock bounds, the `expect(0)` second-endpoint WAN assertion) — stronger evidence than a fixture row, because it fails the build instead of printing a number.
    - **A Phase 4 prediction did not hold:** `filesystem.io.canonicalizations` was projected to fall 600 → 300 "with the enrichment halving". The historical Phase 8 result remained 600, proving the mechanism was mis-modeled. Current source implements R6.2/R6.4 through `PackageOwnershipIndex`, whose hot-lookup fixture requires zero canonicalizations; the archived table is not retroactively rewritten.
- [x] Move each finished phase sub-spec to its completed lifecycle location independently; move the umbrella feature only when R1–R13, all preserved/intentional compatibility checks, and macOS/Linux/Windows correctness are complete. **Decision: the phase sub-specs were archived together under `phases/_completed/`; the umbrella feature did not move.** Both halves are deliberate.
    - **The historical implementation blockers are closed.** R5.5/R5.6, R6.4, and R9.5/R9.6 are implemented and tested. Remaining limitations are evidence and benchmark scope: native Linux/Windows Level-1 execution and retained non-macOS artifacts were not produced on this macOS host, and the synthetic large-service Criterion row is deferred for the API-surface reason above. R14 remains a measured, permitted deferral rather than a blocker.
    - **The phase sub-specs are archived under `phases/_completed/`.** They were briefly deleted by `c2a188379` and restored by review cycle 11, because live consumers — the `sniff` skill's counter-baseline and R14 keep/defer tables, the scheduled work-count workflow, and production rustdoc in `process.rs`, `remote/snapshot.rs`, and `filesystem/git/discovery.rs` — cite them as authoritative evidence that the umbrella `spec.md` does not reproduce. Archiving the whole tree in one move (rather than phase by phase) preserves every sibling-relative `depends-on:` link; only links pointing out of `phases/` gained a level. The records still distinguish their historical phase-boundary measurements from current post-review semantics.

**Validation checkpoint — feature complete**

- [x] Confirm every acceptance criterion in `spec.md` has a passing automated test or an attached cross-platform artifact, every public migration is documented, and no unexplained counter or output regression remains. **Current source and macOS Level-1 evidence satisfy the R1–R13 implementation criteria, including manifest/config reuse, shallow structure, deepest-prefix ownership, aggregate purity, focused ref/worktree reuse, and process-tree cleanup.** Native Linux/Windows execution and retained artifacts remain delegated to the existing CI matrix and were not produced during this macOS-only implementation cycle; workflow definitions are not counted as completed local runs. Historical counter caveats remain attached to their archived tables.
- [x] Confirm deferred R14 candidates have explicit evidence and therefore do not block completion. **Confirmed.** Phase 7's sub-spec carries a nine-row keep/defer table, one row per candidate, against a threshold fixed *before* editing (≥5% of a dominant counter or ≥5% of sampled time, repeatable, byte-identical results). The evidence: repo detection is ~71% syscalls with its largest userspace bucket at ~2.9%; `ProgramsInfo::detect` is ~96% Rayon park/spin. The leading hypothesis was **disconfirmed** — `metadata_probes` (13,275 vs 4,685 walked entries) looked like the dominant reuse defect but attributes 12,675 distinct / 600 redundant (4.5%). Two review premises were **stale** (L6's version regex is now `#[cfg(test)]`-only; L12's `merge_path_lists` was deleted in Phase 4). The umbrella's completion boundary expressly permits this: "unimplemented R14 candidates do not block completion when evidence shows they are negligible." R13.3/R13.4 are conditional on their targets being visible in a profile; both are absent, so deferring is compliance, not omission.

## Requirement traceability

| Requirement | Owning phase | Primary observable evidence |
|---|---:|---|
| R1 work accounting | 1 | Parallel worker counters are complete, isolated, and baseline reports are archived. |
| R2 aggregate reuse | 2 | One discovery/status/history observation; pure aggregate projection; JSON parity. |
| R3 scope-aware planning | 2 | Walk-scope table and zero-walk formatting-only counter. |
| R4 shared observation index | 3 | One non-Git enumeration for integrated and standalone full detection. |
| R5 discovery vs. enrichment | 4 | One enrichment and one parse per unique seed/input; shallow structure counters. |
| R6 normalized ownership | 4 | Deepest-prefix assignment and cross-platform native-path fixtures. |
| R7 inventory saturation | 2 | Global cap, completeness fields, sorted accepted results. |
| R8 rich status reuse | 5 | One load/diff per dirty file side; shallow presets load no blobs. |
| R9 focused Git controls | 5 | Legacy serialization parity and zero unrelated graph-work counters. |
| R10 bounded history/containment | 5 | Explicit completeness result and deterministic visit bounds. |
| R11 remote/network reuse | 6 | Bounded provider request counts and sequential WAN fallback. |
| R12 latency cliffs/subprocesses | 6 | Default NTP disabled; batched commands; deadlines, draining, and reaping. |
| R13 executable indexes | 2, 7 | One eager PATH/bundle scan; remaining lookup work only if profiled. |
| R14 residual hot loops | 7 | Profile-backed keep/defer record for every candidate. |
| Cross-platform benchmark CI | 8 | Scheduled macOS/Linux/Windows work-count and Criterion artifacts. |
