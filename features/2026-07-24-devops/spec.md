---
status: draft
reviewed: true
reviewed_by: claude/default
reviewed_on: 2026-07-24
---

# Reliable CI/CD and DevOps Specification

## Summary

Rusty Biscuit's CI currently produces many red workflow runs without reliably
answering the question that CI exists to answer: whether a change is safe to
merge or release. Bootstrap failures, release automation, compilation,
linting, test harness availability, test timeouts, and benchmarks are presented
as similar-looking failures across several workflows. A single shared-tooling
change can fan out to dozens of jobs before the shared tooling has proved that
it can initialize a clean runner.

This specification establishes a staged, dependency-aware CI/CD architecture
with these properties:

1. A fresh checkout can run Cargo without optional acceleration tools.
2. Shared bootstrap and platform prerequisites are verified before package
   jobs fan out.
3. Required CI uses controlled tool versions and a deliberate nextest profile.
4. Test tiers select only tests whose required resources have been provisioned.
5. Package scope remains dependency-aware and every workspace package has an
   explicit CI owner or exemption.
6. Test validation, release automation, and scheduled performance work produce
   distinct and understandable signals.
7. Failures preserve the first useful error and avoid multiplying one root
   cause into a wall of red jobs.

The implementation is phased so that trustworthy signal is restored before
larger workflow consolidation or test-suite tuning begins.

## Evidence and Current State

The investigation on July 24, 2026 found the following.

### Bootstrap failures

Commit `5a9d473294ab0d5992110e7e8bcdaeeeb9b2e633` added a repository-wide
`rustc-wrapper = "kache"` configuration. Several workflows did not install
`kache` before invoking Cargo.

Observed failures included:

- the primary CI scope job failing while `affected_scope.py` called
  `cargo metadata`;
- all three Rendezvous operating-system jobs failing during their first Cargo
  command;
- the Windows Ctrl+C job failing before its test could run.

The representative runner error was:

```text
could not execute process `kache ... rustc -vV` (never executed)
program not found
```

These were bootstrap failures. They provide no evidence about whether package
tests passed or failed.

### Release failure

Release-plz failed independently while calculating the next version for
`schematic-definitions`. Its temporary checkout modified
`schematic/Cargo.lock`, then could not return to `main` because checkout would
overwrite that dirty file.

Update (2026-07-24, during Phase 1 implementation): `**/Cargo.lock` has been
gitignored repository-wide since 2026-04-08, so neither the root nor the nested
`schematic/Cargo.lock` is tracked. `git check-ignore` confirms both are ignored;
neither appears in `git ls-files`. Under that policy git does not surface a
regenerated lockfile in `git status --porcelain` and does not block a branch
checkout on it, so the specific checkout-overwrite failure described above cannot
recur as originally stated. The remaining, still-valid problem is ordering:
release automation races the validation it should follow. This reframes OQ3
below; its resolution is recorded there.

Release automation currently runs on every push to `main`, so this release
failure appears beside test workflows even though it has a different purpose
and root cause.

### Matrix size and scope

`cargo metadata --no-deps --format-version 1` currently reports 72 workspace
packages. The checked-in statement that the workspace has 48 members is stale
and must not be used for CI calculations.

The primary CI policy contains 18 curated areas. A full-scope invocation
currently expands to 84 area jobs before specialized workflows are counted.
Changes beneath `.cargo/` or `just/`, and changes to other global files, select
the full scope because those files can affect every package-area recipe.

Full scope is sometimes correct. The problem is launching the entire fan-out
before validating the shared bootstrap path.

### Test and lint failures

The preceding full matrix exposed several distinct failure classes:

- `biscuit-speaks` test and lint jobs both failed on the same unused variable;
- Playa's Linux jobs could not find the native ALSA development library;
- new Clippy diagnostics from a floating stable toolchain failed otherwise
  unrelated areas;
- Claudine ran approximately 3,963 tests in one unsharded Linux job and stopped
  after a timeout with most tests unexecuted;
- multiple Darkmatter shards repeatedly hit a generic 30-second timeout under
  contention;
- Claudine generator drift tests timed out on all four attempts;
- the L2 workflow provisioned tmux but selected WezTerm-only tests, then
  hard-failed because `WEZTERM_UNIX_SOCKET` was unavailable.

### Runner behavior

The workflows define a `profile.ci` nextest profile, but do not explicitly
select it. Runner output showed the default profile's five-second slow-test
threshold. Both profiles currently retry failures three times, so deterministic
compile, assertion, and timeout failures can run four times before reporting
the same result.

The scheduled benchmark run shown as canceled ran for approximately its
30-minute job timeout. A timeout reported as cancellation is not equivalent to
a test regression and should not share the required-CI signal.

## Goals

### G1: Restore trustworthy signal

A primary CI run must distinguish bootstrap, build, lint, test, harness,
release, and benchmark failures. A failure before test execution must never be
presented as evidence that tests failed.

### G2: Preserve dependency-aware scope

CI must continue to calculate changed packages, their reverse Cargo dependency
closure, and the affected curated package areas. Routine gates must not fall
back to a bare workspace-wide Cargo command.

### G3: Make clean checkout behavior portable

The repository must compile and run its supported workflows on macOS, Windows,
and Linux. Optional build acceleration must not be a prerequisite for Cargo,
editors, release tools, or package discovery.

### G4: Match tests to provisioned capabilities

L1, L2, L3, browser, real-resource, and slow tests must retain their existing
tier meanings. A workflow may hard-require only a capability it has explicitly
provisioned and verified.

### G5: Bound fan-out and diagnostic cost

Shared failures must stop before launching dependent package jobs. Heavy areas
must be sharded or otherwise bounded so one slow test cannot hide most of an
area's results.

### G6: Separate validation from automation

Required change validation, release publication, nightly benchmarks, coverage,
and fuzzing must be visibly and operationally distinct.

## Non-goals

- This feature does not weaken support for macOS, Windows, or Linux.
- It does not replace nextest with another test runner.
- It does not convert routine final verification into an unscoped workspace
  build or test.
- It does not make Windows failures soft by default for packages whose
  cross-platform contract requires Windows to gate.
- It does not run L3 tests on unattended desktops or permit terminal/browser
  tests to steal focus.
- It does not fix every currently failing product test as part of the workflow
  reorganization. Product defects discovered after the infrastructure is
  trustworthy remain owned by their affected package areas.

## Decisions

### D1: Optional compiler caches must be opt-in

The repository must not commit a global `rustc-wrapper` that names an optional
binary.

The supported execution order is:

```text
install and verify kache
        ↓
activate RUSTC_WRAPPER=kache
        ↓
invoke Cargo
```

Jobs that use `kunobi-ninja/kache-action` may allow the action to install,
place, and activate the pinned wrapper. A local developer command may activate
the wrapper only after `_ensure-kache` has succeeded. Jobs that do not opt into
caching invoke `rustc` directly.

Missing `kache` must reduce performance, not make repository discovery,
compilation, testing, release automation, or editor integration impossible.

### D2: Kache configuration must have one version authority

The pinned kache version must have one authoritative repository value consumed
by local bootstrap and GitHub Actions. Local recipes, documentation, and
workflows must not independently drift to different versions.

Today the local authority is `KACHE_VERSION := "0.8.0"` in the root `justfile`
(consumed by `_ensure-kache`), while GitHub Actions delegate version selection
to `kunobi-ninja/kache-action`. These are two independent authorities that can
drift. The implementation must collapse them to one: either the action reads the
justfile-exported value, or both read a single dedicated file (for example
`.github/kache-version` or a repository variable) that the justfile also
consumes. The chosen mechanism must fail loudly — not silently fall back to a
floating version — if the two sides disagree.

The implementation must verify the installed version before exporting
`RUSTC_WRAPPER`. Cache keys must include all compatibility inputs required by
kache, including its version, target platform, Rust toolchain, and any other
inputs required by the action's contract.

Kache's cache is an optimization. A cache miss, reservation collision, or
unsupported compiler invocation may use the documented pass-through behavior.
A wrapper installation or executable-integrity failure is a bootstrap failure.

### D3: CI must have a bootstrap preflight

The primary workflow gains a preflight stage before package-area fan-out.
Preflight must validate:

- checkout and Git history requirements;
- the selected Rust toolchain;
- `cargo metadata --no-deps --format-version 1`;
- canonical `just` recipes;
- affected-scope unit tests and scope calculation;
- required common tools such as `just`, nextest, and `protoc`;
- the kache installation and version only when caching is enabled;
- platform-specific shell assumptions on every selected runner OS.

At minimum, bootstrap is exercised on Linux, Windows, and macOS whenever a
global CI/tooling file can affect those platforms. Package jobs depend on the
successful preflight result.

Preflight OS breadth scales with scope so that healthy package-local changes do
not pay a triple-OS penalty:

- A package-local change runs preflight on the scope-calculation host (Linux)
  plus only the operating systems its selected areas actually require under
  `full_os`/`soft_os` policy.
- A change to a global CI/tooling input (see D11) runs preflight on all three
  operating systems before any fan-out, because such a change can break
  bootstrap on a platform none of the changed packages name directly.

Package jobs depend on the successful preflight result for the operating systems
they target.

If preflight fails, no dependent area matrix is launched. The workflow must
report one concise bootstrap failure per affected OS instead of dozens of
package failures.

### D4: Area jobs must be staged

Each affected area uses an explicit dependency order:

```text
bootstrap
    ↓
area build/check and lint
    ↓
L1 test shards
    ↓
optional L2 and browser gates
```

A deterministic area compile or lint failure must prevent redundant test shards
from recompiling and reporting the same source error. The implementation may
combine compatible prerequisite work when doing so preserves clear attribution
and cross-platform coverage.

This staging must not turn all areas into one fail-fast chain. Independent areas
continue after another area fails.

### D5: Required CI must use a controlled Rust toolchain

Required pull-request and `main` validation must run on a repository-controlled
Rust version rather than silently changing whenever the `stable` channel moves.
The exact pin and update mechanism must be explicit and reviewable.

The concrete location of that pin is a load-bearing decision, not an
implementation detail, because `rust-toolchain.toml` currently pins only
`channel = "stable"` and every workflow overrides it with
`dtolnay/rust-toolchain@stable`. That same `stable` floating channel is the
documented cause of rustfmt drift poisoning `main`↔branch merges (see the repo
CLAUDE.md "Formatting" section). Pinning an exact version therefore also
stabilizes rustfmt and Clippy, but it changes what toolchain local developers
resolve. The mechanism is resolved in Open Question OQ1.

A separate scheduled or manually dispatched compatibility workflow tests the
current latest stable toolchain. Latest-stable failures are advisory until the
repository deliberately advances the required version and fixes new compiler
or Clippy findings.

Toolchain advancement must include:

- `cargo fmt --check`, never write-mode formatting;
- affected package build, test, and lint gates;
- review of newly enabled compiler and Clippy diagnostics;
- updates to documentation that describes the toolchain policy.

### D6: CI must explicitly select and simplify its nextest profile

Every CI test command must explicitly select the CI nextest profile. The
workflow must not rely on implicit environment detection.

The CI profile must:

- disable blanket retries for deterministic L1 tests;
- use narrowly scoped retries only for documented resource-sensitive tests
  such as terminal/PTY teardown races;
- retain leak detection;
- emit JUnit results;
- define intentional timeout budgets by test class rather than continually
  adding unrelated one-test exceptions;
- run without fail-fast inside a bounded shard when the additional diagnostic
  value does not risk exceeding the job budget.

Retries are not a substitute for isolation. A test that requires retries must
have a comment or configuration note identifying the contested resource or
external instability.

### D7: Heavy areas must be sharded by measured cost

Claudine and Darkmatter must use nextest sharding sized from observed test count
and duration. A shard should be small enough that one timeout does not suppress
most of the area's evidence and should remain comfortably within the job
timeout on a cold runner.

Initial implementation should:

- add Claudine L1 sharding;
- reevaluate Darkmatter's current four-shard policy using archived durations;
- keep shard count as area policy in `.github/ci/areas.json`;
- avoid mixing L2, browser, or real-resource tests into L1 shards;
- publish per-shard JUnit artifacts with stable, collision-free names.

Shard count is policy, not a permanent constant. Changes require evidence from
comparable runner class, operating system, request shape, and test profile.

### D8: Test selection must express required capabilities

The `level2_` tier alone is not sufficient to state which terminal backend a
test requires. L2 selection and harness enforcement must distinguish at least:

- headless tmux/PTY tests;
- WezTerm remote-control tests;
- Kitty remote-control tests;
- Apple Terminal tests;
- other future backend-specific tests.

The implementation may use stable backend identifiers in test/module names,
package-local nextest filters, or equivalent machine-readable metadata. It must
not select every `level2_` test and then globally declare L2 required when only
tmux was provisioned.

For each selected backend:

1. provision or connect the backend without taking foreground focus;
2. verify runtime reachability, not merely executable installation;
3. set the hard-require contract only for that backend;
4. run the matching filter;
5. clean up only resources created by the test harness.

Backend tests that were not selected must be excluded or skip cleanly. L3
remains opt-in and must never run unattended. Browser tests remain headless.

### D9: Native prerequisites belong to area policy

An area's CI policy must declare native packages needed to compile or test that
area on each operating system. Playa's Linux audio configuration, for example,
must either install the ALSA development package or select a feature set that
does not claim to test native ALSA support.

Native setup must be:

- scoped to areas that need it;
- represented in the reusable workflow rather than copied into unrelated
  workflows;
- cross-platform, using each OS's supported installation mechanism;
- validated before build and test commands begin;
- documented in the owning area's dependency documentation.

A missing system library is an environment-provisioning failure, not a product
test failure.

### D10: CI ownership must cover the Cargo workspace

`cargo metadata --no-deps --format-version 1` is the workspace source of truth.
No checked-in package count may drive scope calculation.

Every workspace package must be one of:

- owned by exactly one curated CI area;
- explicitly covered by a specialized workflow with a named owner; or
- explicitly exempted with a reason and replacement evidence.

The affected-scope test suite must fail if a workspace member is added without
one of these outcomes. It must also fail for duplicate ownership, nonexistent
package names, invalid area directories, or unsupported platform policy.

"Platform policy" here refers to the per-area fields already present in
`.github/ci/areas.json`: `full_os` (the exhaustive OS matrix an area must gate
on) and `soft_os` (operating systems whose failures are advisory for that area).
The ownership validator must treat these as first-class:

- An area's `full_os`/`soft_os` entries must name only supported runner OSes.
- `soft_os` must not list an OS that the area's cross-platform contract requires
  to gate. This encodes the corresponding non-goal ("does not make Windows
  failures soft by default …") as a testable rule rather than a convention.
- New capability policy introduced by this spec — shard counts (D7), backend
  requirements (D8), native prerequisites (D9), and canary membership (D11) —
  is declared in the same per-area records so that one file is the single
  policy surface. Adding an area without required capability fields, or with an
  unknown field, fails validation.

The root documentation and existing matrix-testing specification must be
updated where they still state stale workspace or area counts.

### D11: Global scope remains accurate but gains canaries

Changes to genuinely global inputs such as Cargo workspace configuration,
shared `just` recipes, shared nextest configuration, and the reusable area
workflow may continue to select every affected area.

Before full fan-out, global changes run a small canary set that exercises:

- a small pure-Rust area (candidate: `biscuit-hash`);
- a native-dependency area (candidate: `playa`, once its ALSA provisioning is
  fixed under D9);
- a heavy/sharded area (candidate: `darkmatter` or `claudine`);
- the three supported operating systems where relevant.

Canary membership is declared as area policy in `.github/ci/areas.json` (a
per-area `canary` flag), not hard-coded in the workflow, so the set evolves with
the same review path as shards and platform policy. The candidates above are the
recommended initial selection, not a permanent constant.

Canaries supplement bootstrap and catch shared recipe errors that metadata alone
cannot detect. A canary failure prevents the remaining full-scope fan-out.
Canary success does not replace full dependency-derived validation.

Documentation-only files must not select full scope unless they are executable
inputs to code generation, testing, packaging, or release automation.

### D12: Specialized workflows must be orchestrated coherently

The repository should present one primary validation run per commit. Specialized
runtime gates may remain reusable workflows, but the primary orchestrator calls
them when affected scope and platform policy require them.

Examples include:

- Rendezvous native IPC runtime verification;
- Claudine Windows Ctrl+C verification;
- captured-stdout and desktop-feature checks;
- package-specific browser or terminal contracts.

Specialized checks must not independently duplicate shared setup or broad L1
coverage already owned by area CI. Their names must describe the unique
contract they verify.

### D13: Release automation must follow successful validation

Release-plz must not run concurrently with unvalidated `main` changes. It should
be triggered after the required primary CI workflow succeeds on `main`, or by an
equivalent mechanism that enforces that dependency. The concrete trigger
mechanism is resolved in Open Question OQ2.

Release calculation must be hermetic:

- begin from a clean checkout;
- define the role of nested `schematic/Cargo.lock` (resolved in Open Question
  OQ3);
- use locked/frozen Cargo behavior where compatible with release-plz;
- detect and report any file mutation before a branch checkout;
- either prevent expected generated lockfile changes or isolate them in a
  disposable worktree whose lifecycle does not conflict with release-plz;
- finish with a clean working tree unless the workflow intentionally creates a
  release commit.

Release failures must appear under a release-specific workflow and must not be
labeled as test failures.

### D14: Benchmarks, coverage, and fuzzing remain separate

`bench-nightly` must be a true scheduled/manual performance workflow. If a
push-triggered performance smoke check is desired, it receives a different
workflow and job name.

Benchmark jobs must:

- use a timeout based on measured cold-run duration;
- split suites whose combined duration exceeds the budget;
- distinguish benchmark execution failure from optional result-upload failure;
- preserve comparable runner and toolchain identity in results.

Coverage and fuzzing remain their own scheduled or affected-scope workflows.
Their failures must not be visually conflated with required L1 validation.

### D15: CI output must summarize scope and failure class

The primary workflow summary must report:

- event, base, and head revisions;
- changed files used for scope calculation;
- affected packages and reverse dependencies;
- affected and exempted areas;
- whether scope is full and why;
- canary selection;
- selected OSes, shards, and optional tiers;
- toolchain and kache versions;
- failure class: bootstrap, build, lint, L1, L2/harness, browser, release,
  benchmark, coverage, or fuzz.

Job names follow a stable hierarchy such as:

```text
CI / claudine / L1 / ubuntu / shard 2 of 4
CI / claudine / L2 tmux / ubuntu
CI / playa / lint / ubuntu
```

Raw logs and JUnit artifacts remain available, but the summary should identify
the first actionable error without requiring inspection of every matrix leg.

Node runtime deprecation warnings and cache reservation collisions must not be
reported as root causes when a later step contains the actual failure. Action
version upgrades should be handled as separate maintenance work.

### D16: Verification scope must follow repository policy

Every implementation phase must begin by:

1. using GitNexus impact analysis for each changed symbol;
2. using `sniff` to record affected packages, package areas, and downstream
   consumers;
3. recording the verification scope before running gates.

Final gates use affected package-area `just build`, `just test`, and `just lint`
recipes, with exact selectors where supported. Workflow contract tests and
scope-calculation tests are included whenever their inputs change.

An explicitly documented CI aggregation test may exercise the full curated
matrix. Routine local verification must not use a bare root Cargo lifecycle
command or an unscoped root `just` lifecycle recipe.

## Open Questions

These three forks are load-bearing design decisions rather than implementation
details. Each carries a recommendation, but each should be confirmed before the
phase that depends on it begins. The Delivery Plan below assumes the recommended
option in each case.

### OQ1: Where does the required Rust toolchain pin live? (gates Phase 1, D5)

The required-CI toolchain must stop floating with `stable`. The pin's location
determines whether it also fixes the documented rustfmt-drift hazard and whether
local developers are forced onto the pinned version.

- **Option A — Pin an exact version in `rust-toolchain.toml`** (e.g.
  `channel = "1.89.0"`), and drop the `@stable` override from required
  workflows so they honor the file.
  - Pros: one source of truth for local and CI; rustup auto-installs it so
    local builds match CI exactly; **stabilizes rustfmt and Clippy, directly
    curing the `main`↔branch fmt-drift poisoning** called out in CLAUDE.md;
    already the file every tool reads.
  - Cons: every contributor is moved onto the pinned toolchain on next build;
    the latest-stable advisory workflow must override the file explicitly.
- **Option B — Keep `rust-toolchain.toml` at `stable`; pin only in workflows**
  via a repository variable consumed by `dtolnay/rust-toolchain@<version>`.
  - Pros: local developer experience is unchanged; the pin is CI-only and easy
    to bump in one place.
  - Cons: local ≠ CI, so a contributor's `stable` rustfmt still drifts from
    CI's — the fmt-drift hazard survives; two toolchain concepts (local
    "stable", CI "pinned") to reason about.
- **Option C — Introduce a dedicated `.github/rust-version` file** consumed by
  both a `rust-toolchain.toml`-generating step and the workflows.
  - Pros: single authoritative value; keeps `rust-toolchain.toml` generated,
    not hand-edited.
  - Cons: adds a generation/verification step and a new failure mode; more
    moving parts than Option A for the same outcome.

**Recommendation: Option A.** It is the only option that also resolves the
rustfmt-drift problem the repo already documents as actively poisoning merges,
and it keeps local and CI toolchains provably identical. The "forces local
upgrade" cost is a one-time `rustup` install and is exactly the behavior a
controlled toolchain is meant to produce. The advisory latest-stable workflow
(already required by D5) supplies the escape hatch for testing newer compilers.

### OQ2: How is release automation gated on successful CI? (gates Phase 0, D13)

Release-plz currently runs on every push to `main`, beside — and racing — the
validation it should follow.

- **Option A — `workflow_run` trigger:** the Release-plz workflow triggers on
  `workflow_run` of the primary CI workflow with `conclusion == success` on
  `main`.
  - Pros: native GitHub dependency; no change to the primary workflow;
    release cannot start until CI is green.
  - Cons: `workflow_run` workflows execute from the default-branch definition
    (a known footgun for editing the trigger); slightly indirect to reason
    about in the Actions UI.
- **Option B — Branch protection + required status check, release-plz gated on
  the merged PR:** keep release-plz on `pull_request: closed`/`merged`, and
  rely on branch protection to guarantee `main` only advances through green CI.
  - Pros: no new trigger; the "release" job already keys off a merged, labeled
    PR; branch protection is the intended enforcement point.
  - Cons: the `push`-triggered `release-pr` job still races; requires branch
    protection to be configured and kept correct out-of-band (not visible in
    the repo).
- **Option C — Fold release into the primary CI workflow** as a final job
  `needs:`-gated on all validation jobs.
  - Pros: strongest ordering guarantee, fully in-repo and reviewable.
  - Cons: couples release cadence to the CI workflow lifecycle; complicates the
    "one primary validation run per commit" story (D12) by mixing publication
    into validation; larger blast radius for CI edits.

**Recommendation: Option A.** It expresses the dependency the spec actually
wants ("release follows successful required CI") directly and in-repo, without
depending on branch-protection state that is invisible to reviewers, and without
entangling publication with validation as Option C does. The `workflow_run`
default-branch footgun is documented and one-time; note it in a workflow comment
per the Required Documentation Updates.

### OQ3: What is the role of nested `schematic/Cargo.lock`? (gates Phase 0, D13)

`schematic/schema` is excluded from the workspace, so `schematic/` carries its
own `Cargo.lock` that release-plz's transient checkout mutated, which then
blocked its return to `main`. The lockfile's tracked status must be decided.

- **Option A — Commit and pin it; run release-plz with locked/frozen Cargo:**
  keep `schematic/Cargo.lock` tracked and require `--locked` so no operation
  rewrites it mid-run; any needed update is a deliberate, reviewed commit.
  - Pros: reproducible resolution for the excluded crate; the mutation that
    caused the failure cannot recur under `--locked`; matches how workspace
    lockfiles are already treated.
  - Cons: contributors must remember to update it deliberately when
    `schematic/schema` deps change; release-plz must support `--locked` for the
    step that touched it.
- **Option B — Gitignore it:** stop tracking `schematic/Cargo.lock` entirely.
  - Pros: nothing to dirty, so the checkout conflict disappears.
  - Cons: loses reproducible builds for the excluded crate; a library-style
    lockfile omission that undercuts the reproducibility the rest of the repo
    relies on; CI would resolve fresh each run.
- **Option C — Isolate release-plz in a disposable worktree:** let release-plz
  operate in a throwaway worktree whose lifecycle never touches the primary
  checkout's `schematic/Cargo.lock`.
  - Pros: leaves the tracked-vs-ignored question untouched; contains any
    mutation to a directory that is discarded.
  - Cons: more workflow machinery; the excluded-crate lockfile can still drift
    silently since nothing pins it; interacts with release-plz's own worktree
    assumptions (the very thing that failed).

**Recommendation: Option A.** The failure was a *dirty-file-blocks-checkout*
race, and `--locked` plus a committed lockfile removes the mutation at its
source while preserving reproducible resolution for the excluded crate —
consistent with how the workspace lockfile is handled. Option C only relocates
the mutation without pinning it; Option B trades a real reproducibility
guarantee for convenience. If a specific release-plz step genuinely must
regenerate the lockfile, wrap only that step in Option C's disposable worktree
as a targeted supplement to A, not a replacement.

**Resolution (2026-07-24, implemented in Phase 1 — supersedes the recommendation
above).** During implementation the premise of Option A proved stale. The
workspace lockfiles are *not* tracked: `**/Cargo.lock` has been gitignored since
2026-04-08 (see the "Update" note under "Release failure"). Option A's
justification — "consistent with how the workspace lockfile is already treated" —
therefore does not hold, and committing only `schematic/Cargo.lock` would create a
lone tracked lockfile inconsistent with the rest of the repository. `--locked`
would also fail with no committed lockfile to honor. The confirmed choice is
**Option C, adapted to the existing ignore policy**:

- Keep every `Cargo.lock` gitignored; commit no lockfile. The ignore policy is
  itself what isolates the failure — a regenerated `schematic/Cargo.lock` is
  invisible to `git status`/`git checkout`, so it cannot dirty tracked state or
  block the return to `main`. No literal disposable worktree is required, because
  an ignored file cannot affect tracked state in the first place.
- Gate release calculation on a successful `ci` `workflow_run` (OQ2 Option A),
  removing the race that was the other half of the observed failure.
- Assert a clean *tracked* worktree (`git status --porcelain
  --untracked-files=no`) before and after release calculation, so any mutation of
  a *tracked* file surfaces as a release-specific failure.
- A workflow contract test asserts `**/Cargo.lock` stays ignored, protecting this
  premise against a future force-add that would reintroduce the original failure.

**Scope change.** Phase 1 / Task 1.4 therefore does *not* add `--locked` and does
*not* track `schematic/Cargo.lock`. Reproducible dependency resolution for the
workspace-excluded `schematic/schema` crate remains unpinned — but that is the
pre-existing, deliberate repository posture (the root lockfile is ignored too),
unchanged by this feature rather than newly regressed. Acceptance Criterion 27
("release-plz begins from a clean checkout and does not fail because
`schematic/Cargo.lock` became dirty") is satisfied by the ignore policy plus the
tracked-worktree assertions rather than by a committed, `--locked` lockfile.

## Delivery Plan

### Phase 0: Restore basic CI signal

1. Remove the mandatory repository-wide `rustc-wrapper`.
2. Activate kache only in jobs and local commands that installed and verified
   it.
3. Add the bootstrap preflight and make area jobs depend on it.
4. Add regression checks proving that Cargo metadata works without kache on a
   clean runner.
5. Repair or isolate Release-plz's mutation of `schematic/Cargo.lock`.
6. Trigger release automation only after required CI succeeds.

**Exit condition:** a clean checkout can calculate scope on all supported
operating systems, and one missing optional tool cannot create package-test
failures.

### Phase 1: Make required gates deterministic

1. Introduce the controlled required-CI Rust toolchain.
2. Add the latest-stable advisory workflow.
3. Explicitly select the CI nextest profile.
4. Remove blanket L1 retries and retain only evidence-backed scoped retries.
5. Add Claudine sharding and recalibrate Darkmatter shards.
6. Publish collision-free JUnit output for every shard.
7. Stage area build/lint before test fan-out.

**Exit condition:** deterministic failures run once, heavy suites finish within
their budgets, and a failed shard does not hide most of an area's tests.

### Phase 2: Align resources and test tiers

1. Add capability-specific L2 selection.
2. Provision and verify only the terminal backends selected in CI.
3. Ensure L2 and browser jobs remain headless and focus-safe.
4. Represent native OS prerequisites in area policy.
5. Fix Playa's Linux ALSA provisioning or explicitly narrow the tested feature
   contract.
6. Audit existing tests whose names place them in a tier that does not match
   their actual resource requirements.

**Exit condition:** no test hard-fails merely because CI selected a broader
tier than it provisioned.

### Phase 3: Complete scope ownership and orchestration

1. Validate every Cargo workspace member against area ownership or exemption.
2. Remove stale hard-coded workspace and area counts from documentation.
3. Add global-change canaries.
4. Move specialized runtime gates behind the primary orchestrator.
5. Remove duplicated setup and overlapping broad test coverage.
6. Add stable workflow/job naming and summaries.

**Exit condition:** one commit produces one coherent validation run whose scope
can be explained from Cargo dependencies and policy.

### Phase 4: Separate and harden scheduled automation

1. Make benchmark workflows scheduled/manual or rename any push-triggered
   performance checks.
2. Rebudget and split benchmarks using measured cold-run duration.
3. Keep upload failures distinct from benchmark execution failures.
4. Confirm coverage and fuzz workflows use distinct names, schedules, and
   artifacts.
5. Add periodic audits for toolchain, GitHub Action, cache, and runner-image
   updates.

**Exit condition:** scheduled automation is actionable without polluting the
required test signal.

## Acceptance Criteria

### Bootstrap and kache

1. On fresh Linux, Windows, and macOS runners with no `kache` executable,
   `cargo metadata --no-deps --format-version 1` succeeds.
2. A cache-enabled job verifies the pinned kache version before its first Cargo
   invocation through the wrapper.
3. A non-cache job has no `RUSTC_WRAPPER=kache` requirement.
4. Simulating a missing or invalid kache installation fails a named bootstrap
   step and launches no dependent area jobs.
5. Cache misses and reservation collisions do not fail otherwise successful
   builds.

### Scope and ownership

6. Scope calculation derives package membership from Cargo metadata.
7. Adding an unmapped workspace package fails an ownership-policy test with the
   package name and expected remediation.
8. Adding duplicate ownership or an invalid package name fails policy
   validation.
9. A package-local change selects that package, its reverse dependencies, and
   their owning areas.
10. A documentation-only change selects no build/test area unless the document
    is an executable input.
11. A shared-workflow or shared-recipe change selects bootstrap, canaries, and
    then the correct full affected scope.
12. A canary failure prevents non-canary full-scope jobs from starting.

### Toolchain, lint, and tests

13. Required CI reports one exact Rust toolchain version.
14. Latest stable runs separately and cannot silently change required-CI
    behavior.
15. CI logs identify the selected nextest profile.
16. A deterministic failing L1 test executes once.
17. A resource-sensitive retry is configured only by a scoped override with a
    documented reason.
18. Claudine and Darkmatter shards finish within their measured job budgets on
    a cold Linux runner.
19. JUnit artifacts from different areas, OSes, and shards do not overwrite one
    another.
20. A compile or lint failure prevents redundant area test shards from
    reporting the same source failure.

### Harness and platform prerequisites

21. A tmux-only L2 job selects tmux-capable tests and does not hard-require
    WezTerm, Kitty, or Apple Terminal.
22. A WezTerm job verifies remote-control reachability before declaring the
    backend required.
23. L2 and browser gates create no focused window on any runner or developer
    host.
24. L3 remains disabled unless explicitly and safely authorized.
25. Playa's Linux native-audio gate either provisions ALSA development headers
    or clearly tests a non-ALSA feature contract.
26. Missing native prerequisites fail a named provisioning step rather than a
    product test.

### Release and scheduled automation

27. Release-plz begins its calculation from a clean checkout and does not fail
    because `schematic/Cargo.lock` became dirty during its own operation.
28. Release automation starts only after required CI succeeds on `main`.
29. A release failure is shown as release automation, not test validation.
30. Nightly benchmark timeout budgets exceed measured cold-run duration with a
    documented margin, or the suite is split.
31. Optional Bencher upload failure does not erase a successful benchmark
    execution result.
32. Coverage, fuzzing, benchmark, release, and required-CI results have distinct
    workflow names and summaries.

### Diagnostics

33. The primary workflow summary records the calculated package/area scope and
    why full scope was or was not selected.
34. A shared bootstrap regression produces at most one actionable failure per
    selected OS before fan-out.
35. A package regression identifies its area, gate, OS, shard, and first
    actionable error.

## Required Documentation Updates

Implementation must update, as applicable:

- `docs/testing-strategy.md`;
- repository and area dependency documentation for native tools;
- `.claude/skills/rust-testing/`;
- `.claude/skills/rust-devops/kache.md`;
- the `.github/ci/areas.json` policy documentation, covering every field the
  ownership validator enforces — existing (`check_args`, `soft_os`, `full_os`,
  `shards`, `l2`, `browser`) and newly introduced by this spec (`canary`,
  backend requirements, and native-prerequisite declarations);
- root contributor/agent guidance containing stale workspace counts;
- the earlier matrix-testing specification where its current-state statements
  have drifted;
- workflow comments whose described behavior no longer matches the code.

Behavior-changing workflow edits require a pass over adjacent comments. Code is
authoritative when a comment has drifted; stale comments must be corrected or
removed in the same change.

## Risks and Trade-offs

### Staged jobs can increase latency when everything is healthy

Preflight and prerequisite dependencies add serial stages. This is accepted
because they prevent much larger wasted fan-out when shared infrastructure is
broken. The stages should remain small and cache-neutral where possible.

### Controlled toolchains require an update process

Pinning required CI trades surprise upgrades for deliberate maintenance. The
latest-stable advisory workflow prevents the pin from becoming invisible debt.

### Removing blanket retries may initially expose more flakes

That exposure is intentional. Resource-sensitive retries remain available when
they are scoped and justified. Unexplained flakes must be fixed through
isolation, capability selection, or timeout design rather than hidden globally.

### More precise L2 selection adds policy surface

Backend-aware filters and provisioning add configuration. They replace the
current ambiguous contract in which a generic L2 requirement can demand a
backend that the runner cannot provide.

### Consolidation must retain specialized evidence

Moving specialized workflows behind one orchestrator must not weaken native IPC,
Windows console-control, captured-stdout, or other unique runtime contracts.
Only duplicated setup and overlapping broad coverage should be removed.

## Success Measure

This feature is successful when a red required-CI run reliably means that an
affected, required validation contract failed; a green run means that the
dependency-derived scope passed on its required platforms; and release,
benchmark, coverage, and fuzz failures remain independently understandable.

## Implementation Decisions (2026-07-25 session)

These refine D9 and D11 based on what implementation and live CI runs revealed.

### Native libraries: one isolated install step, shared with `just init` (refines D9)

`.github/ci/areas.json` `native` is the single source of truth for the OS system
libraries an area needs (e.g. Playa's Linux ALSA/PulseAudio). The **install
logic must have one implementation**, shared between developer hosts and CI:

- The root `justfile` exposes an **isolated library-install recipe**
  (`_ensure-native-libs`) that reads `areas.json` `native` for the current OS,
  probes each library with `pkg-config`, installs only what is missing, and maps
  the apt package names to `dnf`/`pacman`/`apk`/`brew` on other hosts. It is a
  prerequisite of `just init` but is **isolated** so it can run on its own.
- **CI must run that isolated library-install step before anything is built** —
  i.e. before the area's `check`/`test`/`lint` (and any specialized) build
  commands, so a `-sys` crate never fails to compile for a missing system lib.
  Running the full `just init` in CI is out of scope (it also installs CLIs,
  kache, GitNexus, etc.); only the isolated library-install step runs.
- The current per-area `install-native` composite action duplicates this logic.
  Consolidate to the single recipe so a new `native` requirement reaches both
  developer hosts and CI from one declaration and one installer.

### Canaries must be green areas (refines D11)

Canaries only provide signal when the canary area is otherwise green — a canary
must fail only because a *shared* change broke it, not because the area had
pre-existing product failures. A red canary blocks all global-change fan-out
(D11) and is worse than no canary.

- Keep the canary **mechanism** and the per-area `canary` flag.
- Initial canaries: **`biscuit-hash`** (pure-Rust) and **`playa`** (native-dep;
  now green on `main`). **`darkmatter`** stays out of the canary set until its
  L1 tests are green, then it can be re-added as the heavy/sharded canary.
- **Do not** use `homelab` or `research` as canaries for now.

### Quality-of-life: CI-aware `just commit` (new, optional)

`just commit` uses an LLM (`claudine compose`), audio (`_speak`), and an
interactive-ish flow that is inappropriate in CI. When `CI` is set it should
instead perform a plain, deterministic, non-interactive `git commit` (no LLM, no
audio, no network), with the message supplied by the caller. Local behavior is
unchanged.
