---
prompt: |-
    Do research on how to define a **strategy** for performance testing in a Rust project?
   
    > Note: assume that the project is using either [Criterion]() or [Devon]() as the performance testing crate

    Start by articulating how this strategy should be structured and defined in a crate neutral way and answer the following questions:
    
    1. How can they be used in a repo's workflow/lifecycle? Pre release? Every release? 
    2. Primary platform? Cross platform? 
    3. Should they be run in CI/CD? in local environments? Both?
    4. What parts of a repo make for good performance tests? Rather than testing every function, what is a good way to categorize areas to create a performance test for?
    5. How can you combine a regular cadence of performance tests while leaving flexibility to add adhoc tests too? 
    6. Since variability in the host (and the host's current state) can effect performance. How can the impact of this be minimized? How can one "name" a host in a way that automatically isolates some of these factors.

    Once completed with your research do a quick pass on how using **Criterion** versus **Devon** might vary your approach and use a few code examples to demonstrate.
last_updated: 2026-05-28
---
Let me start by checking the referenced research documents and understanding the crates involved.Now let me research Criterion and Devon to understand their capabilities.Good, I understand the format. Now let me compose the research document.

> **Note:** The research prompt references "Devon" but the actual Rust benchmarking crate is **Divan** (`nvzqz/divan`, v0.1.21). The crate registered as `devon` on crates.io is an unrelated Uzbek NLP library. This research proceeds using Divan as the intended comparison crate.

Here is the replacement Markdown body content:

---

## Defining a Performance Testing Strategy for Rust Projects

### Crate-Neutral Strategy Structure

A performance testing strategy should be defined in three layers:

1. **Policy Layer** -- What you measure, when you measure it, and how you decide pass/fail. This is entirely crate-agnostic.
2. **Infrastructure Layer** -- How benchmarks are organized on disk, how baselines are stored, and how results flow between local and CI environments. This is mostly crate-agnostic, with thin adapter scripts for the chosen crate's CLI.
3. **Implementation Layer** -- The actual benchmark functions, written using whatever crate you chose. This is the only layer that couples to a specific crate.

By keeping the policy and infrastructure layers crate-neutral, you can swap the implementation layer (or even run both) without redesigning your workflow.

---

### 1. How Can Performance Tests Be Used in a Repo's Workflow/Lifecycle?

**Pre-release (recommended):** Run the full benchmark suite against a stable baseline before cutting a release. Compare current measurements to the baseline stored from the previous release. Flag any regression exceeding a configured threshold (e.g., >5% slower on a critical path). This is the most impactful integration point because it catches regressions before users do.

**Every release (recommended):** After tagging a release, re-record baselines from the release commit on your designated benchmark machine. These baselines become the "source of truth" that future pre-release runs compare against. Commit baseline metadata (not raw data) to the repo so the comparison threshold is version-controlled.

**On every PR (optional, lightweight):** Run a fast subset of benchmarks (the "smoke" tier) on every pull request. This catches obvious regressions early but should not gate merging due to CI noise. Use a relaxed threshold (e.g., >20%) and only flag, never block.

**Ad hoc (always available):** Developers run specific benchmarks locally before and after a change they suspect impacts performance. This is the most common day-to-day usage.

**Cadence summary:**

| Trigger          | Scope                        | Threshold                | Action                      |
|------------------|------------------------------|--------------------------|-----------------------------|
| Every PR         | Smoke tier (5-10 benchmarks) | \>20% regression          | Comment on PR, do not block |
| Pre-release      | Full suite                   | \>5% regression           | Block release, investigate  |
| Post-release tag | Full suite                   | N/A (recording baseline) | Store new baseline          |
| Ad hoc           | Developer-selected           | Visual comparison        | Developer judgment          |

---

### 2. Primary Platform? Cross-Platform?

**Designate a single primary platform for authoritative baselines.** This is the machine (or CI runner class) where release baselines are recorded and pre-release comparisons happen. Pick the platform that most closely matches your production deployment. For most Rust projects this is `x86_64-unknown-linux-gnu` or `aarch64-unknown-linux-musl`.

**Cross-platform benchmarks are valuable but secondary.** They catch platform-specific regressions (e.g., different allocator behavior, different `Instant` resolution, different atomic costs) but should not be the gatekeepers. Run them for information, not for gating.

**Why single-platform baselines matter:** Performance numbers are not portable across architectures, OSes, or even different CI runner SKUs. A 10% regression on macOS ARM may be a 0% change on Linux x86_64. Comparing across platforms produces false positives and false negatives. Instead, each platform maintains its own baselines.

**Practical rule:** Record baselines on your primary platform. Run informational benchmarks on secondary platforms if you have the CI budget.

---

### 3. Should They Be Run in CI/CD? In Local Environments? Both?

**Both, with different roles.**

**CI/CD -- the discipline layer:**

- Runs the full suite on a schedule (nightly or pre-release) on dedicated, stable runners.
- Stores baselines as CI artifacts or in a dedicated branch (e.g., `perf/baselines`).
- Produces comparison reports that are posted as PR comments or CI summaries.
- **Critical caveat:** Virtualized CI environments are noisy. Criterion's own FAQ explicitly warns against relying on CI benchmark results. Divan's sample-size scaling is designed to partially mitigate this, but it cannot eliminate it. Use CI benchmarks for trend detection, not absolute gating (unless you have bare-metal runners).
- If you need deterministic CI performance gating, consider Valgrind-based tools like Iai/Iai-Callgrind that count instructions rather than measuring wall-clock time.

**Local -- the precision layer:**

- Developers run benchmarks on their own machines for ad hoc testing.
- Local runs on a quiescent machine produce the most reliable numbers.
- Local baselines are machine-specific and should never be compared against CI baselines.
- Provide a `just bench` or similar recipe that runs the suite with appropriate defaults and saves results to a predictable location.

**Anti-pattern to avoid:** Do not check `target/criterion` or benchmark output directories into the repo. Store only the baseline metadata (JSON/CSV summaries) needed for comparison.

---

### 4. What Parts of a Repo Make for Good Performance Tests?

Do not benchmark every function. Instead, categorize areas using a **tier system** based on the cost of regression and the likelihood of change:

#### Tier 1: Hot Paths (Always benchmark)

These are functions that execute millions of times per second or sit on the critical path of user-facing latency.

- **Parsing/serialization** -- Format parsers (TOML, YAML, JSON), serialization hot paths
- **Hashing** -- Core hashing functions (xxHash, BLAKE3), document fingerprinting
- **String processing** -- Text normalization, search, regex matching
- **Collections** -- Core data structure operations (insert, lookup, iteration) at realistic sizes

#### Tier 2: Integration Points (Benchmark regularly)

These are boundaries where your code interacts with external systems or where abstraction layers meet.

- **File I/O** -- Reading/writing files at various sizes
- **Network** -- Request/response serialization, WebSocket frame encoding
- **Database queries** -- Common query patterns (if applicable)
- **Rendering** -- Terminal output, diagram generation, SVG rasterization

#### Tier 3: Algorithmic Choices (Benchmark when changing)

These compare implementations to make informed decisions, but don't need continuous monitoring.

- **Algorithm comparisons** -- `HashMap` vs `BTreeMap`, different sort strategies
- **Allocation strategies** -- Pre-allocation vs dynamic growth, arena vs heap
- **Concurrency models** -- Thread pool sizing, channel throughput

#### Tier 4: Regression Guards (Add when fixing a perf bug)

When you fix a performance bug, add a benchmark that would catch its reintroduction.

- **Known-bad patterns** -- A benchmark that specifically exercises the code path that was slow
- **N^2 detection** -- Benchmarks at multiple input sizes to catch accidental quadratic behavior

**How to decide what to benchmark (heuristic):**

1. Profile your application under realistic load. The top 10 functions by exclusive time are your Tier 1 candidates.
2. Look at functions with complex logic that has changed recently. Those are Tier 2/3 candidates.
3. Add Tier 4 benchmarks reactively when you fix performance bugs.

---

### 5. Combining Regular Cadence with Ad Hoc Flexibility

Use a **benchmark registry pattern** that separates "what exists" from "what runs."

#### Structured Approach

Organize benchmarks into named groups that map to your tiers:

```text
benches/
├── hot_paths.rs          # Tier 1: always run, always compared
├── integration.rs        # Tier 2: run pre-release
├── algorithmic.rs        # Tier 3: run on demand
└── regression/
    ├── issue_123.rs      # Tier 4: specific regression guard
    └── issue_456.rs
```

Each benchmark file registers itself independently. Your `just bench` recipe supports:

```bash
# Run everything (pre-release)
just bench

# Run only Tier 1 (PR smoke test)
just bench -- "hot_paths"

# Run a specific regression guard
just bench -- "issue_123"
```

#### Cadence Rules

- **Smoke tier** (Tier 1 only): Runs on every PR. Takes \<2 minutes total.
- **Full suite** (Tier 1 + 2 + 4): Runs pre-release and nightly. Takes 5-15 minutes.
- **On demand** (any tier): Developer runs locally with `just bench -- <filter>`.

#### Adding Ad Hoc Benchmarks

When a developer wants to test something quickly:

1. Create a new benchmark file in `benches/` (or add to an existing file).
2. Run it locally with `cargo bench --bench <name>`.
3. If the benchmark proves valuable, promote it to the appropriate tier.
4. If it was a one-off investigation, delete it or leave it in `benches/scratch/` (excluded from CI runs).

The key principle: **all benchmarks exist in the same directory structure, but which ones run and how they're compared is controlled by your CI recipes and justfile targets, not by the benchmarks themselves.**

---

### 6. Minimizing Host Variability and Naming Hosts

#### Why Host Identity Matters

Benchmark numbers are meaningful only when compared against prior runs on the same (or equivalent) hardware. A "host identity" captures the factors that most influence benchmark results:

- **CPU architecture, model, and clock speed** (e.g., Apple M1 Pro vs AMD EPYC 7763)
- **RAM type and amount** (affects cache behavior, allocation patterns)
- **OS and kernel version** (affects scheduler, I/O subsystem)
- **Rust toolchain version** (different LLVM versions produce different optimizations)

#### Constructing a Host Identity

Build a host fingerprint from deterministic, easily-collected attributes:

```rust
use std::env::consts::{ARCH, OS};

fn host_identity() -> String {
    let cpu = read_sysctl_or_default("machdep.cpu.brand_string", "unknown-cpu");
    let cores = read_sysctl_or_default("hw.physicalcpu", "unknown-cores");
    let toolchain = rustc_version().unwrap_or_else(|_| "unknown".into());

    // Example output: "aarch64-macos-apple_m1_pro-8c-1.80.0"
    format!(
        "{}-{}-{}-{}c-{}",
        ARCH,
        OS,
        sanitize(&cpu),
        cores,
        toolchain
    )
}
```

This identity becomes part of every baseline filename and comparison:

```text
baselines/
├── aarch64-macos-apple_m1_pro-8c-1.80.0/
│   ├── hot_paths.json
│   └── integration.json
└── x86_64-linux-amd_epyc_7763-16c-1.80.0/
    ├── hot_paths.json
    └── integration.json
```

#### Minimizing Variability (Practical Checklist)

| Factor                     | Mitigation                                                                                 |
|----------------------------|--------------------------------------------------------------------------------------------|
| Background processes       | Close apps, disable indexing, turn off updates during benchmarking                         |
| CPU frequency scaling      | Set performance governor on Linux; plug in power on laptops                                |
| Thermal throttling         | Ensure adequate cooling; run warmup phase before measurement                               |
| Memory pressure            | Run with minimal other memory usage; pre-allocate, don't swap                              |
| Linker and build artifacts | Use `cargo bench` which builds in release mode; clean `target/` if results seem off        |
| Container/VM jitter        | Prefer bare-metal for authoritative baselines; accept noise for CI trend detection         |
| I/O variability            | Benchmark I/O-bound code with `O_DIRECT` or tmpfs; or separate I/O from compute benchmarks |

#### Automated Environment Capture

Create a `bench-env` script or `just` recipe that captures the host identity and current state:

```bash
#!/bin/bash
echo "host: $(host_identity)"
echo "kernel: $(uname -r)"
echo "load: $(uptime)"
echo "memory: $(vm_stat | head -5)"  # macOS
echo "toolchain: $(rustc --version)"
echo "profile: release"
```

Store this alongside every baseline so you can audit whether a comparison is valid (same host identity) or suspicious (different host or high load).

---

### Criterion vs Divan: How the Choice Affects Your Approach

Both Criterion and Divan serve the same fundamental purpose (statistically-informed microbenchmarking) but differ in API style, feature set, and CI suitability. The strategy layers (policy, infrastructure) remain the same regardless of choice; the differences show up in the implementation layer and CI workflow.

#### Comparison Matrix

| Aspect                 | Criterion (v0.8)                                                                  | Divan (v0.1.21)                                                           |
|------------------------|-----------------------------------------------------------------------------------|---------------------------------------------------------------------------|
| Registration model     | Explicit `criterion_group!` / `criterion_main!` macros                            | `#[divan::bench]` attribute, auto-discovered                              |
| Baseline comparison    | Built-in: `--save-baseline`, `--baseline` flags                                   | Not yet implemented (as of v0.1.21)                                       |
| Statistical rigor      | Bootstrap resampling (100k samples), outlier classification, confidence intervals | Sample-size scaling based on timer precision; median/mean/fastest/slowest |
| HTML reports           | Yes, with interactive plots                                                       | Not yet                                                                   |
| Allocation profiling   | No (external tools needed)                                                        | Built-in `AllocProfiler`                                                  |
| Thread contention      | Manual setup                                                                      | Built-in `threads` option                                                 |
| Generic benchmarks     | Manual parameterization                                                           | Built-in `types` and `consts` attributes                                  |
| CI suitability         | Author warns against CI due to noise                                              | Designed with CI in mind (sample scaling)                                 |
| Async support          | Built-in via `tokio`/`smol`/`futures` features                                    | Not yet                                                                   |
| Throughput measurement | Built-in `Throughput` enum                                                        | Built-in `BytesCount`, `CharsCount`, `ItemsCount`                         |
| Maturity               | Production-grade, widely adopted                                                  | Newer, rapidly evolving, API may change                                   |
| Compile time           | Heavier (more dependencies)                                                       | Lighter                                                                   |

#### Key Architectural Difference

**Criterion** stores previous run data on disk (`target/criterion/`) and performs statistical comparison against it automatically. This makes it excellent for local iterative development where you can see "Performance has improved" or "Performance has regressed" in the output without any extra tooling.

**Divan** uses a simpler, faster approach with no persistent state. It scales sample sizes dynamically based on timer precision (inspired by the "Robust Benchmarking in Noisy Environments" paper). This makes individual runs more reproducible in noisy environments, but you need external tooling for baseline comparison.

#### Code Examples

##### Basic Benchmark

**Criterion:**

```rust
// benches/hashing.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use biscuit_hash::xxhash;

pub fn bench_xxhash(c: &mut Criterion) {
    let data = vec![0u8; 1024];
    c.bench_function("xxhash_1kb", |b| {
        b.iter(|| xxhash(black_box(&data)))
    });
}

criterion_group!(benches, bench_xxhash);
criterion_main!(benches);
```

**Divan:**

```rust
// benches/hashing.rs
fn main() {
    divan::main();
}

#[divan::bench]
fn xxhash_1kb() -> u64 {
    let data = vec![0u8; 1024];
    biscuit_hash::xxhash(divan::black_box(&data))
}
```

##### Parameterized Benchmarks (Multiple Input Sizes)

**Criterion:**

```rust
pub fn bench_xxhash_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("xxhash");
    for size in [64, 256, 1024, 4096, 65536] {
        group.bench_with_input(
            BenchmarkId::new("size", size),
            size,
            |b, &size| {
                let data = vec![0u8; size];
                b.iter(|| xxhash(black_box(&data)))
            },
        );
    }
    group.finish();
}
```

**Divan:**

```rust
#[divan::bench(args = [64, 256, 1024, 4096, 65536])]
fn xxhash(size: usize) -> u64 {
    let data = vec![0u8; size];
    biscuit_hash::xxhash(divan::black_box(&data))
}
```

##### Throughput Measurement

**Criterion:**

```rust
pub fn bench_parse_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse_toml");
    group.throughput(Throughput::Bytes(input.len() as u64));
    group.bench_function("small_doc", |b| {
        b.iter(|| parse_toml(black_box(&input)))
    });
    group.finish();
}
```

**Divan:**

```rust
use divan::counter::BytesCount;

#[divan::bench]
fn parse_toml() -> TomlDocument {
    let input = include_str!("../tests/fixtures/small.toml");
    divan::Bencher::new()
        .counter(BytesCount::of_str(input))
        .bench(|| parse_toml(divan::black_box(input)))
}
```

##### Allocation Profiling (Divan only)

```rust
use divan::AllocProfiler;

#[global_allocator]
static ALLOC: AllocProfiler = AllocProfiler::system();

#[divan::bench(types = [Vec<u8>, Vec<u8>])]
fn from_iter() -> Vec<u8> {
    (0..1024).collect()
}
```

Criterion has no equivalent built-in; you would use `dhat` or custom `GlobalAlloc` wrappers separately.

##### Baseline Comparison in CI

**Criterion** (built-in):

```bash
# On main branch: save baseline
cargo bench -- --save-baseline main

# On PR branch: compare
cargo bench -- --baseline main
```

**Divan** (manual, since baselines are not yet built-in):

```bash
# Record baseline to JSON (would need a wrapper script)
cargo bench --bench hashing 2>&1 | tee baselines/$(host_identity)/hashing.json

# Compare: parse both outputs and compute percentage change
# (This is a gap in Divan's current feature set)
```

#### Which to Choose?

- **Choose Criterion** if you need mature baseline comparison, HTML reports, async benchmarking, or are building for a production environment where stability and ecosystem maturity matter.
- **Choose Divan** if you prioritize developer ergonomics, allocation profiling, thread contention measurement, fast compile times, or CI-first benchmarking where sample-size scaling reduces noise.
- **Consider both** for a transition period: Divan for fast developer feedback during development, Criterion for authoritative pre-release regression checks.

The strategy layers (tiers, cadence, host identity) work identically regardless of which crate you choose. The only implementation difference is how you wire up baseline storage and comparison.

---

### References

- Criterion.rs documentation: <https://bheisler.github.io/criterion.rs/book/>
- Criterion.rs FAQ on CI usage: <https://bheisler.github.io/criterion.rs/book/faq.html>
- Divan repository and docs: <https://github.com/nvzqz/divan>
- Divan announcement post: <https://nikolaivazquez.com/blog/divan/>
- "Robust Benchmarking in Noisy Environments" paper: <https://arxiv.org/abs/1608.04295>
- Iai (instruction-count benchmarking for CI): <https://github.com/bheisler/iai>
