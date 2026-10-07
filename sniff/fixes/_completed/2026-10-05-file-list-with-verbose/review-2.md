---
$schema: feature-review.yaml
ready: true
findings: []
observations:
    - Existing terminal queries precede plain reports in a PTY
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: "2026-10-06T11:48:15-07:00"
spec: 2026-10-05-file-list-with-verbose/spec.md
implemented: false
description: "A **fix** review of `2026-10-05-file-list-with-verbose/spec.md`"
fix: 2026-10-05-file-list-with-verbose/review-2.md
previous: 2026-10-05-file-list-with-verbose/review-1.md
---

# Review 2: File Lists With Verbose

The fix is **production ready**. Both high findings from review 1 are resolved. This review found no remaining high, medium, or low findings against the specification. No human design decision is required.

## Previous review and repair log

Review 1 puts its two findings under `Findings`, rather than a separate `Unblocked Findings` heading. Both were actionable; it contains no blocked findings. The repair log records two fixes, no deferrals, and no disputes. There is therefore no newly unblocked or disputed finding to reconsider.

| Earlier finding | Repair verified | Result |
| --- | --- | --- |
| Hyperlink fallback changes literal filename labels | In the sniff-cli package, [render_file_list](../../cli/src/output/filesystem/file_list.rs:238) renders a styled literal label without an anchor when hyperlinks are unavailable. Final-label tests cover supported links, capability fallback, and URL failure with the same bracketed and markup-sensitive paths. Category tests cover image, programming-language, framework-file, and unknown. CLI tests compare exact labels without adding bracket escapes to expectations. | Resolved |
| Acceptance tests omit required detail, rendering-work, and native-path cases | In the sniff-cli package, [file_list_precedes_framework_and_language_details](../../cli/src/output/filesystem/files.rs:267) now uses nonempty Rust and Vue observations and asserts each entry precedes retained details. The CLI parity fixture also selects framework-file. [root_preparation_and_text_rendering_perform_no_acquisition](../../cli/src/commands/mod.rs:2701) measures the actual captured-result text path, and the Windows URL test includes a verbatim network share with an exact expected destination. | Resolved |

The repair also percent-encodes bracket and parenthesis characters in file URLs before placing them in terminal markup. This prevents the markup parser from interpreting part of a destination as another link. A decoded-path assertion and shipped-CLI PTY tests protect full destination identity.

## Unblocked Findings

None.

## Blocked Findings

None.

## Requirement verification

This fix adds a static report, using existing terminal components and hyperlink support. Level 1 is appropriate for its captured data, reversible labels, ordering, output streams, and destination identity. The PTY tests remain Level 1: they advertise terminal capabilities but do not run an actual emulator. The spec introduces no keyboard interaction, new terminal protocol, or precise glyph/color/layout promise requiring Level 2 or Level 3 evidence. This review does not claim real-terminal visual verification.

| Spec acceptance requirement | Verification inspected and exercised | Assessment |
| --- | --- | --- |
| 1. Captured paths once each, sorted; images and another category; unknown; retained details | Level 1 shipped-CLI text/JSON parity for image, programming-language, framework-file, and unknown. Constructed-result tests assert both language and framework detail ordering. | Met |
| 2. Existing reports and JSON preserved; JSON with performance data remains one document | Level 1 CLI checks cover unfiltered verbose, filtered non-verbose, aggregate filesystem, byte-identical JSON with/without verbosity, and valid JSON under `--json --perf -v`. Request routing leaves JSON outside root preparation. | Met |
| 3. Verbosity placement and level; plain output | Level 1 CLI checks cover `-v` before/after the subcommand and `-vv`, exact list entries, piped plain output, and escape removal from the new section on an OSC8-capable PTY. | Met for the changed report; existing terminal queries noted below |
| 4. Literal names, correct roots, complete URL identity, native Windows spellings, capability fallback | Level 1 small fixtures cover duplicate basenames, spaces, Unicode, brackets, markup characters, package subdirectory, absolute/relative base from outside the repository, and a base outside any package. Unix PTY tests decode complete destinations and compare native absolute paths. Unit tests cover ordinary/verbatim Windows drive and ordinary/verbatim network-share URLs. Shared renderer checks compare the same labels across capability branches. | Met; native Windows assertions inspected, not rerun on this host |
| 5. Empty and truncated observations | Level 1 empty CLI result exits successfully without a list. Constructed captured breakdown is shared by text and JSON projections; tests assert notice-before-list and notice retention with no filtered matches. | Met |
| 6. Unchanged acquisition; no additional acquisition during captured rendering | Level 1 complete-fixture CLI counters compare discovery with/without verbosity. The in-process text test separately collects root preparation and rendering, allowing only the existing ownership lookup's base canonicalizations. Scan, classification, manifest, Git, docs, subprocess, and network acquisition are not added. There are no per-entry existence or canonicalization probes. | Met |
| 7. Controls and non-Unicode native labels | Level 1 byte/code-unit round trips, escape-lookalike distinctions, native path tests, Windows separator doubling, and final list rendering test cover reversible labels and prevention of line/escape injection. | Met; Windows-only native construction remains CI's execution responsibility |
| 8. URL failure, representable targets, removed paths, root failure | Level 1 renderer tests assert unlinked literal fallback and supported native URLs. Nonexistent captured paths remain listed and linked without probes. Resolver tests cover missing absolute anchors and unreadable cwd for relative bases; CLI test asserts stderr error, nonzero exit, and empty stdout. | Met |

The sweep covered the shared label/URL renderer, the four representative category projections, both verbose detail projections, all text/JSON routing branches, and root preparation. No sibling defect remains from either earlier finding. Sorting uses native paths before label formatting; discovery and ownership stay in the sniff library. README, topic documentation, and CLI skill reference describe the new behavior.

## Validation performed

- `just test files_ file_list files_link_root root_preparation`: 115 selected tests passed.
- `just test 'output::filesystem::files::' root_preparation`: 10 selected tests passed, including all file-section projections and the captured text-work check.
- `just check-tier-coverage sniff`: zero stranded tests.

These selections overlap; their counts are not a count of distinct tests. The CLI integration module is declared by the `l1` target in a package with `autotests = false`; the reviewed tests have ordinary L1 names and do not require `test-fixtures`. Windows-specific assertions are compiled on Windows, not skipped through higher-tier names. Cross-OS execution evidence is outside this review's readiness gate.

The repair log also reports full L1, lint, and native Windows validation; those are the repair's results, not additional runs performed by this review. The input robustness matrix does not apply because this change renders captured observations rather than reading a file format or configuration. This review changes only review/spec metadata and this report.

## Observations

### Existing terminal queries precede plain reports in a PTY

The repair log notes that terminal capability detection writes query escape sequences before `--plain` report content when stdout is a PTY. In the sniff-cli package, [files_plain_verbose_strips_links_on_an_osc8_terminal](../../cli/tests/l1/cli.rs:582) consequently checks escape-freedom from `Files:` onward; it also proves the same PTY renders links in rich mode. Capability detection and plain emission predate this fix and were not changed by it, so this is not a regression or a reason to reopen this cycle. If documenting whole-stream escape-freedom, distinguish report content from terminal detection queries; any change to that detection behavior belongs in separate work.

## Recurrence

None. Both earlier defect classes are resolved; this review raises no recurring finding.
