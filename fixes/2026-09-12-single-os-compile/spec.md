---
created: 2026-09-12
status: draft
reviewed: false
implemented: false
area: repository-ci
---

# Compile Once per Compatible Configuration Across Test Tiers

## Problem

During PR #76's CI cleanup validation, Ubuntu completed the Claudine L1 tests
and then spent substantial time compiling again for L2 on another runner.
Completing one tier did not give the next tier a reliable handoff of the
compiled test programs. The same structural problem applies to every package
area and every supported environment, not just Claudine or Ubuntu.

Separate jobs can restore a Cargo cache, but that is not a guarantee that
consumers receive the exact build produced earlier in the same run. A cold,
missing, incompatible, or incomplete cache can turn each test tier into
another full build. Rebuilding delays feedback, consumes runner capacity,
and makes lengthy review cycles more expensive.

The current package workflow separates L1 and L2 jobs and uses Rust cache
restoration. The WSL workflow already demonstrates a useful precedent:
build a nextest archive on Linux and execute it in the guest. This spec
requires reliable reuse across compatible consumers; it does not yet choose
the final artifact format or orchestration design.

## Goal and Scope

Compile each required build configuration once, then supply its compiled
outputs to every compatible test environment and tier that needs them.
This applies repository-wide, including nested areas such as
`claudine/rendezvous`, libraries, CLIs, macOS, Linux, native Windows, and WSL2.
It covers hosted CI and the local/cross-host test orchestration that can
otherwise recreate the same duplication.

“Once per OS” is shorthand for once per compatible configuration, not a
claim that all tests on an OS can always use identical binaries. Architecture,
target ABI, Rust toolchain, profile, feature selection, dependency resolution,
compiler flags, and build-time inputs can require distinct builds. Native
Windows and WSL2 must not be conflated. Linux and WSL2 may share compatible
Linux artifacts while still producing separate execution evidence.

The reusable object may be a test archive or another immutable build bundle.
Sharing a running VM is not required. Consumers still need their own runtime
dependencies, services, fixtures, and environment setup.

## Requirements

1. **One build owner per compatible configuration.** Resolve required targets
   before compilation. Compatible L1, L2, and other test consumers use the
   owner's outputs without invoking another compilation. Extra builds must
   have a recorded compatibility reason, not merely a different tier or area.
2. **Reliable handoff.** Make build outputs explicit producer/consumer inputs.
   A best-effort Cargo cache may accelerate the producer but cannot be the
   only mechanism connecting jobs. Report missing, expired, corrupt, or
   incompatible outputs clearly; do not silently rebuild inside consumers.
3. **Correct identity and invalidation.** Bind outputs to source and build
   inputs, target, toolchain, profile, features, and required runtime assets.
   Define how build scripts, generated files, native dependencies, and
   compile-time environment values affect compatibility. Reuse must not rely
   solely on an OS label or cache-key prefix.
4. **Preserve intended test configurations.** Do not unify feature sets or
   profiles if that changes the behavior or coverage the original cells test.
   Build the union of compatible targets where valid; retain separate builds
   where configurations are intentionally distinct.
5. **Reuse across package areas where compatible.** Shared dependencies must
   not be rebuilt simply because another selected area consumes them. Scope
   builds to the selected targets and their necessary dependency closure;
   do not replace affected-scope planning with an unconditional workspace build.
6. **Keep the canonical plan authoritative.** Extend the existing resolved
   plan with build dependencies if needed. Consumers must not independently
   recalculate scope. Area remains a grouping, while result artifacts,
   receipts, baselines, and JUnit records retain package/environment/tier
   identities. Do not introduce an area-keyed evidence store.
7. **Separate build reuse from result reuse.** A compiled binary proves no
   test passed. Reuse qualifying passing evidence for a required cell on
   every OS; execute required cells without qualifying evidence. Reusing
   binaries never turns Linux execution into WSL or Windows test evidence.
8. **Preserve coverage and policy.** Keep L1 target coverage and the planner's
   example, bench, and reverse-dependency check requirements. Capability gaps,
   missing-cell handling, area rollups, and the merge gate keep their existing
   semantics. This change must not silently drop tests or create OS exclusions.
9. **Support relocated execution.** Supply test binaries, fixtures, companion
   executables, and required assets without depending on the producer's
   absolute filesystem paths. L2/L3 execution must not steal terminal or
   browser focus. Provision runtime facilities separately from compilation.
10. **Explain work and failures.** Report build configuration, producer,
    artifact reuse, compilation time, transfer time, and execution time
    separately. Failed or canceled producers must leave dependent required
    cells visibly blocked or missing, never passing.

## Acceptance Criteria

- A cold run with multiple selected areas and compatible L1/L2 consumers
  demonstrates one compilation of each required target/dependency
  configuration. Consumer logs contain no unexpected Cargo compilation.
- The same contract is demonstrated for macOS, Linux, native Windows, and
  WSL2. Any currently governed tier gap remains explicit and is not counted
  as a successful demonstration of that tier.
- A compatible Linux build can be executed in an isolated WSL2 guest without
  access to the builder's target directory, producing distinct WSL2 results.
- Changed features, toolchains, targets, build inputs, and incompatible
  native dependencies invalidate reuse or select a separate producer.
- Missing/corrupt artifacts and producer failures cannot produce passing
  required cells or an unreported fallback full rebuild.
- Qualifying passing test evidence suppresses repeat execution; cells
  without qualifying evidence still execute, using valid existing build
  outputs where available.
- Measurements compare compilation count and elapsed build/transfer/test
  time for equivalent scope and configuration before and after the change.
  Include cold and warm cases; artifact transfer must not conceal a net
  regression. Record observations rather than assuming a cache hit is reuse.
- Workflow documentation and relevant agent skills describe the final
  build/evidence distinction and the procedure for diagnosing reuse misses.

## Design Questions to Resolve

- Can nextest archives carry all required targets and runtime assets, or are
  additional bundles needed for check-only targets and companion programs?
- How should the canonical plan identify compatible producers without
  changing Cargo feature resolution across selected packages?
- Which artifact retention and transport mechanisms work for both hosted
  runners and declared cross-host test environments?
- Should independent consumers share immutable archives, isolated extracted
  copies, or a retained runner, and what is the measured scheduling tradeoff?
- How are valid build outputs reused across commits while keeping execution
  evidence qualification independent and preserving provenance?

## Relationship to Current Work

This is follow-up architecture work discovered during
`fixes/2026-09-11-cicd-cleanup/spec.md`. Repairing and publishing the pre-push
hook tests does not depend on implementing this draft. This document records
the repository-wide requirement without claiming the compilation redesign
has been implemented or validated.
