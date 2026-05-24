---
ready: false
agent: codex
model: ""
---

# Review 13

## Findings

### High: rich code-block terminal rendering is still only verified at Level 1

The spec now treats browser/terminal code highlighting, info-string titles, line numbers, and highlighted lines as no longer being an accepted gap (`spec.md`, Supporting Design Coverage). The implementation wires `TerminalCodeRenderer` into the terminal entry point (`darkmatter/lib/src/markdown/render_tree/entrypoints.rs:189`) and has Level 1 checks that the terminal string contains ANSI color SGRs and the title (`entrypoints.rs:714`, `render_tree_parity.rs:950`).

That is not enough for this user-observable terminal behavior. The Level 2 suite is the right mechanism for real terminal rendering, but it currently covers headings, ordinary inline styles, tables, mark, dim, and HR attributes only (`darkmatter/lib/tests/level2_render_tree_terminal.rs:201`, `:224`, `:256`, `:284`, `:320`, `:356`). There is no Level 2 fixture that renders a rich fenced block through the wired code renderer and captures the pane to verify the title/header row, highlighted body, line-number layout, or highlighted-line styling survive a real terminal.

Per the review rubric, terminal styling/layout requirements need Level 2 capture, not just in-process string assertions. This should remain a production blocker for declaring the feature ready.

Recommendation: add a Level 2 WezTerm test for a fixture like:

~~~markdown
```rust title="Demo Snippet" line-numbering=true highlight=2
fn parity_demo() {
    println!("render tree");
}
```
~~~

Drive it through the same tree terminal options with `TerminalCodeRenderer`, then assert the captured pane includes the title, body text, line-number gutter evidence, and ANSI/styling evidence for the highlighted line. Keep it under `just test-l2` so `DARKMATTER_LEVEL2_REQUIRED=1` enforces the requirement.

## Verification-Level Summary

| Requirement | Strongest verification found | Assessment |
| --- | --- | --- |
| Mark/dim fold shape and byte ranges | Level 1 unit tests in `span.rs` / `fold.rs` | Appropriate for tree shape and source-range policy. |
| Mark/dim terminal styling | Level 2 WezTerm tests | Appropriate when run via `just test-l2`. |
| HR attribute terminal styling | Level 2 WezTerm test | Appropriate when run via `just test-l2`. |
| Rich code-block terminal styling/layout | Level 1 in-process terminal string tests | Gap; needs Level 2 real-terminal capture. |
| Raw HTML safe HTML default | Level 1 entry-point tests | Appropriate for escaped HTML string policy. |
| Parser-option divergences | Level 1 structural/output tests | Appropriate for parser/fold behavior. |
| Fold/render diagnostic separation in parity | Level 1 parity helper assertions | Prior review gap appears closed. |

## Notes

The requested `root` skill is not present in the local skill catalog for this session, so I used the repo-level instructions and the required `renderable` skill instead.

I did not run the Rust test suite in this review pass; the finding above is from static inspection of the implementation and test coverage.
