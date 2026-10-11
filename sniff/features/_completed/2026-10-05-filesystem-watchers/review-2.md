---
$schema: feature-review.yaml
ready: true
findings: []
observations: []
human_review: false
human_review_items: []
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: 2026-10-11T00:43:12-07:00
spec: 2026-10-05-filesystem-watchers/spec.md
implemented: false
description: "A **feature** review of `2026-10-05-filesystem-watchers/spec.md`"
feature: 2026-10-05-filesystem-watchers/review-2.md
previous: 2026-10-05-filesystem-watchers/review-1.md
---

# Filesystem watchers — Review 2

The feature is **production ready** under the author's approved contract: one shared budget stops new discovery work, and an already-running native call may finish after that budget. Windows uses the approved file-side handle-owner query. Discovery remains advisory and reports unsupported mechanisms; it does not establish whether deletion will fail.

All three findings from Review 1 are resolved. This review found no new high, medium, or low findings and no need for another human design decision. Cross-OS execution remains part of CI/CD, rather than a separate readiness gate for this review.

## Prior findings and repair log

Review 1 put all three findings under `Findings` and marked none blocked. Those were its actionable findings; there were no blocked findings to become unblocked. The [repair log](log.md) records three fixes, no deferrals, and no disputes. I checked the implementations and their final-report assertions rather than treating the log's completion claim as proof.

| Earlier high finding | Repair verified | Result |
| --- | --- | --- |
| Valid Linux watch IDs are dropped or misreported | The Sniff library's [registration reader](../../lib/src/filesystem/query/linux.rs) now decodes watch IDs as hexadecimal. The full-query regression reports `a`, `f`, `10`, and `1a` as 10, 15, 16, and 26, with the correct paths and complete inotify coverage. The Linux live-child test now creates 23 registrations and compares reported IDs against the child's actual registration results. | Resolved |
| Unix backends start cwd inspection after the budget expires | The Sniff library's [Linux backend](../../lib/src/filesystem/query/linux.rs) and [macOS backend](../../lib/src/filesystem/query/macos.rs) check the shared budget before reading a working directory. Full-query tests assert zero cwd reads after expiry, retained descriptor evidence, and `not_attempted` cwd coverage. Shared tests distinguish a mechanism skipped for every process from one inspected for some processes. | Resolved |
| Windows loses collected module paths when the budget expires | The Sniff library's [Windows module backend](../../lib/src/filesystem/query/win32.rs) validates each candidate path during inspection and retains the resulting match. The [report builder](../../lib/src/filesystem/query/mod.rs) projects that match without another filesystem lookup. The full-query regression retains the first module and its process identity when the budget expires before the second. A separate test proves an unvalidated module is reported as a budget gap. | Resolved |

The hexadecimal watch-ID correction departs from the specification's mistaken decimal sentence. That departure is recorded in the implementation log, and the current topic documentation describes the actual kernel format. The historical specification body remains intact.

## Findings

None.

## Related-site checks

I revisited the three defect classes from Review 1 across their in-scope sibling sites. No unresolved sibling or recurring finding was identified.

| Class checked | Sites checked | Evidence and result |
| --- | --- | --- |
| Native numeric values decoded using the wrong base | Linux registration `wd`, `ino`, `sdev`, and `mask`; descriptor-header flags and inode; process start token; PID and descriptor names | Registration fields use hexadecimal, flags use octal, and header inode/process identifiers use decimal. The captured-fixture full-query matrix and hexadecimal-ID regression pass. macOS and Windows do not parse this text format. |
| Starting another usage mechanism after budget expiry | Linux descriptors → cwd; macOS descriptors → cwd; Windows path-holder scan → modules; shared process loop, root recheck, and enrichment | Both repaired cwd transitions are exercised through complete reports with native-read counters. Existing budget tests cover stopping later processes, joined Windows workers, root recheck, and enrichment. Process-lifetime revalidation still runs to establish attribution of evidence already read. |
| Losing acquired evidence during report construction | Linux descriptors, cwd, and inotify; macOS vnodes and cwd; Windows loaded modules and file-side holders; shared report projection | Native-identity observations match against the captured tree. Path-only module observations carry their validated match. Full-query expiry tests retain earlier evidence and expose incomplete coverage; refused module lookups produce no invented evidence. |

## Verification level by requirement

| User-facing requirement | Strongest verification present | Assessment |
| --- | --- | --- |
| One report projected to text/JSON; no inventory, network, or unrelated refresh | Level 1 retained-report CLI tests, live focused CLI queries, and work-counter assertions | Appropriate; focused tests pass. |
| Open-file and cwd discovery on supported backends | Level 1 controlled native children, gated by OS; portable scripted backend tests | Appropriate. Live macOS cases ran here; Linux and Windows live cases are compiled on their respective platforms. |
| Linux inotify identity, multiple registrations, hard links, and exclusion of ancestors/similarly prefixed siblings | Level 1 captured-kernel-fixture full queries and a real-kernel controlled child | Appropriate. The repaired fixture regression passes here; the live child now crosses hexadecimal watch-ID boundaries. |
| Permissions, malformed observations, process exit/reuse, missing identity, and exhaustion in query phases | Level 1 deterministic backend and full-report tests | Appropriate. Repaired tests assert final evidence and coverage, including whether cwd inspection actually started. |
| Honest macOS FSEvents limits | Level 1 scripted and live macOS reports assert unconditional unsupported coverage | Appropriate for the unsupported capability; no claim of subscription inspection or guaranteed watcher invisibility is made. |
| Root aliases/replacement, links, native paths, hard links, and Windows spelling/case/surrogates | Level 1 tree, matching, serialization, and OS-gated native spelling tests | Appropriate; portable cases pass here and platform-specific test wiring is present. |
| Coverage and exit semantics; JSON with performance; literal text; plain output and explicit truncation | Level 1 shipped CLI tests with retained reports and live focused queries | Appropriate; all 30 focused CLI tests pass. |
| Hidden/ignored entries, genuine caller usage, detector handles, and Linux thread exclusion | Level 1 tree and controlled-process tests; explicit Windows caller limitation | Appropriate; no stronger systemwide-discovery guarantee is inferred. |

No feature acceptance requirement depends on keyboard encoding, terminal scrolling, or particular terminal glyphs/colors. Level 2 or Level 3 tests are therefore not required for these report contracts.

The library tests are declared by the query module, and the CLI integration tests are declared by the consolidated Level 1 target. The query API and new repair tests do not require optional remote/network features. The tier audit found no stranded tests.

## Input robustness

The repair changes the Linux native-record parser, not a configuration loader. Its existing captured-fixture matrix runs through `PathUsageReport` and checks all four registration fields for missing, empty, and invalid values, while preserving valid sibling registrations. It also exercises duplicate fields, trailing garbage, empty records, overflow, and mixed valid/invalid records. New watch-ID cases cover signed, prefixed, and overflowing values, together with a positive hexadecimal-ID control. The matrix passed in this review. JSON nulls and array-element shapes are not cells in this kernel text format.

Lossless native strings and serialized reports retain their existing strict-reader tests. No new manifest, lockfile, or configuration format was introduced by the repair.

## Checks performed

- Read the specification, Review 1, repair log, implementation departures, current topic documentation, changed production code and tests, related matching/tree/lifetime handling, CLI dispatch/output, and test-target declarations.
- `LIBRARY_PATH=/opt/homebrew/opt/libgit2/lib just test filesystem::query::` in `sniff/`: **124 passed** on macOS, including portable Linux/Windows backend tests and live macOS tests.
- `LIBRARY_PATH=/opt/homebrew/opt/libgit2/lib just test filesystem_query` in `sniff/`: **30 passed**.
- `just check-tier-coverage sniff`: **no stranded tests**.
- `LIBRARY_PATH=/opt/homebrew/opt/libgit2/lib cargo clippy --color=never -p sniff -p sniff-cli --all-targets -- -D warnings`: **passed** with default features.
- `git diff --check`: **passed**.

The process-local libgit2 path uses the previously documented host workaround. Native Linux, Windows, and WSL2 repair-run results are recorded in the repair log; they were inspected but not rerun or presented as new evidence from this review.

No production code was changed during this review. Review 1 now points to this review and remains marked implemented. The specification now records two review iterations and completion; the feature stays in its active directory for the author to close the lifecycle.

## Observations

None.
