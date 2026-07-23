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

## Review 5 deferred items

### Finding 1: native Linux and Windows Level-1 execution remains absent

- **Review mapping:** "High: native Linux and Windows Level-1 execution remains absent"
  in [review-5.md](review-5.md).
- **Exact implementation evaluated:** the dirty worktree rooted at
  `03b03ea5a85d6f26fa6c257f254943983e99b72c` during the 2026-07-17 review-cycle-5
  implementation. Final closure must use one identifier for the completed cycle-5
  tree because the other serial findings can change that tree after this assessment.
- **Host evidence:** `sniff os --json` reported native arm64 macOS 26.5.2 (Darwin
  25.5.0), and `sniff cpu --json` reported an Apple M4 Max with 16 physical and logical
  cores. This host cannot execute native Linux or Windows binaries or reproduce their
  path, process, registry, Job Object, filesystem, and case behavior.
- **Evidence boundary:** cross-compilation can prove only that a target compiles. WSL
  follows Sniff's Linux runtime path, Docker is not native Windows evidence, and neither
  a workflow definition nor an untriggered scheduled job is execution evidence for this
  implementation. Work-count tables are comparable only within one OS and runner class,
  so a macOS table cannot stand in for Linux or Windows.
- **Why deferred:** the session is non-interactive and explicitly prohibits VM startup,
  external workflow triggers, tool installation, commits, and pushes. Consequently,
  there is no authorized path from this macOS workspace to run the exact implementation
  natively on Linux and Windows or to retain a matched three-OS work-count artifact set.
  This is a platform and execution-authority constraint, not a CPU-load deferral.
- **Existing execution definitions:** `.github/workflows/test.yml` defines the
  `sniff-cross-platform` matrix on `macos-latest`, `ubuntu-latest`, and
  `windows-latest`, including `cd sniff && just test`. The scheduled
  `.github/workflows/sniff-performance.yml` matrix runs
  `cargo run -q -p sniff --release --example work_counts` on the same three OS classes
  and retains each `sniff-work-counts-{os}` artifact for 90 days. These definitions were
  inspected but are not claimed as completed runs.
- **To close:** publish one immutable completed cycle-5 implementation identifier to an
  authorized runner source; obtain green native `cd sniff && just test` and
  `cd sniff && just lint` runs on macOS, Linux, and Windows; run the `work_counts`
  example on each native OS; and retain all three tables plus the test/lint run links
  under that same identifier. Cross-compilation, Docker, WSL, or results from a
  different tree do not close this finding.

## Review 6 deferred items

### Finding 1: native Linux and Windows Level-1 execution remains absent

- **Review mapping:** "High: native Linux and Windows Level-1 execution remains absent"
  in [review-6.md](review-6.md).
- **Exact implementation evaluated:** the dirty worktree rooted at
  `c864518c5` during the 2026-07-18 review-cycle-6 implementation, plus the cycle-6
  changes to `sniff/lib/src/filesystem/repo/aggregate_view.rs`,
  `sniff/lib/src/performance.rs`, `sniff/lib/src/process.rs`,
  `sniff/cli/src/commands/mod.rs`, and `sniff/docs/sniff-library-architecture.md`.
  Final closure must use one identifier for the completed cycle-6 tree.
- **Host evidence:** `uname -a` reported native arm64 macOS (Darwin 25.5.0).
  `rustup target list --installed` reported `aarch64-apple-darwin`, `wasm32-wasip1`,
  `wasm32-wasip2`, and `x86_64-pc-windows-gnu` — no Linux target, and the Windows
  target cannot execute on this host. This host cannot reproduce Windows Job Objects,
  Linux `/proc` process discovery, native path and case behavior, or the
  filesystem work-count fixtures.
- **Why deferred:** unchanged from cycle 5. The session is non-interactive and
  prohibits VM startup, external workflow triggers, tool installation, commits, and
  pushes, so no authorized path exists from this macOS workspace to run the exact
  implementation natively on Linux and Windows or to retain a matched three-OS
  work-count artifact set. This is a platform and execution-authority constraint,
  not a CPU-load deferral.
- **Cycle-6 relevance note:** this cycle changed Unix-specific process-tree
  termination (`sniff/lib/src/process.rs`), so the native Linux leg is now more
  load-bearing than in previous cycles. The new cumulative descendant tracking and
  PID-identity revalidation were exercised natively on macOS only; Linux `/proc`
  descendant discovery through `sysinfo` and the `setsid` escape fixture must be run
  natively on Linux before this finding closes. The Windows Job Object path was not
  changed this cycle and was verified only by cross-compilation.
- **To close:** publish one immutable completed cycle-6 implementation identifier to
  an authorized runner source; obtain green native `cd sniff && just test` and
  `cd sniff && just lint` runs on macOS, Linux, and Windows; run the `work_counts`
  example on each native OS; and retain all three tables plus the test and lint run
  links under that same identifier. Cross-compilation, Docker, WSL, or results from a
  different tree do not close this finding.

## Review 7 deferred items

### Finding 1: native Linux and Windows Level-1 execution and matched work-count artifacts are still absent

- **Review mapping:** "High: native Linux and Windows Level-1 execution and matched
  work-count artifacts are still absent" in [review-7.md](review-7.md).
- **Exact implementation evaluated:** the working tree rooted at
  `c32f78e43139868cf5831905e891c388d5fa3e74`, plus the cycle-7 changes to
  `sniff/lib/src/process.rs`, `sniff/lib/src/programs/install/`, and the
  process-containment documentation. Final closure must use one identifier for the
  completed cycle-7 tree.
- **Host evidence:** `uname -a` reported native arm64 macOS
  (Darwin 25.5.0, `xnu-12377.121.10`, `RELEASE_ARM64_T6041`).
  `rustup target list --installed` reported `aarch64-apple-darwin`, `wasm32-wasip1`,
  `wasm32-wasip2`, and `x86_64-pc-windows-gnu` — no Linux target, and the Windows
  target cannot execute on this host.
- **Publication evidence:** `git branch -r --contains c32f78e43139868cf5831905e891c388d5fa3e74`
  returned no remote branch, so the reviewed commit exists only in this local
  worktree. No hosted CI run for that SHA can exist, which is consistent with the
  review's observation that no retained run is publicly discoverable.
- **Why deferred:** this is an execution-authority constraint, not a CPU-load
  deferral, and it is structurally unchanged from cycles 5 and 6. Closing the
  finding requires the reviewed tree to be committed and pushed so the scheduled
  `sniff-cross-platform` and `sniff-performance` matrices can execute it natively.
  This session is explicitly prohibited from committing, from pushing, and from
  invoking credential helpers such as `ssh`/`gpg`, so no authorized path exists from
  this workspace to produce native Linux and Windows Level-1 runs or a matched
  three-OS `work_counts` artifact set.
- **Docker explicitly not attempted:** a Linux container on this host provides a real
  Linux kernel and would exercise `/proc` descendant discovery, but the review states
  that Docker results do not close this finding. Running a cold container build of the
  workspace would consume a large budget for evidence the reviewer has already ruled
  inadmissible, so it was not attempted.
- **Cycle-7 relevance note:** this cycle again changed Unix-specific process-tree
  behavior — `sniff/lib/src/process.rs` gained a Level-1 fixture that reproduces the
  documented between-samples `setsid()` escape. That fixture depends on Unix
  `fork`/`setsid` semantics and on descendant sampling, so it has been exercised
  natively on macOS only. It must be run natively on Linux, where descendant
  discovery goes through `/proc`, before this finding closes. The Windows Job Object
  path was not changed this cycle.
- **To close:** commit and push one immutable cycle-7 implementation identifier to
  `origin`; obtain green native `cd sniff && just test` and `cd sniff && just lint`
  runs on macOS, Linux, and Windows; run the `work_counts` example on each native OS;
  and retain all three tables plus the test and lint run links under that same
  identifier. Cross-compilation, Docker, WSL, or results from a different tree do not
  close this finding.

## Review 8 deferred items

### Finding 1: native Linux and Windows Level-1 execution and matched work-count artifacts remain absent

- **Maps to:** `sniff/features/2026-07-16-performance/review-8.md`, the single High
  finding titled "native Linux and Windows Level-1 execution and matched work-count
  artifacts remain absent".
- **Umbrella requirement:** [spec.md:396](spec.md#L396) — cross-platform tests must
  pass on macOS, Linux, and Windows, and the scheduled matrix must emit comparable
  per-OS work-count artifacts.
- **Implementation identifier:** the reviewed tree is HEAD
  `98fa4d992adf13a85c31995b3feb3d270a9a26d1` plus this cycle's uncommitted changes to
  the installer timeout contract (`sniff/lib/src/error.rs`,
  `sniff/lib/src/programs/install/*`, `sniff/cli/src/install_ui.rs`,
  `sniff/cli/src/install_plan_cmd.rs`), the deterministic between-samples regression
  (`sniff/lib/src/process.rs`), and the downstream migrations in
  `biscuit-speaks/cli/` and `playa/cli/`.
- **Host evidence:** `uname -a` reports native arm64 macOS (Darwin 25.5.0).
  `rustup target list --installed` carries only `aarch64-apple-darwin`,
  `wasm32-wasip1`, `wasm32-wasip2`, and `x86_64-pc-windows-gnu` — no Linux target and
  no native Linux or Windows execution host.
- **Publication evidence:** `git branch -r --contains 98fa4d992adf13a85c31995b3feb3d270a9a26d1`
  returned no remote branch, so the reviewed commit exists only in this local
  worktree and no hosted CI run for that SHA can exist.
- **Why deferred:** this is an execution-authority constraint, not a CPU-load
  deferral, and it is structurally unchanged from cycles 5, 6, and 7. Closing the
  finding requires the reviewed tree to be committed and pushed so the scheduled
  `sniff-cross-platform` and `sniff-performance` matrices can execute it natively.
  This session is explicitly prohibited from committing, from pushing, and from
  invoking credential helpers such as `ssh`/`gpg`, so no authorized path exists from
  this workspace.
- **Cycle-8 relevance note:** this cycle again changed Unix-specific process
  behavior. `sniff/lib/src/process.rs` replaced the load-dependent between-samples
  fixture with a deterministic `#[cfg(all(unix, test))]` sampler hook. The new
  handshake uses a `getppid()` comparison against the recorded original parent rather
  than `ppid == 1`, specifically so it remains correct under Linux subreapers and PID
  namespaces — but that portability claim has been exercised natively on **macOS
  only**. It must run natively on Linux, where descendant discovery goes through
  `/proc`, before this finding closes. The Windows Job Object path was not changed
  this cycle.
- **To close:** commit and push one immutable cycle-8 implementation identifier to
  `origin`; obtain green native `cd sniff && just test` and `cd sniff && just lint`
  runs on macOS, Linux, and Windows; run the `work_counts` example on each native OS;
  and retain all three tables plus the test and lint run links under that same
  identifier. Cross-compilation, Docker, WSL, or results from a different tree do not
  close this finding.

## Review 9 deferred items

### Finding 1 (High) — native Linux and Windows execution and matched work-count artifacts

- **Maps back to:** [review-9.md](review-9.md), first finding; umbrella
  [spec.md:397](spec.md#L397) acceptance criterion "Cross-platform tests pass on macOS,
  Linux, and Windows, and the scheduled benchmark matrix emits comparable work-count
  artifacts."
- **Requirement:** one immutable final implementation identifier, retained green native
  `just test` and `just lint` runs on all three operating systems, and three matched
  `work_counts` artifacts published under that identifier.
- **Host evidence:** `uname -a` reports native arm64 macOS (Darwin 25.5.0,
  `xnu-12377.121.10~1/RELEASE_ARM64_T6041`). `rustup target list --installed` reports
  `aarch64-apple-darwin`, `wasm32-wasip1`, `wasm32-wasip2`, `x86_64-pc-windows-gnu`, and
  `x86_64-pc-windows-msvc` — **no Linux target at all**, and neither Windows target can
  execute on macOS.
- **Publication evidence:** `git branch -r --contains af4751810e9bc66f3e3dbe5b883c864ce76c77a0`
  returned no remote branch. That is the exact commit review-9 named, so the review's own
  observation is confirmed rather than merely repeated: no hosted matrix can have run for it.
- **Why deferred:** an execution-authority and platform constraint, not a CPU-load
  deferral. It is structurally unchanged from cycles 5 through 8. This session is
  prohibited from committing, pushing, invoking credential helpers, starting VMs, and
  triggering external workflows, so no authorized path to native Linux or Windows
  execution exists from this workspace. Cross-compilation proves compilation only; Docker
  was ruled inadmissible for this finding by review-7; a workflow definition is not an
  execution record.
- **Cycle-9 relevance note:** this cycle changed no Unix or Windows process code. The two
  implemented findings were a `playa/cli` outcome-matching fix and documentation
  corrections, so the specific native-platform risk surface is unchanged from cycle 8 —
  the deterministic between-samples sampler hook and its `getppid()` portability claim
  still have been exercised natively on **macOS only** and still need a native Linux run
  where descendant discovery goes through `/proc`.
- **New this cycle:** the identifier to publish will be a **new** SHA, not
  `af4751810`, because cycle 9 changed `playa/cli/src/main.rs`,
  `playa/cli/src/install_ui.rs`, `sniff/lib/CHANGELOG.md`, `sniff/lib/README.md`,
  `sniff/cli/README.md`, and `sniff/lib/src/programs/mod.rs`. The three-OS evidence must
  be gathered against that final tree, not against any earlier reviewed commit.
- **To close:** commit and push one immutable cycle-9 implementation identifier to
  `origin`; obtain green native `cd sniff && just test` and `cd sniff && just lint` runs on
  macOS, Linux, and Windows; run the `work_counts` example natively on each; and retain all
  three tables plus the test and lint run links under that same identifier. Cross-compilation,
  Docker, WSL, or results from a different tree do not close this finding.

## Review 10 deferred items

### Finding 1 (High) — native Linux and Windows execution and matched work-count artifacts remain absent

- **Maps back to:** [review-10.md](review-10.md), the single High finding titled
  "native Linux and Windows execution and matched work-count artifacts remain absent";
  umbrella [spec.md:397](spec.md#L397) acceptance criterion "Cross-platform tests pass on
  macOS, Linux, and Windows, and the scheduled benchmark matrix emits comparable work-count
  artifacts."
- **Requirement:** one immutable final implementation identifier, retained green native
  `just test` and `just lint` runs on all three operating systems, and three matched
  `work_counts` artifacts published under that identifier.
- **Host evidence:** `uname -a` reports native arm64 macOS (Darwin 25.5.0,
  `xnu-12377.121.10~1/RELEASE_ARM64_T6041`). `rustup target list --installed` reports
  `aarch64-apple-darwin`, `wasm32-wasip1`, `wasm32-wasip2`, `x86_64-pc-windows-gnu`, and
  `x86_64-pc-windows-msvc` — **no Linux target at all**, and neither Windows target can
  execute on macOS.
- **Publication evidence:** `git branch -r --contains 77b3ea5ed0b9fffbc8a88bcca1fcd2bcd9302023`
  returned no remote branch against a fetched `origin` (`git@github.com:yankeeinlondon/rusty-biscuit.git`).
  That is the exact commit review-10 named, so the review's own observation is re-confirmed
  rather than inherited: no hosted matrix can have executed the reviewed tree.
- **Why deferred:** an execution-authority and platform constraint, not a CPU-load
  deferral. It is structurally unchanged from cycles 5 through 9. This session is
  prohibited from committing, pushing, invoking credential helpers, starting VMs, and
  triggering external workflows, so no authorized path to native Linux or Windows
  execution exists from this workspace. Cross-compilation proves compilation only; Docker
  was ruled inadmissible for this finding by review-7; a workflow definition is not an
  execution record.
- **Cycle-10 relevance note:** this cycle changed no production source at all. The two
  implemented findings were verified as already landed by `6a2a00d4d` (playa-cli command
  boundary regression, test-only seam) and `4cf06917e` (documentation and rustdoc
  corrections); the only new writes this cycle are this feature's own records. The
  native-platform risk surface is therefore unchanged from cycle 9 — the deterministic
  between-samples sampler hook and its `getppid()` portability claim have still been
  exercised natively on **macOS only** and still need a native Linux run where descendant
  discovery goes through `/proc`.
- **Implementation identifier:** the final tree is HEAD
  `8e4520645` ("docs(sniff): record blocked phase 01 review cycle") plus this cycle's
  uncommitted record edits to `sniff/features/2026-07-16-performance/log.md`,
  `deferred-perf-tests.md`, and `review-10.md`. The three-OS evidence must be gathered
  against that final tree, not against any earlier reviewed commit.
- **To close:** commit and push one immutable cycle-10 implementation identifier to
  `origin`; obtain green native `cd sniff && just test` and `cd sniff && just lint` runs on
  macOS, Linux, and Windows; run the `work_counts` example natively on each; and retain all
  three tables plus the test and lint run links under that same identifier. Cross-compilation,
  Docker, WSL, or results from a different tree do not close this finding.

## Review 11 deferred items

### Finding 1: native Linux and Windows execution and matched work-count artifacts

- **Review mapping:** "High: native Linux and Windows execution and matched work-count
  artifacts remain absent" ([review-11.md](review-11.md)). Structurally the same finding
  carried by review cycles 5 through 10.
- **Why deferred:** re-verified in cycle 11 rather than inherited from cycle 10.
  `uname -a` reports native arm64 macOS (Darwin 25.5.0,
  `xnu-12377.121.10~1/RELEASE_ARM64_T6041`). `rustup target list --installed` carries
  **no Linux target at all**, and neither installed Windows target
  (`x86_64-pc-windows-gnu`, `x86_64-pc-windows-msvc`) can execute on macOS.
  `git branch -r --contains c2a188379d1be770bfa3638f412552cb05310839` returns no remote
  branch. This session is additionally prohibited from committing, pushing, invoking
  credential helpers, starting VMs, and triggering external workflows, so no authorized
  native Linux or Windows execution path exists from this workspace. This is a **platform
  and execution-authority constraint, not a CPU-load deferral.**
- **Admissibility, restated:** cross-compilation proves compilation only; Docker was ruled
  inadmissible for this finding by review-7; a workflow definition is future coverage, not
  an execution record.
- **Cycle-11 relevance note:** this cycle changed **no production behavior**. Its Rust
  edits are three single-line rustdoc path corrections
  (`sniff/lib/src/process.rs:7`, `sniff/lib/src/remote/snapshot.rs:10`,
  `sniff/lib/src/filesystem/git/discovery.rs:578`); everything else is restored or
  re-linked documentation. The native-platform risk surface is therefore unchanged from
  cycle 10.
- **Identifier note:** the identifier to publish is a **new SHA** — current HEAD
  `c2a188379` plus this cycle's uncommitted record and link restoration — not `c2a188379`
  itself. The three-OS evidence must be gathered against that final tree.
- **To close:** commit and push one immutable cycle-11 implementation identifier to
  `origin`; obtain green native `cd sniff && just test` and `cd sniff && just lint` runs on
  macOS, Linux, and Windows; run the `work_counts` example natively on each; and retain all
  three `sniff-work-counts-{os}` artifacts plus the test and lint run links under that same
  identifier. Cross-compilation, Docker, WSL, or results from a different tree do not close
  this finding.
