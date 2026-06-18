---
ready: false
agent: codex
model: ""
---

# Review 9: God Files

## Findings

### High: Control-flow nesting is still incorrect for multiple supported languages

The new syntax-tree walk fixes Python, Rust, and JavaScript, but the node-kind
allowlist is not complete across the 16 supported grammars
(`lib/src/god_files/analysis.rs:794-855`). The implementation claims to cover
the supported grammars, while its tests exercise only those three languages
(`lib/src/god_files/analysis.rs:1838-1884`).

Public `hug god-files --json` reproductions with eight nested constructs report
`max_nesting_depth: 0` and no `deeply_nested` hint for:

- Perl `unless_statement`
- C++ `for_range_loop`
- Java `try_with_resources_statement`
- Swift `repeat_while_statement`

These are ordinary control-flow constructs present in the pinned grammars but
absent from `is_control_flow_nesting()`. Other unrecognized grammar-specific
forms include Perl's simple/postfix forms and numbered `for_statement_*` nodes.
Consequently, the structured API and rendered signal can materially understate
complexity depending only on source language, violating the specification's
single-tree-walk nesting requirement (`spec.md:191-205`) and the documented
field contract (`lib/src/god_files/types.rs:109-112`).

Make nesting classification language-aware or derive an exhaustive per-grammar
mapping from the pinned node types. Add table-driven Level 1 tests for every
supported language, including each grammar-specific loop, conditional, match,
and exception form that the metric intends to count. Add public JSON
regressions for representative non-Python forms so the CLI boundary cannot
silently regress.

Strongest verification present: **Level 1 unit and CLI tests for Python, Rust,
and JavaScript only**. Appropriate verification: **Level 1 semantic tests for
all supported grammars**. This is a coverage mismatch and a confirmed
implementation failure.

## Verification Levels

| Requirement | Strongest test | Appropriate level | Status |
| --- | --- | --- | --- |
| Candidate discovery, caching, physical screening, thresholds, sorting, and fallback | Level 1 unit/integration | Level 1 | Covered |
| Effective SLOC and comment forms across all 16 languages, including Perl POD | Level 1 unit/query compilation | Level 1 | Covered |
| Block ranking, declaration spans, floor, truncation, member callouts, and hints | Level 1 unit | Level 1 | Covered |
| Top-level count, histogram, and `ManyUnrelatedTopLevel` | Level 1 unit plus public JSON regression | Level 1 | Covered |
| Syntax nesting and `DeeplyNested` across all supported languages | Level 1 for Python, Rust, and JavaScript; public reproductions fail in four other languages | Level 1 | Gap; implementation is incorrect |
| Analysis scaling by symbol count | Level 1 bounded 50,000-symbol regression | Level 1 | Covered |
| Plain/JSON output, empty scans, hyperlinks, degraded parsing, and `--high-risk` | Level 1 CLI integration | Level 1 | Covered |
| SGR styling, Unicode glyphs, and OSC8 links through a real terminal | Level 2 tmux, WezTerm, and Kitty | Level 2 | Covered |
| Keyboard, mouse, paste, IME, or hotkeys | Not applicable | Not applicable | OK |

## Verification

- `cargo test -p tree-hugger --color=never god_files -- --nocapture`: passed,
  51 god-files tests.
- `cargo test -p tree-hugger-cli --test cli --color=never god_files -- --nocapture`:
  passed, 13 CLI tests.
- `just test-l2`: passed all three tmux, WezTerm, and Kitty tests.
- `cargo clippy -p tree-hugger -p tree-hugger-cli --all-targets --color=never -- -D warnings`:
  passed.
- `git diff --check`: passed.
- Public JSON reproductions confirmed depth `0` for eight nested Perl
  `unless`, C++ range-for, Java try-with-resources, and Swift repeat-while
  constructs.

The iteration-8 top-level-symbol fix is effective, and the syntax-tree walk has
the right performance shape. Its cross-language classification is incomplete,
so this feature is not ready for production.
