# Table Component — IR Rendering Design Specification

## Current Status

| Property      | Value                                              |
|---------------|----------------------------------------------------|
| IR State      | both avail, old renders                            |
| bt CLI        | tree                                               |
| Terminal      | ✅                                                  |
| Browser       | ❌                                                  |
| Markdown      | ❌                                                  |
| Tree          | ✅                                                  |

The Table already has a `render_tree_node()` projection (`table.rs:1484-1574`) that produces a canonical `NodeKind::Table` subtree. The `bt table` CLI already renders through the tree renderer (`cli/src/commands/table.rs:134-141`). However:

- The default `TerminalRenderable::render()` still uses the bespoke path (`table.rs:1429-1453`).
- `BrowserRenderable` and `MarkdownRenderable` are not implemented.
- The `bt table` CLI has no `--md`, `--html`, or `--md-plus` switches.

## Design Steps

### Terminal IR Implementation

- The **Table** component already has a `render_tree_node()` projection that produces a `NodeKind::Table` with header and data rows, per-column hints (`TableColumnHints`), terminal-specific hints (`TableTerminalHints` for striping and cursor alignment), cell-level hints (`TableCellHints` for kind/raw-value/alignment), and `Layout` on the table node.
- The tree renderer (`biscuit-terminal/lib/src/render_tree/render.rs:941-1042`) already handles `NodeKind::Table` with a native two-pass renderer (width planning via `Table::plan_widths`, then border/header/data emit). It reconstructs a `Table` purely as a width-planning input and emits borders, header, and data rows itself.
- What remains is:
    - Flip `TerminalRenderable::render()` to delegate through the tree path instead of the bespoke `render_content` / `render_with_cursor_positioning` methods.
    - The IR implementation already drives the `TerminalRenderable` contract — the tree renderer produces the same box-drawing output as the bespoke path.
    - The IR implementation is already what is used by the `bt` CLI.

#### Flipping Strategy

1. Replace the body of `TerminalRenderable::render()` and `render_optimistic()` with:
   ```rust
   fn render(&self, term: &Terminal) -> String {
       let node = self.render_tree_node().expect("table always projects");
       let root = RenderNode::root(vec![node]);
       let opts = TerminalRenderOptions::new(term, RenderStrictness::Warn);
       render_terminal_node(&root, &opts)
           .map(|r| r.output)
           .unwrap_or_default()
   }
   ```
2. Keep the old `render_content` and `render_with_cursor_positioning` methods as `#[cfg(test)]`-gated methods so parity tests can still call them directly.
3. Add a parity test that renders the same `Table` through both paths and asserts semantic equivalence (ANSI-stripped content identical, border structure preserved, stripe colors match). Follow the discipline established by `BlockQuote` in `render_tree_component_parity.rs`.

#### Parity Test Variants

The following test variants will ensure high confidence in the tree renderer and detect regressions from the bespoke path:

1. **Simple table** — two columns, two rows, no title, no striping.
2. **Table with title** — title is rendered above the top border.
3. **Empty table** — no columns or data, returns empty string.
4. **Striped table (background only)** — even rows have background stripe.
5. **Striped table (text only)** — even rows have text stripe.
6. **Striped table (both)** — even rows have both stripes.
7. **Multi-line cell content** — cells with `\n`, row heights differ.
8. **Word-wrapped cells** — `WrapProse` triggers wrapping within columns.
9. **Right-aligned numeric column** — `ColumnType::Currency` / `ColumnType::Integer` alignment.
10. **Fixed-width columns** — `with_fixed_width` overrides auto-sizing.
11. **Min/max width constraints** — columns with `with_min_width` / `with_max_width`.
12. **Conditional visibility** — columns hidden at narrow widths via `Conditional`.
13. **Droppable columns** — columns dropped with `drop_when_space_is_limited`.
14. **Left/center/right block alignment** — `Alignment` on the table's `Layout`.
15. **Non-zero margins** — left/right margins via `Layout`.
16. **Cursor positioning** — `prefer_cursor_alignment` with ANSI escapes.
17. **Styled headers** — `with_header_style` (bold, colored).
18. **Styled body cells** — `with_body_style`.
19. **Uniform alignment** — `uniform_alignment` column flag.
20. **Vertical alignment** — `VerticalAlign::Top / Middle / Bottom` on multi-line cells.
21. **ANSI-colored cell content** — cells containing `\x1b[31m...\x1b[0m`.
22. **OSC8 hyperlink cells** — cells containing `\x1b]8;;url\x07text\x1b]8;;\x07`.

#### Feature Requests for Tree Rendering

##### FR-1: Title Node on Table

**What:** Add a first-class `title` field to the `NodeKind::Table` variant (or a dedicated hint on the table node's attrs). Currently the `render_tree_node()` projection encodes the title only as a side-channel not carried by the tree — the tree renderer's `render_table` method has no awareness of the table title.

**Why:** The tree renderer currently drops the table title because the `NodeKind::Table` variant does not carry it. The bespoke renderer includes it. Without this feature, the tree renderer would produce output missing the title line, creating a parity gap.

**Example usage:**
```rust
let mut node = RenderNode::table(align, rows);
node.attrs.set_table_title(&title); // new hint
```

**Impact without:** The Table would need to prepend the title manually after tree rendering, which is a minor but inelegant workaround. The title is a core table feature; it should be first-class.

##### FR-2: Table Title Hint on NodeAttrs

**What:** Add `set_table_title` / `table_title` accessors on `NodeAttrs`, analogous to `set_layout` / `layout`. The terminal renderer would emit the title above the top border when present.

**Why:** This avoids changing the `NodeKind::Table` enum variant while keeping the title information available to all three renderers. The Markdown renderer would emit it as a heading before the table; the Browser renderer would emit it as a `<caption>` element.

**Impact without:** Minor — the Table's `render()` wrapper would prepend the title after tree rendering. Not a blocker.

#### Assessment

The existing tree renderer is a **good fit** for Table. The two-pass architecture (width planning then native emit) mirrors the bespoke renderer's approach. The `TableColumnHints`, `TableCellHints`, and `TableTerminalHints` already carry all the metadata needed for faithful rendering. The only gap is the title, which is a small feature request.

- `will_use_tree_renderer`: **true** — the tree renderer already handles Table in the `bt` CLI and produces correct output. The title can be prepended as a temporary workaround.
- `will_use_tree_renderer_with_feature`: **true** — with FR-1/FR-2 (title hint), the tree renderer would be feature-complete for Table.

### Browser IR Implementation

- in this section we will provide a design specification for the **Table** component's implementation of the `BrowserRenderable` trait

The Browser tree renderer (`renderable/src/tree/render/browser.rs:448-531`) already handles `NodeKind::Table`:

- `render_table` builds a `<table>` with `<thead>` (first row) and `<tbody>` (remaining rows).
- `render_table_row` emits `<tr>` with column alignment as `style="text-align:..."`.
- `render_table_cell` emits `<th>` (header) or `<td>` (body), carrying column alignment.

The Table's `render_tree_node()` projection already produces the canonical tree. The `BrowserTreeComponent<T>` adapter (`biscuit-terminal/lib/src/render_tree/browser_adapter.rs`) wraps any `TreeRenderable` and provides an infallible `BrowserRenderable` impl by calling `render_tree_node()` then `render_browser_node()`.

#### Design

1. **Implement `BrowserRenderable` for Table** using the `BrowserTreeComponent` adapter pattern — or directly by projecting to the tree and rendering through `render_browser_node`.

2. **Styling considerations:**
   - The `TableStyle` slot (header/body emphasis and color, stripe colors) rides on the tree as `Style` on individual `TableCell` nodes. The browser renderer does not yet lower `Style` to CSS (see layout-and-style.md gap: "Browser `Style` lowering is unbuilt"). When browser `Style` lowering lands, header/body cell styles and stripe colors will automatically be applied as CSS.
   - In the interim, the browser output will be a semantically correct `<table>` with proper `<thead>`/`<tbody>` structure, column alignment, and escaped cell text — but without the styled appearance the terminal renderer applies.

3. **Title handling:**
   - When FR-2 (title hint) is implemented, the browser renderer should emit the title as a `<caption>` element inside the `<table>`.
   - As an interim workaround, `render_html_fragment()` can prepend the title as a heading or `<caption>` before the table fragment.

#### Implementation Approach

```rust
impl BrowserRenderable for Table {
    fn render_html_fragment(&self) -> BrowserFragment<Ready> {
        let node = self.render_tree_node().expect("table always projects");
        let root = RenderNode::root(vec![node]);
        let opts = BrowserRenderOptions::default();
        match render_browser_node(&root, &opts) {
            Ok(rendered) => rendered.output,
            Err(_) => BrowserFragment::new()
                .define_as_block_tag(BlockTag::Div, "table-render-error")
                .finalize(),
        }
    }
}
```

#### Key Test Variants

1. Simple two-column table — produces `<table><thead><tr><th>...</th></tr></thead><tbody>...</tbody></table>`.
2. Table with title — includes `<caption>` (or interim heading).
3. Column alignment — right-aligned column gets `style="text-align:right"`.
4. Empty table — produces empty `<div>` or empty fragment.
5. Single row (header only) — `<thead>` with no `<tbody>`.
6. Multi-line cell content — content is joined, `<td>` contains the full text.
7. Striped table — until browser `Style` lowering lands, striping is absent from HTML output. A future test will verify stripe colors become CSS classes/backgrounds once the feature is built.

### Markdown IR Implementation

The Markdown tree renderer (`renderable/src/tree/render/markdown.rs:332-357`) already handles `NodeKind::Table`:

- `render_table` emits a GFM pipe-delimited table: header row, delimiter row with alignment indicators, then data rows.
- `render_table_row` joins cell content with ` | ` delimiters and wraps in `| ... |`.
- `delimiter_row` emits `:---` (left), `---:` (right), `:---:` (center), or `---` (none) based on `ColumnAlign`.

The Table's `render_tree_node()` projection already produces the canonical tree. The `MarkdownRenderable` trait requires two methods: `render_markdown()` and `render_markdown_plus()`.

#### Design

1. **Implement `MarkdownRenderable` for Table** by projecting to the tree and rendering through the Markdown tree renderer.

2. **Markdown vs MarkdownPlus divergence for Table:**
   - **Table structure** is purely GFM — no inline HTML needed. Both `render_markdown()` and `render_markdown_plus()` produce identical pipe-delimited table output for the table structure itself.
   - **Cell styling** (header emphasis, body color, stripe colors) is where the two formats diverge:
     - **Markdown**: drops all color/styling. Cells are plain text.
     - **MarkdownPlus**: could wrap styled cells in `<span style="color:...">` or `<td style="...">` to preserve appearance. However, since the Markdown tree renderer currently ignores `Style` entirely (by design — see layout-and-style.md), both outputs will be identical until style-aware Markdown rendering is added.
   - **Title**: prepended as a Markdown heading (`### Title`) above the table in both formats.
   - **Conclusion**: For the Table component, `render_markdown()` and `render_markdown_plus()` will return identical output in the initial implementation. The divergence point (cell colors/styling) will be addressed when Markdown `Style` lowering is built.

#### Implementation Approach

```rust
impl MarkdownRenderable for Table {
    fn render_markdown(&self) -> String {
        self.render_markdown_target(MarkdownDialect::Markdown)
    }

    fn render_markdown_plus(&self) -> String {
        self.render_markdown_target(MarkdownDialect::MarkdownPlus)
    }
}

impl Table {
    fn render_markdown_target(&self, dialect: MarkdownDialect) -> String {
        let node = self.render_tree_node().expect("table always projects");
        let root = RenderNode::root(vec![node]);
        let opts = MarkdownRenderOptions { dialect, ..Default::default() };
        match render_markdown_node(&root, &opts) {
            Ok(rendered) => {
                let mut output = String::new();
                if let Some(ref title) = self.title {
                    output.push_str(&format!("### {title}\n\n"));
                }
                output.push_str(&rendered.output);
                output
            }
            Err(_) => String::new(),
        }
    }
}
```

#### Testing Strategy

1. **Simple table** — GFM pipe output with header, delimiter, and data rows.
2. **Table with title** — title appears as `### Title\n\n` before the table.
3. **Column alignment** — delimiter row uses `:---`, `---:`, `:---:` appropriately.
4. **Empty table** — returns empty string.
5. **Numeric cell values** — currency/integer/float values are rendered as their formatted text representation.
6. **Multi-line cells** — newlines within cells are preserved in the Markdown output (GFM allows inline content; multi-line cells may degrade).
7. **Markdown vs MarkdownPlus parity** — for the Table component, both outputs are identical. A test asserts `render_markdown() == render_markdown_plus()`.
8. **Escaping** — cell content containing `|` or other Markdown-special characters is properly escaped.

### `bt` CLI

- this specification will ensure that the **Table** component:
    - has a 'bt' CLI subcommand for rendering this component
    - that the '--md' and '--html' CLI switches are available to render to Markdown and HTML targets respectively (the default render is always for the Terminal)
    - that the '--example' CLI switch is in place to provide a thoughtful example of how this command should be used with the CLI (see other working examples for a template)

#### Current State

| Aspect                    | Status                                                               |
|---------------------------|----------------------------------------------------------------------|
| CLI command exists        | ✅ `bt table` (`cli/src/commands/table.rs`)                           |
| Render method used in CLI | ✅ Tree renderer (`render_tree_node()` → `render_terminal_node()`)    |
| `--md` switch             | ❌ Not present                                                        |
| `--html` switch           | ❌ Not present                                                        |
| `--md-plus` switch        | ❌ Not present                                                        |
| `--example` switch        | ✅ Present and functional (`-e` / `--example`)                       |

The existing `bt table` command (`cli/src/commands/table.rs`):
- Builds a `Table` from `--columns` and `--row` arguments.
- Applies styling flags: `--striped`, `--stripe-bg`, `--stripe-text`, `--bold-header`, `--header-color`, `--body-color`.
- Projects to the render tree via `table.render_tree_node()`.
- Renders through `render_terminal_node()`.
- Has a working `--example` flag.

#### Required Changes

1. **Add `--md`, `--html`, `--md-plus` switches** to `TableArgs`:
   ```rust
   /// Render to an HTML fragment instead of the terminal.
   #[arg(long, conflicts_with_all = ["md", "md_plus"])]
   pub html: bool,

   /// Render to portable Markdown instead of the terminal.
   #[arg(long, conflicts_with_all = ["html", "md_plus"])]
   pub md: bool,

   /// Render to MarkdownPlus instead of the terminal.
   #[arg(long = "md-plus", conflicts_with_all = ["html", "md"])]
   pub md_plus: bool,
   ```

2. **Wire the cross-target rendering** in the `Run` implementation:
   ```rust
   if self.html {
       println!("{}", table.render_html_fragment().render());
       return Ok(());
   }
   if self.md {
       println!("{}", table.render_markdown());
       return Ok(());
   }
   if self.md_plus {
       println!("{}", table.render_markdown_plus());
       return Ok(());
   }
   // Existing terminal rendering path (unchanged)
   ```

3. **Update `--example` to show the appropriate command** for the active target:
   - When `--html` is combined with `--example`, print the example with `--html` appended.
   - When `--md` is combined with `--example`, print the example with `--md` appended.
   - The default `--example` (no target flag) shows the terminal command.

4. **Update `TABLE_EXAMPLE_CMD`** to demonstrate the canonical example:
   ```rust
   const TABLE_EXAMPLE_CMD: &str = r#"bt table --columns "Area,Status,Owner" --row "Parser,Green,Lin" --row "Renderer,Review,Mara" --row "Docs,Updated,Noor" --striped"#;
   ```
   This is already correct — no change needed.

5. **Follow the `bt prose` pattern** for target-switch wiring (`prose.rs:36-100`), which already implements `--html`, `--md`, and `--md-plus` correctly.
