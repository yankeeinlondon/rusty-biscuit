# Opt-In Performance Testing per Package

Status: draft — questions collected for a brainstorming session

Builds on `fixes/2026-08-06-cicd/spec.md` (the package is the unit of CI
selection, execution, and result identity) and on two measured findings from
2026-08-12: `bench-nightly` spent ~34 minutes nightly pushing darkmatter
Criterion results to Bencher.dev whose upload had been silently failing
("Failed to create new report"), and `sniff-performance`'s own header is the
best statement of the metric problem in the repo — work counts are
deterministic and comparable; wall time on a hosted runner reported +330% for
a case whose counters were byte-identical.

## The idea

A package **opts into performance testing** through its own manifest, the
same way it opts into L2 or browser tiers. When a commit impacts that package
— directly or through its dependency closure, exactly as the scope calculator
already decides — a **performance job runs for it, separate from the
functional tiers**. Packages that never opt in never pay.

This replaces the retired `bench-nightly` (workspace-uniform, schedule-driven,
consumer-less) with something selection-driven, package-owned, and visible in
the same result grid as everything else. `sniff-performance` survives as-is
for now, with the explicit intent that its two halves (PR Criterion subset,
nightly 3-OS work-count matrix) migrate onto this mechanism once it exists.

Strawman manifest shape, deliberately underspecified — the questions below
are the spec:

```toml
[package.metadata.ci.perf]
kind = ["counters", "criterion"]   # metric classes this package publishes
gating = false                     # advisory until noise is characterized?
environments = ["ubuntu-latest"]   # where measurement is meaningful
budget-minutes = 10
```

## What already exists to build on

- The scope calculator computes the impacted closure per commit; "perf runs
  when the package is impacted" is one more consumer of that resolution.
- Every package area already has a canonical `bench` recipe (no-op when
  empty), and `[package.metadata.benchmarks]` already exists as a manifest
  pattern in this workspace.
- The result grid already has tiers, states (`NOTHING TO RUN`,
  `POLICY GAP`), a baseline with governance fields, and a verdict that knows
  the difference between advisory notes and blockers.
- darkmatter has 16 orphaned Criterion targets (bench-nightly's payload);
  sniff has both a Criterion subset and the work-count example.
- `feedback_benchmark_drift_bracket` (memory): cross-run wall-clock
  comparisons on this project have already required drift-bracketing once.

## Design questions for the brainstorm

### Metric philosophy

1. **What counts as a performance result?** Deterministic work counters
   (sniff's model: entries visited, opens, parses, subprocesses, HTTP
   requests), wall-clock statistics (Criterion/Divan), instruction counts
   (iai-callgrind, valgrind-based, noise-immune but Linux-only), peak memory,
   binary size? Are these one tier with metric classes, or different things?
2. **Is wall time ever gating on hosted runners?** sniff-performance's
   position is no-until-characterized. Do we adopt "counters may gate, time
   never gates" as a standing rule, or is time-gating achievable with
   instruction counts / self-hosted hardware?
3. **What is a regression, precisely?** Counter diff ≠ 0? Percentage
   threshold per benchmark? Statistical test across N runs? Who sets the
   threshold — the package's manifest, or a global default?

### Selection and scheduling

4. **Does dependency-closure impact include `Cargo.lock`-only bumps?** A dep
   version bump is exactly the change class that regresses performance
   without touching package code — but lockfile changes are frequent. Run
   perf on the lockfile-derived closure too, or only on direct/workspace-code
   impact?
5. **Per-impacting-commit, nightly, or both?** Per-commit gives attribution
   (this PR did it); nightly gives a stable time series on main and absorbs
   noise via repetition. Do opted-in packages get per-PR advisory + nightly
   series, with the nightly replacing what bench-nightly/sniff-performance's
   schedule slots did?
6. **Does perf run block the merge gate?** Options: never (pure advisory
   artifact + summary), gate only on counter regressions, gate per-package by
   manifest choice (`gating = true`). What does a perf FAIL cell mean in the
   verdict, and does the baseline mechanism extend to "known-slow"?

### Results, identity, and comparison

7. **Result identity**: `{package, environment, tier="perf"}` in the same
   schema-v2 result document, or a separate perf document keyed
   `{package, environment, benchmark}`? The rollup currently treats tiers as
   test suites with JUnit evidence — perf evidence is numbers, not pass/fail
   test cases. New producer contract or shoehorn?
8. **Where do baselines live?** Committed file (like `ci-baseline.toml`,
   auditable, churn per accepted change), branch-scoped artifact + fetch
   (invisible, cache-eviction-prone — see the cache-quota finding), or an
   external service (Bencher's job — but its silent-failure mode is exactly
   what we're escaping)? What identity does a baseline entry carry
   (machine class? toolchain version?)?
9. **Comparison anchor**: PR-vs-main-at-merge-base, vs last accepted
   baseline, or vs a trailing window of nightly runs? Merge-base comparison
   requires measuring both sides in one run (double cost) unless main's
   numbers are stored somewhere durable.
10. **How is runner noise handled when time IS measured?** Repetitions with
    IQR filtering, paired A/B interleaving on one runner, drift-bracketing
    (the project's existing pattern), instruction counts instead, or
    dedicated hardware (build-win exists; is a self-hosted perf runner in
    scope)?

### Mechanics

11. **What executes?** The canonical `bench` recipe per package (Criterion),
    a new canonical `perf` recipe (counters + whatever the package declares),
    or nextest-run `perf_`-prefixed tests (the tier-prefix taxonomy already
    reserves package-specific `perf` filters — R4 of the parent spec)?
12. **Toolchain**: Criterion vs Divan vs iai-callgrind vs bespoke
    counter-emitters — is the choice per-package (manifest `kind`) with the
    workflow supporting a closed vocabulary, mirroring `runner-tools`?
13. **Where does the sniff work-count example land?** It's the model for
    "counters" — does it become a small shared crate (`perf-counters`) other
    packages can adopt, and is its table format the interchange format?
14. **Budget and concurrency**: perf jobs are slow and serial by nature (a
    loaded runner invalidates timing). Do perf jobs get `-j 1`, dedicated
    concurrency groups, caps per run (budget-minutes), and exclusion from
    fail-fast?
15. **Cold-cache reality**: the cache-quota finding means CI builds are
    effectively cold; a perf job pays a full compile before measuring. Does
    perf reuse the functional job's build (same runner, sequenced after L1),
    or accept its own compile cost?

### Migration and governance

16. **sniff-performance migration**: which parts survive as-is (nightly 3-OS
    work counts), which fold in (PR Criterion subset → opt-in perf with
    `kind = ["criterion"]`), and does its `Cargo.toml`/`Cargo.lock` PR
    trigger narrow to the closure rule (question 4)?
17. **darkmatter's 16 Criterion targets**: adopt into opt-in (who owns
    triage?), or leave dormant until someone wants them? Their only consumer
    just proved to be /dev/null for weeks.
18. **Is `[package.metadata.benchmarks]` (existing) merged into
    `[package.metadata.ci.perf]`, or are they different concerns** (what
    benches exist vs what CI does with them)?
19. **Does the same opt-in pattern host fuzzing?** fuzz-nightly has the same
    shape-problems (hardcoded target list, no per-package ownership). One
    generic "scheduled deep tiers, package-opt-in" mechanism, or separate
    specs per concern?
20. **Failure visibility**: bench-nightly failed silently for weeks because
    upload was best-effort and green. Standing rule for this feature: what is
    the minimum failure surface (a red advisory job? a verdict note?) so that
    a perf pipeline that stops producing data cannot look healthy?

## Out of scope (proposed — confirm in brainstorm)

- Fixing any current performance regression.
- Self-hosted runner procurement (question 10 may recommend it; buying and
  wiring it is its own effort).
- Fuzzing changes (question 19 decides whether it shares the mechanism; the
  work is separate either way).

## Sequencing

Draft → brainstorm answers the questions above → spec ratified → plan. The
bench-nightly removal (2026-08-12) does not wait for this spec; its darkmatter
bench targets and the `just bench` recipes stay in place as this feature's
future payload.
