# FileSystem Component — IR Migration Specification

**Component:** `FileSystem`
**Location:** `biscuit-terminal/lib/src/components/filesystem/mod.rs`
**Kind:** Block
**Current IR State:** `no changes`
**Current bt CLI:** `bespoke`

FileSystem renders directory trees with Unicode box-drawing characters (`├──`, `└──`, `│`), Nerd Font icons, gitignore-aware dimming, optional file metrics (size, tokens, timestamps, permissions), OSC8 hyperlinks, and configurable styling (italic dotfiles, highlight patterns).

## Design Steps

### Terminal IR Implementation

- The **FileSystem** component does not currently have an IR-based rendering solution.
- This section will describe what is required to ensure that the **FileSystem** component:
    - has an IR implementation
    - the IR implementation drives the TerminalRenderable contract
    - the IR implementation is what is used by the bt CLI (note: **FileSystem** does already have a `bt dir` subcommand but it uses bespoke rendering — the bt CLI section below will cover the migration)

#### Tree Projection Design

FileSystem is a structurally unique component: it produces a **visual tree** with box-drawing connectors that have no direct analogue in the `NodeKind` vocabulary. The existing `NodeKind` variants — `List`, `ListItem`, `Code`, `BlockQuote`, `Table`, etc. — describe document-structural semantics, not visual-tree layout.

The recommended projection maps the FileSystem's internal `TreeNode` tree into the render-tree as follows:

**Root node:** `NodeKind::Root` — wraps the entire output.

**Root header line:** When `show_root` is true, the root directory name (e.g., ` my-project`) is projected as a `NodeKind::Paragraph` containing `Text` with the directory name. The terminal tree renderer must be able to apply bold-blue styling to this line; this is expressed through a `Style` on the paragraph node (bold emphasis, blue color).

**Each tree entry:** Each `TreeNode` (file or directory) is projected as a `NodeKind::ListItem` within an unordered `NodeKind::List`. The list is purely structural — it provides nesting semantics that the terminal tree renderer can walk. The visual prefix (`├── `, `└── `, `│   `, `    `) and icons are terminal-renderer concerns, not tree-node concerns.

**Node payload per entry:** Each `ListItem` contains a `Paragraph` with:
1. An inline `Image` node carrying the resolved icon character (or a `Text` for the icon glyph) — alternatively, the icon can be a `Text` node with a semantic class (e.g., `class="fs-icon"`) so the terminal renderer substitutes the appropriate Nerd Font or Unicode icon based on capability.
2. A `Text` node for the entry name.
3. If file metrics are present, a `Span` with class `fs-metrics` containing the formatted metrics text.
4. If file links are enabled, the filename is wrapped in a `Link` node with the file's absolute path as the URL.

**Semantic classes:** Each `ListItem` receives semantic classes to communicate entry metadata:
- `fs-dir` or `fs-file` — entry type
- `fs-ignored` — gitignored entry (enables dim styling)
- `fs-symlink` — symlink entry (enables cyan styling)
- `fs-error` — permission-error directory (enables red styling)
- `fs-dot` — dotfile/dotdir (enables italic when configured)
- `fs-highlight-red` / `fs-highlight-green` — pattern-based highlighting

These classes are the bridge between the component's rich styling logic and the terminal tree renderer's `Style` application. The terminal tree renderer, when encountering a `ListItem` with these classes, applies the corresponding ANSI styling exactly as the bespoke renderer does today.

**Directory nesting:** A directory `ListItem` contains its children as a nested `List`. This naturally produces the indentation semantics.

**Metrics:** Metrics are attached as additional `Text` content within the `ListItem`'s `Paragraph`, formatted identically to the bespoke renderer's output. The metrics formatting (dim labels, highlight thresholds) uses the same functions and is computed during tree projection.

#### Layout and Style Mapping

**Layout:**
- `FileSystem` already owns a `Layout` with margins, alignment, and word-wrap. This maps directly to `tree_layout()` on the root `RenderNode`. Tree connectors (`├──`, `│`) are never wrapped, so `word_wrap` should be `None` at the tree level — consistent with the bespoke renderer which does not apply word-wrap to tree lines.
- `max_width` is not used by the terminal renderer (per layout-and-style.md §4) and can be left as `None`.

**Style:**
- The bespoke renderer applies per-entry styling (bold blue for dirs, cyan for symlinks, red for errors, dim for ignored, italic for dotfiles, color for highlights). These map to per-`ListItem` `Style` attributes in the IR, keyed by the semantic classes described above.
- The root header line carries a `Style` with bold emphasis and blue color.
- Metrics labels use dim emphasis; highlight-threshold values use bold yellow.

#### Feature Requests for Tree Rendering

##### Feature 1: Custom List Bullet/Prefix Rendering

**What it looks like:** A new hint on `NodeKind::List` or `ListItem` that tells the terminal renderer "do not emit default bullet/number prefixes for this list; the component will supply its own line prefixes." This could be a `NodeAttrs` hint like `list-hints.render-bullets: false` or a dedicated `ListRenderHints` field.

```rust
// Usage in FileSystem tree projection:
let mut list_node = RenderNode::list(false, None, children);
list_node.attrs.set_hint(
    HintNamespace::List,
    "render-bullets",
    serde_json::Value::Bool(false),
);
```

**Why FileSystem needs it:** The box-drawing tree connectors (`├── `, `└── `, `│   `, `    `) are the visual heart of the FileSystem component. The default terminal list renderer emits `- ` or `1. ` prefixes. Without this feature, the tree renderer would need to be told to suppress its own prefix so the FileSystem can inject box-drawing characters instead — but there is no existing mechanism for this.

**Without this feature:** The FileSystem component would need to embed the full visual prefix (`├── ` + icon) directly into each `Text` node. This defeats the purpose of the tree renderer's list handling and essentially makes the tree renderer a passthrough. The component would still use `List`/`ListItem` for nesting but each item's text would already contain the fully-formatted line — a degenerate case where the IR adds overhead without abstraction value.

##### Feature 2: Per-Item Indentation Depth Control

**What it looks like:** A `ListItem` hint that specifies the visual indentation depth, allowing the terminal renderer to calculate the correct box-drawing prefix (`│   ` vs `    ` at each ancestor level) from structural nesting alone.

```rust
// Each ListItem at depth N knows its prefix components:
// - For each ancestor that is NOT the last child: "│   "
// - For each ancestor that IS the last child: "    "
// The terminal renderer builds: prefix + ("├── " or "└── ") + icon + name
```

**Why FileSystem needs it:** The box-drawing prefix depends on whether each ancestor was the last child in its sibling list. This is structural information available during tree projection but not naturally expressed in the `List`/`ListItem` nesting. Without this feature, the FileSystem must either (a) embed the full prefix as text content (same degenerate case as Feature 1), or (b) rely on the terminal renderer to infer prefixes from the tree structure.

**Without this feature:** Same degenerate case as Feature 1 — the component embeds fully-formatted lines in text nodes and the tree renderer becomes a passthrough.

#### Recommendation

The current tree renderer is **not a natural fit** for the FileSystem component. The box-drawing tree layout is a domain-specific visual structure that does not map cleanly onto the `List`/`ListItem` document-structural model without at least Feature 1 (custom prefix suppression). Feature 2 (per-item indentation) would make the mapping truly clean and composable.

Without either feature, the FileSystem would embed fully-formatted lines in `Text` nodes wrapped in `List`/`ListItem` shells — adding IR overhead without meaningful abstraction. The tree renderer would not be applying its own list layout logic; it would simply be concatenating pre-formatted text.

**However**, with Feature 1 alone, the mapping becomes viable: the tree renderer delegates prefix rendering to the component, and the component uses its internal `TreeNode` structure to compute the correct box-drawing prefix during projection. The tree renderer still handles layout (margins, alignment), style application (from semantic classes to ANSI), and the overall block-level rendering contract.

With both features, the mapping becomes genuinely clean: the component supplies nesting structure and metadata, and the terminal renderer produces the visual output from that structure.

- `will_use_tree_renderer`: **false** — without at least Feature 1, the tree renderer adds no value over the bespoke renderer for this component.
- `will_use_tree_renderer_with_features`: **true** — with Feature 1 (and ideally Feature 2), the tree renderer provides real value: layout application, style lowering, and cross-target rendering from a single IR.

#### Critical Test Variants

1. **Basic tree rendering** — a directory with a mix of files and subdirectories, verifying connector characters (`├──`, `└──`, `│`), icon placement, and name display.
2. **Single-entry directory** — only one child, verifying `└──` is used (not `├──`).
3. **Deeply nested tree** — 3+ levels of nesting, verifying vertical continuation lines and indentation accuracy at each level.
4. **Empty directory** — a directory with no entries, verifying no output (or just the root line if `show_root` is true).
5. **Dotfiles and dotdirs** — files and directories starting with `.`, verifying italic styling when configured, and hiding when configured.
6. **Gitignored entries** — entries marked as ignored, verifying dim styling when `dim_gitignore` is true.
7. **Symlinks** — symlink entries, verifying cyan styling and that children are not followed.
8. **Error directories** — permission-denied directories, verifying red styling and error icon.
9. **Depth limit** — directories at `max_depth`, verifying depth-limit icon and no children.
10. **Filter patterns** — filter patterns that match some entries but not others, verifying only matching entries appear and non-matching directories are included only when they have matching descendants.
11. **Highlight patterns** — `highlight_red` and `highlight_green` patterns, verifying color application takes priority over other styling.
12. **File metrics** — `--size`, `--tokens`, `--modified`, etc., verifying metric formatting and placement.
13. **Metric highlight threshold** — values exceeding thresholds, verifying bold-yellow highlighting.
14. **OSC8 file links** — `file_links` enabled, verifying filenames become clickable hyperlinks.
15. **Name truncation** — long filenames exceeding terminal width, verifying truncation with ellipsis.
16. **No root line** — `show_root(false)`, verifying the root directory header is omitted.
17. **Parity with bespoke renderer** — every variant above rendered both ways (bespoke vs tree), comparing stripped-ANSI output for content equivalence.
18. **Layout margins** — left/right margins, verifying the tree block is offset correctly.
19. **Nerd Font vs Unicode icons** — rendering with and without Nerd Font support, verifying correct icon selection.

### Browser IR Implementation

- In this section we will provide a design specification for the **FileSystem** component's implementation of the BrowserRenderable trait.

The FileSystem component has no existing browser rendering implementation. The browser target will render the directory tree as an HTML nested list (`<ul>`/`<li>`) with CSS styling.

#### Browser Rendering Design

The IR already produced for the terminal target (via `TreeRenderable`) maps naturally to the browser through the existing `render_browser_node` renderer. The projection structure — `Root` → `List`/`ListItem` tree — is consumed by the browser renderer to produce:

- A `<ul>` for each `List` node
- A `<li>` for each `ListItem` node
- Text content, links, and styling from the node's children

**Semantic class → CSS mapping:**

| Class | CSS |
|-------|-----|
| `fs-dir` | `font-weight: bold; color: #5c94fc;` |
| `fs-file` | (default text) |
| `fs-ignored` | `opacity: 0.5;` |
| `fs-symlink` | `color: #56d4dd;` |
| `fs-error` | `color: #ff5f5f;` |
| `fs-dot` | `font-style: italic;` |
| `fs-highlight-red` | `color: #ff5f5f;` |
| `fs-highlight-green` | `color: #5fff5f;` |
| `fs-icon` | `margin-right: 0.25em;` |
| `fs-metrics` | `color: #888; font-size: 0.85em; margin-left: 0.5em;` |

**Tree connectors:** The browser rendering replaces box-drawing characters with CSS-based tree indentation. Each nested `<ul>` receives `padding-left` and a left border (`border-left: 1px solid #444`) to produce a visual tree similar to GitHub's file browser. This is a deliberate divergence from the terminal's `├──`/`└──` connectors, which are terminal-specific and do not translate to HTML.

**Icons:** Nerd Font icons are Private Use Area glyphs that may not render in a browser. The browser target substitutes Nerd Font icons with either:
- Inline SVG icons (preferred for fidelity)
- Unicode emoji fallbacks (`📂`, `📄`)
- Or omits icons entirely and uses CSS-based visual differentiation (folder color, font weight)

The exact icon strategy is deferred to implementation; the tree projection stores semantic class information (e.g., `fs-icon-rust`, `fs-icon-dir-git`) so the browser renderer can make the substitution.

**File links:** The `Link` nodes in the tree projection naturally render as `<a href="...">` in the browser, so file hyperlinks work automatically.

**Root header:** The root paragraph renders as a styled heading or `<div>` with the directory name.

#### Key Test Variants

1. **Basic nested list** — verifying correct `<ul>`/`<li>` nesting for a multi-level directory tree.
2. **Semantic classes** — verifying each entry type (dir, file, symlink, ignored, error, dot, highlighted) maps to the expected CSS classes.
3. **File links** — verifying `<a href="...">` tags are present when `file_links` is enabled.
4. **Metrics** — verifying metric text is present in the output with appropriate styling classes.
5. **Empty directory** — verifying empty `<ul>` or no list items.
6. **Deep nesting** — verifying 3+ levels of nested `<ul>` with correct indentation styles.
7. **Icons** — verifying icon nodes are present (content TBD based on icon strategy).

### Markdown IR Implementation

The FileSystem component has no existing Markdown rendering implementation. The Markdown and MarkdownPlus outputs render the directory tree as a nested Markdown list.

#### Markdown vs MarkdownPlus Divergence

Both targets produce a nested Markdown list representing the directory tree. The key divergence points:

1. **Styling/Color:** The terminal applies ANSI styling (bold blue dirs, cyan symlinks, red errors, dim ignored, italic dotfiles). Markdown has no native syntax for these colors.
   - **Markdown:** All styling is dropped. Directories are rendered as bold (`**src/**`) to distinguish them from files. No color information is preserved.
   - **MarkdownPlus:** Inline HTML `<span>` elements preserve color/italic/dim styling where it adds value: `<span style="color:blue;font-weight:bold">src/</span>`, `<span style="opacity:0.5">target/</span>`, `<span style="font-style:italic">.gitignore</span>`. Highlight patterns use inline color spans.

2. **Icons:** Nerd Font PUA glyphs are meaningless in Markdown.
   - **Markdown:** Icons are omitted entirely.
   - **MarkdownPlus:** Unicode emoji icons (`📂`, `📄`, `⚠`) may be included since they are valid Unicode that most Markdown renderers handle.

3. **Tree connectors:** Box-drawing characters (`├──`, `└──`) are Unicode and valid in Markdown. However, they do not produce a native Markdown list structure.
   - **Both:** Use native Markdown nested list syntax (unordered, with `- ` bullets). The visual tree is conveyed through indentation levels, not box-drawing characters. This is more idiomatic for Markdown consumers.
   - A code-block fallback is possible (````tree ... ````) but would lose hyperlink/metric information and is not recommended as the primary output.

4. **File links:**
   - **Markdown:** Native Markdown links: `[name](file:///path/to/name)`.
   - **MarkdownPlus:** Same — no divergence for links.

5. **Metrics:**
   - **Both:** Metrics appear as parenthetical text after the entry name: `- src/ (file size: 1.2 KB)`. Identical in both targets since metrics are plain text.

#### Markdown Rendering Design

The `render_markdown_node` renderer consumes the same `List`/`ListItem` tree produced by `TreeRenderable`. The existing Markdown list rendering already handles nested `List` → `ListItem` → content, so the FileSystem projection maps naturally.

The projection stores entry metadata as semantic classes. The Markdown renderer:
- Ignores all `fs-*` styling classes (consistent with layout-and-style.md §4: "Markdown deliberately ignores Style entirely")
- Uses the `Text` content of each `ListItem` for the bullet text
- Maps `Link` nodes to `[text](url)` syntax

For Markdown bold directory names: the `fs-dir` class is checked during Markdown rendering. When present, the entry name is wrapped in `**...**`. This is a lightweight semantic-to-presentation mapping that does not require inline HTML.

For MarkdownPlus: the `fs-*` classes drive inline `<span>` elements with `style` attributes as described in the Browser section's CSS table.

#### Testing Strategy

1. **Basic nested list** — verifying correct Markdown nested `- ` list structure.
2. **Directory bold** — verifying directory names are wrapped in `**...**` in Markdown output.
3. **Styling stripped in Markdown** — verifying no inline HTML or color styling in Markdown output.
4. **Styling preserved in MarkdownPlus** — verifying inline `<span>` elements with appropriate `style` attributes.
5. **File links** — verifying `[name](file:///path)` syntax in both targets.
6. **Metrics** — verifying plain-text metrics in parenthetical form.
7. **Icons omitted in Markdown** — verifying no PUA glyphs or emoji in Markdown output.
8. **Icons present in MarkdownPlus** — verifying Unicode emoji icons appear.
9. **Symlink indicator** — verifying symlink entries are noted (e.g., `name@` or `(symlink)` annotation).
10. **Error entries** — verifying error directories are annotated (e.g., `(error)` or `⚠` in MarkdownPlus).
11. **Filter patterns** — verifying filtered output includes only matching entries.
12. **Empty directory** — verifying no list items for an empty directory.

### `bt` CLI

- This specification will ensure that the **FileSystem** component:
    - has a 'bt' CLI subcommand for rendering this component
    - that the '--md' and '--html' CLI switches are available to render to Markdown and HTML targets respectively (the default render is always for the Terminal)
    - that the '--example' CLI switch is in place to provide a thoughtful example of how this command should be used with the CLI (see other working examples for a template)

#### Current State

The `bt dir` CLI command exists at `biscuit-terminal/cli/src/commands/dir.rs`.

| Aspect | Current State |
|--------|--------------|
| CLI command exists | Yes — `bt dir [PATH]` |
| Render method | Bespoke — calls `fs.render(&term)` directly via `TerminalRenderable` |
| `--md` switch | No |
| `--html` switch | No |
| `--example` switch | Yes — `bt dir --example` renders `bt dir . --depth 1 --filter ".rs"` |

The existing `DirArgs` struct has:
- `path` (positional, default `.`)
- `--example` / `-e`
- `--depth` / `-d`
- `--filter` / `-f`
- `--skip-root`
- `--size`, `--tokens`, `--modified`, `--updated`
- `--margin-left`, `--margin-right`, `--alignment` (via `LayoutArgs`)

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

2. **When `--md` or `--md-plus` is set:** The command calls `FileSystem`'s `MarkdownRenderable::render_markdown()` or `render_markdown_plus()` instead of the terminal `render()`. The output is printed to STDOUT with optional layout frontmatter (following the `render_markdown_with_layout_frontmatter` pattern from `prose.rs`).

3. **When `--html` is set:** The command calls `FileSystem`'s `BrowserRenderable::render_html_fragment()` and wraps it with layout CSS (following the `render_html_with_layout` pattern from `prose.rs`).

4. **Default (no target switch):** The command calls the terminal renderer. If the tree-render features are approved and implemented, this goes through `TreeComponent<FileSystem>` via `render_terminal_node`. If not approved, the existing bespoke `render()` path is retained.

5. **`--example` remains as-is** — the example command `bt dir . --depth 1 --filter ".rs"` already exists and is functional. The example should be updated to also mention the new `--md` and `--html` switches in a comment or secondary example line, but the primary example remains terminal-focused.

6. **Import updates:** `DirArgs::run()` must import `BrowserRenderable` and `MarkdownRenderable` when the target switches are added, following the import pattern in `prose.rs`.

## Acceptance Criteria Summary

- `FileSystem` implements `TerminalRenderable` (existing, retained)
- `FileSystem` implements `TreeRenderable` (new, projects to `List`/`ListItem` tree)
- `FileSystem` implements `BrowserRenderable` (new, via `BrowserTreeComponent` adapter or direct impl)
- `FileSystem` implements `MarkdownRenderable` (new, renders as nested Markdown list)
- `bt dir` gains `--md`, `--md-plus`, `--html` switches
- `bt dir --example` continues to work with terminal rendering
- Parity tests compare bespoke vs tree terminal output across all critical variants
- Browser and Markdown rendering tests cover all key variants
