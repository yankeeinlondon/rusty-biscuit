---
$schema: feature-review.yaml
ready: false
findings:
  - title: Required post-feature performance sample has no recorded results
    priority: medium
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: "2026-10-05T16:44:03-07:00"
spec: 2026-10-03-symbol-filters-and-reports/spec.md
implemented: true
implemented_by: claude/opus
log: tree-hugger/features/2026-10-03-symbol-filters-and-reports/log.md
description: "A **feature** review of `2026-10-03-symbol-filters-and-reports/spec.md`"
feature: 2026-10-03-symbol-filters-and-reports/review-7.md
previous: 2026-10-03-symbol-filters-and-reports/review-6.md
next: 2026-10-03-symbol-filters-and-reports/review-8.md
---

The feature is **not production ready under the specification's completion requirements**. Review-6's Go documentation finding is repaired across its entire matrix, and this review establishes no new incorrect filter or report result. One unblocked medium-priority requirement remains: the required performance sample after this feature has no recorded results. No human design decision is needed.

## Previous review follow-through

Review-6 had one Unblocked Finding and no Blocked Findings. Reviews 1–5 also had no blocked findings, so no newly unblocked work needs reconciliation.

| Previous requirement | Verification | Assessment |
|---|---|---|
| Preserve absent versus attached-empty Go comments | In the tree-hugger library, [extract_doc_comment](../../lib/src/file/extract/doc_comments.rs:24), which chooses a declaration's documentation, now consumes `Attachment::Absent`, `Empty`, or `Text`. Only absent Go attachments fall back to the wrapper. | Implemented. |
| Sweep all six Go sites and all 21 comment shapes | The [registered public-result matrix](../../lib/tests/l1/symbols_flags.rs:1451) covers grouped types, aliases, vars, consts, multi-name vars and the extracted multi-name const. An independent copied-fixture CLI matrix checked all 126 site/shape cells and all 147 extracted names. | All expected documentation and deprecation facts agree. |
| Preserve clean attachment controls | Absent and detached comments inherit; empty, whitespace-only, empty-block and all directive-only attachments suppress inheritance; own prose and own markers win; directive lookalikes remain prose. | All 21 shapes pass at every Go site. |
| Preserve other languages' wrapper behavior | Independent copied-fixture probes exercised six shapes—absent, empty, blank block, own prose, own marker and ordinary comment—at C/C++ single linkage declarations, C++ templates, Scala single-method extensions and JS/TS variable statements. | All 36 controls retain existing behavior. C/C++/Scala child text wins when nonempty; JS/TS variable records retain their statement documentation. Go suppression was not imposed globally. |
| Verify public consumers | Independent CLI checks compare complete JSON, JSON Lines, undocumented/deprecated counts and stats across the Go matrix. The [CLI regression](../../cli/tests/symbols/filters.rs:883) checks the original reproduction through these formats too. | Corrected facts reach every consumer. |
| Invalidate stale extraction and verify genuine cache reuse | Extractor revision is 17; the [revision-16 cache regression](../../cli/tests/symbols/cache_upgrade.rs:993) carries the old wrong flags. An independent complete-index cold/warm pair produces identical corrected output; the warm run reports one hit, zero parses, zero captures and zero diagnostics runs. | Implemented. Revision 16 was used by the concurrent kind-precision work; the separate revision-17 bump covers this repair. |
| Update comments, documentation and skill | The helper contract, [current documentation](../../docs/symbol-kinds-and-visibility.md:667) and tree-hugger skill distinguish absent and attached-empty Go groups. | Implemented. |

Earlier repairs were checked against their regression coverage: patterned and curried parameters, qualified owner identity, cached and reused file-wide test flags, Python docstring forms, fence boundaries, Go literal stars and directives, language-specific block decoration, and real-terminal report views. No recurring functional defect was established.

## Scope and verification

Used the tree-hugger, rust-testing and biscuit-test-harness skills. Read the spec, plan, both implementation logs, reviews 1–6, metadata extraction, documentation attachment and normalization, predicate matching, query finishing/narrowing, cache projection and validation, class selection, report writers, rendering, current documentation and test registration. Existing extraction limits and concurrent kind-precision work were preserved. This review changes review documents and metadata only.

Independent probes used a newly built `target/debug/hug`, copies of the primary language fixtures in temporary Git repositories, explicit file selection and `--no-cache` except the isolated cache probe. The Go matrix retained the primary fixture's controls and checked each added name's public result. No source implementation or test was modified.

| Check | Result |
|---|---|
| `cargo build -p tree-hugger-cli --color=never` | Passed. |
| Initial `just test -j 4` | 633 passed; the 16-language selector matrix timed out at 30 seconds; 387 tests were not run. |
| Isolated selector-matrix run through `just test -j 1` | Passed in 7.805 seconds. The initial timeout did not reproduce without competing tests. |
| Full `just test -j 2` | 467 passed; the all-language cache-projection comparison timed out at 30 seconds; 553 tests were not run. |
| Feature run, `just test --profile ci -j 2 symbols_` | All 247 selected tests passed; 774 were excluded by the filter. Covers every library `symbols_*` module plus matching CLI tests. The existing CI profile allows 90 seconds per test and still disables retries. Both previously timed-out matrices passed: cache projection in 12.359 seconds and selector coverage in 4.211 seconds. |
| All CLI tests, root `just test tree-hugger-cli --profile ci -j 1` | All 280 passed, zero skipped, including malformed-cache matrices, all stale revisions, report goldens and the Go CLI regression. |
| `just lint` | Library and CLI passed. |
| `just test-l2` | Six selected entry points passed. tmux and WezTerm ran real-terminal captures; both Kitty entry points used the unavailable-backend path. 280 tests were outside the selected tier. |
| Root `just check-tier-coverage tree-hugger` | Zero stranded tests. |
| Independent Go CLI matrix | All 126 site/shape cells and 147 extracted names passed, including count, coverage, JSON Lines and a genuine warm cache hit. |
| Independent non-Go wrapper matrix | All 36 controls passed. |

Neither default-profile full-area attempt completed cleanly. Their timeouts remain recorded above; they are not replaced by a claim that all 1,021 tests passed. The subsequent selected runs complete verification of the feature matrices and every CLI test. The shorter isolated and CI-profile timings support resource contention as the explanation for the two timeouts; no assertion failure or reproducible incorrect result was found.

### Requirement-to-verification assessment

| User-facing requirement | Appropriate level | Verification present |
|---|---|---|
| Scope, async, qualified names, containment and parameter/type predicates | Level 1 | All-language public-result matrices, query comparisons and focused CLI regressions. |
| Test/deprecation flags, documentation presence, unsupported metrics and selector intersections | Level 1 | Registered matrices, filesystem-transition and reuse tests, CLI counts and coverage; independent attachment matrix above. |
| Smart case, exact names, qualified containers, file selection, imports and command conflicts | Level 1 | Public query and isolated subprocess tests, including non-ASCII names and class container/member separation. |
| JSON, JSON Lines, count/stats, envelopes, deterministic order, partial failures and broken pipes | Level 1 | Structural subprocess assertions, output goldens, thread/cache comparisons and pipe tests. |
| Cache widening/projection, stale revisions, stable IDs and scan-root-dependent display names | Level 1 | Public cache tests, revision-upgrade tests and hit/parse counters; independent warm-run control above. |
| Styled lists and class sections; outlines, stats, signature modes, table rendering, wrapping and applicable links | Level 2 | Shared declaration checks executed in tmux and WezTerm. |
| Plain/redirected text and literal source-control escaping | Level 1 | Text/byte assertions and goldens. |
| Keyboard, mouse, paste and input methods | Not applicable | The report commands introduce no interactive input behavior; Level 3 is unnecessary. |

The library's declared consolidated `l1` target includes its matrix modules. The CLI `symbols` target includes filters, formats, goldens, cache robustness and upgrades. Both terminal targets declare `required-features = ["terminal-tests"]`; the live `test-l2` recipe and CI metadata enable that feature. No test was added or renamed by this review. No mismatch between a report requirement and its verification level was found. Cross-OS evidence is a CI responsibility and does not determine readiness here.

### Input robustness matrix

The relevant structured reader is the persistent JSON cache. Its [documented field-by-shape matrices](../../docs/cache-design.md) and [public-output matrix](../../cli/tests/symbols/cache_robustness.rs:623) cover entry identity/coverage and the feature's record fields: scope, qualified name, namespace rooting, async modifiers, test/deprecation flags, metrics-facet presence, parameter count and documentation presence. Each field has control, absent, explicit-null, wrong whole-field type, one wrong element, every element wrong, empty, duplicate-key and trailing-invalid outcomes, including inapplicable shapes. Required metric fields distinguish absence from a nullable unsupported value. The served-entry sentinel distinguishes accepted entries from recomputation.

The complete cache-format matrix passed in the CLI run. The cache reader's defaults and optional fields were inspected against those outcomes. Entry fields have no permissive defaults; schema-defined absent modifier lists mean no modifiers, while null or malformed lists are rejected. The Rust package test rule checks manifest existence without parsing manifest contents. Source metadata readers consume syntax trees and introduce no additional configuration format. The Go attachment matrix checks source-presence semantics separately from serialized-field semantics.

## Unblocked Findings

### Medium — Required post-feature performance sample has no recorded results

**Defect class:** a completed implementation checklist claims the required performance comparison is finished without recording the measurements needed to assess it.

The specification's **Definition of done** requires the prerequisite feature's performance sample to be rerun after this feature and recorded in the implementation log. The plan's **Phase 7: Docs, release notes, performance sample and close** marks that task complete and explicitly requires elapsed time, serialized bytes, peak memory, build state and cache state.

However, [implementation-log.md](./implementation-log.md) declares `completed_phase: 7` but its body stops after Phase 5. It contains no performance sample, timings, serialized-size measurement or peak-memory comparison for this feature. [log.md](./log.md), which records all six repair cycles, also contains no such sample. The README's speed, memory and JSON-size figures appear in the separate **record-native symbol reports** release notes and reproduce measurements from the prerequisite feature. They cannot establish the cost of the metadata, filters and reports added here.

This is an incomplete specification requirement, not a demonstrated runtime slowdown. The spec establishes no numerical performance gate; a measured regression can be recorded with its cause without introducing a benchmark threshold or requesting a human decision.

#### Complete measurement and record sweep

The same check was applied to every measurement in the prerequisite sample: look for a post-feature result, its input, build/cache state and comparison in both logs and the release notes. These are verification records rather than source parsers; a manufactured malformed source fixture would not test the missing record.

| Site / measurement | Shape checked | Observed result | Expected result |
|---|---|---|---|
| `hug functions --no-cache` | Release-build elapsed time on the baseline package input | No post-feature measurement in either log. README has the prerequisite's 0.123-second result, rounded to 0.12 seconds. | A new elapsed-time result with the input and release-build identity. |
| `hug functions`, warm cache | Elapsed time and explicitly warm cache state | No post-feature measurement. README retains the prerequisite's 0.044-second result, rounded to 0.04 seconds. | A new warm result with cache state stated. |
| `hug lint --no-cache` | Baseline comparison command | No post-feature result. | The same baseline command recorded as part of the requested sample. |
| `hug imports --no-cache` | Baseline reference command | No post-feature result. | The same reference command recorded. |
| `hug --json functions --no-cache` | Serialized bytes on the same baseline input | No post-feature size result. README retains the prerequisite's 2.7 MB. | New byte count with its input identified. |
| Repository-wide `hug symbols --no-cache` | Peak resident memory and elapsed time | No post-feature memory comparison. README retains the prerequisite's 1.95 GB. | New peak-memory comparison, noting this feature's deliberate default scope change. |
| `implementation-log.md`, Phase 7 | Claimed completion versus supporting record | Frontmatter says Phase 7 completed; narrative ends at Phase 5. | A compact section containing the required sample and any explained departures. |
| `log.md`, repair cycles 1–6 | Alternative location for a later sample | Correctness/test records are present; no performance measurements. | If the sample was run during a repair, preserve its results in the designated implementation log. |
| README release notes | Origin of existing speed/size/memory figures | Correctly describe the prerequisite feature; new-feature smart-case and scope examples are present. | Clean documentation control. Keep the prerequisite figures attributed to that change; they do not substitute for the new sample. |

Rerun the bounded release-build sample specified by the plan, using the prerequisite's host, commands and inputs, and record the results in `implementation-log.md`. State the source/build identity, input tree, cache state, elapsed times, JSON bytes and peak-memory comparison; explain any regression or necessary input departure. If the measurements were already taken, restore their actual recorded results instead of asserting completion without them. No additional hosts, performance CI cells, benchmark corpus or thresholds are requested.

## Blocked Findings

None. The remaining work is an agent-executable measurement and documentation task.

## Recurrence

Compared with findings in reviews 1–6. No earlier review reports the missing post-feature performance sample; `recurrence: false`. The Go attachment class from review-6 is repaired across its complete site/shape matrix and does not recur here.

## Closure

Recorded one unblocked medium-priority finding. Review-6 is linked forward and marked implemented, and the specification's review count is seven. The specification remains incomplete. No commit, implementation repair, formatting run or lifecycle-directory move was made.
