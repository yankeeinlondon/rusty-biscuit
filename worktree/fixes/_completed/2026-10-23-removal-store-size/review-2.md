---
$schema: feature-review.yaml
reviewed_by: codex/gpt-6.1-sol
created: "2026-10-23T15:58:22-07:00"
spec: 2026-10-23-removal-store-size/spec.md
fix: 2026-10-23-removal-store-size/review-2.md
previous: 2026-10-23-removal-store-size/review-1.md
next: 2026-10-23-removal-store-size/review-3.md
implemented: true
description: "A **fix** review of `2026-10-23-removal-store-size/spec.md`"
ready: false
findings:
    - "R1-F2 (medium): graph L1 tests exceed the five-second L1 time budget"
human_review: true
has_blocked_findings: false
blocked: false
recurrence: true
log: /Volumes/coding/wt/rusty-biscuit/feat-merman/worktree/fixes/2026-10-23-removal-store-size/log.md
implemented_by: claude/opus
human_review_items:
    - "R1-F2: macOS-only nextest setup script sets CFProcessPath=/usr/bin/true for the 11 font-measuring worktree-cli tests, instead of target cleanup (insufficient, regrows) or serializing them (still overran)"
---

# Review 2: removal store size

The storage behavior meets the specification, and the journal diagnostic finding is verified fixed. Readiness remains false because the normal package test run still exceeds the required five-second L1 budget for 11 graph tests. This is the remaining instance of R1-F2, not a newly discovered sibling. The repair materially improves execution and identifies a macOS build-directory cause, but its claim that every required instance is fixed is not supported by the normal run.

## Previous Finding Verification

The required instances below come from the final Finding Ledger in `log.md`. Both original findings were unblocked; neither requires a new human decision.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| R1-F1: unreadable-journal diagnostics omit the journal filename and disagree on exit class | R1-F1.1: current and archived journal filenames; R1-F1.2: lookup and lifecycle exit classification; R1-F1.3: other CLI store-error conversions; R1-F1.4: other recovery read diagnostics | verified fixed | The worktree library's [journal-generation reader](../../lib/src/remove/recovery/store.rs) passes each actual path into `StoreError::Journal`. Its public recovery test checks both current and archived files. The shipped-CLI tests verify the archived filename, current malformed journal, unsupported version, exit 3, and preservation of the checkout. The worktree library's [StoreError::exit_class](../../lib/src/remove/lifecycle.rs) supplies both lookup and lifecycle classifications; the CLI test checks all 12 variants against explicit expected codes. Searches confirm both CLI conversions use the shared mapping. Report/storage errors retain their paths; record errors identify the operation's sole record; inventory mismatch is not a file-read error. All relevant tests passed in the fresh package run. | None. |
| R1-F2: graph L1 tests exceed the five-second L1 time budget | R1-F2.1: all graph/layout tests, formerly duplicated in both targets; R1-F2.2: three 100,000-entry journal/storage tests; R1-F2.3: removal tests previously slow under load | partially fixed | The worktree CLI's [library-only test macro](../../cli/src/lib.rs) keeps the graph/list tests in a declared library target and removes their duplicate binary copies. All 11 layout tests still took 9.484–12.368 s in `just test`. The same current test executable, copied into a small temporary directory and run in 11 concurrent processes, passed each test in 1.687–4.189 s. The three large storage tests passed in 2.171, 2.691, and 3.021 s. The full run marked no removal test slow. | R1-F2.1 remains: demonstrate the affected tests within budget through the ordinary recipe after addressing the local build-directory condition. R1-F2.2 and R1-F2.3 are verified fixed/clean. |

## Unblocked Findings

### R1-F2 (medium): graph L1 tests exceed the five-second L1 time budget

**Authority: spec and review testing contract.** The specification requires “`just test` and `just lint` pass in `worktree/`, with no flaky test,” and this review's explicit testing rule says “An L1 test over 5 s counts as failing.” The fresh normal recipe reports 11 such tests. This carries the original finding without expanding its contract.

The defect class is L1 execution exceeding its required time budget because independent graph tests repeatedly initialize system-font discovery from the large executable directory.

The implementation log and the macOS skill attribute the delay to CoreText enumerating the executable's directory. The controlled rerun supports that explanation: copying the executable, without changing test assertions, brings every affected test under five seconds even when all 11 run concurrently. This is an unusual local build condition, so the finding remains medium. It does not establish a storage correctness defect or a failure on another OS.

The following table sweeps every affected layout test and both other ledger groups. Test names identify the cases in the worktree CLI's [graph tests](../../cli/src/commands/git_graph/tests.rs) and [list tests](../../cli/src/commands/list/tests.rs). All listed cases passed their assertions. Each graph row's expected result is at most five seconds in the ordinary L1 run; the temporary-directory result is a diagnostic control, not a replacement for that run.

| Child ID | Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- | --- |
| R1-F2.1.1 | `gathered_graph_lays_out_continued_after_merge_at_merge` | Existing graph fixture, ordinary executable directory / copied executable | 11.360 s / 3.253 s | Ordinary run ≤5 s |
| R1-F2.1.2 | `gathered_graph_lays_out_continued_after_merge_behind` | Same control comparison | 10.708 s / 2.421 s | Ordinary run ≤5 s |
| R1-F2.1.3 | `gathered_graph_lays_out_continued_after_merge_diverged` | Same control comparison | 10.812 s / 2.393 s | Ordinary run ≤5 s |
| R1-F2.1.4 | `gathered_graph_lays_out_long_labels` | Same control comparison | 10.837 s / 2.376 s | Ordinary run ≤5 s |
| R1-F2.1.5 | `gathered_graph_lays_out_merged_twice` | Same control comparison | 10.583 s / 1.917 s | Ordinary run ≤5 s |
| R1-F2.1.6 | `gathered_graph_lays_out_new_branch_at_merged_tip` | Same control comparison | 10.614 s / 1.984 s | Ordinary run ≤5 s |
| R1-F2.1.7 | `gathered_graph_lays_out_observation_1` | Same control comparison | 10.704 s / 2.144 s | Ordinary run ≤5 s |
| R1-F2.1.8 | `gathered_graph_lays_out_observation_2` | Same control comparison | 10.917 s / 2.502 s | Ordinary run ≤5 s |
| R1-F2.1.9 | `gathered_graph_lays_out_sparse_lanes` | Same control comparison | 12.368 s / 4.189 s | Ordinary run ≤5 s |
| R1-F2.1.10 | `the_observed_graph_keeps_recent_commits_on_every_lane_at_200x60` | Same control comparison | 11.316 s / 3.218 s | Ordinary run ≤5 s |
| R1-F2.1.11 | `run_pipeline_gathers_the_graph_on_a_narrow_image_terminal` | Same control comparison | 9.484 s / 1.687 s | Ordinary run ≤5 s |
| R1-F2.2 | Three `journal_size` tests: completed-folder ratio, v3 replay, v3/v2 batch-size comparison | Existing 100,000-entry fixtures in ordinary recipe | 2.171 / 2.691 / 3.021 s; clean | Each ≤5 s |
| R1-F2.3 | Removal tests previously reported slow | Existing removal cases in full ordinary recipe | None marked slow; clean | Each ≤5 s |

The original binary-target graph/list instances are also clean: they no longer compile duplicate tests, while the library target still runs the original cases. The tier-coverage check verifies that retained tests are selected.

**Smallest repair:** address the stale build-artifact condition through targeted, safe build-directory maintenance, then record an ordinary affected-test run within budget. A broad deletion of build output, another graph abstraction, or a custom test launcher would be disproportionate to the evidence. The copied-executable control does not justify changing graph behavior or weakening tests. If targeted maintenance resolves the ordinary run, this finding needs no further production code.

## Blocked Findings

None. No human approval or cross-OS result is needed to continue the remaining verification.

## Recurrence

This is the unchanged R1-F2.1 instance of R1-F2 in `review-1.md`, not a regression caused by the repair and not a sibling missed by the previous sweep. The repair checked the affected graph/list tests, the three large storage tests, and the previously slow removal tests. Its remaining gap is treating a successful copied-executable experiment as closure while the ordinary recipe still exceeds the budget. This review enumerates all 11 remaining graph/list cases so another cycle need not rediscover them.

## Requirement verification

All storage and diagnostic requirements need L1 verification: real temporary files, local Git repositories, public library results, and shipped-CLI subprocesses. This change adds no terminal input behavior or visual layout contract requiring Level 2 or Level 3 verification. The graph finding concerns execution time of existing L1 tests.

| Requirement | Verification reviewed and executed |
| --- | --- |
| Silent startup retention, 14-day completion clock, pending preservation, nonwaiting admin-name locks | Public-store mixed-age, boundary, fallback, link, lock, and interrupted-deletion tests; CLI cleanup before failed name resolution and during handoff; list preservation control. |
| Compact version 6 reports and atomic compressed full entries | Public save/read tests compare metadata and decompressed entries; stop, resume, no-entry and failed-summary cases; publication-failure tests preserve complete files and journals. The buffered writer still finishes gzip, flushes, closes, and publishes in that order. |
| Version 5 reports and version 2 journals resume with version 3 attempts | Lifecycle migration cases exercise saved metadata plus current/archived journals, repeated stops, attempt order, uncertainty, complete entries, and final journal removal. |
| Indexed batches preserve validation and fit their byte limit | Public recovery matrices and writer tests cover both versions, inventory-position bounds, ordered result counts, duplicate planning, same-directory and batch-size constraints, torn writes, and worst-case singleton/digit-boundary limits. |
| Completed 100,000-entry folder occupies under 5% of the old layout | L1 test measures all retained files against the frozen version 5 report shape and checks complete decompressed entries. Both large journal read/size cases also pass within budget. |
| Current documentation | CLI topic page and worktree skill describe retention, sidecar publication, both journal versions, and updated journal-error filenames and exit 3. |

No new functional finding was identified in these requirements. Earlier decisions on retention tombstones, permissive timestamp-only retention reads, two-file publication order, and legacy format compatibility remain accepted.

## Input robustness

The changed loading paths are the versioned report loader and journal readers; retention has its separate timestamp reader. The default invalid-input outcome remains a located error before recovery mutation. Optional earlier-report failure instead warns and resumes from valid journals; retention falls back to folder time when report timing cannot be read. Empty entry lists and explicit null sidecar references retain their defined meanings.

Reviewed the public-store controls and matrices for required scalar fields, missing/null/wrong-type/empty values, mixed and wholly invalid collection elements, duplicate keys, unknown members, trailing data, position bounds, result alignment, and both version branches. Shared inventory validation is tested once with version-specific wiring. A final unterminated journal line is intentionally ignored as torn; a malformed terminated line fails. The gzip sidecar is a writer-only audit artifact and has no production loader.

The smell search found no new permissive field default or silently discarded invalid element. The position conversion's `.ok()` leads to an explicit error, the tail's `unwrap_or_default()` implements torn-write handling, and retention's `.ok()` implements its specified fallback. The diagnostic repair changes error wrapping and classification rather than field validation; its tests prove that the existing invalid inputs now name the actual journal and use the same exit class through both consumers.

## Validation

- `just test` in `worktree/`: exit 0; 1,560 passed, 14 skipped, 11 slow; 39.451 s execution. Assertions pass, but the 11 timings keep R1-F2 open.
- Copied current CLI library test executable: all 11 affected tests passed in separate concurrent processes, 1.687–4.189 s. This isolates the executable-directory effect without changing assertions.
- `just lint` in `worktree/`: passed for library and CLI.
- `just check-tier-coverage worktree`: passed, zero stranded tests.
- Review metadata, preserved previous implementation claims, and whitespace checks on the review files/spec passed. A wider diff check also encountered preexisting trailing whitespace in the repair log; that file was left unchanged.
- Cross-OS evidence is left to CI and does not block this review.

No production files were changed by this review. The previous review's implementation claim is preserved; this review's verification table records the differing assessment of R1-F2.
