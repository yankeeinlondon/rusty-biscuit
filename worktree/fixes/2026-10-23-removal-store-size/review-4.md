---
$schema: feature-review.yaml
ready: true
findings: []
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: "2026-10-23T16:34:45-07:00"
spec: 2026-10-23-removal-store-size/spec.md
implemented: false
description: "A **fix** review of `2026-10-23-removal-store-size/spec.md`"
fix: 2026-10-23-removal-store-size/review-4.md
previous: 2026-10-23-removal-store-size/review-3.md
---

# Review 4: removal store size

The fix is production ready. Both previous findings are verified fixed, including the remaining sparse-lane test timing instance. No new finding was identified. The ordinary package suite passes with no test exceeding the five-second L1 budget, and the ordinary lint recipe passes without suppressions.

## Previous Finding Verification

Checked all three earlier reviews, the repair Finding Ledgers in `log.md`, and the original implementation log. The table records source inspection and fresh execution, rather than accepting completion flags as evidence. No finding remains blocked, and no accepted decision is reopened.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| R1-F1: unreadable-journal diagnostics omit the journal filename and disagree on exit class | R1-F1.1 current and archived filenames; R1-F1.2 lookup and lifecycle exit classes; R1-F1.3 other CLI conversions; R1-F1.4 other recovery diagnostics | verified fixed | The worktree library's [generation reader](../../lib/src/remove/recovery/store.rs) wraps each parse error with the actual current or archived filename. The public-store current/archive test and shipped-CLI refusal tests pass. Both CLI conversions use the library's [shared exit classification](../../lib/src/remove/lifecycle.rs); its explicit 12-variant test passes in both compiled targets. Storage/report errors carry paths, record errors identify the operation, and inventory mismatch is not a file-read error. | None. |
| R1-F2: graph L1 tests exceed the five-second L1 time budget | R1-F2.1.1–R1-F2.1.11 graph/list cases, including both replacements for R1-F2.1.9; R1-F2.2 three large storage tests; R1-F2.3 removal cases; R1-F2.4 other font-measuring L1 sites and duplicate targets | verified fixed | Every listed instance runs within budget in the ordinary `just test` run. The two sparse-lane tests take 2.118 and 2.446 s. The viewport guard is compiled and runs. Source inspection confirms both widths remain covered, other fixtures still check both widths, and graph/list tests remain library-only. The bounded timing sweep below checks every ledger group. | None. The previously partial repair is now complete; no new sibling was found. |

### Timing and assertion sweep

All observations below come from the same ordinary package run. The expected result for each timed L1 test is at most five seconds; assertions must remain effective and tests must remain selected.

| Child ID | Site | Shape checked | Observed result |
| --- | --- | --- | --- |
| R1-F2.1.1 | `gathered_graph_lays_out_continued_after_merge_at_merge` | Existing fixture at both widths | 2.088 s; clean |
| R1-F2.1.2 | `gathered_graph_lays_out_continued_after_merge_behind` | Existing fixture at both widths | 1.452 s; clean |
| R1-F2.1.3 | `gathered_graph_lays_out_continued_after_merge_diverged` | Existing fixture at both widths | 1.497 s; clean |
| R1-F2.1.4 | `gathered_graph_lays_out_long_labels` | Existing fixture at both widths | 1.471 s; clean |
| R1-F2.1.5 | `gathered_graph_lays_out_merged_twice` | Existing fixture at both widths | 1.166 s; clean |
| R1-F2.1.6 | `gathered_graph_lays_out_new_branch_at_merged_tip` | Existing fixture at both widths | 1.253 s; clean |
| R1-F2.1.7 | `gathered_graph_lays_out_observation_1` | Existing fixture at both widths and both current branches | 1.412 s; clean |
| R1-F2.1.8 | `gathered_graph_lays_out_observation_2` | Existing fixture at both widths | 1.676 s; clean |
| R1-F2.1.9 | Sparse-lane fixture, now `_at_120` and `_at_56` | Each width separately, preserving fixture and layout checks | 2.118 / 2.446 s; guard 0.017 s; fixed |
| R1-F2.1.10 | `the_observed_graph_keeps_recent_commits_on_every_lane_at_200x60` | Existing observed graph | 1.535 s; clean |
| R1-F2.1.11 | `run_pipeline_gathers_the_graph_on_a_narrow_image_terminal` | Existing narrow-terminal pipeline fixture | 1.031 s; clean |
| R1-F2.2 | Three `journal_size` tests | 100,000-entry completed-folder ratio, v3 replay, and v3/v2 batch-size comparison | 2.205 / 2.937 / 3.187 s; clean |
| R1-F2.3 | Previously slow removal tests | Complete ordinary L1 suite | All pass; none exceed five seconds; clean |
| R1-F2.4 | Other font-measuring L1 sites and duplicated targets | Layout call-site search, library/binary declarations, setup filter, executed names, tier audit | Enumerated cases cover the measuring sites; the filter includes both new sparse-lane names. Graph/list unit tests run once in the library target. Zero stranded tests; clean |

The worktree CLI's [layout helper and sparse-lane tests](../../cli/src/commands/git_graph/tests.rs:1815) now accept explicit viewport widths. Both sparse-lane tests still gather the same temporary-repository topology and check tag placement, merge parents, lane visibility, trimming, geometry, and explicit-width behavior. Sorted tag texts are compared against the full graph independently in each test. The trimming check compares with full-graph width where no lane is hidden; the retained fixtures do not lose the previous narrow-width check. Other callers pass both widths. The exhaustive width match in `sparse_lanes_has_a_test_per_width` guards the split's case list, and the ordinary runner executes both named tests. No timeout increase, retry, skip, or tier change closes this finding.

## Unblocked Findings

None.

## Blocked Findings

None. No human decision is needed for closure.

## Recurrence

No finding is carried forward or newly reported, so `recurrence: false`. R1-F2 was the unchanged timing instance carried by reviews 2 and 3; this review verifies its repair. R1-F1 remains fixed. No new sibling or repair regression was identified.

## Requirement verification

Every removal-store requirement needs L1 verification through real temporary files, local Git fixtures, public library results, or CLI subprocesses. This fix adds no terminal-rendering or keyboard-input requirement needing Level 2 or Level 3 evidence. The existing graph tests are assessed here for retained assertions and L1 runtime.

| Requirement | Source and fresh verification |
| --- | --- |
| Silent cleanup before name resolution, including handoff; list remains read-only | Both remove entry points call cleanup before target resolution. CLI tests verify expired removal deletion, output equality with a control, handoff cleanup, and list preservation. |
| Delete only completed operations older than 14 days, holding the nonwaiting admin-name lock; preserve pending, recent, and locked operations | Public-store tests cover the mixed-age fixture, exact boundary, future timestamps, last timeline event, folder-time fallback, pending/unreadable records, locks, staging leftovers, links, and interrupted deletion. Source rechecks completion and lock identity under the lock. |
| Version 6 compact reports with complete compressed entries, unchanged summaries, and updated caveat | Public save/read tests check omitted inline entries, named or null entries file, entry states, metadata, deleted-path cap, and repeated saves. Stop-report rendering tests pass; the caveat names `entries.jsonl.gz`. |
| Atomic entries publication on stops and completion; journals removed only after the saved entries cover them | The writer buffers before gzip, finishes compression, flushes and closes the file, then publishes it. Fault tests verify complete old/new files and retained journals across publication/completion interruptions. Completion persists before journal cleanup. |
| Read version 5 reports and version 2 current/archived journals; resume with version 3 new attempts | Lifecycle migration tests pass, including two legacy attempts followed by a version 3 attempt. They verify attempt ordering, uncertainty, earlier reports, exact decompressed entries, and final journal removal. |
| Version 3 inventory positions and ordered states preserve batch rules | Writer tests and public recovery matrices cover position bounds, kind derivation, one directory per batch, maximum batch size, duplicates within/across batches, aligned results, both format versions, and torn tails. |
| Recomputed byte limit covers worst-case version 3 lines | Singleton-longest-result, digit-boundary, full-batch, and reserved-result-limit tests pass. Source derives the limit from inventory positions and excludes the unchanged inventory line. |
| Completed 100,000-entry fixture with 150-byte paths occupies under 5% of the old layout | The completed-folder test measures all retained files against a frozen version 5 report representation and checks decompressed entry equality. It passes within budget. |
| Current documentation | The remove topic page and worktree skill explain compressed entries, 14-day retention, mixed journal versions, publication order, and retry diagnostics. The graph skill describes the test split. |
| `just test` and `just lint` | Both ordinary recipes pass. No test exceeds the L1 time budget. |

Accepted decisions about retention tombstones, timestamp-only retention reads, publication of two separate files, and legacy compatibility remain accepted. Inventory encoding, notifications about stranded pending operations, and moving the store remain outside the specification.

## Input robustness

The reviewed loaders are the versioned report reader, the per-line journal reader with version 2 and version 3 batch decoders, the shared inventory validator, and the separate retention timestamp reader. The gzip entries file has no production reader.

Default invalid-input behavior is a located error without recovery mutation. Exceptions retain their defined behavior: an unreadable optional earlier report warns and recovery uses valid journals; retention falls back to folder modification time when report timing cannot be read; an unterminated final journal line is a torn write. Empty report entry lists mean no entries, and version 6 explicitly permits a null entries-file reference. Empty batches remain invalid.

Read the public-store control cases and shape matrices and executed them through the ordinary suite. Field coverage includes absent, explicit null, wrong whole-field type, mixed and wholly invalid elements, and empty values where applicable. Document coverage includes repeated keys, unknown members, invalid/trailing content, blank journal lines, and version-specific shapes. Shared inventory validation has version-specific wiring coverage. CLI refusal tests verify filenames, exit class, and preserved checkout state. Retention tests cover both report encodings independently of the entries file.

The smell search found no newly permissive field default or silently discarded invalid collection element in the relevant readers. Position conversion's `.ok()` feeds an explicit bounds error; the journal tail's `unwrap_or_default()` implements torn-write handling; retention's `.ok()` implements the accepted fallback. Existing optional observation metadata retains legacy compatibility. The latest repair changes tests rather than these loaders.

## Validation

- `just test` in `worktree/`: exit 0; 1,562 passed, 14 skipped, zero slow; 27.081 s test execution. All previous diagnostic and storage cases run in this suite.
- `just lint` in `worktree/`: exit 0 for library and CLI, with no warning suppression. The dependency lint failure recorded by the repair no longer reproduces.
- `just check-tier-coverage worktree`: exit 0; zero stranded tests.
- Full test output: `/tmp/removal-store-review4-tests.log`; lint output: `/tmp/removal-store-review4-lint.log`.
- Cross-OS evidence remains CI's responsibility and does not block readiness.

This review changes only this report, the previous review's `next` link, and the spec's review count and completion flag. Previous `implemented` and `implemented_by` claims are preserved. No production code or tests were changed, and the fix directory remains in place for the author's review closure.
