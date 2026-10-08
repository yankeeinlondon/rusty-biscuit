---
$schema: feature-review.yaml
ready: false
findings:
  - title: "R1-F1 — Real-terminal stats assertions still enforce the old report and omit package rendering coverage"
    priority: high
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: "2026-10-08T06:40:20-07:00"
spec: 2026-10-10-stats-package-grouping/spec.md
implemented: true
description: "A **feature** review of `2026-10-10-stats-package-grouping/spec.md`"
feature: 2026-10-10-stats-package-grouping/review-1.md
log: tree-hugger/features/2026-10-10-stats-package-grouping/log.md
implemented_by: codex/gpt-6.1-sol
next: 2026-10-10-stats-package-grouping/review-2.md
---

# Review 1: Stats package grouping

The feature is **not ready for production**. The package attribution and JSON behavior passed the focused checks, but the existing real-terminal stats test fails against the intentional new report. All three terminal backends share that obsolete assertion, and named package rendering has only subprocess coverage.

There are no previous implementation reviews for this feature and therefore no carried findings or partial repairs to reconcile. This review adds one blocking finding. No human decision is needed to repair it. The implementation log records authorization for automatic grouping and manifest-backed identity during Phase 2; this review does not reopen those choices.

## R1-F1 — High: Real-terminal stats assertions still enforce the old report and omit package rendering coverage

**Defect class:** output contract changes were applied to subprocess snapshots but not to the shared real-terminal assertions, leaving a failing verification tier and missing verification of the new package sections.

In tree-hugger-cli, [check_stats](../../cli/tests/level2_declarations.rs:632) verifies the tables captured from a real terminal. Its exact expected rows still begin with the Kind table and end after one documentation coverage line. The implementation now deliberately emits an overall heading and additional group sections. The first comparison at line 671 fails before the narrower-width case can execute. This is an expected-output defect: the captured headings and additional tables agree with the new specification.

The reproduction used the existing fixture without edits:

```sh
cd tree-hugger
BISCUIT_TEST_REQUIRED_BACKENDS=tmux BISCUIT_L2_THREADS=2 \
  just test-l2 level2_declarations_in_tmux --test level2_declarations
```

Nextest run `37b6a9b2-dfd6-4af9-84bd-7777ccd8f7f2` failed after 5.890 seconds at `level2_declarations.rs:671`. Backend proof recorded `tmux run=1 skip=0 panic=0`; this was an executed real-terminal test, not an availability skip. The captured report had `All selected declarations`, the existing overall tables, then `Unassigned` and another complete set of tables. The old expected rows omitted both headings and the entire second section.

The independently expected result for this fixture is an overall section followed by Unassigned, each with 12 selected declarations and coverage `0 of 11 documented (0.0%); 1 unavailable`. The captured output has those values. No workspace catalog in this fixture assigns the selected files to a declared package.

### Instance and consumer sweep

| Child ID | Site | Shape tested or invariant checked | Observed result | Expected result | Verification evidence |
| --- | --- | --- | --- | --- | --- |
| R1-F1.S1 | tmux `level2_declarations_in_tmux` | Existing mixed-language fixture, ordinary terminal width | Fails on the new overall heading and Unassigned section | Exact grouped report accepted; totals unchanged | Executed reproduction above; call to shared helper at line 942 |
| R1-F1.S2 | WezTerm `level2_declarations_in_wezterm` | Same fixture and shared exact comparison | Same obsolete expectation; not executed in this review | Accept the same grouped report, preserving backend-specific capture behavior | Call at line 994 reaches the identical `check_stats` comparison; `shown_rows` preserves nonblank headings, so the new first row cannot equal its expected table border |
| R1-F1.S3 | Kitty `level2_declarations_in_kitty` | Same fixture and shared exact comparison | Same obsolete expectation; not executed in this review | Accept the same grouped report | Call at line 1022 reaches the identical comparison; the same first-row invariant applies |
| R1-F1.S4 | Shared narrow-width branch | `COLUMNS=20`, after the ordinary-width comparison | Unreachable in the failing run; expected rows also omit all new sections | All sections retained and existing natural table width preserved | Loop at lines 669–671 uses the same old array for both widths |
| R1-F1.S5 | Named package headings and sections in L2 | Manifest names, duplicate names at distinct paths, Unicode/markup/control text, empty package counters | No L2 stats fixture exercises named groups or asserts their literal text/style | A real-terminal capture verifies named sections, ordering, literal safe text, and zero-group rendering | Sweep of all `level2*` tests finds only this stats helper; its fixture produces Unassigned |
| R1-F1.S6 | L1 text reports | Named duplicate packages, empty groups, Unicode/markup/control names; terminal/plain/table subprocess modes | Clean at L1 | Ordered sections, correct per-section counts, literal headings, plain output without escapes | Passing `grouping_reports::terminal_sections_are_ordered_and_literal` and `grouping_acceptance::text_modes_match_json_sections_and_counts`; explicit expected headings and counts, rather than output agreement alone |
| R1-F1.S7 | L1 shipped snapshots | Existing multilingual corpus, terminal and JSON stats | Expectations were updated for grouped output | Overall heading plus Unassigned; additive JSON fields | `tests/fixtures/golden/symbols.stats` and `symbols-stats.json`; declared `symbols::golden` tests |

WezTerm and Kitty are classified from their precise shared-call invariant, not claimed as executed failures. No other real-terminal stats consumer was found. JSON, plain output, and the four declaration commands use the same counter/report paths; their L1 checks do not replace terminal rendering evidence.

**Required repair:** update the shared expected report and its stale documentation to the grouped contract. Extend the shared L2 checks with a temporary workspace containing named groups and Unassigned, checking both required sections and absence of incorrect extra sections. Assert heading style and literal handling through captured terminal output, including an empty named group and the existing narrow-width behavior. Retain the old total/count assertions. Run the affected terminal tests through the area's live `test-l2` recipe without taking focus. Do not merely discard headings or truncate the capture to make the old assertion pass.

## Requirement verification

| User-facing requirement | Strongest reviewed coverage | Outcome |
| --- | --- | --- |
| Cargo/npm membership; standalone package without Git; separate library and CLI names | L1 public catalog and shipped CLI tests | Passed focused checks |
| Deepest owner; unnamed nested boundary; root/virtual packages; prefix siblings; duplicate names | L1 `package_identity` and grouping tests with explicit expected identities | Passed |
| Active linked worktree; separate scan/discovery roots; outside-root files | L1 shipped CLI tests | Passed |
| File/declaration filters and classes counting containers | L1 shipped CLI tests and accumulator unit tests | Passed; counters and group presence asserted |
| Group sums, unequal-size documentation ratios, unavailable documentation | L1 report and accumulator tests | Passed; independently expected counts/ratios present |
| Zero-match or failed selected files; successful empty changed selection | L1 grouped reports and partial-error tests | Passed focused cases; zero groups and top-level errors asserted |
| Synthetic cross-package exports belong to the prelude | L1 unit and shipped CLI tests | Passed; target package is not spuriously counted |
| Catalog absence versus returned discovery error; non-stats isolation | L1 injected discovery and shipped CLI tests | Passed; no success envelope on discovery failure |
| Native aliases and cache hits after manifest rename | L1 shipped CLI tests | Passed on macOS; Windows/Linux-specific cases are target-gated, with execution left to CI |
| Additive JSON shape, field order, sorted groups, trailing newline | L1 JSON reports and snapshots | Passed focused checks |
| Plain output, exact section counts, option conflicts across all four commands | L1 subprocess tests | Passed; L1 is appropriate for these contracts |
| Real-terminal grouped sections, safe literal headings, styling, and width behavior | L2 exists but fails; named-section checks only L1 | **Blocked by R1-F1** |

This feature has no keyboard, mouse, paste, or input-encoding behavior requiring L3. Package assignment needs L1 filesystem/subprocess evidence. The new terminal presentation needs L2 capture evidence. Cross-OS proof is not used as a readiness gate for this review.

## Input robustness and ownership trace

Reviewed the plan's per-format matrix and ran the public-result tests in `sniff::l1::package_identity`. They start from checked-in tool-produced fixtures and assert declared versus fallback identity through the catalog and ownership lookup. The following outcomes are encoded in those tests; all selected matrix tests passed.

| Shape | Cargo `package.name` | npm `name` | Python `project.name` | Go `module` | Composer `name` | Serialized `name_source` |
| --- | --- | --- | --- | --- | --- | --- |
| Unedited control | Declared | Declared | Declared | Declared | Declared | Manifest retained |
| Absent | Fallback | Fallback | Fallback | Fallback | Fallback | Legacy fallback |
| Explicit null | Invalid TOML → fallback | Fallback | Invalid TOML → fallback | Bare `null` is a path token | Fallback | Rejected |
| Wrong whole-field type | Fallback | Fallback | Fallback | List/object syntax falls back; bare `123` is a path token | Fallback | Rejected |
| One/all wrong elements | Fallback | Fallback | Fallback | Fallback | Fallback | Rejected |
| Empty | Fallback | Fallback | Fallback | Fallback | Fallback | Rejected |
| Duplicate | Fallback | Existing JSON last-wins | Fallback | Fallback | Existing JSON last-wins | Rejected |
| Trailing/invalid content | Fallback | Fallback | Fallback | Fallback | Fallback | Rejected |

Malformed required root workspace manifests can return discovery errors rather than member fallback; the returned-error tests preserve that distinction. JSON duplicate handling is an explicitly recorded existing-reader policy. Absent and null manifest names intentionally have the same unusable-identity outcome; absent and null persisted provenance remain distinct. The fallback serialization omission is tested as legacy compatibility, not silently treated as declared identity.

The decision chain was checked end to end: sniff resolves a nonblank manifest name and its provenance; the reusable ownership lookup chooses the deepest native component boundary before name usability; tree-hugger-cli checks `Manifest` before creating a named group; file selection initializes even empty or failed groups; already-filtered records update overall and per-group accumulators; one JSON projection and the grouped terminal writer consume the sorted counters. An unnamed nested owner cannot fall through to its named parent. Synthetic exports are attributed from the analyzed prelude path, not their target's display location. Group labels are captured on each request and never written into persistent symbol entries. No additional defect was established in that chain.

## Verification record

- `tree-hugger/`: `just test grouping --test-threads 4` — 30 passed, 1,908 outside the selected filter.
- `tree-hugger/`: `just test --test symbols --test-threads 4` — all 527 passed, zero skipped, including both stats goldens, report compatibility, selection, and cache tests (187.295 seconds).
- `sniff/`: `just test package_identity --test-threads 4` — 14 passed, 3,189 outside the selected filter.
- Repository root: `just check-tier-coverage tree-hugger` — passed, zero stranded tests. The new modules are registered in the declared `symbols` and sniff `l1` targets; ordinary names select L1. The L2 target requires `terminal-tests`, which the live recipe and CI metadata enable.
- Headless tmux command above — one executed L2 failure, two other backend tests outside the filter; no terminal window opened or focused.

The code and current topic/README/release documentation otherwise agree on automatic grouping, path frames, failure behavior, and additive report version 1. Discovery and ownership are request-scoped; assignment occurs once per selected file and only counters are retained. No performance change is recommended on the evidence reviewed. No implementation code was changed in this review.
