---
ready: false
agent: codex
model: ""
---

# Review: Darkmatter Code-Block Rendering Failures

## Findings

### High: The invariant sweep can pass when a matrix shape fails to render

`darkmatter/lib/tests/render_invariants.rs:247` converts any `DarkmatterPage::render` error into a plain string:

```rust
page.render(&md)
    .unwrap_or_else(|e| format!("<render error: {e}>"))
```

That makes the empty `KNOWN_VIOLATIONS` ledger unreliable. A shape/scenario that regresses from valid output to a render error can still satisfy I1/I2a/I5/I5b as the literal text `"<render error: ...>"`, and all code-panel checks are skipped for non-code shapes. This conflicts with the spec's Stage 4 success criterion that every `shapes() x scenarios()` cell satisfies the render contract. Render failure should be a first-class violation, ideally keyed as something like `(shape, scenario, "render")`, or the helper should return `Result<String, String>` and the sweep should record/panic on errors before running layout predicates.

### High: The shared layout invariants count Rust chars, not terminal cells

`biscuit-test-harness/src/layout_invariants.rs:58` defines visible width as `strip_ansi(line).chars().count()`, and `bg_extent` increments the visible column by one per `char` at `layout_invariants.rs:113-145`. The spec defines these invariants in terminal cells: "No rendered line's visible width exceeds the physical terminal width" and Level 2 is explicitly about glyph widths. Character count is not cell width for wide Unicode, combining marks, emoji, and some box/rule glyphs. That can let I1 pass while the real terminal wraps, and can miscompute I2/I3/I4 background extents for any background-bearing line containing non-ASCII source code or chrome.

Use the same terminal-width primitive already used elsewhere in the renderer, e.g. `biscuit_terminal::utils::block_constraint::visible_width`, or a `unicode-width`-backed helper that strips ANSI and counts display cells. Add focused tests with full-width and combining characters so this harness remains reusable by `biscuit-terminal`, not just the current ASCII fixtures.

### Medium: The darkmatter terminal skill still carries a contradictory "terminal only" heading

The implementation and docs now correctly state that HTML also inverts code themes, but `.claude/skills/darkmatter/terminal.md:57` still says `Code blocks invert for page contrast (terminal only)`. The section later says "HTML inverts too", so the body is mostly correct, but the heading preserves the exact misunderstanding Stage 4 was meant to eliminate. Update the heading to "terminal and HTML" or split the HTML note into its own subsection.

## Test Rigor Classification

- Ordinary terminal code fences: Level 1 invariant coverage exists, and Level 2 WezTerm page-path tests now exist for contrast, contiguous background rectangle, right boundary, and blank-line rhythm. I could not run them here.
- Mermaid text and image fallback: deterministic Level 1 coverage now exists; `TerminalImageMode::Never` forces the fallback path, closing the conditional-skip issue from review 2.
- HTML `as_html`, render-tree HTML, and `YamlBlock` browser path: in-process HTML/style assertions exist; acceptable for this non-terminal requirement.
- Matrix-wide layout correctness: strongest routine coverage is Level 1, but the current harness can miss render errors and uses the wrong width unit for terminal-cell invariants.
- No Level 3 requirements found; the spec does not assert OS keyboard-event behavior.

## Verification

Attempted:

```bash
cargo test -p darkmatter --test render_invariants --color=never
```

The command was still compiling after roughly 60 seconds in this non-interactive session, so I stopped it per the session constraints. No green test result was obtained during this review.

## Production Readiness

Not ready for production. The previously identified implementation paths appear substantially improved, but the main correctness harness can still report a clean ledger for broken render cells and does not measure terminal cell width correctly.
