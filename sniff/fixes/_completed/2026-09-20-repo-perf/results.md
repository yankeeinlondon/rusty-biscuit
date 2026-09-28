# Results: parallelize the nested-marker walk

**State: implementation complete; review closed as production ready on
2026-09-26.** The author resolved the R7 escalation by choosing option 2: the
walk caps its workers at `min(available_parallelism, 4)` and accepts the
remaining small-tree cost. The cap was measured and verified in review cycle 2
([`evidence/worker-cap/`](evidence/worker-cap/README.md)).

- Spec: [`spec.md`](spec.md). Plan: [`plan.md`](plan.md). Log:
  [`implementation-log.md`](implementation-log.md). Evidence: [`evidence/`](evidence/).
- Production change: `walk_for_nested_markers` in
  `sniff/lib/src/filesystem/repo/nested.rs` now runs `ignore`'s
  `build_parallel()` with the pre-change builder settings and at most four
  workers (`MAX_NESTED_WALK_WORKERS`; fewer on a host with fewer CPUs). It keeps
  per-visitor marker buffers, merges each buffer once when its visitor drops,
  propagates the `WorkerCollector` to every visitor, and projects candidates
  once through the unchanged `candidates_from_marker_paths`.
- Commits (non-test lib code): harness `43a08f94e` (baseline side),
  implementation `f9af74815`, and parity/fixture suite `2d886cd98`. Review
  cycles 1–2 added the full-output fixture test and the worker cap.

## Acceptance criteria

| # | Criterion | Status | Evidence |
|---|---|---|---|
| AC1 | Test-only serial reference, complete ordered candidate comparison, independent expected candidates, existing root-marker and supplied-evidence tests kept | Met | `serial_reference_paths` and `assert_parity` in `nested.rs` tests; every fixture test asserts independently spelled candidates; the pre-existing `nested.rs` and `detection.rs` tests are unmodified (log § Phase 3) |
| AC2 | Fixture matrix (all 12 names, `.sln`/`.slnx`, many-to-one, one-to-many, depths, empty/root-only), 20× wide-tree repeat, one-worker and multi-worker runs through a private seam | Met | `every_marker_name_registers_its_standards_at_any_depth`, `empty_root_only_and_missing_roots_register_no_candidates`, `wide_tree_parity_holds_across_repeats_and_worker_counts` (`Some(1)`, the production policy, `Some(4)`) |
| AC3 | Git and non-Git roots, ignore rules with isolated Git config, hidden, pruned, and marker-named directories, missing roots, case rules, symlinks, privilege-aware permission test | Met | `git_ignore_rules_apply_under_an_isolated_git_configuration`, `prune_hidden_and_marker_named_directories_keep_their_semantics`, `non_directory_symlinks_are_admitted_and_directory_links_are_not_followed`, `a_symlinked_starting_root_walks_like_its_target_under_the_link_spelling`, `walked_marker_names_follow_the_platform_case_rules`, `non_unicode_basenames_are_not_markers`, `an_unreadable_directory_is_skipped_and_the_walk_continues` (Unix only; R6) |
| AC4 | One `FS_READ_DIRS` and one `REPO_NESTED_MARKER_WALKS` per fallback; zero fallback walks with supplied evidence (including `Some(&[])`); worker propagation proven with a test-only counter | Met | `fallback_walk_records_one_logical_walk_per_invocation`, `a_missing_root_keeps_the_callers_counters`, `an_empty_root_records_one_logical_walk`, `supplied_evidence_starts_no_fallback_walk`, `every_visitor_flushes_its_work_into_the_request` (522 of 522 on the fixture; 12,908 of 12,908 on the checkout) |
| AC5 | Complete detection output before and after, on a controlled fixture and on this checkout | Met | Fixture: `public_structure_detection_matches_the_serial_baseline_on_a_nested_fixture` compares the complete public JSON from serial and parallel fallbacks (review cycle 1). Checkout: [`evidence/detection-output/`](evidence/detection-output/README.md), where the full `detect_repo_structure` JSON is byte-identical at `43a08f94e` and `9d2d39c6d` (77 packages, 2 standards, 2 layers; added in Phase 6) |
| AC6 | `just test` and `just lint` in `sniff/`; focused parity tests on all four operating systems; no CI cells or timing gates added | Met | macOS: 2875 passed / 32 skipped, lint and `--all-targets -D warnings` clippy clean (review cycle 2, with the worker cap). Linux 22/22, native Windows 20/20, WSL2 22/22: [`evidence/cross-os/`](evidence/cross-os/cross-check-nested-tests.txt). CI configuration is unchanged (log § Phase 5) |
| AC7 | `just test` in `darkmatter/` with the three observation-boundary tests | Met for this fix; the suite is red for an unrelated reason | 8,496 of 8,498 passed, and all three boundary tests passed. The 2 failures are a known issue from another change (see Remaining risks) |

## Performance

All numbers are from one macOS host (M4 Max, 16 logical CPUs). The filesystem
cache was warm and the host was heavily loaded by other sessions. Each cell
gives the median of three alternated bracket medians, then the range of those
bracket medians, then the min–max of all samples. Full tables:
[`evidence/after/README.md`](evidence/after/README.md) and
[`summary-table.md`](evidence/after/summary-table.md). Environment:
[`evidence/environment.md`](evidence/environment.md).

| Measure | Before | After | Change |
|---|---|---|---|
| Walk alone, release (same-process serial reference → parallel) | 66.53 ms (64.67–68.52; 63.83–905.08*) | 22.57 ms (20.54–22.81; 19.59–27.94) | 2.95× faster |
| Walk alone, debug | 173.83 ms (173.50–177.55; 167.39–191.11) | 53.00 ms (42.14–64.61; 38.20–76.28) | 3.28× faster |
| `detect_repo_structure(<repo root>)`, release (`43a08f94e` → after) | 79.40 ms (78.11–80.98; 76.81–86.11) | 35.71 ms (33.30–38.92; 32.75–59.31) | 2.22× faster |
| `detect_repo_structure(<repo root>)`, debug | 243.66 ms (238.88–252.14; 236.20–261.58) | 106.91 ms (106.51–109.26; 102.68–188.47) | 2.28× faster |
| Tiny tree (8 files) `detect_repo_structure`, release | 0.49 ms | 3.36 ms | **+2.9 ms (≈6.8× slower)** |
| Tiny tree `detect_repo_structure`, debug | 0.80 ms | 4.05 ms | **+3.2 ms (≈5× slower)** |
| Tiny tree, 1 thread × 400 requests | ≈1,940 req/s | ≈300 req/s | **−84% throughput** |
| Tiny tree, 14 threads (test-runner concurrency) | ≈4,160 req/s | ≈4,075 req/s | −2% (within noise) |
| Checkout, 14 threads: throughput | ≈53 req/s | ≈52 req/s | −2% (within noise) |
| Checkout, 14 threads: p95 latency, CPU, peak RSS | 285–296 ms, 33–35 s, 150–152 MB | 419–420 ms, 38–40 s, 174–196 MB | +42%, +15%, +14–31% |

\* One 905 ms outlier sample in bracket r2; that bracket's median was 68.52 ms.

- **Gain versus variation:** the worst after-side bracket beats the best
  baseline bracket by more than 40 ms (release) and more than 100 ms (debug).
  The walk speedup and the command speedup are reported separately. No
  compose-suite speedup is claimed, because that suite was not measured.
- **Historical figures (context only):** 172 → 30 ms debug and 61 → 18 ms
  release for the walk alone. The checkout has grown since then, from 11,290
  paths and 92 markers to 12,908 entries and 116 markers.
- **Attribution:** in release, "other work" in `detect_repo_structure` is
  unchanged (≈13.6 → ≈13.1 ms). In debug it is ≈71.6 ms on the baseline side,
  against the historical ≈60 of 232 ms. The after-side debug subtraction is
  noisy (about ±11 ms).
- **Worker count:** all 12 default workers visited entries on the checkout in
  every run. Default worker counts on the wide fixture were 10 on macOS and 12
  on Linux and native Windows ([`worker-diagnostics.md`](evidence/after/worker-diagnostics.md)).
- **Why tiny trees are slower:** in `ignore` 0.4.25, each parallel walk spawns
  fresh worker threads, and idle workers poll with a 1 ms sleep until all of
  them are idle. That gives every call a floor of about 3 ms with 12 workers
  (0.27 ms with 1 worker, 1.70 ms with 4).

The table above measures the original 12-worker policy. The production code
now caps workers at four; see the next section.

### Decision rule (R7): triggered, resolved with a four-worker cap

The latency and sequential-throughput regressions on tiny trees were
material, so R7 stopped the work and escalated. On 2026-09-26 the author chose
option 2. They judged the 8-file tree unrepresentatively small and a few
milliseconds an acceptable price for the large-repository gain, and capped the
worker count at `min(available_parallelism, 4)`. Re-measured on the same host,
with only the cap differing between sides (release;
[`evidence/worker-cap/`](evidence/worker-cap/README.md)):

| Measure | 12 workers | 4 workers | Serial baseline (Phase 4) |
|---|---:|---:|---:|
| Checkout `detect_repo_structure` | 34.25 ms | 38.79 ms | 79.40 ms |
| Tiny tree `detect_repo_structure` | 3.1–3.3 ms | 2.0–2.1 ms | 0.49 ms |
| Tiny tree, 1 thread: throughput | ≈320 req/s | ≈500 req/s | ≈1,940 req/s |
| Checkout, 14 threads: p95 latency | 381–427 ms | 315–333 ms | 285–296 ms |
| Checkout, 14 threads: peak RSS | 170–201 MB | 130–157 MB | 150–152 MB |

The command stays about 2× faster on this checkout. The tiny-tree penalty is
about 1.5 ms instead of 2.9 ms, and the concurrent tail-latency and memory
regressions are mostly gone. Work counters are unchanged. The nested parity
tests were rerun with the cap on macOS, Linux, native Windows, and WSL2.

## Work counters

Stable counters were compared separately from timing:
[`evidence/counters/README.md`](evidence/counters/README.md). Every counter
is identical before and after, in release and debug (for example,
`nested_marker_walks` 1, `read_dirs` 2, `manifest_parses` 83,
`metadata_probes` 369). The gain comes from running the same traversal
concurrently and keeping fewer paths, not from doing less logical work. No
worker counts disappeared: the test-only per-entry counter equals the serial
reference's entry count.

## Deviations from the plan

1. **The depth-0 root entry is skipped** (Phase 2, from spike finding 2).
   Before this fix, a symlinked starting root whose link name matched a
   marker registered the link's *parent*, which lies outside the repository,
   as a candidate. That can no longer happen. The serial reference skips the
   root in the same way, and this is documented on both functions. This is
   the only observable behavior difference, and it only affects that unusual
   input. The author can override it in review.
2. **Collector activation order** (Phase 2). The collector is activated after
   the error check, not as the first statement of the callback as in
   `ManifestIndex::build`. `ignore` runs its first visitor on the calling
   thread and gives it root errors, so activating first would erase the
   caller's counters on a missing root. The regression test
   `a_missing_root_keeps_the_callers_counters` failed under the plan's
   original order.
3. **R3 corpus** (Phase 4). Both sides were measured against this checkout
   instead of each side's own switched checkout. This kept the corpus
   byte-identical (12,908 entries in every bracket).
4. **Phase 5 ran before the R7 decision.** It was validation only and changed
   no code (log § Phase 5).
5. **AC5 checkout comparison added in Phase 6.** Earlier phases recorded
   candidate parity and counters but not the full detection output, so
   Phase 6 added [`evidence/detection-output/`](evidence/detection-output/README.md).
   This added no tests or code.

## Remaining risks and follow-ups

- **The tiny-tree floor (R7), accepted.** With the four-worker cap, every
  structure-only caller still pays about 1.5 ms more per call on small trees.
  These callers include the `sniff repo` commands, claudine completion and
  composition, darkmatter compose, and sniff recent-commits. The author
  accepted this cost on 2026-09-26. Serial-first (adaptive) scheduling remains
  a possible future fix.
- **Darkmatter L1 is red on this branch for an unrelated reason.** Two
  `feature_review_incident` tests fail with
  `File not found: ../_writing-clearly.md`. Prompts commit `6c682a7fd` caused
  this, and `fe209ae0f` on `origin/fix/wt-ux` fixes it. Until that fix reaches
  this branch, darkmatter's CI cell will fail. Sniff code is not involved.
- **Latent hazard in `ManifestIndex::build`.** It activates its collector
  first, the same pattern deviation 2 avoids. This is not reachable in
  practice, because its root exists when it runs. It was left unchanged as
  out of scope.
- **Measurement conditions.** All timings come from one heavily loaded host
  with a warm cache. No cold-cache claim is made. Timing was not measured on
  Linux, Windows, or WSL2, which is by design; only parity was checked there.
- **No cross-OS CI receipt was published.** The focused `cross-check` runs
  were filtered, so they publish no receipt. The sniff CI cells prove
  themselves in CI as usual.
- **Stale line in another area's plan.** Drift sweep finding, not edited:
  `claudine/features/2026-08-01-faster-compose/plan.md` still lists
  `walk_for_nested_markers` as a "serial `WalkBuilder`". That line is part of
  a dated 2026-09-08 source review in claudine's active plan. Updating it
  belongs to that plan's next revision.
- **No CHANGELOG entry.** The change is internal, with no public API or output
  difference. Whether to note the speedup in `sniff/lib/CHANGELOG.md` is left
  to the author, after the R7 decision.

## Drift sweep (Phase 6)

These were checked and still match the code: the `nested.rs` module,
function, and inline docs; `sniff/docs/sniff-library-architecture.md` (the
`nested_marker_walks` counter still reads 1 per structure-only request);
`sniff/lib/benches/README.md`; the doc comments in `benches/cases/repo.rs` and
`examples/work_counts.rs`; and the `sniff/lib/tests/fixtures.rs` root-marker
fixture comment. No crates were added or removed (the fix's commits do not
touch `Cargo.lock` or any `Cargo.toml`), so no `docs/dependencies.md` change
is needed. The `sniff` skill already records the parallel-walk facts in
`performance.md`, from Phases 1–4.
