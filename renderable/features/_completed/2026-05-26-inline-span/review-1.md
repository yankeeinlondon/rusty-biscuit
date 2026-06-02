---
ready: false
agent: codex
model: ""
---

# Review: Inline Span Extensions

## Findings

### High: Source rewrite runs inside code spans and code blocks

The rewriter scans the whole Markdown source before `pulldown-cmark` classifies inline code or fenced code blocks: `fold_markdown_spanned_with_frontmatter` rewrites `md.content()` first, then parses the rewritten string ([fold.rs:414](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/darkmatter/lib/src/markdown/render_tree/fold.rs:414), [fold.rs:423](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/darkmatter/lib/src/markdown/render_tree/fold.rs:423)). The scanner itself has no awareness of backtick spans, fenced blocks, indented code, raw HTML, or link destinations; it matches every registered delimiter in the byte stream ([inline_extension.rs:218](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/darkmatter/lib/src/markdown/render_tree/inline_extension.rs:218), [inline_extension.rs:298](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/darkmatter/lib/src/markdown/render_tree/inline_extension.rs:298)).

That changes literal code content. Inputs like `` `==code==` ``, `` `⌄code⌄` ``, and fenced blocks containing `==`/`⌄` will be rewritten to synthetic envelope text before the parser can protect them as code. This regresses established darkmatter behavior; the legacy tests explicitly require inline and fenced code to preserve dim delimiters literally and not trigger dim styling ([terminal.rs:8972](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/darkmatter/lib/src/markdown/output/terminal.rs:8972), [terminal.rs:8991](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/darkmatter/lib/src/markdown/output/terminal.rs:8991)).

Verification level present: Level 1 fold/unit coverage for normal mark/dim spans only. Missing Level 1 regression tests for code spans and fenced/indented code. This is a correctness blocker.

### High: The `{{|TOKEN|}}` envelope can corrupt GFM table parsing

The locked marker contains unescaped pipe characters. Because the rewrite happens before `pulldown-cmark` table parsing, a table cell like `| ==highlighted== | ok |` becomes a table row containing extra raw `|` bytes from `{{|mark|}}` before the table parser decides cell boundaries ([inline_extension.rs:276](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/darkmatter/lib/src/markdown/render_tree/inline_extension.rs:276), [inline_extension.rs:284](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/darkmatter/lib/src/markdown/render_tree/inline_extension.rs:284), [fold.rs:423](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/darkmatter/lib/src/markdown/render_tree/fold.rs:423)).

The prototype notes assert pipes are inert because the envelope “never sits in” table rows, but mark/dim are inline constructs and can naturally appear in table cells. There is no test for mark/dim inside a GFM table cell; the current parity table fixture has no darkmatter inline content ([render_tree_parity.rs:626](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/darkmatter/lib/tests/render_tree_parity.rs:626)).

Verification level present: none for table-cell inline extensions. Missing Level 1 tests that fold and render mark/dim inside tables. This is a correctness blocker unless the marker changes or the rewrite becomes Markdown-structure aware.

### High: User-observable terminal styling is not verified at the required level

The spec requires target-specific observable rendering: `mark` must render as terminal reverse video, `dim` as terminal dim SGR, and browser `mark` as a semantic `<mark>` element. The implementation adds those renderer arms ([render.rs:759](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/biscuit-terminal/lib/src/render_tree/render.rs:759), [browser.rs:302](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/renderable/src/tree/render/browser.rs:302)), and there are Level 1 tests/smokes for fold and entrypoint behavior.

There is no Level 2 real-terminal capture for the new inline extension rendering. Under the requested test-rigor rules, SGR styling and glyph/rendering behavior need Level 2 coverage because unit tests only inspect generated strings or internal structures. The strongest current coverage is Level 1, so the terminal-facing portion cannot be called production-ready.

### Medium: Performance receipts do not measure the production path for no-inline documents

The spec expects documents without darkmatter inline syntax to pay only a cheap rewrite scan plus plain-fold speed. The internal entrypoint always uses `fold_markdown_spanned_with_frontmatter` ([entrypoints.rs:55](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/darkmatter/lib/src/markdown/render_tree/entrypoints.rs:55)), but the migration benchmark routes every fixture except `mark_dim_hr` through `fold_markdown_to_document` ([migration_parity.rs:357](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/darkmatter/lib/benches/migration_parity.rs:357), [migration_parity.rs:367](/Users/ken/.claudine/worktrees/rusty-biscuit/renderable/darkmatter/lib/benches/migration_parity.rs:367)).

That means the published benchmark shape hides the rewrite scan cost for ordinary documents, exactly the scenario the spec calls out. Add a benchmark lane that uses the production spanned fold for no-inline fixtures, or make the production entrypoint use the same conditional path being measured.

## Coverage Notes

Current useful Level 1 coverage includes canonical envelope prototype tests, rewriter unit tests for mark/dim, fold tests for nested mark/dim, provenance tests, and renderer/entrypoint smoke tests. The missing coverage is concentrated around Markdown contexts the source-level rewrite must not alter: code spans, fenced/indented code, table cells, raw HTML, link destinations/titles, and image alt text.

I attempted two targeted Cargo test commands, but both exceeded the non-interactive session limit after waiting on Cargo locks/compilation and were stopped. No successful local test result is recorded in this review.

## Recommendation

Not ready for production. Fix the rewrite so it only operates on eligible prose text, or choose an envelope that cannot perturb Markdown block/table parsing and explicitly protect verbatim regions. Then add Level 1 regression tests for protected Markdown contexts and Level 2 terminal capture tests for the user-observable SGR behavior.
