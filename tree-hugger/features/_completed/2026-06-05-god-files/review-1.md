---
ready: false
agent: codex
model: ""
---

# Review: God Files

## Findings

### High: `--high-risk` output contradicts the specified filtering contract

The spec says `--high-risk` suppresses the moderate section and its heading line, while the report heading still reports both band counts (`spec.md:232-235`). The implementation always prints the moderate section heading and only suppresses the moderate file bodies (`cli/src/main.rs:3066-3077`). The integration test locks in this wrong behavior by asserting `Moderate Risk (1)` is present under `--high-risk` (`cli/tests/cli.rs:1620-1629`).

This is user-facing behavior and currently has only Level 1 CLI coverage. Level 1 is sufficient for byte-level CLI filtering here, but the covered expectation is the inverse of the spec. Fix the renderer and update the test to assert that the moderate heading is absent while the report heading still carries both counts.

### High: The report heading does not report moderate/high counts, and empty-band handling diverges

The specified heading is two count lines: one moderate count and one high-risk count (`spec.md:249-252`, `spec.md:285-287`). The implementation prints `God Files Report` plus `N files analyzed` (`cli/src/main.rs:3043-3048`). For empty scans it returns before printing any moderate/high counts (`cli/src/main.rs:3050-3051`), while the spec requires `0` / `0` to be visible and both sections omitted (`spec.md:319-324`).

This is a functional CLI rendering gap. The current Level 1 tests only check for `0 files analyzed` and section omission (`cli/tests/cli.rs:1634-1647`), so they do not verify the required count contract.

### High: OSC8 links are built from `relative_path`, so explicit `DIR` scans link to the wrong file

`render_god_analysis` builds the hyperlink from `analysis.relative_path` (`cli/src/main.rs:3081-3085`). `hyperlink()` resolves relative paths against the process current directory, not the scan root (`cli/src/main.rs:749-759`). When a user runs `hug god-files /tmp/project` from another directory, the visible label can be `big.py`, but the OSC8 target becomes `$CWD/big.py` instead of `/tmp/project/big.py`.

The spec calls out `GodAnalysis.file` as the resolvable reference that drives the OSC8 hyperlink (`types.rs:85-88`, `spec.md:264-268`). The CLI ignores that field. The existing CLI tests cover default-CWD behavior only (`cli/tests/cli.rs:1555-1600`) and never exercise an explicit scan directory or hyperlink target. Because this is terminal hyperlink behavior, add at least Level 1 output-byte coverage for the OSC8 URL and consider Level 2 capture if the feature is meant to guarantee real-terminal hyperlink rendering.

### High: Unparseable candidates are emitted without the required diagnostic note

The spec requires unparseable candidates to fall back to physical-line banding, emit the file with empty analysis signals, and include a diagnostic note (`spec.md:319-320`). The implementation falls back to `physical_lines` and empty fields (`analysis.rs:175-178`), but `GodAnalysis` has no diagnostic/status field (`types.rs:83-120`) and the renderer has no place to display one.

The current unit test only verifies that an invalid UTF-8 file is retained and banded (`analysis.rs` god-files tests), so the user-visible failure context is untested. Add a structured diagnostic/note field and assert it in both library JSON and plain CLI output.

### Medium: Symbol-block SLOC is not effective SLOC

File-level effective SLOC uses Tree-sitter comment ranges, but symbol-block SLOC recomputes metrics over the source snippet with `tree_file: None` (`analysis.rs:459-462`). That means comment-only lines inside functions/classes are counted as code for block ranking, floor filtering, truncation, member summaries, and dominance hints. The spec says block SLOC is effective lines within the block span (`spec.md:139-144`), with comment-only lines excluded (`spec.md:22-24`).

This has Level 1 tests for ordering, cap, and floor, but not for comment-heavy symbol spans. Add a regression where a large documented block has fewer than `MIN_BLOCK_SLOC` effective lines or should rank below a code-heavy block.

### Medium: Rendering bypasses the specified `Prose` path and is lightly asserted

The spec explicitly chooses biscuit-terminal `Prose` for capability-aware styling and OSC8 hyperlinks (`spec.md:244-268`). The implementation uses `println!`, `owo_colors`, and a raw OSC8 helper. The current tests assert a few substrings and that `--plain` has no escape byte, but they do not verify the specified heading prose, file-list sentence shape, dimmed hint text, block/member formatting, hyperlink target, or empty-band behavior.

At minimum, add Level 1 snapshot-style tests for plain and styled bytes. If the final requirement is real-terminal styling/hyperlink behavior, Level 2 coverage is the appropriate verification level for rendered SGR/OSC8 behavior.

## Verification Level Assessment

- Candidate discovery, banding, re-filtering, lazy cache, block ranking, hints: strongest coverage is Level 1 unit tests. That is appropriate for in-process library behavior, but comment-heavy block SLOC cases are missing.
- CLI JSON/plain shape, default CWD, empty scan, `--high-risk`: strongest coverage is Level 1 assert_cmd tests. That is appropriate for basic CLI byte output, but several assertions do not match the spec.
- Styled report, OSC8 hyperlinks, and capability-aware rendering: strongest coverage is Level 1 substring/escape checks. This is not enough if the feature claims real-terminal rendering semantics; add Level 2 terminal capture for SGR/OSC8 display behavior.

## Tests Run

- `cargo test -p tree-hugger-lib god_files --color=never` failed because the package ID does not exist; Cargo reports the library package is `tree-hugger`.
- `cargo test -p tree-hugger god_files --color=never` passed: 32 god-files library tests.
- `cargo test -p tree-hugger-cli god_files --color=never` passed: 6 CLI integration tests.

## Production Readiness

Not ready for production. The core library path is substantially present, but the CLI report does not meet the specified output contract, explicit-directory hyperlinks are wrong, unparseable files lack the required diagnostic note, and coverage currently preserves at least one spec-incompatible behavior.
