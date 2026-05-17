# Render Tree Implementation Plan

## Status

The current spec is solid enough to implement. I do not see an immediate design
blocker that should stop work. The remaining open questions are correctly
scoped to Milestone 1 implementation details:

- exact `SourceDescriptor` contents
- whether `RenderError` is one shared type or per-target error types
- verified `pulldown-cmark` 0.13 event inventory

The plan below treats those as early deliverables, not prerequisites.

## Success Criteria

The feature is successful when:

- `renderable` owns the canonical tree model, builders, validation, diagnostics,
  Markdown renderer, Browser renderer, and `TreeRenderable`.
- `darkmatter` can fold real `pulldown-cmark` 0.13 event streams into
  `renderable::Document`.
- `biscuit-terminal` can render `RenderNode` / `Document` to terminal output
  without adding any reverse dependency from `renderable`.
- Milestone 1 proves the model with `text -> tree -> Markdown` golden fixtures
  before the API is hardened around untested assumptions.
- No existing public `darkmatter::as_html` / `for_terminal` behavior is migrated
  onto the tree until parity gates pass.

## Non-Goals For This Implementation

- Do not migrate `darkmatter::as_html` or `darkmatter::for_terminal` onto the
  tree in the initial feature.
- Do not design the full `compose/` tree-transform pipeline.
- Do not implement an MDAST JSON adapter unless a separate requirement appears.
- Do not route visual components such as `TerminalImage` or `GraphExpression`
  through the tree.
- Do not add `pulldown-cmark` or `biscuit-terminal` dependencies to
  `renderable`.

## Phase 0 - Orientation And Guardrails

### Tasks

1. Confirm package names and dependency direction with:
   - `cargo metadata --no-deps --format-version 1`
   - `cargo tree -p renderable`
   - `cargo tree -p darkmatter`
   - `cargo tree -p biscuit-terminal`
2. Record the exact `pulldown-cmark` version and enabled options used by
   darkmatter.
3. Identify the existing darkmatter parser construction path used by
   `as_html` / `for_terminal`, including `InlineStyleProcessor` and
   `RuleProcessor`.
4. Decide the module layout before writing code.

### Proposed Module Layout

In `renderable`:

- `src/tree/mod.rs`
- `src/tree/node.rs`
- `src/tree/source.rs`
- `src/tree/document.rs`
- `src/tree/attrs.rs`
- `src/tree/diagnostic.rs`
- `src/tree/error.rs`
- `src/tree/validate.rs`
- `src/tree/builders.rs`
- `src/tree/render/mod.rs`
- `src/tree/render/markdown.rs`
- `src/tree/render/browser.rs`

Compatibility modules:

- Replace `ast.rs` placeholder with a `TreeRenderable` re-export or move the
  trait to `tree/mod.rs` and re-export from `prelude.rs`.
- Keep `ast_utils.rs` as a compatibility shim only if needed; otherwise retire
  it in favor of `tree`.

In `darkmatter`:

- `lib/src/markdown/render_tree/mod.rs`
- `lib/src/markdown/render_tree/fold.rs`
- `lib/src/markdown/render_tree/inventory.rs`
- `lib/src/markdown/render_tree/source.rs`
- `lib/src/markdown/render_tree/tests.rs` or integration fixtures

In `biscuit-terminal`:

- `lib/src/render_tree/mod.rs`
- `lib/src/render_tree/options.rs`
- `lib/src/render_tree/render.rs`
- `lib/src/render_tree/component.rs`

### Validation

- `cargo check -p renderable`
- `cargo check -p darkmatter`
- `cargo check -p biscuit-terminal`

## Phase 1 - Core Tree Types In `renderable`

### Tasks

1. Add `renderable::tree` with:
   - `RenderNode`
   - `NodeKind`
   - `NodeAttrs`
   - `SourceSpan`
   - `SourceLocation`
   - `SourceId`
   - `SourceRegistry`
   - `SourceDescriptor`
   - `Document`
   - `DocumentMetadata`
   - `Frontmatter`
   - `FrontmatterFormat`
   - `Provenance`
   - `HeadingDepth`
   - `ColumnAlign`
2. Implement `Default`, `Debug`, `Clone`, `PartialEq`, `Eq` where appropriate.
3. Derive `Serialize` / `Deserialize` for the public JSON surface.
4. Implement constrained constructors:
   - `HeadingDepth::new(u8) -> Result<Self, HeadingDepthError>`
   - `ColumnAlign` enum with `Left`, `Center`, `Right`, `None`
5. Implement `RenderNode::children()` and `children_mut()`.
6. Implement builder constructors:
   - `RenderNode::root(children)`
   - `RenderNode::paragraph(children)`
   - `RenderNode::text(value)`
   - `RenderNode::heading(depth, children)`
   - `RenderNode::span(classes, children)`
   - common leaf and container helpers needed by the fold
7. Replace `AstRenderable` with:

   ```rust
   pub trait TreeRenderable {
       fn render_tree(&self) -> RenderNode;
   }
   ```

8. Export stable paths:
   - `renderable::tree::*`
   - `renderable::prelude::TreeRenderable`

### Design Decisions To Close Here

`SourceDescriptor` should be concrete enough for serialization and diagnostics.
Recommended first shape:

```rust
pub enum SourceDescriptor {
    File { path: std::path::PathBuf },
    Virtual { name: String },
    Component { name: String },
}
```

Use `SourceRegistry` as the only place paths/origins are stored. Keep
`SourceId` as a small copyable newtype around `u32` or `usize`.

### Tests

- constructors reject invalid heading depths
- builders default `span` to `Synthetic` with `location: None`
- `children()` returns expected slices for every container
- `children()` returns empty slices for every leaf
- serialization snapshots for representative nodes
- `Document` serialization includes source registry and metadata

### Validation

- `cargo test -p renderable tree`
- `cargo check -p renderable`
- `cargo doc -p renderable --no-deps`

## Phase 2 - Diagnostics, Errors, And Validation

### Tasks

1. Add shared diagnostic types:
   - `Diagnostic`
   - `DiagnosticKind`
   - `Severity`
   - optional `SourceSpan`
   - message and context fields
2. Add render result and strictness types:
   - `Rendered<T>`
   - `RenderStrictness`
   - `RenderError`
3. Start with one shared `RenderError` in `renderable`.
   - If terminal rendering needs richer variants later, add
     `biscuit-terminal` wrapper errors rather than pushing terminal-specific
     concepts into `renderable`.
4. Add validation types:
   - `ValidationMode`
   - `ValidationFinding`
   - `ValidationReport`
   - `ValidationError`
5. Implement:
   - `validate(node, ValidationMode) -> ValidationReport`
   - `ensure_valid(node) -> Result<(), ValidationError>`
6. Validation rules for Milestone 1:
   - root only at root
   - table rows only inside table
   - table cells only inside table row
   - list items only inside list
   - block nodes cannot appear inside phrasing-only containers
   - heading depth is constrained by `HeadingDepth`
   - `Unsupported` is warning severity, not structural error
7. Decide renderer validation policy in code:
   - renderers call `ensure_valid` internally
   - structural errors fail regardless of strictness
   - warnings become diagnostics and follow strictness

### Tests

- valid tree has no findings
- orphaned `TableCell` is an error
- `BlockQuote` inside `Paragraph` is an error
- `Unsupported` node is a warning
- `ensure_valid` returns `Err` only for error-severity findings

### Validation

- `cargo test -p renderable validate`
- `cargo check -p renderable`

## Phase 3 - Markdown Renderer In `renderable`

### Tasks

1. Add render options:

   ```rust
   pub struct MarkdownRenderOptions {
       pub dialect: MarkdownDialect,
       pub strictness: RenderStrictness,
       pub style: Option<MarkdownStyleOptions>,
   }
   ```

2. Add:
   - `MarkdownDialect::Markdown`
   - `MarkdownDialect::MarkdownPlus`
3. Implement:
   - `render_markdown_node`
   - `render_markdown_document`
4. Use exhaustive `match` over `NodeKind`. Do not use default-recursing render
   visitors.
5. Handle at least Milestone 1 nodes:
   - `Root`
   - `Heading`
   - `Paragraph`
   - `Text`
   - `Emphasis`
   - `Strong`
   - `Delete`
   - `Span`
   - `InlineCode`
   - `Code`
   - `Link`
   - `Image`
   - `List`
   - `ListItem`
   - `SoftBreak`
   - `HardBreak`
   - `ThematicBreak`
   - `Html`
   - `Unsupported`
6. Implement strictness behavior:
   - `Strict`: unsupported or lossy conversion returns `Err`
   - `Warn`: output plus diagnostics
   - `Lossy`: documented degrade only
7. Define MarkdownPlus behavior for `Html` and `Span`.
8. Keep formatting stable enough for semantic round-trip fixtures, not
   byte-identical source preservation.

### Tests

- node-level rendering fixtures for every implemented node
- strict mode fails on `Unsupported`
- warn mode emits diagnostics and output
- plain Markdown rejects or degrades `Html` according to strictness
- MarkdownPlus emits raw HTML where allowed
- validation errors fail before output

### Validation

- `cargo test -p renderable markdown`
- `cargo check -p renderable`

## Phase 4 - Darkmatter Parser Event Inventory

### Tasks

1. Create an inventory test or small utility that compiles against the exact
   `pulldown-cmark` 0.13 event/tag enum shapes used in the repo.
2. Verify all spec inventory entries:
   - paragraphs/headings/block quotes/code blocks
   - list/items/task markers
   - tables/table head/table rows/table cells
   - links/images
   - footnotes
   - soft/hard breaks/rules
   - HTML block/inline events
   - metadata blocks if present
   - math and definition-list support if present
3. Record the verified table in code comments or a focused markdown note next
   to the fold module.
4. Confirm which `Options` darkmatter enables in each current renderer and use
   the same options in the fold.

### Tests

- compile-time coverage by matching event/tag variants in tests
- fixture that emits each supported event category
- fixture that proves unsupported constructs become diagnostics

### Validation

- `cargo test -p darkmatter render_tree_inventory`
- `cargo check -p darkmatter`

## Phase 5 - Minimal Darkmatter Fold

### Tasks

1. Add public or crate-visible fold entry points:

   ```rust
   pub fn fold_markdown_to_document(
       source: impl Into<SourceDescriptor>,
       input: &str,
   ) -> (renderable::tree::Document, Vec<renderable::tree::Diagnostic>);
   ```

   Adjust the exact signature to match darkmatter conventions.

2. Build a `SourceRegistry` with one parsed source for the input.
3. Consume `Parser::new_ext(...).into_offset_iter()` using the same options and
   processors as the existing render paths where possible.
4. Implement stack-based folding for Milestone 1 common events.
5. Compute spans:
   - leaf nodes from event byte ranges
   - container nodes from first meaningful child through end event
   - malformed structures generate diagnostics rather than panics
6. Map:
   - `TaskListMarker` to enclosing `ListItem.checked`
   - `TableHead` to first `TableRow`
   - metadata block to raw `DocumentMetadata.frontmatter`
7. Produce `Unsupported` nodes and diagnostics for unhandled-but-known events.
8. Keep the fold side-effect free and independent of current public
   `as_html` / `for_terminal` behavior.

### Tests

- simple paragraph
- heading with generated `attrs.id`
- nested emphasis/strong/delete
- fenced code block with lang/meta
- ordered and unordered lists
- task list marker
- table with header row convention
- link and image
- soft and hard break
- raw HTML block and inline HTML
- unsupported fixture yields `Unsupported` plus diagnostic
- malformed task marker placement yields diagnostic

### Validation

- `cargo test -p darkmatter render_tree_fold`
- `cargo check -p darkmatter`

## Phase 6 - Milestone 1 Golden Round Trips

### Tasks

1. Add fixture directory, for example:
   - `darkmatter/lib/tests/fixtures/render_tree/`
2. For each fixture:
   - source Markdown
   - expected folded tree snapshot or selected structural assertions
   - expected Markdown output from `render_markdown_document`
   - expected diagnostics
3. Assert semantic stability, not byte stability.
4. Add strict/warn/lossy cases for unsupported and lossy constructs.
5. Add serialization snapshots for the full public JSON surface.

### Initial Fixture Set

- `paragraph.md`
- `headings.md`
- `inline_styles.md`
- `lists.md`
- `task_list.md`
- `code_block.md`
- `table.md`
- `links_images.md`
- `html.md`
- `unsupported_math_or_definition.md`
- `frontmatter.md`

### Validation

- `cargo test -p renderable`
- `cargo test -p darkmatter render_tree`
- `cargo check -p renderable -p darkmatter`

Milestone 1 is complete only when these tests pass and the tree API has been
exercised by real parser events.

## Phase 7 - Complete Fold Coverage

### Tasks

1. Implement every verified inventory disposition not covered in Phase 5.
2. Add footnote folding:
   - `FootnoteReference`
   - `FootnoteDefinition`
3. Add full raw HTML distinction.
4. Add custom darkmatter inline style folding:
   - `mark`
   - `dim`
   - `sup`
   - `sub`
5. Add HR-with-attributes folding into namespaced `attrs.data`.
6. Promote any `attrs.data` key to typed fields if parity tests show the data is
   load-bearing and stable.

### Tests

- one fixture per inventory row
- strict diagnostics for unsupported v1 constructs
- source span assertions for nested containers
- source registry serialization with transcluded/generated placeholders where
  feasible

### Validation

- `cargo test -p darkmatter render_tree`
- `cargo test -p renderable`

## Phase 8 - Browser Renderer In `renderable`

### Tasks

1. Add `BrowserRenderOptions`:

   ```rust
   pub struct BrowserRenderOptions {
       pub strictness: RenderStrictness,
       pub raw_html: RawHtmlPolicy,
       pub page: Option<PageOptions>,
   }
   ```

2. Add `RawHtmlPolicy`:
   - allow
   - escape
   - reject
3. Implement:
   - `render_browser_node`
   - `render_browser_document`
4. Return:
   - `BrowserFragment<Ready>` for node-level rendering
   - `HtmlPage` for document-level rendering
5. Use exhaustive `match` over `NodeKind`.
6. Map semantic classes to HTML classes.
7. Emit safe typed HTML nodes wherever possible; use raw HTML only for
   `NodeKind::Html` and documented escapes.
8. Apply `PageOptions` only in document-level page assembly or explicit full-page
   rendering paths.

### Tests

- fragment output for paragraphs/headings/lists/code/links/images/tables
- document output applies page options
- raw HTML allow/escape/reject policy
- unsupported strict/warn/lossy behavior
- table header row becomes `<thead>` or equivalent agreed structure
- diagnostics surface in rendered result

### Validation

- `cargo test -p renderable browser`
- `cargo check -p renderable`

## Phase 9 - TreeRenderable And First Component Adoption

### Tasks

1. Export `TreeRenderable` from `renderable`.
2. Remove or replace the placeholder `AstRenderable`.
3. Update docs and prelude exports.
4. Adopt one document-structural component first, preferably `BlockQuote`.
5. Keep target trait impls explicit.
6. For browser delegation, choose the infallible-trait error policy:
   - use `Warn` or `Lossy`
   - render a diagnostic fallback fragment if the renderer returns `Err`
7. Avoid broad component migration until BlockQuote proves the pattern.

### Tests

- `BlockQuote::render_tree()` structural assertions
- existing terminal/browser tests for BlockQuote still pass
- new Markdown/Browser tree renderer tests for BlockQuote

### Validation

- `cargo test -p renderable`
- `cargo test -p biscuit-terminal block_quote`
- `cargo check -p renderable -p biscuit-terminal`

## Phase 10 - Terminal Renderer In `biscuit-terminal`

### Tasks

1. Add `TerminalRenderOptions` and `TerminalRenderContext`.
2. Implement:
   - `render_terminal_node`
   - `render_terminal_document`
3. Reuse existing terminal concepts:
   - `Terminal`
   - `Layout`
   - color depth
   - hyperlink/OSC8 mode
   - wrapping
   - table rendering where practical
4. Use exhaustive `match` over `NodeKind`.
5. Map semantic classes:
   - `mark`
   - `dim`
   - `sup`
   - `sub`
   - unknown classes ignored with diagnostics only when strictness requires
6. Implement `TreeComponent<T: TreeRenderable>` adapter in `biscuit-terminal`.
7. Keep direct bespoke renderers for visual/layout-heavy components.

### Tests

- terminal render fixtures for all common document nodes
- OSC8 link behavior where supported
- code block behavior with syntax highlighting fallback
- tables match existing terminal expectations where feasible
- strict/warn/lossy diagnostics
- `TreeComponent<T>` layout accessors and rendering behavior

### Validation

- `cargo test -p biscuit-terminal render_tree`
- `cargo test -p biscuit-terminal block_quote`
- `cargo check -p biscuit-terminal`

## Phase 11 - Parity Gates Before Darkmatter Migration

### Tasks

1. Build parity fixture set against existing `darkmatter::for_terminal`.
2. Build parity fixture set against existing `darkmatter::as_html`.
3. Classify differences:
   - acceptable formatting difference
   - semantic mismatch
   - missing feature
   - bug in old renderer
   - bug in new renderer
4. Do not migrate public darkmatter render paths until semantic mismatches are
   resolved or explicitly accepted.
5. Add benchmarks:
   - large code blocks
   - large tables
   - deeply nested lists
   - many links/images
   - generated/transcluded content
   - repeated component subtrees

### Validation

- `cargo test -p darkmatter render_tree_parity`
- `cargo bench -p darkmatter`
- `cargo bench -p biscuit-terminal`

## Phase 12 - Documentation And Maintenance

### Tasks

1. Update `renderable/README.md`:
   - tree model
   - `TreeRenderable`
   - Markdown/Browser renderers
2. Update `renderable/docs/components.md` for supported targets.
3. Update `renderable/docs/dependencies.md` if dependencies change.
4. Update darkmatter docs to describe the fold as experimental/internal until
   migration.
5. Update biscuit-terminal docs for the Terminal tree renderer and
   `TreeComponent`.
6. Update `.claude/skills/renderable/SKILL.md` only after the implementation
   lands and public API paths stabilize.
7. Update package-level justfiles only if new test/bench commands are added.

### Validation

- `cargo doc -p renderable --no-deps`
- `cargo doc -p darkmatter --no-deps`
- `cargo doc -p biscuit-terminal --no-deps`

## Recommended Commit Slices

1. `feat(renderable): add render tree core types`
2. `feat(renderable): add tree validation and diagnostics`
3. `feat(renderable): add markdown tree renderer`
4. `feat(darkmatter): verify pulldown event inventory`
5. `feat(darkmatter): fold markdown events into render tree`
6. `test(darkmatter): add render tree round-trip fixtures`
7. `feat(renderable): add browser tree renderer`
8. `feat(renderable): replace AstRenderable with TreeRenderable`
9. `feat(biscuit-terminal): add terminal tree renderer`
10. `test(darkmatter): add tree renderer parity gates`

Each commit should compile on its own for the touched package set.

## Risk Controls

- Keep `renderable` free of parser and terminal dependencies.
- Prefer exhaustive `match` in renderers; do not introduce a render visitor with
  default recursion.
- Treat `Unsupported` as a visible node plus diagnostic, never silent loss.
- Run validation inside renderers so invalid trees cannot leak into output.
- Keep initial component adoption narrow until the adapter pattern is proven.
- Preserve current darkmatter public rendering until parity gates are green.

## Command Checklist

Use focused commands while developing:

```bash
cargo check -p renderable
cargo test -p renderable
cargo doc -p renderable --no-deps

cargo check -p darkmatter
cargo test -p darkmatter render_tree

cargo check -p biscuit-terminal
cargo test -p biscuit-terminal render_tree
```

Before declaring the whole feature ready:

```bash
cargo test -p renderable
cargo test -p darkmatter render_tree
cargo test -p biscuit-terminal render_tree
cargo doc -p renderable -p darkmatter -p biscuit-terminal --no-deps
```
