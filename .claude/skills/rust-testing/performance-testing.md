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

