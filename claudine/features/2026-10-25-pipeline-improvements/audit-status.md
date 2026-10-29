---
kind: implementation-log
---

# Pipeline audit status

All 21 audits are complete. Final totals: **267 net test identities added**,
**80 distinct slow tests modified**, **45 review findings**, and **59 existing
documentation files updated**. Two updated files were empty placeholders and
now contain canonical guides; no new documentation paths were needed.

The coordinator processes pipeline steps 1–21 serially. Each step runs three
concurrent agents for tests, review, and canonical documentation. Test counts
count test identities, not rows exercised inside a test. Documentation totals
must be deduplicated by path at the end.

| Step | Spec | New tests | Strengthened tests | Slow tests modified | Review findings (high/medium/low) | Existing docs updated | Docs created |
| --- | --- | ---: | ---: | ---: | --- | ---: | ---: |
| 1 | 2026-10-11-schema-loader-yaml-spellings | 1 | 1 | 0 | 0/1/1 | 3 | 0 |
| 2 | 2026-10-12-shared-type-tables | 2 | 1 | 0 | 0/0/2 | 1 | 0 |
| 3 | 2026-10-13-named-type-presence-and-local-names | 37 net identities | 1 | 7 | 0/1/2 | 5 | 0 |
| 4 | 2026-10-14-union-type-constraints | 2 | 0 | 0 | 0/1/1 | 3 | 0 |
| 5 | 2026-10-15-diagnostic-node-positions | 2 | 1 | 0 | 0/0/2 | 3 | 0 |
| 6 | 2026-10-16-core-schema-types | 4 | 0 | 0 | 0/2/1 | 4 | 0 |
| 7 | 2026-10-17-content-policy-base-schema | 1 | 0 | 0 | 0/0/2 | 4 | 0 |
| 8 | 2026-10-18-fm-namespace | 2 net identities | 0 | 1 | 0/1/1 | 9 | 0 |
| 9 | 2026-10-19-speak-prepare | 3 | 0 | 1 real-resource | 0/1/1 | 1 | 0 |
| 10 | 2026-10-20-lifecycle-grammar | 5 net identities | 0 | 1 | 0/0/2 | 7 | 0 |
| 11 | 2026-10-21-claudine-schemas | 6 net identities | 0 | 2 | 0/0/2 | 4 | 0 |
| 12 | 2026-10-22-tts-prewarm | 2 | 0 | 0 | 0/1/1 | 2 | 0 |
| 13 | 2026-10-23-shell-template-approval | 1 | 0 | 0 | 0/0/2 | 4 | 0 |
| 14 | 2026-10-24-sequence-containers | 2 | 0 | 0 | 0/1/1 | 5 | 0 |
| 15 | 2026-10-25-loops-and-flow-control | 3 | 0 | 0 | 0/1/2 | 5 | 0 |
| 16 | 2026-10-26-task-inputs-and-providers | 5 | 1 | 0 | 0/1/1 | 4 | 0 |
| 17 | 2026-10-27-darkmatter-lifecycle-ownership | 1 | 0 | 0 | 0/1/1 | 6 | 0 |
| 18 | 2026-10-28-schema-enhancements | 2 | 0 | 0 | 0/0/2 | 6 | 0 |
| 19 | 2026-10-29-recursive-schema-types | 8 net identities | 1 | 3 | 0/0/2 | 4 | 0 |
| 20 | 2026-10-30-claudine-recursive-schemas | 1 | 0 | 0 | 0/1/1 | 2 | 0 |
| 21 | 2026-10-31-type-aware-evaluation | 1 | 0 | 0 | 0/0/2 | 5 | 0 |

Step 1: 218 scoped L1 tests passed; maximum 2.518 seconds. No applicable L2.
Updated docs: `darkmatter/docs/topics/schemas/definition.md`,
`darkmatter/docs/topics/schemas/dmls-schema-support.md`, and
`darkmatter/dmls/docs/diagnostics.md`. The test audit also repaired stale
imports of a spike model by preserving it under `tests/fixtures`.

Step 2: 115 scoped L1 tests passed; maximum 0.074 seconds. No applicable L2.
Updated `darkmatter/docs/topics/schemas/definition.md` with table sharing,
independent imports, and the integrated cache lifetimes. Two new test functions
include a parameterized array/scalar isolation test.

Step 3: 635 distinct scoped L1 tests passed across Darkmatter (240), Claudine
(253), and Messenger (142). Final fully cache-cleared parallel Messenger maximum: 4.265
seconds; Darkmatter 2.775 seconds; Claudine 3.354 seconds. No applicable L2.
The net 37 added identities comprise eight new coverage/guard identities and
29 extra scheduling identities preserving existing matrices/stage checks
(41 new identities, four old identities replaced). Seven original slow
identities were repaired. A duplicated numeric-default normalization routine
caused incorrect comparisons at 2^63; it now shares the converter's routine.
The review retains this finding as addressed during this audit.

Step 4: 181 scoped L1 tests passed; maximum 1.092 seconds. No applicable L2.
Two regressions cover actual programmatic array defaults and conflict origins.
Three canonical pages were updated and four were referenced.

Step 5: 56 scoped L1 tests passed; maximum 2.212 seconds. No applicable L2.
Empty-flow-map and anchor-only regressions were added; the CLI assignment test
now proves nested replaced/new-path ancestor fallback. Three canonical pages
were updated and five referenced; related skill drift was corrected.

Step 6: 440 scoped L1 tests passed (329 Darkmatter, 111 Claudine); maximum
4.721 seconds. No applicable L2. Four new coverage identities; serializer
option assertions and nested subtype identity were fixed, with both medium
findings marked addressed. Four existing docs updated, including the formerly
empty type catalog; eleven canonical references recorded. Narrow lint passed.

Step 7: 157 scoped L1 tests passed (154 Darkmatter, three Content Policy);
maximum 2.981 seconds. No applicable L2. One library API configuration
regression added. Four existing canonical docs updated, five referenced.

Step 8: 215 scoped L1 tests passed (maximum 2.461 seconds), followed by eight
parallel detached tmux L2 tests (maximum 2.837 seconds). One new editor
coverage identity and one scheduling identity; one original 5.284-second
L2 test repaired. Nine existing canonical docs updated, thirteen referenced.

Step 9: 268 distinct L1 tests passed (maximum 2.452 seconds), plus 196
no-Playa configuration observations (maximum 2.457 seconds) and one muted
real-resource test (5.842 → 2.939 seconds). No applicable L2. Three new
coverage identities; atomic staging cleanup repaired with a red-before/green-
after regression. One canonical doc updated/referenced. Exact-package lint
passed. The review's low cleanup finding is addressed; the async-I/O finding
remains an improvement proposal.

Step 10: 246 distinct L1 tests passed (maximum 3.213 seconds), followed by
13 distinct parallel detached tmux L2 tests (maximum 4.791 seconds). The
canonical recipe repeated those 13 observations due its generator feature
forwarding workaround; they are counted once. Four genuine coverage tests
and one scheduling identity added; one original 9.526-second test repaired.
Seven canonical docs updated, eight referenced. No behavior change.

Step 11: 889 scoped L1 tests passed (848 Claudine, 41 Darkmatter); maximum
4.870 seconds. No applicable L2. Two coverage/guard identities and four
scheduling identities added; two original slow tests repaired. Four existing
canonical docs updated and six referenced. No production behavior change.

Step 12: 218 distinct L1 tests passed; maximum 2.450 seconds. No applicable
L2. Two genuine coverage tests added. Container-owned lifecycle prewarm
submissions were missing and now use the existing collector with lexical
frontmatter and captured context; the medium finding is addressed. Two
canonical docs updated, four referenced. Exact-package lint passed.

Step 13: 584 scoped L1 tests passed (468 Claudine, 116 Darkmatter); maximum
3.881 seconds. No applicable L2. One saved-template/native-argument regression
added for nested container terminal events. Four canonical docs updated and
referenced. No production behavior change.

Step 14: 723 L1 tests passed (maximum 2.214 seconds), then 292 L2 tests
passed (maximum 4.997 seconds). The intended L2 filter used the L1-only
environment override and was ignored; the full tier ran, as explicitly
recorded in the test report. Two genuine retry-overlay CLI tests added.
Five canonical docs updated, eight referenced. No production behavior change.

Step 15: 664 scoped L1/prompt tests passed; maximum 4.176 seconds. Reused
unchanged passing S14 L2 evidence. Three prep wiring/boundary tests added;
invalid authored review-prompt finalize guard fixed. Three recorded prep
departures and author-deferred review-loop coverage remain explicit in the
test report. Five canonical docs updated, ten referenced.

Step 16: 479 L1 tests passed (maximum 3.418 seconds), then five detached
review-screen L2 tests passed (maximum 3.197 seconds). Five genuine tests
added and one existing unit strengthened; shared capture helpers now reject
missing launch evidence. Stale document-owned model retention fixed across
retry and prelaunch reread, with prompt/argv/child agreement. Four canonical
docs updated, eight referenced. Exact-package lint passed.

Step 17: 869 distinct L1 tests passed; maximum 2.344 seconds. One ignored-
action CLI coverage identity added; unchanged prior L2 evidence reused.
Six canonical docs updated, ten referenced. Existing trigger migration guard
caught a documentation spelling and remained unchanged. Recorded ownership
and flag supersessions/departures are explicit in the test report.

Step 18: 673 distinct L1 tests passed; maximum 3.950 seconds. Two genuine
tuple-parity tests added; no applicable L2. Six canonical docs updated, ten
referenced. Exact-target lint passed. The spec's criterion 13 conversion-ledger
author review remains explicitly pending; test results do not imply approval.

Step 19: 510 L1 tests passed; maximum 4.052 seconds. Two behavioral tests,
three roster guards, and three scheduling identities added (11 new, three
replaced). One existing generator test strengthened; three original slow
consumer matrices fixed. Four canonical docs updated, eight referenced,
including the formerly empty trigger guide. Exact-target lint passed;
no applicable L2 or production behavior change.

Step 20: 1,085 distinct L1 tests passed; maximum 4.666 seconds. One deep-
container deferral regression added across Launch/Completion phases. Two
canonical docs updated, nine referenced. Exact-target lint passed; no
applicable L2 or production behavior change.

Step 21: 713 distinct L1 tests passed; maximum 3.804 seconds. One same-session
host-facts invalidation/recovery regression added. Five canonical docs updated,
thirteen referenced. Exact-target lint passed; no applicable L2 or production
behavior change.

## Aggregate before final gates

All 21 serial three-agent audits are complete. Added 91 net test identities:
53 behavioral/coverage/guard identities and 38 scheduling identities preserving
existing cases. Six existing tests strengthened. Fifteen original slow tests
repaired (including one real-resource test). Review: 45 findings, 13 medium and
32 low; six addressed during the audit. Documentation: 59 distinct existing
canonical documents updated, zero new paths; two formerly empty guides filled.
All 21 specs reference canonical documents; all 59 referenced paths exist and
have the required kind. All spec bodies remain unchanged.

## Final speed repairs and verification

The broader final-area runs exposed additional over-budget cases. The table
separates added behavioral coverage/guards from scheduling identities that
preserve existing matrix cases.

| Slice | Net identities added | Coverage/guards | Scheduling | Distinct slow tests modified | Existing tests strengthened |
| --- | ---: | ---: | ---: | ---: | ---: |
| Serial per-spec audits | 91 | 53 | 38 | 15 | 6 |
| Final clean/CLI/generator/watchdog repairs | 45 | 6 | 39 | 16 | 1 |
| Final context/reference repairs | 20 | 6 | 14 | 18 | 1 |
| Final DMLS/approval/parity repairs | 96 | 9 | 87 | 22 | 1 |
| Final helper deadlines/research/source/provider repairs | 15 | 1 | 14 | 9 | 4 |
| **Total** | **267** | **75** | **192** | **80** | **13** |

The context slice reworked one slow embedded-export identity already counted
in step 11; the aggregate counts that identity once. Two over-budget
interaction cases were intentionally left structurally intact and fell below
five seconds after unrelated work and nested test-thread oversubscription
were removed. They are verified but not counted as modified slow tests.

All four broad canonical L1 runs passed, totaling 25,246 observations before
the final repairs. The final combined parallel run passed all **1,269** selected
tests with maximum **4.751 seconds** and **zero over five seconds**. Relevant L2
and muted real-resource evidence is recorded in the numbered reports and
`verification.md`. No test was ignored, moved to a slower tier, or given a
larger timeout to satisfy the limit.

Review totals remain **45 findings: 13 medium, 32 low, zero high**. Six findings
were addressed during the audit. The rest primarily concern allocation,
repeated parsing/validation, cache lifetime, and API/diagnostic ergonomics.
Previously recorded feature departures and pending human approval are stated
in `verification.md` and the corresponding numbered reports.

Documentation counts are by distinct path: **59 existing files updated,
zero new paths**, including **two empty guides filled**. All 21 specs carry
valid `canonical_docs` lists, all 59 referenced documents declare
`kind: canonical-documentation`, and every specification body remains
unchanged. All four final canonical lint recipes passed: Messenger, Darkmatter
(including the Zed WASM check), Claudine (including transport and lifecycle
documentation guards), and Biscuit Speaks. `git diff --check` and the final
cross-spec reference/kind/body checks passed.

Detailed test evidence is in the numbered `audit-*-tests.md` files. Mini-review
findings are in `spec.md`; only the current review agent writes that file.
