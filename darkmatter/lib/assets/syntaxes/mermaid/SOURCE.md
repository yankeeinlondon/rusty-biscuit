# Mermaid syntax source

`Mermaid.sublime-syntax` is derived from
[SublimeText/Mermaid](https://github.com/SublimeText/Mermaid) at commit
`a5c6d45d1b0a99f8d4e0fdb2b5ccd7fd07809859` (2025-12-03), licensed under the
MIT license in `LICENSE` beside this file.

Darkmatter highlights with syntect, which cannot load the upstream package as
shipped: two of its three syntaxes `extend` the base syntax, and the package
targets Sublime Text's syntax version 2. The vendored file is one flattened,
version-1 syntax:

- **Merged:** `mermaid.sublime-syntax` (the base), `Mermaid
  Flowchart.sublime-syntax`, and `Mermaid SequenceDiagram.sublime-syntax`. The
  flowchart contexts are prefixed `flowchart-` and the sequence contexts
  `sequence-`; the base's `set: scope:source.mermaid.flowchart` and
  `set: scope:source.mermaid.sequence` became `set: flowchart-main` and
  `set: sequence-main`.
- **Removed keys:** `extends`, `version: 2`, and `meta_prepend`. The
  flowchart's prepended prototype rule (the `;` terminator) is the
  `flowchart-terminator` context, included by the flowchart statement
  contexts. `pop: 1` is written `pop: true`.
- **Rewritten:** the `%%{init}%%` directive rule. Upstream combines `pop` with
  an `embed` of the JavaScript syntax, which syntect reads as a bare pop; it
  is one `embed` of a local context with the comment scope as `embed_scope`,
  so the whole directive highlights as a comment. Each flowchart node-shape
  rule likewise pairs `embed` with `set: maybe-graph-link`, which syntect
  reads as a bare `set`, so node text never closed; the `set` is dropped, and
  after the closing bracket the context's final `match: ''` moves on to the
  link.
- **Made self-contained:** the file references no other syntax. Upstream
  embeds JavaScript in `%%{init}%%`, HTML in flowchart node text and in
  sequence note and message text, and YAML in a flowchart `---` config block;
  here that text keeps only its Mermaid scope (comment or string) or none.
  Darkmatter loads this syntax into a small set of its own (with plain text)
  instead of relinking two-face's whole set, which costs about 20 times the
  base set's load in every process that highlights code.
- **Added:** a header rule for every other diagram family (`gantt`, `pie`,
  `stateDiagram`, `classDiagram`, `erDiagram`, ...), scoped
  `keyword.declaration.diagram.mermaid`, whose body keeps comment and
  quoted-string highlighting. Upstream highlights only the flowchart and
  sequence headers.
- **Dropped:** `CSS (for Mermaid).sublime-syntax`. The `classDef`, `style`,
  and `linkStyle` declarations that upstream embeds as CSS stay unhighlighted
  after the name. The unused `quoted-backtick-string` context, which upstream
  had already disabled, is gone. The package's snippets, menus, commands,
  settings, and syntax tests are not vendored.
