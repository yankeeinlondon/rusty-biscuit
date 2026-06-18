---
ready: true
agent: codex
model: ""
---

# Review: Darkmatter Tree Rendering Migration, Iteration 3

## Resolution Notes (2026-05-20)

All four review findings have been addressed:

| Finding (severity)                                          | Status   | Resolution                                                                                                                                                                                                                                                                                                                                                                                                                          |
|-------------------------------------------------------------|----------|-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| Dim hint not consumed by renderers (High)                   | Resolved | `ContainerKind::Dim` now sets `Style.emphasis.dim = true` via `set_style`, so the browser renderer lowers it to `opacity:0.6` and the terminal renderer emits the dim SGR (`\x1b[2m`) automatically. New entry-point tests `render_tree_html_emits_dim_styling_for_dim_span` and `render_tree_terminal_emits_dim_sgr_for_dim_span` pin the visible-style outcome on both targets.                                                    |
| Mark/dim cannot wrap nested Markdown events (High)          | Resolved | `SpannedInlineStyleProcessor` now tracks mark / dim state across pulldown text events: openers are buffered until a matching closer is seen, and unclosed openers revert to literal text at the paragraph boundary. The dim flanking rule is leniently relaxed at text-event boundaries so the design fixture `⌄*dim and italic*⌄` folds to the expected `Span(dim) → Emphasis → Text` shape. New fold tests `span_aware_fold_wraps_emphasis_inside_{mark,dim}` and `span_aware_fold_unclosed_cross_event_mark_reverts` pin the behavior. |
| Level 2 does not exercise span-aware behavior (High)        | Resolved | `level2_render_tree_terminal.rs` gains a `render_tree_terminal_spanned_to_tempfile` helper that drives `fold_markdown_spanned_with_frontmatter`, plus three new Level 2 tests (`level2_tree_mark_renders_reverse_video_in_real_terminal`, `level2_tree_dim_renders_dim_sgr_in_real_terminal`, `level2_tree_hr_attributes_render_styled_rule_in_real_terminal`) that assert reverse-video / dim SGR / no raw-source leak in a real WezTerm pane.   |
| Benchmark corpus incomplete + no recorded baselines (Med)   | Resolved | `migration_parity` gains four new fixtures (`small_prose`, `large_prose`, `mark_dim_hr`, `image_heavy`) so the corpus now matches the spec's category list, plus a new `bench_fold_once_multi_target` group that quantifies the parse-and-fold-once-render-many payoff. Captured baselines are recorded in `renderable/features/2026-05-20-darkmatter-tree/baselines.md` with capture commands and a per-group ratio table.        |

## Findings

### High: Dim spans are stored in a hint that the target renderers do not consume

The span-aware design says `InlineEvent::Start(InlineTag::Dim)` / `End` should fold to a `Span` with a `Style` hint whose `TextEmphasis.dim = true`, and explicitly says dim should ride in `Style` rather than semantic emphasis (`span-aware-processor-design.md:210`-`216`). The implementation instead builds a plain `Span` with no classes and stores `darkmatter.style.dim = true` in arbitrary `NodeAttrs::data` (`darkmatter/lib/src/markdown/render_tree/fold.rs:697`-`707`).

That does not reach either renderer. Browser style lowering reads `attrs.style()` and lowers `style.emphasis.dim` to `opacity:0.6` (`renderable/src/tree/render/browser.rs:991`-`1047`). Terminal span rendering applies visual dim only for a `"dim"` class, or for actual style handled elsewhere; it never reads the `darkmatter.style` data hint (`biscuit-terminal/lib/src/render_tree/render.rs:1306`-`1325`, `biscuit-terminal/lib/src/render_tree/render.rs:1527`-`1537`). So `⌄dimmed⌄` preserves text but loses the user-observable dim styling in the tree Browser and Terminal paths.

Verification level: current tests are Level 1 and mostly assert that the word survives (`darkmatter/lib/tests/render_tree_parity.rs:691`-`717`) or that the opaque hint exists. That is the wrong verification for a visual style requirement. Browser should assert the emitted DOM/CSS carries dim styling, and terminal needs Level 2 capture showing dim SGR or an accepted documented fallback.

### High: Mark/dim delimiters still cannot wrap nested Markdown events

The design fixture `⌄*dim and italic*⌄` requires a dim `Span` containing an `Emphasis` node (`span-aware-processor-design.md:321`-`335`). The processor cannot produce that shape because delimiter state is local to a single `Event::Text`: `in_mark`, `mark_start_segment`, and `dim_opener_segments` are initialized inside `process_text` (`darkmatter/lib/src/markdown/render_tree/span.rs:175`-`191`) and discarded before the next pulldown event. For input like `⌄*dim and italic*⌄`, pulldown emits delimiter text on either side of `Start(Emphasis)` / `End(Emphasis)`, so the opener and closer are processed in separate calls and are treated as unpaired literals.

The test that was added for the prior sidecar bug only covers `==marked== then *italic*`, an emphasis sibling after a closed mark span (`darkmatter/lib/src/markdown/render_tree/fold.rs:1318`-`1340`). The adjacent comment even calls `==*emphasis inside mark*==` out of scope (`darkmatter/lib/src/markdown/render_tree/fold.rs:1312`-`1316`), but that is not out of scope for this feature: the span-aware design lists nested Markdown inside dim as a required fixture.

Verification level: no Level 1 fold test covers the designed nested-inline fixture, and the implementation does not satisfy it. Add fold tests for dim-with-italic and mark-with-emphasis before relying on target parity.

### High: Level 2 coverage does not exercise the span-aware user-visible behavior

The Level 2 test file folds fixtures through `fold_markdown_to_document`, not `fold_markdown_spanned_with_frontmatter` or the experimental entry points (`darkmatter/lib/tests/level2_render_tree_terminal.rs:93`-`110`). Its fixtures cover heading text, ordinary emphasis/strong, and table cells (`darkmatter/lib/tests/level2_render_tree_terminal.rs:152`-`222`), but not `==mark==`, `⌄dim⌄`, or `--- { style: waves }`.

That leaves the user-observable darkmatter-specific behavior at Level 1. The parity test itself says exact HR styling is checked separately at Level 2 (`darkmatter/lib/tests/render_tree_parity.rs:720`-`727`), but no such Level 2 HR test exists. Under the review rubric, visual terminal behavior such as dim styling, mark highlighting, HR glyphs/styles, widths, and SGR output is not production-ready with only string-level token survival checks.

### Medium: Benchmark coverage and recorded baselines are still incomplete

DMTR-6 requires comparisons for fold-once/multiple-target rendering and compose-then-fold-once/multiple-target rendering, plus corpus categories including small prose, large prose, mark/dim/HR attributes, image/Mermaid, and transclusion-heavy composed documents (`spec.md:304`-`322`). The benchmark harness currently has four stress inputs: large code block, large table, deeply nested lists, and many links/images (`darkmatter/lib/benches/migration_parity.rs:50`-`119`). It also measures full pipeline only for terminal, and I did not find recorded baseline numbers in the feature directory despite the acceptance criterion requiring benchmark commands and baseline numbers (`spec.md:332`-`336`).

This is not a functional blocker for the internal experimental API, but it is a production-readiness blocker for any public cutover decision.

## Verification-Level Summary

| Requirement | Strongest observed verification | Assessment |
| --- | --- | --- |
| Experimental entry points use span-aware fold | Level 1 unit/entry-point tests | Adequate for wiring. |
| `==mark==` folds and text survives targets | Level 1 fold/target token tests | Missing Level 2 terminal style verification. |
| `⌄dim⌄` folds and renders dim styling | Level 1 fold hint/text tests | Gap: renderer ignores the hint; no Level 2 style check. |
| Nested Markdown inside mark/dim | No matching test | Gap: implementation cannot pair delimiters across pulldown events. |
| HR attributes do not leak raw source | Level 1 token tests | Missing Level 2 terminal HR style/glyph coverage. |
| Parser option policy | Level 1 unit/integration tests for several options | Mostly adequate for this stage. |
| Benchmark/readiness evidence | Harness present | Incomplete corpus and no recorded baseline numbers found. |

## Production Readiness

Not ready for production.

The previous review's entry-point wiring and sidecar-frame issues are improved, but dim styling is not actually rendered, nested Markdown inside mark/dim still fails the design contract, and the Level 2 suite does not cover the darkmatter-specific terminal behavior that the feature is meant to prove.

## Verification Performed

- Read the feature spec and span-aware processor design.
- Reviewed `darkmatter::markdown::render_tree` entry points, fold, span processors, parity tests, Level 2 terminal tests, benchmark harness, and relevant renderable/biscuit-terminal span rendering code.
- Attempted `cargo test -p darkmatter --lib markdown::render_tree:: --color=never` and `cargo test -p darkmatter --test render_tree_parity --color=never`; both were still compiling dependencies after the useful non-interactive review window, so I stopped them and do not claim test results from those runs.
- Requested `root` skill could not be used because no `root` skill is available in this session's skill catalog; I used the provided repo instructions and local feature docs instead.
