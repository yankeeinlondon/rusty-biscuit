# Reference Validation Review

`just test` passes for `darkmatter`, but the current test suite does not exercise the feature deeply enough to catch several structural problems in the implementation.

## Highest-priority recommendations

### 1. Fix the recursive graph builder before treating this feature as complete

The current graph implementation is not functionally complete for transcluded documents.

- `darkmatter/lib/src/markdown/reference/graph.rs:38-43` and `:59-64` populate `ReferenceGraph.nodes` from `collect_child_nodes(&root)`.
- `darkmatter/lib/src/markdown/reference/graph.rs:337-343` returns `Vec::new()` unconditionally.
- `darkmatter/lib/src/markdown/reference/graph.rs:276-283` discards the built child nodes and replaces them with empty `Markdown::new("")` placeholders in `loaded_markdown`, which is never read back.

Impact:

- `transclusion_graph()`, `reference_graph()`, `composed_references()`, `composed_links()`, `composed_image_references()`, `ReferenceGraph::to_mermaid()`, `ReferenceGraph::to_dot()`, and the CLI `validate refs --graph` path all collapse to a root-only view.
- Any validation or composed-order analysis that depends on child nodes is currently wrong or incomplete.

Recommended change:

- Store actual `ReferenceGraphNode`s in the runtime and return them.
- Make `collect_child_nodes()` real, or better, construct the flat node list during traversal instead of reconstructing it later.
- Add integration tests with `tempfile` for `::file`, `prologue`, `epilogue`, nested recursion, cycles, and depth limits.

### 2. Remove the `has_transclusions()` gate from InlinePre preparation

The implementation currently applies InlinePre only when the document has transclusions:

- `darkmatter/lib/src/markdown/reference/graph.rs:126-131`

This is too narrow. The spec/design explicitly calls out references being introduced or removed by:

- page blocks
- interpolation
- shell expansion

Those transformations can matter even in leaf documents with no transclusions at all.

Impact:

- A document like `[link]({{ target }})` or a link inside `::block ... ::end-block` is analyzed in raw form unless the document also happens to contain a transclusion directive.
- `validate_references()` can report false positives/false negatives for the effective document.

Recommended change:

- Detect whether any InlinePre-affecting syntax is present, not just transclusions, or simply run the InlinePre subset for graph analysis unconditionally.
- Add tests for page blocks, interpolation, and shell expansion in leaf documents.

### 3. Respect `ReferenceGraphOptions.compose`

`ReferenceGraphOptions` advertises a caller-controlled `compose: ComposeOptions` surface, but almost all of it is ignored.

- `darkmatter/lib/src/markdown/reference/types.rs:385-400` defines `compose`, `include_generated_toc_links`, and `follow_remote_transclusions`.
- `darkmatter/lib/src/markdown/reference/graph.rs:31`, `:52` only use `options.compose.max_transclusion_depth`.
- `darkmatter/lib/src/markdown/reference/graph.rs:289-309` builds a fresh `ComposeOptions::new().only(...)` and ignores the caller's external state, overrides, context, cache settings, shell settings, fail-fast, and freshness controls.

Impact:

- Caller-supplied state/context does not affect reference analysis.
- Cache knobs are effectively ignored.
- `include_generated_toc_links` and `follow_remote_transclusions` are dead options today.

Recommended change:

- Start from `options.compose.clone()`, constrain it to InlinePre-only operations, and preserve source/context/cache/shell settings.
- Either implement `include_generated_toc_links`/`follow_remote_transclusions` now or remove/defer them until they are wired end-to-end.

### 4. Emit actual transclusion reference records

The design added `ReferenceKind::Transclusion`, but the graph never emits any.

- `darkmatter/lib/src/markdown/reference/types.rs:12-27` defines `ReferenceKind::Transclusion`.
- `darkmatter/lib/src/markdown/reference/graph.rs:197-204` explicitly comments that the directive is "already captured", but it is not.
- `darkmatter/lib/src/markdown/reference/mod.rs:76-145` returns `TransclusionRef`s, but `resolved_target` is always `None`.

Impact:

- `reference_graph()` and `composed_references()` omit a whole reference class the spec asked for.
- `::code`, `::url`, `::toc-linking`, `prologue`, and `epilogue` are not represented as reference records in the unified model.
- Validation cannot report on those references as first-class inputs.

Recommended change:

- Convert local `transclusions()` output into `ReferenceRecord`s and include them in `local_references`.
- Fill `resolved_target` when source context is available.
- Add tests proving transclusion records appear in local, graph, and composed views.

### 5. Fix validation to resolve against the reference origin, not only the root document

The validator currently uses the root markdown source and root headings for all records:

- `darkmatter/lib/src/markdown/reference/validate.rs:143`
- `darkmatter/lib/src/markdown/reference/validate.rs:157`
- `darkmatter/lib/src/markdown/reference/validate.rs:162-164`
- `darkmatter/lib/src/markdown/reference/validate.rs:145-150`

Impact:

- Once recursive graph traversal is fixed, child references will still be validated relative to the root file instead of `record.origin.source`.
- Same-document fragment validation uses `md.toc()` from the root markdown, not the composed heading set of the effective document.
- Cross-document fragment checks load raw target markdown, not a prepared/composed view.

Recommended change:

- Resolve local paths relative to `record.origin.source`.
- Build fragment heading sets from the same prepared/composed view used by reference extraction.
- Add tests where a transcluded child contains `./relative.md` and where the only valid fragment target comes from composed content.

### 6. Stop inventing separate path resolution semantics

The design explicitly called for reuse of existing path semantics, including `biscuit-file::FileReference` and repo-root `@` handling.

- `darkmatter/lib/src/markdown/reference/graph.rs:312-325`
- `darkmatter/lib/src/markdown/reference/validate.rs:275-308`
- `darkmatter/lib/src/markdown/reference/validate.rs:338`

Current behavior is just `base_dir.join(raw)`.

Impact:

- Repo-root `@` paths are not supported.
- Reference validation can disagree with compose/transclusion behavior.
- URL/file resolution logic is duplicated in multiple places.

Recommended change:

- Route reference resolution and validation through the same `biscuit-file::FileReference` path logic compose already depends on.
- Add tests for `@repo-root/path.md`, missing source context, and URL-vs-file disambiguation.

## Non-working or incorrect behaviors

### 7. HTML extraction misclassifies tags

There are two correctness bugs in the HTML classifier layer.

#### 7a. `<link>` tags are also being reported as hyperlinks

- `darkmatter/lib/src/markdown/reference/html.rs:116-144`

`classify_a_tag()` does not check `is_tag(html, "a")`; it accepts any HTML node containing `href`.

Impact:

- `<link rel="stylesheet" href="style.css">` is emitted as a hyperlink in addition to being emitted as a CSS/font import.

Recommended change:

- Require `is_tag(html, "a")` before extracting `href`.
- Add a regression test that `<link ...>` does not appear in `extract_html_links()`.

#### 7b. Any `<link href=...>` becomes a CSS import unless it looks like a font

- `darkmatter/lib/src/markdown/reference/html.rs:271-319`

`classify_link_tag()` treats all non-font `<link>` tags as `CssImport`, regardless of `rel`.

Impact:

- `rel="canonical"`, `rel="alternate"`, `rel="preconnect"`, etc. become false CSS imports and can trigger bogus validation failures.

Recommended change:

- Restrict CSS imports to actual stylesheet-like rel values.
- Add tests for canonical/preconnect/alternate links.

### 8. Flattening loses sibling insertions on the same line

- `darkmatter/lib/src/markdown/reference/graph.rs:80-85`

`flatten_node()` uses `BTreeMap<usize, &ReferenceInsertion>`, so only one insertion survives per `directive_line`. `insertion_order` is stored but never used.

Impact:

- Multiple transclusions on the same line, multiple prologues, or multiple epilogues can be flattened in the wrong order or dropped.

Recommended change:

- Use `BTreeMap<usize, Vec<&ReferenceInsertion>>` and sort by `insertion_order`.
- Add tests with multiple prologues/epilogues and multiple directives on the same source line.

## Missing functionality

### 9. The public API surface is still incomplete against the spec/design

Implemented:

- `has_transclusions()`
- `transclusions()`
- `transclusion_graph()`
- `reference_graph()`
- `composed_references()`
- `composed_links()`
- `composed_image_references()`
- local `inline_css()`, `css_imports()`, `inline_scripts()`, `script_imports()`, `font_imports()`, `meta_tags()`, `merge_meta_into_frontmatter()`, `validate_references()`

Still missing:

- `inline_css_graph()`
- `css_import_graph()`
- `inline_script_graph()`
- `script_import_graph()`
- `font_import_graph()`
- `set_meta_tag()`

Recommendation:

- Either finish the remaining graph-aware APIs before marking the feature done, or explicitly scope them out of this feature branch and update the spec/design/plan so the public contract matches reality.

### 10. Generated TOC links and remote transclusions are declared but not implemented

- `darkmatter/lib/src/markdown/reference/types.rs:388-391`
- no call sites use either option

Recommendation:

- Implement them or remove them from the public options for now.

## Validation-model gaps

### 11. Several issue codes exist but are never emitted

- `RemoteDisallowed`
- `MalformedHtmlTag`
- `MalformedCssImport`
- `MalformedMetaTag`

These are defined in `darkmatter/lib/src/markdown/reference/validate.rs:93-114` but not produced anywhere.

Recommendation:

- Either wire them into extraction/validation or delete/defer them to avoid a misleading API surface.

### 12. CSS extraction is regex-based, not parser-based

- `darkmatter/lib/src/markdown/reference/css.rs:15-29`

This is workable for simple cases, but it does not meet the design's stated correctness bar for CSS parsing, and it leaves no structured path to emit `MalformedCssImport`.

Recommendation:

- Switch to `cssparser` for `@import` and `@font-face src` extraction.
- Add tests for comments, nested functions, media queries, multiple `src` URLs, and malformed CSS diagnostics.

## Test coverage recommendations

The current reference-validation coverage is mostly unit-local. That is useful, but it is not enough for a transclusion-aware feature.

### 13. Add library integration tests for the composed graph behavior

Missing high-value tests:

- recursive `::file` traversal
- `prologue` and `epilogue`
- page blocks removing references
- interpolation changing href/src targets
- shell expansion injecting references
- multiple child insertions on the same line
- cycle detection and depth limits
- emitted transclusion records in local/graph/composed views
- generated TOC links when enabled

### 14. Add validation integration tests

Missing high-value tests:

- child-origin relative path validation
- repo-root `@` path resolution
- same-document fragments against composed headings
- cross-document fragments where the target heading is only present after preparation/composition
- remote validation with `wiremock` instead of real network access
- malformed HTML/CSS/meta diagnostics once implemented

### 15. Add CLI integration tests for the new surface

There are no CLI tests covering `validate refs` or `validate refs --graph`.

Recommendation:

- Add CLI tests for text output, JSON output, non-zero exit on errors, `--fragments`, `--graph mermaid`, and `--graph dot`.

## Caching assessment

## 16. The feature does not adequately leverage the recent compose caching work

At the moment, the reference-analysis path is mostly bypassing the new caching architecture.

Evidence:

- `prepare_content()` calls `md.compose_with(...)` per node with a freshly created runtime every time (`darkmatter/lib/src/markdown/reference/graph.rs:289-309`).
- That means the run-local cache owned by `PipelineRuntime` is not shared across graph traversal.
- `ReferenceAnalysisRuntime.loaded_markdown` exists (`darkmatter/lib/src/markdown/reference/graph.rs:20-23`) but is not used as a real document cache.
- Child loads use `Markdown::try_from(...)` directly (`darkmatter/lib/src/markdown/reference/graph.rs:176`, `:217`, `:245`) instead of `runtime.cache.load_markdown(...)`.
- TOC/heading extraction in validation also bypasses compose runtime caching.
- Even persistent cache configuration from `ReferenceGraphOptions.compose` is ignored because `prepare_content()` rebuilds `ComposeOptions` from scratch.

Net assessment:

- The implementation currently benefits from compose caching only accidentally, and only in the narrow case where an individual `compose_with()` call happens to be configured for persistence.
- It does not reuse the run-local cache in the way the compose runtime was designed to support.

Recommended change:

- Share a single `PipelineRuntime` or `RunLocalCache` across the entire reference-analysis traversal.
- Use the same cached `load_markdown()` / `load_toc_headings()` helpers compose already exposes.
- Thread caller cache settings through `ReferenceGraphOptions.compose`.
- Add tests that assert repeated child loads and repeated TOC extraction hit the shared cache path.

## Ergonomics and performance opportunities

### 17. Avoid repeated extraction passes over the same HTML

- `darkmatter/lib/src/markdown/reference/graph.rs:140-149` calls `extract_html_style_blocks()` twice.
- Similar repeated scans exist between local API methods and the graph builder.

Recommendation:

- Parse HTML blocks once per node and fan out into link/image/style/script/meta/import records from the same intermediate representation.

### 18. Prefer direct child-node storage over node-id reconstruction

Today the code creates `child_insertions` plus a separate flat `nodes` collection. That is fine, but the current implementation pays the complexity cost without the benefit.

Recommendation:

- Either store child nodes directly during recursive construction and derive the flat list afterward, or keep a runtime-owned node arena keyed by stable IDs.

## Suggested next order of work

1. Repair graph construction and flattening.
2. Make InlinePre preparation unconditional for analysis-relevant syntax and honor `ReferenceGraphOptions.compose`.
3. Switch validation to `record.origin.source` and shared path-resolution helpers.
4. Emit transclusion records and finish the missing graph-aware APIs.
5. Add the missing integration and CLI tests.
6. Refactor the analysis path to share compose caching/runtime facilities.
