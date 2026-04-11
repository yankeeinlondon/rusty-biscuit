# Review: Performance Testing for Sniff

Date: 2026-04-11
Scope: implemented work under `sniff/` for the 2026-04-11 performance-testing feature

## Findings

### 1. High: the new CI workflow is currently broken because the Criterion invocation passes multiple positional filters

File: `.github/workflows/sniff-performance.yml:47-57`

The workflow calls:

```bash
cargo bench -p sniff --bench perf -- \
  --save-baseline ci \
  "system/detect_summary" \
  "system_full/detect_full" \
  "filesystem_git/git_summary_monorepo" \
  "filesystem_repo/repo_structure_monorepo" \
  "inventory/programs_detect"
```

Criterion accepts a single optional benchmark filter expression, not multiple positional filters. Reproducing the workflow command locally fails immediately with exit status 2:

```bash
cargo bench -p sniff --bench perf -- --save-baseline ci "system/detect_summary" "system_full/detect_full"
```

Result:

```text
error: unexpected argument found
```

So the new `sniff-performance` workflow will fail before running any benchmark. This needs to be changed to either:

- one regex filter string, or
- multiple separate `cargo bench` invocations, one per target benchmark.

### 2. Medium: network performance coverage from the design is still missing, and the “full” system benchmark explicitly avoids the full network path

Files:
- `sniff/lib/benches/perf.rs:15-27`
- `sniff/lib/benches/support/plans.rs:47-56`

The suite registers `system`, `hardware`, `filesystem`, and `inventory` groups, but no `network` group at all. On top of that, `full_plan()` forces `NetworkRequest::interfaces_only()` instead of exercising the default full network request.

That leaves two gaps:

- there is no direct performance signal for `detect_network_with_request`, WAN IP lookup, or cache behavior
- the headline `detect_full` benchmark understates the real full-plan cost because one entire expensive path is intentionally removed

Given the design explicitly called out WAN lookup variance and a `wiremock`-backed network fixture, this is a real functionality gap rather than a documentation choice.

### 3. Medium: the filesystem flamegraph path is not reproducible because it profiles the caller’s current directory instead of the synthetic large fixture

Files:
- `sniff/lib/examples/profile_filesystem.rs:19-35`
- `sniff/justfile:225-245`

The Criterion benches use deterministic synthetic fixtures, but the profiling example takes `argv[1]` or falls back to `current_dir()`. `just profile profile_filesystem` does not supply a fixture path, so the produced flamegraph depends on wherever the command is launched from.

In practice this means the profiling path is no longer aligned with the benchmarked workload:

- benches profile a generated large monorepo fixture
- the flamegraph example usually profiles the `sniff/` package directory or repo root

That makes hotspot comparisons harder and can hide exactly the shared-walk and monorepo-scan costs the feature was intended to study.

### 4. Medium: the new benchmark/profiling infrastructure has almost no direct test coverage

Files:
- `sniff/lib/benches/support/fixtures.rs:38-210`
- `sniff/lib/benches/support/plans.rs:17-57`
- `sniff/justfile:187-245`
- `.github/workflows/sniff-performance.yml:47-66`

The new functionality is largely benchmark fixtures and automation, but there are no unit or integration tests covering it:

- no tests assert fixture shape, commit counts, dirty state, or package counts
- no tests validate that benchmark plan builders still mean what their names claim
- no smoke coverage exists for the profiling examples
- no automated validation exists for the `just` / workflow command lines

This is not theoretical; it is how finding 1 slipped through. The library test suite is strong overall, but the newly added perf infrastructure itself is essentially untested.

## Gaps vs Design / Plan

- The design called for network-related benchmarks and a `wiremock` strategy for WAN-related work. That is not implemented.
- The design called for profiling against representative heavy paths; the current filesystem profiling example does not use the deterministic heavy fixture.
- The plan called out `sniff/just.md`, but the implementation work is functionally concentrated in `sniff/justfile`; the markdown help file was not materially updated to document the new recipes.

## Validation Performed

- `cargo test -p sniff`
- `cargo test -p sniff-cli`
- `cargo bench -p sniff --bench perf --no-run`
- reproduced the workflow command failure locally with the same Criterion argument pattern used in `.github/workflows/sniff-performance.yml`

## Ergonomics / Performance Suggestions

- Use a single source of truth for benchmark IDs. Right now the workflow hardcodes names such as `system_full/detect_full`; a small helper script or shared constants would reduce drift between benches, docs, and CI.
- Add `sniff/lib/benches/cases/network.rs` so the suite covers interface-only detection, WAN lookup behind `wiremock`, and cache-hit vs forced-refresh comparisons.
- Split hardware benches into two layers: direct leaf benches for `detect_audio_devices`, `detect_gpus`, and `detect_storage`, plus request-level benches for `detect_hardware_with_request`. That would make bottlenecks easier to attribute and remove CPU/memory setup cost from the leaf measurements.
- Add small tests for fixture invariants. Even a few assertions around commit count, dirty files, package counts, and workspace manifests would make the benchmark corpus much safer to maintain.
