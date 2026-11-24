---
name: use-dmls
description: "How to USE DMLS, the Darkmatter Language Server, in an editor — installing the `dmls` binary and wiring it into VS Code, Zed, Neovim, or Helix; what it provides (Markdown links/anchors/backlinks, wiki links, schema-aware frontmatter completion/hover/diagnostics, Darkmatter DSL intelligence for `{{{ }}}`/`::file`/`::block`/`::map`, rename, code actions, formatting, semantic tokens); configuring it with `.dmls.toml` (`[workspace]`, `[wiki]`, `[schema]`, `[expressions]`, `[formatting.markdown]`, `[semantic_tokens]`, `DMLS_CACHE_DIR`); and writing the schema files it reads — `$schema`, `kind: schema` type libraries, `kind: schema-trigger` files, `kind: yaml-schema` definitions, `kind: namespace-schema`, `kind: expression-schema` and `kind: mutation-schema` callable catalogs, schema roots, and `SCHEMAS_DIR`. Use when someone sets up, configures, troubleshoots, or authors schemas for DMLS. For changing DMLS's own source code, load the repo's `darkmatter` skill instead."
---

# Using DMLS

DMLS (`dmls`) is a language server for Markdown with first-class support for
Darkmatter documents. It speaks standard LSP 3.17 over stdio, so any LSP
editor can use it. It uses the same `darkmatter` library as the `md` CLI. As
a result, the editor reports the same problems, with the same codes and
positions, as `md compose` and `md schema validate`.

This skill is for **people using DMLS in an editor**. To change DMLS itself,
load the repo's `darkmatter` skill.

## Pick your task

| You want to… | Read |
|---|---|
| Install `dmls` and connect it to an editor | [install.md](install.md) |
| Know what it does in the editor | [features.md](features.md) |
| Change its behavior (`.dmls.toml`, editor settings) | [configuration.md](configuration.md) |
| Validate frontmatter or YAML files with schemas and schema triggers | [schemas.md](schemas.md) |

## Five facts that answer most questions

1. **It never runs anything.** DMLS never executes `::shell` or `$(…)`, never
   fetches a URL, and never writes a file except through an edit you accept.
   Hover and diagnostics only *explain* what `md compose` would do.
2. **References resolve the way `md compose` resolves them from the
   repository root.** That covers `./`, bare paths, `&`, `^`, `@`, `~`,
   `{{{VAR}}}`, and absolute paths. A link that composes is clickable, and one
   that doesn't is flagged.
3. **Configuration lives in `.dmls.toml`** in a workspace folder. Every key
   is optional, edits apply without a restart, and a broken file falls back
   to defaults instead of stopping the server. The `md` CLI never reads this
   file.
4. **Schemas come from files, not from editor settings.** DMLS searches the
   same five **schema roots** as `md`, in this order: `$SCHEMAS_DIR`, the
   package's `schemas/`, the package area's `schemas/`, the repository root's
   `schemas/`, and `~/schemas/`. A `kind: schema-trigger` file in a root
   applies a schema with no `$schema` in the document.
5. **The environment is fixed when the server starts.** `HOME`,
   `SCHEMAS_DIR`, and every `{{{VAR}}}` come from the process that launched
   `dmls`. A GUI editor started from the Dock doesn't see variables set in
   `~/.zshrc`. Launch the editor from a terminal, then **restart the
   server** after changing a variable.

## Quick start

```bash
just -f darkmatter/justfile install-dmls   # from the rusty-biscuit repo root
dmls --version
```

Then follow the guide for your editor in [install.md](install.md). Optionally,
add a `.dmls.toml` at the workspace root:

```toml
[workspace]
exclude = ["target/**", "node_modules/**"]

[schema]
strict = false          # true: missing required keys and unknown keys become errors

[formatting.markdown]
list-item-padding = "none"
```

## Gotchas

- **The editor can't find `dmls`.** GUI-launched editors don't inherit your
  shell's `PATH`. Set the absolute path, usually `~/.cargo/bin/dmls`: in VS
  Code it's `dmls.server.path`, and in Zed it's `lsp.dmls.binary.path`. In
  Zed, write it as `~/.cargo/bin/dmls` rather than an absolute path: the
  setting follows you into remote (SSH) projects, and `~` is expanded against
  that host's home directory.
- **YAML files get no DMLS by default.** The shipped VS Code and Zed
  extensions attach DMLS only to Markdown. The server can validate whole
  `.yaml` files (trigger files, `kind:`-typed YAML), but you only get that in
  editors where you register `dmls` for YAML yourself (Neovim, Helix). See
  [install.md](install.md#yaml-files).
- **VS Code settings other than the binary path never reach the server.**
  The shipped extension sends none. Put configuration in `.dmls.toml`.
- **A schema file saved outside the editor** is picked up through file
  watching, or on the next save when the client has no watcher (Neovim on
  Linux). **Unsaved** edits to a trigger or `$schema` file reach Markdown
  documents only after you save.
- **A host registration change takes about 300 ms to show.** After
  `claudine setup` or an upgrade rewrites a registered host's files, DMLS
  waits until they have been quiet for about 300 ms, keeping the previous
  results meanwhile, then reloads without a restart. A client that can't
  watch those folders reloads after the next save instead. See
  [schemas.md](schemas.md#claudine).
- **Missing required keys are not reported by default.** A value might be
  supplied at compose time. Set `[schema] strict = true` to see them.
- **A missing `file(...)` target is not a diagnostic.** You may not have
  written the file yet. Hover tells you whether `md compose` will require it.
- **Neovim doesn't rewrite links when you rename a file.** It doesn't send
  `workspace/willRenameFiles`. Heading rename still works.
- **Where the logs are.** `dmls` writes logs to stderr or to `--log-file`;
  stdout carries the LSP protocol. In VS Code they're under Output → "Darkmatter
  Language Server". In Zed, use `dev: open language server logs`. Add
  `--log-level debug --log-file /tmp/dmls.log` to the server arguments for a
  full trace.

## Canonical documentation

These files summarize the repo docs. The authoritative pages are in the
rusty-biscuit monorepo (main checkout
`/Volumes/coding/personal/rusty-biscuit/`):

- `darkmatter/dmls/README.md`, `darkmatter/dmls/docs/features.md`,
  `darkmatter/dmls/docs/editors/*.md`: installation, features, and each
  editor's guide
- `darkmatter/docs/lsp/configuration.md`: every `.dmls.toml` key
- `darkmatter/docs/topics/schemas/dmls-schema-support.md`: schemas in the
  editor
- `darkmatter/docs/topics/schemas/definition.md` (schema roots, trigger
  grammar) and `schema-targeting.md`
- `darkmatter/dmls/docs/diagnostics.md`, `hover.md`, `autocomplete.md`: each
  diagnostic code, each hover, and each completion
