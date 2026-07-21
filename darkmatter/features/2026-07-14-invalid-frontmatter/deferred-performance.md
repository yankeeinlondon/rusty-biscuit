---
feature: 2026-07-14-invalid-frontmatter
kind: deferred-performance
---

# Deferred Performance Measurements — Invalid Frontmatter

## Phase 7 benchmark comparison

- **Maps back to:** the review finding *"High — Safety, corpus, performance, and platform acceptance
  evidence is incomplete"* in
  [`review-1.md`](./review-1.md), which states: *"There is likewise no recorded Phase 7 benchmark
  comparison for the no-frontmatter and clean-frontmatter hot paths."*
- **Deferred during:** implementation cycle 1 (`2026-07-18T18:33:32-07:00` → `2026-07-18T20:42:31-07:00`),
  logged in [`log.md`](./log.md).

### What was required

The spec's [Performance](./spec.md) section sets a **relative no-regression** posture, explicitly
deferring any hard per-document millisecond budget. Acceptance is therefore *no measurable
regression* on the two common cases:

1. documents with **no frontmatter**, and
2. documents whose frontmatter is **already clean**.

### Why it was deferred

The host CPU load did not permit a legitimate measurement. Load averages recorded across the task:

| Point in task | 1-min | 5-min | 15-min |
|---|---|---|---|
| Start | 87.07 | 128.63 | 117.97 |
| Peak (mid-task) | 168.92 | — | — |
| Finish | 13.59 | 52.84 | 72.71 |

Multiple agents ran concurrently throughout. A cross-run comparison under this variance would have
measured scheduler contention, not the change. **No number was reported rather than an untrusted
one.**

### What is needed to close it

- A **main-vs-branch drift bracket** on a quiet host: measure `main`, measure the branch, then
  re-measure `main`. If the two `main` measurements do not agree within noise, the host was not quiet
  enough and the comparison is void.
- Both hot paths measured separately — the no-frontmatter path and the already-clean-frontmatter path
  — since the spec's zero-cost guarantee applies specifically to the former.
- Both affected package areas: `biscuit-file` and `darkmatter`.

### Substitute evidence that did land

`darkmatter/lib/tests/clean_counters.rs` (8 tests) expresses the spec's performance requirements as
**counter invariants** rather than timings:

- no frontmatter → zero schema resolution and zero trigger-discovery work;
- trigger discovery and the built validator cached per `clean` invocation;
- safety-gate reparse only on candidate edits — an already-clean document parses once;
- analysis cost does not scale with candidate count.

These hold identically on a loaded host, so they are the durable guarantee. They do **not**, however,
discharge the relative no-regression acceptance, which still requires the timed comparison above.

## Review implementation follow-up — 2026-07-20

- **Maps back to:** the same review finding, *"High — Safety, corpus, performance, and platform
  acceptance evidence is incomplete"*, in [`review-1.md`](./review-1.md).
- **Evaluated during:** review implementation cycle 1, started at
  `2026-07-20T23:38:36-07:00`.

### Benchmark vehicle correction

The audit found that `clean_hot_paths/full_pipeline` still timed only
`Markdown::try_from_content → cleanup → as_string`. The shipped feature now performs raw
frontmatter extraction, YAML analysis, schema analysis, and raw-preserving assembly around that
legacy sequence, so the unchanged harness could not observe the feature's added work.

`darkmatter/lib/benches/clean_hot_paths.rs` now includes those actual feature stages in the two
`full_pipeline` cases. The no-frontmatter fixture takes the real extraction-and-bypass branch. The
already-clean-frontmatter fixture performs one source-first YAML analysis, resolves and applies the
default schema context, cleans the parsed Markdown, and assembles the original frontmatter block
without reserialization. The existing `phase1-before` Criterion baseline therefore remains the
pre-feature comparator while the candidate now measures the shipped path.

### Why the timed comparison remains deferred

No timing was reported in this follow-up. A legitimate result still requires the quiet-host
main/branch/main drift bracket described above. This session used a shared dirty worktree with
concurrent agent activity, so it could neither produce an isolated `main` build nor establish a
quiet, repeatable scheduler baseline. Running Criterion once here would create a precise-looking but
untrustworthy number.

The corrected harness passed `cargo check -p darkmatter --bench clean_hot_paths --color never`, and
the load-independent counter suites passed. These verify that the comparison vehicle compiles and
that the structural cost invariants hold; they do not replace the deferred timing measurement.

## Review 2 Finding 3 — explicit no-regression acceptance

- **Maps back to:** *"High — The explicit no-regression performance acceptance is still
  unverified"* in [`review-2.md`](./review-2.md).
- **Evaluated during:** implementation cycle 2 on 2026-07-21.

### Why the measurement remains deferred

The macOS host was not quiet enough for a legitimate Criterion comparison. Sniff identified an
Apple M4 Max with 16 physical and 16 logical cores. At 01:10 local time, the host reported load
averages of **81.59 / 53.66 / 39.87** and aggregate process CPU use of approximately **923.8%**.
The host was attached to AC power, so power source did not cure the scheduler contention.

The repository state also prevented an isolated bracket. The feature worktree was dirty and shared
with serial review-finding implementations in progress. Its branch tip was `7278b1ccbed5`, while
local `main` was `4bf0db8813de`; measuring either state would have included uncommitted cross-finding
changes. Three older `/private/tmp/dmbench/{before,base,after}` registrations were prunable because
their directories no longer existed, so they contained no retained evidence that could be audited.
No Criterion timing was run and no performance number is claimed.

No completed Linux or Windows runtime result for both corrected full-pipeline cases was present in
the inspected feature or CI records. Workflow configuration and portable source were not treated as
runtime evidence.

### Required rerun procedure

1. Use a quiet, dedicated host on AC power with Low Power Mode disabled and no concurrent builds.
2. Create clean, separate worktrees for the exact `main` and candidate revisions; record both full
   commit IDs, compiler version, OS, CPU, and load averages before and after every run.
3. Use one retained Criterion target directory and measure only
   `clean_hot_paths/no_frontmatter/full_pipeline` and
   `clean_hot_paths/clean_frontmatter/full_pipeline` in this order:
   `main` with `--save-baseline`, candidate with `--baseline`, then `main` again with a distinct
   comparison baseline.
4. Retain terminal output and Criterion raw samples for all three runs. Reject the bracket if the two
   `main` runs disagree beyond their confidence intervals or show material scheduler/thermal drift.
5. Record the candidate confidence intervals, percentage changes, p-values, and the final
   no-regression decision for each case separately.
6. Record completed Linux and Windows runtime gates for the affected Darkmatter and biscuit-file
   package areas alongside the macOS result; do not substitute workflow definitions for run output.

### Bounded structural evidence from cycle 2

- `cargo check -p darkmatter --bench clean_hot_paths --color never`: passed in 12.46 seconds.
- `cargo nextest run -p darkmatter --test clean_counters --color never`: 8 passed, 0 skipped.
- `cargo clippy -p darkmatter --bench clean_hot_paths --color never -- -D warnings`: passed with no
  warnings in 28.77 seconds after waiting for the shared build-directory lock.

These gates prove the corrected benchmark vehicle still compiles, its load-independent hot-path
invariants hold, and the benchmark target is lint-clean. They do not satisfy the timed acceptance.

## Review 3 Finding 3 — performance and cross-platform acceptance

- **Maps back to:** *"High — Performance and cross-platform acceptance are still explicitly
  open"* in [`review-3.md`](./review-3.md), where the finding is explicitly marked
  `DECISION: DEFERRED` and non-blocking.
- **Evaluated during:** implementation cycle 3 on 2026-07-21.

### Review 3 disposition

The timing measurement remains deferred. Sniff identified macOS 26.5.2 on an Apple M4 Max with 16
physical and 16 logical cores and the linked `darkmatter` worktree. At 08:52 local time, the host
reported load averages of **26.27 / 19.37 / 17.79** and aggregate process CPU use of approximately
**228.1%**. The host was attached to AC power with normal power mode selected, but the one-minute
load still exceeded the core count and was not admissible for a low-noise Criterion comparison.

The candidate was also not isolated. Sniff reported the current worktree dirty with 15 changed
paths; `git status` independently showed 13 modified and 2 untracked paths. The registered
`/private/tmp/dmbench/{before,base,after}` directories existed but were no longer Git repositories,
so they could not establish revision identity or supply an auditable main/branch/main bracket.
Existing `target/criterion` samples were left untouched and were not accepted as Review 3 evidence
because they do not carry a retained three-run bracket for the current candidate. No Criterion
timing was run and no performance number is claimed.

The bounded structural check
`cargo check -p darkmatter --bench clean_hot_paths --color never` passed in 6.09 seconds. Source
inspection confirmed that the benchmark continues to register both requirement-matched cases:
`clean_hot_paths/no_frontmatter/full_pipeline` and
`clean_hot_paths/clean_frontmatter/full_pipeline`. This proves the vehicle compiles after the Review
3 functional changes; it is not timing evidence.

Cross-platform acceptance also remains open. The cycle has scoped macOS runtime evidence for both
affected package areas, but no completed Linux or Windows run for the current candidate was found in
the feature records. The public GitHub Actions API returned zero `darkmatter-tests` runs for branch
`darkmatter`, and the Review 3 changes are uncommitted, so no remote commit check can represent this
candidate. The workflow definition's Linux/Windows/macOS matrix is configuration evidence only;
Windows is still soft-fail, and no equivalent three-OS biscuit-file area run was retained. Therefore
the combined cross-platform row is not green.

### Exact rerun procedure

1. Choose exact, committed baseline and candidate revisions. Create three clean worktrees for
   baseline-before, candidate, and baseline-after; record each full commit ID and prove each
   worktree has an empty status before running anything.
2. Use a dedicated macOS host on AC power with normal power mode, no concurrent builds or agent
   sessions, and sustained load comfortably below the logical-core count. Record Sniff OS/hardware/
   worktree output, power state, load averages, compiler version, and aggregate process CPU before
   and after every run.
3. Create a new absolute `CARGO_TARGET_DIR` used only by this bracket. From baseline-before, run:

        CARGO_TARGET_DIR=/absolute/review3-target cargo bench -p darkmatter --bench clean_hot_paths -- full_pipeline --save-baseline review3-main-before

4. From the candidate worktree, use the same target directory and run:

        CARGO_TARGET_DIR=/absolute/review3-target cargo bench -p darkmatter --bench clean_hot_paths -- full_pipeline --baseline review3-main-before

   Immediately retain the complete Criterion tree and terminal output under a candidate-labeled
   artifact directory before the third run can replace any `new` samples.
5. From baseline-after, use the same target directory and run:

        CARGO_TARGET_DIR=/absolute/review3-target cargo bench -p darkmatter --bench clean_hot_paths -- full_pipeline --baseline review3-main-before

   Retain this complete Criterion tree and terminal output separately.
6. Reject the bracket if the two baseline runs disagree beyond their confidence intervals or show
   material load, power, or thermal drift. Otherwise record the confidence interval, median,
   percentage change, p-value, and no-regression verdict separately for the no-frontmatter and
   clean-frontmatter full-pipeline cases. Do not aggregate the two cases.
7. On clean macOS, Linux, and Windows hosts or CI runners at the same candidate commit, retain the
   successful scoped runtime outputs for both affected package areas:

        cd biscuit-file && just test
        cd darkmatter && just test

   Retain package-area lint/build output when required by the CI gate, but do not substitute a
   workflow definition, cross-compilation, or a soft-failed job for successful native runtime
   execution.
