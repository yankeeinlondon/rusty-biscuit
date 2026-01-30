# Module Structure

## Package Layout

```
darkmatter_lib/
├── markdown/
│   ├── mod.rs              # Markdown type
│   ├── frontmatter/        # YAML frontmatter handling
│   ├── output/
│   │   ├── terminal.rs     # ANSI output (uses biscuit-terminal)
│   │   └── html.rs         # HTML output
│   ├── highlighting/       # Syntax highlighting (syntect)
│   └── dsl/                # Code block metadata parsing
├── mermaid/
│   ├── mod.rs              # Mermaid type
│   ├── render_html.rs      # HTML rendering
│   └── render_terminal.rs  # Delegates to biscuit-terminal
├── terminal/
│   ├── ansi.rs             # ANSI escape code builders
│   └── supports.rs         # Thin wrappers over biscuit-terminal
└── render/
    └── link.rs             # Hyperlink rendering
```

## Dependencies

| Crate | Purpose |
|-------|---------|
| `pulldown-cmark` | CommonMark parsing with GFM extensions |
| `syntect` | Syntax highlighting engine |
| `two-face` | Theme loading with bat-curated themes |
| `biscuit-terminal` | Terminal detection, image rendering, mermaid diagrams |
| `comfy-table` | Table rendering with box-drawing characters |
| `serde` | Frontmatter serialization |

## Output Formats

| Format | Function | Notes |
|--------|----------|-------|
| Terminal | `write_terminal()` | ANSI codes, uses biscuit-terminal |
| HTML | `write_html()` | Web-ready with CSS classes |
| MDAST JSON | `to_mdast()` | Abstract syntax tree |
| Cleaned | `cleanup()` | Normalized spacing/tables |

## Resources

- [darkmatter/lib](../../../darkmatter/lib/) - Library source
- [darkmatter/cli](../../../darkmatter/cli/) - CLI source
