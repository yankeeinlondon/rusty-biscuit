---
ready: false
agent: codex
model: ""
---

# Review 2: God Files

## Findings

### High: Styled terminal output and OSC8 links only have Level 1 verification

The spec explicitly requires the pretty report to be rendered through `biscuit-terminal` `Prose`, including color tags, underline/bold section headings, OSC8 hyperlinks, and Unicode report glyphs such as the ellipsis and middle-dot signal separator (`spec.md:59-61`, `spec.md:244-281`). The implementation does render through `Prose` and conditionally emits OSC8 links (`tree-hugger/cli/src/main.rs:3040`, `tree-hugger/cli/src/main.rs:3143`, `tree-hugger/cli/src/main.rs:3187`, `tree-hugger/cli/src/main.rs:3198`, `tree-hugger/cli/src/main.rs:3241`), but the tests are all plain in-process/assert_cmd tests (`tree-hugger/cli/tests/cli.rs:1544-1770`).

Strongest verification present: **Level 1**.

Required verification: **Level 2** for the user-observable styled terminal behavior. Per the review rubric, byte-level assert_cmd coverage is not enough to verify that SGR styling, OSC8 link degradation, glyph widths, indentation, and wrapping render correctly through a real terminal emulator. Add `level2_god_files_pretty_report_in_<backend>` coverage using the shared terminal harness recipe, capturing pane text and, where the harness supports it, verifying styled output behavior through a real terminal. The existing Level 1 tests are still useful for JSON/plain shape and raw forced OSC8 bytes, but they do not close the terminal-rendering requirement.

This blocks production readiness under the requested test-rigor rules.

### Medium: A nonexistent scan directory is reported as an empty successful scan

`GodFiles::discover_candidates` collapses all scanner errors except `NoSourceFiles` into `Vec::new()` (`tree-hugger/lib/src/god_files/analysis.rs:80-85`). In practice, `hug god-files /tmp/definitely-not-present --plain` exits `0` and prints `0` moderate / `0` high files. That is materially different from an empty existing directory: the user supplied a bad scan root, not a valid tree with no candidates.

The spec defines `DIR` as the directory to scan (`spec.md:229-233`) and separately defines the successful empty/no-candidate case (`spec.md:316-317`). Treating missing or unreadable roots as success hides invocation errors and can mask CI/configuration mistakes.

Suggested fix: make the scan-root validity observable before constructing `GodFiles` in the CLI, or change the library API to expose fallible analysis. If the `&Vec` cache API is intentionally kept, the CLI can cheaply reject `!dir.exists()` / `!dir.is_dir()` up front and add an integration test for non-existent and file-valued `DIR`.

Strongest verification present: no negative-path test found.

### Medium: Required lazy/cache and performance-smoke tests are missing

The spec calls out lazy/cache behavior and a performance smoke that candidate screening does not parse files (`spec.md:333-339`). The implementation has good Level 1 unit coverage for candidate thresholds, SLOC banding, block ranking, refactor hints, and unparseable fallback, but I did not find tests asserting:

- `analysis()` populates `candidates()` implicitly.
- repeated `candidates()` and `analysis()` calls return the same cached reference.
- candidate screening avoids `TreeFile` parsing beyond the current invalid-syntax inclusion proxy.

The existing `candidate_discovery_no_parse_required` test (`tree-hugger/lib/src/god_files/analysis.rs:848-861`) proves syntactically invalid files can be discovered, but it does not prove the phase avoids parsing in general or guard against a future implementation that parses and tolerates errors. Add direct cache-reference assertions and either an instrumentation hook or a narrow regression fixture that would fail if discovery invoked the parser.

Strongest verification present: partial **Level 1**.

## Verification Level Matrix

| Requirement | Strongest current test | Appropriate level | Status |
| --- | --- | --- | --- |
| Candidate discovery, physical-line threshold, deterministic sorting | Level 1 unit tests (`analysis.rs`) | Level 1 | OK |
| Effective SLOC re-filter, 399/400/999/1000 banding and demotion | Level 1 unit tests (`analysis.rs`) | Level 1 | OK |
| Block ranking, floor/cap/truncation, effective block SLOC | Level 1 unit tests (`analysis.rs`) | Level 1 | OK |
| Refactor hint synthesis | Level 1 unit tests (`analysis.rs`) | Level 1 | OK |
| CLI JSON/plain shape, default CWD, empty scan, `--high-risk`, unparseable note | Level 1 assert_cmd tests (`cli.rs`) | Level 1 | OK |
| Pretty report styling, OSC8 hyperlinks, glyph/indent rendering through a real terminal | Level 1 assert_cmd/raw byte tests only | Level 2 | Gap |
| Keyboard, mouse, paste, IME, hotkeys | Not applicable | Not applicable | OK |

## Notes

The prior review's main issues appear addressed: `--high-risk` now suppresses the moderate section heading and body while retaining counts, empty scans print both count lines, OSC8 targets are based on the scanned file reference, and unparseable candidates carry a note in plain output and JSON.

I also manually probed a generated Python class with 11 methods and confirmed the public JSON carries `many_members.member_count: 11`, so the container call-out path is functioning for that representative case.

## Commands Run

```bash
cargo test -p tree-hugger god_files --color=never
cargo test -p tree-hugger-cli god_files --color=never
./target/debug/hug god-files /tmp/definitely-not-a-tree-hugger-dir-$$ --plain
```

Both targeted test runs passed. The missing-directory probe exited `0` and printed an empty report, which informed the medium finding above.
