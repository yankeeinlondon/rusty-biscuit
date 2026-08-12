---
title: Retire the Messenger and Rendezvous specialized workflows
status: ready
created: 2026-08-12
phase: 1
total_phases: 6
agent: codex/default
yolo: true
spec: fixes/2026-08-07-retire-specialized-workflows/spec.md
depends_on:
  - fixes/2026-08-06-cicd/spec.md
source_code:
  - .github/workflows/ci.yml
  - .github/workflows/_package-ci.yml
  - .github/workflows/_wsl-ci.yml
  - .github/workflows/messenger-desktop-tests.yml
  - .github/workflows/rendezvous-tests.yml
  - messenger/lib/Cargo.toml
  - messenger/cli/Cargo.toml
  - messenger/lib/src/tests/desktop_helpers.rs
  - scripts/ci/affected_scope.py
  - scripts/ci/test_affected_scope.py
  - tools/test-toolkit/tests/ci_workflow_contracts.rs
docs:
  - .github/ci/README.md
  - docs/topics/ci-cd.md
  - docs/testing-strategy.md
  - claudine/docs/rendezvous/local-ipc.md
  - claudine/features/2026-07-12-rendezvous-dashboard/windows-support-followup.md
  - .claude/skills/claudine/architecture.md
  - fixes/2026-08-07-retire-specialized-workflows/implementation-notes.md
---

# Execution plan

## Objective

Retire `messenger-desktop-tests.yml` and `rendezvous-tests.yml` without losing
their owned test coverage. Messenger and all three Rendezvous packages must
instead produce ordinary package-keyed native and WSL2 evidence that reaches
`ci-verdict`. The checked-in retirement is atomic: do not merge workflow
deletion before messenger fixture delivery, package policy, contracts,
documentation, and any run-proven baseline entries are complete.

## Execution constraints

- The per-package cutover from `fixes/2026-08-06-cicd/spec.md` is a hard
  prerequisite. Package-keyed scope, `_package-ci.yml`, `_wsl-ci.yml`, producer
  statuses, JUnit artifacts, and the package-keyed verdict must already be the
  active authority.
- Preserve the public `messenger/justfile` and
  `claudine/rendezvous/justfile` behavior. This change alters CI ownership and
  test-fixture delivery, not local recipe semantics or production behavior.
- Use nextest through canonical `just` recipes for Rust test validation. Do not
  use `cargo test`, do not run `cargo fmt`, and do not restore specialized
  compile-only jobs for macOS/Linux examples.
- Treat historical completed specs, plans, reviews, handoffs, and measurements
  as immutable records. Active workflows, contracts, docs, manifest comments,
  and the Claudine skill architecture must describe the new authority.
- Baseline entries are evidence-driven. A newly exposed failure may be added
  only after a promotion run proves its exact package, environment, tier,
  ownership, source run, reason, and expiry; failures introduced by this work
  are fixed.
- Parallelization is allowed only at the explicit points below. Tasks within a
  phase otherwise remain ordered by their stated dependencies.

## Phase 1 — Freeze the replacement contracts

- [ ] **Task 1.1 — Confirm the prerequisite cutover.** On the implementation
  branch, verify that affected scope emits package records, `_package-ci.yml`
  runs one package per result-producing job, `_wsl-ci.yml` runs package-keyed
  L1 archives, and `ci-verdict` consumes `{package, environment, tier}` JUnit
  and producer-status artifacts. Record the inspected revision and findings in
  `implementation-notes.md`; stop if the parent cutover is incomplete.

- [ ] **Task 1.2 — Capture a clean-checkout coverage inventory.** From the same
  revision, record each retired command, the test targets it selects, its
  replacement package cell, and the one intentional reduction:
  `rendezvous-daemon`'s non-test `register_compaction_spike` example is no
  longer compile-checked on macOS/Linux. Include test counts or nextest listings
  where they make the mapping non-vacuous.

- [ ] **Task 1.3 — Add policy and scope regression tests.** Extend
  `scripts/ci/test_affected_scope.py` to prove that messenger resolves as
  gating with `all_features = true`, retains `libdbus-1-dev`, owns the
  `messenger-desktop-stubs` runner tool, and passes the same feature contract
  into check, native L1, and WSL archive inputs. Add real-workspace scope tests
  proving a messenger-cli-only change selects `messenger-cli` through normal
  package scope and a `sniff` change selects the exact Cargo-derived
  Rendezvous/Claudine reverse-dependency closure.

- [ ] **Task 1.4 — Add workflow retirement and evidence contracts.** Extend
  `tools/test-toolkit/tests/ci_workflow_contracts.rs` with assertions that the
  deleted workflows and orchestration jobs are absent, the specialized
  inventory contains only the remaining specialized workflows, messenger
  runner tooling reaches both native and WSL2 execution, and messenger and
  Rendezvous failures are represented by package-keyed evidence consumed by
  `ci-verdict` rather than by `needs` alone.

- [ ] **Validation checkpoint 1 — Prove the new tests are meaningful.** Run
  `python3 scripts/ci/test_affected_scope.py` and
  `cargo nextest run -p test-toolkit --test ci_workflow_contracts`; document
  the expected failures against the pre-retirement implementation and verify
  each fails for the intended missing contract rather than fixture setup.

## Phase 2 — Promote messenger and deliver its helper stubs

- [ ] **Task 2.1 — Promote messenger package policy.** In
  `messenger/lib/Cargo.toml`, remove `gates = false` and all exclusion fields,
  retain `ubuntu-latest = ["libdbus-1-dev"]`, replace
  `features = ["desktop"]` with `all-features = true`, and declare
  `runner-tools = ["messenger-desktop-stubs"]`. In
  `messenger/cli/Cargo.toml`, remove the complete exclusion block so the CLI
  uses default gating L1 policy. Update or remove the now-stale manifest
  comments in the same edit.

- [ ] **Task 2.2 — Register the closed runner tool (parallelizable with Task
  2.1 after Phase 1).** Add
  `messenger-desktop-stubs` to `KNOWN_RUNNER_TOOLS` and its validation tests in
  `scripts/ci/affected_scope.py` and `scripts/ci/test_affected_scope.py`.
  Forward the package-owned tool unchanged to native L1 and `_wsl-ci.yml`;
  keep runner tools non-propagating to Cargo dependents.

- [ ] **Task 2.3 — Make stub resolution explicit and testable (parallelizable
  with Tasks 2.1–2.2 after Phase 1).** Update
  `messenger/lib/src/tests/desktop_helpers.rs` so
  `MESSENGER_STUB_BIN_DIR` is the first resolution source, followed by the
  existing target-directory lookup and local Cargo build-on-demand fallback.
  Treat the explicit directory as authoritative: when it is set, a missing
  binary must fail with a clear fixture error instead of consulting source
  timestamps or silently invoking Cargo. Refresh the module and function
  documentation to describe the new precedence and retain the local fallback
  contract.

- [ ] **Task 2.4 — Add resolver unit coverage.** Add tests for explicit
  directory precedence, Windows `.exe` naming, missing explicit fixtures, and
  fallback eligibility without launching a nested Cargo build. Serialize
  environment mutation with the repository test utilities so parallel nextest
  processes cannot race on `MESSENGER_STUB_BIN_DIR`.

- [ ] **Task 2.5 — Prebuild stubs once for native L1.** In
  `_package-ci.yml`, implement the runner tool before the L1 nextest step:
  build messenger's all-feature helper binaries once, verify all six expected
  executables, export their directory through `GITHUB_ENV`, and leave the
  existing package test command responsible only for executing tests. Add an
  observable log/contract assertion that there is one prebuild step and no
  per-test fixture build.

- [ ] **Task 2.6 — Ship a Linux stub sidecar to WSL2.** In `_wsl-ci.yml`, add a
  `runner-tools` input and, only for `messenger-desktop-stubs`, build all six
  Linux helper binaries in the archive job, stage them as a package-specific
  sidecar artifact, download them in the WSL job, copy them into the ext4
  guest with executable permissions and unprivileged ownership, and export
  `MESSENGER_STUB_BIN_DIR` in the archive test process. Fail if any expected
  sidecar executable is absent.

- [ ] **Task 2.7 — Prove the WSL guest remains toolchain-free.** Strengthen the
  WSL workflow contract so the messenger archive step verifies `cargo` and
  `rustc` are unavailable to the unprivileged guest before nextest runs. The
  successful helper tests must therefore prove the shipped sidecar was used
  and the build-on-demand fallback was unreachable.

- [ ] **Validation checkpoint 2 — Validate messenger locally.** Run the
  messenger resolver/unit tests, then run the package's canonical all-feature
  L1 and lint paths with the runner-tool setup reproduced locally. Confirm all
  six stubs are resolved from the explicit directory and no test process starts
  Cargo. Do not treat this macOS result as Windows, Linux, or WSL2 evidence.

## Phase 3 — Remove the specialized workflow graph

- [ ] **Task 3.1 — Delete the two workflow files.** Remove
  `.github/workflows/messenger-desktop-tests.yml` and
  `.github/workflows/rendezvous-tests.yml`, including the Rendezvous SID
  redaction and raw-log upload steps. Do not add SID transformation to JUnit or
  result staging.

- [ ] **Task 3.2 — Remove specialized orchestration from `ci.yml`.** Delete the
  `rendezvous` and `messenger-desktop` jobs, remove them from both
  `ci-verdict.needs` and the advisory summary's `needs`, and remove their
  failure-class lines. Retain the package fan-out as their only CI scheduler
  and leave the unrelated specialized legs unchanged.

- [ ] **Task 3.3 — Remove the messenger-only scope output.** Delete the
  prefix-matched `messenger` output, its scope-job export, and the stale
  comments explaining exempt-package selection. Verify that no remaining
  consumer depends on this flag and that a messenger-cli-only change still
  schedules its normal package cell.

- [ ] **Task 3.4 — Narrow the specialized inventory.** Update `ORCHESTRATED`
  and its comments in `ci_workflow_contracts.rs` to retain only
  `biscuit-tui-windows-captured-stdout.yml` and `playa-windows.yml`. Keep the
  reusable/manual-dispatch assertions for those survivors while separately
  asserting the two retired workflow files and job blocks do not exist.

- [ ] **Task 3.5 — Verify verdict ownership.** Add or update negative-path
  contracts showing a messenger or Rendezvous L1 producer failure yields its
  package-keyed FAIL/MISSING cell and blocks `ci-verdict` unless an exact valid
  baseline entry applies. Confirm no code infers their result from the deleted
  job names or advisory summary.

- [ ] **Validation checkpoint 3 — Prove complete graph deletion.** Use targeted
  `rg` checks over active workflows, scripts, recipes, contract tests, current
  docs, and manifest comments to show there are no live references to either
  filename, job, messenger prefix flag, or Rendezvous SID-redaction machinery.
  Historical lifecycle records are explicitly excluded from this check.

## Phase 4 — Update active authority and implementation evidence

- [ ] **Task 4.1 — Update CI policy documentation.** Update
  `.github/ci/README.md`, `docs/topics/ci-cd.md`, and
  `docs/testing-strategy.md` to add `messenger-desktop-stubs` to the closed
  vocabulary, describe native and WSL2 fixture delivery, name the package grid
  as messenger/Rendezvous coverage authority, and retain only genuinely
  specialized workflows in the active inventory.

- [ ] **Task 4.2 — Update Rendezvous documentation (parallelizable with Task
  4.1 after Phase 3).** Update
  `claudine/docs/rendezvous/local-ipc.md` and the active Windows follow-up to
  replace specialized-workflow ownership with package-keyed native and WSL2
  L1 cells. Remove the retired SID-redaction claim while preserving all
  endpoint/security terminology that describes actual implementation behavior.

- [ ] **Task 4.3 — Update the Claudine skill architecture (parallelizable with
  Tasks 4.1–4.2 after Phase 3).** Replace the stale
  `rendezvous-tests.yml` coverage statement in
  `.claude/skills/claudine/architecture.md` with the normal package-grid
  contract, including macOS/Linux/Windows/WSL2 L1 ownership and the intentional
  macOS/Linux example-only compile reduction.

- [ ] **Task 4.4 — Complete the command-to-cell evidence record.** In
  `implementation-notes.md`, map all six retired commands to the exact
  package/environment/tier cells, record clean-checkout test listings/counts,
  document native single-prebuild and WSL sidecar proof, and explicitly state
  that `register_compaction_spike` no longer receives macOS/Linux compile-only
  coverage. Do not claim equivalent all-target coverage where it does not
  exist.

- [ ] **Task 4.5 — Audit comments and cache-key references.** Search active
  manifests, workflow comments, CI docs, and test comments for claims that the
  deleted workflows own coverage or that messenger uses desktop-only CI.
  Correct or delete drifted comments, treating the implementation as the
  authority, and leave historical records untouched.

- [ ] **Validation checkpoint 4 — Review documentation against contracts.** For
  every active statement about messenger/Rendezvous environments, feature
  selection, fixture delivery, result identity, or specialized inventory,
  identify the enforcing manifest/workflow/contract assertion. Resolve any
  statement without a matching executable contract.

## Phase 5 — Run deterministic local validation

- [ ] **Task 5.1 — Validate scope and workflow contracts.** Run
  `python3 scripts/ci/test_affected_scope.py` and
  `cargo nextest run -p test-toolkit --test ci_workflow_contracts`. Confirm the
  tests are non-vacuous by checking the messenger-cli-only and `sniff` fixtures
  assert exact package sets rather than mere inclusion.

- [ ] **Task 5.2 — Validate messenger.** From `messenger/`, run `just test` and
  `just lint`, then use the shared per-package recipes to run messenger L1 and
  lint with `--all-features` and the explicit stub directory. Record the test
  count and evidence that helper binaries were built once before nextest.

- [ ] **Task 5.3 — Validate Rendezvous and Claudine call sites (parallelizable
  with Task 5.2 after Task 5.1).** From
  `claudine/rendezvous/`, run `just test` and `just lint`; also run the complete
  canonical L1 suite for `claudine-cli` so dashboard, session-report, requeue,
  and command-handler call sites are covered as a strict superset of the
  retired filter. Do not run L2/L3 or any focus-taking terminal/browser suite.

- [ ] **Task 5.4 — Validate workflow structure and stale references.** Parse or
  lint all changed YAML workflows with the repository-supported tooling, rerun
  targeted active-file `rg` checks for the deleted graph and SID transform,
  and inspect `git diff --check`. Verify no unrelated specialized workflow,
  package policy, historical record, or local recipe behavior changed.

- [ ] **Validation checkpoint 5 — Local readiness review.** Require all Phase 5
  commands to pass on macOS and review every changed shell step for bash,
  PowerShell, Windows executable-suffix, Unix permission, path quoting, and
  artifact-name portability. Any platform-only uncertainty becomes an explicit
  promotion-run observation item, not an assumed pass.

## Phase 6 — Prove the promotion in CI and close atomically

- [ ] **Task 6.1 — Run targeted scope scenarios.** On the retirement PR, prove
  that a messenger-cli-only change schedules the messenger-cli package cell
  without a prefix flag, a `sniff` change schedules the exact Cargo-derived
  Rendezvous/Claudine closure, and an unrelated change schedules no
  messenger- or Rendezvous-specific job beyond ordinary impacted package
  cells.

- [ ] **Task 6.2 — Inspect every newly owned environment (parallelizable by
  environment after the promotion run starts).** Review messenger,
  messenger-cli, rendezvous-core, rendezvous-client, rendezvous-daemon, and the
  relevant claudine-cli cells on Ubuntu, Windows, macOS, and WSL2. Record D-Bus
  provisioning, helper sidecar use, Windows named-pipe behavior, JUnit upload,
  producer status, and final verdict state for each scheduled cell.

- [ ] **Task 6.3 — Prove fixture-delivery invariants from logs/artifacts.** Show
  one native stub prebuild occurs before nextest, no native test process starts
  Cargo, the WSL archive job publishes all six Linux stubs, the guest has no
  Cargo or rustc, `MESSENGER_STUB_BIN_DIR` points to the ext4 sidecar directory,
  and all desktop helper tests execute successfully from that directory.

- [ ] **Task 6.4 — Resolve first-run failures under R6.** Fix failures caused by
  policy, fixture, archive, or orchestration changes. For independently owned
  pre-existing failures only, add narrowly keyed entries to
  `.github/ci/ci-baseline.toml` with the observed package, environment, tier,
  `source_run`, owner, reason, and expiry. Add no entry for passing cells and do
  not re-exclude any package or environment.

- [ ] **Task 6.5 — Verify merge-blocking evidence.** Confirm a controlled or
  observed messenger/Rendezvous failure appears as a package-keyed FAIL cell
  and makes `ci-verdict` fail unless its exact valid baseline entry matches.
  Confirm successful replacement cells publish both JUnit and producer-status
  artifacts and that waiting through `needs` is not the source of the verdict.

- [ ] **Task 6.6 — Final acceptance audit.** Check all ten acceptance criteria
  against the final diff and promotion-run artifacts; update
  `implementation-notes.md` with run IDs and outcomes, confirm the two workflow
  files are absent, confirm no replacement SID redaction exists, and verify the
  atomic change is ready to merge with no intermediate retirement state.

- [ ] **Validation checkpoint 6 — Closure gate.** The work is complete only
  when every scheduled native and WSL2 cell either passes or has an honest R6
  baseline entry, `ci-verdict` is green, active documentation matches the
  package-grid authority, and the only remaining specialized workflow entries
  are the explicitly out-of-scope survivors.
