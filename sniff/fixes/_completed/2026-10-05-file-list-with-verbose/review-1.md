---
$schema: feature-review.yaml
ready: false
findings:
    - title: Hyperlink fallback changes literal filename labels
      priority: high
    - title: Acceptance tests omit required detail, rendering-work, and native-path cases
      priority: high
observations: []
human_review: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: "2026-10-06T11:23:51-07:00"
spec: 2026-10-05-file-list-with-verbose/spec.md
implemented: true
implemented_by: claude/opus
log: sniff/fixes/2026-10-05-file-list-with-verbose/log.md
description: "A **fix** review of `2026-10-05-file-list-with-verbose/spec.md`"
fix: 2026-10-05-file-list-with-verbose/review-1.md
next: 2026-10-05-file-list-with-verbose/review-2.md
---

# Review 1: File Lists With Verbose

The fix is **not production ready**. Two high findings remain: ordinary bracketed filenames change spelling in hyperlink fallback, and several explicit acceptance requirements lack tests that exercise the promised behavior. Neither requires a human design decision.

## Findings

### High — Hyperlink fallback changes literal filename labels

**Authority: spec.** The path-label contract says, “Treat names as literal text, including … brackets,” and “Use the same label spelling in rich and plain output.” Acceptance criterion 4 also requires: “Keep escaped labels literal through the list's final render.” A normal filename such as `photo[x].png` violates these requirements in piped output, including `--plain`.

**Defect class:** capability fallback adds presentation escapes to already escaped filename labels, changing their reversible spelling.

In `sniff-cli`, [file_list_item_markup](../../cli/src/output/filesystem/file_list.rs:242), which constructs each displayed filename and link, always constructs an anchor when the target is representable. When OSC8 is unavailable, `biscuit-terminal` turns this into Markdown-like text and inserts a backslash before every closing bracket. Thus:

```text
Stored path:   photo[x].png
Required label: photo[x].png
Actual label:  photo[x\].png
```

The same happens to directory segments: `dir[x]/ordinary.png` becomes `dir[x\]/ordinary.png`. This is an ordinary input already represented by the implementation's bracketed-name fixture, and occurs when users redirect output or use a terminal without hyperlinks. URLs still point to the correct files; the label is wrong.

The scope contains one category-independent list renderer, reached through the filtered image, programming-language, framework-file, and unknown projections. I swept all four using a disposable Cargo package with identical bracketed basenames and directories, changing only each file's extension and content. Each CLI invocation exited 0 with empty stderr. JSON preserved the original paths.

| Site / projection | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- |
| Image list | `photo[x].png`, `dir[x]/ordinary.png`; piped rich and plain | Extra `\` before `]` in both labels | Labels unchanged |
| Programming-language list | Same names with `.rs`; piped rich and plain | Same extra escapes | Labels unchanged |
| Framework-file list | Same names with `.vue`; piped rich and plain | Same extra escapes; framework details survive | Labels unchanged; details retained |
| Unknown list | Same names with `.zzz`; piped rich and plain | Same extra escapes | Labels unchanged |
| Shared OSC8 rendering branch | Same styled bracketed label and anchor through the actual terminal components | Clean: label unchanged after stripping OSC8/style | Label unchanged |
| Shared unlinked branch, including URL failure | Same styled bracketed label without an anchor, with capabilities on and off | Clean: label unchanged | Label unchanged |
| Other label spellings in all four CLI projections | Ordinary name, underscore, literal backslash followed by `n` | Clean: ordinary spelling preserved and native backslash doubled | Existing reversible spelling preserved |

The last rendering-branch checks used the same `Prose`-item `UnorderedList` and markup construction as the implementation, in a disposable Rust component probe; they were not real-terminal captures.

The tests currently bless the defect. In `sniff-cli`, [assert_files_list_entries](../../cli/tests/l1/cli.rs:166), the helper for checking every emitted filename, changes the expected label with `label.replace(']', "\\]")`. The label-parity unit test uses an optimistic OSC8-capable terminal and contains no bracketed path, so it does not compare capability branches.

**Recommended repair:** keep this change in `sniff-cli`: render the styled literal label without an anchor when hyperlinks are unavailable. Keep supported OSC8 links and the existing URL-failure fallback. Assert final labels against the reversible representation without inserting additional bracket escapes, and cover capability fallback with the same bracketed fixture. A dependency-wide escaping layer is unnecessary, and changing `biscuit-terminal` would require the spec's separate scope decision.

### High — Acceptance tests omit required detail, rendering-work, and native-path cases

**Authority: spec.** Acceptance criterion 1 explicitly requires “preservation of language/framework details.” Acceptance criterion 6 requires equal acquisition work with and without verbosity and states, “Rendering captured results performs no additional acquisition.” Acceptance criterion 4 explicitly includes Windows “verbatim drive and network-share spellings.” The existing tests do not exercise all of these contracts.

**Defect class:** tests assert a neighboring branch or input shape while leaving the required branch or shape untested.

In `sniff-cli`, [file_list_precedes_framework_and_language_details](../../cli/src/output/filesystem/files.rs:267), which is meant to prove that the list precedes retained details, creates only programming-language statistics; its framework collection is empty. The CLI parity fixture likewise has no framework file and selects only image, unknown, and programming-language categories. A disposable `.vue` reproduction confirmed that framework output currently works, but that result is not protected by a committed regression test.

In `sniff-cli`, [files_verbose_work_counters_match_non_verbose](../../cli/tests/l1/cli.rs:853), which compares discovery work between verbosity levels, always supplies `--json`. The command's JSON branch bypasses both root preparation and the new file-list renderer. Therefore the comparison cannot catch acquisition introduced by either new text path. Source inspection shows native joining rather than per-file probes, but the acceptance criterion explicitly asks for measured coverage.

The class sweep checked both detail projections, JSON and text work paths, and every Windows URL shape in the added native-path test:

| Site / projection | Shape tested or inspected | Observed result | Expected verification |
| --- | --- | --- | --- |
| Language details | Nonempty `.rs` category through shipped CLI and existing unit/CLI tests | Clean: `Languages:` follows `Files:`; Level 1 assertions exist | Retain these assertions |
| Framework details | Nonempty `.vue` category through shipped CLI; inspect both added detail tests | Runtime clean: `Frameworks: Vue` follows list; no corresponding committed assertion | Level 1 fixture with nonempty framework data, list parity, and detail ordering |
| Discovery with JSON verbosity | Complete fixture, `--json --perf` with and without `-v`; existing comparison rerun | Clean: counters match, but text rendering is bypassed | Keep JSON check and test the changed text path |
| Root preparation and captured-list rendering | Inspect changed text command branch and renderer tests | No acquisition-counter assertion surrounds these operations | Collect counters while preparing/rendering captured data; assert acquisition counters remain zero, ignoring presentation counters |
| Windows ordinary drive | Added test uses `C:\work` | Exact expected URL asserted | Existing Level 1 shape covered |
| Windows verbatim drive | Added test uses `\\?\C:\work` | Exact expected URL asserted | Existing Level 1 shape covered |
| Windows ordinary network share | Added test uses `\\server\share` | Exact expected URL asserted | Existing Level 1 shape covered |
| Windows verbatim network share | Inspect all added URL tests for `\\?\UNC\server\share` | Shape absent | Add this explicit Level 1 URL-identity case |

Windows rows describe the compiled test source, not a claim that native Windows tests ran during this review. The finding concerns absent assertions, not missing cross-OS execution evidence.

**Recommended repair:** extend the existing small fixture with a framework file, add a captured-renderer counter check that actually runs the new text path under collection, and extend the existing Windows URL test with a verbatim network share. These are small additions to existing tests; no new framework, large fixture, benchmark, or OS matrix is needed.

## Requirement verification

This change is a static CLI report. Level 1 verifies its data, ordering, streams, native label encoding, and URL identity. It adds no keyboard interaction requiring Level 3. The checks below do not establish real-terminal visual rendering.

| Acceptance requirement | Strongest relevant verification present | Result |
| --- | --- | --- |
| Captured-path parity, categories, retained details | Level 1 CLI parity for image, programming language, unknown; language detail unit test | Framework assertion missing; finding above |
| Unchanged reports, JSON parity, JSON with performance data | Level 1 CLI assertions, rerun | Passing |
| Verbosity placement, higher verbosity, plain escape removal | Level 1 CLI assertions, rerun | Passing; literal fallback labels still fail the separate contract |
| Names, roots, complete hyperlink identity, capability fallback | Level 1 CLI and native URL tests; manual category sweep and component probe | Bracket fallback defective; verbatim network-share case missing |
| Empty and truncated observations | Level 1 constructed breakdown shared by text/JSON; notice ordering and empty-filter tests, rerun | Passing |
| Equal acquisition work and no acquisition during rendering | Level 1 JSON counter comparison only | Changed text path untested |
| Controls and non-Unicode native values | Level 1 byte/unit round trips, native-label tests, and final list injection test, rerun | Passing for selected host tests; Windows-native cases remain CI's execution responsibility |
| URL failures, removed files, root failures | Level 1 renderer/root tests and Unix CLI error test, rerun | Passing |

## Validation performed

- `just test files_`: 70 selected tests passed.
- `just test file_list`: 40 selected tests passed.
- `just test files_link_root`: 6 selected tests passed.
- `just test 'filesystem::files::'`: 8 selected tests passed.
- `just check-tier-coverage sniff`: no stranded tests. The CLI integration module is declared by the `l1` target; ordinary added tests have no higher-tier name or feature gate.
- Disposable shipped-CLI fixtures reproduced both bracket instances in all four category projections, in piped rich and plain output, while JSON retained the native spellings. Framework details were also checked through `.vue` files.
- A disposable Rust probe checked OSC8, capability fallback, and unlinked rendering through the existing terminal components. An exploratory manufactured-PTY invocation timed out and was terminated; it is not counted as evidence.

These selected runs overlap; their counts are not a count of distinct tests. The input robustness matrix is inapplicable: the fix changes presentation and request selection, not a file-format or configuration reader. The review made no implementation or dependency changes.

## Observations

None.
