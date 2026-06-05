---
last_updated: "2026-05-17"
---

# Challenges of Migrating the `Todo` Component to the Tree Rendering Architecture

## Functional and Design Goals

The `Todo` component was created to provide a terminal-aware, stateful task-item
rendering primitive for the biscuit-terminal library. Its core design goals are:

1. **Multi-state representation** -- a single task item can be in one of five
   states (`Open`, `InProgress`, `Completed`, `Cancelled`, `Blocked`), each
   with a distinct visual representation.
2. **Terminal-capability adaptation** -- rendering adapts to three capability
   tiers: Nerd Font glyphs (icon-based), colored ASCII fallbacks, and plain
   ASCII without any ANSI escape codes.
3. **Prose-aware descriptions** -- the description can optionally be rendered
   through the `Prose` component, enabling inline markup like `<b>bold</b>`
   and `<red>color</red>` within the task text.
4. **Composable layout** -- like all `TerminalRenderable` components, `Todo`
   owns a `Layout` for margins, alignment, and word-wrap, allowing it to be
   composed into larger structures (`Compose`, `UnorderedList`, etc.).

### Where `Todo` Is Used Today

`Todo` is exported from `biscuit_terminal::prelude` and is available as a
building block for any CLI tool in the Rusty Biscuit ecosystem that needs to
display task or checklist items. It does not currently have a dedicated `bt`
CLI subcommand, and no external consumer outside of `biscuit-terminal` itself
uses it directly -- it exists as a library component intended for composition.

Potential and current usage contexts include:

- **Task progress reports** rendered by agentic CLI tools (showing what work
  has been done, what is in progress, what is blocked)
- **Checklist summaries** composed inside `Compose` or `Section` components
- **Feature-planning documents** rendered through the darkmatter pipeline,
  where each feature milestone could be a `Todo` item

### Example Usage

```rust
use biscuit_terminal::prelude::*;

let todo_open = Todo::new("Review pull request #42");
let todo_done = Todo::from_prose("<green>Merge</green> feature branch")
    .with_state(TodoState::Completed);

println!("{}", todo_open.display(&term));
println!("{}", todo_done.display(&term));
```

Rendering output depends on the terminal's detected capabilities:

| Terminal         | Open                  | Completed                | Cancelled                |
|------------------|-----------------------|--------------------------|--------------------------|
| Nerd Font + color| `⬜ Review ...`       | `✓ Merge feature branch` | `[dim] ⃠ Dropped task`   |
| ASCII + color    | `[ ] Review ...`      | `[✔] Merge feature...`   | `[dim] [-] Dropped task` |
| No color         | `[ ] Review ...`      | `[x] Review ...`         | `[-] Dropped task`       |

## Technical Implementation (current)

### Code Structure

`Todo` is defined in `biscuit-terminal/lib/src/components/todo.rs`. Its key
types are:

| Type | Purpose |
|------|---------|
| `TodoState` | 5-variant enum: `Open`, `InProgress`, `Completed`, `Cancelled`, `Blocked` |
| `TodoStateRep` | Pairs a Nerd Font glyph with an ASCII fallback string |
| `TODO_CHAR_LOOKUP` | `LazyLock<HashMap<TodoState, TodoStateRep>>` mapping each state to its representations |
| `Todo` | The component struct |

### Fields of `Todo`

```text
Todo {
    state: TodoState,
    description: String,
    created: DateTime<Utc>,
    last_updated: DateTime<Utc>,
    use_prose: bool,
    layout: Layout,
}
```

The `created` and `last_updated` timestamps are serde-serializable, enabling
`Todo` items to be persisted and restored. The `use_prose` flag and `layout`
are `#[serde(skip)]` because they are runtime-only concerns.

### Rendering Pipeline

The `TerminalRenderable` implementation delegates to `Todo::to_terminal(&Terminal)`:

```text
┌──────────────────────────────────────────────────────────┐
│ render() / render_optimistic()                           │
│                                                          │
│  1. to_terminal(term)                                    │
│     ├─ Look up TodoStateRep via TODO_CHAR_LOOKUP         │
│     ├─ If use_prose: render description via Prose        │
│     ├─ Determine color support (ColorDepth::None?)       │
│     ├─ Select fallback icon (colored vs plain)           │
│     ├─ Match on state:                                   │
│     │   ├─ Cancelled: Dim + Strikethrough (if color)     │
│     │   └─ Others: Icon + description                    │
│     └─ Match on is_nerd_font:                            │
│         ├─ Some(true): use .nerd glyph                   │
│         └─ _ : use .fallback string                      │
│                                                          │
│  2. layout.apply_layout(content, term_width)             │
│     └─ Applies margins, alignment, word-wrap             │
└──────────────────────────────────────────────────────────┘
```

### Key Responsibilities

1. **State-to-glyph resolution** -- maps `TodoState` to the correct Nerd Font
   glyph or ASCII fallback via `TODO_CHAR_LOOKUP`.
2. **Three-tier fallback** -- color-capable terminals get ANSI-colored fallbacks
   (e.g., `[✔]` in green); no-color terminals get plain ASCII (e.g., `[x]`);
   Nerd Font terminals always get the glyph.
3. **Cancelled-state styling** -- applies `FontWeight::Dim` and
   `Style::Strikethrough` wrapping (when color is available).
4. **Prose delegation** -- when `use_prose` is true, the description is rendered
   through `Prose::new(&self.description).render(term)`, which performs its own
   token-to-ANSI conversion.
5. **Layout application** -- the final string is passed through
   `Layout::apply_layout()` for margins, alignment, and word-wrap.

## Implementation Challenges

### Challenges

#### No First-Class "Todo" Node Kind

The `NodeKind` enum has 25 variants. None represents a "todo item" or "task
item." The closest existing variant is `ListItem { checked: Option<bool> }`,
which models GFM task-list items (`[ ]` / `[x]`). However, `ListItem` has only
a boolean `checked` field, while `Todo` has five states. There is no way to
represent `InProgress` or `Blocked` in the current tree model.

**Example:** A `Todo` in state `InProgress` needs to project into the tree. The
natural `ListItem` node can express "checked" or "unchecked," but cannot
express "in progress" -- the renderer would have to guess or discard the
state.

**Suggested test:**

```rust
#[test]
fn in_progress_todo_projects_to_tree_without_losing_state() {
    let todo = todo_with_state("Working on it", TodoState::InProgress);
    let node = todo.render_tree_node().expect("tree projection");
    // How do we recover TodoState::InProgress from the node?
    // This test should assert the state is preserved, not collapsed to checked/unchecked.
    assert_ne!(
        node.kind,
        NodeKind::ListItem { checked: Some(true), children: vec![] }
    );
}
```

#### Nerd Font Glyph vs. ASCII Fallback Decision Belongs to the Terminal Renderer

In the current bespoke implementation, `to_terminal()` queries `term.is_nerd_font`
and `term.color_depth` to decide which representation to emit. The tree
architecture deliberately moves terminal-specific concerns out of the projection
step and into the terminal renderer. But `Todo`'s Nerd Font glyphs are baked
into the component, not into the renderer.

**Example:** If `Todo::render_tree()` emits a `Text` node containing the Nerd
Font glyph, the Markdown renderer will produce a document with a glyph that
renders as a missing-character box outside of Nerd Font terminals. If it emits
the ASCII fallback, the terminal renderer cannot upgrade to the glyph even when
the terminal supports Nerd Fonts.

**Suggested test:**

```rust
#[test]
fn tree_todo_renders_correctly_in_both_nerd_and_ascii_terminals() {
    let todo = Todo::new("Task");
    let node = todo.render_tree_node().expect("tree projection");

    let nerd_term = Terminal::builder().nerd_font(true).build();
    let ascii_term = Terminal::builder().nerd_font(false).build();

    let nerd_output = render_terminal_node(&node, &TerminalRenderOptions::new(&nerd_term, RenderStrictness::Lossy)).unwrap().output;
    let ascii_output = render_terminal_node(&node, &TerminalRenderOptions::new(&ascii_term, RenderStrictness::Lossy)).unwrap().output;

    assert!(nerd_output.contains('\u{f0131}'), "Nerd font terminal should use the glyph");
    assert!(!ascii_output.contains('\u{f0131}'), "ASCII terminal should not use the glyph");
}
```

#### State-Specific Styling Requires Renderer Awareness

The `Cancelled` state applies two styling transforms: `FontWeight::Dim` and
`Style::Strikethrough`. In the tree model, `Delete` (strikethrough) is a
`NodeKind` variant, and `Emphasis`/`Strong` cover italic/bold, but there is no
"dim" node kind. The renderer would need an extension mechanism to apply dim
styling to a subtree.

**Example:** A cancelled todo projects into the tree. Its description should be
both dimmed and struck-through. The tree can express strikethrough via
`Delete`, but dim requires either a new `NodeKind` variant, a `Span` with a
semantic class, or a renderer-specific hint.

**Suggested test:**

```rust
#[test]
fn cancelled_todo_projects_dim_and_strikethrough_into_tree() {
    let todo = todo_with_state("Dropped task", TodoState::Cancelled);
    let node = todo.render_tree_node().expect("tree projection");

    // The tree should encode both dim and strikethrough semantics.
    // At minimum, the description must be wrapped in Delete for strikethrough.
    let rendered = render_terminal_node(
        &node,
        &TerminalRenderOptions::new(&color_terminal(), RenderStrictness::Lossy),
    )
    .unwrap()
    .output;

    let stripped = strip_ansi(&rendered);
    assert!(stripped.contains("Dropped task"));
    // Verify strikethrough escape codes are present when color is supported
    assert!(rendered.contains("\x1b[9m"), "expected strikethrough escape code");
}
```

#### Color-Dependent Fallback Icons Differ Across States

Each `TodoState` has a different colored fallback icon (e.g., `[✔]` in green
for `Completed`, `[!]` in bright red for `Blocked`). These color choices are
semantic -- green signals completion, red signals a problem. The tree is
target-agnostic and does not carry color information. The terminal renderer
would need a way to recover the correct color for each state.

**Example:** A `Blocked` todo should render with a red `[!]` icon in a
color-capable terminal, but the tree projection cannot embed
`BasicColor::BrightRed` as a node attribute in the current architecture.

**Suggested test:**

```rust
#[test]
fn blocked_todo_uses_red_color_in_fallback_icon() {
    let todo = todo_with_state("Blocked task", TodoState::Blocked);
    let node = todo.render_tree_node().expect("tree projection");

    let rendered = render_terminal_node(
        &node,
        &TerminalRenderOptions::new(&color_terminal(), RenderStrictness::Lossy),
    )
    .unwrap()
    .output;

    // Should contain red ANSI escape codes around the icon
    assert!(rendered.contains("\x1b[91m") || rendered.contains("\x1b[1;31m"),
        "Blocked icon should be bright red");
}
```

#### Prose Descriptions Introduce Nested Rendering

When `use_prose` is true, the description is rendered through the `Prose`
component, which performs its own token-to-ANSI conversion. In the tree
architecture, the projection step should produce tree nodes, not ANSI strings.
But `Prose` currently only implements `TerminalRenderable`, not
`TreeRenderable`, so it cannot be projected into the tree.

**Example:** `Todo::from_prose("<b>Important</b> task")` -- the `<b>` token
should survive into the tree as a `Strong` node wrapping "Important," but
`Prose` does not produce `RenderNode` trees.

**Suggested test:**

```rust
#[test]
fn prose_todo_projects_inline_styles_into_tree() {
    let todo = Todo::from_prose("<b>Critical</b> task");
    let node = todo.render_tree_node().expect("tree projection");

    // The tree should contain a Strong node, not a raw <b> tag or ANSI codes.
    let has_strong = find_node_kind(&node, |kind| matches!(kind, NodeKind::Strong { .. }));
    assert!(has_strong, "Prose bold should project as a Strong node");
}

fn find_node_kind(node: &RenderNode, predicate: impl Fn(&NodeKind) -> bool) -> bool {
    if predicate(&node.kind) {
        return true;
    }
    node.children().iter().any(|c| find_node_kind(c, &predicate))
}
```

#### Layout Application Must Survive the Tree Round Trip

The bespoke implementation applies `Layout` (margins, alignment, word-wrap) via
`layout.apply_layout()` after generating the content string. The tree
architecture applies layout from `NodeAttrs::layout()` during the render step.
For `Todo`, the component's `Layout` must be seeded onto the projected node so
that the tree renderer applies it faithfully.

**Example:** A `Todo` with `left_margin(4)` must have its content indented by
four cells whether rendered through the bespoke path or the tree path.

**Suggested test:**

```rust
#[test]
fn todo_layout_survives_tree_round_trip() {
    let todo = Todo::new("Task with margin")
        .left_margin(TargetValue::universal(Length::ch(4)));

    // Bespoke path
    let bespoke = todo.render_optimistic(Some(80));

    // Tree path
    let node = todo.render_tree_node().expect("tree projection");
    let tree = render_terminal_node(
        &node,
        &TerminalRenderOptions::new(
            &Terminal::new_optimistic(80),
            RenderStrictness::Lossy,
        ),
    )
    .unwrap()
    .output;

    // Both paths should have at least 4 spaces of left margin
    assert!(bespoke.starts_with("    "));
    assert!(tree.starts_with("    "));
}
```

#### Parity Gate for Non-Boolean State Representation

The existing component parity gate pattern (established by `BlockQuote`) renders
a component both ways and compares semantic invariants. For `Todo`, the parity
test must account for the fact that the tree path may represent states
differently (e.g., via hints) than the bespoke path (which embeds glyphs
directly). This means the parity assertion cannot simply compare stripped strings
-- it must verify that the *semantic state* is recoverable from both outputs.

**Example:** A `Todo` in state `InProgress` renders as `[▶] Working` (bespoke)
or as a `ListItem` with a custom hint. The parity test needs to verify the
state is present and correct in both representations.

**Suggested test:**

```rust
#[test]
fn todo_parity_across_bespoke_and_tree_paths() {
    for state in [
        TodoState::Open,
        TodoState::InProgress,
        TodoState::Completed,
        TodoState::Cancelled,
        TodoState::Blocked,
    ] {
        let todo = todo_with_state("Test task", state);

        let bespoke = strip_ansi(&todo.render_optimistic(Some(80)));
        let node = todo.render_tree_node().expect("projection");
        let tree = strip_ansi(
            &render_terminal_node(
                &node,
                &TerminalRenderOptions::new(
                    &Terminal::new_optimistic(80),
                    RenderStrictness::Lossy,
                ),
            )
            .unwrap()
            .output,
        );

        // Both outputs should contain the task description
        assert!(bespoke.contains("Test task"), "bespoke missing description for {state:?}");
        assert!(tree.contains("Test task"), "tree missing description for {state:?}");

        // Both outputs should have a state indicator (not just empty)
        assert!(!bespoke.trim().is_empty(), "bespoke empty for {state:?}");
        assert!(!tree.trim().is_empty(), "tree empty for {state:?}");
    }
}
```

#### Cross-Target Rendering Consistency (Markdown, Browser, Terminal)

The tree architecture promises "parse once, build one tree, walk it per target."
A `Todo` projected into the tree must produce sensible output in all three
renderers. Today `Todo` only knows about terminal rendering -- it has no concept
of what a todo item looks like in Markdown or HTML.

**Example:** A `Todo` in state `Blocked` should render as:

- **Terminal**: `[!] Blocked task` (with red coloring)
- **Markdown**: `- [ ] Blocked task` or `- [!] Blocked task` (GFM only has
  `[ ]` / `[x]`)
- **Browser**: `<li class="todo blocked"><input disabled type="checkbox">Blocked task</li>`

**Suggested test:**

```rust
#[test]
fn blocked_todo_renders_sensibly_in_all_targets() {
    let todo = todo_with_state("Blocked task", TodoState::Blocked);
    let node = todo.render_tree_node().expect("projection");

    // Markdown target
    let md = render_markdown_node(&node, &MarkdownRenderOptions::default())
        .unwrap()
        .output;
    assert!(md.contains("Blocked task"), "Markdown should contain description");

    // Browser target
    let html = render_browser_node(&node, &BrowserRenderOptions::default())
        .unwrap()
        .output;
    assert!(html.contains("Blocked task"), "HTML should contain description");

    // Terminal target
    let term = render_terminal_node(
        &node,
        &TerminalRenderOptions::new(
            &Terminal::new_optimistic(80),
            RenderStrictness::Lossy,
        ),
    )
    .unwrap()
    .output;
    assert!(term.contains("Blocked task"), "Terminal should contain description");
}
```

## Solution Suggestions

#### Extend `ListItem` with a Rich State Hint

**Solution:** Rather than adding a new `NodeKind` variant for `Todo`, extend the
existing `ListItem` node to carry a `TodoState` value via `NodeAttrs` hints.
Define a new `TodoHints` struct (analogous to `ProgressHints` or
`ColumnsHints`) with fields for the state, and register it under a new
`HintNamespace::WIDGET_TODO` namespace. The `checked` field on `ListItem` would
be mapped from the state: `Completed` maps to `Some(true)`, `Open` to
`Some(false)`, and all other states map to `None` with the actual state stored
in the hint.

**Which challenges this helps with:**

- **No First-Class "Todo" Node Kind** -- avoids adding a 26th variant by reusing
  `ListItem` as the structural carrier and encoding the extended state in hints.
  The terminal renderer checks for todo hints on a `ListItem` node; if present,
  it renders using the full 5-state logic rather than the binary checked/unchecked.
- **Parity Gate for Non-Boolean State** -- the hint carries the original
  `TodoState`, so the parity test can extract and compare it directly from the
  tree node.
- **Cross-Target Rendering Consistency** -- the Markdown renderer falls back to
  the `checked` boolean for GFM compatibility, while the browser renderer can
  emit richer markup using the hint data.

**Variant solutions:**

1. Add a new `NodeKind::TaskItem { state: TaskState, children }` variant instead
   of using hints. This is more explicit but adds a variant that every renderer
   must handle, and couples the canonical tree to a specific semantic concept.
2. Use a `Span` with semantic classes (e.g., `class="todo in-progress"`) to
   carry the state, keeping the structure as a standard `ListItem`.

#### Introduce a `WIDGET_TODO` Hint Namespace with State-Aware Rendering Hints

**Solution:** Define `HintNamespace::WIDGET_TODO` and a `TodoHints` struct that
carries all terminal-specific rendering data: the `TodoState`, the Nerd Font
glyph, the colored fallback string, and the no-color fallback string. The
terminal renderer checks for these hints on `ListItem` nodes; when present, it
uses them to produce the correct icon, color, and styling. When absent, it
falls back to the standard checked/unchecked rendering.

```rust
pub struct TodoHints {
    pub state: String,           // "open", "in_progress", "completed", "cancelled", "blocked"
    pub nerd_glyph: Option<String>,
    pub color_fallback: Option<String>,
    pub no_color_fallback: Option<String>,
    pub dim: bool,               // true for cancelled
    pub strikethrough: bool,     // true for cancelled
}
```

**Which challenges this helps with:**

- **Nerd Font Glyph vs. ASCII Fallback Decision** -- the hints carry all three
  representations, and the terminal renderer selects the correct one based on
  `term.is_nerd_font` and `term.color_depth`, exactly where that decision
  belongs.
- **State-Specific Styling Requires Renderer Awareness** -- the `dim` and
  `strikethrough` flags in the hints tell the terminal renderer exactly which
  styling transforms to apply, without requiring new `NodeKind` variants.
- **Color-Dependent Fallback Icons** -- the `color_fallback` hint can include
  the ANSI-colored string (e.g., the green `[✔]`), while `no_color_fallback`
  provides the plain ASCII version. The renderer selects based on color depth.

**Variant solutions:**

1. Store only the `TodoState` enum value in the hint and let the terminal
   renderer own the full mapping (icons, colors, styles). This keeps hints
   smaller but duplicates the `TODO_CHAR_LOOKUP` logic in the renderer.
2. Use a generic "semantic class" system (e.g., `class="todo-state-cancelled"`)
   and have each renderer maintain a style mapping for known classes.

#### Bridge `Prose` into the Tree via `TreeRenderable`

**Solution:** Implement `TreeRenderable` for `Prose` so that it projects its
token-parsed inline content into proper `NodeKind` nodes (`Strong`, `Emphasis`,
`Text`, `Span`, etc.). Then, during `Todo::render_tree_node()`, if `use_prose`
is true, delegate to `Prose::render_tree()` to produce the description subtree
rather than calling `Prose::render()` (which produces ANSI strings).

**Which challenges this helps with:**

- **Prose Descriptions Introduce Nested Rendering** -- this directly solves the
  problem by giving `Prose` a tree projection path. The `Todo` component can
  then include rich inline content in its tree output.
- **Cross-Target Rendering Consistency** -- a `Prose`-produced `Strong` node
  renders as `**bold**` in Markdown, `<strong>bold</strong>` in the browser,
  and ANSI bold in the terminal, achieving true cross-target fidelity.

**Variant solutions:**

1. Parse the Prose markup during `Todo::render_tree_node()` inline, without
   making `Prose` itself `TreeRenderable`. This is faster to implement but
   duplicates parsing logic.
2. Use a `CodeRenderer`-style hook where the `Todo` component receives a
   callback that converts Prose text to `RenderNode` children. This keeps
   `Prose` unaware of the tree but adds complexity to the call site.

#### Seed Component Layout onto the Projected Node

**Solution:** Follow the pattern established by `BlockQuote` and other Group 1
components: in `render_tree_node()`, compare the component's layout against its
default. If customized, call `node.attrs.set_layout(&self.layout)` to store it
on the projected node. The terminal renderer already reads and applies
`NodeAttrs::layout()` for block nodes.

**Which challenges this helps with:**

- **Layout Application Must Survive the Tree Round Trip** -- this directly
  mirrors the proven pattern. The tree renderer resolves the layout from the
  node's attributes and applies margins, alignment, and word-wrap during the
  render walk.

**Variant solutions:**

1. Use `TreeRenderable::tree_layout()` instead of embedding in `NodeAttrs`.
   This is simpler for the component but requires the adapter to merge the
   layout into the node. Both approaches are viable; the `NodeAttrs` approach
   is consistent with what `BlockQuote` and `Progress` already do.
2. Apply layout as a post-processing step in `TreeComponent` rather than in
   the renderer. This would centralize layout logic but bypass the renderer's
   width-awareness.

#### Establish a Per-State Parity Gate Pattern

**Solution:** Create a parameterized parity test that iterates over all five
`TodoState` variants, renders each through both the bespoke `TerminalRenderable`
and the tree path (`TreeComponent` + `render_terminal_node`), and asserts
semantic equivalence on the stripped output. This extends the parity discipline
established by `BlockQuote` to a multi-state component.

**Which challenges this helps with:**

- **Parity Gate for Non-Boolean State** -- the parameterized test ensures every
  state is verified, not just happy paths.
- **All other challenges** -- the parity gate is the mechanism by which every
  other challenge is *validated*. Without it, we cannot confirm that the tree
  path faithfully reproduces the bespoke behavior.

**Variant solutions:**

1. Use snapshot testing (e.g., `insta`) for each state's output, comparing
   bespoke vs. tree snapshots rather than inline assertions. This is more
   resilient to minor formatting changes but requires snapshot review.
2. Use property-based testing (`proptest`) to generate arbitrary `Todo`
   configurations (states, descriptions with/without Prose, various layouts)
   and assert that both paths produce output containing the description text.
