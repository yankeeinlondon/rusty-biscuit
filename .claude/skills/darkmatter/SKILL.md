---
name: darkmatter
description: Expert knowledge for the darkmatter Rust library - Markdown parsing, composition, frontmatter, terminal/HTML rendering, style frontmatter, syntax highlighting, and document comparison. Use when parsing or composing Markdown, rendering Markdown to terminal/HTML/Markdown, working with DarkmatterPage, `style:` frontmatter, frontmatter hashing, or comparing documents.
---

# darkmatter

Darkmatter owns Markdown parsing, composition, frontmatter, document comparison,
and the public Markdown rendering paths. Terminal capability detection,
terminal components, images, Mermaid, and graph rendering are delegated to
`biscuit-terminal`.

## Start Here

- Use `darkmatter::markdown::Markdown` for document content.
- Use the compose pipeline for source transformations before rendering.
- Use `DarkmatterPage` for page-level terminal/browser layout around Markdown.
- Use `darkmatter::style` for document `style:` frontmatter.
- Use `biscuit-terminal` components for rich terminal UI outside ordinary
  parsed Markdown rendering.

## Responsibility Split

| Need | Owner |
|------|-------|
| CommonMark/GFM parsing | `darkmatter` |
| Compose pipeline, interpolation, shell directives, transclusion | `darkmatter` |
| Frontmatter extraction and Markdown-aware hashing | `darkmatter` / `md hash` |
| `style:` frontmatter parsing and application | `darkmatter::style` |
| HTML and terminal Markdown renderers | `darkmatter` |
| Terminal capability detection, images, Mermaid, graph adapters | `biscuit-terminal` |
| Shared render tree and target-agnostic layout/style types | `renderable` |

## `style:` Frontmatter Status

The active style-frontmatter wiring phase is
`darkmatter::style::parse::ACTIVE_STYLE_WIRING_SUB_SPEC = 7`.

Implemented:

- schema/parser with kebab-case canonical keys and snake-case deprecation
  aliases
- `style.page.*` layout/background wiring
- `style.table.*`, `style.images.*`, `style.block-quote.*` layout/fill wiring
- `style.ul.*`, `style.ol.*`, `style.li.*` list wiring
- page and component `color` / `bg-color` wiring
- `md --strict-style`, which fails on unknown/deprecated schema keys but not
  on valid future-phase keys
- sub-spec #6 HR migration: top-level `hr:` merges into `style.hr.*` with
  `Deprecated` warnings; inline `{ style: ... }` is parsed as a deprecated alias
  for `{ kind: ... }`; `apply_hr_style` wires `style.hr.*` onto `DarkmatterPage`
- sub-spec #7 bespoke knobs: `page.stylesheet`, `page.meta`, `page.code.theme`,
  hyperlink/image local-style behavior; `apply_bespoke_style` wired into the
  CLI render pipeline; `ACTIVE_STYLE_WIRING_SUB_SPEC` advanced to `7`

No valid v1 schema keys remain unwired.

CLI flags win over frontmatter field-by-field. For implementation details, read
`darkmatter/lib/src/style/{parse.rs,apply.rs}` and
`renderable/features/2026-05-23-style-property/`.

## Common Entry Points

```rust
use darkmatter::markdown::{Markdown, output::{TerminalOptions, write_terminal}};

let md: Markdown = "# Hello\n\nWorld".into();
let mut stdout = std::io::stdout();
write_terminal(&mut stdout, &md, TerminalOptions::default())?;
```

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

## Progressive Disclosure

Open only the topic file needed for the task:

| Topic | File |
|-------|------|
| Compose pipeline | `compose.md` |
| Terminal rendering options | `terminal.md` |
| Frontmatter model | `frontmatter.md` |
| Error/status block conventions | `errors.md` |
| Document comparison | `comparison.md` |
| Module layout | `structure.md` |
| Parser details | `pulldown-cmark.md` |

For render-tree work, switch to the `renderable` skill for the IR model and
the `biscuit-terminal` skill for terminal tree rendering.

## Current Rendering Notes

- Public `Markdown::as_html` and `for_terminal` still use the mature
  `pulldown-cmark` event-stream renderers.
- `darkmatter::markdown::render_tree::fold_markdown_to_document` is the
  experimental Markdown-to-`Document` bridge.
- `YamlBlock` projects to the render tree and uses shared code-block helpers
  for terminal and browser syntax highlighting.
- Code-block themes resolve against the inverted page color mode for contrast;
  ordinary prose follows the real mode.
- Horizontal rules: canonical styling is `style.hr.*` with `apply_hr_style`;
  top-level `hr:` and inline `{ style: ... }` remain deprecated aliases.
