---
$schema: feature-review.yaml
ready: true
findings: []
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: "2026-10-05T01:21:58-07:00"
spec: 2026-10-03-symbol-improvements/spec.md
implemented: false
description: "A **feature** review of `2026-10-03-symbol-improvements/spec.md`"
feature: 2026-10-03-symbol-improvements/review-3.md
previous: 2026-10-03-symbol-improvements/review-2.md
---

The feature is **production ready** within its specified scope. Both unblocked findings from the previous review are resolved, and the broader review found no additional blockers. No human decision is required.

## Previous review follow-through

I checked the two findings in review-2.md's `Unblocked Findings` first. Its `has_blocked_findings` and `blocked` properties were false, and it contained no blocked findings. There was therefore no blocked item that became actionable before the latest implementation.

| Previous finding | Result | Evidence |
|---|---|---|
| Real-terminal assertions can inspect an earlier command's output | Resolved | Each command has unique begin/end markers; plain rows, effective styles, long lines, and links are checked within that report. Both declaration and god-files targets use the shared helper. The retained-output regression tests and live tmux/WezTerm tests pass. |
| Package heading detection still buffers the entire first package | Resolved | Package sections are decided from discovered files before analysis. The rendered-byte holding buffers are gone. Uneven-package tests and independent CLI probes stop within one worker chunk for the tested preclosed pipes. Both JSON writers also flush at chunk boundaries, including batches without entries. |

## Scope and verification

I used the tree-hugger and rust-testing skills and read the specification, both earlier reviews, implementation hand-off, repair log, relevant source, current documentation, and test registration. The review covered header/keyword extraction, selective queries and pass dependencies, source snapshots, cache fingerprints and coverage, projection, prelude reuse, declaration ordering, streaming, and terminal rendering. Implementation code was left unchanged.

| Check run during this review | Result |
|---|---|
| `just test` in `tree-hugger/` | 833 passed, 0 skipped |
| `just lint` in `tree-hugger/` | Passed for both packages |
| `cargo clippy -p tree-hugger-cli --features terminal-tests --all-targets -- -D warnings` | Passed |
| `just check-tier-coverage tree-hugger` | 0 stranded tests |
| `BISCUIT_TEST_REQUIRED_BACKENDS=tmux,wezterm just test-l2` | Both declaration and god-files tests executed and passed on each required backend; backend proof reports two executions each |
| Independent copied-fixture JSON probes | All four commands × both JSON modes × three orders: 24 cases passed record ordering and repeat-output equality |
| Independent uneven-package pipe probes | 26 cases passed; each parsed 8 of 97 files and exited quietly with status 0 |

The Level 2 runner reported six passing tests, but the two Kitty tests returned through their unavailable-backend gates. This is not Kitty rendering evidence. No cross-OS execution was attempted in this review; that evidence belongs to CI and does not determine readiness here. No keyboard interaction was introduced, so Level 3 is unnecessary.

Independent probes ran the built `target/debug/hug` in temporary Git repositories with test-owned home/cache directories, non-interactive Git initialization, and two Rayon workers. JSON probes copied the shipped 16-language `multi/` tree. Pipe probes copied its Python fixture into 96 files in package `a`, with a final file in package `z`, first declaring and then comment-only. Temporary files were removed afterward.

## Defect-class sweeps

### Terminal evidence must belong to the current command

The earlier defect class was that assertions searched shared terminal history without identifying the report under test.

In tree-hugger-cli, [run_scoped and Report](../../cli/tests/support/terminal_report.rs) now delimit the current command's output and reject missing, repeated, or reversed markers rather than falling back to the whole capture. Row-specific checks reject ambiguous matches. Style checks replay the terminal's effective style, including attributes carried from earlier rows; this matters because tmux emits style changes rather than a complete style on every row.

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| Shared report reader | Earlier correctly styled duplicate, broken current heading | Current heading fails the style assertion | Same; clean |
| Shared report reader | Earlier differently styled duplicate, correct current heading | Current heading passes | Same; clean |
| Shared row selection | Row exists only in an earlier report; ambiguous current matches | Earlier row unavailable; ambiguous match rejected | Same; clean |
| Shared marker reader | Typed command, missing/repeated/reversed delimiters | Typed command is not a delimiter; invalid regions rejected | Same; clean |
| Functions and types | Sequential reports in the same terminal history | Current ordered rows and style assertions pass on tmux/WezTerm | Same; clean |
| Grouped symbols and directory grouping | Earlier file locations remain in history | Current headings and locations pass | Same; clean |
| Classes | Earlier compact methods and file locations remain in history | Current file heading, sections, methods, fields, and locations pass | Same; clean |
| Long declaration | Terminal soft wrapping | Report-local logical-row assertion passes | Same; clean |
| Location links | WezTerm OSC 8 capture | Current matched rows contain the expected links | Same; clean |
| God-files sibling target | Shared history and rendered risk styling | Uses the same scoped helper; tmux/WezTerm pass | Same; clean |
| Kitty sibling paths | Source/registration inspection; backend unavailable | Both targets use the same scoped helper and checks | Correct shared implementation; no execution claim |

These checks close the earlier finding across text, style, long-line, and link assertions, rather than merely fixing the first failing heading.

### Layout decisions must not delay file-order writes until a package ends

The earlier defect class was that a data-dependent heading decision retained a whole package before sending output to the writer.

In tree-hugger-cli, [ListingStream](../../cli/src/render.rs) decides whether sections are needed from the discovered package count. It retains the current ordering unit only: a file for ordinary file order, a directory for directory grouping, or a package where flat name/kind order or prelude projection requires it. [write_json and write_terminal](../../cli/src/declarations.rs) flush at chunk boundaries so a pending envelope or small report cannot hide a closed pipe.

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| Functions/types/symbols, default layout | 96-file first package; declaring or empty later package; preclosed reader | Independent probes: 8 of 97 files parsed for each | Stop within bounded scheduled work; clean |
| Same commands, file grouping | Same two package shapes | 8 of 97 parsed for each | Same; clean |
| Same commands, directory grouping | Same two package shapes | 8 of 97 parsed for each | Same; clean |
| Same commands, both grouping flags | Same two package shapes | 8 of 97 parsed for each | Same; clean |
| Classes terminal control | Same two package shapes | 8 of 97 parsed | Same; clean |
| Listings and class/JSON controls | Uneven first package; reader closes after initial bytes | Registered early-reader tests pass | Stop before completing the large first package; clean |
| Default/full JSON, all four commands | 96 comment-only files and preclosed reader | Registered no-entry test passes its one-chunk bound | Flush even when no default entries were added; clean |
| Default/full JSON, all four commands | Uneven declaring packages | Registered preclosed-pipe controls pass | Same bounded producer behavior; clean |
| Single-package/file-order controls | Existing pipe regressions | Pass | Same; clean |
| Flat name/kind ordering and prelude | Source review and ordering/prelude regressions | Package-sized ordering unit retained intentionally | Preserve required package-wide ordering/projection |
| Imports/lint/god-files | Scope and source inspection | Separate legacy/ranked output paths | Outside this feature's output contract |

The package-heading policy has one intentional layout departure: scans spanning several packages now show sections even when fewer than two packages contain matching declarations. Empty packages retain `(no symbols)`. The repair log records the departure, the current report documentation explains it, and exact-output tests cover it. Deciding this from discovery removes the need to wait for later declarations. This is a reasonable implementation tradeoff and requires no further design decision.

## Requirement-to-verification assessment

| Requirement | Strongest relevant verification | Assessment |
|---|---|---|
| Declaring keywords and contiguous display headers across the accepted declarations in all 16 languages | Level 1 keyword/signature matrices, comparison examples, unsupported-shape cases, and coverage guard | Appropriate; pass |
| Selective kinds/capabilities, prerequisite ordering, one definition capture, skipped diagnostics, unknown recursion | Level 1 public query tests and work counters | Appropriate; pass |
| Snapshot bytes are read/hash/parsed consistently; equivalent file spellings share identity | Level 1 public snapshot/error and CLI spelling tests | Appropriate; pass |
| Cache hits skip parsing; invalidation, narrowing/widening, write-back, concurrency and recoverable failures | Level 1 public API tests and shipped-CLI tests | Appropriate; pass |
| JSON selection, complete indexes, errors/exit status, portable paths and stable-key projection | Level 1 shipped-CLI tests | Appropriate; pass |
| Shared ordering and cold/warm/sequential/parallel repeatability | Level 1 tests plus 24 independent command/mode/order probes | Appropriate; pass |
| Prelude aliases, unresolved exports, environment names, external targets and parse reuse | Level 1 shipped-CLI prelude tests | Appropriate; pass |
| Class selection, field flags and static/instance partitioning | Level 1 library and CLI tests | Appropriate for data; pass |
| Compact styled labels, grouped/package headings, locations, class sections and long rows | Level 2 current-report captures on tmux and WezTerm | Appropriate; pass |
| OSC 8 location links | Level 1 byte assertions and Level 2 WezTerm capture | Appropriate; pass |
| Plain output and quiet/verbose stderr | Level 1 goldens and isolated subprocess tests | Appropriate; pass |
| Bounded default output and early pipe termination | Level 1 subprocess/work-counter tests and independent uneven-package probes | Appropriate; pass |
| Performance goals | Recorded before/after release measurements, supported by Level 1 work counters | Accepted supporting evidence; timing sample not repeated |

Test placement is valid. The library's consolidated L1 target declares the symbol modules. The CLI `symbols` target declares the new terminal-scope regressions and report tests. Both real-terminal targets are declared with `terminal-tests`, CI enables that feature, and the live `test-l2` recipe selects them. The tier audit found no stranded tests.

## Input robustness assessment

The file-format reader affected by this feature is the JSON symbol cache. The latest repair adds no new persisted field or supported file format. The existing [cache robustness matrix](../../cli/tests/symbols/cache_robustness.rs) and [cache design contract](../../docs/cache-design.md) were reviewed and their tests passed through public CLI results.

The matrix uses a CLI-written fixture, restores the entry before each edit, and changes a sentinel record so the public output distinguishes a served entry from recomputation. Columns cover entry identity, language, hash, fingerprint, coverage, diagnostics fingerprint, records/imports/exports/diagnostics, and the tested record schema/kind/container/visibility/modifier/header/declaration/relations/type-field values. Each column states its outcome for absent, null, wrong whole type, one wrong element, every element wrong, empty, duplicate key, and trailing invalid content; optional values and empty collections retain their documented schema meaning.

Both version-bearing sites have the additional compatibility checks:

| Shape | Entry schema version | Record schema version |
|---|---|---|
| Current 4.0 control | Served | Served |
| Absent or explicit null | Rejected/recomputed | Rejected/recomputed |
| Wrong whole type, one wrong member, or every member wrong | Rejected/recomputed | Rejected/recomputed |
| Empty object or duplicate field key | Rejected/recomputed | Rejected/recomputed |
| Trailing invalid document content | Rejected/recomputed | Rejected/recomputed |
| Incompatible major or minor | Rejected/recomputed | Rejected/recomputed |

No additional malformed-input defect was reproduced. Source-language grammar support is bounded by the specification's accepted extraction set; this review does not enlarge that set.

## Unblocked Findings

None.

## Blocked Findings

None.

## Recurrence

No finding in this review repeats an earlier defect class. The recurring package-buffering defect reported in review-1.md and review-2.md is now closed across the affected listing layouts and JSON controls. The terminal capture-isolation defect from review-2.md is also closed across both terminal targets.

## Closure

The previously accepted departures remain acceptable: synthetic prelude records reuse the existing synthetic visibility marker; v1 convenience APIs remain; the v1 model has no recursion field to map; and cache widening reparses where a pass needs live syntax nodes. The package-heading departure above is documented and verified. No additional optimization or ergonomic change is required for production readiness.

This review updates review-2.md's link to this report and confirms its existing `implemented: true`. The specification now has `review_iterations: 3` and `completed: true`, as requested. The active feature directory has not been moved; lifecycle closure remains the author's responsibility.
