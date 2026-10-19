# Performance Testing in Rust

Part of the `rust-testing` skill; [SKILL.md](SKILL.md) is the index.

- **Tool: Criterion 0.5.** Every benchmarked package uses it as a
  dev-dependency, with benches in the package's `benches/` directory
  (e.g. `darkmatter/lib/benches/`, `sniff/lib/benches/perf.rs`). Add new
  benches the same way rather than introducing another harness.
- **Run through the area recipe.** `just bench` usually delegates to `_bench`
  in `just/devops.just`, which runs a storage and host preflight that warns about
  battery power, low memory, and high load before timing anything.
- **Before/after comparisons use host-named baselines.** Run
  `just bench-save` before the change and `just bench-compare` after it, where
  the area defines them (for example `worktree/justfile`, backed by
  `_bench_save` / `_bench_compare`). Baselines are named after the machine, so
  a comparison never crosses hosts.
- **Opting out.** A package without benches declares
  `[package.metadata.benchmarks] required = false` and makes its `bench`
  recipe a documented no-op (see `biscuit-location/justfile`).
- **A bench measures latency; it is not a contract test.** Prove contracts
  such as "one request per operation" with work counters in an L1 test
  ([test-design.md](test-design.md)).
