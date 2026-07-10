# DMLS Manual Smoke Checklist

Run this checklist once per target editor after installing `dmls` on `PATH` and
wiring the editor per its guide in this folder. It exercises every v1 feature
family end to end. Record results and any client-quirk observations in the
feature folder ([../../features/2026-07-04-dmls/](../../../features/2026-07-04-dmls/)).

Use a small scratch workspace with a `.dmls.toml` at its root, at least two
Markdown files that link to each other (both Markdown `[text](path)` and wiki
`[[note]]` links), a document with `$schema` frontmatter, and a document using a
`::file` transclusion and a `{{ variable }}` interpolation.

## Per-editor checklist

For each editor (VS Code, Zed, Neovim, Helix):

- [ ] **Open.** Opening a Markdown file attaches `dmls`; no stdout corruption,
  no errors in the LSP log.
- [ ] **Diagnostics.** A broken link and a duplicate heading each surface a
  diagnostic; fixing them clears it on the next edit.
- [ ] **Completion.** Path completion after `/`/`(`, `#`-anchor completion,
  fence-language completion, and `[[`-wiki completion all offer sensible items
  and insert without a stray `.md`.
- [ ] **Definition.** Go-to-definition jumps from a link/anchor/wiki link to its
  target file/heading; a `::file` target and a `$schema`/`file(...)` value
  navigate too.
- [ ] **References / backlinks.** Find-references on a heading and on a
  transclusion target lists the expected sites.
- [ ] **Hover.** Hover on a link shows the target preview; on a frontmatter key
  shows the schema type/constraints; on a `::shell`/`$()` shows the parsed
  command and policy verdict — and nothing executes.
- [ ] **Symbols / folding.** Document symbols show the heading outline; folding
  works (or falls back to editor-native folding on Helix).
- [ ] **Frontmatter schema.** A missing required key is flagged at the parent
  mapping; key/enum completion and hover reflect the effective schema.
- [ ] **Rename.** Heading rename rewrites `#slug` and wiki `#heading`
  references; an ambiguous heading refuses rather than partially applying.
- [ ] **File rename** (VS Code/Zed/Helix). Renaming a file rewrites wiki links
  that resolve uniquely before and after; a conflict refuses atomically.
- [ ] **Formatting.** `textDocument/formatting` output is byte-equivalent to
  `md clean`; frontmatter and directive lines pass through untouched.
- [ ] **No side effects.** Confirm no child process is spawned and no socket is
  opened for any of the above (also proven automatically by
  `tests/no_side_effects.rs`).

## Recording template

```
### <editor> <version> — <date>
- OS: <macOS/Windows/Linux>
- Result: <pass / issues>
- Quirks observed: <...>
- ClientProfile adjustments needed: <none / ...>
```

Fold any newly discovered client quirk into the `ClientProfile` defaults and
note it in [r7-editor-capability-matrix.md](../../design/research/r7-editor-capability-matrix.md).
