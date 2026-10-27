---
spec: /Volumes/coding/wt/rusty-biscuit/feat-work-reduction/claudine/features/2026-10-25-pipeline-work-reduction/spec.md
plan: claudine/features/2026-10-25-pipeline-work-reduction/plan.md
implemented_by: claude/opus
started_phase: "1"
source_files_during_phase_1:
    - claudine/lib/tests/l1/host_catalog.rs
    - darkmatter/cli/src/commands/render.rs
    - darkmatter/lib/benches/pipeline_work_support/mod.rs
    - darkmatter/lib/benches/schema_validation.rs
docs_updated_during_phase_1:
    - claudine/features/2026-10-25-pipeline-work-reduction/spike-report.md
docs_created_during_phase_1:
    - claudine/features/2026-10-27-shared-launch-trigger-discovery/spec.md
    - claudine/features/2026-10-27-shell-site-placeholder-parse-once/spec.md
skills_files_updated_during_phase_1:
    - .claude/skills/rust-testing/performance-testing.md
packages:
    - claudine
    - darkmatter
    - darkmatter-cli
decisions_for_review:
    - |-
      The saved timing baseline (`pipeline-work-pre`) was taken with the computer under moderate load, not on a quiet one. The plan asked for a quiet host; the spike report suggested a load average of about 2. Other worktrees kept compiling, so after waiting 30 minutes I ran the pass once the 1-minute load fell under 8, the bench recipe's own limit (half of 16 cores). It began at 7.7 and ended at 12.3. Every anchor's Criterion interval was within about ±1% of its median.
      Alternatives: keep waiting for a load near 2, which could take hours and was not guaranteed; or take the pass anyway and record the load, as I did.
      If the reviewer wants a quieter baseline, rerun the two `just _bench … --save-baseline pipeline-work-pre` commands in the log on a quiet host before phase 5's comparison. Nothing else depends on it.
    - |-
      I classified placeholder parsing (D8) as qualifying and wrote its own spec (`2026-10-27-shell-site-placeholder-parse-once`). The decision depends on which repeated parses count as D8's. While preparing one sequence shell command, Claudine parses each placeholder five times, once per analysis pass. Removing four of the five is 6.4% of anchor A6's time (over the 5% line); the bytes are 3.2%. Counting only the two repeats that Darkmatter's isolated template code shows gives 3.2% of time, which would close D8 as "measured, no change".
      I chose the wider count because all five passes run inside one site preparation, which is what D8 describes ("placeholders reparsed across analysis passes"). The time share uses the count × unit-cost method the spike used, with Criterion medians from the moderately loaded pass above.
      If the reviewer prefers the narrower count, delete that spec's directory and change D8's row in `spike-report.md` → "Current outcome (2026-10-27)" to "measured, no change".
    - |-
      Two baseline gates were already failing at the starting revision. I fixed both instead of only recording them, because the plan requires these gates to pass and the fixes are mechanical and outside the measured code.
      (1) Darkmatter lint: `darkmatter/cli/src/commands/render.rs::run_render` takes 8 arguments; I added `#[allow(clippy::too_many_arguments)]`, as five other commands in that crate already do.
      (2) The test-toolkit archive-path guard flagged three compile-time manifest paths: a Claudine L1 test (`host_catalog.rs`) and two Darkmatter benches (`pipeline_work_support/mod.rs`, `schema_validation.rs`). I spelled them `biscuit_test_harness::manifest_dir!()`, the guard's remedy.
      Alternatives: leave them failing and report them, or add allowlist entries for the two benches (they never run from an archive). Undoing either fix is a one-line revert per file.
message_to_agent: |-
  Phase 1 (baseline) is done. Facts for phase 2:
  - Criterion baseline `pipeline-work-pre` is saved for `darkmatter` and `claudine` `pipeline_work` (target/criterion). Do not overwrite it; phase 5 compares with `--baseline pipeline-work-pre`. It was taken at load about 7.7–12 on 16 cores (see the log).
  - Refreshed before-change counts and bytes are in this log and in `spike-report.md` → "Current outcome (2026-10-27)". A4's total is now 716,451,720 B (preflight 91.6 MB, group step 624.8 MB); A6 is 10,000 `ExpressionParse` and 26.7 MB.
  - C6 is closed (measured, no change). D3 and D8 qualified and each has its own dated active spec (`2026-10-27-shared-launch-trigger-discovery`, `2026-10-27-shell-site-placeholder-parse-once`). This feature must NOT implement either.
  - Claudine's lib unit-test binary declares the counting allocator under `cfg(all(test, feature = "work-counters"))`. Run the crate-private scenarios from `claudine/` with `just test-library --release --features work-counters --run-ignored only --no-capture pipeline_spike_<name>`, one per process.
  - The `_bench` recipe refuses a loaded host when run non-interactively. Set `BENCH_NONINTERACTIVE=1 BENCH_YES=1` for count-only runs.
---

# Implementation Log for 2026-10-25-pipeline-work-reduction (5 phases)

## Phase 1

### Prerequisites

- Revision: `d7ea96a1f` (branch `feat/work-reduction`); the only uncommitted file at start was this log (empty).
- Host: macOS development host, 16 cores; load average 4.4–5.4 at start (another worktree was running `claudine compose` review jobs).
- Both dependency chains are in that revision:
  - `2026-10-26-dmls-frontmatter-typing-allocation` (last planning commit `aa8f41143`);
  - `2026-10-26-sequence-quadratic-frontmatter` (Phase 4 close `7d67c5473`);
  - `2026-10-25-fix-now` (`98c86432c`) → `2026-10-25-host-function-catalog-leak` (`d7310764d`) → `2026-10-25-schema-diagnostic-provenance` (`dcb13ab9e`) → `2026-10-25-shared-acyclic-definitions` (cycle 2 close `df6ea2d9d`).
- X1 is delivered: `darkmatter/dmls/src/overlay/analysis.rs` keeps prepared `AlternativeMatcher`s keyed by `alternative_fingerprint` across snapshots. As the spec's re-baseline decision says, the A3 hover reference is now the post-sharing 66,277 B (verb) / 69,943 B (expression), replacing 58,763 / 105,025.

### Tooling inventory (existing; nothing recreated)

- Counters: `darkmatter::work_counters` and `claudine::work_counters` behind each lib's `work-counters` feature (`pub mod work_counters` is `#[cfg(feature = "work-counters")]`). `note_work!` expands to a `#[cfg(feature = "work-counters")]` statement, so a default build has no counter state or calls. DMLS enables Darkmatter's feature only as a dev-dependency.
- Counting allocator (`test_toolkit::alloc_count::CountingAllocator`) is declared only in final binaries: `darkmatter/lib/benches/pipeline_work_alloc.rs`, `claudine/lib/benches/pipeline_work_alloc.rs`, `darkmatter/dmls/benches/pipeline_work_a3.rs`, and Claudine's lib unit-test binary under `cfg(all(test, feature = "work-counters"))` for the crate-private `pipeline_spike_*` scenarios (C1, C3, A4 group step, C6, C7 per task).
- Timing targets `darkmatter/lib/benches/pipeline_work.rs` and `claudine/lib/benches/pipeline_work.rs` declare no global allocator (System).
- Fixtures: `darkmatter/lib/benches/fixtures`, `pipeline_work_support/`, `tests/fixtures/recursive_schema_frozen`.
- Report rows: `spike-report.md` has a row for every C1–C8, D1–D13, P1, X1–X4.
- Docs: `tools/test-toolkit/README.md` (`alloc_count`) matches the tooling. The `rust-testing` skill's `performance-testing.md` named only bench targets as allocator owners; it left out the Claudine unit-test-binary exception and how to run those scenarios. I added that paragraph (the only omission repaired).

### Baseline gates (before any phase-1 change)

| Gate | Result |
| ---- | ------ |
| `darkmatter/ just test` | 12,908 passed, 10 skipped |
| `darkmatter/ just lint` | **failed (already on `main`)**: `clippy::too_many_arguments` on `darkmatter/cli/src/commands/render.rs::run_render` (8 arguments), which arrived with the `main` merge `d7ea96a1f`. Fixed with the crate's existing `#[allow(clippy::too_many_arguments)]` idiom (used on five other commands); no behavior change. Lint then passed |
| `claudine/ just test` | 11,709 passed, 15 skipped |
| `claudine/ just lint` | passed |
| root `just test test-toolkit` | **1 failure (already present)**: `archive_path_guard::no_archive_executed_target_bakes_in_a_producer_path` reported three `env!("CARGO_MANIFEST_DIR")` sites: `claudine/lib/tests/l1/host_catalog.rs:331`, `darkmatter/lib/benches/pipeline_work_support/mod.rs:757`, `darkmatter/lib/benches/schema_validation.rs:338`. Fixed by spelling each as `biscuit_test_harness::manifest_dir!()`, the guard's remedy (no allowlist entry), and dropping the now-unused `Path` import in `schema_validation.rs`. Then 477 passed, 4 skipped |
| root `just _lint test-toolkit` | passed |

### Wave 2: count and allocation refresh

Toolchain `rustc 1.98.1 (48a229cea 2026-09-01)`, `cargo 1.98.1 (797e8a9bc 2026-08-05)`, `bench` profile (release settings) for the bench targets and `--release` for the crate-private scenarios. Revision `d7ea96a1f` plus this phase's uncommitted edits (the lint attribute, the three `manifest_dir!()` spellings, the skill paragraph); none of them touches measured code. The host was **loaded** for every count run: another worktree was building and testing `biscuit-terminal`, and other agents were running. Counts and bytes do not depend on load. Times printed by these runs are counting-allocator observations under load and are not used for any verdict.

Commands (repository root, `BENCH_NONINTERACTIVE=1 BENCH_YES=1` because the host-state preflight refuses a loaded host in a non-interactive session):

| Command | Load before → after |
| ------- | ------------------- |
| `just _bench darkmatter --features work-counters --bench pipeline_work_alloc` | 15.41 → 27.15 |
| `just _bench claudine --features work-counters --bench pipeline_work_alloc` | 27.15 → 36.25 |
| `just _bench dmls --bench pipeline_work_a3` | 36.25 → 32.09 |
| `claudine/`: `just test-library --release --features work-counters --run-ignored only --no-capture pipeline_spike_c6` | 17.64 |
| `claudine/`: `just test-library --release --features work-counters --run-ignored only --no-capture pipeline_spike_a4` | 18.99 |

Harness accounting needed no repair. Each scenario resets the peak (`reset_peak`) immediately before it is measured, and the group-step scenario joins its workers before the final snapshot.

Results that changed from the report's last figures:

- **A3** (DMLS edit loop) reproduces the post-sharing figures recorded by `2026-10-25-shared-acyclic-definitions`: `a3x1` warm total 52,848,942 B (diagnostics 37,090,371 B; overlay 14,968,060 B), cold 345,580,121 B; `a3x10` warm 98,051,049 B, cold 390,782,228 B. Hovers are verb 66,277 B and expression 69,943 B. DMLS's own counters are zero in every warm loop.
- **A2** (Claudine bundle, warm / cold bytes): `no_trigger_match` 3,130,659 / 54,362,491; `document_events` 3,209,451 / 56,000,826; `nested_sequence_group` 3,223,037 / 60,422,122; `implement_plan` 4,667,124 / 331,148,774. The cold rows fell from 334–810 MB with sharing.
- **A1**: corpus validate warm 88,826,897 B; with the runtime baseline warm 93,888,707 B, cold 180,609,812 B (was 202.6 MB).
- **A4**: preflight 91,614,330 B (peak 22,457,613; `PredictionStateBuild:98`); group step 624,837,390 B (peak 149,752,757; `RuntimeSnapshot:615`, `RuntimeStateBranch:200`, `LiveFrontmatterView:200`). Total **716,451,720 B** (was 843.4 MB).
- **A6**: 10 / 100 steps → `ExpressionParse` 1,000 / 10,000 (of them `DeferredRootScanParse` 200 / 2,000); bytes 3,200,966 / 26,709,585.
- **C7** per-task bytes 43.80 MB / 100 tasks and 424.59 MB / 1,000 tasks (`PredictionStateBuild` 2 per task), the same as the post-sequence-fix rows.
- **D1** (1,000 × 3-item eager list): normalize only 162,444,780 B with 6,000 `RewriteArmCompile`; isolated rewrite compiles 2 validators per item at every size.
- **D2**: validate then presence check 159,053,226 B vs validate only 88,805,226 B at 1,000 documents (the second preparation is 44.2%).
- **D4**: positions cost 88,805,226 − 73,834,949 = 14,970,277 B of A1 validation (16.9%); schema-free 6,450,188 vs 1,434,911 B.
- **D12**: `edit_and_reuse` 10 paths → `NamedTypeTableParse` 1; 300 paths → 300 (the memo still clears past 256).
- **D13**: `suggested_imports` 1 / 10 / 100 → `AlphanumericCheckCompile` 2 / 11 / 101.
- Closed items (C5, C8, D5, D6, D7, D9, D10, D11, X3, X4) reproduce their spike counts; their verdicts do not change.

### Wave 2: timing baseline `pipeline-work-pre`

- Waited for a quiet host; other worktrees kept the load high. After 30 minutes the pass began at a 1-minute load of 7.70 (5-minute 8.20), which passes the `_bench` preflight (limit 8, half the cores) but is above the "about 2" the spike report asked for.
- Commands, from the repository root (both benches pre-built with `--no-run` so nothing compiled during timing):
  - `just _bench darkmatter --bench pipeline_work -- --save-baseline pipeline-work-pre`: load 7.70 → 7.03, 12:48:26–12:52:14;
  - `just _bench claudine --bench pipeline_work -- --save-baseline pipeline-work-pre`: load 7.03 → 12.29, 12:52:14–12:52:51.
- Default features, so no counters; system allocator; `bench` profile; toolchain as above. Criterion's intervals were narrow: every anchor case was within about ±1% of its median (for example A6 8.1460 / **8.2117** / 8.2866 ms; A4 35.132 / **35.349** / 35.599 ms; A1 warm 23.535 / **23.755** / 23.909 ms). So the pass was not repeated. The full table is in `spike-report.md` → "Current outcome (2026-10-27)".
- DMLS A3 has no Criterion target; its elapsed times come from the counting allocator under load and are recorded as instrumented observations only.

### Wave 2: conditional items (Decision 4)

- **C6, measured, no change.** `pipeline_spike_c6`: `PrewarmStateBuild` is still 1 per phrase, but classification builds over the shared frontmatter view (`EffectiveStateBuilder::with_shared_frontmatter`), so a phrase costs 15 KB at both 10 KB and 200 KB of frontmatter. Before, it was 53 KB and 757 KB. 20 phrases over 200 KB: 301,844 B (was 15.2 MB). Peak 13 KB. The largest real phrase count is 6 (`prompts/_pr/push.md`), so clause 2 would need about 660 phrases. A4 estimate 0.1%.
- **D3, qualifies (clause 1, estimated bytes 5.8%).** The share rose from 5.0% because A4's total fell from 843.4 MB to 716.5 MB. Created `claudine/features/2026-10-27-shared-launch-trigger-discovery/spec.md`.
- **D8, qualifies (clause 1, time 6.4%).**
  - A6 parses each placeholder 10 times per step. To attribute those parses, I added a temporary `D8_PARSE_SITES` backtrace print at `Parser::with_mode`'s counter, ran `cargo bench -p claudine --features work-counters --bench pipeline_work_alloc --profile dev -- A6` once, and **removed it** (`git diff darkmatter/lib/src` is empty).
  - The tally: exactly one parse per placeholder at each of ten sites. Five are passes inside Claudine's `SequencePreflight::prepare_site` (`parse_authored`'s typing, `approval_diagnostics`, `first_target_identity_root`, `project_types`, `mark_late_holes`). The other five are two frontmatter `value_roots` scans, `reject_removed_sequence_reads`, and two interpolations.
  - Removable: 4 × 1,000 = 4,000 parses × 132.18 ns = 0.529 ms of 8.2117 ms. Bytes 3.2%.
  - Created `claudine/features/2026-10-27-shell-site-placeholder-parse-once/spec.md`.
- Every other item keeps its outcome (report section "Every other item").

### Phase 1 gates (after every edit)

| Gate | Result |
| ---- | ------ |
| `claudine/ just test` | 11,709 passed, 15 skipped |
| `claudine/ just lint` | passed |
| `darkmatter/ just test` | 12,908 passed (2 slow), 10 skipped |
| `darkmatter/ just lint` | passed |
| root `just test test-toolkit` / `just _lint test-toolkit` | 477 passed, 4 skipped / passed |
| `cargo check -p darkmatter --benches --features work-counters` | no warnings |

- **Requirement-to-test mapping.** This phase adds no behavior, so it adds no behavior test. Its source edits are a lint attribute, a test's path spelling, and two bench path spellings. The existing tests that cover them:
  - `claudine/lib/tests/l1/host_catalog.rs::claudine_ships_no_host_function_catalog_and_no_facts_artifact` (run alone and in the suite);
  - `tools/test-toolkit/tests/archive_path_guard.rs::no_archive_executed_target_bakes_in_a_producer_path`, now passing;
  - the Darkmatter benches, which compiled and ran in the count and timing passes above.
  The input-robustness matrix does not apply: no reader or loader changed.
- **Dependent packages.** None retested beyond the two areas: the edits change no public symbol or behavior (the `run_render` attribute is lint-only).
- **Cross-OS.** No cross-OS check: `manifest_dir!()` is the repository's portable spelling and nothing OS-specific changed.
- No pre-existing failures remain. The two found at the start (Darkmatter lint, test-toolkit archive guard) are fixed above.
- Scope calculator (`git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`): `packages` = `claudine`, `darkmatter`, `darkmatter-cli`, `test-toolkit`; `change_class` = `package`. The gates could not be skipped, and all four packages' gates ran and passed. `test-toolkit` is in scope because its archive guard reads the edited files; none of its own files changed.
