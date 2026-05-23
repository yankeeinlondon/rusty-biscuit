---
ready: false
agent: codex
model: ""
---

# Review 11

## Findings

### High: `ColorDepth::None` is still broken for tree terminal code blocks

The tree terminal entry point now maps `TerminalOptions::color_depth` into the render-tree terminal context (`darkmatter/lib/src/markdown/render_tree/entrypoints.rs:185`-`197`), but the wired `TerminalCodeRenderer` ignores the context's color depth and creates `TerminalOptions::default()` before calling Darkmatter's highlighted code renderer (`darkmatter/lib/src/markdown/render_tree/code_renderer.rs:63`-`80`). Darkmatter's legacy terminal path treats `ColorDepth::None` as a no-formatting early return (`darkmatter/lib/src/markdown/output/terminal.rs:872`-`877`), so a caller asking for no color can still get ANSI escape sequences from fenced code blocks on the tree path.

Verification level: I found Level 1 tests proving `ColorDepth::None` on the legacy path, and tests proving the tree code renderer emits ANSI when true color is selected, but no test pins `render_tree_terminal(... color_depth: Some(ColorDepth::None))` for a code block. This is user-observable terminal behavior and a production blocker. The renderer should either return `None` when `context.color_depth() == ColorDepth::None` so the plain fallback runs, or construct equivalent no-color options and add a Level 1 assertion that no ANSI is emitted.

### High: Browser/HTML rich code parity is still not implemented or verified

DMTR-5 requires parity coverage for "code blocks with titles, line numbers, highlights, and syntax highlighting" across Browser/HTML, Terminal, Markdown, and MarkdownPlus (`renderable/features/2026-05-20-darkmatter-tree/spec.md:274`-`298`). The new rich-code test only checks HTML body tokens (`darkmatter/lib/tests/render_tree_parity.rs:860`-`869`) and explicitly accepts title, line-numbering, and highlighted-line loss on the tree path (`darkmatter/lib/tests/render_tree_parity.rs:839`-`848`, `888`-`899`). Browser rendering also has no code-renderer hook in `BrowserRenderOptions` (`renderable/src/tree/render/browser.rs:65`-`72`); `NodeKind::Code.meta` is ignored (`renderable/src/tree/render/browser.rs:250`-`254`), and browser code output is always plain `<pre><code>` (`renderable/src/tree/render/browser.rs:587`-`604`).

Verification level: HTML rich-code coverage is Level 1 visible-token preservation only. It does not verify syntax-highlight markup, title rendering, line numbers, or highlighted-line markup. Terminal syntax highlighting is only checked by in-process ANSI presence; there is no Level 2 real-terminal capture for rich code styling. This should stay classified as an experimental adapter gap, not production-ready parity.

### High: HR attributes inside list items still lack the design-required explicit fixture

The span-aware design explicitly calls out `- --- { style: waves }` and says to add an explicit test before implementation because tree behavior must match the legacy `RuleProcessor` rather than infer the desired tree shape (`renderable/features/2026-05-20-darkmatter-tree/span-aware-processor-design.md:420`-`430`). The span-aware processor rewrites any simple paragraph matching the HR-attribute pattern (`darkmatter/lib/src/markdown/render_tree/span.rs:714`-`742`), but I did not find a fold or parity test for the list-item case; the parity corpus only has a top-level `hr_attributes` fixture (`darkmatter/lib/tests/render_tree_parity.rs:161`-`164`).

Verification level: absent for this user-observable behavior. Add a Level 1 fold test and parity test comparing legacy-vs-tree behavior for the list-item fixture, then classify the outcome in the ledger.

## Verification Matrix

| Requirement | Strongest verification found | Assessment |
|---|---:|---|
| Experimental tree entry points for HTML, Terminal, Markdown, MarkdownPlus | Level 1 smoke tests | Adequate for existence only. |
| Mark/dim/HR top-level rendering | Level 1 plus optional Level 2 WezTerm for terminal styling | Mostly adequate. |
| Terminal `ColorDepth::None` including code blocks | No tree-path assertion found | Gap. |
| Rich code titles, line numbers, highlights, syntax highlighting | Level 1 token preservation; terminal ANSI presence only | Gap. |
| Browser/HTML code highlighting | No implemented hook in browser tree renderer | Gap. |
| HR attributes inside list items | No explicit fixture found | Gap. |

## Notes

- I used the required `renderable` skill. The requested `root` skill is not present in this session's skill catalog.
- The prompt path uses `renderable/root/features/...`; the actual feature directory in this worktree is `renderable/features/2026-05-20-darkmatter-tree/`, so I saved this review there.
- I attempted `cargo test -p darkmatter render_tree_terminal_syntax_highlights_code_blocks --color=never`; it was still compiling after about 60 seconds, so I terminated it per the non-interactive session guidance.
