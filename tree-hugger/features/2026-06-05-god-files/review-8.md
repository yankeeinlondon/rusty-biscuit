---
ready: false
agent: codex
model: ""
---

# Review 8: God Files

## Findings

### High: Local symbols are reported as top-level and produce false refactor guidance

The specification defines `top_level_symbol_count` and `kind_histogram` from symbols with no container, then uses that count for `ManyUnrelatedTopLevel` guidance (`spec.md`, sections 4.3-4.4). The implementation equates `identity.module_path.is_none()` with top-level ownership (`lib/src/god_files/analysis.rs:138-143`), but parse records do not consistently populate that field for local variables or parameters.

A public CLI reproduction containing nine top-level Python functions, each with one parameter and 44 local assignments, has 405 effective SLOC. It should report nine top-level symbols. Instead, JSON reports:

```text
top_level_symbol_count: 414
kind_histogram: Function: 9, Variable: 396, Parameter: 9
many_unrelated_top_level.count: 414
```

This makes the structural summary and a user-facing refactor hint wrong on ordinary function-heavy files. Determine top-level ownership from actual declaration containment or resolved parent relations, not only `module_path`, and add Level 1 public-boundary regressions proving locals and parameters are excluded while genuine file-scope variables remain included.

Strongest verification present: **Level 1 tests check histogram ordering, not top-level accuracy**. Required verification: **Level 1 semantic and CLI/JSON regression tests**.

### High: `max_nesting_depth` measures nested symbols, not syntax block nesting

The specification requires the deepest block nesting from a single tree walk and derives `DeeplyNested` guidance from it (`spec.md`, section 4.4). `compute_max_nesting_depth()` instead sweeps extracted symbol declaration spans (`lib/src/god_files/analysis.rs:722-768`). Control-flow blocks such as `if`, loops, matches, and try/catch constructs are not symbols, so they are invisible to this calculation.

A 420-SLOC Python file with one function containing eight nested `if` blocks reports `max_nesting_depth: 2` and emits no `deeply_nested` hint. The reported depth comes from symbol containment, including the function parameter, rather than the eight control-flow levels the signal is intended to expose.

The review-7 performance regression is fixed: the span sweep is now near-linear. However, it efficiently computes the wrong metric. Traverse the Tree-sitter syntax tree and count language-relevant block/control-flow nodes, then add table-driven Level 1 tests for deep control flow and flat control flow. Keep the 50,000-symbol regression only if symbol-span processing remains part of the final design.

Strongest verification present: **Level 1 tests cover nested functions and synthetic symbol-span scaling only**. Required verification: **Level 1 syntax-tree nesting tests plus a public JSON assertion**.

## Verification Levels

| Requirement | Strongest test | Appropriate level | Status |
| --- | --- | --- | --- |
| Candidate discovery, caching, physical screening, thresholds, sorting, and fallback | Level 1 unit/integration | Level 1 | Covered |
| Effective SLOC and comment forms across all 16 languages, including Perl POD | Level 1 unit/query compilation | Level 1 | Covered |
| Block ranking, declaration spans, floor, truncation, member callouts, and most hints | Level 1 unit | Level 1 | Covered |
| Top-level symbol count, histogram, and `ManyUnrelatedTopLevel` hint | Ordering-only Level 1 test; public reproduction fails | Level 1 | Gap; implementation is incorrect |
| Syntax block nesting and `DeeplyNested` hint | Symbol-containment Level 1 tests only; public reproduction fails | Level 1 | Gap; wrong metric |
| Analysis scaling by symbol count | Level 1 bounded 50,000-symbol regression | Level 1 | Covered |
| Plain/JSON output, empty scans, hyperlinks, degraded parsing, and `--high-risk` | Level 1 CLI integration | Level 1 | Covered |
| SGR styling, Unicode glyphs, and OSC8 links through a real terminal | Level 2 tmux, WezTerm, and Kitty | Level 2 | Covered |
| Keyboard, mouse, paste, IME, or hotkeys | Not applicable | Not applicable | OK |

## Verification

- `cargo test -p tree-hugger --color=never god_files -- --nocapture`: passed, 48 god-files tests.
- `cargo test -p tree-hugger-cli --test cli --color=never god_files -- --nocapture`: passed, 11 CLI tests.
- `just test-l2`: passed in tmux, WezTerm, and Kitty.
- `cargo clippy -p tree-hugger -p tree-hugger-cli --all-targets --color=never -- -D warnings`: passed.
- Public JSON reproductions confirmed the incorrect top-level count/hint and block-nesting depth described above.

The report rendering and previous review findings are addressed, but two required analysis signals still produce materially misleading output. This feature is not ready for production.
