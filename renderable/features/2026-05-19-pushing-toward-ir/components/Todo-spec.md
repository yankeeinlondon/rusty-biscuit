# Todo — IR Rendering Design Specification

## Component Status

| Field      | Value                                                                             |
|------------|-----------------------------------------------------------------------------------|
| Name       | Todo                                                                              |
| Kind       | Terminal component that currently reports `is_block_level() -> false`; tree output is a one-item `List` |
| Location   | `biscuit-terminal/lib/src/components/todo.rs`                                     |
| Terminal   | ✅ bespoke `TerminalRenderable`                                                    |
| Browser    | ❌                                                                                 |
| Markdown   | ❌                                                                                 |
| Tree       | ❌                                                                                 |
| IR State   | reviewed; requires typed task-state hints for full terminal parity                |
| bt CLI     | —                                                                                 |

Todo represents a task item with five visual states: Open, InProgress, Completed,
Blocked, and Cancelled. Each state maps to a Nerd Font glyph (when detected) or an
ASCII fallback with optional color. Cancelled applies dim + strikethrough. The
description may optionally be rendered through `Prose` (via `use_prose` flag). The
component also carries `created` and `last_updated` timestamps.

---

## Design Steps

### Terminal IR Implementation

- The **Todo** component does not currently have a IR based rendering solution
- This section will describe what is required to ensure that the **Todo** component:
    - has an IR implementation
    - the IR implementation drives the TerminalRenderable contract
    - the IR implementation is what is used by the bt CLI (note if **Todo** doesn't yet have bt CLI subcommand then it will be designed below in the bt CLI section)

#### Tree Projection Design

Todo maps most naturally to a single `NodeKind::List` containing one
`NodeKind::ListItem` with `checked: Option<bool>`, which is the GFM task-list
representation. A bare `ListItem` is structurally invalid because tree validation
requires `ListItem` nodes to appear directly inside a `List`.

Todo has five states while `ListItem` only carries a binary checkbox. The projection
strategy is:

| TodoState    | `checked`   | CSS class            | Style treatment                                     |
|--------------|-------------|----------------------|-----------------------------------------------------|
| Open         | `Some(false)` | `todo-open`         | No emphasis                                         |
| InProgress   | `Some(false)` | `todo-in-progress`  | Green color on the checkbox glyph                   |
| Completed    | `Some(true)`  | `todo-completed`    | Green color on the checkbox glyph                   |
| Blocked      | `Some(false)` | `todo-blocked`      | Red color on the checkbox glyph                     |
| Cancelled    | `Some(false)` | `todo-cancelled`    | Dim emphasis + strikethrough on description text    |

The `checked` field is set to `Some(true)` only for `Completed`; all other states
use `Some(false)` so Markdown and Browser output remain valid unchecked task-list
items. The extended state is carried through stable `todo-*` classes for CSS hooks
and a typed task-state render hint for renderers that need state fidelity.

**Implementation:**

```rust
impl TreeRenderable for Todo {
    fn render_tree(&self) -> RenderNode {
        let state_class = match self.state {
            TodoState::Open => "todo-open",
            TodoState::InProgress => "todo-in-progress",
            TodoState::Completed => "todo-completed",
            TodoState::Blocked => "todo-blocked",
            TodoState::Cancelled => "todo-cancelled",
        };

        let checked = Some(matches!(self.state, TodoState::Completed));

        let desc_text = if self.use_prose {
            prose_to_plain_text(&self.description)
        } else {
            self.description.clone()
        };

        let desc_node = match self.state {
            TodoState::Cancelled => RenderNode::delete(vec![RenderNode::text(desc_text)]),
            _ => RenderNode::text(desc_text),
        };

        let mut item = RenderNode::list_item(checked, vec![
            RenderNode::paragraph(vec![desc_node]),
        ]);
        item.attrs.classes = vec![state_class.to_string()];
        item.attrs.set_task_hints(&TaskHints {
            state: TaskState::from(self.state.clone()),
        });

        if self.layout != Layout::default() {
            item.attrs.set_layout(&self.layout);
        }

        if let Some(style) = self.state_style() {
            item.attrs.set_style(&style);
        }

        let mut list = RenderNode::list(false, None, vec![item]);
        list.attrs.set_list_hints(&ListRenderHints {
            bullet: Some(String::new()),
            hanging_indent: true,
            indent_children: Some(0),
        });
        list
    }
}
```

The `state_style()` helper returns a `Style` with state-appropriate emphasis:

- **Cancelled**: `TextEmphasis { dim: true, strikethrough: true, .. }`
- **Other states**: No style on the `ListItem` itself. The checkbox glyph styling
  is handled by the terminal tree renderer from typed task-state hints.

The list-level `ListRenderHints` use an empty terminal bullet so terminal output
matches the current Todo contract (`[ ] description`) instead of gaining a normal
list marker (`- [ ] description`). Markdown ignores list hints by design and still
serializes valid GFM task-list syntax.

The legacy `TerminalRenderable::render_tree_node()` compatibility hook, if kept,
must delegate to the same projection helper as `TreeRenderable::render_tree()` so
CLI, terminal, browser, and Markdown paths cannot drift.

#### Checkbox Glyph Rendering

The terminal tree renderer already handles `NodeKind::ListItem` with `checked`
states, rendering `[x]` or `[ ]` after the list prefix. However, Todo needs richer
checkbox glyphs:

| State      | Nerd Font glyph         | Fallback (color)     | Fallback (no color) |
|------------|-------------------------|----------------------|---------------------|
| Open       | `\u{f0131}` (box outline) | `[ ]`              | `[ ]`               |
| InProgress | `\u{f0856}` (intermediate)| `[⏺]` (green)     | `[>]`               |
| Completed  | `\u{f4a7}` (checkmark)    | `[✔]` (green)      | `[x]`               |
| Blocked    | `\u{f0117}` (badge)       | `[⏺]` (red)        | `[!]`               |
| Cancelled  | `\u{f12ed}` (box off)     | `[-]` (dim)        | `[-]`               |

The existing terminal tree renderer's `ListItem` handler produces only `[x]` and
`[ ]`. To preserve Todo's rich glyph rendering, the tree renderer needs typed
task-state hints on `ListItem` and state-specific checkbox rendering. This is
detailed in the Feature Requests section below.

An alternative is to encode the checkbox glyph into the node's visible text, but
this would pollute the semantic structure and produce incorrect Markdown output
(the checkbox glyph would appear in the text content).

#### Layout Mapping

Todo already owns a `Layout` with margins and alignment. The tree projection seeds
this onto the `ListItem` node via `attrs.set_layout(&self.layout)`, because the
item is the component's visible block. The terminal tree renderer's
`render_with_layout` applies it.

Todo is a single-line component; only `margin` and `alignment` are relevant.
`max_width` is Browser-only and `word_wrap` does not apply to a single-line item.

#### Style Mapping

The `Style` on the projected node carries state-dependent emphasis:

- **Cancelled**: `TextEmphasis { dim: true, strikethrough: true, .. }` — the
  terminal renderer lowers these to SGR escapes.
- **Other states**: No `Style` on the node. The checkbox glyph's color (green for
  InProgress/Completed, red for Blocked) is handled by the terminal tree renderer's
  extended checkbox rendering, not by the generic `Style` system. The color is
  widget-specific (applied to the glyph only, not the description text), so it
  does not map to `Style.color`.

#### Parity Test Strategy

Critical test variants for the IR vs bespoke comparison:

| Variant                                                       | Validates                                                                        |
|---------------------------------------------------------------|----------------------------------------------------------------------------------|
| Open state                                                    | `[ ]` glyph present, no ANSI styling                                             |
| Completed state                                               | `[x]` (fallback) or checkmark glyph, green color on glyph                       |
| InProgress state                                              | `[>]` (no-color) or `[⏺]` (color), green on glyph                               |
| Blocked state                                                 | `[!]` (no-color) or `[⏺]` (color), red on glyph                                 |
| Cancelled state                                               | `[-]` glyph, dim + strikethrough on description text                             |
| Plain description                                             | Text content identical in both paths                                             |
| Prose description (`use_prose = true`)                        | Styled text survives (or accepted loss of inline styles documented)              |
| Nerd Font terminal                                            | Nerd Font glyphs used in both paths                                              |
| No-color terminal (`ColorDepth::None`)                        | ASCII fallbacks, no ANSI escapes                                                 |
| TrueColor terminal                                            | Full color escapes present                                                       |
| Left margin applied                                           | Layout margin prefixes the line                                                  |
| Right margin applied                                          | Available width narrowed                                                         |
| Center alignment                                              | Item centered within available width                                             |
| Empty description                                             | Only glyph rendered, no text after                                               |
| Description with special characters                           | Content preserved without mangling                                               |

Parity is asserted on **ANSI-stripped content** (not byte-identical output), with
a `KNOWN_DRIFT` ledger documenting accepted divergences:

- **Checkbox glyph format**: The tree path may produce slightly different spacing
  between the checkbox glyph and the description text. After ANSI stripping, the
  text content must be identical.
- **Prose styling loss**: If `use_prose` is true, the tree path may flatten inline
  styles during projection (same loss as BlockQuote's text extraction). The plain
  text must be identical; styling divergence is documented.
- **Layout application**: Tree path uses `render_with_layout`, bespoke path uses
  `LayoutTerminalExt::apply_layout`. Content semantics must match.

#### Feature Requests for Tree Rendering

##### Feature 1: Extended Task State Checkbox Rendering via `todo-*` Classes

**DENIED**

this feature will not be added to the render-tree tree implementation. You should
try to still use the render-tree where practical and work around the complexity
but if the complexity is too great then you have permission to create a bespoke IR
implementation for this component.

Why: class names are an appropriate browser/CSS hook, but they are too fragile as
the primary terminal rendering contract. The render tree already uses typed hint
payloads for component semantics that affect renderer behavior (`ProgressHints`,
`ListRenderHints`, table hints). Todo should follow that pattern with typed
task-state hints instead of requiring the terminal renderer to parse CSS class
strings.

**What it looks like:**

The terminal tree renderer's `NodeKind::ListItem` handler gains awareness of
`todo-*` CSS classes on the node. When a `ListItem` carries a recognized todo
state class, the checkbox rendering uses the Todo-specific glyph set instead of
the default `[x]` / `[ ]`:

```rust
// In the terminal tree renderer's ListItem handling:
fn render_list_item_checkbox(&self, node: &RenderNode, term: &Terminal) -> String {
    let state_class = node.attrs.classes.iter().find(|c| c.starts_with("todo-"));

    if let Some(class) = state_class {
        match class.as_str() {
            "todo-open" => self.render_todo_checkbox(TodoGlyphState::Open, term),
            "todo-in-progress" => self.render_todo_checkbox(TodoGlyphState::InProgress, term),
            "todo-completed" => self.render_todo_checkbox(TodoGlyphState::Completed, term),
            "todo-blocked" => self.render_todo_checkbox(TodoGlyphState::Blocked, term),
            "todo-cancelled" => self.render_todo_checkbox(TodoGlyphState::Cancelled, term),
            _ => self.render_default_checkbox(node, term),
        }
    } else {
        self.render_default_checkbox(node, term)
    }
}
```

The glyph state logic reuses the existing `TODO_CHAR_LOOKUP` static and the
Nerd Font / fallback / no-color branching that already lives in `todo.rs`.

**Why Todo needs it:**

Without this feature, the terminal tree renderer can only produce `[x]` / `[ ]`
for `ListItem` nodes, which loses three of Todo's five states (InProgress, Blocked,
Cancelled). The `checked: Option<bool>` field is insufficient to carry the full
state semantics.

**Impact of not having this feature:**

Without it, the Todo component would need to bypass the `ListItem` node kind
entirely and use a `NodeKind::Paragraph` or `NodeKind::Span` with the checkbox
glyph embedded in the text. This would:

1. Lose the semantic structure (the node would not be recognizable as a task item)
2. Produce incorrect Markdown output (no `- [x]` / `- [ ]` syntax)
3. Prevent Todo items from participating in list rendering (nesting, bullet
   prefix, etc.)

The alternative would be a component-local bespoke IR, which defeats the purpose
of using the shared tree renderer.

##### Feature 2: Typed task-state hints on `NodeAttrs`

**APPROVED**

this feature request has been approved and WILL be included as part of the
render-tree implementation BEFORE you are asked to implement this solution. Always
refer to the @renderable/docs/tree-rendering.md and
@renderable/docs/layout-and-style.md documents as the definitive guide.

**What it looks like:**

A task hint struct carried on `NodeAttrs`, similar to `ProgressHints`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskState {
    Open,
    InProgress,
    Completed,
    Blocked,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskHints {
    pub state: TaskState,
}
```

Store the hint under a new `renderable.widget.task` namespace with typed
`NodeAttrs::set_task_hints(&TaskHints)` and `NodeAttrs::task_hints() ->
Option<TaskHints>` helpers. Validation must reject task hints on any node other
than `NodeKind::ListItem`.

The terminal tree renderer reads `TaskHints` to select Todo's existing
Nerd Font / colored fallback / no-color fallback checkbox marker. The renderer
must apply the custom marker only to the checkbox marker, not the description
text, except for `Cancelled`, where the projected `Delete` node and/or declared
`Style` continues to handle description strikethrough/dim behavior.

Browser rendering does not need special logic for the first implementation:
`checked: Some(false)` / `Some(true)` already emits a disabled checkbox, and
`todo-*` classes remain available for CSS. Markdown and MarkdownPlus deliberately
degrade to `- [ ]` / `- [x]` task-list syntax while preserving cancelled
strikethrough.

Do not add `chrono` or `biscuit-terminal` component types to `renderable` for this
feature. The `created` and `last_updated` fields remain component metadata until a
cross-target metadata display contract is designed.

**Why Todo would benefit:**

Parsing CSS classes for rendering logic is fragile. A dedicated hint struct
provides type-safe access to the full state and keeps renderer behavior explicit
and testable.

**Impact of not having this feature:**

High for terminal parity. Without typed hints, the tree path can still produce
valid Markdown and Browser task-list output, but terminal rendering cannot
faithfully preserve InProgress, Blocked, or Cancelled checkbox markers without
either class parsing or embedding glyphs into visible text.

#### Tree Renderer Fit Assessment

The tree renderer is a **good fit** for Todo with Feature 2 (typed task-state
hints). A one-item `NodeKind::List` containing a `NodeKind::ListItem` is the
semantically correct representation for a task item, and the existing list
rendering infrastructure handles indentation and task-list syntax.

Without Feature 2, the tree renderer would be a **partial fit** because it cannot
represent three of Todo's five terminal visual states. The component would be
forced to either accept terminal marker drift or embed glyphs in text, both of
which degrade the output across targets.

The tree renderer is the recommended approach because:
1. `List` / `ListItem` is the correct semantic shape for a task item
2. The Feature 2 extension is a small, well-scoped typed hint addition to the
   existing `ListItem` handler
3. The same tree serves all three targets (Terminal, Browser, Markdown) from a
   single projection
4. The component is simple (single line, no nesting) so the tree overhead is
   minimal

`will_use_tree_renderer`: **false** — without Feature 2, the tree renderer cannot
faithfully render InProgress, Blocked, or Cancelled terminal markers, making it an
unacceptable default for replacing `TerminalRenderable::render()`.

`will_use_tree_renderer_with_features`: **true** — with Feature 2 (typed
task-state hints), the tree renderer handles all five Todo states
correctly and is the recommended path.

---

### Browser IR Implementation

- In this section we will provide a design specification for the **Todo** component's implementation of the BrowserRenderable trait

Todo does not currently have a bespoke browser rendering implementation. Since
Terminal IR is designed first and Todo projects to a one-item `NodeKind::List`
with `todo-*` classes on its `ListItem`, browser rendering handles the node
through the tree.

#### Browser Rendering Design

The browser tree renderer currently handles `NodeKind::ListItem` as `<li>` with an
optional `<input type="checkbox">`. For a `ListItem` with `checked: Some(true)`,
it renders `<input type="checkbox" checked disabled>`. For `checked: Some(false)`,
it renders `<input type="checkbox" disabled>`. Todo uses `Some(false)` for every
non-completed state, so all five states render as task-list items.

For Todo's extended states, the browser rendering uses the `todo-*` classes to
produce richer output:

| State      | HTML output                                                                                   |
|------------|-----------------------------------------------------------------------------------------------|
| Open       | `<li class="todo-open"><input type="checkbox" disabled> description</li>`                     |
| InProgress | `<li class="todo-in-progress"><input type="checkbox" disabled> description</li>`              |
| Completed  | `<li class="todo-completed"><input type="checkbox" checked disabled> description</li>`        |
| Blocked    | `<li class="todo-blocked"><input type="checkbox" disabled> description</li>`                  |
| Cancelled  | `<li class="todo-cancelled"><input type="checkbox" disabled> <del>description</del></li>`     |

The `todo-*` classes enable CSS styling by consumers:

```css
.todo-in-progress input[type="checkbox"] {
    accent-color: green;
    outline: 2px solid green;
}
.todo-completed input[type="checkbox"] {
    accent-color: green;
}
.todo-blocked input[type="checkbox"] {
    accent-color: red;
    outline: 2px solid red;
}
.todo-cancelled {
    opacity: 0.5;
    text-decoration: line-through;
}
```

No browser renderer change is required for the initial migration. The existing
`NodeKind::ListItem` arm preserves `todo-*` classes and emits the checkbox from
the `checked` field. For Cancelled, the description is wrapped in `<del>` because
the `NodeKind::Delete` child from the tree projection maps to `<del>` in the
browser renderer.

**BrowserTreeComponent adapter:**

```rust
use biscuit_terminal::render_tree::BrowserTreeComponent;
use renderable::browser::BrowserRenderable;

let todo = Todo::new("Review PR #42");
let component = BrowserTreeComponent::new(todo);
let fragment = component.render_html_fragment();
let html = fragment.render();
```

#### Layout to CSS Mapping

Todo's `Layout` maps to CSS via the existing `layout_to_css` lowering:

- Margins → `margin-*` on the `<li>` element
- Alignment → auto side margins when `max_width` is present, matching the
  existing browser layout lowering
- `max_width` → `max-width` CSS property

No additional CSS mapping is needed beyond what the tree renderer already provides.

#### Key Test Variants

| Variant                                    | Asserts                                                                          |
|--------------------------------------------|----------------------------------------------------------------------------------|
| Open state                                 | `<li class="todo-open">` with unchecked `<input>`                                |
| Completed state                            | `<li class="todo-completed">` with `checked` attribute                           |
| InProgress state                           | `<li class="todo-in-progress">` with unchecked `<input>`                        |
| Blocked state                              | `<li class="todo-blocked">` with unchecked `<input>`                             |
| Cancelled state                            | `<li class="todo-cancelled">` with `<del>` around description                   |
| Plain description                          | Text content present as text node                                                |
| Layout with margins                        | `<li>` has `margin-*` CSS                                                        |
| Empty description                          | `<li>` contains only the checkbox input                                          |
| Wrapped in list                            | Output is `<ul><li>...</li></ul>`                                                |

---

### Markdown IR Implementation

#### Markdown vs MarkdownPlus for Todo

Todo's rendering is largely text-based — a checkbox glyph followed by a description.
The key divergence point is the Cancelled state's strikethrough:

- **Markdown**: GFM supports `~~strikethrough~~` syntax natively, so the Cancelled
  state can use `~~description~~` without inline HTML.
- **MarkdownPlus**: Identical to Markdown for this component — the strikethrough
  is representable in pure Markdown syntax.

For all other states, Markdown and MarkdownPlus produce the same output:
`- [ ] description` (Open/InProgress/Blocked/Cancelled before strikethrough) or
`- [x] description` (Completed).

**Divergence summary:**

For `Todo::new("Task").with_state(TodoState::Cancelled)`:

- **Markdown**: `- [ ] ~~Task~~`
- **MarkdownPlus**: `- [ ] ~~Task~~` (identical — strikethrough is valid Markdown)

For `Todo::new("Task").with_state(TodoState::InProgress)`:

- **Markdown**: `- [ ] Task` (the InProgress state cannot be represented; degrades
  to unchecked)
- **MarkdownPlus**: `- [ ] Task` (same — no inline HTML needed for plain text)

**When Prose markup is present**:

- **Markdown**: Inline styles are flattened to plain text.
- **MarkdownPlus**: Initially identical to Markdown because Todo's projection uses
  the same lossy plain-text extraction as BlockQuote. Rich Prose preservation can
  be added later only if Prose exposes a tree projection that preserves inline
  semantics.

Since the `todo-*` classes carry state semantics but Markdown cannot express them,
both targets degrade to the standard GFM `- [ ]` / `- [x]` syntax. The state class
information is lost in Markdown output, which is the correct behavior — Markdown
optimizes for ergonomics, not for state fidelity.

#### Markdown Rendering Design

The Markdown tree renderer already handles `NodeKind::ListItem` with `checked`:
inside a `List`, `Some(true)` produces `- [x] text` and `Some(false)` produces
`- [ ] text`. For `checked: None`, the renderer produces `- text` (no checkbox),
so Todo must not use `None` for its extended states.

For Todo's extended states (InProgress, Blocked, Cancelled), the projection uses
`checked: Some(false)`. The Markdown output is `- [ ] text` for
InProgress/Blocked and `- [ ] ~~text~~` for Cancelled (unchecked with
strikethrough via the `NodeKind::Delete` child).

The Markdown renderer does not need to be modified — the existing `ListItem`
handling combined with the tree projection's `checked` field and `Delete` child
produces the correct output for all five states.

**Implementation:**

```rust
impl MarkdownRenderable for Todo {
    fn render_markdown(&self) -> String {
        let node = self.render_tree();
        render_markdown_node(&node, &MarkdownRenderOptions::default())
            .map(|r| r.output)
            .unwrap_or_else(|_| self.description.clone())
    }

    fn render_markdown_plus(&self) -> String {
        let node = self.render_tree();
        render_markdown_node(&node, &MarkdownRenderOptions::default_plus())
            .map(|r| r.output)
            .unwrap_or_else(|_| self.description.clone())
    }
}
```

Layout is ignored by the Markdown renderer (by design — locked by test).

#### Key Test Variants

| Variant                                    | Asserts                                                                      |
|--------------------------------------------|------------------------------------------------------------------------------|
| Open state — Markdown                      | Output is `- [ ] description`                                                |
| Open state — MarkdownPlus                  | Identical to Markdown                                                        |
| Completed state — Markdown                 | Output is `- [x] description`                                                |
| Completed state — MarkdownPlus             | Identical to Markdown                                                        |
| InProgress state — Markdown                | Output is `- [ ] description` (degraded to unchecked)                        |
| InProgress state — MarkdownPlus            | Identical to Markdown (no extended state in Markdown)                        |
| Blocked state — Markdown                   | Output is `- [ ] description` (degraded to unchecked)                        |
| Blocked state — MarkdownPlus               | Identical to Markdown                                                        |
| Cancelled state — Markdown                 | Output is `- [ ] ~~description~~` (strikethrough)                            |
| Cancelled state — MarkdownPlus             | Identical to Markdown                                                        |
| Markdown equals MarkdownPlus (all states)  | Both methods produce identical output for all five states                    |
| Prose description — Markdown               | Inline styles flattened to plain text                                        |
| Prose description — MarkdownPlus           | Same as Markdown until Prose has a lossless tree projection                  |
| Layout applied — Markdown                  | Layout has no effect on output (regression test)                             |

---

### `bt` CLI

- This specification will ensure that the **Todo** component:
    - has a 'bt' CLI subcommand for rendering this component
    - that the '--md' and '--html' CLI switches are available to render to Markdown and HTML targets respectively (the default render is always for the Terminal)
    - that the '--example' CLI switch is in place to provide a thoughtful example of how this command should be used with the CLI (see other working examples for a template)

#### Current State

| Aspect              | Status                                                                 |
|---------------------|------------------------------------------------------------------------|
| CLI command exists  | No                                                                     |
| Render method       | N/A                                                                    |
| Has `--md` switch   | No                                                                     |
| Has `--html` switch | No                                                                     |
| Has `--example`     | No                                                                     |

The Todo component has no `bt` CLI subcommand. It is not registered in
`biscuit-terminal/cli/src/args.rs` and has no corresponding command module.

#### Specification Design

Create a new `bt todo` subcommand that renders a single Todo item to the terminal
by default, with optional `--md`, `--md-plus`, and `--html` output targets.

**Args:**

```rust
#[derive(ClapArgs, Debug, Clone)]
pub struct TodoArgs {
    /// Render an example and show the command used
    #[arg(long, short = 'e')]
    pub example: bool,

    /// Description text for the todo item
    #[arg(value_name = "DESCRIPTION", required_unless_present = "example")]
    pub description: Option<String>,

    /// State of the todo item
    #[arg(long, value_enum, default_value = "open")]
    pub state: TodoStateArg,

    /// Interpret description as Prose markup
    #[arg(long)]
    pub prose: bool,

    /// Render to an HTML fragment instead of the terminal.
    #[arg(long, conflicts_with_all = ["md", "md_plus"])]
    pub html: bool,

    /// Render to portable Markdown instead of the terminal.
    #[arg(long, conflicts_with_all = ["html", "md_plus"])]
    pub md: bool,

    /// Render to MarkdownPlus instead of the terminal.
    #[arg(long = "md-plus", conflicts_with_all = ["html", "md"])]
    pub md_plus: bool,
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum TodoStateArg {
    Open,
    InProgress,
    Completed,
    Blocked,
    Cancelled,
}
```

**Render path:**

1. Build `Todo` from flags (description, state, prose). Add a public
   `Todo::with_state(TodoState)` builder or equivalent setter, because `state`
   and `description` are private fields today and the CLI crate cannot construct
   arbitrary states with a struct literal.
2. **Terminal** (default): Project tree → `render_terminal_node()`.
3. **HTML** (`--html`): Wrap in `BrowserTreeComponent` → `render_html_fragment()`.
4. **Markdown** (`--md`): Project tree → `render_markdown_node()`.
5. **MarkdownPlus** (`--md-plus`): Project tree → `render_markdown_node()` with
   MarkdownPlus mode.

**Implementation in a new file `biscuit-terminal/cli/src/commands/todo.rs`:**

```rust
impl Run for TodoArgs {
    fn run(self, _ctx: &CliContext) -> color_eyre::Result<()> {
        let desc = self
            .description
            .or_else(|| self.example.then(|| "Review pull request #42".to_string()))
            .unwrap_or_default();

        let state = match self.state {
            TodoStateArg::Open => TodoState::Open,
            TodoStateArg::InProgress => TodoState::InProgress,
            TodoStateArg::Completed => TodoState::Completed,
            TodoStateArg::Blocked => TodoState::Blocked,
            TodoStateArg::Cancelled => TodoState::Cancelled,
        };

        let todo = if self.prose {
            Todo::from_prose(desc)
        } else {
            Todo::new(desc)
        }
        .with_state(state);

        let node = todo.render_tree();

        if self.html {
            let component = BrowserTreeComponent::new(todo);
            println!("{}", component.render_html_fragment().render());
            return Ok(());
        }
        if self.md {
            let rendered = render_markdown_node(&node, &MarkdownRenderOptions::default())
                .map_err(|e| color_eyre::eyre::eyre!("markdown render failed: {e}"))?;
            println!("{}", rendered.output);
            return Ok(());
        }
        if self.md_plus {
            let rendered = render_markdown_node(&node, &MarkdownRenderOptions::default_plus())
                .map_err(|e| color_eyre::eyre::eyre!("markdown render failed: {e}"))?;
            println!("{}", rendered.output);
            return Ok(());
        }

        let term = detect_terminal_honoring_force_color();
        let opts = TerminalRenderOptions::new(&term, RenderStrictness::Warn);
        let rendered = render_terminal_node(&node, &opts)
            .map_err(|e| color_eyre::eyre::eyre!("render failed: {e}"))?;
        println!("{}", rendered.output);

        if self.example {
            print_example_command(TODO_EXAMPLE_CMD);
        }
        Ok(())
    }
}
```

**Registration in `args.rs`:**

```rust
pub enum Command {
    // ... existing commands ...
    #[command(display_order = 19)]
    Todo(todo::TodoArgs),
}
```

**Example command:**

```rust
const TODO_EXAMPLE_CMD: &str =
    r#"bt todo "Review pull request #42" --state completed"#;
```

**Example output:**

When run with `--example`, the command renders a completed Todo item in the
terminal and prints the command that produced it:

```
✔ Review pull request #42

Command:
bt todo "Review pull request #42" --state completed
```

---

## Acceptance Criteria Summary

- [ ] `Todo` implements `TreeRenderable`, projecting to a one-item `NodeKind::List` with a `ListItem` carrying `todo-*` classes and typed task-state hints
- [ ] `Todo` exposes a public state builder/setter so CLI and external callers can construct all five states without accessing private fields
- [ ] `Todo`'s `TerminalRenderable::render()` delegates to the tree path by default
- [ ] Bespoke render path retained as `render_bespoke()` for parity testing
- [ ] `BrowserRenderable` achieved — browser tree renderer produces `<li>` with state-specific classes and checkbox
- [ ] `MarkdownRenderable` implemented on `Todo` — both Markdown and MarkdownPlus produce GFM task list syntax
- [ ] Render tree adds typed task-state hints on `NodeAttrs` and validates that they only appear on `ListItem` nodes (Feature 2, APPROVED)
- [ ] Terminal tree renderer handles typed task-state hints for extended checkbox glyph rendering
- [ ] `bt todo "description" --state <state>` renders a Todo item to the terminal
- [ ] `bt todo --html` renders HTML output
- [ ] `bt todo --md` renders Markdown output (`- [ ]` / `- [x]`)
- [ ] `bt todo --md-plus` renders MarkdownPlus output
- [ ] `bt todo --example` renders example with command display
- [ ] Parity tests (bespoke vs tree) cover all variants listed in Terminal IR section
- [ ] `KNOWN_DRIFT` ledger documents accepted divergences
- [ ] `Todo` added to the components table in `renderable/docs/components.md` with updated IR State and bt CLI columns
