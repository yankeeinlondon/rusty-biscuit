---
ready: false
agent: codex
model: ""
---

# Review 5: God Files

## Findings

### High: Perl comments are counted as code, producing false god-file reports

The Perl comments query is empty and only contains a comment claiming that line-based parsing is used (`lib/queries/perl/comments.scm:1-3`). No such fallback exists: `TreeFile::comment_ranges()` returns an empty vector for an empty query (`lib/src/file/tree_file.rs:110-129`), and god-file analysis then treats every nonblank line as effective code (`lib/src/god_files/analysis.rs:130-148`, `296-343`).

This breaks the authoritative SLOC metric for one of the 16 supported programming languages. A public-boundary reproduction with a `.pl` file containing 450 `# comment` lines produces a moderate-risk analysis with `effective_sloc: 450`, `comments: 0%`, and no diagnostic note. It should be dropped because its effective SLOC is zero. Perl TODO/FIXME counts, comment density, block SLOC, risk demotion, and sub-400 re-filtering are consequently wrong as well.

The specification explicitly requires SLOC tests per language (`spec.md:331-333`), but the god-files SLOC tests exercise Python only (`lib/src/god_files/analysis.rs:1097-1139`). Add a real Perl comment-range fallback or language-aware line scanner, then add table-driven Level 1 classification tests for comment-only and mixed code/comment lines across all supported languages.

Strongest verification present: **none for Perl SLOC behavior**. Required verification: **Level 1**.

### Medium: Symbol blocks exclude declarations from SLOC and displayed ranges

`record_span_bytes()` prefers `body_span`, then `name_span`, and uses the full `declaration_span` only as a last resort (`lib/src/god_files/analysis.rs:579-610`). As a result, block SLOC and line ranges describe only the body for ordinary functions and classes, not the complete symbol block required by the specification. Symbols without a recognized body can collapse to their name token even when their declaration spans many lines.

This changes floor filtering and ranking, not just presentation. A 400-SLOC Python file containing `def edge():` plus 14 effective body lines yields no `edge` block in JSON: the implementation measures 14 body lines and drops it below `MIN_BLOCK_SLOC`, although the complete symbol is exactly 15 SLOC and should be included. Reported start lines similarly begin at the body rather than the declaration.

Use `declaration_span` for block SLOC and displayed line ranges. If body spans are still useful for ownership calculations, keep that as a separate helper rather than sharing one span policy. Add Level 1 boundary tests where a declaration line is what crosses `MIN_BLOCK_SLOC`, plus a multiline declaration without a recognized body.

## Verification Levels

| Requirement | Strongest test | Appropriate level | Status |
| --- | --- | --- | --- |
| Candidate discovery, caching, thresholds, sorting, hints, and Python SLOC | Level 1 unit | Level 1 | Covered |
| Effective SLOC for all 16 supported languages | Level 1 for Python only | Level 1 per language | Gap; Perl is broken |
| Block ranking, floor, truncation, and member callouts | Level 1 unit | Level 1 | Covered, but declaration-span boundary is missing and broken |
| Plain/JSON grouping, empty scans, degraded parsing, hyperlinks, and `--high-risk` | Level 1 CLI integration | Level 1 | Covered |
| SGR styling, Unicode glyphs, and OSC8 links through a real terminal | Level 2 tmux and WezTerm | Level 2 | Covered; Kitty unavailable and skip-clean |
| Keyboard, mouse, paste, IME, or hotkeys | Not applicable | Not applicable | OK |

## Verification

- `cargo test -p tree-hugger --color=never`: passed.
- `cargo test -p tree-hugger-cli --color=never`: passed.
- `cargo clippy -p tree-hugger -p tree-hugger-cli --all-targets --color=never -- -D warnings`: passed.
- `just test-l2`: passed through the broker-backed recipe; tmux and WezTerm were exercised, while Kitty was unavailable and skip-clean.
- Manual public CLI reproduction confirmed the Perl false positive and the 15-SLOC symbol-floor failure described above.

The Perl classification defect affects the command's primary risk decision, so this feature is not ready for production.
