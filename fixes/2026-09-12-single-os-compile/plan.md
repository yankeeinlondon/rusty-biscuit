---
title: Compile once per compatible configuration across test tiers
status: blocked
created: 2026-09-12
phase: 1
total_phases: 7
agent: codex/default
yolo: true
spec: fixes/2026-09-12-single-os-compile/spec.md
depends_on:
  - fixes/2026-09-11-cicd-cleanup/spec.md
blocked_on: fixes/2026-09-11-cicd-cleanup/plan.md
---

# Compile Once per Compatible Configuration Execution Plan

## Outcome

For every executing L1, L2, or browser cell, the canonical resolved plan names
one compatible, run-scoped build record. One native owner produces that
record's immutable Nextest archive and declared sidecars, and every consumer
verifies and executes those exact outputs without invoking Cargo, rustc,
Clippy, or a linker. Result identity remains
`{package, environment, gate}`; build records remain internal plumbing keyed by
`{package, producer environment, build}`.

The implementation is complete only when macOS, Linux, native Windows, and
WSL2 satisfy the same archive contract, Linux and WSL2 consume the same Linux
artifact, check and lint behavior is unchanged, failure attribution remains
area-owned, and matched measurements show no unapproved regression outside the
repository's 15% noise band.

## Execution constraints

- The CI-cleanup specification is a hard prerequisite. Its canonical plan,
  per-cell evidence overlay, package-keyed artifacts, area-owned rollups, and
  `ci-gate` must be complete before this plan changes their schemas or
  workflows. Phase 1 stops if that is not true; it does not absorb or duplicate
  the prerequisite.
- `scripts/ci/affected_scope.py` remains the sole scheduling authority. Build
  derivation happens only after evidence overlay and may not reopen affected
  scope, package policy, tier policy, baseline policy, or accepted-gap policy.
- Preserve the four-level chain
  `ci.yml -> _area-ci.yml -> _package-ci.yml -> _wsl-ci.yml`. Build owners must
  be sibling jobs in an existing workflow or shared script/composite logic,
  never another reusable-workflow level.
- Use Nextest and the canonical `just` tier recipes. Do not use `cargo test`,
  do not run `cargo fmt`, and do not add a workspace-wide package gate solely
  because CI infrastructure changed.
- Keep `RUSTC_WRAPPER` empty by default. A command-scoped, plan-identified
  measurement wrapper is allowed only around measured producer invocations;
  no `.cargo/config.toml` or host-global wrapper is introduced.
- Human-facing CLI diagnostics must use `TerminalRenderable` components.
  Machine interfaces remain versioned JSON with stable rejection codes.
- Archive, fixture, L2, and browser validation must be headless or backgrounded
  and must never focus, activate, or close a user-owned terminal or browser
  window.
- Phases 4 and 5 are one atomic workflow migration. An intermediate branch may
  use a non-authoritative shadow archive for inventory/timing comparison, but
  no merged cell may choose between old and new results or silently fall back
  to compilation.

## Phase 1 — Clear prerequisites and establish the current baseline

- [ ] **Task 1.1 — Prove the CI-cleanup prerequisite is complete.** Audit the
  dependency specification and active implementation at one recorded revision.
  Confirm that the resolved plan is the only scheduler, evidence is overlaid
  per cell, every store remains package-keyed, every selected area owns its
  rollup, and `ci-gate` is the policy-free required-context fold. Record the
  criterion-by-criterion result in `implementation-notes.md`, update this
  plan's `status` to `ready` only after every prerequisite is satisfied, and
  stop this plan if any prerequisite remains blocked.

- [ ] **Task 1.2 — Refresh code-intelligence evidence before source edits.** Run
  a successful current GitNexus index, then run upstream impact analysis for
  `calculate_scope`, `apply_accepted_cells`, `validate_resolved_plan`, the
  workflow-facing matrix projection, the rollup plan reader, and runner-loss
  classification. Record HIGH/CRITICAL risks and all lower-bound/UNKNOWN
  boundaries in `implementation-notes.md`; confirm incomplete graph edges with
  direct search before editing. Do not treat the failed/stale index observed
  during planning as change-safety evidence.

- [ ] **Task 1.3 — Freeze the compile-path inventory (parallelizable with Task
  1.4 after Tasks 1.1–1.2).** Map every current L1, L2, browser, WSL2 archive,
  runner-tool, companion-suite, native-library, build-script-output, fixture,
  and non-test-binary path from the resolved package record through its
  workflow and canonical recipe. Record which configurations are compatible,
  which must remain separate, and which package-specific files need archive
  includes or sidecars. The inventory must call out the current absolute WSL2
  checkout-path coupling and every runtime Cargo build.

- [ ] **Task 1.4 — Add a cross-platform compiler-work counter
  (parallelizable with Task 1.3 after Tasks 1.1–1.2).** Add a narrowly featured
  `ci-build` binary and tests under `scripts/` that can re-enter as a
  command-scoped rustc wrapper, write one collision-free event file per
  compiler invocation, and aggregate actual compiler calls and elapsed build
  windows by package/configuration. Keep the feature out of the no-default-
  features `ci-rollup` build. Use `biscuit-hash` rather than a new hashing
  implementation, use `TerminalRenderable` for human diagnostics, and update
  `scripts/Cargo.toml`, `Cargo.lock`, and `docs/dependencies.md` together.

- [ ] **Task 1.5 — Instrument the old schedule without changing ownership.** In
  the existing L1, L2, browser, and WSL2 archive paths, set the wrapper only for
  the measured Cargo/Nextest command and publish compiler-work counts plus
  queue, setup, compile, archive, upload/download, extraction, and test timing
  fields. Extend workflow contracts to prove the wrapper is never global, the
  old result cells and artifacts are unchanged, and cache restoration is
  labeled only as a producer optimization.

- [ ] **Task 1.6 — Capture matched cold and warm baselines.** Write
  `baseline-2026-09-12.md` with the exact revision, runner images/tool
  versions, test identities, feature arguments, cache condition, compiler-work
  counts, per-stage durations, total runner compute, and end-to-end critical
  path. Collect three consecutive green observations per environment for both
  controlled cold and warm conditions; report WSL2 build time under the Linux
  producer and WSL2 execution separately.

- [ ] **Validation checkpoint 1 — Prove the baseline did not alter
  scheduling.** Run all Python CI contract suites, the `ci-build` and
  `ci-rollup` Nextest binaries, the `ci_workflow_contracts` Nextest suite, and
  `actionlint` over every changed workflow. Compare the before/after resolved
  plans byte-for-byte except for the explicitly versioned measurement fields,
  and confirm all existing `{package, environment, gate}` cells and outcomes
  are unchanged.

## Phase 2 — Make build ownership part of the canonical plan

- [ ] **Task 2.1 — Freeze the versioned build-record schema.** Add failing or
  pending-first fixtures in `test_schema.py`, `test_affected_scope.py`, and
  `test_resolved_plan.py` for plan-level build records, cell-to-build
  references, unhashed identity fields, producer/execution compatibility,
  consumer lists, artifact identity, and build-owner projections. Advance the
  resolved-plan schema version and regenerate
  `.github/ci/schemas/contract.json`; older scope receipts must miss cleanly as
  `scope-schema`, never be partially upgraded.

- [ ] **Task 2.2 — Declare compile compatibility as data
  (parallelizable with Task 2.1 after Phase 1).** Extend
  `.github/ci/environments.json` with one build contract for each native
  producer: observed compiler host, explicit target triple, Cargo profile,
  Cargo/config and encoded-flag inputs, archive format, pinned Nextest version,
  and compatible execution environments. Make Linux-to-WSL2 the only
  cross-environment compatibility edge and validate architecture, ABI/libc,
  native-library, and tool-version predicates. Native Windows and WSL2 must be
  structurally impossible to pair.

- [ ] **Task 2.3 — Define and compute the planned build key.** Canonically
  serialize the exact source tree and `Cargo.lock`, pinned Rust/Cargo/Nextest
  inputs, compiler host and target, linker/config/flags known at plan time,
  Cargo profile, package/target selection, isolated feature graph,
  build-script/native inputs, archive format, includes, and sidecar
  declarations. Compute its xxHash through the `ci-build` boundary backed by
  `biscuit-hash`; store both the digest and unhashed fields, fail if the helper
  is unavailable, and never substitute Python's `hash()`, SHA, or a second
  implementation.

- [ ] **Task 2.4 — Derive build records only after evidence overlay.** Update
  both fresh selection and `apply_accepted_cells` so an executing L1, L2, or
  browser cell references exactly one build; cells satisfied by evidence or
  governed omission create no consumer demand. Deduplicate equal planned keys,
  assign one native owner, list every consumer, and remove a record when its
  last consumer is satisfied. Keep lint and check cells build-reference-free
  and preserve their existing compile coverage.

- [ ] **Task 2.5 — Reject invalid ownership before workflows run.** Extend
  `validate_resolved_plan` to reject dangling build references, unconsumed
  builds, duplicate owners/keys, incompatible producer/consumer environments,
  missing or unsorted consumers, artifact-name collisions, a test execution
  without a build, and any build attached to lint/check. Add positive fixtures
  for mixed reuse/execute/gap plans and for one Linux build serving distinct
  Linux and WSL2 result cells.

- [ ] **Task 2.6 — Project owners without re-planning.** Extend the legacy
  scope/workflow projection and `ci-plan` renderer with a deterministic native
  build-owner matrix and per-package build references derived only from the
  final plan. Show planned key, producer, consumers, and compatibility reason
  in the terminal plan while keeping area and package result identities
  unchanged.

- [ ] **Validation checkpoint 2 — Exercise plan invariants.** Run the schema,
  scope, resolved-plan, evidence-reuse, local-plan, and plan-renderer suites.
  Include fixtures proving an all-reused plan schedules no owner, mixed
  environments schedule only demanded owners, L1/L2/browser consumers share
  one compatible key, changed feature/flag/target inputs split keys, and no
  unchanged reverse-dependent package or area becomes scheduled.

## Phase 3 — Produce immutable, relocatable archives

- [ ] **Task 3.1 — Implement the producer/manifest contract.** Extend
  `ci-build` with a producer command that reads only its owner slice from the
  resolved plan, executes package archives in deterministic order in one
  target tree, and writes one archive plus manifest per planned key. Use a
  separate Cargo invocation per package unless the Phase 6 feature-graph
  fixture proves a combined invocation equivalent. The manifest must contain
  the planned key, realized build digest, all identity fields, source commit
  and tree, package, producer and compatible environments, tool/linker/native
  versions, archive/sidecar sizes and BLAKE3 checksums, expected test-binary
  inventory, runtime assets, compiler-work counts, and stage timings.

- [ ] **Task 3.2 — Make includes and sidecars declarative.** Extend the existing
  closed package CI metadata vocabulary with validated archive includes and
  named build sidecars; do not add an arbitrary shell-command field. Move
  compile-time runner tools such as messenger desktop stubs and the darkmatter
  CLI fixture into this producer-owned contract, list every emitted file in
  the manifest, and keep runtime-only tools/services on consumers.

- [ ] **Task 3.3 — Implement strict consumer verification
  (parallelizable with Task 3.2 after Task 3.1's manifest format is frozen).**
  Add a `ci-build verify` command that checks plan key, realized digest,
  source/tree identity, producer and execution compatibility, archive and
  sidecar checksums/sizes, expected binaries, required assets, and discovered
  runtime ABI/native libraries before extraction or test startup. Missing,
  corrupt, incomplete, extra, or incompatible inputs must return a stable
  infrastructure rejection; verification must never compile a replacement.

- [ ] **Task 3.4 — Generalize every canonical test recipe to archive mode.** In
  `just/devops.just`, make L1, L2, and browser paths accept the same standalone
  Nextest/archive/workspace-remap inputs while retaining their filtersets,
  JUnit staging, backend proof, browser requirement, concurrency, timeouts,
  and teardown. Archive mode must avoid `cargo metadata` and ignore compile-
  only package/feature flags already baked into the archive without weakening
  the selected tier.

- [ ] **Task 3.5 — Add a portable archive fixture workspace.** Create a focused
  checked-in fixture under `scripts/ci/fixtures/` whose archive contains L1,
  L2-marker, and browser-marker test binaries, a non-test executable, a
  dynamic library, build-script output, a repository fixture, an archive
  include, and each declared sidecar class. Its integration test must build in
  one checkout, hide or rename the producer target directory, copy source to a
  different checkout path, extract elsewhere, and run all three filtersets
  from the same archive checksum with Cargo/rustc/linker absent from `PATH`.
  The fixture proves archive plumbing, not availability of a production
  terminal/browser backend.

- [ ] **Task 3.6 — Remove compile-time path assumptions exposed by the
  fixture.** Audit test code for `env!("CARGO_BIN_EXE_...")`, producer target
  paths, and fixture lookups rooted only in compile-time
  `CARGO_MANIFEST_DIR`. Replace binary lookup with
  `biscuit_test_harness::bin_exe!`/`NEXTEST_BIN_EXE_*`, and resolve fixtures
  through runtime workspace remapping or explicit archive includes. Update
  affected symbol docs/comments in the same edit and record every remaining
  intentional compile-time path.

- [ ] **Validation checkpoint 3 — Prove relocation and tamper detection.** Run
  the `ci-build` integration suite on the host plus canonical archive-mode L1,
  L2, and browser fixture recipes. Inject missing inventory, altered archive,
  altered sidecar, wrong source tree, wrong target, and unavailable-toolchain
  cases and prove each fails before tests start. Verify no test opens or
  focuses a terminal/browser window.

## Phase 4 — Cut Linux and WSL2 over to one Linux owner

- [ ] **Task 4.1 — Add the top-level native build-owner job.** In `ci.yml`, add
  a fail-fast-false matrix driven by the plan's owner projection. For the Linux
  slice, install the union of build-time native prerequisites, restore the
  producer cache, run `ci-build produce` once, and upload package-keyed
  `build-<package>-<producer>-<key>` artifacts with run-scoped retention.
  Process keys deterministically and continue after an individual build
  failure so unrelated keys can complete.

- [ ] **Task 4.2 — Publish build status without creating result cells.** Emit
  one build-status artifact per planned key for success, compile failure,
  archive failure, upload failure, and cancellation, including realized digest
  and timings where available. Extend `ci-rollup` and runner-loss attribution
  so a dependent result cell becomes visibly blocked/missing by the named
  build record, never PASS or baseline-eligible, while an unrelated area or
  environment proceeds.

- [ ] **Task 4.3 — Make native Linux tiers pure consumers.** Change Linux L1,
  L2, and browser jobs in `_package-ci.yml` to download their one build
  artifact, run strict verification, provision only runtime facilities, and
  call the canonical tier recipe in archive mode. Preserve companion-suite
  execution and all JUnit/status identities. Assert the three consumers report
  one planned key and realized digest and contain no Cargo, rustc, Clippy, or
  linker invocation.

- [ ] **Task 4.4 — Reuse the exact Linux artifact in WSL2.** Remove the package-
  local archive producer and builder-path clone workaround from `_wsl-ci.yml`.
  Download the same artifact/checksum used by native Linux, copy the source and
  archive into independent guest paths, verify guest architecture/libc/dynamic
  libraries plus manifest compatibility, prove the unprivileged guest has no
  Cargo/rustc, and execute the canonical L1 recipe. Keep the WSL2 JUnit/status
  cell distinct from Linux.

- [ ] **Task 4.5 — Pin Linux/WSL2 failure isolation contracts.** Extend workflow
  and rollup fixtures for Linux producer compile failure, cancellation, missing
  artifact, corrupt transfer, consumer setup failure, and one unrelated
  successful key. Prove no fallback build occurs, a real test result outranks
  plumbing diagnostics when it exists, setup/upload failures remain
  infrastructure failures, and `ci-gate` still delegates cell policy to each
  area rollup.

- [ ] **Validation checkpoint 4 — Demonstrate one Linux build, two
  environments.** On an exact revision, show one Linux archive checksum and
  realized digest feeding native Linux L1 and WSL2 L1, plus Linux L2/browser
  wherever those cells execute. Hide the producer target and checkout path,
  compare test identities with the old path, and confirm the two environments
  publish separate result cells. Do not merge this cutover independently of
  Phase 5.

## Phase 5 — Cut macOS and native Windows over to native owners

- [ ] **Task 5.1 — Enable macOS and Windows owner slices.** Feed the same
  top-level owner matrix with the demanded macOS and native-Windows records,
  preserving native prerequisite installation, shells, executable suffixes,
  dynamic-library lookup, target directories, and runner-specific target
  triples. Validate the actual compiler host/target against the plan before
  compiling rather than inferring compatibility from `runner.os`.

- [ ] **Task 5.2 — Convert macOS consumers
  (parallelizable with Task 5.3 after Task 5.1).** Route macOS L1 and each
  hostable L2/browser cell through verified archives while preserving tmux
  provisioning, background/headless guarantees, backend proof, and policy gaps
  for unavailable GUI backends. Prove no consumer restores a Cargo target
  cache or installs/uses a Rust toolchain.

- [ ] **Task 5.3 — Convert native-Windows consumers
  (parallelizable with Task 5.2 after Task 5.1).** Route native Windows L1 and
  any hostable higher-tier cell through the Windows owner's archive. Cover
  `.exe` sidecars, drive/UNC/verbatim path spellings, `USERPROFILE`-based home,
  DLL discovery, and handle cleanup; never use the WSL/Linux artifact or
  override the build host's configured target directory.

- [ ] **Task 5.4 — Keep check and lint deliberately separate.** Leave Clippy
  and check-only example/bench/dependent-seam commands in their existing
  result-producing jobs with their existing feature and target selection.
  Update compile-reason reporting so those invocations are labeled distinct
  configurations rather than unexplained duplicates; retain L1 as the compile
  coverage source for lib/bin/test targets.

- [ ] **Task 5.5 — Complete workflow dependency and gate wiring.** Make area
  consumers wait for relevant owner completion under `always()`/`!cancelled()`
  semantics without allowing one failed owner leg to cancel other legs. Add
  the owner job to `ci-gate` only for unrepresented infrastructure failure;
  keep build/test command outcomes in package cells and area rollups. Preserve
  all accepted-gap publishing permissions and the existing reusable-workflow
  depth.

- [ ] **Validation checkpoint 5 — Prove the native contract on all three
  producers.** Run workflow contracts and the archive fixture on macOS, Linux,
  and native Windows. For each producer, show one archive per planned key,
  identical key/digest/checksum across its compatible consumers, no compiler
  process in consumers, unchanged test/JUnit identities, and independent
  progress when another producer fails. Treat cross-compilation as compile
  evidence only; use hosted or declared native build hosts for behavioral
  proof.

## Phase 6 — Consolidate dependency work and align local/cross-host execution

- [ ] **Task 6.1 — Prove shared dependency reuse without feature
  unification.** Add a two-package fixture with one identical dependency
  configuration and a deliberately divergent feature/flag configuration. Run
  each package as a separate archive invocation in one owner target tree.
  Use compiler-work events to prove the identical dependency compiles once,
  the incompatible unit compiles twice, and each package's tests observe only
  its isolated feature graph. Permit a combined Cargo invocation only if this
  fixture remains equivalent; otherwise keep deterministic separate
  invocations.

- [ ] **Task 6.2 — Align same-host local orchestration.** Update
  `just/ci-local.just` so a same-process L1/L2/browser sequence reuses one
  compatible target configuration and reports its planned key even when it
  does not serialize an archive. When execution is separated into another
  process/path, use the same `ci-build` producer, manifest, verifier, and
  archive-mode recipes as CI. Preserve per-cell validation receipts and rerun
  semantics for previously failing evidence.

- [ ] **Task 6.3 — Align cross-host orchestration
  (parallelizable with Task 6.2 after Phase 5).** Update
  `scripts/cross-check.sh` to transfer the immutable archive and manifest,
  verify compatibility on the destination, hide the producer target, use a
  different checkout/extraction path, and record the same build key/digest in
  exact-tree receipts. Keep SSH non-interactive with `BatchMode=yes`, retain
  each host's target-directory rules, and publish no reusable receipt for a
  filtered or dirty-tree run.

- [ ] **Task 6.4 — Close the archive/runtime inventory.** Exercise real
  package examples for dynamic libraries, non-test binaries, build-script
  output, repository fixtures, messenger stubs, darkmatter CLI fixtures,
  harness broker/tooling, and companion suites. For every discovered missing
  class, add a declarative include/sidecar plus a negative test; do not repair
  it with a consumer-side Cargo command.

- [ ] **Task 6.5 — Remove superseded paths and caches.** Delete the old WSL2
  archive job, absolute-path workaround, consumer `rust-cache` steps,
  consumer toolchain setup, and any temporary shadow build after every
  replacement contract is proven. Add source-string/workflow tests that fail
  if a test consumer invokes Cargo/rustc/Clippy/linker or if an artifact miss
  enters a build fallback.

- [ ] **Validation checkpoint 6 — Run the full fault and parity matrix.** Run
  planner/schema, artifact-tool, recipe, workflow, rollup, runner-loss, local-
  evidence, and cross-check tests. Demonstrate producer failure/cancellation,
  missing/corrupt artifacts, wrong ABI, missing native library, consumer setup
  failure, and successful unrelated cells. Confirm L2/browser runs stay
  headless/backgrounded and leave no owned processes, panes, or windows.

## Phase 7 — Document, measure, and complete rollout

- [ ] **Task 7.1 — Complete reporting surfaces.** Ensure producer queue,
  compile, archive, and upload timings and consumer download, extraction, and
  execution timings appear separately in status artifacts and summaries.
  Every executing test cell must display planned key, realized digest, and
  producer; result artifacts, receipts, baselines, and JUnit records must
  remain keyed only by `{package, environment, gate}`. Update `ci-plan` and
  `ci-rollup` with `TerminalRenderable` tables/prose rather than ad hoc ANSI
  output.

- [ ] **Task 7.2 — Update live documentation with each final contract.** Update
  `.github/ci/README.md`, `.github/ci/schemas/README.md`,
  `docs/topics/ci-cd.md`, testing documentation, and any affected package
  README. Update `.claude/skills/rust-devops/ci-cd.md`,
  `.claude/skills/os/wsl.md`, `.claude/skills/os/ci-runners.md`, and
  `.claude/skills/rust-testing/SKILL.md` with the owner/consumer, relocation,
  compatibility, failure, local, and measurement rules. Remove comments that
  describe the retired builder-path/cache behavior; do not preserve drift as
  historical guidance.

- [ ] **Task 7.3 — Run the compact final validation suite.** Run
  `python3 -m py_compile scripts/ci/*.py`, every `scripts/ci/test_*.py` suite,
  `.githooks/tests/test-pre-push.sh` under the repository's documented clean
  environment, Nextest for the `ci-build`, `ci-rollup`, and `ci-plan` bins,
  `cargo nextest run -p test-toolkit --test ci_workflow_contracts`, targeted
  Clippy for changed Rust tooling, and `actionlint` over every shipped
  workflow. Run `just ci-local --plan` and confirm the final matrix honors any
  recorded execution constraints. Do not add a full-workspace package run
  unless dependency-derived impact identifies one.

- [ ] **Task 7.4 — Collect post-cutover cold and warm measurements.** On the
  same test identities/configurations as Phase 1, collect three consecutive
  green runs per environment in controlled cold and warm conditions. Report
  compiler-work counts, total runner compute, and critical path separately;
  identify transfer/setup cost rather than folding it into test time. Treat a
  delta within 15% as noise.

- [ ] **Task 7.5 — Apply the measured architecture gate.** If any environment's
  critical path regresses by more than 15%, stop and record an explicit design
  ruling. Prefer deterministic planner-declared compatibility cohorts, retain
  one owner per build key, and report dependencies duplicated across cohorts.
  Do not silently return to one owner per package or introduce a remote cache.
  If measurements accept the single-owner design, record that decision and
  keep cohorts out.

- [ ] **Task 7.6 — Close the acceptance ledger.** Write
  `rollout-2026-09-12.md` mapping every specification acceptance criterion to
  a test, artifact, run ID, checksum/digest, or documented measurement. Record
  evidence separately for macOS, Linux, native Windows, and WSL2; distinguish
  compile evidence from behavioral evidence and list any governed capability
  gap without narrowing the criterion.

- [ ] **Validation checkpoint 7 — Final change and rollout review.** Run
  GitNexus `detect-changes --scope all` and re-run non-partial analysis for any
  HIGH/CRITICAL affected process before commit preparation. Review the final
  diff for accidental area-keyed stores, changed gate/baseline semantics,
  duplicate owners, unconsumed archives, undocumented dependencies, stale
  comments, consumer compilers, and reusable-workflow depth. The plan is
  complete only when the acceptance ledger is fully evidenced and no required
  work remains.
