---
$schema: feature-review.yaml
ready: false
findings:
  - priority: medium
    title: Regression width sweeps make two ordinary L1 tests slow
human_review: false
reviewed_by: codex/gpt-5.6-sol
created: 2026-09-15T19:47:10-07:00
spec: corrected-perf-flag/spec.md
implemented: false
description: "A **fix** review of `corrected-perf-flag/spec.md`"
fix: corrected-perf-flag/review-2.md
previous: corrected-perf-flag/review-1.md
---

# Review 2 — Corrected Perf Flag

## Verdict

The fix is **not ready for production**. Review 1's two high-severity findings
are correctly implemented: `MetricsTree` now escapes literal labels after
truncation and padding, and Sniff has Level 2 coverage through a real tmux pane
for hierarchy, styling, alignment, wrapping, and narrow-width truncation. The
full Sniff and `biscuit-terminal` Level 1 suites, the required tmux Level 2
tier, both lint gates, and the graph-identified Claudine and Worktree perf
suites pass.

No product-correctness or verification-level gap remains. One test-efficiency
issue still blocks readiness under the review criteria: each of the two new
Level 1 width-sweep regressions takes more than seven seconds despite the
recorded failure being confined to a much smaller width band. Both tests are
ordinary, unprefixed tests, so they run in the fast `sanity` tier. No human
review is required; the remediation is mechanical and preserves the same
behavioral proof.

## Prior Review Closure

- **Narrow terminals corrupt underscore-heavy metric rows — closed.**
  `MetricsTree::build_markup` escapes the already-truncated and padded literal
  label before Prose parses it
  ([`metrics_tree.rs:323`](../../../biscuit-terminal/lib/src/components/metrics_tree.rs#L323)).
  The component regression uses the original underscore/ellipsis failure
  shape, while the Sniff regression uses the production timing and counter key
  sets. Claudine and Worktree, the two graph-resolved downstream consumers,
  retain green perf suites.
- **The new terminal rendering has no Level 2 verification — closed.** The new
  tmux target is registered behind `test-fixtures`
  ([`Cargo.toml:71`](../../cli/Cargo.toml#L71)) and checks the real pane's
  hierarchy, separate trees, SGR styling, display-column alignment, wrapping,
  and truncation behavior
  ([`level2_perf_tree_rendering.rs:333`](../../cli/tests/level2_perf_tree_rendering.rs#L333),
  [`level2_perf_tree_rendering.rs:483`](../../cli/tests/level2_perf_tree_rendering.rs#L483)).
  The review reran the tier with tmux required; backend proof recorded four
  real tmux executions, including both perf tests.
- Review 1 had no `## Blocked Findings` section and no deferred finding, so
  there was nothing to reclassify before this implementation cycle.

## Findings

### Medium — Regression width sweeps make two ordinary L1 tests slow

Both new Level 1 regressions render every width from 20 through 140 in both
Unicode modes
([`metrics_tree.rs:715`](../../../biscuit-terminal/lib/src/components/metrics_tree.rs#L715),
[`perf_tree.rs:899`](../../cli/src/output/perf_tree.rs#L899)). That is 242
renders per test. In this review's warm Nextest execution, the component test
took 7.404 seconds and the Sniff production-key test took 7.557 seconds. The
repository's test contract classifies tests over five seconds as `slow_`, while
these ordinary names place both tests in `sanity`, the fast-confidence tier.

The breadth is not buying equivalent regression evidence. The implementation
and both test comments identify columns 27 through 48 as the corrupt band; the
93 widths from 49 through 140 per glyph mode repeat the already-clean case.
Reduce the Level 1 matrices to the known failing band, optionally retaining one
minimum-width and one first-clean boundary case. Keep an explicit assertion
that at least one render actually truncates immediately after an underscore;
the Sniff test already has that guard, and the component test should use either
the same guard or a pinned original failing width. This keeps the regression in
the mandatory fast tier and should reduce each test to roughly one fifth of its
current work. Do not shrink the 30-through-50 Level 2 sweep: that separate cost
proves terminal-emulator behavior and is already isolated to the L2 tier.

Strongest verification present: Level 1 exhaustive synthetic-terminal sweeps
and Level 2 real-tmux coverage. The boundary is correct and the assertions
distinguish the defect; the remaining problem is unnecessary work in the fast
Level 1 tier.

## Requirement Verification Levels

| Requirement | Strongest verification present | Assessment |
|---|---|---|
| R1 — Collection, worker propagation, and request isolation remain intact | Level 1 library unit/integration tests in the full Sniff suite | Correct level; all remain green and the collector seam is unchanged. |
| R2.1 — Root, dotted hierarchy, aliases, values, calls, and ordering | Level 1 pure projection tests plus Level 2 hierarchy capture | Correct level; projection semantics and terminal presentation are both covered. |
| R2.2 — Wall-clock shares and overlap note | Level 1 calculation tests plus Level 2 italic-note capture | Correct level. |
| R2.3 — Exactly one deterministic HOT measured stage | Level 1 selection tests plus Level 2 SGR/HOT-row capture | Correct level. |
| R3 — Separate generic counter tree with count ordering and unknown shares | Level 1 projection/CLI tests plus Level 2 separate-tree and alignment capture | Correct level. |
| R4 — stdout/stderr routing, valid JSON stdout, and ANSI-free `--plain` output | Level 1 real-binary subprocess tests | Correct level; these are stream/data contracts, not terminal-emulator behavior. |
| R2/R4 — Width handling, glyphs, SGR styling, alignment, and terminal degradation | Level 2 tmux pane capture, including a 30–50-column sweep | Correct level; review 1's gap is closed. |
| Keyboard, hotkey, paste, IME, or mouse behavior | No changed requirement | Level 3 is not applicable. |

## Validation Performed

- `CARGO_TARGET_DIR=<isolated-temp-dir> BISCUIT_TEST_REQUIRED_BACKENDS=tmux just test-l2`
  passed: 4 tests run, 4 passed, 852 skipped; backend proof recorded
  `tmux run=4 skip=0 panic=0`. Both new perf tests ran.
- `CARGO_TARGET_DIR=<isolated-temp-dir> just test` in `biscuit-terminal`
  passed: 3,262 tests, 0 failed, 55 tier skips. The new component regression
  took 7.404 seconds.
- `CARGO_TARGET_DIR=<isolated-temp-dir> just test` in `sniff` passed: 2,661
  tests, 0 failed, 24 tier skips. The new production-key regression took 7.557
  seconds.
- `CARGO_TARGET_DIR=<isolated-temp-dir> just lint` passed in both
  `biscuit-terminal` and `sniff`.
- Targeted graph-derived downstream verification passed: 88 perf tests across
  `claudine-cli` and `worktree-cli`, 0 failed.
- GitNexus reported HIGH upstream impact for `build_markup`: three direct
  renderer entry points and two indirect CLI consumers. Its index was 56
  commits behind HEAD, so source search and the downstream test run were used
  to confirm the affected consumers. Worktree-wide change detection reported
  low aggregate risk with no affected indexed process.
- `git diff --check` passed after the review metadata edits.
- The first L2 attempt against the shared Cargo target failed because cached
  `.rmeta` files were not writable. The isolated-target rerun above completed
  successfully; this was an environment/cache-permission issue, not a product
  failure.

## Production Readiness

Not ready. The rendering fix is correct and now has the required Level 2 proof,
but the two oversized Level 1 regression matrices should be narrowed before
the fix is considered production-ready.
