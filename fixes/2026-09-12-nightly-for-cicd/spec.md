---
status: draft
created: 2026-09-12
area: repo
packages: []
---

# Evaluate nightly Rust for faster CI

## Objective and scope

Determine whether a pinned nightly compiler can materially shorten this
monorepo's CI feedback loop while preserving correctness, platform coverage,
and stable Rust compatibility. Adopt only a configuration that passes the
evaluation below. A documented decision to retain stable is a successful
outcome when the hypothesis does not hold.

This document specifies future experiments and conditional implementation.
No benchmark has been run for this proposal, and no speedup is established
for rusty-biscuit. The initial change is this draft only.

The primary hypothesis is that parallel frontend compilation reduces the
time spent building our test binaries and checking large dependency graphs.
The secondary hypothesis is that faster code generation can reduce total
build-plus-test time. Test execution is a separate outcome: changing the
compiler cannot eliminate intentional waits, subprocess startup, network
latency, or test serialization.

Success means faster required-CI completion, measured from workflow creation
to its required verdict, with queue time and execution time also reported
separately. Compiler microbenchmarks alone cannot justify adoption.

## Research findings

Research reviewed on 2026-09-12. The links below are primary sources; rolling
documentation must be checked against the exact toolchain selected for the
experiment.

| Candidate | Evidence and limits | Disposition |
| --- | --- | --- |
| Nightly without extra flags | A channel change includes newer compiler changes but does not isolate a particular optimization. The original parallel frontend announcement explicitly required opting into multiple threads. | Required control, not an assumed improvement. |
| Parallel frontend | Rust's November 2023 measurements reported compile-time reductions up to 50% with eight threads, with substantial variation and memory increases up to 35%. These are historical results on other workloads, not forecasts for this repo. | First experiment: retain LLVM and change frontend parallelism only. |
| Cranelift backend | Intended to improve debug compilation. Its current support table includes Linux, macOS, and x86_64 Windows, but not every target has a rustup component. Its README lists partial architecture-intrinsic support and experimental unwinding unavailable on Windows and macOS. | Separate, optional compatibility spike before performance trials. |
| Stable configuration changes | Reduced debug information and alternative linkers can improve build times without establishing any benefit from nightly. | Keep current settings fixed; evaluate other changes independently. |

The historical parallelism figures and opt-in behavior come from the
[Rust parallel frontend announcement](https://blog.rust-lang.org/2023/11/09/parallel-rustc/).
The [Cargo build-performance guide](https://doc.rust-lang.org/cargo/guide/build-performance.html)
still describes `-Zthreads` as experimental and warns that Cranelift may
compile faster while executing generated code more slowly. The
[Cranelift project README](https://github.com/rust-lang/rustc_codegen_cranelift)
is the source for its platform and semantic restrictions.

Do not extrapolate the 2023 article's planned stabilization date. The
[compiler development guide](https://rustc-dev-guide.rust-lang.org/parallel-rustc.html)
explicitly warns that parts of its parallelism documentation are outdated.
At experiment setup, record `rustc -vV`, `cargo -V`, compiler help, and nightly
`rustc -Z help`; verify accepted flags, defaults, and target support using a
small compile probe. If the relevant optimization has reached stable, move
it into a stable arm and prefer that route.

Some frequently advertised improvements are already stable. Rust 1.90 made
LLD the default linker for `x86_64-unknown-linux-gnu`; a comparison against
an older compiler can therefore overstate nightly's unique benefit.
See the [Rust 1.90 release announcement](https://blog.rust-lang.org/2025/09/18/Rust-1.90.0/).
Record the actual linker for each measured target.

Do not bundle panic-abort, changed optimization levels, workspace feature
unification, new linkers, or a compiler cache into the initial trial.
Changing panic behavior or feature selection could change what the tests
prove. Neither a faster nextest installation nor a switch from `cargo test`
to nextest is evidence for this hypothesis: the repo already uses nextest.

## Repository baseline

Source inspection baseline: `cd9fdb6221b347c1885a25019b0e7b6b760dfac5`.
Recheck these facts at implementation time; concurrent CI-performance work
must not become an unrecorded difference between measurement arms.

| Existing surface | Implication for the experiment |
| --- | --- |
| [`rust-toolchain.toml`](../../rust-toolchain.toml) pins `1.97.1`, with clippy and rustfmt | Keep this as the stable development and release authority. Do not replace it with floating nightly. |
| [`Cargo.toml`](../../Cargo.toml) uses dev `debug = "line-tables-only"` and dependency `debug = 0` | Reduced debug information is already in the baseline. Freeze profiles, optimization, incremental settings, and codegen units across arms. |
| [`_package-ci.yml`](../../.github/workflows/_package-ci.yml) selects individual Cargo packages and declared features | Benchmark actual package selections, including fixture prebuilds and native prerequisites. An area-wide build can unify different features and is not equivalent. |
| Check, lint, and L1 are independent gates; L2/browser work follows L1 | Faster L1 might not shorten the verdict if stable lint/check or another package remains the bottleneck. |
| [`just/devops.just`](../../just/devops.just) owns tier selection and JUnit staging | Continue using canonical recipes; preserve slow-test policy, fixtures, and failure propagation. Require nextest to be available so the legacy fallback is never measured. |
| [`.config/nextest.toml`](../../.config/nextest.toml) has a CI profile, zero retries, and package-specific concurrency limits | Keep runner version, filters, concurrency, timeouts, and retry policy identical. Compiler thread counts and test thread counts are separate controls. |
| [`_wsl-ci.yml`](../../.github/workflows/_wsl-ci.yml) builds a Linux nextest archive and executes it in WSL2 without a Rust toolchain | Apply compiler changes on the Linux archive builder. Measure archive creation/transfer and WSL execution separately. Preserve workspace remapping and sidecars. |
| Required CI uses `Swatinem/rust-cache`, with `RUSTC_WRAPPER` cleared | Do not add kache/sccache to the experiment. Toolchain and flag changes need isolated cache identities. |
| [`rust-latest-stable.yml`](../../.github/workflows/rust-latest-stable.yml) is an existing advisory workflow | Use the same separation from required gates for experimental nightly runs. Resolve floating stable once to an exact version for comparisons. |
| [`ci_workflow_contracts.rs`](../../tools/test-toolkit/tests/ci_workflow_contracts.rs) protects toolchain authority and workflow behavior | Any eventual required-CI override needs an explicit policy change and corresponding contract tests. |
| [CI documentation](../../.github/ci/README.md) describes pre-push evidence and reuse of successful PR validation | Experimental results must never satisfy stable receipts or cause a platform's benchmark jobs to be omitted. Review both reuse paths before rollout. |

The [runner notes](../../.claude/skills/os/ci-runners.md) record 5–15%
run-to-run noise, relatively constrained macOS runners, slow Windows builds,
and cache eviction across broad runs. These are planning inputs, not newly
measured facts. Discover actual CPU, memory, OS, and architecture through
`sniff` in each run and retain the runner image version. Do not assume an
eight-thread setting suits hosted CI.

Existing proposals for batching, archive reuse, and fixture improvements in
[`2026-08-24-cicd-opportunities`](../2026-08-24-cicd-opportunities/spec.md)
remain separate. Freeze their implementation state for each comparison.

## Experiment design

### Phase 1: establish controls and attribution

Create a manual, non-required experiment workflow. It must accept a fixed
source SHA and a bounded package/variant selection, reuse existing CI policy
and provisioning, and publish results outside the required rollup. Do not
duplicate the complete package workflow or introduce a second feature-policy
catalog. Its concurrency group must not cancel normal CI or another sample.

Select a small cohort initially:

- `biscuit-hash`: small-library control, including a case where setup dominates.
- `darkmatter` and `darkmatter-cli`: heavy library and real CLI build/test paths.
- `claudine-cli`: subprocess-heavy execution where compiler gains may be diluted.
- `sniff`: platform-dependent behavior and existing Windows concurrency policy.

Use each package's CI feature and tier declarations, not guessed flags.
Include relevant fixture builds, such as the `md` fixture for Claudine.
Use the same source SHA and lockfile for every arm; require locked dependency
resolution and report any mutation as an invalid sample. Start with Linux
screening, then compare finalists independently on native macOS, native
Windows, and Linux plus WSL2 archive execution. Before broad adoption, expand
to every package/target affected by the proposed policy, including native
dependencies, proc macros, examples, benches, and auxiliary targets that CI
currently checks.

| Arm | Toolchain and settings | Question |
| --- | --- | --- |
| S0 | Repo's exact stable pin; existing settings | Current baseline. |
| S1 | Latest available stable, resolved and pinned once; existing settings | Would a stable upgrade achieve the improvement? Omit duplicate runs if identical to S0. |
| N0 | One exact `nightly-YYYY-MM-DD`; existing settings and LLVM | Effect of the compiler revision/channel without added flags. |
| N1 | Same nightly and LLVM; frontend threads set to 1 | Explicit serial control if N0's defaults differ or are uncertain. |
| N2 | Same nightly; frontend threads set to 2 | Modest parallelism on hosted runners. |
| NC | Same nightly; frontend threads set to detected available CPU count, capped at 8 | Whether additional parallelism helps within that runner's limits. Deduplicate against N2. |

Choose a dated nightly available for all required host/target combinations;
record the component availability check. Do not invent a pin in advance or
silently use different dates on different OSes. A missing component is an
unavailable arm, never permission to fall forward to floating nightly.

Hold Cargo's job budget constant per environment while varying frontend
threads. Cargo and rustc coordinate concurrency through a jobserver, so
`Cargo jobs × frontend threads` is not a reliable prediction of active CPU
use. Measure CPU utilization and memory instead. If subsequent tuning of
Cargo jobs is worthwhile, label it as a separate experiment and give stable
the same opportunity.

For an eligible frontend probe, use `RUSTUP_TOOLCHAIN` to select the exact
toolchain for the canonical recipe and all fixture subprocesses. The
conceptual invocation is:

```sh
# NIGHTLY_PIN is the verified date recorded in the experiment manifest.
RUSTUP_TOOLCHAIN="$NIGHTLY_PIN" RUSTFLAGS="-Zthreads=2" \
  NEXTEST_PROFILE=ci just _test biscuit-hash --locked --no-fail-fast
```

This is an illustrative L1 command, not a complete benchmark driver. The
driver must preserve existing flags, handle `CARGO_ENCODED_RUSTFLAGS`
precedence, and verify effective compiler invocations. Use an explicit child
environment on Windows rather than relying on POSIX assignment syntax.
Do not set `RUSTC_BOOTSTRAP`, change a developer's default toolchain, or add
unstable settings to the root manifest for the experiment.

### Phase 2: measure reproducibly

Use fresh disposable checkouts or isolated build locations for each arm.
Honor the native Windows build-host target-volume contract; do not override
that host's `CARGO_TARGET_DIR`. Never clear a developer's working cache.
Keep fixture paths valid when selecting an isolated target location.

Measure these cache conditions separately:

1. **Cold artifacts:** no compiled artifacts; compiler/tools and fetched
   dependencies are prepared consistently. Time provisioning separately.
2. **CI restore:** restore only that arm's artifacts through the production
   cache policy. Record exact/partial/missed restores, bytes, and restore/save
   time. An intended warm run with a cache miss is a cold sample.
3. **Warm changed source:** apply the same reviewed, deterministic leaf-source
   patch after warming each arm with its exact recipe. Record the patch and
   confirm recompilation. An unchanged-tree no-op build is only a diagnostic.

Prioritize cold builds and actual restore behavior for the adoption decision;
do not generalize local incremental results to ephemeral CI. A rolling
nightly would regularly invalidate artifacts, which is another reason to pin.
The [rust-cache documentation](https://github.com/Swatinem/rust-cache)
describes compiler/environment-based keys and requires toolchain selection
before cache setup. Add an experiment namespace including variant, platform,
package/features, and flags; prevent cross-arm restore fallbacks. Avoid
filling production cache quota with an unbounded experiment matrix.

For screening, collect three alternating baseline/candidate pairs per cell.
Run one arm at a time on a shared host; alternate AB/BA order or randomize
blocks. Keep failed attempts in the evidence ledger. Advance only promising
arms to at least ten paired samples per decision-critical environment/cache
cell, spread across at least three hosted runs. Hosted runners are not the
same machine: match image/CPU class and record pairing provenance.

Reserve those confirmation samples for the selected configuration; do not
choose a winning thread count and claim statistical confirmation using only
the samples that selected it. If the baseline changes, restart the affected
comparison rather than splice together revisions.

Capture distinct measurements:

- Queue delay, provisioning/toolchain installation, dependency fetch, cache
  restore/save, fixture prebuilds, compile/link, test discovery, runner elapsed,
  artifact upload/download/extraction, and complete job elapsed.
- Whole-workflow required-verdict latency and aggregate runner minutes. Sum
  of jobs measures compute consumption; the dependency path to the verdict
  measures turnaround. Include stable gates retained by the proposed rollout.
- Peak memory with collector/method, available memory, CPU utilization,
  artifact sizes, cache hit state, exit statuses, compiler ICEs, and timeouts.
- Nextest identities, passes, failures, skips, retries, slow tests, and summed
  test durations. The sum of test durations is not parallel runner wall time.

Reuse `tools/test-audit` for capture, JUnit comparison, attribution, and
measurement parsing. Extend that shared tool only where compiler-phase or
paired-toolchain reporting is missing; do not create a competing report
parser under this fix directory. Preserve per-variant staging directories
because canonical recipes reset their JUnit staging area.

Obtain compile-only diagnostics using the selected nextest version's
supported build-without-run path with exactly the recipe's package/features
and tier settings. Measure execution using those binaries or an equivalent
no-rebuild run, and confirm no compilation occurred. Separately measure the
unaltered canonical command end to end; diagnostic prebuilding must not make
the headline sample artificially warm.

Use [`cargo --timings`](https://doc.rust-lang.org/cargo/reference/timings.html)
where supported to identify expensive compilation units and dependency
bottlenecks. It does not fully expose compiler-internal concurrency.
Do not assume legacy `--timings=json` syntax exists: the
[Cargo unstable reference](https://doc.rust-lang.org/cargo/reference/unstable.html)
records its removal. Capture supported HTML reports and external phase
timers; run heavier nightly self-profiling separately from timed trials.

Report median, min/max drift, p90 (explicitly exploratory at ten samples),
each paired improvement `1 - candidate_seconds / baseline_seconds`, and a
95% paired-bootstrap confidence interval for the median improvement. Record
the bootstrap method and seed. Failed/canceled runs are not latency wins;
report their frequency and reason instead of silently discarding them.

### Phase 3: optional Cranelift spike

Proceed only if attribution shows code generation is a major remaining cost.
Use the same dated nightly as the LLVM control and a separately identified
backend arm. Verify the preview component exists for each target, that the
test profile actually selects Cranelift, and that host build scripts and
fixture binaries use the intended backend.

First exercise existing panic/unwind, `catch_unwind`, `should_panic`, native
FFI, and architecture-intrinsic coverage where present. Missing support
disqualifies the affected scope. Do not remove tests, switch panic strategy,
or force LLVM fallbacks without recording the resulting hybrid as a distinct
candidate. A test process per nextest test does not prove unwind semantics
are interchangeable. Measure full build-plus-execution time against nightly
LLVM and stable. Keep release artifacts and performance-sensitive runtime
benchmarks on the production LLVM toolchain.

## Decision gates

These are proposed thresholds to agree before collecting confirmation data,
not measurements or promises. Freeze them in the experiment manifest.

| Gate | Requirement |
| --- | --- |
| Correctness | Same selected test identities and behavior within each environment; no new failures, retries, timeouts, ignored tests, or weakened assertions. All selected required gates pass. Document legitimate cross-platform exclusions separately. |
| Compilation benefit | At least 20% median reduction in the compile/build phase for at least two costly cohort packages on an environment proposed for adoption; 95% paired interval excludes zero benefit. |
| Turnaround benefit | At least 15% and 60 seconds lower median required-verdict latency on matched representative workflow scopes; interval excludes zero. Include installation, caches, archive transfer, and retained stable checks. Reproduce on both narrow and broad scopes before repo-wide rollout. |
| Compute cost | No more than 5% increase in aggregate runner minutes for the final topology, including stable compatibility work. Advisory duplication during the experiment is reported separately. |
| Resource and runtime guard | No OOMs or compiler hangs; retain at least 20% runner-memory headroom under the measured peak. A sustained test-execution or unaffected-job regression above 5% requires investigation and blocks rollout until resolved. |
| Reliability | At least three consecutive fully green hosted confirmation runs per adopted environment, with every intervening failure disclosed. This is a minimum observation window, not proof that rare compiler bugs cannot occur. |
| Stable compatibility | Stable compilation, lint policy, and required behavioral coverage remain enforced. Nightly-only source features or dependency-resolution changes are outside scope. |

Compare against S0 and S1. If a stable upgrade gives an improvement within
measurement uncertainty of nightly, choose stable. A 5–15% anecdotal
per-job change without stronger evidence is inconclusive under the repo's
recorded runner noise. Extend confirmation once, up to twenty pairs per
cell; if uncertainty remains, retain stable and record the result.

Platform-specific adoption is allowed only through an explicit environment
policy with stable retained elsewhere. Linux or WSL2 success cannot establish
native Windows behavior. Existing Windows/WSL2 L2 provisioning gaps remain
unmet criteria, with their owners and required provisioning work recorded;
they must not become exclusions justified by this optimization. Validate
available L2/headless browser routes without gaining focus. Do not run L3.
Do not claim full cross-platform tier validation while gaps remain.

## Conditional implementation and rollout

1. **Publish the decision.** Add `results.md` beside this spec with the exact
   source/toolchains, configurations, run URLs, raw artifact references,
   matched coverage, statistics, gaps, and decision for each environment.
   Record “reject,” “inconclusive,” “stable upgrade,” or the exact nightly
   configuration. Explain the remaining bottleneck if compilation improves
   but the verdict does not.
2. **Prefer stable where sufficient.** Use the existing toolchain-upgrade
   workflow when S1 wins; follow its compatibility checks. Do not retain a
   nightly dependency merely because the investigation started with nightly.
3. **Canary a nightly winner.** Keep the root stable pin. Introduce one
   versioned CI experiment/policy record for the nightly date, flags, and
   environment allowlist, shared by hosted and local reproduction paths.
   Run the selected candidate as advisory for one week and at least three
   green hosted runs per adopted environment. Validate a second dated nightly
   before adoption to show the configuration can be maintained; a failed
   upgrade does not justify changing the known-good pin.
4. **Change only proven gates.** Default proposal: opt selected test builds
   and their Linux archive builders into nightly; keep stable check/lint and
   release authority. Cover all associated fixture, L2, and browser build
   paths consistently. Keep stable behavioral test coverage as a required
   check for the affected scope. If that topology erases the benefit or
   exceeds the compute budget, reject rollout under this spec; reducing
   stable coverage needs a separate policy decision.
5. **Preserve evidence identity.** Extend workflow inputs, cache and artifact
   identities, JUnit provenance, pre-push validation, and PR/main reuse
   receipts as necessary so toolchain/backend/flags cannot be confused.
   Do not permit advisory results to satisfy production gates. Ensure a
   changed CI toolchain policy invalidates stale validation receipts.
6. **Maintain and roll back.** Nominate a maintainer before promotion. Review
   pinned-nightly updates monthly through advisory compatibility and timing
   checks. Never auto-follow nightly. Retain a single policy switch back to
   stable; remove unstable flags/components in the same rollback, restore
   stable cache identity, and rerun affected gates. Missing toolchains, ICEs,
   OOMs, or recurring regressions trigger rollback rather than silent fallback
   or relaxed thresholds. Exercise rollback during the canary.

Before editing implementation symbols, run GitNexus impact analysis for
their callers and execution flows. Expected review surfaces are
`_package-ci.yml`, `_wsl-ci.yml`, `ci.yml`, CI scope/reuse code, shared Just
recipes, and `tools/test-toolkit` workflow contracts. Add focused tests for
policy selection, toolchain provenance, cache separation, rollback, and
rejected/missing evidence. Verify that unsupported variants fail explicitly.
Run the applicable CI-infrastructure self-tests, `actionlint` on changed
workflows, `just test test-toolkit`, and `just check` in `tools/test-audit`
if its implementation changes. Use package `just test`, `just test-l2`, and
`just lint` as applicable; no formatting run is authorized by this spec.

Update `.github/ci/README.md` and relevant OS/testing skills when workflows
change. Update dependency documentation only if dependencies change. Analyze
graph changes before any separately authorized commit.

## Deliverables and completion

- [ ] Draft reviewed; evaluation thresholds and experiment owner recorded.
- [ ] Manual advisory workflow and reproducible local route implemented.
- [ ] Baseline attribution and exact candidate/component manifest captured.
- [ ] Paired measurements, matched test evidence, and decision report published.
- [ ] Accepted configuration canaried, validated, and adopted, or stable retained
  with a documented rejection/inconclusive result and scoped follow-ups.
- [ ] Rollback and evidence-reuse contracts verified for any adopted change.
- [ ] Platform/tier gaps explicitly tracked; no unavailable route counted as pass.

Keep this fix active during evaluation. Move it to `_completed` only after
the decision and any adopted implementation are delivered, distinguishing
implementation completion from remaining platform verification.
