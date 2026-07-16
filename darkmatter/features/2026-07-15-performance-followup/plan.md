---
agent: "claude/"
total_phases: 11
created: 2026-07-15
phase: 11
source_map_commit: bc1c148f26eae1bba36fc1f926298a52573d83bd
source_files_during_phase_1:
  - sniff/lib/src/os/time.rs
  - darkmatter/lib/src/markdown/fs.rs
  - darkmatter/lib/src/markdown/compose/context/capture/datetime.rs
  - darkmatter/cli/tests/hash_directory.rs
docs_updated_during_phase_1:
  - darkmatter/docs/cli/hash.md
docs_created_during_phase_1:
  - darkmatter/features/2026-07-15-performance-followup/results.md
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - darkmatter/lib/tests/benchmark_fixtures.rs
  - darkmatter/features/2026-07-15-performance-followup/benchmarks/generate.sh
docs_updated_during_phase_2:
  - darkmatter/features/2026-07-15-performance-followup/results.md
docs_created_during_phase_2:
  - darkmatter/features/2026-07-15-performance-followup/benchmarks/README.md
  - darkmatter/features/2026-07-15-performance-followup/benchmarks/manifest.yaml
  - darkmatter/features/2026-07-15-performance-followup/benchmarks/raw/README.md
  - darkmatter/features/2026-07-15-performance-followup/benchmarks/fixtures/render_basic.md
  - darkmatter/features/2026-07-15-performance-followup/benchmarks/fixtures/hash_basic.md
  - darkmatter/features/2026-07-15-performance-followup/benchmarks/fixtures/compose_trivial.md
  - darkmatter/features/2026-07-15-performance-followup/benchmarks/fixtures/compose_child.md
  - darkmatter/features/2026-07-15-performance-followup/benchmarks/fixtures/compose_schema_transclusion.md
  - darkmatter/features/2026-07-15-performance-followup/benchmarks/fixtures/render_code_heavy.md
  - darkmatter/features/2026-07-15-performance-followup/benchmarks/fixtures/toc_small.md
  - darkmatter/features/2026-07-15-performance-followup/benchmarks/fixtures/toc_medium.md
  - darkmatter/features/2026-07-15-performance-followup/benchmarks/fixtures/toc_large.md
  - darkmatter/features/2026-07-15-performance-followup/benchmarks/raw/f4-historical-closeout/run-20260715T232610/summary.md
  - darkmatter/features/2026-07-15-performance-followup/benchmarks/raw/f4-historical-closeout/run-20260715T232610/build.log
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - biscuit-terminal/lib/examples/discovery_probe.rs
  - biscuit-terminal/lib/tests/common/pty.rs
  - biscuit-terminal/lib/tests/level2_terminal_osc_cache.rs
  - darkmatter/cli/tests/compose_terminal_detection.rs
docs_updated_during_phase_3:
  - darkmatter/features/2026-07-15-performance-followup/results.md
docs_created_during_phase_3:
  - darkmatter/features/2026-07-15-performance-followup/benchmarks/raw/f2f3f21-terminal-evidence/run-20260716T065617/summary.md
  - darkmatter/features/2026-07-15-performance-followup/benchmarks/raw/f2f3f21-terminal-evidence/run-20260716T065617/piped-compose-vv-perf.json
  - darkmatter/features/2026-07-15-performance-followup/benchmarks/raw/f2f3f21-terminal-evidence/run-20260716T065617/interactive-pty-latency.txt
skills_files_updated_during_phase_3: []
packages_during_phase_3:
  - biscuit-terminal
  - darkmatter-cli
source_files_during_phase_4:
  - darkmatter/lib/src/markdown/compose/context/options.rs
  - darkmatter/lib/src/markdown/compose/cache/hashing.rs
docs_updated_during_phase_4:
  - darkmatter/features/2026-07-15-performance-followup/results.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
packages_during_phase_4:
  - darkmatter
source_files_during_phase_5:
  - darkmatter/lib/src/markdown/compose/cache/hashing.rs
  - darkmatter/lib/src/markdown/compose/transclusion/engine.rs
  - darkmatter/lib/src/markdown/compose/pipeline/phases.rs
  - darkmatter/lib/src/markdown/reference/graph.rs
  - darkmatter/lib/tests/compose_reuse_phase5.rs
  - darkmatter/lib/tests/reference_integration.rs
  - darkmatter/lib/tests/benchmark_fixtures.rs
  - darkmatter/features/2026-07-15-performance-followup/benchmarks/generate.sh
docs_updated_during_phase_5:
  - darkmatter/features/2026-07-15-performance-followup/results.md
  - darkmatter/features/2026-07-15-performance-followup/plan.md
  - darkmatter/features/2026-07-15-performance-followup/benchmarks/manifest.yaml
docs_created_during_phase_5:
  - darkmatter/features/2026-07-15-performance-followup/benchmarks/fixtures/compose_transclusion_heavy.md
  - darkmatter/features/2026-07-15-performance-followup/benchmarks/raw/f5-crosspass-reuse/run-20260716T000000/summary.md
  - darkmatter/features/2026-07-15-performance-followup/benchmarks/raw/f5-crosspass-reuse/run-20260716T000000/compose-transclusion-heavy.json
skills_files_updated_during_phase_5: []
packages_during_phase_5:
  - darkmatter
  - darkmatter-cli
source_files_during_phase_6:
  - darkmatter/lib/src/markdown/compose/frontmatter_interpolation.rs
  - darkmatter/lib/src/markdown/compose/interpolation/rewrite.rs
  - darkmatter/lib/src/markdown/compose/expression/mod.rs
  - darkmatter/lib/src/markdown/compose/context/effective_state.rs
  - darkmatter/lib/src/markdown/compose/subtree.rs
  - darkmatter/lib/src/markdown/compose/conditions.rs
  - darkmatter/lib/src/markdown/compose/replacement.rs
  - darkmatter/lib/Cargo.toml
  - darkmatter/lib/benches/phase6_interpolation.rs
  - darkmatter/lib/tests/compose_phase6.rs
  - darkmatter/lib/tests/benchmark_fixtures.rs
  - darkmatter/features/2026-07-15-performance-followup/benchmarks/generate.sh
docs_updated_during_phase_6:
  - darkmatter/features/2026-07-15-performance-followup/results.md
  - darkmatter/features/2026-07-15-performance-followup/plan.md
  - darkmatter/features/2026-07-15-performance-followup/benchmarks/manifest.yaml
  - darkmatter/docs/dependencies.md
docs_created_during_phase_6:
  - darkmatter/features/2026-07-15-performance-followup/benchmarks/fixtures/compose_interpolation_heavy.md
  - darkmatter/features/2026-07-15-performance-followup/benchmarks/fixtures/replace_heavy.md
  - darkmatter/features/2026-07-15-performance-followup/benchmarks/raw/f11f12f13f14-interpolation/run-20260716T085358/summary.md
  - darkmatter/features/2026-07-15-performance-followup/benchmarks/raw/f11f12f13f14-interpolation/run-20260716T085358/cli-compose-interpolation-heavy.json
  - darkmatter/features/2026-07-15-performance-followup/benchmarks/raw/f11f12f13f14-interpolation/run-20260716T085358/criterion-apply_replacements-baseline.json
  - darkmatter/features/2026-07-15-performance-followup/benchmarks/raw/f11f12f13f14-interpolation/run-20260716T085358/criterion-apply_replacements-candidate.json
  - darkmatter/features/2026-07-15-performance-followup/benchmarks/raw/f11f12f13f14-interpolation/run-20260716T085358/criterion-f14-markdown-scan.json
  - darkmatter/features/2026-07-15-performance-followup/benchmarks/raw/f11f12f13f14-interpolation/run-20260716T085358/criterion-f14-contains-guard.json
skills_files_updated_during_phase_6: []
packages_during_phase_6:
  - darkmatter
  - darkmatter-cli
source_files_during_phase_7:
  - darkmatter/lib/Cargo.toml
  - darkmatter/lib/src/markdown/compose/shell_expansion/executor.rs
  - darkmatter/lib/src/markdown/compose/shell_expansion/mod.rs
  - darkmatter/lib/src/markdown/compose/inline/shell_expansion.rs
  - darkmatter/lib/src/markdown/compose/shell_blocks/mod.rs
  - darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs
docs_updated_during_phase_7:
  - darkmatter/features/2026-07-15-performance-followup/results.md
  - darkmatter/features/2026-07-15-performance-followup/plan.md
  - darkmatter/docs/dependencies.md
docs_created_during_phase_7: []
skills_files_updated_during_phase_7:
  - .claude/skills/darkmatter/compose.md
packages_during_phase_7:
  - darkmatter
source_files_during_phase_8:
  - darkmatter/lib/src/markdown/render_tree/code_renderer.rs
  - darkmatter/lib/benches/phase8_render.rs
  - darkmatter/lib/Cargo.toml
docs_updated_during_phase_8:
  - darkmatter/features/2026-07-15-performance-followup/results.md
  - darkmatter/features/2026-07-15-performance-followup/plan.md
docs_created_during_phase_8:
  - darkmatter/features/2026-07-15-performance-followup/benchmarks/raw/f23f25-render-cleanup/run-20260716T120000/summary.md
  - darkmatter/features/2026-07-15-performance-followup/benchmarks/raw/f23f25-render-cleanup/run-20260716T120000/f25-cleanup-pass-profile.txt
  - darkmatter/features/2026-07-15-performance-followup/benchmarks/raw/f23f25-render-cleanup/run-20260716T120000/criterion-as_terminal_code_heavy-baseline.json
  - darkmatter/features/2026-07-15-performance-followup/benchmarks/raw/f23f25-render-cleanup/run-20260716T120000/criterion-as_terminal_code_heavy-candidate.json
  - darkmatter/features/2026-07-15-performance-followup/benchmarks/raw/f23f25-render-cleanup/run-20260716T120000/criterion-as_html_code_heavy-baseline.json
  - darkmatter/features/2026-07-15-performance-followup/benchmarks/raw/f23f25-render-cleanup/run-20260716T120000/criterion-as_html_code_heavy-candidate.json
  - darkmatter/features/2026-07-15-performance-followup/benchmarks/raw/f23f25-render-cleanup/run-20260716T120000/criterion-code_block_direct-baseline.json
  - darkmatter/features/2026-07-15-performance-followup/benchmarks/raw/f23f25-render-cleanup/run-20260716T120000/criterion-code_block_direct-candidate.json
skills_files_updated_during_phase_8: []
packages_during_phase_8:
  - darkmatter
source_files_during_phase_9:
  - darkmatter/lib/src/markdown/compose/remote.rs
  - darkmatter/lib/tests/benchmark_fixtures.rs
  - darkmatter/lib/benches/phase9_remote.rs
  - darkmatter/lib/Cargo.toml
  - darkmatter/features/2026-07-15-performance-followup/benchmarks/generate.sh
docs_updated_during_phase_9:
  - darkmatter/features/2026-07-15-performance-followup/results.md
  - darkmatter/features/2026-07-15-performance-followup/plan.md
  - darkmatter/features/2026-07-15-performance-followup/benchmarks/manifest.yaml
docs_created_during_phase_9:
  - darkmatter/features/2026-07-15-performance-followup/benchmarks/fixtures/remote_heavy.md
  - darkmatter/features/2026-07-15-performance-followup/benchmarks/raw/f33-remote-discovery/run-20260716T140000/summary.md
  - darkmatter/features/2026-07-15-performance-followup/benchmarks/raw/f33-remote-discovery/run-20260716T140000/criterion-f33_discover_remote_heavy-baseline.json
  - darkmatter/features/2026-07-15-performance-followup/benchmarks/raw/f33-remote-discovery/run-20260716T140000/criterion-f33_discover_remote_heavy-candidate.json
  - darkmatter/features/2026-07-15-performance-followup/benchmarks/raw/f33-remote-discovery/run-20260716T140000/criterion-f33_discover_no_http_guard-baseline.json
  - darkmatter/features/2026-07-15-performance-followup/benchmarks/raw/f33-remote-discovery/run-20260716T140000/criterion-f33_discover_no_http_guard-candidate.json
  - darkmatter/features/2026-07-15-performance-followup/benchmarks/raw/f33-remote-discovery/run-20260716T140000/criterion-f33_discover_http_without_expressions-baseline.json
  - darkmatter/features/2026-07-15-performance-followup/benchmarks/raw/f33-remote-discovery/run-20260716T140000/criterion-f33_discover_http_without_expressions-candidate.json
skills_files_updated_during_phase_9: []
packages_during_phase_9:
  - darkmatter
source_files_during_phase_10:
  - darkmatter/lib/src/markdown/span.rs
  - darkmatter/lib/src/markdown/toc/mod.rs
  - darkmatter/lib/src/markdown/compose/transclusion/engine.rs
  - darkmatter/lib/src/layout/page.rs
  - darkmatter/lib/src/layout/page/tests.rs
  - darkmatter/lib/src/markdown/render_tree/build_context.rs
  - darkmatter/lib/src/markdown/hash/compare.rs
  - darkmatter/lib/src/markdown/hash/explain.rs
  - darkmatter/lib/src/markdown/hash/save.rs
  - darkmatter/lib/benches/phase10_residuals.rs
  - darkmatter/lib/Cargo.toml
docs_updated_during_phase_10:
  - darkmatter/features/2026-07-15-performance-followup/results.md
  - darkmatter/features/2026-07-15-performance-followup/plan.md
docs_created_during_phase_10:
  - darkmatter/features/2026-07-15-performance-followup/benchmarks/raw/f35-residuals/run-20260716T160000/summary.md
  - darkmatter/features/2026-07-15-performance-followup/benchmarks/raw/f35-residuals/run-20260716T160000/f35_3-copy-cost-model.txt
  - darkmatter/features/2026-07-15-performance-followup/benchmarks/raw/f35-residuals/run-20260716T160000/f35_5-hash-artifact-profile.txt
  - darkmatter/features/2026-07-15-performance-followup/benchmarks/raw/f35-residuals/run-20260716T160000/f35_6-rhythm-profile.txt
  - darkmatter/features/2026-07-15-performance-followup/benchmarks/raw/f35-residuals/run-20260716T160000/f35_7-link-policy-profile.txt
  - darkmatter/features/2026-07-15-performance-followup/benchmarks/raw/f35-residuals/run-20260716T160000/criterion-f35_2_relevel_prefix_toc_large-baseline.json
  - darkmatter/features/2026-07-15-performance-followup/benchmarks/raw/f35-residuals/run-20260716T160000/criterion-f35_2_relevel_prefix_toc_large-candidate.json
  - darkmatter/features/2026-07-15-performance-followup/benchmarks/raw/f35-residuals/run-20260716T160000/criterion-f35_2_relevel_overflow_toc_large-baseline.json
  - darkmatter/features/2026-07-15-performance-followup/benchmarks/raw/f35-residuals/run-20260716T160000/criterion-f35_2_relevel_overflow_toc_large-candidate.json
  - darkmatter/features/2026-07-15-performance-followup/benchmarks/raw/f35-residuals/run-20260716T160000/criterion-f35_2_relevel_extract_only-baseline.json
  - darkmatter/features/2026-07-15-performance-followup/benchmarks/raw/f35-residuals/run-20260716T160000/criterion-f35_2_relevel_extract_only-candidate.json
  - darkmatter/features/2026-07-15-performance-followup/benchmarks/raw/f35-residuals/run-20260716T160000/criterion-f35_2_relevel_no_headings-baseline.json
  - darkmatter/features/2026-07-15-performance-followup/benchmarks/raw/f35-residuals/run-20260716T160000/criterion-f35_2_relevel_no_headings-candidate.json
skills_files_updated_during_phase_10: []
packages_during_phase_10:
  - darkmatter
  - darkmatter-cli
source_files_during_phase_11:
  - biscuit-terminal/justfile
docs_updated_during_phase_11:
  - darkmatter/features/2026-07-15-performance-followup/results.md
  - darkmatter/features/2026-07-15-performance-followup/spec.md
  - darkmatter/features/2026-07-15-performance-followup/plan.md
  - darkmatter/reviews/2026-07-12-perf/spec.md
  - darkmatter/reviews/2026-07-12-perf/plan.md
  - darkmatter/reviews/2026-07-12-perf/results.md
  - darkmatter/reviews/2026-07-12-perf/results-2.md
  - darkmatter/reviews/2026-07-12-perf/baseline.md
  - sniff/lib/README.md
docs_created_during_phase_11:
  - darkmatter/features/2026-07-15-performance-followup/benchmarks/raw/f-cumulative-closeout/run-20260716T050518/declared-contract.md
  - darkmatter/features/2026-07-15-performance-followup/benchmarks/raw/f-cumulative-closeout/run-20260716T050518/summary.md
  - darkmatter/features/2026-07-15-performance-followup/benchmarks/raw/f-cumulative-closeout/run-20260716T050518/linux-behavioral-run.txt
skills_files_updated_during_phase_11: []
packages_during_phase_11:
  - darkmatter
  - darkmatter-cli
  - sniff
  - biscuit-terminal
packages:
  - sniff
  - darkmatter
  - darkmatter-cli
  - biscuit-terminal
source_code:
  - sniff/lib/src/os/time.rs
  - darkmatter/lib/Cargo.toml
  - darkmatter/lib/src/layout/page.rs
  - darkmatter/lib/src/layout/page/tests.rs
  - darkmatter/lib/src/markdown/fs.rs
  - darkmatter/lib/src/markdown/span.rs
  - darkmatter/lib/src/markdown/toc/mod.rs
  - darkmatter/lib/src/markdown/hash/compare.rs
  - darkmatter/lib/src/markdown/hash/explain.rs
  - darkmatter/lib/src/markdown/hash/save.rs
  - darkmatter/lib/src/markdown/reference/graph.rs
  - darkmatter/lib/src/markdown/render_tree/build_context.rs
  - darkmatter/lib/src/markdown/render_tree/code_renderer.rs
  - darkmatter/lib/src/markdown/compose/cache/hashing.rs
  - darkmatter/lib/src/markdown/compose/conditions.rs
  - darkmatter/lib/src/markdown/compose/context/capture/datetime.rs
  - darkmatter/lib/src/markdown/compose/context/effective_state.rs
  - darkmatter/lib/src/markdown/compose/context/options.rs
  - darkmatter/lib/src/markdown/compose/expression/mod.rs
  - darkmatter/lib/src/markdown/compose/frontmatter_interpolation.rs
  - darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs
  - darkmatter/lib/src/markdown/compose/inline/shell_expansion.rs
  - darkmatter/lib/src/markdown/compose/interpolation/rewrite.rs
  - darkmatter/lib/src/markdown/compose/pipeline/phases.rs
  - darkmatter/lib/src/markdown/compose/remote.rs
  - darkmatter/lib/src/markdown/compose/replacement.rs
  - darkmatter/lib/src/markdown/compose/shell_blocks/mod.rs
  - darkmatter/lib/src/markdown/compose/shell_expansion/executor.rs
  - darkmatter/lib/src/markdown/compose/shell_expansion/mod.rs
  - darkmatter/lib/src/markdown/compose/subtree.rs
  - darkmatter/lib/src/markdown/compose/transclusion/engine.rs
  - darkmatter/lib/benches/phase6_interpolation.rs
  - darkmatter/lib/benches/phase8_render.rs
  - darkmatter/lib/benches/phase9_remote.rs
  - darkmatter/lib/benches/phase10_residuals.rs
  - darkmatter/lib/tests/benchmark_fixtures.rs
  - darkmatter/lib/tests/compose_phase6.rs
  - darkmatter/lib/tests/compose_reuse_phase5.rs
  - darkmatter/lib/tests/reference_integration.rs
  - darkmatter/cli/tests/hash_directory.rs
  - darkmatter/cli/tests/compose_terminal_detection.rs
  - biscuit-terminal/justfile
  - biscuit-terminal/lib/examples/discovery_probe.rs
  - biscuit-terminal/lib/tests/common/pty.rs
  - biscuit-terminal/lib/tests/level2_terminal_osc_cache.rs
  - darkmatter/features/2026-07-15-performance-followup/benchmarks/generate.sh
documentation:
  - darkmatter/features/2026-07-15-performance-followup/spec.md
  - darkmatter/features/2026-07-15-performance-followup/plan.md
  - darkmatter/features/2026-07-15-performance-followup/results.md
  - darkmatter/features/2026-07-15-performance-followup/benchmarks/README.md
  - darkmatter/features/2026-07-15-performance-followup/benchmarks/manifest.yaml
  - darkmatter/features/2026-07-15-performance-followup/benchmarks/raw/README.md
  - darkmatter/docs/cli/hash.md
  - darkmatter/docs/dependencies.md
  - darkmatter/reviews/2026-07-12-perf/spec.md
  - darkmatter/reviews/2026-07-12-perf/plan.md
  - darkmatter/reviews/2026-07-12-perf/results.md
  - darkmatter/reviews/2026-07-12-perf/results-2.md
  - darkmatter/reviews/2026-07-12-perf/baseline.md
  - sniff/lib/README.md
  - .claude/skills/darkmatter/compose.md
---

# Execution Plan — Performance Follow-up

Closes the delivery-contract gaps found by auditing all 35 findings of the
2026-07-12 performance review against `51c1f16e10ffe825b56987573ba4eabc659c768e`.
It restores two forbidden behavior changes (Findings 1, 22), builds the
requirement-matched terminal and command/TOC evidence the review lacked
(Findings 2/3/21, 4), finishes the deferred optimizations (Findings 7, 11–14,
16, 17, 23, 25, 32, 33, and the seven Finding-35 residuals), and implements
Architecture Decisions A (feature-local evidence behind one fixture manifest)
and B (one exhaustive `ComposeOptions` field classification driving
purpose-specific identities).

Opaque `ReferenceGraph` correctness (Finding 18) is **out of scope** — it is
owned by the linked [Opaque Reference Graph](../2026-07-15-reference-graph/plan.md)
feature. This plan only *coordinates* with it on the shared field-classification
authority (Architecture Decision B / Phase 4).

## Source Map (verified at `bc1c148f26eae1bba36fc1f926298a52573d83bd`)

Symbol names and paths are authoritative; line numbers are navigation hints at
the pinned commit and must be refreshed before each implementation phase.

| Concern | Location | Finding |
|---------|----------|---------|
| `detect_timezone_with_options(probe_ntp)` / bare `detect_timezone()` (delegates to `false` — must restore `true`) | `sniff/lib/src/os/time.rs:429`, `:508` | 1 |
| Darkmatter local-only caller (keep `false`) | `darkmatter/lib/src/markdown/compose/context/capture/datetime.rs:129` | 1 |
| OSC 10 text-color cache; OSC support session cache | `biscuit-terminal/lib/src/discovery/osc_queries/mod.rs:73` (`TEXT_COLOR_CACHE`), `:105`; `.../osc_queries/support.rs:11` | 2 |
| macOS color-mode / prose-theme probe | `darkmatter/lib/src/markdown/highlighting/themes.rs:412` `detect_prose_theme`, `:473` `detect_color_mode`; terminal build `biscuit-terminal/lib/src/terminal.rs:51` | 21 |
| Compose CLI shared terminal `OnceCell` | `darkmatter/cli/src/commands/compose.rs:191` (`term_cell`) | 3 |
| TOC newline offset table + `partition_point` | `darkmatter/lib/src/markdown/toc/mod.rs:193` `newline_offset_table`, `:210` `line_at_offset` | 4 |
| Frontmatter interpolation fixpoint; per-iteration ref extract + seed clone | `darkmatter/lib/src/markdown/compose/frontmatter_interpolation.rs:362` `interpolate_frontmatter_impl`; repeated call sites at `:431`/`:469`/`:559`; helpers `extract_frontmatter_key_refs` at `:871`, `collect_deferred_key_references` at `:964` | 11 |
| Expression `Option<ResolutionContext>` owned clone | `darkmatter/lib/src/markdown/compose/expression/resolve_ctx.rs:19`, `:128` `resolution_context()`; `functions/mod.rs:45` `ContextFn` | 12 |
| Text `replace:` matcher + char-index vector | `darkmatter/lib/src/markdown/compose/replacement.rs:88` `apply_replacements`, `:105` `build_replacement_rules`, `:165` `scan_and_replace`; stage `inline/replacement.rs:19` | 13 |
| Literal `{{{` conversion + guard | `darkmatter/lib/src/markdown/compose/interpolation/rewrite.rs:69` `convert_literals`, `:201` guard; `frontmatter_interpolation.rs:217` `convert_frontmatter_literals` | 14 |
| `options_hash` / `effective_state_hash` / `context_hash` / overlay hashes | `darkmatter/lib/src/markdown/compose/cache/hashing.rs:141`/`:87`/`:103`/`:150`/`:168` | 7,16,AD-B,35 |
| `ComposeOptions` owning module | `darkmatter/lib/src/markdown/compose/context/options.rs:44` | AD-B,7,16 |
| Transclusion key producer + persistent-key consumers | `.../transclusion/engine.rs:1335` (`options_hash` at `:1336`); `.../cache/runtime.rs:68` (`PersistentContext`, consumed by persistent read/write key assembly) | 7,16,AD-B |
| Preflight cached-directive reuse | `.../compose/preflight/mod.rs:122` (`canonical_key`), `:140` (`child_for_source`) | 7,16 |
| Shell 10ms polling loops (**two**) | `darkmatter/lib/src/markdown/compose/shell_expansion/executor.rs:245`/`:309`, `:577`/`:634` | 17 |
| Shell policy snapshot cloned per directive | `.../shell_expansion/mod.rs:188` `shell_runtime.snapshot()`; `.../shell_expansion/types.rs:1060` `ShellExpansionRuntime::snapshot` | 32 |
| Per-code-block environment/theme resolution | `darkmatter/lib/src/markdown/render_tree/code_renderer.rs:176` `code_theme_from_env`, `:231` resolution chain; render entry points `render_tree/entrypoints.rs:612`, `:771` | 23 |
| Cleanup two-stage pipeline; placeholder + line passes; reflow | `darkmatter/lib/src/markdown/cleanup/mod.rs:214` `cleanup_content_internal`; `strip_incidental_newlines` calls; `cleanup/reflow.rs` | 25 |
| Directory-hash vendor exclusion | `darkmatter/lib/src/markdown/fs.rs:8` `SKIPPED_VENDOR_DIRS`, `:35` pruning condition | 22 |
| Remote discovery per-expression prefix rescan | `darkmatter/lib/src/markdown/compose/remote.rs:287` `discover_remote_urls_from_expressions`, `:311` loop calling `byte_offset_to_line` (`:446`) | 33 |
| Child-heading line lookup / repeated relevel copies | `.../transclusion/engine.rs:76` `relevel_with_overflow`, `:138` reverse replacement loop, `:181` `extract_headings` (`content[..start].lines()` at `:197`) | 35.2 |
| Fetched response body (`String` → `Arc<str>`; `Arc` already imported) | `darkmatter/lib/src/markdown/compose/remote_fetch.rs:38` `FetchSlot::Ready`, `:447` `get_content` clone; `cache/remote_cache.rs:58` fetch outcome handoff | 35.3 |
| `::toc-linking` target reads across graph + compose runtimes | `darkmatter/lib/src/markdown/reference/graph.rs:36`/`:49` (`ReferenceAnalysisRuntime` + run-local load), `:487` child load; `.../transclusion/engine.rs:1103`–`:1134` direct read + TOC-heading load | 35.4 |
| `md hash --diff` / `--save` | `darkmatter/cli/src/commands/hash.rs:33`,`:141` `run_hash_diff`, `:164` `run_hash_save`; lib `plan_hash_save`/`apply_hash_save` | 35.5 |
| `normalize_body_rhythm` (ANSI-strip per line) | `darkmatter/lib/src/layout/page.rs:1423` (called `:950`) | 35.6 |
| Link/image URL/title policy application | `darkmatter/lib/src/markdown/render_tree/build_context.rs:375` `apply_link_policy`, `:411` `apply_image_policy` | 35.7 |
| `md delta` command | `darkmatter/cli/src/commands/mod.rs:175`; lib `markdown/delta/mod.rs` | 35 (ctx) |

### Confirmed constraints from the read-through

- Two independent 10 ms polling loops exist in `executor.rs` (`try_wait` at
  `:245`/`:577`, sleeps at `:309`/`:634`); the Finding 17 fix must replace
  **both**.
- `remote_fetch.rs` already imports `Arc`; the Finding-35.3 `Arc<str>` change is
  centered on `FetchSlot::Ready { body }`, the owned `get_content` facade, and
  the `RemoteFetchOutcome` handoff from `cache/remote_cache.rs`.
- `toc_linking/mod.rs` re-reads targets with `read_to_string` in multiple spots,
  but its `process_toc_linking` helper is currently test-only. Finding 35.4's
  production duplication is between `ReferenceAnalysisRuntime::load_markdown`
  and the transclusion engine's direct source/hash + TOC-heading reads; optimize
  those paths rather than the legacy helper.
- The existing `cache::hashing::options_hash` is the incumbent that
  Architecture Decision B must **replace or delegate**, not run parallel to.
- `magic_paths` and `env_path_whitelist` are ordered vectors whose order can
  affect lookup/normalization behavior. They must never be sorted for identity;
  only genuinely unordered maps/sets are canonicalized by sorting.
- The existing OSC probe infrastructure already lives in
  `biscuit-terminal/lib/examples/discovery_probe.rs` and
  `biscuit-terminal/lib/tests/common/pty.rs`; Finding 2 extends that path rather
  than creating a second PTY abstraction.

---

## Standing Contracts (apply to every phase)

These are not a phase; they gate every checkpoint. Each optimization task must
honor them or record an explicit disposition.

- **Compatibility invariants (spec §Compatibility and Correctness Invariants
  1–8):** compose Markdown, validation results, rendered output, graph/CLI JSON,
  diagnostics, and exit status stay byte-for-byte and error-for-error
  compatible; this follow-up introduces no new public Rust API shape change and
  preserves Finding 29's already-approved ownership exception plus owned
  compatibility facade; caches include every semantic input and are bounded or
  run-local and concurrency-safe; internal borrowing never weakens an owned
  public facade without an approved exception; all code compiles and behaves on
  macOS, Linux, and Windows.
- **Benchmark & evidence contract (spec §Benchmark and Evidence Contract):**
  before measuring, every checkpoint declares target operation + control groups,
  fixture identity/size, build profile/commands/environment/host/TTY mode,
  warm-up/sample count/statistic/dispersion, and the minimum repeatable win +
  maximum permitted control regression. Baseline and candidate use identical
  source/fixture/harness bytes except the code under test. Raw samples retained.
  A no-repeatable-win finding closes through a recorded no-win disposition **and
  removal of the unnecessary code**.
- **Hashing authority:** Darkmatter Markdown-aware hashing (`md hash`) for
  Markdown identities; `biscuit-hash` xxHash for non-Markdown or whole-file
  byte identity. No ad hoc hashing.
- **Cache-identity encoding:** use a versioned, typed, length-delimited canonical
  encoder. Preserve ordered collections; sort only genuinely unordered values;
  distinguish `None` from empty values and field/type boundaries; never join
  unescaped `Debug`/display strings. A changed encoding uses a new cache-key
  domain so legacy persistent entries cannot be read under new semantics.
- **Test-tier contract:** L2 is reserved for behavior requiring a real terminal
  or PTY and runs only through `just test-l2`; spawning an ordinary child process
  remains L1. Browser-rendering behavior runs through the headless
  `just test-browser` tier. Do not add Level 3 host-input coverage.
- **Cross-platform gate honesty (spec §Verification Matrix):** OS-divergent
  paths (F17 shell wait primitive, the F2/F3/F21 PTY helper, F22 traversal)
  **require** a real non-macOS behavioral run, not just a cross-compile.
  OS-identical paths state that identity in their disposition and treat Windows
  compile evidence + the macOS behavioral run + ordinary Linux CI as sufficient.
  Classify from the code actually changed, not the finding number.
- **No write-mode formatter** is authorized. `cargo fmt --check` (read-only)
  and `git diff --check` only.

---

## Preflight — source freshness, ownership, and impact

- [x] Record the current commit and working-tree state; preserve unrelated
  changes. Compare it with `source_map_commit`; treat the Source Map as a
  reviewed starting point, not a substitute for resolving each named symbol and
  confirming its current role immediately before its phase. (HEAD
  `b425fb466`; pre-existing unrelated edits in `CLAUDE.md`,
  `compose/context/{options,runtime}.rs`, `reference/provenance.rs` preserved.)
- [x] Confirm the linked Opaque Reference Graph prerequisite/ownership boundary
  before touching `ComposeOptions` or cache identity. One feature owns the
  classification landing commit; this plan consumes it. (Confirmed in Phase 4:
  the reference-graph feature owns the single `classify_options` inventory and
  both identity products, landed in `a8e5e98d9`; this plan consumes that
  classification and only finalizes the compose-cache encoding — no competing
  inventory. Recorded in `results.md` Phase 4.)
- [x] Before editing any function, method, class, or other indexed symbol, run
  GitNexus upstream `impact` for that symbol and record direct callers,
  processes, modules, and risk. Warn and stop for owner direction if risk is
  HIGH or CRITICAL. Documentation/fixture-only changes do not require symbol
  impact analysis. (`detect_timezone` LOW / 3 callers; `collect_markdown_files`
  LOW / 2 direct + `run_subcommand` process; no signature changes.)
- [x] Before each measured optimization, confirm its fixture entry, run-record
  location, target/control operations, threshold, and immediate baseline commit
  are frozen. (Standing item, honored at every measured checkpoint and closed at
  Phase 11. No measured optimization in Phase 1. Fixtures were registered +
  hashed in `manifest.yaml` **before** their checkpoint's baseline in every case
  that added one — `compose_transclusion_heavy` (P5, generator 1.0.0→1.1.0),
  `compose_interpolation_heavy`/`replace_heavy` (P6, →1.2.0), `remote_heavy`
  (P9, →1.3.0); P8, P10, and P11 added none and reused existing entries. Each
  checkpoint declared its threshold + target/control operations before capture
  and retained raw samples under `benchmarks/raw/<checkpoint>/<run-id>/`; the
  Phase-11 cumulative run wrote its `declared-contract.md` before any
  measurement. Deviation recorded rather than hidden: from P10 on, baseline and
  candidate are sampled **interleaved in one process** instead of via cross-run
  Criterion baselines, after identical code re-measured across three runs read
  290→397→420 µs on this loaded host.)

---

## Phase 1 — Compatibility corrections (Findings 1 & 22)

Revert the two forbidden behavior changes. Both are small, independent, and
high-priority; landing them early re-establishes invariants 3 and 4 before the
larger optimization work builds on top.

### Finding 1 — Restore the Sniff timezone compatibility boundary (Work 1)
- [x] In `sniff/lib/src/os/time.rs:508`, restore bare `detect_timezone()` to delegate to `detect_timezone_with_options(true)` (full NTP-reporting convenience API). Align its rustdoc.
- [x] Keep Darkmatter's explicit `detect_timezone_with_options(false)` call at `capture/datetime.rs:129` unchanged. (Now routed through the local `capture_timezone_info` seam, which still passes `false`.)
- [x] Add a narrow Sniff-internal decision seam (for example, an injected probe function below both public entry points) so Sniff tests prove the bare API selects `true` and the configurable API respects both values without making a live NTP request. (`run_ntp_probe()` seam + `#[cfg(test)]` thread-local stub; tests `bare_detect_timezone_probes_ntp`, `detect_timezone_with_options_respects_probe_flag`.)
- [x] Add a Darkmatter-local injectable wrapper or equivalent seam around its Sniff call and prove the production path selects `false`; do not depend on Sniff's `cfg(test)` instrumentation crossing the crate boundary, use a brittle source-text assertion, or introduce a live network dependency in ordinary compose tests. (`capture_timezone_info(fetch)` seam; test `compose_datetime_capture_never_probes_ntp` injects a spy proving `probe_ntp: false`.)
- [x] Gates: Sniff `just test` + `just lint`; Darkmatter context tests + `just test` + `just lint`. (This is a filesystem/network-adjacent but OS-identical logic change — Windows compile + macOS run + Linux CI sufficient; state so in `results.md`.) (Sniff `just test` 1334/1335 — one pre-existing unrelated `filesystem::repo::area` env hang; Sniff/Darkmatter `just lint` clean; OS-identical classification recorded in `results.md`.)

### Finding 22 — Restore directory-hash membership (Work 8)
- [x] Remove the unconditional `node_modules` / `target` / `vendor` exclusion in `darkmatter/lib/src/markdown/fs.rs` (`SKIPPED_VENDOR_DIRS` skip at `:24`) so aggregate membership matches pre-optimization behavior. (Const removed; only dot-prefixed dirs pruned.)
- [x] Add an **end-to-end CLI** test that freezes the aggregate hash, diagnostics, and exit status for a tree containing directories named `node_modules`/`target`/`vendor`. (`hash_directory::test_hash_directory_includes_vendored_dirs` + `..._vendored_membership_matches_plain_dir`.)
- [x] Confirm no hash-migration step is needed (the exclusion was never released; any aggregate under it is a private working-tree artifact). Record in `results.md` that a *future* opt-in ignore policy would require owner approval + migration semantics. (Recorded in `results.md` Finding 22.)
- [x] Gates: Darkmatter `just test` + `just lint`. F22 traversal/path handling is OS-divergent → record behavioral runs of the CLI aggregate test on macOS, Linux, and Windows. (Darkmatter `just lint` clean + CLI aggregate tests pass on macOS; **Linux + Windows behavioral runs deferred to Phase 11** — not executable on this macOS-only host, recorded as a gap in `results.md`.)

**Parallelizable:** Finding 1 (Sniff + Darkmatter caller) and Finding 22
(Darkmatter fs + CLI) touch disjoint code and can proceed concurrently.

### Checkpoint 1
Bare `sniff::detect_timezone()` reports full NTP status again; Darkmatter compose
still performs no NTP probe. `md hash <dir>` includes Markdown under
`node_modules`/`target`/`vendor` exactly as before the perf change. Both areas
green on `just test` + `just lint`; macOS, Linux, and Windows behavioral evidence
recorded for F22.

---

## Phase 2 — Evidence infrastructure & command/TOC closeout (AD-A + Finding 4)

Establish the feature-local evidence home and fixture-manifest schema that every
measured checkpoint consumes. Reconstruct the reproducible historical
command/TOC closeout that Review 3 rejected for using different, unhashed
fixture bytes. **Blocks every measured checkpoint** in later phases.

- [x] Create `darkmatter/features/2026-07-15-performance-followup/results.md` as the disposition + evidence index (one row per retained partial/open/correction finding or sub-item, including evidence-only gaps; disposition, evidence location, and cross-platform classification columns). (AD-A)
- [x] Create a sibling `benchmarks/` directory holding the immutable fixture **manifest** as the single authority for fixture identity, plus either committed fixtures or a checked-in **deterministic generator** (record generator version + exact command). (AD-A, Work 3)
- [x] Define the manifest schema up front. Each fixture entry records generator version/command, exact byte size + structural counts, Darkmatter frontmatter/body hashes for Markdown, and a `biscuit-hash` xxHash whole-file identity where byte identity is required. Preserve ordered fixture collections. (Work 3)
- [x] Keep per-run facts out of the immutable fixture identity: dated run records under `benchmarks/raw/<checkpoint>/<run-id>/` record baseline/candidate commits, commands, release profile, host, environment, TTY mode, warm-up, sample count, statistic/dispersion, thresholds, and raw-result files. `results.md` links each disposition to its run record. Declare the threshold before capturing the baseline. (AD-A, Work 3)
- [x] Populate the Phase-2 fixture set with `md --help`, render, hash, trivial compose, schema/transclusion compose, the three TOC size tiers, and code-heavy render cases. Later phases may add checkpoint-specific fixtures only by registering and hashing them **before** that checkpoint's baseline is captured. (Work 3)
- [x] Record the three runner contracts in `results.md`: existing Criterion recipes for library microbenchmarks, a release CLI runner for command-level measurement, and the existing Biscuit Terminal probe/PTY path extended in Phase 3. Each records commands, raw samples, environment; each consumes the shared manifest for file fixtures. Do **not** force CLI/PTY evidence through `just bench`. (AD-A)
- [x] Historical F4 closeout: create isolated temporary worktrees for the pre-optimization baseline `83aaecc8f` and audit commit `51c1f16e10ffe825b56987573ba4eabc659c768e`; build with the same toolchain/lockfile policy and release profile; run both against the **same immutable fixture directory on the same host**; record threshold pass/fail. These pins reconstruct the accumulated 2026-07-12 result only — they are **not** the baseline/candidate pair for this follow-up's changes. (Work 3)
- [x] Add TOC unit/property coverage confirming line/span behavior over the manifest fixtures (guards the non-quadratic `line_at_offset` path). (Work 3, verification matrix F4)

The manifest schema and deterministic generator can be prepared together. Do
not begin the historical builds until their fixture entries and run-record
contract are frozen.

### Checkpoint 2
`results.md`, the fixture manifest, and the Phase-2 fixtures exist and are
internally consistent (recomputed hashes match recorded ones). The F4 historical
closeout reproduces on identical bytes and meets its predeclared thresholds,
with raw samples retained. No production behavior changes in this phase; test
and benchmark-support edits are permitted.

---

## Phase 3 — Requirement-matched terminal evidence (Findings 2, 3, 21) — Work 2

Extend the checked-in Biscuit Terminal probe/PTY path so it observes OSC
requests independent of the user's shell theme, then add the CLI
single-detection case. This is the evidence gap Review 3 flagged as "wrong
level". Depends on the Phase 2 evidence home for recording latency artifacts.

- [x] Extend `biscuit-terminal/lib/examples/discovery_probe.rs` and `biscuit-terminal/lib/tests/common/pty.rs`; do **not** add a second generic PTY abstraction. Put the assertions in a `level2_*` test binary and run them only through `just test-l2`. Unix-only `expectrl`/PTY code is target-gated so Windows compiles and records a clean unsupported/skip disposition. (spec Work 2.6) (New probe modes `terminal_cache`/`terminal_latency`; `pty.rs` gained `OscAnswer`/`drive_probe`/`count_occurrences` on the existing `expectrl` path; `level2_terminal_osc_cache.rs` with a `#[cfg(not(unix))]` skip stub. Passes `just test-l2`.)
- [x] Run the cache proof in a dedicated child process so prior `OnceLock` state or test ordering cannot contaminate it. Construct two or more `Terminal` values, manufacture a response only for the first OSC 10 request, and assert exactly one request plus the same cached first response on later constructions. This proves reuse rather than coincidental equality. (Work 2.1/2.2) (`level2_terminal_construction_emits_single_osc10_request`: the probe child is the dedicated process; 3 constructions → exactly one `\x1b]10;?\x07`; every construction reports the distinctive manufactured `RgbValue { r: 18, g: 86, b: 154 }`, not any terminal default.)
- [x] Record repeated-construction latency with warm-up, sample count, and dispersion into the feature-local evidence index. (Work 2.3) (`level2_terminal_repeated_construction_latency`: warm-up 3, 50 samples, median 0.970 ms ± 0.022 ms; raw samples + summary in `benchmarks/raw/f2f3f21-terminal-evidence/run-20260716T065617/`.)
- [x] Add an isolated CLI probe case: one `md compose` invocation rendering verbose + performance + warning output performs **one** terminal detection (exercises the `compose.rs:191` `term_cell` `OnceCell`). Count the emitted detection/query requests rather than inferring from equal rendered output. (Work 2.4) (`compose_verbose_perf_performs_single_terminal_detection`: `-vv --perf` + a `{{ 1 + }}` warning exercises the verbose/perf/warning branches; counts the `biscuit_terminal::terminal` "Terminal detected" span == 1.)
- [x] Verify macOS appearance discovery (`detect_color_mode`/`detect_prose_theme`) does **not** spawn for fully redirected output. Keep this redirected-process assertion L1; it does not require a PTY. Serialize environment mutation with the repository test guard. (Work 2.5, Finding 21) (`compose_redirected_does_not_spawn_appearance_defaults` (`cfg(macos)`): a sentinel-writing `defaults` PATH shim is never invoked for redirected output — the real fork is in `biscuit-terminal ::detection::color::detect_color_mode`, gated by `is_tty()`. **Deviation:** PATH is set on the child command only, not the test-process env, so no repository serial guard is needed; non-macOS records a clean skip.)
- [x] Report interactive (PTY) and piped (redirected CLI) measurements **separately**. No Level 3 input-protocol test. (spec Work 2) (Interactive PTY repeated-construction latency and piped `md compose` latency are separate cases in the run record; no L3 added.)

The probe protocol lands first. The Biscuit Terminal cache proof and Darkmatter
CLI case may then use it independently without sharing a process or global
cache state.

### Checkpoint 3
Biscuit Terminal + Darkmatter CLI `just test` / `just test-l2` / `just lint`
green. The L2 artifact shows one OSC 10 request across N constructions and one
detection per `md compose`; interactive vs piped latencies recorded separately;
Windows still compiles and records the Unix-PTY skip disposition. Linux provides
the required real non-macOS L2 behavior evidence.

---

## Phase 4 — Consume the shared `ComposeOptions` classification (Architecture Decision B)

Land Architecture Decision B exactly once. The linked
[Opaque Reference Graph](../2026-07-15-reference-graph/plan.md) Phase 1 owns the
crate-private exhaustive classification, the two purpose-specific identity
products, and the `options_hash` migration. This feature depends on that shared
prerequisite and must not implement a competing inventory. **Blocks Phase 5.**

- [x] Land or merge the linked feature's shared prerequisite before this feature changes a compose reuse boundary. Record the prerequisite commit in `results.md`. (Shared classification landed in `a8e5e98d9`; guard/benches in `16ed1e57a` — both already in this branch's history. Recorded in `results.md` Phase 4 → Prerequisite provenance.)
- [x] Confirm the owning implementation destructures `ComposeOptions` **with no `..`** and derives both `ReferenceGraphOptionsIdentity` and the compose-cache fingerprint from that one field inventory. No third fingerprint or parallel field list is allowed. (AD-B) (`classify_options` — single no-`..` destructure feeds both products; grep confirms no parallel inventory.)
- [x] Confirm ordered vectors (`magic_paths`, `env_path_whitelist`, and any other order-sensitive sequences) retain order. Sort only genuinely unordered maps/sets such as `exclude_keys`, `pre_approved_commands`, allowed-host sets, and canonical context/env maps. (AD-B) (Ordered vectors encoded in order; `exclude_keys`/`pre_approved_commands`/`allowed_hosts` sorted before encoding — proven by the reorder-sensitive vs insertion-order-insensitive tests.)
- [x] Confirm the typed, length-delimited encoding distinguishes field/type boundaries, `None`, and empty values; uses a versioned domain marker; contains no `Debug` encoding; and hashes through `biscuit-hash` xxHash. Add delimiter-collision and `None`/empty regression tests. (AD-B) (`GraphIdentityEncoder`: 8-byte LE length prefixes, per-collection counts, explicit enum discriminant bytes, versioned domain, `xx_hash_bytes`. Cache product migrated off `Debug`/string-join to the same encoder under `dm.compose-cache-options.v1`. New tests: `options_identity_distinguishes_none_from_empty_collection`, `options_hash_distinguishes_none_from_empty_value`, `options_hash_magic_path_element_boundaries_are_injective`.)
- [x] Confirm process-local state participates only in run-local reuse. The run-local key distinguishes independently constructed stateful instances but remains stable across clones of the same `Arc`, without increasing strong counts. Process-local identity bytes never enter a persistent key; when they are required, persistent reads **and** writes are disabled. When equivalence cannot be established, reject reuse. (AD-B) (Weak allocation handles only; `persistent_cache_eligible()==false` for an attached `shell_approval_handler` skips **both** the persistent read at `runtime.rs:323` and write at `:354` while keeping run-local single-flight — proven by the clone-stable / fresh-instance / strong-count / eligibility tests.)
- [x] Replace or delegate `cache::hashing::options_hash` and migrate the direct producer in `transclusion/engine.rs` plus the persistent-key consumers in `cache/runtime.rs` under a new cache-key domain/version. Audit preflight-state participation through the shared classification rather than treating preflight as an `options_hash` call site. Prove a legacy persistent entry cannot be read under the new encoding. (AD-B) (`options_hash` stays a thin delegate to `compose_cache_fingerprint`; cache product moved onto the typed encoder under the new `dm.compose-cache-options.v1` domain — no `Debug` encoding remains. `engine.rs:1335` producer + `runtime.rs` `compose_entry_key` consumers pick up the new value via the delegate. Preflight audited: not an `options_hash` call site — pure path-keyed lookup; its only identity participation is the classification's `preflight_graph` weak handle. Legacy-unreadable proof: `options_hash_not_value_compatible_with_pre_migration_encoding` freezes `0x60a653c15cd5b9d1` and asserts the new value differs, so an entry keyed under the old `entry_key` can never be matched.)
- [x] Tests cover equal identities across unordered insertion order; unequal identities across ordered-vector reordering and representative scalar/collection/context/schema/transclusion/remote/shell families; clone stability; fresh-instance inequality; and persistent-cache ineligibility for process-local state. The no-`..` destructure is the field-addition guard. (All present in `options.rs`/`hashing.rs` test modules; enumerated in `results.md` Phase 4 → Evidence.)

This phase is sequential with the linked feature's provenance work: the shared
prerequisite has one owner and one landing commit. Performance-follow-up work
begins only after that commit is present.

### Checkpoint 4
Darkmatter `just test` + `just lint` green. Exactly one `ComposeOptions` field
inventory exists (the no-`..` classification); `options_hash` is gone or a thin
delegate; no `Debug`-based option encoding remains; legacy cache entries cannot
cross the new domain; and stateful keys cannot touch persistent storage.
Rendered/diagnostic behavior remains byte-identical, while cache reuse may
become conservatively narrower by design.

---

## Phase 5 — Cross-pass compose reuse (Findings 7, 16, 35.1 & 35.4) — Work 4

Finish the remaining validate/preflight/compose duplication using a cache key
whose identity contains **every** semantic input. Depends on Phase 4 (AD-B
compose-cache fingerprint).

- [x] Audit the existing transclusion key (`options_hash` + source + effective state + context + directive-overlay identities) before changing any reuse boundary. Confirm it now derives from the AD-B classification, not the retired `Debug`-based encoding. (`engine.rs:1335` `options_hash` is the Phase-4 delegate to `compose_cache_fingerprint`; grep confirms it is the only producer. Recorded in `results.md` Phase 5 → transclusion-key audit.)
- [x] Implement reuse in the spec's preferred order, stopping at the first safe level: (1) share parsed source + reference metadata; (2) share context-independent prepared representations; (3) share fully rendered content **only** if a complete semantic identity is demonstrated; (4) otherwise retain recomposition and record a same-fixture **no-win** disposition for narrower candidates. (Work 4) (Existing `get_or_compute_compose` already implements level 3 with a complete semantic identity; no safe broadening remained and no speculative reuse code was added. Recorded as F7/F16 "existing level held".)
- [x] Preserve condition-aware behavior: do not reuse bodies whose output depends on parent state, directive position, conditions, or lifecycle decisions. (Findings 7/16) (`when=` evaluated per directive; `state_hash` keys the cache — proven by `differing_parent_state_does_not_reuse_child_output`.)
- [x] The cache is run-local or bounded; retains no unrelated contexts, graphs, callbacks, or runtimes. Because transclusion composes children **concurrently**, any shared prepared-content cache is **concurrency-safe or partitioned per compose run** — no data race, no lock held across child composition. (Work 4) (`RunLocalCache` is run-local; `runtime_mutex` is locked only to `clone_for_child`/`merge_child`, never across child compose — proven by `single_flight_contention` barrier tests.)
- [x] **35.1:** compute `effective_state_hash` once per transclusion phase and thread the value through directive cache-key construction. This belongs here because Phase 5 owns that key's assembly and measurement. (`PhaseStateIdentity::capture` computed once in `pipeline/phases.rs`, threaded through `resolve_prepared_transclusion` → `render_markdown_transclusion`; equality proven by `phase_state_identity_matches_underlying_hashes`; benchmark win recorded.)
- [x] **35.4:** route `::toc-linking` graph-discovery and composition reads through the same compose-run-owned source cache without broadening persistent reuse. `ReferenceAnalysisRuntime` currently constructs its own `RunLocalCache`; two caches of the same type do not satisfy this item. Thread one owner through the production graph and transclusion paths, while preserving authoritative-read and invalidation behavior. This belongs here because it is another cross-pass reuse boundary. (`generate_toc_link_references` now reads via the run's shared `ReferenceAnalysisRuntime.cache` (`load_markdown`) instead of a second `Markdown::try_from`; composition side already used `runtime.cache.load_toc_headings`. Verified by `toc_linking_repeated_target_generates_all_heading_links`.)
- [x] Add a compose benchmark comparing immediate pre-change vs candidate on identical manifest fixtures; declare thresholds per the evidence contract. (`benchmarks/raw/f5-crosspass-reuse/run-20260716T000000/`: fixture `compose_transclusion_heavy`, baseline `b425fb466` vs candidate; byte-identical output; user CPU −8.6% out of noise, wall-clock −1.9% within σ.)
- [x] Record separate target/control dispositions for the general F7/F16 reuse, 35.1 hash hoisting, and 35.4 read reuse so an aggregate result cannot hide a regression. (Three separate dispositions in `results.md` Phase 5 + the run-record `summary.md`.)
- [x] Verification (matrix F7/F16/F35): reference, preflight, transclusion, `::toc-linking`, condition, lifecycle, source-cache, and cache-identity suites pass. Use deterministic L1 concurrency tests with barriers/timeouts to prove concurrent child progress and lock release; an ordinary child process does not make this L2. (Darkmatter `just test` (lib+cli+dmls) + `just lint` green; existing `single_flight_contention` barrier tests cover concurrency/lock-release.)

### Checkpoint 5
Darkmatter `just test` + `just lint` green. Compose/validation output remains
byte-identical. Each reuse item shows a repeatable win or a recorded no-win
disposition with speculative code removed. No lock is held across concurrent
child composition. Classify 35.4 from its filesystem implementation; do not
categorize the whole phase as OS-identical merely because its cache is
run-local.

---

## Phase 6 — Frontmatter & expression rework (Findings 11–14) — Work 5

Four separate benchmark checkpoints share one fixture set. F12 changes the
context path used by the interpolation work and lands first. F11 and F14 then
proceed as coordinated edits because both touch
`frontmatter_interpolation.rs`; F13 remains independent. Each closes on its own
baseline/candidate comparison.

### F11 — Incremental frontmatter interpolation fixpoint
- [x] In `frontmatter_interpolation.rs`, extract each templated key's dependencies **once**; maintain unresolved-dependency counts + reverse edges; enqueue newly eligible keys. Avoid rebuilding the full seed map per successful key where mutation can be incremental. Preserve cycles, shell deferral, best-effort propagation, and key-scoped errors. (`refs_by_key` computed once; `dep_count` + `dependents` reverse edges drive the sweep; one reused `FrontmatterSeedState` mutated in place — no per-key seed/context/`ResolutionContext` clone. Cycles/self-ref defer to the unchanged fallback pass; best-effort + shell-pending deferral preserved. Byte-identical; new units `wide_and_deep_graph_resolves_in_dependency_order`, `self_referential_key_terminates_and_resolves_empty`, `mutual_cycle_terminates_without_hang`.)

### F12 — Borrowed/shared `ResolutionContext`
- [x] Add an internal borrowed/shared path for evaluators and expression functions (`resolve_ctx.rs`, `functions/mod.rs::ContextFn`), retaining the owned public facade where compatibility requires it. No public owned-return API change without an approved exception. (New defaulted `EvaluationLookup::resolution_context_ref() -> Option<&ResolutionContext>`; the evaluator dispatches read-side functions via `Cow::Borrowed`, owned clone only as fallback. Public owned `resolution_context()` unchanged — no exception needed. Overridden in `ResolvingLookup`, `LayeredLookup`, the condition lookup, and `FrontmatterSeedState`.)

### F13 — Faster exact multi-pattern replacement
- [x] Benchmark an exact multi-pattern matcher in `replacement.rs` against the current canonical precedence (descending key byte length, then ascending lexical order). **Reject** any design changing left-to-right non-overlapping matching, the choice at a shared start position, non-recursive replacement output, UTF-8 character-boundary behavior, empty-key omission, or scalar-value coercion. If no win, record a requirement-matched no-win result and remove speculative code. (Aho–Corasick `LeftmostLongest` single-pass — preserves every listed contract; measured **2.371 ms → 0.087 ms (≈27×)** on the 43-rule `replace_heavy` body, byte-identical; **accepted**. `aho-corasick` added as a direct dep, recorded in `docs/dependencies.md`.)

### F14 — Reduced literal / interpolation rescans
- [x] In `interpolation/rewrite.rs` and the frontmatter literal-conversion path, reduce repeated Markdown-aware scans and full-body copies when interpolation is present; construct output once per interpolation depth where practical. Nested interpolation keeps semantic fixpoint behavior; it does **not** authorize rescanning unrelated protected ranges. Benchmark nested and no-expression cases **separately**. (Fast-path guards: `interpolate_text` skips the whole scan/`convert_literals` pipeline when the input has no `{{`; `convert_frontmatter_literals` skips when no value has `{{{`. Isolated: the skipped MarkdownAware parse **240 µs → `contains("{{")` 2.3 µs (≈104×)** on the `toc_large` body. Nested/rescan fixpoint untouched — guard triggers only when `{{` is entirely absent. Byte-identical.)

- [x] Before any Phase-6 baseline, register and hash the shared fixtures: wide dependency graphs, deep dependency chains, cycles, shell-pending keys, best-effort errors, many replacement rules, Unicode, code fences, literal escapes, multiline indentation, and nested interpolation. (spec Work 5) (`compose_interpolation_heavy` + `replace_heavy` registered/hashed in `manifest.yaml`, generator `1.1.0`→`1.2.0`, before the baseline; verified by `benchmark_fixtures.rs`. Cycles / shell-pending / best-effort errors are covered by named unit tests rather than committed compose fixtures — they resolve to warnings/raw text, not a byte-stable artifact. Recorded in `results.md` Phase 6.)
- [x] Verification (matrix F11–F14): focused units + compose integration + scale benchmarks per checkpoint. F12 can reach filesystem-backed expression functions → classify its cross-platform gate from the actual changed path, not categorically OS-identical. (Focused units per finding + end-to-end `compose_phase6.rs` over the shipped fixtures + isolated Criterion microbenchmarks (F13/F14) + whole-pipeline hyperfine control. F12's changed path is borrow-vs-clone only — no `cfg`/filesystem branch — so OS-identical from the actual diff; recorded in `results.md`.)

**Dependency order:** F12 → coordinated F11/F14. F13 may proceed independently
after the fixture set is frozen. Capture each immediate baseline before its
code change so one checkpoint cannot contaminate another's comparison.

### Checkpoint 6
All four checkpoints have either a threshold-meeting benchmark or a recorded
no-win disposition with speculative code removed. Compose output byte-identical.
Darkmatter `just test` + `just lint` green. Cross-platform disposition recorded
per checkpoint (F12 assessed from its changed path).

---

## Phase 7 — Shell polling & policy clones (Findings 17 & 32) — Work 6

OS-divergent — **requires a real non-macOS behavioral run** of the wait
primitive. Independent of Phases 4–6.

### F17 — Replace the 10 ms completion polling loops
- [x] Replace or avoid **both** independent 10 ms `try_wait`/`sleep` loops in `shell_expansion/executor.rs` (`try_wait` at `:245`/`:577`) with a blocking wait primitive or event-driven notification available on all supported OSes. Any platform split is **target-gated and tested**. (Both loops now call one shared `wait_with_timeout(&Arc<SharedChild>, Duration)`: a helper thread performs the OS-blocking `wait()` and hands the status back over a channel consumed with `recv_timeout`; the timeout arm kills+reaps through the same shared handle. **No platform split in Darkmatter code** — `shared_child` owns the `waitid`/`WaitForSingleObject` divergence. Added `shared_child = { version = "1.1", default-features = false }`; the disabled `timeout` feature would have pulled a process-wide SIGCHLD handler via `sigchld`/`signal-hook`, which a library must not install.)
- [x] Preserve concurrent stdout/stderr draining while waiting; do not replace polling with a wait path that can deadlock on a full pipe. Prove unchanged timeout boundaries/granularity, saturated dual-stream capture, descendant/process cleanup, failure/error selection, and source-order execution for both executor variants. Arbitrary directive parallelism remains prohibited. (Drain threads are still spawned **before** the wait in both variants — unchanged structure. New units: `saturated_dual_stream_capture_does_not_deadlock` (standard executor), `..._in_pipeline_executor` (`ReadStrategy::Separate`), `saturated_merged_stream_capture_does_not_deadlock` (`2>&1` single-pipe reader) — 256 KiB/stream interleaved in 8 KiB chunks, well past the 64 KiB pipe buffer; `timed_out_child_process_is_killed_and_reaped` (`cfg(unix)`, `pgrep -P` proves no surviving child); `pipeline_executor_timeout_selects_timeout_error` (error selection on the second variant); `fast_command_completion_is_not_delayed_by_a_poll_interval` (granularity regression guard). Existing timeout/source-order/report-count suites unchanged and green.)

### F32 — Snapshot shell policy once per stage
- [x] Move snapshot ownership to the stage orchestrator in `shell_expansion/mod.rs` and plumb one `ShellRuntimeSnapshot` from `shell_expansion/types.rs` through directive authorization. The matching helpers in `policy.rs` remain borrowed consumers. Do **not** hold the policy mutex across parsing, approval, or command execution. (`prepare_directive` / `execute_directive_detailed` now take `snapshot: &ShellRuntimeSnapshot`; the three stage orchestrators — `inline/shell_expansion.rs`, `shell_blocks/mod.rs`, `frontmatter_shell_expansion.rs` — each call `snapshot()` once at stage open, after `ensure_loaded`. The frontmatter path threads it alongside the existing `policy_paths` through `prepare_optional_branch`/`prepare_branch_pipeline`. `policy.rs` helpers unchanged — still borrowed consumers. The public `execute_directive` keeps its signature and opens its own single-directive snapshot, so **no public API shape change**. The mutex is held only for the clone; proven by `policy_mutex_is_not_held_across_approval`.)
- [x] Define the visibility contract explicitly: all directives admitted to one stage see its opening immutable policy snapshot; approvals/persistence produced during that stage update the runtime but become policy input only for a subsequent stage. Add tests for both halves of that contract. (Contract documented on `prepare_directive`'s rustdoc. Half 1 — `persisted_whitelist_from_one_stage_is_policy_input_for_the_next_stage`: root stage persists `prefix echo`, transcluded child's fresh snapshot sees it → 1 approval. Half 2 — `persistence_mid_stage_is_not_policy_input_for_the_same_stage`: two `echo` directives in one stage both prompt (2 approvals) while the rule is still written to disk. **Deliberate behavior change** — per-directive snapshotting previously let the second directive skip its prompt. Allow-once is exempt (arbitrated live via `reserve_allow_once`, not the snapshot) — `allow_once_still_dedupes_within_a_single_stage`.)

- [x] Verification (matrix F17/F32): cross-platform L1 process/policy tests plus timeout, stream-saturation, and cleanup tests. An ordinary spawned command is not L2; add/run L2 only if the implementation introduces a real-terminal requirement. Record Linux **and Windows** behavioral evidence for the wait primitive. (All new coverage is L1 — no real-terminal requirement introduced, so no L2 added or run. Darkmatter `just test` + `just lint` green on macOS. **Linux + Windows behavioral runs of the wait primitive deferred to Phase 11** — not executable on this macOS-only host; recorded as a gap in `results.md` alongside the F22 gap.)

**Parallelizable:** F17 (executor wait mechanism) and F32 (policy snapshot) touch
disjoint files and can proceed concurrently.

### Checkpoint 7
Shell directives still execute in source order with identical timeout/output/
cleanup/failure semantics; no mutex is held across execution. Darkmatter
`just test` + `just lint` green, plus `just test-l2` only if a terminal-specific
test was actually added. Real Linux and Windows behavioral runs of the wait
primitive are recorded.

---

## Phase 8 — Render & cleanup sub-items (Findings 23 & 25) — Work 7

Independent of Phases 4–7.

### F23 — Resolve code theme once per render snapshot
- [x] Introduce a render-scoped theme/environment snapshot at the render-tree entry point and carry it in `TerminalCodeRenderer` (and the corresponding browser options/context) so `code_theme_from_env`, surface mode, and theme selection are resolved once per render. `output/code_block.rs` continues to receive an already-resolved highlighter; it is not the environment-discovery owner. Preserve explicit per-`CodeBlock` theme overrides. Separate direct `CodeBlock`, `DarkmatterPage`, terminal, and browser render invocations must still observe environment changes allowed by the existing contract. (`TerminalCodeRenderer` gained `env_code_theme` — the `CODE_THEME`/`THEME` snapshot taken in the one private constructor every entry point funnels through — plus a `CodeSurface` memo (`RefCell`, keyed by the render context's theme-name + color-mode) and a keyless `BrowserSurface` `OnceCell` holding the effective `HtmlOptions` + theme variant. `render_terminal_code` / `render_browser_code` now consume the snapshot; `output/code_block.rs` is untouched and still receives a resolved highlighter. `theme_override` keeps first place in the chain; the builders invalidate the memo. Byte-identical.)
- [x] Serialize environment-mutating tests with the repository guard. Add a multi-block assertion proving one snapshot per render and a two-render assertion proving permitted environment changes are observed between renders. (All four new tests are `#[serial]` with `EnvVarGuard`s. Multi-block: `terminal_render_resolves_code_surface_once_per_render` / `browser_render_resolves_code_surface_once_per_render` — a 5-fence document **counts** 1 resolution + 1 env read via the `surface_probe` seam (verified to report 5 and 6 with the memo disabled), rather than inferring reuse from equal output. Two-render: `separate_terminal_renders_observe_theme_environment_change` (page path, `THEME` github→dracula) and `separate_direct_renders_observe_code_theme_environment_change` (page-less direct hook, terminal **and** browser, `CODE_THEME` github→dracula).)

### F25 — Cleanup pass fusion (profile-gated)
- [x] First profile individual cleanup passes (`cleanup/mod.rs`, placeholder + line passes, `reflow.rs`) on representative documents. Combine line passes **only** when ordering and boundary behavior can be made exactly equivalent; preserve exact pass ordering and canonical output. A same-fixture no-win (fusion within noise, or added allocation/complexity without a repeatable end-to-end gain) is an acceptable disposition. (**Profiled first, then closed as a recorded no-win — not implemented.** A temporary in-crate harness replicated `cleanup_content_internal` step by step over `toc_large` / `replace_heavy` / `render_code_heavy` (release, 3 warm-ups, 25 samples, median); raw output retained in the run record, harness deleted. On the largest fixture all seven stage-2 line passes are ≈282 µs = **22.3%** of a 1262 µs cleanup, and three of them carry it all (`normalize_list_spacing` 101.8, `fix_blockquote_formatting` 104.1, `fix_list_indentation` 62.3 µs); fusion cannot remove their per-line work, only the repeated scan/rebuild overhead — under ~7% of cleanup, ≈0.5% of a ~19 ms ± 0.5 ms compose, i.e. below both σ and this checkpoint's measured ~0.6% build drift. The passes are also sequential re-lining rewrites, so "exactly equivalent" fusion is not cheaply available; and GitNexus upstream impact on `cleanup_content_internal` is **HIGH** (35 impacted, 9 direct), which the plan says to stop on. Pass order and canonical output unchanged; no speculative code written or retained.)

- [x] Verification (matrix F23/F25): snapshot/golden output; headless browser computed-style/markup tests for F23; L2 terminal frames only where a real terminal is required; code-heavy render + cleanup benchmarks over manifest fixtures. (Darkmatter `just test` green — 5712 lib + 559 cli + 566 dmls, 0 failures, including the untouched render/cleanup golden + snapshot suites; `just test-browser` green — 104 headless tests; `just lint` clean. **No L2 added or run:** neither F23 nor F25 introduces a real-terminal requirement — F23's terminal evidence is the counted in-process render, and the existing L2 render-tree suite is unchanged. Benchmarks over manifest fixtures: `benchmarks/raw/f23f25-render-cleanup/run-20260716T120000/` — F23 Criterion target/control on `render_code_heavy` (baseline `b425fb466` vs candidate, identical harness bytes) and the F25 cleanup pass profile.)

**Parallelizable:** F23 (render theme snapshot) and F25 (cleanup profiling) are
independent.

### Checkpoint 8
Terminal + browser render output and cleanup canonical output byte-identical
(existing snapshots pass untouched). F23 resolves theme once per render; F25 has
a threshold-meeting fusion or a recorded no-win. Darkmatter `just test` +
`just test-browser` + `just lint` green, plus `just test-l2` only for any
real-terminal assertion. Classify cross-platform behavior from the final
environment/terminal implementation rather than predeclaring the phase
OS-identical.

---

## Phase 9 — Remote discovery line positions (Finding 33) — Work 9

Independent of Phases 4–8.

- [x] Retain the cheap no-HTTP guard in `remote.rs`. For documents that **do** contain remote expressions, replace the per-expression `byte_offset_to_line` prefix rescan (`:311` loop) with **one forward pass** or a shared offset table (reuse the TOC-style `newline_offset_table` approach). (Guard retained verbatim as the first statement — `no_http_guard_short_circuits_before_expression_scan` pins it. `byte_offset_to_line` is **deleted**, replaced by a `newline_offset_table` built once per document plus a `partition_point` lookup. A second early return skips the table (and the `PathBuf` clone) when the document parses no expression at all. The TOC-style table is reused as a *technique*, not the symbol: the TOC's same-named `line_at_offset` adds a `str::lines` trailing-newline adjustment, whereas remote discovery reports the line the `{{` sits on — a plain newline count. Sharing that lookup would have silently changed line numbers, so remote.rs keeps its own two private helpers and the divergence is documented on `line_at_offset`.)
- [x] Verify byte offsets at LF, CRLF, Unicode, start/end-of-file, and multiple expressions on one line. (`expression_lines_with_lf_newlines`, `expression_lines_with_crlf_newlines`, `expression_line_after_multibyte_unicode` + `expression_line_between_multibyte_lines`, `expression_at_start_of_file_is_line_one`, `expression_at_end_of_file_with_and_without_trailing_newline`, `multiple_expressions_on_one_line_share_a_line_number`, plus `consecutive_blank_lines_do_not_skew_line_numbers`, `empty_newline_table_reports_line_one`, and the exhaustive `line_at_offset_matches_naive_at_every_offset`. **Mutation-tested:** an off-by-one (`pos <= offset`) and a char-index table are each caught. The first Unicode test drafted was *not* discriminating — the expression sat after every newline, so byte-vs-char indices coincided and the mutant passed; it was rewritten with a 630-byte/210-char prefix and trailing lines so it now fails the mutant.)
- [x] Benchmark a remote-heavy input (immediate pre-change vs candidate, identical bytes). (`remote_heavy` fixture — 300 expressions / 79028 bytes / xxHash `0dc952a78995bde7` — registered and hashed in `manifest.yaml` (generator 1.2.0 → 1.3.0) **before** the baseline was captured. Target **2.3944 ms → 419.95 µs (−82.5 %)** vs a declared ≥ 30 % floor. Run record: `benchmarks/raw/f33-remote-discovery/run-20260716T140000/`.)
- [x] Verification (matrix F33): focused behavior tests + one target/control benchmark. (Two controls, both investigated rather than waved through: they moved −19 % reproducibly — control 2 is a genuine PathBuf-clone elision, control 1 is build-layout drift on unchanged code. Discounting the whole shift, the target's code-specific win is still ≈ −78 %; no control regressed. Corpus test `remote_discovery_line_positions_match_fixture_text` proves every discovered URL appears on its reported line across all 13 shipped fixtures. Darkmatter `just test` + `just lint` green. **OS-identical** from the actual diff — pure byte/line scan, no filesystem, URL-runtime, or `cfg` path changed.)

### Checkpoint 9
Remote-URL discovery produces identical line positions on all edge cases;
remote-heavy benchmark meets threshold or records a no-win. Darkmatter
`just test` + `just lint` green. Record the actual diff's cross-platform
classification; a pure byte/line scan may use the OS-identical disposition only
when no filesystem, URL-runtime, or `cfg`-specific path changed.

---

## Phase 10 — Remaining Finding 35 residual sub-items — Work 10

Five residual sub-items remain after 35.1 and 35.4 move into Phase 5. Each needs
its **own** behavioral tests and measurement disposition — a single aggregate
benchmark may **not** conceal a no-win or regression in an individual path.
Capture immediate baselines sequentially even where implementation files are
disjoint.

- [x] **35.2** In `relevel_with_overflow`, compute heading line positions in one forward pass and apply all heading edits with one output construction rather than copying the whole child for every replacement (`transclusion/engine.rs:76`, `:138`, `:181`). Preserve byte-identical output, overflow-warning lines/order, and the unchanged/zero-adjustment fast paths. (`extract_headings` builds one **deferred** `newline_offset_table` — never built for a heading-free document — and looks each heading's line up with `partition_point`; `relevel_with_overflow` assembles output in one ascending forward pass over non-overlapping heading spans instead of rebuilding the whole document per replacement. The `newline_offset_table`/`line_at_offset` pair moved from `toc/mod.rs` to `markdown/span.rs` as `pub(crate)` so this is the second consumer of one helper, not a third copy (`remote.rs` keeps its deliberately divergent plain-newline-count variant per Phase 9). **Warning-order gotcha:** the old descending rebuild emitted overflow warnings in *reverse* document order; the forward pass collects them ascending, so `warnings.reverse()` restores the observed contract — pinned by `overflow_warnings_stay_in_reverse_document_order`. Measured **25.351 ms → 314.93 µs (−98.8 %, ≈80×)** prefix, **24.347 ms → 361.50 µs (−98.5 %)** overflow, **18.463 ms → 277.80 µs (−98.5 %)** extract-only; heading-free control at parity (230.59 → 220.62 µs). Byte-identical, proven differentially against a pinned copy of the pre-change algorithm over 16 shaped cases × 6 target levels **and** all 13 shipped fixtures; both an order mutant and a line-table mutant are caught.)
- [x] **35.3** Store fetched response bodies as `Arc<str>` internally, preserving the owned `get_content` facade where required (`remote_fetch.rs:38` `FetchSlot::Ready`, clone at `:447`, and the `cache/remote_cache.rs` outcome handoff; `Arc` already imported). (**Implemented, measured, then closed as a recorded NO-WIN and reverted** per the evidence contract. `FetchSlot::Ready` is populated by *moving* `RemoteFetchOutcome.body` (a `String` from `String::from_utf8`); `Arc<str>` cannot reuse that allocation, so it **adds** one full body copy per URL (**+1.167 µs**) that the old code never paid. The owned `get_content` facade must keep returning `String`, and `Arc<str>::to_string` is *slower* than `String::clone` (0.791 vs **0.667 µs**), so all four owned consumers (`::file`, preflight, resolve_ctx ×2) regress unconditionally; only `::code` (`wrap_in_code_block(&body, ..)`, `&str`-only) can take the refcount bump (−0.667 µs) via a crate-internal shared accessor. Net per typical one-URL/one-consumer document: **+1.29 µs worse** (`::file`), **+0.50 µs worse** (`::code`); `::code` only breaks even at ≈2 consumers of the same URL. And the whole copy budget is **0.125 %** of a *loopback* fetch (0.667 µs of 534.5 µs) — the most favorable case possible — versus a declared ≥5 % floor. The `Arc<str>` slot, the shared accessor, and the `::code` call-site change were all reverted; `remote_fetch.rs` is byte-identical to its pre-phase state and the temporary harness was deleted. Evidence: `benchmarks/raw/f35-residuals/run-20260716T160000/f35_3-copy-cost-model.txt`. Cross-platform: OS-identical from the inspected path (slot storage + in-memory copy; no `cfg`, no filesystem branch, Tokio/reqwest untouched) — and moot, since nothing shipped.)
- [x] **35.5** Within each mutually exclusive `md hash --diff` or `--save` invocation, compute each unique `(kind, effective MdHashOptions)` artifact once and pass it through comparison/planning and explanation output. Preserve `--save`'s legitimate distinction between the stored ignore-policy comparison and the selected current-policy baseline; cache by semantic hash identity rather than assuming one artifact can serve both. Do not change stored hash semantics (`cli/commands/hash.rs`; lib `compare_hash`/`explain_hash_diff`/`plan_hash_save`/`apply_hash_save`). (**Accepted, with a recorded residual.** A `detailed` `--diff` hashed the document **three times**: `compare_hash`, then `explain_hash_diff`'s own `compare_hash`, then `detailed_body`'s third recompute. New `compare_options` (names the like-for-like identity) + `compare_with_computed` (takes an already-computed artifact) let `explain_hash_diff` compute **one** artifact and share it with both the comparison and `detailed_body` — provably the same artifact, since `detailed_body` is only reachable via `ComparisonDetail::Detailed`, which only arises when `stored.kind == Detailed`. `plan_hash_save` computes the comparison artifact once and reuses it for the baseline **only** when `(selected kind, normalized ignore-set)` matches the stored identity — a kind change or ignore-policy change still computes the current-policy baseline separately, so the two `--save` artifacts are **never** conflated (identity tested, not assumed). Measured: library `--diff` sequence **7641.9 → 5399.6 µs (−29.3 %)** on `toc_large`/detailed, 19.1 → 13.0 µs (−32.0 %) on `hash_basic`/detailed; CLI hyperfine **17.2 ms ± 0.5 → 14.1 ms ± 0.7 (−18.0 %, ≈4σ)** on `md hash --diff` for a large detailed doc, vs a declared ≥5 % floor — and both `simple` and small-doc controls stayed within σ (0 % regression budget held). Stored hash semantics unchanged, proven by a write→read→re-save round trip across 5 kinds × 2 ignore policies; the conflation mutant (`baseline_is_compare_artifact = true`) is caught by 2 tests. **Recorded residual:** `--diff` still computes twice (CLI `compare_hash` for exit-2 + `explain_hash_diff`), because `ExplanationBody`/`FmConcern`/`StructuredBody`/`DetailedBody` are all private and closing it needs either a new public `HashExplanation` accessor (barred by the no-new-public-API contract) or an interior-mutability memo on `Markdown` (a `Clone`/`PartialEq` value shared across rayon threads in `run_hash_directory` — Sync + staleness hazard, disproportionate to ~2.3 ms). This is why the `simple`/`structured` rows are unchanged. Evidence: `benchmarks/raw/f35-residuals/run-20260716T160000/f35_5-hash-artifact-profile.txt`. **OS-identical** from the actual diff — pure call-graph restructuring, no `cfg`/filesystem/clock path changed.)
- [x] **35.6** Make `normalize_body_rhythm` avoid allocating an ANSI-stripped string for every output-line check (`layout/page.rs:1423`). (**Accepted.** `strip_escape_codes` takes `Into<String>`, so the predicate allocated **twice per output line** — an owned copy of the line, then the regex output. It now drives the same canonical `ANSI_ESCAPE_RE` directly over the borrowed `&str`, whose `replace_all` returns a `Cow` that *borrows* when the line carries no escape code; and the `\x1b[48` background-fill test hoists ahead of the strip, deciding every filled row without touching the regex (`&&` over two pure operands, so the reorder is result-preserving). Measured interleaved in-process: decorated prose **164.8 → 14.8 µs (−91.1 %)**, code panel **133.7 → 9.0 µs (−93.3 %)**, escape-free control **28.4 → 19.8 µs (−30.3 %, no regression)** vs a declared ≥10 % floor. End-to-end the pass fell from ≈22.5 % of a decorated `DarkmatterPage::render` to **2.55 %**, making that render ≈20 % faster. Byte-identical, proven differentially against the pre-change predicate over 19 line shapes and **all 361 adjacent pairs** (blank-run collapsing + trailing-blank stripping), plus the harness's own per-body equivalence gate. Evidence: `benchmarks/raw/f35-residuals/run-20260716T160000/f35_6-rhythm-profile.txt`. **OS-identical** confirmed from the actual diff — pure in-memory regex predicate, no `cfg`/filesystem/terminal-I/O path changed.)
- [x] **35.7** Borrow link/image URL + title data through `render_tree/build_context.rs::apply_link_policy` and `apply_image_policy`, including the **empty-policy fast path**, while retaining owned public `RenderNode` output. Do not redirect this work to compose-time link normalization or Markdown image-literal escaping; those are different paths. (**Accepted.** Both appliers cloned URL **and** title out of every link/image node before deciding anything — on the empty-policy path those clones were the *only* work done. They now borrow out of `node.kind` and resolve each decision (colors, `CommonStyle`, parsed directive) into an owned value inside one scope, so the borrow ends before the node is mutated; application order is unchanged and the directive parsers are pure, so computing them earlier is equivalent. Owned public `RenderNode` output retained; compose-time link normalization and image-literal escaping untouched. Measured interleaved over 1000 link nodes: **empty policy/no title 72.6 → 58.2 µs (−19.9 %)**, empty/with-title 291.8 → 256.5 µs (−12.1 %), hyperlink policy/no title 97.0 → 89.3 µs (−7.9 %), hyperlink/with-title 974.0 → 937.6 µs (−3.7 %) vs a declared ≥5 % floor on the empty-policy target and a 0 % control-regression budget — **every shape improved, none regressed**. Honest scope: 14.4 µs on 1000 links is only ≈0.44 % of a 3237 µs `as_terminal(toc_large)` render, so this is retained on its **target-operation** win, and because it is a *strict* improvement with no added complexity (two clones removed, no new state or branch) — unlike 35.3, which was rejected as a net pessimization. Byte-identical, proven differentially against the pre-change appliers over 14 URL/title shapes × 5 context shapes for links **and** images, plus a non-link/image no-mutation case. Evidence: `benchmarks/raw/f35-residuals/run-20260716T160000/f35_7-link-policy-profile.txt`. **OS-identical** from the actual diff — pure borrow-vs-clone in two in-memory appliers.)
- [x] Per sub-item: behavioral tests + one target/control benchmark, each with its own disposition in `results.md` (implementation win or recorded no-win with code removed). (Five separate dispositions in `results.md` Phase 10 + `benchmarks/raw/f35-residuals/run-20260716T160000/summary.md`, each on its own row with its own declared threshold — no aggregate number. 35.3's rejection is visible on its own row rather than absorbed into a phase total, and its code **was** removed (`remote_fetch.rs` byte-identical to its pre-phase state). Behavioral tests are differential against a pinned copy of each pre-change algorithm: `engine::tests::finding_35_2` (16 shaped cases × 6 levels + all 13 shipped fixtures), `page::tests::finding_35_6` (19 line shapes + all 361 adjacent pairs), `build_context::finding_35_7` (14 URL/title × 5 context shapes, links **and** images), `save::finding_35_5` (identity-divergence + round trip across 5 kinds × 2 policies). Mutation-checked: the 35.2 warning-order and line-table mutants and the 35.5 artifact-conflation mutant are each caught.)
- [x] Cross-platform classification per sub-item. Do not preclassify 35.3 as OS-identical until its remote/runtime path is inspected; the other allocation/hashing changes still require confirmation from the actual diff. (All five classified **OS-identical from the shipped diff**, individually justified in the run record. 35.3 was **not** preclassified — its remote path was inspected first (`FetchSlot`/`get_content`/`RemoteFetchOutcome` are slot storage + an in-memory copy; the Tokio/reqwest runtime was never touched), and the classification is moot anyway since nothing shipped. 35.5 confirmed to touch no filesystem/path/clock code (the CLI still owns `fs::write`); 35.6's regex is the same shared `ANSI_ESCAPE_RE` static on every platform; 35.2/35.7 are pure in-memory scanning/borrowing. **No Phase-10 sub-item adds an OS-divergent path**, so this phase adds no new Linux/Windows behavioral-run obligation to Phase 11.)

35.3, 35.5, 35.6, and 35.7 have disjoint primary files; 35.2 follows Phase 5's
transclusion edits. Even when implementation work is independent, baseline and
candidate capture remains one checkpoint at a time.

### Checkpoint 10 — MET
Every sub-item has an individual benchmark disposition (no aggregate masking).
Compose/CLI output byte-identical; `Arc<str>` and borrowing changes preserve
owned public facades. Darkmatter `just test` + `just lint` green; run
`just test-l2` only if a remaining item adds or changes real-terminal behavior.

---

## Phase 11 — Documentation, cumulative closeout, cross-platform evidence, final gates

- [x] Add a **dated correction/supersession notice** to the old plan/results (`../../reviews/2026-07-12-perf/`), linking to this feature's audit and final dispositions. Do **not** rewrite their original body or checkboxes — they remain the historical `codex/default` record. (AD-A, Documentation Deliverables) (Five notices, each tailored to what that document actually got wrong: `spec.md` (audit wins on disagreement; F1+F22 reverted), `plan.md` (a checked box means the step *ran*, not that the finding closed), `results.md` (superseded **as a gate** — non-comparable fixture bytes), `baseline.md` (recorded sizes not bytes/hashes → not reproducible), and `results-2.md` (**still current** — F29 sustained, ownership exception preserved). No original body text or checkbox altered.)
- [x] Link the original review to this active follow-up **and** to the opaque graph feature. (Every notice links to this feature's `results.md` + `spec.md` audit table **and** to `../2026-07-15-reference-graph/plan.md` for Finding 18.)
- [x] Confirm `results.md` records one disposition + evidence location for every retained partial/open/correction item: Findings 1–4, 7, 11–14, 16, 17, 21–23, 25, 32, 33, and all seven Finding-35 items. (Confirmed: F1/F22 Phase 1; F4 Phase 2; F2/F3/F21 Phase 3; F7/F16 + 35.1/35.4 Phase 5; F11–F14 Phase 6; F17/F32 Phase 7; F23/F25 Phase 8; F33 Phase 9; 35.2/35.3/35.5/35.6/35.7 Phase 10. All 17 findings + all 7 F35 sub-items, each with disposition, evidence location, and cross-platform classification.)
- [x] Document the restored Sniff and directory-hash compatibility behavior (rustdoc + README where behavior/supported construction changed). (Rustdoc for both was already aligned in Phase 1 and verified accurate here. Added the **missing** `sniff/lib/README.md` statement — the README described NTP detection without ever saying the bare `detect_timezone()` is what pays for it (seconds; ~10 s on Linux) or that `detect_timezone_with_options(false)` opts out. F22 needed no README: `collect_markdown_files`' rustdoc + `docs/cli/hash.md` "Directory input" both already state the restored membership, and no darkmatter README covers directory hashing.)
- [x] Update the audit table + `results.md` so every finding reflects its final honest disposition. (`spec.md`'s table gained a **Final (2026-07-16)** column for all 35 rows + a *Final totals* section; the original `Status`/`Work retained here` columns are preserved as the audit-at-`51c1f16e1` record that scoped the feature, with `Final` superseding on disagreement. `results.md` gained a full Phase 11 section + an *Open at closeout* list.)
- [x] Update the darkmatter skill (`.claude/skills/darkmatter/`) if any architecture/workflow changed; regenerate the skill `hash:` with `md hash <file>`. (**No skill change needed** — and none made, so no `hash:` regeneration. `compose.md` was already updated in Phase 7 for the stage-snapshot contract; Phases 8–11 changed no architecture or workflow the skill documents (no public API change, and Phase 10's `newline_offset_table`/`line_at_offset` move to `markdown/span.rs` is `pub(crate)`, not re-exported).)
- [x] Update `darkmatter/docs/dependencies.md` (and per-area deps doc) if any crate was added/removed. (Already current — `aho-corasick` (F13, Phase 6) and `shared_child` (F17, Phase 7) are both recorded with their rationale. **No crate added or removed in Phase 11.**)
- [x] **Cumulative closeout run:** run the **complete manifest** against the final feature head so the cumulative result includes every follow-up change (distinct from Phase 2's historical `83aaecc8f`→`51c1f16e…` reconstruction). (Run record `benchmarks/raw/f-cumulative-closeout/run-20260716T050518/`; contract declared **before** capture. All 13 fixtures + `md --help`, with `83aaecc8f` and `51c1f16e1` rebuilt and **interleaved in the same `hyperfine` invocation** as the head so all three pins share conditions, in a deliberate low-load window. **Cumulative claim PASS** — `toc_large` 148.30 → 8.80 ms (**−94.1 %**) vs a declared ≥90 % floor; every case at or better than pre-opt; controls flat, so no build-drift caveat. **16/16 byte-identical** output clean-head vs working tree across compose/render×3 targets/toc/hash. ⚠ The **regression gate failed** — 4 compose cases +14–35 % vs the audit commit — and was **investigated, not narrated**: `audit→head` contains only two *code* commits, **both from the reference-graph feature** (`a8e5e98d9`, `16ed1e57a`). Splitting the interval proves this follow-up's own diff is **flat or improving** (−5.0 % to +0.2 %) and the entire regression is theirs. `--perf` localizes it to Command Setup (`validate references` 3.6→6.9 ms, `build options` 4.0→7.4 ms), not the compose pipeline (807→833 µs). Reported to the owner; **deliberately not fixed** — Finding 18 is out of scope by charter.)
- [x] **Cross-platform evidence:** record Linux behavior for the Unix PTY helper and Windows compilation + clean skip/unsupported behavior; record Linux **and Windows behavioral runs** for F17's wait primitive and F22's directory/path CLI case; use Windows compile + macOS behavior + ordinary Linux CI only for findings demonstrated OS-identical by their final diff. macOS-only success is insufficient. (**Mostly closed — the "macOS-only host" premise the earlier phases deferred on was wrong.** A real Linux kernel was reachable (Docker `Linux 6.12.76-linuxkit aarch64`, Debian 13) and the `x86_64-pc-windows-gnu` target was installed, so the gaps were **executed, not carried**. Evidence: `benchmarks/raw/f-cumulative-closeout/run-20260716T050518/linux-behavioral-run{,-2}.txt`. **Linux behavioral — ALL PASS:** F2/F21 PTY helper **2/2** under a real PTY (gap closed); **F17 6/6** — the highest-value result, since `shared_child`'s wait is `waitid` on Unix vs `WaitForSingleObject` on Windows, so Linux is a genuinely different primitive, and all three saturation tests, the kill+reap arm, and the no-poll-loop guard hold; **F22 15/15 CLI + lib unit** — Linux agrees with macOS on vendored membership; **F1 2/2**. **Windows compile — PASS** for `darkmatter`/`darkmatter-cli`/`sniff`, and with `--tests` for `biscuit-terminal`/`sniff`, which compiles the target-gated test code; both clean-skip arms (`cfg(not(unix))`, `cfg(not(target_os = "macos"))`) exist and compile. ⚠ **Windows *behavioral* runs for F17 + F22 remain OPEN** — a cross-compile is not a behavioral run and no Windows host is reachable; recorded in *Open at closeout* with a close recipe and a now-**low** residual-risk assessment. Also surfaced: **sniff's `integration.rs` does not compile on Linux** (stale `detect_linux_package_managers` arity inside a `cfg(target_os = "linux")` test — invisible from macOS); pre-existing and unrelated (this feature's sniff diff is `os/time.rs` + `README.md`), reported to the owner.)
- [x] Final targeted gate matrix: `just build` + `just test` + `just lint` in every affected area selected by impact analysis; `just test-l2` only in Biscuit Terminal/Darkmatter areas containing the F2/F3/F21 PTY tests; Darkmatter `just test-browser` for F23; exact root selectors where supported for the affected Sniff, Darkmatter, and Biscuit Terminal packages/areas; `cargo fmt --check`; `git diff --check`. Do not run a workspace-wide Cargo build/check and do not invoke L2 directly through Cargo/Nextest. (Full matrix in `results.md` Phase 11. **darkmatter** build ✅ / test ✅ **6867 passed 0 failed** / lint ✅ / test-browser ✅ **104 passed**. **sniff** build+lint ✅ / test ⚠ 1334/1335 (pre-existing `detect_area_errors_when_not_in_repo` timeout). **biscuit-terminal** build+lint ✅ / test ⚠ 2616/2617 (pre-existing `layout_matrix_snapshots` — **verified** to fail identically on clean `b425fb466`) / **test-l2 ✅ 2 lib + 76 cli** — which required first **fixing a real defect**: the recipe ran `{{ CLI }}` only, so Phase 3's F2 PTY test (the *only* `level2_` test in the library package) was never gated. `git diff --check` ✅ clean. `cargo fmt --check` read-only: 2350 hunks, but **main itself reports 2241** and clean HEAD 2294 — pre-existing local-rustfmt drift per `CLAUDE.md`'s "main is the formatting authority"; no write-mode formatter run. No workspace-wide Cargo build/check; L2 only via `just test-l2`.)
- [x] Run GitNexus `detect_changes({scope: "compare", base_ref: "main"})` before any commit; confirm the blast radius is confined to the expected compose/cache/shell/render/hash/CLI + Sniff-timezone + terminal-OSC scope. (Run. `base_ref: "main"` reports 237 files / `critical`, but that interval is the **whole branch** including unrelated features (claudine argv, opencode stream, rendezvous) — not a usable scope signal. Re-scoped to this feature's own diff (`scope: "all"`): 56 files, and the code files are **exactly** the expected radius — compose/cache (`context/*`, `frontmatter_interpolation`, `interpolation/rewrite`, `expression`, `replacement`, `conditions`, `subtree`, `remote`, `pipeline/phases`, `transclusion/engine`, `capture/datetime`), shell (`shell_expansion/*`, `shell_blocks`, `inline/shell_expansion`, `frontmatter_shell_expansion`), render (`code_renderer`, `build_context`, `layout/page`), hash/CLI (`hash/{compare,explain,save}`, `markdown/fs`, `cli/tests/hash_directory`), Sniff-timezone (`sniff/lib/src/os/time.rs`), terminal-OSC (`discovery_probe.rs`, `tests/common/pty.rs`), plus `reference/{errors,graph,provenance}` + `markdown/span.rs` from Phase 4/5/10's documented graph + helper work. **No unrelated package.** The `critical` rating is symbol fan-out, not an unexpected file; disclosed rather than absorbed, per the Phase 9 precedent.)

### Final acceptance (maps to spec Acceptance Criteria 1–8)
- [x] Findings 1–4 and 21's compatibility/evidence gaps are closed. (F1 corrected with an injectable seam on **both** sides of the crate boundary; F22 reverted; F4 reconstructed on identical hashed bytes (`toc_large` 488→23 ms) and re-confirmed cumulatively (−94.1 %); F2 proven by a **counted** single OSC 10 across 3 constructions in a dedicated child process — on macOS **and real Linux** — with its gate defect fixed; F3 by **counted** detection events = 1 across the verbose/perf/warning branches; F21 by a PATH-shim sentinel proving no `defaults` fork on redirected output.)
- [x] Findings 7, 11–14, 16, 17, 23, 25, 32, 33, and every Finding-35 sub-item has an implementation or an allowed evidence-backed disposition. (11 implemented — F1/11/12/13/14/17/23/32/33 + 35.1/35.2/35.4/35.5/35.6/35.7; 3 evidence-backed no-wins/no-ops — F25 (profiled, not implemented), 35.3 (implemented, measured, **reverted**), F7/F16 (existing reuse already safe, nothing added so nothing to remove). Each carries its own threshold and run record; no aggregate masks a sub-item.)
- [x] Finding 22's membership change is reverted (no unapproved exception). (`SKIPPED_VENDOR_DIRS` deleted; only dot-prefixed dirs pruned. Frozen by an **end-to-end CLI** test over a tree containing `node_modules`/`target`/`vendor` (aggregate + diagnostics + exit status) plus a lib unit. No migration needed — never released; a future opt-in policy would need owner approval + migration semantics, recorded in `results.md`.)
- [x] No Finding 18 correctness work landed here; the opaque graph feature owns it with no duplication/conflict. (Confirmed — this feature only **consumes** that feature's single `classify_options` authority (commit `a8e5e98d9`); no competing inventory exists (grep-confirmed). The cumulative closeout independently **corroborates** the boundary: the only two code commits in `audit..HEAD` are that feature's, and their compose cost is reported to its owner rather than tuned here.)
- [x] Reproducible same-byte benchmark artifacts meet predeclared thresholds with raw samples retained. (Every checkpoint declares its contract before capture and retains raw samples under `benchmarks/raw/<checkpoint>/<run-id>/`; fixture identity is frozen in `manifest.yaml` and machine-verified by `benchmark_fixtures.rs`. Thresholds met where claimed (F13 ≈27×, F14 ≈104×, F33 −82.5 %, 35.2 −98.8 %, 35.6 −91.1 %, 35.5 −18.0 % CLI, cumulative `toc_large` −94.1 %); where not met, the honest disposition is recorded instead (F23 "contract satisfied, no speed-up"; F25/35.3 no-win). Control movement is investigated, never banked (F33's −19 % drift discounted; Phase 10 switched to interleaved in-process sampling after proving cross-run Criterion unsound at load ~29).)
- [x] Behavioral, L1, requirement-matched L2, headless Browser, lint, workspace, formatting-check, and whitespace gates pass, with Linux and Windows evidence recorded. (See the gate matrix above. **Partially met — recorded honestly, not asserted:** all macOS gates green except two **verified pre-existing** failures (sniff `detect_area` timeout; biscuit-terminal `layout_matrix_snapshots`, proven to fail identically on clean `b425fb466`). **Linux:** F2's PTY helper **passes under a real Linux kernel** (gap closed). **Windows:** compilation — including target-gated test code and both clean-skip arms — passes for all four packages. ⚠ **Windows *behavioral* runs for F17 and F22 remain OPEN**; a cross-compile is not a behavioral run and no Windows host is reachable from this session. Recorded in *Open at closeout*, not waved through.)
- [x] The audit table and original review documentation reflect every finding's final honest disposition. (Audit table has a **Final (2026-07-16)** column for all 35 rows + final totals; five dated supersession notices added to the original review, each specific to what that document got wrong, with no original body or checkbox rewritten.)
- [x] Architecture Decisions A and B are implemented: immutable fixture identity and dated run records remain feature-local behind focused runners; graph provenance and compose caching derive purpose-specific identities from the one exhaustive `ComposeOptions` field classification owned by the linked prerequisite. (**AD-A:** everything is feature-local under `benchmarks/` — `manifest.yaml` is the sole fixture-identity authority (byte size, structural counts, Darkmatter fm/body hashes, `biscuit-hash` xxHash), `generate.sh` reproduces every fixture byte-for-byte, and per-run facts live only in dated `raw/<checkpoint>/<run-id>/` records; the three runners stay focused (Criterion / release-CLI+hyperfine / the extended PTY probe) and CLI+PTY evidence was **not** forced through `just bench`. Every later fixture was registered + hashed **before** its checkpoint's baseline. **AD-B:** `classify_options` destructures every field **without `..`** (a new field is a compile error until its treatment is chosen), and both products — `ReferenceGraphOptionsIdentity::capture` and `compose_cache_fingerprint` — derive from that one destructure; the historical `Debug`/string-join encoding is **deleted** in favour of a typed, length-delimited, domain-seeded encoder, with legacy persistent entries provably unreadable across the new domain.)
