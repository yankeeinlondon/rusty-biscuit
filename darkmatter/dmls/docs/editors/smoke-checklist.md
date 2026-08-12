# DMLS Manual Smoke Checklist

**Status:** This checklist contains **manual** Level 2 (real-editor) verification
steps that remain OUTSTANDING. The automated `darkmatter/dmls/tests/lsp_session.rs`
tests are Level 1 (in-process JSON-RPC protocol tests) and do not exercise real
editor GUIs or OS-specific rendering. The items below require execution in each
real editor (VS Code, Zed, Neovim, Helix) on each target OS (macOS, Windows, Linux).

**Automated for Neovim:** `tests/level2_editor_neovim.rs` (run via
`just test-l2`; requires `nvim` on `PATH`, plus `tmux` for the rendering test)
drives Neovim's real LSP client against the real `dmls` binary and covers the
Neovim column of these semantic-token rows: **capability** (the documented
recipe's highlight groups produce visible SGR color in a real terminal),
**families (full)**, **fenced-code exclusion**, **Unicode + multiline**
positioning after the client's UTF-8 decode, and **config** (both the
`[semantic_tokens] enable` and wiki-only `wiki.enable` toggles repaint live via
the `workspace/semanticTokens/refresh` round trip). The **range** row is not
exercisable on Neovim (its client issues `full` requests only) — verify range
behavior in VS Code. All non-semantic-token rows, and all rows for VS Code,
Zed, and Helix, remain manual.

Run this checklist once per target editor after installing `dmls` on `PATH` and
wiring the editor per its guide in this folder. It exercises every v1 feature
family end to end. Record results and any client-quirk observations in the
feature folder ([../../features/2026-07-04-dmls/](../../../features/2026-07-04-dmls/)).

Use a small scratch workspace with a `.dmls.toml` at its root, at least two
Markdown files that link to each other (both Markdown `[text](path)` and wiki
`[[note]]` links), a document with `$schema` frontmatter, and a document using a
`::file` transclusion and a `{{ variable }}` interpolation.

For the semantic-token checks, also prepare a scratch document containing: an
ordinary `{{ interp }}` and an inert `{{{ literal }}}`, a directive line
(`::file ./intro.md`) and a structural closer (`::end-block`), a
`[[note#heading]]` wiki link, a fenced code block that *contains* `{{ x }}` and
`::file` text, a non-ASCII line (e.g. `café {{ naïve }}`), and a `{{ }}` span
that wraps across two lines.

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
- [ ] **Semantic tokens — capability.** VS Code and Neovim classify tokens with
  no opt-in; Zed requires `"semantic_tokens": "combined"` per language; Helix
  advertises no support and is unaffected (capability-gated off). With the
  editor's styling recipe from its guide applied, machinery reads muted and wiki
  inner text reads link-like.
- [ ] **Semantic tokens — families (full).** On a `full` request (a normal open
  colors the visible document): the ordinary `{{ interp }}` and inert
  `{{{ literal }}}` are both muted (inert may fade harder); the `::file` keyword
  + `./intro.md` target and the `::end-block` closer are muted; the
  `[[note#heading]]` brackets/`#` are muted while `note` and `heading` read
  link-like.
- [ ] **Semantic tokens — range.** Scrolling a large document (editors issue
  `range` requests) styles the same spans as `full`, with no smearing or gaps at
  the viewport edges — the range result is exactly the full result clipped to the
  visible range.
- [ ] **Semantic tokens — fenced-code exclusion.** The `{{ x }}` and `::file`
  text *inside* the fenced code block receive **no** Darkmatter token styling
  (they keep code-block highlighting only).
- [ ] **Semantic tokens — Unicode + multiline.** The non-ASCII line
  (`café {{ naïve }}`) styles the interpolation over the correct columns (no
  off-by-one), and the two-line `{{ … }}` span is styled on **both** lines.
- [ ] **Semantic tokens — config.** Setting `[semantic_tokens] enable = false`
  in `.dmls.toml` removes all Darkmatter token styling without a restart
  (capable clients repaint on refresh; others repaint on the next edit). Setting
  `wiki.enable = false` removes **only** the wiki-link styling, leaving
  interpolation and directive styling in place.
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
