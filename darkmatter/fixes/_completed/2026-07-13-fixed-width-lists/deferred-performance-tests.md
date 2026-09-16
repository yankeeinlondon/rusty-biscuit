---
fix: 2026-07-13-fixed-width-lists
maps_to_finding: "F3 — High — Mandatory performance budgets have no benchmark evidence"
review: darkmatter/fixes/2026-07-13-fixed-width-lists/review-1.md
log: darkmatter/fixes/2026-07-13-fixed-width-lists/log.md
deferred_on: 2026-07-20
blocked_on: quiet host
---

# Deferred Performance Measurement — Fixed-Width Lists

## What this maps back to

- **Finding:** "High — Mandatory performance budgets have no benchmark evidence" in
  [`review-1.md`](review-1.md).
- **Specification:** [`spec.md`](spec.md) → Test Plan → "Performance regression" (~lines 631-645)
  and Acceptance Criterion 15 (~lines 709-711).
- **Implementation log:** [`log.md`](log.md) → "Implementation of Review Findings #1" → F3.

## Status

| Half of the finding | Status |
|---|---|
| AC 15 structural requirement (parse counts) | **SATISFIED** — proved deterministically, load-independent |
| Criterion benchmark harness | **COMPLETE** — written, smoke-tested, verified against the baseline ref |
| Criterion timing measurement | **DEFERRED** — blocked solely on host quiet |

Nothing needs to be re-derived. Only the two benchmark runs remain.

## Why it was deferred

The measurement was attempted on a macOS host running **load averages of 89-147 on 16 physical
cores** — roughly 6-9x oversubscribed — with 7 active sessions and three other agents running
concurrent WezTerm L2 suites throughout.

The tightest budget under test is **10%**. Criterion's own run-to-run variance on an idle machine is
roughly **1-3%**. Under this load, scheduler contention alone produces double-digit median swings, so
the noise floor exceeds the budget being tested and any verdict would be meaningless.

**No timing number was recorded, estimated, or extrapolated.** Reporting a load-contaminated median
would have been worse than deferring, because it would have the shape of evidence without the
substance.

## The structural half that IS proved

AC 15's normative requirement is that the default cleanup path reuses its existing parse and that
fixed-width mode adds no parse beyond the established cleanup-plus-reflow sequence. That is a
structural property, not a timing property, and it is now proved by
`darkmatter/lib/src/markdown/cleanup/tests/parse_count.rs` (8 tests, with a non-vacuity guard):

| Path | Parses |
|---|---|
| `cleanup_content` + all 8 indent/spacing variants | 1 |
| `strip_incidental_newlines` (standalone) | 1 |
| `reflow_to_width` | 1 |
| `cleanup_to_fixed_width` | 2 |
| `cleanup_content` → `reflow_to_width` (`md clean --fixed-width`) | 2 |

Default cleanup adds no second parse. Fixed-width is exactly cleanup-plus-reflow with no third parse.
F1's reference-definition protection reads off the existing offset iterator at zero additional parse
cost — confirmed by measurement rather than inspection.

## The benchmark

`darkmatter/lib/benches/clean_hot_paths.rs`, Criterion group **`clean_list_budgets`** — 8 cases,
being the four fixture classes the spec names × {default cleanup, fixed-width cleanup}:

- representative top-level prose
- flat lists
- deeply nested lists
- blockquoted task lists

Fixtures are generated deterministically from constants (no clock, RNG, or filesystem), 60 repeated
units each, so baseline and candidate see byte-identical input.

Fixed-width cases run `cleanup_content` then `reflow_to_width` — the sequence `apply_cleanup` in
`cli/src/commands/clean.rs` actually executes — rather than the `Markdown` wrapper, whose frontmatter
parse and re-serialization are constant costs shared by both modes and would compress the 2x ratio
toward 1.

## Pre-fix baseline state

Git ref **`96c6616e9`** (`docs(darkmatter): ratify invalid-frontmatter phase 1 deliverables`) — the
parent of `4d0dd908e`, the first commit of this fix's implementation.

Verified: at this ref `reflow.rs` has no semantic-model path, and `cleanup_content`,
`reflow_to_width`, and `strip_incidental_newlines` all exist with today's signatures, so the
benchmark compiles unmodified against it.

**Two workarounds are required, both verified by an actual build and `--test` run:**

1. `96c6616e9` does not build standalone. `darkmatter/lib/src/markdown/span.rs` imports
   `biscuit_file::SourceSpan`, but the biscuit-file side (`7aaa9dccc`) is on the `darkmatter` branch
   and is *not* an ancestor of this ref — the branch was transiently red here. Fix with
   `git checkout darkmatter -- biscuit-file/`.
2. The `clean_hot_paths` bench target is not declared at this ref. Append to
   `darkmatter/lib/Cargo.toml`:

   ```toml
   [[bench]]
   name = "clean_hot_paths"
   harness = false
   ```

## Commands to run

```bash
# Baseline
git worktree add /tmp/dm-baseline 96c6616e9 --detach
cd /tmp/dm-baseline
git checkout darkmatter -- biscuit-file/
printf '\n[[bench]]\nname = "clean_hot_paths"\nharness = false\n' >> darkmatter/lib/Cargo.toml
cp <fix-tree>/darkmatter/lib/benches/clean_hot_paths.rs darkmatter/lib/benches/clean_hot_paths.rs
cargo bench -p darkmatter --bench clean_hot_paths -- --save-baseline fwl-prefix

# Candidate (fix tree, same host, same session, no reboot between)
cargo bench -p darkmatter --bench clean_hot_paths -- --baseline fwl-prefix
```

Criterion baselines live under `target/criterion/`. The two trees have separate `target/`
directories, so either copy `/tmp/dm-baseline/target/criterion/` into the fix tree's
`target/criterion/` before the candidate run, or point both at one `CARGO_TARGET_DIR`.

## Budgets and pass/fail arithmetic

`median` = Criterion's reported median.

| # | Scope | Test |
|---|---|---|
| B1 | `{prose, flat_list, nested_list, blockquoted_tasks}/default_cleanup` | `median_candidate ≤ 1.10 × median_baseline` — all four must pass |
| B2 | `{flat_list, nested_list, blockquoted_tasks}/fixed_width_cleanup` (list-heavy only) | `median_candidate ≤ 1.15 × median_baseline` — all three must pass |
| B3 | candidate run only, same fixture both sides | `median(<fixture>/fixed_width_cleanup) ≤ 2.00 × median(<fixture>/default_cleanup)` — all four fixtures |

B1 and B2 are cross-run. B3 is within a single run and is therefore the only budget partially robust
to load. **Record the verdict per case, not as an aggregate** — an averaged verdict can hide one
fixture blowing its budget.

## Host admissibility

Required before the numbers mean anything:

- **1-minute load average ≤ 2.0 on this 16-core host** (≈12% utilization)
- no other agent sessions
- no concurrent `cargo` builds
- AC power
- Criterion's default sampling

Rationale: the tightest budget is 10% and Criterion's idle-machine variance is 1-3%, so the noise
floor must sit at least ~3x below the budget for a verdict to carry information. At load ≥ 4 on 16
cores, scheduler contention alone produces double-digit median swings and B1 becomes untestable.

Check `uptime` immediately **before and after** each run, and record both.

## Drift bracket

Per repo convention, cross-run comparison requires bracketing. Run **baseline → candidate →
baseline again** and confirm the two baseline medians agree within 3%. If they do not, the host
drifted mid-measurement and the run is void regardless of what the candidate numbers say.

## Review 2 — H2 re-evaluation

### What this maps back to

- **Finding:** "High — Mandatory timing budgets still have no verdict" in
  [`review-2.md`](review-2.md).
- **Exact implementation-log title:** `H2 — Mandatory timing budgets still have no verdict`.
- **Specification:** [`spec.md`](spec.md) → Test Plan → "Performance regression" and Acceptance
  Criterion 15.
- **Implementation log:** [`log.md`](log.md) → the section that started at
  `2026-07-20T15:55:57-07:00` → H2.

### Review 2 status and host evidence

The Review 2 suggestion was evaluated on 2026-07-20. The load-independent parse-count half remains
satisfied: the focused Nextest run passed all 8 tests. The Criterion harness remains present, but a
bounded `cargo bench -p darkmatter --bench clean_hot_paths -- --test` smoke attempt spent its
55-second allowance compiling the release graph and was interrupted before any benchmark case ran.
This does not invalidate the previously recorded successful harness smoke test.

Only the normative timing measurement is deferred. The host is an Apple M4 Max with 16 physical and
logical cores and 128 GiB memory. During this re-evaluation, `uptime` reported 8 users and load
averages of `77.23 67.44 67.80` at 16:16 local; at 16:20 it still reported 8 users and
`61.95 72.24 70.08`. The 1-minute load was therefore 31-39 times the documented ceiling of 2.0.
Scheduler noise at that level is larger than the tightest 10% budget, so no baseline, median,
estimate, or pass/fail timing verdict was created. Shared Criterion target data and a baseline
worktree were deliberately left untouched.

### Quiet-host Review 2 procedure

First satisfy every admissibility condition already listed above: 1-minute load at or below 2.0
before and after each run, no other agent sessions, no concurrent Cargo builds, AC power, and
Criterion's default profile and sampling. Prepare the baseline worktree exactly as documented in
"Pre-fix baseline state," then use one shared absolute `CARGO_TARGET_DIR` for this bracket:

```bash
# Baseline 1, from the prepared pre-fix worktree.
uptime
CARGO_TARGET_DIR=<shared-absolute-target> cargo bench -p darkmatter --bench clean_hot_paths -- --save-baseline fwl-review2-baseline-1
uptime

# Candidate, from the fix worktree, on the same host and in the same session.
uptime
CARGO_TARGET_DIR=<shared-absolute-target> cargo bench -p darkmatter --bench clean_hot_paths -- --baseline fwl-review2-baseline-1
uptime

# Baseline 2, back in the prepared pre-fix worktree.
uptime
CARGO_TARGET_DIR=<shared-absolute-target> cargo bench -p darkmatter --bench clean_hot_paths -- --baseline fwl-review2-baseline-1
uptime
```

Record every per-fixture Criterion median and calculate each verdict independently:

- **Baseline drift:** for all eight cases,
  `abs(median_baseline_2 - median_baseline_1) / median_baseline_1 <= 0.03`; otherwise void the
  complete bracket.
- **B1:** for each of `prose`, `flat_list`, `nested_list`, and `blockquoted_tasks`,
  `median_candidate_default <= 1.10 * median_baseline_1_default`.
- **B2:** for each list-heavy fixture (`flat_list`, `nested_list`, and `blockquoted_tasks`),
  `median_candidate_fixed_width <= 1.15 * median_baseline_1_fixed_width`.
- **B3:** for each of the four candidate fixtures,
  `median_candidate_fixed_width < 2.00 * median_candidate_default`.

All drift checks and all B1, B2, and B3 cases must pass. Do not aggregate fixtures, substitute
means for Criterion medians, or accept a partial vector as the AC 15 verdict.

## Review 3 — H5 re-evaluation

### What this maps back to

- **Finding:** "High — Required performance timing evidence is still deferred" in
  [`review-3.md`](review-3.md).
- **Exact implementation-log title:**
  `H5 — Required performance timing evidence is still deferred`.
- **Specification:** [`spec.md`](spec.md) → Test Plan → "Performance regression" and Acceptance
  Criterion 15.
- **Implementation log:** [`log.md`](log.md) → "Implementation of Review Findings #3" → H5.

### Review 3 status and host evidence

The Review 3 suggestion was evaluated on 2026-07-20. The load-independent parse-count selector
passed all 8 tests, proving that default cleanup and every indent/spacing variant use one parse,
standalone reflow uses one parse, and the CLI fixed-width sequence uses exactly two parses. The
Criterion harness smoke command also passed all 12 cases (the four general cleanup cases and all
eight `clean_list_budgets` fixture/mode cases):

```bash
cargo nextest run -p darkmatter -E 'test(/parse_count/)' --no-tests=fail --color=never
cargo bench -p darkmatter --bench clean_hot_paths -- --test --noplot
```

The normative timing measurement remains deferred. `sniff hardware --json` identified the host as
an Apple M4 Max with 16 physical and logical cores and 128 GiB memory. At 18:24 local, `uptime`
reported 8 users and load averages of `36.66 49.13 50.30`; at 18:26 it reported 8 users and
`82.57 63.00 55.61`. The 1-minute load was therefore 18-41 times the documented ceiling of 2.0.
AC power was attached, but another Cargo process was also active. These conditions violate three
admissibility requirements: quiet load, no other agent sessions, and no concurrent Cargo work.

The 10% B1 budget is smaller than the scheduler noise reasonably expected under this level of
contention. No baseline or candidate timing samples were started, and no medians, estimates,
deltas, or B1/B2/B3 verdicts were recorded. The shared worktree and existing Criterion baseline
data were left untouched.

### Exact quiet-host Review 3 procedure

Use the pre-fix baseline ref and workarounds documented above. Before starting, require a 1-minute
load average at or below 2.0, no other agent sessions, no concurrent Cargo builds, AC power, and
Criterion's default sampling. Use one new absolute `CARGO_TARGET_DIR` shared only by the prepared
baseline worktree and candidate worktree for this bracket:

```bash
# Baseline 1, from the prepared 96c6616e9 baseline worktree.
uptime
CARGO_TARGET_DIR=<new-shared-absolute-target> cargo bench -p darkmatter --bench clean_hot_paths -- --save-baseline fwl-review3-baseline-1
uptime

# Candidate, from the Review 3 fix worktree on the same host and in the same session.
uptime
CARGO_TARGET_DIR=<new-shared-absolute-target> cargo bench -p darkmatter --bench clean_hot_paths -- --baseline fwl-review3-baseline-1
uptime

# Baseline 2, back in the prepared pre-fix worktree.
uptime
CARGO_TARGET_DIR=<new-shared-absolute-target> cargo bench -p darkmatter --bench clean_hot_paths -- --baseline fwl-review3-baseline-1
uptime
```

Record the Criterion median for every case in every run, then apply these independent verdicts:

- **Baseline drift:** for all eight cases,
  `abs(median_baseline_2 - median_baseline_1) / median_baseline_1 <= 0.03`; otherwise void the
  complete bracket.
- **B1:** for each of `prose`, `flat_list`, `nested_list`, and `blockquoted_tasks`,
  `median_candidate_default <= 1.10 * median_baseline_1_default`.
- **B2:** for each list-heavy fixture (`flat_list`, `nested_list`, and `blockquoted_tasks`),
  `median_candidate_fixed_width <= 1.15 * median_baseline_1_fixed_width`.
- **B3:** for each of the four candidate fixtures,
  `median_candidate_fixed_width < 2.00 * median_candidate_default`.

All eight drift checks, all four B1 cases, all three B2 cases, and all four B3 cases must pass.
Do not aggregate fixtures, substitute means for Criterion medians, or accept a partial vector as
the AC 15 verdict.

## Review 5 — High performance-evidence finding re-evaluation

### What this maps back to

- **Finding:** "High — Required performance timing evidence remains deferred" in
  [`review-5.md`](review-5.md).
- **Exact implementation-log title:**
  `High — Required performance timing evidence remains deferred`.
- **Specification:** [`spec.md`](spec.md) → Test Plan → "Performance regression" and Acceptance
  Criterion 15.
- **Implementation log:** [`log.md`](log.md) → "Implementation of Review Findings #5" → the
  finding started at 22:31:16 local time.

### Review 5 status and host evidence

The Review 5 suggestion was evaluated on 2026-07-20. The load-independent parse-count selector
passed all 8 tests, proving that default cleanup and every indent/spacing variant use one parse,
standalone reflow uses one parse, and the CLI fixed-width sequence uses exactly two parses. The
Criterion harness smoke command passed all 12 cases (the four general cleanup cases and all eight
`clean_list_budgets` fixture/mode cases):

```bash
cargo nextest run -p darkmatter -E 'test(/parse_count/)' --no-tests=fail --color=never
cargo bench -p darkmatter --bench clean_hot_paths -- --test --noplot
```

Criterion's `--test` mode executes each benchmark once to verify harness discovery and behavior;
it does not collect timing samples and is not performance evidence.

The normative timing measurement remains deferred. `sniff hardware --json` identified the host as
an Apple M4 Max with 16 physical and logical cores and 128 GiB memory. At 22:31 local, `uptime`
reported 8 users and load averages of `40.35 44.01 36.93`. The one-minute load was therefore more
than 20 times the documented ceiling of 2.0. `pmset -g batt` confirmed AC power was attached, and
exact-name process checks found no `cargo`, `rustc`, or `sccache` process. However, 9 `codex` and 5
`claudine` processes were active, so the quiet-load, single-agent, and logged-in-session gates
failed.

No fresh shared `CARGO_TARGET_DIR` was allocated and no baseline or candidate timing sample was
started. Consequently, Review 5 records no medians, estimates, deltas, baseline-drift results, or
B1/B2/B3 verdicts. Existing `target/criterion` data was not treated as a fresh bracket and was left
untouched.

### Exact quiet-host Review 5 procedure

First require every admissibility condition to pass: one-minute load average at or below 2.0
immediately before and after every run, one logged-in session with no other agent session, no
`cargo`/`rustc`/`sccache` process, AC power, and Criterion's default profile and sampling. Prepare a
detached baseline worktree at `96c6616e9`, check out `biscuit-file/` from the `darkmatter` branch,
declare the `clean_hot_paths` bench target in the baseline `darkmatter/lib/Cargo.toml`, and copy the
candidate `clean_hot_paths.rs` into the baseline tree as documented under "Pre-fix baseline state."

Create one new absolute target directory used only by the baseline and candidate trees for this
complete bracket:

```bash
# Baseline 1, from the prepared 96c6616e9 baseline worktree.
uptime
CARGO_TARGET_DIR=<new-shared-absolute-target> cargo bench -p darkmatter --bench clean_hot_paths -- --save-baseline fwl-review5-baseline-1
uptime

# Candidate, from the Review 5 fix worktree on the same host and in the same session.
uptime
CARGO_TARGET_DIR=<new-shared-absolute-target> cargo bench -p darkmatter --bench clean_hot_paths -- --baseline fwl-review5-baseline-1
uptime

# Baseline 2, back in the prepared pre-fix worktree.
uptime
CARGO_TARGET_DIR=<new-shared-absolute-target> cargo bench -p darkmatter --bench clean_hot_paths -- --baseline fwl-review5-baseline-1
uptime
```

Record the Criterion median for every case in every run, then apply these independent verdicts:

- **Baseline drift:** for all eight cases,
  `abs(median_baseline_2 - median_baseline_1) / median_baseline_1 <= 0.03`; otherwise void the
  complete bracket.
- **B1:** for each of `prose`, `flat_list`, `nested_list`, and `blockquoted_tasks`,
  `median_candidate_default <= 1.10 * median_baseline_1_default`.
- **B2:** for each list-heavy fixture (`flat_list`, `nested_list`, and `blockquoted_tasks`),
  `median_candidate_fixed_width <= 1.15 * median_baseline_1_fixed_width`.
- **B3:** for each of the four candidate fixtures,
  `median_candidate_fixed_width <= 2.00 * median_candidate_default`.

All eight drift checks, all four B1 cases, all three B2 cases, and all four B3 cases must pass.
Do not aggregate fixtures, substitute means for Criterion medians, or accept a partial vector as
the AC 15 verdict.

## Review 4 — High finding 5 re-evaluation

### What this maps back to

- **Finding:** "High — Required performance timing evidence remains deferred" in
  [`review-4.md`](review-4.md).
- **Exact implementation-log title:**
  `High — Required performance timing evidence remains deferred`.
- **Specification:** [`spec.md`](spec.md) → Test Plan → "Performance regression" and Acceptance
  Criterion 15.
- **Implementation log:** [`log.md`](log.md) → "Implementation of Review Findings #4" → the
  finding started at 21:16:43 local time.

### Review 4 status and host evidence

The Review 4 suggestion was evaluated on 2026-07-20. The load-independent parse-count selector
passed all 8 tests, proving that default cleanup and every indent/spacing variant use one parse,
standalone reflow uses one parse, and the CLI fixed-width sequence uses exactly two parses. The
Criterion harness smoke command passed all 12 cases (the four general cleanup cases and all eight
`clean_list_budgets` fixture/mode cases):

```bash
cargo nextest run -p darkmatter -E 'test(/parse_count/)' --no-tests=fail --color=never
cargo bench -p darkmatter --bench clean_hot_paths -- --test --noplot
```

Criterion's `--test` mode executes each benchmark once to verify harness discovery and behavior;
it does not collect timing samples and is not performance evidence.

The normative timing measurement remains deferred. `sniff hardware --json` identified the host as
an Apple M4 Max with 16 physical and logical cores and 128 GiB memory. At 21:18 local, `uptime`
reported 9 users and load averages of `86.31 84.66 65.35`. The 1-minute load was therefore 43
times the documented ceiling of 2.0. AC power was attached and no concurrent Cargo or Rust compiler
process was visible, but another agent session was active. These conditions violate the quiet-load,
single-agent, and logged-in-session admissibility requirements.

The 10% B1 budget is far smaller than the scheduler noise reasonably expected under this level of
contention. No baseline or candidate timing samples were started, and no medians, estimates,
deltas, or B1/B2/B3 verdicts were recorded. The baseline worktree and existing Criterion result
data were left untouched.

### Exact quiet-host Review 4 procedure

Use the pre-fix baseline ref and workarounds documented above. Before starting, require every host
condition to pass: 1-minute load average at or below 2.0 immediately before and after every run,
no other agent sessions, no concurrent Cargo builds, AC power, and Criterion's default sampling.
Use one new absolute `CARGO_TARGET_DIR` shared only by the prepared baseline worktree and candidate
worktree for this bracket:

```bash
# Baseline 1, from the prepared 96c6616e9 baseline worktree.
uptime
CARGO_TARGET_DIR=<new-shared-absolute-target> cargo bench -p darkmatter --bench clean_hot_paths -- --save-baseline fwl-review4-baseline-1
uptime

# Candidate, from the Review 4 fix worktree on the same host and in the same session.
uptime
CARGO_TARGET_DIR=<new-shared-absolute-target> cargo bench -p darkmatter --bench clean_hot_paths -- --baseline fwl-review4-baseline-1
uptime

# Baseline 2, back in the prepared pre-fix worktree.
uptime
CARGO_TARGET_DIR=<new-shared-absolute-target> cargo bench -p darkmatter --bench clean_hot_paths -- --baseline fwl-review4-baseline-1
uptime
```

Record the Criterion median for every case in every run, then apply these independent verdicts:

- **Baseline drift:** for all eight cases,
  `abs(median_baseline_2 - median_baseline_1) / median_baseline_1 <= 0.03`; otherwise void the
  complete bracket.
- **B1:** for each of `prose`, `flat_list`, `nested_list`, and `blockquoted_tasks`,
  `median_candidate_default <= 1.10 * median_baseline_1_default`.
- **B2:** for each list-heavy fixture (`flat_list`, `nested_list`, and `blockquoted_tasks`),
  `median_candidate_fixed_width <= 1.15 * median_baseline_1_fixed_width`.
- **B3:** for each of the four candidate fixtures,
  `median_candidate_fixed_width <= 2.00 * median_candidate_default`.

All eight drift checks, all four B1 cases, all three B2 cases, and all four B3 cases must pass.
Do not aggregate fixtures, substitute means for Criterion medians, or accept a partial vector as
the AC 15 verdict.
