# Module Structure

## Package Layout

```
darkmatter_lib/
├── markdown/
│   ├── mod.rs              # Markdown type, core API
│   ├── frontmatter/        # YAML frontmatter handling
│   ├── output/
│   │   ├── terminal.rs     # ANSI output (uses biscuit-terminal)
│   │   ├── html.rs         # HTML output
│   │   ├── ast.rs          # MDAST JSON output
│   │   └── string.rs       # Plain string output
│   ├── highlighting/       # Syntax highlighting (syntect)
│   ├── delta/              # Structural diff analysis
│   ├── toc/                # Table of contents extraction
│   ├── normalize/          # Heading normalization/releveling
│   ├── compose/            # Inline Pre + Transclusion + Inline Post + Finalization pipeline
│   └── dsl/                # Code block metadata parsing
├── diff/
│   └── visual/             # Markdown-agnostic visual diff utilities
├── mermaid/
│   ├── mod.rs              # Mermaid type, builder API
│   ├── theme.rs            # MermaidTheme, built-in presets
│   ├── render_html.rs      # HTML rendering (CSS variables, mermaid.js)
│   └── render_terminal.rs  # Delegates to biscuit-terminal
├── terminal/               # ANSI color depth constants
├── render/
│   └── link.rs             # Hyperlink rendering (OSC 8)
└── testing/                # Test utilities (cfg(test))
```

## Dependencies

| Crate | Purpose |
|-------|---------|
| `pulldown-cmark` | CommonMark parsing with GFM extensions |
| `syntect` | Syntax highlighting engine |
| `two-face` | Theme loading with bat-curated themes |
| `biscuit-terminal` | Terminal detection, image rendering, mermaid diagrams, table rendering |
| `biscuit-hash` | Content hashing (xxHash) for TOC, delta, and mermaid caching |
| `serde` | Frontmatter serialization |

### Dev-dependencies

| Crate | Purpose |
|-------|---------|
| `chromiumoxide` + `futures-util` | Headless-Chrome browser-render tests (`tests/browser_render.rs`) + `examples/html_to_png.rs` screenshot helper; skip cleanly without a browser |

## Public Modules (`lib.rs`)

| Module | Description |
|--------|-------------|
| `markdown` | Core `Markdown` type with frontmatter, rendering, and manipulation |
| `diff` | Visual diff utilities for strings and files |
| `mermaid` | Mermaid diagram theming |
| `render` | Hyperlink rendering (OSC 8 terminal links) |
| `terminal` | ANSI color depth detection utilities |
| `testing` | Test utilities (cfg(test) only) |

## Output Formats

| Format | Function | Notes |
|--------|----------|-------|
| Terminal | `write_terminal()` / `for_terminal()` | ANSI codes, uses biscuit-terminal |
| HTML | `as_html()` | Standalone with embedded styles |
| MDAST JSON | `as_ast()` | Abstract syntax tree |
| Plain string | `as_string()` | Includes frontmatter if present |
| Cleaned | `cleanup()` | Normalized spacing/tables |

## Resources

- darkmatter/lib - Library source
- darkmatter/cli - CLI source
