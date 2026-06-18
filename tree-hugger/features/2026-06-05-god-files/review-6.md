---
ready: false
agent: codex
model: ""
---

# Review 6: God Files

## Findings

### High: Perl POD documentation is counted as executable SLOC

The Perl comment query now captures hash comments, but it only captures the grammar's `comments` node (`lib/queries/perl/comments.scm:1-4`). Perl POD is parsed separately as `pod_statement`, so POD-only lines never enter `comment_ranges()` and are treated as code by `compute_sloc_metrics()` (`lib/src/god_files/analysis.rs:296-343`).

A public CLI reproduction with a 450-line `.pl` file containing only `=pod`, documentation text, and `=cut` emits `effective_sloc: 450`, `risk: Moderate`, and `comment_density: 0.0`. Perl ignores POD as documentation; this file should have zero effective SLOC and be dropped. This also corrupts risk demotion, block SLOC, comment density, and TODO/FIXME detection inside POD.

The new all-language test uses two hash-comment lines for Perl (`lib/src/god_files/analysis.rs:1687-1728`), so it verifies one comment syntax per language rather than Perl's full comment/documentation behavior. Extend the Perl query or add a language-specific range source for `pod_statement`, then add Level 1 tests for POD-only files, mixed POD/code files, and markers inside POD.

Strongest verification present: **none for Perl POD SLOC behavior**. Required verification: **Level 1**.

### Medium: `todo_fixme_count` counts comment nodes, not debt markers

`count_todo_fixme()` increments once for a comment range as soon as any supported marker is found, then breaks (`lib/src/god_files/analysis.rs:502-516`). A single block comment containing `TODO`, `FIXME`, `HACK`, and `XXX` therefore reports `todo_fixme_count: 1` instead of `4`. Repeated markers in one line or block are also collapsed.

The field and report describe a count of `TODO|FIXME|HACK|XXX` markers, not a count of comment nodes containing at least one marker. Count case-insensitive marker occurrences across comment text and add Level 1 tests for multiple distinct and repeated markers within one block comment. The current test places each marker in a separate Python comment node, so it cannot detect this behavior (`lib/src/god_files/analysis.rs:1473-1483`).

## Verification Levels

| Requirement | Strongest test | Appropriate level | Status |
| --- | --- | --- | --- |
| Candidate discovery, caching, physical-line screening, thresholds, sorting, hints | Level 1 unit | Level 1 | Covered |
| Effective SLOC for all supported language comment forms | Level 1 for one representative syntax per language | Level 1 | Gap; Perl POD is broken |
| Block ranking, declaration spans, floor, truncation, and member callouts | Level 1 unit | Level 1 | Covered |
| TODO/FIXME/HACK/XXX signal | Level 1 unit with one marker per comment node | Level 1 | Gap; multiple markers per node are undercounted |
| Plain/JSON grouping, empty scans, degraded parsing, hyperlinks, and `--high-risk` | Level 1 CLI integration | Level 1 | Covered |
| SGR styling, Unicode glyphs, and OSC8 links through a real terminal | Level 2 tmux, WezTerm, and Kitty | Level 2 | Covered |
| Keyboard, mouse, paste, IME, or hotkeys | Not applicable | Not applicable | OK |

## Verification

- `cargo test -p tree-hugger --color=never`: passed.
- `cargo test -p tree-hugger-cli --color=never`: passed.
- `cargo clippy -p tree-hugger -p tree-hugger-cli --all-targets --color=never -- -D warnings`: passed.
- `just test-l2`: passed through the broker-backed recipe; tmux, WezTerm, and Kitty all passed.
- Manual public CLI reproductions confirmed the Perl POD false positive and the marker undercount described above.

The Perl defect changes the command's authoritative SLOC and risk classification, so this feature is not ready for production.
