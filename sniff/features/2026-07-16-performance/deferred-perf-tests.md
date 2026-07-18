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

### Resolved finding 7: large synthetic service-listing Criterion workload

- **Review mapping:** "Medium: the specified Criterion workload families remain absent."
- **Resolved by review 4 finding 6:** the `bench-internals` feature now exposes a
  doc-hidden synthetic systemd fixture without changing the default production API.
  Criterion registers deterministic 500- and 2,000-service workloads that execute the
  production listing parser, running-service selection, 128-unit chunk construction,
  runner dispatch, show-block parser, and PID projection. Fixture construction and
  per-iteration cursor setup are outside the timed section.
- **Structural evidence:** `large_service_workloads_preserve_cardinality_and_chunk_bounds`
  pins output cardinality and one listing plus `ceil(N / 128)` enrichment calls for both
  workload sizes. `pid_enrichment_costs_one_subprocess_per_chunk_not_per_service` maps
  the same bound to the stable `process.spawns` counter through the real bounded runner.
- **Timing status:** benchmark compilation is verified locally; no wall timing from the
  loaded implementation host is recorded or claimed. The existing stable CI runner can
  collect comparable timing artifacts.

## Review 4 deferred items

### Finding 5: native Linux/Windows Level-1 execution and retained work-count artifacts

- **Review mapping:** "High: native Linux and Windows Level-1 completion remains
  unverified" in [review-4.md](review-4.md).
- **Exact implementation evaluated:** the dirty worktree based on
  `407a1dbfbce1bb953ef80ce8596805c77170b424`, containing the review-cycle-4 changes
  present during the 2026-07-17 verification run.
- **Native macOS evidence:** `just test` passed all 1,664 `sniff-lib` tests and all 779
  `sniff-cli` tests (14 and 3 skipped, respectively), and `just lint` passed on arm64
  macOS 26.5.2 (Darwin 25.5.0).
- **Supplemental Windows compile evidence:** `cargo check -p sniff --all-targets
  --features remote --target x86_64-pc-windows-gnu` passed in 8.23 seconds with four
  target-gated test warnings. This is cross-compilation and is not counted as native
  Windows Level-1 execution. The same check for `x86_64-pc-windows-msvc` stopped in
  native C dependencies because the macOS host has no Windows SDK headers
  (`ctype.h`/`windows.h`) or Visual C++ environment.
- **Available execution investigated:** Docker Desktop exposes a local aarch64 Linux
  6.12.76 kernel and cached Rust images, but the images contain neither `cargo-nextest`
  nor `just`; installing tools was outside this non-interactive task's authority, and a
  non-canonical container check would not verify native filesystem behavior. Suspended
  Parallels Debian 13, Ubuntu, and Windows 11 guests also exist, but starting user VMs
  was not authorized, so none was started.
- **Why deferred:** this run could not execute the canonical Level-1 suite natively on
  Linux or Windows, and no three-OS work-count artifacts for this exact dirty
  implementation were produced or retained. The Linux target is not installed, the
  Windows targets cannot execute on macOS, and this task prohibited commits, pushes,
  installs, VM startup, and unauthorized external workflow triggers. This is an
  execution-authority/platform constraint, not a CPU-load deferral. Existing workflow
  definitions were inspected previously but are not claimed as execution.
- **To close:** place this exact implementation on authorized native macOS, Linux, and
  Windows runners; run the canonical sniff-area `just test` and `just lint` commands on
  each OS; collect the work-count table on each OS; and retain the green run plus all
  three per-OS artifacts under one implementation identifier. Cross-compilation does
  not substitute for any native leg.
