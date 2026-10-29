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
- **Pick the measurement by the claim.**

  | Claim | Tool |
  | ----- | ---- |
  | "this does N units of work" (parses, compilations, reads, snapshots) | a work counter, asserted as a delta in an L1 test: `darkmatter::work_counters` / `claudine::work_counters` behind each package's test-only `work-counters` feature, `dmls::work_counters` |
  | "this allocates / copies less" | `test_toolkit::alloc_count::CountingAllocator`, declared as `#[global_allocator]` in a dedicated `*_alloc` bench target; report, never assert exact byte counts |
  | "this is faster" | Criterion, under the system allocator, compared against a saved baseline |

  New counters go through each crate's root `note_work!` macro, which
  compiles to nothing without the feature, so production builds carry no
  counter state.
- **Where the counting allocator may be declared.** Only a final binary can
  declare it: a dedicated bench target (`pipeline_work_alloc`, DMLS's
  `pipeline_work_a3`), or a crate's own unit-test binary when the scenario
  needs crate-private functions. Claudine's lib declares it under
  `cfg(all(test, feature = "work-counters"))` for its ignored
  `pipeline_spike_*` scenarios, run one at a time from `claudine/` with
  `just test-library --release --features work-counters --run-ignored only
  --no-capture pipeline_spike_<scenario>`. Because `work-counters` is on in
  that package's L1 run, every Claudine lib unit test then allocates through
  it; it changes no result, only adds an atomic per allocation.

