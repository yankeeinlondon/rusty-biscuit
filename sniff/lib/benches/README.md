# sniff Criterion Benchmark Manifest

Each benchmark ID is designed to be self-describing. This file maps every
ID to a human-readable explanation of what is being measured and why it
matters.

## Conventions

| Suffix / Prefix | Meaning |
|-----------------|---------|
| `_small_repo` | Synthetic git repo with ~5 commits |
| `_monorepo` | Synthetic monorepo with ~50 packages |
| `_huge` | Synthetic monorepo with 375 packages (200 Rust, 100 JS, 50 Python, 25 Go) |
| `_only` | A single subsystem is exercised in isolation |
| `_isolated` | Request-level bench with all other flags disabled |
| `_lazy` / `_eager` | Which `ExecutableIndex` build strategy is used |
| Number suffix (e.g., `25_names`, `200_files`) | Data volume for throughput context |

---

## system

| Benchmark | What it measures |
|-----------|------------------|
| `minimal_plan_os_hardware_only` | End-to-end `detect_with_plan` with only OS summary + hardware summary enabled. Baseline for the cheapest useful detection tier. |
| `summary_plan_with_git_repo` | End-to-end `detect_with_plan` with OS, hardware, and a small git repo. Measures the staged-filesystem orchestration cost. |
| `full_plan_monorepo_all_domains` | End-to-end `detect_with_plan` with every domain enabled (OS, hardware, network, filesystem). The worst-case single-call path. |

## hardware_leaf

| Benchmark | What it measures |
|-----------|------------------|
| `gpu_enumeration` | Raw platform GPU detection latency (IOKit on macOS, WMI on Windows, `/sys` + `lspci` parsing on Linux). No CPU or memory work. |
| `storage_enumeration` | Raw storage device detection latency. Platform-specific enumeration without request orchestration. |

## hardware_leaf_audio

| Benchmark | What it measures |
|-----------|------------------|
| `audio_device_enumeration` | Raw audio input/output device detection. Can exceed 1s on macOS Core Audio and some Linux ALSA setups, hence the slow-group config. |

## hardware

| Benchmark | What it measures |
|-----------|------------------|
| `simd_feature_detection` | CPU SIMD capability scan (AVX, AVX2, NEON, etc.). Very fast; exercises the `std::arch` feature-gate path. |
| `hardware_summary_cpu_memory` | `detect_hardware_summary()` — CPU info + memory only. The most common hardware tier. |
| `hardware_full_all_subsystems` | `detect_hardware_with_request(HardwareRequest::full())` — CPU, memory, storage, GPU, audio. Full orchestration overhead. |
| `hardware_request_storage_isolated` | Request-level storage-only path. Isolates the storage-enrichment cost inside the request framework. |
| `hardware_request_gpu_isolated` | Request-level GPU-only path. Isolates GPU-enrichment cost. |
| `hardware_request_audio_isolated` | Request-level audio-only path. On non-macOS/non-Linux this is effectively a no-op stub. |

## filesystem_git

| Benchmark | What it measures |
|-----------|------------------|
| `git_summary_small_repo_5_commits` | Git summary on a small repo (branch name + dirty counts only, no commit history). Fastest git path. |
| `git_full_small_repo_10_commits_stats` | Git full on a small repo — 10 commits plus per-file change stats. Measures diff-aggregation cost on a trivial repo. |
| `git_summary_monorepo_5_commits` | Git summary on a large monorepo. Verifies that summary mode scales with repo size (it should be O(1)). |
| `git_full_monorepo_10_commits_stats` | Git full on a large monorepo. Measures commit-walk + stat aggregation cost at scale. |

## filesystem_repo

| Benchmark | What it measures |
|-----------|------------------|
| `repo_structure_monorepo_manifest_discovery` | `detect_repo_structure()` on a large monorepo. Manifest discovery (Cargo.toml, package.json, etc.) without per-package language scanning. |
| `repo_with_shared_inventory_monorepo` | `detect_repo_with_inventory()` — structure + file inventory in one pass. Measures the shared-work path that avoids re-walking the tree. |
| `repo_full_monorepo_language_scan` | `detect_repo()` — structure + per-package language detection + framework heuristics + dependency parsing. The dominant cost of `RepoRequest::full()`. |
| `repo_structure_huge_375_packages` | Structure-only on the 375-package monorepo fixture (200 Rust, 100 JS, 50 Python, 25 Go). Stresses manifest caching and index normalization. |
| `repo_full_huge_375_packages` | Full repo detection on the 375-package fixture. Worst-case language-scan cost. |
| `package_boundary_refresh_huge` | Isolated `refresh_package_boundaries()` on the 375-package fixture with a pre-built inventory. Measures only the boundary-assignment logic. |

## filesystem_inventory

| Benchmark | What it measures |
|-----------|------------------|
| `file_inventory_walk_small_repo` | `scan_file_inventory()` on a small repo. Baseline for the parallel `ignore`-based walker. |
| `file_inventory_walk_monorepo` | `scan_file_inventory()` on a large monorepo. Measures walker parallelism and ignore-rule evaluation at scale. |
| `file_inventory_walk_mixed_langs` | `scan_file_inventory()` on a tree with files at mixed depths. Stresses directory-depth heuristics. |

## filesystem_languages

| Benchmark | What it measures |
|-----------|------------------|
| `language_detection_mixed_depths` | `detect_languages()` on a tree with shallow and deep source files. Measures `hyperpolyglot` classification across directory depths. |
| `language_detection_monorepo` | `detect_languages()` on a large monorepo. Measures aggregate classification cost for many files. |

## filesystem_docs

| Benchmark | What it measures |
|-----------|------------------|
| `docs_parse_full_frontmatter_200_files` | `detect_docs()` on 200 markdown files. Full frontmatter parse + hash + path resolution. Throughput: 200 elements. |
| `docs_parse_blast_radius_only_40_of_200` | `detect_blast_radius_docs()` on the same 200 files. Only 40 have `blast_radius` frontmatter; the rest short-circuit. Throughput: 200 elements. |

## filesystem_staged

| Benchmark | What it measures |
|-----------|------------------|
| `staged_filesystem_summary_git_plus_repo` | `detect_filesystem_with_request()` with git summary + repo structure only. Measures staged orchestration with two stages enabled. |
| `staged_filesystem_full_all_stages` | `detect_filesystem_with_request()` with all stages (git, repo, inventory, formatting, docs). Worst-case staged path. |

## git_dirty_scaling

Parameterised by dirty-file count (10, 100, optionally 1000).

| Benchmark | What it measures |
|-----------|------------------|
| `git_full_with_file_stats/<count>` | `GitRequest::full()` on a repo with `<count>` dirty working-tree files. Measures per-file stat aggregation cost as dirty count grows. Throughput: `<count>` elements. |
| `git_deep_with_unified_diffs/<count>` | `GitRequest::deep()` on the same repo. Adds full unified diff emission — the worst-case path through the batched diff aggregator. Throughput: `<count>` elements. |

## git_deep_remote

Parameterised by remote-tracking branch count (1, 5, 10, 25).

| Benchmark | What it measures |
|-----------|------------------|
| `git_deep_remote_containment_check/<count>` | `GitRequest::deep()` with 10 commits on a repo with `<count>` fake remote-tracking branches. Measures the ancestry-walk optimisation in `populate_recent_commit_remotes`. Throughput: `<count>` elements. |

## inventory

| Benchmark | What it measures |
|-----------|------------------|
| `programs_detect_all_8_categories` | `ProgramsInfo::detect()` — full 8-category fan-out using Rayon + shared `ExecutableIndex`. End-to-end parallelism test. |
| `services_detect_init_system` | `detect_services()` — init-system detection + service listing. Platform-dependent cost (launchctl, systemctl, etc.). |

## workloads_service_listing

Requires `--features bench-internals`. Fixture strings and per-iteration runner
state are prepared outside Criterion's timed section. The measured call uses the
same systemd listing parser, running-service selection, 128-unit chunk builder,
runner dispatch, show-block parser, and PID projection as production.

| Benchmark | What it measures |
|-----------|------------------|
| `500` | Deterministic listing and enrichment of 500 running services. One listing plus four enrichment chunks. Throughput: 500 elements. |
| `2000` | Deterministic listing and enrichment of 2,000 running services. One listing plus 16 enrichment chunks. Throughput: 2,000 elements. |

## executable_index

| Benchmark | What it measures |
|-----------|------------------|
| `executable_index_build_lazy` | `ExecutableIndex::build()` — lazy path (no PATH scan, `which` fallback on each lookup). |
| `executable_index_lookup_5_names` | 5 `find_with_source()` calls against a pre-built index. Micro-benchmark of the hot lookup path. |

## programs

| Benchmark | What it measures |
|-----------|------------------|
| `executable_index_build_lazy` | Same as `executable_index_build_lazy` in the `programs` group — re-registered for locality. |
| `executable_index_build_eager_path_scan` | `ExecutableIndex::build_eager_path()` — eager path (full PATH scan upfront, O(1) lookups after). |

## programs_bulk_lookup

| Benchmark | What it measures |
|-----------|------------------|
| `bulk_lookup_25_names_lazy_index` | 25 program names resolved against a lazy index. Mix of hits and misses. Throughput: 25 elements. |
| `bulk_lookup_25_names_eager_index` | Same 25 names against an eager index. All hits (fast path). Throughput: 25 elements. |
| `bulk_lookup_150_names_lazy_index` | 150 program names against a lazy index. Simulates a long PATH. Throughput: 150 elements. |
| `bulk_lookup_150_names_eager_index` | Same 150 names against an eager index. Throughput: 150 elements. |

## programs_fanout

| Benchmark | What it measures |
|-----------|------------------|
| `programs_detect_all_8_categories_fanout` | `ProgramsInfo::detect()` — same as `inventory::programs_detect_all_8_categories`, re-registered here for program-domain locality. |

## network

Requires `--features network` for meaningful WAN IP results; otherwise WAN benches measure only orchestration cost.

| Benchmark | What it measures |
|-----------|------------------|
| `network_interfaces_enumeration_only` | `NetworkRequest::interfaces_only()` — local interface enumeration. No HTTP. |
| `wan_ip_http_roundtrip_no_cache` | `NetworkRequest::full().force_refresh(true)` — forces a real HTTP round-trip to the wiremock fixture on every iteration. Measures raw HTTP + JSON parse latency. |
| `wan_ip_cache_hit_plus_interfaces` | `NetworkRequest::full()` with a warm TTL cache. Measures interface enumeration + cache-hit bookkeeping only. |

## repo_package_boundaries

Parameterised by package count (10, 100, optionally 500).

| Benchmark | What it measures |
|-----------|------------------|
| `package_boundary_refresh/<count>` | `refresh_package_boundaries()` on a Cargo workspace with `<count>` packages. Structure and inventory are prepared outside the timed loop. Measures only boundary-assignment logic. Throughput: `<count>` elements. |

## specification workload matrix

The `workloads_*` groups are the production-shaped families required by the
2026-07-16 performance specification. Their fixtures are constructed lazily by
the selected benchmark and outside its timed `b.iter` loop.

| Specification family | Benchmark definition | Deterministic work bound |
|----------------------|----------------------|--------------------------|
| Formatting-only, deep/wide | `workloads_filesystem/formatting_only_deep_24_wide_32` | `formatting_only_request_starts_no_walker` pins zero walker starts and entries. |
| Package-scoped inventory + Git in a large monorepo | `workloads_filesystem/package_scoped_git_inventory_in_500_package_monorepo` | `walk_scope_table` pins Git + inventory to `WalkScope::Package`; Git is not a repository-wide walk consumer. |
| Standalone versus integrated observation; nested discovery with/without supplied evidence | `workloads_repo_observation/{standalone_detect_repo_nested_500,integrated_full_supplied_observation_nested_500}` | `standalone_full_detection_enumerates_the_tree_once`, `integrated_and_standalone_full_detection_agree`, and `nested_workspaces_are_discovered_from_observed_markers` pin one observation and output parity. |
| Mixed-ecosystem structure-only scaling | `workloads_repo_structure_mixed/{100,500,2000}` | Structure-mode tests pin zero package enrichments and inventory acceptance. |
| Inventory-only and inventory+docs above the cap | `workloads_inventory_over_cap/{inventory_only_10500_files,inventory_and_docs_10500_files_2000_docs}` | `inventory_only_walk_stops_at_the_cap_and_reports_truncation` and `combined_walk_keeps_going_past_saturation_for_its_other_observers` pin the 10,000 accepted-file cap and observer behavior. |
| Final assembly and Markdown package-prefix assignment | `workloads_final_assembly/full_500_packages_2000_docs`, `workloads_document_attribution/package_prefix_assignment/{500,2000}` | Ownership/document tests pin deepest component-prefix selection and zero lookup canonicalizations. |
| Dirty Git payload sizes | `workloads_git_dirty_sizes/100_files/{1024,102400,2097152}` | `each_dirty_side_loads_and_diffs_once` pins one blob load and diff per dirty side independent of payload size. |
| Branch-heavy/divergent and many-tip containment | `workloads_git_branches/{branch_heavy_divergent_32_tips,deep_containment_100_remote_tips}` | `focused_ref_consumers_share_one_observation` pins one ref snapshot; containment shares the `git.commit_visits` bound used by the path-history and ref-walk tests. |
| Sparse path history | `workloads_git_path_history/2000_commits_sparse_prefix_every_200` | `commit_visits_are_bounded_by_the_scan_limit` pins visits at the explicit scan bound. |
| Remote provider request counts | `workloads_remote_report/github_provider_request_count_fixture` (requires `remote`) | `github_fetch_report_resolves_metadata_and_tree_once` pins one metadata and one root-tree request. |
| Large synthetic service listing | `workloads_service_listing/{500,2000}` (requires `bench-internals`) | `large_service_workloads_preserve_cardinality_and_chunk_bounds` pins output cardinality and `1 + ceil(N / 128)` runner calls; `pid_enrichment_costs_one_subprocess_per_chunk_not_per_service` maps the same bound to `process.spawns` through the real bounded subprocess runner. |
| Case-sensitive/case-insensitive, warm/cold-ish | `workloads_filesystem_case/{warm_case_variant_tree,coldish_fresh_case_variant_tree}` | The fixture uses native `Path` components and therefore exercises the host filesystem's actual case behavior without emulation. |

---

## Environment Variables

| Variable | Effect |
|----------|--------|
| `SNIFF_BENCH_DEEP_DIRTY=1` | Includes the `1000` dirty-file row in `git_dirty_scaling` |
| `SNIFF_BENCH_DEEP_REPO=1` | Includes the `500` package row in `repo_package_boundaries` |

## What These Timings Are Worth

**Criterion timings here are directional evidence, not acceptance evidence.** Work
counters are what performance claims in sniff are judged on — see
`sniff/lib/src/performance/counters.rs` and
`cargo run -p sniff --release --example work_counts`.

Read this before quoting a number from a report:

- **Only compare within one OS and runner class.** No universal cross-OS
  wall-clock threshold exists, and none should be added.
- **Always include an unchanged case as a drift bracket.** On a loaded host these
  benches are worthless without one: a run at load 57–87 on 16 cores reported
  **+330%** for a case whose counters were byte-identical.
- **Case order matters within a run.** `repo_structure_huge_375_packages` runs
  before `repo_full_huge_375_packages` over the *same* fixture, so the full case
  reads a page cache the structure case warmed. Do not read a structure-vs-full
  ratio off one sequential run.
- **`_huge` is 375 packages but only ~3,755 files** (10 per package). It is
  package-dense and file-sparse, which is precisely the shape where
  `structure()` and `full()` converge. It is not a general model of "a big repo".

The fixture was previously described as "500 packages" in benchmark IDs while the
builder created 375. The IDs were renamed to match the code in Phase 1 of the
2026-07-16 performance feature; **never compare against an archived `huge_500`
result** — the workload never changed, but only post-rename runs are on record.

## How to Read Reports

1. **Console output** — Criterion prints each benchmark ID and its
   average time. The ID itself now contains the scenario description.
2. **HTML reports** — Open `target/criterion/report/index.html` after
   running. The group pages show throughput graphs when `Throughput`
   is configured (look for "per element" metrics).
3. **Throughput** — Benches annotated with `Throughput::Elements(N)`
   show time *per element* in the HTML report, making scaling trends
   immediately visible.
