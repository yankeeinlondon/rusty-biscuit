Performance in Rust YAML parsing often involves a trade-off between **safety** (Pure Rust) and **raw speed** (C-bindings).

* **The "Trap":** Many developers assume the C-binding (`serde_yaml` / `serde_yaml_ng` which uses `libyaml`) is always faster. However, modern pure-Rust parsers like `serde-saphyr` have largely closed this gap and eliminate `unsafe` blocks, but they might consume more memory or compile slower. You won't know which is better for *your* specific file sizes without measuring.

Here is how to set up a benchmark to settle this for your specific use case.

### 1. The Tool Selection

We will use **`divan`**.
While `criterion` is the classic standard, `divan` is a newer, lighter framework created by a Rust contributor. It is significantly easier to set up (it runs as a standard test harness) and automatically calculates throughput (MB/s), which is the most important metric for parsing.

### 2. The Setup

Add the crates to your `Cargo.toml`. We will compare the standard (maintained fork) against the pure-Rust challenger.

```toml
[dependencies]
serde = { version = "1.0", features = ["derive"] }
serde_yaml_ng = "0.10"  # The "C-binding" baseline
serde-saphyr = "0.1"    # The "Pure Rust" challenger

[dev-dependencies]
divan = "0.1"

```

### 3. The Benchmark Code (`benches/yaml_bench.rs`)

Create a file at `benches/yaml_bench.rs`. We will benchmark parsing a "typical" config file.

```rust
use divan::Bencher;
use serde::Deserialize;

fn main() {
    // Run the benchmark
    divan::main();
}

// 1. Define the data structure we are parsing
// We use a mix of strings, ints, and arrays to simulate real configs.
#[derive(Deserialize)]
struct Config {
    name: String,
    version: u32,
    features: Vec<String>,
    #[serde(flatten)]
    extra: std::collections::HashMap<String, serde::de::IgnoredAny>,
}

// 2. Prepare the Input Data
// We use a constant string to ensure both parsers digest the exact same bytes.
const YAML_DATA: &str = r#"
name: "production-server-01"
version: 12
features:
  - "fast-parsing"
  - "security-hardened"
  - "zero-copy"
  - "auto-scaling"
metrics:
  cpu: "high"
  memory: "optimized"
"#;

// 3. Define the Benchmarks
#[divan::bench]
fn bench_serde_yaml_ng(bencher: Bencher) {
    bencher
        .with_inputs(|| YAML_DATA)
        .bench_values(|input| {
            // Measure the C-based parser
            serde_yaml_ng::from_str::<Config>(input).unwrap()
        });
}

#[divan::bench]
fn bench_serde_saphyr(bencher: Bencher) {
    bencher
        .with_inputs(|| YAML_DATA)
        .bench_values(|input| {
            // Measure the Pure Rust parser
            serde_saphyr::from_str::<Config>(input).unwrap()
        });
}

```

### 4. Running and Interpreting Results

Run the benchmark with:

```bash
cargo bench

```

**What to look for in the output:**

1. **Median Time:** The middle number is your reality. If `serde-saphyr` is within 10% of `serde_yaml_ng`, prefer `saphyr` for the memory safety guarantees.
2. **Outliers:** If one has a massive "max" time, it implies the parser might struggle with garbage collection or allocation spikes, which is bad for high-throughput web servers.

### 5. Advanced Benchmarking Tip: "The Heavyweight Test"

Benchmarks on small strings often lie. If you are parsing Kubernetes manifests or large datasets, small constant overheads disappear and **allocation** becomes the bottleneck.

To test this, modify the input in the benchmark to repeat the data 1,000 times:

```rust
// In your setup
let large_yaml = YAML_DATA.repeat(1000);

// In the bencher
bencher.with_inputs(|| large_yaml.as_str())...

```

**The likely result:**

* **`serde_yaml_ng`** (C-binding) usually wins on massive files because `libyaml` is extremely mature and optimized.
* **`serde-saphyr`** (Pure Rust) usually wins on small config files because it avoids the overhead of crossing the FFI (Foreign Function Interface) boundary between Rust and C.

### Final Summary of the Ecosystem

| Feature | `serde_yaml_ng` | `serde-saphyr` |
| --- | --- | --- |
| **Backend** | `libyaml` (C code) | `saphyr` (Pure Rust) |
| **Safety** | `unsafe` blocks required | Safe Rust |
| **Best For** | Legacy apps, Massive files (MBs) | New apps, Security-critical, Config files (KBs) |
| **Schema** | Use `jsonschema` crate | Use `jsonschema` crate |

This concludes the guide on the Rust YAML ecosystem. You now have the tools to choose the right crate, validate it with schemas, lock it down with golden tests, and verify its performance.

