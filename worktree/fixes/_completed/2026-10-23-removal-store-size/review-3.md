---
$schema: feature-review.yaml
ready: false
findings:
    - "R1-F2 (medium): graph L1 tests exceed the five-second L1 time budget"
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: true
created: "2026-10-23T16:22:40-07:00"
spec: 2026-10-23-removal-store-size/spec.md
implemented: true
description: "A **fix** review of `2026-10-23-removal-store-size/spec.md`"
fix: 2026-10-23-removal-store-size/review-3.md
previous: 2026-10-23-removal-store-size/review-2.md
next: 2026-10-23-removal-store-size/review-4.md
log: /Volumes/coding/wt/rusty-biscuit/feat-merman/worktree/fixes/2026-10-23-removal-store-size/log.md
implemented_by: claude/opus
---

# Review 3: removal store size

The storage functionality meets the specification. Readiness remains false because one existing graph test still exceeded the required five-second L1 budget in an ordinary package run. R1-F2 is partially fixed: ten previously affected tests stayed within budget, while the sparse-lane test took 5.403 seconds. A second ordinary run passed within budget, which shows improvement but does not erase the first result. No new functional finding was identified.

## Previous Finding Verification

Read both earlier reviews, the original implementation log, and both repair Finding Ledgers in `log.md`. The table verifies their claims against source and fresh execution. All previous findings were unblocked. Earlier accepted storage and compatibility decisions remain accepted.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| R1-F1: unreadable-journal diagnostics omit the journal filename and disagree on exit class | R1-F1.1 current and archived filenames; R1-F1.2 lookup and lifecycle exit classes; R1-F1.3 other CLI conversions; R1-F1.4 other recovery diagnostics | verified fixed | The worktree library's [generation reader](../../lib/src/remove/recovery/store.rs) retains each current/archive path in `StoreError::Journal`. The public-store filename test and shipped-CLI current/archive refusal cases pass. The library's [exit classification](../../lib/src/remove/lifecycle.rs) is shared by both CLI conversions; the explicit 12-variant classification test passes in both compiled targets. Report/storage errors retain paths; record errors identify the operation; inventory mismatch is not a read error. | None. |
| R1-F2: graph L1 tests exceed the five-second L1 time budget | R1-F2.1.1–R1-F2.1.11 graph/list cases; R1-F2.2 three large storage tests; R1-F2.3 previously slow removal tests; R1-F2.4 other font-measuring L1 sites | partially fixed | The macOS setup script executes in both ordinary runs. All assertions pass. Ten graph/list cases stay under five seconds in both runs, but R1-F2.1.9 takes 5.403 s in the first and 3.672 s in the second. The three large storage tests take 2.350/3.993/3.984 s and 1.915/2.501/2.667 s. No removal test is marked slow. Source search confirms the remaining font-measuring L1 sites are the enumerated graph/list cases; retained library tests are selected and binary duplicates remain removed. | R1-F2.1.9 still needs enough runtime margin in the ordinary package run. All other enumerated instances are verified fixed or clean. |

## Unblocked Findings

### R1-F2 (medium): graph L1 tests exceed the five-second L1 time budget

**Authority: spec and review testing contract.** The specification requires “`just test` and `just lint` pass in `worktree/`, with no flaky test.” The review's explicit rule states “An L1 test over 5 s counts as failing.” The ordinary `just test` run still reports one test beyond that budget. This carries an existing required instance, without adding a stronger storage guarantee.

The defect class is an L1 layout test whose runtime still crosses the required budget under ordinary package-suite contention.

The repair's macOS-only [nextest setup script](../../../.config/nextest.toml) sets `CFProcessPath=/usr/bin/true` for these font-measuring tests. It ran successfully, and the timings confirm a large improvement over the previous review. The remaining overrun is in the worktree CLI's [sparse-lane graph test](../../cli/src/commands/git_graph/tests.rs:2016), which gathers a temporary repository and verifies measured layout across several viewports. Its assertions pass. This is a timing defect in an existing test, not evidence of incorrect removal behavior or an incorrect graph.

The full bounded sweep follows. Each graph row expects at most five seconds in the ordinary recipe. Durations are from two complete ordinary runs, in order. Child IDs preserve the previous review's repair inventory.

| Child ID | Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- | --- |
| R1-F2.1.1 | `gathered_graph_lays_out_continued_after_merge_at_merge` | Existing fixture through ordinary recipe | 2.809 / 2.039 s; clean | ≤5 s |
| R1-F2.1.2 | `gathered_graph_lays_out_continued_after_merge_behind` | Existing fixture through ordinary recipe | 1.947 / 1.426 s; clean | ≤5 s |
| R1-F2.1.3 | `gathered_graph_lays_out_continued_after_merge_diverged` | Existing fixture through ordinary recipe | 1.948 / 1.467 s; clean | ≤5 s |
| R1-F2.1.4 | `gathered_graph_lays_out_long_labels` | Existing fixture through ordinary recipe | 1.911 / 1.450 s; clean | ≤5 s |
| R1-F2.1.5 | `gathered_graph_lays_out_merged_twice` | Existing fixture through ordinary recipe | 1.540 / 1.119 s; clean | ≤5 s |
| R1-F2.1.6 | `gathered_graph_lays_out_new_branch_at_merged_tip` | Existing fixture through ordinary recipe | 1.563 / 1.176 s; clean | ≤5 s |
| R1-F2.1.7 | `gathered_graph_lays_out_observation_1` | Existing fixture through ordinary recipe | 1.776 / 1.358 s; clean | ≤5 s |
| R1-F2.1.8 | `gathered_graph_lays_out_observation_2` | Existing fixture through ordinary recipe | 2.122 / 1.596 s; clean | ≤5 s |
| R1-F2.1.9 | `gathered_graph_lays_out_sparse_lanes` | Existing fixture through ordinary recipe | **5.403** / 3.672 s; 2.244 s alone | ≤5 s in ordinary run |
| R1-F2.1.10 | `the_observed_graph_keeps_recent_commits_on_every_lane_at_200x60` | Existing fixture through ordinary recipe | 2.869 / 1.926 s; clean | ≤5 s |
| R1-F2.1.11 | `run_pipeline_gathers_the_graph_on_a_narrow_image_terminal` | Existing fixture through ordinary recipe | 1.208 / 0.845 s; clean | ≤5 s |
| R1-F2.2 | Three `journal_size` tests | 100,000-entry completed-size, replay, and encoding-comparison fixtures | 2.350/3.993/3.984 s; 1.915/2.501/2.667 s; clean | Each ≤5 s |
| R1-F2.3 | Removal tests previously slow under load | Complete package L1 suite | None marked slow in either run; clean | Each ≤5 s |
| R1-F2.4 | Other font-measuring L1 sites and duplicate targets | Search of layout calls, library/binary module declarations, and executed tests | Measured graph cases are enumerated above; CLI subprocess fixtures run `target/debug/wt`; terminal/performance cases belong to their own tiers. Graph/list unit tests remain library-only; tier audit passes. | No omitted font-measuring L1 instance or lost test |

The isolated sparse-lane run also executes the setup script and takes 2.244 seconds. That supports contention sensitivity, but the normal recipe is the required verification boundary. The second full run is recorded as a diagnostic confirmation, not substituted for the first run's result.

**Smallest repair:** measure the remaining test's fixture setup and repeated layout/geometry work, then remove redundant work or split independent viewport checks into separately scheduled tests while retaining every assertion and a guard for the case list. The evidence does not establish which stage dominates, so another font-discovery mechanism would be premature. Keep the existing setup improvement. A production rendering redesign would be disproportionate to this residual timing overrun; a timeout increase, weaker assertions, or rerunning until green would leave the finding unresolved.

## Blocked Findings

None. No human decision is required to repair or verify the remaining timing instance. The previous review's request to consider the macOS test-only setup choice does not block this work: the setting is narrowly scoped, executes successfully, and leaves the test assertions intact.

## Recurrence

R1-F2 repeats the class reported in `review-1.md` and carried in `review-2.md`. This is the unchanged R1-F2.1.9 sparse-lane instance, not a new sibling and not a demonstrated regression caused by the repair. The earlier sweep included it; the repair substantially reduces its cost but does not consistently keep it under budget in the ordinary recipe. The table above checks all eleven original graph/list cases plus the three other ledger groups, so no sibling is left for another review to discover.

## Requirement verification

All removal-store requirements need L1 verification through real temporary files, local Git fixtures, public library results, and CLI subprocesses. This fix introduces no keyboard-input or terminal-rendering requirement that needs Level 2 or Level 3 evidence. Existing graph-layout tests are reviewed here for their L1 runtime contract.

| Requirement | Verification reviewed and executed |
| --- | --- |
| Silent startup cleanup; completed operations older than 14 days; pending/recent/locked preservation | Public-store mixed-age, boundary, fallback-time, link, and interrupted-deletion tests pass. CLI cleanup precedes name resolution, covers handoff, compares output with a control, and leaves list read-only. Admin-name locks are nonwaiting, and completion is rechecked under the lock. |
| Version 6 compact reports and complete compressed entries | Save/read tests verify omitted inline entries, metadata, null/no-entry behavior, every entry state, and complete gzip output. Publication fault tests verify complete old/new files and preservation of recovery journals. The buffered writer finishes gzip, flushes, closes, and publishes in order. |
| Stop/resume and legacy format compatibility | Lifecycle and recovery tests verify earlier version 5 reports, version 2 current/archived journals, new version 3 attempts, attempt ordering, uncertainty, and complete final entries. |
| Indexed version 3 batches preserve validation and byte bounds | Public recovery matrices cover both versions, positions, ordered states, batch size, one-directory batches, duplicates within/across batches, and torn writes. Writer tests verify worst-case singleton and digit-boundary limits. |
| 100,000 entries with 150-byte paths leave a folder under 5% of the old layout | Completion test measures all retained files against a frozen version 5 representation and verifies decompressed entry equality. Passes in both full runs. |
| Current documentation | Remove topic page and worktree skill explain compressed entries, retention, journal versions, publication ordering, and journal diagnostics. |
| Package tests and lint | Assertions and lint pass, but the first run's sparse-lane timing remains outside the explicit L1 contract. |

No accepted decision about retention tombstones, timestamp-only retention reads, two-file publication, or compatibility was reopened.

## Input robustness

The loaders remain the versioned report reader, per-line version 2/version 3 journal readers, shared inventory validator, and separate retention timestamp reader. Invalid report/journal shapes produce located errors without recovery mutation. An unreadable optional earlier report instead warns and resumes from valid journals; retention uses folder time when report timing cannot be read. Empty entry lists mean no entries, version 6 explicitly accepts a null entries-file reference, and an unterminated journal tail is intentionally treated as a torn write. The gzip entries file is written for inspection and has no production loader.

Reviewed and executed the public-store positive controls and matrices for missing, null, wrong-type, empty, mixed-element, and wholly invalid collection shapes. Document cases cover duplicate keys, unknown members, trailing content, and both journal versions; version-specific wiring reaches the shared inventory validator. Report-version, inline-entry, and sidecar-name validators have public-result tests, and shipped-CLI cases verify refusal location, exit classification, and preserved checkout state. Retention tests cover both report encodings without requiring an entries file.

The smell search found no new permissive default or silently discarded invalid collection element in the changed readers. Position conversion's `.ok()` leads to an explicit bounds error, the journal tail's `unwrap_or_default()` handles torn writes, and retention's `.ok()` implements its accepted timestamp fallback. Existing optional observation metadata preserves legacy compatibility. The latest repair changes test configuration, not these validators.

## Validation

- First `just test` in `worktree/`: exit 0; 1,560 passed, 14 skipped, **one slow**, 27.349 s test execution. Sparse-lane test: 5.403 s.
- Second ordinary `just test`: exit 0; 1,560 passed, 14 skipped, zero slow, 22.377 s. Sparse-lane test: 3.672 s.
- Isolated sparse-lane test through nextest: passed, 2.244 s; setup script ran.
- `just lint` in `worktree/`: passed for library and CLI.
- `just check-tier-coverage worktree`: passed, zero stranded tests.
- Whitespace check of changed worktree library/CLI code and nextest configuration: passed.
- Full test output is retained at `/tmp/removal-store-review3-tests.log` and `/tmp/removal-store-review3-tests-confirm.log`; isolated output at `/tmp/removal-store-review3-sparse.log`.
- Cross-OS evidence is left to CI and does not block readiness.

This review changes only review metadata, the new review document, and the spec's review-iteration count. Previous implementation claims remain unchanged. No production code or tests were modified.
