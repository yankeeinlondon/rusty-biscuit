# VS Code

VS Code is the reference client for DMLS feature testing — it has the richest
capability surface of the four targets. Position encoding is UTF-16 only.

## Option A — generic LSP config

Any generic LSP bridge extension that lets you register a stdio server for a
language works. Point it at `dmls` for `markdown` documents. Exact settings keys
vary by bridge extension; the server command is simply:

```
dmls
```

## Option B — the shipped `vscode-dmls` extension (recommended)

The repo ships a thin client extension at
[`../../vscode-dmls/`](../../vscode-dmls/) (`vscode-languageclient` over stdio,
no language logic). One recipe packages and installs it:

```bash
# from darkmatter/ — npm install, vsce package, code --install-extension
just install-vscode-package
```

Then reload an open window (**Developer: Reload Window**) and open a Markdown
file. Settings:

- `dmls.server.path` — absolute path to the binary (set this if VS Code was
  launched from the GUI and cannot see your shell `PATH`; usually
  `~/.cargo/bin/dmls`).
- `dmls.server.args` — extra server arguments (e.g. `--log-level debug`).

For extension development, open `vscode-dmls/` and press **F5** to run it in
an Extension Development Host instead of installing. See
[vscode-dmls/README.md](../../vscode-dmls/README.md) for details.

## What you get

- Full navigation (definition, references, document/workspace symbols),
  document links, and folding.
- Broken-link/anchor and duplicate-heading diagnostics; wiki-link diagnostics.
- Path, anchor, and fence-language completion; wiki-link completion.
- Frontmatter schema diagnostics, completion, and hover (base schema +
  configured extensions + document `$schema`).
- Directive/transclusion/interpolation intelligence and read-only shell-policy
  hover (`dmls` never executes shell or fetches remote content).
- File + heading rename with workspace-wide reference updates
  (`workspace/willRenameFiles` supported), the v1 code-action set, and
  whole-document formatting.
- Semantic tokens classifying interpolations, directives/closers, and wiki
  links for theme-driven de-emphasis (see below).

## Semantic tokens (de-emphasize Darkmatter machinery)

VS Code enables LSP semantic tokens by default (no opt-in), so once `dmls` is
attached, interpolations, directive lines, and wiki links are classified and
ready to style. Semantic tokens **merge over** the Markdown grammar, and token
colors **cannot carry alpha** — "dim" is a *muted foreground*, not transparency.
Add copyable rules to your `settings.json` and tune the palette to your theme:

```json
{
  "editor.semanticTokenColorCustomizations": {
    "enabled": true,
    "rules": {
      "*.interpolation": { "foreground": "#7d8590" },
      "*.directive": { "foreground": "#7d8590" },
      "*.closer": { "foreground": "#6e7681" },
      "macro.wiki": { "foreground": "#7d8590" },
      "string.wiki": { "foreground": "#539bf5" }
    }
  }
}
```

The custom modifiers (`interpolation`, `directive`, `closer`, `wiki`) are the
targeting surface; `*.interpolation` matches every token type carrying that
modifier. `macro.wiki` mutes the `[[ ]]` brackets and `#`/`|` separators while
`string.wiki` keeps the inner path/heading/alias text link-like. `{{{ literal }}}`
spans additionally carry `inert` (`*.inert`) if you want to fade them harder.
The server-side master switch is `[semantic_tokens] enable = false` in
`.dmls.toml`; `wiki.enable = false` suppresses only the wiki tokens.

Scope the rules to Markdown if a language-wide rule is too broad:

```json
{
  "editor.semanticTokenColorCustomizations": {
    "[Default Dark Modern]": {
      "rules": { "*.directive": { "foreground": "#7d8590" } }
    }
  }
}
```

## Notes

- Hover renders full Markdown; DMLS keeps hover content text-first and does not
  require images.
- Change annotations and resource operations (create/rename/delete) are fully
  supported, so rename previews and "create missing file" actions work.
