# When to use nextest (and when not to)

## Use nextest when

1. **Large test suites / workspaces**
   
   * Many crates and multiple integration test binaries.
   * Nextest runs tests across binaries concurrently (often significantly faster than `cargo test`).
1. **CI/CD pipelines**
   
   * Need **JUnit XML** output.
   * Need test **sharding/partitioning** across multiple CI workers.
   * Want consistent, profile-based configuration.
1. **Flaky tests**
   
   * Built-in retries with backoff strategies and better flake visibility.
1. **Isolation matters**
   
   * Tests touch env vars, filesystem, global state, ports, or crash occasionally.
   * Process-per-test reduces cross-test interference.
1. **Slow/hanging tests**
   
   * Identify slow tests, set per-test/per-package timeouts, terminate hangs.

## Don’t use (or supplement) nextest when

1. **You depend on doctests**
   
   * Nextest does not support Rust doctests (stable limitation).
   * Use a two-step approach in CI:
     * `cargo nextest run`
     * `cargo test --doc`
1. **Very small projects / micro test suites**
   
   * Process-per-test overhead may not pay off.
1. **Extremely old Rust toolchains**
   
   * Running tests with nextest works on old Rust (project MSRV can be low),
   * but **building/installing nextest** requires modern Rust (per docs, 1.89+ to build).
1. **Tools that require `cargo test` semantics/output**
   
   * Some pipelines are hardwired to `cargo test`; validate tool compatibility.

---