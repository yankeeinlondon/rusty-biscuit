# Implementation Plan: Performance Testing for Sniff

Design: `sniff/features/2026-04-11-performance-testing/design.md`

## Confidence: HIGH

This plan is high confidence because the core pieces the design depends on already exist:

- `sniff` already exposes stable, benchmarkable library entry points such as `detect_with_plan`, `detect_filesystem_with_request`, `detect_hardware_with_request`, `detect_git_with_request`, `ProgramsInfo::detect`, and `detect_services`.
- The library already has structured stage/counter instrumentation in `sniff/lib/src/performance.rs`, and the CLI already exposes `--perf`, so benchmark work can align with existing stage names instead of inventing a second observability path.
- `sniff/justfile` is area-local and can absorb benchmark/profile recipes without touching unrelated workspace packages.
- There is no existing benchmark harness in `sniff/lib`, so this can be introduced cleanly rather than migrated.
- Existing fixture helpers in `sniff/lib/tests/fixtures.rs` show the preferred tempdir/git2 style, even though those helpers are not directly reusable from Criterion benches.

## Codebase-Driven Adjustments to the Design

The design is directionally correct, but three implementation details should be adjusted to keep confidence high:

1. **Criterion layout**
   The design suggests `sniff/lib/benches/main.rs`. On stable Cargo, Criterion is simplest if we add a single `harness = false` bench target and keep domain groupings as modules under a support directory. Recommended layout:
   - `sniff/lib/benches/perf.rs`
   - `sniff/lib/benches/cases/system.rs`
   - `sniff/lib/benches/cases/hardware.rs`
   - `sniff/lib/benches/cases/filesystem.rs`
   - `sniff/lib/benches/cases/inventory.rs`
   - `sniff/lib/benches/support/{fixtures,plans,util}.rs`

2. **Fixture reuse**
   The design says to expand `sniff/lib/tests/fixtures.rs`. Criterion benches compile as separate targets and cannot directly import integration-test helper files. The safe approach is to create benchmark fixture builders under `sniff/lib/benches/support/fixtures.rs`, then optionally promote shared builders later if test and bench needs converge.

3. **Profiling profile**
   The design proposes `[profile.release] debug = true`. That is broader than necessary. A safer implementation is a dedicated root Cargo profile:
   - `[profile.profiling] inherits = "release"`
   - `debug = true`
   - `strip = "none"`
   
   Then `cargo flamegraph --profile profiling ...` gives usable stacks without changing normal release builds.

## Step 1: Benchmark Foundation

**Files:**
- `sniff/lib/Cargo.toml`
- `Cargo.toml`

**Changes:**
- Add `criterion` to `sniff/lib` `dev-dependencies`.
- Add a single Criterion bench target in `sniff/lib/Cargo.toml`:
  - `[[bench]]`
  - `name = "perf"`
  - `harness = false`
- Add a root `[profile.profiling]` profile for flamegraphs.

**Why first:**
- This unlocks bench compilation, HTML report generation, and profiling without changing runtime behavior.

**Confidence:** HIGH

## Step 2: Add Bench Support Modules and Stable Fixture Builders

**Files:**
- `sniff/lib/benches/perf.rs`
- `sniff/lib/benches/support/fixtures.rs`
- `sniff/lib/benches/support/plans.rs`
- `sniff/lib/benches/support/util.rs`
- `sniff/lib/benches/cases/mod.rs`

**Changes:**
- Create a single Criterion entry point in `perf.rs`.
- Add shared helpers for:
  - reusable `DetectionPlan` builders
  - common Criterion configuration and throughput labels
  - fixture setup for git repos and monorepos
  - tempdir management for large synthetic repos
- Build fixtures once per benchmark group, not per iteration.
- Use deterministic directory shapes and commit histories so filesystem results are comparable across runs.

**Fixture set to support initially:**
- `small_git_repo()`: about 10 files, 5 commits, a few dirty files.
- `large_monorepo()`: hundreds of packages, thousands of files, nested workspace manifests, deep git history.
- `language_mix_tree()`: shallow and deep directory trees for language/file-type scanning.
- `wiremock_network_fixture()`: deferred until network benches are added; not required for phase 1.

**Implementation note:**
- Do not include fixture creation time in the measurement loop unless the benchmark is explicitly about setup cost.

**Confidence:** HIGH

## Step 3: Land System-Level Criterion Benches

**Files:**
- `sniff/lib/benches/cases/system.rs`

**Bench coverage:**
- `detect_minimal`: `DetectionPlan::new().without_os().without_hardware().without_network().without_filesystem()`
- `detect_summary`: summary-level OS/hardware, interfaces-only network, lightweight filesystem request
- `detect_full`: default full `DetectionPlan`

**Implementation details:**
- Use explicit plan builders in `support/plans.rs` so names and request levels remain stable.
- For `detect_full`, set `base_dir` to a controlled fixture root when filesystem cost is part of the comparison.
- Capture and label stage breakdowns from the existing `performance` collector in benchmark setup notes, but keep Criterion’s wall-clock numbers as the primary assertion surface.

**Why this order:**
- These benches answer the design’s top-level question: what is the total cost of each detection tier?

**Confidence:** HIGH

## Step 4: Add Hardware Benches Around Known Slow Paths

**Files:**
- `sniff/lib/benches/cases/hardware.rs`

**Bench coverage:**
- `detect_simd`
- `detect_hardware_with_request(HardwareRequest::summary())`
- `detect_audio_devices`
- `detect_storage`
- `detect_gpus`

**Platform handling:**
- Always benchmark `detect_simd` and hardware summary.
- Gate audio/GPU benches with `#[cfg(...)]` where platform-specific implementations exist.
- Keep benchmark names stable even if some are omitted on non-macOS hosts; the just/docs layer should make this explicit.

**Why this order:**
- The design calls out audio enumeration and GPU detection as suspect paths, and existing `HardwareRequest` APIs already separate those costs cleanly.

**Confidence:** HIGH

## Step 5: Add Filesystem Benches Using Synthetic Repos

**Files:**
- `sniff/lib/benches/cases/filesystem.rs`

**Bench coverage:**
- `detect_git_with_request(...summary...)` for branch + dirty counts
- `detect_git_with_request(...full...)` for recent commits and file changes
- `detect_repo_structure`
- `detect_repo_with_inventory`
- `detect_languages`
- `detect_filesystem_with_request` for full staged filesystem analysis

**Comparisons to encode:**
- git summary vs full vs deep
- repo structure-only vs full inventory
- file inventory on shallow vs deep trees
- shared-walk filesystem detection vs separate component calls

**Why this matters:**
- The most meaningful Sniff performance risks live in filesystem traversal, git history inspection, and monorepo/package discovery.

**Confidence:** HIGH

## Step 6: Add Programs and Services Inventory Benches

**Files:**
- `sniff/lib/benches/cases/inventory.rs`

**Bench coverage:**
- `ProgramsInfo::detect()`
- `detect_services()`

**Implementation details:**
- Benchmark the full program inventory path because it exercises the shared `ExecutableIndex` plus Rayon fan-out, which is exactly the design’s stated optimization target.
- Keep services enumeration separate because init-system detection and service listing have very different host variance.

**Confidence:** HIGH

## Step 7: Add Profiling Entry Points for Flamegraphs

**Files:**
- `sniff/lib/examples/profile_detect_full.rs`
- `sniff/lib/examples/profile_filesystem.rs`
- optionally `sniff/lib/examples/profile_hardware.rs`

**Changes:**
- Add tiny example binaries that invoke one focused hot path each.
- Prefer example binaries over running Criterion under flamegraph; examples are easier to invoke, easier to document, and produce cleaner stacks.

**Profile targets to support first:**
- full library detection
- filesystem-heavy detection against the large monorepo fixture
- hardware detection on the current machine
- CLI full JSON path via the existing `sniff` binary

**Confidence:** HIGH

## Step 8: Add `just` Automation for Benches and Profiles

**Files:**
- `sniff/justfile`
- `sniff/just.md`

**Recipes to add:**
- `just bench`
  Runs `cargo bench -p sniff --bench perf`
- `just bench-system`
- `just bench-hardware`
- `just bench-filesystem`
- `just bench-inventory`
- `just bench-cli`
  Runs `hyperfine` against selected CLI commands
- `just profile <target>`
  Runs `cargo flamegraph --profile profiling -p sniff --example <target>`
- `just profile-cli <args...>`
  Runs `cargo flamegraph --profile profiling -p sniff-cli -- <args>`

**CLI benchmark set:**
- `sniff --version`
- `sniff --json`
- `sniff hardware --json`

**Implementation detail:**
- Keep `hyperfine` optional but fail with a clear message if the recipe is invoked without it installed.

**Confidence:** HIGH

## Step 9: Document the Workflow and Dependency Changes

**Files:**
- `sniff/lib/README.md`
- `docs/dependencies.md`
- optionally `.claude/skills/sniff/SKILL.md` if workflow guidance is updated as part of implementation

**Changes:**
- Document how to run Criterion benches, where HTML output lands, and how to run flamegraphs.
- Record `criterion` as a new dependency in `docs/dependencies.md`.
- Document platform-specific expectations for hardware benches.
- Document which benchmarks are intended for regression tracking vs exploratory profiling.

**Confidence:** HIGH

## Step 10: Stage CI Integration Instead of Hard-Gating Immediately

**Files:**
- likely new `.github/workflows/sniff-performance.yml`

**Recommended rollout:**

### Phase A: Artifact-only CI
- Trigger on `sniff/**`, root `Cargo.toml`, and workflow changes.
- Run a narrow benchmark subset on a single Linux runner.
- Upload Criterion output and any hyperfine JSON as artifacts.
- Do not fail the build on regressions yet.

### Phase B: Baseline comparison
- Compare current branch against `main` for a small, stable subset:
  - `detect_summary`
  - `detect_full`
  - `detect_git_summary`
  - `ProgramsInfo::detect`
- Publish a markdown summary in the workflow output.

### Phase C: Threshold-based enforcement
- Only after runner noise is characterized, fail on regressions above the agreed threshold.
- Start with Linux-only enforcement and keep macOS benchmarks informational until variance is understood.

**Reason for phasing:**
- The repo currently has no performance workflow, no stored baseline system, and no evidence yet about runner variance. Immediate PR hard-gating at 15% is lower confidence than the rest of the design.

**Confidence:** MEDIUM for enforcement, HIGH for staged rollout

## Validation Checklist for the Implementation

- `cargo bench -p sniff --bench perf` compiles and produces Criterion output.
- `just bench` runs successfully in the `sniff` area.
- `just profile profile_detect_full` produces a readable flamegraph.
- `just bench-cli` benchmarks the selected `sniff` CLI commands with `hyperfine`.
- Bench fixtures are deterministic and do not rebuild inside the hot loop.
- Bench code compiles on non-macOS hosts without referencing unavailable audio/GPU APIs.
- Documentation explains how to interpret Criterion vs `--perf` stage timings.

## Deliverables

1. Criterion-based library benchmark harness for `sniff/lib`
2. Deterministic fixture builders for filesystem and git workloads
3. Flamegraph-ready profiling entry points
4. `just` recipes for library, CLI, and profiling workflows
5. Documentation for running and maintaining the performance suite
6. A staged CI plan that starts with artifact collection and only later adds regression gates
