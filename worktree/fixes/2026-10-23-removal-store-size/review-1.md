---
$schema: feature-review.yaml
reviewed_by: codex/gpt-6.1-sol
created: "2026-10-23T14:39:35-07:00"
spec: 2026-10-23-removal-store-size/spec.md
fix: 2026-10-23-removal-store-size/review-1.md
implemented: true
description: "A **fix** review of `2026-10-23-removal-store-size/spec.md`"
ready: false
findings:
    - "R1-F1 (medium): unreadable-journal diagnostics omit the journal filename and disagree on exit class"
    - "R1-F2 (medium): graph L1 tests exceed the five-second L1 time budget"
human_in_the_loop: null
human-in-the-loop: false
human_review: true
human_review_items:
    - "R1-F1: name-lookup store errors now take the library's exit class (storage/IO exit 4, unusable record/journal/report exit 3; both were exit 1)"
    - "R1-F1: StoreError::Journal carries the journal path instead of the operation id (the path contains the id)"
    - "R1-F2: commands::git_graph and commands::list unit tests run only in the worktree-cli lib target (lib_target_tests!), not twice"
    - "R1-F2: root Cargo.toml test profile optimizes merman, merman-core, merman-render, fontdb, ttf-parser; may widen CI scope"
    - "R1-F2: the remaining local graph-test overrun is CoreText listing a 212k-entry target/debug/deps; tests take 1.3-2.4 s from a small directory; no code change for it"
has_blocked_findings: false
blocked: false
recurrence: false
implemented_by: claude/opus
next: 2026-10-23-removal-store-size/review-2.md
log: /Volumes/coding/wt/rusty-biscuit/feat-merman/worktree/fixes/2026-10-23-removal-store-size/log.md
---

# Review 1: removal store size

The fix is not production ready: two findings remain open (R1-F1, R1-F2). Both were first recorded as observations about code this fix did not change; under the current review rules every defect the review finds is a finding, and every finding blocks readiness. This is the first implementation review, so there are no previous findings to carry forward.

## Decisions and deferred scope

Two independent reviewers examined the implementation log and the specification/plan before the implementation review continued. The log's 17 `decisions_for_review` entries contain no unresolved design choice that needs a human decision.

The consequential implementation choices are reasonable:

- Expired completed folders are renamed to `.<operation-id>.expired` before deletion. If deletion stops halfway through, the remaining folder cannot masquerade as a pending removal with a missing record and block later removals. The next run retries deletion.
- Retention reads only report timestamps and falls back to folder modification time when it cannot obtain a completion time. A durable completed record, checked again while holding its worktree lock, authorizes expiration; a successful-looking report alone does not.
- The compressed entries file is published before report metadata, and completion is recorded before journals are removed. An interruption between publishing the two files can leave newer entries beside an older report; pending recovery still uses the journals.
- Report metadata is read without decompressing the entries file. Older reports and journals remain readable; malformed legacy documents receive stricter validation. Earlier writers already emitted the newly required report members, and no known writer produced duplicate cross-batch journal entries.
- Borrowed inventory serialization avoids copying large inventories. Test-profile optimization of the gzip dependencies and splitting the large read-back test preserve the specified 100,000-entry fixture within the L1 time budget.

The specification deliberately leaves inventory encoding, notifications about stranded pending removals, and moving the store outside Git for separate work. The log also leaves platform-specific timestamp-overflow testing, a Windows deletion-failure injection mechanism, and existing journal diagnostics unchanged. None defers a required user-facing behavior or needs approval to finish this fix.

## Findings

None.

## Requirement verification

All requirements here concern storage, recovery, or ordinary CLI output. L1 tests with real temporary files, local Git repositories, and subprocesses exercise the relevant boundaries; real-terminal or keyboard-injection tests are unnecessary for this change.

| Requirement | Strongest relevant verification reviewed |
| --- | --- |
| Expire only completed operations older than 14 days; keep pending, recent, future-dated, unreadable, and locked records | L1 public-store tests cover the mixed-age fixture, exact boundary, last-event timestamp, fallback timestamps, locks, links, and interrupted deletion. |
| Cleanup runs before name resolution, silently, including handoff; list never prunes | L1 shipped-CLI tests compare cleanup output with a control, exercise handoff, and confirm list preserves expired records. |
| Version 6 reports omit inline entries and preserve summaries and previous attempts | L1 public save/read and lifecycle tests compare report metadata and decompressed entries, including all entry states, the 200-path preview limit, stops, and resumes. |
| Entries publication is atomic and journals survive incomplete publication | L1 public completion tests inject failures at write, flush, entries publication, report publication, and completion, then verify complete old/new files and recoverability. |
| Older reports and version 2 journals resume alongside new version 3 attempts | L1 lifecycle migration test starts with a version 5 report and two version 2 journal generations, resumes with version 3, and checks attempt order, uncertainty, entries, and journal cleanup. |
| Version 3 batches preserve inventory and batch rules and fit the recomputed limit | L1 writer tests and public recovery matrices check positions, ordered states, directory boundaries, duplicates, batch size, budget limits, torn writes, and digit boundaries. |
| Completed 100,000-entry fixture with 150-byte paths occupies under 5% of the old layout | L1 completion test measures every remaining file, compares against a frozen version 5 report shape, and checks decompressed entry equality. Separate large-journal tests verify both encodings. |
| Current documentation explains the new behavior | The remove topic page and worktree skill describe retention, compressed entries, versioned journals, and publication order. |

## Input robustness

Reviewed the report loader, version-specific batch readers, shared inventory validator, and the separate retention timestamp reader. Invalid report or journal shapes produce errors without publishing recovery changes; retention instead uses its specified timestamp fallback or preserves an unreadable record.

The public-store matrices include positive controls, missing/null/wrong-type/empty fields, mixed and wholly invalid collection elements, duplicate keys, unknown members, trailing data, invalid versions, invalid positions, and mismatched results. The shared inventory validator is tested once, with version 2 wiring coverage. Unterminated final journal lines are intentionally treated as torn writes, while malformed newline-terminated lines fail. Report-version exceptions and explicit `entries_file: null` are covered. The gzip entries file has no production reader.

The smell check found no permissive default on a newly load-bearing report or batch field. The position conversion's `.ok()` feeds an explicit out-of-range error, and the journal split's `unwrap_or_default()` handles only the torn tail. Existing optional observation metadata retains its documented compatibility behavior.

## Validation

- `just test` from `worktree/`: 1,636 passed, 15 skipped; 21 existing graph tests were reported slow.
- `just lint` from `worktree/`: passed for both library and CLI.
- `just check-tier-coverage worktree`: passed, zero stranded tests.
- `git diff --check -- worktree/fixes/2026-10-23-removal-store-size`: passed.
- Cross-OS results were not rerun as a readiness gate. The implementation log records Windows library verification; this review's fresh execution is on macOS.

## Findings

### R1-F1 (medium): unreadable-journal diagnostics omit the journal filename and disagree on exit class

An unreadable journal error gives a line number but does not distinguish `journal.jsonl` from an archived `journal-<n>.jsonl`. Pending-operation lookup also reports exit 1 while the library's refusal classification uses exit 3. These behaviors predate this fix and do not violate its contract. Naming the file would help someone inspect a stopped removal, and the lookup and the library classification must agree on one exit class.

### R1-F2 (medium): graph L1 tests exceed the five-second L1 time budget

The fresh full suite reported 21 slow tests in graph behavior outside this fix's changed code. Although nextest reported them as passing, their durations exceed the repository's five-second L1 budget. Every graph L1 test must finish within the budget.
