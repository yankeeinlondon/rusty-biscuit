---
ready: true
agent: codex
model: ""
---

# Review 10: God Files

## Findings

### High: Zsh `select` loops are still invisible to nesting analysis

The per-language classifier still omits Zsh's named `select_statement` node
(`lib/src/god_files/analysis.rs:917-928`). `select` is a looping control-flow
construct in the pinned `tree-sitter-zsh` grammar, so it must contribute to the
same complexity signal as `for`, `while`, and `repeat`.

A public `hug god-files --json` reproduction containing eight nested Zsh
`select` loops reports `max_nesting_depth: 0` and emits no `deeply_nested` hint.
This violates the specification's syntax-tree nesting signal and makes the
result depend on which supported shell construct expresses the loop.

The new table-driven test claims to exercise every supported language's
grammar-specific forms, but its Zsh cases cover only `for`/`if`/`while` and
`repeat`/`case` (`lib/src/god_files/analysis.rs:2110-2121`). The public
cross-language regression covers only Perl, C++, Java, and Swift
(`cli/tests/cli.rs:1671-1770`). Add `select_statement` to the Zsh classifier and
add both a direct table case and a public JSON regression with nested `select`
loops. The classifier comment saying the sets are derived from the pinned
`node-types.json` is currently stronger than the implementation and should
remain only once the inventory is complete.

Strongest verification present: **Level 1 unit tests for selected Zsh
constructs, with no `select` case**. Appropriate verification: **Level 1
semantic and public JSON tests for every supported grammar-specific
control-flow form**. This is a confirmed implementation failure and coverage
mismatch.

## Verification Levels

| Requirement | Strongest test | Appropriate level | Status |
| --- | --- | --- | --- |
| Candidate discovery, caching, physical screening, thresholds, sorting, and fallback | Level 1 unit/integration | Level 1 | Covered |
| Effective SLOC and comment forms across all 16 languages, including Perl POD | Level 1 unit/query compilation | Level 1 | Covered |
| Block ranking, declaration spans, floor, truncation, member callouts, and hints | Level 1 unit | Level 1 | Covered |
| Top-level count, histogram, and `ManyUnrelatedTopLevel` | Level 1 unit plus public JSON regression | Level 1 | Covered |
| Syntax nesting and `DeeplyNested` across all supported languages | Level 1 table plus selected public JSON regressions; nested Zsh `select` fails | Level 1 | Gap; implementation is incorrect |
| Analysis scaling by symbol count | Level 1 bounded 50,000-symbol regression | Level 1 | Covered |
| Plain/JSON output, empty scans, hyperlinks, degraded parsing, and `--high-risk` | Level 1 CLI integration | Level 1 | Covered |
| SGR styling, Unicode glyphs, and OSC8 links through a real terminal | Level 2 tmux, WezTerm, and Kitty | Level 2 | Covered |
| Keyboard, mouse, paste, IME, or hotkeys | Not applicable | Not applicable | OK |

## Verification

- `cargo test -p tree-hugger --color=never god_files -- --nocapture`: passed,
  51 god-files tests.
- `cargo test -p tree-hugger-cli --test cli --color=never god_files -- --nocapture`:
  passed, 14 CLI tests.
- `just test-l2`: passed all three tmux, WezTerm, and Kitty tests.
- `cargo clippy -p tree-hugger -p tree-hugger-cli --all-targets --color=never -- -D warnings`:
  passed.
- `git diff --check`: passed.
- Public JSON reproduction confirmed depth `0` and no `deeply_nested` hint for
  eight nested Zsh `select` loops.

The iteration-9 failures for Perl `unless`, C++ range-for, Java
try-with-resources, and Swift repeat-while are fixed. The remaining supported
Zsh control-flow failure means the feature is not ready for production.
