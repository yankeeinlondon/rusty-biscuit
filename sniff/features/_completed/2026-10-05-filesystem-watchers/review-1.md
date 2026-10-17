---
$schema: feature-review.yaml
ready: false
findings:
  - title: Valid Linux watch IDs are dropped or misreported
    priority: high
  - title: Unix backends start cwd inspection after the budget expires
    priority: high
  - title: Windows loses collected module paths when the budget expires
    priority: high
observations: []
human_review: false
human_review_items: []
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: 2026-10-10T23:59:41-07:00
spec: 2026-10-05-filesystem-watchers/spec.md
implemented: true
next: 2026-10-05-filesystem-watchers/review-2.md
implemented_by: claude/opus
log: sniff/features/2026-10-05-filesystem-watchers/log.md
description: "A **feature** review of `2026-10-05-filesystem-watchers/spec.md`"
feature: 2026-10-05-filesystem-watchers/review-1.md
---

# Filesystem watchers — Review 1

The feature is **not production ready**. Three reproducible defects affect ordinary watcher registrations and queries that exhaust their time budget. None needs a human decision or a new protection layer; the existing query and backend seams can verify the repairs.

The review includes the author's approved Windows file-side discovery approach and the approved deadline contract: stop scheduling new work, while allowing an already-running native call to finish. It does not demand a strict return deadline, a Windows helper executable, or separate cross-OS evidence as a readiness gate.

## Findings

### High — Valid Linux watch IDs are dropped or misreported

**Authority: spec, acceptance criterion 3:** “Linux tests correlate live inotify registrations to the correct device/inode” and “preserve multiple watched paths/registrations.” The report contract also requires preserving the supplied watch identifier. A directory watcher commonly creates more than nine registrations. The spec's separate instruction to parse a *decimal* watch descriptor conflicts with the actual kernel format; following that sentence prevents the feature from satisfying its live-discovery acceptance criterion.

**Defect class:** a native numeric field is decoded using the wrong base, rejecting valid observations or changing their identifiers.

In the Sniff library, [the inotify registration reader](../../lib/src/filesystem/query/linux.rs) calls `parse_decimal` for `wd`. The [Linux kernel's producer](https://raw.githubusercontent.com/torvalds/linux/master/fs/notify/fdinfo.c) writes `wd` with `%x`. Registrations 10–15 therefore contain letters and are rejected as malformed. Registration 16 is written as `10`, which is accepted but becomes identifier 10. Valid sibling registrations survive, so the query can still be usable while omitting the very watcher evidence a caller requested.

I copied the captured `inotify-fdinfo.txt` fixture into the existing synthetic process fixture, changed only the first registration's `wd`, and ran the complete query to inspect its report. The other in-scope registration remained identifier 2 in every case.

| Site / input | Observed report | Expected report |
| --- | --- | --- |
| Inotify `wd:9` | Identifier 9; complete inotify coverage | Same; clean control |
| Inotify `wd:a` | Registration omitted; partial coverage | Identifier 10 retained |
| Inotify `wd:f` | Registration omitted; partial coverage | Identifier 15 retained |
| Inotify `wd:10` | Identifier 10; complete coverage | Identifier 16 retained |
| Inotify `wd:1a` | Registration omitted; partial coverage | Identifier 26 retained |
| Sibling registration fields `ino`, `sdev`, `mask` | Existing full-query fixture matrix retains valid hexadecimal values; device conversion passes | Same; clean |
| Descriptor header `flags` and `ino` | Existing query tests report the expected access flags and detect changed descriptor inodes | Octal flags and decimal header inode; clean |

These are all numeric readers in the changed Linux fdinfo path. The decimal header `ino` is a different field from the hexadecimal registration `ino`; changing both indiscriminately would introduce another defect. macOS and Windows do not read this format.

The existing malformed-input matrix passes, including absent, empty, invalid, duplicate, overflowing, mixed valid/invalid registrations, and trailing garbage. It misses this valid-input failure because its captured fixture uses watch IDs 1–3 and the live child creates only five watches. For the affected `wd` column: absent, empty, signed, duplicate, and malformed values are rejected; a bad registration preserves valid siblings; the defect is acceptance and interpretation of valid hexadecimal values. Literal null and array-element shapes are not part of this kernel text format. Required `ino`, `sdev`, and `mask` absence/empty/invalid cells pass the existing matrix; unknown extra fields remain accepted as required.

Parse registration `wd` as hexadecimal, preserving the public numeric value. Add a full-query regression with IDs crossing 9, 15, and 16 and extend a live child beyond nine registrations. Record the departure from the spec's mistaken decimal sentence in the implementation log and keep the current documentation accurate; do not rewrite the historical specification body.

### High — Unix backends start cwd inspection after the budget expires

**Authority: spec, Library API:** “stop scheduling work when the budget expires.” The author's approved deadline decision permits an already-running call to overrun; it does not permit starting another usage mechanism after expiry.

**Defect class:** a transition between usage mechanisms fails to check the shared budget before starting the next mechanism.

In the Sniff library, the [Linux backend](../../lib/src/filesystem/query/linux.rs:153) and [macOS backend](../../lib/src/filesystem/query/macos.rs:192) call cwd inspection unconditionally after descriptor inspection. The descriptor loop can explicitly observe expiry and return; the next call still reads the process's cwd. This affects busy processes or a short, valid `--timeout`, and can add another slow native operation after the library already knows its budget has run out.

I reused the ordinary two-open-files fixture, added a cwd inside the target, and drove the complete query with the existing deterministic budget seam. In both backends the descriptor scan recorded budget exhaustion, yet the final report contained newly acquired cwd evidence. Root recheck correctly made the final coverage partial; that does not undo the extra inspection.

| Sibling transition | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- |
| Linux descriptors → cwd | Budget expires in descriptor inspection; matching cwd | Cwd is read and reported | Preserve earlier evidence; do not start cwd inspection |
| macOS descriptors → cwd | Same shape through the scripted native source | Cwd is read and reported | Same |
| Shared query: one process → next process / enrichment / root recheck | Existing full-report budget test expires while inspecting a process | Later processes and enrichment skipped; root marked unverified | Same; clean |
| Windows file-side handles → per-process modules | Existing file-side test expires its budget while returning a holder | Holder evidence survives; outer query does not start later process inspections | Same; clean |
| Windows module → next module | Existing two-module budget test | Stops before reading the next module | Same; clean transition, with the separate retention defect below |

The two Unix cwd calls are the sibling sites that start a different usage mechanism inside a per-process inspection. Add a budget check at each transition and an explicit incomplete/skipped result for cwd, without dropping earlier evidence. Test the final report and whether the native cwd source was called; the present descriptor-budget tests assert retained descriptor evidence but do not catch the subsequent cwd read. Preserve process-lifetime safety checks when deciding how to retain the earlier evidence.

### High — Windows loses collected module paths when the budget expires

**Authority: spec, Library API:** “Deadline exhaustion returns collected evidence and explicit incomplete coverage, not an empty success,” and “Preserve evidence before enriching identity.” Acceptance criterion 4 requires budget-exhaustion coverage during each phase.

**Defect class:** matching is deferred until after an inspection batch, so expiry discards already acquired path observations rather than retaining validated matches as they arrive.

In the Sniff library, [Windows module inspection](../../lib/src/filesystem/query/win32.rs:423) retains only raw paths after a textual scope check. The [shared report builder](../../lib/src/filesystem/query/mod.rs:405) subsequently resolves every path through [the matcher](../../lib/src/filesystem/query/matching.rs:55). That matcher refuses a fresh lookup once the budget expires. Consequently, when the backend collects the first in-scope module and expires before the second, the first observation is discarded too. The resulting report has partial loaded-module coverage but no process evidence. A query of a process with many modules can reach this through an ordinary valid timeout.

The existing `a_budget_expiring_between_modules_keeps_what_was_read` test passes because it inspects the backend's raw result, before report construction. A temporary full-query reproduction reused its two-module fixture and deterministic budget. It produced **zero retained evidence records**, despite the backend having retained the first module path.

| Sibling projection | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- |
| Windows loaded modules → shared path matcher | Two in-scope modules; expire before second | Final process list empty; module coverage partial | Validate and retain the first match while its budget is available |
| Linux descriptors/cwd/inotify → identity matcher | Existing descriptor-budget and full-query identity-evidence tests | Already identified evidence survives expiry | Same; clean |
| macOS vnodes → identity matcher | Existing descriptor-budget and shared identity-evidence tests | Already identified evidence survives expiry | Same; clean |
| Windows file-side handles → report | Existing full-query test expires while returning a holder | Holder remains in final report | Same; clean |
| Shared fake backend → identity matcher | Existing full-query test expires during inspection | Previously collected process evidence remains | Same; clean |

Within the shipped backends, loaded modules are the path-only observations that require this deferred lookup; Unix mechanisms and Windows file-side holders carry native identities. The shared matcher is therefore the common projection to repair, rather than adding a Windows-only exemption that performs new lookups after expiry.

Validate each candidate module against the captured tree while processing it, and retain its resulting identity and matched paths for report construction. This also removes the need to defer all module lookups until the batch finishes. Extend the existing budget regression through the final report, and assert that retained evidence and process identity survive with partial coverage. Do not repair this by ignoring the budget in the matcher.

## Verification level by requirement

| Spec requirement | Verification present | Assessment |
| --- | --- | --- |
| One observation projected to text/JSON; no inventory or network; request counters | Level 1 retained-report CLI tests and query counter assertions | Appropriate; focused tests pass |
| Controlled open-file/cwd discovery | Level 1 native controlled children for Linux, macOS, and Windows, gated by OS | Appropriate; live macOS cases ran here; other OS execution remains CI's responsibility |
| Inotify identity, registration multiplicity, ancestor exclusion, hard links | Level 1 real-kernel child and captured-fixture query tests | Correct level, but valid hexadecimal watch IDs are missing from the cases |
| Denial, process exit/reuse, missing identity, shared budgets | Level 1 scripted backends and full-query tests | Correct level; the two budget findings identify gaps in what the assertions prove |
| FSEvents limitations | Level 1 live descriptor/cwd queries and scripted reports assert FSEvents is unsupported; no live FSEvents stream is created | Verifies the unconditional unsupported status; does not establish FSEvents subscription inspection |
| Native paths, root replacement, links/reparse points, Windows spelling/case/surrogates | Level 1 tree/matcher/serialization tests and Windows native spelling cases | Appropriate; matching logic and OS-gated test wiring inspected |
| Coverage, exit codes, JSON/performance, hostile text, truncation, plain output | Level 1 shipped CLI tests on injected reports and live focused queries | Appropriate for the specified noninteractive report; 30 focused CLI tests pass |
| Specific terminal glyphs, colors, scrolling, or keyboard behavior | No such acceptance promise in this feature | No additional Level 2/3 gate imposed |
| Hidden/ignored entries, caller usage, Linux threads | Level 1 tree tests and native controlled-process cases | Appropriate; Windows caller exclusion is an explicit accepted native limitation |

The CLI integration file is declared by the consolidated Level 1 target, and the library backend tests are compiled from the query module. `just check-tier-coverage sniff` reported no stranded tests. The focused API is not gated behind remote or network features.

## Checks performed

- Read the specification, implementation log, library orchestration, matching, root/tree handling, native serialization, each backend, CLI dispatch/rendering, documentation, and feature test placement.
- `just test filesystem::query::`: **117 passed** on macOS, including portable Linux/Windows classifier tests and live macOS cases.
- `just test filesystem_query`: **30 passed**.
- `just check-tier-coverage sniff`: **no stranded tests**.
- Four temporary probes through complete library report construction confirmed the watch-ID failure, both Unix cwd transitions, and Windows module loss. All four completed successfully with the observed defective behavior asserted/reported. They used the existing fixtures and deterministic budget seam, not sleeps or timing races, and were removed afterward.
- The initial probe build encountered an obsolete Homebrew Cellar link path; a process-local `LIBRARY_PATH` pointing at the installed libgit2 allowed the rebuild. This environment issue is recorded in the OS skill and is not a feature finding.

No production code was repaired during this review. Existing edits to the plan and implementation log were preserved. The specification's review iteration is now 1; it is not marked completed.

## Observations

None.
