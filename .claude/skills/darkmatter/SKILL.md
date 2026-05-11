---
name: darkmatter
description: Expert knowledge for the darkmatter Rust library - markdown parsing, rendering (terminal/HTML), syntax highlighting, frontmatter, and document comparison. Delegates terminal rendering to biscuit-terminal. Use when parsing markdown, generating terminal/HTML output, working with frontmatter, or comparing documents.
---

# darkmatter

Markdown parsing, rendering, and composition library. Part of the dockhand monorepo.

**Key principle**: darkmatter handles **markdown parsing and composition**. Terminal rendering (images, mermaid, detection) is delegated to `biscuit-terminal`.

## Responsibility Split

| Responsibility | Package |
|----------------|---------|
| Markdown parsing (CommonMark + GFM) | darkmatter |
| Compose pipeline (Inline Pre + Transclusion + Inline Post + Finalization) | darkmatter |
| Syntax highlighting | darkmatter (syntect) |
| Frontmatter extraction | darkmatter |
| HTML output | darkmatter |
| Visual diff utilities | darkmatter |
| Document comparison/normalization | darkmatter |
| Terminal detection, image/mermaid rendering, tables | **biscuit-terminal** |

## Quick Start

```rust
use darkmatter::markdown::{Markdown, output::{TerminalOptions, write_terminal}};

let md: Markdown = "# Hello\n\nWorld".into();
let mut stdout = std::io::stdout();
write_terminal(&mut stdout, &md, TerminalOptions::default())?;
```

### Page-Level Layout (DarkmatterPage)

`DarkmatterPage` is a page-level layout primitive that owns margin, padding, page background, max-width, line numbers, and per-component alignment/fill settings. It captures terminal capabilities at construction and delegates to the existing terminal renderer, threading a `LayoutContext` through the render pipeline so per-component alignment and fill are applied to images, block quotes, tables, code blocks, and lists.

```rust
use biscuit_terminal::terminal::Terminal;
use darkmatter::layout::{DarkmatterPage, PageBackground};
use darkmatter::markdown::Markdown;

let term = Terminal::new_optimistic(120);
let md: Markdown = "# Hello\n\nWorld".into();
let output = DarkmatterPage::new(&term)
    .with_margin(2)
    .with_padding(1)
    .with_page_background(PageBackground::Subtle)
    .with_max_width(100)
    .render(&md)?;
```

`DarkmatterPage` implements `biscuit_terminal::renderable::Renderable` for composition with the biscuit-terminal component ecosystem. With no builder calls, `render` is byte-for-byte equivalent to `for_terminal(&md, TerminalOptions::default())`.

## Compose Pipeline

Three-phase pipeline for document preparation leveraging the [pulldown-cmark](./pulldown-cmark.md) crate:

**Inline Pre** (serial):
1. **Frontmatter Interpolation** - `{{ variable }}` in frontmatter resolves before effective state is built
2. **Frontmatter Shell Expansion** - top-level `$(cmd)` frontmatter values execute after interpolation and write trimmed `stdout` back into frontmatter
3. **Text Replacement** - `replace:` frontmatter replaces literal strings
4. **Page Blocks** - `::block` / `::end-block` conditional regions; nest to arbitrary depth with stack-based pairing, lazy child evaluation (skipped parents never evaluate inner `when`), and code-fence protection at every depth
5. **Interpolation** - `{{ variable }}` expressions expand to values
6. **Shell Expansion** - `::shell` directives execute approved commands and inject combined `stdout` + `stderr`
7. **Shell Blocks** - `::shell-block` / `::end-block` directives execute multiple approved commands sequentially and render their combined output
8. **Link Resolve (abs)** - Converts local links to absolute paths before transclusion

**Transclusion** (prepared serially, resolved concurrently):
- `::file ./doc.md` - Markdown transclusion with recursive processing
- `::code ./main.rs` - Code/text transclusion with fenced block generation
- `::toc-linking ./doc.md` - Linked heading lists from another document's raw source headings
- `prologue` / `epilogue` - Frontmatter-driven transclusion
- `when="..."` conditions, ancestry-based cycle detection, depth limits
- `set=` / `set.NAME=` - Override child frontmatter before child pipeline stages run (three-layer deep-merge: child FM < object-form `set=` < property-form `set.NAME=`). Overlay does not propagate to grandchildren.
- `--allow-invalid-frontmatter-assignment` / `--allow-reassigned-frontmatter-property` - Permissive-mode CLI flags that downgrade set-override errors to warnings

**Inline Post** (serial):
1. **Cleanup** - Normalizes formatting (spacing, tables)
2. **Normalization** - Adjusts heading levels

**Finalization** (serial, root-only):
1. **Link Normalization** - Converts absolute paths back to portable forms (relative, `~/`, or `${ENV}`)

```rust
use darkmatter::markdown::{Markdown, compose::{ComposeOptions, ComposeOperation}};

// All operations (default)
let mut md: Markdown = content.into();
let report = md.compose_mut()?;

// Full pipeline with transclusion (requires source file path)
let md = Markdown::try_from(std::path::Path::new("docs/root.md"))?;
let options = ComposeOptions::new()
    .with_source_file("docs/root.md");
let (composed, report) = md.compose_with(options)?;

// Only specific operations
let options = ComposeOptions::new()
    .only(&[ComposeOperation::Interpolation, ComposeOperation::Cleanup]);
```

### Interpolation Expressions

| Expression | Example | Description |
|------------|---------|-------------|
| Variable | `{{ name }}` | Frontmatter value |
| Nested | `{{ user.email }}` | Nested object access |
| Context | `{{ ctx.today }}` | Runtime context (today, year, etc.) |
| Environment | `{{ env.HOME }}` | Environment variable |
| Fallback | `{{ color || "unknown" }}` | Default if missing |
| Ternary | `{{ x ? "yes" : "no" }}` | Conditional |
| Comparison | `{{ count > 0 ? "has" : "empty" }}` | Numeric comparison |
| Functions | `{{ length(items) }}` | Helper functions |

### Helper Functions

- `length(x)` - String/array/object length
- `number(x, default)` - Parse as number
- `round(x, default)` - Round to integer

### Expression Module

The `compose::expression` module is the canonical home for expression parsing and evaluation infrastructure:

- **`ast::Expr`** — AST nodes for parsed expressions
- **`lexer::Lexer`** — Tokenizes expression strings
- **`parser::Parser`** — Builds AST from tokens (supports interpolation and condition modes)
- **`EvaluationLookup`** — Trait for variable resolution used by both interpolation and condition evaluation
- **`evaluate`** — Core expression evaluator shared by interpolation and condition evaluation
- **Shared helpers** — `is_truthy`, `to_number`, `scalar_string` for JSON value manipulation

`compose::interpolation` re-exports these types for backward compatibility (`InterpolationLookup` is an alias for `EvaluationLookup`).

### Shortcut API

For external callers who only need the boolean DSL without constructing a full `ComposeContext` + `EffectiveState`, use `compose::conditions::evaluate_condition_against`:

```rust
use darkmatter::markdown::compose::conditions::evaluate_condition_against;
use serde_json::json;
use std::path::Path;

let data = json!({ "draft": true, "audience": "internal" });
let result = evaluate_condition_against(
    "draft && audience == 'internal'",
    &data,
    Path::new("."),
)?;
```

This shortcut resolves:
- Top-level and nested paths against the provided `data`
- `env.*` against `std::env`
- `ctx.*` via lazy runtime context capture (only the groups actually referenced are captured)
- Missing unprefixed keys fall back to `ctx.*` (same behavior as `EffectiveState`)

`ConditionError` implements `biscuit_terminal::errors::BlockError`, so failures render as rich status blocks. Context capture is lazy and short-circuit aware: `false_flag && ctx.repo == "x"` does not trigger repo context capture.

`ConditionError` implements `biscuit_terminal::errors::BlockError`, so failures render as rich status blocks. Context capture is lazy and short-circuit aware: `false_flag && ctx.repo == "x"` does not trigger repo context capture.

### Shell Expansion Notes

- Body `::shell` and top-level frontmatter `$(...)` share policy loading, whitelist/blacklist checks, approval, and timeout behavior.
- Frontmatter shell expansion stores trimmed `stdout` only and treats malformed matching expressions as hard errors.
- Independent top-level frontmatter shell commands execute concurrently after approval/policy resolution.
- `--allow-shell-timeout` / `ComposeOptions::with_allow_shell_timeout(true)` convert timeouts into empty-string replacements plus compose warnings.

### Shell Blocks

`::shell-block` / `::end-block` directives execute multiple approved commands sequentially and render their combined output. Unlike `::shell`, shell blocks use key-value parameter syntax (`when_error="text"`, `timeout=5`).

- Each non-empty line is a logical command; blank lines are ignored
- Trailing backslash `\` joins with the next non-blank line
- Commands are prepared (approval + policy checks) before any execute
- Output is trimmed, empty outputs are dropped, and one blank line separates non-empty outputs
- Error handling options (`when_error`, `when_exit_code`, `stderr_contains`, etc.) apply per command
- Unhandled failures preserve partial output from earlier successful commands in the error
- Shell blocks can nest inside page blocks; if the parent page block is skipped, commands never execute

## Detailed Topics

- [Compose Pipeline](./compose.md) - Text replacement, interpolation, cleanup
- [Terminal Output](./terminal.md) - ANSI rendering, themes, options
- [Frontmatter](./frontmatter.md) - YAML parsing, merge strategies
- [Document Comparison](./comparison.md) - Structural diff, change classification
- [Error Conventions](./errors.md) - `BlockError` body contract, `SourceContext`, snapshot tests
- [Module Structure](./structure.md) - Package organization and dependencies

## CLI

```bash
md doc.md                        # Render to terminal (auto mode)
md doc.md --output html          # HTML output
md doc.md --output json          # AST JSON output
md doc.md --output markdown      # Plain markdown text
md doc.md --clean                # Normalize formatting (stdout)
md doc.md --clean-save           # Normalize and save back to file
md doc.md --output html --show   # Open in default app
md toc doc.md                    # Table of contents
md delta old.md new.md           # Document comparison
md -v delta old.md new.md        # Verbose comparison (-v is top-level)
md graph doc.md                  # Dependency graph visualization
md graph doc.md --follow         # Recurse into transclusions
md graph doc.md --validate       # Inline validation overlays
```

### Layout Flags

Page-level layout is controlled via CLI flags that construct a `DarkmatterPage` internally:

```bash
md doc.md -m 2 --padding 1 --page-bg subtle --max-width 100
md doc.md --alignment center --fill pad=4
md doc.md --align-code-blocks left --fill-code-blocks max=60
```

- **Margin**: `-m` / `--margin`, `--mx`, `--my`, `--mt`, `--mb`, `--ml`, `--mr`
- **Padding**: `--padding`, `--px`, `--py`, `--pt`, `--pb`, `--pl`, `--pr`
- **Page**: `--page-bg` (alias `--page-background`), `--max-width`, `--line-numbers`, `--no-line-numbers`
- **Alignment**: `--alignment`, `--align-images`, `--align-lists`, `--align-block-quotes`, `--align-tables`, `--align-code-blocks`
- **Fill**: `--fill`, `--fill-images`, `--fill-lists`, `--fill-block-quotes`, `--fill-tables`, `--fill-code-blocks`

Fill grammar: `full`, `pad=<n|n%>`, `indent=<n|n%>`, `max=<n|n%>`, `explicit=<n|n%>`. Precedence follows the same rules as the builder API (shorthand → axis → side for margin/padding; global → component-specific for alignment and fill).

### FileTree Component

`FileTree` is a `Renderable` component that visualizes a Markdown file's dependency surface:
- References (hyperlinks, images, CSS/script imports) above the file line
- Transclusions below the file line with section-aware captions
- Optional recursive expansion via `.follow_transclusions()`
- Optional validation overlays via `.validate()`

Located in `darkmatter/lib/src/markdown/reference/file_tree/`.

### Horizontal Rules

Darkmatter styles CommonMark `---` / `___` / `***` horizontal rules from page-level `hr` frontmatter defaults, with optional per-rule attribute-block overrides (YAML flow-mapping syntax), and dispatches rendering to biscuit-terminal's `HorizontalRule` (`Renderable` + `BrowserRenderable`).

- **Markdown syntax**: `--- { style: waves, width: "50%" }`
- **Supported attributes** (all optional):
    - `style`: `dashes` (default), `dots`, `waves`, `line-star`, `line-circle`, `inset-line`, `curtain-rod`
    - `alignment`: `full` (default), `centered`, `left`, `right`
    - `weight`: `thin`, `medium` (default), `thick`
    - `width`: CSS-like string (`"75%"`, `"200px"`)
    - `color`: CSS color name or `#rrggbb`
- **Preferred page defaults:** put these fields under frontmatter `hr:`; per-rule attributes override only specified keys.
- **Validation:** unknown enum values or unknown attribute keys fall back to the component default and emit `tracing::warn!` (visible via `RUST_LOG=darkmatter=warn`).
- **Bare `---`:** produces a configured rule when `hr:` frontmatter is present, otherwise the default dashed rule.
- **Terminal rendering tiers:**
  1. **Tier 1 (SVG → PNG via `resvg` + `TerminalImage`):** primary path for Kitty-compatible image terminals. When no `color` is specified, the image tier detects the terminal's color mode and uses `white` for dark terminals and `black` for light terminals, avoiding invisible black-on-dark output.
  2. **Tier 2 (Unicode):** fallback when image rendering is unavailable and the locale signals UTF-8.
  3. **Tier 3 (ASCII):** fallback otherwise.
- **Browser rendering:** SVG with `--hr-weight`, `--hr-color`, `--hr-width` CSS variables; per-instance overrides via `render_to_browser_with_inline_variables`.

Located in `darkmatter/lib/src/markdown/inline/types.rs`, `darkmatter/lib/src/markdown/block/rule_processor.rs`, and `darkmatter/lib/src/markdown/output/`.

### Shared Code-Block Helpers

`darkmatter/lib/src/markdown/output/code_block.rs` contains `pub(crate)` helpers used by both terminal and HTML rendering:

- `render_terminal_code_block` - ANSI-highlighted code with padding, line numbers, and highlighted ranges
- `render_html_code_block` - `<div class="code-block">` wrapper with optional line-number table or `<pre><code>` output
- `find_syntax` - Language lookup by extension, name, or alias (shared `syntect` behaviour)

These helpers ensure Markdown code fences and `YamlBlock` use identical syntax-highlighting logic.

### YamlBlock Component

`YamlBlock` is a typed, validated YAML payload that renders as a syntax-highlighted `yaml` code block in both terminal and browser output.

- **Constructors:**
    - `YamlBlock::new(yaml)` — from raw YAML string; validates via `serde_yaml_ng`
    - `YamlBlock::from_yaml_file(path)` — from a YAML file on disk
    - `YamlBlock::from_markdown_content(md)` — extracts only the frontmatter; yields `{}` if none
    - `YamlBlock::from_markdown_file(path)` — from a Markdown file on disk
- **Validation:** all constructors parse YAML through `serde_yaml_ng::from_str` and fail fast; the parsed `Value` is not retained
- **Rendering:** implements `Renderable` and `BrowserRenderable` from `biscuit-terminal`, delegating to the shared code-block helpers with `language = "yaml"`
- **No tree view or custom YAML renderer** — produces a standard highlighted code block

Located in `darkmatter/lib/src/markdown/yaml_block.rs`.



## See Also

- [biscuit-terminal skill](../biscuit-terminal/SKILL.md) - Terminal rendering dependency
