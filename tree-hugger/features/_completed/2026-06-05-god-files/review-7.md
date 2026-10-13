---
ready: false
agent: codex
model: ""
---

# Review 7: God Files

## Findings

### High: Nesting-depth analysis is quadratic in the number of symbols

The specification requires maximum nesting depth to come from a single tree walk and presents this feature as a high-performance scanner for unusually large files (`spec.md`, sections 4.4 and 6). Instead, `compute_max_nesting_depth()` compares each symbol span with every preceding span (`lib/src/god_files/analysis.rs:722-759`), making analysis `O(symbols^2)` even when every symbol is flat and the answer is immediately known to be 1.

This becomes a user-visible stall on exactly the files this command is intended to inspect. A public CLI reproduction using one 40,000-line Python file containing 20,000 flat two-line functions did not complete within 60 seconds and had to be terminated. Parsing and the other signals are not sufficient justification for that cost: the containment loop alone performs roughly 200 million span comparisons.

Replace the all-pairs dynamic program with a stack-based containment sweep over spans sorted by start ascending and end descending, or compute depth during a Tree-sitter traversal. Add a performance regression that uses a many-symbol flat file and asserts work scales near-linearly; a Criterion benchmark is appropriate for tracking the slope, with a smaller bounded test protecting normal CI.

Strongest verification present: **none for analysis scaling by symbol count**. Required verification: **Level 1 performance regression**.

## Verification Levels

| Requirement | Strongest test | Appropriate level | Status |
| --- | --- | --- | --- |
| Candidate discovery, caching, physical screening, thresholds, sorting, and fallback | Level 1 unit/integration | Level 1 | Covered |
| Effective SLOC and comment forms across all 16 languages, including Perl POD | Level 1 unit/query compilation | Level 1 | Covered |
| Block ranking, declaration spans, floor, truncation, member callouts, and hints | Level 1 unit | Level 1 | Covered |
| Analysis remains performant on large symbol counts | None; 20,000-symbol CLI reproduction exceeded 60 seconds | Level 1 performance regression | Gap; implementation is quadratic |
| Plain/JSON output, empty scans, hyperlinks, degraded parsing, and `--high-risk` | Level 1 CLI integration | Level 1 | Covered |
| SGR styling, Unicode glyphs, and OSC8 links through a real terminal | Level 2 tmux, WezTerm, and Kitty | Level 2 | Covered |
| Keyboard, mouse, paste, IME, or hotkeys | Not applicable | Not applicable | OK |

## Verification

- `cargo test -p tree-hugger --color=never`: passed.
- `cargo test -p tree-hugger-cli --color=never`: passed, including the available tmux, WezTerm, and Kitty Level 2 tests.
- `cargo clippy -p tree-hugger -p tree-hugger-cli --all-targets --color=never -- -D warnings`: passed.
- A 20,000-symbol flat-file CLI reproduction exceeded 60 seconds and was terminated.

The quadratic analysis path can make the command effectively unusable on large, symbol-dense god files, so this feature is not ready for production.
