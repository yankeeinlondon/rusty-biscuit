---
ready: true
agent: open_code
model: ""
---

# Review 11: God Files

## Summary

The iteration-10 finding (Zsh `select` loops invisible to nesting analysis) is
fully resolved. `select_statement` is now classified in the Zsh control-flow
set (`lib/src/god_files/analysis.rs:926`), exercised by a direct table case
(`analysis.rs:2123-2128`), and covered by a public CLI JSON regression with
eight nested `select` loops (`cli/tests/cli.rs:1742-1752`). No remaining
implementation gaps, broken functionality, or test-coverage mismatches were
found.

## Findings

### Low: Comment coverage vector rebuilt three times per candidate

`build_comment_coverage` allocates a `Vec<bool>` sized to the file's byte
length and is invoked independently inside `compute_sloc_metrics`
(`analysis.rs:305`), `compute_symbol_blocks` (`analysis.rs:529`), and
`count_comment_lines` (`analysis.rs:474`). For the large files this tool
targets, three independent multi-hundred-KB allocations per candidate is
wasteful. Passing the pre-built coverage vector through from a single call site
in `analyze_single_file` would halve the allocation count. This is a
performance observation, not a correctness issue — the tool already only
processes the candidate subset.

## Verification

- `cargo test -p tree-hugger --color=never god_files`: **51 passed**, 0 failed.
- `cargo test -p tree-hugger-cli --test cli --color=never god_files`: **14 passed**, 0 failed.
- `just test-l2`: **3 passed** (tmux, WezTerm, Kitty), 0 failed.
- `cargo clippy -p tree-hugger -p tree-hugger-cli --all-targets --color=never -- -D warnings`: clean.
- `git diff --check`: clean.

## Verification Levels

| Requirement | Strongest test | Appropriate level | Status |
| --- | --- | --- | --- |
| Candidate discovery, caching, physical screening, thresholds, sorting, fallback | Level 1 unit/integration | Level 1 | Covered |
| Effective SLOC and comment forms across all 16 languages, including Perl POD | Level 1 unit/query compilation | Level 1 | Covered |
| Block ranking, declaration spans, floor, truncation, member callouts, hints | Level 1 unit | Level 1 | Covered |
| Top-level count, histogram, and `ManyUnrelatedTopLevel` | Level 1 unit plus public JSON regression | Level 1 | Covered |
| Syntax nesting and `DeeplyNested` across all supported languages | Level 1 table (30+ cases) plus public JSON regressions (Perl, C++, Java, Swift, Zsh) | Level 1 | Covered |
| Analysis scaling by symbol count (near-linear enclosure) | Level 1 bounded 50,000-symbol regression | Level 1 | Covered |
| Plain/JSON output, empty scans, hyperlinks, degraded parsing, `--high-risk`, default CWD, error paths | Level 1 CLI integration | Level 1 | Covered |
| SGR styling, Unicode glyphs, and OSC8 links through a real terminal | Level 2 tmux, WezTerm, and Kitty | Level 2 | Covered |
| Keyboard, mouse, paste, IME, or hotkeys | Not applicable | Not applicable | OK |
