# Benchmarking with Criterion

Criterion provides statistically rigorous benchmarking for Rust code on stable.

## Setup

```toml
# Cargo.toml
[dev-dependencies]
criterion = { version = "0.5", features = ["html_reports"] }

[[bench]]
name = "my_benchmark"
harness = false
```

## Basic Benchmark

```rust
// benches/my_benchmark.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn fibonacci(n: u64) -> u64 {
    match n {
        0 | 1 => 1,
        n => fibonacci(n - 1) + fibonacci(n - 2),
    }
}

fn fibonacci_benchmark(c: &mut Criterion) {
    c.bench_function("fib 20", |b| {
        b.iter(|| fibonacci(black_box(20)))
    });
}

criterion_group!(benches, fibonacci_benchmark);
criterion_main!(benches);
```

Run with:

```bash
cargo bench
```

## Key Concepts

### `black_box`

Prevents compiler from optimizing away the computation:

```rust
// Without black_box, compiler might optimize this away
b.iter(|| fibonacci(20));

// black_box ensures the computation actually runs
b.iter(|| fibonacci(black_box(20)));
```

### Benchmark Groups

Compare related implementations:

```rust
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};

fn compare_implementations(c: &mut Criterion) {
    let mut group = c.benchmark_group("sorting");

    for size in [100, 1000, 10000].iter() {
        let data: Vec<i32> = (0..*size).rev().collect();

        group.bench_with_input(
            BenchmarkId::new("std_sort", size),
            &data,
            |b, data| {
                b.iter(|| {
                    let mut d = data.clone();
                    d.sort();
                    d
                })
            },
        );

        group.bench_with_input(
            BenchmarkId::new("std_sort_unstable", size),
            &data,
            |b, data| {
                b.iter(|| {
                    let mut d = data.clone();
                    d.sort_unstable();
                    d
                })
            },
        );
    }

    group.finish();
}

criterion_group!(benches, compare_implementations);
criterion_main!(benches);
```

## Benchmark with Setup

Separate setup from measured code:

```rust
fn benchmark_with_setup(c: &mut Criterion) {
    c.bench_function("process large data", |b| {
        // Setup runs once per iteration group
        b.iter_batched(
            || generate_large_dataset(),  // Setup
            |data| process(black_box(data)),  // Measured
            criterion::BatchSize::SmallInput,
        )
    });
}
```

## Throughput Measurement

Measure bytes or elements per second:

```rust
use criterion::{criterion_group, criterion_main, Criterion, Throughput};

fn throughput_benchmark(c: &mut Criterion) {
    let data = vec![0u8; 1024 * 1024]; // 1 MB

    let mut group = c.benchmark_group("hashing");
    group.throughput(Throughput::Bytes(data.len() as u64));

    group.bench_function("sha256", |b| {
        b.iter(|| sha256(&data))
    });

    group.finish();
}

criterion_group!(benches, throughput_benchmark);
criterion_main!(benches);
```

## Configuration

```rust
use criterion::{criterion_group, criterion_main, Criterion};
use std::time::Duration;

fn custom_config() -> Criterion {
    Criterion::default()
        .sample_size(100)
        .measurement_time(Duration::from_secs(10))
        .warm_up_time(Duration::from_secs(3))
}

fn my_benchmark(c: &mut Criterion) {
    c.bench_function("my_function", |b| {
        b.iter(|| my_function())
    });
}

criterion_group! {
    name = benches;
    config = custom_config();
    targets = my_benchmark
}
criterion_main!(benches);
```

## Output Interpretation

```
sorting/std_sort/1000   time:   [45.123 us 45.456 us 45.789 us]
                        change: [-2.1234% -1.5678% -0.9876%] (p = 0.00 < 0.05)
                        Performance has improved.
```

- **time**: [lower bound, estimate, upper bound] with 95% confidence
- **change**: Comparison to previous run
- **p-value**: Statistical significance

## Best Practices

1. **Isolate the code under test** - Minimize setup in measured section
2. **Use realistic data** - Benchmark with production-like inputs
3. **Run multiple times** - Criterion handles this automatically
4. **Check for variance** - High variance indicates measurement issues
5. **Disable CPU frequency scaling** for consistent results
6. **Build in release mode** - `cargo bench` does this by default

## Alternative: Divan

Simpler syntax for comparative benchmarks:

```toml
[dev-dependencies]
divan = "0.1"

[[bench]]
name = "divan_bench"
harness = false
```

```rust
fn main() {
    divan::main();
}

#[divan::bench]
fn fibonacci_divan() -> u64 {
    fibonacci(20)
}

#[divan::bench(args = [10, 20, 30])]
fn fibonacci_parametric(n: u64) -> u64 {
    fibonacci(n)
}
```

## Profiling After Benchmarking

Once benchmarks identify slow code:

```bash
# Build with debug symbols
RUSTFLAGS="-C debuginfo=2" cargo build --release

# Profile on Linux
perf record -g target/release/my_binary
perf report

# Generate flamegraph
cargo install flamegraph
cargo flamegraph
```

## Related

- [Unit Tests](./unit-tests.md) - Correctness testing
- [Property Testing](./property-testing.md) - Testing with many inputs
- [nextest](./nextest.md) - Faster test execution
