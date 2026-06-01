# Performance Testing in Rust

## Tooling

The two main crates used in the Rust community for performance testing are:

1. Criterion
2. Divan

Below we provide a list of pros and cons for each but both are excellent choices for performance testing in Rust.

### Criterion

#### Pros

- Statistical Rigor: It uses sophisticated statistics (including bootstrapping and the Mann-Whitney U test) to determine if a performance change is a genuine regression/improvement or just system noise.
- Rich Visual Reports: Automatically generates highly detailed HTML reports complete with plots, probability density functions, and linear regression charts.
- Ecosystem Maturity: Because it has been the standard for years, it has immense community support, extensive documentation, and native integrations with various CI systems and tools.
- Stable Rust Support: Works completely on stable Rust using standard external benchmark harnesses.

#### Cons

- Boilerplate Heavy: Setting up benchmarks requires boilerplate code (creating custom functions, setting up a Criterion struct instance, and configuring the criterion_group! and criterion_main! macros).
- Slower Compile & Execution Times: The heavy statistical calculations and graph-generation code mean your benchmark suite takes noticeably longer to compile and run.
- Clunky Parameterization: Benchmarking across multiple inputs, sizes, or types requires writing nested loops or manually constructing matrix inputs.

#### Use Criterion if:

- You need absolute statistical confidence: If you are building a core database engine, a cryptography crate, or a foundational math library where a $1\%$ variance matters, Criterion's math ensures you aren't chasing ghosts.
- You want beautiful visual progression tracking: If your team relies on hosting HTML performance reports on a dashboard or documentation page over time.
- You are building an established open-source crate: The ecosystem expects Criterion, making it easier for external contributors to run and understand regressions using familiar tools.

### Divan

#### Pros
- Zero Boilerplate API: You can turn almost any function into a benchmark simply by adding #[divan::bench] right above it—very similar to how Rust's native #[test] attribute works.
- Powerful Parameterization: It shines at multi-case testing. You can easily benchmark a function across various inputs (args = [...]), const generics (consts = [...]), and even different types (types = [Vec<i32>, BTreeSet<i32>]) using simple macro attributes.
- Allocation Profiling: Divan includes a built-in AllocProfiler capable of telling you exactly how many heap allocations occurred and how many bytes were allocated during the benchmark.
- Native Thread Contention Testing: Built-in support to run benchmarks across multiple threads to measure lock or atomic contention effortlessly.
- Blazing Fast: The library itself is lightweight, compiles quickly, and prints highly readable, compact terminal grids.

#### Cons

- No HTML Charts: It prioritizes terminal-based outputs. If you want rich, visual charts out-of-the-box for non-technical stakeholders, Divan doesn't provide them natively.
- Simpler Statistics: While highly accurate, its default output relies on standard metrics (fastest, slowest, median, mean) rather than Criterion's deep regression-analysis mathematics.
- Under-the-Hood Complexity: It relies on advanced linker trickery to achieve its global macro registration, which can occasionally encounter edge cases on niche or custom hardware architectures (though it fully supports major platforms like Windows, macOS, and Linux).

#### Use Divan if:

- You want rapid feedback loops: If you want to write a benchmark in 5 seconds without leaving your current source file, run it quickly, optimize, and move on.
- You need to test multiple data structures or data sizes: If you are comparing a HashMap vs. a BTreeMap, or tracking how an algorithm scales across inputs of $10, 100, 1000,$ and $10000$ items.
- Tracking memory matters as much as time: If you need to catch sneaky, accidental heap allocations happening inside your hot paths.
- You are testing concurrent code: If you need to quickly benchmark how your code holds up under multi-threaded lock contention.
