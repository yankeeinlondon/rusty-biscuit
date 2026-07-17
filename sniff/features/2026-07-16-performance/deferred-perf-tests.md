# Deferred Performance Tests — 2026-07-16-performance

This file records the performance measurements deferred during the implementation of
[review-1.md](review-1.md), per the implementation log at [log.md](log.md)
(`deferred_perf_measurement: true`).

## Mapping

Both deferred items map to **Finding 6** ("Medium: the completion evidence is red and
cross-platform results for this implementation are not attached") of
[review-1.md](review-1.md).

## Deferred item 1: three-OS test legs and retained work-count artifacts

- **What the review asks:** run the changed implementation on all three OS legs
  (macOS/Linux/Windows) and retain the work-count artifacts.
- **Why deferred:** this implementation ran on a macOS-only host in a non-interactive
  session; the Linux and Windows legs can only execute in CI. This is a platform
  availability constraint, not a CPU-load constraint.
- **How it is covered instead:**
  - `.github/workflows/test.yml` job `sniff-cross-platform` runs
    `cargo check --all-targets --features remote` plus the nextest tiers on
    `macos-latest`, `ubuntu-latest`, and `windows-latest`. The L1 test fixed in this
    cycle (`detect_area_errors_when_not_in_repo`) is portable and will be exercised on
    all three legs on the next CI run.
  - `.github/workflows/sniff-performance.yml` job `sniff-work-counts` runs the
    `work_counts` baseline nightly on the same 3-OS matrix and uploads per-OS artifacts
    with 90-day retention. Comparisons are only valid within one OS and runner class.
- **To close:** confirm a green `sniff-cross-platform` run and retained
  `sniff-work-counts-{os}` artifacts in CI after this branch merges.

## Deferred item 2: Criterion fixture families / `just bench`

- **What the review asks:** the required Criterion fixture families and `just bench`
  were deferred during the feature; either add/run them or narrow the specification's
  verification commitments.
- **Why deferred:** the fixture families exist under `sniff/lib/benches/` (nine case
  modules driven by `perf.rs`, with `just bench*` recipes in `sniff/justfile`), but
  this repo's performance doctrine accepts **work counters, not wall-clock timing**, as
  evidence — timing on a loaded dev host swings 2x+ on identical code and is explicitly
  non-evidentiary (see the sniff agent skill: "Counters, not wall time"). Running
  Criterion locally here would produce numbers the project does not accept, while the
  accepted evidence (work counts) is already collected nightly by the CI matrix above.
- **To close:** if wall-clock Criterion evidence is still wanted, run `just bench` on
  an idle, dedicated runner (the `ci-linux` baseline class) and archive the artifacts
  per the phase-8 convention; otherwise narrow the specification's verification
  commitments in a follow-up spec edit.

## Review 3 deferred items

The following narrower components map to [review-3.md](review-3.md). All eight findings
were implemented in source, tests, benchmarks, or documentation; these items record
evidence or workload limitations that this macOS-only implementation session could not
close without external execution or unrelated public API expansion.

### Finding 4: native Linux/Windows Level-1 execution and retained artifacts

- **Review mapping:** "High: the canonical Level-1 suite is still red and cross-platform
  completion is unproven."
- **Implemented locally:** the environment-dependent OS and Git fixtures are now stable;
  the canonical macOS run passed all 1,657 `sniff-lib` tests and all 777 `sniff-cli`
  tests. `cargo check -p sniff --all-targets --features remote --target
  x86_64-pc-windows-gnu` also passed.
- **Why deferred:** this host cannot execute native Linux or Windows Level-1 binaries,
  has no Linux Rust target installed, and lacks the Windows SDK headers needed by the
  MSVC native dependencies. A Windows GNU CLI check was stopped while still compiling
  dependencies under the non-interactive session's command-duration rule; it reported
  no source error before termination.
- **To close:** run the `sniff-cross-platform` workflow for this exact implementation and
  retain the passing macOS, Linux, and Windows artifacts. The workflow definition alone
  is not counted as execution evidence.

### Finding 7: large synthetic service-listing Criterion workload

- **Review mapping:** "Medium: the specified Criterion workload families remain absent."
- **Implemented locally:** every other missing workload family now has a Criterion ID,
  reusable fixture, and mapped deterministic counter bound in
  `sniff/lib/benches/README.md`; both `network` and `remote` benchmark targets compile.
- **Why deferred:** `ServiceManager::detect()` intentionally observes the live host, while
  backend parsers and command injection are crate-private. A deterministic hundreds- or
  thousands-service Criterion fixture cannot be supplied through the public API, and
  exposing a production injection seam solely for this benchmark would add unrelated
  public surface.
- **Existing structural evidence:** Level-1 tests already pin enrichment chunking,
  partial failure, subprocess deadlines, and process-tree reaping, so the production
  work bounds remain verified even though the wall-clock workload row is absent.
- **To close:** introduce a separately reviewed internal benchmark seam that does not
  expand the production public API, then add the synthetic service inventory case and
  collect timings on the stable runner.
