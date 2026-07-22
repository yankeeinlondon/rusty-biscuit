---
total_phases: 8
source_files_during_phase_1:
  - biscuit-speaks/cli/src/install_ui.rs
  - biscuit-speaks/cli/src/main.rs
  - playa/cli/src/install_ui.rs
  - playa/cli/src/main.rs
  - sniff/cli/src/args/mod.rs
  - sniff/cli/src/args/repo.rs
  - sniff/cli/src/commands/mod.rs
  - sniff/cli/src/commands/repo.rs
  - sniff/cli/src/install_plan_cmd.rs
  - sniff/cli/src/install_ui.rs
  - sniff/cli/src/output/filesystem/files.rs
  - sniff/cli/src/output/filesystem/mod.rs
  - sniff/cli/src/output/recent_commits.rs
  - sniff/cli/src/output/repo_json.rs
  - sniff/cli/src/perf.rs
  - sniff/cli/tests/cli.rs
  - sniff/cli/tests/snapshots.rs
  - sniff/lib/benches/cases/filesystem.rs
  - sniff/lib/benches/cases/workload_matrix.rs
  - sniff/lib/benches/perf.rs
  - sniff/lib/benches/support/bench_ids.rs
  - sniff/lib/benches/support/builder.rs
  - sniff/lib/benches/support/fixtures.rs
  - sniff/lib/benches/support/remote_report_fixture.rs
  - sniff/lib/examples/work_counts.rs
  - sniff/lib/src/error.rs
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
  - sniff/lib/src/filesystem/repo/area.rs
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
  - sniff/lib/src/filesystem/repo/ownership.rs
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
  - sniff/lib/src/programs/install/command.rs
  - sniff/lib/src/programs/install/execute.rs
  - sniff/lib/src/programs/install/interview.rs
  - sniff/lib/src/programs/install/mod.rs
  - sniff/lib/src/programs/install/options.rs
  - sniff/lib/src/programs/mod.rs
  - sniff/lib/src/programs/schema.rs
  - sniff/lib/src/remote/bitbucket.rs
  - sniff/lib/src/remote/gitea.rs
  - sniff/lib/src/remote/github.rs
  - sniff/lib/src/remote/gitlab.rs
  - sniff/lib/src/remote/mod.rs
  - sniff/lib/src/remote/provider.rs
  - sniff/lib/src/remote/snapshot.rs
  - sniff/lib/src/request.rs
  - sniff/lib/src/services/benchmark.rs
  - sniff/lib/src/services/launchd.rs
  - sniff/lib/src/services/mod.rs
  - sniff/lib/src/services/openrc.rs
  - sniff/lib/src/services/runit.rs
  - sniff/lib/src/services/systemd.rs
  - sniff/lib/tests/benchmark_workloads.rs
  - sniff/lib/tests/git_parity.rs
  - sniff/lib/tests/integration.rs
  - sniff/lib/tests/remote_providers.rs
docs_updated_during_phase_1:
  - .claudine/memory/commits.md
  - CLAUDE.md
  - fixes/2026-07-22-mega-merge/plan.md
  - prompts/_implement/implement-suggestions.md
  - prompts/_reviews/feature-review.md
  - prompts/plan.md
  - sniff/README.md
  - sniff/cli/README.md
  - sniff/docs/sniff-library-architecture.md
  - sniff/lib/CHANGELOG.md
  - sniff/lib/README.md
  - sniff/lib/benches/README.md
docs_created_during_phase_1:
  - sniff/features/2026-07-16-performance/deferred-perf-tests.md
  - sniff/features/2026-07-16-performance/log.md
  - sniff/features/2026-07-16-performance/phases/_completed/01-work-accounting/log.md
  - sniff/features/2026-07-16-performance/phases/_completed/01-work-accounting/spec.md
  - sniff/features/2026-07-16-performance/phases/_completed/02-reuse-and-scope/spec.md
  - sniff/features/2026-07-16-performance/phases/_completed/03-observation-index/spec.md
  - sniff/features/2026-07-16-performance/phases/_completed/04-package-enrichment-and-ownership/spec.md
  - sniff/features/2026-07-16-performance/phases/_completed/05-git-observation/spec.md
  - sniff/features/2026-07-16-performance/phases/_completed/06-remote-network-and-subprocess/spec.md
  - sniff/features/2026-07-16-performance/phases/_completed/07-profile-guided-cleanup/spec.md
  - sniff/features/2026-07-16-performance/phases/_completed/08-cross-platform-validation/spec.md
  - sniff/features/2026-07-16-performance/plan.md
  - sniff/features/2026-07-16-performance/review-1.md
  - sniff/features/2026-07-16-performance/review-10.md
  - sniff/features/2026-07-16-performance/review-11.md
  - sniff/features/2026-07-16-performance/review-12.md
  - sniff/features/2026-07-16-performance/review-2.md
  - sniff/features/2026-07-16-performance/review-3.md
  - sniff/features/2026-07-16-performance/review-4.md
  - sniff/features/2026-07-16-performance/review-5.md
  - sniff/features/2026-07-16-performance/review-6.md
  - sniff/features/2026-07-16-performance/review-7.md
  - sniff/features/2026-07-16-performance/review-8.md
  - sniff/features/2026-07-16-performance/review-9.md
  - sniff/features/2026-07-16-performance/spec.md
  - sniff/reviews/2026-07-13-perf/spec.md
  - sniff/reviews/2026-07-14-filesystem-observation/review.md
skills_files_updated_during_phase_1:
  - .claude/skills/sniff/SKILL.md
  - .claude/skills/sniff/extending.md
  - .claude/skills/sniff/programs.md
source_code:
  - biscuit-speaks/cli/src/install_ui.rs
  - biscuit-speaks/cli/src/main.rs
  - playa/cli/src/install_ui.rs
  - playa/cli/src/main.rs
  - sniff/cli/src/args/mod.rs
  - sniff/cli/src/args/repo.rs
  - sniff/cli/src/commands/mod.rs
  - sniff/cli/src/commands/repo.rs
  - sniff/cli/src/install_plan_cmd.rs
  - sniff/cli/src/install_ui.rs
  - sniff/cli/src/output/filesystem/files.rs
  - sniff/cli/src/output/filesystem/mod.rs
  - sniff/cli/src/output/recent_commits.rs
  - sniff/cli/src/output/repo_json.rs
  - sniff/cli/src/perf.rs
  - sniff/cli/tests/cli.rs
  - sniff/cli/tests/snapshots.rs
  - sniff/lib/benches/cases/filesystem.rs
  - sniff/lib/benches/cases/workload_matrix.rs
  - sniff/lib/benches/perf.rs
  - sniff/lib/benches/support/bench_ids.rs
  - sniff/lib/benches/support/builder.rs
  - sniff/lib/benches/support/fixtures.rs
  - sniff/lib/benches/support/remote_report_fixture.rs
  - sniff/lib/examples/work_counts.rs
  - sniff/lib/src/error.rs
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
  - sniff/lib/src/filesystem/repo/area.rs
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
  - sniff/lib/src/filesystem/repo/ownership.rs
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
  - sniff/lib/src/programs/install/command.rs
  - sniff/lib/src/programs/install/execute.rs
  - sniff/lib/src/programs/install/interview.rs
  - sniff/lib/src/programs/install/mod.rs
  - sniff/lib/src/programs/install/options.rs
  - sniff/lib/src/programs/mod.rs
  - sniff/lib/src/programs/schema.rs
  - sniff/lib/src/remote/bitbucket.rs
  - sniff/lib/src/remote/gitea.rs
  - sniff/lib/src/remote/github.rs
  - sniff/lib/src/remote/gitlab.rs
  - sniff/lib/src/remote/mod.rs
  - sniff/lib/src/remote/provider.rs
  - sniff/lib/src/remote/snapshot.rs
  - sniff/lib/src/request.rs
  - sniff/lib/src/services/benchmark.rs
  - sniff/lib/src/services/launchd.rs
  - sniff/lib/src/services/mod.rs
  - sniff/lib/src/services/openrc.rs
  - sniff/lib/src/services/runit.rs
  - sniff/lib/src/services/systemd.rs
  - sniff/lib/tests/benchmark_workloads.rs
  - sniff/lib/tests/git_parity.rs
  - sniff/lib/tests/integration.rs
  - sniff/lib/tests/remote_providers.rs
documentation:
  - .claude/skills/sniff/SKILL.md
  - .claude/skills/sniff/extending.md
  - .claude/skills/sniff/programs.md
  - .claudine/memory/commits.md
  - CLAUDE.md
  - fixes/2026-07-22-mega-merge/plan.md
  - prompts/_implement/implement-suggestions.md
  - prompts/_reviews/feature-review.md
  - prompts/plan.md
  - sniff/README.md
  - sniff/cli/README.md
  - sniff/docs/sniff-library-architecture.md
  - sniff/features/2026-07-16-performance/deferred-perf-tests.md
  - sniff/features/2026-07-16-performance/log.md
  - sniff/features/2026-07-16-performance/phases/_completed/01-work-accounting/log.md
  - sniff/features/2026-07-16-performance/phases/_completed/01-work-accounting/spec.md
  - sniff/features/2026-07-16-performance/phases/_completed/02-reuse-and-scope/spec.md
  - sniff/features/2026-07-16-performance/phases/_completed/03-observation-index/spec.md
  - sniff/features/2026-07-16-performance/phases/_completed/04-package-enrichment-and-ownership/spec.md
  - sniff/features/2026-07-16-performance/phases/_completed/05-git-observation/spec.md
  - sniff/features/2026-07-16-performance/phases/_completed/06-remote-network-and-subprocess/spec.md
  - sniff/features/2026-07-16-performance/phases/_completed/07-profile-guided-cleanup/spec.md
  - sniff/features/2026-07-16-performance/phases/_completed/08-cross-platform-validation/spec.md
  - sniff/features/2026-07-16-performance/plan.md
  - sniff/features/2026-07-16-performance/review-1.md
  - sniff/features/2026-07-16-performance/review-10.md
  - sniff/features/2026-07-16-performance/review-11.md
  - sniff/features/2026-07-16-performance/review-12.md
  - sniff/features/2026-07-16-performance/review-2.md
  - sniff/features/2026-07-16-performance/review-3.md
  - sniff/features/2026-07-16-performance/review-4.md
  - sniff/features/2026-07-16-performance/review-5.md
  - sniff/features/2026-07-16-performance/review-6.md
  - sniff/features/2026-07-16-performance/review-7.md
  - sniff/features/2026-07-16-performance/review-8.md
  - sniff/features/2026-07-16-performance/review-9.md
  - sniff/features/2026-07-16-performance/spec.md
  - sniff/lib/CHANGELOG.md
  - sniff/lib/README.md
  - sniff/lib/benches/README.md
  - sniff/reviews/2026-07-13-perf/spec.md
  - sniff/reviews/2026-07-14-filesystem-observation/review.md
packages:
  - sniff
  - sniff-cli
  - biscuit-speaks-cli
  - playa-cli
---
# Mega Merge Execution Plan

Status: ready for execution

Source of truth: [spec.md](spec.md)

## Outcome and hard completion gate

Execute one ancestry-preserving integration on the `mega-merge` worktree and
`feat/mega-merge` branch, in dependency order:

1. Sniff
2. Darkmatter
3. Claudine

This plan is complete only when all of the following are true on one unchanged
candidate commit:

- `HEAD` contains the three frozen source tips as ancestors.
- The first-parent history contains one reviewed merge commit for each stream,
  in Sniff → Darkmatter → Claudine order.
- Every workspace Level-1 suite is green.
- Every applicable Level-2 suite is green through its managed `just test-l2`
  recipe; a resource skip is not accepted as a pass for the affected areas.
- All lint recipes complete without warnings or failures.
- Every focused seam, lifecycle, generated-artifact, and platform checkpoint in
  this plan is green.
- Native macOS, Linux, and Windows evidence is attached to the exact candidate
  SHA.
- The Claudine, Darkmatter, and Sniff skills match the merged implementation,
  pass portable Agent Skills validation, have no broken local links, and meet
  the progressive-disclosure gate.
- Final GitNexus change detection has been reviewed against `main`.
- No source worktree or frozen source branch was modified.
- The verified candidate is merged into `main` without changing its tree.

An isolated retry after a full-suite failure is diagnostic evidence only. It
does not satisfy this completion gate; the containing full suite must later
pass.

## Frozen inputs

| Input | Required value |
|---|---|
| Worktree | `/Users/ken/.claudine/worktrees/rusty-biscuit/mega-merge` |
| Branch | `feat/mega-merge` |
| Base `main` | `d30aedd36829256bc677e1d2e73f47a9a2e6005f` |
| Sniff | `0b3286a193899f800a97a24ee3e35c8042602cf6` |
| Darkmatter | `7fb7136dca32a7b1f971b4c83bc1733bcdedebee` |
| Claudine | `8c7a7a8a57d6eebba2e7007df2a6523d9679bbb3` |

Merge the SHA values, not moving branch names. If a source branch advances,
that is a separate integration decision and this plan must be amended before
the new commit enters the candidate.

## Operating rules

- Do not merge `main` into a source branch or merge source branches into one
  another.
- Do not use a global `-X ours`, `-X theirs`, whole-file checkout, or broad
  conflict preference.
- Do not copy a disposable spike tree into the candidate. Replay its decisions
  and tests on the real merge.
- Do not run `cargo fmt` or `rustfmt` in write mode. `main` is the formatting
  authority.
- Preserve unrelated user changes. Stop if an untracked or modified path would
  be overwritten.
- Before editing a function, method, or type during conflict resolution, run
  GitNexus upstream impact analysis for that symbol. Record and surface HIGH or
  CRITICAL results before editing.
- Run GitNexus change detection after each real merge stage and once against
  `main` on the final candidate.
- Run L2 only through the package `just test-l2` recipe. Never invoke L2 through
  Cargo or nextest directly. The managed harness must not steal focus.
- Do not regenerate `CLAUDE.md`, generated provider data, skill hashes, or other
  derived artifacts until production behavior is stable.
- A stage stays uncommitted while red. Create its merge commit only after its
  stage gate is green and its conflict ledger is complete.

## Evidence ledger

Fill these tables during execution. Keep command logs or CI URLs beside the
commit they certify.

### Merge ledger

| Stage | Pre-merge `HEAD` | Incoming SHA | Merge commit | Parent SHAs | Conflicts reviewed | GitNexus result |
|---|---|---|---|---|---|---|
| Sniff | `ae143e497f5a02368f62fad11d6d6adcf49e03e7` | `0b3286a193899f800a97a24ee3e35c8042602cf6` | deferred by no-commit instruction | pending | `CLAUDE.md` | MEDIUM: 5 flows reviewed; 2 CLI performance-output flows covered by L1, 3 benchmark-fixture flows outside production runtime paths |
| Darkmatter | `ae143e497f5a02368f62fad11d6d6adcf49e03e7` + reviewed uncommitted Sniff tree | `7fb7136dca32a7b1f971b4c83bc1733bcdedebee` | deferred by no-commit instruction | pending | pending | pending |
| Claudine | | `8c7a7a8a57d6eebba2e7007df2a6523d9679bbb3` | | | | |

### Conflict ledger

For every conflict and every auto-merged semantic-audit file, record:

| Stage | Path/symbol | Classification | Starting authority | Required additive behavior | Focused proof |
|---|---|---|---|---|---|
| Sniff | `CLAUDE.md` | generated / operational | Candidate generated counts | Removed both nested conflict-marker layers; retained the candidate's pre-merge GitNexus counts pending Phase 4 regeneration. | `rg` found no live conflict markers; scoped source and focused suites compile. |
| Sniff | `get_worktrees_from_snapshot` | behavioral | Sniff focused inspection | Omit absent registered targets without weakening typed errors for existing corrupt repositories. | `get_worktrees_omits_an_absent_registered_target`; `get_worktrees_surfaces_an_existing_corrupt_target`; existing full-detail test. |

### Verification ledger

| Candidate SHA | Host/OS | Gate | Command/workflow | Result | Log or URL |
|---|---|---|---|---|---|
| `ae143e497f5a02368f62fad11d6d6adcf49e03e7` + uncommitted Sniff merge tree | macOS | Focused Spike A | scoped nextest filters for aggregate, worktree, remote/provider, PR/CI, and work counters | 21 passed | local session |
| `ae143e497f5a02368f62fad11d6d6adcf49e03e7` + uncommitted Sniff merge tree | macOS | Sniff L1 | `just test` | 1,679 library + 782 CLI passed | local session |
| `ae143e497f5a02368f62fad11d6d6adcf49e03e7` + uncommitted Sniff merge tree | macOS | Sniff lint | `just lint` | passed without warnings | local session |
| `ae143e497f5a02368f62fad11d6d6adcf49e03e7` + uncommitted Sniff merge tree | macOS | Sniff L2 | `BISCUIT_TEST_LEVEL_REQUIRED=2 just test-l2` | 2 passed through managed harness | local session |
| `ae143e497f5a02368f62fad11d6d6adcf49e03e7` + uncommitted Sniff merge tree | macOS | GitNexus | `detect_changes(scope=all)` | MEDIUM; 5 affected flows reviewed | local session |

## Phase 0 — Freeze and baseline the candidate

### Goal

Prove that execution starts from the intended branch, worktree, and frozen
inputs, with no hidden changes that can contaminate a merge.

### Tasks

- [ ] Confirm the current directory and branch:

  `git rev-parse --show-toplevel` must return the `mega-merge` path and
  `git branch --show-current` must return `feat/mega-merge`.

- [ ] Confirm `HEAD` is the frozen base before the first merge:

  `git rev-parse HEAD` must equal
  `d30aedd36829256bc677e1d2e73f47a9a2e6005f`, except for an explicitly
  reviewed planning-only commit containing this spec and plan.

- [ ] Record `git status --short --branch` and `git worktree list --porcelain`.
  Resolve or explicitly quarantine every unexpected candidate-side change.

- [ ] Confirm each frozen SHA resolves and record the source branch/worktree
  status. Do not clean or modify the source worktrees. In particular, preserve
  Claudine's locally modified generated `CLAUDE.md` as source-worktree state,
  not merge input.

- [ ] Capture the authoritative package catalog with
  `cargo metadata --no-deps --format-version 1` and `sniff repo packages`.

- [ ] Record current CI workflow coverage for Sniff, Biscuit File, Darkmatter,
  and Claudine. Mark every missing native-OS or hard-required L2 leg for Phase
  7; do not mistake a soft or skipped check for evidence.

- [ ] Save the exact focused test filters used by the spikes. If a filter name
  changed in a frozen input, map it to the surviving test rather than silently
  dropping the checkpoint.

### Exit gate

The candidate is clean except for reviewed planning files, all four SHAs are
recorded, source branches remain untouched, and the execution/evidence ledger
is ready.

## Phase 1 — Merge and stabilize Sniff

### Merge

- [x] Record the pre-merge `HEAD`.
- [x] Run:

  `git merge --no-ff --no-commit 0b3286a193899f800a97a24ee3e35c8042602cf6`

- [x] Classify every conflict. Defer final `CLAUDE.md` reconciliation to Phase
  4, but do not let an unresolved generated file hide semantic changes.
- [x] Review all auto-merged files that touch Git discovery, worktrees, remotes,
  request presets, aggregate repository output, or the Sniff skill.

### Required behavioral contract

- [x] Aggregate repository projection reads Git administration metadata without
  opening linked repositories.
- [x] Aggregate projection preserves prunable/stale registrations and keeps the
  zero-linked-repository-open work counter.
- [x] Focused worktree inspection opens registered targets, omits an absent
  stale target, and returns a typed error for an existing corrupt repository.
- [x] Remote selection reuses the request's repository handle and performs no
  repository rediscovery.
- [x] Provider URL, exact/list pull-request, and CI/CD methods remain reachable;
  compatible defaults keep existing provider implementations compiling.
- [x] Bare repositories, linked worktrees, and platform path handling retain
  their documented behavior.

### Verification

- [x] Run the focused aggregate/worktree/remote/work-counter tests identified
  by Spike A.
- [x] Run `just test` and `just lint` from `sniff/`. The full L1 run must be
  warning-free.
- [x] Run `BISCUIT_TEST_LEVEL_REQUIRED=2 just test-l2` from `sniff/` on a host
  with the managed harness.
- [x] Run GitNexus change detection for this stage and review affected flows.
- [x] Record all proof and leave the reviewed merge tree uncommitted for the
  separately authorized commit operation, as required by this implementation
  session's explicit no-commit/no-stage instruction.

### Exit gate

The Sniff stage is independently green, the reviewed integration tree is ready
for the separately authorized merge commit that will preserve the frozen Sniff
tip, and no Darkmatter or Claudine conflict has been resolved early.

## Phase 2 — Merge and stabilize Darkmatter

### Merge

- [x] Record the Sniff-stage `HEAD`.
- [ ] Run:

  `git merge --no-ff --no-commit 7fb7136dca32a7b1f971b4c83bc1733bcdedebee`

- [ ] Resolve the Sniff–Darkmatter boundary from the ownership matrix:
  Sniff owns aggregate observation cost; Darkmatter owns focused target
  validation; remote-provider additions are additive.
- [ ] Review every auto-merged file in Sniff repository discovery, Darkmatter
  context capture, Biscuit File facade/export modules, remote providers,
  schemas, references, caching, and skills.

### Required behavioral contract

- [ ] Preserve Sniff's request-scoped “observe once, project many” architecture.
- [ ] Preserve Darkmatter's bare-repository handling, typed focused-worktree
  errors, exact/list PR operations, and CI/CD operations.
- [ ] Make Biscuit File exports additive: keep Claudine's file/list surfaces and
  Darkmatter's YAML span/analyzer surfaces.
- [ ] Keep Darkmatter context capture demand-driven and routed through Sniff.
- [ ] Confirm there is no accidental network work in aggregate or denied
  request paths.

### Verification

- [ ] Replay all Spike A focused tests on the real candidate.
- [ ] Run `just test` and `just lint` from `sniff/`.
- [ ] Run `just test` and `just lint` from `biscuit-file/`.
- [ ] Run `just test` and `just lint` from `darkmatter/`.
- [ ] Run `BISCUIT_TEST_LEVEL_REQUIRED=2 just test-l2` from `sniff/` and
  `darkmatter/`.
- [ ] Run Darkmatter's headless browser gate through
  `BISCUIT_BROWSER_REQUIRED=1 just test-browser`. It must remain headless and
  must not gain focus.
- [ ] Run GitNexus change detection for this stage and review affected flows.
- [ ] Record all proof, then create the Darkmatter merge commit with both
  parents.

### Exit gate

Sniff, Biscuit File, and Darkmatter are jointly green before Claudine enters
the candidate.

## Phase 3 — Merge Claudine in controlled work packets

### Merge

- [ ] Record the Darkmatter-stage `HEAD`.
- [ ] Run:

  `git merge --no-ff --no-commit 8c7a7a8a57d6eebba2e7007df2a6523d9679bbb3`

- [ ] Classify every conflict and add every auto-merged composition/schema/
  reference/lifecycle file to the semantic audit list.
- [ ] Do not restore Claudine-deleted monolithic lifecycle test files. Port
  surviving assertions into Claudine's split test layout.

### Work packet A — API and facade spine

- [ ] Compose, rather than replace, `biscuit-file` module declarations and
  exports.
- [ ] Establish the compile spine in this order:
  Biscuit File → Darkmatter library → Darkmatter tests → Claudine library →
  Claudine CLI/tests.
- [ ] Run focused compile/check after each seam closes so API-shape failures do
  not mix with orchestration failures.

### Work packet B — Request context and cache identity

- [ ] Keep Claudine's single captured `FileResolutionContext`.
- [ ] Add Darkmatter's remote runtime, cache, identity, provider-query, trigger,
  meta-schema, origin, and dependency state without recapturing ambient CWD,
  environment, or repository state.
- [ ] Give each compose option one cache classification and one encoding;
  `options_hash` must delegate to the canonical compose-cache fingerprint.
- [ ] Prove cache eligibility and cache-key construction cannot drift.

### Work packet C — File/expression projection and authorization parity

- [ ] Preserve typed whole-value object results and the scalar/array fast path.
- [ ] Route string object projection through the lookup hook so configured
  `name_coercion_keys` can select `.name`.
- [ ] Give frontmatter, body interpolation, and shell ternary reads the same
  authorized remote runtime, denial behavior, and cache behavior.
- [ ] Prove deny-before-network behavior on every surface.

### Work packet D — References and freshness

- [ ] Normalize request options exactly as graph construction does before
  comparing identities.
- [ ] Resolve targets through the captured Claudine context.
- [ ] Preserve Darkmatter's prepared-heading cache for cross-document fragment
  validation.
- [ ] Preserve typed `ReferenceError` values; stringify only genuinely
  non-reference graph failures.
- [ ] Replay the freshness mutation matrix: child, descendant, heading, schema
  dependency, and option identity each invalidate through only their documented
  channel.

### Work packet E — Schema assembly and recursion

- [ ] Preserve Claudine request-scoped resolution and typed errors.
- [ ] Restore Darkmatter origin, dependency, namespace/example cache, trigger,
  meta-schema, and source-aware validation state.
- [ ] Thread the immutable request context through scalar and root-union
  recursion while preserving the canonical-path open-frame stack and depth cap.
- [ ] Prove direct/transitive cycles, repeated non-cyclic references, exact
  depth, depth exhaustion, root unions, and dependency collection.

### Work packet F — Lifecycle traversal and test port

- [ ] Extend preflight traversal through array elements and object values;
  object keys remain data.
- [ ] Port the eight surviving lifecycle/container assertions into Claudine's
  split validation and filesystem-lookup suites.
- [ ] Retire the four contradictory launch-area fallback assertions. The
  source-local semantics in Claudine remain authoritative.
- [ ] Preserve handoff, retry, resume, provider selection, and unavailable
  provider refusal behavior.

### Mandatory focused regression groups

All eight groups must pass on the real staged candidate:

1. cache eligibility/hash encoding parity;
2. normalized graph validation identity;
3. prepared-heading cache use for cross-document fragments;
4. authorized frontmatter remote runtime;
5. name-coercion lookup behavior;
6. schema cycle/depth protection;
7. typed reference-error transport;
8. lifecycle traversal of array/object expression literals.

Also require:

- [ ] provider-network and deny-before-network group;
- [ ] prebuilt reference graph compatibility/freshness group;
- [ ] cache-root and fragment-validation groups;
- [ ] Biscuit File captured-context differential oracle;
- [ ] all eight ported lifecycle assertions;
- [ ] unavailable scalar/list provider refusal L1 tests;
- [ ] the managed unavailable-provider retry L2 test four consecutive times.

### Stage verification

- [ ] Run `just test` and `just lint` in `biscuit-file/`, `sniff/`,
  `darkmatter/`, and `claudine/`.
- [ ] Run hard-required managed L2 in `sniff/`, `darkmatter/`, and
  `claudine/`.
- [ ] Run Claudine `signals-check` and `test-gen`.
- [ ] Run Darkmatter's required headless browser gate.
- [ ] Re-run the complete Darkmatter and Claudine L1 suites until each has one
  clean full-suite result. The previously observed slow cleanup and context
  width timeouts remain visible in the ledger; an isolated pass does not close
  them.
- [ ] Run GitNexus change detection for the Claudine stage and review affected
  flows.
- [ ] Record all proof, then create the Claudine merge commit with both parents.

### Exit gate

The candidate contains all three frozen tips and all focused semantic seams are
green before generated files or skills are finalized.

## Phase 4 — Generated artifacts and repository hygiene

- [ ] Confirm the purpose and intended final path of the tracked empty
  `~/features/2026-07-20-router-fixture/log.md` path. Keep, move, or delete it
  only from explicit fixture evidence.
- [ ] Compare generated outputs before regeneration and classify every
  difference. Generated files must not smuggle a semantic conflict resolution.
- [ ] Regenerate provider/catalog outputs once from the settled source, using
  repository recipes.
- [ ] Regenerate `CLAUDE.md`/GitNexus counts from the settled candidate rather
  than taking either branch's generated copy.
- [ ] Confirm no local `.claude/settings.local.json` or source-worktree-only
  change entered the candidate.
- [ ] Review `Cargo.lock`, manifests, generated schemas/catalogs, and symlinks
  explicitly.
- [ ] Re-run the focused tests affected by regeneration and verify a second
  generation is clean (idempotent).

### Exit gate

Derived artifacts are reproducible from the merged source and a second
generation produces no diff.

## Phase 5 — Agent Skills drift and progressive disclosure

Perform this phase after behavior and generated artifacts settle so the skills
describe the final implementation once.

### Common audit for all three skills

- [ ] Compare the merged package architecture, public APIs, CLI commands,
  test recipes, platform behavior, and invariants to every claim in
  `.claude/skills/{claudine,darkmatter,sniff}/`.
- [ ] Assume code is correct when a comment or skill claim drifts; update or
  remove the stale documentation in the same documentation phase.
- [ ] Validate the frontmatter description contains the real trigger contexts
  and the body does not carry a redundant “when to use” section.
- [ ] Search for consumers of top-level `hash` and `last_updated` before
  changing them. Normalize the entry files to portable Agent Skills
  frontmatter; do not retain non-standard top-level keys without a documented
  consumer and reviewed exception.
- [ ] Do not add Codex-specific `agents/openai.yaml` sidecars merely for this
  gate. These repository skills target the provider-neutral Agent Skills core.
- [ ] Keep essential procedures and safety invariants in `SKILL.md`; move
  history, catalogs, long examples, and subsystem detail to directly linked
  references.
- [ ] Keep each `SKILL.md` below 500 lines and approximately 5,000 words.
- [ ] Ensure every retained long operational reference is navigable (compact
  contents list or an explicit, reviewed reason that its existing structure is
  sufficient).
- [ ] Avoid duplicated facts between `SKILL.md` and references. The entry point
  should route; the reference should own the detail.
- [ ] Validate every local Markdown link and symlink resolves from the skill
  directory. Fix the known Darkmatter `code_block.rs` relative link.
- [ ] Confirm all resource paths use portable relative references and that no
  link depends on a source worktree outside the skill package.

### Skill-specific restructuring

- [ ] Claudine: preserve its concise architecture/CLI routing, then audit the
  incoming `error-architecture.md`, `messaging.md`, architecture, CLI, linking,
  and research links for final-behavior drift and duplication.
- [ ] Darkmatter: move the DMLS phase chronology, extracted-surface catalog,
  and detailed rendering implementation notes behind directly linked topic
  references. Keep the composition/schema/context/remote authority and browser
  safety contract in the entry point.
- [ ] Sniff: move work-counter evidence and detailed catalog/CLI material behind
  topic references. Keep platform support, request-cost tiers, aggregate versus
  focused worktree semantics, and cross-platform test gotchas in the entry
  point.

### Mechanical validation

- [ ] Run for each skill:

  `uv run --with pyyaml /Users/ken/.claude/skills/.system/skill-creator/scripts/quick_validate.py .claude/skills/<skill>`

- [ ] Run the link/symlink check from the skill directory and record zero
  broken targets.
- [ ] Record final line and word counts for all three entry points.
- [ ] Render/read each entry point and one routed topic as a cold reader; verify
  that the correct next document is discoverable without loading unrelated
  subsystems.
- [ ] Re-run package documentation drift guards, including Claudine's lifecycle
  facet check and any generated-doc checks.

### Exit gate

All three skills are accurate, portable, validator-green, link-clean, and
progressively disclosed. A reviewed exception must name the rule, rationale,
and owner; “existing file” is not a rationale.

## Phase 6 — Final local verification on macOS

Run these against one recorded candidate SHA. Any subsequent code, test,
generated, skill, manifest, or workflow change invalidates the result and
requires the affected gates plus the final aggregate gates again.

### Workspace-wide gates

- [ ] `just check-canonical`
- [ ] `just build`
- [ ] `just test` — all Cargo workspace packages from metadata, Level 1
- [ ] `just doctest`
- [ ] `just lint` — zero warnings and zero failures
- [ ] `just all` — all canonical tiers for every curated package area
- [ ] `just check-test-interrupts`
- [ ] `just test-leaks sniff biscuit-file darkmatter claudine`

### Hard-required affected-area gates

- [ ] `(cd sniff && BISCUIT_TEST_LEVEL_REQUIRED=2 just test-l2)`
- [ ] `(cd darkmatter && BISCUIT_TEST_LEVEL_REQUIRED=2 just test-l2)`
- [ ] `(cd claudine && BISCUIT_TEST_LEVEL_REQUIRED=2 just test-l2)`
- [ ] `(cd darkmatter && BISCUIT_BROWSER_REQUIRED=1 just test-browser)`
- [ ] `(cd claudine && just signals-check && just test-gen)`

Record executed, passed, skipped, and not-applicable counts separately. A skip
in an affected L1/L2 gate is a blocker unless the test is intentionally
platform-inapplicable and the ledger identifies the native host that executes
it.

## Phase 7 — Native Linux and Windows evidence

The exact candidate SHA must receive native evidence; cross-compilation alone
does not satisfy runtime path, filesystem, or work-counter behavior.

### CI coverage closure

- [ ] Keep the existing Sniff macOS/Linux/Windows all-target and L1 matrix, and
  Unix L2 legs, green.
- [ ] Promote Darkmatter's Windows leg from soft evidence to a required check
  for this candidate. Enable its reusable Linux L2 and headless-browser jobs.
- [ ] Add durable Biscuit File and Claudine area coverage through the shared
  area-CI workflow (or an equally strict existing workflow): macOS all-target
  check, Linux and Windows L1, and Linux hard-required L2 where applicable.
  Preserve Claudine's generator/signals job and Windows Ctrl+C runtime job.
- [ ] Ensure workflow lint/check output treats warnings as failures.
- [ ] Do not use a temporary workflow that is deleted before the final
  candidate SHA; the evidence must correspond to the tree being merged.

### Native functional matrix

| Checkpoint | macOS | Linux | Windows |
|---|---:|---:|---:|
| Sniff all-target compile + L1 | required | required | required |
| Sniff work counters and aggregate/focused worktree behavior | required | required | required |
| Biscuit File captured CWD/env/root/path oracle | required | required | required |
| Darkmatter all-target compile + L1 | required | required | required |
| Claudine all-target compile + L1 | required | required | required |
| Managed L2 | required where harness is supported | hard-required | platform-inapplicable unless a native harness exists |
| Darkmatter headless browser | required locally or Linux CI | hard-required | not required |
| Claudine Windows Ctrl+C/Job Object | not applicable | not applicable | required |
| Lint, docs guards, generated drift | required | required | required or compile-equivalent where shell tooling is unavailable |

Special attention on Linux and Windows:

- [ ] drive-relative and UNC paths;
- [ ] separator/case normalization;
- [ ] symlink and reparse-point containment;
- [ ] worktree administration paths and prunable registrations;
- [ ] no extra repository opens or network probes;
- [ ] typed syntax/permission errors across enumeration, graph construction,
  and validation.

### Exit gate

All required checks are green and non-soft on the exact candidate SHA. A
cancelled, skipped, allowed-to-fail, or superseded run is not evidence.

## Phase 8 — Final review, ancestry proof, and merge to main

### Ancestry and history

- [ ] Run:

    - `git merge-base --is-ancestor 0b3286a193899f800a97a24ee3e35c8042602cf6 HEAD`
    - `git merge-base --is-ancestor 7fb7136dca32a7b1f971b4c83bc1733bcdedebee HEAD`
    - `git merge-base --is-ancestor 8c7a7a8a57d6eebba2e7007df2a6523d9679bbb3 HEAD`

- [ ] Inspect `git log --first-parent --merges --oneline` and the parent list of
  each stage commit. Confirm Sniff → Darkmatter → Claudine order and two-parent
  merge commits.
- [ ] Confirm the source branch tips still equal the frozen SHAs and source
  worktree status has not changed due to this execution.

### Change and conflict review

- [ ] Run GitNexus `detect_changes` with compare scope against `main` and the
  explicit `mega-merge` worktree path.
- [ ] Review all reported affected processes and all HIGH/CRITICAL symbols.
  Compare them to the conflict ledger; unexplained scope is a blocker.
- [ ] Review `git diff --check` and the full `main...HEAD` diff.
- [ ] Confirm no conflict markers, temporary spike paths, generated drift,
  local settings, or accidental formatting-only rewrites remain.
- [ ] Confirm every checkbox and evidence-ledger row is complete.

### Merge

- [ ] Freeze the verified candidate SHA.
- [ ] Merge that exact candidate into `main` without squashing away its source
  ancestry and without changing the candidate tree.
- [ ] Verify the resulting `main` tree matches the candidate tree and all three
  frozen tips remain ancestors of `main`.
- [ ] Do not declare completion until required post-merge branch protection/CI
  checks are green.

## Stop conditions

Stop and update the spec before proceeding if:

- a frozen source SHA changes;
- a proposed resolution contradicts the semantic ownership matrix;
- a whole-file preference appears necessary in schemas, references, context,
  or lifecycle code;
- a source branch/worktree would need modification;
- a focused seam test has no surviving equivalent;
- a HIGH/CRITICAL GitNexus result is not understood;
- a full suite repeatedly passes only when isolated;
- native Windows/Linux evidence requires weakening a check;
- an Agent Skill cannot meet portable validation without breaking a confirmed
  repository consumer.

The response to a stop condition is a documented decision or a new focused
spike—not a broader merge strategy, a skipped gate, or an unexplained
exception.
