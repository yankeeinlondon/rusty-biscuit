---
last_updated: "2026-05-16"
---

# Challenges of Migrating the `YamlBlock` Component to the Tree Rendering Architecture

## Functional and Design Goals

The `YamlBlock` component provides a **validated, syntax-highlighted YAML code block** that
renders identically to a Markdown ` ```yaml ` fence — sharing the same header row, ANSI
highlighting, and HTML code-block wrappers used by darkmatter's Markdown rendering pipeline.

### Why YamlBlock was created

Before `YamlBlock`, callers who wanted to render standalone YAML (outside of a Markdown
document) had to either construct a synthetic Markdown string with a fenced code block and
route it through the full `Markdown → as_terminal()` / `as_html()` pipeline, or hand-roll
their own code-block formatting. `YamlBlock` packages this into a typed component so that:

- YAML is **validated at construction time** via `serde_yaml_ng` parsing — malformed YAML
  fails early with a `YamlBlockError::YamlParse`, preventing downstream rendering of
  invalid content.
- The rendered output is **byte-identical in the body region** to what a Markdown ` ```yaml `
  fence produces, because both paths call the same shared helpers (`render_terminal_code_block`,
  `render_html_code_block`, `format_header_row`, `CodeHighlighter`).
- The component implements both `TerminalRenderable` and `BrowserRenderable`, giving
  callers a single object that can render to either target without depending on a full
  darkmatter `Markdown` instance.
- A `Layout` is owned by the component, enabling margins, alignment, and word-wrap to be
  applied to the rendered code block as a cohesive unit.

### Where YamlBlock is used today

| Consumer | Crate | Usage pattern |
|----------|-------|---------------|
| Public API of `darkmatter::markdown` | `darkmatter` | Exported as `darkmatter::markdown::YamlBlock` for direct instantiation by downstream crates |
| Claudine report harness | `claudine` | Considered for YAML failure snippets but deliberately skipped in favor of lighter-weight `highlight_yaml_lines` — the full code-block frame is too visually heavy for inline failure blocks |

### Example usage

```rust
use darkmatter::markdown::YamlBlock;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::terminal::Terminal;

// From a raw YAML string
let block = YamlBlock::new("server:\n  port: 8080\n  host: localhost").unwrap();

// Render to terminal with syntax highlighting
let term = Terminal::new();
let output = block.render(&term);
// output contains: ANSI-highlighted header row (" yaml "), highlighted body, layout padding

// Render to browser
use biscuit_terminal::components::renderable::BrowserRenderable;
let fragment = block.render_html_fragment();
// fragment contains: <div class="code-block"><pre><code class="language-yaml">...</code></pre></div>

// From a Markdown file's frontmatter
let block = YamlBlock::from_markdown_file("docs/config.md").unwrap();
// block.yaml() contains the normalized frontmatter (comments dropped, whitespace canonicalized)

// With layout customization
let mut block = YamlBlock::new("key: value").unwrap();
block.layout_mut().left_margin = Margin::Chars(4);
let indented = block.render(&term);
```

## Technical Implementation (current)

### Structure

The component lives at `darkmatter/lib/src/markdown/yaml_block.rs` and consists of:

- **`YamlBlockError`** — a 3-variant error enum covering `Io`, `YamlParse`, and
  `MarkdownParse` failures, all derived via `thiserror`.
- **`YamlBlock`** — owns:
  - `yaml: String` — the validated YAML text (original source, not the parsed value)
  - `layout: Layout` — standard layout configuration (margins, alignment, word wrap)

### Constructors

| Constructor | Input | Validation |
|-------------|-------|------------|
| `new(raw_yaml)` | Raw YAML string | `serde_yaml_ng` parse → stores original text |
| `from_yaml_file(path)` | Path to `.yaml` file | Read + `new()` |
| `from_markdown_content(md)` | Raw Markdown string | Parse via `Markdown::try_from_content` → extract frontmatter → re-serialize via `serde_yaml_ng` → `new()` |
| `from_markdown_file(path)` | Path to `.md` file | Read + `from_markdown_content()` |

The Markdown constructors are **lossy round-trips**: comments, anchors/aliases, and custom
YAML tags are dropped because `FrontmatterMap` normalizes through `serde_json::Value`. Key
order is preserved (backed by `IndexMap`). Callers needing byte-exact preservation use
`new()` directly.

### Rendering pipeline

`YamlBlock` implements two render traits:

1. **`TerminalRenderable::render(&self, term)`** — produces an ANSI-highlighted terminal
   string:
   - Detects color mode via `detect_color_mode()` (fresh each call to honor env-var changes)
   - Creates a `CodeHighlighter` with the resolved theme and color mode
   - Builds a header row via `format_header_row("yaml", bg_color, ...)` — the same
     full-width background-filled label bar that Markdown fences use
   - Highlights the body via `render_terminal_code_block("yaml", &highlighter, ...)`
   - Concatenates header + body and applies `Layout::apply_layout()` (margins, alignment,
     word-wrap, row-fill)
   - Returns a `String` with embedded ANSI escape sequences

2. **`BrowserRenderable::render_html_fragment(&self)`** — produces a `BrowserFragment<Ready>`
   containing safe HTML:
   - Creates a `CodeHighlighter` with `HtmlOptions::default()`
   - Delegates to `render_html_code_block("yaml", ...)` which produces
     `<div class="code-block"><pre><code class="language-yaml">...</code></pre></div>`
     with syntax-highlighted spans and HTML-escaped content
   - Wraps the result as a `ComposableNode::RawHtml` island

### Key responsibilities and transforms

| Responsibility | Where it happens |
|----------------|------------------|
| YAML validation | `validate_yaml()` private function — parses as `serde_yaml_ng::Value` |
| Frontmatter extraction and normalization | `from_markdown_content()` — `Markdown::try_from_content()` → `FrontmatterMap` → `serde_yaml_ng::to_string()` |
| Syntax highlighting (terminal) | `render_terminal_code_block()` shared helper in darkmatter's `output::terminal` module |
| Syntax highlighting (browser) | `render_html_code_block()` shared helper in darkmatter's `output::html` module |
| Header row generation | `format_header_row()` — full-width bar with `yaml` language label and theme background color |
| Layout application | `Layout::apply_layout()` — margins, alignment, word-wrap, row-fill |
| Color mode adaptation | `detect_color_mode()` — reads `COLORFGBG` and `NO_COLOR` env vars |
| HTML escaping | `html_escape::encode_text()` inside `render_html_code_block()` (fallback path) |

### Parity contract

A core design goal is **body-region parity** with Markdown ` ```yaml ` fences. The existing
tests (`test_terminal_render_parity_with_markdown_yaml_fence`,
`test_browser_render_parity_with_markdown_yaml_fence`) assert that after stripping
wrapper differences (extra newlines, surrounding blank lines), the YAML body content is
byte-identical between `YamlBlock` output and `Markdown` fence output.

## Implementation Challenges

### Implementation Challenges

#### Header Row Has No Tree Representation

The current `YamlBlock::render()` emits a full-width background-filled header row
containing the `yaml` language label, generated by `format_header_row()`. This header row
is a terminal-specific visual chrome element — it uses ANSI background fills and terminal
width to create a colored bar.

The tree model's `NodeKind::Code` carries `lang` and `meta` fields but has no concept of
a rendered header row. The terminal tree renderer's `render_code()` method (in
`bisc-terminal/lib/src/render_tree/render.rs:372-385`) emits only a dim ` ```yaml `
label followed by an indented body — no background fill, no full-width bar.

**Example:** A `YamlBlock` with content `key: value` currently produces something like:

```
┌──────────────────────────────── yaml ─────────────────────────────────┐
│ key: value                                                            │
└───────────────────────────────────────────────────────────────────────┘
```

But routing through the tree would produce:

```
    ```yaml
    key: value
```

The header row chrome is entirely lost.

**Proposed test:**

```rust
#[test]
fn yaml_block_tree_preserves_header_label() {
    let block = YamlBlock::new("key: value").unwrap();
    let tree = block.render_tree();
    let rendered = render_terminal_node(&tree, &default_opts()).unwrap();

    let plain = strip_escape_codes(&rendered.output);
    assert!(
        plain.contains("yaml"),
        "Tree path must include the 'yaml' language label. Got: {plain:?}"
    );
}
```

#### Syntax Highlighting Is Not in the Tree Path

The current `YamlBlock::render()` uses `CodeHighlighter` (backed by `syntect`) to produce
per-token ANSI escape sequences for YAML syntax (keys in one color, values in another,
strings, booleans, etc.). This is a **darkmatter-specific** capability that depends on
`syntect` themes and color-mode detection.

The tree terminal renderer's `render_code()` method deliberately does **no** syntax
highlighting — it emits the entire body as dim text. The tree architecture doc (Section 2)
states: "syntax highlighting and meta handling are out of scope for this phase."

This means routing `YamlBlock` through the tree would **lose all syntax highlighting**,
producing a plain dim block instead of a colorful YAML rendering.

**Example:** Current output for `foo: 1` includes distinct ANSI colors for the key `foo`,
the colon, and the value `1`. Tree output would render the entire line in a single dim
style.

**Proposed test:**

```rust
#[test]
fn yaml_block_tree_preserves_body_content() {
    let yaml = "server:\n  port: 8080\n  host: localhost";
    let block = YamlBlock::new(yaml).unwrap();
    let tree = block.render_tree();
    let rendered = render_terminal_node(&tree, &default_opts()).unwrap();

    let plain = strip_escape_codes(&rendered.output);
    for needle in ["server:", "port: 8080", "host: localhost"] {
        assert!(
            plain.contains(needle),
            "Tree output must contain '{needle}'. Got: {plain:?}"
        );
    }
}
```

#### Color Mode and Theme Are Resolved at Render Time

The current `TerminalRenderable::render()` calls `detect_color_mode()` on every render
invocation, reading the `COLORFGBG` and `NO_COLOR` environment variables. It then
constructs a `CodeHighlighter` with the resolved theme. This allows the same `YamlBlock`
instance to produce different output when rendered against different terminal states
(e.g., a dark terminal vs. a light terminal).

The tree architecture separates **tree production** (`render_tree()`) from **tree
consumption** (the renderer). `render_tree()` returns a `RenderNode` — a static,
serde-serializable data structure with no environment awareness. Color mode detection
happens only at the renderer level, but the tree renderer has no syntax highlighting at
all (see previous challenge).

Even if syntax highlighting were added to the tree terminal renderer, the theme and color
mode would need to be injected through `TerminalRenderOptions` rather than detected inside
the component's render path — a significant architectural shift.

**Example:** Two renders of the same `YamlBlock` with `COLORFGBG="15;0"` (dark) vs.
`COLORFGBG="0;15"` (light) produce visibly different ANSI background sequences. The tree
path would need the renderer to accept and propagate these theme choices into the
highlighting pipeline.

**Proposed test:**

```rust
#[test]
fn yaml_block_tree_dark_and_light_differ() {
    let block = YamlBlock::new("key: value").unwrap();
    let tree = block.render_tree();

    env::set_var("COLORFGBG", "15;0");
    let dark = render_terminal_node(&tree, &default_opts()).unwrap();

    env::set_var("COLORFGBG", "0;15");
    let light = render_terminal_node(&tree, &default_opts()).unwrap();

    // Once highlighting is in the tree path, these should differ
    // For now this test documents the gap
    assert!(
        dark.output.contains("key") && light.output.contains("key"),
        "Both renders must contain the YAML content"
    );
}
```

#### Layout Ownership and Application

`YamlBlock` owns a `Layout` that callers configure via `layout_mut()`. The current render
path applies this layout via `Layout::apply_layout()` after building the raw output string.

In the tree architecture, `TreeComponent<T>` owns the layout and applies it in its
`TerminalRenderable` impl, but `YamlBlock` would need to project its layout into the
`TreeComponent` wrapper. The tree node itself (`NodeKind::Code`) has no layout fields —
layout is an out-of-band concern handled by the adapter.

This means the `Layout` that callers configure on `YamlBlock` would not flow through
`render_tree()` — it would need to be transferred to the `TreeComponent` wrapper at
construction time or applied as a post-processing step.

**Example:** A caller sets `block.layout_mut().left_margin = Margin::Chars(4)`. When
routed through `TreeComponent::new(block)`, the `TreeComponent` has its own default
layout — the caller's margin is lost unless explicitly transferred.

**Proposed test:**

```rust
#[test]
fn yaml_block_tree_honors_layout_margin() {
    let mut block = YamlBlock::new("key: value").unwrap();
    block.layout_mut().left_margin = Margin::Chars(4);

    let tree_component = TreeComponent::new(block);
    // The TreeComponent should somehow reflect the YamlBlock's layout
    // Currently this fails — TreeComponent uses its own default layout
    let rendered = tree_component.render_optimistic(Some(80));
    let plain = strip_escape_codes(&rendered);

    for line in plain.lines().filter(|l| !l.trim().is_empty()) {
        assert!(
            line.starts_with("    "),
            "Expected 4-space left margin, got: {line:?}"
        );
    }
}
```

#### Browser Rendering Uses Darkmatter-Specific Helpers

The `BrowserRenderable` impl for `YamlBlock` delegates to `render_html_code_block()` — a
darkmatter helper that produces `<div class="code-block">` wrappers with syntect-driven
syntax highlighting spans. This helper depends on darkmatter's `HtmlOptions`, `CodeBlockMeta`,
and `CodeHighlighter` types.

The tree browser renderer's `render_code_block()` (in `renderable/src/tree/render/browser.rs`)
produces bare `<pre><code class="language-yaml">` elements with **no** syntax highlighting
spans — just the raw text content inside the code tag.

Routing `YamlBlock` through the tree browser path would lose both the highlighting spans
and the `<div class="code-block">` wrapper, producing structurally different HTML.

**Example:** Current HTML contains highlighted spans like
`<span class="sy0">:</span>` and `<span class="st0">"value"</span>`. Tree output would
contain plain text: `<pre><code class="language-yaml">key: value</code></pre>`.

**Proposed test:**

```rust
#[test]
fn yaml_block_tree_browser_preserves_code_structure() {
    let block = YamlBlock::new("key: value").unwrap();
    let tree = block.render_tree();
    let rendered = render_browser_node(&tree, &BrowserRenderOptions::default()).unwrap();
    let html = rendered.output.render();

    assert!(
        html.contains("language-yaml"),
        "Must contain language-yaml class. Got: {html}"
    );
    assert!(
        html.contains("<pre>") && html.contains("<code"),
        "Must contain pre/code tags. Got: {html}"
    );
    assert!(
        html.contains("key"),
        "Must contain YAML content. Got: {html}"
    );
}
```

#### YAML Lives in the Wrong Crate for TreeRenderable

`YamlBlock` lives in `darkmatter/lib/src/markdown/yaml_block.rs`. The `TreeRenderable`
trait lives in the `renderable` crate. For `YamlBlock` to implement `TreeRenderable`,
`darkmatter` would need to depend on `renderable`'s tree module — which it already does
(the dependency chain is `darkmatter → biscuit-terminal → renderable`). However,
`YamlBlock`'s render logic depends on darkmatter-specific types (`CodeHighlighter`,
`TerminalOptions`, `HtmlOptions`, `CodeBlockMeta`, `render_terminal_code_block`,
`render_html_code_block`) that cannot move to `renderable` without bringing syntect
along.

The `TreeComponent` adapter lives in `biscuit-terminal`, which is between `darkmatter`
and `renderable` in the dependency chain. The adapter bridges `TreeRenderable →
TerminalRenderable`, but `YamlBlock` cannot use it without its `render_tree()` projection
being meaningful enough for downstream renderers to produce equivalent output.

**Example:** If `YamlBlock` implements `TreeRenderable::render_tree()` by returning a
`NodeKind::Code { lang: Some("yaml"), value: self.yaml.clone(), .. }`, the downstream
renderers produce output that is **qualitatively different** (no highlighting, no header
row, no code-block wrapper) from the bespoke path.

**Proposed test:**

```rust
#[test]
fn yaml_block_tree_projection_is_code_node() {
    let block = YamlBlock::new("key: value").unwrap();
    let tree = block.render_tree();

    match &tree.kind {
        NodeKind::Root { children } => {
            assert_eq!(children.len(), 1, "Expected single child");
            match &children[0].kind {
                NodeKind::Code { lang, value, .. } => {
                    assert_eq!(lang.as_deref(), Some("yaml"));
                    assert_eq!(value, "key: value");
                }
                other => panic!("Expected Code node, got: {other:?}"),
            }
        }
        other => panic!("Expected Root node, got: {other:?}"),
    }
}
```

#### Parity Contract Must Be Maintained

The existing tests enforce a strict body-region parity contract between `YamlBlock` output
and Markdown ` ```yaml ` fence output. Any tree-based rendering must preserve this parity.

The tree architecture's parity discipline (established for `BlockQuote` in
`render_tree_component_parity.rs`) requires that for each adopted component, both the
bespoke `TerminalRenderable` path and the `TreeComponent`-wrapped path produce
semantically equivalent output. For `YamlBlock`, parity means:

- The `yaml` language label appears in both outputs
- The YAML body content appears verbatim in both outputs (after ANSI stripping)
- Both outputs contain ANSI escape sequences (i.e., are syntax-highlighted)

The current tree terminal renderer cannot satisfy the highlighting requirement.

**Example:** The existing `test_terminal_render_parity_with_markdown_yaml_fence` test
asserts that the body slice `"foo: 1\nbar: 2"` appears verbatim in both `YamlBlock` output
and Markdown fence output. A tree-parity test would need to assert the same between the
bespoke path and the `TreeComponent` path.

**Proposed test:**

```rust
#[test]
fn yaml_block_tree_terminal_parity() {
    let yaml_content = "foo: 1\nbar: 2";
    let block = YamlBlock::new(yaml_content).unwrap();
    let term = Terminal::new();

    // Bespoke path
    let bespoke = block.render(&term);

    // Tree path
    let component = TreeComponent::new(block);
    let tree_output = component.render(&term);

    // Content parity (after ANSI stripping)
    let bespoke_plain = strip_ansi_codes(&bespoke);
    let tree_plain = strip_ansi_codes(&tree_output);

    for needle in ["foo: 1", "bar: 2"] {
        assert!(bespoke_plain.contains(needle), "bespoke missing {needle}");
        assert!(tree_plain.contains(needle), "tree missing {needle}");
    }

    // Highlighting parity
    assert!(bespoke.contains("\x1b["), "bespoke has ANSI");
    assert!(tree_output.contains("\x1b["), "tree should have ANSI");
}
```

#### Empty YAML Edge Case

`YamlBlock::new("")` succeeds — an empty string is valid YAML (parses as `Null`). The
current render path handles this by emitting a code block with a header row and an empty
(or padding-only) body.

The tree renderer's `render_code()` calls `value.trim_end_matches('\n')` on the code value,
which for an empty string produces `""`. The resulting output is a dim ` ```yaml ` label
with no body lines — structurally different from the bespoke path which still emits padding
rows and background fills.

**Example:** `YamlBlock::new("")` via bespoke path produces an ANSI-bearing output with a
visible header row. Via the tree path, it produces a minimal dim label with no visual body.

**Proposed test:**

```rust
#[test]
fn yaml_block_tree_handles_empty_yaml() {
    let block = YamlBlock::new("").unwrap();
    let tree = block.render_tree();
    let rendered = render_terminal_node(&tree, &default_opts()).unwrap();

    // Must not panic or return empty output
    assert!(!rendered.output.is_empty(), "Empty YAML should produce output");
    // Should still mention the yaml language
    assert!(
        strip_escape_codes(&rendered.output).contains("yaml"),
        "Empty YAML should still show language label"
    );
}
```

## Solution Suggestions

#### Code-Block Renderer Extension Point

Add an optional **syntax highlighting hook** to the tree renderers (terminal and browser)
that can be supplied via render options. Instead of embedding highlighting logic in the
component, the renderer accepts a `Highlighter` trait object or closure that, given a
language tag and source text, returns highlighted output.

**Which challenges this helps with:**

- **Syntax Highlighting Is Not in the Tree Path** — the hook bridges the gap between the
  tree's plain-text `Code` nodes and the rich highlighting that `YamlBlock` expects. The
  hook is called only for `NodeKind::Code` nodes that have a `lang` field.
- **Color Mode and Theme Are Resolved at Render Time** — the hook is constructed at render
  time with the current theme/color-mode, so environment sensitivity is preserved without
  putting it in the tree node.
- **Browser Rendering Uses Darkmatter-Specific Helpers** — the browser renderer gets a
  similar hook that produces highlighted HTML spans instead of raw text.

**Variant:** Instead of a trait object, the render options could carry an enum of known
highlighters (e.g., `NoHighlight`, `SyntectHighlight { theme, color_mode }`), avoiding
dynamic dispatch at the cost of extensibility.

#### Code-Block Metadata Enrichment

Extend `NodeKind::Code` (or its `meta` field) to carry rendering hints that the terminal
renderer can use to produce richer output — for example, a `"header_row"` hint that tells
the renderer to emit a full-width background-filled header bar with the language label.

**Which challenges this helps with:**

- **Header Row Has No Tree Representation** — the renderer can read the hint and delegate
  to `format_header_row()` or an equivalent, producing the same visual chrome without
  baking it into the tree structure.
- **Empty YAML Edge Case** — the renderer can use the hint to emit padding rows even
  when the body is empty, matching the bespoke behavior.

**Variant:** Instead of overloading `meta`, add a `NodeKind::FencedCode` variant that
distinguishes "code block with full chrome" from `NodeKind::Code` (plain code block).

#### Layout Transfer Protocol

Define a convention (trait method or builder pattern) for transferring a component's
`Layout` to its `TreeComponent` wrapper. When `YamlBlock` (or any component) is wrapped
in `TreeComponent::new(component)`, the wrapper reads the component's layout and applies
it as its own.

**Which challenges this helps with:**

- **Layout Ownership and Application** — the layout transfer ensures that caller-configured
  margins, alignment, and word-wrap survive the transition from bespoke component to tree
  adapter.

**Variant:** Add a `TreeRenderable::default_layout(&self) -> Layout` method that the
`TreeComponent` adapter calls at construction time, letting each component specify its
preferred layout.

#### Dual-Path Rendering with Feature Flag

Keep `YamlBlock`'s bespoke `TerminalRenderable` impl as the **primary** path and add a
`TreeRenderable` impl as a **secondary** path, gated behind a feature flag or runtime
switch. The `TreeRenderable` impl produces a `NodeKind::Code` tree for downstream
consumers (Markdown, HTML) while the bespoke impl continues to handle terminal rendering
with full highlighting and chrome.

**Which challenges this helps with:**

- **YAML Lives in the Wrong Crate for TreeRenderable** — no need to move highlighting
  into the tree renderers. The component stays in darkmatter and keeps its darkmatter
  dependencies.
- **Parity Contract Must Be Maintained** — the bespoke path is untouched, so existing
  parity tests continue to pass. New parity tests compare the tree Markdown/HTML output
  against the bespoke Markdown/HTML output, which is a weaker requirement than terminal
  parity.
- **All other challenges** — this approach sidesteps them by not routing terminal
  rendering through the tree at all, using the tree only for the semantic targets
  (Markdown, HTML) where plain code blocks are acceptable.

**Variant:** Instead of a feature flag, use a separate `TreeYamlBlock` adapter type that
wraps a `YamlBlock` and implements only `TreeRenderable`, leaving the original type's
`TerminalRenderable` impl untouched.

#### Parity Test Infrastructure for Code Blocks

Extend the component parity test infrastructure (currently in
`render_tree_component_parity.rs`) with code-block-specific assertions that compare
body content, language labels, and structural markers between bespoke and tree paths.
This infrastructure would be reusable for any component that renders as a code block.

**Which challenges this helps with:**

- **Parity Contract Must Be Maintained** — provides a structured way to assert parity
  for code-block components without each component reinventing its own comparison logic.
- **Empty YAML Edge Case** — the infrastructure can include specific assertions for
  empty/edge-case content.

**Variant:** Use snapshot testing (via `insta`) for the tree output, comparing against
golden files that capture the expected highlighted and non-highlighted forms separately.
