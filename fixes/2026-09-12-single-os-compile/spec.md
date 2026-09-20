---
created: 2026-09-12
status: draft
reviewed: true
reviewed_by: codex/default
reviewed_on: 2026-09-12
implemented: false
review_iterations: 3
area: repository-ci
depends-on:
  - fixes/2026-09-11-cicd-cleanup/spec.md
---

# Compile Once per Compatible Configuration Across Test Tiers

> **Reader's note (review, 2026-09-12).** The original draft correctly
> separated build reuse from test-result reuse, but left the artifact and
> orchestration model as implementation questions. This revision chooses
> run-scoped Nextest archives, makes build ownership part of the canonical
> resolved plan, and retains package-keyed artifacts and result cells. It also
> narrows “compile once” to equivalent compiler invocations: Clippy, check-only
> targets, incompatible feature graphs, and different target ABIs remain
> distinct builds by design.

## Problem

During PR #76's CI cleanup validation, Ubuntu completed the Claudine L1 tests
and then spent substantial time compiling the same test programs again for L2
on another runner. The same structure exists for browser tests and for other
packages and native environments. A separate WSL2 job already avoids this by
building a Nextest archive on `ubuntu-latest` and executing it in the guest.

Restoring `target/` through `Swatinem/rust-cache` is an optimization, not a
producer/consumer contract. Repository cache pressure routinely makes a job
cold, and an incomplete cache can cause every tier to rebuild. Kache remains a
host opt-in and is deliberately disabled in CI; this specification does not
reverse that ruling or introduce a remote compiler cache.

Repeated compilation delays feedback and consumes runner capacity. More
importantly, the current jobs cannot prove whether a consumer executed the
producer's exact binaries or silently built replacements.

## Objective

For each selected package, compile each compatible test configuration once per
workflow run and supply its immutable outputs to every required execution tier.
Reuse compatible dependency outputs while producing those package archives,
without broadening affected scope or changing Cargo feature resolution.

The design applies to hosted CI and to the repository's local and cross-host
orchestration. It covers macOS, Linux, native Windows, and WSL2. WSL2 remains a
distinct execution environment even when it consumes the Linux build.

This is not literally “once per OS.” A build configuration is compatible only
when all compile-affecting inputs agree, including:

- source tree and `Cargo.lock`;
- Rust toolchain, compiler host, target triple, linker, Cargo profile, Cargo
  configuration, encoded Rust flags, and relevant environment variables;
- package and target selection, resolved feature graph, build-script inputs,
  generated outputs, proc macros, and native-library/linker inputs; and
- archive format and Nextest version.

Architecture, ABI, or any input above can require another build on the same OS.
Conversely, an `x86_64-unknown-linux-gnu` archive may serve both an x86_64 GNU
Linux runner and WSL2 when the plan proves their runtime ABI and native-library
requirements are compatible. Native Windows and WSL2 are never compatible.

## Scope Boundaries

In scope:

- L1, L2, and browser tiers whose only compile-time difference today is their
  runtime Nextest filter;
- non-test binaries, build-script outputs, dynamic libraries, fixtures, and
  package-declared sidecars required by those test binaries;
- plan, workflow, status, timing, and diagnostic changes needed to make the
  producer/consumer relationship explicit; and
- relocation fixes in tests or fixtures exposed by archive-only execution.

Out of scope:

- changing package selection, tier definitions, platform support, baseline
  policy, accepted-gap policy, or `ci-gate` semantics;
- treating a build as passing test evidence;
- sharing native binaries across incompatible target triples or ABIs;
- release artifacts, doctests, fuzzing, Level 3, and real-resource suites not
  currently represented by the normal package workflow; and
- persistent compiled-artifact reuse across workflow runs or commits. Cargo
  caches may still warm a producer, but cross-run build provenance and
  retention are a separate measured design. Existing qualifying validation
  evidence continues to suppress execution across commits.

## Terminology

- **Result cell:** the existing `{package, environment, gate}` policy and
  evidence identity. This remains unchanged.
- **Build record:** a run-scoped plan record describing one immutable compile
  configuration, its producer, package archives, and dependent result cells.
  It is build plumbing, not a result cell and not baseline-eligible.
- **Planned build key:** a canonical digest of every plan-known
  compile-affecting input listed in the Objective. Human-readable OS labels are
  diagnostic fields, never keys.
- **Realized build digest:** the producer's immutable manifest digest. It adds
  discovered inputs that the planner cannot know, such as the actual linker
  and native-library versions, plus the archive and sidecar checksums.
- **Producer environment:** the native environment that compiles an archive.
- **Execution environment:** the environment in which a result cell runs. It
  may differ from the producer only where an explicit compatibility rule, such
  as Linux-to-WSL2, permits it.
- **Consumer:** a test-tier job that verifies a build manifest, extracts an
  archive, provisions runtime facilities, and executes tests without Cargo or
  rustc.

## Design Decisions

### 1. Resolve evidence before scheduling builds

The canonical resolved plan remains the sole scheduling authority. Apply
qualifying per-cell validation evidence first, then derive build records only
for remaining `execute` cells. If all consumers of a build key are already
satisfied by evidence or are governed omissions, no producer is scheduled.

Each executing L1, L2, or browser cell references exactly one planned build
record. Each build record lists every consuming cell. Plan validation rejects
dangling references, unconsumed builds, duplicate owners for one planned build
key, and a consumer whose environment is incompatible with the producer. The
producer does not rewrite the canonical plan: it publishes the realized build
digest in its manifest and status, and consumers verify that the discovered
inputs satisfy the plan's constraints.

### 2. Use one native build owner per producer environment

Start with one build-owner job on each selected native producer environment:
`ubuntu-latest`, `macos-latest`, and `windows-latest`. The owner reads the
resolved plan and invokes Cargo only for selected packages and required build
keys. It must not recalculate scope or perform an unconditional workspace
build.

Within an owner, package archives are built in deterministic order in one
target tree. Cargo may therefore reuse an exact dependency configuration across
packages while retaining separate artifacts for incompatible feature or flag
sets. Packages remain separate Cargo invocations unless a fixture proves a
combined invocation preserves the isolated resolved feature graph. This avoids
silently enabling a union of features.

The owner emits one package archive per planned build key. Artifact names remain keyed
by `{package, producer environment, build}`; `build` is an internal artifact
tier, not a new result-cell gate. Area is never an artifact identity. A single
owner may upload many package archives, so shared dependency compilation does
not require a cross-package artifact name.

This intentionally trades some build parallelism for lower compilation count.
The rollout measurements below are the gate: if one owner creates a material
critical-path regression, split the environment into planner-declared cohorts.
Any split must retain a single owner for each build key and must report
dependency configurations compiled in more than one cohort; returning to one
isolated build per package is not an acceptable optimization.

### 3. Use Nextest archives for executable test outputs

Nextest archives are the standard bundle for L1, L2, and browser consumers.
They contain test programs, Cargo and binary metadata, relevant dynamic
libraries, non-test binaries used by integration tests, and supported
build-script output. Package-specific files not covered by Nextest are declared
as archive includes or explicit sidecars and listed in the build manifest.

Every archive has a machine-readable manifest containing at least:

- planned build key, realized build digest, and all unhashed identity fields
  used to produce them;
- source commit and tree, package, producer environment, target triple, and
  compatible execution environments;
- archive and sidecar checksums and sizes;
- Rust, Cargo, Nextest, linker, and relevant native dependency versions; and
- the expected test-binary inventory and required runtime assets.

Consumers verify the manifest, checksums, source tree, target compatibility,
and expected binary inventory before execution. A consumer must not invoke
Cargo, rustc, Clippy, or a linker. A missing, corrupt, incomplete, or
incompatible bundle is a producer failure; it never triggers a fallback build.

### 4. Make archive execution genuinely relocatable

Consumers extract archives into isolated directories and execute them through
the canonical `just` tier recipes in archive mode. Those recipes continue to
own filtersets, JUnit staging, L2 backend proof, browser requirements,
concurrency, timeouts, and teardown. Runtime services and native libraries are
provisioned on the consumer rather than captured from the producer.

Tests must not require the producer's target directory or absolute checkout
path. Use runtime `CARGO_MANIFEST_DIR`, `NEXTEST_BIN_EXE_*`, workspace remapping,
archive includes, and repository helpers such as
`biscuit_test_harness::bin_exe!` as appropriate. The current WSL2 workaround
that clones the guest at the builder's literal path is transitional and is
superseded by this relocation requirement.

The source tree at the exact planned revision remains available to consumers
because Nextest archives do not contain repository source. Tests that require
fixtures must resolve them through the remapped runtime workspace or include
them explicitly. Validation must hide the producer target directory and use a
different checkout and extraction path.

### 5. Share the Linux archive with WSL2 only by contract

The `ubuntu-latest` owner produces the explicit
`x86_64-unknown-linux-gnu` archive used by both native Linux consumers and
WSL2. The build manifest must prove that the guest architecture, libc/ABI, and
required dynamic libraries satisfy that archive. A future runner-image,
architecture, distribution, or native dependency change that breaks this
predicate creates another build key or fails planning; it must not silently
move compilation into the toolchain-free guest.

Linux and WSL2 still produce separate `{package, environment, gate}` execution
results. A Linux pass is never WSL2 evidence, and native Windows binaries are
never used in WSL2.

### 6. Keep check and lint configurations distinct

`cargo check`, examples, benches, and Clippy do not consume Nextest test
archives. Clippy uses a different compiler driver and flags; check-only target
kinds may not produce executable artifacts. Their work remains in the existing
result-producing jobs and is counted as a separate configuration reason, not
as an unexplained duplicate.

L1 remains the compile-coverage source for library, binary, and test target
kinds. The existing `check` cell remains limited to uncovered example/bench
targets and the Ubuntu dependent seam. Lint and check results retain their
existing package/environment/gate identities and status uploads.

### 7. Preserve workflow depth and area-owned outcomes

The current chain `ci.yml -> _area-ci.yml -> _package-ci.yml -> _wsl-ci.yml`
already uses GitHub's four-level reusable-workflow limit. Do not insert another
reusable workflow into that chain.

Build owners must be top-level sibling jobs in `ci.yml`, direct jobs in an
existing workflow, or shared composite/script implementation that adds no
reusable-workflow depth. Area workflows wait for the relevant build-owner
jobs, then their package-tier consumers download the package artifacts.

Use fail-fast-independent producer legs. A failed macOS build must not cancel a
successful Linux build or prevent unrelated Linux/Windows consumers from
running. Every dependent cell records a visible blocked or missing reason that
names the failed build record; unaffected cells proceed. The owning area rollup
continues to apply baseline, accepted-gap, and missing-cell policy, and
`ci-gate` remains a policy-free fold.

### 8. Keep local and cross-host behavior equivalent

Local orchestration uses the same build-key calculation, archive manifest, and
archive-mode recipes when it separates build and execution across processes or
hosts. A same-process local tier sequence may reuse its target directory, but
diagnostics must still identify the one build configuration that covered its
tiers. Cross-host runs transfer immutable archives and never infer
compatibility from an OS name.

L2 and browser execution must remain headless or backgrounded as their existing
contracts require. No validation step may activate a terminal/browser window,
steal focus, or close a user-owned window.

## Failure and Reporting Contract

- Report producer queue time, compile time, archive time, upload time,
  download time, extraction time, and test execution time separately.
- Report cache restoration as an optimization of the producer, never as proof
  of build reuse. Record actual Cargo/rustc work so a warm cache is not mistaken
  for one compilation.
- Report the planned build key, realized build digest, and producer for each
  consumer cell in status artifacts and area summaries. Result artifacts,
  receipts, baselines, and JUnit records remain keyed by
  `{package, environment, gate}`.
- A failed or canceled producer cannot yield passing dependent cells. A real
  test result, when one exists, outranks a build-plumbing diagnostic just as it
  does today.
- Artifact upload/download/setup failures remain infrastructure failures and
  are never normalized as test failures or accepted by a test baseline.
- Retain run-scoped archives only as long as needed for diagnostics. They must
  not become an undocumented cross-run cache.

## Implementation Sequence

1. Add compile-work counters and matched timing measurements to the current
   workflows. Capture a cold and warm baseline without changing scheduling.
2. Extend resolved-plan schema validation with build records, build keys,
   producer/consumer references, compatibility reasons, and artifact metadata.
   Derive them only after evidence overlay.
3. Generalize the existing WSL archive passthrough so canonical L1, L2, and
   browser recipes run an archive without Cargo. Add relocation and inventory
   contract tests before moving native consumers.
4. Make the Linux owner produce the package archives for Linux and WSL2. Remove
   the package-local WSL archive build only after both consumers pass from the
   same artifact with the builder target directory hidden.
5. Add macOS and native Windows owners and consumers. Preserve platform-native
   shells, paths, prerequisites, and target directories.
6. Consolidate selected package builds within each owner to reuse compatible
   dependency configurations. Verify isolated package feature graphs before
   permitting any combined Cargo invocation.
7. Update workflow documentation, CI schema fixtures, runner-loss attribution,
   timing summaries, and the `rust-devops`, `os`, and `rust-testing` skills in
   the same change that makes each behavior live.

At every phase, the old producer remains authoritative until the new path
passes the same result-cell contract. Do not run both paths and select whichever
passes. A temporary shadow build may compare inventories and timings, but its
result cannot satisfy a cell and must be removed before rollout completes.

## Acceptance Criteria

- Plan fixtures prove that evidence-satisfied cells create no build, every
  executing test cell has one compatible build owner, and no build key has two
  owners.
- A package with L1, L2, and browser cells compiles its test programs once for
  one compatible native configuration. All three consumers run from the same
  archive checksum, and their logs contain no Cargo, rustc, Clippy, or linker
  invocation.
- A fixture with two selected packages and one shared dependency proves that an
  identical dependency configuration is compiled once in the environment
  owner. A deliberately different feature/flag configuration proves that two
  builds occur and that neither package's test behavior changes through feature
  unification.
- The same contract is demonstrated on macOS, Linux, and native Windows. The
  Linux archive also executes in an isolated WSL2 guest and produces a distinct
  WSL2 result without access to the producer target directory or producer
  checkout path.
- Archive tests cover dynamic libraries, non-test binaries, build-script output,
  repository fixtures, and each declared sidecar class. Missing inventory and
  checksum mismatches fail before tests start.
- Producer failure, cancellation, missing artifacts, and consumer setup failure
  cannot produce a passing or silently rebuilt cell. Unaffected environments
  and areas continue and report their results.
- Check-only example/bench coverage, the Ubuntu dependent seam, lint behavior,
  L1 target coverage, JUnit staging, backend proof, governed gaps, area
  rollups, and `ci-gate` retain their established semantics.
- Cold and warm comparisons use matched test identities and configurations,
  report total runner compute and end-to-end critical path separately, and
  include three consecutive green runs per environment. A delta within 15% is
  treated as noise. A regression above that range requires an explicit design
  ruling rather than being hidden by transfer time, fewer tests, or a warm
  cache.
- Relevant workflow and schema contract suites, `actionlint`, and the canonical
  affected CI-tooling tests pass. No full-workspace package run is required
  solely because CI infrastructure changed.

## Open Questions

No implementation-blocking design question remains. If measurements show that
one owner per native producer environment causes a material critical-path
regression, choose among these reviewed alternatives before changing the
architecture:

1. **Planner-declared compatibility cohorts (recommended fallback).** Split an
   environment's selected packages into a small number of deterministic
   cohorts while keeping every build key in exactly one cohort.

   - Pros: restores parallelism; keeps ownership explicit; can isolate unusually
     expensive native toolchains.
   - Cons: dependencies shared by different cohorts may compile more than once;
     cohort construction and diagnostics add planner complexity.

2. **One owner per package.** Keep the current package fan-out and pass one
   archive from that package's L1 build to its higher tiers.

   - Pros: simplest workflow change; high package-level parallelism; small
     failure domains.
   - Cons: preserves cross-package dependency recompilation and therefore does
     not meet the full objective without a reliable compiler-artifact service.

3. **Remote compiler-artifact service.** Restore exact rustc outputs through a
   shared content-addressed backend while retaining package-level owners.

   - Pros: combines parallel owners with cross-run and cross-package reuse.
   - Cons: introduces credentials, storage lifecycle, trust, availability, and
     Windows restore-cost concerns; prior GitHub-cache measurements were poor;
     cache hits still need provenance strong enough to support the compile-once
     claim.

Recommendation: use compatibility cohorts only if the required three-run
measurements reject the single-owner design. It is the smallest fallback that
preserves explicit ownership and does not reopen the repository's measured
decision to keep kache out of CI. The other options require either weakening
this specification's objective or approving a separate cache-service design.

## Relationship to Current Work

This specification depends on
`fixes/2026-09-11-cicd-cleanup/spec.md`: its canonical plan, per-cell evidence,
package/environment/gate result identity, area-owned rollups, and `ci-gate`
must be complete before build records are introduced. The cleanup does not
depend on this optimization.
