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
created: "2026-10-22T16:28:46-07:00"
spec: 2026-10-22-remove-large-checkout/spec.md
implemented: false
description: "A **fix** review of `2026-10-22-remove-large-checkout/spec.md`"
fix: 2026-10-22-remove-large-checkout/review-2.md
previous: 2026-10-22-remove-large-checkout/review-1.md
---

# Review 2: ready within the approved scope

The journal sizing, available-space refusal, reusable usage index, report
compatibility, and repeated discovery message meet the accepted completion
boundary. No findings require repair.

This approval applies to the author's **Human rulings** in
[review 1](review-1.md#human-rulings), which replaced the original requirement
that a 500,000-entry checkout finish in one run. Large macOS removals can still
stop when file-change notifications are dropped; retrying continues them.
That accepted limitation is documented and is not reopened here. The existing
free-space refusal also remains within this review's scope as directed by
the author.

## Previous Finding Verification

Review 1 recorded no code findings. Its repair log nevertheless inventories
the author's documentation requirement as `R1-H1`; the table verifies that
requirement and every child row rather than treating the log as evidence.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| R1-H1: Document the revised completion boundary | R1-H1.1–R1-H1.4, checked individually below | verified fixed | The current docs describe resumable macOS stops and mark one-run completion as planned; the author explicitly accepted this boundary in review 1. | Nothing within this fix. |
| R1-H1.1: Removal topic describes the macOS limitation | `worktree/docs/cli/remove.md`, “Large, built checkouts” | verified fixed | [Current topic](../../docs/cli/remove.md#large-built-checkouts) names dropped notifications, explains rerunning, and labels one-run completion **Planned**. | None. |
| R1-H1.2: README stops promising one-run completion | Built-checkout bullet in `worktree/README.md` | verified fixed | [README](../../README.md) explicitly says macOS can stop after fseventsd drops notifications and that rerunning continues. | None. |
| R1-H1.3: Sweep sibling documentation and source | Library rustdoc, CLI source, worktree docs and skill, Sniff docs and skill | verified fixed | Searched all listed locations for “one run,” “one-run,” “without a retry,” and “no retry.” The sole relevant one-run promise is explicitly planned; other matches concern unrelated timing or the Windows Search recheck. | None. |
| R1-H1.4: Distinguish planned usage policy from current behavior | Usage-pause section in `worktree/docs/cli/remove.md` | verified fixed | [Usage section](../../docs/cli/remove.md#background-programs-writing-into-the-worktree) keeps today's exit-3 refusal and labels warning-and-confirmation behavior **Planned**. It does not delegate current behavior to a feature-spec path. | None. |

There are no partially repaired findings, newly discovered siblings, or
remaining blocked findings from an earlier review.

## Unblocked Findings

None.

## Blocked Findings

None.

## Implementation and Verification

Reviewed the implementation changes from `70bd0f42a` through `1bb479116`,
plus the current README and removal-topic edits. The review checked the
specification and plan against production code and executable tests; the
implementation logs were used to locate claims, not establish correctness.

| Requirement | Implementation and verification evidence | Strongest verification used here |
| --- | --- | --- |
| Size each attempt's journal from its inventory | The worktree library's [batch planner](../../lib/src/remove/observation/deletion/plan.rs) is shared by deletion and [journal sizing](../../lib/src/remove/recovery/journal.rs). Tests check each entry occurs once, content precedes `.git`, directory grouping and chunking agree, and numbering crosses 9/10 and 99/100. Worst-case results fill the computed bound exactly; one byte less rejects the final reservation. | L1: in-process planner and writer tests. |
| Remove the former 64 MiB ceiling | The counting-sink regression emits more than 64 MiB of batch lines without reaching the computed limit. Short, long, escaped, native non-UTF-8, link, hard-link, empty, and wide-directory inventories exercise the serializer; checked sums and counter overflow reject rather than wrap. | L1: synthetic inventories and the production journal writer. |
| Wire first removal and retry to their own bound, preserving consent and branch actions | [Recovery-store tests](../../lib/src/remove/recovery/store/tests.rs) compare both writers' limits with their own inventories. [Lifecycle tests](../../lib/src/remove/lifecycle/tests/large_checkout.rs) stop and resume real small checkouts. The multi-batch tracked-file regression resumes without renewed discard consent, accounts for prior deletions, and performs only the approved branch deletion. | L1: real temporary Git repositories with injected observation and usage results. |
| Refuse insufficient or unknown space before publishing or replacing an attempt | First-attempt and retry tests check exit 4, named paths and byte counts, intact checkout/registration/branch, and preserved journal/report evidence. Store tests cover exact fit, one byte short, unavailable readings, archive-copy cost, identical existing archives, and a write failure after a sufficient reading. The CLI test checks the refusal through the shipped binary. | L1: public lifecycle result, store failure seams, and CLI subprocess. |
| Read available space on the recovery store's actual filesystem | Sniff's [focused query](../../../sniff/lib/src/filesystem/space.rs) canonicalizes the location and uses native filesystem APIs. Tests cover nested mounts and different filesystems through a scripted source, aliases, missing leaves, permissions, failed readings, and zero bytes. Native tests check a symlinked ancestor and counters showing no subprocess or storage inventory. | L1: scripted filesystem source and native temporary paths. |
| Reuse the tree index without walking descendants inside the usage query | Sniff shares an `Arc<TreeIndex>` and uses one matcher for walked and supplied indexes. Tests prove zero tree walks and descendant identity reads with a supplied index, root verification, equivalent holder and hard-link results, partial coverage, visible budget exhaustion, invalid root/scope refusal, native names, and exclusion of links from Windows probe entries. | L1: query results through Sniff's fake backend and work counters. |
| Pass the correct index at every removal decision | First-attempt preflight and resumed preflight receive their respective inventories; the Windows Search recheck receives the same shared index. Post-stop diagnostics receive no index. Observation starts before capture and confirmation still runs after journal sizing, on both initial and resumed deletion paths. | L1: lifecycle and usage-source tests, plus inspection of both production paths. |
| Preserve old serialized reports and journals | Sniff schema 1 reports load as unknown provenance and survive another save/load. Current provenance round-trips and CLI JSON identifies the new schema. Worktree reports 4 and 5 resume with their earlier discovery and confirmed deletions preserved, then save readable version-6 reports. Journal version remains 2. | L1: report readers, CLI replay, and full resume through temporary repositories. |
| Merge only matching discovery notes from the current attempt | The worktree CLI's [renderer](../../cli/src/commands/remove/recovery.rs) compares coverage-note text. Tests exercise initial removal and retry, preserve separate timing/path contexts, and leave different notes, original-path diagnostics, earlier attempts, failed queries, and holder evidence separate. The returned report retains both discovery runs. | L1: production rendering with library values, assertions, and a rendered snapshot. |

These requirements concern filesystem operations, serialized data, and message
content. They introduce no key handling, focus behavior, scrolling rule, or
specific terminal styling that requires L2 or L3 evidence. Windows probe-entry
tests run in-process on this host; they do not establish native Windows
runtime behavior. Cross-OS execution remains CI's responsibility and does not
block this review. The huge real-checkout acceptance run was not repeated:
the author explicitly removed its one-run outcome from this fix's boundary.

## Input Robustness Matrix

The changed report reader defaults to rejecting invalid shapes. The new
load-bearing field is `index_provenance`: current-schema absence, explicit
null, a number, an empty string, an invalid enum value, and duplicate keys
are rejected; valid enum values round-trip. The compatibility exception is
schema 1: absence becomes **unknown**, explicit unknown remains readable
after saving, and explicit walked/supplied provenance is rejected. Collection
element shapes do not apply to this scalar field.

The existing JSON loader tests retain a positive report control, reject
trailing content and unknown fields, and exercise duplicate keys through
text rather than an already deduplicated JSON value. CLI replay checks an
old-report control and a current report missing provenance, including the
field name in stderr and no report on stdout. Inspection for permissive
defaults found only the new `default` paired with the `present` deserializer:
it deliberately distinguishes absence from explicit null before the version
check. No new configuration-file format or loader was introduced.

## Checks Run

All commands ran in this worktree on macOS through the repository recipes.
No terminal or browser window was opened.

- `worktree/`: `just test 'remove::'` — 397 passed.
- `worktree/`: `just test --test remove --test remove_recovery_report --test remove_usage_refusal --test remove_recovery` — 177 passed.
- `sniff/`: `just test 'filesystem::'` — 1,163 passed.
- `sniff/`: `just test filesystem_query::` — 32 passed.
- `worktree/`: `just test the_test_available_space` — 2 passed.
- Repository root: `just check-tier-coverage worktree` — no stranded tests.
- `git diff --check` — passed.

The inspected new tests compile in their declared library or integration
targets and use L1 names. Sniff's consolidated `l1` target includes the CLI
query module; the fresh run selected its new compatibility tests. Two initial
attempts to pass parenthesized nextest expressions through `just test` failed
at shell argument parsing before tests ran; the successful runs above use
explicit test targets or plain name filters.

## Recurrence

None. Review 1 had no code findings, and its accepted scope decision is
verified rather than reopened.

## Observations

None.
