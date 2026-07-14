# Use cases + alternatives

## Best-fit use cases for two-face

* **CLI file viewers** (bat-like output) using syntect ANSI rendering
* **Static site generators** building highlighted HTML at build time
* **Pastebin / snippet services** returning HTML
* **GUIs/TUIs** needing embedded assets (portable single-binary distribution)
* **Internal docs tooling** that must highlight many config formats (TOML, Dockerfile, YAML variants)

## When *not* to use two-face

* Binary size is extremely constrained (embedded/very small WASM)
* You only need a couple of basic languages (plain syntect may be enough)
* You need runtime loading of custom theme/syntax files (use syntect directly)

## Alternatives (quick guide)

* **syntect alone**: maximum control; less bundled coverage; more setup
* **inkjet (tree-sitter)**: higher accuracy for complex languages; different ecosystem/cross-compile considerations
* **bat as a library**: includes paging/git gutters/etc., but heavier dependency footprint

## Integration note

two-face is a “resource bundle”; syntect still performs parsing/highlighting. Use two-face sets as drop-in `SyntaxSet` / `ThemeSet` wherever syntect expects them.