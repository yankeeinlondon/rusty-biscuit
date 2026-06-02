---
last_updated: "2026-05-17"
---

# Challenges of Migrating the `FileTree` Component to the Tree Rendering Architecture

## Functional and Design Goals

The `FileTree` component was created to visualize a Markdown file's **dependency graph** — the complete set of references (hyperlinks, images, CSS imports, scripts, transclusions) emanating from a document, and optionally, the dependency graphs of every document it recursively transcludes.

### Why it was created

The `darkmatter` CLI already had reference analysis infrastructure (`ReferenceGraph`, `ReferenceValidationReport`) but no way to **present** the graph visually in a terminal. The `md graph` subcommand needed a structured, terminal-renderable tree that could:

- Show all references above a file line, grouped by semantic kind (remote hyperlinks, images, CSS imports, etc.)
- Show transclusion edges below the file line with contextual captions ("inserted into the `## Details` section")
- Recursively expand followed transclusions as nested subtrees
- Overlay validation results (missing targets, unreachable URLs) with severity-colored markers
- Distinguish file-level metadata (inline CSS/script/meta counts) as an inline summary on the file head line

### Where it is used today

| Consumer | Location | How it is invoked |
|----------|----------|-------------------|
| `md graph` CLI command | `darkmatter/cli/src/commands.rs:1828` | `FileTree::new(path).follow_transclusions().validate().ensure_built()` then `tree.display(&term)` |
| Integration tests | `darkmatter/lib/tests/reference_integration.rs` (lines 820–1100+) | Tests for follow mode, TOC linking, epilogue/prologue, multiple prologues, `show_root(false)`, section captions |
| Unit tests | `darkmatter/lib/src/markdown/reference/file_tree/` | Model building, rendering, icons |

### Example usage

**Basic graph (no follow):**

```rust
use darkmatter::markdown::reference::file_tree::FileTree;
use biscuit_terminal::components::renderable::TerminalRenderable;

let mut tree = FileTree::new("doc.md")?;
tree.ensure_built()?;
print!("{}", tree.display(&term));
```

This produces terminal output like:

```
╭── 🔗 https://example.com
├── 📸 ./logo.png
│
📄 test.md (1 inline CSS block)
│
│◀─ 📄 @docs/child.md  inserted into the '## Details' section
```

**Recursive follow with validation:**

```rust
let mut tree = FileTree::new("root.md")
    .unwrap()
    .follow_transclusions()
    .validate();
tree.ensure_built()?;
print!("{}", tree.display(&term));
```

This expands every followable transclusion into a nested subtree and annotates broken references with `[missing]`, `[unreachable]`, etc.

**Hiding the root (`show_root(false)`):**

```rust
let mut tree = FileTree::new("root.md").unwrap().show_root(false);
tree.ensure_built()?;
```

Renders the root's references and transclusion edges but omits the root file head line itself — useful when the caller already knows which file it started from.

## Technical Implementation (current)

### Module structure

```
darkmatter/lib/src/markdown/reference/file_tree/
├── mod.rs      — FileTree struct, TerminalRenderable impl, builder API
├── model.rs    — FileTreeModel, FileTreeNode, and graph-to-model transformation
├── render.rs   — Terminal rendering (three-zone layout, box-drawing connectors)
└── icons.rs    — Nerd Font / Unicode fallback icon selection
```

### The `FileTree` struct

A builder-pattern struct that holds:

- `md: Markdown` — the source document
- `follow: bool` — whether to recursively expand transclusions
- `do_validate: bool` — whether to run reference validation
- `show_root: bool` — whether to render the root file head line
- `graph_options: ReferenceGraphOptions` — extraction settings
- `validation_options: ReferenceValidationOptions` — validation settings
- `layout: Layout` — standard TerminalRenderable layout
- `model: Option<FileTreeModel>` — lazily built view model (invalidated by builder mutations)
- `graph: Option<ReferenceGraph>` — the raw reference graph
- `validation_report: Option<ReferenceValidationReport>` — validation results

### Key responsibilities and transforms

1. **Builder API → lazy model construction** — `follow_transclusions()`, `validate()`, `show_root()` etc. set flags and invalidate `model`. `ensure_built()` is idempotent and performs the actual graph extraction, validation, and model construction.

2. **Graph-to-model transformation** (`model.rs:build_file_tree_model`) — Converts a `ReferenceGraph` + optional `ReferenceValidationReport` into a presentation-oriented `FileTreeModel`. Key transforms:
   - Classify each `ReferenceRecord` into a `FileTreeReferenceGroupKind` (remote hyperlinks, images, CSS imports, etc.) or as an inline/transclusion type
   - Deduplicate rows by `display_target` within each group
   - Sort groups by fixed `sort_order()`
   - Build `FileTreeTransclusionEdge` records with contextual captions (e.g. "inserted into the `## Details` section")
   - Match transclusion records to child insertions by `reference_id` (stable) falling back to `directive_line`
   - Recursively build child `FileTreeNode`s when `follow=true`
   - Compute per-node validation summaries (`issues_count`, `has_errors`)
   - Track `FileTreeInlineSummary` counts (inline CSS blocks, scripts, meta tags)

3. **Three-zone terminal rendering** (`render.rs`) — Each node renders in three zones:
   - **Zone 1: Reference groups** above the file head — uses `╭──` for the first row, `├──` for subsequent rows, `│` separators between groups
   - **Zone 2: File head line** — icon + bold filename + dim inline summary
   - **Zone 3: Transclusion edges** below the file head — uses `│◀─` (incoming) or `├─▶`/`╰─▶` (outgoing/TOC-linking) connectors
   - **Zone 4: Followed children** — indented below their edge, using `│   ` or blank indent depending on whether the parent edge was the last one

4. **ANSI styling** — Conditional on `is_tty`:
   - File head: bold filename, dim summary
   - Transclusion targets: blue (256-color #75)
   - Captions: dim + italic, with section heading names in normal weight
   - Prologue/epilogue: inverse label (`prologue` / `epilogue`)
   - Validation errors: red, warnings: yellow, info: cyan

5. **Line truncation** — Each line is truncated to terminal width using `visible_width()` and `split_at_visible_width()`, appending an ellipsis (`…`) when truncated.

6. **Icon selection** (`icons.rs`) — Dual-mode icons:
   - Nerd Font glyphs (when `term.is_nerd_font == Some(true)`) with trailing spaces for visual alignment
   - Unicode emoji fallbacks (🔗, 📄, 📸, 🧠, etc.)

7. **`TerminalRenderable` compliance** — `render(&Terminal)` and `render_optimistic(Option<u32>)` delegate to the render module, then apply `Layout`. The struct is block-level.

## Implementation Challenges

### Structural Mismatch Between FileTree's Zone Model and NodeKind

#### The challenge

The `FileTree` component has a fundamentally different visual structure from any existing `NodeKind`. FileTree renders per-node in a **three-zone layout** (reference groups above, file head, transclusions below) connected by box-drawing characters (`╭`, `├`, `│`, `╰`) that form a continuous vertical line spanning all zones. The existing `NodeKind` variants model document content (headings, paragraphs, lists, block quotes) — none of which has a concept of "content above and below a central divider connected by vertical lines."

The closest analogue is `BlockQuote` (vertical border + children), but FileTree's zones are not a simple parent-child nesting. The reference groups and transclusion edges are siblings of the file head, not children, yet they are visually connected through shared connector characters.

**Example:** A single `FileTreeNode` with 2 reference groups and 2 transclusion edges renders as:

```
╭── 🔗 https://a.com
├── 🔗 https://b.com
│
├── 📸 ./logo.png
│
📄 doc.md
│
│◀─ 📄 child.md  inserted
╰◀─ 📄 utils.md  inserted
```

There is no `NodeKind` variant that can represent this "vertical-line-connected multi-zone" pattern.

**Suggested unit test:**

```rust
#[test]
fn file_tree_zones_cannot_be_expressed_as_existing_node_kind() {
    // Build a simple model with one ref group and one transclusion
    let model = simple_model_with_refs_and_transclusions();

    // Attempting to project to NodeKind should demonstrate that:
    // - BlockQuote only has children (no "above" content)
    // - List items have no vertical-line connection between groups
    // - Table rows are grid-aligned, not tree-connected
    // This test documents the structural gap.
    let zones = count_visual_zones(&model.root);
    assert_eq!(zones, 3, "node has 3 visual zones (refs, head, transclusions)");
    // None of the 25 NodeKind variants can represent 3 connected zones
}
```

#### Recursive Indentation with Position-Aware Connectors

#### The challenge

In follow mode, `FileTree` renders children recursively with incrementing indentation. Each indentation level carries **position-aware** connector strings — `│   ` for a non-last child (vertical line continues) versus 8 spaces for a last child (vertical line terminated). The choice of connector depends on the child's position relative to its siblings, not just its depth.

The current tree renderer applies layout via `Layout` (margins, alignment) but has no concept of **connector-based indentation** where the indent prefix itself is a structural element that changes based on sibling position.

**Example:** A root with two children where the first child has its own transclusion:

```
📄 root.md
│
│◀─ 📄 a.md  inserted
│   ╭── 🔗 https://child-a.com
│   │
│   📄 a.md
│   │
│   ╰◀─ 📄 grandchild.md  inserted
│       📄 grandchild.md
│
╰◀─ 📄 b.md  inserted
    📄 b.md
```

Note how `a.md`'s subtree uses `│   ` prefix (vertical continues to `b.md` below) while `b.md`'s subtree uses 8-space blank prefix (nothing continues below). The tree renderer's `for_child(indent_delta, width_delta)` cannot express "my indent string is `│   ` versus `        ` depending on whether I'm the last sibling."

**Suggested unit test:**

```rust
#[test]
fn position_aware_indent_differs_between_last_and_non_last_child() {
    let model = model_with_two_followed_children();

    let output = render_model_optimistic(&model, 120, true);
    let lines: Vec<&str> = output.lines().collect();

    // Find lines belonging to first child's subtree
    let first_child_lines: Vec<&&str> = lines.iter()
        .filter(|l| l.contains("child-a.com") || l.contains("a.md"))
        .collect();

    // First child's lines should have │ in indent (vertical continues)
    let has_vertical_connector = first_child_lines.iter()
        .any(|l| l.contains('\u{2502}')); // │
    assert!(has_vertical_connector, "first child indent should have vertical connector");

    // Last child's lines should NOT have │ in indent
    let last_child_lines: Vec<&&str> = lines.iter()
        .filter(|l| l.contains("b.md"))
        .collect();
    // Last child subtree uses blank indent, no vertical connector
}
```

#### Box-Drawing Character Continuity Across Zones

#### The challenge

The FileTree's visual identity depends on a **continuous vertical line** (`│`) that connects reference groups above through the file head down to transclusion edges below. This line is not a property of any single zone — it is an emergent property of how zones are concatenated. The current render tree model treats each node as independently renderable; there is no mechanism for sibling nodes to coordinate their connector characters.

In the current bespoke renderer, `render_transclusions_unified()` explicitly manages this continuity: it knows whether a `│` separator is needed between kind changes, whether the first ref row uses `╭`, and whether the last transclusion uses `╰`.

**Example:** When rendering a node with references followed by transclusions, the `│` between the last ref group separator and the file head, then between the file head and the first transclusion, forms one continuous vertical line. If these zones were separate tree nodes, each would need to know about its neighbors to produce the correct connectors.

```
╭── 🔗 https://example.com   ← ╭ because nothing above
│                              ← │ separator between groups
├── 📸 ./logo.png              ← ├ because file head follows
│
📄 doc.md                      ← file head (no connector)
│                              ← │ connects head to edges below
│◀─ 📄 child.md  inserted     ← │◀─ because more edges follow
╰◀─ 📄 utils.md  inserted     ← ╰ because last edge
```

**Suggested unit test:**

```rust
#[test]
fn vertical_line_is_continuous_across_all_zones() {
    let model = model_with_refs_head_and_transclusions();

    let output = render_model_optimistic(&model, 120, true);
    let lines: Vec<&str> = output.lines().collect();

    // Find the column index of the vertical line in each zone
    let ref_line_idx = lines.iter().position(|l| l.contains("example.com")).unwrap();
    let head_idx = lines.iter().position(|l| l.contains("doc.md")).unwrap();
    let edge_idx = lines.iter().position(|l| l.contains("child.md")).unwrap();

    // The vertical line character │ should appear at the same column
    // across all three zones
    let ref_vert_col = lines[ref_line_idx].find('\u{2502}')
        .or_else(|| lines[ref_line_idx].find('\u{256D}')); // ╭ also at that column
    let separator_col = lines[head_idx - 1].find('\u{2502}');
    let edge_vert_col = lines[edge_idx].find('\u{2502}')
        .or_else(|| lines[edge_idx].find('\u{2570}')); // ╰ also at that column

    assert!(ref_vert_col.is_some(), "ref zone should have vertical connector");
    assert!(separator_col.is_some(), "separator should have vertical connector");
    assert!(edge_vert_col.is_some(), "edge zone should have vertical connector");
    assert_eq!(ref_vert_col, separator_col, "vertical column should be consistent");
    assert_eq!(separator_col, edge_vert_col, "vertical column should be consistent");
}
```

#### Multi-Type Edge Styling (Inbound, Outbound, Prologue, Epilogue)

#### The challenge

FileTree's transclusion edges are not homogeneous. Each edge has a `FileTreeTransclusionKind` (File, Code, Url, TocLinking, Prologue, Epilogue) that determines:

1. The connector style (`│◀─` for inbound, `├─▶`/`╰─▶` for outbound/TOC-linking)
2. The label style (inverse badge for prologue/epilogue, blue target for regular edges)
3. The caption style (dim+italic for regular, or "references X" for prologue/epilogue)
4. Whether the edge displays a target at all (literal frontmatter prologues show "includes static text")

The current tree renderer has no mechanism for per-variant styling within a single node type. A `ListItem` is a `ListItem` regardless of content; it cannot switch between inverse-badge rendering and blue-target rendering based on an attribute.

**Example:** A single node with a prologue, a file transclusion, and a TOC-linking edge:

```
📄 root.md
│
│◀─ [prologue] references  prologue.md
│
│◀─ 📄 child.md  inserted into the '## Intro' section
│
╰─▶ 📄 toc-child.md  TOC elements linked
```

Three different visual treatments for what would be three sibling nodes of the same "type."

**Suggested unit test:**

```rust
#[test]
fn edge_kinds_produce_distinct_visual_treatments() {
    let model = model_with_prologue_file_and_toc_edges();

    let term = Terminal { is_tty: true, is_nerd_font: Some(false), ..Default::default() };
    let output = render_model(&model, &term, true);
    let lines: Vec<&str> = output.lines().collect();

    let prologue_line = lines.iter().find(|l| l.contains("prologue")).unwrap();
    let file_edge_line = lines.iter().find(|l| l.contains("child.md")).unwrap();
    let toc_line = lines.iter().find(|l| l.contains("toc-child.md")).unwrap();

    // Prologue uses inverse badge: \x1b[7m
    assert!(prologue_line.contains("\x1b[7m"), "prologue should use inverse styling");
    // File edge uses blue target: \x1b[38;5;75m
    assert!(file_edge_line.contains("\x1b[38;5;75m"), "file edge target should be blue");
    // TOC uses outgoing arrow ▶
    assert!(toc_line.contains('\u{25B6}'), "TOC edge should use outgoing arrow");
}
```

#### Conditional Content Based on Builder Flags

#### The challenge

The `FileTree` component uses several builder flags that control what content appears:

- `show_root: bool` — controls whether the root file head renders
- `follow: bool` — controls whether transclusions expand into children
- `do_validate: bool` — controls whether validation suffixes appear

These flags affect the **shape** of the rendered output, not just its styling. `show_root(false)` removes the root's Zone 2 entirely while preserving Zones 1 and 3. `follow(false)` means transclusion edges appear but have no children beneath them.

The tree renderer's `NodeAttrs` supports `data` (namespaced extension data) and `classes`, but the renderer itself has no built-in concept of "conditionally omit this entire subtree based on a flag."

**Example:** `show_root(false)` produces:

```
╭── 🔗 https://example.com
├── 📸 ./logo.png
│
│◀─ 📄 child.md  inserted
```

The file head line (`📄 doc.md`) is gone but the reference groups above and transclusion edges below are preserved. In a tree model, this would mean "render the root's children but not the root itself" — which is unusual because typically the root is a container that wraps its children.

**Suggested unit test:**

```rust
#[test]
fn show_root_false_preserves_refs_and_edges_but_omits_file_head() {
    let model = model_with_refs_head_and_transclusions();

    let with_root = render_model_optimistic(&model, 120, true);
    let without_root = render_model_optimistic(&model, 120, false);

    assert!(with_root.contains("doc.md"), "with root should have file head");
    assert!(!without_root.contains("doc.md"), "without root should omit file head");
    assert!(without_root.contains("example.com"), "refs should survive");
    assert!(without_root.contains("child.md"), "edges should survive");
}
```

#### Icon and Width-Aware Truncation

#### The challenge

FileTree uses Nerd Font icons that are wider than a single terminal cell (hence the trailing padding spaces in `icons.rs`). The `truncate_line()` function uses `visible_width()` and `split_at_visible_width()` from `biscuit_terminal::utils::block_constraint` to handle ANSI escapes, but it does not account for wide Unicode characters or Nerd Font glyphs specifically — it relies on the existing width calculation infrastructure.

When the tree renderer processes a `Text` node, it has no awareness that the text contains wide glyphs or Nerd Font icons that may need special padding. The existing tree renderer's layout system handles margins and alignment but delegates actual text measurement to the terminal width utilities.

**Example:** A reference row with a long URL that must be truncated:

```
├── 🔗 https://very-long-domain-name.example.com/path/to/resource/that/exceeds/width…
```

The ellipsis replaces the truncated portion. If the Nerd Font icon is miscounted as 1 cell instead of 2, the truncation point is off by 1 cell.

**Suggested unit test:**

```rust
#[test]
fn truncation_respects_nerd_font_icon_width() {
    let mut model = simple_model();
    model.root.reference_groups = vec![FileTreeReferenceGroup {
        kind: FileTreeReferenceGroupKind::RemoteHyperlinks,
        rows: vec![FileTreeReferenceRow {
            kind: ReferenceKind::Hyperlink,
            display_target: "https://example.com/a/very/long/path/that/exceeds/width".into(),
            raw_reference_id: "r1".into(),
            validation: None,
        }],
    }];

    let term = Terminal { is_nerd_font: Some(true), is_tty: true, ..Default::default() };
    // Set a width that forces truncation
    let output = render_model(&model, &term, true);
    let lines: Vec<&str> = output.lines().collect();
    let ref_line = lines.iter().find(|l| l.contains("example")).unwrap();

    // The visible width of the line should not exceed terminal width
    let visible = visible_width(ref_line) as usize;
    assert!(visible <= 80, "visible width {visible} should be <= 80");
    assert!(ref_line.contains('\u{2026}'), "truncated line should have ellipsis");
}
```

#### Reference Group Sorting and Deduplication

#### The challenge

The model-building phase in `model.rs` sorts reference groups by `sort_order()` and deduplicates rows by `display_target` within each group. These are data transformations that happen **before** rendering. In a tree-based approach, this logic would either need to happen during tree projection (the `TreeRenderable::render_tree()` call) or be encoded as node attributes that the renderer interprets.

The current tree renderer does not perform sorting or deduplication — it renders children in the order they appear. Pushing this logic into the tree model means the tree itself must represent "these items are sorted by X and deduplicated by Y," which is a semantic concern currently handled in the model layer.

**Example:** A document with references that would naturally appear in reverse sort order:

```markdown
[link](https://z-site.com)
![img](./logo.png)
[link](https://a-site.com)
```

The model builder groups and sorts them so remote hyperlinks come before images, and URLs within the same group appear in encounter order (not alphabetical). The renderer just displays what the model gives it.

**Suggested unit test:**

```rust
#[test]
fn reference_groups_appear_in_fixed_sort_order_regardless_of_encounter_order() {
    let mut model = simple_model();
    // Add groups in reverse sort order (Images=2, then RemoteHyperlinks=0)
    model.root.reference_groups = vec![
        FileTreeReferenceGroup {
            kind: FileTreeReferenceGroupKind::Images,
            rows: vec![FileTreeReferenceRow {
                kind: ReferenceKind::Image,
                display_target: "./logo.png".into(),
                raw_reference_id: "r1".into(),
                validation: None,
            }],
        },
        FileTreeReferenceGroup {
            kind: FileTreeReferenceGroupKind::RemoteHyperlinks,
            rows: vec![FileTreeReferenceRow {
                kind: ReferenceKind::Hyperlink,
                display_target: "https://example.com".into(),
                raw_reference_id: "r2".into(),
                validation: None,
            }],
        },
    ];

    let output = render_model_optimistic(&model, 120, true);
    let lines: Vec<&str> = output.lines().collect();

    let link_idx = lines.iter().position(|l| l.contains("example.com")).unwrap();
    let img_idx = lines.iter().position(|l| l.contains("logo.png")).unwrap();
    assert!(link_idx < img_idx, "remote hyperlinks (sort_order=0) should appear before images (sort_order=2)");
}
```

#### Follow-Mode Child-Edge Merging

#### The challenge

When `follow=true`, a followed transclusion edge and its child node are **merged** in the rendering: the edge arrow line becomes the child's visual header, and the child's reference groups and transclusions render below it with increased indentation. The child does **not** get its own separate file head line.

This merging of edge + child into a single visual unit is unusual. In the tree model, an edge would be one node and its child would be another — but the FileTree renderer treats them as one unit where the edge arrow is the header. The current tree renderer has no concept of "merge this parent node's label with its first child."

**Example:** Follow mode rendering:

```
📄 root.md
│
│◀─ 📄 child.md  inserted into the '## Intro' section
    ╭── 🔗 https://child-link.com
    ├── 📸 ./child-img.png
    │
    📄 child.md (2 inline CSS blocks)
    │
    ╰◀─ 📄 grandchild.md  inserted
        📄 grandchild.md
```

Note that `child.md`'s file head line appears below its reference groups, indented under the edge arrow. The edge arrow line and the child's entire subtree are one merged visual block.

**Suggested unit test:**

```rust
#[test]
fn follow_mode_merges_edge_with_child_subtree() {
    let model = model_with_followed_child();

    let output = render_model_optimistic(&model, 120, true);
    let lines: Vec<&str> = output.lines().collect();

    // The child.md target should appear exactly once (as edge label, not again as file head)
    let child_mentions = lines.iter().filter(|l| l.contains("child.md")).count();
    assert_eq!(child_mentions, 1, "child.md should appear once (edge merged with child)");

    // The child's references should appear below the edge line
    let edge_idx = lines.iter().position(|l| l.contains("child.md")).unwrap();
    let ref_idx = lines.iter().position(|l| l.contains("child-link.com")).unwrap();
    assert!(ref_idx > edge_idx, "child refs should be below edge line");
}
```

#### Validation Overlay as a Cross-Cutting Concern

#### The challenge

Validation results (`[missing]`, `[unreachable]`, `[invalid url]`) are overlays that attach to both reference rows and transclusion edges. They are not separate nodes — they are annotations on existing rows that change their styling (red/yellow/cyan ANSI colors) and append a suffix string.

In the tree model, the closest analogue would be node attributes or classes. But the tree renderer currently has no mechanism for "if this node has validation=error, color it red and append `[missing]`." The renderer applies layout but does not interpret semantic annotations to determine styling.

**Example:** A reference row with a validation error:

```
├── 🔗 ./missing-file.md [missing]     ← red text when TTY
```

The `[missing]` suffix and red color come from `FileTreeReferenceValidation` attached to the row. This is a per-row decoration that depends on external validation data.

**Suggested unit test:**

```rust
#[test]
fn validation_suffix_colored_by_severity() {
    let mut model = simple_model();
    model.root.reference_groups = vec![FileTreeReferenceGroup {
        kind: FileTreeReferenceGroupKind::LocalHyperlinks,
        rows: vec![FileTreeReferenceRow {
            kind: ReferenceKind::Hyperlink,
            display_target: "./missing.md".into(),
            raw_reference_id: "r1".into(),
            validation: Some(FileTreeReferenceValidation {
                is_valid: false,
                suffix: Some("[missing]".into()),
                severity: ReferenceSeverity::Error,
            }),
        }],
    }];

    let term = Terminal { is_tty: true, ..Default::default() };
    let output = render_model(&model, &term, true);

    assert!(output.contains("[missing]"), "should show validation suffix");
    assert!(output.contains("\x1b[31m"), "error severity should be red");
}
```

#### Multi-Format Output (Terminal, Markdown, HTML)

#### The challenge

The tree rendering architecture explicitly targets multiple output formats (terminal via `biscuit-terminal`, markdown via `renderable`, HTML/browser via `renderable`). However, FileTree's current rendering is heavily terminal-specific: box-drawing characters, ANSI color codes, Nerd Font icons, cursor-positioned text.

Producing meaningful **Markdown** or **HTML** output from a FileTree requires fundamentally different visual representations:

- **Markdown output** might use nested bullet lists with emoji icons instead of box-drawing
- **HTML output** might use a collapsible tree widget with CSS-styled icons and colored badges

The current tree renderer's `NodeKind` vocabulary was designed for document content (headings, paragraphs, lists) — not for tree-visualization patterns. Forcing FileTree into that vocabulary would produce a tree that is semantically awkward and hard to render meaningfully in non-terminal targets.

**Example:** The same dependency graph rendered in three targets:

**Terminal:**

```
╭── 🔗 https://example.com
│
📄 doc.md
│
╰◀─ 📄 child.md  inserted
```

**Markdown:**

```markdown
- 🔗 https://example.com
- 📄 **doc.md**
  - 📄 child.md *(inserted)*
```

**HTML:**

```html
<ul class="file-tree">
  <li class="ref">🔗 <a href="https://example.com">example.com</a></li>
  <li class="file">📄 <strong>doc.md</strong>
    <ul>
      <li class="transclusion">📄 child.md <em>inserted</em></li>
    </ul>
  </li>
</ul>
```

**Suggested unit test:**

```rust
#[test]
fn file_tree_produces_different_structures_per_target() {
    let model = simple_model_with_refs_and_transclusions();

    // Terminal rendering uses box-drawing
    let term = Terminal::default();
    let term_output = render_model(&model, &term, true);
    assert!(term_output.contains('\u{256D}'), "terminal should use ╭");
    assert!(term_output.contains('\u{2502}'), "terminal should use │");

    // Markdown rendering should use bullet lists, not box-drawing
    // (This test documents the expectation; implementation would need
    // a markdown-specific renderer for FileTree)
    // let md_output = render_markdown_from_model(&model);
    // assert!(!md_output.contains('\u{256D}'), "markdown should not use ╭");
    // assert!(md_output.contains("- "), "markdown should use bullet lists");
}
```

## Solution Suggestions

#### Custom NodeKind Variant for FileTree Zones

#### Solution

Introduce a new `NodeKind` variant specifically for tree-visualization components. For example, `NodeKind::TreeVisualization` with typed sub-structure for zones, connectors, and position context:

```rust
TreeVisualization {
    zones: Vec<TreeZone>,
    connector_style: TreeConnectorStyle,
}
```

Where `TreeZone` is an enum capturing the zone types (references, file head, transclusion edges) and `TreeConnectorStyle` controls the box-drawing character set.

Alternatively, avoid extending `NodeKind` and instead treat `FileTree` as an inherently visual component (like `TerminalImage` and `GraphExpression`) that keeps its bespoke renderer permanently and is explicitly **not** intended to route through the tree.

#### Which challenges this helps with

- **Structural Mismatch** — provides a node type that can represent multi-zone layouts
- **Box-Drawing Continuity** — zones within a single `TreeVisualization` node share context for connector coordination
- **Multi-Format Output** — each renderer can interpret `TreeVisualization` according to its target (terminal uses box-drawing, markdown uses nested lists, HTML uses a tree widget)

#### Variants

- **Extension via `NodeAttrs.data`** — instead of a new `NodeKind`, encode the zone structure as namespaced data on a generic container node. This avoids inflating the `NodeKind` enum but makes the data opaque to the renderer.
- **Separate visualization tree model** — create a parallel tree model (`VisTree`) alongside `RenderNode` that is specifically designed for tree-visualization components. Renderers would have a separate dispatch path for `VisTree` nodes.

#### Position-Aware Context in the Render Walk

#### Solution

Extend the render walk to pass **sibling context** to each node. Currently the tree renderer processes children sequentially but does not tell a child "you are index 3 of 7." Adding `child_index` and `sibling_count` to the render context would allow nodes (or their adapters) to make position-dependent decisions about connector characters and indentation.

```rust
pub struct RenderContext {
    pub child_index: usize,
    pub sibling_count: usize,
    pub is_last: bool,
    // ... existing fields
}
```

#### Which challenges this helps with

- **Recursive Indentation with Position-Aware Connectors** — each child knows if it is the last sibling and can choose `│   ` vs. blank indent
- **Box-Drawing Continuity** — a node knows its position and can select `╭`, `├`, or `╰` accordingly

#### Variants

- **Parent-driven prefix injection** — instead of children knowing their position, the parent pre-computes indent prefixes and passes them as part of the child context. This keeps children simpler but requires the parent to understand connector semantics.
- **Two-pass rendering** — first pass computes the tree structure and determines connector types; second pass renders with the pre-computed connectors. This is how the current bespoke renderer works (the recursive call knows its position implicitly).

#### Semantic Styling via Node Attributes

#### Solution

Extend `NodeAttrs` with a typed `styling` field that carries semantic intent (e.g., "error", "warning", "info", "link-target", "badge") rather than raw ANSI codes. Each renderer interprets these semantic hints according to its target capabilities:

- Terminal renderer: maps semantic hints to ANSI codes
- Browser renderer: maps semantic hints to CSS classes
- Markdown renderer: maps semantic hints to formatting markers

```rust
pub enum SemanticStyle {
    Default,
    Error,
    Warning,
    Info,
    LinkTarget,
    Badge(BadgeKind),
    DimItalic,
    Bold,
}
```

#### Which challenges this helps with

- **Multi-Type Edge Styling** — prologue/epilogue edges use `Badge(Prologue)`, regular edges use `LinkTarget`, validation errors use `Error`
- **Validation Overlay** — validated rows carry `SemanticStyle::Error` which each renderer interprets correctly
- **Multi-Format Output** — each target maps semantic styles to its own representation

#### Variants

- **Class-based styling** — use the existing `classes` field on `NodeAttrs` and define a convention (e.g., `"ft-error"`, `"ft-badge-prologue"`) that each renderer interprets. Simpler but less type-safe.
- **Inline style map** — attach a `HashMap<String, SemanticStyle>` per node, where keys are rendering targets. More flexible but harder to maintain.

#### Conditional Rendering via Render Hints

#### Solution

Introduce a `render_hint` field on `NodeAttrs` that carries flags like `omit_self`, `render_children_only`, or `collapse_with_first_child`. These hints would be consumed by the renderer to alter the default parent-then-children rendering order.

```rust
pub enum RenderHint {
    Normal,
    OmitSelf,           // render children but not this node's own content
    MergeWithFirstChild, // this node's label becomes the first child's header
}
```

#### Which challenges this helps with

- **Conditional Content Based on Builder Flags** — `show_root(false)` sets `RenderHint::OmitSelf` on the root node
- **Follow-Mode Child-Edge Merging** — a followed edge sets `RenderHint::MergeWithFirstChild` so the edge arrow becomes the child's header

#### Variants

- **Node-level boolean flags** — instead of an enum, use individual boolean flags (`omit_self: bool`, `merge_with_first_child: bool`). Simpler but grows linearly with new behaviors.
- **Render-mode attribute** — a single string attribute like `"render": "children-only"` that the renderer interprets. Very flexible but completely untyped.

#### Pre-Rendered Subtree as Leaf Strategy

#### Solution

For components like `FileTree` where the visual structure is too specialized for the canonical tree model, adopt a strategy where the component produces a **pre-rendered terminal string** that is wrapped in a special `NodeKind` variant (e.g., `NodeKind::RawBlock { content: String }` or `NodeKind::Foreign { raw: String, kind: String }`). The renderer emits the string verbatim.

This is essentially the "keep bespoke renderer" approach but with a thin tree adapter so the component can participate in the tree pipeline for composition and multi-target dispatch, while delegating actual formatting to its own code.

#### Which challenges this helps with

- **All challenges** — the component keeps its current rendering logic intact
- **Structural Mismatch** — no need to express zones as `NodeKind` variants
- **Multi-Format Output** — only partially; the `RawBlock` approach only works for the terminal target. Markdown and HTML would need separate projection paths.

#### Variants

- **Target-specific raw blocks** — `NodeKind::ForeignTerminal { raw: String }` for terminal, with separate projections for markdown/HTML. This acknowledges that some components need per-target bespoke rendering.
- **Fallback to `Unsupported`** — treat FileTree as terminal-only and return `Unsupported` in the tree for non-terminal targets, with a documented fallback (e.g., a nested bullet list).

#### Treat FileTree as an Inherently Visual Component

#### Solution

Follow the precedent set by `TerminalImage` and `GraphExpression`: explicitly designate `FileTree` as an **inherently visual component** that is not intended to route through the tree. Keep its bespoke `TerminalRenderable` implementation permanently. For multi-target support, give it a separate `BrowserRenderable` implementation (like `HorizontalRule` has today) rather than forcing it through the `NodeKind` vocabulary.

#### Which challenges this helps with

- **All challenges** — none of the structural mismatches need to be solved because the component stays outside the tree pipeline
- **Multi-Format Output** — each target gets a hand-written, purpose-fit implementation rather than trying to derive one from a generic tree model

#### Variants

- **Hybrid approach** — keep the bespoke terminal renderer but add a lightweight tree projection for Markdown/HTML output only. The tree projection would produce a simplified representation (nested lists) that is sufficient for non-terminal targets while the terminal continues using the bespoke renderer.
