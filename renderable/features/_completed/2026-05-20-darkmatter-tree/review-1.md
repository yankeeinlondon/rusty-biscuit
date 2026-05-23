---
ready: true
agent: codex
model: ""
---

# Review: Darkmatter Tree Rendering Migration

## Resolution Notes (2026-05-20)

All six review findings have been addressed. The summary below pairs each
finding with the resolving artefacts; the original analysis follows
unchanged.

| Finding (severity)                                | Status   | Resolution                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
|---------------------------------------------------|----------|--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| Entry points + phase-separated result (High)      | Resolved | New `darkmatter::markdown::render_tree::pipeline::{PipelineResult, PipelineRenderResult}` and the `pub(crate)` entry-point set (`to_render_document`, `render_tree_html`, `render_tree_terminal`, `render_tree_markdown`, `render_tree_markdown_dialect`) under `entrypoints.rs`. Smoke tests in `entrypoints::tests` exercise the adapter boundary and assert that fold and render diagnostics stay in separate streams.                                                       |
| Span-aware mark / dim / HR processors (High)      | Resolved | New `render_tree::span` module defines `SpannedInlineEvent`, `SpannedEventProvenance`, `SpanningAdapter`, `SpannedInlineStyleProcessor`, and `SpannedRuleProcessor`. New `fold_markdown_spanned_with_frontmatter` folds mark → `Span(class="mark")`, dim → `Span` with `darkmatter.style.dim` hint, and HR-attribute paragraphs → `ThematicBreak` with `darkmatter.hr.*` hints and `Provenance::Generated`. Fold tests cover basic mark, unclosed mark, dim, HR attrs, plain HR, and escaped delimiters. |
| Frontmatter above the fold (High)                 | Resolved | New `fold_markdown_with_frontmatter` and `fold_markdown_to_document_with_metadata` accept darkmatter's already extracted frontmatter and populate `renderable::tree::DocumentMetadata::frontmatter`. Round-trip test renamed to `render_tree_frontmatter_is_not_extracted_by_body_fold` (body-only contract) plus `render_tree_frontmatter_above_the_fold_attaches_metadata` (DMTR-4 contract). The entry-point smoke `frontmatter_attaches_via_entry_point` exercises the same path through `to_render_document`.                                                                  |
| Level 2 terminal verification (High)              | Resolved | New `darkmatter/lib/tests/level2_render_tree_terminal.rs` runs the tree-backed terminal renderer through a real WezTerm pane via `biscuit-test-harness`. Three tests cover heading + paragraph text, inline-style ANSI styling, and table cells. Skips silently when `WEZTERM_UNIX_SOCKET` is unset; hard-fails when `DARKMATTER_LEVEL2_REQUIRED=1`.                                                                                                                                                                            |
| Legacy HTML table parser gap (Medium)             | Resolved | `as_html` now parses with `ENABLE_STRIKETHROUGH | ENABLE_TABLES`, and the catch-all event arm now handles `Tag::Table`, `Tag::TableHead`/`TableRow`/`TableCell` (and their `TagEnd`s) so the cell text is wrapped in real `<table>`/`<tr>`/`<td>` elements. The parity test `render_tree_parity_table` was strengthened: in addition to text-token checks it now asserts both pipelines emit structural `<table>`, `<tr>`, and `<td>` elements.                                                                                       |
| Benchmark harness (Medium)                        | Resolved | New `darkmatter/lib/benches/migration_parity.rs` with the spec's six groups (`migration/terminal`, `migration/terminal_no_color`, `migration/browser`, `migration/markdown`, `migration/fold_only`, `migration/full_pipeline`), pinned terminal/browser options, and four fixture generators. `Cargo.toml` registers the `migration_parity` `[[bench]]` target; `cargo bench -p darkmatter --bench migration_parity` compiles cleanly. The existing `render_tree.rs` bench was left untouched per the spec. |

## Findings

## Findings

### High: Required experimental entry points and phase-separated pipeline result are missing

The spec requires internal/module-level tree APIs for `to_render_document`, `render_tree_html`, `render_tree_terminal`, and `render_tree_markdown`, plus a `PipelineResult<T>` / `PipelineRenderResult<T>` shape that keeps fold diagnostics separate from render diagnostics. I could not find those symbols under `darkmatter::markdown::render_tree`; the module currently only re-exports `TerminalCodeRenderer` and `fold_markdown_to_document` (`darkmatter/lib/src/markdown/render_tree/mod.rs:50`-`56`).

This leaves DMTR-1 and the diagnostic-model design unimplemented. The tests exercise the fold directly and then call target renderers manually, so they do not verify the adapter boundary that downstream callers are supposed to use or the requirement that mismatches identify fold-phase versus render-phase diagnostics.

Verification level: Level 1 only, and only for manually composed fold/render calls. No test covers the required user-facing experimental entry points because they do not exist.

### High: Span-aware mark, dim, and HR-attribute processing is not implemented

The design requires a span-aware processor chain so `==mark==`, dim delimiters, escaped delimiters, and horizontal rules with attribute blocks fold with correct byte provenance and Darkmatter HR hints. The current implementation explicitly defers those constructs (`darkmatter/lib/src/markdown/render_tree/mod.rs:22`-`29`; `darkmatter/lib/src/markdown/render_tree/fold.rs:9`-`19`) and the fold still consumes raw `Parser::new_ext(...).into_offset_iter()` events directly (`darkmatter/lib/src/markdown/render_tree/fold.rs:199`).

That means the tree fold cannot see Darkmatter's custom inline styles or HR attributes. The required `Span.mark`, dim `Style`, `darkmatter.hr` hints, escaped-delimiter behavior, and generated provenance are absent.

Verification level: Level 1 tests exist for ordinary Markdown inline styles, but not for the required Darkmatter mark/dim/HR behavior. This is a functionality gap, not just a test gap.

### High: Frontmatter metadata is required but intentionally pinned as absent

DMTR-4 requires a render-tree construction path that accepts Darkmatter's already extracted frontmatter and stores it in `DocumentMetadata`. The fold documents that `DocumentMetadata::frontmatter` is always `None` (`darkmatter/lib/src/markdown/render_tree/fold.rs:161`-`164`), and the integration test asserts that frontmatter is not extracted (`darkmatter/lib/tests/render_tree_roundtrip.rs:342`-`370`).

This directly contradicts the feature acceptance criterion that folded documents can carry extracted frontmatter metadata.

Verification level: Level 1 currently verifies the wrong behavior for this feature stage.

### High: Terminal user-observable rendering is only verified at Level 1

The parity suite renders terminal strings in-process, strips ANSI, and checks semantic text tokens (`darkmatter/lib/tests/render_tree_parity.rs:18`-`31`, `405`-`418`). That is useful, but it is Level 1. The spec's review rubric requires Level 2 for real terminal rendering concerns such as glyphs, widths, SGR styling, and scrolling. This feature includes terminal rendering as an acceptance target, yet there is no real-terminal capture through WezTerm, Kitty, or tmux for the Darkmatter tree path.

As written, the tests cannot catch regressions where the rendered terminal bytes look correct as strings but fail in an actual terminal pane due to width, styling, or wrapping behavior.

Verification level: strongest present is Level 1; expected minimum for terminal user-observable rendering is Level 2.

### Medium: Parser-option policy is documented, but the legacy HTML table gap remains and the parity test is too weak to catch it

The parser-options design says legacy `as_html` should add `ENABLE_TABLES` or the gap must be explicitly recorded before relying on parity. The HTML renderer still parses with only `Options::ENABLE_STRIKETHROUGH` (`darkmatter/lib/src/markdown/output/html.rs:157`-`161`), while the shared Markdown parse options elsewhere include tables (`darkmatter/lib/src/markdown/mod.rs:860`-`862`).

The table parity test only checks stripped text tokens, so legacy HTML that treats the pipe-table syntax as paragraph text can still pass. It does not assert structural table output or `<table>` semantics, so it cannot guard the public-now parser-option contract.

Verification level: Level 1 exists, but it verifies token survival rather than the user-observable table semantics required by the spec.

### Medium: Benchmark harness does not match the specified migration parity benchmark

The spec requires `darkmatter/lib/benches/migration_parity.rs`, paired legacy/tree groups, pinned terminal/browser options, a `terminal_no_color` benchmark group, and the command `cargo bench -p darkmatter --bench migration_parity`. The implementation adds `darkmatter/lib/benches/render_tree.rs` and registers `render_tree` in `Cargo.toml`, but I did not find `migration_parity` (`darkmatter/lib/Cargo.toml:93`-`106`).

The current benchmark measures fold and tree renderers, but it does not provide the specified legacy-vs-tree parity baseline or no-color terminal comparison needed before any production cutover.

## Production Readiness

Not ready for production.

The tree fold covers a useful CommonMark/GFM subset, and the Level 1 round-trip and semantic parity tests are a good starting point. However, multiple required feature items are explicitly absent: adapter entry points, separated pipeline results, span-aware Darkmatter processors, frontmatter metadata, the specified benchmark harness, and Level 2 terminal verification. The branch should stay experimental until those gaps are closed or the feature scope is formally narrowed.

## Verification Performed

- Read the feature spec and companion designs in `renderable/features/2026-05-20-darkmatter-tree/`.
- Reviewed `darkmatter::markdown::render_tree` implementation and render-tree integration tests.
- Attempted `cargo test -p darkmatter --test render_tree_roundtrip --test render_tree_parity --color=never`; it was still compiling after about one minute and was stopped to avoid a long non-interactive build.
