# Design: Performance Testing for Sniff

**Date:** 2026-04-11  
**Status:** Draft  
**Area:** sniff (lib & cli)

## Objective

Introduce a robust performance testing and profiling framework for the Sniff library and CLI to:
1.  **Identify Bottlenecks:** Pinpoint slow detection modules (e.g., audio device enumeration, deep git inspection).
2.  **Prevent Regressions:** Ensure new features or refactors don't negatively impact detection latency.
3.  **Optimize Hot Paths:** Provide empirical data for optimizing parallel execution (Rayon) and filesystem traversal.
4.  **Quantify Overhead:** Measure the cost of different `DetectionPlan` detail levels.

## Tooling Selection

### 1. Criterion (Micro/Macro-benchmarking)
Criterion is the industry standard for Rust benchmarking.
-   **Why:** Provides statistically significant results, handles warm-up, and generates rich HTML reports with trend analysis.
-   **Use Case:** Benchmarking library-level functions like `detect()`, `detect_hardware()`, and `detect_git()`.

### 2. cargo-flamegraph (Profiling)
Flamegraphs provide a visual representation of where time is spent in the call stack.
-   **Why:** Essential for identifying unexpected "heavy" functions or syscalls.
-   **Use Case:** Profiling a full detection run to find hidden bottlenecks.

### 3. Hyperfine (CLI Benchmarking)
Hyperfine is a command-line benchmarking tool.
-   **Why:** Better suited for end-to-end CLI execution than Criterion, as it includes process startup time and shell overhead.
-   **Use Case:** Measuring `sniff --json` vs `sniff hardware`.

## Benchmarking Strategy

### Library Benchmarks (`sniff/lib/benches/`)

Benchmarks will be categorized by domain to allow targeted performance analysis.

#### 1. System Benchmarks (`benches/system.rs`)
Focuses on the high-level API.
-   `bench_detect_summary`: Full system detection with `summary()` requests.
-   `bench_detect_full`: Full system detection with default `full()` requests.
-   `bench_detect_minimal`: Detection with all domains skipped (baseline overhead).

#### 2. Hardware Benchmarks (`benches/hardware.rs`)
Focuses on hardware enumeration which often involves platform-specific syscalls.
-   `bench_cpu_simd`: SIMD capability detection.
-   `bench_audio_devices`: Enumeration of input/output devices (known slow path on macOS).
-   `bench_gpu_metal`: Metal API detection (macOS only).
-   `bench_storage_enumeration`: Disk and mount point scanning.

#### 3. Filesystem Benchmarks (`benches/filesystem.rs`)
Requires standardized fixtures to ensure reproducible results.
-   `bench_git_status`: Measuring dirty file count and branch detection.
-   `bench_git_history`: Retrieving recent commits (10 vs 100).
-   `bench_monorepo_discovery`: Scanning for packages in a Cargo/pnpm workspace.
-   `bench_language_breakdown`: Extension-based scanning of varying directory depths.

#### 4. Programs & Services (`benches/inventory.rs`)
-   `bench_programs_parallel`: Rayon-based parallel detection of 8 program categories.
-   `bench_services_enumeration`: Enumerating systemd/launchd services.

### CLI Benchmarks

CLI benchmarks will be orchestrated via `just` and use `hyperfine`.

-   **Cold Start:** `sniff --version` (measures overhead of library linking and clap parsing).
-   **Standard Run:** `sniff --json` (measures full detection + JSON serialization).
-   **Filtered Run:** `sniff hardware --json` (measures targeted detection).

## Fixture Management

To ensure filesystem benchmarks are consistent across environments (Dev, CI, macOS, Linux):
1.  **Small Repo Fixture:** A standard git repo with ~10 files and 5 commits.
2.  **Large Monorepo Fixture:** A generated directory structure with hundreds of "packages", thousands of files, and a deep git history.
3.  **Mocking:** Use `wiremock` for network-related benchmarks (e.g., WAN IP lookup) to avoid external latency variance.

## Profiling Workflow

A dedicated `just` recipe will be added to generate flamegraphs:

```bash
# Profile the library's detect() function
just sniff profile detect-full

# Profile the CLI
just sniff profile-cli hardware
```

The workflow involves:
1.  Compiling in release mode with debug symbols enabled (`[profile.release] debug = true`).
2.  Running `cargo flamegraph` against a dedicated example or benchmark.
3.  Saving the resulting `.svg` to `sniff/target/flamegraph.svg`.

## CI/CD Integration

### Regression Detection
Benchmarks should run on every PR to the `sniff` package area.
-   **Baseline Comparison:** Compare PR results against the `main` branch.
-   **Failure Threshold:** Fail CI if a core detection path (e.g., `detect_os`) regresses by more than 15%.

### Performance Tracking
Store Criterion results as artifacts or upload them to a performance dashboard to track the "cost of Sniff" over time.

## Implementation Plan

### Step 1: Foundation
-   Add `criterion` to `sniff/lib/Cargo.toml` dev-dependencies.
-   Add `[profile.release] debug = true` to the root `Cargo.toml` (or scoped to sniff).
-   Create `sniff/lib/benches/main.rs` as the benchmark entry point.

### Step 2: Core Benchmarks
-   Implement `benches/system.rs` for `detect_with_plan`.
-   Implement `benches/hardware.rs` focusing on audio and CPU.

### Step 3: Filesystem Fixtures
-   Expand `tests/fixtures.rs` to support benchmark-scale generated environments.
-   Implement `benches/filesystem.rs`.

### Step 4: Automation
-   Add `just bench` and `just profile` recipes to `sniff/justfile`.
-   Add `hyperfine` scripts for CLI verification.

## Future Considerations
-   **Allocation Tracking:** Use `dhat` or `coz` to track memory allocations and causal bottlenecks.
-   **Continuous Profiling:** Integrate with tools like Parca or Pyroscope for long-running environments.
