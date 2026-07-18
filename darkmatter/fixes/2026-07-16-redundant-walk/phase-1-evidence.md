---
created: 2026-07-16
phase: 1
purpose: scratch evidence for results.md (Phase 5)
---

# Phase 1 Evidence — Pre-Implementation Baseline

## 1.1 Worktree / Toolchain State (pre-fix)

| Item | Value |
|---|---|
| `git rev-parse HEAD` | `d672388dd0fed4196295e7f21514cac6fa59f0ae` |
| `git status --porcelain` | ` M CLAUDE.md` (single dirty path; no untracked files) |

Mid-phase drift (recorded at Checkpoint 1 verification): a concurrent session
added edits after the 1.1 capture — `M darkmatter/features/2026-07-15-performance-followup/{performance-compliance.md,review-8.md,spec.md}`
and untracked `review-9.md` in the same directory. All are Markdown docs; no
Rust source, manifest, or benchmark input changed. The fix's own
`plan.md` checkoffs and this evidence file account for the remaining delta.
Benchmark-affecting unrelated inputs stayed unchanged through the baseline
capture (bench xxHash re-verified identical after all runs).
| `rustc --version` | `rustc 1.96.0 (ac68faa20 2026-05-25)` |
| `cargo --version` | `cargo 1.96.0 (30a34c682 2026-05-25)` |
| OS / arch (`uname -mrs`) | `Darwin 25.5.0 arm64` |
| Bench source xxHash (`bh --file darkmatter/lib/benches/reference_graph.rs`) | `10346693882688625444` |
| Hardware | 16 cores, 128 GiB RAM (`sysctl hw.ncpu hw.memsize`) |

Plan note vs reality: the plan anticipated a broadly dirty worktree; the actual
pre-fix porcelain state is a single modified path (`CLAUDE.md`). All
benchmark-affecting inputs are clean. The bench source hash was re-verified
after the baseline runs and is unchanged.

## 1.2 GitNexus Impact Refresh (upstream, repo = this worktree)

| Symbol | Spec-recorded | Refreshed | Depth-1 callers reviewed |
|---|---|---|---|
| `validate` (validate.rs) | HIGH, 17 / 15 direct | **HIGH**, 17 impacted / 15 direct | All 15 direct callers are unit tests inside `validate.rs` (`validate_*`, `report_counts_correct`, `validate_with_graph_matches_validate_with_fragments`). No production caller at d=1 beyond the known `Markdown::validate_references` path. Matches spec; HIGH reported to user. |
| `validate_with_graph` | LOW, 3 direct | **LOW**, 3 direct | Matches spec. |
| `verify_graph_compatibility` | LOW, 1 direct | **LOW**, 1 direct | Matches spec. |
| `validate_references` (Markdown method) | LOW | **LOW**, 0 direct upstream callers | Ambiguous name resolved to `Markdown.validate_references` in `reference/mod.rs:534`. |
| `validate_references_with_graph` | LOW, 0 upstream | **LOW**, 0 direct | Matches spec. |
| `FileTree::ensure_built` | MEDIUM, 13 direct | **MEDIUM**, 5 direct | All 5 direct callers are file-tree unit tests (`from_markdown_builds_and_renders`, `ensure_built_is_idempotent`, `follow_transclusions_invalidates_model`, `validation_report_none_when_not_enabled`, `file_tree_from_real_file`). Count differs from spec (5 vs 13, index drift); risk level matches and no production caller outside the reference subtree appeared. |

Stop conditions evaluated: no production caller that cannot prove the fresh
invariant appeared; no public/cross-area effect beyond the recorded scope.
Proceed.

`detect_changes` before-snapshot (scope `all`, worktree-scoped): 1 changed
symbol — `Section:CLAUDE.md:L138:GitNexus — Code Intelligence` (touched), 0
affected processes, risk low. Any later unstaged delta is meaningful only
relative to this snapshot.

## 1.3 Baseline Capture (task command)

```
cargo bench -p darkmatter --bench reference_graph -- build_and_validate \
  --save-baseline redundant-walk-before --warm-up-time 1 --measurement-time 4
```

Host-load observations: shared host with concurrent agent workloads. At start:
load averages 45.56 / 66.66 / 59.73 (16 cores), another worktree's `rustc`
build and `mds_stores` (Spotlight) active. Run 1 was captured under that load
and produced wide intervals (see below); it was discarded by re-saving the
baseline after the competing build finished. Load fell through the session
(→ ~11 1-min by the final confirmation run). Bench compile: 1m 48s (release,
optimized); kache compilation cache present on host.

Saved-baseline provenance: `redundant-walk-before` = **run 2** below
(`--save-baseline` overwrites; runs 3–4 used `--baseline` and did not modify
the saved data). Persisted at
`target/criterion/reference_graph_{small,large,multi_transclusion}/build_and_validate/redundant-walk-before/`.

## 1.4 Transcribed Numbers (sample count 30 per fixture, warm-up 1 s, measurement 4 s)

| Run (load at start) | Fixture | Median | Confidence interval | Notes |
|---|---|---|---|---|
| 1 — saved then overwritten (45.6 / 66.7 / 59.7) | small | 2.3656 ms | [2.0435, 2.6720] ms | Contended; discarded |
| 1 | large | 26.387 ms | [23.780, 29.143] ms | Contended; discarded |
| 1 | multi_transclusion | 37.761 ms | [34.273, 41.450] ms | Contended; discarded |
| **2 — SAVED `redundant-walk-before`** | **small** | **249.85 µs** | **[241.78, 263.86] µs** | Tight |
| **2** | **large** | **6.4407 ms** | **[6.4118, 6.4723] ms** | Very tight (±0.5 %) |
| **2** | **multi_transclusion** | **10.527 ms** | **[10.446, 10.608] ms** | Very tight (±0.8 %); within 0.7 % of Review 4's quiet-host 10.455 ms |
| 3 — confirmation, `--baseline` (21.8 / 44.0 / 52.3) | small | 233.88 µs | [231.44, 236.87] µs | −23.9 % vs saved; µs-scale scheduler noise |
| 3 | large | 6.4323 ms | [6.3966, 6.4710] ms | +0.05 % vs saved, p = 0.90 — no change; excellent repeatability |
| 3 | multi_transclusion | 12.212 ms | [11.129, 14.379] ms | +13.7 % vs saved; transient contention returned mid-run |
| 4 — confirmation, `--baseline` (11.3 / 36.5 / 48.8) | small | 283.44 µs | [257.08, 322.69] µs | µs-scale noise persists |
| 4 | large | 6.5295 ms | [6.4932, 6.5699] ms | +1.4 % vs saved; stable |
| 4 | multi_transclusion | 10.989 ms | [10.854, 11.126] ms | +4.4 % vs saved; recovered near baseline |

Stability judgment (per plan 1.4 — dispersion and repeatability, not proximity
to Review 4): the saved run's intervals are tight for all three fixtures.
Across repeated runs, `large` repeats within ~1.5 % and `multi_transclusion`
within ~4–14 % depending on transient host load; `small` (µs-scale, ~17k
iterations) swings up to ~25 % with scheduler noise. Expected Phase-5 signal
(~4.15 ms on `multi_transclusion`, ≈ 40 %) is well above the observed
run-to-run noise band; the regression guard (both >5 % and >100 µs) is not
reachable by `small`'s noise (~±60 µs). If Phase 5 finds the host contended
again, the baseline+candidate pair must be recaptured together (plan 5.1/5.2).

Warning observed on `multi_transclusion` runs: "Unable to complete 30 samples
in 4.0s" (collection overran to ~4.9–5.1 s) — informational; 30 samples were
still collected each time.
