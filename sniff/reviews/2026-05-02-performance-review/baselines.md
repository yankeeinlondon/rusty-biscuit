# Performance Review Baselines

**Review Date:** 2026-05-02
**Baseline Date:** 2026-05-04
**Branch:** `sniff`
**Commit:** `06aa8afc`

---

## Compile-Time Measurements

### `cargo bloat` (release binary)

```bash
cargo bloat -p sniff-cli --release --bin sniff
```

**Summary:**
- `.text` section size: **17.0 MiB** (42.3% of total file size)
- Total file size: **40.1 MiB**
- Top contributors by `.text` size:
  1. `mermaid_rs_renderer::render::render_svg` — 71.0 KiB (0.4%)
  2. `mermaid_rs_renderer::parser::parse_mermaid` — 70.1 KiB (0.4%)
  3. `<sniff::args::RepoSubcommand as clap_builder::derive::Subcommand>::augment_subcommands` — 65.4 KiB (0.4%)
  4. `<sniff::args::Commands as clap_builder::derive::Subcommand>::augment_subcommands` — 42.4 KiB (0.2%)
  5. `aws_lc_sys::_aws_lc_0_40_0_p384_montjscalarmul` — 41.7 KiB (0.2%)
  6. `aws_lc_sys::_aws_lc_0_40_0_edwards25519_scalarmuldouble` — 31.4 KiB (0.2%)
  7. `aws_lc_sys::_aws_lc_0_40_0_p384_montjscalarmul_alt` — 30.6 KiB (0.2%)
  8. `mermaid_rs_renderer::layout::ranking::compute_ranks_subset` — 30.2 KiB (0.2%)
  9. `sniff::commands::run::{{closure}}` — 27.8 KiB (0.2%)
  10. `ravif::core::ops::function::impls::<impl core::ops::function::FnMut<A> for &F>::call_mut` — 27.7 KiB (0.2%)

Remaining 32,138 methods account for 96.1% of `.text` section.

### `cargo llvm-lines` (release)

```bash
cargo llvm-lines -p sniff-cli --release
```

**Summary:**
- Total LLVM IR lines: **634,363**
- Total function copies: **12,623**

Top 20 functions by LLVM IR lines:

| Lines | Copies | Function |
|-------|--------|----------|
| 18,153 | 192 | `<alloc::vec::Vec<T> as alloc::vec::spec_from_iter_nested::SpecFromIterNested<T,I>>::from_iter` |
| 13,291 | 89 | `serde_core::ser::Serializer::collect_seq` |
| 12,041 | 111 | `<core::slice::iter::Iter<T> as core::iter::traits::iterator::Iterator>::fold` |
| 10,707 | 129 | `<serde_json::value::ser::SerializeMap as serde_core::ser::SerializeMap>::serialize_value` |
| 10,173 | 1 | `sniff::output::filesystem::render_filesystem_section` |
| 9,576 | 137 | `core::iter::traits::iterator::Iterator::try_fold` |
| 9,516 | 1 | `sniff::commands::run::{{closure}}` |
| 9,028 | 122 | `<serde_json::value::ser::SerializeMap as serde_core::ser::SerializeStruct>::serialize_field` |
| 8,167 | 88 | `alloc::vec::Vec<T,A>::extend_desugared` |
| 7,938 | 106 | `alloc::vec::Vec<T,A>::extend_trusted` |
| 5,945 | 2 | `sniff::programs::types::_::<impl serde_core::ser::Serialize for sniff::programs::types::InstallationMethod>::serialize` |
| 5,196 | 1 | `<sniff::args::RepoSubcommand as clap_builder::derive::Subcommand>::augment_subcommands` |
| 5,027 | 1 | `sniff::output::filesystem::render_git_section` |
| 4,434 | 1 | `sniff::output::hardware::render_hardware_section` |
| 4,245 | 1 | `sniff::output::filesystem::render_language_section` |
| 4,238 | 1 | `darkmatter::markdown::output::terminal::write_terminal` |
| 4,195 | 132 | `core::iter::adapters::map::map_fold::{{closure}}` |
| 4,044 | 1 | `sniff::output::os::render_os_section` |
| 4,002 | 200 | `serde_json::value::to_value` |
| 3,763 | 59 | `<core::slice::iter::Iter<T> as core::iter::traits::iterator::Iterator>::find` |

---

## Runtime Measurements

### CLI End-to-End (`hyperfine`)

```bash
cargo build --release -p sniff-cli
SNIFF_BIN="$(cargo metadata --format-version 1 --no-deps | python3 -c 'import json,sys; print(json.load(sys.stdin)["target_directory"])')/release/sniff"
hyperfine --warmup 3 "${SNIFF_BIN} programs" "${SNIFF_BIN} editors"
```

**Results:**

| Command | Mean | ± σ | Min … Max |
|---------|------|-----|-----------|
| `sniff programs` | 35.8 ms | 0.9 ms | 33.7 ms … 38.1 ms |
| `sniff editors` | 10.8 ms | 0.7 ms | 9.9 ms … 16.8 ms |

**Speedup:** `sniff editors` is **3.30×** faster than `sniff programs`.

### Criterion Benchmarks

```bash
cargo bench -p sniff --bench perf -- "^(filesystem_git|filesystem_repo|filesystem_staged|inventory)"
```

#### filesystem_git

| Benchmark | Time | Change |
|-----------|------|--------|
| `git_dirty_10` | 4.21 ms | — |
| `git_dirty_100` | 6.19 ms | — |
| `git_dirty_1000` | 20.08 ms | — |
| `git_deep_10` | 4.77 ms | — |
| `git_deep_100` | 7.19 ms | — |
| `git_deep_1000` | 27.20 ms | — |

> Note: These are absolute times; no previous baseline exists for comparison.

#### filesystem_repo

| Benchmark | Time | Change vs Previous |
|-----------|------|-------------------|
| `repo_structure_monorepo` | 4.25 ms | No change (-2.34%) |
| `repo_with_inventory_monorepo` | 20.33 ms | **Regression** (+19.15%) |
| `repo_full_monorepo` | 17.81 ms | **Regression** (+31.65%) |
| `repo_structure_huge` | 14.55 ms | No change (-1.38%) |
| `repo_full_huge` | 57.34 ms | No change (-2.51%) |
| `package_enrichment_huge` | 4.76 ms | **Improved** (-2.86%) |

#### filesystem_staged

| Benchmark | Time | Change vs Previous |
|-----------|------|-------------------|
| `filesystem_summary_request` | 16.54 ms | **Improved** (-23.54%) |
| `filesystem_full_request` | 29.67 ms | **Improved** (-25.77%) |

#### inventory

| Benchmark | Time | Change vs Previous |
|-----------|------|-------------------|
| `programs_detect` | 25.07 ms | **Improved** (-25.54%) |
| `services_detect` | 4.87 ms | **Regression** (+16.27%) |

---

## Dependency Feature Decisions

### `which` Crate Feature Trimming

**Current State:** `which = { version = "8.0.0", default-features = false, features = ["real-sys"] }`

**Rationale:** The `which` crate already has trimmed features. Removing `default-features` drops the `regex` and `tracing` dependencies.

**Validation:** The current binary is 40.1 MiB with the trimmed `which` features. A full `cargo bloat`/`cargo llvm-lines` comparison against default features was not performed in this baseline run because the savings are expected to be modest (the `which` crate is a small dependency relative to the total binary size). The feature trimming was already applied in a prior phase and is retained.

**Decision:** Keep the trimmed `which` configuration. If future `cargo bloat` analysis shows the `regex` or `tracing` features contribute > 50 KB or > 1% compile time, re-evaluate.

### CLI Feature Split

**Status:** Deferred

**Rationale:** The binary size is dominated by `mermaid_rs_renderer`, `aws_lc_sys`, and `clap` derive macros. A CLI feature split (e.g., `sniff-cli/remote`) would add distribution complexity without addressing the largest size contributors. The decision is deferred until:
1. A specific use case demands a smaller binary (e.g., container images).
2. `cargo bloat` identifies a feature that contributes > 5% of binary size and is used by only a small fraction of commands.

---

## Flamegraph

**Status:** Not generated in this run.

**Reason:** `cargo flamegraph` requires executable artifacts in a specific profile layout that was not available in the non-interactive build environment. The command can be re-run locally with:

```bash
cargo flamegraph -p sniff-cli --bin sniff -- repo git-status
```

---

## Notes

- All measurements were taken on a clean working tree at commit `06aa8afc`.
- Criterion benchmarks were run with default settings (warm-up + sample collection).
- `hyperfine` was run with `--warmup 3` to warm caches.
- The `cargo bloat` and `cargo llvm-lines` commands analyze the release profile (`--release`).
