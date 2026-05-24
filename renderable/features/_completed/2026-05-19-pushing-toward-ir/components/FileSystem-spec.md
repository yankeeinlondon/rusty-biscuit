# FileSystem Component — IR Migration Specification

**Component:** `FileSystem`
**Location:** `biscuit-terminal/lib/src/components/filesystem/mod.rs`
**Kind:** Block
**Current IR State:** `no changes`
**Current bt CLI:** `bespoke`

FileSystem renders directory trees with Unicode box-drawing characters (`├──`, `└──`, `│`), Nerd Font icons, gitignore-aware dimming, optional file metrics (size, tokens, timestamps, permissions), OSC8 hyperlinks, and configurable styling (italic dotfiles, highlight patterns).

## Review Notes

This specification has been reviewed against `@renderable/docs/tree-rendering.md` and `@renderable/docs/layout-and-style.md`.

The important correction is that the render tree must stay **semantic and target-agnostic**. Terminal box-drawing prefixes are presentation, not document structure. The canonical `TreeRenderable` projection should therefore preserve directory/list nesting, entry names, links, metrics, classes, and typed `Style` hints, but it should not bake terminal connector text into normal `Text` nodes except as a temporary terminal-only compatibility path.

Semantic classes such as `fs-dir` and `fs-ignored` are still useful as browser CSS hooks and debugging metadata, but generic renderers must not be required to understand arbitrary `fs-*` classes. Terminal styling should be expressed through typed `Style` attributes, and Markdown/MarkdownPlus behavior should be implemented by `FileSystem`'s Markdown adapter instead of by changing the generic Markdown renderer to special-case filesystem classes.

## Design Steps

### Terminal IR Implementation

- The **FileSystem** component does not currently have an IR-based rendering solution.
- This section describes what is required to ensure that the **FileSystem** component:
    - has a `TreeRenderable` implementation
    - can eventually drive the `TerminalRenderable` contract through the tree renderer
    - can keep `bt dir` behavior stable while the tree path is parity-gated

#### Tree Projection Design

FileSystem is a structurally unique component: it produces a **visual tree** with box-drawing connectors that have no direct analogue in the current `NodeKind` vocabulary. The existing `NodeKind` variants (`List`, `ListItem`, `Paragraph`, `Span`, `Link`, `Text`, etc.) describe document structure, not terminal connector geometry.

The canonical projection maps the FileSystem's internal `TreeNode` tree into the render tree as follows:

**Root node:** `NodeKind::Root` wraps the entire output. The root node receives `FileSystem`'s block `Layout` through `tree_layout()`.

**Root header line:** When `show_root` is true, the root directory is projected as a `Paragraph`. The paragraph contains an optional icon `Span`, followed by the root directory name as `Text` or `Link`. It carries typed `Style` equivalent to the bespoke root styling: bold emphasis plus blue foreground. It also carries classes such as `fs-root` and `fs-dir` for browser CSS hooks.

**Entry list:** Children are projected into an unordered `NodeKind::List`. The list remains structural. It must not contain pre-rendered terminal bullets unless the implementation is using the temporary compatibility mode described below.

**Each tree entry:** Each `TreeNode` is projected as a `NodeKind::ListItem`. The item receives semantic classes:

- `fs-dir` or `fs-file` — entry type
- `fs-ignored` — gitignored entry
- `fs-symlink` — symlink entry
- `fs-error` — permission-error directory
- `fs-depth-limit` — directory stopped at `max_depth`
- `fs-dot` — dotfile or dotdir when dot styling is configured
- `fs-highlight-red` / `fs-highlight-green` — pattern-based highlighting

The same visual information must also be represented as a typed `Style` on the entry's paragraph or inline spans. Do not rely on the terminal renderer to recognize `fs-*` classes. The current generic terminal renderer only has a small documented semantic class vocabulary (`mark`, `dim`, `sup`, `sub`) and otherwise consumes `Style`.

**Node payload per entry:** Each `ListItem` contains a `Paragraph` with:

1. An icon `Span` with class `fs-icon` and a target-appropriate glyph stored as text. The terminal projection may choose Nerd Font or Unicode based on the available terminal context only when rendering through a terminal adapter; the canonical tree should prefer stable Unicode fallback or semantic classes unless terminal context is explicitly available.
2. The entry name as `Text`, or as a `Link` wrapping the entry name when file links are enabled.
3. A metrics `Span` with class `fs-metrics` when metrics are present and configured for the entry.
4. Optional marker spans for symlink, error, or depth-limit annotations when needed by Markdown/MarkdownPlus.

**Directory nesting:** A directory `ListItem` contains its children as a nested `List`. This preserves the semantic tree and lets browser and Markdown renderers produce native nested-list output.

**Metrics:** Metrics are computed during tree projection with the same formatting rules as the bespoke renderer. Styling within metrics must use typed `Style` on `Span` nodes. Plain Markdown will drop that styling; MarkdownPlus can preserve classes or inline HTML via the component's Markdown adapter.

#### Terminal Connector Strategy

The terminal output needs box-drawing connectors. The renderer can produce them cleanly only if the render tree supports a typed list marker policy. Until that feature is implemented, do **not** flip `FileSystem::render(&Terminal)` to the generic tree renderer.

Accepted migration sequence:

1. Add `TreeRenderable` for FileSystem using structural `List`/`ListItem` nodes and typed `Style`.
2. Keep the existing bespoke `TerminalRenderable` implementation as the production path.
3. Add tree-rendered terminal tests behind an explicit helper/adapter only after the approved list marker policy exists.
4. Flip the production terminal path only after parity tests prove connector geometry, icons, truncation, OSC8 links, metrics, ANSI styling, and layout behavior match the bespoke renderer on semantic invariants.

Temporary compatibility mode is allowed only for tests or transitional adapters: the component may render one preformatted line per entry into `Text` nodes when comparing terminal behavior before the list marker policy lands. That mode must be marked as lossy/terminal-specific and must not become the canonical projection used for Markdown or Browser.

#### Layout and Style Mapping

**Layout:**

- `FileSystem` already owns a `Layout` with margins, alignment, and word-wrap. This maps directly to `tree_layout()` on the root `RenderNode`.
- Tree connectors must never be wrapped. The FileSystem tree projection should force `WordWrap::None` for the root tree block, matching the bespoke renderer's connector-preserving behavior.
- `max_width` is currently not applied by the terminal renderer, per `layout-and-style.md`. Do not rely on it for terminal truncation. Name truncation remains a FileSystem responsibility because it depends on connector and icon visible widths.

**Style:**

- Entry styling must be lowered into typed `Style` attributes, not only semantic classes.
- Current styling precedence must be preserved:
    - highlight red / highlight green have highest priority
    - error directories are red
    - ignored entries are dim when `dim_gitignore` is true
    - configured dotfiles/dotdirs are italic
    - directories are bold blue unless overridden by error/highlight behavior
    - symlinks are cyan
- The root header line carries bold blue `Style`.
- Metrics labels use dim emphasis; threshold-highlighted metric values use bold yellow.
- Browser CSS classes are additive hooks and should not be treated as the source of terminal truth.

#### Feature Requests for Tree Rendering

##### Feature 1: Custom List Marker Policy

**APPROVED**

this feature request has been approved and WILL be included as part of the render-tree implementation BEFORE you are asked to implement this solution. Always refer to the @renderable/docs/tree-rendering.md and @renderable/docs/layout-and-style.md documents as the definitive guide.

**Improved request:** Add a typed, target-agnostic list marker policy to the render tree. The smallest acceptable design is a typed `ListMarkerPolicy` hint on `NodeKind::List` with at least:

- `Default` — current ordered/unordered renderer behavior
- `None` — render list item bodies without renderer-inserted bullets or numbers
- `TreeConnectors` — terminal renderer may render filesystem-style connector prefixes from nested `List`/`ListItem` structure; Browser and Markdown may degrade to native nested lists or no-marker lists according to strictness and dialect

The policy must be represented through a typed helper instead of ad hoc JSON string reads at call sites. The normal list behavior must remain unchanged when no policy is present.

**Why this is approved:** FileSystem's terminal contract depends on connector geometry that is neither a normal Markdown bullet nor an ordered-list marker. Suppressing or replacing the default marker is a general render-tree concern, not a FileSystem-only hack: it applies to any component that has semantic list nesting but needs target-specific marker presentation. This keeps connector rendering in the renderer where width, layout, and terminal capability information are available.

Required behavior:

- Validation must reject invalid marker policies on non-list nodes.
- Terminal rendering must preserve child order and infer connector geometry from nested list structure.
- Markdown rendering must keep valid Markdown output. In plain Markdown, `TreeConnectors` may degrade to normal nested `- ` lists with a diagnostic in `Warn` mode; `None` may render item bodies without bullets only when doing so does not create invalid block structure.
- MarkdownPlus and Browser may use classes/CSS (`list-style: none`, connector pseudo-elements, or nested-list styling) without embedding terminal box-drawing text by default.
- Tests must cover default list behavior unchanged, no-marker lists, tree-connector lists, nested lists, single-child lists, and strict/warn/lossy behavior for targets that cannot faithfully represent a marker policy.

##### Feature 2: Per-Item Indentation Depth Control

**DENIED**

this feature will not be added to the render-tree tree implementation. You should try to still use the render-tree where practical and work around the complexity but if the complexity is too great then you have permission to create a bespoke IR implementation for this component.

**Why this is denied:** Explicit per-item depth hints duplicate information already present in nested `List`/`ListItem` structure and can become inconsistent with the actual tree. The terminal renderer can infer depth and "last child" state while walking the list. If connector rendering needs ancestor continuation state, that state belongs in the renderer traversal stack, not in serialized per-item hints.

The approved list marker policy above is the right place to request connector-style rendering. It keeps the canonical tree normalized and avoids invalid states such as a depth-3 item physically nested at depth 1.

#### Recommendation

The current tree renderer is **not yet a natural production replacement** for FileSystem's terminal renderer. FileSystem should add a semantic `TreeRenderable` projection now, but the production `TerminalRenderable` path should remain bespoke until the approved list marker policy is implemented and parity-gated.

- `will_use_tree_renderer`: **false** for production terminal rendering today
- `will_use_tree_renderer_with_features`: **true** after the approved list marker policy lands and parity tests pass
- `canonical_tree_projection`: **true** now, as structural `Root`/`List`/`ListItem`/`Paragraph`/`Span`/`Link` nodes with typed `Style`
- `temporary_preformatted_terminal_projection`: **allowed only as a transitional test aid**, not as the canonical IR

#### Critical Test Variants

1. **Basic tree rendering** — directory with files and subdirectories, verifying connector characters, icon placement, and name display in terminal output once list marker policy exists.
2. **Single-entry directory** — one child uses `└──`, not `├──`.
3. **Deeply nested tree** — 3+ levels verify vertical continuation lines and indentation.
4. **Empty directory** — no entries; root line appears only when `show_root` is true.
5. **Dotfiles and dotdirs** — italic styling when configured and hiding when configured.
6. **Gitignored entries** — dim styling when `dim_gitignore` is true; skip recursion when configured.
7. **Symlinks** — cyan styling and no following children.
8. **Error directories** — red styling and error/depth indicators.
9. **Depth limit** — directories at `max_depth` show depth-limit state and no children.
10. **Filter patterns** — only matching entries appear; directories with matching descendants are retained.
11. **Highlight patterns** — highlight colors take precedence over directory, symlink, dotfile, and ignored styling.
12. **File metrics** — size, tokens, modified, updated, and permissions formatting and placement.
13. **Metric highlight threshold** — threshold values use bold yellow styling.
14. **OSC8 file links** — terminal links target absolute file paths when enabled.
15. **Name truncation** — long filenames truncate with ellipsis without breaking connector or icon columns.
16. **No root line** — `show_root(false)` omits the root directory header.
17. **Parity with bespoke renderer** — terminal tree path vs bespoke path compare stripped-ANSI content and connector geometry after the approved marker policy lands.
18. **Layout margins** — root `Layout` margins and alignment offset the whole tree block consistently.
19. **Nerd Font vs Unicode icons** — terminal-aware icon selection uses Nerd Font only when capability indicates support; fallback remains stable.
20. **Style projection tests** — inspect the render tree directly to verify `Style` and classes are placed on the intended nodes.
21. **Validation tests** — invalid marker-policy placement is rejected once the approved render-tree feature is implemented.

### Browser IR Implementation

FileSystem has no existing browser rendering implementation. The browser target should render the directory tree as a nested HTML list (`<ul>`/`<li>`) with CSS styling.

#### Browser Rendering Design

The canonical `TreeRenderable` projection maps naturally to the existing browser tree renderer:

- `List` renders as `<ul>`
- `ListItem` renders as `<li>`
- `Span` classes become `class` attributes
- `Link` nodes render as anchors
- root `Layout` becomes inline layout CSS through the existing layout lowering

The browser tree renderer does **not** currently lower `Style` to CSS. Therefore FileSystem's `BrowserRenderable` implementation should wrap the projected tree through `BrowserTreeComponent<FileSystem>` and provide a component stylesheet or direct wrapper classes for the filesystem-specific visual treatment. The CSS classes are the browser contract; typed `Style` remains the terminal contract until browser style lowering is implemented.

**Semantic class → CSS mapping:**

| Class | CSS |
|-------|-----|
| `fs-root` | `font-weight: 700; color: #2563eb;` |
| `fs-dir` | `font-weight: 700; color: #2563eb;` |
| `fs-file` | default text |
| `fs-ignored` | `opacity: 0.55;` |
| `fs-symlink` | `color: #0891b2;` |
| `fs-error` | `color: #dc2626;` |
| `fs-depth-limit` | `color: #ca8a04;` |
| `fs-dot` | `font-style: italic;` |
| `fs-highlight-red` | `color: #dc2626;` |
| `fs-highlight-green` | `color: #16a34a;` |
| `fs-icon` | `display: inline-block; margin-right: 0.25em; width: 1.25em;` |
| `fs-metrics` | `color: #6b7280; font-size: 0.85em; margin-left: 0.5em;` |

**Tree connectors:** Browser rendering should use nested-list CSS, not terminal box-drawing text. The approved `TreeConnectors` marker policy may be represented with CSS classes and pseudo-elements later, but the baseline browser output should remain readable nested HTML.

**Icons:** Nerd Font PUA glyphs are not portable in browsers. Browser output should prefer Unicode fallback glyphs or CSS/SVG icons selected from semantic classes. The exact SVG icon set is an implementation detail, but the tree projection must expose enough class metadata to make that substitution possible.

**File links:** File links render as `<a href="file:///absolute/path">name</a>` or another explicit URL policy chosen by implementation. Raw local paths should not be emitted as ambiguous relative web URLs.

**Root header:** The root paragraph renders as a styled root line before the list.

#### Key Test Variants

1. **Basic nested list** — verifies correct `<ul>`/`<li>` nesting.
2. **Semantic classes** — verifies each entry type class appears on the expected node.
3. **Component CSS** — verifies filesystem CSS classes or stylesheet hooks are emitted.
4. **File links** — verifies anchor tags and URL policy.
5. **Metrics** — verifies metric spans and classes.
6. **Empty directory** — verifies empty output/root-only behavior.
7. **Deep nesting** — verifies 3+ nested `<ul>` levels.
8. **Icons** — verifies portable icon output or semantic icon classes.
9. **Layout** — verifies root layout CSS is present when margins/alignment are configured.
10. **No terminal connectors** — verifies browser output does not contain `├──`, `└──`, or `│` unless an explicit future connector CSS feature chooses to display them.

### Markdown IR Implementation

FileSystem has no existing Markdown rendering implementation. Markdown and MarkdownPlus outputs should render the directory tree as nested Markdown lists.

#### Markdown vs MarkdownPlus Divergence

Both targets produce a nested Markdown list representing the directory tree.

1. **Styling/Color:**
   - **Markdown:** Drop color and most styling. Directories may be rendered as bold (`**src/**`) by FileSystem's Markdown adapter because this is component-specific semantic presentation.
   - **MarkdownPlus:** Preserve classes with inline HTML spans where useful, for example `<span class="fs-dir">src/</span>`. Prefer classes over inline style declarations so the HTML/CSS contract matches browser output.

2. **Icons:**
   - **Markdown:** Omit Nerd Font PUA glyphs and terminal icons.
   - **MarkdownPlus:** Use portable Unicode icons or classed spans if icons add value.

3. **Tree connectors:**
   - **Both:** Use native nested Markdown list syntax. Do not use terminal box-drawing characters as the primary Markdown representation.

4. **File links:**
   - **Both:** Use native Markdown links with an explicit URL policy, e.g. `[name](file:///path/to/name)`.

5. **Metrics:**
   - **Both:** Metrics appear as parenthetical text after the entry name.

#### Markdown Rendering Design

There is no generic `MarkdownTreeComponent` adapter today. `FileSystem` should implement `MarkdownRenderable` directly and may internally call `render_markdown_node` / `render_markdown_document` for the structural tree where that output is sufficient.

Do not modify the generic Markdown renderer to special-case `fs-dir` or other FileSystem classes. The generic Markdown renderer's current behavior is:

- plain Markdown degrades classed spans to their inner text according to strictness
- MarkdownPlus renders classed spans as `<span class="...">...</span>`
- `Style` is ignored

Therefore any FileSystem-specific bold directory names, symlink annotations, error annotations, icon omissions, and metric formatting belong in FileSystem's Markdown adapter or in the canonical tree content itself.

#### Testing Strategy

1. **Basic nested list** — verifies nested `- ` structure.
2. **Directory bold** — verifies directory names are bold in plain Markdown if that policy is implemented.
3. **Styling stripped in Markdown** — verifies no inline HTML or color styling in plain Markdown.
4. **Classes preserved in MarkdownPlus** — verifies classed `<span>` output for entries/metrics where used.
5. **File links** — verifies `[name](file:///path)` syntax.
6. **Metrics** — verifies plain-text parenthetical metrics.
7. **Icons omitted in Markdown** — verifies no PUA glyphs or emoji in plain Markdown.
8. **Icons portable in MarkdownPlus** — verifies Unicode or classed icon strategy.
9. **Symlink indicator** — verifies symlink entries are annotated.
10. **Error entries** — verifies error directories are annotated.
11. **Depth-limit entries** — verifies depth-limited directories are annotated.
12. **Filter patterns** — verifies filtered output.
13. **Empty directory** — verifies no list items for an empty directory.
14. **No terminal connectors** — verifies Markdown output does not contain terminal connector glyphs.
15. **Strictness behavior** — verifies class/style loss is intentional in plain Markdown and does not fail unexpectedly under the chosen adapter policy.

### `bt` CLI

- This specification will ensure that the **FileSystem** component:
    - keeps the existing `bt dir` subcommand
    - adds `--md`, `--md-plus`, and `--html` target switches
    - keeps terminal rendering as the default
    - keeps `--example` functional

#### Current State

The `bt dir` CLI command exists at `biscuit-terminal/cli/src/commands/dir.rs`.

| Aspect | Current State |
|--------|---------------|
| CLI command exists | Yes — `bt dir [PATH]` |
| Render method | Bespoke — calls `fs.render(&term)` directly via `TerminalRenderable` |
| `--md` switch | No |
| `--md-plus` switch | No |
| `--html` switch | No |
| `--example` switch | Yes — `bt dir --example` renders `bt dir . --depth 1 --filter ".rs"` |

The existing `DirArgs` struct has:

- `path` (positional, default `.`)
- `--example` / `-e`
- `--depth` / `-d`
- `--filter` / `-f`
- `--skip-root`
- `--size`, `--tokens`, `--modified`, `--updated`
- `--margin-left`, `--margin-right`, `--alignment` through `LayoutArgs`

#### Specification for bt CLI Completion

1. **Add `--md`, `--md-plus`, and `--html` switches** to `DirArgs`, following the same `conflicts_with_all` pattern used by `bt prose`:

   ```rust
   #[arg(long, conflicts_with_all = ["md", "md_plus"])]
   pub html: bool,

   #[arg(long, conflicts_with_all = ["html", "md_plus"])]
   pub md: bool,

   #[arg(long = "md-plus", conflicts_with_all = ["html", "md"])]
   pub md_plus: bool,
   ```

2. **When `--md` or `--md-plus` is set:** The command calls `FileSystem`'s `MarkdownRenderable::render_markdown()` or `render_markdown_plus()`. Output is printed to STDOUT. If layout frontmatter is used, follow the pattern already used by `bt prose`.

3. **When `--html` is set:** The command calls `FileSystem`'s `BrowserRenderable::render_html_fragment()` or a `BrowserTreeComponent<FileSystem>` wrapper and wraps it with the same layout-aware HTML helper pattern used by `bt prose`.

4. **Default (no target switch):** The command keeps calling the terminal renderer. This remains the bespoke `fs.render(&term)` path until the approved list marker policy is implemented and FileSystem terminal parity tests pass.

5. **`--example` remains terminal-focused**. The primary example remains `bt dir . --depth 1 --filter ".rs"`. The example output may add secondary lines showing `--md` and `--html`, but must not change the existing successful terminal example.

6. **Import updates:** `DirArgs::run()` must import `BrowserRenderable` and `MarkdownRenderable` when target switches are added, following the import pattern in `prose.rs`.

7. **Tests:** Add CLI tests for target-switch conflicts, terminal default behavior, `--md`, `--md-plus`, `--html`, and `--example`.

## Acceptance Criteria Summary

- `FileSystem` implements `TreeRenderable` with a semantic `Root`/`List`/`ListItem` projection.
- The tree projection contains typed `Style` where terminal styling is required and classes where browser/MarkdownPlus hooks are useful.
- Existing `FileSystem` `TerminalRenderable` behavior is retained until the approved list marker policy exists and parity tests pass.
- `FileSystem` implements `BrowserRenderable`, either directly or through `BrowserTreeComponent<FileSystem>` plus component CSS.
- `FileSystem` implements `MarkdownRenderable` directly, using the tree where practical but keeping FileSystem-specific Markdown policy out of the generic Markdown renderer.
- `bt dir` gains `--md`, `--md-plus`, and `--html` switches.
- `bt dir --example` continues to work with terminal rendering.
- Parity tests compare bespoke vs tree terminal output across all critical variants before any production terminal flip.
- Browser and Markdown rendering tests cover the listed key variants.
- Approved render-tree functionality is tracked in `@renderable/features/2026-05-19-pushing-toward-ir/approved-render-tree-functionality.md`.
