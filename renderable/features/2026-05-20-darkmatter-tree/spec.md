# Darkmatter Tree Rendering Migration

## Status

Ready for implementation. The supporting design notes in this directory have
been reviewed and their decisions are referenced below; they remain the
detailed source of truth for implementation-specific fixtures, API shapes, and
benchmark commands.

This spec assumes renderable Stage 3 has completed: the migrated
`biscuit-terminal` components project structurally through `TreeRenderable`,
nested component projections no longer flatten to ANSI-stripped text, and the
remaining terminal-only escape hatches are documented.

If Stage 3 review keeps any deliberately-inline component exceptions (for
example a container that must flatten block children to satisfy its own
contract), record those as component-backend caveats. They do not block this
Darkmatter parsed-Markdown fold, whose primary producer is `pulldown-cmark`,
but they matter for any future Darkmatter feature that splices component
projections into parsed documents.

## Goal

Move Darkmatter's rendering pipeline toward the render-tree architecture while
preserving current public behavior until parity and performance are proven.

The target architecture is:

```text
Markdown source
    |
    v
pulldown-cmark Event stream
    |
    v
darkmatter fold
    |
    v
renderable::tree::Document
    |
    +--> Markdown renderer
    +--> MarkdownPlus renderer
    +--> Browser renderer
    +--> Terminal renderer
```

`pulldown-cmark` remains the parser. The render tree is the owned
intermediate representation Darkmatter folds into and the shared target
renderers lower from.

## Non-Goals

- Do not replace `pulldown-cmark`.
- Do not rewrite the compose pipeline as a tree-native transform pipeline in
  this stage.
- Do not flip `Markdown::as_html` or `Markdown::as_terminal` (or the free
  function `for_terminal` that backs it) wholesale before parity and benchmark
  evidence exist.
- Do not route ordinary parsed Markdown through `TerminalRenderable`,
  `MarkdownRenderable`, or `BrowserRenderable` component trait objects.
- Do not do aggressive performance tuning yet: no arenas, string interning,
  lifetime-parametric `RenderNode`, broad `SmallVec` conversion, or public tree
  shape changes for memory reasons.

## Current-State Observations

Darkmatter's current public renderers are event-driven, but they are not
memory-minimal streaming pipelines.

The HTML path:

- preprocesses escaped inline markers into a new Markdown body string;
- parses with `pulldown-cmark`;
- wraps the parser with `InlineStyleProcessor` and `RuleProcessor`;
- builds one full output `String`;
- buffers code blocks before syntax highlighting;
- creates additional strings for highlighted code, Mermaid, images, and link
  attributes.

The terminal path:

- preprocesses escaped inline markers into a new Markdown body string;
- parses with `pulldown-cmark`;
- wraps the parser with `InlineStyleProcessor` and `RuleProcessor`;
- writes through a `LineWrapper` that owns the full output string;
- `for_terminal()` writes to a `Vec<u8>` and then converts it into a final
  `String`;
- buffers code blocks, tables, image alt text, and link text;
- clones table rows and cells before rendering through `biscuit-terminal`
  table components.

This means the tree path will add an owned document, but the baseline is not
"borrowed parser events directly to stdout." The first tree implementation may
still use more memory for simple single-target prose, but realistic Darkmatter
documents already pay several buffering and cloning costs.

## Design Principles

### Keep `pulldown-cmark` Central

The fold should consume `pulldown-cmark` events. Parser options must be treated
as a public behavior contract, not incidental implementation detail.

### Converge at `RenderNode`

Parsed Markdown and components are separate producers:

```text
Markdown source -> pulldown-cmark -> fold -> RenderNode tree
Component -> TreeRenderable::render_tree -> RenderNode tree
```

They should share the same IR and renderer backend. Parsed Markdown should not
be lowered by constructing component objects for every table, list, quote, or
section.

### Preserve Public Behavior While Proving the New Path

Tree-backed rendering should land behind explicit internal or experimental
entry points first. Public `Markdown::as_html`, `Markdown::as_terminal` (and
its underlying free function `for_terminal`), and CLI render behavior should
flip only after parity gaps are closed or deliberately accepted.

### Optimize Obvious Clones, Defer Aggressive Tuning

Avoid introducing unnecessary string clones during the migration, especially
where ownership can be moved naturally. Do not let allocation optimization
obscure the architectural migration.

## Work Items

The design prerequisites for this work live alongside this spec:

- `parser-options.md`
- `span-aware-processor-design.md`
- `entry-point-shape.md`
- `diagnostic-model.md`
- `benchmark-harness-shape.md`

Implementation should follow that order, with raw HTML policy documented before
any public Browser/HTML cutover.

### Supporting Design Coverage

The companion documents are intentionally detailed. The main implementation
contract is:

- `parser-options.md` classifies parser flags as public now
  (`ENABLE_TABLES`, `ENABLE_STRIKETHROUGH`), tree-experimental only
  (`ENABLE_TASKLISTS`, `ENABLE_FOOTNOTES`, `ENABLE_SUPERSCRIPT`,
  `ENABLE_SUBSCRIPT`), or deferred. The fold must list enabled flags
  explicitly and must not use `Options::all()` or `ENABLE_GFM` as shorthand.
- `span-aware-processor-design.md` defines the internal
  `SpannedInlineEvent` / `SpannedEventProvenance` transport, exact byte-range
  policy for split text, escaped delimiters, generated HR provenance, and
  `darkmatter.hr` hint storage for HR attributes.
- `entry-point-shape.md` chooses internal, module-level experimental
  functions under `darkmatter::markdown::render_tree`, including
  `to_render_document`, `render_tree_html`, `render_tree_terminal`, and
  `render_tree_markdown`. No new public `Markdown` methods are part of this
  stage.
- `diagnostic-model.md` defines `PipelineResult<T>` and
  `PipelineRenderResult<T>`, keeping fold diagnostics and render diagnostics
  in separate vectors.
- `benchmark-harness-shape.md` defines the Criterion bench target
  `darkmatter/lib/benches/migration_parity.rs`, the paired legacy/tree groups,
  pinned terminal/browser options, the `terminal_no_color` benchmark group,
  and the command `cargo bench -p darkmatter --bench migration_parity`.

Known experimental adapter gaps from `entry-point-shape.md` are accepted only
for the internal tree path: Mermaid lowering, HR CSS variables, and terminal
image rendering. They must be represented in parity ledgers before any public
target cutover. Browser/terminal code highlighting — including info-string
titles, line numbers, and highlighted lines — is **no longer a gap**: the
`CodeRenderer` hook carries `meta` and is wired on both surfaces (review-11),
so rich code blocks reach full parity with the legacy renderers. Terminal
styling/layout for rich code blocks is verified at Level 2 — a WezTerm pane
test renders the wired `TerminalCodeRenderer` and asserts the title, body,
line-number gutter, syntax-highlight SGRs, and a distinct highlighted-line
background survive a real terminal (`tests/level2_render_tree_terminal.rs`,
enforced under `just test-l2`; review-13).

### DMTR-1: Add Explicit Tree Rendering Entry Points

Add internal or experimental APIs that make the tree path easy to exercise
without changing stable behavior.

Candidate APIs:

```text
Markdown -> renderable::tree::Document
Markdown -> Document -> Markdown
Markdown -> Document -> MarkdownPlus
Markdown -> Document -> Browser HTML
Markdown -> Document -> Terminal
```

Requirements:

- Use the existing `darkmatter::markdown::render_tree::fold_markdown_to_document`
  as the initial fold implementation.
- Return diagnostics from fold and render phases separately enough that tests
  can identify where a mismatch originates.
- Keep public render entry points unchanged during this work item.

Acceptance criteria:

- A caller can render a `Markdown` value through the tree to each target without
  invoking the current public renderers.
- Tests cover at least a small smoke fixture for each target.

### DMTR-2: Establish Parser Option Policy

The current fold and current public renderers do not use the same
`pulldown-cmark` option set. This must become deliberate before cutover.

The full classification — public-now options, tree-experimental options, and
deferred options — lives in `parser-options.md` alongside this spec. That
document is the source of truth; this work item gates on it landing first.

Requirements:

- Define a shared rendering parse-options function or policy document.
- Classify each option currently used by the fold but not by public renderers:
  task lists, footnotes, superscript, and subscript.
- Decide whether each widened option is accepted now, deferred, or kept only on
  the experimental tree path.
- Add fixtures that make the behavior visible.
- Fix the pre-existing legacy `as_html` gap (it currently parses with only
  `ENABLE_STRIKETHROUGH`, so GFM tables are dropped in HTML) by adding
  `ENABLE_TABLES`, or record the gap as accepted in the parity ledger. See
  `parser-options.md` decision #1.

Acceptance criteria:

- The spec or tests make it impossible to silently parse the same document
  differently between legacy and tree render paths.
- Any behavior change from widening parser options is explicitly documented.

### DMTR-3: Make Darkmatter Processors Span-Aware

The fold currently depends on `Parser::new_ext(...).into_offset_iter()` to
preserve source byte ranges. Darkmatter's `InlineStyleProcessor` and
`RuleProcessor` operate on ordinary event iterators and synthesize events
without preserving ranges.

Requirements:

- Design a span-aware event wrapper for Darkmatter processor output.
- Teach `InlineStyleProcessor` to preserve or synthesize ranges for mark and
  dim segments.
- Teach `RuleProcessor` to preserve or synthesize ranges for horizontal rules
  with attribute blocks.
- Document the range policy for split text events and synthetic events.

Acceptance criteria:

- The tree fold can see Darkmatter mark, dim, and HR-attribute constructs
  without losing source provenance.
- Fixtures compare legacy and tree rendering for mark, dim, and HR attributes.

### DMTR-4: Attach Frontmatter Metadata Above the Fold

Darkmatter already owns frontmatter extraction. The fold should consume the
already extracted metadata instead of parsing frontmatter itself.

Requirements:

- Add a render-tree construction path that accepts extracted frontmatter.
- Store it in `DocumentMetadata`.
- Keep the body fold focused on Markdown event structure.

Acceptance criteria:

- A folded document can carry frontmatter metadata.
- The implementation does not duplicate Darkmatter's frontmatter parser.

### DMTR-5: Parity Fixtures for Legacy vs Tree Rendering

Build a parity suite before public cutover.

Fixture categories:

- plain prose and headings
- nested emphasis, strong, strikethrough, inline code, links
- block quotes and nested lists
- GFM tables
- code blocks with titles, line numbers, highlights, and syntax highlighting
- images with alt/title/width behavior
- Mermaid in off/text/image modes where deterministic
- mark and dim custom inline syntax
- horizontal rules with attributes
- raw HTML behavior
- parser-option-sensitive constructs: task lists, footnotes, superscript,
  subscript

Requirements:

- Compare semantics where byte parity is unrealistic.
- Classify accepted divergences in a ledger.
- Keep failure messages specific enough to identify fold vs renderer mismatch.

Acceptance criteria:

- Tree-backed Browser/HTML, Terminal, Markdown, and MarkdownPlus behavior is
  either equivalent to legacy behavior or explicitly classified.

### DMTR-6: Baseline Benchmarks and Memory Measurement

Add measurement before migration tuning decisions.

Benchmarks should compare:

- legacy `for_terminal`
- fold plus terminal renderer
- legacy `as_html`
- fold plus browser renderer
- fold once plus multiple target renders
- compose then legacy render
- compose then fold once then render multiple targets

Corpus categories:

- small prose document
- large prose document
- table-heavy document
- code-heavy document
- document with mark/dim/HR attributes
- image/Mermaid document
- transclusion-heavy composed document

Metrics:

- wall-clock time
- fold time vs render time
- output size
- peak RSS or allocator statistics when practical
- allocation count if a suitable allocator profiler is easy to wire in

Acceptance criteria:

- The first tree-backed path lands with benchmark commands and baseline
  numbers.
- No public renderer flips without a benchmark note comparing legacy and tree
  behavior.

### DMTR-7: Avoid Obvious String Cloning During Migration

This is a hygiene item, not an aggressive tuning pass.

Target now:

- avoid `to_string()` when ownership can be moved directly;
- pre-size obvious `String` or `Vec` buffers;
- avoid row/cell clones where buffered table data can be moved;
- avoid collecting `LinesWithEndings` into a `Vec<&str>` unless line count is
  required;
- avoid building a Markdown image literal just to parse it again when event
  data already carries alt, destination, and title;
- avoid repeating full-body preprocessing across compose, fold, and render
  when one prepared body can be passed down.

Defer:

- arenas;
- interning;
- `Arc<str>` conversion;
- lifetime-parametric tree nodes;
- broad `SmallVec` adoption;
- renderer rewrites whose only goal is allocation reduction.

Acceptance criteria:

- Migration patches do not add avoidable clones in new tree code.
- Any opportunistic clone reduction remains local and behavior-preserving.

### DMTR-8: Target-by-Target Cutover Plan

Do not flip all targets at once.

Recommended order:

1. Browser/HTML tree path, because structural HTML benefits most directly from
   a document tree and can preserve rich hints.
2. MarkdownPlus, because it can preserve richer behavior through inline HTML
   where portable Markdown cannot.
3. Portable Markdown, because lossiness and round-trip expectations need strict
   handling.
4. Terminal, because it has the most capability-sensitive behavior and the
   strongest performance sensitivity.

Acceptance criteria:

- Each target has its own parity ledger and benchmark note before public
  cutover.
- A target can remain legacy-backed while another target moves to tree-backed
  rendering.

### DMTR-9: Preserve Raw HTML Safety First

Current Darkmatter HTML rendering escapes raw HTML from Markdown source. The
render tree has `NodeKind::Html`, and the Browser renderer can either escape,
allow, or reject raw HTML through `RawHtmlPolicy`.

Initial migration policy:

- Browser/HTML tree rendering must default to `RawHtmlPolicy::Escape`.
- Legacy-vs-tree parity should treat escaped raw HTML as the baseline.
- Mermaid, embedded HTML widgets, and future richer browser features should not
  be enabled by globally switching to `RawHtmlPolicy::Allow`.
- Any target or feature that needs raw HTML passthrough must get a separate
  opt-in design with tests and security notes.

Acceptance criteria:

- Raw HTML fixtures exist before Browser/HTML public cutover.
- The experimental Browser/HTML tree path preserves legacy escaping by default.
- Any accepted divergence from legacy raw HTML behavior is explicitly recorded
  in the parity ledger.

## Component Relationship

Components implementing `TerminalRenderable`, `MarkdownRenderable`, and
`BrowserRenderable` are not the primary renderer for parsed Markdown.

The shared backend is:

```text
RenderNode tree -> target tree renderer
```

Components matter because they are another producer of `RenderNode`s and
because their migration has strengthened the shared renderer backend. They may
also be used by Darkmatter-generated features that naturally instantiate a
component and splice its `TreeRenderable` projection into a document.

Ordinary Markdown constructs should remain fold-produced nodes, not component
objects.

## Performance Expectations

Initial expectations:

- Simple single-target tree rendering may use more memory and wall-clock time
  than the current renderer because it builds an owned document and then walks
  it.
- The gap may be smaller than a pure streaming comparison suggests because the
  current implementation already allocates and buffers substantially.
- Multi-target rendering should be the tree path's strongest performance case:
  parse/fold once, render many targets.
- `ColorDepth::None` and similarly simple terminal cases need explicit fast
  paths or they will regress against the current early return.

Performance tuning posture:

- Add measurement now.
- Do local clone reductions when they naturally fall out of the migration.
- Defer aggressive tuning until Darkmatter's DSL and tree-backed behavior are
  stable.

## Open Questions

- What memory profiler should become the standard for this workspace?
- After internal parity is established, what exact semantic threshold is
  required before the first public Browser/HTML cutover?
- Once the tree path is public, should the non-spanned legacy processors be
  retired or should all render paths converge on the spanned chain?

## Out of Scope for This Stage

- Tree-native compose transforms.
- LSP-specific incremental folding.
- Persistent folded-document cache.
- Full aggressive allocation tuning.
- New Darkmatter DSL features not needed for render parity.

## Acceptance Criteria

- [x] Experimental tree-backed Darkmatter render entry points exist for
  Markdown, MarkdownPlus (a `MarkdownDialect` of the Markdown renderer),
  Browser, and Terminal. *(See `darkmatter::markdown::render_tree::entrypoints`.)*
- [x] Parser option policy is documented and fixture-backed. *(See
  `render_tree_parser_options` plus the strengthened
  `render_tree_parity_table` test that pins structural `<table>` semantics.)*
- [x] Mark, dim, and HR-attribute processors have a span-aware path or are
  explicitly recorded as the blocking parity gap. *(See `render_tree::span`
  and `fold_markdown_spanned_with_frontmatter`.)*
- [x] Folded documents can carry extracted frontmatter metadata. *(See
  `fold_markdown_with_frontmatter` and the
  `render_tree_frontmatter_above_the_fold_attaches_metadata` test.)*
- [x] Legacy-vs-tree parity fixtures exist across the listed fixture
  categories.
- [x] Benchmark commands and baseline results exist before any public cutover.
  *(See `darkmatter/lib/benches/migration_parity.rs`; run with `cargo bench -p
  darkmatter --bench migration_parity`.)*
- [x] New tree code avoids obvious string cloning without introducing broad
  optimization work.
- [x] Raw HTML defaults to legacy-safe escaping on the tree-backed Browser/HTML
  path unless a separate opt-in policy is approved. *(See `entrypoints::
  browser_options_from_html_options`.)*
- [x] Parsed Markdown and renderable components converge at `RenderNode`; the
  migration does not render ordinary Markdown by constructing component objects.
