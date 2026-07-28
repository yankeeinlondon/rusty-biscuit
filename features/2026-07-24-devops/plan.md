---
total_phases: 5
created: 2026-07-24
phase: 1
agent: codex/default
yolo: true
status: "phases 1-5 implemented; PRs #5/#7/#8/#9 stacked for review"
---

# Reliable CI/CD and DevOps Execution Plan

## Planning Baseline

This plan adopts the functional specification's recommended resolutions for its
three open questions:

- Pin the required Rust version in `rust-toolchain.toml` and make required CI
  honor that file.
- Gate release automation with a successful `workflow_run` of the primary CI
  workflow on `main`.
- Keep release automation hermetic during release calculation. (Superseded
  2026-07-24: the original "keep `schematic/Cargo.lock` tracked + `--locked`"
  wording assumed a tracked lockfile, but `**/Cargo.lock` is gitignored
  repository-wide. Resolved to OQ3 Option C — keep every lockfile ignored, gate
  on `workflow_run`, and assert a clean *tracked* worktree. See the spec's OQ3
  "Resolution" block.)

The current discovery baseline is 72 Cargo workspace packages reported by
`sniff repo packages` and 18 curated area records in `.github/ci/areas.json`.
Cargo metadata remains the runtime source of truth; neither number becomes a
checked-in CI input.

Every implementation phase begins by running GitNexus upstream impact analysis
for each symbol that will change and `sniff` discovery for the affected
packages, package areas, and downstream consumers. Record that verification
scope before running package-area gates. If GitNexus reports HIGH or CRITICAL
risk, stop and review the blast radius before editing. Do not use unscoped
workspace Cargo commands or root lifecycle recipes as routine final gates.

## Phase 1: Restore Bootstrap and Release Signal

- [x] **Task 1.1: Record the Phase 1 change and verification scope.**
  - Run GitNexus impact analysis before changing any Python, Rust, or shell
    symbol and use `sniff` to identify affected packages, areas, and downstream
    consumers.
  - Record the selected package-area `just build`, `just test`, and `just lint`
    gates plus the workflow-contract and scope-calculation tests that will be
    run.

- [x] **Task 1.2: Make kache optional and establish one version authority.**
  - Remove the repository-wide `rustc-wrapper = "kache"` setting from
    `.cargo/config.toml`.
  - Move the pinned kache version to one dedicated repository value consumed by
    both the root `_ensure-kache` recipe and GitHub Actions; remove the duplicate
    literal from `.github/workflows/_area-ci.yml`.
  - Make local and CI bootstrap verify the installed executable and exact
    version before exporting `RUSTC_WRAPPER=kache`.
  - Include kache version, runner OS/architecture, Rust toolchain, and the
    action-required compatibility inputs in cache keys.
  - Add negative-path checks proving that missing/invalid kache fails a named
    bootstrap step, while a cache miss or reservation collision uses documented
    pass-through behavior.

- [x] **Task 1.3: Add dependency-aware bootstrap preflight before area fan-out.**
  - Extend `scripts/ci/affected_scope.py` and its tests to classify global
    CI/tooling inputs separately from package-local and documentation-only
    changes and to emit the preflight OS matrix and reason.
  - Add a preflight stage in `.github/workflows/ci.yml` that verifies checkout
    history, the selected Rust toolchain, `cargo metadata --no-deps
    --format-version 1`, canonical `just` recipes, scope unit tests, `just`,
    nextest, `protoc`, shell assumptions, and kache only when enabled.
  - Run Linux plus the selected areas' required `full_os`/`soft_os` platforms
    for package-local changes; run Linux, Windows, and macOS for global
    CI/tooling changes.
  - Make each area job depend on successful preflight for its target OS so a
    failed bootstrap creates at most one actionable failure per selected OS and
    launches no dependent area jobs.

- [x] **Task 1.4: Make release calculation clean, locked, and validation-gated.**
  - Update `.github/workflows/release-plz.yml` so release-PR calculation runs
    only after the primary `ci` workflow succeeds on `main`, using
    `workflow_run` with explicit branch, repository, conclusion, and source-SHA
    guards.
  - Document in the workflow that `workflow_run` executes the default-branch
    workflow definition.
  - Keep `schematic/Cargo.lock` tracked, use locked/frozen Cargo behavior where
    release-plz supports it, and add clean-worktree assertions before release
    calculation, before branch changes, and after non-publishing runs.
  - If release-plz must regenerate a lockfile, isolate only that operation in a
    disposable worktree and prove cleanup; do not weaken the tracked-lockfile
    contract.
  - Preserve the labeled merged-release-PR publication contract while ensuring
    release failures remain under release-specific job and workflow names.

- [x] **Task 1.5: Add bootstrap and release workflow contract tests.**
  - Expand `scripts/ci/test_affected_scope.py` with Windows-path normalization,
    package-local OS selection, global three-OS preflight, documentation-only
    scope, missing-kache, invalid-kache, and fan-out suppression cases.
  - Update `tools/test-toolkit/tests/phase7_ci_workflows.rs` (renaming it to a
    durable CI contract-test name if needed) to assert preflight dependencies,
    opt-in wrapper activation, single kache authority, release trigger guards,
    and clean/locked release behavior.
  - Add a clean-checkout regression that runs Cargo metadata with kache absent
    and `RUSTC_WRAPPER` unset.

- [x] **Validation checkpoint 1: Prove clean bootstrap and release ordering.**
  - Run the recorded Phase 1 package-area build, test, and lint gates and the
    Python/workflow contract tests.
  - Exercise preflight on Linux, Windows, and macOS through GitHub Actions,
    including a deliberately unavailable kache case and a healthy non-cache
    case.
  - Confirm no area matrix starts after a preflight failure and release-plz does
    not start for failed or canceled required CI.
  - Confirm release calculation leaves both root and nested Cargo lockfiles
    clean.

## Phase 2: Make Required Gates Deterministic and Bounded

- [x] **Task 2.1: Record the Phase 2 change and verification scope.**
  - Repeat symbol impact analysis and `sniff` package/package-area/downstream
    discovery for toolchain, nextest, area workflow, Claudine, and Darkmatter
    changes.
  - Record comparable cold-run runner class, OS, request shape, test count, and
    duration evidence before changing timeout or shard policy.

- [x] **Task 2.2: Pin required CI and add latest-stable advisory coverage.**
  - Replace `channel = "stable"` in `rust-toolchain.toml` with the reviewed exact
    Rust version and include every component required by build, Clippy, and
    read-only formatting checks.
  - Remove required-workflow `dtolnay/rust-toolchain@stable` overrides so the
    repository file is authoritative.
  - Add a scheduled/manual latest-stable advisory workflow that explicitly
    overrides the pin, runs `cargo fmt --check`, and executes dependency-scoped
    build, test, and lint gates without becoming a required status.
  - Document the reviewed toolchain-advancement procedure and required
    diagnostics review.

- [x] **Task 2.3: Make the CI nextest profile explicit and evidence-based.**
  - Pass `--profile ci` from every CI nextest invocation and log the selected
    profile.
  - Set blanket CI L1 retries to zero in `.config/nextest.toml`.
  - Retain retries only in narrowly scoped overrides whose adjacent note names
    the contested resource or external instability; preserve leak detection.
  - Replace generic timeout exceptions with documented test-class budgets and
    use no-fail-fast within a shard only where it remains safely inside the job
    budget.
  - Extend nextest verification tests to prove a deterministic failure runs
    once, a scoped resource-sensitive override retries as configured, and CI
    emits JUnit.

- [x] **Task 2.4: Recalculate heavy-area shards from comparable measurements.**
  - Archive cold Linux CI counts and durations for Claudine and each existing
    Darkmatter shard before changing policy.
  - Add Claudine L1 shards and update Darkmatter's four-shard setting in
    `.github/ci/areas.json` only when the measurements justify the resulting
    count.
  - Keep L2, browser, real-resource, and slow tests out of L1 partitions.
  - Publish JUnit per area, OS, gate, and shard with stable collision-free
    artifact names and retain the measurement metadata used to tune policy.

- [x] **Task 2.5: Stage area prerequisites before test fan-out.**
  - Refactor `.github/workflows/_area-ci.yml` into explicit bootstrap,
    area build/check and lint, L1 shard, and optional-tier dependencies.
  - Prevent L1, L2, and browser jobs from starting after deterministic build or
    lint failure while keeping unrelated areas independent with
    `fail-fast: false`.
  - Preserve required macOS, Windows, and Linux coverage and identify the gate
    and OS in stable job names.

- [x] **Task 2.6: Update deterministic-gate documentation and contracts.**
  - Update `docs/testing-strategy.md`, `.claude/skills/rust-testing/`, and
    `.claude/skills/rust-devops/kache.md` for the pinned toolchain, explicit CI
    profile, retry rules, shard evidence, and staged gate graph.
  - Correct or remove adjacent workflow and nextest comments that no longer
    describe behavior.

- [x] **Validation checkpoint 2: Prove deterministic, bounded required gates.**
  - Run the recorded Phase 2 package-area gates, nextest configuration tests,
    scope tests, and workflow contract tests.
  - Confirm one deterministic L1 failure executes once and prevents redundant
    test shards after build/lint failure.
  - Confirm Claudine and Darkmatter cold Linux shards finish comfortably inside
    their job budgets and every shard uploads a unique JUnit artifact.
  - Confirm required CI reports one exact Rust version and latest stable cannot
    change required-CI behavior.

## Phase 3: Align Test Tiers with Provisioned Capabilities

- [x] **Task 3.1: Record the Phase 3 change and verification scope.**
  - Run impact analysis before changing harness, recipe, or test-selection
    symbols and use `sniff` to record all harness consumers and native-dependent
    areas.
  - Record platform-specific compile checks and focus-safe L2/browser tests in
    the verification scope.

- [x] **Task 3.2: Define capability policy in `.github/ci/areas.json`.**
  - Add explicit per-area backend requirements and per-OS native prerequisites
    alongside existing L2/browser policy.
  - Define a strict policy schema with required/defaulted fields, supported
    runner OS values, and unknown-field rejection.
  - Document every enforced field: `check_args`, `full_os`, `check_os`,
    `soft_os`, `shards`, `l2`, `browser`, `kache`, AI-provider stubs, backend
    requirements, native prerequisites, and the later canary flag.

- [x] **Task 3.3: Split L2 selection by terminal backend.**
  - Introduce stable machine-readable selectors for tmux/PTY, WezTerm, Kitty,
    Apple Terminal, and future backends using names, filters, or equivalent
    metadata.
  - Add backend-specific `just` recipes/filters so a tmux job cannot select
    WezTerm-, Kitty-, or Apple-Terminal-only tests.
  - Set a hard-require contract for only the selected backend and make
    unselected backend tests exclude or skip cleanly.
  - Audit existing `level2_` tests and reclassify selectors that do not match
    their actual resource requirements without changing L1/L2/L3 semantics.

- [x] **Task 3.4: Provision and verify selected backends without taking focus.**
  - For each selected backend, provision or attach headlessly, verify runtime
    reachability rather than executable presence, export only that backend's
    contract, and clean up only resources created by the harness.
  - Keep browser execution headless and serialized where required.
  - Add focus-safety assertions for L2/browser workflows and keep L3 disabled
    unless explicitly and safely authorized.

- [x] **Task 3.5: Move native prerequisites into area policy.**
  - Teach the reusable area workflow to install and verify only the native
    packages declared for the selected area and OS, with named provisioning
    failures before build/test commands.
  - For Playa on Linux, install and verify ALSA development headers or explicitly
    configure and name a non-ALSA feature contract; do not imply native ALSA
    coverage when it is disabled.
  - Update `playa/docs/dependencies.md` (creating the area dependency document if
    absent) and other owning-area dependency docs for declared native tools.

- [x] **Task 3.6: Add capability and provisioning regression tests.**
  - Add policy tests for unknown backends, invalid OS/package-manager mappings,
    unsupported hard requirements, and missing native prerequisites.
  - Add workflow/harness contracts proving tmux-only selection excludes other
    backends, WezTerm reachability is checked before hard requirement, L3 stays
    off, and missing ALSA fails provisioning rather than product tests.

- [x] **Validation checkpoint 3: Prove resource-matched, focus-safe tiers.**
  - Run the recorded harness consumer and native-area build, test, and lint
    gates, plus policy and workflow contracts.
  - Exercise each selected L2 backend on its supported runner and verify no
    terminal or browser window gains focus.
  - Confirm a tmux-only job passes without WezTerm/Kitty/Apple Terminal and a
    missing native library produces a named provisioning failure.

## Phase 4: Complete Ownership, Canaries, Orchestration, and Diagnostics

- [x] **Task 4.1: Record the Phase 4 change and verification scope.**
  - Run impact analysis for changed scope/policy symbols and use `sniff` plus
    Cargo metadata to capture all workspace packages, owning areas, specialized
    workflow owners, exemptions, and downstream consumers.
  - Record the exact CI aggregation scenarios used to validate global canaries
    and specialized orchestration.

- [x] **Task 4.2: Enforce complete, unique workspace ownership.**
  - Extend the area-policy validator so every Cargo workspace member is owned
    by exactly one curated area, named specialized workflow, or explicit
    exemption with reason and replacement evidence.
  - Fail with actionable package names for unmapped members, duplicate
    ownership, nonexistent packages, invalid area directories, missing required
    policy fields, unknown fields, unsupported OS values, and invalid soft-OS
    contracts.
  - Keep Cargo metadata as the package source of truth and add fixtures for all
    positive and negative ownership cases.

- [x] **Task 4.3: Add policy-driven global-change canaries.**
  - Add a per-area `canary` field and select an initial pure-Rust,
    native-dependency, and heavy/sharded area based on the specification's
    candidates and measured runtime.
  - Change global scope orchestration to run bootstrap, then selected
    cross-platform canaries, then the remaining dependency-derived full scope.
  - Prevent non-canary fan-out after canary failure without allowing canary
    success to replace full validation.
  - Refine global path classification so documentation-only changes remain
    empty unless the document is an executable input.

- [x] **Task 4.4: Orchestrate specialized runtime contracts from primary CI.**
  - Convert Rendezvous native IPC, Claudine Windows Ctrl+C, captured-stdout,
    desktop-feature, browser, terminal, and other affected specialized checks
    into reusable workflows or uniquely scoped jobs called by `.github/workflows/ci.yml`.
  - Select them from affected packages and area platform/capability policy.
  - Remove duplicated bootstrap and broad L1 coverage while preserving each
    workflow's unique runtime evidence.
  - Name every job using the stable hierarchy `CI / area / gate / OS / shard`.
  - Rename the opaque `hooks-tests` workflow. It tests the `.githooks/pre-push`
    git hook and the `changed-areas` justfile recipe, but "hooks-tests" reads
    ambiguously (git hooks vs. webhooks vs. UI hooks). Rename to a self-describing
    name (e.g. `pre-push-hook-tests`) or fold it in as `CI / githooks / <gate>`.
    Note: the hook is opt-in (`core.hooksPath` defaults to `.git/hooks`, not
    `.githooks/`), so CI only verifies the script is correct.

- [x] **Task 4.5: Emit one actionable scope and failure summary.**
  - Carry event, base/head revisions, normalized changed files, affected seeds,
    reverse dependencies, owned/exempt areas, full-scope reason, canaries, OSes,
    shards, optional tiers, toolchain, and kache version through scope outputs.
  - Add primary workflow summaries that classify the first actionable failure
    as bootstrap, build, lint, L1, L2/harness, browser, release, benchmark,
    coverage, or fuzz without promoting Node warnings or cache collisions over
    later root causes.
  - Preserve raw logs and JUnit artifacts while ensuring the summary alone
    identifies area, gate, OS, shard, and first useful error.

- [x] **Task 4.6: Remove stale scope documentation and policy drift.**
  - Update `AGENTS.md`, `CLAUDE.md`, `README.md`,
    `docs/testing-strategy.md`, `docs/topics/ci-cd.md`,
    `features/2026-06-07-matrix-testing/spec.md`, and affected local skills to
    remove stale package/area counts and obsolete top-level-directory scope
    descriptions.
  - Replace hard-coded counts with Cargo-metadata/sniff discovery instructions
    where a count is not essential.
  - Review adjacent comments for every behavior-changing workflow/scope edit
    and correct or remove drifted prose.

- [ ] **Validation checkpoint 4: Prove complete scope and coherent orchestration.**
  - Run ownership/policy fixtures, affected-scope tests, workflow contract
    tests, and the recorded affected package-area gates.
  - Test package-local, shared dependency, documentation-only, unmapped package,
    duplicate ownership, invalid policy, global canary success, and global
    canary failure scenarios.
  - Run the explicitly documented curated CI aggregation test and confirm one
    coherent validation run explains every selected job and exemption.
  - Confirm each specialized contract runs only when selected and retains its
    unique platform evidence.

## Phase 5: Separate and Harden Scheduled Automation

- [x] **Task 5.1: Record the Phase 5 change and verification scope.**
  - Run impact analysis for changed automation/helper symbols and use `sniff` to
    record affected benchmark, coverage, fuzz, release, and package-area owners.
  - Capture comparable cold-run benchmark durations, runner identity,
    toolchain, suite, and upload behavior before changing budgets.

- [x] **Task 5.2: Make benchmark automation truly scheduled/manual.**
  - Remove the `push` trigger from `.github/workflows/bench-nightly.yml`; if a
    push-triggered smoke check is retained, give it a separate workflow, scope,
    timeout, and non-nightly name.
  - Split benchmark suites whose measured cold duration plus documented margin
    exceeds the job budget.
  - Record runner image/class and toolchain with results so comparisons remain
    valid.
  - Separate benchmark execution from optional Bencher upload so upload failure
    cannot erase or relabel a successful benchmark result.

- [x] **Task 5.3: Keep coverage and fuzzing operationally distinct.**
  - Audit `.github/workflows/coverage.yml` and
    `.github/workflows/fuzz-nightly.yml` for unique names, schedules or
    affected-scope triggers, timeouts, artifacts, summaries, and failure
    classes.
  - Remove duplicated required-L1 work and shared bootstrap where the primary
    orchestrator already supplies equivalent evidence.
  - Ensure coverage, fuzz, benchmark, release, and required-CI checks cannot be
    mistaken for one another in required status configuration or the Actions
    UI.

- [x] **Task 5.4: Add recurring maintenance audits.**
  - Add a scheduled/manual audit that reports, without silently updating,
    available Rust toolchain, GitHub Action, kache, nextest, and runner-image
    changes.
  - Document the review-and-pin update process, including `cargo fmt --check`,
    affected build/test/lint gates, diagnostic review, cache compatibility, and
    rollback.
  - Keep maintenance findings advisory until a reviewed repository change
    advances an authority value.

- [x] **Task 5.5: Add scheduled-automation contract tests and documentation.**
  - Assert benchmark trigger separation, measured timeout metadata,
    execution/upload result separation, distinct workflow names, and advisory
    maintenance behavior.
  - Update `docs/testing-strategy.md`, `docs/topics/ci-cd.md`, relevant
    dependency docs, and the Rust DevOps/testing skills to describe the final
    validation, release, performance, coverage, fuzz, and maintenance signals.

- [ ] **Validation checkpoint 5: Prove independent scheduled signals.**
  - Run the recorded affected package-area gates and all workflow/policy
    contract tests.
  - Manually dispatch benchmark, coverage, fuzz, and maintenance workflows and
    confirm distinct names, artifacts, summaries, and failure classes.
  - Simulate successful benchmark execution with failed upload and verify the
    benchmark result remains visible and successful.
  - Confirm the final required-CI success means the dependency-derived required
    contracts passed, while release and scheduled automation remain separately
    actionable.

