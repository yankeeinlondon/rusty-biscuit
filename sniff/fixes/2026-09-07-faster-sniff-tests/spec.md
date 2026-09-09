---
area: sniff
status: draft
created: 2026-09-07
reviewed: true
reviewed_by: opencode/zai-coding-plan/glm-5.3
reviewed_on: 2026-09-07
implemented: true
review_iterations: 1
coordinates_with:
    - sniff/fixes/2026-07-22-inefficient-calling/spec.md
packages:
    - sniff
    - sniff-cli
---

# Faster Sniff tests with explicit discovery contracts

## Outcome

Evaluate every Sniff test family and remove incidental host/repository discovery,
unnecessary subprocess probes, repeated fixture work, and waiting. Preserve
focused real-detector coverage on macOS, Linux, and Windows.

Discovery is the product: the objective is to make each test request and prove
the observations it needs, not to replace every detector with a fake.

This document is a draft specification, not evidence that every suspected cost
is present; each audit lead below requires measurement before remediation.
Follow the
[rust-testing contract](../../../.claude/skills/rust-testing/SKILL.md) and
[audit guidance](../../../.claude/skills/rust-testing/test-suite-audits.md).

## Evidence and scope

`cli/tests/cli.rs::run_isolated_software` already limits PATH/home/install
directories for selected CLI-contract tests, but it does not pin the launch
CWD, which nextest leaves at the package directory inside the checkout. The
remaining ~370 `cargo_bin` spawn sites across `cli/tests/` — `cli.rs`, the
`run_stdout` helper in `snapshots.rs`, `install_plan.rs`,
`install_interview_cli.rs`, and `tty.rs` — construct raw commands that inherit
the full host environment. These are audit leads; a raw command does not by
itself prove unnecessary discovery or a measured slowdown.

Sniff already exposes request plans, focused APIs, captured observations, and
stable counters. Reuse those seams. The
[Sniff performance guidance](../../../.claude/skills/sniff/performance.md)
documents collector propagation and baseline compatibility. Two integration
suites (`lib/tests/benchmark_workloads.rs`, `lib/tests/integration.rs`)
already demonstrate collector-based assertions; extend that pattern rather
than inventing new measurement seams.

Further audit leads from the current tree:

- `cli/tests/install_interactive_pty.rs` is gated on a bespoke
  `SNIFF_INTERACTIVE_PTY=1` environment variable, so no canonical recipe
  selects it.
- `lib/tests/foo.rs` contains only a placeholder `#[test] fn bar() {}` — a
  dead test binary.
- The `real_` network tests embedded in `lib/src/package/network.rs` are
  reachable only through `just test-real` with the `network` feature.
- `lib/tests/git_parity.rs` and `lib/tests/bench_fixtures.rs` already share
  the deterministic repository builder at `lib/benches/support/builder.rs`;
  that seam is the model for sharing fixture construction instead of
  repeating repository setup per test.

Deliverables live in this fix's directory: `inventory.md` (classification and
dispositions) and `results.md` (measurements and closure), matching the
artifact convention of the sibling faster-tests fixes.

The active production-caching fix
([2026-07-22-inefficient-calling](../2026-07-22-inefficient-calling/spec.md))
changes detector costs. Coordinate with it: record whether each baseline
revision predates or includes that fix, and re-baseline affected cohorts if it
lands during this work. No timing comparison may span that landing.

Inventory all embedded/external tests and shared helpers in `sniff` and
`sniff-cli`, plus doctests, benchmarks, and fuzz entry points where present.
Include real-resource, terminal, ignored, cfg-gated, and feature-gated tests.
Ordinary `just test` enables library `remote`; local CLI L1 leaves
`test-fixtures` off while CI/L2 enables it. Preserve that coverage distinction.

Production detector redesign, process-wide caching, new remote integrations,
changing request semantics, new platform support, and global CI changes are
outside scope. A production defect uncovered by the audit gets separate
evidence and a linked follow-up rather than a test-only workaround.

## Required behavior

### 1. Inventory and classify every test

Produce `inventory.md` with every identity mapped to one reviewed row or an
explicitly enumerated family. Record assertion quality, request shape, observed
domains, input ownership, subprocess/network/tool dependencies, setup/cleanup,
timing floor, tier/features/platforms, canonical recipe, cost, and disposition.

Separate three purposes:

- Deterministic parsing/projection/planning/CLI contracts, which use controlled
  observations, tool stubs, or local repositories.
- Native detector contracts, which must exercise real platform APIs or bounded
  subprocess behavior and assert portable invariants rather than a developer's
  current hardware or installed-software inventory.
- External-resource contracts, which require explicit endpoints/devices or live
  services and retain the appropriate resource gate.

OS specificity alone does not move a test to L2/L3 or require ignoring it.
Basic native OS API coverage may remain ordinary cfg-gated L1. Map tier to the
actual resource required and document every proposed classification change.

Reconcile source definitions and feature gates with runner discovery. Include
embedded unit tests such as the `real_` network tests in
`lib/src/package/network.rs`, not just `tests/` targets. Bespoke environment
gates such as `SNIFF_INTERACTIVE_PTY=1` must resolve to a canonical execution
route or be recorded as unreachable with a reason. Inspect the `test-real`
feature selection explicitly: `network` alone must not be mistaken for
coverage of tests requiring `remote`.

### 2. Isolate deterministic CLI and repository tests

Provide one fixture-owned deterministic command path, reusing existing helpers
where useful. Pin launch CWD, home/config/cache, application environment, Git
plumbing, rendering inputs, and controlled PATH/install roots. Platform-specific
software search roots must not expose the host roster accidentally.

Use disposable repositories, worktrees, manifests, and command stubs. Fixture
Git setup must not inherit `GIT_DIR`, `GIT_WORK_TREE`, or related plumbing.
Contain fixtures outside the checkout, including canonicalized paths, and
preserve Windows shell/PATH requirements.

Host discovery is an explicit choice with a documented domain and reason.
Retain real detector tests rather than converting them into snapshots of fake
results. Structural protection follows the repo-wide shape documented in the
rust-testing contract and proven in claudine's `CliProcessFixture` and spawn
guard: one shared command builder per package area plus a source-scan guard
with an explicit `(file, reason)` allowlist whose stale entries fail, so
accidental raw deterministic spawns and stale exemptions are rejected without
outlawing intentional host observations. Named PATH/context escapes require a
call-site comment naming the tool or the proof they need.
`run_isolated_software` is grown into that builder or becomes one named
escape; it must not survive as a parallel convention.

### 3. Bound and prove requested work

Use focused requests and captured observations when incidental full detection
is unnecessary. Tests of aggregate behavior retain aggregate execution. Do not
narrow the request that is itself under test merely to improve timing.

Assert existing stable counters for request-cost contracts: absent work is
zero, seeded execution does not rediscover Git, and projection does not
reacquire observations. Account for acquisition and execution separately when
collection boundaries differ. Verify collector propagation across threads,
Rayon, and walker workers before interpreting a lower count as less work.

Audit repeated repository construction, software version probes, full discovery
inside loops, and benchmark-fixture tests. Share immutable data or use smaller
representative fixtures where equivalent proof exists. Keep separate scaling
benchmarks for large topologies and required real process behavior.
Process-local caches do not share setup across nextest test processes.

Snapshot normalization may remove volatile values but must not erase the
identity, selection, ordering, or error behavior being asserted. Repair weak
assertions and record every replacement or removed test's coverage mapping.

### 4. Bound external effects and cleanup

Remote-provider deterministic tests use local controlled servers, retain
exact-host consent/credential-scope behavior, and assert request counts,
pagination bounds, and typed failures. No live network is needed just to verify
serialization or projection. Genuine network probes remain explicit real tests.

Subprocess fixtures drain output while waiting, use deadlines, and reap children
on errors and cancellation. Prove cleanup for process-owning cohorts with
nextest's per-test leak policy plus the root `just test-leaks` post-run sweep,
not by inspection alone. Tests of timeout behavior preserve real termination
paths and their semantic timing floors. Replace readiness sleeps with bounded
condition/protocol observation.

Use unique sockets/ports/directories and shutdown/join local server workers.
Keep runner-visible serialization only for real shared resources; investigate
the Windows L1 group using evidence before proposing any concurrency change.
Real-terminal coverage must run through canonical harness recipes without
taking focus.

### 5. Measurements and budgets

Record baseline revision, toolchain, feature/request shape, test identities,
profile, concurrency, cache state, environment, failures, and skips. Separate
compilation, fixture setup, execution wall clock, and per-test serial sum.

Collect five alternating warm local runs per revision for affected cohorts and
the relevant full L1 suite, including the `just sanity` cohort so the
fast-confidence path is measured rather than assumed. If the coordinated
production-caching fix lands during this work, re-baseline affected cohorts at
the new revision. Pair timings with work counters and an unchanged
comparison case where practical. Do not infer request-cost ratios from one
ordered run that progressively warms the page cache.

Ratify per-family work and timing budgets after attribution and before evaluating
remediation. Preserve native-detector costs as a separate cohort; do not compare
a real detector before with a fake projection afterward as a product speedup.
No universal percentage target is assumed.

Collect three consecutive candidate runs for configured CI package/environment
legs, retaining baseline artifacts and intervening failures. Compare only within
compatible OS/runner/request/counter versions; native Windows and WSL are
distinct. No additional CI matrix is required. Missing execution evidence stays
pending and cannot be satisfied by a feature-disabled run.

## Verification and acceptance

- Every test/family has an evaluated purpose, disposition, and real execution
  route; no exclusions based on historical speed.
- Deterministic CLI/repository tests inherit no accidental checkout, user
  configuration, software roster, or Git plumbing. Named host-discovery tests
  retain native behavior and platform-appropriate assertions.
- Request/work-count contracts prove eliminated incidental work, with collection
  boundaries and worker propagation verified.
- Remote and subprocess fixtures are bounded and cleaned up on missing
  interaction, failure, and cancellation; no focus changes or hidden live API
  requests occur in deterministic tests.
- Generic fixture-migration exemptions are eliminated. Technical exceptions
  have specific reasons and equivalent isolation/ownership evidence.
- `just test`, `just check`, `just lint`, and `just doctest` preserve `remote`
  coverage. Run affected L2 tests with `just test-l2`; use `just test-real`
  only for the relevant available resources after verifying required features.
  `just sanity` stays green and its measured duration is reported against the
  15-second fast-confidence budget. Cross-platform compilation/runtime
  evidence uses the same feature contract, with the `windows-latest` CI leg
  as the compile authority for Windows-only test targets (this area has no
  local mingw check recipe).
- Bespoke environment gates and placeholder test binaries are removed or
  routed through a canonical recipe; none remain silently unreachable.
- `results.md` in this fix's directory includes coverage changes, work
  counts, ratified timing budgets, failures/skips, local/CI results, and
  linked deferred findings. No reduced coverage, new retries, or
  timeout-limit increases substitute for optimization. Process-owning
  cohorts show clean `just test-leaks` sweeps.
- Update the Sniff skill and area docs when fixture or workflow contracts change.
  Shared production changes require a separate scope and downstream impact review.

## Draft decisions to resolve during baseline review

- Which host observations are intentional coverage and which are accidental?
- Which existing observation/tool seams are sufficient for deterministic tests?
- Which feature/tier combinations currently leave tests unreachable?
- Which bespoke environment gates (for example `SNIFF_INTERACTIVE_PTY`) become
  tiered or recipe-routed tests, and which are removed?
- What cost and concurrency budgets are justified for each native platform?
