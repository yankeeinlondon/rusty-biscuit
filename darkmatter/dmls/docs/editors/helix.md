# Helix

Helix registers `dmls` from its `languages.toml`. It has the most conservative
capability set of the four targets. Helix advertises UTF-8, so `dmls` negotiates
UTF-8 there.

## Configuration

Add to `~/.config/helix/languages.toml` (global) or a project
`.helix/languages.toml`:

```toml
[language-server.dmls]
command = "dmls"

[[language]]
name = "markdown"
language-servers = ["dmls"]
```

Reload the language configuration with `:config-reload` or restart Helix.

## Client quirks that affect `dmls`

- **No LSP folding / selection ranges.** Helix does not advertise LSP
  folding-range or selection-range capabilities; it uses its own tree-sitter
  folding and native selection expansion. `dmls` gates folding off for Helix.
- **No change annotations.** Rename does not honor `ChangeAnnotation`s; `dmls`
  puts the explanation in code-action titles so nothing is lost.
- **Snippets are opt-in.** Helix only expands snippets when `enable_snippets` is
  on. `dmls` completion emits plain insert text (eager `textEdit`), so this does
  not affect it.
- **One-character code-action selection.** Helix reports a one-character
  code-action selection as an empty selection at the start position. `dmls`
  carries this as the named `helix_one_char_selection_is_empty` profile quirk
  (inherited from the IWES special case) for selection-sensitive actions.
- **File operations are supported.** Helix advertises will/did create, rename,
  and delete, so `workspace/willRenameFiles` link rewriting works.

## Semantic tokens

Helix's LSP client does not advertise semantic-token support, so `dmls` does not
offer the `semanticTokens` capability to Helix at all — it is **capability-gated
off**. This is safe and lossless: Helix keeps its existing tree-sitter Markdown
highlighting unchanged, and no configuration is needed or possible. If Helix adds
semantic-token support in a future release, `dmls` will advertise the provider
automatically with no server change. Interpolation/directive/wiki de-emphasis is
available today in VS Code, Zed, and Neovim.

## What you get

Navigation, diagnostics, completion, hover, frontmatter schema intelligence,
directive/transclusion/interpolation intelligence, read-only shell-policy hover,
heading + file rename, the v1 code-action set, and whole-document formatting.
Hover rendering is conservative (text-first Markdown; no images). Folding falls
back to Helix's tree-sitter folding.
